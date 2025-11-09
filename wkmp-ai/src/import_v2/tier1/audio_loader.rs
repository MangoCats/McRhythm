// PLAN023: Audio File Loader
//
// Provides PCM extraction from audio files using symphonia.
// Handles tick-based range extraction for passage boundaries.
//
// Requirements: REQ-AI-010 (Audio File Loading)
// Architecture: Tier 1 (Source Extractor)

use anyhow::{Context, Result};
use std::path::Path;
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use tracing::debug;
use rubato::{Resampler, SincFixedIn, SincInterpolationType, SincInterpolationParameters, WindowFunction};

/// SPEC017 tick rate: 28,224,000 Hz (1 tick ≈ 35.4 nanoseconds)
pub const TICK_RATE: i64 = 28_224_000;

/// Audio file loader with tick-based range extraction
pub struct AudioLoader {
    /// Target sample rate for output PCM (44.1 kHz)
    target_sample_rate: u32,
}

impl Default for AudioLoader {
    fn default() -> Self {
        Self {
            target_sample_rate: 44100,
        }
    }
}

impl AudioLoader {
    /// Create new audio loader with target sample rate
    pub fn new(target_sample_rate: u32) -> Self {
        Self {
            target_sample_rate,
        }
    }

    /// Extract PCM samples from audio file for specified tick range
    ///
    /// **[REQ-AI-010]** Load audio data for passage boundaries
    ///
    /// # Arguments
    /// * `file_path` - Path to audio file (FLAC, MP3, AAC, etc.)
    /// * `start_ticks` - Start position in ticks from file beginning
    /// * `end_ticks` - End position in ticks from file beginning
    ///
    /// # Returns
    /// * `Ok(AudioSegment)` - Extracted PCM data
    /// * `Err` - Decode error, unsupported format, or I/O error
    ///
    /// # Example
    /// ```no_run
    /// # use wkmp_ai::import_v2::tier1::audio_loader::AudioLoader;
    /// # use std::path::Path;
    /// let loader = AudioLoader::default();
    /// // Extract 10 seconds (10s * 28,224,000 ticks/s = 282,240,000 ticks)
    /// let segment = loader.load_segment(Path::new("music.flac"), 0, 282_240_000)?;
    /// assert_eq!(segment.sample_rate, 44100);
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn load_segment<P: AsRef<Path>>(
        &self,
        file_path: P,
        start_ticks: i64,
        end_ticks: i64,
    ) -> Result<AudioSegment> {
        let path = file_path.as_ref();
        debug!(
            "Loading audio segment: {} ticks [{}, {}] ({:.2}s - {:.2}s)",
            path.display(),
            start_ticks,
            end_ticks,
            ticks_to_seconds(start_ticks),
            ticks_to_seconds(end_ticks)
        );

        // Open file
        let file = std::fs::File::open(path)
            .with_context(|| format!("Failed to open audio file: {}", path.display()))?;

        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        // Probe format
        let mut hint = Hint::new();
        if let Some(ext) = path.extension() {
            hint.with_extension(ext.to_str().unwrap_or(""));
        }

        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
            .context("Failed to probe audio format")?;

        let mut format = probed.format;

        // Find default audio track
        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .context("No audio tracks found in file")?;

        let track_id = track.id;
        let codec_params = track.codec_params.clone();

        // Get native sample rate
        let native_sample_rate = codec_params
            .sample_rate
            .context("Sample rate not specified in codec params")?;

        debug!(
            "Native sample rate: {} Hz, Target: {} Hz",
            native_sample_rate, self.target_sample_rate
        );

        // Create decoder
        let mut decoder = symphonia::default::get_codecs()
            .make(&codec_params, &DecoderOptions::default())
            .context("Failed to create decoder")?;

        // Calculate sample ranges
        let start_sample = ticks_to_samples(start_ticks, native_sample_rate);
        let end_sample = ticks_to_samples(end_ticks, native_sample_rate);
        let target_samples = (end_sample - start_sample) as usize;

        debug!(
            "Sample range: {} - {} ({} samples at {} Hz)",
            start_sample, end_sample, target_samples, native_sample_rate
        );

        // Decode and collect samples
        let mut all_samples = Vec::new();
        let mut current_sample = 0u64;

        loop {
            let packet = match format.next_packet() {
                Ok(packet) => packet,
                Err(symphonia::core::errors::Error::IoError(e))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    break;
                }
                Err(e) => return Err(e).context("Failed to read packet")?,
            };

            // Only decode packets from our track
            if packet.track_id() != track_id {
                continue;
            }

            let decoded = decoder.decode(&packet).context("Failed to decode packet")?;

            // Get stereo samples (convert mono to stereo if needed)
            let stereo_samples = match decoded {
                AudioBufferRef::F32(buf) => extract_stereo_f32(&buf),
                AudioBufferRef::U8(buf) => convert_u8_to_f32(&buf),
                AudioBufferRef::U16(buf) => convert_u16_to_f32(&buf),
                AudioBufferRef::U24(buf) => convert_u24_to_f32(&buf),
                AudioBufferRef::U32(buf) => convert_u32_to_f32(&buf),
                AudioBufferRef::S8(buf) => convert_s8_to_f32(&buf),
                AudioBufferRef::S16(buf) => convert_s16_to_f32(&buf),
                AudioBufferRef::S24(buf) => convert_s24_to_f32(&buf),
                AudioBufferRef::S32(buf) => convert_s32_to_f32(&buf),
                AudioBufferRef::F64(buf) => convert_f64_to_f32(&buf),
            };

            let frames_in_packet = stereo_samples.len() / 2;
            let packet_start = current_sample;
            let packet_end = current_sample + frames_in_packet as u64;

            // Check if this packet overlaps our target range
            if packet_end > start_sample && packet_start < end_sample {
                // Calculate overlap
                let overlap_start = packet_start.max(start_sample) - packet_start;
                let overlap_end = packet_end.min(end_sample) - packet_start;

                let start_idx = (overlap_start * 2) as usize; // stereo
                let end_idx = (overlap_end * 2) as usize;

                all_samples.extend_from_slice(&stereo_samples[start_idx..end_idx]);
            }

            current_sample = packet_end;

            // Stop if we've passed the end
            if current_sample >= end_sample {
                break;
            }
        }

