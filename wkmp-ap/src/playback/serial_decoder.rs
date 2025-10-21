//! Serial Decoder
//!
//! Single-threaded decoder with priority-based scheduling and decode-and-skip optimization.
//!
//! **Traceability:**
//! - [DBD-DEC-040] Serial decode execution (one stream at a time)
//! - [DBD-DEC-050] Priority queue: Current > Next > Future
//! - [DBD-DEC-060] Decode-and-skip using codec seek tables
//! - [DBD-DEC-070] Yield control for higher-priority passages
//! - [DBD-DEC-080] Sample-accurate positioning using tick-based timing
//! - [DBD-FADE-030] Pre-buffer fade-in application
//! - [DBD-FADE-050] Pre-buffer fade-out application

use crate::audio::decoder::SimpleDecoder;
use crate::audio::resampler::Resampler;
use crate::db::passages::PassageWithTiming;
use crate::error::{Error, Result};
use crate::playback::buffer_manager::BufferManager;
use crate::playback::types::DecodePriority;
use std::collections::BinaryHeap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Instant;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Standard output sample rate (44.1kHz)
/// [DBD-PARAM-020] working_sample_rate = 44,100 Hz
const STANDARD_SAMPLE_RATE: u32 = 44100;

/// Decode chunk size for yield points
/// [DBD-PARAM-060] decode_chunk_size = 8,192 samples
const DECODE_CHUNK_SIZE: usize = 8192;

/// Decode request with priority
#[derive(Debug, Clone)]
pub struct DecodeRequest {
    /// Queue entry identifier (passage buffer key)
    pub queue_entry_id: Uuid,

    /// Passage identifier (for logging)
    pub passage_id: Option<Uuid>,

    /// Passage timing information
    pub passage: PassageWithTiming,

    /// Request priority (lower value = higher priority)
    pub priority: DecodePriority,

    /// True = full decode, False = partial (15 seconds)
    pub full_decode: bool,
}

/// Priority ordering for BinaryHeap (lower priority value = higher priority)
impl Ord for DecodeRequest {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Reverse ordering so lower priority values come first
        other.priority.cmp(&self.priority)
    }
}

impl PartialOrd for DecodeRequest {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for DecodeRequest {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority
    }
}

impl Eq for DecodeRequest {}

/// Shared state for serial decoder
struct SharedDecoderState {
    /// Priority queue of decode requests
    queue: Mutex<BinaryHeap<DecodeRequest>>,

    /// Condition variable for notifying worker
    condvar: Condvar,

    /// Stop flag for shutdown
    stop_flag: AtomicBool,
}

/// Serial audio decoder
///
/// [DBD-DEC-040] Decodes one stream at a time with priority-based scheduling.
/// [DBD-DEC-050] Priority order: Immediate (current) > Next > Prefetch (future).
pub struct SerialDecoder {
    /// Shared state between main thread and worker
    state: Arc<SharedDecoderState>,

    /// Worker thread handle
    thread: Option<JoinHandle<()>>,

    /// Buffer manager for storing decoded buffers
    buffer_manager: Arc<BufferManager>,
}

impl SerialDecoder {
    /// Create serial decoder with single worker thread
    ///
    /// [DBD-DEC-040] Fixed 1-thread pool for serial execution.
    pub fn new(buffer_manager: Arc<BufferManager>) -> Self {
        let state = Arc::new(SharedDecoderState {
            queue: Mutex::new(BinaryHeap::new()),
            condvar: Condvar::new(),
            stop_flag: AtomicBool::new(false),
        });

        // Capture Tokio runtime handle before spawning std::thread
        // Worker needs this to call async buffer_manager methods
        let rt_handle = tokio::runtime::Handle::current();

        // Spawn single worker thread
        let state_clone = Arc::clone(&state);
        let buffer_manager_clone = Arc::clone(&buffer_manager);

        let handle = thread::spawn(move || {
            Self::worker_loop(state_clone, buffer_manager_clone, rt_handle);
        });

        info!("Serial decoder started with 1 worker thread");

        Self {
            state,
            thread: Some(handle),
            buffer_manager,
        }
    }

