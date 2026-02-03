//! Failure pattern analysis for identifying algorithm weaknesses
//!
//! **[PLAN031 Increment 6]** Failure analysis and improvement suggestions
//!
//! This module provides:
//! - Pattern detection in failure categories
//! - Root cause analysis for failed identifications
//! - Improvement suggestion generation
//!
//! ## Requirements
//!
//! - **SPEC031-FA-010**: Failure analysis engine
//! - **SPEC031-FA-020**: Pattern detection in failures
//! - **SPEC031-FA-030**: Improvement suggestions generation

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::batch::BatchResultEntry;
use super::types::{Classification, FailureCategory};

/// Detected pattern in failures
///
/// **[SPEC031-FA-020]** Pattern detection structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailurePattern {
    /// Category of failures in this pattern
    pub category: FailureCategory,
    /// Number of occurrences
    pub occurrence_count: usize,
    /// Percentage of total failures
    pub percentage: f64,
    /// File hashes affected
    pub affected_files: Vec<String>,
    /// Common characteristics observed
    pub common_characteristics: Vec<String>,
    /// Suggested improvements
    pub suggested_improvements: Vec<String>,
}

/// Improvement suggestion types
///
/// **[SPEC031-FA-030]** Improvement suggestion variants
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ImprovementSuggestion {
    /// Adjust a threshold parameter
    AdjustThreshold {
        /// Parameter name
        parameter: String,
        /// Current value
        current: f64,
        /// Suggested value
        suggested: f64,
        /// Rationale for change
        rationale: String,
    },
    /// Add string normalization
    AddNormalization {
        /// Field to normalize
        field: String,
        /// Pattern to match
        pattern: String,
        /// Replacement value
        replacement: String,
    },
    /// Adjust source weight
    WeightAdjustment {
        /// Data source name
        source: String,
        /// Context for adjustment (e.g., "for classical music")
        context: String,
        /// Current weight
        current_weight: f64,
        /// Suggested weight
        suggested_weight: f64,
    },
    /// New heuristic to implement
    NewHeuristic {
        /// Description of the heuristic
        description: String,
        /// Pseudocode or implementation hint
        pseudocode: String,
    },
    /// Skip strategy for certain conditions
    SkipStrategy {
        /// Condition for skipping
        condition: String,
        /// Reason for skipping
        reason: String,
    },
}

impl ImprovementSuggestion {
    /// Get a short description of the suggestion
    pub fn description(&self) -> String {
        match self {
            ImprovementSuggestion::AdjustThreshold {
                parameter,
                current,
                suggested,
                ..
            } => {
                format!(
                    "Adjust {} from {:.2} to {:.2}",
                    parameter, current, suggested
                )
            }
            ImprovementSuggestion::AddNormalization { field, pattern, .. } => {
                format!("Add normalization for {} matching '{}'", field, pattern)
            }
            ImprovementSuggestion::WeightAdjustment {
                source, context, ..
            } => {
                format!("Adjust {} weight {}", source, context)
            }
            ImprovementSuggestion::NewHeuristic { description, .. } => {
                format!("New heuristic: {}", description)
            }
            ImprovementSuggestion::SkipStrategy { condition, .. } => {
                format!("Skip when: {}", condition)
            }
        }
    }
}

/// Analysis result for a batch of failures
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FailureAnalysis {
    /// Total failures analyzed
    pub total_failures: usize,
    /// Patterns detected
    pub patterns: Vec<FailurePattern>,
    /// Generated improvement suggestions
    pub suggestions: Vec<ImprovementSuggestion>,
    /// Summary statistics
    pub summary: AnalysisSummary,
}

/// Summary statistics for failure analysis
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnalysisSummary {
    /// Most common failure category
    pub most_common_category: Option<FailureCategory>,
    /// Count of most common category
    pub most_common_count: usize,
    /// Number of unique categories
    pub unique_categories: usize,
    /// Whether patterns suggest systemic issues
    pub systemic_issues_detected: bool,
}

/// Failure analyzer for detecting patterns and suggesting improvements
///
/// **[SPEC031-FA-010]** Failure analysis engine
#[derive(Debug, Default)]
pub struct FailureAnalyzer {
    /// Threshold for considering a pattern significant (percentage)
    significance_threshold: f64,
}

