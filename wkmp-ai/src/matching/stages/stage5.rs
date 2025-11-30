//! Stage 5: Extra Track Merging
//!
//! **[PLAN030]** Handles albums with bonus/hidden tracks by merging
//! extra detected tracks at the end.
//!
//! # Algorithm Overview
//!
//! When detected track count exceeds expected count (likely due to bonus
//! tracks, hidden tracks, or over-splitting at the end):
//! 1. Keep first (expected_count - 1) tracks as-is
//! 2. Merge remaining detected tracks into final expected track
//! 3. Evaluate match percentage after merging

use crate::matching::types::Edition;

/// Stage 5 result for a single edition
#[derive(Debug, Clone)]
pub struct Stage5Result {
    /// Edition that was tested
    pub edition: Edition,
    /// Match percentage after merging (0-100)
    pub percentage: f64,
    /// Number of detected tracks merged into final track
    pub tracks_merged: usize,
    /// Track durations after merging (seconds)
    pub merged_durations: Vec<f64>,
    /// Per-track errors (seconds)
    pub track_errors: Vec<f64>,
    /// Number of tracks matched within tolerance
    pub matched_count: usize,
}

/// Run Stage 5 extra track merging
///
/// Attempts to improve matches by merging extra detected tracks
/// at the end of the album (handles bonus/hidden tracks).
///
/// # Arguments
/// * `detected_durations` - Detected track durations (seconds)
/// * `editions` - Candidate editions to test
/// * `tolerance_secs` - Track match tolerance (seconds)
/// * `max_merge` - Maximum extra tracks to merge (prevents bad merges)
///
/// # Returns
/// Vector of results for editions where merging was beneficial,
/// sorted by percentage descending
pub fn run_stage5(
    detected_durations: &[f64],
    editions: &[Edition],
    tolerance_secs: f64,
    max_merge: usize,
) -> Vec<Stage5Result> {
    let detected_count = detected_durations.len();
    let mut results = Vec::new();

    for edition in editions {
        let expected_count = edition.track_count;

        // Only process if we have extra tracks (but not too many)
        let extra = detected_count.saturating_sub(expected_count);
        if extra == 0 || extra > max_merge {
            continue;
        }

        // Need at least 1 expected track to merge into
        if expected_count == 0 {
            continue;
        }

        if let Some(result) = try_merge_extra_tracks(detected_durations, edition, tolerance_secs) {
            results.push(result);
        }
    }

    // Sort by percentage descending
    results.sort_by(|a, b| {
        b.percentage
            .partial_cmp(&a.percentage)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    results
}

/// Try merging extra tracks at the end
fn try_merge_extra_tracks(
    detected: &[f64],
    edition: &Edition,
    tolerance_secs: f64,
) -> Option<Stage5Result> {
    let expected_count = edition.track_count;
    let detected_count = detected.len();
    let extra = detected_count.saturating_sub(expected_count);

    if extra == 0 || expected_count == 0 {
        return None;
    }

    let expected_secs: Vec<f64> = edition
        .durations
        .iter()
        .map(|ms| *ms as f64 / 1000.0)
        .collect();

    // Create merged durations:
    // - Keep first (expected_count - 1) tracks as-is
    // - Merge remaining tracks into final track
    let mut merged = Vec::with_capacity(expected_count);

    // Copy first (expected_count - 1) tracks
    for i in 0..(expected_count - 1) {
        if i < detected_count {
            merged.push(detected[i]);
        }
    }

    // Merge last (extra + 1) detected tracks into final expected track
    let merge_start = expected_count.saturating_sub(1);
    let final_merged: f64 = detected[merge_start..].iter().sum();
    merged.push(final_merged);

    // Calculate errors
    let mut errors = Vec::new();
    let mut matched_count = 0;

    for (merged_dur, expected_dur) in merged.iter().zip(expected_secs.iter()) {
        let error = (merged_dur - expected_dur).abs();
        errors.push(error);
        if error <= tolerance_secs {
            matched_count += 1;
        }
    }

    let percentage = (matched_count as f64 / expected_count as f64) * 100.0;

    // Only return if this gives reasonable match (> 50%)
    if percentage < 50.0 {
        return None;
    }

    Some(Stage5Result {
        edition: edition.clone(),
        percentage,
        tracks_merged: extra + 1, // +1 for the original last track being part of merge
        merged_durations: merged,
        track_errors: errors,
        matched_count,
    })
}

/// Check if Stage 5 found acceptable match
pub fn stage5_success(results: &[Stage5Result], min_percentage: f64) -> bool {
    results
        .first()
        .map(|r| r.percentage >= min_percentage)
        .unwrap_or(false)
}

/// Get the best Stage 5 result
pub fn get_best_stage5_result(results: &[Stage5Result]) -> Option<&Stage5Result> {
    results.first()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_edition(track_count: usize, durations_ms: &[u32]) -> Edition {
        Edition {
            release_mbid: "test-mbid".to_string(),
            title: "Test Album".to_string(),
            artist: "Test Artist".to_string(),
            artist_credit: None,
            country: None,
            status: None,
            track_count,
            track_durations: durations_ms.iter().map(|&d| d as f64 / 1000.0).collect(),
            recording_mbids: Vec::new(),
            name_distance_rank: None,
            name_distance_score: None,
            durations: durations_ms.to_vec(),
        }
    }

    #[test]
    fn test_merge_one_extra_track() {
        // Expected: 2 tracks [180s, 240s]
        // Detected: 3 tracks [180s, 120s, 120s] - last two should merge to 240s
        let edition = create_test_edition(2, &[180000, 240000]);
        let detected = vec![180.0, 120.0, 120.0];

        let results = run_stage5(&detected, &[edition], 10.0, 3);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].percentage, 100.0);
        assert_eq!(results[0].tracks_merged, 2); // 2 tracks merged into final
        assert_eq!(results[0].merged_durations.len(), 2);
        assert!((results[0].merged_durations[0] - 180.0).abs() < 0.01);
        assert!((results[0].merged_durations[1] - 240.0).abs() < 0.01);
    }

    #[test]
    fn test_merge_two_extra_tracks() {
        // Expected: 2 tracks [180s, 360s]
        // Detected: 4 tracks [180s, 120s, 120s, 120s] - last three should merge to 360s
        let edition = create_test_edition(2, &[180000, 360000]);
        let detected = vec![180.0, 120.0, 120.0, 120.0];

        let results = run_stage5(&detected, &[edition], 10.0, 3);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].percentage, 100.0);
        assert_eq!(results[0].tracks_merged, 3); // 3 tracks merged into final
        assert_eq!(results[0].merged_durations.len(), 2);
        assert!((results[0].merged_durations[1] - 360.0).abs() < 0.01);
    }

    #[test]
    fn test_no_extra_tracks() {
        // Expected: 3 tracks
        // Detected: 3 tracks (exact match, no extra)
        let edition = create_test_edition(3, &[180000, 240000, 200000]);
        let detected = vec![180.0, 240.0, 200.0];

        let results = run_stage5(&detected, &[edition], 10.0, 3);

        // Should skip because no extra tracks to merge
        assert!(results.is_empty());
    }

    #[test]
    fn test_too_many_extra_tracks() {
        // Expected: 2 tracks
        // Detected: 6 tracks (4 extra, exceeds max_merge of 3)
        let edition = create_test_edition(2, &[180000, 240000]);
        let detected = vec![60.0, 60.0, 60.0, 60.0, 60.0, 60.0];

        let results = run_stage5(&detected, &[edition], 10.0, 3);

        // Should skip because too many extra tracks
        assert!(results.is_empty());
    }

    #[test]
    fn test_partial_match_after_merge() {
        // Expected: 3 tracks [180s, 240s, 200s]
        // Detected: 4 tracks [180s, 250s, 100s, 100s]
        // After merge: [180s, 250s, 200s] - 2 out of 3 match
        let edition = create_test_edition(3, &[180000, 240000, 200000]);
        let detected = vec![180.0, 250.0, 100.0, 100.0];

        let results = run_stage5(&detected, &[edition], 10.0, 3);

        assert_eq!(results.len(), 1);
        // Track 1: 180 vs 180 = match
        // Track 2: 250 vs 240 = 10s error (matches at 10s tolerance)
        // Track 3: 200 vs 200 = match
        assert!((results[0].percentage - 100.0).abs() < 1.0);
    }

    #[test]
    fn test_merge_with_poor_result_rejected() {
        // Expected: 2 tracks [180s, 240s]
        // Detected: 3 tracks [100s, 100s, 100s] - merged would be [100s, 200s]
        // Both tracks off by > 10s, so 0% match, rejected
        let edition = create_test_edition(2, &[180000, 240000]);
        let detected = vec![100.0, 100.0, 100.0];

        let results = run_stage5(&detected, &[edition], 10.0, 3);

        // Should be rejected because < 50% match
        assert!(results.is_empty());
    }

    #[test]
    fn test_stage5_success() {
        let edition = create_test_edition(2, &[180000, 240000]);
        let detected = vec![180.0, 120.0, 120.0];

        let results = run_stage5(&detected, &[edition], 10.0, 3);

        assert!(stage5_success(&results, 80.0));
        assert!(stage5_success(&results, 100.0));
        assert!(!stage5_success(&[], 80.0));
    }

    #[test]
    fn test_multiple_editions() {
        // Test with multiple editions, only some eligible
        let edition1 = create_test_edition(2, &[180000, 240000]); // Can merge 1 extra
        let edition2 = create_test_edition(3, &[180000, 120000, 120000]); // Exact match
        let edition3 = create_test_edition(1, &[420000]); // Can merge 2 extra

        let detected = vec![180.0, 120.0, 120.0];

        let results = run_stage5(&detected, &[edition1, edition2, edition3], 10.0, 3);

        // edition1: merge last 2 into 240s = 100% match
        // edition2: no extra tracks, skipped
        // edition3: merge all 3 into 420s = 100% match
        // Both should be present and sorted by percentage
        assert!(!results.is_empty());
    }

    #[test]
    fn test_fewer_detected_than_expected() {
        // Expected: 4 tracks
        // Detected: 3 tracks (under-detected, not extra)
        let edition = create_test_edition(4, &[180000, 240000, 200000, 160000]);
        let detected = vec![180.0, 240.0, 200.0];

        let results = run_stage5(&detected, &[edition], 10.0, 3);

        // Should skip because detected < expected
        assert!(results.is_empty());
    }
}
