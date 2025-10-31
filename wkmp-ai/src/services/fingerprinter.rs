//! Audio fingerprinting service using Chromaprint
//!
//! **[AIA-COMP-010]** Chromaprint fingerprint generation (Decision 3: static linking)
//! Uses chromaprint-sys-next for official Chromaprint algorithm without external binary

use base64::{engine::general_purpose, Engine as _};
use std::path::Path;
use symphonia::core::audio::AudioBufferRef;
use thiserror::Error;

/// Fingerprinting errors
#[derive(Debug, Error)]
pub enum FingerprintError {
    #[error("Failed to decode audio: {0}")]
    DecodeError(String),

    #[error("Chromaprint error: {0}")]
    ChromaprintError(String),

    #[error("Audio too short (minimum 10 seconds required)")]
    AudioTooShort,

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Resample error: {0}")]
    ResampleError(String),
}

/// Audio fingerprinter
pub struct Fingerprinter {
    /// Use first N seconds for fingerprinting (default: 120 seconds)
    duration_seconds: usize,
}

impl Fingerprinter {
    /// Create new fingerprinter
    pub fn new() -> Self {
        Self {
            duration_seconds: 120, // AcoustID recommends 120 seconds
        }
    }

    /// Set fingerprint duration
    pub fn with_duration(mut self, seconds: usize) -> Self {
        self.duration_seconds = seconds;
        self
    }

    /// Generate Chromaprint fingerprint from audio file
    ///
    /// **[TC-COMP-005]** Chromaprint generation test
    ///
    /// Returns base64-encoded fingerprint string suitable for AcoustID API
    pub fn fingerprint_file(&self, audio_path: &Path) -> Result<String, FingerprintError> {
        // Decode audio to PCM
        let (pcm_data, sample_rate) = self.decode_audio(audio_path)?;

        // Generate fingerprint from PCM
        self.fingerprint_pcm(&pcm_data, sample_rate)
    }

    /// Generate fingerprint from PCM data
    ///
    /// PCM data should be:
    /// - Mono (single channel)
    /// - 16-bit signed integers
    /// - Sample rate typically 44100 Hz or 48000 Hz
    pub fn fingerprint_pcm(
        &self,
        pcm_data: &[i16],
        sample_rate: u32,
    ) -> Result<String, FingerprintError> {
        // Verify minimum length (10 seconds)
        let min_samples = sample_rate as usize * 10;
        if pcm_data.len() < min_samples {
            return Err(FingerprintError::AudioTooShort);
        }

        // Truncate to desired duration
        let max_samples = sample_rate as usize * self.duration_seconds;
        let samples_to_use = pcm_data.len().min(max_samples);
        let pcm_slice = &pcm_data[..samples_to_use];

        // Resample to 44.1kHz if needed
        let (resampled_pcm, resampled_rate) = self.resample_to_44100(pcm_slice, sample_rate)?;

        // Generate Chromaprint fingerprint (returns Base64-encoded string)
        let fingerprint = self.generate_chromaprint(&resampled_pcm, resampled_rate)?;

        Ok(fingerprint)
    }

    /// Generate Chromaprint fingerprint using FFI
    ///
    /// **[REQ-FP-030]** Chromaprint fingerprinting with chromaprint-sys-next
    ///
    /// # Safety
    ///
    /// Uses unsafe FFI calls to chromaprint C library. All FFI calls are wrapped
    /// with error checking and proper resource cleanup (RAII pattern).
    fn generate_chromaprint(
        &self,
        pcm_data: &[i16],
        sample_rate: u32,
    ) -> Result<String, FingerprintError> {
        use chromaprint_sys_next::*;

        unsafe {
            // Step 1: Allocate Chromaprint context (algorithm 0 = DEFAULT, required for AcoustID)
            let ctx = chromaprint_new(0);
            if ctx.is_null() {
                return Err(FingerprintError::ChromaprintError(
                    "Failed to create Chromaprint context".to_string()
                ));
            }

            // Step 2: Start fingerprinting (sample_rate, 1 channel = mono)
            let ret = chromaprint_start(ctx, sample_rate as i32, 1);
            if ret != 1 {
                chromaprint_free(ctx);
                return Err(FingerprintError::ChromaprintError(
                    "chromaprint_start failed".to_string()
                ));
            }

            // Step 3: Feed audio samples to Chromaprint
            let ret = chromaprint_feed(ctx, pcm_data.as_ptr(), pcm_data.len() as i32);
            if ret != 1 {
                chromaprint_free(ctx);
                return Err(FingerprintError::ChromaprintError(
                    "chromaprint_feed failed".to_string()
                ));
            }

            // Step 4: Finish processing
            let ret = chromaprint_finish(ctx);
            if ret != 1 {
                chromaprint_free(ctx);
                return Err(FingerprintError::ChromaprintError(
                    "chromaprint_finish failed".to_string()
                ));
            }

            // Step 5: Get fingerprint as compressed string
            let mut fp_ptr: *mut i8 = std::ptr::null_mut();
            let ret = chromaprint_get_fingerprint(ctx, &mut fp_ptr);
            if ret != 1 || fp_ptr.is_null() {
                chromaprint_free(ctx);
                return Err(FingerprintError::ChromaprintError(
                    "chromaprint_get_fingerprint failed".to_string()
                ));
            }

            // Step 6: Convert C string to Rust String
            let c_str = std::ffi::CStr::from_ptr(fp_ptr);
            let fingerprint = c_str.to_str()
                .map_err(|e| {
                    chromaprint_dealloc(fp_ptr as *mut std::ffi::c_void);
                    chromaprint_free(ctx);
                    FingerprintError::ChromaprintError(format!("UTF-8 conversion failed: {}", e))
                })?
                .to_string();

            // Step 7: Free resources
            chromaprint_dealloc(fp_ptr as *mut std::ffi::c_void);
            chromaprint_free(ctx);

            tracing::debug!(
                fingerprint_len = fingerprint.len(),
                fingerprint_preview = &fingerprint[..fingerprint.len().min(50)],
                "Generated Chromaprint fingerprint"
            );

            Ok(fingerprint)
        }
    }

