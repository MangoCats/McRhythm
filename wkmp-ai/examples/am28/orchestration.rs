//! # Edition Testing Orchestration
//!
//! High-level orchestration for testing editions through all stages (2-5).
//!
//! ## Features
//! - **Stage integration**: Coordinates Stages 2-5 in sequence
//! - **Early-exit optimization**: Stops processing when 100% match found (with grace period)
//! - **Best result tracking**: Maintains best match across all stages for each edition
//! - **Stage 4 penalty**: Applies 25% penalty to quiet spot detection results
//! - **Parallel edition testing**: Designed to run in parallel across editions
//!
//! ## Stage Flow
//! For each edition:
//! 1. **Stage 2**: Parameter grid search (180 combinations) with early-exit
//! 2. **Stage 3**: Dynamic programming assembly (if over-segmented candidates found)
//! 3. **Stage 4**: Edition-guided quiet spot detection (25% penalty)
//! 4. **Stage 5**: Extra track merging (if 100% match + extra tracks)
//!
//! After each stage, check for:
//! - 100% match → signal perfect match, return immediately
//! - Early-exit condition → stop processing this edition
//!
//! ## Related Modules
//! - `stages::stage2`: Parameter optimization
//! - `stages::stage3`: Over-segmentation assembly
//! - `stages::stage4`: Quiet spot detection
//! - `stages::stage5`: Extra track merging
//! - `utils::early_exit`: Early-exit coordination

use crate::constants::STAGE4_PENALTY_PERCENT;
use crate::stages::{
    run_stage2_single_edition_cached, run_stage3_single_edition, run_stage4_single_edition,
    run_stage5_single_edition,
};
use crate::types::SilenceCache;
use crate::types::{CandidateTestResult, Edition, EditionTestResult};
use crate::utils::{should_exit_early, signal_perfect_match};
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::time::Instant;

// =============================================================================
// Edition Result Construction
// =============================================================================

/// Construct EditionTestResult with all metadata
///
/// Helper to create EditionTestResult consistently across all exit points
/// in test_single_edition().
///
/// # Arguments
/// * `edition_idx` - Zero-based edition index
/// * `best_percentage` - Best match percentage achieved (0-100)
/// * `best_result` - Best CandidateTestResult found (None if no match)
/// * `best_stage` - Name of stage that produced best result
/// * `best_threshold` - Best silence threshold (dB) if applicable
/// * `best_min_duration` - Best min duration (s) if applicable
/// * `expected_durations` - Expected track durations from MusicBrainz
/// * `log_messages` - Accumulated log messages for this edition
///
/// # Returns
/// EditionTestResult with all fields populated
pub(crate) fn make_edition_result(
    edition_idx: usize,
    best_percentage: f64,
    best_result: Option<CandidateTestResult>,
    best_stage: &'static str,
    best_threshold: Option<f64>,
    best_min_duration: Option<f64>,
    expected_durations: &[u32],
    log_messages: Vec<String>,
) -> EditionTestResult {
    EditionTestResult {
        edition_idx,
        best_percentage,
        best_result,
        best_stage,
        best_threshold,
        best_min_duration,
        expected_durations: expected_durations.to_vec(),
        log_messages,
    }
}

// =============================================================================
// Edition Testing - Stage Integration
// =============================================================================

