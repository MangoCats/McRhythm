# Increment 11: Stage Orchestration

**Estimated Effort:** 4 hours
**Dependencies:** Increments 7-10
**Deliverables:** matching/orchestrator.rs

---

## Objective

Implement stage orchestration logic that runs stages in sequence (2→3→4→5) with early exit on success.

---

## Source Files

| am28 File | Lines | Action |
|-----------|-------|--------|
| orchestration.rs | ~361 | Adapt to library |

---

## Tasks

### 11.1 Create matching/orchestrator.rs

```rust
//! Stage Orchestration
//!
//! Coordinates multi-stage album matching with early exit.

use crate::matching::{
    types::*,
    silence_detection::SilenceCache,
    stages::{
        stage2::{run_stage2, Stage2Result, stage2_success},
        stage3::{run_stage3, Stage3Result, stage3_success},
        stage4::{run_stage4, Stage4Result, stage4_success, RmsProfile},
        stage5::{run_stage5, Stage5Result, stage5_success},
    },
};

/// Complete matching result from orchestrator
#[derive(Debug, Clone)]
pub struct OrchestrationResult {
    /// Which stage produced the final match
    pub winning_stage: MatchingStage,
    /// Matched edition
    pub matched_edition: Option<Edition>,
    /// Final match percentage
    pub match_percentage: f64,
    /// Detected track durations
    pub detected_durations: Vec<f64>,
    /// Per-track errors
    pub track_errors: Vec<f64>,
    /// All stage results for diagnostics
    pub stage_results: StageResults,
}

/// Collected results from all stages
#[derive(Debug, Clone, Default)]
pub struct StageResults {
    pub stage2: Vec<Stage2Result>,
    pub stage3: Vec<Stage3Result>,
    pub stage4: Vec<Stage4Result>,
    pub stage5: Vec<Stage5Result>,
}

/// Orchestrator configuration
#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    /// Minimum acceptable match percentage
    pub min_match_percentage: f64,
    /// Track duration tolerance (seconds)
    pub tolerance_secs: f64,
    /// Enable early exit on 100% match
    pub early_exit: bool,
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
            quiet_spot_window_secs: 5.0,
            max_merge_tracks: 3,
        }
    }
}

/// Run full multi-stage matching
///
/// # Arguments
/// * `silence_cache` - Pre-computed silence detection results
/// * `rms_profile` - RMS profile for Stage 4
/// * `total_samples` - Total audio samples
/// * `editions` - Candidate editions to test
/// * `config` - Orchestrator configuration
///
/// # Returns
/// Complete orchestration result
pub fn run_orchestration(
    silence_cache: &SilenceCache,
    rms_profile: &RmsProfile,
    total_samples: usize,
    editions: &[Edition],
    config: &OrchestratorConfig,
) -> OrchestrationResult {
    let mut stage_results = StageResults::default();

    // Stage 2: Parameter grid search
    stage_results.stage2 = run_stage2(
        silence_cache,
        editions,
        config.tolerance_secs,
        config.early_exit,
    );

    if stage2_success(&stage_results.stage2, config.min_match_percentage) {
        let best = &stage_results.stage2[0];
        return OrchestrationResult {
            winning_stage: MatchingStage::Stage2,
            matched_edition: Some(best.edition.clone()),
            match_percentage: best.best_percentage,
            detected_durations: best.detected_durations.clone(),
            track_errors: best.track_errors.clone(),
            stage_results,
        };
    }

    // Get best Stage 2 durations for subsequent stages
    let best_stage2_durations = stage_results.stage2
        .first()
        .map(|r| r.detected_durations.clone())
        .unwrap_or_default();

    // Stage 3: Over-segmentation assembly
    stage_results.stage3 = run_stage3(
        &best_stage2_durations,
        editions,
        config.tolerance_secs,
    );

    if stage3_success(&stage_results.stage3, config.min_match_percentage) {
        let best = &stage_results.stage3[0];
        return OrchestrationResult {
            winning_stage: MatchingStage::Stage3,
            matched_edition: Some(best.edition.clone()),
            match_percentage: best.best_percentage,
            detected_durations: best.assembled_durations.clone(),
            track_errors: best.track_errors.clone(),
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
            stage_results,
        };
    }

    // No stage succeeded - return best available result
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
        winning_stage: MatchingStage::Stage2, // Default to Stage 2 for partial
        matched_edition: stage_results.stage2.first().map(|r| r.edition.clone()),
        match_percentage: best_percentage,
        detected_durations: best_stage2_durations,
        track_errors: stage_results.stage2
            .first()
            .map(|r| r.track_errors.clone())
            .unwrap_or_default(),
        stage_results,
    }
}
```

---

## Tests

| Test ID | Description | Pass Criteria |
|---------|-------------|---------------|
| TC-U-011-01 | Stage 2 success early exit | Stops at Stage 2 |
| TC-U-011-02 | Stage 3 fallback | Runs Stage 3 when Stage 2 fails |
| TC-U-011-03 | Stage 4 fallback | Runs Stage 4 when Stage 3 fails |
| TC-U-011-04 | Stage 5 fallback | Runs Stage 5 when Stage 4 fails |
| TC-U-011-05 | All stages fail | Returns best partial match |
| TC-I-011-01 | Full orchestration | Correct stage sequence |

---

## Acceptance Criteria

- [ ] orchestrator.rs created
- [ ] Stage sequence correct (2→3→4→5)
- [ ] Early exit on success working
- [ ] Best partial match returned on failure
- [ ] All 6 tests pass
