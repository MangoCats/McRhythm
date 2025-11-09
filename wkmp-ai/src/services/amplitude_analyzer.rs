//! Amplitude-based lead-in/lead-out detection
//!
//! **[AIA-COMP-010]** RMS-based amplitude analysis
//!
//! Per [IMPL009](../../docs/IMPL009-amplitude_analyzer_implementation.md)
//!
//! **Sprint 6 Implementation:** Full RMS-based amplitude analysis using AudioLoader

use std::path::Path;
use thiserror::Error;

use crate::import_v2::tier1::audio_loader::AudioLoader;
use crate::models::AmplitudeParameters;

/// Amplitude analysis errors
#[derive(Debug, Error)]
pub enum AnalysisError {
    #[error("Failed to read audio file: {0}")]
    ReadError(String),

    #[error("Unsupported audio format: {0}")]
    UnsupportedFormat(String),

    #[error("Analysis failed: {0}")]
    AnalysisFailed(String),
}

/// Result of amplitude analysis
#[derive(Debug, Clone)]
pub struct AmplitudeAnalysisResult {
    /// Peak RMS level (linear scale 0.0-1.0, not dB)
    pub peak_rms: f64,

    /// Lead-in duration (seconds)
    pub lead_in_duration: f64,

    /// Lead-out duration (seconds)
    pub lead_out_duration: f64,

    /// Quick ramp up detected
    pub quick_ramp_up: bool,

    /// Quick ramp down detected
    pub quick_ramp_down: bool,

    /// RMS envelope (for visualization/debugging)
    pub rms_profile: Option<Vec<f32>>,
}

/// Amplitude analyzer service
///
/// **Algorithm:**
/// 1. Load audio segment using AudioLoader
/// 2. Calculate RMS over sliding windows
/// 3. Detect lead-in (rise from silence to peak)
/// 4. Detect lead-out (fall from peak to silence)
/// 5. Detect quick ramps (steep slope changes)
pub struct AmplitudeAnalyzer {
    params: AmplitudeParameters,
    audio_loader: AudioLoader,
}

impl AmplitudeAnalyzer {
    pub fn new(params: AmplitudeParameters) -> Self {
        Self {
            params,
            audio_loader: AudioLoader::default(),
        }
    }

    /// Analyze audio file for lead-in/lead-out timing
    ///
    /// **[AIA-COMP-010]** Full RMS-based implementation
    ///
    /// # Algorithm
    /// 1. Load audio segment (start_time to end_time)
    /// 2. Calculate RMS envelope over windows
    /// 3. Find peak RMS value
    /// 4. Detect lead-in: time from start until RMS reaches threshold (e.g., -20 dB below peak)
    /// 5. Detect lead-out: time from peak until RMS falls below threshold
    /// 6. Detect quick ramps: slope > quick_ramp_threshold
    pub async fn analyze_file(
        &self,
        file_path: &Path,
        start_time: f64,
        end_time: f64,
    ) -> Result<AmplitudeAnalysisResult, AnalysisError> {
        tracing::debug!(
            file = %file_path.display(),
            start = start_time,
            end = end_time,
            "Amplitude analysis (full RMS implementation)"
        );

        // **Phase 1: Load audio segment**
        // Convert seconds to ticks (TICK_RATE = 28,224,000 Hz)
        const TICKS_PER_SECOND: f64 = 28_224_000.0;
        let start_ticks = (start_time * TICKS_PER_SECOND) as i64;
        let end_ticks = (end_time * TICKS_PER_SECOND) as i64;

        let audio_segment = self.audio_loader
            .load_segment(file_path, start_ticks, end_ticks)
            .map_err(|e| AnalysisError::ReadError(e.to_string()))?;

        tracing::debug!(
            "Loaded {} stereo samples at {} Hz",
            audio_segment.samples.len(),
            audio_segment.sample_rate
        );

        // **Phase 2: Calculate RMS envelope**
        let rms_profile = self.calculate_rms_envelope(&audio_segment.samples, audio_segment.sample_rate);

        // **Phase 3: Find peak RMS**
        let peak_rms = rms_profile.iter().cloned().fold(0.0f32, f32::max) as f64;

        tracing::debug!("Peak RMS: {:.4}", peak_rms);

        // **Phase 4: Detect lead-in/lead-out**
        // Convert threshold from dB to linear ratio
        // dB = 20 * log10(ratio), so ratio = 10^(dB/20)
        let threshold_db = self.params.lead_in_threshold_db;  // Default: -12.0 dB
        let threshold_ratio = 10.0f64.powf(threshold_db / 20.0);
        let threshold = peak_rms * threshold_ratio;

        let (lead_in_duration, lead_out_duration) =
            self.detect_lead_in_out(&rms_profile, threshold as f32);

        // **Phase 5: Detect quick ramps**
        let (quick_ramp_up, quick_ramp_down) =
            self.detect_quick_ramps(&rms_profile, peak_rms as f32);

        tracing::info!(
            "Amplitude analysis complete: peak_rms={:.4}, lead_in={:.3}s, lead_out={:.3}s, quick_up={}, quick_down={}",
            peak_rms,
            lead_in_duration,
            lead_out_duration,
            quick_ramp_up,
            quick_ramp_down
        );

        Ok(AmplitudeAnalysisResult {
            peak_rms,
            lead_in_duration,
            lead_out_duration,
            quick_ramp_up,
            quick_ramp_down,
            rms_profile: Some(rms_profile),
        })
    }

