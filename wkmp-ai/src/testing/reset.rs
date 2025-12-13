//! Database reset operations for test isolation
//!
//! **[PLAN031 Increment 7]** Database reset functionality
//!
//! This module provides:
//! - Full database reset (preserves ground_truth)
//! - Test data only reset
//! - Batch-specific reset
//! - Incremental mode (no reset)
//!
//! ## Requirements
//!
//! - **SPEC031-DB-010**: Database reset modes

use anyhow::Result;
use sqlx::SqlitePool;
use tracing::{info, warn};

use super::types::ResetMode;

/// Reset the database according to the specified mode
///
/// **[SPEC031-DB-010]** Database reset implementation
///
/// # Arguments
///
/// * `pool` - Database connection pool
/// * `mode` - Reset mode to apply
///
/// # Returns
///
/// Number of rows affected
pub async fn reset_for_testing(pool: &SqlitePool, mode: ResetMode) -> Result<ResetResult> {
    match mode {
        ResetMode::Full => reset_full(pool).await,
        ResetMode::TestDataOnly => reset_test_data(pool).await,
        ResetMode::BatchOnly { batch_id } => reset_batch(pool, &batch_id).await,
        ResetMode::Incremental => Ok(ResetResult::no_reset()),
    }
}

/// Result of a reset operation
#[derive(Debug, Clone, Default)]
pub struct ResetResult {
    /// Number of test_results rows deleted
    pub test_results_deleted: usize,
    /// Number of test_runs rows deleted
    pub test_runs_deleted: usize,
    /// Number of files rows deleted (test imports)
    pub files_deleted: usize,
    /// Number of passages rows deleted
    pub passages_deleted: usize,
    /// Whether reset was performed
    pub was_reset: bool,
    /// Description of reset performed
    pub description: String,
}

impl ResetResult {
    fn no_reset() -> Self {
        Self {
            was_reset: false,
            description: "Incremental mode - no reset performed".to_string(),
            ..Default::default()
        }
    }

    /// Total rows affected
    pub fn total_affected(&self) -> usize {
        self.test_results_deleted
            + self.test_runs_deleted
            + self.files_deleted
            + self.passages_deleted
    }
}

/// Full reset - clears all test data, preserves ground_truth
async fn reset_full(pool: &SqlitePool) -> Result<ResetResult> {
    info!("Performing full database reset (preserving ground_truth)");

    let mut result = ResetResult {
        was_reset: true,
        description: "Full reset - cleared all test data".to_string(),
        ..Default::default()
    };

    // Delete test_results first (references test_runs)
    let test_results = sqlx::query("DELETE FROM test_results")
        .execute(pool)
        .await;

    if let Ok(res) = test_results {
        result.test_results_deleted = res.rows_affected() as usize;
    }

    // Delete test_runs
    let test_runs = sqlx::query("DELETE FROM test_runs")
        .execute(pool)
        .await;

    if let Ok(res) = test_runs {
        result.test_runs_deleted = res.rows_affected() as usize;
    }

    // Delete passages from test imports
    let passages = sqlx::query("DELETE FROM passages WHERE file_id IN (SELECT id FROM files WHERE import_session LIKE 'test_%')")
        .execute(pool)
        .await;

    if let Ok(res) = passages {
        result.passages_deleted = res.rows_affected() as usize;
    }

    // Delete files from test imports
    let files = sqlx::query("DELETE FROM files WHERE import_session LIKE 'test_%'")
        .execute(pool)
        .await;

    if let Ok(res) = files {
        result.files_deleted = res.rows_affected() as usize;
    }

    info!(
        "Full reset complete: {} test_results, {} test_runs, {} passages, {} files deleted",
        result.test_results_deleted,
        result.test_runs_deleted,
        result.passages_deleted,
        result.files_deleted
    );

    Ok(result)
}

