//! Silence Detection Module
//!
//! **[PLAN030]** Provides silence detection functionality using WindowDbProfile for
//! efficient 180-parameter sweep in Stage 2.
//!
//! ## Key Features
//! - **WindowDbProfile**: Pre-computed dB profile for fast parameter testing
//! - **Adaptive RMS window sizing**: Automatically adjusts based on min_duration parameter
//! - **Single-pass analysis**: Computes dB profile once, tests 180 parameter combinations efficiently
//!
//! ## Performance Optimization
//! Stage 2 uses `compute_window_db_profile()` to scan audio once, then `find_silence_regions_from_profile()`
//! to test all 180 parameter combinations against the cached profile. This replaces 180 separate
//! scans with 1 scan + 180 fast filters.

use rayon::prelude::*;

use super::constants::{
    RMS_WINDOW_OVERLAP, RMS_WINDOW_SHORT_SECS, RMS_WINDOW_MEDIUM_SECS, RMS_WINDOW_STANDARD_SECS,
    RMS_WINDOW_THRESHOLD_SHORT, RMS_WINDOW_THRESHOLD_MEDIUM, SILENCE_DB_FLOOR, SILENCE_RMS_EPSILON,
};
use super::types::SilenceCache;

// =============================================================================
// WindowDbProfile (Stage 2 Performance Optimization)
// =============================================================================

/// Pre-computed dB profile for efficient silence detection parameter sweeps
///
/// Stores dB level for each time window in the audio. This allows testing
/// multiple (threshold, min_duration) parameter combinations without re-scanning
/// the audio samples.
///
/// Used by Stage 2 to test 180 parameter combinations efficiently:
/// - Compute profile once: O(n)
/// - Test each parameter: O(w) where w = number of windows
/// - Total: O(n + 180w) instead of O(180n)
pub struct WindowDbProfile {
    /// dB level for each window
    pub db_values: Vec<f64>,
    /// Samples per window step (for sample position calculation)
    pub window_step: usize,
    /// Total samples in the original audio
    pub total_samples: usize,
}

/// Single-pass dB profile computation
///
/// Scans the audio ONCE and stores dB for each window.
/// This replaces 180 separate scans with 1 scan + 180 cheap filters.
///
/// Uses finest window (25ms) for best temporal resolution.
///
/// # Arguments
/// * `samples` - Audio samples (mono or interleaved stereo)
/// * `sample_rate` - Sample rate in Hz (e.g., 44100)
///
/// # Returns
/// WindowDbProfile containing dB values for all time windows
pub fn compute_window_db_profile(samples: &[f32], sample_rate: u32) -> WindowDbProfile {
    // Use finest window (25ms) for best temporal resolution
    let rms_window_secs = RMS_WINDOW_SHORT_SECS; // 0.025s
    let window_size = (sample_rate as f64 * rms_window_secs) as usize;
    let window_step = (sample_rate as f64 * rms_window_secs * RMS_WINDOW_OVERLAP) as usize;

    // Scan all windows and record their dB levels
    let mut db_values: Vec<f64> = Vec::new();
    for window_start in (0..samples.len()).step_by(window_step.max(1)) {
        let window_end = (window_start + window_size).min(samples.len());
        let db = calculate_db(&samples[window_start..window_end]) as f64;
        db_values.push(db);
    }

    WindowDbProfile {
        db_values,
        window_step: window_step.max(1),
        total_samples: samples.len(),
    }
}

/// Find silence regions from pre-computed window dB values
///
/// This is the same algorithm as `detect_silence()`, but operates on
/// pre-computed dB values instead of re-scanning samples.
///
/// # Arguments
/// * `profile` - Pre-computed WindowDbProfile
/// * `threshold_db` - Silence threshold in dB (e.g., -50.0)
/// * `min_duration_samples` - Minimum silence duration in samples
///
/// # Returns
/// List of (start_sample, end_sample) tuples marking silence regions
pub fn find_silence_regions_from_profile(
    profile: &WindowDbProfile,
    threshold_db: f64,
    min_duration_samples: usize,
) -> Vec<(usize, usize)> {
    let mut silence_regions = Vec::new();
    let mut in_silence = false;
    let mut silence_start = 0;

    for (i, &db) in profile.db_values.iter().enumerate() {
        let is_silent = db < threshold_db;

        if is_silent && !in_silence {
            silence_start = i * profile.window_step;
            in_silence = true;
        } else if !is_silent && in_silence {
            let silence_end = i * profile.window_step;
            let duration = silence_end - silence_start;

            if duration >= min_duration_samples {
                silence_regions.push((silence_start, silence_end));
            }
            in_silence = false;
        }
    }

    // Handle silence at end of file
    if in_silence {
        let silence_end = profile.total_samples;
        let duration = silence_end - silence_start;
        if duration >= min_duration_samples {
            silence_regions.push((silence_start, silence_end));
        }
    }

    silence_regions
}

