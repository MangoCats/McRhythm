// Tier 3 Validators - Quality Validation
//
// PLAN023: REQ-AI-060 series - Consistency checks and quality scoring
// 3 validators: Consistency Validator, Quality Scorer, Conflict Detector

pub mod consistency_validator;
pub mod quality_scorer;

use crate::fusion::{ExtractionResult, FusionResult, ValidationResult, ValidationCheck};
use anyhow::Result;
use tracing::debug;

/// Validate fused result for consistency and quality
///
/// # Arguments
/// * `fusion` - Fused data from Tier 2
/// * `extractions` - Original extraction results for cross-validation
///
/// # Returns
/// * `ValidationResult` with status, quality score, and detailed checks
pub async fn validate_fusion(
    fusion: &FusionResult,
    extractions: &[ExtractionResult],
) -> Result<ValidationResult> {
    debug!("Starting validation pipeline with {} extractions", extractions.len());

    let mut checks = Vec::new();
    let mut warnings = Vec::new();

    // REQ-AI-061: Title Consistency Check
    // Compare titles from different sources if multiple metadata extractions exist
    let metadata_sources: Vec<_> = extractions
        .iter()
        .filter_map(|e| e.metadata.as_ref())
        .collect();

    if metadata_sources.len() >= 2 {
        // Check first two titles
        if let (Some(title1), Some(title2)) = (
            metadata_sources[0].title.as_ref(),
            metadata_sources[1].title.as_ref(),
        ) {
            let check = consistency_validator::validate_title_consistency(title1, title2);
            checks.push(check);
        }
    }

    // REQ-AI-062: Duration Consistency Check
    // Compare durations if multiple sources provide them
    let durations: Vec<f64> = metadata_sources
        .iter()
        .filter_map(|m| m.duration_seconds)
        .collect();

    if durations.len() >= 2 {
        let check = consistency_validator::validate_duration_consistency(durations[0], durations[1]);
        checks.push(check);
    }

    // REQ-AI-063: Genre-Flavor Alignment (stub - non-critical)
    // Note: Full implementation pending (see consistency_validator.rs for details)
    if let Some(_title) = &fusion.metadata.title {
        // Extract genre from ID3 if available (simplified - just check if we have it)
        let genre_check = consistency_validator::validate_genre_flavor_alignment(
            "Unknown", // Placeholder - would need to extract from ID3 metadata
            &fusion.flavor,
        );
        checks.push(genre_check);
    }

    // Additional Check: Metadata Completeness
    let completeness_check = ValidationCheck {
        name: "Metadata Completeness".to_string(),
        passed: fusion.metadata.completeness >= 0.5,
        score: Some(fusion.metadata.completeness),
        message: if fusion.metadata.completeness < 0.5 {
            Some(format!(
                "Metadata is incomplete ({:.0}% complete)",
                fusion.metadata.completeness * 100.0
            ))
        } else {
            None
        },
    };
    checks.push(completeness_check);

    // Additional Check: Flavor Completeness
    let flavor_check = ValidationCheck {
        name: "Flavor Completeness".to_string(),
        passed: fusion.flavor.completeness >= 0.5,
        score: Some(fusion.flavor.completeness),
        message: if fusion.flavor.completeness < 0.5 {
            Some(format!(
                "Musical flavor is incomplete ({:.0}% of expected characteristics)",
                fusion.flavor.completeness * 100.0
            ))
        } else {
            None
        },
    };
    checks.push(flavor_check);

    // Additional Check: Identity Confidence
    let identity_check = ValidationCheck {
        name: "Identity Confidence".to_string(),
        passed: fusion.identity.confidence >= 0.7,
        score: Some(fusion.identity.confidence),
        message: if fusion.identity.confidence < 0.7 {
            Some(format!(
                "Identity confidence is low ({:.0}%)",
                fusion.identity.confidence * 100.0
            ))
        } else {
            None
        },
    };
    checks.push(identity_check);

    // Check for conflicts
    if !fusion.metadata.conflicts.is_empty() {
        warnings.push(format!(
            "{} metadata conflict(s) detected",
            fusion.metadata.conflicts.len()
        ));
    }

    if !fusion.identity.conflicts.is_empty() {
        warnings.push(format!(
            "{} identity conflict(s) detected",
            fusion.identity.conflicts.len()
        ));
    }

    // REQ-AI-064: Calculate overall quality score
    let (quality_score, status) = quality_scorer::calculate_quality_score(&checks);

    debug!(
        "Validation complete: {} checks, quality={:.1}%, status={:?}",
        checks.len(),
        quality_score,
        status
    );

    Ok(ValidationResult {
        status,
        quality_score,
        checks,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fusion::{
        ExtractionResult, FusedFlavor, FusedIdentity, FusedMetadata, IdentityExtraction,
        MetadataExtraction, MusicalFlavor, ValidationStatus,
    };
    use std::collections::HashMap;

    fn create_test_fusion_high_quality() -> FusionResult {
        FusionResult {
            metadata: FusedMetadata {
                title: Some("Test Song".to_string()),
                title_source: Some("ID3".to_string()),
                title_confidence: Some(0.95),
                artist: Some("Test Artist".to_string()),
                artist_source: Some("ID3".to_string()),
                artist_confidence: Some(0.90),
                album: Some("Test Album".to_string()),
                completeness: 0.85,
                conflicts: vec![],
            },
            flavor: FusedFlavor {
                characteristics: MusicalFlavor::default(),
                source_blend: vec!["Essentia:0.9".to_string()],
                confidence_map: HashMap::new(),
                completeness: 0.80,
            },
            identity: FusedIdentity {
                recording_mbid: Some("test-mbid-123".to_string()),
                confidence: 0.92,
                conflicts: vec![],
            },
        }
    }

    fn create_test_fusion_low_quality() -> FusionResult {
        FusionResult {
            metadata: FusedMetadata {
                title: Some("Test Song".to_string()),
                title_source: Some("ID3".to_string()),
                title_confidence: Some(0.40),
                artist: None,
                artist_source: None,
                artist_confidence: None,
                album: None,
                completeness: 0.30, // Low completeness
                conflicts: vec![],
            },
            flavor: FusedFlavor {
                characteristics: MusicalFlavor::default(),
                source_blend: vec![],
                confidence_map: HashMap::new(),
                completeness: 0.25, // Low completeness
            },
            identity: FusedIdentity {
                recording_mbid: None,
                confidence: 0.40, // Low confidence
                conflicts: vec![],
            },
        }
    }

    fn create_test_extractions_matching() -> Vec<ExtractionResult> {
        vec![
            ExtractionResult {
                source: "ID3".to_string(),
                confidence: 0.9,
                timestamp: 1234567890,
                metadata: Some(MetadataExtraction {
                    title: Some("Test Song".to_string()),
                    artist: Some("Test Artist".to_string()),
                    album: Some("Test Album".to_string()),
                    duration_seconds: Some(180.0),
                    title_confidence: Some(0.9),
                    artist_confidence: Some(0.9),
                }),
                flavor: None,
                identity: None,
            },
            ExtractionResult {
                source: "AcoustID".to_string(),
                confidence: 0.85,
                timestamp: 1234567891,
                metadata: Some(MetadataExtraction {
                    title: Some("Test Song".to_string()),
                    artist: Some("Test Artist".to_string()),
                    album: None,
                    duration_seconds: Some(181.0), // Slightly different duration
                    title_confidence: Some(0.85),
                    artist_confidence: Some(0.85),
                }),
                flavor: None,
                identity: Some(IdentityExtraction {
                    recording_mbid: "test-mbid-123".to_string(),
                    confidence: 0.85,
                    context: None,
                }),
            },
        ]
    }

    fn create_test_extractions_conflicting() -> Vec<ExtractionResult> {
        vec![
            ExtractionResult {
                source: "ID3".to_string(),
                confidence: 0.9,
                timestamp: 1234567890,
                metadata: Some(MetadataExtraction {
                    title: Some("Test Song A".to_string()),
                    artist: Some("Artist One".to_string()),
                    album: None,
                    duration_seconds: Some(180.0),
                    title_confidence: Some(0.9),
                    artist_confidence: Some(0.9),
                }),
                flavor: None,
                identity: None,
            },
            ExtractionResult {
                source: "AcoustID".to_string(),
                confidence: 0.85,
                timestamp: 1234567891,
                metadata: Some(MetadataExtraction {
                    title: Some("Different Song".to_string()),
                    artist: Some("Artist Two".to_string()),
                    album: None,
                    duration_seconds: Some(240.0), // Very different duration
                    title_confidence: Some(0.85),
                    artist_confidence: Some(0.85),
                }),
                flavor: None,
                identity: None,
            },
        ]
    }

    #[tokio::test]
    async fn test_validate_fusion_high_quality() {
        let fusion = create_test_fusion_high_quality();
        let extractions = create_test_extractions_matching();

        let result = validate_fusion(&fusion, &extractions).await;
        assert!(result.is_ok());

        let validation = result.unwrap();
        assert_eq!(validation.status, ValidationStatus::Pass);
        assert!(validation.quality_score >= 80.0);
        assert!(!validation.checks.is_empty());
    }

    #[tokio::test]
    async fn test_validate_fusion_low_quality() {
        let fusion = create_test_fusion_low_quality();
        let extractions = create_test_extractions_matching();

        let result = validate_fusion(&fusion, &extractions).await;
        assert!(result.is_ok());

        let validation = result.unwrap();
        // Should be Warning or Fail due to low completeness and confidence
        assert!(validation.status == ValidationStatus::Warning || validation.status == ValidationStatus::Fail);
        assert!(validation.quality_score < 80.0);
    }

    #[tokio::test]
    async fn test_validate_fusion_title_consistency() {
        let fusion = create_test_fusion_high_quality();
        let extractions = create_test_extractions_matching();

        let result = validate_fusion(&fusion, &extractions).await;
        assert!(result.is_ok());

        let validation = result.unwrap();
        // Check that title consistency check was performed
        let title_check = validation.checks.iter().find(|c| c.name == "Title Consistency");
        assert!(title_check.is_some());

        // Titles match ("Test Song" in both extractions)
        let check = title_check.unwrap();
        assert!(check.passed);
    }

    #[tokio::test]
    async fn test_validate_fusion_duration_consistency() {
        let fusion = create_test_fusion_high_quality();
        let extractions = create_test_extractions_matching();

        let result = validate_fusion(&fusion, &extractions).await;
        assert!(result.is_ok());

        let validation = result.unwrap();
        // Check that duration consistency check was performed
        let duration_check = validation.checks.iter().find(|c| c.name == "Duration Consistency");
        assert!(duration_check.is_some());

        // Durations are close (180.0 vs 181.0)
        let check = duration_check.unwrap();
        assert!(check.passed);
    }

    #[tokio::test]
    async fn test_validate_fusion_conflicting_data() {
        let fusion = create_test_fusion_high_quality();
        let extractions = create_test_extractions_conflicting();

        let result = validate_fusion(&fusion, &extractions).await;
        assert!(result.is_ok());

        let validation = result.unwrap();
        // Should have failing checks due to conflicting titles and durations
        let title_check = validation.checks.iter().find(|c| c.name == "Title Consistency");
        if let Some(check) = title_check {
            // "Test Song A" vs "Different Song" should fail consistency
            assert!(!check.passed);
        }

        let duration_check = validation.checks.iter().find(|c| c.name == "Duration Consistency");
        if let Some(check) = duration_check {
            // 180.0 vs 240.0 should fail consistency (>10% difference)
            assert!(!check.passed);
        }
    }

    #[tokio::test]
    async fn test_validate_fusion_completeness_checks() {
        let fusion = create_test_fusion_high_quality();
        let extractions = vec![]; // Empty extractions

        let result = validate_fusion(&fusion, &extractions).await;
        assert!(result.is_ok());

        let validation = result.unwrap();

        // Should have completeness checks
        let metadata_check = validation.checks.iter().find(|c| c.name == "Metadata Completeness");
        assert!(metadata_check.is_some());
        assert!(metadata_check.unwrap().passed); // 0.85 >= 0.5

        let flavor_check = validation.checks.iter().find(|c| c.name == "Flavor Completeness");
        assert!(flavor_check.is_some());
        assert!(flavor_check.unwrap().passed); // 0.80 >= 0.5

        let identity_check = validation.checks.iter().find(|c| c.name == "Identity Confidence");
        assert!(identity_check.is_some());
        assert!(identity_check.unwrap().passed); // 0.92 >= 0.7
    }

    #[tokio::test]
    async fn test_validate_fusion_no_extractions() {
        let fusion = create_test_fusion_high_quality();
        let extractions = vec![];

        let result = validate_fusion(&fusion, &extractions).await;
        assert!(result.is_ok());

        let validation = result.unwrap();
        // No title/duration consistency checks with <2 extractions
        // But completeness checks should still run
        assert!(!validation.checks.is_empty());
        assert!(validation.quality_score > 0.0);
    }
}
