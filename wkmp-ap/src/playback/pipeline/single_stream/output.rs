//! Cross-platform audio output using cpal
//!
//! This module implements the audio output component that receives mixed PCM data
//! from the CrossfadeMixer and outputs it to the system audio device.
//!
//! Implements requirements from single-stream-design.md - Phase 3: Audio Output

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::collections::VecDeque;
use tokio::sync::{RwLock, mpsc, Mutex};
use anyhow::{Result, anyhow, Context};
use tracing::{info, warn, error};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Stream, StreamConfig, SampleFormat, SampleRate};

use super::mixer::CrossfadeMixer;

/// Standard sample rate for audio output
const STANDARD_SAMPLE_RATE: u32 = 44100;

/// Target buffer size in milliseconds
const TARGET_BUFFER_MS: u32 = 50;

/// Ring buffer size (in frames)
const RING_BUFFER_SIZE: usize = STANDARD_SAMPLE_RATE as usize * 2; // 2 seconds

/// Audio output status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputStatus {
    /// Output is stopped
    Stopped,
    /// Output is playing
    Playing,
    /// Output is paused
    Paused,
    /// Output is in error state
    Error,
}

/// Statistics for audio output performance
#[derive(Debug, Clone, Copy, Default)]
pub struct OutputStats {
    /// Total frames output
    pub frames_output: u64,
    /// Number of buffer underruns
    pub underruns: u64,
    /// Current buffer fill level (0.0 to 1.0)
    pub buffer_fill: f32,
    /// Output latency in milliseconds
    pub latency_ms: f32,
}

/// Cross-platform audio output using cpal
///
/// This component:
/// 1. Manages the audio output device
/// 2. Maintains a ring buffer for smooth playback
/// 3. Pulls mixed audio from the CrossfadeMixer
/// 4. Handles device selection and error recovery
pub struct AudioOutput {
    /// Reference to the crossfade mixer
    mixer: Arc<CrossfadeMixer>,

    /// The audio output stream
    stream: Arc<Mutex<Option<Stream>>>,

    /// Current output device
    device: Arc<RwLock<Option<Device>>>,

    /// Ring buffer for audio data
    ring_buffer: Arc<RwLock<VecDeque<f32>>>,

    /// Current output status
    status: Arc<RwLock<OutputStatus>>,

    /// Output statistics
    stats: Arc<RwLock<OutputStats>>,

    /// Flag to control playback
    playing: Arc<AtomicBool>,

    /// Total frames output
    frames_output: Arc<AtomicU64>,

    /// Channel for sending commands to the audio thread
    command_tx: mpsc::Sender<OutputCommand>,
}

/// Commands for controlling audio output
enum OutputCommand {
    Play,
    Pause,
    Stop,
    UpdateVolume(f32),
}

impl AudioOutput {
    /// Create a new audio output instance
    pub async fn new(mixer: Arc<CrossfadeMixer>) -> Result<Self> {
        info!("Initializing AudioOutput with cpal");

        // Create command channel
        let (command_tx, command_rx) = mpsc::channel::<OutputCommand>(32);

        // Initialize cpal host
        let host = cpal::default_host();

        // Get default output device
        let device = host.default_output_device()
            .ok_or_else(|| anyhow!("No audio output device available"))?;

        info!(
            device_name = ?device.name().unwrap_or_else(|_| "Unknown".to_string()),
            "Selected audio output device"
        );

        // Get device configuration
        let supported_configs = device.supported_output_configs()
            .context("Failed to get device configurations")?;

        // Find a suitable configuration (prefer 44.1kHz stereo)
        let config = find_best_config(supported_configs)?;
        info!(?config, "Selected audio configuration");

        // Create ring buffer
        let ring_buffer = Arc::new(RwLock::new(VecDeque::with_capacity(RING_BUFFER_SIZE * 2)));

        // Create shared state
        let status = Arc::new(RwLock::new(OutputStatus::Stopped));
        let stats = Arc::new(RwLock::new(OutputStats::default()));
        let playing = Arc::new(AtomicBool::new(false));
        let frames_output = Arc::new(AtomicU64::new(0));

        // Create the audio stream
        let stream = create_output_stream(
            &device,
            &config,
            Arc::clone(&ring_buffer),
            Arc::clone(&mixer),
            Arc::clone(&playing),
            Arc::clone(&frames_output),
            Arc::clone(&stats),
        )?;

        let stream = Arc::new(Mutex::new(Some(stream)));

        Ok(Self {
            mixer,
            stream,
            device: Arc::new(RwLock::new(Some(device))),
            ring_buffer,
            status,
            stats,
            playing,
            frames_output,
            command_tx,
        })
    }

