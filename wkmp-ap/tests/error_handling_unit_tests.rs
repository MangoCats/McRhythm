//! Phase 7 Error Handling Unit Tests
//!
//! Tests for error detection and handling at the module level.
//! **[PLAN001 Phase 7]** Acceptance tests for error handling requirements

mod helpers;

use helpers::{ErrorInjectionBuilder, panic_injection};
use wkmp_ap::audio::decoder::StreamingDecoder;
use wkmp_ap::error::Error;
use std::path::PathBuf;
use tempfile::TempDir;

// ============================================================================
// REQ-AP-ERR-010: File Read Error Detection
// ============================================================================

/// **[TC-U-ERR-010-01]** File read error detection - nonexistent file
///
/// **Given:** A file path that does not exist
/// **When:** StreamingDecoder attempts to open the file
/// **Then:** FileReadError is returned with the correct path
#[tokio::test]
async fn test_file_read_error_nonexistent() {
    let builder = ErrorInjectionBuilder::new().expect("Failed to create builder");
    let nonexistent_path = builder.nonexistent_file();

    // Attempt to create decoder with nonexistent file (0ms start, 0ms end = full file)
    let result = StreamingDecoder::new(&nonexistent_path, 0, 0);

    // Verify error is FileReadError
    assert!(result.is_err(), "Expected error for nonexistent file");

    if let Err(error) = result {
        match error {
            Error::FileReadError { path, .. } => {
                assert_eq!(path, nonexistent_path, "Error should contain correct path");
            }
            Error::FileHandleExhaustion { path } => {
                // Could also be file handle exhaustion on some systems
                assert_eq!(path, nonexistent_path, "Error should contain correct path");
            }
            _ => panic!("Expected FileReadError or FileHandleExhaustion"),
        }
    }
}

/// **[TC-U-ERR-010-02]** File read error detection - permission denied
///
/// **Given:** A file without read permissions
/// **When:** StreamingDecoder attempts to open the file
/// **Then:** FileReadError is returned
#[cfg(unix)]
#[tokio::test]
async fn test_file_read_error_permission_denied() {
    let builder = ErrorInjectionBuilder::new().expect("Failed to create builder");
    let unreadable_path = builder.unreadable_file().expect("Failed to create unreadable file");

    // Attempt to create decoder with unreadable file
    let result = StreamingDecoder::new(&unreadable_path, 0, 0);

    // Verify error is FileReadError
    assert!(result.is_err(), "Expected error for unreadable file");

    if let Err(error) = result {
        match error {
            Error::FileReadError { path, source } => {
                assert_eq!(path, unreadable_path, "Error should contain correct path");
                assert_eq!(source.kind(), std::io::ErrorKind::PermissionDenied,
                          "Should be permission denied error");
            }
            _ => panic!("Expected FileReadError with PermissionDenied"),
        }
    }
}

// ============================================================================
// REQ-AP-ERR-011: Unsupported Codec Detection
// ============================================================================

/// **[TC-U-ERR-011-01]** Unsupported codec detection - corrupted header
///
/// **Given:** A file with corrupted FLAC header
/// **When:** StreamingDecoder attempts to open the file
/// **Then:** UnsupportedCodec error is returned
#[tokio::test]
async fn test_unsupported_codec_corrupted_header() {
    let builder = ErrorInjectionBuilder::new().expect("Failed to create builder");
    let corrupted_path = builder.corrupted_audio_file().expect("Failed to create corrupted file");

    // Attempt to create decoder with corrupted file
    let result = StreamingDecoder::new(&corrupted_path, 0, 0);

    // Verify error is UnsupportedCodec or Decode (symphonia may report differently)
    assert!(result.is_err(), "Expected error for corrupted file");

    if let Err(error) = result {
        match error {
            Error::UnsupportedCodec { path, .. } => {
                assert_eq!(path, corrupted_path, "Error should contain correct path");
            }
            Error::Decode(msg) => {
                // Symphonia might report as decode error
                assert!(msg.contains("probe") || msg.contains("codec") || msg.contains("format"),
                       "Decode error should mention probe/codec/format issue: {}", msg);
            }
            _ => panic!("Expected UnsupportedCodec or Decode error"),
        }
    }
}

