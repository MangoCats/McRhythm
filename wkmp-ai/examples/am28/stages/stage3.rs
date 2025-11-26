//! # Stage 3: Dynamic Programming Assembly
//!
//! Assembles over-segmented tracks into proper track boundaries using
//! dynamic programming to find optimal segment merging.
//!
//! ## Features
//! - **Dynamic programming assembly**: Optimal segment merging using DP[i][j] state table
//! - **Over-segmentation handling**: Merges N detected segments into K expected tracks
//! - **Error minimization**: Finds assembly with lowest total duration error
//! - **Early-exit support**: Honors grace period for parallel processing
//!
//! ## Algorithm
//! Uses dynamic programming to find the optimal way to group N over-segmented
//! tracks into K expected tracks, minimizing total duration error.
//!
//! **State Definition:**
//! ```ignore
//! dp[i][j] = (min_error, split_positions)
//! // min_error: minimum total duration error when grouping first i segments into j tracks
//! // split_positions: where to split the segments to achieve this grouping
//! ```
//!
//! **Recurrence:**
//! ```ignore
//! dp[i][j] = min over start ∈ [j-1, i) of {
//!     dp[start][j-1].error + |sum(segments[start..i]) - expected[j-1]|
//! }
//! ```
//!
//! **Example:**
//! 7 detected segments → 5 expected tracks
//! ```ignore
//! Detected: [180, 90, 70, 200, 50, 150, 190]
//! Expected: [182, 165, 195, 155, 191]
//! Assembly: [180, 90+70=160, 200, 50+150=200, 190]
//! ```
//!
//! ## Requirements Coverage
//! - TEST-FUNC-005: Dynamic programming assembly for over-segmented tracks
//!
//! ## Related Modules
//! - `types`: OverSegmentedCandidate, CandidateTestResult
//! - `matching::candidate`: test_segmentation_against_single_edition()
//! - `utils::early_exit`: should_exit_early(), grace period coordination

use crate::types::{OverSegmentedCandidate, CandidateTestResult};
use crate::matching::candidate::test_segmentation_against_single_edition;
use crate::utils::early_exit::should_exit_early;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::time::Instant;
use tracing::info;

// =============================================================================
// Dynamic Programming Assembly Algorithm
// =============================================================================

/// Assemble over-segmented tracks using dynamic programming
///
/// Finds the optimal way to merge N detected segments into K expected tracks
/// by minimizing total duration error. Uses dynamic programming with O(N²K)
/// time complexity.
///
/// # Arguments
/// * `detected_durations` - Detected track durations (N segments, over-segmented)
/// * `expected_durations` - Expected track durations (K tracks from MusicBrainz)
///
/// # Returns
/// * `Some(assembled_durations)` - K assembled track durations if successful
/// * `None` - If N ≤ K (not over-segmented) or DP failed to find valid solution
///
/// # Algorithm Details
/// **State:** dp[i][j] = (min_error, split_positions)
/// - i: number of segments used (0..=N)
/// - j: number of tracks formed (0..=K)
/// - min_error: minimum total duration error for this subproblem
/// - split_positions: where to split segments to achieve this grouping
///
/// **Base Case:** dp[0][0] = (0.0, [])
/// - 0 segments into 0 tracks has 0 error
///
/// **Recurrence:** For each (i, j):
/// ```ignore
/// for start in (j-1)..i {
///     track_duration = sum(detected[start..i])
///     error = |track_duration - expected[j-1]|
///     total_error = dp[start][j-1].error + error
///     if total_error < dp[i][j].error {
///         dp[i][j] = (total_error, dp[start][j-1].splits + [start])
///     }
/// }
/// ```
///
/// **Solution:** dp[N][K].splits defines the optimal grouping
///
/// # Example
/// ```ignore
/// let detected = vec![180.0, 90.0, 70.0, 200.0, 190.0];  // 5 segments
/// let expected = vec![182, 165, 195, 191];  // 4 tracks
/// let assembled = assemble_segments_dp(&detected, &expected);
/// assert_eq!(assembled, Some(vec![180.0, 160.0, 200.0, 190.0]));
/// // Grouping: [180] [90+70] [200] [190]
/// ```
///
/// # Complexity
/// - **Time:** O(N²K) where N = detected segments, K = expected tracks
/// - **Space:** O(NK) for DP table
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

