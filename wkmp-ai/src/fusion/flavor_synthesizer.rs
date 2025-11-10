//! Flavor Synthesizer (Tier 2 Fuser)
//!
//! Performs weighted fusion of musical flavor characteristics from multiple extractors.
//! Merges flavor vectors using confidence-weighted averaging.
//!
//! # Implementation
//! - TASK-014: Flavor Synthesizer (PLAN024)
//! - Fusion strategy: Confidence-weighted averaging per characteristic
//!
//! # Architecture
//! Implements `Fusion` trait for integration with 3-tier architecture.
//! Accepts Vec<FlavorExtraction> and produces FusedFlavor with merged characteristics.
//!
//! # Weighted Fusion
//! For each characteristic present in any source:
//! - Compute weighted average: Σ(value_i * confidence_i) / Σ(confidence_i)
//! - Track per-characteristic confidence (average of contributing source confidences)
//! - Record source blend (which extractors contributed which characteristics)
//! - Compute completeness score (present characteristics / expected characteristics)
//!
//! # Example
//! ```rust,ignore
//! use wkmp_ai::fusion::FlavorSynthesizer;
//! use wkmp_ai::types::{Fusion, FlavorExtraction};
//!
//! let synthesizer = FlavorSynthesizer::new();
//! let flavors = vec![
//!     FlavorExtraction {
//!         characteristics: [("danceability", 0.8), ("energy", 0.7)].into(),
//!         confidence: 0.9,
//!         source: "Essentia".into(),
//!     },
//!     FlavorExtraction {
//!         characteristics: [("danceability", 0.7), ("valence", 0.6)].into(),
//!         confidence: 0.6,
//!         source: "AudioDerived".into(),
//!     },
//! ];
//!
//! let fused = synthesizer.fuse(flavors).await?;
//! // danceability = (0.8*0.9 + 0.7*0.6) / (0.9 + 0.6) = 0.76
//! ```

use crate::types::{FlavorExtraction, Fusion, FusionError, FusionResult, FusedFlavor};
use async_trait::async_trait;
use std::collections::HashMap;
use tracing::debug;

/// Flavor Synthesizer
///
/// Performs weighted fusion of musical flavor characteristics from multiple extraction sources.
/// Uses confidence-weighted averaging to merge flavor vectors.
///
/// # Fusion Strategy
/// For each characteristic (danceability, energy, valence, etc.):
/// 1. Collect all values from sources that provide this characteristic
/// 2. Compute weighted average: Σ(value_i * confidence_i) / Σ(confidence_i)
/// 3. Track per-characteristic confidence (average of contributing confidences)
/// 4. Record source blend (source → overall contribution weight)
///
/// # Completeness Scoring
/// Completeness = (present characteristics count) / (expected characteristics count)
/// Expected characteristics: 13 standard AcousticBrainz high-level features
///
/// # Example Calculation
/// Given two sources for "danceability":
/// - Source A: value=0.8, confidence=0.9
/// - Source B: value=0.7, confidence=0.6
///
/// Weighted average = (0.8 * 0.9 + 0.7 * 0.6) / (0.9 + 0.6)
///                  = (0.72 + 0.42) / 1.5
///                  = 1.14 / 1.5
///                  = 0.76
///
/// Characteristic confidence = (0.9 + 0.6) / 2 = 0.75
pub struct FlavorSynthesizer {
    /// Expected number of characteristics for completeness calculation
    /// (13 standard AcousticBrainz high-level features)
    expected_characteristic_count: usize,
}

impl FlavorSynthesizer {
    /// Create new Flavor Synthesizer with default settings
    pub fn new() -> Self {
        Self {
            expected_characteristic_count: 13, // AcousticBrainz high-level feature count
        }
    }

    /// Create Flavor Synthesizer with custom expected characteristic count
    pub fn with_expected_count(expected_count: usize) -> Self {
        Self {
            expected_characteristic_count: expected_count,
        }
    }

