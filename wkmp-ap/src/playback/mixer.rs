//! Audio mixer for crossfade overlap and master volume
//!
//! Implements mixer per SPEC016 DBD-MIX-010.
//!
//! # Architecture
//!
//! Per SPEC016 DBD-MIX-041:
//! - Reads **pre-faded samples** from buffers (Fader applied curves before buffering)
//! - Sums overlapping samples (simple addition - no runtime fade calculations)
//! - Applies master volume
//! - Outputs single stream for audio device
//!
//! # Architectural Separation
//!
//! Per SPEC016 DBD-MIX-042:
//! - **Fader**: Applies passage-specific fade curves BEFORE buffering
//! - **Buffer**: Stores pre-faded samples
//! - **Mixer**: Reads pre-faded samples, sums overlaps, applies master volume
//!
//! # Phase 5 Scope
//!
//! Phase 5: Basic mixing (single passage, master volume)
//! Phase 6+: Crossfade overlap, pause mode decay, advanced features

use crate::{audio::RingBuffer, Result};
use wkmp_common::FadeCurve;

/// Audio mixer state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MixerState {
    /// Mixer is playing (reading from buffers)
    Playing,

    /// Mixer is paused (outputting silence)
    Paused,
}

/// Resume fade-in state
///
/// Tracks fade-in progress after resuming from pause.
/// Per SPEC016 [DBD-MIX-040], this is a mixer-level fade applied multiplicatively
/// to the final mixed output (orthogonal to passage-level fades applied by Fader).
#[derive(Debug, Clone)]
struct ResumeState {
    /// Fade-in duration in samples
    ///
    /// Example: 0.5s * 44100 = 22050 samples
    fade_duration_samples: usize,

    /// Fade-in curve (linear, exponential, cosine, etc.)
    fade_in_curve: FadeCurve,

    /// Number of samples processed since resume (for fade calculation)
    samples_since_resume: usize,
}

/// Audio mixer
///
/// Reads pre-faded samples from passage buffers, sums overlaps, applies master volume.
///
/// # Examples
///
/// ```ignore
/// let mut mixer = Mixer::new(1.0); // Master volume = 100%
///
/// // Mix samples for audio callback
/// mixer.mix_frame(&mut buffer)?;
/// ```
pub struct Mixer {
    /// Master volume (0.0 to 1.0)
    master_volume: f32,

    /// Mixer state (Playing/Paused)
    state: MixerState,

    /// Last output sample (for pause mode decay)
    last_sample_left: f32,
    last_sample_right: f32,

    /// Pause decay factor per SPEC016 DBD-PARAM-090
    pause_decay_factor: f32,

    /// Pause decay floor per SPEC016 DBD-PARAM-100
    pause_decay_floor: f32,

    /// Resume fade-in state
    ///
    /// Some when fading in after resume from pause, None otherwise.
    /// Per SPEC016 [DBD-MIX-040], this is a mixer-level fade (orthogonal to passage-level fades).
    resume_state: Option<ResumeState>,
}

impl Mixer {
    /// Create new mixer
    ///
    /// # Arguments
    ///
    /// * `master_volume` - Master volume (0.0 to 1.0)
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mixer = Mixer::new(1.0); // 100% volume
    /// let mixer = Mixer::new(0.5); // 50% volume
    /// ```
    pub fn new(master_volume: f32) -> Self {
        Self {
            master_volume: master_volume.clamp(0.0, 1.0),
            state: MixerState::Playing,
            last_sample_left: 0.0,
            last_sample_right: 0.0,
            pause_decay_factor: 0.96875, // 31/32 per SPEC016 DBD-PARAM-090
            pause_decay_floor: 0.0001778, // per SPEC016 DBD-PARAM-100
            resume_state: None,
        }
    }

    /// Set master volume
    ///
    /// # Arguments
    ///
    /// * `volume` - New master volume (0.0 to 1.0), will be clamped
    ///
    /// # Examples
    ///
    /// ```ignore
    /// mixer.set_master_volume(0.75); // 75% volume
    /// ```
    pub fn set_master_volume(&mut self, volume: f32) {
        self.master_volume = volume.clamp(0.0, 1.0);
    }

    /// Get master volume
    pub fn master_volume(&self) -> f32 {
        self.master_volume
    }

    /// Set mixer state
    ///
    /// # Arguments
    ///
    /// * `state` - New state (Playing/Paused)
    ///
    /// # Examples
    ///
    /// ```ignore
    /// mixer.set_state(MixerState::Paused);
    /// ```
    pub fn set_state(&mut self, state: MixerState) {
        self.state = state;
    }

    /// Get mixer state
    pub fn state(&self) -> MixerState {
        self.state
    }

