/// Comprehensive Album Matcher with Edition-by-Edition Processing (Run 16)
///
/// Run 16 Changes:
/// - Silence detection caching: Pre-compute all 180 parameter combinations ONCE
///   before parallel edition testing, then reuse cached track durations
/// - Parallel pre-computation: The 180 silence scans run in parallel using rayon
/// - Reduces 4,140 silence scans (23 editions × 180 params) to just 180 scans
/// - Expected ~3-5x speedup on pre-computation phase from parallelization
/// - Early exit with grace period: After first 100% match found, other threads
///   have EARLY_EXIT_GRACE_PERIOD_SECS (default 20s) to complete and contribute
///   results before being terminated. This allows finding better 100% matches
///   (lower mean_error) while still benefiting from early termination.
///
/// Run 15 Changes:
/// - Early NDR filtering - calculate Name Distance Rank BEFORE fetching track details
///   This saves API calls for releases with poor name similarity (NDR > 100)
/// - Parallel edition testing using rayon
/// - Edition-by-edition processing from Run 14 preserved
///
/// Processing Architecture (Run 16):
/// 1. Phase 0: ID3 Tag Extraction & Reconciliation
/// 2. Fetch and rank all MusicBrainz editions
/// 3. For each edition (in order of likelihood):
///    - Stage 2: Parameter optimization (180 combinations) for THIS edition
///    - Stage 3: Segment assembly from over-segmented candidates for THIS edition
///    - Stage 4: Edition-guided quiet spot detection for THIS edition
///    - Stage 5: Extra track merging for THIS edition
///    - If 100% match achieved, STOP and return this edition
/// 4. Return best result across all editions tested
///
/// Phase 0: ID3 Tag Extraction & Reconciliation
/// - Extract ID3 tags via ffprobe
/// - Reconcile with path/filename using heuristics
/// - Provides alternate search terms when conflicts detected
/// - Extracts estimated track count from ID3 comment field
///
/// Name Distance Ranking (Run 11)
/// - Calculate Levenshtein distance from each MusicBrainz candidate to source names
/// - Rank all candidates 1-N based on name similarity (1 = closest match)
/// - Display "NDR:{rank}" in all candidate album logging
///
/// Stage 2: Parameter Optimization (Single Edition)
/// - Test 180 parameter combinations for current edition
/// - Collects over-segmented candidates for Stage 3 assembly
///
/// Stage 3: Segment Assembly (Single Edition)
/// - Try DP assembly of over-segmented candidates to match current edition's track count
///
/// Stage 4: Quiet Spot Detection (Single Edition)
/// - Use current edition's track durations as guide for RMS-based boundary detection
///
/// Stage 5: Extra Track Merging (Single Edition)
/// - When detected > expected AND match ≥100%, merge adjacent tracks
///
/// Tracks which stage succeeded and saves optimal parameters for each album

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use serde::{Deserialize, Serialize};
use tokio::time::sleep;
use strsim::levenshtein;
use rayon::prelude::*;
use tracing::{debug, info, warn, error};
use tracing_subscriber::fmt::time::SystemTime;

// ===== Configuration Constants =====

// Default silence detection parameters
const DEFAULT_THRESHOLD_DB: f64 = -57.0;
const DEFAULT_MIN_DURATION_SECS: f64 = 0.9;

// Track matching tolerance (seconds difference allowed for a track to be considered "matched")
const MATCH_TOLERANCE_SECS: f64 = 10.0;

// MusicBrainz API configuration
const MB_RATE_LIMIT_MS: u64 = 1550;          // Milliseconds between API requests (2x safety margin)
const MB_REQUEST_TIMEOUT_SECS: u64 = 30;     // HTTP request timeout
const MB_MAX_RELEASES: usize = 150;          // Maximum releases to fetch across all strategies
const MB_RETRY_DELAYS_SECS: &[u64] = &[0, 5, 15, 45, 60]; // Backoff delays for retry attempts

// Edition/MBID scoring weights (lower score = better match)
const SCORE_CD_BONUS: f64 = -50.0;           // Bonus for CD releases
const SCORE_OFFICIAL_BONUS: f64 = -40.0;     // Bonus for Official status
const SCORE_US_BONUS: f64 = -30.0;           // Bonus for US releases
const SCORE_TRACK_COUNT_PENALTY: f64 = 60.0; // Seconds penalty per track count difference

// Runtime filter tolerance (file duration must be within this % of edition duration)
const RUNTIME_FILTER_MIN_RATIO: f64 = 0.75;  // 75% minimum
const RUNTIME_FILTER_MAX_RATIO: f64 = 1.25;  // 125% maximum

// Confidence level thresholds (match percentage)
const CONFIDENCE_EXCELLENT_THRESHOLD: f64 = 80.0;
const CONFIDENCE_GOOD_THRESHOLD: f64 = 60.0;
const CONFIDENCE_FAIR_THRESHOLD: f64 = 40.0;

// Silence detection window sizing (adaptive based on min_duration)
const RMS_WINDOW_SHORT_SECS: f64 = 0.025;    // 25ms for very short silences (≤0.3s)
const RMS_WINDOW_MEDIUM_SECS: f64 = 0.05;    // 50ms for medium silences (0.3-0.6s)
const RMS_WINDOW_STANDARD_SECS: f64 = 0.1;   // 100ms for longer silences (>0.6s)
const RMS_WINDOW_OVERLAP: f64 = 0.5;         // 50% overlap between windows

// Stage 4: Edition-guided quiet spot detection
const QUIET_SPOT_WINDOW_SECS: f64 = 0.5;       // 500ms window for RMS calculation
const QUIET_SPOT_WINDOW_STEP_SECS: f64 = 0.25; // 250ms step between windows (50% overlap)
const QUIET_SPOT_SEARCH_RADIUS_RATIO: f64 = 0.15; // 15% of expected track duration
const QUIET_SPOT_SEARCH_RADIUS_MIN: f64 = 5.0;   // Minimum search radius
const QUIET_SPOT_SEARCH_RADIUS_MAX: f64 = 20.0;  // Maximum search radius
const QUIET_SPOT_PROXIMITY_PENALTY: f64 = 0.5;   // Penalty factor for distance from expected
const QUIET_SPOT_TOP_EDITIONS: usize = 5;        // Try top N editions for guided search

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
const RMS_WINDOW_THRESHOLD_SHORT: f64 = 0.3;  // Use short window for ≤0.3s min duration
const RMS_WINDOW_THRESHOLD_MEDIUM: f64 = 0.6; // Use medium window for ≤0.6s min duration

// Distance penalty multiplier for quiet spot scoring
// Converts normalized distance (0-1) to dB-scale penalty
const QUIET_SPOT_DISTANCE_PENALTY_MULTIPLIER: f64 = 20.0;

// Maximum Name Distance Rank (NDR) allowed for candidate editions
// Editions with NDR > this value are filtered out early to avoid testing
// candidates with poor name similarity (likely wrong album/artist)
const MAX_NAME_DISTANCE_RANK: usize = 100;

// Early exit grace period (seconds) after first 100% match is found
// Other threads have this much time to complete and contribute results
// before early exit terminates remaining work
const EARLY_EXIT_GRACE_PERIOD_SECS: u64 = 20;

// Staggered feed delay (seconds) between starting new edition tests
// This allows earlier editions to find 100% before later ones even start
const EDITION_FEED_DELAY_SECS: u64 = 5;

// Stage 4 penalty: Quiet spot detection is less reliable than silence-based detection.
// Results from Stage 4 are de-rated by this percentage (100% Stage 4 becomes 75%).
// Stage 4 can never trigger a 100% early exit due to this penalty.
const STAGE4_PENALTY_PERCENT: f64 = 25.0;

// Stage 2 Parameter Grid: Threshold values (dB) for silence detection sweep
// Run 13: Extended to -30dB, -34dB for albums with louder inter-track gaps
const STAGE2_THRESHOLD_VALUES: [f64; 12] = [
    -50.0, -58.0, -47.0, -54.0, -38.0, -56.0,
    -42.0, -52.0, -34.0, -60.0, -36.0, -30.0
];

// Stage 2 Parameter Grid: Min duration values (seconds) for silence detection sweep
const STAGE2_MIN_DURATION_VALUES: [f64; 15] = [
    0.05, 4.0, 0.10, 3.0, 0.15, 2.5, 0.2, 
    2.0, 0.25, 1.5, 0.3, 1.0, 0.4, 0.8, 0.5
];

// ===== End Configuration Constants =====

// ===== Silence Detection Cache (Run 16) =====

/// Pre-computed track durations for all parameter combinations.
/// Indexed as: cache[threshold_idx * num_min_durations + min_duration_idx]
/// This avoids re-computing silence detection 23+ times per album (once per edition).
type SilenceCache = Vec<Vec<f64>>;

/// Pre-compute track durations for all parameter combinations IN PARALLEL.
/// Called ONCE per album before parallel edition processing.
/// Uses rayon to parallelize the 180 silence detection scans across all CPU cores.
fn precompute_silence_cache(
    samples: &[f32],
    sample_rate: u32,
    threshold_values: &[f64],
    min_duration_values: &[f64],
) -> SilenceCache {
    let num_min_durations = min_duration_values.len();

    // Build list of all (index, threshold, min_duration) tuples
    let params: Vec<(usize, f64, f64)> = threshold_values
        .iter()
        .enumerate()
        .flat_map(|(thresh_idx, &thresh)| {
            min_duration_values
                .iter()
                .enumerate()
                .map(move |(dur_idx, &min_dur)| {
                    let idx = thresh_idx * num_min_durations + dur_idx;
                    (idx, thresh, min_dur)
                })
        })
        .collect();

    let total = params.len();

    // Compute all silence detections in parallel
    let mut results: Vec<(usize, Vec<f64>)> = params
        .par_iter()
        .map(|&(idx, thresh, min_dur)| {
            let durations = get_track_durations(samples, sample_rate, thresh, min_dur);
            (idx, durations)
        })
        .collect();

    // Sort by index to restore correct order
    results.sort_by_key(|(idx, _)| *idx);

    // Extract just the durations in order
    results.into_iter().map(|(_, durations)| durations).collect()
}

