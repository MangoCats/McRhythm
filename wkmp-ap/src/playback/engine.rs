//! Playback engine coordinating GStreamer and queue

use anyhow::Result;
use sqlx::SqlitePool;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use super::queue::QueueManager;
use super::state::{PlaybackState as State, SharedPlaybackState};

/// Playback engine coordinates GStreamer pipelines and queue management
pub struct PlaybackEngine {
    root_folder: PathBuf,
    queue: QueueManager,
    state: SharedPlaybackState,
    // GStreamer pipelines will be added later
}

impl PlaybackEngine {
    /// Create a new playback engine
    pub fn new(db: SqlitePool, root_folder: PathBuf) -> Self {
        let queue = QueueManager::new(db.clone(), root_folder.clone());
        let state = SharedPlaybackState::new();

        Self {
            root_folder,
            queue,
            state,
        }
    }

    /// Initialize the playback engine
    pub async fn init(&mut self) -> Result<()> {
        info!("Initializing playback engine...");

        // Initialize queue manager
        self.queue.init().await?;

        // TODO: Initialize GStreamer pipelines

        // Load initial playback state from settings
        // (will implement after settings loading is added)

        info!("Playback engine initialized");
        Ok(())
    }

    /// Start playback
    pub async fn play(&mut self) -> Result<()> {
        let current_state = self.state.get_state().await;

        match current_state {
            State::Stopped => {
                // Load first item from queue and start playing
                if let Some(entry) = self.queue.get_next().await {
                    info!("Starting playback of: {}", entry.file_path);
                    // TODO: Load into GStreamer pipeline and start
                    self.state.set_state(State::Playing).await;
                } else {
                    warn!("Queue is empty, cannot start playback");
                    // Set to playing state anyway per spec (ARCH-STARTUP-010)
                    self.state.set_state(State::Playing).await;
                }
            }
            State::Paused => {
                // Resume playback
                info!("Resuming playback");
                // TODO: Resume GStreamer pipeline
                self.state.set_state(State::Playing).await;
            }
            State::Playing => {
                debug!("Already playing");
            }
        }

        Ok(())
    }

    /// Pause playback
    pub async fn pause(&mut self) -> Result<()> {
        let current_state = self.state.get_state().await;

        if current_state == State::Playing {
            info!("Pausing playback");
            // TODO: Pause GStreamer pipeline
            self.state.set_state(State::Paused).await;
        } else {
            debug!("Not playing, cannot pause");
        }

        Ok(())
    }

    /// Skip to next track in queue
    pub async fn skip(&mut self) -> Result<()> {
        info!("Skipping to next track");

        // Remove current entry from queue
        if let Some(current) = self.queue.get_next().await {
            self.queue.remove(&current.guid).await?;
        }

        // Start next track if playing
        if self.state.get_state().await == State::Playing {
            self.play().await?;
        }

        Ok(())
    }

    /// Get current playback state
    pub async fn get_state(&self) -> State {
        self.state.get_state().await
    }

    /// Get queue manager reference
    pub fn queue(&self) -> &QueueManager {
        &self.queue
    }

    /// Get shared playback state
    pub fn shared_state(&self) -> &SharedPlaybackState {
        &self.state
    }
}
