//! Stage 3: Over-Segmentation Assembly
//!
//! **[PLAN030]** Uses dynamic programming to find optimal merging of
//! over-segmented tracks to match expected edition durations.
//!
//! # Algorithm Overview
//!
//! When Stage 2 detects more tracks than expected (over-segmentation),
//! Stage 3 tries to merge adjacent detected segments to match the
//! expected track count and durations.
//!
//! Uses dynamic programming:
//! - State: dp[exp_idx][det_idx] = minimum total error to match first
//!   exp_idx expected tracks using first det_idx detected segments
//! - Transition: try merging 1, 2, 3... consecutive segments for each track
//! - Goal: find assignment that uses all detected segments and minimizes error

use crate::matching::types::Edition;

/// Stage 3 result for a single edition
#[derive(Debug, Clone)]
pub struct Stage3Result {
    /// Edition that was tested
    pub edition: Edition,
    /// Best match percentage achieved (0-100)
    pub best_percentage: f64,
    /// Assembled track durations after merging (seconds)
    pub assembled_durations: Vec<f64>,
    /// Merge map: which detected segment indices form each track
    /// merge_map[track_idx] = [segment_idx, segment_idx, ...]
    pub merge_map: Vec<Vec<usize>>,
    /// Per-track errors after assembly (seconds)
    pub track_errors: Vec<f64>,
    /// Number of tracks matched within tolerance
    pub matched_count: usize,
}