    /// Start audio playback
    pub async fn play(&self) -> Result<()> {
        info!("Starting audio playback");

        // Set playing flag
        self.playing.store(true, Ordering::SeqCst);

        // Start the stream
        if let Some(stream) = self.stream.lock().await.as_ref() {
            stream.play().context("Failed to start audio stream")?;
            *self.status.write().await = OutputStatus::Playing;
            info!("Audio playback started");
        } else {
            return Err(anyhow!("No audio stream available"));
        }

        Ok(())
    }

    /// Pause audio playback
    pub async fn pause(&self) -> Result<()> {
        info!("Pausing audio playback");

        // Clear playing flag
        self.playing.store(false, Ordering::SeqCst);

        // Pause the stream
        if let Some(stream) = self.stream.lock().await.as_ref() {
            stream.pause().context("Failed to pause audio stream")?;
            *self.status.write().await = OutputStatus::Paused;
            info!("Audio playback paused");
        }

        Ok(())
    }

    /// Stop audio playback
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping audio playback");

        // Clear playing flag
        self.playing.store(false, Ordering::SeqCst);

        // Stop and drop the stream
        let mut stream_guard = self.stream.lock().await;
        if let Some(stream) = stream_guard.take() {
            drop(stream); // This stops the stream
        }

        // Clear the ring buffer
        self.ring_buffer.write().await.clear();

        // Reset statistics
        self.frames_output.store(0, Ordering::SeqCst);
        *self.stats.write().await = OutputStats::default();

        *self.status.write().await = OutputStatus::Stopped;
        info!("Audio playback stopped");

        Ok(())
    }

    /// Get current output status
    pub async fn get_status(&self) -> OutputStatus {
        *self.status.read().await
    }

    /// Get output statistics
    pub async fn get_stats(&self) -> OutputStats {
        *self.stats.read().await
    }

    /// Get the current playback position in milliseconds
    pub async fn get_position_ms(&self) -> f64 {
        let frames = self.frames_output.load(Ordering::SeqCst);
        frames as f64 * 1000.0 / STANDARD_SAMPLE_RATE as f64
    }

    /// List available output devices
    pub async fn list_devices() -> Result<Vec<String>> {
        let host = cpal::default_host();
        let devices = host.output_devices()
            .context("Failed to enumerate output devices")?;

        let mut device_names = Vec::new();
        for device in devices {
            if let Ok(name) = device.name() {
                device_names.push(name);
            }
        }

        Ok(device_names)
    }

    /// Select a specific output device by name
    pub async fn select_device(&self, device_name: &str) -> Result<()> {
        info!(device_name, "Selecting output device");

        let host = cpal::default_host();
        let devices = host.output_devices()
            .context("Failed to enumerate output devices")?;

        for device in devices {
            if let Ok(name) = device.name() {
                if name == device_name {
                    // Stop current playback
                    self.stop().await?;

                    // Update device
                    *self.device.write().await = Some(device);

                    info!(device_name, "Output device selected");
                    return Ok(());
                }
            }
        }

        Err(anyhow!("Device '{}' not found", device_name))
    }
}

/// Find the best audio configuration for our needs
fn find_best_config(
    configs: impl Iterator<Item = cpal::SupportedStreamConfigRange>
) -> Result<StreamConfig> {
    // Collect all configs
    let configs: Vec<_> = configs.collect();

    // First, try to find exact match: 44.1kHz, 2 channels, f32
    for config in &configs {
        if config.channels() == 2
            && config.min_sample_rate() <= SampleRate(STANDARD_SAMPLE_RATE)
            && config.max_sample_rate() >= SampleRate(STANDARD_SAMPLE_RATE)
            && config.sample_format() == SampleFormat::F32 {

            return Ok(StreamConfig {
                channels: 2,
                sample_rate: SampleRate(STANDARD_SAMPLE_RATE),
                buffer_size: cpal::BufferSize::Default,
            });
        }
    }

    // If no exact match, take the first stereo config and we'll resample if needed
    for config in &configs {
        if config.channels() == 2 {
            let sample_rate = if config.min_sample_rate() <= SampleRate(STANDARD_SAMPLE_RATE)
                && config.max_sample_rate() >= SampleRate(STANDARD_SAMPLE_RATE) {
                SampleRate(STANDARD_SAMPLE_RATE)
            } else {
                config.max_sample_rate()
            };

            return Ok(StreamConfig {
                channels: 2,
                sample_rate,
                buffer_size: cpal::BufferSize::Default,
            });
        }
    }

    // Last resort: use the first available config
    configs.into_iter()
        .next()
        .map(|c| c.with_max_sample_rate().into())
        .ok_or_else(|| anyhow!("No audio configurations available"))
}

