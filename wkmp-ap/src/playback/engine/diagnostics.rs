//! Diagnostics and monitoring module
//!
//! **Responsibilities:**
//! - Status accessors (get_volume_arc, get_buffer_manager, is_audio_expected, etc.)
//! - Monitoring configuration (set_buffer_monitor_rate, trigger_buffer_monitor_update)
//! - Event handlers (position_event_handler, buffer_event_handler)
//! - SSE emitters (buffer_chain_status_emitter, playback_position_emitter)
//! - Pipeline metrics (get_pipeline_metrics, get_buffer_chains, verify_queue_sync)
//!
//! **Traceability:**
//! - [REQ-DEBT-QUALITY-002-010] Diagnostics module for monitoring
//! - [SSD-ENG-020] Status reporting and diagnostics

use super::core::PlaybackEngine;
use crate::playback::buffer_manager::BufferManager;
use crate::playback::buffer_events::BufferEvent;
use crate::playback::events::PlaybackEvent;
use crate::state::CurrentPassage;
use sqlx::{Pool, Sqlite};
use std::sync::{Arc, Mutex};
use std::sync::atomic::Ordering;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

impl PlaybackEngine {
    /// Get shared volume Arc for testing
    ///
    /// Provides access to volume control for integration tests.
    /// Used to verify volume settings are correctly applied.
    ///
    /// # Returns
    /// Cloned Arc to volume (f32 in range 0.0-1.0)
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

    /// Get buffer chains with passage/queue metadata
    ///
    /// **[DBD-OV-080]** Returns all buffer chains (0-11) with associated passage data.
    ///
    /// # Returns
    /// Vector of BufferChainInfo for ALL chains (idle chains included)
    ///
    /// **[DEBT-007]** Some fields stubbed (decoder_state, fade_stage) pending pool integration
    pub async fn get_buffer_chains(&self) -> Vec<wkmp_common::events::BufferChainInfo> {
        use wkmp_common::events::BufferChainInfo;

        // **[DBD-PARAM-020]** Get working sample rate for timing calculations
        let sample_rate = *self.working_sample_rate.read().unwrap();

        // **[DBD-PARAM-050]** Loaded from settings (default: 12)
        let maximum_decode_streams = self.maximum_decode_streams;

        // Get mixer state to determine active passages
        // [SUB-INC-4B] Query SPEC016 mixer for current state
        let mixer = self.mixer.read().await;
        let current_passage_id = mixer.get_current_passage_id();
        let current_position_frames = mixer.get_current_tick() as usize;
        drop(mixer);

        // **[PLAN014]** Next passage and crossfade state tracked by mixer via markers
        // Telemetry currently reports only primary passage; future enhancement for next passage
        let next_passage_id: Option<Uuid> = None;
        let next_position_frames: usize = 0;
        let is_crossfading = false;

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
                    current_passage_id == entry.passage_id
                } else if queue_position == 1 {
                    next_passage_id == entry.passage_id
                } else {
                    false
                };

                let mixer_role = if queue_position == 0 {
                    if is_crossfading {
                        "Crossfading".to_string()
                    } else {
                        "Current".to_string()
                    }
                } else if queue_position == 1 {
                    if is_crossfading {
                        "Crossfading".to_string()
                    } else {
                        "Next".to_string()
                    }
                } else {
                    "Queued".to_string()
                };

