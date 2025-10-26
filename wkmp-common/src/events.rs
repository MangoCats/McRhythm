//! Event types for WKMP event system
//!
//! Provides shared event definitions and EventBus for all WKMP modules.

use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use uuid::Uuid;

/// WKMP event types
///
/// **DRY Pattern:** Shared across all 5 WKMP modules (wkmp-ap, wkmp-ui, wkmp-pd, wkmp-ai, wkmp-le)
///
/// Events are broadcast via EventBus and can be serialized for SSE transmission.
/// Per SPEC011: All events use this central enum for type safety and exhaustive matching.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WkmpEvent {
    /// Playback state changed (Playing ↔ Paused)
    ///
    /// Triggers:
    /// - SSE: Update UI controls
    /// - State Persistence: Save current state
    /// - Platform Integration: Update MPRIS/media keys
    PlaybackStateChanged {
        old_state: PlaybackState,
        new_state: PlaybackState,
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Passage started playing
    ///
    /// Triggers:
    /// - Historian: Record play start
    /// - SSE: Update all connected UIs
    /// - Lyrics Display: Show passage lyrics
    PassageStarted {
        passage_id: Uuid,
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Passage completed or skipped
    ///
    /// Triggers:
    /// - Historian: Record play completion
    /// - Queue Manager: Advance queue
    /// - SSE: Update UI playback state
    PassageCompleted {
        passage_id: Uuid,
        duration_played: f64,
        completed: bool, // false if skipped
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Current song within passage changed
    ///
    /// NOTE: Distinct from PassageStarted (which is for passage transitions).
    /// Fires when crossing song boundaries within a single passage.
    ///
    /// Triggers:
    /// - UI: Update album art display
    /// - UI: Reset album rotation timer if song has multiple albums
    /// - UI: Update now playing song information
    CurrentSongChanged {
        passage_id: Uuid,
        song_id: Option<Uuid>,
        song_albums: Vec<Uuid>, // All albums for this song, ordered by release date (newest first)
        position_ms: u64,
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Playback progress update
    ///
    /// Emitted periodically during playback (configurable frequency, default: 5000ms).
    /// Also emitted once when Pause/Play initiated.
    ///
    /// NOTE: NOT persisted to database during playback (only transmitted via SSE).
    /// Database persistence only on clean shutdown via settings.last_played_position_ticks.
    ///
    /// Triggers:
    /// - SSE: Update progress bar
    PlaybackProgress {
        passage_id: Uuid,
        position_ms: u64,
        duration_ms: u64,
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Queue changed
    ///
    /// Triggers:
    /// - SSE: Update queue display
    /// - Auto-replenishment: Check if refill needed
    QueueChanged {
        queue: Vec<Uuid>,
        trigger: QueueChangeTrigger,
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Queue state update (full queue contents for SSE)
    /// [SSE-UI-020] Queue Updates
    QueueStateUpdate {
        timestamp: chrono::DateTime<chrono::Utc>,
        queue: Vec<QueueEntryInfo>,
    },

    /// Playback position update (sent every 1s during playback)
    /// [SSE-UI-030] Playback Position Updates
    PlaybackPosition {
        timestamp: chrono::DateTime<chrono::Utc>,
        passage_id: Uuid,
        position_ms: u64,
        duration_ms: u64,
        playing: bool,
    },

    /// Passage enqueued (added to queue)
    ///
    /// Triggers:
    /// - SSE: Animate new queue entry
    /// - Analytics: Track auto vs manual enqueue
    PassageEnqueued {
        passage_id: Uuid,
        position: usize,
        source: EnqueueSource,
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Passage dequeued (removed from queue)
    ///
    /// Triggers:
    /// - SSE: Update queue display
    PassageDequeued {
        passage_id: Uuid,
        was_playing: bool,
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Queue became empty
    ///
    /// NOTE: This does NOT change Play/Pause state automatically
    ///
    /// Triggers:
    /// - SSE: Update UI to show empty queue state
    /// - UI: May show "Queue Empty" message
    /// - Automatic selector: Already stopped (no valid candidates)
    QueueEmpty {
        playback_state: PlaybackState, // Current Play/Pause state (unchanged)
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Volume changed
    ///
    /// Triggers:
    /// - SSE: Update volume slider
    /// - State Persistence: Save volume preference
    VolumeChanged {
        old_volume: f32, // 0.0-1.0 (system-level scale for precision)
        new_volume: f32, // 0.0-1.0 (system-level scale for precision)
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Buffer state changed for a passage
    ///
    /// Purpose: Notify clients of passage buffer decode/playback state changes
    /// for monitoring and debugging.
    ///
    /// Triggers:
    /// - Developer UI: Show buffer state transitions for debugging
    /// - Performance monitoring: Track decode speed vs. playback speed
    /// - UI display: Show decode progress for large files
    BufferStateChanged {
        passage_id: Uuid,
        old_state: BufferStatus,
        new_state: BufferStatus,
        decode_progress_percent: Option<f32>, // Only for Decoding state
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// User performed an action (satisfies REQ-CF-082, REQ-CF-082A)
    ///
    /// Used for multi-user synchronization and edge case handling:
    /// - Skip throttling (5-second window, REQ-CF-085A, REQ-CF-085B, REQ-CF-085C)
    /// - Concurrent operation handling
    ///
    /// Triggers:
    /// - SSE: Broadcast to all other connected clients
    /// - Skip Throttle: Track recent skip actions
    /// - Analytics: User interaction tracking
    UserAction {
        action: UserActionType,
        user_id: String, // User's persistent UUID
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// User liked a passage (Full/Lite versions only)
    ///
    /// Triggers:
    /// - Database: Record like associated with user UUID
    /// - Taste Manager: Update user's taste profile
    /// - SSE: Update like button state for all connected clients
    PassageLiked {
        passage_id: Uuid,
        user_id: Uuid, // UUID of user who liked (may be Anonymous UUID)
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// User disliked a passage (Full/Lite versions only)
    ///
    /// Triggers:
    /// - Database: Record dislike associated with user UUID
    /// - Taste Manager: Update user's taste profile
    /// - SSE: Update dislike button state for all connected clients
    /// - Program Director: Adjust selection probability (Phase 2)
    PassageDisliked {
        passage_id: Uuid,
        user_id: Uuid, // UUID of user who disliked (may be Anonymous UUID)
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Temporary flavor override set by user
    ///
    /// Implements REQ-FLV-020: Temporary override behavior
    ///
    /// Triggers:
    /// - Queue Manager: Flush existing queue
    /// - Playback Controller: Skip remaining time on current passage
    /// - Program Director: Use new target for selection
    /// - SSE: Show override indicator in UI
    TemporaryFlavorOverride {
        target_flavor: Vec<f64>, // FlavorVector: 8-dimensional AcousticBrainz vector
        expiration: chrono::DateTime<chrono::Utc>,
        duration_seconds: u64,
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Temporary flavor override expired
    ///
    /// Triggers:
    /// - Program Director: Revert to timeslot-based target
    /// - SSE: Remove override indicator
    TemporaryFlavorOverrideExpired {
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Timeslot changed (e.g., midnight → morning)
    ///
    /// NOTE: Does NOT affect currently queued passages (REQ-FLV-030)
    ///
    /// Triggers:
    /// - Program Director: Update target flavor for new selections
    /// - SSE: Update current timeslot indicator
    TimeslotChanged {
        old_timeslot_id: Uuid,
        new_timeslot_id: Uuid,
        new_target_flavor: Vec<f64>, // FlavorVector
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Network connectivity status changed
    ///
    /// Implements REQ-NET-010: Network error handling
    ///
    /// Triggers:
    /// - External API clients: Pause/resume requests
    /// - SSE: Show offline indicator
    NetworkStatusChanged {
        available: bool,
        retry_count: u32,
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Library scan completed (Full version only)
    ///
    /// Triggers:
    /// - SSE: Update library stats
    /// - Program Director: Refresh available passages
    LibraryScanCompleted {
        files_added: usize,
        files_updated: usize,
        files_removed: usize,
        duration_seconds: u64,
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Database error occurred
    ///
    /// Triggers:
    /// - Error logging
    /// - SSE: Show error notification
    /// - Retry logic: Attempt recovery
    DatabaseError {
        operation: String,
        error: String,
        retry_attempted: bool,
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Initial state sent on SSE connection
    /// [SSE-UI-050] Initial State on Connection
    InitialState {
        timestamp: chrono::DateTime<chrono::Utc>,
        queue: Vec<QueueEntryInfo>,
        position: Option<PlaybackPositionInfo>,
        volume: f32,
    },

    /// Crossfade started
    CrossfadeStarted {
        from_passage_id: Uuid,
        to_passage_id: Uuid,
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Buffer chain status update (sent every 1s when data changes)
    /// Shows decoder-resampler-fade-buffer chains for monitoring
    BufferChainStatus {
        timestamp: chrono::DateTime<chrono::Utc>,
        chains: Vec<BufferChainInfo>,
    },

    /// Endpoint discovered during decode
    /// **[DBD-DEC-095]** Emitted when decoder discovers actual file duration for undefined endpoints
    /// Sent by decoder → buffer manager when passage has NULL end_time_ticks
    EndpointDiscovered {
        queue_entry_id: Uuid,
        actual_end_ticks: i64,
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Pipeline validation succeeded (periodic check)
    /// **[ARCH-AUTO-VAL-001]** Automatic validation service - success result
    ValidationSuccess {
        timestamp: chrono::DateTime<chrono::Utc>,
        passage_count: usize,
        total_decoder_samples: u64,
        total_buffer_written: u64,
        total_buffer_read: u64,
        total_mixer_frames: u64,
    },

    /// Pipeline validation failed (conservation law violated)
    /// **[ARCH-AUTO-VAL-001]** Automatic validation service - failure result
    ValidationFailure {
        timestamp: chrono::DateTime<chrono::Utc>,
        passage_count: usize,
        total_decoder_samples: u64,
        total_buffer_written: u64,
        total_buffer_read: u64,
        total_mixer_frames: u64,
        errors: Vec<String>,
    },

    /// Pipeline validation warning (approaching tolerance threshold)
    /// **[ARCH-AUTO-VAL-001]** Automatic validation service - warning result (>80% of tolerance)
    ValidationWarning {
        timestamp: chrono::DateTime<chrono::Utc>,
        passage_count: usize,
        total_decoder_samples: u64,
        total_buffer_written: u64,
        total_buffer_read: u64,
        total_mixer_frames: u64,
        warnings: Vec<String>,
    },
}

/// Queue entry information for SSE events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueEntryInfo {
    pub queue_entry_id: Uuid,
    pub passage_id: Option<Uuid>,
    pub file_path: String,
}

/// Playback position information for SSE events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackPositionInfo {
    pub passage_id: Uuid,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub playing: bool,
}

/// Buffer chain information for SSE monitoring
///
/// **[DBD-OV-040]** Full pipeline visibility: Decoder → Resampler → Fade → Buffer → Mixer
/// **[DBD-OV-080]** Passage-based chain association (not position-based)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BufferChainInfo {
    pub slot_index: usize,
    pub queue_entry_id: Option<Uuid>,
    pub passage_id: Option<Uuid>,
    pub file_name: Option<String>,

    // Queue position tracking **[DBD-OV-060]** [DBD-OV-070]**
    /// Position in queue (1 = now playing, 2 = next, 3+ = pre-buffering, None = idle)
    pub queue_position: Option<usize>,

    // Decoder stage visibility
    /// Decoder state: Idle, Decoding, Paused
    pub decoder_state: Option<DecoderState>,
    /// Decode progress (0-100%)
    pub decode_progress_percent: Option<u8>,
    /// Currently being processed by decoder pool
    pub is_actively_decoding: Option<bool>,

    // Resampler stage visibility **[DBD-OV-010]** **[DBD-RSMP-010]**
    /// Source file sample rate (Hz)
    pub source_sample_rate: Option<u32>,
    /// Resampler active (true if source rate != working rate)
    pub resampler_active: Option<bool>,
    /// Target sample rate (always 44100 Hz)
    #[serde(default = "default_working_sample_rate")]
    pub target_sample_rate: u32,

    // Fade handler stage visibility **[DBD-FADE-010]**
    /// Current fade stage: PreStart, FadeIn, Body, FadeOut, PostEnd
    pub fade_stage: Option<FadeStage>,

    // Buffer stage visibility **[DBD-BUF-020]** through **[DBD-BUF-060]**
    /// Buffer state: Empty, Filling, Ready, Playing, Finished
    pub buffer_state: Option<String>,
    pub buffer_fill_percent: f32,
    pub buffer_fill_samples: usize,
    pub buffer_capacity_samples: usize,
    /// Total frames written to buffer (cumulative, for decode progress tracking)
    pub total_decoded_frames: usize,

    // Mixer stage visibility
    pub playback_position_frames: usize,
    pub playback_position_ms: u64,
    pub duration_ms: Option<u64>,
    pub is_active_in_mixer: bool,
    pub mixer_role: String, // "Idle", "Current", "Next", "Crossfading"
    pub started_at: Option<String>,
}

fn default_working_sample_rate() -> u32 {
    44100 // **[DBD-PARAM-020]** working_sample_rate default
}

impl BufferChainInfo {
    /// Create an idle chain info for unused slots
    pub fn idle(slot_index: usize) -> Self {
        Self {
            slot_index,
            queue_entry_id: None,
            passage_id: None,
            file_name: None,
            queue_position: None,
            decoder_state: Some(DecoderState::Idle),
            decode_progress_percent: Some(0),
            is_actively_decoding: Some(false),
            source_sample_rate: None,
            resampler_active: Some(false),
            target_sample_rate: 44100,
            fade_stage: None,
            buffer_state: Some("Idle".to_string()),
            buffer_fill_percent: 0.0,
            buffer_fill_samples: 0,
            buffer_capacity_samples: 0,
            total_decoded_frames: 0,
            playback_position_frames: 0,
            playback_position_ms: 0,
            duration_ms: None,
            is_active_in_mixer: false,
            mixer_role: "Idle".to_string(),
            started_at: None,
        }
    }
}

/// Decoder state enumeration
/// **[DBD-DEC-030]** Decoder pauses when buffer full, resumes as data consumed
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum DecoderState {
    /// Waiting for work
    Idle,
    /// Actively decoding audio
    Decoding,
    /// Paused (buffer full or lower priority)
    Paused,
}

impl std::fmt::Display for DecoderState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecoderState::Idle => write!(f, "Idle"),
            DecoderState::Decoding => write!(f, "Decoding"),
            DecoderState::Paused => write!(f, "Paused"),
        }
    }
}

/// Fade processing stage enumeration
/// **[DBD-FADE-010]** through **[DBD-FADE-060]**
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum FadeStage {
    /// Before passage start (discarding samples) **[DBD-FADE-020]**
    PreStart,
    /// Applying fade-in curve **[DBD-FADE-030]**
    FadeIn,
    /// No fade applied (passthrough) **[DBD-FADE-040]**
    Body,
    /// Applying fade-out curve **[DBD-FADE-050]**
    FadeOut,
    /// After passage end (decode complete) **[DBD-FADE-060]**
    PostEnd,
}

impl std::fmt::Display for FadeStage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FadeStage::PreStart => write!(f, "PreStart"),
            FadeStage::FadeIn => write!(f, "FadeIn"),
            FadeStage::Body => write!(f, "Body"),
            FadeStage::FadeOut => write!(f, "FadeOut"),
            FadeStage::PostEnd => write!(f, "PostEnd"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PlaybackState {
    Playing,
    Paused,
}

impl std::fmt::Display for PlaybackState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlaybackState::Playing => write!(f, "playing"),
            PlaybackState::Paused => write!(f, "paused"),
        }
    }
}

impl WkmpEvent {
    /// Get event type as string for filtering
    pub fn event_type(&self) -> &str {
        match self {
            WkmpEvent::PlaybackStateChanged { .. } => "PlaybackStateChanged",
            WkmpEvent::PassageStarted { .. } => "PassageStarted",
            WkmpEvent::PassageCompleted { .. } => "PassageCompleted",
            WkmpEvent::CurrentSongChanged { .. } => "CurrentSongChanged",
            WkmpEvent::PlaybackProgress { .. } => "PlaybackProgress",
            WkmpEvent::QueueChanged { .. } => "QueueChanged",
            WkmpEvent::QueueStateUpdate { .. } => "QueueStateUpdate",
            WkmpEvent::PlaybackPosition { .. } => "PlaybackPosition",
            WkmpEvent::PassageEnqueued { .. } => "PassageEnqueued",
            WkmpEvent::PassageDequeued { .. } => "PassageDequeued",
            WkmpEvent::QueueEmpty { .. } => "QueueEmpty",
            WkmpEvent::VolumeChanged { .. } => "VolumeChanged",
            WkmpEvent::BufferStateChanged { .. } => "BufferStateChanged",
            WkmpEvent::UserAction { .. } => "UserAction",
            WkmpEvent::PassageLiked { .. } => "PassageLiked",
            WkmpEvent::PassageDisliked { .. } => "PassageDisliked",
            WkmpEvent::TemporaryFlavorOverride { .. } => "TemporaryFlavorOverride",
            WkmpEvent::TemporaryFlavorOverrideExpired { .. } => "TemporaryFlavorOverrideExpired",
            WkmpEvent::TimeslotChanged { .. } => "TimeslotChanged",
            WkmpEvent::NetworkStatusChanged { .. } => "NetworkStatusChanged",
            WkmpEvent::LibraryScanCompleted { .. } => "LibraryScanCompleted",
            WkmpEvent::DatabaseError { .. } => "DatabaseError",
            WkmpEvent::InitialState { ..} => "InitialState",
            WkmpEvent::CrossfadeStarted { .. } => "CrossfadeStarted",
            WkmpEvent::BufferChainStatus { .. } => "BufferChainStatus",
            WkmpEvent::EndpointDiscovered { .. } => "EndpointDiscovered",
            WkmpEvent::ValidationSuccess { .. } => "ValidationSuccess",
            WkmpEvent::ValidationFailure { .. } => "ValidationFailure",
            WkmpEvent::ValidationWarning { .. } => "ValidationWarning",
        }
    }
}

// ========================================
// Supporting Enums
// ========================================

/// Buffer status for passage decode/playback lifecycle
///
/// Per SPEC016 Buffers:
/// - DBD-BUF-020: Empty on start
/// - DBD-BUF-030: Mixer can't read empty buffer
/// - DBD-BUF-040: Returns last sample if empty
/// - DBD-BUF-050: Decoder pauses when nearly full
/// - DBD-BUF-060: Informs queue on completion
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum BufferStatus {
    /// Buffer currently being populated from audio file
    Decoding,
    /// Buffer fully decoded and ready for playback
    Ready,
    /// Buffer currently being read for audio output
    Playing,
    /// Buffer playback completed
    Exhausted,
}

impl std::fmt::Display for BufferStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BufferStatus::Decoding => write!(f, "Decoding"),
            BufferStatus::Ready => write!(f, "Ready"),
            BufferStatus::Playing => write!(f, "Playing"),
            BufferStatus::Exhausted => write!(f, "Exhausted"),
        }
    }
}

/// User action types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
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

impl std::fmt::Display for UserActionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserActionType::Skip => write!(f, "Skip"),
            UserActionType::Play => write!(f, "Play"),
            UserActionType::Pause => write!(f, "Pause"),
            UserActionType::Seek => write!(f, "Seek"),
            UserActionType::VolumeChange => write!(f, "VolumeChange"),
            UserActionType::QueueAdd => write!(f, "QueueAdd"),
            UserActionType::QueueRemove => write!(f, "QueueRemove"),
            UserActionType::Like => write!(f, "Like"),
            UserActionType::Dislike => write!(f, "Dislike"),
            UserActionType::TemporaryOverride => write!(f, "TemporaryOverride"),
        }
    }
}

/// Why the queue changed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum QueueChangeTrigger {
    AutomaticReplenishment,
    UserEnqueue,
    UserDequeue,
    PassageCompletion,
    TemporaryOverride,
}

impl std::fmt::Display for QueueChangeTrigger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueueChangeTrigger::AutomaticReplenishment => write!(f, "AutomaticReplenishment"),
            QueueChangeTrigger::UserEnqueue => write!(f, "UserEnqueue"),
            QueueChangeTrigger::UserDequeue => write!(f, "UserDequeue"),
            QueueChangeTrigger::PassageCompletion => write!(f, "PassageCompletion"),
            QueueChangeTrigger::TemporaryOverride => write!(f, "TemporaryOverride"),
        }
    }
}

/// How a passage was enqueued
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum EnqueueSource {
    Automatic,
    Manual,
}

impl std::fmt::Display for EnqueueSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EnqueueSource::Automatic => write!(f, "Automatic"),
            EnqueueSource::Manual => write!(f, "Manual"),
        }
    }
}

