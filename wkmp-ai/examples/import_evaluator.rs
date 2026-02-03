//! # Import Evaluator - Full Pipeline Test Runner
//!
//! Tests the wkmp-ai import pipeline by discovering files, extracting metadata,
//! and evaluating against ground truth.
//!
//! Usage:
//!   cargo run --example import_evaluator -- --root "C:\Users\User\Music" [OPTIONS]
//!
//! Options:
//!   --root <path>         Root folder to scan (required)
//!   --ground-truth <path> JSON file with ground truth (optional)
//!   --save-gt             Save results as ground truth for future runs
//!   --min-confidence <f>  Minimum confidence threshold (default: 0.50)
//!   --max-files <n>       Maximum files to process (default: unlimited)
//!   --verbose             Show per-file progress

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use wkmp_ai::services::content_type_classifier::{ContentType, TriagePath};
use wkmp_ai::services::metadata_extractor::MetadataExtractor;
use wkmp_ai::testing::{
    import_harness::{
        EvaluatedImportResult, ImportHarnessConfig, ImportTestHarness, ImportTestReport,
        ImportTestResult,
    },
    Classification,
};

/// Command line configuration
#[derive(Debug, Clone)]
struct Config {
    root_folder: PathBuf,
    ground_truth_file: Option<PathBuf>,
    save_ground_truth: bool,
    min_confidence: f64,
    max_files: Option<usize>,
    verbose: bool,
}

impl Config {
    fn from_args() -> Result<Self> {
        let args: Vec<String> = std::env::args().collect();

        let root_folder = args
            .iter()
            .enumerate()
            .find_map(|(i, arg)| {
                if arg == "--root" && i + 1 < args.len() {
                    Some(PathBuf::from(&args[i + 1]))
                } else {
                    None
                }
            })
            .ok_or_else(|| anyhow::anyhow!("--root <path> is required"))?;

        let ground_truth_file = args.iter().enumerate().find_map(|(i, arg)| {
            if arg == "--ground-truth" && i + 1 < args.len() {
                Some(PathBuf::from(&args[i + 1]))
            } else {
                None
            }
        });

        let save_ground_truth = args.iter().any(|a| a == "--save-gt");

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
            .unwrap_or(0.50);

        let max_files = args.iter().enumerate().find_map(|(i, arg)| {
            if arg == "--max-files" && i + 1 < args.len() {
                args[i + 1].parse::<usize>().ok()
            } else {
                None
            }
        });

        let verbose = args.iter().any(|a| a == "--verbose");

        Ok(Config {
            root_folder,
            ground_truth_file,
            save_ground_truth,
            min_confidence,
            max_files,
            verbose,
        })
    }
}

/// Ground truth entry from JSON file (compatible with album_matcher_results)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct GroundTruthEntry {
    #[serde(default)]
    file_path: String,
    #[serde(default)]
    album_path: String,
    #[serde(default)]
    mbid: String,
    #[serde(default)]
    recording_mbid: Option<String>,
    #[serde(default)]
    release_mbid: Option<String>,
    #[serde(default)]
    content_type: Option<String>,
    #[serde(default)]
    confidence: Option<f64>,
    #[serde(default)]
    match_percentage: Option<f64>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    artist: Option<String>,
    #[serde(default)]
    album: Option<String>,
}

impl GroundTruthEntry {
    /// Get the file path (supports both file_path and album_path formats)
    fn path(&self) -> &str {
        if !self.file_path.is_empty() {
            &self.file_path
        } else {
            &self.album_path
        }
    }

    /// Get the MBID (supports multiple formats)
    fn get_mbid(&self) -> Option<&str> {
        if !self.mbid.is_empty() {
            Some(&self.mbid)
        } else {
            self.recording_mbid
                .as_deref()
                .or(self.release_mbid.as_deref())
        }
    }

    /// Get confidence (supports multiple formats)
    fn get_confidence(&self) -> f64 {
        self.confidence.or(self.match_percentage.map(|p| p / 100.0)).unwrap_or(0.0)
    }
}