/// Run Stage 3 over-segmentation assembly
///
/// Attempts to merge over-segmented tracks using dynamic programming
/// to match expected edition durations.
///
/// # Arguments
/// * `detected_durations` - Over-segmented track durations from Stage 2 (seconds)
/// * `editions` - Candidate editions to test
/// * `tolerance_secs` - Track match tolerance (seconds)
///
/// # Returns
/// Vector of results for editions where assembly was possible,
/// sorted by match percentage descending
pub fn run_stage3(
    detected_durations: &[f64],
    editions: &[Edition],
    tolerance_secs: f64,
) -> Vec<Stage3Result> {
    let detected_count = detected_durations.len();
    let mut results = Vec::new();

    for edition in editions {
        let expected_count = edition.track_count;

        // Only process if we detected MORE tracks than expected (over-segmented)
        if detected_count <= expected_count {
            continue;
        }

        if let Some(result) = try_assemble_tracks(detected_durations, edition, tolerance_secs) {
            results.push(result);
        }
    }

    // Sort by percentage descending
    results.sort_by(|a, b| {
        b.best_percentage
            .partial_cmp(&a.best_percentage)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    results
}

/// Try to assemble detected segments into expected track count using DP
fn try_assemble_tracks(
    detected: &[f64],
    edition: &Edition,
    tolerance_secs: f64,
) -> Option<Stage3Result> {
    let n_detected = detected.len();
    let n_expected = edition.track_count;

    // Convert expected durations to seconds
    let expected_secs: Vec<f64> = edition
        .durations
        .iter()
        .map(|ms| *ms as f64 / 1000.0)
        .collect();

    // DP state: dp[exp_idx][det_idx] = (min_error, merge_choices)
    // exp_idx: number of expected tracks matched so far
    // det_idx: number of detected segments used so far
    let inf = f64::MAX;
    let mut dp: Vec<Vec<(f64, Vec<Vec<usize>>)>> =
        vec![vec![(inf, Vec::new()); n_detected + 1]; n_expected + 1];
    dp[0][0] = (0.0, Vec::new());

    for exp_idx in 0..n_expected {
        for det_start in 0..=n_detected {
            if dp[exp_idx][det_start].0 == inf {
                continue;
            }

            let (prev_error, prev_merges) = dp[exp_idx][det_start].clone();
            let target = expected_secs[exp_idx];

            // Try merging 1, 2, 3... consecutive detected segments
            let mut merged_sum = 0.0;
            for det_end in (det_start + 1)..=n_detected {
                merged_sum += detected[det_end - 1];
                let error = (merged_sum - target).abs();

                // Only consider if within reasonable bounds (3x tolerance for exploration)
                if error <= tolerance_secs * 3.0 {
                    let total_error = prev_error + error;

                    if total_error < dp[exp_idx + 1][det_end].0 {
                        let mut new_merges = prev_merges.clone();
                        let segment_indices: Vec<usize> = (det_start..det_end).collect();
                        new_merges.push(segment_indices);
                        dp[exp_idx + 1][det_end] = (total_error, new_merges);
                    }
                }
            }
        }
    }

    // Find best solution that uses all detected segments
    let (total_error, merge_map) = &dp[n_expected][n_detected];

    if *total_error == inf {
        return None;
    }

    // Calculate assembled durations and per-track errors
    let mut assembled = Vec::new();
    let mut errors = Vec::new();

    for (exp_idx, indices) in merge_map.iter().enumerate() {
        let merged: f64 = indices.iter().map(|&i| detected[i]).sum();
        let target = expected_secs[exp_idx];
        assembled.push(merged);
        errors.push((merged - target).abs());
    }

    // Calculate match percentage
    let matched_count = errors.iter().filter(|&&e| e <= tolerance_secs).count();
    let percentage = (matched_count as f64 / n_expected as f64) * 100.0;

    Some(Stage3Result {
        edition: edition.clone(),
        best_percentage: percentage,
        assembled_durations: assembled,
        merge_map: merge_map.clone(),
        track_errors: errors,
        matched_count,
    })
}

/// Check if Stage 3 found acceptable match
pub fn stage3_success(results: &[Stage3Result], min_percentage: f64) -> bool {
    results
        .first()
        .map(|r| r.best_percentage >= min_percentage)
        .unwrap_or(false)
}

/// Get the best Stage 3 result
pub fn get_best_stage3_result(results: &[Stage3Result]) -> Option<&Stage3Result> {
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
    fn test_basic_merge_two_segments() {
        // Expected: 1 track of 180 seconds
        // Detected: 2 segments of 90 seconds each
        let edition = create_test_edition(1, &[180000]);
        let detected = vec![90.0, 90.0];

        let results = run_stage3(&detected, &[edition], 10.0);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].best_percentage, 100.0);
        assert_eq!(results[0].assembled_durations.len(), 1);
        assert!((results[0].assembled_durations[0] - 180.0).abs() < 0.01);
        assert_eq!(results[0].merge_map[0], vec![0, 1]);
    }

    #[test]
    fn test_complex_merge_pattern() {
        // Expected: 3 tracks of [180, 240, 200] seconds
        // Detected: 5 segments that can be merged as [180], [120+120], [100+100]
        let edition = create_test_edition(3, &[180000, 240000, 200000]);
        let detected = vec![180.0, 120.0, 120.0, 100.0, 100.0];

        let results = run_stage3(&detected, &[edition], 10.0);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].best_percentage, 100.0);
        assert_eq!(results[0].assembled_durations.len(), 3);

        // Track 1: single segment
        assert_eq!(results[0].merge_map[0].len(), 1);
        // Tracks 2 and 3: each merged from 2 segments
        assert_eq!(results[0].merge_map[1].len(), 2);
        assert_eq!(results[0].merge_map[2].len(), 2);
    }

    #[test]
    fn test_no_valid_assembly() {
        // Expected: 2 tracks of [180, 240] seconds
        // Detected: 5 small segments that can't reasonably combine
        let edition = create_test_edition(2, &[180000, 240000]);
        let detected = vec![30.0, 30.0, 30.0, 30.0, 30.0]; // Total 150s, can't make 420s

        let results = run_stage3(&detected, &[edition], 10.0);

        // Should not find a valid assembly (errors too large)
        assert!(results.is_empty() || results[0].best_percentage < 50.0);
    }

    #[test]
    fn test_skips_non_oversegmented() {
        // Expected: 3 tracks
        // Detected: only 3 segments (not over-segmented)
        let edition = create_test_edition(3, &[180000, 240000, 200000]);
        let detected = vec![180.0, 240.0, 200.0];

        let results = run_stage3(&detected, &[edition], 10.0);

        // Should skip because detected count <= expected count
        assert!(results.is_empty());
    }

    #[test]
    fn test_skips_undersegmented() {
        // Expected: 3 tracks
        // Detected: only 2 segments (under-segmented)
        let edition = create_test_edition(3, &[180000, 240000, 200000]);
        let detected = vec![180.0, 440.0];

        let results = run_stage3(&detected, &[edition], 10.0);

        // Should skip because detected count < expected count
        assert!(results.is_empty());
    }

    #[test]
    fn test_partial_match_assembly() {
        // Expected: 3 tracks of [180, 240, 200] seconds
        // Detected: can match first two tracks but third is off
        let edition = create_test_edition(3, &[180000, 240000, 200000]);
        let detected = vec![180.0, 120.0, 120.0, 150.0, 70.0]; // Last track assembles to 220, 20s off

        let results = run_stage3(&detected, &[edition], 10.0);

        assert_eq!(results.len(), 1);
        // 2 out of 3 tracks should match
        assert!((results[0].best_percentage - 66.67).abs() < 1.0);
    }

    #[test]
    fn test_stage3_success() {
        let edition = create_test_edition(1, &[180000]);
        let detected = vec![90.0, 90.0];

        let results = run_stage3(&detected, &[edition], 10.0);

        assert!(stage3_success(&results, 80.0));
        assert!(stage3_success(&results, 100.0));
        assert!(!stage3_success(&[], 80.0));
    }

    #[test]
    fn test_multiple_editions() {
        // Test with multiple editions, only one over-segmented match
        let edition1 = create_test_edition(1, &[180000]); // Can match
        let edition2 = create_test_edition(3, &[60000, 60000, 60000]); // Can match differently
        let edition3 = create_test_edition(5, &[30000; 5]); // Too many expected

        let detected = vec![90.0, 90.0]; // 2 segments, total 180s

        let results = run_stage3(&detected, &[edition1, edition2, edition3], 10.0);

        // Should only process edition1 (1 track) and maybe edition2 if it works
        // edition3 expects 5 tracks but we only have 2 segments
        assert!(!results.is_empty());
        // Best result should be 100% for edition1
        assert_eq!(results[0].best_percentage, 100.0);
    }
}
