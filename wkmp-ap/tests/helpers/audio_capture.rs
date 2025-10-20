//! Audio output capture for integration tests
//!
//! Records audio samples that would go to speakers for analysis

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Audio capture buffer
pub struct AudioCapture {
    samples: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
    start_time: Instant,
}

impl AudioCapture {
    /// Create new audio capture
    pub fn new(sample_rate: u32) -> Self {
        Self {
            samples: Arc::new(Mutex::new(Vec::new())),
            sample_rate,
            start_time: Instant::now(),
        }
    }

    /// Record audio samples (called from audio thread)
    pub fn record(&self, samples: &[f32]) {
        self.samples.lock().unwrap().extend_from_slice(samples);
    }

    /// Get all recorded samples
    pub fn get_samples(&self) -> Vec<f32> {
        self.samples.lock().unwrap().clone()
    }

    /// Get sample count
    pub fn sample_count(&self) -> usize {
        self.samples.lock().unwrap().len()
    }

    /// Get duration of recorded audio
    pub fn get_duration(&self) -> f64 {
        let count = self.sample_count();
        // Stereo interleaved, so divide by 2
        let frames = count / 2;
        frames as f64 / self.sample_rate as f64
    }

    /// Wait for first audio sample above threshold
    pub async fn wait_for_audio(
        &self,
        timeout: Duration,
        threshold: f32,
    ) -> Option<Instant> {
        let deadline = Instant::now() + timeout;

        loop {
            if Instant::now() > deadline {
                return None;
            }

            let samples = self.samples.lock().unwrap();
            if let Some(_) = samples.iter().find(|&&s| s.abs() > threshold) {
                return Some(Instant::now());
            }
            drop(samples); // Release lock

            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    /// Convert timestamp to sample index
    pub fn timestamp_to_sample_index(&self, timestamp: Instant) -> usize {
        let elapsed = timestamp.duration_since(self.start_time);
        let elapsed_seconds = elapsed.as_secs_f64();
        let frame_index = (elapsed_seconds * self.sample_rate as f64) as usize;
        frame_index * 2 // Stereo interleaved
    }

    /// Get sample rate
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Clear recorded samples
    pub fn clear(&self) {
        self.samples.lock().unwrap().clear();
    }

    /// Get samples in a specific time range
    pub fn get_samples_range(&self, start_ms: u64, end_ms: u64) -> Vec<f32> {
        let samples = self.samples.lock().unwrap();

        let start_frame = (start_ms as f64 / 1000.0 * self.sample_rate as f64) as usize;
        let end_frame = (end_ms as f64 / 1000.0 * self.sample_rate as f64) as usize;

        let start_idx = start_frame * 2; // Stereo
        let end_idx = end_frame * 2;

        if start_idx >= samples.len() {
            return Vec::new();
        }

        let end_idx = end_idx.min(samples.len());
        samples[start_idx..end_idx].to_vec()
    }
}

/// Mock audio capture that can be used with TestServer
///
/// In real implementation, this would hook into the audio output pipeline.
/// For now, it provides the interface that tests expect.
pub struct MockAudioCapture {
    capture: AudioCapture,
}

impl MockAudioCapture {
    pub fn new() -> Self {
        Self {
            capture: AudioCapture::new(44100),
        }
    }

    /// Start capturing audio (mock - would hook into real audio in production)
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // In real implementation, this would:
        // 1. Hook into AudioOutput or CrossfadeMixer
        // 2. Intercept audio samples before they go to cpal
        // 3. Record them to self.capture
        //
        // For now, tests can manually inject samples via inject_samples()
        Ok(())
    }

    /// Inject samples for testing (mock method)
    pub fn inject_samples(&mut self, samples: &[f32]) {
        self.capture.record(samples);
    }

    /// Get captured samples
    pub fn get_samples(&self) -> Vec<f32> {
        self.capture.get_samples()
    }

    /// Wait for audio
    pub async fn wait_for_audio(
        &self,
        timeout: Duration,
        threshold: f32,
    ) -> Option<Instant> {
        self.capture.wait_for_audio(timeout, threshold).await
    }

    /// Convert timestamp to sample index
    pub fn timestamp_to_sample_index(&self, timestamp: Instant) -> usize {
        self.capture.timestamp_to_sample_index(timestamp)
    }

    /// Get duration
    pub fn get_duration(&self) -> f64 {
        self.capture.get_duration()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_capture_basic() {
        let capture = AudioCapture::new(44100);

        // Record some samples
        let samples = vec![0.5, 0.6, 0.7, 0.8];
        capture.record(&samples);

        assert_eq!(capture.sample_count(), 4);
        assert_eq!(capture.get_samples(), samples);
    }

    #[tokio::test]
    async fn test_wait_for_audio() {
        let capture = AudioCapture::new(44100);

        // Spawn task to add audio after delay
        let capture_clone = Arc::new(capture);
        let capture_task = capture_clone.clone();

        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            capture_task.record(&[0.0, 0.0, 0.5, 0.6]); // Audio starts at index 2
        });

        let result = capture_clone
            .wait_for_audio(Duration::from_secs(1), 0.1)
            .await;

        assert!(result.is_some());
    }

    #[test]
    fn test_duration_calculation() {
        let capture = AudioCapture::new(44100);

        // 1 second of stereo audio = 88,200 samples
        let samples = vec![0.0; 88_200];
        capture.record(&samples);

        let duration = capture.get_duration();
        assert!((duration - 1.0).abs() < 0.01);
    }
}
