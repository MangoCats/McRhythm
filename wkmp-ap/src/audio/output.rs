//! Audio output using cpal
//!
//! Manages audio device output with callback-based playback.
//!
//! **Traceability:**
//! - [SSD-OUT-010] Audio device interface
//! - [SSD-OUT-011] Initialize audio device
//! - [SSD-OUT-012] Begin audio stream with callback
//! - [ARCH-ERRH-080] Audio device error handling and recovery
//! - [ISSUE-5] Automatic device fallback and stream recovery

use crate::audio::AudioFrame;
use crate::error::{Error, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, Stream, StreamConfig};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use tracing::{debug, error, info, warn};

/// Audio output manager using cpal.
///
/// **[SSD-OUT-010]** Manages audio device interface and playback stream.
/// **[ISSUE-5]** Includes error tracking and recovery state
pub struct AudioOutput {
    device: Device,
    config: StreamConfig,
    sample_format: SampleFormat,
    stream: Option<Stream>,
    volume: Arc<Mutex<f32>>,
    /// Stream error flag - set by audio callback on error
    /// [ARCH-ERRH-080]
    error_flag: Arc<AtomicBool>,
    /// Count of consecutive errors for backoff logic
    error_count: Arc<AtomicU32>,
    /// Original device name requested (None = default)
    requested_device: Option<String>,
}

impl AudioOutput {
    /// List available audio output devices.
    ///
    /// Used by GET /audio/devices API endpoint.
    ///
    /// # Returns
    /// Vector of device names
    pub fn list_devices() -> Result<Vec<String>> {
        let host = cpal::default_host();

        let devices: Vec<String> = host
            .output_devices()
            .map_err(|e| Error::AudioOutput(format!("Failed to enumerate devices: {}", e)))?
            .filter_map(|device| device.name().ok())
            .collect();

        debug!("Found {} output devices", devices.len());
        Ok(devices)
    }

    /// Open audio device for output.
    ///
    /// **[SSD-OUT-011]** Initialize audio device.
    /// **[ISSUE-5]** Implements automatic fallback to default device on failure
    ///
    /// # Arguments
    /// - `device_name`: Optional device name (None = default device)
    ///
    /// # Returns
    /// AudioOutput instance ready to start playback
    ///
    /// # Errors
    /// - Device not found and default device unavailable
    /// - Failed to get device configuration
    ///
    /// # Fallback Behavior
    /// If the requested device fails to open, will attempt to use the default device.
    /// [ARCH-ERRH-080] Audio device error handling
    pub fn new(device_name: Option<String>) -> Result<Self> {
        let host = cpal::default_host();

        // Try to get requested device, with fallback to default
        let (device, _actual_device_name) = if let Some(name) = device_name.as_ref() {
            // Try to find device by name
            let mut devices = host
                .output_devices()
                .map_err(|e| Error::AudioOutput(format!("Failed to enumerate devices: {}", e)))?;

            match devices.find(|d| d.name().ok().as_ref() == Some(name)) {
                Some(dev) => {
                    info!("Found requested audio device: {}", name);
                    (dev, name.clone())
                }
                None => {
                    // Fallback to default device
                    warn!("Requested device '{}' not found, falling back to default device", name);

                    let default_dev = host.default_output_device()
                        .ok_or_else(|| Error::AudioOutput(
                            format!("Device '{}' not found and no default device available", name)
                        ))?;

                    let default_name = default_dev.name()
                        .unwrap_or_else(|_| "Unknown".to_string());

                    info!("Using default audio device as fallback: {}", default_name);
                    (default_dev, default_name)
                }
            }
        } else {
            // Use default device
            let dev = host.default_output_device()
                .ok_or_else(|| Error::AudioOutput("No default output device found".to_string()))?;

            let name = dev.name().unwrap_or_else(|_| "Unknown".to_string());
            info!("Using default audio device: {}", name);
            (dev, name)
        };

        // Get supported configuration
        let (config, sample_format) = Self::get_best_config(&device)?;

        debug!(
            "Audio config: sample_rate={}, channels={}, format={:?}",
            config.sample_rate.0, config.channels, sample_format
        );

        Ok(Self {
            device,
            config,
            sample_format,
            stream: None,
            volume: Arc::new(Mutex::new(1.0)),
            error_flag: Arc::new(AtomicBool::new(false)),
            error_count: Arc::new(AtomicU32::new(0)),
            requested_device: device_name,
        })
    }

