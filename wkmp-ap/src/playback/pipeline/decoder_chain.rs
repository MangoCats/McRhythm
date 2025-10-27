//! Decoder chain - integrates decode → resample → fade → buffer pipeline
//!
//! Provides a unified interface for the single-threaded decoder worker architecture.
//! Each `DecoderChain` encapsulates the complete pipeline for one passage:
//! 1. Decode: StreamingDecoder produces PCM chunks
//! 2. Resample: StatefulResampler converts to 44.1kHz
//! 3. Fade: Fader applies fade-in/out curves
//! 4. Buffer: Push to PlayoutRingBuffer via BufferManager
//!
//! **Architecture:** Used by the new single-threaded worker loop in engine.rs.
//! Replaces the old multi-threaded SerialDecoder/DecoderPool architecture.
//!
//! **Traceability:**
//! - [DBD-DEC-040] Serial decoding (one decoder at a time)
//! - [DBD-DEC-090] Streaming/incremental operation
//! - [DBD-DEC-110] ~1 second chunk processing
//! - [DBD-DEC-130] State preservation for pause/resume
//! - [DBD-FADE-030] Pre-buffer fade-in
//! - [DBD-FADE-050] Pre-buffer fade-out

use crate::audio::decoder::StreamingDecoder;
use crate::audio::resampler::StatefulResampler;
use crate::db::passages::PassageWithTiming;
use crate::error::{Error, Result};
use crate::playback::buffer_manager::BufferManager;
use crate::playback::pipeline::Fader;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Result of processing one chunk through the decoder chain
#[derive(Debug, Clone, PartialEq)]
pub enum ChunkProcessResult {
    /// Chunk processed successfully and pushed to buffer
    Processed {
        /// Number of frames (stereo sample pairs) pushed to buffer
        frames_pushed: usize,
    },

    /// Buffer is full, cannot push more data right now
    /// Chain should yield to allow mixer to drain buffer
    BufferFull {
        /// Number of frames that were pushed (may be partial)
        frames_pushed: usize,
    },

    /// Decoding is complete, no more chunks to process
    Finished {
        /// Total frames pushed for this passage
        total_frames: usize,
    },
}

/// Decoder chain for a single passage
///
/// Encapsulates the complete decode → resample → fade → buffer pipeline.
/// Maintains state across chunk processing for seamless streaming.
///
/// **[DBD-DEC-130]** State preservation enables pause/resume between chunks.
pub struct DecoderChain {
    // Identification
    queue_entry_id: Uuid,
    passage_id: Option<Uuid>, // **[Phase 7]** For error event emission
    chain_index: usize,

    // Pipeline components
    decoder: StreamingDecoder,
    resampler: StatefulResampler,
    fader: Fader,

    // State tracking
    chunk_count: u64,
    total_frames_pushed: usize,

    // Configuration
    chunk_duration_ms: u64,

    // For logging
    file_path: PathBuf,

    /// Pending samples from previous chunk that couldn't be pushed due to buffer full
    ///
    /// **[BUGFIX]** Critical fix for audio skipping issue.
    /// When ring buffer is full, we must save decoded samples and retry on next call
    /// instead of discarding them and decoding the next chunk (which causes skipping).
    pending_samples: Option<Vec<f32>>,
}

