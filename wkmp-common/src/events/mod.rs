//! Event types for WKMP event system
//!
//! Provides shared event definitions and EventBus for all WKMP modules.

// Sub-modules (supporting types)
mod playback_types;
mod queue_types;
mod import_types;
mod shared_types;

// Re-export all types for backward compatibility
pub use playback_types::{BufferStatus, DecoderState, FadeStage, PlaybackState};
pub use queue_types::{EnqueueSource, QueueChangeTrigger, UserActionType};
pub use import_types::{
    AnalyzedPassageInfo, PhaseProgressData, PhaseStatistics, PhaseStatusData, RecordedPassageInfo,
    SubTaskData,
};
pub use shared_types::{BufferChainInfo, PlaybackPositionInfo, QueueEntryInfo};

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
        /// Playback state before change
        old_state: PlaybackState,
        /// Playback state after change
        new_state: PlaybackState,
        /// When state changed
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Passage started playing
    ///
    /// Triggers:
    /// - Historian: Record play start
    /// - SSE: Update all connected UIs
    /// - Lyrics Display: Show passage lyrics
    PassageStarted {
        /// Passage UUID that started playing
        passage_id: Uuid,
        /// **[REQ-DEBT-FUNC-003]** All albums for this passage, for cooldown/stats
        album_uuids: Vec<Uuid>,
        /// When passage started
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Passage completed or skipped
    ///
    /// Triggers:
    /// - Historian: Record play completion
    /// - Queue Manager: Advance queue
    /// - SSE: Update UI playback state
    PassageCompleted {
        /// Passage UUID that completed
        passage_id: Uuid,
        /// **[REQ-DEBT-FUNC-003]** All albums for this passage, for cooldown/stats
        album_uuids: Vec<Uuid>,
        /// Duration played in seconds
        duration_played: f64,
        /// Whether passage was completed (false if skipped)
        completed: bool,
        /// When passage completed
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
        /// Passage UUID currently playing
        passage_id: Uuid,
        /// Song UUID (None if passage has no song association)
        song_id: Option<Uuid>,
        /// All albums for this song, ordered by release date (newest first)
        song_albums: Vec<Uuid>,
        /// Current position in passage (milliseconds)
        position_ms: u64,
        /// When song changed
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
        /// Currently playing passage UUID
        passage_id: Uuid,
        /// Current playback position (milliseconds)
        position_ms: u64,
        /// Total passage duration (milliseconds)
        duration_ms: u64,
        /// Progress update timestamp
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Queue changed
    ///
    /// Triggers:
    /// - SSE: Update queue display
    /// - Auto-replenishment: Check if refill needed
    QueueChanged {
        /// Queue passage UUIDs
        queue: Vec<Uuid>,
        /// Why queue changed
        trigger: QueueChangeTrigger,
        /// When queue changed
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Queue state update (full queue contents for SSE)
    /// [SSE-UI-020] Queue Updates
    QueueStateUpdate {
        /// Update timestamp
        timestamp: chrono::DateTime<chrono::Utc>,
        /// Full queue with entry details
        queue: Vec<QueueEntryInfo>,
    },

    /// Playback position update (sent every 1s during playback)
    /// [SSE-UI-030] Playback Position Updates
    PlaybackPosition {
        /// Update timestamp
        timestamp: chrono::DateTime<chrono::Utc>,
        /// Currently playing passage UUID
        passage_id: Uuid,
        /// Current position in milliseconds
        position_ms: u64,
        /// Total duration in milliseconds
        duration_ms: u64,
        /// Whether currently playing (vs paused)
        playing: bool,
    },

    /// Passage enqueued (added to queue)
    ///
    /// Triggers:
    /// - SSE: Animate new queue entry
    /// - Analytics: Track auto vs manual enqueue
    PassageEnqueued {
        /// Passage UUID that was enqueued
        passage_id: Uuid,
        /// Position in queue (0-based)
        position: usize,
        /// How passage was enqueued (automatic vs manual)
        source: EnqueueSource,
        /// When passage was enqueued
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Passage dequeued (removed from queue)
    ///
    /// Triggers:
    /// - SSE: Update queue display
    PassageDequeued {
        /// Passage UUID that was dequeued
        passage_id: Uuid,
        /// Whether passage was currently playing when removed
        was_playing: bool,
        /// When passage was dequeued
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
        /// Current playback state (unchanged)
        playback_state: PlaybackState,
        /// When queue became empty
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Volume changed
    ///
    /// Triggers:
    /// - SSE: Update volume slider
    /// - State Persistence: Save volume preference
    VolumeChanged {
        /// Previous volume (0.0-1.0)
        old_volume: f32,
        /// New volume (0.0-1.0)
        new_volume: f32,
        /// When volume changed
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
        /// Passage UUID
        passage_id: Uuid,
        /// Previous buffer state
        old_state: BufferStatus,
        /// New buffer state
        new_state: BufferStatus,
        /// Decode progress percentage (only for Decoding state)
        decode_progress_percent: Option<f32>,
        /// When state changed
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
        /// Type of user action
        action: UserActionType,
        /// User's persistent UUID
        user_id: String,
        /// When action occurred
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// User liked a passage (Full/Lite versions only)
    ///
    /// Triggers:
    /// - Database: Record like associated with user UUID
    /// - Taste Manager: Update user's taste profile
    /// - SSE: Update like button state for all connected clients
    PassageLiked {
        /// Passage UUID that was liked
        passage_id: Uuid,
        /// User UUID who liked (may be Anonymous UUID)
        user_id: Uuid,
        /// When like occurred
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
        /// Passage UUID that was disliked
        passage_id: Uuid,
        /// User UUID who disliked (may be Anonymous UUID)
        user_id: Uuid,
        /// When dislike occurred
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
        /// Target musical flavor (8-dimensional AcousticBrainz vector)
        target_flavor: Vec<f64>,
        /// When override expires
        expiration: chrono::DateTime<chrono::Utc>,
        /// Override duration in seconds
        duration_seconds: u64,
        /// When override was set
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Temporary flavor override expired
    ///
    /// Triggers:
    /// - Program Director: Revert to timeslot-based target
    /// - SSE: Remove override indicator
    TemporaryFlavorOverrideExpired {
        /// When override expired
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
        /// Previous timeslot UUID
        old_timeslot_id: Uuid,
        /// New timeslot UUID
        new_timeslot_id: Uuid,
        /// New target flavor vector
        new_target_flavor: Vec<f64>,
        /// When timeslot changed
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
        /// Whether network is available
        available: bool,
        /// Number of reconnection retries
        retry_count: u32,
        /// When status changed
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Library scan completed (Full version only)
    ///
    /// Triggers:
    /// - SSE: Update library stats
    /// - Program Director: Refresh available passages
    LibraryScanCompleted {
        /// Number of files added to library
        files_added: usize,
        /// Number of files updated in library
        files_updated: usize,
        /// Number of files removed from library
        files_removed: usize,
        /// Scan duration in seconds
        duration_seconds: u64,
        /// When scan completed
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Database error occurred
    ///
    /// Triggers:
    /// - Error logging
    /// - SSE: Show error notification
    /// - Retry logic: Attempt recovery
    DatabaseError {
        /// Database operation that failed
        operation: String,
        /// Error message
        error: String,
        /// Whether retry was attempted
        retry_attempted: bool,
        /// When error occurred
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Initial state sent on SSE connection
    /// [SSE-UI-050] Initial State on Connection
    InitialState {
        /// When initial state was sent
        timestamp: chrono::DateTime<chrono::Utc>,
        /// Full queue with entry details
        queue: Vec<QueueEntryInfo>,
        /// Current playback position (if any)
        position: Option<PlaybackPositionInfo>,
        /// Current volume (0.0-1.0)
        volume: f32,
    },

    /// Crossfade started
    CrossfadeStarted {
        /// Passage UUID fading out
        from_passage_id: Uuid,
        /// Passage UUID fading in
        to_passage_id: Uuid,
        /// When crossfade started
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Buffer chain status update (sent every 1s when data changes)
    /// Shows decoder-resampler-fade-buffer chains for monitoring
    BufferChainStatus {
        /// When status was captured
        timestamp: chrono::DateTime<chrono::Utc>,
        /// Buffer chain information for all slots
        chains: Vec<BufferChainInfo>,
    },

    /// Endpoint discovered during decode
    /// **[DBD-DEC-095]** Emitted when decoder discovers actual file duration for undefined endpoints
    /// Sent by decoder → buffer manager when passage has NULL end_time_ticks
    EndpointDiscovered {
        /// Queue entry UUID
        queue_entry_id: Uuid,
        /// Actual end time in ticks
        actual_end_ticks: i64,
        /// When endpoint was discovered
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Pipeline validation succeeded (periodic check)
    /// **[ARCH-AUTO-VAL-001]** Automatic validation service - success result
    ValidationSuccess {
        /// When validation occurred
        timestamp: chrono::DateTime<chrono::Utc>,
        /// Number of passages validated
        passage_count: usize,
        /// Total samples decoded
        total_decoder_samples: u64,
        /// Total samples written to buffers
        total_buffer_written: u64,
        /// Total samples read from buffers
        total_buffer_read: u64,
        /// Total frames mixed to output
        total_mixer_frames: u64,
    },

    /// Pipeline validation failed (conservation law violated)
    /// **[ARCH-AUTO-VAL-001]** Automatic validation service - failure result
    ValidationFailure {
        /// When validation occurred
        timestamp: chrono::DateTime<chrono::Utc>,
        /// Number of passages validated
        passage_count: usize,
        /// Total samples decoded
        total_decoder_samples: u64,
        /// Total samples written to buffers
        total_buffer_written: u64,
        /// Total samples read from buffers
        total_buffer_read: u64,
        /// Total frames mixed to output
        total_mixer_frames: u64,
        /// Validation error messages
        errors: Vec<String>,
    },

    /// Pipeline validation warning (approaching tolerance threshold)
    /// **[ARCH-AUTO-VAL-001]** Automatic validation service - warning result (>80% of tolerance)
    ValidationWarning {
        /// When validation occurred
        timestamp: chrono::DateTime<chrono::Utc>,
        /// Number of passages validated
        passage_count: usize,
        /// Total samples decoded
        total_decoder_samples: u64,
        /// Total samples written to buffers
        total_buffer_written: u64,
        /// Total samples read from buffers
        total_buffer_read: u64,
        /// Total frames mixed to output
        total_mixer_frames: u64,
        /// Validation warning messages
        warnings: Vec<String>,
    },

    // ========================================================================
    // Error Events (Phase 7 - Error Handling & Recovery)
    // Per SPEC021 ERH-EVENT-010
    // ========================================================================

    /// Passage decode failed (file read error)
    /// **[REQ-AP-ERR-010]** Decode errors skip passage, continue with next
    PassageDecodeFailed {
        /// Passage UUID (if known)
        passage_id: Option<Uuid>,
        /// Error type classification
        error_type: String,
        /// Error message details
        error_message: String,
        /// File path that failed to decode
        file_path: String,
        /// When decode failed
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Passage has unsupported codec
    /// **[REQ-AP-ERR-011]** Unsupported codecs marked to prevent re-queue
    PassageUnsupportedCodec {
        /// Passage UUID (if known)
        passage_id: Option<Uuid>,
        /// File path with unsupported codec
        file_path: String,
        /// Codec hint (if detected)
        codec_hint: Option<String>,
        /// When codec issue detected
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Passage partially decoded (truncated file)
    /// **[REQ-AP-ERR-012]** Partial decode ≥50% allows playback
    PassagePartialDecode {
        /// Passage UUID (if known)
        passage_id: Option<Uuid>,
        /// File path that was partially decoded
        file_path: String,
        /// Expected duration in milliseconds
        expected_duration_ms: u64,
        /// Actual decoded duration in milliseconds
        actual_duration_ms: u64,
        /// Percentage successfully decoded
        percentage: f64,
        /// When partial decode detected
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Decoder panicked during decode
    /// **[REQ-AP-ERR-013]** Decoder panics caught and recovered
    PassageDecoderPanic {
        /// Passage UUID (if known)
        passage_id: Option<Uuid>,
        /// File path being decoded when panic occurred
        file_path: String,
        /// Panic message details
        panic_message: String,
        /// When panic occurred
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Buffer underrun detected
    /// **[REQ-AP-ERR-020]** Buffer underrun emergency refill with timeout
    BufferUnderrun {
        /// Passage UUID experiencing underrun
        passage_id: Uuid,
        /// Buffer fill percentage when underrun detected
        buffer_fill_percent: f32,
        /// When underrun detected
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Buffer underrun recovered
    /// **[REQ-AP-ERR-020]** Underrun recovery successful
    BufferUnderrunRecovered {
        /// Passage UUID that recovered
        passage_id: Uuid,
        /// Recovery time in milliseconds
        recovery_time_ms: u64,
        /// When recovery completed
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Audio callback underrun (real-time detection)
    /// Emitted EVERY time the audio callback finds the ring buffer empty
    /// This provides immediate detection of gaps/stutters
    AudioCallbackUnderrun {
        /// Total underrun count since startup
        underrun_count: u64,
        /// When underrun occurred
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Audio callback irregular interval detected
    /// Emitted when callback timing deviates significantly from expected
    /// (throttled to avoid spam - max once per 5 seconds)
    AudioCallbackIrregular {
        /// Actual interval measured in milliseconds
        actual_interval_ms: u64,
        /// Expected interval in milliseconds
        expected_interval_ms: u64,
        /// Deviation from expected in milliseconds
        deviation_ms: u64,
        /// Total irregular intervals since startup
        total_irregular_count: u64,
        /// When irregular interval detected
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Audio device lost (disconnected)
    /// **[REQ-AP-ERR-030]** Device disconnect retry 30s before fallback
    AudioDeviceLost {
        /// Human-readable device name
        device_name: String,
        /// Device identifier
        device_id: String,
        /// When device was lost
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Audio device restored after disconnection
    /// **[REQ-AP-ERR-030]** Device reconnected successfully
    AudioDeviceRestored {
        /// Human-readable device name
        device_name: String,
        /// When device was restored
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Audio device fallback (original device unavailable)
    /// **[REQ-AP-ERR-030]** Fallback to system default device
    AudioDeviceFallback {
        /// Original device name that was unavailable
        original_device: String,
        /// Fallback device name being used
        fallback_device: String,
        /// When fallback occurred
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Audio device unavailable (all attempts failed)
    /// **[REQ-AP-ERR-030]** No audio device available
    AudioDeviceUnavailable {
        /// When unavailability determined
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Audio device configuration error
    /// **[REQ-AP-ERR-031]** Device config errors attempt 4 fallback configs
    AudioDeviceConfigError {
        /// Device name with config error
        device_name: String,
        /// Configuration that was requested
        requested_config: String,
        /// Error message details
        error_message: String,
        /// When config error occurred
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Audio device configuration fallback succeeded
    /// **[REQ-AP-ERR-031]** Fallback configuration successful
    AudioDeviceConfigFallback {
        /// Device name using fallback config
        device_name: String,
        /// Fallback configuration being used
        fallback_config: String,
        /// When fallback succeeded
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Audio device incompatible (all configs failed)
    /// **[REQ-AP-ERR-031]** Device incompatible with all configurations
    AudioDeviceIncompatible {
        /// Incompatible device name
        device_name: String,
        /// When incompatibility determined
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Queue validation error (invalid entry)
    /// **[REQ-AP-ERR-040]** Invalid queue entries auto-removed with logging
    QueueValidationError {
        /// Queue entry UUID with validation error
        queue_entry_id: Uuid,
        /// Passage UUID (if known)
        passage_id: Option<Uuid>,
        /// Validation error message
        validation_error: String,
        /// When validation error detected
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Queue depth warning (all chains busy)
    /// **[REQ-AP-ERR-040]** Queue depth exceeds available chains
    QueueDepthWarning {
        /// Current queue depth
        queue_depth: usize,
        /// Number of available chains
        available_chains: usize,
        /// When warning occurred
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Resampling failed (initialization)
    /// **[REQ-AP-ERR-050]** Resampling init fails, skip passage or bypass
    ResamplingFailed {
        /// Passage UUID with resampling failure
        passage_id: Uuid,
        /// Source sample rate in Hz
        source_rate: u32,
        /// Target sample rate in Hz
        target_rate: u32,
        /// Error message details
        error_message: String,
        /// When resampling failed
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Resampling runtime error
    /// **[REQ-AP-ERR-051]** Resampling runtime errors skip passage
    ResamplingRuntimeError {
        /// Passage UUID with runtime error
        passage_id: Uuid,
        /// Position in passage when error occurred (milliseconds)
        position_ms: u64,
        /// Error message details
        error_message: String,
        /// When runtime error occurred
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Timing system failure (fatal)
    /// **[SPEC021 ERH-TIME-010]** Tick overflow or timing corruption
    TimingSystemFailure {
        /// Error type classification
        error_type: String,
        /// When timing failure occurred
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Position drift warning
    /// **[REQ-AP-ERR-060]** Position drift <100 samples auto-corrected
    PositionDriftWarning {
        /// Passage UUID experiencing drift
        passage_id: Uuid,
        /// Expected position in milliseconds
        expected_position_ms: u64,
        /// Actual position in milliseconds
        actual_position_ms: u64,
        /// Position delta in milliseconds (signed)
        delta_ms: i64,
        /// When drift detected
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// System resource exhausted
    /// **[REQ-AP-ERR-070]** Resource exhaustion cleanup and retry
    SystemResourceExhausted {
        /// Type of resource exhausted
        resource_type: String,
        /// When exhaustion detected
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// System resource recovered
    /// **[REQ-AP-ERR-070]** Resource exhaustion resolved
    SystemResourceRecovered {
        /// Type of resource recovered
        resource_type: String,
        /// When recovery occurred
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// File handle exhaustion
    /// **[REQ-AP-ERR-071]** File handle exhaustion, reduce chain count
    FileHandleExhaustion {
        /// File path that failed to open
        attempted_file: String,
        /// When exhaustion detected
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// System degraded mode activated
    /// **[REQ-AP-DEGRADE-010/020/030]** Graceful degradation
    SystemDegradedMode {
        /// Reason for degraded mode
        reason: String,
        /// When degraded mode activated
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// System shutdown required (fatal error)
    /// **[SPEC021 ERH-TAX-010]** FATAL error requires shutdown
    SystemShutdownRequired {
        /// Reason for shutdown requirement
        reason: String,
        /// When shutdown required
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    // ========================================================================
    // Audio Ingest Events (wkmp-ai - Full version only)
    // **[AIA-MS-010]** SSE event streaming for import workflow
    // ========================================================================

    /// Import session started
    ///
    /// Triggers:
    /// - SSE: Show import progress UI
    /// - Database: Session record created
    ImportSessionStarted {
        /// Import session UUID
        session_id: Uuid,
        /// Root folder being scanned
        root_folder: String,
        /// When session started
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Import progress update
    ///
    /// Emitted during workflow progression through states
    ///
    /// Triggers:
    /// - SSE: Update progress bar and status
    ///
    /// **[REQ-AIA-UI-001, REQ-AIA-UI-004]** Extended with phase tracking and current file
    ImportProgressUpdate {
        /// Import session UUID
        session_id: Uuid,
        /// Current workflow state
        state: String,
        /// Current item count
        current: usize,
        /// Total item count
        total: usize,
        /// Progress percentage (0.0-100.0)
        percentage: f32,
        /// Current operation description
        current_operation: String,
        /// Elapsed time in seconds
        elapsed_seconds: u64,
        /// Estimated remaining time in seconds (if available)
        estimated_remaining_seconds: Option<u64>,
        /// Phase-level progress (6 phases: SCANNING through FLAVORING)
        #[serde(default)]
        phases: Vec<PhaseProgressData>,
        /// Current file being processed
        #[serde(default)]
        current_file: Option<String>,
        /// **PLAN024 Phase-Specific Statistics** (per wkmp-ai_refinement.md)
        #[serde(default)]
        phase_statistics: Vec<PhaseStatistics>,
        /// When progress updated
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Import session completed successfully
    ///
    /// Triggers:
    /// - SSE: Show completion notification
    /// - Database: Mark session as completed
    /// - Program Director: Refresh available passages
    ImportSessionCompleted {
        /// Import session UUID
        session_id: Uuid,
        /// Number of files processed
        files_processed: usize,
        /// Session duration in seconds
        duration_seconds: u64,
        /// When session completed
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Import session failed
    ///
    /// Triggers:
    /// - SSE: Show error notification
    /// - Database: Mark session as failed
    ImportSessionFailed {
        /// Import session UUID
        session_id: Uuid,
        /// Error message details
        error_message: String,
        /// Number of files processed before failure
        files_processed: usize,
        /// When session failed
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Import session cancelled by user
    ///
    /// Triggers:
    /// - SSE: Show cancellation notification
    /// - Database: Mark session as cancelled
    ImportSessionCancelled {
        /// Import session UUID
        session_id: Uuid,
        /// Number of files processed before cancellation
        files_processed: usize,
        /// Number of files skipped due to cancellation
        files_skipped: usize,
        /// When session cancelled
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// **[PLAN020 Phase 5]** Watchdog intervention occurred
    ///
    /// Emitted when watchdog safety net must intervene due to event system failure.
    ///
    /// Triggers:
    /// - SSE: Update watchdog status indicator immediately
    /// - Developer UI: Flash indicator to draw attention
    /// - Monitoring: Track event system health
    WatchdogIntervention {
        /// Type of intervention ("decode" or "mixer")
        intervention_type: String,
        /// Total interventions since startup
        interventions_total: u64,
        /// When intervention occurred
        timestamp: chrono::DateTime<chrono::Utc>,
    },
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
            // Error events (Phase 7)
            WkmpEvent::PassageDecodeFailed { .. } => "PassageDecodeFailed",
            WkmpEvent::PassageUnsupportedCodec { .. } => "PassageUnsupportedCodec",
            WkmpEvent::PassagePartialDecode { .. } => "PassagePartialDecode",
            WkmpEvent::PassageDecoderPanic { .. } => "PassageDecoderPanic",
            WkmpEvent::BufferUnderrun { .. } => "BufferUnderrun",
            WkmpEvent::BufferUnderrunRecovered { .. } => "BufferUnderrunRecovered",
            WkmpEvent::AudioCallbackUnderrun { .. } => "AudioCallbackUnderrun",
            WkmpEvent::AudioCallbackIrregular { .. } => "AudioCallbackIrregular",
            WkmpEvent::AudioDeviceLost { .. } => "AudioDeviceLost",
            WkmpEvent::AudioDeviceRestored { .. } => "AudioDeviceRestored",
            WkmpEvent::AudioDeviceFallback { .. } => "AudioDeviceFallback",
            WkmpEvent::AudioDeviceUnavailable { .. } => "AudioDeviceUnavailable",
            WkmpEvent::AudioDeviceConfigError { .. } => "AudioDeviceConfigError",
            WkmpEvent::AudioDeviceConfigFallback { .. } => "AudioDeviceConfigFallback",
            WkmpEvent::AudioDeviceIncompatible { .. } => "AudioDeviceIncompatible",
            WkmpEvent::QueueValidationError { .. } => "QueueValidationError",
            WkmpEvent::QueueDepthWarning { .. } => "QueueDepthWarning",
            WkmpEvent::ResamplingFailed { .. } => "ResamplingFailed",
            WkmpEvent::ResamplingRuntimeError { .. } => "ResamplingRuntimeError",
            WkmpEvent::TimingSystemFailure { .. } => "TimingSystemFailure",
            WkmpEvent::PositionDriftWarning { .. } => "PositionDriftWarning",
            WkmpEvent::SystemResourceExhausted { .. } => "SystemResourceExhausted",
            WkmpEvent::SystemResourceRecovered { .. } => "SystemResourceRecovered",
            WkmpEvent::FileHandleExhaustion { .. } => "FileHandleExhaustion",
            WkmpEvent::SystemDegradedMode { .. } => "SystemDegradedMode",
            WkmpEvent::SystemShutdownRequired { .. } => "SystemShutdownRequired",
            // Audio Ingest events (wkmp-ai)
            WkmpEvent::ImportSessionStarted { .. } => "ImportSessionStarted",
            WkmpEvent::ImportProgressUpdate { .. } => "ImportProgressUpdate",
            WkmpEvent::ImportSessionCompleted { .. } => "ImportSessionCompleted",
            WkmpEvent::ImportSessionFailed { .. } => "ImportSessionFailed",
            WkmpEvent::ImportSessionCancelled { .. } => "ImportSessionCancelled",
            // **[PLAN020 Phase 5]** Watchdog monitoring event
            WkmpEvent::WatchdogIntervention { .. } => "WatchdogIntervention",
        }
    }
}

// ========================================
// Supporting Enums
// ========================================

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
#[derive(Clone)]
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
    #[allow(clippy::result_large_err)]
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
            resampler_algorithm: None,
            decode_duration_ms: Some(150),
            source_file_path: Some("/music/test.mp3".to_string()),
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
            resampler_algorithm: Some("SincFixedIn".to_string()),
            decode_duration_ms: Some(80),
            source_file_path: Some("/music/next.mp3".to_string()),
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
        assert!(
            chain_up_next.resampler_active.expect("resampler_active should be Some"),
            "48kHz source should require resampling"
        );

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
            resampler_algorithm: None,
            decode_duration_ms: Some(30),
            source_file_path: Some("/music/queued.mp3".to_string()),
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
            resampler_algorithm: Some("SincFixedIn".to_string()),
            decode_duration_ms: Some(120),
            source_file_path: Some("/music/test.mp3".to_string()),
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

    /// **[EVENTBUS-TEST-010]** Test EventBus::new() creates bus with correct capacity
    #[test]
    fn test_eventbus_new() {
        let bus = EventBus::new(100);
        assert_eq!(bus.capacity(), 100);
        assert_eq!(bus.subscriber_count(), 0);
    }

    /// **[EVENTBUS-TEST-020]** Test EventBus::subscribe() creates working receiver
    #[test]
    fn test_eventbus_subscribe() {
        let bus = EventBus::new(10);
        let _rx = bus.subscribe();
        assert_eq!(bus.subscriber_count(), 1);

        let _rx2 = bus.subscribe();
        assert_eq!(bus.subscriber_count(), 2);
    }

    /// **[EVENTBUS-TEST-030]** Test EventBus::emit() delivers events to subscribers
    #[test]
    fn test_eventbus_emit() {
        use std::sync::Arc;
        let bus = Arc::new(EventBus::new(10));
        let mut rx = bus.subscribe();

        let event = WkmpEvent::PlaybackStateChanged {
            old_state: PlaybackState::Paused,
            new_state: PlaybackState::Playing,
            timestamp: chrono::Utc::now(),
        };

        bus.emit(event.clone()).expect("emit should succeed");

        // Receive event
        let received = rx.try_recv().expect("Should receive event");
        assert_eq!(received.event_type(), "PlaybackStateChanged");
    }

    /// **[EVENTBUS-TEST-040]** Test EventBus::emit_lossy() does not panic on full channel
    #[test]
    fn test_eventbus_emit_lossy() {
        use std::sync::Arc;
        let bus = Arc::new(EventBus::new(2)); // Small capacity
        let mut _rx = bus.subscribe(); // Subscribe but don't receive

        // Fill the channel
        for i in 0..10 {
            let event = WkmpEvent::PlaybackProgress {
                passage_id: Uuid::new_v4(),
                position_ms: i * 1000,
                duration_ms: 180000,
                timestamp: chrono::Utc::now(),
            };
            bus.emit_lossy(event); // Should not panic even when full
        }

        // Function should complete without panic
        assert_eq!(bus.capacity(), 2);
    }

    /// **[EVENTBUS-TEST-050]** Test multiple subscribers receive same event
    #[test]
    fn test_eventbus_multiple_subscribers() {
        use std::sync::Arc;
        let bus = Arc::new(EventBus::new(10));
        let mut rx1 = bus.subscribe();
        let mut rx2 = bus.subscribe();
        let mut rx3 = bus.subscribe();

        assert_eq!(bus.subscriber_count(), 3);

        let event = WkmpEvent::QueueChanged {
            queue: vec![Uuid::new_v4()],
            trigger: QueueChangeTrigger::UserEnqueue,
            timestamp: chrono::Utc::now(),
        };

        bus.emit(event.clone()).expect("emit should succeed");

        // All three subscribers should receive the event
        let r1 = rx1.try_recv().expect("rx1 should receive");
        let r2 = rx2.try_recv().expect("rx2 should receive");
        let r3 = rx3.try_recv().expect("rx3 should receive");

        assert_eq!(r1.event_type(), "QueueChanged");
        assert_eq!(r2.event_type(), "QueueChanged");
        assert_eq!(r3.event_type(), "QueueChanged");
    }

    /// **[EVENTBUS-TEST-060]** Test WkmpEvent::event_type() for all major events
    #[test]
    fn test_event_type_method() {
        let events = vec![
            (
                WkmpEvent::PlaybackStateChanged {
                    old_state: PlaybackState::Paused,
                    new_state: PlaybackState::Playing,
                    timestamp: chrono::Utc::now(),
                },
                "PlaybackStateChanged"
            ),
            (
                WkmpEvent::PassageStarted {
                    passage_id: Uuid::new_v4(),
                    album_uuids: vec![],
                    timestamp: chrono::Utc::now(),
                },
                "PassageStarted"
            ),
            (
                WkmpEvent::PassageCompleted {
                    passage_id: Uuid::new_v4(),
                    album_uuids: vec![],
                    duration_played: 120.5,
                    completed: true,
                    timestamp: chrono::Utc::now(),
                },
                "PassageCompleted"
            ),
            (
                WkmpEvent::QueueChanged {
                    queue: vec![],
                    trigger: QueueChangeTrigger::UserEnqueue,
                    timestamp: chrono::Utc::now(),
                },
                "QueueChanged"
            ),
            (
                WkmpEvent::PlaybackProgress {
                    passage_id: Uuid::new_v4(),
                    position_ms: 5000,
                    duration_ms: 180000,
                    timestamp: chrono::Utc::now(),
                },
                "PlaybackProgress"
            ),
        ];

        for (event, expected_type) in events {
            assert_eq!(event.event_type(), expected_type);
        }
    }
}