// ========================================
// EventBus Implementation
// ========================================

/// Central event distribution bus for application-wide events
///
/// The EventBus uses tokio::broadcast internally, providing:
/// - Non-blocking publish (slow subscribers don't block producers)
/// - Multiple concurrent subscribers
/// - Automatic cleanup when subscribers drop
/// - Lagged message detection for slow subscribers
///
/// **DRY Pattern:** Shared across all 5 WKMP modules (wkmp-ap, wkmp-ui, wkmp-pd, wkmp-ai, wkmp-le)
///
/// # Capacity Recommendations
///
/// Per SPEC011:
/// - Development/Desktop: 1000
/// - Raspberry Pi Zero2W: 500
/// - Testing: 10-100
///
/// # Examples
///
/// ```
/// use wkmp_common::events::{EventBus, WkmpEvent, PlaybackState};
/// use std::sync::Arc;
///
/// let event_bus = Arc::new(EventBus::new(1000));
///
/// // Subscribe to events
/// let mut rx = event_bus.subscribe();
///
/// // Emit an event
/// event_bus.emit(WkmpEvent::PlaybackStateChanged {
///     old_state: PlaybackState::Paused,
///     new_state: PlaybackState::Playing,
///     timestamp: chrono::Utc::now(),
/// }).ok();
///
/// // Receive events (in async context)
/// // while let Ok(event) = rx.recv().await {
/// //     match event {
/// //         WkmpEvent::PlaybackStateChanged { .. } => {
/// //             // Handle state change
/// //         }
/// //         _ => {}
/// //     }
/// // }
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
    ///   Recommended values (per SPEC011):
    ///   - Development/Desktop: 1000
    ///   - Raspberry Pi Zero2W: 500
    ///   - Testing: 10-100
    ///
    /// # Examples
    ///
    /// ```
    /// use wkmp_common::events::EventBus;
    ///
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
    /// use wkmp_common::events::EventBus;
    /// use std::sync::Arc;
    ///
    /// let event_bus = Arc::new(EventBus::new(1000));
    /// let mut rx = event_bus.subscribe();
    ///
    /// // In async context:
    /// // tokio::spawn(async move {
    /// //     while let Ok(event) = rx.recv().await {
    /// //         println!("Received event: {:?}", event);
    /// //     }
    /// // });
    /// ```
    pub fn subscribe(&self) -> broadcast::Receiver<WkmpEvent> {
        self.tx.subscribe()
    }

    /// Emit an event to all subscribers
    ///
    /// Returns `Ok(subscriber_count)` if at least one subscriber exists.
    /// Returns `Err` if no subscribers are listening.
    ///
    /// Per SPEC011 EVT-ERR-PROP-010: The first component to detect an error emits the event.
    /// Errors are NOT propagated through multiple layers.
    ///
    /// # Examples
    ///
    /// ```
    /// use wkmp_common::events::{EventBus, WkmpEvent, PlaybackState};
    ///
    /// let event_bus = EventBus::new(100);
    ///
    /// // Critical event - log if no subscribers
    /// let event = WkmpEvent::PlaybackStateChanged {
    ///     old_state: PlaybackState::Paused,
    ///     new_state: PlaybackState::Playing,
    ///     timestamp: chrono::Utc::now(),
    /// };
    ///
    /// if let Err(_) = event_bus.emit(event) {
    ///     eprintln!("Warning: No subscribers for critical event");
    /// }
    /// ```
    pub fn emit(
        &self,
        event: WkmpEvent,
    ) -> Result<usize, broadcast::error::SendError<WkmpEvent>> {
        self.tx.send(event)
    }

    /// Emit an event, ignoring if no subscribers are listening
    ///
    /// This is useful for non-critical events where it's acceptable if
    /// no component is currently listening.
    ///
    /// Per SPEC011 EVT-ERR-PROP-020: If emit fails (no subscribers), log error locally, continue operation.
    ///
    /// # Examples
    ///
    /// ```
    /// use wkmp_common::events::{EventBus, WkmpEvent};
    /// use uuid::Uuid;
    ///
    /// let event_bus = EventBus::new(100);
    ///
    /// // Position updates - OK if no one is listening
    /// event_bus.emit_lossy(WkmpEvent::PlaybackProgress {
    ///     passage_id: Uuid::new_v4(),
    ///     position_ms: 42000,
    ///     duration_ms: 180000,
    ///     timestamp: chrono::Utc::now(),
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

