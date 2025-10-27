//! Parameter space search algorithms
//!
//! **Purpose:** Binary search and parameter exploration for finding optimal buffer values.
//!
//! **Traceability:**
//! - TUNE-SRC-010: Explore parameter space systematically
//! - TUNE-SEARCH-010: Binary search for minimum stable buffer
//! - TUNE-SEARCH-020: Early termination conditions

use crate::tuning::metrics::{TestResult, Verdict};

/// Binary search for minimum stable buffer size
///
/// Given a mixer interval, finds the smallest audio_buffer_size that maintains
/// stability (Verdict::Stable or Verdict::Warning).
///
/// **Algorithm:**
/// 1. Start with low=64, high=4096 (or provided range)
/// 2. Test midpoint: mid = (low + high) / 2
/// 3. If stable: Try smaller (high = mid)
/// 4. If unstable: Need larger (low = mid + 1)
/// 5. Converge when (high - low) ≤ 128 frames
///
/// **Convergence:** 128 frames (~3ms @ 44.1kHz) precision
///
/// **Traceability:** TUNE-SEARCH-010 (lines 185-205 in specification)
///
/// # Arguments
/// - `interval_ms`: Mixer check interval in milliseconds
/// - `low`: Minimum buffer size to search (inclusive)
/// - `high`: Maximum buffer size to search (inclusive)
/// - `test_fn`: Function that tests a configuration and returns TestResult
///
/// # Returns
/// Minimum stable buffer size (in frames)
///
/// # Panics
/// Never panics - handles all edge cases gracefully
pub fn binary_search_min_buffer<F>(
    interval_ms: u64,
    mut low: u32,
    mut high: u32,
    test_fn: F,
) -> u32
where
    F: Fn(u64, u32) -> TestResult,
{
    // Convergence threshold: 128 frames (ISSUE-H-003 resolution)
    const CONVERGENCE_THRESHOLD: u32 = 128;

    let mut best_stable = high; // Start with maximum as fallback

    // Binary search until convergence
    while (high - low) > CONVERGENCE_THRESHOLD {
        let mid = (low + high) / 2;
        let result = test_fn(interval_ms, mid);

        match result.verdict {
            Verdict::Stable | Verdict::Warning => {
                // This size works - try smaller
                best_stable = mid;
                high = mid;
            }
            Verdict::Unstable => {
                // This size doesn't work - need larger
                low = mid + 1;
            }
        }
    }

    // After convergence, test the low bound to see if we can go smaller
    // This handles the "all stable" case where we want to return the minimum
    if low < best_stable {
        let low_result = test_fn(interval_ms, low);
        if matches!(low_result.verdict, Verdict::Stable | Verdict::Warning) {
            return low;
        }
    }

    best_stable
}

/// Phase 1: Coarse sweep across mixer intervals
///
/// Tests a range of mixer intervals with a default buffer size to identify
/// which intervals are viable (stable or marginally stable).
///
/// **Default Test Points:**
/// - 1ms, 2ms, 5ms, 10ms, 20ms, 50ms, 100ms
///
/// **Traceability:** TUNE-ALG-010 (Phase 1: Coarse sweep)
///
/// # Arguments
/// - `default_buffer_size`: Buffer size to use for all tests (typically 512)
/// - `test_fn`: Function that tests a configuration
///
/// # Returns
/// Vec of (interval_ms, TestResult) for viable intervals
pub fn coarse_sweep<F>(default_buffer_size: u32, test_fn: F) -> Vec<(u64, TestResult)>
where
    F: Fn(u64, u32) -> TestResult,
{
    const TEST_INTERVALS: &[u64] = &[1, 2, 5, 10, 20, 50, 100];

    TEST_INTERVALS
        .iter()
        .map(|&interval_ms| {
            let result = test_fn(interval_ms, default_buffer_size);
            (interval_ms, result)
        })
        .collect()
}

/// Filter viable intervals from coarse sweep results
///
/// Returns only intervals where the test passed with Stable or Warning verdict.
///
/// # Arguments
/// - `results`: Results from coarse_sweep
///
/// # Returns
/// Vec of viable interval values (ms)
pub fn filter_viable_intervals(results: &[(u64, TestResult)]) -> Vec<u64> {
    results
        .iter()
        .filter(|(_, result)| matches!(result.verdict, Verdict::Stable | Verdict::Warning))
        .map(|(interval, _)| *interval)
        .collect()
}

/// Early termination check
///
/// Determines if search should be aborted early based on failure patterns.
///
/// **Termination Conditions (TUNE-SEARCH-020):**
/// - 3 consecutive failures at different buffer sizes
/// - Interval ≥50ms with buffer size 64 fails (system too slow)
///
/// # Arguments
/// - `interval_ms`: Current mixer interval being tested
/// - `failure_count`: Number of consecutive failures
///
/// # Returns
/// true if search should terminate early
pub fn should_terminate_early(interval_ms: u64, failure_count: u32) -> bool {
    // 3 consecutive failures - system likely unstable for this interval
    if failure_count >= 3 {
        return true;
    }

    // If 50ms+ interval can't work even with large buffer, system is too slow
    if interval_ms >= 50 && failure_count > 0 {
        return true;
    }

    false
}