                // Get playback position (only for current passage)
                let (playback_position_frames, playback_position_ms) = if queue_position == 0 {
                    (current_position_frames,
                     (current_position_frames as u64 * 1000) / sample_rate as u64)
                } else if queue_position == 1 && is_crossfading {
                    (next_position_frames,
                     (next_position_frames as u64 * 1000) / sample_rate as u64)
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

                    // Decoder stage - **[DEBT-007]** Requires decoder pool state exposure
                    // TODO: Add decoder_worker state tracking to expose per-chain decoder state
                    // Would need: decoder_pool.get_chain_state(queue_entry_id) -> Option<DecoderState>
                    decoder_state: None,
                    decode_progress_percent: None,
                    is_actively_decoding: None,

                    // Decoder telemetry **[REQ-DEBT-FUNC-001]**
                    decode_duration_ms: buffer_info.as_ref().and_then(|b| b.decode_duration_ms),
                    source_file_path: buffer_info.as_ref().and_then(|b| b.file_path.clone()),

                    // Resampler stage - **[DEBT-007]** Use actual source rate from decoder metadata
                    source_sample_rate: buffer_info.as_ref().and_then(|b| b.source_sample_rate),
                    resampler_active: buffer_info.as_ref()
                        .and_then(|b| b.source_sample_rate)
                        .map(|src_rate| src_rate != sample_rate),
                    target_sample_rate: {
                        debug!("[Chain {}] Setting target_sample_rate to {} Hz", chain_index, sample_rate);
                        sample_rate // **[DBD-PARAM-020]** working_sample_rate (device native)
                    },
                    resampler_algorithm: Some("Septic polynomial".to_string()), // **[SPEC020-MONITOR-070]** rubato FastFixedIn with Septic degree

                    // Fade stage - **[DEBT-007]** Requires fade state tracking in decoder_chain
                    // TODO: Add Fader::current_stage() method to expose FadeStage enum
                    // Would need: decoder_chain.fade_stage() -> Option<FadeStage>
                    fade_stage: None,

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
                    // **[DEBT-007]** Use passage_start_time for current passage timestamp
                    started_at: if is_active_in_mixer {
                        self.passage_start_time.read().await
                            .map(|instant| {
                                let elapsed = instant.elapsed();
                                let now = chrono::Utc::now();
                                let started = now - chrono::Duration::from_std(elapsed).unwrap_or_default();
                                started.to_rfc3339()
                            })
                    } else {
                        None
                    },
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
    /// true if synchronized, false if mismatch detected
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

    /// Get buffer statuses for all buffers
    ///
    /// Returns buffer statuses for all managed buffers.
    ///
    /// # Returns
    /// HashMap mapping queue_entry_id to BufferStatus
    pub async fn get_buffer_statuses(&self) -> std::collections::HashMap<Uuid, crate::audio::types::BufferStatus> {
        self.buffer_manager.get_all_statuses().await
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
        // [SUB-INC-4B] Replace get_total_frames_mixed() with get_frames_written()
        let mixer_total_frames_mixed = self.mixer.read().await.get_frames_written();

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

    // ========================================================================
    // EVENT HANDLERS (internal, spawned by start())
    // ========================================================================

    /// Position event handler - processes position updates and passage completions
    ///
    /// **[REV002]** Event-driven position tracking
    ///
    /// Handles:
    /// - PositionUpdate: Song boundary crossing, progress events
    /// - PassageComplete: Cleanup and queue advancement
    pub(super) async fn position_event_handler(&self) {
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
                    // **[DBD-PARAM-020]** Read working sample rate on each event (may change during runtime)
                    let sample_rate = *self.working_sample_rate.read().unwrap();
                    // [1] Check song boundary
                    let mut timeline = self.current_song_timeline.write().await;
                    if let Some(timeline) = timeline.as_mut() {
                        let (crossed, new_song_id) = timeline.check_boundary(position_ms);

                        if crossed {
                            // Get passage_id for event
                            let queue = self.queue.read().await;
                            let passage_id = queue.current()
                                .and_then(|e| e.passage_id)
                                .unwrap_or_else(uuid::Uuid::nil);
                            drop(queue);

                            // **[DEBT-005]** Fetch album UUIDs for CurrentSongChanged event
                            let song_albums = if passage_id != uuid::Uuid::nil() {
                                crate::db::passages::get_passage_album_uuids(&self.db_pool, passage_id)
                                    .await
                                    .unwrap_or_else(|e| {
                                        warn!("Failed to fetch album UUIDs for passage {}: {}", passage_id, e);
                                        Vec::new()
                                    })
                            } else {
                                Vec::new()
                            };

                            // Emit CurrentSongChanged event
                            info!(
                                "Song boundary crossed: new_song={:?}, position={}ms",
                                new_song_id, position_ms
                            );

                            self.state.broadcast_event(wkmp_common::events::WkmpEvent::CurrentSongChanged {
                                passage_id,
                                song_id: new_song_id,
                                song_albums,
                                position_ms,
                                timestamp: chrono::Utc::now(),
                            });
                        }
                    }
                    drop(timeline);

                    // [2] Update shared state on EVERY PositionUpdate (for 1s PlaybackPosition emissions)
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
                                    // If end_time is None, estimate from buffer stats (ephemeral passages)
                                    if let Some(buffer_ref) = self.buffer_manager.get_buffer(queue_entry_id).await {
                                        // No lock needed - stats() uses atomics
                                        let stats = buffer_ref.stats();
                                        // **[DBD-PARAM-020]** Use working sample rate (matches device)
                                        wkmp_common::timing::samples_to_ticks(stats.total_written as usize, sample_rate)
                                    } else {
                                        0
                                    }
                                };
                                let duration_ms = wkmp_common::timing::ticks_to_ms(duration_ticks) as u64;

                                // Update SharedState (used by 1s PlaybackPosition emitter)
                                let current_passage = CurrentPassage {
                                    queue_entry_id: current.queue_entry_id,
                                    passage_id: current.passage_id,
                                    position_ms,
                                    duration_ms,
                                };
                                self.state.set_current_passage(Some(current_passage)).await;

                                // [3] Check if PlaybackProgress interval elapsed (5s for analytics/logging)
                                if position_ms >= last_progress_position_ms + progress_interval_ms {
                                    last_progress_position_ms = position_ms;

                                    let passage_id = current.passage_id.unwrap_or_else(uuid::Uuid::nil);

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
                    } else {
                        // No current passage
                        self.state.set_current_passage(None).await;
                    }
                }

                Some(PlaybackEvent::PassageComplete { queue_entry_id }) => {
                    info!("PassageComplete event received for queue_entry_id: {}", queue_entry_id);

                    // **[REQ-QUEUE-DEDUP-010, REQ-QUEUE-DEDUP-020]** Deduplication check (5-second window)
                    let now = tokio::time::Instant::now();
                    let dedup_window = std::time::Duration::from_secs(5);

                    let is_duplicate = {
                        let mut completed = self.completed_passages.write().await;

                        // Clean up expired entries (>5 seconds old)
                        completed.retain(|_, &mut timestamp| {
                            now.duration_since(timestamp) < dedup_window
                        });

                        // Check if this is a duplicate
                        if let Some(&previous_timestamp) = completed.get(&queue_entry_id) {
                            let elapsed = now.duration_since(previous_timestamp);
                            debug!(
                                "Duplicate PassageComplete event for {} (previous event {:.1}ms ago) - skipping",
                                queue_entry_id,
                                elapsed.as_secs_f64() * 1000.0
                            );
                            true
                        } else {
                            // Record this completion
                            completed.insert(queue_entry_id, now);
                            false
                        }
                    };

                    // Skip duplicate events
                    if is_duplicate {
                        continue;
                    }

                    // Stop mixer and clear passage
                    {
                        let mut mixer = self.mixer.write().await;
                        mixer.clear_passage();
                        mixer.clear_all_markers();
                    }

                    // Remove completed entry from database + memory + emit events
                    // [SUB-INC-4B] Use shared helper to ensure consistent removal behavior
                    if let Err(e) = self.complete_passage_removal(
                        queue_entry_id,
                        wkmp_common::events::QueueChangeTrigger::PassageCompletion
                    ).await {
                        error!("Failed to complete passage removal: {}", e);
                    }

                    // Trigger watchdog check to ensure next passage starts if available
                    if let Err(e) = self.watchdog_check().await {
                        error!("Failed to run watchdog check after passage complete: {}", e);
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

    /// Buffer event handler - processes buffer state changes
    ///
    /// **[SSD-FLOW-030]** Event-driven mixer start on ReadyForStart
    ///
    /// Handles:
    /// - ReadyForStart: Instant mixer start when buffer ready
    /// - Exhausted: Emergency refill on underrun
    /// - EndpointDiscovered: Update queue with actual file length
    pub(super) async fn buffer_event_handler(&self) {
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

                    // **[PLAN020 FR-002]** Event-Driven Mixer Startup
                    // Check if this is the current passage in queue
                    let current = {
                        let queue = self.queue.read().await;
                        if queue.current().map(|c| c.queue_entry_id) != Some(queue_entry_id) {
                            debug!(
                                "Buffer ready for {} but not current passage, ignoring",
                                queue_entry_id
                            );
                            continue;
                        }
                        queue.current().cloned()
                    };

                    let current = match current {
                        Some(c) => c,
                        None => continue,
                    };

                    // Use extracted mixer startup method (shared with watchdog_check)
                    match self.start_mixer_for_current(&current).await {
                        Ok(true) => {
                            // Event-driven path succeeded - mixer was started
                            let elapsed = start_time.elapsed();
                            info!(
                                "✅ Mixer started in {:.2}ms (event-driven instant start)",
                                elapsed.as_secs_f64() * 1000.0
                            );
                        }
                        Ok(false) => {
                            // Mixer already playing or buffer not ready - not an error in event path
                            debug!("Event-driven mixer start: mixer already playing for {}", queue_entry_id);
                        }
                        Err(e) => {
                            warn!("Failed to start mixer for {}: {}", queue_entry_id, e);
                        }
                    }
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
    pub(super) async fn buffer_chain_status_emitter(&self) {
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

    /// Background task: Emit PlaybackPosition events every 1 second
    ///
    /// **[SSE-UI-030]** Playback Position Updates
    ///
    /// This background task runs continuously and emits PlaybackPosition
    /// events to SSE clients every 1 second during active playback.
    pub(super) async fn playback_position_emitter(&self) {
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