impl FailureAnalyzer {
    /// Create a new failure analyzer
    pub fn new() -> Self {
        Self {
            significance_threshold: 10.0, // 10% threshold for significance
        }
    }

    /// Create with custom significance threshold
    pub fn with_threshold(threshold: f64) -> Self {
        Self {
            significance_threshold: threshold,
        }
    }

    /// Analyze a batch of results for failure patterns
    ///
    /// **[SPEC031-FA-020]** Pattern detection
    pub fn analyze(&self, results: &[BatchResultEntry]) -> FailureAnalysis {
        // Filter to only failures
        let failures: Vec<_> = results
            .iter()
            .filter(|r| r.classification.is_failure())
            .collect();

        if failures.is_empty() {
            return FailureAnalysis::default();
        }

        // Categorize failures
        let categorized = self.categorize_failures(&failures);

        // Detect patterns
        let patterns = self.detect_patterns(&categorized, failures.len());

        // Generate suggestions based on patterns
        let suggestions = self.generate_suggestions(&patterns);

        // Build summary
        let summary = self.build_summary(&patterns);

        FailureAnalysis {
            total_failures: failures.len(),
            patterns,
            suggestions,
            summary,
        }
    }

    /// Categorize failures by type
    fn categorize_failures<'a>(
        &self,
        failures: &[&'a BatchResultEntry],
    ) -> HashMap<FailureCategory, Vec<&'a BatchResultEntry>> {
        let mut categories: HashMap<FailureCategory, Vec<&BatchResultEntry>> = HashMap::new();

        for failure in failures {
            let category = self.determine_category(failure);
            categories.entry(category).or_default().push(failure);
        }

