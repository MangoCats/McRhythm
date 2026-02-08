//! # Algorithm Tester - Closed-Loop Accuracy Evaluation
//!
//! Evaluates album matcher accuracy using PLAN031 testing infrastructure.
//!
//! Usage:
//!   cargo run --example algorithm_tester -- --ground-truth album_matcher_results_run29f.json --test album_matcher_results.json
//!
//! The ground truth file should contain validated results from a known-good run.
//! The test file contains results from the current algorithm to evaluate.

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::{info, warn, error};

/// Album matcher result structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlbumMatcherResult {
    pub album_path: String,
    pub artist: String,
    pub album: String,
    pub mbid: String,
    #[serde(default)]
    pub musicbrainz_url: String,
    #[serde(default)]
    pub expected_track_count: usize,
    #[serde(default)]
    pub detected_track_count: usize,
    #[serde(default)]
    pub perfect_count_match: bool,
    #[serde(default)]
    pub matched_tracks_count: usize,
    #[serde(default)]
    pub match_percentage: f64,
    #[serde(default)]
    pub mean_error: f64,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub matching_stage: String,
    #[serde(default)]
    pub confidence: String,
    #[serde(default)]
    pub matched_artist: String,
    #[serde(default)]
    pub matched_album: String,
    #[serde(default)]
    pub artist_mismatch: bool,
}

/// Classification result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Classification {
    TruePositive,   // Correct MBID assigned
    FalsePositive,  // Wrong MBID assigned
    TrueNegative,   // Correctly no MBID (no ground truth)
    FalseNegative,  // Missed identification
}

impl Classification {
    pub fn is_failure(&self) -> bool {
        matches!(self, Classification::FalsePositive | Classification::FalseNegative)
    }
}

/// Evaluation metrics
#[derive(Debug, Default)]
pub struct Metrics {
    pub true_positives: usize,
    pub false_positives: usize,
    pub true_negatives: usize,
    pub false_negatives: usize,
}

impl Metrics {
    pub fn total(&self) -> usize {
        self.true_positives + self.false_positives + self.true_negatives + self.false_negatives
    }

    pub fn accuracy(&self) -> f64 {
        let total = self.total();
        if total == 0 {
            return 0.0;
        }
        (self.true_positives + self.true_negatives) as f64 / total as f64
    }

    pub fn precision(&self) -> f64 {
        let denom = self.true_positives + self.false_positives;
        if denom == 0 {
            return 0.0;
        }
        self.true_positives as f64 / denom as f64
    }

    pub fn recall(&self) -> f64 {
        let denom = self.true_positives + self.false_negatives;
        if denom == 0 {
            return 0.0;
        }
        self.true_positives as f64 / denom as f64
    }

    pub fn f1_score(&self) -> f64 {
        let p = self.precision();
        let r = self.recall();
        if p + r == 0.0 {
            return 0.0;
        }
        2.0 * p * r / (p + r)
    }
}

/// Evaluation detail
#[derive(Debug)]
pub struct EvalDetail {
    pub album_path: String,
    pub artist: String,
    pub album: String,
    pub expected_mbid: Option<String>,
    pub assigned_mbid: Option<String>,
    pub classification: Classification,
    pub confidence: f64,
    pub stage: String,
    pub ground_truth_confidence: f64,
}

