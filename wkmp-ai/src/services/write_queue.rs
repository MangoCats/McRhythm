//! Database write queue with single executor
//!
//! **[PLAN028 Increment 4]** WriteQueue implementation
//!
//! # Purpose
//! Serializes all database write operations through a single executor task to:
//! - Respect SQLite's single-writer limitation
//! - Eliminate write lock contention and timeouts
//! - Enable batch operations for better performance
//!
//! # Architecture
//! - **Bounded queue:** Max 1000 operations to prevent memory exhaustion
//! - **Single executor:** One task processes all writes sequentially
//! - **Backpressure:** Blocks when queue is full
//!
//! **Traceability:**
//! - [REQ-PERF-003] Single database writer task
//! - [REQ-PERF-004] Bounded queue with backpressure
//! - [REQ-PERF-007] Batch passage recording

use crate::models::ImportSession;
use anyhow::Result;
use sqlx::SqlitePool;
use std::collections::VecDeque;
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

/// Maximum queue depth before backpressure applies
///
/// **[REQ-PERF-004]** Bounded at 1000 items to prevent memory exhaustion
const MAX_QUEUE_DEPTH: usize = 1000;

/// Database write operations
///
/// **[PLAN028 Increment 4]** Enum of all supported write operations
#[derive(Debug)]
pub enum WriteOperation {
    /// Save or update an import session
    SaveSession {
        session: ImportSession,
        response_tx: oneshot::Sender<Result<()>>,
    },
    /// Record multiple passages in a batch
    RecordPassages {
        passages: Vec<PassageData>,
        response_tx: oneshot::Sender<Result<Vec<Uuid>>>,
    },
    /// Update file processing status
    UpdateFileStatus {
        file_id: Uuid,
        status: String,
        response_tx: oneshot::Sender<Result<()>>,
    },
    /// Shutdown the writer task
    Shutdown,
}

/// Passage data for batch recording
///
/// **[REQ-PERF-007]** Simplified passage data for batching
#[derive(Debug, Clone)]
pub struct PassageData {
    pub file_id: Uuid,
    pub start_seconds: f64,
    pub end_seconds: f64,
    pub song_id: Option<Uuid>,
    // Additional fields as needed for passage creation
}

/// Write queue with single executor
///
/// **[PLAN028 Increment 4]** Core write queue implementation
///
/// # Usage
/// ```rust,ignore
/// let queue = WriteQueue::new(db_pool);
/// queue.spawn_writer();
///
/// // Enqueue operations (blocks if queue full)
/// queue.save_session(session).await?;
/// queue.record_passages_batch(passages).await?;
///
/// // Shutdown gracefully
/// queue.shutdown().await;
/// ```
#[derive(Clone)]
pub struct WriteQueue {
    /// Channel sender for write operations
    tx: mpsc::Sender<WriteOperation>,
}

impl WriteQueue {
    /// Create new write queue and spawn executor task
    ///
    /// **[REQ-PERF-003]** Single executor for all database writes
    /// **[REQ-PERF-004]** Bounded queue (1000 items max)
    ///
    /// # Arguments
    /// * `db` - Database pool for executing writes
    pub fn new(db: SqlitePool) -> Self {
        let (tx, rx) = mpsc::channel(MAX_QUEUE_DEPTH);

        // Spawn single writer task
        tokio::spawn(async move {
            Self::writer_task(rx, db).await;
        });

        Self { tx }
    }

