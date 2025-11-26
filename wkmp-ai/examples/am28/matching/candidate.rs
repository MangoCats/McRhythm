//! # Candidate Testing Logic
//!
//! Compares detected track boundaries against expected durations,
//! calculates match quality percentage.
//!
//! ## Features
//! - **Track-by-track matching**: Compares detected vs expected durations with tolerance
//! - **Match percentage calculation**: Based on expected track count (not detected)
//! - **Error metrics**: Per-track error and mean error for matched tracks
//! - **Edition testing**: Combines analysis with metadata packaging
//!
//! ## Related Modules
//! - `types`: TrackMatch, CandidateTestResult (data structures)
//! - `stages`: All stages use test_segmentation_against_single_edition()

use crate::types::{TrackMatch, CandidateTestResult};

// =============================================================================
// Core Track Matching Logic
// =============================================================================

/// Analyze track-by-track matching between detected and expected durations
///
/// Compares detected track boundaries against expected track durations from
/// MusicBrainz, applying tolerance threshold to determine matches.
///
/// # Arguments
/// * `detected` - Detected track durations in seconds (from silence detection or assembly)
/// * `expected` - Expected track durations in seconds (from MusicBrainz edition)
/// * `tolerance_secs` - Tolerance for matching (typically MATCH_TOLERANCE_SECS = 3.0s)
///
/// # Returns
/// Tuple of (matches, matched_count, percentage):
/// * `matches` - Vec of TrackMatch structs with per-track details
/// * `matched_count` - Number of tracks within tolerance
/// * `percentage` - Match percentage (matched_count / expected.len() × 100)
///
/// # Algorithm
/// ```ignore
/// for i in 0..min(detected.len(), expected.len()) {
///     error = abs(detected[i] - expected[i])
///     is_match = error <= tolerance_secs
///     if is_match { matched_count++ }
/// }
/// percentage = (matched_count / expected.len()) × 100
/// ```
///
/// # Example
/// ```ignore
/// let detected = vec![180.5, 245.2, 190.0];
/// let expected = vec![182, 243, 191];
/// let (matches, count, pct) = analyze_track_matching(&detected, &expected, 3.0);
/// assert_eq!(count, 3);  // All within 3.0s tolerance
/// assert_eq!(pct, 100.0);
/// ```
///
/// # Note on Percentage Calculation
/// Percentage is based on **expected** track count, not detected count.
/// This means:
/// - Detected 8/10 tracks with 8 matches → 80% (8/10)
/// - Detected 12/10 tracks with 10 matches → 100% (10/10)
/// Extra detected tracks don't reduce percentage (handled by Stage 5 merging).
pub(crate) fn analyze_track_matching(
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

// =============================================================================
// Edition Testing (Combines Analysis + Metadata)
// =============================================================================

/// Test segmentation against a single edition (wrapper for analyze_track_matching)
///
/// Packages track matching analysis with edition metadata into a CandidateTestResult.
/// This is the primary interface used by all stages (2-5) for testing candidates.
///
/// # Arguments
/// * `detected_durations` - Detected track durations in seconds
/// * `expected_durations` - Expected track durations from MusicBrainz in seconds
/// * `edition_id` - MusicBrainz Release ID (MBID) for this edition
/// * `tolerance` - Tolerance for track matching in seconds
///
/// # Returns
/// CandidateTestResult with:
/// * `percentage` - Match percentage (0-100+)
/// * `matched_count` - Number of tracks within tolerance
/// * `matches` - Per-track match details
/// * `mbid` - Edition identifier
/// * `expected_durations` - Copy of expected durations
/// * `mean_error` - Mean error for matched tracks (0 if no matches)
/// * `detected_durations` - Copy of detected durations
///
/// # Usage
/// All stages use this function to evaluate candidates:
/// - **Stage 2**: Test each of 180 parameter combinations
/// - **Stage 3**: Test assembled track sequences
/// - **Stage 4**: Test edition-guided quiet spot boundaries
/// - **Stage 5**: Test merged track sequences
///
/// # Example
/// ```ignore
/// let detected = vec![180.5, 245.2];
/// let expected = vec![182, 243];
/// let result = test_segmentation_against_single_edition(
///     &detected,
///     &expected,
///     "550e8400-e29b-41d4-a716-446655440000",
///     3.0
/// );
/// assert_eq!(result.percentage, 100.0);
/// assert_eq!(result.mean_error, 1.4);  // (1.5 + 2.2) / 2
/// ```
pub(crate) fn test_segmentation_against_single_edition(
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
