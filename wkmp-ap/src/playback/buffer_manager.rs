//! Buffer Manager
//!
//! Manages passage buffer lifecycle (full vs. partial decode strategy).
//!
//! **Traceability:**
//! - [SSD-BUF-010] Buffer management strategy
//! - [SSD-FBUF-010] Full decode strategy
//! - [SSD-PBUF-010] Partial buffer strategy (15 seconds)
//! - [SSD-BUF-020] Buffer state tracking
//! - [PERF-POLL-010] Event-driven buffer readiness notification

use crate::audio::types::{BufferStatus, PassageBuffer};
use crate::playback::types::BufferEvent;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

/// Wrapper for buffer with metadata
struct ManagedBuffer {
    /// The actual passage buffer
    buffer: Arc<RwLock<PassageBuffer>>,

    /// Current buffer status
    status: BufferStatus,

    /// When decode started
    decode_started: Instant,

    /// Whether we've already sent ReadyForStart notification for this buffer
    /// **[PERF-POLL-010]** Prevent duplicate notifications
    ready_notified: bool,
}

/// Manages passage buffers
///
/// [SSD-BUF-010] Buffer management strategy:
/// - Full decode for current and next passages
/// - Partial decode (15 seconds) for queued passages
/// **[PERF-POLL-010]** Event-driven buffer readiness notification
pub struct BufferManager {
    /// Map of passage_id -> managed buffer
    buffers: Arc<RwLock<HashMap<Uuid, ManagedBuffer>>>,

    /// Optional channel for buffer events (ReadyForStart notifications)
    /// **[PERF-POLL-010]** Enable event-driven playback start
    event_tx: Arc<RwLock<Option<mpsc::UnboundedSender<BufferEvent>>>>,

    /// Minimum buffer threshold in milliseconds
    /// **[PERF-START-010]** Configurable minimum buffer for instant startup
    min_buffer_threshold_ms: Arc<RwLock<u64>>,

    /// Whether any passage has ever been played (for first-passage optimization)
    /// **[PERF-FIRST-010]** Use smaller buffer for first passage only
    ever_played: Arc<AtomicBool>,
}

impl BufferManager {
    /// Create new buffer manager
    pub fn new() -> Self {
        Self {
            buffers: Arc::new(RwLock::new(HashMap::new())),
            event_tx: Arc::new(RwLock::new(None)),
            min_buffer_threshold_ms: Arc::new(RwLock::new(3000)), // Default: 3 seconds
            ever_played: Arc::new(AtomicBool::new(false)), // **[PERF-FIRST-010]** Track first passage
        }
    }

    /// Set buffer event channel for ReadyForStart notifications
    ///
    /// **[PERF-POLL-010]** Enable event-driven playback start
    pub async fn set_event_channel(&self, tx: mpsc::UnboundedSender<BufferEvent>) {
        *self.event_tx.write().await = Some(tx);
    }

    /// Set minimum buffer threshold
    ///
    /// **[PERF-START-010]** Configurable minimum buffer
    pub async fn set_min_buffer_threshold(&self, threshold_ms: u64) {
        *self.min_buffer_threshold_ms.write().await = threshold_ms;
    }

