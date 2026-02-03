//! Report generation for test batches and progress tracking
//!
//! **[PLAN031 Increment 8]** Reporting and progress display
//!
//! This module provides:
//! - Batch report formatting
//! - Cumulative progress tracking
//! - Console-friendly report output
//!
//! ## Requirements
//!
//! - **SPEC031-RP-010**: Batch report generation
//! - **SPEC031-RP-020**: Cumulative progress tracking

use serde::{Deserialize, Serialize};

use super::batch::BatchReport;
use super::evaluation::EvaluationMetrics;
use super::failure::FailureAnalysis;
use super::types::TestPhase;

/// Progress report tracking improvement across batches
///
/// **[SPEC031-RP-020]** Cumulative progress tracking
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProgressReport {
    /// Number of batches completed
    pub batches_completed: usize,
    /// Total files processed across all batches
    pub total_files_processed: usize,
    /// Accuracy history per batch
    pub accuracy_history: Vec<f64>,
    /// Accuracy trend (linear regression slope)
    pub accuracy_trend: f64,
    /// Current phase
    pub current_phase: TestPhase,
    /// List of improvements applied
    pub improvements_applied: Vec<String>,
    /// Estimated batches to reach target (if improving)
    pub estimated_batches_to_target: Option<usize>,
}

impl ProgressReport {
    /// Create a new progress report from batch history
    pub fn from_batches(batches: &[BatchReport]) -> Self {
        if batches.is_empty() {
            return Self::default();
        }

        let accuracy_history: Vec<f64> = batches.iter().map(|b| b.metrics.accuracy).collect();

        let trend = calculate_trend(&accuracy_history);

        let current_phase = batches.last().map(|b| b.phase).unwrap_or(TestPhase::Phase1);

        // Estimate batches to target (90% accuracy)
        let estimated = if trend > 0.01 {
            let current = accuracy_history.last().copied().unwrap_or(0.0);
            let remaining = 0.90 - current;
            if remaining > 0.0 {
                Some((remaining / trend).ceil() as usize)
            } else {
                Some(0)
            }
        } else {
            None
        };

        Self {
            batches_completed: batches.len(),
            total_files_processed: batches.iter().map(|b| b.files_processed.len()).sum(),
            accuracy_history,
            accuracy_trend: trend,
            current_phase,
            improvements_applied: Vec::new(),
            estimated_batches_to_target: estimated,
        }
    }

    /// Add an applied improvement
    pub fn add_improvement(&mut self, improvement: impl Into<String>) {
        self.improvements_applied.push(improvement.into());
    }
}

/// Calculate linear trend from accuracy history
fn calculate_trend(values: &[f64]) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }

    let n = values.len() as f64;
    let x_mean = (n - 1.0) / 2.0;
    let y_mean: f64 = values.iter().sum::<f64>() / n;

    let mut numerator = 0.0;
    let mut denominator = 0.0;

    for (i, &y) in values.iter().enumerate() {
        let x = i as f64;
        numerator += (x - x_mean) * (y - y_mean);
        denominator += (x - x_mean).powi(2);
    }

    if denominator.abs() < f64::EPSILON {
        0.0
    } else {
        numerator / denominator
    }
}

/// Report formatter for console output
pub struct ReportFormatter;

impl ReportFormatter {
    /// Format a batch report for console display
    ///
    /// **[SPEC031-RP-010]** Batch report format
    pub fn format_batch_report(report: &BatchReport, analysis: Option<&FailureAnalysis>) -> String {
        let mut output = String::new();

        // Header
        output.push_str(&format!(
            "\n{}\n",
            "═".repeat(65)
        ));
        output.push_str(&format!(
            "BATCH REPORT: batch_{:03}\n",
            report.batch_id
        ));
        output.push_str(&format!("{}\n", "═".repeat(65)));

        // Summary stats
        output.push_str(&format!(
            "Files Processed:    {}\n",
            report.files_processed.len()
        ));
        output.push_str(&format!(
            "Duration:           {}\n",
            format_duration(report.duration_ms)
        ));
        output.push_str(&format!("Phase:              {}\n", report.phase));
        output.push_str(&format!("Stop Reason:        {}\n\n", report.stop_reason));

        // Accuracy metrics
        output.push_str("ACCURACY METRICS:\n");
        output.push_str(&format_metrics(&report.metrics));

        // Confidence calibration
        output.push_str("\nCONFIDENCE CALIBRATION:\n");
        output.push_str(&format_confidence(&report.metrics));

        // Failure analysis (if provided)
        if let Some(analysis) = analysis {
            if !analysis.patterns.is_empty() {
                output.push_str("\nFAILURE ANALYSIS:\n");
                for pattern in &analysis.patterns {
                    output.push_str(&format!(
                        "  {}: {} ({:.1}%)\n",
                        pattern.category, pattern.occurrence_count, pattern.percentage
                    ));
                }
            }

            if !analysis.suggestions.is_empty() {
                output.push_str("\nSUGGESTED IMPROVEMENTS:\n");
                for (i, suggestion) in analysis.suggestions.iter().enumerate() {
                    output.push_str(&format!("  {}. {}\n", i + 1, suggestion.description()));
                }
            }
        }

        output.push_str(&format!("\n{}\n", "═".repeat(65)));

        output
    }

