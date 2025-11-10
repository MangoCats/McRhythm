//! AudioDerived Extractor (Tier 1)
//!
//! Extracts musical flavor characteristics using custom DSP algorithms.
//! Performs direct signal processing on audio samples to compute features like
//! energy, spectral characteristics, and temporal properties.
//!
//! # Implementation
//! - TASK-010: AudioDerived Extractor (PLAN024)
//! - Confidence: 0.65 (algorithmic analysis, simpler than Essentia)
//!
//! # Architecture
//! Implements `SourceExtractor` trait for integration with parallel extraction pipeline.
//! Analyzes audio samples directly without external dependencies.
//!
//! # Extracted Features
//! - Energy (RMS energy level)
//! - Spectral Centroid (brightness)
//! - Spectral Rolloff (frequency distribution)
//! - Spectral Flatness (tonality vs. noise)
//! - Zero-Crossing Rate (noisiness/percussion)
//! - Dynamic Range (loudness variation)

use crate::types::{
    ExtractionError, ExtractionResult, FlavorExtraction, PassageContext, SourceExtractor,
};
use async_trait::async_trait;
use std::collections::HashMap;
use tracing::debug;

/// AudioDerived Extractor
///
/// Extracts musical flavor characteristics using custom DSP algorithms.
/// Analyzes audio samples to compute energy, spectral, and temporal features.
///
/// # Confidence
/// Base confidence: 0.65
/// - Algorithmic analysis (not authoritative)
/// - Simpler algorithms than Essentia (0.7)
/// - More reliable than genre mapping (0.5)
/// - Provides complementary features to other extractors
///
/// # Requirements
/// - Audio samples in f32 format (mono or stereo)
/// - Sample rate (for frequency calculations)
/// - Sufficient audio data (minimum ~1 second recommended)
///
/// # Example
/// ```rust,ignore
/// use wkmp_ai::extractors::audio_derived_extractor::AudioDerivedExtractor;
/// use wkmp_ai::types::{SourceExtractor, PassageContext};
///
/// let extractor = AudioDerivedExtractor::new();
/// let result = extractor.extract(&passage_ctx).await?;
///
/// if let Some(flavor) = result.musical_flavor {
///     println!("Energy: {}", flavor.characteristics.get("energy").unwrap_or(&0.0));
/// }
/// ```
pub struct AudioDerivedExtractor {
    /// Base confidence for derived features
    base_confidence: f32,
}

impl AudioDerivedExtractor {
    /// Create new AudioDerived extractor with default confidence (0.65)
    pub fn new() -> Self {
        Self {
            base_confidence: 0.65,
        }
    }

    /// Extract features from audio samples
    ///
    /// # Arguments
    /// * `samples` - Audio samples in f32 format (interleaved if stereo)
    /// * `sample_rate` - Sample rate in Hz
    /// * `num_channels` - Number of channels (1=mono, 2=stereo)
    ///
    /// # Returns
    /// Musical flavor characteristics extracted from audio
    ///
    /// # Errors
    /// Returns error if:
    /// - No audio samples provided
    /// - Invalid parameters
    fn extract_features(
        &self,
        samples: &[f32],
        sample_rate: u32,
        num_channels: u8,
    ) -> Result<FlavorExtraction, ExtractionError> {
        if samples.is_empty() {
            return Err(ExtractionError::Internal(
                "No audio samples provided".to_string(),
            ));
        }

        debug!(
            sample_count = samples.len(),
            sample_rate = sample_rate,
            channels = num_channels,
            "Extracting audio-derived features"
        );

        // Convert to mono if stereo (average channels)
        let mono_samples = if num_channels == 2 {
            samples
                .chunks_exact(2)
                .map(|chunk| (chunk[0] + chunk[1]) / 2.0)
                .collect::<Vec<f32>>()
        } else {
            samples.to_vec()
        };

        let mut characteristics = HashMap::new();

        // 1. Energy (RMS)
        let energy = self.compute_rms_energy(&mono_samples);
        characteristics.insert("energy".to_string(), energy);

        // 2. Dynamic Range
        let dynamic_range = self.compute_dynamic_range(&mono_samples);
        characteristics.insert("dynamic_range".to_string(), dynamic_range);

        // 3. Zero-Crossing Rate (normalized)
        let zcr = self.compute_zero_crossing_rate(&mono_samples);
        characteristics.insert("zero_crossing_rate".to_string(), zcr);

        // 4. Spectral Centroid (requires FFT - simplified version)
        let spectral_centroid = self.compute_spectral_centroid(&mono_samples, sample_rate);
        characteristics.insert("spectral_centroid".to_string(), spectral_centroid);

        // 5. Spectral Flatness
        let spectral_flatness = self.compute_spectral_flatness(&mono_samples);
        characteristics.insert("spectral_flatness".to_string(), spectral_flatness);

        // 6. Peak Level
        let peak_level = self.compute_peak_level(&mono_samples);
        characteristics.insert("peak_level".to_string(), peak_level);

        debug!(
            feature_count = characteristics.len(),
            energy = energy,
            dynamic_range = dynamic_range,
            "Audio-derived feature extraction complete"
        );

        Ok(FlavorExtraction {
            characteristics,
            confidence: self.base_confidence,
            source: "AudioDerived".to_string(),
        })
    }

