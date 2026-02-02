//! Partial Album Matching Module
//!
//! **[PLAN028]** Support for matching files that contain a contiguous subset
//! of tracks from the beginning of an album.
//!
//! # Problem
//!
//! Some audio files contain tracks 1-N of an album (e.g., tracks 1-8 of an
//! 11-track album). These files are rejected by the standard 85% duration
//! filter but could still provide valid Recording MBIDs for AcousticBrainz.
//!
//! # Solution
//!
//! This module detects partial album candidates (50-85% duration ratio) and
//! finds the best N tracks that match the file duration, enabling boundary
//! detection to run on just those N tracks.
//!
//! # Example
//!
//! ```rust,ignore
//! use wkmp_ai::matching::partial_matching::*;
//!
//! // Fluke - Puppy: file is 2995s, album is 4039s (11 tracks)
//! let file_duration = 2995.0;
//! let edition_duration = 4039.0;
//!
//! // Check if this is a partial album candidate (74.2% ratio)
//! if is_partial_album_candidate(file_duration, edition_duration) {
//!     // Track durations for Fluke - Puppy
//!     let track_durations = vec![373.0, 362.0, 383.0, 365.0, 402.0,
//!                                333.0, 349.0, 422.0, 348.0, 362.0, 340.0];
//!
//!     // Find that tracks 1-8 (cumulative: 2989s) matches file duration
//!     if let Some(track_count) = find_partial_track_count(file_duration, &track_durations, 0.02) {
//!         assert_eq!(track_count, 8);
//!
//!         // Verify 60% minimum coverage
//!         assert!(is_partial_match_acceptable(track_count, track_durations.len()));
//!     }
//! }
//! ```

/// Minimum duration ratio for partial album candidate detection (50%)
pub const MIN_PARTIAL_RATIO: f64 = 0.50;

/// Maximum duration ratio for partial album candidate (85%)
/// Above this, the standard full-match algorithm should be used
pub const MAX_PARTIAL_RATIO: f64 = 0.85;

/// Minimum track coverage required for partial match (60%)
pub const MIN_TRACK_COVERAGE: f64 = 0.60;

/// Default tolerance for cumulative duration matching (2%)
pub const DEFAULT_DURATION_TOLERANCE: f64 = 0.02;

/// Check if a file is a partial album candidate based on duration ratio.
///
/// A file is considered a partial album candidate if its duration is between
/// 50% and 85% of the edition's total duration.
///
/// # Arguments
///
/// * `file_duration` - Duration of the audio file in seconds
/// * `edition_duration` - Total duration of the edition in seconds
///
/// # Returns
///
/// `true` if the file might contain tracks 1-N of the album
///
/// # Example
///
/// ```rust,ignore
/// // Fluke/Puppy.mp3: 2995s file, 4039s album = 74.2% ratio
/// assert!(is_partial_album_candidate(2995.0, 4039.0));
///
/// // 49% ratio - too short
/// assert!(!is_partial_album_candidate(1960.0, 4000.0));
///
/// // 90% ratio - use full match instead
/// assert!(!is_partial_album_candidate(3600.0, 4000.0));
/// ```
pub fn is_partial_album_candidate(file_duration: f64, edition_duration: f64) -> bool {
    if edition_duration <= 0.0 {
        return false;
    }
    let ratio = file_duration / edition_duration;
    ratio >= MIN_PARTIAL_RATIO && ratio < MAX_PARTIAL_RATIO
}

/// Calculate cumulative track durations.
///
/// Returns a vector where each element is the sum of all track durations
/// up to and including that index.
///
/// # Arguments
///
/// * `track_durations` - Slice of track durations in seconds
///
/// # Returns
///
/// Vector of cumulative durations (same length as input)
///
/// # Example
///
/// ```rust,ignore
/// let durations = vec![300.0, 250.0, 280.0];
/// let cumulative = calculate_cumulative_durations(&durations);
/// assert_eq!(cumulative, vec![300.0, 550.0, 830.0]);
/// ```
pub fn calculate_cumulative_durations(track_durations: &[f64]) -> Vec<f64> {
    let mut cumulative = Vec::with_capacity(track_durations.len());
    let mut sum = 0.0;
    for &duration in track_durations {
        sum += duration;
        cumulative.push(sum);
    }
    cumulative
}

