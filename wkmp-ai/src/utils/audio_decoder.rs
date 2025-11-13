//! Audio Decoding Utilities
//!
//! **Purpose:** Decode audio files to mono f32 PCM samples for PLAN024 pipeline
//!
//! Uses symphonia for format-agnostic decoding (MP3, FLAC, AAC, etc.)

use anyhow::{Context, Result};
use std::path::Path;
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::conv::FromSample;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::sample::Sample;

/// Decoded audio result
#[derive(Debug)]
pub struct DecodedAudio {
    /// Mono audio samples (f32, range [-1.0, 1.0])
    pub samples: Vec<f32>,
    /// Sample rate in Hz
    pub sample_rate: u32,
    /// Original channel count
    pub channels: usize,
    /// Duration in seconds
    pub duration_seconds: f64,
}

/// Decode audio file to mono f32 PCM samples
///
/// **Algorithm:**
/// 1. Open file and probe format using symphonia
/// 2. Find default audio track
/// 3. Create decoder for track codec
/// 4. Decode all packets to PCM samples
/// 5. Convert multi-channel to mono (average channels)
/// 6. Return mono f32 samples + sample rate
///
/// **Supported Formats:** MP3, FLAC, AAC, WAV, OGG, etc. (via symphonia)
///
/// # Arguments
/// * `file_path` - Path to audio file
///
/// # Returns
/// * `DecodedAudio` - Mono samples, sample rate, metadata
///
/// # Errors
/// * File I/O errors
/// * Unsupported format
/// * Corrupt audio data
pub fn decode_audio_file(file_path: &Path) -> Result<DecodedAudio> {
    tracing::debug!(path = %file_path.display(), "Decoding audio file");

    // Open the audio file
    let file = std::fs::File::open(file_path)
        .with_context(|| format!("Failed to open audio file: {}", file_path.display()))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    // Create format hint from file extension
    let mut hint = Hint::new();
    if let Some(extension) = file_path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(extension);
    }

    // Probe the media source
    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .with_context(|| format!("Failed to probe audio file: {}", file_path.display()))?;

    let mut format = probed.format;

    // Find the default audio track
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .context("No audio track found in file")?;

    let track_id = track.id;
    let sample_rate = track
        .codec_params
        .sample_rate
        .context("Sample rate unknown")?;
    let channels = track.codec_params.channels.context("Channels unknown")?;
    let channel_count = channels.count();

    tracing::debug!(
        path = %file_path.display(),
        sample_rate = sample_rate,
        channels = channel_count,
        "Audio file info"
    );

    // Create decoder
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .with_context(|| format!("Failed to create decoder for: {}", file_path.display()))?;

    // Decode all packets
    let mut all_samples: Vec<f32> = Vec::new();

    loop {
        // Get next packet
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(symphonia::core::errors::Error::IoError(e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                // End of stream
                break;
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Error reading packet: {}", e));
            }
        };

        // Skip packets from other tracks
        if packet.track_id() != track_id {
            continue;
        }

        // Decode packet
        let decoded = decoder
            .decode(&packet)
            .with_context(|| format!("Failed to decode packet in: {}", file_path.display()))?;

        // Convert to f32 samples and mix to mono
        let mono_samples = convert_to_mono_f32(&decoded);
        all_samples.extend_from_slice(&mono_samples);
    }

    let duration_seconds = all_samples.len() as f64 / sample_rate as f64;

    tracing::debug!(
        path = %file_path.display(),
        total_samples = all_samples.len(),
        duration_seconds = format!("{:.2}", duration_seconds),
        "Audio decoding complete"
    );

    Ok(DecodedAudio {
        samples: all_samples,
        sample_rate,
        channels: channel_count,
        duration_seconds,
    })
}

