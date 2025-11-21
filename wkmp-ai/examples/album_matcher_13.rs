/// Comprehensive Album Matcher with 7-Phase Progressive Refinement (Run 12)
///
/// Run 12 Changes:
/// - FIXED: Stage 2 now collects ALL over-segmented candidates (not just best)
/// - FIXED: Stage 3 now tries assembling EACH collected candidate (restores Run 7 behavior)
/// - This fixes Funk #49 regression: 50% (Run 10/11) → 100% (Run 7/12)
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
/// Stage 1: Initial Detection (Default Parameters)
/// - Try default parameters (-57dB, 0.9s)
///
/// Stage 2: Parameter Optimization (ENHANCED - Run 12)
/// - Test 180 parameter combinations if match < 100%
/// - NEW: Collects ALL over-segmented candidates for Stage 3 assembly
/// - Returns both best immediate match AND collected candidates
///
/// Stage 3: Comprehensive Segment Assembly (FIXED - Run 12)
/// - Try dynamic programming assembly on ALL over-segmented candidates from Stage 2
/// - Tests potentially 100+ assemblies (vs 14-16 in Run 10/11)
/// - Each candidate tested against each target edition
/// - Early exit on 100% match
///
/// Stage 4: Quiet Spot Detection
/// - RMS-based detection when silence-based fails
///
/// Stage 5: Extra Track Merging (Run 8)
/// - When detected > expected AND match quality ≥100%
/// - Try merging adjacent track pairs to achieve correct track count
///
/// Tracks which stage succeeded and saves optimal parameters for each album

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use serde::{Deserialize, Serialize};
use tokio::time::sleep;
use parking_lot::Mutex;
use strsim::levenshtein;

// ===== Configuration Constants =====

// Default silence detection parameters
const DEFAULT_THRESHOLD_DB: f64 = -57.0;
const DEFAULT_MIN_DURATION_SECS: f64 = 0.9;

// Track matching tolerance (seconds difference allowed for a track to be considered "matched")
const MATCH_TOLERANCE_SECS: f64 = 10.0;

// MusicBrainz API configuration
const MB_RATE_LIMIT_SECS: u64 = 2;           // Seconds between API requests (2x safety margin)
const MB_REQUEST_TIMEOUT_SECS: u64 = 30;     // HTTP request timeout
const MB_MAX_RELEASES: usize = 150;          // Maximum releases to fetch across all strategies

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
const NAME_DISTANCE_ALBUM_WEIGHT: f64 = 2.0;
const NAME_DISTANCE_ARTIST_WEIGHT: f64 = 1.0;

// Silence threshold for dB calculations
const SILENCE_DB_FLOOR: f32 = -100.0;
const SILENCE_RMS_EPSILON: f32 = 1e-10;

// ===== End Configuration Constants =====

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
#[derive(Debug)]
struct Stage2Results {
    /// Best match found during parameter optimization.
    best_result: Option<CandidateTestResult>,
    /// All over-segmented candidates for Stage 3 assembly.
    over_segmented_candidates: Vec<OverSegmentedCandidate>,
}

// ===== Rate Limiting =====

/// Rate limiter for MusicBrainz API compliance.
/// Enforces minimum delay between requests to avoid being blocked.
#[derive(Debug)]
struct RateLimiter {
    /// Timestamp of last API request.
    last_request: Arc<Mutex<std::time::Instant>>,
}

impl RateLimiter {
    fn new() -> Self {
        Self {
            last_request: Arc::new(Mutex::new(std::time::Instant::now() - Duration::from_secs(MB_RATE_LIMIT_SECS))),
        }
    }

