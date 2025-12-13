//! Batch orchestration for test runs
//!
//! **[PLAN031 Increment 4]** Batch management and execution
//!
//! This module provides:
//! - Test run management with unique IDs
//! - Batch execution with configurable parameters
//! - Progress tracking and stop conditions
//! - Phase progression logic
//!
//! ## Requirements
//!
//! - **SPEC031-BM-010**: Batch management and execution
//! - **SPEC031-BM-020**: Three-phase progression strategy

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use uuid::Uuid;

use super::evaluation::{EvaluationEngine, EvaluationMetrics, ImportResult};
use super::ground_truth::{self, GroundTruth};
use super::types::{BatchConfig, Classification, StopReason, TestPhase};

/// Unique identifier for a test run
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RunId(pub String);

impl RunId {
    /// Create a new unique run ID
    pub fn new() -> Self {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let suffix = &Uuid::new_v4().to_string()[..8];
        Self(format!("run_{}_{}", timestamp, suffix))
    }

    /// Create a run ID from an existing string
    pub fn from_str(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

impl Default for RunId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for RunId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Status of a test run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunStatus {
    /// The run ID
    pub run_id: RunId,
    /// Current phase
    pub phase: TestPhase,
    /// Number of batches completed
    pub batches_completed: usize,
    /// Total files processed
    pub files_processed: usize,
    /// Current accuracy (0.0-1.0)
    pub current_accuracy: f64,
    /// Consecutive batches meeting accuracy threshold
    pub consecutive_passing_batches: usize,
    /// Whether the run is still active
    pub is_active: bool,
    /// Stop reason if not active
    pub stop_reason: Option<StopReason>,
}

impl RunStatus {
    fn new(run_id: RunId, phase: TestPhase) -> Self {
        Self {
            run_id,
            phase,
            batches_completed: 0,
            files_processed: 0,
            current_accuracy: 0.0,
            consecutive_passing_batches: 0,
            is_active: true,
            stop_reason: None,
        }
    }

    /// Check and update phase progression
    ///
    /// **[SPEC031-BM-020]** Three-phase progression criteria
    fn check_phase_progression(&mut self, accuracy_threshold: f64) {
        match self.phase {
            TestPhase::Phase1 => {
                // Phase 1 → Phase 2: 3+ consecutive batches at >85% accuracy
                if self.consecutive_passing_batches >= 3 && self.current_accuracy >= 0.85 {
                    self.phase = TestPhase::Phase2;
                    self.consecutive_passing_batches = 0;
                }
            }
            TestPhase::Phase2 => {
                // Phase 2 → Phase 3: 5+ batches at >90% accuracy
                if self.consecutive_passing_batches >= 5 && self.current_accuracy >= accuracy_threshold
                {
                    self.phase = TestPhase::Phase3;
                    self.consecutive_passing_batches = 0;
                }
            }
            TestPhase::Phase3 => {
                // Phase 3: Speed optimization, no further progression
            }
        }
    }
}

/// Report for a single batch execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchReport {
    /// Batch identifier (sequential within run)
    pub batch_id: usize,
    /// Run this batch belongs to
    pub run_id: RunId,
    /// Files included in this batch
    pub files_processed: Vec<PathBuf>,
    /// Results for each file
    pub results: Vec<BatchResultEntry>,
    /// Aggregated metrics
    pub metrics: EvaluationMetrics,
    /// Why the batch stopped
    pub stop_reason: StopReason,
    /// Duration of batch execution
    pub duration_ms: u64,
    /// Current phase
    pub phase: TestPhase,
    /// Consecutive failures during this batch
    pub consecutive_failures: usize,
}

/// Result entry for a single file in a batch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResultEntry {
    /// File path
    pub file_path: PathBuf,
    /// File hash
    pub file_hash: String,
    /// Ground truth for this file
    pub expected_mbid: Option<String>,
    /// Assigned MBID from import
    pub assigned_mbid: Option<String>,
    /// Classification result
    pub classification: Classification,
    /// Confidence of assignment
    pub confidence: Option<f64>,
    /// Processing time for this file
    pub processing_time_ms: u64,
}

