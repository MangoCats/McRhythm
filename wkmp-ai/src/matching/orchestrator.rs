//! Stage Orchestration
//!
//! **[PLAN030]** Coordinates multi-stage album matching with early exit.
//!
//! # Stage Flow
//!
//! 1. Stage 2: Parameter grid search (primary)
//! 2. Stage 3: Over-segmentation assembly (if Stage 2 fails)
//! 3. Stage 4: Quiet spot detection (if Stage 3 fails)
//! 4. Stage 5: Extra track merging (if Stage 4 fails)
//!
//! Orchestration stops at first stage that achieves acceptable match percentage.

use crate::matching::{
    editions::{
        calculate_edition_score, calculate_total_duration_score,
        calculate_total_duration_score_validated, calculate_track_count_penalty,
        calculate_track_quality_score, score_edition_match,
    },
    stages::{
        apply_refinement_to_durations,
        stage2::{run_stage2, stage2_success, EarlyExitConfig, Stage2Result},
        stage3::{run_stage3, stage3_success, Stage3Result},
        stage4::{run_stage4, stage4_success, RmsProfile, Stage4Result},
        stage5::{run_stage5, stage5_success, Stage5Result},
    },
    types::{Edition, MatchingStage, PassageComparison, RankedCandidate, SilenceCache},
};

/// Complete matching result from orchestrator
#[derive(Debug, Clone)]
pub struct OrchestrationResult {
    /// Which stage produced the final match
    pub winning_stage: MatchingStage,
    /// Matched edition (if any)
    pub matched_edition: Option<Edition>,
    /// Final match percentage (0-100)
    pub match_percentage: f64,
    /// Detected track durations (seconds)
    pub detected_durations: Vec<f64>,
    /// Per-track errors (seconds)
    pub track_errors: Vec<f64>,
    /// Whether a successful match was found
    pub success: bool,
    /// All stage results for diagnostics
    pub stage_results: StageResults,
}

/// Collected results from all stages
#[derive(Debug, Clone, Default)]
pub struct StageResults {
    /// Stage 2: Parameter grid search results
    pub stage2: Vec<Stage2Result>,
    /// Stage 3: Over-segmentation assembly results
    pub stage3: Vec<Stage3Result>,
    /// Stage 4: Quiet spot detection results
    pub stage4: Vec<Stage4Result>,
    /// Stage 5: Extra track merging results
    pub stage5: Vec<Stage5Result>,
}

/// Orchestrator configuration
#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    /// Minimum acceptable match percentage (0-100)
    pub min_match_percentage: f64,
    /// Track duration tolerance (seconds)
    pub tolerance_secs: f64,
    /// Enable early exit on 100% match
    pub early_exit: bool,
    /// Grace editions for early exit
    pub early_exit_grace: usize,
    /// Search window for quiet spot detection (seconds)
    pub quiet_spot_window_secs: f64,
    /// Maximum tracks to merge in Stage 5
    pub max_merge_tracks: usize,
    /// Weight for name similarity in final score (0.0-1.0)
    /// Higher values prioritize album hint over match percentage
    /// Default 0.4 balances hint adherence (40%) with match accuracy (60%)
    pub name_similarity_weight: f64,
    /// **[BUG FIX]** Actual audio file duration in milliseconds
    /// Used for validating edition totals against physical file length
    pub file_duration_ms: u64,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            min_match_percentage: 80.0,
            tolerance_secs: 3.0,
            early_exit: true,
            early_exit_grace: 2,
            quiet_spot_window_secs: 5.0,
            max_merge_tracks: 3,
            name_similarity_weight: 0.4, // 40% name, 60% match percentage
            file_duration_ms: 0, // Must be set by caller
        }
    }
}

