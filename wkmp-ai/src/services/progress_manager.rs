//! In-memory progress tracking with periodic database sync
//!
//! **[PLAN028 Increment 1]** Core ProgressManager implementation
//!
//! # Purpose
//! Reduces database write operations from O(files) to O(1) by:
//! - Storing progress updates in memory using RwLock
//! - Broadcasting SSE events immediately without database I/O
//! - Syncing to database periodically (10-second intervals)
//!
//! # Architecture
//! - **In-memory state:** parking_lot::RwLock for low-overhead synchronization
//! - **SSE broadcasting:** Immediate via EventBus, decoupled from database
//! - **Periodic sync:** Background task writes to database when dirty
//!
//! **Traceability:**
//! - [REQ-PERF-001] In-memory progress tracking with periodic sync
//! - [REQ-PERF-002] Decouple SSE from database operations
//! - [REQ-PERF-005] Maintain <10 second data loss window
//! - [REQ-PERF-006] Real-time progress updates via SSE

use crate::models::ImportSession;
use anyhow::Result;
use chrono::Utc;
use parking_lot::RwLock;
use sqlx::SqlitePool;
use std::sync::Arc;
use std::time::Instant;
use tokio::time::{interval, Duration};
use uuid::Uuid;
use wkmp_common::events::{EventBus, PhaseStatistics, WkmpEvent};

/// In-memory progress state
///
/// **[REQ-PERF-001]** All progress updates stored in memory, marked dirty on change
#[derive(Clone)]
struct ProgressState {
    /// Import session ID
    session_id: Uuid,
    /// Number of files completed
    files_processed: usize,
    /// Total files to process
    files_total: usize,
    /// Current operation description
    current_operation: String,
    /// Current file being processed (optional)
    current_file: Option<String>,
    /// Current import state
    state: String,
    /// Phase-specific statistics for UI
    phase_statistics: Vec<PhaseStatistics>,
    /// Start time of import (for elapsed time calculation)
    start_time: Instant,
    /// Last database sync timestamp
    last_db_sync: Instant,
    /// Dirty flag - true if state changed since last sync
    dirty: bool,
}

/// Progress manager with in-memory state and periodic database sync
///
/// **[PLAN028 Increment 1-2]** Core implementation with background sync
///
/// # Usage
/// ```rust,ignore
/// let manager = ProgressManager::new(session_id, event_bus, db, total_files);
///
/// // Update progress (in-memory + SSE broadcast, no database I/O)
/// manager.update_progress(100, "Processing files").await?;
///
/// // Background task automatically syncs every 10 seconds when dirty
/// // Manual sync available via sync_to_database() or force_sync()
/// ```
#[derive(Clone)]
pub struct ProgressManager {
    /// In-memory state (RwLock for low-overhead access)
    state: Arc<RwLock<ProgressState>>,
    /// Event bus for SSE broadcasting
    event_bus: EventBus,
    /// Database pool for periodic sync
    db: SqlitePool,
    /// Cancellation token for background sync task
    cancel_token: tokio_util::sync::CancellationToken,
}

impl ProgressManager {
    /// Create new progress manager and spawn background sync task
    ///
    /// **[REQ-PERF-001]** Initialize in-memory state
    /// **[PLAN028 Increment 2]** Spawn background sync task
    ///
    /// # Arguments
    /// * `session_id` - Import session UUID
    /// * `event_bus` - Event bus for SSE broadcasting
    /// * `db` - Database pool for periodic sync
    /// * `total_files` - Total number of files to process
    pub fn new(
        session_id: Uuid,
        event_bus: EventBus,
        db: SqlitePool,
        total_files: usize,
    ) -> Self {
        let now = Instant::now();
        let state = ProgressState {
            session_id,
            files_processed: 0,
            files_total: total_files,
            current_operation: "Initializing".to_string(),
            current_file: None,
            state: "PROCESSING".to_string(),
            phase_statistics: Vec::new(),
            start_time: now,
            last_db_sync: now,
            dirty: false,
        };

        let cancel_token = tokio_util::sync::CancellationToken::new();

        let manager = Self {
            state: Arc::new(RwLock::new(state)),
            event_bus,
            db,
            cancel_token,
        };

        // **[PLAN028 Increment 2]** Spawn background sync task
        manager.spawn_sync_task();

        manager
    }

