//! Playback engine coordinating GStreamer and queue

use anyhow::Result;
use sqlx::SqlitePool;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn, error};

use super::pipeline::SinglePipeline;
use super::queue::QueueManager;
use super::state::{PlaybackState as State, SharedPlaybackState};

/// Playback engine coordinates GStreamer pipelines and queue management
pub struct PlaybackEngine {
    root_folder: PathBuf,
    queue: QueueManager,
    state: SharedPlaybackState,
    current_pipeline: Option<SinglePipeline>,
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
            current_pipeline: None,
        }
    }

    /// Initialize the playback engine
    pub async fn init(&mut self) -> Result<()> {
        info!("Initializing playback engine...");

        // Initialize queue manager
        self.queue.init().await?;

        info!("Playback engine initialized");
        Ok(())
    }

    /// Load and prepare the next track from queue
    async fn load_next_track(&mut self) -> Result<()> {
        // Get next entry from queue
        let entry = self.queue.get_next().await
            .ok_or_else(|| anyhow::anyhow!("Queue is empty"))?;

        info!("Loading track: {}", entry.file_path);

        // Build full path
        let full_path = self.root_folder.join(&entry.file_path);

        // Create pipeline
        let start_ms = entry.start_time_ms.unwrap_or(0);
        let end_ms = entry.end_time_ms.unwrap_or(180000); // Default 3 min

        let pipeline = SinglePipeline::new(&full_path, start_ms, end_ms)?;

        // Seek to start position if not zero
        if start_ms > 0 {
            pipeline.seek_to(start_ms)?;
        }

        // Store pipeline
        self.current_pipeline = Some(pipeline);

        // Update state
        let passage_id = entry.passage_guid.as_ref()
            .and_then(|s| uuid::Uuid::parse_str(s).ok());
        self.state.set_currently_playing(passage_id).await;

        Ok(())
    }

    /// Start playback
    pub async fn play(&mut self) -> Result<()> {
        let current_state = self.state.get_state().await;

        match current_state {
            State::Stopped => {
                // Load first item from queue and start playing
                if self.queue.is_empty().await {
                    warn!("Queue is empty, cannot start playback");
                    // Set to playing state anyway per spec (ARCH-STARTUP-010)
                    self.state.set_state(State::Playing).await;
                    return Ok(());
                }

                // Load next track
                if let Err(e) = self.load_next_track().await {
                    error!("Failed to load track: {}", e);
                    return Err(e);
                }

                // Start pipeline
                if let Some(ref pipeline) = self.current_pipeline {
                    pipeline.play()?;
                    self.state.set_state(State::Playing).await;
                    info!("Playback started");
                } else {
                    error!("Pipeline not loaded");
                    return Err(anyhow::anyhow!("Pipeline not loaded"));
                }
            }
            State::Paused => {
                // Resume playback
                info!("Resuming playback");
                if let Some(ref pipeline) = self.current_pipeline {
                    pipeline.play()?;
                    self.state.set_state(State::Playing).await;
                } else {
                    warn!("No pipeline to resume");
                }
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
            if let Some(ref pipeline) = self.current_pipeline {
                pipeline.pause()?;
                self.state.set_state(State::Paused).await;
            }
        } else {
            debug!("Not playing, cannot pause");
        }

        Ok(())
    }

    /// Skip to next track in queue
    pub async fn skip(&mut self) -> Result<()> {
        info!("Skipping to next track");

        // Stop current pipeline
        if let Some(ref pipeline) = self.current_pipeline {
            let _ = pipeline.stop();
        }
        self.current_pipeline = None;

        // Remove current entry from queue
        if let Some(current) = self.queue.get_next().await {
            self.queue.remove(&current.guid).await?;
        }

        // Set to stopped and restart if we were playing
        let was_playing = self.state.get_state().await == State::Playing;
        self.state.set_state(State::Stopped).await;

        if was_playing {
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

    /// Update position from pipeline (call this periodically)
    pub async fn update_position(&self) {
        if let Some(ref pipeline) = self.current_pipeline {
            if let Some(pos_ms) = pipeline.position_ms() {
                if let Some(dur_ms) = pipeline.duration_ms() {
                    self.state.set_position(pos_ms as u64, dur_ms as u64).await;
                }
            }
        }
    }

    /// Check if end of stream and advance to next track
    pub async fn check_eos(&mut self) -> Result<()> {
        if let Some(ref pipeline) = self.current_pipeline {
            if pipeline.is_eos() {
                info!("End of stream reached, advancing to next track");
                self.skip().await?;
            }
        }
        Ok(())
    }
}
