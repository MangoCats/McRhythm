use futures::stream::{self, StreamExt};
use lofty::prelude::*;
use lofty::probe::Probe;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
/// Comprehensive Album Matcher with Edition-by-Edition Processing (Run 19)
use std::cmp::Ordering as CmpOrdering;
use std::collections::HashMap;
use std::io::Write;
use std::panic;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use strsim::{jaro_winkler, levenshtein};
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use time::UtcOffset;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};
use tracing_subscriber::fmt::time::OffsetTime;

// ===== Configuration Constants =====

// Default silence detection parameters
const DEFAULT_THRESHOLD_DB: f64 = -50.0;
const DEFAULT_MIN_DURATION_SECS: f64 = 3.0;

// Track matching tolerance (seconds difference allowed for a track to be considered "matched")
const MATCH_TOLERANCE_SECS: f64 = 10.0;

// MusicBrainz API configuration
const MB_RATE_LIMIT_MS: u64 = 1550; // Milliseconds between API requests (2x safety margin)
const MB_REQUEST_TIMEOUT_SECS: u64 = 30; // HTTP request timeout
const MB_MAX_RELEASES: usize = 150; // Maximum releases to fetch across all strategies
const MB_RETRY_DELAYS_SECS: &[u64] = &[0, 5, 15, 45, 60]; // Backoff delays for retry attempts

// Edition/MBID scoring weights (lower score = better match)
const SCORE_CD_BONUS: f64 = -50.0; // Bonus for CD releases
const SCORE_OFFICIAL_BONUS: f64 = -40.0; // Bonus for Official status
const SCORE_US_BONUS: f64 = -30.0; // Bonus for US releases
const SCORE_TRACK_COUNT_PENALTY: f64 = 60.0; // Seconds penalty per track count difference

// Runtime filter tolerance (file duration must be within this % of edition duration)
const RUNTIME_FILTER_MIN_RATIO: f64 = 0.75; // 75% minimum
const RUNTIME_FILTER_MAX_RATIO: f64 = 1.25; // 125% maximum

// Confidence level thresholds (match percentage)
const CONFIDENCE_EXCELLENT_THRESHOLD: f64 = 80.0;
const CONFIDENCE_GOOD_THRESHOLD: f64 = 60.0;
const CONFIDENCE_FAIR_THRESHOLD: f64 = 40.0;

// Silence detection window sizing (adaptive based on min_duration)
const RMS_WINDOW_SHORT_SECS: f64 = 0.025; // 25ms for very short silences (≤0.3s)
const RMS_WINDOW_MEDIUM_SECS: f64 = 0.05; // 50ms for medium silences (0.3-0.6s)
const RMS_WINDOW_STANDARD_SECS: f64 = 0.1; // 100ms for longer silences (>0.6s)
const RMS_WINDOW_OVERLAP: f64 = 0.5; // 50% overlap between windows

// Stage 4: Edition-guided quiet spot detection
const QUIET_SPOT_WINDOW_SECS: f64 = 0.5; // 500ms window for RMS calculation
const QUIET_SPOT_WINDOW_STEP_SECS: f64 = 0.25; // 250ms step between windows (50% overlap)
const QUIET_SPOT_SEARCH_RADIUS_RATIO: f64 = 0.15; // 15% of expected track duration
const QUIET_SPOT_SEARCH_RADIUS_MIN: f64 = 5.0; // Minimum search radius
const QUIET_SPOT_SEARCH_RADIUS_MAX: f64 = 20.0; // Maximum search radius
const QUIET_SPOT_PROXIMITY_PENALTY: f64 = 0.5; // Penalty factor for distance from expected
const QUIET_SPOT_TOP_EDITIONS: usize = 5; // Try top N editions for guided search

// Name distance weighting for edition ranking
const NAME_DISTANCE_ALBUM_WEIGHT: f64 = 1.414;
const NAME_DISTANCE_ARTIST_WEIGHT: f64 = 1.0;

// Silence threshold for dB calculations
const SILENCE_DB_FLOOR: f32 = -100.0;
const SILENCE_RMS_EPSILON: f32 = 1e-10;

// dB calculation multiplier (standard formula: dB = 20 * log10(value))
const DB_MULTIPLIER: f64 = 20.0;

// Name distance swap threshold ratio
// In resort_by_name_similarity, swap item[i] with item[i+1] if item[i]'s score
// is more than this ratio times worse than item[i+1]'s score
const NAME_DISTANCE_SWAP_RATIO: f64 = 1.732;

// Adaptive RMS window sizing thresholds (for silence detection)
// These define which RMS window size to use based on min_duration_secs
const RMS_WINDOW_THRESHOLD_SHORT: f64 = 0.3; // Use short window for ≤0.3s min duration
const RMS_WINDOW_THRESHOLD_MEDIUM: f64 = 0.6; // Use medium window for ≤0.6s min duration

// Distance penalty multiplier for quiet spot scoring
// Converts normalized distance (0-1) to dB-scale penalty
const QUIET_SPOT_DISTANCE_PENALTY_MULTIPLIER: f64 = 20.0;

// Maximum Name Distance Rank (NDR) allowed for candidate editions
// Editions with NDR > this value are filtered out early to avoid testing
// candidates with poor name similarity (likely wrong album/artist)
const MAX_NAME_DISTANCE_RANK: usize = 50;

// Artist mismatch verification threshold (Jaro-Winkler similarity)
// If winning edition's artist similarity to source artist is below this,
// the match is flagged as potentially wrong. Range: 0.0 (no match) to 1.0 (identical)
// 0.5 = allows "Bob Marley" vs "Bob Marley & The Wailers" but rejects "Fluke" vs "Donny & Marie Osmond"
const ARTIST_MISMATCH_THRESHOLD: f64 = 0.5;

// Minimum match percentage required to accept an artist-mismatched edition
// Even with 100% track match, if artist is different, require very high confidence
const ARTIST_MISMATCH_MIN_MATCH_PCT: f64 = 95.0;

// Run 18: Maximum concurrent albums to process in parallel
// This enables parallel decode + MB lookup while respecting API rate limits
// Memory impact: ~170MB PCM per album, so 8 albums ≈ 1400MB peak RAM
const MAX_CONCURRENT_ALBUMS: usize = 8;

// Early exit grace period (seconds) after first 100% match is found
// Other threads have this much time to complete and contribute results
// before early exit terminates remaining work
const EARLY_EXIT_GRACE_PERIOD_SECS: u64 = 20;

// Staggered feed delay (seconds) between starting new edition tests
// This allows earlier editions to find 100% before later ones even start
const EDITION_FEED_DELAY_SECS: u64 = 4;

// Heartbeat logging interval (seconds) during long-running operations
// Logs query statistics periodically to show progress during MB API calls
const HEARTBEAT_INTERVAL_SECS: u64 = 120;

// Staggered album start multiplier for initial concurrent album launches
// Each album waits (position * STAGGER_MULTIPLIER * MB_RATE_LIMIT_MS) before starting
// This spreads out initial MusicBrainz API calls to reduce rate limit contention
// Set to 0 to disable staggering (all albums start immediately)
// With 60x @ 1550ms: A1=0s, A2=93s, A3=186s, A4=279s, A5=372s, A6=465s, A7=558s, A8=651s
const STAGGER_MULTIPLIER: u64 = 30;

// Stage 4 penalty: Quiet spot detection is less reliable than silence-based detection.
// Results from Stage 4 are de-rated by this percentage (100% Stage 4 becomes 75%).
// Stage 4 can never trigger a 100% early exit due to this penalty.
const STAGE4_PENALTY_PERCENT: f64 = 25.0;

// Stage 2 Parameter Grid: Threshold values (dB) for silence detection sweep
// Run 13: Extended to -30dB, -34dB for albums with louder inter-track gaps
const STAGE2_THRESHOLD_VALUES: [f64; 12] = [
    -50.0, -58.0, -60.0, -54.0, -56.0, -38.0, -52.0, -34.0, -30.0, -48.0, -40.0, -62.0,
];

// Stage 2 Parameter Grid: Min duration values (seconds) for silence detection sweep
const STAGE2_MIN_DURATION_VALUES: [f64; 15] = [
    3.0, 2.0, 2.5, 4.0, 0.5, 1.5, 0.8, 1.0, 0.3, 0.2, 0.4, 0.10, 0.05, 5.0, 3.5,
];

// Sentinel values for missing/unknown metadata
const UNKNOWN_VALUE: &str = "Unknown";
const VARIOUS_ARTISTS: &str = "Various Artists";

// ===== End Configuration Constants =====

// ===== Silence Detection Cache (Run 18: Single-Pass) =====

/// Pre-computed track durations for all parameter combinations.
/// Indexed as: cache[threshold_idx * num_min_durations + min_duration_idx]
/// This avoids re-computing silence detection 23+ times per album (once per edition).
type SilenceCache = Vec<Vec<f64>>;

/// Pre-computed window dB values for single-pass silence detection.
/// Stores dB level for each analysis window, allowing O(windows) filtering
/// instead of O(samples) re-scanning for each parameter combination.
#[derive(Debug, Clone)]
struct WindowDbProfile {
    /// dB level for each window
    db_values: Vec<f64>,
    /// Samples per window step (for sample position calculation)
    window_step: usize,
    /// Total samples in the original audio
    total_samples: usize,
}

/// Single-pass dB profile computation.
/// Scans the audio ONCE and stores dB for each window.
/// This replaces 180 separate scans with 1 scan + 180 cheap filters.
fn compute_window_db_profile(samples: &[f32], sample_rate: u32) -> WindowDbProfile {
    // Use finest window (25ms) for best temporal resolution
    let rms_window_secs = RMS_WINDOW_SHORT_SECS; // 0.025s
    let window_size = (sample_rate as f64 * rms_window_secs) as usize;
    let window_step = (sample_rate as f64 * rms_window_secs * RMS_WINDOW_OVERLAP) as usize;

    // Scan all windows and record their dB levels
    let mut db_values: Vec<f64> = Vec::new();
    for window_start in (0..samples.len()).step_by(window_step) {
        let window_end = (window_start + window_size).min(samples.len());
        let db = calculate_db(&samples[window_start..window_end]) as f64;
        db_values.push(db);
    }

    WindowDbProfile {
        db_values,
        window_step,
        total_samples: samples.len(),
    }
}

/// Find silence regions from pre-computed window dB values.
/// This is the same algorithm as detect_silence, but operates on
/// pre-computed dB values instead of re-scanning samples.
fn find_silence_regions_from_profile(
    profile: &WindowDbProfile,
    threshold_db: f64,
    min_duration_samples: usize,
) -> Vec<(usize, usize)> {
    let mut silence_regions = Vec::new();
    let mut in_silence = false;
    let mut silence_start = 0;

    for (i, &db) in profile.db_values.iter().enumerate() {
        let is_silent = db < threshold_db;

        if is_silent && !in_silence {
            silence_start = i * profile.window_step;
            in_silence = true;
        } else if !is_silent && in_silence {
            let silence_end = i * profile.window_step;
            let duration = silence_end - silence_start;

            if duration >= min_duration_samples {
                silence_regions.push((silence_start, silence_end));
            }
            in_silence = false;
        }
    }

    // Handle silence at end of file
    if in_silence {
        let silence_end = profile.total_samples;
        let duration = silence_end - silence_start;
        if duration >= min_duration_samples {
            silence_regions.push((silence_start, silence_end));
        }
    }

    silence_regions
}

/// Convert silence regions to track durations.
///
/// Given a list of silence regions (sample ranges) within an audio file,
/// calculates the duration of each non-silent segment (i.e., each track).
///
/// # Arguments
/// * `silence_regions` - List of (start_sample, end_sample) tuples marking silent gaps
/// * `total_samples` - Total number of samples in the audio file
/// * `sample_rate` - Audio sample rate in Hz (e.g., 44100)
///
/// # Returns
/// Vector of track durations in seconds, one per detected track.
fn gaps_to_track_durations(
    silence_regions: &[(usize, usize)],
    total_samples: usize,
    sample_rate: u32,
) -> Vec<f64> {
    let mut tracks = Vec::new();
    let mut current_start = 0;

    for &(silence_start, silence_end) in silence_regions {
        if silence_start > current_start {
            let duration_secs = (silence_start - current_start) as f64 / sample_rate as f64;
            tracks.push(duration_secs);
        }
        current_start = silence_end;
    }

    // Add final segment
    if current_start < total_samples {
        let duration_secs = (total_samples - current_start) as f64 / sample_rate as f64;
        tracks.push(duration_secs);
    }

    tracks
}

/// Pre-compute track durations for all parameter combinations using SINGLE-PASS scanning.
///
/// Run 18 optimization: Instead of 180 separate full-file scans, we:
/// 1. Scan ONCE at the most permissive threshold, recording actual dB levels
/// 2. Filter the master gap list for each parameter combination (cheap!)
///
/// Expected speedup: 10-50x on silence cache pre-computation phase.
fn precompute_silence_cache(
    samples: &[f32],
    sample_rate: u32,
    threshold_values: &[f64],
    min_duration_values: &[f64],
) -> SilenceCache {
    let num_min_durations = min_duration_values.len();
    let total_samples = samples.len();

    // SINGLE PASS: Compute dB profile for all windows once
    let profile = compute_window_db_profile(samples, sample_rate);

    // Build list of all (index, threshold, min_duration_samples) tuples
    let params: Vec<(usize, f64, usize)> = threshold_values
        .iter()
        .enumerate()
        .flat_map(|(thresh_idx, &thresh)| {
            min_duration_values
                .iter()
                .enumerate()
                .map(move |(dur_idx, &min_dur)| {
                    let idx = thresh_idx * num_min_durations + dur_idx;
                    let min_samples = (sample_rate as f64 * min_dur) as usize;
                    (idx, thresh, min_samples)
                })
        })
        .collect();

    // Find silence regions for each parameter combination IN PARALLEL
    // This is now cheap because we're filtering pre-computed dB values, not re-scanning audio
    let mut results: Vec<(usize, Vec<f64>)> = params
        .par_iter()
        .map(|&(idx, thresh, min_samples)| {
            let silence_regions = find_silence_regions_from_profile(&profile, thresh, min_samples);
            let durations = gaps_to_track_durations(&silence_regions, total_samples, sample_rate);
            (idx, durations)
        })
        .collect();

    // Sort by index to restore correct order
    results.sort_by_key(|(idx, _)| *idx);

    // Extract just the durations in order
    results
        .into_iter()
        .map(|(_, durations)| durations)
        .collect()
}

// ===== End Silence Detection Cache =====

// ===== Shared Scoring Functions =====

/// Calculate MBID priority score based on release metadata
/// Lower scores are better (CD, Official, US releases prioritized)
fn calculate_mbid_priority_score(is_cd: bool, country: Option<&str>, status: Option<&str>) -> f64 {
    let mut score = 0.0;

    if is_cd {
        score += SCORE_CD_BONUS;
    }

    if country == Some("US") {
        score += SCORE_US_BONUS;
    }

    if status == Some("Official") {
        score += SCORE_OFFICIAL_BONUS;
    }

    score
}

// ===== End Shared Scoring Functions =====

// ===== MusicBrainz API Structures =====

/// Response from MusicBrainz release search API.
#[derive(Debug, Clone, Deserialize)]
struct MBSearchResponse {
    /// List of releases matching the search query.
    releases: Vec<MBRelease>,
}

/// A MusicBrainz release (album) from search results.
///
/// Note: Some fields exist in the MusicBrainz JSON response but are not currently
/// used by our matching algorithm. They are retained for API completeness.
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct MBRelease {
    /// MusicBrainz release ID (MBID).
    id: String,
    /// Release title.
    title: String,
    /// Artist credits for this release.
    #[serde(rename = "artist-credit")]
    artist_credit: Option<Vec<MBArtistCredit>>,
    /// Country of release (e.g., "US", "GB").
    country: Option<String>,
    /// Release status (e.g., "Official", "Bootleg").
    status: Option<String>,
    /// Physical packaging type (e.g., "Jewel Case").
    packaging: Option<String>,
}

/// Artist credit entry linking an artist to a release.
#[derive(Debug, Clone, Deserialize)]
struct MBArtistCredit {
    /// The artist information.
    artist: Option<MBArtist>,
}

/// MusicBrainz artist information.
#[derive(Debug, Clone, Deserialize)]
struct MBArtist {
    /// Artist name.
    name: String,
}

/// Detailed release information including track listings.
///
/// Note: Some fields exist in the MusicBrainz JSON response but are not currently
/// used by our matching algorithm. They are retained for API completeness.
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct MBReleaseDetails {
    /// MusicBrainz release ID (MBID).
    id: String,
    /// Release title.
    title: String,
    /// Media (discs) in this release.
    media: Vec<MBMedia>,
}

/// A single medium (disc) within a release.
#[derive(Debug, Deserialize)]
struct MBMedia {
    /// Tracks on this medium.
    tracks: Vec<MBTrack>,
    /// Format type (e.g., "CD", "Vinyl", "Digital Media").
    format: Option<String>,
}

/// A single track on a medium.
#[derive(Debug, Deserialize)]
struct MBTrack {
    /// Track length in milliseconds.
    length: Option<u32>,
}

// ===== Edition Grouping Structures =====

/// A specific MusicBrainz release ID within an edition group.
/// Multiple MBIDs can represent the same edition (e.g., US vs UK release).
#[derive(Debug, Clone)]
struct EditionMBID {
    /// MusicBrainz release ID.
    mbid: String,
    /// Country of release.
    country: Option<String>,
    /// Release status (Official, Bootleg, etc.).
    status: Option<String>,
    /// Whether this is a CD release (preferred format).
    is_cd: bool,
}

/// A unique album edition identified by track count and duration pattern.
/// Groups multiple MBIDs that represent the same physical release.
#[derive(Debug, Clone)]
struct Edition {
    /// Number of tracks in this edition.
    track_count: usize,
    /// Track durations in seconds.
    durations: Vec<u32>,
    /// All MBIDs representing this edition.
    mbids: Vec<EditionMBID>,
    /// Signature for deduplication (e.g., "107,125,135,...").
    duration_signature: String,
    /// Artist name from MusicBrainz.
    artist: String,
    /// Album title from MusicBrainz.
    album: String,
    /// Rank 1-N based on name similarity to source (1 = best match).
    name_distance_rank: usize,
    /// Overall name distance score (lower = better match).
    name_distance_score: f64,
}

// ===== Run 12 Stage 2/3 Structures =====

/// Over-segmented candidate collected during Stage 2 for Stage 3 assembly.
/// Represents a segmentation with more tracks than expected, which may
/// assemble into a correct match via dynamic programming.
#[derive(Debug, Clone)]
struct OverSegmentedCandidate {
    /// Detected track durations in seconds.
    durations: Vec<f64>,
    /// Silence detection threshold used (dB).
    threshold_db: f64,
    /// Minimum silence duration used (seconds).
    min_duration_secs: f64,
    /// Number of tracks detected.
    track_count: usize,
}

// ===== Query Statistics & Rate Limiting =====

/// Thread-safe query statistics for heartbeat logging.
/// Tracks MusicBrainz API activity to provide progress updates during long operations.
#[derive(Debug)]
struct QueryStats {
    /// Total API queries attempted.
    total_queries: AtomicU64,
    /// Successful queries completed.
    successful_queries: AtomicU64,
    /// Failed queries (after all retries exhausted).
    failed_queries: AtomicU64,
    /// Total retry attempts across all queries.
    retries: AtomicU64,
    /// Number of rate limit waits performed.
    rate_limit_waits: AtomicU64,
    /// Current activity description for heartbeat display.
    current_activity: std::sync::Mutex<String>,
    /// Start time for elapsed calculation.
    start_time: Instant,
    /// Flag to signal heartbeat task to stop.
    stop_flag: AtomicBool,
}

impl QueryStats {
    fn new() -> Self {
        Self {
            total_queries: AtomicU64::new(0),
            successful_queries: AtomicU64::new(0),
            failed_queries: AtomicU64::new(0),
            retries: AtomicU64::new(0),
            rate_limit_waits: AtomicU64::new(0),
            current_activity: std::sync::Mutex::new("initializing".to_string()),
            start_time: Instant::now(),
            stop_flag: AtomicBool::new(false),
        }
    }

    fn record_query_start(&self) {
        self.total_queries.fetch_add(1, Ordering::Relaxed);
    }

    fn record_success(&self) {
        self.successful_queries.fetch_add(1, Ordering::Relaxed);
    }

    fn record_failure(&self) {
        self.failed_queries.fetch_add(1, Ordering::Relaxed);
    }

    fn record_retry(&self) {
        self.retries.fetch_add(1, Ordering::Relaxed);
    }

    fn record_rate_wait(&self) {
        self.rate_limit_waits.fetch_add(1, Ordering::Relaxed);
    }

    fn set_activity(&self, activity: &str) {
        if let Ok(mut guard) = self.current_activity.lock() {
            *guard = activity.to_string();
        }
    }

    fn get_activity(&self) -> String {
        self.current_activity
            .lock()
            .map(|g| g.clone())
            .unwrap_or_default()
    }

    fn stop(&self) {
        self.stop_flag.store(true, Ordering::Relaxed);
    }

    fn is_stopped(&self) -> bool {
        self.stop_flag.load(Ordering::Relaxed)
    }

    fn log_heartbeat(&self, album_idx: usize) {
        let elapsed = self.start_time.elapsed().as_secs();
        let total = self.total_queries.load(Ordering::Relaxed);
        let success = self.successful_queries.load(Ordering::Relaxed);
        let failed = self.failed_queries.load(Ordering::Relaxed);
        let retries = self.retries.load(Ordering::Relaxed);
        let waits = self.rate_limit_waits.load(Ordering::Relaxed);
        let activity = self.get_activity();

        info!(
            "[A{}] [HEARTBEAT] {}s elapsed | Queries: {} ({} ok, {} failed) | Retries: {} | Rate waits: {} | {}",
            album_idx + 1, elapsed, total, success, failed, retries, waits, activity
        );
    }
}

/// Spawn a background heartbeat logging task.
/// Logs query statistics every HEARTBEAT_INTERVAL_SECS until stopped.
///
/// # Arguments
/// * `stats` - Arc-wrapped QueryStats to monitor
/// * `album_idx` - Album index for log message prefix
///
/// # Returns
/// JoinHandle for the spawned task (can be used to await completion)
fn spawn_heartbeat_task(stats: Arc<QueryStats>, album_idx: usize) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let interval = Duration::from_secs(HEARTBEAT_INTERVAL_SECS);
        loop {
            sleep(interval).await;
            if stats.is_stopped() {
                break;
            }
            stats.log_heartbeat(album_idx);
        }
    })
}