/// Find the number of tracks from the beginning that best match the file duration.
///
/// Searches cumulative track durations to find where the file duration
/// matches within the specified tolerance.
///
/// # Arguments
///
/// * `file_duration` - Duration of the audio file in seconds
/// * `track_durations` - Slice of track durations in seconds
/// * `tolerance` - Relative tolerance for matching (e.g., 0.02 for 2%)
///
/// # Returns
///
/// `Some(n)` where n is the number of tracks (1-indexed) that best match,
/// or `None` if no cumulative duration matches within tolerance.
///
/// # Example
///
/// ```rust,ignore
/// // Fluke - Puppy track durations (11 tracks)
/// let tracks = vec![373.0, 362.0, 383.0, 365.0, 402.0,
///                   333.0, 349.0, 422.0, 348.0, 362.0, 340.0];
///
/// // File duration 2995s matches cumulative 1-8 (2989s) within 2%
/// let count = find_partial_track_count(2995.0, &tracks, 0.02);
/// assert_eq!(count, Some(8));
/// ```
pub fn find_partial_track_count(
    file_duration: f64,
    track_durations: &[f64],
    tolerance: f64,
) -> Option<usize> {
    if track_durations.is_empty() || file_duration <= 0.0 {
        return None;
    }

    let cumulative = calculate_cumulative_durations(track_durations);

    // Find the best match (smallest relative error within tolerance)
    let mut best_match: Option<(usize, f64)> = None;

    for (idx, &cum_duration) in cumulative.iter().enumerate() {
        if cum_duration <= 0.0 {
            continue;
        }

        let relative_error = (file_duration - cum_duration).abs() / cum_duration;

        if relative_error <= tolerance {
            let track_count = idx + 1; // Convert 0-indexed to 1-indexed track count

            // Keep the best match (smallest error)
            match best_match {
                None => best_match = Some((track_count, relative_error)),
                Some((_, prev_error)) if relative_error < prev_error => {
                    best_match = Some((track_count, relative_error))
                }
                _ => {}
            }
        }
    }

    best_match.map(|(count, _)| count)
}

/// Check if a partial match has acceptable track coverage.
///
/// A partial match must cover at least 60% of the album's tracks to be
/// considered useful for AcousticBrainz lookup.
///
/// # Arguments
///
/// * `matched_tracks` - Number of tracks matched
/// * `total_tracks` - Total number of tracks in the album
///
/// # Returns
///
/// `true` if the coverage meets the 60% minimum threshold
///
/// # Example
///
/// ```rust,ignore
/// // 8/11 tracks = 72.7% - acceptable
/// assert!(is_partial_match_acceptable(8, 11));
///
/// // 6/11 tracks = 54.5% - below threshold
/// assert!(!is_partial_match_acceptable(6, 11));
///
/// // Exactly 60% - acceptable
/// assert!(is_partial_match_acceptable(6, 10));
/// ```
pub fn is_partial_match_acceptable(matched_tracks: usize, total_tracks: usize) -> bool {
    if total_tracks == 0 {
        return false;
    }
    let coverage = matched_tracks as f64 / total_tracks as f64;
    coverage >= MIN_TRACK_COVERAGE
}

/// Result of partial album analysis
#[derive(Debug, Clone)]
pub struct PartialAlbumAnalysis {
    /// Whether this is a partial album candidate
    pub is_candidate: bool,
    /// Duration ratio (file / edition)
    pub duration_ratio: f64,
    /// Number of tracks that match file duration (if found)
    pub matched_track_count: Option<usize>,
    /// Total tracks in the edition
    pub total_tracks: usize,
    /// Relative error between file duration and matched cumulative duration
    pub duration_error: Option<f64>,
    /// Whether the track coverage is acceptable
    pub coverage_acceptable: bool,
}