/// Test file processor trait for abstraction
///
/// Implement this trait to connect the batch orchestrator to actual import logic
#[async_trait::async_trait]
pub trait TestFileProcessor: Send + Sync {
    /// Process a single file and return the import result
    async fn process_file(&self, file_path: &PathBuf) -> anyhow::Result<ImportResult>;
}

/// Mock processor for testing
#[derive(Debug, Default)]
pub struct MockFileProcessor {
    /// Results to return for each file hash
    results: std::collections::HashMap<String, ImportResult>,
}

impl MockFileProcessor {
    /// Create a new mock processor
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a mock result for a file hash
    pub fn add_result(&mut self, file_hash: String, result: ImportResult) {
        self.results.insert(file_hash, result);
    }
}

#[async_trait::async_trait]
impl TestFileProcessor for MockFileProcessor {
    async fn process_file(&self, file_path: &PathBuf) -> anyhow::Result<ImportResult> {
        // In real implementation, would hash the file
        // For mock, use path as key
        let path_str = file_path.to_string_lossy().to_string();
        if let Some(result) = self.results.get(&path_str) {
            Ok(result.clone())
        } else {
            // Return a default "no match" result
            Ok(ImportResult::new(
                format!("hash_{}", path_str),
                path_str,
            ))
        }
    }
}

/// Test orchestrator for managing test runs
///
/// **[SPEC031-BM-010]** Batch management and execution
pub struct TestOrchestrator {
    /// Database connection pool
    pool: Arc<SqlitePool>,
    /// File processor for running imports
    processor: Arc<dyn TestFileProcessor>,
    /// Current run status
    current_run: Option<RunStatus>,
    /// Batch reports for current run
    batch_reports: Vec<BatchReport>,
}

impl TestOrchestrator {
    /// Create a new test orchestrator
    pub fn new(pool: Arc<SqlitePool>, processor: Arc<dyn TestFileProcessor>) -> Self {
        Self {
            pool,
            processor,
            current_run: None,
            batch_reports: Vec::new(),
        }
    }

    /// Start a new test run
    ///
    /// **[SPEC031-BM-010]** Start a new test run with specified configuration
    pub async fn start_run(&mut self, config: &BatchConfig) -> anyhow::Result<RunId> {
        let run_id = RunId::new();

        // Determine starting phase based on config
        let phase = if config.batch_size <= 10 {
            TestPhase::Phase1
        } else if config.batch_size <= 100 {
            TestPhase::Phase2
        } else {
            TestPhase::Phase3
        };

        self.current_run = Some(RunStatus::new(run_id.clone(), phase));
        self.batch_reports.clear();

        Ok(run_id)
    }

    /// Process the next batch in the current run
    ///
    /// **[SPEC031-BM-010]** Process next batch in current run
    pub async fn process_batch(
        &mut self,
        config: &BatchConfig,
        file_paths: &[PathBuf],
    ) -> anyhow::Result<BatchReport> {
        let run_status = self
            .current_run
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("No active run"))?;

        if !run_status.is_active {
            return Err(anyhow::anyhow!(
                "Run is not active: {:?}",
                run_status.stop_reason
            ));
        }

        let start_time = Instant::now();
        let batch_id = run_status.batches_completed + 1;
        let run_id = run_status.run_id.clone();
        let phase = run_status.phase;

        let mut engine = EvaluationEngine::new();
        let mut results: Vec<BatchResultEntry> = Vec::new();
        let mut files_processed: Vec<PathBuf> = Vec::new();
        let mut consecutive_failures = 0;
        let mut stop_reason = StopReason::Completed;

        // Process files up to batch_size
        let files_to_process = file_paths.iter().take(config.batch_size);

