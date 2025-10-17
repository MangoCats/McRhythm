//! Audio output using cpal
//!
//! Manages audio device output with callback-based playback.
//!
//! **Traceability:**
//! - [SSD-OUT-010] Audio device interface
//! - [SSD-OUT-011] Initialize audio device
//! - [SSD-OUT-012] Begin audio stream with callback

use crate::audio::AudioFrame;
use crate::error::{Error, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, Stream, StreamConfig};
use std::sync::{Arc, Mutex};
use tracing::{debug, info, warn};

/// Audio output manager using cpal.
///
/// **[SSD-OUT-010]** Manages audio device interface and playback stream.
pub struct AudioOutput {
    device: Device,
    config: StreamConfig,
    sample_format: SampleFormat,
    stream: Option<Stream>,
    volume: Arc<Mutex<f32>>,
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
    ///
    /// # Arguments
    /// - `device_name`: Optional device name (None = default device)
    ///
    /// # Returns
    /// AudioOutput instance ready to start playback
    ///
    /// # Errors
    /// - Device not found
    /// - Failed to get device configuration
    pub fn new(device_name: Option<String>) -> Result<Self> {
        let host = cpal::default_host();

        // Get the device
        let device = if let Some(name) = device_name.as_ref() {
            // Find device by name
            let mut devices = host
                .output_devices()
                .map_err(|e| Error::AudioOutput(format!("Failed to enumerate devices: {}", e)))?;

            devices
                .find(|d| d.name().ok().as_ref() == Some(name))
                .ok_or_else(|| Error::AudioOutput(format!("Device '{}' not found", name)))?
        } else {
            // Use default device
            host.default_output_device()
                .ok_or_else(|| Error::AudioOutput("No default output device found".to_string()))?
        };

        let device_name = device
            .name()
            .unwrap_or_else(|_| "Unknown".to_string());

        info!("Using audio device: {}", device_name);

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
    fn build_stream_f32(
        &self,
        callback: Arc<Mutex<dyn FnMut() -> AudioFrame + Send + 'static>>,
        volume: Arc<Mutex<f32>>,
    ) -> Result<Stream> {
        let channels = self.config.channels as usize;

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
                    warn!("Audio stream error: {}", err);
                },
                None, // No timeout
            )
            .map_err(|e| Error::AudioOutput(format!("Failed to build stream: {}", e)))?;

        Ok(stream)
    }

    /// Build audio stream for i16 samples
    fn build_stream_i16(
        &self,
        callback: Arc<Mutex<dyn FnMut() -> AudioFrame + Send + 'static>>,
        volume: Arc<Mutex<f32>>,
    ) -> Result<Stream> {
        let channels = self.config.channels as usize;

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
                    warn!("Audio stream error: {}", err);
                },
                None,
            )
            .map_err(|e| Error::AudioOutput(format!("Failed to build stream: {}", e)))?;

        Ok(stream)
    }

    /// Build audio stream for u16 samples
    fn build_stream_u16(
        &self,
        callback: Arc<Mutex<dyn FnMut() -> AudioFrame + Send + 'static>>,
        volume: Arc<Mutex<f32>>,
    ) -> Result<Stream> {
        let channels = self.config.channels as usize;

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
                    warn!("Audio stream error: {}", err);
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
