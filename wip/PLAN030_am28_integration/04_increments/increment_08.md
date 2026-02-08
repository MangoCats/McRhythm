# Increment 8: Stage 3 - Over-Segmentation Assembly

**Estimated Effort:** 4 hours
**Dependencies:** Increment 7
**Deliverables:** matching/stages/stage3.rs

---

## Objective

Implement Stage 3: dynamic programming assembly for over-segmented tracks. When silence detection finds too many boundaries (more tracks than expected), this stage tries to merge adjacent segments to match expected durations.

---

## Source Files

| am28 File | Lines | Action |
|-----------|-------|--------|
| stages/stage3.rs | ~350 | Adapt to library |

---

## Tasks

### 8.1 Create matching/stages/stage3.rs

```rust
//! Stage 3: Over-Segmentation Assembly
//!
//! Uses dynamic programming to find optimal merging of
//! over-segmented tracks to match expected edition durations.

use crate::matching::types::*;

/// Stage 3 result
#[derive(Debug, Clone)]
pub struct Stage3Result {
    /// Edition tested
    pub edition: Edition,
    /// Best match percentage achieved
    pub best_percentage: f64,
    /// Assembled track durations
    pub assembled_durations: Vec<f64>,
    /// Merge map: which detected segments form each track
    pub merge_map: Vec<Vec<usize>>,
    /// Per-track errors
    pub track_errors: Vec<f64>,
}

/// Run Stage 3 over-segmentation assembly
///
/// # Arguments
/// * `detected_durations` - Over-segmented track durations from Stage 2
/// * `editions` - Candidate editions (only those with fewer tracks than detected)
/// * `tolerance_secs` - Track match tolerance
///
/// # Returns
/// Vector of results for eligible editions
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

        if let Some(result) = try_assemble_tracks(
            detected_durations,
            edition,
            tolerance_secs,
        ) {
            results.push(result);
        }
    }

    // Sort by percentage descending
    results.sort_by(|a, b| {
        b.best_percentage.partial_cmp(&a.best_percentage)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    results
}

/// Try to assemble detected segments into expected track count
fn try_assemble_tracks(
    detected: &[f64],
    edition: &Edition,
    tolerance_secs: f64,
) -> Option<Stage3Result> {
    let n_detected = detected.len();
    let n_expected = edition.track_count;

    // DP state: dp[i][j] = best error sum to match first i expected tracks
    //                      using first j detected segments
    // We want to minimize total error

    let expected_secs: Vec<f64> = edition.durations.iter()
        .map(|ms| *ms as f64 / 1000.0)
        .collect();

    // dp[expected_idx][detected_idx] = (min_error, merge_choices)
    let inf = f64::MAX;
    let mut dp = vec![vec![(inf, Vec::new()); n_detected + 1]; n_expected + 1];
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

                // Only consider if within reasonable bounds (2x tolerance)
                if error <= tolerance_secs * 2.0 {
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
    let matched = errors.iter().filter(|&&e| e <= tolerance_secs).count();
    let percentage = (matched as f64 / n_expected as f64) * 100.0;

    Some(Stage3Result {
        edition: edition.clone(),
        best_percentage: percentage,
        assembled_durations: assembled,
        merge_map: merge_map.clone(),
        track_errors: errors,
    })
}

/// Check if Stage 3 found acceptable match
pub fn stage3_success(results: &[Stage3Result], min_percentage: f64) -> bool {
    results.first()
        .map(|r| r.best_percentage >= min_percentage)
        .unwrap_or(false)
}
```

---

## Tests

| Test ID | Description | Pass Criteria |
|---------|-------------|---------------|
| TC-U-008-01 | DP assembly basic | Merges 2 segments correctly |
| TC-U-008-02 | Complex merge pattern | Multi-segment merges work |
| TC-U-008-03 | No valid assembly | Returns None |
| TC-U-008-04 | Exact segment count | Skips (not over-segmented) |
| TC-I-008-01 | Integration with Stage 2 output | Improves Stage 2 result |

---

## Acceptance Criteria

- [ ] stage3.rs created
- [ ] DP algorithm correctly finds optimal merge
- [ ] Merge map accurately tracks segment grouping
- [ ] Percentage calculation correct
- [ ] All 5 tests pass
