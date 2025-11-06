//! Audio callback timing monitor for gap/stutter detection
//!
//! Tracks audio callback invocations to detect timing irregularities
//! that cause audible gaps/stutters.
//!
//! **Purpose:** Detect gaps happening in audio output layer that
//! current ring buffer instrumentation misses.

use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tracing::{warn, debug, info, trace};
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
/// **Dynamic Calibration:** Learns actual callback interval during first 100 callbacks
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
    /// Atomic for dynamic calibration updates
    expected_interval_ns: AtomicU64,

    /// Tolerance for irregular interval detection (nanoseconds)
    /// Atomic for dynamic calibration updates
    tolerance_ns: AtomicU64,

    /// Calibration buffer for measuring actual intervals
    /// Locked (not used in audio callback - only monitoring thread)
    calibration_samples: Mutex<Vec<u64>>,

    /// Calibration complete flag
    calibration_complete: AtomicBool,

    /// Shared state for event emission (used by monitoring thread)
    state: Option<Arc<SharedState>>,

    /// Audio expected flag from playback engine
    /// True when Playing state with non-empty queue
    /// False when Paused or queue is empty
    /// Used to distinguish expected underruns (idle) from problematic ones (active playback)
    audio_expected: Arc<AtomicBool>,
}

impl CallbackMonitor {
    /// Create new callback monitor with dynamic interval detection
    ///
    /// # Arguments
    /// - `sample_rate`: Audio sample rate (e.g., 44100)
    /// - `buffer_size`: Audio buffer size in frames (e.g., 512)
    /// - `state`: Optional shared state for event emission
    /// - `audio_expected`: Flag indicating if audio output is expected (Playing with non-empty queue)
    ///
    /// # Dynamic Calibration
    /// The monitor starts with calculated expected interval but will calibrate
    /// to actual device behavior during the first 100 callbacks. This handles
    /// cases where audio drivers (esp. Windows WASAPI) use different actual
    /// callback frequencies than the negotiated buffer size suggests.
    pub fn new(
        sample_rate: u32,
        buffer_size: u32,
        state: Option<Arc<SharedState>>,
        audio_expected: Arc<AtomicBool>,
    ) -> Self {
        // Calculate initial expected callback interval
        // interval = buffer_size / sample_rate (in seconds)
        // Convert to nanoseconds: * 1_000_000_000
        let expected_interval_ns = ((buffer_size as f64 / sample_rate as f64) * 1_000_000_000.0) as u64;

        // Initial tolerance: 20% of expected interval (handles WASAPI variability)
        let tolerance_ns = (expected_interval_ns as f64 * 0.20) as u64;

        info!(
            "CallbackMonitor initialized: sample_rate={}, buffer_size={}, initial_expected_interval={:.2}ms, tolerance={:.2}ms (calibrating...)",
            sample_rate, buffer_size,
            expected_interval_ns as f64 / 1_000_000.0,
            tolerance_ns as f64 / 1_000_000.0
        );

        Self {
            start_time: Instant::now(),
            last_callback_ns: AtomicU64::new(0),
            callback_count: AtomicU64::new(0),
            underrun_count: AtomicU64::new(0),
            irregular_intervals: AtomicU64::new(0),
            expected_interval_ns: AtomicU64::new(expected_interval_ns),
            tolerance_ns: AtomicU64::new(tolerance_ns),
            calibration_samples: Mutex::new(Vec::with_capacity(100)),
            calibration_complete: AtomicBool::new(false),
            state,
            audio_expected,
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
        let count = self.callback_count.fetch_add(1, Ordering::Relaxed);

        // Skip first callback (no previous timestamp)
        if last_ns == 0 {
            return;
        }

        // Calculate actual interval
        let actual_interval_ns = now_ns.saturating_sub(last_ns);

        // During calibration phase (first 100 callbacks), store interval for monitoring thread
        // Uses try_lock to avoid blocking audio thread - if lock fails, skip this sample
        // Note: count is the OLD value before increment, so count=0..99 for callbacks 1-100
        if !self.calibration_complete.load(Ordering::Relaxed) && count > 0 && count <= 100 {
            if let Ok(mut samples) = self.calibration_samples.try_lock() {
                samples.push(actual_interval_ns);
            }
        }

        // Load expected interval and tolerance atomically
        let expected_interval_ns = self.expected_interval_ns.load(Ordering::Relaxed);
        let tolerance_ns = self.tolerance_ns.load(Ordering::Relaxed);

        let deviation_ns = actual_interval_ns.abs_diff(expected_interval_ns);

        // Check if irregular - ONLY increment counter, NO logging/events
        if deviation_ns > tolerance_ns {
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
            expected_interval_ms: self.expected_interval_ns.load(Ordering::Relaxed) / 1_000_000,
            calibration_complete: self.calibration_complete.load(Ordering::Relaxed),
        }
    }

    /// Check calibration status and finalize if ready (called by monitoring thread)
    ///
    /// Checks if audio callback has collected enough samples, then calculates
    /// median interval and updates expected_interval_ns and tolerance_ns.
    ///
    /// **Thread Safety:** Uses mutex (NOT called from audio callback)
    fn check_calibration(&self) {
        if self.calibration_complete.load(Ordering::Relaxed) {
            return;
        }

        // Check if we have enough samples (audio callback populates this during first 100 callbacks)
        // Note: We may get 99 samples due to off-by-one with fetch_add returning old value
        if let Ok(samples) = self.calibration_samples.lock() {
            let sample_count = samples.len();
            if sample_count > 0 || self.callback_count.load(Ordering::Relaxed) < 10 {
                // Log during early startup or when we have samples
                debug!("Calibration check: {} samples collected, {} callbacks so far",
                       sample_count, self.callback_count.load(Ordering::Relaxed));
            }
            if samples.len() >= 50 {  // Require at least 50 samples for reasonable median
                // Clone samples for sorting (release lock quickly)
                let mut sorted_samples = samples.clone();
                drop(samples); // Release lock before sorting

                // Sort to find median
                sorted_samples.sort_unstable();
                let median_index = sorted_samples.len() / 2;
                let median_interval_ns = sorted_samples[median_index];

                // Calculate tolerance as 20% of median interval
                let new_tolerance_ns = (median_interval_ns as f64 * 0.20) as u64;

                // Update atomics
                self.expected_interval_ns.store(median_interval_ns, Ordering::Relaxed);
                self.tolerance_ns.store(new_tolerance_ns, Ordering::Relaxed);
                self.calibration_complete.store(true, Ordering::Relaxed);

                info!(
                    "CallbackMonitor calibration complete: measured_interval={:.2}ms (median of {} samples), tolerance={:.2}ms (20%)",
                    median_interval_ns as f64 / 1_000_000.0,
                    sorted_samples.len(),
                    new_tolerance_ns as f64 / 1_000_000.0
                );
            }
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

            // Track 15-second window for recent percentage calculation
            let mut window_start_callback_count = 0u64;
            let mut window_start_irregular_count = 0u64;
            let mut window_start_time = std::time::Instant::now();

            debug!("CallbackMonitor: Monitoring task started");

            while !shutdown_clone.load(Ordering::Relaxed) {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                let stats = monitor.stats();

                // Calibration phase: check if audio callback has collected enough samples
                if !stats.calibration_complete {
                    monitor.check_calibration();
                }

                // Check for new underruns
                if stats.underrun_count > last_underrun_count {
                    let new_underruns = stats.underrun_count - last_underrun_count;

                    // Check if audio output is expected (Playing with non-empty queue)
                    // When audio_expected is false (Paused or empty queue), underruns are expected
                    let audio_expected = monitor.audio_expected.load(Ordering::Relaxed);

                    if !audio_expected {
                        // Idle/Paused state: underruns are expected (not trying to fill buffer)
                        trace!(
                            "Audio callback underrun during idle: {} total underruns (+{} since last check)",
                            stats.underrun_count, new_underruns
                        );
                    } else {
                        // Active playback: underruns are problematic
                        warn!(
                            "Audio callback underrun detected: {} total underruns (+{} since last check)",
                            stats.underrun_count, new_underruns
                        );

                        // Emit event only during active playback
                        if let Some(state) = &monitor.state {
                            state.broadcast_event(WkmpEvent::AudioCallbackUnderrun {
                                underrun_count: stats.underrun_count,
                                timestamp: chrono::Utc::now(),
                            });
                        }
                    }

                    last_underrun_count = stats.underrun_count;
                }

                // Check for new irregular intervals (throttled to once per 5 seconds)
                if stats.irregular_intervals > last_irregular_count {
                    let new_irregular = stats.irregular_intervals - last_irregular_count;

                    // Reset 15-second window if elapsed
                    if window_start_time.elapsed().as_secs() >= 15 {
                        window_start_callback_count = stats.callback_count;
                        window_start_irregular_count = stats.irregular_intervals;
                        window_start_time = std::time::Instant::now();
                    }

                    // Log every 100 irregular intervals
                    if new_irregular >= 100 || stats.irregular_intervals.is_multiple_of(1000) {
                        // Calculate overall percentage
                        let overall_pct = if stats.callback_count > 0 {
                            (stats.irregular_intervals as f64 / stats.callback_count as f64) * 100.0
                        } else {
                            0.0
                        };

                        // Calculate recent (15s window) percentage
                        let recent_callbacks = stats.callback_count.saturating_sub(window_start_callback_count);
                        let recent_irregular = stats.irregular_intervals.saturating_sub(window_start_irregular_count);
                        let recent_pct = if recent_callbacks > 0 {
                            (recent_irregular as f64 / recent_callbacks as f64) * 100.0
                        } else {
                            0.0
                        };

                        warn!(
                            "Audio callback irregular intervals: {} total (+{} since last), {} total callbacks | Overall: {:.1}%, Recent 15s: {:.1}%",
                            stats.irregular_intervals, new_irregular, stats.callback_count, overall_pct, recent_pct
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
                if stats.callback_count > last_callback_count && stats.callback_count.is_multiple_of(3000) {
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
    pub calibration_complete: bool,
}
