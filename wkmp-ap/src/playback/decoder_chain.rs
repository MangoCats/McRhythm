//! Decoder-buffer chain pipeline
//!
//! Implements the DecoderChain pipeline per SPEC016 DBD-IMPL-020.
//!
//! # Pipeline
//!
//! Per SPEC016 DBD-IMPL-020:
//! 1. **Decoder**: Chunk-based audio decoding using symphonia
//! 2. **Resampler**: Sample rate conversion to working_sample_rate
//! 3. **Fader**: Sample-accurate fade-in/fade-out application
//! 4. **Buffer**: Ring buffer for decoded samples
//!
//! # State Preservation
//!
//! Per SPEC016 DBD-IMPL-030, maintains state across chunks:
//! - Decoder position and EOF detection
//! - Resampler filter coefficients (prevents phase discontinuities)
//! - Fader frame position (sample-accurate crossfading)
//! - Total frames pushed (for buffer finalization)
//!
//! # Processing Results
//!
//! Per SPEC016 DBD-IMPL-040:
//! - `Processed { frames_pushed }` - Chunk successfully processed
//! - `BufferFull { frames_pushed }` - Yield required (buffer at capacity)
//! - `Finished { total_frames }` - Decoding complete
//!
//! # Architecture
//!
//! Phase 4: Incremental chunk-based decoding with buffer backpressure
//! Integration: Used by DecoderWorker for serial processing

use crate::{
    audio::{AudioDecoder, Resampler},
    playback::fader::{FadeCurve, Fader},
    audio::RingBuffer,
    Result,
};
use std::path::PathBuf;
use uuid::Uuid;

/// Processing result from chunk decode
///
/// Per SPEC016 DBD-IMPL-040
#[derive(Debug)]
pub enum ProcessResult {
    /// Chunk processed successfully
    Processed {
        /// Number of stereo frames pushed to buffer
        frames_pushed: usize,
    },

    /// Buffer full, decoder must yield
    ///
    /// Per SPEC016 DBD-DEC-150.1: Immediate yield on hysteresis
    BufferFull {
        /// Number of stereo frames pushed before buffer full
        frames_pushed: usize,
    },

    /// Decoding complete
    ///
    /// Per SPEC016 DBD-FADE-060: End time reached
    Finished {
        /// Total stereo frames pushed to buffer
        total_frames: usize,
    },
}

/// Decoder-buffer chain pipeline
///
/// Encapsulates the full decoding pipeline: Decoder → Resampler → Fader → Buffer
///
/// # Examples
///
/// ```ignore
/// let mut chain = DecoderChain::new(
///     passage_id,
///     "/path/to/audio.mp3",
///     0, 0, 28_224_000,          // Timing points (ticks)
///     282_240_000, 282_240_000, 310_464_000,
///     FadeCurve::Exponential,
///     FadeCurve::Logarithmic,
///     44100,                      // Working sample rate
///     88200,                      // Buffer capacity (samples)
/// )?;
///
/// loop {
///     match chain.process_chunk()? {
///         ProcessResult::Processed { frames_pushed } => {
///             println!("Processed {} frames", frames_pushed);
///         }
///         ProcessResult::BufferFull { .. } => {
///             println!("Buffer full, yielding");
///             break;
///         }
///         ProcessResult::Finished { total_frames } => {
///             println!("Finished, total {} frames", total_frames);
///             break;
///         }
///     }
/// }
/// ```
pub struct DecoderChain {
    /// Passage UUID
    passage_id: Uuid,

    /// File path
    file_path: PathBuf,

    /// Audio decoder
    decoder: AudioDecoder,

    /// Resampler (stateful across chunks)
    resampler: Resampler,

    /// Fader (stateful position tracking)
    fader: Fader,

    /// Ring buffer
    buffer: RingBuffer,

    /// Total stereo frames pushed to buffer
    total_frames_pushed: usize,

    /// Whether decoder reached EOF
    eof_reached: bool,

    /// Passage end time in ticks
    passage_end_ticks: i64,

    /// Working sample rate (stored for potential diagnostics)
    sample_rate: u32,
}

