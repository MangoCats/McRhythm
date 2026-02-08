//! # Test Runner - Closed-Loop Algorithm Testing
//!
//! Integrates PLAN031 testing infrastructure with the album matcher to:
//! 1. Load ground truth from validated album_matcher_results.json
//! 2. Run the matcher against test files
//! 3. Evaluate accuracy (TP/FP/TN/FN)
//! 4. Analyze failure patterns
//! 5. Generate improvement suggestions
//! 6. Report progress

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tracing::{info, warn, error};

use wkmp_ai::testing::{
    evaluation::{EvaluationEngine, ImportResult},
    failure::FailureAnalyzer,
    ground_truth::{self, GroundTruth, VerificationMethod},
    reporting::{ProgressReport, ReportFormatter},
    types::{BatchConfig, Classification, TestPhase},
};

/// Album matcher result from previous validated runs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlbumMatcherResult {
    pub album_path: String,
    pub artist: String,
    pub album: String,
    pub mbid: String,
    pub musicbrainz_url: String,
    pub expected_track_count: usize,
    pub detected_track_count: usize,
    pub perfect_count_match: bool,
    pub matched_tracks_count: usize,
    pub match_percentage: f64,
    pub mean_error: f64,
    pub status: String,
    pub matching_stage: String,
    pub confidence: String,
    pub matched_artist: String,
    pub matched_album: String,
    pub artist_mismatch: bool,
}

/// Load ground truth from a validated album_matcher_results JSON file
pub async fn load_ground_truth_from_results(
    pool: &SqlitePool,
    results_path: &Path,
) -> Result<usize> {
    info!("Loading ground truth from: {}", results_path.display());

    let json_content = std::fs::read_to_string(results_path)?;
    let results: Vec<AlbumMatcherResult> = serde_json::from_str(&json_content)?;

    let mut loaded = 0;
    let mut skipped = 0;

    for result in &results {
        // Only use successful matches with high confidence as ground truth
        if result.status != "Success" {
            skipped += 1;
            continue;
        }

        if result.match_percentage < 80.0 {
            // Skip low-confidence matches
            skipped += 1;
            continue;
        }

        if result.artist_mismatch {
            // Skip artist mismatches
            skipped += 1;
            continue;
        }

        // Create file hash from path (simplified - in production use actual file hash)
        let file_hash = format!("hash_{}", md5_hash(&result.album_path));

        let gt = GroundTruth {
            id: None,
            file_hash: file_hash.clone(),
            file_path: result.album_path.clone(),
            expected_mbid: Some(result.mbid.clone()),
            expected_album_mbid: Some(result.mbid.clone()),
            verification_method: VerificationMethod::AlgorithmicHigh,
            verified_at: chrono::Utc::now().to_rfc3339(),
            confidence: result.match_percentage / 100.0,
            notes: Some(format!(
                "Loaded from {} ({}%, stage: {})",
                results_path.display(),
                result.match_percentage,
                result.matching_stage
            )),
        };

        ground_truth::upsert(pool, &gt).await?;
        loaded += 1;
    }

    info!(
        "Ground truth loaded: {} entries ({} skipped - low confidence or errors)",
        loaded, skipped
    );

    Ok(loaded)
}

