//! Test harness for parameter testing
//!
//! **Purpose:** Simplified playback for testing parameter combinations.
//!
//! **Traceability:** TUNE-INT-020, TUNE-SRC-020, TUNE-TEST-040

use crate::audio::output::AudioOutput;
use crate::audio::types::AudioFrame;
use crate::audio::resampler::StatefulResampler;
use crate::error::Result;
use crate::playback::callback_monitor::CallbackMonitor;
use crate::playback::ring_buffer::AudioRingBuffer;
use crate::tuning::metrics::{TestResult, UnderrunMetrics, JitterMetrics, BufferOccupancyMetrics, CpuMetrics};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::info;

/// Test configuration for parameter combination
#[derive(Debug, Clone)]
pub struct TestConfig {
    /// Mixer check interval in milliseconds (TUNE-OBJ-010)
    pub mixer_check_interval_ms: u64,

    /// Audio buffer size in frames (TUNE-OBJ-010)
    pub audio_buffer_size: u32,

    /// Test duration in seconds (TUNE-SRC-020: 30-60s)
    pub test_duration_secs: u64,

    /// Audio device name (None = default)
    pub device_name: Option<String>,
}

impl TestConfig {
    /// Create a new test configuration
    pub fn new(mixer_interval_ms: u64, buffer_size: u32) -> Self {
        Self {
            mixer_check_interval_ms: mixer_interval_ms,
            audio_buffer_size: buffer_size,
            test_duration_secs: 30, // TUNE-SRC-020: minimum 30 seconds
            device_name: None,
        }
    }

    /// Set test duration in seconds
    pub fn with_duration(mut self, duration_secs: u64) -> Self {
        self.test_duration_secs = duration_secs;
        self
    }

    /// Set device name
    pub fn with_device(mut self, device: Option<String>) -> Self {
        self.device_name = device;
        self
    }
}

/// Test audio generator
///
/// **TUNE-TEST-020:** Generate 440 Hz sine wave at 48kHz, resample to 44.1kHz
/// **TUNE-TEST-040:** f32 stereo, -6dB amplitude
struct TestAudioGenerator {
    /// Sample rate (48000 Hz for generation, resampled to 44100 Hz)
    sample_rate: u32,

    /// Current phase for sine wave generation
    phase: f64,

    /// Phase increment per sample (frequency / sample_rate * 2π)
    phase_increment: f64,

    /// Amplitude (-6dB = 0.5)
    amplitude: f32,

    /// Resampler (48kHz → 44.1kHz)
    resampler: Option<StatefulResampler>,
}

impl TestAudioGenerator {
    /// Create new test audio generator
    ///
    /// Generates 440 Hz sine wave at 48kHz, resampled to 44.1kHz
    fn new() -> Result<Self> {
        let sample_rate = 48000;
        let frequency = 440.0; // A4
        let phase_increment = frequency / sample_rate as f64 * 2.0 * std::f64::consts::PI;
        let amplitude = 0.5; // -6dB

        // Create resampler: 48kHz → 44.1kHz
        let resampler = StatefulResampler::new(
            48000,  // input sample rate
            44100,  // output sample rate
            2,      // channels (stereo)
            1024,   // chunk size
        )?;

        Ok(Self {
            sample_rate,
            phase: 0.0,
            phase_increment,
            amplitude,
            resampler: Some(resampler),
        })
    }

    /// Generate next chunk of audio frames
    ///
    /// Returns frames resampled to 44.1kHz, ready for playback
    fn generate_chunk(&mut self, frame_count: usize) -> Result<Vec<AudioFrame>> {
        // Generate at 48kHz (interleaved stereo samples)
        let mut input_samples = Vec::with_capacity(frame_count * 2); // 2 channels

        for _ in 0..frame_count {
            let sample = (self.phase.sin() * self.amplitude as f64) as f32;
            input_samples.push(sample); // Left
            input_samples.push(sample); // Right

            self.phase += self.phase_increment;
            if self.phase >= 2.0 * std::f64::consts::PI {
                self.phase -= 2.0 * std::f64::consts::PI;
            }
        }

        // Resample to 44.1kHz
        let output_samples = if let Some(resampler) = &mut self.resampler {
            resampler.process_chunk(&input_samples)?
        } else {
            input_samples // Fallback: no resampling
        };

        // Convert interleaved samples to AudioFrame vec
        let mut frames = Vec::with_capacity(output_samples.len() / 2);
        for chunk in output_samples.chunks_exact(2) {
            frames.push(AudioFrame {
                left: chunk[0],
                right: chunk[1],
            });
        }

        Ok(frames)
    }
}