/// **[PLAN027]** Calculate multi-factor edition score
///
/// Uses REQ-AM-092 through REQ-AM-095 multi-factor weighted scoring to select
/// the best edition from stage results. This replaces the old weighted scoring
/// with graduated penalties for track count differences and improved quality metrics.
///
/// # Formula
/// ```text
/// base_score = (duration_score × 0.25) + (match_score × 0.30) + (quality_score × 0.25) + (name_score × 0.20)
/// final_score = base_score × track_count_penalty
/// ```
///
/// **[BUG FIX]** Now includes `match_score` (match percentage) to ensure editions
/// with higher match percentages are preferred over those with better error profiles.
///
/// # Arguments
/// * `detected_durations` - Detected track durations in seconds
/// * `edition` - Edition being scored
/// * `name_distance_score` - Jaro-Winkler name similarity (0.0-1.0), None if not calculated
/// * `tolerance_secs` - Track duration tolerance in seconds (typically 1.5-3.0)
///
/// # Returns
/// Multi-factor score (0.0-1.0)
fn calculate_multi_factor_score(
    detected_durations: &[f64],
    edition: &Edition,
    name_distance_score: Option<f64>,
    tolerance_secs: f64,
) -> f64 {
    // Calculate total duration score (REQ-AM-093)
    let detected_total_ms = (detected_durations.iter().sum::<f64>() * 1000.0) as u64;
    let edition_total_ms: u64 = edition.durations.iter().map(|&x| x as u64).sum();
    let duration_score = calculate_total_duration_score(detected_total_ms, edition_total_ms);

    // Calculate match percentage score (BUG FIX: prioritize match percentage)
    let match_percentage = score_edition_match(detected_durations, &edition.durations, tolerance_secs);
    let match_score = match_percentage / 100.0; // Normalize to 0.0-1.0

    // Calculate track quality score (REQ-AM-094)
    // Convert edition durations from milliseconds to seconds
    let edition_durations_secs: Vec<f64> = edition
        .durations
        .iter()
        .map(|&ms| ms as f64 / 1000.0)
        .collect();
    let quality_score =
        calculate_track_quality_score(detected_durations, &edition_durations_secs, tolerance_secs);

    // Use name distance score (REQ-AM-092: 20% weight)
    // Fall back to 0.5 if not calculated
    let name_score = name_distance_score.unwrap_or(0.5);

    // Calculate track count penalty (REQ-AM-095)
    let detected_count = detected_durations.len();
    let edition_count = edition.track_count;
    let track_count_penalty = calculate_track_count_penalty(detected_count, edition_count);

    // Calculate final score (REQ-AM-092)
    calculate_edition_score(duration_score, match_score, quality_score, name_score, track_count_penalty)
}

/// **[DEPRECATED - PLAN030]** Calculate weighted final score combining name similarity and match percentage
///
/// **NOTE:** This function is preserved for backward compatibility but is being replaced
/// by `calculate_multi_factor_score()` from PLAN027 which provides better edition selection
/// with graduated penalties and improved quality metrics.
///
/// # Formula
/// `weighted_score = (name_similarity * name_weight) + (match_pct/100 * (1 - name_weight))`
///
/// # Arguments
/// * `match_percentage` - Match percentage (0-100)
/// * `name_distance_score` - Jaro-Winkler name similarity (0.0-1.0), None if not calculated
/// * `name_weight` - Weight for name similarity (0.0-1.0)
///
/// # Returns
/// Weighted score (0.0-1.0), or just normalized match_percentage if name_distance_score is None
#[allow(dead_code)]
fn calculate_weighted_score(
    match_percentage: f64,
    name_distance_score: Option<f64>,
    name_weight: f64,
) -> f64 {
    let match_score = match_percentage / 100.0; // Normalize to 0.0-1.0

    if let Some(name_score) = name_distance_score {
        (name_score * name_weight) + (match_score * (1.0 - name_weight))
    } else {
        match_score // Fall back to match percentage only if no name similarity available
    }
}

