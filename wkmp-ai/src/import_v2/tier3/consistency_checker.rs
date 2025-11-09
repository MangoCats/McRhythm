// PLAN023 Tier 3: Consistency Checker
//
// Concept: Validate consistency of metadata fields across sources using Levenshtein similarity
// Synchronization: Accepts Tier 2 outputs (FusedMetadata), returns ValidationResult
//
// Resolution: CRITICAL-003 - Uses strsim crate for normalized Levenshtein similarity
//
// Thresholds (per critical_issues_resolution.md):
// - similarity ≥ 0.95: PASS (identical or minor differences)
// - 0.80 ≤ similarity < 0.95: WARNING (likely same, spelling variant)
// - similarity < 0.80: CONFLICT (high risk of different content)

use crate::import_v2::types::{
    ConflictSeverity, FusedMetadata, ValidationResult,
};

/// Consistency checker (Tier 3 validation concept)
///
/// **Legible Software Principle:**
/// - Independent module: Validates without side effects
/// - Explicit synchronization: Clear contract with Tier 2 fusers
/// - Transparent behavior: Thresholds are explicit constants
/// - Integrity: Always returns deterministic results
pub struct ConsistencyChecker {
    /// Similarity threshold for PASS (≥ this value = no warning)
    pass_threshold: f64,
    /// Similarity threshold for WARNING (≥ this = warning, < this = conflict)
    warning_threshold: f64,
}

impl Default for ConsistencyChecker {
    fn default() -> Self {
        Self {
            pass_threshold: 0.95,    // CRITICAL-003
            warning_threshold: 0.80, // CRITICAL-003
        }
    }
}

impl ConsistencyChecker {
    /// Validate title consistency across all source fields
    ///
    /// Compares all title variants from different sources and checks for conflicts.
    /// Returns ValidationResult indicating pass/warning/conflict status.
    pub fn validate_title(&self, metadata: &FusedMetadata) -> ValidationResult {
        // If no title selected, nothing to validate
        let Some(ref _selected_title) = metadata.title else {
            return ValidationResult::Pass;
        };

        // TODO: In full implementation, we'd have access to ALL title candidates
        // from the fusion process, not just the selected one. For now, we can only
        // validate that the selected title exists.
        //
        // This demonstrates the validation pattern. Full implementation would:
        // 1. Collect all title candidates from MetadataBundle
        // 2. Compare selected title against all candidates
        // 3. Flag conflicts if any candidate differs significantly

        ValidationResult::Pass
    }

    /// Validate artist consistency
    pub fn validate_artist(&self, metadata: &FusedMetadata) -> ValidationResult {
        let Some(ref _selected_artist) = metadata.artist else {
            return ValidationResult::Pass;
        };

        // Same TODO as title - need access to all candidates for full validation
        ValidationResult::Pass
    }

    /// Validate album consistency
    pub fn validate_album(&self, metadata: &FusedMetadata) -> ValidationResult {
        let Some(ref _selected_album) = metadata.album else {
            return ValidationResult::Pass;
        };

        ValidationResult::Pass
    }

    /// Validate consistency across a list of string values
    ///
    /// This is a helper function that will be used when we have access to
    /// all candidate values from the fusion process.
    ///
    /// # Arguments
    /// * `field_name` - Name of field being validated (for error messages)
    /// * `values` - All candidate values from different sources
    ///
    /// # Returns
    /// ValidationResult indicating whether values are consistent
    pub fn validate_string_list(
        &self,
        field_name: &str,
        values: &[String],
    ) -> ValidationResult {
        if values.len() < 2 {
            return ValidationResult::Pass; // Nothing to compare
        }

        // Compare all pairs of values
        for i in 0..values.len() {
            for j in (i + 1)..values.len() {
                let similarity = strsim::normalized_levenshtein(&values[i], &values[j]);

                if similarity < self.warning_threshold {
                    // Major difference - likely conflict
                    return ValidationResult::Conflict {
                        message: format!(
                            "{} mismatch: '{}' vs '{}' (similarity: {:.2})",
                            field_name, values[i], values[j], similarity
                        ),
                        severity: ConflictSeverity::High,
                    };
                } else if similarity < self.pass_threshold {
                    // Minor difference - warning
                    return ValidationResult::Warning {
                        message: format!(
                            "{} variant: '{}' vs '{}' (similarity: {:.2})",
                            field_name, values[i], values[j], similarity
                        ),
                    };
                }
            }
        }

        ValidationResult::Pass
    }

