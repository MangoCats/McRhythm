//! Stage 7: Progressive RMS Boundary Refinement
//!
//! Engages when at least 2 tracks have duration errors ≥30 seconds after earlier stages.
//! Uses a three-stage progressive RMS energy scan to precisely locate track boundaries.
//!
//! ## Algorithm
//!
//! 1. **Identify Problem Range**: First and last tracks with error >10s
//! 2. **Progressive RMS Scan** for each problem boundary:
//!    - Coarse: 10% blocks in 1% increments (100 scans)
//!    - Medium: 1% blocks in 0.1% increments within winning coarse block (100 scans)
//!    - Fine: 0.1s increments within winning medium block
//! 3. **Validation**: Accept only if total absolute error improves

use tracing::{debug, info};

/// Apply Stage 7 progressive refinement if needed
///
/// # Arguments
/// * `detected_boundaries` - Current best boundaries from earlier stages
/// * `expected_durations` - Expected track durations from MBID edition (seconds)
/// * `audio_samples` - Raw audio sample data
/// * `sample_rate` - Sample rate in Hz
///
/// # Returns
/// Refined boundaries if improvement found, otherwise original boundaries
pub fn apply_stage7_if_needed(
    detected_boundaries: &[usize],
    expected_durations: &[f64],
    audio_samples: &[f32],
    sample_rate: f64,
) -> Vec<usize> {
    // Check engagement criteria: at least 2 tracks with error ≥30s
    let errors = calculate_track_errors(detected_boundaries, expected_durations, sample_rate);
    let severe_errors = errors.iter().filter(|&&e| e.abs() >= 30.0).count();

    if severe_errors < 2 {
        debug!("Stage 7 not needed: only {} tracks with ≥30s error", severe_errors);
        return detected_boundaries.to_vec();
    }

    info!("Stage 7 ENGAGED: {} tracks with ≥30s error", severe_errors);

    // Identify problem range
    let (first_problem, last_problem) = identify_problem_range(&errors);

    if first_problem.is_none() || last_problem.is_none() {
        debug!("Stage 7: No problem range identified");
        return detected_boundaries.to_vec();
    }

    let first_problem = first_problem.unwrap();
    let last_problem = last_problem.unwrap();

    info!(
        "Stage 7: Problem range tracks {}-{} ({} tracks, {} boundaries)",
        first_problem + 1,
        last_problem + 1,
        last_problem - first_problem + 1,
        last_problem - first_problem
    );

    // Refine problem boundaries
    let refined = refine_problem_boundaries(
        detected_boundaries,
        expected_durations,
        audio_samples,
        sample_rate,
        first_problem,
        last_problem,
    );

    // Validate improvement
    let original_error = calculate_total_absolute_error(detected_boundaries, expected_durations, sample_rate);
    let refined_error = calculate_total_absolute_error(&refined, expected_durations, sample_rate);

    if refined_error < original_error {
        let improvement = original_error - refined_error;
        info!(
            "Stage 7 ACCEPTED: Total error {:.2}s → {:.2}s (improvement: {:.2}s)",
            original_error,
            refined_error,
            improvement
        );
        refined
    } else {
        info!(
            "Stage 7 REJECTED: No improvement ({:.2}s → {:.2}s)",
            original_error,
            refined_error
        );
        detected_boundaries.to_vec()
    }
}

/// Calculate per-track duration errors
fn calculate_track_errors(
    boundaries: &[usize],
    expected_durations: &[f64],
    sample_rate: f64,
) -> Vec<f64> {
    let mut errors = Vec::new();

    for i in 0..expected_durations.len() {
        if i + 1 >= boundaries.len() {
            break;
        }

        let detected_duration = (boundaries[i + 1] - boundaries[i]) as f64 / sample_rate;
        let error = detected_duration - expected_durations[i];
        errors.push(error);
    }

    errors
}

/// Identify first and last problem tracks (error >10s)
fn identify_problem_range(errors: &[f64]) -> (Option<usize>, Option<usize>) {
    let mut first_problem = None;
    let mut last_problem = None;

    // Find first problem track (from start)
    for (i, &error) in errors.iter().enumerate() {
        if error.abs() > 10.0 {
            first_problem = Some(i);
            break;
        }
    }

    // Find last problem track (from end)
    for (i, &error) in errors.iter().enumerate().rev() {
        if error.abs() > 10.0 {
            last_problem = Some(i);
            break;
        }
    }

    (first_problem, last_problem)
}

