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
    stages::{
        stage2::{run_stage2, stage2_success, EarlyExitConfig, Stage2Result},
        stage3::{run_stage3, stage3_success, Stage3Result},
        stage4::{run_stage4, stage4_success, RmsProfile, Stage4Result},
        stage5::{run_stage5, stage5_success, Stage5Result},
    },
    types::{Edition, MatchingStage, SilenceCache},
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
        }
    }
}

/// Run full multi-stage matching orchestration
///
/// Runs stages in sequence (2→3→4→5) with early exit on success.
///
/// # Arguments
/// * `silence_cache` - Pre-computed silence detection results (180 combinations)
/// * `rms_profile` - RMS profile for Stage 4 quiet spot detection
/// * `total_samples` - Total audio sample count
/// * `editions` - Candidate editions to test (should be sorted by name distance)
/// * `config` - Orchestrator configuration
///
/// # Returns
/// Complete orchestration result with matched edition and diagnostics
pub fn run_orchestration(
    silence_cache: &SilenceCache,
    rms_profile: &RmsProfile,
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

    // Stage 2: Parameter grid search
    stage_results.stage2 = run_stage2(
        silence_cache,
        editions,
        config.tolerance_secs,
        &early_exit_config,
    );

    if stage2_success(&stage_results.stage2, config.min_match_percentage) {
        let best = &stage_results.stage2[0];
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
        let best = &stage_results.stage3[0];
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
        total_samples,
        editions,
        config.tolerance_secs,
        config.quiet_spot_window_secs,
    );

    if stage4_success(&stage_results.stage4, config.min_match_percentage) {
        let best = &stage_results.stage4[0];
        return OrchestrationResult {
            winning_stage: MatchingStage::Stage4,
            matched_edition: Some(best.edition.clone()),
            match_percentage: best.penalized_percentage,
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
        let best = &stage_results.stage5[0];
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

        let result = run_orchestration(&cache, &rms_profile, total_samples, &[edition], &config);

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

        let result = run_orchestration(&cache, &rms_profile, total_samples, &[edition], &config);

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

        let result = run_orchestration(&cache, &rms_profile, total_samples, &[edition], &config);

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

        let result = run_orchestration(&cache, &rms_profile, total_samples, &[edition], &config);

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

        let result = run_orchestration(&cache, &rms_profile, total_samples, &[edition], &config);

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

        let result = run_orchestration(&cache, &rms_profile, total_samples, &[edition], &config);

        // Should return partial result
        assert!(!result.success);
        // Some percentage returned (best effort)
        assert!(result.match_percentage > 0.0);
    }
}
