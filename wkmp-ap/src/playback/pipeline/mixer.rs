//! Crossfade Mixer
//!
//! Implements sample-accurate crossfading between passages using a state machine.
//! Applies fade curves and mixes overlapping passages to produce a single audio stream.
//!
//! **Traceability:**
//! - [SSD-MIX-010] Crossfade mixer component
//! - [SSD-MIX-020] Mixer states
//! - [SSD-MIX-030] Single passage playback
//! - [SSD-MIX-040] Crossfade initiation
//! - [SSD-MIX-050] Sample-accurate mixing
//! - [SSD-MIX-060] Passage completion detection

use crate::audio::types::{AudioFrame, PassageBuffer};
use crate::error::Error;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use wkmp_common::FadeCurve;

/// Standard sample rate for all audio processing
const STANDARD_SAMPLE_RATE: u32 = 44100;

/// Crossfade mixer state machine
///
/// **[SSD-MIX-010]** Main component for sample-accurate mixing
pub struct CrossfadeMixer {
    /// Current mixer state
    state: MixerState,

    /// Sample rate (always 44100)
    sample_rate: u32,
}

/// Mixer state machine
///
/// **[SSD-MIX-020]** Three states for mixer operation:
/// - None: No audio playing
/// - SinglePassage: One passage playing (no crossfade)
/// - Crossfading: Two passages overlapping with fade curves
#[derive(Debug)]
enum MixerState {
    /// No audio playing
    None,

    /// Single passage playing (no crossfade)
    ///
    /// **[SSD-MIX-030]** Single passage state with optional fade-in
    SinglePassage {
        buffer: Arc<RwLock<PassageBuffer>>,
        passage_id: Uuid,
        position: usize, // Current frame position in buffer
        fade_in_curve: Option<FadeCurve>,
        fade_in_duration_samples: usize,
    },

    /// Crossfading between two passages
    ///
    /// **[SSD-MIX-040]** Crossfade state with dual buffer mixing
    Crossfading {
        // Current passage (fading out)
        current_buffer: Arc<RwLock<PassageBuffer>>,
        current_passage_id: Uuid,
        current_position: usize,
        fade_out_curve: FadeCurve,
        fade_out_duration_samples: usize,
        fade_out_progress: usize, // Samples into fade-out

        // Next passage (fading in)
        next_buffer: Arc<RwLock<PassageBuffer>>,
        next_passage_id: Uuid,
        next_position: usize,
        fade_in_curve: FadeCurve,
        fade_in_duration_samples: usize,
        fade_in_progress: usize, // Samples into fade-in
    },
}

impl CrossfadeMixer {
    /// Create new crossfade mixer
    pub fn new() -> Self {
        CrossfadeMixer {
            state: MixerState::None,
            sample_rate: STANDARD_SAMPLE_RATE,
        }
    }

    /// Start playing a passage (no crossfade)
    ///
    /// **[SSD-MIX-030]** Initiates single passage playback with optional fade-in
    ///
    /// # Arguments
    /// * `buffer` - Passage buffer to play
    /// * `passage_id` - UUID of passage
    /// * `fade_in_curve` - Optional fade-in curve
    /// * `fade_in_duration_ms` - Fade-in duration in milliseconds
    pub async fn start_passage(
        &mut self,
        buffer: Arc<RwLock<PassageBuffer>>,
        passage_id: Uuid,
        fade_in_curve: Option<FadeCurve>,
        fade_in_duration_ms: u32,
    ) {
        let fade_in_duration_samples = if fade_in_curve.is_some() {
            self.ms_to_samples(fade_in_duration_ms)
        } else {
            0
        };

        self.state = MixerState::SinglePassage {
            buffer,
            passage_id,
            position: 0,
            fade_in_curve,
            fade_in_duration_samples,
        };
    }