impl DecoderChain {
    /// Create a new decoder chain for a passage
    ///
    /// # Arguments
    /// * `queue_entry_id` - UUID of the queue entry being decoded
    /// * `chain_index` - Chain index (0-11) for logging
    /// * `passage` - Passage timing information
    /// * `buffer_manager` - Buffer manager (used to check if buffer exists)
    ///
    /// # Returns
    /// Initialized decoder chain ready to process chunks
    ///
    /// # Errors
    /// Returns error if decoder creation fails or passage is invalid
    pub async fn new(
        queue_entry_id: Uuid,
        chain_index: usize,
        passage: &PassageWithTiming,
        buffer_manager: &BufferManager,
    ) -> Result<Self> {
        info!(
            "[Chain {}] Creating decoder chain for passage: {}",
            chain_index,
            passage.file_path.display()
        );

        // Verify buffer exists
        if buffer_manager.get_buffer(queue_entry_id).await.is_none() {
            return Err(Error::Decode(format!(
                "Buffer not found for queue entry: {}",
                queue_entry_id
            )));
        }

        // **[DBD-DEC-110]** Use 1 second chunks
        let chunk_duration_ms = 1000;

        // Convert passage timing to milliseconds for decoder
        let start_ms = wkmp_common::timing::ticks_to_ms(passage.start_time_ticks) as u64;
        let end_ms = passage
            .end_time_ticks
            .map(|t| wkmp_common::timing::ticks_to_ms(t) as u64)
            .unwrap_or(u64::MAX); // Undefined endpoint = decode to file end

        // Create streaming decoder
        let decoder = StreamingDecoder::new(&passage.file_path, start_ms, end_ms)?;
        let (source_sample_rate, source_channels) = decoder.format_info();

        debug!(
            "[Chain {}] Decoder format: {}Hz, {} channels",
            chain_index, source_sample_rate, source_channels
        );

        // Create stateful resampler
        // Chunk size for resampler = samples per chunk at source rate
        let chunk_size_samples = (source_sample_rate as u64 * chunk_duration_ms / 1000) as usize;
        let resampler = StatefulResampler::new(
            source_sample_rate,
            44100, // TARGET_SAMPLE_RATE
            source_channels,
            chunk_size_samples,
        )?;

        // Create fader
        // Note: discovered_end_ticks will be None initially, Fader handles this
        let fader = Fader::new(passage, source_sample_rate, None);

        debug!(
            "[Chain {}] Pipeline initialized: decode({}Hz) -> resample(44.1kHz) -> fade -> buffer",
            chain_index, source_sample_rate
        );

        Ok(Self {
            queue_entry_id,
            passage_id: passage.passage_id, // **[Phase 7]** Store for error events
            chain_index,
            decoder,
            resampler,
            fader,
            chunk_count: 0,
            total_frames_pushed: 0,
            chunk_duration_ms,
            file_path: passage.file_path.clone(),
            pending_samples: None, // **[BUGFIX]** Initialize pending buffer
        })
    }

