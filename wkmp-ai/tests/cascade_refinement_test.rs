//! Cascade Boundary Refinement Test
//!
//! Tests MusicBrainz-guided boundary refinement on albums with cascade errors.
//!
//! Cascade errors occur when one misplaced boundary causes a chain reaction of
//! misaligned tracks. This algorithm detects and repairs such patterns.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use wkmp_ai::services::MusicBrainzClient;
use wkmp_ai::workflow::{FileAudioData, PassageBoundary};

/// Cascade region (consecutive tracks with >15s error)
#[derive(Debug)]
struct CascadeRegion {
    start_track: usize, // First track index in cascade
    end_track: usize,   // Last track index in cascade
}

/// Detected boundaries and durations for comparison
#[derive(Debug, Clone)]
struct BoundarySet {
    boundaries: Vec<f64>, // Boundary times in seconds (includes file start and end)
    durations: Vec<f64>,  // Track durations in seconds
}

impl BoundarySet {
    fn from_passage_boundaries(boundaries: &[PassageBoundary], sample_rate: u32) -> Self {
        let tick_rate: f64 = 28_224_000.0;
        let mut boundary_times = vec![0.0]; // File starts at 0

        for b in boundaries {
            let end_seconds = (b.end_time as f64 / tick_rate) as f64;
            boundary_times.push(end_seconds);
        }

        // Calculate durations from boundaries
        let mut durations = Vec::new();
        for i in 0..boundary_times.len() - 1 {
            durations.push(boundary_times[i + 1] - boundary_times[i]);
        }

        BoundarySet {
            boundaries: boundary_times,
            durations,
        }
    }

    fn to_passage_boundaries(&self, sample_rate: u32) -> Vec<PassageBoundary> {
        let tick_rate: f64 = 28_224_000.0;
        let mut passages = Vec::new();

        for i in 0..self.durations.len() {
            let start_ticks = (self.boundaries[i] * tick_rate) as i64;
            let end_ticks = (self.boundaries[i + 1] * tick_rate) as i64;

            passages.push(PassageBoundary {
                start_time: start_ticks,
                end_time: end_ticks,
                confidence: 0.8, // Higher confidence for refined boundaries
            });
        }

        passages
    }
}

/// Detect cascade patterns (2+ consecutive tracks with >30s error)
/// STRICTER threshold (30s instead of 15s) to reduce false positives
fn detect_cascade_patterns(
    detected_durations: &[f64],
    expected_durations: &[f64],
) -> Vec<CascadeRegion> {
    if detected_durations.len() != expected_durations.len() {
        // Track count mismatch - can't detect cascades
        return Vec::new();
    }

    let mut cascades = Vec::new();
    let mut in_cascade = false;
    let mut cascade_start = 0;

    for i in 0..detected_durations.len() {
        let error = (detected_durations[i] - expected_durations[i]).abs();

        // STRICTER: 30s threshold to identify only severe cascades
        if error > 30.0 {
            // Large error - potential cascade member
            if !in_cascade {
                in_cascade = true;
                cascade_start = i;
            }
        } else {
            // Good track - end cascade if one was in progress
            if in_cascade {
                // Require 2+ consecutive errors for cascade
                if i - cascade_start >= 2 {
                    cascades.push(CascadeRegion {
                        start_track: cascade_start,
                        end_track: i - 1,
                    });
                }
                in_cascade = false;
            }
        }
    }

    // Handle cascade at end of album
    if in_cascade && detected_durations.len() - cascade_start >= 2 {
        cascades.push(CascadeRegion {
            start_track: cascade_start,
            end_track: detected_durations.len() - 1,
        });
    }

    cascades
}

