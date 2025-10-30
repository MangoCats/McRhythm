//! Audio fingerprinting service using Chromaprint
//!
//! **[AIA-COMP-010]** Chromaprint fingerprint generation (Decision 3: static linking)
//! Uses chromaprint-sys-next for official Chromaprint algorithm without external binary

use base64::{engine::general_purpose, Engine as _};
use std::path::Path;
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

        // Generate Chromaprint fingerprint
        // Note: chromaprint-sys-next provides the official Chromaprint C library
        // For now, return placeholder until we integrate the actual library
        // TODO: Integrate chromaprint-sys-next properly
        let raw_fingerprint = self.generate_chromaprint(pcm_slice, sample_rate)?;

        // Encode as base64
        Ok(self.encode_base64(&raw_fingerprint))
    }

    /// Generate raw Chromaprint fingerprint
    fn generate_chromaprint(
        &self,
        _pcm_data: &[i16],
        _sample_rate: u32,
    ) -> Result<Vec<u8>, FingerprintError> {
        // Placeholder implementation
        // TODO: Integrate chromaprint-sys-next
        // This will use the chromaprint_calculate function from chromaprint-sys-next

        // For now, return a dummy fingerprint for testing
        Ok(vec![0x01, 0x02, 0x03, 0x04, 0xAB, 0xCD, 0xEF])
    }

    /// Encode fingerprint as base64
    ///
    /// **[TC-COMP-006]** Base64 encoding test
    pub fn encode_base64(&self, raw_fingerprint: &[u8]) -> String {
        general_purpose::STANDARD.encode(raw_fingerprint)
    }

    /// Decode audio file to mono PCM i16
    fn decode_audio(&self, audio_path: &Path) -> Result<(Vec<i16>, u32), FingerprintError> {
        // Use symphonia to decode audio
        // For now, return placeholder
        // TODO: Implement full symphonia decoding

        // Return dummy PCM data for testing (44.1kHz, 120 seconds)
        let sample_rate = 44100u32;
        let duration_samples = sample_rate as usize * 120;

        // Generate silence (will be replaced with real decoding)
        let pcm_data = vec![0i16; duration_samples];

        Ok((pcm_data, sample_rate))
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