    /// Process one chunk through the pipeline
    ///
    /// **[DBD-DEC-110]** Decodes ~1 second of audio per call.
    ///
    /// # Process Flow
    /// 1. Decode: Get PCM samples from StreamingDecoder
    /// 2. Resample: Convert to 44.1kHz via StatefulResampler
    /// 3. Fade: Apply fade curves via Fader
    /// 4. Push: Send to buffer via BufferManager
    ///
    /// # Arguments
    /// * `buffer_manager` - Buffer manager for pushing samples
    ///
    /// # Returns
    /// Result indicating success, buffer full, or finished
    ///
    /// # Errors
    /// Returns error if decode, resample, or push fails
    pub async fn process_chunk(
        &mut self,
        buffer_manager: &BufferManager,
    ) -> Result<ChunkProcessResult> {
        // Check if already finished
        if self.decoder.is_finished() && self.pending_samples.is_none() {
            debug!(
                "[Chain {}] Already finished, total frames: {}",
                self.chain_index, self.total_frames_pushed
            );
            return Ok(ChunkProcessResult::Finished {
                total_frames: self.total_frames_pushed,
            });
        }

        // **[BUGFIX]** Step 1: Check for pending samples from previous call
        let faded_samples = if let Some(pending) = self.pending_samples.take() {
            // Retrying push of samples that were decoded but couldn't be pushed last time
            debug!(
                "[Chain {}] Retrying push of {} pending samples (from previous buffer-full)",
                self.chain_index,
                pending.len()
            );
            pending
        } else {
            // No pending samples - decode new chunk
            self.chunk_count += 1;

            // **[DBD-DEC-110] Step 1a:** Decode chunk
            debug!(
                "[Chain {}] Decoding chunk {} ({}ms)",
                self.chain_index, self.chunk_count, self.chunk_duration_ms
            );

            // **[REQ-AP-ERR-013]** Wrap decode_chunk in panic catching
            let decode_result = catch_unwind(AssertUnwindSafe(|| {
                self.decoder.decode_chunk(self.chunk_duration_ms)
            }));

            let chunk_samples = match decode_result {
                Ok(Ok(Some(samples))) => samples,
                Ok(Ok(None)) => {
                    // Decoder finished normally
                    info!(
                        "[Chain {}] Decoding complete after {} chunks, {} frames total",
                        self.chain_index, self.chunk_count, self.total_frames_pushed
                    );

                    // Get discovered endpoint if available
                    if let Some(discovered_end_ticks) = self.decoder.get_discovered_endpoint() {
                        debug!(
                            "[Chain {}] Discovered endpoint: {}ticks ({}ms)",
                            self.chain_index,
                            discovered_end_ticks,
                            wkmp_common::timing::ticks_to_ms(discovered_end_ticks)
                        );
                    }

                    return Ok(ChunkProcessResult::Finished {
                        total_frames: self.total_frames_pushed,
                    });
                }
                Ok(Err(e)) => {
                    // Decode error (already typed errors from StreamingDecoder)
                    return Err(e);
                }
                Err(panic_payload) => {
                    // Decoder panicked - convert to DecoderPanic error
                    let panic_msg = if let Some(s) = panic_payload.downcast_ref::<&str>() {
                        s.to_string()
                    } else if let Some(s) = panic_payload.downcast_ref::<String>() {
                        s.clone()
                    } else {
                        "Unknown panic in decoder".to_string()
                    };

                    error!(
                        "[Chain {}] Decoder panicked: {}",
                        self.chain_index, panic_msg
                    );

                    return Err(Error::DecoderPanic {
                        path: self.file_path.clone(),
                        message: panic_msg,
                    });
                }
            };

            let decoded_frames = chunk_samples.len() / 2; // Stereo
            debug!(
                "[Chain {}] Decoded {} frames",
                self.chain_index, decoded_frames
            );

            // **[DBD-DEC-110] Step 2:** Resample to 44.1kHz
            // **[REQ-AP-ERR-051]** Catch resampling runtime errors and add position context
            let resampled_samples = self.resampler.process_chunk(&chunk_samples).map_err(|e| {
                // Calculate current position in milliseconds for error reporting
                let position_ms = (self.fader.current_frame() as u64 * 1000) / 44100;
                match e {
                    Error::Decode(msg) if msg.contains("Resampling failed") => {
                        Error::ResamplingRuntimeError {
                            position_ms,
                            message: msg,
                        }
                    }
                    other => other, // Other errors pass through unchanged
                }
            })?;
            let resampled_frames = resampled_samples.len() / 2;

            if !self.resampler.is_pass_through() {
                debug!(
                    "[Chain {}] Resampled {} frames -> {} frames",
                    self.chain_index, decoded_frames, resampled_frames
                );
            }

            // **[DBD-FADE-030/050] Step 3:** Apply fades
            let faded = self.fader.process_chunk(resampled_samples);

            if !self.fader.is_pass_through() {
                debug!(
                    "[Chain {}] Applied fades (frame position: {})",
                    self.chain_index,
                    self.fader.current_frame()
                );
            }

            faded
        };

        // **[BUGFIX] Step 2:** Try to push to buffer (with retry support)
        let total_frames_in_chunk = faded_samples.len() / 2;

        let frames_pushed = match buffer_manager
            .push_samples(self.queue_entry_id, &faded_samples)
            .await
        {
            Ok(frames_pushed) => {
                // push_samples returns frames count, not samples
                self.total_frames_pushed += frames_pushed;
                debug!(
                    "[Chain {}] Pushed {} frames to buffer (total: {})",
                    self.chain_index, frames_pushed, self.total_frames_pushed
                );

                // **[REQ-AP-ERR-060]** Position drift detection
                // Only check when we pushed the full chunk (no partial push)
                if frames_pushed == total_frames_in_chunk {
                    let expected_position = self.fader.current_frame();
                    let actual_position = self.total_frames_pushed;
                    let drift = expected_position.abs_diff(actual_position);

                    if drift > 0 {
                        if drift < 100 {
                            // Minor drift (< 3ms @ 44.1kHz) - log only
                            debug!(
                                "[Chain {}] Minor position drift: expected={}, actual={}, delta={} frames",
                                self.chain_index, expected_position, actual_position, drift
                            );
                        } else if drift >= 44100 {
                            // Severe drift (> 1 second) - critical error
                            error!(
                                "[Chain {}] SEVERE position drift: expected={}, actual={}, delta={} frames ({}ms) - position corrupted",
                                self.chain_index, expected_position, actual_position, drift, (drift * 1000) / 44100
                            );
                            // Return error to trigger passage skip
                            return Err(Error::Decode(format!(
                                "Position corrupted: drift {} frames ({}ms)",
                                drift, (drift * 1000) / 44100
                            )));
                        } else {
                            // Moderate drift (>= 100 samples, < 1 second) - emit warning
                            let drift_ms = (drift * 1000) as u64 / 44100;
                            warn!(
                                "[Chain {}] Position drift detected: expected={}, actual={}, delta={} frames ({}ms)",
                                self.chain_index, expected_position, actual_position, drift, drift_ms
                            );
                            // Return error for decoder_worker to handle and emit event
                            return Err(Error::PositionDrift {
                                expected_frames: expected_position,
                                actual_frames: actual_position,
                                drift_frames: drift,
                                drift_ms,
                            });
                        }
                    }
                }

                frames_pushed
            }
            Err(e) if e.contains("BufferFullError") => {
                // **[BUGFIX]** Buffer is completely full - save samples for retry
                debug!(
                    "[Chain {}] Buffer full at chunk {}, saving {} samples for retry",
                    self.chain_index, self.chunk_count, faded_samples.len()
                );
                self.pending_samples = Some(faded_samples);
                return Ok(ChunkProcessResult::BufferFull { frames_pushed: 0 });
            }
            Err(e) => {
                return Err(Error::Decode(format!(
                    "Failed to push samples to buffer: {}",
                    e
                )));
            }
        };

        // **[BUGFIX] Step 3:** Check if we pushed partial data (buffer became full mid-chunk)
        if frames_pushed < total_frames_in_chunk {
            debug!(
                "[Chain {}] Partial push: {} of {} frames (buffer filling up)",
                self.chain_index, frames_pushed, total_frames_in_chunk
            );

            // Save remaining samples for next retry
            let samples_pushed = frames_pushed * 2; // Convert frames to samples (stereo)
            self.pending_samples = Some(faded_samples[samples_pushed..].to_vec());

            debug!(
                "[Chain {}] Saved {} remaining samples for next push",
                self.chain_index,
                self.pending_samples.as_ref().unwrap().len()
            );

            return Ok(ChunkProcessResult::BufferFull { frames_pushed });
        }

        Ok(ChunkProcessResult::Processed { frames_pushed })
    }

