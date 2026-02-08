# Increment 7: Stage 2 - Parameter Grid Search

**Estimated Effort:** 5 hours
**Dependencies:** Increment 4, 6
**Deliverables:** matching/stages/stage2.rs

---

## Objective

Implement Stage 2: parameter grid search across 180 combinations (12 thresholds × 15 min_durations) to find best silence detection parameters for each edition.

---

## Source Files

| am28 File | Lines | Action |
|-----------|-------|--------|
| stages/stage2.rs | ~400 | Adapt to library |
| utils/early_exit.rs | ~100 | Integrate |

---

## Tasks

### 7.1 Create matching/stages/mod.rs

```rust
//! Multi-Stage Album Matching
//!
//! Stage 2: Parameter grid search (primary)
//! Stage 3: Over-segmentation assembly
//! Stage 4: Quiet spot detection
//! Stage 5: Extra track merging

pub mod stage2;
pub mod stage3;
pub mod stage4;
pub mod stage5;

pub use stage2::*;
pub use stage3::*;
pub use stage4::*;
pub use stage5::*;
```

### 7.2 Create matching/stages/stage2.rs

```rust
//! Stage 2: Parameter Grid Search
//!
//! Tests all 180 parameter combinations against each edition.
//! Uses pre-computed silence cache for efficiency.

use crate::matching::{
    constants::*,
    types::*,
    silence_detection::SilenceCache,
    editions::{score_edition_match, analyze_track_matching},
};

/// Stage 2 result for a single edition
#[derive(Debug, Clone)]
pub struct Stage2Result {
    /// Edition that was tested
    pub edition: Edition,
    /// Best match percentage achieved
    pub best_percentage: f64,
    /// Parameter index that achieved best match
    pub best_threshold_idx: usize,
    pub best_duration_idx: usize,
    /// Detected track durations at best parameters
    pub detected_durations: Vec<f64>,
    /// Per-track errors at best parameters
    pub track_errors: Vec<f64>,
    /// Whether all tracks matched within tolerance
    pub all_tracks_matched: bool,
}

/// Run Stage 2 parameter grid search
///
/// # Arguments
/// * `silence_cache` - Pre-computed silence detections
/// * `editions` - Candidate editions to test
/// * `tolerance_secs` - Track match tolerance
/// * `early_exit` - Enable 100% match early exit
///
/// # Returns
/// Vector of results for each edition, sorted by best_percentage descending
pub fn run_stage2(
    silence_cache: &SilenceCache,
    editions: &[Edition],
    tolerance_secs: f64,
    early_exit: bool,
) -> Vec<Stage2Result> {
    let mut results = Vec::new();
    let mut found_perfect_match = false;

    for edition in editions {
        if found_perfect_match && early_exit {
            // Skip remaining editions after 100% match
            break;
        }

        let result = test_edition_stage2(silence_cache, edition, tolerance_secs);

        if result.best_percentage >= 100.0 {
            found_perfect_match = true;
        }

        results.push(result);
    }

    // Sort by best percentage descending
    results.sort_by(|a, b| {
        b.best_percentage.partial_cmp(&a.best_percentage)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    results
}

/// Test single edition across all parameter combinations
fn test_edition_stage2(
    silence_cache: &SilenceCache,
    edition: &Edition,
    tolerance_secs: f64,
) -> Stage2Result {
    let mut best_percentage = 0.0;
    let mut best_threshold_idx = 0;
    let mut best_duration_idx = 0;
    let mut best_durations = Vec::new();
    let mut best_errors = Vec::new();

    for threshold_idx in 0..silence_cache.num_thresholds {
        for duration_idx in 0..silence_cache.num_min_durations {
            let detected = silence_cache.get(threshold_idx, duration_idx);

            let result = analyze_track_matching(
                detected,
                &edition.durations,
                tolerance_secs,
            );

            if result.percentage > best_percentage {
                best_percentage = result.percentage;
                best_threshold_idx = threshold_idx;
                best_duration_idx = duration_idx;
                best_durations = result.detected_durations.clone();
                best_errors = result.errors.clone();
            }

            // Early exit on 100% for this edition
            if best_percentage >= 100.0 {
                break;
            }
        }
        if best_percentage >= 100.0 {
            break;
        }
    }

    Stage2Result {
        edition: edition.clone(),
        best_percentage,
        best_threshold_idx,
        best_duration_idx,
        detected_durations: best_durations,
        track_errors: best_errors,
        all_tracks_matched: best_percentage >= 100.0,
    }
}

/// Check if Stage 2 found acceptable match
pub fn stage2_success(results: &[Stage2Result], min_percentage: f64) -> bool {
    results.first()
        .map(|r| r.best_percentage >= min_percentage)
        .unwrap_or(false)
}
```

### 7.3 Add Early Exit Logic

```rust
/// Early exit configuration
#[derive(Debug, Clone)]
pub struct EarlyExitConfig {
    /// Enable early exit on perfect match
    pub enabled: bool,
    /// Grace period after 100% match (seconds)
    pub grace_period_secs: u64,
    /// Minimum match percentage to consider
    pub min_acceptable_percentage: f64,
}

impl Default for EarlyExitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            grace_period_secs: EARLY_EXIT_GRACE_PERIOD_SECS,
            min_acceptable_percentage: 80.0,
        }
    }
}
```

---

## Tests

| Test ID | Description | Pass Criteria |
|---------|-------------|---------------|
| TC-U-007-01 | Grid search all 180 combinations | All tested for single edition |
| TC-U-007-02 | Best parameter selection | Highest percentage selected |
| TC-U-007-03 | Early exit on 100% match | Stops after perfect match |
| TC-U-007-04 | Edition ranking | Best edition first in results |
| TC-I-007-01 | Integration with silence cache | Correct track detection |

---

## Acceptance Criteria

- [ ] stage2.rs created
- [ ] Grid search tests all 180 combinations
- [ ] Best parameters correctly identified
- [ ] Early exit working
- [ ] All 5 tests pass