#[cfg(test)]
mod tests {
    use super::*;

    /// **[SPEC020-TEST-010]** Test queue position semantics (0-based indexing per SPEC008)
    ///
    /// Verifies that queue_position follows SPEC008 convention:
    /// - Position 0 = "now playing" [SPEC020-MONITOR-050]
    /// - Position 1 = "up next"
    /// - Position 2+ = queued passages
    /// - None = idle chain
    #[test]
    fn test_buffer_chain_info_queue_position_semantics() {
        // Position 0 = "now playing" [SPEC020-MONITOR-050]
        let chain_now_playing = BufferChainInfo {
            slot_index: 0,
            queue_entry_id: Some(uuid::Uuid::new_v4()),
            passage_id: Some(uuid::Uuid::new_v4()),
            file_name: Some("test.mp3".to_string()),
            queue_position: Some(0),  // 0-BASED: now playing
            decoder_state: Some(DecoderState::Decoding),
            decode_progress_percent: Some(45),
            is_actively_decoding: Some(true),
            source_sample_rate: Some(44100),
            resampler_active: Some(false),
            target_sample_rate: 44100,
            fade_stage: Some(FadeStage::Body),
            buffer_state: Some("Playing".to_string()),
            buffer_fill_percent: 65.5,
            buffer_fill_samples: 28900,
            buffer_capacity_samples: 44100,
            total_decoded_frames: 41000,
            playback_position_frames: 12000,
            playback_position_ms: 272,
            duration_ms: Some(180000),
            is_active_in_mixer: true,
            mixer_role: "Current".to_string(),
            started_at: Some("2025-10-20T12:00:00Z".to_string()),
        };

        assert_eq!(
            chain_now_playing.queue_position,
            Some(0),
            "[SPEC008] Position 0 should be 'now playing'"
        );
        assert!(chain_now_playing.is_active_in_mixer, "Now playing should be active in mixer");
        assert_eq!(chain_now_playing.mixer_role, "Current");

        // Position 1 = "up next" [SPEC020-MONITOR-050]
        let chain_up_next = BufferChainInfo {
            slot_index: 1,
            queue_entry_id: Some(uuid::Uuid::new_v4()),
            passage_id: Some(uuid::Uuid::new_v4()),
            file_name: Some("next.mp3".to_string()),
            queue_position: Some(1),  // 0-BASED: up next
            decoder_state: Some(DecoderState::Decoding),
            decode_progress_percent: Some(12),
            is_actively_decoding: Some(true),
            source_sample_rate: Some(48000),
            resampler_active: Some(true),
            target_sample_rate: 44100,
            fade_stage: Some(FadeStage::PreStart),
            buffer_state: Some("Filling".to_string()),
            buffer_fill_percent: 15.2,
            buffer_fill_samples: 6703,
            buffer_capacity_samples: 44100,
            total_decoded_frames: 6703,
            playback_position_frames: 0,
            playback_position_ms: 0,
            duration_ms: Some(240000),
            is_active_in_mixer: false,
            mixer_role: "Idle".to_string(),
            started_at: None,
        };

        assert_eq!(
            chain_up_next.queue_position,
            Some(1),
            "[SPEC008] Position 1 should be 'up next'"
        );
        assert!(chain_up_next.resampler_active.unwrap(), "48kHz source should require resampling");

        // Position 2+ = queued passages [SPEC020-MONITOR-050]
        let chain_queued = BufferChainInfo {
            slot_index: 2,
            queue_entry_id: Some(uuid::Uuid::new_v4()),
            passage_id: Some(uuid::Uuid::new_v4()),
            file_name: Some("queued.mp3".to_string()),
            queue_position: Some(2),  // 0-BASED: queued
            decoder_state: Some(DecoderState::Decoding),
            decode_progress_percent: Some(5),
            is_actively_decoding: Some(true),
            source_sample_rate: Some(44100),
            resampler_active: Some(false),
            target_sample_rate: 44100,
            fade_stage: Some(FadeStage::PreStart),
            buffer_state: Some("Filling".to_string()),
            buffer_fill_percent: 3.1,
            buffer_fill_samples: 1367,
            buffer_capacity_samples: 44100,
            total_decoded_frames: 1367,
            playback_position_frames: 0,
            playback_position_ms: 0,
            duration_ms: Some(200000),
            is_active_in_mixer: false,
            mixer_role: "Idle".to_string(),
            started_at: None,
        };

        assert_eq!(
            chain_queued.queue_position,
            Some(2),
            "[SPEC008] Position 2+ should be queued passages"
        );
        assert!(!chain_queued.is_active_in_mixer, "Queued passages should not be in mixer");

        // None = idle chain [SPEC020-MONITOR-050]
        let idle_chain = BufferChainInfo::idle(5);
        assert_eq!(
            idle_chain.queue_position,
            None,
            "[SPEC008] Idle chain should have queue_position None"
        );
        assert_eq!(idle_chain.buffer_state, Some("Idle".to_string()));
        assert_eq!(idle_chain.buffer_fill_percent, 0.0);
        assert!(!idle_chain.is_active_in_mixer);
    }