        debug!(
            "Decoded {} stereo samples ({} frames)",
            all_samples.len(),
            all_samples.len() / 2
        );

        // Resample if needed
        let final_samples = if native_sample_rate != self.target_sample_rate {
            debug!(
                "Resampling from {} Hz to {} Hz using rubato",
                native_sample_rate, self.target_sample_rate
            );
            self.resample_stereo(all_samples, native_sample_rate)
                .context("Failed to resample audio")?
        } else {
            all_samples
        };

        Ok(AudioSegment {
            samples: final_samples,
            sample_rate: self.target_sample_rate,
            channels: 2,
        })
    }

    /// Load entire audio file as PCM
    pub fn load_full<P: AsRef<Path>>(&self, file_path: P) -> Result<AudioSegment> {
        // Load from tick 0 to a very large tick value (effectively the whole file)
        self.load_segment(file_path, 0, i64::MAX)
    }

    /// Resample interleaved stereo PCM samples to target sample rate
    ///
    /// **[P1-2]** High-quality resampling using rubato SincFixedIn
    ///
    /// # Arguments
    /// * `samples` - Interleaved stereo samples (L, R, L, R, ...)
    /// * `source_rate` - Original sample rate in Hz
    ///
    /// # Returns
    /// * Resampled interleaved stereo samples at target_sample_rate
    ///
    /// # Algorithm
    /// - Uses sinc interpolation with BlackmanHarris2 window
    /// - 256-tap filter for high-quality resampling
    /// - 0.95 cutoff frequency to prevent aliasing
    /// - Processes in chunks to handle arbitrary input lengths
    fn resample_stereo(&self, samples: Vec<f32>, source_rate: u32) -> Result<Vec<f32>> {
        if samples.is_empty() {
            return Ok(samples);
        }

        let num_frames = samples.len() / 2;

        // De-interleave stereo samples into separate channels
        let mut left = Vec::with_capacity(num_frames);
        let mut right = Vec::with_capacity(num_frames);

        for i in 0..num_frames {
            left.push(samples[i * 2]);
            right.push(samples[i * 2 + 1]);
        }

        // Configure high-quality sinc interpolation
        let params = SincInterpolationParameters {
            sinc_len: 256,           // 256-tap filter for high quality
            f_cutoff: 0.95,          // 95% of Nyquist to prevent aliasing
            interpolation: SincInterpolationType::Linear,
            oversampling_factor: 256,
            window: WindowFunction::BlackmanHarris2,
        };

        let resample_ratio = self.target_sample_rate as f64 / source_rate as f64;

        // Create resampler for stereo (2 channels)
        // Use chunk size equal to input length for single-pass processing
        let mut resampler = SincFixedIn::<f32>::new(
            resample_ratio,
            2.0,                     // Max resample ratio factor (allows 2x up/down)
            params,
            num_frames,              // Chunk size = input length
            2,                       // 2 channels (stereo)
        ).context("Failed to create rubato resampler")?;

        // Prepare input as Vec<Vec<f32>> (per-channel)
        let input_channels = vec![left, right];

        // Process resampling
        let output_channels = resampler
            .process(&input_channels, None)
            .context("Rubato resampling failed")?;

        // Re-interleave output channels
        let output_frames = output_channels[0].len();
        let mut output = Vec::with_capacity(output_frames * 2);

        for i in 0..output_frames {
            output.push(output_channels[0][i]);
            output.push(output_channels[1][i]);
        }

        debug!(
            "Resampled {} frames ({} Hz) → {} frames ({} Hz)",
            num_frames, source_rate, output_frames, self.target_sample_rate
        );

        Ok(output)
    }
}

