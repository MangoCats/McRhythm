//! Buffer Manager
//!
//! Manages passage buffer lifecycle (full vs. partial decode strategy).
//!
//! **Traceability:**
//! - [SSD-BUF-010] Buffer management strategy
//! - [SSD-FBUF-010] Full decode strategy
//! - [SSD-PBUF-010] Partial buffer strategy (15 seconds)
//! - [SSD-BUF-020] Buffer state tracking

use crate::audio::types::{BufferStatus, PassageBuffer};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Wrapper for buffer with metadata
struct ManagedBuffer {
    /// The actual passage buffer
    buffer: Arc<RwLock<PassageBuffer>>,

    /// Current buffer status
    status: BufferStatus,

    /// When decode started
    decode_started: Instant,
}

/// Manages passage buffers
///
/// [SSD-BUF-010] Buffer management strategy:
/// - Full decode for current and next passages
/// - Partial decode (15 seconds) for queued passages
pub struct BufferManager {
    /// Map of passage_id -> managed buffer
    buffers: Arc<RwLock<HashMap<Uuid, ManagedBuffer>>>,
}

impl BufferManager {
    /// Create new buffer manager
    pub fn new() -> Self {
        Self {
            buffers: Arc::new(RwLock::new(HashMap::new())),
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
    pub async fn mark_playing(&self, passage_id: Uuid) {
        let mut buffers = self.buffers.write().await;

        if let Some(managed) = buffers.get_mut(&passage_id) {
            managed.status = BufferStatus::Playing;
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
}