    /// Fuse flavor characteristics from multiple sources
    fn fuse_flavors(
        &self,
        flavor_list: Vec<FlavorExtraction>,
    ) -> Result<FusedFlavor, FusionError> {
        if flavor_list.is_empty() {
            return Ok(FusedFlavor {
                characteristics: HashMap::new(),
                confidence_map: HashMap::new(),
                source_blend: vec![],
                completeness: 0.0,
            });
        }

        debug!(
            flavor_count = flavor_list.len(),
            "Starting flavor synthesis"
        );

        // Collect all unique characteristic names
        let mut all_characteristics: Vec<String> = flavor_list
            .iter()
            .flat_map(|f| f.characteristics.keys().cloned())
            .collect();
        all_characteristics.sort();
        all_characteristics.dedup();

        debug!(
            characteristic_count = all_characteristics.len(),
            "Found unique characteristics"
        );

        // Fuse each characteristic independently using confidence-weighted averaging
        let mut fused_characteristics = HashMap::new();
        let mut confidence_map = HashMap::new();

        for char_name in &all_characteristics {
            let (fused_value, char_confidence) =
                self.fuse_characteristic(char_name, &flavor_list);

            if let Some(value) = fused_value {
                fused_characteristics.insert(char_name.clone(), value);
                confidence_map.insert(char_name.clone(), char_confidence);
            }
        }

        // Compute source blend (overall contribution of each source)
        let source_blend = self.compute_source_blend(&flavor_list);

        // Compute completeness (present / expected)
        let completeness = if self.expected_characteristic_count > 0 {
            fused_characteristics.len() as f32 / self.expected_characteristic_count as f32
        } else {
            0.0
        }
        .min(1.0); // Cap at 1.0 even if more than expected

        debug!(
            fused_count = fused_characteristics.len(),
            completeness = completeness,
            "Flavor synthesis complete"
        );

        Ok(FusedFlavor {
            characteristics: fused_characteristics,
            confidence_map,
            source_blend,
            completeness,
        })
    }

    /// Fuse a single characteristic using confidence-weighted averaging
    ///
    /// Returns: (fused_value, characteristic_confidence)
    ///
    /// Formula:
    /// - fused_value = Σ(value_i * confidence_i) / Σ(confidence_i)
    /// - characteristic_confidence = Σ(confidence_i) / N
    fn fuse_characteristic(
        &self,
        char_name: &str,
        flavors: &[FlavorExtraction],
    ) -> (Option<f32>, f32) {
        // Collect all sources that provide this characteristic
        let sources: Vec<(f32, f32)> = flavors
            .iter()
            .filter_map(|flavor| {
                flavor
                    .characteristics
                    .get(char_name)
                    .map(|&value| (value, flavor.confidence))
            })
            .collect();

        if sources.is_empty() {
            return (None, 0.0);
        }

        // Confidence-weighted average
        let weighted_sum: f32 = sources.iter().map(|(value, conf)| value * conf).sum();
        let confidence_sum: f32 = sources.iter().map(|(_, conf)| conf).sum();

        let fused_value = if confidence_sum > 0.0 {
            weighted_sum / confidence_sum
        } else {
            0.0
        };

        // Characteristic confidence is average of contributing source confidences
        let char_confidence = confidence_sum / sources.len() as f32;

        debug!(
            characteristic = char_name,
            value = fused_value,
            confidence = char_confidence,
            source_count = sources.len(),
            "Fused characteristic"
        );

        (Some(fused_value), char_confidence)
    }

    /// Compute source blend (overall contribution weight of each source)
    ///
    /// For each source, compute its overall weight based on:
    /// - Number of characteristics it provided
    /// - Its confidence score
    ///
    /// Returns: Vec of (source_name, contribution_weight) sorted by weight descending
    fn compute_source_blend(&self, flavors: &[FlavorExtraction]) -> Vec<(String, f32)> {
        let mut source_weights: HashMap<String, f32> = HashMap::new();

        for flavor in flavors {
            let contribution = flavor.characteristics.len() as f32 * flavor.confidence;
            *source_weights.entry(flavor.source.clone()).or_insert(0.0) += contribution;
        }

        // Normalize weights to sum to 1.0
        let total_weight: f32 = source_weights.values().sum();
        let mut blend: Vec<(String, f32)> = if total_weight > 0.0 {
            source_weights
                .into_iter()
                .map(|(source, weight)| (source, weight / total_weight))
                .collect()
        } else {
            source_weights
                .into_iter()
                .collect()
        };

        // Sort by weight descending
        blend.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        blend
    }
}

impl Default for FlavorSynthesizer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Fusion for FlavorSynthesizer {
    type Input = Vec<FlavorExtraction>;
    type Output = FusedFlavor;

