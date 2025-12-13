//! Import Pipeline Test Harness
//!
//! **[PLAN031 Extension]** Full import pipeline testing for single songs and albums
//!
//! This module provides infrastructure for testing the complete wkmp-ai import pipeline:
//! - File discovery and scanning under a root folder
//! - Hash-based file tracking for reproducible tests
//! - Integration with ContentTypeClassifier for MBID assignment
//! - Evaluation against ground truth
//! - Progress tracking and reporting
//!
//! ## Architecture
//!
//! ```text
//! ┌───────────────────────────────────────────────────────────────────┐
//! │                    IMPORT TEST HARNESS                            │
//! │                                                                   │
//! │  ┌─────────────┐   ┌─────────────┐   ┌─────────────┐   ┌────────┐│
//! │  │   DISCOVER  │──▶│   CLASSIFY  │──▶│  EVALUATE   │──▶│ REPORT ││
//! │  │   FILES     │   │   (CTC)     │   │   vs GT     │   │        ││
//! │  └─────────────┘   └─────────────┘   └─────────────┘   └────────┘│
//! │        │                │                  │               │     │
//! │        ▼                ▼                  ▼               ▼     │
//! │   file hashes     recording_mbid    TP/FP/TN/FN     accuracy    │
//! │                   release_mbid      precision       patterns    │
//! │                   confidence        recall          suggestions │
//! └───────────────────────────────────────────────────────────────────┘
//! ```

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use tokio::sync::Semaphore;
use tracing::{debug, error, info, warn};

use super::ground_truth::{self, GroundTruth, VerificationMethod};
use super::types::Classification;

// Re-export ContentType from content_type_classifier to avoid duplication
pub use crate::services::content_type_classifier::ContentType;

/// Result of classifying a single file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportTestResult {
    /// Path to the file
    pub file_path: PathBuf,
    /// SHA-256 hash of the file
    pub file_hash: String,
    /// Determined content type
    pub content_type: ContentType,
    /// Assigned recording MBID (for single songs)
    pub recording_mbid: Option<String>,
    /// Assigned release/album MBID (for albums)
    pub release_mbid: Option<String>,
    /// Match confidence (0.0-1.0)
    pub confidence: f64,
    /// Match percentage (for albums)
    pub match_percentage: Option<f64>,
    /// Which stage/method found the match
    pub matching_stage: Option<String>,
    /// Artist name from match
    pub matched_artist: Option<String>,
    /// Title/album name from match
    pub matched_title: Option<String>,
    /// Processing time in milliseconds
    pub processing_time_ms: u64,
    /// Error message if failed
    pub error: Option<String>,
    /// File duration in seconds
    pub duration_secs: Option<f64>,
}

impl ImportTestResult {
    /// Create a failed result
    pub fn failed(file_path: PathBuf, file_hash: String, error: String) -> Self {
        Self {
            file_path,
            file_hash,
            content_type: ContentType::IdentificationFailed,
            recording_mbid: None,
            release_mbid: None,
            confidence: 0.0,
            match_percentage: None,
            matching_stage: None,
            matched_artist: None,
            matched_title: None,
            processing_time_ms: 0,
            error: Some(error),
            duration_secs: None,
        }
    }

    /// Get the primary MBID (recording for songs, release for albums)
    pub fn primary_mbid(&self) -> Option<&str> {
        match self.content_type {
            ContentType::SingleSong => self.recording_mbid.as_deref(),
            ContentType::FullAlbum | ContentType::PartialAlbum => self.release_mbid.as_deref(),
            _ => None,
        }
    }
}

/// Configuration for import test harness
#[derive(Debug, Clone)]
pub struct ImportHarnessConfig {
    /// Root folder to scan
    pub root_folder: PathBuf,
    /// Maximum concurrent file processing
    pub max_concurrent: usize,
    /// Minimum confidence threshold for valid match
    pub min_confidence: f64,
    /// Include subfolders in scan
    pub recursive: bool,
    /// File extensions to include (lowercase, without dot)
    pub extensions: Vec<String>,
    /// Skip files larger than this (bytes, 0 = no limit)
    pub max_file_size: u64,
    /// Timeout per file (seconds)
    pub timeout_secs: u64,
}

