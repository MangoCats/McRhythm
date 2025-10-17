//! Passage buffer management for single-stream audio playback
//!
//! This module implements PCM buffer management for queued passages, providing
//! sample-accurate audio data for crossfading and playback.
//!
//! Implements requirements from single-stream-design.md - Phase 1: Core Infrastructure

use std::collections::HashMap;
use std::sync::Arc;
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::RwLock;
use uuid::Uuid;
use anyhow::Result;
use tracing::{debug, info};

/// Default buffer duration for pre-loading passages
/// Implements design specification from single-stream-design.md: 15-second buffers
const DEFAULT_BUFFER_DURATION_SECS: f64 = 15.0;

/// Standard sample rate for all audio playback
/// All audio is resampled to this rate for consistent mixing
const STANDARD_SAMPLE_RATE: u32 = 44100;

/// Number of audio channels (stereo)
const CHANNELS: u16 = 2;

/// Status of a passage buffer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferStatus {
    /// Buffer is being filled by decoder
    Decoding,
    /// Buffer is ready for playback
    Ready,
    /// Buffer is currently being played
    Playing,
    /// Buffer has been consumed and can be recycled
    Exhausted,
}

/// Fade curve types for volume transitions
/// Implements XFD-CURV-020 and XFD-CURV-030 from crossfade.md
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FadeCurve {
    /// Linear volume change (constant rate)
    Linear,
    /// Logarithmic fade-out (fast start, slow finish)
    Logarithmic,
    /// Exponential fade-in (slow start, fast finish)
    Exponential,
    /// S-Curve (smooth acceleration/deceleration)
    SCurve,
}

/// PCM buffer for a single passage
///
/// Contains decoded audio data and timing information for crossfade mixing
pub struct PassageBuffer {
    /// Unique identifier for this passage
    pub passage_id: Uuid,

    /// Interleaved stereo PCM data: [L, R, L, R, ...]
    /// Using f32 for compatibility with audio processing
    pub pcm_data: Vec<f32>,

    /// Sample rate of the decoded audio
    pub sample_rate: u32,

    /// Number of channels (always 2 for stereo)
    pub channels: u16,

    /// Current status of this buffer
    pub status: BufferStatus,

    /// Fade-in curve to apply during playback
    /// Implements XFD-CURV-010 from crossfade.md
    pub fade_in_curve: FadeCurve,

    /// Fade-out curve to apply during playback
    /// Implements XFD-CURV-010 from crossfade.md
    pub fade_out_curve: FadeCurve,

    /// Number of samples for fade-in (0 = no fade-in)
    pub fade_in_samples: u64,

    /// Number of samples for fade-out (0 = no fade-out)
    pub fade_out_samples: u64,

    /// File path this buffer was decoded from
    pub file_path: PathBuf,

    /// Start position in the original file (samples)
    pub start_sample: u64,

    /// End position in the original file (samples)
    pub end_sample: u64,
}

impl PassageBuffer {
    /// Create a new passage buffer
    pub fn new(
        passage_id: Uuid,
        file_path: PathBuf,
        start_sample: u64,
        end_sample: u64,
    ) -> Self {
        let capacity = ((end_sample - start_sample) * CHANNELS as u64) as usize;

        Self {
            passage_id,
            pcm_data: Vec::with_capacity(capacity),
            sample_rate: STANDARD_SAMPLE_RATE,
            channels: CHANNELS,
            status: BufferStatus::Decoding,
            fade_in_curve: FadeCurve::Exponential,
            fade_out_curve: FadeCurve::Logarithmic,
            fade_in_samples: 0,
            fade_out_samples: 0,
            file_path,
            start_sample,
            end_sample,
        }
    }

    /// Get the duration of this buffer in seconds
    pub fn duration(&self) -> f64 {
        let samples = self.pcm_data.len() / self.channels as usize;
        samples as f64 / self.sample_rate as f64
    }

    /// Get the number of samples in this buffer
    pub fn sample_count(&self) -> u64 {
        (self.pcm_data.len() / self.channels as usize) as u64
    }

    /// Check if the buffer is ready for playback
    pub fn is_ready(&self) -> bool {
        self.status == BufferStatus::Ready
    }

    /// Mark the buffer as ready for playback
    pub fn mark_ready(&mut self) {
        self.status = BufferStatus::Ready;
        info!(
            passage_id = %self.passage_id,
            duration = self.duration(),
            samples = self.sample_count(),
            "Passage buffer ready for playback"
        );
    }

    /// Mark the buffer as currently playing
    pub fn mark_playing(&mut self) {
        self.status = BufferStatus::Playing;
    }

    /// Mark the buffer as exhausted (can be recycled)
    pub fn mark_exhausted(&mut self) {
        self.status = BufferStatus::Exhausted;
        self.pcm_data.clear();
        self.pcm_data.shrink_to_fit(); // Free memory
    }
}