    /// Start resume fade-in
    ///
    /// Initiates a fade-in from 0.0 to 1.0 applied multiplicatively to the final mixed output.
    /// This is a mixer-level fade (orthogonal to passage-level fades applied by Fader component).
    ///
    /// # Arguments
    ///
    /// * `fade_duration_samples` - Fade-in duration in samples (e.g., 22050 for 0.5s at 44.1kHz)
    /// * `fade_in_curve` - Fade curve to use (Linear, Exponential, etc.)
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Start 500ms fade-in at 44.1kHz
    /// mixer.start_resume_fade(22050, FadeCurve::Exponential);
    /// ```
    pub fn start_resume_fade(&mut self, fade_duration_samples: usize, fade_in_curve: FadeCurve) {
        self.resume_state = Some(ResumeState {
            fade_duration_samples,
            fade_in_curve,
            samples_since_resume: 0,
        });
    }

    /// Check if currently fading in from resume
    ///
    /// # Returns
    ///
    /// `true` if resume fade-in is active, `false` otherwise
    ///
    /// # Examples
    ///
    /// ```ignore
    /// if mixer.is_resume_fading() {
    ///     println!("Fading in from pause");
    /// }
    /// ```
    pub fn is_resume_fading(&self) -> bool {
        self.resume_state.is_some()
    }

    /// Mix single passage into output buffer
    ///
    /// Per SPEC016 DBD-MIX-040: Reads pre-faded samples, applies master volume.
    ///
    /// # Arguments
    ///
    /// * `passage_buffer` - Passage buffer with pre-faded samples
    /// * `output` - Output buffer to fill (interleaved stereo f32)
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mut output = vec![0.0f32; 1024]; // 512 stereo samples
    /// mixer.mix_single(&mut passage_buffer, &mut output)?;
    /// ```
    pub fn mix_single(&mut self, passage_buffer: &mut RingBuffer, output: &mut [f32]) -> Result<()> {
        // Validate stereo sample count
        if output.len() % 2 != 0 {
            return Err(crate::AudioPlayerError::Buffer(
                crate::error::BufferError::InvalidSampleCount(output.len()),
            ));
        }

        match self.state {
            MixerState::Playing => {
                // Read pre-faded samples from buffer
                let frames_requested = output.len() / 2;
                let samples = passage_buffer.pop(frames_requested)?;

                // Copy samples to output with master volume, then apply resume fade if active
                for (i, &sample) in samples.iter().enumerate() {
                    // Apply master volume first
                    let mut out_sample = sample * self.master_volume;

                    // Apply resume fade-in multiplicatively (mixer-level fade)
                    if let Some(ref resume) = self.resume_state {
                        if resume.samples_since_resume < resume.fade_duration_samples {
                            let fade_position = resume.samples_since_resume as f32
                                / resume.fade_duration_samples as f32;
                            let multiplier = resume.fade_in_curve.calculate_fade_in(fade_position);
                            out_sample *= multiplier;
                        }
                    }

                    output[i] = out_sample;
                }

                // Update resume fade progress and clear if complete
                if let Some(ref mut resume) = self.resume_state {
                    resume.samples_since_resume += samples.len();
                    if resume.samples_since_resume >= resume.fade_duration_samples {
                        self.resume_state = None; // Fade complete
                    }
                }

                // Store last sample for potential pause mode
                if !samples.is_empty() {
                    let last_idx = samples.len() - 2;
                    self.last_sample_left = samples[last_idx];
                    self.last_sample_right = samples[last_idx + 1];
                }

                // Fill remainder with silence if buffer underrun
                for i in samples.len()..output.len() {
                    output[i] = 0.0;
                }
            }

            MixerState::Paused => {
                // Pause mode: Exponential decay per SPEC016 DBD-MIX-050
                self.fill_pause_mode(output);
            }
        }

        Ok(())
    }

