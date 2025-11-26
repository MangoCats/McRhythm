//! # Configuration Constants
//!
//! Global constants including STAGE2 parameter arrays and tolerance values.
//!
//! ## CRITICAL: Parameter Array Ordering
//!
//! STAGE2_THRESHOLD_VALUES and STAGE2_MIN_DURATION_VALUES are ordered by
//! empirical frequency from Run 27 analysis (193 successful albums).
//! This ordering is CRITICAL for early-exit optimization performance.
//!
//! **DO NOT reorder these arrays** - order affects early-exit behavior.

// =============================================================================
// Stage 2 Parameter Grid (Run 28: Empirically Reordered)
// =============================================================================

/// Stage 2 Parameter Grid: Threshold values (dB) for silence detection sweep
///
/// Run 28: REORDERED BY EMPIRICAL FREQUENCY from Run 27 analysis (193 successful albums)
/// Testing order optimized for early-exit when 100% match found
///
/// Frequency by threshold (in top 20 combinations):
///   -50dB: appears in 7 of top 20 combos (104 albums, 53.9%)
///   -58dB: appears in 6 of top 20 combos (30 albums, 15.5%)
///   -60dB: appears in 3 of top 20 combos (10 albums, 5.2%)
///   -62dB: appears in 2 of top 20 combos (6 albums, 3.1%)
///   Others: rare or not in top 20
///
/// **CRITICAL:** This specific ordering is required for early-exit optimization.
/// Reordering will break performance characteristics (TEST-FUNC-002a).
pub(crate) const STAGE2_THRESHOLD_VALUES: [f64; 12] = [
    -50.0,  // Rank 1: Most common (53.9% of successful albums)
    -58.0,  // Rank 2: Second most common (15.5%)
    -60.0,  // Rank 3: Third (5.2%)
    -62.0,  // Rank 4: Fourth (3.1%)
    -56.0,  // Rank 5: Appears in top 20 (rank 16)
    -52.0,  // Rank 6: Appears in top 20 (rank 20)
    -54.0,  // Rank 7: Not in top 20, but was DEFAULT in Run 27
    -48.0,  // Rank 8: Not in top 20 (rare)
    -40.0,  // Rank 9: Not in top 20 (rare)
    -38.0,  // Rank 10: Not in top 20 (rare)
    -34.0,  // Rank 11: Not in top 20 (rare)
    -30.0   // Rank 12: Not in top 20 (rare)
];

/// Stage 2 Parameter Grid: Min duration values (seconds) for silence detection sweep
///
/// Run 28: REORDERED BY EMPIRICAL FREQUENCY from Run 27 analysis (193 successful albums)
/// Testing order optimized for early-exit when 100% match found
///
/// Frequency by min_duration (in top 20 combinations):
///   3.0s: appears in ranks 1,6 (46 albums, 23.8%)
///   2.0s: appears in ranks 2,4,12,14 (49 albums, 25.4%)
///   0.5s: appears in rank 3 (17 albums, 8.8%)
///   1.5s: appears in ranks 5,8 (10 albums, 5.2%)
///   0.3s: appears in ranks 7,10,16,17 (11 albums, 5.7%)
///   0.8s: appears in ranks 11,19,20 (7 albums, 3.6%)
///   0.1s: appears in ranks 9,18 (6 albums, 3.1%)
///   Others: rare or not in top 20
///
/// **CRITICAL:** This specific ordering is required for early-exit optimization.
/// Reordering will break performance characteristics (TEST-FUNC-002b).
pub(crate) const STAGE2_MIN_DURATION_VALUES: [f64; 15] = [
    3.0,   // Rank 1: Very common, especially with -50dB (rank 1: 41 albums)
    2.0,   // Rank 2: Very common (ranks 2,4,12,14: 49 albums total)
    0.5,   // Rank 3: Common with -50dB (rank 3: 17 albums)
    1.5,   // Rank 4: Moderate (ranks 5,8: 10 albums)
    0.3,   // Rank 5: Moderate (ranks 7,10,16,17: 11 albums)
    0.8,   // Rank 6: Moderate (ranks 11,19,20: 7 albums)
    0.1,   // Rank 7: Moderate (ranks 9,18: 6 albums) - Note: same as 0.10
    4.0,   // Rank 8: Low frequency (rank 15: 3 albums)
    0.2,   // Rank 9: Low frequency (rank 13: 3 albums)
    2.5,   // Rank 10: Not in top 20 (rare)
    1.0,   // Rank 11: Not in top 20 (rare)
    0.4,   // Rank 12: Not in top 20 (rare)
    5.0,   // Rank 13: Not in top 20 (rare)
    3.5,   // Rank 14: Not in top 20 (rare)
    0.05   // Rank 15: Not in top 20 (very rare)
];

