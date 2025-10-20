//! Playback engine orchestration
//!
//! Coordinates queue processing, buffer management, decoding, and audio output.
//!
//! **Traceability:**
//! - [SSD-FLOW-010] Complete playback sequence
//! - [SSD-ENG-020] Queue processing
//! - [REV002] Event-driven position tracking

use crate::audio::output::AudioOutput;
use crate::audio::types::AudioFrame;
use crate::db::passages::{create_ephemeral_passage, get_passage_with_timing, validate_passage_timing, PassageWithTiming};
use crate::error::{Error, Result};
use crate::playback::buffer_manager::BufferManager;
use crate::playback::buffer_events::BufferEvent;
use crate::playback::decoder_pool::DecoderPool;
use crate::playback::events::PlaybackEvent;
use crate::playback::pipeline::mixer::CrossfadeMixer;
use crate::playback::queue_manager::{QueueEntry, QueueManager};
use crate::playback::ring_buffer::AudioRingBuffer;
use crate::playback::song_timeline::SongTimeline;
use crate::playback::types::DecodePriority;
use crate::state::{CurrentPassage, PlaybackState, SharedState};
use sqlx::{Pool, Sqlite};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio::sync::RwLock;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tokio::time::{interval, Duration};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Playback position tracking with lock-free atomic frame position
///
/// **[ISSUE-8]** Optimized to reduce lock contention in playback loop.
/// Frame position is updated on every iteration (~100Hz), so uses AtomicU64.
/// Queue entry ID is updated rarely (only on passage change), so uses RwLock.
struct PlaybackPosition {
    /// Current passage UUID (queue entry) - updated infrequently
    queue_entry_id: Arc<RwLock<Option<Uuid>>>,

    /// Current frame position in buffer - updated every loop iteration
    /// [ISSUE-8] AtomicU64 for lock-free updates in hot path
    frame_position: Arc<AtomicU64>,
}

