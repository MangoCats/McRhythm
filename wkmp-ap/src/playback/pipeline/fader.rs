//! Fade processing for audio chunks
//!
//! Applies fade-in and fade-out curves to decoded audio chunks based on passage timing.
//! Maintains state across chunks for sample-accurate fade application.
//!
//! **Traceability:**
//! - [DBD-FADE-030] Pre-buffer fade-in application using passage timing
//! - [DBD-FADE-050] Pre-buffer fade-out application using passage timing
//! - [DBD-FADE-065] Use discovered endpoint for fade-out when endpoint is undefined
//! - [DBD-DEC-080] Sample-accurate fade timing
//! - [XFD-OV-020] Zero-duration timing behavior (pass-through mode)

use crate::db::passages::PassageWithTiming;
use tracing::{debug, warn};
use wkmp_common::FadeCurve;

/// Stateful fade processor for a single passage
///
/// Applies fade-in and fade-out curves to audio chunks as they pass through the pipeline.
/// Maintains frame position to ensure sample-accurate fade timing across chunk boundaries.
///
/// **[XFD-OV-020]** When fade durations are zero, operates in pass-through mode (no fades applied).
pub struct Fader {
    // Passage timing information (for debugging/logging)
    /// **Phase 4:** Timing fields reserved for passage validation diagnostics
    passage_start_ticks: i64,
    passage_end_ticks: Option<i64>,

    // Fade-in configuration
    fade_in_duration_samples: usize,
    fade_in_curve: FadeCurve,

    // Fade-out configuration
    fade_out_start_samples: usize,
    fade_out_duration_samples: usize,
    fade_out_curve: FadeCurve,

    // Total passage duration (for logging and validation)
    total_passage_duration_samples: usize,

    // State: current frame position in passage (stereo: 1 frame = 2 samples)
    current_frame: usize,

    // Sample rate (for logging conversions)
    /// **Phase 4:** Sample rate reserved for diagnostic time conversions (not yet logged)
    sample_rate: u32,

    // Pass-through mode flag (zero-duration fades)
    is_pass_through: bool,
}

impl Fader {
    /// Create a new Fader for the given passage
    ///
    /// # Arguments
    /// * `passage` - Passage timing information
    /// * `sample_rate` - Working sample rate (e.g., 44100 Hz)
    /// * `discovered_end_ticks` - Actual file endpoint discovered by decoder (for undefined endpoints)
    ///
    /// # Returns
    /// Fader configured for the passage's fade-in and fade-out curves
    pub fn new(
        passage: &PassageWithTiming,
        sample_rate: u32,
        discovered_end_ticks: Option<i64>,
    ) -> Self {
        // Calculate fade-in duration in samples
        let fade_in_duration_ticks = passage
            .fade_in_point_ticks
            .saturating_sub(passage.start_time_ticks);
        let fade_in_duration_samples =
            wkmp_common::timing::ticks_to_samples(fade_in_duration_ticks, sample_rate);

        // **[DBD-FADE-065]** Use discovered endpoint when passage endpoint is undefined
        let passage_end_ticks = if let Some(defined_end) = passage.end_time_ticks {
            debug!(
                "Using defined endpoint: {}ticks ({}ms)",
                defined_end,
                wkmp_common::timing::ticks_to_ms(defined_end)
            );
            defined_end
        } else if let Some(discovered_end) = discovered_end_ticks {
            debug!(
                "Using discovered endpoint for fades: {}ticks ({}ms)",
                discovered_end,
                wkmp_common::timing::ticks_to_ms(discovered_end)
            );
            discovered_end
        } else {
            // Fallback: should never happen in practice
            warn!("No defined or discovered endpoint available, using 10-second fallback");
            passage.start_time_ticks + wkmp_common::timing::ms_to_ticks(10000)
        };

        let passage_duration_ticks = passage_end_ticks.saturating_sub(passage.start_time_ticks);
        let passage_duration_samples =
            wkmp_common::timing::ticks_to_samples(passage_duration_ticks, sample_rate);

        // Calculate fade-out timing
        let fade_out_point_ticks = passage.fade_out_point_ticks.unwrap_or(passage_end_ticks);
        let fade_out_start_ticks = fade_out_point_ticks.saturating_sub(passage.start_time_ticks);
        let fade_out_start_samples =
            wkmp_common::timing::ticks_to_samples(fade_out_start_ticks, sample_rate);
        let fade_out_duration_samples =
            passage_duration_samples.saturating_sub(fade_out_start_samples);

        // Check if this is pass-through mode (zero-duration fades)
        // [XFD-OV-020] Zero-duration behavior
        let is_pass_through = fade_in_duration_samples == 0 && fade_out_duration_samples == 0;

        if is_pass_through {
            debug!(
                "Fader in pass-through mode (zero-duration fades): passage_duration={}ms",
                wkmp_common::timing::ticks_to_ms(passage_duration_ticks)
            );
        } else {
            debug!(
                "Fader initialized: fade_in={}samples ({}ms), fade_out_start={}samples ({}ms), \
                 fade_out_duration={}samples ({}ms), total_passage_duration={}samples ({}ms)",
                fade_in_duration_samples,
                wkmp_common::timing::ticks_to_ms(fade_in_duration_ticks),
                fade_out_start_samples,
                wkmp_common::timing::ticks_to_ms(fade_out_start_ticks),
                fade_out_duration_samples,
                wkmp_common::timing::ticks_to_ms(
                    passage_duration_ticks.saturating_sub(fade_out_start_ticks)
                ),
                passage_duration_samples,
                wkmp_common::timing::ticks_to_ms(passage_duration_ticks)
            );
        }

        Self {
            passage_start_ticks: passage.start_time_ticks,
            passage_end_ticks: Some(passage_end_ticks),
            fade_in_duration_samples,
            fade_in_curve: passage.fade_in_curve,
            fade_out_start_samples,
            fade_out_duration_samples,
            fade_out_curve: passage.fade_out_curve,
            total_passage_duration_samples: passage_duration_samples,
            current_frame: 0,
            sample_rate,
            is_pass_through,
        }
    }

