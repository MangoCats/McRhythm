//! Stage 4: Quiet Spot Detection
//!
//! **[PLAN030]** Uses RMS profiling to find quiet spots near expected track
//! boundaries. Applied when silence-based detection fails.
//!
//! # Algorithm Overview
//!
//! 1. Create RMS profile from audio samples (windowed RMS values)
//! 2. For each expected track boundary from edition:
//!    - Calculate expected position based on cumulative duration
//!    - Search for local RMS minimum within search window
//!    - Use quiet spot as detected boundary
//! 3. Convert boundaries to track durations
//! 4. Apply penalty factor (lower confidence than silence detection)

use crate::matching::constants::STAGE4_PENALTY_PERCENT;
use crate::matching::types::Edition;

/// Stage 4 result for a single edition
#[derive(Debug, Clone)]
pub struct Stage4Result {
    /// Edition that was tested
    pub edition: Edition,
    /// Match percentage with penalty applied (0-100)
    pub penalized_percentage: f64,
    /// Raw match percentage before penalty (0-100)
    pub raw_percentage: f64,
    /// Detected boundary positions (sample indices)
    pub boundary_positions: Vec<usize>,
    /// Detected track durations (seconds)
    pub detected_durations: Vec<f64>,
    /// Per-track errors (seconds)
    pub track_errors: Vec<f64>,
    /// Number of tracks matched within tolerance
    pub matched_count: usize,
}

/// RMS (Root Mean Square) profile for audio analysis
///
/// Pre-computed RMS values for windowed audio segments,
/// used to find quiet spots near expected track boundaries.
#[derive(Debug, Clone)]
pub struct RmsProfile {
    /// RMS values per window (linear scale)
    pub values: Vec<f32>,
    /// Window size in samples
    pub window_size: usize,
    /// Sample rate (Hz)
    pub sample_rate: u32,
}

impl RmsProfile {
    /// Create RMS profile from audio samples
    ///
    /// # Arguments
    /// * `samples` - Audio samples (mono, normalized to -1.0..1.0)
    /// * `sample_rate` - Sample rate in Hz
    /// * `window_ms` - Window size in milliseconds
    pub fn from_samples(samples: &[f32], sample_rate: u32, window_ms: f64) -> Self {
        let window_size = ((sample_rate as f64 * window_ms / 1000.0) as usize).max(1);

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
    ///
    /// # Arguments
    /// * `target_sample` - Target sample position to search around
    /// * `search_window_samples` - Number of samples on each side to search
    ///
    /// # Returns
    /// Sample position of quietest spot, or None if search fails
    pub fn find_quiet_spot(
        &self,
        target_sample: usize,
        search_window_samples: usize,
    ) -> Option<usize> {
        let target_idx = target_sample / self.window_size;
        let search_windows = search_window_samples / self.window_size;

        let start_idx = target_idx.saturating_sub(search_windows);
        let end_idx = (target_idx + search_windows).min(self.values.len());

        if start_idx >= end_idx || self.values.is_empty() {
            return None;
        }

        // Find minimum RMS in search window
        let (min_idx, _) = self.values[start_idx..end_idx]
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))?;

        Some((start_idx + min_idx) * self.window_size)
    }

    /// Get total duration in seconds
    pub fn total_duration_secs(&self) -> f64 {
        (self.values.len() * self.window_size) as f64 / self.sample_rate as f64
    }
}