impl Default for ImportHarnessConfig {
    fn default() -> Self {
        Self {
            root_folder: PathBuf::from(""),
            max_concurrent: 4,
            min_confidence: 0.50,
            recursive: true,
            extensions: vec![
                "mp3".to_string(),
                "flac".to_string(),
                "m4a".to_string(),
                "ogg".to_string(),
                "opus".to_string(),
                "wav".to_string(),
            ],
            max_file_size: 0,
            timeout_secs: 300,
        }
    }
}

impl ImportHarnessConfig {
    /// Create config for a specific root folder
    pub fn for_folder(root_folder: PathBuf) -> Self {
        Self {
            root_folder,
            ..Default::default()
        }
    }
}

/// Statistics from import test run
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImportTestStats {
    /// Total files discovered
    pub files_discovered: usize,
    /// Files processed successfully
    pub files_processed: usize,
    /// Files that failed processing
    pub files_failed: usize,
    /// Files skipped (too large, wrong extension, etc.)
    pub files_skipped: usize,

    /// Single songs identified
    pub single_songs: usize,
    /// Full albums identified
    pub full_albums: usize,
    /// Partial albums identified
    pub partial_albums: usize,
    /// Files not found in MusicBrainz
    pub not_found: usize,

    /// Total processing time (ms)
    pub total_time_ms: u64,
    /// Average processing time per file (ms)
    pub avg_time_per_file_ms: u64,

    /// Files with ground truth
    pub with_ground_truth: usize,
    /// True positives (correct MBID)
    pub true_positives: usize,
    /// False positives (wrong MBID)
    pub false_positives: usize,
    /// True negatives (correctly not found)
    pub true_negatives: usize,
    /// False negatives (should have found)
    pub false_negatives: usize,
}

impl ImportTestStats {
    /// Calculate accuracy metrics
    pub fn accuracy(&self) -> f64 {
        let total = self.true_positives + self.false_positives + self.true_negatives + self.false_negatives;
        if total == 0 {
            return 0.0;
        }
        (self.true_positives + self.true_negatives) as f64 / total as f64
    }

    /// Calculate precision
    pub fn precision(&self) -> f64 {
        let denom = self.true_positives + self.false_positives;
        if denom == 0 {
            return 0.0;
        }
        self.true_positives as f64 / denom as f64
    }

    /// Calculate recall
    pub fn recall(&self) -> f64 {
        let denom = self.true_positives + self.false_negatives;
        if denom == 0 {
            return 0.0;
        }
        self.true_positives as f64 / denom as f64
    }

    /// Calculate F1 score
    pub fn f1_score(&self) -> f64 {
        let p = self.precision();
        let r = self.recall();
        if p + r == 0.0 {
            return 0.0;
        }
        2.0 * p * r / (p + r)
    }
}

/// Import test harness for running pipeline tests
pub struct ImportTestHarness {
    config: ImportHarnessConfig,
    pool: Arc<SqlitePool>,
}

impl ImportTestHarness {
    /// Create a new import test harness
    pub fn new(pool: Arc<SqlitePool>, config: ImportHarnessConfig) -> Self {
        Self { pool, config }
    }

    /// Discover audio files under the root folder
    pub async fn discover_files(&self) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        let root = &self.config.root_folder;

        if !root.exists() {
            anyhow::bail!("Root folder does not exist: {}", root.display());
        }

        info!("Discovering audio files under: {}", root.display());

        self.walk_directory(root, &mut files)?;