/// Refine all boundaries in the problem range
fn refine_problem_boundaries(
    boundaries: &[usize],
    expected_durations: &[f64],
    audio_samples: &[f32],
    sample_rate: f64,
    first_problem: usize,
    last_problem: usize,
) -> Vec<usize> {
    let mut refined = boundaries.to_vec();

    // Start from first problem track's beginning
    let problem_start = boundaries[first_problem];

    // Refine each boundary in problem range
    for track_idx in first_problem..=last_problem {
        // Calculate expected boundary position
        let expected_offset: f64 = expected_durations[first_problem..=track_idx].iter().sum();
        let expected_boundary = problem_start + (expected_offset * sample_rate) as usize;

        let boundary_idx = track_idx + 1;
        if boundary_idx >= refined.len() {
            break;
        }

        info!(
            "Stage 7: Refining boundary {} (between tracks {} and {})",
            boundary_idx,
            track_idx + 1,
            track_idx + 2
        );

        // Progressive RMS scan
        if let Some(new_boundary) = progressive_rms_scan(
            audio_samples,
            expected_boundary,
            sample_rate,
            track_idx + 1,
        ) {
            debug!(
                "Stage 7: Boundary {} moved from {} to {} (expected: {})",
                boundary_idx,
                refined[boundary_idx],
                new_boundary,
                expected_boundary
            );
            refined[boundary_idx] = new_boundary;
        }
    }

    refined
}

/// Progressive three-stage RMS energy scan
///
/// # Arguments
/// * `audio_samples` - Audio sample data
/// * `expected_boundary` - Expected boundary position (samples)
/// * `sample_rate` - Sample rate in Hz
/// * `track_num` - Track number for logging
///
/// # Returns
/// Optimal boundary position, or None if search failed
fn progressive_rms_scan(
    audio_samples: &[f32],
    expected_boundary: usize,
    sample_rate: f64,
    track_num: usize,
) -> Option<usize> {
    // Search window: ±30 seconds
    const SEARCH_WINDOW_SECS: f64 = 30.0;
    let window_samples = (SEARCH_WINDOW_SECS * sample_rate) as usize;

    let search_start = expected_boundary.saturating_sub(window_samples);
    let search_end = (expected_boundary + window_samples).min(audio_samples.len());
    let search_range = search_end - search_start;

    if search_range < (sample_rate * 6.0) as usize {
        debug!(
            "Stage 7 (track {}): Search range too small ({:.1}s)",
            track_num,
            search_range as f64 / sample_rate
        );
        return None;
    }

    debug!(
        "Stage 7 (track {}): Search window [{}, {}] ({:.1}s range)",
        track_num,
        search_start,
        search_end,
        search_range as f64 / sample_rate
    );

    // Stage 1: Coarse scan (10% blocks in 1% increments)
    let coarse_block_size = search_range / 10; // 10% of range
    let coarse_step = search_range / 100; // 1% of range

    let coarse_winner = find_lowest_rms_block(
        audio_samples,
        search_start,
        search_end,
        coarse_block_size,
        coarse_step,
    );

    debug!(
        "Stage 7 (track {}): Coarse scan winner at sample {} (RMS: {:.6})",
        track_num,
        coarse_winner.position,
        coarse_winner.rms
    );

    // Stage 2: Medium scan (10% blocks in 1% increments within coarse winner)
    let medium_search_start = coarse_winner.position;
    let medium_search_end = (coarse_winner.position + coarse_block_size).min(audio_samples.len());
    let medium_range = medium_search_end - medium_search_start;

    let medium_block_size = medium_range / 10; // 10% of 6s coarse block = 600ms
    let medium_step = medium_range / 100; // 1% of 6s coarse block = 60ms

    let medium_winner = find_lowest_rms_block(
        audio_samples,
        medium_search_start,
        medium_search_end,
        medium_block_size,
        medium_step,
    );

    debug!(
        "Stage 7 (track {}): Medium scan winner at sample {} (RMS: {:.6})",
        track_num,
        medium_winner.position,
        medium_winner.rms
    );

    // Stage 3: Fine scan (100ms windows in 10ms increments within medium winner)
    let fine_search_start = medium_winner.position;
    let fine_search_end = (medium_winner.position + medium_block_size).min(audio_samples.len());
    let fine_window_size = (0.1 * sample_rate) as usize; // 100ms measurement window
    let fine_step = (0.01 * sample_rate) as usize; // 10ms step size

    let fine_winner = find_lowest_rms_point(
        audio_samples,
        fine_search_start,
        fine_search_end,
        fine_window_size,
        fine_step,
    );

    // Return the CENTER of the 100ms winner block
    let boundary_position = fine_winner.position + (fine_window_size / 2);

    info!(
        "Stage 7 (track {}): Fine scan winner at sample {} (center: {}, RMS: {:.6})",
        track_num,
        fine_winner.position,
        boundary_position,
        fine_winner.rms
    );

    Some(boundary_position)
}