/// Test data only reset - clears test results but keeps ground_truth and settings
async fn reset_test_data(pool: &SqlitePool) -> Result<ResetResult> {
    info!("Performing test data reset (preserving ground_truth and settings)");

    let mut result = ResetResult {
        was_reset: true,
        description: "Test data reset - cleared test results only".to_string(),
        ..Default::default()
    };

    // Delete test_results
    let test_results = sqlx::query("DELETE FROM test_results")
        .execute(pool)
        .await;

    if let Ok(res) = test_results {
        result.test_results_deleted = res.rows_affected() as usize;
    }

    // Delete test_runs
    let test_runs = sqlx::query("DELETE FROM test_runs")
        .execute(pool)
        .await;

    if let Ok(res) = test_runs {
        result.test_runs_deleted = res.rows_affected() as usize;
    }

    info!(
        "Test data reset complete: {} test_results, {} test_runs deleted",
        result.test_results_deleted, result.test_runs_deleted
    );

    Ok(result)
}

/// Batch-specific reset - clears only results from a specific batch
async fn reset_batch(pool: &SqlitePool, batch_id: &str) -> Result<ResetResult> {
    info!("Performing batch reset for batch_id: {}", batch_id);

    let mut result = ResetResult {
        was_reset: true,
        description: format!("Batch reset - cleared batch {}", batch_id),
        ..Default::default()
    };

    // Delete test_results for this batch
    let test_results = sqlx::query("DELETE FROM test_results WHERE run_id = ?")
        .bind(batch_id)
        .execute(pool)
        .await;

    if let Ok(res) = test_results {
        result.test_results_deleted = res.rows_affected() as usize;
    }

    // Delete the test_run entry
    let test_runs = sqlx::query("DELETE FROM test_runs WHERE id = ?")
        .bind(batch_id)
        .execute(pool)
        .await;

    if let Ok(res) = test_runs {
        result.test_runs_deleted = res.rows_affected() as usize;
    }

    if result.total_affected() == 0 {
        warn!("No data found for batch_id: {}", batch_id);
    }

    info!(
        "Batch reset complete: {} test_results, {} test_runs deleted for batch {}",
        result.test_results_deleted, result.test_runs_deleted, batch_id
    );

    Ok(result)
}

/// Check if test tables exist
pub async fn tables_exist(pool: &SqlitePool) -> Result<bool> {
    let result = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('ground_truth', 'test_runs', 'test_results')"
    )
    .fetch_one(pool)
    .await?;

    Ok(result == 3)
}

/// Get current database statistics for testing tables
pub async fn get_test_statistics(pool: &SqlitePool) -> Result<TestStatistics> {
    let mut stats = TestStatistics::default();

    // Count ground_truth entries
    if let Ok(count) = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM ground_truth")
        .fetch_one(pool)
        .await
    {
        stats.ground_truth_entries = count as usize;
    }

    // Count test_runs
    if let Ok(count) = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM test_runs")
        .fetch_one(pool)
        .await
    {
        stats.test_runs = count as usize;
    }

    // Count test_results
    if let Ok(count) = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM test_results")
        .fetch_one(pool)
        .await
    {
        stats.test_results = count as usize;
    }

    Ok(stats)
}

/// Statistics about test tables
#[derive(Debug, Clone, Default)]
pub struct TestStatistics {
    /// Number of ground truth entries
    pub ground_truth_entries: usize,
    /// Number of test runs
    pub test_runs: usize,
    /// Number of test results
    pub test_results: usize,
}