/// Find best boundary in search window using energy dips
/// Returns None if no confident boundary found (energy confidence check)
fn find_best_boundary_in_window(
    audio_energy: &[f32],
    sample_rate: u32,
    search_start_sec: f64,
    search_end_sec: f64,
) -> Option<f64> {
    let start_sample = (search_start_sec * sample_rate as f64) as usize;
    let end_sample = (search_end_sec * sample_rate as f64) as usize;

    if start_sample >= audio_energy.len() || end_sample > audio_energy.len() || start_sample >= end_sample {
        return None;
    }

    // Find minimum energy point in window (likely boundary)
    let window = &audio_energy[start_sample..end_sample];
    let (min_offset, &min_energy) = window
        .iter()
        .enumerate()
        .min_by(|a, b| a.1.partial_cmp(b.1).unwrap())?;

    // Calculate mean energy in window
    let mean_energy: f32 = window.iter().sum::<f32>() / window.len() as f32;

    // CONFIDENCE CHECK: Only accept if minimum is significantly below mean
    // (i.e., there's a clear silence/quiet region indicating a real boundary)
    let energy_ratio = if mean_energy > 0.0 {
        min_energy / mean_energy
    } else {
        1.0
    };

    if energy_ratio > 0.5 {
        // Minimum isn't significantly quieter than average - low confidence boundary
        tracing::debug!(
            "Rejecting boundary candidate at {:.1}s: energy ratio {:.2} (min/mean) - not a clear boundary",
            (start_sample + min_offset) as f64 / sample_rate as f64,
            energy_ratio
        );
        return None;
    }

    tracing::debug!(
        "Accepting boundary candidate at {:.1}s: energy ratio {:.2} (min/mean) - clear energy dip",
        (start_sample + min_offset) as f64 / sample_rate as f64,
        energy_ratio
    );

    let best_sample = start_sample + min_offset;
    Some(best_sample as f64 / sample_rate as f64)
}

/// Calculate errors for specific tracks
fn calculate_errors_for_tracks(
    detected_durations: &[f64],
    expected_durations: &[f64],
    track_indices: &[usize],
) -> Vec<f64> {
    track_indices
        .iter()
        .map(|&i| detected_durations[i] - expected_durations[i])
        .collect()
}

/// Log detailed track-by-track comparison
fn log_track_comparison(
    track_names: &[String],
    expected_durations: &[f64],
    original_durations: &[f64],
    refined_durations: &[f64],
    cascade_tracks: &[usize],
) {
    tracing::info!("");
    tracing::info!("=== Track-by-Track Comparison (Cascade Region) ===");
    tracing::info!("");
    tracing::info!(
        "{:<4} {:<40} {:>10} {:>10} {:>12} {:>12} {:>10} {:>4}",
        "Trk", "Title", "Expected", "Original", "Refined", "Orig Err", "New Err", "Chg"
    );
    tracing::info!("{}", "-".repeat(115));

    for &i in cascade_tracks {
        let title = track_names.get(i).map(|s| s.as_str()).unwrap_or("Unknown");
        let title_short = if title.len() > 40 {
            format!("{}...", &title[..37])
        } else {
            title.to_string()
        };

        let expected = expected_durations[i];
        let original = original_durations[i];
        let refined = refined_durations[i];

        let orig_error = original - expected;
        let new_error = refined - expected;

        let orig_status = if orig_error.abs() <= 10.0 { "✓" } else { "✗" };
        let new_status = if new_error.abs() <= 10.0 { "✓" } else { "✗" };

        let change = if new_error.abs() < orig_error.abs() - 1.0 {
            "↑" // Better
        } else if new_error.abs() > orig_error.abs() + 1.0 {
            "↓" // Worse
        } else {
            "=" // Same
        };

        tracing::info!(
            "{:<4} {:<40} {:>8.1}s {:>8.1}s{} {:>8.1}s{} {:>+9.1}s {:>+9.1}s {:>4}",
            i + 1,
            title_short,
            expected,
            original,
            orig_status,
            refined,
            new_status,
            orig_error,
            new_error,
            change
        );
    }

    tracing::info!("{}", "-".repeat(115));
    tracing::info!("");
}

/// Refine boundaries for a single cascade region (returns refined boundaries, doesn't modify in-place)
fn refine_cascade_region(
    cascade: &CascadeRegion,
    boundaries: &BoundarySet,
    expected_durations: &[f64],
    audio_energy: &[f32],
    sample_rate: u32,
) -> BoundarySet {
    let mut refined = boundaries.clone();
    let region_start_boundary = refined.boundaries[cascade.start_track];

    for i in cascade.start_track..=cascade.end_track {
        // Expected boundary based on MusicBrainz durations from cascade start
        let cumulative_duration: f64 = expected_durations[cascade.start_track..=i].iter().sum();
        let expected_boundary = region_start_boundary + cumulative_duration;

        // Wider search for cascades: ±60s window
        let search_start = (expected_boundary - 60.0).max(region_start_boundary);
        let search_end = expected_boundary + 60.0;

        if let Some(better_boundary) =
            find_best_boundary_in_window(audio_energy, sample_rate, search_start, search_end)
        {
            refined.boundaries[i + 1] = better_boundary;
        } else {
            tracing::debug!("No confident boundary found for track {}, keeping original", i + 1);
        }
    }

    // Recalculate durations for refined region
    for i in cascade.start_track..=cascade.end_track {
        refined.durations[i] = refined.boundaries[i + 1] - refined.boundaries[i];
    }

    refined
}

