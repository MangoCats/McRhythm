//! Quality Scorer (Tier 3 Validator)
//!
//! Assesses overall data quality across fused identity, metadata, and flavor data.
//! Combines consistency and completeness insights with confidence-based quality metrics.
//!
//! # Implementation
//! - TASK-018: Quality Scorer (PLAN024)
//! - Validation strategy: Holistic quality assessment
//!
//! # Architecture
//! Implements `Validation` trait for integration with 3-tier architecture.
//! Accepts FusedPassage and produces ValidationResult with quality breakdown.
//!
//! # Quality Dimensions
//! 1. **Data Reliability**: Source confidence levels and agreement
//! 2. **Information Richness**: Completeness and depth of data
//! 3. **Consistency**: Internal coherence (no contradictions)
//! 4. **Usability**: Fitness for automatic passage selection
//!
//! # Scoring Algorithm
//! - **Reliability Score** (weight: 0.35):
//!   - Identity confidence (MBID + posterior probability)
//!   - Metadata field confidences (average)
//!   - Flavor characteristic confidences (average)
//! - **Richness Score** (weight: 0.30):
//!   - Metadata completeness
//!   - Flavor completeness
//!   - Optional field presence
//! - **Consistency Score** (weight: 0.20):
//!   - No conflicts: 1.0
//!   - Minor conflicts (≤2): 0.8
//!   - Major conflicts (>2): 0.5
//! - **Usability Score** (weight: 0.15):
//!   - Can identify recording? (MBID present)
//!   - Can display to user? (title + artist present)
//!   - Can select by flavor? (flavor completeness ≥ 0.3)
//!
//! # Status Determination
//! - Pass: overall ≥ 0.80 (high quality)
//! - Warning: overall ≥ 0.60 (acceptable quality)
//! - Fail: overall < 0.60 (insufficient quality)
//!
//! # Example
//! ```rust,ignore
//! use wkmp_ai::validators::QualityScorer;
//! use wkmp_ai::workflow::FusedPassage;
//!
//! let scorer = QualityScorer::new();
//! let result = scorer.validate(&fused_passage).await?;
//!
//! println!("Quality: {:.1}%", result.score * 100.0);
//! println!("Recommendations: {:?}", result.issues);
//! ```

use crate::types::{Validation, ValidationError, ValidationResult, ValidationStatus};
use crate::workflow::FusedPassage;
use async_trait::async_trait;
use serde_json::json;
use tracing::debug;

/// Quality Scorer
///
/// Assesses overall data quality combining reliability, richness, consistency, and usability.
///
/// # Quality Assessment
/// - Reliability: Source confidence levels
/// - Richness: Data completeness and depth
/// - Consistency: Internal coherence
/// - Usability: Fitness for purpose
///
/// # Scoring
/// Weighted average of four dimensions:
/// - Reliability: 35% weight
/// - Richness: 30% weight
/// - Consistency: 20% weight
/// - Usability: 15% weight
///
/// Overall score determines validation status and recommendations.
pub struct QualityScorer {
    /// Minimum overall score for Pass status
    pass_threshold: f32,
    /// Minimum overall score for Warning status (below this is Fail)
    warning_threshold: f32,
    /// Minimum confidence threshold for high-quality data
    min_high_confidence: f32,
}

impl QualityScorer {
    /// Create new Quality Scorer with default thresholds
    pub fn new() -> Self {
        Self {
            pass_threshold: 0.80,
            warning_threshold: 0.60,
            min_high_confidence: 0.8,
        }
    }

    /// Create Quality Scorer with custom thresholds
    pub fn with_thresholds(
        pass_threshold: f32,
        warning_threshold: f32,
        min_high_confidence: f32,
    ) -> Self {
        Self {
            pass_threshold,
            warning_threshold,
            min_high_confidence,
        }
    }