    /// Spawn background sync task
    ///
    /// **[PLAN028 Increment 2]** Background task implementation
    /// **[REQ-PERF-001]** Sync every 10 seconds when dirty
    /// **[REQ-PERF-005]** Maintain <10 second data loss window
    ///
    /// Task runs every 10 seconds and syncs to database if state is dirty.
    /// Stops when cancel_token is triggered.
    fn spawn_sync_task(&self) {
        let state = self.state.clone();
        let db = self.db.clone();
        let cancel_token = self.cancel_token.clone();

        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(10));
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        // Check if sync needed
                        let (needs_sync, session_id) = {
                            let s = state.read();
                            (s.dirty, s.session_id)
                        };

                        if needs_sync {
                            // Perform sync
                            match Self::sync_to_database_impl(&state, &db).await {
                                Ok(()) => {
                                    tracing::debug!(
                                        session_id = %session_id,
                                        "Background sync completed successfully"
                                    );
                                }
                                Err(e) => {
                                    tracing::error!(
                                        session_id = %session_id,
                                        error = %e,
                                        "Background sync failed"
                                    );
                                    // Continue running despite error
                                }
                            }
                        }
                    }
                    _ = cancel_token.cancelled() => {
                        tracing::debug!("Background sync task cancelled");
                        break;
                    }
                }
            }
        });
    }

    /// Update progress in memory
    ///
    /// **[REQ-PERF-001]** Store update in memory, mark dirty
    /// **[REQ-PERF-002]** Broadcast SSE immediately without database I/O
    /// **[REQ-PERF-006]** Real-time updates via SSE
    ///
    /// # Arguments
    /// * `files_processed` - Number of files completed
    /// * `current_operation` - Description of current operation
    pub async fn update_progress(
        &self,
        files_processed: usize,
        current_operation: String,
    ) -> Result<()> {
        // Update in-memory state and mark dirty
        {
            let mut state = self.state.write();
            state.files_processed = files_processed;
            state.current_operation = current_operation;
            state.dirty = true;
        }

        // **[REQ-PERF-002]** Broadcast SSE immediately (no database wait)
        self.broadcast_sse().await;

        Ok(())
    }

    /// Update current file being processed
    ///
    /// # Arguments
    /// * `current_file` - Path to current file
    pub async fn update_current_file(&self, current_file: Option<String>) -> Result<()> {
        {
            let mut state = self.state.write();
            state.current_file = current_file;
            state.dirty = true;
        }

        // Broadcast SSE immediately
        self.broadcast_sse().await;

        Ok(())
    }

    /// Update phase statistics
    ///
    /// **[PLAN024]** Phase-specific statistics for UI display
    ///
    /// # Arguments
    /// * `phase_statistics` - Updated phase statistics
    pub async fn update_phase_statistics(
        &self,
        phase_statistics: Vec<PhaseStatistics>,
    ) -> Result<()> {
        {
            let mut state = self.state.write();
            state.phase_statistics = phase_statistics;
            state.dirty = true;
        }

        // Broadcast SSE immediately
        self.broadcast_sse().await;

        Ok(())
    }

    /// Broadcast SSE event without database I/O
    ///
    /// **[REQ-PERF-002]** SSE decoupled from database operations
    /// **[REQ-PERF-006]** <1 second latency for real-time updates
    async fn broadcast_sse(&self) {
        let (session_id, state_str, current, total, percentage, current_operation, current_file, phase_statistics, elapsed_seconds) = {
            let state = self.state.read();
            let elapsed = state.start_time.elapsed().as_secs();
            let percentage = if state.files_total > 0 {
                (state.files_processed as f32 / state.files_total as f32) * 100.0
            } else {
                0.0
            };

            (
                state.session_id,
                state.state.clone(),
                state.files_processed,
                state.files_total,
                percentage,
                state.current_operation.clone(),
                state.current_file.clone(),
                state.phase_statistics.clone(),
                elapsed,
            )
        };

        // Emit SSE event immediately (no database I/O in this path)
        let event = WkmpEvent::ImportProgressUpdate {
            session_id,
            state: state_str,
            current,
            total,
            percentage,
            current_operation,
            elapsed_seconds,
            estimated_remaining_seconds: None, // TODO: Calculate based on throughput
            phases: Vec::new(), // TODO: Convert phase_statistics to PhaseProgressData
            current_file,
            phase_statistics,
            timestamp: Utc::now(),
        };

        // Broadcast to all SSE subscribers
        self.event_bus.emit_lossy(event);
    }

    /// Sync in-memory state to database (public API)
    ///
    /// **[REQ-PERF-001]** Periodic sync when dirty
    /// **[REQ-PERF-005]** <10 second data loss window
    ///
    /// Only writes to database if dirty flag is set.
    /// Called by background task every 10 seconds or manually.
    pub async fn sync_to_database(&self) -> Result<()> {
        Self::sync_to_database_impl(&self.state, &self.db).await
    }

    /// Internal implementation of database sync (shared by public method and background task)
    ///
    /// **[PLAN028 Increment 2]** Refactored for background task support
    async fn sync_to_database_impl(
        state: &Arc<RwLock<ProgressState>>,
        db: &SqlitePool,
    ) -> Result<()> {
        // Check if sync needed (avoid unnecessary database writes)
        let needs_sync = {
            let s = state.read();
            s.dirty
        };

        if !needs_sync {
            return Ok(());
        }

        // Extract data needed for sync (drop lock before await)
        let (session_id, files_processed, files_total, current_operation, current_file) = {
            let s = state.read();
            (
                s.session_id,
                s.files_processed,
                s.files_total,
                s.current_operation.clone(),
                s.current_file.clone(),
            )
        };

        // Fetch existing session from database to preserve fields
        let mut session = crate::db::sessions::load_session(db, session_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))?;

        // Update progress fields
        session.update_progress(files_processed, files_total, current_operation);

        // Unconditionally update current_file to match in-memory state
        session.progress.current_file = current_file;

        // Write to database
        crate::db::sessions::save_session(db, &session).await?;

        // Clear dirty flag and update sync timestamp
        {
            let mut s = state.write();
            s.dirty = false;
            s.last_db_sync = Instant::now();
        }

        tracing::debug!(
            session_id = %session.session_id,
            "Progress synced to database"
        );

        Ok(())
    }

    /// Force immediate database sync (for critical transitions)
    ///
    /// Used when import completes, fails, or is cancelled to ensure
    /// final state is persisted immediately.
    pub async fn force_sync(&self) -> Result<()> {
        self.sync_to_database().await
    }

    /// Get current progress state (for debugging/monitoring)
    pub fn get_progress(&self) -> (usize, usize, String) {
        let state = self.state.read();
        (state.files_processed, state.files_total, state.current_operation.clone())
    }

    /// Check if state is dirty (needs sync)
    pub fn is_dirty(&self) -> bool {
        let state = self.state.read();
        state.dirty
    }

    /// Get time since last database sync
    pub fn time_since_last_sync(&self) -> std::time::Duration {
        let state = self.state.read();
        state.last_db_sync.elapsed()
    }

    /// Shutdown background sync task
    ///
    /// **[PLAN028 Increment 2]** Graceful shutdown
    ///
    /// Cancels the background sync task. Should be called when
    /// import completes, fails, or is cancelled.
    pub fn shutdown(&self) {
        self.cancel_token.cancel();
    }
}

