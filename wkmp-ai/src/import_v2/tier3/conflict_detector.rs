// PLAN023 Tier 3: Conflict Detector
//
// Concept: High-level conflict detection and aggregation across validation results
// Synchronization: Accepts validation outputs from all Tier 3 validators, outputs ValidationReport
//
// Algorithm (per SPEC_wkmp_ai_recode.md):
// 1. Collect conflicts from ConsistencyChecker
// 2. Check completeness score from CompletenessScorer
// 3. Aggregate all warnings and conflicts
// 4. Determine overall quality score and conflict status
// 5. Return ValidationReport with actionable feedback

use crate::import_v2::types::{ConflictSeverity, FusedMetadata, ValidationReport};

/// Conflict detector (Tier 3 validation concept)
///
/// **Legible Software Principle:**
/// - Independent module: Pure aggregation logic, no side effects
/// - Explicit synchronization: Clear contract with other Tier 3 validators
/// - Transparent behavior: Conflict detection rules are explicit
/// - Integrity: Produces actionable ValidationReport for decision-making
pub struct ConflictDetector {
    /// Minimum quality score to avoid warnings
    min_quality_score: f64,
    /// Minimum confidence for required fields
    min_required_confidence: f64,
}

impl Default for ConflictDetector {
    fn default() -> Self {
        Self {
            min_quality_score: 0.5,    // Warn if quality < 50%
            min_required_confidence: 0.6, // Warn if title/artist confidence < 60%
        }
    }
}

impl ConflictDetector {
    /// Detect conflicts and generate validation report
    ///
    /// # Algorithm: Multi-Source Validation Aggregation
    /// 1. Check completeness (quality score)
    /// 2. Check required fields (title, artist presence and confidence)
    /// 3. Aggregate consistency conflicts from field comparisons
    /// 4. Determine overall conflict status
    /// 5. Generate actionable warnings
    ///
    /// # Arguments
    /// * `metadata` - Fused metadata to validate
    /// * `quality_score` - Score from CompletenessScorer [0.0, 1.0]
    /// * `consistency_conflicts` - Conflicts from ConsistencyChecker
    ///
    /// # Returns
    /// ValidationReport with quality score, conflict status, and warnings
    pub fn detect(
        &self,
        metadata: &FusedMetadata,
        quality_score: f64,
        consistency_conflicts: Vec<(String, ConflictSeverity)>,
    ) -> ValidationReport {
        let mut warnings: Vec<String> = Vec::new();
        let mut conflicts: Vec<(String, ConflictSeverity)> = Vec::new();

        // Step 1: Check quality score
        if quality_score < self.min_quality_score {
            warnings.push(format!(
                "Low metadata quality: {:.1}% completeness (minimum: {:.0}%)",
                quality_score * 100.0,
                self.min_quality_score * 100.0
            ));
        }

        // Step 2: Check required fields (use High severity for missing required fields)
        if metadata.title.is_none() {
            conflicts.push((
                "Missing required field: title".to_string(),
                ConflictSeverity::High,
            ));
        } else if let Some(ref title) = metadata.title {
            if title.confidence < self.min_required_confidence {
                warnings.push(format!(
                    "Low confidence for title: {:.1}% (minimum: {:.0}%)",
                    title.confidence * 100.0,
                    self.min_required_confidence * 100.0
                ));
            }
        }

        if metadata.artist.is_none() {
            conflicts.push((
                "Missing required field: artist".to_string(),
                ConflictSeverity::High,
            ));
        } else if let Some(ref artist) = metadata.artist {
            if artist.confidence < self.min_required_confidence {
                warnings.push(format!(
                    "Low confidence for artist: {:.1}% (minimum: {:.0}%)",
                    artist.confidence * 100.0,
                    self.min_required_confidence * 100.0
                ));
            }
        }

        // Step 3: Aggregate consistency conflicts
        conflicts.extend(consistency_conflicts);

        // Step 4: Generate informational warnings for missing optional fields
        if metadata.album.is_none() {
            warnings.push("Optional field missing: album".to_string());
        }

        if metadata.release_date.is_none() {
            warnings.push("Optional field missing: release_date".to_string());
        }

        // Step 5: Determine overall conflict status
        let has_conflicts = !conflicts.is_empty();

        tracing::info!(
            "Conflict detection: quality={:.3}, conflicts={}, warnings={}",
            quality_score,
            conflicts.len(),
            warnings.len()
        );

        ValidationReport {
            quality_score,
            has_conflicts,
            warnings,
            conflicts,
        }
    }

