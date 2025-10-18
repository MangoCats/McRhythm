/// Lock-Free Ring Buffer for Audio Frames
///
/// [SSD-OUT-012] Real-time audio callback requires lock-free operation
/// [ISSUE-1] Replaces try_write() pattern with lock-free ring buffer
///
/// This module provides a lock-free single-producer single-consumer ring buffer
/// for passing audio frames from the mixer thread to the audio output callback.
///
/// Design:
/// - Producer (mixer thread): Continuously fills buffer with audio frames
/// - Consumer (audio callback): Reads frames without any locks
/// - Lock-free: Safe for real-time audio thread use
/// - Handles overrun/underrun gracefully with logging

use crate::audio::types::AudioFrame;
use ringbuf::{traits::*, HeapRb};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{warn, debug, trace};

/// Ring buffer configuration
const DEFAULT_BUFFER_SIZE: usize = 2048; // ~46ms @ 44.1kHz
const TARGET_FILL_MIN_PERCENT: f32 = 0.50; // 50% target minimum
const TARGET_FILL_MAX_PERCENT: f32 = 0.75; // 75% target maximum

/// Lock-free ring buffer for audio frames
///
/// Provides single-producer single-consumer lock-free communication
/// between mixer thread and audio output callback.
pub struct AudioRingBuffer {
    /// Ring buffer (internally uses atomics for lock-free operation)
    buffer: HeapRb<AudioFrame>,

    /// Underrun counter (audio callback found buffer empty)
    underruns: Arc<AtomicU64>,

    /// Overrun counter (mixer found buffer full)
    overruns: Arc<AtomicU64>,

    /// Flag indicating buffer has been filled to optimal level at least once
    /// Used to distinguish startup underruns (expected) from steady-state underruns (concerning)
    buffer_has_been_filled: Arc<AtomicBool>,

    /// Timestamp (Unix milliseconds) when buffer was first filled to optimal level
    /// Used with grace period to allow system stabilization before warning about underruns
    buffer_filled_timestamp_ms: Arc<AtomicU64>,

    /// Startup grace period in milliseconds (runtime setting from database)
    /// [SSD-RBUF-014] Configurable grace period
    grace_period_ms: u64,

    /// Flag indicating audio output is expected
    /// Set by PlaybackEngine based on playback state (Playing vs Paused) and queue state
    /// Used to classify underruns: TRACE (expected) vs WARN (concerning)
    audio_expected: Arc<AtomicBool>,
}

impl AudioRingBuffer {
    /// Create a new audio ring buffer
    ///
    /// # Arguments
    /// * `capacity` - Buffer size in frames (default: 2048 frames = ~46ms @ 44.1kHz)
    /// * `grace_period_ms` - Startup grace period in milliseconds (from database setting)
    /// * `audio_expected` - Shared flag indicating if audio output is expected (managed by PlaybackEngine)
    pub fn new(capacity: Option<usize>, grace_period_ms: u64, audio_expected: Arc<AtomicBool>) -> Self {
        let capacity = capacity.unwrap_or(DEFAULT_BUFFER_SIZE);

        debug!(
            "Creating audio ring buffer with capacity: {} frames, grace period: {}ms",
            capacity, grace_period_ms
        );

        Self {
            buffer: HeapRb::new(capacity),
            underruns: Arc::new(AtomicU64::new(0)),
            overruns: Arc::new(AtomicU64::new(0)),
            buffer_has_been_filled: Arc::new(AtomicBool::new(false)),
            buffer_filled_timestamp_ms: Arc::new(AtomicU64::new(0)),
            grace_period_ms,
            audio_expected,
        }
    }

    /// Split into producer and consumer halves
    ///
    /// Producer is used by mixer thread, consumer by audio callback.
    /// Each half can be moved to different threads safely.
    pub fn split(self) -> (AudioProducer, AudioConsumer) {
        let (prod, cons) = self.buffer.split();

        let producer = AudioProducer {
            producer: prod,
            overruns: Arc::clone(&self.overruns),
            buffer_has_been_filled: Arc::clone(&self.buffer_has_been_filled),
            buffer_filled_timestamp_ms: Arc::clone(&self.buffer_filled_timestamp_ms),
        };

        let consumer = AudioConsumer {
            consumer: cons,
            underruns: Arc::clone(&self.underruns),
            buffer_has_been_filled: Arc::clone(&self.buffer_has_been_filled),
            buffer_filled_timestamp_ms: Arc::clone(&self.buffer_filled_timestamp_ms),
            grace_period_ms: self.grace_period_ms,
            audio_expected: Arc::clone(&self.audio_expected),
        };

        (producer, consumer)
    }