    /// Check if buffer should notify readiness and send event if needed
    ///
    /// **[PERF-POLL-010]** Event-driven buffer readiness
    /// **[PERF-FIRST-010]** Use 500ms threshold for first passage, then normal threshold
    /// Called after appending samples to check if threshold is reached
    async fn check_and_notify_ready(&self, queue_entry_id: Uuid) {
        use tracing::{debug, info};

        // **[PERF-FIRST-010]** First-passage optimization: Use 500ms for instant startup
        let configured_threshold = *self.min_buffer_threshold_ms.read().await;
        let ever_played_value = self.ever_played.load(Ordering::Relaxed);
        let is_first_passage = !ever_played_value;
        let threshold_ms = if is_first_passage {
            500.min(configured_threshold) // Use 500ms or configured, whichever is smaller
        } else {
            configured_threshold
        };

        // [DIAGNOSTIC] Log threshold calculation
        debug!(
            "🔍 check_and_notify_ready({}): ever_played={}, is_first_passage={}, configured={}ms, threshold={}ms",
            queue_entry_id, ever_played_value, is_first_passage, configured_threshold, threshold_ms
        );

        // Check if we should notify
        let should_notify = {
            let mut buffers = self.buffers.write().await;

            if let Some(managed) = buffers.get_mut(&queue_entry_id) {
                // Only notify if not already notified
                if !managed.ready_notified {
                    // Get buffer duration
                    let buffer_duration_ms = {
                        let buf = managed.buffer.read().await;
                        buf.duration_ms()
                    };

                    if buffer_duration_ms >= threshold_ms {
                        // Mark as notified to prevent duplicate events
                        managed.ready_notified = true;
                        Some(buffer_duration_ms)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        };

        // Send notification if threshold reached
        if let Some(buffer_duration_ms) = should_notify {
            let event_tx = self.event_tx.read().await;
            if let Some(ref tx) = *event_tx {
                let event = BufferEvent::ReadyForStart {
                    queue_entry_id,
                    buffer_duration_ms,
                };

                if let Err(e) = tx.send(event) {
                    debug!("Failed to send ReadyForStart event: {}", e);
                } else {
                    if is_first_passage {
                        info!(
                            "🚀 FIRST PASSAGE ready for instant playback: {} ({}ms >= {}ms threshold) [FAST START]",
                            queue_entry_id, buffer_duration_ms, threshold_ms
                        );
                    } else {
                        info!(
                            "⚡ Buffer ready for playback: {} ({}ms >= {}ms threshold)",
                            queue_entry_id, buffer_duration_ms, threshold_ms
                        );
                    }
                }
            }
        }
    }

    /// Register a buffer as decoding
    ///
    /// Called by decoder pool when decode starts.
    /// Initializes buffer with Decoding status and returns writable buffer handle.
    ///
    /// [SSD-BUF-020] Buffer state: None → Decoding
    /// [SSD-PBUF-028] Returns buffer handle for incremental filling
    ///
    /// # Returns
    /// Arc<RwLock<PassageBuffer>> for incremental sample appending
    pub async fn register_decoding(&self, passage_id: Uuid) -> Arc<RwLock<PassageBuffer>> {
        use tracing::debug;
        let mut buffers = self.buffers.write().await;

        // Only insert if not already present (avoid overwriting in-progress or completed buffers)
        if !buffers.contains_key(&passage_id) {
            debug!("Registering new buffer for decoding: passage_id={}", passage_id);
            let buffer_arc = Arc::new(RwLock::new(PassageBuffer::new(
                passage_id,
                Vec::new(), // Empty initially - will be filled incrementally
                44100,
                2,
            )));

            buffers.insert(
                passage_id,
                ManagedBuffer {
                    buffer: Arc::clone(&buffer_arc),
                    status: BufferStatus::Decoding { progress_percent: 0 },
                    decode_started: Instant::now(),
                    ready_notified: false, // [PERF-POLL-010] Not notified yet
                },
            );

            buffer_arc
        } else {
            debug!("Buffer already exists for passage_id={}, returning existing", passage_id);
            // Return existing buffer
            Arc::clone(&buffers.get(&passage_id).unwrap().buffer)
        }
    }

    /// Update decode progress
    ///
    /// Called periodically by decoder to report progress.
    /// [SSD-BUF-021] Emit progress updates every 10%
    pub async fn update_decode_progress(&self, passage_id: Uuid, progress_percent: u8) {
        let mut buffers = self.buffers.write().await;

        if let Some(managed) = buffers.get_mut(&passage_id) {
            managed.status = BufferStatus::Decoding { progress_percent };
        }
    }

    /// Notify after samples appended (check readiness threshold)
    ///
    /// **[PERF-POLL-010]** Check buffer threshold and send event if ready
    /// Called by decoder after appending each chunk to check if threshold reached
    pub async fn notify_samples_appended(&self, queue_entry_id: Uuid) {
        self.check_and_notify_ready(queue_entry_id).await;
    }

    /// Mark buffer as ready
    ///
    /// Called by decoder pool when decode completes.
    /// Updates status to Ready (buffer already filled incrementally).
    ///
    /// [SSD-BUF-020] Buffer state: Decoding → Ready
    /// [SSD-PBUF-028] With incremental decode, buffer already contains all samples
    pub async fn mark_ready(&self, passage_id: Uuid) {
        use tracing::debug;

        let mut buffers = self.buffers.write().await;

        if let Some(managed) = buffers.get_mut(&passage_id) {
            // Get sample count from buffer for logging
            let sample_count = {
                let buf = managed.buffer.read().await;
                buf.sample_count
            };

            debug!("Marking buffer ready for passage_id={}, frames={}", passage_id, sample_count);
            managed.status = BufferStatus::Ready;
        } else {
            debug!("mark_ready called but passage_id={} not found in buffers", passage_id);
        }
    }

    /// Get buffer for playback
    ///
    /// Returns Arc to buffer if it exists.
    /// Mixer reads from this buffer during playback.
    ///
    /// [SSD-MIX-010] Mixer reads buffers
    pub async fn get_buffer(&self, passage_id: Uuid) -> Option<Arc<RwLock<PassageBuffer>>> {
        let buffers = self.buffers.read().await;
        buffers.get(&passage_id).map(|m| Arc::clone(&m.buffer))
    }

    /// Get buffer status
    ///
    /// [SSD-BUF-020] Buffer state tracking
    pub async fn get_status(&self, passage_id: Uuid) -> Option<BufferStatus> {
        let buffers = self.buffers.read().await;
        buffers.get(&passage_id).map(|m| m.status)
    }

    /// Mark buffer as playing
    ///
    /// Called when mixer starts reading buffer.
    /// [SSD-BUF-020] Buffer state: Ready → Playing
    /// **[PERF-FIRST-010]** Mark that we've played at least one passage
    pub async fn mark_playing(&self, passage_id: Uuid) {
        use tracing::debug;

        let mut buffers = self.buffers.write().await;

        if let Some(managed) = buffers.get_mut(&passage_id) {
            managed.status = BufferStatus::Playing;

            // **[PERF-FIRST-010]** Mark that we've started playback at least once
            let was_played_before = self.ever_played.load(Ordering::Relaxed);
            self.ever_played.store(true, Ordering::Relaxed);

            // [DIAGNOSTIC] Log when ever_played flag is set
            debug!(
                "🎵 mark_playing({}): Setting ever_played from {} to true",
                passage_id, was_played_before
            );
        }
    }

    /// Mark buffer as exhausted
    ///
    /// Called when mixer finishes reading buffer.
    /// [SSD-BUF-020] Buffer state: Playing → Exhausted
    pub async fn mark_exhausted(&self, passage_id: Uuid) {
        let mut buffers = self.buffers.write().await;

        if let Some(managed) = buffers.get_mut(&passage_id) {
            managed.status = BufferStatus::Exhausted;
        }
    }

    /// Finalize buffer after decode completes
    ///
    /// **[PCF-DUR-010][PCF-COMP-010]** Caches duration and sets completion sentinel.
    /// This method should be called by the decoder when it reaches end-of-file.
    ///
    /// After finalization:
    /// - `buffer.duration_ms()` returns cached value (won't grow)
    /// - `buffer.is_exhausted(position)` can accurately detect completion
    /// - Duration display in UI remains stable
    pub async fn finalize_buffer(&self, passage_id: Uuid) {
        // Get buffer handle
        let buffer_arc = {
            let buffers = self.buffers.read().await;
            buffers.get(&passage_id).map(|m| Arc::clone(&m.buffer))
        };

        // Finalize the buffer (cache duration and total_frames)
        if let Some(buffer_arc) = buffer_arc {
            let mut buffer = buffer_arc.write().await;
            buffer.finalize();
        }
    }

    /// Remove buffer (cleanup)
    ///
    /// Removes buffer from manager, freeing memory.
    /// Call this after passage completes playback.
    pub async fn remove(&self, passage_id: Uuid) -> bool {
        let mut buffers = self.buffers.write().await;
        buffers.remove(&passage_id).is_some()
    }

    /// Clear all buffers
    ///
    /// Removes all buffers from manager.
    /// Useful for queue clear or shutdown.
    pub async fn clear(&self) {
        let mut buffers = self.buffers.write().await;
        buffers.clear();
    }

    /// Get all buffer statuses
    ///
    /// Returns map of passage_id -> status for all managed buffers.
    /// Useful for API status endpoint.
    pub async fn get_all_statuses(&self) -> HashMap<Uuid, BufferStatus> {
        let buffers = self.buffers.read().await;
        buffers
            .iter()
            .map(|(id, managed)| (*id, managed.status))
            .collect()
    }

    /// Check if buffer is ready for playback
    ///
    /// Returns true if buffer exists and is in Ready or Playing state.
    pub async fn is_ready(&self, passage_id: Uuid) -> bool {
        let buffers = self.buffers.read().await;

        if let Some(managed) = buffers.get(&passage_id) {
            matches!(managed.status, BufferStatus::Ready | BufferStatus::Playing)
        } else {
            false
        }
    }

    /// Check if buffer has minimum playback buffer available
    ///
    /// [SSD-PBUF-028] Minimum playback buffer threshold
    /// Returns true if buffer has at least `min_duration_ms` of audio decoded.
    /// Enables partial buffer playback - start playing before full decode completes.
    ///
    /// **Default threshold:** 3000ms (3 seconds)
    /// **Use case:** Start current passage playback as soon as minimum buffer available
    pub async fn has_minimum_playback_buffer(&self, passage_id: Uuid, min_duration_ms: u64) -> bool {
        // First check if buffer exists
        let buffer_arc = {
            let buffers = self.buffers.read().await;
            buffers.get(&passage_id).map(|m| Arc::clone(&m.buffer))
        };

        if let Some(buffer_arc) = buffer_arc {
            // Read buffer and check duration
            let buffer = buffer_arc.read().await;
            let available_ms = buffer.duration_ms();
            available_ms >= min_duration_ms
        } else {
            false
        }
    }

    /// Get decode elapsed time
    ///
    /// Returns duration since decode started for a passage.
    /// Useful for diagnostics and underrun detection.
    pub async fn get_decode_elapsed(&self, passage_id: Uuid) -> Option<std::time::Duration> {
        let buffers = self.buffers.read().await;
        buffers
            .get(&passage_id)
            .map(|managed| managed.decode_started.elapsed())
    }
}

impl Default for BufferManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_buffer_manager_creation() {
        let manager = BufferManager::new();
        let statuses = manager.get_all_statuses().await;
        assert_eq!(statuses.len(), 0);
    }

    #[tokio::test]
    async fn test_buffer_lifecycle() {
        let manager = BufferManager::new();
        let passage_id = Uuid::new_v4();

        // Initially, buffer doesn't exist
        assert!(manager.get_status(passage_id).await.is_none());
        assert!(!manager.is_ready(passage_id).await);

        // Register decoding - get writable buffer handle
        let buffer_handle = manager.register_decoding(passage_id).await;
        let status = manager.get_status(passage_id).await.unwrap();
        assert!(matches!(status, BufferStatus::Decoding { .. }));
        assert!(!manager.is_ready(passage_id).await);

        // Update progress
        manager.update_decode_progress(passage_id, 50).await;
        let status = manager.get_status(passage_id).await.unwrap();
        assert_eq!(status, BufferStatus::Decoding { progress_percent: 50 });

        // Append samples to buffer (simulating incremental decode)
        {
            let mut buffer = buffer_handle.write().await;
            buffer.append_samples(vec![0.0; 1000]);  // 500 stereo frames
        }

        // Mark ready (buffer already filled incrementally)
        manager.mark_ready(passage_id).await;
        let status = manager.get_status(passage_id).await.unwrap();
        assert_eq!(status, BufferStatus::Ready);
        assert!(manager.is_ready(passage_id).await);

        // Get buffer
        let retrieved = manager.get_buffer(passage_id).await;
        assert!(retrieved.is_some());

        // Mark playing
        manager.mark_playing(passage_id).await;
        let status = manager.get_status(passage_id).await.unwrap();
        assert_eq!(status, BufferStatus::Playing);
        assert!(manager.is_ready(passage_id).await);

        // Mark exhausted
        manager.mark_exhausted(passage_id).await;
        let status = manager.get_status(passage_id).await.unwrap();
        assert_eq!(status, BufferStatus::Exhausted);
        assert!(!manager.is_ready(passage_id).await);

        // Remove
        let removed = manager.remove(passage_id).await;
        assert!(removed);
        assert!(manager.get_status(passage_id).await.is_none());
    }

    #[tokio::test]
    async fn test_buffer_manager_multiple_buffers() {
        let manager = BufferManager::new();

        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let id3 = Uuid::new_v4();

        // Register multiple buffers - get writable handles
        let handle1 = manager.register_decoding(id1).await;
        manager.register_decoding(id2).await;
        manager.register_decoding(id3).await;

        let statuses = manager.get_all_statuses().await;
        assert_eq!(statuses.len(), 3);

        // Fill buffer 1 and mark ready
        {
            let mut buffer = handle1.write().await;
            buffer.append_samples(vec![0.0; 100]);  // 50 stereo frames
        }
        manager.mark_ready(id1).await;

        assert!(manager.is_ready(id1).await);
        assert!(!manager.is_ready(id2).await);
        assert!(!manager.is_ready(id3).await);
    }

    #[tokio::test]
    async fn test_buffer_manager_clear() {
        let manager = BufferManager::new();

        // Add some buffers
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        manager.register_decoding(id1).await;
        manager.register_decoding(id2).await;

        let statuses = manager.get_all_statuses().await;
        assert_eq!(statuses.len(), 2);

        // Clear
        manager.clear().await;
        let statuses = manager.get_all_statuses().await;
        assert_eq!(statuses.len(), 0);
    }

    #[tokio::test]
    async fn test_decode_elapsed_time() {
        let manager = BufferManager::new();
        let passage_id = Uuid::new_v4();

        // No buffer yet
        assert!(manager.get_decode_elapsed(passage_id).await.is_none());

        // Register decoding
        manager.register_decoding(passage_id).await;

        // Small delay
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Should have some elapsed time
        let elapsed = manager.get_decode_elapsed(passage_id).await.unwrap();
        assert!(elapsed.as_millis() >= 10);
    }

    #[tokio::test]
    async fn test_buffer_manager_remove_nonexistent() {
        let manager = BufferManager::new();
        let passage_id = Uuid::new_v4();

        let removed = manager.remove(passage_id).await;
        assert!(!removed);
    }

    // --- REV004: Tests for partial buffer playback ---

    #[tokio::test]
    async fn test_has_minimum_playback_buffer_no_buffer() {
        let manager = BufferManager::new();
        let passage_id = Uuid::new_v4();

        // Buffer doesn't exist - should return false
        assert!(!manager.has_minimum_playback_buffer(passage_id, 3000).await);
    }

    #[tokio::test]
    async fn test_has_minimum_playback_buffer_below_threshold() {
        let manager = BufferManager::new();
        let passage_id = Uuid::new_v4();

        // Register and get writable handle
        let handle = manager.register_decoding(passage_id).await;

        // Append 2 seconds of audio (88200 samples/sec * 2)
        {
            let mut buffer = handle.write().await;
            buffer.append_samples(vec![0.0; 88200 * 2]);  // 2 seconds
        }

        // Threshold is 3000ms, buffer has 2000ms - should be false
        assert!(!manager.has_minimum_playback_buffer(passage_id, 3000).await);
    }

    #[tokio::test]
    async fn test_has_minimum_playback_buffer_at_threshold() {
        let manager = BufferManager::new();
        let passage_id = Uuid::new_v4();

        // Register and get handle
        let handle = manager.register_decoding(passage_id).await;

        // Append exactly 3 seconds
        {
            let mut buffer = handle.write().await;
            buffer.append_samples(vec![0.0; 88200 * 3]);  // 3 seconds
        }

        // Threshold is 3000ms, buffer has 3000ms - should be true
        assert!(manager.has_minimum_playback_buffer(passage_id, 3000).await);
    }

    #[tokio::test]
    async fn test_has_minimum_playback_buffer_above_threshold() {
        let manager = BufferManager::new();
        let passage_id = Uuid::new_v4();

        // Register and get handle
        let handle = manager.register_decoding(passage_id).await;

        // Append 5 seconds
        {
            let mut buffer = handle.write().await;
            buffer.append_samples(vec![0.0; 88200 * 5]);  // 5 seconds
        }

        // Threshold is 3000ms, buffer has 5000ms - should be true
        assert!(manager.has_minimum_playback_buffer(passage_id, 3000).await);
    }

    #[tokio::test]
    async fn test_has_minimum_playback_buffer_incremental() {
        let manager = BufferManager::new();
        let passage_id = Uuid::new_v4();

        // Register and get handle
        let handle = manager.register_decoding(passage_id).await;

        // Initially false (no buffer)
        assert!(!manager.has_minimum_playback_buffer(passage_id, 3000).await);

        // Append 1 second - still below threshold
        {
            let mut buffer = handle.write().await;
            buffer.append_samples(vec![0.0; 88200]);
        }
        assert!(!manager.has_minimum_playback_buffer(passage_id, 3000).await);

        // Append 1 more second (total 2) - still below
        {
            let mut buffer = handle.write().await;
            buffer.append_samples(vec![0.0; 88200]);
        }
        assert!(!manager.has_minimum_playback_buffer(passage_id, 3000).await);

        // Append 1 more second (total 3) - now at threshold
        {
            let mut buffer = handle.write().await;
            buffer.append_samples(vec![0.0; 88200]);
        }
        assert!(manager.has_minimum_playback_buffer(passage_id, 3000).await);

        // Append more - still true
        {
            let mut buffer = handle.write().await;
            buffer.append_samples(vec![0.0; 88200 * 2]);
        }
        assert!(manager.has_minimum_playback_buffer(passage_id, 5000).await);
    }

    #[tokio::test]
    async fn test_register_decoding_returns_writable_handle() {
        let manager = BufferManager::new();
        let passage_id = Uuid::new_v4();

        // Register and get writable handle
        let handle = manager.register_decoding(passage_id).await;

        // Should be able to write to buffer via handle
        {
            let mut buffer = handle.write().await;
            buffer.append_samples(vec![0.1, 0.2, 0.3, 0.4]);
        }

        // Verify buffer was updated
        let retrieved = manager.get_buffer(passage_id).await.unwrap();
        {
            let buffer = retrieved.read().await;
            assert_eq!(buffer.sample_count, 2);
            assert_eq!(buffer.samples[0], 0.1);
        }
    }

    #[tokio::test]
    async fn test_register_decoding_duplicate_returns_same_handle() {
        let manager = BufferManager::new();
        let passage_id = Uuid::new_v4();

        // Register first time
        let handle1 = manager.register_decoding(passage_id).await;

        // Write some data
        {
            let mut buffer = handle1.write().await;
            buffer.append_samples(vec![0.1, 0.2]);
        }

        // Register again with same ID - should return existing
        let handle2 = manager.register_decoding(passage_id).await;

        // Data should be preserved
        {
            let buffer = handle2.read().await;
            assert_eq!(buffer.sample_count, 1);
            assert_eq!(buffer.samples[0], 0.1);
        }
    }

    // ============================================================================
    // Phase 1: Event-Driven Notification Tests [PERF-POLL-010]
    // ============================================================================

    #[tokio::test]
    async fn test_event_channel_setup() {
        use tokio::sync::mpsc;
        use crate::playback::types::BufferEvent;

        let manager = BufferManager::new();
        let (tx, _rx) = mpsc::unbounded_channel();

        // Should succeed
        manager.set_event_channel(tx).await;

        // Manager should now have event channel configured
        let event_tx = manager.event_tx.read().await;
        assert!(event_tx.is_some(), "Event channel should be configured");
    }

    #[tokio::test]
    async fn test_buffer_ready_event_emission() {
        use tokio::sync::mpsc;
        use crate::playback::types::BufferEvent;
        use tokio::time::Duration;

        let manager = BufferManager::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        manager.set_event_channel(tx).await;
        manager.set_min_buffer_threshold(3000).await;

        let passage_id = Uuid::new_v4();
        let buffer_handle = manager.register_decoding(passage_id).await;

        // Append 3+ seconds of audio (264600 samples = 3s @ 44.1kHz stereo)
        {
            let mut buffer = buffer_handle.write().await;
            buffer.append_samples(vec![0.0; 264600]);
        }

        // Trigger notification
        manager.notify_samples_appended(passage_id).await;

        // Verify ReadyForStart event emitted
        let event = tokio::time::timeout(Duration::from_millis(100), rx.recv())
            .await
            .expect("Should receive event within timeout")
            .expect("Channel should not be closed");

        match event {
            BufferEvent::ReadyForStart { queue_entry_id, buffer_duration_ms } => {
                assert_eq!(queue_entry_id, passage_id);
                assert!(buffer_duration_ms >= 3000, "Buffer should have 3000ms+, got {}ms", buffer_duration_ms);
            }
        }
    }

    #[tokio::test]
    async fn test_buffer_ready_event_with_threshold() {
        use tokio::sync::mpsc;
        use crate::playback::types::BufferEvent;
        use tokio::time::Duration;

        let manager = BufferManager::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        manager.set_event_channel(tx).await;
        manager.set_min_buffer_threshold(2000).await; // 2 second threshold

        // Register and mark a dummy passage as playing to disable first-passage optimization
        let dummy_id = Uuid::new_v4();
        let _dummy_handle = manager.register_decoding(dummy_id).await;
        manager.mark_playing(dummy_id).await;

        let passage_id = Uuid::new_v4();
        let buffer_handle = manager.register_decoding(passage_id).await;

        // Append 1.9 seconds (below threshold)
        {
            let mut buffer = buffer_handle.write().await;
            buffer.append_samples(vec![0.0; 167580]); // 1.9s @ 44.1kHz stereo
        }
        manager.notify_samples_appended(passage_id).await;

        // No event yet (below threshold)
        tokio::time::timeout(Duration::from_millis(50), rx.recv())
            .await
            .expect_err("Should timeout - buffer below threshold");

        // Append +200ms (total 2.1s, above threshold)
        {
            let mut buffer = buffer_handle.write().await;
            buffer.append_samples(vec![0.0; 17640]); // +200ms
        }
        manager.notify_samples_appended(passage_id).await;

        // Event should be emitted
        let event = tokio::time::timeout(Duration::from_millis(100), rx.recv())
            .await
            .expect("Should receive event")
            .expect("Channel should not be closed");

        match event {
            BufferEvent::ReadyForStart { queue_entry_id, buffer_duration_ms } => {
                assert_eq!(queue_entry_id, passage_id);
                assert!(buffer_duration_ms >= 2000, "Buffer should have 2000ms+");
            }
        }
    }

    #[tokio::test]
    async fn test_event_deduplication() {
        use tokio::sync::mpsc;
        use crate::playback::types::BufferEvent;
        use tokio::time::Duration;

        let manager = BufferManager::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        manager.set_event_channel(tx).await;
        manager.set_min_buffer_threshold(1000).await;

        let passage_id = Uuid::new_v4();
        let buffer_handle = manager.register_decoding(passage_id).await;

        // Append 1+ second
        {
            let mut buffer = buffer_handle.write().await;
            buffer.append_samples(vec![0.0; 88200]); // 1s
        }
        manager.notify_samples_appended(passage_id).await;

        // First event should be emitted
        let _event = rx.recv().await.expect("Should receive first event");

        // Append more samples
        {
            let mut buffer = buffer_handle.write().await;
            buffer.append_samples(vec![0.0; 88200]); // +1s = 2s total
        }
        manager.notify_samples_appended(passage_id).await;

        // Should NOT emit duplicate event
        tokio::time::timeout(Duration::from_millis(50), rx.recv())
            .await
            .expect_err("Should not emit duplicate ReadyForStart event");
    }

    #[tokio::test]
    async fn test_event_channel_failure_handling() {
        use tokio::sync::mpsc;

        let manager = BufferManager::new();
        let (tx, rx) = mpsc::unbounded_channel();
        manager.set_event_channel(tx).await;
        manager.set_min_buffer_threshold(1000).await;

        // Drop receiver to simulate channel failure
        drop(rx);

        let passage_id = Uuid::new_v4();
        let buffer_handle = manager.register_decoding(passage_id).await;

        // Append samples
        {
            let mut buffer = buffer_handle.write().await;
            buffer.append_samples(vec![0.0; 88200]); // 1s
        }

        // Should not panic when channel is closed
        manager.notify_samples_appended(passage_id).await;
    }

    #[tokio::test]
    async fn test_notify_samples_appended_without_event_channel() {
        let manager = BufferManager::new();
        manager.set_min_buffer_threshold(1000).await;

        let passage_id = Uuid::new_v4();
        let buffer_handle = manager.register_decoding(passage_id).await;

        {
            let mut buffer = buffer_handle.write().await;
            buffer.append_samples(vec![0.0; 88200]);
        }

        // Should not panic when event channel is not configured
        manager.notify_samples_appended(passage_id).await;
    }

    // ============================================================================
    // Phase 1: First-Passage Optimization Tests [PERF-FIRST-010]
    // ============================================================================

    #[tokio::test]
    async fn test_first_passage_uses_500ms_threshold() {
        use tokio::sync::mpsc;
        use crate::playback::types::BufferEvent;
        use tokio::time::Duration;

        let manager = BufferManager::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        manager.set_event_channel(tx).await;
        manager.set_min_buffer_threshold(3000).await; // Normal threshold = 3000ms

        let passage_id = Uuid::new_v4();
        let buffer_handle = manager.register_decoding(passage_id).await;

        // Append exactly 500ms of audio
        {
            let mut buffer = buffer_handle.write().await;
            buffer.append_samples(vec![0.0; 44100]); // 500ms @ 44.1kHz stereo
        }

        manager.notify_samples_appended(passage_id).await;

        // First passage should trigger at 500ms (not 3000ms)
        let event = tokio::time::timeout(Duration::from_millis(100), rx.recv())
            .await
            .expect("Should receive event within timeout")
            .expect("Channel should not be closed");

        match event {
            BufferEvent::ReadyForStart { buffer_duration_ms, .. } => {
                assert!(buffer_duration_ms >= 500, "First passage should trigger at 500ms");
                assert!(buffer_duration_ms < 1000, "First passage triggered at {}ms", buffer_duration_ms);
            }
        }
    }

    #[tokio::test]
    async fn test_subsequent_passage_uses_configured_threshold() {
        use tokio::sync::mpsc;
        use crate::playback::types::BufferEvent;
        use tokio::time::Duration;

        let manager = BufferManager::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        manager.set_event_channel(tx).await;
        manager.set_min_buffer_threshold(3000).await;

        // First passage - should use 500ms
        let passage1 = Uuid::new_v4();
        let handle1 = manager.register_decoding(passage1).await;
        {
            let mut buffer = handle1.write().await;
            buffer.append_samples(vec![0.0; 44100]); // 500ms
        }
        manager.notify_samples_appended(passage1).await;
        let _event = rx.recv().await.expect("First passage should emit at 500ms");

        // Mark as playing (sets ever_played flag)
        manager.mark_playing(passage1).await;

        // Second passage - should use 3000ms
        let passage2 = Uuid::new_v4();
        let handle2 = manager.register_decoding(passage2).await;

        // 500ms should NOT trigger for second passage
        {
            let mut buffer = handle2.write().await;
            buffer.append_samples(vec![0.0; 44100]); // 500ms
        }
        manager.notify_samples_appended(passage2).await;

        tokio::time::timeout(Duration::from_millis(100), rx.recv())
            .await
            .expect_err("Second passage should not emit at 500ms");

        // 3000ms SHOULD trigger for second passage
        {
            let mut buffer = handle2.write().await;
            buffer.append_samples(vec![0.0; 220500]); // +2.5s = 3s total
        }
        manager.notify_samples_appended(passage2).await;

        let event = rx.recv().await.expect("Second passage should emit at 3000ms");
        assert!(matches!(event, BufferEvent::ReadyForStart { .. }));
    }

    #[tokio::test]
    async fn test_ever_played_flag_behavior() {
        use std::sync::atomic::Ordering;

        let manager = BufferManager::new();

        // Initially, ever_played should be false
        assert!(!manager.ever_played.load(Ordering::Relaxed), "ever_played should be false initially");

        let passage_id = Uuid::new_v4();
        manager.register_decoding(passage_id).await;

        // Still false after registration
        assert!(!manager.ever_played.load(Ordering::Relaxed));

        // Mark as playing
        manager.mark_playing(passage_id).await;

        // Should be true now
        assert!(manager.ever_played.load(Ordering::Relaxed), "ever_played should be true after mark_playing");
    }

    #[tokio::test]
    async fn test_first_passage_optimization_with_partial_buffer() {
        use tokio::sync::mpsc;
        use crate::playback::types::BufferEvent;
        use tokio::time::Duration;

        let manager = BufferManager::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        manager.set_event_channel(tx).await;
        manager.set_min_buffer_threshold(2000).await; // 2s threshold

        let passage_id = Uuid::new_v4();
        let buffer_handle = manager.register_decoding(passage_id).await;

        // Append 499ms (below 500ms minimum for first passage)
        {
            let mut buffer = buffer_handle.write().await;
            buffer.append_samples(vec![0.0; 43956]); // 499ms
        }
        manager.notify_samples_appended(passage_id).await;

        // Should not emit yet
        tokio::time::timeout(Duration::from_millis(50), rx.recv())
            .await
            .expect_err("Should not emit below 500ms even for first passage");

        // Append +2ms (total 501ms, above 500ms minimum)
        {
            let mut buffer = buffer_handle.write().await;
            buffer.append_samples(vec![0.0; 176]); // +2ms
        }
        manager.notify_samples_appended(passage_id).await;

        // Should emit now
        let event = rx.recv().await.expect("Should emit at 501ms for first passage");
        assert!(matches!(event, BufferEvent::ReadyForStart { .. }));
    }

    // ============================================================================
    // Phase 1: Configurable Threshold Tests [PERF-START-010]
    // ============================================================================

    #[tokio::test]
    async fn test_set_min_buffer_threshold() {
        let manager = BufferManager::new();

        // Default should be some reasonable value
        let default_threshold = *manager.min_buffer_threshold_ms.read().await;
        assert!(default_threshold > 0, "Default threshold should be positive");

        // Set custom threshold
        manager.set_min_buffer_threshold(1500).await;
        let threshold = *manager.min_buffer_threshold_ms.read().await;
        assert_eq!(threshold, 1500, "Threshold should be updated");
    }

    #[tokio::test]
    async fn test_dynamic_threshold_configuration() {
        let manager = BufferManager::new();

        // Test setting threshold
        manager.set_min_buffer_threshold(1500).await;

        // Verify threshold is used in checks
        let passage_id = Uuid::new_v4();
        let buffer_handle = manager.register_decoding(passage_id).await;

        // Mark as playing first to avoid first-passage optimization
        manager.mark_playing(Uuid::new_v4()).await;

        // 1400ms should be below threshold
        {
            let mut buffer = buffer_handle.write().await;
            buffer.append_samples(vec![0.0; 123480]); // 1.4s
        }
        assert!(!manager.has_minimum_playback_buffer(passage_id, 1500).await, "1400ms should be below 1500ms threshold");

        // 1600ms should be above threshold
        {
            let mut buffer = buffer_handle.write().await;
            buffer.append_samples(vec![0.0; 17640]); // +200ms = 1.6s total
        }
        assert!(manager.has_minimum_playback_buffer(passage_id, 1500).await, "1600ms should be above 1500ms threshold");
    }

    #[tokio::test]
    async fn test_threshold_configuration_with_events() {
        use tokio::sync::mpsc;
        use crate::playback::types::BufferEvent;
        use tokio::time::Duration;

        let manager = BufferManager::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        manager.set_event_channel(tx).await;

        // Set threshold to 800ms
        manager.set_min_buffer_threshold(800).await;

        // Register and mark a dummy passage as playing to disable first-passage optimization
        let dummy_id = Uuid::new_v4();
        let _dummy_handle = manager.register_decoding(dummy_id).await;
        manager.mark_playing(dummy_id).await;

        let passage_id = Uuid::new_v4();
        let buffer_handle = manager.register_decoding(passage_id).await;

        // 750ms should not trigger
        {
            let mut buffer = buffer_handle.write().await;
            buffer.append_samples(vec![0.0; 66150]); // 750ms
        }
        manager.notify_samples_appended(passage_id).await;

        tokio::time::timeout(Duration::from_millis(50), rx.recv())
            .await
            .expect_err("Should not emit at 750ms with 800ms threshold");

        // 850ms should trigger
        {
            let mut buffer = buffer_handle.write().await;
            buffer.append_samples(vec![0.0; 8820]); // +100ms = 850ms total
        }
        manager.notify_samples_appended(passage_id).await;

        let event = rx.recv().await.expect("Should emit at 850ms");
        assert!(matches!(event, BufferEvent::ReadyForStart { .. }));
    }

    // ==================== Phase 3: Performance/Stress Tests ====================

    /// Test high-frequency buffer append operations
    /// Verifies system can handle rapid small appends without performance degradation
    #[tokio::test]
    async fn test_high_frequency_buffer_appends() {
        let manager = BufferManager::new();
        let passage_id = Uuid::new_v4();
        let buffer_handle = manager.register_decoding(passage_id).await;

        // Append 1000 small chunks (200 samples each = 100 stereo frames = ~2.2ms per chunk)
        // Total: 200,000 samples = ~2.2 seconds of audio
        for _ in 0..1000 {
            let mut buffer = buffer_handle.write().await;
            buffer.append_samples(vec![0.0; 200]);
        }

        // Verify final buffer has all samples
        let buffer = buffer_handle.read().await;
        assert_eq!(buffer.samples.len(), 200_000, "All appends should accumulate");

        // Verify duration calculation is accurate (samples / (sample_rate * channels) * 1000)
        let duration_ms = (buffer.samples.len() as f64 / (44100.0 * 2.0) * 1000.0) as u64;
        assert!(duration_ms >= 2200 && duration_ms <= 2300, "Duration should be ~2.2s, got {}ms", duration_ms);
    }

    /// Test concurrent buffer registration from multiple tasks
    /// Verifies no race conditions or deadlocks during parallel registration
    #[tokio::test]
    async fn test_concurrent_buffer_registration() {
        let manager = Arc::new(BufferManager::new());
        let mut handles = Vec::new();

        // Spawn 50 concurrent tasks, each registering a different passage
        for _ in 0..50 {
            let manager_clone = Arc::clone(&manager);
            handles.push(tokio::spawn(async move {
                let passage_id = Uuid::new_v4();
                let buffer_handle = manager_clone.register_decoding(passage_id).await;

                // Append some data
                let mut buffer = buffer_handle.write().await;
                buffer.append_samples(vec![0.5; 1000]);

                passage_id
            }));
        }

        // Wait for all tasks to complete
        let mut passage_ids = Vec::new();
        for handle in handles {
            let id = handle.await.unwrap();
            passage_ids.push(id);
        }

        // Verify all 50 passages are registered
        assert_eq!(passage_ids.len(), 50);
        for id in passage_ids {
            assert!(manager.get_buffer(id).await.is_some(), "Passage {} should be registered", id);
        }
    }

    /// Test rapid state transitions (Ready → Playing → Exhausted)
    /// Verifies state machine handles quick succession without errors
    #[tokio::test]
    async fn test_rapid_state_transitions() {
        let manager = BufferManager::new();

        // Create 20 passages and cycle them through states rapidly
        for _ in 0..20 {
            let passage_id = Uuid::new_v4();
            let buffer_handle = manager.register_decoding(passage_id).await;

            // Fill buffer (Decoding → Ready)
            {
                let mut buffer = buffer_handle.write().await;
                buffer.append_samples(vec![0.0; 10000]);
            }
            manager.mark_ready(passage_id).await;

            // Start playing (Ready → Playing)
            manager.mark_playing(passage_id).await;
            let status = manager.get_status(passage_id).await.unwrap();
            assert_eq!(status, BufferStatus::Playing);

            // Exhaust (Playing → Exhausted)
            manager.mark_exhausted(passage_id).await;
            let status = manager.get_status(passage_id).await.unwrap();
            assert_eq!(status, BufferStatus::Exhausted);

            // Remove
            manager.remove(passage_id).await;
            assert!(manager.get_buffer(passage_id).await.is_none());
        }
    }

    /// Test concurrent event notifications from multiple passages
    /// Verifies event channel handles many simultaneous ReadyForStart events
    #[tokio::test]
    async fn test_concurrent_event_notifications() {
        use tokio::sync::mpsc;
        use crate::playback::types::BufferEvent;

        let manager = Arc::new(BufferManager::new());
        let (tx, mut rx) = mpsc::unbounded_channel();
        manager.set_event_channel(tx).await;
        manager.set_min_buffer_threshold(500).await; // Low threshold for quick events

        // Disable first-passage optimization
        let dummy_id = Uuid::new_v4();
        let _dummy = manager.register_decoding(dummy_id).await;
        manager.mark_playing(dummy_id).await;

        let mut handles = Vec::new();

        // Spawn 20 tasks, each filling a buffer to trigger event
        for _ in 0..20 {
            let manager_clone = Arc::clone(&manager);
            handles.push(tokio::spawn(async move {
                let passage_id = Uuid::new_v4();
                let buffer_handle = manager_clone.register_decoding(passage_id).await;

                // Append enough to trigger 500ms threshold
                {
                    let mut buffer = buffer_handle.write().await;
                    buffer.append_samples(vec![0.0; 44100]); // 500ms @ 44.1kHz stereo
                }

                manager_clone.notify_samples_appended(passage_id).await;
                passage_id
            }));
        }

        // Wait for all tasks
        for handle in handles {
            handle.await.unwrap();
        }

        // Collect all events (should be 20)
        let mut event_count = 0;
        while let Ok(event) = rx.try_recv() {
            if matches!(event, BufferEvent::ReadyForStart { .. }) {
                event_count += 1;
            }
        }

        assert_eq!(event_count, 20, "Should receive 20 ReadyForStart events");
    }

    /// **[PCF-DUR-010][PCF-COMP-010]** Test finalize_buffer() caches duration and completion metadata
    #[tokio::test]
    async fn test_finalize_buffer() {
        let manager = BufferManager::new();
        let passage_id = Uuid::new_v4();
        let buffer_handle = manager.register_decoding(passage_id).await;

        // Append some samples (2 seconds)
        {
            let mut buffer = buffer_handle.write().await;
            buffer.append_samples(vec![0.0; 176400]); // 88200 frames = 2 seconds
        }

        // Before finalize: duration calculated from sample_count
        {
            let buffer = buffer_handle.read().await;
            assert_eq!(buffer.duration_ms(), 2000);
            assert!(!buffer.decode_complete);
            assert_eq!(buffer.total_frames, None);
            assert_eq!(buffer.total_duration_ms, None);
        }

        // Finalize the buffer (decode complete)
        manager.finalize_buffer(passage_id).await;

        // After finalize: metadata is cached
        {
            let buffer = buffer_handle.read().await;
            assert!(buffer.decode_complete);
            assert_eq!(buffer.total_frames, Some(88200));
            assert_eq!(buffer.total_duration_ms, Some(2000));
            assert_eq!(buffer.duration_ms(), 2000); // Uses cached value
        }

        // Append more samples (defensive test - shouldn't happen in practice)
        {
            let mut buffer = buffer_handle.write().await;
            buffer.append_samples(vec![0.0; 88200]); // +1 second
        }

        // Duration should still be 2000ms (cached, not recalculated)
        {
            let buffer = buffer_handle.read().await;
            assert_eq!(buffer.duration_ms(), 2000); // Still 2000ms, not 3000ms!
        }
    }

    /// **[PCF-COMP-010]** Test is_exhausted() detection after finalize
    #[tokio::test]
    async fn test_buffer_exhaustion_detection() {
        let manager = BufferManager::new();
        let passage_id = Uuid::new_v4();
        let buffer_handle = manager.register_decoding(passage_id).await;

        // Append 1 second of audio
        {
            let mut buffer = buffer_handle.write().await;
            buffer.append_samples(vec![0.0; 88200]); // 44100 frames = 1 second
        }

        // Before finalize: cannot be exhausted
        {
            let buffer = buffer_handle.read().await;
            assert!(!buffer.is_exhausted(0));
            assert!(!buffer.is_exhausted(44100)); // Position at end
            assert!(!buffer.is_exhausted(100000)); // Position past end
        }

        // Finalize the buffer
        manager.finalize_buffer(passage_id).await;

        // After finalize: can detect exhaustion
        {
            let buffer = buffer_handle.read().await;
            assert!(!buffer.is_exhausted(0));      // Position at start
            assert!(!buffer.is_exhausted(44099));  // Just before end
            assert!(buffer.is_exhausted(44100));   // At end (exhausted)
            assert!(buffer.is_exhausted(50000));   // Past end (exhausted)
        }
    }

    /// Test handling large buffer (60 seconds of audio)
    /// Verifies memory management and duration calculations for large passages
    #[tokio::test]
    async fn test_large_buffer_stress() {
        let manager = BufferManager::new();
        let passage_id = Uuid::new_v4();
        let buffer_handle = manager.register_decoding(passage_id).await;

        // 60 seconds @ 44.1kHz stereo = 5,292,000 samples
        // This is ~20MB of f32 data - tests memory handling
        let sixty_seconds_samples = 60 * 44100 * 2;

        {
            let mut buffer = buffer_handle.write().await;
            buffer.append_samples(vec![0.5; sixty_seconds_samples]);
        }

        // Verify duration calculation
        let buffer = buffer_handle.read().await;
        let duration_ms = (buffer.samples.len() as f64 / (44100.0 * 2.0) * 1000.0) as u64;
        assert!(duration_ms >= 59_900 && duration_ms <= 60_100,
                "Duration should be ~60s, got {}ms", duration_ms);

        // Verify buffer status is correct
        let status = manager.get_status(passage_id).await.unwrap();
        assert!(matches!(status, BufferStatus::Decoding { .. }));

        // Mark ready and verify
        manager.mark_ready(passage_id).await;
        let status = manager.get_status(passage_id).await.unwrap();
        assert_eq!(status, BufferStatus::Ready);
    }
}
