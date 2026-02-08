# Increment 10: Stage 5 - Extra Track Merging

**Estimated Effort:** 3 hours
**Dependencies:** Increment 7
**Deliverables:** matching/stages/stage5.rs

---

## Objective

Implement Stage 5: extra track merging for albums with bonus/hidden tracks. When detected track count exceeds expected, this stage tries merging final tracks.

---

## Source Files

| am28 File | Lines | Action |
|-----------|-------|--------|
| stages/stage5.rs | ~200 | Adapt to library |

---

## Tasks

### 10.1 Create matching/stages/stage5.rs

```rust
//! Stage 5: Extra Track Merging
//!
//! Handles albums with bonus/hidden tracks by merging
//! extra detected tracks at the end.

use crate::matching::types::*;

/// Stage 5 result
#[derive(Debug, Clone)]
pub struct Stage5Result {
    /// Edition tested
    pub edition: Edition,
    /// Match percentage after merging
    pub percentage: f64,
    /// Number of extra tracks merged
    pub tracks_merged: usize,
    /// Final track durations after merging
    pub merged_durations: Vec<f64>,
    /// Per-track errors
    pub track_errors: Vec<f64>,
}

/// Run Stage 5 extra track merging
///
/// # Arguments
/// * `detected_durations` - Detected track durations
/// * `editions` - Candidate editions (only those with fewer tracks)
/// * `tolerance_secs` - Track match tolerance
/// * `max_merge` - Maximum tracks to merge at end
///
/// # Returns
/// Vector of results for eligible editions
pub fn run_stage5(
    detected_durations: &[f64],
    editions: &[Edition],
    tolerance_secs: f64,
    max_merge: usize,
) -> Vec<Stage5Result> {
    let detected_count = detected_durations.len();
    let mut results = Vec::new();

    for edition in editions {
        let expected_count = edition.track_count;

        // Only process if we have extra tracks
        let extra = detected_count.saturating_sub(expected_count);
        if extra == 0 || extra > max_merge {
            continue;
        }

        if let Some(result) = try_merge_extra_tracks(
            detected_durations,
            edition,
            tolerance_secs,
        ) {
            results.push(result);
        }
    }

    // Sort by percentage descending
    results.sort_by(|a, b| {
        b.percentage.partial_cmp(&a.percentage)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    results
}

/// Try merging extra tracks at the end
fn try_merge_extra_tracks(
    detected: &[f64],
    edition: &Edition,
    tolerance_secs: f64,
) -> Option<Stage5Result> {
    let expected_count = edition.track_count;
    let detected_count = detected.len();
    let extra = detected_count - expected_count;

    let expected_secs: Vec<f64> = edition.durations.iter()
        .map(|ms| *ms as f64 / 1000.0)
        .collect();

    // Create merged durations: keep first (expected_count - 1) tracks,
    // merge remaining tracks into final track
    let mut merged = Vec::with_capacity(expected_count);

    // Copy all but the "extra" tracks
    for i in 0..(expected_count - 1) {
        merged.push(detected[i]);
    }

    // Merge last (extra + 1) detected tracks into final expected track
    let merge_start = expected_count - 1;
    let final_merged: f64 = detected[merge_start..].iter().sum();
    merged.push(final_merged);

    // Calculate errors
    let mut errors = Vec::new();
    let mut matched = 0;
    for (merged_dur, expected_dur) in merged.iter().zip(expected_secs.iter()) {
        let error = (merged_dur - expected_dur).abs();
        errors.push(error);
        if error <= tolerance_secs {
            matched += 1;
        }
    }

    let percentage = (matched as f64 / expected_count as f64) * 100.0;

    // Only return if this gives reasonable match (> 50%)
    if percentage < 50.0 {
        return None;
    }

    Some(Stage5Result {
        edition: edition.clone(),
        percentage,
        tracks_merged: extra + 1, // +1 for the original last track
        merged_durations: merged,
        track_errors: errors,
    })
}

/// Check if Stage 5 found acceptable match
pub fn stage5_success(results: &[Stage5Result], min_percentage: f64) -> bool {
    results.first()
        .map(|r| r.percentage >= min_percentage)
        .unwrap_or(false)
}
```

---

## Tests

| Test ID | Description | Pass Criteria |
|---------|-------------|---------------|
| TC-U-010-01 | Merge 1 extra track | Correct merged duration |
| TC-U-010-02 | Merge 2 extra tracks | Both merged correctly |
| TC-U-010-03 | No extra tracks | Returns empty (skips) |
| TC-U-010-04 | Too many extra tracks | Returns empty (beyond max) |
| TC-I-010-01 | Integration with Stage 2 | Improves partial match |

---

## Acceptance Criteria

- [ ] stage5.rs created
- [ ] Extra track detection working
- [ ] Merging logic correct
- [ ] Percentage calculation accurate
- [ ] All 5 tests pass