    /// **[SPEC020-TEST-020]** Test BufferChainInfo::idle() constructor
    ///
    /// Verifies that idle chains are properly initialized with default values
    #[test]
    fn test_buffer_chain_info_idle_constructor() {
        for slot in 0..12 {
            let idle = BufferChainInfo::idle(slot);

            assert_eq!(idle.slot_index, slot, "slot_index should match");
            assert_eq!(idle.queue_entry_id, None, "idle chain has no queue_entry_id");
            assert_eq!(idle.passage_id, None, "idle chain has no passage_id");
            assert_eq!(idle.file_name, None, "idle chain has no file_name");
            assert_eq!(idle.queue_position, None, "idle chain has no queue_position");
            assert_eq!(idle.decoder_state, Some(DecoderState::Idle));
            assert_eq!(idle.decode_progress_percent, Some(0));
            assert_eq!(idle.is_actively_decoding, Some(false));
            assert_eq!(idle.source_sample_rate, None);
            assert_eq!(idle.resampler_active, Some(false));
            assert_eq!(idle.target_sample_rate, 44100, "target rate always 44100 Hz");
            assert_eq!(idle.fade_stage, None);
            assert_eq!(idle.buffer_state, Some("Idle".to_string()));
            assert_eq!(idle.buffer_fill_percent, 0.0);
            assert_eq!(idle.buffer_fill_samples, 0);
            assert_eq!(idle.buffer_capacity_samples, 0);
            assert_eq!(idle.playback_position_frames, 0);
            assert_eq!(idle.playback_position_ms, 0);
            assert_eq!(idle.duration_ms, None);
            assert!(!idle.is_active_in_mixer);
            assert_eq!(idle.mixer_role, "Idle");
            assert_eq!(idle.started_at, None);
        }
    }

