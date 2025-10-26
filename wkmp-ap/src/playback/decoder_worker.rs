//! Single-threaded decoder worker using DecoderChain architecture
//!
//! Replaces the old SerialDecoder/DecoderPool architecture with a simpler
//! single-threaded worker that processes decoder chains serially.
//!
//! **Architecture:**
//! - Maintains map of active DecoderChains (one per queue_entry_id)
//! - Priority queue for pending decode requests
//! - Single worker loop: start chains, process chunks, handle yields
//!
//! **Traceability:**
//! - [DBD-DEC-040] Serial decoding (one at a time for cache coherency)
//! - [DBD-DEC-090] Streaming/incremental operation
//! - [DBD-DEC-110] ~1 second chunk processing
//! - [DBD-DEC-130] State preservation for pause/resume

use crate::db::passages::PassageWithTiming;
use crate::error::{Error, Result};
use crate::playback::buffer_manager::BufferManager;
use crate::playback::pipeline::{ChunkProcessResult, DecoderChain};
use crate::playback::types::DecodePriority;
use crate::state::SharedState;
use sqlx::{Pool, Sqlite};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use wkmp_common::events::WkmpEvent;

/// Decode request submitted to the worker
#[derive(Clone, Debug)]
struct DecodeRequest {
    queue_entry_id: Uuid,
    passage: PassageWithTiming,
    priority: DecodePriority,
    full_decode: bool,
}

/// Ordering for priority queue (highest priority first)
impl Ord for DecodeRequest {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.priority.cmp(&other.priority)
    }
}

impl PartialOrd for DecodeRequest {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for DecodeRequest {
    fn eq(&self, other: &Self) -> bool {
        self.queue_entry_id == other.queue_entry_id
    }
}

impl Eq for DecodeRequest {}

/// Shared state for the decoder worker
struct WorkerState {
    /// Pending decode requests (priority queue)
    pending_requests: BinaryHeap<Reverse<DecodeRequest>>,

    /// Active decoder chains (queue_entry_id -> chain)
    active_chains: HashMap<Uuid, DecoderChain>,

    /// Chains that need to yield (buffer full)
    yielded_chains: HashMap<Uuid, DecoderChain>,

    /// Stop flag
    stop_flag: Arc<AtomicBool>,

    /// Chain counter for assigning chain indices
    next_chain_index: usize,
}

/// Single-threaded decoder worker
///
/// Processes decoder chains serially for optimal cache coherency.
/// Replaces the old multi-threaded SerialDecoder architecture.
///
/// **[Phase 7]** Enhanced with error handling for decode failures.
pub struct DecoderWorker {
    /// Buffer manager reference
    buffer_manager: Arc<BufferManager>,

    /// Shared state for event emission
    /// **[REQ-AP-ERR-010, REQ-AP-ERR-011]** Error event emission
    shared_state: Arc<SharedState>,

    /// Database pool for passage status updates
    /// **[REQ-AP-ERR-011]** Update decode_status on unsupported codec
    db_pool: Pool<Sqlite>,

    /// Worker state (protected by async mutex)
    state: Arc<Mutex<WorkerState>>,

    /// Stop flag (shared with worker task)
    stop_flag: Arc<AtomicBool>,
}

impl DecoderWorker {
    /// Create a new decoder worker
    ///
    /// **[Phase 7]** Now requires shared_state and db_pool for error handling.
    pub fn new(
        buffer_manager: Arc<BufferManager>,
        shared_state: Arc<SharedState>,
        db_pool: Pool<Sqlite>,
    ) -> Self {
        let stop_flag = Arc::new(AtomicBool::new(false));

        let state = WorkerState {
            pending_requests: BinaryHeap::new(),
            active_chains: HashMap::new(),
            yielded_chains: HashMap::new(),
            stop_flag: Arc::clone(&stop_flag),
            next_chain_index: 0,
        };

        Self {
            buffer_manager,
            shared_state,
            db_pool,
            state: Arc::new(Mutex::new(state)),
            stop_flag,
        }
    }