/// Load ground truth from a JSON file
fn load_ground_truth_file(path: &Path) -> Result<HashMap<String, GroundTruthEntry>> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read ground truth file: {}", path.display()))?;

    let entries: Vec<GroundTruthEntry> = serde_json::from_str(&content)?;

    let mut map = HashMap::new();
    for entry in entries {
        // Only include successful entries with high confidence
        let is_success = entry.status.as_deref() == Some("Success") || entry.status.is_none();
        if is_success && entry.get_confidence() >= 0.50 {
            let path_key = entry.path().to_string();
            map.insert(path_key, entry);
        }
    }

    Ok(map)
}

/// Process a single file - extract metadata and determine triage path
fn process_file(
    extractor: &MetadataExtractor,
    file_path: &Path,
    config: &Config,
) -> ImportTestResult {
    let start = Instant::now();

    // Compute file hash
    let file_hash = match ImportTestHarness::compute_file_hash(file_path) {
        Ok(h) => h,
        Err(e) => {
            return ImportTestResult::failed(
                file_path.to_path_buf(),
                String::new(),
                format!("Hash error: {}", e),
            );
        }
    };

    // Extract metadata and duration
    let metadata = match extractor.extract(file_path) {
        Ok(meta) => meta,
        Err(e) => {
            return ImportTestResult::failed(
                file_path.to_path_buf(),
                file_hash,
                format!("Metadata error: {}", e),
            );
        }
    };

    let duration_secs = metadata.duration_seconds.unwrap_or(0.0);

    if duration_secs <= 0.0 {
        return ImportTestResult::failed(
            file_path.to_path_buf(),
            file_hash,
            "Could not determine file duration".to_string(),
        );
    }

    // Determine triage path (without actual classification)
    let triage = TriagePath::from_duration(duration_secs);
    let content_type = match triage {
        TriagePath::SingleSong => ContentType::SingleSong,
        TriagePath::Album => ContentType::FullAlbum,
        TriagePath::DualPath => ContentType::SingleSong, // Default to single for now
    };

    let elapsed = start.elapsed();

    ImportTestResult {
        file_path: file_path.to_path_buf(),
        file_hash,
        content_type,
        recording_mbid: None,
        release_mbid: None,
        confidence: 0.0, // No classification done yet
        match_percentage: None,
        matching_stage: Some(format!("triage:{:?}", triage)),
        matched_artist: metadata.artist,
        matched_title: metadata.title,
        processing_time_ms: elapsed.as_millis() as u64,
        error: None,
        duration_secs: Some(duration_secs),
    }
}

/// Classify result against ground truth
fn classify_result(
    result: &ImportTestResult,
    gt: Option<&GroundTruthEntry>,
    min_confidence: f64,
) -> (Classification, Option<String>) {
    match gt {
        Some(gt_entry) => {
            // We have ground truth
            let expected = gt_entry.get_mbid();
            let assigned = result
                .recording_mbid
                .as_deref()
                .or(result.release_mbid.as_deref());

            match (assigned, expected) {
                (Some(a), Some(e)) => {
                    if a == e {
                        (Classification::TruePositive, Some(e.to_string()))
                    } else {
                        (Classification::FalsePositive, Some(e.to_string()))
                    }
                }
                (Some(_), None) => (Classification::FalsePositive, None),
                (None, Some(e)) => (Classification::FalseNegative, Some(e.to_string())),
                (None, None) => (Classification::TrueNegative, None),
            }
        }
        None => {
            // No ground truth - can't classify definitively
            if result.confidence >= min_confidence {
                (Classification::TruePositive, None)
            } else {
                (Classification::TrueNegative, None)
            }
        }
    }
}

/// File statistics
#[derive(Debug, Default)]
struct FileStats {
    total_files: usize,
    single_song_candidates: usize,
    album_candidates: usize,
    dual_path_candidates: usize,
    with_metadata: usize,
    failed: usize,
    with_artist: usize,
    with_title: usize,
    with_album_tag: usize,
    total_duration_hours: f64,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .init();

