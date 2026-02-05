//! Database connection pool manager with performance monitoring
//!
//! **[PLAN029]** Pool management with statistics tracking
//!
//! # Purpose
//! - Manage database connection pool
//! - Track connection acquisition metrics
//! - Monitor for performance bottlenecks
//! - Support future dual-pool architecture

use anyhow::Result;
use parking_lot::RwLock;
use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

/// Manages database connection pool with performance monitoring
///
/// **[PLAN029 Task 1.1]** Pool manager with statistics
///
/// Current implementation uses single pool. Can be extended to dual
/// read/write pools if benchmarking shows benefit.
pub struct PoolManager {
    /// Main connection pool
    pool: SqlitePool,
    /// Tracks pool statistics
    stats: Arc<RwLock<PoolStats>>,
}

#[derive(Default, Debug, Clone)]
struct PoolStats {
    /// Total connection acquisitions
    acquisitions: u64,
    /// Maximum wait time observed (milliseconds)
    max_wait_ms: u64,
    /// Total wait time (milliseconds)
    total_wait_ms: u64,
    /// Acquisitions that took >100ms
    slow_acquisitions: u64,
}

impl PoolManager {
    /// Create new pool manager
    ///
    /// **[PLAN029]** Initialize pool with optimal settings
    ///
    /// # Arguments
    /// * `db_path` - Path to SQLite database file
    /// * `pool_size` - Maximum number of connections
    /// * `busy_timeout_ms` - SQLite busy timeout in milliseconds
    ///
    /// # Configuration
    /// - WAL mode for concurrent readers
    /// - Busy timeout for lock handling
    /// - Connection limit enforced
    pub async fn new(db_path: &str, pool_size: u32, busy_timeout_ms: u64) -> Result<Self> {
        let connect_options = SqliteConnectOptions::from_str(db_path)?
            .busy_timeout(Duration::from_millis(busy_timeout_ms))
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .pragma("foreign_keys", "ON");

        let pool = sqlx::pool::PoolOptions::new()
            .max_connections(pool_size)
            .min_connections((pool_size / 4).max(1)) // 25% min connections
            .acquire_timeout(Duration::from_secs(30))
            .connect_with(connect_options)
            .await?;

        tracing::info!(
            pool_size,
            busy_timeout_ms,
            "PoolManager initialized with {} connections",
            pool_size
        );

        Ok(Self {
            pool,
            stats: Arc::new(RwLock::new(PoolStats::default())),
        })
    }

    /// Get database pool
    ///
    /// **[PLAN029]** Track acquisition time and log slow operations
    ///
    /// Returns the underlying SQLitePool. Caller should use pool.acquire()
    /// or pool.begin() to get connections.
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Record a connection acquisition
    ///
    /// **[PLAN029]** Statistics tracking for performance monitoring
    ///
    /// # Arguments
    /// * `wait_time` - Duration waited for connection
    pub fn record_acquisition(&self, wait_time: Duration) {
        let wait_ms = wait_time.as_millis() as u64;

        let mut stats = self.stats.write();
        stats.acquisitions += 1;
        stats.total_wait_ms += wait_ms;
        stats.max_wait_ms = stats.max_wait_ms.max(wait_ms);

        if wait_ms > 100 {
            stats.slow_acquisitions += 1;
            tracing::warn!(
                wait_ms,
                "Slow connection acquisition: {}ms (target <100ms)",
                wait_ms
            );
        }
    }

    /// Log pool statistics
    ///
    /// **[PLAN029]** Performance monitoring
    ///
    /// Logs current statistics including:
    /// - Total acquisitions
    /// - Average wait time
    /// - Maximum wait time
    /// - Slow acquisition count
    pub fn log_stats(&self) {
        let stats = self.stats.read();

        let avg_wait_ms = if stats.acquisitions > 0 {
            stats.total_wait_ms / stats.acquisitions
        } else {
            0
        };

        tracing::info!(
            acquisitions = stats.acquisitions,
            avg_wait_ms,
            max_wait_ms = stats.max_wait_ms,
            slow_acquisitions = stats.slow_acquisitions,
            "Pool statistics - Acquisitions: {} (avg {}ms, max {}ms, slow {})",
            stats.acquisitions,
            avg_wait_ms,
            stats.max_wait_ms,
            stats.slow_acquisitions
        );
    }

    /// Get current statistics
    ///
    /// Returns a snapshot of current pool statistics
    pub fn get_stats(&self) -> PoolStatistics {
        let stats = self.stats.read();
        PoolStatistics {
            total_acquisitions: stats.acquisitions,
            avg_wait_ms: if stats.acquisitions > 0 {
                stats.total_wait_ms / stats.acquisitions
            } else {
                0
            },
            max_wait_ms: stats.max_wait_ms,
            slow_acquisitions: stats.slow_acquisitions,
        }
    }

    /// Shutdown pool gracefully
    ///
    /// Closes all connections in the pool
    pub async fn shutdown(self) {
        self.log_stats();
        self.pool.close().await;
        tracing::info!("PoolManager shutdown complete");
    }
}

/// Public statistics structure
#[derive(Debug, Clone)]
pub struct PoolStatistics {
    /// Total connection acquisitions
    pub total_acquisitions: u64,
    /// Average wait time in milliseconds
    pub avg_wait_ms: u64,
    /// Maximum wait time in milliseconds
    pub max_wait_ms: u64,
    /// Number of slow acquisitions (>100ms)
    pub slow_acquisitions: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pool_manager_creation() {
        let pool_manager = PoolManager::new(":memory:", 5, 5000).await.unwrap();

        // Verify pool is usable
        let conn = pool_manager.pool().acquire().await.unwrap();
        drop(conn);

        pool_manager.shutdown().await;
    }

    #[tokio::test]
    async fn test_statistics_tracking() {
        let pool_manager = PoolManager::new(":memory:", 5, 5000).await.unwrap();

        // Record some acquisitions
        pool_manager.record_acquisition(Duration::from_millis(50));
        pool_manager.record_acquisition(Duration::from_millis(150)); // Slow
        pool_manager.record_acquisition(Duration::from_millis(75));

        let stats = pool_manager.get_stats();
        assert_eq!(stats.total_acquisitions, 3);
        assert_eq!(stats.max_wait_ms, 150);
        assert_eq!(stats.slow_acquisitions, 1); // One >100ms
        assert_eq!(stats.avg_wait_ms, (50 + 150 + 75) / 3);

        pool_manager.shutdown().await;
    }

    #[tokio::test]
    async fn test_concurrent_connections() {
        let pool_manager = Arc::new(PoolManager::new(":memory:", 10, 5000).await.unwrap());

        // Spawn 20 concurrent tasks trying to acquire connections
        let mut tasks = vec![];
        for _ in 0..20 {
            let pm = pool_manager.clone();
            tasks.push(tokio::spawn(async move {
                let start = Instant::now();
                let _conn = pm.pool().acquire().await.unwrap();
                let elapsed = start.elapsed();
                pm.record_acquisition(elapsed);
                tokio::time::sleep(Duration::from_millis(10)).await;
            }));
        }

        // Wait for all tasks
        for task in tasks {
            task.await.unwrap();
        }

        let stats = pool_manager.get_stats();
        assert_eq!(stats.total_acquisitions, 20);

        // Average wait should be reasonable (not multiple seconds)
        assert!(
            stats.avg_wait_ms < 1000,
            "Average wait too high: {}ms",
            stats.avg_wait_ms
        );

        Arc::try_unwrap(pool_manager).ok().unwrap().shutdown().await;
    }
}
