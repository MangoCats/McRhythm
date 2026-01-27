//! Stage 6: Boundary Refinement Helper Functions
//!
//! Pattern detection and refinement strategies for fine-tuning boundary positions
//! within matched MusicBrainz editions.

use crate::matching::types::MatchedTrack;

/// Cascade pattern (2+ consecutive tracks with >30s errors)
#[derive(Debug)]
pub struct CascadePattern {
    pub start_track: usize,
    pub count: usize,
}

/// Complementary error pair (one track over, next under by similar amount)
#[derive(Debug)]
pub struct ComplementaryPair {
    pub track_index: usize,
    pub error1: f64,
    pub error2: f64,
}

/// Detect cascade patterns in track errors
///
/// A cascade occurs when 2+ consecutive tracks have >30s timing errors.
/// This indicates one misplaced boundary affecting multiple tracks.
pub fn detect_cascade_patterns(tracks: &[MatchedTrack]) -> Vec<CascadePattern> {
    let mut patterns = Vec::new();
    let mut i = 0;

    while i < tracks.len() {
        if tracks[i].timing_error > 30.0 {
            // Start of potential cascade
            let start = i;
            let mut count = 1;

            // Count consecutive tracks with >30s errors
            while i + 1 < tracks.len() && tracks[i + 1].timing_error > 30.0 {
                count += 1;
                i += 1;
            }

            // Cascade requires at least 2 consecutive tracks
            if count >= 2 {
                patterns.push(CascadePattern {
                    start_track: start,
                    count,
                });
            }
        }
        i += 1;
    }

    patterns
}

/// Detect complementary error pairs
///
/// A complementary pair occurs when one track is over-allocated and the next
/// is under-allocated by a similar amount, indicating a misplaced boundary.
pub fn detect_complementary_pairs(tracks: &[MatchedTrack]) -> Vec<ComplementaryPair> {
    let mut pairs = Vec::new();

    for i in 0..tracks.len().saturating_sub(1) {
        let detected1 = tracks[i].detected_duration;
        let expected1 = tracks[i].expected_duration;
        let error1 = detected1 - expected1;

        let detected2 = tracks[i + 1].detected_duration;
        let expected2 = tracks[i + 1].expected_duration;
        let error2 = detected2 - expected2;

        // Check for complementary pattern: one over, next under by similar amount
        if error1 > 30.0 && error2 < -30.0 {
            let magnitude_diff = (error1.abs() - error2.abs()).abs();
            if magnitude_diff < 20.0 {
                pairs.push(ComplementaryPair {
                    track_index: i,
                    error1,
                    error2,
                });
            }
        } else if error1 < -30.0 && error2 > 30.0 {
            let magnitude_diff = (error1.abs() - error2.abs()).abs();
            if magnitude_diff < 20.0 {
                pairs.push(ComplementaryPair {
                    track_index: i,
                    error1,
                    error2,
                });
            }
        }
    }

    pairs
}

/// Refine cascade pattern by searching for better boundary positions
///
/// Searches ±60s window around expected boundary positions to find
/// better low-energy spots for each boundary in the cascade.
pub fn refine_cascade_pattern(
    cascade: &CascadePattern,
    boundaries: &[usize],
    expected_durations: &[f64],
    audio_energy: &[f32],
    sample_rate: u32,
) -> Option<Vec<usize>> {
    let mut refined = boundaries.to_vec();

    // Calculate expected boundary positions
    let expected_cumulative: Vec<f64> = expected_durations
        .iter()
        .scan(0.0, |acc, &dur| {
            *acc += dur;
            Some(*acc)
        })
        .collect();

    // Refine each boundary in cascade region
    for i in cascade.start_track..(cascade.start_track + cascade.count) {
        if i >= expected_cumulative.len() {
            break;
        }

        let expected_sample = (expected_cumulative[i] * sample_rate as f64) as usize;
        let search_window_samples = (60.0 * sample_rate as f64) as usize; // ±60s

        if let Some(better_boundary) =
            find_energy_minimum(audio_energy, expected_sample, search_window_samples, sample_rate)
        {
            refined[i + 1] = better_boundary;
        }
    }

    Some(refined)
}