/// Failure category for pattern analysis
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FailureCategory {
    WrongRecording,      // Got different MBID
    MissedIdentification, // Should have matched but didn't
    LowConfidence,       // Match below threshold
    ArtistMismatch,      // Artist name didn't match
    StageRegression,     // Matched in different stage
}

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .init();

    let args: Vec<String> = std::env::args().collect();

    // Parse arguments
    let ground_truth_path = args
        .iter()
        .enumerate()
        .find_map(|(i, arg)| {
            if arg == "--ground-truth" && i + 1 < args.len() {
                Some(PathBuf::from(&args[i + 1]))
            } else {
                None
            }
        })
        .unwrap_or_else(|| PathBuf::from("album_matcher_results_run29f.json"));

    let test_path = args
        .iter()
        .enumerate()
        .find_map(|(i, arg)| {
            if arg == "--test" && i + 1 < args.len() {
                Some(PathBuf::from(&args[i + 1]))
            } else {
                None
            }
        })
        .unwrap_or_else(|| PathBuf::from("album_matcher_results.json"));

    let min_confidence = args
        .iter()
        .enumerate()
        .find_map(|(i, arg)| {
            if arg == "--min-confidence" && i + 1 < args.len() {
                args[i + 1].parse::<f64>().ok()
            } else {
                None
            }
        })
        .unwrap_or(80.0);

    info!("=== Algorithm Accuracy Tester ===");
    info!("Ground Truth: {}", ground_truth_path.display());
    info!("Test Results: {}", test_path.display());
    info!("Min Confidence: {}%", min_confidence);
    println!();

    // Load ground truth
    info!("Loading ground truth...");
    let gt_content = std::fs::read_to_string(&ground_truth_path)?;
    let gt_results: Vec<AlbumMatcherResult> = serde_json::from_str(&gt_content)?;

    // Filter to high-confidence successful matches only
    let ground_truth: HashMap<String, AlbumMatcherResult> = gt_results
        .into_iter()
        .filter(|r| {
            r.status == "Success"
                && r.match_percentage >= min_confidence
                && !r.artist_mismatch
                && !r.mbid.is_empty()
        })
        .map(|r| (r.album_path.clone(), r))
        .collect();

    info!("Ground truth entries: {} (filtered to ≥{}% confidence)", ground_truth.len(), min_confidence);

    // Load test results
    info!("Loading test results...");
    let test_content = std::fs::read_to_string(&test_path)?;
    let test_results: Vec<AlbumMatcherResult> = serde_json::from_str(&test_content)?;
    info!("Test results: {}", test_results.len());
    println!();

    // Evaluate
    let mut metrics = Metrics::default();
    let mut details: Vec<EvalDetail> = Vec::new();
    let mut failure_categories: HashMap<FailureCategory, Vec<String>> = HashMap::new();

    for test in &test_results {
        let gt = ground_truth.get(&test.album_path);

        let (classification, expected, gt_conf) = match gt {
            Some(gt_entry) => {
                let expected = Some(gt_entry.mbid.clone());
                let gt_conf = gt_entry.match_percentage / 100.0;

                if test.status != "Success" || test.mbid.is_empty() {
                    // Test didn't find MBID but ground truth has one
                    (Classification::FalseNegative, expected, gt_conf)
                } else if test.mbid == gt_entry.mbid {
                    // Correct MBID
                    (Classification::TruePositive, expected, gt_conf)
                } else {
                    // Wrong MBID
                    (Classification::FalsePositive, expected, gt_conf)
                }
            }
            None => {
                // No ground truth - can't evaluate definitively
                // Skip or mark as TN if test also failed
                if test.status != "Success" || test.mbid.is_empty() {
                    (Classification::TrueNegative, None, 0.0)
                } else {
                    // Test found something but we have no ground truth to validate
                    // Skip for now - we can't evaluate without ground truth
                    continue;
                }
            }
        };

        // Update metrics
        match classification {
            Classification::TruePositive => metrics.true_positives += 1,
            Classification::FalsePositive => metrics.false_positives += 1,
            Classification::TrueNegative => metrics.true_negatives += 1,
            Classification::FalseNegative => metrics.false_negatives += 1,
        }

        // Track failures by category
        if classification.is_failure() {
            let category = categorize_failure(&test, gt);
            failure_categories
                .entry(category)
                .or_default()
                .push(test.album_path.clone());
        }

        details.push(EvalDetail {
            album_path: test.album_path.clone(),
            artist: test.artist.clone(),
            album: test.album.clone(),
            expected_mbid: expected,
            assigned_mbid: if test.status == "Success" && !test.mbid.is_empty() {
                Some(test.mbid.clone())
            } else {
                None
            },
            classification,
            confidence: test.match_percentage / 100.0,
            stage: test.matching_stage.clone(),
            ground_truth_confidence: gt_conf,
        });
    }

    // Print report
    print_report(&metrics, &details, &failure_categories);

    Ok(())
}

fn categorize_failure(test: &AlbumMatcherResult, gt: Option<&AlbumMatcherResult>) -> FailureCategory {
    if test.artist_mismatch {
        return FailureCategory::ArtistMismatch;
    }

    if test.status != "Success" || test.mbid.is_empty() {
        return FailureCategory::MissedIdentification;
    }

    if test.match_percentage < 60.0 {
        return FailureCategory::LowConfidence;
    }

    if let Some(gt_entry) = gt {
        if test.matching_stage != gt_entry.matching_stage {
            return FailureCategory::StageRegression;
        }
    }

    FailureCategory::WrongRecording
}