// =============================================================================
// Original Silence Detection (Stages 3-5)
// =============================================================================

/// Detect silence regions with given parameters
///
/// Original implementation that scans samples directly (not using WindowDbProfile).
/// Used by Stages 3-5 which don't need parameter sweeps.
///
/// Adaptive RMS window sizing:
/// - min_duration ≤ 0.3s: 25ms window (fine-grained detection)
/// - min_duration ≤ 0.6s: 50ms window
/// - min_duration > 0.6s: 100ms window (standard)
///
/// # Arguments
/// * `samples` - Audio samples
/// * `sample_rate` - Sample rate in Hz
/// * `threshold_db` - Silence threshold in dB
/// * `min_duration_secs` - Minimum silence duration in seconds
///
/// # Returns
/// List of (start_sample, end_sample) tuples marking silence regions
pub fn detect_silence(
    samples: &[f32],
    sample_rate: u32,
    threshold_db: f64,
    min_duration_secs: f64,
) -> Vec<(usize, usize)> {
    // Adaptive RMS window sizing based on min_duration
    let rms_window_secs = if min_duration_secs <= RMS_WINDOW_THRESHOLD_SHORT {
        RMS_WINDOW_SHORT_SECS // 25ms for short durations
    } else if min_duration_secs <= RMS_WINDOW_THRESHOLD_MEDIUM {
        RMS_WINDOW_MEDIUM_SECS // 50ms for medium durations
    } else {
        RMS_WINDOW_STANDARD_SECS // 100ms for long durations
    };

    let window_size = (sample_rate as f64 * rms_window_secs) as usize;
    let window_step = (sample_rate as f64 * rms_window_secs * RMS_WINDOW_OVERLAP) as usize;
    let min_silence_samples = (sample_rate as f64 * min_duration_secs) as usize;

    let mut is_silent = Vec::new();

    for window_start in (0..samples.len()).step_by(window_step.max(1)) {
        let window_end = (window_start + window_size).min(samples.len());
        let db = calculate_db(&samples[window_start..window_end]);
        is_silent.push((db as f64) < threshold_db);
    }

    // Find continuous silence regions
    let mut silence_regions = Vec::new();
    let mut in_silence = false;
    let mut silence_start = 0;

    for (i, &silent) in is_silent.iter().enumerate() {
        if silent && !in_silence {
            silence_start = i * window_step.max(1);
            in_silence = true;
        } else if !silent && in_silence {
            let silence_end = i * window_step.max(1);
            let duration = silence_end - silence_start;

            if duration >= min_silence_samples {
                silence_regions.push((silence_start, silence_end));
            }
            in_silence = false;
        }
    }

    silence_regions
}

/// Get track durations for given parameters
///
/// Convenience function that combines silence detection and gap-to-track conversion.
///
/// # Arguments
/// * `samples` - Audio samples
/// * `sample_rate` - Sample rate in Hz
/// * `threshold_db` - Silence threshold in dB
/// * `min_duration_secs` - Minimum silence duration in seconds
///
/// # Returns
/// Vector of track durations in seconds
pub fn get_track_durations(
    samples: &[f32],
    sample_rate: u32,
    threshold_db: f64,
    min_duration_secs: f64,
) -> Vec<f64> {
    let silence_regions = detect_silence(samples, sample_rate, threshold_db, min_duration_secs);
    gaps_to_track_durations(&silence_regions, samples.len(), sample_rate)
}

/// Convert silence regions to track durations
///
/// Given a list of silence regions (sample ranges) within an audio file,
/// calculates the duration of each non-silent segment (i.e., each track).
///
/// # Arguments
/// * `silence_regions` - List of (start_sample, end_sample) tuples marking silent gaps
/// * `total_samples` - Total number of samples in the audio file
/// * `sample_rate` - Audio sample rate in Hz (e.g., 44100)
///
/// # Returns
/// Vector of track durations in seconds, one per detected track
pub fn gaps_to_track_durations(
    silence_regions: &[(usize, usize)],
    total_samples: usize,
    sample_rate: u32,
) -> Vec<f64> {
    let mut tracks = Vec::new();
    let mut current_start = 0;

    for &(silence_start, silence_end) in silence_regions {
        if silence_start > current_start {
            let duration_secs = (silence_start - current_start) as f64 / sample_rate as f64;
            tracks.push(duration_secs);
        }
        current_start = silence_end;
    }

    // Add final segment
    if current_start < total_samples {
        let duration_secs = (total_samples - current_start) as f64 / sample_rate as f64;
        tracks.push(duration_secs);
    }

    tracks
}

