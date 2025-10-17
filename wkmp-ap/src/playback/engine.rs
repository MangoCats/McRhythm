//! Playback engine that orchestrates the audio pipeline
//!
//! This module connects PassageBufferManager, DecoderPool, CrossfadeMixer, and AudioOutput
//! to provide a complete playback system with queue management.
//!
//! Implements requirements from api_design.md - Audio Player API

use std::sync::Arc;
use std::path::PathBuf;
use std::collections::VecDeque;
use std::time::Duration;
use tokio::sync::{RwLock, mpsc};
use anyhow::{Result, anyhow, Context};
use uuid::Uuid;
use tracing::{info, debug, error};
use serde::{Serialize, Deserialize};

use crate::playback::pipeline::{
    PassageBufferManager,
    DecoderPool,
    CrossfadeMixer,
    DecodeRequest,
    DecodePriority,
};
use crate::playback::pipeline::single_stream::BufferStatus;

// Use simplified audio output for now to get compilation working
use crate::playback::pipeline::single_stream::output_simple::AudioOutput;

/// Playback state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlaybackState {
    /// Playback is active (or waiting for queue)
    Playing,
    /// Playback is paused
    Paused,
}

/// Queue entry with timing information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueEntry {
    /// Unique ID for this queue entry
    pub queue_entry_id: Uuid,
    /// Passage ID from database (optional)
    pub passage_id: Option<Uuid>,
    /// Play order in queue
    pub play_order: u32,
    /// Audio file path relative to root folder
    pub file_path: String,
    /// Timing override for this queue entry
    pub timing_override: Option<TimingOverride>,
}

/// Timing override for a queue entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingOverride {
    /// Start time in milliseconds
    pub start_time_ms: Option<u32>,
    /// End time in milliseconds
    pub end_time_ms: Option<u32>,
    /// Lead-in point in milliseconds
    pub lead_in_point_ms: Option<u32>,
    /// Lead-out point in milliseconds
    pub lead_out_point_ms: Option<u32>,
    /// Fade-in point in milliseconds
    pub fade_in_point_ms: Option<u32>,
    /// Fade-out point in milliseconds
    pub fade_out_point_ms: Option<u32>,
    /// Fade-in curve type
    pub fade_in_curve: Option<String>,
    /// Fade-out curve type
    pub fade_out_curve: Option<String>,
}

/// Enqueue request
#[derive(Debug, Clone, Deserialize)]
pub struct EnqueueRequest {
    /// File path relative to root folder
    pub file_path: String,
    /// Start time in milliseconds (optional)
    pub start_time_ms: Option<u32>,
    /// End time in milliseconds (optional)
    pub end_time_ms: Option<u32>,
    /// Lead-in point in milliseconds (optional)
    pub lead_in_point_ms: Option<u32>,
    /// Lead-out point in milliseconds (optional)
    pub lead_out_point_ms: Option<u32>,
    /// Fade-in point in milliseconds (optional)
    pub fade_in_point_ms: Option<u32>,
    /// Fade-out point in milliseconds (optional)
    pub fade_out_point_ms: Option<u32>,
    /// Fade-in curve type (optional)
    pub fade_in_curve: Option<String>,
    /// Fade-out curve type (optional)
    pub fade_out_curve: Option<String>,
    /// Passage GUID for identification (optional)
    pub passage_guid: Option<Uuid>,
    /// Position in queue (optional)
    pub position: Option<QueuePosition>,
}

/// Queue position for insertion
#[derive(Debug, Clone, Deserialize)]
pub struct QueuePosition {
    /// Position type
    #[serde(rename = "type")]
    pub position_type: PositionType,
    /// Reference GUID for relative positioning
    pub reference_guid: Option<Uuid>,
}

/// Position type for queue insertion
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PositionType {
    /// Insert after reference
    After,
    /// Insert before reference
    Before,
    /// Insert at specific order
    AtOrder,
    /// Append to end (default)
    Append,
}

/// Current playback position
#[derive(Debug, Clone, Serialize)]
pub struct PlaybackPosition {
    /// Current passage ID (if any)
    pub passage_id: Option<Uuid>,
    /// Position in milliseconds
    pub position_ms: u32,
    /// Duration in milliseconds
    pub duration_ms: u32,
    /// Current state
    pub state: PlaybackState,
}