    /// Compute RMS (Root Mean Square) energy
    ///
    /// Returns normalized energy level (0.0-1.0)
    fn compute_rms_energy(&self, samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }

        let sum_squares: f32 = samples.iter().map(|&s| s * s).sum();
        let rms = (sum_squares / samples.len() as f32).sqrt();

        // Normalize to 0-1 range (assuming typical RMS range 0.0-0.5)
        (rms * 2.0).clamp(0.0, 1.0)
    }

    /// Compute dynamic range
    ///
    /// Returns normalized dynamic range (0.0-1.0)
    /// Based on difference between peak and RMS levels
    fn compute_dynamic_range(&self, samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }

        let peak = samples.iter().map(|&s| s.abs()).fold(0.0f32, f32::max);
        let sum_squares: f32 = samples.iter().map(|&s| s * s).sum();
        let rms = (sum_squares / samples.len() as f32).sqrt();

        if peak == 0.0 {
            return 0.0;
        }

        // Dynamic range = peak / RMS (crest factor)
        // Normalize: typical range 2.0-20.0 → 0.0-1.0
        let crest_factor = peak / rms.max(0.001);
        ((crest_factor - 2.0) / 18.0).clamp(0.0, 1.0)
    }

    /// Compute zero-crossing rate
    ///
    /// Returns normalized ZCR (0.0-1.0)
    /// High ZCR indicates noisy/percussive content
    fn compute_zero_crossing_rate(&self, samples: &[f32]) -> f32 {
        if samples.len() < 2 {
            return 0.0;
        }

        let mut crossings = 0;
        for i in 1..samples.len() {
            if (samples[i - 1] >= 0.0 && samples[i] < 0.0)
                || (samples[i - 1] < 0.0 && samples[i] >= 0.0)
            {
                crossings += 1;
            }
        }

        let zcr = crossings as f32 / (samples.len() - 1) as f32;

        // Normalize: typical range 0.0-0.5 → 0.0-1.0
        (zcr * 2.0).clamp(0.0, 1.0)
    }

    /// Compute spectral centroid (simplified)
    ///
    /// Returns normalized spectral centroid (0.0-1.0)
    /// Indicates "brightness" of the sound
    ///
    /// Note: This is a simplified version without full FFT.
    /// Uses high-frequency energy approximation.
    fn compute_spectral_centroid(&self, samples: &[f32], _sample_rate: u32) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }

        // Simplified approach: measure high-frequency content
        // using difference between samples (approximates derivative)
        let mut high_freq_energy = 0.0f32;
        let mut total_energy = 0.0f32;

        for i in 1..samples.len() {
            let diff = samples[i] - samples[i - 1];
            high_freq_energy += diff * diff;
            total_energy += samples[i] * samples[i];
        }

        if total_energy == 0.0 {
            return 0.0;
        }

        // Ratio of high-frequency to total energy
        let brightness = high_freq_energy / total_energy;

        // Normalize to 0-1 range
        brightness.clamp(0.0, 1.0)
    }

    /// Compute spectral flatness
    ///
    /// Returns normalized spectral flatness (0.0-1.0)
    /// 0.0 = tonal (harmonic), 1.0 = noise-like
    ///
    /// Note: Simplified version using statistical measures
    fn compute_spectral_flatness(&self, samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }

        // Simplified: measure variance vs. mean
        // High variance relative to mean = noise-like
        // Low variance = tonal

        let mean: f32 = samples.iter().map(|&s| s.abs()).sum::<f32>() / samples.len() as f32;

        if mean == 0.0 {
            return 0.0;
        }

        let variance: f32 = samples
            .iter()
            .map(|&s| {
                let diff = s.abs() - mean;
                diff * diff
            })
            .sum::<f32>()
            / samples.len() as f32;

        let std_dev = variance.sqrt();

        // Coefficient of variation: std_dev / mean
        // Normalize to 0-1 range (typical range 0-3)
        (std_dev / mean / 3.0).clamp(0.0, 1.0)
    }

    /// Compute peak level
    ///
    /// Returns normalized peak level (0.0-1.0)
    fn compute_peak_level(&self, samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }

        samples
            .iter()
            .map(|&s| s.abs())
            .fold(0.0f32, f32::max)
            .clamp(0.0, 1.0)
    }
}

