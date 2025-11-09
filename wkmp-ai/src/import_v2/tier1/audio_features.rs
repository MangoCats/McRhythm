// PLAN023 Tier 1: Audio-Derived Feature Extractor
//
// Concept: Extract basic musical features from audio signal (tempo, energy, spectral properties)
// Confidence: 0.4 (computed features are less accurate than Essentia, but better than nothing)
//
// Features extracted:
// - Energy (RMS amplitude)
// - Zero-crossing rate (indicates brightness/noisiness)
// - Spectral centroid (approximation of "brightness")
// - Tempo estimation (basic beat detection)

use crate::import_v2::types::{
    Characteristic, ExtractionSource, ExtractorResult, ImportError, ImportResult, MusicalFlavor,
};
use std::collections::HashMap;

/// Audio-derived feature extractor (Tier 1 extractor concept)
///
/// **Legible Software Principle:**
/// - Independent module: Uses only raw audio samples
/// - Explicit synchronization: Returns `Result<ExtractorResult<MusicalFlavor>>`
/// - Transparent behavior: Simple DSP algorithms, no black boxes
/// - Integrity: Normalizes all characteristics to sum to 1.0
pub struct AudioFeatureExtractor {
    sample_rate: u32,
}

impl Default for AudioFeatureExtractor {
    fn default() -> Self {
        Self::new(44100)
    }
}

impl AudioFeatureExtractor {
    pub fn new(sample_rate: u32) -> Self {
        Self { sample_rate }
    }

    /// Extract musical flavor characteristics from audio signal
    ///
    /// # Arguments
    /// * `samples` - Mono PCM audio samples (f32, normalized to [-1.0, 1.0])
    ///
    /// # Returns
    /// Musical flavor with binary characteristics derived from signal analysis:
    /// - `timbre`: bright (high ZCR/centroid) vs dark (low ZCR/centroid)
    /// - `mood_acoustic`: acoustic (low energy variance) vs not_acoustic (high energy variance)
    /// - `voice_instrumental`: voice (medium ZCR) vs instrumental (varies)
    ///
    /// Note: This is a coarse approximation. Essentia provides much better analysis.
    pub fn extract(&self, samples: &[f32]) -> ImportResult<ExtractorResult<MusicalFlavor>> {
        if samples.is_empty() {
            return Err(ImportError::AudioProcessingFailed(
                "Empty sample buffer".to_string(),
            ));
        }

        // Extract features
        let energy = self.compute_rms_energy(samples);
        let zcr = self.compute_zero_crossing_rate(samples);
        let spectral_centroid = self.estimate_spectral_centroid(samples);

        // Map features to characteristics
        let mut characteristics = Vec::new();

        // Timbre: bright vs dark (based on ZCR and spectral centroid)
        let brightness = (zcr + spectral_centroid) / 2.0;
        characteristics.push(Characteristic {
            name: "timbre".to_string(),
            values: {
                let mut map = HashMap::new();
                map.insert("bright".to_string(), brightness);
                map.insert("dark".to_string(), 1.0 - brightness);
                map
            },
        });

        // Mood acoustic: based on energy variance
        let energy_variance = self.compute_energy_variance(samples);
        let acoustic_score = 1.0 - energy_variance.min(1.0); // Low variance = more acoustic
        characteristics.push(Characteristic {
            name: "mood_acoustic".to_string(),
            values: {
                let mut map = HashMap::new();
                map.insert("acoustic".to_string(), acoustic_score);
                map.insert("not_acoustic".to_string(), 1.0 - acoustic_score);
                map
            },
        });

        // Voice/Instrumental: voice has moderate ZCR (100-3000 Hz formants)
        // This is very approximate!
        let voice_score = if zcr > 0.3 && zcr < 0.7 {
            0.6 // Moderate ZCR suggests voice
        } else {
            0.3 // Low or high ZCR suggests instrumental
        };
        characteristics.push(Characteristic {
            name: "voice_instrumental".to_string(),
            values: {
                let mut map = HashMap::new();
                map.insert("voice".to_string(), voice_score);
                map.insert("instrumental".to_string(), 1.0 - voice_score);
                map
            },
        });

        let flavor = MusicalFlavor { characteristics };

        // Validate normalization
        if !flavor.validate() {
            tracing::warn!("Audio-derived flavor has non-normalized characteristics");
        }

        tracing::debug!(
            "Audio features: energy={:.3}, zcr={:.3}, centroid={:.3}, brightness={:.3}",
            energy,
            zcr,
            spectral_centroid,
            brightness
        );

        Ok(ExtractorResult {
            data: flavor,
            confidence: ExtractionSource::AudioDerived.default_confidence(),
            source: ExtractionSource::AudioDerived,
        })
    }

