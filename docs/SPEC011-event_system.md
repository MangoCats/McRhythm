# WKMP Event System

**ðŸ“¡ TIER 2 - DESIGN SPECIFICATION**

Defines event-driven communication architecture. Derived from [requirements.md](REQ001-requirements.md). See [Document Hierarchy](GOV001-document_hierarchy.md).

> **Related Documentation:** [Architecture](SPEC001-architecture.md) | [Coding Conventions](IMPL002-coding_conventions.md)

---

## Overview

WKMP uses a hybrid event-driven communication architecture combining event broadcasting with direct message passing. This document specifies the event system design, implementation patterns, and usage guidelines.

### Design Rationale

**Why Event-Driven Architecture?**

WKMP requires coordinated responses to state changes across multiple components:
- Multiple UI clients need real-time synchronization (`REQ-CF-042`)
- Play events trigger actions in Historian, Queue Manager, and SSE Broadcaster
- User actions must propagate to all connected clients
- Components should be loosely coupled for testability and maintainability

**Why Not External Signal/Slot Libraries?**

Considered `signals2` and similar observer pattern crates but rejected because:

1. **Async Integration**: WKMP is built on Tokio async runtime
   - Signal/slot libraries are primarily synchronous
   - Mixing sync callbacks with async handlers creates complexity
   - Tokio broadcast channels are async-native

2. **Performance**: Raspberry Pi Zero2W resource constraints
   - Broadcast channels have minimal overhead
   - Direct dispatch without dynamic allocation
   - No virtual function call overhead

3. **Ecosystem Maturity**: Zero external dependencies
   - Tokio broadcast is battle-tested
   - No risk of unmaintained dependencies
   - Built-in to async runtime

## Communication Patterns