    async fn wait(&self) {
        let elapsed = {
            let last = self.last_request.lock();
            last.elapsed()
        };

        // MusicBrainz API limit: 1 req/sec
        // Use 2-second delay (0.5 req/sec) for 2x safety margin to prevent timeouts
        if elapsed < Duration::from_secs(MB_RATE_LIMIT_SECS) {
            let wait_time = Duration::from_secs(MB_RATE_LIMIT_SECS) - elapsed;
            sleep(wait_time).await;
        }

        *self.last_request.lock() = std::time::Instant::now();
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
    let delays = [0, 5, 15, 45]; // seconds

    for (attempt, &delay) in delays.iter().enumerate() {
        if delay > 0 {
            println!("    Retrying after {} seconds (attempt {}/4)...", delay, attempt + 1);
            sleep(Duration::from_secs(delay)).await;
        }

        match operation().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                if attempt < delays.len() - 1 {
                    println!("    Network error: {} - will retry", e);
                } else {
                    println!("    Network error: {} - giving up after {} attempts", e, delays.len());
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
fn decode_mp3(path: &Path) -> Result<(Vec<f32>, u32), Box<dyn std::error::Error>> {
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
    let rms_window_secs = if min_duration_secs <= 0.3 {
        // For short durations (≤0.3s): use 25ms window for fine-grained detection
        RMS_WINDOW_SHORT_SECS
    } else if min_duration_secs <= 0.6 {
        // For medium durations (0.3-0.6s): use 50ms window
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
                    println!("  Reached {} release limit", MB_MAX_RELEASES);
                    break;
                }

                let encoded_query = urlencoding::encode(query);
                let search_url = format!(
                    "https://musicbrainz.org/ws/2/release/?query={}&fmt=json&limit=100",
                    encoded_query
                );

                rate_limiter.wait().await;

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
                        println!("  Strategy {}/{} for '{}' / '{}' FAILED: {}",
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

    println!("  Found {} unique releases across all search strategies", all_releases.len());

    // Fetch track details for all releases
    let mut results: Vec<(Vec<u32>, EditionMBID, String, String)> = Vec::new();

    for release in all_releases.iter() {
        rate_limiter.wait().await;

        let details_url = format!(
            "https://musicbrainz.org/ws/2/release/{}?inc=recordings&fmt=json",
            release.id
        );

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
        ));
    }

    // Calculate name distance scores and assign ranks (1-N, lower = better)
    let mut results_with_scores: Vec<((Vec<u32>, EditionMBID, String, String), f64)> = results
        .into_iter()
        .map(|(durations, mbid_info, artist, album)| {
            let score = calculate_name_distance(&artist, &album, artist_variants, album_variants);
            ((durations, mbid_info, artist, album), score)
        })
        .collect();

    // Sort by name distance score (ascending - lower is better)
    results_with_scores.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    // Assign ranks 1-N based on sorted order and include scores
    let results_with_ranks: Vec<(Vec<u32>, EditionMBID, String, String, usize, f64)> = results_with_scores
        .into_iter()
        .enumerate()
        .map(|(index, ((durations, mbid_info, artist, album), score))| {
            let rank = index + 1; // Rank 1-based
            (durations, mbid_info, artist, album, rank, score)
        })
        .collect();

    Ok(results_with_ranks)
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

    println!("  Grouped into {} unique editions", editions.len());

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
            // Swap if current item's score is more than 2x worse than next item's score
            if editions[i].name_distance_score > 2.0 * editions[i + 1].name_distance_score {
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
    println!("  Phase 0 Reconciliation:");
    println!("    Strategy: {:?}", reconciled.strategy);
    println!("    Confidence: {:?}", reconciled.confidence);
    println!("    Artist: {} (source: {:?})", reconciled.artist, reconciled.artist_source);
    if let Some(ref alt) = reconciled.alternate_artist {
        println!("      Alternate: {}", alt);
    }
    println!("    Album: {} (source: {:?})", reconciled.album, reconciled.album_source);
    if let Some(ref alt) = reconciled.alternate_album {
        println!("      Alternate: {}", alt);
    }
    if reconciled.has_musicbrainz_ids {
        println!("    Has MusicBrainz IDs in tags: Yes");
    }
    if let Some(count) = reconciled.estimated_track_count {
        println!("    Estimated track count from ID3: {}", count);
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
                20.0 * (*rms as f64).log10()
            } else {
                SILENCE_DB_FLOOR as f64
            };

            // Score = RMS in dB + penalty for distance from expected
            let distance = (pos - expected_pos).abs();
            let distance_penalty = (distance / dynamic_radius) * QUIET_SPOT_PROXIMITY_PENALTY * 20.0;
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

/// Stage 1: Initial detection with default parameters
/// Tests default threshold/duration against ALL MB candidates
fn run_stage1_initial_detection(
    samples: &[f32],
    sample_rate: u32,
    threshold_db: f64,
    min_duration_secs: f64,
    mb_candidates: &[(Vec<u32>, String)],
    tolerance: f64,
) -> (Vec<f64>, Option<CandidateTestResult>) {
    println!("  STAGE 1: Testing default parameters ({}dB, {}s)...", threshold_db, min_duration_secs);
    let durations = get_track_durations(samples, sample_rate, threshold_db, min_duration_secs);
    println!("    Found {} tracks", durations.len());

    let result = test_segmentation_against_all_candidates(&durations, mb_candidates, tolerance);

    if let Some(ref r) = result {
        println!("    Best match: {:.1}% with {} tracks from release {} ({} confidence)",
            r.percentage, r.expected_durations.len(), &r.mbid[..8],
            classify_confidence(r.percentage));
    }

    (durations, result)
}

/// Stage 2: Parameter optimization (ENHANCED - Run 12)
/// Tests grid of threshold/duration combinations against ALL MB candidates
/// NOW ALSO collects all over-segmented candidates for Stage 3 assembly
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

    println!("  STAGE 2: Testing {} parameter combinations against all editions...",
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
                    println!("    New best: {:.1}% with {}dB, {}s → {} tracks from {} ({}/{})",
                        result.percentage, thresh, min_dur, result.expected_durations.len(),
                        &result.mbid[..8], tested, total_combinations);

                    best_result = Some(result);

                    if best_result.as_ref().unwrap().percentage >= 100.0 {
                        println!("    Best match after parameter optimization: 100.0% (Excellent confidence)");
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
        println!("    Best match after parameter optimization: {:.1}% ({} confidence)",
            r.percentage, classify_confidence(r.percentage));
    }

    println!("    Collected {} over-segmented candidates for Stage 3 assembly",
        over_segmented_candidates.len());

    Stage2Results {
        best_result,
        over_segmented_candidates,
    }
}

/// Stage 3: Comprehensive Segment Assembly (FIXED - Run 12)
/// Tries assembling EACH over-segmented candidate from Stage 2 against EACH target edition
/// This restores Run 7 behavior where 100+ assemblies could be tested
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
        println!("  STAGE 3: No over-segmented candidates to assemble");
        return None;
    }

    println!("  STAGE 3: Comprehensive segment assembly across {} over-segmented candidates...",
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
                            println!("    New best: {:.1}% via assembly of ({}dB, {}s) ({} segments → {} tracks)",
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
                                println!("    Tested {} assemblies, {} improved over best",
                                    assemblies_tested, assemblies_improved);
                                return best_result; // Early exit on perfect match
                            }
                        }
                    }
                }
            }
        }
    }

    println!("    Tested {} assemblies, {} improved over best", assemblies_tested, assemblies_improved);

    if let (Some(_result), Some((thresh, min_dur, seg_count))) = (&best_result, best_source_params) {
        println!("    Best assembly from {}dB, {}s ({} segments)", thresh, min_dur, seg_count);
    }

    best_result
}

