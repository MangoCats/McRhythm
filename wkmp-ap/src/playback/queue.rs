//! Queue manager with database persistence

use anyhow::Result;
use sqlx::SqlitePool;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info};
use uuid::Uuid;
use wkmp_common::db::QueueEntry;

/// Queue manager handles queue persistence and operations
#[derive(Clone)]
pub struct QueueManager {
    db: SqlitePool,
    root_folder: PathBuf,
    queue_cache: Arc<RwLock<Vec<QueueEntry>>>,
}

impl QueueManager {
    /// Create a new queue manager
    pub fn new(db: SqlitePool, root_folder: PathBuf) -> Self {
        Self {
            db,
            root_folder,
            queue_cache: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Initialize queue manager by loading queue from database
    pub async fn init(&self) -> Result<()> {
        info!("Initializing queue manager...");
        self.load_queue().await?;
        Ok(())
    }

    /// Load queue from database into memory
    async fn load_queue(&self) -> Result<()> {
        let entries = sqlx::query_as::<_, (
            String,
            String,
            Option<String>,
            i64,
            Option<i64>,
            Option<i64>,
            Option<i64>,
            Option<i64>,
            Option<i64>,
            Option<i64>,
            Option<String>,
            Option<String>,
        )>(
            r#"
            SELECT guid, file_path, passage_guid, play_order,
                   start_time_ms, end_time_ms, lead_in_point_ms, lead_out_point_ms,
                   fade_in_point_ms, fade_out_point_ms, fade_in_curve, fade_out_curve
            FROM queue
            ORDER BY play_order ASC
            "#,
        )
        .fetch_all(&self.db)
        .await?;

        let mut queue = self.queue_cache.write().await;
        queue.clear();

        for entry in entries {
            queue.push(QueueEntry {
                guid: entry.0,
                file_path: entry.1,
                passage_guid: entry.2,
                play_order: entry.3,
                start_time_ms: entry.4,
                end_time_ms: entry.5,
                lead_in_point_ms: entry.6,
                lead_out_point_ms: entry.7,
                fade_in_point_ms: entry.8,
                fade_out_point_ms: entry.9,
                fade_in_curve: entry.10,
                fade_out_curve: entry.11,
            });
        }

        debug!("Loaded {} entries from queue", queue.len());
        Ok(())
    }

    /// Get the next entry in the queue (lowest play_order)
    pub async fn get_next(&self) -> Option<QueueEntry> {
        let queue = self.queue_cache.read().await;
        queue.first().cloned()
    }

    /// Get all queue entries
    pub async fn get_all(&self) -> Vec<QueueEntry> {
        self.queue_cache.read().await.clone()
    }

    /// Remove entry from queue by guid
    pub async fn remove(&self, guid: &str) -> Result<()> {
        // Remove from database
        sqlx::query("DELETE FROM queue WHERE guid = ?")
            .bind(guid)
            .execute(&self.db)
            .await?;

        // Remove from cache
        let mut queue = self.queue_cache.write().await;
        queue.retain(|entry| entry.guid != guid);

        debug!("Removed entry {} from queue", guid);
        Ok(())
    }

    /// Add entry to queue
    pub async fn enqueue(
        &self,
        file_path: String,
        passage_guid: Option<String>,
        start_time_ms: Option<i64>,
        end_time_ms: Option<i64>,
        lead_in_point_ms: Option<i64>,
        lead_out_point_ms: Option<i64>,
        fade_in_point_ms: Option<i64>,
        fade_out_point_ms: Option<i64>,
        fade_in_curve: Option<String>,
        fade_out_curve: Option<String>,
    ) -> Result<String> {
        let guid = Uuid::new_v4().to_string();

        // Determine play_order (append to end)
        let play_order = {
            let queue = self.queue_cache.read().await;
            queue.last().map(|e| e.play_order + 10).unwrap_or(10)
        };

        // Insert into database
        sqlx::query(
            r#"
            INSERT INTO queue (
                guid, file_path, passage_guid, play_order,
                start_time_ms, end_time_ms, lead_in_point_ms, lead_out_point_ms,
                fade_in_point_ms, fade_out_point_ms, fade_in_curve, fade_out_curve
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&guid)
        .bind(&file_path)
        .bind(&passage_guid)
        .bind(play_order)
        .bind(start_time_ms)
        .bind(end_time_ms)
        .bind(lead_in_point_ms)
        .bind(lead_out_point_ms)
        .bind(fade_in_point_ms)
        .bind(fade_out_point_ms)
        .bind(&fade_in_curve)
        .bind(&fade_out_curve)
        .execute(&self.db)
        .await?;

        // Add to cache
        let mut queue = self.queue_cache.write().await;
        queue.push(QueueEntry {
            guid: guid.clone(),
            file_path,
            passage_guid,
            play_order,
            start_time_ms,
            end_time_ms,
            lead_in_point_ms,
            lead_out_point_ms,
            fade_in_point_ms,
            fade_out_point_ms,
            fade_in_curve,
            fade_out_curve,
        });

        debug!("Enqueued entry {} at position {}", guid, play_order);
        Ok(guid)
    }

    /// Get queue size
    pub async fn size(&self) -> usize {
        self.queue_cache.read().await.len()
    }

    /// Check if queue is empty
    pub async fn is_empty(&self) -> bool {
        self.queue_cache.read().await.is_empty()
    }
}
