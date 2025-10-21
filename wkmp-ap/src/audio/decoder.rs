//! Audio decoder using symphonia
//!
//! Decodes various audio formats (MP3, FLAC, AAC, Vorbis, Opus) to PCM samples.
//!
//! **Traceability:**
//! - [SSD-DEC-010] Decode-from-start-and-skip approach
//! - [SSD-DEC-013] Always decode from beginning (never use compressed seek)
//! - [SSD-FBUF-021] Decode-and-skip for accurate timing
//! - [REQ-TECH-022A] Opus codec via C library FFI (symphonia-adapter-libopus)

use crate::error::{Error, Result};
use std::path::PathBuf;
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use tracing::{debug, warn};

/// Result from decoding a passage
///
/// **[DBD-DEC-090]** Endpoint discovery support for undefined endpoints
#[derive(Debug, Clone)]
pub struct DecodeResult {
    /// Decoded PCM samples (interleaved)
    pub samples: Vec<f32>,

    /// Original sample rate (before resampling)
    pub sample_rate: u32,

    /// Number of channels in source (1=mono, 2=stereo, etc.)
    pub channels: u16,

    /// Actual endpoint discovered when decoding to EOF
    /// **[DBD-DEC-095]** Set when passage has NULL end_time_ticks
    /// Contains actual file duration in ticks
    pub actual_end_ticks: Option<i64>,
}

// Import Opus adapter to register codec with symphonia
// [REQ-TECH-022A]: Opus support via libopus C library FFI
use symphonia::core::codecs::CodecRegistry;
use symphonia_adapter_libopus::OpusDecoder;
use std::sync::OnceLock;

/// Get codec registry with Opus support
/// [REQ-TECH-022A]: Registers OpusDecoder with symphonia codec registry
fn get_codec_registry() -> &'static CodecRegistry {
    static CODEC_REGISTRY: OnceLock<CodecRegistry> = OnceLock::new();
    CODEC_REGISTRY.get_or_init(|| {
        let mut registry = CodecRegistry::new();
        // Register Opus decoder first
        registry.register_all::<OpusDecoder>();
        // Register default codecs (MP3, FLAC, Vorbis, etc.)
        registry.register_all::<symphonia::default::codecs::MpaDecoder>();
        registry.register_all::<symphonia::default::codecs::PcmDecoder>();
        registry.register_all::<symphonia::default::codecs::VorbisDecoder>();
        registry.register_all::<symphonia::default::codecs::FlacDecoder>();
        registry.register_all::<symphonia::default::codecs::AdpcmDecoder>();
        registry.register_all::<symphonia::default::codecs::AacDecoder>();
        registry
    })
}

/// Simple audio decoder using symphonia.
///
/// **[SSD-DEC-010]** Uses decode-and-skip approach for reliable, sample-accurate positioning.
pub struct SimpleDecoder;

impl SimpleDecoder {
    /// Decode entire audio file to PCM samples.
    ///
    /// **[SSD-DEC-011]** Decodes from file start, returns all samples.
    ///
    /// # Returns
    /// `DecodeResult` containing:
    /// - `samples`: Interleaved stereo f32 samples (converted from source format)
    /// - `sample_rate`: Original sample rate (before resampling)
    /// - `channels`: Number of channels in source (1=mono, 2=stereo, etc.)
    /// - `actual_end_ticks`: None (not applicable for full file decode)
    ///
    /// # Errors
    /// - Failed to open file
    /// - Unsupported audio format
    /// - Decode error
    pub fn decode_file(path: &PathBuf) -> Result<DecodeResult> {
        debug!("Decoding entire file: {}", path.display());

        // Open the file
        let file = std::fs::File::open(path)
            .map_err(|e| Error::Decode(format!("Failed to open file {}: {}", path.display(), e)))?;

        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        // Create a hint to help the format registry guess the format
        let mut hint = Hint::new();
        if let Some(extension) = path.extension() {
            if let Some(ext_str) = extension.to_str() {
                hint.with_extension(ext_str);
            }
        }

        // Probe the file to get the format reader
        let format_opts = FormatOptions::default();
        let metadata_opts = MetadataOptions::default();

        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &format_opts, &metadata_opts)
            .map_err(|e| Error::Decode(format!("Failed to probe format: {}", e)))?;

        let mut format = probed.format;

        // Get the default audio track
        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or_else(|| Error::Decode("No audio track found".to_string()))?;

