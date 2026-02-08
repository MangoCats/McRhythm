//! Integration tests for the closed-loop testing system
//!
//! **[PLAN031 Increment 9]** End-to-end testing of all components
//!
//! Tests the full closed-loop cycle:
//! 1. Load ground truth
//! 2. Process batch with orchestrator
//! 3. Evaluate results
//! 4. Analyze failures
//! 5. Generate reports
//! 6. Reset and repeat

#[cfg(test)]
mod tests {
    use crate::testing::{
        batch::{BatchReport, MockFileProcessor, RunId, TestFileProcessor, TestOrchestrator},
        evaluation::{EvaluationEngine, EvaluationMetrics, ImportResult},
        failure::{FailureAnalyzer, FailureAnalysis},
        ground_truth::{self, GroundTruth, VerificationMethod},
        hooks::TestEvaluator,
        reporting::{ProgressReport, ReportFormatter},
        reset::{reset_for_testing, ResetResult},
        types::{BatchConfig, Classification, ResetMode, StopReason, TestPhase},
    };
    use std::path::PathBuf;
    use std::sync::Arc;
    use sqlx::SqlitePool;

    /// Create in-memory test database with all required tables
    async fn create_test_db() -> SqlitePool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();

        // Create ground_truth table
        ground_truth::ensure_tables(&pool).await.unwrap();

