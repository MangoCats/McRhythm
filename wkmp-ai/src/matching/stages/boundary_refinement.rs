//! Boundary Refinement
//!
//! Post-processes detected track boundaries to fix "split failure" patterns where
//! one track absorbed an adjacent track due to a missed boundary detection.
//!
//! # Split Failure Pattern
//! - Track N: detected < expected × 0.2 (nearly zero - boundary missed)
//! - Track N+1: detected > expected × 1.5 (absorbed the missing track)
//! - Errors roughly balance: |error_N + error_N+1| < tolerance × 2.0
//!
//! # Solution
//! Re-search for the missing boundary near the expected position using local RMS analysis.
//!
//! # Usage
//! This module provides refinement for all stages via `apply_refinement_to_durations()`,
//! which converts durations to boundaries, applies refinement, and converts back.

use tracing::{debug, info};

/// Apply boundary refinement to detected durations
///
/// Converts durations to boundaries, applies refinement, converts back to durations.
/// This allows refinement to be applied to any stage's output.
///
/// # Arguments
/// * `detected_durations` - Track durations in seconds (from any stage)
/// * `expected_durations` - Expected track durations from edition (seconds)
/// * `audio_samples` - Raw audio sample data
/// * `sample_rate` - Sample rate in Hz
/// * `tolerance_secs` - Track match tolerance (seconds)
///
/// # Returns
/// Refined track durations in seconds
pub fn apply_refinement_to_durations(
    detected_durations: &[f64],
    expected_durations: &[f64],
    audio_samples: &[f32],
    sample_rate: f64,
    tolerance_secs: f64,
) -> Vec<f64> {
    if detected_durations.is_empty() {
        return Vec::new();
    }

    // Convert durations to boundaries
    let mut boundaries = vec![0];
    let mut cumulative_samples = 0;
    for &duration_secs in detected_durations {
        cumulative_samples += (duration_secs * sample_rate) as usize;
        boundaries.push(cumulative_samples);
    }

    // Apply refinement
    let refined_boundaries = refine_missed_boundaries(
        &boundaries,
        expected_durations,
        audio_samples,
        sample_rate,
        tolerance_secs,
    );

    // Convert boundaries back to durations
    let mut refined_durations = Vec::new();
    for i in 1..refined_boundaries.len() {
        let duration_secs = (refined_boundaries[i] - refined_boundaries[i - 1]) as f64 / sample_rate;
        refined_durations.push(duration_secs);
    }

    refined_durations
}

/// Post-process detected boundaries to fix "split failures"
///
/// # Arguments
/// * `detected_boundaries` - Sample positions of detected boundaries (including 0 and total_samples)
/// * `expected_durations` - Expected track durations in seconds
/// * `audio_samples` - Audio sample data
/// * `sample_rate` - Audio sample rate
/// * `tolerance_secs` - Tolerance for considering errors balanced
///
/// # Returns
/// Refined boundary positions
pub fn refine_missed_boundaries(
    detected_boundaries: &[usize],
    expected_durations: &[f64],
    audio_samples: &[f32],
    sample_rate: f64,
    tolerance_secs: f64,
) -> Vec<usize> {
    if detected_boundaries.len() < 3 {
        // Need at least [start, boundary, end] to have one track
        return detected_boundaries.to_vec();
    }

    let mut refined = detected_boundaries.to_vec();
    let mut any_refined = false;

    // Calculate detected durations from boundaries
    let mut detected_durations = calculate_durations(&refined, sample_rate);

    // Iteratively scan for split failure patterns
    // Use loop to handle consecutive failures (like tracks 7-10 in Panorama)
    let max_iterations = 3; // Prevent infinite loops
    for iteration in 0..max_iterations {
        let mut refined_this_iteration = false;

        for i in 0..detected_durations.len().saturating_sub(1) {
            let detected_i = detected_durations[i];
            let expected_i = expected_durations.get(i).copied().unwrap_or(0.0);
            let detected_i1 = detected_durations[i + 1];
            let expected_i1 = expected_durations.get(i + 1).copied().unwrap_or(0.0);

            if expected_i <= 0.0 || expected_i1 <= 0.0 {
                continue;
            }

            let error_i = detected_i - expected_i;
            let error_i1 = detected_i1 - expected_i1;

            // Pattern: Track i too short, track i+1 too long, errors roughly cancel
            let is_split_failure = detected_i < expected_i * 0.2
                && detected_i1 > expected_i1 * 1.5
                && (error_i + error_i1).abs() < tolerance_secs * 2.0;

            if is_split_failure {
                debug!(
                    "Split failure detected at track {}: detected={:.2}s (expected {:.2}s), next={:.2}s (expected {:.2}s)",
                    i + 1,
                    detected_i,
                    expected_i,
                    detected_i1,
                    expected_i1
                );

                // Attempt to find missing boundary between i and i+1
                let search_start = refined[i]; // Start of track i
                let search_end = refined[i + 1]; // Original boundary i+1

                // Expected location for boundary between i and i+1
                let expected_boundary_samples =
                    search_start + (expected_i * sample_rate) as usize;

                // Search window: ±15 seconds around expected position
                let window_samples = (15.0 * sample_rate) as usize;
                let window_start = expected_boundary_samples.saturating_sub(window_samples);
                let window_end = (expected_boundary_samples + window_samples).min(search_end);

                // Find local minimum (quiet spot) in the search window
                if let Some(new_boundary) =
                    find_local_quiet_spot(audio_samples, window_start, window_end, sample_rate)
                {
                    info!(
                        "Refining boundary {}: moving from sample {} to {} (expected: {})",
                        i + 1,
                        refined[i + 1],
                        new_boundary,
                        expected_boundary_samples
                    );
                    refined[i + 1] = new_boundary;
                    refined_this_iteration = true;
                    any_refined = true;
                }
            }
        }

        if refined_this_iteration {
            // Recalculate durations for next iteration
            detected_durations = calculate_durations(&refined, sample_rate);
        } else {
            // No more refinements found
            break;
        }
    }

    if any_refined {
        info!(
            "Boundary refinement complete: {} boundaries adjusted",
            refined
                .iter()
                .zip(detected_boundaries.iter())
                .filter(|(r, d)| r != d)
                .count()
        );
    }

    refined
}

