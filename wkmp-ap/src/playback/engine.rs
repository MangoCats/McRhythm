//! Playback engine orchestration
//!
//! Coordinates queue processing, buffer management, decoding, and audio output.
//!
//! **Traceability:**
//! - [SSD-FLOW-010] Complete playback sequence
//! - [SSD-ENG-020] Queue processing

use crate::db::passages::{create_ephemeral_passage, get_passage_with_timing, PassageWithTiming};
use crate::error::Result;
use crate::playback::buffer_manager::BufferManager;
use crate::playback::decoder_pool::DecoderPool;
use crate::playback::queue_manager::{QueueEntry, QueueManager};
use crate::playback::types::DecodePriority;
use crate::state::{CurrentPassage, PlaybackState, SharedState};
use sqlx::{Pool, Sqlite};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::{debug, error, info};
use uuid::Uuid;

/// Playback position tracking
struct PlaybackPosition {
    /// Current passage UUID (queue entry)
    queue_entry_id: Option<Uuid>,

    /// Current frame position in buffer
    frame_position: usize,

    /// Last position update timestamp
    last_update: Instant,
}

impl PlaybackPosition {
    fn new() -> Self {
        Self {
            queue_entry_id: None,
            frame_position: 0,
            last_update: Instant::now(),
        }
    }
}

/// Playback engine - orchestrates all playback components
///
/// [SSD-FLOW-010] Top-level coordinator for the entire playback pipeline.
pub struct PlaybackEngine {
    /// Database connection pool
    db_pool: Pool<Sqlite>,

    /// Shared state
    state: Arc<SharedState>,

    /// Queue manager (tracks current/next/queued)
    queue: Arc<RwLock<QueueManager>>,

    /// Buffer manager (manages buffer lifecycle)
    buffer_manager: Arc<BufferManager>,

    /// Decoder pool (multi-threaded decoder)
    decoder_pool: Arc<RwLock<Option<DecoderPool>>>,

    /// Current playback position
    position: Arc<RwLock<PlaybackPosition>>,

    /// Playback loop running flag
    running: Arc<RwLock<bool>>,
}

impl PlaybackEngine {
    /// Create new playback engine
    ///
    /// [SSD-FLOW-010] Initialize all components
    pub async fn new(db_pool: Pool<Sqlite>, state: Arc<SharedState>) -> Result<Self> {
        info!("Creating playback engine");

        // Create buffer manager
        let buffer_manager = Arc::new(BufferManager::new());

        // Create decoder pool
        let decoder_pool = DecoderPool::new(Arc::clone(&buffer_manager));

        // Load queue from database
        let queue_manager = QueueManager::load_from_db(&db_pool).await?;
        info!("Loaded queue: {} entries", queue_manager.len());

        Ok(Self {
            db_pool,
            state,
            queue: Arc::new(RwLock::new(queue_manager)),
            buffer_manager,
            decoder_pool: Arc::new(RwLock::new(Some(decoder_pool))),
            position: Arc::new(RwLock::new(PlaybackPosition::new())),
            running: Arc::new(RwLock::new(false)),
        })
    }

    /// Start playback engine background tasks
    ///
    /// [SSD-FLOW-010] Begin processing queue and managing buffers
    pub async fn start(&self) -> Result<()> {
        info!("Starting playback engine");

        // Mark as running
        *self.running.write().await = true;

        // Start playback loop in background
        let self_clone = self.clone_handles();
        tokio::spawn(async move {
            if let Err(e) = self_clone.playback_loop().await {
                error!("Playback loop error: {}", e);
            }
        });

        // Start position tracking loop
        let self_clone = self.clone_handles();
        tokio::spawn(async move {
            self_clone.position_tracking_loop().await;
        });

        info!("Playback engine started");
        Ok(())
    }

    /// Stop playback engine gracefully
    ///
    /// [SSD-DEC-033] Shutdown decoder pool with timeout
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping playback engine");

        // Mark as not running
        *self.running.write().await = false;

        // Shutdown decoder pool
        if let Some(decoder_pool) = self.decoder_pool.write().await.take() {
            decoder_pool.shutdown()?;
        }

