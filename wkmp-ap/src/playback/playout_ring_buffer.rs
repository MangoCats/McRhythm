//! Playout Ring Buffer for Per-Chain Audio Buffering
//!
//! **Traceability:**
//! - [DBD-BUF-010] Fixed-capacity ring buffer holds playout_ringbuffer_size stereo samples
//! - [DBD-BUF-020] Ring buffer starts empty when assigned to passage
//! - [DBD-BUF-030] Returns last sample on empty buffer read (underrun handling)
//! - [DBD-BUF-040] Returns buffer empty status when mixer attempts read from empty buffer
//! - [DBD-BUF-050] Decoder pauses when buffer has ≤ headroom samples free
//! - [DBD-BUF-060] Buffer signals completion when last sample removed
//! - [DBD-PARAM-070] Default capacity: 661,941 samples (15.01s @ 44.1kHz)
//! - [DBD-PARAM-080] Default headroom: 4410 samples (0.1s @ 44.1kHz)
//! - [CO-104] Requirement traceability in code comments
//!
//! This module provides a lock-free ring buffer for per-chain audio playout.
//! Each decoder-buffer chain has its own PlayoutRingBuffer instance that:
//! - Starts empty (0% fill) when assigned to a passage
//! - Fills progressively as decoder writes samples (0% → ~99%)
//! - Signals decoder to pause when nearly full (capacity - headroom)
//! - Drains as mixer consumes samples during playback
//! - Signals exhaustion when decode complete AND buffer empty
//!
//! ## Design
//!
//! ```text
//! Decoder → Resampler → Fade Unit → push_frame()
//!                                         ↓
//!                                   PlayoutRingBuffer (per chain)
//!                                   - Capacity: 661,941 frames
//!                                   - Starts: 0%
//!                                   - Fills: → ~99%
//!                                   - Drains: → 0%
//!                                         ↓
//!                                    pop_frame()
//!                                         ↓
//!                                      Mixer
//! ```

use crate::audio::types::AudioFrame;
use ringbuf::{traits::*, HeapRb, HeapProd, HeapCons};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;
use thiserror::Error;
use tracing::{debug, trace};
use uuid::Uuid;

/// **[DBD-PARAM-070]** Default playout ring buffer capacity (661941 samples = 15.01s @ 44.1kHz)
fn default_capacity() -> usize {
    *wkmp_common::params::PARAMS.playout_ringbuffer_size.read().unwrap()
}

/// **[DBD-PARAM-080]** Default buffer headroom (4410 samples = 0.1s @ 44.1kHz)
fn default_headroom() -> usize {
    *wkmp_common::params::PARAMS.playout_ringbuffer_headroom.read().unwrap()
}

/// **[DBD-PARAM-085]** Default decoder resume hysteresis (44100 samples = 1.0s @ 44.1kHz)
fn default_resume_hysteresis() -> usize {
    *wkmp_common::params::PARAMS.decoder_resume_hysteresis_samples.read().unwrap() as usize
}

/// Error returned when attempting to push to a full buffer
#[derive(Debug, Error)]
#[error("Buffer full: cannot push frame (capacity: {capacity}, occupied: {occupied})")]
pub struct BufferFullError {
    pub capacity: usize,
    pub occupied: usize,
}

/// Error returned when attempting to pop from an empty buffer
#[derive(Debug, Error)]
#[error("Buffer empty: cannot pop frame")]
pub struct BufferEmptyError {
    /// Last valid frame returned (for underrun mitigation)
    /// **[DBD-BUF-030][DBD-BUF-040]** Return last sample on empty read
    pub last_frame: AudioFrame,
}

/// Buffer statistics for pipeline integrity validation
///
/// **[PHASE1-INTEGRITY]** Snapshot of buffer counters for diagnostics
#[derive(Debug, Clone, Copy)]
pub struct BufferStatistics {
    /// Total samples written to buffer (frames × 2 channels)
    pub total_samples_written: u64,

    /// Total samples read from buffer (frames × 2 channels)
    pub total_samples_read: u64,

    /// Associated passage ID
    pub passage_id: Option<Uuid>,
}

/// Playout ring buffer for per-chain audio buffering
///
/// **[DBD-BUF-010]** Holds playout_ringbuffer_size stereo samples in a lock-free ring buffer.
///
/// This ring buffer uses lock-free atomics for thread-safe coordination between:
/// - Decoder thread (producer): Pushes decoded/resampled/faded frames
/// - Mixer thread (consumer): Pops frames for playback
///
/// ## Thread Safety
///
/// **Lock-Free Design:**
/// The ring buffer is split into Producer (decoder) and Consumer (mixer) at construction.
/// - Producer handle (`prod`) is protected by Mutex for safe mutable access by decoder thread
/// - Consumer handle (`cons`) is protected by Mutex for safe mutable access by mixer thread
/// - Atomics used for coordination: fill_level, decoder_should_pause, decode_complete, counters
/// - last_frame stored as two AtomicU64 values (left + right channels as f32→u64 bit-cast)
///
/// **Memory Ordering:**
/// - Statistics (fill_level, counters): Relaxed ordering (exact value not critical)
/// - Coordination flags (decoder_should_pause, decode_complete): Acquire/Release for synchronization
/// - last_frame: Relaxed ordering (stale value acceptable for underrun mitigation)
pub struct PlayoutRingBuffer {
    /// Lock-free ring buffer producer (decoder writes)
    /// Protected by Mutex since push_frame requires mutable access
    /// Mutex is necessary because HeapProd::try_push requires &mut self
    prod: Mutex<HeapProd<AudioFrame>>,