/// Playback engine that orchestrates the audio pipeline
pub struct PlaybackEngine {
    /// Root folder for audio files
    root_folder: PathBuf,

    /// Current playback state
    state: Arc<RwLock<PlaybackState>>,

    /// Playback queue
    queue: Arc<RwLock<VecDeque<QueueEntry>>>,

    /// Currently playing entry
    current_entry: Arc<RwLock<Option<QueueEntry>>>,

    /// Next play order number
    next_play_order: Arc<RwLock<u32>>,

    /// Maximum queue size
    max_queue_size: usize,

    /// Passage buffer manager
    buffer_manager: Arc<PassageBufferManager>,

    /// Decoder pool for parallel decoding
    decoder_pool: Arc<DecoderPool>,

    /// Crossfade mixer
    mixer: Arc<CrossfadeMixer>,

    /// Audio output
    audio_output: Arc<AudioOutput>,

    /// Channel for engine commands
    command_tx: mpsc::Sender<EngineCommand>,
}

/// Commands for the playback engine
enum EngineCommand {
    Play,
    Pause,
    Enqueue(EnqueueRequest, mpsc::Sender<Result<QueueEntry>>),
    Dequeue(Uuid, mpsc::Sender<Result<()>>),
    Skip,
}

impl PlaybackEngine {
    /// Create a new playback engine
    pub async fn new(root_folder: PathBuf) -> Result<Self> {
        info!("Initializing PlaybackEngine");

        // Create pipeline components
        let buffer_manager = Arc::new(PassageBufferManager::new());
        let decoder_pool = Arc::new(
            DecoderPool::new(Arc::clone(&buffer_manager), Some(4))
        );
        let mixer = Arc::new(CrossfadeMixer::new(Arc::clone(&buffer_manager)));
        let audio_output = Arc::new(AudioOutput::new(Arc::clone(&mixer)).await?);

        // Create command channel
        let (command_tx, mut command_rx) = mpsc::channel::<EngineCommand>(32);

        let engine = Self {
            root_folder,
            state: Arc::new(RwLock::new(PlaybackState::Paused)),
            queue: Arc::new(RwLock::new(VecDeque::new())),
            current_entry: Arc::new(RwLock::new(None)),
            next_play_order: Arc::new(RwLock::new(10)),
            max_queue_size: 100,
            buffer_manager,
            decoder_pool,
            mixer,
            audio_output,
            command_tx,
        };

        // Spawn command handler
        let state = Arc::clone(&engine.state);
        let audio_output = Arc::clone(&engine.audio_output);
        let queue = Arc::clone(&engine.queue);
        let current_entry = Arc::clone(&engine.current_entry);
        let next_play_order = Arc::clone(&engine.next_play_order);
        let decoder_pool = Arc::clone(&engine.decoder_pool);
        let root_folder = engine.root_folder.clone();

        tokio::spawn(async move {
            while let Some(command) = command_rx.recv().await {
                match command {
                    EngineCommand::Play => {
                        *state.write().await = PlaybackState::Playing;
                        if let Err(e) = audio_output.play().await {
                            error!("Failed to start playback: {}", e);
                        }
                    }
                    EngineCommand::Pause => {
                        *state.write().await = PlaybackState::Paused;
                        if let Err(e) = audio_output.pause().await {
                            error!("Failed to pause playback: {}", e);
                        }
                    }
                    EngineCommand::Enqueue(request, reply) => {
                        info!("Enqueueing: {}", request.file_path);

                        // 1. Create queue entry
                        let queue_entry_id = Uuid::new_v4();
                        let passage_id = Uuid::new_v4();
                        let play_order = *next_play_order.write().await;
                        *next_play_order.write().await += 10;

                        let timing_override = request.start_time_ms.or(request.end_time_ms).map(|_| TimingOverride {
                            start_time_ms: request.start_time_ms,
                            end_time_ms: request.end_time_ms,
                            lead_in_point_ms: None,
                            lead_out_point_ms: None,
                            fade_in_point_ms: None,
                            fade_out_point_ms: None,
                            fade_in_curve: None,
                            fade_out_curve: None,
                        });

                        let entry = QueueEntry {
                            queue_entry_id,
                            passage_id: Some(passage_id),
                            play_order,
                            file_path: request.file_path.clone(),
                            timing_override,
                        };

                        // 2. Add to queue
                        queue.write().await.push_back(entry.clone());
                        info!("Added to queue: passage_id={}, play_order={}", passage_id, play_order);

                        // 3. Convert timing from ms to samples (44.1kHz)
                        let sample_rate = 44100;
                        let start_sample = request.start_time_ms
                            .map(|ms| (ms as u64 * sample_rate) / 1000)
                            .unwrap_or(0);
                        let end_sample = request.end_time_ms
                            .map(|ms| (ms as u64 * sample_rate) / 1000)
                            .unwrap_or(u64::MAX);

                        // 4. Create decode request
                        let file_full_path = root_folder.join(&request.file_path);
                        info!("Submitting decode request: {:?} (samples {} to {})", file_full_path, start_sample, end_sample);

                        let decode_req = DecodeRequest {
                            passage_id,
                            file_path: file_full_path,
                            start_sample,
                            end_sample,
                            priority: DecodePriority::Next,
                        };

                        // 5. Submit to decoder pool
                        match decoder_pool.decode_passage(decode_req).await {
                            Ok(_) => {
                                info!("Decode request submitted successfully for passage {}", passage_id);
                                let _ = reply.send(Ok(entry)).await;
                            }
                            Err(e) => {
                                error!("Failed to submit decode request: {}", e);
                                // Remove from queue on failure
                                queue.write().await.retain(|e| e.queue_entry_id != queue_entry_id);
                                let _ = reply.send(Err(e)).await;
                            }
                        }
                    }
                    EngineCommand::Dequeue(id, reply) => {
                        info!("Dequeueing: {}", id);
                        let mut q = queue.write().await;
                        let original_len = q.len();
                        q.retain(|e| e.queue_entry_id != id);
                        let removed = original_len != q.len();

                        if removed {
                            info!("Removed entry {} from queue", id);
                            let _ = reply.send(Ok(())).await;
                        } else {
                            let _ = reply.send(Err(anyhow!("Queue entry not found"))).await;
                        }
                    }
                    EngineCommand::Skip => {
                        info!("Skip requested");
                        // Move current entry to completed, advance to next
                        // TODO: Implement skip logic
                    }
                }
            }
        });

        // Spawn playback coordination loop
        let state_clone = Arc::clone(&engine.state);
        let queue_clone = Arc::clone(&engine.queue);
        let current_entry_clone = Arc::clone(&engine.current_entry);
        let mixer_clone = Arc::clone(&engine.mixer);
        let buffer_manager_clone = Arc::clone(&engine.buffer_manager);

        tokio::spawn(async move {
            playback_loop(
                state_clone,
                queue_clone,
                current_entry_clone,
                mixer_clone,
                buffer_manager_clone,
            ).await;
        });

        info!("PlaybackEngine initialized");
        Ok(engine)
    }

