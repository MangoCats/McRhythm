//! SSE event types for real-time playback updates

use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use uuid::Uuid;

/// SSE event wrapper for transmission
#[derive(Debug, Clone, Serialize)]
pub struct SseEvent {
    /// Event type name
    pub event: String,

    /// Event data (JSON)
    pub data: SseEventData,

    /// Event ID for client reconnection
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

impl SseEvent {
    /// Create a new SSE event
    pub fn new(event: &str, data: SseEventData) -> Self {
        Self {
            event: event.to_string(),
            data,
            id: Some(Uuid::new_v4().to_string()),
        }
    }

    /// Format as SSE protocol string
    pub fn to_sse_string(&self) -> String {
        let mut output = String::new();

        if let Some(id) = &self.id {
            output.push_str(&format!("id: {}\n", id));
        }

        output.push_str(&format!("event: {}\n", self.event));

        let data_json = serde_json::to_string(&self.data).unwrap_or_default();
        output.push_str(&format!("data: {}\n\n", data_json));

        output
    }
}

/// SSE event data variants
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SseEventData {
    /// Playback state changed (Playing/Paused)
    PlaybackStateChanged {
        state: String,
        timestamp: u64,
    },

    /// Playback progress update (5s interval)
    PlaybackProgress {
        passage_id: Option<String>,
        position_ms: u64,
        duration_ms: u64,
        timestamp: u64,
    },

    /// Current song changed within passage
    CurrentSongChanged {
        passage_id: String,
        song_id: Option<String>,
        position_ms: u64,
        timestamp: u64,
    },

    /// Queue contents changed
    QueueChanged {
        entries: Vec<QueueEntryInfo>,
        count: usize,
        timestamp: u64,
    },

    /// Volume changed
    VolumeChanged {
        volume: i32, // 0-100 for user-facing
        timestamp: u64,
    },

    /// Keep-alive ping
    KeepAlive {
        timestamp: u64,
    },
}

/// Simplified queue entry info for SSE transmission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueEntryInfo {
    pub guid: String,
    pub file_path: String,
    pub passage_guid: Option<String>,
}

impl SseEventData {
    /// Get current timestamp in milliseconds since UNIX epoch
    fn current_timestamp_ms() -> u64 {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }

    /// Create PlaybackStateChanged event
    pub fn playback_state_changed(state: &str) -> Self {
        Self::PlaybackStateChanged {
            state: state.to_string(),
            timestamp: Self::current_timestamp_ms(),
        }
    }

    /// Create PlaybackProgress event
    pub fn playback_progress(passage_id: Option<String>, position_ms: u64, duration_ms: u64) -> Self {
        Self::PlaybackProgress {
            passage_id,
            position_ms,
            duration_ms,
            timestamp: Self::current_timestamp_ms(),
        }
    }

    /// Create CurrentSongChanged event
    pub fn current_song_changed(passage_id: String, song_id: Option<String>, position_ms: u64) -> Self {
        Self::CurrentSongChanged {
            passage_id,
            song_id,
            position_ms,
            timestamp: Self::current_timestamp_ms(),
        }
    }

    /// Create QueueChanged event
    pub fn queue_changed(entries: Vec<QueueEntryInfo>) -> Self {
        let count = entries.len();
        Self::QueueChanged {
            entries,
            count,
            timestamp: Self::current_timestamp_ms(),
        }
    }

    /// Create VolumeChanged event
    pub fn volume_changed(volume: i32) -> Self {
        Self::VolumeChanged {
            volume,
            timestamp: Self::current_timestamp_ms(),
        }
    }

    /// Create KeepAlive event
    pub fn keep_alive() -> Self {
        Self::KeepAlive {
            timestamp: Self::current_timestamp_ms(),
        }
    }
}