    let config = Config::from_args()?;

    println!("{}", "═".repeat(70));
    println!("IMPORT PIPELINE EVALUATOR");
    println!("{}", "═".repeat(70));
    println!("Root Folder:     {}", config.root_folder.display());
    println!("Min Confidence:  {:.0}%", config.min_confidence * 100.0);
    if let Some(ref gt) = config.ground_truth_file {
        println!("Ground Truth:    {}", gt.display());
    }
    println!();

    // Load ground truth if provided
    let ground_truth = if let Some(ref gt_path) = config.ground_truth_file {
        info!("Loading ground truth...");
        match load_ground_truth_file(gt_path) {
            Ok(gt) => {
                info!("Loaded {} ground truth entries", gt.len());
                Some(gt)
            }
            Err(e) => {
                warn!("Failed to load ground truth: {}", e);
                None
            }
        }
    } else {
        None
    };

    // Discover files
    let harness_config = ImportHarnessConfig::for_folder(config.root_folder.clone());
    info!("Discovering audio files...");

    let mut files = Vec::new();
    discover_files_recursive(&config.root_folder, &harness_config.extensions, &mut files)?;

    // Apply max_files limit
    if let Some(max) = config.max_files {
        if files.len() > max {
            info!(
                "Limiting to {} files (out of {} discovered)",
                max,
                files.len()
            );
            files.truncate(max);
        }
    }

    info!("Found {} audio files to process", files.len());
    println!();

    // Initialize metadata extractor
    let extractor = MetadataExtractor::new();

    // Process files
    let mut report = ImportTestReport::new(&config.root_folder);
    report.stats.files_discovered = files.len();

    let mut file_stats = FileStats::default();
    let total = files.len();
    let start_time = Instant::now();

    for (i, file_path) in files.iter().enumerate() {
        if config.verbose {
            info!("[{}/{}] Processing: {}", i + 1, total, file_path.display());
        } else if (i + 1) % 50 == 0 || i + 1 == total {
            println!(
                "[{}/{}] Progress: {:.1}%",
                i + 1,
                total,
                (i + 1) as f64 / total as f64 * 100.0
            );
        }

        // Process file
        let result = process_file(&extractor, file_path, &config);

        // Update file stats
        file_stats.total_files += 1;
        if result.error.is_none() {
            file_stats.with_metadata += 1;
            if result.matched_artist.is_some() {
                file_stats.with_artist += 1;
            }
            if result.matched_title.is_some() {
                file_stats.with_title += 1;
            }
            if let Some(duration) = result.duration_secs {
                file_stats.total_duration_hours += duration / 3600.0;

                let triage = TriagePath::from_duration(duration);
                match triage {
                    TriagePath::SingleSong => file_stats.single_song_candidates += 1,
                    TriagePath::Album => file_stats.album_candidates += 1,
                    TriagePath::DualPath => file_stats.dual_path_candidates += 1,
                }
            }
        } else {
            file_stats.failed += 1;
        }

        // Get ground truth (by full path)
        let gt = ground_truth.as_ref().and_then(|gt_map| {
            let path_str = file_path.to_string_lossy().to_string();
            gt_map.get(&path_str)
        });

        // Classify
        let (classification, expected_mbid) =
            classify_result(&result, gt, config.min_confidence);

        let evaluated = EvaluatedImportResult {
            result,
            expected_mbid,
            classification,
            ground_truth_confidence: gt.map(|g| g.get_confidence()),
        };

        report.add_result(evaluated);
    }

    let total_time = start_time.elapsed();
    report.stats.total_time_ms = total_time.as_millis() as u64;
    if total > 0 {
        report.stats.avg_time_per_file_ms = report.stats.total_time_ms / total as u64;
    }