    /// Check if this decoder chain has finished processing
    pub fn is_finished(&self) -> bool {
        self.decoder.is_finished()
    }

    /// Get the queue entry ID for this chain
    pub fn queue_entry_id(&self) -> Uuid {
        self.queue_entry_id
    }

    /// Get the chain index (0-11)
    pub fn chain_index(&self) -> usize {
        self.chain_index
    }

    /// Get passage ID (may be None for ephemeral passages)
    /// **[Phase 7]** Used for error event emission
    pub fn passage_id(&self) -> Option<Uuid> {
        self.passage_id
    }

    /// Get total frames pushed so far
    pub fn total_frames_pushed(&self) -> usize {
        self.total_frames_pushed
    }

    /// Get current chunk count
    pub fn chunk_count(&self) -> u64 {
        self.chunk_count
    }

    /// Get file path being decoded
    pub fn file_path(&self) -> &PathBuf {
        &self.file_path
    }

    /// Check if this was a partial decode (truncated file)
    ///
    /// **[REQ-AP-ERR-012]** Partial decode detection.
    ///
    /// Should be called after decoding is complete (Finished result returned).
    ///
    /// # Returns
    /// `Some((expected_ms, actual_ms, percentage))` if partial decode detected, `None` otherwise
    pub fn get_partial_decode_info(&self) -> Option<(u64, u64, f64)> {
        self.decoder.get_partial_decode_info()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: DecoderChain requires actual audio files and BufferManager setup,
    // so comprehensive testing is done via integration tests.
    // Unit tests here focus on the ChunkProcessResult enum.

    #[test]
    fn test_chunk_process_result_variants() {
        let processed = ChunkProcessResult::Processed { frames_pushed: 100 };
        assert!(matches!(processed, ChunkProcessResult::Processed { .. }));

        let buffer_full = ChunkProcessResult::BufferFull { frames_pushed: 50 };
        assert!(matches!(buffer_full, ChunkProcessResult::BufferFull { .. }));

        let finished = ChunkProcessResult::Finished { total_frames: 1000 };
        assert!(matches!(finished, ChunkProcessResult::Finished { .. }));
    }

    #[test]
    fn test_chunk_process_result_equality() {
        let r1 = ChunkProcessResult::Processed { frames_pushed: 100 };
        let r2 = ChunkProcessResult::Processed { frames_pushed: 100 };
        assert_eq!(r1, r2);

        let r3 = ChunkProcessResult::Finished { total_frames: 500 };
        let r4 = ChunkProcessResult::Finished { total_frames: 500 };
        assert_eq!(r3, r4);
    }
}