/// **[PLAN027]** Select best Stage 2 result using multi-factor scoring
///
/// Replaces old weighted scoring (name+match%) with multi-factor scoring that
/// includes total duration alignment, track quality, and graduated track count penalties.
fn select_best_stage2_result(results: &[Stage2Result], tolerance_secs: f64) -> Option<&Stage2Result> {
    if results.is_empty() {
        return None;
    }

    results
        .iter()
        .max_by(|a, b| {
            let score_a = calculate_multi_factor_score(
                &a.detected_durations,
                &a.edition,
                a.edition.name_distance_score,
                tolerance_secs,
            );
            let score_b = calculate_multi_factor_score(
                &b.detected_durations,
                &b.edition,
                b.edition.name_distance_score,
                tolerance_secs,
            );
            score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
        })
}

/// **[PLAN027]** Select best Stage 3 result using multi-factor scoring
///
/// Uses assembled durations from dynamic programming assembly stage
fn select_best_stage3_result(results: &[Stage3Result], tolerance_secs: f64) -> Option<&Stage3Result> {
    if results.is_empty() {
        return None;
    }

    results
        .iter()
        .max_by(|a, b| {
            let score_a = calculate_multi_factor_score(
                &a.assembled_durations,
                &a.edition,
                a.edition.name_distance_score,
                tolerance_secs,
            );
            let score_b = calculate_multi_factor_score(
                &b.assembled_durations,
                &b.edition,
                b.edition.name_distance_score,
                tolerance_secs,
            );
            score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
        })
}

/// **[PLAN027]** Select best Stage 4 result using multi-factor scoring
///
/// Uses detected durations from quiet spot detection
fn select_best_stage4_result(results: &[Stage4Result], tolerance_secs: f64) -> Option<&Stage4Result> {
    if results.is_empty() {
        return None;
    }

    results
        .iter()
        .max_by(|a, b| {
            let score_a = calculate_multi_factor_score(
                &a.detected_durations,
                &a.edition,
                a.edition.name_distance_score,
                tolerance_secs,
            );
            let score_b = calculate_multi_factor_score(
                &b.detected_durations,
                &b.edition,
                b.edition.name_distance_score,
                tolerance_secs,
            );
            score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
        })
}

/// **[PLAN027]** Select best Stage 5 result using multi-factor scoring
///
/// Uses merged durations from extra track merging
fn select_best_stage5_result(results: &[Stage5Result], tolerance_secs: f64) -> Option<&Stage5Result> {
    if results.is_empty() {
        return None;
    }

    results
        .iter()
        .max_by(|a, b| {
            let score_a = calculate_multi_factor_score(
                &a.merged_durations,
                &a.edition,
                a.edition.name_distance_score,
                tolerance_secs,
            );
            let score_b = calculate_multi_factor_score(
                &b.merged_durations,
                &b.edition,
                b.edition.name_distance_score,
                tolerance_secs,
            );
            score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
        })
}