        categories
    }

    /// Determine the failure category for a result
    fn determine_category(&self, result: &BatchResultEntry) -> FailureCategory {
        match result.classification {
            Classification::FalsePositive => {
                // We got an MBID but it's wrong - determine why
                if result.expected_mbid.is_some() && result.assigned_mbid.is_some() {
                    // Check confidence to determine if overconfident
                    if result.confidence.unwrap_or(0.0) > 0.9 {
                        FailureCategory::OverconfidentWrong
                    } else {
                        FailureCategory::WrongRecording
                    }
                } else if result.expected_mbid.is_none() {
                    // Should not have identified, but did
                    FailureCategory::WrongRecording
                } else {
                    FailureCategory::WrongRecording
                }
            }
            Classification::FalseNegative => {
                // Expected MBID but didn't get one
                FailureCategory::MissedIdentification
            }
            // Other classifications shouldn't be here (they're successes)
            _ => FailureCategory::NoGroundTruth,
        }
    }

    /// Detect patterns from categorized failures
    fn detect_patterns(
        &self,
        categorized: &HashMap<FailureCategory, Vec<&BatchResultEntry>>,
        total_failures: usize,
    ) -> Vec<FailurePattern> {
        let mut patterns = Vec::new();

        for (category, entries) in categorized {
            let count = entries.len();
            let percentage = (count as f64 / total_failures as f64) * 100.0;

            // Skip non-significant patterns
            if percentage < self.significance_threshold {
                continue;
            }

            let affected_files: Vec<String> =
                entries.iter().map(|e| e.file_hash.clone()).collect();

            // Analyze common characteristics
            let characteristics = self.analyze_characteristics(entries);

            // Generate category-specific improvements
            let improvements = self.suggest_for_category(*category, &characteristics);

            patterns.push(FailurePattern {
                category: *category,
                occurrence_count: count,
                percentage,
                affected_files,
                common_characteristics: characteristics,
                suggested_improvements: improvements,
            });
        }

        // Sort by occurrence count (most common first)
        patterns.sort_by(|a, b| b.occurrence_count.cmp(&a.occurrence_count));

        patterns
    }

    /// Analyze common characteristics among failures
    fn analyze_characteristics(&self, entries: &[&BatchResultEntry]) -> Vec<String> {
        let mut characteristics = Vec::new();

        // Check for low confidence pattern
        let low_confidence_count = entries
            .iter()
            .filter(|e| e.confidence.unwrap_or(0.0) < 0.5)
            .count();

        if low_confidence_count > entries.len() / 2 {
            characteristics.push("Low confidence scores (<0.5)".to_string());
        }

        // Check for high confidence but wrong
        let high_confidence_wrong = entries
            .iter()
            .filter(|e| e.confidence.unwrap_or(0.0) > 0.9)
            .count();

        if high_confidence_wrong > entries.len() / 3 {
            characteristics.push("High confidence but incorrect (>0.9)".to_string());
        }

        // Check for no MBID assigned
        let no_mbid_count = entries.iter().filter(|e| e.assigned_mbid.is_none()).count();

        if no_mbid_count > entries.len() / 2 {
            characteristics.push("No MBID assigned".to_string());
        }

        // Check for all different file types (diverse failures)
        if entries.len() > 3 && characteristics.is_empty() {
            characteristics.push("Diverse failure patterns".to_string());
        }

        characteristics
    }

    /// Generate category-specific improvement suggestions
    fn suggest_for_category(
        &self,
        category: FailureCategory,
        characteristics: &[String],
    ) -> Vec<String> {
        let mut suggestions = Vec::new();

        match category {
            FailureCategory::WrongRecording => {
                suggestions.push("Review string matching thresholds".to_string());
                suggestions.push("Consider adding title normalization".to_string());
            }
            FailureCategory::WrongAlbum => {
                suggestions.push("Verify album matching logic".to_string());
                suggestions.push("Check for compilation album handling".to_string());
            }
            FailureCategory::WrongArtist => {
                suggestions.push("Improve artist name normalization".to_string());
                suggestions.push("Handle 'featuring' credits better".to_string());
            }
            FailureCategory::MissedIdentification => {
                suggestions.push("Lower confidence threshold".to_string());
                suggestions.push("Add fallback identification methods".to_string());
            }
            FailureCategory::OverconfidentWrong => {
                suggestions.push("Review confidence calculation".to_string());
                suggestions.push("Add validation cross-checks".to_string());
            }
            FailureCategory::UnderconfidentRight => {
                suggestions.push("Boost confidence for matching cases".to_string());
            }
            FailureCategory::AcoustIdMismatch => {
                suggestions.push("Reduce AcoustID weight in fusion".to_string());
                suggestions.push("Add acoustic fingerprint validation".to_string());
            }
            FailureCategory::MetadataSearchFailed => {
                suggestions.push("Improve search query construction".to_string());
                suggestions.push("Add fuzzy search fallback".to_string());
            }
            FailureCategory::ID3TagsUnreliable => {
                suggestions.push("Reduce ID3 tag weight".to_string());
                suggestions.push("Add ID3 validation heuristics".to_string());
            }
            FailureCategory::MultipleValidMatches => {
                suggestions.push("Implement disambiguation logic".to_string());
                suggestions.push("Prefer original releases".to_string());
            }
            _ => {}
        }

        // Add characteristic-based suggestions
        for char in characteristics {
            if char.contains("Low confidence") {
                suggestions.push("Review confidence calculation formula".to_string());
            }
            if char.contains("High confidence but incorrect") {
                suggestions.push("Add validation step before high-confidence assignments".to_string());
            }
        }

        suggestions
    }

    /// Generate actionable improvement suggestions
    fn generate_suggestions(&self, patterns: &[FailurePattern]) -> Vec<ImprovementSuggestion> {
        let mut suggestions = Vec::new();

        for pattern in patterns {
            match pattern.category {
                FailureCategory::OverconfidentWrong if pattern.percentage > 20.0 => {
                    suggestions.push(ImprovementSuggestion::AdjustThreshold {
                        parameter: "confidence_threshold".to_string(),
                        current: 0.9,
                        suggested: 0.95,
                        rationale: format!(
                            "{:.1}% of failures are high-confidence wrong matches",
                            pattern.percentage
                        ),
                    });
                }
                FailureCategory::MissedIdentification if pattern.percentage > 30.0 => {
                    suggestions.push(ImprovementSuggestion::AdjustThreshold {
                        parameter: "minimum_match_threshold".to_string(),
                        current: 0.8,
                        suggested: 0.7,
                        rationale: format!(
                            "{:.1}% of failures are missed identifications",
                            pattern.percentage
                        ),
                    });
                }
                FailureCategory::AcoustIdMismatch if pattern.percentage > 15.0 => {
                    suggestions.push(ImprovementSuggestion::WeightAdjustment {
                        source: "acoustid".to_string(),
                        context: "in multi-source fusion".to_string(),
                        current_weight: 1.0,
                        suggested_weight: 0.7,
                    });
                }
                FailureCategory::WrongArtist if pattern.percentage > 10.0 => {
                    suggestions.push(ImprovementSuggestion::AddNormalization {
                        field: "artist_name".to_string(),
                        pattern: "featuring|feat\\.|ft\\.".to_string(),
                        replacement: "".to_string(),
                    });
                }
                _ => {}
            }
        }

        suggestions
    }

    /// Build summary statistics
    fn build_summary(&self, patterns: &[FailurePattern]) -> AnalysisSummary {
        let most_common = patterns.first();

        let systemic = patterns
            .iter()
            .any(|p| p.percentage > 50.0 || p.occurrence_count > 10);

        AnalysisSummary {
            most_common_category: most_common.map(|p| p.category),
            most_common_count: most_common.map(|p| p.occurrence_count).unwrap_or(0),
            unique_categories: patterns.len(),
            systemic_issues_detected: systemic,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_failure_entry(
        hash: &str,
        classification: Classification,
        expected: Option<&str>,
        assigned: Option<&str>,
        confidence: Option<f64>,
    ) -> BatchResultEntry {
        BatchResultEntry {
            file_path: PathBuf::from(format!("{}.mp3", hash)),
            file_hash: hash.to_string(),
            expected_mbid: expected.map(String::from),
            assigned_mbid: assigned.map(String::from),
            classification,
            confidence,
            processing_time_ms: 100,
        }
    }

    // TC-U-FA-001: Categorize failures correctly
    #[test]
    fn test_categorize_false_positive() {
        let analyzer = FailureAnalyzer::new();

        let entry = make_failure_entry(
            "hash_1",
            Classification::FalsePositive,
            Some("expected_mbid"),
            Some("wrong_mbid"),
            Some(0.85),
        );

        let category = analyzer.determine_category(&entry);
        assert_eq!(category, FailureCategory::WrongRecording);
    }

    #[test]
    fn test_categorize_overconfident_wrong() {
        let analyzer = FailureAnalyzer::new();

        let entry = make_failure_entry(
            "hash_1",
            Classification::FalsePositive,
            Some("expected_mbid"),
            Some("wrong_mbid"),
            Some(0.95), // High confidence but wrong
        );

        let category = analyzer.determine_category(&entry);
        assert_eq!(category, FailureCategory::OverconfidentWrong);
    }

    #[test]
    fn test_categorize_false_negative() {
        let analyzer = FailureAnalyzer::new();

        let entry = make_failure_entry(
            "hash_1",
            Classification::FalseNegative,
            Some("expected_mbid"),
            None,
            None,
        );

        let category = analyzer.determine_category(&entry);
        assert_eq!(category, FailureCategory::MissedIdentification);
    }

    // TC-U-FA-002: Detect patterns
    #[test]
    fn test_detect_patterns() {
        let analyzer = FailureAnalyzer::with_threshold(0.0); // Include all patterns

        let results = vec![
            make_failure_entry(
                "hash_1",
                Classification::FalseNegative,
                Some("mbid_1"),
                None,
                None,
            ),
            make_failure_entry(
                "hash_2",
                Classification::FalseNegative,
                Some("mbid_2"),
                None,
                None,
            ),
            make_failure_entry(
                "hash_3",
                Classification::FalseNegative,
                Some("mbid_3"),
                None,
                None,
            ),
            make_failure_entry(
                "hash_4",
                Classification::FalsePositive,
                Some("mbid_4"),
                Some("wrong_4"),
                Some(0.7),
            ),
        ];

        let analysis = analyzer.analyze(&results);

        assert_eq!(analysis.total_failures, 4);
        assert!(!analysis.patterns.is_empty());

        // Most common should be MissedIdentification
        let most_common = &analysis.patterns[0];
        assert_eq!(most_common.category, FailureCategory::MissedIdentification);
        assert_eq!(most_common.occurrence_count, 3);
    }

    // TC-U-FA-003: Generate improvement suggestions
    #[test]
    fn test_generate_suggestions() {
        let analyzer = FailureAnalyzer::with_threshold(0.0);

        // Create failures dominated by missed identifications
        let results: Vec<BatchResultEntry> = (0..10)
            .map(|i| {
                make_failure_entry(
                    &format!("hash_{}", i),
                    Classification::FalseNegative,
                    Some(&format!("mbid_{}", i)),
                    None,
                    None,
                )
            })
            .collect();

        let analysis = analyzer.analyze(&results);

        // Should suggest lowering threshold
        assert!(!analysis.suggestions.is_empty());
    }

    #[test]
    fn test_empty_results() {
        let analyzer = FailureAnalyzer::new();
        let analysis = analyzer.analyze(&[]);

        assert_eq!(analysis.total_failures, 0);
        assert!(analysis.patterns.is_empty());
        assert!(analysis.suggestions.is_empty());
    }

    #[test]
    fn test_no_failures() {
        let analyzer = FailureAnalyzer::new();

        let results = vec![
            make_failure_entry(
                "hash_1",
                Classification::TruePositive,
                Some("mbid_1"),
                Some("mbid_1"),
                Some(0.95),
            ),
            make_failure_entry(
                "hash_2",
                Classification::TrueNegative,
                None,
                None,
                None,
            ),
        ];

        let analysis = analyzer.analyze(&results);

        assert_eq!(analysis.total_failures, 0);
        assert!(analysis.patterns.is_empty());
    }

    #[test]
    fn test_significance_threshold() {
        let analyzer = FailureAnalyzer::with_threshold(50.0); // 50% threshold

        let results = vec![
            // 2 missed identifications (40%)
            make_failure_entry(
                "hash_1",
                Classification::FalseNegative,
                Some("mbid_1"),
                None,
                None,
            ),
            make_failure_entry(
                "hash_2",
                Classification::FalseNegative,
                Some("mbid_2"),
                None,
                None,
            ),
            // 3 wrong recordings (60%)
            make_failure_entry(
                "hash_3",
                Classification::FalsePositive,
                Some("mbid_3"),
                Some("wrong_3"),
                Some(0.7),
            ),
            make_failure_entry(
                "hash_4",
                Classification::FalsePositive,
                Some("mbid_4"),
                Some("wrong_4"),
                Some(0.7),
            ),
            make_failure_entry(
                "hash_5",
                Classification::FalsePositive,
                Some("mbid_5"),
                Some("wrong_5"),
                Some(0.7),
            ),
        ];

        let analysis = analyzer.analyze(&results);

        // Only WrongRecording should pass 50% threshold
        assert_eq!(analysis.patterns.len(), 1);
        assert_eq!(
            analysis.patterns[0].category,
            FailureCategory::WrongRecording
        );
    }

    #[test]
    fn test_systemic_issues_detection() {
        let analyzer = FailureAnalyzer::with_threshold(0.0);

        // All same category - systemic issue
        let results: Vec<BatchResultEntry> = (0..15)
            .map(|i| {
                make_failure_entry(
                    &format!("hash_{}", i),
                    Classification::FalseNegative,
                    Some(&format!("mbid_{}", i)),
                    None,
                    None,
                )
            })
            .collect();

        let analysis = analyzer.analyze(&results);

        assert!(analysis.summary.systemic_issues_detected);
    }

    #[test]
    fn test_improvement_suggestion_description() {
        let suggestion = ImprovementSuggestion::AdjustThreshold {
            parameter: "confidence".to_string(),
            current: 0.8,
            suggested: 0.9,
            rationale: "test".to_string(),
        };

        let desc = suggestion.description();
        assert!(desc.contains("Adjust confidence"));
        assert!(desc.contains("0.80"));
        assert!(desc.contains("0.90"));
    }

    #[test]
    fn test_characteristics_detection() {
        let analyzer = FailureAnalyzer::new();

        // All low confidence
        let entries: Vec<BatchResultEntry> = (0..5)
            .map(|i| {
                make_failure_entry(
                    &format!("hash_{}", i),
                    Classification::FalsePositive,
                    Some(&format!("mbid_{}", i)),
                    Some(&format!("wrong_{}", i)),
                    Some(0.3), // Low confidence
                )
            })
            .collect();

        let entry_refs: Vec<&BatchResultEntry> = entries.iter().collect();
        let chars = analyzer.analyze_characteristics(&entry_refs);

        assert!(chars.iter().any(|c| c.contains("Low confidence")));
    }
}