/// Audio segment with PCM data
#[derive(Debug, Clone)]
pub struct AudioSegment {
    /// Interleaved stereo PCM samples (f32, normalized -1.0 to 1.0)
    pub samples: Vec<f32>,
    /// Sample rate in Hz
    pub sample_rate: u32,
    /// Number of channels (always 2 for stereo)
    pub channels: u8,
}

impl AudioSegment {
    /// Get duration in ticks
    pub fn duration_ticks(&self) -> i64 {
        let frames = (self.samples.len() / 2) as i64;
        samples_to_ticks(frames as u64, self.sample_rate)
    }

    /// Get duration in seconds
    pub fn duration_seconds(&self) -> f64 {
        ticks_to_seconds(self.duration_ticks())
    }
}

// ============================================================================
// Tick <-> Sample Conversion Utilities
// ============================================================================

/// Convert ticks to seconds
#[inline]
pub fn ticks_to_seconds(ticks: i64) -> f64 {
    ticks as f64 / TICK_RATE as f64
}

/// Convert seconds to ticks
#[inline]
pub fn seconds_to_ticks(seconds: f64) -> i64 {
    (seconds * TICK_RATE as f64).round() as i64
}

/// Convert ticks to sample index at given sample rate
#[inline]
pub fn ticks_to_samples(ticks: i64, sample_rate: u32) -> u64 {
    let seconds = ticks as f64 / TICK_RATE as f64;
    (seconds * sample_rate as f64).round() as u64
}

/// Convert sample index to ticks at given sample rate
#[inline]
pub fn samples_to_ticks(samples: u64, sample_rate: u32) -> i64 {
    let seconds = samples as f64 / sample_rate as f64;
    (seconds * TICK_RATE as f64).round() as i64
}

// ============================================================================
// Sample Format Conversion
// ============================================================================