    /// Submit decode request
    ///
    /// [DBD-DEC-050] Inserts request into priority queue.
    /// **Fix for queue flooding:** Registers buffer immediately to prevent duplicate submissions.
    pub async fn submit(
        &self,
        queue_entry_id: Uuid,
        passage: PassageWithTiming,
        priority: DecodePriority,
        full_decode: bool,
    ) -> Result<()> {
        if self.state.stop_flag.load(Ordering::Relaxed) {
            return Err(Error::Playback("Serial decoder is shutting down".to_string()));
        }

        // **FIX: Register buffer BEFORE queuing to prevent duplicate submissions**
        // This makes is_managed() return true immediately, preventing race condition
        // where engine submits duplicates before worker registers buffer.
        self.buffer_manager.register_decoding(queue_entry_id).await;

        let request = DecodeRequest {
            queue_entry_id,
            passage_id: passage.passage_id,
            passage: passage.clone(),
            priority,
            full_decode,
        };

        // Extract filename for human-readable logging
        let filename = passage.file_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("<unknown>");

        debug!(
            "Submitting decode request: {} (queue_entry_id={}, priority={:?}, full={})",
            filename, queue_entry_id, priority, full_decode
        );

        // Add to priority queue
        {
            let mut queue = self.state.queue.lock().unwrap();
            queue.push(request);
        }

        // Notify worker
        self.state.condvar.notify_one();

        Ok(())
    }

    /// Worker thread main loop
    ///
    /// [DBD-DEC-040] Serial decode execution - processes one request at a time.
    /// [DBD-DEC-070] Yields every DECODE_CHUNK_SIZE samples to check for higher priority.
    fn worker_loop(
        state: Arc<SharedDecoderState>,
        buffer_manager: Arc<BufferManager>,
        rt_handle: tokio::runtime::Handle,
    ) {
        debug!("Serial decoder worker started");

        loop {
            // Get next request from queue
            let request = {
                let mut queue = state.queue.lock().unwrap();

                // Wait for work or shutdown signal
                while queue.is_empty() && !state.stop_flag.load(Ordering::Relaxed) {
                    queue = state.condvar.wait(queue).unwrap();
                }

                // Check if we should exit
                if state.stop_flag.load(Ordering::Relaxed) {
                    debug!("Serial decoder worker received shutdown signal");
                    break;
                }

                // Pop highest priority request
                queue.pop()
            };

            if let Some(request) = request {
                // Extract filename for human-readable logging
                let filename = request.passage.file_path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("<unknown>");

                debug!(
                    "Serial decoder processing: {} (queue_entry_id={}, priority={:?})",
                    filename, request.queue_entry_id, request.priority
                );

                let decode_start = Instant::now();

                // Perform decode with incremental buffer appending
                match Self::decode_passage_serial(&request, &buffer_manager, &rt_handle, &state) {
                    Ok(()) => {
                        let decode_elapsed = decode_start.elapsed();

                        // **[PCF-DUR-010][PCF-COMP-010]** Finalize buffer (cache duration + total_frames)
                        // Get total samples from buffer
                        let total_samples = rt_handle.block_on(async {
                            if let Some(buffer_arc) = buffer_manager.get_buffer(request.queue_entry_id).await {
                                let buffer = buffer_arc.lock().await;
                                buffer.occupied()
                            } else {
                                0
                            }
                        });

                        rt_handle.block_on(async {
                            if let Err(e) = buffer_manager.finalize_buffer(request.queue_entry_id, total_samples).await {
                                warn!("Failed to finalize buffer: {}", e);
                            }
                            // mark_ready is now no-op (handled by state machine)
                            buffer_manager.mark_ready(request.queue_entry_id).await;
                        });

                        info!(
                            "Serial decoder completed: {} (queue_entry_id={}, elapsed={:.2}s)",
                            filename, request.queue_entry_id, decode_elapsed.as_secs_f64()
                        );
                    }
                    Err(e) => {
                        error!(
                            "Serial decoder failed for {}: {} (queue_entry_id={})",
                            filename, e, request.queue_entry_id
                        );

                        // Remove from buffer manager on failure
                        rt_handle.block_on(async {
                            buffer_manager.remove(request.queue_entry_id).await;
                        });
                    }
                }
            }
        }

        debug!("Serial decoder worker exiting");
    }