impl Default for AudioDerivedExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SourceExtractor for AudioDerivedExtractor {
    fn name(&self) -> &'static str {
        "AudioDerived"
    }

    fn base_confidence(&self) -> f32 {
        self.base_confidence
    }

    async fn extract(&self, ctx: &PassageContext) -> Result<ExtractionResult, ExtractionError> {
        debug!(
            passage_id = %ctx.passage_id,
            file_path = ?ctx.file_path,
            "Extracting audio-derived features"
        );

        // Check if audio samples are available
        let Some(ref samples) = ctx.audio_samples else {
            return Err(ExtractionError::Internal(
                "No audio samples available".to_string(),
            ));
        };

        let Some(sample_rate) = ctx.sample_rate else {
            return Err(ExtractionError::Internal(
                "Sample rate not specified".to_string(),
            ));
        };

        let Some(num_channels) = ctx.num_channels else {
            return Err(ExtractionError::Internal(
                "Number of channels not specified".to_string(),
            ));
        };

        let flavor = self.extract_features(samples, sample_rate, num_channels)?;

        debug!(
            passage_id = %ctx.passage_id,
            feature_count = flavor.characteristics.len(),
            "AudioDerived extraction complete"
        );

        Ok(ExtractionResult {
            metadata: None,
            identity: None,
            musical_flavor: Some(flavor),
        })
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use uuid::Uuid;

    #[test]
    fn test_extractor_name() {
        let extractor = AudioDerivedExtractor::new();
        assert_eq!(extractor.name(), "AudioDerived");
    }

    #[test]
    fn test_default_confidence() {
        let extractor = AudioDerivedExtractor::new();
        assert_eq!(extractor.base_confidence(), 0.65);
    }

    #[test]
    fn test_default_trait() {
        let extractor = AudioDerivedExtractor::default();
        assert_eq!(extractor.base_confidence(), 0.65);
    }

    #[test]
    fn test_compute_rms_energy_silence() {
        let extractor = AudioDerivedExtractor::new();
        let samples = vec![0.0f32; 1000];
        let energy = extractor.compute_rms_energy(&samples);
        assert_eq!(energy, 0.0, "Silence should have zero energy");
    }

    #[test]
    fn test_compute_rms_energy_signal() {
        let extractor = AudioDerivedExtractor::new();
        // Generate 440 Hz sine wave
        let samples: Vec<f32> = (0..44100)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 44100.0).sin() * 0.5)
            .collect();
        let energy = extractor.compute_rms_energy(&samples);
        assert!(energy > 0.5 && energy < 0.9, "Sine wave should have moderate energy");
    }

    #[test]
    fn test_compute_zero_crossing_rate() {
        let extractor = AudioDerivedExtractor::new();

        // Low-frequency signal (few crossings)
        let low_freq: Vec<f32> = (0..1000)
            .map(|i| (2.0 * std::f32::consts::PI * 10.0 * i as f32 / 1000.0).sin())
            .collect();
        let low_zcr = extractor.compute_zero_crossing_rate(&low_freq);

        // High-frequency signal (many crossings)
        let high_freq: Vec<f32> = (0..1000)
            .map(|i| (2.0 * std::f32::consts::PI * 100.0 * i as f32 / 1000.0).sin())
            .collect();
        let high_zcr = extractor.compute_zero_crossing_rate(&high_freq);

        assert!(high_zcr > low_zcr, "High frequency should have higher ZCR");
    }

    #[test]
    fn test_compute_peak_level() {
        let extractor = AudioDerivedExtractor::new();

        let samples = vec![0.1, -0.5, 0.3, -0.8, 0.2];
        let peak = extractor.compute_peak_level(&samples);
        assert_eq!(peak, 0.8, "Peak should be 0.8");
    }

    #[test]
    fn test_compute_dynamic_range() {
        let extractor = AudioDerivedExtractor::new();

        // High dynamic range (quiet with peaks)
        let high_dynamic: Vec<f32> = vec![0.01, 0.01, 0.9, 0.01, 0.01];
        let high_dr = extractor.compute_dynamic_range(&high_dynamic);

        // Low dynamic range (consistent level)
        let low_dynamic: Vec<f32> = vec![0.5, 0.5, 0.5, 0.5, 0.5];
        let low_dr = extractor.compute_dynamic_range(&low_dynamic);

        assert!(high_dr > low_dr, "High dynamic range should be greater");
    }

    #[tokio::test]
    async fn test_extract_missing_samples() {
        let extractor = AudioDerivedExtractor::new();
        let ctx = PassageContext {
            passage_id: Uuid::new_v4(),
            file_id: Uuid::new_v4(),
            file_path: PathBuf::from("/test/audio.mp3"),
            start_time_ticks: 0,
            end_time_ticks: 1000000,
            audio_samples: None, // No samples
            sample_rate: Some(44100),
            num_channels: Some(2),
            import_session_id: Uuid::new_v4(),
        };

        let result = extractor.extract(&ctx).await;
        assert!(result.is_err(), "Should fail when no audio samples provided");
    }

    #[tokio::test]
    async fn test_extract_with_mono_samples() {
        let extractor = AudioDerivedExtractor::new();

        // Generate 1 second of 440 Hz sine wave
        let samples: Vec<f32> = (0..44100)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 44100.0).sin() * 0.5)
            .collect();

        let ctx = PassageContext {
            passage_id: Uuid::new_v4(),
            file_id: Uuid::new_v4(),
            file_path: PathBuf::from("/test/audio.mp3"),
            start_time_ticks: 0,
            end_time_ticks: 28224000, // 1 second
            audio_samples: Some(samples),
            sample_rate: Some(44100),
            num_channels: Some(1),
            import_session_id: Uuid::new_v4(),
        };

        let result = extractor.extract(&ctx).await;
        assert!(result.is_ok(), "Should extract features successfully");

        let extraction = result.unwrap();
        assert!(extraction.musical_flavor.is_some());

        let flavor = extraction.musical_flavor.unwrap();
        assert_eq!(flavor.source, "AudioDerived");
        assert_eq!(flavor.confidence, 0.65);
        assert!(flavor.characteristics.contains_key("energy"));
        assert!(flavor.characteristics.contains_key("dynamic_range"));
        assert!(flavor.characteristics.contains_key("zero_crossing_rate"));
    }
}
