//! Playback types shared across modules

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

// Note: BufferEvent moved to buffer_events.rs for Phase 4C event-driven architecture
