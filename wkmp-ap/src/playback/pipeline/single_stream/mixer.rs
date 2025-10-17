//! Sample-accurate crossfade mixing for single-stream audio playback
//!
//! This module implements the CrossfadeMixer component for mixing two audio streams
//! with various fade curves and sample-accurate timing.
//!
//! Implements requirements from:
//! - single-stream-design.md - Phase 2: CrossfadeMixer
//! - crossfade.md - XFD-CURV-* fade curve specifications

use std::sync::Arc;
use std::collections::VecDeque;
use tokio::sync::RwLock;
use uuid::Uuid;
use anyhow::{Result, anyhow};
use tracing::{info, warn};

use super::buffer::{PassageBufferManager, FadeCurve, BufferStatus};

/// Standard sample rate for all audio mixing
const STANDARD_SAMPLE_RATE: u32 = 44100;

/// Number of audio channels (stereo)
const CHANNELS: u16 = 2;

/// Minimum crossfade duration in milliseconds
/// Implements XFD-PREC-040: Precision of 0.02ms, minimum 20ms duration
const MIN_CROSSFADE_MS: f64 = 20.0;

/// Maximum crossfade duration in milliseconds
const MAX_CROSSFADE_MS: f64 = 10000.0;

/// Crossfade state for tracking mix progress
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CrossfadeState {
    /// No crossfade active, playing single passage
    SinglePassage,
    /// Crossfade in progress
    Crossfading {
        /// Progress through the crossfade (0.0 to 1.0)
        progress: f32,
        /// Total samples in this crossfade
        total_samples: u64,
        /// Samples processed so far
        current_sample: u64,
    },
    /// Crossfade completed, waiting for old passage cleanup
    Completed,
}

/// 6-point crossfade timing model
/// Implements XFD-MOD-060 from crossfade.md
#[derive(Debug, Clone)]
pub struct CrossfadePoints {
    /// Start of passage A (sample position)
    pub start_a: u64,
    /// Start of fade-in for passage B
    pub fade_in_start: u64,
    /// End of fade-in for passage B / Start of overlap
    pub lead_in_end: u64,
    /// Start of fade-out for passage A
    pub lead_out_start: u64,
    /// End of fade-out for passage A
    pub fade_out_end: u64,
    /// End of passage B
    pub end_b: u64,
}

impl CrossfadePoints {
    /// Calculate crossfade points from timing parameters
    ///
    /// Implements XFD-MOD-060: 6-point timing model
    pub fn calculate(
        fade_in_duration_ms: f64,
        fade_out_duration_ms: f64,
        overlap_ms: f64,
    ) -> Result<Self> {
        // Validate timing parameters
        if fade_in_duration_ms < MIN_CROSSFADE_MS || fade_in_duration_ms > MAX_CROSSFADE_MS {
            return Err(anyhow!(
                "Fade-in duration {}ms out of range [{}, {}]",
                fade_in_duration_ms,
                MIN_CROSSFADE_MS,
                MAX_CROSSFADE_MS
            ));
        }

        if fade_out_duration_ms < MIN_CROSSFADE_MS || fade_out_duration_ms > MAX_CROSSFADE_MS {
            return Err(anyhow!(
                "Fade-out duration {}ms out of range [{}, {}]",
                fade_out_duration_ms,
                MIN_CROSSFADE_MS,
                MAX_CROSSFADE_MS
            ));
        }

        // Convert to samples
        let fade_in_samples = (fade_in_duration_ms * STANDARD_SAMPLE_RATE as f64 / 1000.0) as u64;
        let fade_out_samples = (fade_out_duration_ms * STANDARD_SAMPLE_RATE as f64 / 1000.0) as u64;
        let overlap_samples = (overlap_ms * STANDARD_SAMPLE_RATE as f64 / 1000.0) as u64;

        // Calculate timing points
        Ok(CrossfadePoints {
            start_a: 0,
            fade_in_start: 0, // B starts immediately for now
            lead_in_end: fade_in_samples,
            lead_out_start: fade_in_samples.saturating_sub(overlap_samples),
            fade_out_end: fade_in_samples + fade_out_samples - overlap_samples,
            end_b: u64::MAX, // Continues indefinitely
        })
    }
}

/// Sample-accurate crossfade mixer for blending audio passages
///
/// Implements requirements from single-stream-design.md - Phase 2
pub struct CrossfadeMixer {
    /// Reference to the buffer manager for accessing PCM data
    buffer_manager: Arc<PassageBufferManager>,

    /// Currently playing passage
    current_passage: Arc<RwLock<Option<Uuid>>>,

    /// Next passage for crossfading
    next_passage: Arc<RwLock<Option<Uuid>>>,