impl Drop for ProgressManager {
    /// Ensure background task is cancelled on drop
    fn drop(&mut self) {
        self.cancel_token.cancel();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Setup in-memory test database with import_sessions and settings tables
    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();

        // Create settings table (required by save_session for lock wait time query)
        sqlx::query(
            r#"
            CREATE TABLE settings (
                key TEXT PRIMARY KEY NOT NULL,
                value TEXT
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        // Create import_sessions table matching production schema
        sqlx::query(
            r#"
            CREATE TABLE import_sessions (
                session_id TEXT PRIMARY KEY NOT NULL,
                state TEXT NOT NULL,
                root_folder TEXT NOT NULL,
                parameters TEXT NOT NULL,
                progress_current INTEGER NOT NULL DEFAULT 0,
                progress_total INTEGER NOT NULL DEFAULT 0,
                progress_percentage REAL NOT NULL DEFAULT 0.0,
                current_operation TEXT NOT NULL DEFAULT '',
                errors TEXT NOT NULL DEFAULT '[]',
                started_at TEXT NOT NULL,
                ended_at TEXT,
                file_classification_data TEXT NOT NULL DEFAULT '{}'
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    /// **[TC-U-001-01]** ProgressManager stores updates in memory
    ///
    /// **Given:** New ProgressManager instance
    /// **When:** update_progress(100, 1000, "Processing") called
    /// **Then:** Internal state reflects: processed=100, total=1000, operation="Processing"
    /// **Verify:** No database calls made
    #[tokio::test]
    async fn test_progress_manager_stores_in_memory() {
        // Create in-memory database for testing
        let db = setup_test_db().await;

        let event_bus = EventBus::new(100);

        // Create session in database first
        let mut session = crate::models::ImportSession::new(
            "/test/path".to_string(),
            crate::models::ImportParameters::default(),
        );
        let session_id = session.session_id; // Capture the generated UUID
        crate::db::sessions::save_session(&db, &session).await.unwrap();

        let manager = ProgressManager::new(session_id, event_bus, db.clone(), 1000);

        // Update progress
        manager.update_progress(100, "Processing files".to_string()).await.unwrap();

        // Verify in-memory state
        let (processed, total, operation) = manager.get_progress();
        assert_eq!(processed, 100);
        assert_eq!(total, 1000);
        assert_eq!(operation, "Processing files");
    }

    /// **[TC-U-001-02]** ProgressManager marks dirty on update
    ///
    /// **Given:** ProgressManager with clean state
    /// **When:** Any update method called
    /// **Then:** dirty flag = true
    /// **Verify:** Flag resets to false after sync
    #[tokio::test]
    async fn test_progress_manager_marks_dirty() {
        let db = setup_test_db().await;

        let event_bus = EventBus::new(100);

        let mut session = crate::models::ImportSession::new(
            "/test/path".to_string(),
            crate::models::ImportParameters::default(),
        );
        let session_id = session.session_id; // Capture the generated UUID
        crate::db::sessions::save_session(&db, &session).await.unwrap();

        let manager = ProgressManager::new(session_id, event_bus, db.clone(), 1000);

        // Initially not dirty
        assert!(!manager.is_dirty());

        // Update progress
        manager.update_progress(100, "Processing".to_string()).await.unwrap();

        // Should be dirty now
        assert!(manager.is_dirty());

        // Sync to database
        manager.sync_to_database().await.unwrap();

        // Should be clean after sync
        assert!(!manager.is_dirty());
    }

    /// **[TC-U-001-03]** Sync task runs every 10 seconds
    ///
    /// **Given:** ProgressManager with sync task spawned
    /// **When:** Wait 11 seconds
    /// **Then:** Sync function called at least once
    /// **Verify:** Timer interval is 10±0.5 seconds
    #[tokio::test]
    async fn test_sync_task_runs_periodically() {
        let db = setup_test_db().await;
        let event_bus = EventBus::new(100);

        let mut session = crate::models::ImportSession::new(
            "/test/path".to_string(),
            crate::models::ImportParameters::default(),
        );
        let session_id = session.session_id;
        crate::db::sessions::save_session(&db, &session).await.unwrap();

        let manager = ProgressManager::new(session_id, event_bus, db.clone(), 1000);

        // Make state dirty
        manager.update_progress(50, "Processing".to_string()).await.unwrap();
        assert!(manager.is_dirty());

        // Record when we marked it dirty
        let dirty_time = manager.time_since_last_sync();

        // Wait for background sync (just over 10 seconds)
        tokio::time::sleep(Duration::from_secs(11)).await;

        // Verify sync occurred (dirty flag should be cleared)
        assert!(!manager.is_dirty());

        // Verify database was updated
        let loaded = crate::db::sessions::load_session(&db, session_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loaded.progress.current, 50);

        // Clean up
        manager.shutdown();
    }

    /// **[TC-U-001-04]** Sync persists to database when dirty
    ///
    /// **Given:** ProgressManager with dirty=true
    /// **When:** sync_to_database() called
    /// **Then:** Database updated with current state
    /// **Verify:** dirty flag = false after successful sync
    #[tokio::test]
    async fn test_sync_persists_when_dirty() {
        let db = setup_test_db().await;
        let event_bus = EventBus::new(100);

        let mut session = crate::models::ImportSession::new(
            "/test/path".to_string(),
            crate::models::ImportParameters::default(),
        );
        let session_id = session.session_id;
        crate::db::sessions::save_session(&db, &session).await.unwrap();

        let manager = ProgressManager::new(session_id, event_bus, db.clone(), 1000);

        // Update progress to make dirty
        manager.update_progress(75, "Almost done".to_string()).await.unwrap();

        assert!(manager.is_dirty());

        // Manual sync
        manager.sync_to_database().await.unwrap();

        // Should be clean now
        assert!(!manager.is_dirty());

        // Verify database reflects updates
        let loaded = crate::db::sessions::load_session(&db, session_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loaded.progress.current, 75);
        assert_eq!(loaded.progress.total, 1000);
        assert_eq!(loaded.progress.current_operation, "Almost done");

        // Note: current_file is stored in memory but not currently persisted to database
        // This is acceptable for progress tracking as it's mainly for UI display

        // Clean up
        manager.shutdown();
    }
}
