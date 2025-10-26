//! Crossfade timing calculation
//!
//! Calculates when to trigger crossfades based on passage timing parameters.
//! Implements the algorithm specified in crossfade.md.
//!
//! **Traceability:**
//! - [XFD-IMPL-020] Timing calculation algorithm
//! - [XFD-IMPL-030] Clamped crossfade time calculation

use crate::error::Error;

/// Passage timing data from database/API
///
/// All times in milliseconds relative to audio file
#[derive(Debug, Clone)]
pub struct PassageTiming {
    /// Passage start time in audio file (ms)
    pub start_time_ms: u64,

    /// Passage end time in audio file (ms)
    pub end_time_ms: u64,

    /// Fade-in point (when volume reaches 100%), or None for default
    pub fade_in_point_ms: Option<u64>,

    /// Lead-in point (latest time previous passage may still be playing), or None for default
    pub lead_in_point_ms: Option<u64>,

    /// Lead-out point (earliest time next passage may start), or None for default
    pub lead_out_point_ms: Option<u64>,

    /// Fade-out point (when volume begins decreasing), or None for default
    pub fade_out_point_ms: Option<u64>,
}

/// Crossfade timing calculation result
///
/// **[XFD-IMPL-020]** Result of timing calculation
#[derive(Debug, Clone)]
pub struct CrossfadeTiming {
    /// When lead-out starts in current passage (ms from passage start)
    pub lead_out_start_ms: u64,

    /// When fade-out starts in current passage (ms from passage start)
    pub fade_out_start_ms: u64,

    /// When fade-in starts in next passage (ms from passage start)
    pub fade_in_start_ms: u64,

    /// Duration of crossfade overlap (ms)
    pub crossfade_duration_ms: u32,

    /// Fade-out duration for current passage (ms)
    pub fade_out_duration_ms: u32,

    /// Fade-in duration for next passage (ms)
    pub fade_in_duration_ms: u32,
}

impl CrossfadeTiming {
    /// Calculate crossfade timing for passage transition
    ///
    /// **[XFD-IMPL-020]** Main timing calculation algorithm
    ///
    /// # Arguments
    /// * `current` - Timing data for currently playing passage
    /// * `next` - Timing data for next passage
    /// * `global_crossfade_time_ms` - Global crossfade time from settings
    ///
    /// # Returns
    /// Crossfade timing parameters, or error if passages invalid
    pub fn calculate(
        current: &PassageTiming,
        next: &PassageTiming,
        global_crossfade_time_ms: u32,
    ) -> Result<Self, Error> {
        // Validate passage durations
        if current.start_time_ms >= current.end_time_ms {
            return Err(Error::InvalidTiming(format!(
                "Current passage: invalid start/end times ({} >= {})",
                current.start_time_ms, current.end_time_ms
            )));
        }

        if next.start_time_ms >= next.end_time_ms {
            return Err(Error::InvalidTiming(format!(
                "Next passage: invalid start/end times ({} >= {})",
                next.start_time_ms, next.end_time_ms
            )));
        }

        // Calculate passage durations
        let current_duration_ms = current.end_time_ms - current.start_time_ms;
        let next_duration_ms = next.end_time_ms - next.start_time_ms;

        // [XFD-IMPL-030] Clamp global crossfade time to 50% of shortest passage
        let max_allowed_current = current_duration_ms / 2;
        let max_allowed_next = next_duration_ms / 2;
        let max_allowed = max_allowed_current.min(max_allowed_next);
        let effective_crossfade_time_ms =
            (global_crossfade_time_ms as u64).min(max_allowed) as u32;

        // Step 1: Determine lead-out point of current passage
        // [XFD-IMPL-020] Use defined value or compute from global crossfade time
        let lead_out_start_ms = current
            .lead_out_point_ms
            .unwrap_or_else(|| current.end_time_ms.saturating_sub(effective_crossfade_time_ms as u64));

        // Step 2: Determine fade-out start of current passage
        let fade_out_start_ms = current
            .fade_out_point_ms
            .unwrap_or_else(|| current.end_time_ms.saturating_sub(effective_crossfade_time_ms as u64));

        // Step 3: Determine fade-in start of next passage
        let fade_in_start_ms = next
            .fade_in_point_ms
            .unwrap_or_else(|| next.start_time_ms + effective_crossfade_time_ms as u64);

        // Step 4: Determine lead-in start of next passage
        let lead_in_start_ms = next
            .lead_in_point_ms
            .unwrap_or_else(|| next.start_time_ms + effective_crossfade_time_ms as u64);

        // Step 5: Calculate crossfade durations
        // Duration from current fade-out start to current lead-out end (passage end)
        let fade_out_duration_ms =
            (current.end_time_ms.saturating_sub(fade_out_start_ms)) as u32;

        // Duration from next lead-in start (passage start) to next fade-in end
        let fade_in_duration_ms =
            (fade_in_start_ms.saturating_sub(next.start_time_ms)) as u32;

        // Lead-out duration (from lead-out point to end)
        let lead_out_duration_ms =
            (current.end_time_ms.saturating_sub(lead_out_start_ms)) as u32;

        // Lead-in duration (from start to lead-in point)
        let lead_in_duration_ms =
            (lead_in_start_ms.saturating_sub(next.start_time_ms)) as u32;

        // Crossfade duration is minimum of lead-out and lead-in durations
        let crossfade_duration_ms = lead_out_duration_ms.min(lead_in_duration_ms);

        Ok(CrossfadeTiming {
            lead_out_start_ms,
            fade_out_start_ms,
            fade_in_start_ms,
            crossfade_duration_ms,
            fade_out_duration_ms,
            fade_in_duration_ms,
        })
    }