/// Run Stage 4 quiet spot detection
///
/// Detects track boundaries by finding quiet spots (RMS minima)
/// near expected boundary positions from edition durations.
///
/// # Arguments
/// * `rms_profile` - Pre-computed RMS profile
/// * `total_samples` - Total audio sample count
/// * `editions` - Candidate editions to test
/// * `tolerance_secs` - Track match tolerance (seconds)
/// * `search_window_secs` - Window to search for quiet spots (seconds)
///
/// # Returns
/// Vector of results for each edition, sorted by penalized_percentage descending
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
        b.penalized_percentage
            .partial_cmp(&a.penalized_percentage)
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
    let expected_secs: Vec<f64> = edition
        .durations
        .iter()
        .map(|ms| *ms as f64 / 1000.0)
        .collect();

    // **[BUG FIX]** Validate edition total duration against actual file duration
    let expected_total_secs: f64 = expected_secs.iter().sum();
    let actual_total_secs = total_samples as f64 / sample_rate as f64;

    const MAX_OVERAGE_RATIO: f64 = 1.10; // 10% allowance for rounding
    if expected_total_secs > actual_total_secs * MAX_OVERAGE_RATIO {
        // Edition total exceeds file duration - impossible match
        return Stage4Result {
            edition: edition.clone(),
            penalized_percentage: 0.0,
            raw_percentage: 0.0,
            boundary_positions: Vec::new(),
            detected_durations: Vec::new(),
            track_errors: Vec::new(),
            matched_count: 0,
        };
    }

    let mut expected_boundaries = Vec::new();
    let mut cumulative = 0.0;
    // Boundaries are between tracks, so N tracks have N-1 boundaries
    for dur in &expected_secs[..expected_secs.len().saturating_sub(1)] {
        cumulative += dur;
        let sample_pos = (cumulative * sample_rate as f64) as usize;

        // **[BUG FIX]** Validate boundary position before using it
        if sample_pos >= total_samples {
            // Expected boundary beyond file end - impossible match
            return Stage4Result {
                edition: edition.clone(),
                penalized_percentage: 0.0,
                raw_percentage: 0.0,
                boundary_positions: Vec::new(),
                detected_durations: Vec::new(),
                track_errors: Vec::new(),
                matched_count: 0,
            };
        }

        expected_boundaries.push(sample_pos);
    }

    // Find quiet spots near each expected boundary
    let mut detected_boundaries = Vec::new();
    for &expected_pos in &expected_boundaries {
        if let Some(quiet_pos) = rms_profile.find_quiet_spot(expected_pos, search_window_samples) {
            detected_boundaries.push(quiet_pos);
        } else {
            // Fall back to expected position if search fails
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

    // **[BUG FIX]** Final track: validate prev_pos before calculating
    if prev_pos >= total_samples {
        // Last boundary at or beyond file end - impossible match
        return Stage4Result {
            edition: edition.clone(),
            penalized_percentage: 0.0,
            raw_percentage: 0.0,
            boundary_positions: Vec::new(),
            detected_durations: Vec::new(),
            track_errors: Vec::new(),
            matched_count: 0,
        };
    }

    // Final track: from last boundary to end
    let final_duration = (total_samples - prev_pos) as f64 / sample_rate as f64;
    detected_durations.push(final_duration);

    // Calculate errors and match percentage
    let mut errors = Vec::new();
    let mut matched_count = 0;
    for (detected, expected) in detected_durations.iter().zip(expected_secs.iter()) {
        let error = (detected - expected).abs();
        errors.push(error);
        if error <= tolerance_secs {
            matched_count += 1;
        }
    }

    let raw_percentage = if expected_secs.is_empty() {
        0.0
    } else {
        (matched_count as f64 / expected_secs.len() as f64) * 100.0
    };

    // Apply penalty since this is a less reliable method
    let penalized_percentage = raw_percentage * (1.0 - STAGE4_PENALTY_PERCENT / 100.0);

    Stage4Result {
        edition: edition.clone(),
        penalized_percentage,
        raw_percentage,
        boundary_positions: detected_boundaries,
        detected_durations,
        track_errors: errors,
        matched_count,
    }
}

/// Check if Stage 4 found acceptable match
pub fn stage4_success(results: &[Stage4Result], min_percentage: f64) -> bool {
    results
        .first()
        .map(|r| r.penalized_percentage >= min_percentage)
        .unwrap_or(false)
}

/// Get the best Stage 4 result
pub fn get_best_stage4_result(results: &[Stage4Result]) -> Option<&Stage4Result> {
    results.first()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_edition(track_count: usize, durations_ms: &[u32]) -> Edition {
        Edition {
            release_mbid: "test-mbid".to_string(),
            title: "Test Album".to_string(),
            artist: "Test Artist".to_string(),
            artist_credit: None,
            country: None,
            status: None,
            track_count,
            track_durations: durations_ms.iter().map(|&d| d as f64 / 1000.0).collect(),
            recording_mbids: Vec::new(),
            name_distance_rank: None,
            name_distance_score: None,
            durations: durations_ms.to_vec(),
            track_titles: (1..=track_count).map(|i| format!("Track {}", i)).collect(),
        }
    }

    #[test]
    fn test_rms_profile_creation() {
        // Create simple synthetic audio: constant amplitude
        let sample_rate = 44100u32;
        let samples: Vec<f32> = vec![0.5; 44100]; // 1 second of constant 0.5

        let profile = RmsProfile::from_samples(&samples, sample_rate, 100.0); // 100ms windows

        // Should have ~10 windows (100ms each for 1 second)
        assert!(profile.values.len() >= 9 && profile.values.len() <= 11);

        // Each window should have RMS close to 0.5
        for &rms in &profile.values {
            assert!((rms - 0.5).abs() < 0.01);
        }
    }

    #[test]
    fn test_rms_profile_quiet_spot() {
        // Create audio with a quiet spot in the middle
        let sample_rate = 44100u32;
        let window_ms = 100.0;
        let window_samples = (sample_rate as f64 * window_ms / 1000.0) as usize;

        let mut samples: Vec<f32> = Vec::new();
        // 5 windows of loud audio
        for _ in 0..(5 * window_samples) {
            samples.push(0.8);
        }
        // 1 window of quiet audio
        for _ in 0..window_samples {
            samples.push(0.1);
        }
        // 5 more windows of loud audio
        for _ in 0..(5 * window_samples) {
            samples.push(0.8);
        }

        let profile = RmsProfile::from_samples(&samples, sample_rate, window_ms);

        // Target the middle of the audio, should find the quiet spot
        let target = samples.len() / 2;
        let search_window = 3 * window_samples;

        let quiet_pos = profile.find_quiet_spot(target, search_window);
        assert!(quiet_pos.is_some());

        // Quiet spot should be around window 5 (0-indexed)
        let pos = quiet_pos.unwrap();
        let window_idx = pos / window_samples;
        assert_eq!(window_idx, 5);
    }

    #[test]
    fn test_boundary_detection() {
        // Create audio that matches a 2-track album
        let sample_rate = 44100u32;
        let window_ms = 100.0;

        // Track 1: 180s, Track 2: 180s = 360s total
        // Create profile with quiet spot at 180s boundary
        let total_samples = 360 * sample_rate as usize;
        let boundary_sample = 180 * sample_rate as usize;

        // Create RMS profile with minimum at boundary
        let window_samples = (sample_rate as f64 * window_ms / 1000.0) as usize;
        let num_windows = total_samples / window_samples;

        let mut values: Vec<f32> = vec![0.5; num_windows];
        // Set quiet spot at boundary
        let boundary_window = boundary_sample / window_samples;
        if boundary_window < num_windows {
            values[boundary_window] = 0.1;
        }

        let profile = RmsProfile {
            values,
            window_size: window_samples,
            sample_rate,
        };

        let edition = create_test_edition(2, &[180000, 180000]);

        let results = run_stage4(&profile, total_samples, &[edition], 10.0, 5.0);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].detected_durations.len(), 2);

        // Durations should be close to 180s each
        assert!((results[0].detected_durations[0] - 180.0).abs() < 15.0);
        assert!((results[0].detected_durations[1] - 180.0).abs() < 15.0);
    }

    #[test]
    fn test_penalty_application() {
        // STAGE4_PENALTY_PERCENT is 25%, so 100% raw -> 75% penalized
        let sample_rate = 44100u32;
        let total_samples = 180 * sample_rate as usize;

        // Simple profile with one track
        let profile = RmsProfile {
            values: vec![0.5; 10],
            window_size: total_samples / 10,
            sample_rate,
        };

        let edition = create_test_edition(1, &[180000]);

        let results = run_stage4(&profile, total_samples, &[edition], 10.0, 5.0);

        // Should have perfect raw match
        assert_eq!(results[0].raw_percentage, 100.0);

        // Penalized should be 75% (25% penalty)
        assert!((results[0].penalized_percentage - 75.0).abs() < 0.01);
    }

    #[test]
    fn test_stage4_success() {
        let sample_rate = 44100u32;
        let total_samples = 180 * sample_rate as usize;

        let profile = RmsProfile {
            values: vec![0.5; 10],
            window_size: total_samples / 10,
            sample_rate,
        };

        let edition = create_test_edition(1, &[180000]);

        let results = run_stage4(&profile, total_samples, &[edition], 10.0, 5.0);

        // Penalized percentage is 75%
        assert!(stage4_success(&results, 70.0));
        assert!(!stage4_success(&results, 80.0)); // 75% < 80%
        assert!(!stage4_success(&[], 50.0));
    }

    #[test]
    fn test_multiple_editions() {
        let sample_rate = 44100u32;
        let total_samples = 180 * sample_rate as usize;

        let profile = RmsProfile {
            values: vec![0.5; 10],
            window_size: total_samples / 10,
            sample_rate,
        };

        let edition1 = create_test_edition(1, &[180000]); // Perfect match
        let edition2 = create_test_edition(2, &[90000, 90000]); // Different structure
        let edition3 = create_test_edition(1, &[200000]); // Wrong duration

        let results = run_stage4(
            &profile,
            total_samples,
            &[edition1, edition2, edition3],
            10.0,
            5.0,
        );

        // Results should be sorted by penalized percentage
        assert!(results[0].penalized_percentage >= results[1].penalized_percentage);
    }

    #[test]
    fn test_empty_edition() {
        let sample_rate = 44100u32;
        let total_samples = 180 * sample_rate as usize;

        let profile = RmsProfile {
            values: vec![0.5; 10],
            window_size: total_samples / 10,
            sample_rate,
        };

        let edition = create_test_edition(0, &[]);

        let results = run_stage4(&profile, total_samples, &[edition], 10.0, 5.0);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].raw_percentage, 0.0);
    }
}