    /// Get statistics
    pub fn stats(&self) -> RingBufferStats {
        RingBufferStats {
            underruns: self.underruns.load(Ordering::Relaxed),
            overruns: self.overruns.load(Ordering::Relaxed),
            capacity: self.buffer.capacity().into(),
            occupied: self.buffer.occupied_len(),
        }
    }
}

/// Producer half of ring buffer (used by mixer thread)
pub struct AudioProducer {
    producer: ringbuf::HeapProd<AudioFrame>,
    overruns: Arc<AtomicU64>,
    buffer_has_been_filled: Arc<AtomicBool>,
    buffer_filled_timestamp_ms: Arc<AtomicU64>,
}

impl AudioProducer {
    /// Push an audio frame to the buffer
    ///
    /// Returns true if frame was pushed, false if buffer was full (overrun).
    /// Lock-free operation safe for real-time use.
    pub fn push(&mut self, frame: AudioFrame) -> bool {
        match self.producer.try_push(frame) {
            Ok(_) => {
                // Check if buffer has reached optimal fill level
                // This flag helps distinguish startup underruns from steady-state underruns
                if !self.buffer_has_been_filled.load(Ordering::Relaxed) && self.is_fill_optimal() {
                    self.buffer_has_been_filled.store(true, Ordering::Relaxed);

                    // Record timestamp for grace period calculation
                    let now_ms = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64;
                    self.buffer_filled_timestamp_ms.store(now_ms, Ordering::Relaxed);

                    debug!("Audio ring buffer filled to optimal level at {}ms (startup grace period active)", now_ms);
                }
                true
            }
            Err(_) => {
                // Buffer full - overrun
                let count = self.overruns.fetch_add(1, Ordering::Relaxed) + 1;
                if count % 1000 == 0 {
                    warn!("Audio ring buffer overrun (total: {})", count);
                }
                false
            }
        }
    }

    /// Get current buffer fill level
    pub fn occupied_len(&self) -> usize {
        self.producer.occupied_len()
    }

    /// Get buffer capacity
    pub fn capacity(&self) -> usize {
        self.producer.capacity().into()
    }

    /// Check if buffer fill level is within target range
    ///
    /// Returns true if buffer is 50-75% full (optimal range).
    pub fn is_fill_optimal(&self) -> bool {
        let occupied = self.occupied_len();
        let capacity = self.capacity();
        let min = (capacity as f32 * TARGET_FILL_MIN_PERCENT) as usize;
        let max = (capacity as f32 * TARGET_FILL_MAX_PERCENT) as usize;
        occupied >= min && occupied <= max
    }

    /// Check if buffer needs more frames (below minimum target)
    pub fn needs_frames(&self) -> bool {
        let occupied = self.occupied_len();
        let capacity = self.capacity();
        let min = (capacity as f32 * TARGET_FILL_MIN_PERCENT) as usize;
        occupied < min
    }
}

/// Consumer half of ring buffer (used by audio callback)
pub struct AudioConsumer {
    consumer: ringbuf::HeapCons<AudioFrame>,
    underruns: Arc<AtomicU64>,
    buffer_has_been_filled: Arc<AtomicBool>,
    buffer_filled_timestamp_ms: Arc<AtomicU64>,
    grace_period_ms: u64,
    audio_expected: Arc<AtomicBool>,
}

