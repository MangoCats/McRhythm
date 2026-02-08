# Increment 9: Stage 4 - Quiet Spot Detection

**Estimated Effort:** 4 hours
**Dependencies:** Increment 4, 7
**Deliverables:** matching/stages/stage4.rs

---

## Objective

Implement Stage 4: edition-guided quiet spot detection using RMS profiling. When silence detection fails, this stage uses expected track boundaries to search for quiet spots (local RMS minima) near expected positions.

---

## Source Files

| am28 File | Lines | Action |
|-----------|-------|--------|
| stages/stage4.rs | ~380 | Adapt to library |

---

## Tasks

### 9.1 Create matching/stages/stage4.rs

```rust
//! Stage 4: Quiet Spot Detection
//!
//! Uses RMS profiling to find quiet spots near expected track boundaries.
//! Applied when silence-based detection fails.

use crate::matching::types::*;
use crate::matching::constants::STAGE4_PENALTY_PERCENT;

/// Stage 4 result
#[derive(Debug, Clone)]
pub struct Stage4Result {
    /// Edition tested
    pub edition: Edition,
    /// Match percentage (with penalty applied)
    pub penalized_percentage: f64,
    /// Raw match percentage (before penalty)
    pub raw_percentage: f64,
    /// Detected boundaries (sample positions)
    pub boundary_positions: Vec<usize>,
    /// Detected track durations
    pub detected_durations: Vec<f64>,
    /// Per-track errors
    pub track_errors: Vec<f64>,
}

/// RMS profile for audio
#[derive(Debug, Clone)]
pub struct RmsProfile {
    /// RMS values per window
    pub values: Vec<f32>,
    /// Window size in samples
    pub window_size: usize,
    /// Sample rate
    pub sample_rate: u32,
}

impl RmsProfile {
    /// Create RMS profile from audio samples
    pub fn from_samples(samples: &[f32], sample_rate: u32, window_ms: f64) -> Self {
        let window_size = (sample_rate as f64 * window_ms / 1000.0) as usize;
        let values: Vec<f32> = samples
            .chunks(window_size)
            .map(|chunk| {
                let sum_sq: f32 = chunk.iter().map(|s| s * s).sum();
                (sum_sq / chunk.len() as f32).sqrt()
            })
            .collect();

        Self {
            values,
            window_size,
            sample_rate,
        }
    }

    /// Find quietest spot within search window around target position
    pub fn find_quiet_spot(
        &self,
        target_sample: usize,
        search_window_samples: usize,
    ) -> Option<usize> {
        let target_idx = target_sample / self.window_size;
        let search_windows = search_window_samples / self.window_size;

        let start_idx = target_idx.saturating_sub(search_windows);
        let end_idx = (target_idx + search_windows).min(self.values.len());

        if start_idx >= end_idx {
            return None;
        }

        // Find minimum RMS in search window
        let (min_idx, _) = self.values[start_idx..end_idx]
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))?;

        Some((start_idx + min_idx) * self.window_size)
    }
}

/// Run Stage 4 quiet spot detection
///
/// # Arguments
/// * `rms_profile` - Pre-computed RMS profile
/// * `total_samples` - Total audio samples
/// * `editions` - Candidate editions to test
/// * `tolerance_secs` - Track match tolerance
/// * `search_window_secs` - Window to search for quiet spots
///
/// # Returns
/// Vector of results for each edition
pub fn run_stage4(
    rms_profile: &RmsProfile,
    total_samples: usize,
    editions: &[Edition],
    tolerance_secs: f64,
    search_window_secs: f64,
) -> Vec<Stage4Result> {
    let sample_rate = rms_profile.sample_rate;
    let search_window_samples = (search_window_secs * sample_rate as f64) as usize;
    let mut results = Vec::new();

    for edition in editions {
        let result = detect_quiet_spots(
            rms_profile,
            total_samples,
            edition,
            tolerance_secs,
            search_window_samples,
        );
        results.push(result);
    }

    // Sort by penalized percentage descending
    results.sort_by(|a, b| {
        b.penalized_percentage.partial_cmp(&a.penalized_percentage)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    results
}

/// Detect quiet spots for a single edition
fn detect_quiet_spots(
    rms_profile: &RmsProfile,
    total_samples: usize,
    edition: &Edition,
    tolerance_secs: f64,
    search_window_samples: usize,
) -> Stage4Result {
    let sample_rate = rms_profile.sample_rate;

    // Calculate expected boundary positions from edition durations
    let expected_secs: Vec<f64> = edition.durations.iter()
        .map(|ms| *ms as f64 / 1000.0)
        .collect();

    let mut expected_boundaries = Vec::new();
    let mut cumulative = 0.0;
    for dur in &expected_secs[..expected_secs.len().saturating_sub(1)] {
        cumulative += dur;
        let sample_pos = (cumulative * sample_rate as f64) as usize;
        expected_boundaries.push(sample_pos);
    }

    // Find quiet spots near each expected boundary
    let mut detected_boundaries = Vec::new();
    for &expected_pos in &expected_boundaries {
        if let Some(quiet_pos) = rms_profile.find_quiet_spot(expected_pos, search_window_samples) {
            detected_boundaries.push(quiet_pos);
        } else {
            // Fall back to expected position
            detected_boundaries.push(expected_pos);
        }
    }

    // Convert boundaries to durations
    let mut detected_durations = Vec::new();
    let mut prev_pos = 0usize;
    for &boundary in &detected_boundaries {
        let duration_secs = (boundary - prev_pos) as f64 / sample_rate as f64;
        detected_durations.push(duration_secs);
        prev_pos = boundary;
    }
    // Final track
    let final_duration = (total_samples - prev_pos) as f64 / sample_rate as f64;
    detected_durations.push(final_duration);

    // Calculate errors and match percentage
    let mut errors = Vec::new();
    let mut matched = 0;
    for (detected, expected) in detected_durations.iter().zip(expected_secs.iter()) {
        let error = (detected - expected).abs();
        errors.push(error);
        if error <= tolerance_secs {
            matched += 1;
        }
    }

    let raw_percentage = (matched as f64 / expected_secs.len() as f64) * 100.0;
    // Apply penalty since this is a less reliable method
    let penalized_percentage = raw_percentage * (1.0 - STAGE4_PENALTY_PERCENT / 100.0);

    Stage4Result {
        edition: edition.clone(),
        penalized_percentage,
        raw_percentage,
        boundary_positions: detected_boundaries,
        detected_durations,
        track_errors: errors,
    }
}

/// Check if Stage 4 found acceptable match
pub fn stage4_success(results: &[Stage4Result], min_percentage: f64) -> bool {
    results.first()
        .map(|r| r.penalized_percentage >= min_percentage)
        .unwrap_or(false)
}
```

---

## Tests

| Test ID | Description | Pass Criteria |
|---------|-------------|---------------|
| TC-U-009-01 | RMS profile creation | Correct window RMS values |
| TC-U-009-02 | Quiet spot finding | Finds local minimum |
| TC-U-009-03 | Boundary detection | Correct positions from quiet spots |
| TC-U-009-04 | Penalty application | 25% penalty correctly applied |
| TC-I-009-01 | Integration with audio samples | Detects real quiet spots |

---

## Acceptance Criteria

- [ ] stage4.rs created
- [ ] RMS profiling working
- [ ] Quiet spot detection finds local minima
- [ ] Penalty correctly applied
- [ ] All 5 tests pass