    /// Start playback
    pub async fn play(&self) -> Result<()> {
        debug!("Starting playback");
        self.command_tx.send(EngineCommand::Play).await
            .context("Failed to send play command")?;
        Ok(())
    }

    /// Pause playback
    pub async fn pause(&self) -> Result<()> {
        debug!("Pausing playback");
        self.command_tx.send(EngineCommand::Pause).await
            .context("Failed to send pause command")?;
        Ok(())
    }

    /// Get current playback state
    pub async fn get_state(&self) -> PlaybackState {
        *self.state.read().await
    }

    /// Get current playback position
    pub async fn get_position(&self) -> PlaybackPosition {
        let state = self.get_state().await;
        let current = self.current_entry.read().await;

        if let Some(entry) = current.as_ref() {
            // Get position from audio output
            let position_ms = self.audio_output.get_position_ms().await as u32;

            // TODO: Get actual duration from file
            let duration_ms = 300000; // 5 minutes placeholder

            PlaybackPosition {
                passage_id: entry.passage_id,
                position_ms,
                duration_ms,
                state,
            }
        } else {
            PlaybackPosition {
                passage_id: None,
                position_ms: 0,
                duration_ms: 0,
                state,
            }
        }
    }

    /// Enqueue a passage
    pub async fn enqueue(&self, request: EnqueueRequest) -> Result<QueueEntry> {
        debug!("Enqueueing passage: {:?}", request.file_path);

        // Check queue size
        if self.queue.read().await.len() >= self.max_queue_size {
            return Err(anyhow!("Queue is full"));
        }

        // Send enqueue command to the command handler which will:
        // 1. Create queue entry
        // 2. Add to queue
        // 3. Submit decode request
        let (reply_tx, mut reply_rx) = mpsc::channel(1);
        self.command_tx.send(EngineCommand::Enqueue(request, reply_tx)).await
            .context("Failed to send enqueue command")?;

        // Wait for the result
        reply_rx.recv().await
            .ok_or_else(|| anyhow!("No reply from enqueue command"))?
    }

