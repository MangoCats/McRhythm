// Audio-Derived Feature Extractor
//
// PLAN023: REQ-AI-041 - Extract basic audio features (tempo, key, energy)
// Confidence: 0.6-0.8 (simple audio analysis)

use crate::fusion::extractors::Extractor;
use crate::fusion::{ExtractionResult, FlavorExtraction, MusicalFlavor, Confidence};
use anyhow::Result;
use async_trait::async_trait;
use std::path::Path;

pub struct AudioDerivedExtractor;

impl Default for AudioDerivedExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioDerivedExtractor {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Extractor for AudioDerivedExtractor {
    fn source_id(&self) -> &'static str {
        "AudioDerived"
    }

    async fn extract(
        &self,
        file_path: &Path,
        start_seconds: f64,
        end_seconds: f64,
    ) -> Result<ExtractionResult> {
        use crate::fusion::extractors::audio_extractor::extract_passage_audio;
        
        use tracing::debug;

        // Extract passage audio to temporary WAV
        let temp_audio = extract_passage_audio(file_path, start_seconds, end_seconds).await?;

        // Read WAV samples (temp_audio handle keeps file alive)
        let mut reader = hound::WavReader::open(temp_audio.path())?;
        let spec = reader.spec();
        let samples: Vec<f32> = reader
            .samples::<i16>()
            .map(|s| s.map(|v| v as f32 / i16::MAX as f32))
            .collect::<Result<Vec<_>, _>>()?;

        debug!(
            "Analyzing {} samples at {}Hz for audio-derived features",
            samples.len(),
            spec.sample_rate
        );

        // Calculate basic audio features
        let rms_energy = calculate_rms_energy(&samples);
        let zcr = calculate_zero_crossing_rate(&samples);
        let spectral_centroid = calculate_spectral_centroid(&samples, spec.sample_rate);

        // Map features to musical flavor characteristics
        let mut characteristics = MusicalFlavor::new();

        // RMS Energy → Acoustic vs Electronic
        // High energy suggests electronic/loud, low energy suggests acoustic/soft
        let electronic_score = (rms_energy * 10.0).min(1.0) as f64;
        let acoustic_score = 1.0 - electronic_score;
        characteristics.insert("mood_acoustic.acoustic".to_string(), acoustic_score);
        characteristics.insert("mood_acoustic.not_acoustic".to_string(), electronic_score);

        // Zero-Crossing Rate → Timbre (bright vs dark)
        // High ZCR = bright/trebly, Low ZCR = dark/bassy
        let bright_score = (zcr * 2.0).min(1.0) as f64;
        let dark_score = 1.0 - bright_score;
        characteristics.insert("timbre.bright".to_string(), bright_score);
        characteristics.insert("timbre.dark".to_string(), dark_score);

        // Spectral Centroid → Additional timbre cues
        // Normalized centroid (0.0-1.0 range)
        let centroid_norm = (spectral_centroid / (spec.sample_rate as f32 / 2.0)).min(1.0) as f64;
        characteristics.insert("timbre.centroid".to_string(), centroid_norm);

        // Calculate confidence based on sample count
        // More samples = higher confidence (up to 0.8 max)
        let duration = samples.len() as f64 / spec.sample_rate as f64;
        let confidence = (0.6 + (duration / 60.0) * 0.2).min(0.8);

        debug!(
            "Audio features: RMS={:.3}, ZCR={:.3}, Centroid={:.1}Hz, confidence={:.2}",
            rms_energy, zcr, spectral_centroid, confidence
        );

        // Cleanup temporary file
        let _ = std::fs::remove_file(&temp_audio);

        Ok(ExtractionResult {
            source: self.source_id().to_string(),
            confidence,
            timestamp: chrono::Utc::now().timestamp(),
            metadata: None,
            flavor: Some(FlavorExtraction {
                characteristics,
                characteristic_confidence: None,
            }),
            identity: None,
        })
    }

    fn confidence_range(&self) -> (Confidence, Confidence) {
        (0.6, 0.8) // Simple analysis: moderate confidence
    }
}

/// Calculate RMS (Root Mean Square) energy
fn calculate_rms_energy(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }

    let sum_squares: f32 = samples.iter().map(|s| s * s).sum();
    (sum_squares / samples.len() as f32).sqrt()
}

/// Calculate Zero-Crossing Rate
fn calculate_zero_crossing_rate(samples: &[f32]) -> f32 {
    if samples.len() < 2 {
        return 0.0;
    }

    let mut crossings = 0;
    for i in 1..samples.len() {
        if (samples[i - 1] >= 0.0 && samples[i] < 0.0) || (samples[i - 1] < 0.0 && samples[i] >= 0.0) {
            crossings += 1;
        }
    }

    crossings as f32 / samples.len() as f32
}

/// Calculate Spectral Centroid (simplified)
fn calculate_spectral_centroid(samples: &[f32], sample_rate: u32) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }

    // Simple approximation: use zero-crossing rate as proxy
    // True spectral centroid requires FFT, which is beyond scope of "basic" features
    // Higher ZCR correlates with higher spectral centroid
    let zcr = calculate_zero_crossing_rate(samples);
    zcr * (sample_rate as f32 / 2.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_id() {
        let extractor = AudioDerivedExtractor::new();
        assert_eq!(extractor.source_id(), "AudioDerived");
    }

    #[test]
    fn test_confidence_range() {
        let extractor = AudioDerivedExtractor::new();
        assert_eq!(extractor.confidence_range(), (0.6, 0.8));
    }

    #[test]
    fn test_rms_energy_silence() {
        let samples = vec![0.0; 1000];
        let rms = calculate_rms_energy(&samples);
        assert_eq!(rms, 0.0);
    }

    #[test]
    fn test_rms_energy_full_scale() {
        let samples = vec![1.0; 1000];
        let rms = calculate_rms_energy(&samples);
        assert_eq!(rms, 1.0);
    }

    #[test]
    fn test_zero_crossing_rate() {
        // Alternating signal: maximum zero-crossing rate
        let samples: Vec<f32> = (0..1000).map(|i| if i % 2 == 0 { 1.0 } else { -1.0 }).collect();
        let zcr = calculate_zero_crossing_rate(&samples);
        assert!((zcr - 0.999).abs() < 0.01, "ZCR should be ~1.0 for alternating signal");
    }

    #[test]
    fn test_zero_crossing_rate_dc() {
        // DC signal: no zero crossings
        let samples = vec![0.5; 1000];
        let zcr = calculate_zero_crossing_rate(&samples);
        assert_eq!(zcr, 0.0);
    }

    #[test]
    fn test_spectral_centroid() {
        let samples: Vec<f32> = (0..1000).map(|i| if i % 2 == 0 { 1.0 } else { -1.0 }).collect();
        let centroid = calculate_spectral_centroid(&samples, 44100);
        assert!(centroid > 0.0, "Spectral centroid should be positive for non-silent signal");
    }
}
