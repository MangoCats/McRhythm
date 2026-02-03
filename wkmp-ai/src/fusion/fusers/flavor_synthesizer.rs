// Flavor Synthesizer - Characteristic-Wise Weighted Averaging
//
// PLAN023: REQ-AI-040 series - Musical flavor synthesis with normalization
// Expected characteristics: 18 (per CRITICAL-002 resolution)

use crate::fusion::{ExtractionResult, FusedFlavor, MusicalFlavor, CharacteristicKey, Confidence};
use anyhow::Result;
use std::collections::HashMap;

const EXPECTED_CHARACTERISTICS: usize = 18; // CRITICAL-002: 18 AcousticBrainz categories

/// Synthesize musical flavor from multiple sources using characteristic-wise weighted averaging
///
/// # Arguments
/// * `extractions` - Results from extractors with flavor information
///
/// # Returns
/// * `FusedFlavor` with normalized characteristics, source blend, and completeness
pub fn synthesize_flavor(extractions: &[ExtractionResult]) -> Result<FusedFlavor> {
    let flavor_extractions: Vec<_> = extractions
        .iter()
        .filter(|e| e.flavor.is_some())
        .collect();

    if flavor_extractions.is_empty() {
        return Ok(FusedFlavor {
            characteristics: MusicalFlavor::new(),
            source_blend: vec![],
            confidence_map: HashMap::new(),
            completeness: 0.0,
        });
    }

    // Build source blend list
    let source_blend: Vec<String> = flavor_extractions
        .iter()
        .map(|e| format!("{}:{:.2}", e.source, e.confidence))
        .collect();

    // Collect all characteristic values with confidence weights
    let mut char_values: HashMap<CharacteristicKey, Vec<(f64, Confidence)>> = HashMap::new();

    for extraction in &flavor_extractions {
        if let Some(flavor) = &extraction.flavor {
            for (key, value) in &flavor.characteristics {
                // Get per-characteristic confidence if available, otherwise use overall
                let confidence = flavor
                    .characteristic_confidence
                    .as_ref()
                    .and_then(|map| map.get(key))
                    .copied()
                    .unwrap_or(extraction.confidence);

                char_values
                    .entry(key.clone())
                    .or_default()
                    .push((*value, confidence));
            }
        }
    }

    // Apply characteristic-wise weighted averaging
    let mut fused_characteristics = MusicalFlavor::new();
    let mut confidence_map = HashMap::new();

    for (key, values) in char_values {
        let (weighted_value, avg_confidence) = weighted_average(&values);
        fused_characteristics.insert(key.clone(), weighted_value);
        confidence_map.insert(key, avg_confidence);
    }

    // Normalize within categories
    normalize_flavor(&mut fused_characteristics);

    // Calculate completeness
    let present_count = fused_characteristics.len();
    let completeness = present_count as f64 / EXPECTED_CHARACTERISTICS as f64;

    Ok(FusedFlavor {
        characteristics: fused_characteristics,
        source_blend,
        confidence_map,
        completeness,
    })
}

/// Calculate weighted average of values
///
/// Returns: (weighted_average, average_confidence)
fn weighted_average(values: &[(f64, Confidence)]) -> (f64, Confidence) {
    if values.is_empty() {
        return (0.0, 0.0);
    }

    let sum_weighted: f64 = values.iter().map(|(val, conf)| val * conf).sum();
    let sum_weights: f64 = values.iter().map(|(_, conf)| conf).sum();

    let weighted_avg = if sum_weights > 0.0 {
        sum_weighted / sum_weights
    } else {
        0.0
    };

    let avg_confidence = sum_weights / values.len() as f64;

    (weighted_avg, avg_confidence)
}

/// Normalize musical flavor characteristics within categories
///
/// # Arguments
/// * `flavor` - Musical flavor characteristics map
///
/// # Returns
/// * Normalized flavor (each category sums to 1.0)
pub fn normalize_flavor(flavor: &mut MusicalFlavor) {
    // Group by category (first part before first '.')
    let mut categories: HashMap<String, Vec<CharacteristicKey>> = HashMap::new();
    for key in flavor.keys() {
        if let Some(category) = key.split('.').next() {
            categories
                .entry(category.to_string())
                .or_default()
                .push(key.clone());
        }
    }

    // Normalize each category
    for (_category, keys) in categories {
        let sum: f64 = keys.iter().filter_map(|k| flavor.get(k)).sum();
        if sum > 0.0 {
            for key in keys {
                if let Some(value) = flavor.get_mut(&key) {
                    *value /= sum;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_flavor() {
        let mut flavor = MusicalFlavor::new();
        flavor.insert("danceability.danceable".to_string(), 0.6);
        flavor.insert("danceability.not_danceable".to_string(), 0.8);
        flavor.insert("mood_aggressive.aggressive".to_string(), 0.3);
        flavor.insert("mood_aggressive.not_aggressive".to_string(), 0.7);

        normalize_flavor(&mut flavor);

        // Check danceability category sums to 1.0
        let dance_sum: f64 = flavor
            .iter()
            .filter(|(k, _)| k.starts_with("danceability"))
            .map(|(_, v)| v)
            .sum();
        assert!((dance_sum - 1.0).abs() < 0.0001, "Danceability should sum to 1.0");

        // Check mood_aggressive category sums to 1.0
        let mood_sum: f64 = flavor
            .iter()
            .filter(|(k, _)| k.starts_with("mood_aggressive"))
            .map(|(_, v)| v)
            .sum();
        assert!((mood_sum - 1.0).abs() < 0.0001, "Mood aggressive should sum to 1.0");
    }

    #[test]
    fn test_expected_characteristics_constant() {
        assert_eq!(EXPECTED_CHARACTERISTICS, 18);
    }
}
