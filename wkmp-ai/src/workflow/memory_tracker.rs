//! Memory Usage Tracking
//!
//! Monitors large audio buffer allocations to detect potential memory issues.
//! Provides visibility into RAM usage without changing operation.

use std::sync::atomic::{AtomicUsize, Ordering};
use tracing::{info, warn};

/// Global atomic counter for total estimated audio buffer memory (bytes)
///
/// Note: This is an estimate based on logged allocations. Vec clones and
/// other copies may cause actual usage to differ. Use for monitoring trends,
/// not precise measurement.
static TOTAL_AUDIO_BUFFER_BYTES: AtomicUsize = AtomicUsize::new(0);

/// Warning threshold: 20GB
const WARNING_THRESHOLD_BYTES: usize = 20_000_000_000;

/// Log allocation of an audio buffer and update global estimate
///
/// # Arguments
/// * `samples` - Number of f32 samples allocated
/// * `label` - Description for logging (e.g., "album_matcher decode", "boundary_detector cache")
///
/// # Returns
/// * Allocation ID (sample count) for later deallocation tracking
pub fn track_allocation(samples: usize, label: &str) -> usize {
    const BYTES_PER_SAMPLE: usize = 4; // f32
    let bytes = samples * BYTES_PER_SAMPLE;

    let previous = TOTAL_AUDIO_BUFFER_BYTES.fetch_add(bytes, Ordering::SeqCst);
    let total = previous + bytes;

    info!(
        "MEMORY: +{:.1} MB [{}] → Total: {:.1} MB ({:.2} GB) | {} samples",
        bytes as f64 / 1_000_000.0,
        label,
        total as f64 / 1_000_000.0,
        total as f64 / 1_000_000_000.0,
        samples
    );

    if total > WARNING_THRESHOLD_BYTES {
        warn!(
            "⚠️  MEMORY WARNING: Total audio buffers ({:.2} GB) exceeds 20 GB threshold! Current allocation: '{}'. Operation continues but may cause memory pressure.",
            total as f64 / 1_000_000_000.0,
            label
        );
    }

    samples // Return ID for deallocation
}

/// Log deallocation of an audio buffer and update global estimate
///
/// # Arguments
/// * `samples` - Number of f32 samples deallocated (from track_allocation return value)
/// * `label` - Description for logging
pub fn track_deallocation(samples: usize, label: &str) {
    const BYTES_PER_SAMPLE: usize = 4; // f32
    let bytes = samples * BYTES_PER_SAMPLE;

    let previous = TOTAL_AUDIO_BUFFER_BYTES.fetch_sub(bytes, Ordering::SeqCst);
    let total = previous.saturating_sub(bytes);

    info!(
        "MEMORY: -{:.1} MB [{}] → Total: {:.1} MB ({:.2} GB)",
        bytes as f64 / 1_000_000.0,
        label,
        total as f64 / 1_000_000.0,
        total as f64 / 1_000_000_000.0
    );
}

/// Get current total audio buffer memory estimate (bytes)
pub fn current_total_bytes() -> usize {
    TOTAL_AUDIO_BUFFER_BYTES.load(Ordering::SeqCst)
}

/// Get current total audio buffer memory estimate (GB)
pub fn current_total_gb() -> f64 {
    current_total_bytes() as f64 / 1_000_000_000.0
}

/// Log current memory status (useful for checkpoints)
pub fn log_status(context: &str) {
    let total = current_total_bytes();
    info!(
        "MEMORY STATUS [{}]: {:.1} MB ({:.2} GB) audio buffers tracked",
        context,
        total as f64 / 1_000_000.0,
        total as f64 / 1_000_000_000.0
    );
}

/// RAII guard for automatic memory tracking
///
/// Tracks allocation on creation, automatically tracks deallocation on drop.
/// This prevents forgetting to call track_deallocation().
///
/// # Example
/// ```
/// let samples = vec![0.0f32; 1_000_000];
/// let _guard = MemoryGuard::new(samples.len(), "my_buffer");
/// // ... use samples ...
/// // guard automatically tracks deallocation when dropped
/// ```
#[derive(Debug)]
pub struct MemoryGuard {
    samples: usize,
    label: String,
}

impl MemoryGuard {
    /// Create new memory guard and track allocation
    ///
    /// # Arguments
    /// * `samples` - Number of f32 samples allocated
    /// * `label` - Description for logging
    pub fn new(samples: usize, label: impl Into<String>) -> Self {
        let label = label.into();
        track_allocation(samples, &label);
        Self { samples, label }
    }
}

impl Drop for MemoryGuard {
    fn drop(&mut self) {
        track_deallocation(self.samples, &self.label);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_tracking() {
        // Reset counter
        TOTAL_AUDIO_BUFFER_BYTES.store(0, Ordering::SeqCst);

        let id1 = track_allocation(1_000_000, "test1"); // 4 MB
        assert_eq!(current_total_bytes(), 4_000_000);

        let id2 = track_allocation(2_000_000, "test2"); // 8 MB
        assert_eq!(current_total_bytes(), 12_000_000);

        track_deallocation(id2, "test2");
        assert_eq!(current_total_bytes(), 4_000_000);

        track_deallocation(id1, "test1");
        assert_eq!(current_total_bytes(), 0);
    }

    #[test]
    fn test_warning_threshold() {
        TOTAL_AUDIO_BUFFER_BYTES.store(0, Ordering::SeqCst);

        // Allocate just under threshold
        let samples_under = 4_900_000_000; // 19.6 GB
        track_allocation(samples_under, "under_threshold");
        assert!(current_total_bytes() < WARNING_THRESHOLD_BYTES);

        // Allocate over threshold - should warn (check logs)
        let samples_over = 200_000_000; // 0.8 GB more = 20.4 GB total
        track_allocation(samples_over, "over_threshold");
        assert!(current_total_bytes() > WARNING_THRESHOLD_BYTES);
    }
}