    // Print file statistics
    println!();
    println!("{}", "═".repeat(70));
    println!("FILE DISCOVERY STATISTICS");
    println!("{}", "═".repeat(70));
    println!();
    println!("FILES BY TRIAGE PATH:");
    println!(
        "  Single Song (<12min):  {:5} ({:.1}%)",
        file_stats.single_song_candidates,
        file_stats.single_song_candidates as f64 / file_stats.total_files as f64 * 100.0
    );
    println!(
        "  Album (>25min):        {:5} ({:.1}%)",
        file_stats.album_candidates,
        file_stats.album_candidates as f64 / file_stats.total_files as f64 * 100.0
    );
    println!(
        "  Dual Path (12-25min):  {:5} ({:.1}%)",
        file_stats.dual_path_candidates,
        file_stats.dual_path_candidates as f64 / file_stats.total_files as f64 * 100.0
    );
    println!();

    println!("METADATA COVERAGE:");
    println!(
        "  With Metadata:  {:5} ({:.1}%)",
        file_stats.with_metadata,
        file_stats.with_metadata as f64 / file_stats.total_files as f64 * 100.0
    );
    println!(
        "  With Artist:    {:5} ({:.1}%)",
        file_stats.with_artist,
        file_stats.with_artist as f64 / file_stats.total_files as f64 * 100.0
    );
    println!(
        "  With Title:     {:5} ({:.1}%)",
        file_stats.with_title,
        file_stats.with_title as f64 / file_stats.total_files as f64 * 100.0
    );
    println!("  Failed:         {:5}", file_stats.failed);
    println!();

    println!(
        "TOTAL DURATION:   {:.1} hours",
        file_stats.total_duration_hours
    );
    println!(
        "PROCESSING TIME:  {:.1} seconds ({:.0} ms/file)",
        total_time.as_secs_f64(),
        if total > 0 {
            total_time.as_millis() as f64 / total as f64
        } else {
            0.0
        }
    );

    // Print main report
    println!();
    println!("{}", report.summary());

    // Save ground truth if requested
    if config.save_ground_truth {
        let gt_path = config.root_folder.join("import_ground_truth.json");
        info!("Saving ground truth to: {}", gt_path.display());

        let gt_entries: Vec<GroundTruthEntry> = report
            .results
            .iter()
            .filter(|r| r.result.confidence >= config.min_confidence)
            .map(|r| GroundTruthEntry {
                file_path: r.result.file_path.to_string_lossy().to_string(),
                album_path: String::new(),
                mbid: String::new(),
                recording_mbid: r.result.recording_mbid.clone(),
                release_mbid: r.result.release_mbid.clone(),
                content_type: Some(r.result.content_type.as_str().to_string()),
                confidence: Some(r.result.confidence),
                match_percentage: r.result.match_percentage,
                status: Some("Success".to_string()),
                artist: r.result.matched_artist.clone(),
                album: None,
            })
            .collect();

        let json = serde_json::to_string_pretty(&gt_entries)?;
        std::fs::write(&gt_path, json)?;
        info!("Saved {} ground truth entries", gt_entries.len());
    }

    // Save full results
    let results_path = config.root_folder.join("import_evaluation_results.json");
    info!("Saving results to: {}", results_path.display());
    let json = serde_json::to_string_pretty(&report.results)?;
    std::fs::write(&results_path, json)?;

    Ok(())
}

/// Recursively discover audio files
fn discover_files_recursive(
    dir: &Path,
    extensions: &[String],
    files: &mut Vec<PathBuf>,
) -> Result<()> {
    if !dir.exists() {
        anyhow::bail!("Directory does not exist: {}", dir.display());
    }

    let entries = std::fs::read_dir(dir)
        .with_context(|| format!("Failed to read directory: {}", dir.display()))?;

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_dir() {
            discover_files_recursive(&path, extensions, files)?;
        } else if path.is_file() {
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_lowercase())
                .unwrap_or_default();

            if extensions.contains(&ext) {
                files.push(path);
            }
        }
    }

    Ok(())
}
