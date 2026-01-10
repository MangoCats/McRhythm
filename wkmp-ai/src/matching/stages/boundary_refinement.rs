//! Boundary Refinement
//!
//! Post-processes detected track boundaries to fix "split failure" patterns where
//! one track absorbed an adjacent track due to a missed boundary detection.
//!
//! ## Error-Based Detection
//!
//! Detects split failures by analyzing error magnitudes and cancellation:
//! - Both tracks have significant errors (≥30s each)
//! - Errors in opposite directions (one too long, one too short)
//! - Errors nearly cancel (residual < 2% of combined duration)
//! - Combined error magnitude ≥60s
//!
//! This catches both extreme failures (track collapses to 0.01s) and
//! moderate failures (boundary 30-80s off) without relying on percentage
//! thresholds that vary with track length.
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

            // Error-based split failure detection
            // Detects moderate and extreme split failures via error analysis
            const MIN_ERROR_MAGNITUDE_SECS: f64 = 30.0;  // Minimum per-track error
            const MIN_COMBINED_ERROR_SECS: f64 = 60.0;   // Minimum total deviation
            const MAX_RESIDUAL_PCT: f64 = 0.02;          // Max 2% residual of combined duration

            // Calculate error metrics
            let abs_error_i = error_i.abs();
            let abs_error_i1 = error_i1.abs();
            let combined_error = abs_error_i + abs_error_i1;
            let residual = (error_i + error_i1).abs();
            let combined_duration = expected_i + expected_i1;

            // Check detection criteria
            let significant_errors = abs_error_i >= MIN_ERROR_MAGNITUDE_SECS
                                  && abs_error_i1 >= MIN_ERROR_MAGNITUDE_SECS;
            let opposite_directions = (error_i > 0.0) != (error_i1 > 0.0);

            // Error cancellation: use the MORE PERMISSIVE of percentage-based or tolerance-based limit
            let pct_limit = combined_duration * MAX_RESIDUAL_PCT;
            let tolerance_limit = tolerance_secs * 2.0;
            let residual_limit = pct_limit.max(tolerance_limit);
            let errors_cancel = residual < residual_limit;

            let combined_significant = combined_error >= MIN_COMBINED_ERROR_SECS;

            let is_split_failure = significant_errors
                                && opposite_directions
                                && errors_cancel
                                && combined_significant;

            if is_split_failure {
                debug!(
                    "Split failure detected at track {}: errors={:+.2}s/{:+.2}s, residual={:.2}s ({:.1}% of {:.2}s), combined_error={:.2}s",
                    i + 1,
                    error_i,
                    error_i1,
                    residual,
                    (residual / combined_duration) * 100.0,
                    combined_duration,
                    combined_error
                );

                // Attempt to find missing boundary between i and i+1
                let search_start = refined[i]; // Start of track i
                // **[BUG FIX]** Search should extend through BOTH tracks, not just to current boundary
                // If track i is too short, the correct boundary could be anywhere in track i+1's region
                let search_end = if i + 2 < refined.len() {
                    refined[i + 2] // End of track i+1
                } else {
                    audio_samples.len() // Last track pair - search to end of file
                };

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
                        "Refining boundary {}: moving from sample {} to {} (expected: {}) [errors: {:+.2}s/{:+.2}s]",
                        i + 1,
                        refined[i + 1],
                        new_boundary,
                        expected_boundary_samples,
                        error_i,
                        error_i1
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
        let region_size = if end_sample >= start_sample {
            end_sample - start_sample
        } else {
            0
        };
        debug!(
            "Search region too small: {} samples (need at least {})",
            region_size,
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
        // sqrt((0.25 + 0.25 + 0.09 + 0.09) / 4) = sqrt(0.68/4) = sqrt(0.17) ≈ 0.412
        let samples = vec![0.5, -0.5, 0.3, -0.3];
        let rms = calculate_rms(&samples);
        assert!((rms - 0.412).abs() < 0.02, "Expected ~0.412, got {}", rms);
    }

    #[test]
    fn test_calculate_durations() {
        let boundaries = vec![0, 44100, 88200]; // 0s, 1s, 2s at 44.1kHz
        let durations = calculate_durations(&boundaries, 44100.0);
        assert_eq!(durations.len(), 2);
        assert!((durations[0] - 1.0).abs() < 0.001);
        assert!((durations[1] - 1.0).abs() < 0.001);
    }

    /// Helper to create test audio with quiet regions at expected boundaries
    ///
    /// Creates audio that is mostly loud (0.5) with quiet regions (0.001) at
    /// specified boundary positions. This allows `find_local_quiet_spot` to
    /// successfully locate boundaries.
    ///
    /// # Arguments
    /// * `duration_secs` - Total audio duration in seconds
    /// * `sample_rate` - Sample rate in Hz
    /// * `quiet_positions_secs` - Positions (in seconds) where quiet regions should be added
    fn create_test_audio_with_quiet_spots(
        duration_secs: f64,
        sample_rate: f64,
        quiet_positions_secs: &[f64],
    ) -> Vec<f32> {
        let total_samples = (duration_secs * sample_rate) as usize;
        let mut audio = vec![0.5_f32; total_samples]; // Loud audio by default

        // Add quiet regions (±1 second) around each specified position
        for &pos_secs in quiet_positions_secs {
            let center_sample = (pos_secs * sample_rate) as usize;
            let quiet_window_samples = (1.0 * sample_rate) as usize; // ±1 second

            let start = center_sample.saturating_sub(quiet_window_samples);
            let end = (center_sample + quiet_window_samples).min(total_samples);

            for sample in &mut audio[start..end] {
                *sample = 0.001; // Quiet region
            }
        }

        audio
    }

    /// Helper to create uniform loud audio (for tests that don't expect refinement)
    fn create_loud_audio(duration_secs: f64, sample_rate: f64) -> Vec<f32> {
        vec![0.5_f32; (duration_secs * sample_rate) as usize]
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

        // Create audio with quiet spot at expected boundary (200s)
        let audio_samples = create_test_audio_with_quiet_spots(620.0, sample_rate, &[200.0]);

        // Use 11s tolerance so residual limit is 22s, allowing 20s residual to pass
        let tolerance = 11.0;

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

        // Create audio with quiet spot at expected boundary (184s)
        let audio_samples = create_test_audio_with_quiet_spots(467.0, sample_rate, &[184.0]);

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

        let audio_samples = create_loud_audio(598.0, sample_rate);
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

        let audio_samples = create_loud_audio(770.0, sample_rate);
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

        // Create audio with quiet spot at expected boundary (350s = 150 + 200)
        let audio_samples = create_test_audio_with_quiet_spots(530.0, sample_rate, &[350.0]);
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
        // Pattern 1 (forward): Track 0 collapsed to 25s, Track 1 absorbed it (+75s)
        // Pattern 2 (reverse): Track 2 absorbed Track 3 (+100s), Track 3 collapsed to 20s
        // Detected: [ 25s, 225s, 280s,  20s]

        let sample_rate = 44100.0;
        let expected = vec![100.0, 150.0, 180.0, 120.0];

        // Create detected boundaries for two split failures
        let detected_boundaries = vec![
            0,
            (25.0 * sample_rate) as usize,   // Track 0 collapsed: -75s error
            (250.0 * sample_rate) as usize,  // Track 1 absorbed: 25 + 225 = 250, +75s error
            (530.0 * sample_rate) as usize,  // Track 2 absorbed: 250 + 280 = 530, +100s error
            (550.0 * sample_rate) as usize,  // Track 3 collapsed: 530 + 20 = 550, -100s error
        ];

        // Create audio with quiet spots at expected boundaries
        // Expected boundaries: 100s (0+100), 250s (100+150), 430s (100+150+180), 550s (100+150+180+120)
        let audio_samples = create_test_audio_with_quiet_spots(550.0, sample_rate, &[100.0, 430.0]);
        let tolerance = 11.0; // Allow residuals to pass

        let refined = refine_missed_boundaries(
            &detected_boundaries,
            &expected,
            &audio_samples,
            sample_rate,
            tolerance,
        );

        let refined_durations = calculate_durations(&refined, sample_rate);

        // Both boundaries should be refined
        // Track 0 should be closer to 100s
        assert!(
            (refined_durations[0] - 100.0).abs() < (25.0_f64 - 100.0).abs(),
            "Track 0 should be refined closer to 100s (got {:.2}s)",
            refined_durations[0]
        );

        // Track 3 should be closer to 120s
        assert!(
            (refined_durations[3] - 120.0).abs() < (20.0_f64 - 120.0).abs(),
            "Track 3 should be refined closer to 120s (got {:.2}s)",
            refined_durations[3]
        );
    }

    #[test]
    fn test_moderate_split_failure_eagles_case() {
        // Eagles - "The Long Run" tracks 8-9
        // Track 8: expected 223.99s, detected 147.27s (-76.72s)
        // Track 9: expected 138.40s, detected 209.70s (+71.31s)
        // Residual: -5.41s (1.5% of 362.39s combined)

        let sample_rate = 44100.0;
        let expected = vec![223.99, 138.40];

        // Create boundaries for detected durations
        let boundary_1 = (147.27 * sample_rate) as usize;
        let boundary_2 = boundary_1 + (209.70 * sample_rate) as usize;
        let detected_boundaries = vec![0, boundary_1, boundary_2];

        // Calculate total duration and create audio with quiet spot at expected boundary
        let total_duration_secs = boundary_2 as f64 / sample_rate;
        let audio = create_test_audio_with_quiet_spots(
            total_duration_secs,
            sample_rate,
            &[223.99], // Expected boundary position
        );

        let tolerance = 5.0;
        let refined = refine_missed_boundaries(
            &detected_boundaries,
            &expected,
            &audio,
            sample_rate,
            tolerance,
        );

        // Verify refinement occurred
        assert_ne!(refined[1], boundary_1, "Boundary should be refined");

        // Verify new boundary near expected
        let refined_pos_secs = refined[1] as f64 / sample_rate;
        assert!(
            (refined_pos_secs - 223.99).abs() < 5.0_f64,
            "Refined boundary should be within 5s of expected (223.99s), got {:.2}s",
            refined_pos_secs
        );
    }

    #[test]
    fn test_no_detection_insufficient_error_cancellation() {
        // Errors significant but don't cancel (10s residual on 360s = 2.8%)
        let sample_rate = 44100.0;
        let expected = vec![220.0, 140.0];

        // Track 1: -70s error, Track 2: +60s error = 10s residual (2.8%)
        let boundary_1 = (150.0 * sample_rate) as usize;  // 220 - 70
        let boundary_2 = boundary_1 + (200.0 * sample_rate) as usize;  // 140 + 60
        let detected_boundaries = vec![0, boundary_1, boundary_2];

        let audio = vec![0.1_f32; boundary_2];
        let tolerance = 5.0;

        let refined = refine_missed_boundaries(
            &detected_boundaries,
            &expected,
            &audio,
            sample_rate,
            tolerance,
        );

        // Should NOT refine (residual 10s > 2% of 360s = 7.2s)
        assert_eq!(refined[1], boundary_1, "Should not refine - residual too high");
    }
}
