//! Fade curve application for crossfades
//!
//! Implements fade-in/fade-out curve application per SPEC002.
//!
//! # Fade Curves
//!
//! Per SPEC002 XFD-CURV-010:
//! - **Exponential**: Slow start, fast finish (fade-in)
//! - **Logarithmic**: Fast start, slow finish (fade-out)
//! - **Cosine**: Smooth S-curve (both)
//! - **Linear**: Constant rate (both)
//!
//! # Timing Points
//!
//! Per SPEC002 XFD-TP-010:
//! - **Start**: Passage begins (silence before)
//! - **Fade-In**: Volume ramp starts
//! - **Lead-In**: Full volume begins
//! - **Lead-Out**: Fade-out begins
//! - **Fade-Out**: Volume ramp to zero starts
//! - **End**: Passage ends (silence after)
//!
//! # Architecture
//!
//! Phase 4: Fader applies curves to samples based on tick-based position
//! Integration: Used by DecoderChain to apply fades to decoded/resampled audio

use crate::Result;

/// Fade curve types per SPEC002 XFD-CURV-010
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FadeCurve {
    /// Exponential curve (fade-in: slow start, fast finish)
    /// Formula: y = x^2
    Exponential,

    /// Logarithmic curve (fade-out: fast start, slow finish)
    /// Formula: y = sqrt(x)
    Logarithmic,

    /// Cosine S-curve (smooth acceleration/deceleration)
    /// Formula: y = (1 - cos(πx)) / 2
    Cosine,

    /// Linear curve (constant rate of change)
    /// Formula: y = x
    Linear,
}

/// Fader for applying fade curves to audio samples
///
/// Applies fade-in/fade-out curves based on tick-based position.
///
/// # Examples
///
/// ```ignore
/// let mut fader = Fader::new(
///     0,                  // passage_start_ticks
///     0,                  // fade_in_start_ticks
///     28_224_000,         // lead_in_start_ticks (1 second)
///     282_240_000,        // lead_out_start_ticks (10 seconds)
///     282_240_000,        // fade_out_start_ticks
///     310_464_000,        // passage_end_ticks (11 seconds)
///     FadeCurve::Exponential,
///     FadeCurve::Logarithmic,
///     44100,
/// );
///
/// let mut samples = vec![1.0f32; 8];  // 4 stereo samples
/// fader.apply_fade(&mut samples)?;
/// ```
pub struct Fader {
    /// Passage start in ticks
    passage_start_ticks: i64,

    /// Fade-in start in ticks
    fade_in_start_ticks: i64,

    /// Lead-in start in ticks (full volume begins)
    lead_in_start_ticks: i64,

    /// Lead-out start in ticks (fade-out begins)
    lead_out_start_ticks: i64,

    /// Fade-out start in ticks
    fade_out_start_ticks: i64,

    /// Passage end in ticks
    passage_end_ticks: i64,

    /// Fade-in curve type
    fade_in_curve: FadeCurve,

    /// Fade-out curve type
    fade_out_curve: FadeCurve,

    /// Current position in ticks
    position_ticks: i64,

    /// Working sample rate (for tick to sample conversion)
    sample_rate: u32,
}