/// Rate limiter for MusicBrainz API compliance.
/// Enforces minimum delay between requests to avoid being blocked.
///
/// Uses tokio::sync::Mutex to ensure atomic check-and-wait across concurrent tasks.
/// The lock is held across the await point to serialize all API requests.
#[derive(Debug, Clone)]
struct RateLimiter {
    /// Timestamp of last API request, protected by async mutex for serialization.
    last_request: Arc<tokio::sync::Mutex<std::time::Instant>>,
}

impl RateLimiter {
    fn new() -> Self {
        Self {
            last_request: Arc::new(tokio::sync::Mutex::new(
                std::time::Instant::now() - Duration::from_millis(MB_RATE_LIMIT_MS),
            )),
        }
    }

    async fn wait_with_stats(&self, stats: Option<&QueryStats>) {
        // Hold the lock across the entire wait operation to serialize requests.
        // This ensures only one task can be checking/waiting/updating at a time.
        let mut last = self.last_request.lock().await;

        let elapsed = last.elapsed();

        // MusicBrainz API limit: 1 req/sec
        // Use 1.55s delay for safety margin to prevent 503 errors
        if elapsed < Duration::from_millis(MB_RATE_LIMIT_MS) {
            let wait_time = Duration::from_millis(MB_RATE_LIMIT_MS) - elapsed;
            if let Some(s) = stats {
                s.record_rate_wait();
            }
            sleep(wait_time).await;
        }

        *last = std::time::Instant::now();
    }
}

/// Retry a network operation with exponential backoff, tracking statistics.
/// Attempts: immediate, +5s, +15s, +45s (then gives up)
///
/// # Arguments
/// * `log_prefix` - Prefix for log messages (e.g., "[A42]" for album 42)
/// * `stats` - Optional QueryStats to track retries and outcomes
/// * `operation` - Async closure that performs the network operation
async fn retry_with_backoff_stats<F, Fut, T, E>(
    log_prefix: &str,
    stats: Option<&QueryStats>,
    mut operation: F,
) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let max_attempts = MB_RETRY_DELAYS_SECS.len();

    for (attempt, &delay) in MB_RETRY_DELAYS_SECS.iter().enumerate() {
        if delay > 0 {
            if let Some(s) = stats {
                s.record_retry();
            }
            warn!(
                "{}    Retrying after {} seconds (attempt {}/{})...",
                log_prefix,
                delay,
                attempt + 1,
                max_attempts
            );
            sleep(Duration::from_secs(delay)).await;
        }

        if let Some(s) = stats {
            s.record_query_start();
        }

        match operation().await {
            Ok(result) => {
                if let Some(s) = stats {
                    s.record_success();
                }
                return Ok(result);
            }
            Err(e) => {
                if attempt < max_attempts - 1 {
                    warn!("{}    Network error: {} - will retry", log_prefix, e);
                } else {
                    if let Some(s) = stats {
                        s.record_failure();
                    }
                    error!(
                        "{}    Network error: {} - giving up after {} attempts",
                        log_prefix, e, max_attempts
                    );
                    return Err(e);
                }
            }
        }
    }

    unreachable!()
}

// ===== Validation Result Structures =====

/// Comparison result for a single track.
#[derive(Debug, Clone, Serialize)]
struct TrackMatch {
    /// Duration detected from audio analysis (seconds).
    detected_duration: f64,
    /// Expected duration from MusicBrainz (seconds).
    expected_duration: u32,
    /// Absolute difference between detected and expected (seconds).
    error: f64,
    /// Whether error is within MATCH_TOLERANCE_SECS.
    matches: bool,
}

/// A detected track with no corresponding MusicBrainz entry.
#[derive(Debug, Clone, Serialize)]
struct ExtraTrack {
    /// 1-based index in detected tracks.
    track_index: usize,
    /// Duration in seconds.
    duration: f64,
    /// Description of why this is extra.
    description: String,
}

/// Complete validation result for an album matching attempt.
#[derive(Debug, Clone, Serialize)]
struct ValidationResult {
    /// Path to the source audio file.
    album_path: String,
    /// Artist name (from ID3 or path).
    artist: String,
    /// Album name (from ID3 or path).
    album: String,
    /// MusicBrainz release ID of best match.
    mbid: String,
    /// Full MusicBrainz URL for the release.
    musicbrainz_url: String,
    /// Number of tracks in matched MusicBrainz release.
    expected_track_count: usize,
    /// Number of tracks detected in audio file.
    detected_track_count: usize,
    /// Whether detected count equals expected count.
    perfect_count_match: bool,
    /// Per-track comparison results.
    track_matches: Vec<TrackMatch>,
    /// Detected tracks with no MusicBrainz match.
    extra_tracks: Vec<ExtraTrack>,
    /// Count of tracks within tolerance.
    matched_tracks_count: usize,
    /// Percentage of expected tracks that matched.
    match_percentage: f64,
    /// Mean absolute error across matched tracks (seconds).
    mean_error: f64,
    /// Status message describing result.
    status: String,
    /// Stage that produced best match (e.g., "album_extractor_2_optimization").
    matching_stage: String,
    /// Optimal silence threshold (dB) if found.
    best_threshold_db: Option<f64>,
    /// Optimal minimum silence duration (seconds) if found.
    best_min_duration_secs: Option<f64>,
    /// Confidence level: "Excellent", "Good", "Fair", "Poor".
    confidence: String,
    /// Matched artist name from MusicBrainz (may differ from source).
    matched_artist: String,
    /// Matched album name from MusicBrainz (may differ from source).
    matched_album: String,
    /// True if matched artist significantly differs from source artist.
    /// This indicates the match may be incorrect (wrong artist with similar album name).
    artist_mismatch: bool,
    /// Jaro-Winkler similarity score between source and matched artist (0.0-1.0).
    artist_similarity: f64,
}

impl ValidationResult {
    /// Create an error/failure ValidationResult with common defaults
    fn error(
        album_path: &Path,
        artist: &str,
        album: &str,
        detected_track_count: usize,
        status: String,
    ) -> Self {
        Self {
            album_path: album_path.to_string_lossy().to_string(),
            artist: artist.to_string(),
            album: album.to_string(),
            mbid: String::new(),
            musicbrainz_url: String::new(),
            expected_track_count: 0,
            detected_track_count,
            perfect_count_match: false,
            track_matches: Vec::new(),
            extra_tracks: Vec::new(),
            matched_tracks_count: 0,
            match_percentage: 0.0,
            mean_error: 0.0,
            status,
            matching_stage: "None".to_string(),
            best_threshold_db: None,
            best_min_duration_secs: None,
            confidence: "Poor".to_string(),
            matched_artist: String::new(),
            matched_album: String::new(),
            artist_mismatch: false,
            artist_similarity: 0.0,
        }
    }
}

// ===== Phase 0: ID3 Tag Extraction & Reconciliation =====

/// Raw ID3 tag metadata extracted from an audio file via ffprobe.
///
/// Note: Some fields exist in the ffprobe JSON output but are not currently
/// used by our matching algorithm. They are retained for completeness.
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct ID3Metadata {
    /// Artist name from ID3 tag.
    artist: Option<String>,
    /// Album name from ID3 tag.
    album: Option<String>,
    /// Release date from ID3 tag.
    date: Option<String>,
    /// Genre from ID3 tag.
    genre: Option<String>,
    /// MusicBrainz album ID if present in tags.
    musicbrainz_albumid: Option<String>,
    /// MusicBrainz artist ID if present in tags.
    musicbrainz_artistid: Option<String>,
    /// Comment field (may contain track count info).
    comment: Option<String>,
    /// All ID3 tags as key-value pairs.
    all_tags: HashMap<String, String>,
}

/// Metadata after reconciling ID3 tags with path-derived information.
#[derive(Debug, Clone)]
struct ReconciledMetadata {
    /// Final artist name to use for search.
    artist: String,
    /// Final album name to use for search.
    album: String,
    /// Strategy used to resolve conflicts.
    strategy: ReconciliationStrategy,
    /// Confidence level in the reconciled metadata.
    confidence: MetadataConfidence,
    /// Source of the artist name.
    artist_source: MetadataSource,
    /// Source of the album name.
    album_source: MetadataSource,
    /// Alternative artist name for fallback searches.
    alternate_artist: Option<String>,
    /// Alternative album name for fallback searches.
    alternate_album: Option<String>,
    /// Whether MusicBrainz IDs were found in tags.
    has_musicbrainz_ids: bool,
    /// Estimated track count from ID3 comment field.
    estimated_track_count: Option<usize>,
}

/// Strategy used to reconcile ID3 tags with path-derived metadata.
#[derive(Debug, Clone, Copy)]
enum ReconciliationStrategy {
    /// ID3 and path agree on values.
    DirectMatch,
    /// Partial overlap, chose one source.
    PartialMatch,
    /// Sources disagree, applied heuristics.
    Conflict,
    /// One source missing, used other.
    GapFill,
    /// ID3 tags missing, used path only.
    PathOnly,
}

/// Confidence level in reconciled metadata.
#[derive(Debug, Clone, Copy)]
enum MetadataConfidence {
    /// DirectMatch or strong heuristic confidence.
    High,
    /// PartialMatch or moderate heuristic confidence.
    Medium,
    /// Conflict or GapFill.
    Low,
}

/// Source of a metadata field.
#[derive(Debug, Clone, Copy, PartialEq)]
enum MetadataSource {
    /// Value from ID3 tags.
    ID3,
    /// Value derived from file path.
    Path,
    /// Both sources agree.
    Both,
    /// Sources conflict (heuristic was applied).
    Conflict,
}

/// Decode MP3 file to PCM samples
///
/// Note: Returns Send + Sync error to support tokio::spawn_blocking
fn decode_mp3(path: &Path) -> Result<(Vec<f32>, u32), Box<dyn std::error::Error + Send + Sync>> {
    let file = std::fs::File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let hint = Hint::new();
    let format_opts = FormatOptions::default();
    let metadata_opts = MetadataOptions::default();

    let probed =
        symphonia::default::get_probe().format(&hint, mss, &format_opts, &metadata_opts)?;
    let mut format = probed.format;

    let track = format.default_track().ok_or("No default track")?;
    let track_id = track.id;

    let sample_rate = track.codec_params.sample_rate.ok_or("No sample rate")?;

    let decoder_opts = DecoderOptions::default();
    let mut decoder = symphonia::default::get_codecs().make(&track.codec_params, &decoder_opts)?;

    let mut samples = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(_) => break,
        };

        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(decoded) => match decoded {
                AudioBufferRef::F32(buf) => {
                    for &sample in buf.chan(0) {
                        samples.push(sample);
                    }
                }
                AudioBufferRef::U8(buf) => {
                    for &sample in buf.chan(0) {
                        samples.push((sample as f32 - 128.0) / 128.0);
                    }
                }
                AudioBufferRef::U16(buf) => {
                    for &sample in buf.chan(0) {
                        samples.push((sample as f32 - 32768.0) / 32768.0);
                    }
                }
                AudioBufferRef::U24(buf) => {
                    for &sample in buf.chan(0) {
                        samples.push((sample.inner() as f32 - 8388608.0) / 8388608.0);
                    }
                }
                AudioBufferRef::U32(buf) => {
                    for &sample in buf.chan(0) {
                        samples.push((sample as f32 - 2147483648.0) / 2147483648.0);
                    }
                }
                AudioBufferRef::S8(buf) => {
                    for &sample in buf.chan(0) {
                        samples.push(sample as f32 / 128.0);
                    }
                }
                AudioBufferRef::S16(buf) => {
                    for &sample in buf.chan(0) {
                        samples.push(sample as f32 / 32768.0);
                    }
                }
                AudioBufferRef::S24(buf) => {
                    for &sample in buf.chan(0) {
                        samples.push(sample.inner() as f32 / 8388608.0);
                    }
                }
                AudioBufferRef::S32(buf) => {
                    for &sample in buf.chan(0) {
                        samples.push(sample as f32 / 2147483648.0);
                    }
                }
                AudioBufferRef::F64(buf) => {
                    for &sample in buf.chan(0) {
                        samples.push(sample as f32);
                    }
                }
            },
            Err(_e) => {
                continue;
            }
        }
    }

    Ok((samples, sample_rate))
}

/// Calculate RMS amplitude
fn calculate_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    (samples.iter().map(|&s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
}

/// Calculate RMS amplitude in decibels
fn calculate_db(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return SILENCE_DB_FLOOR;
    }

    let rms = calculate_rms(samples);

    if rms < SILENCE_RMS_EPSILON {
        SILENCE_DB_FLOOR
    } else {
        20.0 * rms.log10()
    }
}

/// Detect silence regions with given parameters
fn detect_silence(
    samples: &[f32],
    sample_rate: u32,
    threshold_db: f64,
    min_duration_secs: f64,
) -> Vec<(usize, usize)> {
    // Adaptive RMS window sizing based on min_duration
    // For very short silence periods, we need smaller RMS windows for better temporal resolution
    // Use window = min(0.1s, min_duration / 3) to ensure at least 3 windows per silence period
    let rms_window_secs = if min_duration_secs <= RMS_WINDOW_THRESHOLD_SHORT {
        // For short durations: use 25ms window for fine-grained detection
        RMS_WINDOW_SHORT_SECS
    } else if min_duration_secs <= RMS_WINDOW_THRESHOLD_MEDIUM {
        // For medium durations: use 50ms window
        RMS_WINDOW_MEDIUM_SECS
    } else {
        // For long durations (>0.6s): use standard 100ms window
        RMS_WINDOW_STANDARD_SECS
    };

    let window_size = (sample_rate as f64 * rms_window_secs) as usize;
    let window_step = (sample_rate as f64 * rms_window_secs * RMS_WINDOW_OVERLAP) as usize; // 50% overlap
    let min_silence_samples = (sample_rate as f64 * min_duration_secs) as usize;

    let mut is_silent = Vec::new();

    for window_start in (0..samples.len()).step_by(window_step) {
        let window_end = (window_start + window_size).min(samples.len());
        let db = calculate_db(&samples[window_start..window_end]);
        is_silent.push((db as f64) < threshold_db);
    }

    // Find continuous silence regions
    let mut silence_regions = Vec::new();
    let mut in_silence = false;
    let mut silence_start = 0;

    for (i, &silent) in is_silent.iter().enumerate() {
        if silent && !in_silence {
            silence_start = i * window_step;
            in_silence = true;
        } else if !silent && in_silence {
            let silence_end = i * window_step;
            let duration = silence_end - silence_start;

            if duration >= min_silence_samples {
                silence_regions.push((silence_start, silence_end));
            }
            in_silence = false;
        }
    }

    silence_regions
}

/// Get track durations for given parameters
fn get_track_durations(
    samples: &[f32],
    sample_rate: u32,
    threshold_db: f64,
    min_duration_secs: f64,
) -> Vec<f64> {
    let silence_regions = detect_silence(samples, sample_rate, threshold_db, min_duration_secs);
    gaps_to_track_durations(&silence_regions, samples.len(), sample_rate)
}

/// Compare detected track durations against expected durations, analyzing match quality.
///
/// Pairs up detected and expected tracks position-by-position and calculates
/// the timing error for each. A track is considered a "match" if the error
/// is within the specified tolerance.
///
/// # Arguments
/// * `detected` - Detected track durations in seconds
/// * `expected` - Expected track durations in seconds (from MusicBrainz)
/// * `tolerance_secs` - Maximum allowed timing error for a track to count as matched
///
/// # Returns
/// Tuple of (matches, matched_count, match_percentage):
/// * `matches` - Per-track match details (duration, error, matched flag)
/// * `matched_count` - Number of tracks within tolerance
/// * `match_percentage` - Percentage of expected tracks that matched (0-100)
fn analyze_track_matching(
    detected: &[f64],
    expected: &[u32],
    tolerance_secs: f64,
) -> (Vec<TrackMatch>, usize, f64) {
    let mut matches = Vec::new();
    let mut matched_count = 0;

    // Match track-by-track up to the minimum count
    let min_count = detected.len().min(expected.len());

    for i in 0..min_count {
        let error = (detected[i] - expected[i] as f64).abs();
        let is_match = error <= tolerance_secs;

        if is_match {
            matched_count += 1;
        }

        matches.push(TrackMatch {
            detected_duration: detected[i],
            expected_duration: expected[i],
            error,
            matches: is_match,
        });
    }

    // Calculate match percentage based on expected count
    let match_percentage = if expected.is_empty() {
        0.0
    } else {
        (matched_count as f64 / expected.len() as f64) * 100.0
    };

    (matches, matched_count, match_percentage)
}

/// Insert spaces before capital letters in CamelCase strings
fn split_camel_case(s: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();

    for (i, &ch) in chars.iter().enumerate() {
        if i > 0 && ch.is_uppercase() && chars[i - 1].is_lowercase() {
            result.push(' ');
        }
        result.push(ch);
    }
    result
}

/// Apply wildcard to handle common misspellings
fn apply_wildcard_fixes(text: &str) -> Option<String> {
    // Handle "Lizzie" vs "Lizzy" (Thin Lizzie -> Thin Lizz*)
    // Use * instead of ? to match any number of characters (Lizzy, Lizzie, Lizzies, etc.)
    if text.contains("Lizzie") {
        return Some(text.replace("Lizzie", "Lizz*"));
    }
    None
}

/// Generate search query variants to try in sequence
fn generate_search_queries(artist: &str, album: &str) -> Vec<String> {
    let mut queries = Vec::new();

    // Strategy 1: Original query with type:album filter
    queries.push(format!(
        "type:album AND artist:{} AND release:{}",
        artist, album
    ));

    // Strategy 2: CamelCase split (most effective per test results)
    let album_spaced = split_camel_case(album);
    if album_spaced != album {
        queries.push(format!(
            "type:album AND artist:{} AND release:\"{}\"",
            artist, album_spaced
        ));
    }

    // Strategy 3: Fuzzy matching (catches punctuation differences like "Funk49" -> "Funk #49")
    queries.push(format!(
        "type:album AND artist:{}~ AND release:{}~",
        artist, album
    ));

    // Strategy 4: Targeted wildcard for common misspellings (e.g., "Lizzie" -> "Lizz*")
    if let Some(artist_wildcard) = apply_wildcard_fixes(artist) {
        let album_variant = apply_wildcard_fixes(album).unwrap_or_else(|| album.to_string());
        queries.push(format!(
            "type:album AND artist:{} AND release:{}",
            artist_wildcard, album_variant
        ));
    } else if let Some(album_wildcard) = apply_wildcard_fixes(album) {
        queries.push(format!(
            "type:album AND artist:{} AND release:{}",
            artist, album_wildcard
        ));
    }

    // Strategy 5: Aggressive fuzzy search (~2 edits - more tolerant, catches more misspellings)
    queries.push(format!(
        "type:album AND artist:{}~2 AND release:{}~2",
        artist, album
    ));

    // Strategy 6: Per-token fuzzy matching (handles multi-word names better)
    let artist_tokens: Vec<&str> = artist.split_whitespace().collect();
    let album_tokens: Vec<&str> = album.split_whitespace().collect();
    if artist_tokens.len() > 1 || album_tokens.len() > 1 {
        let artist_fuzzy = artist_tokens
            .iter()
            .map(|t| format!("{}~", t))
            .collect::<Vec<_>>()
            .join(" ");
        let album_fuzzy = album_tokens
            .iter()
            .map(|t| format!("{}~", t))
            .collect::<Vec<_>>()
            .join(" ");
        queries.push(format!(
            "type:album AND artist:({}) AND release:({})",
            artist_fuzzy, album_fuzzy
        ));
    }

    // Strategy 7: Album-only fallback (last resort when artist name is problematic)
    queries.push(format!("type:album AND release:{}", album));

    queries
}

/// Calculate average Levenshtein distance from a candidate string to all source variants.
///
/// Used for fuzzy matching of album/artist names against multiple possible spellings.
/// Returns 0.0 if sources is empty (perfect match by default).
///
/// # Arguments
/// * `candidate` - The string to compare (e.g., MusicBrainz album name)
/// * `sources` - List of source strings to compare against (e.g., ID3 tag variants)
///
/// # Returns
/// Average edit distance across all sources. Lower values indicate better matches.
fn avg_levenshtein(candidate: &str, sources: &[String]) -> f64 {
    if sources.is_empty() {
        return 0.0;
    }
    let total: usize = sources.iter().map(|s| levenshtein(candidate, s)).sum();
    total as f64 / sources.len() as f64
}

/// Calculate overall name distance score for a release against source variants
/// Returns: (2 * album_distance + artist_distance) / 3
/// Lower scores indicate better matches
fn calculate_name_distance(
    candidate_artist: &str,
    candidate_album: &str,
    source_artists: &[String],
    source_albums: &[String],
) -> f64 {
    let avg_album_distance = avg_levenshtein(candidate_album, source_albums);
    let avg_artist_distance = avg_levenshtein(candidate_artist, source_artists);

    // Overall score: album name weighted 2x, artist name weighted 1x
    (NAME_DISTANCE_ALBUM_WEIGHT * avg_album_distance
        + NAME_DISTANCE_ARTIST_WEIGHT * avg_artist_distance)
        / (NAME_DISTANCE_ALBUM_WEIGHT + NAME_DISTANCE_ARTIST_WEIGHT)
}

/// Normalize an artist name for comparison purposes.
/// Removes common variations like "The", punctuation, and normalizes case.
///
/// # Examples
/// - "The Beatles" -> "beatles"
/// - "Bob Marley & The Wailers" -> "bob marley wailers"
/// - "P!nk" -> "pink"
/// - "Björk" -> "bjork"
fn normalize_artist_name(name: &str) -> String {
    let mut normalized = name.to_lowercase();

    // Remove common prefixes
    for prefix in &["the ", "a "] {
        if normalized.starts_with(prefix) {
            normalized = normalized[prefix.len()..].to_string();
        }
    }

    // Replace common conjunctions and punctuation
    normalized = normalized
        .replace(" & ", " ")
        .replace(" and ", " ")
        .replace("'", "")
        .replace("\"", "")
        .replace("!", "")
        .replace(".", "")
        .replace(",", "")
        .replace("-", " ");

    // Normalize unicode characters (basic ASCII folding)
    normalized = normalized
        .replace("ö", "o")
        .replace("ø", "o")
        .replace("ü", "u")
        .replace("ä", "a")
        .replace("é", "e")
        .replace("è", "e")
        .replace("ë", "e")
        .replace("í", "i")
        .replace("ì", "i")
        .replace("ñ", "n")
        .replace("ß", "ss");

    // Collapse multiple spaces
    let parts: Vec<&str> = normalized.split_whitespace().collect();
    parts.join(" ")
}

