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

    /// Queue changed
    QueueChanged {
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Volume changed
    VolumeChanged {
        volume: f64,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
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
            WkmpEvent::VolumeChanged { .. } => "VolumeChanged",
        }
    }
}
