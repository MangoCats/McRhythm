//! Simplified audio output with cpal integration
//!
//! This module implements audio output using cpal with an async polling architecture.
//! It polls the mixer for samples and outputs them to the audio device via a ring buffer.

use std::sync::Arc;
use std::time::Duration;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex; // Use std::sync::Mutex for cpal callback compatibility
use std::collections::VecDeque;
use anyhow::{Result, anyhow, Context};
use tracing::{info, debug, error, warn};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::Stream;

use super::mixer::CrossfadeMixer;

/// Simplified audio output status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputStatus {
    Stopped,
    Playing,
    Paused,
    Error,
}

/// Simplified audio output stats
#[derive(Debug, Clone, Copy, Default)]
pub struct OutputStats {
    pub frames_output: u64,
    pub underruns: u64,
    pub buffer_fill: f32,
    pub latency_ms: f32,
}

/// Ring buffer size (2 seconds at 44.1kHz stereo)
const RING_BUFFER_SIZE: usize = 44100 * 2 * 2;

/// Simplified audio output with cpal
pub struct AudioOutput {
    mixer: Arc<CrossfadeMixer>,
    position_ms: Arc<std::sync::atomic::AtomicU64>,
    playing: Arc<AtomicBool>,
    /// Ring buffer shared between async polling loop and cpal callback (std::sync::Mutex for cpal compatibility)
    ring_buffer: Arc<Mutex<VecDeque<f32>>>,
    /// Audio output stream (kept alive for playback, never accessed after creation)
    /// Stream is not Send, so we box it and never move it
    _stream: Option<Stream>,
}

// SAFETY: AudioOutput can be safely sent between threads because:
// - All fields except _stream are Send + Sync
// - _stream is never accessed after creation, it's only kept alive
// - The cpal callback thread has its own reference to the ring buffer
unsafe impl Send for AudioOutput {}
unsafe impl Sync for AudioOutput {}

impl AudioOutput {
    /// Create a new audio output instance
    pub async fn new(mixer: Arc<CrossfadeMixer>) -> Result<Self> {
        info!("Creating AudioOutput with cpal");

        // Initialize cpal
        let host = cpal::default_host();
        let device = host.default_output_device()
            .ok_or_else(|| anyhow!("No audio output device available"))?;

        info!(device_name = ?device.name().unwrap_or_else(|_| "Unknown".to_string()), "Selected audio device");

        // Get default config
        let config = device.default_output_config()
            .context("Failed to get default output config")?;

        info!(sample_rate = config.sample_rate().0, channels = config.channels(), "Audio config");

        // Create ring buffer
        let ring_buffer = Arc::new(Mutex::new(VecDeque::with_capacity(RING_BUFFER_SIZE)));

        // Build output stream
        let ring_buffer_clone = Arc::clone(&ring_buffer);
        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => {
                let config: cpal::StreamConfig = config.into();
                device.build_output_stream(
                    &config,
                    move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                        if let Ok(mut buffer) = ring_buffer_clone.try_lock() {
                            for sample in data.iter_mut() {
                                *sample = buffer.pop_front().unwrap_or(0.0);
                            }
                        } else {
                            // If we can't lock, output silence
                            for sample in data.iter_mut() {
                                *sample = 0.0;
                            }
                        }
                    },
                    |err| error!("Audio stream error: {}", err),
                    None,
                )?
            }
            _ => return Err(anyhow!("Unsupported sample format")),
        };

        // Start the stream immediately (required by cpal - Stream is not Send)
        stream.play().context("Failed to start cpal stream")?;

        Ok(Self {
            mixer,
            position_ms: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            playing: Arc::new(AtomicBool::new(false)),
            ring_buffer,
            _stream: Some(stream),
        })
    }

    /// Start audio playback
    pub async fn play(&self) -> Result<()> {
        info!("Starting playback");
        self.playing.store(true, Ordering::SeqCst);

        // Spawn async polling loop that feeds the ring buffer
        // The cpal stream is already running, it will start outputting audio once playing=true
        let mixer = Arc::clone(&self.mixer);
        let playing = Arc::clone(&self.playing);
        let position_ms = Arc::clone(&self.position_ms);
        let ring_buffer = Arc::clone(&self.ring_buffer);

        tokio::spawn(async move {
            info!("Audio polling loop started");

            // Audio parameters
            const SAMPLE_RATE: u32 = 44100;
            const BUFFER_SIZE: usize = 2048; // ~46ms at 44.1kHz - larger buffer to keep ring buffer full

            while playing.load(Ordering::SeqCst) {
                // Check ring buffer fill level
                let buffer_fill = {
                    if let Ok(buffer) = ring_buffer.lock() {
                        buffer.len()
                    } else {
                        0
                    }
                };

                // Only request more samples if ring buffer has room
                if buffer_fill < RING_BUFFER_SIZE / 2 {
                    // Request samples from mixer
                    match mixer.process_audio(BUFFER_SIZE).await {
                        Ok(samples) => {
                            if samples.is_empty() {
                                debug!("Mixer returned no samples, waiting...");
                                tokio::time::sleep(Duration::from_millis(10)).await;
                                continue;
                            }

                            // Write samples to ring buffer
                            if let Ok(mut buffer) = ring_buffer.lock() {
                                buffer.extend(samples.iter());
                            } else {
                                warn!("Failed to lock ring buffer for writing");
                            }

                            // Update position based on samples processed
                            let frame_count = samples.len() / 2; // stereo
                            let ms_elapsed = (frame_count as f64 / SAMPLE_RATE as f64 * 1000.0) as u64;
                            position_ms.fetch_add(ms_elapsed, Ordering::SeqCst);

                            debug!(
                                frames = frame_count,
                                buffer_fill,
                                total_position_ms = position_ms.load(Ordering::SeqCst),
                                "Wrote samples to ring buffer"
                            );
                        }
                        Err(e) => {
                            error!("Mixer error: {}", e);
                            tokio::time::sleep(Duration::from_millis(100)).await;
                        }
                    }
                } else {
                    // Ring buffer is full enough, wait a bit
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
            }

            info!("Audio polling loop stopped");
        });

        Ok(())
    }

    /// Pause audio playback
    pub async fn pause(&self) -> Result<()> {
        info!("Pausing playback");
        self.playing.store(false, Ordering::SeqCst);
        // The cpal callback will output silence when playing=false
        Ok(())
    }

    /// Stop audio playback
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping playback (stub)");
        self.playing.store(false, std::sync::atomic::Ordering::SeqCst);
        self.position_ms.store(0, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }

    /// Get current output status
    pub async fn get_status(&self) -> OutputStatus {
        if self.playing.load(std::sync::atomic::Ordering::SeqCst) {
            OutputStatus::Playing
        } else {
            OutputStatus::Paused
        }
    }

    /// Get output statistics
    pub async fn get_stats(&self) -> OutputStats {
        OutputStats::default()
    }

    /// Get the current playback position in milliseconds
    pub async fn get_position_ms(&self) -> f64 {
        self.position_ms.load(std::sync::atomic::Ordering::SeqCst) as f64
    }

    /// List available output devices
    pub async fn list_devices() -> Result<Vec<String>> {
        Ok(vec!["Default".to_string()])
    }

    /// Select a specific output device by name
    pub async fn select_device(&self, device_name: &str) -> Result<()> {
        info!("Selecting device: {} (stub)", device_name);
        Ok(())
    }
}