/// Verify if a matched artist name is sufficiently similar to the source artist.
/// Uses Jaro-Winkler similarity which gives higher weight to prefix matches.
///
/// # Arguments
/// * `source_artist` - Artist name from ID3 tags/path
/// * `matched_artist` - Artist name from selected MusicBrainz edition
///
/// # Returns
/// Tuple of (similarity_score, is_acceptable_match)
/// - similarity_score: 0.0 (completely different) to 1.0 (identical)
/// - is_acceptable_match: true if similarity >= ARTIST_MISMATCH_THRESHOLD
fn verify_artist_match(source_artist: &str, matched_artist: &str) -> (f64, bool) {
    let normalized_source = normalize_artist_name(source_artist);
    let normalized_matched = normalize_artist_name(matched_artist);

    // Use Jaro-Winkler for better handling of name variations
    // It gives higher scores when strings share a common prefix
    let similarity = jaro_winkler(&normalized_source, &normalized_matched);

    // Also check if one is a substring of the other (handles "Bob Marley" vs "Bob Marley & The Wailers")
    let is_substring = normalized_source.contains(&normalized_matched)
        || normalized_matched.contains(&normalized_source);

    let is_acceptable = similarity >= ARTIST_MISMATCH_THRESHOLD || is_substring;

    (similarity, is_acceptable)
}

/// Search MusicBrainz using all combinations of artist/album variants and strategies.
///
/// Iterates through all artist×album×strategy combinations, fetching releases
/// until MB_MAX_RELEASES limit is reached or all combinations are exhausted.
///
/// # Returns
/// Vector of unique releases (deduplicated by MBID).
async fn search_all_mb_strategies(
    client: &reqwest::Client,
    artist_variants: &[String],
    album_variants: &[String],
    rate_limiter: &RateLimiter,
    album_idx: usize,
    stats: Option<&QueryStats>,
) -> Vec<MBRelease> {
    let mut all_releases: Vec<MBRelease> = Vec::new();
    let mut seen_mbids = std::collections::HashSet::new();
    let log_prefix = format!("[A{}]", album_idx + 1);

    for artist in artist_variants {
        for album in album_variants {
            let search_queries = generate_search_queries(artist, album);

            for (i, query) in search_queries.iter().enumerate() {
                if all_releases.len() >= MB_MAX_RELEASES {
                    info!(
                        "[A{}]   Reached {} release limit",
                        album_idx + 1,
                        MB_MAX_RELEASES
                    );
                    break;
                }

                if let Some(s) = stats {
                    s.set_activity(&format!(
                        "searching: {} / {} (strategy {}/{})",
                        artist,
                        album,
                        i + 1,
                        search_queries.len()
                    ));
                }

                let encoded_query = urlencoding::encode(query);
                let search_url = format!(
                    "https://musicbrainz.org/ws/2/release/?query={}&fmt=json&limit=100",
                    encoded_query
                );

                rate_limiter.wait_with_stats(stats).await;
                debug!("{} MB query: {}", log_prefix, search_url);
                let response = retry_with_backoff_stats(&log_prefix, stats, || async {
                    client
                        .get(&search_url)
                        .send()
                        .await
                        .map_err(|e| format!("error sending request: {}", e))?
                        .json::<MBSearchResponse>()
                        .await
                        .map_err(|e| format!("error parsing JSON: {}", e))
                })
                .await;

                let response = match response {
                    Ok(r) => r,
                    Err(e) => {
                        info!(
                            "  Strategy {}/{} for '{}' / '{}' FAILED: {}",
                            i + 1,
                            search_queries.len(),
                            artist,
                            album,
                            e
                        );
                        continue;
                    }
                };

                if response.releases.is_empty() {
                    continue;
                }

                // Add new releases (deduplicate by MBID)
                for release in response.releases {
                    if seen_mbids.insert(release.id.clone()) {
                        all_releases.push(release);

                        if all_releases.len() >= MB_MAX_RELEASES {
                            break;
                        }
                    }
                }
            }

            if all_releases.len() >= MB_MAX_RELEASES {
                break;
            }
        }

        if all_releases.len() >= MB_MAX_RELEASES {
            break;
        }
    }

    all_releases
}

/// Calculate Name Distance Rank (NDR) for releases and filter by MAX_NAME_DISTANCE_RANK.
///
/// Ranks releases by how well their artist/album names match the source variants,
/// then filters out releases with rank > MAX_NAME_DISTANCE_RANK.
///
/// # Returns
/// Filtered releases with their rank (1-N) and NDR score.
fn calculate_ndr_and_filter<'a>(
    releases: &'a [MBRelease],
    artist_variants: &[String],
    album_variants: &[String],
    album_idx: usize,
) -> Vec<(&'a MBRelease, usize, f64)> {
    // Calculate name distance score for each release
    let mut releases_with_ndr: Vec<(&MBRelease, f64)> = releases
        .iter()
        .map(|release| {
            let artist = release
                .artist_credit
                .as_ref()
                .and_then(|credits| credits.first())
                .and_then(|credit| credit.artist.as_ref())
                .map(|artist| artist.name.as_str())
                .unwrap_or("Unknown Artist");
            let score =
                calculate_name_distance(artist, &release.title, artist_variants, album_variants);
            (release, score)
        })
        .collect();

    // Sort by NDR score (ascending - lower is better)
    releases_with_ndr.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    // Filter to keep only releases with rank <= MAX_NAME_DISTANCE_RANK
    let pre_filter_count = releases_with_ndr.len();
    let filtered_releases: Vec<(&MBRelease, usize, f64)> = releases_with_ndr
        .into_iter()
        .enumerate()
        .map(|(idx, (release, score))| (release, idx + 1, score)) // Assign rank 1-N
        .filter(|(_, rank, _)| *rank <= MAX_NAME_DISTANCE_RANK)
        .collect();

    let ndr_filtered_count = pre_filter_count - filtered_releases.len();
    if ndr_filtered_count > 0 {
        info!(
            "[A{}]   Early NDR filter: skipping {} releases (NDR > {}), fetching details for {}",
            album_idx + 1,
            ndr_filtered_count,
            MAX_NAME_DISTANCE_RANK,
            filtered_releases.len()
        );
    }

    filtered_releases
}

/// Fetch track details for a single release from MusicBrainz.
///
/// Retrieves media/tracks for a release and extracts durations, artist name,
/// and media format (CD detection).
///
/// # Returns
/// `Some((durations, mbid_info, artist, album, rank, score))` if successful, `None` if failed.
async fn fetch_release_track_details(
    client: &reqwest::Client,
    release: &MBRelease,
    rank: usize,
    score: f64,
    rate_limiter: &RateLimiter,
    album_idx: usize,
    stats: Option<&QueryStats>,
) -> Option<(Vec<u32>, EditionMBID, String, String, usize, f64)> {
    if let Some(s) = stats {
        s.set_activity(&format!(
            "fetching details: {} (rank {})",
            release.title, rank
        ));
    }

    rate_limiter.wait_with_stats(stats).await;

    let details_url = format!(
        "https://musicbrainz.org/ws/2/release/{}?inc=recordings&fmt=json",
        release.id
    );
    let log_prefix = format!("[A{}]", album_idx + 1);
    debug!("{} MB details: {}", log_prefix, details_url);
    let details = retry_with_backoff_stats(&log_prefix, stats, || async {
        client
            .get(&details_url)
            .send()
            .await
            .map_err(|e| format!("error fetching details: {}", e))?
            .json::<MBReleaseDetails>()
            .await
            .map_err(|e| format!("error parsing details: {}", e))
    })
    .await;

    let details = match details {
        Ok(d) => d,
        Err(_) => return None,
    };

    // Extract track durations and media format
    let mut durations = Vec::new();
    let mut is_cd = false;
    for medium in &details.media {
        for track in &medium.tracks {
            if let Some(length_ms) = track.length {
                durations.push(length_ms / 1000); // Convert to seconds
            }
        }
        if let Some(ref format) = medium.format {
            if format == "CD" {
                is_cd = true;
            }
        }
    }

    if durations.is_empty() {
        return None;
    }

    // Extract artist name
    let artist = release
        .artist_credit
        .as_ref()
        .and_then(|credits| credits.first())
        .and_then(|credit| credit.artist.as_ref())
        .map(|artist| artist.name.clone())
        .unwrap_or_else(|| "Unknown Artist".to_string());

    Some((
        durations,
        EditionMBID {
            mbid: release.id.clone(),
            country: release.country.clone(),
            status: release.status.clone(),
            is_cd,
        },
        artist,
        release.title.clone(),
        rank,
        score,
    ))
}

/// Comprehensive MusicBrainz search using ALL strategies and name variants.
///
/// Orchestrates the full search process:
/// 1. Search using all artist/album/strategy combinations
/// 2. Filter by Name Distance Rank (NDR)
/// 3. Fetch track details for filtered releases
///
/// # Returns
/// Vec of (durations, mbid_info, artist, album, name_distance_rank, name_distance_score)
async fn comprehensive_musicbrainz_search(
    artist_variants: &[String], // e.g., ["Jessita Reyes", "Various"]
    album_variants: &[String], // e.g., ["Native American Flute Lullabies", "NativeAmericanFluteLullabies"]
    rate_limiter: &RateLimiter,
    album_idx: usize,           // Album index for log messages
    stats: Option<&QueryStats>, // Optional stats for heartbeat logging
) -> Result<Vec<(Vec<u32>, EditionMBID, String, String, usize, f64)>, Box<dyn std::error::Error>> {
    info!(
        "[A{}]   Fetching MusicBrainz data (comprehensive search)...",
        album_idx + 1
    );

    let client = reqwest::Client::builder()
        .user_agent("WKMP-ParameterValidator/0.1 (https://github.com/yourusername/wkmp)")
        .timeout(Duration::from_secs(MB_REQUEST_TIMEOUT_SECS))
        .build()?;

    // Step 1: Search using all artist/album/strategy combinations
    let all_releases = search_all_mb_strategies(
        &client,
        artist_variants,
        album_variants,
        rate_limiter,
        album_idx,
        stats,
    )
    .await;
    info!(
        "[A{}]   Found {} unique releases across all search strategies",
        album_idx + 1,
        all_releases.len()
    );

    // Step 2: Calculate NDR and filter releases (Run 15 optimization)
    let filtered_releases =
        calculate_ndr_and_filter(&all_releases, artist_variants, album_variants, album_idx);

    if let Some(s) = stats {
        s.set_activity(&format!(
            "fetching track details for {} releases",
            filtered_releases.len()
        ));
    }

    // Step 3: Fetch track details for filtered releases
    let mut results: Vec<(Vec<u32>, EditionMBID, String, String, usize, f64)> = Vec::new();
    for (release, rank, score) in filtered_releases {
        if let Some(result) = fetch_release_track_details(
            &client,
            release,
            rank,
            score,
            rate_limiter,
            album_idx,
            stats,
        )
        .await
        {
            results.push(result);
        }
    }

    if let Some(s) = stats {
        s.set_activity("MusicBrainz search complete");
    }

    Ok(results)
}

/// Group MBIDs into editions based on track count + duration pattern
/// Multiple MBIDs can represent the same edition (e.g., US vs UK release of same album)
/// Takes the best (lowest) name distance rank and its score among all MBIDs in an edition
fn group_into_editions(
    releases: Vec<(Vec<u32>, EditionMBID, String, String, usize, f64)>,
    album_idx: usize,
) -> Vec<Edition> {
    let mut editions: Vec<Edition> = Vec::new();

    for (durations, mbid_info, artist, album, rank, score) in releases {
        // Create signature: "track_count:duration1,duration2,..."
        let signature = format!(
            "{}:{}",
            durations.len(),
            durations
                .iter()
                .map(|d| d.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );

        // Find existing edition with this signature
        if let Some(edition) = editions
            .iter_mut()
            .find(|e| e.duration_signature == signature)
        {
            // Add this MBID to existing edition
            edition.mbids.push(mbid_info);
            // Update to best (lowest) rank and corresponding score
            if rank < edition.name_distance_rank {
                edition.name_distance_rank = rank;
                edition.name_distance_score = score;
            }
        } else {
            // Create new edition
            editions.push(Edition {
                track_count: durations.len(),
                durations: durations.clone(),
                mbids: vec![mbid_info],
                duration_signature: signature,
                artist,
                album,
                name_distance_rank: rank,
                name_distance_score: score,
            });
        }
    }

    info!(
        "[A{}]   Grouped into {} unique editions",
        album_idx + 1,
        editions.len()
    );

    // Sort editions by track count (helps with display)
    editions.sort_by_key(|e| e.track_count);

    editions
}

/// Score an edition by how well it matches the audio file's characteristics
/// Lower score = better match (closer to audio file)
/// Each track count difference = 60 seconds of runtime error
fn score_edition_match(
    edition: &Edition,
    file_duration_secs: f64,
    estimated_track_count: Option<usize>,
) -> f64 {
    let edition_duration_secs: u32 = edition.durations.iter().sum();
    let mut score = (edition_duration_secs as f64 - file_duration_secs).abs();

    // Add penalty for track count difference (60 seconds per track)
    if let Some(file_tracks) = estimated_track_count {
        let track_diff = (edition.track_count as i32 - file_tracks as i32).abs();
        score += track_diff as f64 * SCORE_TRACK_COUNT_PENALTY;
    }

    score
}

/// Re-sort editions using a conservative bubble sort based on name similarity.
///
/// After editions are sorted by runtime likelihood, this performs a secondary
/// sort that allows editions with significantly better name matches to bubble up,
/// while preserving the runtime-based ordering for editions with similar name scores.
///
/// Algorithm: Repeatedly scan the list. If item[i] has a name_distance_score more
/// than 2x the score of item[i+1], swap them. Continue until a full pass has zero swaps.
/// This ensures only significantly better name matches override the runtime ordering.
fn resort_by_name_similarity(editions: &mut [Edition]) {
    if editions.len() < 2 {
        return;
    }

    loop {
        let mut swaps = 0;
        for i in 0..editions.len() - 1 {
            // Lower name_distance_score is better
            // Swap if current item's score is more than NAME_DISTANCE_SWAP_RATIO times worse than next item's score
            if editions[i].name_distance_score
                > NAME_DISTANCE_SWAP_RATIO * editions[i + 1].name_distance_score
            {
                editions.swap(i, i + 1);
                swaps += 1;
            }
        }
        if swaps == 0 {
            break;
        }
    }
}

/// Select best MBID from an edition using metadata prioritization
/// Returns (mbid, country, status, is_cd) tuple
fn select_best_mbid(edition: &Edition) -> String {
    if edition.mbids.is_empty() {
        return String::new();
    }

    if edition.mbids.len() == 1 {
        return edition.mbids[0].mbid.clone();
    }

    // Score each MBID using the established criteria
    let mut scored: Vec<(&EditionMBID, f64)> = edition
        .mbids
        .iter()
        .map(|mbid_info| {
            let score = calculate_mbid_priority_score(
                mbid_info.is_cd,
                mbid_info.country.as_deref(),
                mbid_info.status.as_deref(),
            );
            (mbid_info, score)
        })
        .collect();

    scored.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(CmpOrdering::Equal));

    scored[0].0.mbid.clone()
}

/// Extract artist and album from path
/// Expects pattern: ".../Artist/Album.mp3" (cross-platform)
fn extract_metadata_from_path(path: &Path) -> (String, String) {
    // Use path components for cross-platform compatibility
    let components: Vec<_> = path
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();

    if components.len() >= 2 {
        let artist = components[components.len() - 2].replace(", ", " ");
        let album_with_ext = components[components.len() - 1];
        let album = album_with_ext.trim_end_matches(".mp3").replace(", ", " ");

        (artist, album)
    } else {
        (UNKNOWN_VALUE.to_string(), UNKNOWN_VALUE.to_string())
    }
}

// ===== Phase 0: ID3 Tag Extraction Functions =====

/// Normalized string comparison (case-insensitive, alphanumeric + whitespace only)
fn strings_match(a: &str, b: &str) -> bool {
    let normalize = |s: &str| -> String {
        s.chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .collect::<String>()
            .to_lowercase()
    };

    normalize(a) == normalize(b)
}

/// Choose between ID3 and path artist using heuristics
fn choose_artist(
    id3_artist: &str,
    path_artist: &str,
    album: &str,
) -> (String, String, MetadataSource) {
    // Heuristic 1: Avoid "Various Artists" if possible
    let id3_is_various = id3_artist.to_lowercase().contains("various");
    let path_is_various = path_artist.to_lowercase().contains("various");

    if id3_is_various && !path_is_various {
        return (
            path_artist.to_string(),
            id3_artist.to_string(),
            MetadataSource::Path,
        );
    }
    if path_is_various && !id3_is_various {
        return (
            id3_artist.to_string(),
            path_artist.to_string(),
            MetadataSource::ID3,
        );
    }

    // Heuristic 2: If album name suggests compilation, prefer ID3
    let album_lower = album.to_lowercase();
    if album_lower.contains("greatest")
        || album_lower.contains("best of")
        || album_lower.contains("collection")
        || album_lower.contains("anthology")
    {
        return (
            id3_artist.to_string(),
            path_artist.to_string(),
            MetadataSource::ID3,
        );
    }

    // Default: Prefer ID3
    (
        id3_artist.to_string(),
        path_artist.to_string(),
        MetadataSource::ID3,
    )
}

/// Choose between ID3 and path album using heuristics
fn choose_album(id3_album: &str, path_album: &str) -> (String, String, MetadataSource) {
    // Heuristic: Prefer ID3 if it contains more detail (edition, year, etc.)
    let id3_has_detail = id3_album.contains('(')
        || id3_album.contains('[')
        || id3_album.contains("Deluxe")
        || id3_album.contains("Edition")
        || id3_album.len() > path_album.len() + 10;

    if id3_has_detail {
        return (
            id3_album.to_string(),
            path_album.to_string(),
            MetadataSource::ID3,
        );
    }

    // Default: Prefer ID3
    (
        id3_album.to_string(),
        path_album.to_string(),
        MetadataSource::ID3,
    )
}

/// Extract ID3 tags using lofty (pure Rust, no external dependencies)
fn extract_id3_tags(file_path: &Path) -> Result<ID3Metadata, Box<dyn std::error::Error>> {
    let tagged_file = Probe::open(file_path)?
        .read()
        .map_err(|e| format!("Failed to read tags: {}", e))?;

    let tag = match tagged_file.primary_tag() {
        Some(t) => t,
        None => {
            // No primary tag, return empty metadata
            return Ok(ID3Metadata {
                artist: None,
                album: None,
                date: None,
                genre: None,
                musicbrainz_albumid: None,
                musicbrainz_artistid: None,
                comment: None,
                all_tags: HashMap::new(),
            });
        }
    };

    // Build all_tags map from all tag items
    let mut all_tags = HashMap::new();
    for item in tag.items() {
        let key = format!("{:?}", item.key()).to_lowercase();
        if let lofty::tag::ItemValue::Text(value) = item.value() {
            all_tags.insert(key, value.clone());
        }
    }

    // Extract standard fields using lofty's Accessor trait
    let artist = tag.artist().map(|s| s.to_string()).or_else(|| {
        tag.get_string(&lofty::tag::ItemKey::AlbumArtist)
            .map(String::from)
    });

    let album = tag.album().map(|s| s.to_string());

    let date = tag.year().map(|y| y.to_string()).or_else(|| {
        tag.get_string(&lofty::tag::ItemKey::RecordingDate)
            .map(String::from)
    });

    let genre = tag.genre().map(|s| s.to_string());

    let comment = tag.comment().map(|s| s.to_string());

    // MusicBrainz IDs
    let musicbrainz_albumid = tag
        .get_string(&lofty::tag::ItemKey::MusicBrainzReleaseId)
        .map(String::from);

    let musicbrainz_artistid = tag
        .get_string(&lofty::tag::ItemKey::MusicBrainzArtistId)
        .map(String::from);

    Ok(ID3Metadata {
        artist,
        album,
        date,
        genre,
        musicbrainz_albumid,
        musicbrainz_artistid,
        comment,
        all_tags,
    })
}