    /// Mix two passages with crossfade overlap
    ///
    /// Per SPEC016 DBD-MIX-041: Simple addition of pre-faded samples.
    ///
    /// # Arguments
    ///
    /// * `current_buffer` - Current passage buffer (fading out)
    /// * `next_buffer` - Next passage buffer (fading in)
    /// * `output` - Output buffer to fill
    ///
    /// # Examples
    ///
    /// ```ignore
    /// mixer.mix_crossfade(&mut current_buffer, &mut next_buffer, &mut output)?;
    /// ```
    pub fn mix_crossfade(
        &mut self,
        current_buffer: &mut RingBuffer,
        next_buffer: &mut RingBuffer,
        output: &mut [f32],
    ) -> Result<()> {
        // Validate stereo sample count
        if output.len() % 2 != 0 {
            return Err(crate::AudioPlayerError::Buffer(
                crate::error::BufferError::InvalidSampleCount(output.len()),
            ));
        }

        if self.state != MixerState::Playing {
            // Not playing, output silence
            self.fill_pause_mode(output);
            return Ok(());
        }

        let frames_requested = output.len() / 2;

        // Read pre-faded samples from both buffers
        let current_samples = current_buffer.pop(frames_requested)?;
        let next_samples = next_buffer.pop(frames_requested)?;

        // Mix: Simple addition per SPEC016 DBD-MIX-041
        // Both samples already have fade curves applied by Fader
        let min_len = current_samples.len().min(next_samples.len());

        for i in 0..min_len {
            // Sum pre-faded samples (crossfade overlap)
            let mixed = current_samples[i] + next_samples[i];

            // Apply master volume
            let mut out_sample = mixed * self.master_volume;

            // Apply resume fade-in multiplicatively (mixer-level fade)
            if let Some(ref resume) = self.resume_state {
                if resume.samples_since_resume < resume.fade_duration_samples {
                    let fade_position = resume.samples_since_resume as f32
                        / resume.fade_duration_samples as f32;
                    let multiplier = resume.fade_in_curve.calculate_fade_in(fade_position);
                    out_sample *= multiplier;
                }
            }

            output[i] = out_sample;
        }

        // Update resume fade progress and clear if complete
        if let Some(ref mut resume) = self.resume_state {
            resume.samples_since_resume += min_len;
            if resume.samples_since_resume >= resume.fade_duration_samples {
                self.resume_state = None; // Fade complete
            }
        }

        // Store last sample
        if min_len >= 2 {
            let last_idx = min_len - 2;
            self.last_sample_left = output[last_idx];
            self.last_sample_right = output[last_idx + 1];
        }

        // Fill remainder with silence
        for i in min_len..output.len() {
            output[i] = 0.0;
        }

        Ok(())
    }

    /// Fill output buffer with pause mode decay
    ///
    /// Per SPEC016 DBD-MIX-050: Exponential decay from last sample.
    fn fill_pause_mode(&mut self, output: &mut [f32]) {
        for i in (0..output.len()).step_by(2) {
            // Apply decay
            self.last_sample_left *= self.pause_decay_factor;
            self.last_sample_right *= self.pause_decay_factor;

            // Floor check per SPEC016 DBD-MIX-052
            if self.last_sample_left.abs() < self.pause_decay_floor {
                self.last_sample_left = 0.0;
            }
            if self.last_sample_right.abs() < self.pause_decay_floor {
                self.last_sample_right = 0.0;
            }

            output[i] = self.last_sample_left;
            output[i + 1] = self.last_sample_right;
        }
    }
}

// ========================================
// Tests
// ========================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mixer_creation() {
        let mixer = Mixer::new(1.0);
        assert_eq!(mixer.master_volume(), 1.0);
        assert_eq!(mixer.state(), MixerState::Playing);
    }

    #[test]
    fn test_master_volume_clamping() {
        let mut mixer = Mixer::new(1.5); // Over 1.0
        assert_eq!(mixer.master_volume(), 1.0);

        mixer.set_master_volume(-0.5); // Below 0.0
        assert_eq!(mixer.master_volume(), 0.0);

        mixer.set_master_volume(0.75);
        assert_eq!(mixer.master_volume(), 0.75);
    }

    #[test]
    fn test_mixer_state() {
        let mut mixer = Mixer::new(1.0);
        assert_eq!(mixer.state(), MixerState::Playing);

        mixer.set_state(MixerState::Paused);
        assert_eq!(mixer.state(), MixerState::Paused);
    }

    #[test]
    fn test_mix_single_odd_samples_fails() {
        let mut mixer = Mixer::new(1.0);
        let mut buffer = RingBuffer::new(1024);
        let mut output = vec![0.0f32; 7]; // Odd number

        let result = mixer.mix_single(&mut buffer, &mut output);
        assert!(result.is_err());
    }

    #[test]
    fn test_mix_crossfade_odd_samples_fails() {
        let mut mixer = Mixer::new(1.0);
        let mut buffer1 = RingBuffer::new(1024);
        let mut buffer2 = RingBuffer::new(1024);
        let mut output = vec![0.0f32; 7]; // Odd number

        let result = mixer.mix_crossfade(&mut buffer1, &mut buffer2, &mut output);
        assert!(result.is_err());
    }

    #[test]
    fn test_pause_mode_output() {
        let mut mixer = Mixer::new(1.0);

        // Set last sample
        mixer.last_sample_left = 1.0;
        mixer.last_sample_right = 1.0;

        // Enter pause mode
        mixer.set_state(MixerState::Paused);

        let mut output = vec![0.0f32; 8]; // 4 stereo samples
        mixer.fill_pause_mode(&mut output);

        // First sample should be decayed
        assert!(output[0] < 1.0);
        assert!(output[1] < 1.0);

        // Each subsequent sample should be smaller
        assert!(output[2] < output[0]);
        assert!(output[4] < output[2]);
    }

    // Note: Full integration tests with real buffers deferred to Phase 6+
}
