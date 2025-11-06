//! Playback control methods for PlaybackEngine
//!
//! **Responsibilities:**
//! - Playback state transitions (play, pause, seek)
//! - Crossfade logic and timing
//! - Playback loop and watchdog monitoring
//! - Mixer initialization for passages
//!
//! **Traceability:**
//! - [REQ-DEBT-QUALITY-002-010] Playback methods extracted from core.rs
//! - [XFD-PAUS-010] Pause handling
//! - [XFD-PAUS-020] Resume with fade-in
//! - [API] Playback control endpoints (play, pause, seek)
//! - [PLAN021] Technical debt remediation (playback.rs extracted from core.rs)

use super::PlaybackEngine;
use crate::db::passages::{get_passage_album_uuids, PassageWithTiming};
use crate::error::{Error, Result};
use crate::playback::mixer::{MixerState, PositionMarker, MarkerEvent};
use crate::playback::queue_manager::QueueEntry;
use crate::playback::types::DecodePriority;
use crate::state::PlaybackState;
use std::sync::atomic::Ordering;
use tokio::time::{interval, Duration};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

impl PlaybackEngine {
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
                let sample_rate = *self.working_sample_rate.read().unwrap(); // [DBD-PARAM-020] Use device sample rate
                let fade_samples = ((fade_duration_ms as u64 * sample_rate as u64) / 1000) as usize;
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
            old_state,
            new_state: wkmp_common::events::PlaybackState::Playing,
            timestamp: chrono::Utc::now(),
        });

        // Also emit PlaybackProgress immediately
        if let Some(passage) = self.state.get_current_passage().await {
            self.state.broadcast_event(wkmp_common::events::WkmpEvent::PlaybackProgress {
                passage_id: passage.passage_id.unwrap_or_else(Uuid::nil),
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
            old_state,
            new_state: wkmp_common::events::PlaybackState::Paused,
            timestamp: chrono::Utc::now(),
        });

        // Also emit PlaybackProgress immediately
        if let Some(passage) = self.state.get_current_passage().await {
            self.state.broadcast_event(wkmp_common::events::WkmpEvent::PlaybackProgress {
                passage_id: passage.passage_id.unwrap_or_else(Uuid::nil),
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

    /// Skip forward 10 seconds in current passage
    ///
    /// **Traceability:** [REQ-SF-010] through [REQ-SF-050]
    /// **Requirements:**
    /// - [REQ-SF-020] 10-second skip with sample accuracy
    /// - [REQ-SF-030] Buffer availability validation (≥11s required)
    /// - [REQ-SF-040] Works in Playing/Paused states
    /// - [REQ-SF-050] Emits PlaybackProgress event
    ///
    /// [API] POST /playback/skip-forward
    ///
    /// # Returns
    /// Ok(new_position_ms) on success, Err with descriptive message on failure
    ///
    /// # Safety
    /// Validates that ≥11 seconds of audio are buffered before skipping to prevent underruns.
    pub async fn skip_forward(&self) -> Result<u64> {
        info!("Skip forward command received (10 seconds)");

        // Step 1: Get current queue entry
        let queue = self.queue.read().await;
        let current = queue.current().cloned();
        drop(queue);

        let current = match current {
            Some(c) => c,
            None => {
                return Err(Error::Playback("no passage currently playing".to_string()));
            }
        };

        // Step 2: Get buffer for current passage
        let buffer_ref = self.buffer_manager.get_buffer(current.queue_entry_id).await;
        let buffer = match buffer_ref {
            Some(b) => b,
            None => {
                return Err(Error::Playback(format!(
                    "buffer not available for queue_entry_id={}",
                    current.queue_entry_id
                )));
            }
        };

        // Step 3: Validate buffer availability
        // [REQ-SF-030] Require ≥11 seconds of buffered audio (10s skip + 1s safety margin)
        let sample_rate = *self.working_sample_rate.read().unwrap();
        let required_frames = ((11000 * sample_rate as u64) / 1000) as usize; // 11 seconds in frames
        let available_frames = buffer.occupied();

        if available_frames < required_frames {
            let available_seconds = (available_frames as f64) / (sample_rate as f64);
            return Err(Error::Playback(format!(
                "insufficient buffer - only {:.1}s available (need 11s)",
                available_seconds
            )));
        }

        // Step 4: Get current playback position
        let mixer = self.mixer.read().await;
        let current_tick = mixer.get_current_tick();
        drop(mixer);

        // Step 5: Calculate new position (10 seconds forward)
        // Convert ticks → ms → add 10s → pass to seek()
        let current_position_ms = wkmp_common::timing::ticks_to_ms(current_tick) as u64;
        let new_position_ms = current_position_ms + 10000; // Add 10 seconds

        // Step 6: Call existing seek() logic (reuse validation, clamping, events)
        self.seek(new_position_ms).await?;

        info!(
            "Skip forward complete: {}ms → {}ms",
            current_position_ms, new_position_ms
        );

        Ok(new_position_ms)
    }

    /// Seek to position in current passage
    ///
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
        let sample_rate = *self.working_sample_rate.read().unwrap(); // [DBD-PARAM-020] Use device sample rate

        // Convert milliseconds to frames
        let position_frames = ((position_ms as f32 / 1000.0) * sample_rate as f32) as usize;

        // Clamp to buffer bounds
        let stats = buffer.stats();
        let max_frames = stats.total_written as usize;
        let clamped_position = position_frames.min(max_frames.saturating_sub(1));

        // Calculate how many frames to skip forward from current position
        let mixer = self.mixer.read().await;
        let current_tick = mixer.get_current_tick();
        drop(mixer);

        // Convert ticks to frames using the proper conversion function
        let current_frames = wkmp_common::timing::ticks_to_samples(current_tick, sample_rate);
        let frames_to_skip = if clamped_position > current_frames {
            clamped_position - current_frames
        } else {
            // Seeking backwards not supported - would require buffer rewind
            warn!("Seek backwards not supported: current={}ms, requested={}ms",
                  (current_frames as f32 / sample_rate as f32 * 1000.0), position_ms);
            return Err(Error::Playback("Seek backwards not supported".to_string()));
        };

        // Actually skip frames in the buffer by discarding them
        debug!("Seeking: discarding {} frames from buffer ({}ms → {}ms)",
               frames_to_skip,
               (current_frames as f32 / sample_rate as f32 * 1000.0),
               (clamped_position as f32 / sample_rate as f32 * 1000.0));

        let mut frames_skipped = 0;
        for _ in 0..frames_to_skip {
            match buffer.pop_frame() {
                Ok(_) => frames_skipped += 1,
                Err(_) => {
                    // Buffer ran out before we could skip all frames
                    warn!("Buffer exhausted during seek: skipped {}/{} frames",
                          frames_skipped, frames_to_skip);
                    break;
                }
            }
        }

        // Update mixer position to reflect the actual skip
        // [SUB-INC-4B] Replace set_position() with set_current_passage()
        // Note: Marker recalculation deferred to Phase 4 (currently just updates position)
        let mut mixer = self.mixer.write().await;
        if let Some(passage_id) = current.passage_id {
            // Convert frames back to ticks for mixer
            let new_frames = current_frames + frames_skipped;
            let new_tick = wkmp_common::timing::samples_to_ticks(new_frames, sample_rate);
            mixer.set_current_passage(passage_id, current.queue_entry_id, new_tick);
            // **[PLAN014]** Marker recalculation from seek deferred to future enhancement
            // Current implementation: markers calculate from passage start; seek invalidates markers
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
                    // **[DBD-PARAM-020]** Use working sample rate (matches device)
                    wkmp_common::timing::samples_to_ticks(stats.total_written as usize, sample_rate)
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

    /// Update audio_expected flag based on current state
    pub(super) async fn update_audio_expected_flag(&self) {
        let state = self.state.get_playback_state().await;
        let queue = self.queue.read().await;
        let has_passages = !queue.is_empty();
        drop(queue);

        let expected = state == PlaybackState::Playing && has_passages;

        // Use Release ordering to ensure visibility across threads
        self.audio_expected.store(expected, Ordering::Release);

        debug!("Audio expected flag updated: {} (state={:?}, queue_len={})",
               expected, state, if has_passages { "non-empty" } else { "empty" });
    }

    /// Calculate crossfade start milliseconds from passage timing
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

        // Convert ticks to samples for mixer (calculated for validation, not used in marker-driven crossfade)
        let _fade_out_duration_samples = wkmp_common::timing::ticks_to_samples(
            fade_out_duration_ticks,
            44100 // STANDARD_SAMPLE_RATE
        );

        let _fade_in_duration_samples = wkmp_common::timing::ticks_to_samples(
            fade_in_duration_ticks,
            44100 // STANDARD_SAMPLE_RATE
        );

        // Start the crossfade!
        // [SUB-INC-4B] Crossfading now handled via StartCrossfade marker
        //
        // **Control Flow Explanation:**
        // 1. The StartCrossfade marker was added earlier in process_queue() when the current
        //    passage was loaded (see core.rs:1497-1501)
        // 2. The marker's tick position was calculated based on fade_out_point or lead_out
        //    (5 seconds before end if no explicit fade-out point)
        // 3. When the mixer reaches that tick during playback, it triggers this handler
        // 4. This handler broadcasts the CrossfadeStarted event and marks the next buffer as playing
        //
        // The actual crossfade mixing happens in the audio thread via fader multiplication,
        // not in this event handler. This just coordinates state transitions.
        info!("Crossfade trigger handled via StartCrossfade marker (added in process_queue)");

        // **[SSE-UI-040]** Emit CrossfadeStarted event
        self.state.broadcast_event(wkmp_common::events::WkmpEvent::CrossfadeStarted {
            from_passage_id: current.passage_id.unwrap_or_else(Uuid::nil),
            to_passage_id: next.passage_id.unwrap_or_else(Uuid::nil),
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
            passage_id: next.passage_id.unwrap_or_else(Uuid::nil),
            album_uuids,
            timestamp: chrono::Utc::now(),
        });

        Ok(true)
    }

    /// Main playback loop
    ///
    /// [SSD-FLOW-010] Core orchestration logic
    pub(super) async fn playback_loop(&self) -> Result<()> {
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

            // Watchdog check (detection-only safety net)
            self.watchdog_check().await?;
        }

        info!("Playback loop stopped");
        Ok(())
    }

    /// Watchdog check: detect and intervene when event-driven system fails
    ///
    /// **[PLAN020 Phase 4]** Converted from proactive orchestrator to detection-only safety net.
    /// Event-driven paths (enqueue, queue advance, startup restoration, buffer events) handle all normal operations.
    /// This watchdog detects stuck states and logs WARN when intervention is required.
    ///
    /// **Detection & Intervention:**
    /// - Decode failures (no buffer exists for current passage) - triggers emergency decode
    /// - Mixer startup failures (buffer ready but mixer not started) - starts mixer
    ///
    /// **Proactive Operations Removed:**
    /// - Regular decode triggering (now event-driven via enqueue/queue advance/startup)
    /// - See PLAN020 FR-001, FR-002 for event-driven implementation
    pub(super) async fn watchdog_check(&self) -> Result<()> {
        // **[ASYNC-LOCK-001]** Clone queue entries immediately to avoid holding lock across awaits
        // Holding RwLock read guard across await points can cause deadlocks when other tasks
        // (like SSE handlers) try to acquire read locks while a write lock request is pending
        let (current_entry, _next_entry, _queued_entries) = {
            let queue = self.queue.read().await;
            (
                queue.current().cloned(),
                queue.next().cloned(),
                queue.queued().to_vec(),
            )
        }; // Lock is dropped here

        // **[PLAN020 Phase 4]** Proactive decode operations removed - now event-driven
        // Event-driven path: enqueue_file(), complete_passage_removal(), and assign_chains_to_loaded_queue() trigger decode immediately
        // Watchdog role: Detection-only (check for missing buffers, WARN if event system failed)

        // **[PLAN020 Phase 4 Bug Fix]** Watchdog: Detect decode failures (detection-only)
        // Event-driven paths should trigger decode for all queue entries
        // If current passage has no buffer, decode was never triggered (event system failure)
        if let Some(current) = &current_entry {
            // Check if buffer exists for current passage
            if self.buffer_manager.get_buffer(current.queue_entry_id).await.is_none() {
                // No buffer exists - decode was never triggered or failed
                warn!(
                    "[WATCHDOG] Event system failure: No buffer exists for current passage {}. Triggering emergency decode...",
                    current.queue_entry_id
                );

                // Trigger decode as emergency intervention
                if let Err(e) = self.request_decode(current, DecodePriority::Immediate, true).await {
                    warn!("[WATCHDOG] Failed to trigger emergency decode for {}: {}", current.queue_entry_id, e);
                }

                // **[PLAN020 Phase 5]** Increment watchdog intervention counter and emit SSE event
                self.state.increment_watchdog_interventions();
                self.state.broadcast_event(wkmp_common::events::WkmpEvent::WatchdogIntervention {
                    intervention_type: "decode".to_string(),
                    interventions_total: self.state.get_watchdog_interventions(),
                    timestamp: chrono::Utc::now(),
                });
            }
        }

        // **[PLAN020 Phase 4]** Watchdog: Detect mixer startup failures (detection-only)
        // Event-driven path: BufferEvent::ReadyForStart → buffer_event_handler() → start_mixer_for_current()
        // Watchdog path: Detection-only safety net if event system fails
        // [SSD-MIX-030] Single passage playback initiation
        // [SSD-PBUF-028] Start playback with minimum buffer (3 seconds)
        if let Some(current) = &current_entry {
            // Check if buffer has minimum playback buffer available (3 seconds)
            const MIN_PLAYBACK_BUFFER_MS: u64 = 3000;
            let buffer_has_minimum = self.buffer_manager
                .has_minimum_playback_buffer(current.queue_entry_id, MIN_PLAYBACK_BUFFER_MS)
                .await;

            if buffer_has_minimum {
                debug!("Watchdog: Buffer has minimum playback buffer for {}", current.queue_entry_id);
                // Watchdog detection: Check if mixer should be playing but isn't
                match self.start_mixer_for_current(current).await {
                    Ok(true) => {
                        // Watchdog INTERVENED - event system failed to start mixer
                        warn!(
                            "[WATCHDOG] Event system failure: Buffer ready ({} ms) but mixer not started for {}. Intervening...",
                            MIN_PLAYBACK_BUFFER_MS, current.queue_entry_id
                        );
                        // **[PLAN020 Phase 5]** Increment watchdog intervention counter and emit SSE event
                        self.state.increment_watchdog_interventions();
                        self.state.broadcast_event(wkmp_common::events::WkmpEvent::WatchdogIntervention {
                            intervention_type: "mixer".to_string(),
                            interventions_total: self.state.get_watchdog_interventions(),
                            timestamp: chrono::Utc::now(),
                        });
                    }
                    Ok(false) => {
                        // No intervention needed - mixer already playing or buffer not ready
                        debug!("Watchdog: No intervention needed for {} (mixer already playing or buffer not found)", current.queue_entry_id);
                    }
                    Err(e) => {
                        warn!("[WATCHDOG] Failed to start mixer for {}: {}", current.queue_entry_id, e);
                    }
                }
            } else {
                debug!("Watchdog: Buffer does NOT have minimum playback buffer for {}", current.queue_entry_id);
            }
        }

        // Check for crossfade triggering
        // [XFD-IMPL-070] Crossfade initiation timing
        // [SSD-MIX-040] Crossfade state transition
        // [ISSUE-7] Refactored to use helper methods for reduced complexity
        let mixer = self.mixer.read().await;
        // [SUB-INC-4B] TODO: Track crossfade state in engine (marker-driven)
        let is_crossfading = false; // Stub: crossfading handled by markers now
        let current_passage_id = mixer.get_current_passage_id();
        // [SUB-INC-4B] Replace get_position() with get_current_tick()
        let mixer_position_frames = mixer.get_current_tick() as usize; // Convert tick to frames
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
                                // **[DBD-PARAM-020]** Use working sample rate (matches audio device)
                                let sample_rate = *self.working_sample_rate.read().unwrap();
                                let position_ms = (mixer_position_frames as u64 * 1000) / sample_rate as u64;

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

        // **[PLAN020 Phase 4]** Proactive decode for next/queued passages removed
        // Event-driven path handles all decode triggering:
        // - enqueue_file(): Triggers decode for newly enqueued passage (any position)
        // - complete_passage_removal(): Triggers decode for promoted passages
        // Watchdog detection would go here (TC-WD-003: missing next buffer)

        // **[XFD-COMP-010]** Check for crossfade completion BEFORE normal completion
        // When a crossfade completes, the outgoing passage has finished fading out
        // and the incoming passage continues playing as the new current passage.
        // We must advance the queue WITHOUT stopping the mixer to avoid interrupting
        // the already-playing incoming passage.
        //
        // **[XFD-COMP-020]** Critical: Do NOT call mixer.stop() in this path!
        // [SUB-INC-4B] Crossfade completion now handled via markers (PassageComplete event)
        // This legacy polling mechanism is obsolete with event-driven markers
        let crossfade_completed_id = {
            None::<Uuid> // Stub: crossfade completion detected via markers
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
                    // [SUB-INC-4B] passage_start_time now tracked in engine
                    let duration_played = {
                        if let Some(start_time) = *self.passage_start_time.read().await {
                            start_time.elapsed().as_secs_f64()
                        } else {
                            0.0
                        }
                    };

                    // **[Event-PassageCompleted]** Emit completion event for OUTGOING passage
                    self.state.broadcast_event(wkmp_common::events::WkmpEvent::PassageCompleted {
                        passage_id: passage_id_opt.unwrap_or_else(Uuid::nil),
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

                    // Remove completed passage from database to keep in sync (idempotent)
                    match crate::db::queue::remove_from_queue(&self.db_pool, completed_id).await {
                        Ok(was_removed) => {
                            if was_removed {
                                info!("Queue advanced (crossfade) and synced to database (removed {})", completed_id);
                            } else {
                                debug!("Queue entry {} already removed during crossfade", completed_id);
                            }
                        }
                        Err(e) => {
                            warn!("Failed to remove completed passage from database: {}", e);
                        }
                    }

                    // Update audio_expected flag
                    self.update_audio_expected_flag().await;

                    // Try to assign freed chain to unassigned entries (now that queue is consistent)
                    // **[DBD-DEC-045]** Must happen AFTER advance() to avoid reassigning to deleted entry
                    self.assign_chains_to_unassigned_entries().await;

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
        // [SUB-INC-4B] Passage completion now handled via PassageComplete marker event
        // This polling mechanism is obsolete with event-driven markers
        let is_finished = false; // Stub: completion detected via markers

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
                // [SUB-INC-4B] passage_start_time now tracked in engine
                let duration_played = {
                    if let Some(start_time) = *self.passage_start_time.read().await {
                        start_time.elapsed().as_secs_f64()
                    } else {
                        0.0
                    }
                };

                // Emit PassageCompleted event
                // [Event-PassageCompleted] Passage playback finished
                self.state.broadcast_event(wkmp_common::events::WkmpEvent::PassageCompleted {
                    passage_id: current_pid.unwrap_or_else(Uuid::nil),
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

                // Remove completed passage from database to keep in sync (idempotent)
                if let Some(completed_id) = completed_queue_entry_id {
                    match crate::db::queue::remove_from_queue(&self.db_pool, completed_id).await {
                        Ok(was_removed) => {
                            if was_removed {
                                info!("Queue advanced and synced to database (removed {})", completed_id);
                            } else {
                                debug!("Queue entry {} already removed", completed_id);
                            }
                        }
                        Err(e) => {
                            // Log error but don't fail - queue already advanced in memory
                            warn!("Failed to remove completed passage from database queue: {} (continuing anyway)", e);
                        }
                    }
                } else {
                    info!("Queue advanced to next passage");
                }

                // Update audio_expected flag for ring buffer underrun classification
                // This ensures TRACE logging when queue becomes empty after passage finishes
                self.update_audio_expected_flag().await;

                // Try to assign freed chain to unassigned entries (now that queue is consistent)
                // **[DBD-DEC-045]** Must happen AFTER advance() to avoid reassigning to deleted entry
                self.assign_chains_to_unassigned_entries().await;

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

    /// Start mixer for current passage (event-driven mixer startup)
    ///
    /// **[PLAN020 FR-002]** Event-Driven Mixer Startup
    /// Extracted from watchdog_check() to enable event-driven triggering via BufferEvent::ReadyForStart
    ///
    /// Starts playback when buffer reaches minimum threshold (3 seconds).
    /// Handles:
    /// - Mixer state initialization
    /// - Song timeline loading
    /// - Position markers
    /// - Crossfade markers
    /// - Fade-in curves
    /// - PassageStarted event emission
    ///
    /// # Arguments
    /// * `current` - Current queue entry to start playing
    ///
    /// # Returns
    /// * `Ok(())` - Mixer started successfully
    /// * `Err(_)` - Mixer startup failed
    /// Start mixer for current passage (event-driven mixer startup)
    ///
    /// **[PLAN020 FR-002]** Event-Driven Mixer Startup
    /// Extracted from watchdog_check() to enable event-driven triggering via BufferEvent::ReadyForStart
    ///
    /// **Returns:** `Ok(true)` if mixer was started, `Ok(false)` if mixer already playing or buffer not ready
    pub(super) async fn start_mixer_for_current(&self, current: &QueueEntry) -> Result<bool> {
        // Check if mixer is already playing (prevent duplicate start)
        let mixer = self.mixer.read().await;
        let mixer_idle = mixer.get_current_passage_id().is_none();
        let mixer_passage_id = mixer.get_current_passage_id();
        drop(mixer);

        debug!("start_mixer_for_current: mixer_idle={}, mixer_passage_id={:?}, queue_entry_id={}",
               mixer_idle, mixer_passage_id, current.queue_entry_id);

        if !mixer_idle {
            debug!("Mixer already playing passage {:?} - skipping start for {}", mixer_passage_id, current.queue_entry_id);
            return Ok(false); // Not an intervention - mixer already running
        }

        // Get buffer from buffer manager
        if self.buffer_manager.get_buffer(current.queue_entry_id).await.is_none() {
            warn!("Buffer not found for {} - cannot start mixer", current.queue_entry_id);
            return Ok(false); // Non-fatal: watchdog will retry if needed
        }

        debug!("start_mixer_for_current: Buffer found for {}, proceeding with mixer startup", current.queue_entry_id);

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

                        // **[DEBT-005]** Fetch album UUIDs for CurrentSongChanged event
                        let song_albums = crate::db::passages::get_passage_album_uuids(&self.db_pool, passage_id)
                            .await
                            .unwrap_or_else(|e| {
                                warn!("Failed to fetch album UUIDs for passage {}: {}", passage_id, e);
                                Vec::new()
                            });

                        self.state.broadcast_event(wkmp_common::events::WkmpEvent::CurrentSongChanged {
                            passage_id,
                            song_id: initial_song_id,
                            song_albums,
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
            current.passage_id.unwrap_or_else(Uuid::nil),
            current.queue_entry_id,
            fade_in_duration_samples,
            fade_in_duration_ticks
        );

        // Start mixer
        // [SUB-INC-4B] Replace start_passage() with set_current_passage() + markers
        {
            let mut mixer = self.mixer.write().await;

            // Use passage_id or Uuid::nil() for ephemeral passages
            let mixer_passage_id = current.passage_id.unwrap_or_else(Uuid::nil);
            mixer.set_current_passage(mixer_passage_id, current.queue_entry_id, 0);

            // [SUB-INC-4B] Calculate and add markers
            // For ephemeral passages (passage_id = None), only add position/complete markers
            // Per SPEC007: ephemeral passages have no crossfade (all lead/fade points = 0)

            // 1. Position update markers (configurable interval from settings)
            // **[DEBT-004]** Load from settings (default: 1000ms, range: 100-5000ms)
            let position_interval_ms = self.position_interval_ms as i64;
            let position_interval_ticks = wkmp_common::timing::ms_to_ticks(position_interval_ms);

            // Calculate passage duration in ticks
            // Default to 24 hours for ephemeral passages (safety fallback - EOF detection is primary mechanism)
            let passage_end_ticks = passage.end_time_ticks.unwrap_or(passage.fade_out_point_ticks.unwrap_or(passage.lead_out_point_ticks.unwrap_or(passage.start_time_ticks + wkmp_common::timing::ms_to_ticks(86_400_000)))); // Default 24hr max
            let passage_duration_ticks = passage_end_ticks.saturating_sub(passage.start_time_ticks);

            // Add position update markers
            let marker_count = (passage_duration_ticks / position_interval_ticks) as usize;
            for i in 1..=marker_count {
                let tick = i as i64 * position_interval_ticks;
                let position_ms = wkmp_common::timing::ticks_to_ms(tick) as u64;

                mixer.add_marker(PositionMarker {
                    tick,
                    passage_id: mixer_passage_id,
                    event_type: MarkerEvent::PositionUpdate { position_ms },
                });
            }

            // Get next entry for crossfade marker
            let next_entry = {
                let queue = self.queue.read().await;
                queue.next().cloned()
            };

            // 2. Crossfade marker (if next passage exists AND current is not ephemeral)
            if current.passage_id.is_some() {
                if let Some(ref _next) = next_entry {
                    // Calculate crossfade start point
                    let fade_out_start_tick = if let Some(fade_out_ticks) = passage.fade_out_point_ticks {
                        fade_out_ticks.saturating_sub(passage.start_time_ticks)
                    } else {
                        // No explicit fade-out point, use lead-out - 5 seconds
                        let lead_out_tick = passage.lead_out_point_ticks.unwrap_or(passage_duration_ticks);
                        lead_out_tick.saturating_sub(wkmp_common::timing::ms_to_ticks(5000))
                    };

                    // Only add if there's a next passage in queue
                    if let Some(next_passage_id) = next_entry.as_ref().and_then(|n| n.passage_id) {
                        mixer.add_marker(PositionMarker {
                            tick: fade_out_start_tick,
                            passage_id: mixer_passage_id,
                            event_type: MarkerEvent::StartCrossfade { next_passage_id },
                        });

                        debug!(
                            "Added crossfade marker at tick {} for passage {} → {}",
                            fade_out_start_tick, mixer_passage_id, next_passage_id
                        );
                    }
                }
            } else {
                debug!("Ephemeral passage - skipping crossfade marker (no crossfade per SPEC007)");
            }

            // 3. Passage complete marker (at fade-out end)
            let complete_tick = if let Some(fade_out_ticks) = passage.fade_out_point_ticks {
                fade_out_ticks.saturating_sub(passage.start_time_ticks)
            } else {
                passage_duration_ticks
            };

            mixer.add_marker(PositionMarker {
                tick: complete_tick,
                passage_id: mixer_passage_id,
                event_type: MarkerEvent::PassageComplete,
            });

            debug!(
                "Added markers for passage {}: {} position updates, complete at tick {}",
                mixer_passage_id, marker_count, complete_tick
            );

            // 4. Handle fade-in via start_resume_fade() if needed
            if fade_in_duration_samples > 0 {
                mixer.start_resume_fade(fade_in_duration_samples, fade_in_curve);
                debug!(
                    "Started fade-in: {} samples, curve: {:?}",
                    fade_in_duration_samples, fade_in_curve
                );
            }

            // Set passage start time
            *self.passage_start_time.write().await = Some(tokio::time::Instant::now());
        }

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
            passage_id: current.passage_id.unwrap_or_else(Uuid::nil),
            album_uuids,
            timestamp: chrono::Utc::now(),
        });

        info!("⚡ Mixer started for passage {} (event-driven)", current.queue_entry_id);

        Ok(true) // Mixer was started successfully
    }
}