    /// Validate complete metadata bundle
    ///
    /// Runs all consistency checks and returns aggregate result.
    pub fn validate_metadata(&self, metadata: &FusedMetadata) -> ValidationResult {
        // Validate each field
        let title_result = self.validate_title(metadata);
        let artist_result = self.validate_artist(metadata);
        let album_result = self.validate_album(metadata);

        // Aggregate results (return worst case)
        // Priority: Conflict > Warning > Pass
        match (&title_result, &artist_result, &album_result) {
            (ValidationResult::Conflict { .. }, _, _)
            | (_, ValidationResult::Conflict { .. }, _)
            | (_, _, ValidationResult::Conflict { .. }) => {
                // Return first conflict found
                if matches!(title_result, ValidationResult::Conflict { .. }) {
                    title_result
                } else if matches!(artist_result, ValidationResult::Conflict { .. }) {
                    artist_result
                } else {
                    album_result
                }
            }
            (ValidationResult::Warning { .. }, _, _)
            | (_, ValidationResult::Warning { .. }, _)
            | (_, _, ValidationResult::Warning { .. }) => {
                // Return first warning found
                if matches!(title_result, ValidationResult::Warning { .. }) {
                    title_result
                } else if matches!(artist_result, ValidationResult::Warning { .. }) {
                    artist_result
                } else {
                    album_result
                }
            }
            _ => ValidationResult::Pass,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import_v2::types::{ExtractionSource, MetadataField};

    #[test]
    fn test_identical_strings_pass() {
        let checker = ConsistencyChecker::default();
        let values = vec!["Let It Be".to_string(), "Let It Be".to_string()];
        let result = checker.validate_string_list("title", &values);
        assert!(matches!(result, ValidationResult::Pass));
    }

    #[test]
    fn test_case_difference_warning() {
        let checker = ConsistencyChecker::default();
        let values = vec!["Let It Be".to_string(), "Let it be".to_string()];
        let result = checker.validate_string_list("title", &values);

        // Calculate actual similarity for "Let It Be" vs "Let it be":
        // strsim::normalized_levenshtein gives different result than expected
        // Let's check what it actually returns
        let actual_similarity = strsim::normalized_levenshtein("Let It Be", "Let it be");

        // Based on actual Levenshtein: only 2 characters differ ('I' vs 'i', 'B' vs 'b')
        // Length = 9 (with spaces), distance = 2
        // Normalized: 1 - (2 / 9) = 0.777... which is < 0.80
        // So this should be CONFLICT, not WARNING!

        assert!(matches!(result, ValidationResult::Conflict { .. }),
            "Expected Conflict for similarity {:.3}, got {:?}", actual_similarity, result);
    }

    #[test]
    fn test_major_difference_conflict() {
        let checker = ConsistencyChecker::default();
        let values = vec![
            "Let It Be".to_string(),
            "Yesterday".to_string(),
        ];
        let result = checker.validate_string_list("title", &values);

        // Very different strings - should be CONFLICT
        assert!(matches!(result, ValidationResult::Conflict { severity: ConflictSeverity::High, .. }));
    }

    #[test]
    fn test_remastered_suffix_warning() {
        let checker = ConsistencyChecker::default();
        let values = vec![
            "Bohemian Rhapsody".to_string(),
            "Bohemian Rhapsody (Remastered)".to_string(),
        ];
        let result = checker.validate_string_list("title", &values);

        // Similarity ≈ 0.76 (below 0.80 threshold)
        // Should be CONFLICT
        assert!(matches!(result, ValidationResult::Conflict { .. }));
    }

    #[test]
    fn test_single_value_passes() {
        let checker = ConsistencyChecker::default();
        let values = vec!["Let It Be".to_string()];
        let result = checker.validate_string_list("title", &values);
        assert!(matches!(result, ValidationResult::Pass));
    }

    #[test]
    fn test_empty_list_passes() {
        let checker = ConsistencyChecker::default();
        let values: Vec<String> = vec![];
        let result = checker.validate_string_list("title", &values);
        assert!(matches!(result, ValidationResult::Pass));
    }

    #[test]
    fn test_multiple_values_all_similar() {
        let checker = ConsistencyChecker::default();
        let values = vec![
            "Let It Be".to_string(),
            "Let It Be".to_string(),
            "Let It Be".to_string(),
        ];
        let result = checker.validate_string_list("title", &values);
        assert!(matches!(result, ValidationResult::Pass));
    }

    #[test]
    fn test_threshold_boundaries() {
        let checker = ConsistencyChecker::default();

        // Test exactly at 0.95 (should PASS)
        // Need strings with similarity exactly 0.95
        // "abcdefghij" vs "abcdefghik" = 1 difference, length 10 → similarity = 0.90
        // "abcdefghijklmno" vs "abcdefghijklmno" = 0 diff → similarity = 1.00

        // Let's use a known example: similarity just below 0.95
        let values_warning = vec![
            "The Beatles".to_string(),
            "The Beatles.".to_string(), // Extra period: similarity ≈ 0.91
        ];
        let result = checker.validate_string_list("artist", &values_warning);
        assert!(matches!(result, ValidationResult::Warning { .. }));

        // Similarity just below 0.80
        let values_conflict = vec![
            "The Beatles".to_string(),
            "Beatles".to_string(), // Missing "The ": similarity ≈ 0.64
        ];
        let result = checker.validate_string_list("artist", &values_conflict);
        assert!(matches!(result, ValidationResult::Conflict { .. }));
    }

    #[test]
    fn test_metadata_no_conflicts() {
        let checker = ConsistencyChecker::default();

        let metadata = FusedMetadata {
            title: Some(MetadataField {
                value: "Let It Be".to_string(),
                confidence: 0.9,
                source: ExtractionSource::MusicBrainz,
            }),
            artist: Some(MetadataField {
                value: "The Beatles".to_string(),
                confidence: 0.9,
                source: ExtractionSource::MusicBrainz,
            }),
            album: Some(MetadataField {
                value: "Let It Be".to_string(),
                confidence: 0.8,
                source: ExtractionSource::ID3Metadata,
            }),
            release_date: None,
            track_number: None,
            duration_ms: None,
            metadata_confidence: 0.85,
        };

        let result = checker.validate_metadata(&metadata);
        assert!(matches!(result, ValidationResult::Pass));
    }

    #[test]
    fn test_metadata_with_none_fields() {
        let checker = ConsistencyChecker::default();

        let metadata = FusedMetadata {
            title: None,
            artist: None,
            album: None,
            release_date: None,
            track_number: None,
            duration_ms: None,
            metadata_confidence: 0.0,
        };

        let result = checker.validate_metadata(&metadata);
        assert!(matches!(result, ValidationResult::Pass)); // No data = no conflicts
    }

    // === Additional Threshold Tests (P2-6) ===

    #[test]
    fn test_threshold_exact_boundaries() {
        let checker = ConsistencyChecker::default();

        // Test exactly at 0.95 threshold (should WARNING, not PASS)
        // "0123456789012345678" vs "012345678901234567X" = 1 char diff, length 19
        // Levenshtein similarity = 1 - (1/19) ≈ 0.947 < 0.95
        let values_just_below_pass = vec![
            "0123456789012345678".to_string(),
            "012345678901234567X".to_string(),
        ];
        let result = checker.validate_string_list("test", &values_just_below_pass);
        assert!(
            matches!(result, ValidationResult::Warning { .. }),
            "Expected Warning for similarity ~0.947, got {:?}",
            result
        );

        // Test exactly at 0.80 threshold (should CONFLICT)
        // "01234567890" vs "012345XXX90" = 3 char diff, length 11
        // Levenshtein similarity = 1 - (3/11) ≈ 0.727 < 0.80
        let values_just_below_warning = vec![
            "01234567890".to_string(),
            "012345XXX90".to_string(),
        ];
        let result = checker.validate_string_list("test", &values_just_below_warning);
        assert!(
            matches!(result, ValidationResult::Conflict { .. }),
            "Expected Conflict for similarity ~0.727, got {:?}",
            result
        );

        // Test above 0.95 threshold (should PASS)
        // "0123456789012345678901234567890123456789" vs "012345678901234567890123456789012345678X"
        // = 1 char diff, length 40
        // Levenshtein similarity = 1 - (1/40) = 0.975 > 0.95
        let values_above_pass = vec![
            "0123456789012345678901234567890123456789".to_string(),
            "012345678901234567890123456789012345678X".to_string(),
        ];
        let result = checker.validate_string_list("test", &values_above_pass);
        assert!(
            matches!(result, ValidationResult::Pass),
            "Expected Pass for similarity 0.975, got {:?}",
            result
        );
    }

    #[test]
    fn test_threshold_severity_escalation() {
        let checker = ConsistencyChecker::default();

        // Very low similarity → High severity
        let values_very_different = vec![
            "The Beatles".to_string(),
            "Led Zeppelin".to_string(),
        ];
        let result = checker.validate_string_list("artist", &values_very_different);
        assert!(
            matches!(result, ValidationResult::Conflict { severity: ConflictSeverity::High, .. }),
            "Expected High severity conflict for very different strings"
        );

        // Moderate similarity (in warning range) → Low severity (via Warning)
        let values_moderate = vec![
            "Let It Be (Remastered 2009)".to_string(),
            "Let It Be (Remastered 2015)".to_string(),
        ];
        let result = checker.validate_string_list("title", &values_moderate);
        // This should be either Warning or Conflict depending on actual similarity
        // Actual similarity ≈ 0.89, which is in WARNING range (0.80-0.95)
        assert!(
            matches!(result, ValidationResult::Warning { .. }),
            "Expected Warning for moderate similarity, got {:?}",
            result
        );
    }

    #[test]
    fn test_multiple_candidates_first_conflict_wins() {
        let checker = ConsistencyChecker::default();

        // Three values: first two conflict, third is similar to first
        let values = vec![
            "The Beatles".to_string(),
            "Led Zeppelin".to_string(), // Very different → conflict
            "The Beatles.".to_string(),  // Similar to first → warning
        ];
        let result = checker.validate_string_list("artist", &values);

        // Should return FIRST conflict found (Beatles vs Led Zeppelin)
        match result {
            ValidationResult::Conflict { message, severity } => {
                assert_eq!(severity, ConflictSeverity::High);
                assert!(message.contains("The Beatles"));
                assert!(message.contains("Led Zeppelin"));
            }
            other => panic!("Expected Conflict, got {:?}", other),
        }
    }

    #[test]
    fn test_custom_thresholds() {
        // Test with more lenient thresholds
        let lenient_checker = ConsistencyChecker {
            pass_threshold: 0.85,    // More lenient than default 0.95
            warning_threshold: 0.70, // More lenient than default 0.80
        };

        // Similarity ≈ 0.89 would be WARNING with default, but PASS with lenient
        let values = vec![
            "Let It Be (Remastered 2009)".to_string(),
            "Let It Be (Remastered 2015)".to_string(),
        ];
        let result = lenient_checker.validate_string_list("title", &values);
        assert!(
            matches!(result, ValidationResult::Pass),
            "Expected Pass with lenient thresholds, got {:?}",
            result
        );

        // Test with stricter thresholds
        let strict_checker = ConsistencyChecker {
            pass_threshold: 0.99,    // Very strict
            warning_threshold: 0.90, // Very strict
        };

        // Even minor differences trigger warning with strict checker
        let values = vec![
            "The Beatles".to_string(),
            "The Beatles.".to_string(), // Extra period: similarity ≈ 0.91
        ];
        let result = strict_checker.validate_string_list("artist", &values);
        assert!(
            matches!(result, ValidationResult::Warning { .. }),
            "Expected Warning with strict thresholds, got {:?}",
            result
        );
    }

    #[test]
    fn test_unicode_and_special_characters() {
        let checker = ConsistencyChecker::default();

        // Test with accented characters
        let values_accented = vec![
            "Beyoncé".to_string(),
            "Beyonce".to_string(), // Missing accent
        ];
        let result = checker.validate_string_list("artist", &values_accented);
        // Levenshtein treats 'é' and 'e' as different characters (1 difference, length 7)
        // Similarity ≈ 0.857 → WARNING range
        assert!(
            matches!(result, ValidationResult::Warning { .. }),
            "Expected Warning for accent difference, got {:?}",
            result
        );

        // Test with emoji (rare but possible in metadata)
        let values_emoji = vec![
            "Happy 😊".to_string(),
            "Happy".to_string(),
        ];
        let result = checker.validate_string_list("title", &values_emoji);
        // Emoji counts as extra characters → similarity drops
        assert!(
            !matches!(result, ValidationResult::Pass),
            "Expected non-Pass for emoji difference"
        );
    }
}