    /// Format a progress report for console display
    ///
    /// **[SPEC031-RP-020]** Progress report format
    pub fn format_progress_report(report: &ProgressReport) -> String {
        let mut output = String::new();

        output.push_str(&format!("\n{}\n", "═".repeat(65)));
        output.push_str("CUMULATIVE PROGRESS REPORT\n");
        output.push_str(&format!("{}\n\n", "═".repeat(65)));

        output.push_str(&format!(
            "Batches Completed:      {}\n",
            report.batches_completed
        ));
        output.push_str(&format!(
            "Total Files Processed:  {}\n",
            report.total_files_processed
        ));
        output.push_str(&format!("Current Phase:          {}\n", report.current_phase));

        if !report.accuracy_history.is_empty() {
            output.push_str(&format!(
                "Current Accuracy:       {:.1}%\n",
                report.accuracy_history.last().unwrap_or(&0.0) * 100.0
            ));
        }

        output.push_str(&format!(
            "Accuracy Trend:         {}\n",
            format_trend(report.accuracy_trend)
        ));

        if let Some(est) = report.estimated_batches_to_target {
            if est == 0 {
                output.push_str("Target Status:          ACHIEVED (90%+)\n");
            } else {
                output.push_str(&format!(
                    "Est. Batches to 90%:    ~{}\n",
                    est
                ));
            }
        } else {
            output.push_str("Est. Batches to 90%:    Unknown (flat/declining trend)\n");
        }

        if !report.improvements_applied.is_empty() {
            output.push_str("\nIMPROVEMENTS APPLIED:\n");
            for improvement in &report.improvements_applied {
                output.push_str(&format!("  - {}\n", improvement));
            }
        }

        // Accuracy history chart (simple ASCII)
        if report.accuracy_history.len() >= 2 {
            output.push_str("\nACCURACY HISTORY:\n");
            output.push_str(&format_accuracy_chart(&report.accuracy_history));
        }

        output.push_str(&format!("\n{}\n", "═".repeat(65)));

        output
    }

    /// Format a compact summary for quick display
    pub fn format_compact_summary(metrics: &EvaluationMetrics) -> String {
        format!(
            "Accuracy: {:.1}% | Precision: {:.1}% | Recall: {:.1}% | F1: {:.3}",
            metrics.accuracy * 100.0,
            metrics.precision * 100.0,
            metrics.recall * 100.0,
            metrics.f1_score
        )
    }
}

fn format_metrics(metrics: &EvaluationMetrics) -> String {
    let total = metrics.total();
    let mut output = String::new();

    output.push_str(&format!(
        "  True Positives:   {:3} ({:.1}%)\n",
        metrics.true_positives,
        if total > 0 {
            metrics.true_positives as f64 / total as f64 * 100.0
        } else {
            0.0
        }
    ));
    output.push_str(&format!(
        "  False Positives:  {:3} ({:.1}%)\n",
        metrics.false_positives,
        if total > 0 {
            metrics.false_positives as f64 / total as f64 * 100.0
        } else {
            0.0
        }
    ));
    output.push_str(&format!(
        "  True Negatives:   {:3} ({:.1}%)\n",
        metrics.true_negatives,
        if total > 0 {
            metrics.true_negatives as f64 / total as f64 * 100.0
        } else {
            0.0
        }
    ));
    output.push_str(&format!(
        "  False Negatives:  {:3} ({:.1}%)\n\n",
        metrics.false_negatives,
        if total > 0 {
            metrics.false_negatives as f64 / total as f64 * 100.0
        } else {
            0.0
        }
    ));

    output.push_str(&format!(
        "  Overall Accuracy: {:.1}%\n",
        metrics.accuracy * 100.0
    ));
    output.push_str(&format!("  Precision:        {:.1}%\n", metrics.precision * 100.0));
    output.push_str(&format!("  Recall:           {:.1}%\n", metrics.recall * 100.0));
    output.push_str(&format!("  F1 Score:         {:.3}\n", metrics.f1_score));

    output
}