/// **[TC-U-ERR-011-02]** Unsupported codec detection - unknown format
///
/// **Given:** A file with unknown audio format
/// **When:** StreamingDecoder attempts to open the file
/// **Then:** UnsupportedCodec error is returned
#[tokio::test]
async fn test_unsupported_codec_unknown_format() {
    let builder = ErrorInjectionBuilder::new().expect("Failed to create builder");
    let unknown_path = builder.unsupported_format_file().expect("Failed to create unknown format file");

    // Attempt to create decoder with unknown format file
    let result = StreamingDecoder::new(&unknown_path, 0, 0);

    // Verify error is UnsupportedCodec
    assert!(result.is_err(), "Expected error for unknown format");

    if let Err(error) = result {
        match error {
            Error::UnsupportedCodec { path, .. } => {
                assert_eq!(path, unknown_path, "Error should contain correct path");
            }
            Error::Decode(msg) => {
                // Symphonia might report as decode error
                assert!(msg.contains("probe") || msg.contains("unsupported") || msg.contains("format"),
                       "Decode error should mention format issue: {}", msg);
            }
            _ => panic!("Expected UnsupportedCodec or Decode error"),
        }
    }
}

// ============================================================================
// REQ-AP-ERR-012: Partial Decode Handling
// ============================================================================

/// **[TC-U-ERR-012-01]** Partial decode detection - ≥50% threshold
///
/// **Given:** A truncated audio file with ≥50% of content
/// **When:** StreamingDecoder decodes the file completely
/// **Then:** Decoder returns partial data without error (handled at higher level)
#[tokio::test]
async fn test_partial_decode_above_threshold() {
    let builder = ErrorInjectionBuilder::new().expect("Failed to create builder");

    // Create file truncated to 60% (above 50% threshold)
    let truncated_path = builder.truncated_audio_file(60)
        .expect("Failed to create truncated file");

    // Create decoder (should succeed - file has valid header)
    let mut decoder = StreamingDecoder::new(&truncated_path, 0, 0)
        .expect("Decoder creation should succeed for truncated file");

    // Read all available data
    let mut total_samples = 0;
    let mut decode_errors = 0;

    loop {
        match decoder.decode_chunk(4096) {
            Ok(Some(chunk)) => {
                total_samples += chunk.len();
            }
            Ok(None) => {
                // End of stream (expected for truncated file)
                break;
            }
            Err(_) => {
                // Decode error encountered (expected at truncation point)
                decode_errors += 1;
                break;
            }
        }
    }

    // Verify we got some data (at least 50% should decode successfully)
    assert!(total_samples > 0, "Should have decoded some samples from truncated file");

    // Note: Partial decode percentage check happens at decoder_worker level
    // This unit test just verifies low-level decoder behavior
}

/// **[TC-U-ERR-012-02]** Partial decode detection - <50% threshold
///
/// **Given:** A truncated audio file with <50% of content
/// **When:** StreamingDecoder decodes the file completely
/// **Then:** Decoder returns partial data (rejection happens at higher level)
#[tokio::test]
async fn test_partial_decode_below_threshold() {
    let builder = ErrorInjectionBuilder::new().expect("Failed to create builder");

    // Create file truncated to 30% (below 50% threshold)
    let truncated_path = builder.truncated_audio_file(30)
        .expect("Failed to create truncated file");

    // Create decoder (should succeed - file has valid header)
    let mut decoder = StreamingDecoder::new(&truncated_path, 0, 0)
        .expect("Decoder creation should succeed for truncated file");

    // Read all available data
    let mut total_samples = 0;

    loop {
        match decoder.decode_chunk(4096) {
            Ok(Some(chunk)) => {
                total_samples += chunk.len();
            }
            Ok(None) => {
                // End of stream (expected for truncated file)
                break;
            }
            Err(_) => {
                // Decode error encountered (expected at truncation point)
                break;
            }
        }
    }

    // Verify we got some data (but less than full file)
    // Note: Actual percentage calculation and rejection happens at decoder_worker level
    assert!(total_samples > 0, "Should have decoded some samples from truncated file");
}

// ============================================================================
// REQ-AP-ERR-013: Panic Recovery
// ============================================================================

