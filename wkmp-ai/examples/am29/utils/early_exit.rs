//! # Early-Exit Coordination
//!
//! Thread-safe coordination for early-exit when 100% match found during parallel processing.
//!
//! ## Features
//! - **Grace period**: Allows slower editions to complete after first 100% match
//! - **Thread-safe signaling**: Atomic operations for match detection
//! - **Configurable grace**: EARLY_EXIT_GRACE_PERIOD_SECS constant
//!
//! ## Usage Pattern
//! ```ignore
//! let perfect_match_found = Arc::new(AtomicBool::new(false));
//! let perfect_match_time_ms = Arc::new(AtomicU64::new(0));
//! let start_time = Instant::now();
//!
//! // When 100% match found:
//! signal_perfect_match(&perfect_match_found, &perfect_match_time_ms, start_time);
//!
//! // In processing loops:
//! if should_exit_early(&perfect_match_found, &perfect_match_time_ms, start_time) {
//!     break;  // Grace period expired, stop processing
//! }
//! ```
//!
//! ## Related Modules
//! - `constants`: EARLY_EXIT_GRACE_PERIOD_SECS
//! - `stages::stage3`: Uses should_exit_early during assembly loop

use crate::constants::EARLY_EXIT_GRACE_PERIOD_SECS;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Instant;

// =============================================================================
// Early-Exit Coordination Functions
// =============================================================================

/// Check if early exit should occur based on 100% match found and grace period
///
/// Returns true if a 100% match was found AND the grace period has expired.
/// This allows parallel edition processing to continue for a grace period
/// after the first 100% match, giving slower-processing editions a chance
/// to complete and potentially find better matches.
///
/// # Arguments
/// * `perfect_match_found` - Atomic flag indicating if any edition found 100% match
/// * `perfect_match_time_ms` - Timestamp (ms since start) when first match found
/// * `start_time` - Reference start time for elapsed calculation
///
/// # Returns
/// * `true` - Grace period has expired, should exit processing loop
/// * `false` - Either no match found yet, or grace period still active
///
/// # Grace Period Behavior
/// - If no match found: Returns false (continue processing)
/// - If match found but grace period not expired: Returns false (continue)
/// - If match found and grace period expired: Returns true (exit early)
///
/// # Example
/// ```ignore
/// loop {
///     if should_exit_early(&perfect_match_found, &perfect_match_time_ms, start_time) {
///         info!("Early exit: grace period expired");
///         break;
///     }
///     // ... process edition ...
/// }
/// ```
///
/// # Requirements
/// Uses EARLY_EXIT_GRACE_PERIOD_SECS (20 seconds in Run 28)
pub(crate) fn should_exit_early(
    perfect_match_found: &AtomicBool,
    perfect_match_time_ms: &AtomicU64,
    start_time: Instant,
) -> bool {
    // Fast path: no match found yet
    if !perfect_match_found.load(Ordering::Relaxed) {
        return false;
    }

    // Get timestamp when match was found
    let match_time_ms = perfect_match_time_ms.load(Ordering::Relaxed);
    if match_time_ms == 0 {
        return false; // Timestamp not set yet (race condition edge case)
    }

    // Calculate time elapsed since match was found
    let current_elapsed_ms = start_time.elapsed().as_millis() as u64;
    let elapsed_since_match_ms = current_elapsed_ms - match_time_ms;

    // Exit if grace period has expired
    elapsed_since_match_ms >= EARLY_EXIT_GRACE_PERIOD_SECS * 1000
}

/// Signal that a 100% match was found (thread-safe, only sets once)
///
/// Atomically sets the perfect_match_found flag and records the timestamp.
/// Uses compare_exchange to ensure only the first caller succeeds, so the
/// timestamp represents when the FIRST 100% match was found (not subsequent ones).
///
/// # Arguments
/// * `perfect_match_found` - Atomic flag to set
/// * `perfect_match_time_ms` - Atomic timestamp to record (ms since start_time)
/// * `start_time` - Reference start time for elapsed calculation
///
/// # Thread Safety
/// Multiple threads may call this simultaneously, but only the first caller
/// will set the flag and timestamp. Subsequent calls are no-ops.
///
/// # Example
/// ```ignore
/// let result = test_edition(...);
/// if result.percentage >= 100.0 {
///     signal_perfect_match(&perfect_match_found, &perfect_match_time_ms, start_time);
/// }
/// ```
pub(crate) fn signal_perfect_match(
    perfect_match_found: &AtomicBool,
    perfect_match_time_ms: &AtomicU64,
    start_time: Instant,
) {
    // Only set if not already set (compare_exchange ensures atomicity)
    // Success: previous value was false, we set it to true
    // Failure: another thread already set it to true
    if perfect_match_found
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::Relaxed)
        .is_ok()
    {
        // We won the race - record the timestamp
        let elapsed_ms = start_time.elapsed().as_millis() as u64;
        perfect_match_time_ms.store(elapsed_ms, Ordering::SeqCst);
    }
}
