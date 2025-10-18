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
        }
    }

    /// Get duration in milliseconds
    pub fn duration_ms(&self) -> u64 {
        (self.sample_count as u64 * 1000) / self.sample_rate as u64
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
}