impl Fader {
    /// Create new fader with timing points
    ///
    /// # Arguments
    ///
    /// * `passage_start_ticks` - Passage start (silence before)
    /// * `fade_in_start_ticks` - Fade-in begins
    /// * `lead_in_start_ticks` - Full volume begins
    /// * `lead_out_start_ticks` - Fade-out begins
    /// * `fade_out_start_ticks` - Volume ramp to zero begins
    /// * `passage_end_ticks` - Passage ends (silence after)
    /// * `fade_in_curve` - Curve type for fade-in
    /// * `fade_out_curve` - Curve type for fade-out
    /// * `sample_rate` - Working sample rate (typically 44100)
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let fader = Fader::new(
    ///     0, 0, 28_224_000,
    ///     282_240_000, 282_240_000, 310_464_000,
    ///     FadeCurve::Exponential,
    ///     FadeCurve::Logarithmic,
    ///     44100,
    /// );
    /// ```
    pub fn new(
        passage_start_ticks: i64,
        fade_in_start_ticks: i64,
        lead_in_start_ticks: i64,
        lead_out_start_ticks: i64,
        fade_out_start_ticks: i64,
        passage_end_ticks: i64,
        fade_in_curve: FadeCurve,
        fade_out_curve: FadeCurve,
        sample_rate: u32,
    ) -> Self {
        Self {
            passage_start_ticks,
            fade_in_start_ticks,
            lead_in_start_ticks,
            lead_out_start_ticks,
            fade_out_start_ticks,
            passage_end_ticks,
            fade_in_curve,
            fade_out_curve,
            position_ticks: passage_start_ticks,
            sample_rate,
        }
    }

    /// Apply fade to audio samples in-place
    ///
    /// Modifies samples based on current position and fade curves.
    /// Advances position by sample count.
    ///
    /// # Arguments
    ///
    /// * `samples` - Interleaved stereo f32 samples [L, R, L, R, ...]
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mut samples = vec![1.0f32; 8];  // 4 stereo samples
    /// fader.apply_fade(&mut samples)?;
    /// ```
    pub fn apply_fade(&mut self, samples: &mut [f32]) -> Result<()> {
        // Validate stereo sample count
        if samples.len() % 2 != 0 {
            return Err(crate::AudioPlayerError::Buffer(
                crate::error::BufferError::InvalidSampleCount(samples.len()),
            ));
        }

        let frames = samples.len() / 2;
        let ticks_per_sample = Self::ticks_per_sample(self.sample_rate);

        for frame_idx in 0..frames {
            let frame_ticks = self.position_ticks + (frame_idx as i64 * ticks_per_sample);
            let multiplier = self.calculate_multiplier(frame_ticks);

            // Apply to both L and R channels
            samples[frame_idx * 2] *= multiplier;
            samples[frame_idx * 2 + 1] *= multiplier;
        }

        // Advance position
        self.position_ticks += frames as i64 * ticks_per_sample;

        Ok(())
    }

    /// Calculate fade multiplier (0.0 to 1.0) for given tick position
    fn calculate_multiplier(&self, ticks: i64) -> f32 {
        // Before passage start: silence
        if ticks < self.passage_start_ticks {
            return 0.0;
        }

        // After passage end: silence
        if ticks >= self.passage_end_ticks {
            return 0.0;
        }

        // Fade-in region
        if ticks < self.lead_in_start_ticks {
            let fade_start = self.fade_in_start_ticks;
            let fade_end = self.lead_in_start_ticks;
            let fade_duration = fade_end - fade_start;

            if fade_duration <= 0 {
                return 1.0; // No fade-in
            }

            let progress = (ticks - fade_start) as f64 / fade_duration as f64;
            let progress = progress.clamp(0.0, 1.0);

            return Self::apply_curve(progress, self.fade_in_curve) as f32;
        }

        // Lead-out region (full volume)
        if ticks < self.lead_out_start_ticks {
            return 1.0;
        }

        // Fade-out region
        if ticks < self.passage_end_ticks {
            let fade_start = self.fade_out_start_ticks;
            let fade_end = self.passage_end_ticks;
            let fade_duration = fade_end - fade_start;

            if fade_duration <= 0 {
                return 1.0; // No fade-out
            }

            let progress = (ticks - fade_start) as f64 / fade_duration as f64;
            let progress = progress.clamp(0.0, 1.0);

            // Fade-out: 1.0 at start, 0.0 at end
            return Self::apply_curve(1.0 - progress, self.fade_out_curve) as f32;
        }

        0.0
    }

    /// Apply fade curve to progress value (0.0 to 1.0)
    fn apply_curve(progress: f64, curve: FadeCurve) -> f64 {
        match curve {
            FadeCurve::Linear => progress,

            FadeCurve::Exponential => {
                // Exponential: y = x^2 (slow start, fast finish)
                progress * progress
            }

            FadeCurve::Logarithmic => {
                // Logarithmic: y = sqrt(x) (fast start, slow finish)
                progress.sqrt()
            }

            FadeCurve::Cosine => {
                // Cosine S-curve: y = (1 - cos(πx)) / 2
                (1.0 - (std::f64::consts::PI * progress).cos()) / 2.0
            }
        }
    }

    /// Calculate ticks per sample for given sample rate
    fn ticks_per_sample(sample_rate: u32) -> i64 {
        // Per SPEC017 SRC-TICK-020: tick_rate = 28,224,000 ticks/second
        const TICK_RATE: i64 = 28_224_000;
        TICK_RATE / sample_rate as i64
    }

    /// Get current position in ticks
    pub fn position_ticks(&self) -> i64 {
        self.position_ticks
    }

    /// Seek to specific tick position
    pub fn seek(&mut self, ticks: i64) {
        self.position_ticks = ticks;
    }
}

// ========================================
// Tests
// ========================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_fade_in() {
        // Create fader with 1-second fade-in at 44100 Hz
        let sample_rate = 44100;
        let tick_rate = 28_224_000;
        let fade_duration_ticks = tick_rate; // 1 second

        let fader = Fader::new(
            0,                       // passage_start
            0,                       // fade_in_start
            fade_duration_ticks,     // lead_in_start (end of fade-in)
            tick_rate * 10,          // lead_out_start (10s)
            tick_rate * 10,          // fade_out_start
            tick_rate * 11,          // passage_end
            FadeCurve::Linear,
            FadeCurve::Linear,
            sample_rate,
        );

        // Test at 0% (start)
        assert_eq!(fader.calculate_multiplier(0), 0.0);

        // Test at 50% (middle)
        assert!((fader.calculate_multiplier(fade_duration_ticks / 2) - 0.5).abs() < 0.01);