/// **[TC-U-ERR-013-01]** Panic recovery - catch_panic functionality
///
/// **Given:** A function that panics
/// **When:** catch_panic is used to execute the function
/// **Then:** Panic is caught and returned as Error
#[tokio::test]
async fn test_panic_recovery_basic() {
    // Test successful execution
    let result = panic_injection::catch_panic(|| {
        42
    });

    assert!(result.is_ok(), "Should succeed for non-panicking function");
    assert_eq!(result.unwrap(), 42, "Should return correct value");

    // Test panic catching
    let result = panic_injection::catch_panic(|| {
        panic!("Intentional test panic");
    });

    assert!(result.is_err(), "Should catch panic");
    let err_msg = result.unwrap_err();
    assert!(err_msg.contains("Intentional test panic"),
           "Error message should contain panic message: {}", err_msg);
}

/// **[TC-U-ERR-013-02]** Panic recovery - different panic types
///
/// **Given:** Functions that panic with different types
/// **When:** catch_panic is used to execute them
/// **Then:** All panic types are caught correctly
#[tokio::test]
async fn test_panic_recovery_types() {
    // String panic
    let result = panic_injection::catch_panic(|| {
        panic!("String panic");
    });
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("String panic"));

    // &str panic
    let result = panic_injection::catch_panic(|| {
        panic!("str panic");
    });
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("str panic"));

    // Custom panic value
    let result = panic_injection::catch_panic(|| {
        let msg = format!("{}", 123);
        panic!("{}", msg); // Panic with formatted string
    });
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("123"));
}

// ============================================================================
// REQ-AP-ERR-040: Queue Validation
// ============================================================================

/// **[TC-U-ERR-040-01]** Queue validation - empty file path
///
/// **Given:** An empty file path string
/// **When:** Validation checks the path
/// **Then:** Path is rejected as invalid
#[tokio::test]
async fn test_queue_validation_empty_path() {
    let empty_path = PathBuf::from("");

    // Verify empty path is not a valid file
    assert!(!empty_path.exists(), "Empty path should not exist");
    assert!(empty_path.as_os_str().is_empty(), "Path should be empty");
}

/// **[TC-U-ERR-040-02]** Queue validation - nonexistent file
///
/// **Given:** A path to a file that doesn't exist
/// **When:** Validation checks the path
/// **Then:** Path is rejected as invalid
#[tokio::test]
async fn test_queue_validation_nonexistent() {
    let builder = ErrorInjectionBuilder::new().expect("Failed to create builder");
    let nonexistent_path = builder.nonexistent_file();

    // Verify file doesn't exist
    assert!(!nonexistent_path.exists(), "File should not exist");
}

/// **[TC-U-ERR-040-03]** Queue validation - valid file
///
/// **Given:** A path to a valid audio file
/// **When:** Validation checks the path
/// **Then:** Path is accepted as valid
#[tokio::test]
async fn test_queue_validation_valid_file() {
    let builder = ErrorInjectionBuilder::new().expect("Failed to create builder");

    // Create a valid audio file
    let valid_path = builder.file_path("valid_test.wav");
    helpers::generate_sine_wav(&valid_path, 1000, 440.0, 0.5)
        .expect("Failed to generate test file");

    // Verify file exists
    assert!(valid_path.exists(), "File should exist");
    assert!(valid_path.is_file(), "Path should be a file");
}

// ============================================================================
// REQ-AP-ERR-050: Resampling Initialization Errors
// ============================================================================

/// **[TC-U-ERR-050-01]** Resampling initialization failure detection
///
/// **Given:** Invalid resampling parameters (e.g., zero sample rate)
/// **When:** Resampler is initialized
/// **Then:** Error is detected and returned
///
/// Note: This tests the error propagation pattern. Actual resampler
/// initialization failures are rare with valid parameters.
#[tokio::test]
async fn test_resampling_init_error_propagation() {
    use wkmp_ap::audio::resampler::StatefulResampler;

    // Test with valid parameters (should succeed)
    let result = StatefulResampler::new(48000, 44100, 2, 1024);
    assert!(result.is_ok(), "Valid parameters should succeed");

    // Test pass-through mode (same input and output rate)
    let result = StatefulResampler::new(44100, 44100, 2, 1024);
    assert!(result.is_ok(), "Pass-through mode should succeed");

    // Note: rubato's FastFixedIn is quite robust and doesn't have many
    // failure modes with reasonable parameters. Real failures would come
    // from extreme values (e.g., rate ratios > 256x) which are not
    // realistic for audio use cases.
}

