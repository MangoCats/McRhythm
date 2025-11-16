//! Bootstrap configuration for wkmp-ai startup
//!
//! **[AIA-INIT-010]** Two-stage database initialization:
//! - Stage 1: Bootstrap - Read RESTART_REQUIRED parameters with minimal connection
//! - Stage 2: Production - Create configured pool based on Stage 1 parameters
//!
//! **RESTART_REQUIRED Parameters (IMPL016):**
//! - `ai_database_connection_pool_size` - Pool size for concurrent operations
//! - `ai_database_lock_retry_ms` - SQLite busy_timeout per connection
//! - `ai_database_max_lock_wait_ms` - Total retry budget for lock contention
//! - `ai_processing_thread_count` - Worker parallelism for import pipeline

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions, SqliteSynchronous};
use sqlx::Row;
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;
use anyhow::{Context, Result};

/// Bootstrap configuration read from settings table at startup
///
/// **Purpose:** Hold RESTART_REQUIRED parameters that affect database pool creation
///
/// **Lifecycle:**
/// 1. Read from database via single-connection bootstrap pool
/// 2. Bootstrap pool closed
/// 3. Production pool created using these parameters
#[derive(Debug, Clone)]
pub struct WkmpAiBootstrapConfig {
    /// Database connection pool size
    ///
    /// **Default:** 96 connections
    /// **Interdependency:** Should be ≥ `processing_thread_count` × 8
    /// **Memory:** ~1 MB per connection (96 connections = ~96 MB)
    pub connection_pool_size: u32,

    /// SQLite busy_timeout - time to wait for lock before error
    ///
    /// **Default:** 250 ms
    /// **Applied:** Per-connection PRAGMA on all pool connections
    pub lock_retry_ms: u64,

    /// Maximum total retry time for database operations
    ///
    /// **Default:** 5000 ms
    /// **Purpose:** Total retry budget before giving up (prevents infinite loops)
    pub max_lock_wait_ms: u64,

    /// Worker thread count for parallel import processing
    ///
    /// **Default:** CPU core count + 1 (auto-detected)
    /// **Range:** 1-64 (recommended: CPU cores × 1.5)
    pub processing_thread_count: usize,
}

impl WkmpAiBootstrapConfig {
    /// Read RESTART_REQUIRED parameters from database (Stage 1)
    ///
    /// **Database Access:**
    /// - Creates single-connection pool
    /// - Reads 4 parameters from settings table
    /// - Closes pool immediately after read
    ///
    /// **Performance:** ~10-20ms one-time overhead at startup
    ///
    /// **Error Handling:**
    /// - Missing parameters: Use compiled defaults
    /// - NULL values: Use defaults or auto-detect (thread count)
    /// - Invalid values: Return error (fail-fast on misconfiguration)
    pub async fn from_database(db_path: &Path) -> Result<Self> {
        // Stage 1: Single-connection bootstrap pool
        tracing::debug!("Creating bootstrap connection to read RESTART_REQUIRED parameters");

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect(db_path.to_str().context("Invalid database path")?)
            .await
            .context("Failed to create bootstrap database connection")?;

        // Read all RESTART_REQUIRED parameters in single query
        // Note: Using runtime query instead of compile-time macro to avoid offline compilation issues
        let row = sqlx::query(
            r#"
            SELECT
                COALESCE(
                    (SELECT value FROM settings WHERE key = 'ai_database_connection_pool_size'),
                    '96'
                ) as pool_size,
                COALESCE(
                    (SELECT value FROM settings WHERE key = 'ai_database_lock_retry_ms'),
                    '250'
                ) as lock_retry,
                COALESCE(
                    (SELECT value FROM settings WHERE key = 'ai_database_max_lock_wait_ms'),
                    '5000'
                ) as max_wait,
                (SELECT value FROM settings WHERE key = 'ai_processing_thread_count') as thread_count
            "#
        )
        .fetch_one(&pool)
        .await
        .context("Failed to read RESTART_REQUIRED parameters from settings table")?;

        // Parse parameters with validation using runtime query row access
        let pool_size_str: String = row.try_get("pool_size")
            .context("Failed to get pool_size from query result")?;
        let connection_pool_size: u32 = pool_size_str
            .parse()
            .context("Invalid ai_database_connection_pool_size (must be integer 1-500)")?;

        let lock_retry_str: String = row.try_get("lock_retry")
            .context("Failed to get lock_retry from query result")?;
        let lock_retry_ms: u64 = lock_retry_str
            .parse()
            .context("Invalid ai_database_lock_retry_ms (must be integer 50-5000)")?;

        let max_wait_str: String = row.try_get("max_wait")
            .context("Failed to get max_wait from query result")?;
        let max_lock_wait_ms: u64 = max_wait_str
            .parse()
            .context("Invalid ai_database_max_lock_wait_ms (must be integer 500-30000)")?;

        // Thread count: NULL triggers auto-detection
        let thread_count_opt: Option<String> = row.try_get("thread_count")
            .context("Failed to get thread_count from query result")?;

        let processing_thread_count = if let Some(thread_count_str) = thread_count_opt {
            thread_count_str
                .parse()
                .context("Invalid ai_processing_thread_count (must be integer 1-64)")?
        } else {
            // Auto-detect: CPU core count + 1
            let cpu_count = num_cpus::get();
            let auto_count = cpu_count + 1;
            tracing::info!(
                "ai_processing_thread_count is NULL, auto-detected: {} (CPU cores: {})",
                auto_count,
                cpu_count
            );
            auto_count
        };

        // Close bootstrap pool before returning
        pool.close().await;
        tracing::debug!("Bootstrap connection closed");

        // Validate interdependencies
        let recommended_pool_size = processing_thread_count * 8;
        if (connection_pool_size as usize) < recommended_pool_size {
            tracing::warn!(
                "ai_database_connection_pool_size ({}) is less than recommended ({}). \
                Consider setting to at least {} for optimal performance.",
                connection_pool_size,
                recommended_pool_size,
                recommended_pool_size
            );
        }

        Ok(Self {
            connection_pool_size,
            lock_retry_ms,
            max_lock_wait_ms,
            processing_thread_count,
        })
    }