fn extract_stereo_f32(buf: &symphonia::core::audio::AudioBuffer<f32>) -> Vec<f32> {
    let channels = buf.spec().channels.count();
    let frames = buf.frames();

    if channels == 2 {
        // Already stereo, interleave
        let left = buf.chan(0);
        let right = buf.chan(1);
        let mut output = Vec::with_capacity(frames * 2);
        for i in 0..frames {
            output.push(left[i]);
            output.push(right[i]);
        }
        output
    } else if channels == 1 {
        // Mono, duplicate to stereo
        let mono = buf.chan(0);
        let mut output = Vec::with_capacity(frames * 2);
        for &sample in mono.iter().take(frames) {
            output.push(sample);
            output.push(sample);
        }
        output
    } else {
        // Multi-channel, downmix to stereo (take first 2 channels)
        let left = buf.chan(0);
        let right = buf.chan(1.min(channels - 1));
        let mut output = Vec::with_capacity(frames * 2);
        for i in 0..frames {
            output.push(left[i]);
            output.push(right[i]);
        }
        output
    }
}

// Helper conversion functions for other sample formats
fn convert_u8_to_f32(buf: &symphonia::core::audio::AudioBuffer<u8>) -> Vec<f32> {
    let channels = buf.spec().channels.count();
    let frames = buf.frames();
    let mut output = Vec::with_capacity(frames * 2);

    let left = buf.chan(0);
    let right = if channels > 1 {
        buf.chan(1)
    } else {
        buf.chan(0)
    };

    for i in 0..frames {
        output.push((left[i] as f32 - 128.0) / 128.0);
        output.push((right[i] as f32 - 128.0) / 128.0);
    }
    output
}

fn convert_s16_to_f32(buf: &symphonia::core::audio::AudioBuffer<i16>) -> Vec<f32> {
    let channels = buf.spec().channels.count();
    let frames = buf.frames();
    let mut output = Vec::with_capacity(frames * 2);

    let left = buf.chan(0);
    let right = if channels > 1 {
        buf.chan(1)
    } else {
        buf.chan(0)
    };

    for i in 0..frames {
        output.push(left[i] as f32 / 32768.0);
        output.push(right[i] as f32 / 32768.0);
    }
    output
}

fn convert_s32_to_f32(buf: &symphonia::core::audio::AudioBuffer<i32>) -> Vec<f32> {
    let channels = buf.spec().channels.count();
    let frames = buf.frames();
    let mut output = Vec::with_capacity(frames * 2);

    let left = buf.chan(0);
    let right = if channels > 1 {
        buf.chan(1)
    } else {
        buf.chan(0)
    };

    for i in 0..frames {
        output.push(left[i] as f32 / 2147483648.0);
        output.push(right[i] as f32 / 2147483648.0);
    }
    output
}

fn convert_f64_to_f32(buf: &symphonia::core::audio::AudioBuffer<f64>) -> Vec<f32> {
    let channels = buf.spec().channels.count();
    let frames = buf.frames();
    let mut output = Vec::with_capacity(frames * 2);

    let left = buf.chan(0);
    let right = if channels > 1 {
        buf.chan(1)
    } else {
        buf.chan(0)
    };

    for i in 0..frames {
        output.push(left[i] as f32);
        output.push(right[i] as f32);
    }
    output
}

// Stub implementations for less common formats
fn convert_u16_to_f32(buf: &symphonia::core::audio::AudioBuffer<u16>) -> Vec<f32> {
    let channels = buf.spec().channels.count();
    let frames = buf.frames();
    let mut output = Vec::with_capacity(frames * 2);

    let left = buf.chan(0);
    let right = if channels > 1 {
        buf.chan(1)
    } else {
        buf.chan(0)
    };

    for i in 0..frames {
        output.push((left[i] as f32 - 32768.0) / 32768.0);
        output.push((right[i] as f32 - 32768.0) / 32768.0);
    }
    output
}