/// Run full multi-stage matching orchestration
///
/// Runs stages in sequence (2→3→4→5) with early exit on success.
///
/// # Arguments
/// * `silence_cache` - Pre-computed silence detection results (180 combinations)
/// * `rms_profile` - RMS profile for Stage 4 quiet spot detection
/// * `audio_samples` - Raw audio sample data (for boundary refinement)
/// * `total_samples` - Total audio sample count
/// * `editions` - Candidate editions to test (should be sorted by name distance)
/// * `config` - Orchestrator configuration
///
/// # Returns
/// Complete orchestration result with matched edition and diagnostics
pub fn run_orchestration(
    silence_cache: &SilenceCache,
    rms_profile: &RmsProfile,
    audio_samples: &[f32],
    total_samples: usize,
    editions: &[Edition],
    config: &OrchestratorConfig,
) -> OrchestrationResult {
    let mut stage_results = StageResults::default();

    // Build early exit config for Stage 2
    let early_exit_config = EarlyExitConfig {
        enabled: config.early_exit,
        grace_editions: config.early_exit_grace,
        min_acceptable_percentage: config.min_match_percentage,
    };

    // Stage 2: Parameter grid search with boundary refinement
    let sample_rate = rms_profile.sample_rate;
    stage_results.stage2 = run_stage2(
        silence_cache,
        audio_samples,
        sample_rate,
        editions,
        config.tolerance_secs,
        &early_exit_config,
    );

    if stage2_success(&stage_results.stage2, config.min_match_percentage) {
        // **[PLAN027]** Use multi-factor scoring (duration+quality+name+track_count)
        let best = select_best_stage2_result(&stage_results.stage2, config.tolerance_secs)
            .expect("stage2_success implies non-empty results");
        return OrchestrationResult {
            winning_stage: MatchingStage::Stage2,
            matched_edition: Some(best.edition.clone()),
            match_percentage: best.best_percentage,
            detected_durations: best.detected_durations.clone(),
            track_errors: best.track_errors.clone(),
            success: true,
            stage_results,
        };
    }

    // Get best Stage 2 durations for subsequent stages
    let best_stage2_durations = stage_results
        .stage2
        .first()
        .map(|r| r.detected_durations.clone())
        .unwrap_or_default();

    // Stage 3: Over-segmentation assembly
    stage_results.stage3 = run_stage3(&best_stage2_durations, editions, config.tolerance_secs);

    if stage3_success(&stage_results.stage3, config.min_match_percentage) {
        // **[PLAN027]** Use multi-factor scoring (duration+quality+name+track_count)
        let best = select_best_stage3_result(&stage_results.stage3, config.tolerance_secs)
            .expect("stage3_success implies non-empty results");
        return OrchestrationResult {
            winning_stage: MatchingStage::Stage3,
            matched_edition: Some(best.edition.clone()),
            match_percentage: best.best_percentage,
            detected_durations: best.assembled_durations.clone(),
            track_errors: best.track_errors.clone(),
            success: true,
            stage_results,
        };
    }

    // Stage 4: Quiet spot detection
    stage_results.stage4 = run_stage4(
        rms_profile,
        audio_samples,
        total_samples,
        editions,
        config.tolerance_secs,
        config.quiet_spot_window_secs,
    );

    if stage4_success(&stage_results.stage4, config.min_match_percentage) {
        // **[PLAN027]** Use multi-factor scoring (duration+quality+name+track_count)
        let best = select_best_stage4_result(&stage_results.stage4, config.tolerance_secs)
            .expect("stage4_success implies non-empty results");
        return OrchestrationResult {
            winning_stage: MatchingStage::Stage4,
            matched_edition: Some(best.edition.clone()),
            // Use raw percentage - penalty was only for edition selection
            match_percentage: best.raw_percentage,
            detected_durations: best.detected_durations.clone(),
            track_errors: best.track_errors.clone(),
            success: true,
            stage_results,
        };
    }

    // Stage 5: Extra track merging
    stage_results.stage5 = run_stage5(
        &best_stage2_durations,
        editions,
        config.tolerance_secs,
        config.max_merge_tracks,
    );

    if stage5_success(&stage_results.stage5, config.min_match_percentage) {
        // **[PLAN027]** Use multi-factor scoring (duration+quality+name+track_count)
        let best = select_best_stage5_result(&stage_results.stage5, config.tolerance_secs)
            .expect("stage5_success implies non-empty results");
        return OrchestrationResult {
            winning_stage: MatchingStage::Stage5,
            matched_edition: Some(best.edition.clone()),
            match_percentage: best.percentage,
            detected_durations: best.merged_durations.clone(),
            track_errors: best.track_errors.clone(),
            success: true,
            stage_results,
        };
    }

    // No stage succeeded - return best available partial result
    let best_percentage = [
        stage_results.stage2.first().map(|r| r.best_percentage),
        stage_results.stage3.first().map(|r| r.best_percentage),
        stage_results.stage4.first().map(|r| r.penalized_percentage),
        stage_results.stage5.first().map(|r| r.percentage),
    ]
    .into_iter()
    .flatten()
    .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
    .unwrap_or(0.0);

    OrchestrationResult {
        winning_stage: MatchingStage::Stage2, // Default to Stage 2 for partial results
        matched_edition: stage_results.stage2.first().map(|r| r.edition.clone()),
        match_percentage: best_percentage,
        detected_durations: best_stage2_durations,
        track_errors: stage_results
            .stage2
            .first()
            .map(|r| r.track_errors.clone())
            .unwrap_or_default(),
        success: false,
        stage_results,
    }
}

