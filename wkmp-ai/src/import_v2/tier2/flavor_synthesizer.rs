// PLAN023 Tier 2: Musical Flavor Synthesizer
//
// Concept: Fuse multiple musical flavor extractions using characteristic-wise weighted averaging
// Synchronization: Accepts Vec<FlavorExtraction> from Tier 1, outputs SynthesizedFlavor
//
// Algorithm (per SPEC_wkmp_ai_recode.md):
// 1. Group extractions by characteristic name
// 2. For each characteristic, compute weighted average of dimension probabilities
// 3. Normalize to ensure sum = 1.0 per category
// 4. Calculate overall confidence and completeness

use crate::import_v2::types::{
    Characteristic, ExtractionSource, FlavorExtraction, ImportError, ImportResult,
    MusicalFlavor, SynthesizedFlavor,
};
use std::collections::HashMap;

/// Musical Flavor Synthesizer (Tier 2 fusion concept)
///
/// **Legible Software Principle:**
/// - Explicit synchronization: Clear contract with Tier 1 extractors
/// - Transparency: Weighted averaging algorithm is explicit
/// - Integrity: Maintains normalization invariant (sum to 1.0)
/// - Independence: No side effects, pure fusion logic
pub struct FlavorSynthesizer {
    /// Minimum confidence threshold for considering a source
    min_confidence: f64,
    /// Normalization tolerance (for validation)
    normalization_tolerance: f64,
}

impl Default for FlavorSynthesizer {
    fn default() -> Self {
        Self {
            min_confidence: 0.1, // Ignore sources with confidence < 0.1
            normalization_tolerance: 0.0001, // per CRITICAL-002
        }
    }
}

impl FlavorSynthesizer {
    /// Synthesize musical flavor from multiple extractions
    ///
    /// # Algorithm
    /// 1. Filter out low-confidence sources (< min_confidence)
    /// 2. Group extractions by characteristic name
    /// 3. For each characteristic:
    ///    a. Collect all dimension → probability mappings from all sources
    ///    b. Compute weighted average: avg = Σ(prob_i * conf_i) / Σ(conf_i)
    ///    c. Normalize to sum to 1.0
    /// 4. Calculate overall confidence (weighted average of source confidences)
    /// 5. Calculate completeness (present_characteristics / 18)
    ///
    /// # Returns
    /// SynthesizedFlavor with fused characteristics, confidence, and provenance
    ///
    /// # Errors
    /// Returns error if no valid sources or normalization fails
    pub fn synthesize(
        &self,
        extractions: Vec<FlavorExtraction>,
    ) -> ImportResult<SynthesizedFlavor> {
        if extractions.is_empty() {
            return Err(ImportError::FusionFailed(
                "No flavor extractions provided".to_string(),
            ));
        }

        // Filter low-confidence sources
        let valid_extractions: Vec<&FlavorExtraction> = extractions
            .iter()
            .filter(|e| e.confidence >= self.min_confidence)
            .collect();

        if valid_extractions.is_empty() {
            return Err(ImportError::FusionFailed(
                "All flavor extractions below confidence threshold".to_string(),
            ));
        }

        tracing::debug!(
            "Synthesizing flavor from {} sources: {:?}",
            valid_extractions.len(),
            valid_extractions.iter().map(|e| e.source).collect::<Vec<_>>()
        );

        // Group characteristics by name across all sources
        let mut char_groups: HashMap<String, Vec<(&Characteristic, f64)>> = HashMap::new();

        for extraction in &valid_extractions {
            for char in &extraction.flavor.characteristics {
                char_groups
                    .entry(char.name.clone())
                    .or_default()
                    .push((char, extraction.confidence));
            }
        }

        // Fuse each characteristic group
        let mut fused_characteristics = Vec::new();

        for (char_name, sources) in char_groups {
            let fused_char = self.fuse_characteristic(&char_name, sources)?;
            fused_characteristics.push(fused_char);
        }

        // Calculate overall confidence (weighted average)
        let total_confidence: f64 = valid_extractions.iter().map(|e| e.confidence).sum();
        let flavor_confidence = total_confidence / valid_extractions.len() as f64;

        // Create synthesized flavor
        let flavor = MusicalFlavor {
            characteristics: fused_characteristics,
        };

        // Validate
        if !flavor.validate() {
            tracing::error!("Synthesized flavor failed validation (non-normalized characteristics)");
            return Err(ImportError::FusionFailed(
                "Flavor normalization validation failed".to_string(),
            ));
        }

        let flavor_completeness = flavor.completeness();
        let sources_used: Vec<ExtractionSource> =
            valid_extractions.iter().map(|e| e.source).collect();

        tracing::info!(
            "Synthesized flavor: {} characteristics ({:.1}% complete), confidence {:.2}",
            flavor.count_present(),
            flavor_completeness * 100.0,
            flavor_confidence
        );

        Ok(SynthesizedFlavor {
            flavor,
            flavor_confidence,
            flavor_completeness,
            sources_used,
        })
    }

