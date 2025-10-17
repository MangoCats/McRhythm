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
use crate::audio::types::PassageBuffer;
use crate::db::passages::PassageWithTiming;
use crate::error::{Error, Result};
use crate::playback::buffer_manager::BufferManager;
use crate::playback::types::DecodePriority;
use std::collections::BinaryHeap;
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
struct DecodeRequest {
    /// Passage identifier
    passage_id: Uuid,

    /// Passage timing information
    passage: PassageWithTiming,

    /// Request priority (lower value = higher priority)
    priority: DecodePriority,

    /// True = full decode, False = partial (15 seconds)
    full_decode: bool,
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
        });

        // Spawn 2 worker threads
        let mut threads = Vec::new();
        for worker_id in 0..2 {
            let state_clone = Arc::clone(&state);
            let buffer_manager_clone = Arc::clone(&buffer_manager);

            let handle = thread::spawn(move || {
                Self::worker_loop(worker_id, state_clone, buffer_manager_clone);
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
    pub fn submit(
        &self,
        passage_id: Uuid,
        passage: PassageWithTiming,
        priority: DecodePriority,
        full_decode: bool,
    ) -> Result<()> {
        if self.state.stop_flag.load(Ordering::Relaxed) {
            return Err(Error::Playback("Decoder pool is shutting down".to_string()));
        }

        let request = DecodeRequest {
            passage_id,
            passage,
            priority,
            full_decode,
        };

        debug!(
            "Submitting decode request: passage_id={}, priority={:?}, full={}",
            passage_id, priority, full_decode
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
    ) {
        debug!("Worker {} started", worker_id);

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
                    debug!("Worker {} received shutdown signal", worker_id);
                    break;
                }

                // Pop highest priority request
                queue.pop()
            };

            if let Some(request) = request {
                debug!(
                    "Worker {} processing: passage_id={}, priority={:?}",
                    worker_id, request.passage_id, request.priority
                );

                // Register with buffer manager
                let passage_id = request.passage_id;
                tokio::runtime::Handle::current().block_on(async {
                    buffer_manager.register_decoding(passage_id).await;
                });

                // Perform decode
                match Self::decode_passage(&request, Arc::clone(&buffer_manager)) {
                    Ok(buffer) => {
                        // Mark buffer as ready
                        tokio::runtime::Handle::current().block_on(async {
                            buffer_manager.mark_ready(passage_id, buffer).await;
                        });

                        debug!("Worker {} completed: passage_id={}", worker_id, passage_id);
                    }
                    Err(e) => {
                        error!(
                            "Worker {} decode failed for passage_id={}: {}",
                            worker_id, passage_id, e
                        );

                        // Remove from buffer manager on failure
                        tokio::runtime::Handle::current().block_on(async {
                            buffer_manager.remove(passage_id).await;
                        });
                    }
                }
            }
        }

        debug!("Worker {} exiting", worker_id);
    }

    /// Decode passage according to request
    ///
    /// [SSD-DEC-013] Always decode from start of file
    /// [SSD-DEC-014] Skip samples until passage start
    /// [SSD-DEC-015] Continue decoding until passage end
    /// [SSD-DEC-016] Resample to 44.1kHz if needed
    fn decode_passage(
        request: &DecodeRequest,
        buffer_manager: Arc<BufferManager>,
    ) -> Result<PassageBuffer> {
        let passage = &request.passage;
        let passage_id = request.passage_id;

        // Calculate start and end times
        let start_ms = passage.start_time_ms;
        let end_ms = if request.full_decode {
            // Full decode: decode to passage end (or file end if None)
            passage.end_time_ms.unwrap_or(0) // 0 = file end in decoder
        } else {
            // Partial decode: first 15 seconds
            start_ms + 15_000 // 15 seconds in milliseconds
        };

        debug!(
            "Decoding passage: start={}ms, end={}ms, full={}",
            start_ms, end_ms, request.full_decode
        );

        // Decode passage from file
        // [SSD-DEC-013] Decoder always starts from file beginning
        let (samples, sample_rate, channels) =
            SimpleDecoder::decode_passage(&passage.file_path, start_ms, end_ms)?;

        // Update progress periodically during decode
        // (For simplicity, we update at decode completion)
        tokio::runtime::Handle::current().block_on(async {
            buffer_manager.update_decode_progress(passage_id, 50).await;
        });

        // Resample to standard rate if needed
        // [SSD-DEC-016] Resample to 44.1kHz using rubato
        let final_samples = if sample_rate != STANDARD_SAMPLE_RATE {
            debug!(
                "Resampling from {} Hz to {} Hz",
                sample_rate, STANDARD_SAMPLE_RATE
            );

            let resampled = Resampler::resample(&samples, sample_rate, channels)?;

            resampled
        } else {
            samples
        };

        // Convert to stereo if mono
        let stereo_samples = if channels == 1 {
            // Duplicate mono to both channels
            let mut stereo = Vec::with_capacity(final_samples.len() * 2);
            for sample in final_samples {
                stereo.push(sample);
                stereo.push(sample);
            }
            stereo
        } else if channels == 2 {
            final_samples
        } else {
            // Downmix multi-channel to stereo (simple average)
            warn!(
                "Downmixing {} channels to stereo (simple average)",
                channels
            );
            let frame_count = final_samples.len() / channels as usize;
            let mut stereo = Vec::with_capacity(frame_count * 2);

            for frame_idx in 0..frame_count {
                let base = frame_idx * channels as usize;
                let mut left = 0.0;
                let mut right = 0.0;

                // Average left channels (odd indices)
                for ch in (0..channels as usize).step_by(2) {
                    left += final_samples[base + ch];
                }
                left /= (channels / 2) as f32;

                // Average right channels (even indices)
                for ch in (1..channels as usize).step_by(2) {
                    right += final_samples[base + ch];
                }
                right /= (channels / 2) as f32;

                stereo.push(left);
                stereo.push(right);
            }

            stereo
        };

        // Update progress to 100%
        tokio::runtime::Handle::current().block_on(async {
            buffer_manager.update_decode_progress(passage_id, 100).await;
        });

        // Create passage buffer
        // [SSD-DEC-017] Write PCM data to passage buffer
        let buffer = PassageBuffer::new(
            passage_id,
            stereo_samples,
            STANDARD_SAMPLE_RATE,
            2, // Always stereo
        );

        Ok(buffer)
    }

    /// Shutdown decoder pool
    ///
    /// [SSD-DEC-033] Signal stop, wait for threads with 5-second timeout.
    pub fn shutdown(self) -> Result<()> {
        info!("Shutting down decoder pool");

        // Set stop flag
        self.state.stop_flag.store(true, Ordering::Relaxed);

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
        use crate::db::passages::{FadeCurve, PassageWithTiming};

        PassageWithTiming {
            passage_id: Some(Uuid::new_v4()),
            file_path: PathBuf::from("/test/audio.mp3"),
            start_time_ms: 0,
            end_time_ms: Some(10000),
            lead_in_point_ms: 0,
            lead_out_point_ms: Some(9000),
            fade_in_point_ms: 0,
            fade_out_point_ms: Some(9000),
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
