//! Buffer event types for event-driven playback
//!
//! **Traceability:**
//! - [DBD-BUF-020] through [DBD-BUF-060] Buffer lifecycle states
//! - [PERF-POLL-010] Event-driven buffer readiness notification
//! - [DBD-BUF-070] Buffer exhaustion detection
//! - [DBD-BUF-080] Underrun recovery

use std::time::Instant;
use uuid::Uuid;

/// Buffer lifecycle states
///
/// [DBD-BUF-020] through [DBD-BUF-060] State machine for buffer management
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferState {
    /// [DBD-BUF-020] Buffer allocated but no samples written yet
    Empty,

    /// [DBD-BUF-030] Decoder actively writing samples (0.5s threshold for Ready)
    Filling,

    /// [DBD-BUF-040] Minimum samples buffered, playable but still filling
    Ready,

    /// [DBD-BUF-050] Mixer actively reading samples
    Playing,

    /// [DBD-BUF-060] All samples decoded, EOF reached
    Finished,
}

impl std::fmt::Display for BufferState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BufferState::Empty => write!(f, "Empty"),
            BufferState::Filling => write!(f, "Filling"),
            BufferState::Ready => write!(f, "Ready"),
            BufferState::Playing => write!(f, "Playing"),
            BufferState::Finished => write!(f, "Finished"),
        }
    }
}

/// Buffer metadata for state tracking
///
/// Tracks buffer position and timing information
#[derive(Debug, Clone)]
pub struct BufferMetadata {
    /// Current buffer state
    pub state: BufferState,

    /// Samples written by decoder (interleaved stereo, so frames = samples / 2)
    pub write_position: usize,

    /// Samples read by mixer (interleaved stereo)
    pub read_position: usize,

    /// Total samples (known after decode complete)
    pub total_samples: Option<usize>,

    /// When buffer was created
    pub created_at: Instant,

    /// When first sample was written
    pub first_sample_at: Option<Instant>,

    /// When Ready state was reached
    pub ready_at: Option<Instant>,

    /// When Playing state started
    pub playing_at: Option<Instant>,

    /// Whether ReadyForStart event has been emitted (prevent duplicates)
    pub ready_notified: bool,
}

impl BufferMetadata {
    /// Create new metadata for Empty buffer
    pub fn new() -> Self {
        Self {
            state: BufferState::Empty,
            write_position: 0,
            read_position: 0,
            total_samples: None,
            created_at: Instant::now(),
            first_sample_at: None,
            ready_at: None,
            playing_at: None,
            ready_notified: false,
        }
    }

    /// Calculate headroom (available samples = write - read)
    pub fn headroom(&self) -> usize {
        self.write_position.saturating_sub(self.read_position)
    }

    /// Check if buffer is exhausted (read caught up to write, and decode finished)
    pub fn is_exhausted(&self) -> bool {
        if let Some(total) = self.total_samples {
            // Decode finished: exhausted if read position >= total
            self.read_position >= total
        } else {
            // Decode still running: not exhausted
            false
        }
    }
}

impl Default for BufferMetadata {
    fn default() -> Self {
        Self::new()
    }
}

/// Buffer events emitted on state transitions
///
/// [PERF-POLL-010] Event-driven buffer readiness notification
#[derive(Debug, Clone)]
pub enum BufferEvent {
    /// Buffer state changed
    StateChanged {
        queue_entry_id: Uuid,
        old_state: BufferState,
        new_state: BufferState,
        samples_buffered: usize,
    },

    /// Buffer reached threshold and is ready to start playback
    /// [PERF-POLL-010] Replaces polling-based readiness checks
    ReadyForStart {
        queue_entry_id: Uuid,
        samples_buffered: usize,
        buffer_duration_ms: u64,
    },

    /// Buffer exhaustion detected (mixer reading faster than decoder writing)
    /// [DBD-BUF-070] Underrun detection
    Exhausted {
        queue_entry_id: Uuid,
        headroom: usize,
    },

    /// Decode finished (all samples written)
    /// [DBD-BUF-060] Completion notification
    Finished {
        queue_entry_id: Uuid,
        total_samples: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_metadata_creation() {
        let metadata = BufferMetadata::new();
        assert_eq!(metadata.state, BufferState::Empty);
        assert_eq!(metadata.write_position, 0);
        assert_eq!(metadata.read_position, 0);
        assert_eq!(metadata.total_samples, None);
        assert!(!metadata.ready_notified);
    }

    #[test]
    fn test_headroom_calculation() {
        let mut metadata = BufferMetadata::new();
        metadata.write_position = 10000;
        metadata.read_position = 3000;

        assert_eq!(metadata.headroom(), 7000);
    }

    #[test]
    fn test_headroom_underflow_protection() {
        let mut metadata = BufferMetadata::new();
        metadata.write_position = 1000;
        metadata.read_position = 5000; // Read ahead of write (shouldn't happen)

        // saturating_sub prevents underflow
        assert_eq!(metadata.headroom(), 0);
    }

    #[test]
    fn test_is_exhausted_not_finished() {
        let mut metadata = BufferMetadata::new();
        metadata.write_position = 5000;
        metadata.read_position = 5000;
        metadata.total_samples = None; // Decode not finished

        // Not exhausted until decode completes
        assert!(!metadata.is_exhausted());
    }

    #[test]
    fn test_is_exhausted_finished_not_read() {
        let mut metadata = BufferMetadata::new();
        metadata.write_position = 10000;
        metadata.read_position = 5000;
        metadata.total_samples = Some(10000); // Decode finished

        // Not exhausted, still have samples to read
        assert!(!metadata.is_exhausted());
    }

    #[test]
    fn test_is_exhausted_finished_and_read() {
        let mut metadata = BufferMetadata::new();
        metadata.write_position = 10000;
        metadata.read_position = 10000;
        metadata.total_samples = Some(10000); // Decode finished

        // Exhausted: decode done and all samples read
        assert!(metadata.is_exhausted());
    }

    #[test]
    fn test_is_exhausted_read_past_end() {
        let mut metadata = BufferMetadata::new();
        metadata.write_position = 10000;
        metadata.read_position = 12000; // Read beyond total
        metadata.total_samples = Some(10000);

        // Definitely exhausted
        assert!(metadata.is_exhausted());
    }

    #[test]
    fn test_buffer_state_transitions() {
        // Valid transitions in lifecycle
        let states = vec![
            BufferState::Empty,
            BufferState::Filling,
            BufferState::Ready,
            BufferState::Playing,
            BufferState::Finished,
        ];

        // Verify all states are distinct
        for (i, state1) in states.iter().enumerate() {
            for (j, state2) in states.iter().enumerate() {
                if i == j {
                    assert_eq!(state1, state2);
                } else {
                    assert_ne!(state1, state2);
                }
            }
        }
    }
}