        let track_id = track.id;
        let codec_params = &track.codec_params;

        // Get sample rate and channels
        let sample_rate = codec_params
            .sample_rate
            .ok_or_else(|| Error::Decode("Sample rate not found".to_string()))?;

        let channels = codec_params
            .channels
            .map(|c| c.count() as u16)
            .ok_or_else(|| Error::Decode("Channel count not found".to_string()))?;

        debug!(
            "Audio format: sample_rate={}, channels={}",
            sample_rate, channels
        );

        // Create decoder
        // [REQ-TECH-022A]: Use custom codec registry with Opus support
        let decoder_opts = DecoderOptions::default();
        let mut decoder = get_codec_registry()
            .make(&codec_params, &decoder_opts)
            .map_err(|e| Error::Decode(format!("Failed to create decoder: {}", e)))?;

        // Decode all packets
        let mut samples = Vec::new();

        loop {
            // Read next packet
            let packet = match format.next_packet() {
                Ok(packet) => packet,
                Err(symphonia::core::errors::Error::IoError(ref e))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    // End of stream
                    debug!("Reached end of file");
                    break;
                }
                Err(e) => {
                    warn!("Error reading packet: {}", e);
                    break;
                }
            };

            // Skip packets for other tracks
            if packet.track_id() != track_id {
                continue;
            }