    /// Lock-free ring buffer consumer (mixer reads)
    /// Protected by Mutex since pop_frame requires mutable access
    /// Mutex is necessary because HeapCons::try_pop requires &mut self
    cons: Mutex<HeapCons<AudioFrame>>,

    /// Current fill level in frames (updated atomically)
    /// Used for fill_percent() calculation and monitoring
    /// Ordering: Relaxed (statistics only, exact value not critical)
    fill_level: AtomicUsize,

    /// Total capacity in frames (fixed at construction)
    capacity: usize,

    /// Headroom threshold in frames (fixed at construction)
    /// **[DBD-BUF-050]** Decoder pauses when free space ≤ headroom
    headroom: usize,

    /// Resume hysteresis threshold in frames (fixed at construction)
    /// **[DBD-BUF-050]** Decoder resumes when free space ≥ resume_hysteresis
    /// Default: 44100 samples (1.0 second @ 44.1kHz)
    resume_hysteresis: usize,

    /// Passage ID this buffer is assigned to (immutable after creation)
    passage_id: Option<Uuid>,

    /// Flag: decoder should pause (buffer nearly full)
    /// **[DBD-BUF-050]** Set when fill_level >= (capacity - headroom)
    /// Ordering: Relaxed for read (fast check), Relaxed for write (best-effort coordination)
    decoder_should_pause: AtomicBool,

    /// Flag: passage decoding is complete (no more samples to decode)
    /// **[DBD-BUF-060]** Set by decoder when end-of-passage reached
    /// Ordering: Release on write (decoder), Acquire on read (mixer)
    decode_complete: AtomicBool,

    /// Total frames written since buffer creation (monotonic counter)
    /// Ordering: Relaxed (statistics only)
    total_frames_written: AtomicU64,

    /// Total frames read since buffer creation (monotonic counter)
    /// Ordering: Relaxed (statistics only)
    total_frames_read: AtomicU64,

    /// Last valid frame - left channel (for underrun mitigation)
    /// **[DBD-BUF-030]** Cached for return on empty buffer read
    /// Stored as u64 bit-cast of f32 value
    /// Ordering: Relaxed (stale value acceptable for underrun mitigation)
    last_frame_left: AtomicU64,

    /// Last valid frame - right channel (for underrun mitigation)
    /// Stored as u64 bit-cast of f32 value
    /// Ordering: Relaxed (stale value acceptable for underrun mitigation)
    last_frame_right: AtomicU64,
}

impl std::fmt::Debug for PlayoutRingBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlayoutRingBuffer")
            .field("capacity", &self.capacity)
            .field("headroom", &self.headroom)
            .field("resume_hysteresis", &self.resume_hysteresis)
            .field("passage_id", &self.passage_id)
            .field("fill_level", &self.fill_level.load(Ordering::Relaxed))
            .field("fill_percent", &self.fill_percent())
            .field("decode_complete", &self.decode_complete.load(Ordering::Relaxed))
            .field("decoder_should_pause", &self.decoder_should_pause.load(Ordering::Relaxed))
            .finish()
    }
}

impl PlayoutRingBuffer {
    /// Create a new playout ring buffer
    ///
    /// **[DBD-BUF-020]** Ring buffer starts empty when assigned to passage.
    ///
    /// # Arguments
    /// * `capacity` - Buffer capacity in stereo frames (default: 661,941)
    /// * `headroom` - Headroom threshold in frames (default: 4410)
    /// * `resume_hysteresis` - Resume threshold in frames (default: 44,100)
    /// * `passage_id` - Optional passage UUID this buffer is assigned to
    ///
    /// # Returns
    /// A new empty playout ring buffer ready for decoder push operations
    pub fn new(
        capacity: Option<usize>,
        headroom: Option<usize>,
        resume_hysteresis: Option<usize>,
        passage_id: Option<Uuid>,
    ) -> Self {
        let capacity = capacity.unwrap_or_else(default_capacity);
        let headroom = headroom.unwrap_or_else(default_headroom);
        let resume_hysteresis = resume_hysteresis.unwrap_or_else(default_resume_hysteresis);

        debug!(
            "Creating playout ring buffer: capacity={} frames ({:.2}s @ 44.1kHz), headroom={} frames, resume_hysteresis={} frames, passage_id={:?}",
            capacity,
            capacity as f64 / 44100.0,
            headroom,
            resume_hysteresis,
            passage_id
        );

        // Create ring buffer and split into producer/consumer
        let rb = HeapRb::<AudioFrame>::new(capacity);
        let (prod, cons) = rb.split();

        // Initialize last_frame atomics with zero frame (silence)
        // f32::to_bits() converts f32 to u32, then cast to u64
        let zero_bits = 0.0f32.to_bits() as u64;

        Self {
            prod: Mutex::new(prod),
            cons: Mutex::new(cons),
            fill_level: AtomicUsize::new(0),
            capacity,
            headroom,
            resume_hysteresis,
            passage_id,
            decoder_should_pause: AtomicBool::new(false),
            decode_complete: AtomicBool::new(false),
            total_frames_written: AtomicU64::new(0),
            total_frames_read: AtomicU64::new(0),
            last_frame_left: AtomicU64::new(zero_bits),
            last_frame_right: AtomicU64::new(zero_bits),
        }
    }