    /// Remove entry from queue
    pub async fn dequeue(&self, queue_entry_id: Uuid) -> Result<()> {
        let (reply_tx, mut reply_rx) = mpsc::channel(1);
        self.command_tx.send(EngineCommand::Dequeue(queue_entry_id, reply_tx)).await
            .context("Failed to send dequeue command")?;

        reply_rx.recv().await
            .ok_or_else(|| anyhow!("No reply from engine"))?
    }

    /// Get queue contents
    pub async fn get_queue(&self) -> Vec<QueueEntry> {
        self.queue.read().await.iter().cloned().collect()
    }

    /// Process next item in queue
    async fn process_next(&self) -> Result<()> {
        let next_entry = self.queue.write().await.pop_front();

        if let Some(entry) = next_entry {
            info!("Processing queue entry: {:?}", entry.file_path);

            // Build full file path
            let file_path = self.root_folder.join(&entry.file_path);

            // Create decode request
            let passage_id = entry.passage_id.unwrap_or_else(Uuid::new_v4);

            // Convert milliseconds to samples (44100 Hz sample rate)
            const SAMPLE_RATE: u64 = 44100;
            let start_ms = entry.timing_override.as_ref()
                .and_then(|t| t.start_time_ms)
                .unwrap_or(0) as u64;
            let end_ms = entry.timing_override.as_ref()
                .and_then(|t| t.end_time_ms)
                .unwrap_or(300000) as u64; // TODO: Get actual duration

            let request = DecodeRequest {
                passage_id,
                file_path: file_path.clone(),
                start_sample: (start_ms * SAMPLE_RATE) / 1000,
                end_sample: (end_ms * SAMPLE_RATE) / 1000,
                priority: DecodePriority::Immediate,
            };

            // Submit decode request
            self.decoder_pool.decode_passage(request).await?;

            // Update current entry
            *self.current_entry.write().await = Some(entry);
        }

        Ok(())
    }
}