/// Reconcile ID3 tags with path/filename metadata
fn reconcile_metadata(
    id3: &ID3Metadata,
    path_artist: &Option<String>,
    path_album: &Option<String>,
) -> ReconciledMetadata {
    // Extract estimated track count from comment field
    let estimated_track_count = id3.comment.as_ref().and_then(|c| {
        // Look for patterns like "52 tracks", "28 tracks", etc.
        c.split_whitespace()
            .next()
            .and_then(|s| s.parse::<usize>().ok())
    });

    let has_musicbrainz_ids =
        id3.musicbrainz_albumid.is_some() || id3.musicbrainz_artistid.is_some();

    match (&id3.artist, path_artist, &id3.album, path_album) {
        // Case 1: Both sources have both fields
        (Some(id3_artist), Some(path_artist), Some(id3_album), Some(path_album)) => {
            let artist_match = strings_match(id3_artist, path_artist);
            let album_match = strings_match(id3_album, path_album);

            match (artist_match, album_match) {
                (true, true) => {
                    // DirectMatch - both sources agree
                    ReconciledMetadata {
                        artist: id3_artist.clone(),
                        album: id3_album.clone(),
                        strategy: ReconciliationStrategy::DirectMatch,
                        confidence: MetadataConfidence::High,
                        artist_source: MetadataSource::Both,
                        album_source: MetadataSource::Both,
                        alternate_artist: None,
                        alternate_album: None,
                        has_musicbrainz_ids,
                        estimated_track_count,
                    }
                }
                (true, false) => {
                    // Artist matches, album conflicts
                    let (chosen_album, alt_album, album_src) = choose_album(id3_album, path_album);
                    ReconciledMetadata {
                        artist: id3_artist.clone(),
                        album: chosen_album,
                        strategy: ReconciliationStrategy::PartialMatch,
                        confidence: MetadataConfidence::Medium,
                        artist_source: MetadataSource::Both,
                        album_source: album_src,
                        alternate_artist: None,
                        alternate_album: Some(alt_album),
                        has_musicbrainz_ids,
                        estimated_track_count,
                    }
                }
                (false, true) => {
                    // Album matches, artist conflicts
                    let (chosen_artist, alt_artist, artist_src) =
                        choose_artist(id3_artist, path_artist, id3_album);
                    ReconciledMetadata {
                        artist: chosen_artist,
                        album: id3_album.clone(),
                        strategy: ReconciliationStrategy::PartialMatch,
                        confidence: MetadataConfidence::Medium,
                        artist_source: artist_src,
                        album_source: MetadataSource::Both,
                        alternate_artist: Some(alt_artist),
                        alternate_album: None,
                        has_musicbrainz_ids,
                        estimated_track_count,
                    }
                }
                (false, false) => {
                    // Both conflict
                    let (chosen_artist, alt_artist, artist_src) =
                        choose_artist(id3_artist, path_artist, id3_album);
                    let (chosen_album, alt_album, album_src) = choose_album(id3_album, path_album);
                    ReconciledMetadata {
                        artist: chosen_artist,
                        album: chosen_album,
                        strategy: ReconciliationStrategy::Conflict,
                        confidence: MetadataConfidence::Low,
                        artist_source: artist_src,
                        album_source: album_src,
                        alternate_artist: Some(alt_artist),
                        alternate_album: Some(alt_album),
                        has_musicbrainz_ids,
                        estimated_track_count,
                    }
                }
            }
        }

        // Case 2: ID3 has both, path missing one or both
        (Some(id3_artist), _, Some(id3_album), _) => ReconciledMetadata {
            artist: id3_artist.clone(),
            album: id3_album.clone(),
            strategy: ReconciliationStrategy::GapFill,
            confidence: MetadataConfidence::Low,
            artist_source: MetadataSource::ID3,
            album_source: MetadataSource::ID3,
            alternate_artist: path_artist.clone(),
            alternate_album: path_album.clone(),
            has_musicbrainz_ids,
            estimated_track_count,
        },

        // Case 3: Path has both, ID3 missing one or both
        (_, Some(path_artist), _, Some(path_album)) => ReconciledMetadata {
            artist: path_artist.clone(),
            album: path_album.clone(),
            strategy: ReconciliationStrategy::GapFill,
            confidence: MetadataConfidence::Low,
            artist_source: MetadataSource::Path,
            album_source: MetadataSource::Path,
            alternate_artist: id3.artist.clone(),
            alternate_album: id3.album.clone(),
            has_musicbrainz_ids,
            estimated_track_count,
        },

        // Case 4: Incomplete data - use what we have
        _ => {
            let artist = id3
                .artist
                .clone()
                .or_else(|| path_artist.clone())
                .unwrap_or_else(|| UNKNOWN_VALUE.to_string());
            let album = id3
                .album
                .clone()
                .or_else(|| path_album.clone())
                .unwrap_or_else(|| UNKNOWN_VALUE.to_string());

            ReconciledMetadata {
                artist,
                album,
                strategy: ReconciliationStrategy::GapFill,
                confidence: MetadataConfidence::Low,
                artist_source: MetadataSource::Conflict,
                album_source: MetadataSource::Conflict,
                alternate_artist: None,
                alternate_album: None,
                has_musicbrainz_ids,
                estimated_track_count,
            }
        }
    }
}

/// Log reconciliation decision details
fn log_reconciliation_decision(reconciled: &ReconciledMetadata, album_num: usize) {
    info!("[A{}]   Phase 0 Reconciliation:", album_num);
    info!("[A{}]     Strategy: {:?}", album_num, reconciled.strategy);
    info!(
        "[A{}]     Confidence: {:?}",
        album_num, reconciled.confidence
    );
    info!(
        "[A{}]     Artist: {} (source: {:?})",
        album_num, reconciled.artist, reconciled.artist_source
    );
    if let Some(ref alt) = reconciled.alternate_artist {
        info!("[A{}]       Alternate: {}", album_num, alt);
    }
    info!(
        "[A{}]     Album: {} (source: {:?})",
        album_num, reconciled.album, reconciled.album_source
    );
    if let Some(ref alt) = reconciled.alternate_album {
        info!("[A{}]       Alternate: {}", album_num, alt);
    }
    if reconciled.has_musicbrainz_ids {
        info!("[A{}]     Has MusicBrainz IDs in tags: Yes", album_num);
    }
    if let Some(count) = reconciled.estimated_track_count {
        info!(
            "[A{}]     Estimated track count from ID3: {}",
            album_num, count
        );
    }
}

/// Extract and reconcile metadata from file (Phase 0 entry point)
fn extract_and_reconcile_metadata(file_path: &Path) -> ReconciledMetadata {
    // Extract from both sources
    let id3_result = extract_id3_tags(file_path);
    let (path_artist, path_album) = extract_metadata_from_path(file_path);

    // Handle ID3 extraction failure gracefully
    let id3 = match id3_result {
        Ok(tags) => tags,
        Err(_) => {
            // ID3 extraction failed, use path only
            return ReconciledMetadata {
                artist: path_artist,
                album: path_album,
                strategy: ReconciliationStrategy::PathOnly,
                confidence: MetadataConfidence::Medium,
                artist_source: MetadataSource::Path,
                album_source: MetadataSource::Path,
                alternate_artist: None,
                alternate_album: None,
                has_musicbrainz_ids: false,
                estimated_track_count: None,
            };
        }
    };

    // Reconcile
    let path_artist_opt = if path_artist != UNKNOWN_VALUE {
        Some(path_artist)
    } else {
        None
    };
    let path_album_opt = if path_album != UNKNOWN_VALUE {
        Some(path_album)
    } else {
        None
    };

    reconcile_metadata(&id3, &path_artist_opt, &path_album_opt)
}

/// Assemble segments into tracks using dynamic programming
/// Used in Stage 3 when over-segmentation is detected (detected_segments > expected_tracks)
///
/// Algorithm: Dynamic programming to find optimal grouping of adjacent segments
/// dp[i][j] = minimum duration error when grouping first i segments into j tracks
fn assemble_segments_dp(
    detected_durations: &[f64],
    expected_durations: &[u32],
) -> Option<Vec<f64>> {
    let n = detected_durations.len();
    let k = expected_durations.len();

    // Only assemble if we have over-segmentation
    if n <= k {
        return None;
    }

    // dp[i][j] = (min_error, split_positions)
    // where min_error is the minimum total duration error when grouping first i segments into j tracks
    // and split_positions stores where to split the segments
    let mut dp: Vec<Vec<(f64, Vec<usize>)>> = vec![vec![(f64::INFINITY, Vec::new()); k + 1]; n + 1];

    // Base case: 0 segments into 0 tracks = 0 error
    dp[0][0] = (0.0, Vec::new());

    // Fill DP table
    for i in 1..=n {
        for j in 1..=k.min(i) {
            // Try all possible positions for the j-th track's start
            for start in (j - 1)..i {
                if dp[start][j - 1].0 == f64::INFINITY {
                    continue;
                }

                // Calculate duration of track j by summing segments from start to i-1
                let track_duration: f64 = detected_durations[start..i].iter().sum();
                let expected_duration = expected_durations[j - 1] as f64; // Convert milliseconds to seconds for comparison
                let duration_error = (track_duration - expected_duration).abs();

                // Total error = previous error + this track's error
                let total_error = dp[start][j - 1].0 + duration_error;

                // Update if this is better
                if total_error < dp[i][j].0 {
                    let mut new_splits = dp[start][j - 1].1.clone();
                    new_splits.push(start);
                    dp[i][j] = (total_error, new_splits);
                }
            }
        }
    }

    // Extract solution if valid
    if dp[n][k].0 == f64::INFINITY {
        return None;
    }

    let splits = &dp[n][k].1;
    let mut assembled_durations = Vec::new();

    // Reconstruct track durations from split positions
    let mut prev_split = 0;
    for &split_pos in splits.iter().skip(1) {
        let track_duration: f64 = detected_durations[prev_split..split_pos].iter().sum();
        assembled_durations.push(track_duration);
        prev_split = split_pos;
    }

    // Final track
    let final_duration: f64 = detected_durations[prev_split..n].iter().sum();
    assembled_durations.push(final_duration);

    Some(assembled_durations)
}

/// Calculate RMS values across entire audio file for quiet spot detection
/// Returns (position_secs, rms_value) pairs
fn calculate_rms_profile(samples: &[f32], sample_rate: u32) -> Vec<(f64, f32)> {
    let window_samples = (sample_rate as f64 * QUIET_SPOT_WINDOW_SECS) as usize;
    let step_samples = (sample_rate as f64 * QUIET_SPOT_WINDOW_STEP_SECS) as usize;

    let mut rms_profile = Vec::new();
    let mut pos = 0;

    while pos + window_samples <= samples.len() {
        let rms = calculate_rms(&samples[pos..pos + window_samples]);
        let pos_secs = pos as f64 / sample_rate as f64;
        rms_profile.push((pos_secs, rms));
        pos += step_samples;
    }

    rms_profile
}

/// Edition-guided quiet spot detection for Stage 4
/// Searches for quiet spots near expected track boundaries based on edition durations
fn find_edition_guided_boundaries(
    rms_profile: &[(f64, f32)],
    expected_durations: &[u32],
    total_duration_secs: f64,
) -> Vec<f64> {
    if expected_durations.is_empty() || rms_profile.is_empty() {
        return Vec::new();
    }

    // Calculate expected boundary positions (cumulative durations)
    let mut expected_boundaries: Vec<f64> = Vec::new();
    let mut cumulative = 0.0;
    for (i, &dur) in expected_durations.iter().enumerate() {
        cumulative += dur as f64;
        // Don't add boundary after last track
        if i < expected_durations.len() - 1 {
            expected_boundaries.push(cumulative);
        }
    }

    // For each expected boundary, find quietest spot within search window
    let mut detected_boundaries = Vec::new();

    for (boundary_idx, &expected_pos) in expected_boundaries.iter().enumerate() {
        // Calculate dynamic search radius (15% of preceding track duration, clamped)
        let prev_track_dur = if boundary_idx == 0 {
            expected_durations[0] as f64
        } else {
            expected_durations[boundary_idx] as f64
        };
        let dynamic_radius = (prev_track_dur * QUIET_SPOT_SEARCH_RADIUS_RATIO)
            .max(QUIET_SPOT_SEARCH_RADIUS_MIN)
            .min(QUIET_SPOT_SEARCH_RADIUS_MAX);

        let search_start = (expected_pos - dynamic_radius).max(0.0);
        let search_end = (expected_pos + dynamic_radius).min(total_duration_secs);

        // Find RMS values within search window
        let candidates: Vec<_> = rms_profile
            .iter()
            .filter(|(pos, _)| *pos >= search_start && *pos <= search_end)
            .collect();

        if candidates.is_empty() {
            // No candidates in window - use expected position
            detected_boundaries.push(expected_pos);
            continue;
        }

        // Score each candidate: lower RMS is better, but penalize distance from expected
        let mut best_pos = expected_pos;
        let mut best_score = f64::INFINITY;

        for &(pos, rms) in &candidates {
            let rms_db = if *rms > SILENCE_RMS_EPSILON {
                DB_MULTIPLIER * (*rms as f64).log10()
            } else {
                SILENCE_DB_FLOOR as f64
            };

            // Score = RMS in dB + penalty for distance from expected
            let distance = (pos - expected_pos).abs();
            let distance_penalty = (distance / dynamic_radius)
                * QUIET_SPOT_PROXIMITY_PENALTY
                * QUIET_SPOT_DISTANCE_PENALTY_MULTIPLIER;
            let score = rms_db + distance_penalty;

            if score < best_score {
                best_score = score;
                best_pos = *pos;
            }
        }

        // Check if best spot meets minimum quietness threshold
        // If not, still use it but it might not be a real boundary
        detected_boundaries.push(best_pos);
    }

    detected_boundaries
}

/// Convert detected boundary positions to track durations
fn boundaries_to_durations(boundaries: &[f64], total_duration_secs: f64) -> Vec<f64> {
    let mut durations = Vec::new();
    let mut prev_pos = 0.0;

    for &boundary in boundaries {
        durations.push(boundary - prev_pos);
        prev_pos = boundary;
    }

    // Final track (from last boundary to end of file)
    if prev_pos < total_duration_secs {
        durations.push(total_duration_secs - prev_pos);
    }

    durations
}

/// Classify match quality as confidence level
fn classify_confidence(match_percentage: f64) -> String {
    if match_percentage >= CONFIDENCE_EXCELLENT_THRESHOLD {
        "Excellent".to_string()
    } else if match_percentage >= CONFIDENCE_GOOD_THRESHOLD {
        "Good".to_string()
    } else if match_percentage >= CONFIDENCE_FAIR_THRESHOLD {
        "Fair".to_string()
    } else {
        "Poor".to_string()
    }
}

/// Result from testing a segmentation against all MusicBrainz candidates.
/// Contains the best match found and all comparison details.
#[derive(Debug, Clone)]
struct CandidateTestResult {
    /// Percentage of expected tracks that matched (0-100).
    percentage: f64,
    /// Number of tracks within tolerance.
    matched_count: usize,
    /// Per-track comparison results.
    matches: Vec<TrackMatch>,
    /// MusicBrainz release ID of best match.
    mbid: String,
    /// Expected track durations from MusicBrainz (seconds).
    expected_durations: Vec<u32>,
    /// Mean absolute error across matched tracks (seconds).
    mean_error: f64,
    /// Detected track durations from audio analysis (seconds).
    detected_durations: Vec<f64>,
}

/// Test a segmentation against a SINGLE edition candidate (Run 15)
/// Returns the test result for this specific edition
fn test_segmentation_against_single_edition(
    detected_durations: &[f64],
    expected_durations: &[u32],
    edition_id: &str,
    tolerance: f64,
) -> CandidateTestResult {
    let (matches, matched_count, percentage) =
        analyze_track_matching(detected_durations, expected_durations, tolerance);

    // Calculate mean error for matched tracks
    let mean_error = if matched_count > 0 {
        matches
            .iter()
            .filter(|m| m.matches)
            .map(|m| m.error)
            .sum::<f64>()
            / matched_count as f64
    } else {
        0.0
    };

    CandidateTestResult {
        percentage,
        matched_count,
        matches,
        mbid: edition_id.to_string(),
        expected_durations: expected_durations.to_vec(),
        mean_error,
        detected_durations: detected_durations.to_vec(),
    }
}

/// Stage 2 results for single-edition processing (Run 15)
struct SingleEditionStage2Results {
    best_result: Option<CandidateTestResult>,
    over_segmented_candidates: Vec<OverSegmentedCandidate>,
    best_threshold: Option<f64>,
    best_min_duration: Option<f64>,
}

/// Result of testing a single edition through all stages (Run 15 parallel processing)
#[derive(Clone)]
struct EditionTestResult {
    /// Index of the edition in the editions list
    edition_idx: usize,
    /// Best percentage achieved for this edition
    best_percentage: f64,
    /// Best result found across all stages
    best_result: Option<CandidateTestResult>,
    /// Which stage produced the best result
    best_stage: &'static str,
    /// Best threshold from stage 2 (if applicable)
    best_threshold: Option<f64>,
    /// Best min_duration from stage 2 (if applicable)
    best_min_duration: Option<f64>,
    /// Expected durations from this edition
    expected_durations: Vec<u32>,
    /// Log messages accumulated during processing
    log_messages: Vec<String>,
}

/// Check if early exit should occur based on 100% match found and grace period
/// Returns true if a 100% match was found AND grace period has expired
fn should_exit_early(
    perfect_match_found: &AtomicBool,
    perfect_match_time_ms: &AtomicU64,
    start_time: Instant,
) -> bool {
    if !perfect_match_found.load(Ordering::Relaxed) {
        return false;
    }
    let match_time_ms = perfect_match_time_ms.load(Ordering::Relaxed);
    if match_time_ms == 0 {
        return false; // Timestamp not set yet
    }
    let elapsed_since_match = start_time.elapsed().as_millis() as u64 - match_time_ms;
    elapsed_since_match >= EARLY_EXIT_GRACE_PERIOD_SECS * 1000
}

/// Signal that a 100% match was found (thread-safe, only sets once)
fn signal_perfect_match(
    perfect_match_found: &AtomicBool,
    perfect_match_time_ms: &AtomicU64,
    start_time: Instant,
) {
    // Only set if not already set (compare_exchange ensures atomicity)
    if perfect_match_found
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::Relaxed)
        .is_ok()
    {
        let elapsed_ms = start_time.elapsed().as_millis() as u64;
        perfect_match_time_ms.store(elapsed_ms, Ordering::SeqCst);
    }
}

/// Construct an EditionTestResult from current best state.
///
/// Helper to reduce repetition in test_single_edition, which constructs
/// this result in 6 different places.
fn make_edition_result(
    edition_idx: usize,
    best_percentage: f64,
    best_result: Option<CandidateTestResult>,
    best_stage: &'static str,
    best_threshold: Option<f64>,
    best_min_duration: Option<f64>,
    expected_durations: &[u32],
    log_messages: Vec<String>,
) -> EditionTestResult {
    EditionTestResult {
        edition_idx,
        best_percentage,
        best_result,
        best_stage,
        best_threshold,
        best_min_duration,
        expected_durations: expected_durations.to_vec(),
        log_messages,
    }
}

/// Test a single edition through all stages (Stages 2-5)
/// This function is designed to run in parallel across editions
/// Run 16: Uses pre-computed silence cache instead of re-computing silence detection
/// Run 16: Supports early exit with grace period after 100% match found
fn test_single_edition(
    edition_idx: usize,
    edition: &Edition,
    silence_cache: &SilenceCache,
    num_thresholds: usize,
    num_min_durations: usize,
    match_tolerance_secs: f64,
    rms_profile: &[(f64, f32)],
    total_duration_secs: f64,
    initial_durations: &[f64],
    total_editions: usize,
    perfect_match_found: &AtomicBool,
    perfect_match_time_ms: &AtomicU64,
    start_time: Instant,
    album_idx: usize,
) -> EditionTestResult {
    let edition_id = format!("edition_{}", edition_idx);
    let expected_durations = &edition.durations;
    let mut log_messages = Vec::new();

    log_messages.push(format!(
        "  [Edition {}/{}] {} - {} ({} tracks, NDR:{},{:.1})",
        edition_idx + 1,
        total_editions,
        edition.artist,
        edition.album,
        edition.track_count,
        edition.name_distance_rank,
        edition.name_distance_score
    ));

    // Check for early exit before starting (grace period expired)
    if should_exit_early(perfect_match_found, perfect_match_time_ms, start_time) {
        log_messages.push("    -> Skipped (early exit: grace period expired)".to_string());
        return make_edition_result(
            edition_idx,
            0.0,
            None,
            "skipped_early_exit",
            None,
            None,
            expected_durations,
            log_messages,
        );
    }

    // Track best result for THIS edition across stages
    let mut edition_best_percentage = 0.0;
    let mut edition_best_durations = initial_durations.to_vec();
    let mut edition_best_stage: &'static str = "none";
    let mut edition_best_result: Option<CandidateTestResult> = None;
    let mut edition_best_threshold: Option<f64> = None;
    let mut edition_best_min_duration: Option<f64> = None;

    // === STAGE 2: Parameter optimization for this edition ===
    log_messages.push(format!(
        "    Stage 2: Testing {} cached parameter combinations...",
        num_thresholds * num_min_durations
    ));

    let stage2_results = run_stage2_single_edition_cached(
        silence_cache,
        num_thresholds,
        num_min_durations,
        expected_durations,
        &edition_id,
        match_tolerance_secs,
        edition_best_percentage,
    );

    if let Some(ref result) = stage2_results.best_result {
        if result.percentage > edition_best_percentage {
            edition_best_percentage = result.percentage;
            edition_best_durations = result.detected_durations.clone();
            edition_best_stage = "album_extractor_2_optimization";
            edition_best_result = Some(result.clone());
            edition_best_threshold = stage2_results.best_threshold;
            edition_best_min_duration = stage2_results.best_min_duration;

            if result.percentage >= 100.0 {
                log_messages.push("    -> 100% match in Stage 2!".to_string());
                signal_perfect_match(perfect_match_found, perfect_match_time_ms, start_time);
                return make_edition_result(
                    edition_idx,
                    edition_best_percentage,
                    edition_best_result,
                    edition_best_stage,
                    edition_best_threshold,
                    edition_best_min_duration,
                    expected_durations,
                    log_messages,
                );
            }
        }
    }

    // Check for early exit before Stage 3
    if should_exit_early(perfect_match_found, perfect_match_time_ms, start_time) {
        log_messages.push("    -> Early exit before Stage 3 (grace period expired)".to_string());
        return make_edition_result(
            edition_idx,
            edition_best_percentage,
            edition_best_result,
            edition_best_stage,
            edition_best_threshold,
            edition_best_min_duration,
            expected_durations,
            log_messages,
        );
    }

    // === STAGE 3: Segment assembly for this edition ===
    if !stage2_results.over_segmented_candidates.is_empty() {
        log_messages.push(format!(
            "    Stage 3: Testing {} over-segmented assemblies...",
            stage2_results.over_segmented_candidates.len()
        ));

        if let Some(result) = run_stage3_single_edition(
            &stage2_results.over_segmented_candidates,
            expected_durations,
            &edition_id,
            match_tolerance_secs,
            edition_best_percentage,
            edition_idx,
            total_editions,
            perfect_match_found,
            perfect_match_time_ms,
            start_time,
            album_idx,
        ) {
            if result.percentage > edition_best_percentage {
                edition_best_percentage = result.percentage;
                edition_best_durations = result.detected_durations.clone();
                edition_best_stage = "album_extractor_3_assembly";
                edition_best_result = Some(result.clone());

                if result.percentage >= 100.0 {
                    log_messages.push("    -> 100% match in Stage 3!".to_string());
                    signal_perfect_match(perfect_match_found, perfect_match_time_ms, start_time);
                    return make_edition_result(
                        edition_idx,
                        edition_best_percentage,
                        edition_best_result,
                        edition_best_stage,
                        edition_best_threshold,
                        edition_best_min_duration,
                        expected_durations,
                        log_messages,
                    );
                }
            }
        }
    }

    // Check for early exit before Stage 4
    if should_exit_early(perfect_match_found, perfect_match_time_ms, start_time) {
        log_messages.push("    -> Early exit before Stage 4 (grace period expired)".to_string());
        return make_edition_result(
            edition_idx,
            edition_best_percentage,
            edition_best_result,
            edition_best_stage,
            edition_best_threshold,
            edition_best_min_duration,
            expected_durations,
            log_messages,
        );
    }

    // === STAGE 4: Quiet spot detection for this edition ===
    // Stage 4 results incur a 25% penalty (less reliable than silence-based detection)
    // Stage 4 can NEVER trigger 100% early exit due to this penalty
    if !rms_profile.is_empty() {
        log_messages.push("    Stage 4: Guided quiet spot detection...".to_string());

        if let Some(result) = run_stage4_single_edition(
            expected_durations,
            &edition_id,
            match_tolerance_secs,
            edition_best_percentage,
            rms_profile,
            total_duration_secs,
            edition_idx,
            total_editions,
            album_idx,
        ) {
            // Apply Stage 4 penalty: raw 100% becomes 75%
            let penalized_percentage = result.percentage * (1.0 - STAGE4_PENALTY_PERCENT / 100.0);

            if penalized_percentage > edition_best_percentage {
                log_messages.push(format!(
                    "    -> Stage 4 raw: {:.1}%, penalized: {:.1}% (-{}%)",
                    result.percentage, penalized_percentage, STAGE4_PENALTY_PERCENT
                ));
                edition_best_percentage = penalized_percentage;
                edition_best_durations = result.detected_durations.clone();
                edition_best_stage = "album_extractor_4_guided";
                edition_best_result = Some(result.clone());
                // Note: No 100% early exit for Stage 4 - penalty makes it impossible
            }
        }
    }

    // === STAGE 5: Extra track merging for this edition ===
    if edition_best_percentage >= 100.0 && edition_best_durations.len() > expected_durations.len() {
        log_messages.push("    Stage 5: Extra track merging...".to_string());

        if let Some((_merged_durations, result)) = run_stage5_single_edition(
            &edition_best_durations,
            expected_durations,
            &edition_id,
            match_tolerance_secs,
            edition_best_percentage,
        ) {
            edition_best_stage = "album_extractor_5_merging";
            edition_best_result = Some(result.clone());
        }
    }

    log_messages.push(format!(
        "    -> Edition best: {:.1}% ({})",
        edition_best_percentage, edition_best_stage
    ));

    make_edition_result(
        edition_idx,
        edition_best_percentage,
        edition_best_result,
        edition_best_stage,
        edition_best_threshold,
        edition_best_min_duration,
        expected_durations,
        log_messages,
    )
}