    /// Current crossfade state
    state: Arc<RwLock<CrossfadeState>>,

    /// Crossfade timing points
    crossfade_points: Arc<RwLock<Option<CrossfadePoints>>>,

    /// Output buffer for mixed audio
    output_buffer: Arc<RwLock<VecDeque<f32>>>,

    /// Current playback position in samples
    playback_position: Arc<RwLock<u64>>,
}

impl CrossfadeMixer {
    /// Create a new crossfade mixer
    pub fn new(buffer_manager: Arc<PassageBufferManager>) -> Self {
        info!("Initializing CrossfadeMixer");

        Self {
            buffer_manager,
            current_passage: Arc::new(RwLock::new(None)),
            next_passage: Arc::new(RwLock::new(None)),
            state: Arc::new(RwLock::new(CrossfadeState::SinglePassage)),
            crossfade_points: Arc::new(RwLock::new(None)),
            output_buffer: Arc::new(RwLock::new(VecDeque::with_capacity(
                STANDARD_SAMPLE_RATE as usize * CHANNELS as usize * 2 // 2 seconds buffer
            ))),
            playback_position: Arc::new(RwLock::new(0)),
        }
    }

    /// Start playback of a passage
    pub async fn start_passage(&self, passage_id: Uuid) -> Result<()> {
        info!(passage_id = %passage_id, "Starting passage playback");

        // Verify buffer is ready
        let status = self.buffer_manager
            .get_status(&passage_id)
            .await
            .ok_or_else(|| anyhow!("Passage {} not found in buffer", passage_id))?;

        if status != BufferStatus::Ready {
            return Err(anyhow!("Passage {} not ready for playback (status: {:?})", passage_id, status));
        }

        // Set as current passage
        *self.current_passage.write().await = Some(passage_id);
        *self.state.write().await = CrossfadeState::SinglePassage;

        // Mark buffer as playing
        self.buffer_manager.mark_playing(&passage_id).await?;

        Ok(())
    }

    /// Queue the next passage for crossfading
    pub async fn queue_next_passage(
        &self,
        passage_id: Uuid,
        fade_in_ms: f64,
        fade_out_ms: f64,
        overlap_ms: f64,
    ) -> Result<()> {
        info!(
            passage_id = %passage_id,
            fade_in_ms,
            fade_out_ms,
            overlap_ms,
            "Queueing next passage for crossfade"
        );

        // Verify buffer is ready
        let status = self.buffer_manager
            .get_status(&passage_id)
            .await
            .ok_or_else(|| anyhow!("Passage {} not found in buffer", passage_id))?;

        if status != BufferStatus::Ready {
            return Err(anyhow!("Passage {} not ready for playback (status: {:?})", passage_id, status));
        }

        // Calculate crossfade points
        let points = CrossfadePoints::calculate(fade_in_ms, fade_out_ms, overlap_ms)?;

        // Set up for crossfade
        *self.next_passage.write().await = Some(passage_id);
        *self.crossfade_points.write().await = Some(points);

        Ok(())
    }

    /// Process audio samples and fill the output buffer
    ///
    /// This is the core mixing function that:
    /// 1. Reads PCM data from passage buffers
    /// 2. Applies fade curves during crossfades
    /// 3. Mixes audio streams sample-accurately
    pub async fn process_audio(&self, num_samples: usize) -> Result<Vec<f32>> {
        let mut output = Vec::with_capacity(num_samples);

        // Get current state
        let state = *self.state.read().await;
        let current_id = *self.current_passage.read().await;
        let next_id = *self.next_passage.read().await;

        match state {
            CrossfadeState::SinglePassage => {
                // Simple playback of current passage
                if let Some(passage_id) = current_id {
                    self.process_single_passage(&passage_id, &mut output, num_samples).await?;
                }
            }
            CrossfadeState::Crossfading { progress, total_samples, current_sample } => {
                // Mix two passages with crossfading
                if let (Some(current_id), Some(next_id)) = (current_id, next_id) {
                    self.process_crossfade(
                        &current_id,
                        &next_id,
                        &mut output,
                        num_samples,
                        progress,
                        total_samples,
                        current_sample,
                    ).await?;
                }
            }
            CrossfadeState::Completed => {
                // Transition to single passage playback of the next passage
                if let Some(next_id) = next_id {
                    self.process_single_passage(&next_id, &mut output, num_samples).await?;

                    // Clean up and transition state
                    self.complete_crossfade().await?;
                }
            }
        }

        // Update playback position
        let mut position = self.playback_position.write().await;
        *position += (num_samples / CHANNELS as usize) as u64;

        Ok(output)
    }

