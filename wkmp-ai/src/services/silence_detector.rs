//! Silence detection service for passage boundary detection
//!
//! **[AIA-COMP-010]** Silence-based segmentation
//! **[IMPL005]** Audio file segmentation workflow

use thiserror::Error;

/// Silence detection errors
#[derive(Debug, Error)]
pub enum SilenceError {
    /// Invalid silence threshold value
    #[error("Invalid threshold: {0}")]
    InvalidThreshold(String),

    /// Invalid detection parameters
    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),
}

/// Silence region (start_sec, end_sec)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SilenceRegion {
    /// Start time of silence region in seconds
    pub start_seconds: f32,
    /// End time of silence region in seconds
    pub end_seconds: f32,
}

impl SilenceRegion {
    /// Create new silence region
    ///
    /// # Arguments
    /// * `start_seconds` - Start time in seconds
    /// * `end_seconds` - End time in seconds
    pub fn new(start_seconds: f32, end_seconds: f32) -> Self {
        Self {
            start_seconds,
            end_seconds,
        }
    }

    /// Calculate duration of silence region in seconds
    pub fn duration(&self) -> f32 {
        self.end_seconds - self.start_seconds
    }
}

/// Silence detector
pub struct SilenceDetector {
    /// Silence threshold in dB (default: -60dB for Vinyl preset)
    threshold_db: f32,

    /// Minimum silence duration in seconds (default: 0.5s)
    min_duration_sec: f32,

    /// RMS window size in samples (default: 4410 = 100ms at 44.1kHz)
    window_size_samples: usize,
}

impl SilenceDetector {
    /// Create new silence detector with defaults
    pub fn new() -> Self {
        Self {
            threshold_db: -60.0,
            min_duration_sec: 0.5,
            window_size_samples: 4410, // 100ms at 44.1kHz
        }
    }

    /// Set silence threshold in dB
    pub fn with_threshold_db(mut self, threshold_db: f32) -> Result<Self, SilenceError> {
        if threshold_db > 0.0 {
            return Err(SilenceError::InvalidThreshold(
                "Threshold must be negative dB".to_string(),
            ));
        }
        self.threshold_db = threshold_db;
        Ok(self)
    }

    /// Set minimum silence duration
    pub fn with_min_duration(mut self, min_duration_sec: f32) -> Result<Self, SilenceError> {
        if min_duration_sec < 0.0 {
            return Err(SilenceError::InvalidParameters(
                "Min duration must be >= 0".to_string(),
            ));
        }
        self.min_duration_sec = min_duration_sec;
        Ok(self)
    }

    /// Detect silence regions in audio
    ///
    /// **[TC-COMP-013]** Threshold-based detection test
    /// **[TC-COMP-014]** Minimum duration filtering test
    ///
    /// Returns list of silence regions (start_sec, end_sec)
    pub fn detect(
        &self,
        samples: &[f32],
        sample_rate: usize,
    ) -> Result<Vec<SilenceRegion>, SilenceError> {
        if samples.is_empty() {
            return Ok(Vec::new());
        }

        // Convert threshold from dB to linear
        let threshold_linear = Self::db_to_linear(self.threshold_db);
        // REQ-F-004: Unit clarity - samples (PCM frames, SPEC023 Callback Time)
        let min_duration_samples = (self.min_duration_sec * sample_rate as f32) as usize;

        let mut silence_regions = Vec::new();
        let mut in_silence = false;
        let mut silence_start_sample = 0;  // samples, PCM frame position

        // Process audio in windows
        for (window_idx, chunk) in samples.chunks(self.window_size_samples).enumerate() {
            let rms = Self::calculate_rms(chunk);
            // REQ-F-004: Unit clarity - samples (PCM frame position in file)
            let sample_position = window_idx * self.window_size_samples;

            if rms < threshold_linear {
                // Below threshold - silence
                if !in_silence {
                    in_silence = true;
                    silence_start_sample = sample_position;  // samples
                }
            } else {
                // Above threshold - sound
                if in_silence {
                    let silence_end_sample = sample_position;  // samples
                    // REQ-F-004: Unit clarity - samples (duration in PCM frames)
                    let duration_samples = silence_end_sample - silence_start_sample;

                    // Only include if longer than minimum duration
                    if duration_samples >= min_duration_samples {
                        let start_sec = silence_start_sample as f32 / sample_rate as f32;
                        let end_sec = silence_end_sample as f32 / sample_rate as f32;
                        silence_regions.push(SilenceRegion::new(start_sec, end_sec));
                    }

                    in_silence = false;
                }
            }
        }

        // Handle silence at end of file
        if in_silence {
            let silence_end_sample = samples.len();  // samples
            // REQ-F-004: Unit clarity - samples (duration in PCM frames)
            let duration_samples = silence_end_sample - silence_start_sample;

            if duration_samples >= min_duration_samples {
                let start_sec = silence_start_sample as f32 / sample_rate as f32;
                let end_sec = silence_end_sample as f32 / sample_rate as f32;
                silence_regions.push(SilenceRegion::new(start_sec, end_sec));
            }
        }

        Ok(silence_regions)
    }

