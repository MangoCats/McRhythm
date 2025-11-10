//! Amplitude-based lead-in/lead-out detection
//!
//! **[AIA-COMP-010]** RMS-based amplitude analysis (simplified stub)
//!
//! Per [IMPL009](../../docs/IMPL009-amplitude_analyzer_implementation.md)
//!
//! NOTE: This is a simplified stub implementation. Full implementation requires
//! symphonia for audio decoding and dasp for signal processing.

use std::path::Path;
use thiserror::Error;

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
    /// Peak RMS level (dB)
    pub peak_rms: f64,

    /// Lead-in duration (seconds)
    pub lead_in_duration: f64,

    /// Lead-out duration (seconds)
    pub lead_out_duration: f64,

    /// Quick ramp up detected
    pub quick_ramp_up: bool,

    /// Quick ramp down detected
    pub quick_ramp_down: bool,

    /// RMS envelope (optional, for debugging)
    pub rms_profile: Option<Vec<f32>>,
}

/// Amplitude analyzer service
pub struct AmplitudeAnalyzer {
    #[allow(dead_code)]
    params: AmplitudeParameters,
}

impl AmplitudeAnalyzer {
    /// Create new amplitude analyzer with specified parameters
    pub fn new(params: AmplitudeParameters) -> Self {
        Self { params }
    }

    /// Analyze audio file for lead-in/lead-out timing
    ///
    /// **[AIA-COMP-010]** Real implementation using symphonia
    pub async fn analyze_file(
        &self,
        file_path: &Path,
        start_time: f64,
        end_time: f64,
    ) -> Result<AmplitudeAnalysisResult, AnalysisError> {
        use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
        use symphonia::core::formats::FormatOptions;
        use symphonia::core::io::MediaSourceStream;
        use symphonia::core::meta::MetadataOptions;
        use symphonia::core::probe::Hint;
        use std::fs::File;

        tracing::debug!(
            file = %file_path.display(),
            start = start_time,
            end = end_time,
            "Amplitude analysis (real implementation)"
        );

        // Open audio file
        let file = File::open(file_path)
            .map_err(|e| AnalysisError::ReadError(e.to_string()))?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        let mut hint = Hint::new();
        if let Some(ext) = file_path.extension() {
            hint.with_extension(ext.to_str().unwrap_or(""));
        }

        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
            .map_err(|e| AnalysisError::UnsupportedFormat(e.to_string()))?;

        let mut format = probed.format;
        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or_else(|| AnalysisError::UnsupportedFormat("No valid audio track".to_string()))?;

        let track_id = track.id;
        let sample_rate = track.codec_params.sample_rate.unwrap_or(44100);

        let mut decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())
            .map_err(|e| AnalysisError::UnsupportedFormat(e.to_string()))?;

        // Decode audio and calculate RMS
        let mut all_samples = Vec::new();
        loop {
            match format.next_packet() {
                Ok(packet) if packet.track_id() == track_id => {
                    match decoder.decode(&packet) {
                        Ok(decoded) => {
                            let samples = self.extract_samples_mono(&decoded)
                                .map_err(AnalysisError::AnalysisFailed)?;
                            all_samples.extend(samples);
                        }
                        Err(_) => continue,
                    }
                }
                Ok(_) => continue,
                Err(_) => break,
            }
        }

        // Calculate RMS profile
        let window_size = (sample_rate as f64 * 0.1) as usize; // 100ms windows
        let rms_profile = self.calculate_rms_profile(&all_samples, window_size);

        // Detect peak and lead-in/lead-out
        let peak_rms = rms_profile.iter().cloned().fold(0.0f32, f32::max) as f64;
        let threshold = peak_rms * 0.1; // 10% of peak

        // Find lead-in: first point exceeding threshold
        let lead_in_windows = rms_profile
            .iter()
            .position(|&v| v as f64 > threshold)
            .unwrap_or(0);
        let lead_in_duration = (lead_in_windows as f64 * 0.1).max(0.1);

        // Find lead-out: last point exceeding threshold
        let lead_out_windows = rms_profile
            .iter()
            .rposition(|&v| v as f64 > threshold)
            .unwrap_or(rms_profile.len());
        let lead_out_duration = ((rms_profile.len() - lead_out_windows) as f64 * 0.1).max(0.1);

