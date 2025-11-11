//! Test Fixture Generation for PLAN027 Sprint 3
//!
//! Run with: cargo test --test generate_test_fixtures -- --ignored --nocapture
//!
//! Generates audio test files for integration testing.

use hound::{SampleFormat, WavSpec, WavWriter};
use std::f32::consts::PI;
use std::fs;
use std::path::PathBuf;

/// Generate sine wave samples (stereo)
fn generate_sine_wave(frequency: f32, duration_secs: f32, sample_rate: u32) -> Vec<i16> {
    let num_samples = (duration_secs * sample_rate as f32) as usize;
    let mut samples = Vec::with_capacity(num_samples * 2); // Stereo

    for i in 0..num_samples {
        let t = i as f32 / sample_rate as f32;
        let sample = (2.0 * PI * frequency * t).sin();
        let amplitude = (sample * 0.5 * i16::MAX as f32) as i16; // 50% volume

        samples.push(amplitude); // Left channel
        samples.push(amplitude); // Right channel
    }

    samples
}

/// Generate silence samples (stereo)
fn generate_silence(duration_secs: f32, sample_rate: u32) -> Vec<i16> {
    let num_samples = (duration_secs * sample_rate as f32) as usize;
    vec![0i16; num_samples * 2] // Stereo
}

/// Write samples to WAV file
fn write_wav(path: &PathBuf, samples: &[i16], sample_rate: u32) {
    let spec = WavSpec {
        channels: 2,
        sample_rate,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    let mut writer = WavWriter::create(path, spec).expect("Failed to create WAV writer");

    for &sample in samples {
        writer.write_sample(sample).expect("Failed to write sample");
    }

    writer.finalize().expect("Failed to finalize WAV file");
}

#[test]
#[ignore] // Run explicitly with: cargo test --test generate_test_fixtures -- --ignored --nocapture
fn generate_all_test_fixtures() {
    let fixtures_dir = PathBuf::from("tests/fixtures/audio");
    fs::create_dir_all(&fixtures_dir).expect("Failed to create fixtures directory");

    println!("\n🔧 Generating test fixtures in {:?}...\n", fixtures_dir);

    // Fixture 1: Multi-track album (3 tracks with 2-second silence between)
    println!("1. Generating multi_track_album.wav (19 seconds, 3 tracks + silence)...");
    {
        let sample_rate = 44100;
        let mut samples = Vec::new();

        // Track 1: 440Hz (A4) - 5 seconds
        samples.extend(generate_sine_wave(440.0, 5.0, sample_rate));
        // Silence: 2 seconds
        samples.extend(generate_silence(2.0, sample_rate));

        // Track 2: 523.25Hz (C5) - 5 seconds
        samples.extend(generate_sine_wave(523.25, 5.0, sample_rate));
        // Silence: 2 seconds
        samples.extend(generate_silence(2.0, sample_rate));

        // Track 3: 659.25Hz (E5) - 5 seconds
        samples.extend(generate_sine_wave(659.25, 5.0, sample_rate));

        let path = fixtures_dir.join("multi_track_album.wav");
        write_wav(&path, &samples, sample_rate);
        println!("   ✓ Created: {} ({:.1}s, {} samples)",
                 path.display(), samples.len() as f32 / 2.0 / sample_rate as f32, samples.len() / 2);
    }

    // Fixture 2: Minimal valid (3 seconds for chromaprint minimum)
    println!("2. Generating minimal_valid.wav (3 seconds)...");
    {
        let sample_rate = 44100;
        let samples = generate_sine_wave(440.0, 3.0, sample_rate);

        let path = fixtures_dir.join("minimal_valid.wav");
        write_wav(&path, &samples, sample_rate);
        println!("   ✓ Created: {} ({:.1}s, {} samples)",
                 path.display(), samples.len() as f32 / 2.0 / sample_rate as f32, samples.len() / 2);
    }

    // Fixture 3: Short invalid (1 second - too short for chromaprint)
    println!("3. Generating short_invalid.wav (1 second - too short)...");
    {
        let sample_rate = 44100;
        let samples = generate_sine_wave(440.0, 1.0, sample_rate);

        let path = fixtures_dir.join("short_invalid.wav");
        write_wav(&path, &samples, sample_rate);
        println!("   ✓ Created: {} ({:.1}s, {} samples)",
                 path.display(), samples.len() as f32 / 2.0 / sample_rate as f32, samples.len() / 2);
    }

    // Fixture 4: No silence (single track, no gaps)
    println!("4. Generating no_silence.wav (5 seconds continuous)...");
    {
        let sample_rate = 44100;
        let samples = generate_sine_wave(440.0, 5.0, sample_rate);

        let path = fixtures_dir.join("no_silence.wav");
        write_wav(&path, &samples, sample_rate);
        println!("   ✓ Created: {} ({:.1}s, {} samples)",
                 path.display(), samples.len() as f32 / 2.0 / sample_rate as f32, samples.len() / 2);
    }

    println!("\n✅ All test fixtures generated successfully!");
    println!("\nFixtures location: {}\n", fixtures_dir.display());
}