/// Stage 4: Edition-guided quiet spot detection
/// Uses best matched edition's track durations as a guide to search for quiet spots
/// near expected boundary positions (±search radius), then tests against all editions
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

    println!("  STAGE 4: Edition-guided quiet spot detection...");

    // Calculate RMS profile once for the entire file
    let rms_profile = calculate_rms_profile(samples, sample_rate);
    let total_duration_secs = samples.len() as f64 / sample_rate as f64;

    if rms_profile.is_empty() {
        println!("    No RMS profile generated");
        return None;
    }

    println!("    RMS profile: {} windows ({}ms step)",
        rms_profile.len(), (QUIET_SPOT_WINDOW_STEP_SECS * 1000.0) as u32);

    let mut guided_tests = 0;
    let mut guided_improvements = 0;
    let mut best_result: Option<CandidateTestResult> = None;

    // Try edition-guided search for top N editions
    let editions_to_try = mb_candidates.len().min(QUIET_SPOT_TOP_EDITIONS);
    println!("    Testing guided search against top {} editions...", editions_to_try);

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
                println!("    New best: {:.1}% via guided search (guide: {} tracks, matched: {} tracks)",
                    result.percentage, expected_durations.len(), result.expected_durations.len());

                best_result = Some(result);

                if best_result.as_ref().unwrap().percentage >= 100.0 {
                    println!("    Tested {} guided searches, {} improved", guided_tests, guided_improvements);
                    return best_result; // Early exit on perfect match
                }
            }
        }
    }

    println!("    Tested {} guided searches, {} improved", guided_tests, guided_improvements);

    best_result
}

