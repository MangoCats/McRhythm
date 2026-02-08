//! Album Matching Constants
//!
//! **[PLAN030]** Default parameters for the am28 album matching algorithm.
//!
//! ## Parameter Grid Ordering
//!
//! THRESHOLD_VALUES and MIN_DURATION_VALUES are ordered by empirical frequency
//! from Run 27 analysis (193 successful albums). This ordering is critical for
//! early-exit optimization performance.

// =============================================================================
// Stage 2 Parameter Grid (Empirically Ordered)
// =============================================================================

/// Stage 2 Parameter Grid: Threshold values (dB) for silence detection
///
/// Ordered by empirical frequency (most common first for early-exit optimization):
/// - -50dB: 53.9% of successful albums
/// - -58dB: 15.5%
/// - -60dB: 5.2%
/// - Others: rare
pub const THRESHOLD_VALUES: [f64; 12] = [
    -50.0, -58.0, -60.0, -62.0, -56.0, -52.0, -54.0, -48.0, -40.0, -38.0, -34.0, -30.0,
];

/// Stage 2 Parameter Grid: Minimum silence duration values (seconds)
///
/// Ordered by empirical frequency (most common first for early-exit optimization):
/// - 3.0s: 23.8% of successful albums
/// - 2.0s: 25.4%
/// - 0.5s: 8.8%
/// - Others: less common
pub const MIN_DURATION_VALUES: [f64; 15] = [
    3.0, 2.0, 0.5, 1.5, 0.3, 0.8, 0.1, 4.0, 0.2, 2.5, 1.0, 0.4, 5.0, 3.5, 0.05,
];

/// Default silence detection threshold (dB) - most common parameter
pub const DEFAULT_THRESHOLD_DB: f64 = -50.0;

/// Default minimum silence duration (seconds) - most common parameter
pub const DEFAULT_MIN_DURATION_SECS: f64 = 3.0;

/// Default threshold index in THRESHOLD_VALUES array
pub const DEFAULT_THRESHOLD_IDX: usize = 0;

/// Default min duration index in MIN_DURATION_VALUES array
pub const DEFAULT_MIN_DURATION_IDX: usize = 0;

/// Total parameter combinations (12 × 15 = 180)
pub const TOTAL_PARAM_COMBINATIONS: usize = THRESHOLD_VALUES.len() * MIN_DURATION_VALUES.len();

// =============================================================================
// Silence Detection Constants
// =============================================================================

/// RMS window for short silences (≤0.3s min duration)
pub const RMS_WINDOW_SHORT_SECS: f64 = 0.025;

/// RMS window for medium silences (0.3-0.6s)
pub const RMS_WINDOW_MEDIUM_SECS: f64 = 0.05;

/// RMS window for longer silences (>0.6s)
pub const RMS_WINDOW_STANDARD_SECS: f64 = 0.1;

/// 50% overlap between RMS windows
pub const RMS_WINDOW_OVERLAP: f64 = 0.5;

/// Threshold for using short window
pub const RMS_WINDOW_THRESHOLD_SHORT: f64 = 0.3;

/// Threshold for using medium window
pub const RMS_WINDOW_THRESHOLD_MEDIUM: f64 = 0.6;

/// Minimum dB floor for silence calculations
pub const SILENCE_DB_FLOOR: f32 = -100.0;

/// RMS epsilon threshold
pub const SILENCE_RMS_EPSILON: f32 = 1e-10;

// =============================================================================
// Match Tolerance and Grace Periods
// =============================================================================

/// Tolerance for track duration matching (seconds)
pub const MATCH_TOLERANCE_SECS: f64 = 10.0;

/// Grace period for short tracks (seconds)
pub const GRACE_PERIOD_SECS: f64 = 2.0;

/// Threshold for applying grace period (seconds)
pub const GRACE_PERIOD_THRESHOLD_SECS: u32 = 180;

// =============================================================================
// Edition Selection and Penalties
// =============================================================================

/// Penalty per extra track when selecting winning edition
pub const TRACK_COUNT_PENALTY_PER_EXTRA: f64 = 2.0;

/// Maximum number of editions to test per album
pub const MAX_EDITIONS_TO_TEST: usize = 10;

/// Match percentage penalty for Stage 4 results (quiet spot detection)
pub const STAGE4_PENALTY_PERCENT: f64 = 25.0;

// =============================================================================
// Sentinel Values
// =============================================================================

/// Sentinel value for missing/unknown metadata
pub const UNKNOWN_VALUE: &str = "Unknown";

/// Sentinel value for Various Artists compilations
pub const VARIOUS_ARTISTS: &str = "Various Artists";

// =============================================================================
// Single-Track Discriminator Constants
// =============================================================================

