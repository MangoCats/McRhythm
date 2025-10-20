//! Audio quality analysis functions for integration tests
//!
//! Provides FFT, RMS, phase analysis to detect clicks, pops, and other artifacts

use std::f32::consts::PI;

/// Click event detected in audio
#[derive(Debug, Clone)]
pub struct ClickEvent {
    pub sample_index: usize,
    pub timestamp_ms: u64,
    pub peak_db: f32,
    pub frequency_hz: f32,
}

/// Pop event detected in audio
#[derive(Debug, Clone)]
pub struct PopEvent {
    pub sample_index: usize,
    pub timestamp_ms: u64,
    pub amplitude_change_db: f32,
}

/// RMS continuity analysis report
#[derive(Debug, Clone)]
pub struct RmsContinuityReport {
    pub passed: bool,
    pub max_jump_db: f32,
    pub jump_locations: Vec<(usize, f32)>, // (sample_index, jump_db)
    pub rms_timeline: Vec<(u64, f32)>,     // (timestamp_ms, rms)
}

/// Phase continuity analysis report
#[derive(Debug, Clone)]
pub struct PhaseContinuityReport {
    pub passed: bool,
    pub inversions_detected: Vec<usize>, // sample indices
    pub stereo_coherence: f32,           // 0.0-1.0
}

/// Complete audio analysis report
#[derive(Debug, Clone)]
pub struct AudioAnalysisReport {
    pub clicks_detected: usize,
    pub pops_detected: usize,
    pub click_events: Vec<ClickEvent>,
    pub pop_events: Vec<PopEvent>,
    pub rms_report: Option<RmsContinuityReport>,
    pub phase_report: Option<PhaseContinuityReport>,
}

impl AudioAnalysisReport {
    /// Analyze audio samples
    pub fn analyze(samples: &[f32], sample_rate: u32) -> Self {
        let clicks = detect_clicks(samples, sample_rate);
        let pops = detect_pops(samples, sample_rate);
        let rms_report = Some(verify_rms_continuity(samples, (0, samples.len()), sample_rate));
        let phase_report = Some(verify_phase_continuity(samples, sample_rate));

        Self {
            clicks_detected: clicks.len(),
            pops_detected: pops.len(),
            click_events: clicks,
            pop_events: pops,
            rms_report,
            phase_report,
        }
    }
}

/// Detect clicks using FFT analysis
///
/// Clicks appear as wideband frequency spikes >-60dB above baseline
pub fn detect_clicks(samples: &[f32], sample_rate: u32) -> Vec<ClickEvent> {
    let mut clicks = Vec::new();

    // Use simplified click detection (real implementation would use realfft)
    // For now, detect sudden high-frequency content

    const WINDOW_SIZE: usize = 2048; // ~46ms at 44.1kHz
    const HOP_SIZE: usize = 512;     // 75% overlap
    const CLICK_THRESHOLD_DB: f32 = -60.0;

    if samples.len() < WINDOW_SIZE * 2 {
        return clicks; // Not enough samples
    }

    // Process overlapping windows
    for i in (0..samples.len() - WINDOW_SIZE * 2).step_by(HOP_SIZE * 2) {
        // Get stereo window (interleaved)
        let window_left: Vec<f32> = samples[i..i + WINDOW_SIZE * 2]
            .iter()
            .step_by(2)
            .copied()
            .collect();

        // Calculate high-frequency energy (simplified - real FFT would be better)
        let high_freq_energy = calculate_high_freq_energy(&window_left);
        let high_freq_db = 20.0 * high_freq_energy.log10();

        if high_freq_db > CLICK_THRESHOLD_DB {
            clicks.push(ClickEvent {
                sample_index: i,
                timestamp_ms: (i as f64 / 2.0 / sample_rate as f64 * 1000.0) as u64,
                peak_db: high_freq_db,
                frequency_hz: 0.0, // Would be calculated by FFT peak detection
            });
        }
    }

    clicks
}

/// Calculate high-frequency energy (simplified version without FFT)
fn calculate_high_freq_energy(samples: &[f32]) -> f32 {
    // Simple high-pass filter approximation
    let mut energy = 0.0;

    for i in 1..samples.len() {
        let diff = samples[i] - samples[i - 1];
        energy += diff * diff;
    }

    (energy / samples.len() as f32).sqrt()
}

