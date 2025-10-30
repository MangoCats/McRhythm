//! Audio resampling using rubato
//!
//! Converts audio to the standard 44.1kHz sample rate for playback.
//!
//! **Traceability:**
//! - [SSD-FBUF-020] All audio normalized to 44100 Hz
//! - [SSD-FBUF-021] High-quality resampling for playback

use crate::error::{Error, Result};
use rubato::{
    FastFixedIn, Resampler as RubatoResampler, SincFixedIn, SincInterpolationParameters,
    SincInterpolationType, WindowFunction,
};
use tracing::debug;

/// Standard output sample rate for all audio
/// **[SSD-FBUF-020]**
///
/// **Phase 4:** Target sample rate constant reserved for future configuration flexibility
#[allow(dead_code)]
pub const TARGET_SAMPLE_RATE: u32 = 44100;

/// Stateful audio resampler that maintains filter state across chunks
///
/// Prevents phase discontinuities by reusing the same rubato resampler instance
/// across multiple chunk processing calls. This is critical for high-quality
/// streaming audio where chunks are processed sequentially.
///
/// **Architecture:** Used in the new single-threaded decoder pipeline to ensure
/// seamless resampling across chunk boundaries.
pub enum StatefulResampler {
    /// No resampling needed (input rate == output rate)
    ///
    /// Simply copies input to output without processing.
    ///
    /// **Phase 4:** Channels field reserved for future multi-channel support beyond stereo
    PassThrough {
        #[allow(dead_code)]
        channels: u16
    },

    /// Active resampling with maintained filter state
    ///
    /// Reuses the same rubato resampler instance to preserve filter state.
    ///
    /// **Phase 4:** Rate/channel/chunk fields reserved for future telemetry and diagnostics
    Active {
        resampler: FastFixedIn<f32>,
        #[allow(dead_code)]
        input_rate: u32,
        #[allow(dead_code)]
        output_rate: u32,
        channels: u16,
        #[allow(dead_code)]
        chunk_size: usize,
    },
}

impl StatefulResampler {
    /// Create a new stateful resampler
    ///
    /// # Arguments
    /// * `input_rate` - Input sample rate (e.g., 48000 Hz)
    /// * `output_rate` - Output sample rate (typically 44100 Hz)
    /// * `channels` - Number of channels (typically 2 for stereo)
    /// * `chunk_size` - Expected chunk size in frames (for pre-allocation)
    ///
    /// # Returns
    /// Resampler instance (either PassThrough or Active based on rates)
    pub fn new(
        input_rate: u32,
        output_rate: u32,
        channels: u16,
        chunk_size: usize,
    ) -> Result<Self> {
        if input_rate == output_rate {
            debug!(
                "Creating pass-through resampler ({}Hz, {} channels)",
                input_rate, channels
            );
            Ok(Self::PassThrough { channels })
        } else {
            debug!(
                "Creating stateful resampler: {}Hz -> {}Hz ({} channels, chunk_size={})",
                input_rate, output_rate, channels, chunk_size
            );
            let resampler = Resampler::create_resampler(
                input_rate,
                output_rate,
                channels,
                chunk_size,
            )?;
            Ok(Self::Active {
                resampler,
                input_rate,
                output_rate,
                channels,
                chunk_size,
            })
        }
    }

    /// Process a chunk of audio, maintaining filter state
    ///
    /// # Arguments
    /// * `input` - Interleaved audio samples to process
    ///
    /// # Returns
    /// Resampled audio at output rate (or copy if pass-through)
    ///
    /// # Notes
    /// - Filter state is preserved between calls for seamless streaming
    /// - **Input chunk size must match the chunk_size specified in `new()`** for Active resamplers
    /// - PassThrough mode accepts any chunk size
    pub fn process_chunk(&mut self, input: &[f32]) -> Result<Vec<f32>> {
        match self {
            Self::PassThrough { .. } => {
                // No resampling needed, just copy
                Ok(input.to_vec())
            }
            Self::Active {
                resampler,
                channels,
                ..
            } => {
                // De-interleave samples for rubato
                let planar_input = Resampler::deinterleave(input, *channels);

                // Process through stateful resampler
                let planar_output = resampler
                    .process(&planar_input, None)
                    .map_err(|e| Error::Decode(format!("Resampling failed: {}", e)))?;

                // Re-interleave output
                Ok(Resampler::interleave(planar_output))
            }
        }
    }

    /// Check if this resampler is in pass-through mode
    pub fn is_pass_through(&self) -> bool {
        matches!(self, Self::PassThrough { .. })
    }

    /// Get the output rate for this resampler
    ///
    /// **Phase 4:** Output rate accessor reserved for future telemetry and diagnostics
    #[allow(dead_code)]
    pub fn output_rate(&self) -> u32 {
        match self {
            Self::PassThrough { .. } => TARGET_SAMPLE_RATE,
            Self::Active { output_rate, .. } => *output_rate,
        }
    }