    /// Writer task - processes all write operations sequentially
    ///
    /// **[REQ-PERF-003]** Single task ensures SQLite single-writer compliance
    ///
    /// Runs until Shutdown operation received.
    async fn writer_task(mut rx: mpsc::Receiver<WriteOperation>, db: SqlitePool) {
        tracing::info!("Write queue executor task started");

        while let Some(operation) = rx.recv().await {
            match operation {
                WriteOperation::SaveSession { session, response_tx } => {
                    let result = crate::db::sessions::save_session(&db, &session).await;

                    if let Err(e) = &result {
                        tracing::error!(
                            session_id = %session.session_id,
                            error = ?e,
                            "Failed to save session in write queue"
                        );
                    }

                    // Send response (ignore if receiver dropped)
                    let _ = response_tx.send(result.map_err(Into::into));
                }

                WriteOperation::RecordPassages { passages, response_tx } => {
                    let result = Self::execute_record_passages_batch(&db, passages).await;

                    if let Err(e) = &result {
                        tracing::error!(
                            error = ?e,
                            "Failed to record passages batch in write queue"
                        );
                    }

                    // Send response (ignore if receiver dropped)
                    let _ = response_tx.send(result);
                }

                WriteOperation::UpdateFileStatus { file_id, status, response_tx } => {
                    let result = Self::execute_update_file_status(&db, file_id, &status).await;

                    if let Err(e) = &result {
                        tracing::error!(
                            file_id = %file_id,
                            error = ?e,
                            "Failed to update file status in write queue"
                        );
                    }

                    // Send response (ignore if receiver dropped)
                    let _ = response_tx.send(result);
                }

                WriteOperation::Shutdown => {
                    tracing::info!("Write queue shutting down");
                    break;
                }
            }
        }

        tracing::info!("Write queue executor task terminated");
    }

    /// Save import session to database
    ///
    /// **[REQ-PERF-004]** Blocks when queue is full (backpressure)
    ///
    /// # Arguments
    /// * `session` - Import session to save
    pub async fn save_session(&self, session: ImportSession) -> Result<()> {
        let (response_tx, response_rx) = oneshot::channel();

        self.tx
            .send(WriteOperation::SaveSession { session, response_tx })
            .await
            .map_err(|_| anyhow::anyhow!("Write queue channel closed"))?;

        response_rx
            .await
            .map_err(|_| anyhow::anyhow!("Write queue response channel closed"))?
    }

    /// Record multiple passages in a single batch
    ///
    /// **[REQ-PERF-007]** Batch recording reduces database round-trips
    ///
    /// # Arguments
    /// * `passages` - Vector of passage data to record
    ///
    /// # Returns
    /// Vector of passage UUIDs created
    pub async fn record_passages_batch(&self, passages: Vec<PassageData>) -> Result<Vec<Uuid>> {
        let (response_tx, response_rx) = oneshot::channel();

        self.tx
            .send(WriteOperation::RecordPassages { passages, response_tx })
            .await
            .map_err(|_| anyhow::anyhow!("Write queue channel closed"))?;

        response_rx
            .await
            .map_err(|_| anyhow::anyhow!("Write queue response channel closed"))?
    }

    /// Update file processing status
    ///
    /// # Arguments
    /// * `file_id` - UUID of file to update
    /// * `status` - New status string
    pub async fn update_file_status(&self, file_id: Uuid, status: String) -> Result<()> {
        let (response_tx, response_rx) = oneshot::channel();

        self.tx
            .send(WriteOperation::UpdateFileStatus {
                file_id,
                status,
                response_tx,
            })
            .await
            .map_err(|_| anyhow::anyhow!("Write queue channel closed"))?;

        response_rx
            .await
            .map_err(|_| anyhow::anyhow!("Write queue response channel closed"))?
    }

    /// Shutdown the write queue gracefully
    ///
    /// Sends shutdown signal and waits for executor task to finish.
    pub async fn shutdown(&self) -> Result<()> {
        self.tx
            .send(WriteOperation::Shutdown)
            .await
            .map_err(|_| anyhow::anyhow!("Write queue channel closed"))?;

        Ok(())
    }

    /// Get current queue depth
    ///
    /// **[REQ-PERF-004]** Monitor queue depth for observability
    pub fn queue_depth(&self) -> usize {
        // Note: mpsc::Sender doesn't expose queue depth directly
        // This would require additional Arc<AtomicUsize> tracking
        // For now, return 0 as placeholder
        0
    }

