//! Event types for WKMP event system

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// WKMP event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WkmpEvent {
    /// Playback state changed
    PlaybackStateChanged {
        state: PlaybackState,
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Passage started playing
    PassageStarted {
        passage_id: Uuid,
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Passage completed
    PassageCompleted {
        passage_id: Uuid,
        completed: bool,
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Current song changed
    CurrentSongChanged {
        passage_id: Uuid,
        song_id: Option<Uuid>,
        position_ms: u64,
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Playback progress update
    PlaybackProgress {
        passage_id: Uuid,
        position_ms: u64,
        duration_ms: u64,
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Queue changed (notification only - no data)
    QueueChanged {
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

    /// Volume changed
    VolumeChanged {
        volume: f64,
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
    Stopped,
}

impl std::fmt::Display for PlaybackState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlaybackState::Playing => write!(f, "playing"),
            PlaybackState::Paused => write!(f, "paused"),
            PlaybackState::Stopped => write!(f, "stopped"),
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
            WkmpEvent::VolumeChanged { .. } => "VolumeChanged",
            WkmpEvent::InitialState { .. } => "InitialState",
            WkmpEvent::CrossfadeStarted { .. } => "CrossfadeStarted",
            WkmpEvent::BufferChainStatus { .. } => "BufferChainStatus",
        }
    }
}