    /// Decode passage with serial execution and priority yields
    ///
    /// [DBD-DEC-060] Decode-and-skip: Uses timing module to convert ticks → milliseconds for decoder
    /// [DBD-DEC-070] Yields every DECODE_CHUNK_SIZE samples to check priority queue
    /// [DBD-DEC-080] Sample-accurate positioning using tick-based timing
    /// [DBD-FADE-030] Pre-buffer fade-in application
    /// [DBD-FADE-050] Pre-buffer fade-out application
    fn decode_passage_serial(
        request: &DecodeRequest,
        buffer_manager: &Arc<BufferManager>,
        rt_handle: &tokio::runtime::Handle,
        state: &Arc<SharedDecoderState>,
    ) -> Result<()> {
        let passage = &request.passage;
        let queue_entry_id = request.queue_entry_id;

        // Calculate start and end times in milliseconds
        // Convert ticks to milliseconds for decoder
        let start_time_ms = wkmp_common::timing::ticks_to_ms(passage.start_time_ticks) as u64;
        let end_time_ms = if request.full_decode {
            // Full decode: decode to passage end (or file end if None)
            passage.end_time_ticks
                .map(|t| wkmp_common::timing::ticks_to_ms(t) as u64)
                .unwrap_or(0) // 0 = file end in decoder
        } else {
            // Partial decode: first 15 seconds
            start_time_ms + 15_000 // 15 seconds in milliseconds
        };

        // Extract filename for logging
        let filename = passage.file_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("<unknown>");

        debug!(
            "Decoding {}: start={}ms, end={}ms, full={}",
            filename, start_time_ms, end_time_ms, request.full_decode
        );

        // **[DBD-DEC-060]** Decode-and-skip: Decoder uses internal seek tables
        // **[DBD-DEC-090]** Endpoint discovery: When end_ms=0, decoder returns actual_end_ticks
        let decode_start = Instant::now();
        let decode_result = SimpleDecoder::decode_passage(&passage.file_path, start_time_ms, end_time_ms)?;

        let decode_elapsed = decode_start.elapsed();
        debug!(
            "Raw decode completed in {:.2}ms: {} samples @ {}Hz {}ch",
            decode_elapsed.as_millis(), decode_result.samples.len(), decode_result.sample_rate, decode_result.channels
        );

        // **[DBD-DEC-095]** Notify buffer manager of discovered endpoint
        if let Some(actual_end_ticks) = decode_result.actual_end_ticks {
            debug!(
                "Endpoint discovered for {}: {}ticks ({}ms)",
                queue_entry_id,
                actual_end_ticks,
                wkmp_common::timing::ticks_to_ms(actual_end_ticks)
            );

            rt_handle.block_on(async {
                if let Err(e) = buffer_manager.set_discovered_endpoint(queue_entry_id, actual_end_ticks).await {
                    warn!("Failed to set discovered endpoint: {}", e);
                }
            });
        }

        // Update progress periodically during decode
        rt_handle.block_on(async {
            buffer_manager.update_decode_progress(queue_entry_id, 40).await;
        });

        // Resample to standard rate if needed
        // [DBD-PARAM-020] Resample to 44.1kHz using rubato
        let final_samples = if decode_result.sample_rate != STANDARD_SAMPLE_RATE {
            debug!(
                "Resampling from {} Hz to {} Hz",
                decode_result.sample_rate, STANDARD_SAMPLE_RATE
            );

            let resampled = Resampler::resample(&decode_result.samples, decode_result.sample_rate, decode_result.channels)?;

            rt_handle.block_on(async {
                buffer_manager.update_decode_progress(queue_entry_id, 60).await;
            });

            resampled
        } else {
            decode_result.samples
        };

        // Convert to stereo if mono
        let stereo_samples = if decode_result.channels == 1 {
            // Duplicate mono to both channels
            let mut stereo = Vec::with_capacity(final_samples.len() * 2);
            for sample in final_samples {
                stereo.push(sample);
                stereo.push(sample);
            }
            stereo
        } else if decode_result.channels == 2 {
            final_samples
        } else {
            // Downmix multi-channel to stereo (simple average)
            warn!(
                "Downmixing {} channels to stereo (simple average)",
                decode_result.channels
            );
            let frame_count = final_samples.len() / decode_result.channels as usize;
            let mut stereo = Vec::with_capacity(frame_count * 2);

            for frame_idx in 0..frame_count {
                let base = frame_idx * decode_result.channels as usize;
                let mut left = 0.0;
                let mut right = 0.0;

                // Average left channels (odd indices)
                for ch in (0..decode_result.channels as usize).step_by(2) {
                    left += final_samples[base + ch];
                }
                left /= (decode_result.channels / 2) as f32;

                // Average right channels (even indices)
                for ch in (1..decode_result.channels as usize).step_by(2) {
                    right += final_samples[base + ch];
                }
                right /= (decode_result.channels / 2) as f32;

                stereo.push(left);
                stereo.push(right);
            }

            stereo
        };

        rt_handle.block_on(async {
            buffer_manager.update_decode_progress(queue_entry_id, 70).await;
        });

        // **[DBD-FADE-030]** Apply fade-in curve to samples (pre-buffer)
        // **[DBD-FADE-050]** Apply fade-out curve to samples (pre-buffer)
        // **[DBD-FADE-065]** Pass discovered endpoint for fade-out calculation
        let faded_samples = Self::apply_fades_to_samples(
            stereo_samples,
            passage,
            STANDARD_SAMPLE_RATE,
            decode_result.actual_end_ticks,
        );

        rt_handle.block_on(async {
            buffer_manager.update_decode_progress(queue_entry_id, 80).await;
        });

        // Append samples in chunks to enable partial buffer playback
        // [DBD-PARAM-060] Chunk size: 8,192 samples per chunk
        // [DBD-DEC-070] Yield every chunk to check priority queue
        let total_samples = faded_samples.len();
        let total_chunks = (total_samples + DECODE_CHUNK_SIZE - 1) / DECODE_CHUNK_SIZE;

        debug!(
            "Appending {} samples in {} chunks (chunk_size={})",
            total_samples, total_chunks, DECODE_CHUNK_SIZE
        );

        for chunk_idx in 0..total_chunks {
            // **[DBD-DEC-070]** Check for higher-priority requests before each chunk
            if Self::should_yield_to_higher_priority(state, request.priority) {
                warn!(
                    "Serial decoder yielding: higher priority request available (current={:?})",
                    request.priority
                );

                // Re-queue this request and return
                let mut queue = state.queue.lock().unwrap();
                queue.push(request.clone());
                return Ok(());
            }

            let start = chunk_idx * DECODE_CHUNK_SIZE;
            let end = (start + DECODE_CHUNK_SIZE).min(total_samples);
            let chunk = faded_samples[start..end].to_vec();

            // Append chunk to buffer using BufferManager API
            // This automatically triggers state transitions and ReadyForStart events
            rt_handle.block_on(async {
                match buffer_manager.push_samples(queue_entry_id, &chunk).await {
                    Ok(frames_pushed) => {
                        if frames_pushed < chunk.len() / 2 {
                            warn!(
                                "Partial chunk write: {} of {} frames pushed (buffer full)",
                                frames_pushed, chunk.len() / 2
                            );
                        }
                    }
                    Err(e) => {
                        warn!("Failed to push samples: {}", e);
                    }
                }
            });

            // Update progress
            let progress = ((chunk_idx + 1) * 100 / total_chunks).min(100) as u8;
            if progress % 10 == 0 || progress == 100 {
                // Update every 10%
                rt_handle.block_on(async {
                    buffer_manager.update_decode_progress(queue_entry_id, progress).await;
                });
            }

            if (chunk_idx + 1) % 10 == 0 || chunk_idx == total_chunks - 1 {
                debug!(
                    "Appended chunk {}/{} ({:.1}%)",
                    chunk_idx + 1,
                    total_chunks,
                    progress as f32
                );
            }
        }

        Ok(())
    }