/// Convert audio buffer to mono f32 samples
///
/// **Algorithm:**
/// - If mono: Direct conversion to f32
/// - If stereo/multi-channel: Average all channels to mono
///
/// # Arguments
/// * `decoded` - AudioBufferRef from symphonia decoder
///
/// # Returns
/// * Vec<f32> - Mono audio samples [-1.0, 1.0]
fn convert_to_mono_f32(decoded: &AudioBufferRef) -> Vec<f32> {
    // Helper trait to convert samples to f32
    fn to_f32_sample<S: Sample>(sample: S) -> f32
    where
        f32: FromSample<S>,
    {
        f32::from_sample(sample)
    }

    match decoded {
        AudioBufferRef::F32(buf) => {
            // Already f32
            let num_channels = buf.spec().channels.count();
            let num_frames = buf.frames();
            let mut mono = Vec::with_capacity(num_frames);

            for frame_idx in 0..num_frames {
                let mut sum = 0.0f32;
                for ch in 0..num_channels {
                    sum += buf.chan(ch)[frame_idx];
                }
                mono.push(sum / num_channels as f32);
            }

            mono
        }
        AudioBufferRef::U8(buf) => {
            let num_channels = buf.spec().channels.count();
            let num_frames = buf.frames();
            let mut mono = Vec::with_capacity(num_frames);

            for frame_idx in 0..num_frames {
                let mut sum = 0.0f32;
                for ch in 0..num_channels {
                    sum += to_f32_sample(buf.chan(ch)[frame_idx]);
                }
                mono.push(sum / num_channels as f32);
            }

            mono
        }
        AudioBufferRef::U16(buf) => {
            let num_channels = buf.spec().channels.count();
            let num_frames = buf.frames();
            let mut mono = Vec::with_capacity(num_frames);

            for frame_idx in 0..num_frames {
                let mut sum = 0.0f32;
                for ch in 0..num_channels {
                    sum += to_f32_sample(buf.chan(ch)[frame_idx]);
                }
                mono.push(sum / num_channels as f32);
            }

            mono
        }
        AudioBufferRef::U24(buf) => {
            let num_channels = buf.spec().channels.count();
            let num_frames = buf.frames();
            let mut mono = Vec::with_capacity(num_frames);

            for frame_idx in 0..num_frames {
                let mut sum = 0.0f32;
                for ch in 0..num_channels {
                    sum += to_f32_sample(buf.chan(ch)[frame_idx]);
                }
                mono.push(sum / num_channels as f32);
            }

            mono
        }
        AudioBufferRef::U32(buf) => {
            let num_channels = buf.spec().channels.count();
            let num_frames = buf.frames();
            let mut mono = Vec::with_capacity(num_frames);

            for frame_idx in 0..num_frames {
                let mut sum = 0.0f32;
                for ch in 0..num_channels {
                    sum += to_f32_sample(buf.chan(ch)[frame_idx]);
                }
                mono.push(sum / num_channels as f32);
            }

            mono
        }
        AudioBufferRef::S8(buf) => {
            let num_channels = buf.spec().channels.count();
            let num_frames = buf.frames();
            let mut mono = Vec::with_capacity(num_frames);

            for frame_idx in 0..num_frames {
                let mut sum = 0.0f32;
                for ch in 0..num_channels {
                    sum += to_f32_sample(buf.chan(ch)[frame_idx]);
                }
                mono.push(sum / num_channels as f32);
            }

            mono
        }
        AudioBufferRef::S16(buf) => {
            let num_channels = buf.spec().channels.count();
            let num_frames = buf.frames();
            let mut mono = Vec::with_capacity(num_frames);

            for frame_idx in 0..num_frames {
                let mut sum = 0.0f32;
                for ch in 0..num_channels {
                    sum += to_f32_sample(buf.chan(ch)[frame_idx]);
                }
                mono.push(sum / num_channels as f32);
            }

            mono
        }
        AudioBufferRef::S24(buf) => {
            let num_channels = buf.spec().channels.count();
            let num_frames = buf.frames();
            let mut mono = Vec::with_capacity(num_frames);

            for frame_idx in 0..num_frames {
                let mut sum = 0.0f32;
                for ch in 0..num_channels {
                    sum += to_f32_sample(buf.chan(ch)[frame_idx]);
                }
                mono.push(sum / num_channels as f32);
            }

            mono
        }
        AudioBufferRef::S32(buf) => {
            let num_channels = buf.spec().channels.count();
            let num_frames = buf.frames();
            let mut mono = Vec::with_capacity(num_frames);

            for frame_idx in 0..num_frames {
                let mut sum = 0.0f32;
                for ch in 0..num_channels {
                    sum += to_f32_sample(buf.chan(ch)[frame_idx]);
                }
                mono.push(sum / num_channels as f32);
            }

            mono
        }
        AudioBufferRef::F64(buf) => {
            let num_channels = buf.spec().channels.count();
            let num_frames = buf.frames();
            let mut mono = Vec::with_capacity(num_frames);

            for frame_idx in 0..num_frames {
                let mut sum = 0.0f32;
                for ch in 0..num_channels {
                    sum += to_f32_sample(buf.chan(ch)[frame_idx]);
                }
                mono.push(sum / num_channels as f32);
            }

            mono
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_audio_file_not_found() {
        let result = decode_audio_file(Path::new("/nonexistent/file.mp3"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Failed to open audio file"));
    }

    // NOTE: Real audio file tests would require test fixtures
    // These should be added as integration tests with known test audio files
}
