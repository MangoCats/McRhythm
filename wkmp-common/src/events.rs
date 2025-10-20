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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BufferChainInfo {
    pub slot_index: usize,
    pub queue_entry_id: Option<Uuid>,
    pub passage_id: Option<Uuid>,
    pub file_name: Option<String>,
    pub buffer_fill_percent: f32,
    pub buffer_fill_samples: usize,
    pub buffer_capacity_samples: usize,
    pub playback_position_frames: usize,
    pub playback_position_ms: u64,
    pub duration_ms: Option<u64>,
    pub is_active_in_mixer: bool,
    pub mixer_role: String, // "Idle", "Current", "Next", "Crossfading"
    pub started_at: Option<String>,
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
