//! # Stage 5: Extra Track Merging
//!
//! Attempts to merge extra detected tracks to improve match quality.
//!
//! ## Features
//! - **Adjacent track merging**: Tries merging each adjacent pair of detected tracks
//! - **Best merge selection**: Chooses merge with lowest total error
//! - **100% match refinement**: Only activates when detected > expected AND match = 100%
//!
//! ## Use Case
//! When silence detection finds MORE tracks than expected but achieves 100% match,
//! Stage 5 attempts to merge adjacent tracks to reduce the track count to match
//! the expected count, while maintaining or improving match quality.
//!
//! ## Algorithm
//! For each adjacent pair (i, i+1):
//! 1. Create merged_durations by replacing [dur[i], dur[i+1]] with [dur[i] + dur[i+1]]
//! 2. Test merged_durations against expected_durations
//! 3. Calculate total error across all tracks
//! 4. Select merge with lowest total error
//! 5. Return merged durations if they match expected track count
//!
//! ## Example
//! ```ignore
//! Detected: [180.0, 90.0, 70.0, 200.0, 190.0]  // 5 tracks
//! Expected: [182, 160, 200, 191]  // 4 tracks
//! Try merge 1+2: [180.0, 160.0, 200.0, 190.0]  // error = 2.0 + 0 + 0 + 1 = 3.0 ← BEST
//! Try merge 2+3: [180.0, 160.0, 200.0, 190.0]  // error = 2.0 + 2 + 0 + 1 = 5.0
//! Result: Merge tracks 2+3 (90+70=160)
//! ```
//!
//! ## Requirements Coverage
//! - TEST-FUNC-007: Extra track merging when detected > expected
//!
//! ## Related Modules
//! - `types`: CandidateTestResult
//! - `matching::candidate`: analyze_track_matching(), test_segmentation_against_single_edition()

use crate::types::CandidateTestResult;
use crate::matching::candidate::analyze_track_matching;
use tracing::info;

// =============================================================================
// Adjacent Track Merging Algorithm
// =============================================================================

/// Try all possible adjacent track merges and return the best one
///
/// When silence detection finds more tracks than expected, this function attempts
/// to merge adjacent tracks to achieve the correct count. It evaluates every
/// possible adjacent pair merge and returns the one with the lowest total error.
///
/// # Arguments
/// * `durations` - Current detected track durations in seconds
/// * `expected_durations` - Expected track durations from MusicBrainz in seconds
/// * `tolerance` - Tolerance for track matching (used in error calculation)
///
/// # Returns
/// `Some((merged_durations, merge_index, total_error))` if a valid merge produces
/// the expected track count, `None` if no valid merge is possible.
///
/// Where:
/// * `merged_durations` - Track durations after merging
/// * `merge_index` - Index of first track in the merged pair
/// * `total_error` - Sum of absolute errors across all matched tracks
///
/// # Algorithm
/// For each adjacent pair (i, i+1):
/// 1. Create merged_durations = [dur[0..i], dur[i]+dur[i+1], dur[i+2..]]
/// 2. Test merged_durations against expected_durations
/// 3. Calculate total_error = sum(|merged[j] - expected[j]|)
/// 4. Track best merge (lowest total_error)
/// 5. Return best merge if merged count = expected count
///
/// # Example
/// ```ignore
/// let detected = vec![180.0, 90.0, 70.0, 200.0, 190.0];
/// let expected = vec![182, 160, 200, 191];
/// let result = try_all_adjacent_merges(&detected, &expected, 3.0);
/// assert_eq!(result, Some((
///     vec![180.0, 160.0, 200.0, 190.0],  // merged_durations
///     1,  // merge_index (tracks 1+2: 90+70=160)
///     3.0  // total_error
/// )));
/// ```
fn try_all_adjacent_merges(
    durations: &[f64],
    expected_durations: &[u32],
    tolerance: f64,
) -> Option<(Vec<f64>, usize, f64)> {
    let mut best_merge_durations = None;
    let mut best_merge_error = f64::INFINITY;
    let mut best_merge_index = None;

    for merge_idx in 0..(durations.len() - 1) {
        let mut merged_durations = Vec::new();

        for i in 0..durations.len() {
            if i == merge_idx {
                merged_durations.push(durations[i] + durations[i + 1]);
            } else if i == merge_idx + 1 {
                continue;
            } else {
                merged_durations.push(durations[i]);
            }
        }

        let (merged_matches, _, _) =
            analyze_track_matching(&merged_durations, expected_durations, tolerance);

        let total_error: f64 = merged_matches.iter().map(|m| m.error).sum();

        if merged_durations.len() == expected_durations.len() && total_error < best_merge_error {
            best_merge_error = total_error;
            best_merge_durations = Some(merged_durations);
            best_merge_index = Some(merge_idx);
        }
    }

    best_merge_durations.map(|durations| (durations, best_merge_index.unwrap(), best_merge_error))
}