        // Test at 100% (end of fade-in)
        assert_eq!(fader.calculate_multiplier(fade_duration_ticks), 1.0);
    }

    #[test]
    fn test_linear_fade_out() {
        // Create fader with 1-second fade-out at 44100 Hz
        let sample_rate = 44100;
        let tick_rate = 28_224_000;
        let fade_duration_ticks = tick_rate; // 1 second

        let fader = Fader::new(
            0,                       // passage_start
            0,                       // fade_in_start
            0,                       // lead_in_start (no fade-in)
            tick_rate * 10,          // lead_out_start (10s)
            tick_rate * 10,          // fade_out_start
            tick_rate * 11,          // passage_end
            FadeCurve::Linear,
            FadeCurve::Linear,
            sample_rate,
        );

        // Test at start of fade-out
        assert_eq!(fader.calculate_multiplier(tick_rate * 10), 1.0);

        // Test at 50% of fade-out
        let mid_fade = tick_rate * 10 + fade_duration_ticks / 2;
        assert!((fader.calculate_multiplier(mid_fade) - 0.5).abs() < 0.01);

        // Test at end of fade-out
        assert_eq!(fader.calculate_multiplier(tick_rate * 11), 0.0);
    }

    #[test]
    fn test_exponential_curve() {
        let progress = 0.5;
        let result = Fader::apply_curve(progress, FadeCurve::Exponential);
        // Exponential at 0.5 should be 0.25 (0.5^2)
        assert!((result - 0.25).abs() < 0.01);
    }

    #[test]
    fn test_logarithmic_curve() {
        let progress = 0.25;
        let result = Fader::apply_curve(progress, FadeCurve::Logarithmic);
        // Logarithmic at 0.25 should be 0.5 (sqrt(0.25))
        assert!((result - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_cosine_curve() {
        let progress = 0.5;
        let result = Fader::apply_curve(progress, FadeCurve::Cosine);
        // Cosine S-curve at 0.5 should be 0.5
        assert!((result - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_apply_fade_to_samples() {
        let sample_rate = 44100;
        let tick_rate = 28_224_000;

        let mut fader = Fader::new(
            0,
            0,
            tick_rate,           // 1 second fade-in
            tick_rate * 10,
            tick_rate * 10,
            tick_rate * 11,
            FadeCurve::Linear,
            FadeCurve::Linear,
            sample_rate,
        );

        // Create test samples (4 stereo samples = 8 f32 values)
        let mut samples = vec![1.0f32; 8];

        // Apply fade at start (should be ~0.0)
        fader.seek(0);
        fader.apply_fade(&mut samples).unwrap();

        // Samples should be near zero
        assert!(samples[0].abs() < 0.01);
        assert!(samples[1].abs() < 0.01);
    }

    #[test]
    fn test_apply_fade_full_volume() {
        let sample_rate = 44100;
        let tick_rate = 28_224_000;

        let mut fader = Fader::new(
            0,
            0,
            tick_rate,           // 1 second fade-in
            tick_rate * 10,
            tick_rate * 10,
            tick_rate * 11,
            FadeCurve::Linear,
            FadeCurve::Linear,
            sample_rate,
        );

        // Create test samples
        let mut samples = vec![1.0f32; 8];

        // Apply fade in middle (full volume region)
        fader.seek(tick_rate * 5);
        fader.apply_fade(&mut samples).unwrap();

        // Samples should remain 1.0
        assert_eq!(samples[0], 1.0);
        assert_eq!(samples[1], 1.0);
    }

    #[test]
    fn test_apply_fade_odd_samples_fails() {
        let sample_rate = 44100;
        let tick_rate = 28_224_000;

        let mut fader = Fader::new(
            0, 0, tick_rate, tick_rate * 10, tick_rate * 10, tick_rate * 11,
            FadeCurve::Linear, FadeCurve::Linear, sample_rate,
        );

        let mut samples = vec![1.0f32; 7]; // Odd number
        let result = fader.apply_fade(&mut samples);

        assert!(result.is_err());
    }

    #[test]
    fn test_position_advances() {
        let sample_rate = 44100;
        let tick_rate = 28_224_000;

        let mut fader = Fader::new(
            0, 0, tick_rate, tick_rate * 10, tick_rate * 10, tick_rate * 11,
            FadeCurve::Linear, FadeCurve::Linear, sample_rate,
        );

        let initial_position = fader.position_ticks();

        // Apply fade to 4 stereo samples
        let mut samples = vec![1.0f32; 8];
        fader.apply_fade(&mut samples).unwrap();

        // Position should advance by 4 samples worth of ticks
        let ticks_per_sample = 28_224_000 / 44100; // 640 ticks/sample
        let expected_position = initial_position + (4 * ticks_per_sample);
        assert_eq!(fader.position_ticks(), expected_position);
    }
}