    /// Fuse a single characteristic from multiple sources using weighted averaging
    ///
    /// # Algorithm
    /// For each dimension in the characteristic:
    ///   weighted_prob = Σ(prob_i * confidence_i) / Σ(confidence_i)
    ///
    /// Then normalize to ensure sum = 1.0
    fn fuse_characteristic(
        &self,
        name: &str,
        sources: Vec<(&Characteristic, f64)>,
    ) -> ImportResult<Characteristic> {
        // Collect all unique dimensions across sources
        let mut all_dimensions: HashMap<String, Vec<(f64, f64)>> = HashMap::new();

        for (char, confidence) in sources {
            for (dimension, &prob) in &char.values {
                all_dimensions
                    .entry(dimension.clone())
                    .or_default()
                    .push((prob, confidence));
            }
        }

        // Compute weighted average for each dimension
        let mut fused_values = HashMap::new();

        for (dimension, prob_conf_pairs) in all_dimensions {
            let weighted_sum: f64 = prob_conf_pairs
                .iter()
                .map(|(prob, conf)| prob * conf)
                .sum();

            let confidence_sum: f64 = prob_conf_pairs.iter().map(|(_, conf)| conf).sum();

            let weighted_avg = if confidence_sum > 0.0 {
                weighted_sum / confidence_sum
            } else {
                0.0
            };

            fused_values.insert(dimension, weighted_avg);
        }

        // Create characteristic and normalize
        let mut characteristic = Characteristic {
            name: name.to_string(),
            values: fused_values,
        };

        characteristic.normalize();

        // Validate normalization
        if !characteristic.is_normalized() {
            tracing::warn!(
                "Characteristic '{}' normalization outside tolerance: sum = {:.6}",
                name,
                characteristic.values.values().sum::<f64>()
            );
        }

        Ok(characteristic)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_flavor(char_name: &str, values: Vec<(&str, f64)>) -> MusicalFlavor {
        let mut char_values = HashMap::new();
        for (dim, prob) in values {
            char_values.insert(dim.to_string(), prob);
        }

        MusicalFlavor {
            characteristics: vec![Characteristic {
                name: char_name.to_string(),
                values: char_values,
            }],
        }
    }

    #[test]
    fn test_empty_extractions() {
        let synthesizer = FlavorSynthesizer::default();
        let result = synthesizer.synthesize(vec![]);
        assert!(result.is_err());
    }

    #[test]
    fn test_single_source() {
        let synthesizer = FlavorSynthesizer::default();

        let extraction = FlavorExtraction {
            flavor: create_test_flavor("danceability", vec![("danceable", 0.7), ("not_danceable", 0.3)]),
            confidence: 0.9,
            source: ExtractionSource::Essentia,
        };

        let result = synthesizer.synthesize(vec![extraction]);
        assert!(result.is_ok());

        let synthesized = result.unwrap();
        assert_eq!(synthesized.sources_used.len(), 1);
        assert_eq!(synthesized.sources_used[0], ExtractionSource::Essentia);
        assert!((synthesized.flavor_confidence - 0.9).abs() < 0.001);
    }

    #[test]
    fn test_weighted_averaging() {
        let synthesizer = FlavorSynthesizer::default();

        // Source 1: High confidence (0.9), says danceable=0.8
        let extraction1 = FlavorExtraction {
            flavor: create_test_flavor("danceability", vec![("danceable", 0.8), ("not_danceable", 0.2)]),
            confidence: 0.9,
            source: ExtractionSource::Essentia,
        };

        // Source 2: Low confidence (0.3), says danceable=0.2
        let extraction2 = FlavorExtraction {
            flavor: create_test_flavor("danceability", vec![("danceable", 0.2), ("not_danceable", 0.8)]),
            confidence: 0.3,
            source: ExtractionSource::GenreMapping,
        };

        let result = synthesizer.synthesize(vec![extraction1, extraction2]);
        assert!(result.is_ok());

        let synthesized = result.unwrap();
        let danceability = synthesized.flavor.get("danceability").unwrap();
        let danceable = danceability.values.get("danceable").unwrap();

        // Expected: (0.8 * 0.9 + 0.2 * 0.3) / (0.9 + 0.3) = (0.72 + 0.06) / 1.2 = 0.65
        assert!((danceable - 0.65).abs() < 0.01, "Expected ~0.65, got {}", danceable);
    }

    #[test]
    fn test_normalization_maintained() {
        let synthesizer = FlavorSynthesizer::default();

        let extraction = FlavorExtraction {
            flavor: create_test_flavor("danceability", vec![("danceable", 0.6), ("not_danceable", 0.4)]),
            confidence: 0.8,
            source: ExtractionSource::Essentia,
        };

        let result = synthesizer.synthesize(vec![extraction]).unwrap();

        // All characteristics should be normalized
        for char in &result.flavor.characteristics {
            assert!(char.is_normalized(), "Characteristic '{}' not normalized", char.name);
        }
    }

    #[test]
    fn test_multiple_characteristics() {
        let synthesizer = FlavorSynthesizer::default();

        let mut flavor = MusicalFlavor {
            characteristics: vec![
                Characteristic {
                    name: "danceability".to_string(),
                    values: {
                        let mut map = HashMap::new();
                        map.insert("danceable".to_string(), 0.7);
                        map.insert("not_danceable".to_string(), 0.3);
                        map
                    },
                },
                Characteristic {
                    name: "mood_happy".to_string(),
                    values: {
                        let mut map = HashMap::new();
                        map.insert("happy".to_string(), 0.6);
                        map.insert("not_happy".to_string(), 0.4);
                        map
                    },
                },
            ],
        };

        let extraction = FlavorExtraction {
            flavor,
            confidence: 0.9,
            source: ExtractionSource::Essentia,
        };

        let result = synthesizer.synthesize(vec![extraction]).unwrap();
        assert_eq!(result.flavor.characteristics.len(), 2);
    }

    #[test]
    fn test_completeness_calculation() {
        let synthesizer = FlavorSynthesizer::default();

        // Create flavor with 9 characteristics (50% of expected 18)
        let characteristics: Vec<Characteristic> = (0..9)
            .map(|i| Characteristic {
                name: format!("char_{}", i),
                values: {
                    let mut map = HashMap::new();
                    map.insert("value_a".to_string(), 0.5);
                    map.insert("value_b".to_string(), 0.5);
                    map
                },
            })
            .collect();

        let extraction = FlavorExtraction {
            flavor: MusicalFlavor { characteristics },
            confidence: 0.9,
            source: ExtractionSource::Essentia,
        };

        let result = synthesizer.synthesize(vec![extraction]).unwrap();

        // Completeness should be 9/18 = 0.5
        assert!((result.flavor_completeness - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_low_confidence_filtering() {
        let synthesizer = FlavorSynthesizer::default();

        // Very low confidence source (below 0.1 threshold)
        let extraction = FlavorExtraction {
            flavor: create_test_flavor("danceability", vec![("danceable", 0.5), ("not_danceable", 0.5)]),
            confidence: 0.05,
            source: ExtractionSource::GenreMapping,
        };

        let result = synthesizer.synthesize(vec![extraction]);
        assert!(result.is_err()); // Should fail due to all sources filtered out
    }

    #[test]
    fn test_missing_dimensions_handled() {
        let synthesizer = FlavorSynthesizer::default();

        // Source 1 has dimensions A and B
        let extraction1 = FlavorExtraction {
            flavor: create_test_flavor("genre_electronic", vec![
                ("ambient", 0.6),
                ("house", 0.4),
            ]),
            confidence: 0.8,
            source: ExtractionSource::Essentia,
        };

        // Source 2 has dimensions B and C (B overlaps, C is new)
        let extraction2 = FlavorExtraction {
            flavor: create_test_flavor("genre_electronic", vec![
                ("house", 0.3),
                ("techno", 0.7),
            ]),
            confidence: 0.6,
            source: ExtractionSource::GenreMapping,
        };

        let result = synthesizer.synthesize(vec![extraction1, extraction2]);
        assert!(result.is_ok());

        let synthesized = result.unwrap();
        let genre = synthesized.flavor.get("genre_electronic").unwrap();

        // Should have all three dimensions: ambient, house, techno
        assert!(genre.values.contains_key("ambient"));
        assert!(genre.values.contains_key("house"));
        assert!(genre.values.contains_key("techno"));

        // Should be normalized
        assert!(genre.is_normalized());
    }

    #[test]
    fn test_overall_confidence_calculation() {
        let synthesizer = FlavorSynthesizer::default();

        let extraction1 = FlavorExtraction {
            flavor: create_test_flavor("danceability", vec![("danceable", 0.5), ("not_danceable", 0.5)]),
            confidence: 0.9,
            source: ExtractionSource::Essentia,
        };

        let extraction2 = FlavorExtraction {
            flavor: create_test_flavor("mood_happy", vec![("happy", 0.5), ("not_happy", 0.5)]),
            confidence: 0.7,
            source: ExtractionSource::AudioDerived,
        };

        let result = synthesizer.synthesize(vec![extraction1, extraction2]).unwrap();

        // Overall confidence should be average: (0.9 + 0.7) / 2 = 0.8
        assert!((result.flavor_confidence - 0.8).abs() < 0.01);
    }
}