    /// Create production database pool from bootstrap config (Stage 2)
    ///
    /// **Pool Configuration:**
    /// - Max connections: From `connection_pool_size` parameter
    /// - Acquire timeout: From `lock_retry_ms` parameter
    /// - Per-connection PRAGMA: busy_timeout, journal_mode, synchronous
    ///
    /// **Performance:** ~50-100ms (PRAGMA setup, connection warming)
    ///
    /// **Error Handling:** Returns error if pool creation fails
    pub async fn create_pool(&self, db_path: &Path) -> Result<SqlitePool> {
        tracing::debug!(
            "Creating production database pool: {} connections, busy_timeout={}ms",
            self.connection_pool_size,
            self.lock_retry_ms
        );

        let pool = SqlitePoolOptions::new()
            .max_connections(self.connection_pool_size)
            .acquire_timeout(Duration::from_millis(self.max_lock_wait_ms))
            .connect_with(
                SqliteConnectOptions::from_str(db_path.to_str().context("Invalid database path")?)
                    .context("Failed to parse database path")?
                    .busy_timeout(Duration::from_millis(self.lock_retry_ms))
                    .journal_mode(SqliteJournalMode::Wal)
                    .synchronous(SqliteSynchronous::Normal)
                    .create_if_missing(true)
            )
            .await
            .context("Failed to create production database pool")?;

        tracing::info!(
            "Production database pool ready: {} connections, busy_timeout={}ms, thread_count={}",
            self.connection_pool_size,
            self.lock_retry_ms,
            self.processing_thread_count
        );

        Ok(pool)
    }

    /// Get processing thread count for worker pool initialization
    pub fn processing_thread_count(&self) -> usize {
        self.processing_thread_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_bootstrap_with_defaults() {
        // Create temporary database file
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        // Initialize database with settings table
        let init_pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::from_str(db_path.to_str().unwrap())
                    .unwrap()
                    .create_if_missing(true)
            )
            .await
            .unwrap();

        wkmp_common::db::init::create_settings_table(&init_pool)
            .await
            .unwrap();

        init_pool.close().await;

        // Bootstrap should succeed with defaults
        let config = WkmpAiBootstrapConfig::from_database(&db_path).await.unwrap();

        assert_eq!(config.connection_pool_size, 96);
        assert_eq!(config.lock_retry_ms, 250);
        assert_eq!(config.max_lock_wait_ms, 5000);
        assert!(config.processing_thread_count >= 1); // Auto-detected
    }

    #[tokio::test]
    async fn test_bootstrap_with_custom_values() {
        // Create temporary database file
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        // Initialize database with settings table
        let init_pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::from_str(db_path.to_str().unwrap())
                    .unwrap()
                    .create_if_missing(true)
            )
            .await
            .unwrap();

        wkmp_common::db::init::create_settings_table(&init_pool)
            .await
            .unwrap();

        sqlx::query("INSERT INTO settings (key, value) VALUES ('ai_database_connection_pool_size', '64')")
            .execute(&init_pool)
            .await
            .unwrap();

        sqlx::query("INSERT INTO settings (key, value) VALUES ('ai_database_lock_retry_ms', '500')")
            .execute(&init_pool)
            .await
            .unwrap();

        sqlx::query("INSERT INTO settings (key, value) VALUES ('ai_database_max_lock_wait_ms', '10000')")
            .execute(&init_pool)
            .await
            .unwrap();

        sqlx::query("INSERT INTO settings (key, value) VALUES ('ai_processing_thread_count', '8')")
            .execute(&init_pool)
            .await
            .unwrap();

        init_pool.close().await;

        // Bootstrap should read custom values
        let config = WkmpAiBootstrapConfig::from_database(&db_path).await.unwrap();

        assert_eq!(config.connection_pool_size, 64);
        assert_eq!(config.lock_retry_ms, 500);
        assert_eq!(config.max_lock_wait_ms, 10000);
        assert_eq!(config.processing_thread_count, 8);
    }

    #[tokio::test]
    async fn test_create_pool() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let config = WkmpAiBootstrapConfig {
            connection_pool_size: 4,
            lock_retry_ms: 100,
            max_lock_wait_ms: 1000,
            processing_thread_count: 2,
        };

        let pool = config.create_pool(&db_path).await.unwrap();

        // Verify pool works
        let conn = pool.acquire().await.unwrap();
        drop(conn);

        pool.close().await;
    }
}