        for file_path in files_to_process {
            // Get ground truth for this file
            let file_hash = format!("hash_{}", file_path.to_string_lossy());
            let ground_truth = ground_truth::get_by_hash(&self.pool, &file_hash).await?;

            // Process the file
            let import_result = self.processor.process_file(file_path).await?;

            // Create ground truth entry for evaluation (use default if none exists)
            let gt_entry = ground_truth.unwrap_or_else(|| GroundTruth {
                id: None,
                file_hash: file_hash.clone(),
                file_path: file_path.to_string_lossy().to_string(),
                expected_mbid: None,
                expected_album_mbid: None,
                verification_method: super::ground_truth::VerificationMethod::Curated,
                verified_at: Utc::now().to_rfc3339(),
                confidence: 0.0,
                notes: Some("No ground truth available".to_string()),
            });

            // Classify the result
            let classification = engine.classify_result(&gt_entry, &import_result);
            engine.record_result(&gt_entry, &import_result);

            // Track consecutive failures for Phase 1
            if classification.is_failure() {
                consecutive_failures += 1;
                if consecutive_failures >= config.failure_threshold
                    && phase == TestPhase::Phase1
                {
                    stop_reason = StopReason::FailureThreshold;
                }
            } else {
                consecutive_failures = 0;
            }

            results.push(BatchResultEntry {
                file_path: file_path.clone(),
                file_hash,
                expected_mbid: gt_entry.expected_mbid.clone(),
                assigned_mbid: import_result.assigned_mbid.clone(),
                classification,
                confidence: import_result.confidence,
                processing_time_ms: import_result.processing_time_ms,
            });

            files_processed.push(file_path.clone());

            // Check if we should stop due to failure threshold
            if matches!(stop_reason, StopReason::FailureThreshold) {
                break;
            }
        }

        let metrics = engine.calculate_metrics();
        let duration_ms = start_time.elapsed().as_millis() as u64;

        // Update run status
        run_status.batches_completed += 1;
        run_status.files_processed += files_processed.len();
        run_status.current_accuracy = metrics.accuracy;

        // Check accuracy threshold for Phase 2/3
        if metrics.accuracy >= config.accuracy_threshold {
            run_status.consecutive_passing_batches += 1;
        } else {
            run_status.consecutive_passing_batches = 0;
            if phase != TestPhase::Phase1 && metrics.accuracy < config.accuracy_threshold {
                // Don't stop, but note accuracy is below threshold
            }
        }

        // Handle phase progression
        run_status.check_phase_progression(config.accuracy_threshold);

        // Create batch report
        let report = BatchReport {
            batch_id,
            run_id,
            files_processed,
            results,
            metrics,
            stop_reason,
            duration_ms,
            phase,
            consecutive_failures,
        };

        self.batch_reports.push(report.clone());