    /// Calculate RMS (Root Mean Square) of samples
    fn calculate_rms(samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }

        let sum_squares: f32 = samples.iter().map(|&s| s * s).sum();
        (sum_squares / samples.len() as f32).sqrt()
    }

    /// Convert dB to linear amplitude
    fn db_to_linear(db: f32) -> f32 {
        10.0_f32.powf(db / 20.0)
    }

    /// Convert linear amplitude to dB
    #[allow(dead_code)]
    fn linear_to_db(linear: f32) -> f32 {
        20.0 * linear.log10()
    }
}

impl Default for SilenceDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_silence_detector_creation() {
        let detector = SilenceDetector::new();
        assert_eq!(detector.threshold_db, -60.0);
        assert_eq!(detector.min_duration_sec, 0.5);
    }

    #[test]
    fn test_with_threshold() {
        let detector = SilenceDetector::new().with_threshold_db(-50.0).unwrap();
        assert_eq!(detector.threshold_db, -50.0);
    }

    #[test]
    fn test_invalid_threshold() {
        let result = SilenceDetector::new().with_threshold_db(10.0);
        assert!(result.is_err());
    }

    #[test]
    fn test_rms_calculation() {
        // Test with known sine wave (amplitude 1.0)
        let samples: Vec<f32> = (0..100)
            .map(|i| (2.0 * std::f32::consts::PI * i as f32 / 100.0).sin())
            .collect();

        let rms = SilenceDetector::calculate_rms(&samples);

        // RMS of sine wave with amplitude 1.0 should be ~0.707 (1/sqrt(2))
        let expected = 1.0 / std::f32::consts::SQRT_2;
        assert!((rms - expected).abs() < 0.01);
    }

    #[test]
    fn test_db_conversion() {
        let db = -60.0;
        let linear = SilenceDetector::db_to_linear(db);

        // -60dB = 0.001 linear
        assert!((linear - 0.001).abs() < 0.0001);

        // Round trip
        let db_back = SilenceDetector::linear_to_db(linear);
        assert!((db_back - db).abs() < 0.01);
    }

    #[test]
    fn test_detect_silence_simple() {
        let sample_rate = 44100;
        let detector = SilenceDetector::new();

        // Create audio: 10s sound, 2s silence, 10s sound
        let mut samples = Vec::new();

        // Sound (0-10s)
        for _ in 0..(10 * sample_rate) {
            samples.push(0.5);
        }

        // Silence (10-12s)
        for _ in 0..(2 * sample_rate) {
            samples.push(0.0001);
        }

        // Sound (12-22s)
        for _ in 0..(10 * sample_rate) {
            samples.push(0.5);
        }

        let regions = detector.detect(&samples, sample_rate).unwrap();

        // Should detect one silence region around 10-12s
        assert!(!regions.is_empty(), "Should detect silence");
        let region = regions[0];
        assert!(region.start_seconds >= 9.0 && region.start_seconds <= 11.0);
        assert!(region.end_seconds >= 11.0 && region.end_seconds <= 13.0);
    }

    #[test]
    fn test_minimum_duration_filter() {
        let sample_rate = 44100;
        let detector = SilenceDetector::new()
            .with_min_duration(0.5)
            .unwrap();

        let mut samples = Vec::new();

        // Sound
        for _ in 0..(10 * sample_rate) {
            samples.push(0.5);
        }

        // Brief silence (0.2s) - should be filtered out
        for _ in 0..(sample_rate / 5) {
            samples.push(0.0001);
        }

        // Sound
        for _ in 0..(10 * sample_rate) {
            samples.push(0.5);
        }

        // Long silence (1.0s) - should be detected
        for _ in 0..sample_rate {
            samples.push(0.0001);
        }

        // Sound to close region
        for _ in 0..(5 * sample_rate) {
            samples.push(0.5);
        }

        let regions = detector.detect(&samples, sample_rate).unwrap();

        // Should only detect the 1-second silence
        assert_eq!(regions.len(), 1, "Should detect only long silence");
    }
}