        info!("Playback engine stopped");
        Ok(())
    }

    /// Play (resume)
    ///
    /// [API] POST /playback/play
    pub async fn play(&self) -> Result<()> {
        info!("Play command received");
        let old_state = self.state.get_playback_state().await;
        self.state.set_playback_state(PlaybackState::Playing).await;

        // Emit PlaybackStateChanged event
        self.state.broadcast_event(wkmp_common::events::WkmpEvent::PlaybackStateChanged {
            state: wkmp_common::events::PlaybackState::Playing,
            timestamp: chrono::Utc::now(),
        });

        // Also emit PlaybackProgress immediately
        if let Some(passage) = self.state.get_current_passage().await {
            self.state.broadcast_event(wkmp_common::events::WkmpEvent::PlaybackProgress {
                passage_id: passage.passage_id.unwrap_or_else(|| Uuid::nil()),
                position_ms: passage.position_ms,
                duration_ms: passage.duration_ms,
                timestamp: chrono::Utc::now(),
            });
        }

        info!("Playback state changed: {:?} -> Playing", old_state);
        Ok(())
    }

    /// Pause
    ///
    /// [API] POST /playback/pause
    pub async fn pause(&self) -> Result<()> {
        info!("Pause command received");
        let old_state = self.state.get_playback_state().await;
        self.state.set_playback_state(PlaybackState::Paused).await;

        // Emit PlaybackStateChanged event
        self.state.broadcast_event(wkmp_common::events::WkmpEvent::PlaybackStateChanged {
            state: wkmp_common::events::PlaybackState::Paused,
            timestamp: chrono::Utc::now(),
        });

        // Also emit PlaybackProgress immediately
        if let Some(passage) = self.state.get_current_passage().await {
            self.state.broadcast_event(wkmp_common::events::WkmpEvent::PlaybackProgress {
                passage_id: passage.passage_id.unwrap_or_else(|| Uuid::nil()),
                position_ms: passage.position_ms,
                duration_ms: passage.duration_ms,
                timestamp: chrono::Utc::now(),
            });
        }

        info!("Playback state changed: {:?} -> Paused", old_state);
        Ok(())
    }

    /// Skip to next passage
    ///
    /// [API] POST /playback/next
    pub async fn skip_next(&self) -> Result<()> {
        info!("Skip next command received");

        let mut queue = self.queue.write().await;
        queue.advance();

        Ok(())
    }

    /// Enqueue passage for playback
    ///
    /// [API] POST /playback/enqueue
    pub async fn enqueue_file(&self, file_path: PathBuf) -> Result<Uuid> {
        info!("Enqueuing file: {}", file_path.display());

        // Create ephemeral passage
        let passage = create_ephemeral_passage(file_path.clone());

        // Add to database queue
        let queue_entry_id = crate::db::queue::enqueue(
            &self.db_pool,
            file_path.to_string_lossy().to_string(),
            passage.passage_id,
            None, // Append to end
            Some(passage.start_time_ms as i64),
            passage.end_time_ms.map(|v| v as i64),
            Some(passage.lead_in_point_ms as i64),
            passage.lead_out_point_ms.map(|v| v as i64),
            Some(passage.fade_in_point_ms as i64),
            passage.fade_out_point_ms.map(|v| v as i64),
            Some(passage.fade_in_curve.to_str().to_string()),
            Some(passage.fade_out_curve.to_str().to_string()),
        )
        .await?;

        // Add to in-memory queue
        let entry = QueueEntry {
            queue_entry_id,
            passage_id: passage.passage_id,
            file_path,
            play_order: 0, // Will be managed by database
            start_time_ms: Some(passage.start_time_ms),
            end_time_ms: passage.end_time_ms,
            lead_in_point_ms: Some(passage.lead_in_point_ms),
            lead_out_point_ms: passage.lead_out_point_ms,
            fade_in_point_ms: Some(passage.fade_in_point_ms),
            fade_out_point_ms: passage.fade_out_point_ms,
            fade_in_curve: Some(passage.fade_in_curve.to_str().to_string()),
            fade_out_curve: Some(passage.fade_out_curve.to_str().to_string()),
        };

        self.queue.write().await.enqueue(entry);

        Ok(queue_entry_id)
    }

    /// Main playback loop
    ///
    /// [SSD-FLOW-010] Core orchestration logic
    async fn playback_loop(&self) -> Result<()> {
        let mut tick = interval(Duration::from_millis(100)); // Check every 100ms

        loop {
            tick.tick().await;

            // Check if we should continue running
            if !*self.running.read().await {
                debug!("Playback loop stopping");
                break;
            }

            // Check playback state
            let playback_state = self.state.get_playback_state().await;
            if playback_state != PlaybackState::Playing {
                continue; // Paused, skip processing
            }

            // Process queue
            self.process_queue().await?;
        }

        Ok(())
    }

    /// Process queue: trigger decodes for current/next/queued passages
    async fn process_queue(&self) -> Result<()> {
        let queue = self.queue.read().await;

        // Trigger decode for current passage if needed
        if let Some(current) = queue.current() {
            if !self.buffer_manager.is_ready(current.queue_entry_id).await {
                debug!("Requesting decode for current passage: {}", current.queue_entry_id);
                self.request_decode(current, DecodePriority::Immediate, true)
                    .await?;
            }
        }

        // Trigger decode for next passage if needed
        if let Some(next) = queue.next() {
            if !self.buffer_manager.is_ready(next.queue_entry_id).await {
                debug!("Requesting decode for next passage: {}", next.queue_entry_id);
                self.request_decode(next, DecodePriority::Next, true)
                    .await?;
            }
        }

        // Trigger decode for queued passages (first 3)
        for queued in queue.queued().iter().take(3) {
            if !self.buffer_manager.is_ready(queued.queue_entry_id).await {
                debug!("Requesting decode for queued passage: {}", queued.queue_entry_id);
                self.request_decode(queued, DecodePriority::Prefetch, false)
                    .await?;
            }
        }

        Ok(())
    }

    /// Request decode for a passage
    async fn request_decode(
        &self,
        entry: &QueueEntry,
        priority: DecodePriority,
        full_decode: bool,
    ) -> Result<()> {
        // Get passage timing
        let passage = self.get_passage_timing(entry).await?;

        // Submit to decoder pool
        if let Some(decoder_pool) = self.decoder_pool.read().await.as_ref() {
            decoder_pool.submit(
                entry.queue_entry_id,
                passage,
                priority,
                full_decode,
            )?;
        }

        Ok(())
    }

    /// Get passage timing from entry
    async fn get_passage_timing(&self, entry: &QueueEntry) -> Result<PassageWithTiming> {
        // If entry has a passage_id, load from database
        if let Some(passage_id) = entry.passage_id {
            get_passage_with_timing(&self.db_pool, passage_id).await
        } else {
            // Ephemeral passage: create from entry data
            Ok(create_ephemeral_passage(entry.file_path.clone()))
        }
    }

    /// Position tracking loop
    ///
    /// Updates SharedState with current position periodically
    async fn position_tracking_loop(&self) {
        let mut tick = interval(Duration::from_millis(1000)); // Update every second
        let mut progress_counter = 0;

        loop {
            tick.tick().await;

            // Check if we should continue running
            if !*self.running.read().await {
                break;
            }

            // Update current passage in shared state
            let queue = self.queue.read().await;
            let position = self.position.read().await;

            if let Some(current) = queue.current() {
                // Get buffer to calculate duration
                if let Some(buffer_ref) = self.buffer_manager.get_buffer(current.queue_entry_id).await {
                    let buffer = buffer_ref.read().await;

                    let position_ms = (position.frame_position as u64 * 1000) / buffer.sample_rate as u64;
                    let duration_ms = buffer.duration_ms();

                    let current_passage = CurrentPassage {
                        queue_entry_id: current.queue_entry_id,
                        passage_id: current.passage_id,
                        position_ms,
                        duration_ms,
                    };

                    self.state.set_current_passage(Some(current_passage.clone())).await;

                    // Emit PlaybackProgress event every 5 seconds (if playing)
                    progress_counter += 1;
                    if progress_counter >= 5 {
                        progress_counter = 0;
                        let playback_state = self.state.get_playback_state().await;
                        if playback_state == PlaybackState::Playing {
                            self.state.broadcast_event(wkmp_common::events::WkmpEvent::PlaybackProgress {
                                passage_id: current_passage.passage_id.unwrap_or_else(|| Uuid::nil()),
                                position_ms: current_passage.position_ms,
                                duration_ms: current_passage.duration_ms,
                                timestamp: chrono::Utc::now(),
                            });
                        }
                    }
                }
            } else {
                // No current passage
                self.state.set_current_passage(None).await;
                progress_counter = 0;
            }
        }
    }

    /// Clone handles for spawned tasks
    fn clone_handles(&self) -> Self {
        Self {
            db_pool: self.db_pool.clone(),
            state: Arc::clone(&self.state),
            queue: Arc::clone(&self.queue),
            buffer_manager: Arc::clone(&self.buffer_manager),
            decoder_pool: Arc::clone(&self.decoder_pool),
            position: Arc::clone(&self.position),
            running: Arc::clone(&self.running),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn create_test_db() -> Pool<Sqlite> {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();

        // Create minimal schema
        sqlx::query(
            r#"
            CREATE TABLE queue (
                guid TEXT PRIMARY KEY,
                file_path TEXT NOT NULL,
                passage_guid TEXT,
                play_order INTEGER NOT NULL,
                start_time_ms INTEGER,
                end_time_ms INTEGER,
                lead_in_point_ms INTEGER,
                lead_out_point_ms INTEGER,
                fade_in_point_ms INTEGER,
                fade_out_point_ms INTEGER,
                fade_in_curve TEXT,
                fade_out_curve TEXT
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    #[tokio::test]
    async fn test_playback_engine_creation() {
        let db = create_test_db().await;
        let state = Arc::new(SharedState::new());

        let engine = PlaybackEngine::new(db, state).await;
        assert!(engine.is_ok());
    }

    #[tokio::test]
    async fn test_playback_state_control() {
        let db = create_test_db().await;
        let state = Arc::new(SharedState::new());

        let engine = PlaybackEngine::new(db, state.clone()).await.unwrap();

        // Play
        engine.play().await.unwrap();
        assert_eq!(state.get_playback_state().await, PlaybackState::Playing);

        // Pause
        engine.pause().await.unwrap();
        assert_eq!(state.get_playback_state().await, PlaybackState::Paused);
    }
}
