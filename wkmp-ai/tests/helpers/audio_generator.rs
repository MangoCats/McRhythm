//! Audio Test Fixture Generator
//!
//! Utilities for generating test audio files with various characteristics

use std::path::{Path, PathBuf};

/// Configuration for generated audio
#[derive(Debug, Clone)]
pub struct AudioConfig {
    pub duration_seconds: f64,
    pub sample_rate: u32,
    pub channels: u16,
    pub silence_gap_start: Option<f64>,
    pub silence_gap_duration: Option<f64>,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            duration_seconds: 35.0,
            sample_rate: 44100,
            channels: 2,
            silence_gap_start: None,
            silence_gap_duration: None,
        }
    }
}

/// Generate a test WAV file with specified configuration
///
/// # Arguments
/// * `path` - Output file path
/// * `config` - Audio configuration
///
/// # Returns
/// Generated file path
pub fn generate_test_wav(path: &Path, config: &AudioConfig) -> anyhow::Result<PathBuf> {
    let spec = hound::WavSpec {
        channels: config.channels,
        sample_rate: config.sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::create(path, spec)?;
    let total_samples = (config.duration_seconds * config.sample_rate as f64) as usize;

    // Calculate silence region in samples
    let (silence_start, silence_end) = if let (Some(start), Some(duration)) =
        (config.silence_gap_start, config.silence_gap_duration)
    {
        let start_sample = (start * config.sample_rate as f64) as usize;
        let end_sample = start_sample + (duration * config.sample_rate as f64) as usize;
        (start_sample, end_sample)
    } else {
        (total_samples + 1, total_samples + 2) // No silence
    };

    // Generate audio with simple tone + optional silence gap
    for i in 0..total_samples {
        let sample = if i >= silence_start && i < silence_end {
            // Silence gap
            0
        } else {
            // Simple 440Hz tone at 30% amplitude
            let t = i as f32 / config.sample_rate as f32;
            let amplitude = 0.3;
            let freq = 440.0;
            (amplitude * (2.0 * std::f32::consts::PI * freq * t).sin() * i16::MAX as f32) as i16
        };

        // Write stereo samples
        for _ in 0..config.channels {
            writer.write_sample(sample)?;
        }
    }

    writer.finalize()?;
    Ok(path.to_path_buf())
}

/// Generate multiple test audio files in a directory
///
/// # Arguments
/// * `dir` - Output directory
/// * `count` - Number of files to generate
/// * `config` - Audio configuration (applied to all files)
///
/// # Returns
/// Vector of generated file paths
pub fn generate_test_library(
    dir: &Path,
    count: usize,
    config: &AudioConfig,
) -> anyhow::Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    for i in 0..count {
        let filename = format!("test_track_{:03}.wav", i + 1);
        let file_path = dir.join(filename);
        generate_test_wav(&file_path, config)?;
        files.push(file_path);
    }

    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_generate_simple_wav() {
        let temp_dir = TempDir::new().unwrap();
        let wav_path = temp_dir.path().join("test.wav");

        let config = AudioConfig::default();
        let result = generate_test_wav(&wav_path, &config);

        assert!(result.is_ok());
        assert!(wav_path.exists());

        // Verify file is non-empty
        let metadata = std::fs::metadata(&wav_path).unwrap();
        assert!(metadata.len() > 1000, "WAV file should be non-trivial size");
    }

    #[test]
    fn test_generate_wav_with_silence() {
        let temp_dir = TempDir::new().unwrap();
        let wav_path = temp_dir.path().join("test_silence.wav");

        let config = AudioConfig {
            duration_seconds: 10.0,
            silence_gap_start: Some(3.0),
            silence_gap_duration: Some(2.0),
            ..Default::default()
        };

        let result = generate_test_wav(&wav_path, &config);
        assert!(result.is_ok());
        assert!(wav_path.exists());
    }

    #[test]
    fn test_generate_library() {
        let temp_dir = TempDir::new().unwrap();
        let config = AudioConfig {
            duration_seconds: 5.0,
            ..Default::default()
        };

        let files = generate_test_library(temp_dir.path(), 3, &config);
        assert!(files.is_ok());

        let files = files.unwrap();
        assert_eq!(files.len(), 3);

        for file in files {
            assert!(file.exists());
        }
    }
}