/// Default silence detection threshold (dB) - most common parameter
pub(crate) const DEFAULT_THRESHOLD_DB: f64 = -50.0;

/// Default minimum silence duration (seconds) - most common parameter
pub(crate) const DEFAULT_MIN_DURATION_SECS: f64 = 3.0;

// =============================================================================
// Silence Detection Constants
// =============================================================================

/// RMS window sizing for silence detection (adaptive based on min_duration)
/// For very short silences (≤0.3s)
pub(crate) const RMS_WINDOW_SHORT_SECS: f64 = 0.025;

/// RMS window for medium silences (0.3-0.6s)
pub(crate) const RMS_WINDOW_MEDIUM_SECS: f64 = 0.05;

/// RMS window for longer silences (>0.6s)
pub(crate) const RMS_WINDOW_STANDARD_SECS: f64 = 0.1;

/// 50% overlap between RMS windows
pub(crate) const RMS_WINDOW_OVERLAP: f64 = 0.5;

/// Use short window for ≤0.3s min duration
pub(crate) const RMS_WINDOW_THRESHOLD_SHORT: f64 = 0.3;

/// Use medium window for ≤0.6s min duration
pub(crate) const RMS_WINDOW_THRESHOLD_MEDIUM: f64 = 0.6;

/// Minimum dB floor for silence calculations
pub(crate) const SILENCE_DB_FLOOR: f32 = -100.0;

/// RMS epsilon threshold (values below this are treated as silence)
pub(crate) const SILENCE_RMS_EPSILON: f32 = 1e-10;

// =============================================================================
// Match Tolerance and Grace Periods
// =============================================================================

/// Tolerance for track duration matching (seconds)
/// Tracks within ±MATCH_TOLERANCE_SECS of expected duration are considered matches
pub(crate) const MATCH_TOLERANCE_SECS: f64 = 3.0;

/// Grace period for short tracks (seconds)
/// Additional tolerance allowed for tracks shorter than GRACE_PERIOD_THRESHOLD_SECS
pub(crate) const GRACE_PERIOD_SECS: f64 = 2.0;

/// Threshold for applying grace period (seconds)
/// Tracks < GRACE_PERIOD_THRESHOLD_SECS get additional tolerance
pub(crate) const GRACE_PERIOD_THRESHOLD_SECS: u32 = 180;

// =============================================================================
// Edition Selection and Penalties
// =============================================================================

/// Penalty per extra track when selecting winning edition
/// Lower track count editions preferred (penalty applied to percentage)
pub(crate) const TRACK_COUNT_PENALTY_PER_EXTRA: f64 = 2.0;

/// Maximum number of editions to test per album
/// Limit prevents excessive processing for albums with many MB matches
pub(crate) const MAX_EDITIONS_TO_TEST: usize = 10;

// =============================================================================
// Sentinel Values
// =============================================================================

/// Sentinel value for missing/unknown metadata
pub(crate) const UNKNOWN_VALUE: &str = "Unknown";

/// Sentinel value for Various Artists compilations
pub(crate) const VARIOUS_ARTISTS: &str = "Various Artists";

// =============================================================================
// Single-Track Discriminator Constants
// =============================================================================

/// Filename pattern for detecting track number prefixes
/// Matches: "04 - Song.mp3", "12_Track.mp3", "8. Title.mp3"
pub(crate) const SINGLE_TRACK_FILENAME_PATTERN: &str = r"^(\d{1,2})\s*[-_\.]\s*";