    /// Get the input rate for this resampler
    ///
    /// **Phase 4:** Input rate accessor reserved for future telemetry and diagnostics
    #[allow(dead_code)]
    pub fn input_rate(&self) -> u32 {
        match self {
            Self::PassThrough { .. } => TARGET_SAMPLE_RATE,
            Self::Active { input_rate, .. } => *input_rate,
        }
    }
}

/// Audio resampler using rubato for high-quality sample rate conversion.
///
/// **Note:** For streaming use cases, prefer `StatefulResampler` to maintain
/// filter state across chunks and avoid phase discontinuities.
pub struct Resampler;

impl Resampler {
    /// Resample audio to target sample rate (44.1kHz).
    ///
    /// **[SSD-FBUF-020]** All audio normalized to 44100 Hz for consistent playback.
    ///
    /// # Arguments
    /// - `input`: Interleaved audio samples
    /// - `input_rate`: Input sample rate
    /// - `channels`: Number of channels (typically 2 for stereo)
    ///
    /// # Returns
    /// Resampled interleaved audio at 44.1kHz
    ///
    /// # Notes
    /// If input is already at 44.1kHz, returns a copy without resampling
    ///
    /// **Phase 4:** One-shot resampling reserved for future features (superseded by StatefulResampler)
    #[allow(dead_code)]
    pub fn resample(input: &[f32], input_rate: u32, channels: u16) -> Result<Vec<f32>> {
        let output_rate = TARGET_SAMPLE_RATE;

        // If already at target rate, return copy
        if input_rate == output_rate {
            debug!("Sample rate already at {}Hz, skipping resample", output_rate);
            return Ok(input.to_vec());
        }

        debug!(
            "Resampling from {}Hz to {}Hz ({} channels)",
            input_rate, output_rate, channels
        );

        // De-interleave samples for rubato (which expects planar format)
        let planar_input = Self::deinterleave(input, channels);

        // Calculate the number of frames in the input
        let input_frames = planar_input[0].len();

        // Create resampler
        let mut resampler = Self::create_resampler(input_rate, output_rate, channels, input_frames)?;

        // Resample each channel
        let planar_output = resampler
            .process(&planar_input, None)
            .map_err(|e| Error::Decode(format!("Resampling failed: {}", e)))?;

        // Re-interleave samples
        let interleaved_output = Self::interleave(planar_output);

        debug!(
            "Resampled {} input frames to {} output frames",
            input_frames,
            interleaved_output.len() / channels as usize
        );

        Ok(interleaved_output)
    }

    /// Create a rubato resampler.
    ///
    /// Uses FastFixedIn for efficiency (good quality/performance tradeoff).
    /// For highest quality, could use SincFixedIn but with higher CPU cost.
    ///
    /// **[REQ-AP-ERR-050]** Returns ResamplingInitFailed on initialization errors
    fn create_resampler(
        input_rate: u32,
        output_rate: u32,
        channels: u16,
        chunk_size: usize,
    ) -> Result<FastFixedIn<f32>> {
        // Use FastFixedIn for good quality and efficiency
        // Alternative: SincFixedIn for highest quality (slower)
        let resampler = FastFixedIn::<f32>::new(
            output_rate as f64 / input_rate as f64,
            1.0, // max_relative_ratio (no runtime changes)
            rubato::PolynomialDegree::Septic, // High quality polynomial
            chunk_size,
            channels as usize,
        )
        .map_err(|e| Error::ResamplingInitFailed {
            source_rate: input_rate,
            target_rate: output_rate,
            message: format!("Failed to create resampler: {}", e),
        })?;

        Ok(resampler)
    }

    /// Create a high-quality sinc resampler (alternative to FastFixedIn).
    ///
    /// This provides the highest quality but is more CPU intensive.
    /// Currently not used, but available for future quality improvements.
    #[allow(dead_code)]
    fn create_sinc_resampler(
        input_rate: u32,
        output_rate: u32,
        channels: u16,
        chunk_size: usize,
    ) -> Result<SincFixedIn<f32>> {
        let params = SincInterpolationParameters {
            sinc_len: 256,
            f_cutoff: 0.95,
            interpolation: SincInterpolationType::Linear,
            oversampling_factor: 256,
            window: WindowFunction::BlackmanHarris2,
        };

        let resampler = SincFixedIn::<f32>::new(
            output_rate as f64 / input_rate as f64,
            1.0,
            params,
            chunk_size,
            channels as usize,
        )
        .map_err(|e| Error::Decode(format!("Failed to create sinc resampler: {}", e)))?;

        Ok(resampler)
    }

    /// Convert interleaved samples to planar format.
    ///
    /// Input:  [L, R, L, R, L, R, ...]
    /// Output: [[L, L, L, ...], [R, R, R, ...]]
    fn deinterleave(samples: &[f32], channels: u16) -> Vec<Vec<f32>> {
        let num_channels = channels as usize;
        let num_frames = samples.len() / num_channels;

        let mut planar = vec![Vec::with_capacity(num_frames); num_channels];

        for frame_idx in 0..num_frames {
            for ch_idx in 0..num_channels {
                let sample = samples[frame_idx * num_channels + ch_idx];
                planar[ch_idx].push(sample);
            }
        }

        planar
    }

