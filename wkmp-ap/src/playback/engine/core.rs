//! Core playback engine - lifecycle and orchestration
//!
//! **Responsibilities:**
//! - PlaybackEngine struct definition and initialization
//! - Lifecycle control (start, stop, play, pause, seek)
//! - Orchestration hub (watchdog_check - detection-only safety net for event-driven system)
//! - Buffer chain management (assign_chain, release_chain)
//!
//! **Traceability:**
//! - [REQ-DEBT-QUALITY-002-010] Core module for lifecycle/orchestration
//! - [SSD-FLOW-010] Complete playback sequence
//! - [SSD-ENG-020] Queue processing

use crate::audio::output::AudioOutput;
use crate::audio::types::AudioFrame;
use crate::db::passages::{create_ephemeral_passage, get_passage_with_timing, PassageWithTiming};
use crate::error::{Error, Result};
use crate::playback::buffer_manager::BufferManager;
use crate::playback::buffer_events::BufferEvent;
use crate::playback::decoder_worker::DecoderWorker;
use crate::playback::events::PlaybackEvent;
use crate::playback::mixer::{Mixer, MarkerEvent};
use crate::playback::queue_manager::{QueueEntry, QueueManager};
use crate::playback::ring_buffer::{AudioRingBuffer, AudioProducer};
use crate::playback::song_timeline::SongTimeline;
use crate::playback::types::DecodePriority;
use crate::state::SharedState;
use sqlx::{Pool, Sqlite};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio::sync::RwLock;
use std::sync::atomic::{AtomicBool, AtomicU64};
use tokio::time::{interval, Duration};
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use std::collections::{HashMap, BinaryHeap};
use std::cmp::Reverse;

// Test-only imports
#[cfg(test)]
use crate::state::PlaybackState;

/// Playback position tracking with lock-free atomic frame position
///
/// **[ISSUE-8]** Optimized to reduce lock contention in playback loop.
/// Frame position is updated on every iteration (~100Hz), so uses AtomicU64.
/// Queue entry ID is updated rarely (only on passage change), so uses RwLock.
pub(super) struct PlaybackPosition {
    /// Current passage UUID (queue entry) - updated infrequently
    pub(super) queue_entry_id: Arc<RwLock<Option<Uuid>>>,

    /// Current frame position in buffer - updated every loop iteration
    /// [ISSUE-8] AtomicU64 for lock-free updates in hot path
    pub(super) frame_position: Arc<AtomicU64>,
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
    pub(super) db_pool: Pool<Sqlite>,

    /// Shared state
    pub(super) state: Arc<SharedState>,

    /// Queue manager (tracks current/next/queued)
    pub(super) queue: Arc<RwLock<QueueManager>>,

    /// Buffer manager (manages buffer lifecycle)
    pub(super) buffer_manager: Arc<BufferManager>,

    /// Decoder worker (single-threaded decoder using DecoderChain architecture)
    pub(super) decoder_worker: Arc<DecoderWorker>,

    /// SPEC016-compliant mixer (batch mixing with event-driven markers)
    /// [SSD-MIX-010] Mixer component for audio frame generation
    /// [SUB-INC-4B] Replaced CrossfadeMixer with Mixer (SPEC016)
    pub(super) mixer: Arc<RwLock<Mixer>>,

    /// Passage start time for elapsed time calculations
    /// [SUB-INC-4B] Tracked in engine (not in mixer) for passage lifecycle
    pub(super) passage_start_time: Arc<RwLock<Option<tokio::time::Instant>>>,

    /// Current playback position
    /// [ISSUE-8] Now uses internal atomics for lock-free frame position updates
    pub(super) position: PlaybackPosition,

    /// Playback loop running flag
    pub(super) running: Arc<RwLock<bool>>,

    /// Position event channel sender
    /// **[REV002]** Event-driven position tracking
    /// Mixer sends position events to handler via this channel
    pub(super) position_event_tx: mpsc::UnboundedSender<PlaybackEvent>,

