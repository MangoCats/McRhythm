//! # Stage 2: Parameter Optimization (180-param sweep)
//!
//! Tests all 180 parameter combinations (12 thresholds × 15 min_durations)
//! in frequency order with early-exit on 100% match.
//!
//! ## Features
//! - **180-parameter grid search**: 12 thresholds × 15 min_durations
//! - **Empirical ordering**: Parameters ordered by Run 27 frequency (most common first)
//! - **Early-exit optimization**: Stops at first 100% match (87.6% of albums benefit)
//! - **Over-segmented collection**: Gathers candidates for Stage 3 assembly
//!
//! ## Performance Characteristics
//! From Run 27 analysis (193 successful albums):
//! - **87.6% early-exit rate**: 169 albums found 100% match before testing all 180 params
//! - **Median exit rank**: 4.0 (57.4% of perfect matches exit in first 5 attempts)
//! - **Mean exit rank**: 10.7 parameters tested (vs 180 without early-exit)
//! - **Expected savings**: ~0.35s per album
//!
//! ## Algorithm
//! ```ignore
//! for thresh_idx in 0..12 {
//!     for min_dur_idx in 0..15 {
//!         durations = silence_cache[thresh_idx * 15 + min_dur_idx]
//!         result = test_segmentation(durations, expected)
//!         if result.percentage > best_percentage {
//!             best_result = result
//!             if result.percentage >= 100.0 {
//!                 return early  // Early-exit optimization
//!             }
//!         }
//!         if durations.len() > expected.len() {
//!             collect for Stage 3
//!         }
//!     }
//! }
//! ```
//!
//! ## Requirements Coverage
//! - TEST-FUNC-002a: STAGE2_THRESHOLD_VALUES empirical ordering preserved
//! - TEST-FUNC-002b: STAGE2_MIN_DURATION_VALUES empirical ordering preserved
//! - TEST-FUNC-004: Early-exit when 100% match found
//!
//! ## Related Modules
//! - `constants`: STAGE2_THRESHOLD_VALUES, STAGE2_MIN_DURATION_VALUES
//! - `types`: SilenceCache, OverSegmentedCandidate, SingleEditionStage2Results
//! - `matching::candidate`: test_segmentation_against_single_edition()

use crate::constants::{STAGE2_THRESHOLD_VALUES, STAGE2_MIN_DURATION_VALUES};
use crate::types::{SilenceCache, OverSegmentedCandidate, SingleEditionStage2Results, CandidateTestResult};
use crate::matching::candidate::test_segmentation_against_single_edition;

// =============================================================================
// Stage 2: 180-Parameter Grid Search with Early-Exit
// =============================================================================