    /// Process a chunk of audio, applying fades based on current position
    ///
    /// **[DBD-DEC-080]** Sample-accurate fade timing maintained across chunk boundaries.
    ///
    /// # Arguments
    /// * `samples` - Stereo interleaved samples (L, R, L, R, ...)
    ///
    /// # Returns
    /// Processed samples with fades applied (or original samples if pass-through)
    pub fn process_chunk(&mut self, mut samples: Vec<f32>) -> Vec<f32> {
        // [XFD-OV-020] Pass-through mode for zero-duration fades
        if self.is_pass_through {
            let frame_count = samples.len() / 2;
            self.current_frame += frame_count;
            return samples;
        }

        let frame_count = samples.len() / 2; // Stereo: 2 samples per frame

        // Apply fades frame by frame
        for frame_idx in 0..frame_count {
            let absolute_frame = self.current_frame + frame_idx;
            let mut fade_multiplier = 1.0;

            // **[DBD-FADE-030]** Fade-in
            if absolute_frame < self.fade_in_duration_samples {
                let fade_in_progress = if self.fade_in_duration_samples > 0 {
                    absolute_frame as f32 / self.fade_in_duration_samples as f32
                } else {
                    1.0
                };
                fade_multiplier *= self.fade_in_curve.calculate_fade_in(fade_in_progress);
            }

            // **[DBD-FADE-050]** Fade-out
            if absolute_frame >= self.fade_out_start_samples {
                let fade_out_position = absolute_frame.saturating_sub(self.fade_out_start_samples);
                let fade_out_progress = if self.fade_out_duration_samples > 0 {
                    fade_out_position as f32 / self.fade_out_duration_samples as f32
                } else {
                    1.0
                };
                fade_multiplier *= self.fade_out_curve.calculate_fade_out(fade_out_progress);
            }

            // Apply multiplier to stereo samples
            let sample_idx = frame_idx * 2;
            if sample_idx + 1 < samples.len() {
                samples[sample_idx] *= fade_multiplier; // Left
                samples[sample_idx + 1] *= fade_multiplier; // Right
            }
        }

        // Update state for next chunk
        self.current_frame += frame_count;

        samples
    }

    /// Check if this fader is in pass-through mode
    ///
    /// Returns true if both fade-in and fade-out durations are zero.
    /// **[XFD-OV-020]** Zero-duration timing behavior.
    pub fn is_pass_through(&self) -> bool {
        self.is_pass_through
    }

    /// Get current frame position (for debugging/validation)
    pub fn current_frame(&self) -> usize {
        self.current_frame
    }

    /// Get total passage duration in frames (for debugging/validation)
    ///
    /// **Phase 4:** Duration accessor reserved for diagnostics (not yet used)
    pub fn total_duration_frames(&self) -> usize {
        self.total_passage_duration_samples
    }

