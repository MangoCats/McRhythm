//! # Single Song Evaluator - Cross-Validation with AcoustID
//!
//! Evaluates single-song identification using AcoustID as a confirmation source,
//! NOT as ground truth. AcoustID has gaps and errors, so we:
//!
//! 1. Run AcoustID fingerprinting
//! 2. Cross-validate with ID3 metadata (artist/title)
//! 3. Only trust high-confidence matches where sources agree
//! 4. Track coverage and disagreement rates
//!
//! Usage:
//!   cargo run --example single_song_evaluator -- --root "C:\Users\User\Music" [OPTIONS]
//!
//! Options:
//!   --root <path>         Root folder to scan (required)
//!   --max-files <n>       Maximum files to process (default: 50)
//!   --min-confidence <f>  Minimum AcoustID confidence (default: 0.70)
//!   --save-gt             Save verified matches as ground truth
//!   --verbose             Show per-file details

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::info;

use wkmp_ai::services::fingerprinter::Fingerprinter;
use wkmp_ai::services::metadata_extractor::MetadataExtractor;
use wkmp_ai::testing::import_harness::{ImportHarnessConfig, ImportTestHarness};
use wkmp_ai::utils::jaro_winkler_similarity;

/// Cross-validation result for a single file
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CrossValidationResult {
    file_path: String,
    file_hash: String,
    duration_secs: f64,

    // ID3 metadata
    id3_artist: Option<String>,
    id3_title: Option<String>,
    id3_album: Option<String>,

    // AcoustID result
    acoustid_mbid: Option<String>,
    acoustid_confidence: f64,
    acoustid_artist: Option<String>,
    acoustid_title: Option<String>,

    // Cross-validation
    artist_similarity: f64,
    title_similarity: f64,
    sources_agree: bool,

    // Final verdict
    verified_mbid: Option<String>,
    verification_confidence: f64,
    verification_method: String,

    // Timing
    processing_time_ms: u64,
    error: Option<String>,
}

/// Evaluation statistics
#[derive(Debug, Default)]
struct EvaluationStats {
    total_files: usize,
    acoustid_found: usize,
    acoustid_not_found: usize,
    acoustid_errors: usize,

    high_confidence_matches: usize,  // AcoustID ≥80%
    medium_confidence_matches: usize, // AcoustID 50-80%
    low_confidence_matches: usize,   // AcoustID <50%

    sources_agree: usize,            // ID3 + AcoustID agree
    sources_disagree: usize,         // ID3 and AcoustID conflict
    id3_missing: usize,              // No ID3 metadata to compare

    verified_ground_truth: usize,    // High confidence + agreement
}

impl EvaluationStats {
    fn print_report(&self) {
        println!("\n{}", "═".repeat(70));
        println!("SINGLE SONG CROSS-VALIDATION REPORT");
        println!("{}\n", "═".repeat(70));

        println!("ACOUSTID COVERAGE:");
        println!("  Files Processed:     {:5}", self.total_files);
        println!("  AcoustID Found:      {:5} ({:.1}%)",
            self.acoustid_found,
            self.acoustid_found as f64 / self.total_files as f64 * 100.0);
        println!("  AcoustID Not Found:  {:5} ({:.1}%)",
            self.acoustid_not_found,
            self.acoustid_not_found as f64 / self.total_files as f64 * 100.0);
        println!("  AcoustID Errors:     {:5}", self.acoustid_errors);
        println!();

        println!("ACOUSTID CONFIDENCE DISTRIBUTION:");
        println!("  High (≥80%):    {:5} ({:.1}%)",
            self.high_confidence_matches,
            self.high_confidence_matches as f64 / self.total_files as f64 * 100.0);
        println!("  Medium (50-80%): {:5} ({:.1}%)",
            self.medium_confidence_matches,
            self.medium_confidence_matches as f64 / self.total_files as f64 * 100.0);
        println!("  Low (<50%):      {:5} ({:.1}%)",
            self.low_confidence_matches,
            self.low_confidence_matches as f64 / self.total_files as f64 * 100.0);
        println!();

        println!("CROSS-VALIDATION (ID3 vs AcoustID):");
        println!("  Sources Agree:     {:5} ({:.1}%)",
            self.sources_agree,
            if self.acoustid_found > 0 {
                self.sources_agree as f64 / self.acoustid_found as f64 * 100.0
            } else { 0.0 });
        println!("  Sources Disagree:  {:5} ({:.1}%)",
            self.sources_disagree,
            if self.acoustid_found > 0 {
                self.sources_disagree as f64 / self.acoustid_found as f64 * 100.0
            } else { 0.0 });
        println!("  ID3 Missing:       {:5}", self.id3_missing);
        println!();

        println!("VERIFIED GROUND TRUTH:");
        println!("  High-confidence + Agreement: {:5} ({:.1}% of total)",
            self.verified_ground_truth,
            self.verified_ground_truth as f64 / self.total_files as f64 * 100.0);
        println!();

        // Key insight
        if self.acoustid_found > 0 {
            let agreement_rate = self.sources_agree as f64 / self.acoustid_found as f64 * 100.0;
            let coverage_rate = self.acoustid_found as f64 / self.total_files as f64 * 100.0;
            println!("KEY METRICS:");
            println!("  AcoustID Coverage:    {:.1}%", coverage_rate);
            println!("  Agreement Rate:       {:.1}% (when AcoustID finds match)", agreement_rate);
            println!("  Verification Rate:    {:.1}% (usable as ground truth)",
                self.verified_ground_truth as f64 / self.total_files as f64 * 100.0);
        }

        println!("{}", "═".repeat(70));
    }
}