            // Decode packet
            match decoder.decode(&packet) {
                Ok(decoded) => {
                    // Convert decoded audio to f32 samples
                    Self::convert_samples_to_f32(&decoded, &mut samples);
                }
                Err(e) => {
                    warn!("Decode error: {}", e);
                    continue;
                }
            }
        }

        debug!(
            "Decoded {} samples ({} frames)",
            samples.len(),
            samples.len() / channels as usize
        );

        Ok(DecodeResult {
            samples,
            sample_rate,
            channels,
            actual_end_ticks: None, // Not applicable for full file decode
        })
    }

    /// Decode passage with start/end time trimming.
    ///
    /// **[SSD-DEC-012]** Decode-and-skip: decode from start, discard before start_time,
    /// stop at end_time.
    /// **[DBD-DEC-090]** Endpoint discovery: When end_ms=0 (undefined endpoint), returns
    /// actual_end_ticks calculated from decoded sample count.
    ///
    /// # Arguments
    /// - `path`: Path to audio file
    /// - `start_ms`: Passage start time in milliseconds (0 = file start)
    /// - `end_ms`: Passage end time in milliseconds (0 = file end, undefined endpoint)
    ///
    /// # Returns
    /// `DecodeResult` containing:
    /// - `samples`: Trimmed interleaved stereo f32 samples
    /// - `sample_rate`: Original sample rate (before resampling)
    /// - `channels`: Number of channels in source
    /// - `actual_end_ticks`: Discovered endpoint when end_ms=0, None otherwise
    ///
    /// **[DBD-DEC-095]** Calculates actual_end_ticks = start_ticks + samples_to_ticks(sample_count)
    pub fn decode_passage(
        path: &PathBuf,
        start_ms: u64,
        end_ms: u64,
    ) -> Result<DecodeResult> {
        debug!(
            "Decoding passage: {} ({}ms - {}ms)",
            path.display(),
            start_ms,
            end_ms
        );

        // **PERFORMANCE FIX:** Decode only until passage end, not entire file
        // This reduces decode time from O(file_length) to O(passage_length)

        // Open the file
        let file = std::fs::File::open(path)
            .map_err(|e| Error::Decode(format!("Failed to open file {}: {}", path.display(), e)))?;

        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        // Create format hint
        let mut hint = Hint::new();
        if let Some(extension) = path.extension() {
            if let Some(ext_str) = extension.to_str() {
                hint.with_extension(ext_str);
            }
        }

        // Probe the file
        let format_opts = FormatOptions::default();
        let metadata_opts = MetadataOptions::default();

        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &format_opts, &metadata_opts)
            .map_err(|e| Error::Decode(format!("Failed to probe format: {}", e)))?;

        let mut format = probed.format;

        // Get the default audio track
        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or_else(|| Error::Decode("No audio track found".to_string()))?;

        let track_id = track.id;
        let codec_params = &track.codec_params;

        // Get sample rate and channels
        let sample_rate = codec_params
            .sample_rate
            .ok_or_else(|| Error::Decode("Sample rate not found".to_string()))?;

        let channels = codec_params
            .channels
            .map(|c| c.count() as u16)
            .ok_or_else(|| Error::Decode("Channel count not found".to_string()))?;

        debug!(
            "Audio format: sample_rate={}, channels={}",
            sample_rate, channels
        );

        // Calculate target sample counts
        let start_sample_idx = ((start_ms * sample_rate as u64) / 1000) as usize * channels as usize;
        let end_sample_idx = if end_ms == 0 {
            usize::MAX // Decode to file end
        } else {
            ((end_ms * sample_rate as u64) / 1000) as usize * channels as usize
        };

        // Create decoder
        let decoder_opts = DecoderOptions::default();
        let mut decoder = get_codec_registry()
            .make(&codec_params, &decoder_opts)
            .map_err(|e| Error::Decode(format!("Failed to create decoder: {}", e)))?;

        // Decode packets until we reach passage end
        let mut all_samples = Vec::new();
        let mut current_sample_idx = 0;

        loop {
            // Stop early if we've reached passage end
            if current_sample_idx >= end_sample_idx {
                debug!("Reached passage end at sample {}, stopping decode", current_sample_idx);
                break;
            }

            // Read next packet
            let packet = match format.next_packet() {
                Ok(packet) => packet,
                Err(symphonia::core::errors::Error::IoError(ref e))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    debug!("Reached end of file at sample {}", current_sample_idx);
                    break;
                }
                Err(e) => {
                    warn!("Error reading packet: {}", e);
                    break;
                }
            };

            // Skip packets for other tracks
            if packet.track_id() != track_id {
                continue;
            }

            // Decode packet
            match decoder.decode(&packet) {
                Ok(decoded) => {
                    // Convert decoded audio to f32 samples
                    let before_len = all_samples.len();
                    Self::convert_samples_to_f32(&decoded, &mut all_samples);
                    let decoded_count = all_samples.len() - before_len;
                    current_sample_idx += decoded_count;
                }
                Err(e) => {
                    warn!("Decode error: {}", e);
                    continue;
                }
            }
        }

        // Trim to passage boundaries
        if start_sample_idx >= all_samples.len() {
            return Err(Error::InvalidTiming(format!(
                "Start time {}ms is beyond decoded audio",
                start_ms
            )));
        }

        let actual_end_sample = end_sample_idx.min(all_samples.len());

        if start_sample_idx >= actual_end_sample {
            return Err(Error::InvalidTiming(format!(
                "Invalid passage timing: start={}ms, end={}ms",
                start_ms, end_ms
            )));
        }

        // Extract the passage samples
        let passage_samples = all_samples[start_sample_idx..actual_end_sample].to_vec();

        debug!(
            "Trimmed passage: {} samples ({} frames), stopped at sample {} (saved {} samples)",
            passage_samples.len(),
            passage_samples.len() / channels as usize,
            current_sample_idx,
            all_samples.len().saturating_sub(actual_end_sample)
        );

        // **[DBD-DEC-090][DBD-DEC-095]** Calculate actual_end_ticks for undefined endpoints
        // When end_ms=0 (undefined endpoint), we decoded to EOF and need to report the
        // actual duration discovered from the file.
        let actual_end_ticks = if end_ms == 0 {
            // Calculate actual endpoint from decoded samples
            // passage_samples.len() = total interleaved stereo samples
            // Frame count = samples / 2 (stereo)
            let frame_count = passage_samples.len() / 2;

            // Convert frames to ticks at source sample rate
            let duration_ticks = wkmp_common::timing::samples_to_ticks(frame_count, sample_rate);

            // Add to start time to get absolute endpoint
            let start_ticks = wkmp_common::timing::ms_to_ticks(start_ms as i64);
            let endpoint_ticks = start_ticks + duration_ticks;

            debug!(
                "Endpoint discovered: start={}ms ({}ticks), duration={}frames ({}ticks), end={}ticks ({}ms)",
                start_ms,
                start_ticks,
                frame_count,
                duration_ticks,
                endpoint_ticks,
                wkmp_common::timing::ticks_to_ms(endpoint_ticks)
            );

            Some(endpoint_ticks)
        } else {
            // Defined endpoint - no discovery needed
            None
        };

        Ok(DecodeResult {
            samples: passage_samples,
            sample_rate,
            channels,
            actual_end_ticks,
        })
    }

    /// Convert symphonia AudioBufferRef to f32 samples.
    ///
    /// Handles various sample formats and normalizes to [-1.0, 1.0] range.
    fn convert_samples_to_f32(decoded: &AudioBufferRef, output: &mut Vec<f32>) {
        match decoded {
            AudioBufferRef::F32(buf) => {
                // Already f32, copy directly
                Self::interleave_planar_f32(buf, output);
            }
            AudioBufferRef::F64(buf) => {
                // Convert f64 to f32
                Self::interleave_planar_f64(buf, output);
            }
            AudioBufferRef::S32(buf) => {
                // Convert i32 to f32 (normalize by i32::MAX)
                Self::interleave_planar_s32(buf, output);
            }
            AudioBufferRef::S16(buf) => {
                // Convert i16 to f32 (normalize by i16::MAX)
                Self::interleave_planar_s16(buf, output);
            }
            AudioBufferRef::U32(buf) => {
                // Convert u32 to f32
                Self::interleave_planar_u32(buf, output);
            }
            AudioBufferRef::U16(buf) => {
                // Convert u16 to f32
                Self::interleave_planar_u16(buf, output);
            }
            AudioBufferRef::U8(buf) => {
                // Convert u8 to f32
                Self::interleave_planar_u8(buf, output);
            }
            AudioBufferRef::S24(buf) => {
                // Convert i24 to f32
                Self::interleave_planar_s24(buf, output);
            }
            AudioBufferRef::U24(buf) => {
                // Convert u24 to f32
                Self::interleave_planar_u24(buf, output);
            }
            AudioBufferRef::S8(buf) => {
                // Convert i8 to f32
                Self::interleave_planar_s8(buf, output);
            }
        }
    }

    /// Interleave planar f32 samples
    fn interleave_planar_f32(buf: &symphonia::core::audio::AudioBuffer<f32>, output: &mut Vec<f32>) {
        let num_channels = buf.spec().channels.count();
        let num_frames = buf.frames();

        for frame_idx in 0..num_frames {
            for ch_idx in 0..num_channels {
                let sample = buf.chan(ch_idx)[frame_idx];
                output.push(sample);
            }
        }

        // If mono, duplicate to stereo
        if num_channels == 1 {
            Self::mono_to_stereo(output);
        }
    }

    /// Interleave planar f64 samples and convert to f32
    fn interleave_planar_f64(buf: &symphonia::core::audio::AudioBuffer<f64>, output: &mut Vec<f32>) {
        let num_channels = buf.spec().channels.count();
        let num_frames = buf.frames();

        for frame_idx in 0..num_frames {
            for ch_idx in 0..num_channels {
                let sample = buf.chan(ch_idx)[frame_idx] as f32;
                output.push(sample);
            }
        }

        if num_channels == 1 {
            Self::mono_to_stereo(output);
        }
    }

    /// Interleave planar i32 samples and convert to f32
    fn interleave_planar_s32(buf: &symphonia::core::audio::AudioBuffer<i32>, output: &mut Vec<f32>) {
        let num_channels = buf.spec().channels.count();
        let num_frames = buf.frames();

        for frame_idx in 0..num_frames {
            for ch_idx in 0..num_channels {
                let sample = buf.chan(ch_idx)[frame_idx];
                output.push(sample as f32 / i32::MAX as f32);
            }
        }

        if num_channels == 1 {
            Self::mono_to_stereo(output);
        }
    }

    /// Interleave planar i16 samples and convert to f32
    fn interleave_planar_s16(buf: &symphonia::core::audio::AudioBuffer<i16>, output: &mut Vec<f32>) {
        let num_channels = buf.spec().channels.count();
        let num_frames = buf.frames();

        for frame_idx in 0..num_frames {
            for ch_idx in 0..num_channels {
                let sample = buf.chan(ch_idx)[frame_idx];
                output.push(sample as f32 / i16::MAX as f32);
            }
        }

        if num_channels == 1 {
            Self::mono_to_stereo(output);
        }
    }

    /// Interleave planar u32 samples and convert to f32
    fn interleave_planar_u32(buf: &symphonia::core::audio::AudioBuffer<u32>, output: &mut Vec<f32>) {
        let num_channels = buf.spec().channels.count();
        let num_frames = buf.frames();

        for frame_idx in 0..num_frames {
            for ch_idx in 0..num_channels {
                let sample = buf.chan(ch_idx)[frame_idx];
                // Convert u32 to signed, then normalize
                let signed = sample as i32;
                output.push(signed as f32 / i32::MAX as f32);
            }
        }

        if num_channels == 1 {
            Self::mono_to_stereo(output);
        }
    }

    /// Interleave planar u16 samples and convert to f32
    fn interleave_planar_u16(buf: &symphonia::core::audio::AudioBuffer<u16>, output: &mut Vec<f32>) {
        let num_channels = buf.spec().channels.count();
        let num_frames = buf.frames();

        for frame_idx in 0..num_frames {
            for ch_idx in 0..num_channels {
                let sample = buf.chan(ch_idx)[frame_idx];
                // Convert u16 to signed, then normalize
                let signed = (sample as i32) - 32768;
                output.push(signed as f32 / 32768.0);
            }
        }

        if num_channels == 1 {
            Self::mono_to_stereo(output);
        }
    }

    /// Interleave planar u8 samples and convert to f32
    fn interleave_planar_u8(buf: &symphonia::core::audio::AudioBuffer<u8>, output: &mut Vec<f32>) {
        let num_channels = buf.spec().channels.count();
        let num_frames = buf.frames();

        for frame_idx in 0..num_frames {
            for ch_idx in 0..num_channels {
                let sample = buf.chan(ch_idx)[frame_idx];
                // Convert u8 to signed, then normalize
                let signed = (sample as i32) - 128;
                output.push(signed as f32 / 128.0);
            }
        }

        if num_channels == 1 {
            Self::mono_to_stereo(output);
        }
    }

    /// Interleave planar i24 samples and convert to f32
    fn interleave_planar_s24(buf: &symphonia::core::audio::AudioBuffer<symphonia::core::sample::i24>, output: &mut Vec<f32>) {
        let num_channels = buf.spec().channels.count();
        let num_frames = buf.frames();

        for frame_idx in 0..num_frames {
            for ch_idx in 0..num_channels {
                let sample = buf.chan(ch_idx)[frame_idx];
                // i24 is a 24-bit signed integer, normalize by 2^23
                // Convert i24 to i32 first (using inner() instead of deprecated into_i32())
                let sample_i32 = sample.inner();
                output.push(sample_i32 as f32 / 8388608.0);
            }
        }

        if num_channels == 1 {
            Self::mono_to_stereo(output);
        }
    }

    /// Interleave planar u24 samples and convert to f32
    fn interleave_planar_u24(buf: &symphonia::core::audio::AudioBuffer<symphonia::core::sample::u24>, output: &mut Vec<f32>) {
        let num_channels = buf.spec().channels.count();
        let num_frames = buf.frames();

        for frame_idx in 0..num_frames {
            for ch_idx in 0..num_channels {
                let sample = buf.chan(ch_idx)[frame_idx];
                // u24 is a 24-bit unsigned integer, convert to signed and normalize
                let sample_u32 = sample.inner();
                let signed = (sample_u32 as i32) - 8388608; // Center around 0
                output.push(signed as f32 / 8388608.0);
            }
        }

        if num_channels == 1 {
            Self::mono_to_stereo(output);
        }
    }

    /// Interleave planar i8 samples and convert to f32
    fn interleave_planar_s8(buf: &symphonia::core::audio::AudioBuffer<i8>, output: &mut Vec<f32>) {
        let num_channels = buf.spec().channels.count();
        let num_frames = buf.frames();

        for frame_idx in 0..num_frames {
            for ch_idx in 0..num_channels {
                let sample = buf.chan(ch_idx)[frame_idx];
                output.push(sample as f32 / i8::MAX as f32);
            }
        }

        if num_channels == 1 {
            Self::mono_to_stereo(output);
        }
    }

    /// Convert mono samples to stereo by duplicating channel.
    ///
    /// Modifies the output vector in place: [L, L, L] -> [L, L, L, L, L, L]
    fn mono_to_stereo(samples: &mut Vec<f32>) {
        let original_len = samples.len();
        samples.reserve(original_len); // Reserve space for duplication

        // We need to insert in reverse order to avoid shifting issues
        for i in (0..original_len).rev() {
            let sample = samples[i];
            samples.insert(i + 1, sample); // Duplicate after current position
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mono_to_stereo() {
        let mut samples = vec![0.1, 0.2, 0.3];
        SimpleDecoder::mono_to_stereo(&mut samples);
        assert_eq!(samples, vec![0.1, 0.1, 0.2, 0.2, 0.3, 0.3]);
    }

    // Note: File decoding tests require actual audio files
    // These should be integration tests with test fixtures
}
