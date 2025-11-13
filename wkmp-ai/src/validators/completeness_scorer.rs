//! Completeness Scorer (Tier 3 Validator)
//!
//! Assesses data completeness across fused identity, metadata, and flavor data.
//! Computes coverage scores and identifies missing critical fields.
//!
//! # Implementation
//! - TASK-017: Completeness Scorer (PLAN024)
//! - Validation strategy: Multi-dimensional completeness assessment
//!
//! # Architecture
//! Implements `Validation` trait for integration with 3-tier architecture.
//! Accepts FusedPassage and produces ValidationResult with completeness breakdown.
//!
//! # Completeness Dimensions
//! 1. **Metadata Completeness**: Critical fields (title, artist) and optional fields (album, MBID)
//! 2. **Identity Completeness**: Recording MBID presence and confidence level
//! 3. **Flavor Completeness**: Musical characteristics coverage for selection algorithm
//! 4. **Overall Completeness**: Weighted average of above dimensions
//!
//! # Scoring Algorithm
//! - **Metadata Score** (weight: 0.4):
//!   - Critical fields (title, artist): 0.5 each (100% if both present)
//!   - Optional fields (album, MBID): bonus points up to 0.2
//! - **Identity Score** (weight: 0.3):
//!   - MBID present: base 0.7
//!   - High confidence (≥0.8): full 1.0
//!   - Medium confidence (0.5-0.8): 0.85
//!   - Low confidence (<0.5): 0.7
//! - **Flavor Score** (weight: 0.3):
//!   - Based on flavor.completeness field (0.0-1.0)
//!   - Minimum threshold: 0.3 (30% of expected characteristics)
//!
//! # Status Determination
//! - Pass: overall ≥ 0.75 (good completeness)
//! - Warning: overall ≥ 0.5 (acceptable completeness)
//! - Fail: overall < 0.5 (insufficient data)
//!
//! # Example
//! ```rust,ignore
//! use wkmp_ai::validators::CompletenessScorer;
//! use wkmp_ai::workflow::FusedPassage;
//!
//! let scorer = CompletenessScorer::new();
//! let result = scorer.validate(&fused_passage).await?;
//!
//! println!("Completeness: {:.1}%", result.score * 100.0);
//! println!("Missing: {:?}", result.issues);
//! ```

use crate::types::{Validation, ValidationError, ValidationResult, ValidationStatus};
use crate::workflow::FusedPassage;
use async_trait::async_trait;
use serde_json::json;
use tracing::debug;

/// Completeness Scorer
///
/// Assesses data completeness across identity, metadata, and flavor dimensions.
///
/// # Completeness Assessment
/// - Metadata: critical fields (title, artist) + optional fields (album, MBID)
/// - Identity: MBID presence and confidence level
/// - Flavor: characteristic coverage for selection algorithm
///
/// # Scoring
/// Weighted average of three dimensions:
/// - Metadata: 40% weight
/// - Identity: 30% weight
/// - Flavor: 30% weight
///
/// Overall score determines validation status.
pub struct CompletenessScorer {
    /// Minimum overall score for Pass status
    pass_threshold: f32,
    /// Minimum overall score for Warning status (below this is Fail)
    warning_threshold: f32,
    /// Minimum flavor completeness threshold (0.0-1.0)
    min_flavor_completeness: f32,
}

impl CompletenessScorer {
    /// Create new Completeness Scorer with default thresholds
    pub fn new() -> Self {
        Self {
            pass_threshold: 0.75,
            warning_threshold: 0.5,
            min_flavor_completeness: 0.3,
        }
    }

    /// Create Completeness Scorer with custom thresholds
    pub fn with_thresholds(
        pass_threshold: f32,
        warning_threshold: f32,
        min_flavor_completeness: f32,
    ) -> Self {
        Self {
            pass_threshold,
            warning_threshold,
            min_flavor_completeness,
        }
    }

