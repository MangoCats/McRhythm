//! Metrics collection and classification for buffer auto-tuning tests
//!
//! **Purpose:** Detect buffer underruns, measure audio callback health, and classify
//! parameter stability using defined thresholds.
//!
//! **Traceability:**
//! - TUNE-DET-010: Underrun detection and counting
//! - TUNE-DET-020: Audio health metrics (jitter, occupancy, CPU)
//! - TUNE-DET-030: Stability classification thresholds

use serde::{Deserialize, Serialize};

/// Test stability verdict based on underrun rate
///
/// **Thresholds (TUNE-DET-030):**
/// - Stable: <0.1% underruns (safe for production)
/// - Warning: 0.1-1% underruns (marginal, use with caution)
/// - Unstable: >1% underruns (unsafe, unacceptable)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Verdict {
    /// <0.1% underrun rate - safe for production use
    Stable,
    /// 0.1-1% underrun rate - marginal stability, use with caution
    Warning,
    /// >1% underrun rate - unacceptable for production use
    Unstable,
}

impl Verdict {
    /// Classify underrun rate into stability verdict
    ///
    /// **Thresholds:**
    /// - Unstable: >1.0% underrun rate
    /// - Warning: 0.1-1.0% underrun rate
    /// - Stable: <0.1% underrun rate
    ///
    /// **Traceability:** TUNE-DET-030
    pub fn from_underrun_rate(underrun_rate: f64) -> Self {
        if underrun_rate > 1.0 {
            Verdict::Unstable
        } else if underrun_rate >= 0.1 {
            Verdict::Warning
        } else {
            Verdict::Stable
        }
    }
}

/// Buffer underrun metrics
///
/// Tracks underrun events and calculates underrun rate as percentage
/// of total audio callback invocations.
///
/// **Traceability:** TUNE-DET-010
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct UnderrunMetrics {
    /// Total underrun events during test
    pub underrun_count: u64,

    /// Total audio callback invocations during test
    pub callback_count: u64,

    /// Underrun rate as percentage (0-100)
    /// Calculated as: (underrun_count / callback_count) * 100
    pub underrun_rate: f64,
}

impl UnderrunMetrics {
    /// Create underrun metrics from counts
    ///
    /// Automatically calculates underrun rate as percentage.
    ///
    /// # Arguments
    /// - `underrun_count`: Number of underrun events detected
    /// - `callback_count`: Total number of audio callbacks invoked
    ///
    /// # Returns
    /// UnderrunMetrics with calculated rate
    pub fn new(underrun_count: u64, callback_count: u64) -> Self {
        let underrun_rate = if callback_count > 0 {
            (underrun_count as f64 / callback_count as f64) * 100.0
        } else {
            0.0
        };

        Self {
            underrun_count,
            callback_count,
            underrun_rate,
        }
    }

    /// Get stability verdict based on underrun rate
    pub fn verdict(&self) -> Verdict {
        Verdict::from_underrun_rate(self.underrun_rate)
    }
}

/// Audio callback jitter metrics
///
/// Tracks callback timing regularity to detect scheduling issues.
/// High jitter indicates CPU scheduling problems that may cause gaps.
///
/// **Traceability:** TUNE-DET-020 (callback regularity)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct JitterMetrics {
    /// Mean callback interval in milliseconds
    pub mean_interval_ms: f64,

    /// Standard deviation of callback intervals in milliseconds
    pub std_dev_ms: f64,

    /// Maximum callback interval observed in milliseconds
    pub max_interval_ms: f64,

    /// Count of irregular intervals (>2ms deviation from expected)
    pub irregular_count: u64,
}

impl JitterMetrics {
    /// Create jitter metrics from timing samples
    ///
    /// # Arguments
    /// - `intervals_ms`: Array of callback intervals in milliseconds
    ///
    /// # Returns
    /// JitterMetrics with calculated statistics
    pub fn from_intervals(intervals_ms: &[f64]) -> Self {
        if intervals_ms.is_empty() {
            return Self {
                mean_interval_ms: 0.0,
                std_dev_ms: 0.0,
                max_interval_ms: 0.0,
                irregular_count: 0,
            };
        }

        let mean = intervals_ms.iter().sum::<f64>() / intervals_ms.len() as f64;

        let variance = intervals_ms
            .iter()
            .map(|&x| {
                let diff = x - mean;
                diff * diff
            })
            .sum::<f64>()
            / intervals_ms.len() as f64;

        let std_dev = variance.sqrt();
        let max_interval = intervals_ms.iter().copied().fold(0.0_f64, f64::max);

        // Count irregularities (>2ms deviation from mean)
        let irregular_count = intervals_ms
            .iter()
            .filter(|&&interval| (interval - mean).abs() > 2.0)
            .count() as u64;

        Self {
            mean_interval_ms: mean,
            std_dev_ms: std_dev,
            max_interval_ms: max_interval,
            irregular_count,
        }
    }
}