fn convert_u24_to_f32(buf: &symphonia::core::audio::AudioBuffer<symphonia::core::sample::u24>) -> Vec<f32> {
    let channels = buf.spec().channels.count();
    let frames = buf.frames();
    let mut output = Vec::with_capacity(frames * 2);

    let left = buf.chan(0);
    let right = if channels > 1 {
        buf.chan(1)
    } else {
        buf.chan(0)
    };

    for i in 0..frames {
        // u24 is stored as i32 internally, center at 8388608 (2^23)
        output.push((left[i].inner() as f32 - 8388608.0) / 8388608.0);
        output.push((right[i].inner() as f32 - 8388608.0) / 8388608.0);
    }
    output
}

fn convert_u32_to_f32(buf: &symphonia::core::audio::AudioBuffer<u32>) -> Vec<f32> {
    let channels = buf.spec().channels.count();
    let frames = buf.frames();
    let mut output = Vec::with_capacity(frames * 2);

    let left = buf.chan(0);
    let right = if channels > 1 {
        buf.chan(1)
    } else {
        buf.chan(0)
    };

    for i in 0..frames {
        output.push((left[i] as f32 - 2147483648.0) / 2147483648.0);
        output.push((right[i] as f32 - 2147483648.0) / 2147483648.0);
    }
    output
}

fn convert_s8_to_f32(buf: &symphonia::core::audio::AudioBuffer<i8>) -> Vec<f32> {
    let channels = buf.spec().channels.count();
    let frames = buf.frames();
    let mut output = Vec::with_capacity(frames * 2);

    let left = buf.chan(0);
    let right = if channels > 1 {
        buf.chan(1)
    } else {
        buf.chan(0)
    };

    for i in 0..frames {
        output.push(left[i] as f32 / 128.0);
        output.push(right[i] as f32 / 128.0);
    }
    output
}