/// Directory file count threshold for single-track detection
/// >= 4 files in directory = likely individual tracks
pub(crate) const SINGLE_TRACK_DIR_FILE_THRESHOLD: usize = 4;

/// ID3 track total threshold for single-track detection
/// If total > 1, definitely individual track file
pub(crate) const SINGLE_TRACK_ID3_TOTAL_THRESHOLD: u32 = 1;

/// Minimum expected album duration (minutes)
/// Albums typically > 20 minutes
pub(crate) const SINGLE_TRACK_MIN_ALBUM_DURATION_MINS: f64 = 20.0;

/// Typical single track duration (minutes)
/// Individual tracks typically < 8 minutes
pub(crate) const SINGLE_TRACK_TYPICAL_DURATION_MINS: f64 = 8.0;

/// Minimum expected silence gaps in album
/// Albums typically have >= 4 tracks (>= 3 gaps)
pub(crate) const SINGLE_TRACK_MIN_EXPECTED_GAPS: usize = 3;

/// Single-track score threshold
/// Score >= 1.5 = likely individual track file
pub(crate) const SINGLE_TRACK_SCORE_THRESHOLD: f64 = 1.5;

// Individual layer scores for single-track discriminator
pub(crate) const SCORE_FILENAME_PATTERN: f64 = 1.0;
pub(crate) const SCORE_DIR_FILES_HIGH: f64 = 0.8;
pub(crate) const SCORE_DIR_FILES_MEDIUM: f64 = 0.3;
pub(crate) const SCORE_DIR_FILES_SINGLE: f64 = -0.5;
pub(crate) const SCORE_ID3_TRACK_TOTAL: f64 = 1.0;
pub(crate) const SCORE_ID3_TRACK_NUMBER_ONLY: f64 = 0.4;
pub(crate) const SCORE_DURATION_SHORT: f64 = 0.7;
pub(crate) const SCORE_DURATION_SUSPICIOUS: f64 = 0.4;
pub(crate) const SCORE_DURATION_ALBUM_LENGTH: f64 = -0.3;
pub(crate) const SCORE_SILENCE_GAPS_FEW: f64 = 1.0;
pub(crate) const SCORE_SILENCE_GAPS_MANY: f64 = -0.5;

/// Audio file extensions for directory scanning
pub(crate) const AUDIO_EXTENSIONS: &[&str] = &["mp3", "flac", "m4a", "ogg", "wav", "aac", "wma", "opus"];

// =============================================================================
// MusicBrainz API Rate Limiting and Retry Constants
// =============================================================================

/// MusicBrainz API rate limit (milliseconds between requests)
/// Uses 1.55s delay for safety margin to prevent 503 errors (official limit: 1 req/sec)
pub(crate) const MB_RATE_LIMIT_MS: u64 = 1550;

/// Exponential backoff delays for retry attempts (seconds)
/// Attempts: immediate, +5s, +15s, +45s, +60s (then gives up)
pub(crate) const MB_RETRY_DELAYS_SECS: &[u64] = &[0, 5, 15, 45, 60];

/// HTTP request timeout for MusicBrainz API calls (seconds)
pub(crate) const MB_REQUEST_TIMEOUT_SECS: u64 = 30;

// =============================================================================
// Album Processing Timing Constants
// =============================================================================

/// Heartbeat logging interval (seconds)
/// QueryStats logs progress every 120 seconds while processing
pub(crate) const HEARTBEAT_INTERVAL_SECS: u64 = 120;

/// Stagger multiplier for concurrent album processing
/// Each album waits (position × STAGGER_MULTIPLIER × MB_RATE_LIMIT_MS) before starting
/// Example: Album 1 waits 0s, Album 2 waits 46.5s, Album 3 waits 93s
pub(crate) const STAGGER_MULTIPLIER: u64 = 30;

/// Maximum number of albums to process concurrently
/// Used for both stagger calculation and tokio buffer_unordered limit
pub(crate) const MAX_CONCURRENT_ALBUMS: usize = 6;