/// Buffer occupancy metrics
///
/// Tracks ring buffer fill levels over time to detect starvation/saturation.
///
/// **Traceability:** TUNE-DET-020 (buffer occupancy)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BufferOccupancyMetrics {
    /// Minimum buffer occupancy observed (frames)
    pub min_frames: u32,

    /// Maximum buffer occupancy observed (frames)
    pub max_frames: u32,

    /// Mean buffer occupancy (frames)
    pub mean_frames: f64,

    /// 10th percentile (frames) - indicates how close to underrun
    pub p10_frames: u32,

    /// 90th percentile (frames) - indicates how close to overrun
    pub p90_frames: u32,
}

impl BufferOccupancyMetrics {
    /// Create occupancy metrics from buffer samples
    ///
    /// # Arguments
    /// - `samples`: Array of buffer occupancy samples (in frames)
    ///
    /// # Returns
    /// BufferOccupancyMetrics with calculated statistics
    pub fn from_samples(samples: &[u32]) -> Self {
        if samples.is_empty() {
            return Self {
                min_frames: 0,
                max_frames: 0,
                mean_frames: 0.0,
                p10_frames: 0,
                p90_frames: 0,
            };
        }

        let min = *samples.iter().min().unwrap_or(&0);
        let max = *samples.iter().max().unwrap_or(&0);
        let mean = samples.iter().map(|&x| x as f64).sum::<f64>() / samples.len() as f64;

        // Calculate percentiles
        let mut sorted = samples.to_vec();
        sorted.sort_unstable();

        let p10_idx = (samples.len() as f64 * 0.10) as usize;
        let p90_idx = (samples.len() as f64 * 0.90) as usize;

        let p10 = sorted.get(p10_idx).copied().unwrap_or(min);
        let p90 = sorted.get(p90_idx.min(sorted.len() - 1)).copied().unwrap_or(max);

        Self {
            min_frames: min,
            max_frames: max,
            mean_frames: mean,
            p10_frames: p10,
            p90_frames: p90,
        }
    }
}

/// CPU usage metrics
///
/// Tracks process CPU utilization during test.
/// Measured as percentage of wall-clock time (0-100% per core).
///
/// **Traceability:** TUNE-DET-020 (CPU usage)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CpuMetrics {
    /// Average CPU usage as percentage (0-100)
    pub avg_percent: f64,

    /// Peak CPU usage as percentage (0-100)
    pub peak_percent: f64,

    /// Whether CPU measurement was available
    /// (Linux: /proc/self/stat, other platforms: may be unavailable)
    pub available: bool,
}

impl CpuMetrics {
    /// Create CPU metrics from usage samples
    ///
    /// # Arguments
    /// - `samples`: Array of CPU usage percentages
    ///
    /// # Returns
    /// CpuMetrics with calculated statistics
    pub fn from_samples(samples: &[f64]) -> Self {
        if samples.is_empty() {
            return Self {
                avg_percent: 0.0,
                peak_percent: 0.0,
                available: false,
            };
        }

        let avg = samples.iter().sum::<f64>() / samples.len() as f64;
        let peak = samples.iter().copied().fold(0.0_f64, f64::max);

        Self {
            avg_percent: avg,
            peak_percent: peak,
            available: true,
        }
    }

    /// Create unavailable CPU metrics
    pub fn unavailable() -> Self {
        Self {
            avg_percent: 0.0,
            peak_percent: 0.0,
            available: false,
        }
    }
}

/// Complete test result for a single parameter combination
///
/// Contains all metrics and stability verdict for one test run.
///
/// **Traceability:** TUNE-DET-010, TUNE-DET-020, TUNE-DET-030
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    /// Parameters tested
    pub mixer_check_interval_ms: u64,
    pub audio_buffer_size: u32,

    /// Test duration in seconds
    pub test_duration_secs: u64,

    /// Underrun metrics
    pub underruns: UnderrunMetrics,

    /// Jitter metrics
    pub jitter: JitterMetrics,

    /// Buffer occupancy metrics
    pub occupancy: BufferOccupancyMetrics,

    /// CPU usage metrics
    pub cpu: CpuMetrics,

    /// Overall stability verdict
    pub verdict: Verdict,
}

impl TestResult {
    /// Create test result from collected metrics
    ///
    /// Verdict is automatically determined from underrun rate.
    pub fn new(
        mixer_check_interval_ms: u64,
        audio_buffer_size: u32,
        test_duration_secs: u64,
        underruns: UnderrunMetrics,
        jitter: JitterMetrics,
        occupancy: BufferOccupancyMetrics,
        cpu: CpuMetrics,
    ) -> Self {
        let verdict = underruns.verdict();

        Self {
            mixer_check_interval_ms,
            audio_buffer_size,
            test_duration_secs,
            underruns,
            jitter,
            occupancy,
            cpu,
            verdict,
        }
    }

