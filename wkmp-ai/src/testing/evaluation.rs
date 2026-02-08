//! Evaluation engine for comparing import results against ground truth
//!
//! **[PLAN031 Increment 3]** Evaluation framework implementation
//!
//! This module provides:
//! - Classification of results as TP/FP/TN/FN
//! - Calculation of accuracy metrics (precision, recall, F1)
//! - Confidence calibration analysis
//!
//! ## Requirements
//!
//! - **SPEC031-EV-010**: Evaluation framework core
//! - **SPEC031-EV-020**: TP/FP/TN/FN classification
//! - **SPEC031-EV-030**: Evaluation metrics collection

use serde::{Deserialize, Serialize};

use super::ground_truth::GroundTruth;
use super::types::{AlbumClassification, Classification};

/// Result of an import attempt for a single file
///
/// Used as input to the evaluation engine for classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    /// SHA-256 hash of the audio file
    pub file_hash: String,
    /// Path to the file that was imported
    pub file_path: String,
    /// Assigned recording MBID (None if no identification)
    pub assigned_mbid: Option<String>,
    /// Assigned album MBID (for album-level tests)
    pub assigned_album_mbid: Option<String>,
    /// Confidence score of the assignment (0.0 to 1.0)
    pub confidence: Option<f64>,
    /// Time taken to process the file (milliseconds)
    pub processing_time_ms: u64,
    /// Number of API calls made during processing
    pub api_calls: usize,
}

impl ImportResult {
    /// Create a new import result
    pub fn new(file_hash: String, file_path: String) -> Self {
        Self {
            file_hash,
            file_path,
            assigned_mbid: None,
            assigned_album_mbid: None,
            confidence: None,
            processing_time_ms: 0,
            api_calls: 0,
        }
    }

    /// Builder: set assigned MBID
    pub fn with_mbid(mut self, mbid: impl Into<String>) -> Self {
        self.assigned_mbid = Some(mbid.into());
        self
    }

    /// Builder: set assigned album MBID
    pub fn with_album_mbid(mut self, mbid: impl Into<String>) -> Self {
        self.assigned_album_mbid = Some(mbid.into());
        self
    }

    /// Builder: set confidence
    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = Some(confidence);
        self
    }

    /// Builder: set processing time
    pub fn with_processing_time(mut self, time_ms: u64) -> Self {
        self.processing_time_ms = time_ms;
        self
    }

    /// Builder: set API call count
    pub fn with_api_calls(mut self, count: usize) -> Self {
        self.api_calls = count;
        self
    }
}

/// Evaluation metrics for a batch of test results
///
/// **[SPEC031-EV-030]** Evaluation metrics collection
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EvaluationMetrics {
    // Accuracy metrics (raw counts)
    /// Count of true positives
    pub true_positives: usize,
    /// Count of false positives
    pub false_positives: usize,
    /// Count of true negatives
    pub true_negatives: usize,
    /// Count of false negatives
    pub false_negatives: usize,

    // Derived metrics (calculated)
    /// Overall accuracy: (TP + TN) / Total
    pub accuracy: f64,
    /// Precision: TP / (TP + FP)
    pub precision: f64,
    /// Recall: TP / (TP + FN)
    pub recall: f64,
    /// F1 Score: 2 * (precision * recall) / (precision + recall)
    pub f1_score: f64,

    // Confidence calibration
    /// Average confidence for correct classifications
    pub avg_confidence_correct: f64,
    /// Average confidence for incorrect classifications
    pub avg_confidence_incorrect: f64,
    /// Confidence-correctness correlation (higher = better calibrated)
    pub confidence_correlation: f64,

    // Timing metrics
    /// Average processing time per file (milliseconds)
    pub avg_time_per_file_ms: u64,
    /// Total batch processing time (milliseconds)
    pub total_batch_time_ms: u64,
    /// Average API calls per file
    pub api_calls_per_file: f64,
}

impl EvaluationMetrics {
    /// Total number of results evaluated
    pub fn total(&self) -> usize {
        self.true_positives + self.false_positives + self.true_negatives + self.false_negatives
    }

    /// Number of correct classifications (TP + TN)
    pub fn correct(&self) -> usize {
        self.true_positives + self.true_negatives
    }