    /// Check if decoder should yield to a higher-priority request
    ///
    /// [DBD-DEC-070] Yield control for higher-priority passages
    fn should_yield_to_higher_priority(state: &Arc<SharedDecoderState>, current_priority: DecodePriority) -> bool {
        let queue = state.queue.lock().unwrap();

        if queue.is_empty() {
            return false;
        }

        // Peek at highest priority request in queue
        if let Some(next_request) = queue.peek() {
            // Yield if next request has higher priority (lower value)
            next_request.priority < current_priority
        } else {
            false
        }
    }

    /// Apply fade-in and fade-out curves to samples (pre-buffer)
    ///
    /// [DBD-FADE-030] Pre-buffer fade-in application using passage timing
    /// [DBD-FADE-050] Pre-buffer fade-out application using passage timing
    /// [DBD-FADE-065] Use discovered endpoint for fade-out when endpoint is undefined
    /// [DBD-DEC-080] Sample-accurate fade timing
    fn apply_fades_to_samples(
        mut samples: Vec<f32>,
        passage: &PassageWithTiming,
        sample_rate: u32,
        discovered_end_ticks: Option<i64>,
    ) -> Vec<f32> {
        let frame_count = samples.len() / 2; // Stereo: 2 samples per frame

        // Calculate fade regions in samples
        // Convert ticks to samples using timing module
        // Samples are relative to passage start (position 0 = passage start_time)
        let fade_in_duration_ticks = passage.fade_in_point_ticks.saturating_sub(passage.start_time_ticks);
        let fade_in_duration_samples = wkmp_common::timing::ticks_to_samples(fade_in_duration_ticks, sample_rate);

        // **[DBD-FADE-065]** Use discovered endpoint when passage endpoint is undefined
        // This fixes the issue where fade-out was incorrectly applied at 7 seconds
        // instead of near the actual file end for undefined endpoint passages.
        let passage_end_ticks = if let Some(defined_end) = passage.end_time_ticks {
            // Use user-defined endpoint
            debug!(
                "Using defined endpoint: {}ticks ({}ms)",
                defined_end,
                wkmp_common::timing::ticks_to_ms(defined_end)
            );
            defined_end
        } else if let Some(discovered_end) = discovered_end_ticks {
            // Use discovered endpoint from decoder
            debug!(
                "Using discovered endpoint for fades: {}ticks ({}ms)",
                discovered_end,
                wkmp_common::timing::ticks_to_ms(discovered_end)
            );
            discovered_end
        } else {
            // Fallback: should never happen in practice because decoder always discovers endpoint
            warn!("No defined or discovered endpoint available, using 10-second fallback");
            passage.start_time_ticks + wkmp_common::timing::ms_to_ticks(10000)
        };

        let passage_duration_ticks = passage_end_ticks.saturating_sub(passage.start_time_ticks);
        let passage_duration_samples = wkmp_common::timing::ticks_to_samples(passage_duration_ticks, sample_rate);

        let fade_out_point_ticks = passage.fade_out_point_ticks.unwrap_or(passage_end_ticks);
        let fade_out_start_ticks = fade_out_point_ticks.saturating_sub(passage.start_time_ticks);
        let fade_out_start_samples = wkmp_common::timing::ticks_to_samples(fade_out_start_ticks, sample_rate);
        let fade_out_duration_samples = passage_duration_samples.saturating_sub(fade_out_start_samples);

        debug!(
            "Applying fades: fade_in={}samples ({}ms), fade_out_start={}samples ({}ms), fade_out_duration={}samples ({}ms), total_frames={}, passage_duration={}ms",
            fade_in_duration_samples,
            wkmp_common::timing::ticks_to_ms(fade_in_duration_ticks),
            fade_out_start_samples,
            wkmp_common::timing::ticks_to_ms(fade_out_start_ticks),
            fade_out_duration_samples,
            wkmp_common::timing::ticks_to_ms(passage_duration_ticks.saturating_sub(fade_out_start_ticks)),
            frame_count,
            wkmp_common::timing::ticks_to_ms(passage_duration_ticks)
        );

        // Apply fades frame by frame
        for frame_idx in 0..frame_count {
            let mut fade_multiplier = 1.0;

            // **[DBD-FADE-030]** Fade-in
            if frame_idx < fade_in_duration_samples {
                let fade_in_progress = frame_idx as f32 / fade_in_duration_samples as f32;
                fade_multiplier *= passage.fade_in_curve.calculate_fade_in(fade_in_progress);
            }

            // **[DBD-FADE-050]** Fade-out
            if frame_idx >= fade_out_start_samples {
                let fade_out_position = frame_idx.saturating_sub(fade_out_start_samples);
                let fade_out_progress = if fade_out_duration_samples > 0 {
                    fade_out_position as f32 / fade_out_duration_samples as f32
                } else {
                    1.0
                };
                fade_multiplier *= passage.fade_out_curve.calculate_fade_out(fade_out_progress);
            }

            // Apply multiplier to stereo samples
            let sample_idx = frame_idx * 2;
            if sample_idx + 1 < samples.len() {
                samples[sample_idx] *= fade_multiplier;     // Left
                samples[sample_idx + 1] *= fade_multiplier; // Right
            }
        }

        samples
    }