/// Find quietest spot in a localized region using RMS analysis
///
/// # Arguments
/// * `audio_samples` - Audio sample data
/// * `start_sample` - Start of search region
/// * `end_sample` - End of search region
/// * `sample_rate` - Audio sample rate
///
/// # Returns
/// Sample position of quietest spot, or None if search failed
fn find_local_quiet_spot(
    audio_samples: &[f32],
    start_sample: usize,
    end_sample: usize,
    sample_rate: f64,
) -> Option<usize> {
    const WINDOW_SIZE_SECS: f64 = 0.5; // 500ms RMS window
    let window_size = (WINDOW_SIZE_SECS * sample_rate) as usize;

    if end_sample <= start_sample + window_size {
        debug!(
            "Search region too small: {} samples (need at least {})",
            end_sample - start_sample,
            window_size
        );
        return None;
    }

    if end_sample > audio_samples.len() {
        debug!(
            "Search region exceeds audio length: {} > {}",
            end_sample,
            audio_samples.len()
        );
        return None;
    }

    let mut min_rms = f32::MAX;
    let mut min_pos = start_sample;

    // Slide window through search region, find minimum RMS
    for pos in start_sample..=(end_sample - window_size) {
        let window_end = pos + window_size;
        if window_end > audio_samples.len() {
            break;
        }

        // Calculate RMS for this window
        let rms = calculate_rms(&audio_samples[pos..window_end]);

        if rms < min_rms {
            min_rms = rms;
            min_pos = pos;
        }
    }

    // Center the boundary in the quiet window (take midpoint)
    let refined_pos = min_pos + window_size / 2;
    debug!(
        "Found quiet spot at sample {} (RMS: {:.6}) in range [{}, {}]",
        refined_pos, min_rms, start_sample, end_sample
    );
    Some(refined_pos)
}

/// Calculate RMS (Root Mean Square) of audio samples
fn calculate_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_squares: f32 = samples.iter().map(|&s| s * s).sum();
    (sum_squares / samples.len() as f32).sqrt()
}

/// Calculate track durations from boundary positions
fn calculate_durations(boundaries: &[usize], sample_rate: f64) -> Vec<f64> {
    boundaries
        .windows(2)
        .map(|w| (w[1] - w[0]) as f64 / sample_rate)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_rms() {
        let samples = vec![0.5, -0.5, 0.3, -0.3];
        let rms = calculate_rms(&samples);
        assert!((rms - 0.4).abs() < 0.01);
    }

    #[test]
    fn test_calculate_durations() {
        let boundaries = vec![0, 44100, 88200]; // 0s, 1s, 2s at 44.1kHz
        let durations = calculate_durations(&boundaries, 44100.0);
        assert_eq!(durations.len(), 2);
        assert!((durations[0] - 1.0).abs() < 0.001);
        assert!((durations[1] - 1.0).abs() < 0.001);
    }
}