/// Simple MD5 hash for path -> file_hash conversion
fn md5_hash(input: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Evaluate current algorithm accuracy against ground truth
pub async fn evaluate_accuracy(
    pool: &SqlitePool,
    results_path: &Path,
) -> Result<EvaluationReport> {
    info!("Evaluating accuracy against ground truth...");

    // Load current run results
    let json_content = std::fs::read_to_string(results_path)?;
    let results: Vec<AlbumMatcherResult> = serde_json::from_str(&json_content)?;

    let mut engine = EvaluationEngine::new();
    let mut details = Vec::new();

    for result in &results {
        let file_hash = format!("hash_{}", md5_hash(&result.album_path));

        // Get ground truth for this file
        let gt = match ground_truth::get_by_hash(pool, &file_hash).await? {
            Some(gt) => gt,
            None => {
                // No ground truth - can't evaluate
                continue;
            }
        };

        // Create import result from matcher output
        let import_result = if result.status == "Success" && !result.mbid.is_empty() {
            ImportResult::new(file_hash.clone(), result.album_path.clone())
                .with_mbid(&result.mbid)
                .with_album_mbid(&result.mbid)
                .with_confidence(result.match_percentage / 100.0)
                .with_processing_time(0) // Not tracked
        } else {
            ImportResult::new(file_hash.clone(), result.album_path.clone())
        };

        // Classify and record
        let classification = engine.classify_result(&gt, &import_result);
        engine.record_result(&gt, &import_result);

        details.push(EvaluationDetail {
            file_path: result.album_path.clone(),
            artist: result.artist.clone(),
            album: result.album.clone(),
            expected_mbid: gt.expected_mbid.clone(),
            assigned_mbid: if result.status == "Success" { Some(result.mbid.clone()) } else { None },
            classification,
            confidence: result.match_percentage / 100.0,
            matching_stage: result.matching_stage.clone(),
        });
    }

    let metrics = engine.calculate_metrics();

    Ok(EvaluationReport {
        total_evaluated: details.len(),
        metrics,
        details,
    })
}

/// Detailed evaluation result
#[derive(Debug)]
pub struct EvaluationReport {
    pub total_evaluated: usize,
    pub metrics: wkmp_ai::testing::evaluation::EvaluationMetrics,
    pub details: Vec<EvaluationDetail>,
}

#[derive(Debug)]
pub struct EvaluationDetail {
    pub file_path: String,
    pub artist: String,
    pub album: String,
    pub expected_mbid: Option<String>,
    pub assigned_mbid: Option<String>,
    pub classification: Classification,
    pub confidence: f64,
    pub matching_stage: String,
}

/// Print evaluation report
pub fn print_evaluation_report(report: &EvaluationReport) {
    println!("\n{}", "═".repeat(70));
    println!("ALGORITHM ACCURACY EVALUATION");
    println!("{}\n", "═".repeat(70));

    println!("Files Evaluated: {}\n", report.total_evaluated);

    println!("CLASSIFICATION BREAKDOWN:");
    println!(
        "  True Positives:  {:3} (correct MBID assigned)",
        report.metrics.true_positives
    );
    println!(
        "  False Positives: {:3} (wrong MBID assigned)",
        report.metrics.false_positives
    );
    println!(
        "  True Negatives:  {:3} (correctly rejected)",
        report.metrics.true_negatives
    );
    println!(
        "  False Negatives: {:3} (missed identification)",
        report.metrics.false_negatives
    );

    println!("\nACCURACY METRICS:");
    println!("  Overall Accuracy: {:.1}%", report.metrics.accuracy * 100.0);
    println!("  Precision:        {:.1}%", report.metrics.precision * 100.0);
    println!("  Recall:           {:.1}%", report.metrics.recall * 100.0);
    println!("  F1 Score:         {:.3}", report.metrics.f1_score);

    // Show failures
    let failures: Vec<_> = report
        .details
        .iter()
        .filter(|d| d.classification.is_failure())
        .collect();

    if !failures.is_empty() {
        println!("\nFAILURES ({}):", failures.len());
        for (i, detail) in failures.iter().take(10).enumerate() {
            println!(
                "  {}. {} - {} [{:?}]",
                i + 1,
                detail.artist,
                detail.album,
                detail.classification
            );
            if let Some(ref expected) = detail.expected_mbid {
                println!("     Expected: {}", expected);
            }
            if let Some(ref assigned) = detail.assigned_mbid {
                println!("     Assigned: {}", assigned);
            }
            println!("     Stage: {} (conf: {:.1}%)", detail.matching_stage, detail.confidence * 100.0);
        }
        if failures.len() > 10 {
            println!("  ... and {} more", failures.len() - 10);
        }
    }

    println!("\n{}", "═".repeat(70));
}

/// Analyze failure patterns and generate suggestions
pub fn analyze_failures(report: &EvaluationReport) {
    use wkmp_ai::testing::batch::BatchResultEntry;

    // Convert details to batch result entries for failure analyzer
    let entries: Vec<BatchResultEntry> = report
        .details
        .iter()
        .map(|d| BatchResultEntry {
            file_path: PathBuf::from(&d.file_path),
            file_hash: format!("hash_{}", md5_hash(&d.file_path)),
            expected_mbid: d.expected_mbid.clone(),
            assigned_mbid: d.assigned_mbid.clone(),
            classification: d.classification,
            confidence: Some(d.confidence),
            processing_time_ms: 0,
        })
        .collect();

    let analyzer = FailureAnalyzer::with_threshold(5.0); // 5% significance
    let analysis = analyzer.analyze(&entries);

    if analysis.total_failures == 0 {
        println!("\n✅ No failures to analyze - 100% accuracy!");
        return;
    }

    println!("\n{}", "═".repeat(70));
    println!("FAILURE PATTERN ANALYSIS");
    println!("{}\n", "═".repeat(70));

    println!("Total Failures: {}", analysis.total_failures);

    if !analysis.patterns.is_empty() {
        println!("\nDETECTED PATTERNS:");
        for pattern in &analysis.patterns {
            println!(
                "  {}: {} occurrences ({:.1}%)",
                pattern.category, pattern.occurrence_count, pattern.percentage
            );
            for char in &pattern.common_characteristics {
                println!("    - {}", char);
            }
        }
    }

    if !analysis.suggestions.is_empty() {
        println!("\nIMPROVEMENT SUGGESTIONS:");
        for (i, suggestion) in analysis.suggestions.iter().enumerate() {
            println!("  {}. {}", i + 1, suggestion.description());
        }
    }

    if analysis.summary.systemic_issues_detected {
        println!("\n⚠️  SYSTEMIC ISSUES DETECTED - Major algorithm changes may be needed");
    }

    println!("\n{}", "═".repeat(70));
}