    /// Push a stereo frame to the buffer (decoder writes)
    ///
    /// # Arguments
    /// * `frame` - Stereo audio frame to push
    ///
    /// # Returns
    /// * `Ok(())` - Frame successfully pushed
    /// * `Err(BufferFullError)` - Buffer is full, cannot accept more frames
    ///
    /// # Thread Safety
    /// Uses &self (not &mut self) for lock-free coordination.
    /// Producer Mutex acquired only for the ring buffer push operation.
    ///
    /// # Side Effects
    /// - Updates fill_level counter (atomic)
    /// - Sets decoder_should_pause flag when threshold reached (atomic)
    /// - Updates last_frame atomics for underrun mitigation (atomic)
    pub fn push_frame(&self, frame: AudioFrame) -> Result<(), BufferFullError> {
        // **[DBD-BUF-010]** Try to push to ring buffer
        // Acquire producer lock only for this operation
        let mut prod = self.prod.lock().unwrap();

        if prod.try_push(frame).is_ok() {
            // Release lock immediately
            drop(prod);

            // Update fill level (Relaxed: statistics only)
            let new_level = self.fill_level.fetch_add(1, Ordering::Relaxed) + 1;
            self.total_frames_written.fetch_add(1, Ordering::Relaxed);

            // Cache last frame for underrun mitigation [DBD-BUF-030]
            // Store as bit-casted u64 atomics (Relaxed: stale value acceptable)
            self.last_frame_left.store(frame.left.to_bits() as u64, Ordering::Relaxed);
            self.last_frame_right.store(frame.right.to_bits() as u64, Ordering::Relaxed);

            // **[DBD-BUF-050]** Check if decoder should pause (buffer nearly full)
            let free_space = self.capacity.saturating_sub(new_level);
            if free_space <= self.headroom && !self.decoder_should_pause.load(Ordering::Relaxed) {
                // Use Release ordering to ensure buffer writes visible to decoder
                self.decoder_should_pause.store(true, Ordering::Release);
                trace!(
                    "Playout buffer pause threshold reached: fill={}/{} ({:.1}%), free={}, headroom={}",
                    new_level,
                    self.capacity,
                    self.fill_percent(),
                    free_space,
                    self.headroom
                );
            }

            Ok(())
        } else {
            // Buffer full - cannot push
            let occupied = prod.occupied_len();
            drop(prod);

            Err(BufferFullError {
                capacity: self.capacity,
                occupied,
            })
        }
    }

    /// Pop a stereo frame from the buffer (mixer reads)
    ///
    /// **[DBD-BUF-040]** Returns error with last valid frame when buffer is empty.
    ///
    /// # Returns
    /// * `Ok(AudioFrame)` - Frame successfully popped from buffer
    /// * `Err(BufferEmptyError)` - Buffer is empty (underrun)
    ///   - Error contains last valid frame for underrun mitigation [DBD-BUF-030]
    ///
    /// # Thread Safety
    /// Uses &self (not &mut self) for lock-free coordination.
    /// Consumer Mutex acquired only for the ring buffer pop operation.
    ///
    /// # Side Effects
    /// - Updates fill_level counter (atomic)
    /// - Clears decoder_should_pause flag when space available (atomic)
    /// - Does NOT modify last_frame atomics (preserved for next underrun)
    pub fn pop_frame(&self) -> Result<AudioFrame, BufferEmptyError> {
        // Acquire consumer lock only for this operation
        let mut cons = self.cons.lock().unwrap();

        if let Some(frame) = cons.try_pop() {
            // Release lock immediately
            drop(cons);

            // Update fill level (Relaxed: statistics only)
            let new_level = self.fill_level.fetch_sub(1, Ordering::Relaxed).saturating_sub(1);
            self.total_frames_read.fetch_add(1, Ordering::Relaxed);

            // **[DBD-BUF-050]** **[DBD-PARAM-085]** Clear pause flag if buffer has space again
            // Pause when: free_space ≤ headroom (4410)
            // Resume when: free_space ≥ resume_hysteresis + headroom (48510)
            let free_space = self.capacity.saturating_sub(new_level);
            let resume_threshold = self.resume_hysteresis.saturating_add(self.headroom);
            if free_space >= resume_threshold && self.decoder_should_pause.load(Ordering::Relaxed) {
                // Use Release ordering to ensure buffer state visible to decoder
                self.decoder_should_pause.store(false, Ordering::Release);
                trace!(
                    "Playout buffer resume threshold reached: fill={}/{} ({:.1}%), free={}, resume_threshold={}",
                    new_level,
                    self.capacity,
                    self.fill_percent(),
                    free_space,
                    resume_threshold
                );
            }

            Ok(frame)
        } else {
            // Release lock
            drop(cons);

            // **[DBD-BUF-030][DBD-BUF-040]** Buffer empty - return last valid frame in error
            // Load from atomics and convert back to f32 (Relaxed: stale value acceptable)
            let left_bits = self.last_frame_left.load(Ordering::Relaxed) as u32;
            let right_bits = self.last_frame_right.load(Ordering::Relaxed) as u32;
            let last_frame = AudioFrame {
                left: f32::from_bits(left_bits),
                right: f32::from_bits(right_bits),
            };

            Err(BufferEmptyError { last_frame })
        }
    }