/// Test a single edition through all stages (Stages 2-5)
///
/// Main orchestration function that runs all matching stages sequentially for
/// one edition. Designed to be called in parallel across multiple editions.
///
/// # Arguments
/// * `edition_idx` - Zero-based edition index (for logging/results)
/// * `edition` - Edition metadata (artist, album, track durations, etc.)
/// * `silence_cache` - Pre-computed silence detection for all parameter combinations
/// * `num_thresholds` - Number of threshold values to test (12)
/// * `num_min_durations` - Number of min_duration values to test (15)
/// * `match_tolerance_secs` - Tolerance for track matching (seconds)
/// * `rms_profile` - Pre-computed RMS profile for quiet spot detection
/// * `total_duration_secs` - Total audio file duration (seconds)
/// * `initial_durations` - Initial track durations (from default parameters)
/// * `total_editions` - Total number of editions being tested (for logging)
/// * `perfect_match_found` - Atomic flag set when any edition achieves 100%
/// * `perfect_match_time_ms` - Timestamp when perfect match was found
/// * `start_time` - Processing start time (for grace period calculation)
/// * `album_idx` - Zero-based album index (for logging)
///
/// # Returns
/// EditionTestResult with best match found across all stages
///
/// # Stage Flow
/// 1. **Stage 2**: Parameter grid search
///    - Test 180 combinations (12 thresholds × 15 min_durations)
///    - Early-exit on 100% match (87.6% benefit rate)
///    - Collect over-segmented candidates for Stage 3
///    - If 100% → signal perfect match and return
///
/// 2. **Stage 3**: Over-segmentation assembly (if candidates exist)
///    - Dynamic programming optimal segment merging
///    - Check early-exit before starting
///    - If 100% → signal perfect match and return
///
/// 3. **Stage 4**: Quiet spot detection
///    - Edition-guided RMS profiling
///    - **25% penalty applied to results** (less reliable)
///    - Cannot trigger early-exit due to penalty
///    - Check early-exit before starting
///
/// 4. **Stage 5**: Extra track merging (if conditions met)
///    - Only if: current_best >= 100% AND detected > expected
///    - Merges adjacent tracks to match expected count
///    - Maintains 100% match quality
///
/// # Early-Exit Coordination
/// - Checks `should_exit_early()` before Stages 3 and 4
/// - If grace period expired, skip remaining stages
/// - Each 100% match signals `perfect_match_found` flag
/// - Grace period: 20 seconds after first 100% match
///
/// # Example
/// ```ignore
/// let result = test_single_edition(
///     edition_idx,
///     &edition,
///     &silence_cache,
///     12,  // num_thresholds
///     15,  // num_min_durations
///     3.0,  // match_tolerance_secs
///     &rms_profile,
///     total_duration_secs,
///     &initial_durations,
///     total_editions,
///     &perfect_match_found,
///     &perfect_match_time_ms,
///     start_time,
///     album_idx,
/// );
/// println!("Best: {:.1}% via {}", result.best_percentage, result.best_stage);
/// ```
pub(crate) fn test_single_edition(
    edition_idx: usize,
    edition: &Edition,
    silence_cache: &SilenceCache,
    num_thresholds: usize,
    num_min_durations: usize,
    match_tolerance_secs: f64,
    rms_profile: &[(f64, f32)],
    total_duration_secs: f64,
    initial_durations: &[f64],
    total_editions: usize,
    perfect_match_found: &AtomicBool,
    perfect_match_time_ms: &AtomicU64,
    start_time: Instant,
    album_idx: usize,
) -> EditionTestResult {
    let edition_id = format!("edition_{}", edition_idx);
    let expected_durations = &edition.durations;
    let mut log_messages = Vec::new();

    log_messages.push(format!(
        "  [Edition {}/{}] {} - {} ({} tracks, NDR:{},{:.1})",
        edition_idx + 1,
        total_editions,
        edition.artist,
        edition.album,
        edition.track_count,
        edition.name_distance_rank,
        edition.name_distance_score
    ));

    // Check for early exit before starting (grace period expired)
    if should_exit_early(perfect_match_found, perfect_match_time_ms, start_time) {
        log_messages.push("    -> Skipped (early exit: grace period expired)".to_string());
        return make_edition_result(
            edition_idx,
            0.0,
            None,
            "skipped_early_exit",
            None,
            None,
            expected_durations,
            log_messages,
        );
    }

    // Track best result for THIS edition across stages
    let mut edition_best_percentage = 0.0;
    let mut edition_best_durations = initial_durations.to_vec();
    let mut edition_best_stage: &'static str = "none";
    let mut edition_best_result: Option<CandidateTestResult> = None;
    let mut edition_best_threshold: Option<f64> = None;
    let mut edition_best_min_duration: Option<f64> = None;

    // === STAGE 2: Parameter optimization for this edition ===
    log_messages.push(format!(
        "    Stage 2: Testing {} cached parameter combinations...",
        num_thresholds * num_min_durations
    ));

    let stage2_results = run_stage2_single_edition_cached(
        silence_cache,
        num_thresholds,
        num_min_durations,
        expected_durations,
        &edition_id,
        match_tolerance_secs,
        edition_best_percentage,
    );

    if let Some(ref result) = stage2_results.best_result {
        if result.percentage > edition_best_percentage {
            edition_best_percentage = result.percentage;
            edition_best_durations = result.detected_durations.clone();
            edition_best_stage = "album_extractor_2_optimization";
            edition_best_result = Some(result.clone());
            edition_best_threshold = stage2_results.best_threshold;
            edition_best_min_duration = stage2_results.best_min_duration;

            if result.percentage >= 100.0 {
                log_messages.push("    -> 100% match in Stage 2!".to_string());
                signal_perfect_match(perfect_match_found, perfect_match_time_ms, start_time);
                return make_edition_result(
                    edition_idx,
                    edition_best_percentage,
                    edition_best_result,
                    edition_best_stage,
                    edition_best_threshold,
                    edition_best_min_duration,
                    expected_durations,
                    log_messages,
                );
            }
        }
    }

    // Check for early exit before Stage 3
    if should_exit_early(perfect_match_found, perfect_match_time_ms, start_time) {
        log_messages.push("    -> Early exit before Stage 3 (grace period expired)".to_string());
        return make_edition_result(
            edition_idx,
            edition_best_percentage,
            edition_best_result,
            edition_best_stage,
            edition_best_threshold,
            edition_best_min_duration,
            expected_durations,
            log_messages,
        );
    }

    // === STAGE 3: Segment assembly for this edition ===
    if !stage2_results.over_segmented_candidates.is_empty() {
        log_messages.push(format!(
            "    Stage 3: Testing {} over-segmented assemblies...",
            stage2_results.over_segmented_candidates.len()
        ));

        if let Some(result) = run_stage3_single_edition(
            &stage2_results.over_segmented_candidates,
            expected_durations,
            &edition_id,
            match_tolerance_secs,
            edition_best_percentage,
            edition_idx,
            total_editions,
            perfect_match_found,
            perfect_match_time_ms,
            start_time,
            album_idx,
        ) {
            if result.percentage > edition_best_percentage {
                edition_best_percentage = result.percentage;
                edition_best_durations = result.detected_durations.clone();
                edition_best_stage = "album_extractor_3_assembly";
                edition_best_result = Some(result.clone());

                if result.percentage >= 100.0 {
                    log_messages.push("    -> 100% match in Stage 3!".to_string());
                    signal_perfect_match(perfect_match_found, perfect_match_time_ms, start_time);
                    return make_edition_result(
                        edition_idx,
                        edition_best_percentage,
                        edition_best_result,
                        edition_best_stage,
                        edition_best_threshold,
                        edition_best_min_duration,
                        expected_durations,
                        log_messages,
                    );
                }
            }
        }
    }

    // Check for early exit before Stage 4
    if should_exit_early(perfect_match_found, perfect_match_time_ms, start_time) {
        log_messages.push("    -> Early exit before Stage 4 (grace period expired)".to_string());
        return make_edition_result(
            edition_idx,
            edition_best_percentage,
            edition_best_result,
            edition_best_stage,
            edition_best_threshold,
            edition_best_min_duration,
            expected_durations,
            log_messages,
        );
    }

    // === STAGE 4: Quiet spot detection for this edition ===
    // Stage 4 results incur a 25% penalty (less reliable than silence-based detection)
    // Stage 4 can NEVER trigger 100% early exit due to this penalty
    if !rms_profile.is_empty() {
        log_messages.push("    Stage 4: Guided quiet spot detection...".to_string());

        if let Some(result) = run_stage4_single_edition(
            expected_durations,
            &edition_id,
            match_tolerance_secs,
            edition_best_percentage,
            rms_profile,
            total_duration_secs,
            edition_idx,
            total_editions,
            album_idx,
        ) {
            // Apply Stage 4 penalty: raw 100% becomes 75%
            let penalized_percentage = result.percentage * (1.0 - STAGE4_PENALTY_PERCENT / 100.0);

            if penalized_percentage > edition_best_percentage {
                log_messages.push(format!(
                    "    -> Stage 4 raw: {:.1}%, penalized: {:.1}% (-{}%)",
                    result.percentage, penalized_percentage, STAGE4_PENALTY_PERCENT
                ));
                edition_best_percentage = penalized_percentage;
                edition_best_durations = result.detected_durations.clone();
                edition_best_stage = "album_extractor_4_guided";
                edition_best_result = Some(result.clone());
                // Note: No 100% early exit for Stage 4 - penalty makes it impossible
            }
        }
    }

    // === STAGE 5: Extra track merging for this edition ===
    if edition_best_percentage >= 100.0 && edition_best_durations.len() > expected_durations.len() {
        log_messages.push("    Stage 5: Extra track merging...".to_string());

        if let Some((_merged_durations, result)) = run_stage5_single_edition(
            &edition_best_durations,
            expected_durations,
            &edition_id,
            match_tolerance_secs,
            edition_best_percentage,
        ) {
            edition_best_stage = "album_extractor_5_merging";
            edition_best_result = Some(result.clone());
        }
    }

    log_messages.push(format!(
        "    -> Edition best: {:.1}% ({})",
        edition_best_percentage, edition_best_stage
    ));

    make_edition_result(
        edition_idx,
        edition_best_percentage,
        edition_best_result,
        edition_best_stage,
        edition_best_threshold,
        edition_best_min_duration,
        expected_durations,
        log_messages,
    )
}