impl PlaybackPosition {
    fn new() -> Self {
        Self {
            queue_entry_id: Arc::new(RwLock::new(None)),
            frame_position: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Clone the inner Arcs for sharing across threads
    fn clone_handles(&self) -> Self {
        Self {
            queue_entry_id: Arc::clone(&self.queue_entry_id),
            frame_position: Arc::clone(&self.frame_position),
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
    /// [ISSUE-8] Now uses internal atomics for lock-free frame position updates
    position: PlaybackPosition,

    /// Playback loop running flag
    running: Arc<RwLock<bool>>,

    /// Position event channel sender
    /// **[REV002]** Event-driven position tracking
    /// Mixer sends position events to handler via this channel
    position_event_tx: mpsc::UnboundedSender<PlaybackEvent>,

    /// Master volume control (shared with AudioOutput)
    /// **[ARCH-VOL-020]** Volume Arc shared between engine and audio output
    /// Updated by API handlers, read by audio callback
    volume: Arc<Mutex<f32>>,

    /// Position event channel receiver
    /// **[REV002]** Taken by position_event_handler on start
    /// Wrapped in Option so it can be taken once
    position_event_rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<PlaybackEvent>>>>,

    /// Song timeline for current passage
    /// **[REV002]** Loaded when passage starts, used for boundary detection
    /// None when no passage playing or passage has no songs
    current_song_timeline: Arc<RwLock<Option<SongTimeline>>>,

    /// Audio expected flag for ring buffer underrun classification
    /// Set to true when Playing state with non-empty queue
    /// Set to false when Paused or queue is empty
    /// Shared with audio ring buffer consumer for context-aware logging
    audio_expected: Arc<AtomicBool>,

    /// Buffer event channel receiver for instant mixer start
    /// **[PERF-POLL-010]** Event-driven buffer readiness
    /// Receives ReadyForStart events when buffers reach minimum threshold
    buffer_event_rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<BufferEvent>>>>,
}

impl PlaybackEngine {
    /// Create new playback engine
    ///
    /// [SSD-FLOW-010] Initialize all components
    /// **[REV002]** Configure event-driven position tracking
    pub async fn new(db_pool: Pool<Sqlite>, state: Arc<SharedState>) -> Result<Self> {
        use std::time::Instant;
        let engine_start = Instant::now();
        info!("Creating playback engine");

        // **[PERF-INIT-010]** Parallel database queries for faster initialization
        let db_start = Instant::now();
        let (initial_volume, min_buffer_threshold, interval_ms, grace_period_ms, mixer_config) = tokio::join!(
            crate::db::settings::get_volume(&db_pool),
            crate::db::settings::load_minimum_buffer_threshold(&db_pool),
            crate::db::settings::load_position_event_interval(&db_pool),
            crate::db::settings::load_ring_buffer_grace_period(&db_pool),
            crate::db::settings::load_mixer_thread_config(&db_pool),
        );
        let db_elapsed = db_start.elapsed();

        let initial_volume = initial_volume?;
        let min_buffer_threshold = min_buffer_threshold?;
        let interval_ms = interval_ms?;
        let _grace_period_ms = grace_period_ms?;  // Loaded in parallel for performance
        let _mixer_config = mixer_config?;  // Loaded in parallel for performance

        info!(
            "⚡ Parallel config loaded in {:.2}ms: volume={:.2}, buffer_threshold={}ms, interval={}ms",
            db_elapsed.as_secs_f64() * 1000.0,
            initial_volume,
            min_buffer_threshold,
            interval_ms
        );

        let volume = Arc::new(Mutex::new(initial_volume));

        // **[PERF-POLL-010]** Create buffer event channel for instant mixer start
        let (buffer_event_tx, buffer_event_rx) = mpsc::unbounded_channel();

        // Create buffer manager
        let buffer_manager = Arc::new(BufferManager::new());

        // **[PERF-POLL-010]** Configure buffer manager with event channel and threshold
        buffer_manager.set_event_channel(buffer_event_tx).await;
        buffer_manager.set_min_buffer_threshold(min_buffer_threshold).await;

        // Create decoder pool
        let decoder_pool = DecoderPool::new(Arc::clone(&buffer_manager));

        // **[REV002]** Create position event channel
        let (position_event_tx, position_event_rx) = mpsc::unbounded_channel();

        // Create mixer
        // [SSD-MIX-010] Crossfade mixer for sample-accurate mixing
        let mut mixer = CrossfadeMixer::new();

        // **[REV002]** Configure mixer with event channel
        mixer.set_event_channel(position_event_tx.clone());

        // **[REV002]** Use already-loaded position_event_interval_ms
        mixer.set_position_event_interval_ms(interval_ms);

        // **[SSD-UND-010]** Configure mixer with buffer manager for underrun detection
        mixer.set_buffer_manager(Arc::clone(&buffer_manager));

        let mixer = Arc::new(RwLock::new(mixer));

        // Load queue from database
        let queue_start = Instant::now();
        let queue_manager = QueueManager::load_from_db(&db_pool).await?;
        let queue_elapsed = queue_start.elapsed();
        info!(
            "Queue loaded in {:.2}ms: {} entries",
            queue_elapsed.as_secs_f64() * 1000.0,
            queue_manager.len()
        );

        let total_elapsed = engine_start.elapsed();
        info!(
            "✅ Playback engine created in {:.2}ms",
            total_elapsed.as_secs_f64() * 1000.0
        );

        Ok(Self {
            db_pool,
            state,
            queue: Arc::new(RwLock::new(queue_manager)),
            buffer_manager,
            decoder_pool: Arc::new(RwLock::new(Some(decoder_pool))),
            mixer,
            position: PlaybackPosition::new(), // [ISSUE-8] Direct initialization, Arcs inside
            running: Arc::new(RwLock::new(false)),
            position_event_tx,
            volume, // [ARCH-VOL-020] Shared volume control
            position_event_rx: Arc::new(RwLock::new(Some(position_event_rx))),
            current_song_timeline: Arc::new(RwLock::new(None)),
            audio_expected: Arc::new(AtomicBool::new(false)), // Initially no audio expected
            buffer_event_rx: Arc::new(RwLock::new(Some(buffer_event_rx))), // [PERF-POLL-010] Buffer event channel
        })
    }

    /// Start playback engine background tasks
    ///
    /// [SSD-FLOW-010] Begin processing queue and managing buffers
    /// [ISSUE-1] Lock-free audio callback using ring buffer
    pub async fn start(&self) -> Result<()> {
        use std::time::Instant;
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

        // **[REV002]** Start position event handler (replaces position_tracking_loop)
        let self_clone = self.clone_handles();
        tokio::spawn(async move {
            self_clone.position_event_handler().await;
        });

        // **[PERF-POLL-010]** Start buffer event handler for instant mixer start
        let self_clone = self.clone_handles();
        tokio::spawn(async move {
            self_clone.buffer_event_handler().await;
        });

        // **[SSE-UI-030]** Start PlaybackPosition emission task (every 1s)
        let self_clone = self.clone_handles();
        tokio::spawn(async move {
            self_clone.playback_position_emitter().await;
        });

        // Start BufferChainStatus emission task (every 1s, only when changed)
        let self_clone = self.clone_handles();
        tokio::spawn(async move {
            self_clone.buffer_chain_status_emitter().await;
        });

        // Create lock-free ring buffer for audio frames
        // [SSD-OUT-012] Real-time audio callback requires lock-free operation
        // [ISSUE-1] Replaces try_write() pattern with lock-free ring buffer
        // [SSD-RBUF-014] Load grace period from database settings
        info!("Creating lock-free audio ring buffer");
        let rb_start = Instant::now();
        let grace_period_ms = crate::db::settings::load_ring_buffer_grace_period(&self.db_pool).await?;
        let mixer_config = crate::db::settings::load_mixer_thread_config(&self.db_pool).await?;
        let ring_buffer = AudioRingBuffer::new(None, grace_period_ms, Arc::clone(&self.audio_expected)); // Use default size (2048 frames = ~46ms @ 44.1kHz)
        let (mut producer, mut consumer) = ring_buffer.split();
        let rb_elapsed = rb_start.elapsed();

        info!(
            "⚡ Ring buffer created in {:.2}ms (grace_period={}ms, check_interval={}μs)",
            rb_elapsed.as_secs_f64() * 1000.0,
            grace_period_ms,
            mixer_config.check_interval_us
        );

        // Set initial audio_expected flag based on current state
        self.update_audio_expected_flag().await;

        // Start mixer thread that fills the ring buffer
        // This thread continuously calls mixer.get_next_frame() and pushes to ring buffer
        let mixer_clone = Arc::clone(&self.mixer);
        let running_clone = Arc::clone(&self.running);
        let audio_expected_clone = Arc::clone(&self.audio_expected);
        let check_interval_us = mixer_config.check_interval_us;
        let batch_size_low = mixer_config.batch_size_low;
        let batch_size_optimal = mixer_config.batch_size_optimal;
        tokio::spawn(async move {
            info!("Mixer thread started");
            let mut check_interval = interval(Duration::from_micros(check_interval_us));

            loop {
                // Check if engine stopped
                if !*running_clone.read().await {
                    info!("Mixer thread stopping");
                    break;
                }

                // Check if audio output is expected (queue has passages to play)
                let audio_expected = audio_expected_clone.load(std::sync::atomic::Ordering::Acquire);

                if !audio_expected {
                    // Queue is empty or paused - yield when idle
                    check_interval.tick().await;

                    let occupied = producer.occupied_len();
                    let capacity = producer.capacity();
                    let fill_percent = occupied as f32 / capacity as f32;

                    if fill_percent < 0.10 {
                        // Buffer nearly empty - add silence
                        // [XFD-PAUS-010] When paused, output flatline silence (not audio)
                        for _ in 0..4 {
                            if !producer.push(AudioFrame::zero()) {
                                break;
                            }
                        }
                    }
                    continue;
                }

                // Audio IS expected - check if mixer is actually playing
                let mixer_playing = {
                    let mixer = mixer_clone.read().await;
                    mixer.get_current_passage_id().is_some()
                };

                if !mixer_playing {
                    // Mixer not playing yet (still in None state) - yield and wait
                    check_interval.tick().await;
                    continue;
                }

                // Mixer IS playing - use graduated filling strategy
                // [SSD-MIX-020] Use configurable batch sizes from database
                let needs_filling = producer.needs_frames(); // < 50%
                let is_optimal = producer.is_fill_optimal(); // 50-75%

                if needs_filling {
                    // Buffer < 50% - fill moderately
                    let mut mixer = mixer_clone.write().await;

                    for _ in 0..batch_size_low {
                        let frame = mixer.get_next_frame().await;
                        if !producer.push(frame) {
                            break;
                        }
                    }
                    // Lock released here

                    // Yield to allow audio callback to consume
                    check_interval.tick().await;
                } else if is_optimal {
                    // Buffer 50-75% - top up conservatively
                    check_interval.tick().await;

                    let mut mixer = mixer_clone.write().await;
                    for _ in 0..batch_size_optimal {
                        let frame = mixer.get_next_frame().await;
                        if !producer.push(frame) {
                            break;
                        }
                    }
                } else {
                    // Buffer > 75% - just yield and wait for consumption
                    check_interval.tick().await;
                }
            }

            info!("Mixer thread stopped");
        });

        // Initialize audio output in a background thread
        // [SSD-OUT-010] Create audio device interface
        // [SSD-OUT-012] Begin audio stream with lock-free callback
        // Note: AudioOutput is not Send/Sync due to cpal::Stream, so we create it in a thread
        // that just keeps it alive. The audio stream runs independently once started.
        info!("Initializing audio output with lock-free callback");
        let running_clone2 = Arc::clone(&self.running);
        let volume_clone = Arc::clone(&self.volume); // [ARCH-VOL-020] Share volume with audio output

        // Capture runtime handle while we're still in async context
        let rt_handle = tokio::runtime::Handle::current();

        std::thread::spawn(move || {
            // Create audio output (must be done on non-async thread for cpal)
            // [ARCH-VOL-020] Pass shared volume Arc for synchronized control
            let mut audio_output = match AudioOutput::new_with_volume(None, Some(volume_clone)) {
                Ok(output) => output,
                Err(e) => {
                    error!("Failed to create audio output: {}", e);
                    return;
                }
            };

            // Lock-free audio callback - reads from ring buffer only
            // [SSD-OUT-012] Real-time audio callback with no locks
            // [ISSUE-1] Fixed: No more try_write() or block_on() in audio callback
            let audio_callback = move || {
                // Lock-free read from ring buffer
                consumer.pop().unwrap_or_else(|| {
                    // Buffer underrun - return silence
                    // This is logged automatically by the ring buffer
                    AudioFrame::zero()
                })
            };

            if let Err(e) = audio_output.start(audio_callback) {
                error!("Failed to start audio output: {}", e);
                return;
            }

            info!("Audio output started successfully with lock-free callback");

            // Keep audio output alive while engine is running
            // The audio stream continues running as long as audio_output isn't dropped
            loop {
                std::thread::sleep(std::time::Duration::from_millis(500));

                // Check if engine stopped (blocking check is ok on this thread)
                // Use captured runtime handle from async context
                let should_stop = rt_handle.block_on(async {
                    !*running_clone2.read().await
                });

                if should_stop {
                    info!("Audio output stopping");
                    break;
                }
            }

            info!("Audio output stopped");
        });

        info!("Playback engine started with lock-free audio architecture");
        Ok(())
    }

    /// Stop playback engine gracefully
    ///
    /// [SSD-DEC-033] Shutdown decoder pool with timeout
    /// [ISSUE-2] Persist playback state on clean shutdown
    /// [ISSUE-12] Handle decoder pool shutdown errors gracefully
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping playback engine");

        // Persist playback state before shutdown
        // [ISSUE-2] Save state on clean shutdown
        if let Some(passage) = self.state.get_current_passage().await {
            // Save current position (convert u64 to i64 safely)
            let position_ms_i64 = passage.position_ms.min(i64::MAX as u64) as i64;
            if let Err(e) = crate::db::settings::save_playback_position(&self.db_pool, position_ms_i64).await {
                warn!("Failed to persist playback position on shutdown: {}", e);
            }

            // Save current passage ID
            if let Some(passage_id) = passage.passage_id {
                if let Err(e) = crate::db::settings::save_last_passage_id(&self.db_pool, passage_id).await {
                    warn!("Failed to persist last passage ID on shutdown: {}", e);
                }
            }

            // Save queue state
            if let Err(e) = crate::db::settings::save_queue_state(&self.db_pool, Some(passage.queue_entry_id)).await {
                warn!("Failed to persist queue state on shutdown: {}", e);
            }

            info!("Persisted playback state on clean shutdown");
        }

        // Mark as not running
        *self.running.write().await = false;

        // Shutdown decoder pool
        // [ISSUE-12] Log errors but don't propagate them (continue shutdown)
        if let Some(decoder_pool) = self.decoder_pool.write().await.take() {
            if let Err(e) = decoder_pool.shutdown() {
                error!("Decoder pool shutdown error (continuing anyway): {}", e);
                // Continue shutdown - don't propagate error
            } else {
                info!("Decoder pool shut down successfully");
            }
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

        // [XFD-PAUS-020] Check if resuming from pause
        let resuming_from_pause = old_state == PlaybackState::Paused;

        self.state.set_playback_state(PlaybackState::Playing).await;

        // [XFD-PAUS-020] If resuming from pause, initiate resume fade-in
        if resuming_from_pause {
            // Load resume fade-in settings from database
            let fade_duration_ms = crate::db::settings::load_resume_fade_in_duration(&self.db_pool)
                .await
                .unwrap_or(500); // Default: 0.5 seconds
            let fade_curve = crate::db::settings::load_resume_fade_in_curve(&self.db_pool)
                .await
                .unwrap_or_else(|_| "exponential".to_string());

            self.mixer.write().await.resume(fade_duration_ms, &fade_curve);

            info!("Resuming from pause with {}ms {} fade-in", fade_duration_ms, fade_curve);
        }

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

        // Update audio_expected flag for ring buffer underrun classification
        self.update_audio_expected_flag().await;

        Ok(())
    }

    /// Pause
    ///
    /// [API] POST /playback/pause
    /// [REQ-PERS-011] Persist playback position on pause
    /// [ISSUE-2] Database persistence for playback state
    pub async fn pause(&self) -> Result<()> {
        info!("Pause command received");
        let old_state = self.state.get_playback_state().await;
        self.state.set_playback_state(PlaybackState::Paused).await;

        // [XFD-PAUS-010] Tell mixer to enter pause state (outputs silence)
        self.mixer.write().await.pause();

        // Persist playback state to database
        // [REQ-PERS-011] Save position and passage ID on pause
        if let Some(passage) = self.state.get_current_passage().await {
            // Save current position (convert u64 to i64 safely)
            let position_ms_i64 = passage.position_ms.min(i64::MAX as u64) as i64;
            if let Err(e) = crate::db::settings::save_playback_position(&self.db_pool, position_ms_i64).await {
                warn!("Failed to persist playback position: {}", e);
            }

            // Save current passage ID
            if let Some(passage_id) = passage.passage_id {
                if let Err(e) = crate::db::settings::save_last_passage_id(&self.db_pool, passage_id).await {
                    warn!("Failed to persist last passage ID: {}", e);
                }
            }

            // Save queue entry ID
            if let Err(e) = crate::db::settings::save_queue_state(&self.db_pool, Some(passage.queue_entry_id)).await {
                warn!("Failed to persist queue state: {}", e);
            }

            debug!("Persisted playback state: position={} ms, passage_id={:?}",
                passage.position_ms, passage.passage_id);
        }

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

        // Update audio_expected flag for ring buffer underrun classification
        self.update_audio_expected_flag().await;

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

        // [ISSUE-13] Validate that there's something to skip
        let current = current.ok_or_else(|| {
            Error::InvalidState("No passage to skip - queue is empty".to_string())
        })?;

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

        // Advance queue to next passage
        let mut queue_write = self.queue.write().await;
        queue_write.advance();
        drop(queue_write);

        info!("Queue advanced to next passage");

        // Update audio_expected flag for ring buffer underrun classification
        self.update_audio_expected_flag().await;

        Ok(())
    }

    /// Clear entire queue
    ///
    /// [API] POST /playback/queue/clear
    /// Clear all passages from queue
    ///
    /// Stops current playback immediately and clears all queue entries.
    /// Emits QueueChanged event.
    pub async fn clear_queue(&self) -> Result<()> {
        info!("Clear queue command received");

        // Get current passage to clean up buffer if playing
        let queue = self.queue.read().await;
        let current = queue.current().cloned();
        drop(queue);

        // Stop mixer immediately
        let mut mixer = self.mixer.write().await;
        let passage_id_before_stop = mixer.get_current_passage_id();
        mixer.stop();
        let passage_id_after_stop = mixer.get_current_passage_id();
        drop(mixer);

        info!(
            "Mixer stopped: passage_before={:?}, passage_after={:?}",
            passage_id_before_stop, passage_id_after_stop
        );

        // Clean up current buffer if exists
        if let Some(current) = current {
            if let Some(passage_id) = current.passage_id {
                self.buffer_manager.remove(passage_id).await;
                info!("Removed current buffer: {}", passage_id);
            }
        }

        // Clear all buffers from buffer manager
        self.buffer_manager.clear().await;

        // Clear in-memory queue
        let mut queue_write = self.queue.write().await;
        queue_write.clear();
        drop(queue_write);

        info!("In-memory queue cleared");

        // Clear shared state
        self.state.set_current_passage(None).await;

        // Update audio_expected flag for ring buffer underrun classification
        self.update_audio_expected_flag().await;

        // Emit QueueChanged event
        self.state.broadcast_event(wkmp_common::events::WkmpEvent::QueueChanged {
            timestamp: chrono::Utc::now(),
        });

        info!("Queue cleared successfully");

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

    /// Get shared volume Arc for API access
    ///
    /// **[ARCH-VOL-020]** Provides direct access to shared volume control
    ///
    /// # Returns
    /// Cloned Arc to volume Mutex - can be used to update volume from API handlers
    pub fn get_volume_arc(&self) -> Arc<Mutex<f32>> {
        Arc::clone(&self.volume)
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
        tracing::debug!("get_queue_entries: Starting");
        tracing::debug!("get_queue_entries: About to acquire queue read lock");
        let queue = self.queue.read().await;
        tracing::debug!("get_queue_entries: Acquired queue read lock");
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

        tracing::debug!("get_queue_entries: Returning {} entries", entries.len());
        entries
    }
    /// Get buffer chain status for monitoring
    ///
    /// Returns status of all decoder-resampler-fade-buffer chains (slots 0-1).
    /// Used for developer UI monitoring panel.
    pub async fn get_buffer_chains(&self) -> Vec<wkmp_common::events::BufferChainInfo> {
        use wkmp_common::events::BufferChainInfo;

        let mut chains = Vec::new();

        // Get mixer state to determine active passages
        let mixer = self.mixer.read().await;
        let mixer_state = mixer.get_state_info();
        drop(mixer);

        // Get queue entries
        let queue = self.queue.read().await;
        let current = queue.current().cloned();
        let next = queue.next().cloned();
        drop(queue);

        // Slot 0: Current passage (if playing)
        if let Some(ref entry) = current {
            let buffer_info = self.buffer_manager.get_buffer_info(entry.queue_entry_id).await;
            let is_active = mixer_state.current_passage_id == entry.passage_id;

            let file_name = entry.file_path.file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string());

            chains.push(BufferChainInfo {
                slot_index: 0,
                queue_entry_id: Some(entry.queue_entry_id),
                passage_id: entry.passage_id,
                file_name,
                buffer_fill_percent: buffer_info.as_ref().map(|b| b.fill_percent).unwrap_or(0.0),
                buffer_fill_samples: buffer_info.as_ref().map(|b| b.samples_buffered).unwrap_or(0),
                buffer_capacity_samples: buffer_info.as_ref().map(|b| b.capacity_samples).unwrap_or(0),
                playback_position_frames: mixer_state.current_position_frames,
                playback_position_ms: (mixer_state.current_position_frames as u64 * 1000) / 44100,
                duration_ms: buffer_info.as_ref().and_then(|b| b.duration_ms),
                is_active_in_mixer: is_active,
                mixer_role: if mixer_state.is_crossfading { "Crossfading".to_string() } else { "Current".to_string() },
                started_at: None, // TODO: Track start time
            });
        } else {
            // Idle slot
            chains.push(BufferChainInfo {
                slot_index: 0,
                queue_entry_id: None,
                passage_id: None,
                file_name: None,
                buffer_fill_percent: 0.0,
                buffer_fill_samples: 0,
                buffer_capacity_samples: 0,
                playback_position_frames: 0,
                playback_position_ms: 0,
                duration_ms: None,
                is_active_in_mixer: false,
                mixer_role: "Idle".to_string(),
                started_at: None,
            });
        }

        // Slot 1: Next passage (if queued)
        if let Some(ref entry) = next {
            let buffer_info = self.buffer_manager.get_buffer_info(entry.queue_entry_id).await;
            let is_active = mixer_state.next_passage_id == entry.passage_id;

            let file_name = entry.file_path.file_name()
                .and_then(|n| n.to_str())
                .map(|s| s.to_string());

            chains.push(BufferChainInfo {
                slot_index: 1,
                queue_entry_id: Some(entry.queue_entry_id),
                passage_id: entry.passage_id,
                file_name,
                buffer_fill_percent: buffer_info.as_ref().map(|b| b.fill_percent).unwrap_or(0.0),
                buffer_fill_samples: buffer_info.as_ref().map(|b| b.samples_buffered).unwrap_or(0),
                buffer_capacity_samples: buffer_info.as_ref().map(|b| b.capacity_samples).unwrap_or(0),
                playback_position_frames: mixer_state.next_position_frames,
                playback_position_ms: (mixer_state.next_position_frames as u64 * 1000) / 44100,
                duration_ms: buffer_info.as_ref().and_then(|b| b.duration_ms),
                is_active_in_mixer: is_active,
                mixer_role: if mixer_state.is_crossfading { "Crossfading".to_string() } else { "Next".to_string() },
                started_at: None,
            });
        } else {
            // Idle slot
            chains.push(BufferChainInfo {
                slot_index: 1,
                queue_entry_id: None,
                passage_id: None,
                file_name: None,
                buffer_fill_percent: 0.0,
                buffer_fill_samples: 0,
                buffer_capacity_samples: 0,
                playback_position_frames: 0,
                playback_position_ms: 0,
                duration_ms: None,
                is_active_in_mixer: false,
                mixer_role: "Idle".to_string(),
                started_at: None,
            });
        }

        chains
    }


    /// Verify queue synchronization between in-memory and database
    ///
    /// **[ISSUE-6]** Queue consistency validation
    ///
    /// Compares in-memory queue state with database queue table.
    /// Logs warnings if discrepancies detected.
    ///
    /// # Returns
    /// true if synchronized, false if mismatches found
    ///
    /// # Notes
    /// This is a diagnostic/validation method, not used in normal operation.
    /// Can be called after operations to verify sync, or periodically for health checks.
    pub async fn verify_queue_sync(&self) -> bool {
        use tracing::warn;

        // Get in-memory queue state
        let queue = self.queue.read().await;
        let mem_entries = {
            let mut entries = Vec::new();
            if let Some(current) = queue.current() {
                entries.push(current.queue_entry_id);
            }
            if let Some(next) = queue.next() {
                entries.push(next.queue_entry_id);
            }
            for queued in queue.queued() {
                entries.push(queued.queue_entry_id);
            }
            entries
        };
        drop(queue);

        // Get database queue state
        let db_entries = match crate::db::queue::get_queue(&self.db_pool).await {
            Ok(entries) => entries.into_iter()
                .filter_map(|e| uuid::Uuid::parse_str(&e.guid).ok())
                .collect::<Vec<_>>(),
            Err(e) => {
                warn!("Queue sync verification failed - cannot read database: {}", e);
                return false;
            }
        };

        // Compare lengths
        if mem_entries.len() != db_entries.len() {
            warn!(
                "Queue sync mismatch: in-memory has {} entries, database has {} entries",
                mem_entries.len(),
                db_entries.len()
            );
            return false;
        }

        // Compare entry IDs in order
        for (i, (mem_id, db_id)) in mem_entries.iter().zip(db_entries.iter()).enumerate() {
            if mem_id != db_id {
                warn!(
                    "Queue sync mismatch at position {}: in-memory={}, database={}",
                    i, mem_id, db_id
                );
                return false;
            }
        }

        debug!("Queue sync verification passed ({} entries)", mem_entries.len());
        true
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

    /// Update audio_expected flag based on playback state and queue state
    ///
    /// Sets flag to true when: Playing state AND queue has passages
    /// Sets flag to false when: Paused OR queue is empty
    ///
    /// This flag is used by the ring buffer consumer to classify underrun log levels:
    /// - Expected underruns (startup, paused, empty queue): TRACE
    /// - Unexpected underruns (active playback): WARN
    async fn update_audio_expected_flag(&self) {
        let state = self.state.get_playback_state().await;
        let queue = self.queue.read().await;
        let has_passages = queue.len() > 0;
        drop(queue);

        let expected = state == PlaybackState::Playing && has_passages;

        // Use Release ordering to ensure visibility across threads
        self.audio_expected.store(expected, Ordering::Release);

        debug!("Audio expected flag updated: {} (state={:?}, queue_len={})",
               expected, state, if has_passages { "non-empty" } else { "empty" });
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
    /// [ISSUE-4] Validate passage timing and file existence
    /// [XFD-IMPL-040] through [XFD-IMPL-043] Timing validation
    pub async fn enqueue_file(&self, file_path: PathBuf) -> Result<Uuid> {
        info!("Enqueuing file: {}", file_path.display());

        // [ISSUE-4] Validate file exists and is readable
        if !file_path.exists() {
            return Err(Error::Config(format!(
                "File does not exist: {}",
                file_path.display()
            )));
        }

        if !file_path.is_file() {
            return Err(Error::Config(format!(
                "Path is not a file: {}",
                file_path.display()
            )));
        }

        // Create ephemeral passage
        let passage = create_ephemeral_passage(file_path.clone());

        // [ISSUE-4] Validate passage timing per XFD-IMPL-040 through XFD-IMPL-043
        // This corrects invalid values and logs warnings
        let passage = validate_passage_timing(passage)?;

        debug!(
            "Validated passage timing: start={} ticks, end={:?} ticks, fade_in={} ticks, fade_out={:?} ticks",
            passage.start_time_ticks,
            passage.end_time_ticks,
            passage.fade_in_point_ticks,
            passage.fade_out_point_ticks
        );

        // Add to database queue
        // Convert ticks to milliseconds for database storage
        let queue_entry_id = crate::db::queue::enqueue(
            &self.db_pool,
            file_path.to_string_lossy().to_string(),
            passage.passage_id,
            None, // Append to end
            Some(wkmp_common::timing::ticks_to_ms(passage.start_time_ticks)),
            passage.end_time_ticks.map(|t| wkmp_common::timing::ticks_to_ms(t)),
            Some(wkmp_common::timing::ticks_to_ms(passage.lead_in_point_ticks)),
            passage.lead_out_point_ticks.map(|t| wkmp_common::timing::ticks_to_ms(t)),
            Some(wkmp_common::timing::ticks_to_ms(passage.fade_in_point_ticks)),
            passage.fade_out_point_ticks.map(|t| wkmp_common::timing::ticks_to_ms(t)),
            Some(passage.fade_in_curve.to_db_string().to_string()),
            Some(passage.fade_out_curve.to_db_string().to_string()),
        )
        .await?;

        // Add to in-memory queue
        // Convert ticks to milliseconds for queue entry (matches database format)
        let entry = QueueEntry {
            queue_entry_id,
            passage_id: passage.passage_id,
            file_path,
            play_order: 0, // Will be managed by database
            start_time_ms: Some(wkmp_common::timing::ticks_to_ms(passage.start_time_ticks) as u64),
            end_time_ms: passage.end_time_ticks.map(|t| wkmp_common::timing::ticks_to_ms(t) as u64),
            lead_in_point_ms: Some(wkmp_common::timing::ticks_to_ms(passage.lead_in_point_ticks) as u64),
            lead_out_point_ms: passage.lead_out_point_ticks.map(|t| wkmp_common::timing::ticks_to_ms(t) as u64),
            fade_in_point_ms: Some(wkmp_common::timing::ticks_to_ms(passage.fade_in_point_ticks) as u64),
            fade_out_point_ms: passage.fade_out_point_ticks.map(|t| wkmp_common::timing::ticks_to_ms(t) as u64),
            fade_in_curve: Some(passage.fade_in_curve.to_db_string().to_string()),
            fade_out_curve: Some(passage.fade_out_curve.to_db_string().to_string()),
        };

        self.queue.write().await.enqueue(entry);

        // Update audio_expected flag for ring buffer underrun classification
        self.update_audio_expected_flag().await;

        Ok(queue_entry_id)
    }

    /// Calculate crossfade start position in milliseconds
    ///
    /// [ISSUE-7] Extracted helper method to reduce complexity
    /// [XFD-IMPL-070] Crossfade timing calculation
    /// [SRC-TICK-020] Uses tick-based passage timing
    fn calculate_crossfade_start_ms(&self, passage: &PassageWithTiming) -> u64 {
        // Convert fade_out_point from ticks to ms
        if let Some(fade_out_ticks) = passage.fade_out_point_ticks {
            wkmp_common::timing::ticks_to_ms(fade_out_ticks) as u64
        } else {
            // If no explicit fade_out_point, calculate from end
            // Default: start crossfade 5 seconds before end
            if let Some(end_ticks) = passage.end_time_ticks {
                let end_ms = wkmp_common::timing::ticks_to_ms(end_ticks) as u64;
                if end_ms > 5000 {
                    end_ms - 5000
                } else {
                    end_ms
                }
            } else {
                0
            }
        }
    }

    /// Check if crossfade should be triggered
    ///
    /// [ISSUE-7] Extracted helper method to reduce complexity
    /// Returns true if all conditions are met for crossfade triggering
    async fn should_trigger_crossfade(
        &self,
        _current_queue_entry_id: Uuid,
        position_ms: u64,
        crossfade_start_ms: u64,
    ) -> bool {
        // Check position reached trigger point
        if position_ms < crossfade_start_ms {
            return false;
        }

        // Check we have a next passage in queue
        let queue = self.queue.read().await;
        let next = queue.next();

        match next {
            Some(next_entry) => {
                // Verify next buffer is ready
                self.buffer_manager.is_ready(next_entry.queue_entry_id).await
            }
            None => false,
        }
    }

    /// Try to trigger crossfade transition
    ///
    /// [ISSUE-7] Refactored from complex nested logic
    /// [SSD-MIX-040] Crossfade initiation
    /// [XFD-IMPL-070] Crossfade timing
    async fn try_trigger_crossfade(
        &self,
        current: &QueueEntry,
        current_passage: &PassageWithTiming,
        position_ms: u64,
        crossfade_start_ms: u64,
    ) -> Result<bool> {
        // Get next queue entry
        let queue = self.queue.read().await;
        let next = match queue.next() {
            Some(n) => n.clone(),
            None => return Ok(false),
        };
        drop(queue);

        // Get next buffer
        let next_buffer = match self.buffer_manager.get_buffer(next.queue_entry_id).await {
            Some(buf) => buf,
            None => {
                warn!("Next buffer marked ready but not found: {}", next.queue_entry_id);
                return Ok(false);
            }
        };

        // Get next passage timing
        let next_passage = match self.get_passage_timing(&next).await {
            Ok(p) => p,
            Err(_) => {
                warn!("Could not get timing for next passage {}", next.queue_entry_id);
                return Ok(false);
            }
        };

        info!(
            "Triggering crossfade: {} → {} at position {}ms",
            current.queue_entry_id, next.queue_entry_id, position_ms
        );

        // Calculate crossfade durations (in ticks)
        let fade_out_duration_ticks = if let Some(end_ticks) = current_passage.end_time_ticks {
            end_ticks.saturating_sub(wkmp_common::timing::ms_to_ticks(crossfade_start_ms as i64))
        } else {
            wkmp_common::timing::ms_to_ticks(5000) // Default 5-second crossfade
        };

        let fade_in_duration_ticks = next_passage
            .fade_in_point_ticks
            .saturating_sub(next_passage.start_time_ticks);

        // Convert ticks to samples for mixer
        let fade_out_duration_samples = wkmp_common::timing::ticks_to_samples(
            fade_out_duration_ticks,
            44100 // STANDARD_SAMPLE_RATE
        );

        let fade_in_duration_samples = wkmp_common::timing::ticks_to_samples(
            fade_in_duration_ticks,
            44100 // STANDARD_SAMPLE_RATE
        );

        // Start the crossfade!
        self.mixer
            .write()
            .await
            .start_crossfade(
                next_buffer,
                next.queue_entry_id,
                current_passage.fade_out_curve,
                fade_out_duration_samples,
                next_passage.fade_in_curve,
                fade_in_duration_samples,
            )
            .await?;

        info!("Crossfade started successfully");

        // **[SSE-UI-040]** Emit CrossfadeStarted event
        self.state.broadcast_event(wkmp_common::events::WkmpEvent::CrossfadeStarted {
            from_passage_id: current.passage_id.unwrap_or_else(|| Uuid::nil()),
            to_passage_id: next.passage_id.unwrap_or_else(|| Uuid::nil()),
            timestamp: chrono::Utc::now(),
        });

        // Mark next buffer as playing
        self.buffer_manager.mark_playing(next.queue_entry_id).await;

        // Emit PassageStarted event for next passage
        self.state.broadcast_event(wkmp_common::events::WkmpEvent::PassageStarted {
            passage_id: next.passage_id.unwrap_or_else(|| Uuid::nil()),
            timestamp: chrono::Utc::now(),
        });

        Ok(true)
    }

    /// Main playback loop
    ///
    /// [SSD-FLOW-010] Core orchestration logic
    async fn playback_loop(&self) -> Result<()> {
        info!("Playback loop started");
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
                debug!("Playback loop: State is {:?}, skipping", playback_state);
                continue; // Paused, skip processing
            }

            // Process queue
            self.process_queue().await?;
        }

        info!("Playback loop stopped");
        Ok(())
    }

    /// Process queue: trigger decodes for current/next/queued passages
    async fn process_queue(&self) -> Result<()> {
        // **[ASYNC-LOCK-001]** Clone queue entries immediately to avoid holding lock across awaits
        // Holding RwLock read guard across await points can cause deadlocks when other tasks
        // (like SSE handlers) try to acquire read locks while a write lock request is pending
        let (current_entry, next_entry, queued_entries) = {
            let queue = self.queue.read().await;
            (
                queue.current().cloned(),
                queue.next().cloned(),
                queue.queued().to_vec(),
            )
        }; // Lock is dropped here

        // Trigger decode for current passage if needed
        // **Fix for queue flooding:** Only request decode if buffer doesn't exist yet
        // Previously checked is_ready() which allowed duplicate requests while Decoding
        if let Some(current) = &current_entry {
            if !self.buffer_manager.is_managed(current.queue_entry_id).await {
                debug!("Requesting decode for current passage: {}", current.queue_entry_id);
                self.request_decode(current, DecodePriority::Immediate, true)
                    .await?;
            }
        }

        // Start mixer if current passage has minimum playback buffer
        // [SSD-MIX-030] Single passage playback initiation
        // [SSD-PBUF-028] Start playback with minimum buffer (3 seconds)
        if let Some(current) = &current_entry {
            debug!("process_queue: Found current passage: {}", current.queue_entry_id);

            // Check if buffer has minimum playback buffer available (3 seconds)
            // This enables instant play start - decode continues in background
            const MIN_PLAYBACK_BUFFER_MS: u64 = 3000;
            let buffer_has_minimum = self.buffer_manager
                .has_minimum_playback_buffer(current.queue_entry_id, MIN_PLAYBACK_BUFFER_MS)
                .await;
            debug!("process_queue: Minimum buffer for {}: {}", current.queue_entry_id, buffer_has_minimum);

            if buffer_has_minimum {
                // Check if mixer is currently idle
                let mixer = self.mixer.read().await;
                let mixer_idle = mixer.get_current_passage_id().is_none();
                debug!("process_queue: Mixer idle: {}", mixer_idle);

                if mixer_idle {
                    // Mixer is idle and buffer is ready - start playback!
                    drop(mixer); // Release read lock before acquiring write lock

                    // Get buffer from buffer manager
                    if let Some(buffer) = self.buffer_manager.get_buffer(current.queue_entry_id).await {
                        // Get passage timing information
                        let passage = self.get_passage_timing(current).await?;

                        // **[REV002]** Load song timeline for passage
                        if let Some(passage_id) = current.passage_id {
                            match crate::db::passage_songs::load_song_timeline(&self.db_pool, passage_id).await {
                                Ok(timeline) => {
                                    // Get initial song at position 0
                                    let initial_song_id = timeline.get_current_song(0);
                                    let timeline_len = timeline.len();
                                    let timeline_not_empty = !timeline.is_empty();

                                    // Store timeline for boundary detection
                                    *self.current_song_timeline.write().await = Some(timeline);

                                    // Emit initial CurrentSongChanged if passage starts within a song
                                    if initial_song_id.is_some() || timeline_not_empty {
                                        debug!(
                                            "Passage starts with song: {:?}",
                                            initial_song_id
                                        );

                                        self.state.broadcast_event(wkmp_common::events::WkmpEvent::CurrentSongChanged {
                                            passage_id,
                                            song_id: initial_song_id,
                                            position_ms: 0,
                                            timestamp: chrono::Utc::now(),
                                        });
                                    }

                                    info!("Loaded song timeline: {} entries", timeline_len);
                                }
                                Err(e) => {
                                    warn!(
                                        "Failed to load song timeline for passage {}: {} (continuing without song boundaries)",
                                        passage_id, e
                                    );
                                    // Continue playback without song boundary detection
                                    *self.current_song_timeline.write().await = None;
                                }
                            }
                        } else {
                            // Ephemeral passage - no song timeline
                            *self.current_song_timeline.write().await = None;
                        }

                        // Calculate fade-in duration from timing points (in ticks)
                        // fade_in_point_ticks is where fade-in completes, so duration = fade_in - start
                        let fade_in_duration_ticks = passage.fade_in_point_ticks.saturating_sub(passage.start_time_ticks);

                        // Convert ticks to samples for mixer
                        let fade_in_duration_samples = wkmp_common::timing::ticks_to_samples(
                            fade_in_duration_ticks,
                            44100 // STANDARD_SAMPLE_RATE
                        );

                        // Determine fade-in curve (default to Exponential if not specified)
                        let fade_in_curve = if let Some(curve_str) = current.fade_in_curve.as_ref() {
                            wkmp_common::FadeCurve::from_str(curve_str)
                                .unwrap_or(wkmp_common::FadeCurve::Exponential)
                        } else {
                            wkmp_common::FadeCurve::Exponential
                        };

                        info!(
                            "Starting playback of passage {} (queue_entry: {}) with fade-in: {} samples ({} ticks)",
                            current.passage_id.unwrap_or_else(|| Uuid::nil()),
                            current.queue_entry_id,
                            fade_in_duration_samples,
                            fade_in_duration_ticks
                        );

                        // Start mixer
                        self.mixer.write().await.start_passage(
                            buffer,
                            current.queue_entry_id,
                            Some(fade_in_curve),
                            fade_in_duration_samples,
                        ).await;

                        // Mark buffer as playing
                        self.buffer_manager.mark_playing(current.queue_entry_id).await;

                        // Update position tracking
                        // [ISSUE-8] Use internal RwLock for queue_entry_id
                        *self.position.queue_entry_id.write().await = Some(current.queue_entry_id);

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
        // [ISSUE-7] Refactored to use helper methods for reduced complexity
        let mixer = self.mixer.read().await;
        let is_crossfading = mixer.is_crossfading();
        let current_passage_id = mixer.get_current_passage_id();
        let mixer_position_frames = mixer.get_position();
        drop(mixer);

        // Only trigger crossfade if mixer is playing and not already crossfading
        if let Some(current_id) = current_passage_id {
            if !is_crossfading {
                // Verify current matches queue
                if let Some(current) = &current_entry {
                    if current.queue_entry_id == current_id {
                        // Get passage timing and buffer to calculate position
                        if let Ok(passage) = self.get_passage_timing(current).await {
                            if let Some(buffer_ref) = self.buffer_manager.get_buffer(current.queue_entry_id).await {
                                // Convert frame position to milliseconds
                                let buffer = buffer_ref.read().await;
                                let position_ms = (mixer_position_frames as u64 * 1000) / buffer.sample_rate as u64;
                                drop(buffer);

                                // Calculate when crossfade should start
                                let crossfade_start_ms = self.calculate_crossfade_start_ms(&passage);

                                // Check if conditions are met for crossfade
                                if self.should_trigger_crossfade(current_id, position_ms, crossfade_start_ms).await {
                                    // Attempt to trigger crossfade
                                    if let Err(e) = self.try_trigger_crossfade(current, &passage, position_ms, crossfade_start_ms).await {
                                        error!("Failed to trigger crossfade: {}", e);
                                    }
                                } else if position_ms >= crossfade_start_ms {
                                    // Position reached but buffer not ready
                                    debug!("Next passage buffer not ready yet at crossfade point (position: {}ms)", position_ms);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Trigger decode for next passage if needed
        // [SSD-FBUF-011] Full decode for currently playing OR next-to-be-played
        // [SSD-PBUF-013] Facilitate instant skip to queued passages
        // This MUST happen before completion check to ensure prefetch while playing
        // **Fix for queue flooding:** Only request if not already managed
        if let Some(next) = &next_entry {
            if !self.buffer_manager.is_managed(next.queue_entry_id).await {
                debug!("Requesting decode for next passage: {}", next.queue_entry_id);
                self.request_decode(next, DecodePriority::Next, true)
                    .await?;
            }
        }

        // Trigger decode for queued passages (first 3)
        // [SSD-PBUF-010] Partial buffer decode for queued passages
        // **CRITICAL FIX:** Always request FULL decode (full=true) to prevent
        // third-file bug where passages decoded as "queued" with full=false
        // never get upgraded to full decode when promoted to "next".
        // Trade-off: Slightly higher memory usage, but guarantees correct playback.
        for queued in queued_entries.iter().take(3) {
            if !self.buffer_manager.is_managed(queued.queue_entry_id).await {
                debug!("Requesting decode for queued passage: {}", queued.queue_entry_id);
                self.request_decode(queued, DecodePriority::Prefetch, true)
                    .await?;
            }
        }

        // **[XFD-COMP-010]** Check for crossfade completion BEFORE normal completion
        // When a crossfade completes, the outgoing passage has finished fading out
        // and the incoming passage continues playing as the new current passage.
        // We must advance the queue WITHOUT stopping the mixer to avoid interrupting
        // the already-playing incoming passage.
        //
        // **[XFD-COMP-020]** Critical: Do NOT call mixer.stop() in this path!
        let crossfade_completed_id = {
            let mut mixer = self.mixer.write().await;
            mixer.take_crossfade_completed()
        };

        if let Some(completed_id) = crossfade_completed_id {
            debug!("Processing crossfade completion for passage {}", completed_id);

            // Verify this is the current passage in queue
            let queue = self.queue.read().await;
            if let Some(current) = queue.current() {
                if current.queue_entry_id == completed_id {
                    let passage_id_opt = current.passage_id;
                    drop(queue);

                    info!("Passage {} completed (via crossfade)", completed_id);

                    // **[Event-PassageCompleted]** Emit completion event for OUTGOING passage
                    self.state.broadcast_event(wkmp_common::events::WkmpEvent::PassageCompleted {
                        passage_id: passage_id_opt.unwrap_or_else(|| Uuid::nil()),
                        completed: true, // Crossfade completed naturally
                        timestamp: chrono::Utc::now(),
                    });

                    // **[XFD-COMP-020]** Advance queue WITHOUT stopping mixer
                    // The incoming passage is already playing as current in the mixer
                    let mut queue_write = self.queue.write().await;
                    queue_write.advance();
                    drop(queue_write);

                    // **[SSE-UI-020]** Emit QueueStateUpdate after queue advance
                    let queue_entries = self.get_queue_entries().await;
                    let queue_info: Vec<wkmp_common::events::QueueEntryInfo> = queue_entries.into_iter()
                        .map(|e| wkmp_common::events::QueueEntryInfo {
                            queue_entry_id: e.queue_entry_id,
                            passage_id: e.passage_id,
                            file_path: e.file_path.to_string_lossy().to_string(),
                        })
                        .collect();
                    self.state.broadcast_event(wkmp_common::events::WkmpEvent::QueueStateUpdate {
                        timestamp: chrono::Utc::now(),
                        queue: queue_info,
                    });

                    // Remove completed passage from database to keep in sync
                    if let Err(e) = crate::db::queue::remove_from_queue(&self.db_pool, completed_id).await {
                        warn!("Failed to remove completed passage from database: {}", e);
                    } else {
                        info!("Queue advanced (crossfade) and synced to database (removed {})", completed_id);
                    }

                    // Update audio_expected flag
                    self.update_audio_expected_flag().await;

                    // **[XFD-COMP-020]** Clean up outgoing buffer (incoming continues playing)
                    if let Some(p_id) = passage_id_opt {
                        self.buffer_manager.remove(p_id).await;
                    }

                    // **[XFD-COMP-020]** CRITICAL: DO NOT stop mixer!
                    // The incoming passage is already playing seamlessly in the mixer.
                    // Stopping would interrupt playback and cause the bug we're fixing.
                    debug!("Crossfade completion handled - mixer continues playing incoming passage");

                    return Ok(()); // Exit early - do not fall through to normal completion
                } else {
                    warn!(
                        "Crossfade completed ID {} doesn't match queue current {}",
                        completed_id, current.queue_entry_id
                    );
                }
            }
        }

        // Check for passage completion (normal case - no crossfade)
        // [SSD-MIX-060] Passage completion detection
        // [SSD-ENG-020] Queue processing and advancement
        let mixer = self.mixer.read().await;
        let is_finished = mixer.is_current_finished().await;
        drop(mixer);

        if is_finished {
            // Get current queue entry for event emission and logging
            // **[ASYNC-LOCK-001]** Acquire lock only for the data we need, then drop immediately
            let (current_qe_id, current_pid) = {
                let queue = self.queue.read().await;
                if let Some(current) = queue.current() {
                    (Some(current.queue_entry_id), current.passage_id)
                } else {
                    (None, None)
                }
            };

            if let Some(queue_entry_id) = current_qe_id {
                info!("Passage {} completed", queue_entry_id);

                // Emit PassageCompleted event
                // [Event-PassageCompleted] Passage playback finished
                self.state.broadcast_event(wkmp_common::events::WkmpEvent::PassageCompleted {
                    passage_id: current_pid.unwrap_or_else(|| Uuid::nil()),
                    completed: true, // true = finished naturally, false = skipped/interrupted
                    timestamp: chrono::Utc::now(),
                });

                // Mark buffer as exhausted
                self.buffer_manager.mark_exhausted(queue_entry_id).await;

                info!("Marked buffer {} as exhausted", queue_entry_id);

                // Capture passage_id for buffer cleanup later (after queue advance)
                let passage_id_for_cleanup = current_pid;

                // [ISSUE-6] Sync queue advance to database
                // Store ID of completed passage before advancing
                let completed_queue_entry_id = {
                    let queue_read = self.queue.read().await;
                    queue_read.current().map(|c| c.queue_entry_id)
                };

                // Advance queue to next passage (in-memory)
                // This removes the completed passage and moves next → current
                let mut queue_write = self.queue.write().await;
                queue_write.advance();
                drop(queue_write);

                // **[SSE-UI-020]** Emit QueueStateUpdate after queue advance
                let queue_entries = self.get_queue_entries().await;
                let queue_info: Vec<wkmp_common::events::QueueEntryInfo> = queue_entries.into_iter()
                    .map(|e| wkmp_common::events::QueueEntryInfo {
                        queue_entry_id: e.queue_entry_id,
                        passage_id: e.passage_id,
                        file_path: e.file_path.to_string_lossy().to_string(),
                    })
                    .collect();
                self.state.broadcast_event(wkmp_common::events::WkmpEvent::QueueStateUpdate {
                    timestamp: chrono::Utc::now(),
                    queue: queue_info,
                });

                // Remove completed passage from database to keep in sync
                if let Some(completed_id) = completed_queue_entry_id {
                    if let Err(e) = crate::db::queue::remove_from_queue(&self.db_pool, completed_id).await {
                        // Log error but don't fail - queue already advanced in memory
                        warn!("Failed to remove completed passage from database queue: {} (continuing anyway)", e);
                    } else {
                        info!("Queue advanced and synced to database (removed {})", completed_id);
                    }
                } else {
                    info!("Queue advanced to next passage");
                }

                // Update audio_expected flag for ring buffer underrun classification
                // This ensures TRACE logging when queue becomes empty after passage finishes
                self.update_audio_expected_flag().await;

                // Clean up the exhausted buffer (free memory)
                if let Some(p_id) = passage_id_for_cleanup {
                    self.buffer_manager.remove(p_id).await;
                }

                // Stop the mixer (will be restarted on next iteration with new passage)
                self.mixer.write().await.stop();

                debug!("Mixer stopped after passage completion");

                // Return early - next iteration will start the new current passage
                return Ok(());
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

        // Submit to decoder pool (now async - registers buffer immediately)
        if let Some(decoder_pool) = self.decoder_pool.read().await.as_ref() {
            decoder_pool.submit(
                entry.queue_entry_id,
                passage,
                priority,
                full_decode,
            ).await?;
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

    /// Position event handler (replaces position_tracking_loop)
    ///
    /// **[REV002]** Event-driven position tracking
    /// **[ARCH-SNGC-030]** Event-driven position tracking architecture
    ///
    /// Receives PositionUpdate events from mixer and:
    /// 1. Checks song boundaries and emits CurrentSongChanged events
    /// 2. Emits PlaybackProgress events every N seconds
    /// 3. Updates shared state with current position
    async fn position_event_handler(&self) {
        // Take ownership of receiver (only one handler allowed)
        let mut rx = match self.position_event_rx.write().await.take() {
            Some(rx) => rx,
            None => {
                error!("Position event receiver already taken!");
                return;
            }
        };

        // Load playback_progress_interval_ms from database
        let progress_interval_ms = crate::db::settings::load_progress_interval(&self.db_pool)
            .await
            .unwrap_or(5000); // Default: 5 seconds

        let mut last_progress_position_ms = 0u64;

        info!("Position event handler started (progress interval: {}ms)", progress_interval_ms);

        loop {
            // Wait for position event
            match rx.recv().await {
                Some(PlaybackEvent::PositionUpdate { queue_entry_id, position_ms }) => {
                    // [1] Check song boundary
                    let mut timeline = self.current_song_timeline.write().await;
                    if let Some(timeline) = timeline.as_mut() {
                        let (crossed, new_song_id) = timeline.check_boundary(position_ms);

                        if crossed {
                            // Get passage_id for event
                            let queue = self.queue.read().await;
                            let passage_id = queue.current()
                                .and_then(|e| e.passage_id)
                                .unwrap_or_else(|| uuid::Uuid::nil());
                            drop(queue);

                            // Emit CurrentSongChanged event
                            info!(
                                "Song boundary crossed: new_song={:?}, position={}ms",
                                new_song_id, position_ms
                            );

                            self.state.broadcast_event(wkmp_common::events::WkmpEvent::CurrentSongChanged {
                                passage_id,
                                song_id: new_song_id,
                                position_ms,
                                timestamp: chrono::Utc::now(),
                            });
                        }
                    }
                    drop(timeline);

                    // [2] Check if PlaybackProgress interval elapsed
                    if position_ms >= last_progress_position_ms + progress_interval_ms {
                        last_progress_position_ms = position_ms;

                        // Get duration from buffer
                        if let Some(buffer_ref) = self.buffer_manager.get_buffer(queue_entry_id).await {
                            let buffer = buffer_ref.read().await;
                            let duration_ms = buffer.duration_ms();

                            // Get passage_id for event
                            let queue = self.queue.read().await;
                            let passage_id = queue.current()
                                .and_then(|e| e.passage_id)
                                .unwrap_or_else(|| uuid::Uuid::nil());
                            drop(queue);

                            debug!(
                                "PlaybackProgress: position={}ms, duration={}ms",
                                position_ms, duration_ms
                            );

                            self.state.broadcast_event(wkmp_common::events::WkmpEvent::PlaybackProgress {
                                passage_id,
                                position_ms,
                                duration_ms,
                                timestamp: chrono::Utc::now(),
                            });
                        }
                    }

                    // [3] Update shared state
                    let queue = self.queue.read().await;
                    if let Some(current) = queue.current() {
                        if current.queue_entry_id == queue_entry_id {
                            if let Some(buffer_ref) = self.buffer_manager.get_buffer(queue_entry_id).await {
                                let buffer = buffer_ref.read().await;
                                let duration_ms = buffer.duration_ms();

                                let current_passage = CurrentPassage {
                                    queue_entry_id: current.queue_entry_id,
                                    passage_id: current.passage_id,
                                    position_ms,
                                    duration_ms,
                                };

                                self.state.set_current_passage(Some(current_passage)).await;
                            }
                        }
                    } else {
                        // No current passage
                        self.state.set_current_passage(None).await;
                    }
                }

                Some(PlaybackEvent::StateChanged { .. }) => {
                    // Future: Handle state change events
                }

                None => {
                    // Channel closed, handler should exit
                    info!("Position event handler stopping (channel closed)");
                    break;
                }
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
            position: self.position.clone_handles(), // [ISSUE-8] Clone inner Arcs
            running: Arc::clone(&self.running),
            position_event_tx: self.position_event_tx.clone(), // **[REV002]** Clone sender
            volume: Arc::clone(&self.volume), // [ARCH-VOL-020] Clone volume Arc
            position_event_rx: Arc::clone(&self.position_event_rx), // **[REV002]** Clone receiver
            current_song_timeline: Arc::clone(&self.current_song_timeline), // **[REV002]** Clone timeline
            audio_expected: Arc::clone(&self.audio_expected), // Clone audio_expected flag for ring buffer
            buffer_event_rx: Arc::clone(&self.buffer_event_rx), // **[PERF-POLL-010]** Clone buffer event receiver
        }
    }

    /// Buffer event handler for instant mixer start
    ///
    /// **[PERF-POLL-010]** Event-driven buffer readiness
    ///
    /// Listens for ReadyForStart events from BufferManager and immediately
    /// starts the mixer when a buffer reaches the minimum threshold.
    /// This replaces the polling loop that checked has_minimum_playback_buffer().
    async fn buffer_event_handler(&self) {
        use std::time::Instant;

        // Take ownership of receiver (only one handler allowed)
        let mut rx = match self.buffer_event_rx.write().await.take() {
            Some(rx) => rx,
            None => {
                error!("Buffer event receiver already taken!");
                return;
            }
        };

        info!("Buffer event handler started (event-driven mixer start)");

        loop {
            // Wait for buffer event
            match rx.recv().await {
                Some(BufferEvent::ReadyForStart { queue_entry_id, buffer_duration_ms, .. }) => {
                    let start_time = Instant::now();

                    info!(
                        "🚀 Buffer ready event received: {} ({}ms available)",
                        queue_entry_id, buffer_duration_ms
                    );

                    // Check if this is the current passage in queue
                    let queue = self.queue.read().await;
                    let is_current = queue.current().map(|c| c.queue_entry_id) == Some(queue_entry_id);

                    if !is_current {
                        debug!(
                            "Buffer ready for {} but not current passage, ignoring",
                            queue_entry_id
                        );
                        continue;
                    }

                    let current = match queue.current() {
                        Some(c) => c.clone(),
                        None => continue,
                    };
                    drop(queue);

                    // Check if mixer is already playing
                    let mixer = self.mixer.read().await;
                    let mixer_idle = mixer.get_current_passage_id().is_none();
                    drop(mixer);

                    if !mixer_idle {
                        debug!("Mixer already playing, ignoring ready event for {}", queue_entry_id);
                        continue;
                    }

                    // Get buffer from buffer manager
                    let buffer = match self.buffer_manager.get_buffer(queue_entry_id).await {
                        Some(buf) => buf,
                        None => {
                            warn!("Buffer ready event but buffer not found: {}", queue_entry_id);
                            continue;
                        }
                    };

                    // Get passage timing information
                    let passage = match self.get_passage_timing(&current).await {
                        Ok(p) => p,
                        Err(e) => {
                            error!("Failed to get passage timing: {}", e);
                            continue;
                        }
                    };

                    // **[REV002]** Load song timeline for passage
                    if let Some(passage_id) = current.passage_id {
                        match crate::db::passage_songs::load_song_timeline(&self.db_pool, passage_id).await {
                            Ok(timeline) => {
                                let initial_song_id = timeline.get_current_song(0);
                                *self.current_song_timeline.write().await = Some(timeline);

                                if initial_song_id.is_some() {
                                    self.state.broadcast_event(wkmp_common::events::WkmpEvent::CurrentSongChanged {
                                        passage_id,
                                        song_id: initial_song_id,
                                        position_ms: 0,
                                        timestamp: chrono::Utc::now(),
                                    });
                                }
                            }
                            Err(e) => {
                                warn!("Failed to load song timeline: {}", e);
                                *self.current_song_timeline.write().await = None;
                            }
                        }
                    }

                    // Calculate fade-in duration (in ticks)
                    let fade_in_duration_ticks = passage.fade_in_point_ticks.saturating_sub(passage.start_time_ticks);

                    // Convert ticks to samples for mixer
                    let fade_in_duration_samples = wkmp_common::timing::ticks_to_samples(
                        fade_in_duration_ticks,
                        44100 // STANDARD_SAMPLE_RATE
                    );

                    // Determine fade-in curve
                    let fade_in_curve = current.fade_in_curve.as_ref()
                        .and_then(|s| wkmp_common::FadeCurve::from_str(s))
                        .unwrap_or(wkmp_common::FadeCurve::Exponential);

                    info!(
                        "⚡ Starting playback instantly (buffer ready): passage={}, fade_in={} samples ({} ticks)",
                        current.passage_id.unwrap_or_else(|| uuid::Uuid::nil()),
                        fade_in_duration_samples,
                        fade_in_duration_ticks
                    );

                    // Start mixer immediately
                    self.mixer.write().await.start_passage(
                        buffer,
                        queue_entry_id,
                        Some(fade_in_curve),
                        fade_in_duration_samples,
                    ).await;

                    // Mark buffer as playing
                    self.buffer_manager.mark_playing(queue_entry_id).await;

                    // Update position tracking
                    *self.position.queue_entry_id.write().await = Some(queue_entry_id);

                    // Emit PassageStarted event
                    self.state.broadcast_event(wkmp_common::events::WkmpEvent::PassageStarted {
                        passage_id: current.passage_id.unwrap_or_else(|| uuid::Uuid::nil()),
                        timestamp: chrono::Utc::now(),
                    });

                    let elapsed = start_time.elapsed();
                    info!(
                        "✅ Mixer started in {:.2}ms (event-driven instant start)",
                        elapsed.as_secs_f64() * 1000.0
                    );
                }

                Some(BufferEvent::StateChanged { .. }) |
                Some(BufferEvent::Exhausted { .. }) |
                Some(BufferEvent::Finished { .. }) => {
                    // Future: Handle other buffer events
                    // For now, only ReadyForStart is used for instant mixer start
                }

                None => {
                    info!("Buffer event handler stopping (channel closed)");
                    break;
                }
            }
        }
    }

    /// Background task: Emit BufferChainStatus events every 1 second when data changes
    ///
    /// Tracks decoder-resampler-fade-buffer chain states for developer UI monitoring.
    /// Only emits when data changes to reduce SSE traffic.
    async fn buffer_chain_status_emitter(&self) {
        use tokio::time::interval;
        use std::time::Duration;

        info!("BufferChainStatus emitter started");

        let mut tick = interval(Duration::from_secs(1));
        let mut last_chains: Option<Vec<wkmp_common::events::BufferChainInfo>> = None;

        loop {
            tick.tick().await;

            // Check if engine is still running
            if !*self.running.read().await {
                info!("BufferChainStatus emitter stopping");
                break;
            }

            // Get current buffer chain status
            let current_chains = self.get_buffer_chains().await;

            // Only emit if data has changed
            let should_emit = match &last_chains {
                None => true, // First emission
                Some(prev) => prev != &current_chains, // Data changed
            };

            if should_emit {
                // Emit BufferChainStatus event
                self.state.broadcast_event(wkmp_common::events::WkmpEvent::BufferChainStatus {
                    timestamp: chrono::Utc::now(),
                    chains: current_chains.clone(),
                });

                last_chains = Some(current_chains);
            }
        }
    }

    /// Background task: Emit PlaybackPosition events every 1 second
    ///
    /// **[SSE-UI-030]** Playback Position Updates
    ///
    /// This background task runs continuously and emits PlaybackPosition
    /// events to SSE clients every 1 second during active playback.
    async fn playback_position_emitter(&self) {
        use tokio::time::interval;
        use std::time::Duration;

        info!("PlaybackPosition emitter started");

        let mut tick = interval(Duration::from_secs(1));

        loop {
            tick.tick().await;

            // Check if engine is still running
            if !*self.running.read().await {
                info!("PlaybackPosition emitter stopping");
                break;
            }

            // Only emit if currently playing (not paused/stopped)
            let playback_state = self.state.get_playback_state().await;
            if playback_state != crate::state::PlaybackState::Playing {
                continue;
            }

            // Get current passage from SharedState
            if let Some(current) = self.state.get_current_passage().await {
                // Emit PlaybackPosition event (use queue_entry_id as fallback for ephemeral passages)
                let passage_id = current.passage_id.unwrap_or(current.queue_entry_id);

                self.state.broadcast_event(wkmp_common::events::WkmpEvent::PlaybackPosition {
                    timestamp: chrono::Utc::now(),
                    passage_id,
                    position_ms: current.position_ms,
                    duration_ms: current.duration_ms,
                    playing: true,
                });
            }
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

        // Create settings table for pause/resume configuration tests
        sqlx::query(
            r#"
            CREATE TABLE settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
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

        // Create temporary files for testing
        // [ISSUE-4] File existence validation requires real files
        let temp_dir = std::env::temp_dir();
        let passage1 = temp_dir.join("test_song1.mp3");
        let passage2 = temp_dir.join("test_song2.mp3");
        let passage3 = temp_dir.join("test_song3.mp3");

        // Create empty files
        std::fs::write(&passage1, b"").unwrap();
        std::fs::write(&passage2, b"").unwrap();
        std::fs::write(&passage3, b"").unwrap();

        // Enqueue 3 test passages
        engine.enqueue_file(passage1.clone()).await.unwrap();
        engine.enqueue_file(passage2.clone()).await.unwrap();
        engine.enqueue_file(passage3.clone()).await.unwrap();

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

        // Clean up temporary files
        let _ = std::fs::remove_file(&passage1);
        let _ = std::fs::remove_file(&passage2);
        let _ = std::fs::remove_file(&passage3);
    }

    #[tokio::test]
    async fn test_skip_empty_queue() {
        let db = create_test_db().await;
        let state = Arc::new(SharedState::new());

        let engine = PlaybackEngine::new(db, state.clone()).await.unwrap();

        // [ISSUE-13] Try to skip with empty queue (should return error)
        let result = engine.skip_next().await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No passage to skip"));

        // Queue should still be empty
        {
            let queue = engine.queue.read().await;
            assert_eq!(queue.len(), 0);
        }
    }

    #[tokio::test]
    async fn test_pause_integration() {
        // [XFD-PAUS-010] Verify engine.pause() integrates with mixer and state
        let db = create_test_db().await;
        let state = Arc::new(SharedState::new());

        let engine = PlaybackEngine::new(db.clone(), state.clone()).await.unwrap();

        // Start in Playing state
        engine.play().await.unwrap();
        assert_eq!(state.get_playback_state().await, PlaybackState::Playing);

        // Pause
        engine.pause().await.unwrap();

        // Verify state transitioned to Paused
        assert_eq!(state.get_playback_state().await, PlaybackState::Paused);

        // Verify pause state persists in mixer (indirectly - mixer should output silence)
        // Note: Cannot directly verify mixer.pause() was called due to encapsulation,
        // but the state change verifies integration path was executed
    }

    #[tokio::test]
    async fn test_resume_from_pause_with_custom_settings() {
        // [XFD-PAUS-020] Verify engine.play() loads resume settings from database
        let db = create_test_db().await;
        let state = Arc::new(SharedState::new());

        // Set custom resume fade-in settings in database
        crate::db::settings::set_setting(&db, "resume_from_pause_fade_in_duration", 1000u64)
            .await
            .unwrap();
        crate::db::settings::set_setting(&db, "resume_from_pause_fade_in_curve", "linear".to_string())
            .await
            .unwrap();

        let engine = PlaybackEngine::new(db.clone(), state.clone()).await.unwrap();

        // Start playing
        engine.play().await.unwrap();
        assert_eq!(state.get_playback_state().await, PlaybackState::Playing);

        // Pause
        engine.pause().await.unwrap();
        assert_eq!(state.get_playback_state().await, PlaybackState::Paused);

        // Resume (should load custom settings: 1000ms linear fade-in)
        engine.play().await.unwrap();
        assert_eq!(state.get_playback_state().await, PlaybackState::Playing);

        // Verify settings were loaded from database
        let duration = crate::db::settings::load_resume_fade_in_duration(&db)
            .await
            .unwrap();
        let curve = crate::db::settings::load_resume_fade_in_curve(&db)
            .await
            .unwrap();

        assert_eq!(duration, 1000, "Custom resume fade-in duration should be 1000ms");
        assert_eq!(curve, "linear", "Custom resume fade-in curve should be linear");

        // Note: Cannot directly verify mixer.resume() parameters due to encapsulation,
        // but state transitions and settings persistence verify integration path
    }

    /// **[ARCH-VOL-020]** Test that PlaybackEngine loads volume from database on startup
    #[tokio::test]
    async fn test_engine_loads_volume_from_database() {
        let db = create_test_db().await;

        // Set custom volume in database before creating engine
        crate::db::settings::set_volume(&db, 0.6).await.unwrap();

        let state = Arc::new(SharedState::new());
        let engine = PlaybackEngine::new(db, state).await.unwrap();

        // Verify volume was loaded from database
        let volume_arc = engine.get_volume_arc();
        let volume = *volume_arc.lock().unwrap();
        assert_eq!(volume, 0.6, "Engine should load volume 0.6 from database");
    }

    /// **[ARCH-VOL-020]** Test that get_volume_arc() returns the correct shared Arc
    #[tokio::test]
    async fn test_get_volume_arc() {
        let db = create_test_db().await;
        let state = Arc::new(SharedState::new());
        let engine = PlaybackEngine::new(db, state).await.unwrap();

        // Get volume Arc
        let volume_arc = engine.get_volume_arc();
        let original_value = *volume_arc.lock().unwrap();

        // Modify volume via Arc
        *volume_arc.lock().unwrap() = 0.8;

        // Get Arc again and verify it's the same instance
        let volume_arc2 = engine.get_volume_arc();
        let new_value = *volume_arc2.lock().unwrap();

        assert_eq!(new_value, 0.8, "Volume Arc should reflect updated value");
        assert_ne!(new_value, original_value, "Volume should have changed");
    }

    /// **[ARCH-VOL-020]** Test volume Arc synchronization between API and AudioOutput
    #[tokio::test]
    async fn test_volume_arc_synchronization() {
        let db = create_test_db().await;
        let state = Arc::new(SharedState::new());
        let engine = PlaybackEngine::new(db, state).await.unwrap();

        // Get shared volume Arc (simulating what API handler does)
        let volume_arc = engine.get_volume_arc();

        // Verify initial value
        let initial = *volume_arc.lock().unwrap();
        assert_eq!(initial, 0.5, "Initial volume should be 0.5 (default)");

        // Update volume via Arc (simulating API handler)
        *volume_arc.lock().unwrap() = 0.3;

        // Get Arc again (simulating what AudioOutput would see)
        let volume_arc2 = engine.get_volume_arc();
        let updated = *volume_arc2.lock().unwrap();

        assert_eq!(updated, 0.3, "Volume change should be visible through same Arc");

        // Verify both Arcs point to same underlying data
        *volume_arc.lock().unwrap() = 0.9;
        let final_value = *volume_arc2.lock().unwrap();
        assert_eq!(final_value, 0.9, "Changes via first Arc should be visible in second Arc");
    }
}
