//! Memory usage monitoring and management
//!
//! **[PLAN029 Task 2.2]** Memory monitoring with automatic cleanup
//!
//! # Purpose
//! - Monitor process memory usage in real-time
//! - Track high water mark for diagnostics
//! - Warn when memory exceeds thresholds
//! - Trigger automatic cleanup on critical usage
//! - Prevent memory-related performance degradation

use parking_lot::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use sysinfo::{Pid, System};

/// Memory monitoring service with automatic threshold management
///
/// **[PLAN029 Task 2.2]** Monitors process memory and triggers cleanup
///
/// Tracks memory usage and provides three-tier alerting:
/// - Normal: Below warning threshold (default 500MB)
/// - Warning: Above threshold but below critical (500MB-1GB)
/// - Critical: Above 2x threshold (>1GB), triggers automatic cleanup
pub struct MemoryMonitor {
    /// System information provider
    system: Arc<RwLock<System>>,
    /// Current process ID
    pid: Pid,
    /// Highest memory usage observed (bytes)
    high_water_mark: Arc<AtomicU64>,
    /// Warning threshold in bytes (default 500MB)
    warning_threshold: u64,
}

impl MemoryMonitor {
    /// Create new memory monitor with default 500MB warning threshold
    ///
    /// **[PLAN029]** Initialize monitoring service
    ///
    /// # Default Thresholds
    /// - Warning: 500MB (triggers logging)
    /// - Critical: 1GB (2x warning, triggers cleanup)
    pub fn new() -> Self {
        let pid = sysinfo::get_current_pid().expect("Failed to get current PID");

        Self {
            system: Arc::new(RwLock::new(System::new())),
            pid,
            high_water_mark: Arc::new(AtomicU64::new(0)),
            warning_threshold: 500_000_000, // 500MB in bytes
        }
    }

    /// Create monitor with custom warning threshold
    ///
    /// # Arguments
    /// * `warning_threshold_bytes` - Warning threshold in bytes
    pub fn with_threshold(warning_threshold_bytes: u64) -> Self {
        let pid = sysinfo::get_current_pid().expect("Failed to get current PID");

        Self {
            system: Arc::new(RwLock::new(System::new())),
            pid,
            high_water_mark: Arc::new(AtomicU64::new(0)),
            warning_threshold: warning_threshold_bytes,
        }
    }

    /// Check current memory usage and return status
    ///
    /// **[PLAN029]** Memory status classification
    ///
    /// # Returns
    /// - `MemoryStatus::Normal(bytes)` - Below warning threshold
    /// - `MemoryStatus::Warning(bytes)` - Above warning, below critical
    /// - `MemoryStatus::Critical(bytes)` - Above 2x warning threshold
    /// - `MemoryStatus::Unknown` - Unable to read process memory
    pub fn check_memory(&self) -> MemoryStatus {
        let mut sys = self.system.write();

        // Refresh process information
        sys.refresh_all();

        if let Some(process) = sys.process(self.pid) {
            // sysinfo 0.32 returns memory in bytes
            let memory_bytes = process.memory();

            // Update high water mark
            self.high_water_mark.fetch_max(memory_bytes, Ordering::Relaxed);

            // Classify memory status
            let critical_threshold = self.warning_threshold * 2;

            if memory_bytes > critical_threshold {
                tracing::error!(
                    memory_mb = memory_bytes / 1_000_000,
                    threshold_mb = critical_threshold / 1_000_000,
                    "CRITICAL: Memory usage {}MB exceeds 2x threshold ({}MB)",
                    memory_bytes / 1_000_000,
                    critical_threshold / 1_000_000
                );
                return MemoryStatus::Critical(memory_bytes);
            } else if memory_bytes > self.warning_threshold {
                tracing::warn!(
                    memory_mb = memory_bytes / 1_000_000,
                    threshold_mb = self.warning_threshold / 1_000_000,
                    "High memory usage: {}MB (threshold {}MB)",
                    memory_bytes / 1_000_000,
                    self.warning_threshold / 1_000_000
                );
                return MemoryStatus::Warning(memory_bytes);
            }

            MemoryStatus::Normal(memory_bytes)
        } else {
            tracing::warn!("Unable to read process memory for PID {}", self.pid);
            MemoryStatus::Unknown
        }
    }

    /// Get high water mark (maximum memory usage observed)
    ///
    /// Returns the highest memory usage in bytes since monitor creation
    pub fn get_high_water_mark(&self) -> u64 {
        self.high_water_mark.load(Ordering::Relaxed)
    }

    /// Get current memory usage in bytes
    ///
    /// Returns None if unable to read process memory
    pub fn get_current_usage(&self) -> Option<u64> {
        match self.check_memory() {
            MemoryStatus::Normal(bytes)
            | MemoryStatus::Warning(bytes)
            | MemoryStatus::Critical(bytes) => Some(bytes),
            MemoryStatus::Unknown => None,
        }
    }

    /// Log current memory statistics
    ///
    /// **[PLAN029]** Memory usage reporting
    pub fn log_stats(&self) {
        let current = self.get_current_usage().unwrap_or(0);
        let high_water = self.get_high_water_mark();

        tracing::info!(
            current_mb = current / 1_000_000,
            high_water_mb = high_water / 1_000_000,
            threshold_mb = self.warning_threshold / 1_000_000,
            "Memory statistics - Current: {}MB, High water: {}MB, Threshold: {}MB",
            current / 1_000_000,
            high_water / 1_000_000,
            self.warning_threshold / 1_000_000
        );
    }