    /// Calculate RMS envelope over sliding windows
    ///
    /// # Arguments
    /// * `samples` - Interleaved stereo samples (L, R, L, R, ...)
    /// * `sample_rate` - Sample rate in Hz
    ///
    /// # Returns
    /// Vec of RMS values, one per window
    ///
    /// # Algorithm
    /// 1. Convert window size from ms to samples
    /// 2. Iterate over windows (non-overlapping for efficiency)
    /// 3. For each window, calculate RMS: sqrt(mean(sample^2))
    fn calculate_rms_envelope(&self, samples: &[f32], sample_rate: u32) -> Vec<f32> {
        // Window size in samples (stereo frames * 2)
        let window_ms = self.params.rms_window_ms;
        let window_frames = ((sample_rate as f64 * window_ms as f64) / 1000.0) as usize;
        let window_samples = window_frames * 2;  // Stereo

        if samples.is_empty() || window_samples == 0 {
            return vec![];
        }

        let num_windows = (samples.len() / window_samples).max(1);
        let mut rms_values = Vec::with_capacity(num_windows);

        for window_idx in 0..num_windows {
            let start = window_idx * window_samples;
            let end = (start + window_samples).min(samples.len());

            if end <= start {
                break;
            }

            // Calculate RMS for this window
            let sum_squares: f64 = samples[start..end]
                .iter()
                .map(|&s| (s as f64) * (s as f64))
                .sum();

            let rms = (sum_squares / (end - start) as f64).sqrt() as f32;
            rms_values.push(rms);
        }

        tracing::debug!(
            "Calculated RMS envelope: {} windows ({}ms window, {} Hz)",
            rms_values.len(),
            window_ms,
            sample_rate
        );

        rms_values
    }

    /// Detect lead-in and lead-out durations
    ///
    /// # Arguments
    /// * `rms_profile` - RMS values over time
    /// * `threshold` - RMS threshold for detecting start/end of audio
    ///
    /// # Returns
    /// (lead_in_duration_secs, lead_out_duration_secs)
    ///
    /// # Algorithm
    /// - Lead-in: Find first window where RMS exceeds threshold
    /// - Lead-out: Find last window where RMS exceeds threshold, measure from that point to end
    fn detect_lead_in_out(&self, rms_profile: &[f32], threshold: f32) -> (f64, f64) {
        if rms_profile.is_empty() {
            return (0.0, 0.0);
        }

        // Time per window in seconds
        let window_duration_secs = (self.params.rms_window_ms as f64) / 1000.0;

        // Find lead-in: first window above threshold
        let lead_in_windows = rms_profile.iter().position(|&rms| rms >= threshold).unwrap_or(0);
        let lead_in_duration = lead_in_windows as f64 * window_duration_secs;

        // Find lead-out: last window above threshold
        let lead_out_windows = rms_profile.iter().rposition(|&rms| rms >= threshold).unwrap_or(rms_profile.len() - 1);
        let lead_out_duration = (rms_profile.len() - 1 - lead_out_windows) as f64 * window_duration_secs;

        (lead_in_duration, lead_out_duration)
    }

    /// Detect quick ramps (steep amplitude changes)
    ///
    /// # Arguments
    /// * `rms_profile` - RMS values over time
    /// * `peak_rms` - Peak RMS value
    ///
    /// # Returns
    /// (quick_ramp_up, quick_ramp_down)
    ///
    /// # Algorithm
    /// - Check first few windows for steep rise (quick_ramp_up)
    /// - Check last few windows for steep fall (quick_ramp_down)
    /// - "Quick" = reaching >50% of peak within 3 windows (default 300ms)
    fn detect_quick_ramps(&self, rms_profile: &[f32], peak_rms: f32) -> (bool, bool) {
        const QUICK_WINDOW_COUNT: usize = 3;  // 3 windows = ~300ms at 100ms/window
        const QUICK_THRESHOLD: f32 = 0.5;      // 50% of peak

        if rms_profile.len() < QUICK_WINDOW_COUNT {
            return (false, false);
        }

        let threshold = peak_rms * QUICK_THRESHOLD;

        // Quick ramp up: any of first 3 windows exceeds threshold
        let quick_ramp_up = rms_profile.iter().take(QUICK_WINDOW_COUNT).any(|&rms| rms >= threshold);

        // Quick ramp down: any of last 3 windows falls below threshold (coming from higher values)
        let last_windows = &rms_profile[rms_profile.len().saturating_sub(QUICK_WINDOW_COUNT)..];
        let quick_ramp_down = last_windows.iter().any(|&rms| rms < threshold);

        (quick_ramp_up, quick_ramp_down)
    }