    /// Number of incorrect classifications (FP + FN)
    pub fn incorrect(&self) -> usize {
        self.false_positives + self.false_negatives
    }
}

/// A single classified result for tracking
#[derive(Debug, Clone)]
pub struct ClassifiedResult {
    classification: Classification,
    confidence: Option<f64>,
    processing_time_ms: u64,
    api_calls: usize,
}

/// Evaluation engine for classifying import results against ground truth
///
/// **[SPEC031-EV-010]** Evaluation framework core
#[derive(Debug, Default)]
pub struct EvaluationEngine {
    /// Collected results during evaluation
    results: Vec<ClassifiedResult>,
}

impl EvaluationEngine {
    /// Create a new evaluation engine
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
        }
    }

    /// Classify a single result against ground truth
    ///
    /// **[SPEC031-EV-020]** TP/FP/TN/FN classification logic
    ///
    /// Classification rules:
    /// - **TRUE_POSITIVE**: Assigned MBID matches expected MBID
    /// - **FALSE_POSITIVE**: Assigned MBID differs from expected (or expected is None)
    /// - **TRUE_NEGATIVE**: No MBID assigned and expected is None
    /// - **FALSE_NEGATIVE**: No MBID assigned but expected exists
    pub fn classify_result(
        &self,
        ground_truth: &GroundTruth,
        result: &ImportResult,
    ) -> Classification {
        match (&ground_truth.expected_mbid, &result.assigned_mbid) {
            // Both have MBID - check if they match
            (Some(expected), Some(assigned)) => {
                if expected == assigned {
                    Classification::TruePositive
                } else {
                    Classification::FalsePositive
                }
            }
            // Expected MBID but none assigned - missed identification
            (Some(_), None) => Classification::FalseNegative,
            // No expected MBID and none assigned - correctly rejected
            (None, None) => Classification::TrueNegative,
            // No expected MBID but one was assigned - false identification
            (None, Some(_)) => Classification::FalsePositive,
        }
    }

    /// Classify album-level result against ground truth
    ///
    /// **[SPEC031-EV-020]** Album classification logic
    pub fn classify_album_result(
        &self,
        expected_album_mbid: Option<&str>,
        assigned_album_mbid: Option<&str>,
        track_classifications: &[Classification],
    ) -> AlbumClassification {
        match (expected_album_mbid, assigned_album_mbid) {
            // Both have album MBID
            (Some(expected), Some(assigned)) => {
                if expected == assigned {
                    // Check if all tracks are correct
                    let all_tracks_correct = track_classifications
                        .iter()
                        .all(|c| c.is_success());

                    if all_tracks_correct {
                        AlbumClassification::AlbumMatch
                    } else {
                        AlbumClassification::PartialMatch
                    }
                } else {
                    AlbumClassification::WrongAlbum
                }
            }
            // Expected album but none assigned
            (Some(_), None) => AlbumClassification::NoMatch,
            // No expected album
            (None, _) => AlbumClassification::NoMatch,
        }
    }

    /// Record a classification for metrics calculation
    pub fn record_result(&mut self, ground_truth: &GroundTruth, result: &ImportResult) {
        let classification = self.classify_result(ground_truth, result);
        self.results.push(ClassifiedResult {
            classification,
            confidence: result.confidence,
            processing_time_ms: result.processing_time_ms,
            api_calls: result.api_calls,
        });
    }

    /// Calculate metrics from all recorded results
    ///
    /// **[SPEC031-EV-030]** Metrics calculation
    pub fn calculate_metrics(&self) -> EvaluationMetrics {
        let mut metrics = EvaluationMetrics::default();

        if self.results.is_empty() {
            return metrics;
        }

        // Count classifications
        for result in &self.results {
            match result.classification {
                Classification::TruePositive => metrics.true_positives += 1,
                Classification::FalsePositive => metrics.false_positives += 1,
                Classification::TrueNegative => metrics.true_negatives += 1,
                Classification::FalseNegative => metrics.false_negatives += 1,
            }
        }

        let total = metrics.total() as f64;

        // Calculate derived metrics
        if total > 0.0 {
            metrics.accuracy = metrics.correct() as f64 / total;
        }

        let tp = metrics.true_positives as f64;
        let fp = metrics.false_positives as f64;
        let fn_ = metrics.false_negatives as f64;

        // Precision: TP / (TP + FP)
        if tp + fp > 0.0 {
            metrics.precision = tp / (tp + fp);
        }

        // Recall: TP / (TP + FN)
        if tp + fn_ > 0.0 {
            metrics.recall = tp / (tp + fn_);
        }

        // F1 Score: harmonic mean of precision and recall
        if metrics.precision + metrics.recall > 0.0 {
            metrics.f1_score =
                2.0 * (metrics.precision * metrics.recall) / (metrics.precision + metrics.recall);
        }

        // Confidence calibration
        let (correct_confidences, incorrect_confidences): (Vec<_>, Vec<_>) = self
            .results
            .iter()
            .filter_map(|r| r.confidence.map(|c| (c, r.classification.is_success())))
            .partition(|(_, is_success)| *is_success);

        if !correct_confidences.is_empty() {
            let sum: f64 = correct_confidences.iter().map(|(c, _)| c).sum();
            metrics.avg_confidence_correct = sum / correct_confidences.len() as f64;
        }

        if !incorrect_confidences.is_empty() {
            let sum: f64 = incorrect_confidences.iter().map(|(c, _)| c).sum();
            metrics.avg_confidence_incorrect = sum / incorrect_confidences.len() as f64;
        }

        // Confidence correlation: difference between correct and incorrect avg confidence
        // Positive = better calibrated (higher confidence correlates with correctness)
        if !correct_confidences.is_empty() && !incorrect_confidences.is_empty() {
            metrics.confidence_correlation =
                metrics.avg_confidence_correct - metrics.avg_confidence_incorrect;
        }

        // Timing metrics
        let total_time: u64 = self.results.iter().map(|r| r.processing_time_ms).sum();
        let total_api_calls: usize = self.results.iter().map(|r| r.api_calls).sum();

        metrics.total_batch_time_ms = total_time;
        metrics.avg_time_per_file_ms = total_time / self.results.len() as u64;
        metrics.api_calls_per_file = total_api_calls as f64 / self.results.len() as f64;

        metrics
    }

    /// Get all recorded results
    pub fn results(&self) -> &[ClassifiedResult] {
        &self.results
    }

    /// Clear all recorded results
    pub fn clear(&mut self) {
        self.results.clear();
    }

    /// Get the number of recorded results
    pub fn len(&self) -> usize {
        self.results.len()
    }

    /// Check if no results have been recorded
    pub fn is_empty(&self) -> bool {
        self.results.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::ground_truth::VerificationMethod;

    fn make_ground_truth(expected_mbid: Option<&str>) -> GroundTruth {
        GroundTruth {
            id: Some(1),
            file_hash: "test_hash".to_string(),
            file_path: "test.mp3".to_string(),
            expected_mbid: expected_mbid.map(String::from),
            expected_album_mbid: None,
            verification_method: VerificationMethod::EmbeddedTag,
            verified_at: "2025-01-01".to_string(),
            confidence: 1.0,
            notes: None,
        }
    }

    fn make_import_result(assigned_mbid: Option<&str>) -> ImportResult {
        let mut result = ImportResult::new("test_hash".to_string(), "test.mp3".to_string());
        if let Some(mbid) = assigned_mbid {
            result = result.with_mbid(mbid);
        }
        result
    }

    // TC-U-EV-001: Classify TRUE_POSITIVE correctly
    #[test]
    fn test_classify_true_positive() {
        let engine = EvaluationEngine::new();
        let gt = make_ground_truth(Some("abc123"));
        let result = make_import_result(Some("abc123"));

        assert_eq!(
            engine.classify_result(&gt, &result),
            Classification::TruePositive
        );
    }

    // TC-U-EV-002: Classify FALSE_POSITIVE correctly (wrong MBID)
    #[test]
    fn test_classify_false_positive_wrong_mbid() {
        let engine = EvaluationEngine::new();
        let gt = make_ground_truth(Some("abc123"));
        let result = make_import_result(Some("xyz789"));

        assert_eq!(
            engine.classify_result(&gt, &result),
            Classification::FalsePositive
        );
    }

    // TC-U-EV-002b: Classify FALSE_POSITIVE correctly (unexpected MBID)
    #[test]
    fn test_classify_false_positive_unexpected_mbid() {
        let engine = EvaluationEngine::new();
        let gt = make_ground_truth(None); // Should not have MBID
        let result = make_import_result(Some("xyz789")); // But got one

        assert_eq!(
            engine.classify_result(&gt, &result),
            Classification::FalsePositive
        );
    }

    // TC-U-EV-003: Classify TRUE_NEGATIVE correctly
    #[test]
    fn test_classify_true_negative() {
        let engine = EvaluationEngine::new();
        let gt = make_ground_truth(None); // No expected MBID
        let result = make_import_result(None); // No assigned MBID

        assert_eq!(
            engine.classify_result(&gt, &result),
            Classification::TrueNegative
        );
    }

    // TC-U-EV-004: Classify FALSE_NEGATIVE correctly
    #[test]
    fn test_classify_false_negative() {
        let engine = EvaluationEngine::new();
        let gt = make_ground_truth(Some("abc123")); // Expected MBID
        let result = make_import_result(None); // No assigned MBID

        assert_eq!(
            engine.classify_result(&gt, &result),
            Classification::FalseNegative
        );
    }

    // TC-U-EV-005: Calculate accuracy metrics
    #[test]
    fn test_calculate_metrics_accuracy() {
        let mut engine = EvaluationEngine::new();

        // Record 8 TP, 1 FP, 1 FN (80% accuracy for TP+TN/Total)
        // But we have no TN, so accuracy = TP / Total = 80%
        for i in 0..8 {
            let gt = make_ground_truth(Some(&format!("mbid_{}", i)));
            let result = make_import_result(Some(&format!("mbid_{}", i)));
            engine.record_result(&gt, &result);
        }

        // 1 FP (wrong MBID)
        let gt = make_ground_truth(Some("expected"));
        let result = make_import_result(Some("wrong"));
        engine.record_result(&gt, &result);

        // 1 FN (missed)
        let gt = make_ground_truth(Some("expected2"));
        let result = make_import_result(None);
        engine.record_result(&gt, &result);

        let metrics = engine.calculate_metrics();

        assert_eq!(metrics.true_positives, 8);
        assert_eq!(metrics.false_positives, 1);
        assert_eq!(metrics.true_negatives, 0);
        assert_eq!(metrics.false_negatives, 1);
        assert_eq!(metrics.total(), 10);

        // Accuracy = (TP + TN) / Total = 8 / 10 = 0.80
        assert!((metrics.accuracy - 0.80).abs() < 0.001);

        // Precision = TP / (TP + FP) = 8 / 9 ≈ 0.889
        assert!((metrics.precision - 0.889).abs() < 0.01);

        // Recall = TP / (TP + FN) = 8 / 9 ≈ 0.889
        assert!((metrics.recall - 0.889).abs() < 0.01);
    }

    // TC-U-EV-006: Calculate F1 score
    #[test]
    fn test_calculate_f1_score() {
        let mut engine = EvaluationEngine::new();

        // 6 TP, 2 FP, 2 FN
        for i in 0..6 {
            let gt = make_ground_truth(Some(&format!("mbid_{}", i)));
            let result = make_import_result(Some(&format!("mbid_{}", i)));
            engine.record_result(&gt, &result);
        }
        for _ in 0..2 {
            let gt = make_ground_truth(Some("expected"));
            let result = make_import_result(Some("wrong"));
            engine.record_result(&gt, &result);
        }
        for _ in 0..2 {
            let gt = make_ground_truth(Some("expected"));
            let result = make_import_result(None);
            engine.record_result(&gt, &result);
        }

        let metrics = engine.calculate_metrics();

        // Precision = 6 / 8 = 0.75
        // Recall = 6 / 8 = 0.75
        // F1 = 2 * (0.75 * 0.75) / (0.75 + 0.75) = 0.75
        assert!((metrics.precision - 0.75).abs() < 0.001);
        assert!((metrics.recall - 0.75).abs() < 0.001);
        assert!((metrics.f1_score - 0.75).abs() < 0.001);
    }

    #[test]
    fn test_confidence_calibration() {
        let mut engine = EvaluationEngine::new();

        // Record correct results with high confidence
        for i in 0..5 {
            let gt = make_ground_truth(Some(&format!("mbid_{}", i)));
            let result = make_import_result(Some(&format!("mbid_{}", i)))
                .with_confidence(0.95);
            engine.record_result(&gt, &result);
        }

        // Record incorrect results with lower confidence
        for _ in 0..3 {
            let gt = make_ground_truth(Some("expected"));
            let result = make_import_result(Some("wrong"))
                .with_confidence(0.60);
            engine.record_result(&gt, &result);
        }

        let metrics = engine.calculate_metrics();

        // Correct: avg 0.95, Incorrect: avg 0.60
        assert!((metrics.avg_confidence_correct - 0.95).abs() < 0.001);
        assert!((metrics.avg_confidence_incorrect - 0.60).abs() < 0.001);

        // Correlation should be positive (0.95 - 0.60 = 0.35)
        assert!((metrics.confidence_correlation - 0.35).abs() < 0.001);
    }

    #[test]
    fn test_timing_metrics() {
        let mut engine = EvaluationEngine::new();

        // Record results with timing data
        let gt = make_ground_truth(Some("mbid_1"));
        let result = make_import_result(Some("mbid_1"))
            .with_processing_time(100)
            .with_api_calls(3);
        engine.record_result(&gt, &result);

        let gt = make_ground_truth(Some("mbid_2"));
        let result = make_import_result(Some("mbid_2"))
            .with_processing_time(200)
            .with_api_calls(5);
        engine.record_result(&gt, &result);

        let metrics = engine.calculate_metrics();

        assert_eq!(metrics.total_batch_time_ms, 300);
        assert_eq!(metrics.avg_time_per_file_ms, 150);
        assert!((metrics.api_calls_per_file - 4.0).abs() < 0.001);
    }

    #[test]
    fn test_album_classification_full_match() {
        let engine = EvaluationEngine::new();
        let track_classifications = vec![
            Classification::TruePositive,
            Classification::TruePositive,
            Classification::TruePositive,
        ];

        assert_eq!(
            engine.classify_album_result(
                Some("album_123"),
                Some("album_123"),
                &track_classifications
            ),
            AlbumClassification::AlbumMatch
        );
    }

    #[test]
    fn test_album_classification_partial_match() {
        let engine = EvaluationEngine::new();
        let track_classifications = vec![
            Classification::TruePositive,
            Classification::FalsePositive, // One wrong track
            Classification::TruePositive,
        ];

        assert_eq!(
            engine.classify_album_result(
                Some("album_123"),
                Some("album_123"),
                &track_classifications
            ),
            AlbumClassification::PartialMatch
        );
    }

    #[test]
    fn test_album_classification_wrong_album() {
        let engine = EvaluationEngine::new();
        let track_classifications = vec![Classification::TruePositive];

        assert_eq!(
            engine.classify_album_result(
                Some("album_123"),
                Some("album_456"), // Wrong album
                &track_classifications
            ),
            AlbumClassification::WrongAlbum
        );
    }

    #[test]
    fn test_album_classification_no_match() {
        let engine = EvaluationEngine::new();
        let track_classifications = vec![];

        assert_eq!(
            engine.classify_album_result(
                Some("album_123"),
                None, // No album assigned
                &track_classifications
            ),
            AlbumClassification::NoMatch
        );
    }

    #[test]
    fn test_empty_metrics() {
        let engine = EvaluationEngine::new();
        let metrics = engine.calculate_metrics();

        assert_eq!(metrics.total(), 0);
        assert_eq!(metrics.accuracy, 0.0);
        assert_eq!(metrics.precision, 0.0);
        assert_eq!(metrics.recall, 0.0);
        assert_eq!(metrics.f1_score, 0.0);
    }

    #[test]
    fn test_clear_results() {
        let mut engine = EvaluationEngine::new();

        let gt = make_ground_truth(Some("mbid_1"));
        let result = make_import_result(Some("mbid_1"));
        engine.record_result(&gt, &result);

        assert_eq!(engine.len(), 1);
        assert!(!engine.is_empty());

        engine.clear();

        assert_eq!(engine.len(), 0);
        assert!(engine.is_empty());
    }
}