fn format_confidence(metrics: &EvaluationMetrics) -> String {
    let mut output = String::new();

    output.push_str(&format!(
        "  Avg confidence (correct):   {:.2}\n",
        metrics.avg_confidence_correct
    ));
    output.push_str(&format!(
        "  Avg confidence (incorrect): {:.2}\n",
        metrics.avg_confidence_incorrect
    ));

    let calibration = if metrics.confidence_correlation > 0.2 {
        "GOOD"
    } else if metrics.confidence_correlation > 0.0 {
        "FAIR"
    } else {
        "POOR"
    };

    output.push_str(&format!(
        "  Calibration:                {} ({:.2})\n",
        calibration, metrics.confidence_correlation
    ));

    output
}

fn format_duration(ms: u64) -> String {
    if ms < 1000 {
        format!("{}ms", ms)
    } else if ms < 60000 {
        format!("{:.1}s", ms as f64 / 1000.0)
    } else {
        let mins = ms / 60000;
        let secs = (ms % 60000) / 1000;
        format!("{}m {}s", mins, secs)
    }
}

fn format_trend(trend: f64) -> String {
    if trend > 0.05 {
        format!("↑ Strong (+{:.1}% per batch)", trend * 100.0)
    } else if trend > 0.01 {
        format!("↑ Improving (+{:.1}% per batch)", trend * 100.0)
    } else if trend > -0.01 {
        "→ Stable".to_string()
    } else if trend > -0.05 {
        format!("↓ Declining ({:.1}% per batch)", trend * 100.0)
    } else {
        format!("↓ Strongly declining ({:.1}% per batch)", trend * 100.0)
    }
}