/// Calculate RMS energy for audio samples (100ms windows)
fn calculate_energy_envelope(samples: &[f32], sample_rate: u32) -> Vec<f32> {
    let window_size = (sample_rate as f64 * 0.1) as usize; // 100ms windows
    let mut energy = Vec::new();

    for chunk in samples.chunks(window_size) {
        let rms: f32 = (chunk.iter().map(|s| s * s).sum::<f32>() / chunk.len() as f32).sqrt();
        energy.push(rms);
    }

    // Expand to per-sample resolution for easier indexing
    let mut per_sample_energy = Vec::with_capacity(samples.len());
    for &rms in &energy {
        for _ in 0..window_size {
            per_sample_energy.push(rms);
        }
    }

    // Pad to match sample count
    while per_sample_energy.len() < samples.len() {
        per_sample_energy.push(*energy.last().unwrap_or(&0.0));
    }

    per_sample_energy.truncate(samples.len());
    per_sample_energy
}

/// Complementary error pair (one track over-allocated, next track under-allocated)
#[derive(Debug)]
struct ComplementaryPair {
    track_index: usize, // Index of first track in the pair
    error1: f64,        // Error for first track (positive)
    error2: f64,        // Error for second track (negative)
}

/// Detect complementary error pairs where boundary is misplaced between two tracks
/// Pattern: track[i] has +N seconds error, track[i+1] has -N seconds error (similar magnitude)
fn detect_complementary_pairs(
    detected_durations: &[f64],
    expected_durations: &[f64],
) -> Vec<ComplementaryPair> {
    if detected_durations.len() != expected_durations.len() || detected_durations.len() < 2 {
        return Vec::new();
    }

    let mut pairs = Vec::new();

    for i in 0..detected_durations.len() - 1 {
        let error1 = detected_durations[i] - expected_durations[i];
        let error2 = detected_durations[i + 1] - expected_durations[i + 1];

        // Check for complementary pattern:
        // 1. First track over-allocated (>30s)
        // 2. Second track under-allocated (<-30s)
        // 3. Similar magnitude (within 20s)
        if error1 > 30.0 && error2 < -30.0 {
            let magnitude_diff = (error1.abs() - error2.abs()).abs();
            if magnitude_diff < 20.0 {
                pairs.push(ComplementaryPair {
                    track_index: i,
                    error1,
                    error2,
                });
            }
        }
        // Also check reverse pattern (under then over)
        else if error1 < -30.0 && error2 > 30.0 {
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

/// Refine a single complementary pair by finding better boundary between the two tracks
fn refine_complementary_pair(
    pair: &ComplementaryPair,
    boundaries: &BoundarySet,
    expected_durations: &[f64],
    audio_energy: &[f32],
    sample_rate: u32,
) -> Option<BoundarySet> {
    let mut refined = boundaries.clone();

    // Calculate expected boundary position based on MusicBrainz durations
    let expected_boundary = boundaries.boundaries[0]
        + expected_durations[0..=pair.track_index].iter().sum::<f64>();

    // Search window: ±40s around expected boundary
    let search_start = (expected_boundary - 40.0).max(boundaries.boundaries[pair.track_index]);
    let search_end = (expected_boundary + 40.0).min(boundaries.boundaries[pair.track_index + 2]);

    // Find best boundary using energy minimum detection
    if let Some(better_boundary) =
        find_best_boundary_in_window(audio_energy, sample_rate, search_start, search_end)
    {
        // Update boundary
        refined.boundaries[pair.track_index + 1] = better_boundary;

        // Recalculate durations for affected tracks
        refined.durations[pair.track_index] =
            refined.boundaries[pair.track_index + 1] - refined.boundaries[pair.track_index];
        refined.durations[pair.track_index + 1] =
            refined.boundaries[pair.track_index + 2] - refined.boundaries[pair.track_index + 1];

        Some(refined)
    } else {
        None
    }
}

/// Apply MusicBrainz-guided cascade refinement with per-cascade validation
/// ONLY accepts refinements that improve match quality
fn refine_boundaries_with_mb_hints(
    file_audio: &FileAudioData,
    expected_durations: Vec<f64>, // MusicBrainz durations in seconds
    track_names: Vec<String>,     // Track names for logging
) -> Vec<PassageBoundary> {
    let original_boundaries = BoundarySet::from_passage_boundaries(&file_audio.boundaries, file_audio.sample_rate);

    // Only refine if track counts match
    if original_boundaries.durations.len() != expected_durations.len() {
        tracing::warn!(
            "Track count mismatch: detected {}, expected {}. Skipping refinement.",
            original_boundaries.durations.len(),
            expected_durations.len()
        );
        return file_audio.boundaries.clone();
    }

    // Detect cascade patterns (2+ consecutive tracks with >30s error)
    let cascades = detect_cascade_patterns(&original_boundaries.durations, &expected_durations);

    if cascades.is_empty() {
        tracing::info!("No cascade patterns detected (>30s error threshold). Boundaries unchanged.");
        return file_audio.boundaries.clone();
    }

    tracing::info!("Detected {} cascade region(s) with >30s errors", cascades.len());
    for cascade in &cascades {
        tracing::info!(
            "  Cascade region: tracks {}-{} ({} tracks)",
            cascade.start_track + 1,
            cascade.end_track + 1,
            cascade.end_track - cascade.start_track + 1
        );
    }

    // Calculate energy envelope for boundary search
    let audio_energy = calculate_energy_envelope(&file_audio.samples, file_audio.sample_rate);

    // Start with original boundaries, selectively apply refinements
    let mut best_boundaries = original_boundaries.clone();
    let mut accepted_count = 0;
    let mut rejected_count = 0;

    // Try refining each cascade independently, only accept if it improves things
    for (cascade_idx, cascade) in cascades.iter().enumerate() {
        tracing::info!("");
        tracing::info!(
            "--- Evaluating Cascade #{} (tracks {}-{}) ---",
            cascade_idx + 1,
            cascade.start_track + 1,
            cascade.end_track + 1
        );

        // Apply refinement to this cascade
        let test_boundaries = refine_cascade_region(cascade, &best_boundaries, &expected_durations, &audio_energy, file_audio.sample_rate);

        // Calculate errors for CASCADE REGION (for detailed logging)
        let cascade_track_indices: Vec<usize> = (cascade.start_track..=cascade.end_track).collect();

        let cascade_original_errors = calculate_errors_for_tracks(
            &best_boundaries.durations,
            &expected_durations,
            &cascade_track_indices,
        );

        let cascade_refined_errors = calculate_errors_for_tracks(
            &test_boundaries.durations,
            &expected_durations,
            &cascade_track_indices,
        );

        // Log detailed comparison for cascade region
        log_track_comparison(
            &track_names,
            &expected_durations,
            &best_boundaries.durations,
            &test_boundaries.durations,
            &cascade_track_indices,
        );

        // Calculate errors for ALL TRACKS (for accept/reject decision)
        let all_track_indices: Vec<usize> = (0..expected_durations.len()).collect();

        let all_original_errors = calculate_errors_for_tracks(
            &best_boundaries.durations,
            &expected_durations,
            &all_track_indices,
        );

        let all_refined_errors = calculate_errors_for_tracks(
            &test_boundaries.durations,
            &expected_durations,
            &all_track_indices,
        );

        // Metrics for cascade region (for logging)
        let cascade_original_within = cascade_original_errors.iter().filter(|e| e.abs() <= 10.0).count();
        let cascade_refined_within = cascade_refined_errors.iter().filter(|e| e.abs() <= 10.0).count();
        let cascade_mean_original = cascade_original_errors.iter().map(|e| e.abs()).sum::<f64>() / cascade_original_errors.len() as f64;
        let cascade_mean_refined = cascade_refined_errors.iter().map(|e| e.abs()).sum::<f64>() / cascade_refined_errors.len() as f64;

        // Metrics for FULL ALBUM (for accept/reject decision)
        let all_original_within = all_original_errors.iter().filter(|e| e.abs() <= 10.0).count();
        let all_refined_within = all_refined_errors.iter().filter(|e| e.abs() <= 10.0).count();
        let all_mean_original = all_original_errors.iter().map(|e| e.abs()).sum::<f64>() / all_original_errors.len() as f64;
        let all_mean_refined = all_refined_errors.iter().map(|e| e.abs()).sum::<f64>() / all_refined_errors.len() as f64;

        // Check if we made ANY track significantly worse (>5s increase in error)
        let made_worse = all_refined_errors
            .iter()
            .zip(&all_original_errors)
            .any(|(new, old)| new.abs() > old.abs() + 5.0);

        // DECISION CRITERIA (based on FULL ALBUM):
        // Accept refinement ONLY if:
        // 1. More tracks (across entire album) are now within tolerance, OR
        // 2. Same number within tolerance BUT mean error improved by at least 2s, AND
        // 3. We didn't make any track significantly worse (>5s)
        let improved = all_refined_within > all_original_within
            || (all_refined_within == all_original_within && all_mean_refined < all_mean_original - 2.0);

        let accept = improved && !made_worse;

        if accept {
            tracing::info!("✓ ACCEPTING refinement for cascade tracks {}-{}",
                cascade.start_track + 1, cascade.end_track + 1);
            tracing::info!(
                "  Cascade region: {}/{} → {}/{} within tolerance, mean error {:.1}s → {:.1}s",
                cascade_original_within,
                cascade_track_indices.len(),
                cascade_refined_within,
                cascade_track_indices.len(),
                cascade_mean_original,
                cascade_mean_refined
            );
            tracing::info!(
                "  Full album:     {}/{} → {}/{} within tolerance, mean error {:.1}s → {:.1}s",
                all_original_within,
                expected_durations.len(),
                all_refined_within,
                expected_durations.len(),
                all_mean_original,
                all_mean_refined
            );

            best_boundaries = test_boundaries;
            accepted_count += 1;
        } else {
            let reason = if made_worse {
                "made some tracks significantly worse"
            } else if all_refined_within < all_original_within {
                "fewer tracks within tolerance (full album)"
            } else {
                "insufficient improvement in mean error (full album)"
            };

            tracing::warn!(
                "✗ REJECTING refinement for cascade tracks {}-{} ({})",
                cascade.start_track + 1,
                cascade.end_track + 1,
                reason
            );
            tracing::warn!(
                "  Cascade region: {}/{} → {}/{} within tolerance, mean error {:.1}s → {:.1}s",
                cascade_original_within,
                cascade_track_indices.len(),
                cascade_refined_within,
                cascade_track_indices.len(),
                cascade_mean_original,
                cascade_mean_refined
            );
            tracing::warn!(
                "  Full album:     {}/{} → {}/{} within tolerance, mean error {:.1}s → {:.1}s (REJECTED)",
                all_original_within,
                expected_durations.len(),
                all_refined_within,
                expected_durations.len(),
                all_mean_original,
                all_mean_refined
            );

            rejected_count += 1;
        }
    }

    tracing::info!("");
    tracing::info!(
        "=== Cascade Refinement Summary: {} accepted, {} rejected ===",
        accepted_count,
        rejected_count
    );
    tracing::info!("");

    // PHASE 2: Complementary Error Correction
    // Detect and fix single misplaced boundaries between complementary error pairs
    let complementary_pairs = detect_complementary_pairs(&best_boundaries.durations, &expected_durations);

    if !complementary_pairs.is_empty() {
        tracing::info!(
            "Detected {} complementary error pair(s) with misplaced boundaries",
            complementary_pairs.len()
        );
        for pair in &complementary_pairs {
            tracing::info!(
                "  Pair at tracks {}-{}: {:.1}s / {:.1}s errors",
                pair.track_index + 1,
                pair.track_index + 2,
                pair.error1,
                pair.error2
            );
        }

        let mut complementary_accepted = 0;
        let mut complementary_rejected = 0;

        // Try refining each complementary pair independently
        for (pair_idx, pair) in complementary_pairs.iter().enumerate() {
            tracing::info!("");
            tracing::info!(
                "--- Evaluating Complementary Pair #{} (tracks {}-{}) ---",
                pair_idx + 1,
                pair.track_index + 1,
                pair.track_index + 2
            );

            // Apply refinement to this pair
            if let Some(test_boundaries) = refine_complementary_pair(
                pair,
                &best_boundaries,
                &expected_durations,
                &audio_energy,
                file_audio.sample_rate,
            ) {
                // Calculate errors for the pair tracks
                let pair_track_indices: Vec<usize> = vec![pair.track_index, pair.track_index + 1];

                let pair_original_errors = calculate_errors_for_tracks(
                    &best_boundaries.durations,
                    &expected_durations,
                    &pair_track_indices,
                );

                let pair_refined_errors = calculate_errors_for_tracks(
                    &test_boundaries.durations,
                    &expected_durations,
                    &pair_track_indices,
                );

                // Log detailed comparison for pair
                log_track_comparison(
                    &track_names,
                    &expected_durations,
                    &best_boundaries.durations,
                    &test_boundaries.durations,
                    &pair_track_indices,
                );

                // Calculate errors for ALL TRACKS (for accept/reject decision)
                let all_track_indices: Vec<usize> = (0..expected_durations.len()).collect();

                let all_original_errors = calculate_errors_for_tracks(
                    &best_boundaries.durations,
                    &expected_durations,
                    &all_track_indices,
                );

                let all_refined_errors = calculate_errors_for_tracks(
                    &test_boundaries.durations,
                    &expected_durations,
                    &all_track_indices,
                );

                // Metrics for pair (for logging)
                let pair_original_within = pair_original_errors.iter().filter(|e| e.abs() <= 10.0).count();
                let pair_refined_within = pair_refined_errors.iter().filter(|e| e.abs() <= 10.0).count();
                let pair_mean_original =
                    pair_original_errors.iter().map(|e| e.abs()).sum::<f64>() / pair_original_errors.len() as f64;
                let pair_mean_refined =
                    pair_refined_errors.iter().map(|e| e.abs()).sum::<f64>() / pair_refined_errors.len() as f64;

                // Metrics for FULL ALBUM (for accept/reject decision)
                let all_original_within = all_original_errors.iter().filter(|e| e.abs() <= 10.0).count();
                let all_refined_within = all_refined_errors.iter().filter(|e| e.abs() <= 10.0).count();
                let all_mean_original =
                    all_original_errors.iter().map(|e| e.abs()).sum::<f64>() / all_original_errors.len() as f64;
                let all_mean_refined =
                    all_refined_errors.iter().map(|e| e.abs()).sum::<f64>() / all_refined_errors.len() as f64;

                // Check if we made ANY track significantly worse (>5s increase in error)
                let made_worse = all_refined_errors
                    .iter()
                    .zip(&all_original_errors)
                    .any(|(new, old)| new.abs() > old.abs() + 5.0);

                // DECISION CRITERIA (based on FULL ALBUM):
                // Accept ONLY if full album improves or stays same
                let improved = all_refined_within > all_original_within
                    || (all_refined_within == all_original_within && all_mean_refined < all_mean_original - 2.0);

                let accept = improved && !made_worse;

                if accept {
                    tracing::info!(
                        "✓ ACCEPTING complementary pair refinement for tracks {}-{}",
                        pair.track_index + 1,
                        pair.track_index + 2
                    );
                    tracing::info!(
                        "  Pair tracks:    {}/{} → {}/{} within tolerance, mean error {:.1}s → {:.1}s",
                        pair_original_within,
                        pair_track_indices.len(),
                        pair_refined_within,
                        pair_track_indices.len(),
                        pair_mean_original,
                        pair_mean_refined
                    );
                    tracing::info!(
                        "  Full album:     {}/{} → {}/{} within tolerance, mean error {:.1}s → {:.1}s",
                        all_original_within,
                        expected_durations.len(),
                        all_refined_within,
                        expected_durations.len(),
                        all_mean_original,
                        all_mean_refined
                    );

                    best_boundaries = test_boundaries;
                    complementary_accepted += 1;
                } else {
                    let reason = if made_worse {
                        "made some tracks significantly worse"
                    } else if all_refined_within < all_original_within {
                        "fewer tracks within tolerance (full album)"
                    } else {
                        "insufficient improvement in mean error (full album)"
                    };

                    tracing::warn!(
                        "✗ REJECTING complementary pair refinement for tracks {}-{} ({})",
                        pair.track_index + 1,
                        pair.track_index + 2,
                        reason
                    );
                    tracing::warn!(
                        "  Pair tracks:    {}/{} → {}/{} within tolerance, mean error {:.1}s → {:.1}s",
                        pair_original_within,
                        pair_track_indices.len(),
                        pair_refined_within,
                        pair_track_indices.len(),
                        pair_mean_original,
                        pair_mean_refined
                    );
                    tracing::warn!(
                        "  Full album:     {}/{} → {}/{} within tolerance, mean error {:.1}s → {:.1}s (REJECTED)",
                        all_original_within,
                        expected_durations.len(),
                        all_refined_within,
                        expected_durations.len(),
                        all_mean_original,
                        all_mean_refined
                    );

                    complementary_rejected += 1;
                }
            } else {
                tracing::warn!(
                    "✗ REJECTING complementary pair refinement for tracks {}-{} (no confident boundary found)",
                    pair.track_index + 1,
                    pair.track_index + 2
                );
                complementary_rejected += 1;
            }
        }

        tracing::info!("");
        tracing::info!(
            "=== Complementary Pair Summary: {} accepted, {} rejected ===",
            complementary_accepted,
            complementary_rejected
        );
        tracing::info!("");
    } else {
        tracing::info!("No complementary error pairs detected. Skipping complementary correction.");
        tracing::info!("");
    }

    // Return the best boundaries (with only accepted refinements applied)
    best_boundaries.to_passage_boundaries(file_audio.sample_rate)
}

#[derive(Debug, Serialize, Deserialize)]
struct RefinementComparison {
    artist: String,
    album: String,
    file_path: String,

    original_match_percentage: f64,
    refined_match_percentage: f64,

    original_tracks_within_tolerance: usize,
    refined_tracks_within_tolerance: usize,

    total_tracks: usize,

    improvement: f64, // Refined - Original percentage
}

/// Test cascade refinement on the 6 regression albums
#[tokio::test]
#[ignore] // Run explicitly: cargo test cascade_refinement --ignored
async fn test_cascade_refinement_on_regressions() -> Result<()> {
    // Initialize tracing for logging
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_test_writer()
        .try_init();

    tracing::info!("Starting cascade refinement test on 6 regression albums");

    // The 6 regression albums identified by analyze_cascade_patterns.ps1
    let regression_files = vec![
        ("Eagles", "TheLongRun.mp3"),
        ("Imagine Dragons", "NightVisions.mp3"),
        ("Kraftwerk", "TransEuropeExpress.mp3"),
        ("Nova, Heather", "Pearl.mp3"),
        ("Police", "ZenyattaMondatta.mp3"),
        ("Rolling Stones", "LetItBleed.mp3"),
    ];

    let music_folder = PathBuf::from(std::env::var("WKMP_MUSIC_FOLDER").unwrap_or_else(|_| {
        "C:/Users/Mango Cat/Music".to_string()
    }));

    // Initialize database for album matching (MusicBrainz caching)
    let cache_dir = PathBuf::from(".cache");
    if !cache_dir.exists() {
        std::fs::create_dir_all(&cache_dir)?;
    }

    let db_path = cache_dir.join("cascade_refinement_test.db");
    let db_url = format!("sqlite:{}?mode=rwc", db_path.display());

    let db_pool = sqlx::SqlitePool::connect(&db_url).await?;
    wkmp_ai::db::release_cache::ensure_tables(&db_pool).await?;

    // Create MusicBrainz client and album matcher
    let mb_client = MusicBrainzClient::new()?;
    let config = wkmp_ai::matching::album_matcher::AlbumMatcherConfig::default();
    let matcher = wkmp_ai::matching::album_matcher::AlbumMatcher::with_pool(
        config,
        mb_client,
        db_pool,
    );

    let mut comparisons = Vec::new();

    for (artist_folder, filename) in regression_files {
        let file_path = music_folder.join(artist_folder).join(filename);

        if !file_path.exists() {
            tracing::warn!("File not found: {:?}, skipping", file_path);
            continue;
        }

        tracing::info!("==========================================");
        tracing::info!("Testing refinement on: {:?}", file_path);

        // Step 1: Get original match result
        let artist = artist_folder.replace(",", ""); // "Nova, Heather" -> "Nova Heather"
        let album_name = filename.replace(".mp3", "");

        let original_result = matcher.match_album(&file_path, Some(&artist), Some(&album_name)).await?;

        if !original_result.matched {
            tracing::warn!("Album failed to match originally, skipping refinement test");
            continue;
        }

        let original_tracks_within_tolerance = original_result
            .tracks
            .iter()
            .filter(|t| t.within_tolerance)
            .count();

        let original_match_pct = (original_tracks_within_tolerance as f64 / original_result.tracks.len() as f64) * 100.0;

        tracing::info!("Original match: {:.1}% ({}/{} tracks)",
            original_match_pct,
            original_tracks_within_tolerance,
            original_result.tracks.len());

        // Step 2: Detect boundaries with audio for refinement
        let file_audio = wkmp_ai::workflow::boundary_detector::detect_boundaries_with_audio(&file_path).await?;

        tracing::info!("Detected {} boundaries", file_audio.boundaries.len());

        // Step 3: Extract MusicBrainz track durations and names
        let mb_durations: Vec<f64> = original_result
            .tracks
            .iter()
            .map(|t| t.expected_duration) // MusicBrainz duration in seconds
            .collect();

        let track_names: Vec<String> = original_result
            .tracks
            .iter()
            .map(|t| t.title.clone())
            .collect();

        tracing::info!("MusicBrainz track count: {}", mb_durations.len());

        // Step 4: Apply cascade refinement with validation
        let refined_boundaries = refine_boundaries_with_mb_hints(&file_audio, mb_durations.clone(), track_names);

        tracing::info!("Refined to {} boundaries", refined_boundaries.len());

        // Step 5: Re-validate against MusicBrainz durations
        let mut refined_tracks_within_tolerance = 0;
        let tolerance_ms = 10000; // 10 seconds

        for (i, &mb_duration) in mb_durations.iter().enumerate() {
            if i >= refined_boundaries.len() {
                break;
            }

            let boundary = &refined_boundaries[i];
            let tick_rate: f64 = 28_224_000.0;
            let our_duration = (boundary.end_time - boundary.start_time) as f64 / tick_rate;

            let error_ms = ((our_duration - mb_duration) * 1000.0).abs();

            if error_ms <= tolerance_ms as f64 {
                refined_tracks_within_tolerance += 1;
            }

            tracing::debug!(
                "Track {}: MB={:.1}s Our={:.1}s Error={:.1}ms Within={}",
                i + 1,
                mb_duration,
                our_duration,
                error_ms,
                error_ms <= tolerance_ms as f64
            );
        }

        let refined_match_pct = (refined_tracks_within_tolerance as f64 / mb_durations.len() as f64) * 100.0;

        tracing::info!("Refined match: {:.1}% ({}/{} tracks)",
            refined_match_pct,
            refined_tracks_within_tolerance,
            mb_durations.len());

        let improvement = refined_match_pct - original_match_pct;

        tracing::info!("Improvement: {:+.1}%", improvement);

        comparisons.push(RefinementComparison {
            artist: artist.clone(),
            album: album_name.clone(),
            file_path: file_path.display().to_string(),
            original_match_percentage: original_match_pct,
            refined_match_percentage: refined_match_pct,
            original_tracks_within_tolerance,
            refined_tracks_within_tolerance,
            total_tracks: mb_durations.len(),
            improvement,
        });
    }

    // Output results
    let json = serde_json::to_string_pretty(&comparisons)?;
    std::fs::write("wkmp-ai/cascade_refinement_comparison.json", json)?;

    println!("\n=== Cascade Refinement Test Results ===\n");
    for comp in &comparisons {
        println!("{} - {}", comp.artist, comp.album);
        println!(
            "  Original: {:.1}% ({}/{} tracks)",
            comp.original_match_percentage, comp.original_tracks_within_tolerance, comp.total_tracks
        );
        println!(
            "  Refined:  {:.1}% ({}/{} tracks)",
            comp.refined_match_percentage, comp.refined_tracks_within_tolerance, comp.total_tracks
        );
        println!("  Improvement: {:+.1}%", comp.improvement);
        println!();
    }

    Ok(())
}