    /// Batch analyze multiple files
    pub async fn analyze_batch(
        &self,
        files: &[(impl AsRef<Path>, f64, f64)],
    ) -> Vec<Result<AmplitudeAnalysisResult, AnalysisError>> {
        let mut results = Vec::with_capacity(files.len());

        for (path, start, end) in files {
            results.push(self.analyze_file(path.as_ref(), *start, *end).await);
        }

        results
    }
}

impl Default for AmplitudeAnalyzer {
    fn default() -> Self {
        Self::new(AmplitudeParameters::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyzer_creation() {
        let params = AmplitudeParameters::default();
        let analyzer = AmplitudeAnalyzer::new(params);
        assert_eq!(analyzer.params.rms_window_ms, 100);
    }

    #[test]
    fn test_calculate_rms_envelope() {
        let analyzer = AmplitudeAnalyzer::default();

        // Create 1 second of audio at 44.1 kHz (440 Hz sine wave)
        let sample_rate = 44100;
        let duration_secs = 1.0;
        let frequency = 440.0;
        let num_frames = (sample_rate as f64 * duration_secs) as usize;

        let mut samples = Vec::with_capacity(num_frames * 2);
        for i in 0..num_frames {
            let t = i as f64 / sample_rate as f64;
            let sample = (2.0 * std::f64::consts::PI * frequency * t).sin() as f32 * 0.5;
            samples.push(sample); // left
            samples.push(sample); // right
        }

        let rms_profile = analyzer.calculate_rms_envelope(&samples, sample_rate);

        // Should have 10 windows (100ms each over 1 second)
        assert_eq!(rms_profile.len(), 10, "Expected 10 windows for 1 second at 100ms/window");

        // RMS of sine wave should be approximately amplitude / sqrt(2)
        // Amplitude = 0.5, so RMS ≈ 0.5 / 1.414 ≈ 0.354
        for &rms in &rms_profile {
            assert!(
                rms >= 0.3 && rms <= 0.4,
                "RMS should be ~0.354, got {}",
                rms
            );
        }
    }

    #[test]
    fn test_detect_lead_in_out() {
        let analyzer = AmplitudeAnalyzer::default();

        // Create RMS profile: silence (0.0), rise to peak (0.5), sustain, fall to silence
        let rms_profile = vec![
            0.01, 0.02, 0.05,  // Lead-in (3 windows)
            0.5, 0.5, 0.5, 0.5, 0.5,  // Sustain (5 windows)
            0.05, 0.02, 0.01,  // Lead-out (3 windows)
        ];

        let threshold = 0.1;  // 10% of peak (0.5)

        let (lead_in, lead_out) = analyzer.detect_lead_in_out(&rms_profile, threshold);

        // Lead-in: ~3 windows * 100ms = 0.3s
        // Lead-out: ~3 windows * 100ms = 0.3s
        assert!((lead_in - 0.3).abs() < 0.05, "Lead-in should be ~0.3s, got {}", lead_in);
        assert!((lead_out - 0.3).abs() < 0.05, "Lead-out should be ~0.3s, got {}", lead_out);
    }

    #[test]
    fn test_detect_quick_ramps() {
        let analyzer = AmplitudeAnalyzer::default();
        let peak_rms = 0.8;

        // Test quick ramp up: high value in first 3 windows
        let rms_quick_up = vec![0.0, 0.0, 0.6, 0.8, 0.8, 0.8, 0.8, 0.8, 0.8];
        let (quick_up, _) = analyzer.detect_quick_ramps(&rms_quick_up, peak_rms);
        assert!(quick_up, "Should detect quick ramp up");

        // Test quick ramp down: low value in last 3 windows
        let rms_quick_down = vec![0.8, 0.8, 0.8, 0.8, 0.8, 0.8, 0.6, 0.0, 0.0];
        let (_, quick_down) = analyzer.detect_quick_ramps(&rms_quick_down, peak_rms);
        assert!(quick_down, "Should detect quick ramp down");

        // Test no quick ramps: gradual rise and fall
        // For no quick ramp: first 3 windows should stay below 50% of peak (0.8)
        // 50% of 0.8 = 0.4, so first 3 windows are 0.05, 0.1, 0.2 (all < 0.4)
        let rms_gradual = vec![0.05, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.7, 0.6, 0.5];
        let (no_quick_up, _) = analyzer.detect_quick_ramps(&rms_gradual, 0.8);
        assert!(!no_quick_up, "Should not detect quick ramp up for gradual rise");
    }

    #[test]
    fn test_empty_audio() {
        let analyzer = AmplitudeAnalyzer::default();

        let rms_profile = analyzer.calculate_rms_envelope(&[], 44100);
        assert!(rms_profile.is_empty(), "Empty input should produce empty RMS profile");

        let (lead_in, lead_out) = analyzer.detect_lead_in_out(&[], 0.1);
        assert_eq!(lead_in, 0.0, "Empty RMS profile should have 0 lead-in");
        assert_eq!(lead_out, 0.0, "Empty RMS profile should have 0 lead-out");
    }
}