/// Configuration
#[derive(Debug, Clone)]
struct Config {
    root_folder: PathBuf,
    max_files: usize,
    min_confidence: f64,
    save_ground_truth: bool,
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

        let max_files = args
            .iter()
            .enumerate()
            .find_map(|(i, arg)| {
                if arg == "--max-files" && i + 1 < args.len() {
                    args[i + 1].parse::<usize>().ok()
                } else {
                    None
                }
            })
            .unwrap_or(50);

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
            .unwrap_or(0.70);

        let save_ground_truth = args.iter().any(|a| a == "--save-gt");
        let verbose = args.iter().any(|a| a == "--verbose");

        Ok(Config {
            root_folder,
            max_files,
            min_confidence,
            save_ground_truth,
            verbose,
        })
    }
}

/// Check if artist and title are similar using Jaro-Winkler
fn check_metadata_agreement(
    id3_artist: Option<&str>,
    id3_title: Option<&str>,
    acoustid_artist: Option<&str>,
    acoustid_title: Option<&str>,
) -> (f64, f64, bool) {
    let artist_sim = match (id3_artist, acoustid_artist) {
        (Some(a), Some(b)) => jaro_winkler_similarity(a, b),
        _ => 0.0,
    };

    let title_sim = match (id3_title, acoustid_title) {
        (Some(a), Some(b)) => jaro_winkler_similarity(a, b),
        _ => 0.0,
    };

    // Sources agree if both artist AND title have ≥0.80 similarity
    let agree = artist_sim >= 0.80 && title_sim >= 0.80;

    (artist_sim, title_sim, agree)
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_level(true)
        .with_env_filter("info,wkmp_ai=debug")
        .init();

    let config = Config::from_args()?;

    println!("{}", "═".repeat(70));
    println!("SINGLE SONG CROSS-VALIDATION EVALUATOR");
    println!("{}", "═".repeat(70));
    println!("Root Folder:      {}", config.root_folder.display());
    println!("Max Files:        {}", config.max_files);
    println!("Min Confidence:   {:.0}%", config.min_confidence * 100.0);
    println!("Save Ground Truth: {}", config.save_ground_truth);
    println!();

    // Initialize services
    info!("Initializing services...");

    // Check for AcoustID API key (ENV → TOML → error)
    let api_key = std::env::var("ACOUSTID_API_KEY")
        .or_else(|_| std::env::var("WKMP_ACOUSTID_API_KEY"))
        .or_else(|_| {
            // Try reading from TOML config (~/.config/wkmp/wkmp-ai.toml on Linux, AppData on Windows)
            let config_path = if cfg!(windows) {
                std::env::var("APPDATA")
                    .map(|p| PathBuf::from(p).join("wkmp").join("wkmp-ai.toml"))
                    .ok()
            } else {
                std::env::var("HOME")
                    .map(|p| PathBuf::from(p).join(".config").join("wkmp").join("wkmp-ai.toml"))
                    .ok()
            };

            if let Some(config_path) = config_path {
                if config_path.exists() {
                    if let Ok(contents) = std::fs::read_to_string(&config_path) {
                        // Simple TOML parsing for acoustid_api_key
                        for line in contents.lines() {
                            let line = line.trim();
                            if line.starts_with("acoustid_api_key") {
                                if let Some(value) = line.split('=').nth(1) {
                                    let value = value.trim().trim_matches('"').trim_matches('\'');
                                    if !value.is_empty() && value != "new_valid_key" {
                                        return Ok(value.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Err(std::env::VarError::NotPresent)
        })
        .map_err(|_| anyhow::anyhow!(
            "AcoustID API key not found. Please set ACOUSTID_API_KEY or WKMP_ACOUSTID_API_KEY env var.\n\
             Obtain a free API key at: https://acoustid.org/api-key"
        ))?;

    info!("Using AcoustID API key: {}...", &api_key[..4.min(api_key.len())]);

    let fingerprinter = Fingerprinter::new();
    let metadata_extractor = MetadataExtractor::new();

    // Discover single-song files
    let harness_config = ImportHarnessConfig::for_folder(config.root_folder.clone());
    info!("Discovering audio files...");

    let mut all_files = Vec::new();
    discover_single_songs(&config.root_folder, &harness_config.extensions, &mut all_files)?;

    info!("Found {} potential single-song files", all_files.len());

    // Limit to max_files
    let files: Vec<_> = all_files.into_iter().take(config.max_files).collect();
    info!("Processing {} files", files.len());
    println!();

    // Process files
    let mut stats = EvaluationStats::default();
    let mut results: Vec<CrossValidationResult> = Vec::new();
    let mut disagreements: Vec<CrossValidationResult> = Vec::new();
    let mut verified: Vec<CrossValidationResult> = Vec::new();

    for (i, file_path) in files.iter().enumerate() {
        if (i + 1) % 10 == 0 || i + 1 == files.len() {
            println!("[{}/{}] Progress: {:.1}%", i + 1, files.len(), (i + 1) as f64 / files.len() as f64 * 100.0);
        }

        let result = process_file(
            file_path,
            &fingerprinter,
            &metadata_extractor,
            &api_key,
            &config,
        ).await;

        stats.total_files += 1;

        match &result.error {
            Some(_) => stats.acoustid_errors += 1,
            None => {
                if result.acoustid_mbid.is_some() {
                    stats.acoustid_found += 1;

                    // Confidence distribution
                    if result.acoustid_confidence >= 0.80 {
                        stats.high_confidence_matches += 1;
                    } else if result.acoustid_confidence >= 0.50 {
                        stats.medium_confidence_matches += 1;
                    } else {
                        stats.low_confidence_matches += 1;
                    }

                    // Cross-validation
                    if result.id3_artist.is_none() && result.id3_title.is_none() {
                        stats.id3_missing += 1;
                    } else if result.sources_agree {
                        stats.sources_agree += 1;

                        // Verified ground truth: high confidence + agreement
                        if result.acoustid_confidence >= 0.80 {
                            stats.verified_ground_truth += 1;
                            verified.push(result.clone());
                        }
                    } else {
                        stats.sources_disagree += 1;
                        disagreements.push(result.clone());
                    }
                } else {
                    stats.acoustid_not_found += 1;
                }
            }
        }

        if config.verbose && result.acoustid_mbid.is_some() {
            println!("  {} - {}",
                result.id3_artist.as_deref().unwrap_or("?"),
                result.id3_title.as_deref().unwrap_or("?"));
            println!("    AcoustID: {} (conf: {:.0}%)",
                result.acoustid_mbid.as_deref().unwrap_or("none"),
                result.acoustid_confidence * 100.0);
            println!("    Agreement: {} (artist: {:.0}%, title: {:.0}%)",
                if result.sources_agree { "YES" } else { "NO" },
                result.artist_similarity * 100.0,
                result.title_similarity * 100.0);
        }

        results.push(result);

        // Rate limiting - AcoustID has 3 req/sec limit
        tokio::time::sleep(Duration::from_millis(350)).await;
    }

    // Print report
    stats.print_report();

    // Show disagreements (potential AcoustID errors)
    if !disagreements.is_empty() {
        println!("\nDISAGREEMENTS (Potential AcoustID Errors):");
        for (i, d) in disagreements.iter().take(10).enumerate() {
            println!("{}. ID3: {} - {}", i + 1,
                d.id3_artist.as_deref().unwrap_or("?"),
                d.id3_title.as_deref().unwrap_or("?"));
            println!("   AcoustID: {} - {} (conf: {:.0}%)",
                d.acoustid_artist.as_deref().unwrap_or("?"),
                d.acoustid_title.as_deref().unwrap_or("?"),
                d.acoustid_confidence * 100.0);
            println!("   Similarity: artist={:.0}%, title={:.0}%",
                d.artist_similarity * 100.0,
                d.title_similarity * 100.0);
        }
        if disagreements.len() > 10 {
            println!("... and {} more disagreements", disagreements.len() - 10);
        }
    }

    // Save results
    let results_path = config.root_folder.join("single_song_crossval_results.json");
    info!("Saving results to: {}", results_path.display());
    let json = serde_json::to_string_pretty(&results)?;
    std::fs::write(&results_path, json)?;

    // Save verified ground truth
    if config.save_ground_truth && !verified.is_empty() {
        let gt_path = config.root_folder.join("single_song_ground_truth.json");
        info!("Saving {} verified ground truth entries to: {}", verified.len(), gt_path.display());
        let json = serde_json::to_string_pretty(&verified)?;
        std::fs::write(&gt_path, json)?;
    }

    Ok(())
}

/// Process a single file
async fn process_file(
    file_path: &Path,
    fingerprinter: &Fingerprinter,
    metadata_extractor: &MetadataExtractor,
    api_key: &str,
    _config: &Config,
) -> CrossValidationResult {
    let start = Instant::now();

    // Compute file hash
    let file_hash = ImportTestHarness::compute_file_hash(file_path)
        .unwrap_or_else(|_| "unknown".to_string());

    // Extract ID3 metadata
    let metadata = metadata_extractor.extract(file_path).ok();
    let duration_secs = metadata.as_ref()
        .and_then(|m| m.duration_seconds)
        .unwrap_or(0.0);

    let id3_artist = metadata.as_ref().and_then(|m| m.artist.clone());
    let id3_title = metadata.as_ref().and_then(|m| m.title.clone());
    let id3_album = metadata.as_ref().and_then(|m| m.album.clone());

    // Generate fingerprint
    let fingerprint = match fingerprinter.fingerprint_file(file_path) {
        Ok(fp) => fp,
        Err(e) => {
            return CrossValidationResult {
                file_path: file_path.to_string_lossy().to_string(),
                file_hash,
                duration_secs,
                id3_artist,
                id3_title,
                id3_album,
                acoustid_mbid: None,
                acoustid_confidence: 0.0,
                acoustid_artist: None,
                acoustid_title: None,
                artist_similarity: 0.0,
                title_similarity: 0.0,
                sources_agree: false,
                verified_mbid: None,
                verification_confidence: 0.0,
                verification_method: "none".to_string(),
                processing_time_ms: start.elapsed().as_millis() as u64,
                error: Some(format!("Fingerprint error: {}", e)),
            };
        }
    };

    // Query AcoustID directly (bypassing our client to avoid DB dependency)
    let acoustid_result = query_acoustid_direct(api_key, &fingerprint, duration_secs as u64).await;

    let (acoustid_mbid, acoustid_confidence, acoustid_artist, acoustid_title) = match acoustid_result {
        Ok(r) => r,
        Err(e) => {
            return CrossValidationResult {
                file_path: file_path.to_string_lossy().to_string(),
                file_hash,
                duration_secs,
                id3_artist,
                id3_title,
                id3_album,
                acoustid_mbid: None,
                acoustid_confidence: 0.0,
                acoustid_artist: None,
                acoustid_title: None,
                artist_similarity: 0.0,
                title_similarity: 0.0,
                sources_agree: false,
                verified_mbid: None,
                verification_confidence: 0.0,
                verification_method: "none".to_string(),
                processing_time_ms: start.elapsed().as_millis() as u64,
                error: Some(format!("AcoustID error: {}", e)),
            };
        }
    };

    // Cross-validate
    let (artist_sim, title_sim, sources_agree) = check_metadata_agreement(
        id3_artist.as_deref(),
        id3_title.as_deref(),
        acoustid_artist.as_deref(),
        acoustid_title.as_deref(),
    );

    // Determine verified MBID
    let (verified_mbid, verification_confidence, verification_method) =
        if acoustid_mbid.is_some() && acoustid_confidence >= 0.80 && sources_agree {
            (acoustid_mbid.clone(), acoustid_confidence, "cross_validated".to_string())
        } else if acoustid_mbid.is_some() && acoustid_confidence >= 0.95 {
            // Very high AcoustID confidence can stand alone
            (acoustid_mbid.clone(), acoustid_confidence * 0.9, "acoustid_high".to_string())
        } else {
            (None, 0.0, "unverified".to_string())
        };

    CrossValidationResult {
        file_path: file_path.to_string_lossy().to_string(),
        file_hash,
        duration_secs,
        id3_artist,
        id3_title,
        id3_album,
        acoustid_mbid,
        acoustid_confidence,
        acoustid_artist,
        acoustid_title,
        artist_similarity: artist_sim,
        title_similarity: title_sim,
        sources_agree,
        verified_mbid,
        verification_confidence,
        verification_method,
        processing_time_ms: start.elapsed().as_millis() as u64,
        error: None,
    }
}

/// Query AcoustID API directly using POST (fingerprints are too long for GET URLs)
async fn query_acoustid_direct(
    api_key: &str,
    fingerprint: &str,
    duration: u64,
) -> Result<(Option<String>, f64, Option<String>, Option<String>)> {
    let client = reqwest::Client::builder()
        .user_agent("WKMP/0.1.0 (single_song_evaluator)")
        .timeout(Duration::from_secs(10))
        .build()?;

    // Use POST with form encoding - fingerprints are too long for GET URLs
    let params = [
        ("client", api_key),
        ("meta", "recordings"),
        ("duration", &duration.to_string()),
        ("fingerprint", fingerprint),
    ];

    let response: serde_json::Value = client
        .post("https://api.acoustid.org/v2/lookup")
        .form(&params)
        .send()
        .await?
        .json()
        .await?;

    // Check for API error
    if let Some(error) = response.get("error") {
        let msg = error.get("message").and_then(|m| m.as_str()).unwrap_or("unknown");
        anyhow::bail!("AcoustID API error: {}", msg);
    }

    // Parse response
    if let Some(results) = response.get("results").and_then(|r| r.as_array()) {
        for result in results {
            let score = result.get("score").and_then(|s| s.as_f64()).unwrap_or(0.0);

            if let Some(recordings) = result.get("recordings").and_then(|r| r.as_array()) {
                if let Some(recording) = recordings.first() {
                    let mbid = recording.get("id").and_then(|i| i.as_str()).map(|s| s.to_string());
                    let title = recording.get("title").and_then(|t| t.as_str()).map(|s| s.to_string());

                    // Get first artist
                    let artist = recording.get("artists")
                        .and_then(|a| a.as_array())
                        .and_then(|a| a.first())
                        .and_then(|a| a.get("name"))
                        .and_then(|n| n.as_str())
                        .map(|s| s.to_string());

                    return Ok((mbid, score, artist, title));
                }
            }
        }
    }

    Ok((None, 0.0, None, None))
}

/// Discover single-song files (< 12 minutes based on file size heuristic)
fn discover_single_songs(
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
            discover_single_songs(&path, extensions, files)?;
        } else if path.is_file() {
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_lowercase())
                .unwrap_or_default();

            if extensions.contains(&ext) {
                // Rough filter: files < 50MB are likely single songs
                if let Ok(metadata) = std::fs::metadata(&path) {
                    if metadata.len() < 50_000_000 {
                        files.push(path);
                    }
                }
            }
        }
    }

    Ok(())
}