/// Filename pattern for detecting track number prefixes
pub const SINGLE_TRACK_FILENAME_PATTERN: &str = r"^(\d{1,2})\s*[-_\.]\s*";

/// Directory file count threshold for single-track detection
pub const SINGLE_TRACK_DIR_FILE_THRESHOLD: usize = 4;

/// Minimum expected album duration (minutes)
pub const SINGLE_TRACK_MIN_ALBUM_DURATION_MINS: f64 = 20.0;

/// Typical single track duration (minutes)
pub const SINGLE_TRACK_TYPICAL_DURATION_MINS: f64 = 8.0;

/// Minimum expected silence gaps in album
pub const SINGLE_TRACK_MIN_EXPECTED_GAPS: usize = 3;

/// Single-track score threshold
pub const SINGLE_TRACK_SCORE_THRESHOLD: f64 = 1.5;

/// ID3 track total threshold (if total > this, definitely single track)
pub const SINGLE_TRACK_ID3_TOTAL_THRESHOLD: u32 = 1;

// Single-track layer scores (positive = single-track indicator, negative = album indicator)

/// Score for filename matching track number pattern (e.g., "01 - Track.mp3")
pub const SCORE_FILENAME_PATTERN: f64 = 1.0;
/// Score when directory has many audio files (album indicator)
pub const SCORE_DIR_FILES_HIGH: f64 = 0.8;
/// Score when directory has moderate files
pub const SCORE_DIR_FILES_MEDIUM: f64 = 0.3;
/// Score when directory has single audio file
pub const SCORE_DIR_FILES_SINGLE: f64 = -0.5;
/// Score when ID3 track/total metadata present
pub const SCORE_ID3_TRACK_TOTAL: f64 = 1.0;
/// Score when only track number (no total) present
pub const SCORE_ID3_TRACK_NUMBER_ONLY: f64 = 0.4;
/// Score for short duration (<8 min typical single)
pub const SCORE_DURATION_SHORT: f64 = 0.7;
/// Score for ambiguous duration range
pub const SCORE_DURATION_SUSPICIOUS: f64 = 0.4;
/// Score for album-length duration (>20 min)
pub const SCORE_DURATION_ALBUM_LENGTH: f64 = -0.3;
/// Score when few silence gaps detected (single-track indicator)
pub const SCORE_SILENCE_GAPS_FEW: f64 = 1.0;
/// Score when many silence gaps detected (album indicator)
pub const SCORE_SILENCE_GAPS_MANY: f64 = -0.5;

/// Minimum segment duration threshold for near-zero artifact filtering.
/// Segments below this threshold are merged into their predecessor (or successor
/// if first segment) during silence detection. No real music track is < 0.5s.
pub const MIN_SEGMENT_DURATION_SECS: f64 = 0.5;

/// Audio file extensions for directory scanning
pub const AUDIO_EXTENSIONS: &[&str] = &["mp3", "flac", "m4a", "ogg", "wav", "aac", "wma", "opus"];

// =============================================================================
// MusicBrainz API Rate Limiting
// =============================================================================

/// MusicBrainz API rate limit (milliseconds between requests)
pub const MB_RATE_LIMIT_MS: u64 = 1550;

/// Exponential backoff delays for retry attempts (seconds)
pub const MB_RETRY_DELAYS_SECS: &[u64] = &[0, 5, 15, 45, 60];

/// HTTP request timeout for MusicBrainz API calls (seconds)
pub const MB_REQUEST_TIMEOUT_SECS: u64 = 30;

// =============================================================================
// Stage 4: Quiet Spot Detection Constants
// =============================================================================

/// RMS window size for quiet spot detection (seconds)
pub const QUIET_SPOT_WINDOW_SECS: f64 = 0.5;

/// Search radius as percentage of expected track duration
pub const QUIET_SPOT_SEARCH_RADIUS_RATIO: f64 = 0.15;

/// Minimum search radius (seconds)
pub const QUIET_SPOT_SEARCH_RADIUS_MIN: f64 = 5.0;

/// Maximum search radius (seconds)
pub const QUIET_SPOT_SEARCH_RADIUS_MAX: f64 = 20.0;

/// Number of top editions to test with guided quiet spot detection
pub const QUIET_SPOT_TOP_EDITIONS: usize = 5;

// =============================================================================
// Early-Exit Grace Period
// =============================================================================

/// Grace period after finding 100% match before allowing early-exit (seconds)
pub const EARLY_EXIT_GRACE_PERIOD_SECS: u64 = 20;

// =============================================================================
// Artist Matching Constants
// =============================================================================