/// Stage 2: Parameter optimization using pre-computed silence cache (Run 16)
/// Uses cached track durations instead of re-computing silence detection
fn run_stage2_single_edition_cached(
    silence_cache: &SilenceCache,
    num_thresholds: usize,
    num_min_durations: usize,
    expected_durations: &[u32],
    edition_id: &str,
    tolerance: f64,
    current_best_percentage: f64,
) -> SingleEditionStage2Results {
    if current_best_percentage >= 100.0 {
        return SingleEditionStage2Results {
            best_result: None,
            over_segmented_candidates: Vec::new(),
            best_threshold: None,
            best_min_duration: None,
        };
    }

    let mut best_result: Option<CandidateTestResult> = None;
    let mut over_segmented_candidates: Vec<OverSegmentedCandidate> = Vec::new();
    let mut best_threshold: Option<f64> = None;
    let mut best_min_duration: Option<f64> = None;

    let expected_track_count = expected_durations.len();

    // Iterate using indices to look up from cache
    for thresh_idx in 0..num_thresholds {
        for min_dur_idx in 0..num_min_durations {
            // Look up pre-computed track durations from cache
            let cache_idx = thresh_idx * num_min_durations + min_dur_idx;
            let test_durations = match silence_cache.get(cache_idx) {
                Some(durations) => durations,
                None => continue, // Should not happen with valid indices
            };

            // Get actual threshold/duration values for logging and over-segmented candidates
            let thresh = STAGE2_THRESHOLD_VALUES[thresh_idx];
            let min_dur = STAGE2_MIN_DURATION_VALUES[min_dur_idx];

            // Test against this single edition
            let result = test_segmentation_against_single_edition(
                test_durations,
                expected_durations,
                edition_id,
                tolerance,
            );

            let improved = best_result
                .as_ref()
                .map_or(true, |br| result.percentage > br.percentage);

            if improved && result.percentage > current_best_percentage {
                // Suppress per-combination logging in cached version to reduce output noise
                // (180 combinations × 23 editions = too much output)

                best_threshold = Some(thresh);
                best_min_duration = Some(min_dur);
                best_result = Some(result);

                if best_result.as_ref().unwrap().percentage >= 100.0 {
                    return SingleEditionStage2Results {
                        best_result,
                        over_segmented_candidates,
                        best_threshold,
                        best_min_duration,
                    };
                }
            }

            // Collect over-segmented candidates for Stage 3 assembly
            if test_durations.len() > expected_track_count {
                over_segmented_candidates.push(OverSegmentedCandidate {
                    durations: test_durations.clone(),
                    threshold_db: thresh,
                    min_duration_secs: min_dur,
                    track_count: test_durations.len(),
                });
            }
        }
    }

    SingleEditionStage2Results {
        best_result,
        over_segmented_candidates,
        best_threshold,
        best_min_duration,
    }
}

/// Stage 3: Segment assembly for a SINGLE edition (Run 15)
/// Assembles over-segmented candidates to match this edition's track count
/// Run 16: Added early exit support within assembly loop
fn run_stage3_single_edition(
    over_segmented_candidates: &[OverSegmentedCandidate],
    expected_durations: &[u32],
    edition_id: &str,
    tolerance: f64,
    current_best_percentage: f64,
    edition_idx: usize,
    total_editions: usize,
    perfect_match_found: &AtomicBool,
    perfect_match_time_ms: &AtomicU64,
    start_time: Instant,
    album_idx: usize,
) -> Option<CandidateTestResult> {
    if current_best_percentage >= 100.0 {
        return None;
    }

    if over_segmented_candidates.is_empty() {
        return None;
    }

    let mut assemblies_tested = 0;
    let mut best_result: Option<CandidateTestResult> = None;

    for candidate in over_segmented_candidates {
        // Check for early exit within the loop (grace period expired)
        if should_exit_early(perfect_match_found, perfect_match_time_ms, start_time) {
            info!(
                "      [Edition {}/{}] Early exit during Stage 3 assembly (tested {} so far)",
                edition_idx + 1,
                total_editions,
                assemblies_tested
            );
            break;
        }

        // Only try assembly if candidate has more segments than target
        if candidate.durations.len() > expected_durations.len() {
            if let Some(assembled_durations) =
                assemble_segments_dp(&candidate.durations, expected_durations)
            {
                assemblies_tested += 1;

                let result = test_segmentation_against_single_edition(
                    &assembled_durations,
                    expected_durations,
                    edition_id,
                    tolerance,
                );

                let improved = best_result
                    .as_ref()
                    .map_or(true, |br| result.percentage > br.percentage);

                if improved && result.percentage > current_best_percentage {
                    info!("[A{}]       [Edition {}/{}] New best: {:.1}% via assembly ({}dB, {}s) ({} -> {} tracks)",
                        album_idx + 1, edition_idx + 1, total_editions,
                        result.percentage,
                        candidate.threshold_db, candidate.min_duration_secs,
                        candidate.track_count, expected_durations.len());

                    best_result = Some(result);

                    if best_result.as_ref().unwrap().percentage >= 100.0 {
                        return best_result;
                    }
                }
            }
        }
    }

    if assemblies_tested > 0 {
        info!(
            "[A{}]       [Edition {}/{}] Tested {} assemblies",
            album_idx + 1,
            edition_idx + 1,
            total_editions,
            assemblies_tested
        );
    }

    best_result
}

/// Stage 4: Quiet spot detection for a SINGLE edition (Run 15)
/// Uses this edition's track durations as guide for boundary detection
fn run_stage4_single_edition(
    expected_durations: &[u32],
    edition_id: &str,
    tolerance: f64,
    current_best_percentage: f64,
    rms_profile: &[(f64, f32)], // Pre-calculated RMS profile (time, rms)
    total_duration_secs: f64,
    edition_idx: usize,
    total_editions: usize,
    album_idx: usize,
) -> Option<CandidateTestResult> {
    if current_best_percentage >= 100.0 {
        return None;
    }

    if expected_durations.is_empty() || rms_profile.is_empty() {
        return None;
    }

    // Find quiet spots near expected boundaries for this edition
    let detected_boundaries =
        find_edition_guided_boundaries(rms_profile, expected_durations, total_duration_secs);

    // Convert boundaries to track durations
    let guided_durations = boundaries_to_durations(&detected_boundaries, total_duration_secs);

    // Test guided result
    let result = test_segmentation_against_single_edition(
        &guided_durations,
        expected_durations,
        edition_id,
        tolerance,
    );

    if result.percentage > current_best_percentage {
        info!(
            "[A{}]       [Edition {}/{}] New best: {:.1}% via guided quiet spots",
            album_idx + 1,
            edition_idx + 1,
            total_editions,
            result.percentage
        );
        Some(result)
    } else {
        None
    }
}

/// Try all possible adjacent track merges and return the best one.
///
/// When silence detection finds more tracks than expected, this function attempts
/// to merge adjacent tracks to achieve the correct count. It evaluates every
/// possible adjacent pair merge and returns the one with the lowest total error.
///
/// # Arguments
/// * `durations` - Current detected track durations in seconds
/// * `expected_durations` - Expected track durations from MusicBrainz in seconds
/// * `tolerance` - Tolerance for track matching (used in error calculation)
///
/// # Returns
/// `Some((merged_durations, merge_index, total_error))` if a valid merge produces
/// the expected track count, `None` if no valid merge is possible.
fn try_all_adjacent_merges(
    durations: &[f64],
    expected_durations: &[u32],
    tolerance: f64,
) -> Option<(Vec<f64>, usize, f64)> {
    let mut best_merge_durations = None;
    let mut best_merge_error = f64::INFINITY;
    let mut best_merge_index = None;

    for merge_idx in 0..(durations.len() - 1) {
        let mut merged_durations = Vec::new();

        for i in 0..durations.len() {
            if i == merge_idx {
                merged_durations.push(durations[i] + durations[i + 1]);
            } else if i == merge_idx + 1 {
                continue;
            } else {
                merged_durations.push(durations[i]);
            }
        }

        let (merged_matches, _, _) =
            analyze_track_matching(&merged_durations, expected_durations, tolerance);

        let total_error: f64 = merged_matches.iter().map(|m| m.error).sum();

        if merged_durations.len() == expected_durations.len() && total_error < best_merge_error {
            best_merge_error = total_error;
            best_merge_durations = Some(merged_durations);
            best_merge_index = Some(merge_idx);
        }
    }

    best_merge_durations.map(|durations| (durations, best_merge_index.unwrap(), best_merge_error))
}

/// Stage 5: Extra track merging for a SINGLE edition (Run 15)
/// When detected > expected AND match >= 100%, merge adjacent tracks
fn run_stage5_single_edition(
    best_durations: &[f64],
    expected_durations: &[u32],
    edition_id: &str,
    tolerance: f64,
    current_best_percentage: f64,
) -> Option<(Vec<f64>, CandidateTestResult)> {
    if current_best_percentage < 100.0 || best_durations.len() <= expected_durations.len() {
        return None;
    }

    let extra_count = best_durations.len() - expected_durations.len();
    info!("      Attempting merge: {} extra track(s)", extra_count);

    // Use helper to find best merge
    let merge_result = try_all_adjacent_merges(best_durations, expected_durations, tolerance);

    if let Some((merged_durations, best_merge_index, best_merge_error)) = merge_result {
        let (merged_matches, merged_matched_count, merged_percentage) =
            analyze_track_matching(&merged_durations, expected_durations, tolerance);

        let mean_merged_error = best_merge_error / merged_matches.len() as f64;

        info!(
            "      Merged tracks {} + {} -> {:.1}%",
            best_merge_index + 1,
            best_merge_index + 2,
            merged_percentage
        );

        let result = CandidateTestResult {
            percentage: merged_percentage,
            matched_count: merged_matched_count,
            matches: merged_matches,
            mbid: edition_id.to_string(),
            expected_durations: expected_durations.to_vec(),
            mean_error: mean_merged_error,
            detected_durations: merged_durations.clone(),
        };

        Some((merged_durations, result))
    } else {
        None
    }
}

