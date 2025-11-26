//! # Query Statistics and Heartbeat Logging
//!
//! Thread-safe statistics tracking for API queries with periodic heartbeat logging.
//!
//! ## Features
//! - **Atomic counters**: Total queries, successes, failures, retries, rate limit waits
//! - **Activity tracking**: Current operation description for heartbeat display
//! - **Background heartbeat**: Logs statistics every 120 seconds until stopped
//! - **Thread-safe**: AtomicU64 for counters, Mutex for activity string
//!
//! ## Usage
//! ```ignore
//! let stats = Arc::new(QueryStats::new());
//! let heartbeat_handle = spawn_heartbeat_task(stats.clone(), album_idx);
//!
//! stats.record_query_start();
//! stats.set_activity("Fetching MusicBrainz data");
//! // ... perform query ...
//! stats.record_success();
//!
//! stats.stop();  // Signal heartbeat task to exit
//! heartbeat_handle.await?;  // Wait for graceful shutdown
//! ```
//!
//! ## Heartbeat Output Format
//! ```text
//! [A1] [HEARTBEAT] 120s elapsed | Queries: 45 (42 ok, 3 failed) | Retries: 7 | Rate waits: 38 | Fetching MusicBrainz data
//! ```

use crate::constants::HEARTBEAT_INTERVAL_SECS;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::time::{sleep, Duration};
use tracing::info;

// =============================================================================
// Query Statistics Tracking
// =============================================================================

/// Thread-safe query statistics tracker for API monitoring
///
/// Provides atomic counters for query metrics and background heartbeat logging.
/// All counters use Relaxed ordering for performance (statistics don't require
/// strict memory ordering).
///
/// # Thread Safety
/// - AtomicU64 counters: lock-free concurrent access
/// - Mutex<String> activity: synchronized updates
/// - AtomicBool stop_flag: lock-free signaling
///
/// # Example
/// ```ignore
/// let stats = QueryStats::new();
/// stats.record_query_start();
/// stats.set_activity("Querying MusicBrainz");
/// stats.record_success();
/// stats.log_heartbeat(0);  // Manual heartbeat
/// ```
pub(crate) struct QueryStats {
    /// Total API queries attempted.
    total_queries: AtomicU64,
    /// Successful queries completed.
    successful_queries: AtomicU64,
    /// Failed queries (after all retries exhausted).
    failed_queries: AtomicU64,
    /// Total retry attempts across all queries.
    retries: AtomicU64,
    /// Number of rate limit waits performed.
    rate_limit_waits: AtomicU64,
    /// Current activity description for heartbeat display.
    current_activity: Mutex<String>,
    /// Start time for elapsed calculation.
    start_time: Instant,
    /// Flag to signal heartbeat task to stop.
    stop_flag: AtomicBool,
}

impl QueryStats {
    /// Create new QueryStats with all counters initialized to zero
    ///
    /// Sets initial activity to "initializing" and records current time
    /// as start_time for elapsed duration calculation.
    pub(crate) fn new() -> Self {
        Self {
            total_queries: AtomicU64::new(0),
            successful_queries: AtomicU64::new(0),
            failed_queries: AtomicU64::new(0),
            retries: AtomicU64::new(0),
            rate_limit_waits: AtomicU64::new(0),
            current_activity: Mutex::new("initializing".to_string()),
            start_time: Instant::now(),
            stop_flag: AtomicBool::new(false),
        }
    }