/// Minimum artist similarity for pre-filtering (0.60 = 60%)
///
/// Uses hybrid Jaccard + Levenshtein similarity (not Jaro-Winkler).
/// This threshold filters out clearly incorrect artists before expensive
/// multi-stage matching begins.
///
/// **Rationale for 0.60:**
/// - "The Beatles" vs "Beatles": ~0.636 (passes)
/// - "The Cars" vs "Cars": ~0.667 (passes)
/// - "The Cars" vs "Stephan Mathieu": ~0.160 (filtered)
/// - Typos like "Led Zepelin" vs "Led Zeppelin": ~0.950 (passes)
///
/// Lower threshold (0.50) would allow too many false positives.
/// Higher threshold (0.65+) would filter "Beatles" variation (~0.636).
pub const MIN_ARTIST_SIMILARITY: f64 = 0.60;

/// Album title Jaro-Winkler similarity threshold below which a penalty is applied.
/// An album_sim < 0.35 means the titles are substantially different (e.g.,
/// "Dancer and the Moon" vs "BeyondTheSunset"), strong evidence of wrong edition.
pub const ALBUM_MISMATCH_THRESHOLD: f64 = 0.35;

/// Penalty subtracted from name_distance_score when album_sim < ALBUM_MISMATCH_THRESHOLD.
/// Applied in `calculate_name_distance()` to reduce score for clearly wrong album titles.
pub const ALBUM_MISMATCH_PENALTY: f64 = 0.15;

// =============================================================================
// Post-Selection Name Sanity Check
// =============================================================================

/// Minimum name_score gap between a better-named candidate and the current winner
/// to consider overriding. 0.15 means the alternative must have ≥15% higher name score.
pub const NAME_OVERRIDE_GAP: f64 = 0.15;

/// Minimum match percentage for the override candidate. Must have at least some
/// tracks matching to be considered a valid alternative.
pub const NAME_OVERRIDE_MIN_MATCH_PCT: f64 = 25.0;

/// Last track timing error threshold (seconds) that flags suspicious Stage4 overflow.
/// When Stage4 can't find a boundary, it dumps remaining audio into the last track,
/// causing errors >100s.
pub const LAST_TRACK_ERROR_THRESHOLD: f64 = 60.0;

/// Name score below which the winner is considered suspiciously low, even without
/// a large last-track error.
pub const NAME_OVERRIDE_LOW_THRESHOLD: f64 = 0.60;

/// Maximum number of releases to fetch from MusicBrainz
pub const MB_MAX_RELEASES: usize = 150;

/// Maximum Name Distance Rank to accept for release filtering
pub const MAX_NAME_DISTANCE_RANK: usize = 50;

// =============================================================================
// Duration Filtering Constants
// =============================================================================

/// Minimum duration ratio for edition filtering (80%)
/// Allows file to be up to 20% longer than edition's total duration.
/// Lowered from 85% to 80% to capture borderline cases like
/// Guardians of the Galaxy (84.95% ratio, rejected at 85%).
pub const MIN_DURATION_RATIO: f64 = 0.80;

/// Maximum duration ratio for edition filtering (125%)
/// Rejects editions whose total duration exceeds 125% of file duration.
pub const MAX_DURATION_RATIO: f64 = 1.25;

// =============================================================================
// Artist-Relaxed Fallback Search
// =============================================================================

/// Maximum releases to fetch in album-only fallback search
pub const FALLBACK_SEARCH_LIMIT: usize = 25;

// =============================================================================
// Match Confidence Thresholds
// =============================================================================

/// Excellent match threshold (>= 80% of tracks matched)
pub const CONFIDENCE_EXCELLENT_THRESHOLD: f64 = 80.0;

/// Good match threshold (>= 60% of tracks matched)
pub const CONFIDENCE_GOOD_THRESHOLD: f64 = 60.0;

/// Fair match threshold (>= 40% of tracks matched)
pub const CONFIDENCE_FAIR_THRESHOLD: f64 = 40.0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_threshold_values_in_valid_range() {
        for threshold in THRESHOLD_VALUES {
            assert!(
                threshold >= -70.0 && threshold <= -30.0,
                "Threshold {} not in valid range [-70, -30] dB",
                threshold
            );
        }
    }

    #[test]
    fn test_min_duration_values_positive() {
        for duration in MIN_DURATION_VALUES {
            assert!(duration > 0.0, "Duration {} must be positive", duration);
        }
    }

    #[test]
    fn test_total_combinations() {
        assert_eq!(TOTAL_PARAM_COMBINATIONS, 180);
        assert_eq!(
            THRESHOLD_VALUES.len() * MIN_DURATION_VALUES.len(),
            TOTAL_PARAM_COMBINATIONS
        );
    }
}
