# Increment 4: Silence Detection

**Estimated Effort:** 4 hours
**Dependencies:** Increment 1, 2, 3
**Deliverables:** matching/silence_detection.rs

---

## Objective

Create silence detection module with parameter grid pre-computation for efficient Stage 2 processing.

---

## Source Files

| am28 File | Lines | Action |
|-----------|-------|--------|
| silence_detection.rs | 387 | Adapt to library |

---

## Tasks

### 4.1 Create matching/silence_detection.rs

```rust
//! Silence Detection for Album Matching
//!
//! Pre-computes silence regions for all parameter combinations
//! (180 by default: 12 thresholds × 15 min durations).

use crate::matching::constants::*;

/// Pre-computed silence detection for all parameter combinations
#[derive(Debug, Clone)]
pub struct SilenceCache {
    /// Detected track durations for each (threshold_idx, min_duration_idx)
    /// Shape: [num_thresholds][num_min_durations]
    pub durations: Vec<Vec<Vec<f64>>>,
    /// Number of threshold values
    pub num_thresholds: usize,
    /// Number of min duration values
    pub num_min_durations: usize,
}

impl SilenceCache {
    /// Get track durations for specific parameters
    pub fn get(&self, threshold_idx: usize, min_duration_idx: usize) -> &[f64] {
        &self.durations[threshold_idx][min_duration_idx]
    }

    /// Total combinations
    pub fn total_combinations(&self) -> usize {
        self.num_thresholds * self.num_min_durations
    }
}

/// Pre-compute silence cache for all parameter combinations
///
/// # Arguments
/// * `samples` - Mono audio samples (normalized -1.0 to 1.0)
/// * `sample_rate` - Audio sample rate (Hz)
/// * `threshold_values` - Threshold values to test (dB)
/// * `min_duration_values` - Minimum duration values to test (seconds)
///
/// # Returns
/// SilenceCache with pre-computed track durations
pub fn precompute_silence_cache(
    samples: &[f32],
    sample_rate: u32,
    threshold_values: &[f64],
    min_duration_values: &[f64],
) -> SilenceCache {
    let num_thresholds = threshold_values.len();
    let num_min_durations = min_duration_values.len();

    let mut durations = Vec::with_capacity(num_thresholds);

    for threshold_db in threshold_values {
        let mut threshold_durations = Vec::with_capacity(num_min_durations);

        for min_duration_secs in min_duration_values {
            let track_durations = detect_track_boundaries(
                samples,
                sample_rate,
                *threshold_db,
                *min_duration_secs,
            );
            threshold_durations.push(track_durations);
        }

        durations.push(threshold_durations);
    }

    SilenceCache {
        durations,
        num_thresholds,
        num_min_durations,
    }
}

/// Detect track boundaries based on silence
///
/// # Arguments
/// * `samples` - Mono audio samples
/// * `sample_rate` - Sample rate in Hz
/// * `threshold_db` - Silence threshold in dB
/// * `min_duration_secs` - Minimum silence duration in seconds
///
/// # Returns
/// Vector of track durations in seconds
pub fn detect_track_boundaries(
    samples: &[f32],
    sample_rate: u32,
    threshold_db: f64,
    min_duration_secs: f64,
) -> Vec<f64> {
    let threshold_linear = db_to_linear(threshold_db);
    let min_samples = (min_duration_secs * sample_rate as f64) as usize;
    let window_size = (sample_rate as f64 * 0.05) as usize; // 50ms windows

    let mut boundaries = Vec::new();
    let mut current_silence_start: Option<usize> = None;

    // Scan for silence regions using RMS in windows
    for (window_idx, window) in samples.chunks(window_size).enumerate() {
        let rms = calculate_rms(window);
        let sample_pos = window_idx * window_size;

        if rms < threshold_linear {
            // In silence
            if current_silence_start.is_none() {
                current_silence_start = Some(sample_pos);
            }
        } else {
            // Not in silence
            if let Some(start) = current_silence_start {
                let silence_samples = sample_pos - start;
                if silence_samples >= min_samples {
                    // Valid silence region - mark midpoint as boundary
                    let midpoint = start + silence_samples / 2;
                    boundaries.push(midpoint);
                }
                current_silence_start = None;
            }
        }
    }

    // Convert boundaries to track durations
    boundaries_to_durations(&boundaries, samples.len(), sample_rate)
}

/// Calculate RMS of audio samples
pub fn calculate_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_squares: f32 = samples.iter().map(|s| s * s).sum();
    (sum_squares / samples.len() as f32).sqrt()
}

/// Convert dB to linear amplitude
fn db_to_linear(db: f64) -> f32 {
    10.0_f64.powf(db / 20.0) as f32
}

/// Convert boundary positions to track durations
fn boundaries_to_durations(
    boundaries: &[usize],
    total_samples: usize,
    sample_rate: u32,
) -> Vec<f64> {
    let mut durations = Vec::new();
    let mut prev_pos = 0usize;

    for &boundary in boundaries {
        let duration_samples = boundary - prev_pos;
        let duration_secs = duration_samples as f64 / sample_rate as f64;
        durations.push(duration_secs);
        prev_pos = boundary;
    }

    // Final track
    let final_duration = (total_samples - prev_pos) as f64 / sample_rate as f64;
    if final_duration > 0.1 {
        durations.push(final_duration);
    }

    durations
}
```

### 4.2 Add Module to matching/mod.rs

```rust
pub mod silence_detection;
pub use silence_detection::{SilenceCache, precompute_silence_cache, detect_track_boundaries, calculate_rms};
```

---

## Tests

| Test ID | Description | Pass Criteria |
|---------|-------------|---------------|
| TC-U-004-01 | Silence cache generation | All 180 combinations computed |
| TC-U-004-02 | Track boundary detection from silence | Boundaries at silence regions |
| TC-U-004-03 | Parameter grid iteration | Correct indexing into cache |
| TC-U-004-04 | RMS calculation | Correct RMS values for known input |
| TC-I-004-01 | Integration with decoded audio | Detects tracks in real audio |

---

## Acceptance Criteria

- [ ] silence_detection.rs created
- [ ] Pre-computation generates all 180 combinations
- [ ] Correct track boundaries detected
- [ ] RMS calculation correct
- [ ] All 5 tests pass
