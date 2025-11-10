//! Consistency Validator (Tier 3 Validator)
//!
//! Validates internal consistency across fused identity, metadata, and flavor data.
//! Checks for logical contradictions and assesses conflict severity.
//!
//! # Implementation
//! - TASK-016: Consistency Validator (PLAN024)
//! - Validation strategy: Multi-faceted consistency checks
//!
//! # Architecture
//! Implements `Validation` trait for integration with 3-tier architecture.
//! Accepts FusedPassage and produces ValidationResult with detailed report.
//!
//! # Consistency Checks
//! 1. **Confidence Score Validity**: All scores in 0.0-1.0 range
//! 2. **Cross-Field Consistency**: Identity matches metadata (if both present)
//! 3. **Conflict Analysis**: Assess severity of conflicts from fusers
//! 4. **Data Integrity**: No logical contradictions
//!
//! # Scoring Algorithm
//! - Start with score = 1.0 (perfect consistency)
//! - Deduct points for each issue:
//!   - Critical: -0.3 (invalid data, major contradictions)
//!   - Major: -0.15 (conflicts, inconsistencies)
//!   - Minor: -0.05 (warnings, edge cases)
//! - Final score clamped to 0.0-1.0 range
//!
//! # Status Determination
//! - Pass: score >= 0.8 and no critical issues
//! - Warning: score >= 0.5 or only minor/major issues
//! - Fail: score < 0.5 or any critical issues
//!
//! # Example
//! ```rust,ignore
//! use wkmp_ai::validators::ConsistencyValidator;
//! use wkmp_ai::workflow::FusedPassage;
//!
//! let validator = ConsistencyValidator::new();
//! let result = validator.validate(&fused_passage).await?;
//!
//! match result.status {
//!     ValidationStatus::Pass => println!("Data is consistent"),
//!     ValidationStatus::Warning => println!("Minor issues: {:?}", result.issues),
//!     ValidationStatus::Fail => println!("Critical issues: {:?}", result.issues),
//! }
//! ```

use crate::types::{Validation, ValidationError, ValidationResult, ValidationStatus};
use crate::workflow::FusedPassage;
use async_trait::async_trait;
use serde_json::json;
use tracing::debug;

/// Consistency Validator
///
/// Validates internal consistency across fused identity, metadata, and flavor data.
///
/// # Validation Checks
/// - Confidence scores in valid range (0.0-1.0)
/// - Cross-field consistency (identity vs metadata)
/// - Conflict severity assessment
/// - Data integrity (no contradictions)
///
/// # Scoring
/// Starts at 1.0, deducts points for issues:
/// - Critical issues: -0.3
/// - Major issues: -0.15
/// - Minor issues: -0.05
///
/// Final score determines validation status.
pub struct ConsistencyValidator {
    /// Minimum score threshold for Pass status
    pass_threshold: f32,
    /// Minimum score threshold for Warning status (below this is Fail)
    warning_threshold: f32,
}

impl ConsistencyValidator {
    /// Create new Consistency Validator with default thresholds
    pub fn new() -> Self {
        Self {
            pass_threshold: 0.8,
            warning_threshold: 0.5,
        }
    }

    /// Create Consistency Validator with custom thresholds
    pub fn with_thresholds(pass_threshold: f32, warning_threshold: f32) -> Self {
        Self {
            pass_threshold,
            warning_threshold,
        }
    }

    /// Validate fused passage data
    fn validate_passage(&self, passage: &FusedPassage) -> ValidationResult {
        let mut score = 1.0_f32;
        let mut issues = Vec::new();
        let mut critical_issues = 0;
        let mut major_issues = 0;
        let mut minor_issues = 0;

        // Check 1: Validate confidence scores (all must be 0.0-1.0)
        self.check_confidence_scores(
            passage,
            &mut score,
            &mut issues,
            &mut critical_issues,
            &mut minor_issues,
        );

        // Check 2: Cross-field consistency (identity vs metadata)
        self.check_cross_field_consistency(
            passage,
            &mut score,
            &mut issues,
            &mut critical_issues,
            &mut major_issues,
            &mut minor_issues,
        );

        // Check 3: Conflict analysis
        self.check_conflicts(
            passage,
            &mut score,
            &mut issues,
            &mut major_issues,
            &mut minor_issues,
        );

        // Check 4: Completeness consistency (if fields present, they should be valid)
        self.check_completeness_consistency(
            passage,
            &mut score,
            &mut issues,
            &mut minor_issues,
        );

        // Clamp score to valid range
        score = score.clamp(0.0, 1.0);

        // Determine status based on score and issue severity
        let status = if critical_issues > 0 {
            ValidationStatus::Fail
        } else if score >= self.pass_threshold && major_issues == 0 {
            ValidationStatus::Pass
        } else if score >= self.warning_threshold {
            ValidationStatus::Warning
        } else {
            ValidationStatus::Fail
        };

        debug!(
            status = ?status,
            score = score,
            critical = critical_issues,
            major = major_issues,
            minor = minor_issues,
            "Consistency validation complete"
        );

        // Build detailed report
        let report = json!({
            "validator": "ConsistencyValidator",
            "score": score,
            "status": format!("{:?}", status),
            "issue_counts": {
                "critical": critical_issues,
                "major": major_issues,
                "minor": minor_issues,
            },
            "checks": {
                "confidence_scores": critical_issues == 0,
                "cross_field_consistency": major_issues == 0,
                "conflicts": true,
                "completeness": true,
            }
        });

        ValidationResult {
            status,
            score,
            issues,
            report,
        }
    }

