//! Playback engine orchestration
//!
//! Coordinates queue processing, buffer management, decoding, and audio output.
//!
//! **Traceability:**
//! - [SSD-FLOW-010] Complete playback sequence
//! - [SSD-ENG-020] Queue processing

use crate::audio::output::AudioOutput;
use crate::audio::types::AudioFrame;
use crate::db::passages::{create_ephemeral_passage, get_passage_with_timing, PassageWithTiming};
use crate::error::{Error, Result};
use crate::playback::buffer_manager::BufferManager;
use crate::playback::decoder_pool::DecoderPool;
use crate::playback::pipeline::mixer::CrossfadeMixer;
use crate::playback::queue_manager::{QueueEntry, QueueManager};
use crate::playback::types::DecodePriority;
use crate::state::{CurrentPassage, PlaybackState, SharedState};
use sqlx::{Pool, Sqlite};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::{debug, error, info, warn};
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

    /// Crossfade mixer (sample-accurate mixing)
    /// [SSD-MIX-010] Mixer component for audio frame generation
    mixer: Arc<RwLock<CrossfadeMixer>>,

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

        // Create mixer
        // [SSD-MIX-010] Crossfade mixer for sample-accurate mixing
        let mixer = Arc::new(RwLock::new(CrossfadeMixer::new()));

        // Load queue from database
        let queue_manager = QueueManager::load_from_db(&db_pool).await?;
        info!("Loaded queue: {} entries", queue_manager.len());

        Ok(Self {
            db_pool,
            state,
            queue: Arc::new(RwLock::new(queue_manager)),
            buffer_manager,
            decoder_pool: Arc::new(RwLock::new(Some(decoder_pool))),
            mixer,
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

        // Initialize audio output in a background task
        // [SSD-OUT-010] Create audio device interface
        // [SSD-OUT-012] Begin audio stream with callback
        // Note: AudioOutput is not Send/Sync due to cpal::Stream, so we create it in a task
        // that just keeps it alive. The audio stream runs independently once started.
        info!("Initializing audio output");
        let mixer_clone = Arc::clone(&self.mixer);
        let running_clone = Arc::clone(&self.running);

        std::thread::spawn(move || {
            // Create audio output (must be done on non-async thread for cpal)
            let mut audio_output = match AudioOutput::new(None) {
                Ok(output) => output,
                Err(e) => {
                    error!("Failed to create audio output: {}", e);
                    return;
                }
            };

            // Wire mixer to audio output via callback
            // The callback runs on audio thread, so we use try_write() to avoid blocking
            let mixer_callback = move || {
                // Try to get mixer lock without blocking audio thread
                if let Ok(mut mixer) = mixer_clone.try_write() {
                    // Use tokio runtime to call async get_next_frame()
                    match tokio::runtime::Handle::try_current() {
                        Ok(handle) => handle.block_on(async {
                            mixer.get_next_frame().await
                        }),
                        Err(_) => {
                            // No tokio runtime available, return silence
                            AudioFrame::zero()
                        }
                    }
                } else {
                    // Mixer locked, return silence to avoid blocking audio thread
                    AudioFrame::zero()
                }
            };

            if let Err(e) = audio_output.start(mixer_callback) {
                error!("Failed to start audio output: {}", e);
                return;
            }

            info!("Audio output started successfully");

            // Keep audio output alive while engine is running
            // The audio stream continues running as long as audio_output isn't dropped
            loop {
                std::thread::sleep(std::time::Duration::from_millis(500));

                // Check if engine stopped (blocking check is ok on this thread)
                let rt = tokio::runtime::Handle::current();
                let should_stop = rt.block_on(async {
                    !*running_clone.read().await
                });

                if should_stop {
                    info!("Audio output stopping");
                    break;
                }
            }

            info!("Audio output stopped");
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
    /// Skip to next passage
    ///
    /// Stops current passage immediately and advances to next passage in queue.
    /// Emits PassageCompleted event with completed=false (skipped).
    ///
    /// [SSD-ENG-025] Skip passage functionality
    /// [API] POST /playback/skip
    pub async fn skip_next(&self) -> Result<()> {
        info!("Skip next command received");

        // Get current passage info before advancing
        let queue = self.queue.read().await;
        let current = queue.current().cloned();
        drop(queue);

        if let Some(current) = current {
            info!("Skipping passage: {:?}", current.passage_id);

            // Emit PassageCompleted event with completed=false (skipped)
            self.state.broadcast_event(wkmp_common::events::WkmpEvent::PassageCompleted {
                passage_id: current.passage_id.unwrap_or_else(|| Uuid::nil()),
                completed: false, // false = skipped
                timestamp: chrono::Utc::now(),
            });

            // Mark buffer as exhausted
            self.buffer_manager.mark_exhausted(current.queue_entry_id).await;

            // Stop mixer immediately
            let mut mixer = self.mixer.write().await;
            mixer.stop();
            drop(mixer);

            // Remove buffer from memory
            if let Some(passage_id) = current.passage_id {
                self.buffer_manager.remove(passage_id).await;
            }

            info!("Mixer stopped and buffer cleaned up");
        } else {
            info!("No current passage to skip");
        }

        // Advance queue to next passage
        let mut queue_write = self.queue.write().await;
        queue_write.advance();
        drop(queue_write);

        info!("Queue advanced to next passage");

        Ok(())
    }

    /// Seek to position in current passage
    ///
    /// Updates playback position to specified time. Clamps to passage bounds.
    /// Emits PlaybackProgress event with new position.
    ///
    /// # Arguments
    /// * `position_ms` - Target position in milliseconds
    ///
    /// # Returns
    /// * `Ok(())` if seek successful
    /// * `Err` if no passage playing or position invalid
    ///
    /// [SSD-ENG-026] Seek position control
    /// [API] POST /playback/seek
    pub async fn seek(&self, position_ms: u64) -> Result<()> {
        info!("Seek command received: position={}ms", position_ms);

        // Get current passage to validate seek bounds
        let queue = self.queue.read().await;
        let current = queue.current().cloned();
        drop(queue);

        if current.is_none() {
            return Err(Error::Playback("Cannot seek: no passage playing".to_string()));
        }

        let current = current.unwrap();

        // Get buffer to calculate position in frames
        let buffer_ref = self.buffer_manager.get_buffer(current.queue_entry_id).await;
        if buffer_ref.is_none() {
            return Err(Error::Playback("Cannot seek: buffer not available".to_string()));
        }

        let buffer = buffer_ref.unwrap();
        let buf_read = buffer.read().await;
        let sample_rate = buf_read.sample_rate;

        // Convert milliseconds to frames
        let position_frames = ((position_ms as f32 / 1000.0) * sample_rate as f32) as usize;

        // Clamp to buffer bounds
        let max_frames = buf_read.sample_count;
        let clamped_position = position_frames.min(max_frames.saturating_sub(1));

        drop(buf_read);
        drop(buffer);

        // Update mixer position
        let mut mixer = self.mixer.write().await;
        mixer.set_position(clamped_position).await?;
        drop(mixer);

        // Emit PlaybackProgress event with new position
        if let Some(passage_id) = current.passage_id {
            let actual_position_ms = ((clamped_position as f32 / sample_rate as f32) * 1000.0) as u64;

            // Get buffer again to read duration
            let buffer_ref = self.buffer_manager.get_buffer(current.queue_entry_id).await;
            if let Some(buffer) = buffer_ref {
                let buf_read = buffer.read().await;
                let duration_ms = buf_read.duration_ms();

                self.state.broadcast_event(wkmp_common::events::WkmpEvent::PlaybackProgress {
                    passage_id,
                    position_ms: actual_position_ms,
                    duration_ms,
                    timestamp: chrono::Utc::now(),
                });
            }

            info!("Seek complete: new position={}ms ({}ms requested)", actual_position_ms, position_ms);
        }

        Ok(())
    }

    /// Get current queue length
    ///
    /// Returns the number of passages in the playback queue.
    ///
    /// [API] GET /playback/queue (returns queue length in response)
    pub async fn queue_len(&self) -> usize {
        let queue = self.queue.read().await;
        queue.len()
    }

    /// Get all queue entries for API response
    ///
    /// Returns current, next, and all queued passages.
    /// [SSD-ENG-020] Queue processing
    /// [API] GET /playback/queue
    pub async fn get_queue_entries(&self) -> Vec<crate::playback::queue_manager::QueueEntry> {
        let queue = self.queue.read().await;
        let mut entries = Vec::new();

        // Add current if exists
        if let Some(current) = queue.current() {
            entries.push(current.clone());
        }

        // Add next if exists
        if let Some(next) = queue.next() {
            entries.push(next.clone());
        }

        // Add all queued entries
        entries.extend_from_slice(queue.queued());

        entries
    }

    /// Reorder a queue entry to a new position
    ///
    /// Moves the specified queue entry to the new position (0-based index).
    /// Only affects entries in the "queued" portion (not current/next).
    ///
    /// [API] POST /playback/queue/reorder
    /// [DB-QUEUE-080] Queue reordering
    pub async fn reorder_queue_entry(&self, queue_entry_id: Uuid, new_position: i32) -> Result<()> {
        info!("Reorder queue request: entry={}, position={}", queue_entry_id, new_position);

        // Call database reorder function
        crate::db::queue::reorder_queue(&self.db_pool, queue_entry_id, new_position).await?;

        // Reload queue from database to sync in-memory state
        let mut queue = self.queue.write().await;
        *queue = crate::playback::queue_manager::QueueManager::load_from_db(&self.db_pool).await?;
        drop(queue);

        info!("Queue reordered successfully");

        Ok(())
    }

    /// Get buffer statuses for all managed buffers
    ///
    /// Returns a map of passage_id to buffer status.
    ///
    /// [API] GET /playback/buffer_status
    pub async fn get_buffer_statuses(&self) -> std::collections::HashMap<Uuid, crate::audio::types::BufferStatus> {
        self.buffer_manager.get_all_statuses().await
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
            Some(passage.fade_in_curve.to_db_string().to_string()),
            Some(passage.fade_out_curve.to_db_string().to_string()),
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
            fade_in_curve: Some(passage.fade_in_curve.to_db_string().to_string()),
            fade_out_curve: Some(passage.fade_out_curve.to_db_string().to_string()),
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

        // Start mixer if current passage is ready and not playing
        // [SSD-MIX-030] Single passage playback initiation
        if let Some(current) = queue.current() {
            // Check if buffer is ready
            if self.buffer_manager.is_ready(current.queue_entry_id).await {
                // Check if mixer is currently idle
                let mixer = self.mixer.read().await;
                if mixer.get_current_passage_id().is_none() {
                    // Mixer is idle and buffer is ready - start playback!
                    drop(mixer); // Release read lock before acquiring write lock

                    // Get buffer from buffer manager
                    if let Some(buffer) = self.buffer_manager.get_buffer(current.queue_entry_id).await {
                        // Get passage timing information
                        let passage = self.get_passage_timing(current).await?;

                        // Calculate fade-in duration from timing points
                        // fade_in_point_ms is where fade-in completes, so duration = fade_in - start
                        let fade_in_duration_ms = passage.fade_in_point_ms.saturating_sub(passage.start_time_ms) as u32;

                        // Determine fade-in curve (default to Exponential if not specified)
                        let fade_in_curve = if let Some(curve_str) = current.fade_in_curve.as_ref() {
                            wkmp_common::FadeCurve::from_str(curve_str)
                                .unwrap_or(wkmp_common::FadeCurve::Exponential)
                        } else {
                            wkmp_common::FadeCurve::Exponential
                        };

                        info!(
                            "Starting playback of passage {} (queue_entry: {}) with fade-in: {}ms",
                            current.passage_id.unwrap_or_else(|| Uuid::nil()),
                            current.queue_entry_id,
                            fade_in_duration_ms
                        );

                        // Start mixer
                        self.mixer.write().await.start_passage(
                            buffer,
                            current.queue_entry_id,
                            Some(fade_in_curve),
                            fade_in_duration_ms,
                        ).await;

                        // Mark buffer as playing
                        self.buffer_manager.mark_playing(current.queue_entry_id).await;

                        // Update position tracking
                        self.position.write().await.queue_entry_id = Some(current.queue_entry_id);

                        // Emit PassageStarted event
                        // [Event-PassageStarted] Passage playback began
                        self.state.broadcast_event(wkmp_common::events::WkmpEvent::PassageStarted {
                            passage_id: current.passage_id.unwrap_or_else(|| Uuid::nil()),
                            timestamp: chrono::Utc::now(),
                        });

                        info!("Passage started successfully");
                    } else {
                        warn!("Buffer ready but not found in BufferManager for {}", current.queue_entry_id);
                    }
                }
            }
        }

        // Check for crossfade triggering
        // [XFD-IMPL-070] Crossfade initiation timing
        // [SSD-MIX-040] Crossfade state transition
        let mixer = self.mixer.read().await;
        let is_crossfading = mixer.is_crossfading();
        let current_passage_id = mixer.get_current_passage_id();
        let mixer_position_frames = mixer.get_position();
        drop(mixer);

        // Only trigger crossfade if:
        // 1. Mixer is playing a passage
        // 2. Mixer is NOT already crossfading
        // 3. We have a next passage in queue
        if let Some(current_id) = current_passage_id {
            if !is_crossfading {
                if let Some(current) = queue.current() {
                    if current.queue_entry_id == current_id {
                        // Get current passage timing to determine crossfade point
                        if let Ok(passage) = self.get_passage_timing(current).await {
                            // Get buffer to convert position frames to milliseconds
                            if let Some(buffer_ref) = self.buffer_manager.get_buffer(current.queue_entry_id).await {
                                let buffer = buffer_ref.read().await;
                                let position_ms = (mixer_position_frames as u64 * 1000) / buffer.sample_rate as u64;
                                drop(buffer);

                                // Calculate crossfade start point
                                // fade_out_point_ms is when crossfade should begin
                                let crossfade_start_ms = passage.fade_out_point_ms.unwrap_or_else(|| {
                                    // If no explicit fade_out_point, calculate from end
                                    // Default: start crossfade 5 seconds before end
                                    let end_ms = passage.end_time_ms.unwrap_or(0);
                                    if end_ms > 5000 {
                                        end_ms - 5000
                                    } else {
                                        end_ms
                                    }
                                });

                                // Check if we've reached the crossfade trigger point
                                if position_ms >= crossfade_start_ms {
                                    // Check if we have a next passage
                                    if let Some(next) = queue.next() {
                                        // Verify next buffer is ready
                                        if self.buffer_manager.is_ready(next.queue_entry_id).await {
                                            // Get next buffer
                                            if let Some(next_buffer) = self.buffer_manager.get_buffer(next.queue_entry_id).await {
                                                // Get next passage timing for fade curves
                                                if let Ok(next_passage) = self.get_passage_timing(next).await {
                                                    info!(
                                                        "Triggering crossfade: {} → {} at position {}ms",
                                                        current.queue_entry_id,
                                                        next.queue_entry_id,
                                                        position_ms
                                                    );

                                                    // Calculate crossfade durations
                                                    let fade_out_duration_ms = if let Some(end_ms) = passage.end_time_ms {
                                                        end_ms.saturating_sub(crossfade_start_ms) as u32
                                                    } else {
                                                        5000 // Default 5-second crossfade
                                                    };

                                                    let fade_in_duration_ms = next_passage.fade_in_point_ms
                                                        .saturating_sub(next_passage.start_time_ms) as u32;

                                                    // Start the crossfade!
                                                    // [SSD-MIX-040] Crossfade initiation
                                                    if let Err(e) = self.mixer.write().await.start_crossfade(
                                                        next_buffer,
                                                        next.queue_entry_id,
                                                        passage.fade_out_curve,
                                                        fade_out_duration_ms,
                                                        next_passage.fade_in_curve,
                                                        fade_in_duration_ms,
                                                    ).await {
                                                        error!("Failed to start crossfade: {}", e);
                                                    } else {
                                                        info!("Crossfade started successfully");

                                                        // Mark next buffer as playing
                                                        self.buffer_manager.mark_playing(next.queue_entry_id).await;

                                                        // Emit PassageStarted event for next passage
                                                        // (it's starting to fade in during crossfade)
                                                        self.state.broadcast_event(wkmp_common::events::WkmpEvent::PassageStarted {
                                                            passage_id: next.passage_id.unwrap_or_else(|| Uuid::nil()),
                                                            timestamp: chrono::Utc::now(),
                                                        });
                                                    }
                                                } else {
                                                    warn!("Could not get timing for next passage {}", next.queue_entry_id);
                                                }
                                            } else {
                                                warn!("Next buffer marked ready but not found: {}", next.queue_entry_id);
                                            }
                                        } else {
                                            debug!("Next passage buffer not ready yet at crossfade point (position: {}ms)", position_ms);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Check for passage completion
        // [SSD-MIX-060] Passage completion detection
        // [SSD-ENG-020] Queue processing and advancement
        let mixer = self.mixer.read().await;
        let is_finished = mixer.is_current_finished().await;
        let current_passage_id = mixer.get_current_passage_id();
        drop(mixer);

        if is_finished {
            if let Some(passage_id) = current_passage_id {
                info!("Passage {} completed", passage_id);

                // Get current queue entry for event emission
                if let Some(current) = queue.current() {
                    // Emit PassageCompleted event
                    // [Event-PassageCompleted] Passage playback finished
                    self.state.broadcast_event(wkmp_common::events::WkmpEvent::PassageCompleted {
                        passage_id: current.passage_id.unwrap_or_else(|| Uuid::nil()),
                        completed: true, // true = finished naturally, false = skipped/interrupted
                        timestamp: chrono::Utc::now(),
                    });

                    // Mark buffer as exhausted
                    self.buffer_manager.mark_exhausted(current.queue_entry_id).await;

                    info!("Marked buffer {} as exhausted", current.queue_entry_id);
                }

                // Release queue read lock before advancing (needs write lock)
                drop(queue);

                // Advance queue to next passage
                // This removes the completed passage and moves next → current
                let mut queue_write = self.queue.write().await;
                queue_write.advance();
                drop(queue_write);

                info!("Queue advanced to next passage");

                // Clean up the exhausted buffer (free memory)
                self.buffer_manager.remove(passage_id).await;

                // Stop the mixer (will be restarted on next iteration with new passage)
                self.mixer.write().await.stop();

                debug!("Mixer stopped after passage completion");

                // Return early - next iteration will start the new current passage
                return Ok(());
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

            // Get current position from mixer
            // [SSD-MIX-010] Mixer tracks playback position
            let mixer = self.mixer.read().await;
            let mixer_position_frames = mixer.get_position();
            let mixer_passage_id = mixer.get_current_passage_id();
            drop(mixer);

            // Update position in PlaybackPosition for reference
            self.position.write().await.frame_position = mixer_position_frames;

            // Update current passage in shared state
            let queue = self.queue.read().await;

            if let Some(current) = queue.current() {
                // Only update if mixer is playing the current queue entry
                if mixer_passage_id == Some(current.queue_entry_id) {
                    // Get buffer to calculate duration
                    if let Some(buffer_ref) = self.buffer_manager.get_buffer(current.queue_entry_id).await {
                        let buffer = buffer_ref.read().await;

                        // Convert frame position to milliseconds
                        let position_ms = (mixer_position_frames as u64 * 1000) / buffer.sample_rate as u64;
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
                                debug!(
                                    "PlaybackProgress: position={}ms duration={}ms",
                                    current_passage.position_ms,
                                    current_passage.duration_ms
                                );
                                self.state.broadcast_event(wkmp_common::events::WkmpEvent::PlaybackProgress {
                                    passage_id: current_passage.passage_id.unwrap_or_else(|| Uuid::nil()),
                                    position_ms: current_passage.position_ms,
                                    duration_ms: current_passage.duration_ms,
                                    timestamp: chrono::Utc::now(),
                                });
                            }
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
            mixer: Arc::clone(&self.mixer),
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

    #[tokio::test]
    async fn test_skip_next() {
        let db = create_test_db().await;
        let state = Arc::new(SharedState::new());

        let engine = PlaybackEngine::new(db, state.clone()).await.unwrap();

        // Enqueue 3 test passages
        let passage1 = PathBuf::from("/test/song1.mp3");
        let passage2 = PathBuf::from("/test/song2.mp3");
        let passage3 = PathBuf::from("/test/song3.mp3");

        engine.enqueue_file(passage1).await.unwrap();
        engine.enqueue_file(passage2).await.unwrap();
        engine.enqueue_file(passage3).await.unwrap();

        // Verify queue has 3 entries
        {
            let queue = engine.queue.read().await;
            assert_eq!(queue.len(), 3);
            assert!(queue.current().is_some());
        }

        // Skip current passage
        engine.skip_next().await.unwrap();

        // Verify queue advanced (now has 2 entries)
        {
            let queue = engine.queue.read().await;
            assert_eq!(queue.len(), 2);
            assert!(queue.current().is_some());
        }

        // Skip again
        engine.skip_next().await.unwrap();

        // Verify queue advanced again (now has 1 entry)
        {
            let queue = engine.queue.read().await;
            assert_eq!(queue.len(), 1);
            assert!(queue.current().is_some());
        }

        // Skip final passage
        engine.skip_next().await.unwrap();

        // Verify queue is now empty
        {
            let queue = engine.queue.read().await;
            assert_eq!(queue.len(), 0);
            assert!(queue.current().is_none());
        }
    }

    #[tokio::test]
    async fn test_skip_empty_queue() {
        let db = create_test_db().await;
        let state = Arc::new(SharedState::new());

        let engine = PlaybackEngine::new(db, state.clone()).await.unwrap();

        // Try to skip with empty queue (should not panic)
        let result = engine.skip_next().await;
        assert!(result.is_ok());

        // Queue should still be empty
        {
            let queue = engine.queue.read().await;
            assert_eq!(queue.len(), 0);
        }
    }
}