        // Detect quick ramps (>50% change in <0.5s)
        let quick_ramp_up = self.detect_quick_ramp(&rms_profile[..10.min(rms_profile.len())]);
        let quick_ramp_down = self.detect_quick_ramp(
            &rms_profile[rms_profile.len().saturating_sub(10)..]
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

    /// Extract mono samples from decoded audio
    fn extract_samples_mono(&self, buffer: &symphonia::core::audio::AudioBufferRef) -> Result<Vec<f32>, String> {
        use symphonia::core::audio::{AudioBufferRef, Signal};

        match buffer {
            AudioBufferRef::F32(buf) => {
                let num_channels = buf.spec().channels.count();
                let num_frames = buf.frames();
                let mut samples = Vec::with_capacity(num_frames);

                for frame in 0..num_frames {
                    let mut sum = 0.0f32;
                    for ch in 0..num_channels {
                        sum += buf.chan(ch)[frame];
                    }
                    samples.push(sum / num_channels as f32);
                }
                Ok(samples)
            }
            AudioBufferRef::S16(buf) => {
                let num_channels = buf.spec().channels.count();
                let num_frames = buf.frames();
                let mut samples = Vec::with_capacity(num_frames);

                for frame in 0..num_frames {
                    let mut sum = 0.0f32;
                    for ch in 0..num_channels {
                        sum += buf.chan(ch)[frame] as f32 / 32768.0;
                    }
                    samples.push(sum / num_channels as f32);
                }
                Ok(samples)
            }
            _ => Err("Unsupported audio buffer format".to_string()),
        }
    }

    /// Calculate RMS profile with windowing
    fn calculate_rms_profile(&self, samples: &[f32], window_size: usize) -> Vec<f32> {
        samples
            .chunks(window_size)
            .map(|chunk| {
                let sum_squares: f32 = chunk.iter().map(|s| s * s).sum();
                (sum_squares / chunk.len() as f32).sqrt()
            })
            .collect()
    }

    /// Detect quick ramp (>50% change in window)
    fn detect_quick_ramp(&self, windows: &[f32]) -> bool {
        if windows.len() < 2 {
            return false;
        }

        let first = windows[0] as f64;
        let last = windows[windows.len() - 1] as f64;
        let change = (last - first).abs() / first.max(0.001);

        change > 0.5 // >50% change
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

    #[tokio::test]
    async fn test_analyze_constant_amplitude() {
        use hound::WavWriter;
        use tempfile::NamedTempFile;

        // Create a test WAV file with constant amplitude
        let temp_file = NamedTempFile::new().unwrap();
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 44100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = WavWriter::create(temp_file.path(), spec).unwrap();

        // Write 1 second of audio (440Hz sine wave at constant amplitude)
        for t in 0..44100 {
            let sample = (t as f32 * 440.0 * 2.0 * std::f32::consts::PI / 44100.0).sin();
            writer.write_sample((sample * i16::MAX as f32) as i16).unwrap();
        }
        writer.finalize().unwrap();

        // Analyze the file
        let analyzer = AmplitudeAnalyzer::default();
        let result = analyzer
            .analyze_file(temp_file.path(), 0.0, 1.0)
            .await;

        assert!(result.is_ok());
        let analysis = result.unwrap();
        assert!(analysis.peak_rms > 0.0);
        assert!(analysis.peak_rms < 1.0); // Should be normalized
        assert!(analysis.lead_in_duration >= 0.0);
        assert!(analysis.lead_out_duration >= 0.0);
        // Constant amplitude should not have quick ramps
        assert!(!analysis.quick_ramp_up);
        assert!(!analysis.quick_ramp_down);
    }

    #[tokio::test]
    async fn test_analyze_with_ramp_up() {
        use hound::WavWriter;
        use tempfile::NamedTempFile;

        // Create a test WAV file with ramp up at the start
        let temp_file = NamedTempFile::new().unwrap();
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 44100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = WavWriter::create(temp_file.path(), spec).unwrap();

        // Write 2 seconds: 0.5s ramp up, then 1.5s constant
        for t in 0..(44100 * 2) {
            let ramp = if t < 22050 {
                (t as f32 / 22050.0) // Linear ramp from 0 to 1 over 0.5s
            } else {
                1.0
            };
            let sample = (t as f32 * 440.0 * 2.0 * std::f32::consts::PI / 44100.0).sin() * ramp;
            writer.write_sample((sample * i16::MAX as f32) as i16).unwrap();
        }
        writer.finalize().unwrap();

        // Analyze the file
        let analyzer = AmplitudeAnalyzer::default();
        let result = analyzer
            .analyze_file(temp_file.path(), 0.0, 2.0)
            .await;

        assert!(result.is_ok());
        let analysis = result.unwrap();
        assert!(analysis.peak_rms > 0.0);
        // Should detect quick ramp up (100% change in 0.5s)
        assert!(analysis.quick_ramp_up);
        // Should have a lead-in duration close to the ramp duration
        assert!(analysis.lead_in_duration > 0.0);
    }

    #[tokio::test]
    async fn test_analyze_with_silence() {
        use hound::WavWriter;
        use tempfile::NamedTempFile;

        // Create a test WAV file with silence at start and end
        let temp_file = NamedTempFile::new().unwrap();
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 44100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = WavWriter::create(temp_file.path(), spec).unwrap();

        // Write 3 seconds: 1s silence, 1s audio, 1s silence
        for t in 0..(44100 * 3) {
            let sample = if t < 44100 || t >= (44100 * 2) {
                0.0 // Silence (f32)
            } else {
                let audio_t = t - 44100;
                (audio_t as f32 * 440.0 * 2.0 * std::f32::consts::PI / 44100.0).sin()
            };
            writer.write_sample((sample * i16::MAX as f32) as i16).unwrap();
        }
        writer.finalize().unwrap();

        // Analyze the file
        let analyzer = AmplitudeAnalyzer::default();
        let result = analyzer
            .analyze_file(temp_file.path(), 0.0, 3.0)
            .await;

        assert!(result.is_ok());
        let analysis = result.unwrap();
        assert!(analysis.peak_rms > 0.0);
        // Should detect lead-in close to 1 second (silence before audio)
        assert!(analysis.lead_in_duration >= 0.9);
        // Should detect lead-out close to 1 second (silence after audio)
        assert!(analysis.lead_out_duration >= 0.9);
    }

    #[tokio::test]
    async fn test_analyze_stereo_file() {
        use hound::WavWriter;
        use tempfile::NamedTempFile;

        // Create a stereo WAV file
        let temp_file = NamedTempFile::new().unwrap();
        let spec = hound::WavSpec {
            channels: 2, // Stereo
            sample_rate: 44100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = WavWriter::create(temp_file.path(), spec).unwrap();

        // Write 1 second of stereo audio (440Hz sine wave in both channels)
        for t in 0..44100 {
            let sample = (t as f32 * 440.0 * 2.0 * std::f32::consts::PI / 44100.0).sin();
            let sample_i16 = (sample * i16::MAX as f32) as i16;
            writer.write_sample(sample_i16).unwrap(); // Left
            writer.write_sample(sample_i16).unwrap(); // Right
        }
        writer.finalize().unwrap();

        // Analyze the file (should convert to mono)
        let analyzer = AmplitudeAnalyzer::default();
        let result = analyzer
            .analyze_file(temp_file.path(), 0.0, 1.0)
            .await;

        assert!(result.is_ok());
        let analysis = result.unwrap();
        assert!(analysis.peak_rms > 0.0);
    }

    #[tokio::test]
    async fn test_analyze_nonexistent_file() {
        let analyzer = AmplitudeAnalyzer::default();
        let result = analyzer
            .analyze_file(std::path::Path::new("/nonexistent/file.wav"), 0.0, 1.0)
            .await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AnalysisError::ReadError(_)));
    }

    #[tokio::test]
    async fn test_rms_profile_generation() {
        use hound::WavWriter;
        use tempfile::NamedTempFile;

        // Create a test WAV file
        let temp_file = NamedTempFile::new().unwrap();
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 44100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut writer = WavWriter::create(temp_file.path(), spec).unwrap();

        // Write 1 second of audio
        for t in 0..44100 {
            let sample = (t as f32 * 440.0 * 2.0 * std::f32::consts::PI / 44100.0).sin();
            writer.write_sample((sample * i16::MAX as f32) as i16).unwrap();
        }
        writer.finalize().unwrap();

        // Analyze the file
        let analyzer = AmplitudeAnalyzer::default();
        let result = analyzer
            .analyze_file(temp_file.path(), 0.0, 1.0)
            .await;

        assert!(result.is_ok());
        let analysis = result.unwrap();

        // RMS profile should be generated (100ms windows for 1 second = ~10 windows)
        assert!(analysis.rms_profile.is_some());
        let profile = analysis.rms_profile.unwrap();
        assert!(profile.len() >= 9); // At least 9 windows (allowing for edge effects)
        assert!(profile.len() <= 11); // At most 11 windows

        // All RMS values should be positive
        for rms in profile {
            assert!(rms > 0.0);
        }
    }

    #[tokio::test]
    async fn test_batch_analyze() {
        use hound::WavWriter;
        use tempfile::TempDir;

        // Create multiple test files
        let temp_dir = TempDir::new().unwrap();
        let mut files = Vec::new();

        for i in 0..3 {
            let file_path = temp_dir.path().join(format!("test{}.wav", i));
            let spec = hound::WavSpec {
                channels: 1,
                sample_rate: 44100,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            };

            let mut writer = WavWriter::create(&file_path, spec).unwrap();

            // Write 1 second of audio
            for t in 0..44100 {
                let sample = (t as f32 * 440.0 * 2.0 * std::f32::consts::PI / 44100.0).sin();
                writer.write_sample((sample * i16::MAX as f32) as i16).unwrap();
            }
            writer.finalize().unwrap();

            files.push((file_path, 0.0, 1.0));
        }

        // Batch analyze
        let analyzer = AmplitudeAnalyzer::default();
        let results = analyzer.analyze_batch(&files).await;

        assert_eq!(results.len(), 3);
        for result in results {
            assert!(result.is_ok());
        }
    }
}