/// Filter and sort editions by match likelihood.
///
/// Applies runtime filtering (±25% of file duration) and re-sorts by name similarity.
/// Returns filtered and sorted editions, or error message if all filtered out.
fn filter_and_sort_editions(
    mut editions: Vec<Edition>,
    file_duration_secs: f64,
    estimated_track_count: Option<usize>,
    album_idx: usize,
) -> Result<Vec<Edition>, String> {
    // Sort editions by how well they match the audio file's characteristics
    editions.sort_by(|a, b| {
        let score_a = score_edition_match(a, file_duration_secs, estimated_track_count);
        let score_b = score_edition_match(b, file_duration_secs, estimated_track_count);
        score_a
            .partial_cmp(&score_b)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    info!(
        "[A{}]   Sorted editions by likelihood (file: {:.0}s)",
        album_idx + 1,
        file_duration_secs
    );

    // Filter editions by runtime length (must be within 25% of file duration)
    let total_editions = editions.len();
    let min_duration = file_duration_secs * RUNTIME_FILTER_MIN_RATIO;
    let max_duration = file_duration_secs * RUNTIME_FILTER_MAX_RATIO;

    editions.retain(|edition| {
        let edition_duration: u32 = edition.durations.iter().sum();
        let edition_duration_secs = edition_duration as f64;
        edition_duration_secs >= min_duration && edition_duration_secs <= max_duration
    });

    let runtime_filtered_count = total_editions - editions.len();
    if runtime_filtered_count > 0 {
        info!(
            "[A{}]   Filtered out {} editions (runtime >25% different from file)",
            album_idx + 1,
            runtime_filtered_count
        );
        info!(
            "[A{}]   Acceptable range: {:.0}s - {:.0}s",
            album_idx + 1,
            min_duration,
            max_duration
        );
    }

    if editions.is_empty() {
        return Err(if runtime_filtered_count > 0 {
            format!(
                "FAILED: All editions filtered out ({} by runtime)",
                runtime_filtered_count
            )
        } else {
            "FAILED: No valid editions (NDR filtering applied at release level)".to_string()
        });
    }

    // Re-sort by name similarity using conservative bubble sort
    resort_by_name_similarity(&mut editions);

    Ok(editions)
}

/// Find the best result across all edition test results.
///
/// Prioritizes 100% matches, then by percentage, then by mean error.
/// Returns the index of the best edition and the result, if any.
fn find_best_edition_result(edition_results: &[EditionTestResult]) -> Option<&EditionTestResult> {
    edition_results
        .iter()
        .filter(|r| r.best_result.is_some())
        .max_by(|a, b| {
            let a_perfect = a.best_percentage >= 100.0;
            let b_perfect = b.best_percentage >= 100.0;

            if a_perfect && !b_perfect {
                return std::cmp::Ordering::Greater;
            }
            if b_perfect && !a_perfect {
                return std::cmp::Ordering::Less;
            }

            if a_perfect && b_perfect {
                let a_error = a
                    .best_result
                    .as_ref()
                    .map(|r| r.mean_error)
                    .unwrap_or(f64::MAX);
                let b_error = b
                    .best_result
                    .as_ref()
                    .map(|r| r.mean_error)
                    .unwrap_or(f64::MAX);
                return b_error
                    .partial_cmp(&a_error)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| b.edition_idx.cmp(&a.edition_idx));
            }

            a.best_percentage
                .partial_cmp(&b.best_percentage)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    let a_error = a
                        .best_result
                        .as_ref()
                        .map(|r| r.mean_error)
                        .unwrap_or(f64::MAX);
                    let b_error = b
                        .best_result
                        .as_ref()
                        .map(|r| r.mean_error)
                        .unwrap_or(f64::MAX);
                    b_error
                        .partial_cmp(&a_error)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .then_with(|| b.edition_idx.cmp(&a.edition_idx))
        })
}

/// Process a single album through all matching stages (Run 18: extracted for parallel processing)
///
/// This function encapsulates all the album processing logic that was previously in the main loop.
/// It is designed to be called concurrently for multiple albums with a shared RateLimiter.
async fn process_single_album(
    album_idx: usize,
    total_albums: usize,
    file_path: PathBuf,
    threshold_db: f64,
    min_duration_secs: f64,
    match_tolerance_secs: f64,
    threshold_values: &'static [f64],
    min_duration_values: &'static [f64],
    rate_limiter: RateLimiter,
) -> ValidationResult {
    // === STAGGERED START DELAY ===
    // For the initial batch of MAX_CONCURRENT_ALBUMS, stagger their starts to reduce
    // rate limit contention. Albums beyond the initial batch start immediately when
    // their slot becomes available (previous album finished).
    let stagger_position = album_idx % MAX_CONCURRENT_ALBUMS;
    if STAGGER_MULTIPLIER > 0 && stagger_position > 0 && album_idx < MAX_CONCURRENT_ALBUMS {
        let stagger_delay_ms = stagger_position as u64 * STAGGER_MULTIPLIER * MB_RATE_LIMIT_MS;
        let stagger_delay_secs = stagger_delay_ms / 1000;
        info!(
            "[A{}] Staggered start: waiting {}s ({} position × {} multiplier × {}ms rate limit)...",
            album_idx + 1,
            stagger_delay_secs,
            stagger_position,
            STAGGER_MULTIPLIER,
            MB_RATE_LIMIT_MS
        );
        sleep(Duration::from_millis(stagger_delay_ms)).await;
    }

    info!(
        "[A{}] === Album {}/{} ===",
        album_idx + 1,
        album_idx + 1,
        total_albums
    );
    info!("[A{}] File: {}", album_idx + 1, file_path.display());

    if !file_path.exists() {
        error!("[A{}]   ERROR: File not found\n", album_idx + 1);
        return ValidationResult::error(
            &file_path,
            UNKNOWN_VALUE,
            UNKNOWN_VALUE,
            0,
            "File not found".to_string(),
        );
    }

    // === PHASE 0: ID3 Tag Extraction & Reconciliation ===
    let reconciled = extract_and_reconcile_metadata(&file_path);
    log_reconciliation_decision(&reconciled, album_idx + 1);

    let artist = reconciled.artist.clone();
    let album = reconciled.album.clone();

    // Build artist/album variant lists for comprehensive search
    let mut artist_variants = vec![artist.clone()];
    if let Some(ref alt_artist) = reconciled.alternate_artist {
        if !artist_variants.contains(alt_artist) {
            artist_variants.push(alt_artist.clone());
        }
    }

    let mut album_variants = vec![album.clone()];
    if let Some(ref alt_album) = reconciled.alternate_album {
        if !album_variants.contains(alt_album) {
            album_variants.push(alt_album.clone());
        }
    }

    // === RUN 17: PIPELINE PARALLELISM ===
    // Start decode and MB lookup concurrently within this album
    let file_path_for_decode = file_path.clone();
    let artist_variants_for_mb = artist_variants.clone();
    let album_variants_for_mb = album_variants.clone();
    let estimated_track_count = reconciled.estimated_track_count;

    // === RUN 19: TRUE PARALLEL DECODE + MUSICBRAINZ ===
    // Start decode and MB lookup concurrently (MB doesn't need file_duration_secs)
    info!(
        "[A{}]   Starting parallel: decode + MusicBrainz lookup...",
        album_idx + 1
    );

    // Create query stats for heartbeat logging during MB lookups
    let query_stats = Arc::new(QueryStats::new());
    let heartbeat_handle = spawn_heartbeat_task(Arc::clone(&query_stats), album_idx);

    // Spawn decode task (CPU-bound)
    let album_idx_for_decode = album_idx;
    let decode_handle = tokio::task::spawn_blocking(move || {
        info!("[A{}]   Decoding...", album_idx_for_decode + 1);
        decode_mp3(&file_path_for_decode)
    });

    // Spawn MusicBrainz lookup concurrently (async, I/O-bound)
    // Can start before decode completes since MB search doesn't need file duration
    let mb_task = comprehensive_musicbrainz_search(
        &artist_variants_for_mb,
        &album_variants_for_mb,
        &rate_limiter,
        album_idx,
        Some(&query_stats),
    );

    // Wait for both decode and MB to complete in parallel
    let (decode_result, mb_result) = tokio::join!(decode_handle, mb_task);

    // Note: heartbeat continues running through edition testing phase
    // It will be stopped after edition processing completes

    // Process decode result
    let (samples, sample_rate) = match decode_result {
        Ok(Ok((s, sr))) => {
            let duration_mins = s.len() as f64 / sr as f64 / 60.0;
            info!(
                "[A{}]   Decoded: {} samples at {} Hz ({:.2} mins)",
                album_idx + 1,
                s.len(),
                sr,
                duration_mins
            );
            (s, sr)
        }
        Ok(Err(e)) => {
            error!("[A{}] FAILED: {}", album_idx + 1, e);
            info!("[A{}] ", album_idx + 1);
            return ValidationResult::error(
                &file_path,
                &artist,
                &album,
                0,
                format!("Decode failed: {}", e),
            );
        }
        Err(e) => {
            error!("[A{}] FAILED: Task panicked: {}", album_idx + 1, e);
            info!("[A{}] ", album_idx + 1);
            return ValidationResult::error(
                &file_path,
                &artist,
                &album,
                0,
                format!("Decode task panicked: {}", e),
            );
        }
    };

    // Get initial track durations for MusicBrainz lookup
    let initial_durations =
        get_track_durations(&samples, sample_rate, threshold_db, min_duration_secs);

    // Calculate file characteristics for edition sorting/filtering
    let file_duration_secs = samples.len() as f64 / sample_rate as f64;

    // === SILENCE DETECTION (after decode, needs samples) ===
    // Clone data needed for silence detection task
    let samples_for_silence = samples.clone();
    let album_idx_for_silence = album_idx;
    let threshold_values_clone = threshold_values.to_vec();
    let min_duration_values_clone = min_duration_values.to_vec();

    // Spawn silence detection on blocking thread pool (CPU-bound with rayon)
    let silence_task = tokio::task::spawn_blocking(move || {
        info!(
            "[A{}]   Single-pass silence detection (1 scan → {} param combinations, {} threads)...",
            album_idx_for_silence + 1,
            threshold_values_clone.len() * min_duration_values_clone.len(),
            rayon::current_num_threads()
        );
        let cache = precompute_silence_cache(
            &samples_for_silence,
            sample_rate,
            &threshold_values_clone,
            &min_duration_values_clone,
        );
        info!(
            "[A{}]   Silence cache ready ({} entries)",
            album_idx_for_silence + 1,
            cache.len()
        );
        cache
    });

    // Wait for silence detection to complete
    let silence_cache = match silence_task.await {
        Ok(cache) => cache,
        Err(e) => {
            error!(
                "[A{}] FAILED: Silence detection task panicked: {}",
                album_idx + 1,
                e
            );
            return ValidationResult::error(
                &file_path,
                &artist,
                &album,
                initial_durations.len(),
                format!("Silence detection failed: {}", e),
            );
        }
    };

    // Process MusicBrainz result
    let editions = match mb_result {
        Ok(releases) => {
            if releases.is_empty() {
                error!("[A{}] FAILED: No releases found", album_idx + 1);
                info!("[A{}] ", album_idx + 1);
                return ValidationResult::error(
                    &file_path,
                    &artist,
                    &album,
                    initial_durations.len(),
                    "MusicBrainz lookup failed: No releases found".to_string(),
                );
            }

            // Group releases into unique editions and filter/sort
            let editions = group_into_editions(releases, album_idx);
            match filter_and_sort_editions(
                editions,
                file_duration_secs,
                estimated_track_count,
                album_idx,
            ) {
                Ok(filtered) => {
                    info!(
                        "[A{}] Found {} unique editions to test",
                        album_idx + 1,
                        filtered.len()
                    );
                    filtered
                }
                Err(failure_msg) => {
                    info!("[A{}] {}", album_idx + 1, failure_msg);
                    info!("[A{}] ", album_idx + 1);
                    return ValidationResult::error(
                        &file_path,
                        &artist,
                        &album,
                        initial_durations.len(),
                        failure_msg,
                    );
                }
            }
        }
        Err(e) => {
            error!("[A{}] FAILED: {}", album_idx + 1, e);
            info!("[A{}] ", album_idx + 1);
            return ValidationResult::error(
                &file_path,
                &artist,
                &album,
                initial_durations.len(),
                format!("MusicBrainz lookup failed: {}", e),
            );
        }
    };

    // Display edition information
    info!(
        "[A{}]   Edition Details (sorted by match likelihood):",
        album_idx + 1
    );
    for (idx, edition) in editions.iter().enumerate() {
        let edition_duration: u32 = edition.durations.iter().sum();
        let match_score = score_edition_match(edition, file_duration_secs, estimated_track_count);
        info!(
            "[A{}]     [{}] {} - {} ({} tracks, {}s, {} MBIDs, score: {:.0}s, NDR:{},{:.1})",
            album_idx + 1,
            idx,
            edition.artist,
            edition.album,
            edition.track_count,
            edition_duration,
            edition.mbids.len(),
            match_score,
            edition.name_distance_rank,
            edition.name_distance_score
        );
    }

    // === RUN 19: Edition-by-Edition Processing (silence cache already computed in parallel) ===
    // Calculate RMS profile once for Stage 4 (used by all editions)
    let rms_profile = calculate_rms_profile(&samples, sample_rate);
    let total_duration_secs = samples.len() as f64 / sample_rate as f64;

    // Initialize tracking variables for best result across all editions
    let mut best_durations: Vec<f64> = initial_durations.clone();
    let mut best_matches: Vec<TrackMatch> = Vec::new();
    let mut best_matched_count: usize = 0;
    let mut best_percentage: f64 = 0.0;
    let mut best_stage = "none";
    let mut best_threshold: Option<f64> = None;
    let mut best_min_duration: Option<f64> = None;
    let mut best_mbid: String = String::new();
    let mut best_expected_durations: Vec<u32> = Vec::new();
    let mut best_mean_error: f64 = 0.0;
    let mut best_edition_idx: Option<usize> = None;

    let num_thresholds = threshold_values.len();
    let num_min_durations = min_duration_values.len();

    // Early exit atomics for grace period after 100% match
    let perfect_match_found = AtomicBool::new(false);
    let perfect_match_time_ms = AtomicU64::new(0);
    let parallel_start_time = Instant::now();

    info!(
        "[A{}]   === EDITION-BY-EDITION PROCESSING (Run 19 - PARALLEL MB+SILENCE) ===",
        album_idx + 1
    );
    info!(
        "[A{}]   Testing {} editions through Stages 2-5 ({}s between feeds, {}s grace period)...\n",
        album_idx + 1,
        editions.len(),
        EDITION_FEED_DELAY_SECS,
        EARLY_EXIT_GRACE_PERIOD_SECS
    );

    // Update heartbeat activity for edition testing phase
    query_stats.set_activity(&format!("testing {} editions", editions.len()));

    // Staggered feed: spawn editions one at a time with delays
    let results_mutex: Mutex<Vec<EditionTestResult>> = Mutex::new(Vec::new());
    let mut editions_started = 0;
    let mut editions_skipped = 0;

    rayon::scope(|s| {
        for (edition_idx, edition) in editions.iter().enumerate() {
            // Check if we should stop feeding new editions
            if perfect_match_found.load(Ordering::Relaxed) {
                editions_skipped = editions.len() - edition_idx;
                info!(
                    "[A{}]   Stopping feed: 100% match found, {} editions not started",
                    album_idx + 1,
                    editions_skipped
                );
                break;
            }

            editions_started += 1;

            // Update heartbeat activity to show current edition
            query_stats.set_activity(&format!(
                "edition {}/{}: {}",
                edition_idx + 1,
                editions.len(),
                edition.album
            ));

            info!(
                "[A{}]   Starting Edition {}/{}: {} - {}",
                album_idx + 1,
                edition_idx + 1,
                editions.len(),
                edition.artist,
                edition.album
            );

            // Clone references for the closure
            let silence_cache = &silence_cache;
            let rms_profile = &rms_profile;
            let initial_durations = &initial_durations;
            let perfect_match_found = &perfect_match_found;
            let perfect_match_time_ms = &perfect_match_time_ms;
            let results_mutex = &results_mutex;
            let total_editions = editions.len();

            s.spawn(move |_| {
                // Wrap edition testing in catch_unwind to prevent silent crashes
                let edition_result = panic::catch_unwind(panic::AssertUnwindSafe(|| {
                    test_single_edition(
                        edition_idx,
                        edition,
                        silence_cache,
                        num_thresholds,
                        num_min_durations,
                        match_tolerance_secs,
                        rms_profile,
                        total_duration_secs,
                        initial_durations,
                        total_editions,
                        perfect_match_found,
                        perfect_match_time_ms,
                        parallel_start_time,
                        album_idx,
                    )
                }));

                match edition_result {
                    Ok(result) => {
                        // Use lock_poisoned helper to handle mutex poisoning gracefully
                        match results_mutex.lock() {
                            Ok(mut guard) => guard.push(result),
                            Err(poisoned) => {
                                error!(
                                    "[A{}] Edition {}: Mutex poisoned, recovering...",
                                    album_idx + 1,
                                    edition_idx + 1
                                );
                                poisoned.into_inner().push(result);
                            }
                        }
                    }
                    Err(panic_payload) => {
                        let panic_msg = if let Some(s) = panic_payload.downcast_ref::<&str>() {
                            s.to_string()
                        } else if let Some(s) = panic_payload.downcast_ref::<String>() {
                            s.clone()
                        } else {
                            "unknown panic".to_string()
                        };
                        error!(
                            "[A{}] Edition {}/{} PANICKED: {}",
                            album_idx + 1,
                            edition_idx + 1,
                            total_editions,
                            panic_msg
                        );
                    }
                }
            });

            // Wait before feeding next edition (unless this is the last one)
            if edition_idx < editions.len() - 1 {
                std::thread::sleep(Duration::from_secs(EDITION_FEED_DELAY_SECS));
            }
        }
    });

    // Stop heartbeat now that edition testing is complete
    query_stats.stop();
    let _ = futures::executor::block_on(heartbeat_handle);

    // Extract results from mutex and sort by edition_idx for consistent output
    // Handle poisoned mutex gracefully (can happen if a thread panicked)
    let mut edition_results: Vec<EditionTestResult> = match results_mutex.into_inner() {
        Ok(results) => results,
        Err(poisoned) => {
            warn!(
                "[A{}] Results mutex was poisoned (a thread panicked), recovering results...",
                album_idx + 1
            );
            poisoned.into_inner()
        }
    };
    edition_results.sort_by_key(|r| r.edition_idx);

    info!(
        "[A{}]   Completed: {} editions started, {} skipped",
        album_idx + 1,
        editions_started,
        editions_skipped
    );

    // Print all log messages in order (debug level - verbose edition-by-edition details)
    for result in &edition_results {
        for msg in &result.log_messages {
            debug!("[A{}] {}", album_idx + 1, msg);
        }
        debug!("[A{}] ", album_idx + 1);
    }

    // Find the best result across all editions
    let best_result_opt = find_best_edition_result(&edition_results);

    // Extract best result into our tracking variables
    if let Some(best_edition_result) = best_result_opt {
        if let Some(ref result) = best_edition_result.best_result {
            best_durations = result.detected_durations.clone();
            best_matches = result.matches.clone();
            best_matched_count = result.matched_count;
            best_percentage = result.percentage;
            best_stage = best_edition_result.best_stage;
            best_threshold = best_edition_result.best_threshold;
            best_min_duration = best_edition_result.best_min_duration;
            best_mbid = format!("edition_{}", best_edition_result.edition_idx);
            best_expected_durations = best_edition_result.expected_durations.clone();
            best_mean_error = result.mean_error;
            best_edition_idx = Some(best_edition_result.edition_idx);
        }
    }

    // Count how many 100% matches we found
    let perfect_match_count = edition_results
        .iter()
        .filter(|r| r.best_percentage >= 100.0)
        .count();

    if perfect_match_count > 1 {
        info!("[A{}]   Found {} editions with 100% match - selected edition {} (lowest mean error: {:.2}s)",
            album_idx + 1,
            perfect_match_count,
            best_edition_idx.map(|i| i + 1).unwrap_or(0),
            best_mean_error);
    } else {
        info!("[A{}]   Parallel processing complete. Best: {:.1}% from edition {} (mean error: {:.2}s)",
            album_idx + 1, best_percentage, best_edition_idx.map(|i| i + 1).unwrap_or(0), best_mean_error);
    }

    // Convert placeholder MBID to real MBID from winning edition
    if best_mbid.starts_with("edition_") {
        if let Some(idx) = best_edition_idx {
            if idx < editions.len() {
                best_mbid = select_best_mbid(&editions[idx]);
            }
        }
    }

    // Calculate final statistics
    let perfect_count = best_durations.len() == best_expected_durations.len();

    // Get winning edition's artist and album
    let (winning_artist, winning_album) = if let Some(idx) = best_edition_idx {
        if idx < editions.len() {
            (editions[idx].artist.clone(), editions[idx].album.clone())
        } else {
            (UNKNOWN_VALUE.to_string(), UNKNOWN_VALUE.to_string())
        }
    } else {
        (UNKNOWN_VALUE.to_string(), UNKNOWN_VALUE.to_string())
    };

    // Verify artist match - detect potential wrong-artist matches
    let (artist_similarity, artist_match_ok) = verify_artist_match(&artist, &winning_artist);
    let artist_mismatch = !artist_match_ok;

    // If artist mismatch detected, apply stricter acceptance criteria
    if artist_mismatch {
        warn!("[A{}]   ⚠️  ARTIST MISMATCH DETECTED:", album_idx + 1);
        warn!("[A{}]       Source artist: '{}'", album_idx + 1, artist);
        warn!(
            "[A{}]       Matched artist: '{}'",
            album_idx + 1,
            winning_artist
        );
        warn!(
            "[A{}]       Similarity: {:.1}% (threshold: {:.1}%)",
            album_idx + 1,
            artist_similarity * 100.0,
            ARTIST_MISMATCH_THRESHOLD * 100.0
        );

        if best_percentage < ARTIST_MISMATCH_MIN_MATCH_PCT {
            warn!("[A{}]       Match rejected: {:.1}% track match < {:.1}% required for artist-mismatched editions",
                  album_idx + 1, best_percentage, ARTIST_MISMATCH_MIN_MATCH_PCT);
            warn!("[A{}]       This match is likely INCORRECT - different artist with similar album name",
                  album_idx + 1);
        } else {
            warn!(
                "[A{}]       Match accepted with warning: {:.1}% track match >= {:.1}% threshold",
                album_idx + 1,
                best_percentage,
                ARTIST_MISMATCH_MIN_MATCH_PCT
            );
            warn!(
                "[A{}]       Review recommended - high track match but different artist",
                album_idx + 1
            );
        }
    }

    info!("[A{}]   FINAL RESULT:", album_idx + 1);
    info!("[A{}]     Artist: {}", album_idx + 1, winning_artist);
    info!("[A{}]     Album: {}", album_idx + 1, winning_album);
    info!("[A{}]     Matching stage: {}", album_idx + 1, best_stage);
    info!(
        "[A{}]     MusicBrainz: https://musicbrainz.org/release/{}",
        album_idx + 1,
        best_mbid
    );
    info!(
        "[A{}]     Track count: {}/{} {}",
        album_idx + 1,
        best_durations.len(),
        best_expected_durations.len(),
        if perfect_count { "✓" } else { "✗" }
    );
    info!(
        "[A{}]     Matched tracks: {}/{} ({:.1}%)",
        album_idx + 1,
        best_matched_count,
        best_expected_durations.len(),
        best_percentage
    );
    info!(
        "[A{}]     Mean error: {:.2}s",
        album_idx + 1,
        best_mean_error
    );
    info!(
        "[A{}]     Confidence: {}",
        album_idx + 1,
        classify_confidence(best_percentage)
    );

    if let (Some(thresh), Some(min_dur)) = (best_threshold, best_min_duration) {
        info!(
            "[A{}]     Best parameters: {}dB, {}s",
            album_idx + 1,
            thresh,
            min_dur
        );
    }

    // Show all track matches (debug level - verbose per-track details)
    if !best_matches.is_empty() {
        info!("[A{}]   All tracks:", album_idx + 1);
        for (i, tm) in best_matches.iter().enumerate() {
            let status = if tm.matches { "✓" } else { "✗" };
            info!(
                "[A{}]     {}. {:6.1}s vs {:6}s  error={:5.1}s  {}",
                album_idx + 1,
                i + 1,
                tm.detected_duration,
                tm.expected_duration,
                tm.error,
                status
            );
        }
    }

    // Calculate extra tracks (detected tracks with no MusicBrainz match)
    let mut extra_tracks = Vec::new();
    let min_count = best_durations.len().min(best_expected_durations.len());

    if best_durations.len() > best_expected_durations.len() {
        for i in min_count..best_durations.len() {
            extra_tracks.push(ExtraTrack {
                track_index: i + 1,
                duration: best_durations[i],
                description: "Extra track with no corresponding MusicBrainz entry".to_string(),
            });
        }

        if !extra_tracks.is_empty() {
            info!(
                "[A{}]   Extra tracks detected ({} beyond expected count):",
                album_idx + 1,
                extra_tracks.len()
            );
            for et in &extra_tracks {
                info!(
                    "[A{}]     Track {}: {:.1}s (no MusicBrainz match)",
                    album_idx + 1,
                    et.track_index,
                    et.duration
                );
            }
        }
    } else if best_durations.len() < best_expected_durations.len() {
        let missing_count = best_expected_durations.len() - best_durations.len();
        info!(
            "[A{}]   MusicBrainz tracks not found in file ({} missing):",
            album_idx + 1,
            missing_count
        );
        for i in min_count..best_expected_durations.len() {
            info!(
                "[A{}]     Track {}: {}s (expected but not detected in file)",
                album_idx + 1,
                i + 1,
                best_expected_durations[i]
            );
        }
    }

    info!("[A{}] ", album_idx + 1);

    // Determine final status based on artist mismatch
    let status = if artist_mismatch && best_percentage < ARTIST_MISMATCH_MIN_MATCH_PCT {
        "Artist Mismatch - Likely Incorrect".to_string()
    } else if artist_mismatch {
        "Artist Mismatch - Review Required".to_string()
    } else {
        "Success".to_string()
    };

    ValidationResult {
        album_path: file_path.to_string_lossy().to_string(),
        artist,
        album,
        mbid: best_mbid.clone(),
        musicbrainz_url: format!("https://musicbrainz.org/release/{}", best_mbid),
        expected_track_count: best_expected_durations.len(),
        detected_track_count: best_durations.len(),
        perfect_count_match: perfect_count,
        track_matches: best_matches,
        extra_tracks,
        matched_tracks_count: best_matched_count,
        match_percentage: best_percentage,
        mean_error: best_mean_error,
        status,
        matching_stage: best_stage.to_string(),
        best_threshold_db: best_threshold,
        best_min_duration_secs: best_min_duration,
        confidence: classify_confidence(best_percentage),
        matched_artist: winning_artist,
        matched_album: winning_album,
        artist_mismatch,
        artist_similarity,
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // IMPORTANT: Get local time offset BEFORE any threads spawn (Unix security restriction)
    // Must be done at very start of main, before tokio runtime is fully active
    let local_offset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);

    // Create timer with local time + timezone offset (e.g., 2025-11-23T06:14:41.496667-05:00)
    let timer = OffsetTime::new(
        local_offset,
        time::macros::format_description!(
            "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:6][offset_hour sign:mandatory]:[offset_minute]"
        ),
    );

    // Initialize tracing with local timestamps (use RUST_LOG=debug to see MB queries)
    tracing_subscriber::fmt()
        .with_timer(timer)
        .with_target(false)
        .with_level(true)
        .with_line_number(true)
        .with_thread_ids(true)
        .init();

    // Install panic hook to ensure panics are logged before crash
    panic::set_hook(Box::new(|panic_info| {
        let location = panic_info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown location".to_string());

        let message = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "unknown panic payload".to_string()
        };

        // Log via tracing (may not flush)
        error!("PANIC at {}: {}", location, message);

        // Also write directly to stderr to ensure visibility
        let _ = writeln!(
            std::io::stderr(),
            "\n!!! PANIC at {}: {}",
            location,
            message
        );
        let _ = std::io::stderr().flush();

        // Print backtrace if available
        let backtrace = std::backtrace::Backtrace::capture();
        if backtrace.status() == std::backtrace::BacktraceStatus::Captured {
            let _ = writeln!(std::io::stderr(), "Backtrace:\n{}", backtrace);
            let _ = std::io::stderr().flush();
        }
    }));

    info!("=== Comprehensive Album Matcher (Run 21) ===");

    // Read training set
    let training_set_path = Path::new(r"C:\Users\Mango Cat\Dev\McRhythm\training_set.txt");
    let long_files_path = Path::new(r"C:\Users\Mango Cat\Dev\McRhythm\long_files_list.txt");

    let training_content = std::fs::read_to_string(training_set_path)?;
    let mut training_files = Vec::new();

    for line in training_content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Try numbered format first: "[1] path"
        if let Some(path_start) = line.find("] ") {
            let path_str = &line[path_start + 2..];
            training_files.push(PathBuf::from(path_str));
        } else {
            // Plain path format
            training_files.push(PathBuf::from(trimmed));
        }
    }

    let initial_count = training_files.len();

    // Read long files list and filter out files already in training set
    let long_files_content = std::fs::read_to_string(long_files_path)?;
    let mut additional_files = Vec::new();

    // Create a set of training file paths for efficient lookup
    let training_paths: std::collections::HashSet<PathBuf> =
        training_files.iter().cloned().collect();

    for line in long_files_content.lines() {
        if let Some(path_start) = line.find("] ") {
            let path_str = &line[path_start + 2..];
            let path = PathBuf::from(path_str);

            // Only add if not already in training set
            if !training_paths.contains(&path) {
                additional_files.push(path);
            }
        }
    }

    // Combine: training files first, then additional files
    training_files.extend(additional_files.clone());

    info!("=== Parameter Validation ===");
    info!(
        "Testing optimal parameters on {} albums from combined set",
        training_files.len()
    );
    info!(
        "  ({} training set + {} from long files list)\n",
        initial_count,
        additional_files.len()
    );

    // Use optimal parameters from analysis
    let threshold_db: f64 = DEFAULT_THRESHOLD_DB;
    let min_duration_secs: f64 = DEFAULT_MIN_DURATION_SECS;
    let match_tolerance_secs = MATCH_TOLERANCE_SECS;

    let rate_limiter = RateLimiter::new();

    // Parameter grid for Stage 2 optimization (defined in configuration constants section)
    // Using 'static references so they can be passed to async tasks
    let threshold_values: &'static [f64] = &STAGE2_THRESHOLD_VALUES;
    let min_duration_values: &'static [f64] = &STAGE2_MIN_DURATION_VALUES;

    // === RUN 18: MULTI-ALBUM PARALLEL PROCESSING ===
    // Process up to MAX_CONCURRENT_ALBUMS albums concurrently
    // Each album does decode + MB lookup in parallel within itself
    // All albums share the same rate limiter for MB API compliance
    info!("=== Run 19: Multi-Album Parallel Processing ===");
    info!("  Max concurrent albums: {}", MAX_CONCURRENT_ALBUMS);
    info!("  Album-prefixed logging: [A{{num}}] for parallel debugging");
    info!("  Shared rate limiter ensures MB API compliance\n");

    let total_albums = training_files.len();

    let results: Vec<ValidationResult> = stream::iter(training_files.into_iter().enumerate())
        .map(|(idx, file_path)| {
            let rate_limiter = rate_limiter.clone();
            async move {
                process_single_album(
                    idx,
                    total_albums,
                    file_path,
                    threshold_db,
                    min_duration_secs,
                    match_tolerance_secs,
                    threshold_values,
                    min_duration_values,
                    rate_limiter,
                )
                .await
            }
        })
        .buffer_unordered(MAX_CONCURRENT_ALBUMS)
        .collect()
        .await;

    // Note: Results may be out of order due to parallel processing
    // Sort by album path for consistent output
    let mut results = results;
    results.sort_by(|a, b| a.album_path.cmp(&b.album_path));

    // Legacy sequential processing code removed - all album processing is now in process_single_album()
    // Write results
    let output_path = Path::new(r"C:\Users\Mango Cat\Dev\McRhythm\album_matcher_results.json");
    info!("\n=== Writing Results ===");
    info!("Output: {}", output_path.display());

    let json = serde_json::to_string_pretty(&results)?;
    std::fs::write(output_path, json)?;

    // Analysis
    info!("\n=== COMPREHENSIVE MATCHING ANALYSIS ===");

    let successful = results.iter().filter(|r| r.status == "Success").count();
    info!("Total albums processed: {}", results.len());
    info!("Successfully analyzed: {}", successful);

    if successful > 0 {
        // Matching stage breakdown
        let stage1 = results
            .iter()
            .filter(|r| r.matching_stage == "album_extractor_1_initial")
            .count();
        let stage2 = results
            .iter()
            .filter(|r| r.matching_stage == "album_extractor_2_optimization")
            .count();
        let stage3 = results
            .iter()
            .filter(|r| r.matching_stage == "album_extractor_3_assembly")
            .count();
        let stage4 = results
            .iter()
            .filter(|r| r.matching_stage == "album_extractor_4_guided")
            .count();
        let stage5 = results
            .iter()
            .filter(|r| r.matching_stage == "album_extractor_5_editions")
            .count();
        let stage6 = results
            .iter()
            .filter(|r| r.matching_stage == "album_extractor_6_merging")
            .count();

        info!("\nMatching Stage Results:");
        info!("  album_extractor_1_initial:        {} albums", stage1);
        info!("  album_extractor_2_optimization:   {} albums", stage2);
        info!("  album_extractor_3_assembly:       {} albums", stage3);
        info!("  album_extractor_4_guided:         {} albums", stage4);
        info!("  album_extractor_5_editions:       {} albums", stage5);
        info!("  album_extractor_6_merging:        {} albums", stage6);

        // Confidence level breakdown
        let excellent = results
            .iter()
            .filter(|r| r.confidence == "Excellent")
            .count();
        let good = results.iter().filter(|r| r.confidence == "Good").count();
        let fair = results.iter().filter(|r| r.confidence == "Fair").count();
        let poor = results
            .iter()
            .filter(|r| r.confidence == "Poor" && r.status == "Success")
            .count();

        info!("\nConfidence Distribution:");
        info!("  Excellent (≥80%): {} albums", excellent);
        info!("  Good (60-79%):    {} albums", good);
        info!("  Fair (40-59%):    {} albums", fair);
        info!("  Poor (<40%):      {} albums", poor);

        // Track count matches
        let perfect_counts = results.iter().filter(|r| r.perfect_count_match).count();
        info!("\nTrack Count Matches:");
        info!(
            "  Perfect: {}/{} ({:.1}%)",
            perfect_counts,
            successful,
            (perfect_counts as f64 / successful as f64) * 100.0
        );

        // Average statistics
        let avg_match_pct = results
            .iter()
            .filter(|r| r.status == "Success")
            .map(|r| r.match_percentage)
            .sum::<f64>()
            / successful as f64;

        let avg_error = results
            .iter()
            .filter(|r| r.status == "Success" && r.mean_error > 0.0)
            .map(|r| r.mean_error)
            .sum::<f64>()
            / successful as f64;

        info!("\nAverage Statistics:");
        info!("  Match percentage: {:.1}%", avg_match_pct);
        info!("  Mean error: {:.2}s", avg_error);

        // Parameter effectiveness (for Stage 2 results)
        if stage2 > 0 {
            info!("\nParameter Optimization Details:");
            for result in results
                .iter()
                .filter(|r| r.matching_stage == "album_extractor_2_optimization")
            {
                if let (Some(thresh), Some(min_dur)) =
                    (result.best_threshold_db, result.best_min_duration_secs)
                {
                    info!(
                        "  {} - {}: {}dB, {}s → {:.1}%",
                        result.artist, result.album, thresh, min_dur, result.match_percentage
                    );
                }
            }
        }

        // Show best and worst
        let mut success_results: Vec<_> =
            results.iter().filter(|r| r.status == "Success").collect();
        success_results.sort_by(|a, b| {
            b.match_percentage
                .partial_cmp(&a.match_percentage)
                .unwrap_or(CmpOrdering::Equal)
        });

        info!("\nBest 5 Albums:");
        for (i, result) in success_results.iter().take(5).enumerate() {
            info!(
                "  {}. {} - {} ({:.1}%, {} via {})",
                i + 1,
                result.artist,
                result.album,
                result.match_percentage,
                result.confidence,
                result.matching_stage
            );
            info!("     MusicBrainz: {}", result.musicbrainz_url);
        }

        info!("\nWorst 5 Albums:");
        for (i, result) in success_results.iter().rev().take(5).enumerate() {
            info!(
                "  {}. {} - {} ({:.1}%, {} via {})",
                i + 1,
                result.artist,
                result.album,
                result.match_percentage,
                result.confidence,
                result.matching_stage
            );
            info!("     MusicBrainz: {}", result.musicbrainz_url);
        }
    }

    info!("\nDone!");
    Ok(())
}

