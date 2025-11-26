//! # Audio Decoding Utilities
//!
//! Symphonia-based MP3 decoding and sample extraction.
//!
//! ## Features
//! - **Universal audio decoding**: Supports all formats via Symphonia (MP3, FLAC, M4A, OGG, etc.)
//! - **Automatic format detection**: No need to specify codec
//! - **Mono channel extraction**: Extracts first channel for analysis
//! - **Normalized samples**: Returns f32 samples in range -1.0 to 1.0
//!
//! ## Related Modules
//! - `silence_detection`: Primary consumer of decoded samples

use std::path::Path;
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// Decode audio file to mono f32 samples
///
/// Uses Symphonia to decode any supported audio format (MP3, FLAC, M4A, OGG, WAV, etc.)
/// and extracts the first channel as normalized f32 samples.
///
/// # Arguments
/// * `path` - Path to audio file
///
/// # Returns
/// Tuple of (samples, sample_rate) where:
/// - `samples`: Vec<f32> of normalized audio samples (-1.0 to 1.0)
/// - `sample_rate`: Sample rate in Hz (e.g., 44100)
///
/// # Errors
/// Returns error if:
/// - File cannot be opened
/// - Format cannot be detected
/// - No default audio track found
/// - Decoding fails
///
/// # Example
/// ```ignore
/// let (samples, sample_rate) = decode_mp3(Path::new("album.mp3"))?;
/// println!("Decoded {} samples at {} Hz", samples.len(), sample_rate);
/// ```
pub(crate) fn decode_mp3(
    path: &Path,
) -> Result<(Vec<f32>, u32), Box<dyn std::error::Error + Send + Sync>> {
    let file = std::fs::File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let hint = Hint::new();
    let format_opts = FormatOptions::default();
    let metadata_opts = MetadataOptions::default();

    let probed = symphonia::default::get_probe().format(&hint, mss, &format_opts, &metadata_opts)?;
    let mut format = probed.format;

    let track = format.default_track().ok_or("No default track")?;
    let track_id = track.id;

    let sample_rate = track
        .codec_params
        .sample_rate
        .ok_or("No sample rate")?;

    let decoder_opts = DecoderOptions::default();
    let mut decoder =
        symphonia::default::get_codecs().make(&track.codec_params, &decoder_opts)?;

    let mut samples = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(_) => break,
        };

        if packet.track_id() != track_id {
            continue;
        }

        match decoder.decode(&packet) {
            Ok(decoded) => {
                // Extract first channel and convert to f32
                match decoded {
                    AudioBufferRef::F32(buf) => {
                        for &sample in buf.chan(0) {
                            samples.push(sample);
                        }
                    }
                    AudioBufferRef::U8(buf) => {
                        for &sample in buf.chan(0) {
                            samples.push((sample as f32 - 128.0) / 128.0);
                        }
                    }
                    AudioBufferRef::U16(buf) => {
                        for &sample in buf.chan(0) {
                            samples.push((sample as f32 - 32768.0) / 32768.0);
                        }
                    }
                    AudioBufferRef::U24(buf) => {
                        for &sample in buf.chan(0) {
                            samples.push((sample.inner() as f32 - 8388608.0) / 8388608.0);
                        }
                    }
                    AudioBufferRef::U32(buf) => {
                        for &sample in buf.chan(0) {
                            samples.push((sample as f32 - 2147483648.0) / 2147483648.0);
                        }
                    }
                    AudioBufferRef::S8(buf) => {
                        for &sample in buf.chan(0) {
                            samples.push(sample as f32 / 128.0);
                        }
                    }
                    AudioBufferRef::S16(buf) => {
                        for &sample in buf.chan(0) {
                            samples.push(sample as f32 / 32768.0);
                        }
                    }
                    AudioBufferRef::S24(buf) => {
                        for &sample in buf.chan(0) {
                            samples.push(sample.inner() as f32 / 8388608.0);
                        }
                    }
                    AudioBufferRef::S32(buf) => {
                        for &sample in buf.chan(0) {
                            samples.push(sample as f32 / 2147483648.0);
                        }
                    }
                    AudioBufferRef::F64(buf) => {
                        for &sample in buf.chan(0) {
                            samples.push(sample as f32);
                        }
                    }
                }
            }
            Err(_e) => {
                // Skip decode errors (e.g., partial packets at end of file)
                continue;
            }
        }
    }

    Ok((samples, sample_rate))
}