    /// Start crossfade to next passage
    ///
    /// **[SSD-MIX-040]** Transitions from SinglePassage to Crossfading state
    ///
    /// # Arguments
    /// * `next_buffer` - Buffer for next passage
    /// * `next_passage_id` - UUID of next passage
    /// * `fade_out_curve` - Curve for fading out current passage
    /// * `fade_out_duration_ms` - Fade-out duration in milliseconds
    /// * `fade_in_curve` - Curve for fading in next passage
    /// * `fade_in_duration_ms` - Fade-in duration in milliseconds
    ///
    /// # Returns
    /// Ok if crossfade started, Err if no passage is currently playing
    pub async fn start_crossfade(
        &mut self,
        next_buffer: Arc<RwLock<PassageBuffer>>,
        next_passage_id: Uuid,
        fade_out_curve: FadeCurve,
        fade_out_duration_ms: u32,
        fade_in_curve: FadeCurve,
        fade_in_duration_ms: u32,
    ) -> Result<(), Error> {
        match &self.state {
            MixerState::SinglePassage {
                buffer, passage_id, position, ..
            } => {
                self.state = MixerState::Crossfading {
                    current_buffer: buffer.clone(),
                    current_passage_id: *passage_id,
                    current_position: *position,
                    fade_out_curve,
                    fade_out_duration_samples: self.ms_to_samples(fade_out_duration_ms),
                    fade_out_progress: 0,
                    next_buffer,
                    next_passage_id,
                    next_position: 0,
                    fade_in_curve,
                    fade_in_duration_samples: self.ms_to_samples(fade_in_duration_ms),
                    fade_in_progress: 0,
                };
                Ok(())
            }
            _ => Err(Error::InvalidState(
                "Cannot start crossfade: no passage playing".to_string(),
            )),
        }
    }

    /// Get next audio frame
    ///
    /// **[SSD-MIX-050]** Sample-accurate mixing with fade curve application
    ///
    /// # Returns
    /// Next audio frame (stereo sample), or silence if no audio playing
    pub async fn get_next_frame(&mut self) -> AudioFrame {
        match &mut self.state {
            MixerState::None => AudioFrame::zero(),

            MixerState::SinglePassage {
                buffer,
                position,
                fade_in_curve,
                fade_in_duration_samples,
                ..
            } => {
                // Read frame from buffer
                let mut frame = read_frame(buffer, *position).await;

                // Apply fade-in if active
                if let Some(curve) = fade_in_curve {
                    if *position < *fade_in_duration_samples {
                        let fade_position = *position as f32 / *fade_in_duration_samples as f32;
                        let multiplier = curve.calculate_fade_in(fade_position);
                        frame.apply_volume(multiplier);
                    }
                }

                *position += 1;
                frame
            }

            MixerState::Crossfading {
                current_buffer,
                current_position,
                fade_out_curve,
                fade_out_duration_samples,
                fade_out_progress,
                next_buffer,
                next_passage_id,
                next_position,
                fade_in_curve,
                fade_in_duration_samples,
                fade_in_progress,
                ..
            } => {
                // Read from both buffers
                let mut current_frame = read_frame(current_buffer, *current_position).await;
                let mut next_frame = read_frame(next_buffer, *next_position).await;

                // Calculate fade positions (0.0 to 1.0)
                let fade_out_pos = *fade_out_progress as f32 / *fade_out_duration_samples as f32;
                let fade_in_pos = *fade_in_progress as f32 / *fade_in_duration_samples as f32;

                // Apply fade curves
                let fade_out_mult = fade_out_curve.calculate_fade_out(fade_out_pos);
                let fade_in_mult = fade_in_curve.calculate_fade_in(fade_in_pos);

                current_frame.apply_volume(fade_out_mult);
                next_frame.apply_volume(fade_in_mult);

                // Mix (sum) the frames
                let mut mixed = current_frame;
                mixed.add(&next_frame);

                // Check for clipping and clamp
                // [SSD-CLIP-010] Clipping detection
                if mixed.left.abs() > 1.0 || mixed.right.abs() > 1.0 {
                    // Log warning (only once per crossfade)
                    // TODO: Add clipping warning log
                }
                mixed.clamp();

                // Advance positions
                *current_position += 1;
                *next_position += 1;
                *fade_out_progress += 1;
                *fade_in_progress += 1;

                // Check if crossfade complete
                if *fade_out_progress >= *fade_out_duration_samples
                    && *fade_in_progress >= *fade_in_duration_samples
                {
                    // Transition to SinglePassage with next buffer
                    let new_passage_id = *next_passage_id;
                    let new_position = *next_position;
                    let new_buffer = next_buffer.clone();

                    self.state = MixerState::SinglePassage {
                        buffer: new_buffer,
                        passage_id: new_passage_id,
                        position: new_position,
                        fade_in_curve: None,
                        fade_in_duration_samples: 0,
                    };
                }

                mixed
            }
        }
    }