    fn name(&self) -> &'static str {
        "FlavorSynthesizer"
    }

    async fn fuse(&self, inputs: Self::Input) -> Result<FusionResult<Self::Output>, FusionError> {
        debug!(input_count = inputs.len(), "Fusing flavor extractions");

        let fused_flavor = self.fuse_flavors(inputs)?;

        // Overall confidence is average of all characteristic confidences
        let confidence = if fused_flavor.confidence_map.is_empty() {
            0.0
        } else {
            fused_flavor.confidence_map.values().sum::<f32>()
                / fused_flavor.confidence_map.len() as f32
        };

        // Extract source names from source blend
        let sources: Vec<String> = fused_flavor
            .source_blend
            .iter()
            .map(|(source, weight)| format!("{} ({:.1}%)", source, weight * 100.0))
            .collect();

        Ok(FusionResult {
            output: fused_flavor,
            confidence,
            sources,
        })
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_synthesizer_name() {
        let synthesizer = FlavorSynthesizer::new();
        assert_eq!(synthesizer.name(), "FlavorSynthesizer");
    }

    #[test]
    fn test_default_expected_count() {
        let synthesizer = FlavorSynthesizer::new();
        assert_eq!(synthesizer.expected_characteristic_count, 13);
    }

    #[test]
    fn test_custom_expected_count() {
        let synthesizer = FlavorSynthesizer::with_expected_count(10);
        assert_eq!(synthesizer.expected_characteristic_count, 10);
    }

    #[tokio::test]
    async fn test_fuse_empty_input() {
        let synthesizer = FlavorSynthesizer::new();
        let result = synthesizer.fuse(vec![]).await;
        assert!(result.is_ok());

        let fusion = result.unwrap();
        assert!(fusion.output.characteristics.is_empty());
        assert_eq!(fusion.output.completeness, 0.0);
        assert_eq!(fusion.confidence, 0.0);
    }

    #[tokio::test]
    async fn test_fuse_single_flavor() {
        let synthesizer = FlavorSynthesizer::new();
        let mut characteristics = HashMap::new();
        characteristics.insert("danceability".to_string(), 0.8);
        characteristics.insert("energy".to_string(), 0.7);

        let flavors = vec![FlavorExtraction {
            characteristics,
            confidence: 0.9,
            source: "Essentia".to_string(),
        }];

        let result = synthesizer.fuse(flavors).await;
        assert!(result.is_ok());

        let fusion = result.unwrap();
        assert_eq!(
            fusion.output.characteristics.get("danceability"),
            Some(&0.8)
        );
        assert_eq!(fusion.output.characteristics.get("energy"), Some(&0.7));
        assert_eq!(fusion.output.characteristics.len(), 2);
        assert_eq!(fusion.output.source_blend.len(), 1);
        assert_eq!(fusion.output.source_blend[0].0, "Essentia");
    }

    #[tokio::test]
    async fn test_fuse_weighted_averaging() {
        let synthesizer = FlavorSynthesizer::new();

        // Source A: danceability=0.8, confidence=0.9
        let mut char_a = HashMap::new();
        char_a.insert("danceability".to_string(), 0.8);

        // Source B: danceability=0.7, confidence=0.6
        let mut char_b = HashMap::new();
        char_b.insert("danceability".to_string(), 0.7);

        let flavors = vec![
            FlavorExtraction {
                characteristics: char_a,
                confidence: 0.9,
                source: "Essentia".to_string(),
            },
            FlavorExtraction {
                characteristics: char_b,
                confidence: 0.6,
                source: "AudioDerived".to_string(),
            },
        ];

        let result = synthesizer.fuse(flavors).await;
        assert!(result.is_ok());

        let fusion = result.unwrap();

        // Expected: (0.8 * 0.9 + 0.7 * 0.6) / (0.9 + 0.6)
        //         = (0.72 + 0.42) / 1.5
        //         = 1.14 / 1.5
        //         = 0.76
        let fused_danceability = fusion.output.characteristics.get("danceability").unwrap();
        assert!(
            (*fused_danceability - 0.76).abs() < 0.001,
            "Expected ~0.76, got {}",
            fused_danceability
        );

        // Characteristic confidence should be average: (0.9 + 0.6) / 2 = 0.75
        let char_confidence = fusion.output.confidence_map.get("danceability").unwrap();
        assert_eq!(*char_confidence, 0.75);
    }

    #[tokio::test]
    async fn test_fuse_merges_different_characteristics() {
        let synthesizer = FlavorSynthesizer::new();

        // Source A provides: danceability, energy
        let mut char_a = HashMap::new();
        char_a.insert("danceability".to_string(), 0.8);
        char_a.insert("energy".to_string(), 0.7);

        // Source B provides: danceability, valence
        let mut char_b = HashMap::new();
        char_b.insert("danceability".to_string(), 0.7);
        char_b.insert("valence".to_string(), 0.6);

        let flavors = vec![
            FlavorExtraction {
                characteristics: char_a,
                confidence: 0.9,
                source: "Essentia".to_string(),
            },
            FlavorExtraction {
                characteristics: char_b,
                confidence: 0.6,
                source: "AudioDerived".to_string(),
            },
        ];

        let result = synthesizer.fuse(flavors).await;
        assert!(result.is_ok());

        let fusion = result.unwrap();

        // Should have all 3 unique characteristics
        assert_eq!(fusion.output.characteristics.len(), 3);
        assert!(fusion.output.characteristics.contains_key("danceability")); // Both sources
        assert!(fusion.output.characteristics.contains_key("energy")); // Only source A
        assert!(fusion.output.characteristics.contains_key("valence")); // Only source B

        // Energy should equal source A value (only contributor)
        assert_eq!(fusion.output.characteristics.get("energy"), Some(&0.7));

        // Valence should equal source B value (only contributor)
        assert_eq!(fusion.output.characteristics.get("valence"), Some(&0.6));
    }

    #[tokio::test]
    async fn test_source_blend_calculation() {
        let synthesizer = FlavorSynthesizer::new();

        // Source A: 2 characteristics, confidence 0.9
        let mut char_a = HashMap::new();
        char_a.insert("danceability".to_string(), 0.8);
        char_a.insert("energy".to_string(), 0.7);

        // Source B: 1 characteristic, confidence 0.6
        let mut char_b = HashMap::new();
        char_b.insert("valence".to_string(), 0.6);

        let flavors = vec![
            FlavorExtraction {
                characteristics: char_a,
                confidence: 0.9,
                source: "Essentia".to_string(),
            },
            FlavorExtraction {
                characteristics: char_b,
                confidence: 0.6,
                source: "AudioDerived".to_string(),
            },
        ];

        let result = synthesizer.fuse(flavors).await;
        assert!(result.is_ok());

        let fusion = result.unwrap();

        // Source A contribution: 2 * 0.9 = 1.8
        // Source B contribution: 1 * 0.6 = 0.6
        // Total: 2.4
        // Normalized: Essentia = 1.8/2.4 = 0.75, AudioDerived = 0.6/2.4 = 0.25

        assert_eq!(fusion.output.source_blend.len(), 2);

        // Should be sorted by weight descending (Essentia first)
        assert_eq!(fusion.output.source_blend[0].0, "Essentia");
        assert!(
            (fusion.output.source_blend[0].1 - 0.75).abs() < 0.001,
            "Expected ~0.75, got {}",
            fusion.output.source_blend[0].1
        );

        assert_eq!(fusion.output.source_blend[1].0, "AudioDerived");
        assert!(
            (fusion.output.source_blend[1].1 - 0.25).abs() < 0.001,
            "Expected ~0.25, got {}",
            fusion.output.source_blend[1].1
        );
    }

    #[tokio::test]
    async fn test_completeness_score() {
        let synthesizer = FlavorSynthesizer::with_expected_count(10);

        // Provide 5 characteristics
        let mut characteristics = HashMap::new();
        characteristics.insert("danceability".to_string(), 0.8);
        characteristics.insert("energy".to_string(), 0.7);
        characteristics.insert("valence".to_string(), 0.6);
        characteristics.insert("acousticness".to_string(), 0.5);
        characteristics.insert("instrumentalness".to_string(), 0.4);

        let flavors = vec![FlavorExtraction {
            characteristics,
            confidence: 0.9,
            source: "Essentia".to_string(),
        }];

        let result = synthesizer.fuse(flavors).await;
        assert!(result.is_ok());

        let fusion = result.unwrap();

        // Completeness = 5 / 10 = 0.5
        assert_eq!(fusion.output.completeness, 0.5);
    }

    #[tokio::test]
    async fn test_completeness_capped_at_one() {
        let synthesizer = FlavorSynthesizer::with_expected_count(2);

        // Provide 5 characteristics (more than expected)
        let mut characteristics = HashMap::new();
        characteristics.insert("danceability".to_string(), 0.8);
        characteristics.insert("energy".to_string(), 0.7);
        characteristics.insert("valence".to_string(), 0.6);
        characteristics.insert("acousticness".to_string(), 0.5);
        characteristics.insert("instrumentalness".to_string(), 0.4);

        let flavors = vec![FlavorExtraction {
            characteristics,
            confidence: 0.9,
            source: "Essentia".to_string(),
        }];

        let result = synthesizer.fuse(flavors).await;
        assert!(result.is_ok());

        let fusion = result.unwrap();

        // Completeness should be capped at 1.0 even though 5 > 2
        assert_eq!(fusion.output.completeness, 1.0);
    }
}