impl PartialAlbumAnalysis {
    /// Analyze whether a file could be a partial album match.
    ///
    /// This performs the full analysis: checking ratio, finding best track count,
    /// and validating coverage.
    ///
    /// # Arguments
    ///
    /// * `file_duration` - Duration of the audio file in seconds
    /// * `track_durations` - Slice of track durations in seconds
    /// * `tolerance` - Relative tolerance for matching (e.g., 0.02 for 2%)
    ///
    /// # Returns
    ///
    /// Analysis result with all computed fields
    pub fn analyze(file_duration: f64, track_durations: &[f64], tolerance: f64) -> Self {
        let total_tracks = track_durations.len();
        let edition_duration: f64 = track_durations.iter().sum();

        let duration_ratio = if edition_duration > 0.0 {
            file_duration / edition_duration
        } else {
            0.0
        };

        let is_candidate = is_partial_album_candidate(file_duration, edition_duration);

        if !is_candidate {
            return Self {
                is_candidate: false,
                duration_ratio,
                matched_track_count: None,
                total_tracks,
                duration_error: None,
                coverage_acceptable: false,
            };
        }

        // Find the best track count
        let matched_track_count = find_partial_track_count(file_duration, track_durations, tolerance);

        let (duration_error, coverage_acceptable) = match matched_track_count {
            Some(count) => {
                // Calculate the actual error
                let cumulative = calculate_cumulative_durations(track_durations);
                let cum_duration = cumulative.get(count - 1).copied().unwrap_or(0.0);
                let error = if cum_duration > 0.0 {
                    (file_duration - cum_duration).abs() / cum_duration
                } else {
                    1.0
                };

                let acceptable = is_partial_match_acceptable(count, total_tracks);
                (Some(error), acceptable)
            }
            None => (None, false),
        };

        Self {
            is_candidate,
            duration_ratio,
            matched_track_count,
            total_tracks,
            duration_error,
            coverage_acceptable,
        }
    }

    /// Check if this analysis indicates a viable partial match.
    ///
    /// A viable partial match has:
    /// - Is a candidate (50-85% ratio)
    /// - Found a matching track count
    /// - Coverage is acceptable (>=60%)
    pub fn is_viable(&self) -> bool {
        self.is_candidate && self.matched_track_count.is_some() && self.coverage_acceptable
    }
}

use crate::matching::types::Edition;

/// Create a partial edition from a full edition.
///
/// Takes the first N tracks of an edition to create a new edition for
/// partial album matching. The partial edition can then be used with
/// the standard matching pipeline.
///
/// # Arguments
///
/// * `edition` - The original full edition
/// * `track_count` - Number of tracks to include (from the beginning)
///
/// # Returns
///
/// A new Edition with only the first `track_count` tracks
pub fn create_partial_edition(edition: &Edition, track_count: usize) -> Edition {
    let track_count = track_count.min(edition.track_count);

    Edition {
        release_mbid: edition.release_mbid.clone(),
        title: edition.title.clone(),
        artist: edition.artist.clone(),
        artist_credit: edition.artist_credit.clone(),
        country: edition.country.clone(),
        status: edition.status.clone(),
        track_count,
        track_durations: edition.track_durations.iter().take(track_count).copied().collect(),
        recording_mbids: edition.recording_mbids.iter().take(track_count).cloned().collect(),
        track_titles: edition.track_titles.iter().take(track_count).cloned().collect(),
        durations: edition.durations.iter().take(track_count).copied().collect(),
        name_distance_rank: edition.name_distance_rank,
        name_distance_score: edition.name_distance_score,
    }
}