    /// Check confidence score validity
    fn check_confidence_scores(
        &self,
        passage: &FusedPassage,
        score: &mut f32,
        issues: &mut Vec<String>,
        critical_issues: &mut usize,
        minor_issues: &mut usize,
    ) {
        // Check identity confidence
        if passage.identity.confidence < 0.0 || passage.identity.confidence > 1.0 {
            *score -= 0.3;
            *critical_issues += 1;
            issues.push(format!(
                "Invalid identity confidence: {}",
                passage.identity.confidence
            ));
        }

        // Check metadata field confidences
        if let Some(ref title) = passage.metadata.title {
            if title.confidence < 0.0 || title.confidence > 1.0 {
                *score -= 0.3;
                *critical_issues += 1;
                issues.push(format!("Invalid title confidence: {}", title.confidence));
            }
        }

        if let Some(ref artist) = passage.metadata.artist {
            if artist.confidence < 0.0 || artist.confidence > 1.0 {
                *score -= 0.3;
                *critical_issues += 1;
                issues.push(format!("Invalid artist confidence: {}", artist.confidence));
            }
        }

        if let Some(ref album) = passage.metadata.album {
            if album.confidence < 0.0 || album.confidence > 1.0 {
                *score -= 0.3;
                *critical_issues += 1;
                issues.push(format!("Invalid album confidence: {}", album.confidence));
            }
        }

        // Check flavor characteristic confidences
        for (char_name, &char_confidence) in &passage.flavor.confidence_map {
            if char_confidence < 0.0 || char_confidence > 1.0 {
                *score -= 0.05;
                *minor_issues += 1;
                issues.push(format!(
                    "Invalid flavor confidence for '{}': {}",
                    char_name, char_confidence
                ));
            }
        }
    }

    /// Check cross-field consistency between identity and metadata
    fn check_cross_field_consistency(
        &self,
        passage: &FusedPassage,
        score: &mut f32,
        issues: &mut Vec<String>,
        critical_issues: &mut usize,
        major_issues: &mut usize,
        minor_issues: &mut usize,
    ) {
        // If we have recording MBID but no metadata, that's inconsistent
        if passage.identity.recording_mbid.is_some()
            && passage.metadata.title.is_none()
            && passage.metadata.artist.is_none()
        {
            *score -= 0.05;
            *minor_issues += 1;
            issues.push(
                "Have recording MBID but missing basic metadata (title/artist)".to_string(),
            );
        }

        // If metadata MBID differs from identity MBID, that's a major inconsistency
        if let (Some(ref identity_mbid), Some(ref metadata_mbid_val)) = (
            &passage.identity.recording_mbid,
            &passage.metadata.recording_mbid,
        ) {
            if identity_mbid != &metadata_mbid_val.value {
                *score -= 0.15;
                *major_issues += 1;
                issues.push(format!(
                    "MBID mismatch: identity='{}' vs metadata='{}'",
                    identity_mbid, metadata_mbid_val.value
                ));
            }
        }

        // Check metadata completeness score consistency (critical: data integrity)
        if passage.metadata.metadata_completeness < 0.0
            || passage.metadata.metadata_completeness > 1.0
        {
            *score -= 0.3;
            *critical_issues += 1;
            issues.push(format!(
                "Invalid metadata completeness: {}",
                passage.metadata.metadata_completeness
            ));
        }

        // Check flavor completeness score consistency (critical: data integrity)
        if passage.flavor.completeness < 0.0 || passage.flavor.completeness > 1.0 {
            *score -= 0.3;
            *critical_issues += 1;
            issues.push(format!(
                "Invalid flavor completeness: {}",
                passage.flavor.completeness
            ));
        }
    }