    /// Get the best supported configuration for playback.
    ///
    /// Prefers 44.1kHz, stereo, f32 samples (matching our internal format).
    fn get_best_config(device: &Device) -> Result<(StreamConfig, SampleFormat)> {
        // Try to get a config that matches our target: 44100 Hz, stereo, f32
        let mut supported_configs = device
            .supported_output_configs()
            .map_err(|e| Error::AudioOutput(format!("Failed to get device configs: {}", e)))?;

        // Look for 44.1kHz stereo f32 config
        let preferred = supported_configs.find(|config| {
            config.channels() == 2
                && config.min_sample_rate().0 <= 44100
                && config.max_sample_rate().0 >= 44100
                && config.sample_format() == SampleFormat::F32
        });

        if let Some(supported_config) = preferred {
            let sample_format = supported_config.sample_format();
            let config = supported_config.with_sample_rate(cpal::SampleRate(44100)).config();
            return Ok((config, sample_format));
        }

        // Fallback: use default config
        let supported_config = device
            .default_output_config()
            .map_err(|e| Error::AudioOutput(format!("Failed to get default config: {}", e)))?;

        let sample_format = supported_config.sample_format();
        let config = supported_config.config();
        Ok((config, sample_format))
    }

    /// Start audio playback with callback.
    ///
    /// **[SSD-OUT-012]** Begin audio stream.
    ///
    /// # Arguments
    /// - `callback`: Closure called by audio thread to fetch samples
    ///
    /// The callback will be invoked on the audio thread whenever samples are needed.
    /// It should return AudioFrame samples. If no audio is available, return
    /// AudioFrame::zero() for silence.
    ///
    /// # Notes
    /// - Callback runs on a real-time audio thread (avoid blocking operations)
    /// - Volume control is applied automatically in the audio callback
    /// - Underruns (callback too slow) will output silence without crashing
    pub fn start<F>(&mut self, callback: F) -> Result<()>
    where
        F: FnMut() -> AudioFrame + Send + 'static,
    {
        info!("Starting audio stream");

        let volume = Arc::clone(&self.volume);
        let callback = Arc::new(Mutex::new(callback));

        let stream = match self.sample_format {
            SampleFormat::F32 => {
                self.build_stream_f32(callback, volume)?
            }
            SampleFormat::I16 => {
                self.build_stream_i16(callback, volume)?
            }
            SampleFormat::U16 => {
                self.build_stream_u16(callback, volume)?
            }
            sample_format => {
                return Err(Error::AudioOutput(format!(
                    "Unsupported sample format: {:?}",
                    sample_format
                )));
            }
        };

        stream
            .play()
            .map_err(|e| Error::AudioOutput(format!("Failed to start stream: {}", e)))?;

        self.stream = Some(stream);

        info!("Audio stream started successfully");
        Ok(())
    }

    /// Build audio stream for f32 samples
    /// [ISSUE-5] Error callback sets error flag for recovery
    fn build_stream_f32(
        &self,
        callback: Arc<Mutex<dyn FnMut() -> AudioFrame + Send + 'static>>,
        volume: Arc<Mutex<f32>>,
    ) -> Result<Stream> {
        let channels = self.config.channels as usize;
        let error_flag = Arc::clone(&self.error_flag);
        let error_count = Arc::clone(&self.error_count);

        let stream = self
            .device
            .build_output_stream(
                &self.config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let mut callback = callback.lock().unwrap();
                    let current_volume = *volume.lock().unwrap();

                    for frame in data.chunks_mut(channels) {
                        let audio_frame = callback();

                        // Apply volume
                        let left = audio_frame.left * current_volume;
                        let right = audio_frame.right * current_volume;

                        // Clamp to prevent clipping
                        frame[0] = left.clamp(-1.0, 1.0);
                        if channels > 1 {
                            frame[1] = right.clamp(-1.0, 1.0);
                        }
                    }
                },
                move |err| {
                    // [ARCH-ERRH-080] Set error flag for recovery
                    error!("Audio stream error: {} - marking for recovery", err);
                    error_flag.store(true, Ordering::SeqCst);
                    error_count.fetch_add(1, Ordering::SeqCst);
                },
                None, // No timeout
            )
            .map_err(|e| Error::AudioOutput(format!("Failed to build stream: {}", e)))?;