    /// Score fused passage completeness
    fn score_passage(&self, passage: &FusedPassage) -> ValidationResult {
        let mut issues = Vec::new();

        // Dimension 1: Metadata completeness (40% weight)
        let (metadata_score, metadata_issues) = self.score_metadata(&passage.metadata);
        issues.extend(metadata_issues);

        // Dimension 2: Identity completeness (30% weight)
        let (identity_score, identity_issues) = self.score_identity(&passage.identity);
        issues.extend(identity_issues);

        // Dimension 3: Flavor completeness (30% weight)
        let (flavor_score, flavor_issues) = self.score_flavor(&passage.flavor);
        issues.extend(flavor_issues);

        // Compute weighted overall score
        let overall_score = (metadata_score * 0.4) + (identity_score * 0.3) + (flavor_score * 0.3);

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
            metadata = metadata_score,
            identity = identity_score,
            flavor = flavor_score,
            "Completeness scoring complete"
        );

        // Build detailed report
        let report = json!({
            "validator": "CompletenessScorer",
            "overall_score": overall_score,
            "status": format!("{:?}", status),
            "dimensions": {
                "metadata": {
                    "score": metadata_score,
                    "weight": 0.4,
                },
                "identity": {
                    "score": identity_score,
                    "weight": 0.3,
                },
                "flavor": {
                    "score": flavor_score,
                    "weight": 0.3,
                },
            },
            "thresholds": {
                "pass": self.pass_threshold,
                "warning": self.warning_threshold,
                "min_flavor": self.min_flavor_completeness,
            }
        });

        ValidationResult {
            status,
            score: overall_score,
            issues,
            report,
        }
    }

    /// Score metadata completeness
    ///
    /// Returns: (score, issues)
    ///
    /// Scoring:
    /// - Critical fields (title, artist): 0.5 each (required for basic functionality)
    /// - Optional fields (album, MBID): bonus 0.1 each (enhance quality)
    fn score_metadata(
        &self,
        metadata: &crate::types::FusedMetadata,
    ) -> (f32, Vec<String>) {
        let mut score = 0.0_f32;
        let mut issues = Vec::new();

        // Critical field: title (50% of score)
        if metadata.title.is_some() {
            score += 0.5;
        } else {
            issues.push("Missing metadata: title (critical)".to_string());
        }

        // Critical field: artist (50% of score)
        if metadata.artist.is_some() {
            score += 0.5;
        } else {
            issues.push("Missing metadata: artist (critical)".to_string());
        }

        // Optional bonus: album (10% bonus)
        if metadata.album.is_some() {
            score += 0.1;
        }

        // Optional bonus: MBID (10% bonus)
        if metadata.recording_mbid.is_some() {
            score += 0.1;
        }

        // Cap score at 1.0
        score = score.min(1.0);

        (score, issues)
    }

    /// Score identity completeness
    ///
    /// Returns: (score, issues)
    ///
    /// Scoring:
    /// - No MBID: 0.0 (cannot identify recording)
    /// - MBID with low confidence (<0.5): 0.7
    /// - MBID with medium confidence (0.5-0.8): 0.85
    /// - MBID with high confidence (≥0.8): 1.0
    fn score_identity(
        &self,
        identity: &crate::types::FusedIdentity,
    ) -> (f32, Vec<String>) {
        let mut issues = Vec::new();

        let score = if let Some(ref _mbid) = identity.recording_mbid {
            // Have MBID, score based on confidence
            if identity.confidence >= 0.8 {
                1.0 // High confidence
            } else if identity.confidence >= 0.5 {
                0.85 // Medium confidence
            } else {
                0.7 // Low confidence
            }
        } else {
            // No MBID
            issues.push("Missing recording MBID (identity incomplete)".to_string());
            0.0
        };

        (score, issues)
    }

    /// Score flavor completeness
    ///
    /// Returns: (score, issues)
    ///
    /// Scoring:
    /// - Uses flavor.completeness field directly (0.0-1.0)
    /// - Issues generated if below minimum threshold
    fn score_flavor(
        &self,
        flavor: &crate::types::FusedFlavor,
    ) -> (f32, Vec<String>) {
        let mut issues = Vec::new();

        let score = flavor.completeness;

        if score < self.min_flavor_completeness {
            issues.push(format!(
                "Insufficient flavor characteristics: {:.1}% (minimum: {:.1}%)",
                score * 100.0,
                self.min_flavor_completeness * 100.0
            ));
        }

        (score, issues)
    }
}

impl Default for CompletenessScorer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Validation for CompletenessScorer {
    type Input = FusedPassage;

