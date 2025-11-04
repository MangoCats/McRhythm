//! Queue operations module
//!
//! **Responsibilities:**
//! - Queue mutations (enqueue, skip, clear, remove, reorder)
//! - Queue queries (queue_len, get_queue_entries)
//! - Queue event emission (QueueChanged, QueueIndexChanged)
//! - Passage removal cleanup (buffer release, event emission)
//!
//! **Traceability:**
//! - [REQ-DEBT-QUALITY-002-010] Queue module for queue operations
//! - [SSD-ENG-020] Queue processing

use super::core::PlaybackEngine;
use crate::error::{Error, Result};
use crate::db::passages::{create_ephemeral_passage, get_passage_album_uuids, validate_passage_timing};
use crate::playback::queue_manager::QueueEntry;
use uuid::Uuid;
use std::path::PathBuf;
use tracing::{debug, error, info, warn};

impl PlaybackEngine {
    /// Skip to next passage in queue
    ///
    /// [API] POST /playback/skip
    /// Skips current passage, emits PassageCompleted (completed=false), and starts next.
    ///
    /// [SSD-ENG-024] Skip to next passage
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
        // [SUB-INC-4B] passage_start_time now tracked in engine
        let duration_played = {
            if let Some(start_time) = *self.passage_start_time.read().await {
                start_time.elapsed().as_secs_f64()
            } else {
                0.0
            }
        };

        // Emit PassageCompleted event with completed=false (skipped)
        self.state.broadcast_event(wkmp_common::events::WkmpEvent::PassageCompleted {
            passage_id: current.passage_id.unwrap_or_else(Uuid::nil),
            album_uuids,
            duration_played,
            completed: false, // false = skipped
            timestamp: chrono::Utc::now(),
        });

        // Mark buffer as exhausted
        self.buffer_manager.mark_exhausted(current.queue_entry_id).await;

        // Stop mixer immediately
        // [SUB-INC-4B] Replace stop() with SPEC016 operations (don't set to Paused - let process_queue handle state)
        let mut mixer = self.mixer.write().await;
        mixer.clear_all_markers();
        mixer.clear_passage();
        drop(mixer);

        // Remove buffer from memory
        if let Some(passage_id) = current.passage_id {
            self.buffer_manager.remove(passage_id).await;
        }

        info!("Mixer stopped and buffer cleaned up");

        // **[DBD-LIFECYCLE-020]** Release decoder-buffer chain before removing from queue
        // Implements requirement that chains are freed when passage is removed (skip counts as removal)
        self.release_chain(current.queue_entry_id).await;

        // Remove skipped entry from database + memory + emit events
        // [SUB-INC-4B] Use shared helper to ensure consistent removal behavior
        if let Err(e) = self.complete_passage_removal(
            current.queue_entry_id,
            wkmp_common::events::QueueChangeTrigger::UserDequeue
        ).await {
            error!("Failed to complete passage removal: {}", e);
        }

        // Try to assign freed chain to unassigned entries (now that queue is consistent)
        // **[DBD-DEC-045]** Must happen AFTER removal to avoid reassigning to deleted entry
        self.assign_chains_to_unassigned_entries().await;

        // Start next passage if available
        // [SUB-INC-4B] Call process_queue to trigger playback of next entry
        if let Err(e) = self.process_queue().await {
            error!("Failed to process queue after skip: {}", e);
        }

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
        // [SUB-INC-4B] Replace stop() with SPEC016 operations (don't set to Paused - queue is empty)
        let mut mixer = self.mixer.write().await;
        let passage_id_before_stop = mixer.get_current_passage_id();
        mixer.clear_all_markers();
        mixer.clear_passage();
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

        // Emit both queue change events (queue is now empty)
        // [SUB-INC-4B] Use helper to ensure both events always emitted together
        self.emit_queue_change_events(wkmp_common::events::QueueChangeTrigger::UserDequeue).await;

        info!("Queue cleared successfully");