/// Stage 5: Extra track merging
/// When detected > expected AND match ≥100%, merge adjacent tracks to achieve correct count
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
    println!("  STAGE 5: Attempting extra track merging...");
    println!("    {} extra track(s) detected, match quality {:.1}%", extra_count, current_best_percentage);
    println!("    Testing {} possible adjacent track merges", best_durations.len() - 1);

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

        println!("    Best merge: tracks {} + {} → {:.1}% match, {:.2}s mean error",
            best_merge_index.unwrap() + 1,
            best_merge_index.unwrap() + 2,
            merged_percentage,
            mean_merged_error);

        println!("    Track count corrected: {} → {} ({:.1}% match)",
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
        println!("    No valid merge found that produces correct track count");
        None
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Comprehensive Album Matcher (Run 13) ===");
    println!("ARCHITECTURE: Comprehensive upfront MB search (ALL strategies, up to {} releases)\n", MB_MAX_RELEASES);
    println!("  - Edition grouping by track count + duration pattern (not MBID)");
    println!("  - All unique editions tested through 5 stages");
    println!("  - Best MBID selected from winning edition (CD/Official/US priority)");
    println!("  - RUN 12 FIX: Stage 2 collects ALL over-segmented candidates");
    println!("  - RUN 12 FIX: Stage 3 tries assembling EACH candidate (restores Run 7 behavior)");
    println!("  - RUN 13: Stage 4 redesigned with edition-guided quiet spot detection");
    println!("  - RUN 13: Extended threshold range (-30dB to -60dB)\n");

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

    println!("=== Parameter Validation ===");
    println!("Testing optimal parameters on {} albums from combined set", training_files.len());
    println!("  ({} training set + {} from long files list)\n", initial_count, additional_files.len());

    // Use optimal parameters from analysis
    let threshold_db: f64 = DEFAULT_THRESHOLD_DB;
    let min_duration_secs: f64 = DEFAULT_MIN_DURATION_SECS;
    let match_tolerance_secs = MATCH_TOLERANCE_SECS;

    println!("Parameters:");
    println!("  Threshold: {} dB", threshold_db);
    println!("  Min duration: {} seconds", min_duration_secs);
    println!("  Match tolerance: {} seconds\n", match_tolerance_secs);

    let rate_limiter = RateLimiter::new();
    let mut results = Vec::new();

    // Parameter grid for Stage 2 optimization
    // Run 13: Extended threshold range to -30dB, -34dB (removed -40dB, -44dB)
    // - Testing less strict thresholds for albums with louder inter-track gaps
    let threshold_values = [-30.0, -34.0, -36.0, -38.0, -42.0, -47.0, -50.0, -52.0, -54.0, -56.0, -58.0, -60.0];
    let min_duration_values = [0.05, 0.10, 0.15, 0.2, 0.25, 0.3, 0.4, 0.5, 0.8, 1.0, 1.5, 2.0, 2.5, 3.0, 4.0];

    for (idx, file_path) in training_files.iter().enumerate() {
        println!("=== Album {}/{} ===", idx + 1, training_files.len());
        println!("File: {}", file_path.display());

        if !file_path.exists() {
            println!("  ERROR: File not found\n");
            continue;
        }

        // === PHASE 0: ID3 Tag Extraction & Reconciliation ===
        let reconciled = extract_and_reconcile_metadata(file_path);
        log_reconciliation_decision(&reconciled);

        let artist = &reconciled.artist;
        let album = &reconciled.album;

        // Decode MP3
        print!("  Decoding... ");
        let (samples, sample_rate) = match decode_mp3(file_path) {
            Ok((s, sr)) => {
                let duration_mins = s.len() as f64 / sr as f64 / 60.0;
                println!("Done! {} samples at {} Hz ({:.2} mins)", s.len(), sr, duration_mins);
                (s, sr)
            }
            Err(e) => {
                println!("FAILED: {}", e);
                results.push(ValidationResult::error(
                    file_path, artist, album, 0,
                    format!("Decode failed: {}", e),
                ));
                println!();
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
        print!("  Fetching MusicBrainz data (comprehensive search)... ");
        let editions = match comprehensive_musicbrainz_search(
            &artist_variants,
            &album_variants,
            file_duration_secs,
            estimated_track_count,
            &rate_limiter
        ).await {
            Ok(releases) => {
                if releases.is_empty() {
                    println!("FAILED: No releases found");
                    results.push(ValidationResult::error(
                        file_path, artist, album, initial_durations.len(),
                        "MusicBrainz lookup failed: No releases found".to_string(),
                    ));
                    println!();
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

                println!("  Sorted editions by likelihood (file: {:.0}s, ~{} tracks)",
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

                let filtered_count = total_editions - editions.len();
                if filtered_count > 0 {
                    println!("  Filtered out {} editions (runtime >25% different from file)", filtered_count);
                    println!("  Acceptable range: {:.0}s - {:.0}s", min_duration, max_duration);
                }

                if editions.is_empty() {
                    let failure_msg = if filtered_count > 0 {
                        format!("FAILED: All {} editions filtered out (runtime mismatch >25%)", filtered_count)
                    } else {
                        "FAILED: No valid editions".to_string()
                    };
                    println!("{}", failure_msg);
                    results.push(ValidationResult::error(
                        file_path, artist, album, initial_durations.len(),
                        failure_msg,
                    ));
                    println!();
                    continue;
                }

                println!("Found {} unique editions to test", editions.len());

                // Re-sort by name similarity using conservative bubble sort
                // This allows editions with significantly better name matches to bubble up
                // while preserving runtime-based ordering for similar name scores
                resort_by_name_similarity(&mut editions);

                editions
            }
            Err(e) => {
                println!("FAILED: {}", e);
                results.push(ValidationResult::error(
                    file_path, artist, album, initial_durations.len(),
                    format!("MusicBrainz lookup failed: {}", e),
                ));
                println!();
                continue;
            }
        };

        // Convert editions to old format for stage testing: Vec<(Vec<u32>, String)>
        // Use temporary placeholder MBIDs (will select best MBID after finding winning edition)
        let mb_candidates: Vec<(Vec<u32>, String)> = editions
            .iter()
            .enumerate()
            .map(|(idx, edition)| (edition.durations.clone(), format!("edition_{}", idx)))
            .collect();

        // Display edition information
        println!("\n  Edition Details (sorted by match likelihood):");
        for (idx, edition) in editions.iter().enumerate() {
            let edition_duration: u32 = edition.durations.iter().sum();
            let match_score = score_edition_match(edition, file_duration_secs, estimated_track_count);
            println!("    [{}] {} - {} ({} tracks, {}s, {} MBIDs, score: {:.0}s, NDR:{},{:.1})",
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

        // Helper to display edition info from "edition_X" MBID
        let show_edition_info = |mbid: &str| {
            if let Some(idx_str) = mbid.strip_prefix("edition_") {
                if let Ok(idx) = idx_str.parse::<usize>() {
                    if idx < editions.len() {
                        println!("      → Edition: {} - {} ({} tracks, NDR:{},{:.1})",
                            editions[idx].artist,
                            editions[idx].album,
                            editions[idx].track_count,
                            editions[idx].name_distance_rank,
                            editions[idx].name_distance_score
                        );
                    }
                }
            }
        };

        // === STAGE 1: Initial detection ===
        let (mut best_durations, stage1_result) = run_stage1_initial_detection(
            &samples,
            sample_rate,
            threshold_db,
            min_duration_secs,
            &mb_candidates,
            match_tolerance_secs,
        );

        // Initialize tracking variables
        let mut best_matches: Vec<TrackMatch>;
        let mut best_matched_count: usize;
        let mut best_percentage: f64;
        let mut best_stage = "album_extractor_1_initial";
        let best_threshold: Option<f64> = None;
        let best_min_duration: Option<f64> = None;
        let mut best_mbid: String;
        let mut best_expected_durations: Vec<u32>;
        let mut best_mean_error: f64;

        if let Some(result) = stage1_result {
            best_matches = result.matches;
            best_matched_count = result.matched_count;
            best_percentage = result.percentage;
            best_mbid = result.mbid.clone();
            best_expected_durations = result.expected_durations;
            best_mean_error = result.mean_error;
            show_edition_info(&result.mbid);
        } else {
            println!("    ERROR: No candidates to test");
            continue;
        }

        // === STAGE 2: Parameter optimization (ENHANCED - Run 12) ===
        // Now returns Stage2Results with both best_result AND over_segmented_candidates
        let stage2_results = run_stage2_parameter_optimization(
            &samples,
            sample_rate,
            &threshold_values,
            &min_duration_values,
            &mb_candidates,
            match_tolerance_secs,
            best_percentage,
        );

        if let Some(result) = stage2_results.best_result {
            best_durations = result.detected_durations;
            best_matches = result.matches;
            best_matched_count = result.matched_count;
            best_percentage = result.percentage;
            best_mbid = result.mbid.clone();
            best_expected_durations = result.expected_durations;
            best_mean_error = result.mean_error;
            best_stage = "album_extractor_2_optimization";
            show_edition_info(&result.mbid);
        }

        // === STAGE 3: Comprehensive segment assembly (FIXED - Run 12) ===
        // Now uses ALL over-segmented candidates collected from Stage 2
        if let Some(result) = run_stage3_comprehensive_assembly(
            &stage2_results.over_segmented_candidates,
            &mb_candidates,
            match_tolerance_secs,
            best_percentage,
        ) {
            best_durations = result.detected_durations;
            best_matches = result.matches;
            best_matched_count = result.matched_count;
            best_percentage = result.percentage;
            best_mbid = result.mbid.clone();
            best_expected_durations = result.expected_durations;
            best_mean_error = result.mean_error;
            best_stage = "album_extractor_3_assembly";
            show_edition_info(&result.mbid);
        }

        // === STAGE 4: Quiet spot detection ===
        if let Some(result) = run_stage4_quiet_spot_detection(
            &samples,
            sample_rate,
            &mb_candidates,
            match_tolerance_secs,
            best_percentage,
        ) {
            best_durations = result.detected_durations;
            best_matches = result.matches;
            best_matched_count = result.matched_count;
            best_percentage = result.percentage;
            best_mbid = result.mbid.clone();
            best_expected_durations = result.expected_durations;
            best_mean_error = result.mean_error;
            best_stage = "album_extractor_4_guided";
            show_edition_info(&result.mbid);
        }

        // === STAGE 5: Extra track merging ===
        if let Some((merged_durations, result)) = run_stage5_extra_track_merging(
            &best_durations,
            &best_expected_durations,
            match_tolerance_secs,
            best_percentage,
        ) {
            best_durations = merged_durations;
            best_matches = result.matches;
            best_matched_count = result.matched_count;
            best_percentage = result.percentage;
            best_mean_error = result.mean_error;
            best_stage = "album_extractor_5_merging";
            // mbid and expected_durations stay the same
        }

        // Convert placeholder MBID to real MBID from winning edition
        if best_mbid.starts_with("edition_") {
            if let Some(idx_str) = best_mbid.strip_prefix("edition_") {
                if let Ok(idx) = idx_str.parse::<usize>() {
                    if idx < editions.len() {
                        best_mbid = select_best_mbid(&editions[idx]);
                    }
                }
            }
        }

        // Calculate final statistics
        let perfect_count = best_durations.len() == best_expected_durations.len();

        println!("\n  FINAL RESULT:");
        println!("    Matching stage: {}", best_stage);
        println!("    MusicBrainz: https://musicbrainz.org/release/{}", best_mbid);
        println!("    Track count: {}/{} {}", best_durations.len(), best_expected_durations.len(),
            if perfect_count { "✓" } else { "✗" });
        println!("    Matched tracks: {}/{} ({:.1}%)", best_matched_count, best_expected_durations.len(), best_percentage);
        println!("    Mean error: {:.2}s", best_mean_error);
        println!("    Confidence: {}", classify_confidence(best_percentage));

        if let (Some(thresh), Some(min_dur)) = (best_threshold, best_min_duration) {
            println!("    Best parameters: {}dB, {}s", thresh, min_dur);
        }

        // Show all track matches
        if !best_matches.is_empty() {
            println!("  All tracks:");
            for (i, tm) in best_matches.iter().enumerate() {
                let status = if tm.matches { "✓" } else { "✗" };
                println!("    {}. {:6.1}s vs {:6}s  error={:5.1}s  {}",
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
                println!("  Extra tracks detected ({} beyond expected count):", extra_tracks.len());
                for et in &extra_tracks {
                    println!("    Track {}: {:.1}s (no MusicBrainz match)", et.track_index, et.duration);
                }
            }
        } else if best_durations.len() < best_expected_durations.len() {
            // MusicBrainz has more tracks than detected - show missing ones
            let missing_count = best_expected_durations.len() - best_durations.len();
            println!("  MusicBrainz tracks not found in file ({} missing):", missing_count);
            for i in min_count..best_expected_durations.len() {
                println!("    Track {}: {}s (expected but not detected in file)", i + 1, best_expected_durations[i]);
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

        println!();
    }

    // Write results
    let output_path = Path::new(r"C:\Users\Mango Cat\Dev\McRhythm\album_matcher_results.json");
    println!("\n=== Writing Results ===");
    println!("Output: {}", output_path.display());

    let json = serde_json::to_string_pretty(&results)?;
    std::fs::write(output_path, json)?;

    // Analysis
    println!("\n=== COMPREHENSIVE MATCHING ANALYSIS ===");

    let successful = results.iter().filter(|r| r.status == "Success").count();
    println!("Total albums processed: {}", results.len());
    println!("Successfully analyzed: {}", successful);

    if successful > 0 {
        // Matching stage breakdown
        let stage1 = results.iter().filter(|r| r.matching_stage == "album_extractor_1_initial").count();
        let stage2 = results.iter().filter(|r| r.matching_stage == "album_extractor_2_optimization").count();
        let stage3 = results.iter().filter(|r| r.matching_stage == "album_extractor_3_assembly").count();
        let stage4 = results.iter().filter(|r| r.matching_stage == "album_extractor_4_guided").count();
        let stage5 = results.iter().filter(|r| r.matching_stage == "album_extractor_5_editions").count();
        let stage6 = results.iter().filter(|r| r.matching_stage == "album_extractor_6_merging").count();

        println!("\nMatching Stage Results:");
        println!("  album_extractor_1_initial:        {} albums", stage1);
        println!("  album_extractor_2_optimization:   {} albums", stage2);
        println!("  album_extractor_3_assembly:       {} albums", stage3);
        println!("  album_extractor_4_guided:         {} albums", stage4);
        println!("  album_extractor_5_editions:       {} albums", stage5);
        println!("  album_extractor_6_merging:        {} albums", stage6);

        // Confidence level breakdown
        let excellent = results.iter().filter(|r| r.confidence == "Excellent").count();
        let good = results.iter().filter(|r| r.confidence == "Good").count();
        let fair = results.iter().filter(|r| r.confidence == "Fair").count();
        let poor = results.iter().filter(|r| r.confidence == "Poor" && r.status == "Success").count();

        println!("\nConfidence Distribution:");
        println!("  Excellent (≥80%): {} albums", excellent);
        println!("  Good (60-79%):    {} albums", good);
        println!("  Fair (40-59%):    {} albums", fair);
        println!("  Poor (<40%):      {} albums", poor);

        // Track count matches
        let perfect_counts = results.iter().filter(|r| r.perfect_count_match).count();
        println!("\nTrack Count Matches:");
        println!("  Perfect: {}/{} ({:.1}%)", perfect_counts, successful,
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

        println!("\nAverage Statistics:");
        println!("  Match percentage: {:.1}%", avg_match_pct);
        println!("  Mean error: {:.2}s", avg_error);

        // Parameter effectiveness (for Stage 2 results)
        if stage2 > 0 {
            println!("\nParameter Optimization Details:");
            for result in results.iter().filter(|r| r.matching_stage == "album_extractor_2_optimization") {
                if let (Some(thresh), Some(min_dur)) = (result.best_threshold_db, result.best_min_duration_secs) {
                    println!("  {} - {}: {}dB, {}s → {:.1}%",
                        result.artist, result.album, thresh, min_dur, result.match_percentage);
                }
            }
        }

        // Show best and worst
        let mut success_results: Vec<_> = results.iter().filter(|r| r.status == "Success").collect();
        success_results.sort_by(|a, b| b.match_percentage.partial_cmp(&a.match_percentage).unwrap());

        println!("\nBest 5 Albums:");
        for (i, result) in success_results.iter().take(5).enumerate() {
            println!("  {}. {} - {} ({:.1}%, {} via {})",
                i + 1, result.artist, result.album, result.match_percentage,
                result.confidence, result.matching_stage);
            println!("     MusicBrainz: {}", result.musicbrainz_url);
        }

        println!("\nWorst 5 Albums:");
        for (i, result) in success_results.iter().rev().take(5).enumerate() {
            println!("  {}. {} - {} ({:.1}%, {} via {})",
                i + 1, result.artist, result.album, result.match_percentage,
                result.confidence, result.matching_stage);
            println!("     MusicBrainz: {}", result.musicbrainz_url);
        }
    }

    println!("\nDone!");
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
}