    /// **[SPEC020-TEST-030]** Test BufferChainInfo JSON serialization for SSE
    ///
    /// Verifies that BufferChainInfo serializes correctly for SSE BufferChainStatus events
    #[test]
    fn test_buffer_chain_info_serialization() {
        let chain = BufferChainInfo {
            slot_index: 0,
            queue_entry_id: Some(uuid::Uuid::from_u128(0x12345678_1234_1234_1234_123456789abc)),
            passage_id: Some(uuid::Uuid::from_u128(0x87654321_4321_4321_4321_cba987654321)),
            file_name: Some("test.mp3".to_string()),
            queue_position: Some(0),  // 0-based: now playing
            decoder_state: Some(DecoderState::Decoding),
            decode_progress_percent: Some(45),
            is_actively_decoding: Some(true),
            source_sample_rate: Some(48000),
            resampler_active: Some(true),
            target_sample_rate: 44100,
            fade_stage: Some(FadeStage::FadeIn),
            buffer_state: Some("Playing".to_string()),
            buffer_fill_percent: 65.5,
            buffer_fill_samples: 28900,
            buffer_capacity_samples: 44100,
            total_decoded_frames: 41000,
            playback_position_frames: 12000,
            playback_position_ms: 272,
            duration_ms: Some(180000),
            is_active_in_mixer: true,
            mixer_role: "Current".to_string(),
            started_at: Some("2025-10-20T12:00:00Z".to_string()),
        };

        // Serialize to JSON
        let json = serde_json::to_string(&chain).expect("Serialization should succeed");

        // Verify key fields in JSON
        assert!(json.contains("\"slot_index\":0"), "slot_index should serialize");
        assert!(json.contains("\"queue_position\":0"), "queue_position 0 (now playing) should serialize");
        assert!(json.contains("\"buffer_fill_percent\":65.5"), "buffer_fill_percent should serialize");
        assert!(json.contains("\"target_sample_rate\":44100"), "target_sample_rate should serialize");
        assert!(json.contains("\"mixer_role\":\"Current\""), "mixer_role should serialize");
        assert!(json.contains("\"decoder_state\":\"Decoding\""), "decoder_state should serialize");
        assert!(json.contains("\"fade_stage\":\"FadeIn\""), "fade_stage should serialize");

        // Deserialize back
        let deserialized: BufferChainInfo = serde_json::from_str(&json).expect("Deserialization should succeed");

        assert_eq!(deserialized.slot_index, 0);
        assert_eq!(deserialized.queue_position, Some(0));
        assert_eq!(deserialized.buffer_fill_percent, 65.5);
        assert_eq!(deserialized.mixer_role, "Current");
    }