fn convert_s24_to_f32(buf: &symphonia::core::audio::AudioBuffer<symphonia::core::sample::i24>) -> Vec<f32> {
    let channels = buf.spec().channels.count();
    let frames = buf.frames();
    let mut output = Vec::with_capacity(frames * 2);

    let left = buf.chan(0);
    let right = if channels > 1 {
        buf.chan(1)
    } else {
        buf.chan(0)
    };

    for i in 0..frames {
        // i24 is stored as i32 internally
        output.push(left[i].inner() as f32 / 8388608.0);
        output.push(right[i].inner() as f32 / 8388608.0);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tick_conversions() {
        // 1 second = 28,224,000 ticks
        assert_eq!(seconds_to_ticks(1.0), TICK_RATE);
        assert_eq!(ticks_to_seconds(TICK_RATE), 1.0);

        // 10 seconds
        assert_eq!(seconds_to_ticks(10.0), TICK_RATE * 10);
        assert_eq!(ticks_to_seconds(TICK_RATE * 10), 10.0);
    }

    #[test]
    fn test_sample_conversions() {
        // At 44.1 kHz, 1 second = 44,100 samples
        let ticks_1s = seconds_to_ticks(1.0);
        assert_eq!(ticks_to_samples(ticks_1s, 44100), 44100);

        // At 48 kHz, 1 second = 48,000 samples
        assert_eq!(ticks_to_samples(ticks_1s, 48000), 48000);

        // Reverse conversion
        assert_eq!(samples_to_ticks(44100, 44100), ticks_1s);
        assert_eq!(samples_to_ticks(48000, 48000), ticks_1s);
    }

    #[test]
    fn test_audio_segment_duration() {
        // 1 second of stereo at 44.1 kHz = 88,200 samples
        let segment = AudioSegment {
            samples: vec![0.0; 88200],
            sample_rate: 44100,
            channels: 2,
        };

        assert_eq!(segment.duration_ticks(), TICK_RATE);
        assert!((segment.duration_seconds() - 1.0).abs() < 0.001);
    }

    // ============================================================================
    // [P1-2] Resampling Tests
    // ============================================================================

    #[test]
    fn test_resample_48khz_to_44khz() {
        // Test downsampling from 48 kHz to 44.1 kHz
        let loader = AudioLoader::new(44100);

        // Create 1 second of 48 kHz stereo sine wave (440 Hz, A4 note)
        let source_rate = 48000;
        let duration_s = 1.0;
        let frequency = 440.0;
        let num_frames = (source_rate as f64 * duration_s) as usize;

        let mut samples = Vec::with_capacity(num_frames * 2);
        for i in 0..num_frames {
            let t = i as f64 / source_rate as f64;
            let sample = (2.0 * std::f64::consts::PI * frequency * t).sin() as f32;
            samples.push(sample); // left
            samples.push(sample); // right
        }

        // Resample
        let resampled = loader.resample_stereo(samples, source_rate).unwrap();

        // Expected output length: 44100 frames * 2 channels = 88,200 samples
        let expected_frames = (44100.0 * duration_s) as usize;
        let expected_samples = expected_frames * 2;

        // Allow ±1% tolerance for frame count due to rounding
        let tolerance = (expected_samples as f64 * 0.01) as usize;
        assert!(
            resampled.len() >= expected_samples - tolerance
                && resampled.len() <= expected_samples + tolerance,
            "Expected ~{} samples, got {}",
            expected_samples,
            resampled.len()
        );

        // Verify samples are in valid range [-1.0, 1.0] with small tolerance for ringing
        // Sinc interpolation can produce slight overshoot due to Gibbs phenomenon
        for (i, &sample) in resampled.iter().enumerate() {
            assert!(
                sample >= -1.01 && sample <= 1.01,
                "Sample {} out of range: {} (expected [-1.01, 1.01])",
                i,
                sample
            );
        }
    }

    #[test]
    fn test_resample_96khz_to_44khz() {
        // Test downsampling from 96 kHz to 44.1 kHz (larger ratio)
        let loader = AudioLoader::new(44100);

        // Create 0.5 seconds of 96 kHz stereo silence
        let source_rate = 96000;
        let duration_s = 0.5;
        let num_frames = (source_rate as f64 * duration_s) as usize;

        let samples = vec![0.0; num_frames * 2];

        // Resample
        let resampled = loader.resample_stereo(samples, source_rate).unwrap();

        // Expected output: 22,050 frames * 2 channels = 44,100 samples
        let expected_frames = (44100.0 * duration_s) as usize;
        let expected_samples = expected_frames * 2;

        // Allow ±1% tolerance
        let tolerance = (expected_samples as f64 * 0.01) as usize;
        assert!(
            resampled.len() >= expected_samples - tolerance
                && resampled.len() <= expected_samples + tolerance,
            "Expected ~{} samples, got {}",
            expected_samples,
            resampled.len()
        );

        // All samples should remain 0.0
        for &sample in &resampled {
            assert_eq!(sample, 0.0, "Expected silence to remain silence");
        }
    }

    #[test]
    fn test_resample_empty_input() {
        // Test edge case: empty input
        let loader = AudioLoader::new(44100);
        let empty = Vec::new();

        let resampled = loader.resample_stereo(empty, 48000).unwrap();

        assert_eq!(resampled.len(), 0, "Empty input should produce empty output");
    }

    #[test]
    fn test_resample_preserves_stereo_separation() {
        // Test that left/right channels remain distinct after resampling
        let loader = AudioLoader::new(44100);
        let source_rate = 48000;
        let num_frames = 1000;

        // Create stereo with different values on each channel
        let mut samples = Vec::with_capacity(num_frames * 2);
        for _ in 0..num_frames {
            samples.push(0.5);  // left = 0.5
            samples.push(-0.5); // right = -0.5
        }

        let resampled = loader.resample_stereo(samples, source_rate).unwrap();

        // Check that left/right channels are still different
        // (they should be approximately 0.5 and -0.5, allowing some error)
        for i in 0..(resampled.len() / 2) {
            let left = resampled[i * 2];
            let right = resampled[i * 2 + 1];

            assert!(
                (left - 0.5).abs() < 0.1,
                "Left channel should be ~0.5, got {}",
                left
            );
            assert!(
                (right - (-0.5)).abs() < 0.1,
                "Right channel should be ~-0.5, got {}",
                right
            );
        }
    }
}