/// Stage 2: Parameter optimization using pre-computed silence cache (Run 16+)
///
/// Tests all 180 parameter combinations (12 thresholds × 15 min_durations) against
/// a single edition, using pre-computed track durations from SilenceCache. Returns
/// the best result found and collects over-segmented candidates for Stage 3.
///
/// # Arguments
/// * `silence_cache` - Pre-computed track durations for all 180 parameter combinations
/// * `num_thresholds` - Number of threshold values (12 in Run 28)
/// * `num_min_durations` - Number of min_duration values (15 in Run 28)
/// * `expected_durations` - Expected track durations from MusicBrainz edition (seconds)
/// * `edition_id` - MusicBrainz Release ID (MBID) for logging
/// * `tolerance` - Tolerance for track matching (seconds, typically 3.0)
/// * `current_best_percentage` - Best percentage achieved so far (from previous stages/editions)
///
/// # Returns
/// SingleEditionStage2Results with:
/// * `best_result` - Best CandidateTestResult found (None if early-exit or no improvement)
/// * `over_segmented_candidates` - Candidates with more tracks than expected (for Stage 3)
/// * `best_threshold` - Threshold value that produced best result
/// * `best_min_duration` - Min duration value that produced best result
///
/// # Early-Exit Behavior
/// **Input early-exit**: If `current_best_percentage >= 100.0`, returns immediately
/// with None result (no need to test this edition).
///
/// **Output early-exit**: If any parameter combination achieves 100% match, returns
/// immediately with that result (no need to test remaining parameters).
///
/// **Run 28 Optimization**: Combined with empirically reordered STAGE2 arrays
/// (most likely parameters tested first), achieves early-exit for 87.6% of albums
/// (169 of 193 successful in Run 27). Median exit rank 4.0 means 57.4% of perfect
/// matches exit in first 5 attempts.
///
/// # Over-Segmented Candidates
/// Any parameter combination that produces more tracks than expected is collected
/// for Stage 3 dynamic programming assembly. Stage 3 attempts to merge segments
/// to match the expected track count.
///
/// # Cache Index Calculation
/// ```ignore
/// cache_idx = thresh_idx * num_min_durations + min_dur_idx
/// // Example: thresh_idx=2, min_dur_idx=5, num_min_durations=15
/// // cache_idx = 2 * 15 + 5 = 35
/// ```
///
/// # Example
/// ```ignore
/// let cache = build_silence_cache(&samples, 44100);
/// let expected = vec![182, 243, 191];  // 3 tracks
/// let result = run_stage2_single_edition_cached(
///     &cache,
///     12,  // num_thresholds
///     15,  // num_min_durations
///     &expected,
///     "550e8400-e29b-41d4-a716-446655440000",
///     3.0,  // tolerance
///     0.0   // current_best_percentage
/// );
/// if let Some(best) = result.best_result {
///     println!("Best: {:.1}% with {}dB, {}s",
///         best.percentage,
///         result.best_threshold.unwrap(),
///         result.best_min_duration.unwrap()
///     );
/// }
/// ```
///
/// # Requirements
/// TEST-FUNC-002a: STAGE2_THRESHOLD_VALUES ordering preserved (via constants.rs)
/// TEST-FUNC-002b: STAGE2_MIN_DURATION_VALUES ordering preserved (via constants.rs)
/// TEST-FUNC-004: Early-exit when 100% match found
pub(crate) fn run_stage2_single_edition_cached(
    silence_cache: &SilenceCache,
    num_thresholds: usize,
    num_min_durations: usize,
    expected_durations: &[u32],
    edition_id: &str,
    tolerance: f64,
    current_best_percentage: f64,
) -> SingleEditionStage2Results {
    // Early-exit if already have 100% match from previous edition/stage
    if current_best_percentage >= 100.0 {
        return SingleEditionStage2Results {
            best_result: None,
            over_segmented_candidates: Vec::new(),
            best_threshold: None,
            best_min_duration: None,
        };
    }

    let mut best_result: Option<CandidateTestResult> = None;
    let mut over_segmented_candidates: Vec<OverSegmentedCandidate> = Vec::new();
    let mut best_threshold: Option<f64> = None;
    let mut best_min_duration: Option<f64> = None;

    let expected_track_count = expected_durations.len();

    // Iterate using indices to look up from cache
    for thresh_idx in 0..num_thresholds {
        for min_dur_idx in 0..num_min_durations {
            // Look up pre-computed track durations from cache
            let cache_idx = thresh_idx * num_min_durations + min_dur_idx;
            let test_durations = match silence_cache.get(cache_idx) {
                Some(durations) => durations,
                None => continue, // Should not happen with valid indices
            };

            // Get actual threshold/duration values for logging and over-segmented candidates
            let thresh = STAGE2_THRESHOLD_VALUES[thresh_idx];
            let min_dur = STAGE2_MIN_DURATION_VALUES[min_dur_idx];

            // Test against this single edition
            let result = test_segmentation_against_single_edition(
                test_durations,
                expected_durations,
                edition_id,
                tolerance,
            );

            let improved = best_result.as_ref().map_or(true, |br| result.percentage > br.percentage);

            if improved && result.percentage > current_best_percentage {
                // Suppress per-combination logging in cached version to reduce output noise
                // (180 combinations × 23 editions = too much output)

                best_threshold = Some(thresh);
                best_min_duration = Some(min_dur);
                best_result = Some(result);

                // Run 28: Early-exit optimization when 100% match found
                // Combined with reordered STAGE2 arrays (most likely parameters tested first),
                // this achieves early-exit for 87.6% of albums (169 of 193 successful in Run 27)
                // Expected benefit: ~0.35s savings per album (mean exit rank 10.7 vs 180 params)
                // Median exit rank 4.0 means 57.4% of perfect matches exit in first 5 attempts
                if best_result.as_ref().unwrap().percentage >= 100.0 {
                    return SingleEditionStage2Results {
                        best_result,
                        over_segmented_candidates,
                        best_threshold,
                        best_min_duration,
                    };
                }
            }

            // Collect over-segmented candidates for Stage 3 assembly
            if test_durations.len() > expected_track_count {
                over_segmented_candidates.push(OverSegmentedCandidate {
                    durations: test_durations.clone(),
                    threshold_db: thresh,
                    min_duration_secs: min_dur,
                    track_count: test_durations.len(),
                });
            }
        }
    }

    SingleEditionStage2Results {
        best_result,
        over_segmented_candidates,
        best_threshold,
        best_min_duration,
    }
}