/// Manages PCM buffers for all queued passages
///
/// Coordinates buffer allocation, decoding, and recycling to minimize memory usage
/// while ensuring passages are ready for seamless playback.
pub struct PassageBufferManager {
    /// All passage buffers, indexed by passage ID
    passages: Arc<RwLock<HashMap<Uuid, PassageBuffer>>>,

    /// Maximum duration for each buffer (configurable)
    buffer_duration: Duration,
}

impl PassageBufferManager {
    /// Create a new buffer manager with default settings
    pub fn new() -> Self {
        Self::with_buffer_duration(Duration::from_secs_f64(DEFAULT_BUFFER_DURATION_SECS))
    }

    /// Create a new buffer manager with custom buffer duration
    pub fn with_buffer_duration(buffer_duration: Duration) -> Self {
        info!(
            buffer_duration_secs = buffer_duration.as_secs_f64(),
            "Initializing PassageBufferManager"
        );

        Self {
            passages: Arc::new(RwLock::new(HashMap::new())),
            buffer_duration,
        }
    }

    /// Allocate a new buffer for a passage
    ///
    /// Creates an empty buffer ready to receive decoded audio data
    pub async fn allocate_buffer(
        &self,
        passage_id: Uuid,
        file_path: PathBuf,
        start_sample: u64,
        end_sample: u64,
    ) -> Result<()> {
        let buffer = PassageBuffer::new(passage_id, file_path, start_sample, end_sample);

        debug!(
            passage_id = %passage_id,
            start_sample,
            end_sample,
            expected_samples = end_sample - start_sample,
            "Allocating passage buffer"
        );

        let mut passages = self.passages.write().await;
        passages.insert(passage_id, buffer);

        Ok(())
    }

    /// Get a mutable reference to a passage buffer
    /// Returns None if the buffer doesn't exist
    ///
    /// Note: The decoder uses this to append PCM data during decoding
    pub async fn get_buffer_mut(&self, passage_id: &Uuid) -> Option<tokio::sync::RwLockWriteGuard<'_, HashMap<Uuid, PassageBuffer>>> {
        let passages = self.passages.write().await;
        if passages.contains_key(passage_id) {
            Some(passages)
        } else {
            None
        }
    }

    /// Get an immutable reference to a passage buffer
    /// Returns None if the buffer doesn't exist
    pub async fn get_buffer(&self, passage_id: &Uuid) -> Option<tokio::sync::RwLockReadGuard<'_, HashMap<Uuid, PassageBuffer>>> {
        let passages = self.passages.read().await;
        if passages.contains_key(passage_id) {
            Some(passages)
        } else {
            None
        }
    }

    /// Check if a passage buffer exists
    pub async fn has_buffer(&self, passage_id: &Uuid) -> bool {
        self.passages.read().await.contains_key(passage_id)
    }

    /// Remove a passage buffer and free its memory
    pub async fn remove_buffer(&self, passage_id: &Uuid) -> Option<PassageBuffer> {
        let mut passages = self.passages.write().await;
        let buffer = passages.remove(passage_id);

        if let Some(ref b) = buffer {
            debug!(
                passage_id = %passage_id,
                status = ?b.status,
                "Removed passage buffer"
            );
        }

        buffer
    }

    /// Get the status of a passage buffer
    pub async fn get_status(&self, passage_id: &Uuid) -> Option<BufferStatus> {
        self.passages.read().await
            .get(passage_id)
            .map(|b| b.status)
    }

    /// Mark a buffer as ready for playback
    pub async fn mark_ready(&self, passage_id: &Uuid) -> Result<()> {
        if let Some(mut buffers) = self.get_buffer_mut(passage_id).await {
            if let Some(buffer) = buffers.get_mut(passage_id) {
                buffer.mark_ready();
                Ok(())
            } else {
                Err(anyhow::anyhow!("Buffer not found for passage {}", passage_id))
            }
        } else {
            Err(anyhow::anyhow!("Buffer not found for passage {}", passage_id))
        }
    }

    /// Mark a buffer as currently playing
    pub async fn mark_playing(&self, passage_id: &Uuid) -> Result<()> {
        if let Some(mut buffers) = self.get_buffer_mut(passage_id).await {
            if let Some(buffer) = buffers.get_mut(passage_id) {
                buffer.mark_playing();
                Ok(())
            } else {
                Err(anyhow::anyhow!("Buffer not found for passage {}", passage_id))
            }
        } else {
            Err(anyhow::anyhow!("Buffer not found for passage {}", passage_id))
        }
    }

    /// Mark a buffer as exhausted and recycle its memory
    pub async fn mark_exhausted(&self, passage_id: &Uuid) -> Result<()> {
        if let Some(mut buffers) = self.get_buffer_mut(passage_id).await {
            if let Some(buffer) = buffers.get_mut(passage_id) {
                buffer.mark_exhausted();
                Ok(())
            } else {
                Err(anyhow::anyhow!("Buffer not found for passage {}", passage_id))
            }
        } else {
            Err(anyhow::anyhow!("Buffer not found for passage {}", passage_id))
        }
    }

    /// Get memory usage statistics
    pub async fn memory_stats(&self) -> MemoryStats {
        let passages = self.passages.read().await;

        let mut total_bytes = 0;
        let mut ready_count = 0;
        let mut playing_count = 0;
        let mut decoding_count = 0;
        let mut exhausted_count = 0;

        for buffer in passages.values() {
            let buffer_bytes = buffer.pcm_data.len() * std::mem::size_of::<f32>();
            total_bytes += buffer_bytes;

            match buffer.status {
                BufferStatus::Ready => ready_count += 1,
                BufferStatus::Playing => playing_count += 1,
                BufferStatus::Decoding => decoding_count += 1,
                BufferStatus::Exhausted => exhausted_count += 1,
            }
        }

        MemoryStats {
            total_buffers: passages.len(),
            total_memory_mb: total_bytes as f64 / (1024.0 * 1024.0),
            ready_count,
            playing_count,
            decoding_count,
            exhausted_count,
        }
    }

    /// Clean up exhausted buffers to free memory
    ///
    /// Removes all buffers marked as Exhausted
    pub async fn cleanup_exhausted(&self) -> usize {
        let mut passages = self.passages.write().await;
        let exhausted_ids: Vec<Uuid> = passages
            .iter()
            .filter(|(_, buffer)| buffer.status == BufferStatus::Exhausted)
            .map(|(id, _)| *id)
            .collect();

        let count = exhausted_ids.len();
        for id in exhausted_ids {
            passages.remove(&id);
        }

        if count > 0 {
            info!(count, "Cleaned up exhausted buffers");
        }

        count
    }
}