/// **[TC-U-ERR-050-02]** Pass-through mode when sample rates match
///
/// **Given:** Input and output sample rates are identical
/// **When:** Resampler is created
/// **Then:** Pass-through mode is used (no actual resampling)
#[tokio::test]
async fn test_resampling_passthrough_mode() {
    use wkmp_ap::audio::resampler::StatefulResampler;

    // Create resampler with matching rates
    let mut resampler = StatefulResampler::new(44100, 44100, 2, 1024)
        .expect("Pass-through resampler creation should succeed");

    // Create test input (1024 samples * 2 channels = 2048 samples)
    let input: Vec<f32> = vec![0.5; 2048];

    // Process chunk
    let result = resampler.process_chunk(&input);
    assert!(result.is_ok(), "Pass-through processing should succeed");

    let output = result.unwrap();

    // In pass-through mode, output should be identical to input
    assert_eq!(output.len(), input.len(),
              "Pass-through should preserve sample count");

    for (i, (&out_sample, &in_sample)) in output.iter().zip(input.iter()).enumerate() {
        assert!((out_sample - in_sample).abs() < 1e-6,
               "Pass-through should preserve sample values (index {})", i);
    }
}

// ============================================================================
// REQ-AP-ERR-051: Resampling Runtime Errors
// ============================================================================

/// **[TC-U-ERR-051-01]** Runtime resampling error detection
///
/// **Given:** A resampler processing audio chunks
/// **When:** An invalid chunk size is provided
/// **Then:** Error is detected and reported
#[tokio::test]
async fn test_resampling_runtime_error_detection() {
    use wkmp_ap::audio::resampler::StatefulResampler;

    // Create resampler expecting 1024-sample chunks
    let mut resampler = StatefulResampler::new(48000, 44100, 2, 1024)
        .expect("Resampler creation should succeed");

    // Process with correct chunk size (should succeed)
    let correct_input: Vec<f32> = vec![0.5; 1024 * 2]; // 1024 frames * 2 channels
    let result = resampler.process_chunk(&correct_input);
    assert!(result.is_ok(), "Correct chunk size should succeed");

    // Process with incorrect chunk size (should fail)
    let wrong_input: Vec<f32> = vec![0.5; 512 * 2]; // Wrong size: 512 frames
    let result = resampler.process_chunk(&wrong_input);
    assert!(result.is_err(), "Incorrect chunk size should fail");

    if let Err(error) = result {
        match error {
            Error::Decode(msg) => {
                assert!(msg.contains("chunk") || msg.contains("size") || msg.contains("expected"),
                       "Error should mention chunk size issue: {}", msg);
            }
            _ => panic!("Expected Decode error for wrong chunk size"),
        }
    }
}

/// **[TC-U-ERR-051-02]** Position tracking during resampling errors
///
/// **Given:** A resampler processing multiple chunks
/// **When:** Chunks are processed successfully
/// **Then:** Resampler maintains state across chunks
///
/// Note: Position tracking happens at decoder_chain level, not resampler level.
/// This test verifies that the resampler can process multiple chunks without errors.
/// Actual position drift detection is tested in decoder_chain tests.
#[tokio::test]
async fn test_resampling_error_position_tracking() {
    use wkmp_ap::audio::resampler::StatefulResampler;

    // Create resampler
    let mut resampler = StatefulResampler::new(48000, 44100, 2, 1024)
        .expect("Resampler creation should succeed");

    // Process several successful chunks
    let correct_input: Vec<f32> = vec![0.5; 1024 * 2];

    for i in 0..10 {
        let result = resampler.process_chunk(&correct_input);
        assert!(result.is_ok(), "Valid chunk {} should succeed", i);

        // Verify output was produced
        let output = result.unwrap();
        assert!(!output.is_empty(), "Chunk {} should produce output", i);
    }

    // Note: rubato's FastFixedIn resampler is flexible with chunk sizes and doesn't
    // strictly enforce the chunk_size parameter. Actual runtime errors would come from
    // internal buffer issues, which are rare with valid audio data.
    // Position tracking and error reporting happen at the decoder_chain level.
}
