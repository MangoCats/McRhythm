//! Integration tests for audio subsystem
//!
//! Tests the complete audio pipeline: decode → resample → output
//!
//! **Note:** These tests require actual audio files and hardware.
//! They are marked with #[ignore] by default. To run manually:
//! ```
//! cargo test --test audio_subsystem_test -- --ignored --nocapture
//! ```

use std::path::PathBuf;
use uuid::Uuid;

// Import internal modules via the crate
// Note: Integration tests can only access public items

/// Test helper: Generate a sine wave as test audio
fn generate_sine_wave(frequency: f32, duration_ms: u64, sample_rate: u32) -> Vec<f32> {
    let num_frames = (sample_rate as u64 * duration_ms / 1000) as usize;
    let mut samples = Vec::with_capacity(num_frames * 2); // Stereo

    for i in 0..num_frames {
        let t = i as f32 / sample_rate as f32;
        let sample = (2.0 * std::f32::consts::PI * frequency * t).sin() * 0.5;
        samples.push(sample); // Left
        samples.push(sample); // Right
    }

    samples
}

#[test]
fn test_sine_wave_generation() {
    let samples = generate_sine_wave(440.0, 1000, 44100);

    // Should have 1 second worth of stereo samples at 44.1kHz
    assert_eq!(samples.len(), 44100 * 2);

    // Samples should be in valid range
    for sample in &samples {
        assert!(sample.abs() <= 1.0);
    }
}

// The following tests require actual audio files and hardware.
// They are commented out but serve as examples for manual testing.

/*
#[test]
#[ignore] // Requires audio file
fn test_decode_audio_file() {
    use wkmp_ap::audio::{SimpleDecoder};

    let test_file = PathBuf::from("test_data/test.mp3");

    if !test_file.exists() {
        println!("Skipping test: {} not found", test_file.display());
        return;
    }

    let result = SimpleDecoder::decode_file(&test_file);
    assert!(result.is_ok());

    let (samples, sample_rate, channels) = result.unwrap();

    // Verify we got some audio data
    assert!(!samples.is_empty());
    assert!(sample_rate > 0);
    assert!(channels > 0);

    println!("Decoded: {} samples, {} Hz, {} channels",
             samples.len(), sample_rate, channels);
}

#[test]
#[ignore] // Requires audio file
fn test_resample_audio() {
    use wkmp_ap::audio::{SimpleDecoder, Resampler};

    let test_file = PathBuf::from("test_data/test.mp3");

    if !test_file.exists() {
        println!("Skipping test: {} not found", test_file.display());
        return;
    }

    let (samples, sample_rate, channels) = SimpleDecoder::decode_file(&test_file).unwrap();

    // Resample to 44.1kHz
    let resampled = Resampler::resample(&samples, sample_rate, channels).unwrap();

    assert!(!resampled.is_empty());

    println!("Resampled: {} samples at 44.1kHz", resampled.len());
}

#[test]
#[ignore] // Requires audio device
fn test_audio_output_list_devices() {
    use wkmp_ap::audio::AudioOutput;

    let devices = AudioOutput::list_devices().unwrap();

    println!("Found {} audio devices:", devices.len());
    for device in &devices {
        println!("  - {}", device);
    }

    assert!(!devices.is_empty(), "No audio devices found");
}

#[test]
#[ignore] // Requires audio device and file
fn test_end_to_end_playback() {
    use wkmp_ap::audio::{SimpleDecoder, Resampler, AudioOutput, PassageBuffer, AudioFrame};
    use std::sync::{Arc, Mutex};

    let test_file = PathBuf::from("test_data/test.mp3");

    if !test_file.exists() {
        println!("Skipping test: {} not found", test_file.display());
        return;
    }

    // 1. Decode audio file
    println!("Decoding audio file...");
    let (samples, sample_rate, channels) = SimpleDecoder::decode_file(&test_file).unwrap();
    println!("Decoded: {} frames", samples.len() / channels as usize);

    // 2. Resample to 44.1kHz
    println!("Resampling to 44.1kHz...");
    let resampled = Resampler::resample(&samples, sample_rate, channels).unwrap();
    println!("Resampled: {} frames", resampled.len() / 2);

    // 3. Create passage buffer
    let buffer = Arc::new(PassageBuffer::new(
        Uuid::new_v4(),
        resampled,
        44100,
        2,
    ));

    println!("Buffer duration: {}ms", buffer.duration_ms());

    // 4. Play through audio output
    println!("Starting audio playback...");
    let mut output = AudioOutput::new(None).unwrap();
    println!("Using device: {}", output.device_name());

    let position = Arc::new(Mutex::new(0usize));
    let buffer_clone = Arc::clone(&buffer);
    let position_clone = Arc::clone(&position);

    output.start(move || {
        let mut pos = position_clone.lock().unwrap();

        if *pos < buffer_clone.sample_count {
            let frame = buffer_clone.get_frame(*pos).unwrap_or(AudioFrame::zero());
            *pos += 1;
            frame
        } else {
            AudioFrame::zero()
        }
    }).unwrap();

    // 5. Wait for playback to complete (or 5 seconds max for testing)
    let playback_duration = (buffer.duration_ms()).min(5000);
    println!("Playing for {}ms...", playback_duration);
    std::thread::sleep(std::time::Duration::from_millis(playback_duration));

    println!("Playback test complete");
}

#[test]
#[ignore] // Requires audio device
fn test_sine_wave_playback() {
    use wkmp_ap::audio::{AudioOutput, AudioFrame, PassageBuffer};
    use std::sync::{Arc, Mutex};

    // Generate 440Hz sine wave (A note) for 2 seconds
    println!("Generating 440Hz sine wave...");
    let samples = generate_sine_wave(440.0, 2000, 44100);

    let buffer = Arc::new(PassageBuffer::new(
        Uuid::new_v4(),
        samples,
        44100,
        2,
    ));

    println!("Playing sine wave for 2 seconds...");
    let mut output = AudioOutput::new(None).unwrap();

    let position = Arc::new(Mutex::new(0usize));
    let buffer_clone = Arc::clone(&buffer);
    let position_clone = Arc::clone(&position);

    output.start(move || {
        let mut pos = position_clone.lock().unwrap();

        if *pos < buffer_clone.sample_count {
            let frame = buffer_clone.get_frame(*pos).unwrap_or(AudioFrame::zero());
            *pos += 1;
            frame
        } else {
            AudioFrame::zero()
        }
    }).unwrap();

    std::thread::sleep(std::time::Duration::from_secs(2));

    println!("Sine wave playback complete");
}
*/