/// Create the audio output stream
fn create_output_stream(
    device: &Device,
    config: &StreamConfig,
    ring_buffer: Arc<RwLock<VecDeque<f32>>>,
    mixer: Arc<CrossfadeMixer>,
    playing: Arc<AtomicBool>,
    frames_output: Arc<AtomicU64>,
    stats: Arc<RwLock<OutputStats>>,
) -> Result<Stream> {
    let channels = config.channels as usize;
    let sample_rate = config.sample_rate.0;

    info!(
        channels,
        sample_rate,
        "Creating audio output stream"
    );

    // Calculate buffer size
    let buffer_size = (sample_rate as usize * TARGET_BUFFER_MS as usize / 1000) * channels;

    let err_fn = |err| {
        error!("Audio stream error: {}", err);
    };

    // Create the output callback
    let stream = device.build_output_stream(
        config,
        move |data: &mut [f32], _info: &cpal::OutputCallbackInfo| {
            // If not playing, output silence
            if !playing.load(Ordering::SeqCst) {
                for sample in data.iter_mut() {
                    *sample = 0.0;
                }
                return;
            }

            // Try to get data from ring buffer
            let runtime = tokio::runtime::Handle::try_current();
            if runtime.is_err() {
                // No tokio runtime in audio callback, output silence
                for sample in data.iter_mut() {
                    *sample = 0.0;
                }
                return;
            }

            // Use blocking to access async resources from sync callback
            let result = tokio::task::block_in_place(|| {
                runtime.unwrap().block_on(async {
                    fill_output_buffer(
                        data,
                        &ring_buffer,
                        &mixer,
                        channels,
                    ).await
                })
            });

            if let Ok(frames) = result {
                frames_output.fetch_add(frames as u64, Ordering::SeqCst);
            }
        },
        err_fn,
        None, // No timeout
    )?;

    Ok(stream)
}

/// Fill the output buffer with audio data
async fn fill_output_buffer(
    output: &mut [f32],
    ring_buffer: &Arc<RwLock<VecDeque<f32>>>,
    mixer: &Arc<CrossfadeMixer>,
    channels: usize,
) -> Result<usize> {
    let mut buffer = ring_buffer.write().await;

    // Calculate how many samples we need
    let samples_needed = output.len();

    // If ring buffer doesn't have enough, request more from mixer
    while buffer.len() < samples_needed {
        let chunk_size = samples_needed.max(STANDARD_SAMPLE_RATE as usize / 10); // At least 100ms
        match mixer.process_audio(chunk_size).await {
            Ok(audio_data) => {
                buffer.extend(audio_data);
            }
            Err(e) => {
                warn!("Failed to get audio from mixer: {}", e);
                break;
            }
        }
    }

    // Fill output buffer
    let samples_available = buffer.len().min(samples_needed);
    for i in 0..samples_available {
        if let Some(sample) = buffer.pop_front() {
            output[i] = sample;
        } else {
            output[i] = 0.0; // Silence for underrun
        }
    }

    // Fill remaining with silence if underrun
    for i in samples_available..samples_needed {
        output[i] = 0.0;
    }

    // Return number of frames (not samples)
    Ok(samples_available / channels)
}

impl Drop for AudioOutput {
    fn drop(&mut self) {
        // Stop playback on drop
        self.playing.store(false, Ordering::SeqCst);

        // The stream will be dropped automatically
        info!("AudioOutput dropped");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_list_devices() {
        let devices = AudioOutput::list_devices().await;
        assert!(devices.is_ok());

        let device_list = devices.unwrap();
        println!("Available audio devices: {:?}", device_list);
    }
}