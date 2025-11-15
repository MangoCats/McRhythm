//! Connection pool monitoring utilities
//!
//! Provides instrumented versions of pool operations to track connection
//! acquisition and release timing for diagnosing pool saturation issues.

use sqlx::{Sqlite, Transaction};
use std::time::Instant;
use wkmp_common::Result;

/// Monitored transaction wrapper that logs acquisition and release timing
pub struct MonitoredTransaction<'c> {
    tx: Option<Transaction<'c, Sqlite>>,
    caller: &'static str,
    acquired_at: Instant,
}

impl<'c> MonitoredTransaction<'c> {
    /// Create a new monitored transaction
    fn new(tx: Transaction<'c, Sqlite>, caller: &'static str, acquired_at: Instant) -> Self {
        Self {
            tx: Some(tx),
            caller,
            acquired_at,
        }
    }

    /// Commit the transaction and log release timing
    pub async fn commit(mut self) -> Result<()> {
        let elapsed = self.acquired_at.elapsed();
        let tx = self.tx.take().expect("Transaction already consumed");

        tx.commit().await
            .map_err(|e| wkmp_common::Error::Database(e))?;

        tracing::debug!(
            caller = self.caller,
            held_ms = elapsed.as_millis(),
            "Connection released (commit)"
        );

        Ok(())
    }

    /// Rollback the transaction and log release timing
    pub async fn rollback(mut self) -> Result<()> {
        let elapsed = self.acquired_at.elapsed();
        let tx = self.tx.take().expect("Transaction already consumed");

        tx.rollback().await
            .map_err(|e| wkmp_common::Error::Database(e))?;

        tracing::debug!(
            caller = self.caller,
            held_ms = elapsed.as_millis(),
            "Connection released (rollback)"
        );

        Ok(())
    }

    /// Get a mutable reference to the inner transaction
    pub fn inner_mut(&mut self) -> &mut Transaction<'c, Sqlite> {
        self.tx.as_mut().expect("Transaction already consumed")
    }
}

impl<'c> Drop for MonitoredTransaction<'c> {
    fn drop(&mut self) {
        if self.tx.is_some() {
            let elapsed = self.acquired_at.elapsed();
            tracing::debug!(
                caller = self.caller,
                held_ms = elapsed.as_millis(),
                "Connection released (drop)"
            );
        }
    }
}

/// Begin a monitored transaction with connection pool timing logs
///
/// Logs:
/// - DEBUG: "Connection acquisition requested" (before pool.begin())
/// - DEBUG: "Connection acquired" with wait_ms (after pool.begin())
/// - DEBUG: "Connection released" with held_ms (on commit/rollback/drop)
///
/// # Example
/// ```ignore
/// let mut tx = begin_monitored(&pool, "passage_recorder::record").await?;
/// // ... use transaction ...
/// tx.commit().await?;
/// ```
pub async fn begin_monitored<'c>(
    pool: &'c sqlx::SqlitePool,
    caller: &'static str,
) -> Result<MonitoredTransaction<'c>> {
    let start = Instant::now();

    tracing::debug!(
        caller = caller,
        "Connection acquisition requested"
    );

    let tx = pool.begin().await
        .map_err(|e| wkmp_common::Error::Database(e))?;

    let wait_ms = start.elapsed().as_millis();

    tracing::debug!(
        caller = caller,
        wait_ms = wait_ms,
        "Connection acquired"
    );

    Ok(MonitoredTransaction::new(tx, caller, Instant::now()))
}
