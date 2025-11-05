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
use crate::playback::queue_manager::QueueManager;
use crate::playback::types::DecodePriority;
use crate::state::SharedState;
use sqlx::{Pool, Sqlite};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{sleep, Duration};
use tracing::{debug, error, info, warn};
use uuid::Uuid;
use wkmp_common::events::WkmpEvent;

/// Decode request submitted to the worker
///
/// **Phase 4:** full_decode flag reserved for future decode mode control (currently always partial)
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
    ///
    /// **Phase 4:** Stop flag reserved for graceful shutdown (currently uses drop-based cleanup)
    stop_flag: Arc<AtomicBool>,

    /// Chain counter for assigning chain indices
    next_chain_index: usize,

    /// **[DBD-DEC-045]** Currently filling decoder (None = need to select)
    current_decoder_id: Option<Uuid>,

    /// **[DBD-DEC-045]** Last priority re-evaluation time (for decode_work_period trigger)
    last_reevaluation: tokio::time::Instant,

    /// **[DBD-DEC-045]** Chain assignments generation - incremented on chain assign/release
    /// Triggers re-evaluation when chains are added/removed/resumed
    chain_assignments_generation: u64,

    /// **[DBD-DEC-045]** Last observed chain assignments generation
    last_observed_generation: u64,
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

    /// Working sample rate (Hz) - matches audio device native rate
    /// **[DBD-PARAM-020]** Passed to decoder chains for resampling target
    working_sample_rate: Arc<std::sync::RwLock<u32>>,

    /// Queue manager reference for priority queries
    /// **Query-based prioritization:** Used to get current play_order for active chains
    queue: Arc<RwLock<QueueManager>>,

    /// Worker state (protected by async mutex)
    state: Arc<Mutex<WorkerState>>,

    /// Stop flag (shared with worker task)
    stop_flag: Arc<AtomicBool>,
}