// =============================================================================
// Stage 3: Assembly Loop with Early-Exit
// =============================================================================

/// Stage 3: Segment assembly for a SINGLE edition (Run 15+)
///
/// Attempts to assemble over-segmented candidates to match the expected track count
/// for this edition. Tests all candidates and returns the best result found.
///
/// # Arguments
/// * `over_segmented_candidates` - Candidates with more tracks than expected (from Stage 2)
/// * `expected_durations` - Expected track durations from MusicBrainz edition (seconds)
/// * `edition_id` - MusicBrainz Release ID (MBID) for logging
/// * `tolerance` - Tolerance for track matching (seconds)
/// * `current_best_percentage` - Best percentage achieved so far
/// * `edition_idx` - Index of this edition (0-based, for logging)
/// * `total_editions` - Total number of editions being tested (for logging)
/// * `perfect_match_found` - Shared atomic flag for early-exit coordination
/// * `perfect_match_time_ms` - Timestamp when first 100% match was found
/// * `start_time` - Reference start time for grace period calculation
/// * `album_idx` - Album index (0-based, for logging)
///
/// # Returns
/// * `Some(result)` - Best CandidateTestResult found via assembly
/// * `None` - If early-exit triggered, no candidates, or no improvement
///
/// # Early-Exit Behavior
/// **Input early-exit:** If `current_best_percentage >= 100.0`, returns None immediately.
///
/// **Loop early-exit:** Checks should_exit_early() before each candidate assembly.
/// If grace period has expired, stops processing and returns current best.
///
/// **Output early-exit:** If assembly achieves 100% match, returns immediately
/// with that result.
///
/// # Candidate Processing
/// For each over-segmented candidate:
/// 1. Check early-exit conditions
/// 2. Call assemble_segments_dp() to merge segments
/// 3. Test assembled result against expected durations
/// 4. Update best result if improved
/// 5. Return immediately if 100% match found
///
/// # Logging
/// Logs progress every assembly attempt that improves the best result, including:
/// - Edition index and total count
/// - New best percentage
/// - Source parameter combination (threshold, min_duration)
/// - Track count change (before → after assembly)
///
/// At end of stage, logs total number of assemblies tested.
///
/// # Example
/// ```ignore
/// let candidates = vec![
///     OverSegmentedCandidate {
///         durations: vec![180.0, 90.0, 70.0, 200.0, 190.0],
///         threshold_db: -50.0,
///         min_duration_secs: 3.0,
///         track_count: 5,
///     },
/// ];
/// let result = run_stage3_single_edition(
///     &candidates,
///     &vec![182, 160, 200, 191],  // Expected 4 tracks
///     "edition-mbid",
///     3.0,  // tolerance
///     0.0,  // current_best
///     0,    // edition_idx
///     1,    // total_editions
///     &AtomicBool::new(false),
///     &AtomicU64::new(0),
///     Instant::now(),
///     0,    // album_idx
/// );
/// ```
pub(crate) fn run_stage3_single_edition(
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
    album_idx: usize,
) -> Option<CandidateTestResult> {
    // Early-exit if already have 100% match
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
                    info!("[A{}]       [Edition {}/{}] New best: {:.1}% via assembly ({}dB, {}s) ({} -> {} tracks)",
                        album_idx + 1, edition_idx + 1, total_editions,
                        result.percentage,
                        candidate.threshold_db, candidate.min_duration_secs,
                        candidate.track_count, expected_durations.len());

                    best_result = Some(result);

                    // Early-exit if 100% match found
                    if best_result.as_ref().unwrap().percentage >= 100.0 {
                        return best_result;
                    }
                }
            }
        }
    }

    if assemblies_tested > 0 {
        info!("[A{}]       [Edition {}/{}] Tested {} assemblies", album_idx + 1, edition_idx + 1, total_editions, assemblies_tested);
    }

    best_result
}