/// Memory usage statistics for buffer manager
#[derive(Debug, Clone)]
pub struct MemoryStats {
    /// Total number of buffers
    pub total_buffers: usize,
    /// Total memory used by all buffers (MB)
    pub total_memory_mb: f64,
    /// Number of buffers ready for playback
    pub ready_count: usize,
    /// Number of buffers currently playing
    pub playing_count: usize,
    /// Number of buffers being decoded
    pub decoding_count: usize,
    /// Number of exhausted buffers awaiting cleanup
    pub exhausted_count: usize,
}

impl Default for PassageBufferManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_buffer_allocation() {
        let manager = PassageBufferManager::new();
        let passage_id = Uuid::new_v4();
        let file_path = PathBuf::from("/test/file.mp3");

        // Allocate a buffer
        manager.allocate_buffer(passage_id, file_path.clone(), 0, 44100).await.unwrap();

        // Check it exists
        assert!(manager.has_buffer(&passage_id).await);

        // Check initial status
        let status = manager.get_status(&passage_id).await;
        assert_eq!(status, Some(BufferStatus::Decoding));
    }

    #[tokio::test]
    async fn test_buffer_lifecycle() {
        let manager = PassageBufferManager::new();
        let passage_id = Uuid::new_v4();
        let file_path = PathBuf::from("/test/file.mp3");

        // Allocate and get buffer
        manager.allocate_buffer(passage_id, file_path, 0, 44100).await.unwrap();

        // Transition through states
        manager.mark_ready(&passage_id).await.unwrap();
        assert_eq!(manager.get_status(&passage_id).await, Some(BufferStatus::Ready));

        manager.mark_playing(&passage_id).await.unwrap();
        assert_eq!(manager.get_status(&passage_id).await, Some(BufferStatus::Playing));

        manager.mark_exhausted(&passage_id).await.unwrap();
        assert_eq!(manager.get_status(&passage_id).await, Some(BufferStatus::Exhausted));
    }

    #[tokio::test]
    async fn test_memory_cleanup() {
        let manager = PassageBufferManager::new();

        // Create multiple buffers
        for i in 0..5 {
            let id = Uuid::new_v4();
            let path = PathBuf::from(format!("/test/file{}.mp3", i));
            manager.allocate_buffer(id, path, 0, 44100).await.unwrap();

            if i < 3 {
                manager.mark_exhausted(&id).await.unwrap();
            }
        }

        // Should have 5 buffers total
        let stats = manager.memory_stats().await;
        assert_eq!(stats.total_buffers, 5);
        assert_eq!(stats.exhausted_count, 3);

        // Clean up exhausted
        let cleaned = manager.cleanup_exhausted().await;
        assert_eq!(cleaned, 3);

        // Should have 2 buffers remaining
        let stats = manager.memory_stats().await;
        assert_eq!(stats.total_buffers, 2);
        assert_eq!(stats.exhausted_count, 0);
    }

    #[test]
    fn test_buffer_duration() {
        let mut buffer = PassageBuffer::new(
            Uuid::new_v4(),
            PathBuf::from("/test.mp3"),
            0,
            44100,
        );

        // Add 1 second of stereo audio (44100 * 2 channels)
        buffer.pcm_data.resize(88200, 0.0);

        assert_eq!(buffer.duration(), 1.0);
        assert_eq!(buffer.sample_count(), 44100);
    }
}