    /// Process audio from a single passage
    async fn process_single_passage(
        &self,
        passage_id: &Uuid,
        output: &mut Vec<f32>,
        num_samples: usize,
    ) -> Result<()> {
        // Get buffer access
        if let Some(buffers) = self.buffer_manager.get_buffer(passage_id).await {
            if let Some(buffer) = buffers.get(passage_id) {
                // Read PCM data
                let position = *self.playback_position.read().await;
                let start_idx = (position * CHANNELS as u64) as usize;
                let end_idx = (start_idx + num_samples).min(buffer.pcm_data.len());

                if start_idx < buffer.pcm_data.len() {
                    output.extend_from_slice(&buffer.pcm_data[start_idx..end_idx]);

                    // Check if we've reached the end of the buffer
                    if end_idx >= buffer.pcm_data.len() {
                        // Buffer exhausted - check if there's a next passage queued
                        let next_id = *self.next_passage.read().await;
                        if next_id.is_some() {
                            warn!(
                                passage_id = %passage_id,
                                "Current passage buffer exhausted, transitioning to next passage"
                            );
                            // Transition to next passage
                            self.complete_crossfade().await?;
                            // The next process_audio call will read from the new current passage
                        }
                    }

                    // Pad with silence if needed
                    if output.len() < num_samples {
                        output.resize(num_samples, 0.0);
                    }
                }
            }
        }

        Ok(())
    }

    /// Process crossfading between two passages
    async fn process_crossfade(
        &self,
        current_id: &Uuid,
        next_id: &Uuid,
        output: &mut Vec<f32>,
        num_samples: usize,
        progress: f32,
        total_samples: u64,
        current_sample: u64,
    ) -> Result<()> {
        // Get both buffers
        let current_buffer = self.buffer_manager.get_buffer(current_id).await;
        let next_buffer = self.buffer_manager.get_buffer(next_id).await;

        if let (Some(current_buffers), Some(next_buffers)) = (current_buffer, next_buffer) {
            if let (Some(current), Some(next)) =
                (current_buffers.get(current_id), next_buffers.get(next_id)) {

                let position = *self.playback_position.read().await;
                let samples_per_frame = CHANNELS as usize;
                let num_frames = num_samples / samples_per_frame;

                for frame in 0..num_frames {
                    let sample_idx = current_sample + frame as u64;
                    let frame_progress = sample_idx as f32 / total_samples as f32;

                    // Calculate fade gains
                    let current_gain = calculate_fade_gain(
                        current.fade_out_curve,
                        1.0 - frame_progress,
                        false // fade out
                    );

                    let next_gain = calculate_fade_gain(
                        next.fade_in_curve,
                        frame_progress,
                        true // fade in
                    );

                    // Mix samples for each channel
                    let current_idx = ((position + frame as u64) * CHANNELS as u64) as usize;
                    let next_idx = (frame as u64 * CHANNELS as u64) as usize;

                    for ch in 0..samples_per_frame {
                        let current_sample = if current_idx + ch < current.pcm_data.len() {
                            current.pcm_data[current_idx + ch]
                        } else {
                            0.0
                        };

                        let next_sample = if next_idx + ch < next.pcm_data.len() {
                            next.pcm_data[next_idx + ch]
                        } else {
                            0.0
                        };

                        // Mix with calculated gains
                        let mixed = (current_sample * current_gain) + (next_sample * next_gain);
                        output.push(mixed.clamp(-1.0, 1.0)); // Prevent clipping
                    }
                }

                // Update crossfade state
                let new_sample = current_sample + num_frames as u64;
                if new_sample >= total_samples {
                    *self.state.write().await = CrossfadeState::Completed;
                } else {
                    *self.state.write().await = CrossfadeState::Crossfading {
                        progress: new_sample as f32 / total_samples as f32,
                        total_samples,
                        current_sample: new_sample,
                    };
                }
            }
        }

        Ok(())
    }

    /// Complete the crossfade and transition to single passage playback
    async fn complete_crossfade(&self) -> Result<()> {
        let current_id = *self.current_passage.read().await;
        let next_id = *self.next_passage.read().await;

        if let Some(old_id) = current_id {
            // Mark old buffer as exhausted
            self.buffer_manager.mark_exhausted(&old_id).await?;
        }

        if let Some(new_id) = next_id {
            // Transition next to current
            *self.current_passage.write().await = Some(new_id);
            *self.next_passage.write().await = None;
            *self.state.write().await = CrossfadeState::SinglePassage;
            *self.crossfade_points.write().await = None;

            info!(passage_id = %new_id, "Crossfade completed, now playing single passage");
        }

        Ok(())
    }