fn format_accuracy_chart(history: &[f64]) -> String {
    let mut output = String::new();
    let height = 5;
    let width = history.len().min(20);

    // Find min/max for scaling
    let min_val = history.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_val = history.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = (max_val - min_val).max(0.1);

    // Create chart rows
    for row in (0..height).rev() {
        let threshold = min_val + (row as f64 / (height - 1) as f64) * range;
        output.push_str("  ");
        if row == height - 1 {
            output.push_str(&format!("{:>3}%│", (max_val * 100.0) as i32));
        } else if row == 0 {
            output.push_str(&format!("{:>3}%│", (min_val * 100.0) as i32));
        } else {
            output.push_str("    │");
        }

        for &val in history.iter().take(width) {
            if val >= threshold {
                output.push('█');
            } else {
                output.push(' ');
            }
        }
        output.push('\n');
    }

    // X axis
    output.push_str("      └");
    output.push_str(&"─".repeat(width));
    output.push_str(&format!(" (batches 1-{})\n", history.len()));

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::batch::BatchResultEntry;
    use crate::testing::failure::FailureAnalyzer;
    use crate::testing::types::{Classification, StopReason};
    use std::path::PathBuf;

    fn make_test_metrics(tp: usize, fp: usize, tn: usize, fn_: usize) -> EvaluationMetrics {
        let total = (tp + fp + tn + fn_) as f64;
        let precision = if tp + fp > 0 {
            tp as f64 / (tp + fp) as f64
        } else {
            0.0
        };
        let recall = if tp + fn_ > 0 {
            tp as f64 / (tp + fn_) as f64
        } else {
            0.0
        };

        EvaluationMetrics {
            true_positives: tp,
            false_positives: fp,
            true_negatives: tn,
            false_negatives: fn_,
            accuracy: (tp + tn) as f64 / total,
            precision,
            recall,
            f1_score: if precision + recall > 0.0 {
                2.0 * precision * recall / (precision + recall)
            } else {
                0.0
            },
            avg_confidence_correct: 0.9,
            avg_confidence_incorrect: 0.5,
            confidence_correlation: 0.4,
            avg_time_per_file_ms: 100,
            total_batch_time_ms: 1000,
            api_calls_per_file: 2.0,
        }
    }

    fn make_test_batch_report(batch_id: usize, accuracy: f64) -> BatchReport {
        let tp = (accuracy * 10.0) as usize;
        let fn_ = 10 - tp;

        BatchReport {
            batch_id,
            run_id: crate::testing::batch::RunId::from_str("test_run"),
            files_processed: (0..10).map(|i| PathBuf::from(format!("file_{}.mp3", i))).collect(),
            results: vec![],
            metrics: make_test_metrics(tp, 0, 0, fn_),
            stop_reason: StopReason::Completed,
            duration_ms: 5000,
            phase: TestPhase::Phase1,
            consecutive_failures: 0,
        }
    }

    // TC-U-RP-001: Format batch report
    #[test]
    fn test_format_batch_report() {
        let report = make_test_batch_report(1, 0.8);
        let output = ReportFormatter::format_batch_report(&report, None);

        assert!(output.contains("BATCH REPORT"));
        assert!(output.contains("Files Processed"));
        assert!(output.contains("ACCURACY METRICS"));
        assert!(output.contains("Precision"));
        assert!(output.contains("Recall"));
    }

    // TC-U-RP-002: Format progress report
    #[test]
    fn test_format_progress_report() {
        let batches = vec![
            make_test_batch_report(1, 0.6),
            make_test_batch_report(2, 0.7),
            make_test_batch_report(3, 0.8),
        ];

        let progress = ProgressReport::from_batches(&batches);
        let output = ReportFormatter::format_progress_report(&progress);

        assert!(output.contains("CUMULATIVE PROGRESS"));
        assert!(output.contains("Batches Completed:      3"));
        assert!(output.contains("Accuracy Trend"));
    }

    #[test]
    fn test_progress_from_batches() {
        let batches = vec![
            make_test_batch_report(1, 0.5),
            make_test_batch_report(2, 0.6),
            make_test_batch_report(3, 0.7),
            make_test_batch_report(4, 0.8),
        ];

        let progress = ProgressReport::from_batches(&batches);

        assert_eq!(progress.batches_completed, 4);
        assert_eq!(progress.total_files_processed, 40); // 4 batches * 10 files
        assert_eq!(progress.accuracy_history.len(), 4);
        assert!(progress.accuracy_trend > 0.0); // Improving
    }

    #[test]
    fn test_calculate_trend_improving() {
        let values = vec![0.5, 0.6, 0.7, 0.8, 0.9];
        let trend = calculate_trend(&values);
        assert!(trend > 0.0);
    }

    #[test]
    fn test_calculate_trend_declining() {
        let values = vec![0.9, 0.8, 0.7, 0.6, 0.5];
        let trend = calculate_trend(&values);
        assert!(trend < 0.0);
    }

    #[test]
    fn test_calculate_trend_stable() {
        let values = vec![0.75, 0.76, 0.74, 0.75, 0.75];
        let trend = calculate_trend(&values);
        assert!(trend.abs() < 0.02);
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(500), "500ms");
        assert_eq!(format_duration(1500), "1.5s");
        assert_eq!(format_duration(65000), "1m 5s");
    }

    #[test]
    fn test_format_compact_summary() {
        let metrics = make_test_metrics(8, 1, 0, 1);
        let summary = ReportFormatter::format_compact_summary(&metrics);

        assert!(summary.contains("Accuracy"));
        assert!(summary.contains("Precision"));
        assert!(summary.contains("Recall"));
    }

    #[test]
    fn test_empty_batches() {
        let progress = ProgressReport::from_batches(&[]);
        assert_eq!(progress.batches_completed, 0);
        assert_eq!(progress.accuracy_trend, 0.0);
    }

    #[test]
    fn test_batch_report_with_failure_analysis() {
        let report = make_test_batch_report(1, 0.7);

        // Create some failure entries for analysis
        let results: Vec<BatchResultEntry> = (0..3)
            .map(|i| BatchResultEntry {
                file_path: PathBuf::from(format!("file_{}.mp3", i)),
                file_hash: format!("hash_{}", i),
                expected_mbid: Some(format!("mbid_{}", i)),
                assigned_mbid: None, // False negatives
                classification: Classification::FalseNegative,
                confidence: None,
                processing_time_ms: 100,
            })
            .collect();

        let analyzer = FailureAnalyzer::with_threshold(0.0);
        let analysis = analyzer.analyze(&results);

        let output = ReportFormatter::format_batch_report(&report, Some(&analysis));

        assert!(output.contains("FAILURE ANALYSIS"));
    }

    #[test]
    fn test_add_improvement() {
        let mut progress = ProgressReport::default();
        progress.add_improvement("Adjusted confidence threshold");
        progress.add_improvement("Added artist normalization");

        assert_eq!(progress.improvements_applied.len(), 2);
    }
}