// =============================================================================
// Stage 4: Quiet Spot Detection Constants
// =============================================================================

/// RMS window size for quiet spot detection (seconds)
pub(crate) const QUIET_SPOT_WINDOW_SECS: f64 = 0.5;

/// RMS window step size (50% overlap)
pub(crate) const QUIET_SPOT_WINDOW_STEP_SECS: f64 = 0.25;

/// Search radius as percentage of expected track duration
pub(crate) const QUIET_SPOT_SEARCH_RADIUS_RATIO: f64 = 0.15;

/// Minimum search radius (seconds)
pub(crate) const QUIET_SPOT_SEARCH_RADIUS_MIN: f64 = 5.0;

/// Maximum search radius (seconds)
pub(crate) const QUIET_SPOT_SEARCH_RADIUS_MAX: f64 = 20.0;

/// Penalty factor for distance from expected boundary position
pub(crate) const QUIET_SPOT_PROXIMITY_PENALTY: f64 = 0.5;

/// Number of top editions to test with guided quiet spot detection
pub(crate) const QUIET_SPOT_TOP_EDITIONS: usize = 5;

/// Distance penalty multiplier for quiet spot scoring
pub(crate) const QUIET_SPOT_DISTANCE_PENALTY_MULTIPLIER: f64 = 20.0;

/// Decibel multiplier for RMS-to-dB conversion (20 for amplitude, 10 for power)
pub(crate) const DB_MULTIPLIER: f64 = 20.0;

/// Match percentage penalty for Stage 4 results (quiet spot detection)
/// Stage 4 raw results are multiplied by 0.75 (25% penalty) due to lower reliability
pub(crate) const STAGE4_PENALTY_PERCENT: f64 = 25.0;

// =============================================================================
// Edition Filtering Constants
// =============================================================================

/// Minimum runtime ratio for edition filtering (75%)
pub(crate) const RUNTIME_FILTER_MIN_RATIO: f64 = 0.75;

/// Maximum runtime ratio for edition filtering (125%)
pub(crate) const RUNTIME_FILTER_MAX_RATIO: f64 = 1.25;

// =============================================================================
// Early-Exit Grace Period Constants
// =============================================================================

/// Grace period after finding 100% match before allowing early-exit (seconds)
/// Allows slower-processing editions to complete and potentially find better matches
pub(crate) const EARLY_EXIT_GRACE_PERIOD_SECS: u64 = 20;

// =============================================================================
// Edition Scoring and Selection Constants
// =============================================================================

/// Seconds penalty per track count difference when scoring edition matches
/// Each track difference = 60 seconds of runtime error (edition scoring)
pub(crate) const SCORE_TRACK_COUNT_PENALTY: f64 = 60.0;

/// Swap ratio threshold for name similarity bubble sort
/// Editions with name_distance_score > 1.732× worse than next are swapped
pub(crate) const NAME_DISTANCE_SWAP_RATIO: f64 = 1.732;

/// Match percentage penalty per extra track beyond expected count
/// Example: 5 extra tracks = 20% penalty (5 × 4.0)
pub(crate) const TRACK_COUNT_PENALTY_PER_TRACK: f64 = 4.0;

/// Minimum combined similarity for runner-up to qualify
/// Runner must have (artist_sim + album_sim) / 2 >= 0.60
pub(crate) const ARTIST_FALLBACK_MIN_SIMILARITY: f64 = 0.60;

/// Absolute delta threshold for "significantly better" name match
/// Runner combined_sim must exceed winner by at least 0.20
pub(crate) const ARTIST_FALLBACK_DELTA: f64 = 0.20;

/// Ratio threshold for "significantly better" name match
/// Runner combined_sim must exceed winner by factor of 1.4
pub(crate) const ARTIST_FALLBACK_RATIO: f64 = 1.4;

/// Time-fit delta threshold for accepting runner-up
/// Runner must have time_fit_delta >= -0.5 to not be "significantly worse"
pub(crate) const TIME_FIT_DELTA_THRESHOLD: f64 = -0.5;

