//! Playback engine coordinating GStreamer and queue

use anyhow::Result;
use sqlx::SqlitePool;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn, error};

use super::pipeline::{DualPipeline, ActivePipeline};
use super::queue::QueueManager;
use super::state::{PlaybackState as State, SharedPlaybackState};
use crate::sse::{SseBroadcaster, SseEvent, SseEventData};

/// Playback engine coordinates GStreamer pipelines and queue management
pub struct PlaybackEngine {
    root_folder: PathBuf,
    queue: QueueManager,
    state: SharedPlaybackState,
    pipeline: Option<DualPipeline>,
    sse_broadcaster: Option<SseBroadcaster>,
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
            pipeline: None,
            sse_broadcaster: None,
        }
    }

    /// Set the SSE broadcaster for emitting events
    pub fn set_sse_broadcaster(&mut self, broadcaster: SseBroadcaster) {
        self.sse_broadcaster = Some(broadcaster);
    }

    /// Emit a playback state changed event
    fn emit_state_changed(&self, state: State) {
        if let Some(ref broadcaster) = self.sse_broadcaster {
            let event_data = SseEventData::playback_state_changed(&state.to_string());
            let sse_event = SseEvent::new("playback_state_changed", event_data);
            broadcaster.broadcast_lossy(sse_event);
        }
    }

    /// Initialize the playback engine
    pub async fn init(&mut self) -> Result<()> {
        info!("Initializing playback engine...");

        // Initialize queue manager
        self.queue.init().await?;

        // Create dual pipeline
        let pipeline = DualPipeline::new()?;
        self.pipeline = Some(pipeline);

        info!("Playback engine initialized with dual pipeline");
        Ok(())
    }

    /// Load and prepare the next track from queue
    async fn load_next_track(&mut self) -> Result<()> {
        // Get pipeline
        let pipeline = self.pipeline.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Pipeline not initialized"))?;

        // Get next entry from queue
        let entry = self.queue.get_next().await
            .ok_or_else(|| anyhow::anyhow!("Queue is empty"))?;

        info!("Loading track: {}", entry.file_path);

        // Build full path
        let full_path = self.root_folder.join(&entry.file_path);

        // Determine which pipeline to load into
        let active = pipeline.active().await;
        let currently_playing = self.state.get_currently_playing().await;

        // If nothing is currently playing, load into active pipeline (initial load)
        // Otherwise, load into inactive pipeline for crossfade pre-loading
        let target = if currently_playing.is_none() {
            info!("Initial load - using active pipeline {:?}", active);
            active
        } else {
            info!("Pre-loading next track into inactive pipeline");
            active.other()
        };

        // Load file into target pipeline
        pipeline.load_file(target, &full_path).await?;

        info!("Loaded track into pipeline {:?}", target);

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
                if let Some(ref pipeline) = self.pipeline {
                    pipeline.play()?;
                    self.state.set_state(State::Playing).await;
                    self.emit_state_changed(State::Playing);
                    info!("Playback started");
                } else {
                    error!("Pipeline not initialized");
                    return Err(anyhow::anyhow!("Pipeline not initialized"));
                }
            }
            State::Paused => {
                // Resume playback
                info!("Resuming playback");
                if let Some(ref pipeline) = self.pipeline {
                    pipeline.play()?;
                    self.state.set_state(State::Playing).await;
                    self.emit_state_changed(State::Playing);
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
            if let Some(ref pipeline) = self.pipeline {
                pipeline.pause()?;
                self.state.set_state(State::Paused).await;
                self.emit_state_changed(State::Paused);
            }
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

        // Switch to next pipeline and start loading the next track
        if let Some(ref pipeline) = self.pipeline {
            // Switch active pipeline
            pipeline.switch_active().await;
            info!("Switched to next pipeline");
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
        if let Some(ref pipeline) = self.pipeline {
            if let Some(pos_ms) = pipeline.position_ms().await {
                if let Some(dur_ms) = pipeline.duration_ms().await {
                    self.state.set_position(pos_ms as u64, dur_ms as u64).await;
                }
            }
        }
    }

    /// Check if end of stream and advance to next track
    pub async fn check_eos(&mut self) -> Result<()> {
        if let Some(ref pipeline) = self.pipeline {
            if pipeline.is_eos() {
                info!("End of stream reached, advancing to next track");
                self.skip().await?;
            }
        }
        Ok(())
    }

    /// Set volume (0.0 to 1.0)
    pub async fn set_volume(&self, volume: f64) -> Result<()> {
        let clamped_volume = volume.clamp(0.0, 1.0);

        // Update state
        self.state.set_volume(clamped_volume).await;

        // Update master volume on pipeline
        if let Some(ref pipeline) = self.pipeline {
            pipeline.set_master_volume(clamped_volume).await?;
        }

        // Emit volume changed event (0-100 scale for user-facing)
        if let Some(ref broadcaster) = self.sse_broadcaster {
            let volume_percent = (clamped_volume * 100.0) as i32;
            let event_data = SseEventData::volume_changed(volume_percent);
            let sse_event = SseEvent::new("volume_changed", event_data);
            broadcaster.broadcast_lossy(sse_event);
        }

        info!("Volume set to {:.2}", clamped_volume);
        Ok(())
    }

    /// Seek to position in milliseconds
    pub async fn seek(&self, position_ms: i64) -> Result<()> {
        if let Some(ref pipeline) = self.pipeline {
            pipeline.seek_to(position_ms)?;
            info!("Seeked to {}ms", position_ms);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Pipeline not initialized"))
        }
    }
}