    /// Convenience accessor for underrun rate
    pub fn underrun_rate(&self) -> f64 {
        self.underruns.underrun_rate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verdict_classification() {
        // Stable: <0.1%
        assert_eq!(Verdict::from_underrun_rate(0.0), Verdict::Stable);
        assert_eq!(Verdict::from_underrun_rate(0.05), Verdict::Stable);
        assert_eq!(Verdict::from_underrun_rate(0.09), Verdict::Stable);

        // Warning: 0.1-1.0%
        assert_eq!(Verdict::from_underrun_rate(0.1), Verdict::Warning);
        assert_eq!(Verdict::from_underrun_rate(0.5), Verdict::Warning);
        assert_eq!(Verdict::from_underrun_rate(1.0), Verdict::Warning);

        // Unstable: >1.0%
        assert_eq!(Verdict::from_underrun_rate(1.01), Verdict::Unstable);
        assert_eq!(Verdict::from_underrun_rate(5.0), Verdict::Unstable);
        assert_eq!(Verdict::from_underrun_rate(100.0), Verdict::Unstable);
    }

    #[test]
    fn test_underrun_metrics() {
        // No underruns (0% rate)
        let metrics = UnderrunMetrics::new(0, 1000);
        assert_eq!(metrics.underrun_count, 0);
        assert_eq!(metrics.callback_count, 1000);
        assert_eq!(metrics.underrun_rate, 0.0);
        assert_eq!(metrics.verdict(), Verdict::Stable);

        // 1 underrun in 1000 callbacks (0.1% - boundary of Warning)
        let metrics = UnderrunMetrics::new(1, 1000);
        assert_eq!(metrics.underrun_rate, 0.1);
        assert_eq!(metrics.verdict(), Verdict::Warning);

        // 5 underruns in 1000 callbacks (0.5% - Warning)
        let metrics = UnderrunMetrics::new(5, 1000);
        assert_eq!(metrics.underrun_rate, 0.5);
        assert_eq!(metrics.verdict(), Verdict::Warning);

        // 20 underruns in 1000 callbacks (2% - Unstable)
        let metrics = UnderrunMetrics::new(20, 1000);
        assert_eq!(metrics.underrun_rate, 2.0);
        assert_eq!(metrics.verdict(), Verdict::Unstable);
    }

    #[test]
    fn test_jitter_metrics() {
        // Perfect regularity (no jitter)
        let intervals = vec![10.0, 10.0, 10.0, 10.0, 10.0];
        let jitter = JitterMetrics::from_intervals(&intervals);
        assert_eq!(jitter.mean_interval_ms, 10.0);
        assert_eq!(jitter.std_dev_ms, 0.0);
        assert_eq!(jitter.max_interval_ms, 10.0);
        assert_eq!(jitter.irregular_count, 0);

        // Some jitter
        let intervals = vec![10.0, 11.0, 9.0, 10.5, 10.0];
        let jitter = JitterMetrics::from_intervals(&intervals);
        assert!((jitter.mean_interval_ms - 10.1).abs() < 0.01);
        assert!(jitter.std_dev_ms > 0.0);
        assert_eq!(jitter.max_interval_ms, 11.0);

        // High jitter (>2ms deviation)
        let intervals = vec![10.0, 15.0, 8.0, 10.0, 13.0];
        let jitter = JitterMetrics::from_intervals(&intervals);
        assert!(jitter.irregular_count > 0);
    }

    #[test]
    fn test_buffer_occupancy_metrics() {
        let samples = vec![100, 200, 300, 400, 500, 600, 700, 800, 900, 1000];
        let occupancy = BufferOccupancyMetrics::from_samples(&samples);

        assert_eq!(occupancy.min_frames, 100);
        assert_eq!(occupancy.max_frames, 1000);
        assert_eq!(occupancy.mean_frames, 550.0);
        // p10 = 10% of 10 values = index 1 = 200
        assert_eq!(occupancy.p10_frames, 200);
        // p90 = 90% of 10 values = index 9 = 1000
        assert_eq!(occupancy.p90_frames, 1000);
    }

    #[test]
    fn test_cpu_metrics() {
        let samples = vec![10.0, 15.0, 12.0, 20.0, 18.0];
        let cpu = CpuMetrics::from_samples(&samples);

        assert_eq!(cpu.avg_percent, 15.0);
        assert_eq!(cpu.peak_percent, 20.0);
        assert!(cpu.available);

        let unavailable = CpuMetrics::unavailable();
        assert!(!unavailable.available);
    }

    #[test]
    fn test_test_result() {
        let underruns = UnderrunMetrics::new(0, 1000);
        let jitter = JitterMetrics::from_intervals(&[10.0, 10.0, 10.0]);
        let occupancy = BufferOccupancyMetrics::from_samples(&[500, 600, 700]);
        let cpu = CpuMetrics::from_samples(&[15.0, 20.0]);

        let result = TestResult::new(10, 512, 30, underruns, jitter, occupancy, cpu);

        assert_eq!(result.mixer_check_interval_ms, 10);
        assert_eq!(result.audio_buffer_size, 512);
        assert_eq!(result.test_duration_secs, 30);
        assert_eq!(result.verdict, Verdict::Stable);
        assert_eq!(result.underrun_rate(), 0.0);
    }
}