/// Find partial album candidates from a list of editions.
///
/// Analyzes each edition to determine if it could be a partial album match
/// for the given file duration. Returns viable partial match candidates
/// with their analysis.
///
/// # Arguments
///
/// * `editions` - List of editions to analyze
/// * `file_duration_secs` - Audio file duration in seconds
/// * `tolerance` - Relative tolerance for duration matching (e.g., 0.02 for 2%)
///
/// # Returns
///
/// Vector of (edition, analysis, partial_track_count) tuples for viable candidates
pub fn find_partial_candidates(
    editions: &[Edition],
    file_duration_secs: f64,
    tolerance: f64,
) -> Vec<(Edition, PartialAlbumAnalysis, usize)> {
    editions
        .iter()
        .filter_map(|edition| {
            let analysis = PartialAlbumAnalysis::analyze(
                file_duration_secs,
                &edition.track_durations,
                tolerance,
            );

            if analysis.is_viable() {
                let track_count = analysis.matched_track_count.unwrap();
                let partial_edition = create_partial_edition(edition, track_count);
                Some((partial_edition, analysis, track_count))
            } else {
                None
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================================================
    // TC-U-PAM-001: Partial candidate detection (50-85%)
    // ==========================================================================

    #[test]
    fn test_partial_candidate_lower_bound_50_percent() {
        // TC-U-PAM-001-A: Lower Bound (50%)
        assert!(is_partial_album_candidate(2000.0, 4000.0)); // Exactly 50%
    }

    #[test]
    fn test_partial_candidate_upper_bound_84_9_percent() {
        // TC-U-PAM-001-B: Upper Bound (84.9%)
        assert!(is_partial_album_candidate(3396.0, 4000.0)); // 84.9%
    }

    #[test]
    fn test_partial_candidate_puppy_case_74_2_percent() {
        // TC-U-PAM-001-C: Typical Partial (74.2% - Puppy case)
        assert!(is_partial_album_candidate(2995.0, 4039.0)); // 74.2%
    }

    // ==========================================================================
    // TC-U-PAM-002: Below threshold rejection
    // ==========================================================================

    #[test]
    fn test_reject_below_threshold_49_percent() {
        // TC-U-PAM-002-A: Below Threshold (49%)
        assert!(!is_partial_album_candidate(1960.0, 4000.0)); // 49%
    }

    #[test]
    fn test_reject_very_low_ratio_25_percent() {
        // TC-U-PAM-002-B: Very Low Ratio (25%)
        assert!(!is_partial_album_candidate(1000.0, 4000.0)); // 25%
    }

    // ==========================================================================
    // TC-U-PAM-003: Above threshold skip (use full match)
    // ==========================================================================

    #[test]
    fn test_skip_at_threshold_85_percent() {
        // TC-U-PAM-003-A: At Threshold (85%)
        assert!(!is_partial_album_candidate(3400.0, 4000.0)); // Exactly 85%
    }

    #[test]
    fn test_skip_above_threshold_95_percent() {
        // TC-U-PAM-003-B: Above Threshold (95%)
        assert!(!is_partial_album_candidate(3800.0, 4000.0)); // 95%
    }

    // ==========================================================================
    // TC-U-PAM-004: Cumulative duration calculation
    // ==========================================================================

    #[test]
    fn test_cumulative_basic() {
        // TC-U-PAM-004-A: Basic Cumulative
        let durations = vec![300.0, 250.0, 280.0, 320.0, 290.0];
        let cumulative = calculate_cumulative_durations(&durations);
        assert_eq!(cumulative, vec![300.0, 550.0, 830.0, 1150.0, 1440.0]);
    }

    #[test]
    fn test_cumulative_single_track() {
        // TC-U-PAM-004-B: Single Track
        let durations = vec![400.0];
        let cumulative = calculate_cumulative_durations(&durations);
        assert_eq!(cumulative, vec![400.0]);
    }

    #[test]
    fn test_cumulative_empty_list() {
        // TC-U-PAM-004-C: Empty List
        let durations: Vec<f64> = vec![];
        let cumulative = calculate_cumulative_durations(&durations);
        assert!(cumulative.is_empty());
    }

    // ==========================================================================
    // TC-U-PAM-005: Best N tracks selection
    // ==========================================================================

    #[test]
    fn test_find_tracks_puppy_scenario() {
        // TC-U-PAM-005-A: Puppy.mp3 Scenario
        // Fluke - Puppy track durations (11 tracks)
        let track_durations = vec![
            373.0, 362.0, 383.0, 365.0, 402.0, 333.0, 349.0, 422.0, 348.0, 362.0, 340.0,
        ];
        // Cumulative 1-8: 373+362+383+365+402+333+349+422 = 2989
        // File duration: 2995

        let count = find_partial_track_count(2995.0, &track_durations, 0.02);
        assert_eq!(count, Some(8));

        // Verify the math
        let cumulative = calculate_cumulative_durations(&track_durations);
        let cum_8 = cumulative[7]; // 0-indexed
        assert!((cum_8 - 2989.0).abs() < 1.0);

        // Verify error is within tolerance
        let error = (2995.0 - cum_8).abs() / cum_8;
        assert!(error < 0.02); // 0.2% < 2%
    }

    #[test]
    fn test_find_tracks_no_match_within_tolerance() {
        // TC-U-PAM-005-B: No Match Within Tolerance
        let track_durations = vec![400.0, 400.0, 400.0, 400.0, 400.0];
        // Cumulative: 400, 800, 1200, 1600, 2000
        // File: 1500 doesn't match any cumulative within 2%

        let count = find_partial_track_count(1500.0, &track_durations, 0.02);
        assert_eq!(count, None);
    }

    #[test]
    fn test_find_tracks_exact_match() {
        // TC-U-PAM-005-C: Exact Match
        let track_durations = vec![400.0, 400.0, 400.0, 400.0, 400.0];
        // Cumulative: 400, 800, 1200, 1600, 2000

        let count = find_partial_track_count(1200.0, &track_durations, 0.02);
        assert_eq!(count, Some(3));
    }

    // ==========================================================================
    // TC-U-PAM-006: Minimum 60% coverage
    // ==========================================================================

    #[test]
    fn test_coverage_above_minimum() {
        // TC-U-PAM-006-A: 11-Track Album at Minimum (7/11 = 63.6%)
        assert!(is_partial_match_acceptable(7, 11));
    }

    #[test]
    fn test_coverage_below_minimum() {
        // TC-U-PAM-006-B: 11-Track Album Below Minimum (6/11 = 54.5%)
        assert!(!is_partial_match_acceptable(6, 11));
    }

    #[test]
    fn test_coverage_puppy_case() {
        // TC-U-PAM-006-C: Puppy Case (8/11 = 72.7%)
        assert!(is_partial_match_acceptable(8, 11));
    }

    #[test]
    fn test_coverage_exactly_60_percent() {
        // TC-U-PAM-006-D: 10-Track Album Minimum (6/10 = 60%)
        assert!(is_partial_match_acceptable(6, 10));
    }

    #[test]
    fn test_coverage_below_60_percent() {
        // TC-U-PAM-006-E: Below 60% (5/10 = 50%)
        assert!(!is_partial_match_acceptable(5, 10));
    }

    // ==========================================================================
    // Integration: PartialAlbumAnalysis
    // ==========================================================================

    #[test]
    fn test_analysis_puppy_full() {
        // Full analysis of Fluke/Puppy.mp3 scenario
        let track_durations = vec![
            373.0, 362.0, 383.0, 365.0, 402.0, 333.0, 349.0, 422.0, 348.0, 362.0, 340.0,
        ];

        let analysis = PartialAlbumAnalysis::analyze(2995.0, &track_durations, 0.02);

        assert!(analysis.is_candidate);
        assert!((analysis.duration_ratio - 0.742).abs() < 0.01); // ~74.2%
        assert_eq!(analysis.matched_track_count, Some(8));
        assert_eq!(analysis.total_tracks, 11);
        assert!(analysis.duration_error.unwrap() < 0.01); // ~0.2%
        assert!(analysis.coverage_acceptable);
        assert!(analysis.is_viable());
    }

    #[test]
    fn test_analysis_not_candidate() {
        // File at 90% ratio should not be a candidate
        let track_durations = vec![100.0, 100.0, 100.0, 100.0, 100.0];

        let analysis = PartialAlbumAnalysis::analyze(450.0, &track_durations, 0.02);

        assert!(!analysis.is_candidate);
        assert!((analysis.duration_ratio - 0.9).abs() < 0.01); // 90%
        assert!(!analysis.is_viable());
    }

    #[test]
    fn test_analysis_insufficient_coverage() {
        // Test case: 12-track album, file matches 6 tracks = 50% coverage (below 60%)
        let track_durations = vec![100.0; 12]; // 1200s total

        // File at 600s (50% ratio) matches first 6 tracks exactly
        // 6/12 = 50% track coverage - below 60% threshold
        let analysis = PartialAlbumAnalysis::analyze(600.0, &track_durations, 0.02);

        assert!(analysis.is_candidate); // 50% ratio is a candidate (50-85%)
        assert_eq!(analysis.matched_track_count, Some(6)); // Matches 6 tracks exactly
        assert!(!analysis.coverage_acceptable); // 6/12 = 50%, below 60%
        assert!(!analysis.is_viable());
    }

    // ==========================================================================
    // Edge cases
    // ==========================================================================

    #[test]
    fn test_zero_edition_duration() {
        assert!(!is_partial_album_candidate(1000.0, 0.0));
    }

    #[test]
    fn test_negative_duration() {
        assert!(!is_partial_album_candidate(-100.0, 1000.0));
    }

    #[test]
    fn test_empty_track_durations() {
        let result = find_partial_track_count(1000.0, &[], 0.02);
        assert_eq!(result, None);
    }

    #[test]
    fn test_zero_total_tracks() {
        assert!(!is_partial_match_acceptable(5, 0));
    }
}
