//! Audio decoder using symphonia
//!
//! Implements audio file decoding per SPEC016.
//!
//! # Supported Formats
//!
//! Per Cargo.toml symphonia features:
//! - MP3 (mp3)
//! - FLAC (flac)
//! - AAC (aac)
//! - MP4/M4A (isomp4)
//! - Vorbis (vorbis)
//! - Opus (via symphonia-adapter-libopus)
//!
//! # Sample Format
//!
//! Per SPEC016 DBD-FMT-010:
//! - Output: Stereo f32 samples (interleaved: [L, R, L, R, ...])
//! - Mono files: duplicated to stereo
//! - Multi-channel: downmixed to stereo
//!
//! # Architecture
//!
//! Phase 3: Basic decoder with file opening and chunk-based decoding
//! Phase 4: Add seeking, position tracking, and integration with DecoderChain

use crate::{AudioPlayerError, Result};
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::{Decoder, DecoderOptions};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::{FormatOptions, FormatReader};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use std::fs::File;
use std::path::{Path, PathBuf};

/// Audio decoder handle
///
/// Wraps symphonia for audio file decoding.
///
/// # Examples
///
/// ```ignore
/// let mut decoder = AudioDecoder::new("audio.mp3")?;
/// while let Some(chunk) = decoder.decode_chunk()? {
///     println!("Decoded {} samples at {} Hz", chunk.samples.len() / 2, chunk.sample_rate);
/// }
/// ```
pub struct AudioDecoder {
    /// Symphonia format reader
    format: Box<dyn FormatReader>,

    /// Symphonia decoder
    decoder: Box<dyn Decoder>,

    /// Track index being decoded
    track_id: u32,

    /// Native sample rate of the audio file
    native_sample_rate: u32,

    /// Number of channels in the audio file
    native_channels: usize,
}

impl AudioDecoder {
    /// Create new decoder for file
    ///
    /// # Arguments
    ///
    /// * `file_path` - Path to audio file
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let decoder = AudioDecoder::new("/path/to/audio.mp3")?;
    /// ```
    pub fn new<P: AsRef<Path>>(file_path: P) -> Result<Self> {
        // Open file
        let file = File::open(file_path.as_ref()).map_err(|_e| {
            AudioPlayerError::Decoder(crate::error::DecoderError::FileNotFound {
                path: file_path.as_ref().to_path_buf(),
            })
        })?;

        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        // Create hint from file extension
        let mut hint = Hint::new();
        if let Some(ext) = file_path.as_ref().extension() {
            hint.with_extension(ext.to_str().unwrap_or(""));
        }

        // Probe format
        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
            .map_err(|e| {
                AudioPlayerError::Decoder(crate::error::DecoderError::UnsupportedCodec {
                    codec: format!("{:?}", e),
                    file_path: file_path.as_ref().to_path_buf(),
                })
            })?;

        let format = probed.format;

        // Get default track
        let track = format.default_track().ok_or_else(|| {
            AudioPlayerError::Decoder(crate::error::DecoderError::UnsupportedCodec {
                codec: "No audio track found".to_string(),
                file_path: file_path.as_ref().to_path_buf(),
            })
        })?;

        let track_id = track.id;
        let codec_params = track.codec_params.clone();
        let native_sample_rate = codec_params.sample_rate.unwrap_or(44100);
        let native_channels = codec_params.channels.map(|c| c.count()).unwrap_or(2);

        // Create decoder
        let decoder = symphonia::default::get_codecs()
            .make(&codec_params, &DecoderOptions::default())
            .map_err(|e| {
                AudioPlayerError::Decoder(crate::error::DecoderError::UnsupportedCodec {
                    codec: format!("{:?}", e),
                    file_path: file_path.as_ref().to_path_buf(),
                })
            })?;

        Ok(Self {
            format,
            decoder,
            track_id,
            native_sample_rate,
            native_channels,
        })
    }

