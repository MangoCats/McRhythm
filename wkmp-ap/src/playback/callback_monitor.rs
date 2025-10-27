//! Audio callback timing monitor for gap/stutter detection
//!
//! Tracks audio callback invocations to detect timing irregularities
//! that cause audible gaps/stutters.
//!
//! **Purpose:** Detect gaps happening in audio output layer that
//! current ring buffer instrumentation misses.

use std::sync::atomic::{AtomicU64, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tracing::{warn, debug, error, info};
use wkmp_common::events::WkmpEvent;
use crate::state::SharedState;

/// Audio callback timing monitor
///
/// Tracks every callback invocation to detect:
/// - Irregular callback intervals (timing jitter)
/// - Underrun events (even single occurrences)
/// - Callback frequency deviation
///
/// **Design:** Lock-free for use in real-time audio callback
pub struct CallbackMonitor {
    /// Start time for monotonic elapsed time calculation
    /// Uses Instant (monotonic clock) instead of SystemTime (can go backwards)
    start_time: Instant,

    /// Last callback elapsed time (nanoseconds since start_time)
    last_callback_ns: AtomicU64,

    /// Total callback invocations
    callback_count: AtomicU64,

    /// Total underrun events (buffer empty)
    underrun_count: AtomicU64,

    /// Count of irregular intervals (>2ms deviation from expected)
    irregular_intervals: AtomicU64,

    /// Expected interval between callbacks (nanoseconds)
    /// Calculated as: (buffer_size / sample_rate) * 1e9
    /// Example: 512 frames @ 44.1kHz = 11.6ms = 11,600,000 ns
    expected_interval_ns: u64,

    /// Tolerance for irregular interval detection (nanoseconds)
    /// Default: 2ms = 2,000,000 ns
    tolerance_ns: u64,

    /// Shared state for event emission (used by monitoring thread)
    state: Option<Arc<SharedState>>,
}

impl CallbackMonitor {
    /// Create new callback monitor
    ///
    /// # Arguments
    /// - `sample_rate`: Audio sample rate (e.g., 44100)
    /// - `buffer_size`: Audio buffer size in frames (e.g., 512)
    /// - `state`: Optional shared state for event emission
    pub fn new(sample_rate: u32, buffer_size: u32, state: Option<Arc<SharedState>>) -> Self {
        // Calculate expected callback interval
        // interval = buffer_size / sample_rate (in seconds)
        // Convert to nanoseconds: * 1_000_000_000
        let expected_interval_ns = ((buffer_size as f64 / sample_rate as f64) * 1_000_000_000.0) as u64;

        debug!(
            "CallbackMonitor initialized: sample_rate={}, buffer_size={}, expected_interval={}ms",
            sample_rate, buffer_size, expected_interval_ns / 1_000_000
        );

        Self {
            start_time: Instant::now(),
            last_callback_ns: AtomicU64::new(0),
            callback_count: AtomicU64::new(0),
            underrun_count: AtomicU64::new(0),
            irregular_intervals: AtomicU64::new(0),
            expected_interval_ns,
            tolerance_ns: 2_000_000, // 2ms tolerance
            state,
        }
    }

    /// Record callback invocation (call at start of audio callback)
    ///
    /// **REAL-TIME SAFE**: Only atomic operations, no logging, no events, no system calls
    ///
    /// **Thread Safety:** Lock-free, safe for real-time audio callback
    pub fn record_callback(&self) {
        // Use monotonic time (nanoseconds since start_time)
        let now_ns = self.start_time.elapsed().as_nanos() as u64;
        let last_ns = self.last_callback_ns.swap(now_ns, Ordering::Relaxed);
        self.callback_count.fetch_add(1, Ordering::Relaxed);

        // Skip first callback (no previous timestamp)
        if last_ns == 0 {
            return;
        }

        // Calculate actual interval
        let actual_interval_ns = now_ns.saturating_sub(last_ns);
        let deviation_ns = actual_interval_ns.abs_diff(self.expected_interval_ns);

        // Check if irregular - ONLY increment counter, NO logging/events
        if deviation_ns > self.tolerance_ns {
            self.irregular_intervals.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Record underrun event (buffer empty)
    ///
    /// **REAL-TIME SAFE**: Only atomic operations, no logging, no events, no system calls
    ///
    /// **Thread Safety:** Lock-free, safe for real-time audio callback
    pub fn record_underrun(&self) {
        // ONLY increment counter - monitoring thread will detect and log
        self.underrun_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Get current statistics
    pub fn stats(&self) -> CallbackStats {
        CallbackStats {
            callback_count: self.callback_count.load(Ordering::Relaxed),
            underrun_count: self.underrun_count.load(Ordering::Relaxed),
            irregular_intervals: self.irregular_intervals.load(Ordering::Relaxed),
            expected_interval_ms: self.expected_interval_ns / 1_000_000,
        }
    }

    /// Spawn monitoring task that polls stats and emits events/logs
    ///
    /// Runs on tokio runtime, separate from real-time audio callback.
    /// Polls statistics every 100ms and emits events when counters change.
    ///
    /// **Returns:** Shutdown flag (set to true to stop monitoring)
    pub fn spawn_monitoring_task(
        self: Arc<Self>,
        rt_handle: tokio::runtime::Handle,
    ) -> Arc<std::sync::atomic::AtomicBool> {
        let shutdown = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let shutdown_clone = Arc::clone(&shutdown);
        let monitor = Arc::clone(&self);

        rt_handle.spawn(async move {
            let mut last_callback_count = 0u64;
            let mut last_underrun_count = 0u64;
            let mut last_irregular_count = 0u64;
            let mut last_irregular_event = std::time::Instant::now();

            debug!("CallbackMonitor: Monitoring task started");

            while !shutdown_clone.load(Ordering::Relaxed) {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                let stats = monitor.stats();

                // Check for new underruns
                if stats.underrun_count > last_underrun_count {
                    let new_underruns = stats.underrun_count - last_underrun_count;
                    warn!(
                        "Audio callback underrun detected: {} total underruns (+{} since last check)",
                        stats.underrun_count, new_underruns
                    );

                    // Emit event
                    if let Some(state) = &monitor.state {
                        state.broadcast_event(WkmpEvent::AudioCallbackUnderrun {
                            underrun_count: stats.underrun_count,
                            timestamp: chrono::Utc::now(),
                        });
                    }

                    last_underrun_count = stats.underrun_count;
                }

                // Check for new irregular intervals (throttled to once per 5 seconds)
                if stats.irregular_intervals > last_irregular_count {
                    let new_irregular = stats.irregular_intervals - last_irregular_count;

                    // Log every 100 irregular intervals
                    if new_irregular >= 100 || stats.irregular_intervals % 1000 == 0 {
                        warn!(
                            "Audio callback irregular intervals: {} total (+{} since last check), {} total callbacks",
                            stats.irregular_intervals, new_irregular, stats.callback_count
                        );
                    }

                    // Emit event (throttled to 5 seconds)
                    if last_irregular_event.elapsed().as_secs() >= 5 {
                        if let Some(state) = &monitor.state {
                            state.broadcast_event(WkmpEvent::AudioCallbackIrregular {
                                actual_interval_ms: 0, // Don't have interval data here
                                expected_interval_ms: stats.expected_interval_ms,
                                deviation_ms: 0, // Don't have deviation data here
                                total_irregular_count: stats.irregular_intervals,
                                timestamp: chrono::Utc::now(),
                            });
                        }
                        last_irregular_event = std::time::Instant::now();
                    }

                    last_irregular_count = stats.irregular_intervals;
                }

                // Periodic health check (every ~30 seconds)
                if stats.callback_count > last_callback_count && stats.callback_count % 3000 == 0 {
                    debug!(
                        "Audio callback health: {} callbacks, {} underruns, {} irregular intervals ({:.1}% irregular)",
                        stats.callback_count,
                        stats.underrun_count,
                        stats.irregular_intervals,
                        (stats.irregular_intervals as f64 / stats.callback_count as f64) * 100.0
                    );
                }

                last_callback_count = stats.callback_count;
            }

            info!("CallbackMonitor: Monitoring task stopped");
        });

        shutdown
    }
}

/// Callback statistics snapshot
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct CallbackStats {
    pub callback_count: u64,
    pub underrun_count: u64,
    pub irregular_intervals: u64,
    pub expected_interval_ms: u64,
}