    /// Check if we've processed all expected frames for this passage
    ///
    /// Note: May return false positives if discovered endpoint differs from expected.
    ///
    /// **Phase 4:** Completion check reserved for diagnostics (not yet used)
    pub fn is_complete(&self) -> bool {
        self.current_frame >= self.total_passage_duration_samples
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_passage(
        fade_in_duration_ms: i64,
        fade_out_duration_ms: i64,
        total_duration_ms: i64,
    ) -> PassageWithTiming {
        let start_ticks = 0;
        let end_ticks = wkmp_common::timing::ms_to_ticks(total_duration_ms);
        let fade_in_point_ticks = wkmp_common::timing::ms_to_ticks(fade_in_duration_ms);
        let fade_out_point_ticks = end_ticks - wkmp_common::timing::ms_to_ticks(fade_out_duration_ms);

        PassageWithTiming {
            passage_id: None,
            file_path: PathBuf::from("/test/audio.mp3"),
            start_time_ticks: start_ticks,
            end_time_ticks: Some(end_ticks),
            lead_in_point_ticks: start_ticks,
            lead_out_point_ticks: Some(end_ticks),
            fade_in_point_ticks,
            fade_out_point_ticks: Some(fade_out_point_ticks),
            fade_in_curve: FadeCurve::Linear,
            fade_out_curve: FadeCurve::Linear,
        }
    }

    #[test]
    fn test_pass_through_mode_zero_duration_fades() {
        let passage = create_test_passage(0, 0, 5000); // No fades
        let fader = Fader::new(&passage, 44100, None);

        assert!(fader.is_pass_through());
    }

    #[test]
    fn test_pass_through_processes_without_modification() {
        let passage = create_test_passage(0, 0, 5000);
        let mut fader = Fader::new(&passage, 44100, None);

        let samples = vec![0.5, -0.5, 0.8, -0.8]; // 2 stereo frames
        let processed = fader.process_chunk(samples.clone());

        assert_eq!(processed, samples); // Should be unchanged
        assert_eq!(fader.current_frame(), 2); // Should advance position
    }

    #[test]
    fn test_fade_in_linear() {
        let passage = create_test_passage(1000, 0, 5000); // 1s fade-in
        let mut fader = Fader::new(&passage, 44100, None);

        // Create 100ms chunk at start (should be in fade-in region)
        let chunk_size = 4410 * 2; // 100ms of stereo samples
        let samples = vec![1.0; chunk_size]; // All at max volume

        let processed = fader.process_chunk(samples);

        // First sample should be at ~0.0 (start of fade)
        assert!(processed[0] < 0.1, "First sample should be near silence");

        // Last sample should be at ~0.1 (10% through 1s fade)
        assert!(processed[chunk_size - 1] > 0.05 && processed[chunk_size - 1] < 0.15);
    }

    #[test]
    fn test_fade_out_linear() {
        let passage = create_test_passage(0, 1000, 5000); // 1s fade-out at end
        let mut fader = Fader::new(&passage, 44100, None);

        // Skip INTO fade-out region (4.5s into 5s passage = halfway through 1s fade-out)
        let skip_frames = (44100.0 * 4.5) as usize; // 4.5 seconds
        fader.current_frame = skip_frames;

        // Create 100ms chunk (should be in fade-out region)
        let chunk_size = 4410 * 2; // 100ms stereo
        let samples = vec![1.0; chunk_size];

        let processed = fader.process_chunk(samples);

        // Should be fading out (multiplier < 1.0)
        // At 4.5s into 5s passage with 1s fade-out starting at 4s,
        // we're 0.5s into a 1s fade, so multiplier should be ~0.5
        assert!(processed[0] < 0.8 && processed[0] > 0.4, "Should be ~0.5 faded out at midpoint");
        assert!(processed[chunk_size - 1] < processed[0], "Should decrease over time");
    }

    #[test]
    fn test_multiple_chunks_maintain_position() {
        let passage = create_test_passage(1000, 1000, 5000);
        let mut fader = Fader::new(&passage, 44100, None);

        let chunk_size = 4410 * 2; // 100ms chunks

        for _ in 0..3 {
            let samples = vec![1.0; chunk_size];
            fader.process_chunk(samples);
        }

        // Should have processed 300ms = 13230 frames
        assert_eq!(fader.current_frame(), 13230);
    }

    #[test]
    fn test_discovered_endpoint_used_when_undefined() {
        let mut passage = create_test_passage(0, 1000, 5000);
        passage.end_time_ticks = None; // Undefined endpoint

        let discovered_end = wkmp_common::timing::ms_to_ticks(6000); // 6s discovered
        let fader = Fader::new(&passage, 44100, Some(discovered_end));

        // Should use discovered endpoint for fade-out calculation
        let expected_duration = wkmp_common::timing::ticks_to_samples(discovered_end, 44100);
        assert_eq!(fader.total_duration_frames(), expected_duration);
    }
}
