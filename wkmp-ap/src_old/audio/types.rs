//! Core audio data types
//!
//! Defines structures for audio buffers and frames used throughout the audio pipeline.
//!
//! **Traceability:**
//! - [SSD-FBUF-010] Full buffer contains entire passage in RAM
//! - [SSD-FBUF-020] All audio normalized to 44100 Hz
//! - [SSD-BUF-020] Buffer state transitions for event emission

use uuid::Uuid;

/// PassageBuffer holds decoded and resampled audio data ready for playback.
///
/// **[SSD-FBUF-010]** Full buffer contains entire passage in RAM for sample-accurate
/// seeking and crossfade operations.
///
/// **Format:**
/// - Samples are f32 (floating point -1.0 to 1.0)
/// - Stereo interleaved: [L, R, L, R, ...]
/// - Sample rate always 44100 Hz after resampling
#[derive(Debug, Clone)]
pub struct PassageBuffer {
    /// Passage UUID from database
    pub passage_id: Uuid,

    /// PCM audio samples (interleaved stereo)
    /// Index pattern: 0=left, 1=right, 2=left, 3=right, etc.
    pub samples: Vec<f32>,

    /// Sample rate (always 44100 after resampling)
    /// **[SSD-FBUF-020]** Standard rate for all playback
    pub sample_rate: u32,

    /// Channel count (always 2 for stereo)
    pub channel_count: u16,

    /// Number of stereo frames (samples.len() / 2)
    pub sample_count: usize,

    /// **[PCF-DUR-010]** Flag indicating if decode is complete
    /// When true, duration is cached and won't change even if buffer grows
    pub decode_complete: bool,

    /// **[PCF-COMP-010]** Total frames when decode completes (sentinel for completion)
    /// Set by finalize(), used for race-free exhaustion detection
    pub total_frames: Option<usize>,

    /// **[PCF-DUR-010]** Cached duration when decode completes
    /// Prevents duration from growing after decode finishes
    pub total_duration_ms: Option<u64>,
}

impl PassageBuffer {
    /// Create a new PassageBuffer from decoded and resampled audio data
    pub fn new(passage_id: Uuid, samples: Vec<f32>, sample_rate: u32, channel_count: u16) -> Self {
        let sample_count = samples.len() / channel_count as usize;

        Self {
            passage_id,
            samples,
            sample_rate,
            channel_count,
            sample_count,
            decode_complete: false,        // [PCF-DUR-010] Initially false, set by finalize()
            total_frames: None,            // [PCF-COMP-010] Set when decode completes
            total_duration_ms: None,       // [PCF-DUR-010] Cached when decode completes
        }
    }

    /// Get duration in milliseconds
    ///
    /// **[PCF-DUR-010]** After finalize() is called, returns cached duration
    /// to prevent duration from growing as buffer is appended to.
    pub fn duration_ms(&self) -> u64 {
        self.total_duration_ms.unwrap_or_else(|| {
            (self.sample_count as u64 * 1000) / self.sample_rate as u64
        })
    }

    /// Get duration in seconds
    pub fn duration_seconds(&self) -> f32 {
        self.sample_count as f32 / self.sample_rate as f32
    }

    /// Get audio frame at specific frame index
    pub fn get_frame(&self, frame_index: usize) -> Option<AudioFrame> {
        let sample_index = frame_index * 2;
        if sample_index + 1 < self.samples.len() {
            Some(AudioFrame {
                left: self.samples[sample_index],
                right: self.samples[sample_index + 1],
            })
        } else {
            None
        }
    }

    /// Append samples to buffer (for incremental decode)
    ///
    /// [SSD-PBUF-028] Support for partial buffer playback
    /// Allows decoder to progressively fill buffer as it decodes.
    ///
    /// # Arguments
    /// * `new_samples` - Stereo interleaved samples to append
    ///
    /// # Panics
    /// Panics if new_samples length is not even (must be stereo pairs)
    pub fn append_samples(&mut self, new_samples: Vec<f32>) {
        assert_eq!(new_samples.len() % 2, 0, "Samples must be stereo pairs");

        self.samples.extend(new_samples);
        self.sample_count = self.samples.len() / self.channel_count as usize;
    }