    /// Pop and discard a frame for seek/skip operations (does NOT increment read counter)
    ///
    /// **[PLAN022]** Skip forward validation accounting fix
    ///
    /// Similar to `pop_frame()` but does NOT increment `total_frames_read` counter.
    /// Used for seek/skip operations where frames are discarded without being processed by mixer.
    ///
    /// This prevents validation accounting mismatch where:
    /// - `total_frames_read` would include skipped frames
    /// - `mixer_frames_mixed` would NOT include skipped frames
    /// - Validation expects: `total_frames_read / 2 ≈ mixer_frames_mixed`
    ///
    /// # Returns
    /// - `Ok(())` - Frame discarded successfully
    /// - `Err(BufferEmptyError)` - Buffer empty (same as `pop_frame()`)
    ///
    /// # Side Effects
    /// - Updates fill_level counter (atomic) - SAME as pop_frame()
    /// - Clears decoder_should_pause flag when space available (atomic) - SAME as pop_frame()
    /// - Does NOT increment total_frames_read (DIFFERENT from pop_frame())
    ///
    /// # Usage
    /// ```ignore
    /// // In seek() implementation:
    /// for _ in 0..frames_to_skip {
    ///     buffer.pop_frame_skip()?;  // Discard without accounting
    /// }
    /// ```
    pub fn pop_frame_skip(&self) -> Result<(), BufferEmptyError> {
        // Acquire consumer lock only for this operation
        let mut cons = self.cons.lock().unwrap();

        if cons.try_pop().is_some() {
            // Release lock immediately
            drop(cons);

            // Update fill level (Relaxed: statistics only)
            let new_level = self.fill_level.fetch_sub(1, Ordering::Relaxed).saturating_sub(1);
            // **[PLAN022]** CRITICAL: DO NOT increment total_frames_read here!
            // These frames are discarded, not processed by mixer

            // **[DBD-BUF-050]** **[DBD-PARAM-085]** Clear pause flag if buffer has space again
            // (Same logic as pop_frame() - decoder should resume regardless of how space was freed)
            let free_space = self.capacity.saturating_sub(new_level);
            let resume_threshold = self.resume_hysteresis.saturating_add(self.headroom);
            if free_space >= resume_threshold && self.decoder_should_pause.load(Ordering::Relaxed) {
                // Use Release ordering to ensure buffer state visible to decoder
                self.decoder_should_pause.store(false, Ordering::Release);
                trace!(
                    "Playout buffer resume threshold reached (skip): fill={}/{} ({:.1}%), free={}, resume_threshold={}",
                    new_level,
                    self.capacity,
                    self.fill_percent(),
                    free_space,
                    resume_threshold
                );
            }

            Ok(())
        } else {
            // Release lock
            drop(cons);

            // **[DBD-BUF-030][DBD-BUF-040]** Buffer empty - return last valid frame in error
            let left_bits = self.last_frame_left.load(Ordering::Relaxed) as u32;
            let right_bits = self.last_frame_right.load(Ordering::Relaxed) as u32;
            let last_frame = AudioFrame {
                left: f32::from_bits(left_bits),
                right: f32::from_bits(right_bits),
            };

            Err(BufferEmptyError { last_frame })
        }
    }

    /// Get current buffer fill percentage
    ///
    /// # Returns
    /// Fill percentage as f32 in range [0.0, 100.0]
    ///
    /// # Examples
    /// - 0.0 = Empty buffer
    /// - 50.0 = Half full
    /// - 99.3 = Nearly full (typical pause threshold)
    /// - 100.0 = Completely full
    pub fn fill_percent(&self) -> f32 {
        let level = self.fill_level.load(Ordering::Relaxed);
        (level as f32 / self.capacity as f32) * 100.0
    }

    /// Check if decoder should pause due to buffer fill level
    ///
    /// **[DBD-BUF-050]** Returns true when buffer has ≤ headroom free space.
    ///
    /// Decoder should check this flag after each decode chunk and pause if true.
    ///
    /// # Returns
    /// * `true` - Buffer nearly full, decoder should pause
    /// * `false` - Buffer has space, decoder can continue
    pub fn should_decoder_pause(&self) -> bool {
        // Use Acquire ordering to synchronize with Release stores from pop_frame
        self.decoder_should_pause.load(Ordering::Acquire)
    }

    /// Check if buffer is exhausted (decode complete AND empty)
    ///
    /// **[DBD-BUF-060]** Returns true when passage end reached and all samples consumed.
    ///
    /// # Returns
    /// * `true` - Decode complete AND buffer empty (passage finished)
    /// * `false` - Still decoding OR samples remain in buffer
    pub fn is_exhausted(&self) -> bool {
        // Use Acquire ordering to synchronize with Release store in mark_decode_complete
        let decode_done = self.decode_complete.load(Ordering::Acquire);
        let buffer_empty = self.fill_level.load(Ordering::Relaxed) == 0;
        decode_done && buffer_empty
    }

    /// Get buffer statistics for pipeline integrity validation
    ///
    /// **[PHASE1-INTEGRITY]** Returns snapshot of buffer counters for diagnostics
    ///
    /// # Returns
    /// BufferStatistics with total samples written/read and passage ID
    pub fn get_statistics(&self) -> BufferStatistics {
        BufferStatistics {
            total_samples_written: self.total_frames_written.load(Ordering::Relaxed) * 2,
            total_samples_read: self.total_frames_read.load(Ordering::Relaxed) * 2,
            passage_id: self.passage_id,
        }
    }

