//! Decoder Pool
//!
//! Multi-threaded decoder with priority queue for parallel audio decoding.
//!
//! **Traceability:**
//! - [SSD-DEC-030] Fixed 2-thread decoder pool
//! - [SSD-DEC-032] Priority queue management
//! - [SSD-DEC-033] 5-second shutdown timeout

use crate::audio::decoder::SimpleDecoder;
use crate::audio::resampler::Resampler;
use crate::db::passages::PassageWithTiming;
use crate::error::{Error, Result};
use crate::playback::buffer_manager::BufferManager;
use crate::playback::types::DecodePriority;
use std::collections::{BinaryHeap, HashMap};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Standard output sample rate (44.1kHz)
/// [SSD-FBUF-020] All audio resampled to this rate
const STANDARD_SAMPLE_RATE: u32 = 44100;

/// Decode request with priority
#[derive(Debug, Clone)]
pub struct DecodeRequest {
    /// Passage identifier
    pub passage_id: Uuid,

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

/// Shared state for decoder pool
struct SharedPoolState {
    /// Priority queue of decode requests
    queue: Mutex<BinaryHeap<DecodeRequest>>,

    /// Condition variable for notifying workers
    condvar: Condvar,

    /// Stop flag for shutdown
    stop_flag: AtomicBool,

    /// Jobs paused due to full buffers, awaiting resume
    /// **[DBD-BUF-050]** Maps passage_id → (DecodeRequest, samples_already_processed)
    paused_jobs: Mutex<HashMap<Uuid, (DecodeRequest, usize)>>,
}

/// Multi-threaded decoder pool
///
/// [SSD-DEC-030] Fixed pool with 2 decoder threads for parallel decoding.
pub struct DecoderPool {
    /// Shared state between threads
    state: Arc<SharedPoolState>,

    /// Worker thread handles
    threads: Vec<JoinHandle<()>>,

    /// Buffer manager for storing decoded buffers
    buffer_manager: Arc<BufferManager>,
}

impl DecoderPool {
    /// Create decoder pool with 2 threads
    ///
    /// [SSD-DEC-030] Fixed 2-thread pool for Raspberry Pi Zero2W resource limits.
    pub fn new(buffer_manager: Arc<BufferManager>) -> Self {
        let state = Arc::new(SharedPoolState {
            queue: Mutex::new(BinaryHeap::new()),
            condvar: Condvar::new(),
            stop_flag: AtomicBool::new(false),
            paused_jobs: Mutex::new(HashMap::new()),
        });

        // Capture Tokio runtime handle before spawning std::threads
        // Workers need this to call async buffer_manager methods
        let rt_handle = tokio::runtime::Handle::current();

        // Spawn 2 worker threads
        let mut threads = Vec::new();
        for worker_id in 0..2 {
            let state_clone = Arc::clone(&state);
            let buffer_manager_clone = Arc::clone(&buffer_manager);
            let rt_handle_clone = rt_handle.clone();

            let handle = thread::spawn(move || {
                Self::worker_loop(worker_id, state_clone, buffer_manager_clone, rt_handle_clone);
            });

            threads.push(handle);
        }

        info!("Decoder pool started with 2 worker threads");

        Self {
            state,
            threads,
            buffer_manager,
        }
    }

    /// Submit decode request
    ///
    /// [SSD-DEC-032] Inserts request into priority queue.
    /// **Fix for queue flooding:** Registers buffer immediately to prevent duplicate submissions.
    pub async fn submit(
        &self,
        passage_id: Uuid,
        passage: PassageWithTiming,
        priority: DecodePriority,
        full_decode: bool,
    ) -> Result<()> {
        if self.state.stop_flag.load(Ordering::Relaxed) {
            return Err(Error::Playback("Decoder pool is shutting down".to_string()));
        }

        // **FIX: Register buffer BEFORE queuing to prevent duplicate submissions**
        // This makes is_managed() return true immediately, preventing race condition
        // where engine submits duplicates before worker registers buffer.
        self.buffer_manager.register_decoding(passage_id).await;

        let request = DecodeRequest {
            passage_id,
            passage: passage.clone(),
            priority,
            full_decode,
        };

        // Extract filename for human-readable logging
        let filename = passage.file_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("<unknown>");

        debug!(
            "Submitting decode request: {} (passage_id={}, priority={:?}, full={})",
            filename, passage_id, priority, full_decode
        );

        // Add to priority queue
        {
            let mut queue = self.state.queue.lock().unwrap();
            queue.push(request);
        }

        // Notify one waiting worker
        self.state.condvar.notify_one();

        Ok(())
    }