    /// Convert planar samples to interleaved format.
    ///
    /// Input:  [[L, L, L, ...], [R, R, R, ...]]
    /// Output: [L, R, L, R, L, R, ...]
    fn interleave(planar: Vec<Vec<f32>>) -> Vec<f32> {
        if planar.is_empty() {
            return Vec::new();
        }

        let num_channels = planar.len();
        let num_frames = planar[0].len();
        let mut interleaved = Vec::with_capacity(num_frames * num_channels);

        for frame_idx in 0..num_frames {
            for ch_idx in 0..num_channels {
                interleaved.push(planar[ch_idx][frame_idx]);
            }
        }

        interleaved
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deinterleave() {
        let interleaved = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]; // 3 stereo frames
        let planar = Resampler::deinterleave(&interleaved, 2);

        assert_eq!(planar.len(), 2); // 2 channels
        assert_eq!(planar[0], vec![1.0, 3.0, 5.0]); // Left channel
        assert_eq!(planar[1], vec![2.0, 4.0, 6.0]); // Right channel
    }

    #[test]
    fn test_interleave() {
        let planar = vec![vec![1.0, 3.0, 5.0], vec![2.0, 4.0, 6.0]];
        let interleaved = Resampler::interleave(planar);

        assert_eq!(interleaved, vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    }

    #[test]
    fn test_resample_same_rate() {
        let input = vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6];
        let output = Resampler::resample(&input, 44100, 2).unwrap();

        // Should return copy when already at target rate
        assert_eq!(output, input);
    }

    #[test]
    fn test_resample_different_rate() {
        // Create a simple sine wave at 48kHz
        let input_rate = 48000;
        let channels = 2;
        let duration_frames = 1000;

        let mut input = Vec::with_capacity(duration_frames * channels);
        for i in 0..duration_frames {
            let t = i as f32 / input_rate as f32;
            let sample = (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.5;
            input.push(sample); // Left
            input.push(sample); // Right
        }

        let output = Resampler::resample(&input, input_rate, 2).unwrap();

        // Output should be roughly (44100/48000) times the input length
        let expected_frames = (duration_frames as f64 * 44100.0 / input_rate as f64) as usize;
        let output_frames = output.len() / channels;

        // Allow some variance due to resampler internals
        assert!(
            output_frames >= expected_frames - 10 && output_frames <= expected_frames + 10,
            "Expected ~{} frames, got {}",
            expected_frames,
            output_frames
        );
    }

    #[test]
    fn test_deinterleave_mono() {
        let interleaved = vec![1.0, 2.0, 3.0, 4.0];
        let planar = Resampler::deinterleave(&interleaved, 1);

        assert_eq!(planar.len(), 1);
        assert_eq!(planar[0], vec![1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn test_interleave_empty() {
        let planar: Vec<Vec<f32>> = vec![];
        let interleaved = Resampler::interleave(planar);

        assert_eq!(interleaved, Vec::<f32>::new());
    }

    // StatefulResampler tests
    #[test]
    fn test_stateful_resampler_pass_through() {
        let mut resampler = StatefulResampler::new(44100, 44100, 2, 1000).unwrap();

        assert!(resampler.is_pass_through());
        assert_eq!(resampler.input_rate(), 44100);
        assert_eq!(resampler.output_rate(), 44100);

        let input = vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6];
        let output = resampler.process_chunk(&input).unwrap();

        assert_eq!(output, input);
    }

    #[test]
    fn test_stateful_resampler_active() {
        let mut resampler = StatefulResampler::new(48000, 44100, 2, 1000).unwrap();

        assert!(!resampler.is_pass_through());
        assert_eq!(resampler.input_rate(), 48000);
        assert_eq!(resampler.output_rate(), 44100);
    }

    #[test]
    fn test_stateful_resampler_maintains_state_across_chunks() {
        let mut resampler = StatefulResampler::new(48000, 44100, 2, 1000).unwrap();

        // Process multiple chunks
        for _ in 0..5 {
            let input = vec![0.1; 2000]; // 1000 stereo frames
            let output = resampler.process_chunk(&input).unwrap();

            // Output should be resampled (roughly 44100/48000 ratio)
            let expected_frames = (1000.0 * 44100.0 / 48000.0) as usize;
            let output_frames = output.len() / 2;

            // Allow variance for filter state
            assert!(
                output_frames >= expected_frames - 50 && output_frames <= expected_frames + 50,
                "Expected ~{} frames, got {}",
                expected_frames,
                output_frames
            );
        }
    }

    #[test]
    fn test_stateful_resampler_consistent_chunk_size() {
        let chunk_size = 1000;
        let mut resampler = StatefulResampler::new(48000, 44100, 2, chunk_size).unwrap();

        // Process multiple chunks of consistent size
        // Note: FastFixedIn requires consistent chunk sizes
        for _ in 0..5 {
            let input = vec![0.5; chunk_size * 2]; // Stereo
            let output = resampler.process_chunk(&input).unwrap();

            // Should process successfully
            assert!(!output.is_empty());
        }
    }
}
