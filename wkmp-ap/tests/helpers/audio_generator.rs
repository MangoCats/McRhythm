//! Audio Test File Generation Utilities
//!
//! **[PHASE1-INTEGRITY]** Generate deterministic WAV files for pipeline testing
//!
//! This module provides utilities to generate simple audio test files with known
//! characteristics for validating pipeline integrity:
//! - Silent audio (all zeros)
//! - Sine waves at specific frequencies
//!
//! These files are used to test sample conservation laws through the
//! decoder → buffer → mixer pipeline.

use hound::{WavSpec, WavWriter};
use std::f32::consts::PI;
use std::path::Path;

/// Standard test sample rate (44.1 kHz)
const TEST_SAMPLE_RATE: u32 = 44100;

/// Generate silent stereo WAV file
///
/// **[PHASE1-INTEGRITY]** Creates deterministic silent audio for baseline testing
///
/// # Arguments
/// * `path` - Output file path
/// * `duration_ms` - Duration in milliseconds
///
/// # Returns
/// * `Ok(())` - File created successfully
/// * `Err` - I/O or encoding error
///
/// # Example
/// ```no_run
/// # use std::path::Path;
/// generate_silent_wav(Path::new("/tmp/silent_1s.wav"), 1000)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn generate_silent_wav<P: AsRef<Path>>(path: P, duration_ms: u64) -> Result<(), hound::Error> {
    let spec = WavSpec {
        channels: 2,
        sample_rate: TEST_SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = WavWriter::create(path, spec)?;

    // Calculate total samples needed
    let total_frames = (TEST_SAMPLE_RATE as u64 * duration_ms) / 1000;
    let total_samples = total_frames * 2; // stereo

    // Write silence (zeros)
    for _ in 0..total_samples {
        writer.write_sample(0i16)?;
    }

    writer.finalize()?;
    Ok(())
}