    /// Mark passage decode as complete
    ///
    /// **[DBD-BUF-060]** Called by decoder when end-of-passage reached.
    ///
    /// After calling this method, `is_exhausted()` will return true once
    /// all remaining buffered samples are consumed by the mixer.
    ///
    /// # Thread Safety
    /// Uses &self (not &mut self) with atomic Release ordering.
    /// Release ensures all prior writes are visible to mixer thread.
    pub fn mark_decode_complete(&self) {
        if !self.decode_complete.load(Ordering::Relaxed) {
            self.decode_complete.store(true, Ordering::Release);
            let level = self.fill_level.load(Ordering::Relaxed);
            debug!(
                "Playout buffer decode complete: passage_id={:?}, remaining_frames={}, fill={:.1}%",
                self.passage_id,
                level,
                self.fill_percent()
            );
        }
    }

    /// Get current buffer statistics
    ///
    /// Returns snapshot of buffer state for monitoring/debugging.
    pub fn stats(&self) -> PlayoutBufferStats {
        let level = self.fill_level.load(Ordering::Relaxed);
        let free_space = self.capacity.saturating_sub(level);

        PlayoutBufferStats {
            capacity: self.capacity,
            occupied: level,
            free: free_space,
            fill_percent: self.fill_percent(),
            should_pause: self.should_decoder_pause(),
            decode_complete: self.decode_complete.load(Ordering::Relaxed),
            is_exhausted: self.is_exhausted(),
            total_written: self.total_frames_written.load(Ordering::Relaxed),
            total_read: self.total_frames_read.load(Ordering::Relaxed),
        }
    }

    /// Get passage ID this buffer is assigned to
    pub fn passage_id(&self) -> Option<Uuid> {
        self.passage_id
    }

    /// Get buffer capacity in frames
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Get current occupied frame count
    pub fn occupied(&self) -> usize {
        self.fill_level.load(Ordering::Relaxed)
    }

    /// Get headroom threshold in frames
    pub fn headroom(&self) -> usize {
        self.headroom
    }

    /// Get resume hysteresis threshold
    ///
    /// **[DBD-BUF-050]** Resume threshold in samples
    pub fn resume_hysteresis(&self) -> usize {
        self.resume_hysteresis
    }

    /// Check if decoder can resume (hysteresis check)
    ///
    /// **[DBD-BUF-050]** Decoder resumes when free space >= resume_hysteresis + headroom
    /// **[DBD-PARAM-085]** Using the sum prevents issues where headroom > hysteresis
    ///
    /// # Returns
    /// * `true` - Buffer has enough free space to resume decoding
    /// * `false` - Buffer still too full, decoder should remain paused
    pub fn can_decoder_resume(&self) -> bool {
        let occupied = self.fill_level.load(Ordering::Acquire);
        let free_space = self.capacity.saturating_sub(occupied);
        let resume_threshold = self.resume_hysteresis.saturating_add(self.headroom);
        let can_resume = free_space >= resume_threshold;

        debug!(
            "can_decoder_resume: occupied={}, free_space={}, capacity={}, resume_threshold={} (hysteresis={} + headroom={}), result={}",
            occupied, free_space, self.capacity, resume_threshold, self.resume_hysteresis, self.headroom, can_resume
        );

        can_resume
    }

    /// Reset buffer to empty state (for reuse)
    ///
    /// Clears all samples, resets counters, and clears flags.
    /// Does NOT change capacity, headroom, or passage_id.
    ///
    /// # Warning
    /// This operation is NOT lock-free. Caller must ensure no concurrent
    /// push/pop operations are in progress.
    ///
    /// # Thread Safety
    /// Acquires both producer and consumer locks to drain buffer.
    /// Safe to call with &self since all state is atomic or mutex-protected.
    pub fn reset(&self) {
        // Clear ring buffer contents by draining consumer
        let mut cons = self.cons.lock().unwrap();
        while cons.try_pop().is_some() {}
        drop(cons);

        // Reset counters and flags (atomic)
        self.fill_level.store(0, Ordering::Relaxed);
        self.decoder_should_pause.store(false, Ordering::Relaxed);
        self.decode_complete.store(false, Ordering::Relaxed);
        self.total_frames_written.store(0, Ordering::Relaxed);
        self.total_frames_read.store(0, Ordering::Relaxed);

        // Reset last frame to silence (atomic)
        let zero_bits = 0.0f32.to_bits() as u64;
        self.last_frame_left.store(zero_bits, Ordering::Relaxed);
        self.last_frame_right.store(zero_bits, Ordering::Relaxed);

        debug!("Playout buffer reset: passage_id={:?}", self.passage_id);
    }
}

/// Playout buffer statistics snapshot
#[derive(Debug, Clone, Copy)]
pub struct PlayoutBufferStats {
    /// Total buffer capacity in frames
    pub capacity: usize,

    /// Current occupied frames
    pub occupied: usize,

    /// Current free space in frames
    pub free: usize,

    /// Fill percentage (0.0 - 100.0)
    pub fill_percent: f32,

    /// Decoder should pause flag
    pub should_pause: bool,

    /// Decode complete flag
    pub decode_complete: bool,