fn print_report(
    metrics: &Metrics,
    details: &[EvalDetail],
    failure_categories: &HashMap<FailureCategory, Vec<String>>,
) {
    println!("{}", "═".repeat(70));
    println!("ALGORITHM ACCURACY EVALUATION REPORT");
    println!("{}\n", "═".repeat(70));

    println!("SUMMARY:");
    println!("  Files Evaluated: {}", metrics.total());
    println!();

    println!("CLASSIFICATION BREAKDOWN:");
    println!("  True Positives:  {:4} (correct MBID assigned)", metrics.true_positives);
    println!("  False Positives: {:4} (wrong MBID assigned)", metrics.false_positives);
    println!("  True Negatives:  {:4} (correctly no match)", metrics.true_negatives);
    println!("  False Negatives: {:4} (missed identification)", metrics.false_negatives);
    println!();

    println!("ACCURACY METRICS:");
    println!("  Overall Accuracy: {:6.2}%", metrics.accuracy() * 100.0);
    println!("  Precision:        {:6.2}%", metrics.precision() * 100.0);
    println!("  Recall:           {:6.2}%", metrics.recall() * 100.0);
    println!("  F1 Score:         {:6.3}", metrics.f1_score());
    println!();

    // Failure analysis
    let total_failures = metrics.false_positives + metrics.false_negatives;
    if total_failures > 0 {
        println!("FAILURE PATTERN ANALYSIS ({} failures):", total_failures);

        let mut categories: Vec<_> = failure_categories.iter().collect();
        categories.sort_by_key(|(_, files)| std::cmp::Reverse(files.len()));

        for (category, files) in categories {
            let pct = files.len() as f64 / total_failures as f64 * 100.0;
            println!(
                "  {:?}: {} ({:.1}%)",
                category,
                files.len(),
                pct
            );
        }
        println!();

        // Show specific failures
        println!("FAILURE DETAILS (first 15):");
        let failures: Vec<_> = details.iter().filter(|d| d.classification.is_failure()).take(15).collect();

        for (i, detail) in failures.iter().enumerate() {
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
            println!(
                "     Stage: {} | Confidence: {:.1}% | GT Conf: {:.1}%",
                detail.stage,
                detail.confidence * 100.0,
                detail.ground_truth_confidence * 100.0
            );
        }

        if total_failures > 15 {
            println!("  ... and {} more failures", total_failures - 15);
        }
        println!();

        // Improvement suggestions
        println!("IMPROVEMENT SUGGESTIONS:");
        suggest_improvements(failure_categories);
    } else {
        println!("✅ PERFECT ACCURACY - No failures detected!");
    }

    println!("{}", "═".repeat(70));
}

fn suggest_improvements(failure_categories: &HashMap<FailureCategory, Vec<String>>) {
    let total: usize = failure_categories.values().map(|v| v.len()).sum();
    if total == 0 {
        return;
    }

    for (category, files) in failure_categories {
        let pct = files.len() as f64 / total as f64 * 100.0;
        if pct < 10.0 {
            continue; // Skip minor patterns
        }

        match category {
            FailureCategory::WrongRecording => {
                println!("  • Wrong Recording ({:.0}%): Consider improving edition selection logic", pct);
                println!("    - Review track duration matching tolerance");
                println!("    - Add ISRC/recording-level verification");
            }
            FailureCategory::MissedIdentification => {
                println!("  • Missed Identification ({:.0}%): Expand search strategies", pct);
                println!("    - Lower minimum confidence threshold for fallback");
                println!("    - Add fuzzy artist/album name matching [PLAN026]");
                println!("    - Implement AcoustID fallback for failed MB searches");
            }
            FailureCategory::LowConfidence => {
                println!("  • Low Confidence ({:.0}%): Improve matching algorithms", pct);
                println!("    - Use Jaro-Winkler instead of Levenshtein [PLAN026]");
                println!("    - Add multi-source Bayesian fusion [PLAN026]");
            }
            FailureCategory::ArtistMismatch => {
                println!("  • Artist Mismatch ({:.0}%): Improve artist normalization", pct);
                println!("    - Handle 'featuring' credits better");
                println!("    - Normalize Unicode and punctuation");
            }
            FailureCategory::StageRegression => {
                println!("  • Stage Regression ({:.0}%): Review stage selection logic", pct);
                println!("    - Verify stage preference ordering");
            }
        }
    }
}
