// Chromaprint Fingerprint Analyzer
//
// PLAN023: REQ-AI-031 - Generate acoustic fingerprint using chromaprint
// Confidence: N/A (fingerprint generation only, used by AcoustID)

use super::extract_passage_audio;
use anyhow::{Context, Result};
use std::path::Path;
use tracing::debug;

/// Generate Chromaprint fingerprint for audio passage
///
/// # Arguments
/// * `file_path` - Path to audio file
/// * `start_seconds` - Passage start time
/// * `end_seconds` - Passage end time
///
/// # Returns
/// * Base64-encoded Chromaprint fingerprint
pub async fn generate_fingerprint(
    file_path: &Path,
    start_seconds: f64,
    end_seconds: f64,
) -> Result<String> {
    debug!(
        "Generating Chromaprint fingerprint: {} ({:.2}s - {:.2}s)",
        file_path.display(),
        start_seconds,
        end_seconds
    );

    // Extract passage audio to temporary WAV file
    let temp_audio = extract_passage_audio(file_path, start_seconds, end_seconds)
        .await
        .context("Failed to extract passage audio")?;

    // Read WAV file samples (temp_audio handle keeps file alive)
    let mut reader = hound::WavReader::open(temp_audio.path())
        .context("Failed to open extracted audio")?;

    let spec = reader.spec();
    let sample_rate = spec.sample_rate;
    let channels = spec.channels;

    debug!(
        "WAV spec: {}Hz, {} channels, {} bits",
        sample_rate, channels, spec.bits_per_sample
    );

    // Collect samples as i16
    let samples: Vec<i16> = reader
        .samples::<i16>()
        .collect::<Result<Vec<_>, _>>()
        .context("Failed to read audio samples")?;

    debug!("Read {} samples", samples.len());

    // Generate fingerprint using chromaprint FFI
    let fingerprint = unsafe {
        use chromaprint_sys_next::*;

        // Create chromaprint context
        // Algorithm 2 is the default (TEST2/production algorithm)
        let ctx = chromaprint_new(2);
        if ctx.is_null() {
            anyhow::bail!("Failed to create Chromaprint context");
        }

        // Start fingerprinting
        let start_result = chromaprint_start(ctx, sample_rate as i32, channels as i32);
        if start_result != 1 {
            chromaprint_free(ctx);
            anyhow::bail!("Failed to start Chromaprint");
        }

        // Feed audio data
        let feed_result = chromaprint_feed(ctx, samples.as_ptr(), samples.len() as i32);
        if feed_result != 1 {
            chromaprint_free(ctx);
            anyhow::bail!("Failed to feed audio to Chromaprint");
        }

        // Finish and get fingerprint
        let finish_result = chromaprint_finish(ctx);
        if finish_result != 1 {
            chromaprint_free(ctx);
            anyhow::bail!("Failed to finish Chromaprint");
        }

        // Get encoded fingerprint
        let mut fingerprint_ptr: *mut i8 = std::ptr::null_mut();
        let get_result = chromaprint_get_fingerprint(ctx, &mut fingerprint_ptr as *mut *mut i8);
        if get_result != 1 || fingerprint_ptr.is_null() {
            chromaprint_free(ctx);
            anyhow::bail!("Failed to get Chromaprint fingerprint");
        }

        // Convert C string to Rust String
        let c_str = std::ffi::CStr::from_ptr(fingerprint_ptr);
        let fingerprint = c_str.to_string_lossy().to_string();

        // Free resources
        chromaprint_dealloc(fingerprint_ptr as *mut std::ffi::c_void);
        chromaprint_free(ctx);

        fingerprint
    };

    debug!("Generated fingerprint: {} characters", fingerprint.len());

    // temp_audio handle will auto-delete file when dropped
    Ok(fingerprint)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn test_fixture_path(filename: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join(filename)
    }

    #[tokio::test]
    async fn test_fingerprint_basic() {
        // Test with 440 Hz sine wave (5 seconds)
        let audio_file = test_fixture_path("sine_440hz_5s.wav");

        if !audio_file.exists() {
            eprintln!("Skipping test: fixture not found at {:?}", audio_file);
            eprintln!("Run: cd tests/fixtures && python3 generate_test_audio.py");
            return;
        }

        let result = generate_fingerprint(&audio_file, 0.0, 5.0).await;
        assert!(result.is_ok(), "Fingerprinting failed: {:?}", result.err());

        let fingerprint = result.unwrap();
        // Simple sine wave produces minimal fingerprint (low spectral complexity)
        // Just verify we got something back
        assert!(!fingerprint.is_empty(), "Fingerprint should not be empty");
    }

    #[tokio::test]
    async fn test_fingerprint_with_offset() {
        // Test with chirp (2 seconds), extract middle 1 second
        let audio_file = test_fixture_path("chirp_2s.wav");

        if !audio_file.exists() {
            eprintln!("Skipping test: fixture not found");
            return;
        }

        let result = generate_fingerprint(&audio_file, 0.5, 1.5).await;
        assert!(result.is_ok(), "Fingerprinting with offset failed: {:?}", result.err());

        let fingerprint = result.unwrap();
        assert!(!fingerprint.is_empty(), "Fingerprint should not be empty");
    }

    #[tokio::test]
    async fn test_fingerprint_deterministic() {
        // Verify same audio produces same fingerprint (deterministic)
        let audio_file = test_fixture_path("chirp_2s.wav");

        if !audio_file.exists() {
            eprintln!("Skipping test: fixture not found");
            return;
        }

        let fp1 = generate_fingerprint(&audio_file, 0.0, 2.0).await.unwrap();
        let fp2 = generate_fingerprint(&audio_file, 0.0, 2.0).await.unwrap();

        assert_eq!(fp1, fp2, "Same audio should produce identical fingerprints");
    }
}