    /// Score fused passage quality
    fn score_passage(&self, passage: &FusedPassage) -> ValidationResult {
        let mut recommendations = Vec::new();

        // Dimension 1: Data reliability (35% weight)
        let (reliability_score, reliability_recs) = self.score_reliability(passage);
        recommendations.extend(reliability_recs);

        // Dimension 2: Information richness (30% weight)
        let (richness_score, richness_recs) = self.score_richness(passage);
        recommendations.extend(richness_recs);

        // Dimension 3: Consistency (20% weight)
        let (consistency_score, consistency_recs) = self.score_consistency(passage);
        recommendations.extend(consistency_recs);

        // Dimension 4: Usability (15% weight)
        let (usability_score, usability_recs) = self.score_usability(passage);
        recommendations.extend(usability_recs);

        // Compute weighted overall score
        let overall_score = (reliability_score * 0.35)
            + (richness_score * 0.30)
            + (consistency_score * 0.20)
            + (usability_score * 0.15);

        // Determine status
        let status = if overall_score >= self.pass_threshold {
            ValidationStatus::Pass
        } else if overall_score >= self.warning_threshold {
            ValidationStatus::Warning
        } else {
            ValidationStatus::Fail
        };

        debug!(
            status = ?status,
            overall = overall_score,
            reliability = reliability_score,
            richness = richness_score,
            consistency = consistency_score,
            usability = usability_score,
            "Quality scoring complete"
        );

        // Build detailed report
        let report = json!({
            "validator": "QualityScorer",
            "overall_score": overall_score,
            "status": format!("{:?}", status),
            "dimensions": {
                "reliability": {
                    "score": reliability_score,
                    "weight": 0.35,
                },
                "richness": {
                    "score": richness_score,
                    "weight": 0.30,
                },
                "consistency": {
                    "score": consistency_score,
                    "weight": 0.20,
                },
                "usability": {
                    "score": usability_score,
                    "weight": 0.15,
                },
            },
            "thresholds": {
                "pass": self.pass_threshold,
                "warning": self.warning_threshold,
                "min_high_confidence": self.min_high_confidence,
            }
        });

        ValidationResult {
            status,
            score: overall_score,
            issues: recommendations,
            report,
        }
    }

    /// Score data reliability (source confidence levels)
    ///
    /// Returns: (score, recommendations)
    ///
    /// Components:
    /// - Identity confidence (MBID + posterior)
    /// - Metadata field confidences (average)
    /// - Flavor characteristic confidences (average)
    fn score_reliability(&self, passage: &FusedPassage) -> (f32, Vec<String>) {
        let mut recommendations = Vec::new();

        // Identity reliability (weight: 0.4 of reliability score)
        let identity_reliability = if passage.identity.recording_mbid.is_some() {
            // Use posterior probability if available, otherwise confidence
            passage.identity.posterior_probability.max(passage.identity.confidence)
        } else {
            0.0
        };

        if identity_reliability < self.min_high_confidence && passage.identity.recording_mbid.is_some() {
            recommendations.push(format!(
                "Low identity confidence: {:.1}% (recommend manual verification)",
                identity_reliability * 100.0
            ));
        }

        // Metadata reliability (weight: 0.4 of reliability score)
        let metadata_confidences: Vec<f32> = [
            passage.metadata.title.as_ref().map(|v| v.confidence),
            passage.metadata.artist.as_ref().map(|v| v.confidence),
            passage.metadata.album.as_ref().map(|v| v.confidence),
            passage.metadata.recording_mbid.as_ref().map(|v| v.confidence),
        ]
        .iter()
        .filter_map(|&c| c)
        .collect();

        let metadata_reliability = if metadata_confidences.is_empty() {
            0.0
        } else {
            metadata_confidences.iter().sum::<f32>() / metadata_confidences.len() as f32
        };

        if metadata_reliability < 0.7 && !metadata_confidences.is_empty() {
            recommendations.push(format!(
                "Low metadata confidence: {:.1}% (consider additional sources)",
                metadata_reliability * 100.0
            ));
        }

        // Flavor reliability (weight: 0.2 of reliability score)
        let flavor_reliability = if passage.flavor.confidence_map.is_empty() {
            0.0
        } else {
            passage.flavor.confidence_map.values().sum::<f32>()
                / passage.flavor.confidence_map.len() as f32
        };

        let overall_reliability = (identity_reliability * 0.4)
            + (metadata_reliability * 0.4)
            + (flavor_reliability * 0.2);

        (overall_reliability, recommendations)
    }