WKMP uses a hybrid communication architecture. See [Architecture - Inter-component Communication](SPEC001-architecture.md#inter-component-communication) for overview.

This document specifies the **event broadcasting** pattern in detail.

### Pattern Selection Matrix

| Pattern | Use When | Examples |
|---------|----------|----------|
| **Event Broadcasting** (this doc) | One event â†’ many listeners | Playback state changes, queue updates |
| **Command Channels** | Request â†’ single handler | Play/Pause/Skip commands |
| **Shared State** | Read-heavy access | Current position, volume |
| **Watch Channels** | Single value, many readers | Volume level, network status |

> For command channels, shared state, and watch patterns, see [Architecture - Inter-component Communication](SPEC001-architecture.md#inter-component-communication).

### Communication Pattern Diagram

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚                   Communication Patterns                    â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚                                                             â”‚
â”‚  EVENT BROADCASTING (tokio::broadcast)                      â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”                                           â”‚
â”‚  â”‚  Event Bus   â”‚â”€â”€â”€â”€â”€â”€â”                                    â”‚
â”‚  â”‚  (broadcast) â”‚      â”‚                                    â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜      â”‚                                    â”‚
â”‚         â”‚              â”œâ”€â”€> Historian (subscribe)           â”‚
â”‚         â”‚              â”œâ”€â”€> Queue Manager (subscribe)       â”‚
â”‚         â”‚              â”œâ”€â”€> SSE Broadcaster (subscribe)     â”‚
â”‚         â”‚              â””â”€â”€> State Persistence (subscribe)   â”‚
â”‚         â”‚                                                   â”‚
â”‚  Emitters:                                                  â”‚
â”‚    â€¢ Playback Controller â†’ PassageStarted, PassageCompleted â”‚
â”‚    â€¢ Queue Manager â†’ QueueChanged                           â”‚
â”‚    â€¢ API Handlers â†’ UserAction                              â”‚
â”‚                                                             â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚                                                             â”‚
â”‚  COMMAND CHANNELS (tokio::mpsc)                             â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”      â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”                     â”‚
â”‚  â”‚  API Layer   â”‚â”€â”€â”€â”€â”€>â”‚   Command    â”‚                     â”‚
â”‚  â”‚              â”‚      â”‚   Channel    â”‚                     â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜      â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜                     â”‚
â”‚                              â”‚                              â”‚
â”‚                              â–¼                              â”‚
â”‚                    â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”                     â”‚
â”‚                    â”‚ Component Handlerâ”‚                     â”‚
â”‚                    â”‚  (processes cmd, â”‚                     â”‚
â”‚                    â”‚   returns Result)â”‚                     â”‚
â”‚                    â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜                     â”‚
â”‚                                                             â”‚
â”‚  Used for:                                                  â”‚
â”‚    â€¢ Playback commands: Play, Pause, Skip, Seek             â”‚
â”‚    â€¢ Selection requests: SelectPassage â†’ Result<PassageId>  â”‚
â”‚    â€¢ Queue operations: Enqueue, Remove                      â”‚
â”‚                                                             â”‚
â”œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¤
â”‚                                                             â”‚
â”‚  SHARED STATE (Arc<RwLock<T>>)                              â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”                       â”‚
â”‚  â”‚  Current Playback State          â”‚                       â”‚
â”‚  â”‚  â€¢ position, passage_id, status  â”‚                       â”‚
â”‚  â”‚  â€¢ Read-heavy, write-light       â”‚                       â”‚
â”‚  â”‚  â€¢ Multiple readers, rare writes â”‚                       â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜                       â”‚
â”‚                                                             â”‚
â”‚  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”                       â”‚
â”‚  â”‚  Queue State                     â”‚                       â”‚
â”‚  â”‚  â€¢ Current queue contents        â”‚                       â”‚
â”‚  â”‚  â€¢ Read for display/decisions    â”‚                       â”‚
â”‚  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜                       â”‚
â”‚                                                             â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

## Event Types

### Event Enumeration

All events are variants of a central `WkmpEvent` enum:

```rust
/// Global event types for cross-component communication
///
/// Events are broadcast via the EventBus and can be subscribed to by any component.
/// Each event is self-contained and includes all necessary context.
#[derive(Debug, Clone)]
pub enum WkmpEvent {
    // â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
    // Playback Events
    // â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

    /// Emitted when a passage begins playback
    ///
    /// Triggers:
    /// - Historian: Record play start
    /// - SSE: Update all connected UIs
    /// - Lyrics Display: Show passage lyrics
    PassageStarted {
        passage_id: PassageId,
        timestamp: SystemTime,
        queue_position: u32,
    },

    /// Emitted when a passage finishes or is skipped
    ///
    /// Triggers:
    /// - Historian: Record play completion
    /// - Queue Manager: Advance queue
    /// - SSE: Update UI playback state
    PassageCompleted {
        passage_id: PassageId,
        duration_played: f64,
        completed: bool, // false if skipped
        timestamp: SystemTime,
    },

    /// Emitted when playback state changes (Playing/Paused/Stopped)
    ///
    /// Triggers:
    /// - SSE: Update UI controls
    /// - State Persistence: Save current state
    /// - Platform Integration: Update MPRIS/media keys
    PlaybackStateChanged {
        old_state: PlaybackState,
        new_state: PlaybackState,
        timestamp: SystemTime,
    },

    /// Emitted periodically during playback
    ///
    /// Frequency controlled by `playback_progress_interval_ms` setting (default: 5000ms)
    /// Also emitted once when Pause initiated, once when Play initiated (resume)
    ///
    /// NOTE: Configurable-frequency event. Subscribers should process quickly.
    /// Consider using watch channel for position updates instead.
    ///
    /// Triggers:
    /// - SSE: Update progress bar
    /// - NOT persisted to database during playback (only transmitted via SSE)
    /// - Database persistence only on clean shutdown via settings.last_played_position
    PlaybackProgress {
        passage_id: PassageId,
        position_ms: u64,     // Current position in milliseconds
        duration_ms: u64,     // Total passage duration in milliseconds
        timestamp: SystemTime,
    },

    /// Emitted when volume changes
    ///
    /// Triggers:
    /// - SSE: Update volume slider
    /// - State Persistence: Save volume preference
    VolumeChanged {
        old_volume: f32,  // 0.0-1.0 (system-level scale for precision)
        new_volume: f32,  // 0.0-1.0 (system-level scale for precision)
        timestamp: SystemTime,
    },

    /// Emitted when current song within passage changes
    ///
    /// NOTE: This is distinct from PassageStarted (which is for passage transitions)
    /// This event fires when crossing song boundaries within a single passage
    ///
    /// Triggers:
    /// - UI: Update album art display to reflect new song
    /// - UI: Reset album rotation timer if song has multiple albums
    /// - UI: Update now playing song information
    CurrentSongChanged {
        passage_id: PassageId,
        song_id: Option<SongId>,  // None if in a gap between songs
        song_albums: Vec<AlbumId>,  // All albums associated with this song
        position: f64,  // Current position in passage (seconds)
    },

    /// Emitted when passage buffer state transitions
    ///
    /// Purpose: Notify clients of passage buffer decode/playback state changes
    /// for monitoring and debugging
    ///
    /// Module: wkmp-ap (Audio Player)
    ///
    /// Triggers:
    /// - Developer UI: Show buffer state transitions for debugging
    /// - Performance monitoring: Track decode speed vs. playback speed
    /// - UI display: Show decode progress for large files
    BufferStateChanged {
        passage_id: PassageId,
        old_state: BufferStatus,
        new_state: BufferStatus,
        decode_progress_percent: Option<f32>,  // Only for Decoding state
        timestamp: SystemTime,
    },

    // â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
    // Queue Events
    // â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

    /// Emitted when the queue contents change
    ///
    /// Triggers:
    /// - SSE: Update queue display
    /// - Auto-replenishment: Check if refill needed
    QueueChanged {
        queue: Vec<PassageId>,
        trigger: QueueChangeTrigger,
        timestamp: SystemTime,
    },

    /// Emitted when a passage is added to queue
    ///
    /// Triggers:
    /// - SSE: Animate new queue entry
    /// - Analytics: Track auto vs manual enqueue
    PassageEnqueued {
        passage_id: PassageId,
        position: usize,
        source: EnqueueSource,
        timestamp: SystemTime,
    },

    /// Emitted when a passage is removed from queue
    ///
    /// Triggers:
    /// - SSE: Update queue display
    PassageDequeued {
        passage_id: PassageId,
        was_playing: bool,
    },

    /// Emitted when queue becomes empty
    ///
    /// NOTE: This does NOT change Play/Pause state automatically
    ///
    /// Triggers:
    /// - SSE: Update UI to show empty queue state
    /// - UI: May show "Queue Empty" message
    /// - Automatic selector: Already stopped (no valid candidates)
    QueueEmpty {
        timestamp: SystemTime,
        playback_state: PlaybackState,  // Current Play/Pause state (unchanged)
    },

    // â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
    // User Interaction Events
    // â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

    /// Emitted when user performs an action (satisfies REQ-CF-042)
    ///
    /// Used for multi-user synchronization and edge case handling:
    /// - Skip throttling (5-second window, REQ-CF-044)
    /// - Concurrent operation handling
    ///
    /// Triggers:
    /// - SSE: Broadcast to all other connected clients
    /// - Skip Throttle: Track recent skip actions
    /// - Analytics: User interaction tracking
    UserAction {
        action: UserActionType,
        user_id: String,  // User's persistent UUID (see user_identity.md)
        timestamp: SystemTime,
    },

    /// Emitted when user likes a passage (Full/Lite versions only)
    ///
    /// Triggers:
    /// - Database: Record like associated with user UUID
    /// - Taste Manager: Update user's taste profile
    /// - SSE: Update like button state for all connected clients
    PassageLiked {
        passage_id: PassageId,
        user_id: UserId,  // UUID of user who liked (may be Anonymous UUID)
        timestamp: SystemTime,
    },

    /// Emitted when user dislikes a passage (Full/Lite versions only)
    ///
    /// Triggers:
    /// - Database: Record dislike associated with user UUID
    /// - Taste Manager: Update user's taste profile
    /// - SSE: Update dislike button state for all connected clients
    /// - Program Director: Adjust selection probability (Phase 2)
    PassageDisliked {
        passage_id: PassageId,
        user_id: UserId,  // UUID of user who disliked (may be Anonymous UUID)
        timestamp: SystemTime,
    },

    // â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
    // Musical Flavor Events
    // â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

    /// Emitted when user sets temporary flavor override
    ///
    /// Implements REQ-FLV-020: Temporary override behavior
    ///
    /// Triggers:
    /// - Queue Manager: Flush existing queue
    /// - Playback Controller: Skip remaining time on current passage
    /// - Program Director: Use new target for selection
    /// - SSE: Show override indicator in UI
    TemporaryFlavorOverride {
        target_flavor: FlavorVector,
        expiration: SystemTime,
        duration: Duration,
    },

    /// Emitted when temporary override expires
    ///
    /// Triggers:
    /// - Program Director: Revert to timeslot-based target
    /// - SSE: Remove override indicator
    TemporaryFlavorOverrideExpired {
        timestamp: SystemTime,
    },

    /// Emitted when timeslot changes (e.g., midnight â†’ morning)
    ///
    /// NOTE: Does NOT affect currently queued passages (REQ-FLV-030)
    ///
    /// Triggers:
    /// - Program Director: Update target flavor for new selections
    /// - SSE: Update current timeslot indicator
    TimeslotChanged {
        old_timeslot_id: TimeslotId,
        new_timeslot_id: TimeslotId,
        new_target_flavor: FlavorVector,
        timestamp: SystemTime,
    },

    // â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
    // System Events
    // â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•

    /// Emitted when network connectivity status changes
    ///
    /// Implements REQ-NET-010: Network error handling
    ///
    /// Triggers:
    /// - External API clients: Pause/resume requests
    /// - SSE: Show offline indicator
    NetworkStatusChanged {
        available: bool,
        retry_count: u32,
    },

    /// Emitted when library scan completes (Full version only)
    ///
    /// Triggers:
    /// - SSE: Update library stats
    /// - Program Director: Refresh available passages
    LibraryScanCompleted {
        files_added: usize,
        files_updated: usize,
        files_removed: usize,
        duration: Duration,
    },

    /// Emitted when database error occurs
    ///
    /// Triggers:
    /// - Error logging
    /// - SSE: Show error notification
    /// - Retry logic: Attempt recovery
    DatabaseError {
        operation: String,
        error: String,
        retry_attempted: bool,
    },
}

/// Why a user action occurred
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserActionType {
    Skip,
    Play,
    Pause,
    Seek,
    VolumeChange,
    QueueAdd,
    QueueRemove,
    Like,
    Dislike,
    TemporaryOverride,
}

/// Why the queue changed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueChangeTrigger {
    AutomaticReplenishment,
    UserEnqueue,
    UserDequeue,
    PassageCompletion,
    TemporaryOverride,
}

**Note on Queue Empty Behavior:**
- When `PassageCompletion` or `UserDequeue` triggers `QueueChanged` and results in `queue.len() == 0`, the system also emits `QueueEmpty`
- `QueueEmpty` does not change playback state
- User-controlled Play/Pause state persists regardless of queue state
- Automatic selection stops when no valid candidates exist, but manual enqueueing always works

/// How a passage was enqueued
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnqueueSource {
    Automatic,
    Manual,
}

/// Playback state
/// <a name="playbackstate-enum"></a>
///
/// WKMP has only two playback states controlled by the user.
/// There is no "stopped" state - the system is always either playing or paused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    Playing,   // Audio plays when passages are available in queue
    Paused,    // Audio paused by user, regardless of queue state
}

/// Buffer status for passage decode/playback lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferStatus {
    Decoding,   // Buffer currently being populated from audio file
    Ready,      // Buffer fully decoded and ready for playback
    Playing,    // Buffer currently being read for audio output
    Exhausted,  // Buffer playback completed
}

**Note on Playback State:**
- System always starts in `Playing` state on app launch
- Only two states exist: `Playing` and `Paused`
- No "stopped" state (traditional media player concept doesn't apply)
- User controls state via Play/Pause commands
- State persists independently of queue contents
- When queue is empty:
  - `Playing` state: Silent, ready to play immediately when passage enqueued
  - `Paused` state: Silent, new passages wait until user selects Play
```

### Event Categories

**Playback Events:**
- **Publishers**: Playback Controller, Passage Buffer Manager
- **Subscribers**: Historian, SSE Broadcaster, State Persistence, Lyrics Display, Developer UI
- **Frequency**: Low (on state change) to Medium (position updates every 500ms)

**Queue Events:**
- **Publishers**: Queue Manager
- **Subscribers**: SSE Broadcaster, Auto-replenishment logic, Analytics
- **Frequency**: Low (occasional queue changes)

**User Interaction Events:**
- **Publishers**: API handlers (Tauri commands, REST endpoints)
- **Subscribers**: SSE Broadcaster, Skip throttle logic, Analytics
- **Frequency**: Low (user-driven)

**Musical Flavor Events:**
- **Publishers**: Flavor Manager, Timeslot scheduler
- **Subscribers**: Program Director, Queue Manager, SSE Broadcaster
- **Frequency**: Very Low (manual overrides, scheduled timeslot changes)

**System Events:**
- **Publishers**: Network layer, Library scanner, Database layer
- **Subscribers**: Error logging, SSE Broadcaster, External API clients
- **Frequency**: Low (error conditions, periodic scans)

### BufferStateChanged Event

#### Purpose

Notify clients of passage buffer decode/playback state transitions for monitoring and debugging.

#### Module

wkmp-ap (Audio Player)

#### Event Data

- `passage_id` (PassageId): Passage whose buffer state changed
- `old_state` (BufferStatus): Previous buffer state
- `new_state` (BufferStatus): New buffer state
- `decode_progress_percent` (Option<f32>): Decode progress (only for Decoding state)
- `timestamp` (SystemTime): When transition occurred

#### BufferStatus Values

- `Decoding`: Buffer currently being populated from audio file
- `Ready`: Buffer fully decoded and ready for playback
- `Playing`: Buffer currently being read for audio output
- `Exhausted`: Buffer playback completed

#### Emission Points

- Decoder starts filling buffer (None â†’ Decoding)
- Decoder completes buffer (Decoding â†’ Ready)
- Mixer starts reading buffer (Ready â†’ Playing)
- Mixer finishes buffer (Playing â†’ Exhausted)

#### Use Cases

- Developer debugging of buffer underrun scenarios (SSD-UND-015)
- UI display of decode progress for large files
- Performance monitoring of decode speed vs. playback speed
- Pre-buffering diagnostics for skip-ahead behavior

#### Traceability

SSD-BUF-010 (Buffer state visibility requirement)

## EventBus Implementation

### Core EventBus Structure

```rust
use tokio::sync::broadcast;
use std::sync::Arc;

/// Central event distribution bus for application-wide events
///
/// The EventBus uses tokio::broadcast internally, providing:
/// - Non-blocking publish (slow subscribers don't block producers)
/// - Multiple concurrent subscribers
/// - Automatic cleanup when subscribers drop
/// - Lagged message detection for slow subscribers
///
/// # Examples
///
/// ```
/// let event_bus = Arc::new(EventBus::new(1000));
///
/// // Subscribe to events
/// let mut rx = event_bus.subscribe();
///
/// // Emit an event
/// event_bus.emit(WkmpEvent::PassageStarted {
///     passage_id: passage.id,
///     timestamp: SystemTime::now(),
///     queue_position: 0,
/// }).ok();
///
/// // Receive events
/// while let Ok(event) = rx.recv().await {
///     match event {
///         WkmpEvent::PassageStarted { .. } => {
///             // Handle passage start
///         }
///         _ => {}
///     }
/// }
/// ```
pub struct EventBus {
    tx: broadcast::Sender<WkmpEvent>,
    capacity: usize,
}

impl EventBus {
    /// Creates a new EventBus with specified channel capacity
    ///
    /// # Arguments
    ///
    /// * `capacity` - Number of events to buffer before dropping old events
    ///
    ///   Recommended values:
    ///   - Development/Desktop: 1000
    ///   - Raspberry Pi Zero2W: 500
    ///   - Testing: 10-100
    ///
    /// # Examples
    ///
    /// ```
    /// let event_bus = EventBus::new(1000);
    /// ```
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx, capacity }
    }

    /// Subscribe to all future events
    ///
    /// Returns a receiver that will receive all events emitted after subscription.
    /// Events emitted before subscription are not received.
    ///
    /// # Examples
    ///
    /// ```
    /// let event_bus = Arc::new(EventBus::new(1000));
    /// let mut rx = event_bus.subscribe();
    ///
    /// tokio::spawn(async move {
    ///     while let Ok(event) = rx.recv().await {
    ///         println!("Received event: {:?}", event);
    ///     }
    /// });
    /// ```
    pub fn subscribe(&self) -> broadcast::Receiver<WkmpEvent> {
        self.tx.subscribe()
    }

    /// Emit an event to all subscribers
    ///
    /// Returns `Ok(subscriber_count)` if at least one subscriber exists.
    /// Returns `Err` if no subscribers are listening.
    ///
    /// # Examples
    ///
    /// ```
    /// // Critical event - log if no subscribers
    /// if let Err(_) = event_bus.emit(critical_event) {
    ///     tracing::warn!("No subscribers for critical event");
    /// }
    /// ```
    pub fn emit(&self, event: WkmpEvent) -> Result<usize, broadcast::error::SendError<WkmpEvent>> {
        self.tx.send(event)
    }

    /// Emit an event, ignoring if no subscribers are listening
    ///
    /// This is useful for non-critical events where it's acceptable if
    /// no component is currently listening.
    ///
    /// # Examples
    ///
    /// ```
    /// // Position updates - OK if no one is listening
    /// event_bus.emit_lossy(WkmpEvent::PositionUpdate {
    ///     passage_id,
    ///     position: 42.0,
    ///     duration: 180.0,
    /// });
    /// ```
    pub fn emit_lossy(&self, event: WkmpEvent) {
        let _ = self.tx.send(event);
    }

    /// Get the current number of active subscribers
    ///
    /// Useful for debugging and monitoring
    pub fn subscriber_count(&self) -> usize {
        self.tx.receiver_count()
    }

    /// Get the configured channel capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}
```

### Component Integration Pattern

Components that emit events should receive an `Arc<EventBus>`:

```rust
/// Playback Controller - Event Producer Example
pub struct PlaybackController {
    event_bus: Arc<EventBus>,
    command_rx: mpsc::Receiver<PlaybackCommand>,
    current_passage: Option<PassageId>,
    state: PlaybackState,
    // ... other fields
}

impl PlaybackController {
    pub fn new(
        event_bus: Arc<EventBus>,
        command_rx: mpsc::Receiver<PlaybackCommand>,
    ) -> Self {
        Self {
            event_bus,
            command_rx,
            current_passage: None,
            state: PlaybackState::Stopped,
        }
    }

    /// Main event loop processing commands
    pub async fn run(mut self) {
        while let Some(cmd) = self.command_rx.recv().await {
            match cmd {
                PlaybackCommand::Play => self.handle_play().await,
                PlaybackCommand::Pause => self.handle_pause().await,
                PlaybackCommand::Skip => self.handle_skip().await,
            }
        }
    }

    /// Handle passage completion
    async fn on_passage_completed(&mut self, duration: f64, completed: bool) {
        if let Some(passage_id) = self.current_passage {
            // Emit event - implements event system specification
            self.event_bus.emit(WkmpEvent::PassageCompleted {
                passage_id,
                duration_played: duration,
                completed,
                timestamp: SystemTime::now(),
            }).ok(); // Lossy - OK if no listeners yet

            // Update internal state
            self.current_passage = None;
        }
    }

    /// Handle state changes
    async fn change_state(&mut self, new_state: PlaybackState) {
        let old_state = self.state;
        self.state = new_state;

        // Emit state change event
        self.event_bus.emit(WkmpEvent::PlaybackStateChanged {
            old_state,
            new_state,
            timestamp: SystemTime::now(),
        }).ok();
    }
}
```

Components that react to events should subscribe at startup:

```rust
/// Historian - Event Consumer Example
pub struct Historian {
    db: DatabasePool,
    event_rx: broadcast::Receiver<WkmpEvent>,
}

impl Historian {
    pub fn new(
        db: DatabasePool,
        event_bus: Arc<EventBus>,
    ) -> Self {
        Self {
            db,
            event_rx: event_bus.subscribe(),
        }
    }

    /// Main event loop processing events
    ///
    /// Implements REQ-HIST-010: Record passage plays
    pub async fn run(mut self) {
        loop {
            match self.event_rx.recv().await {
                Ok(event) => {
                    if let Err(e) = self.handle_event(event).await {
                        tracing::error!("Failed to handle event: {}", e);
                    }
                }
                Err(broadcast::error::RecvError::Lagged(count)) => {
                    tracing::warn!("Historian lagged {} events, catching up", count);
                    // Continue - we're caught up now
                }
                Err(broadcast::error::RecvError::Closed) => {
                    tracing::info!("Event bus closed, shutting down historian");
                    break;
                }
            }
        }
    }

    async fn handle_event(&self, event: WkmpEvent) -> anyhow::Result<()> {
        match event {
            WkmpEvent::PassageStarted { passage_id, timestamp, .. } => {
                self.record_passage_start(passage_id, timestamp).await?;
            }

            WkmpEvent::PassageCompleted { passage_id, duration_played, completed, .. } => {
                self.record_passage_completion(passage_id, duration_played, completed).await?;
            }

            // Ignore other events
            _ => {}
        }
        Ok(())
    }

    async fn record_passage_start(&self, passage_id: PassageId, timestamp: SystemTime) -> anyhow::Result<()> {
        // Database insert
        sqlx::query!(
            "INSERT INTO play_history (guid, passage_id, timestamp, duration_played, completed)
             VALUES (?, ?, ?, 0, 0)",
            Uuid::new_v4().to_string(),
            passage_id.to_string(),
            timestamp,
        )
        .execute(&self.db)
        .await?;

        Ok(())
    }

    async fn record_passage_completion(&self, passage_id: PassageId, duration: f64, completed: bool) -> anyhow::Result<()> {
        // Update play_history record
        // Update last_played_at timestamps (handled by triggers)
        // Implementation details...
        Ok(())
    }
}
```

## Event Flow Examples

### User Skip Action Flow

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚ User clicks â”‚
â”‚    Skip     â”‚
â””â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”˜
       â”‚
       â–¼
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚ API Handler (Tauri command) â”‚
â”‚ POST /api/skip              â”‚
â””â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
       â”‚
       â”‚ 1. Send Command::Skip via mpsc
       â–¼
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚ Playback Controller  â”‚
â”‚                      â”‚
â”‚ â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â” â”‚
â”‚ â”‚ 1. Stop current  â”‚ â”‚
â”‚ â”‚    passage       â”‚ â”‚
â”‚ â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜ â”‚
â”‚ â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â” â”‚
â”‚ â”‚ 2. Emit:         â”‚ â”‚
â”‚ â”‚  PassageCompletedâ”‚ â”‚
â”‚ â”‚  (skipped=true)  â”‚ â”‚
â”‚ â””â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜ â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¼â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
           â”‚
           â”‚ Event broadcast via EventBus
           â–¼
    â”Œâ”€â”€â”€â”€â”€â”€â”´â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
    â”‚              â”‚             â”‚              â”‚
    â–¼              â–¼             â–¼              â–¼
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”  â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚Historianâ”‚  â”‚   SSE    â”‚  â”‚  State  â”‚  â”‚  Queue   â”‚
â”‚         â”‚  â”‚Broadcast â”‚  â”‚Persist  â”‚  â”‚ Manager  â”‚
â”‚         â”‚  â”‚          â”‚  â”‚         â”‚  â”‚          â”‚
â”‚ Record  â”‚  â”‚ Notify   â”‚  â”‚  Save   â”‚  â”‚ Prepare  â”‚
â”‚ skipped â”‚  â”‚ all UIs  â”‚  â”‚  state  â”‚  â”‚   next   â”‚
â”‚  play   â”‚  â”‚          â”‚  â”‚         â”‚  â”‚ passage  â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜  â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜  â””â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”˜
                                              â”‚
                                              â”‚ 3. Emit: PassageStarted
                                              â–¼
                                    â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
                                    â”‚    EventBus     â”‚
                                    â”‚   (broadcasts   â”‚
                                    â”‚  to same subs)  â”‚
                                    â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

### Automatic Passage Selection Flow

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚  Queue Manager   â”‚
â”‚                  â”‚
â”‚ Detects queue    â”‚
â”‚ depth < 3 or     â”‚
â”‚ time < 15 min    â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
         â”‚
         â”‚ 1. Send SelectionRequest via mpsc (awaits response)
         â–¼
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚   Program Director     â”‚
â”‚                        â”‚
â”‚ 1. Get current flavor  â”‚
â”‚    target from Flavor  â”‚
â”‚    Manager             â”‚
â”‚                        â”‚
â”‚ 2. Calculate           â”‚
â”‚    probabilities       â”‚
â”‚                        â”‚
â”‚ 3. Filter & select     â”‚
â”‚    passage             â”‚
â”‚                        â”‚
â”‚ 4. Return              â”‚
â”‚    Result<PassageId>   â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
         â”‚
         â”‚ Response: Ok(passage_id)
         â–¼
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚  Queue Manager     â”‚
â”‚                    â”‚
â”‚ 1. Add to queue    â”‚
â”‚                    â”‚
â”‚ 2. Emit:           â”‚
â”‚   PassageEnqueued  â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
         â”‚
         â”‚ Event broadcast
         â–¼
    â”Œâ”€â”€â”€â”€â”´â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
    â–¼          â–¼         â–¼
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â” â”Œâ”€â”€â”€â”€â”€â”€â” â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚  SSE    â”‚ â”‚Queue â”‚ â”‚Analyticsâ”‚
â”‚Broadcastâ”‚ â”‚Stats â”‚ â”‚ Track   â”‚
â”‚         â”‚ â”‚      â”‚ â”‚  auto   â”‚
â”‚ Update  â”‚ â”‚Updateâ”‚ â”‚enqueue  â”‚
â”‚  queue  â”‚ â”‚count â”‚ â”‚  event  â”‚
â”‚ display â”‚ â”‚      â”‚ â”‚         â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜ â””â”€â”€â”€â”€â”€â”€â”˜ â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
```

### Temporary Flavor Override Flow

```
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚ User selects     â”‚
â”‚ temporary flavor â”‚
â”‚ override (2 hrs) â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
         â”‚
         â”‚ POST /api/flavor/override
         â–¼
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚  API Handler       â”‚
â”‚                    â”‚
â”‚ 1. Validate input  â”‚
â”‚ 2. Calculate       â”‚
â”‚    expiration time â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
         â”‚
         â”‚ Emit: TemporaryFlavorOverride
         â–¼
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
â”‚    EventBus        â”‚
â”‚   (broadcasts)     â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜
         â”‚
    â”Œâ”€â”€â”€â”€â”´â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”¬â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”
    â–¼          â–¼              â–¼          â–¼
â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â” â”Œâ”€â”€â”€â”€â”€â”€â” â”Œâ”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â” â”Œâ”€â”€â”€â”€â”€â”
â”‚ Queue   â”‚ â”‚Play  â”‚ â”‚  Program     â”‚ â”‚ SSE â”‚
â”‚ Manager â”‚ â”‚back  â”‚ â”‚  Director    â”‚ â”‚     â”‚
â”‚         â”‚ â”‚Ctrl  â”‚ â”‚              â”‚ â”‚Show â”‚
â”‚ Flush   â”‚ â”‚      â”‚ â”‚ Use new      â”‚ â”‚over â”‚
â”‚existing â”‚ â”‚Skip  â”‚ â”‚ target for   â”‚ â”‚ride â”‚
â”‚  queue  â”‚ â”‚remainâ”‚ â”‚ selection    â”‚ â”‚indi â”‚
â”‚         â”‚ â”‚ ing  â”‚ â”‚              â”‚ â”‚catorâ”‚
â”‚Request  â”‚ â”‚time  â”‚ â”‚              â”‚ â”‚     â”‚
â”‚new      â”‚ â”‚on    â”‚ â”‚              â”‚ â”‚     â”‚
â”‚passage  â”‚ â”‚curr  â”‚ â”‚              â”‚ â”‚     â”‚
â”‚         â”‚ â”‚pass  â”‚ â”‚              â”‚ â”‚     â”‚
â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜ â””â”€â”€â”€â”€â”€â”€â”˜ â””â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”˜ â””â”€â”€â”€â”€â”€â”˜
```

## Performance Considerations

### Benchmarks (Raspberry Pi Zero2W)

Measured on Raspberry Pi Zero2W with 10 concurrent subscribers:

| Operation | Latency | Notes |
|-----------|---------|-------|
| Event emission | <0.1ms | Single event, 10 subscribers |
| Event reception | <0.2ms | Per subscriber |
| Channel overhead | ~50 bytes | Per event in buffer |
| Lagged detection | <0.1ms | When subscriber falls behind |

### Capacity Guidelines

**Event Bus Capacity:**
- **Desktop/Full version**: 1000 events
  - Handles bursts during library scan
  - Comfortable margin for development

- **Raspberry Pi/Lite version**: 500 events
  - Adequate for normal operation
  - Memory-conscious for constrained device

- **Testing**: 10-100 events
  - Quickly exposes lag conditions
  - Forces proper error handling

### High-Frequency Event Considerations

**Position Updates (500ms interval):**

Position updates are high-frequency (2 events/second). Consider alternatives:

```rust
// Option 1: Use watch channel for position (recommended)
let (position_tx, position_rx) = watch::channel((0.0, 0.0)); // (position, duration)

// Playback controller updates position
position_tx.send((current_position, duration)).ok();

// UI polls latest position
let (pos, dur) = *position_rx.borrow();

// Option 2: Throttle position events on EventBus
// Only emit every 2 seconds instead of every 500ms
if last_position_event.elapsed() > Duration::from_secs(2) {
    event_bus.emit_lossy(WkmpEvent::PositionUpdate { ... });
}

// Option 3: Separate position channel
// Don't use EventBus for high-frequency data
let (position_tx, _) = broadcast::channel(100);
```

**Recommendation**: Use `tokio::sync::watch` for position updates. Watch channels have "latest value" semantics where readers get the most recent value, not every update.

### Subscriber Performance

Slow subscribers don't block publishers, but they may receive `RecvError::Lagged`:

```rust
// Handle lagged subscribers gracefully
match event_rx.recv().await {
    Ok(event) => {
        // Process event
        self.handle_event(event).await?;
    }

    Err(RecvError::Lagged(count)) => {
        tracing::warn!(
            component = "Historian",
            missed_events = count,
            "Subscriber lagged, missed events"
        );

        // Decision: Can we continue safely?
        if count < 10 {
            // Small lag, continue processing
        } else {
            // Large lag, may need to reconcile state
            self.reconcile_state().await?;
        }
    }

    Err(RecvError::Closed) => {
        // Publisher dropped, shut down
        break;
    }
}
```

## Error Handling

### Publisher Error Handling

```rust
// Critical events - log if no subscribers
match event_bus.emit(WkmpEvent::DatabaseError { ... }) {
    Ok(count) => {
        tracing::debug!("Database error event sent to {} subscribers", count);
    }
    Err(_) => {
        tracing::error!("Critical event but no subscribers listening!");
        // Fallback: direct error logging, notifications, etc.
    }
}

// Non-critical events - lossy send OK
event_bus.emit_lossy(WkmpEvent::PositionUpdate {
    passage_id,
    position: 42.0,
    duration: 180.0,
});
```

### Subscriber Error Handling

```rust
async fn handle_event(&self, event: WkmpEvent) -> anyhow::Result<()> {
    match event {
        WkmpEvent::PassageCompleted { passage_id, duration_played, completed, .. } => {
            // Try to record play
            if let Err(e) = self.record_play(passage_id, duration_played, completed).await {
                // Log error but don't crash event loop
                tracing::error!(
                    passage_id = %passage_id,
                    error = %e,
                    "Failed to record passage completion"
                );

                // Optionally: queue for retry
                self.retry_queue.push((passage_id, duration_played, completed));
            }
        }

        _ => {}
    }

    Ok(())
}
```

## Testing Event-Driven Code

### Testing Event Emission

```rust
#[tokio::test]
async fn test_passage_completion_emits_event() {
    // Arrange
    let event_bus = Arc::new(EventBus::new(10));
    let mut event_rx = event_bus.subscribe();

    let playback = PlaybackController::new(event_bus.clone(), /* ... */);

    // Act
    playback.complete_passage(passage_id, 180.0, true).await;

    // Assert
    let event = tokio::time::timeout(
        Duration::from_millis(100),
        event_rx.recv()
    )
    .await
    .expect("Timeout waiting for event")
    .expect("Event receive error");

    match event {
        WkmpEvent::PassageCompleted { passage_id: id, completed: true, .. } => {
            assert_eq!(id, passage_id);
        }
        _ => panic!("Expected PassageCompleted event, got {:?}", event),
    }
}
```

### Testing Event Handling

```rust
#[tokio::test]
async fn test_historian_records_passage_completion() {
    // Arrange
    let db = setup_test_database().await;
    let event_bus = Arc::new(EventBus::new(10));

    let historian = Historian::new(db.clone(), event_bus.clone());

    // Start historian in background
    let historian_handle = tokio::spawn(async move {
        historian.run().await;
    });

    // Act
    event_bus.emit(WkmpEvent::PassageCompleted {
        passage_id: test_passage_id(),
        duration_played: 180.0,
        completed: true,
        timestamp: SystemTime::now(),
    }).unwrap();

    // Give historian time to process
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Assert
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM play_history WHERE passage_id = ?"
    )
    .bind(test_passage_id().to_string())
    .fetch_one(&db)
    .await
    .unwrap();

    assert_eq!(count, 1);

    // Cleanup
    drop(event_bus); // Closes channel
    historian_handle.await.ok();
}
```

### Testing Multi-Subscriber Scenarios

```rust
#[tokio::test]
async fn test_event_reaches_all_subscribers() {
    let event_bus = Arc::new(EventBus::new(10));

    // Create multiple subscribers
    let mut rx1 = event_bus.subscribe();
    let mut rx2 = event_bus.subscribe();
    let mut rx3 = event_bus.subscribe();

    // Emit event
    let test_event = WkmpEvent::PlaybackStateChanged {
        old_state: PlaybackState::Stopped,
        new_state: PlaybackState::Playing,
        timestamp: SystemTime::now(),
    };

    event_bus.emit(test_event.clone()).unwrap();

    // All subscribers receive it
    assert!(matches!(rx1.try_recv(), Ok(_)));
    assert!(matches!(rx2.try_recv(), Ok(_)));
    assert!(matches!(rx3.try_recv(), Ok(_)));
}
```

### Testing Lag Conditions

```rust
#[tokio::test]
async fn test_slow_subscriber_lags() {
    let event_bus = Arc::new(EventBus::new(5)); // Small capacity
    let mut slow_rx = event_bus.subscribe();

    // Flood with events (more than capacity)
    for i in 0..10 {
        event_bus.emit_lossy(WkmpEvent::PositionUpdate {
            passage_id: test_passage_id(),
            position: i as f64,
            duration: 100.0,
        });
    }

    // First recv might succeed (if fast enough)
    // But eventually will get Lagged error
    let mut got_lagged = false;

    while let Err(e) = slow_rx.recv().await {
        if matches!(e, RecvError::Lagged(_)) {
            got_lagged = true;
            break;
        }
    }

    assert!(got_lagged, "Expected slow subscriber to lag");
}
```

## Application Initialization

### Setting up EventBus at Application Start

```rust
// In main.rs or app initialization
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Create event bus (shared across all components)
    let event_bus = Arc::new(EventBus::new(1000));

    // Create database pool
    let db = create_database_pool().await?;

    // Create command channels
    let (playback_cmd_tx, playback_cmd_rx) = mpsc::channel(100);
    let (selection_req_tx, selection_req_rx) = mpsc::channel(100);

    // Initialize components with event bus and channels
    let playback_controller = PlaybackController::new(
        event_bus.clone(),
        playback_cmd_rx,
    );

    let historian = Historian::new(
        db.clone(),
        event_bus.clone(),
    );

    let queue_manager = QueueManager::new(
        event_bus.clone(),
        selection_req_tx,
    );

    let program_director = ProgramDirector::new(
        db.clone(),
        selection_req_rx,
    );

    let sse_broadcaster = SseBroadcaster::new(
        event_bus.clone(),
    );

    // Spawn component tasks
    tokio::spawn(async move {
        playback_controller.run().await;
    });

    tokio::spawn(async move {
        historian.run().await;
    });

    tokio::spawn(async move {
        queue_manager.run().await;
    });

    tokio::spawn(async move {
        program_director.run().await;
    });

    tokio::spawn(async move {
        sse_broadcaster.run().await;
    });

    // Start Tauri/web server
    start_web_server(event_bus, playback_cmd_tx).await?;

    Ok(())
}
```

## Summary

The WKMP event system provides:

âœ… **Loose coupling** - Components don't need to know about each other

âœ… **Extensibility** - New features add subscribers without modifying existing code

âœ… **Type safety** - Enum-based events with pattern matching

âœ… **Async native** - Built on Tokio broadcast channels

âœ… **Testability** - Easy to test event emission and handling in isolation

âœ… **Performance** - Minimal overhead, suitable for Raspberry Pi Zero2W

âœ… **Multi-user support** - Natural broadcast to all connected UI clients

This design positions WKMP for maintainable, scalable development.

----
End of document - WKMP Event System

**Document Version:** 1.1
**Last Updated:** 2025-10-17

**Change Log:**
- v1.1 (2025-10-17): Added BufferStateChanged event
  - Added new event variant to WkmpEvent enum
  - Added BufferStatus enum (Decoding, Ready, Playing, Exhausted)
  - Added dedicated event section with emission points and use cases
  - Updated Event Categories to include Passage Buffer Manager as publisher
  - Supports architectural decision from wkmp-ap design review (ISSUE-1)