/// Result from RMS scan
#[derive(Debug, Clone)]
struct ScanResult {
    position: usize,
    rms: f32,
}

/// Find block with lowest total RMS energy
fn find_lowest_rms_block(
    audio_samples: &[f32],
    search_start: usize,
    search_end: usize,
    block_size: usize,
    step_size: usize,
) -> ScanResult {
    let mut min_rms = f32::MAX;
    let mut min_positions = Vec::new();

    let mut pos = search_start;
    while pos + block_size <= search_end {
        let block_end = (pos + block_size).min(audio_samples.len());
        let rms = calculate_rms(&audio_samples[pos..block_end]);

        if rms < min_rms - 1e-8 {
            // New minimum found
            min_rms = rms;
            min_positions.clear();
            min_positions.push(pos);
        } else if (rms - min_rms).abs() < 1e-8 {
            // Tie - collect this position
            min_positions.push(pos);
        }

        pos += step_size.max(1);
    }

    // If tie, use middle position
    let winner_pos = if min_positions.len() > 1 {
        min_positions[min_positions.len() / 2]
    } else if !min_positions.is_empty() {
        min_positions[0]
    } else {
        search_start
    };

    ScanResult {
        position: winner_pos,
        rms: min_rms,
    }
}

/// Find point with lowest RMS energy (fine scan)
fn find_lowest_rms_point(
    audio_samples: &[f32],
    search_start: usize,
    search_end: usize,
    window_size: usize,
    step_size: usize,
) -> ScanResult {
    let mut min_rms = f32::MAX;
    let mut min_positions = Vec::new();

    let mut pos = search_start;
    while pos + window_size <= search_end {
        let window_end = pos + window_size;
        if window_end > audio_samples.len() {
            break;
        }

        let rms = calculate_rms(&audio_samples[pos..window_end]);

        if rms < min_rms - 1e-8 {
            min_rms = rms;
            min_positions.clear();
            min_positions.push(pos);
        } else if (rms - min_rms).abs() < 1e-8 {
            min_positions.push(pos);
        }

        pos += step_size.max(1);
    }

    // If tie, use middle position
    let winner_pos = if min_positions.len() > 1 {
        min_positions[min_positions.len() / 2]
    } else if !min_positions.is_empty() {
        min_positions[0]
    } else {
        search_start
    };

    ScanResult {
        position: winner_pos,
        rms: min_rms,
    }
}

/// Calculate RMS (Root Mean Square) of audio samples
fn calculate_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_squares: f32 = samples.iter().map(|&s| s * s).sum();
    (sum_squares / samples.len() as f32).sqrt()
}

/// Calculate total absolute error across all tracks
fn calculate_total_absolute_error(
    boundaries: &[usize],
    expected_durations: &[f64],
    sample_rate: f64,
) -> f64 {
    let mut total_error = 0.0;

    for i in 0..expected_durations.len() {
        if i + 1 >= boundaries.len() {
            break;
        }

        let detected_duration = (boundaries[i + 1] - boundaries[i]) as f64 / sample_rate;
        let error = (detected_duration - expected_durations[i]).abs();
        total_error += error;
    }

    total_error
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identify_problem_range() {
        // Errors: [2.0, 5.0, -35.0, 50.0, -40.0, 8.0, 3.0]
        let errors = vec![2.0, 5.0, -35.0, 50.0, -40.0, 8.0, 3.0];
        let (first, last) = identify_problem_range(&errors);

        assert_eq!(first, Some(2)); // First track with |error| > 10s
        assert_eq!(last, Some(4)); // Last track with |error| > 10s
    }

    #[test]
    fn test_no_problem_range() {
        let errors = vec![2.0, -5.0, 8.0, -3.0];
        let (first, last) = identify_problem_range(&errors);

        assert_eq!(first, None);
        assert_eq!(last, None);
    }

    #[test]
    fn test_calculate_track_errors() {
        let sample_rate = 44100.0;
        let boundaries = vec![
            0,
            (100.0 * sample_rate) as usize,
            (210.0 * sample_rate) as usize,
        ];
        let expected = vec![100.0, 100.0];

        let errors = calculate_track_errors(&boundaries, &expected, sample_rate);

        assert_eq!(errors.len(), 2);
        assert!((errors[0] - 0.0).abs() < 0.01); // First track correct
        assert!((errors[1] - 10.0).abs() < 0.01); // Second track +10s
    }
}