    /// Reserve capacity for expected total samples
    ///
    /// Optimization to reduce reallocations during incremental decode
    pub fn reserve_capacity(&mut self, total_frames: usize) {
        let total_samples = total_frames * self.channel_count as usize;
        self.samples.reserve(total_samples.saturating_sub(self.samples.len()));
    }

    /// Finalize buffer after decode completes
    ///
    /// **[PCF-DUR-010][PCF-COMP-010]** Sets the completion sentinel and caches duration.
    /// After finalize() is called:
    /// - `decode_complete` is true
    /// - `total_frames` holds the final frame count
    /// - `total_duration_ms` holds the cached duration
    /// - `duration_ms()` will return cached value even if buffer grows
    /// - `is_exhausted()` can accurately detect completion
    ///
    /// This method should be called by decoder when it reaches end of file.
    pub fn finalize(&mut self) {
        self.decode_complete = true;
        self.total_frames = Some(self.sample_count);
        self.total_duration_ms = Some(
            (self.sample_count as u64 * 1000) / self.sample_rate as u64
        );
    }

    /// Check if playback position has exhausted all available audio
    ///
    /// **[PCF-COMP-010]** Race-free completion detection using cached total_frames.
    ///
    /// Returns true if:
    /// - Decode is complete (finalized) AND
    /// - Position >= total_frames
    ///
    /// Returns false if decode is not yet complete (cannot be exhausted until
    /// we know the final length).
    ///
    /// # Arguments
    /// * `position` - Current playback position in frames
    pub fn is_exhausted(&self, position: usize) -> bool {
        if let Some(total) = self.total_frames {
            position >= total
        } else {
            false
        }
    }
}

/// AudioFrame represents a single stereo sample (one frame of audio).
///
/// Used for passing audio data between mixer and output device.
#[derive(Debug, Clone, Copy)]
pub struct AudioFrame {
    /// Left channel sample
    pub left: f32,

    /// Right channel sample
    pub right: f32,
}

impl AudioFrame {
    /// Create a silent frame (0.0, 0.0)
    pub fn zero() -> Self {
        AudioFrame { left: 0.0, right: 0.0 }
    }

    /// Create a frame from mono sample (duplicate to both channels)
    pub fn from_mono(sample: f32) -> Self {
        AudioFrame { left: sample, right: sample }
    }

    /// Create a frame from left and right samples
    pub fn from_stereo(left: f32, right: f32) -> Self {
        AudioFrame { left, right }
    }

    /// Apply volume scaling to both channels
    pub fn apply_volume(&mut self, volume: f32) {
        self.left *= volume;
        self.right *= volume;
    }

    /// Add another frame to this frame (for mixing)
    pub fn add(&mut self, other: &AudioFrame) {
        self.left += other.left;
        self.right += other.right;
    }

    /// Clamp samples to valid range [-1.0, 1.0] to prevent clipping
    pub fn clamp(&mut self) {
        self.left = self.left.clamp(-1.0, 1.0);
        self.right = self.right.clamp(-1.0, 1.0);
    }
}