    /// Encode fingerprint as base64
    ///
    /// **[TC-COMP-006]** Base64 encoding test
    pub fn encode_base64(&self, raw_fingerprint: &[u8]) -> String {
        general_purpose::STANDARD.encode(raw_fingerprint)
    }

    /// Decode audio file to mono PCM i16
    ///
    /// **[REQ-FP-010]** Audio decoding with Symphonia
    fn decode_audio(&self, audio_path: &Path) -> Result<(Vec<i16>, u32), FingerprintError> {
        use symphonia::core::codecs::DecoderOptions;
        use symphonia::core::formats::FormatOptions;
        use symphonia::core::io::MediaSourceStream;
        use symphonia::core::meta::MetadataOptions;
        use symphonia::core::probe::Hint;
        use std::fs::File;

        // Open the audio file
        let file = File::open(audio_path)?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());

        // Create the format reader (probe for format)
        let mut hint = Hint::new();
        if let Some(extension) = audio_path.extension() {
            if let Some(ext_str) = extension.to_str() {
                hint.with_extension(ext_str);
            }
        }

        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
            .map_err(|e| FingerprintError::DecodeError(format!("Format probe failed: {}", e)))?;

        let mut format_reader = probed.format;

        // Get the default audio track
        let track = format_reader
            .default_track()
            .ok_or_else(|| FingerprintError::DecodeError("No audio track found".to_string()))?;

        let sample_rate = track
            .codec_params
            .sample_rate
            .ok_or_else(|| FingerprintError::DecodeError("No sample rate in track".to_string()))?;