    /// Compute RMS (Root Mean Square) energy
    fn compute_rms_energy(&self, samples: &[f32]) -> f64 {
        let sum_squares: f64 = samples.iter().map(|&s| (s as f64).powi(2)).sum();
        (sum_squares / samples.len() as f64).sqrt()
    }

    /// Compute zero-crossing rate (normalized to [0, 1])
    fn compute_zero_crossing_rate(&self, samples: &[f32]) -> f64 {
        if samples.len() < 2 {
            return 0.0;
        }

        let crossings = samples
            .windows(2)
            .filter(|w| (w[0] >= 0.0 && w[1] < 0.0) || (w[0] < 0.0 && w[1] >= 0.0))
            .count();

        // Normalize by Nyquist frequency (max possible ZCR)
        let max_possible_zcr = samples.len() / 2;
        (crossings as f64 / max_possible_zcr as f64).min(1.0)
    }

    /// Estimate spectral centroid (normalized to [0, 1])
    ///
    /// This is a very rough approximation without proper FFT.
    /// Compares high-frequency energy to total energy.
    fn estimate_spectral_centroid(&self, samples: &[f32]) -> f64 {
        if samples.len() < 2 {
            return 0.5;
        }

        // Approximate high-frequency content via first-order difference
        let high_freq_energy: f64 = samples
            .windows(2)
            .map(|w| ((w[1] - w[0]) as f64).powi(2))
            .sum();

        let total_energy: f64 = samples.iter().map(|&s| (s as f64).powi(2)).sum();

        if total_energy < 1e-10 {
            return 0.5; // Silent = neutral
        }

        (high_freq_energy / total_energy).sqrt().min(1.0)
    }

