//! Buffer Manager
//!
//! Event-driven buffer lifecycle management with state machine.
//!
//! **Traceability:**
//! - [DBD-BUF-010] Buffer management strategy
//! - [DBD-BUF-020] through [DBD-BUF-060] Buffer lifecycle states
//! - [DBD-BUF-070] Buffer exhaustion detection
//! - [DBD-BUF-080] Underrun recovery
//! - [PERF-POLL-010] Event-driven buffer readiness notification

use crate::audio::types::AudioFrame;
use crate::playback::buffer_events::{BufferEvent, BufferMetadata, BufferState};
use crate::playback::playout_ring_buffer::{PlayoutRingBuffer, BufferFullError};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info, warn};
use uuid::Uuid;

/// **[DBD-PARAM-020]** Read working sample rate from GlobalParams (default: 44100 Hz per SPEC016)
fn standard_sample_rate() -> u32 {
    *wkmp_common::params::PARAMS.working_sample_rate.read().unwrap()
}

/// [DBD-PARAM-080] Buffer headroom threshold (5 seconds = 220,500 samples @ 44.1kHz stereo)
const BUFFER_HEADROOM_THRESHOLD: usize = 220_500;

/// Wrapper for buffer with state machine metadata
struct ManagedBuffer {
    /// The actual playout ring buffer
    /// **[DBD-BUF-010]** Fixed-capacity ring buffer holds playout_ringbuffer_size stereo samples
    /// No outer Mutex needed - PlayoutRingBuffer is internally lock-free
    buffer: Arc<PlayoutRingBuffer>,

    /// Buffer state and position metadata
    metadata: BufferMetadata,
}

/// Manages passage buffers with event-driven state machine
///
/// **[DBD-BUF-010]** Buffer management strategy:
/// - Event-driven state transitions (no polling)
/// - Buffer exhaustion detection
/// - Automatic lifecycle management
pub struct BufferManager {
    /// Map of queue_entry_id -> managed buffer
    buffers: Arc<RwLock<HashMap<Uuid, ManagedBuffer>>>,

    /// Event channel for buffer state notifications
    /// **[PERF-POLL-010]** Enable event-driven playback
    event_tx: Arc<RwLock<Option<mpsc::UnboundedSender<BufferEvent>>>>,

    /// Minimum buffer threshold in milliseconds
    /// **[PERF-START-010]** Configurable minimum buffer for instant startup
    min_buffer_threshold_ms: Arc<RwLock<u64>>,

    /// Resume hysteresis threshold in samples
    /// **[DBD-BUF-050]** Configurable hysteresis prevents pause/resume oscillation
    resume_hysteresis: Arc<RwLock<usize>>,

    /// Ring buffer capacity in stereo frames
    /// **[DBD-PARAM-070]** Configurable buffer capacity (default: 661,941)
    buffer_capacity: Arc<RwLock<usize>>,

    /// Ring buffer headroom threshold in stereo frames
    /// **[DBD-PARAM-080]** Configurable headroom threshold (default: 4,410)
    buffer_headroom: Arc<RwLock<usize>>,

    /// Whether any passage has ever been played (for first-passage optimization)
    /// **[PERF-FIRST-010]** Use smaller buffer for first passage only
    ever_played: Arc<AtomicBool>,
}