/// BufferStatus tracks buffer decode/playback state.
///
/// **[SSD-BUF-020]** Buffer state transitions for event emission
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferStatus {
    /// Buffer is currently being decoded
    Decoding {
        /// Decode progress percentage (0-100)
        progress_percent: u8
    },

    /// Buffer is fully decoded and ready for playback
    Ready,

    /// Buffer is currently being played
    Playing,

    /// Buffer has been fully consumed (playback complete)
    Exhausted,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_passage_buffer_creation() {
        let passage_id = Uuid::new_v4();
        let samples = vec![0.5, -0.5, 0.25, -0.25]; // 2 stereo frames
        let buffer = PassageBuffer::new(passage_id, samples.clone(), 44100, 2);

        assert_eq!(buffer.passage_id, passage_id);
        assert_eq!(buffer.samples, samples);
        assert_eq!(buffer.sample_rate, 44100);
        assert_eq!(buffer.channel_count, 2);
        assert_eq!(buffer.sample_count, 2);
    }

    #[test]
    fn test_passage_buffer_duration() {
        let passage_id = Uuid::new_v4();
        // 44100 samples = 1 second at 44.1kHz
        let samples = vec![0.0; 44100 * 2]; // 2 channels
        let buffer = PassageBuffer::new(passage_id, samples, 44100, 2);

        assert_eq!(buffer.duration_ms(), 1000);
    }

    #[test]
    fn test_passage_buffer_get_frame() {
        let passage_id = Uuid::new_v4();
        let samples = vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6];
        let buffer = PassageBuffer::new(passage_id, samples, 44100, 2);

        let frame0 = buffer.get_frame(0).unwrap();
        assert_eq!(frame0.left, 0.1);
        assert_eq!(frame0.right, 0.2);

        let frame1 = buffer.get_frame(1).unwrap();
        assert_eq!(frame1.left, 0.3);
        assert_eq!(frame1.right, 0.4);

        let frame2 = buffer.get_frame(2).unwrap();
        assert_eq!(frame2.left, 0.5);
        assert_eq!(frame2.right, 0.6);

        // Out of bounds
        assert!(buffer.get_frame(3).is_none());
    }

    #[test]
    fn test_audio_frame_zero() {
        let frame = AudioFrame::zero();
        assert_eq!(frame.left, 0.0);
        assert_eq!(frame.right, 0.0);
    }

    #[test]
    fn test_audio_frame_from_mono() {
        let frame = AudioFrame::from_mono(0.5);
        assert_eq!(frame.left, 0.5);
        assert_eq!(frame.right, 0.5);
    }

    #[test]
    fn test_audio_frame_apply_volume() {
        let mut frame = AudioFrame::from_stereo(0.5, -0.5);
        frame.apply_volume(0.5);
        assert_eq!(frame.left, 0.25);
        assert_eq!(frame.right, -0.25);
    }

    #[test]
    fn test_audio_frame_add() {
        let mut frame1 = AudioFrame::from_stereo(0.3, 0.4);
        let frame2 = AudioFrame::from_stereo(0.2, 0.1);
        frame1.add(&frame2);
        assert_eq!(frame1.left, 0.5);
        assert_eq!(frame1.right, 0.5);
    }

    #[test]
    fn test_audio_frame_clamp() {
        let mut frame = AudioFrame::from_stereo(1.5, -1.5);
        frame.clamp();
        assert_eq!(frame.left, 1.0);
        assert_eq!(frame.right, -1.0);
    }

    // --- REV004: Tests for incremental buffer methods ---

    #[test]
    fn test_append_samples_basic() {
        let passage_id = Uuid::new_v4();
        let mut buffer = PassageBuffer::new(passage_id, vec![0.1, 0.2, 0.3, 0.4], 44100, 2);

        // Initial state: 2 stereo frames (4 samples)
        assert_eq!(buffer.sample_count, 2);
        assert_eq!(buffer.samples.len(), 4);

        // Append 2 more frames (4 samples)
        buffer.append_samples(vec![0.5, 0.6, 0.7, 0.8]);

        // Should now have 4 frames total
        assert_eq!(buffer.sample_count, 4);
        assert_eq!(buffer.samples.len(), 8);
        assert_eq!(buffer.samples[0], 0.1);  // Original data preserved
        assert_eq!(buffer.samples[1], 0.2);
        assert_eq!(buffer.samples[4], 0.5);  // New data appended
        assert_eq!(buffer.samples[5], 0.6);
    }

    #[test]
    fn test_append_samples_multiple_times() {
        let passage_id = Uuid::new_v4();
        let mut buffer = PassageBuffer::new(passage_id, vec![], 44100, 2);

        // Start with empty buffer
        assert_eq!(buffer.sample_count, 0);

        // Append first chunk (1 second @ 44.1kHz = 88200 samples)
        buffer.append_samples(vec![0.0; 88200]);
        assert_eq!(buffer.sample_count, 44100);  // 44100 frames
        assert_eq!(buffer.duration_ms(), 1000);   // 1 second

        // Append second chunk
        buffer.append_samples(vec![0.0; 88200]);
        assert_eq!(buffer.sample_count, 88200);  // 88200 frames
        assert_eq!(buffer.duration_ms(), 2000);   // 2 seconds

        // Append third chunk
        buffer.append_samples(vec![0.0; 88200]);
        assert_eq!(buffer.sample_count, 132300); // 132300 frames
        assert_eq!(buffer.duration_ms(), 3000);   // 3 seconds
    }

    #[test]
    fn test_append_samples_updates_duration() {
        let passage_id = Uuid::new_v4();
        let mut buffer = PassageBuffer::new(passage_id, vec![0.0; 88200], 44100, 2);

        // Initial duration: 1 second
        assert_eq!(buffer.duration_ms(), 1000);
        assert_eq!(buffer.duration_seconds(), 1.0);

        // Append 2 more seconds
        buffer.append_samples(vec![0.0; 176400]);  // 2 seconds worth

        // Duration should be 3 seconds
        assert_eq!(buffer.duration_ms(), 3000);
        assert_eq!(buffer.duration_seconds(), 3.0);
    }

    #[test]
    #[should_panic(expected = "Samples must be stereo pairs")]
    fn test_append_samples_panics_on_odd_length() {
        let passage_id = Uuid::new_v4();
        let mut buffer = PassageBuffer::new(passage_id, vec![], 44100, 2);

        // Attempt to append odd number of samples - should panic
        buffer.append_samples(vec![1.0, 2.0, 3.0]);  // 3 samples (not stereo pairs)
    }

    #[test]
    fn test_append_samples_empty_vec() {
        let passage_id = Uuid::new_v4();
        let mut buffer = PassageBuffer::new(passage_id, vec![0.1, 0.2], 44100, 2);

        // Append empty vector (valid - no-op)
        buffer.append_samples(vec![]);

        // Buffer unchanged
        assert_eq!(buffer.sample_count, 1);
        assert_eq!(buffer.samples.len(), 2);
    }

    #[test]
    fn test_reserve_capacity() {
        let passage_id = Uuid::new_v4();
        let mut buffer = PassageBuffer::new(passage_id, vec![], 44100, 2);

        // Reserve capacity for 10 seconds of audio
        let frames = 44100 * 10;  // 10 seconds @ 44.1kHz
        buffer.reserve_capacity(frames);

        // Capacity should be reserved (at least as much as requested)
        assert!(buffer.samples.capacity() >= frames * 2);  // *2 for stereo

        // Length should still be 0
        assert_eq!(buffer.samples.len(), 0);
        assert_eq!(buffer.sample_count, 0);
    }

    #[test]
    fn test_reserve_capacity_reduces_reallocations() {
        let passage_id = Uuid::new_v4();
        let mut buffer = PassageBuffer::new(passage_id, vec![], 44100, 2);

        // Reserve for full 10-second passage
        buffer.reserve_capacity(441000);  // 10 seconds of frames
        let capacity_after_reserve = buffer.samples.capacity();

        // Append 10 chunks of 1 second each
        for _ in 0..10 {
            buffer.append_samples(vec![0.0; 88200]);  // 1 second
        }

        // Capacity should not have grown (no reallocations needed)
        assert_eq!(buffer.samples.capacity(), capacity_after_reserve);
        assert_eq!(buffer.duration_ms(), 10000);
    }

    #[test]
    fn test_get_frame_after_append() {
        let passage_id = Uuid::new_v4();
        let mut buffer = PassageBuffer::new(passage_id, vec![0.1, 0.2], 44100, 2);

        // Append more frames
        buffer.append_samples(vec![0.3, 0.4, 0.5, 0.6]);

        // Should be able to access all frames
        let frame0 = buffer.get_frame(0).unwrap();
        assert_eq!(frame0.left, 0.1);
        assert_eq!(frame0.right, 0.2);

        let frame1 = buffer.get_frame(1).unwrap();
        assert_eq!(frame1.left, 0.3);
        assert_eq!(frame1.right, 0.4);

        let frame2 = buffer.get_frame(2).unwrap();
        assert_eq!(frame2.left, 0.5);
        assert_eq!(frame2.right, 0.6);

        // Out of bounds
        assert!(buffer.get_frame(3).is_none());
    }

    // --- [PCF-DUR-010] Tests for duration caching after decode completion ---

    #[test]
    fn test_duration_fixed_after_finalize() {
        let passage_id = Uuid::new_v4();
        let mut buffer = PassageBuffer::new(passage_id, vec![], 44100, 2);

        // Append first chunk (500ms)
        buffer.append_samples(vec![0.0; 88200]); // 44100 frames * 2 channels
        assert_eq!(buffer.duration_ms(), 1000);

        // Append second chunk (+500ms = 1000ms total)
        buffer.append_samples(vec![0.0; 88200]);
        assert_eq!(buffer.duration_ms(), 2000);  // Still growing

        // Finalize decode - duration should be locked
        buffer.finalize();
        assert_eq!(buffer.duration_ms(), 2000);  // Locked at 2000ms

        // Simulate decoder appending more (shouldn't happen, but test defensively)
        buffer.append_samples(vec![0.0; 88200]); // Buffer grows to 3000ms of samples

        // Duration should STILL be 2000ms (cached value)
        assert_eq!(buffer.duration_ms(), 2000);
    }

    #[test]
    fn test_finalize_sets_all_fields() {
        let passage_id = Uuid::new_v4();
        let mut buffer = PassageBuffer::new(passage_id, vec![0.0; 88200], 44100, 2);

        // Before finalize
        assert!(!buffer.decode_complete);
        assert_eq!(buffer.total_frames, None);
        assert_eq!(buffer.total_duration_ms, None);

        // Finalize
        buffer.finalize();

        // After finalize
        assert!(buffer.decode_complete);
        assert_eq!(buffer.total_frames, Some(44100));  // 44100 stereo frames
        assert_eq!(buffer.total_duration_ms, Some(1000));  // 1 second
    }

    // --- [PCF-COMP-010] Tests for completion detection ---

    #[test]
    fn test_is_exhausted_before_finalize() {
        let passage_id = Uuid::new_v4();
        let buffer = PassageBuffer::new(passage_id, vec![0.0; 88200], 44100, 2);

        // Not finalized yet, cannot be exhausted
        assert!(!buffer.is_exhausted(0));
        assert!(!buffer.is_exhausted(44100));
        assert!(!buffer.is_exhausted(100000));
    }

    #[test]
    fn test_is_exhausted_after_finalize() {
        let passage_id = Uuid::new_v4();
        let mut buffer = PassageBuffer::new(passage_id, vec![0.0; 88200], 44100, 2);

        buffer.finalize();
        assert_eq!(buffer.total_frames, Some(44100));

        // Check exhaustion at various positions
        assert!(!buffer.is_exhausted(0));       // Position at start
        assert!(!buffer.is_exhausted(10000));   // Position < total
        assert!(!buffer.is_exhausted(44099));   // Position just before end
        assert!(buffer.is_exhausted(44100));    // Position == total (exhausted)
        assert!(buffer.is_exhausted(50000));    // Position > total (also exhausted)
    }

    #[test]
    fn test_is_exhausted_exact_boundary() {
        let passage_id = Uuid::new_v4();
        let mut buffer = PassageBuffer::new(passage_id, vec![0.0; 176400], 44100, 2);  // 2 seconds

        buffer.finalize();
        assert_eq!(buffer.total_frames, Some(88200));  // 88200 stereo frames = 2 seconds

        // Position one frame before end
        assert!(!buffer.is_exhausted(88199));

        // Position exactly at end
        assert!(buffer.is_exhausted(88200));

        // Position one frame past end
        assert!(buffer.is_exhausted(88201));
    }

    #[test]
    fn test_finalize_with_empty_buffer() {
        let passage_id = Uuid::new_v4();
        let mut buffer = PassageBuffer::new(passage_id, vec![], 44100, 2);

        buffer.finalize();

        assert!(buffer.decode_complete);
        assert_eq!(buffer.total_frames, Some(0));
        assert_eq!(buffer.total_duration_ms, Some(0));

        // Empty buffer is immediately exhausted
        assert!(buffer.is_exhausted(0));
    }

    #[test]
    fn test_duration_uses_cached_value_after_finalize() {
        let passage_id = Uuid::new_v4();
        let mut buffer = PassageBuffer::new(passage_id, vec![0.0; 88200], 44100, 2);

        // Before finalize: calculated from current buffer
        let duration_before = buffer.duration_ms();
        assert_eq!(duration_before, 1000);

        // Finalize
        buffer.finalize();
        let duration_after = buffer.duration_ms();
        assert_eq!(duration_after, 1000);

        // Manually corrupt sample_count (simulate race condition)
        // This would normally cause duration to change, but cached value should be used
        buffer.sample_count = 88200; // Double the frames

        // Duration should STILL be 1000ms (using cached value, not recalculating)
        assert_eq!(buffer.duration_ms(), 1000);
    }
}
