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
    -50.0, -58.0, -60.0, -62.0, -56.0, -52.0,
    -54.0, -48.0, -40.0, -38.0, -34.0, -30.0,
];

/// Stage 2 Parameter Grid: Minimum silence duration values (seconds)
///
/// Ordered by empirical frequency (most common first for early-exit optimization):
/// - 3.0s: 23.8% of successful albums
/// - 2.0s: 25.4%
/// - 0.5s: 8.8%
/// - Others: less common
pub const MIN_DURATION_VALUES: [f64; 15] = [
    3.0, 2.0, 0.5, 1.5, 0.3, 0.8, 0.1, 4.0,
    0.2, 2.5, 1.0, 0.4, 5.0, 3.5, 0.05,
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
pub const TOTAL_PARAM_COMBINATIONS: usize =
    THRESHOLD_VALUES.len() * MIN_DURATION_VALUES.len();

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

// Single-track layer scores
pub const SCORE_FILENAME_PATTERN: f64 = 1.0;
pub const SCORE_DIR_FILES_HIGH: f64 = 0.8;
pub const SCORE_DIR_FILES_MEDIUM: f64 = 0.3;
pub const SCORE_DIR_FILES_SINGLE: f64 = -0.5;
pub const SCORE_ID3_TRACK_TOTAL: f64 = 1.0;
pub const SCORE_ID3_TRACK_NUMBER_ONLY: f64 = 0.4;
pub const SCORE_DURATION_SHORT: f64 = 0.7;
pub const SCORE_DURATION_SUSPICIOUS: f64 = 0.4;
pub const SCORE_DURATION_ALBUM_LENGTH: f64 = -0.3;
pub const SCORE_SILENCE_GAPS_FEW: f64 = 1.0;
pub const SCORE_SILENCE_GAPS_MANY: f64 = -0.5;

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

/// Minimum Jaro-Winkler similarity for artist name match (0.5 = 50%)
pub const MIN_ARTIST_SIMILARITY: f64 = 0.50;

/// Minimum Jaro-Winkler similarity for album name match
pub const ALBUM_MISMATCH_THRESHOLD: f64 = 0.5;

/// Maximum number of releases to fetch from MusicBrainz
pub const MB_MAX_RELEASES: usize = 150;

/// Maximum Name Distance Rank to accept for release filtering
pub const MAX_NAME_DISTANCE_RANK: usize = 50;

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
            assert!(
                duration > 0.0,
                "Duration {} must be positive",
                duration
            );
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