    /// **[SPEC020-TEST-040]** Test DecoderState enum variants
    #[test]
    fn test_decoder_state_enum() {
        assert_eq!(DecoderState::Idle.to_string(), "Idle");
        assert_eq!(DecoderState::Decoding.to_string(), "Decoding");
        assert_eq!(DecoderState::Paused.to_string(), "Paused");

        // Test equality
        assert_eq!(DecoderState::Idle, DecoderState::Idle);
        assert_ne!(DecoderState::Idle, DecoderState::Decoding);
    }

    /// **[SPEC020-TEST-050]** Test FadeStage enum variants
    #[test]
    fn test_fade_stage_enum() {
        assert_eq!(FadeStage::PreStart.to_string(), "PreStart");
        assert_eq!(FadeStage::FadeIn.to_string(), "FadeIn");
        assert_eq!(FadeStage::Body.to_string(), "Body");
        assert_eq!(FadeStage::FadeOut.to_string(), "FadeOut");
        assert_eq!(FadeStage::PostEnd.to_string(), "PostEnd");

        // Test equality
        assert_eq!(FadeStage::Body, FadeStage::Body);
        assert_ne!(FadeStage::FadeIn, FadeStage::FadeOut);
    }

    /// **[SPEC020-TEST-060]** Test BufferChainStatus SSE event structure
    #[test]
    fn test_buffer_chain_status_sse_event() {
        use chrono::Utc;

        let chains = vec![
            BufferChainInfo::idle(0),
            BufferChainInfo::idle(1),
        ];

        let event = WkmpEvent::BufferChainStatus {
            timestamp: Utc::now(),
            chains: chains.clone(),
        };

        assert_eq!(event.event_type(), "BufferChainStatus");

        // Serialize event
        let json = serde_json::to_string(&event).expect("Event serialization should succeed");
        assert!(json.contains("\"type\":\"BufferChainStatus\""));
        assert!(json.contains("\"chains\":"));

        // Deserialize back
        let deserialized: WkmpEvent = serde_json::from_str(&json).expect("Event deserialization should succeed");
        match deserialized {
            WkmpEvent::BufferChainStatus { chains: deserialized_chains, .. } => {
                assert_eq!(deserialized_chains.len(), 2);
                assert_eq!(deserialized_chains[0].slot_index, 0);
                assert_eq!(deserialized_chains[1].slot_index, 1);
            }
            _ => panic!("Wrong event type deserialized"),
        }
    }
}