    /// Submit a decode request
    ///
    /// # Arguments
    /// * `queue_entry_id` - Queue entry UUID
    /// * `passage` - Passage timing information
    /// * `priority` - Decode priority (Immediate/Next/Prefetch)
    /// * `full_decode` - Whether to decode entire passage immediately
    ///
    /// # Returns
    /// Ok if request was queued successfully
    pub async fn submit(
        &self,
        queue_entry_id: Uuid,
        passage: PassageWithTiming,
        priority: DecodePriority,
        full_decode: bool,
    ) -> Result<()> {
        if self.stop_flag.load(Ordering::Relaxed) {
            return Err(Error::Playback(
                "Decoder worker is shutting down".to_string(),
            ));
        }

        // Register buffer immediately (prevents duplicate submissions)
        self.buffer_manager
            .register_decoding(queue_entry_id)
            .await;

        let request = DecodeRequest {
            queue_entry_id,
            passage,
            priority,
            full_decode,
        };

        // Queue the request
        let mut state = self.state.lock().await;
        state.pending_requests.push(Reverse(request.clone()));

        debug!(
            "Queued decode request: queue_entry={}, priority={:?}, pending_count={}",
            queue_entry_id,
            priority,
            state.pending_requests.len()
        );

        Ok(())
    }

    /// Start the decoder worker loop
    ///
    /// Spawns a background task that processes decoder chains.
    /// Returns immediately, worker runs in background until shutdown.
    pub fn start(self: Arc<Self>) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            info!("Decoder worker started");
            self.worker_loop().await;
            info!("Decoder worker stopped");
        })
    }

    /// Main worker loop - processes chains serially
    ///
    /// **[DBD-DEC-040]** Serial decoding for cache coherency
    async fn worker_loop(&self) {
        let mut iteration = 0u64;

        while !self.stop_flag.load(Ordering::Relaxed) {
            iteration += 1;

            // Process one iteration
            match self.process_iteration().await {
                Ok(did_work) => {
                    if !did_work {
                        // No work to do, sleep briefly
                        sleep(Duration::from_millis(10)).await;
                    }
                }
                Err(e) => {
                    error!("Worker iteration {} error: {}", iteration, e);
                    sleep(Duration::from_millis(100)).await;
                }
            }

            // Periodic logging (more frequent for debugging)
            if iteration % 10 == 0 {
                let state = self.state.lock().await;
                debug!(
                    "Worker iteration {}: pending={}, active={}, yielded={}",
                    iteration,
                    state.pending_requests.len(),
                    state.active_chains.len(),
                    state.yielded_chains.len()
                );
            }
        }
    }

    /// Process one worker iteration
    ///
    /// Returns true if work was done, false if idle
    async fn process_iteration(&self) -> Result<bool> {
        let mut state = self.state.lock().await;

        // Step 1: Try to resume yielded chains (check if buffers have drained)
        self.try_resume_yielded_chains(&mut state).await;

        // Step 2: Start pending requests (if slots available)
        self.start_pending_requests(&mut state).await?;

        // Release lock before processing chunk (which is async and may take time)
        drop(state);

        // Step 3: Process one chunk from highest priority active chain
        if let Some(did_work) = self.process_one_chunk().await? {
            return Ok(did_work);
        }

        Ok(false)
    }

    /// Try to resume yielded chains whose buffers have drained
    async fn try_resume_yielded_chains(&self, state: &mut WorkerState) {
        let queue_entry_ids: Vec<Uuid> = state.yielded_chains.keys().copied().collect();

        if !queue_entry_ids.is_empty() {
            debug!("Checking {} yielded chains for resume", queue_entry_ids.len());
        }

        for queue_entry_id in queue_entry_ids {
            let can_resume = self
                .buffer_manager
                .can_decoder_resume(queue_entry_id)
                .await
                .unwrap_or(false);

            if let Some(chain) = state.yielded_chains.get(&queue_entry_id) {
                debug!(
                    "[Chain {}] Resume check: can_resume={}",
                    chain.chain_index(),
                    can_resume
                );
            }

            if can_resume {
                if let Some(chain) = state.yielded_chains.remove(&queue_entry_id) {
                    info!("[Chain {}] Resuming (buffer drained)", chain.chain_index());
                    state.active_chains.insert(queue_entry_id, chain);
                }
            }
        }
    }

    /// Start pending requests (create DecoderChains)
    async fn start_pending_requests(&self, state: &mut WorkerState) -> Result<()> {
        // Process pending requests while we have capacity
        // Note: For single-threaded architecture, we allow unlimited active chains
        // (only one processes at a time anyway)
        while let Some(Reverse(request)) = state.pending_requests.pop() {
            // Skip if already active or yielded
            if state.active_chains.contains_key(&request.queue_entry_id)
                || state.yielded_chains.contains_key(&request.queue_entry_id)
            {
                debug!(
                    "Skipping duplicate request for queue_entry={}",
                    request.queue_entry_id
                );
                continue;
            }

            // Assign chain index
            let chain_index = state.next_chain_index;
            state.next_chain_index = (state.next_chain_index + 1) % 12;

            // Create decoder chain
            match DecoderChain::new(
                request.queue_entry_id,
                chain_index,
                &request.passage,
                &self.buffer_manager,
            )
            .await
            {
                Ok(chain) => {
                    info!(
                        "[Chain {}] Started: queue_entry={}, file={}",
                        chain_index,
                        request.queue_entry_id,
                        request.passage.file_path.display()
                    );
                    state.active_chains.insert(request.queue_entry_id, chain);
                }
                Err(e) => {
                    // **[Phase 7]** Comprehensive error handling per SPEC021
                    self.handle_decode_error(&request, &e).await;
                }
            }
        }

        Ok(())
    }

    /// Process one chunk from the highest priority active chain
    ///
    /// Returns Some(true) if work was done, Some(false) if no active chains, None on error
    async fn process_one_chunk(&self) -> Result<Option<bool>> {
        // Acquire lock to extract chain
        let (queue_entry_id, mut chain) = {
            let mut state = self.state.lock().await;

            if state.active_chains.is_empty() {
                return Ok(Some(false)); // No work to do
            }

            // **[DBD-DEC-040]** Serial decoding: process one chain at a time
            // For now, just pick the first active chain (could be priority-based in future)
            let queue_entry_id = *state
                .active_chains
                .keys()
                .next()
                .expect("checked non-empty");

            // Remove chain temporarily for processing
            let chain = state
                .active_chains
                .remove(&queue_entry_id)
                .expect("just checked exists");

            (queue_entry_id, chain)
            // Lock automatically dropped here when state goes out of scope
        };

        // Process one chunk (no lock held during async operation)
        let result = chain.process_chunk(&self.buffer_manager).await?;

        // Re-acquire lock to update state
        let mut state = self.state.lock().await;

        match result {
            ChunkProcessResult::Processed { frames_pushed } => {
                debug!(
                    "[Chain {}] Processed chunk: {} frames",
                    chain.chain_index(),
                    frames_pushed
                );
                // Put chain back in active set
                state.active_chains.insert(queue_entry_id, chain);
                Ok(Some(true))
            }

            ChunkProcessResult::BufferFull { frames_pushed } => {
                debug!(
                    "[Chain {}] Buffer full (partial push: {} frames), yielding",
                    chain.chain_index(),
                    frames_pushed
                );
                // Move to yielded set
                state.yielded_chains.insert(queue_entry_id, chain);
                Ok(Some(true))
            }

            ChunkProcessResult::Finished { total_frames } => {
                info!(
                    "[Chain {}] Finished: queue_entry={}, total_frames={}",
                    chain.chain_index(),
                    queue_entry_id,
                    total_frames
                );

                // **[REQ-AP-ERR-012]** Check for partial decode
                if let Some((expected_ms, actual_ms, percentage)) = chain.get_partial_decode_info() {
                    let passage_id = chain.passage_id();
                    let file_path = chain.file_path().display().to_string();
                    let timestamp = chrono::Utc::now();

                    if percentage >= 50.0 {
                        // ≥50%: Allow playback but emit warning event
                        warn!(
                            "Partial decode ({}%) for passage_id={:?}, file={}: expected={}ms, actual={}ms",
                            percentage, passage_id, file_path, expected_ms, actual_ms
                        );

                        self.shared_state.broadcast_event(WkmpEvent::PassagePartialDecode {
                            passage_id,
                            file_path,
                            expected_duration_ms: expected_ms,
                            actual_duration_ms: actual_ms,
                            percentage,
                            timestamp,
                        });

                        // Continue - finalize buffer and allow playback
                    } else {
                        // <50%: Skip passage
                        error!(
                            "Partial decode ({}%) below threshold for passage_id={:?}, file={}: expected={}ms, actual={}ms - skipping",
                            percentage, passage_id, file_path, expected_ms, actual_ms
                        );

                        self.shared_state.broadcast_event(WkmpEvent::PassageDecodeFailed {
                            passage_id,
                            error_type: "partial_decode_insufficient".to_string(),
                            error_message: format!(
                                "Only {:.1}% decoded (threshold: 50%)",
                                percentage
                            ),
                            file_path,
                            timestamp,
                        });

                        // Clean up buffer instead of finalizing
                        self.buffer_manager.remove(queue_entry_id).await;

                        return Ok(Some(true)); // Don't finalize
                    }
                }

                // Finalize buffer (for successful or ≥50% partial decodes)
                self.buffer_manager
                    .finalize_buffer(queue_entry_id, total_frames * 2) // stereo
                    .await
                    .ok(); // Ignore errors on finalize

                // Chain is complete, don't put back
                Ok(Some(true))
            }
        }
    }

    /// Handle decode errors per Phase 7 error handling strategy
    ///
    /// **[REQ-AP-ERR-010]** File read errors: skip passage, emit event
    /// **[REQ-AP-ERR-011]** Unsupported codec: mark in DB, skip passage, emit event
    ///
    /// # Arguments
    /// * `request` - The decode request that failed
    /// * `error` - The error that occurred
    async fn handle_decode_error(&self, request: &DecodeRequest, error: &Error) {
        let passage_id = request.passage.passage_id;
        let file_path = request.passage.file_path.display().to_string();
        let timestamp = chrono::Utc::now();

        match error {
            Error::FileReadError { path, source } => {
                // **[REQ-AP-ERR-010]** File read error handling
                error!(
                    "File read error for queue_entry={}, passage_id={:?}, file={}: {}",
                    request.queue_entry_id, passage_id, path.display(), source
                );

                // Emit error event
                self.shared_state.broadcast_event(WkmpEvent::PassageDecodeFailed {
                    passage_id,
                    error_type: "file_read_error".to_string(),
                    error_message: source.to_string(),
                    file_path: path.display().to_string(),
                    timestamp,
                });

                // Clean up buffer registration
                self.buffer_manager
                    .remove(request.queue_entry_id)
                    .await;
            }

            Error::DecoderPanic { path, message } => {
                // **[REQ-AP-ERR-013]** Decoder panic handling
                error!(
                    "Decoder panic for queue_entry={}, passage_id={:?}, file={}: {}",
                    request.queue_entry_id, passage_id, path.display(), message
                );

                // Emit error event
                self.shared_state.broadcast_event(WkmpEvent::PassageDecoderPanic {
                    passage_id,
                    file_path: path.display().to_string(),
                    panic_message: message.clone(),
                    timestamp,
                });

                // Clean up buffer registration
                self.buffer_manager
                    .remove(request.queue_entry_id)
                    .await;
            }

            Error::UnsupportedCodec { path, codec } => {
                // **[REQ-AP-ERR-011]** Unsupported codec handling
                error!(
                    "Unsupported codec for queue_entry={}, passage_id={:?}, file={}: {}",
                    request.queue_entry_id, passage_id, path.display(), codec
                );

                // Emit error event
                self.shared_state.broadcast_event(WkmpEvent::PassageUnsupportedCodec {
                    passage_id,
                    file_path: path.display().to_string(),
                    codec_hint: Some(codec.clone()),
                    timestamp,
                });

                // Update database decode_status if passage has an ID
                if let Some(pid) = passage_id {
                    match sqlx::query(
                        "UPDATE passages SET decode_status = 'unsupported_codec' WHERE guid = ?"
                    )
                    .bind(pid.to_string())
                    .execute(&self.db_pool)
                    .await
                    {
                        Ok(_) => {
                            info!(
                                "Marked passage {} as unsupported_codec in database",
                                pid
                            );
                        }
                        Err(e) => {
                            warn!(
                                "Failed to update decode_status for passage {}: {}",
                                pid, e
                            );
                        }
                    }
                }

                // Clean up buffer registration
                self.buffer_manager
                    .remove(request.queue_entry_id)
                    .await;
            }

            Error::ResamplingInitFailed { source_rate, target_rate, message } => {
                // **[REQ-AP-ERR-050]** Resampling initialization failure
                error!(
                    "Resampling init failed for queue_entry={}, passage_id={:?}, file={}: {}Hz -> {}Hz: {}",
                    request.queue_entry_id, passage_id, file_path, source_rate, target_rate, message
                );

                // Emit error event
                self.shared_state.broadcast_event(WkmpEvent::ResamplingFailed {
                    passage_id: passage_id.unwrap_or_else(|| uuid::Uuid::nil()),
                    source_rate: *source_rate,
                    target_rate: *target_rate,
                    error_message: message.clone(),
                    timestamp,
                });

                // Clean up buffer registration
                self.buffer_manager
                    .remove(request.queue_entry_id)
                    .await;
            }

            Error::ResamplingRuntimeError { position_ms, message } => {
                // **[REQ-AP-ERR-051]** Resampling runtime error
                error!(
                    "Resampling runtime error for queue_entry={}, passage_id={:?}, file={} at {}ms: {}",
                    request.queue_entry_id, passage_id, file_path, position_ms, message
                );

                // Emit error event
                self.shared_state.broadcast_event(WkmpEvent::ResamplingRuntimeError {
                    passage_id: passage_id.unwrap_or_else(|| uuid::Uuid::nil()),
                    position_ms: *position_ms,
                    error_message: message.clone(),
                    timestamp,
                });

                // Clean up buffer registration
                self.buffer_manager
                    .remove(request.queue_entry_id)
                    .await;
            }

            Error::FileHandleExhaustion { path } => {
                // **[REQ-AP-ERR-071]** File handle exhaustion
                error!(
                    "File handle exhaustion for queue_entry={}, passage_id={:?}, file={}: too many open files",
                    request.queue_entry_id, passage_id, path.display()
                );

                // Emit error event
                self.shared_state.broadcast_event(WkmpEvent::FileHandleExhaustion {
                    attempted_file: path.display().to_string(),
                    timestamp,
                });

                // Clean up buffer registration
                self.buffer_manager
                    .remove(request.queue_entry_id)
                    .await;

                // Note: Full retry logic with idle handle cleanup deferred
                // For now, simply skip the passage
            }

            Error::PositionDrift { expected_frames, actual_frames, drift_frames, drift_ms } => {
                // **[REQ-AP-ERR-060]** Position drift warning (moderate drift)
                warn!(
                    "Position drift for queue_entry={}, passage_id={:?}, file={}: expected={} actual={} drift={} frames ({}ms)",
                    request.queue_entry_id, passage_id, file_path, expected_frames, actual_frames, drift_frames, drift_ms
                );

                // Emit warning event
                // Convert frames to milliseconds @ 44.1kHz
                let expected_ms = (*expected_frames as u64 * 1000) / 44100;
                let actual_ms = (*actual_frames as u64 * 1000) / 44100;
                let delta_ms = (expected_ms as i64) - (actual_ms as i64);

                self.shared_state.broadcast_event(WkmpEvent::PositionDriftWarning {
                    passage_id: passage_id.unwrap_or_else(|| uuid::Uuid::nil()),
                    expected_position_ms: expected_ms,
                    actual_position_ms: actual_ms,
                    delta_ms,
                    timestamp,
                });

                // Note: According to spec, position should be resynced
                // For now, we skip the passage as the drift indicates a problem
                // Future enhancement: implement position resync logic

                // Clean up buffer registration
                self.buffer_manager
                    .remove(request.queue_entry_id)
                    .await;
            }

            _ => {
                // Other errors (generic decode errors)
                error!(
                    "Decode error for queue_entry={}, passage_id={:?}, file={}: {}",
                    request.queue_entry_id, passage_id, file_path, error
                );

                // Emit generic decode failed event
                self.shared_state.broadcast_event(WkmpEvent::PassageDecodeFailed {
                    passage_id,
                    error_type: "decode_error".to_string(),
                    error_message: error.to_string(),
                    file_path,
                    timestamp,
                });

                // Clean up buffer registration
                self.buffer_manager
                    .remove(request.queue_entry_id)
                    .await;
            }
        }
    }

    /// Shutdown the worker
    pub async fn shutdown(self: Arc<Self>) {
        info!("Shutting down decoder worker");
        self.stop_flag.store(true, Ordering::Relaxed);

        // Clear all state
        let mut state = self.state.lock().await;
        state.pending_requests.clear();
        state.active_chains.clear();
        state.yielded_chains.clear();

        info!("Decoder worker shutdown complete");
    }
}