/// Refine complementary pair by finding better boundary between the two tracks
pub fn refine_complementary_pair(
    pair: &ComplementaryPair,
    boundaries: &[usize],
    expected_durations: &[f64],
    audio_energy: &[f32],
    sample_rate: u32,
) -> Option<Vec<usize>> {
    let mut refined = boundaries.to_vec();

    // Calculate expected boundary position between the two tracks
    let expected_boundary = expected_durations[0..=pair.track_index].iter().sum::<f64>();
    let expected_sample = (expected_boundary * sample_rate as f64) as usize;

    // Search ±40s window for better boundary
    let search_window_samples = (40.0 * sample_rate as f64) as usize;

    if let Some(better_boundary) =
        find_energy_minimum(audio_energy, expected_sample, search_window_samples, sample_rate)
    {
        refined[pair.track_index + 1] = better_boundary;
    }

    Some(refined)
}

/// Find energy minimum within search window
///
/// Searches for the lowest RMS energy position within ±search_window_samples
/// of the target position.
fn find_energy_minimum(
    audio_energy: &[f32],
    target_sample: usize,
    search_window_samples: usize,
    sample_rate: u32,
) -> Option<usize> {
    // Energy envelope is 100ms windows
    let window_size_samples = (sample_rate as f64 * 0.1) as usize; // 100ms

    // Convert target and window to energy indices
    let target_idx = target_sample / window_size_samples;
    let search_window_indices = search_window_samples / window_size_samples;

    let start_idx = target_idx.saturating_sub(search_window_indices);
    let end_idx = (target_idx + search_window_indices).min(audio_energy.len());

    if start_idx >= end_idx || audio_energy.is_empty() {
        return None;
    }

    // Find minimum energy in search window
    let (min_idx, _min_energy) = audio_energy[start_idx..end_idx]
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))?;

    // Convert back to sample position
    Some((start_idx + min_idx) * window_size_samples)
}

/// Validate that boundary refinement improves full-album match
///
/// Compares before/after by counting tracks within tolerance.
/// Accepts refinement ONLY if:
/// - More tracks are within tolerance, OR
/// - Same tracks within tolerance but mean error decreases significantly
pub fn validate_full_album_improvement(
    original_boundaries: &[usize],
    refined_boundaries: &[usize],
    expected_durations: &[f64],
    sample_rate: u32,
    tolerance_secs: f64,
) -> bool {
    // Calculate original errors
    let original_within = count_tracks_within_tolerance(
        original_boundaries,
        expected_durations,
        sample_rate,
        tolerance_secs,
    );

    // Calculate refined errors
    let refined_within = count_tracks_within_tolerance(
        refined_boundaries,
        expected_durations,
        sample_rate,
        tolerance_secs,
    );

    // Accept if more tracks within tolerance
    if refined_within > original_within {
        return true;
    }

    // If same tracks within tolerance, check mean error improvement
    if refined_within == original_within {
        let original_mean = calculate_mean_error(original_boundaries, expected_durations, sample_rate);
        let refined_mean = calculate_mean_error(refined_boundaries, expected_durations, sample_rate);

        // Accept if mean error decreases by at least 2 seconds
        if refined_mean < original_mean - 2.0 {
            return true;
        }
    }

    false
}

/// Count tracks within tolerance
pub fn count_tracks_within_tolerance(
    boundaries: &[usize],
    expected_durations: &[f64],
    sample_rate: u32,
    tolerance_secs: f64,
) -> usize {
    let durations = crate::matching::boundaries_to_durations(boundaries, sample_rate as f64);

    durations
        .iter()
        .zip(expected_durations.iter())
        .filter(|(&detected, &expected)| (detected - expected).abs() <= tolerance_secs)
        .count()
}

/// Calculate mean absolute error
fn calculate_mean_error(
    boundaries: &[usize],
    expected_durations: &[f64],
    sample_rate: u32,
) -> f64 {
    let durations = crate::matching::boundaries_to_durations(boundaries, sample_rate as f64);

    let count = durations.len().min(expected_durations.len());
    if count == 0 {
        return 0.0;
    }

    let total_error: f64 = durations
        .iter()
        .zip(expected_durations.iter())
        .map(|(&detected, &expected)| (detected - expected).abs())
        .sum();

    total_error / count as f64
}