    /// Shutdown serial decoder
    ///
    /// [DBD-DEC-033] Signal stop, wait for thread with 5-second timeout.
    pub fn shutdown(mut self) -> Result<()> {
        info!("Shutting down serial decoder");

        // Set stop flag
        self.state.stop_flag.store(true, Ordering::Relaxed);

        // Notify worker
        self.state.condvar.notify_one();

        // Join thread with timeout
        if let Some(handle) = self.thread.take() {
            match handle.join() {
                Ok(_) => {
                    debug!("Serial decoder worker joined successfully");
                }
                Err(e) => {
                    error!("Serial decoder worker join failed: {:?}", e);
                }
            }
        }

        info!("Serial decoder shut down");
        Ok(())
    }

    /// Get queue length (for diagnostics)
    pub fn queue_len(&self) -> usize {
        self.state.queue.lock().unwrap().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wkmp_common::FadeCurve;

    // Note: Full integration tests with real audio files are in tests/
    // These are just unit tests for the structure

    #[test]
    fn test_decode_request_priority_ordering() {
        use wkmp_common::timing::ms_to_ticks;

        let mut heap = BinaryHeap::new();

        let passage = PassageWithTiming {
            passage_id: Some(Uuid::new_v4()),
            file_path: std::path::PathBuf::from("/test/audio.mp3"),
            start_time_ticks: 0,
            end_time_ticks: Some(ms_to_ticks(10000)),
            lead_in_point_ticks: 0,
            lead_out_point_ticks: Some(ms_to_ticks(9000)),
            fade_in_point_ticks: 0,
            fade_out_point_ticks: Some(ms_to_ticks(9000)),
            fade_in_curve: FadeCurve::Exponential,
            fade_out_curve: FadeCurve::Logarithmic,
        };

        let req_immediate = DecodeRequest {
            queue_entry_id: Uuid::new_v4(),
            passage_id: Some(Uuid::new_v4()),
            passage: passage.clone(),
            priority: DecodePriority::Immediate,
            full_decode: true,
        };

        let req_next = DecodeRequest {
            queue_entry_id: Uuid::new_v4(),
            passage_id: Some(Uuid::new_v4()),
            passage: passage.clone(),
            priority: DecodePriority::Next,
            full_decode: true,
        };

        let req_prefetch = DecodeRequest {
            queue_entry_id: Uuid::new_v4(),
            passage_id: Some(Uuid::new_v4()),
            passage: passage.clone(),
            priority: DecodePriority::Prefetch,
            full_decode: false,
        };

        // Add in random order
        heap.push(req_prefetch.clone());
        heap.push(req_immediate.clone());
        heap.push(req_next.clone());

        // Pop should give highest priority first
        assert_eq!(heap.pop().unwrap().priority, DecodePriority::Immediate);
        assert_eq!(heap.pop().unwrap().priority, DecodePriority::Next);
        assert_eq!(heap.pop().unwrap().priority, DecodePriority::Prefetch);
    }

    #[test]
    fn test_fade_calculations() {
        use wkmp_common::timing::ms_to_ticks;

        // Verify fade calculations work correctly
        let passage = PassageWithTiming {
            passage_id: Some(Uuid::new_v4()),
            file_path: std::path::PathBuf::from("/test/audio.mp3"),
            start_time_ticks: 0,
            end_time_ticks: Some(ms_to_ticks(10000)),  // 10 seconds
            lead_in_point_ticks: 0,
            lead_out_point_ticks: Some(ms_to_ticks(9000)),
            fade_in_point_ticks: ms_to_ticks(2000),  // 2 second fade-in
            fade_out_point_ticks: Some(ms_to_ticks(8000)),  // Fade out starts at 8s
            fade_in_curve: FadeCurve::Linear,
            fade_out_curve: FadeCurve::Linear,
        };

        // Create dummy samples (10 seconds @ 44.1kHz stereo = 882,000 samples)
        let dummy_samples = vec![0.5_f32; 882_000];

        // Apply fades (no discovered endpoint needed since passage has defined endpoint)
        let faded = SerialDecoder::apply_fades_to_samples(dummy_samples, &passage, 44100, None);

        // Verify first sample is near zero (fade-in start)
        assert!(faded[0].abs() < 0.05, "First sample should be faded in");

        // Verify sample at 1 second is partially faded in
        let one_second_frame = 44100;
        let one_second_sample = one_second_frame * 2;
        assert!(faded[one_second_sample] > 0.2 && faded[one_second_sample] < 0.8,
            "Sample at 1s should be mid-fade");

        // Verify sample at 2 seconds is fully faded in
        let two_second_frame = 88200;
        let two_second_sample = two_second_frame * 2;
        assert!(faded[two_second_sample] > 0.45 && faded[two_second_sample] < 0.55,
            "Sample at 2s should be near full volume");
    }

    #[test]
    fn test_fade_with_discovered_endpoint() {
        use wkmp_common::timing::ms_to_ticks;

        // **[DBD-FADE-065]** Test that fade-out uses discovered endpoint for undefined passages
        // This is the critical fix: passages with undefined endpoints should use the discovered
        // endpoint (actual file duration) instead of the 10-second fallback.

        let passage = PassageWithTiming {
            passage_id: Some(Uuid::new_v4()),
            file_path: std::path::PathBuf::from("/test/audio.mp3"),
            start_time_ticks: 0,
            end_time_ticks: None,  // Undefined endpoint
            lead_in_point_ticks: 0,
            lead_out_point_ticks: None,  // Will use endpoint - 3s
            fade_in_point_ticks: ms_to_ticks(2000),  // 2 second fade-in
            fade_out_point_ticks: None,  // Will use lead-out (endpoint - 3s)
            fade_in_curve: FadeCurve::Linear,
            fade_out_curve: FadeCurve::Linear,
        };

        // Create dummy samples for a 30-second file @ 44.1kHz stereo = 2,646,000 samples
        let dummy_samples = vec![0.5_f32; 2_646_000];

        // Discovered endpoint: 30 seconds (simulating what decoder would discover)
        let discovered_end = ms_to_ticks(30000);

        // Apply fades with discovered endpoint
        let faded = SerialDecoder::apply_fades_to_samples(dummy_samples, &passage, 44100, Some(discovered_end));

        // Verify first sample is near zero (fade-in start)
        assert!(faded[0].abs() < 0.05, "First sample should be faded in");

        // Verify sample at 1 second is partially faded in
        let one_second_frame = 44100;
        let one_second_sample = one_second_frame * 2;
        assert!(faded[one_second_sample] > 0.2 && faded[one_second_sample] < 0.8,
            "Sample at 1s should be mid-fade");

        // Verify sample at 2 seconds is fully faded in
        let two_second_frame = 88200;
        let two_second_sample = two_second_frame * 2;
        assert!(faded[two_second_sample] > 0.45 && faded[two_second_sample] < 0.55,
            "Sample at 2s should be near full volume");

        // Verify sample at 15 seconds is still at full volume (not faded out yet)
        // This is the key test: with the old code, fade-out would start at ~7s (10s - 3s)
        // With the fix, fade-out should start at ~27s (30s - 3s)
        let fifteen_second_frame = 661500;  // 15s * 44100
        let fifteen_second_sample = fifteen_second_frame * 2;
        assert!(faded[fifteen_second_sample] > 0.45 && faded[fifteen_second_sample] < 0.55,
            "Sample at 15s should be at full volume (not faded out)");

        // Verify sample at 27 seconds is starting to fade out
        // Lead-out is at 27s (30s - 3s default), fade-out should be in progress
        let twentyseven_second_frame = 1_190_700;  // 27s * 44100
        let twentyseven_second_sample = twentyseven_second_frame * 2;
        assert!(faded[twentyseven_second_sample] > 0.2 && faded[twentyseven_second_sample] < 0.8,
            "Sample at 27s should be fading out");
    }
}