impl BufferManager {
    /// Create new buffer manager
    pub fn new() -> Self {
        Self {
            buffers: Arc::new(RwLock::new(HashMap::new())),
            event_tx: Arc::new(RwLock::new(None)),
            min_buffer_threshold_ms: Arc::new(RwLock::new(3000)), // Default: 3 seconds
            resume_hysteresis: Arc::new(RwLock::new(44100)), // Default: 1.0 second @ 44.1kHz
            buffer_capacity: Arc::new(RwLock::new(661_941)), // Default: 15.01s @ 44.1kHz
            buffer_headroom: Arc::new(RwLock::new(4_410)), // Default: 0.1s @ 44.1kHz
            ever_played: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Set buffer event channel
    ///
    /// **[PERF-POLL-010]** Enable event-driven playback start
    pub async fn set_event_channel(&self, tx: mpsc::UnboundedSender<BufferEvent>) {
        *self.event_tx.write().await = Some(tx);
    }

    /// Set minimum buffer threshold
    ///
    /// **[PERF-START-010]** Configurable minimum buffer
    pub async fn set_min_buffer_threshold(&self, threshold_ms: u64) {
        *self.min_buffer_threshold_ms.write().await = threshold_ms;
    }

    /// Get minimum buffer threshold
    ///
    /// **[REQ-AP-ERR-020]** Used for buffer underrun recovery threshold
    pub async fn get_min_buffer_threshold(&self) -> u64 {
        *self.min_buffer_threshold_ms.read().await
    }

    /// Set decoder resume hysteresis threshold
    ///
    /// **[DBD-BUF-050]** Configurable hysteresis prevents pause/resume oscillation
    pub async fn set_resume_hysteresis(&self, hysteresis_samples: usize) {
        *self.resume_hysteresis.write().await = hysteresis_samples;
    }

    /// Set ring buffer capacity
    ///
    /// **[DBD-PARAM-070]** Configurable buffer capacity in stereo frames
    pub async fn set_buffer_capacity(&self, capacity: usize) {
        *self.buffer_capacity.write().await = capacity;
    }

    /// Set ring buffer headroom threshold
    ///
    /// **[DBD-PARAM-080]** Configurable headroom threshold in stereo frames
    pub async fn set_buffer_headroom(&self, headroom: usize) {
        *self.buffer_headroom.write().await = headroom;
    }

    /// Set decode start time for telemetry
    ///
    /// **[REQ-DEBT-FUNC-001]** Decoder telemetry tracking
    pub async fn set_decode_started(&self, queue_entry_id: Uuid) -> Result<(), String> {
        let mut buffers = self.buffers.write().await;
        if let Some(managed) = buffers.get_mut(&queue_entry_id) {
            managed.metadata.decode_started_at = Some(Instant::now());
            Ok(())
        } else {
            Err(format!("Buffer {} not found", queue_entry_id))
        }
    }

    /// Set decode completion time for telemetry
    ///
    /// **[REQ-DEBT-FUNC-001]** Decoder telemetry tracking
    pub async fn set_decode_completed(&self, queue_entry_id: Uuid) -> Result<(), String> {
        let mut buffers = self.buffers.write().await;
        if let Some(managed) = buffers.get_mut(&queue_entry_id) {
            managed.metadata.decode_completed_at = Some(Instant::now());
            Ok(())
        } else {
            Err(format!("Buffer {} not found", queue_entry_id))
        }
    }

    /// Set file path for telemetry
    ///
    /// **[REQ-DEBT-FUNC-001]** Decoder telemetry tracking
    pub async fn set_file_path(&self, queue_entry_id: Uuid, file_path: String) -> Result<(), String> {
        let mut buffers = self.buffers.write().await;
        if let Some(managed) = buffers.get_mut(&queue_entry_id) {
            managed.metadata.file_path = Some(file_path);
            Ok(())
        } else {
            Err(format!("Buffer {} not found", queue_entry_id))
        }
    }

    /// Set source sample rate for telemetry
    ///
    /// **[DEBT-007]** Buffer chain telemetry - track actual source file sample rate
    pub async fn set_source_sample_rate(&self, queue_entry_id: Uuid, sample_rate: u32) -> Result<(), String> {
        let mut buffers = self.buffers.write().await;
        if let Some(managed) = buffers.get_mut(&queue_entry_id) {
            managed.metadata.source_sample_rate = Some(sample_rate);
            Ok(())
        } else {
            Err(format!("Buffer {} not found", queue_entry_id))
        }
    }

    /// Validate buffer configuration settings
    ///
    /// **[REQ-DEBT-FUNC-002-040]** Validates buffer capacity and headroom
    ///
    /// # Arguments
    /// * `capacity` - Buffer capacity in stereo frames
    /// * `headroom` - Headroom threshold in stereo frames
    ///
    /// # Returns
    /// `Ok(())` if valid, `Err(String)` with error message if invalid
    ///
    /// # Validation Rules
    /// - Capacity must be > 0
    /// - Headroom must be > 0
    /// - Capacity must be > headroom (need space beyond threshold)
    /// - Capacity should be ≥ 2x headroom (recommended margin)
    pub fn validate_buffer_config(capacity: usize, headroom: usize) -> Result<(), String> {
        if capacity == 0 {
            return Err("Buffer capacity must be greater than zero".to_string());
        }

        if headroom == 0 {
            return Err("Buffer headroom must be greater than zero".to_string());
        }

        if capacity <= headroom {
            return Err(format!(
                "Buffer capacity ({}) must be greater than headroom ({})",
                capacity, headroom
            ));
        }

        // Warning (not error) if capacity < 2x headroom
        if capacity < headroom * 2 {
            tracing::warn!(
                "Buffer capacity ({}) is less than 2x headroom ({}). Consider increasing capacity for better stability.",
                capacity, headroom
            );
        }

        Ok(())
    }

    /// Get current buffer capacity
    pub async fn get_buffer_capacity(&self) -> usize {
        *self.buffer_capacity.read().await
    }

    /// Get current buffer headroom
    pub async fn get_buffer_headroom(&self) -> usize {
        *self.buffer_headroom.read().await
    }

    /// Allocate new buffer (Empty state)
    ///
    /// **[DBD-BUF-020]** Buffer starts in Empty state
    /// **[DBD-PARAM-070]** Default capacity: 661,941 samples (15.01s @ 44.1kHz)
    /// **[DBD-PARAM-080]** Default headroom: 4410 samples (0.1s @ 44.1kHz)
    pub async fn allocate_buffer(&self, queue_entry_id: Uuid) -> Arc<PlayoutRingBuffer> {
        let mut buffers = self.buffers.write().await;

        // Check if buffer already exists
        if let Some(managed) = buffers.get(&queue_entry_id) {
            debug!("Buffer already exists for {}, returning existing", queue_entry_id);
            return Arc::clone(&managed.buffer);
        }

        // Create new playout ring buffer with configured settings
        // **[DBD-PARAM-070]** Read capacity from database settings
        // **[DBD-PARAM-080]** Read headroom from database settings
        let capacity = *self.buffer_capacity.read().await;
        let headroom = *self.buffer_headroom.read().await;
        let hysteresis = *self.resume_hysteresis.read().await;
        let buffer_arc = Arc::new(PlayoutRingBuffer::new(
            Some(capacity), // Use configured capacity
            Some(headroom), // Use configured headroom
            Some(hysteresis), // Use configured resume hysteresis
            Some(queue_entry_id),
        ));

        let managed = ManagedBuffer {
            buffer: Arc::clone(&buffer_arc),
            metadata: BufferMetadata::new(),
        };

        buffers.insert(queue_entry_id, managed);
        debug!("Allocated new playout ring buffer for {} (Empty state)", queue_entry_id);

        buffer_arc
    }

    /// Notify samples appended by decoder
    ///
    /// **[DBD-BUF-030]** Transitions: Empty→Filling, Filling→Ready (threshold)
    /// **[PERF-POLL-010]** Emits ReadyForStart event when threshold reached
    pub async fn notify_samples_appended(&self, queue_entry_id: Uuid, count: usize) -> Result<(), String> {
        let mut buffers = self.buffers.write().await;

        let managed = buffers.get_mut(&queue_entry_id)
            .ok_or_else(|| format!("Buffer not found: {}", queue_entry_id))?;

        let old_state = managed.metadata.state;
        let old_write_pos = managed.metadata.write_position;

        // Update write position
        managed.metadata.write_position += count;

        // State transitions
        match old_state {
            BufferState::Empty => {
                // Empty → Filling (first sample written)
                managed.metadata.state = BufferState::Filling;
                managed.metadata.first_sample_at = Some(Instant::now());

                debug!(
                    "Buffer {} transitioned Empty → Filling (first {} samples)",
                    queue_entry_id, count
                );

                self.emit_event(BufferEvent::StateChanged {
                    queue_entry_id,
                    old_state,
                    new_state: BufferState::Filling,
                    samples_buffered: managed.metadata.write_position,
                }).await;

                // Check if threshold already reached (large first append)
                let threshold_samples = self.get_ready_threshold_samples().await;
                if managed.metadata.write_position >= threshold_samples {
                    // Transition to Ready immediately
                    managed.metadata.state = BufferState::Ready;
                    managed.metadata.ready_at = Some(Instant::now());

                    // Calculate buffer duration
                    let buffer_duration_ms = (managed.metadata.write_position as u64 * 1000)
                        / (standard_sample_rate() as u64 * 2); // /2 for stereo

                    debug!(
                        "Buffer {} transitioned Filling → Ready (threshold reached: {} samples, {}ms)",
                        queue_entry_id, managed.metadata.write_position, buffer_duration_ms
                    );

                    // Emit state change
                    self.emit_event(BufferEvent::StateChanged {
                        queue_entry_id,
                        old_state: BufferState::Filling,
                        new_state: BufferState::Ready,
                        samples_buffered: managed.metadata.write_position,
                    }).await;

                    // Emit ReadyForStart
                    managed.metadata.ready_notified = true;
                    self.emit_event(BufferEvent::ReadyForStart {
                        queue_entry_id,
                        samples_buffered: managed.metadata.write_position,
                        buffer_duration_ms,
                    }).await;

                    info!(
                        "⚡ Buffer ready for playback: {} ({}ms buffered)",
                        queue_entry_id, buffer_duration_ms
                    );
                }
            }

            BufferState::Filling => {
                // Check if threshold reached for Filling → Ready
                let threshold_samples = self.get_ready_threshold_samples().await;

                if old_write_pos < threshold_samples && managed.metadata.write_position >= threshold_samples {
                    // Transition to Ready
                    managed.metadata.state = BufferState::Ready;
                    managed.metadata.ready_at = Some(Instant::now());

                    // Calculate buffer duration
                    let buffer_duration_ms = (managed.metadata.write_position as u64 * 1000)
                        / (standard_sample_rate() as u64 * 2); // /2 for stereo

                    debug!(
                        "Buffer {} transitioned Filling → Ready (threshold reached: {} samples, {}ms)",
                        queue_entry_id, managed.metadata.write_position, buffer_duration_ms
                    );

                    // Emit state change
                    self.emit_event(BufferEvent::StateChanged {
                        queue_entry_id,
                        old_state,
                        new_state: BufferState::Ready,
                        samples_buffered: managed.metadata.write_position,
                    }).await;

                    // Emit ReadyForStart (if not already notified)
                    if !managed.metadata.ready_notified {
                        managed.metadata.ready_notified = true;

                        self.emit_event(BufferEvent::ReadyForStart {
                            queue_entry_id,
                            samples_buffered: managed.metadata.write_position,
                            buffer_duration_ms,
                        }).await;

                        info!(
                            "⚡ Buffer ready for playback: {} ({}ms buffered)",
                            queue_entry_id, buffer_duration_ms
                        );
                    }
                }
            }

            BufferState::Ready | BufferState::Playing => {
                // Still filling while ready/playing
                // Check for buffer exhaustion warning
                self.check_buffer_exhaustion(queue_entry_id, &managed.metadata).await;
            }

            BufferState::Finished => {
                // Shouldn't happen (decode already finished)
                warn!(
                    "Samples appended to Finished buffer {} (unexpected)",
                    queue_entry_id
                );
            }
        }

        Ok(())
    }

    /// Get ready threshold in samples (depends on first-passage optimization)
    ///
    /// **[PERF-FIRST-010]** 0.5s for first passage, 3.0s for subsequent
    async fn get_ready_threshold_samples(&self) -> usize {
        let configured_threshold_ms = *self.min_buffer_threshold_ms.read().await;
        let is_first_passage = !self.ever_played.load(Ordering::Relaxed);

        let threshold_ms = if is_first_passage {
            500.min(configured_threshold_ms) // 0.5s or configured, whichever is smaller
        } else {
            configured_threshold_ms
        };

        // Convert ms to frames (stereo @ 44.1kHz)
        // Note: Buffer counts stereo frames, not individual L+R samples
        threshold_ms as usize * standard_sample_rate() as usize / 1000
    }

    /// Push samples to ring buffer (decoder writes)
    ///
    /// **[DBD-BUF-010]** Pushes samples to playout ring buffer
    /// Replaces PassageBuffer::append_samples() with ring buffer push operations
    ///
    /// # Arguments
    /// * `queue_entry_id` - Queue entry to push samples to
    /// * `samples` - Interleaved stereo samples [L, R, L, R, ...]
    ///
    /// # Returns
    /// * `Ok(usize)` - Number of frames pushed successfully
    /// * `Err(String)` - Error if buffer not found or buffer full
    pub async fn push_samples(&self, queue_entry_id: Uuid, samples: &[f32]) -> Result<usize, String> {
        // Get managed buffer
        let buffers = self.buffers.read().await;
        let managed = buffers.get(&queue_entry_id)
            .ok_or_else(|| format!("Buffer not found: {}", queue_entry_id))?;

        let buffer_arc = Arc::clone(&managed.buffer);
        drop(buffers); // Release lock before async operation

        // Convert samples to AudioFrame vector BEFORE acquiring lock
        // This minimizes lock hold time for better mixer concurrency
        let frames: Vec<AudioFrame> = samples
            .chunks_exact(2)
            .map(|chunk| AudioFrame::from_stereo(chunk[0], chunk[1]))
            .collect();

        let total_frames = frames.len();

        // Push frames in batches to allow async yielding
        // Batch size: 220 frames (5ms @ 44.1kHz) provides good granularity
        // PlayoutRingBuffer.push_frame() is lock-free (uses internal mutexes only for ring buffer ops)
        const BATCH_SIZE: usize = 220;
        let mut frames_pushed = 0;

        for batch_start in (0..total_frames).step_by(BATCH_SIZE) {
            let batch_end = (batch_start + BATCH_SIZE).min(total_frames);
            let batch = &frames[batch_start..batch_end];

            // Push batch - no outer lock needed (buffer is lock-free)
            for frame in batch {
                match buffer_arc.push_frame(*frame) {
                    Ok(()) => {
                        frames_pushed += 1;
                    }
                    Err(BufferFullError { capacity, occupied }) => {
                        // Buffer full - decoder should pause
                        debug!(
                            "Ring buffer full for {}: pushed {}/{} frames (capacity={}, occupied={})",
                            queue_entry_id, frames_pushed, total_frames, capacity, occupied
                        );
                        self.notify_samples_appended(queue_entry_id, frames_pushed).await?;
                        return Ok(frames_pushed);
                    }
                }
            }

            // Yield to give mixer thread a chance to run
            tokio::task::yield_now().await;
        }

        // Notify samples appended for state machine transitions
        self.notify_samples_appended(queue_entry_id, frames_pushed).await?;

        Ok(frames_pushed)
    }

    /// Check if decoder should pause (buffer nearly full)
    ///
    /// **[DBD-BUF-050]** Returns true when buffer has ≤ headroom free space
    ///
    /// # Arguments
    /// * `queue_entry_id` - Queue entry to check
    ///
    /// # Returns
    /// * `Ok(true)` - Decoder should pause
    /// * `Ok(false)` - Decoder can continue
    /// * `Err(String)` - Buffer not found
    pub async fn should_decoder_pause(&self, queue_entry_id: Uuid) -> Result<bool, String> {
        let buffers = self.buffers.read().await;
        let managed = buffers.get(&queue_entry_id)
            .ok_or_else(|| format!("Buffer not found: {}", queue_entry_id))?;

        let buffer_arc = Arc::clone(&managed.buffer);
        drop(buffers);

        // No lock needed - buffer methods use &self with atomics
        Ok(buffer_arc.should_decoder_pause())
    }

    /// Check if decoder can resume (hysteresis check)
    ///
    /// **[DBD-BUF-050]** Resume when buffer has room for decoder_resume_hysteresis_samples + headroom
    /// **[DBD-PARAM-085]** decoder_resume_hysteresis_samples (default: 44100)
    /// **[DBD-PARAM-080]** playout_ringbuffer_headroom (default: 4410)
    /// - Pause threshold: free_space ≤ playout_ringbuffer_headroom (4410 samples)
    /// - Resume threshold: free_space ≥ decoder_resume_hysteresis_samples + playout_ringbuffer_headroom (48510 samples)
    /// - Hysteresis gap: decoder_resume_hysteresis_samples (44100 samples)
    ///
    /// Using the sum prevents issues where headroom could be set larger than hysteresis.
    ///
    /// # Arguments
    /// * `queue_entry_id` - Queue entry to check
    ///
    /// # Returns
    /// * `Some(true)` - Decoder can resume
    /// * `Some(false)` - Decoder should stay paused
    /// * `None` - Buffer not found
    pub async fn can_decoder_resume(&self, queue_entry_id: Uuid) -> Option<bool> {
        let buffers = self.buffers.read().await;
        let managed = buffers.get(&queue_entry_id)?;

        let buffer_arc = Arc::clone(&managed.buffer);
        drop(buffers);

        // No lock needed - buffer methods use &self with atomics
        // **[DBD-BUF-050]** Use buffer's configurable hysteresis check
        Some(buffer_arc.can_decoder_resume())
    }

    /// Set discovered endpoint for passage with undefined end_time_ticks
    ///
    /// **[DBD-DEC-095]** Called by decoder when actual file duration is discovered
    /// Emits EndpointDiscovered event for downstream processing
    pub async fn set_discovered_endpoint(&self, queue_entry_id: Uuid, actual_end_ticks: i64) -> Result<(), String> {
        let mut buffers = self.buffers.write().await;

        let managed = buffers.get_mut(&queue_entry_id)
            .ok_or_else(|| format!("Buffer not found: {}", queue_entry_id))?;

        // Store discovered endpoint in metadata
        managed.metadata.discovered_end_ticks = Some(actual_end_ticks);

        debug!(
            "Endpoint discovered for {}: {}ticks ({}ms)",
            queue_entry_id,
            actual_end_ticks,
            wkmp_common::timing::ticks_to_ms(actual_end_ticks)
        );

        // Emit event
        self.emit_event(BufferEvent::EndpointDiscovered {
            queue_entry_id,
            actual_end_ticks,
        }).await;

        Ok(())
    }

    /// Finalize buffer after decode completes
    ///
    /// **[DBD-BUF-060]** Transitions: Filling/Ready/Playing → Finished
    pub async fn finalize_buffer(&self, queue_entry_id: Uuid, total_samples: usize) -> Result<(), String> {
        let mut buffers = self.buffers.write().await;

        let managed = buffers.get_mut(&queue_entry_id)
            .ok_or_else(|| format!("Buffer not found: {}", queue_entry_id))?;

        let old_state = managed.metadata.state;

        // Set total samples
        managed.metadata.total_samples = Some(total_samples);

        // Transition to Finished (if not already)
        if old_state != BufferState::Finished {
            managed.metadata.state = BufferState::Finished;

            debug!(
                "Buffer {} transitioned {:?} → Finished (total_samples={})",
                queue_entry_id, old_state, total_samples
            );

            self.emit_event(BufferEvent::StateChanged {
                queue_entry_id,
                old_state,
                new_state: BufferState::Finished,
                samples_buffered: total_samples,
            }).await;

            self.emit_event(BufferEvent::Finished {
                queue_entry_id,
                total_samples,
            }).await;
        }

        // Mark decode complete on the ring buffer
        let buffer = Arc::clone(&managed.buffer);
        drop(buffers); // Release lock before operation

        // No lock needed - mark_decode_complete uses &self with atomics
        buffer.mark_decode_complete();

        Ok(())
    }

    /// Start playback (mixer starts reading)
    ///
    /// **[DBD-BUF-050]** Transitions: Ready → Playing, Finished → Playing
    ///
    /// Note: Finished state means "decode complete" not "playback complete".
    /// Pre-decoded buffers will be in Finished state when playback starts.
    pub async fn start_playback(&self, queue_entry_id: Uuid) -> Result<(), String> {
        let mut buffers = self.buffers.write().await;

        let managed = buffers.get_mut(&queue_entry_id)
            .ok_or_else(|| format!("Buffer not found: {}", queue_entry_id))?;

        let old_state = managed.metadata.state;

        // Accept both Ready and Finished states
        // Ready = decode in progress, threshold reached
        // Finished = decode complete (pre-decoded buffers)
        if old_state == BufferState::Ready || old_state == BufferState::Finished {
            managed.metadata.state = BufferState::Playing;
            managed.metadata.playing_at = Some(Instant::now());

            // **[PERF-FIRST-010]** Mark that we've played at least one passage
            self.ever_played.store(true, Ordering::Relaxed);

            debug!(
                "Buffer {} transitioned {:?} → Playing",
                queue_entry_id, old_state
            );

            self.emit_event(BufferEvent::StateChanged {
                queue_entry_id,
                old_state,
                new_state: BufferState::Playing,
                samples_buffered: managed.metadata.write_position,
            }).await;
        } else {
            warn!(
                "start_playback called on buffer {} in state {:?} (expected Ready or Finished)",
                queue_entry_id, old_state
            );
        }

        Ok(())
    }

    /// Mixer reads samples (advance read position)
    ///
    /// **[DBD-BUF-070]** Checks for buffer exhaustion
    pub async fn advance_read_position(&self, queue_entry_id: Uuid, count: usize) -> Result<(), String> {
        let mut buffers = self.buffers.write().await;

        let managed = buffers.get_mut(&queue_entry_id)
            .ok_or_else(|| format!("Buffer not found: {}", queue_entry_id))?;

        managed.metadata.read_position += count;

        // Check for buffer exhaustion
        self.check_buffer_exhaustion(queue_entry_id, &managed.metadata).await;

        Ok(())
    }

    /// Check headroom (available samples)
    ///
    /// Returns: write_position - read_position
    pub async fn get_headroom(&self, queue_entry_id: Uuid) -> Result<usize, String> {
        let buffers = self.buffers.read().await;

        let managed = buffers.get(&queue_entry_id)
            .ok_or_else(|| format!("Buffer not found: {}", queue_entry_id))?;

        Ok(managed.metadata.headroom())
    }

    /// Check buffer exhaustion (mixer reading faster than decoder writing)
    ///
    /// **[DBD-BUF-070]** Buffer exhaustion detection
    /// **[DBD-BUF-080]** Underrun recovery trigger
    async fn check_buffer_exhaustion(&self, queue_entry_id: Uuid, metadata: &BufferMetadata) {
        if metadata.state != BufferState::Playing {
            return; // Only check during playback
        }

        let headroom = metadata.headroom();

        // **[DBD-PARAM-080]** buffer_headroom_threshold = 220,500 samples (5s @ 44.1kHz stereo)
        if headroom < BUFFER_HEADROOM_THRESHOLD && metadata.total_samples.is_none() {
            // Decode still running but headroom low
            warn!(
                "⚠️  Buffer exhaustion detected for {}: {} samples remaining (threshold: {})",
                queue_entry_id, headroom, BUFFER_HEADROOM_THRESHOLD
            );

            self.emit_event(BufferEvent::Exhausted {
                queue_entry_id,
                headroom,
            }).await;
        }
    }

    /// Emit buffer event to channel
    async fn emit_event(&self, event: BufferEvent) {
        let event_tx = self.event_tx.read().await;
        if let Some(ref tx) = *event_tx {
            if let Err(e) = tx.send(event) {
                debug!("Failed to send buffer event: {}", e);
            }
        }
    }

    /// Get buffer for playback (returns Arc to PlayoutRingBuffer)
    pub async fn get_buffer(&self, queue_entry_id: Uuid) -> Option<Arc<PlayoutRingBuffer>> {
        let buffers = self.buffers.read().await;
        buffers.get(&queue_entry_id).map(|m| Arc::clone(&m.buffer))
    }

    /// Get buffer state
    pub async fn get_state(&self, queue_entry_id: Uuid) -> Option<BufferState> {
        let buffers = self.buffers.read().await;
        buffers.get(&queue_entry_id).map(|m| m.metadata.state)
    }

    /// Check if buffer is ready for playback
    pub async fn is_ready(&self, queue_entry_id: Uuid) -> bool {
        let buffers = self.buffers.read().await;
        buffers.get(&queue_entry_id)
            .map(|m| matches!(m.metadata.state, BufferState::Ready | BufferState::Playing))
            .unwrap_or(false)
    }

    /// Check if buffer is managed (exists in any state)
    pub async fn is_managed(&self, queue_entry_id: Uuid) -> bool {
        let buffers = self.buffers.read().await;
        buffers.contains_key(&queue_entry_id)
    }

    /// Remove buffer (cleanup)
    ///
    /// **[DBD-BUF-010]** Clears ring buffer before removal
    pub async fn remove(&self, queue_entry_id: Uuid) -> bool {
        let mut buffers = self.buffers.write().await;

        // Get buffer before removing
        if let Some(managed) = buffers.get(&queue_entry_id) {
            let buffer_arc = Arc::clone(&managed.buffer);

            // Clear ring buffer (no lock needed - reset uses &self)
            buffer_arc.reset();

            // Remove from map
            buffers.remove(&queue_entry_id);
            true
        } else {
            false
        }
    }

    /// Clear all buffers
    pub async fn clear(&self) {
        let mut buffers = self.buffers.write().await;
        buffers.clear();
    }

    // ==================== Legacy API Compatibility ====================
    // These methods provide compatibility with existing code during Phase 4C
    // They map to the new state machine architecture

    /// Register decoding (legacy API - maps to allocate_buffer)
    pub async fn register_decoding(&self, queue_entry_id: Uuid) -> Arc<PlayoutRingBuffer> {
        self.allocate_buffer(queue_entry_id).await
    }

    /// Mark ready (legacy API - now handled automatically by notify_samples_appended)
    pub async fn mark_ready(&self, queue_entry_id: Uuid) {
        // No-op: state transitions happen automatically via notify_samples_appended
        debug!("mark_ready({}) - no-op (handled by state machine)", queue_entry_id);
    }

    /// Mark playing (legacy API - maps to start_playback)
    pub async fn mark_playing(&self, queue_entry_id: Uuid) {
        if let Err(e) = self.start_playback(queue_entry_id).await {
            warn!("mark_playing failed: {}", e);
        }
    }

    /// Update decode progress (legacy API - for progress events only)
    pub async fn update_decode_progress(&self, _queue_entry_id: Uuid, _progress_percent: u8) {
        // No-op: progress tracking removed in favor of event-driven architecture
    }

    /// Mark exhausted (legacy API - no longer needed with state machine)
    pub async fn mark_exhausted(&self, queue_entry_id: Uuid) {
        debug!("mark_exhausted({}) - legacy no-op (state machine handles this)", queue_entry_id);
    }

    /// Get all buffer statuses (legacy API - returns simplified state map)
    pub async fn get_all_statuses(&self) -> std::collections::HashMap<Uuid, crate::audio::types::BufferStatus> {
        use crate::audio::types::BufferStatus;
        let buffers = self.buffers.read().await;

        buffers.iter().map(|(id, managed)| {
            let status = match managed.metadata.state {
                BufferState::Empty | BufferState::Filling => BufferStatus::Decoding { progress_percent: 0 },
                BufferState::Ready => BufferStatus::Ready,
                BufferState::Playing => BufferStatus::Playing,
                BufferState::Finished => BufferStatus::Exhausted,
            };
            (*id, status)
        }).collect()
    }

    /// Get buffer status (legacy API - maps to get_state)
    pub async fn get_status(&self, queue_entry_id: Uuid) -> Option<crate::audio::types::BufferStatus> {
        use crate::audio::types::BufferStatus;
        let state = self.get_state(queue_entry_id).await?;

        Some(match state {
            BufferState::Empty | BufferState::Filling => BufferStatus::Decoding { progress_percent: 0 },
            BufferState::Ready => BufferStatus::Ready,
            BufferState::Playing => BufferStatus::Playing,
            BufferState::Finished => BufferStatus::Exhausted,
        })
    }

    /// Check if buffer has minimum playback buffer available (legacy API)
    pub async fn has_minimum_playback_buffer(&self, queue_entry_id: Uuid, min_duration_ms: u64) -> bool {
        // Get buffer and check duration based on occupied frames
        if let Some(buffer_arc) = self.get_buffer(queue_entry_id).await {
            // No lock needed - occupied() uses atomics
            let occupied_frames = buffer_arc.occupied();
            let available_ms = (occupied_frames as u64 * 1000) / standard_sample_rate() as u64;
            available_ms >= min_duration_ms
        } else {
            false
        }
    }

    /// Get decode elapsed time (legacy API - returns time since creation)
    pub async fn get_decode_elapsed(&self, queue_entry_id: Uuid) -> Option<std::time::Duration> {
        let buffers = self.buffers.read().await;
        buffers.get(&queue_entry_id)
            .map(|managed| managed.metadata.created_at.elapsed())
    }

    /// Pop (drain) a frame from ring buffer
    ///
    /// **[DBD-BUF-040]** Returns last valid frame if buffer empty
    pub async fn pop_frame(&self, queue_entry_id: Uuid) -> Option<AudioFrame> {
        let buffers = self.buffers.read().await;
        let managed = buffers.get(&queue_entry_id)?;

        let buffer_arc = Arc::clone(&managed.buffer);
        drop(buffers); // Release lock before operation

        // No lock needed - pop_frame uses &self with internal locks
        // pop_frame returns Result - convert to Option
        match buffer_arc.pop_frame() {
            Ok(frame) => Some(frame),
            Err(err) => {
                // Buffer empty - return last frame from error
                Some(err.last_frame)
            }
        }
    }

    /// Check if buffer is exhausted (decode complete AND drained to 0)
    ///
    /// **[DBD-BUF-060]** Returns true when passage end reached and all samples consumed
    pub async fn is_buffer_exhausted(&self, queue_entry_id: Uuid) -> Option<bool> {
        let buffers = self.buffers.read().await;
        let managed = buffers.get(&queue_entry_id)?;

        let buffer_arc = Arc::clone(&managed.buffer);
        drop(buffers);

        // No lock needed - is_exhausted uses atomics
        Some(buffer_arc.is_exhausted())
    }

    /// Get buffer monitoring info for developer UI
    ///
    /// **[DBD-BUF-010]** Returns ring buffer fill percentage and capacity
    pub async fn get_buffer_info(&self, queue_entry_id: Uuid) -> Option<BufferMonitorInfo> {
        let buffers = self.buffers.read().await;
        let managed = buffers.get(&queue_entry_id)?;

        let buffer_arc = Arc::clone(&managed.buffer);
        let metadata = &managed.metadata;

        // Calculate decode duration from telemetry timestamps
        let decode_duration_ms = if let (Some(started), Some(completed)) =
            (metadata.decode_started_at, metadata.decode_completed_at) {
            Some(completed.duration_since(started).as_millis() as u64)
        } else {
            None
        };

        let file_path = metadata.file_path.clone();
        let source_sample_rate = metadata.source_sample_rate; // **[DEBT-007]** Copy before dropping lock

        drop(buffers);

        // No lock needed - all buffer methods use &self with atomics
        let capacity = buffer_arc.capacity();
        let occupied = buffer_arc.occupied();
        let fill_percent = buffer_arc.fill_percent();
        let stats = buffer_arc.get_statistics();

        Some(BufferMonitorInfo {
            fill_percent,
            samples_buffered: occupied,
            capacity_samples: capacity,
            duration_ms: if occupied > 0 {
                Some((occupied as u64 * 1000) / standard_sample_rate() as u64)
            } else {
                None
            },
            total_decoded_frames: (stats.total_samples_written / 2) as usize, // Convert samples to frames
            decode_duration_ms,
            file_path,
            source_sample_rate, // **[DEBT-007]** Propagate from decoder
        })
    }

    /// Get buffer state for monitoring (**[DBD-BUF-020]** through **[DBD-BUF-060]**)
    pub async fn get_buffer_state(&self, queue_entry_id: Uuid) -> Option<BufferState> {
        let buffers = self.buffers.read().await;
        buffers.get(&queue_entry_id).map(|managed| managed.metadata.state)
    }

    /// Get all buffer statistics for pipeline integrity validation
    ///
    /// **[PHASE1-INTEGRITY]** Returns HashMap of passage_id -> BufferStatistics
    ///
    /// # Returns
    /// HashMap with statistics for all currently managed buffers
    pub async fn get_all_buffer_statistics(&self) -> HashMap<Uuid, crate::playback::playout_ring_buffer::BufferStatistics> {
        let buffers = self.buffers.read().await;
        let mut stats_map = HashMap::new();

        for (queue_entry_id, managed) in buffers.iter() {
            let buffer_stats = managed.buffer.get_statistics();
            stats_map.insert(*queue_entry_id, buffer_stats);
        }

        stats_map
    }
}

/// Buffer monitoring information
#[derive(Debug, Clone)]
pub struct BufferMonitorInfo {
    pub fill_percent: f32,
    pub samples_buffered: usize,
    pub capacity_samples: usize,
    pub duration_ms: Option<u64>,
    pub total_decoded_frames: usize,

    // Decoder telemetry **[REQ-DEBT-FUNC-001]** **[DEBT-007]**
    pub decode_duration_ms: Option<u64>,
    pub file_path: Option<String>,
    pub source_sample_rate: Option<u32>, // **[DEBT-007]** Actual source file sample rate
}

impl Default for BufferManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_buffer_manager_creation() {
        let manager = BufferManager::new();
        assert!(!manager.is_managed(Uuid::new_v4()).await);
    }

    #[tokio::test]
    async fn test_allocate_buffer_empty_state() {
        let manager = BufferManager::new();
        let queue_entry_id = Uuid::new_v4();

        let _buffer = manager.allocate_buffer(queue_entry_id).await;

        let state = manager.get_state(queue_entry_id).await.unwrap();
        assert_eq!(state, BufferState::Empty);
    }

    #[tokio::test]
    async fn test_buffer_state_transitions() {
        let manager = BufferManager::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        manager.set_event_channel(tx).await;
        manager.set_min_buffer_threshold(500).await; // 500ms threshold

        let queue_entry_id = Uuid::new_v4();
        let _buffer = manager.allocate_buffer(queue_entry_id).await;

        // Empty state initially
        assert_eq!(manager.get_state(queue_entry_id).await.unwrap(), BufferState::Empty);

        // Empty → Filling (first samples)
        manager.notify_samples_appended(queue_entry_id, 1000).await.unwrap();
        assert_eq!(manager.get_state(queue_entry_id).await.unwrap(), BufferState::Filling);

        // Filling → Ready (threshold reached: 500ms = 22,050 samples @ 44.1kHz stereo)
        manager.notify_samples_appended(queue_entry_id, 22_050).await.unwrap();
        assert_eq!(manager.get_state(queue_entry_id).await.unwrap(), BufferState::Ready);

        // Ready → Playing
        manager.start_playback(queue_entry_id).await.unwrap();
        assert_eq!(manager.get_state(queue_entry_id).await.unwrap(), BufferState::Playing);

        // Playing → Finished
        manager.finalize_buffer(queue_entry_id, 100_000).await.unwrap();
        assert_eq!(manager.get_state(queue_entry_id).await.unwrap(), BufferState::Finished);
    }

    #[tokio::test]
    async fn test_ready_threshold_detection() {
        let manager = BufferManager::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        manager.set_event_channel(tx).await;
        manager.set_min_buffer_threshold(3000).await; // 3 seconds

        // Disable first-passage optimization
        manager.ever_played.store(true, Ordering::Relaxed);

        let queue_entry_id = Uuid::new_v4();
        let _buffer = manager.allocate_buffer(queue_entry_id).await;

        // Append below threshold (2.9s = 128,520 samples)
        manager.notify_samples_appended(queue_entry_id, 128_520).await.unwrap();
        assert_eq!(manager.get_state(queue_entry_id).await.unwrap(), BufferState::Filling);

        // Append to reach threshold (3.0s = 132,300 samples total)
        manager.notify_samples_appended(queue_entry_id, 3_780).await.unwrap();
        assert_eq!(manager.get_state(queue_entry_id).await.unwrap(), BufferState::Ready);

        // Verify ReadyForStart event emitted
        let event = rx.try_recv().ok();
        assert!(event.is_some(), "ReadyForStart event should be emitted");
    }

    #[tokio::test]
    async fn test_headroom_calculation() {
        let manager = BufferManager::new();
        let queue_entry_id = Uuid::new_v4();
        let _buffer = manager.allocate_buffer(queue_entry_id).await;

        // Write 10,000 samples
        manager.notify_samples_appended(queue_entry_id, 10_000).await.unwrap();

        // Read 3,000 samples
        manager.advance_read_position(queue_entry_id, 3_000).await.unwrap();

        // Headroom should be 7,000
        let headroom = manager.get_headroom(queue_entry_id).await.unwrap();
        assert_eq!(headroom, 7_000);
    }

    #[tokio::test]
    async fn test_event_deduplication() {
        let manager = BufferManager::new();
        let (tx, mut rx) = mpsc::unbounded_channel();
        manager.set_event_channel(tx).await;
        manager.set_min_buffer_threshold(500).await;

        let queue_entry_id = Uuid::new_v4();
        let _buffer = manager.allocate_buffer(queue_entry_id).await;

        // First time reaching threshold - should emit
        manager.notify_samples_appended(queue_entry_id, 22_050).await.unwrap();

        // Count ReadyForStart events
        let mut ready_count = 0;
        while let Ok(event) = rx.try_recv() {
            if matches!(event, BufferEvent::ReadyForStart { .. }) {
                ready_count += 1;
            }
        }
        assert_eq!(ready_count, 1, "Should emit ReadyForStart exactly once");

        // Append more samples - should NOT emit again
        manager.notify_samples_appended(queue_entry_id, 22_050).await.unwrap();

        let second_ready_count = rx.try_recv()
            .ok()
            .filter(|e| matches!(e, BufferEvent::ReadyForStart { .. }))
            .map(|_| 1)
            .unwrap_or(0);

        assert_eq!(second_ready_count, 0, "Should not emit duplicate ReadyForStart");
    }

    #[tokio::test]
    async fn test_first_passage_optimization() {
        let manager = BufferManager::new();
        manager.set_min_buffer_threshold(3000).await; // 3s normal threshold

        // First passage should use 500ms threshold = 22,050 samples
        let threshold = manager.get_ready_threshold_samples().await;
        assert_eq!(threshold, 22_050, "First passage should use 500ms threshold");

        // After playing, should use configured threshold = 132,300 samples
        manager.ever_played.store(true, Ordering::Relaxed);
        let threshold2 = manager.get_ready_threshold_samples().await;
        assert_eq!(threshold2, 132_300, "Subsequent passages should use 3s threshold");
    }

    #[tokio::test]
    async fn test_remove_buffer() {
        let manager = BufferManager::new();
        let queue_entry_id = Uuid::new_v4();

        let _buffer = manager.allocate_buffer(queue_entry_id).await;
        assert!(manager.is_managed(queue_entry_id).await);

        let removed = manager.remove(queue_entry_id).await;
        assert!(removed);
        assert!(!manager.is_managed(queue_entry_id).await);
    }

    #[tokio::test]
    async fn test_clear_all_buffers() {
        let manager = BufferManager::new();
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        manager.allocate_buffer(id1).await;
        manager.allocate_buffer(id2).await;

        assert!(manager.is_managed(id1).await);
        assert!(manager.is_managed(id2).await);

        manager.clear().await;

        assert!(!manager.is_managed(id1).await);
        assert!(!manager.is_managed(id2).await);
    }

    /// **[DBD-BUF-020]** Test get_buffer_state() exposes buffer state for monitoring
    #[tokio::test]
    async fn test_get_buffer_state() {
        let manager = BufferManager::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        manager.set_event_channel(tx).await;
        manager.set_min_buffer_threshold(500).await;

        let queue_entry_id = Uuid::new_v4();

        // No buffer yet - should return None
        assert_eq!(manager.get_buffer_state(queue_entry_id).await, None);

        // Allocate buffer - should be Empty
        let _buffer = manager.allocate_buffer(queue_entry_id).await;
        assert_eq!(
            manager.get_buffer_state(queue_entry_id).await,
            Some(BufferState::Empty)
        );

        // Append samples - should transition to Filling
        manager.notify_samples_appended(queue_entry_id, 1000).await.unwrap();
        assert_eq!(
            manager.get_buffer_state(queue_entry_id).await,
            Some(BufferState::Filling)
        );

        // Reach threshold - should transition to Ready
        manager.notify_samples_appended(queue_entry_id, 22_050).await.unwrap();
        assert_eq!(
            manager.get_buffer_state(queue_entry_id).await,
            Some(BufferState::Ready)
        );

        // Start playback - should transition to Playing
        manager.start_playback(queue_entry_id).await.unwrap();
        assert_eq!(
            manager.get_buffer_state(queue_entry_id).await,
            Some(BufferState::Playing)
        );

        // Finalize decode - should transition to Finished
        manager.finalize_buffer(queue_entry_id, 100_000).await.unwrap();
        assert_eq!(
            manager.get_buffer_state(queue_entry_id).await,
            Some(BufferState::Finished)
        );

        // Remove buffer - should return None
        manager.remove(queue_entry_id).await;
        assert_eq!(manager.get_buffer_state(queue_entry_id).await, None);
    }
}
