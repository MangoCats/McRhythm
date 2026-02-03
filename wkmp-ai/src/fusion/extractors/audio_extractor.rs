// Audio Passage Extractor - Helper for extractors that need raw audio data
//
// PLAN023: Extract passage audio to temporary PCM WAV file using symphonia

use anyhow::{Context, Result};
use std::fs::File;
use std::path::Path;
use symphonia::core::audio::{AudioBufferRef, Signal};
use tempfile::NamedTempFile;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::units::Time;
use tracing::{debug, warn};

/// Extract passage audio to temporary WAV file
///
/// # Arguments
/// * `file_path` - Source audio file
/// * `start_seconds` - Passage start time
/// * `end_seconds` - Passage end time
///
/// # Returns
/// * `NamedTempFile` handle to temporary WAV file (16-bit PCM, sample rate matches source)
///   Caller owns the file lifecycle - file will be deleted when handle is dropped
pub async fn extract_passage_audio(
    file_path: &Path,
    start_seconds: f64,
    end_seconds: f64,
) -> Result<NamedTempFile> {
    debug!(
        "Extracting passage audio: {} ({:.2}s - {:.2}s)",
        file_path.display(),
        start_seconds,
        end_seconds
    );

    // Create temporary file for extracted audio
    let temp_file = tempfile::Builder::new()
        .prefix("wkmp_passage_")
        .suffix(".wav")
        .tempfile()
        .context("Failed to create temporary file")?;
    let temp_path = temp_file.path();

    // Open the source audio file
    let file = File::open(file_path).context("Failed to open audio file")?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    // Create a hint to help the format registry
    let mut hint = Hint::new();
    if let Some(extension) = file_path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(extension);
    }

    // Probe the media source
    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .context("Failed to probe audio file")?;

    let mut format = probed.format;

    // Find the default audio track
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .context("No audio track found")?;

    let track_id = track.id;
    let sample_rate = track.codec_params.sample_rate.context("Sample rate unknown")?;
    let channels = track.codec_params.channels.context("Channels unknown")?;

    debug!(
        "Source audio: {}Hz, {} channels",
        sample_rate,
        channels.count()
    );

    // Create decoder
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .context("Failed to create decoder")?;

    // Calculate start/end sample positions
    let start_samples = (start_seconds * sample_rate as f64) as u64;
    let end_samples = (end_seconds * sample_rate as f64) as u64;
    let duration_samples = end_samples - start_samples;

    // Seek to start position (if supported)
    if start_seconds > 0.0 {
        let seek_time = Time::from(start_seconds);
        match format.seek(symphonia::core::formats::SeekMode::Accurate, symphonia::core::formats::SeekTo::Time { time: seek_time, track_id: Some(track_id) }) {
            Ok(_) => debug!("Seeked to {:.2}s", start_seconds),
            Err(e) => {
                warn!("Seek not supported or failed: {}. Will decode from start.", e);
                // Fall back to decoding from start and discarding samples
            }
        }
    }

    // Prepare WAV file writer
    let spec = hound::WavSpec {
        channels: channels.count() as u16,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::create(temp_path, spec)
        .context("Failed to create WAV writer")?;

    let mut samples_written = 0u64;
    let mut current_sample = 0u64;

    // Decode and write samples
    loop {
        // Get the next packet
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(Error::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                debug!("End of file reached");
                break;
            }
            Err(e) => return Err(e).context("Error reading packet"),
        };

        // Decode the packet
        let decoded = match decoder.decode(&packet) {
            Ok(decoded) => decoded,
            Err(Error::DecodeError(e)) => {
                warn!("Decode error: {}", e);
                continue;
            }
            Err(e) => return Err(e).context("Fatal decode error"),
        };

        // Extract samples as i16
        let samples = extract_samples_i16(&decoded)?;

        // Write samples that fall within the target range
        for sample in samples {
            if current_sample >= start_samples && samples_written < duration_samples {
                writer
                    .write_sample(sample)
                    .context("Failed to write sample")?;
                samples_written += 1;
            }
            current_sample += 1;

            // Stop if we've written enough samples
            if samples_written >= duration_samples {
                break;
            }
        }

        if samples_written >= duration_samples {
            break;
        }
    }

    writer.finalize().context("Failed to finalize WAV file")?;

    debug!(
        "Extracted {} samples ({:.2}s) to {}",
        samples_written,
        samples_written as f64 / sample_rate as f64,
        temp_path.display()
    );

    // Return temp file handle - caller controls lifecycle
    Ok(temp_file)
}

/// Extract samples from AudioBufferRef as i16
fn extract_samples_i16(buffer: &AudioBufferRef) -> Result<Vec<i16>> {
    match buffer {
        AudioBufferRef::F32(buf) => {
            // Convert f32 samples to i16
            Ok(buf
                .chan(0)
                .iter()
                .map(|&s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16)
                .collect())
        }
        AudioBufferRef::F64(buf) => {
            // Convert f64 samples to i16
            Ok(buf
                .chan(0)
                .iter()
                .map(|&s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16)
                .collect())
        }
        AudioBufferRef::S16(buf) => {
            // Already i16
            Ok(buf.chan(0).to_vec())
        }
        AudioBufferRef::S32(buf) => {
            // Convert i32 samples to i16
            Ok(buf
                .chan(0)
                .iter()
                .map(|&s| (s >> 16) as i16)
                .collect())
        }
        _ => anyhow::bail!("Unsupported audio buffer format"),
    }
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
    async fn test_extract_passage_audio_basic() {
        // Test extracting full audio segment
        let audio_file = test_fixture_path("sine_440hz_5s.wav");

        if !audio_file.exists() {
            eprintln!("Skipping test: fixture not found at {:?}", audio_file);
            eprintln!("Run: cd tests/fixtures && python3 generate_test_audio.py");
            return;
        }

        let result = extract_passage_audio(&audio_file, 0.0, 5.0).await;
        assert!(result.is_ok(), "Audio extraction failed: {:?}", result.err());

        let temp_wav = result.unwrap();
        assert!(temp_wav.path().exists(), "Temporary WAV file should exist");

        // Verify file has content (WAV header + audio data)
        let metadata = std::fs::metadata(temp_wav.path()).expect("Failed to read temp file metadata");
        assert!(metadata.len() > 44, "WAV file should be larger than header (44 bytes)");

        // For 5 seconds at 44100 Hz mono 16-bit: ~441KB (sine_440hz_5s.wav is mono)
        assert!(metadata.len() > 400_000, "WAV file should have reasonable size for 5 seconds");
    }

    #[tokio::test]
    async fn test_extract_passage_audio_with_offset() {
        // Test extracting a segment from the middle
        let audio_file = test_fixture_path("chirp_2s.wav");

        if !audio_file.exists() {
            eprintln!("Skipping test: fixture not found");
            return;
        }

        // Extract middle 1 second (0.5s to 1.5s)
        let result = extract_passage_audio(&audio_file, 0.5, 1.5).await;
        assert!(result.is_ok(), "Audio extraction with offset failed: {:?}", result.err());

        let temp_wav = result.unwrap();
        assert!(temp_wav.path().exists(), "Temporary WAV file should exist");

        // Verify file has reasonable size for 1 second
        let metadata = std::fs::metadata(temp_wav.path()).expect("Failed to read temp file metadata");
        assert!(metadata.len() > 44, "WAV file should be larger than header");

        // For 1 second at 44100 Hz mono 16-bit: ~88KB (chirp_2s.wav is mono)
        assert!(metadata.len() > 80_000, "WAV file should have reasonable size for 1 second");
    }
}