    /// Background monitoring task
    ///
    /// **[PLAN029]** Continuous memory monitoring
    ///
    /// Checks memory every 30 seconds and logs status. On critical
    /// memory usage, triggers cleanup and pauses if still critical.
    ///
    /// # Usage
    /// ```rust,no_run
    /// let monitor = Arc::new(MemoryMonitor::new());
    /// tokio::spawn(monitor.clone().monitor_task());
    /// ```
    pub async fn monitor_task(self: Arc<Self>) {
        tracing::info!("Starting memory monitor task (check interval: 30s)");
        let mut interval = tokio::time::interval(Duration::from_secs(30));

        loop {
            interval.tick().await;

            match self.check_memory() {
                MemoryStatus::Critical(bytes) => {
                    tracing::error!(
                        "Attempting memory recovery, current: {}MB",
                        bytes / 1_000_000
                    );

                    // Attempt cleanup
                    self.clear_caches().await;

                    // Re-check after cleanup
                    tokio::time::sleep(Duration::from_secs(5)).await;

                    if let MemoryStatus::Critical(bytes_after) = self.check_memory() {
                        tracing::error!(
                            "Memory still critical after cleanup ({}MB), pausing operations for 30s",
                            bytes_after / 1_000_000
                        );
                        tokio::time::sleep(Duration::from_secs(30)).await;
                    } else {
                        tracing::info!("Memory recovered after cleanup");
                    }
                }
                MemoryStatus::Warning(bytes) => {
                    tracing::info!(
                        "Memory check: {}MB (warning level)",
                        bytes / 1_000_000
                    );
                }
                MemoryStatus::Normal(bytes) => {
                    tracing::debug!("Memory check: {}MB (normal)", bytes / 1_000_000);
                }
                MemoryStatus::Unknown => {
                    tracing::warn!("Unable to check memory usage");
                }
            }
        }
    }

    /// Trigger cache clearing operations
    ///
    /// **[PLAN029]** Memory recovery mechanism
    ///
    /// Currently a placeholder for future cache clearing logic.
    /// Will be integrated with WorkflowOrchestrator cache management.
    async fn clear_caches(&self) {
        tracing::info!("Clearing caches to free memory");
        // Future: Integrate with actual cache clearing mechanisms
        // For now, this serves as a hook point for integration
    }
}

impl Default for MemoryMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Memory usage status classification
///
/// **[PLAN029]** Three-tier alerting system
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryStatus {
    /// Memory usage is below warning threshold (normal operation)
    Normal(u64),
    /// Memory usage is above warning but below critical (elevated)
    Warning(u64),
    /// Memory usage is above 2x warning threshold (requires action)
    Critical(u64),
    /// Unable to determine memory usage
    Unknown,
}

impl MemoryStatus {
    /// Get memory usage in bytes if available
    pub fn bytes(&self) -> Option<u64> {
        match self {
            MemoryStatus::Normal(b)
            | MemoryStatus::Warning(b)
            | MemoryStatus::Critical(b) => Some(*b),
            MemoryStatus::Unknown => None,
        }
    }

    /// Check if memory usage is critical
    pub fn is_critical(&self) -> bool {
        matches!(self, MemoryStatus::Critical(_))
    }

    /// Check if memory usage is elevated (warning or critical)
    pub fn is_elevated(&self) -> bool {
        matches!(self, MemoryStatus::Warning(_) | MemoryStatus::Critical(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_monitor_creation() {
        let monitor = MemoryMonitor::new();

        // Should be able to check memory
        let status = monitor.check_memory();
        assert!(matches!(
            status,
            MemoryStatus::Normal(_) | MemoryStatus::Warning(_) | MemoryStatus::Critical(_)
        ));
    }

    #[test]
    fn test_custom_threshold() {
        let monitor = MemoryMonitor::with_threshold(100_000_000); // 100MB in bytes
        assert_eq!(monitor.warning_threshold, 100_000_000);
    }

    #[test]
    fn test_high_water_mark() {
        let monitor = MemoryMonitor::new();

        // Check memory to populate high water mark
        monitor.check_memory();

        // High water mark should be set
        let hwm = monitor.get_high_water_mark();
        assert!(hwm > 0, "High water mark should be greater than 0");
    }

    #[test]
    fn test_memory_status_helpers() {
        let normal = MemoryStatus::Normal(100_000_000);
        assert!(!normal.is_critical());
        assert!(!normal.is_elevated());
        assert_eq!(normal.bytes(), Some(100_000_000));

        let warning = MemoryStatus::Warning(600_000_000);
        assert!(!warning.is_critical());
        assert!(warning.is_elevated());

        let critical = MemoryStatus::Critical(1_200_000_000);
        assert!(critical.is_critical());
        assert!(critical.is_elevated());

        let unknown = MemoryStatus::Unknown;
        assert!(!unknown.is_critical());
        assert_eq!(unknown.bytes(), None);
    }

    #[tokio::test]
    async fn test_log_stats() {
        let monitor = MemoryMonitor::new();

        // Should not panic
        monitor.log_stats();
    }
}
