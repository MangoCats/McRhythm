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
use crate::db::passages::{create_ephemeral_passage, get_passage_album_uuids, get_passage_with_timing, validate_passage_timing, PassageWithTiming};
use crate::error::{Error, Result};
use crate::playback::buffer_manager::BufferManager;
use crate::playback::buffer_events::BufferEvent;
use crate::playback::decoder_worker::DecoderWorker;
use crate::playback::events::PlaybackEvent;
// [SUB-INC-4B] SPEC016-compliant mixer integration
use crate::playback::mixer::{Mixer, MixerState, PositionMarker, MarkerEvent};
use crate::playback::queue_manager::{QueueEntry, QueueManager};
use crate::playback::ring_buffer::{AudioRingBuffer, AudioProducer};
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
use std::collections::{HashMap, BinaryHeap};
use std::cmp::Reverse;

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

    /// Decoder worker (single-threaded decoder using DecoderChain architecture)
    decoder_worker: Arc<DecoderWorker>,

    /// SPEC016-compliant mixer (batch mixing with event-driven markers)
    /// [SSD-MIX-010] Mixer component for audio frame generation
    /// [SUB-INC-4B] Replaced CrossfadeMixer with Mixer (SPEC016)
    mixer: Arc<RwLock<Mixer>>,

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

    /// Audio callback monitor (for gap/stutter detection)
    /// Created in audio thread, stored here for API access
    callback_monitor: Arc<RwLock<Option<Arc<crate::playback::callback_monitor::CallbackMonitor>>>>,

    /// Buffer event channel receiver for instant mixer start
    /// **[PERF-POLL-010]** Event-driven buffer readiness
    /// Receives ReadyForStart events when buffers reach minimum threshold
    buffer_event_rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<BufferEvent>>>>,

    /// Maximum number of decoder-resampler-fade-buffer chains
    /// **[DBD-PARAM-050]** Configurable maximum decode streams (default: 12)
    maximum_decode_streams: usize,

    /// Chain assignment tracking
    /// **[DBD-LIFECYCLE-040]** Maps queue_entry_id to chain_index for persistent association
    /// Implements requirement that chains remain associated with passages throughout lifecycle
    chain_assignments: Arc<RwLock<HashMap<Uuid, usize>>>,

    /// Available chain pool
    /// **[DBD-LIFECYCLE-030]** Min-heap for lowest-numbered chain allocation
    /// Chains are allocated in ascending order (0, 1, 2, ...) for visual consistency
    available_chains: Arc<RwLock<BinaryHeap<Reverse<usize>>>>,

    /// Buffer chain monitor update rate (milliseconds)
    /// **[SPEC020-MONITOR-120]** Client-controlled SSE emission rate
    /// Values: 100 (fast), 1000 (normal), or 0 (manual/disabled)
    buffer_monitor_rate_ms: Arc<RwLock<u64>>,

    /// Force immediate buffer chain status emission
    /// **[SPEC020-MONITOR-130]** Manual update trigger
    /// Set to true to force one immediate emission, then automatically reset
    buffer_monitor_update_now: Arc<AtomicBool>,

    /// Audio output buffer size in frames per callback
    /// **[DBD-PARAM-110]** Configurable audio buffer size (default: 512)
    audio_buffer_size: u32,
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
        let (initial_volume, min_buffer_threshold, interval_ms, grace_period_ms, mixer_config, maximum_decode_streams, resume_hysteresis, mixer_min_start_level, audio_buffer_size, buffer_capacity, buffer_headroom) = tokio::join!(
            crate::db::settings::get_volume(&db_pool),
            crate::db::settings::load_minimum_buffer_threshold(&db_pool),
            crate::db::settings::load_position_event_interval(&db_pool),
            crate::db::settings::load_ring_buffer_grace_period(&db_pool),
            crate::db::settings::load_mixer_thread_config(&db_pool),
            crate::db::settings::load_maximum_decode_streams(&db_pool),
            crate::db::settings::get_decoder_resume_hysteresis(&db_pool),
            crate::db::settings::load_mixer_min_start_level(&db_pool), // [DBD-PARAM-088]
            crate::db::settings::load_audio_buffer_size(&db_pool), // [DBD-PARAM-110]
            crate::db::settings::load_playout_ringbuffer_capacity(&db_pool), // [DBD-PARAM-070]
            crate::db::settings::load_playout_ringbuffer_headroom(&db_pool), // [DBD-PARAM-080]
        );
        let db_elapsed = db_start.elapsed();

        let initial_volume = initial_volume?;
        let min_buffer_threshold = min_buffer_threshold?;
        let interval_ms = interval_ms?;
        let _grace_period_ms = grace_period_ms?;  // Loaded in parallel for performance
        let _mixer_config = mixer_config?;  // Loaded in parallel for performance
        let maximum_decode_streams = maximum_decode_streams?;
        let resume_hysteresis = resume_hysteresis?;
        let mixer_min_start_level = mixer_min_start_level?; // [DBD-PARAM-088]
        let audio_buffer_size = audio_buffer_size?; // [DBD-PARAM-110]
        let buffer_capacity = buffer_capacity?; // [DBD-PARAM-070]
        let buffer_headroom = buffer_headroom?; // [DBD-PARAM-080]

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
        buffer_manager.set_resume_hysteresis(resume_hysteresis).await;
        // **[DBD-PARAM-070]** Configure buffer capacity from database
        buffer_manager.set_buffer_capacity(buffer_capacity).await;
        // **[DBD-PARAM-080]** Configure buffer headroom from database
        buffer_manager.set_buffer_headroom(buffer_headroom).await;

        // Create decoder worker
        // **[Phase 7]** Pass shared_state and db_pool for error handling
        let decoder_worker = Arc::new(DecoderWorker::new(
            Arc::clone(&buffer_manager),
            Arc::clone(&state),
            db_pool.clone(),
        ));

        // **[REV002]** Create position event channel
        let (position_event_tx, position_event_rx) = mpsc::unbounded_channel();

        // Create mixer
        // [SSD-MIX-010] SPEC016-compliant mixer for batch mixing
        // [SUB-INC-4B] Replaced CrossfadeMixer with Mixer (event-driven markers)
        // Note: master_volume initialized from settings, marker calculation happens in start_passage
        let master_volume = 1.0; // TODO: Load from settings
        let mixer = Mixer::new(master_volume);
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

        // **[DBD-LIFECYCLE-030]** Initialize available chains pool with all chain indices
        // Implements lowest-numbered allocation strategy (0, 1, 2, ...)
        let mut available_chains_heap = BinaryHeap::new();
        for i in 0..maximum_decode_streams {
            available_chains_heap.push(Reverse(i));
        }
        debug!(
            "Initialized {} decoder-buffer chains for assignment pool",
            maximum_decode_streams
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
            decoder_worker,
            mixer,
            position: PlaybackPosition::new(), // [ISSUE-8] Direct initialization, Arcs inside
            running: Arc::new(RwLock::new(false)),
            position_event_tx,
            volume, // [ARCH-VOL-020] Shared volume control
            position_event_rx: Arc::new(RwLock::new(Some(position_event_rx))),
            current_song_timeline: Arc::new(RwLock::new(None)),
            audio_expected: Arc::new(AtomicBool::new(false)), // Initially no audio expected
            callback_monitor: Arc::new(RwLock::new(None)), // Created later in audio thread
            buffer_event_rx: Arc::new(RwLock::new(Some(buffer_event_rx))), // [PERF-POLL-010] Buffer event channel
            maximum_decode_streams, // [DBD-PARAM-050] Configurable decode stream limit
            chain_assignments: Arc::new(RwLock::new(HashMap::new())), // [DBD-LIFECYCLE-040] Track passage→chain mapping
            available_chains: Arc::new(RwLock::new(available_chains_heap)), // [DBD-LIFECYCLE-030] Min-heap for lowest-first allocation
            buffer_monitor_rate_ms: Arc::new(RwLock::new(1000)), // [SPEC020-MONITOR-120] Default 1000ms update rate
            buffer_monitor_update_now: Arc::new(AtomicBool::new(false)), // [SPEC020-MONITOR-130] Manual update trigger
            audio_buffer_size, // [DBD-PARAM-110] Configurable audio buffer size
        })
    }

    /// Assign chains to all queue entries loaded from database
    ///
    /// **[DBD-LIFECYCLE-060]** Post-load chain assignment for database restore path
    ///
    /// This method should be called after engine creation to assign chains to
    /// passages that were loaded from database during initialization. Ensures
    /// uniform handling of chain assignment regardless of enqueue source.
    pub async fn assign_chains_to_loaded_queue(&self) {
        debug!("🔍 assign_chains_to_loaded_queue: START");
        let queue = self.queue.read().await;
        debug!("🔍 assign_chains_to_loaded_queue: Acquired queue read lock");

        // Collect all queue entry IDs
        let mut queue_entry_ids = Vec::new();
        if let Some(current) = queue.current() {
            debug!("🔍 assign_chains_to_loaded_queue: Found current entry: {}", current.queue_entry_id);
            queue_entry_ids.push(current.queue_entry_id);
        }
        if let Some(next) = queue.next() {
            debug!("🔍 assign_chains_to_loaded_queue: Found next entry: {}", next.queue_entry_id);
            queue_entry_ids.push(next.queue_entry_id);
        }
        for entry in queue.queued() {
            debug!("🔍 assign_chains_to_loaded_queue: Found queued entry: {}", entry.queue_entry_id);
            queue_entry_ids.push(entry.queue_entry_id);
        }
        drop(queue);
        debug!("🔍 assign_chains_to_loaded_queue: Dropped queue lock");

        // Save count before moving vector
        let count = queue_entry_ids.len();
        debug!("🔍 assign_chains_to_loaded_queue: Will assign chains to {} entries", count);

        // Assign chains to each entry
        for (idx, queue_entry_id) in queue_entry_ids.iter().enumerate() {
            debug!("🔍 assign_chains_to_loaded_queue: Processing entry {}/{}: {}", idx + 1, count, queue_entry_id);
            self.assign_chain(*queue_entry_id).await;
            debug!("🔍 assign_chains_to_loaded_queue: Completed entry {}/{}", idx + 1, count);
        }

        info!("Assigned chains to {} loaded queue entries", count);
        debug!("🔍 assign_chains_to_loaded_queue: DONE");
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

        // Start decoder worker
        Arc::clone(&self.decoder_worker).start();
        info!("Decoder worker started");

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

        // Start BufferChainStatus emission task with client-controlled rate
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
        // [SUB-INC-4B] Now uses batch mixing with event-driven markers
        let mixer_clone = Arc::clone(&self.mixer);
        let running_clone = Arc::clone(&self.running);
        let audio_expected_clone = Arc::clone(&self.audio_expected);
        let check_interval_us = mixer_config.check_interval_us;
        let batch_size_low = mixer_config.batch_size_low;
        let batch_size_optimal = mixer_config.batch_size_optimal;
        // [SUB-INC-4B] Clone additional variables for batch mixing
        let buffer_manager_clone = Arc::clone(&self.buffer_manager);
        let position_event_tx_clone = self.position_event_tx.clone();
        tokio::spawn(async move {
            info!("Mixer thread started");
            let mut check_interval = interval(Duration::from_micros(check_interval_us));

            // [SUB-INC-4B] Track crossfade state and current passages
            let mut is_crossfading = false;
            let mut current_passage_id: Option<Uuid> = None;
            let mut next_passage_id: Option<Uuid> = None;

            // [SUB-INC-4B] Fixed batch size for SPEC016 mixer (512 frames ~= 11ms @ 44.1kHz)
            const BATCH_SIZE_FRAMES: usize = 512;

            // [SUB-INC-4B] Helper function for batch mixing and ring buffer push
            async fn mix_and_push_batch(
                mixer: &Arc<RwLock<Mixer>>,
                buffer_manager: &Arc<BufferManager>,
                event_tx: &mpsc::UnboundedSender<PlaybackEvent>,
                producer: &mut AudioProducer,
                is_crossfading: &mut bool,
                current_passage_id: &mut Option<Uuid>,
                _next_passage_id: &mut Option<Uuid>,
                frames_to_mix: usize,
            ) {
                // Allocate output buffer (stereo: 2 samples per frame)
                let mut output = vec![0.0f32; frames_to_mix * 2];

                let mut mixer_guard = mixer.write().await;

                // Update current passage ID if changed
                *current_passage_id = mixer_guard.get_current_passage_id();

                // If no passage playing, fill with silence
                let Some(passage_id) = *current_passage_id else {
                    // No passage - push silence
                    for _ in 0..frames_to_mix {
                        if !producer.push(AudioFrame::zero()) {
                            break;
                        }
                    }
                    return;
                };

                // Mix batch
                let events = if *is_crossfading {
                    // TODO: Crossfade mixing (Phase 3)
                    // For now, fall back to single passage
                    warn!("Crossfade not yet implemented in batch mixer");
                    mixer_guard.mix_single(buffer_manager, passage_id, &mut output)
                        .await
                        .unwrap_or_else(|e| {
                            error!("Mix error: {}", e);
                            vec![]
                        })
                } else {
                    // Single passage mixing
                    mixer_guard.mix_single(buffer_manager, passage_id, &mut output)
                        .await
                        .unwrap_or_else(|e| {
                            error!("Mix error: {}", e);
                            vec![]
                        })
                };

                // Release mixer lock before pushing to ring buffer
                drop(mixer_guard);

                // Handle marker events
                handle_marker_events(events, event_tx, is_crossfading);

                // Push frames to ring buffer
                for i in (0..output.len()).step_by(2) {
                    let frame = AudioFrame {
                        left: output[i],
                        right: output[i + 1],
                    };
                    if !producer.push(frame) {
                        // Ring buffer full, stop pushing
                        break;
                    }
                }
            }

            // [SUB-INC-4B] Convert MarkerEvents to PlaybackEvents
            // TODO Phase 3: Map all MarkerEvent types to WkmpEvent (not PlaybackEvent)
            // For now, only handle marker tracking, defer event emission to Phase 3
            fn handle_marker_events(
                events: Vec<MarkerEvent>,
                _event_tx: &mpsc::UnboundedSender<PlaybackEvent>,
                is_crossfading: &mut bool,
            ) {
                for event in events {
                    match event {
                        MarkerEvent::PositionUpdate { position_ms: _ } => {
                            // TODO Phase 3: Emit position update with queue_entry_id
                            // event_tx.send(PlaybackEvent::PositionUpdate { queue_entry_id, position_ms }).ok();
                        }
                        MarkerEvent::StartCrossfade { next_passage_id: _ } => {
                            *is_crossfading = true;
                            // TODO Phase 3: Handle crossfade start
                        }
                        MarkerEvent::PassageComplete => {
                            *is_crossfading = false;
                            // TODO Phase 3: Handle passage complete
                        }
                        MarkerEvent::SongBoundary { new_song_id: _ } => {
                            // TODO Phase 3: Handle song boundary
                        }
                        MarkerEvent::EndOfFile { unreachable_markers } => {
                            warn!("EOF reached with {} unreachable markers", unreachable_markers.len());
                            // TODO Phase 3: Handle early EOF
                        }
                        MarkerEvent::EndOfFileBeforeLeadOut { planned_crossfade_tick, .. } => {
                            warn!("EOF before crossfade at tick {}", planned_crossfade_tick);
                            // TODO Phase 3: Emergency passage switch
                        }
                    }
                }
            }

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

                // **[BUGFIX]** Graduated filling strategy with underrun prevention
                // [SSD-MIX-020] Use configurable batch sizes from database
                //
                // Three-tier strategy to prevent ring buffer underruns:
                // - CRITICAL (< 25%): Fill aggressively without sleeping (underrun imminent!)
                // - LOW (25-50%): Fill moderately with minimal sleep
                // - OPTIMAL (50-75%): Top up conservatively
                // - HIGH (> 75%): Just sleep and wait

                let occupied = producer.occupied_len();
                let capacity = producer.capacity();
                let fill_percent = occupied as f32 / capacity as f32;

                let needs_filling = producer.needs_frames(); // < 50%
                let is_optimal = producer.is_fill_optimal(); // 50-75%
                let is_critical = fill_percent < 0.25; // < 25% = underrun risk!

                // [SUB-INC-4B] Replaced frame-by-frame with batch mixing
                if is_critical {
                    // **[BUGFIX]** Buffer CRITICALLY low (< 25%) - UNDERRUN IMMINENT!
                    // Fill aggressively WITHOUT sleeping to prevent gaps in audio
                    let frames_to_mix = BATCH_SIZE_FRAMES * 2;
                    mix_and_push_batch(
                        &mixer_clone,
                        &buffer_manager_clone,
                        &position_event_tx_clone,
                        &mut producer,
                        &mut is_crossfading,
                        &mut current_passage_id,
                        &mut next_passage_id,
                        frames_to_mix,
                    ).await;
                    // NO SLEEP - loop immediately to refill!

                } else if needs_filling {
                    // Buffer LOW (25-50%) - fill moderately
                    let frames_to_mix = BATCH_SIZE_FRAMES;
                    mix_and_push_batch(
                        &mixer_clone,
                        &buffer_manager_clone,
                        &position_event_tx_clone,
                        &mut producer,
                        &mut is_crossfading,
                        &mut current_passage_id,
                        &mut next_passage_id,
                        frames_to_mix,
                    ).await;

                    // Minimal sleep when buffer is low
                    check_interval.tick().await;

                } else if is_optimal {
                    // Buffer OPTIMAL (50-75%) - top up conservatively
                    check_interval.tick().await;

                    let frames_to_mix = BATCH_SIZE_FRAMES / 2;
                    mix_and_push_batch(
                        &mixer_clone,
                        &buffer_manager_clone,
                        &position_event_tx_clone,
                        &mut producer,
                        &mut is_crossfading,
                        &mut current_passage_id,
                        &mut next_passage_id,
                        frames_to_mix,
                    ).await;
                } else {
                    // Buffer HIGH (> 75%) - just yield and wait for consumption
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
        let state_clone = Arc::clone(&self.state); // For callback monitor event emission
        let callback_monitor_slot = Arc::clone(&self.callback_monitor);
        let audio_expected_clone = Arc::clone(&self.audio_expected); // For callback monitor idle detection

        // Capture runtime handle while we're still in async context
        let rt_handle = tokio::runtime::Handle::current();

        // Use buffer size already loaded from database
        // [DBD-PARAM-110] Audio buffer size configuration
        let buffer_size = self.audio_buffer_size;

        std::thread::spawn(move || {
            // Create audio output (must be done on non-async thread for cpal)
            // [ARCH-VOL-020] Pass shared volume Arc for synchronized control
            // [DBD-PARAM-110] Pass configurable buffer size
            let mut audio_output = match AudioOutput::new_with_volume(None, Some(volume_clone), Some(buffer_size)) {
                Ok(output) => output,
                Err(e) => {
                    error!("Failed to create audio output: {}", e);
                    return;
                }
            };

            // Create callback monitor for gap/stutter detection with actual device configuration
            let actual_sample_rate = audio_output.sample_rate();
            let actual_buffer_size = audio_output.buffer_size();

            info!(
                "Audio device config: {}Hz, {} frames per callback ({:.2}ms interval)",
                actual_sample_rate,
                actual_buffer_size,
                (actual_buffer_size as f64 / actual_sample_rate as f64) * 1000.0
            );

            let monitor = Arc::new(crate::playback::callback_monitor::CallbackMonitor::new(
                actual_sample_rate,
                actual_buffer_size,
                Some(state_clone),
                audio_expected_clone,
            ));

            // Store monitor in engine for API access and clone for monitoring task
            rt_handle.block_on(async {
                *callback_monitor_slot.write().await = Some(Arc::clone(&monitor));
            });

            // Clone for monitoring task (spawn_monitoring_task takes ownership)
            let monitor_for_task = Arc::clone(&monitor);

            // Spawn monitoring task for logging and event emission
            // Runs on tokio runtime, separate from real-time audio callback
            let monitor_shutdown = monitor_for_task.spawn_monitoring_task(rt_handle.clone());

            let monitor_clone = Arc::clone(&monitor);

            // Lock-free audio callback - reads from ring buffer only
            // [SSD-OUT-012] Real-time audio callback with no locks
            // [ISSUE-1] Fixed: No more try_write() or block_on() in audio callback
            let audio_callback = move || {
                // Lock-free read from ring buffer
                match consumer.pop() {
                    Some(frame) => frame,
                    None => {
                        // Buffer underrun - record and return silence
                        monitor_clone.record_underrun();
                        AudioFrame::zero()
                    }
                }
            };

            // Pass monitor to AudioOutput so it can record callback timing once per buffer
            if let Err(e) = audio_output.start(audio_callback, Some(monitor.clone())) {
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

            // Shutdown monitoring task
            monitor_shutdown.store(true, std::sync::atomic::Ordering::Relaxed);

            // Log final callback statistics
            let stats = monitor.stats();
            info!(
                "Audio output stopped - Callback stats: {} total callbacks, {} underruns, {} irregular intervals ({:.1}% irregular)",
                stats.callback_count,
                stats.underrun_count,
                stats.irregular_intervals,
                (stats.irregular_intervals as f64 / stats.callback_count.max(1) as f64) * 100.0
            );
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

        // Shutdown decoder worker
        Arc::clone(&self.decoder_worker).shutdown().await;
        info!("Decoder worker shut down successfully");

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
            let fade_curve_str = crate::db::settings::load_resume_fade_in_curve(&self.db_pool)
                .await
                .unwrap_or_else(|_| "exponential".to_string());

            // [SUB-INC-4B] Replace resume() with start_resume_fade() + set_state(Playing)
            {
                let sample_rate = 44100; // [DBD-BUF-010] Fixed 44.1kHz sample rate
                let fade_samples = ((fade_duration_ms as u64 * sample_rate) / 1000) as usize;
                let fade_curve = wkmp_common::FadeCurve::from_str(&fade_curve_str)
                    .unwrap_or(wkmp_common::FadeCurve::Exponential);

                let mut mixer = self.mixer.write().await;
                mixer.start_resume_fade(fade_samples, fade_curve);
                mixer.set_state(MixerState::Playing);
            }

            info!("Resuming from pause with {}ms {} fade-in", fade_duration_ms, fade_curve_str);
        }

        // Emit PlaybackStateChanged event
        self.state.broadcast_event(wkmp_common::events::WkmpEvent::PlaybackStateChanged {
            old_state: old_state,
            new_state: wkmp_common::events::PlaybackState::Playing,
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
        // [SUB-INC-4B] Replace pause() with set_state(Paused)
        self.mixer.write().await.set_state(MixerState::Paused);

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
            old_state: old_state,
            new_state: wkmp_common::events::PlaybackState::Paused,
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

        // Fetch album UUIDs for PassageCompleted event **[REQ-DEBT-FUNC-003]**
        let album_uuids = if let Some(pid) = current.passage_id {
            get_passage_album_uuids(&self.db_pool, pid)
                .await
                .unwrap_or_else(|e| {
                    warn!("Failed to fetch album UUIDs for passage {}: {}", pid, e);
                    Vec::new()
                })
        } else {
            Vec::new()
        };

        // Calculate duration_played **[REQ-DEBT-FUNC-002]**
        let duration_played = {
            let mixer = self.mixer.read().await;
            if let Some(start_time) = mixer.passage_start_time() {
                start_time.elapsed().as_secs_f64()
            } else {
                0.0
            }
        };

        // Emit PassageCompleted event with completed=false (skipped)
        self.state.broadcast_event(wkmp_common::events::WkmpEvent::PassageCompleted {
            passage_id: current.passage_id.unwrap_or_else(|| Uuid::nil()),
            album_uuids,
            duration_played,
            completed: false, // false = skipped
            timestamp: chrono::Utc::now(),
        });

        // Mark buffer as exhausted
        self.buffer_manager.mark_exhausted(current.queue_entry_id).await;

        // Stop mixer immediately
        // [SUB-INC-4B] Replace stop() with SPEC016 operations
        let mut mixer = self.mixer.write().await;
        mixer.clear_all_markers();
        mixer.clear_passage();
        mixer.set_state(MixerState::Paused);
        drop(mixer);

        // Remove buffer from memory
        if let Some(passage_id) = current.passage_id {
            self.buffer_manager.remove(passage_id).await;
        }

        info!("Mixer stopped and buffer cleaned up");

        // **[DBD-LIFECYCLE-020]** Release decoder-buffer chain before removing from queue
        // Implements requirement that chains are freed when passage is removed (skip counts as removal)
        self.release_chain(current.queue_entry_id).await;

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
        // [SUB-INC-4B] Replace stop() with SPEC016 operations
        let mut mixer = self.mixer.write().await;
        let passage_id_before_stop = mixer.get_current_passage_id();
        mixer.clear_all_markers();
        mixer.clear_passage();
        mixer.set_state(MixerState::Paused);
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

        // Emit QueueChanged event (queue is now empty)
        self.state.broadcast_event(wkmp_common::events::WkmpEvent::QueueChanged {
            queue: Vec::new(),
            trigger: wkmp_common::events::QueueChangeTrigger::UserDequeue,
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
        // No lock needed - buffer methods use &self with atomics
        let sample_rate = 44100; // [DBD-BUF-010] Fixed 44.1kHz sample rate

        // Convert milliseconds to frames
        let position_frames = ((position_ms as f32 / 1000.0) * sample_rate as f32) as usize;

        // Clamp to buffer bounds
        let stats = buffer.stats();
        let max_frames = stats.total_written as usize;
        let clamped_position = position_frames.min(max_frames.saturating_sub(1));

        // Update mixer position
        // [SUB-INC-4B] Replace set_position() with set_current_passage()
        // Note: Marker recalculation deferred to Phase 4 (currently just updates position)
        let mut mixer = self.mixer.write().await;
        if let Some(passage_id) = current.passage_id {
            let seek_tick = clamped_position as i64; // Convert frames to ticks (1:1 for now)
            mixer.set_current_passage(passage_id, seek_tick);
            // TODO Phase 4: Recalculate markers from seek point
        }
        drop(mixer);

        // Emit PlaybackProgress event with new position
        if let Some(passage_id) = current.passage_id {
            let actual_position_ms = ((clamped_position as f32 / sample_rate as f32) * 1000.0) as u64;

            // Get passage timing to calculate duration
            if let Ok(passage) = self.get_passage_timing(&current).await {
                let duration_ticks = if let Some(end_ticks) = passage.end_time_ticks {
                    end_ticks - passage.start_time_ticks
                } else {
                    // If end_time is None, calculate from buffer total_written
                    wkmp_common::timing::samples_to_ticks(stats.total_written as usize, 44100)
                };
                let duration_ms = wkmp_common::timing::ticks_to_ms(duration_ticks) as u64;

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

    /// Get shared buffer manager Arc for testing
    ///
    /// Provides access to buffer manager for integration tests.
    /// Used to verify buffer configuration settings are correctly loaded and applied.
    ///
    /// # Returns
    /// Cloned Arc to BufferManager
    ///
    /// **Phase 4:** Buffer manager accessor reserved for integration tests (not yet used by API)
    #[allow(dead_code)]
    pub fn get_buffer_manager(&self) -> Arc<BufferManager> {
        Arc::clone(&self.buffer_manager)
    }

    /// Get audio expected flag
    ///
    /// Returns whether audio output is expected (Playing state with non-empty queue).
    /// Used by monitoring services to distinguish expected underruns (idle) from problematic ones.
    ///
    /// # Returns
    /// true if Playing with non-empty queue, false if Paused or queue empty
    pub fn is_audio_expected(&self) -> bool {
        self.audio_expected.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Get callback monitor statistics
    ///
    /// Returns callback timing statistics for gap/stutter analysis.
    /// Returns None if audio thread hasn't started yet.
    ///
    /// [API] GET /playback/callback_stats
    pub async fn get_callback_stats(&self) -> Option<crate::playback::callback_monitor::CallbackStats> {
        let monitor_guard = self.callback_monitor.read().await;
        monitor_guard.as_ref().map(|m| m.stats())
    }

    /// Get current queue length
    ///
    /// Returns the number of passages in the playback queue.
    ///
    /// [API] GET /playback/queue (returns queue length in response)
    ///
    /// **Phase 4:** Queue length API reserved for queue management UI (not yet exposed via REST)
    #[allow(dead_code)]
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
    /// **[DBD-OV-040]** Returns status of all decoder-resampler-fade-buffer chains.
    /// **[DBD-OV-050]** Up to `maximum_decode_streams` chains (default: 12).
    /// **[DBD-OV-080]** Passage-based chain association (not position-based).
    ///
    /// Used for developer UI monitoring panel.
    pub async fn get_buffer_chains(&self) -> Vec<wkmp_common::events::BufferChainInfo> {
        use wkmp_common::events::BufferChainInfo;

        // **[DBD-PARAM-050]** Loaded from settings (default: 12)
        let maximum_decode_streams = self.maximum_decode_streams;

        // Get mixer state to determine active passages
        let mixer = self.mixer.read().await;
        let mixer_state = mixer.get_state_info();
        drop(mixer);

        // **[DBD-OV-080]** Get all queue entries (passage-based iteration)
        let queue = self.queue.read().await;
        let mut all_entries = Vec::new();

        // Add current passage (position 1 - "now playing") **[DBD-OV-060]**
        if let Some(current) = queue.current() {
            all_entries.push(current.clone());
        }

        // Add next passage (position 2 - "playing next") **[DBD-OV-070]**
        if let Some(next) = queue.next() {
            all_entries.push(next.clone());
        }

        // Add queued passages (positions 3-12 - pre-buffering)
        all_entries.extend(queue.queued().iter().cloned());
        drop(queue);

        // **[DBD-OV-080]** Chains remain associated with passages via persistent mapping
        // **[DBD-LIFECYCLE-040]** Look up chain assignments from HashMap
        let assignments = self.chain_assignments.read().await;

        // Build chain infos for all assigned chains
        // Use Vec<(chain_index, BufferChainInfo)> to maintain chain→passage association
        let mut chain_tuples: Vec<(usize, BufferChainInfo)> = Vec::new();

        for (queue_position, entry) in all_entries.iter().enumerate() {
            // Check if this entry has an assigned chain
            if let Some(&chain_index) = assignments.get(&entry.queue_entry_id) {
                // Get buffer information
                let buffer_info = self.buffer_manager.get_buffer_info(entry.queue_entry_id).await;
                let buffer_state = self.buffer_manager.get_buffer_state(entry.queue_entry_id).await;

                // Extract file name
                let file_name = entry.file_path.file_name()
                    .and_then(|n| n.to_str())
                    .map(|s| s.to_string());

                // Determine mixer role based on queue_position (not chain_index!)
                let is_active_in_mixer = if queue_position == 0 {
                    mixer_state.current_passage_id == entry.passage_id
                } else if queue_position == 1 {
                    mixer_state.next_passage_id == entry.passage_id
                } else {
                    false
                };

                let mixer_role = if queue_position == 0 {
                    if mixer_state.is_crossfading {
                        "Crossfading".to_string()
                    } else {
                        "Current".to_string()
                    }
                } else if queue_position == 1 {
                    if mixer_state.is_crossfading {
                        "Crossfading".to_string()
                    } else {
                        "Next".to_string()
                    }
                } else {
                    "Queued".to_string()
                };

                // Get playback position (only for current passage)
                let (playback_position_frames, playback_position_ms) = if queue_position == 0 {
                    (mixer_state.current_position_frames,
                     (mixer_state.current_position_frames as u64 * 1000) / 44100)
                } else if queue_position == 1 && mixer_state.is_crossfading {
                    (mixer_state.next_position_frames,
                     (mixer_state.next_position_frames as u64 * 1000) / 44100)
                } else {
                    (0, 0)
                };

                // **[DBD-BUF-065]** Calculate duration from discovered endpoint if available
                // **[DBD-COMP-015]** Ensures UI displays accurate duration instead of growing estimate
                let duration_ms = if let Some(discovered_ticks) = self.queue.read().await.get_discovered_endpoint(entry.queue_entry_id) {
                    // Use discovered endpoint for duration calculation
                    let duration = wkmp_common::timing::ticks_to_ms(discovered_ticks) as u64;
                    debug!(
                        "Using discovered endpoint for duration display: {} ticks = {} ms",
                        discovered_ticks, duration
                    );
                    Some(duration)
                } else {
                    // Fall back to buffer duration (samples buffered)
                    buffer_info.as_ref().and_then(|b| b.duration_ms)
                };

                chain_tuples.push((chain_index, BufferChainInfo {
                    slot_index: chain_index, // Use assigned chain_index, not enumerate position
                    queue_entry_id: Some(entry.queue_entry_id),
                    passage_id: entry.passage_id,
                    file_name,
                    queue_position: Some(queue_position), // 0-indexed per [SPEC020-MONITOR-050]

                    // Decoder stage (stubbed for Phase 3c)
                    decoder_state: None, // TODO: Query decoder pool status
                    decode_progress_percent: None,
                    is_actively_decoding: None,

                    // Decoder telemetry **[REQ-DEBT-FUNC-001]**
                    decode_duration_ms: buffer_info.as_ref().and_then(|b| b.decode_duration_ms),
                    source_file_path: buffer_info.as_ref().and_then(|b| b.file_path.clone()),

                    // Resampler stage (stubbed for Phase 3c)
                    source_sample_rate: None, // TODO: Get from decoder
                    resampler_active: None,
                    target_sample_rate: 44100, // **[DBD-PARAM-020]** working_sample_rate

                    // Fade stage (stubbed for Phase 3c)
                    fade_stage: None, // TODO: Get from decoder

                    // Buffer stage **[DBD-BUF-020]** through **[DBD-BUF-060]**
                    buffer_state: buffer_state.map(|s| s.to_string()),
                    buffer_fill_percent: buffer_info.as_ref().map(|b| b.fill_percent).unwrap_or(0.0),
                    buffer_fill_samples: buffer_info.as_ref().map(|b| b.samples_buffered).unwrap_or(0),
                    buffer_capacity_samples: buffer_info.as_ref().map(|b| b.capacity_samples).unwrap_or(0),
                    total_decoded_frames: buffer_info.as_ref().map(|b| b.total_decoded_frames).unwrap_or(0),

                    // Mixer stage
                    playback_position_frames,
                    playback_position_ms,
                    duration_ms,
                    is_active_in_mixer,
                    mixer_role,
                    started_at: None, // TODO: Track start time
                }));
            }
        }

        drop(assignments); // Release read lock

        // **[DBD-LIFECYCLE-030]** Fill idle chains for all unassigned chain indices
        // Implements requirement to report all chains 0..(maximum_decode_streams-1)
        for chain_idx in 0..maximum_decode_streams {
            if !chain_tuples.iter().any(|(idx, _)| *idx == chain_idx) {
                chain_tuples.push((chain_idx, BufferChainInfo::idle(chain_idx)));
            }
        }

        // Sort by chain_index for consistent display order
        chain_tuples.sort_by_key(|(idx, _)| *idx);

        // Extract BufferChainInfo from tuples and return
        chain_tuples.into_iter().map(|(_, info)| info).collect()
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
    ///
    /// **Phase 4:** Queue sync verification reserved for diagnostics (not yet used)
    #[allow(dead_code)]
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

        // **[DBD-LIFECYCLE-060]** Save existing chain assignments before reload
        // Reordering changes queue positions but not passage identities, so we
        // preserve chain assignments to maintain buffer chain stability
        let existing_assignments = self.chain_assignments.read().await.clone();

        // Call database reorder function
        crate::db::queue::reorder_queue(&self.db_pool, queue_entry_id, new_position).await?;

        // Reload queue from database to sync in-memory state
        let mut queue = self.queue.write().await;
        *queue = crate::playback::queue_manager::QueueManager::load_from_db(&self.db_pool).await?;
        drop(queue);

        // **[DBD-LIFECYCLE-060]** Restore chain assignments after reload
        // This ensures passages keep their assigned chains despite queue reordering
        let mut assignments = self.chain_assignments.write().await;
        *assignments = existing_assignments;
        drop(assignments);

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
            discovered_end_ticks: None, // **[DBD-BUF-065]** Will be set when endpoint discovered
        };

        self.queue.write().await.enqueue(entry);

        // Update audio_expected flag for ring buffer underrun classification
        self.update_audio_expected_flag().await;

        // **[DBD-LIFECYCLE-010]** Assign decoder-buffer chain on enqueue if available
        // Implements requirement that chains are assigned immediately when passage is enqueued
        self.assign_chain(queue_entry_id).await;

        Ok(queue_entry_id)
    }

    /// Remove queue entry from in-memory queue
    ///
    /// [API] DELETE /playback/queue/{queue_entry_id}
    /// **Traceability:** Removes entry from in-memory queue manager to match database deletion
    ///
    /// Returns true if entry was found and removed, false if not found
    pub async fn remove_queue_entry(&self, queue_entry_id: Uuid) -> bool {
        info!("Removing queue entry from in-memory queue: {}", queue_entry_id);

        // [BUG001] Check if removed entry is currently playing
        // If yes, must perform lifecycle cleanup: release chain, stop mixer, start next
        let is_current = {
            let queue = self.queue.read().await;
            queue.current().map(|c| c.queue_entry_id) == Some(queue_entry_id)
        };

        if is_current {
            // Currently playing passage - perform full lifecycle cleanup
            // [REQ-FIX-010] Stop playback immediately
            // [REQ-FIX-020] Release decoder chain resources
            // [REQ-FIX-030] Clear mixer state
            info!("Removing currently playing passage - performing lifecycle cleanup");

            // 1. Release decoder-buffer chain (free resources, allow reassignment)
            // [REQ-FIX-020] Release decoder chain resources
            self.release_chain(queue_entry_id).await;

            // 2. Stop mixer (clear playback state)
            // [REQ-FIX-030] Clear mixer state
            // [REQ-FIX-010] Stop playback immediately
            // [SUB-INC-4B] Replace stop() with SPEC016 operations
            {
                let mut mixer = self.mixer.write().await;
                mixer.clear_all_markers();
                mixer.clear_passage();
                mixer.set_state(MixerState::Paused);
            }
            info!("Mixer stopped for removed passage");

            // 3. Remove from queue structure
            // [REQ-FIX-040] Update queue structure
            let removed = self.queue.write().await.remove(queue_entry_id);

            if removed {
                info!("Successfully removed current passage {} from queue", queue_entry_id);

                // Update audio_expected flag for ring buffer underrun classification
                self.update_audio_expected_flag().await;

                // 4. Start next passage if queue has one
                // [REQ-FIX-050] Start next passage if queue non-empty
                // [REQ-FIX-080] New passage starts correctly after removal
                let has_current = self.queue.read().await.current().is_some();
                if has_current {
                    info!("Starting next passage after current removed");
                    // Trigger process_queue to start the next passage immediately
                    // This replicates the behavior from natural passage completion (line 2105)
                    // where next iteration starts new current passage
                    if let Err(e) = self.process_queue().await {
                        warn!("Failed to start next passage after removal: {}", e);
                    }
                } else {
                    info!("Queue empty after removing current passage");
                }
            }

            removed
        } else {
            // Non-current passage - simple removal (existing behavior)
            // [REQ-FIX-060] No disruption when removing non-current passage
            let removed = self.queue.write().await.remove(queue_entry_id);

            if removed {
                info!("Successfully removed queue entry {} from in-memory queue", queue_entry_id);
                // Update audio_expected flag for ring buffer underrun classification
                self.update_audio_expected_flag().await;
            } else {
                warn!("Queue entry {} not found in in-memory queue", queue_entry_id);
            }

            removed
        }
    }

    /// Calculate crossfade start position in milliseconds
    ///
    /// [ISSUE-7] Extracted helper method to reduce complexity
    /// [XFD-IMPL-070] Crossfade timing calculation
    /// [SRC-TICK-020] Uses tick-based passage timing
    /// **[DBD-BUF-065]** Uses discovered endpoint if available
    /// **[DBD-COMP-015]** Enables crossfade timing with undefined endpoints
    async fn calculate_crossfade_start_ms(&self, passage: &PassageWithTiming, queue_entry_id: Uuid) -> u64 {
        // Convert fade_out_point from ticks to ms
        if let Some(fade_out_ticks) = passage.fade_out_point_ticks {
            wkmp_common::timing::ticks_to_ms(fade_out_ticks) as u64
        } else {
            // If no explicit fade_out_point, calculate from end
            // Default: start crossfade 5 seconds before end

            // **[DBD-BUF-065]** Check for discovered endpoint first
            let queue = self.queue.read().await;
            let discovered_end = queue.get_discovered_endpoint(queue_entry_id);
            drop(queue);

            let end_ticks = if let Some(discovered) = discovered_end {
                debug!(
                    "Using discovered endpoint for crossfade calculation: {} ticks ({} ms)",
                    discovered,
                    wkmp_common::timing::ticks_to_ms(discovered)
                );
                Some(discovered)
            } else {
                passage.end_time_ticks
            };

            if let Some(end_ticks) = end_ticks {
                let end_ms = wkmp_common::timing::ticks_to_ms(end_ticks) as u64;
                if end_ms > 5000 {
                    end_ms - 5000
                } else {
                    end_ms
                }
            } else {
                // No endpoint known yet - return max value to prevent premature crossfade
                // Crossfade will be triggered once discovered endpoint is available
                debug!("No endpoint available for crossfade calculation, delaying crossfade");
                u64::MAX
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
        _crossfade_start_ms: u64,
    ) -> Result<bool> {
        // Get next queue entry
        let queue = self.queue.read().await;
        let next = match queue.next() {
            Some(n) => n.clone(),
            None => return Ok(false),
        };
        drop(queue);

        // Get next buffer
        let _next_buffer = match self.buffer_manager.get_buffer(next.queue_entry_id).await {
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
        // **BUG FIX**: Duration should be (end - fade_out_point), NOT (end - crossfade_start_ms)
        // Using crossfade_start_ms causes ticks→ms→ticks conversion loss and incorrect duration
        let fade_out_duration_ticks = if let Some(fade_out_ticks) = current_passage.fade_out_point_ticks {
            // Use discovered or defined endpoint
            let end_ticks = if let Some(discovered) = self.queue.read().await.get_discovered_endpoint(current.queue_entry_id) {
                discovered
            } else {
                current_passage.end_time_ticks.unwrap_or(fade_out_ticks)
            };
            end_ticks.saturating_sub(fade_out_ticks)
        } else {
            // No fade_out_point set - use default crossfade duration (5 seconds)
            wkmp_common::timing::ms_to_ticks(5000)
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

        // Fetch album UUIDs for PassageStarted event **[REQ-DEBT-FUNC-003]**
        let album_uuids = if let Some(pid) = next.passage_id {
            get_passage_album_uuids(&self.db_pool, pid)
                .await
                .unwrap_or_else(|e| {
                    warn!("Failed to fetch album UUIDs for passage {}: {}", pid, e);
                    Vec::new()
                })
        } else {
            Vec::new()
        };

        // Emit PassageStarted event for next passage
        self.state.broadcast_event(wkmp_common::events::WkmpEvent::PassageStarted {
            passage_id: next.passage_id.unwrap_or_else(|| Uuid::nil()),
            album_uuids,
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
                    if let Some(_buffer) = self.buffer_manager.get_buffer(current.queue_entry_id).await {
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
                                            song_albums: Vec::new(), // TODO: Fetch album UUIDs from database
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
                            current.queue_entry_id,
                            Some(fade_in_curve),
                            fade_in_duration_samples,
                        ).await;

                        // Mark buffer as playing
                        self.buffer_manager.mark_playing(current.queue_entry_id).await;

                        // Update position tracking
                        // [ISSUE-8] Use internal RwLock for queue_entry_id
                        *self.position.queue_entry_id.write().await = Some(current.queue_entry_id);

                        // Fetch album UUIDs for PassageStarted event **[REQ-DEBT-FUNC-003]**
                        let album_uuids = if let Some(pid) = current.passage_id {
                            get_passage_album_uuids(&self.db_pool, pid)
                                .await
                                .unwrap_or_else(|e| {
                                    warn!("Failed to fetch album UUIDs for passage {}: {}", pid, e);
                                    Vec::new()
                                })
                        } else {
                            Vec::new()
                        };

                        // Emit PassageStarted event
                        // [Event-PassageStarted] Passage playback began
                        self.state.broadcast_event(wkmp_common::events::WkmpEvent::PassageStarted {
                            passage_id: current.passage_id.unwrap_or_else(|| Uuid::nil()),
                            album_uuids,
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
                            if let Some(_buffer_ref) = self.buffer_manager.get_buffer(current.queue_entry_id).await {
                                // Convert frame position to milliseconds
                                // [DBD-BUF-010] Fixed 44.1kHz sample rate
                                let position_ms = (mixer_position_frames as u64 * 1000) / 44100;

                                // Calculate when crossfade should start
                                // **[DBD-BUF-065]** Pass queue_entry_id to check for discovered endpoint
                                let crossfade_start_ms = self.calculate_crossfade_start_ms(&passage, current.queue_entry_id).await;

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

        // Trigger decode for queued passages
        // [SSD-PBUF-010] Partial buffer decode for queued passages
        // **[DBD-PARAM-050]** Decode up to (maximum_decode_streams - 2) queued passages
        // Subtract 2 for current and next passages
        // **CRITICAL FIX:** Always request FULL decode (full=true) to prevent
        // third-file bug where passages decoded as "queued" with full=false
        // never get upgraded to full decode when promoted to "next".
        // Trade-off: Slightly higher memory usage, but guarantees correct playback.
        let max_queued = self.maximum_decode_streams.saturating_sub(2);
        for queued in queued_entries.iter().take(max_queued) {
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

                    // Fetch album UUIDs for PassageCompleted event **[REQ-DEBT-FUNC-003]**
                    let album_uuids = if let Some(pid) = passage_id_opt {
                        get_passage_album_uuids(&self.db_pool, pid)
                            .await
                            .unwrap_or_else(|e| {
                                warn!("Failed to fetch album UUIDs for passage {}: {}", pid, e);
                                Vec::new()
                            })
                    } else {
                        Vec::new()
                    };

                    // Calculate duration_played **[REQ-DEBT-FUNC-002]**
                    let duration_played = {
                        let mixer = self.mixer.read().await;
                        if let Some(start_time) = mixer.passage_start_time() {
                            start_time.elapsed().as_secs_f64()
                        } else {
                            0.0
                        }
                    };

                    // **[Event-PassageCompleted]** Emit completion event for OUTGOING passage
                    self.state.broadcast_event(wkmp_common::events::WkmpEvent::PassageCompleted {
                        passage_id: passage_id_opt.unwrap_or_else(|| Uuid::nil()),
                        album_uuids,
                        duration_played,
                        completed: true, // Crossfade completed naturally
                        timestamp: chrono::Utc::now(),
                    });

                    // **[DBD-LIFECYCLE-020]** Release decoder-buffer chain before removing from queue
                    // Passage completed via crossfade, free chain for reassignment
                    self.release_chain(completed_id).await;

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

                // Fetch album UUIDs for PassageCompleted event **[REQ-DEBT-FUNC-003]**
                let album_uuids = if let Some(pid) = current_pid {
                    get_passage_album_uuids(&self.db_pool, pid)
                        .await
                        .unwrap_or_else(|e| {
                            warn!("Failed to fetch album UUIDs for passage {}: {}", pid, e);
                            Vec::new()
                        })
                } else {
                    Vec::new()
                };

                // Calculate duration_played **[REQ-DEBT-FUNC-002]**
                let duration_played = {
                    let mixer = self.mixer.read().await;
                    if let Some(start_time) = mixer.passage_start_time() {
                        start_time.elapsed().as_secs_f64()
                    } else {
                        0.0
                    }
                };

                // Emit PassageCompleted event
                // [Event-PassageCompleted] Passage playback finished
                self.state.broadcast_event(wkmp_common::events::WkmpEvent::PassageCompleted {
                    passage_id: current_pid.unwrap_or_else(|| Uuid::nil()),
                    album_uuids,
                    duration_played,
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

                // **[DBD-LIFECYCLE-020]** Release decoder-buffer chain before removing from queue
                // Passage completed normally, free chain for reassignment
                self.release_chain(queue_entry_id).await;

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
                // [SUB-INC-4B] Replace stop() with SPEC016 operations
                {
                    let mut mixer = self.mixer.write().await;
                    mixer.clear_all_markers();
                    mixer.clear_passage();
                    mixer.set_state(MixerState::Paused);
                }

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

        // Submit to decoder worker (async - registers buffer immediately)
        self.decoder_worker
            .submit(entry.queue_entry_id, passage, priority, full_decode)
            .await?;

        debug!(
            "Submitted decode request for queue_entry_id={}, priority={:?}, full={}",
            entry.queue_entry_id, priority, full_decode
        );

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

    /// Handle buffer underrun with emergency refill
    ///
    /// **[REQ-AP-ERR-020]** Buffer underrun emergency refill with timeout
    ///
    /// # Strategy
    /// 1. Emit BufferUnderrun event
    /// 2. Request immediate priority decode (emergency refill)
    /// 3. Wait up to timeout for buffer to reach minimum threshold
    /// 4. If recovered: emit BufferUnderrunRecovered event
    /// 5. If timeout: skip passage and continue
    async fn handle_buffer_underrun(&self, queue_entry_id: uuid::Uuid, headroom: usize) {
        warn!(
            "⚠️  Buffer underrun: queue_entry={}, headroom={} samples",
            queue_entry_id, headroom
        );

        // Get current passage info for event emission
        let queue = self.queue.read().await;
        let current_entry = queue.current().cloned();
        drop(queue);

        let passage_id = current_entry
            .as_ref()
            .and_then(|e| e.passage_id)
            .unwrap_or_else(|| uuid::Uuid::nil());

        // Calculate buffer fill percent
        // **[DBD-PARAM-080]** buffer capacity = playout_ringbuffer_size (from settings)
        // For now, use standard 10-second buffer = 441,000 samples @ 44.1kHz stereo
        const STANDARD_BUFFER_CAPACITY: usize = 441_000;
        let buffer_fill_percent = (headroom as f32 / STANDARD_BUFFER_CAPACITY as f32) * 100.0;

        // **[ERH-BUF-010]** Emit BufferUnderrun event
        self.state.broadcast_event(wkmp_common::events::WkmpEvent::BufferUnderrun {
            passage_id,
            buffer_fill_percent,
            timestamp: chrono::Utc::now(),
        });

        // **[ERH-BUF-015]** Load recovery timeout from database settings
        let recovery_timeout_ms = match crate::db::settings::load_buffer_underrun_timeout(&self.db_pool).await {
            Ok(timeout) => timeout,
            Err(e) => {
                warn!("Failed to load buffer_underrun_recovery_timeout_ms from settings: {}, using default 2000ms", e);
                2000 // Default: 2 seconds (spec says 500ms, but summary indicated 2000ms for slow hardware)
            }
        };

        debug!(
            "Buffer underrun recovery: timeout={}ms, requesting emergency decode",
            recovery_timeout_ms
        );

        // **[ERH-BUF-010]** Request immediate priority decode (emergency refill)
        if let Some(entry) = current_entry {
            match self.request_decode(&entry, DecodePriority::Immediate, false).await {
                Ok(_) => {
                    debug!("Emergency decode request submitted for queue_entry={}", queue_entry_id);
                }
                Err(e) => {
                    error!(
                        "Failed to request emergency decode for queue_entry={}: {}",
                        queue_entry_id, e
                    );
                    // Continue with timeout logic anyway - buffer may already be refilling
                }
            }

            // Wait for buffer recovery with timeout
            let recovery_start = std::time::Instant::now();
            let timeout_duration = std::time::Duration::from_millis(recovery_timeout_ms);

            // **[DBD-PARAM-110]** mixer_min_start_level (configurable, default ~500ms)
            // For recovery, we need at least this much buffered to resume playback
            let min_buffer_threshold_ms = self.buffer_manager.get_min_buffer_threshold().await;
            let min_buffer_samples = (min_buffer_threshold_ms as usize * 44100 / 1000) * 2; // Stereo

            loop {
                // Check if buffer has refilled to minimum threshold
                if let Some(buffer) = self.buffer_manager.get_buffer(queue_entry_id).await {
                    let available = buffer.occupied();
                    if available >= min_buffer_samples {
                        let recovery_time_ms = recovery_start.elapsed().as_millis() as u64;
                        info!(
                            "✅ Buffer underrun recovered: queue_entry={}, recovery_time={}ms, available={} samples",
                            queue_entry_id, recovery_time_ms, available
                        );

                        // **[ERH-BUF-010]** Emit BufferUnderrunRecovered event
                        self.state.broadcast_event(wkmp_common::events::WkmpEvent::BufferUnderrunRecovered {
                            passage_id,
                            recovery_time_ms,
                            timestamp: chrono::Utc::now(),
                        });

                        return; // Recovery successful
                    }
                }

                // Check timeout
                if recovery_start.elapsed() >= timeout_duration {
                    error!(
                        "❌ Buffer underrun recovery timeout after {}ms: queue_entry={}, skipping passage",
                        recovery_timeout_ms, queue_entry_id
                    );

                    // **[ERH-BUF-010]** Timeout: skip passage and continue
                    match self.skip_next().await {
                        Ok(_) => {
                            info!("Skipped to next passage after buffer underrun timeout");
                        }
                        Err(e) => {
                            error!("Failed to skip passage after underrun timeout: {}", e);
                        }
                    }

                    return;
                }

                // Sleep briefly before next check (10ms polling interval)
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            }
        } else {
            warn!("No current passage found during buffer underrun, cannot request emergency decode");
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
                                song_albums: Vec::new(), // TODO: Fetch album UUIDs from database
                                position_ms,
                                timestamp: chrono::Utc::now(),
                            });
                        }
                    }
                    drop(timeline);

                    // [2] Check if PlaybackProgress interval elapsed
                    if position_ms >= last_progress_position_ms + progress_interval_ms {
                        last_progress_position_ms = position_ms;

                        // Get passage_id and timing for duration calculation
                        let queue = self.queue.read().await;
                        let current = queue.current().cloned();
                        drop(queue);

                        if let Some(current) = current {
                            if current.queue_entry_id == queue_entry_id {
                                // Calculate duration from passage timing
                                if let Ok(passage) = self.get_passage_timing(&current).await {
                                    let duration_ticks = if let Some(end_ticks) = passage.end_time_ticks {
                                        end_ticks - passage.start_time_ticks
                                    } else {
                                        // If end_time is None, estimate from buffer stats
                                        if let Some(buffer_ref) = self.buffer_manager.get_buffer(queue_entry_id).await {
                                            // No lock needed - stats() uses atomics
                                            let stats = buffer_ref.stats();
                                            wkmp_common::timing::samples_to_ticks(stats.total_written as usize, 44100)
                                        } else {
                                            0
                                        }
                                    };
                                    let duration_ms = wkmp_common::timing::ticks_to_ms(duration_ticks) as u64;

                                    let passage_id = current.passage_id.unwrap_or_else(|| uuid::Uuid::nil());

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
                        }
                    }

                    // [3] Update shared state
                    let queue = self.queue.read().await;
                    if let Some(current) = queue.current() {
                        if current.queue_entry_id == queue_entry_id {
                            // Calculate duration from passage timing
                            if let Ok(passage) = self.get_passage_timing(current).await {
                                let duration_ticks = if let Some(end_ticks) = passage.end_time_ticks {
                                    end_ticks - passage.start_time_ticks
                                } else {
                                    // If end_time is None, estimate from buffer stats
                                    if let Some(buffer_ref) = self.buffer_manager.get_buffer(queue_entry_id).await {
                                        // No lock needed - stats() uses atomics
                                        let stats = buffer_ref.stats();
                                        wkmp_common::timing::samples_to_ticks(stats.total_written as usize, 44100)
                                    } else {
                                        0
                                    }
                                };
                                let duration_ms = wkmp_common::timing::ticks_to_ms(duration_ticks) as u64;

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
            decoder_worker: Arc::clone(&self.decoder_worker),
            mixer: Arc::clone(&self.mixer),
            position: self.position.clone_handles(), // [ISSUE-8] Clone inner Arcs
            running: Arc::clone(&self.running),
            position_event_tx: self.position_event_tx.clone(), // **[REV002]** Clone sender
            volume: Arc::clone(&self.volume), // [ARCH-VOL-020] Clone volume Arc
            position_event_rx: Arc::clone(&self.position_event_rx), // **[REV002]** Clone receiver
            current_song_timeline: Arc::clone(&self.current_song_timeline), // **[REV002]** Clone timeline
            audio_expected: Arc::clone(&self.audio_expected), // Clone audio_expected flag for ring buffer
            buffer_event_rx: Arc::clone(&self.buffer_event_rx), // **[PERF-POLL-010]** Clone buffer event receiver
            maximum_decode_streams: self.maximum_decode_streams, // [DBD-PARAM-050] Copy decode stream limit
            chain_assignments: Arc::clone(&self.chain_assignments), // [DBD-LIFECYCLE-040] Clone chain assignment tracking
            available_chains: Arc::clone(&self.available_chains), // [DBD-LIFECYCLE-030] Clone available chains pool
            buffer_monitor_rate_ms: Arc::clone(&self.buffer_monitor_rate_ms), // [SPEC020-MONITOR-120] Clone monitor rate
            buffer_monitor_update_now: Arc::clone(&self.buffer_monitor_update_now), // [SPEC020-MONITOR-130] Clone update trigger
            callback_monitor: Arc::clone(&self.callback_monitor), // Clone callback monitor for gap detection
            audio_buffer_size: self.audio_buffer_size, // [DBD-PARAM-110] Copy audio buffer size
        }
    }

    /// Assign a decoder-buffer chain to a queue entry
    ///
    /// **[DBD-LIFECYCLE-010]** Chain assignment on enqueue
    /// **[DBD-LIFECYCLE-030]** Lowest-numbered available chain allocation
    ///
    /// Allocates the lowest-numbered available chain from the pool and assigns it
    /// to the given queue entry. Returns the assigned chain index, or None if all
    /// chains are currently allocated.
    ///
    /// # Arguments
    /// * `queue_entry_id` - UUID of the queue entry to assign a chain to
    ///
    /// # Returns
    /// * `Some(chain_index)` - The chain index (0..maximum_decode_streams) assigned
    /// * `None` - No chains available (all maximum_decode_streams chains in use)
    async fn assign_chain(&self, queue_entry_id: Uuid) -> Option<usize> {
        debug!("🔍 assign_chain: START for {}", queue_entry_id);
        debug!("🔍 assign_chain: Acquiring available_chains write lock...");
        let mut available = self.available_chains.write().await;
        debug!("🔍 assign_chain: Acquired available_chains write lock, {} chains available", available.len());
        if let Some(Reverse(chain_index)) = available.pop() {
            debug!("🔍 assign_chain: Popped chain_index {}, acquiring chain_assignments write lock...", chain_index);
            let mut assignments = self.chain_assignments.write().await;
            debug!("🔍 assign_chain: Acquired chain_assignments write lock");
            assignments.insert(queue_entry_id, chain_index);
            debug!(
                queue_entry_id = %queue_entry_id,
                chain_index = chain_index,
                "Assigned decoder-buffer chain to passage"
            );
            debug!("🔍 assign_chain: DONE - returning Some({})", chain_index);
            Some(chain_index)
        } else {
            warn!(
                queue_entry_id = %queue_entry_id,
                "No available chains for assignment (all {} chains in use)",
                self.maximum_decode_streams
            );
            debug!("🔍 assign_chain: DONE - returning None");
            None
        }
    }

    /// Release a decoder-buffer chain from a queue entry
    ///
    /// **[DBD-LIFECYCLE-020]** Chain release on completion
    /// **[DBD-LIFECYCLE-030]** Return chain to available pool for reuse
    ///
    /// Removes the passage→chain mapping and returns the chain to the available pool
    /// for assignment to future queue entries. This is called when a passage completes
    /// playback or is removed from the queue.
    ///
    /// # Arguments
    /// * `queue_entry_id` - UUID of the queue entry whose chain should be released
    async fn release_chain(&self, queue_entry_id: Uuid) {
        let mut assignments = self.chain_assignments.write().await;
        if let Some(chain_index) = assignments.remove(&queue_entry_id) {
            let mut available = self.available_chains.write().await;
            available.push(Reverse(chain_index));
            debug!(
                queue_entry_id = %queue_entry_id,
                chain_index = chain_index,
                "Released decoder-buffer chain from passage"
            );
        } else {
            // Not an error - passage may have been queued when all chains were allocated
            debug!(
                queue_entry_id = %queue_entry_id,
                "No chain to release (passage was not assigned a chain)"
            );
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
                    let _buffer = match self.buffer_manager.get_buffer(queue_entry_id).await {
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
                                        song_albums: Vec::new(), // TODO: Fetch album UUIDs from database
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
                        queue_entry_id,
                        Some(fade_in_curve),
                        fade_in_duration_samples,
                    ).await;

                    // Mark buffer as playing
                    self.buffer_manager.mark_playing(queue_entry_id).await;

                    // Update position tracking
                    *self.position.queue_entry_id.write().await = Some(queue_entry_id);

                    // Fetch album UUIDs for PassageStarted event **[REQ-DEBT-FUNC-003]**
                    let album_uuids = if let Some(pid) = current.passage_id {
                        get_passage_album_uuids(&self.db_pool, pid)
                            .await
                            .unwrap_or_else(|e| {
                                warn!("Failed to fetch album UUIDs for passage {}: {}", pid, e);
                                Vec::new()
                            })
                    } else {
                        Vec::new()
                    };

                    // Emit PassageStarted event
                    self.state.broadcast_event(wkmp_common::events::WkmpEvent::PassageStarted {
                        passage_id: current.passage_id.unwrap_or_else(|| uuid::Uuid::nil()),
                        album_uuids,
                        timestamp: chrono::Utc::now(),
                    });

                    let elapsed = start_time.elapsed();
                    info!(
                        "✅ Mixer started in {:.2}ms (event-driven instant start)",
                        elapsed.as_secs_f64() * 1000.0
                    );
                }

                Some(BufferEvent::Exhausted { queue_entry_id, headroom }) => {
                    // **[REQ-AP-ERR-020]** Buffer underrun emergency refill
                    self.handle_buffer_underrun(queue_entry_id, headroom).await;
                }

                Some(BufferEvent::StateChanged { .. }) |
                Some(BufferEvent::Finished { .. }) => {
                    // Future: Handle other buffer events
                    // For now, only ReadyForStart is used for instant mixer start
                }

                Some(BufferEvent::EndpointDiscovered { queue_entry_id, actual_end_ticks }) => {
                    // **[DBD-DEC-095]** Endpoint discovery notification
                    // **[DBD-BUF-065]** Propagate discovered endpoint to queue manager
                    // **[DBD-COMP-015]** Enables crossfade timing with undefined endpoints
                    info!(
                        "Endpoint discovered for {}: {} ticks ({} ms)",
                        queue_entry_id,
                        actual_end_ticks,
                        wkmp_common::timing::ticks_to_ms(actual_end_ticks)
                    );

                    // Update queue manager with discovered endpoint
                    let mut queue = self.queue.write().await;
                    if queue.set_discovered_endpoint(queue_entry_id, actual_end_ticks) {
                        info!(
                            "Queue updated with discovered endpoint for {}: {} ticks ({} ms)",
                            queue_entry_id,
                            actual_end_ticks,
                            wkmp_common::timing::ticks_to_ms(actual_end_ticks)
                        );
                    } else {
                        warn!(
                            "Failed to update queue with discovered endpoint for {}: entry not found",
                            queue_entry_id
                        );
                    }
                }

                None => {
                    info!("Buffer event handler stopping (channel closed)");
                    break;
                }
            }
        }
    }

    /// Background task: Emit BufferChainStatus events at client-controlled rate
    ///
    /// **[SPEC020-MONITOR-120]** Client-controlled SSE emission rate
    /// **[SPEC020-MONITOR-130]** Manual update trigger support
    ///
    /// The emission rate is controlled by `buffer_monitor_rate_ms`:
    /// - 100: Fast updates (10Hz) for visualizing rapid buffer filling
    /// - 1000: Normal updates (1Hz) for typical monitoring
    /// - 0: Manual mode (no automatic updates, only on update_now trigger)
    ///
    /// The `buffer_monitor_update_now` flag forces immediate emission regardless of mode.
    async fn buffer_chain_status_emitter(&self) {
        use tokio::time::interval;
        use std::time::Duration;

        info!("BufferChainStatus emitter started (client-controlled rate)");

        let mut tick = interval(Duration::from_millis(10)); // Fast poll internal state (10ms)
        let mut last_emission = std::time::Instant::now();
        let mut last_chains: Option<Vec<wkmp_common::events::BufferChainInfo>> = None;

        loop {
            tick.tick().await;

            // Check if engine is still running
            if !*self.running.read().await {
                info!("BufferChainStatus emitter stopping");
                break;
            }

            // Check current update rate
            let rate_ms = *self.buffer_monitor_rate_ms.read().await;
            let update_now = self.buffer_monitor_update_now.swap(false, Ordering::Relaxed);

            // Determine if we should emit
            let should_emit = if update_now {
                // Manual "update now" trigger
                true
            } else if rate_ms == 0 {
                // Manual mode - no automatic updates
                false
            } else {
                // Automatic mode - check if interval has elapsed
                last_emission.elapsed().as_millis() >= rate_ms as u128
            };

            if should_emit {
                // Get current buffer chain status
                let chains = self.get_buffer_chains().await;

                // Only emit if data has changed (or forced update_now)
                let chains_changed = update_now || match &last_chains {
                    None => true, // First iteration - always emit
                    Some(prev) => {
                        // Compare key fields that indicate meaningful changes
                        prev.len() != chains.len() ||
                        prev.iter().zip(chains.iter()).any(|(p, c)| {
                            p.queue_position != c.queue_position ||
                            p.buffer_state != c.buffer_state ||
                            p.buffer_fill_percent != c.buffer_fill_percent ||
                            p.total_decoded_frames != c.total_decoded_frames ||
                            p.mixer_role != c.mixer_role ||
                            p.is_active_in_mixer != c.is_active_in_mixer ||
                            p.fade_stage != c.fade_stage ||
                            p.playback_position_ms != c.playback_position_ms
                        })
                    }
                };

                if chains_changed {
                    self.state.broadcast_event(wkmp_common::events::WkmpEvent::BufferChainStatus {
                        timestamp: chrono::Utc::now(),
                        chains: chains.clone(),
                    });
                    last_chains = Some(chains);
                    last_emission = std::time::Instant::now();
                }
            }
        }
    }

    /// Set buffer chain monitor update rate
    ///
    /// **[SPEC020-MONITOR-120]** Client-controlled SSE emission rate
    ///
    /// # Arguments
    /// * `rate_ms` - Update interval in milliseconds (100, 1000, or 0 for manual)
    pub async fn set_buffer_monitor_rate(&self, rate_ms: u64) {
        *self.buffer_monitor_rate_ms.write().await = rate_ms;
        info!("Buffer monitor rate set to: {}ms", rate_ms);
    }

    /// Trigger immediate buffer chain status update
    ///
    /// **[SPEC020-MONITOR-130]** Manual update trigger
    ///
    /// Forces one immediate BufferChainStatus SSE emission, regardless of current mode.
    pub fn trigger_buffer_monitor_update(&self) {
        self.buffer_monitor_update_now.store(true, Ordering::Relaxed);
        debug!("Buffer monitor update now triggered");
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

    /// Get pipeline metrics for integrity validation
    ///
    /// **[PHASE1-INTEGRITY]** Returns aggregated metrics from buffer manager and mixer
    ///
    /// # Returns
    /// PipelineMetrics with buffer statistics and mixer frame count
    ///
    /// # Note
    /// Decoder statistics are not yet integrated in this initial implementation.
    /// Validation will use buffer write counters as proxy for decoder output.
    pub async fn get_pipeline_metrics(&self) -> crate::playback::PipelineMetrics {
        use crate::playback::PassageMetrics;
        use std::collections::HashMap;

        // Get all buffer statistics
        let buffer_stats = self.buffer_manager.get_all_buffer_statistics().await;

        // Convert buffer stats to passage metrics
        // Note: decoder_frames_pushed is approximated from buffer_samples_written / 2
        // This is acceptable since the validation checks buffer integrity primarily
        let mut passages = HashMap::new();
        for (queue_entry_id, buf_stats) in buffer_stats {
            let passage_metrics = PassageMetrics::new(
                queue_entry_id,
                (buf_stats.total_samples_written / 2) as usize, // Approximate decoder frames from buffer writes
                buf_stats.total_samples_written,
                buf_stats.total_samples_read,
                None, // file_path not available from buffer stats
            );
            passages.insert(queue_entry_id, passage_metrics);
        }

        // Get mixer total frames mixed
        let mixer_total_frames_mixed = self.mixer.write().await.get_total_frames_mixed();

        crate::playback::PipelineMetrics::new(passages, mixer_total_frames_mixed)
    }

    /// Start automatic validation service
    ///
    /// **[ARCH-AUTO-VAL-001]** Starts periodic pipeline integrity validation
    ///
    /// Creates and starts a ValidationService background task that loads its
    /// configuration from database settings and runs periodic validations,
    /// emitting validation events via SSE.
    ///
    /// # Arguments
    /// * `engine` - Arc reference to self (PlaybackEngine)
    /// * `db_pool` - Database connection pool for loading configuration
    ///
    /// # Note
    /// This should be called once during engine initialization. The validation
    /// service will continue running in the background until the engine is dropped.
    pub async fn start_validation_service(engine: Arc<Self>, db_pool: Pool<Sqlite>) {
        use crate::playback::validation_service::{ValidationConfig, ValidationService};

        // Load config from database
        let config = ValidationConfig::from_database(&db_pool).await;

        let validation_service = Arc::new(ValidationService::new(
            config,
            engine.clone(),
            engine.state.clone(),
        ));

        validation_service.run();
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

    /// **[DBD-OV-080]** Test get_buffer_chains() returns all 12 chains (passage-based iteration)
    #[tokio::test]
    async fn test_buffer_chain_12_passage_iteration() {
        let db = create_test_db().await;
        let state = Arc::new(SharedState::new());
        let engine = PlaybackEngine::new(db, state).await.unwrap();

        // Create temporary files for testing
        let temp_dir = std::env::temp_dir();
        let mut passages = Vec::new();
        for i in 0..15 {
            let passage = temp_dir.join(format!("test_buffer_chain_{}.mp3", i));
            std::fs::write(&passage, b"").unwrap();
            passages.push(passage);
        }

        // Enqueue 15 passages (should see maximum_decode_streams = 12)
        for passage in &passages {
            engine.enqueue_file(passage.clone()).await.unwrap();
        }

        // Get buffer chains
        let chains = engine.get_buffer_chains().await;

        // Should always return exactly 12 chains (maximum_decode_streams default)
        assert_eq!(chains.len(), 12, "get_buffer_chains() should return exactly 12 chains");

        // First 12 should have queue_entry_id and passage_id (active chains)
        for i in 0..12 {
            assert!(
                chains[i].queue_entry_id.is_some(),
                "Chain {} should have queue_entry_id",
                i
            );
        }

        // Clean up
        for passage in &passages {
            let _ = std::fs::remove_file(passage);
        }
    }

    /// **[DBD-OV-080]** Test passage-based association (queue_entry_id persistence)
    #[tokio::test]
    async fn test_buffer_chain_passage_based_association() {
        let db = create_test_db().await;
        let state = Arc::new(SharedState::new());
        let engine = PlaybackEngine::new(db, state).await.unwrap();

        // Create temporary files
        let temp_dir = std::env::temp_dir();
        let passage1 = temp_dir.join("test_passage_based_1.mp3");
        let passage2 = temp_dir.join("test_passage_based_2.mp3");
        let passage3 = temp_dir.join("test_passage_based_3.mp3");

        std::fs::write(&passage1, b"").unwrap();
        std::fs::write(&passage2, b"").unwrap();
        std::fs::write(&passage3, b"").unwrap();

        // Enqueue 3 passages
        engine.enqueue_file(passage1.clone()).await.unwrap();
        engine.enqueue_file(passage2.clone()).await.unwrap();
        engine.enqueue_file(passage3.clone()).await.unwrap();

        // Get initial buffer chains (3 passages + 9 idle)
        let chains_before = engine.get_buffer_chains().await;

        // Verify we have 12 chains total
        assert_eq!(chains_before.len(), 12);

        // **[DBD-OV-080]** Passages stay in their assigned chains
        // Before skip: Passage1=chain0(pos0), Passage2=chain1(pos1), Passage3=chain2(pos2)
        let passage2_qe_id = chains_before[1].queue_entry_id;
        assert!(passage2_qe_id.is_some(), "Chain 1 should have passage");

        // Skip current passage (advance queue - passage1 in chain 0)
        engine.skip_next().await.unwrap();

        // Get buffer chains again after queue advance
        let chains_after = engine.get_buffer_chains().await;

        // **[DBD-OV-080]** After skip: chain0=idle, Passage2 STILL in chain1 (now pos0), Passage3 STILL in chain2 (now pos1)
        assert_eq!(
            chains_after[0].queue_entry_id,
            None,
            "Chain 0 should be idle after passage was skipped and chain released"
        );

        // Passage2 should STILL be in chain 1 (chains remain associated with passages)
        assert_eq!(
            chains_after[1].queue_entry_id,
            passage2_qe_id,
            "Passage-based association: passage should stay in same chain 1"
        );

        // Passage2's queue_position should have changed from 1 to 0
        assert_eq!(
            chains_after[1].queue_position,
            Some(0),
            "queue_position should update to 0 (now playing)"
        );

        // Clean up
        let _ = std::fs::remove_file(&passage1);
        let _ = std::fs::remove_file(&passage2);
        let _ = std::fs::remove_file(&passage3);
    }

    /// **[DBD-OV-060]** **[DBD-OV-070]** Test queue_position tracking (1-indexed)
    #[tokio::test]
    async fn test_buffer_chain_queue_position_tracking() {
        let db = create_test_db().await;
        let state = Arc::new(SharedState::new());
        let engine = PlaybackEngine::new(db, state).await.unwrap();

        // Create temporary files
        let temp_dir = std::env::temp_dir();
        let passage1 = temp_dir.join("test_queue_pos_1.mp3");
        let passage2 = temp_dir.join("test_queue_pos_2.mp3");
        let passage3 = temp_dir.join("test_queue_pos_3.mp3");
        let passage4 = temp_dir.join("test_queue_pos_4.mp3");

        std::fs::write(&passage1, b"").unwrap();
        std::fs::write(&passage2, b"").unwrap();
        std::fs::write(&passage3, b"").unwrap();
        std::fs::write(&passage4, b"").unwrap();

        // Enqueue 4 passages
        engine.enqueue_file(passage1.clone()).await.unwrap();
        engine.enqueue_file(passage2.clone()).await.unwrap();
        engine.enqueue_file(passage3.clone()).await.unwrap();
        engine.enqueue_file(passage4.clone()).await.unwrap();

        // Get buffer chains
        let chains = engine.get_buffer_chains().await;

        // Verify queue_position values (0-indexed per [SPEC020-MONITOR-050])
        // **[DBD-OV-060]** Position 0 = "now playing"
        assert_eq!(
            chains[0].queue_position,
            Some(0),
            "Current passage should have queue_position 0 (now playing)"
        );

        // **[DBD-OV-070]** Position 1 = "playing next"
        assert_eq!(
            chains[1].queue_position,
            Some(1),
            "Next passage should have queue_position 1 (playing next)"
        );

        // Positions 2-3 = "queued passages"
        assert_eq!(
            chains[2].queue_position,
            Some(2),
            "Queued passage should have queue_position 2"
        );
        assert_eq!(
            chains[3].queue_position,
            Some(3),
            "Queued passage should have queue_position 3"
        );

        // Positions 4-11 = idle (no queue_position)
        for i in 4..12 {
            assert_eq!(
                chains[i].queue_position,
                None,
                "Idle chain {} should have queue_position None",
                i
            );
        }

        // Clean up
        let _ = std::fs::remove_file(&passage1);
        let _ = std::fs::remove_file(&passage2);
        let _ = std::fs::remove_file(&passage3);
        let _ = std::fs::remove_file(&passage4);
    }

    /// **[DBD-OV-080]** Test idle chain filling when queue < 12 entries
    #[tokio::test]
    async fn test_buffer_chain_idle_filling() {
        let db = create_test_db().await;
        let state = Arc::new(SharedState::new());
        let engine = PlaybackEngine::new(db, state).await.unwrap();

        // Empty queue - should have 12 idle chains
        let chains_empty = engine.get_buffer_chains().await;
        assert_eq!(chains_empty.len(), 12, "Should always return 12 chains");

        for (i, chain) in chains_empty.iter().enumerate() {
            assert_eq!(
                chain.queue_entry_id,
                None,
                "Empty queue: chain {} should have no queue_entry_id",
                i
            );
            assert_eq!(
                chain.queue_position,
                None,
                "Empty queue: chain {} should have no queue_position",
                i
            );
            assert_eq!(
                chain.buffer_state,
                Some("Idle".to_string()),
                "Empty queue: chain {} should be Idle",
                i
            );
        }

        // Create temporary files
        let temp_dir = std::env::temp_dir();
        let passage1 = temp_dir.join("test_idle_1.mp3");
        let passage2 = temp_dir.join("test_idle_2.mp3");

        std::fs::write(&passage1, b"").unwrap();
        std::fs::write(&passage2, b"").unwrap();

        // Enqueue 2 passages - should have 2 active + 10 idle chains
        engine.enqueue_file(passage1.clone()).await.unwrap();
        engine.enqueue_file(passage2.clone()).await.unwrap();

        let chains_partial = engine.get_buffer_chains().await;
        assert_eq!(chains_partial.len(), 12, "Should always return 12 chains");

        // First 2 should be active
        for i in 0..2 {
            assert!(
                chains_partial[i].queue_entry_id.is_some(),
                "Chain {} should be active",
                i
            );
            assert!(
                chains_partial[i].queue_position.is_some(),
                "Chain {} should have queue_position",
                i
            );
        }

        // Remaining 10 should be idle
        for i in 2..12 {
            assert_eq!(
                chains_partial[i].queue_entry_id,
                None,
                "Chain {} should be idle (no queue_entry_id)",
                i
            );
            assert_eq!(
                chains_partial[i].queue_position,
                None,
                "Chain {} should be idle (no queue_position)",
                i
            );
            assert_eq!(
                chains_partial[i].buffer_state,
                Some("Idle".to_string()),
                "Chain {} should have Idle state",
                i
            );
        }

        // Clean up
        let _ = std::fs::remove_file(&passage1);
        let _ = std::fs::remove_file(&passage2);
    }
}
