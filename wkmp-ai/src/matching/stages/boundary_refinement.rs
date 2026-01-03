//! Boundary Refinement
//!
//! Post-processes detected track boundaries to fix "split failure" patterns where
//! one track absorbed an adjacent track due to a missed boundary detection.
//!
//! # Split Failure Patterns
//!
//! ## Forward Pattern (Track N collapses, Track N+1 absorbs)
//! - Track N: detected < expected × 0.2 (nearly zero - boundary missed)
//! - Track N+1: detected > expected × 1.5 (absorbed the missing track)
//! - Errors roughly balance: |error_N + error_N+1| < tolerance × 2.0
//!
//! ## Reverse Pattern (Track N absorbs, Track N+1 collapses)
//! - Track N: detected > expected × 1.5 (absorbed the next track)
//! - Track N+1: detected < expected × 0.2 (nearly zero - boundary detected too late)
//! - Errors roughly balance: |error_N + error_N+1| < tolerance × 2.0
//!
//! # Solution
//! Re-search for the missing boundary near the expected position using local RMS analysis.
//! The search strategy is identical for both patterns.
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

            // Check magnitude thresholds for both patterns
            let i_too_short = detected_i < expected_i * 0.2;
            let i_too_long = detected_i > expected_i * 1.5;
            let i1_too_short = detected_i1 < expected_i1 * 0.2;
            let i1_too_long = detected_i1 > expected_i1 * 1.5;

            // Check if errors cancel out (both patterns)
            let errors_cancel = (error_i + error_i1).abs() < tolerance_secs * 2.0;

            // Pattern 1: Forward split failure (i collapsed, i+1 absorbed)
            let is_forward_split = i_too_short && i1_too_long && errors_cancel;

            // Pattern 2: Reverse split failure (i absorbed, i+1 collapsed)
            let is_reverse_split = i_too_long && i1_too_short && errors_cancel;

            let is_split_failure = is_forward_split || is_reverse_split;

            if is_split_failure {
                let pattern_type = if is_forward_split { "forward" } else { "reverse" };
                debug!(
                    "Split failure detected ({}) at track {}: detected={:.2}s (expected {:.2}s), next={:.2}s (expected {:.2}s)",
                    pattern_type,
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
                        "Refining boundary {} ({}): moving from sample {} to {} (expected: {})",
                        i + 1,
                        pattern_type,
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

    /// Helper to create silent audio samples
    fn create_silent_audio(duration_secs: f64, sample_rate: f64) -> Vec<f32> {
        vec![0.0; (duration_secs * sample_rate) as usize]
    }

    #[test]
    fn test_forward_split_failure_detection() {
        // Test Case 1: Forward pattern (track 1 collapsed, track 2 absorbed it)
        // Expected: [200s, 180s, 220s]
        // Detected: [ 30s, 370s, 220s]  (boundary 1 missed, track 1 collapsed)

        let sample_rate = 44100.0;
        let expected = vec![200.0, 180.0, 220.0];

        // Create detected boundaries that simulate forward split failure
        let detected_boundaries = vec![
            0,                                    // Start
            (30.0 * sample_rate) as usize,        // Track 1 collapsed to 30s
            (400.0 * sample_rate) as usize,       // Track 2 absorbed: 30 + 370 = 400
            (620.0 * sample_rate) as usize,       // Track 3 normal: 400 + 220 = 620
        ];

        // Create audio samples (silent for test)
        let audio_samples = create_silent_audio(620.0, sample_rate);

        let tolerance = 10.0; // 10 second tolerance

        let refined = refine_missed_boundaries(
            &detected_boundaries,
            &expected,
            &audio_samples,
            sample_rate,
            tolerance,
        );

        // Verify that boundary 1 was refined
        // Should move from 30s position toward 200s position
        let refined_durations = calculate_durations(&refined, sample_rate);

        // First track should be closer to 200s than original 30s
        assert!(
            (refined_durations[0] - 200.0).abs() < (30.0_f64 - 200.0).abs(),
            "Forward pattern: track 1 duration should be refined closer to 200s (got {:.2}s)",
            refined_durations[0]
        );
    }

    #[test]
    fn test_reverse_split_failure_detection() {
        // Test Case 2: Reverse pattern (track 1 absorbed track 2, track 2 collapsed)
        // Expected: [184s, 293s]  (Doobie Brothers case)
        // Detected: [466s,   1s]  (boundary detected too late, track 1 absorbed track 2)

        let sample_rate = 44100.0;
        let expected = vec![184.0, 293.0];

        // Create detected boundaries that simulate reverse split failure
        let detected_boundaries = vec![
            0,                                    // Start
            (466.0 * sample_rate) as usize,       // Track 1 absorbed track 2
            (467.0 * sample_rate) as usize,       // Track 2 collapsed to ~1s
        ];

        // Create audio samples (silent for test)
        let audio_samples = create_silent_audio(467.0, sample_rate);

        let tolerance = 10.0; // 10 second tolerance

        let refined = refine_missed_boundaries(
            &detected_boundaries,
            &expected,
            &audio_samples,
            sample_rate,
            tolerance,
        );

        // Verify that boundary 1 was refined
        let refined_durations = calculate_durations(&refined, sample_rate);

        // First track should be closer to 184s than original 466s
        assert!(
            (refined_durations[0] - 184.0).abs() < (466.0_f64 - 184.0).abs(),
            "Reverse pattern: track 1 duration should be refined closer to 184s (got {:.2}s)",
            refined_durations[0]
        );

        // Second track should be closer to 293s than original 1s
        assert!(
            (refined_durations[1] - 293.0).abs() < (1.0_f64 - 293.0).abs(),
            "Reverse pattern: track 2 duration should be refined closer to 293s (got {:.2}s)",
            refined_durations[1]
        );
    }

    #[test]
    fn test_no_refinement_needed() {
        // Test Case 3: All tracks within tolerance
        // Expected: [200s, 180s, 220s]
        // Detected: [198s, 182s, 218s]  (all within tolerance)

        let sample_rate = 44100.0;
        let expected = vec![200.0, 180.0, 220.0];

        let detected_boundaries = vec![
            0,
            (198.0 * sample_rate) as usize,
            (380.0 * sample_rate) as usize,  // 198 + 182
            (598.0 * sample_rate) as usize,  // 380 + 218
        ];

        let audio_samples = create_silent_audio(598.0, sample_rate);
        let tolerance = 10.0;

        let refined = refine_missed_boundaries(
            &detected_boundaries,
            &expected,
            &audio_samples,
            sample_rate,
            tolerance,
        );

        // Boundaries should remain unchanged
        assert_eq!(
            detected_boundaries, refined,
            "No refinement should occur when all tracks are within tolerance"
        );
    }

    #[test]
    fn test_errors_dont_cancel() {
        // Test Case 4: Errors don't cancel (should NOT trigger refinement)
        // Expected: [200s, 180s, 220s]
        // Detected: [ 50s, 500s, 220s]
        // Error 1: -150s, Error 2: +320s, Sum: +170s > tolerance

        let sample_rate = 44100.0;
        let expected = vec![200.0, 180.0, 220.0];

        let detected_boundaries = vec![
            0,
            (50.0 * sample_rate) as usize,
            (550.0 * sample_rate) as usize,  // 50 + 500
            (770.0 * sample_rate) as usize,  // 550 + 220
        ];

        let audio_samples = create_silent_audio(770.0, sample_rate);
        let tolerance = 10.0;

        let refined = refine_missed_boundaries(
            &detected_boundaries,
            &expected,
            &audio_samples,
            sample_rate,
            tolerance,
        );

        // Boundaries should remain unchanged (errors don't cancel)
        assert_eq!(
            detected_boundaries, refined,
            "No refinement should occur when errors don't cancel out"
        );
    }

    #[test]
    fn test_three_track_reverse_pattern() {
        // Test Case 5: Three tracks with reverse split in middle
        // Expected: [150s, 200s, 180s]
        // Detected: [150s, 379s,   1s]  (track 2 absorbed track 3)

        let sample_rate = 44100.0;
        let expected = vec![150.0, 200.0, 180.0];

        let detected_boundaries = vec![
            0,
            (150.0 * sample_rate) as usize,  // Track 1 correct
            (529.0 * sample_rate) as usize,  // Track 2 absorbed track 3: 150 + 379
            (530.0 * sample_rate) as usize,  // Track 3 collapsed to 1s
        ];

        let audio_samples = create_silent_audio(530.0, sample_rate);
        let tolerance = 10.0;

        let refined = refine_missed_boundaries(
            &detected_boundaries,
            &expected,
            &audio_samples,
            sample_rate,
            tolerance,
        );

        let refined_durations = calculate_durations(&refined, sample_rate);

        // Track 1 should remain unchanged
        assert!(
            (refined_durations[0] - 150.0).abs() < 5.0_f64,
            "Track 1 should remain near 150s (got {:.2}s)",
            refined_durations[0]
        );

        // Track 2 should be refined closer to 200s
        assert!(
            (refined_durations[1] - 200.0).abs() < (379.0_f64 - 200.0).abs(),
            "Track 2 should be refined closer to 200s (got {:.2}s)",
            refined_durations[1]
        );

        // Track 3 should be refined closer to 180s
        assert!(
            (refined_durations[2] - 180.0).abs() < (1.0_f64 - 180.0).abs(),
            "Track 3 should be refined closer to 180s (got {:.2}s)",
            refined_durations[2]
        );
    }

    #[test]
    fn test_consecutive_failures() {
        // Test Case 6: Multiple consecutive split failures
        // Expected: [100s, 150s, 180s, 120s]
        // Detected: [ 20s, 330s, 300s,   1s]
        // Pattern: Track 1 collapsed (forward), Track 3 absorbed track 4 (reverse)

        let sample_rate = 44100.0;
        let expected = vec![100.0, 150.0, 180.0, 120.0];

        let detected_boundaries = vec![
            0,
            (20.0 * sample_rate) as usize,   // Track 1 collapsed
            (350.0 * sample_rate) as usize,  // Track 2 absorbed track 1: 20 + 330
            (650.0 * sample_rate) as usize,  // Track 3 absorbed track 4: 350 + 300
            (651.0 * sample_rate) as usize,  // Track 4 collapsed to 1s
        ];

        let audio_samples = create_silent_audio(651.0, sample_rate);
        let tolerance = 10.0;

        let refined = refine_missed_boundaries(
            &detected_boundaries,
            &expected,
            &audio_samples,
            sample_rate,
            tolerance,
        );

        let refined_durations = calculate_durations(&refined, sample_rate);

        // Both boundaries should be refined
        // Track 1 should be closer to 100s
        assert!(
            (refined_durations[0] - 100.0).abs() < (20.0_f64 - 100.0).abs(),
            "Track 1 should be refined closer to 100s (got {:.2}s)",
            refined_durations[0]
        );

        // Track 4 should be closer to 120s
        assert!(
            (refined_durations[3] - 120.0).abs() < (1.0_f64 - 120.0).abs(),
            "Track 4 should be refined closer to 120s (got {:.2}s)",
            refined_durations[3]
        );
    }
}