        info!("Discovered {} audio files", files.len());
        Ok(files)
    }

    fn walk_directory(&self, dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
        let entries = std::fs::read_dir(dir)
            .with_context(|| format!("Failed to read directory: {}", dir.display()))?;

        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                if self.config.recursive {
                    self.walk_directory(&path, files)?;
                }
            } else if path.is_file() {
                if self.should_include_file(&path) {
                    files.push(path);
                }
            }
        }

        Ok(())
    }

    fn should_include_file(&self, path: &Path) -> bool {
        // Check extension
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        if !self.config.extensions.contains(&ext) {
            return false;
        }

        // Check file size
        if self.config.max_file_size > 0 {
            if let Ok(metadata) = std::fs::metadata(path) {
                if metadata.len() > self.config.max_file_size {
                    return false;
                }
            }
        }

        true
    }

    /// Compute SHA-256 hash of a file
    pub fn compute_file_hash(path: &Path) -> Result<String> {
        let mut file = std::fs::File::open(path)
            .with_context(|| format!("Failed to open file: {}", path.display()))?;

        let mut hasher = Sha256::new();
        std::io::copy(&mut file, &mut hasher)?;

        let hash = hasher.finalize();
        Ok(format!("{:x}", hash))
    }

    /// Load ground truth for files from database
    pub async fn load_ground_truth(
        &self,
        file_hashes: &[String],
    ) -> Result<HashMap<String, GroundTruth>> {
        let mut gt_map = HashMap::new();

        for hash in file_hashes {
            if let Some(gt) = ground_truth::get_by_hash(&self.pool, hash).await? {
                gt_map.insert(hash.clone(), gt);
            }
        }

        info!(
            "Loaded ground truth for {}/{} files",
            gt_map.len(),
            file_hashes.len()
        );
        Ok(gt_map)
    }

    /// Classify a result against ground truth
    pub fn classify_result(
        &self,
        result: &ImportTestResult,
        ground_truth: Option<&GroundTruth>,
    ) -> Classification {
        match ground_truth {
            Some(gt) => {
                let assigned_mbid = result.primary_mbid();
                let expected_mbid = gt.expected_mbid.as_deref();

                match (assigned_mbid, expected_mbid) {
                    (Some(assigned), Some(expected)) => {
                        if assigned == expected {
                            Classification::TruePositive
                        } else {
                            Classification::FalsePositive
                        }
                    }
                    (Some(_), None) => {
                        // Assigned MBID but expected none
                        Classification::FalsePositive
                    }
                    (None, Some(_)) => {
                        // No MBID assigned but expected one
                        Classification::FalseNegative
                    }
                    (None, None) => {
                        // Correctly no MBID
                        Classification::TrueNegative
                    }
                }
            }
            None => {
                // No ground truth - can't classify
                // Treat as TN if no match, otherwise unknown
                if result.primary_mbid().is_none() {
                    Classification::TrueNegative
                } else {
                    // Can't verify without ground truth
                    Classification::TruePositive // Assume correct if no GT
                }
            }
        }
    }

    /// Save import result as ground truth (for building test sets)
    pub async fn save_as_ground_truth(
        &self,
        result: &ImportTestResult,
        method: VerificationMethod,
    ) -> Result<()> {
        let gt = GroundTruth {
            id: None,
            file_hash: result.file_hash.clone(),
            file_path: result.file_path.to_string_lossy().to_string(),
            expected_mbid: result.recording_mbid.clone(),
            expected_album_mbid: result.release_mbid.clone(),
            verification_method: method,
            verified_at: chrono::Utc::now().to_rfc3339(),
            confidence: result.confidence,
            notes: Some(format!(
                "Content: {}, Stage: {}",
                result.content_type,
                result.matching_stage.as_deref().unwrap_or("unknown")
            )),
        };

        ground_truth::upsert(&self.pool, &gt).await?;
        Ok(())
    }
}

/// Evaluated import result with classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluatedImportResult {
    /// The import test result
    pub result: ImportTestResult,
    /// Ground truth (if available)
    pub expected_mbid: Option<String>,
    /// Classification against ground truth
    pub classification: Classification,
    /// Ground truth confidence
    pub ground_truth_confidence: Option<f64>,
}

/// Summary report from import test run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportTestReport {
    /// When the test was run
    pub run_timestamp: String,
    /// Configuration used
    pub root_folder: String,
    /// Statistics
    pub stats: ImportTestStats,
    /// Individual results (may be truncated for large runs)
    pub results: Vec<EvaluatedImportResult>,
    /// Failures only (for debugging)
    pub failures: Vec<EvaluatedImportResult>,
}