/// Detect pops (sudden amplitude changes >6dB in <10ms)
pub fn detect_pops(samples: &[f32], sample_rate: u32) -> Vec<PopEvent> {
    let mut pops = Vec::new();

    const WINDOW_MS: f32 = 10.0; // 10ms window
    const POP_THRESHOLD_DB: f32 = 6.0;

    let window_size = ((WINDOW_MS / 1000.0) * sample_rate as f32) as usize * 2; // Stereo

    if samples.len() < window_size * 2 {
        return pops;
    }

    // Calculate RMS in sliding windows
    let mut prev_rms = calculate_rms(&samples[0..window_size]);

    for i in (window_size..samples.len() - window_size).step_by(window_size) {
        let current_rms = calculate_rms(&samples[i..i + window_size]);

        // Calculate dB change
        let db_change = 20.0 * (current_rms / prev_rms.max(0.0001)).log10();

        if db_change.abs() > POP_THRESHOLD_DB {
            pops.push(PopEvent {
                sample_index: i,
                timestamp_ms: (i as f64 / 2.0 / sample_rate as f64 * 1000.0) as u64,
                amplitude_change_db: db_change,
            });
        }

        prev_rms = current_rms;
    }

    pops
}

/// Verify RMS continuity (no jumps >1dB)
pub fn verify_rms_continuity(
    samples: &[f32],
    region: (usize, usize),
    sample_rate: u32,
) -> RmsContinuityReport {
    let (start, end) = region;
    let region_samples = &samples[start..end.min(samples.len())];

    const WINDOW_MS: f32 = 100.0; // 100ms windows
    const MAX_JUMP_DB: f32 = 1.0;

    let window_size = ((WINDOW_MS / 1000.0) * sample_rate as f32) as usize * 2; // Stereo

    let mut rms_timeline = Vec::new();
    let mut jump_locations = Vec::new();
    let mut max_jump_db = 0.0;

    if region_samples.len() < window_size * 2 {
        return RmsContinuityReport {
            passed: true,
            max_jump_db: 0.0,
            jump_locations: Vec::new(),
            rms_timeline: Vec::new(),
        };
    }

    // Calculate RMS for each window
    let mut prev_rms = calculate_rms(&region_samples[0..window_size]);
    rms_timeline.push((0, prev_rms));

    for (idx, i) in (window_size..region_samples.len() - window_size)
        .step_by(window_size)
        .enumerate()
    {
        let current_rms = calculate_rms(&region_samples[i..i + window_size]);
        let timestamp_ms = ((start + i) as f64 / 2.0 / sample_rate as f64 * 1000.0) as u64;

        rms_timeline.push((timestamp_ms, current_rms));

        // Calculate jump in dB
        let jump_db = 20.0 * (current_rms / prev_rms.max(0.0001)).log10().abs();

        if jump_db > max_jump_db {
            max_jump_db = jump_db;
        }

        if jump_db > MAX_JUMP_DB {
            jump_locations.push((start + i, jump_db));
        }

        prev_rms = current_rms;
    }

    RmsContinuityReport {
        passed: max_jump_db <= MAX_JUMP_DB,
        max_jump_db,
        jump_locations,
        rms_timeline,
    }
}

/// Verify phase continuity (no inversions)
pub fn verify_phase_continuity(samples: &[f32], sample_rate: u32) -> PhaseContinuityReport {
    let mut inversions = Vec::new();

    // Simplified phase check: look for sudden polarity flips
    // Real implementation would use cross-correlation

    const WINDOW_SIZE: usize = 1024; // ~23ms at 44.1kHz

    if samples.len() < WINDOW_SIZE * 4 {
        return PhaseContinuityReport {
            passed: true,
            inversions_detected: Vec::new(),
            stereo_coherence: 1.0,
        };
    }

    // Check for polarity inversions
    for i in (0..samples.len() - WINDOW_SIZE * 2).step_by(WINDOW_SIZE * 2) {
        let window = &samples[i..i + WINDOW_SIZE * 2];

        // Calculate L/R correlation
        let correlation = calculate_stereo_correlation(window);

        // Negative correlation suggests phase inversion
        if correlation < -0.5 {
            inversions.push(i);
        }
    }

    // Calculate overall stereo coherence
    let coherence = calculate_stereo_correlation(samples);

    PhaseContinuityReport {
        passed: inversions.is_empty(),
        inversions_detected: inversions,
        stereo_coherence: coherence,
    }
}

/// Calculate RMS (root mean square) of samples
pub fn calculate_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }

    let sum: f32 = samples.iter().map(|s| s * s).sum();
    (sum / samples.len() as f32).sqrt()
}