/// Background loop that coordinates playback between queue, buffer manager, and mixer
///
/// Implements requirement from single-stream-design.md: Queue-based playback management
async fn playback_loop(
    state: Arc<RwLock<PlaybackState>>,
    queue: Arc<RwLock<VecDeque<QueueEntry>>>,
    current_entry: Arc<RwLock<Option<QueueEntry>>>,
    mixer: Arc<CrossfadeMixer>,
    buffer_manager: Arc<PassageBufferManager>,
) {
    info!("Playback coordination loop started");

    loop {
        // Check if we're in playing state
        if *state.read().await != PlaybackState::Playing {
            tokio::time::sleep(Duration::from_millis(100)).await;
            continue;
        }

        // Get next entry from queue
        let next_entry = queue.write().await.pop_front();

        if let Some(entry) = next_entry {
            if let Some(initial_passage_id) = entry.passage_id {
                // Make passage_id and passage_duration_ms mutable for the continuous crossfade loop
                let mut passage_id = initial_passage_id;
                let mut passage_duration_ms;

                info!(
                    passage_id = %passage_id,
                    file_path = %entry.file_path,
                    "Starting passage playback"
                );

                // Wait for buffer to be ready
                loop {
                    match buffer_manager.get_status(&passage_id).await {
                        Some(BufferStatus::Ready) => {
                            info!(passage_id = %passage_id, "Buffer ready for playback");
                            break;
                        }
                        Some(BufferStatus::Decoding) => {
                            debug!(passage_id = %passage_id, "Waiting for buffer to finish decoding");
                            tokio::time::sleep(Duration::from_millis(100)).await;
                        }
                        Some(status) => {
                            error!(
                                passage_id = %passage_id,
                                status = ?status,
                                "Buffer in unexpected state"
                            );
                            break;
                        }
                        None => {
                            error!(passage_id = %passage_id, "Buffer not found");
                            break;
                        }
                    }
                }

                // Start passage in mixer
                if let Err(e) = mixer.start_passage(passage_id).await {
                    error!(
                        passage_id = %passage_id,
                        error = %e,
                        "Failed to start passage in mixer"
                    );
                    continue;
                }

                info!(passage_id = %passage_id, "Passage started in mixer");

                // Update current entry
                *current_entry.write().await = Some(entry.clone());

                // Get passage duration from buffer
                passage_duration_ms = if let Some(buffers) = buffer_manager.get_buffer(&passage_id).await {
                    if let Some(buffer) = buffers.get(&passage_id) {
                        let samples = buffer.pcm_data.len() / 2; // stereo
                        (samples as f64 / 44100.0 * 1000.0) as u64
                    } else {
                        10000 // default 10 seconds
                    }
                } else {
                    10000 // default 10 seconds
                };

                info!(
                    passage_id = %passage_id,
                    duration_ms = passage_duration_ms,
                    "Passage duration calculated"
                );

                // Continue playing this passage with crossfades to subsequent passages
                loop {
                    // Check if there's a next passage to crossfade to
                    let next_in_queue = queue.read().await.front().cloned();

                    if let Some(next_entry) = next_in_queue {
                        if let Some(next_id) = next_entry.passage_id {
                            // Queue next passage for crossfade
                            let fade_in_ms = 3000.0;
                            let fade_out_ms = 3000.0;
                            let overlap_ms = 3000.0;

                            info!(
                                next_passage_id = %next_id,
                                fade_in_ms,
                                fade_out_ms,
                                overlap_ms,
                                "Queueing next passage for crossfade"
                            );

                            if mixer.queue_next_passage(next_id, fade_in_ms, fade_out_ms, overlap_ms).await.is_ok() {
                                // Calculate when to start crossfade (3 seconds before end)
                                let crossfade_duration_ms = 3000;
                                let wait_before_crossfade_ms = passage_duration_ms.saturating_sub(crossfade_duration_ms);

                                info!(
                                    passage_id = %passage_id,
                                    wait_ms = wait_before_crossfade_ms,
                                    "Waiting before starting crossfade"
                                );

                                // Wait until it's time to start crossfade
                                tokio::time::sleep(Duration::from_millis(wait_before_crossfade_ms)).await;

                                // Start the crossfade
                                info!(passage_id = %passage_id, "Starting crossfade");
                                if let Err(e) = mixer.start_crossfade().await {
                                    error!(
                                        passage_id = %passage_id,
                                        error = %e,
                                        "Failed to start crossfade"
                                    );
                                    break;
                                }

                                // Wait for crossfade to complete
                                tokio::time::sleep(Duration::from_millis(crossfade_duration_ms)).await;
                                info!(passage_id = %passage_id, "Crossfade completed");

                                // Remove from queue and transition
                                queue.write().await.pop_front();
                                *current_entry.write().await = Some(next_entry.clone());

                                // Update for next iteration
                                passage_id = next_id;
                                passage_duration_ms = if let Some(buffers) = buffer_manager.get_buffer(&next_id).await {
                                    if let Some(buffer) = buffers.get(&next_id) {
                                        let samples = buffer.pcm_data.len() / 2; // stereo
                                        (samples as f64 / 44100.0 * 1000.0) as u64
                                    } else {
                                        10000
                                    }
                                } else {
                                    10000
                                };

                                info!(
                                    passage_id = %passage_id,
                                    duration_ms = passage_duration_ms,
                                    "Transitioned to next passage, continuing playback loop"
                                );

                                // Continue loop to process this passage
                                continue;
                            }
                        }
                    }

                    // No next passage or queueing failed - play remainder and exit loop
                    info!(passage_id = %passage_id, "No more passages to crossfade, playing full duration");
                    tokio::time::sleep(Duration::from_millis(passage_duration_ms)).await;
                    info!(passage_id = %passage_id, "Passage playback completed");
                    break;
                }
            } else {
                error!("Queue entry has no passage_id");
            }
        } else {
            // Queue empty, wait before checking again
            debug!("Queue empty, waiting for entries");
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }
}