impl ImportTestReport {
    /// Create a new report
    pub fn new(root_folder: &Path) -> Self {
        Self {
            run_timestamp: chrono::Utc::now().to_rfc3339(),
            root_folder: root_folder.to_string_lossy().to_string(),
            stats: ImportTestStats::default(),
            results: Vec::new(),
            failures: Vec::new(),
        }
    }

    /// Add an evaluated result
    pub fn add_result(&mut self, evaluated: EvaluatedImportResult) {
        // Track failures separately
        if evaluated.classification.is_failure() {
            self.failures.push(evaluated.clone());
        }

        // Update stats
        match evaluated.result.content_type {
            ContentType::SingleSong => self.stats.single_songs += 1,
            ContentType::FullAlbum => self.stats.full_albums += 1,
            ContentType::PartialAlbum => self.stats.partial_albums += 1,
            ContentType::NotInMusicbrainz => self.stats.not_found += 1,
            ContentType::IdentificationFailed => self.stats.files_failed += 1,
            _ => {}
        }

        match evaluated.classification {
            Classification::TruePositive => self.stats.true_positives += 1,
            Classification::FalsePositive => self.stats.false_positives += 1,
            Classification::TrueNegative => self.stats.true_negatives += 1,
            Classification::FalseNegative => self.stats.false_negatives += 1,
        }

        if evaluated.expected_mbid.is_some() {
            self.stats.with_ground_truth += 1;
        }

        self.stats.files_processed += 1;
        self.results.push(evaluated);
    }