// =============================================================================
// Audio Analysis Helpers
// =============================================================================

/// Calculate Root Mean Square (RMS) amplitude
///
/// # Arguments
/// * `samples` - Audio samples
///
/// # Returns
/// RMS amplitude (0.0-1.0 typically)
pub fn calculate_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    (samples.iter().map(|&s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
}

/// Calculate RMS amplitude in decibels
///
/// Converts RMS amplitude to dB scale using standard formula: dB = 20 * log10(RMS)
///
/// # Arguments
/// * `samples` - Audio samples
///
/// # Returns
/// dB level (typically -100 to 0 dB)
pub fn calculate_db(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return SILENCE_DB_FLOOR;
    }

    let rms = calculate_rms(samples);

    if rms < SILENCE_RMS_EPSILON {
        SILENCE_DB_FLOOR
    } else {
        20.0 * rms.log10()
    }
}

// =============================================================================
// Silence Detection Cache (Stage 2 Optimization)
// =============================================================================

/// Pre-compute track durations for all parameter combinations
///
/// Optimizes Stage 2 by:
/// 1. Computing dB profile once (single scan of audio)
/// 2. Testing all 180 parameter combinations in parallel using that profile
///
/// # Arguments
/// * `samples` - Decoded audio samples
/// * `sample_rate` - Sample rate in Hz
/// * `threshold_values` - Array of silence thresholds to test (dB)
/// * `min_duration_values` - Array of minimum silence durations to test (seconds)
///
/// # Returns
/// SilenceCache where cache[idx] contains track durations for:
/// - `idx = threshold_idx * num_min_durations + min_duration_idx`
///
/// # Performance
/// - Single-pass dB profiling: O(N) where N = number of samples
/// - Parallel region finding: O(180) combinations tested concurrently
/// - Typical speedup: 180x vs sequential scanning
pub fn precompute_silence_cache(
    samples: &[f32],
    sample_rate: u32,
    threshold_values: &[f64],
    min_duration_values: &[f64],
) -> SilenceCache {
    let num_min_durations = min_duration_values.len();
    let total_samples = samples.len();

    // SINGLE PASS: Compute dB profile for all windows once
    let profile = compute_window_db_profile(samples, sample_rate);

    // Build list of all (index, threshold, min_duration_samples) tuples
    let params: Vec<(usize, f64, usize)> = threshold_values
        .iter()
        .enumerate()
        .flat_map(|(thresh_idx, &thresh)| {
            min_duration_values
                .iter()
                .enumerate()
                .map(move |(dur_idx, &min_dur)| {
                    let idx = thresh_idx * num_min_durations + dur_idx;
                    let min_samples = (sample_rate as f64 * min_dur) as usize;
                    (idx, thresh, min_samples)
                })
        })
        .collect();

    // Find silence regions for each parameter combination IN PARALLEL
    // This is now cheap because we're filtering pre-computed dB values, not re-scanning audio
    let mut results: Vec<(usize, Vec<f64>)> = params
        .par_iter()
        .map(|&(idx, thresh, min_samples)| {
            let silence_regions = find_silence_regions_from_profile(&profile, thresh, min_samples);
            let durations = gaps_to_track_durations(&silence_regions, total_samples, sample_rate);
            (idx, durations)
        })
        .collect();

    // Sort by index to restore correct order
    results.sort_by_key(|(idx, _)| *idx);

    // Extract just the durations in order
    results.into_iter().map(|(_, durations)| durations).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_rms_empty() {
        assert_eq!(calculate_rms(&[]), 0.0);
    }

    #[test]
    fn test_calculate_rms_known_values() {
        // RMS of [1.0, -1.0, 1.0, -1.0] = sqrt((1+1+1+1)/4) = 1.0
        let samples = [1.0f32, -1.0, 1.0, -1.0];
        let rms = calculate_rms(&samples);
        assert!((rms - 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_calculate_rms_sine() {
        // RMS of full-amplitude sine wave ≈ 0.707
        let samples: Vec<f32> = (0..1000)
            .map(|i| (i as f32 * std::f32::consts::PI * 2.0 / 100.0).sin())
            .collect();
        let rms = calculate_rms(&samples);
        assert!((rms - 0.707).abs() < 0.01);
    }

    #[test]
    fn test_calculate_db_silence() {
        let samples = [0.0f32; 100];
        let db = calculate_db(&samples);
        assert_eq!(db, SILENCE_DB_FLOOR);
    }

    #[test]
    fn test_calculate_db_full_scale() {
        // Full scale sine wave should be near 0 dB (RMS ≈ 0.707 → -3 dB)
        let samples: Vec<f32> = (0..1000)
            .map(|i| (i as f32 * std::f32::consts::PI * 2.0 / 100.0).sin())
            .collect();
        let db = calculate_db(&samples);
        assert!(db > -5.0 && db < 0.0);
    }

    #[test]
    fn test_gaps_to_track_durations_no_gaps() {
        let durations = gaps_to_track_durations(&[], 44100, 44100);
        assert_eq!(durations.len(), 1);
        assert!((durations[0] - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_gaps_to_track_durations_one_gap() {
        // Audio: 1s track, 0.5s silence, 1.5s track
        // Silence at samples 44100-66150 (22050 samples = 0.5s)
        let silence_regions = vec![(44100, 66150)];
        let total = 44100 + 22050 + 66150; // 3s total at 44100 Hz
        let durations = gaps_to_track_durations(&silence_regions, total, 44100);

        assert_eq!(durations.len(), 2);
        assert!((durations[0] - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_detect_silence_with_synthetic_audio() {
        // Create audio with clear silence: loud-quiet-loud pattern
        let sample_rate = 44100u32;
        let mut samples = Vec::new();

        // 1 second of "loud" audio (sine wave)
        for i in 0..44100 {
            samples.push((i as f32 * 0.01).sin() * 0.5);
        }

        // 1 second of silence
        for _ in 0..44100 {
            samples.push(0.0);
        }

        // 1 second of "loud" audio
        for i in 0..44100 {
            samples.push((i as f32 * 0.01).sin() * 0.5);
        }

        let silence_regions = detect_silence(&samples, sample_rate, -40.0, 0.5);

        // Should detect at least one silence region
        assert!(!silence_regions.is_empty());

        // The silence region should be roughly in the middle
        let (start, end) = silence_regions[0];
        let start_sec = start as f64 / sample_rate as f64;
        let end_sec = end as f64 / sample_rate as f64;
        assert!(start_sec >= 0.8 && start_sec <= 1.5);
        assert!(end_sec >= 1.5 && end_sec <= 2.5);
    }

    #[test]
    fn test_precompute_silence_cache_dimensions() {
        let sample_rate = 44100u32;
        let samples = vec![0.0f32; sample_rate as usize * 3]; // 3 seconds of silence

        let thresholds = [-50.0, -55.0, -60.0];
        let min_durations = [0.5, 1.0, 1.5, 2.0];

        let cache = precompute_silence_cache(&samples, sample_rate, &thresholds, &min_durations);

        // Should have 3 * 4 = 12 entries
        assert_eq!(cache.len(), 12);
    }

    #[test]
    fn test_window_db_profile() {
        let sample_rate = 44100u32;
        // Create 1 second of silence
        let samples = vec![0.0f32; sample_rate as usize];

        let profile = compute_window_db_profile(&samples, sample_rate);

        // Should have computed some windows
        assert!(!profile.db_values.is_empty());
        assert_eq!(profile.total_samples, samples.len());

        // All windows should be silent (very low dB)
        for db in &profile.db_values {
            assert!(*db < -90.0);
        }
    }

    #[test]
    fn test_get_track_durations() {
        // Create audio: 1s loud, 1s silence, 1s loud
        let sample_rate = 44100u32;
        let mut samples = Vec::new();

        // Loud section
        for i in 0..44100 {
            samples.push((i as f32 * 0.02).sin() * 0.7);
        }
        // Silent section
        for _ in 0..44100 {
            samples.push(0.0);
        }
        // Loud section
        for i in 0..44100 {
            samples.push((i as f32 * 0.02).sin() * 0.7);
        }

        let durations = get_track_durations(&samples, sample_rate, -50.0, 0.3);

        // Should detect 2 tracks (before and after silence)
        assert_eq!(durations.len(), 2);

        // Each track should be roughly 1 second
        assert!(durations[0] > 0.5 && durations[0] < 1.5);
        assert!(durations[1] > 0.5 && durations[1] < 1.5);
    }
}