impl TestStatistics {
    /// Check if any test data exists
    pub fn has_test_data(&self) -> bool {
        self.test_runs > 0 || self.test_results > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::ground_truth;

    async fn create_test_pool() -> SqlitePool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();
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
            )"
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
            )"
        )
        .execute(&pool)
        .await
        .unwrap();

        // Create files table (for full reset testing)
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS files (
                id TEXT PRIMARY KEY,
                import_session TEXT,
                path TEXT
            )"
        )
        .execute(&pool)
        .await
        .unwrap();

        // Create passages table
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS passages (
                id TEXT PRIMARY KEY,
                file_id TEXT
            )"
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    // TC-U-DB-001: Full reset preserves ground_truth
    #[tokio::test]
    async fn test_full_reset_preserves_ground_truth() {
        let pool = create_test_pool().await;

        // Add ground truth entry
        let gt = crate::testing::GroundTruth {
            id: None,
            file_hash: "test_hash".to_string(),
            file_path: "test.mp3".to_string(),
            expected_mbid: Some("test_mbid".to_string()),
            expected_album_mbid: None,
            verification_method: crate::testing::VerificationMethod::Curated,
            verified_at: "2025-01-01".to_string(),
            confidence: 1.0,
            notes: None,
        };
        ground_truth::upsert(&pool, &gt).await.unwrap();

        // Verify it exists
        let count_before = ground_truth::count_total(&pool).await.unwrap();
        assert_eq!(count_before, 1);

        // Perform full reset
        let result = reset_for_testing(&pool, ResetMode::Full).await.unwrap();
        assert!(result.was_reset);

        // Ground truth should still exist
        let count_after = ground_truth::count_total(&pool).await.unwrap();
        assert_eq!(count_after, 1);
    }

    // TC-U-DB-002: Test data reset
    #[tokio::test]
    async fn test_data_reset() {
        let pool = create_test_pool().await;

        // Add a test run
        sqlx::query("INSERT INTO test_runs (id, phase, config_json, started_at) VALUES ('run_1', 'Phase1', '{}', '2025-01-01')")
            .execute(&pool)
            .await
            .unwrap();

        // Verify it exists
        let count_before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM test_runs")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count_before, 1);

        // Perform test data reset
        let result = reset_for_testing(&pool, ResetMode::TestDataOnly)
            .await
            .unwrap();
        assert!(result.was_reset);
        assert_eq!(result.test_runs_deleted, 1);

        // Test run should be gone
        let count_after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM test_runs")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count_after, 0);
    }

    #[tokio::test]
    async fn test_incremental_mode_no_reset() {
        let pool = create_test_pool().await;

        // Add a test run
        sqlx::query("INSERT INTO test_runs (id, phase, config_json, started_at) VALUES ('run_1', 'Phase1', '{}', '2025-01-01')")
            .execute(&pool)
            .await
            .unwrap();

        // Perform incremental (no reset)
        let result = reset_for_testing(&pool, ResetMode::Incremental)
            .await
            .unwrap();
        assert!(!result.was_reset);
        assert_eq!(result.total_affected(), 0);

        // Test run should still exist
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM test_runs")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_batch_reset() {
        let pool = create_test_pool().await;

        // Add two test runs
        sqlx::query("INSERT INTO test_runs (id, phase, config_json, started_at) VALUES ('run_1', 'Phase1', '{}', '2025-01-01')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO test_runs (id, phase, config_json, started_at) VALUES ('run_2', 'Phase1', '{}', '2025-01-01')")
            .execute(&pool)
            .await
            .unwrap();

        // Reset only run_1
        let result = reset_for_testing(
            &pool,
            ResetMode::BatchOnly {
                batch_id: "run_1".to_string(),
            },
        )
        .await
        .unwrap();

        assert!(result.was_reset);
        assert_eq!(result.test_runs_deleted, 1);

        // Only run_2 should remain
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM test_runs")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_tables_exist() {
        let pool = create_test_pool().await;
        let exists = tables_exist(&pool).await.unwrap();
        assert!(exists);
    }

    #[tokio::test]
    async fn test_get_statistics() {
        let pool = create_test_pool().await;

        // Add ground truth
        let gt = crate::testing::GroundTruth {
            id: None,
            file_hash: "hash_1".to_string(),
            file_path: "test.mp3".to_string(),
            expected_mbid: Some("mbid_1".to_string()),
            expected_album_mbid: None,
            verification_method: crate::testing::VerificationMethod::Curated,
            verified_at: "2025-01-01".to_string(),
            confidence: 1.0,
            notes: None,
        };
        ground_truth::upsert(&pool, &gt).await.unwrap();

        // Add test run
        sqlx::query("INSERT INTO test_runs (id, phase, config_json, started_at) VALUES ('run_1', 'Phase1', '{}', '2025-01-01')")
            .execute(&pool)
            .await
            .unwrap();

        let stats = get_test_statistics(&pool).await.unwrap();
        assert_eq!(stats.ground_truth_entries, 1);
        assert_eq!(stats.test_runs, 1);
        assert!(stats.has_test_data());
    }

    #[tokio::test]
    async fn test_reset_result_total() {
        let result = ResetResult {
            test_results_deleted: 5,
            test_runs_deleted: 2,
            files_deleted: 3,
            passages_deleted: 10,
            was_reset: true,
            description: "test".to_string(),
        };

        assert_eq!(result.total_affected(), 20);
    }
}