    /// Score information richness (completeness and depth)
    ///
    /// Returns: (score, recommendations)
    ///
    /// Components:
    /// - Metadata completeness (from FusedMetadata)
    /// - Flavor completeness (from FusedFlavor)
    /// - Optional field presence (bonus)
    fn score_richness(&self, passage: &FusedPassage) -> (f32, Vec<String>) {
        let mut recommendations = Vec::new();

        // Metadata richness (weight: 0.5 of richness score)
        let metadata_richness = passage.metadata.metadata_completeness;

        if metadata_richness < 0.5 {
            recommendations.push(format!(
                "Incomplete metadata: {:.1}% (missing critical fields)",
                metadata_richness * 100.0
            ));
        }

        // Flavor richness (weight: 0.5 of richness score)
        let flavor_richness = passage.flavor.completeness;

        if flavor_richness < 0.5 {
            recommendations.push(format!(
                "Incomplete flavor data: {:.1}% (may affect selection quality)",
                flavor_richness * 100.0
            ));
        }

        let overall_richness = (metadata_richness * 0.5) + (flavor_richness * 0.5);

        (overall_richness, recommendations)
    }

    /// Score consistency (internal coherence)
    ///
    /// Returns: (score, recommendations)
    ///
    /// Based on conflict count from identity fusion
    fn score_consistency(&self, passage: &FusedPassage) -> (f32, Vec<String>) {
        let mut recommendations = Vec::new();

        let conflict_count = passage.identity.conflicts.len();

        let consistency_score = if conflict_count == 0 {
            1.0 // Perfect consistency
        } else if conflict_count <= 2 {
            recommendations.push(format!(
                "Minor identity conflicts detected ({}) - resolution confidence may be reduced",
                conflict_count
            ));
            0.8
        } else {
            recommendations.push(format!(
                "Multiple identity conflicts detected ({}) - recommend manual review",
                conflict_count
            ));
            0.5
        };

        (consistency_score, recommendations)
    }

    /// Score usability (fitness for automatic passage selection)
    ///
    /// Returns: (score, recommendations)
    ///
    /// Checks:
    /// - Can identify recording? (MBID present)
    /// - Can display to user? (title + artist present)
    /// - Can select by flavor? (sufficient characteristics)
    fn score_usability(&self, passage: &FusedPassage) -> (f32, Vec<String>) {
        let mut recommendations = Vec::new();
        let mut usability_components = Vec::new();

        // Component 1: Can identify recording? (weight: 0.4)
        let can_identify = if passage.identity.recording_mbid.is_some() {
            1.0
        } else {
            recommendations.push("Missing recording MBID (cannot track playback history)".to_string());
            0.0
        };
        usability_components.push(can_identify * 0.4);

        // Component 2: Can display to user? (weight: 0.4)
        let can_display = if passage.metadata.title.is_some() && passage.metadata.artist.is_some() {
            1.0
        } else {
            recommendations.push("Missing title or artist (cannot display properly)".to_string());
            0.0
        };
        usability_components.push(can_display * 0.4);

        // Component 3: Can select by flavor? (weight: 0.2)
        let can_select = if passage.flavor.completeness >= 0.3 {
            1.0
        } else {
            recommendations.push(format!(
                "Insufficient flavor data: {:.1}% (may not participate in flavor-based selection)",
                passage.flavor.completeness * 100.0
            ));
            passage.flavor.completeness / 0.3 // Partial credit
        };
        usability_components.push(can_select * 0.2);

        let usability_score = usability_components.iter().sum();

        (usability_score, recommendations)
    }
}

impl Default for QualityScorer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Validation for QualityScorer {
    type Input = FusedPassage;