    /// Check if current passage finished
    ///
    /// **[SSD-MIX-060]** Passage completion detection
    ///
    /// # Returns
    /// true if current passage has been fully consumed
    pub async fn is_current_finished(&self) -> bool {
        match &self.state {
            MixerState::SinglePassage { buffer, position, .. } => {
                // Check if position >= buffer length
                if let Ok(buf) = buffer.try_read() {
                    *position >= buf.sample_count
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Stop playback
    ///
    /// Transitions to None state, stopping all audio
    pub fn stop(&mut self) {
        self.state = MixerState::None;
    }

    /// Get current passage ID (if any)
    ///
    /// # Returns
    /// UUID of currently playing passage, or None
    pub fn get_current_passage_id(&self) -> Option<Uuid> {
        match &self.state {
            MixerState::SinglePassage { passage_id, .. } => Some(*passage_id),
            MixerState::Crossfading {
                current_passage_id, ..
            } => Some(*current_passage_id),
            MixerState::None => None,
        }
    }

    /// Get next passage ID (during crossfade)
    ///
    /// # Returns
    /// UUID of next passage during crossfade, or None
    pub fn get_next_passage_id(&self) -> Option<Uuid> {
        match &self.state {
            MixerState::Crossfading {
                next_passage_id, ..
            } => Some(*next_passage_id),
            _ => None,
        }
    }

    /// Get current playback position in samples
    ///
    /// # Returns
    /// Current frame position, or 0 if not playing
    pub fn get_position(&self) -> usize {
        match &self.state {
            MixerState::SinglePassage { position, .. } => *position,
            MixerState::Crossfading {
                current_position, ..
            } => *current_position,
            MixerState::None => 0,
        }
    }

    /// Set playback position (seek)
    ///
    /// Updates the position in the current passage. Clamps to buffer bounds.
    /// Does nothing if mixer is idle.
    ///
    /// # Arguments
    /// * `position_frames` - Target position in frames
    ///
    /// # Returns
    /// * `Ok(())` if position updated
    /// * `Err` if position invalid or no passage playing
    ///
    /// **[SSD-ENG-026]** Seek position control
    pub async fn set_position(&mut self, position_frames: usize) -> crate::error::Result<()> {
        match &mut self.state {
            MixerState::SinglePassage {
                position, buffer, ..
            } => {
                // Clamp position to buffer bounds
                let buf = buffer.read().await;
                let max_position = buf.sample_count.saturating_sub(1);
                *position = position_frames.min(max_position);
                Ok(())
            }
            MixerState::Crossfading {
                current_position,
                current_buffer,
                ..
            } => {
                // During crossfade, only seek the current (fading out) passage
                // This maintains crossfade state while allowing position control
                let buf = current_buffer.read().await;
                let max_position = buf.sample_count.saturating_sub(1);
                *current_position = position_frames.min(max_position);
                Ok(())
            }
            MixerState::None => {
                Err(crate::error::Error::Playback(
                    "Cannot seek: no passage playing".to_string(),
                ))
            }
        }
    }

    /// Check if currently crossfading
    ///
    /// # Returns
    /// true if in Crossfading state
    pub fn is_crossfading(&self) -> bool {
        matches!(self.state, MixerState::Crossfading { .. })
    }

    // Helper methods

    /// Convert milliseconds to samples
    fn ms_to_samples(&self, ms: u32) -> usize {
        ((ms as f32 / 1000.0) * self.sample_rate as f32) as usize
    }
}

/// Read frame from buffer at position
///
/// Returns zero frame if position out of bounds or buffer locked
async fn read_frame(buffer: &Arc<RwLock<PassageBuffer>>, position: usize) -> AudioFrame {
    if let Ok(buf) = buffer.try_read() {
        buf.get_frame(position).unwrap_or_else(AudioFrame::zero)
    } else {
        AudioFrame::zero()
    }
}

impl Default for CrossfadeMixer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::types::PassageBuffer;

    /// Create a test buffer with sine wave samples
    fn create_test_buffer(passage_id: Uuid, sample_count: usize, amplitude: f32) -> Arc<RwLock<PassageBuffer>> {
        let mut samples = Vec::with_capacity(sample_count * 2);
        for i in 0..sample_count {
            let value = amplitude * (i as f32 * 0.01).sin();
            samples.push(value); // left
            samples.push(value); // right
        }

        Arc::new(RwLock::new(PassageBuffer::new(
            passage_id,
            samples,
            44100,
            2,
        )))
    }

    #[tokio::test]
    async fn test_mixer_creation() {
        let mixer = CrossfadeMixer::new();
        assert_eq!(mixer.sample_rate, 44100);
        assert!(mixer.get_current_passage_id().is_none());
    }

    #[tokio::test]
    async fn test_start_passage_no_fade() {
        let mut mixer = CrossfadeMixer::new();
        let passage_id = Uuid::new_v4();
        let buffer = create_test_buffer(passage_id, 1000, 0.5);

        mixer
            .start_passage(buffer, passage_id, None, 0)
            .await;

        assert_eq!(mixer.get_current_passage_id(), Some(passage_id));
        assert_eq!(mixer.get_position(), 0);
        assert!(!mixer.is_crossfading());
    }

    #[tokio::test]
    async fn test_start_passage_with_fade_in() {
        let mut mixer = CrossfadeMixer::new();
        let passage_id = Uuid::new_v4();
        let buffer = create_test_buffer(passage_id, 1000, 0.5);

        mixer
            .start_passage(buffer, passage_id, Some(FadeCurve::Linear), 100)
            .await;

        assert_eq!(mixer.get_current_passage_id(), Some(passage_id));

        // First frame should be silent (fade-in start)
        let frame = mixer.get_next_frame().await;
        assert_eq!(frame.left, 0.0);
        assert_eq!(frame.right, 0.0);
    }

    #[tokio::test]
    async fn test_single_passage_playback() {
        let mut mixer = CrossfadeMixer::new();
        let passage_id = Uuid::new_v4();
        let buffer = create_test_buffer(passage_id, 100, 0.5);

        mixer
            .start_passage(buffer, passage_id, None, 0)
            .await;

        // Read 50 frames
        for _ in 0..50 {
            let frame = mixer.get_next_frame().await;
            assert!(frame.left.abs() <= 1.0);
            assert!(frame.right.abs() <= 1.0);
        }

        assert_eq!(mixer.get_position(), 50);
        assert!(!mixer.is_current_finished().await);

        // Read remaining frames
        for _ in 0..50 {
            mixer.get_next_frame().await;
        }

        assert_eq!(mixer.get_position(), 100);
        assert!(mixer.is_current_finished().await);
    }

    #[tokio::test]
    async fn test_start_crossfade() {
        let mut mixer = CrossfadeMixer::new();
        let passage_id_1 = Uuid::new_v4();
        let passage_id_2 = Uuid::new_v4();
        let buffer1 = create_test_buffer(passage_id_1, 1000, 0.5);
        let buffer2 = create_test_buffer(passage_id_2, 1000, 0.5);

        // Start first passage
        mixer
            .start_passage(buffer1, passage_id_1, None, 0)
            .await;

        // Start crossfade
        let result = mixer
            .start_crossfade(
                buffer2,
                passage_id_2,
                FadeCurve::Linear,
                1000,
                FadeCurve::Linear,
                1000,
            )
            .await;

        assert!(result.is_ok());
        assert!(mixer.is_crossfading());
        assert_eq!(mixer.get_current_passage_id(), Some(passage_id_1));
        assert_eq!(mixer.get_next_passage_id(), Some(passage_id_2));
    }

    #[tokio::test]
    async fn test_crossfade_mixing() {
        let mut mixer = CrossfadeMixer::new();
        let passage_id_1 = Uuid::new_v4();
        let passage_id_2 = Uuid::new_v4();
        let buffer1 = create_test_buffer(passage_id_1, 1000, 0.5);
        let buffer2 = create_test_buffer(passage_id_2, 1000, 0.5);

        // Start first passage
        mixer
            .start_passage(buffer1, passage_id_1, None, 0)
            .await;

        // Start crossfade with 100ms duration (4410 samples)
        mixer
            .start_crossfade(
                buffer2,
                passage_id_2,
                FadeCurve::Linear,
                100,
                FadeCurve::Linear,
                100,
            )
            .await
            .unwrap();

        // Read frames during crossfade
        let crossfade_samples = 4410;
        for _ in 0..crossfade_samples {
            let frame = mixer.get_next_frame().await;
            // Frames should be within valid range
            assert!(frame.left.abs() <= 1.0);
            assert!(frame.right.abs() <= 1.0);
        }

        // Crossfade should be complete, now on second passage
        assert!(!mixer.is_crossfading());
        assert_eq!(mixer.get_current_passage_id(), Some(passage_id_2));
    }

    #[tokio::test]
    async fn test_crossfade_transition_to_single_passage() {
        let mut mixer = CrossfadeMixer::new();
        let passage_id_1 = Uuid::new_v4();
        let passage_id_2 = Uuid::new_v4();
        let buffer1 = create_test_buffer(passage_id_1, 100, 0.5);
        let buffer2 = create_test_buffer(passage_id_2, 100, 0.5);

        mixer
            .start_passage(buffer1, passage_id_1, None, 0)
            .await;

        mixer
            .start_crossfade(
                buffer2,
                passage_id_2,
                FadeCurve::Linear,
                10,
                FadeCurve::Linear,
                10,
            )
            .await
            .unwrap();

        // 10ms at 44100 = 441 samples
        for _ in 0..441 {
            mixer.get_next_frame().await;
        }

        // Should transition to single passage
        assert!(!mixer.is_crossfading());
        assert_eq!(mixer.get_current_passage_id(), Some(passage_id_2));
    }

    #[tokio::test]
    async fn test_stop() {
        let mut mixer = CrossfadeMixer::new();
        let passage_id = Uuid::new_v4();
        let buffer = create_test_buffer(passage_id, 1000, 0.5);

        mixer
            .start_passage(buffer, passage_id, None, 0)
            .await;

        assert!(mixer.get_current_passage_id().is_some());

        mixer.stop();

        assert!(mixer.get_current_passage_id().is_none());
        assert_eq!(mixer.get_position(), 0);

        // Next frame should be silent
        let frame = mixer.get_next_frame().await;
        assert_eq!(frame.left, 0.0);
        assert_eq!(frame.right, 0.0);
    }

    #[tokio::test]
    async fn test_start_crossfade_invalid_state() {
        let mut mixer = CrossfadeMixer::new();
        let passage_id = Uuid::new_v4();
        let buffer = create_test_buffer(passage_id, 1000, 0.5);

        // Try to start crossfade when nothing is playing
        let result = mixer
            .start_crossfade(
                buffer,
                passage_id,
                FadeCurve::Linear,
                1000,
                FadeCurve::Linear,
                1000,
            )
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_ms_to_samples() {
        let mixer = CrossfadeMixer::new();

        // 1 second = 44100 samples
        assert_eq!(mixer.ms_to_samples(1000), 44100);

        // 100ms = 4410 samples
        assert_eq!(mixer.ms_to_samples(100), 4410);

        // 10ms = 441 samples
        assert_eq!(mixer.ms_to_samples(10), 441);
    }

    #[tokio::test]
    async fn test_set_position() {
        let mut mixer = CrossfadeMixer::new();
        let passage_id = Uuid::new_v4();
        let buffer = create_test_buffer(passage_id, 1000, 0.5);

        // Start passage
        mixer.start_passage(buffer, passage_id, None, 0).await;
        assert_eq!(mixer.get_position(), 0);

        // Seek to position 100
        mixer.set_position(100).await.unwrap();
        assert_eq!(mixer.get_position(), 100);

        // Seek to position 500
        mixer.set_position(500).await.unwrap();
        assert_eq!(mixer.get_position(), 500);

        // Seek beyond buffer (should clamp to max-1)
        mixer.set_position(5000).await.unwrap();
        assert_eq!(mixer.get_position(), 999); // buffer has 1000 frames (0-999)

        // Seek back to 0
        mixer.set_position(0).await.unwrap();
        assert_eq!(mixer.get_position(), 0);
    }

    #[tokio::test]
    async fn test_set_position_no_passage() {
        let mut mixer = CrossfadeMixer::new();

        // Try to seek when nothing is playing
        let result = mixer.set_position(100).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_set_position_during_crossfade() {
        let mut mixer = CrossfadeMixer::new();
        let passage_id_1 = Uuid::new_v4();
        let passage_id_2 = Uuid::new_v4();
        let buffer1 = create_test_buffer(passage_id_1, 1000, 0.5);
        let buffer2 = create_test_buffer(passage_id_2, 1000, 0.5);

        // Start first passage and play a bit
        mixer.start_passage(buffer1, passage_id_1, None, 0).await;
        for _ in 0..100 {
            mixer.get_next_frame().await;
        }

        // Start crossfade
        mixer
            .start_crossfade(
                buffer2,
                passage_id_2,
                FadeCurve::Linear,
                1000,
                FadeCurve::Linear,
                1000,
            )
            .await
            .unwrap();

        assert!(mixer.is_crossfading());

        // Seek during crossfade (should seek the current/fading out passage)
        mixer.set_position(200).await.unwrap();
        assert_eq!(mixer.get_position(), 200);
    }
}