        Ok(report)
    }

    /// Get current run status
    pub fn get_status(&self) -> Option<&RunStatus> {
        self.current_run.as_ref()
    }

    /// Get all batch reports for current run
    pub fn get_batch_reports(&self) -> &[BatchReport] {
        &self.batch_reports
    }

    /// Stop the current run
    pub fn stop_run(&mut self, reason: StopReason) {
        if let Some(ref mut status) = self.current_run {
            status.is_active = false;
            status.stop_reason = Some(reason);
        }
    }

    /// Reset and start a new run
    pub async fn reset_and_restart(&mut self, config: &BatchConfig) -> anyhow::Result<RunId> {
        self.current_run = None;
        self.batch_reports.clear();
        self.start_run(config).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Test processor that returns configurable results
    struct ConfigurableProcessor {
        results: Mutex<Vec<ImportResult>>,
        call_index: Mutex<usize>,
    }

    impl ConfigurableProcessor {
        fn new(results: Vec<ImportResult>) -> Self {
            Self {
                results: Mutex::new(results),
                call_index: Mutex::new(0),
            }
        }
    }

    #[async_trait::async_trait]
    impl TestFileProcessor for ConfigurableProcessor {
        async fn process_file(&self, file_path: &PathBuf) -> anyhow::Result<ImportResult> {
            let results = self.results.lock().unwrap();
            let mut index = self.call_index.lock().unwrap();

            let result = if *index < results.len() {
                results[*index].clone()
            } else {
                ImportResult::new(
                    format!("hash_{}", file_path.to_string_lossy()),
                    file_path.to_string_lossy().to_string(),
                )
            };

            *index += 1;
            Ok(result)
        }
    }

    async fn create_test_pool() -> SqlitePool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
        ground_truth::ensure_tables(&pool).await.unwrap();
        pool
    }

    // TC-U-BM-001: Start test run
    #[tokio::test]
    async fn test_start_run() {
        let pool = Arc::new(create_test_pool().await);
        let processor: Arc<dyn TestFileProcessor> = Arc::new(MockFileProcessor::new());
        let mut orchestrator = TestOrchestrator::new(pool, processor);

        let config = BatchConfig::phase1();
        let run_id = orchestrator.start_run(&config).await.unwrap();

        assert!(!run_id.0.is_empty());
        assert!(run_id.0.starts_with("run_"));

        let status = orchestrator.get_status().unwrap();
        assert!(status.is_active);
        assert_eq!(status.phase, TestPhase::Phase1);
        assert_eq!(status.batches_completed, 0);
    }

    // TC-U-BM-002: Process batch with all successes
    #[tokio::test]
    async fn test_process_batch_all_success() {
        let pool = Arc::new(create_test_pool().await);

        // Add ground truth entries
        for i in 0..5 {
            let gt = GroundTruth {
                id: None,
                file_hash: format!("hash_file{}.mp3", i),
                file_path: format!("file{}.mp3", i),
                expected_mbid: Some(format!("mbid_{}", i)),
                expected_album_mbid: None,
                verification_method: super::super::ground_truth::VerificationMethod::Curated,
                verified_at: Utc::now().to_rfc3339(),
                confidence: 1.0,
                notes: None,
            };
            ground_truth::upsert(&pool, &gt).await.unwrap();
        }

        // Create processor that returns matching results
        let results: Vec<ImportResult> = (0..5)
            .map(|i| {
                ImportResult::new(format!("hash_file{}.mp3", i), format!("file{}.mp3", i))
                    .with_mbid(format!("mbid_{}", i))
                    .with_confidence(0.95)
                    .with_processing_time(100)
            })
            .collect();

        let processor: Arc<dyn TestFileProcessor> = Arc::new(ConfigurableProcessor::new(results));
        let mut orchestrator = TestOrchestrator::new(pool, processor);

        let config = BatchConfig::phase1();
        orchestrator.start_run(&config).await.unwrap();

        let files: Vec<PathBuf> = (0..5)
            .map(|i| PathBuf::from(format!("file{}.mp3", i)))
            .collect();

        let report = orchestrator.process_batch(&config, &files).await.unwrap();

        assert_eq!(report.metrics.true_positives, 5);
        assert_eq!(report.metrics.false_positives, 0);
        assert_eq!(report.metrics.false_negatives, 0);
        assert!((report.metrics.accuracy - 1.0).abs() < 0.001);
        assert_eq!(report.stop_reason, StopReason::Completed);
    }

    // TC-U-BM-003: Stop on failure threshold
    #[tokio::test]
    async fn test_stop_on_failure_threshold() {
        let pool = Arc::new(create_test_pool().await);

        // Add ground truth - expect MBIDs but processor won't return them
        for i in 0..10 {
            let gt = GroundTruth {
                id: None,
                file_hash: format!("hash_file{}.mp3", i),
                file_path: format!("file{}.mp3", i),
                expected_mbid: Some(format!("expected_mbid_{}", i)),
                expected_album_mbid: None,
                verification_method: super::super::ground_truth::VerificationMethod::Curated,
                verified_at: Utc::now().to_rfc3339(),
                confidence: 1.0,
                notes: None,
            };
            ground_truth::upsert(&pool, &gt).await.unwrap();
        }

        // Create processor that returns no MBIDs (all failures)
        let results: Vec<ImportResult> = (0..10)
            .map(|i| {
                ImportResult::new(format!("hash_file{}.mp3", i), format!("file{}.mp3", i))
                // No MBID assigned - will be FALSE_NEGATIVE
            })
            .collect();

        let processor: Arc<dyn TestFileProcessor> = Arc::new(ConfigurableProcessor::new(results));
        let mut orchestrator = TestOrchestrator::new(pool, processor);

        let config = BatchConfig {
            batch_size: 10,
            failure_threshold: 3, // Stop after 3 consecutive failures
            ..BatchConfig::phase1()
        };

        orchestrator.start_run(&config).await.unwrap();

        let files: Vec<PathBuf> = (0..10)
            .map(|i| PathBuf::from(format!("file{}.mp3", i)))
            .collect();

        let report = orchestrator.process_batch(&config, &files).await.unwrap();

        // Should stop after 3 failures
        assert_eq!(report.files_processed.len(), 3);
        assert_eq!(report.consecutive_failures, 3);
        assert_eq!(report.stop_reason, StopReason::FailureThreshold);
    }

    #[tokio::test]
    async fn test_run_id_uniqueness() {
        let id1 = RunId::new();
        let id2 = RunId::new();

        assert_ne!(id1.0, id2.0);
    }

    #[tokio::test]
    async fn test_phase_determination() {
        let pool = Arc::new(create_test_pool().await);
        let processor: Arc<dyn TestFileProcessor> = Arc::new(MockFileProcessor::new());
        let mut orchestrator = TestOrchestrator::new(pool, processor);

        // Phase 1: batch_size <= 10
        let config = BatchConfig::phase1();
        orchestrator.start_run(&config).await.unwrap();
        assert_eq!(orchestrator.get_status().unwrap().phase, TestPhase::Phase1);

        // Phase 2: batch_size <= 100
        orchestrator.current_run = None;
        let config = BatchConfig::phase2();
        orchestrator.start_run(&config).await.unwrap();
        assert_eq!(orchestrator.get_status().unwrap().phase, TestPhase::Phase2);

        // Phase 3: batch_size > 100
        orchestrator.current_run = None;
        let config = BatchConfig::phase3(4);
        orchestrator.start_run(&config).await.unwrap();
        assert_eq!(orchestrator.get_status().unwrap().phase, TestPhase::Phase3);
    }

    #[tokio::test]
    async fn test_stop_run() {
        let pool = Arc::new(create_test_pool().await);
        let processor: Arc<dyn TestFileProcessor> = Arc::new(MockFileProcessor::new());
        let mut orchestrator = TestOrchestrator::new(pool, processor);

        let config = BatchConfig::phase1();
        orchestrator.start_run(&config).await.unwrap();

        assert!(orchestrator.get_status().unwrap().is_active);

        orchestrator.stop_run(StopReason::Cancelled);

        let status = orchestrator.get_status().unwrap();
        assert!(!status.is_active);
        assert_eq!(status.stop_reason, Some(StopReason::Cancelled));
    }

    #[tokio::test]
    async fn test_reset_and_restart() {
        let pool = Arc::new(create_test_pool().await);
        let processor: Arc<dyn TestFileProcessor> = Arc::new(MockFileProcessor::new());
        let mut orchestrator = TestOrchestrator::new(pool, processor);

        let config = BatchConfig::phase1();
        let run_id1 = orchestrator.start_run(&config).await.unwrap();

        let run_id2 = orchestrator.reset_and_restart(&config).await.unwrap();

        assert_ne!(run_id1.0, run_id2.0);
        assert!(orchestrator.get_status().unwrap().is_active);
        assert!(orchestrator.get_batch_reports().is_empty());
    }
}