impl DecoderWorker {
    /// Create a new decoder worker
    ///
    /// **[Phase 7]** Now requires shared_state and db_pool for error handling.
    /// **[DBD-PARAM-020]** Now requires working_sample_rate for device-matched resampling.
    /// **Query-based prioritization:** Now requires queue for play_order queries.
    pub fn new(
        buffer_manager: Arc<BufferManager>,
        shared_state: Arc<SharedState>,
        db_pool: Pool<Sqlite>,
        working_sample_rate: Arc<std::sync::RwLock<u32>>,
        queue: Arc<RwLock<QueueManager>>,
    ) -> Self {
        let stop_flag = Arc::new(AtomicBool::new(false));

        let state = WorkerState {
            pending_requests: BinaryHeap::new(),
            active_chains: HashMap::new(),
            yielded_chains: HashMap::new(),
            stop_flag: Arc::clone(&stop_flag),
            next_chain_index: 0,
            current_decoder_id: None,
            last_reevaluation: tokio::time::Instant::now(),
            chain_assignments_generation: 0,
            last_observed_generation: 0,
        };

        Self {
            buffer_manager,
            shared_state,
            db_pool,
            working_sample_rate,
            queue,
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

    /// Cancel decode for a specific queue entry
    ///
    /// **[DBD-DEC-045]** Called when passage removed from queue - cleans up decoder chain
    /// Removes chain from pending_requests, active_chains, and yielded_chains
    /// Also removes buffer and increments chain_assignments_generation
    ///
    /// # Arguments
    /// * `queue_entry_id` - UUID of queue entry to cancel
    pub async fn cancel_decode(&self, queue_entry_id: Uuid) {
        let mut state = self.state.lock().await;

        // Remove from pending requests if present
        state.pending_requests.retain(|Reverse(req)| req.queue_entry_id != queue_entry_id);

        // Remove from active chains if present
        if state.active_chains.remove(&queue_entry_id).is_some() {
            debug!("Cancelled active decode chain for queue_entry={}", queue_entry_id);
            // **[DBD-DEC-045]** Chain released - increment generation
            state.chain_assignments_generation += 1;

            // Clear current_decoder_id if this was the current decoder
            if state.current_decoder_id == Some(queue_entry_id) {
                state.current_decoder_id = None;
            }
        }

        // Remove from yielded chains if present
        if state.yielded_chains.remove(&queue_entry_id).is_some() {
            debug!("Cancelled yielded decode chain for queue_entry={}", queue_entry_id);
            // **[DBD-DEC-045]** Chain released - increment generation
            state.chain_assignments_generation += 1;
        }

        // Release lock before async buffer removal
        drop(state);

        // Remove buffer
        self.buffer_manager.remove(queue_entry_id).await;
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
            if iteration.is_multiple_of(10) {
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
                    debug!("[Chain {}] Resuming (buffer drained)", chain.chain_index());
                    state.active_chains.insert(queue_entry_id, chain);
                    // **[DBD-DEC-045]** Chain assignment changed (yielded → active)
                    state.chain_assignments_generation += 1;
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
            // **[DBD-PARAM-050]** Use maximum_decode_streams from GlobalParams
            state.next_chain_index = (state.next_chain_index + 1)
                % *wkmp_common::params::PARAMS.maximum_decode_streams.read().unwrap();

            // Create decoder chain
            // **[DBD-PARAM-020]** Read working_sample_rate (set by audio thread after device init)
            let working_sample_rate = *self.working_sample_rate.read().unwrap();
            match DecoderChain::new(
                request.queue_entry_id,
                chain_index,
                &request.passage,
                &self.buffer_manager,
                working_sample_rate,
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

                    // **[DEBT-007]** Set source sample rate for telemetry
                    let source_rate = chain.source_sample_rate();
                    if let Err(e) = self.buffer_manager.set_source_sample_rate(request.queue_entry_id, source_rate).await {
                        warn!("[Chain {}] Failed to set source sample rate: {}", chain_index, e);
                    }

                    state.active_chains.insert(request.queue_entry_id, chain);
                    // **[DBD-DEC-045]** New chain assigned
                    state.chain_assignments_generation += 1;
                }
                Err(e) => {
                    // **[Phase 7]** Comprehensive error handling per SPEC021
                    self.handle_decode_error(&request, &e).await;
                }
            }
        }

        Ok(())
    }

    /// Process one chunk from current or newly selected decoder
    ///
    /// **[DBD-DEC-045]** Trigger-based re-evaluation:
    /// - Continues filling current_decoder_id until re-evaluation trigger
    /// - Re-evaluation triggers: buffer full, decode_work_period elapsed
    /// - On trigger: selects new chain with lowest play_order whose buffer needs filling
    ///
    /// Returns Some(true) if work was done, Some(false) if no active chains or all buffers full, None on error
    async fn process_one_chunk(&self) -> Result<Option<bool>> {
        // Check if we need to re-evaluate priorities
        let (current_decoder_id, last_reevaluation, chain_gen, last_obs_gen) = {
            let state = self.state.lock().await;
            (
                state.current_decoder_id,
                state.last_reevaluation,
                state.chain_assignments_generation,
                state.last_observed_generation,
            )
        };

        let should_reevaluate = self
            .should_reevaluate(
                current_decoder_id,
                last_reevaluation,
                chain_gen,
                last_obs_gen,
            )
            .await;

        // Acquire lock to select/extract chain
        let (queue_entry_id, mut chain) = {
            let mut state = self.state.lock().await;

            if state.active_chains.is_empty() {
                return Ok(Some(false)); // No work to do
            }

            // Determine which chain to process
            let queue_entry_id = if should_reevaluate {
                // **[DBD-DEC-045]** Re-evaluation triggered: select new chain
                debug!("Re-evaluating priorities");
                match self.select_highest_priority_chain(&state).await {
                    Some(id) => {
                        // Update state: new decoder selected, reset timer, update observed generation
                        state.current_decoder_id = Some(id);
                        state.last_reevaluation = tokio::time::Instant::now();
                        state.last_observed_generation = state.chain_assignments_generation;
                        debug!("Selected new decoder: {}", id);
                        id
                    }
                    None => {
                        // All buffers are full, no work to do
                        debug!("All active chain buffers are full, idling");
                        state.current_decoder_id = None;
                        state.last_observed_generation = state.chain_assignments_generation;
                        return Ok(Some(false));
                    }
                }
            } else {
                // Continue with current decoder (no re-evaluation trigger)
                let id = current_decoder_id.expect("current_decoder_id is Some when !should_reevaluate");

                // Verify current decoder still exists in active_chains
                if !state.active_chains.contains_key(&id) {
                    // Current decoder finished/removed, force re-evaluation on next iteration
                    debug!("Current decoder {} no longer active, clearing selection", id);
                    state.current_decoder_id = None;
                    return Ok(Some(false));
                }

                debug!("Continuing current decoder: {}", id);
                id
            };

            // Remove chain temporarily for processing
            let chain = state
                .active_chains
                .remove(&queue_entry_id)
                .expect("selected chain exists");

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
                // Move to yielded set and clear current decoder (will re-evaluate on next iteration)
                state.yielded_chains.insert(queue_entry_id, chain);
                state.current_decoder_id = None;
                // **[DBD-DEC-045]** Chain assignment changed (active → yielded)
                state.chain_assignments_generation += 1;
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

                        // **[DBD-DEC-045]** Chain released (decode failed - partial < 50%)
                        state.chain_assignments_generation += 1;

                        return Ok(Some(true)); // Don't finalize
                    }
                }

                // Finalize buffer (for successful or ≥50% partial decodes)
                self.buffer_manager
                    .finalize_buffer(queue_entry_id, total_frames * 2) // stereo
                    .await
                    .ok(); // Ignore errors on finalize

                // Chain is complete, clear current decoder (will re-evaluate on next iteration)
                state.current_decoder_id = None;
                // **[DBD-DEC-045]** Chain released (decode finished)
                state.chain_assignments_generation += 1;
                Ok(Some(true))
            }
        }
    }

    /// Check if priority re-evaluation should occur
    ///
    /// **[DBD-DEC-045]** Re-evaluation triggers:
    /// 1. Chain assignments changed (new chain, chain released, chain yielded/resumed)
    /// 2. Current decoder's buffer reached pause threshold (should_decoder_pause == true)
    /// 3. decode_work_period elapsed since last re-evaluation
    ///
    /// # Arguments
    /// * `current_decoder_id` - Currently filling decoder (if any)
    /// * `last_reevaluation` - Time of last priority re-evaluation
    /// * `chain_assignments_generation` - Current chain assignments generation counter
    /// * `last_observed_generation` - Last observed generation counter
    ///
    /// # Returns
    /// * `true` - Should re-evaluate priorities and select new chain
    /// * `false` - Continue filling current chain
    async fn should_reevaluate(
        &self,
        current_decoder_id: Option<Uuid>,
        last_reevaluation: tokio::time::Instant,
        chain_assignments_generation: u64,
        last_observed_generation: u64,
    ) -> bool {
        // Trigger 1: No current decoder (initial state or decoder finished)
        if current_decoder_id.is_none() {
            debug!("Re-evaluation trigger: no current decoder");
            return true;
        }

        // Trigger 2: Chain assignments changed (new/released/yielded/resumed chains)
        if chain_assignments_generation != last_observed_generation {
            debug!(
                "Re-evaluation trigger: chain assignments changed (generation {} → {})",
                last_observed_generation, chain_assignments_generation
            );
            return true;
        }

        let queue_entry_id = current_decoder_id.unwrap();

        // Trigger 3: Current decoder's buffer is at pause threshold
        if let Ok(should_pause) = self.buffer_manager.should_decoder_pause(queue_entry_id).await {
            if should_pause {
                debug!(
                    "Re-evaluation trigger: decoder {} buffer full (pause threshold reached)",
                    queue_entry_id
                );
                return true;
            }
        }

        // Trigger 4: decode_work_period elapsed
        let decode_work_period_ms = *wkmp_common::params::PARAMS.decode_work_period.read().unwrap();
        let elapsed = last_reevaluation.elapsed();
        if elapsed >= tokio::time::Duration::from_millis(decode_work_period_ms as u64) {
            debug!(
                "Re-evaluation trigger: decode_work_period elapsed ({:?} >= {}ms)",
                elapsed, decode_work_period_ms
            );
            return true;
        }

        // No trigger - continue with current decoder
        false
    }

    /// Select the highest priority chain (lowest play_order) from active chains
    ///
    /// **[DBD-DEC-045]** Buffer-fill-aware priority selection:
    /// 1. Sort active chains by play_order (0, 1, 2, ...)
    /// 2. For each chain in priority order, check if buffer needs filling (can_decoder_resume)
    /// 3. Return first chain whose buffer needs filling
    /// 4. If all buffers are full, return None (worker should idle)
    ///
    /// Handles edge cases:
    /// - Passage removed from queue → use i64::MAX (lowest priority)
    /// - Multiple chains with same priority → arbitrary selection (HashMap iteration order)
    ///
    /// # Arguments
    /// * `state` - Worker state containing active chains
    ///
    /// # Returns
    /// Some(UUID) of the chain to process next, or None if all buffers are full
    async fn select_highest_priority_chain(&self, state: &WorkerState) -> Option<Uuid> {
        let queue = self.queue.read().await;

        // Build list of (queue_entry_id, play_order) pairs
        let mut candidates: Vec<(Uuid, i64)> = state
            .active_chains
            .keys()
            .map(|queue_entry_id| {
                let play_order = Self::get_play_order_for_entry(&queue, *queue_entry_id);
                (*queue_entry_id, play_order)
            })
            .collect();

        // Sort by play_order (lowest first = highest priority)
        candidates.sort_by_key(|(_, play_order)| *play_order);

        drop(queue); // Release queue lock before async buffer checks

        debug!(
            "Selecting highest priority chain from {} active chains",
            state.active_chains.len()
        );

        // Check each candidate in priority order
        for (queue_entry_id, play_order) in candidates {
            // Check if buffer needs filling
            let can_resume = self
                .buffer_manager
                .can_decoder_resume(queue_entry_id)
                .await
                .unwrap_or(false);

            debug!(
                "  Chain {}: play_order={}, can_resume={}",
                queue_entry_id, play_order, can_resume
            );

            if can_resume {
                debug!(
                    "Selected chain {} with play_order={} (buffer needs filling)",
                    queue_entry_id, play_order
                );
                return Some(queue_entry_id);
            }
        }

        // All buffers are full
        debug!("All buffers full, no chain selected");
        None
    }

    /// Get play_order for a queue entry
    ///
    /// **Helper for query-based prioritization.** Searches queue for entry and returns play_order.
    /// Returns i64::MAX if entry not found (passage was removed from queue).
    ///
    /// # Arguments
    /// * `queue` - QueueManager reference
    /// * `queue_entry_id` - UUID to look up
    ///
    /// # Returns
    /// play_order value (0=current, 1=next, 2+=queued) or i64::MAX if not found
    fn get_play_order_for_entry(queue: &QueueManager, queue_entry_id: Uuid) -> i64 {
        // Check current
        if let Some(entry) = queue.current() {
            if entry.queue_entry_id == queue_entry_id {
                return entry.play_order;
            }
        }

        // Check next
        if let Some(entry) = queue.next() {
            if entry.queue_entry_id == queue_entry_id {
                return entry.play_order;
            }
        }

        // Check queued
        for entry in queue.queued() {
            if entry.queue_entry_id == queue_entry_id {
                return entry.play_order;
            }
        }

        // Not found - passage was removed from queue
        // Use maximum priority (process last) to allow higher priority chains to finish first
        i64::MAX
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
                    passage_id: passage_id.unwrap_or_else(uuid::Uuid::nil),
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
                    passage_id: passage_id.unwrap_or_else(uuid::Uuid::nil),
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
                // Convert frames to milliseconds @ working sample rate
                // **[DBD-PARAM-020]** Use working sample rate (matches device)
                let working_rate = *self.working_sample_rate.read().unwrap();
                let expected_ms = (*expected_frames as u64 * 1000) / working_rate as u64;
                let actual_ms = (*actual_frames as u64 * 1000) / working_rate as u64;
                let delta_ms = (expected_ms as i64) - (actual_ms as i64);

                self.shared_state.broadcast_event(WkmpEvent::PositionDriftWarning {
                    passage_id: passage_id.unwrap_or_else(uuid::Uuid::nil),
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

    // ========================================================================
    // Test Helpers
    // ========================================================================

    /// Get current decoder target (which buffer being filled)
    ///
    /// **Test helper only** - Hidden from public docs via `#[doc(hidden)]`
    ///
    /// Returns `Some(Uuid)` if decoder is currently filling a buffer,
    /// or `None` if no target selected (all buffers full).
    #[doc(hidden)]
    pub async fn test_get_current_target(&self) -> Option<Uuid> {
        let state = self.state.lock().await;
        state.current_decoder_id
    }

    /// Get chain assignments generation counters
    ///
    /// **Test helper only** - Hidden from public docs via `#[doc(hidden)]`
    ///
    /// Returns `(current_generation, last_observed_generation)`.
    /// When these differ, re-evaluation is pending.
    #[doc(hidden)]
    pub async fn test_get_generation(&self) -> (u64, u64) {
        let state = self.state.lock().await;
        (
            state.chain_assignments_generation,
            state.last_observed_generation,
        )
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