        Ok(())
    }

    /// Enqueue file for playback
    ///
    /// [API] POST /playback/queue/enqueue
    /// Creates ephemeral passage and adds to queue
    ///
    /// # Arguments
    /// * `file_path` - Path to audio file
    ///
    /// # Returns
    /// * `Ok(queue_entry_id)` - UUID of created queue entry
    /// * `Err` if file doesn't exist or invalid
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
            passage.end_time_ticks.map(wkmp_common::timing::ticks_to_ms),
            Some(wkmp_common::timing::ticks_to_ms(passage.lead_in_point_ticks)),
            passage.lead_out_point_ticks.map(wkmp_common::timing::ticks_to_ms),
            Some(wkmp_common::timing::ticks_to_ms(passage.fade_in_point_ticks)),
            passage.fade_out_point_ticks.map(wkmp_common::timing::ticks_to_ms),
            Some(passage.fade_in_curve.to_db_string().to_string()),
            Some(passage.fade_out_curve.to_db_string().to_string()),
        )
        .await?;

        // **BUG FIX #3:** Get the play_order assigned by database
        // Previously hardcoded to 0, causing all newly enqueued passages to have same priority
        // This broke decoder priority selection, causing haphazard buffer filling order
        // **[DBD-DEC-045]** Decoder uses get_play_order_for_entry() which reads in-memory queue
        let db_entry = crate::db::queue::get_queue_entry_by_id(&self.db_pool, queue_entry_id).await?;
        let assigned_play_order = db_entry.play_order;

        debug!(
            "Enqueued {} with play_order={} (assigned by database)",
            queue_entry_id, assigned_play_order
        );

        // Add to in-memory queue
        // Convert ticks to milliseconds for queue entry (matches database format)
        let entry = QueueEntry {
            queue_entry_id,
            passage_id: passage.passage_id,
            file_path,
            play_order: assigned_play_order, // Use play_order from database
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
            // [SUB-INC-4B] Replace stop() with SPEC016 operations (don't set to Paused - let process_queue handle state)
            {
                let mut mixer = self.mixer.write().await;
                mixer.clear_all_markers();
                mixer.clear_passage();
            }
            info!("Mixer cleared for removed passage");

            // 3. Remove from queue + emit events
            // [SUB-INC-4B] Use shared helper for consistent removal behavior
            // Note: API handler also emits events, but using helper ensures internal consistency
            let removed = match self.complete_passage_removal(
                queue_entry_id,
                wkmp_common::events::QueueChangeTrigger::UserDequeue
            ).await {
                Ok(result) => result,
                Err(e) => {
                    error!("Failed to complete passage removal: {}", e);
                    false
                }
            };

            if removed {
                // 4. Try to assign freed chain to unassigned entries (now that queue is consistent)
                // **[DBD-DEC-045]** Must happen AFTER removal to avoid reassigning to deleted entry
                self.assign_chains_to_unassigned_entries().await;

                // 5. Start next passage if queue has one
                // [REQ-FIX-050] Start next passage if queue non-empty
                // [REQ-FIX-080] New passage starts correctly after removal
                let has_current = self.queue.read().await.current().is_some();
                if has_current {
                    info!("Starting next passage after current removed");
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

            // Persist to database first (queue state persistence principle)
            if let Err(e) = crate::db::queue::remove_from_queue(&self.db_pool, queue_entry_id).await {
                error!("Failed to remove entry from database: {}", e);
            }

            // Remove from in-memory queue
            let removed = self.queue.write().await.remove(queue_entry_id);

            if removed {
                info!("Successfully removed queue entry {} from in-memory queue", queue_entry_id);
                // Update audio_expected flag for ring buffer underrun classification
                self.update_audio_expected_flag().await;

                // Release chain if assigned (chain cleanup happens per-item)
                self.release_chain(queue_entry_id).await;

                // Try to assign freed chain to unassigned entries (now that queue is consistent)
                // **[DBD-DEC-045]** Must happen AFTER removal to avoid reassigning to deleted entry
                self.assign_chains_to_unassigned_entries().await;
            } else {
                warn!("Queue entry {} not found in in-memory queue", queue_entry_id);
            }

            removed
        }
    }

    /// Reorder queue entry position
    ///
    /// [API] PATCH /playback/queue/{queue_entry_id}
    /// Changes play order of queued passage
    ///
    /// # Arguments
    /// * `queue_entry_id` - UUID of entry to reorder
    /// * `new_position` - New position in queue (0-based)
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

    /// Get queue length
    ///
    /// Returns count of all queue entries (current + next + queued)
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

    // ========== Helper Functions (Internal) ==========

    /// Emit queue change events (QueueChanged + QueueStateUpdate)
    ///
    /// Helper to emit both required queue change events atomically.
    /// Every queue modification MUST emit both events for proper UI updates.
    ///
    /// **[SUB-INC-4B]** Consolidates duplicate event emission pattern
    ///
    /// # Arguments
    /// * `trigger` - Queue change trigger type (UserEnqueue, UserDequeue, PassageCompletion, etc.)
    async fn emit_queue_change_events(&self, trigger: wkmp_common::events::QueueChangeTrigger) {
        let queue_entries = self.get_queue_entries().await;
        let queue_ids: Vec<Uuid> = queue_entries.iter()
            .filter_map(|e| e.passage_id)
            .collect();

        // Emit QueueChanged (for tracking/analytics)
        self.state.broadcast_event(wkmp_common::events::WkmpEvent::QueueChanged {
            queue: queue_ids,
            trigger,
            timestamp: chrono::Utc::now(),
        });

        // Emit QueueStateUpdate (for UI display)
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
    }

    /// Complete passage removal: database + memory + events
    ///
    /// Performs the standard passage removal workflow used by skip, PassageComplete,
    /// and remove_queue_entry. Consolidates duplicate code and ensures consistent
    /// behavior across all removal paths.
    ///
    /// **[SUB-INC-4B]** Implements marker-driven queue advancement pattern
    ///
    /// # Arguments
    /// * `queue_entry_id` - UUID of queue entry to remove
    /// * `trigger` - Event trigger type (PassageCompletion or UserAction)
    ///
    /// # Returns
    /// * `Ok(true)` - Entry removed successfully
    /// * `Ok(false)` - Entry not found in queue
    /// * `Err(_)` - Database or broadcast error (non-fatal, logged)
    pub(super) async fn complete_passage_removal(
        &self,
        queue_entry_id: Uuid,
        trigger: wkmp_common::events::QueueChangeTrigger,
    ) -> Result<bool> {
        // Remove from database FIRST (persistence before memory)
        // [SUB-INC-4B] Persist removal to database (matches PassageComplete handler)
        if let Err(e) = crate::db::queue::remove_from_queue(&self.db_pool, queue_entry_id).await {
            error!("Failed to remove entry from database: {}", e);
        }

        // Remove from in-memory queue
        // [SUB-INC-4B] Same logic as PassageComplete handler
        let removed = {
            let mut queue_write = self.queue.write().await;
            queue_write.remove(queue_entry_id)
        };

        if !removed {
            warn!("Failed to remove queue entry from memory: {}", queue_entry_id);
            return Ok(false);
        }

        info!("Removed queue entry: {}", queue_entry_id);

        // Update audio_expected flag for ring buffer underrun classification
        self.update_audio_expected_flag().await;

        // Emit both queue change events (QueueChanged + QueueStateUpdate)
        // [SUB-INC-4B] Use helper to ensure both events always emitted together
        self.emit_queue_change_events(trigger).await;

        Ok(true)
    }
}