/// Generate sine wave stereo WAV file
///
/// **[PHASE1-INTEGRITY]** Creates deterministic sine wave audio for testing
///
/// # Arguments
/// * `path` - Output file path
/// * `duration_ms` - Duration in milliseconds
/// * `frequency_hz` - Sine wave frequency in Hz (e.g., 440.0 for A4)
/// * `amplitude` - Amplitude 0.0-1.0 (0.5 recommended to avoid clipping)
///
/// # Returns
/// * `Ok(())` - File created successfully
/// * `Err` - I/O or encoding error
///
/// # Example
/// ```no_run
/// # use std::path::Path;
/// // Generate 1 second of 440 Hz sine wave at 50% amplitude
/// generate_sine_wav(Path::new("/tmp/sine_440hz_1s.wav"), 1000, 440.0, 0.5)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn generate_sine_wav<P: AsRef<Path>>(
    path: P,
    duration_ms: u64,
    frequency_hz: f32,
    amplitude: f32,
) -> Result<(), hound::Error> {
    let spec = WavSpec {
        channels: 2,
        sample_rate: TEST_SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = WavWriter::create(path, spec)?;

    // Calculate total frames
    let total_frames = (TEST_SAMPLE_RATE as u64 * duration_ms) / 1000;

    // Generate sine wave
    let amplitude_i16 = (amplitude * i16::MAX as f32) as i16;

    for frame_idx in 0..total_frames {
        let t = frame_idx as f32 / TEST_SAMPLE_RATE as f32;
        let sample_value = (2.0 * PI * frequency_hz * t).sin();
        let sample_i16 = (sample_value * amplitude_i16 as f32) as i16;

        // Write same value to both channels (stereo)
        writer.write_sample(sample_i16)?;
        writer.write_sample(sample_i16)?;
    }

    writer.finalize()?;
    Ok(())
}

/// Generate chirp (frequency sweep) stereo WAV file
///
/// **[PHASE1-INTEGRITY]** Creates frequency sweep for testing decoder/resampler
///
/// # Arguments
/// * `path` - Output file path
/// * `duration_ms` - Duration in milliseconds
/// * `start_freq_hz` - Starting frequency in Hz
/// * `end_freq_hz` - Ending frequency in Hz
/// * `amplitude` - Amplitude 0.0-1.0 (0.5 recommended)
///
/// # Returns
/// * `Ok(())` - File created successfully
/// * `Err` - I/O or encoding error
///
/// # Example
/// ```no_run
/// # use std::path::Path;
/// // Generate 2 second chirp from 100 Hz to 10 kHz
/// generate_chirp_wav(Path::new("/tmp/chirp_100_10k.wav"), 2000, 100.0, 10000.0, 0.5)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn generate_chirp_wav<P: AsRef<Path>>(
    path: P,
    duration_ms: u64,
    start_freq_hz: f32,
    end_freq_hz: f32,
    amplitude: f32,
) -> Result<(), hound::Error> {
    let spec = WavSpec {
        channels: 2,
        sample_rate: TEST_SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = WavWriter::create(path, spec)?;

    let total_frames = (TEST_SAMPLE_RATE as u64 * duration_ms) / 1000;
    let amplitude_i16 = (amplitude * i16::MAX as f32) as i16;
    let duration_s = duration_ms as f32 / 1000.0;

    for frame_idx in 0..total_frames {
        let t = frame_idx as f32 / TEST_SAMPLE_RATE as f32;
        let progress = t / duration_s;

        // Linear frequency sweep
        let freq = start_freq_hz + (end_freq_hz - start_freq_hz) * progress;

        // Instantaneous phase calculation
        let phase = 2.0 * PI * freq * t;
        let sample_value = phase.sin();
        let sample_i16 = (sample_value * amplitude_i16 as f32) as i16;

        // Write same value to both channels (stereo)
        writer.write_sample(sample_i16)?;
        writer.write_sample(sample_i16)?;
    }

    writer.finalize()?;
    Ok(())
}

/// Calculate exact sample count for a duration
///
/// **[PHASE1-INTEGRITY]** Helper to compute expected sample counts for validation
///
/// # Arguments
/// * `duration_ms` - Duration in milliseconds
///
/// # Returns
/// Total number of stereo samples (frames × 2)
///
/// # Example
/// ```
/// # use wkmp_ap::tests::helpers::audio_generator::calculate_sample_count;
/// let samples = calculate_sample_count(1000); // 1 second
/// assert_eq!(samples, 88200); // 44100 frames × 2 channels
/// ```
pub fn calculate_sample_count(duration_ms: u64) -> u64 {
    let frames = (TEST_SAMPLE_RATE as u64 * duration_ms) / 1000;
    frames * 2 // stereo
}

/// Calculate exact frame count for a duration
///
/// **[PHASE1-INTEGRITY]** Helper to compute expected frame counts for validation
///
/// # Arguments
/// * `duration_ms` - Duration in milliseconds
///
/// # Returns
/// Total number of stereo frames
///
/// # Example
/// ```
/// # use wkmp_ap::tests::helpers::audio_generator::calculate_frame_count;
/// let frames = calculate_frame_count(1000); // 1 second
/// assert_eq!(frames, 44100); // 44.1 kHz
/// ```
pub fn calculate_frame_count(duration_ms: u64) -> u64 {
    (TEST_SAMPLE_RATE as u64 * duration_ms) / 1000
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_generate_silent_wav() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("silent_test.wav");

        // Generate 100ms silent file
        generate_silent_wav(&path, 100).unwrap();

        // Verify file exists and has reasonable size
        let metadata = fs::metadata(&path).unwrap();
        assert!(metadata.len() > 0);
        assert!(metadata.len() < 100_000); // Should be small for 100ms

        // Clean up handled by TempDir drop
    }

    #[test]
    fn test_generate_sine_wav() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("sine_test.wav");

        // Generate 100ms sine wave at 440 Hz
        generate_sine_wav(&path, 100, 440.0, 0.5).unwrap();

        // Verify file exists
        let metadata = fs::metadata(&path).unwrap();
        assert!(metadata.len() > 0);
    }

    #[test]
    fn test_generate_chirp_wav() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("chirp_test.wav");

        // Generate 200ms chirp from 100 Hz to 1000 Hz
        generate_chirp_wav(&path, 200, 100.0, 1000.0, 0.5).unwrap();

        // Verify file exists
        let metadata = fs::metadata(&path).unwrap();
        assert!(metadata.len() > 0);
    }

    #[test]
    fn test_calculate_sample_count() {
        // 1 second @ 44.1kHz stereo = 88,200 samples
        assert_eq!(calculate_sample_count(1000), 88_200);

        // 100ms @ 44.1kHz stereo = 8,820 samples
        assert_eq!(calculate_sample_count(100), 8_820);

        // 0ms = 0 samples
        assert_eq!(calculate_sample_count(0), 0);
    }

    #[test]
    fn test_calculate_frame_count() {
        // 1 second @ 44.1kHz = 44,100 frames
        assert_eq!(calculate_frame_count(1000), 44_100);

        // 100ms @ 44.1kHz = 4,410 frames
        assert_eq!(calculate_frame_count(100), 4_410);

        // 0ms = 0 frames
        assert_eq!(calculate_frame_count(0), 0);
    }
}
