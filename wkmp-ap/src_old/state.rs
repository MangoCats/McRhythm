//! Shared playback state
//!
//! Thread-safe shared state for playback coordination between components.
//!
//! **Traceability:**
//! - CO-120 (Shared state pattern)
//! - SSD-MIX-040 (Playback state management)

use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;
use wkmp_common::events::WkmpEvent;

/// Playback state enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    /// Playback is active (or waiting for queue to be populated)
    Playing,
    /// Playback is paused
    Paused,
}

/// Current passage information
#[derive(Debug, Clone)]
pub struct CurrentPassage {
    /// Queue entry ID
    pub queue_entry_id: Uuid,
    /// Passage ID (may be None for ephemeral passages)
    pub passage_id: Option<Uuid>,
    /// Current position in milliseconds
    pub position_ms: u64,
    /// Total duration in milliseconds
    pub duration_ms: u64,
}

/// Shared state accessible by all components
///
/// Uses RwLock for concurrent read access with rare writes
///
/// **Traceability:** CO-145 (Appropriate mutex selection for async)
pub struct SharedState {
    /// Current playback state (Playing or Paused)
    pub playback_state: RwLock<PlaybackState>,

    /// Currently playing passage (None if queue empty)
    pub current_passage: RwLock<Option<CurrentPassage>>,

    /// Master volume (0.0-1.0, system-level scale)
    pub volume: RwLock<f32>,

    /// Event broadcaster for SSE events
    pub event_tx: broadcast::Sender<WkmpEvent>,
}

impl SharedState {
    /// Create new shared state with default values
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(100); // Buffer up to 100 events
        Self {
            playback_state: RwLock::new(PlaybackState::Playing), // Default to Playing on startup
            current_passage: RwLock::new(None),
            volume: RwLock::new(0.75), // Default 75% volume
            event_tx,
        }
    }

    /// Broadcast an event to all SSE listeners
    pub fn broadcast_event(&self, event: WkmpEvent) {
        // Ignore send errors (no receivers is OK)
        let _ = self.event_tx.send(event);
    }

    /// Subscribe to event stream for SSE
    pub fn subscribe_events(&self) -> broadcast::Receiver<WkmpEvent> {
        self.event_tx.subscribe()
    }

    /// Get current playback state
    pub async fn get_playback_state(&self) -> PlaybackState {
        *self.playback_state.read().await
    }

    /// Set playback state
    pub async fn set_playback_state(&self, state: PlaybackState) {
        *self.playback_state.write().await = state;
    }

    /// Get current passage information
    pub async fn get_current_passage(&self) -> Option<CurrentPassage> {
        self.current_passage.read().await.clone()
    }

    /// Set current passage
    pub async fn set_current_passage(&self, passage: Option<CurrentPassage>) {
        *self.current_passage.write().await = passage;
    }

    /// Get master volume (0.0-1.0)
    pub async fn get_volume(&self) -> f32 {
        *self.volume.read().await
    }

    /// Set master volume (0.0-1.0)
    pub async fn set_volume(&self, volume: f32) {
        *self.volume.write().await = volume.clamp(0.0, 1.0);
    }
}

impl Default for SharedState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_playback_state() {
        let state = SharedState::new();

        // Default is Playing
        assert_eq!(state.get_playback_state().await, PlaybackState::Playing);

        // Can set to Paused
        state.set_playback_state(PlaybackState::Paused).await;
        assert_eq!(state.get_playback_state().await, PlaybackState::Paused);
    }

    #[tokio::test]
    async fn test_volume() {
        let state = SharedState::new();

        // Default volume is 0.75
        assert_eq!(state.get_volume().await, 0.75);

        // Can set volume
        state.set_volume(0.5).await;
        assert_eq!(state.get_volume().await, 0.5);

        // Volume is clamped to 0.0-1.0
        state.set_volume(1.5).await;
        assert_eq!(state.get_volume().await, 1.0);

        state.set_volume(-0.5).await;
        assert_eq!(state.get_volume().await, 0.0);
    }

    #[tokio::test]
    async fn test_current_passage() {
        let state = SharedState::new();

        // Default is None
        assert!(state.get_current_passage().await.is_none());

        // Can set passage
        let passage = CurrentPassage {
            queue_entry_id: Uuid::new_v4(),
            passage_id: Some(Uuid::new_v4()),
            position_ms: 1000,
            duration_ms: 180000,
        };

        state.set_current_passage(Some(passage.clone())).await;
        let retrieved = state.get_current_passage().await.unwrap();
        assert_eq!(retrieved.queue_entry_id, passage.queue_entry_id);
        assert_eq!(retrieved.position_ms, 1000);
    }
}