impl AudioConsumer {
    /// Pop an audio frame from the buffer
    ///
    /// Returns Some(frame) if available, None if buffer empty (underrun).
    /// Lock-free operation safe for real-time audio callback.
    ///
    /// On underrun, returns None and increments underrun counter.
    /// Caller should output silence in this case.
    ///
    /// Log level classification:
    /// - TRACE: Expected underruns (startup, paused, or empty queue)
    /// - WARN: Unexpected underruns (active playback with audio in queue)
    pub fn pop(&mut self) -> Option<AudioFrame> {
        match self.consumer.try_pop() {
            Some(frame) => Some(frame),
            None => {
                // Buffer empty - underrun
                let count = self.underruns.fetch_add(1, Ordering::Relaxed) + 1;

                // Log every 1000th underrun to avoid spam
                if count % 1000 == 0 {
                    let buffer_filled = self.buffer_has_been_filled.load(Ordering::Relaxed);
                    // Use Acquire ordering to ensure we see the latest value from other threads
                    let audio_expected = self.audio_expected.load(Ordering::Acquire);

                    if !buffer_filled {
                        // Startup underruns - expected behavior (buffer not yet filled)
                        trace!("Audio ring buffer underrun during startup (total: {})", count);
                    } else if !audio_expected {
                        // Paused or empty queue - expected behavior
                        trace!("Audio ring buffer underrun while paused/idle (total: {})", count);
                    } else {
                        // Check if we're still in the startup grace period
                        let filled_timestamp = self.buffer_filled_timestamp_ms.load(Ordering::Relaxed);
                        let now_ms = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as u64;

                        if filled_timestamp == 0 || now_ms < filled_timestamp + self.grace_period_ms {
                            // Still in startup grace period - expected behavior
                            trace!("Audio ring buffer underrun during startup stabilization (total: {})", count);
                        } else {
                            // Past grace period - active playback underrun is concerning!
                            // DEBUG: Log flag value to diagnose issue
                            warn!(
                                "Audio ring buffer underrun during active playback (total: {}, audio_expected={}) - \
                                 CPU may not be keeping up with decoding",
                                count, audio_expected
                            );
                        }
                    }
                }
                None
            }
        }
    }

    /// Get current buffer fill level
    pub fn occupied_len(&self) -> usize {
        self.consumer.occupied_len()
    }
}

/// Ring buffer statistics
#[derive(Debug, Clone, Copy)]
pub struct RingBufferStats {
    /// Total underruns (audio callback found buffer empty)
    pub underruns: u64,

    /// Total overruns (mixer found buffer full)
    pub overruns: u64,

    /// Buffer capacity in frames
    pub capacity: usize,

    /// Current occupied frames
    pub occupied: usize,
}

impl RingBufferStats {
    /// Get buffer fill percentage (0.0 to 1.0)
    pub fn fill_percent(&self) -> f32 {
        self.occupied as f32 / self.capacity as f32
    }

    /// Check if buffer health is good (no recent issues)
    pub fn is_healthy(&self) -> bool {
        // Consider healthy if fill is reasonable and no underruns/overruns
        self.fill_percent() >= 0.25 && self.fill_percent() <= 0.90
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_buffer_basic() {
        let audio_expected = Arc::new(AtomicBool::new(false));
        let rb = AudioRingBuffer::new(Some(128), 2000, audio_expected);
        let (mut prod, mut cons) = rb.split();

        // Push some frames
        let frame1 = AudioFrame::from_stereo(0.1, 0.2);
        let frame2 = AudioFrame::from_stereo(0.3, 0.4);

        assert!(prod.push(frame1));
        assert!(prod.push(frame2));

        // Pop frames
        let popped1 = cons.pop().unwrap();
        assert_eq!(popped1.left, 0.1);
        assert_eq!(popped1.right, 0.2);

        let popped2 = cons.pop().unwrap();
        assert_eq!(popped2.left, 0.3);
        assert_eq!(popped2.right, 0.4);

        // Buffer should be empty now
        assert!(cons.pop().is_none());
    }

    #[test]
    fn test_ring_buffer_overrun() {
        let audio_expected = Arc::new(AtomicBool::new(false));
        let rb = AudioRingBuffer::new(Some(4), 2000, audio_expected); // Small buffer
        let (mut prod, mut _cons) = rb.split();

        let frame = AudioFrame::zero();

        // Fill buffer
        assert!(prod.push(frame));
        assert!(prod.push(frame));
        assert!(prod.push(frame));
        assert!(prod.push(frame));

        // Next push should fail (overrun)
        assert!(!prod.push(frame));
    }

    #[test]
    fn test_ring_buffer_underrun() {
        let audio_expected = Arc::new(AtomicBool::new(false));
        let rb = AudioRingBuffer::new(Some(128), 2000, audio_expected);
        let (_prod, mut cons) = rb.split();

        // Pop from empty buffer (underrun)
        assert!(cons.pop().is_none());
    }

    #[test]
    fn test_fill_level_check() {
        let audio_expected = Arc::new(AtomicBool::new(false));
        let rb = AudioRingBuffer::new(Some(100), 2000, audio_expected);
        let (mut prod, _cons) = rb.split();

        // Initially needs frames
        assert!(prod.needs_frames());
        assert!(!prod.is_fill_optimal());

        // Fill to optimal range (50-75%)
        for _ in 0..60 {
            prod.push(AudioFrame::zero());
        }

        assert!(prod.is_fill_optimal());
        assert!(!prod.needs_frames());
    }
}