/// Test harness for parameter combination testing
///
/// **Purpose:** Run audio playback with specific parameters and collect metrics
pub struct TestHarness {
    config: TestConfig,
    runtime: tokio::runtime::Handle,
}

impl TestHarness {
    /// Create new test harness
    pub fn new(config: TestConfig, runtime: tokio::runtime::Handle) -> Self {
        Self { config, runtime }
    }

    /// Run test and collect metrics
    ///
    /// **Algorithm (TUNE-TEST-010):**
    /// 1. Apply parameter values
    /// 2. Initialize audio output with new parameters
    /// 3. Start playback of test audio (30s duration)
    /// 4. Monitor underrun events continuously
    /// 5. Record results (underrun count, callback stats)
    /// 6. Shut down audio output cleanly
    /// 7. Wait brief cooldown (2s) before next test
    ///
    /// **Returns:** Test result with underrun metrics
    pub fn run_test(&self) -> Result<TestResult> {
        info!(
            "Starting test: mixer_interval={}ms, buffer_size={}, duration={}s",
            self.config.mixer_check_interval_ms,
            self.config.audio_buffer_size,
            self.config.test_duration_secs
        );

        let start_time = Instant::now();

        // Generate test audio
        let mut generator = TestAudioGenerator::new()?;

        // Create ring buffer for audio transport
        let audio_expected = Arc::new(AtomicBool::new(true));
        let ring_buffer = AudioRingBuffer::new(
            Some(8192), // Ring buffer capacity (independent of output buffer size)
            2000,       // 2-second grace period for audio system initialization
            audio_expected.clone(),
        );
        let (mut producer, mut consumer) = ring_buffer.split();

        // Create callback monitor
        let monitor = Arc::new(CallbackMonitor::new(
            44100, // sample rate
            self.config.audio_buffer_size,
            None,  // No event emission needed for tuning
            audio_expected.clone(), // Pass audio_expected flag for idle detection
        ));

        // Start monitoring task
        let monitor_clone = Arc::clone(&monitor);
        let shutdown_flag = monitor_clone.spawn_monitoring_task(self.runtime.clone());

        // Create audio output with specified buffer size
        let mut audio_output = AudioOutput::new_with_volume(
            self.config.device_name.clone(),
            None, // No volume control needed
            Some(self.config.audio_buffer_size),
        )?;

        // Create audio callback that reads from ring buffer
        // Track underrun state to avoid counting every missing frame as a separate underrun
        // We want to count "underrun events" (buffer went empty) not "missing frames"
        let monitor_for_callback = Arc::clone(&monitor);
        let underrun_active = Arc::new(AtomicBool::new(false));
        let audio_callback = move || {
            // Lock-free read from ring buffer
            match consumer.pop() {
                Some(frame) => {
                    // Got data - reset underrun state
                    underrun_active.store(false, Ordering::Relaxed);
                    frame
                }
                None => {
                    // Buffer empty - record underrun only once per underrun event
                    // (not once per missing frame)
                    if !underrun_active.swap(true, Ordering::Relaxed) {
                        // First frame of underrun - record it
                        monitor_for_callback.record_underrun();
                    }
                    AudioFrame::zero()
                }
            }
        };

        // Start audio stream
        audio_output.start(audio_callback, Some(Arc::clone(&monitor)))?;

        // Run test for specified duration
        let test_duration = Duration::from_secs(self.config.test_duration_secs);
        let mixer_interval = Duration::from_millis(self.config.mixer_check_interval_ms);
        let end_time = start_time + test_duration;

        // Calculate how much audio we need to generate per mixer cycle
        // mixer_interval in ms * 44.1 samples/ms = samples needed
        // Add 20% safety margin to account for timing variations
        let frames_per_cycle = ((self.config.mixer_check_interval_ms as f64 * 44.1) * 1.2) as usize;

        // Round up to nearest 1024 boundary for efficient resampling
        let chunk_size = ((frames_per_cycle + 1023) / 1024) * 1024;

        info!(
            "Mixer simulation: generating {} frames every {}ms ({}ms of audio)",
            chunk_size,
            self.config.mixer_check_interval_ms,
            (chunk_size as f64 / 44.1) as u64
        );

        // Pre-fill the ring buffer to avoid initial underruns
        info!("Pre-filling ring buffer...");
        for _ in 0..3 {
            let frames = generator.generate_chunk(chunk_size)?;
            for frame in frames {
                if !producer.push(frame) {
                    break; // Buffer is full enough
                }
            }
        }

        info!("Starting test measurement...");

        // Simulate mixer behavior: generate chunk_size worth of audio, then sleep for mixer_interval
        // This tests if the buffer configuration can handle the mixer only waking up every N milliseconds
        while Instant::now() < end_time {
            // Generate enough audio for the next mixer interval period
            let frames = generator.generate_chunk(chunk_size)?;

            // Push frames to ring buffer
            for frame in frames {
                while !producer.push(frame) {
                    // Ring buffer full, wait briefly
                    // This is normal - it means we're keeping ahead of playback
                    std::thread::sleep(Duration::from_micros(100));
                }
            }

            // Sleep for mixer interval to simulate mixer thread checking periodically
            std::thread::sleep(mixer_interval);
        }

        // Stop audio output
        let _ = audio_output.stop();

        // Stop monitoring
        shutdown_flag.store(true, Ordering::Relaxed);

        // Collect final statistics
        let stats = monitor.stats();
        let elapsed = start_time.elapsed();

        // Build test result
        let underruns = UnderrunMetrics::new(
            stats.underrun_count,
            stats.callback_count,
        );

        // For now, use placeholder metrics for jitter, occupancy, and CPU
        // These could be enhanced later by collecting samples during the test
        let jitter = JitterMetrics::from_intervals(&[stats.expected_interval_ms as f64]);
        let occupancy = BufferOccupancyMetrics::from_samples(&[]);
        let cpu = CpuMetrics::unavailable();

        let result = TestResult::new(
            self.config.mixer_check_interval_ms,
            self.config.audio_buffer_size,
            elapsed.as_secs(),
            underruns,
            jitter,
            occupancy,
            cpu,
        );

        info!(
            "Test complete: verdict={:?}, underrun_rate={:.2}%, callbacks={}",
            result.verdict,
            underruns.underrun_rate,
            stats.callback_count
        );

        // Cooldown period (TUNE-TEST-010: 2s cooldown)
        std::thread::sleep(Duration::from_secs(2));

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_generator_creates() {
        let generator = TestAudioGenerator::new();
        assert!(generator.is_ok());
    }

    #[test]
    fn test_audio_generator_produces_frames() {
        let mut generator = TestAudioGenerator::new().unwrap();
        // Must use chunk size matching resampler's expected size (1024)
        let frames = generator.generate_chunk(1024);
        if let Err(e) = &frames {
            eprintln!("Error generating frames: {:?}", e);
        }
        assert!(frames.is_ok(), "Failed to generate frames: {:?}", frames.err());

        let frames = frames.unwrap();
        assert!(!frames.is_empty());

        // Verify amplitude is within expected range (-6dB = 0.5 max)
        for frame in &frames {
            assert!(frame.left.abs() <= 0.6); // Allow some headroom for resampling
            assert!(frame.right.abs() <= 0.6);
        }
    }

    #[test]
    fn test_config_builder() {
        let config = TestConfig::new(5, 512)
            .with_duration(60)
            .with_device(Some("test_device".to_string()));

        assert_eq!(config.mixer_check_interval_ms, 5);
        assert_eq!(config.audio_buffer_size, 512);
        assert_eq!(config.test_duration_secs, 60);
        assert_eq!(config.device_name, Some("test_device".to_string()));
    }
}