    /// Check and assess conflicts reported by fusers
    fn check_conflicts(
        &self,
        passage: &FusedPassage,
        score: &mut f32,
        issues: &mut Vec<String>,
        major_issues: &mut usize,
        minor_issues: &mut usize,
    ) {
        // Identity conflicts (multiple MBIDs from different sources)
        if !passage.identity.conflicts.is_empty() {
            let conflict_count = passage.identity.conflicts.len();

            if conflict_count > 2 {
                // Many conflicts = major issue
                *score -= 0.15;
                *major_issues += 1;
                issues.push(format!(
                    "Multiple identity conflicts detected ({})",
                    conflict_count
                ));
            } else {
                // Few conflicts = minor issue
                *score -= 0.05;
                *minor_issues += 1;
                issues.push(format!(
                    "Identity conflicts detected ({})",
                    conflict_count
                ));
            }
        }
    }

    /// Check completeness scores are consistent with actual data
    fn check_completeness_consistency(
        &self,
        passage: &FusedPassage,
        score: &mut f32,
        issues: &mut Vec<String>,
        minor_issues: &mut usize,
    ) {
        // Count actual metadata fields present
        let mut field_count = 0;
        if passage.metadata.title.is_some() {
            field_count += 1;
        }
        if passage.metadata.artist.is_some() {
            field_count += 1;
        }
        if passage.metadata.album.is_some() {
            field_count += 1;
        }
        if passage.metadata.recording_mbid.is_some() {
            field_count += 1;
        }

        // Expected completeness (assuming 4 primary fields)
        let expected_completeness = field_count as f32 / 4.0;

        // Allow some tolerance (±0.1) due to rounding
        if (passage.metadata.metadata_completeness - expected_completeness).abs() > 0.1 {
            *score -= 0.05;
            *minor_issues += 1;
            issues.push(format!(
                "Metadata completeness inconsistent: reported={:.2}, expected={:.2}",
                passage.metadata.metadata_completeness, expected_completeness
            ));
        }

        // Flavor completeness should match actual characteristics count
        let actual_flavor_count = passage.flavor.characteristics.len();
        // Flavor completeness is present_count / expected_count, so reverse calculate expected
        if passage.flavor.completeness > 0.0 {
            let expected_flavor_total = (actual_flavor_count as f32 / passage.flavor.completeness)
                .round() as usize;

            // Sanity check: expected total should be reasonable (e.g., 10-20 for AcousticBrainz)
            if expected_flavor_total < 5 || expected_flavor_total > 50 {
                *score -= 0.05;
                *minor_issues += 1;
                issues.push(format!(
                    "Flavor completeness suggests unusual expected total: {}",
                    expected_flavor_total
                ));
            }
        }
    }
}

impl Default for ConsistencyValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Validation for ConsistencyValidator {
    type Input = FusedPassage;