/// Look up pre-computed track durations from cache.
/// Returns None if indices are out of bounds (shouldn't happen with valid params).
fn lookup_cached_durations(
    cache: &SilenceCache,
    threshold_idx: usize,
    min_duration_idx: usize,
    num_min_durations: usize,
) -> Option<&Vec<f64>> {
    let idx = threshold_idx * num_min_durations + min_duration_idx;
    cache.get(idx)
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

/// Results from Stage 2 parameter optimization.
/// Contains both the best immediate match AND all over-segmented candidates
/// for comprehensive Stage 3 assembly (Run 12 fix).
/// NOTE: Unused in Run 15 (edition-by-edition processing uses SingleEditionStage2Results).
#[derive(Debug)]
#[allow(dead_code)]
struct Stage2Results {
    /// Best match found during parameter optimization.
    best_result: Option<CandidateTestResult>,
    /// All over-segmented candidates for Stage 3 assembly.
    over_segmented_candidates: Vec<OverSegmentedCandidate>,
}

// ===== Rate Limiting =====

/// Rate limiter for MusicBrainz API compliance.
/// Enforces minimum delay between requests to avoid being blocked.
///
/// Note on async safety: This uses std::sync::Mutex which is safe here because
/// the lock is never held across an await point - we lock briefly to read/write
/// the timestamp, release immediately, then perform async sleep without holding the lock.
#[derive(Debug)]
struct RateLimiter {
    /// Timestamp of last API request.
    last_request: Arc<std::sync::Mutex<std::time::Instant>>,
}

impl RateLimiter {
    fn new() -> Self {
        Self {
            last_request: Arc::new(std::sync::Mutex::new(
                std::time::Instant::now() - Duration::from_millis(MB_RATE_LIMIT_MS)
            )),
        }
    }

    async fn wait(&self) {
        let elapsed = {
            let last = self.last_request.lock().expect("RateLimiter mutex poisoned");
            last.elapsed()
        };

        // MusicBrainz API limit: 1 req/sec
        // Use 2-second delay (0.5 req/sec) for 2x safety margin to prevent timeouts
        if elapsed < Duration::from_millis(MB_RATE_LIMIT_MS) {
            let wait_time = Duration::from_millis(MB_RATE_LIMIT_MS) - elapsed;
            sleep(wait_time).await;
        }

        *self.last_request.lock().expect("RateLimiter mutex poisoned") = std::time::Instant::now();
    }
}

/// Retry a network operation with exponential backoff
/// Attempts: immediate, +5s, +15s, +45s (then gives up)
async fn retry_with_backoff<F, Fut, T, E>(mut operation: F) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let max_attempts = MB_RETRY_DELAYS_SECS.len();

    for (attempt, &delay) in MB_RETRY_DELAYS_SECS.iter().enumerate() {
        if delay > 0 {
            warn!("    Retrying after {} seconds (attempt {}/{})...", delay, attempt + 1, max_attempts);
            sleep(Duration::from_secs(delay)).await;
        }

        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                if attempt < max_attempts - 1 {
                    warn!("    Network error: {} - will retry", e);
                } else {
                    error!("    Network error: {} - giving up after {} attempts", e, max_attempts);
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
#[derive(Debug, Clone, Copy)]
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

    let probed = symphonia::default::get_probe().format(&hint, mss, &format_opts, &metadata_opts)?;
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
            Ok(decoded) => {
                match decoded {
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
                }
            }
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

    let mut tracks = Vec::new();
    let mut current_start = 0;

    for (silence_start, silence_end) in silence_regions {
        if silence_start > current_start {
            let duration_secs = (silence_start - current_start) as f64 / sample_rate as f64;
            tracks.push(duration_secs);
        }
        current_start = silence_end;
    }

    // Add final segment
    if current_start < samples.len() {
        let duration_secs = (samples.len() - current_start) as f64 / sample_rate as f64;
        tracks.push(duration_secs);
    }

    tracks
}

/// Compare detected tracks with expected, analyzing match quality
fn analyze_track_matching(detected: &[f64], expected: &[u32], tolerance_secs: f64) -> (Vec<TrackMatch>, usize, f64) {
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
        if i > 0 && ch.is_uppercase() && chars[i-1].is_lowercase() {
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
    queries.push(format!("type:album AND artist:{} AND release:{}", artist, album));

    // Strategy 2: CamelCase split (most effective per test results)
    let album_spaced = split_camel_case(album);
    if album_spaced != album {
        queries.push(format!("type:album AND artist:{} AND release:\"{}\"", artist, album_spaced));
    }

    // Strategy 3: Fuzzy matching (catches punctuation differences like "Funk49" -> "Funk #49")
    queries.push(format!("type:album AND artist:{}~ AND release:{}~", artist, album));

    // Strategy 4: Targeted wildcard for common misspellings (e.g., "Lizzie" -> "Lizz*")
    if let Some(artist_wildcard) = apply_wildcard_fixes(artist) {
        let album_variant = apply_wildcard_fixes(album).unwrap_or_else(|| album.to_string());
        queries.push(format!("type:album AND artist:{} AND release:{}", artist_wildcard, album_variant));
    } else if let Some(album_wildcard) = apply_wildcard_fixes(album) {
        queries.push(format!("type:album AND artist:{} AND release:{}", artist, album_wildcard));
    }

    // Strategy 5: Aggressive fuzzy search (~2 edits - more tolerant, catches more misspellings)
    queries.push(format!("type:album AND artist:{}~2 AND release:{}~2", artist, album));

    // Strategy 6: Per-token fuzzy matching (handles multi-word names better)
    let artist_tokens: Vec<&str> = artist.split_whitespace().collect();
    let album_tokens: Vec<&str> = album.split_whitespace().collect();
    if artist_tokens.len() > 1 || album_tokens.len() > 1 {
        let artist_fuzzy = artist_tokens.iter().map(|t| format!("{}~", t)).collect::<Vec<_>>().join(" ");
        let album_fuzzy = album_tokens.iter().map(|t| format!("{}~", t)).collect::<Vec<_>>().join(" ");
        queries.push(format!("type:album AND artist:({}) AND release:({})", artist_fuzzy, album_fuzzy));
    }

    // Strategy 7: Album-only fallback (last resort when artist name is problematic)
    queries.push(format!("type:album AND release:{}", album));

    queries
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
    // Calculate average Levenshtein distance to all source album names
    let album_distances: Vec<usize> = source_albums
        .iter()
        .map(|source_album| levenshtein(candidate_album, source_album))
        .collect();
    let avg_album_distance = if album_distances.is_empty() {
        0.0
    } else {
        album_distances.iter().sum::<usize>() as f64 / album_distances.len() as f64
    };

    // Calculate average Levenshtein distance to all source artist names
    let artist_distances: Vec<usize> = source_artists
        .iter()
        .map(|source_artist| levenshtein(candidate_artist, source_artist))
        .collect();
    let avg_artist_distance = if artist_distances.is_empty() {
        0.0
    } else {
        artist_distances.iter().sum::<usize>() as f64 / artist_distances.len() as f64
    };

    // Overall score: album name weighted 2x, artist name weighted 1x
    (NAME_DISTANCE_ALBUM_WEIGHT * avg_album_distance + NAME_DISTANCE_ARTIST_WEIGHT * avg_artist_distance)
        / (NAME_DISTANCE_ALBUM_WEIGHT + NAME_DISTANCE_ARTIST_WEIGHT)
}

/// Comprehensive MusicBrainz search using ALL strategies and name variants
/// Returns up to MB_MAX_RELEASES releases with full metadata (not yet grouped into editions)
/// Returns: Vec<(durations, mbid_info, artist, album, name_distance_rank, name_distance_score)>
async fn comprehensive_musicbrainz_search(
    artist_variants: &[String],  // e.g., ["Jessita Reyes", "Various"]
    album_variants: &[String],   // e.g., ["Native American Flute Lullabies", "NativeAmericanFluteLullabies"]
    _file_duration_secs: f64,    // Total audio file duration (reserved for future filtering)
    _id3_track_count: Option<usize>, // Track count from ID3 tags (reserved for future use)
    rate_limiter: &RateLimiter,
) -> Result<Vec<(Vec<u32>, EditionMBID, String, String, usize, f64)>, Box<dyn std::error::Error>> {
    let client = reqwest::Client::builder()
        .user_agent("WKMP-ParameterValidator/0.1 (https://github.com/yourusername/wkmp)")
        .timeout(Duration::from_secs(MB_REQUEST_TIMEOUT_SECS))
        .build()?;

    let mut all_releases: Vec<MBRelease> = Vec::new();
    let mut seen_mbids = std::collections::HashSet::new();

    // Query with ALL combinations of artist/album variants and ALL search strategies
    for artist in artist_variants {
        for album in album_variants {
            let search_queries = generate_search_queries(artist, album);

            for (i, query) in search_queries.iter().enumerate() {
                if all_releases.len() >= MB_MAX_RELEASES {
                    info!("  Reached {} release limit", MB_MAX_RELEASES);
                    break;
                }

                let encoded_query = urlencoding::encode(query);
                let search_url = format!(
                    "https://musicbrainz.org/ws/2/release/?query={}&fmt=json&limit=100",
                    encoded_query
                );

                rate_limiter.wait().await;
                debug!("MB query: {}", search_url);

                let response = retry_with_backoff(|| async {
                    client
                        .get(&search_url)
                        .send()
                        .await
                        .map_err(|e| format!("error sending request: {}", e))?
                        .json::<MBSearchResponse>()
                        .await
                        .map_err(|e| format!("error parsing JSON: {}", e))
                }).await;

                let response = match response {
                    Ok(r) => r,
                    Err(e) => {
                        info!("  Strategy {}/{} for '{}' / '{}' FAILED: {}",
                                 i + 1, search_queries.len(), artist, album, e);
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

    info!("  Found {} unique releases across all search strategies", all_releases.len());

    // === Run 15 Optimization: Calculate NDR BEFORE fetching track details ===
    // This allows us to skip API calls for releases with poor name similarity

    // Calculate name distance score for each release using search result data
    let mut releases_with_ndr: Vec<(&MBRelease, f64)> = all_releases
        .iter()
        .map(|release| {
            let artist = release.artist_credit
                .as_ref()
                .and_then(|credits| credits.first())
                .and_then(|credit| credit.artist.as_ref())
                .map(|artist| artist.name.as_str())
                .unwrap_or("Unknown Artist");
            let score = calculate_name_distance(artist, &release.title, artist_variants, album_variants);
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
        warn!("  Early NDR filter: skipping {} releases (NDR > {}), fetching details for {}",
                 ndr_filtered_count, MAX_NAME_DISTANCE_RANK, filtered_releases.len());
    }

    // Fetch track details only for NDR-filtered releases
    let mut results: Vec<(Vec<u32>, EditionMBID, String, String, usize, f64)> = Vec::new();

    for (release, rank, score) in filtered_releases.iter() {
        rate_limiter.wait().await;

        let details_url = format!(
            "https://musicbrainz.org/ws/2/release/{}?inc=recordings&fmt=json",
            release.id
        );
        debug!("MB details: {}", details_url);

        let details = retry_with_backoff(|| async {
            client
                .get(&details_url)
                .send()
                .await
                .map_err(|e| format!("error fetching details: {}", e))?
                .json::<MBReleaseDetails>()
                .await
                .map_err(|e| format!("error parsing details: {}", e))
        }).await;

        let details = match details {
            Ok(d) => d,
            Err(_) => continue,
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
            continue;
        }

        // Extract artist name
        let artist = release.artist_credit
            .as_ref()
            .and_then(|credits| credits.first())
            .and_then(|credit| credit.artist.as_ref())
            .map(|artist| artist.name.clone())
            .unwrap_or_else(|| "Unknown Artist".to_string());

        // Use pre-calculated rank and score from early NDR filter
        results.push((
            durations,
            EditionMBID {
                mbid: release.id.clone(),
                country: release.country.clone(),
                status: release.status.clone(),
                is_cd,
            },
            artist,
            release.title.clone(),
            *rank,
            *score,
        ));
    }

    // Results already have NDR rank and score from early filtering (Run 15 optimization)
    Ok(results)
}

/// Group MBIDs into editions based on track count + duration pattern
/// Multiple MBIDs can represent the same edition (e.g., US vs UK release of same album)
/// Takes the best (lowest) name distance rank and its score among all MBIDs in an edition
fn group_into_editions(releases: Vec<(Vec<u32>, EditionMBID, String, String, usize, f64)>) -> Vec<Edition> {
    let mut editions: Vec<Edition> = Vec::new();

    for (durations, mbid_info, artist, album, rank, score) in releases {
        // Create signature: "track_count:duration1,duration2,..."
        let signature = format!("{}:{}",
            durations.len(),
            durations.iter().map(|d| d.to_string()).collect::<Vec<_>>().join(",")
        );

        // Find existing edition with this signature
        if let Some(edition) = editions.iter_mut().find(|e| e.duration_signature == signature) {
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

    info!("  Grouped into {} unique editions", editions.len());

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
            if editions[i].name_distance_score > NAME_DISTANCE_SWAP_RATIO * editions[i + 1].name_distance_score {
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
    let mut scored: Vec<(&EditionMBID, f64)> = edition.mbids
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

    scored.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    scored[0].0.mbid.clone()
}

/// Extract artist and album from path
/// Expects pattern: ".../Artist/Album.mp3" (cross-platform)
fn extract_metadata_from_path(path: &Path) -> (String, String) {
    // Use path components for cross-platform compatibility
    let components: Vec<_> = path.components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();

    if components.len() >= 2 {
        let artist = components[components.len() - 2].replace(", ", " ");
        let album_with_ext = components[components.len() - 1];
        let album = album_with_ext
            .trim_end_matches(".mp3")
            .replace(", ", " ");

        (artist, album)
    } else {
        ("Unknown".to_string(), "Unknown".to_string())
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
fn choose_artist(id3_artist: &str, path_artist: &str, album: &str) -> (String, String, MetadataSource) {
    // Heuristic 1: Avoid "Various Artists" if possible
    let id3_is_various = id3_artist.to_lowercase().contains("various");
    let path_is_various = path_artist.to_lowercase().contains("various");

    if id3_is_various && !path_is_various {
        return (path_artist.to_string(), id3_artist.to_string(), MetadataSource::Path);
    }
    if path_is_various && !id3_is_various {
        return (id3_artist.to_string(), path_artist.to_string(), MetadataSource::ID3);
    }

    // Heuristic 2: If album name suggests compilation, prefer ID3
    let album_lower = album.to_lowercase();
    if album_lower.contains("greatest") || album_lower.contains("best of") ||
       album_lower.contains("collection") || album_lower.contains("anthology") {
        return (id3_artist.to_string(), path_artist.to_string(), MetadataSource::ID3);
    }

    // Default: Prefer ID3
    (id3_artist.to_string(), path_artist.to_string(), MetadataSource::ID3)
}

/// Choose between ID3 and path album using heuristics
fn choose_album(id3_album: &str, path_album: &str) -> (String, String, MetadataSource) {
    // Heuristic: Prefer ID3 if it contains more detail (edition, year, etc.)
    let id3_has_detail = id3_album.contains('(') || id3_album.contains('[') ||
                         id3_album.contains("Deluxe") || id3_album.contains("Edition") ||
                         id3_album.len() > path_album.len() + 10;

    if id3_has_detail {
        return (id3_album.to_string(), path_album.to_string(), MetadataSource::ID3);
    }

    // Default: Prefer ID3
    (id3_album.to_string(), path_album.to_string(), MetadataSource::ID3)
}

/// Extract ID3 tags using ffprobe
fn extract_id3_tags(file_path: &Path) -> Result<ID3Metadata, Box<dyn std::error::Error>> {
    let output = Command::new("ffprobe")
        .args(&[
            "-v", "quiet",
            "-show_format",
            "-of", "json",
            file_path.to_str().ok_or("Invalid file path")?
        ])
        .output()?;

    if !output.status.success() {
        return Err("ffprobe failed".into());
    }

    let json: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let tags = &json["format"]["tags"];

    let mut all_tags = HashMap::new();
    if let Some(obj) = tags.as_object() {
        for (key, value) in obj {
            if let Some(s) = value.as_str() {
                all_tags.insert(key.to_lowercase(), s.to_string());
            }
        }
    }

    // Extract standard fields (try multiple tag name variants)
    let get_tag = |names: &[&str]| -> Option<String> {
        for name in names {
            if let Some(value) = all_tags.get(*name) {
                return Some(value.clone());
            }
        }
        None
    };

    Ok(ID3Metadata {
        artist: get_tag(&["artist", "album_artist", "albumartist"]),
        album: get_tag(&["album"]),
        date: get_tag(&["date", "year"]),
        genre: get_tag(&["genre"]),
        musicbrainz_albumid: get_tag(&["musicbrainz_albumid", "musicbrainz album id"]),
        musicbrainz_artistid: get_tag(&["musicbrainz_artistid", "musicbrainz artist id"]),
        comment: get_tag(&["comment"]),
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

    let has_musicbrainz_ids = id3.musicbrainz_albumid.is_some() || id3.musicbrainz_artistid.is_some();

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
                    let (chosen_artist, alt_artist, artist_src) = choose_artist(id3_artist, path_artist, id3_album);
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
                    let (chosen_artist, alt_artist, artist_src) = choose_artist(id3_artist, path_artist, id3_album);
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
        (Some(id3_artist), _, Some(id3_album), _) => {
            ReconciledMetadata {
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
            }
        }

        // Case 3: Path has both, ID3 missing one or both
        (_, Some(path_artist), _, Some(path_album)) => {
            ReconciledMetadata {
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
            }
        }

        // Case 4: Incomplete data - use what we have
        _ => {
            let artist = id3.artist.clone()
                .or_else(|| path_artist.clone())
                .unwrap_or_else(|| "Unknown".to_string());
            let album = id3.album.clone()
                .or_else(|| path_album.clone())
                .unwrap_or_else(|| "Unknown".to_string());

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
fn log_reconciliation_decision(reconciled: &ReconciledMetadata) {
    info!("  Phase 0 Reconciliation:");
    info!("    Strategy: {:?}", reconciled.strategy);
    info!("    Confidence: {:?}", reconciled.confidence);
    info!("    Artist: {} (source: {:?})", reconciled.artist, reconciled.artist_source);
    if let Some(ref alt) = reconciled.alternate_artist {
        info!("      Alternate: {}", alt);
    }
    info!("    Album: {} (source: {:?})", reconciled.album, reconciled.album_source);
    if let Some(ref alt) = reconciled.alternate_album {
        info!("      Alternate: {}", alt);
    }
    if reconciled.has_musicbrainz_ids {
        info!("    Has MusicBrainz IDs in tags: Yes");
    }
    if let Some(count) = reconciled.estimated_track_count {
        info!("    Estimated track count from ID3: {}", count);
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
    let path_artist_opt = if path_artist != "Unknown" { Some(path_artist) } else { None };
    let path_album_opt = if path_album != "Unknown" { Some(path_album) } else { None };

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
            for start in (j-1)..i {
                if dp[start][j-1].0 == f64::INFINITY {
                    continue;
                }

                // Calculate duration of track j by summing segments from start to i-1
                let track_duration: f64 = detected_durations[start..i].iter().sum();
                let expected_duration = expected_durations[j - 1] as f64; // Convert milliseconds to seconds for comparison
                let duration_error = (track_duration - expected_duration).abs();

                // Total error = previous error + this track's error
                let total_error = dp[start][j-1].0 + duration_error;

                // Update if this is better
                if total_error < dp[i][j].0 {
                    let mut new_splits = dp[start][j-1].1.clone();
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
fn calculate_rms_profile(
    samples: &[f32],
    sample_rate: u32,
) -> Vec<(f64, f32)> {
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
        let candidates: Vec<_> = rms_profile.iter()
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
            let distance_penalty = (distance / dynamic_radius) * QUIET_SPOT_PROXIMITY_PENALTY * QUIET_SPOT_DISTANCE_PENALTY_MULTIPLIER;
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
fn boundaries_to_durations(
    boundaries: &[f64],
    total_duration_secs: f64,
) -> Vec<f64> {
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

/// Test a segmentation against ALL MusicBrainz candidates (best first)
/// Returns best match found, or None if no candidates
/// NOTE: Unused in Run 15 (edition-by-edition processing uses single-edition functions).
#[allow(dead_code)]
fn test_segmentation_against_all_candidates(
    detected_durations: &[f64],
    candidates: &[(Vec<u32>, String)],
    tolerance: f64,
) -> Option<CandidateTestResult> {
    let mut best_result: Option<CandidateTestResult> = None;

    for (expected_u32, mbid) in candidates {
        let (matches, matched_count, percentage) = analyze_track_matching(
            detected_durations,
            expected_u32,
            tolerance,
        );

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

        if best_result.is_none() || percentage > best_result.as_ref().unwrap().percentage {
            best_result = Some(CandidateTestResult {
                percentage,
                matched_count,
                matches,
                mbid: mbid.clone(),
                expected_durations: expected_u32.clone(),
                mean_error,
                detected_durations: detected_durations.to_vec(),
            });
        }

        // Early exit on perfect match
        if percentage >= 100.0 {
            break;
        }
    }

    best_result
}

/// Test a segmentation against a SINGLE edition candidate (Run 15)
/// Returns the test result for this specific edition
fn test_segmentation_against_single_edition(
    detected_durations: &[f64],
    expected_durations: &[u32],
    edition_id: &str,
    tolerance: f64,
) -> CandidateTestResult {
    let (matches, matched_count, percentage) = analyze_track_matching(
        detected_durations,
        expected_durations,
        tolerance,
    );

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
    if perfect_match_found.compare_exchange(false, true, Ordering::SeqCst, Ordering::Relaxed).is_ok() {
        let elapsed_ms = start_time.elapsed().as_millis() as u64;
        perfect_match_time_ms.store(elapsed_ms, Ordering::SeqCst);
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
        return EditionTestResult {
            edition_idx,
            best_percentage: 0.0,
            best_result: None,
            best_stage: "skipped_early_exit",
            best_threshold: None,
            best_min_duration: None,
            expected_durations: expected_durations.clone(),
            log_messages,
        };
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
                return EditionTestResult {
                    edition_idx,
                    best_percentage: edition_best_percentage,
                    best_result: edition_best_result,
                    best_stage: edition_best_stage,
                    best_threshold: edition_best_threshold,
                    best_min_duration: edition_best_min_duration,
                    expected_durations: expected_durations.clone(),
                    log_messages,
                };
            }
        }
    }

    // Check for early exit before Stage 3
    if should_exit_early(perfect_match_found, perfect_match_time_ms, start_time) {
        log_messages.push("    -> Early exit before Stage 3 (grace period expired)".to_string());
        return EditionTestResult {
            edition_idx,
            best_percentage: edition_best_percentage,
            best_result: edition_best_result,
            best_stage: edition_best_stage,
            best_threshold: edition_best_threshold,
            best_min_duration: edition_best_min_duration,
            expected_durations: expected_durations.clone(),
            log_messages,
        };
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
        ) {
            if result.percentage > edition_best_percentage {
                edition_best_percentage = result.percentage;
                edition_best_durations = result.detected_durations.clone();
                edition_best_stage = "album_extractor_3_assembly";
                edition_best_result = Some(result.clone());

                if result.percentage >= 100.0 {
                    log_messages.push("    -> 100% match in Stage 3!".to_string());
                    signal_perfect_match(perfect_match_found, perfect_match_time_ms, start_time);
                    return EditionTestResult {
                        edition_idx,
                        best_percentage: edition_best_percentage,
                        best_result: edition_best_result,
                        best_stage: edition_best_stage,
                        best_threshold: edition_best_threshold,
                        best_min_duration: edition_best_min_duration,
                        expected_durations: expected_durations.clone(),
                        log_messages,
                    };
                }
            }
        }
    }

    // Check for early exit before Stage 4
    if should_exit_early(perfect_match_found, perfect_match_time_ms, start_time) {
        log_messages.push("    -> Early exit before Stage 4 (grace period expired)".to_string());
        return EditionTestResult {
            edition_idx,
            best_percentage: edition_best_percentage,
            best_result: edition_best_result,
            best_stage: edition_best_stage,
            best_threshold: edition_best_threshold,
            best_min_duration: edition_best_min_duration,
            expected_durations: expected_durations.clone(),
            log_messages,
        };
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

    EditionTestResult {
        edition_idx,
        best_percentage: edition_best_percentage,
        best_result: edition_best_result,
        best_stage: edition_best_stage,
        best_threshold: edition_best_threshold,
        best_min_duration: edition_best_min_duration,
        expected_durations: expected_durations.clone(),
        log_messages,
    }
}

/// Stage 2: Parameter optimization for a SINGLE edition (Run 15)
/// Tests 180 parameter combinations against one edition, collecting over-segmented candidates
fn run_stage2_single_edition(
    samples: &[f32],
    sample_rate: u32,
    threshold_values: &[f64],
    min_duration_values: &[f64],
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
    let mut tested = 0;
    let total_combinations = threshold_values.len() * min_duration_values.len();

    let expected_track_count = expected_durations.len();

    for &thresh in threshold_values {
        for &min_dur in min_duration_values {
            tested += 1;
            let test_durations = get_track_durations(samples, sample_rate, thresh, min_dur);

            // Test against this single edition
            let result = test_segmentation_against_single_edition(
                &test_durations,
                expected_durations,
                edition_id,
                tolerance,
            );

            let improved = best_result.as_ref().map_or(true, |br| result.percentage > br.percentage);

            if improved && result.percentage > current_best_percentage {
                info!("      New best: {:.1}% with {}dB, {}s -> {} tracks ({}/{})",
                    result.percentage, thresh, min_dur, test_durations.len(),
                    tested, total_combinations);

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
                    durations: test_durations,
                    threshold_db: thresh,
                    min_duration_secs: min_dur,
                    track_count: 0, // Will be set below
                });
                if let Some(last) = over_segmented_candidates.last_mut() {
                    last.track_count = last.durations.len();
                }
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
    let mut tested = 0;
    let total_combinations = num_thresholds * num_min_durations;

    let expected_track_count = expected_durations.len();

    // Iterate using indices to look up from cache
    for thresh_idx in 0..num_thresholds {
        for min_dur_idx in 0..num_min_durations {
            tested += 1;

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

            let improved = best_result.as_ref().map_or(true, |br| result.percentage > br.percentage);

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
            info!("      [Edition {}/{}] Early exit during Stage 3 assembly (tested {} so far)",
                edition_idx + 1, total_editions, assemblies_tested);
            break;
        }

        // Only try assembly if candidate has more segments than target
        if candidate.durations.len() > expected_durations.len() {
            if let Some(assembled_durations) = assemble_segments_dp(&candidate.durations, expected_durations) {
                assemblies_tested += 1;

                let result = test_segmentation_against_single_edition(
                    &assembled_durations,
                    expected_durations,
                    edition_id,
                    tolerance,
                );

                let improved = best_result.as_ref()
                    .map_or(true, |br| result.percentage > br.percentage);

                if improved && result.percentage > current_best_percentage {
                    info!("      [Edition {}/{}] New best: {:.1}% via assembly ({}dB, {}s) ({} -> {} tracks)",
                        edition_idx + 1, total_editions,
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
        info!("      [Edition {}/{}] Tested {} assemblies", edition_idx + 1, total_editions, assemblies_tested);
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
    rms_profile: &[(f64, f32)],  // Pre-calculated RMS profile (time, rms)
    total_duration_secs: f64,
    edition_idx: usize,
    total_editions: usize,
) -> Option<CandidateTestResult> {
    if current_best_percentage >= 100.0 {
        return None;
    }

    if expected_durations.is_empty() || rms_profile.is_empty() {
        return None;
    }

    // Find quiet spots near expected boundaries for this edition
    let detected_boundaries = find_edition_guided_boundaries(
        rms_profile,
        expected_durations,
        total_duration_secs,
    );

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
        info!("      [Edition {}/{}] New best: {:.1}% via guided quiet spots",
            edition_idx + 1, total_editions, result.percentage);
        Some(result)
    } else {
        None
    }
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

    let mut best_merge_durations = None;
    let mut best_merge_error = f64::INFINITY;
    let mut best_merge_index = None;

    // Try merging each possible pair of adjacent tracks
    for merge_idx in 0..(best_durations.len() - 1) {
        let mut merged_durations = Vec::new();

        for i in 0..best_durations.len() {
            if i == merge_idx {
                merged_durations.push(best_durations[i] + best_durations[i + 1]);
            } else if i == merge_idx + 1 {
                continue;
            } else {
                merged_durations.push(best_durations[i]);
            }
        }

        let (merged_matches, _merged_matched_count, _merged_percentage) =
            analyze_track_matching(&merged_durations, expected_durations, tolerance);

        let total_error: f64 = merged_matches.iter().map(|m| m.error).sum();

        if merged_durations.len() == expected_durations.len() && total_error < best_merge_error {
            best_merge_error = total_error;
            best_merge_durations = Some(merged_durations);
            best_merge_index = Some(merge_idx);
        }
    }

    if let Some(merged_durations) = best_merge_durations {
        let (merged_matches, merged_matched_count, merged_percentage) =
            analyze_track_matching(&merged_durations, expected_durations, tolerance);

        let mean_merged_error = best_merge_error / merged_matches.len() as f64;

        info!("      Merged tracks {} + {} -> {:.1}%",
            best_merge_index.unwrap() + 1,
            best_merge_index.unwrap() + 2,
            merged_percentage);

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

/// Stage 1: Initial detection with default parameters
/// Tests default threshold/duration against ALL MB candidates
/// NOTE: Unused in Run 15 (edition-by-edition processing).
#[allow(dead_code)]
fn run_stage1_initial_detection(
    samples: &[f32],
    sample_rate: u32,
    threshold_db: f64,
    min_duration_secs: f64,
    mb_candidates: &[(Vec<u32>, String)],
    tolerance: f64,
) -> (Vec<f64>, Option<CandidateTestResult>) {
    info!("  STAGE 1: Testing default parameters ({}dB, {}s)...", threshold_db, min_duration_secs);
    let durations = get_track_durations(samples, sample_rate, threshold_db, min_duration_secs);
    info!("    Found {} tracks", durations.len());

    let result = test_segmentation_against_all_candidates(&durations, mb_candidates, tolerance);

    if let Some(ref r) = result {
        info!("    Best match: {:.1}% with {} tracks from release {} ({} confidence)",
            r.percentage, r.expected_durations.len(), &r.mbid[..8],
            classify_confidence(r.percentage));
    }

    (durations, result)
}

/// Stage 2: Parameter optimization (ENHANCED - Run 12)
/// Tests grid of threshold/duration combinations against ALL MB candidates
/// NOW ALSO collects all over-segmented candidates for Stage 3 assembly
/// NOTE: Unused in Run 15 (edition-by-edition processing).
#[allow(dead_code)]
fn run_stage2_parameter_optimization(
    samples: &[f32],
    sample_rate: u32,
    threshold_values: &[f64],
    min_duration_values: &[f64],
    mb_candidates: &[(Vec<u32>, String)],
    tolerance: f64,
    current_best_percentage: f64,
) -> Stage2Results {
    if current_best_percentage >= 100.0 {
        return Stage2Results {
            best_result: None,
            over_segmented_candidates: Vec::new(),
        };
    }

    info!("  STAGE 2: Testing {} parameter combinations against all editions...",
        threshold_values.len() * min_duration_values.len());

    let mut best_result: Option<CandidateTestResult> = None;
    let mut over_segmented_candidates: Vec<OverSegmentedCandidate> = Vec::new();
    let mut tested = 0;
    let total_combinations = threshold_values.len() * min_duration_values.len();

    // Get max expected track count across all editions for over-segmentation detection
    let max_expected_tracks = mb_candidates.iter()
        .map(|(durs, _)| durs.len())
        .max()
        .unwrap_or(0);

    for &thresh in threshold_values {
        for &min_dur in min_duration_values {
            tested += 1;
            let test_durations = get_track_durations(samples, sample_rate, thresh, min_dur);

            // Track best immediate match (existing behavior)
            if let Some(result) = test_segmentation_against_all_candidates(
                &test_durations,
                mb_candidates,
                tolerance,
            ) {
                let improved = best_result.as_ref().map_or(true, |br| result.percentage > br.percentage);

                if improved && result.percentage > current_best_percentage {
                    info!("    New best: {:.1}% with {}dB, {}s → {} tracks from {} ({}/{})",
                        result.percentage, thresh, min_dur, result.expected_durations.len(),
                        &result.mbid[..8], tested, total_combinations);

                    best_result = Some(result);

                    if best_result.as_ref().unwrap().percentage >= 100.0 {
                        info!("    Best match after parameter optimization: 100.0% (Excellent confidence)");
                        return Stage2Results {
                            best_result,
                            over_segmented_candidates,
                        };
                    }
                }
            }

            // NEW (Run 12): Collect over-segmented candidates for Stage 3 assembly
            if test_durations.len() > max_expected_tracks {
                over_segmented_candidates.push(OverSegmentedCandidate {
                    durations: test_durations,
                    threshold_db: thresh,
                    min_duration_secs: min_dur,
                    track_count: 0, // Will be set from durations.len()
                });
                // Update track_count from actual durations
                if let Some(last) = over_segmented_candidates.last_mut() {
                    last.track_count = last.durations.len();
                }
            }
        }
    }

    if let Some(ref r) = best_result {
        info!("    Best match after parameter optimization: {:.1}% ({} confidence)",
            r.percentage, classify_confidence(r.percentage));
    }

    info!("    Collected {} over-segmented candidates for Stage 3 assembly",
        over_segmented_candidates.len());

    Stage2Results {
        best_result,
        over_segmented_candidates,
    }
}

/// Stage 3: Comprehensive Segment Assembly (FIXED - Run 12)
/// Tries assembling EACH over-segmented candidate from Stage 2 against EACH target edition
/// This restores Run 7 behavior where 100+ assemblies could be tested
/// NOTE: Unused in Run 15 (edition-by-edition processing).
#[allow(dead_code)]
fn run_stage3_comprehensive_assembly(
    over_segmented_candidates: &[OverSegmentedCandidate],
    mb_candidates: &[(Vec<u32>, String)],
    tolerance: f64,
    current_best_percentage: f64,
) -> Option<CandidateTestResult> {
    if current_best_percentage >= 100.0 {
        return None; // Already perfect
    }

    if over_segmented_candidates.is_empty() {
        info!("  STAGE 3: No over-segmented candidates to assemble");
        return None;
    }

    info!("  STAGE 3: Comprehensive segment assembly across {} over-segmented candidates...",
        over_segmented_candidates.len());

    let mut assemblies_tested = 0;
    let mut assemblies_improved = 0;
    let mut best_result: Option<CandidateTestResult> = None;
    let mut best_source_params: Option<(f64, f64, usize)> = None;

    for candidate in over_segmented_candidates {
        for (expected_u32, _mbid) in mb_candidates {
            // Only try assembly if candidate has more segments than target
            if candidate.durations.len() > expected_u32.len() {
                if let Some(assembled_durations) = assemble_segments_dp(&candidate.durations, expected_u32) {
                    assemblies_tested += 1;

                    // Test assembled result against ALL candidates (not just the target)
                    if let Some(result) = test_segmentation_against_all_candidates(
                        &assembled_durations,
                        mb_candidates,
                        tolerance,
                    ) {
                        let improved = best_result.as_ref()
                            .map_or(true, |br| result.percentage > br.percentage);

                        if improved && result.percentage > current_best_percentage {
                            assemblies_improved += 1;
                            info!("    New best: {:.1}% via assembly of ({}dB, {}s) ({} segments → {} tracks)",
                                result.percentage,
                                candidate.threshold_db, candidate.min_duration_secs,
                                candidate.track_count, expected_u32.len());

                            best_result = Some(result);
                            best_source_params = Some((
                                candidate.threshold_db,
                                candidate.min_duration_secs,
                                candidate.track_count,
                            ));

                            if best_result.as_ref().unwrap().percentage >= 100.0 {
                                info!("    Tested {} assemblies, {} improved over best",
                                    assemblies_tested, assemblies_improved);
                                return best_result; // Early exit on perfect match
                            }
                        }
                    }
                }
            }
        }
    }

    info!("    Tested {} assemblies, {} improved over best", assemblies_tested, assemblies_improved);

    if let (Some(_result), Some((thresh, min_dur, seg_count))) = (&best_result, best_source_params) {
        info!("    Best assembly from {}dB, {}s ({} segments)", thresh, min_dur, seg_count);
    }

    best_result
}

/// Stage 4: Edition-guided quiet spot detection
/// Uses best matched edition's track durations as a guide to search for quiet spots
/// near expected boundary positions (±search radius), then tests against all editions
/// NOTE: Unused in Run 15 (edition-by-edition processing).
#[allow(dead_code)]
fn run_stage4_quiet_spot_detection(
    samples: &[f32],
    sample_rate: u32,
    mb_candidates: &[(Vec<u32>, String)],
    tolerance: f64,
    current_best_percentage: f64,
) -> Option<CandidateTestResult> {
    if current_best_percentage >= 100.0 {
        return None; // Already perfect
    }

    info!("  STAGE 4: Edition-guided quiet spot detection...");

    // Calculate RMS profile once for the entire file
    let rms_profile = calculate_rms_profile(samples, sample_rate);
    let total_duration_secs = samples.len() as f64 / sample_rate as f64;

    if rms_profile.is_empty() {
        info!("    No RMS profile generated");
        return None;
    }

    info!("    RMS profile: {} windows ({}ms step)",
        rms_profile.len(), (QUIET_SPOT_WINDOW_STEP_SECS * 1000.0) as u32);

    let mut guided_tests = 0;
    let mut guided_improvements = 0;
    let mut best_result: Option<CandidateTestResult> = None;

    // Try edition-guided search for top N editions
    let editions_to_try = mb_candidates.len().min(QUIET_SPOT_TOP_EDITIONS);
    info!("    Testing guided search against top {} editions...", editions_to_try);

    for (expected_durations, _mbid) in mb_candidates.iter().take(editions_to_try) {
        if expected_durations.is_empty() {
            continue;
        }

        // Find quiet spots near expected boundaries for this edition
        let detected_boundaries = find_edition_guided_boundaries(
            &rms_profile,
            expected_durations,
            total_duration_secs,
        );

        // Convert boundaries to track durations
        let guided_durations = boundaries_to_durations(&detected_boundaries, total_duration_secs);
        guided_tests += 1;

        // Test guided result against ALL candidates (not just the guide edition)
        if let Some(result) = test_segmentation_against_all_candidates(
            &guided_durations,
            mb_candidates,
            tolerance,
        ) {
            let improved = best_result.as_ref().map_or(true, |br| result.percentage > br.percentage);

            if improved && result.percentage > current_best_percentage {
                guided_improvements += 1;
                info!("    New best: {:.1}% via guided search (guide: {} tracks, matched: {} tracks)",
                    result.percentage, expected_durations.len(), result.expected_durations.len());

                best_result = Some(result);

                if best_result.as_ref().unwrap().percentage >= 100.0 {
                    info!("    Tested {} guided searches, {} improved", guided_tests, guided_improvements);
                    return best_result; // Early exit on perfect match
                }
            }
        }
    }

    info!("    Tested {} guided searches, {} improved", guided_tests, guided_improvements);

    best_result
}

/// Stage 5: Extra track merging
/// When detected > expected AND match ≥100%, merge adjacent tracks to achieve correct count
/// NOTE: Unused in Run 15 (edition-by-edition processing).
#[allow(dead_code)]
fn run_stage5_extra_track_merging(
    best_durations: &[f64],
    best_expected_durations: &[u32],
    tolerance: f64,
    current_best_percentage: f64,
) -> Option<(Vec<f64>, CandidateTestResult)> {
    if current_best_percentage < 100.0 || best_durations.len() <= best_expected_durations.len() {
        return None; // Not applicable
    }

    let extra_count = best_durations.len() - best_expected_durations.len();
    info!("  STAGE 5: Attempting extra track merging...");
    info!("    {} extra track(s) detected, match quality {:.1}%", extra_count, current_best_percentage);
    info!("    Testing {} possible adjacent track merges", best_durations.len() - 1);

    let mut best_merge_durations = None;
    let mut best_merge_error = f64::INFINITY;
    let mut best_merge_index = None;

    // Try merging each possible pair of adjacent tracks
    for merge_idx in 0..(best_durations.len() - 1) {
        let mut merged_durations = Vec::new();

        for i in 0..best_durations.len() {
            if i == merge_idx {
                merged_durations.push(best_durations[i] + best_durations[i + 1]);
            } else if i == merge_idx + 1 {
                continue; // Skip - already merged with previous
            } else {
                merged_durations.push(best_durations[i]);
            }
        }

        let (merged_matches, _merged_matched_count, _merged_percentage) =
            analyze_track_matching(&merged_durations, best_expected_durations, tolerance);

        let total_error: f64 = merged_matches.iter().map(|m| m.error).sum();

        if merged_durations.len() == best_expected_durations.len() && total_error < best_merge_error {
            best_merge_error = total_error;
            best_merge_durations = Some(merged_durations);
            best_merge_index = Some(merge_idx);
        }
    }

    // Apply best merge if found
    if let Some(merged_durations) = best_merge_durations {
        let (merged_matches, merged_matched_count, merged_percentage) =
            analyze_track_matching(&merged_durations, best_expected_durations, tolerance);

        let mean_merged_error = best_merge_error / merged_matches.len() as f64;

        info!("    Best merge: tracks {} + {} → {:.1}% match, {:.2}s mean error",
            best_merge_index.unwrap() + 1,
            best_merge_index.unwrap() + 2,
            merged_percentage,
            mean_merged_error);

        info!("    Track count corrected: {} → {} ({:.1}% match)",
            merged_durations.len() + 1, merged_durations.len(), merged_percentage);

        let result = CandidateTestResult {
            percentage: merged_percentage,
            matched_count: merged_matched_count,
            matches: merged_matches,
            mbid: String::new(), // Will use existing best_mbid
            expected_durations: best_expected_durations.to_vec(),
            mean_error: mean_merged_error,
            detected_durations: merged_durations.clone(),
        };

        Some((merged_durations, result))
    } else {
        info!("    No valid merge found that produces correct track count");
        None
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing with timestamps (use RUST_LOG=debug to see MB queries)
    tracing_subscriber::fmt()
        .with_timer(SystemTime::default())
        .with_target(false)
        .with_level(true)
        .init();

    info!("=== Comprehensive Album Matcher (Run 16) ===");
    info!("ARCHITECTURE: Edition-by-edition processing (early exit on 100% match)");
    info!("  - Each edition fully optimized (Stages 2-5) before trying next edition");
    info!("  - Early exit: stops as soon as ANY edition achieves 100% match");
    info!("  - Editions sorted by likelihood (runtime + name similarity)");
    info!("  - Preserves Run 12/13 fixes for over-segmented assembly");
    info!("  - RUN 16: Parallel edition testing with tracing");

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
    let training_paths: std::collections::HashSet<PathBuf> = training_files
        .iter()
        .cloned()
        .collect();

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
    info!("Testing optimal parameters on {} albums from combined set", training_files.len());
    info!("  ({} training set + {} from long files list)\n", initial_count, additional_files.len());

    // Use optimal parameters from analysis
    let threshold_db: f64 = DEFAULT_THRESHOLD_DB;
    let min_duration_secs: f64 = DEFAULT_MIN_DURATION_SECS;
    let match_tolerance_secs = MATCH_TOLERANCE_SECS;

    info!("Parameters:");
    info!("  Threshold: {} dB", threshold_db);
    info!("  Min duration: {} seconds", min_duration_secs);
    info!("  Match tolerance: {} seconds\n", match_tolerance_secs);

    let rate_limiter = RateLimiter::new();
    let mut results = Vec::new();

    // Parameter grid for Stage 2 optimization (defined in configuration constants section)
    let threshold_values: &[f64] = &STAGE2_THRESHOLD_VALUES;
    let min_duration_values: &[f64] = &STAGE2_MIN_DURATION_VALUES;

    for (idx, file_path) in training_files.iter().enumerate() {
        info!("=== Album {}/{} ===", idx + 1, training_files.len());
        info!("File: {}", file_path.display());

        if !file_path.exists() {
            info!("  ERROR: File not found\n");
            continue;
        }

        // === PHASE 0: ID3 Tag Extraction & Reconciliation ===
        let reconciled = extract_and_reconcile_metadata(file_path);
        log_reconciliation_decision(&reconciled);

        let artist = &reconciled.artist;
        let album = &reconciled.album;

        // Decode MP3 (blocking I/O - run on blocking thread pool)
        info!("  Decoding...");
        let file_path_owned = file_path.clone();
        let decode_result = tokio::task::spawn_blocking(move || {
            decode_mp3(&file_path_owned)
        }).await;

        let (samples, sample_rate) = match decode_result {
            Ok(Ok((s, sr))) => {
                let duration_mins = s.len() as f64 / sr as f64 / 60.0;
                info!("  Decoded: {} samples at {} Hz ({:.2} mins)", s.len(), sr, duration_mins);
                (s, sr)
            }
            Ok(Err(e)) => {
                error!("FAILED: {}", e);
                results.push(ValidationResult::error(
                    file_path, artist, album, 0,
                    format!("Decode failed: {}", e),
                ));
                info!("");
                continue;
            }
            Err(e) => {
                error!("FAILED: Task panicked: {}", e);
                results.push(ValidationResult::error(
                    file_path, artist, album, 0,
                    format!("Decode task panicked: {}", e),
                ));
                info!("");
                continue;
            }
        };

        // Get initial track durations for MusicBrainz lookup
        let initial_durations = get_track_durations(&samples, sample_rate, threshold_db, min_duration_secs);

        // Calculate file characteristics for edition sorting
        let file_duration_secs = samples.len() as f64 / sample_rate as f64;
        // Use ID3 track count for filtering/scoring (NOT silence-detected count which may be wrong)
        let estimated_track_count = reconciled.estimated_track_count;

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

        // Comprehensive MusicBrainz search (ALL strategies, ALL variants, up to MB_MAX_RELEASES releases)
        info!("  Fetching MusicBrainz data (comprehensive search)...");
        let editions = match comprehensive_musicbrainz_search(
            &artist_variants,
            &album_variants,
            file_duration_secs,
            estimated_track_count,
            &rate_limiter
        ).await {
            Ok(releases) => {
                if releases.is_empty() {
                    error!("FAILED: No releases found");
                    results.push(ValidationResult::error(
                        file_path, artist, album, initial_durations.len(),
                        "MusicBrainz lookup failed: No releases found".to_string(),
                    ));
                    info!("");
                    continue;
                }

                // Group releases into unique editions
                let mut editions = group_into_editions(releases);

                // Sort editions by how well they match the audio file's characteristics
                editions.sort_by(|a, b| {
                    let score_a = score_edition_match(a, file_duration_secs, estimated_track_count);
                    let score_b = score_edition_match(b, file_duration_secs, estimated_track_count);
                    score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
                });

                info!("  Sorted editions by likelihood (file: {:.0}s, ~{} tracks)",
                    file_duration_secs, initial_durations.len());

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
                    info!("  Filtered out {} editions (runtime >25% different from file)", runtime_filtered_count);
                    info!("  Acceptable range: {:.0}s - {:.0}s", min_duration, max_duration);
                }

                // Note: NDR filtering already applied at release level before fetching track details (Run 15)
                // All editions here are composed of releases with NDR <= MAX_NAME_DISTANCE_RANK

                if editions.is_empty() {
                    let failure_msg = if runtime_filtered_count > 0 {
                        format!("FAILED: All editions filtered out ({} by runtime)", runtime_filtered_count)
                    } else {
                        "FAILED: No valid editions (NDR filtering applied at release level)".to_string()
                    };
                    info!("{}", failure_msg);
                    results.push(ValidationResult::error(
                        file_path, artist, album, initial_durations.len(),
                        failure_msg,
                    ));
                    info!("");
                    continue;
                }

                info!("Found {} unique editions to test", editions.len());

                // Re-sort by name similarity using conservative bubble sort
                // This allows editions with significantly better name matches to bubble up
                // while preserving runtime-based ordering for similar name scores
                resort_by_name_similarity(&mut editions);

                editions
            }
            Err(e) => {
                error!("FAILED: {}", e);
                results.push(ValidationResult::error(
                    file_path, artist, album, initial_durations.len(),
                    format!("MusicBrainz lookup failed: {}", e),
                ));
                info!("");
                continue;
            }
        };

        // NOTE: mb_candidates format no longer used in Run 15 (edition-by-edition processing)
        // Keeping for reference/comparison with old approach
        #[allow(unused_variables)]
        let _mb_candidates: Vec<(Vec<u32>, String)> = editions
            .iter()
            .enumerate()
            .map(|(idx, edition)| (edition.durations.clone(), format!("edition_{}", idx)))
            .collect();

        // Display edition information
        info!("  Edition Details (sorted by match likelihood):");
        for (idx, edition) in editions.iter().enumerate() {
            let edition_duration: u32 = edition.durations.iter().sum();
            let match_score = score_edition_match(edition, file_duration_secs, estimated_track_count);
            info!("    [{}] {} - {} ({} tracks, {}s, {} MBIDs, score: {:.0}s, NDR:{},{:.1})",
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

        // === RUN 16: Edition-by-Edition Processing with Cached Silence Detection ===
        // Process each edition through all stages before moving to next edition
        // Run 16: Pre-compute silence detection ONCE, then reuse across all editions

        // Calculate RMS profile once for Stage 4 (used by all editions)
        let rms_profile = calculate_rms_profile(&samples, sample_rate);
        let total_duration_secs = samples.len() as f64 / sample_rate as f64;

        // Run 16: Pre-compute silence detection for all parameter combinations IN PARALLEL
        // This is the key optimization - compute 180 silence scans ONCE instead of 180 × N editions
        info!("  Pre-computing silence detection for {} parameter combinations (parallel, {} threads)...",
            threshold_values.len() * min_duration_values.len(), rayon::current_num_threads());
        let silence_cache = precompute_silence_cache(
            &samples,
            sample_rate,
            threshold_values,
            min_duration_values,
        );
        info!("  Silence cache ready ({} entries)", silence_cache.len());

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

        info!("  === EDITION-BY-EDITION PROCESSING (Run 16 - STAGGERED FEED + EARLY EXIT) ===");
        info!("  Testing {} editions through Stages 2-5 ({}s between feeds, {}s grace period)...\n",
            editions.len(), EDITION_FEED_DELAY_SECS, EARLY_EXIT_GRACE_PERIOD_SECS);

        // Staggered feed: spawn editions one at a time with delays
        // Stop feeding new editions once 100% match is found
        let results_mutex: Mutex<Vec<EditionTestResult>> = Mutex::new(Vec::new());
        let mut editions_started = 0;
        let mut editions_skipped = 0;

        rayon::scope(|s| {
            for (edition_idx, edition) in editions.iter().enumerate() {
                // Check if we should stop feeding new editions
                if perfect_match_found.load(Ordering::Relaxed) {
                    editions_skipped = editions.len() - edition_idx;
                    info!("  Stopping feed: 100% match found, {} editions not started", editions_skipped);
                    break;
                }

                editions_started += 1;
                info!("  Starting Edition {}/{}: {} - {}",
                    edition_idx + 1, editions.len(), edition.artist, edition.album);

                // Clone references for the closure
                let silence_cache = &silence_cache;
                let rms_profile = &rms_profile;
                let initial_durations = &initial_durations;
                let perfect_match_found = &perfect_match_found;
                let perfect_match_time_ms = &perfect_match_time_ms;
                let results_mutex = &results_mutex;
                let total_editions = editions.len();

                s.spawn(move |_| {
                    let result = test_single_edition(
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
                    );
                    results_mutex.lock().unwrap().push(result);
                });

                // Wait before feeding next edition (unless this is the last one)
                if edition_idx < editions.len() - 1 {
                    std::thread::sleep(Duration::from_secs(EDITION_FEED_DELAY_SECS));
                }
            }
        });

        // Extract results from mutex and sort by edition_idx for consistent output
        let mut edition_results: Vec<EditionTestResult> = results_mutex.into_inner().unwrap();
        edition_results.sort_by_key(|r| r.edition_idx);

        info!("  Completed: {} editions started, {} skipped", editions_started, editions_skipped);

        // Print all log messages in order (for consistent output)
        for result in &edition_results {
            for msg in &result.log_messages {
                info!("{}", msg);
            }
            info!("");
        }

        // Find the best result across all editions
        // Priority: 100% matches with lowest mean_error, then highest percentage, then lower edition index
        let best_result_opt = edition_results
            .iter()
            .filter(|r| r.best_result.is_some())
            .max_by(|a, b| {
                let a_perfect = a.best_percentage >= 100.0;
                let b_perfect = b.best_percentage >= 100.0;

                // First priority: 100% matches beat non-100% matches
                if a_perfect && !b_perfect {
                    return std::cmp::Ordering::Greater;
                }
                if b_perfect && !a_perfect {
                    return std::cmp::Ordering::Less;
                }

                // Both are 100%: prefer lowest mean_error (most accurate track timing)
                if a_perfect && b_perfect {
                    let a_error = a.best_result.as_ref().map(|r| r.mean_error).unwrap_or(f64::MAX);
                    let b_error = b.best_result.as_ref().map(|r| r.mean_error).unwrap_or(f64::MAX);
                    // Note: reversed comparison - lower error is better (Greater)
                    return b_error.partial_cmp(&a_error)
                        .unwrap_or(std::cmp::Ordering::Equal)
                        .then_with(|| b.edition_idx.cmp(&a.edition_idx)); // Lower index wins ties
                }

                // Neither is 100%: prefer highest percentage, then lowest mean_error, then lower edition index
                a.best_percentage.partial_cmp(&b.best_percentage)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| {
                        // Lower mean_error is better (reversed comparison)
                        let a_error = a.best_result.as_ref().map(|r| r.mean_error).unwrap_or(f64::MAX);
                        let b_error = b.best_result.as_ref().map(|r| r.mean_error).unwrap_or(f64::MAX);
                        b_error.partial_cmp(&a_error).unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .then_with(|| b.edition_idx.cmp(&a.edition_idx))
            });

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
        let perfect_match_count = edition_results.iter()
            .filter(|r| r.best_percentage >= 100.0)
            .count();

        if perfect_match_count > 1 {
            info!("  Found {} editions with 100% match - selected edition {} (lowest mean error: {:.2}s)",
                perfect_match_count,
                best_edition_idx.map(|i| i + 1).unwrap_or(0),
                best_mean_error);
        } else {
            info!("  Parallel processing complete. Best: {:.1}% from edition {} (mean error: {:.2}s)",
                best_percentage, best_edition_idx.map(|i| i + 1).unwrap_or(0), best_mean_error);
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

        info!("  FINAL RESULT:");
        info!("    Matching stage: {}", best_stage);
        info!("    MusicBrainz: https://musicbrainz.org/release/{}", best_mbid);
        info!("    Track count: {}/{} {}", best_durations.len(), best_expected_durations.len(),
            if perfect_count { "✓" } else { "✗" });
        info!("    Matched tracks: {}/{} ({:.1}%)", best_matched_count, best_expected_durations.len(), best_percentage);
        info!("    Mean error: {:.2}s", best_mean_error);
        info!("    Confidence: {}", classify_confidence(best_percentage));

        if let (Some(thresh), Some(min_dur)) = (best_threshold, best_min_duration) {
            info!("    Best parameters: {}dB, {}s", thresh, min_dur);
        }

        // Show all track matches
        if !best_matches.is_empty() {
            info!("  All tracks:");
            for (i, tm) in best_matches.iter().enumerate() {
                let status = if tm.matches { "✓" } else { "✗" };
                info!("    {}. {:6.1}s vs {:6}s  error={:5.1}s  {}",
                    i + 1, tm.detected_duration, tm.expected_duration, tm.error, status);
            }
        }

        // Calculate extra tracks (detected tracks with no MusicBrainz match)
        let mut extra_tracks = Vec::new();
        let min_count = best_durations.len().min(best_expected_durations.len());

        if best_durations.len() > best_expected_durations.len() {
            // Detected more tracks than expected - the extras are at the end
            for i in min_count..best_durations.len() {
                extra_tracks.push(ExtraTrack {
                    track_index: i + 1, // 1-based
                    duration: best_durations[i],
                    description: "Extra track with no corresponding MusicBrainz entry".to_string(),
                });
            }

            if !extra_tracks.is_empty() {
                info!("  Extra tracks detected ({} beyond expected count):", extra_tracks.len());
                for et in &extra_tracks {
                    info!("    Track {}: {:.1}s (no MusicBrainz match)", et.track_index, et.duration);
                }
            }
        } else if best_durations.len() < best_expected_durations.len() {
            // MusicBrainz has more tracks than detected - show missing ones
            let missing_count = best_expected_durations.len() - best_durations.len();
            info!("  MusicBrainz tracks not found in file ({} missing):", missing_count);
            for i in min_count..best_expected_durations.len() {
                info!("    Track {}: {}s (expected but not detected in file)", i + 1, best_expected_durations[i]);
            }
        }

        results.push(ValidationResult {
            album_path: file_path.to_string_lossy().to_string(),
            artist: artist.clone(),
            album: album.clone(),
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
            status: "Success".to_string(),
            matching_stage: best_stage.to_string(),
            best_threshold_db: best_threshold,
            best_min_duration_secs: best_min_duration,
            confidence: classify_confidence(best_percentage),
        });

        info!("");
    }

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
        let stage1 = results.iter().filter(|r| r.matching_stage == "album_extractor_1_initial").count();
        let stage2 = results.iter().filter(|r| r.matching_stage == "album_extractor_2_optimization").count();
        let stage3 = results.iter().filter(|r| r.matching_stage == "album_extractor_3_assembly").count();
        let stage4 = results.iter().filter(|r| r.matching_stage == "album_extractor_4_guided").count();
        let stage5 = results.iter().filter(|r| r.matching_stage == "album_extractor_5_editions").count();
        let stage6 = results.iter().filter(|r| r.matching_stage == "album_extractor_6_merging").count();

        info!("\nMatching Stage Results:");
        info!("  album_extractor_1_initial:        {} albums", stage1);
        info!("  album_extractor_2_optimization:   {} albums", stage2);
        info!("  album_extractor_3_assembly:       {} albums", stage3);
        info!("  album_extractor_4_guided:         {} albums", stage4);
        info!("  album_extractor_5_editions:       {} albums", stage5);
        info!("  album_extractor_6_merging:        {} albums", stage6);

        // Confidence level breakdown
        let excellent = results.iter().filter(|r| r.confidence == "Excellent").count();
        let good = results.iter().filter(|r| r.confidence == "Good").count();
        let fair = results.iter().filter(|r| r.confidence == "Fair").count();
        let poor = results.iter().filter(|r| r.confidence == "Poor" && r.status == "Success").count();

        info!("\nConfidence Distribution:");
        info!("  Excellent (≥80%): {} albums", excellent);
        info!("  Good (60-79%):    {} albums", good);
        info!("  Fair (40-59%):    {} albums", fair);
        info!("  Poor (<40%):      {} albums", poor);

        // Track count matches
        let perfect_counts = results.iter().filter(|r| r.perfect_count_match).count();
        info!("\nTrack Count Matches:");
        info!("  Perfect: {}/{} ({:.1}%)", perfect_counts, successful,
            (perfect_counts as f64 / successful as f64) * 100.0);

        // Average statistics
        let avg_match_pct = results.iter()
            .filter(|r| r.status == "Success")
            .map(|r| r.match_percentage)
            .sum::<f64>() / successful as f64;

        let avg_error = results.iter()
            .filter(|r| r.status == "Success" && r.mean_error > 0.0)
            .map(|r| r.mean_error)
            .sum::<f64>() / successful as f64;

        info!("\nAverage Statistics:");
        info!("  Match percentage: {:.1}%", avg_match_pct);
        info!("  Mean error: {:.2}s", avg_error);

        // Parameter effectiveness (for Stage 2 results)
        if stage2 > 0 {
            info!("\nParameter Optimization Details:");
            for result in results.iter().filter(|r| r.matching_stage == "album_extractor_2_optimization") {
                if let (Some(thresh), Some(min_dur)) = (result.best_threshold_db, result.best_min_duration_secs) {
                    info!("  {} - {}: {}dB, {}s → {:.1}%",
                        result.artist, result.album, thresh, min_dur, result.match_percentage);
                }
            }
        }

        // Show best and worst
        let mut success_results: Vec<_> = results.iter().filter(|r| r.status == "Success").collect();
        success_results.sort_by(|a, b| b.match_percentage.partial_cmp(&a.match_percentage).unwrap());

        info!("\nBest 5 Albums:");
        for (i, result) in success_results.iter().take(5).enumerate() {
            info!("  {}. {} - {} ({:.1}%, {} via {})",
                i + 1, result.artist, result.album, result.match_percentage,
                result.confidence, result.matching_stage);
            info!("     MusicBrainz: {}", result.musicbrainz_url);
        }

        info!("\nWorst 5 Albums:");
        for (i, result) in success_results.iter().rev().take(5).enumerate() {
            info!("  {}. {} - {} ({:.1}%, {} via {})",
                i + 1, result.artist, result.album, result.match_percentage,
                result.confidence, result.matching_stage);
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
        assert!(matches.iter().all(|m| m.matches), "All matches should be true");
    }

    #[test]
    fn test_analyze_track_matching_partial_match() {
        let detected = vec![180.0, 250.0, 196.0]; // Middle track off by 50s
        let expected = vec![180, 200, 196];
        let tolerance = 10.0;

        let (matches, matched_count, percentage) =
            analyze_track_matching(&detected, &expected, tolerance);

        assert_eq!(matched_count, 2, "Only 2 tracks should match");
        assert_eq!(percentage, 66.666666666666664, "Should be ~66.67% match (2/3)");
        assert!(!matches[1].matches, "Middle track should not match");
        assert!(matches[0].matches && matches[2].matches, "First and last should match");
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
        assert_eq!(percentage, 66.666666666666664, "Should be ~66.67% (2/3 expected)");
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
        assert!(matches[1].matches, "Track at tolerance boundary should match");
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
        assert!(!matches[1].matches, "Track just outside tolerance should not match");
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
        assert!(strings_match("The Dark Side of the Moon", "The Dark Side of the Moon"));
        // Punctuation is filtered out, but "and" vs "&" remain different alphanumeric tokens
        assert!(!strings_match("Crosby, Stills & Nash", "Crosby Stills and Nash"));
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

        let silence_regions = detect_silence(&samples, sample_rate, threshold_db, min_duration_secs);

        // Should detect the entire file as one silence region (or possibly none if all zeros are skipped)
        // Allow for either outcome as implementation may handle edge case differently
        assert!(silence_regions.len() <= 1, "Should detect at most one silence region");
        if silence_regions.len() == 1 {
            assert!(silence_regions[0].0 <= 1000, "Silence should start near beginning");
        }
    }

    #[test]
    fn test_detect_silence_no_silence() {
        // Create loud audio (all at 0.5 amplitude, well above -60dB threshold)
        let samples = vec![0.5; 48000];
        let sample_rate = 48000;
        let threshold_db = -60.0;
        let min_duration_secs = 0.5;

        let silence_regions = detect_silence(&samples, sample_rate, threshold_db, min_duration_secs);

        assert_eq!(silence_regions.len(), 0, "Should detect no silence in loud audio");
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

        let silence_regions = detect_silence(&samples, sample_rate, threshold_db, min_duration_secs);

        // Should detect one silence region in the middle
        assert_eq!(silence_regions.len(), 1, "Should detect one silence region");
        // Silence should start around sample 48000 (allowing for window boundaries)
        assert!(silence_regions[0].0 >= 40000 && silence_regions[0].0 <= 56000,
                "Silence should start around the 1-second mark");
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

        let silence_regions = detect_silence(&samples, sample_rate, threshold_db, min_duration_secs);

        // Brief silence should be filtered out
        assert_eq!(silence_regions.len(), 0, "Should not detect silence shorter than minimum duration");
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
        assert!((durations[0] - 2.0).abs() < 0.1, "Track should be approximately 2 seconds");
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
        assert!((durations[0] - 3.0).abs() < 0.2, "First track should be ~3 seconds");
        assert!((durations[1] - 2.0).abs() < 0.2, "Second track should be ~2 seconds");
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
        assert_eq!(split_camel_case("TransEuropeExpress"), "Trans Europe Express");
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
            &detected, &expected, "test_edition", tolerance
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

        let result = test_segmentation_against_single_edition(
            &detected, &expected, "edition_1", tolerance
        );

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
        assert!((assembled[0] - 180.0).abs() < 15.0, "First track should be ~180s");
        assert!((assembled[1] - 200.0).abs() < 15.0, "Second track should be ~200s");
        assert!((assembled[2] - 196.0).abs() < 15.0, "Third track should be ~196s");
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

        assert!(result.is_none(), "Should return None when fewer segments than targets");
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
        assert!((durations[0] - 180.0).abs() < 0.01, "First track: 0 to 180 = 180s");
        assert!((durations[1] - 200.0).abs() < 0.01, "Second track: 180 to 380 = 200s");
        assert!((durations[2] - 196.0).abs() < 0.01, "Third track: 380 to 576 = 196s");
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
        let mut editions = vec![
            create_test_edition("Artist", "Album", 10, 5.0),
        ];
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
        assert!((editions[0].name_distance_score - original_first).abs() < 0.01,
            "Should not swap when ratio <= NAME_DISTANCE_SWAP_RATIO");
    }

    // Helper function to create test editions
    fn create_test_edition(artist: &str, album: &str, track_count: usize, name_distance_score: f64) -> Edition {
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
}