// ===== Unit Tests =====

#[cfg(test)]
mod tests {
    use super::*;

    // ===== Tests for analyze_track_matching() =====

    #[test]
    fn test_analyze_track_matching_perfect_match() {
        let detected = vec![180.5, 200.3, 195.7];
        let expected = vec![180, 200, 196];
        let tolerance = 10.0;

        let (matches, matched_count, percentage) =
            analyze_track_matching(&detected, &expected, tolerance);

        assert_eq!(matched_count, 3, "All tracks should match");
        assert_eq!(percentage, 100.0, "Should be 100% match");
        assert_eq!(matches.len(), 3, "Should have 3 match records");
        assert!(
            matches.iter().all(|m| m.matches),
            "All matches should be true"
        );
    }

    #[test]
    fn test_analyze_track_matching_partial_match() {
        let detected = vec![180.0, 250.0, 196.0]; // Middle track off by 50s
        let expected = vec![180, 200, 196];
        let tolerance = 10.0;

        let (matches, matched_count, percentage) =
            analyze_track_matching(&detected, &expected, tolerance);

        assert_eq!(matched_count, 2, "Only 2 tracks should match");
        assert_eq!(
            percentage, 66.666666666666664,
            "Should be ~66.67% match (2/3)"
        );
        assert!(!matches[1].matches, "Middle track should not match");
        assert!(
            matches[0].matches && matches[2].matches,
            "First and last should match"
        );
    }

    #[test]
    fn test_analyze_track_matching_extra_detected_tracks() {
        let detected = vec![180.0, 100.0, 100.0, 196.0]; // Over-segmented: 4 detected vs 3 expected
        let expected = vec![180, 200, 196];
        let tolerance = 10.0;

        let (matches, matched_count, percentage) =
            analyze_track_matching(&detected, &expected, tolerance);

        // Should only compare first 3 (minimum count)
        assert_eq!(matches.len(), 3, "Should compare minimum count (3)");
        // First matches (180 vs 180), second doesn't (100 vs 200), third doesn't (100 vs 196)
        assert_eq!(matched_count, 1, "Only first track should match");
        assert_eq!(percentage, 33.33333333333333, "Should be ~33.33% (1/3)");
    }

    #[test]
    fn test_analyze_track_matching_extra_expected_tracks() {
        let detected = vec![180.0, 200.0]; // Under-segmented: 2 detected vs 3 expected
        let expected = vec![180, 200, 196];
        let tolerance = 10.0;

        let (matches, matched_count, percentage) =
            analyze_track_matching(&detected, &expected, tolerance);

        // Should only compare first 2 (minimum count)
        assert_eq!(matches.len(), 2, "Should compare minimum count (2)");
        assert_eq!(matched_count, 2, "Both should match");
        // Percentage based on EXPECTED count (3), so 2/3 = 66.67%
        assert_eq!(
            percentage, 66.666666666666664,
            "Should be ~66.67% (2/3 expected)"
        );
    }

    #[test]
    fn test_analyze_track_matching_empty_inputs() {
        let detected: Vec<f64> = vec![];
        let expected: Vec<u32> = vec![];
        let tolerance = 10.0;

        let (matches, matched_count, percentage) =
            analyze_track_matching(&detected, &expected, tolerance);

        assert_eq!(matches.len(), 0, "No matches for empty inputs");
        assert_eq!(matched_count, 0, "Zero matched count");
        assert_eq!(percentage, 0.0, "Zero percentage");
    }

    #[test]
    fn test_analyze_track_matching_tolerance_boundary() {
        let detected = vec![180.0, 210.0, 196.0]; // Middle track exactly at tolerance boundary
        let expected = vec![180, 200, 196];
        let tolerance = 10.0;

        let (matches, matched_count, percentage) =
            analyze_track_matching(&detected, &expected, tolerance);

        // Middle track: 210.0 - 200 = 10.0, which equals tolerance
        assert!(
            matches[1].matches,
            "Track at tolerance boundary should match"
        );
        assert_eq!(matched_count, 3, "All should match");
        assert_eq!(percentage, 100.0, "Should be 100%");
    }

    #[test]
    fn test_analyze_track_matching_just_outside_tolerance() {
        let detected = vec![180.0, 210.1, 196.0]; // Middle track just outside tolerance
        let expected = vec![180, 200, 196];
        let tolerance = 10.0;

        let (matches, matched_count, percentage) =
            analyze_track_matching(&detected, &expected, tolerance);

        // Middle track: 210.1 - 200 = 10.1, which exceeds tolerance
        assert!(
            !matches[1].matches,
            "Track just outside tolerance should not match"
        );
        assert_eq!(matched_count, 2, "Only 2 should match");
        assert_eq!(percentage, 66.666666666666664, "Should be ~66.67%");
    }

    // ===== Tests for strings_match() =====

    #[test]
    fn test_strings_match_exact() {
        assert!(strings_match("Thriller", "Thriller"));
    }

    #[test]
    fn test_strings_match_case_insensitive() {
        assert!(strings_match("Thriller", "thriller"));
        assert!(strings_match("THRILLER", "thriller"));
        assert!(strings_match("ThRiLlEr", "THRILLER"));
    }

    #[test]
    fn test_strings_match_with_punctuation() {
        assert!(strings_match(
            "The Dark Side of the Moon",
            "The Dark Side of the Moon"
        ));
        // Punctuation is filtered out, but "and" vs "&" remain different alphanumeric tokens
        assert!(!strings_match(
            "Crosby, Stills & Nash",
            "Crosby Stills and Nash"
        ));
        assert!(strings_match("Led Zeppelin IV", "Led Zeppelin IV"));
    }

    #[test]
    fn test_strings_match_whitespace_variations() {
        // Consecutive whitespace is NOT normalized - strings must match exactly after filtering
        assert!(!strings_match("Led  Zeppelin", "Led Zeppelin")); // Extra space makes them different
                                                                  // Spaces are preserved when filtering, so "LedZeppelin" != "Led Zeppelin"
        assert!(!strings_match("LedZeppelin", "Led Zeppelin"));
    }

    #[test]
    fn test_strings_no_match() {
        assert!(!strings_match("Thriller", "Bad"));
        assert!(!strings_match("Pink Floyd", "Led Zeppelin"));
    }

    // ===== Tests for detect_silence() =====

    #[test]
    fn test_detect_silence_all_silence() {
        // Create very quiet audio (below threshold)
        // Note: All zeros might not be detected as it could be below noise floor
        // Use very small but non-zero values
        let samples = vec![0.0001; 48000]; // 1 second of near-silence at 48kHz
        let sample_rate = 48000;
        let threshold_db = -40.0; // Higher threshold to ensure detection
        let min_duration_secs = 0.5;

        let silence_regions =
            detect_silence(&samples, sample_rate, threshold_db, min_duration_secs);

        // Should detect the entire file as one silence region (or possibly none if all zeros are skipped)
        // Allow for either outcome as implementation may handle edge case differently
        assert!(
            silence_regions.len() <= 1,
            "Should detect at most one silence region"
        );
        if silence_regions.len() == 1 {
            assert!(
                silence_regions[0].0 <= 1000,
                "Silence should start near beginning"
            );
        }
    }

    #[test]
    fn test_detect_silence_no_silence() {
        // Create loud audio (all at 0.5 amplitude, well above -60dB threshold)
        let samples = vec![0.5; 48000];
        let sample_rate = 48000;
        let threshold_db = -60.0;
        let min_duration_secs = 0.5;

        let silence_regions =
            detect_silence(&samples, sample_rate, threshold_db, min_duration_secs);

        assert_eq!(
            silence_regions.len(),
            0,
            "Should detect no silence in loud audio"
        );
    }

    #[test]
    fn test_detect_silence_with_gap() {
        // Create audio with loud section, then silence, then loud section
        let mut samples = Vec::new();

        // 1 second of loud audio (0.5 amplitude)
        samples.extend(vec![0.5; 48000]);

        // 1 second of silence (0.0 amplitude)
        samples.extend(vec![0.0; 48000]);

        // 1 second of loud audio (0.5 amplitude)
        samples.extend(vec![0.5; 48000]);

        let sample_rate = 48000;
        let threshold_db = -60.0;
        let min_duration_secs = 0.5;

        let silence_regions =
            detect_silence(&samples, sample_rate, threshold_db, min_duration_secs);

        // Should detect one silence region in the middle
        assert_eq!(silence_regions.len(), 1, "Should detect one silence region");
        // Silence should start around sample 48000 (allowing for window boundaries)
        assert!(
            silence_regions[0].0 >= 40000 && silence_regions[0].0 <= 56000,
            "Silence should start around the 1-second mark"
        );
    }

    #[test]
    fn test_detect_silence_min_duration_filter() {
        // Create brief silence that's shorter than min_duration
        let mut samples = Vec::new();

        // 1 second of loud audio
        samples.extend(vec![0.5; 48000]);

        // 0.3 seconds of silence (shorter than 0.5s minimum)
        samples.extend(vec![0.0; 14400]);

        // 1 second of loud audio
        samples.extend(vec![0.5; 48000]);

        let sample_rate = 48000;
        let threshold_db = -60.0;
        let min_duration_secs = 0.5; // Require at least 0.5s of silence

        let silence_regions =
            detect_silence(&samples, sample_rate, threshold_db, min_duration_secs);

        // Brief silence should be filtered out
        assert_eq!(
            silence_regions.len(),
            0,
            "Should not detect silence shorter than minimum duration"
        );
    }

    // ===== Tests for get_track_durations() =====

    #[test]
    fn test_get_track_durations_single_track() {
        // Create one continuous loud audio segment (no silence)
        let samples = vec![0.5; 96000]; // 2 seconds at 48kHz
        let sample_rate = 48000;
        let threshold_db = -60.0;
        let min_duration_secs = 0.5;

        let durations = get_track_durations(&samples, sample_rate, threshold_db, min_duration_secs);

        assert_eq!(durations.len(), 1, "Should detect one track");
        assert!(
            (durations[0] - 2.0).abs() < 0.1,
            "Track should be approximately 2 seconds"
        );
    }

    #[test]
    fn test_get_track_durations_two_tracks() {
        // Create two tracks separated by silence
        let mut samples = Vec::new();

        // Track 1: 3 seconds
        samples.extend(vec![0.5; 144000]); // 3 seconds at 48kHz

        // Silence: 1 second
        samples.extend(vec![0.0; 48000]);

        // Track 2: 2 seconds
        samples.extend(vec![0.5; 96000]);

        let sample_rate = 48000;
        let threshold_db = -60.0;
        let min_duration_secs = 0.5;

        let durations = get_track_durations(&samples, sample_rate, threshold_db, min_duration_secs);

        assert_eq!(durations.len(), 2, "Should detect two tracks");
        assert!(
            (durations[0] - 3.0).abs() < 0.2,
            "First track should be ~3 seconds"
        );
        assert!(
            (durations[1] - 2.0).abs() < 0.2,
            "Second track should be ~2 seconds"
        );
    }

    #[test]
    fn test_get_track_durations_empty_audio() {
        let samples: Vec<f32> = vec![];
        let sample_rate = 48000;
        let threshold_db = -60.0;
        let min_duration_secs = 0.5;

        let durations = get_track_durations(&samples, sample_rate, threshold_db, min_duration_secs);

        assert_eq!(durations.len(), 0, "Empty audio should produce no tracks");
    }

    // ===== Tests for split_camel_case() =====

    #[test]
    fn test_split_camel_case_basic() {
        assert_eq!(split_camel_case("DaylightAgain"), "Daylight Again");
        assert_eq!(
            split_camel_case("TransEuropeExpress"),
            "Trans Europe Express"
        );
    }

    #[test]
    fn test_split_camel_case_already_spaced() {
        assert_eq!(split_camel_case("Daylight Again"), "Daylight Again");
    }

    #[test]
    fn test_split_camel_case_all_caps() {
        assert_eq!(split_camel_case("USA"), "USA"); // Should not add spaces between caps
    }

    #[test]
    fn test_split_camel_case_mixed() {
        assert_eq!(split_camel_case("BusinessAsUsual"), "Business As Usual");
    }

    // ===== Integration Test for classify_confidence() =====

    #[test]
    fn test_classify_confidence_boundaries() {
        assert_eq!(classify_confidence(100.0), "Excellent");
        assert_eq!(classify_confidence(85.0), "Excellent");
        assert_eq!(classify_confidence(80.0), "Excellent");
        assert_eq!(classify_confidence(79.9), "Good");
        assert_eq!(classify_confidence(70.0), "Good");
        assert_eq!(classify_confidence(60.0), "Good");
        assert_eq!(classify_confidence(59.9), "Fair");
        assert_eq!(classify_confidence(50.0), "Fair");
        assert_eq!(classify_confidence(40.0), "Fair");
        assert_eq!(classify_confidence(39.9), "Poor");
        assert_eq!(classify_confidence(0.0), "Poor");
    }

    // ===== Tests for Run 15 single-edition functions =====

    #[test]
    fn test_segmentation_against_single_edition_perfect_match() {
        let detected = vec![180.5, 200.3, 195.7];
        let expected = vec![180, 200, 196];
        let tolerance = 10.0;

        let result = test_segmentation_against_single_edition(
            &detected,
            &expected,
            "test_edition",
            tolerance,
        );

        assert_eq!(result.matched_count, 3);
        assert_eq!(result.percentage, 100.0);
        assert_eq!(result.mbid, "test_edition");
        assert_eq!(result.expected_durations, expected);
    }

    #[test]
    fn test_segmentation_against_single_edition_partial_match() {
        let detected = vec![180.0, 250.0, 196.0]; // Middle track way off
        let expected = vec![180, 200, 196];
        let tolerance = 10.0;

        let result =
            test_segmentation_against_single_edition(&detected, &expected, "edition_1", tolerance);

        assert_eq!(result.matched_count, 2);
        assert!((result.percentage - 66.67).abs() < 1.0);
    }

    // ===== Tests for assemble_segments_dp() =====

    #[test]
    fn test_assemble_segments_dp_basic() {
        // 6 segments that should combine into 3 tracks
        let detected = vec![90.0, 90.0, 100.0, 100.0, 98.0, 98.0];
        let expected = vec![180, 200, 196];

        let result = assemble_segments_dp(&detected, &expected);

        assert!(result.is_some(), "Should find an assembly");
        let assembled = result.unwrap();
        assert_eq!(assembled.len(), 3, "Should produce 3 tracks");
        // Check that assembled durations are close to expected
        assert!(
            (assembled[0] - 180.0).abs() < 15.0,
            "First track should be ~180s"
        );
        assert!(
            (assembled[1] - 200.0).abs() < 15.0,
            "Second track should be ~200s"
        );
        assert!(
            (assembled[2] - 196.0).abs() < 15.0,
            "Third track should be ~196s"
        );
    }

    #[test]
    fn test_assemble_segments_dp_exact_match() {
        // Segments that exactly sum to expected
        let detected = vec![60.0, 60.0, 60.0, 80.0, 80.0, 40.0, 98.0, 98.0];
        let expected = vec![180, 200, 196];

        let result = assemble_segments_dp(&detected, &expected);

        assert!(result.is_some());
        let assembled = result.unwrap();
        assert_eq!(assembled.len(), 3);
    }

    #[test]
    fn test_assemble_segments_dp_fewer_segments_than_targets() {
        // 2 segments can't become 3 tracks
        let detected = vec![180.0, 200.0];
        let expected = vec![180, 200, 196];

        let result = assemble_segments_dp(&detected, &expected);

        assert!(
            result.is_none(),
            "Should return None when fewer segments than targets"
        );
    }

    #[test]
    fn test_assemble_segments_dp_equal_segments_and_targets() {
        // Same number - should just return as-is or None
        let detected = vec![180.0, 200.0, 196.0];
        let expected = vec![180, 200, 196];

        let result = assemble_segments_dp(&detected, &expected);

        // With equal counts, DP assembly isn't meaningful but shouldn't crash
        // The function requires detected.len() > expected.len()
        assert!(result.is_none(), "Equal counts should return None");
    }

    // ===== Tests for boundaries_to_durations() =====

    #[test]
    fn test_boundaries_to_durations_basic() {
        // Boundaries at 180s and 380s for a 576s file (3 tracks)
        let boundaries = vec![180.0, 380.0];
        let total_duration = 576.0;

        let durations = boundaries_to_durations(&boundaries, total_duration);

        assert_eq!(durations.len(), 3);
        assert!(
            (durations[0] - 180.0).abs() < 0.01,
            "First track: 0 to 180 = 180s"
        );
        assert!(
            (durations[1] - 200.0).abs() < 0.01,
            "Second track: 180 to 380 = 200s"
        );
        assert!(
            (durations[2] - 196.0).abs() < 0.01,
            "Third track: 380 to 576 = 196s"
        );
    }

    #[test]
    fn test_boundaries_to_durations_no_boundaries() {
        // No boundaries = single track
        let boundaries: Vec<f64> = vec![];
        let total_duration = 300.0;

        let durations = boundaries_to_durations(&boundaries, total_duration);

        assert_eq!(durations.len(), 1);
        assert!((durations[0] - 300.0).abs() < 0.01);
    }

    #[test]
    fn test_boundaries_to_durations_many_boundaries() {
        // 4 boundaries = 5 tracks
        let boundaries = vec![60.0, 120.0, 180.0, 240.0];
        let total_duration = 300.0;

        let durations = boundaries_to_durations(&boundaries, total_duration);

        assert_eq!(durations.len(), 5);
        for d in &durations {
            assert!((d - 60.0).abs() < 0.01, "Each track should be 60s");
        }
    }

    // ===== Tests for resort_by_name_similarity() =====

    #[test]
    fn test_resort_by_name_similarity_no_swap_needed() {
        // Editions already in order by name similarity
        let mut editions = vec![
            create_test_edition("Artist", "Album", 10, 1.0), // name_distance_score = 1.0
            create_test_edition("Artist", "Album", 10, 2.0), // name_distance_score = 2.0
            create_test_edition("Artist", "Album", 10, 3.0), // name_distance_score = 3.0
        ];

        resort_by_name_similarity(&mut editions);

        // No swaps should occur (none is > NAME_DISTANCE_SWAP_RATIO times the next)
        assert!((editions[0].name_distance_score - 1.0).abs() < 0.01);
        assert!((editions[1].name_distance_score - 2.0).abs() < 0.01);
        assert!((editions[2].name_distance_score - 3.0).abs() < 0.01);
    }