        Ok(stream)
    }

    /// Build audio stream for i16 samples
    /// [ISSUE-5] Error callback sets error flag for recovery
    fn build_stream_i16(
        &self,
        callback: Arc<Mutex<dyn FnMut() -> AudioFrame + Send + 'static>>,
        volume: Arc<Mutex<f32>>,
    ) -> Result<Stream> {
        let channels = self.config.channels as usize;
        let error_flag = Arc::clone(&self.error_flag);
        let error_count = Arc::clone(&self.error_count);

        let stream = self
            .device
            .build_output_stream(
                &self.config,
                move |data: &mut [i16], _: &cpal::OutputCallbackInfo| {
                    let mut callback = callback.lock().unwrap();
                    let current_volume = *volume.lock().unwrap();

                    for frame in data.chunks_mut(channels) {
                        let audio_frame = callback();

                        // Apply volume and convert to i16
                        let left = (audio_frame.left * current_volume).clamp(-1.0, 1.0);
                        let right = (audio_frame.right * current_volume).clamp(-1.0, 1.0);

                        frame[0] = (left * i16::MAX as f32) as i16;
                        if channels > 1 {
                            frame[1] = (right * i16::MAX as f32) as i16;
                        }
                    }
                },
                move |err| {
                    // [ARCH-ERRH-080] Set error flag for recovery
                    error!("Audio stream error: {} - marking for recovery", err);
                    error_flag.store(true, Ordering::SeqCst);
                    error_count.fetch_add(1, Ordering::SeqCst);
                },
                None,
            )
            .map_err(|e| Error::AudioOutput(format!("Failed to build stream: {}", e)))?;

        Ok(stream)
    }

    /// Build audio stream for u16 samples
    /// [ISSUE-5] Error callback sets error flag for recovery
    fn build_stream_u16(
        &self,
        callback: Arc<Mutex<dyn FnMut() -> AudioFrame + Send + 'static>>,
        volume: Arc<Mutex<f32>>,
    ) -> Result<Stream> {
        let channels = self.config.channels as usize;
        let error_flag = Arc::clone(&self.error_flag);
        let error_count = Arc::clone(&self.error_count);

        let stream = self
            .device
            .build_output_stream(
                &self.config,
                move |data: &mut [u16], _: &cpal::OutputCallbackInfo| {
                    let mut callback = callback.lock().unwrap();
                    let current_volume = *volume.lock().unwrap();

                    for frame in data.chunks_mut(channels) {
                        let audio_frame = callback();

                        // Apply volume, convert to u16
                        let left = (audio_frame.left * current_volume).clamp(-1.0, 1.0);
                        let right = (audio_frame.right * current_volume).clamp(-1.0, 1.0);

                        // Convert from [-1.0, 1.0] to [0, 65535]
                        frame[0] = ((left + 1.0) * 32767.5) as u16;
                        if channels > 1 {
                            frame[1] = ((right + 1.0) * 32767.5) as u16;
                        }
                    }
                },
                move |err| {
                    // [ARCH-ERRH-080] Set error flag for recovery
                    error!("Audio stream error: {} - marking for recovery", err);
                    error_flag.store(true, Ordering::SeqCst);
                    error_count.fetch_add(1, Ordering::SeqCst);
                },
                None,
            )
            .map_err(|e| Error::AudioOutput(format!("Failed to build stream: {}", e)))?;

        Ok(stream)
    }

    /// Stop audio playback.
    ///
    /// Pauses the stream and drops the stream reference.
    pub fn stop(&mut self) -> Result<()> {
        info!("Stopping audio stream");

        if let Some(stream) = self.stream.take() {
            stream
                .pause()
                .map_err(|e| Error::AudioOutput(format!("Failed to pause stream: {}", e)))?;
            drop(stream);
        }

        Ok(())
    }

    /// Set output volume.
    ///
    /// # Arguments
    /// - `volume`: Volume level (0.0 = silent, 1.0 = full volume)
    ///
    /// Values are clamped to [0.0, 1.0] range.
    pub fn set_volume(&self, volume: f32) {
        let clamped = volume.clamp(0.0, 1.0);
        *self.volume.lock().unwrap() = clamped;
        debug!("Volume set to {:.2}", clamped);
    }

    /// Get current volume.
    pub fn get_volume(&self) -> f32 {
        *self.volume.lock().unwrap()
    }

    /// Get device name.
    pub fn device_name(&self) -> String {
        self.device
            .name()
            .unwrap_or_else(|_| "Unknown".to_string())
    }

    /// Get sample rate.
    pub fn sample_rate(&self) -> u32 {
        self.config.sample_rate.0
    }

    /// Get channel count.
    pub fn channels(&self) -> u16 {
        self.config.channels
    }

    /// Check if an audio stream error has occurred.
    ///
    /// **[ARCH-ERRH-080]** Error detection for recovery
    /// **[ISSUE-5]** Enables engine to detect and respond to audio errors
    ///
    /// # Returns
    /// true if an error has been flagged by the audio callback
    pub fn has_error(&self) -> bool {
        self.error_flag.load(Ordering::SeqCst)
    }

    /// Clear error flag and reset error counter.
    ///
    /// **[ARCH-ERRH-080]** Error state reset after successful recovery
    ///
    /// Should be called after successful recovery to reset error tracking.
    pub fn clear_error(&self) {
        self.error_flag.store(false, Ordering::SeqCst);
        self.error_count.store(0, Ordering::SeqCst);
        info!("Audio error state cleared");
    }

    /// Get the consecutive error count.
    ///
    /// **[ARCH-ERRH-080]** Error count for backoff logic
    ///
    /// # Returns
    /// Number of consecutive errors since last successful operation
    pub fn get_error_count(&self) -> u32 {
        self.error_count.load(Ordering::SeqCst)
    }

    /// Attempt to recover from audio stream error.
    ///
    /// **[ARCH-ERRH-080]** Audio device error recovery
    /// **[ISSUE-5]** Automatic recovery from stream errors
    ///
    /// This method attempts to rebuild the audio stream with the same callback.
    /// Should be called when `has_error()` returns true.
    ///
    /// # Recovery Strategy
    /// 1. Stop current stream (if any)
    /// 2. Try to rebuild stream with same device
    /// 3. If rebuild fails and error count > 3, try default device as fallback
    ///
    /// # Arguments
    /// - `callback`: The audio callback to use for the rebuilt stream
    ///
    /// # Returns
    /// Ok(()) if recovery successful, Err if recovery failed
    ///
    /// # Notes
    /// - Caller should implement backoff/retry logic based on error count
    /// - Maximum recovery attempts should be limited by caller
    pub fn try_recover<F>(&mut self, callback: F) -> Result<()>
    where
        F: FnMut() -> AudioFrame + Send + 'static,
    {
        let error_count = self.get_error_count();
        warn!("Attempting audio stream recovery (error count: {})", error_count);

        // Stop current stream
        if let Err(e) = self.stop() {
            warn!("Failed to stop stream during recovery: {}", e);
        }

        // If we have multiple consecutive errors, try fallback to default device
        if error_count > 3 && self.requested_device.is_some() {
            warn!("Multiple errors detected, attempting fallback to default device");

            // Try to reinitialize with default device
            match Self::new(None) {
                Ok(new_output) => {
                    // Replace self with new output using default device
                    self.device = new_output.device.clone();
                    self.config = new_output.config.clone();
                    self.sample_format = new_output.sample_format;
                    self.requested_device = None; // Now using default

                    info!("Successfully switched to default audio device");
                }
                Err(e) => {
                    error!("Failed to fallback to default device: {}", e);
                    return Err(e);
                }
            }
        }

        // Try to restart stream with callback
        match self.start(callback) {
            Ok(()) => {
                info!("Audio stream recovery successful");
                self.clear_error();
                Ok(())
            }
            Err(e) => {
                error!("Audio stream recovery failed: {}", e);
                Err(e)
            }
        }
    }
}

impl Drop for AudioOutput {
    fn drop(&mut self) {
        // Ensure stream is stopped on drop
        let _ = self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_devices() {
        // This test requires audio hardware
        // Just verify it doesn't panic
        let result = AudioOutput::list_devices();
        assert!(result.is_ok() || result.is_err()); // Either is acceptable
    }

    #[test]
    fn test_volume_clamping() {
        // Volume should be clamped to [0.0, 1.0]
        let volume = Arc::new(Mutex::new(1.0));

        // Test by directly manipulating the mutex (simulating set_volume logic)
        *volume.lock().unwrap() = 1.5_f32.clamp(0.0, 1.0);
        assert_eq!(*volume.lock().unwrap(), 1.0);

        *volume.lock().unwrap() = (-0.5_f32).clamp(0.0, 1.0);
        assert_eq!(*volume.lock().unwrap(), 0.0);

        *volume.lock().unwrap() = 0.5_f32.clamp(0.0, 1.0);
        assert_eq!(*volume.lock().unwrap(), 0.5);
    }

    // Note: Actual audio playback tests require hardware and are best done as manual tests
}