    /// Master volume control (shared with AudioOutput)
    /// **[ARCH-VOL-020]** Volume Arc shared between engine and audio output
    /// Updated by API handlers, read by audio callback
    pub(super) volume: Arc<Mutex<f32>>,

    /// Position event channel receiver
    /// **[REV002]** Taken by position_event_handler on start
    /// Wrapped in Option so it can be taken once
    pub(super) position_event_rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<PlaybackEvent>>>>,

    /// Song timeline for current passage
    /// **[REV002]** Loaded when passage starts, used for boundary detection
    /// None when no passage playing or passage has no songs
    pub(super) current_song_timeline: Arc<RwLock<Option<SongTimeline>>>,

    /// Audio expected flag for ring buffer underrun classification
    /// Set to true when Playing state with non-empty queue
    /// Set to false when Paused or queue is empty
    /// Shared with audio ring buffer consumer for context-aware logging
    pub(super) audio_expected: Arc<AtomicBool>,

    /// Audio callback monitor (for gap/stutter detection)
    /// Created in audio thread, stored here for API access
    pub(super) callback_monitor: Arc<RwLock<Option<Arc<crate::playback::callback_monitor::CallbackMonitor>>>>,

    /// Buffer event channel receiver for instant mixer start
    /// **[PERF-POLL-010]** Event-driven buffer readiness
    /// Receives ReadyForStart events when buffers reach minimum threshold
    pub(super) buffer_event_rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<BufferEvent>>>>,

    /// Maximum number of decoder-resampler-fade-buffer chains
    /// **[DBD-PARAM-050]** Configurable maximum decode streams (default: 12)
    pub(super) maximum_decode_streams: usize,

    /// Chain assignment tracking
    /// **[DBD-LIFECYCLE-040]** Maps queue_entry_id to chain_index for persistent association
    /// Implements requirement that chains remain associated with passages throughout lifecycle
    pub(super) chain_assignments: Arc<RwLock<HashMap<Uuid, usize>>>,

    /// Available chain pool
    /// **[DBD-LIFECYCLE-030]** Min-heap for lowest-numbered chain allocation
    /// Chains are allocated in ascending order (0, 1, 2, ...) for visual consistency
    pub(super) available_chains: Arc<RwLock<BinaryHeap<Reverse<usize>>>>,

    /// Buffer chain monitor update rate (milliseconds)
    /// **[SPEC020-MONITOR-120]** Client-controlled SSE emission rate
    /// Values: 100 (fast), 1000 (normal), or 0 (manual/disabled)
    pub(super) buffer_monitor_rate_ms: Arc<RwLock<u64>>,

    /// Force immediate buffer chain status emission
    /// **[SPEC020-MONITOR-130]** Manual update trigger
    /// Set to true to force one immediate emission, then automatically reset
    pub(super) buffer_monitor_update_now: Arc<AtomicBool>,

    /// Audio output buffer size in frames per callback
    /// **[DBD-PARAM-110]** Configurable audio buffer size (default: 512)
    pub(super) audio_buffer_size: u32,

    /// Working sample rate (Hz) - matches audio device native rate
    /// **[DBD-PARAM-020]** Determines resampling target in decoder chain
    /// All decoded audio is resampled to this rate before playback
    /// Set from AudioOutput device configuration (e.g., 44100, 48000, 96000)
    /// Uses std::sync::RwLock for compatibility with DecoderWorker (non-async context)
    pub(super) working_sample_rate: Arc<std::sync::RwLock<u32>>,

    /// Position marker interval in milliseconds
    /// **[DEBT-004]** Loaded from settings (default: 1000ms)
    /// **[REV002]** Event-driven position tracking interval
    /// Clamped to range: 100-5000ms
    pub(super) position_interval_ms: u32,
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
        let _mixer_min_start_level = mixer_min_start_level?; // [DBD-PARAM-088] Reserved for future use
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

        // **[DBD-PARAM-020]** Create working sample rate (default 44.1kHz, updated by audio thread)
        // Uses std::sync::RwLock for DecoderWorker compatibility (non-async context)
        let working_sample_rate = Arc::new(std::sync::RwLock::new(44100));

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