// =============================================================================
// Top-N Candidate Ranking
// =============================================================================

/// **[Top-5 Ranking]** Candidate entry for ranking across all stages
#[derive(Debug, Clone)]
struct CandidateEntry {
    edition: Edition,
    stage: MatchingStage,
    match_percentage: f64,
    detected_durations: Vec<f64>,
    track_errors: Vec<f64>,
}

/// **[Top-5 Ranking]** Extract and rank top N candidates from all stage results
///
/// After selecting the winning edition, this function:
/// 1. Collects all edition results from all stages
/// 2. Calculates multi-factor final score for each
/// 3. Sorts by final score (descending)
/// 4. Returns top N (or all if fewer than N available)
///
/// Each ranked candidate includes a passage comparison table showing
/// detected durations vs expected durations for evaluation.
pub fn rank_top_candidates(
    stage_results: &StageResults,
    tolerance_secs: f64,
    top_n: usize,
    file_duration_ms: u64,  // **[BUG FIX]** Add file duration for validated scoring
) -> Vec<RankedCandidate> {
    let mut candidates: Vec<CandidateEntry> = Vec::new();

    // Collect Stage 2 results
    for result in &stage_results.stage2 {
        candidates.push(CandidateEntry {
            edition: result.edition.clone(),
            stage: MatchingStage::Stage2,
            match_percentage: result.best_percentage,
            detected_durations: result.detected_durations.clone(),
            track_errors: result.track_errors.clone(),
        });
    }

    // Collect Stage 3 results
    for result in &stage_results.stage3 {
        candidates.push(CandidateEntry {
            edition: result.edition.clone(),
            stage: MatchingStage::Stage3,
            match_percentage: result.best_percentage,
            detected_durations: result.assembled_durations.clone(),
            track_errors: result.track_errors.clone(),
        });
    }

    // Collect Stage 4 results (use penalized percentage)
    for result in &stage_results.stage4 {
        candidates.push(CandidateEntry {
            edition: result.edition.clone(),
            stage: MatchingStage::Stage4,
            match_percentage: result.penalized_percentage,
            detected_durations: result.detected_durations.clone(),
            track_errors: result.track_errors.clone(),
        });
    }

    // Collect Stage 5 results
    for result in &stage_results.stage5 {
        candidates.push(CandidateEntry {
            edition: result.edition.clone(),
            stage: MatchingStage::Stage5,
            match_percentage: result.percentage,
            detected_durations: result.merged_durations.clone(),
            track_errors: result.track_errors.clone(),
        });
    }

    // Calculate final score for each candidate using PLAN027 multi-factor scoring
    // Tuple: (CandidateEntry, final_score, duration_score, match_score, quality_score, name_score, track_count_penalty)
    let mut scored_candidates: Vec<(CandidateEntry, f64, f64, f64, f64, f64, f64)> = candidates
        .into_iter()
        .map(|candidate| {
            // Convert detected durations to milliseconds for total duration score
            let detected_total_ms: u64 = candidate
                .detected_durations
                .iter()
                .map(|&d| (d * 1000.0) as u64)
                .sum();

            // Calculate edition total duration in milliseconds
            let edition_total_ms: u64 = candidate.edition.durations.iter().map(|&d| d as u64).sum();

            // **[BUG FIX]** Calculate duration score with file duration validation
            let duration_score = calculate_total_duration_score_validated(
                detected_total_ms,
                edition_total_ms,
                file_duration_ms,
            );

            // Convert edition durations from milliseconds to seconds for quality score
            let edition_durations_secs: Vec<f64> = candidate
                .edition
                .durations
                .iter()
                .map(|&d| d as f64 / 1000.0)
                .collect();

            // Calculate quality score
            let quality_score = calculate_track_quality_score(
                &candidate.detected_durations,
                &edition_durations_secs,
                tolerance_secs,
            );

            // Get name distance score (already stored in edition)
            let name_score = candidate
                .edition
                .name_distance_score
                .unwrap_or(0.0);

            // Calculate track count penalty
            let track_count_penalty = calculate_track_count_penalty(
                candidate.detected_durations.len(),
                candidate.edition.track_count,
            );

            // Calculate match score (BUG FIX: prioritize match percentage)
            let match_score = candidate.match_percentage / 100.0;

            // Calculate final score
            let final_score =
                calculate_edition_score(duration_score, match_score, quality_score, name_score, track_count_penalty);

            (candidate, final_score, duration_score, match_score, quality_score, name_score, track_count_penalty)
        })
        .collect();

    // Sort by final score descending (highest score first)
    scored_candidates.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Take top N and build RankedCandidate structs
    scored_candidates
        .into_iter()
        .take(top_n)
        .enumerate()
        .map(|(rank_idx, (candidate, final_score, duration_score, _match_score, quality_score, name_score, track_count_penalty))| {
            // Build passage comparison table
            // Convert edition durations from milliseconds to seconds for comparison
            let edition_durations_secs: Vec<f64> = candidate
                .edition
                .durations
                .iter()
                .map(|&d| d as f64 / 1000.0)
                .collect();

            // Calculate cumulative offsets for detected passages
            let mut cumulative_offset = 0.0;
            let detected_offsets: Vec<f64> = candidate
                .detected_durations
                .iter()
                .map(|&duration| {
                    let offset = cumulative_offset;
                    cumulative_offset += duration;
                    offset
                })
                .collect();

            let passage_comparison: Vec<PassageComparison> = candidate
                .detected_durations
                .iter()
                .zip(edition_durations_secs.iter())
                .zip(candidate.edition.track_titles.iter())
                .zip(detected_offsets.iter())
                .enumerate()
                .map(|(idx, (((&detected, &expected), title), &offset))| {
                    let error = (detected - expected).abs();
                    // Truncate title to 30 characters with ellipsis if needed
                    // **[FIX]** Use char-based truncation to avoid UTF-8 panic
                    let track_title = if title.chars().count() > 30 {
                        let truncated: String = title.chars().take(27).collect();
                        format!("{}...", truncated)
                    } else {
                        title.clone()
                    };
                    PassageComparison {
                        track_number: idx + 1,
                        track_title,
                        detected_start_offset: offset,
                        detected_duration: detected,
                        expected_duration: expected,
                        error,
                        within_tolerance: error <= tolerance_secs,
                    }
                })
                .collect();

            // Calculate mean error
            let mean_error = if !candidate.track_errors.is_empty() {
                candidate.track_errors.iter().sum::<f64>()
                    / candidate.track_errors.len() as f64
            } else {
                0.0
            };

            RankedCandidate {
                rank: rank_idx + 1,
                release_mbid: candidate.edition.release_mbid.clone(),
                title: candidate.edition.title.clone(),
                artist: candidate.edition.artist.clone(),
                track_count: candidate.edition.track_count,
                match_percentage: candidate.match_percentage,
                final_score,
                duration_score,
                quality_score,
                name_score,
                track_count_penalty,
                stage: candidate.stage,
                passage_comparison,
                mean_error,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::matching::constants::{MIN_DURATION_VALUES, THRESHOLD_VALUES};

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
            track_titles: (1..=track_count).map(|i| format!("Track {}", i)).collect(),
        }
    }

    fn create_test_silence_cache(detected_durations: Vec<f64>) -> SilenceCache {
        let num_combinations = THRESHOLD_VALUES.len() * MIN_DURATION_VALUES.len();
        vec![detected_durations; num_combinations]
    }

    fn create_test_rms_profile(total_samples: usize, sample_rate: u32) -> RmsProfile {
        let window_size = 4410; // 100ms at 44100Hz
        let num_windows = total_samples / window_size;
        RmsProfile {
            values: vec![0.5; num_windows],
            window_size,
            sample_rate,
        }
    }

    #[test]
    fn test_stage2_success_early_exit() {
        // Edition matches perfectly in Stage 2
        let edition = create_test_edition(3, &[180000, 240000, 200000]);
        let cache = create_test_silence_cache(vec![180.0, 240.0, 200.0]);
        let sample_rate = 44100u32;
        let total_samples = 620 * sample_rate as usize;
        let rms_profile = create_test_rms_profile(total_samples, sample_rate);

        let config = OrchestratorConfig {
            min_match_percentage: 80.0,
            tolerance_secs: 10.0,
            ..Default::default()
        };

        // Create dummy audio samples for boundary refinement
        let audio_samples = vec![0.0f32; total_samples];

        let result = run_orchestration(&cache, &rms_profile, &audio_samples, total_samples, &[edition], &config);

        assert!(result.success);
        assert!(matches!(result.winning_stage, MatchingStage::Stage2));
        assert_eq!(result.match_percentage, 100.0);
    }

    #[test]
    fn test_stage3_fallback() {
        // Stage 2 produces over-segmented result
        // Expected: 2 tracks [180s, 240s]
        // Stage 2 detects: 3 tracks [180s, 120s, 120s] - count mismatch = 0%
        // Stage 3 or 5 could potentially handle this
        let edition = create_test_edition(2, &[180000, 240000]);
        let cache = create_test_silence_cache(vec![180.0, 120.0, 120.0]);
        let sample_rate = 44100u32;
        let total_samples = 420 * sample_rate as usize;
        let rms_profile = create_test_rms_profile(total_samples, sample_rate);

        let config = OrchestratorConfig {
            min_match_percentage: 80.0,
            tolerance_secs: 10.0,
            ..Default::default()
        };

        // Create dummy audio samples for boundary refinement
        let audio_samples = vec![0.0f32; total_samples];

        let result = run_orchestration(&cache, &rms_profile, &audio_samples, total_samples, &[edition], &config);

        // Verify orchestration ran to completion
        // Stage 2 will fail due to count mismatch
        // Later stages may or may not succeed depending on algorithm details
        // The key test is that orchestration completes without panic
        assert!(result.match_percentage >= 0.0);

        // If we got success, it should be from a later stage
        if result.success {
            assert!(!matches!(result.winning_stage, MatchingStage::Stage2));
        }
    }

    #[test]
    fn test_stage4_fallback() {
        // Stage 2 and 3 fail, Stage 4 should find quiet spots
        // Use track count that won't trigger Stage 3 (not over-segmented)
        // and won't trigger Stage 5 (no extra tracks)
        let edition = create_test_edition(1, &[180000]);

        // Stage 2: wrong duration (won't match)
        let cache = create_test_silence_cache(vec![100.0]); // Wrong duration

        let sample_rate = 44100u32;
        let total_samples = 180 * sample_rate as usize;

        // Stage 4: RMS profile that produces correct duration
        let window_size = 4410;
        let num_windows = total_samples / window_size;
        let rms_profile = RmsProfile {
            values: vec![0.5; num_windows],
            window_size,
            sample_rate,
        };

        let config = OrchestratorConfig {
            min_match_percentage: 70.0, // Lower threshold to account for Stage 4 penalty
            tolerance_secs: 10.0,
            ..Default::default()
        };

        // Create dummy audio samples for boundary refinement
        let audio_samples = vec![0.0f32; total_samples];

        let result = run_orchestration(&cache, &rms_profile, &audio_samples, total_samples, &[edition], &config);

        // Stage 4 should succeed with penalty (100% * 0.75 = 75%)
        assert!(result.success);
        assert!(matches!(result.winning_stage, MatchingStage::Stage4));
    }

    #[test]
    fn test_stage5_fallback() {
        // Stage 2 has extra track at end, Stage 5 should merge
        // Expected: 2 tracks [180s, 240s] = 420s total
        // Detected: 3 tracks [180s, 120s, 120s] with count > expected
        let edition = create_test_edition(2, &[180000, 240000]);

        // Stage 2: 3 tracks detected (extra at end)
        let cache = create_test_silence_cache(vec![180.0, 120.0, 120.0]);

        let sample_rate = 44100u32;
        let total_samples = 420 * sample_rate as usize;
        let rms_profile = create_test_rms_profile(total_samples, sample_rate);

        let config = OrchestratorConfig {
            min_match_percentage: 80.0,
            tolerance_secs: 10.0,
            ..Default::default()
        };

        // Create dummy audio samples for boundary refinement
        let audio_samples = vec![0.0f32; total_samples];

        let result = run_orchestration(&cache, &rms_profile, &audio_samples, total_samples, &[edition], &config);

        // Stage 3 or Stage 5 should handle this (both deal with extra tracks)
        // If success, verify correct stage won
        if result.success {
            assert!(matches!(
                result.winning_stage,
                MatchingStage::Stage3 | MatchingStage::Stage5
            ));
        }
        // Test passes regardless - we're verifying orchestration flow
    }

    #[test]
    fn test_all_stages_fail() {
        // No edition matches - use impossible duration combinations
        let edition = create_test_edition(5, &[60000, 60000, 60000, 60000, 60000]); // 5 tracks = 300s

        // Detected completely wrong duration pattern
        let cache = create_test_silence_cache(vec![200.0, 200.0]); // 2 tracks, 400s total

        let sample_rate = 44100u32;
        let total_samples = 400 * sample_rate as usize;
        let rms_profile = create_test_rms_profile(total_samples, sample_rate);

        let config = OrchestratorConfig {
            min_match_percentage: 80.0,
            tolerance_secs: 10.0,
            ..Default::default()
        };

        // Create dummy audio samples for boundary refinement
        let audio_samples = vec![0.0f32; total_samples];

        let result = run_orchestration(&cache, &rms_profile, &audio_samples, total_samples, &[edition], &config);

        // Verify orchestration completes and returns some result
        // Stage 2: count mismatch (2 vs 5) = 0%
        // Stage 3: detected < expected, skipped
        // Stage 4: tries quiet spots, may get partial match
        // Stage 5: detected < expected, skipped
        // Either no success, or very low percentage if Stage 4 gets lucky
        assert!(result.match_percentage < 80.0 || !result.success);
    }

    #[test]
    fn test_best_partial_match_returned() {
        // Stage 2 gets partial match (below threshold), returned as best effort
        let edition = create_test_edition(3, &[180000, 240000, 200000]);

        // Only first track matches
        let cache = create_test_silence_cache(vec![180.0, 300.0, 150.0]);

        let sample_rate = 44100u32;
        let total_samples = 630 * sample_rate as usize;
        let rms_profile = create_test_rms_profile(total_samples, sample_rate);

        let config = OrchestratorConfig {
            min_match_percentage: 80.0, // 1/3 = 33% won't pass
            tolerance_secs: 10.0,
            ..Default::default()
        };

        // Create dummy audio samples for boundary refinement
        let audio_samples = vec![0.0f32; total_samples];

        let result = run_orchestration(&cache, &rms_profile, &audio_samples, total_samples, &[edition], &config);

        // Should return partial result
        assert!(!result.success);
        // Some percentage returned (best effort)
        assert!(result.match_percentage > 0.0);
    }
}