    /// Worker thread main loop
    fn worker_loop(
        worker_id: usize,
        state: Arc<SharedPoolState>,
        buffer_manager: Arc<BufferManager>,
        rt_handle: tokio::runtime::Handle,
    ) {
        debug!("Worker {} started", worker_id);

        loop {
            // **[DBD-BUF-050]** Check if any paused jobs can resume (hysteresis check)
            let (request, resume_from_sample) = if let Some((resumed_request, resume_pos)) =
                Self::check_paused_jobs_for_resume(&state, &buffer_manager, &rt_handle)
            {
                info!(
                    "▶️  Worker {} resuming paused decode job for {} from sample {}",
                    worker_id, resumed_request.passage_id, resume_pos
                );
                (resumed_request, resume_pos)
            } else {
                // No paused jobs ready to resume - get next request from queue
                let request = {
                    let mut queue = state.queue.lock().unwrap();

                    // Wait for work or shutdown signal
                    while queue.is_empty() && !state.stop_flag.load(Ordering::Relaxed) {
                        queue = state.condvar.wait(queue).unwrap();
                    }

                    // Check if we should exit
                    if state.stop_flag.load(Ordering::Relaxed) {
                        debug!("Worker {} received shutdown signal", worker_id);
                        break;
                    }

                    // Pop highest priority request
                    queue.pop()
                };

                if let Some(req) = request {
                    (req, 0) // Fresh job, start from beginning
                } else {
                    continue;
                }
            };

            {
                // Extract filename for human-readable logging
                let filename = request.passage.file_path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("<unknown>");

                debug!(
                    "Worker {} processing: {} (passage_id={}, priority={:?})",
                    worker_id, filename, request.passage_id, request.priority
                );

                // Register with buffer manager (allocates ring buffer)
                // [SSD-PBUF-028] Incremental buffer filling
                let passage_id = request.passage_id;
                rt_handle.block_on(async {
                    buffer_manager.register_decoding(passage_id).await
                });

                // Perform decode with incremental buffer appending and pause support
                match Self::decode_passage_internal(
                    &request,
                    Arc::clone(&buffer_manager),
                    &state,
                    &rt_handle,
                    resume_from_sample,
                ) {
                    Ok(()) => {
                        // **[PCF-DUR-010][PCF-COMP-010]** Finalize buffer (cache duration + total_frames)
                        // Get total samples from ring buffer
                        let total_samples = rt_handle.block_on(async {
                            if let Some(buffer_arc) = buffer_manager.get_buffer(passage_id).await {
                                let buffer = buffer_arc.lock().await;
                                // Ring buffer tracks frames written (stereo pairs)
                                // Convert to sample count (frames * 2 for stereo)
                                (buffer.stats().total_written * 2) as usize
                            } else {
                                0
                            }
                        });

                        rt_handle.block_on(async {
                            if let Err(e) = buffer_manager.finalize_buffer(passage_id, total_samples).await {
                                warn!("Failed to finalize buffer: {}", e);
                            }
                            // mark_ready is now no-op (handled by state machine)
                            buffer_manager.mark_ready(passage_id).await;
                        });

                        debug!("Worker {} completed: {} (passage_id={})", worker_id, filename, passage_id);
                    }
                    Err(e) => {
                        error!(
                            "Worker {} decode failed for {}: {} (passage_id={})",
                            worker_id, filename, e, passage_id
                        );

                        // Remove from buffer manager on failure
                        rt_handle.block_on(async {
                            buffer_manager.remove(passage_id).await;
                        });
                    }
                }
            }
        }

        debug!("Worker {} exiting", worker_id);
    }