        // Load queue from database (before DecoderWorker::new since it needs queue reference)
        let queue_start = Instant::now();
        let queue_manager = QueueManager::load_from_db(&db_pool).await?;
        let queue_elapsed = queue_start.elapsed();
        info!(
            "Queue loaded in {:.2}ms: {} entries",
            queue_elapsed.as_secs_f64() * 1000.0,
            queue_manager.len()
        );

        // Create queue Arc (needed for DecoderWorker and PlaybackEngine)
        let queue = Arc::new(RwLock::new(queue_manager));

        // Create decoder worker
        // **[Phase 7]** Pass shared_state and db_pool for error handling
        // **[DBD-PARAM-020]** Pass working_sample_rate for device-matched resampling
        // **Query-based prioritization:** Pass queue for play_order queries
        let decoder_worker = Arc::new(DecoderWorker::new(
            Arc::clone(&buffer_manager),
            Arc::clone(&state),
            db_pool.clone(),
            Arc::clone(&working_sample_rate),
            Arc::clone(&queue),
        ));

        // **[REV002]** Create position event channel
        let (position_event_tx, position_event_rx) = mpsc::unbounded_channel();

        // Create mixer
        // [SSD-MIX-010] SPEC016-compliant mixer for batch mixing
        // [SUB-INC-4B] Replaced CrossfadeMixer with Mixer (event-driven markers)
        // **[DEBT-003]** Master volume loaded from settings (default: 0.5)
        // **[DBD-PARAM-020]** Pass working_sample_rate to mixer for tick calculations
        let mixer = Mixer::new(initial_volume, Arc::clone(&working_sample_rate));
        let mixer = Arc::new(RwLock::new(mixer));

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
            queue,
            buffer_manager,
            decoder_worker,
            mixer,
            passage_start_time: Arc::new(RwLock::new(None)), // [SUB-INC-4B] Track passage start time
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
            working_sample_rate, // [DBD-PARAM-020] Default 44.1kHz, updated when AudioOutput starts
            position_interval_ms: interval_ms, // [DEBT-004] Position marker interval from settings
        })
    }

    // Chain management methods moved to chains.rs (PLAN021)

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
        let output_ring_capacity = crate::db::settings::load_output_ringbuffer_capacity(&self.db_pool).await?;
        let ring_buffer = AudioRingBuffer::new(Some(output_ring_capacity), grace_period_ms, Arc::clone(&self.audio_expected)); // [DBD-PARAM-030]
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
        let _batch_size_low = mixer_config.batch_size_low; // Reserved for adaptive batch sizing
        let _batch_size_optimal = mixer_config.batch_size_optimal; // Reserved for adaptive batch sizing
        // [SUB-INC-4B] Clone additional variables for batch mixing
        let buffer_manager_clone = Arc::clone(&self.buffer_manager);
        let position_event_tx_clone = self.position_event_tx.clone();
        tokio::spawn(async move {
            info!("Mixer thread started");
            let mut check_interval = interval(Duration::from_micros(check_interval_us));

            // [SUB-INC-4B] Track crossfade state and current passages
            let mut is_crossfading = false;
            let mut current_passage_id: Option<Uuid> = None;
            let mut current_queue_entry_id: Option<Uuid> = None;
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
                current_queue_entry_id: &mut Option<Uuid>,
                _next_passage_id: &mut Option<Uuid>,
                frames_to_mix: usize,
            ) {
                // Allocate output buffer (stereo: 2 samples per frame)
                let mut output = vec![0.0f32; frames_to_mix * 2];

                let mut mixer_guard = mixer.write().await;

                // Update current passage ID and queue entry ID if changed
                *current_passage_id = mixer_guard.get_current_passage_id();
                *current_queue_entry_id = mixer_guard.get_current_queue_entry_id();

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
                    // **[PLAN014]** Crossfade mixing now handled via marker system (StartCrossfade marker)
                    // This branch should not be reached - crossfading transitions via markers
                    warn!("Unexpected crossfade state - should be marker-driven");
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
                handle_marker_events(events, event_tx, is_crossfading, current_queue_entry_id);

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
            fn handle_marker_events(
                events: Vec<MarkerEvent>,
                event_tx: &mpsc::UnboundedSender<PlaybackEvent>,
                is_crossfading: &mut bool,
                current_queue_entry_id: &Option<Uuid>,
            ) {
                for event in events {
                    match event {
                        MarkerEvent::PositionUpdate { position_ms } => {
                            if let Some(queue_entry_id) = *current_queue_entry_id {
                                event_tx.send(PlaybackEvent::PositionUpdate { queue_entry_id, position_ms }).ok();
                            }
                        }
                        MarkerEvent::StartCrossfade { next_passage_id: _ } => {
                            *is_crossfading = true;
                            // **[PLAN014]** Crossfade start now marker-driven via REV002 system
                            // State tracked automatically; mixer applies fade curves to pre-decoded samples
                        }
                        MarkerEvent::PassageComplete => {
                            *is_crossfading = false;
                            // Send PassageComplete event to trigger queue advancement
                            if let Some(queue_entry_id) = *current_queue_entry_id {
                                event_tx.send(PlaybackEvent::PassageComplete { queue_entry_id }).ok();
                                debug!("Sent PassageComplete event for queue_entry_id: {}", queue_entry_id);
                            }
                        }
                        MarkerEvent::SongBoundary { new_song_id: _ } => {
                            // **[PLAN014]** Song boundary tracking now marker-driven
                            // Currently no action needed; reserved for future cooldown system integration
                        }
                        MarkerEvent::EndOfFile { unreachable_markers } => {
                            warn!("EOF reached with {} unreachable markers", unreachable_markers.len());
                            // Treat EOF as passage complete - emit PassageComplete event
                            *is_crossfading = false;
                            if let Some(queue_entry_id) = *current_queue_entry_id {
                                event_tx.send(PlaybackEvent::PassageComplete { queue_entry_id }).ok();
                                debug!("Sent PassageComplete event (EOF) for queue_entry_id: {}", queue_entry_id);
                            }
                        }
                        MarkerEvent::EndOfFileBeforeLeadOut { planned_crossfade_tick, .. } => {
                            warn!("EOF before crossfade at tick {}", planned_crossfade_tick);
                            // Treat early EOF as passage complete - emit PassageComplete event
                            *is_crossfading = false;
                            if let Some(queue_entry_id) = *current_queue_entry_id {
                                event_tx.send(PlaybackEvent::PassageComplete { queue_entry_id }).ok();
                                debug!("Sent PassageComplete event (early EOF) for queue_entry_id: {}", queue_entry_id);
                            }
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
                        &mut current_queue_entry_id,
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
                        &mut current_queue_entry_id,
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
                        &mut current_queue_entry_id,
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
        let working_sample_rate_clone = Arc::clone(&self.working_sample_rate); // [DBD-PARAM-020] Update from device config

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

            // Update working sample rate to match device configuration
            // [DBD-PARAM-020] Decoder chains will resample to this rate
            *working_sample_rate_clone.write().unwrap() = actual_sample_rate;
            info!("Working sample rate set to {}Hz (matches device)", actual_sample_rate);

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

    // Playback control methods (play, pause, seek, watchdog, crossfade, etc.) moved to playback.rs (PLAN021)

    /// Request decode for a passage
    ///
    /// **[PLAN020]** Made pub(super) for event-driven decode triggering from queue module
    pub(super) async fn request_decode(
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
    pub(super) async fn get_passage_timing(&self, entry: &QueueEntry) -> Result<PassageWithTiming> {
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
    pub(super) async fn handle_buffer_underrun(&self, queue_entry_id: uuid::Uuid, headroom: usize) {
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
            .unwrap_or_else(uuid::Uuid::nil);

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
            // **[DBD-PARAM-020]** Use working sample rate (matches device)
            let sample_rate = *self.working_sample_rate.read().unwrap();
            let min_buffer_samples = (min_buffer_threshold_ms as usize * sample_rate as usize / 1000) * 2; // Stereo

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

    // assign_chain() method moved to chains.rs (PLAN021)

    // release_chain() method moved to chains.rs (PLAN021)

    // assign_chains_to_unassigned_entries() method moved to chains.rs (PLAN021)

    /// Clone the engine's handles for sharing across threads
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
            passage_start_time: Arc::clone(&self.passage_start_time), // [SUB-INC-4B] Clone passage start time tracking
            working_sample_rate: Arc::clone(&self.working_sample_rate), // [DBD-PARAM-020] Clone working sample rate
            position_interval_ms: self.position_interval_ms, // [DEBT-004] Copy position interval from settings
        }
    }
}

// **[TEST-HARNESS]** Test helpers for integration testing
// Note: These are NOT #[cfg(test)] so they're accessible from integration tests
impl PlaybackEngine {
    // test_get_chain_assignments() and test_get_available_chains() methods moved to chains.rs (PLAN021)

    /// Get buffer fill percentage for a queue entry
    ///
    /// **[TEST-HARNESS]** For testing only
    #[doc(hidden)]
    pub async fn test_get_buffer_fill_percent(&self, queue_entry_id: Uuid) -> Option<f32> {
        self.buffer_manager.get_fill_percent(queue_entry_id).await
    }

    /// Get queue entries for testing (test-specific version)
    ///
    /// **[TEST-HARNESS]** For testing only
    #[doc(hidden)]
    pub async fn test_get_queue_entries_from_db(&self) -> Result<Vec<crate::playback::queue_manager::QueueEntry>> {
        use crate::db::queue;

        let db_entries = queue::get_queue(&self.db_pool).await?;

        // Convert to playback QueueEntry
        let mut entries = Vec::new();
        for db_entry in db_entries {
            let queue_entry_id = Uuid::parse_str(&db_entry.guid)
                .map_err(|e| Error::Queue(format!("Invalid queue entry UUID: {}", e)))?;

            let passage_id = db_entry.passage_guid
                .as_ref()
                .map(|s| Uuid::parse_str(s))
                .transpose()
                .map_err(|e| Error::Queue(format!("Invalid passage UUID: {}", e)))?;

            entries.push(crate::playback::queue_manager::QueueEntry {
                queue_entry_id,
                passage_id,
                file_path: std::path::PathBuf::from(db_entry.file_path),
                play_order: db_entry.play_order,
                start_time_ms: db_entry.start_time_ms.map(|v| v as u64),
                end_time_ms: db_entry.end_time_ms.map(|v| v as u64),
                lead_in_point_ms: db_entry.lead_in_point_ms.map(|v| v as u64),
                lead_out_point_ms: db_entry.lead_out_point_ms.map(|v| v as u64),
                fade_in_point_ms: db_entry.fade_in_point_ms.map(|v| v as u64),
                fade_out_point_ms: db_entry.fade_out_point_ms.map(|v| v as u64),
                fade_in_curve: db_entry.fade_in_curve,
                fade_out_curve: db_entry.fade_out_curve,
                discovered_end_ticks: None,
            });
        }

        // Sort by play_order
        entries.sort_by_key(|e| e.play_order);

        Ok(entries)
    }

    /// Get in-memory queue entries (NOT from database)
    ///
    /// **[TEST-HARNESS]** For testing only - returns in-memory queue state
    /// This is what the decoder uses for priority selection via get_play_order_for_entry()
    #[doc(hidden)]
    pub async fn test_get_queue_entries_from_memory(&self) -> Vec<crate::playback::queue_manager::QueueEntry> {
        let queue = self.queue.read().await;
        let mut entries = Vec::new();

        // Get current
        if let Some(entry) = queue.current() {
            entries.push(entry.clone());
        }

        // Get next
        if let Some(entry) = queue.next() {
            entries.push(entry.clone());
        }

        // Get queued
        for entry in queue.queued() {
            entries.push(entry.clone());
        }

        entries
    }

    /// Get current decoder target (which buffer being filled)
    ///
    /// **[TEST-HARNESS]** For testing only
    #[doc(hidden)]
    pub async fn test_get_decoder_target(&self) -> Option<Uuid> {
        self.decoder_worker.test_get_current_target().await
    }

    /// Get chain assignments generation counters
    ///
    /// **[TEST-HARNESS]** For testing only
    ///
    /// Returns `(current_generation, last_observed_generation)`.
    /// When these differ, re-evaluation is pending.
    #[doc(hidden)]
    pub async fn test_get_generation_counter(&self) -> (u64, u64) {
        self.decoder_worker.test_get_generation().await
    }

    /// Wait for generation counter to change (re-evaluation occurred)
    ///
    /// **[TEST-HARNESS]** For testing only
    ///
    /// Returns `true` if generation changed within timeout, `false` if timeout.
    #[doc(hidden)]
    pub async fn test_wait_for_generation_change(&self, timeout_ms: u64) -> bool {
        let (initial_gen, _) = self.test_get_generation_counter().await;
        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_millis(timeout_ms);

        loop {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

            let (current_gen, _) = self.test_get_generation_counter().await;
            if current_gen != initial_gen {
                return true; // Generation changed (re-evaluation occurred)
            }

            if tokio::time::Instant::now() >= deadline {
                return false; // Timeout
            }
        }
    }

    /// Get current passage being played by mixer
    ///
    /// **[TEST-HARNESS][PLAN020 Phase 5]** For integration testing
    ///
    /// Returns `Some(queue_entry_id)` if mixer is playing, `None` if idle.
    #[doc(hidden)]
    pub async fn test_get_mixer_current_passage(&self) -> Option<Uuid> {
        let mixer = self.mixer.read().await;
        mixer.get_current_passage_id()
    }

    /// Simulate buffer fill and trigger ReadyForStart event
    ///
    /// **[TEST-HARNESS][PLAN020 Phase 5]** For integration testing
    ///
    /// Simulates buffer filling to threshold (3000ms) and emits BufferEvent::ReadyForStart
    /// to test event-driven mixer startup without requiring real decode.
    #[doc(hidden)]
    pub async fn test_simulate_buffer_fill(&self, queue_entry_id: Uuid, target_ms: u64) -> anyhow::Result<()> {
        use crate::playback::buffer_events::BufferEvent;

        // Calculate samples for target duration (44.1kHz stereo = 88200 samples/sec)
        let samples_buffered = ((target_ms as f64 / 1000.0) * 88200.0) as usize;

        // If target >= 3000ms, emit ReadyForStart event
        if target_ms >= 3000 {
            let event = BufferEvent::ReadyForStart {
                queue_entry_id,
                samples_buffered,
                buffer_duration_ms: target_ms,
            };

            // Emit via buffer manager's test method
            self.buffer_manager.test_emit_event(event).await;

            // Give event time to propagate
            tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
        }

        Ok(())
    }

    /// Simulate passage completion (for queue advance testing)
    ///
    /// **[TEST-HARNESS][PLAN020 Phase 5]** For integration testing
    ///
    /// Simulates passage completing playback by:
    /// 1. Removing the passage from queue (triggers queue advance)
    /// 2. Waiting for event-driven decode to trigger for promoted passages
    #[doc(hidden)]
    pub async fn test_simulate_passage_complete(&self, queue_entry_id: Uuid) -> anyhow::Result<()> {
        // Remove current passage (same as user skip or natural completion)
        // Note: remove_queue_entry returns bool, not Result
        let _removed = self.remove_queue_entry(queue_entry_id).await;

        // Give event system time to process queue advance and trigger decode
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        Ok(())
    }
}

// Note: Additional methods for PlaybackEngine are implemented in:
// - queue.rs: Queue operations (skip_next, clear_queue, enqueue_file, etc.)
// - diagnostics.rs: Diagnostics and monitoring (get_buffer_chains, event handlers, etc.)
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