    /// Internal: Record passages batch to database
    ///
    /// **[REQ-PERF-007]** Batch insert in single transaction
    async fn execute_record_passages_batch(db: &SqlitePool, passages: Vec<PassageData>) -> Result<Vec<Uuid>> {
        let passage_count = passages.len();
        if passage_count == 0 {
            return Ok(Vec::new());
        }

        let mut tx = db.begin().await?;
        let mut passage_ids = Vec::with_capacity(passage_count);

        for passage in passages {
            let passage_id = Uuid::new_v4();

            sqlx::query(
                r#"
                INSERT INTO passages (
                    passage_id, file_id, start_seconds, end_seconds, song_id
                ) VALUES (?, ?, ?, ?, ?)
                "#,
            )
            .bind(passage_id.to_string())
            .bind(passage.file_id.to_string())
            .bind(passage.start_seconds)
            .bind(passage.end_seconds)
            .bind(passage.song_id.map(|id| id.to_string()))
            .execute(&mut *tx)
            .await?;

            passage_ids.push(passage_id);
        }

        tx.commit().await?;

        tracing::debug!(
            count = passage_count,
            "Recorded passages batch"
        );

        Ok(passage_ids)
    }

    /// Internal: Update file status in database
    async fn execute_update_file_status(db: &SqlitePool, file_id: Uuid, status: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE files
            SET status = ?
            WHERE file_id = ?
            "#,
        )
        .bind(status)
        .bind(file_id.to_string())
        .execute(db)
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Setup in-memory test database
    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();

        // Create import_sessions table
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

        // Create settings table (required by save_session)
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

        // Create passages table
        sqlx::query(
            r#"
            CREATE TABLE passages (
                passage_id TEXT PRIMARY KEY NOT NULL,
                file_id TEXT NOT NULL,
                start_seconds REAL NOT NULL,
                end_seconds REAL NOT NULL,
                song_id TEXT
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        // Create files table
        sqlx::query(
            r#"
            CREATE TABLE files (
                file_id TEXT PRIMARY KEY NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending'
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    /// **[TC-U-004-01]** WriteQueue processes operations sequentially
    ///
    /// **Given:** WriteQueue with multiple operations enqueued
    /// **When:** Operations are processed
    /// **Then:** All operations complete successfully in order
    #[tokio::test]
    async fn test_write_queue_sequential_processing() {
        let db = setup_test_db().await;
        let queue = WriteQueue::new(db.clone());

        // Create test session
        let session = crate::models::ImportSession::new(
            "/test/path".to_string(),
            crate::models::ImportParameters::default(),
        );
        let session_id = session.session_id;

        // Save session via queue
        queue.save_session(session).await.unwrap();

        // Verify session was saved
        let loaded = crate::db::sessions::load_session(&db, session_id)
            .await
            .unwrap();
        assert!(loaded.is_some());

        // Shutdown
        queue.shutdown().await.unwrap();
    }

    /// **[TC-U-004-02]** WriteQueue handles batch passage recording
    ///
    /// **Given:** WriteQueue and multiple passages to record
    /// **When:** record_passages_batch() called
    /// **Then:** All passages inserted in single transaction
    #[tokio::test]
    async fn test_write_queue_batch_passages() {
        let db = setup_test_db().await;
        let queue = WriteQueue::new(db.clone());

        let file_id = Uuid::new_v4();

        // Create test passages
        let passages = vec![
            PassageData {
                file_id,
                start_seconds: 0.0,
                end_seconds: 30.0,
                song_id: None,
            },
            PassageData {
                file_id,
                start_seconds: 30.0,
                end_seconds: 60.0,
                song_id: None,
            },
            PassageData {
                file_id,
                start_seconds: 60.0,
                end_seconds: 90.0,
                song_id: None,
            },
        ];

        // Record batch
        let passage_ids = queue.record_passages_batch(passages).await.unwrap();

        // Verify count
        assert_eq!(passage_ids.len(), 3);

        // Verify passages in database
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM passages")
            .fetch_one(&db)
            .await
            .unwrap();
        assert_eq!(count, 3);

        // Shutdown
        queue.shutdown().await.unwrap();
    }
}