    fn name(&self) -> &'static str {
        "CompletenessScorer"
    }

    async fn validate(&self, input: &Self::Input) -> Result<ValidationResult, ValidationError> {
        debug!("Scoring fused passage completeness");
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

    fn create_complete_passage() -> FusedPassage {
        let mut characteristics = HashMap::new();
        characteristics.insert("danceability".to_string(), 0.8);
        characteristics.insert("energy".to_string(), 0.7);

        let mut confidence_map = HashMap::new();
        confidence_map.insert("danceability".to_string(), 0.85);
        confidence_map.insert("energy".to_string(), 0.8);

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
                    0.85,
                    "ID3".to_string(),
                )),
                album: Some(ConfidenceValue::new(
                    "Test Album".to_string(),
                    0.8,
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
                completeness: 0.8, // 80% complete
            },
        }
    }

    #[test]
    fn test_scorer_name() {
        let scorer = CompletenessScorer::new();
        assert_eq!(scorer.name(), "CompletenessScorer");
    }

    #[test]
    fn test_default_thresholds() {
        let scorer = CompletenessScorer::new();
        assert_eq!(scorer.pass_threshold, 0.75);
        assert_eq!(scorer.warning_threshold, 0.5);
        assert_eq!(scorer.min_flavor_completeness, 0.3);
    }

    #[test]
    fn test_custom_thresholds() {
        let scorer = CompletenessScorer::with_thresholds(0.8, 0.6, 0.4);
        assert_eq!(scorer.pass_threshold, 0.8);
        assert_eq!(scorer.warning_threshold, 0.6);
        assert_eq!(scorer.min_flavor_completeness, 0.4);
    }

    #[tokio::test]
    async fn test_score_complete_passage() {
        let scorer = CompletenessScorer::new();
        let passage = create_complete_passage();

        let result = scorer.validate(&passage).await;
        assert!(result.is_ok());

        let validation = result.unwrap();
        assert_eq!(validation.status, ValidationStatus::Pass);
        assert!(validation.score >= 0.75);
        assert!(validation.issues.is_empty());
    }

    #[tokio::test]
    async fn test_score_missing_title() {
        let scorer = CompletenessScorer::new();
        let mut passage = create_complete_passage();
        passage.metadata.title = None;

        let result = scorer.validate(&passage).await;
        assert!(result.is_ok());

        let validation = result.unwrap();
        // Metadata: artist (0.5) + album bonus (0.1) + MBID bonus (0.1) = 0.7 → 0.7 * 0.4 = 0.28
        // Identity: 1.0 * 0.3 = 0.3
        // Flavor: 0.8 * 0.3 = 0.24
        // Total: 0.82 (above pass threshold due to bonus fields)
        // The critical field is missing so we get a warning in issues
        assert!(validation.score > 0.75);
        assert!(!validation.issues.is_empty());
        assert!(validation
            .issues
            .iter()
            .any(|issue| issue.contains("title")));
    }

    #[tokio::test]
    async fn test_score_missing_artist() {
        let scorer = CompletenessScorer::new();
        let mut passage = create_complete_passage();
        passage.metadata.artist = None;

        let result = scorer.validate(&passage).await;
        assert!(result.is_ok());

        let validation = result.unwrap();
        // Metadata: title (0.5) + album bonus (0.1) + MBID bonus (0.1) = 0.7 → 0.7 * 0.4 = 0.28
        // Identity: 1.0 * 0.3 = 0.3
        // Flavor: 0.8 * 0.3 = 0.24
        // Total: 0.82 (above pass threshold due to bonus fields)
        assert!(validation.score > 0.75);
        assert!(!validation.issues.is_empty());
        assert!(validation
            .issues
            .iter()
            .any(|issue| issue.contains("artist")));
    }

    #[tokio::test]
    async fn test_score_missing_both_critical_fields() {
        let scorer = CompletenessScorer::new();
        let mut passage = create_complete_passage();
        passage.metadata.title = None;
        passage.metadata.artist = None;

        let result = scorer.validate(&passage).await;
        assert!(result.is_ok());

        let validation = result.unwrap();
        // Metadata: 0.0 (no critical) + album bonus (0.1) + MBID bonus (0.1) = 0.2 → 0.2 * 0.4 = 0.08
        // Identity: 1.0 * 0.3 = 0.3
        // Flavor: 0.8 * 0.3 = 0.24
        // Total: 0.62 (above warning threshold, below pass threshold)
        assert!(validation.score >= 0.6 && validation.score < 0.75);
        assert_eq!(validation.status, ValidationStatus::Warning);
        assert_eq!(
            validation.issues.iter().filter(|i| i.contains("Missing metadata")).count(),
            2
        );
    }

    #[tokio::test]
    async fn test_score_no_mbid() {
        let scorer = CompletenessScorer::new();
        let mut passage = create_complete_passage();
        passage.identity.recording_mbid = None;

        let result = scorer.validate(&passage).await;
        assert!(result.is_ok());

        let validation = result.unwrap();
        // Identity score = 0.0 (30% weight), so overall reduced by 0.3
        // Expected: 0.4 (metadata) + 0.0 (identity) + 0.24 (flavor 0.8*0.3) = 0.64
        assert!(validation.score < 0.75); // Below pass threshold
        assert!(!validation.issues.is_empty());
        assert!(validation
            .issues
            .iter()
            .any(|issue| issue.contains("MBID")));
    }

    #[tokio::test]
    async fn test_score_low_confidence_mbid() {
        let scorer = CompletenessScorer::new();
        let mut passage = create_complete_passage();
        passage.identity.confidence = 0.4; // Low confidence (<0.5)

        let result = scorer.validate(&passage).await;
        assert!(result.is_ok());

        let validation = result.unwrap();
        // Identity score = 0.7 (low confidence MBID)
        // Expected: 0.4 (metadata) + 0.21 (identity 0.7*0.3) + 0.24 (flavor) = 0.85
        // Should still pass but with reduced score
        assert!(validation.score >= 0.75);
    }

    #[tokio::test]
    async fn test_score_medium_confidence_mbid() {
        let scorer = CompletenessScorer::new();
        let mut passage = create_complete_passage();
        passage.identity.confidence = 0.6; // Medium confidence (0.5-0.8)

        let result = scorer.validate(&passage).await;
        assert!(result.is_ok());

        let validation = result.unwrap();
        // Identity score = 0.85 (medium confidence)
        assert!(validation.score >= 0.75);
    }

    #[tokio::test]
    async fn test_score_low_flavor_completeness() {
        let scorer = CompletenessScorer::new();
        let mut passage = create_complete_passage();
        passage.flavor.completeness = 0.2; // Below default min_flavor_completeness (0.3)

        let result = scorer.validate(&passage).await;
        assert!(result.is_ok());

        let validation = result.unwrap();
        // Flavor score = 0.2, should generate issue
        assert!(!validation.issues.is_empty());
        assert!(validation
            .issues
            .iter()
            .any(|issue| issue.contains("Insufficient flavor")));
    }

    #[tokio::test]
    async fn test_score_minimal_acceptable_passage() {
        let scorer = CompletenessScorer::new();
        let mut passage = create_complete_passage();

        // Minimal: title + artist + no MBID + minimal flavor
        passage.metadata.album = None;
        passage.metadata.recording_mbid = None;
        passage.identity.recording_mbid = None;
        passage.flavor.completeness = 0.3; // Exactly at minimum

        let result = scorer.validate(&passage).await;
        assert!(result.is_ok());

        let validation = result.unwrap();
        // Metadata: 1.0 (both critical fields) * 0.4 = 0.4
        // Identity: 0.0 (no MBID) * 0.3 = 0.0
        // Flavor: 0.3 * 0.3 = 0.09
        // Total: 0.49 (just below warning threshold)
        assert!(validation.score < 0.5);
        assert_eq!(validation.status, ValidationStatus::Fail);
    }

    #[tokio::test]
    async fn test_report_structure() {
        let scorer = CompletenessScorer::new();
        let passage = create_complete_passage();

        let result = scorer.validate(&passage).await;
        assert!(result.is_ok());

        let validation = result.unwrap();
        let report = &validation.report;

        // Verify report structure
        assert!(report["validator"].is_string());
        assert_eq!(report["validator"], "CompletenessScorer");
        assert!(report["overall_score"].is_f64());
        assert!(report["status"].is_string());
        assert!(report["dimensions"].is_object());
        assert!(report["dimensions"]["metadata"]["score"].is_f64());
        assert!(report["dimensions"]["identity"]["score"].is_f64());
        assert!(report["dimensions"]["flavor"]["score"].is_f64());
        assert!(report["thresholds"].is_object());
    }
}