    fn name(&self) -> &'static str {
        "QualityScorer"
    }

    async fn validate(&self, input: &Self::Input) -> Result<ValidationResult, ValidationError> {
        debug!("Scoring fused passage quality");
        Ok(self.score_passage(input))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ConfidenceValue, FusedFlavor, FusedIdentity, FusedMetadata};
    use std::collections::HashMap;

    fn create_high_quality_passage() -> FusedPassage {
        let mut characteristics = HashMap::new();
        characteristics.insert("danceability".to_string(), 0.8);
        characteristics.insert("energy".to_string(), 0.7);
        characteristics.insert("valence".to_string(), 0.6);

        let mut confidence_map = HashMap::new();
        confidence_map.insert("danceability".to_string(), 0.9);
        confidence_map.insert("energy".to_string(), 0.85);
        confidence_map.insert("valence".to_string(), 0.8);

        FusedPassage {
            identity: FusedIdentity {
                recording_mbid: Some("test-mbid-123".to_string()),
                confidence: 0.9,
                posterior_probability: 0.95,
                conflicts: vec![],
            },
            metadata: FusedMetadata {
                title: Some(ConfidenceValue::new(
                    "Test Song".to_string(),
                    0.9,
                    "ID3".to_string(),
                )),
                artist: Some(ConfidenceValue::new(
                    "Test Artist".to_string(),
                    0.9,
                    "ID3".to_string(),
                )),
                album: Some(ConfidenceValue::new(
                    "Test Album".to_string(),
                    0.85,
                    "ID3".to_string(),
                )),
                recording_mbid: Some(ConfidenceValue::new(
                    "test-mbid-123".to_string(),
                    0.9,
                    "AcoustID".to_string(),
                )),
                metadata_completeness: 1.0,
                additional: std::collections::HashMap::new(),
            },
            flavor: FusedFlavor {
                characteristics,
                confidence_map,
                source_blend: vec![("Essentia".to_string(), 1.0)],
                completeness: 0.8,
            },
        }
    }

    #[test]
    fn test_scorer_name() {
        let scorer = QualityScorer::new();
        assert_eq!(scorer.name(), "QualityScorer");
    }

    #[test]
    fn test_default_thresholds() {
        let scorer = QualityScorer::new();
        assert_eq!(scorer.pass_threshold, 0.80);
        assert_eq!(scorer.warning_threshold, 0.60);
        assert_eq!(scorer.min_high_confidence, 0.8);
    }

    #[test]
    fn test_custom_thresholds() {
        let scorer = QualityScorer::with_thresholds(0.85, 0.65, 0.75);
        assert_eq!(scorer.pass_threshold, 0.85);
        assert_eq!(scorer.warning_threshold, 0.65);
        assert_eq!(scorer.min_high_confidence, 0.75);
    }

    #[tokio::test]
    async fn test_score_high_quality_passage() {
        let scorer = QualityScorer::new();
        let passage = create_high_quality_passage();

        let result = scorer.validate(&passage).await;
        assert!(result.is_ok());

        let validation = result.unwrap();
        assert_eq!(validation.status, ValidationStatus::Pass);
        assert!(validation.score >= 0.80);
        assert!(validation.issues.is_empty() || validation.issues.len() <= 1);
    }

    #[tokio::test]
    async fn test_score_low_identity_confidence() {
        let scorer = QualityScorer::new();
        let mut passage = create_high_quality_passage();
        passage.identity.confidence = 0.4;
        passage.identity.posterior_probability = 0.45;

        let result = scorer.validate(&passage).await;
        assert!(result.is_ok());

        let validation = result.unwrap();
        // Reliability reduced, should still be acceptable
        assert!(validation.score < 0.90);
        assert!(!validation.issues.is_empty());
        assert!(validation
            .issues
            .iter()
            .any(|issue| issue.contains("identity confidence")));
    }

    #[tokio::test]
    async fn test_score_missing_mbid() {
        let scorer = QualityScorer::new();
        let mut passage = create_high_quality_passage();
        passage.identity.recording_mbid = None;

        let result = scorer.validate(&passage).await;
        assert!(result.is_ok());

        let validation = result.unwrap();
        // No MBID = reliability 0, usability reduced
        // Expected: reliability * 0.35 = 0, but other dimensions still contribute
        assert!(validation.score < 0.80);
        assert!(!validation.issues.is_empty());
        assert!(validation
            .issues
            .iter()
            .any(|issue| issue.contains("MBID")));
    }

    #[tokio::test]
    async fn test_score_incomplete_metadata() {
        let scorer = QualityScorer::new();
        let mut passage = create_high_quality_passage();
        passage.metadata.title = None;
        passage.metadata.metadata_completeness = 0.3;

        let result = scorer.validate(&passage).await;
        assert!(result.is_ok());

        let validation = result.unwrap();
        // Reduced richness and usability
        assert!(validation.score < 0.80);
        assert!(!validation.issues.is_empty());
    }

    #[tokio::test]
    async fn test_score_conflicts_present() {
        let scorer = QualityScorer::new();
        let mut passage = create_high_quality_passage();
        passage.identity.conflicts = vec![
            "Conflict 1".to_string(),
            "Conflict 2".to_string(),
            "Conflict 3".to_string(),
        ];

        let result = scorer.validate(&passage).await;
        assert!(result.is_ok());

        let validation = result.unwrap();
        // Consistency reduced to 0.5 (>2 conflicts)
        assert!(validation.score < 0.90);
        assert!(!validation.issues.is_empty());
        assert!(validation
            .issues
            .iter()
            .any(|issue| issue.contains("conflicts")));
    }

    #[tokio::test]
    async fn test_score_low_flavor_completeness() {
        let scorer = QualityScorer::new();
        let mut passage = create_high_quality_passage();
        passage.flavor.completeness = 0.2; // Below 0.3 threshold

        let result = scorer.validate(&passage).await;
        assert!(result.is_ok());

        let validation = result.unwrap();
        // Richness affected (flavor 0.2), but other dimensions still high
        // Reliability: 0.92, Richness: 0.6, Consistency: 1.0, Usability: 0.93
        // Overall: ~0.84 (still passes but with recommendations)
        assert!(validation.score >= 0.80); // Still passes overall
        assert!(!validation.issues.is_empty()); // But has recommendations
        assert!(validation
            .issues
            .iter()
            .any(|issue| issue.contains("flavor")));
    }

    #[tokio::test]
    async fn test_score_minimal_usable_passage() {
        let scorer = QualityScorer::new();
        let mut passage = create_high_quality_passage();

        // Minimal: MBID + title + artist + minimal flavor
        passage.metadata.album = None;
        passage.metadata.recording_mbid = None;
        passage.metadata.metadata_completeness = 0.5;
        passage.flavor.completeness = 0.3; // Exactly at minimum

        let result = scorer.validate(&passage).await;
        assert!(result.is_ok());

        let validation = result.unwrap();
        // Should be in warning range (0.60-0.80)
        assert!(validation.score >= 0.60 && validation.score < 0.80);
        assert_eq!(validation.status, ValidationStatus::Warning);
    }

    #[tokio::test]
    async fn test_score_unusable_passage() {
        let scorer = QualityScorer::new();
        let mut passage = create_high_quality_passage();

        // Missing critical fields
        passage.identity.recording_mbid = None;
        passage.metadata.title = None;
        passage.metadata.metadata_completeness = 0.2;
        passage.flavor.completeness = 0.1;

        let result = scorer.validate(&passage).await;
        assert!(result.is_ok());

        let validation = result.unwrap();
        assert_eq!(validation.status, ValidationStatus::Fail);
        assert!(validation.score < 0.60);
        assert!(validation.issues.len() >= 2);
    }

    #[tokio::test]
    async fn test_report_structure() {
        let scorer = QualityScorer::new();
        let passage = create_high_quality_passage();

        let result = scorer.validate(&passage).await;
        assert!(result.is_ok());

        let validation = result.unwrap();
        let report = &validation.report;

        // Verify report structure
        assert!(report["validator"].is_string());
        assert_eq!(report["validator"], "QualityScorer");
        assert!(report["overall_score"].is_f64());
        assert!(report["status"].is_string());
        assert!(report["dimensions"].is_object());
        assert!(report["dimensions"]["reliability"]["score"].is_f64());
        assert!(report["dimensions"]["richness"]["score"].is_f64());
        assert!(report["dimensions"]["consistency"]["score"].is_f64());
        assert!(report["dimensions"]["usability"]["score"].is_f64());
        assert!(report["thresholds"].is_object());
    }
}