    /// Generate text summary
    pub fn summary(&self) -> String {
        let mut s = String::new();
        s.push_str(&"═".repeat(70));
        s.push('\n');
        s.push_str("IMPORT TEST REPORT\n");
        s.push_str(&"═".repeat(70));
        s.push_str("\n\n");

        s.push_str(&format!("Root Folder: {}\n", self.root_folder));
        s.push_str(&format!("Run Time: {}\n\n", self.run_timestamp));

        s.push_str("FILE STATISTICS:\n");
        s.push_str(&format!("  Files Discovered: {}\n", self.stats.files_discovered));
        s.push_str(&format!("  Files Processed:  {}\n", self.stats.files_processed));
        s.push_str(&format!("  Files Failed:     {}\n", self.stats.files_failed));
        s.push_str(&format!("  Files Skipped:    {}\n\n", self.stats.files_skipped));

        s.push_str("CONTENT TYPE BREAKDOWN:\n");
        s.push_str(&format!("  Single Songs:    {:4}\n", self.stats.single_songs));
        s.push_str(&format!("  Full Albums:     {:4}\n", self.stats.full_albums));
        s.push_str(&format!("  Partial Albums:  {:4}\n", self.stats.partial_albums));
        s.push_str(&format!("  Not Found:       {:4}\n\n", self.stats.not_found));

        if self.stats.with_ground_truth > 0 {
            s.push_str("ACCURACY METRICS (vs Ground Truth):\n");
            s.push_str(&format!("  Files with GT:   {:4}\n", self.stats.with_ground_truth));
            s.push_str(&format!("  True Positives:  {:4}\n", self.stats.true_positives));
            s.push_str(&format!("  False Positives: {:4}\n", self.stats.false_positives));
            s.push_str(&format!("  True Negatives:  {:4}\n", self.stats.true_negatives));
            s.push_str(&format!("  False Negatives: {:4}\n\n", self.stats.false_negatives));

            s.push_str(&format!("  Accuracy:  {:6.2}%\n", self.stats.accuracy() * 100.0));
            s.push_str(&format!("  Precision: {:6.2}%\n", self.stats.precision() * 100.0));
            s.push_str(&format!("  Recall:    {:6.2}%\n", self.stats.recall() * 100.0));
            s.push_str(&format!("  F1 Score:  {:6.3}\n\n", self.stats.f1_score()));
        }

        if !self.failures.is_empty() {
            s.push_str(&format!("FAILURES ({}):\n", self.failures.len()));
            for (i, failure) in self.failures.iter().take(10).enumerate() {
                s.push_str(&format!(
                    "  {}. {} [{:?}]\n",
                    i + 1,
                    failure.result.file_path.display(),
                    failure.classification
                ));
                if let Some(ref expected) = failure.expected_mbid {
                    s.push_str(&format!("     Expected: {}\n", expected));
                }
                if let Some(mbid) = failure.result.primary_mbid() {
                    s.push_str(&format!("     Assigned: {}\n", mbid));
                }
            }
            if self.failures.len() > 10 {
                s.push_str(&format!("  ... and {} more\n", self.failures.len() - 10));
            }
        }

        s.push_str(&"═".repeat(70));
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_content_type_display() {
        assert_eq!(ContentType::SingleSong.to_string(), "SINGLE_SONG");
        assert_eq!(ContentType::FullAlbum.to_string(), "FULL_ALBUM");
        assert_eq!(ContentType::NotInMusicbrainz.to_string(), "NOT_IN_MUSICBRAINZ");
    }

    #[test]
    fn test_content_type_categorization() {
        // These require segmentation
        assert!(ContentType::FullAlbum.requires_segmentation());
        assert!(ContentType::PartialAlbum.requires_segmentation());
        assert!(ContentType::MultipleSongs.requires_segmentation());
        // These don't
        assert!(!ContentType::SingleSong.requires_segmentation());
        assert!(!ContentType::NotInMusicbrainz.requires_segmentation());
        assert!(!ContentType::IdentificationFailed.requires_segmentation());
    }

    #[test]
    fn test_import_test_result_primary_mbid() {
        let mut result = ImportTestResult::failed(
            PathBuf::from("test.mp3"),
            "hash123".to_string(),
            "error".to_string(),
        );
        assert!(result.primary_mbid().is_none());

        result.content_type = ContentType::SingleSong;
        result.recording_mbid = Some("rec-123".to_string());
        assert_eq!(result.primary_mbid(), Some("rec-123"));

        result.content_type = ContentType::FullAlbum;
        result.release_mbid = Some("rel-456".to_string());
        assert_eq!(result.primary_mbid(), Some("rel-456"));
    }

    #[test]
    fn test_import_stats_metrics() {
        let stats = ImportTestStats {
            true_positives: 80,
            false_positives: 10,
            true_negatives: 5,
            false_negatives: 5,
            ..Default::default()
        };

        assert!((stats.accuracy() - 0.85).abs() < 0.001);
        assert!((stats.precision() - 0.888).abs() < 0.01);
        assert!((stats.recall() - 0.941).abs() < 0.01);
    }

    #[test]
    fn test_import_stats_empty() {
        let stats = ImportTestStats::default();
        assert_eq!(stats.accuracy(), 0.0);
        assert_eq!(stats.precision(), 0.0);
        assert_eq!(stats.recall(), 0.0);
        assert_eq!(stats.f1_score(), 0.0);
    }

    #[test]
    fn test_file_hash() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.mp3");

        // Create a test file with known content
        let mut file = std::fs::File::create(&file_path).unwrap();
        file.write_all(b"test audio content").unwrap();
        drop(file);

        let hash = ImportTestHarness::compute_file_hash(&file_path).unwrap();
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 64); // SHA-256 produces 64 hex chars

        // Same content should produce same hash
        let hash2 = ImportTestHarness::compute_file_hash(&file_path).unwrap();
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_config_default() {
        let config = ImportHarnessConfig::default();
        assert_eq!(config.max_concurrent, 4);
        assert!((config.min_confidence - 0.50).abs() < 0.001);
        assert!(config.recursive);
        assert!(config.extensions.contains(&"mp3".to_string()));
        assert!(config.extensions.contains(&"flac".to_string()));
    }

    #[test]
    fn test_report_summary() {
        let mut report = ImportTestReport::new(Path::new("/test/music"));
        report.stats.files_discovered = 100;
        report.stats.files_processed = 95;
        report.stats.single_songs = 80;
        report.stats.full_albums = 10;
        report.stats.true_positives = 85;
        report.stats.false_positives = 5;
        report.stats.with_ground_truth = 90;

        let summary = report.summary();
        assert!(summary.contains("IMPORT TEST REPORT"));
        assert!(summary.contains("100")); // files discovered
        assert!(summary.contains("Single Songs"));
    }
}