        // Create the decoder
        let mut decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())
            .map_err(|e| FingerprintError::DecodeError(format!("Decoder creation failed: {}", e)))?;

        let track_id = track.id; // Copy track ID to avoid borrow issues

        let mut samples_i16 = Vec::new();
        let max_samples = sample_rate as usize * self.duration_seconds;

        // Decode packets until we have enough samples
        while let Ok(packet) = format_reader.next_packet() {
            // Only decode packets from the audio track
            if packet.track_id() != track_id {
                continue;
            }

            // Decode the packet
            match decoder.decode(&packet) {
                Ok(decoded) => {
                    // Convert to mono i16
                    let mono_i16 = self.convert_to_mono_i16(&decoded);
                    samples_i16.extend_from_slice(&mono_i16);

                    // Stop if we have enough samples
                    if samples_i16.len() >= max_samples {
                        samples_i16.truncate(max_samples);
                        break;
                    }
                }
                Err(e) => {
                    tracing::warn!("Decode error (continuing): {}", e);
                    // Continue on non-fatal decode errors
                }
            }
        }

        Ok((samples_i16, sample_rate))
    }

    /// Resample audio to 44.1kHz if needed
    ///
    /// **[REQ-FP-020]** Audio resampling using Rubato
    fn resample_to_44100(&self, pcm_data: &[i16], sample_rate: u32) -> Result<(Vec<i16>, u32), FingerprintError> {
        const TARGET_SAMPLE_RATE: u32 = 44100;

        // Skip resampling if already at target rate (optimization)
        if sample_rate == TARGET_SAMPLE_RATE {
            return Ok((pcm_data.to_vec(), sample_rate));
        }

        use rubato::{
            Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
        };

        // Convert i16 to f32 for resampling (Rubato works with f32)
        let samples_f32: Vec<f32> = pcm_data
            .iter()
            .map(|&s| s as f32 / 32768.0) // Normalize to [-1.0, 1.0]
            .collect();

        // Configure high-quality resampler
        let params = SincInterpolationParameters {
            sinc_len: 256,
            f_cutoff: 0.95,
            interpolation: SincInterpolationType::Linear,
            oversampling_factor: 256,
            window: WindowFunction::BlackmanHarris2,
        };

        let resample_ratio = TARGET_SAMPLE_RATE as f64 / sample_rate as f64;

        let mut resampler = SincFixedIn::<f32>::new(
            resample_ratio,
            2.0,
            params,
            samples_f32.len(),
            1, // Mono
        )
        .map_err(|e| FingerprintError::ResampleError(e.to_string()))?;

        // Process audio (Rubato expects Vec<Vec<f32>> for multi-channel)
        let waves_in = vec![samples_f32];
        let waves_out = resampler
            .process(&waves_in, None)
            .map_err(|e| FingerprintError::ResampleError(e.to_string()))?;

        // Convert f32 back to i16
        let resampled_i16: Vec<i16> = waves_out[0]
            .iter()
            .map(|&s| (s * 32768.0).clamp(-32768.0, 32767.0) as i16)
            .collect();

        Ok((resampled_i16, TARGET_SAMPLE_RATE))
    }

    /// Convert Symphonia AudioBuffer to mono i16
    ///
    /// **[REQ-FP-020]** Channel mixing for mono conversion
    fn convert_to_mono_i16(&self, buffer: &AudioBufferRef) -> Vec<i16> {
        use symphonia::core::audio::Signal;
        use symphonia::core::conv::FromSample;

        let channels = buffer.spec().channels.count();
        let frames = buffer.frames();
        let mut mono = Vec::with_capacity(frames);

        // Mix down to mono by averaging channels
        for frame_idx in 0..frames {
            let mut sum = 0.0f32;
            for ch in 0..channels {
                // Get sample and convert to f32
                let sample = match buffer {
                    AudioBufferRef::U8(buf) => f32::from_sample(buf.chan(ch)[frame_idx]),
                    AudioBufferRef::U16(buf) => f32::from_sample(buf.chan(ch)[frame_idx]),
                    AudioBufferRef::U24(buf) => f32::from_sample(buf.chan(ch)[frame_idx]),
                    AudioBufferRef::U32(buf) => f32::from_sample(buf.chan(ch)[frame_idx]),
                    AudioBufferRef::S8(buf) => f32::from_sample(buf.chan(ch)[frame_idx]),
                    AudioBufferRef::S16(buf) => f32::from_sample(buf.chan(ch)[frame_idx]),
                    AudioBufferRef::S24(buf) => f32::from_sample(buf.chan(ch)[frame_idx]),
                    AudioBufferRef::S32(buf) => f32::from_sample(buf.chan(ch)[frame_idx]),
                    AudioBufferRef::F32(buf) => buf.chan(ch)[frame_idx],
                    AudioBufferRef::F64(buf) => buf.chan(ch)[frame_idx] as f32,
                };
                sum += sample;
            }
            // Average and convert to i16
            let avg = sum / channels as f32;
            let i16_sample = (avg * 32767.0).clamp(-32768.0, 32767.0) as i16;
            mono.push(i16_sample);
        }

        mono
    }
}

impl Default for Fingerprinter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fingerprinter_creation() {
        let fp = Fingerprinter::new();
        assert_eq!(fp.duration_seconds, 120);
    }

    #[test]
    fn test_fingerprinter_with_duration() {
        let fp = Fingerprinter::new().with_duration(60);
        assert_eq!(fp.duration_seconds, 60);
    }

    #[test]
    fn test_encode_base64() {
        let fp = Fingerprinter::new();
        let raw = vec![0x01, 0x02, 0x03, 0x04];
        let encoded = fp.encode_base64(&raw);

        // Verify it's valid base64
        let decoded = general_purpose::STANDARD.decode(&encoded).unwrap();
        assert_eq!(decoded, raw);
    }

    #[test]
    fn test_audio_too_short() {
        let fp = Fingerprinter::new();

        // 5 seconds of audio at 44100 Hz (too short, need 10+)
        let short_pcm = vec![0i16; 44100 * 5];

        let result = fp.fingerprint_pcm(&short_pcm, 44100);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), FingerprintError::AudioTooShort));
    }

    #[test]
    fn test_fingerprint_pcm_valid() {
        let fp = Fingerprinter::new();

        // 120 seconds of audio at 44100 Hz
        let pcm = vec![0i16; 44100 * 120];

        let result = fp.fingerprint_pcm(&pcm, 44100);
        assert!(result.is_ok());

        let fingerprint = result.unwrap();
        assert!(!fingerprint.is_empty());
    }
}