    /// Count conflicts by severity
    pub fn count_by_severity(
        &self,
        conflicts: &[(String, ConflictSeverity)],
    ) -> (usize, usize, usize) {
        let mut high = 0;
        let mut medium = 0;
        let mut low = 0;

        for (_, severity) in conflicts {
            match severity {
                ConflictSeverity::High => high += 1,
                ConflictSeverity::Medium => medium += 1,
                ConflictSeverity::Low => low += 1,
            }
        }

        (high, medium, low)
    }

    /// Check if validation is acceptable (no high-severity conflicts from missing required fields)
    pub fn is_acceptable(&self, report: &ValidationReport) -> bool {
        // Check if any conflicts are about missing required fields (High severity)
        !report.conflicts.iter().any(|(msg, severity)| {
            *severity == ConflictSeverity::High && msg.contains("Missing required field")
        })
    }

    /// Get summary message from validation report
    pub fn summary_message(&self, report: &ValidationReport) -> String {
        if !report.has_conflicts && report.warnings.is_empty() {
            return format!(
                "Validation passed: {:.1}% quality",
                report.quality_score * 100.0
            );
        }

        let (high, medium, low) = self.count_by_severity(&report.conflicts);

        if high > 0 {
            format!(
                "Validation failed: {} high-severity conflict(s), {:.1}% quality",
                high,
                report.quality_score * 100.0
            )
        } else if medium > 0 {
            format!(
                "Validation warning: {} medium-severity conflict(s), {:.1}% quality",
                medium,
                report.quality_score * 100.0
            )
        } else if low > 0 {
            format!(
                "Validation passed with {} low-severity conflict(s), {:.1}% quality",
                low,
                report.quality_score * 100.0
            )
        } else {
            format!(
                "Validation passed: {} warning(s), {:.1}% quality",
                report.warnings.len(),
                report.quality_score * 100.0
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import_v2::types::{ExtractionSource, MetadataField};

    fn create_field<T>(value: T, confidence: f64) -> MetadataField<T> {
        MetadataField {
            value,
            confidence,
            source: ExtractionSource::MusicBrainz,
        }
    }

    #[test]
    fn test_perfect_metadata() {
        let detector = ConflictDetector::default();

        let metadata = FusedMetadata {
            title: Some(create_field("Let It Be".to_string(), 0.95)),
            artist: Some(create_field("The Beatles".to_string(), 0.95)),
            album: Some(create_field("Let It Be".to_string(), 0.9)),
            release_date: Some(create_field("1970-05-08".to_string(), 0.85)),
            track_number: Some(create_field(6, 0.8)),
            duration_ms: Some(create_field(240000, 0.9)),
            metadata_confidence: 0.9,
        };

        let report = detector.detect(&metadata, 0.9, vec![]);

        assert_eq!(report.quality_score, 0.9);
        assert!(!report.has_conflicts);
        assert_eq!(report.conflicts.len(), 0);
        assert_eq!(report.warnings.len(), 0); // No warnings for high-quality metadata
        assert!(detector.is_acceptable(&report));
    }

    #[test]
    fn test_missing_title_critical() {
        let detector = ConflictDetector::default();

        let metadata = FusedMetadata {
            title: None, // Critical missing field
            artist: Some(create_field("The Beatles".to_string(), 0.9)),
            album: None,
            release_date: None,
            track_number: None,
            duration_ms: None,
            metadata_confidence: 0.5,
        };

        let report = detector.detect(&metadata, 0.5, vec![]);

        assert!(report.has_conflicts);
        assert_eq!(report.conflicts.len(), 1);
        assert_eq!(
            report.conflicts[0].0,
            "Missing required field: title"
        );
        assert_eq!(report.conflicts[0].1, ConflictSeverity::High);
        assert!(!detector.is_acceptable(&report)); // Missing required field = not acceptable
    }

    #[test]
    fn test_missing_artist_critical() {
        let detector = ConflictDetector::default();

        let metadata = FusedMetadata {
            title: Some(create_field("Let It Be".to_string(), 0.9)),
            artist: None, // Critical missing field
            album: None,
            release_date: None,
            track_number: None,
            duration_ms: None,
            metadata_confidence: 0.5,
        };

        let report = detector.detect(&metadata, 0.5, vec![]);

        assert!(report.has_conflicts);
        assert_eq!(report.conflicts.len(), 1);
        assert_eq!(
            report.conflicts[0].0,
            "Missing required field: artist"
        );
        assert_eq!(report.conflicts[0].1, ConflictSeverity::High);
    }

    #[test]
    fn test_low_quality_score_warning() {
        let detector = ConflictDetector::default();

        let metadata = FusedMetadata {
            title: Some(create_field("Title".to_string(), 0.7)),
            artist: Some(create_field("Artist".to_string(), 0.7)),
            album: None,
            release_date: None,
            track_number: None,
            duration_ms: None,
            metadata_confidence: 0.4,
        };

        let report = detector.detect(&metadata, 0.4, vec![]); // 40% < 50% threshold

        assert!(!report.has_conflicts); // No conflicts, just warnings
        assert!(report.warnings.len() > 0);
        assert!(report
            .warnings
            .iter()
            .any(|w| w.contains("Low metadata quality")));
    }

    #[test]
    fn test_low_confidence_required_fields() {
        let detector = ConflictDetector::default();

        let metadata = FusedMetadata {
            title: Some(create_field("Title".to_string(), 0.3)), // < 0.6 threshold
            artist: Some(create_field("Artist".to_string(), 0.4)), // < 0.6 threshold
            album: None,
            release_date: None,
            track_number: None,
            duration_ms: None,
            metadata_confidence: 0.35,
        };

        let report = detector.detect(&metadata, 0.5, vec![]);

        // Should have warnings for low confidence
        assert!(report
            .warnings
            .iter()
            .any(|w| w.contains("Low confidence for title")));
        assert!(report
            .warnings
            .iter()
            .any(|w| w.contains("Low confidence for artist")));
        assert!(!report.has_conflicts); // Warnings, not conflicts
    }

    #[test]
    fn test_consistency_conflicts_aggregation() {
        let detector = ConflictDetector::default();

        let metadata = FusedMetadata {
            title: Some(create_field("Title".to_string(), 0.9)),
            artist: Some(create_field("Artist".to_string(), 0.9)),
            album: None,
            release_date: None,
            track_number: None,
            duration_ms: None,
            metadata_confidence: 0.9,
        };

        let consistency_conflicts = vec![
            (
                "Title mismatch: 'Title A' vs 'Title B' (similarity: 0.2)".to_string(),
                ConflictSeverity::High,
            ),
            (
                "Artist mismatch: 'Artist A' vs 'Artist B' (similarity: 0.3)".to_string(),
                ConflictSeverity::High,
            ),
        ];

        let report = detector.detect(&metadata, 0.9, consistency_conflicts);

        assert!(report.has_conflicts);
        assert_eq!(report.conflicts.len(), 2);
        assert!(report.conflicts[0].0.contains("Title mismatch"));
        assert!(report.conflicts[1].0.contains("Artist mismatch"));
    }

    #[test]
    fn test_optional_field_warnings() {
        let detector = ConflictDetector::default();

        let metadata = FusedMetadata {
            title: Some(create_field("Title".to_string(), 0.9)),
            artist: Some(create_field("Artist".to_string(), 0.9)),
            album: None,        // Optional - should warn
            release_date: None, // Optional - should warn
            track_number: None,
            duration_ms: None,
            metadata_confidence: 0.75,
        };

        let report = detector.detect(&metadata, 0.75, vec![]);

        assert!(!report.has_conflicts);
        assert!(report
            .warnings
            .iter()
            .any(|w| w.contains("Optional field missing: album")));
        assert!(report
            .warnings
            .iter()
            .any(|w| w.contains("Optional field missing: release_date")));
    }

    #[test]
    fn test_count_by_severity() {
        let detector = ConflictDetector::default();

        let conflicts = vec![
            ("High 1".to_string(), ConflictSeverity::High),
            ("High 2".to_string(), ConflictSeverity::High),
            ("Medium 1".to_string(), ConflictSeverity::Medium),
            ("Low 1".to_string(), ConflictSeverity::Low),
        ];

        let (high, medium, low) = detector.count_by_severity(&conflicts);

        assert_eq!(high, 2);
        assert_eq!(medium, 1);
        assert_eq!(low, 1);
    }

    #[test]
    fn test_is_acceptable_no_missing_required() {
        let detector = ConflictDetector::default();

        let report = ValidationReport {
            quality_score: 0.8,
            has_conflicts: true,
            warnings: vec![],
            conflicts: vec![
                ("Some other conflict".to_string(), ConflictSeverity::High),
                ("Medium conflict".to_string(), ConflictSeverity::Medium),
            ],
        };

        assert!(detector.is_acceptable(&report)); // Acceptable if no missing required fields
    }

    #[test]
    fn test_is_acceptable_with_missing_required() {
        let detector = ConflictDetector::default();

        let report = ValidationReport {
            quality_score: 0.5,
            has_conflicts: true,
            warnings: vec![],
            conflicts: vec![
                ("Missing required field: title".to_string(), ConflictSeverity::High),
            ],
        };

        assert!(!detector.is_acceptable(&report)); // Not acceptable with missing required field
    }

    #[test]
    fn test_summary_message_perfect() {
        let detector = ConflictDetector::default();

        let report = ValidationReport {
            quality_score: 0.95,
            has_conflicts: false,
            warnings: vec![],
            conflicts: vec![],
        };

        let summary = detector.summary_message(&report);
        assert!(summary.contains("Validation passed"));
        assert!(summary.contains("95.0% quality"));
    }

    #[test]
    fn test_summary_message_high_severity() {
        let detector = ConflictDetector::default();

        let report = ValidationReport {
            quality_score: 0.5,
            has_conflicts: true,
            warnings: vec![],
            conflicts: vec![
                ("High 1".to_string(), ConflictSeverity::High),
                ("High 2".to_string(), ConflictSeverity::High),
            ],
        };

        let summary = detector.summary_message(&report);
        assert!(summary.contains("Validation failed"));
        assert!(summary.contains("2 high-severity conflict"));
    }

    #[test]
    fn test_summary_message_medium_severity() {
        let detector = ConflictDetector::default();

        let report = ValidationReport {
            quality_score: 0.7,
            has_conflicts: true,
            warnings: vec![],
            conflicts: vec![
                ("Medium 1".to_string(), ConflictSeverity::Medium),
            ],
        };

        let summary = detector.summary_message(&report);
        assert!(summary.contains("Validation warning"));
        assert!(summary.contains("1 medium-severity conflict"));
    }

    #[test]
    fn test_summary_message_warnings_only() {
        let detector = ConflictDetector::default();

        let report = ValidationReport {
            quality_score: 0.8,
            has_conflicts: false,
            warnings: vec!["Warning 1".to_string(), "Warning 2".to_string()],
            conflicts: vec![],
        };

        let summary = detector.summary_message(&report);
        assert!(summary.contains("Validation passed"));
        assert!(summary.contains("2 warning(s)"));
        assert!(summary.contains("80.0% quality"));
    }

    #[test]
    fn test_custom_thresholds() {
        let detector = ConflictDetector {
            min_quality_score: 0.8,    // Higher threshold
            min_required_confidence: 0.9, // Higher threshold
        };

        let metadata = FusedMetadata {
            title: Some(create_field("Title".to_string(), 0.7)), // Now too low
            artist: Some(create_field("Artist".to_string(), 0.7)), // Now too low
            album: None,
            release_date: None,
            track_number: None,
            duration_ms: None,
            metadata_confidence: 0.7,
        };

        let report = detector.detect(&metadata, 0.7, vec![]); // 70% < 80% threshold

        // Should trigger both quality and confidence warnings
        assert!(report
            .warnings
            .iter()
            .any(|w| w.contains("Low metadata quality")));
        assert!(report
            .warnings
            .iter()
            .any(|w| w.contains("Low confidence for title")));
        assert!(report
            .warnings
            .iter()
            .any(|w| w.contains("Low confidence for artist")));
    }
}