/// Percentage of editions to evaluate as runner-ups (top 25%)
pub(crate) const ARTIST_FALLBACK_TOP_PCT: f64 = 0.25;

/// Minimum number of runner-ups to evaluate (even if <25%)
pub(crate) const ARTIST_FALLBACK_MIN_CANDIDATES: usize = 3;

// =============================================================================
// Name Matching and Validation Constants
// =============================================================================

/// Weight for album name in combined name distance score
/// Album name weighted √2 (1.414) vs artist weight 1.0
pub(crate) const NAME_DISTANCE_ALBUM_WEIGHT: f64 = 1.414;

/// Weight for artist name in combined name distance score
pub(crate) const NAME_DISTANCE_ARTIST_WEIGHT: f64 = 1.0;

/// Minimum Jaro-Winkler similarity for artist name match (0.5 = 50%)
pub(crate) const ARTIST_MISMATCH_THRESHOLD: f64 = 0.5;

/// Minimum Jaro-Winkler similarity for album name match (0.5 = 50%)
pub(crate) const ALBUM_MISMATCH_THRESHOLD: f64 = 0.5;

/// Maximum number of releases to fetch from MusicBrainz across all strategies
pub(crate) const MB_MAX_RELEASES: usize = 150;

/// Maximum Name Distance Rank to accept for release filtering
/// Releases ranked > 50 by name distance are filtered out before detail fetching
pub(crate) const MAX_NAME_DISTANCE_RANK: usize = 50;

/// Minimum combined weighted score for regular albums (Run 25c Combination A)
/// Combined = artist_ratio * ARTIST_WEIGHT + album_ratio * ALBUM_WEIGHT
pub(crate) const MIN_COMBINED_RATIO: f64 = 0.42;

/// Weight for artist name in Combination A filter
pub(crate) const ARTIST_WEIGHT: f64 = 0.4;

/// Weight for album name in Combination A filter
pub(crate) const ALBUM_WEIGHT: f64 = 0.6;

/// Minimum album similarity for "Various Artists" releases (Run 25c)
/// Various Artists releases are filtered by album similarity only
pub(crate) const MIN_VARIOUS_ALBUM_RATIO: f64 = 0.35;

// =============================================================================
// MBID Selection Scoring Constants
// =============================================================================

/// Bonus for CD releases when selecting best MBID (-50 = higher priority)
/// Lower scores = higher priority (minimize score)
pub(crate) const SCORE_CD_BONUS: f64 = -50.0;

/// Bonus for Official status releases (-40 = higher priority)
pub(crate) const SCORE_OFFICIAL_BONUS: f64 = -40.0;

/// Bonus for US releases (-30 = higher priority)
pub(crate) const SCORE_US_BONUS: f64 = -30.0;

// =============================================================================
// Match Confidence Thresholds
// =============================================================================

/// Excellent match threshold (>= 80% of tracks matched)
pub(crate) const CONFIDENCE_EXCELLENT_THRESHOLD: f64 = 80.0;

/// Good match threshold (>= 60% of tracks matched)
pub(crate) const CONFIDENCE_GOOD_THRESHOLD: f64 = 60.0;

/// Fair match threshold (>= 40% of tracks matched)
pub(crate) const CONFIDENCE_FAIR_THRESHOLD: f64 = 40.0;

// =============================================================================
// Artist Mismatch and AcoustID Verification Thresholds
// =============================================================================

/// Minimum match percentage required when artist names don't match
/// Below this threshold, artist mismatches are flagged as "Likely Incorrect"
pub(crate) const ARTIST_MISMATCH_MIN_MATCH_PCT: f64 = 95.0;

/// Minimum match percentage required to trigger AcoustID verification (60%)
pub(crate) const ACOUSTID_VERIFICATION_MIN_MATCH_PCT: f64 = 60.0;

// =============================================================================
// Edition Processing Timing Constants
// =============================================================================

/// Delay between feeding editions to rayon thread pool (seconds)
/// Staggered feeding prevents all editions starting simultaneously
pub(crate) const EDITION_FEED_DELAY_SECS: u64 = 4;

