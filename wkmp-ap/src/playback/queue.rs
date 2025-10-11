//! Queue manager with database persistence

use anyhow::{anyhow, Result};
use sqlx::SqlitePool;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;
use wkmp_common::db::QueueEntry;

use crate::api::ResolvedTiming;

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

    /// Resolve timing parameters using precedence order:
    /// 1. Explicit timing override fields (highest priority)
    /// 2. Passage defaults from passage_guid
    /// 3. System defaults (fallback)
    pub async fn resolve_timing(
        &self,
        file_path: &str,
        passage_guid: Option<&str>,
        explicit_start_time_ms: Option<i64>,
        explicit_end_time_ms: Option<i64>,
        explicit_lead_in_point_ms: Option<i64>,
        explicit_lead_out_point_ms: Option<i64>,
        explicit_fade_in_point_ms: Option<i64>,
        explicit_fade_out_point_ms: Option<i64>,
        explicit_fade_in_curve: Option<&str>,
        explicit_fade_out_curve: Option<&str>,
    ) -> Result<ResolvedTiming> {
        // Try to get passage defaults if passage_guid provided
        let passage_defaults = if let Some(guid) = passage_guid {
            self.load_passage_defaults(guid).await.ok()
        } else {
            None
        };

        // Get file duration for system defaults
        let file_duration_ms = self.get_file_duration(file_path).await?;

        // Apply precedence order for each field
        let start_time_ms = explicit_start_time_ms
            .or(passage_defaults.as_ref().and_then(|p| p.start_time_ms))
            .unwrap_or(0);

        let end_time_ms = explicit_end_time_ms
            .or(passage_defaults.as_ref().and_then(|p| p.end_time_ms))
            .unwrap_or(file_duration_ms);

        let lead_in_point_ms = explicit_lead_in_point_ms
            .or(passage_defaults.as_ref().and_then(|p| p.lead_in_point_ms))
            .unwrap_or(start_time_ms);

        let lead_out_point_ms = explicit_lead_out_point_ms
            .or(passage_defaults.as_ref().and_then(|p| p.lead_out_point_ms))
            .unwrap_or(end_time_ms);

        let fade_in_point_ms = explicit_fade_in_point_ms
            .or(passage_defaults.as_ref().and_then(|p| p.fade_in_point_ms))
            .unwrap_or(start_time_ms);

        let fade_out_point_ms = explicit_fade_out_point_ms
            .or(passage_defaults.as_ref().and_then(|p| p.fade_out_point_ms))
            .unwrap_or(end_time_ms);

        let fade_in_curve = explicit_fade_in_curve
            .map(String::from)
            .or(passage_defaults.as_ref().and_then(|p| p.fade_in_curve.clone()))
            .unwrap_or_else(|| "exponential".to_string());

        let fade_out_curve = explicit_fade_out_curve
            .map(String::from)
            .or(passage_defaults.as_ref().and_then(|p| p.fade_out_curve.clone()))
            .unwrap_or_else(|| "logarithmic".to_string());

        Ok(ResolvedTiming {
            start_time_ms,
            end_time_ms,
            lead_in_point_ms,
            lead_out_point_ms,
            fade_in_point_ms,
            fade_out_point_ms,
            fade_in_curve,
            fade_out_curve,
        })
    }

    /// Load passage defaults from database (if exists in passages table)
    async fn load_passage_defaults(&self, _passage_guid: &str) -> Result<QueueEntry> {
        // TODO: Query passages table when it's implemented
        // For now, return not found
        Err(anyhow!("Passages table not yet implemented"))
    }

    /// Get file duration in milliseconds
    async fn get_file_duration(&self, file_path: &str) -> Result<i64> {
        // Try to get from files table first
        let result = sqlx::query_as::<_, (Option<f64>,)>(
            "SELECT duration FROM files WHERE path = ?"
        )
        .bind(file_path)
        .fetch_optional(&self.db)
        .await?;

        if let Some((Some(duration),)) = result {
            return Ok((duration * 1000.0) as i64);
        }

        // If not in database, try to probe the file with GStreamer
        let full_path = self.root_folder.join(file_path);
        match probe_file_duration(&full_path) {
            Ok(duration_ms) => {
                // Cache in database for future use
                let duration_s = duration_ms as f64 / 1000.0;
                let guid = Uuid::new_v4().to_string();
                let _ = sqlx::query(
                    "INSERT OR IGNORE INTO files (guid, path, hash, duration, modification_time) VALUES (?, ?, '', ?, datetime('now'))"
                )
                .bind(&guid)
                .bind(file_path)
                .bind(duration_s)
                .execute(&self.db)
                .await;

                Ok(duration_ms)
            }
            Err(e) => {
                warn!("Failed to probe file duration for {}: {}, using 180000ms default", file_path, e);
                // Default to 3 minutes
                Ok(180000)
            }
        }
    }
}

/// Probe file duration using GStreamer
fn probe_file_duration(file_path: &PathBuf) -> Result<i64> {
    use gstreamer::prelude::*;

    // Create a simple pipeline to discover the file
    let uri = format!("file://{}", file_path.display());
    let pipeline = gstreamer::parse::launch(&format!("uridecodebin uri={}", uri))?;

    // Set to paused to pre-roll and get duration
    pipeline.set_state(gstreamer::State::Paused)?;

    // Wait for state change
    let _ = pipeline.state(gstreamer::ClockTime::from_seconds(5));

    // Query duration
    let duration = pipeline
        .query_duration::<gstreamer::ClockTime>()
        .ok_or_else(|| anyhow!("Could not query duration"))?;

    // Clean up
    pipeline.set_state(gstreamer::State::Null)?;

    Ok(duration.mseconds() as i64)
}