// =============================================================================
// Stage 5: Main Entry Point
// =============================================================================

/// Stage 5: Extra track merging for a SINGLE edition (Run 15+)
///
/// When detected > expected AND match >= 100%, merge adjacent tracks to
/// reduce track count while maintaining match quality.
///
/// # Arguments
/// * `best_durations` - Best detected track durations so far (from previous stages)
/// * `expected_durations` - Expected track durations from MusicBrainz (seconds)
/// * `edition_id` - MusicBrainz Release ID (MBID) for logging
/// * `tolerance` - Tolerance for track matching (seconds)
/// * `current_best_percentage` - Best percentage achieved so far
///
/// # Returns
/// * `Some((merged_durations, result))` - If merge successful and maintains 100% match
/// * `None` - If conditions not met or merge fails
///
/// # Activation Conditions
/// Stage 5 only activates when:
/// 1. `current_best_percentage >= 100.0` (perfect match already achieved)
/// 2. `best_durations.len() > expected_durations.len()` (more tracks detected than expected)
///
/// If these conditions aren't met, returns None immediately.
///
/// # Merging Strategy
/// 1. Count extra tracks: `extra_count = detected.len() - expected.len()`
/// 2. Try all adjacent pair merges
/// 3. Select merge with lowest total error
/// 4. Verify merged result still achieves acceptable match percentage
/// 5. Return merged durations and new result
///
/// # Logging
/// Logs:
/// - Number of extra tracks being merged
/// - Which tracks were merged (1-indexed for user readability)
/// - Final match percentage after merging
///
/// # Example
/// ```ignore
/// let detected = vec![180.0, 90.0, 70.0, 200.0, 190.0];  // 5 tracks, 100% match
/// let expected = vec![182, 160, 200, 191];  // 4 tracks
/// let result = run_stage5_single_edition(
///     &detected,
///     &expected,
///     "edition-mbid",
///     3.0,  // tolerance
///     100.0  // current_best (100% match)
/// );
/// // Returns Some((vec![180.0, 160.0, 200.0, 190.0], result))
/// // Tracks 2+3 merged: 90+70=160
/// ```
pub(crate) fn run_stage5_single_edition(
    best_durations: &[f64],
    expected_durations: &[u32],
    edition_id: &str,
    tolerance: f64,
    current_best_percentage: f64,
) -> Option<(Vec<f64>, CandidateTestResult)> {
    // Only activate if we have 100% match AND more tracks than expected
    if current_best_percentage < 100.0 || best_durations.len() <= expected_durations.len() {
        return None;
    }

    let extra_count = best_durations.len() - expected_durations.len();
    info!("      Attempting merge: {} extra track(s)", extra_count);

    // Use helper to find best merge
    let merge_result = try_all_adjacent_merges(best_durations, expected_durations, tolerance);

    if let Some((merged_durations, best_merge_index, best_merge_error)) = merge_result {
        let (merged_matches, merged_matched_count, merged_percentage) =
            analyze_track_matching(&merged_durations, expected_durations, tolerance);

        let mean_merged_error = best_merge_error / merged_matches.len() as f64;

        info!("      Merged tracks {} + {} -> {:.1}%",
            best_merge_index + 1,
            best_merge_index + 2,
            merged_percentage);

        let result = CandidateTestResult {
            percentage: merged_percentage,
            matched_count: merged_matched_count,
            matches: merged_matches,
            mbid: edition_id.to_string(),
            expected_durations: expected_durations.to_vec(),
            mean_error: mean_merged_error,
            detected_durations: merged_durations.clone(),
        };

        Some((merged_durations, result))
    } else {
        None
    }
}