    /// Initiate a crossfade between current and next passages
    pub async fn start_crossfade(&self) -> Result<()> {
        let next_id = self.next_passage.read().await;
        if next_id.is_none() {
            return Err(anyhow!("No next passage queued for crossfade"));
        }

        let points = self.crossfade_points.read().await;
        if let Some(points) = points.as_ref() {
            let total_samples = points.fade_out_end - points.fade_in_start;

            *self.state.write().await = CrossfadeState::Crossfading {
                progress: 0.0,
                total_samples,
                current_sample: 0,
            };

            info!(
                total_samples,
                duration_ms = total_samples as f64 * 1000.0 / STANDARD_SAMPLE_RATE as f64,
                "Starting crossfade"
            );
        } else {
            return Err(anyhow!("No crossfade points configured"));
        }

        Ok(())
    }

    /// Get the current playback position in milliseconds
    pub async fn get_position_ms(&self) -> f64 {
        let position = *self.playback_position.read().await;
        position as f64 * 1000.0 / STANDARD_SAMPLE_RATE as f64
    }

    /// Get the current crossfade state
    pub async fn get_state(&self) -> CrossfadeState {
        *self.state.read().await
    }
}

/// Calculate fade gain for a given progress and curve type
///
/// Implements fade curves from crossfade.md:
/// - XFD-CURV-010: Linear fade
/// - XFD-CURV-020: Logarithmic fade (for fade-out)
/// - XFD-CURV-030: Exponential fade (for fade-in)
/// - XFD-CURV-040: S-Curve (Cosine interpolation)
pub fn calculate_fade_gain(curve: FadeCurve, progress: f32, is_fade_in: bool) -> f32 {
    let clamped_progress = progress.clamp(0.0, 1.0);

    match curve {
        FadeCurve::Linear => {
            // Linear: constant rate of change
            if is_fade_in {
                clamped_progress
            } else {
                1.0 - clamped_progress
            }
        }
        FadeCurve::Logarithmic => {
            // Logarithmic: fast start, slow finish (natural for fade-out)
            if is_fade_in {
                // For fade-in, use inverted logarithmic
                1.0 - (-10.0 * clamped_progress).exp()
            } else {
                // For fade-out, standard logarithmic
                (-10.0 * (1.0 - clamped_progress)).exp()
            }
        }
        FadeCurve::Exponential => {
            // Exponential: slow start, fast finish (natural for fade-in)
            if is_fade_in {
                // For fade-in, standard exponential
                clamped_progress * clamped_progress
            } else {
                // For fade-out, inverted exponential
                let inv = 1.0 - clamped_progress;
                1.0 - (inv * inv)
            }
        }
        FadeCurve::SCurve => {
            // S-Curve: smooth acceleration and deceleration using cosine
            let cos_val = (clamped_progress * std::f32::consts::PI).cos();
            let s_curve = (1.0 - cos_val) / 2.0;

            if is_fade_in {
                s_curve
            } else {
                1.0 - s_curve
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fade_curves() {
        // Test linear fade
        assert!((calculate_fade_gain(FadeCurve::Linear, 0.5, true) - 0.5).abs() < 0.001);
        assert!((calculate_fade_gain(FadeCurve::Linear, 0.5, false) - 0.5).abs() < 0.001);

        // Test boundary conditions
        assert_eq!(calculate_fade_gain(FadeCurve::Linear, 0.0, true), 0.0);
        assert_eq!(calculate_fade_gain(FadeCurve::Linear, 1.0, true), 1.0);
        assert_eq!(calculate_fade_gain(FadeCurve::Linear, 0.0, false), 1.0);
        assert_eq!(calculate_fade_gain(FadeCurve::Linear, 1.0, false), 0.0);

        // Test S-Curve midpoint
        let s_mid = calculate_fade_gain(FadeCurve::SCurve, 0.5, true);
        assert!((s_mid - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_crossfade_points_calculation() {
        let points = CrossfadePoints::calculate(1000.0, 1000.0, 500.0).unwrap();

        // At 44.1kHz, 1000ms = 44100 samples
        assert_eq!(points.lead_in_end, 44100);
        assert_eq!(points.fade_out_end - points.lead_out_start, 44100);
    }

    #[test]
    fn test_crossfade_validation() {
        // Too short
        assert!(CrossfadePoints::calculate(10.0, 1000.0, 500.0).is_err());

        // Too long
        assert!(CrossfadePoints::calculate(15000.0, 1000.0, 500.0).is_err());

        // Valid
        assert!(CrossfadePoints::calculate(1000.0, 1000.0, 500.0).is_ok());
    }
}