    /// Increment total query counter (called when query starts)
    pub(crate) fn record_query_start(&self) {
        self.total_queries.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment successful query counter
    pub(crate) fn record_success(&self) {
        self.successful_queries.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment failed query counter
    pub(crate) fn record_failure(&self) {
        self.failed_queries.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment retry attempt counter
    pub(crate) fn record_retry(&self) {
        self.retries.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment rate limit wait counter
    pub(crate) fn record_rate_wait(&self) {
        self.rate_limit_waits.fetch_add(1, Ordering::Relaxed);
    }

    /// Update current activity description for heartbeat display
    ///
    /// # Arguments
    /// * `activity` - Description of current operation (e.g., "Fetching MusicBrainz data")
    ///
    /// # Note
    /// Silently ignores mutex poisoning (unlikely in practice)
    pub(crate) fn set_activity(&self, activity: &str) {
        if let Ok(mut guard) = self.current_activity.lock() {
            *guard = activity.to_string();
        }
    }

    /// Get current activity description
    ///
    /// # Returns
    /// Current activity string, or empty string if mutex is poisoned
    pub(crate) fn get_activity(&self) -> String {
        self.current_activity.lock().map(|g| g.clone()).unwrap_or_default()
    }

    /// Signal heartbeat task to stop logging
    ///
    /// Sets stop_flag to true, causing background heartbeat task to exit
    /// on next iteration. Non-blocking.
    pub(crate) fn stop(&self) {
        self.stop_flag.store(true, Ordering::Relaxed);
    }

    /// Check if stop signal has been set
    ///
    /// # Returns
    /// true if stop() has been called
    pub(crate) fn is_stopped(&self) -> bool {
        self.stop_flag.load(Ordering::Relaxed)
    }

    /// Log current statistics snapshot with album context
    ///
    /// Outputs all counters and current activity in structured format.
    /// Called periodically by background heartbeat task.
    ///
    /// # Arguments
    /// * `album_idx` - Zero-based album index (displayed as 1-based)
    ///
    /// # Output Format
    /// ```text
    /// [A1] [HEARTBEAT] 120s elapsed | Queries: 45 (42 ok, 3 failed) | Retries: 7 | Rate waits: 38 | Fetching MusicBrainz data
    /// ```
    pub(crate) fn log_heartbeat(&self, album_idx: usize) {
        let elapsed = self.start_time.elapsed().as_secs();
        let total = self.total_queries.load(Ordering::Relaxed);
        let success = self.successful_queries.load(Ordering::Relaxed);
        let failed = self.failed_queries.load(Ordering::Relaxed);
        let retries = self.retries.load(Ordering::Relaxed);
        let waits = self.rate_limit_waits.load(Ordering::Relaxed);
        let activity = self.get_activity();

        info!(
            "[A{}] [HEARTBEAT] {}s elapsed | Queries: {} ({} ok, {} failed) | Retries: {} | Rate waits: {} | {}",
            album_idx + 1, elapsed, total, success, failed, retries, waits, activity
        );
    }
}

// =============================================================================
// Background Heartbeat Task
// =============================================================================

/// Spawn a background heartbeat logging task
///
/// Creates an async task that logs query statistics every HEARTBEAT_INTERVAL_SECS (120s)
/// until QueryStats::stop() is called. Provides periodic progress visibility for
/// long-running operations.
///
/// # Arguments
/// * `stats` - Arc-wrapped QueryStats to monitor
/// * `album_idx` - Album index for log message prefix (zero-based)
///
/// # Returns
/// JoinHandle for the spawned task (can be used to await completion)
///
/// # Lifecycle
/// 1. Spawns background tokio task
/// 2. Sleeps for HEARTBEAT_INTERVAL_SECS (120s)
/// 3. Checks stop_flag via stats.is_stopped()
/// 4. If not stopped, calls stats.log_heartbeat()
/// 5. Repeats until stopped
///
/// # Example
/// ```ignore
/// let stats = Arc::new(QueryStats::new());
/// let heartbeat_handle = spawn_heartbeat_task(stats.clone(), album_idx);
///
/// // ... perform work ...
///
/// stats.stop();  // Signal task to exit
/// heartbeat_handle.await?;  // Wait for graceful shutdown
/// ```
pub(crate) fn spawn_heartbeat_task(stats: Arc<QueryStats>, album_idx: usize) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let interval = Duration::from_secs(HEARTBEAT_INTERVAL_SECS);
        loop {
            sleep(interval).await;
            if stats.is_stopped() {
                break;
            }
            stats.log_heartbeat(album_idx);
        }
    })
}