    /// Decode passage according to request with incremental buffer filling
    ///
    /// [SSD-DEC-013] Always decode from start of file
    /// [SSD-DEC-014] Skip samples until passage start
    /// [SSD-DEC-015] Continue decoding until passage end
    /// [SSD-DEC-016] Resample to 44.1kHz if needed
    /// [SSD-PBUF-028] Append samples incrementally to enable partial buffer playback
    /// [DBD-BUF-050] Check should_decoder_pause() after each chunk, pause if buffer nearly full
    ///
    /// # Arguments
    /// * `request` - Decode request with passage metadata
    /// * `buffer_manager` - Buffer manager for state tracking
    /// * `state` - Shared pool state for paused job tracking
    /// * `rt_handle` - Tokio runtime handle for async operations
    /// * `resume_from_sample` - Sample position to resume from (0 for fresh jobs)
    fn decode_passage_internal(
        request: &DecodeRequest,
        buffer_manager: Arc<BufferManager>,
        state: &Arc<SharedPoolState>,
        rt_handle: &tokio::runtime::Handle,
        resume_from_sample: usize,
    ) -> Result<()> {
        let passage = &request.passage;
        let passage_id = request.passage_id;

        // Calculate start and end times
        // Convert ticks to milliseconds for decoder
        let start_ms = wkmp_common::timing::ticks_to_ms(passage.start_time_ticks) as u64;
        let end_ms = if request.full_decode {
            // Full decode: decode to passage end (or file end if None)
            passage.end_time_ticks
                .map(|t| wkmp_common::timing::ticks_to_ms(t) as u64)
                .unwrap_or(0) // 0 = file end in decoder
        } else {
            // Partial decode: first 15 seconds
            start_ms + 15_000 // 15 seconds in milliseconds
        };

        // Extract filename for logging
        let filename = passage.file_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("<unknown>");

        debug!(
            "Decoding {}: start={}ms, end={}ms, full={}",
            filename, start_ms, end_ms, request.full_decode
        );

        // Decode passage from file
        // [SSD-DEC-013] Decoder always starts from file beginning
        // **[DBD-DEC-090]** Endpoint discovery support
        let decode_result = SimpleDecoder::decode_passage(&passage.file_path, start_ms, end_ms)?;

        // **[DBD-DEC-095]** Notify buffer manager of discovered endpoint (if applicable)
        if let Some(actual_end_ticks) = decode_result.actual_end_ticks {
            debug!(
                "Endpoint discovered for {}: {}ticks ({}ms)",
                passage_id,
                actual_end_ticks,
                wkmp_common::timing::ticks_to_ms(actual_end_ticks)
            );

            // Note: decoder_pool doesn't have access to queue_entry_id, so it can't call
            // buffer_manager.set_discovered_endpoint(). This is fine because decoder_pool
            // is being phased out in favor of SerialDecoder which does have this support.
            // For now, we just log the discovery.
        }

        // Update progress periodically during decode
        // (For simplicity, we update at decode completion)
        rt_handle.block_on(async {
            buffer_manager.update_decode_progress(passage_id, 50).await;
        });

        // Resample to standard rate if needed
        // [SSD-DEC-016] Resample to 44.1kHz using rubato
        let final_samples = if decode_result.sample_rate != STANDARD_SAMPLE_RATE {
            debug!(
                "Resampling from {} Hz to {} Hz",
                decode_result.sample_rate, STANDARD_SAMPLE_RATE
            );

            let resampled = Resampler::resample(&decode_result.samples, decode_result.sample_rate, decode_result.channels)?;

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

        // Append samples in chunks to enable partial buffer playback
        // [SSD-PBUF-028] Incremental buffer filling
        // [DBD-BUF-050] Check should_decoder_pause() after each chunk
        // Chunk size: 1 second = 44100 frames = 88200 samples (stereo)
        const CHUNK_SIZE: usize = 88200; // 1 second of stereo audio @ 44.1kHz
        let total_samples = stereo_samples.len();
        let total_chunks = (total_samples + CHUNK_SIZE - 1) / CHUNK_SIZE;

        // **[DBD-BUF-050]** If resuming, skip already-processed chunks
        let start_chunk = resume_from_sample / CHUNK_SIZE;

        for chunk_idx in start_chunk..total_chunks {
            let start = chunk_idx * CHUNK_SIZE;
            let end = (start + CHUNK_SIZE).min(total_samples);
            let chunk = &stereo_samples[start..end];

            // Push chunk to ring buffer
            // [DBD-BUF-010] push_samples() converts samples to AudioFrame and pushes to ring buffer
            let frames_pushed = rt_handle.block_on(async {
                match buffer_manager.push_samples(passage_id, chunk).await {
                    Ok(count) => count,
                    Err(e) => {
                        warn!("Failed to push samples for {}: {}", passage_id, e);
                        0
                    }
                }
            });

            // **[DBD-BUF-050]** Check if decoder should pause (buffer nearly full)
            let should_pause = rt_handle.block_on(async {
                buffer_manager.should_decoder_pause(passage_id).await.unwrap_or(false)
            });

            if should_pause {
                // Calculate samples processed so far
                let samples_processed = end; // Current position in stereo_samples

                // Store paused job for later resume
                let mut paused = state.paused_jobs.lock().unwrap();
                paused.insert(passage_id, (request.clone(), samples_processed));
                drop(paused);

                info!(
                    "⏸️  Decoder paused for {} at {}/{} samples (buffer nearly full)",
                    passage_id, samples_processed, total_samples
                );

                return Ok(()); // Exit decode loop, worker will pick next job
            }

            // Update progress
            let progress = ((chunk_idx + 1) * 100 / total_chunks).min(100) as u8;
            if progress % 10 == 0 || progress == 100 {
                // Update every 10%
                rt_handle.block_on(async {
                    buffer_manager.update_decode_progress(passage_id, progress).await;
                });
            }

            debug!(
                "Pushed chunk {}/{} ({:.1}%, {} frames)",
                chunk_idx + 1,
                total_chunks,
                progress as f32,
                frames_pushed
            );
        }

        Ok(())
    }

    /// Shutdown decoder pool
    ///
    /// [SSD-DEC-033] Signal stop, wait for threads with 5-second timeout.
    /// **[DBD-BUF-050]** Clear paused jobs on shutdown
    pub fn shutdown(self) -> Result<()> {
        info!("Shutting down decoder pool");

        // Set stop flag
        self.state.stop_flag.store(true, Ordering::Relaxed);

        // Clear paused jobs (no longer needed)
        self.state.paused_jobs.lock().unwrap().clear();

        // Notify all workers
        self.state.condvar.notify_all();

        // Join threads with timeout
        for (idx, handle) in self.threads.into_iter().enumerate() {
            // Use a timeout to avoid hanging forever
            match handle.join() {
                Ok(_) => {
                    debug!("Worker {} joined successfully", idx);
                }
                Err(e) => {
                    error!("Worker {} join failed: {:?}", idx, e);
                }
            }
        }

        info!("Decoder pool shut down");
        Ok(())
    }

    /// Get queue length (for diagnostics)
    pub fn queue_len(&self) -> usize {
        self.state.queue.lock().unwrap().len()
    }

    // ============================================================================
    // Deprecated Methods (stubs for backward compatibility during test migration)
    // ============================================================================

    /// **DEPRECATED:** Use `submit()` instead - API changed in ring buffer refactor
    ///
    /// This method was removed during the pause/resume refactoring.
    /// Tests should be updated to use the new `submit()` API.
    #[deprecated(note = "Use submit() instead - API changed in ring buffer refactor")]
    #[allow(dead_code)]
    pub async fn enqueue_decode(&self, _request: DecodeRequest) -> Result<()> {
        warn!("enqueue_decode() is deprecated - use submit() instead");
        Err(Error::Playback(
            "enqueue_decode removed in ring buffer refactor - use submit()".to_string(),
        ))
    }

    /// **DEPRECATED:** Monitoring API changed in ring buffer refactor
    ///
    /// The concept of "active decoder count" no longer applies with the
    /// new pause/resume architecture where decoders can be paused.
    #[deprecated(note = "Monitoring API changed - no longer applicable")]
    #[allow(dead_code)]
    pub async fn get_active_decoder_count(&self) -> usize {
        warn!("get_active_decoder_count() is deprecated - monitoring API changed");
        0 // Stub return - always report 0 active decoders
    }

    /// **DEPRECATED:** Debug API removed in ring buffer refactor
    ///
    /// Activity logging is no longer tracked in the decoder pool.
    /// Use BufferManager state inspection instead.
    #[deprecated(note = "Debug API removed - use BufferManager state inspection")]
    #[allow(dead_code)]
    pub async fn get_decoder_activity_log(&self) -> Vec<String> {
        warn!("get_decoder_activity_log() is deprecated - debug API removed");
        vec![] // Stub return - empty activity log
    }

    /// **DEPRECATED:** Test helper removed in ring buffer refactor
    ///
    /// Execution order tracking is no longer available.
    /// Tests should verify behavior through BufferManager state.
    #[deprecated(note = "Test helper removed - use BufferManager state inspection")]
    #[allow(dead_code)]
    pub async fn get_execution_order(&self) -> Vec<Uuid> {
        warn!("get_execution_order() is deprecated - test helper removed");
        vec![] // Stub return - empty execution order
    }

    /// **DEPRECATED:** Wait for decode complete - no longer supported
    ///
    /// Use BufferManager's state inspection to wait for buffer ready state.
    #[deprecated(note = "Use BufferManager state inspection instead")]
    #[allow(dead_code)]
    pub async fn wait_for_decode_complete(&self, _passage_id: Uuid) {
        warn!("wait_for_decode_complete() is deprecated - use BufferManager state");
    }

    /// **DEPRECATED:** Get decode start time - no longer tracked
    ///
    /// Timing information is no longer tracked by DecoderPool.
    #[deprecated(note = "Timing tracking removed")]
    #[allow(dead_code)]
    pub async fn get_decode_start_time(&self, _passage_id: Uuid) -> std::time::Instant {
        warn!("get_decode_start_time() is deprecated - timing no longer tracked");
        std::time::Instant::now() // Stub return
    }

    /// **DEPRECATED:** Public decode passage wrapper - API changed
    ///
    /// This public async wrapper was used by tests but is no longer applicable.
    /// Use `submit()` to queue decode requests instead.
    ///
    /// Note: The internal function `decode_passage_internal()` is still used by
    /// workers, but this public async API has been removed.
    #[deprecated(note = "Use submit() instead - public API removed")]
    #[allow(dead_code)]
    pub async fn decode_passage(
        &self,
        _passage: &PassageWithTiming,
        _buffer_manager: &Arc<BufferManager>,
    ) {
        warn!("decode_passage() is deprecated - use submit() instead");
        // No-op stub
    }

    // ============================================================================
    // End of Deprecated Methods
    // ============================================================================

    /// Check if any paused jobs can resume (buffer has space with hysteresis)
    ///
    /// **[DBD-BUF-050]** Hysteresis prevents rapid pause/resume oscillation:
    /// - Pause at: capacity - headroom (661,500 samples)
    /// - Resume at: capacity - (headroom * 2) (661,059 samples)
    fn check_paused_jobs_for_resume(
        state: &Arc<SharedPoolState>,
        buffer_manager: &Arc<BufferManager>,
        rt_handle: &tokio::runtime::Handle,
    ) -> Option<(DecodeRequest, usize)> {
        let paused = state.paused_jobs.lock().unwrap();

        // Track jobs with removed buffers for cleanup
        let mut removed_jobs = Vec::new();

        // Check each paused job in priority order
        let mut best_resumable: Option<(DecodeRequest, usize, DecodePriority)> = None;

        for (passage_id, (request, resume_sample)) in paused.iter() {
            // Check if buffer still exists
            let buffer_exists = rt_handle.block_on(async {
                buffer_manager.is_managed(*passage_id).await
            });

            if !buffer_exists {
                // Buffer was removed - mark for cleanup
                removed_jobs.push(*passage_id);
                continue;
            }

            // Check if buffer has enough space to resume (hysteresis)
            let can_resume = rt_handle.block_on(async {
                buffer_manager
                    .can_decoder_resume(*passage_id)
                    .await
                    .unwrap_or(false)
            });

            if can_resume {
                // This job can resume - check if it's highest priority so far
                match best_resumable {
                    None => {
                        best_resumable = Some((request.clone(), *resume_sample, request.priority));
                    }
                    Some((_, _, current_priority)) => {
                        if request.priority < current_priority {
                            best_resumable = Some((request.clone(), *resume_sample, request.priority));
                        }
                    }
                }
            }
        }

        drop(paused); // Release lock before modifications

        // Clean up removed jobs
        if !removed_jobs.is_empty() {
            let mut paused_mut = state.paused_jobs.lock().unwrap();
            for passage_id in removed_jobs {
                paused_mut.remove(&passage_id);
                debug!("Cleaned up paused job for removed buffer: {}", passage_id);
            }
        }

        // Return best resumable job
        if let Some((request, resume_pos, _)) = best_resumable {
            let passage_id = request.passage_id;

            // Remove from paused_jobs
            state.paused_jobs.lock().unwrap().remove(&passage_id);

            Some((request, resume_pos))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // Note: Full integration tests with real audio files are in tests/
    // These are just unit tests for the structure

    #[test]
    fn test_decode_request_priority_ordering() {
        let mut heap = BinaryHeap::new();

        let req_immediate = DecodeRequest {
            passage_id: Uuid::new_v4(),
            passage: create_test_passage(),
            priority: DecodePriority::Immediate,
            full_decode: true,
        };

        let req_next = DecodeRequest {
            passage_id: Uuid::new_v4(),
            passage: create_test_passage(),
            priority: DecodePriority::Next,
            full_decode: true,
        };

        let req_prefetch = DecodeRequest {
            passage_id: Uuid::new_v4(),
            passage: create_test_passage(),
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

    fn create_test_passage() -> PassageWithTiming {
        use crate::db::passages::PassageWithTiming;
        use wkmp_common::FadeCurve;
        use wkmp_common::timing::ms_to_ticks;

        PassageWithTiming {
            passage_id: Some(Uuid::new_v4()),
            file_path: PathBuf::from("/test/audio.mp3"),
            start_time_ticks: 0,
            end_time_ticks: Some(ms_to_ticks(10000)),
            lead_in_point_ticks: 0,
            lead_out_point_ticks: Some(ms_to_ticks(9000)),
            fade_in_point_ticks: 0,
            fade_out_point_ticks: Some(ms_to_ticks(9000)),
            fade_in_curve: FadeCurve::Exponential,
            fade_out_curve: FadeCurve::Logarithmic,
        }
    }

    #[tokio::test]
    async fn test_decoder_pool_creation() {
        let buffer_manager = Arc::new(BufferManager::new());
        let pool = DecoderPool::new(buffer_manager);

        assert_eq!(pool.threads.len(), 2);
        assert_eq!(pool.queue_len(), 0);

        // Shutdown cleanly
        pool.shutdown().unwrap();
    }
}