impl DecoderChain {
    /// Create new decoder chain
    ///
    /// # Arguments
    ///
    /// * `passage_id` - Passage UUID
    /// * `file_path` - Path to audio file
    /// * `passage_start_ticks` - Passage start time
    /// * `fade_in_start_ticks` - Fade-in start
    /// * `lead_in_start_ticks` - Full volume begins
    /// * `lead_out_start_ticks` - Fade-out begins
    /// * `fade_out_start_ticks` - Volume ramp to zero
    /// * `passage_end_ticks` - Passage end time
    /// * `fade_in_curve` - Fade-in curve type
    /// * `fade_out_curve` - Fade-out curve type
    /// * `working_sample_rate` - Target sample rate (typically 44100)
    /// * `buffer_capacity_samples` - Buffer capacity in stereo samples
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let chain = DecoderChain::new(
    ///     passage_uuid,
    ///     "/path/to/audio.mp3",
    ///     0, 0, 28_224_000,
    ///     282_240_000, 282_240_000, 310_464_000,
    ///     FadeCurve::Exponential,
    ///     FadeCurve::Logarithmic,
    ///     44100,
    ///     88200,
    /// )?;
    /// ```
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        passage_id: Uuid,
        file_path: PathBuf,
        passage_start_ticks: i64,
        fade_in_start_ticks: i64,
        lead_in_start_ticks: i64,
        lead_out_start_ticks: i64,
        fade_out_start_ticks: i64,
        passage_end_ticks: i64,
        fade_in_curve: FadeCurve,
        fade_out_curve: FadeCurve,
        working_sample_rate: u32,
        buffer_capacity_samples: usize,
    ) -> Result<Self> {
        // Create decoder
        let decoder = AudioDecoder::new(&file_path)?;

        // Create resampler
        let native_sample_rate = decoder.sample_rate();
        let resampler = Resampler::new(native_sample_rate, working_sample_rate)?;

        // Create fader with timing points
        let fader = Fader::new(
            passage_start_ticks,
            fade_in_start_ticks,
            lead_in_start_ticks,
            lead_out_start_ticks,
            fade_out_start_ticks,
            passage_end_ticks,
            fade_in_curve,
            fade_out_curve,
            working_sample_rate,
        );

        // Create buffer
        let buffer = RingBuffer::new(buffer_capacity_samples);

        Ok(Self {
            passage_id,
            file_path,
            decoder,
            resampler,
            fader,
            buffer,
            total_frames_pushed: 0,
            eof_reached: false,
            passage_end_ticks,
            sample_rate: working_sample_rate,
        })
    }

    /// Process one chunk of audio
    ///
    /// Per SPEC016 DBD-DEC-110: Chunk-based decoding process:
    /// 1. Decode chunk (~1 second)
    /// 2. Resample chunk (if needed)
    /// 3. Apply fades to chunk
    /// 4. Append chunk to buffer
    ///
    /// # Returns
    ///
    /// - `Processed`: Chunk successfully processed
    /// - `BufferFull`: Buffer full, decoder must yield
    /// - `Finished`: Decoding complete
    ///
    /// # Examples
    ///
    /// ```ignore
    /// match chain.process_chunk()? {
    ///     ProcessResult::Processed { frames_pushed } => { /* ... */ }
    ///     ProcessResult::BufferFull { .. } => { /* yield */ }
    ///     ProcessResult::Finished { total_frames } => { /* done */ }
    /// }
    /// ```
    pub fn process_chunk(&mut self) -> Result<ProcessResult> {
        // Check if already finished
        if self.eof_reached {
            return Ok(ProcessResult::Finished {
                total_frames: self.total_frames_pushed,
            });
        }

        // Step 1: Decode chunk
        let chunk = match self.decoder.decode_chunk()? {
            Some(chunk) => chunk,
            None => {
                // EOF reached
                self.eof_reached = true;
                return Ok(ProcessResult::Finished {
                    total_frames: self.total_frames_pushed,
                });
            }
        };

        // Step 2: Resample chunk (if needed)
        let mut samples = self.resampler.resample(&chunk.samples)?;

        // Step 3: Apply fades to chunk
        self.fader.apply_fade(&mut samples)?;

        // Check if we've passed the passage end time
        let fader_position = self.fader.position_ticks();
        if fader_position >= self.passage_end_ticks {
            self.eof_reached = true;
            // Don't push samples beyond end time
            return Ok(ProcessResult::Finished {
                total_frames: self.total_frames_pushed,
            });
        }

        // Step 4: Append chunk to buffer
        let frames_pushed = self.buffer.push(&samples)?;
        self.total_frames_pushed += frames_pushed;

        // Check if buffer is full (backpressure)
        if frames_pushed < samples.len() / 2 {
            // Not all samples pushed, buffer is full
            return Ok(ProcessResult::BufferFull { frames_pushed });
        }

        // Successfully processed chunk
        Ok(ProcessResult::Processed { frames_pushed })
    }

    /// Get passage ID
    pub fn passage_id(&self) -> Uuid {
        self.passage_id
    }

    /// Get file path
    pub fn file_path(&self) -> &PathBuf {
        &self.file_path
    }

    /// Get total frames pushed
    pub fn total_frames_pushed(&self) -> usize {
        self.total_frames_pushed
    }

    /// Check if finished
    pub fn is_finished(&self) -> bool {
        self.eof_reached
    }

    /// Get buffer (for reading samples)
    pub fn buffer(&mut self) -> &mut RingBuffer {
        &mut self.buffer
    }
}

// ========================================
// Tests
// ========================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decoder_chain_nonexistent_file() {
        let result = DecoderChain::new(
            Uuid::new_v4(),
            PathBuf::from("/nonexistent/file.mp3"),
            0,
            0,
            28_224_000,
            282_240_000,
            282_240_000,
            310_464_000,
            FadeCurve::Linear,
            FadeCurve::Linear,
            44100,
            88200,
        );

        assert!(result.is_err());
    }

    // Note: Additional tests would require test audio files
    // See Phase 4 integration tests for comprehensive testing
}