    /// Calculate when to trigger crossfade in current passage
    ///
    /// Returns the sample position (relative to passage start) when crossfade should begin
    ///
    /// # Arguments
    /// * `sample_rate` - Audio sample rate (typically 44100)
    ///
    /// # Returns
    /// Sample position to trigger crossfade
    pub fn crossfade_trigger_sample(&self, sample_rate: u32) -> usize {
        // Trigger when we reach lead_out_start_ms in current passage
        self.ms_to_samples(self.lead_out_start_ms, sample_rate)
    }

    /// Convert milliseconds to samples
    fn ms_to_samples(&self, ms: u64, sample_rate: u32) -> usize {
        ((ms as f64 / 1000.0) * sample_rate as f64) as usize
    }

    /// Convert to samples for mixer
    ///
    /// Returns (fade_out_samples, fade_in_samples)
    pub fn to_samples(&self, sample_rate: u32) -> (usize, usize) {
        let fade_out_samples = self.ms_to_samples(self.fade_out_duration_ms as u64, sample_rate);
        let fade_in_samples = self.ms_to_samples(self.fade_in_duration_ms as u64, sample_rate);
        (fade_out_samples, fade_in_samples)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_crossfade_timing() {
        // Two 60-second passages with 5-second global crossfade time
        let current = PassageTiming {
            start_time_ms: 0,
            end_time_ms: 60000,
            fade_in_point_ms: None,
            lead_in_point_ms: None,
            lead_out_point_ms: None,
            fade_out_point_ms: None,
        };

        let next = PassageTiming {
            start_time_ms: 0,
            end_time_ms: 60000,
            fade_in_point_ms: None,
            lead_in_point_ms: None,
            lead_out_point_ms: None,
            fade_out_point_ms: None,
        };

        let timing = CrossfadeTiming::calculate(&current, &next, 5000).unwrap();

        // Lead-out should start at 55 seconds (60 - 5)
        assert_eq!(timing.lead_out_start_ms, 55000);

        // Fade-out should start at 55 seconds
        assert_eq!(timing.fade_out_start_ms, 55000);

        // Fade-in should complete at 5 seconds
        assert_eq!(timing.fade_in_start_ms, 5000);

        // Crossfade duration should be 5 seconds
        assert_eq!(timing.crossfade_duration_ms, 5000);
    }

    #[test]
    fn test_crossfade_with_clamping() {
        // Short 10-second passage with 30-second global crossfade time
        // Should clamp to 5 seconds (50% of 10 seconds)
        let current = PassageTiming {
            start_time_ms: 0,
            end_time_ms: 10000,
            fade_in_point_ms: None,
            lead_in_point_ms: None,
            lead_out_point_ms: None,
            fade_out_point_ms: None,
        };

        let next = PassageTiming {
            start_time_ms: 0,
            end_time_ms: 60000,
            fade_in_point_ms: None,
            lead_in_point_ms: None,
            lead_out_point_ms: None,
            fade_out_point_ms: None,
        };

        let timing = CrossfadeTiming::calculate(&current, &next, 30000).unwrap();

        // Should clamp to 5000ms (50% of 10-second passage)
        assert_eq!(timing.lead_out_start_ms, 5000);
        assert_eq!(timing.crossfade_duration_ms, 5000);
    }

    #[test]
    fn test_crossfade_with_explicit_timing() {
        // Passages with explicit timing points
        let current = PassageTiming {
            start_time_ms: 0,
            end_time_ms: 60000,
            fade_in_point_ms: Some(2000),
            lead_in_point_ms: Some(3000),
            lead_out_point_ms: Some(50000),
            fade_out_point_ms: Some(55000),
        };

        let next = PassageTiming {
            start_time_ms: 0,
            end_time_ms: 60000,
            fade_in_point_ms: Some(7000),
            lead_in_point_ms: Some(8000),
            lead_out_point_ms: None,
            fade_out_point_ms: None,
        };

        let timing = CrossfadeTiming::calculate(&current, &next, 5000).unwrap();

        // Should use explicit values, not defaults
        assert_eq!(timing.lead_out_start_ms, 50000);
        assert_eq!(timing.fade_out_start_ms, 55000);
        assert_eq!(timing.fade_in_start_ms, 7000);

        // Crossfade duration is min(lead_out_duration, lead_in_duration)
        // lead_out_duration = 60000 - 50000 = 10000
        // lead_in_duration = 8000 - 0 = 8000
        // min = 8000
        assert_eq!(timing.crossfade_duration_ms, 8000);
    }

    #[test]
    fn test_zero_duration_crossfade() {
        // Zero lead durations should result in no overlap
        let current = PassageTiming {
            start_time_ms: 0,
            end_time_ms: 60000,
            fade_in_point_ms: Some(0),
            lead_in_point_ms: Some(0),
            lead_out_point_ms: Some(60000),
            fade_out_point_ms: Some(60000),
        };

        let next = PassageTiming {
            start_time_ms: 0,
            end_time_ms: 60000,
            fade_in_point_ms: Some(0),
            lead_in_point_ms: Some(0),
            lead_out_point_ms: Some(60000),
            fade_out_point_ms: Some(60000),
        };

        let timing = CrossfadeTiming::calculate(&current, &next, 5000).unwrap();

        // Lead-out at end, no duration
        assert_eq!(timing.lead_out_start_ms, 60000);
        assert_eq!(timing.crossfade_duration_ms, 0);
    }

    #[test]
    fn test_asymmetric_fade_durations() {
        // Different fade-in and fade-out durations
        let current = PassageTiming {
            start_time_ms: 0,
            end_time_ms: 60000,
            fade_in_point_ms: None,
            lead_in_point_ms: None,
            lead_out_point_ms: Some(50000), // 10-second lead-out
            fade_out_point_ms: Some(56000), // 4-second fade-out
        };

        let next = PassageTiming {
            start_time_ms: 0,
            end_time_ms: 60000,
            fade_in_point_ms: Some(3000), // 3-second fade-in
            lead_in_point_ms: Some(8000), // 8-second lead-in
            lead_out_point_ms: None,
            fade_out_point_ms: None,
        };

        let timing = CrossfadeTiming::calculate(&current, &next, 5000).unwrap();

        assert_eq!(timing.fade_out_duration_ms, 4000);
        assert_eq!(timing.fade_in_duration_ms, 3000);

        // Crossfade duration is min(10000, 8000) = 8000
        assert_eq!(timing.crossfade_duration_ms, 8000);
    }

    #[test]
    fn test_invalid_passage_timing() {
        // Start time >= end time should error
        let invalid = PassageTiming {
            start_time_ms: 60000,
            end_time_ms: 60000,
            fade_in_point_ms: None,
            lead_in_point_ms: None,
            lead_out_point_ms: None,
            fade_out_point_ms: None,
        };

        let valid = PassageTiming {
            start_time_ms: 0,
            end_time_ms: 60000,
            fade_in_point_ms: None,
            lead_in_point_ms: None,
            lead_out_point_ms: None,
            fade_out_point_ms: None,
        };

        let result = CrossfadeTiming::calculate(&invalid, &valid, 5000);
        assert!(result.is_err());

        let result2 = CrossfadeTiming::calculate(&valid, &invalid, 5000);
        assert!(result2.is_err());
    }

    #[test]
    fn test_crossfade_trigger_sample() {
        let timing = CrossfadeTiming {
            lead_out_start_ms: 55000,
            fade_out_start_ms: 55000,
            fade_in_start_ms: 5000,
            crossfade_duration_ms: 5000,
            fade_out_duration_ms: 5000,
            fade_in_duration_ms: 5000,
        };

        // At 44100 Hz, 55 seconds = 55 * 44100 = 2,425,500 samples
        let trigger = timing.crossfade_trigger_sample(44100);
        assert_eq!(trigger, 2_425_500);
    }

    #[test]
    fn test_to_samples() {
        let timing = CrossfadeTiming {
            lead_out_start_ms: 55000,
            fade_out_start_ms: 55000,
            fade_in_start_ms: 5000,
            crossfade_duration_ms: 5000,
            fade_out_duration_ms: 3000,
            fade_in_duration_ms: 2000,
        };

        let (fade_out_samples, fade_in_samples) = timing.to_samples(44100);

        // 3000ms = 3 * 44100 = 132,300 samples
        assert_eq!(fade_out_samples, 132_300);

        // 2000ms = 2 * 44100 = 88,200 samples
        assert_eq!(fade_in_samples, 88_200);
    }
}
