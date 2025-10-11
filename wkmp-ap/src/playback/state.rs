//! Playback state management

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Playback state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

/// Shared playback state
#[derive(Debug, Clone)]
pub struct SharedPlaybackState {
    inner: Arc<RwLock<PlaybackStateInner>>,
}

#[derive(Debug)]
struct PlaybackStateInner {
    pub state: PlaybackState,
    pub currently_playing_passage_id: Option<Uuid>,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub volume: f64,
}

impl SharedPlaybackState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(PlaybackStateInner {
                state: PlaybackState::Stopped,
                currently_playing_passage_id: None,
                position_ms: 0,
                duration_ms: 0,
                volume: 0.75, // Default volume from database
            })),
        }
    }

    pub async fn get_state(&self) -> PlaybackState {
        self.inner.read().await.state
    }

    pub async fn set_state(&self, state: PlaybackState) {
        self.inner.write().await.state = state;
    }

    pub async fn get_currently_playing(&self) -> Option<Uuid> {
        self.inner.read().await.currently_playing_passage_id
    }

    pub async fn set_currently_playing(&self, passage_id: Option<Uuid>) {
        self.inner.write().await.currently_playing_passage_id = passage_id;
    }

    pub async fn get_position(&self) -> (u64, u64) {
        let inner = self.inner.read().await;
        (inner.position_ms, inner.duration_ms)
    }

    pub async fn set_position(&self, position_ms: u64, duration_ms: u64) {
        let mut inner = self.inner.write().await;
        inner.position_ms = position_ms;
        inner.duration_ms = duration_ms;
    }

    pub async fn get_volume(&self) -> f64 {
        self.inner.read().await.volume
    }

    pub async fn set_volume(&self, volume: f64) {
        self.inner.write().await.volume = volume.clamp(0.0, 1.0);
    }
}

impl Default for SharedPlaybackState {
    fn default() -> Self {
        Self::new()
    }
}