/// Calculate stereo correlation (L/R channel correlation)
fn calculate_stereo_correlation(samples: &[f32]) -> f32 {
    if samples.len() < 2 {
        return 1.0;
    }

    let mut sum_l = 0.0;
    let mut sum_r = 0.0;
    let mut sum_lr = 0.0;
    let mut sum_l2 = 0.0;
    let mut sum_r2 = 0.0;
    let n = samples.len() / 2;

    for i in 0..n {
        let l = samples[i * 2];
        let r = samples[i * 2 + 1];

        sum_l += l;
        sum_r += r;
        sum_lr += l * r;
        sum_l2 += l * l;
        sum_r2 += r * r;
    }

    let mean_l = sum_l / n as f32;
    let mean_r = sum_r / n as f32;

    let numerator = (sum_lr / n as f32) - (mean_l * mean_r);
    let denominator = ((sum_l2 / n as f32 - mean_l * mean_l) * (sum_r2 / n as f32 - mean_r * mean_r)).sqrt();

    if denominator < 0.0001 {
        return 1.0; // Avoid division by zero
    }

    numerator / denominator
}

/// Calculate variance of RMS timeline
pub fn calculate_variance(rms_timeline: &[(u64, f32)]) -> f32 {
    if rms_timeline.is_empty() {
        return 0.0;
    }

    let values: Vec<f32> = rms_timeline.iter().map(|(_, rms)| *rms).collect();
    let mean: f32 = values.iter().sum::<f32>() / values.len() as f32;

    let variance: f32 = values.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / values.len() as f32;

    variance
}

/// Measure startup latency
pub fn measure_startup_latency(
    start_time: std::time::Instant,
    audio_capture: &super::AudioCapture,
    threshold: f32,
) -> Option<std::time::Duration> {
    let samples = audio_capture.get_samples();

    // Find first sample above threshold
    for (i, sample) in samples.iter().enumerate() {
        if sample.abs() > threshold {
            // Calculate time to this sample
            let sample_rate = audio_capture.sample_rate();
            let elapsed_samples = i / 2; // Stereo interleaved
            let elapsed_seconds = elapsed_samples as f64 / sample_rate as f64;
            return Some(std::time::Duration::from_secs_f64(elapsed_seconds));
        }
    }

    None
}

/// Calculate linear regression slope (for memory leak detection)
pub fn calculate_linear_regression_slope(values: &[u64]) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }

    let n = values.len() as f64;
    let sum_x: f64 = (0..values.len()).map(|i| i as f64).sum();
    let sum_y: f64 = values.iter().map(|&v| v as f64).sum();
    let sum_xy: f64 = values.iter().enumerate().map(|(i, &v)| i as f64 * v as f64).sum();
    let sum_x2: f64 = (0..values.len()).map(|i| (i * i) as f64).sum();

    let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x * sum_x);

    slope
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_rms() {
        let samples = vec![0.0, 0.0, 1.0, 1.0];
        let rms = calculate_rms(&samples);
        assert!((rms - 0.707).abs() < 0.01); // sqrt(0.5) ≈ 0.707
    }

    #[test]
    fn test_detect_pops() {
        // Create samples with a pop (sudden amplitude jump)
        let mut samples = vec![0.1; 4410 * 2]; // 100ms at 44.1kHz, stereo

        // Insert pop: sudden jump from 0.1 to 0.9
        for i in 0..4410 * 2 {
            samples.push(0.9);
        }

        let pops = detect_pops(&samples, 44100);

        // Should detect at least one pop
        assert!(pops.len() > 0);
        assert!(pops[0].amplitude_change_db > 6.0);
    }

    #[test]
    fn test_stereo_correlation() {
        // Perfect correlation (mono signal)
        let samples = vec![0.5, 0.5, 0.6, 0.6, 0.7, 0.7];
        let corr = calculate_stereo_correlation(&samples);
        assert!(corr > 0.99);

        // Perfect anti-correlation (inverted)
        let samples = vec![0.5, -0.5, 0.6, -0.6, 0.7, -0.7];
        let corr = calculate_stereo_correlation(&samples);
        assert!(corr < -0.99);
    }

    #[test]
    fn test_variance() {
        let timeline = vec![
            (0, 0.5),
            (100, 0.5),
            (200, 0.5),
            (300, 0.5),
        ];
        let variance = calculate_variance(&timeline);
        assert!(variance < 0.001); // Near zero for constant values

        let timeline = vec![
            (0, 0.1),
            (100, 0.9),
            (200, 0.1),
            (300, 0.9),
        ];
        let variance = calculate_variance(&timeline);
        assert!(variance > 0.1); // High for varying values
    }

    #[test]
    fn test_linear_regression() {
        // Perfect linear growth: y = 2x
        let values = vec![0, 2, 4, 6, 8, 10];
        let slope = calculate_linear_regression_slope(&values);
        assert!((slope - 2.0).abs() < 0.01);

        // Flat line
        let values = vec![5, 5, 5, 5, 5];
        let slope = calculate_linear_regression_slope(&values);
        assert!(slope.abs() < 0.01);
    }
}