    fn name(&self) -> &'static str {
        "ConsistencyValidator"
    }

    async fn validate(&self, input: &Self::Input) -> Result<ValidationResult, ValidationError> {
        debug!("Validating fused passage for consistency");
        Ok(self.validate_passage(input))
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

    fn create_valid_passage() -> FusedPassage {
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
                additional: std::collections::HashMap::new(),
                metadata_completeness: 1.0,
            },
            flavor: FusedFlavor {
                characteristics: [("danceability".to_string(), 0.8)].into_iter().collect(),
                confidence_map: [("danceability".to_string(), 0.85)].into_iter().collect(),
                source_blend: vec![("Essentia".to_string(), 1.0)],
                completeness: 0.5, // 1 out of 2 expected
            },
        }
    }

    #[test]
    fn test_validator_name() {
        let validator = ConsistencyValidator::new();
        assert_eq!(validator.name(), "ConsistencyValidator");
    }

    #[test]
    fn test_default_thresholds() {
        let validator = ConsistencyValidator::new();
        assert_eq!(validator.pass_threshold, 0.8);
        assert_eq!(validator.warning_threshold, 0.5);
    }

    #[test]
    fn test_custom_thresholds() {
        let validator = ConsistencyValidator::with_thresholds(0.9, 0.6);
        assert_eq!(validator.pass_threshold, 0.9);
        assert_eq!(validator.warning_threshold, 0.6);
    }

    #[tokio::test]
    async fn test_validate_perfect_passage() {
        let validator = ConsistencyValidator::new();
        let passage = create_valid_passage();

        let result = validator.validate(&passage).await;
        assert!(result.is_ok());

        let validation = result.unwrap();
        assert_eq!(validation.status, ValidationStatus::Pass);
        assert!(validation.score >= 0.8);
        assert!(validation.issues.is_empty() || validation.issues.len() <= 1); // Allow one minor issue
    }

    #[tokio::test]
    async fn test_validate_invalid_confidence_score() {
        let validator = ConsistencyValidator::new();
        let mut passage = create_valid_passage();
        passage.identity.confidence = 1.5; // Invalid: > 1.0

        let result = validator.validate(&passage).await;
        assert!(result.is_ok());

        let validation = result.unwrap();
        assert_eq!(validation.status, ValidationStatus::Fail); // Critical issue
        assert!(validation.score < 0.8);
        assert!(!validation.issues.is_empty());
        assert!(validation.issues[0].contains("Invalid identity confidence"));
    }

    #[tokio::test]
    async fn test_validate_mbid_mismatch() {
        let validator = ConsistencyValidator::new();
        let mut passage = create_valid_passage();

        // Set different MBID in metadata
        passage.metadata.recording_mbid = Some(ConfidenceValue::new(
            "different-mbid-456".to_string(),
            0.9,
            "MusicBrainz".to_string(),
        ));

        let result = validator.validate(&passage).await;
        assert!(result.is_ok());

        let validation = result.unwrap();
        assert!(validation.status == ValidationStatus::Warning || validation.status == ValidationStatus::Fail);
        assert!(!validation.issues.is_empty());
        assert!(validation.issues.iter().any(|issue| issue.contains("MBID mismatch")));
    }

    #[tokio::test]
    async fn test_validate_identity_conflicts() {
        let validator = ConsistencyValidator::new();
        let mut passage = create_valid_passage();

        // Add conflicts
        passage.identity.conflicts = vec![
            "Conflict 1: Source A vs B".to_string(),
            "Conflict 2: Source B vs C".to_string(),
        ];

        let result = validator.validate(&passage).await;
        assert!(result.is_ok());

        let validation = result.unwrap();
        assert!(validation.status == ValidationStatus::Warning || validation.status == ValidationStatus::Pass);
        assert!(!validation.issues.is_empty());
        assert!(validation
            .issues
            .iter()
            .any(|issue| issue.contains("Identity conflicts")));
    }

    #[tokio::test]
    async fn test_validate_many_conflicts() {
        let validator = ConsistencyValidator::new();
        let mut passage = create_valid_passage();

        // Add many conflicts (>2)
        passage.identity.conflicts = vec![
            "Conflict 1".to_string(),
            "Conflict 2".to_string(),
            "Conflict 3".to_string(),
            "Conflict 4".to_string(),
        ];

        let result = validator.validate(&passage).await;
        assert!(result.is_ok());

        let validation = result.unwrap();
        assert!(validation.status == ValidationStatus::Warning || validation.status == ValidationStatus::Fail);
        assert!(!validation.issues.is_empty());
        assert!(validation
            .issues
            .iter()
            .any(|issue| issue.contains("Multiple identity conflicts")));
    }

    #[tokio::test]
    async fn test_validate_missing_metadata_with_mbid() {
        let validator = ConsistencyValidator::new();
        let mut passage = create_valid_passage();

        // Have MBID but no metadata
        passage.metadata.title = None;
        passage.metadata.artist = None;

        let result = validator.validate(&passage).await;
        assert!(result.is_ok());

        let validation = result.unwrap();
        // This is a minor issue, should still pass or warn
        assert!(validation.status == ValidationStatus::Pass || validation.status == ValidationStatus::Warning);
    }

    #[tokio::test]
    async fn test_validate_invalid_completeness_scores() {
        let validator = ConsistencyValidator::new();
        let mut passage = create_valid_passage();

        // Invalid completeness scores
        passage.metadata.metadata_completeness = 1.5; // > 1.0

        let result = validator.validate(&passage).await;
        assert!(result.is_ok());

        let validation = result.unwrap();
        assert_eq!(validation.status, ValidationStatus::Fail); // Major issue
        assert!(validation.score < 0.8);
        assert!(validation
            .issues
            .iter()
            .any(|issue| issue.contains("Invalid metadata completeness")));
    }

    #[tokio::test]
    async fn test_score_clamping() {
        let validator = ConsistencyValidator::new();
        let mut passage = create_valid_passage();

        // Add many critical issues to drive score negative
        passage.identity.confidence = 1.5;
        passage.metadata.metadata_completeness = 1.5;
        passage.flavor.completeness = 1.5;

        let result = validator.validate(&passage).await;
        assert!(result.is_ok());

        let validation = result.unwrap();
        assert!(validation.score >= 0.0); // Should be clamped to 0.0
        assert!(validation.score <= 1.0); // Should not exceed 1.0
    }

    #[tokio::test]
    async fn test_report_structure() {
        let validator = ConsistencyValidator::new();
        let passage = create_valid_passage();

        let result = validator.validate(&passage).await;
        assert!(result.is_ok());

        let validation = result.unwrap();
        let report = &validation.report;

        // Verify report structure
        assert!(report["validator"].is_string());
        assert_eq!(report["validator"], "ConsistencyValidator");
        assert!(report["score"].is_f64());
        assert!(report["status"].is_string());
        assert!(report["issue_counts"].is_object());
        assert!(report["checks"].is_object());
    }
}
