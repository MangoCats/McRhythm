//! # Helper Utilities
//!
//! Small utility functions used across the album matcher.
//!
//! ## Contents
//! - Error result construction with logging
//! - Match confidence classification
//! - Common helper functions

use crate::constants::{
    CONFIDENCE_EXCELLENT_THRESHOLD, CONFIDENCE_FAIR_THRESHOLD, CONFIDENCE_GOOD_THRESHOLD,
};
use crate::types::ValidationResult;
use crate::utils::query_stats::QueryStats;
use std::path::Path;
use tracing::{error, info};

// =============================================================================
// Error Result Construction
// =============================================================================

/// Create error ValidationResult with logging and query stats cleanup
///
/// Convenience function that:
/// 1. Logs error message with album_id prefix
/// 2. Logs blank line for visual separation
/// 3. Stops query stats heartbeat task
/// 4. Returns ValidationResult::error with provided details
///
/// # Arguments
/// * `query_stats` - QueryStats to stop heartbeat logging
/// * `file_path` - Path to audio file
/// * `artist` - Artist name (from ID3 or path)
/// * `album` - Album name (from ID3 or path)
/// * `track_count` - Number of tracks detected
/// * `album_id` - Album identifier for logging (e.g., "A1")
/// * `message` - Error message describing failure
///
/// # Returns
/// ValidationResult with error status and zero/empty fields
///
/// # Example
/// ```ignore
/// if editions.is_empty() {
///     return make_error_result(
///         &query_stats,
///         &file_path,
///         &artist,
///         &album,
///         detected_tracks,
///         &album_id,
///         "No matching editions found in MusicBrainz".to_string(),
///     );
/// }
/// ```
pub(crate) fn make_error_result(
    query_stats: &QueryStats,
    file_path: &Path,
    artist: &str,
    album: &str,
    track_count: usize,
    album_id: &str,
    message: String,
) -> ValidationResult {
    error!("[{}] FAILED: {}", album_id, message);
    info!("[{}] ", album_id);
    query_stats.stop();
    ValidationResult::error(file_path, artist, album, track_count, message)
}

// =============================================================================
// Match Confidence Classification
// =============================================================================

/// Classify match percentage into confidence level
///
/// Maps match percentage to human-readable confidence category.
/// Used for ValidationResult.confidence field.
///
/// # Arguments
/// * `match_percentage` - Percentage of expected tracks that matched (0-100)
///
/// # Returns
/// Confidence string: "Excellent", "Good", "Fair", or "Poor"
///
/// # Thresholds
/// - Excellent: >= 80%
/// - Good: >= 60%
/// - Fair: >= 40%
/// - Poor: < 40%
///
/// # Example
/// ```ignore
/// assert_eq!(classify_confidence(85.0), "Excellent");
/// assert_eq!(classify_confidence(65.0), "Good");
/// assert_eq!(classify_confidence(50.0), "Fair");
/// assert_eq!(classify_confidence(30.0), "Poor");
/// ```
pub(crate) fn classify_confidence(match_percentage: f64) -> String {
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