    /// Buffer exhausted (decode done + empty)
    pub is_exhausted: bool,

    /// Total frames written (lifetime counter)
    pub total_written: u64,

    /// Total frames read (lifetime counter)
    pub total_read: u64,
}

impl PlayoutBufferStats {
    /// Check if buffer is healthy (reasonable fill level)
    pub fn is_healthy(&self) -> bool {
        // Healthy if:
        // - Not exhausted
        // - Has some content OR decode not complete (still filling)
        // - Not completely full (should never happen with pause threshold)
        !self.is_exhausted && (self.occupied > 0 || !self.decode_complete) && self.free > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **[DBD-BUF-020]** Test buffer starts empty
    #[test]
    fn test_buffer_starts_empty() {
        let buffer = PlayoutRingBuffer::new(Some(1000), Some(10), None, None);

        assert_eq!(buffer.capacity(), 1000);
        assert_eq!(buffer.occupied(), 0);
        assert_eq!(buffer.fill_percent(), 0.0);
        assert!(!buffer.should_decoder_pause());
        assert!(!buffer.is_exhausted());
    }

    /// **[DBD-BUF-010]** Test basic push/pop operations
    #[test]
    fn test_basic_push_pop() {
        let mut buffer = PlayoutRingBuffer::new(Some(100), Some(5), None, None);

        // Push some frames
        let frame1 = AudioFrame::from_stereo(0.1, 0.2);
        let frame2 = AudioFrame::from_stereo(0.3, 0.4);
        let frame3 = AudioFrame::from_stereo(0.5, 0.6);

        assert!(buffer.push_frame(frame1).is_ok());
        assert!(buffer.push_frame(frame2).is_ok());
        assert!(buffer.push_frame(frame3).is_ok());

        assert_eq!(buffer.occupied(), 3);
        assert_eq!(buffer.fill_percent(), 3.0);

        // Pop frames in FIFO order
        let popped1 = buffer.pop_frame().unwrap();
        assert_eq!(popped1.left, 0.1);
        assert_eq!(popped1.right, 0.2);

        let popped2 = buffer.pop_frame().unwrap();
        assert_eq!(popped2.left, 0.3);
        assert_eq!(popped2.right, 0.4);

        let popped3 = buffer.pop_frame().unwrap();
        assert_eq!(popped3.left, 0.5);
        assert_eq!(popped3.right, 0.6);

        assert_eq!(buffer.occupied(), 0);
        assert_eq!(buffer.fill_percent(), 0.0);
    }

    /// **[DBD-BUF-030][DBD-BUF-040]** Test empty buffer returns last frame in error
    #[test]
    fn test_pop_from_empty_returns_last_frame() {
        let mut buffer = PlayoutRingBuffer::new(Some(100), Some(5), None, None);

        // Push and pop one frame
        let frame = AudioFrame::from_stereo(0.7, 0.8);
        buffer.push_frame(frame).unwrap();
        let popped = buffer.pop_frame().unwrap();
        assert_eq!(popped.left, 0.7);
        assert_eq!(popped.right, 0.8);

        // Buffer now empty - pop should return error with last frame
        let err = buffer.pop_frame().unwrap_err();
        assert_eq!(err.last_frame.left, 0.7);
        assert_eq!(err.last_frame.right, 0.8);

        // Multiple pops from empty should return same last frame
        let err2 = buffer.pop_frame().unwrap_err();
        assert_eq!(err2.last_frame.left, 0.7);
        assert_eq!(err2.last_frame.right, 0.8);
    }

    /// Test fill percentage calculation (0% → 100%)
    #[test]
    fn test_fill_percentage_calculation() {
        let mut buffer = PlayoutRingBuffer::new(Some(100), Some(5), None, None);

        // 0% full
        assert_eq!(buffer.fill_percent(), 0.0);

        // 25% full
        for _ in 0..25 {
            buffer.push_frame(AudioFrame::zero()).unwrap();
        }
        assert_eq!(buffer.fill_percent(), 25.0);

        // 50% full
        for _ in 0..25 {
            buffer.push_frame(AudioFrame::zero()).unwrap();
        }
        assert_eq!(buffer.fill_percent(), 50.0);

        // 100% full
        for _ in 0..50 {
            buffer.push_frame(AudioFrame::zero()).unwrap();
        }
        assert_eq!(buffer.fill_percent(), 100.0);
        assert_eq!(buffer.occupied(), 100);
    }

    /// **[DBD-BUF-050]** Test decoder pause threshold detection
    #[test]
    fn test_decoder_pause_threshold() {
        let headroom = 10;
        let capacity = 100;
        // Set resume_hysteresis to 0 (no hysteresis for this test)
        // Default 44100 is too large for small test buffer
        // With 0 hysteresis, resume threshold = headroom (10)
        let mut buffer = PlayoutRingBuffer::new(Some(capacity), Some(headroom), Some(0), None);

        // Fill to just before threshold (capacity - headroom - 1 = 89 frames)
        for _ in 0..(capacity - headroom - 1) {
            buffer.push_frame(AudioFrame::zero()).unwrap();
            assert!(!buffer.should_decoder_pause(), "Should not pause yet");
        }

        // Push one more frame to reach threshold (90 frames = capacity - headroom)
        buffer.push_frame(AudioFrame::zero()).unwrap();
        assert!(buffer.should_decoder_pause(), "Should pause at threshold");

        // Fill remaining to capacity
        for _ in 0..headroom {
            buffer.push_frame(AudioFrame::zero()).unwrap();
        }
        assert!(buffer.should_decoder_pause(), "Should still pause when full");

        // Pop some frames to create space > headroom
        for _ in 0..(headroom + 1) {
            buffer.pop_frame().unwrap();
        }
        assert!(!buffer.should_decoder_pause(), "Should resume when space available");
    }

    /// **[DBD-BUF-060]** Test buffer exhaustion detection
    #[test]
    fn test_buffer_exhaustion_detection() {
        let mut buffer = PlayoutRingBuffer::new(Some(100), Some(5), None, None);

        // Not exhausted: decode not complete
        assert!(!buffer.is_exhausted());

        // Fill buffer
        for _ in 0..10 {
            buffer.push_frame(AudioFrame::zero()).unwrap();
        }

        // Not exhausted: decode not complete (even though buffer has data)
        assert!(!buffer.is_exhausted());

        // Mark decode complete
        buffer.mark_decode_complete();

        // Not exhausted: decode complete but buffer not empty
        assert!(!buffer.is_exhausted());

        // Drain buffer
        for _ in 0..10 {
            buffer.pop_frame().unwrap();
        }

        // NOW exhausted: decode complete AND buffer empty
        assert!(buffer.is_exhausted());
    }

    /// Test buffer full error
    #[test]
    fn test_buffer_full_error() {
        let mut buffer = PlayoutRingBuffer::new(Some(5), Some(1), None, None);

        // Fill buffer to capacity
        for _ in 0..5 {
            assert!(buffer.push_frame(AudioFrame::zero()).is_ok());
        }

        // Next push should fail
        let err = buffer.push_frame(AudioFrame::zero()).unwrap_err();
        assert_eq!(err.capacity, 5);
        assert_eq!(err.occupied, 5);
    }

    /// Test concurrent fill and drain pattern
    #[test]
    fn test_concurrent_fill_drain_pattern() {
        // Set resume_hysteresis to 100 (reasonable for 1000-frame buffer)
        // Default 44100 is too large for small test buffer
        let mut buffer = PlayoutRingBuffer::new(Some(1000), Some(50), Some(100), None);

        // Simulate decoder filling buffer rapidly
        for i in 0..950 {
            let frame = AudioFrame::from_stereo(i as f32, i as f32);
            buffer.push_frame(frame).unwrap();
        }

        assert_eq!(buffer.occupied(), 950);
        assert!(buffer.should_decoder_pause()); // Should pause at ~95% full

        // Simulate mixer draining while decoder paused
        for _ in 0..100 {
            buffer.pop_frame().unwrap();
        }

        assert_eq!(buffer.occupied(), 850);
        assert!(!buffer.should_decoder_pause()); // Should resume, plenty of space now

        // Decoder can resume filling
        for i in 950..1000 {
            let frame = AudioFrame::from_stereo(i as f32, i as f32);
            buffer.push_frame(frame).unwrap();
        }

        assert_eq!(buffer.occupied(), 900);
    }

    /// Test statistics snapshot
    #[test]
    fn test_stats_snapshot() {
        let mut buffer = PlayoutRingBuffer::new(Some(100), Some(5), None, None);

        // Initial stats
        let stats = buffer.stats();
        assert_eq!(stats.capacity, 100);
        assert_eq!(stats.occupied, 0);
        assert_eq!(stats.free, 100);
        assert_eq!(stats.fill_percent, 0.0);
        assert!(!stats.should_pause);
        assert!(!stats.decode_complete);
        assert!(!stats.is_exhausted);
        assert!(stats.is_healthy());

        // Fill partially
        for _ in 0..50 {
            buffer.push_frame(AudioFrame::zero()).unwrap();
        }

        let stats = buffer.stats();
        assert_eq!(stats.occupied, 50);
        assert_eq!(stats.free, 50);
        assert_eq!(stats.fill_percent, 50.0);
        assert!(!stats.should_pause);
        assert!(stats.is_healthy());

        // Fill to pause threshold
        for _ in 0..45 {
            buffer.push_frame(AudioFrame::zero()).unwrap();
        }

        let stats = buffer.stats();
        assert_eq!(stats.occupied, 95);
        assert!(stats.should_pause);
        assert!(stats.is_healthy());
    }

    /// Test reset operation
    #[test]
    fn test_reset_buffer() {
        let mut buffer = PlayoutRingBuffer::new(Some(100), Some(5), None, Some(Uuid::new_v4()));

        // Fill buffer
        for i in 0..50 {
            buffer.push_frame(AudioFrame::from_stereo(i as f32, i as f32)).unwrap();
        }
        buffer.mark_decode_complete();

        assert_eq!(buffer.occupied(), 50);
        assert!(buffer.decode_complete.load(Ordering::Relaxed));

        // Reset
        buffer.reset();

        // Should be back to initial state
        assert_eq!(buffer.occupied(), 0);
        assert_eq!(buffer.fill_percent(), 0.0);
        assert!(!buffer.should_decoder_pause());
        assert!(!buffer.decode_complete.load(Ordering::Relaxed));
        assert!(!buffer.is_exhausted());

        // Passage ID preserved
        assert!(buffer.passage_id().is_some());

        // Capacity/headroom preserved
        assert_eq!(buffer.capacity(), 100);
        assert_eq!(buffer.headroom(), 5);
    }

    /// Test default parameters match SPEC016
    #[test]
    fn test_default_parameters() {
        let buffer = PlayoutRingBuffer::new(None, None, None, None);

        // **[DBD-PARAM-070]** Default capacity: 661,941 samples = 15.01s @ 44.1kHz
        assert_eq!(buffer.capacity(), 661_941);

        // **[DBD-PARAM-080]** Default headroom: 4410 samples = 0.1s @ 44.1kHz
        assert_eq!(buffer.headroom(), 4410);

        // Verify duration calculation
        let duration_sec = buffer.capacity() as f64 / 44100.0;
        assert!((duration_sec - 15.01).abs() < 0.001); // ~15.01 seconds
    }

    /// Test passage ID tracking
    #[test]
    fn test_passage_id_tracking() {
        let passage_id = Uuid::new_v4();
        let buffer = PlayoutRingBuffer::new(Some(100), Some(5), None, Some(passage_id));

        assert_eq!(buffer.passage_id(), Some(passage_id));
    }

    /// Test lifetime counters
    #[test]
    fn test_lifetime_counters() {
        let mut buffer = PlayoutRingBuffer::new(Some(100), Some(5), None, None);

        // Initial counters
        let stats = buffer.stats();
        assert_eq!(stats.total_written, 0);
        assert_eq!(stats.total_read, 0);

        // Write 50 frames
        for _ in 0..50 {
            buffer.push_frame(AudioFrame::zero()).unwrap();
        }

        let stats = buffer.stats();
        assert_eq!(stats.total_written, 50);
        assert_eq!(stats.total_read, 0);

        // Read 30 frames
        for _ in 0..30 {
            buffer.pop_frame().unwrap();
        }

        let stats = buffer.stats();
        assert_eq!(stats.total_written, 50);
        assert_eq!(stats.total_read, 30);
        assert_eq!(stats.occupied, 20); // 50 - 30 = 20 remaining

        // Write another 25 frames
        for _ in 0..25 {
            buffer.push_frame(AudioFrame::zero()).unwrap();
        }

        let stats = buffer.stats();
        assert_eq!(stats.total_written, 75); // 50 + 25
        assert_eq!(stats.total_read, 30);
        assert_eq!(stats.occupied, 45); // 20 + 25 = 45 remaining
    }

    /// Test edge case: empty buffer exhaustion
    #[test]
    fn test_empty_buffer_exhaustion() {
        let mut buffer = PlayoutRingBuffer::new(Some(100), Some(5), None, None);

        // Mark decode complete without writing any frames
        buffer.mark_decode_complete();

        // Should be immediately exhausted (decode done + empty)
        assert!(buffer.is_exhausted());
    }

    /// Test edge case: pause threshold at boundary
    #[test]
    fn test_pause_threshold_exact_boundary() {
        let capacity = 100;
        let headroom = 10;
        let threshold = capacity - headroom; // = 90
        let mut buffer = PlayoutRingBuffer::new(Some(capacity), Some(headroom), None, None);

        // Fill to exactly threshold - 1 (89 frames)
        for _ in 0..(threshold - 1) {
            buffer.push_frame(AudioFrame::zero()).unwrap();
        }
        assert!(!buffer.should_decoder_pause());

        // Push one more to hit threshold exactly (90 frames)
        buffer.push_frame(AudioFrame::zero()).unwrap();
        assert!(buffer.should_decoder_pause());
    }

    /// Benchmark-style test: large buffer fill/drain cycle
    #[test]
    fn test_large_buffer_fill_drain_cycle() {
        let mut buffer = PlayoutRingBuffer::new(None, None, None, None); // Use defaults (661,941 frames)

        // Fill to pause threshold (capacity - headroom = 657,531 frames)
        // With default headroom of 4410, this is 99.33% full
        let pause_threshold = buffer.capacity() - buffer.headroom();
        for i in 0..pause_threshold {
            let frame = AudioFrame::from_stereo((i % 1000) as f32, (i % 1000) as f32);
            buffer.push_frame(frame).unwrap();
        }

        assert!(buffer.fill_percent() > 99.3);
        assert!(buffer.should_decoder_pause());

        // Drain completely (mimics mixer playback)
        for _ in 0..pause_threshold {
            buffer.pop_frame().unwrap();
        }

        assert_eq!(buffer.fill_percent(), 0.0);
        assert!(!buffer.should_decoder_pause());
    }

    /// Test health check logic
    #[test]
    fn test_health_check_logic() {
        let mut buffer = PlayoutRingBuffer::new(Some(100), Some(5), None, None);

        // Healthy: empty but not exhausted (decode not complete)
        assert!(buffer.stats().is_healthy());

        // Healthy: partially filled
        for _ in 0..50 {
            buffer.push_frame(AudioFrame::zero()).unwrap();
        }
        assert!(buffer.stats().is_healthy());

        // Healthy: nearly full but not exhausted
        for _ in 0..49 {
            buffer.push_frame(AudioFrame::zero()).unwrap();
        }
        assert!(buffer.stats().is_healthy());

        // Drain and complete
        for _ in 0..99 {
            buffer.pop_frame().unwrap();
        }
        buffer.mark_decode_complete();

        // Unhealthy: exhausted
        assert!(!buffer.stats().is_healthy());
    }
}