    /// Decode next chunk of audio
    ///
    /// Returns decoded chunk or None if end of file reached.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// while let Some(chunk) = decoder.decode_chunk()? {
    ///     // Process chunk...
    /// }
    /// ```
    pub fn decode_chunk(&mut self) -> Result<Option<DecodedChunk>> {
        // Get next packet
        let packet = match self.format.next_packet() {
            Ok(packet) => packet,
            Err(SymphoniaError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                return Ok(None); // EOF
            }
            Err(e) => {
                return Err(AudioPlayerError::Decoder(
                    crate::error::DecoderError::DecoderPanic {
                        message: format!("{:?}", e),
                        file_path: PathBuf::from("unknown"),  // TODO: Store file path in decoder
                    },
                ));
            }
        };

        // Skip packets from other tracks
        if packet.track_id() != self.track_id {
            return self.decode_chunk(); // Recursively try next packet
        }

        // Decode packet
        let decoded = self.decoder.decode(&packet).map_err(|e| {
            AudioPlayerError::Decoder(crate::error::DecoderError::DecoderPanic {
                message: format!("{:?}", e),
                file_path: PathBuf::from("unknown"),  // TODO: Store file path in decoder
            })
        })?;

        // Convert to stereo f32
        let samples = Self::convert_to_stereo_f32_static(&decoded)?;

        Ok(Some(DecodedChunk {
            samples,
            sample_rate: self.native_sample_rate,
        }))
    }

    /// Get native sample rate
    pub fn sample_rate(&self) -> u32 {
        self.native_sample_rate
    }

    /// Get native channel count
    pub fn channels(&self) -> usize {
        self.native_channels
    }

    /// Convert audio buffer to stereo f32 format
    fn convert_to_stereo_f32_static(buffer: &AudioBufferRef) -> Result<Vec<f32>> {
        match buffer {
            AudioBufferRef::F32(buf) => {
                let channels = buf.spec().channels.count();
                let frames = buf.frames();

                match channels {
                    1 => {
                        // Mono: duplicate to stereo
                        let mono = buf.chan(0);
                        let mut stereo = Vec::with_capacity(frames * 2);
                        for &sample in mono {
                            stereo.push(sample);
                            stereo.push(sample);
                        }
                        Ok(stereo)
                    }
                    2 => {
                        // Stereo: interleave
                        let left = buf.chan(0);
                        let right = buf.chan(1);
                        let mut stereo = Vec::with_capacity(frames * 2);
                        for i in 0..frames {
                            stereo.push(left[i]);
                            stereo.push(right[i]);
                        }
                        Ok(stereo)
                    }
                    _ => {
                        // Multi-channel: simple downmix to stereo (average channels)
                        let mut stereo = Vec::with_capacity(frames * 2);
                        for frame_idx in 0..frames {
                            let mut left_sum = 0.0f32;
                            let mut right_sum = 0.0f32;

                            // Average all channels (simple downmix)
                            for ch_idx in 0..channels {
                                let sample = buf.chan(ch_idx)[frame_idx];
                                if ch_idx % 2 == 0 {
                                    left_sum += sample;
                                } else {
                                    right_sum += sample;
                                }
                            }

                            stereo.push(left_sum / (channels as f32 / 2.0));
                            stereo.push(right_sum / (channels as f32 / 2.0));
                        }
                        Ok(stereo)
                    }
                }
            }
            _ => {
                // Convert other sample formats to f32
                // For now, just return empty (will implement if needed)
                Err(AudioPlayerError::Decoder(
                    crate::error::DecoderError::UnsupportedCodec {
                        codec: "Non-f32 sample format not yet supported".to_string(),
                        file_path: PathBuf::from("unknown"),
                    },
                ))
            }
        }
    }
}

/// Decoded audio chunk
///
/// Contains decoded stereo f32 samples and sample rate information.
#[derive(Debug)]
pub struct DecodedChunk {
    /// Interleaved stereo f32 samples [L, R, L, R, ...]
    pub samples: Vec<f32>,

    /// Sample rate of decoded audio
    pub sample_rate: u32,
}

// ========================================
// Tests
// ========================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decoder_nonexistent_file() {
        let result = AudioDecoder::new("/nonexistent/file.mp3");
        assert!(result.is_err());
    }

    // Note: Additional tests would require test audio files
    // See Phase 4 for comprehensive integration tests with actual audio files
}
