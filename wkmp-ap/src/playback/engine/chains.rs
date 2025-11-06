// ! Buffer chain management for PlaybackEngine
//!
//! **Responsibilities:**
//! - Assign decoder-buffer chains to queue entries
//! - Release chains when passages complete
//! - Manage chain availability pool
//!
//! **Traceability:**
//! - [REQ-DEBT-QUALITY-002-010] Chain management methods extracted from core.rs
//! - [DBD-LIFECYCLE-020] Chain assignment on queue entry addition
//! - [DBD-LIFECYCLE-030] Chain release on passage completion
//! - [DBD-DEC-045] Decoder cancellation on chain release

use super::PlaybackEngine;
use crate::playback::types::DecodePriority;
use std::cmp::Reverse;
use std::collections::HashMap;
use tracing::{debug, info, warn};
use uuid::Uuid;

impl PlaybackEngine {
    /// Assign chains to all entries in the loaded queue
    ///
    /// **[DBD-LIFECYCLE-020]** Initial chain assignment after database queue load
    /// **[PLAN020 Phase 4 Fix]** Triggers decode for loaded entries (event-driven path)
    ///
    /// Called during engine initialization to assign decoder-buffer chains to all
    /// queue entries loaded from the database. This ensures every entry has a chain
    /// before playback begins.
    ///
    /// Unlike enqueue_passage() which handles newly added entries, this method provides
    /// uniform handling of chain assignment regardless of enqueue source.
    pub async fn assign_chains_to_loaded_queue(&self) {
        debug!("🔍 assign_chains_to_loaded_queue: START");
        let queue = self.queue.read().await;
        debug!("🔍 assign_chains_to_loaded_queue: Acquired queue read lock");

        // Collect all queue entry IDs with position for decode priority
        let mut queue_entry_ids = Vec::new();
        let mut queue_entry_ids_with_position: Vec<(usize, Uuid)> = Vec::new();

        if let Some(current) = queue.current() {
            debug!("🔍 assign_chains_to_loaded_queue: Found current entry: {}", current.queue_entry_id);
            queue_entry_ids.push(current.queue_entry_id);
            queue_entry_ids_with_position.push((0, current.queue_entry_id)); // Position 0 = Current
        }
        if let Some(next) = queue.next() {
            debug!("🔍 assign_chains_to_loaded_queue: Found next entry: {}", next.queue_entry_id);
            queue_entry_ids.push(next.queue_entry_id);
            queue_entry_ids_with_position.push((1, next.queue_entry_id)); // Position 1 = Next
        }
        for (queue_idx, entry) in queue.queued().iter().enumerate() {
            debug!("🔍 assign_chains_to_loaded_queue: Found queued entry: {}", entry.queue_entry_id);
            queue_entry_ids.push(entry.queue_entry_id);
            queue_entry_ids_with_position.push((2 + queue_idx, entry.queue_entry_id)); // Position 2+ = Queued
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

        // **[PLAN020 Phase 4 Fix]** Trigger decode for loaded queue entries
        // Queue entries loaded from database need decode triggered (event-driven path)
        // Before PLAN020: watchdog proactively triggered decode
        // After PLAN020: must explicitly trigger decode for loaded entries
        debug!("🔍 assign_chains_to_loaded_queue: Triggering decode for loaded entries");
        for (idx, (position, queue_entry_id)) in queue_entry_ids_with_position.iter().enumerate() {
            // Get the entry to pass to request_decode
            let entry = {
                let queue = self.queue.read().await;
                // Check current, next, and queued
                if let Some(current) = queue.current() {
                    if current.queue_entry_id == *queue_entry_id {
                        Some(current.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
                .or_else(|| {
                    if let Some(next) = queue.next() {
                        if next.queue_entry_id == *queue_entry_id {
                            Some(next.clone())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .or_else(|| {
                    queue.queued().iter().find(|e| e.queue_entry_id == *queue_entry_id).cloned()
                })
            };

            if let Some(entry) = entry {
                // Map position to priority
                let priority = match position {
                    0 => DecodePriority::Immediate, // Current
                    1 => DecodePriority::Next,      // Next
                    _ => DecodePriority::Prefetch,  // Queued
                };

                debug!("🔍 assign_chains_to_loaded_queue: Triggering decode for entry {}/{}: {} (priority: {:?})",
                       idx + 1, count, queue_entry_id, priority);

                // Trigger decode (non-fatal if it fails - watchdog will retry)
                if let Err(e) = self.request_decode(&entry, priority, true).await {
                    warn!("Failed to trigger decode for loaded entry {}: {}", queue_entry_id, e);
                }
            }
        }

        debug!("🔍 assign_chains_to_loaded_queue: DONE");
    }

    /// Assign a decoder-buffer chain to a queue entry
    ///
    /// **[DBD-LIFECYCLE-020]** Chain assignment on queue entry addition
    ///
    /// Assigns one of the N available decoder-buffer chains (N = maximum_decode_streams)
    /// to the given queue entry. If all chains are in use, returns None.
    ///
    /// # Returns
    /// * `Some(chain_index)` - Chain index assigned (0..maximum_decode_streams-1)
    /// * `None` - No chains available (all maximum_decode_streams chains in use)
    pub(super) async fn assign_chain(&self, queue_entry_id: Uuid) -> Option<usize> {
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
    pub(super) async fn release_chain(&self, queue_entry_id: Uuid) {
        // **[DBD-DEC-045]** Cancel decode in decoder_worker first
        // This removes the DecoderChain from active/yielded sets and removes buffer
        self.decoder_worker.cancel_decode(queue_entry_id).await;

        // Then release the chain index assignment
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
        drop(assignments);

        // **[DBD-DEC-045]** DO NOT call assign_chains_to_unassigned_entries() here!
        // Callers must ensure queue state is consistent before reassigning chains.
        // If called here, we may reassign to entries that are being removed.
        // See queue.rs:292 for proper placement after complete_passage_removal().
    }

    /// Assign chains to queue entries that don't have them yet
    ///
    /// Called after releasing chains to assign newly available chains to passages
    /// that were enqueued when all chains were in use.
    ///
    /// **[DBD-DEC-045]** IMPORTANT: Only call this after queue state is consistent!
    /// Do NOT call during removal operations while entry is still in queue.
    pub(super) async fn assign_chains_to_unassigned_entries(&self) {
        let queue = self.queue.read().await;
        let mut unassigned_ids = Vec::new();

        // Collect all queue entry IDs
        let mut all_entries = Vec::new();
        if let Some(current) = queue.current() {
            all_entries.push(current.queue_entry_id);
        }
        if let Some(next) = queue.next() {
            all_entries.push(next.queue_entry_id);
        }
        all_entries.extend(queue.queued().iter().map(|e| e.queue_entry_id));

        debug!("Checking for unassigned entries: {} total queue entries", all_entries.len());
        drop(queue);

        // Check which ones don't have chain assignments
        let assignments = self.chain_assignments.read().await;
        for queue_entry_id in &all_entries {
            if !assignments.contains_key(queue_entry_id) {
                debug!("Found unassigned entry: {}", queue_entry_id);
                unassigned_ids.push(*queue_entry_id);
            }
        }
        drop(assignments);

        if unassigned_ids.is_empty() {
            debug!("No unassigned entries found");
            return;
        }

        debug!("Attempting to assign chains to {} unassigned entries", unassigned_ids.len());

        // Assign chains to unassigned entries (up to available chain limit)
        for queue_entry_id in unassigned_ids {
            if self.assign_chain(queue_entry_id).await.is_some() {
                info!("Assigned newly available chain to queue_entry={}", queue_entry_id);
                // Note: Decode request will be submitted on next process_queue() tick
            } else {
                // Still no chains available - they'll get assigned when a chain is released
                break;
            }
        }
    }

    /// Get chain assignments map
    ///
    /// **[TEST-HARNESS]** For testing only
    #[doc(hidden)]
    pub async fn test_get_chain_assignments(&self) -> HashMap<Uuid, usize> {
        self.chain_assignments.read().await.clone()
    }

    /// Get available chain indexes
    ///
    /// **[TEST-HARNESS]** For testing only
    #[doc(hidden)]
    pub async fn test_get_available_chains(&self) -> Vec<usize> {
        self.available_chains.read().await
            .iter()
            .map(|Reverse(idx)| *idx)
            .collect()
    }
}
