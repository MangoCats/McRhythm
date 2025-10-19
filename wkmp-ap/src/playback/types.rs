//! Playback types shared across modules

use uuid::Uuid;

/// Decode priority for decoder pool
///
/// [SSD-DEC-032] Priority queue management
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DecodePriority {
    /// Currently playing (underrun recovery)
    Immediate = 0,

    /// Next to play
    Next = 1,

    /// Queued passages (prefetch)
    Prefetch = 2,
}

/// Buffer events for event-driven playback start
///
/// **[PERF-POLL-010]** Event-driven buffer readiness notification
#[derive(Debug, Clone)]
pub enum BufferEvent {
    /// Buffer has reached minimum threshold and is ready to start playback
    ReadyForStart {
        /// Queue entry ID (passage identifier)
        queue_entry_id: Uuid,

        /// Current buffer duration in milliseconds
        buffer_duration_ms: u64,
    },
}