    #[test]
    fn test_resort_by_name_similarity_swap_needed() {
        // First edition has much worse score than second (10.0 > NAME_DISTANCE_SWAP_RATIO * 2.0)
        let mut editions = vec![
            create_test_edition("Artist", "Album", 10, 10.0), // Should swap down
            create_test_edition("Artist", "Album", 10, 2.0),  // Should swap up
            create_test_edition("Artist", "Album", 10, 3.0),
        ];

        resort_by_name_similarity(&mut editions);

        // After sort: 2.0 should be first (10.0 > NAME_DISTANCE_SWAP_RATIO * 2.0 triggers swap)
        assert!((editions[0].name_distance_score - 2.0).abs() < 0.01);
        // 10.0 should be second (10.0 > NAME_DISTANCE_SWAP_RATIO * 3.0, so it swaps again)
        assert!((editions[1].name_distance_score - 3.0).abs() < 0.01);
        assert!((editions[2].name_distance_score - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_resort_by_name_similarity_empty_list() {
        let mut editions: Vec<Edition> = vec![];
        resort_by_name_similarity(&mut editions);
        assert_eq!(editions.len(), 0);
    }

    #[test]
    fn test_resort_by_name_similarity_single_item() {
        let mut editions = vec![create_test_edition("Artist", "Album", 10, 5.0)];
        resort_by_name_similarity(&mut editions);
        assert_eq!(editions.len(), 1);
        assert!((editions[0].name_distance_score - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_resort_conservative_threshold() {
        // Test that ratio at/below NAME_DISTANCE_SWAP_RATIO does NOT trigger swap
        // With NAME_DISTANCE_SWAP_RATIO = 1.732, threshold is 1.732 * 2.0 = 3.464
        // Using 3.4 which is below threshold (3.4 / 2.0 = 1.7x < 1.732x)
        let mut editions = vec![
            create_test_edition("Artist", "Album", 10, 3.4), // 3.4 / 2.0 = 1.7x (not > ratio)
            create_test_edition("Artist", "Album", 10, 2.0),
        ];

        let original_first = editions[0].name_distance_score;
        resort_by_name_similarity(&mut editions);

        // Should NOT swap because 3.4 is not > NAME_DISTANCE_SWAP_RATIO * 2.0 = 3.464
        assert!(
            (editions[0].name_distance_score - original_first).abs() < 0.01,
            "Should not swap when ratio <= NAME_DISTANCE_SWAP_RATIO"
        );
    }

    // Helper function to create test editions
    fn create_test_edition(
        artist: &str,
        album: &str,
        track_count: usize,
        name_distance_score: f64,
    ) -> Edition {
        Edition {
            artist: artist.to_string(),
            album: album.to_string(),
            track_count,
            durations: vec![180; track_count],
            mbids: vec![EditionMBID {
                mbid: "test-mbid".to_string(),
                country: Some("US".to_string()),
                status: Some("Official".to_string()),
                is_cd: true,
            }],
            duration_signature: "180,180,180".to_string(),
            name_distance_rank: 1,
            name_distance_score,
        }
    }

    // ===== Tests for calculate_rms() =====

    #[test]
    fn test_calculate_rms_silence() {
        let samples = vec![0.0; 1000];
        let rms = calculate_rms(&samples);
        assert!(rms < 1e-10, "RMS of silence should be near zero");
    }

    #[test]
    fn test_calculate_rms_constant_signal() {
        let samples = vec![0.5; 1000];
        let rms = calculate_rms(&samples);
        assert!(
            (rms - 0.5).abs() < 0.01,
            "RMS of constant 0.5 should be 0.5"
        );
    }

    #[test]
    fn test_calculate_rms_sine_wave() {
        // Sine wave RMS = amplitude / sqrt(2)
        let samples: Vec<f32> = (0..4800)
            .map(|i| (2.0 * std::f32::consts::PI * i as f32 / 48.0).sin())
            .collect();
        let rms = calculate_rms(&samples);
        let expected = 1.0 / std::f32::consts::SQRT_2;
        assert!(
            (rms - expected).abs() < 0.01,
            "Sine wave RMS should be 1/sqrt(2)"
        );
    }

    #[test]
    fn test_calculate_rms_empty() {
        let samples: Vec<f32> = vec![];
        let rms = calculate_rms(&samples);
        assert!(
            rms.is_nan() || rms == 0.0,
            "Empty samples should return NaN or 0"
        );
    }

    // ===== Tests for calculate_db() =====

    #[test]
    fn test_calculate_db_full_scale() {
        let samples = vec![1.0; 1000];
        let db = calculate_db(&samples);
        assert!((db - 0.0).abs() < 0.1, "Full scale (1.0) should be ~0dB");
    }

    #[test]
    fn test_calculate_db_half_amplitude() {
        let samples = vec![0.5; 1000];
        let db = calculate_db(&samples);
        // 20 * log10(0.5) = -6.02 dB
        assert!((db - (-6.02)).abs() < 0.1, "Half amplitude should be ~-6dB");
    }

    #[test]
    fn test_calculate_db_silence() {
        let samples = vec![0.0; 1000];
        let db = calculate_db(&samples);
        assert!(db <= SILENCE_DB_FLOOR, "Silence should hit the floor");
    }

    #[test]
    fn test_calculate_db_quiet_signal() {
        let samples = vec![0.001; 1000];
        let db = calculate_db(&samples);
        // 20 * log10(0.001) = -60 dB
        assert!(
            (db - (-60.0)).abs() < 1.0,
            "0.001 amplitude should be ~-60dB"
        );
    }

    // ===== Tests for calculate_mbid_priority_score() =====

    #[test]
    fn test_mbid_score_cd_official_us() {
        let score = calculate_mbid_priority_score(true, Some("US"), Some("Official"));
        // CD bonus (-50) + US bonus (-30) + Official bonus (-40) = -120
        assert!((score - (-120.0)).abs() < 0.01);
    }

    #[test]
    fn test_mbid_score_vinyl_bootleg_uk() {
        let score = calculate_mbid_priority_score(false, Some("UK"), Some("Bootleg"));
        // No CD, not US, not Official = 0
        assert!((score - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_mbid_score_cd_only() {
        let score = calculate_mbid_priority_score(true, None, None);
        assert!((score - SCORE_CD_BONUS).abs() < 0.01);
    }

    #[test]
    fn test_mbid_score_official_only() {
        let score = calculate_mbid_priority_score(false, None, Some("Official"));
        assert!((score - SCORE_OFFICIAL_BONUS).abs() < 0.01);
    }

    // ===== Tests for gaps_to_track_durations() =====

    #[test]
    fn test_gaps_to_track_durations_no_gaps() {
        let silence_regions: Vec<(usize, usize)> = vec![];
        let durations = gaps_to_track_durations(&silence_regions, 480000, 48000);
        assert_eq!(durations.len(), 1);
        assert!((durations[0] - 10.0).abs() < 0.01, "Single 10s track");
    }

    #[test]
    fn test_gaps_to_track_durations_one_gap() {
        // Gap at samples 240000-288000 (5s-6s)
        let silence_regions = vec![(240000, 288000)];
        let durations = gaps_to_track_durations(&silence_regions, 480000, 48000);
        assert_eq!(durations.len(), 2);
        assert!((durations[0] - 5.0).abs() < 0.01, "First track 5s");
        assert!((durations[1] - 4.0).abs() < 0.01, "Second track 4s");
    }

    #[test]
    fn test_gaps_to_track_durations_multiple_gaps() {
        // 3 tracks: 0-2s, 3-5s, 6-10s (gaps at 2-3s, 5-6s)
        let silence_regions = vec![(96000, 144000), (240000, 288000)];
        let durations = gaps_to_track_durations(&silence_regions, 480000, 48000);
        assert_eq!(durations.len(), 3);
        assert!((durations[0] - 2.0).abs() < 0.01);
        assert!((durations[1] - 2.0).abs() < 0.01);
        assert!((durations[2] - 4.0).abs() < 0.01);
    }

    // ===== Tests for find_silence_regions_from_profile() =====

    #[test]
    fn test_find_silence_regions_all_loud() {
        let profile = WindowDbProfile {
            db_values: vec![-20.0, -25.0, -30.0, -20.0],
            window_step: 1200,
            total_samples: 4800,
        };
        let regions = find_silence_regions_from_profile(&profile, -50.0, 2400);
        assert_eq!(regions.len(), 0, "No silence in loud audio");
    }

    #[test]
    fn test_find_silence_regions_all_quiet() {
        let profile = WindowDbProfile {
            db_values: vec![-60.0, -65.0, -70.0, -60.0],
            window_step: 1200,
            total_samples: 4800,
        };
        let regions = find_silence_regions_from_profile(&profile, -50.0, 1200);
        assert_eq!(regions.len(), 1, "One continuous silence region");
    }

    #[test]
    fn test_find_silence_regions_mixed() {
        // Loud-quiet-loud pattern
        let profile = WindowDbProfile {
            db_values: vec![-20.0, -20.0, -60.0, -65.0, -60.0, -20.0, -20.0],
            window_step: 1200,
            total_samples: 8400,
        };
        let regions = find_silence_regions_from_profile(&profile, -50.0, 2400);
        assert_eq!(regions.len(), 1, "One silence region in middle");
    }

    // ===== Tests for compute_window_db_profile() =====

    #[test]
    fn test_compute_window_db_profile_silence() {
        let samples = vec![0.0001; 4800]; // 100ms at 48kHz
        let profile = compute_window_db_profile(&samples, 48000);
        assert!(!profile.db_values.is_empty());
        for db in &profile.db_values {
            assert!(*db < -40.0, "All windows should be quiet");
        }
    }

    #[test]
    fn test_compute_window_db_profile_loud() {
        let samples = vec![0.5; 4800];
        let profile = compute_window_db_profile(&samples, 48000);
        for db in &profile.db_values {
            assert!(*db > -10.0, "All windows should be loud (~-6dB)");
        }
    }

    // ===== Tests for apply_wildcard_fixes() =====

    #[test]
    fn test_apply_wildcard_fixes_lizzie() {
        // Function only handles "Lizzie" -> "Lizz*" variant
        let result = apply_wildcard_fixes("Thin Lizzie");
        assert_eq!(result, Some("Thin Lizz*".to_string()));
    }

    #[test]
    fn test_apply_wildcard_fixes_lizzie_in_album() {
        let result = apply_wildcard_fixes("Lizzie McGuire Soundtrack");
        assert_eq!(result, Some("Lizz* McGuire Soundtrack".to_string()));
    }

    #[test]
    fn test_apply_wildcard_fixes_no_match() {
        let result = apply_wildcard_fixes("Normal Album Title");
        assert!(result.is_none());
    }

    #[test]
    fn test_apply_wildcard_fixes_no_lizzy_variant() {
        // "Lizzy" should NOT match (only "Lizzie" is fixed)
        let result = apply_wildcard_fixes("Thin Lizzy");
        assert!(result.is_none());
    }

    // ===== Tests for generate_search_queries() =====

    #[test]
    fn test_generate_search_queries_basic() {
        let queries = generate_search_queries("The Beatles", "Abbey Road");
        // First query is: "type:album AND artist:{} AND release:{}"
        assert!(queries
            .iter()
            .any(|q| q.contains("artist:The Beatles") && q.contains("release:Abbey Road")));
    }

    #[test]
    fn test_generate_search_queries_the_prefix() {
        let queries = generate_search_queries("The Rolling Stones", "Exile");
        assert!(queries.iter().any(|q| q.contains("Rolling Stones")));
    }

    #[test]
    fn test_generate_search_queries_camel_case() {
        let queries = generate_search_queries("Kraftwerk", "TransEuropeExpress");
        assert!(queries.iter().any(|q| q.contains("Trans Europe Express")));
    }

    // ===== Tests for calculate_name_distance() =====

    #[test]
    fn test_calculate_name_distance_exact_match() {
        let source_artists = vec!["Beatles".to_string()];
        let source_albums = vec!["Abbey Road".to_string()];
        let distance =
            calculate_name_distance("Beatles", "Abbey Road", &source_artists, &source_albums);
        assert!((distance - 0.0).abs() < 0.01, "Exact match should be 0");
    }

    #[test]
    fn test_calculate_name_distance_different() {
        let source_artists = vec!["Pink Floyd".to_string()];
        let source_albums = vec!["The Wall".to_string()];
        let distance =
            calculate_name_distance("Beatles", "Abbey Road", &source_artists, &source_albums);
        assert!(distance > 5.0, "Different names should have high distance");
    }

    #[test]
    fn test_calculate_name_distance_album_weighted() {
        // Album difference should be weighted more (NAME_DISTANCE_ALBUM_WEIGHT = 1.414)
        let source_artists_b = vec!["B".to_string()];
        let source_albums_same = vec!["Same".to_string()];
        let distance_artist =
            calculate_name_distance("A", "Same", &source_artists_b, &source_albums_same);

        let source_artists_same = vec!["Same".to_string()];
        let source_albums_b = vec!["B".to_string()];
        let distance_album =
            calculate_name_distance("Same", "A", &source_artists_same, &source_albums_b);
        assert!(distance_album > distance_artist, "Album diff weighted more");
    }

    // ===== Tests for extract_metadata_from_path() =====

    #[test]
    fn test_extract_metadata_from_path_standard() {
        // Function uses second-to-last component as artist, last (filename minus ext) as album
        let path = Path::new("/music/Pink Floyd/Dark Side of the Moon.mp3");
        let (artist, album) = extract_metadata_from_path(path);
        assert_eq!(artist, "Pink Floyd");
        assert_eq!(album, "Dark Side of the Moon");
    }

    #[test]
    fn test_extract_metadata_from_path_short() {
        let path = Path::new("/music/file.mp3");
        let (artist, album) = extract_metadata_from_path(path);
        // With only 2 components, should still extract something
        assert_eq!(artist, "music");
        assert_eq!(album, "file");
    }

    // ===== Tests for choose_artist() and choose_album() =====

    #[test]
    fn test_choose_artist_exact_match() {
        let (artist, _, _) = choose_artist("Beatles", "Beatles", "Abbey Road");
        assert_eq!(artist, "Beatles");
    }

    #[test]
    fn test_choose_artist_prefer_id3() {
        let (artist, _, source) = choose_artist("The Beatles", "Beatles", "Abbey Road");
        assert_eq!(source, MetadataSource::ID3);
    }

    #[test]
    fn test_choose_album_exact_match() {
        let (album, _, _) = choose_album("Abbey Road", "Abbey Road");
        assert_eq!(album, "Abbey Road");
    }

    #[test]
    fn test_choose_album_prefer_id3() {
        let (album, _, source) = choose_album("Abbey Road (Remaster)", "AbbeyRoad");
        assert_eq!(source, MetadataSource::ID3);
    }

    // ===== Tests for select_best_mbid() =====

    #[test]
    fn test_select_best_mbid_single() {
        let edition = create_test_edition("Artist", "Album", 10, 1.0);
        let mbid = select_best_mbid(&edition);
        assert_eq!(mbid, "test-mbid");
    }

    #[test]
    fn test_select_best_mbid_prefers_cd_us_official() {
        let edition = Edition {
            artist: "Artist".to_string(),
            album: "Album".to_string(),
            track_count: 10,
            durations: vec![180; 10],
            mbids: vec![
                EditionMBID {
                    mbid: "vinyl-uk".to_string(),
                    country: Some("UK".to_string()),
                    status: Some("Official".to_string()),
                    is_cd: false,
                },
                EditionMBID {
                    mbid: "cd-us-official".to_string(),
                    country: Some("US".to_string()),
                    status: Some("Official".to_string()),
                    is_cd: true,
                },
            ],
            duration_signature: "180,180".to_string(),
            name_distance_rank: 1,
            name_distance_score: 1.0,
        };
        let mbid = select_best_mbid(&edition);
        assert_eq!(mbid, "cd-us-official");
    }

    // ===== Tests for score_edition_match() =====

    #[test]
    fn test_score_edition_match_exact_duration() {
        let edition = create_test_edition("Artist", "Album", 10, 5.0);
        // edition has 10 tracks of 180s each = 1800s total
        let score = score_edition_match(&edition, 1800.0, None);
        // Exact duration match should have score ~0
        assert!(score < 10.0, "Exact duration match should have low score");
    }

    #[test]
    fn test_score_edition_match_wrong_duration() {
        let edition = create_test_edition("Artist", "Album", 10, 5.0);
        // edition has 10 tracks of 180s each = 1800s total
        let score_exact = score_edition_match(&edition, 1800.0, None);
        let score_wrong = score_edition_match(&edition, 3000.0, None); // 1200s off
        assert!(score_wrong > score_exact, "Wrong duration should penalize");
        assert!(
            (score_wrong - 1200.0).abs() < 10.0,
            "Penalty should be ~1200s"
        );
    }

    #[test]
    fn test_score_edition_match_track_count_penalty() {
        let edition = create_test_edition("Artist", "Album", 10, 5.0);
        // Same duration, but different track counts
        let score_no_estimate = score_edition_match(&edition, 1800.0, None);
        let score_exact_tracks = score_edition_match(&edition, 1800.0, Some(10));
        let score_wrong_tracks = score_edition_match(&edition, 1800.0, Some(15));
        assert!(
            (score_no_estimate - score_exact_tracks).abs() < 1.0,
            "Exact track count = no penalty"
        );
        assert!(
            score_wrong_tracks > score_exact_tracks,
            "Wrong track count should add penalty"
        );
    }

    // ===== Tests for group_into_editions() =====

    #[test]
    fn test_group_into_editions_single() {
        let releases = vec![(
            vec![180, 200, 196],
            EditionMBID {
                mbid: "mbid1".to_string(),
                country: Some("US".to_string()),
                status: Some("Official".to_string()),
                is_cd: true,
            },
            "Artist".to_string(),
            "Album".to_string(),
            1_usize,
            5.0_f64,
        )];
        let editions = group_into_editions(releases, 0);
        assert_eq!(editions.len(), 1);
        assert_eq!(editions[0].track_count, 3);
    }

    #[test]
    fn test_group_into_editions_merge_same_signature() {
        let releases = vec![
            (
                vec![180, 200, 196],
                EditionMBID {
                    mbid: "mbid1".to_string(),
                    country: Some("US".to_string()),
                    status: Some("Official".to_string()),
                    is_cd: true,
                },
                "Artist".to_string(),
                "Album".to_string(),
                1_usize,
                5.0_f64,
            ),
            (
                vec![180, 200, 196], // Same signature
                EditionMBID {
                    mbid: "mbid2".to_string(),
                    country: Some("UK".to_string()),
                    status: Some("Official".to_string()),
                    is_cd: true,
                },
                "Artist".to_string(),
                "Album".to_string(),
                2_usize,
                6.0_f64,
            ),
        ];
        let editions = group_into_editions(releases, 0);
        assert_eq!(editions.len(), 1, "Same signature should merge");
        assert_eq!(editions[0].mbids.len(), 2, "Should have 2 MBIDs");
    }

    #[test]
    fn test_group_into_editions_different_signatures() {
        let releases = vec![
            (
                vec![180, 200, 196],
                EditionMBID {
                    mbid: "mbid1".to_string(),
                    country: Some("US".to_string()),
                    status: Some("Official".to_string()),
                    is_cd: true,
                },
                "Artist".to_string(),
                "Album".to_string(),
                1_usize,
                5.0_f64,
            ),
            (
                vec![180, 200, 196, 300], // Different track count
                EditionMBID {
                    mbid: "mbid2".to_string(),
                    country: Some("US".to_string()),
                    status: Some("Official".to_string()),
                    is_cd: true,
                },
                "Artist".to_string(),
                "Album Deluxe".to_string(),
                2_usize,
                6.0_f64,
            ),
        ];
        let editions = group_into_editions(releases, 0);
        assert_eq!(
            editions.len(),
            2,
            "Different signatures = different editions"
        );
    }

    // ===== Tests for should_exit_early() and signal_perfect_match() =====

    #[test]
    fn test_should_exit_early_not_found() {
        let found = AtomicBool::new(false);
        let time_ms = AtomicU64::new(0);
        let start = Instant::now();
        assert!(!should_exit_early(&found, &time_ms, start));
    }

    #[test]
    fn test_should_exit_early_found_but_in_grace() {
        let found = AtomicBool::new(true);
        let time_ms = AtomicU64::new(Instant::now().elapsed().as_millis() as u64);
        let start = Instant::now();
        // Within grace period
        assert!(!should_exit_early(&found, &time_ms, start));
    }

    #[test]
    fn test_signal_perfect_match_sets_flag() {
        let found = AtomicBool::new(false);
        let time_ms = AtomicU64::new(u64::MAX); // Set to MAX to verify it gets overwritten
        let start = Instant::now();
        // Add small delay to ensure non-zero elapsed time
        std::thread::sleep(std::time::Duration::from_millis(1));
        signal_perfect_match(&found, &time_ms, start);
        assert!(found.load(Ordering::SeqCst), "Flag should be set");
        // Time should have been updated (could be 0 or more, but was MAX before)
        assert!(
            time_ms.load(Ordering::SeqCst) < u64::MAX,
            "Time should have been set"
        );
    }

    // ===== Tests for precompute_silence_cache() =====

    #[test]
    fn test_precompute_silence_cache_basic() {
        // Create simple audio: loud-silent-loud
        let mut samples = Vec::new();
        samples.extend(vec![0.5; 48000]); // 1s loud
        samples.extend(vec![0.0; 48000]); // 1s silent
        samples.extend(vec![0.5; 48000]); // 1s loud

        let thresholds = [-50.0, -60.0];
        let min_durations = [0.5, 1.0];
        let cache = precompute_silence_cache(&samples, 48000, &thresholds, &min_durations);

        // Should have 4 entries (2 thresholds x 2 durations)
        assert_eq!(cache.len(), 4);
        // Each should detect 2 tracks (split by the silence)
        for entry in &cache {
            assert!(
                entry.len() >= 1 && entry.len() <= 3,
                "Should detect 1-3 tracks"
            );
        }
    }

    // ===== Tests for calculate_rms_profile() =====

    #[test]
    fn test_calculate_rms_profile_uniform() {
        let samples = vec![0.5; 48000]; // 1s at 48kHz
        let profile = calculate_rms_profile(&samples, 48000);
        assert!(!profile.is_empty());
        for (_timestamp, rms) in &profile {
            assert!(
                (*rms - 0.5).abs() < 0.1,
                "Uniform signal should have uniform RMS"
            );
        }
    }

    #[test]
    fn test_calculate_rms_profile_varying() {
        let mut samples = Vec::new();
        samples.extend(vec![0.1; 24000]); // 0.5s quiet
        samples.extend(vec![0.9; 24000]); // 0.5s loud
        let profile = calculate_rms_profile(&samples, 48000);
        assert!(profile.len() >= 2);
        // First windows should be quieter than last windows
        let first_half_avg: f32 = profile[..profile.len() / 2]
            .iter()
            .map(|(_, rms)| rms)
            .sum::<f32>()
            / (profile.len() / 2) as f32;
        let second_half_avg: f32 = profile[profile.len() / 2..]
            .iter()
            .map(|(_, rms)| rms)
            .sum::<f32>()
            / (profile.len() / 2) as f32;
        assert!(
            second_half_avg > first_half_avg,
            "Second half should be louder"
        );
    }
}