/// Parameter space exploration
///
/// Comprehensive search combining coarse sweep and binary search.
///
/// **Algorithm (TUNE-ALG-010):**
/// 1. Phase 1: Coarse sweep with default buffer
/// 2. Filter viable intervals
/// 3. Phase 2: Binary search for minimum buffer per interval
///
/// # Arguments
/// - `default_buffer_size`: Starting buffer size for coarse sweep
/// - `test_fn`: Function that tests a configuration
///
/// # Returns
/// Vec of (interval_ms, min_stable_buffer_size) for all viable configurations
pub fn explore_parameter_space<F>(default_buffer_size: u32, test_fn: F) -> Vec<(u64, u32)>
where
    F: Fn(u64, u32) -> TestResult,
{
    // Phase 1: Coarse sweep
    let sweep_results = coarse_sweep(default_buffer_size, &test_fn);
    let viable_intervals = filter_viable_intervals(&sweep_results);

    // Phase 2: Binary search for each viable interval
    viable_intervals
        .into_iter()
        .map(|interval_ms| {
            let min_buffer = binary_search_min_buffer(interval_ms, 64, 4096, &test_fn);
            (interval_ms, min_buffer)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tuning::metrics::{
        BufferOccupancyMetrics, CpuMetrics, JitterMetrics, TestResult, UnderrunMetrics, Verdict,
    };

    /// Mock test function: Buffers ≥384 are stable, <384 are unstable
    fn mock_test_boundary_384(_interval_ms: u64, buffer_size: u32) -> TestResult {
        let underruns = if buffer_size >= 384 {
            UnderrunMetrics::new(1, 2000) // 0.05% - Stable
        } else {
            UnderrunMetrics::new(30, 2000) // 1.5% - Unstable
        };

        TestResult::new(
            _interval_ms,
            buffer_size,
            30,
            underruns,
            JitterMetrics::from_intervals(&[10.0]),
            BufferOccupancyMetrics::from_samples(&[500]),
            CpuMetrics::unavailable(),
        )
    }

    /// Mock test function: All buffer sizes stable
    fn mock_all_stable(_interval_ms: u64, buffer_size: u32) -> TestResult {
        let underruns = UnderrunMetrics::new(0, 1000); // 0% - Stable

        TestResult::new(
            _interval_ms,
            buffer_size,
            30,
            underruns,
            JitterMetrics::from_intervals(&[10.0]),
            BufferOccupancyMetrics::from_samples(&[500]),
            CpuMetrics::unavailable(),
        )
    }

    /// Mock test function: All buffer sizes unstable
    fn mock_all_unstable(_interval_ms: u64, buffer_size: u32) -> TestResult {
        let underruns = UnderrunMetrics::new(100, 2000); // 5% - Unstable

        TestResult::new(
            _interval_ms,
            buffer_size,
            30,
            underruns,
            JitterMetrics::from_intervals(&[10.0]),
            BufferOccupancyMetrics::from_samples(&[500]),
            CpuMetrics::unavailable(),
        )
    }

    #[test]
    fn test_binary_search_convergence() {
        // Test case: Boundary at 384 frames
        let result = binary_search_min_buffer(10, 64, 4096, mock_test_boundary_384);

        // Should converge within 128 frames of boundary (384-512 range)
        assert!(result >= 384, "Result {} below stability boundary", result);
        assert!(
            result <= 512,
            "Result {} beyond convergence threshold",
            result
        );

        // Verify result is actually stable
        let verification = mock_test_boundary_384(10, result);
        assert!(matches!(
            verification.verdict,
            Verdict::Stable | Verdict::Warning
        ));
    }

    #[test]
    fn test_binary_search_all_stable() {
        // When all sizes are stable, should return minimum
        let result = binary_search_min_buffer(10, 64, 4096, mock_all_stable);
        assert_eq!(result, 64, "Should return minimum when all stable");
    }

    #[test]
    fn test_binary_search_all_unstable() {
        // When all sizes are unstable, should return maximum
        let result = binary_search_min_buffer(10, 64, 4096, mock_all_unstable);
        assert_eq!(result, 4096, "Should return maximum when all unstable");
    }

    #[test]
    fn test_coarse_sweep() {
        let results = coarse_sweep(512, mock_all_stable);

        // Should test all 7 intervals
        assert_eq!(results.len(), 7);

        // All should be stable
        for (_, result) in &results {
            assert_eq!(result.verdict, Verdict::Stable);
        }

        // Intervals should match expected values
        assert_eq!(results[0].0, 1);
        assert_eq!(results[1].0, 2);
        assert_eq!(results[2].0, 5);
        assert_eq!(results[3].0, 10);
        assert_eq!(results[4].0, 20);
        assert_eq!(results[5].0, 50);
        assert_eq!(results[6].0, 100);
    }

    #[test]
    fn test_filter_viable_intervals() {
        // Create mixed results
        let results = vec![
            (
                1,
                mock_all_unstable(1, 512), // Unstable
            ),
            (
                5,
                mock_all_stable(5, 512), // Stable
            ),
            (
                10,
                mock_all_stable(10, 512), // Stable
            ),
            (
                20,
                mock_all_unstable(20, 512), // Unstable
            ),
        ];

        let viable = filter_viable_intervals(&results);

        // Should only return stable intervals
        assert_eq!(viable.len(), 2);
        assert_eq!(viable[0], 5);
        assert_eq!(viable[1], 10);
    }

    #[test]
    fn test_early_termination() {
        // 3 consecutive failures - should terminate
        assert!(should_terminate_early(10, 3));
        assert!(!should_terminate_early(10, 2));

        // 50ms+ interval with any failure - should terminate
        assert!(should_terminate_early(50, 1));
        assert!(should_terminate_early(100, 1));
        assert!(!should_terminate_early(20, 1));
    }

    #[test]
    fn test_explore_parameter_space() {
        let results = explore_parameter_space(512, mock_all_stable);

        // Should find all 7 intervals viable
        assert_eq!(results.len(), 7);

        // All should have minimum buffer size (64) since everything is stable
        for (interval, buffer_size) in &results {
            assert_eq!(*buffer_size, 64, "Interval {}ms should have min buffer", interval);
        }
    }
}