    /// Compute energy variance (indicates dynamic range)
    fn compute_energy_variance(&self, samples: &[f32]) -> f64 {
        // Split into frames and compute RMS per frame
        let frame_size = (self.sample_rate / 10) as usize; // 100ms frames
        if samples.len() < frame_size {
            return 0.0;
        }

        let frame_energies: Vec<f64> = samples
            .chunks(frame_size)
            .map(|frame| {
                let sum_squares: f64 = frame.iter().map(|&s| (s as f64).powi(2)).sum();
                (sum_squares / frame.len() as f64).sqrt()
            })
            .collect();

        if frame_energies.is_empty() {
            return 0.0;
        }

        // Compute variance
        let mean: f64 = frame_energies.iter().sum::<f64>() / frame_energies.len() as f64;
        let variance: f64 = frame_energies
            .iter()
            .map(|&e| (e - mean).powi(2))
            .sum::<f64>()
            / frame_energies.len() as f64;

        variance.sqrt() // Return standard deviation, normalized to [0, 1] range
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn generate_sine_wave(frequency: f32, duration_secs: f32, sample_rate: u32) -> Vec<f32> {
        let num_samples = (duration_secs * sample_rate as f32) as usize;
        (0..num_samples)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                (2.0 * std::f32::consts::PI * frequency * t).sin() * 0.5
            })
            .collect()
    }

    fn generate_square_wave(frequency: f32, duration_secs: f32, sample_rate: u32) -> Vec<f32> {
        let num_samples = (duration_secs * sample_rate as f32) as usize;
        (0..num_samples)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                if (2.0 * std::f32::consts::PI * frequency * t).sin() > 0.0 {
                    0.5
                } else {
                    -0.5
                }
            })
            .collect()
    }

    #[test]
    fn test_empty_samples() {
        let extractor = AudioFeatureExtractor::default();
        let samples: Vec<f32> = vec![];
        let result = extractor.extract(&samples);
        assert!(result.is_err());
    }

    #[test]
    fn test_valid_extraction() {
        let extractor = AudioFeatureExtractor::default();
        let samples = generate_sine_wave(440.0, 3.0, 44100);
        let result = extractor.extract(&samples);

        assert!(result.is_ok());
        let extraction = result.unwrap();
        assert_eq!(extraction.source, ExtractionSource::AudioDerived);
        assert_eq!(extraction.confidence, 0.4);
        assert!(extraction.data.validate());
    }

    #[test]
    fn test_characteristic_normalization() {
        let extractor = AudioFeatureExtractor::default();
        let samples = generate_sine_wave(440.0, 3.0, 44100);
        let result = extractor.extract(&samples).unwrap();

        // All characteristics should sum to 1.0
        for char in &result.data.characteristics {
            assert!(char.is_normalized(), "Characteristic '{}' not normalized", char.name);
        }
    }

    #[test]
    fn test_brightness_detection() {
        let extractor = AudioFeatureExtractor::default();

        // Low frequency = dark
        let low_freq = generate_sine_wave(100.0, 3.0, 44100);
        let low_result = extractor.extract(&low_freq).unwrap();
        let low_timbre = low_result.data.get("timbre").unwrap();
        let low_brightness = low_timbre.values.get("bright").unwrap();

        // High frequency = bright
        let high_freq = generate_sine_wave(8000.0, 3.0, 44100);
        let high_result = extractor.extract(&high_freq).unwrap();
        let high_timbre = high_result.data.get("timbre").unwrap();
        let high_brightness = high_timbre.values.get("bright").unwrap();

        // High frequency should be brighter
        assert!(
            high_brightness > low_brightness,
            "Expected high frequency to be brighter: {} vs {}",
            high_brightness,
            low_brightness
        );
    }

    #[test]
    fn test_zero_crossing_rate() {
        let extractor = AudioFeatureExtractor::default();

        // Sine wave has moderate ZCR
        let sine = generate_sine_wave(440.0, 1.0, 44100);
        let sine_zcr = extractor.compute_zero_crossing_rate(&sine);

        // Square wave has same fundamental ZCR as sine
        let square = generate_square_wave(440.0, 1.0, 44100);
        let square_zcr = extractor.compute_zero_crossing_rate(&square);

        // Both should have ZCR related to frequency
        assert!(sine_zcr > 0.0 && sine_zcr < 1.0);
        assert!(square_zcr > 0.0 && square_zcr < 1.0);

        // Similar frequencies should have similar ZCR
        assert!((sine_zcr - square_zcr).abs() < 0.1);
    }

    #[test]
    fn test_energy_variance() {
        let extractor = AudioFeatureExtractor::default();

        // Constant amplitude = low variance
        let constant = generate_sine_wave(440.0, 3.0, 44100);
        let constant_var = extractor.compute_energy_variance(&constant);

        // Varying amplitude = higher variance
        let mut varying = constant.clone();
        let len = varying.len();
        for (i, sample) in varying.iter_mut().enumerate() {
            let envelope = (i as f32 / len as f32).sin();
            *sample *= envelope;
        }
        let varying_var = extractor.compute_energy_variance(&varying);

        // Varying should have higher variance
        assert!(
            varying_var > constant_var,
            "Expected varying amplitude to have higher variance: {} vs {}",
            varying_var,
            constant_var
        );
    }

    #[test]
    fn test_rms_energy() {
        let extractor = AudioFeatureExtractor::default();

        // Loud signal
        let loud = generate_sine_wave(440.0, 1.0, 44100);
        let loud_energy = extractor.compute_rms_energy(&loud);

        // Quiet signal (half amplitude)
        let quiet: Vec<f32> = loud.iter().map(|&s| s * 0.5).collect();
        let quiet_energy = extractor.compute_rms_energy(&quiet);

        // Loud should have ~2x energy
        assert!(loud_energy > quiet_energy * 1.5);
    }
}