        // Create test_runs table
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS test_runs (
                id TEXT PRIMARY KEY,
                phase TEXT NOT NULL,
                config_json TEXT NOT NULL,
                started_at TEXT NOT NULL,
                completed_at TEXT,
                files_processed INTEGER DEFAULT 0,
                accuracy REAL
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        // Create test_results table
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS test_results (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                run_id TEXT NOT NULL,
                file_hash TEXT NOT NULL,
                classification TEXT NOT NULL,
                expected_mbid TEXT,
                assigned_mbid TEXT,
                confidence REAL,
                FOREIGN KEY (run_id) REFERENCES test_runs(id)
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        // Create files table (for reset testing)
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS files (
                id TEXT PRIMARY KEY,
                import_session TEXT,
                path TEXT
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        // Create passages table
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS passages (
                id TEXT PRIMARY KEY,
                file_id TEXT
            )",
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    /// Helper to create ground truth entries
    fn make_gt(hash: &str, mbid: Option<&str>) -> GroundTruth {
        GroundTruth {
            id: None,
            file_hash: hash.to_string(),
            file_path: format!("/test/{}.mp3", hash),
            expected_mbid: mbid.map(String::from),
            expected_album_mbid: None,
            verification_method: VerificationMethod::Curated,
            verified_at: "2025-01-01".to_string(),
            confidence: 1.0,
            notes: None,
        }
    }

    // =========================================================================
    // TC-I-001: Evaluation Engine with Ground Truth
    // =========================================================================

    #[tokio::test]
    async fn test_evaluation_engine_with_ground_truth() {
        let pool = create_test_db().await;

        // Load ground truth
        let gt1 = make_gt("hash_1", Some("mbid_correct_1"));
        let gt2 = make_gt("hash_2", Some("mbid_correct_2"));
        let gt3 = make_gt("hash_3", None); // No MBID expected
        ground_truth::upsert(&pool, &gt1).await.unwrap();
        ground_truth::upsert(&pool, &gt2).await.unwrap();
        ground_truth::upsert(&pool, &gt3).await.unwrap();

        assert_eq!(ground_truth::count_total(&pool).await.unwrap(), 3);

        // Create import results
        let result1 = ImportResult::new("hash_1".to_string(), "file1.mp3".to_string())
            .with_mbid("mbid_correct_1") // TP - correct match
            .with_confidence(0.95);

        let result2 = ImportResult::new("hash_2".to_string(), "file2.mp3".to_string())
            .with_mbid("mbid_wrong") // FP - wrong match
            .with_confidence(0.85);

        let result3 = ImportResult::new("hash_3".to_string(), "file3.mp3".to_string());
        // No MBID assigned - TN (expected None, got None)

        // Evaluate using EvaluationEngine
        let mut engine = EvaluationEngine::new();

        let class1 = engine.classify_result(&gt1, &result1);
        engine.record_result(&gt1, &result1);
        assert_eq!(class1, Classification::TruePositive);

        let class2 = engine.classify_result(&gt2, &result2);
        engine.record_result(&gt2, &result2);
        assert_eq!(class2, Classification::FalsePositive);

        let class3 = engine.classify_result(&gt3, &result3);
        engine.record_result(&gt3, &result3);
        assert_eq!(class3, Classification::TrueNegative);

        // Calculate metrics
        let metrics = engine.calculate_metrics();
        assert_eq!(metrics.true_positives, 1);
        assert_eq!(metrics.false_positives, 1);
        assert_eq!(metrics.true_negatives, 1);
        assert_eq!(metrics.false_negatives, 0);

        // Accuracy = (TP + TN) / Total = 2/3 ≈ 0.667
        assert!((metrics.accuracy - 0.667).abs() < 0.01);
    }

    // =========================================================================
    // TC-I-002: Batch Reset Between Runs
    // =========================================================================

    #[tokio::test]
    async fn test_batch_reset_between_runs() {
        let pool = create_test_db().await;

        // Create some ground truth
        let gt = make_gt("hash_reset", Some("mbid_reset"));
        ground_truth::upsert(&pool, &gt).await.unwrap();

        // Insert test data
        sqlx::query("INSERT INTO test_runs (id, phase, config_json, started_at) VALUES ('run_reset_1', 'Phase1', '{}', '2025-01-01')")
            .execute(&pool)
            .await
            .unwrap();

        // Verify data exists
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM test_runs")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 1);

        // Reset test data
        let reset_result = reset_for_testing(&pool, ResetMode::TestDataOnly).await.unwrap();
        assert!(reset_result.was_reset);
        assert_eq!(reset_result.test_runs_deleted, 1);

        // Verify test data cleared
        let count_after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM test_runs")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count_after, 0);

        // Ground truth should still exist
        let gt_count = ground_truth::count_total(&pool).await.unwrap();
        assert_eq!(gt_count, 1);
    }

    // =========================================================================
    // TC-I-003: Orchestrator with Mock Processor
    // =========================================================================

    #[tokio::test]
    async fn test_orchestrator_integration() {
        let pool = Arc::new(create_test_db().await);

        // Setup ground truth
        for i in 0..5 {
            let gt = GroundTruth {
                id: None,
                file_hash: format!("hash_file{}.mp3", i),
                file_path: format!("file{}.mp3", i),
                expected_mbid: Some(format!("mbid_{}", i)),
                expected_album_mbid: None,
                verification_method: VerificationMethod::Curated,
                verified_at: "2025-01-01".to_string(),
                confidence: 1.0,
                notes: None,
            };
            ground_truth::upsert(&pool, &gt).await.unwrap();
        }

        // Create mock processor with matching results
        let mut processor = MockFileProcessor::new();
        for i in 0..5 {
            let result = ImportResult::new(
                format!("hash_file{}.mp3", i),
                format!("file{}.mp3", i),
            )
            .with_mbid(format!("mbid_{}", i))
            .with_confidence(0.95)
            .with_processing_time(100);

            processor.add_result(format!("file{}.mp3", i), result);
        }

        let processor: Arc<dyn TestFileProcessor> = Arc::new(processor);
        let mut orchestrator = TestOrchestrator::new(pool, processor);

        let config = BatchConfig::phase1();
        let run_id = orchestrator.start_run(&config).await.unwrap();

        assert!(!run_id.0.is_empty());
        assert!(run_id.0.starts_with("run_"));

        // Process files
        let files: Vec<PathBuf> = (0..5)
            .map(|i| PathBuf::from(format!("file{}.mp3", i)))
            .collect();

        let report = orchestrator.process_batch(&config, &files).await.unwrap();

        assert_eq!(report.metrics.true_positives, 5);
        assert_eq!(report.stop_reason, StopReason::Completed);
    }

    // =========================================================================
    // TC-I-004: Test Evaluator Hook Integration
    // =========================================================================

    #[tokio::test]
    async fn test_evaluator_hook_integration() {
        let evaluator = TestEvaluator::new();

        // Simulate import pipeline events
        evaluator
            .record_fusion_result(
                "hook_test_hash",
                Some("mbid_hook"),
                0.95,
                vec!["acoustid".to_string(), "musicbrainz".to_string()],
            )
            .await;

        evaluator
            .record_classification("hook_test_hash", "single_song", 0.95)
            .await;

        evaluator
            .record_api_call("hook_test_hash", "acoustid", true, 100)
            .await;

        evaluator
            .record_processing_complete("hook_test_hash", true, None, 150)
            .await;

        // Finalize and get summary
        let summary = evaluator.finalize_batch().await;

        assert_eq!(summary.files_processed, 1);
        assert_eq!(summary.files_identified, 1);
        assert_eq!(summary.total_api_calls, 1);
        assert!(summary.total_time_ms > 0);

        // Check file-specific events
        let file_events = evaluator.get_events_for_file("hook_test_hash").await;
        assert_eq!(file_events.len(), 4);
    }

    // =========================================================================
    // TC-I-005: Failure Analysis with Patterns
    // =========================================================================

    #[tokio::test]
    async fn test_failure_analysis_patterns_integration() {
        use crate::testing::batch::BatchResultEntry;
        use crate::testing::types::FailureCategory;

        // Create batch result entries with failures
        let mut results: Vec<BatchResultEntry> = Vec::new();

        // 3 TP entries
        for i in 0..3 {
            results.push(BatchResultEntry {
                file_path: PathBuf::from(format!("file_{}.mp3", i)),
                file_hash: format!("hash_{}", i),
                expected_mbid: Some(format!("mbid_{}", i)),
                assigned_mbid: Some(format!("mbid_{}", i)),
                classification: Classification::TruePositive,
                confidence: Some(0.9),
                processing_time_ms: 100,
            });
        }

        // 7 FP entries (wrong recordings)
        for i in 3..10 {
            results.push(BatchResultEntry {
                file_path: PathBuf::from(format!("file_{}.mp3", i)),
                file_hash: format!("hash_{}", i),
                expected_mbid: Some(format!("expected_{}", i)),
                assigned_mbid: Some(format!("wrong_{}", i)),
                classification: Classification::FalsePositive,
                confidence: Some(0.85),
                processing_time_ms: 100,
            });
        }

        // Analyze failures
        let analyzer = FailureAnalyzer::with_threshold(10.0); // 10% threshold
        let analysis = analyzer.analyze(&results);

        // 70% FP rate should be detected
        assert_eq!(analysis.total_failures, 7);
        assert!(!analysis.patterns.is_empty());

        // Should have WrongRecording pattern
        let wrong_pattern = analysis
            .patterns
            .iter()
            .find(|p| p.category == FailureCategory::WrongRecording);
        assert!(wrong_pattern.is_some());

        let pattern = wrong_pattern.unwrap();
        assert_eq!(pattern.occurrence_count, 7);
        assert!((pattern.percentage - 100.0).abs() < 0.01); // All failures are WrongRecording
    }

    // =========================================================================
    // TC-I-006: Progress Report Across Multiple Batches
    // =========================================================================

    #[test]
    fn test_progress_report_across_batches() {
        use crate::testing::batch::BatchResultEntry;

        // Create batch reports with improving accuracy
        let reports: Vec<BatchReport> = vec![
            create_test_batch_report(1, 0.7),
            create_test_batch_report(2, 0.8),
            create_test_batch_report(3, 0.9),
        ];

        // Create progress report
        let progress = ProgressReport::from_batches(&reports);

        assert_eq!(progress.batches_completed, 3);
        assert_eq!(progress.accuracy_history.len(), 3);

        // Verify trend is improving
        assert!(progress.accuracy_trend > 0.0);

        // Format report
        let output = ReportFormatter::format_progress_report(&progress);

        assert!(output.contains("CUMULATIVE PROGRESS"));
        assert!(output.contains("3")); // batches completed
    }

    // =========================================================================
    // TC-I-007: Compact Summary Format
    // =========================================================================

    #[test]
    fn test_compact_summary_format() {
        let metrics = EvaluationMetrics {
            true_positives: 8,
            false_positives: 1,
            true_negatives: 0,
            false_negatives: 1,
            accuracy: 0.8,
            precision: 8.0 / 9.0,
            recall: 8.0 / 9.0,
            f1_score: 8.0 / 9.0,
            avg_confidence_correct: 0.9,
            avg_confidence_incorrect: 0.6,
            confidence_correlation: 0.3,
            avg_time_per_file_ms: 100,
            total_batch_time_ms: 1000,
            api_calls_per_file: 2.0,
        };

        let compact = ReportFormatter::format_compact_summary(&metrics);

        // Should be single line with key metrics
        assert!(!compact.contains('\n'));
        assert!(compact.contains("Accuracy"));
        assert!(compact.contains("Precision"));
        assert!(compact.contains("Recall"));
    }

    // =========================================================================
    // TC-I-008: Incremental Mode Preserves Data
    // =========================================================================

    #[tokio::test]
    async fn test_incremental_mode_preserves_data() {
        let pool = create_test_db().await;

        // Create test data
        sqlx::query("INSERT INTO test_runs (id, phase, config_json, started_at) VALUES ('run_inc_1', 'Phase1', '{}', '2025-01-01')")
            .execute(&pool)
            .await
            .unwrap();

        let count_before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM test_runs")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count_before, 1);

        // Reset with incremental mode (no reset)
        let reset_result = reset_for_testing(&pool, ResetMode::Incremental).await.unwrap();
        assert!(!reset_result.was_reset);
        assert_eq!(reset_result.total_affected(), 0);

        // Data should still exist
        let count_after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM test_runs")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count_after, 1);
    }

    // =========================================================================
    // TC-I-009: Full Reset Preserves Ground Truth
    // =========================================================================

    #[tokio::test]
    async fn test_full_reset_preserves_ground_truth() {
        let pool = create_test_db().await;

        // Add ground truth
        let gt = make_gt("test_hash", Some("test_mbid"));
        ground_truth::upsert(&pool, &gt).await.unwrap();

        // Add test run
        sqlx::query("INSERT INTO test_runs (id, phase, config_json, started_at) VALUES ('run_1', 'Phase1', '{}', '2025-01-01')")
            .execute(&pool)
            .await
            .unwrap();

        // Full reset
        let reset_result = reset_for_testing(&pool, ResetMode::Full).await.unwrap();
        assert!(reset_result.was_reset);

        // Test runs should be cleared
        let run_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM test_runs")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(run_count, 0);

        // Ground truth should remain
        let gt_count = ground_truth::count_total(&pool).await.unwrap();
        assert_eq!(gt_count, 1);
    }

    // =========================================================================
    // TC-I-010: Batch Report with Failure Analysis
    // =========================================================================

    #[test]
    fn test_batch_report_with_failure_analysis() {
        use crate::testing::batch::BatchResultEntry;

        let report = create_test_batch_report(1, 0.7);

        // Create failure entries for analysis
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

        assert!(output.contains("BATCH REPORT"));
        assert!(output.contains("FAILURE ANALYSIS"));
    }

    // =========================================================================
    // TC-I-011: Phase Determination by Batch Size
    // =========================================================================

    #[tokio::test]
    async fn test_phase_determination_by_batch_size() {
        let pool = Arc::new(create_test_db().await);
        let processor: Arc<dyn TestFileProcessor> = Arc::new(MockFileProcessor::new());

        // Phase 1: batch_size <= 10
        let config1 = BatchConfig::phase1();
        let mut orchestrator = TestOrchestrator::new(Arc::clone(&pool), Arc::clone(&processor));
        orchestrator.start_run(&config1).await.unwrap();
        assert_eq!(orchestrator.get_status().unwrap().phase, TestPhase::Phase1);

        // Phase 2: batch_size <= 100
        let config2 = BatchConfig::phase2();
        let mut orchestrator2 = TestOrchestrator::new(Arc::clone(&pool), Arc::clone(&processor));
        orchestrator2.start_run(&config2).await.unwrap();
        assert_eq!(orchestrator2.get_status().unwrap().phase, TestPhase::Phase2);

        // Phase 3: batch_size > 100
        let config3 = BatchConfig::phase3(4);
        let mut orchestrator3 = TestOrchestrator::new(Arc::clone(&pool), processor);
        orchestrator3.start_run(&config3).await.unwrap();
        assert_eq!(orchestrator3.get_status().unwrap().phase, TestPhase::Phase3);
    }

    // =========================================================================
    // TC-I-012: Evaluator Enable/Disable
    // =========================================================================

    #[tokio::test]
    async fn test_evaluator_enable_disable() {
        let evaluator = TestEvaluator::new();

        // Initially enabled
        assert!(evaluator.is_enabled().await);

        // Record event while enabled
        evaluator
            .record_fusion_result("hash_1", Some("mbid_1"), 0.9, vec![])
            .await;
        assert_eq!(evaluator.get_events().await.len(), 1);

        // Disable and try to record
        evaluator.disable().await;
        assert!(!evaluator.is_enabled().await);

        evaluator
            .record_fusion_result("hash_2", Some("mbid_2"), 0.8, vec![])
            .await;
        // Should not record while disabled
        assert_eq!(evaluator.get_events().await.len(), 1);

        // Re-enable
        evaluator.enable().await;
        evaluator
            .record_fusion_result("hash_3", Some("mbid_3"), 0.7, vec![])
            .await;
        assert_eq!(evaluator.get_events().await.len(), 2);
    }

    // =========================================================================
    // Helper Functions
    // =========================================================================

    fn create_test_batch_report(batch_id: usize, accuracy: f64) -> BatchReport {
        let tp = (accuracy * 10.0) as usize;
        let fn_ = 10 - tp;

        BatchReport {
            batch_id,
            run_id: RunId::from_str("test_run"),
            files_processed: (0..10)
                .map(|i| PathBuf::from(format!("file_{}.mp3", i)))
                .collect(),
            results: vec![],
            metrics: EvaluationMetrics {
                true_positives: tp,
                false_positives: 0,
                true_negatives: 0,
                false_negatives: fn_,
                accuracy,
                precision: if tp > 0 { tp as f64 / tp as f64 } else { 0.0 },
                recall: if tp + fn_ > 0 {
                    tp as f64 / (tp + fn_) as f64
                } else {
                    0.0
                },
                f1_score: accuracy,
                avg_confidence_correct: 0.9,
                avg_confidence_incorrect: 0.5,
                confidence_correlation: 0.4,
                avg_time_per_file_ms: 100,
                total_batch_time_ms: 1000,
                api_calls_per_file: 2.0,
            },
            stop_reason: StopReason::Completed,
            duration_ms: 5000,
            phase: TestPhase::Phase1,
            consecutive_failures: 0,
        }
    }
}
