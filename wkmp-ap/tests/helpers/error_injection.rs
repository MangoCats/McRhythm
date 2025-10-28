//! Error injection test utilities for Phase 7 Error Handling
//!
//! Provides helpers for testing error handling behavior including:
//! - File system error injection
//! - Codec failure simulation
//! - Panic injection and recovery testing
//! - Buffer underrun simulation
//! - Position drift injection
//!
//! **[PLAN001 Phase 7]** Error injection test framework

use std::path::{Path, PathBuf};
use std::fs;
use std::io::Write;
use tempfile::TempDir;

/// Result type for error injection operations
pub type InjectionResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Test file generator for error injection scenarios
pub struct ErrorInjectionBuilder {
    temp_dir: TempDir,
}

impl ErrorInjectionBuilder {
    /// Create a new error injection builder with temporary directory
    pub fn new() -> InjectionResult<Self> {
        Ok(Self {
            temp_dir: TempDir::new()?,
        })
    }

    /// Get the path to the temporary directory
    pub fn temp_path(&self) -> &Path {
        self.temp_dir.path()
    }

    /// Create a file path in the temporary directory
    pub fn file_path(&self, filename: &str) -> PathBuf {
        self.temp_dir.path().join(filename)
    }

    /// **[REQ-AP-ERR-010]** Create a non-existent file path for testing file read errors
    pub fn nonexistent_file(&self) -> PathBuf {
        self.file_path("nonexistent_file.flac")
    }

    /// **[REQ-AP-ERR-010]** Create an unreadable file (permission denied)
    #[cfg(unix)]
    pub fn unreadable_file(&self) -> InjectionResult<PathBuf> {
        use std::os::unix::fs::PermissionsExt;

        let path = self.file_path("unreadable.flac");
        let mut file = fs::File::create(&path)?;
        file.write_all(b"dummy content")?;
        drop(file);

        // Set permissions to write-only (no read permission)
        let mut perms = fs::metadata(&path)?.permissions();
        perms.set_mode(0o200);
        fs::set_permissions(&path, perms)?;

        Ok(path)
    }

    /// **[REQ-AP-ERR-011]** Create a file with invalid/corrupted header (unsupported codec)
    pub fn corrupted_audio_file(&self) -> InjectionResult<PathBuf> {
        let path = self.file_path("corrupted.flac");
        let mut file = fs::File::create(&path)?;

        // Write FLAC magic bytes followed by garbage
        file.write_all(b"fLaC")?;
        file.write_all(&[0xFF; 1024])?; // Garbage data

        Ok(path)
    }

    /// **[REQ-AP-ERR-011]** Create a file with unsupported audio format
    pub fn unsupported_format_file(&self) -> InjectionResult<PathBuf> {
        let path = self.file_path("unsupported.xyz");
        let mut file = fs::File::create(&path)?;

        // Write arbitrary binary data with unknown format
        file.write_all(b"UNKNOWN_FORMAT_HEADER")?;
        file.write_all(&vec![0xAB; 4096])?;

        Ok(path)
    }

    /// **[REQ-AP-ERR-012]** Create a truncated audio file (partial decode scenario)
    pub fn truncated_audio_file(&self, percentage: u8) -> InjectionResult<PathBuf> {
        let path = self.file_path(&format!("truncated_{}pct.wav", percentage));

        // Generate a valid WAV file first using audio_generator
        let full_path = self.file_path("temp_full.wav");
        super::audio_generator::generate_sine_wav(&full_path, 5000, 440.0, 0.5)?;

        // Read full file and truncate to specified percentage
        let full_data = fs::read(&full_path)?;
        let truncated_size = (full_data.len() as f32 * (percentage as f32 / 100.0)) as usize;
        let truncated_data = &full_data[..truncated_size];

        fs::write(&path, truncated_data)?;
        fs::remove_file(full_path)?;

        Ok(path)
    }

    /// **[REQ-AP-ERR-020]** Create test files with specific decode timing for buffer underrun testing
    pub fn slow_decode_file(&self) -> InjectionResult<PathBuf> {
        // Create a very large audio file that decodes slowly
        let path = self.file_path("slow_decode.wav");
        super::audio_generator::generate_sine_wav(&path, 30000, 440.0, 0.5)?;
        Ok(path)
    }

    /// **[REQ-AP-ERR-050/051]** Create audio file with standard sample rate for resampling tests
    ///
    /// Note: Generates 44.1kHz file. Resampling tests will use different target rates.
    pub fn audio_file_for_resampling(&self) -> InjectionResult<PathBuf> {
        let path = self.file_path("resampling_test.wav");
        super::audio_generator::generate_sine_wav(&path, 2000, 440.0, 0.5)?;
        Ok(path)
    }

    /// **[REQ-AP-ERR-071]** Create many small files to test file handle exhaustion
    pub fn many_files(&self, count: usize) -> InjectionResult<Vec<PathBuf>> {
        let mut paths = Vec::new();
        for i in 0..count {
            let path = self.file_path(&format!("file_{:04}.wav", i));
            super::audio_generator::generate_sine_wav(&path, 100, 440.0, 0.5)?;
            paths.push(path);
        }
        Ok(paths)
    }
}

/// Panic injection helpers for testing panic recovery
pub mod panic_injection {
    /// **[REQ-AP-ERR-013]** Execute function and catch panic
    ///
    /// Returns Ok(T) if function succeeds, Err if panic occurs
    pub fn catch_panic<F, T>(f: F) -> Result<T, String>
    where
        F: FnOnce() -> T + std::panic::UnwindSafe,
    {
        match std::panic::catch_unwind(f) {
            Ok(result) => Ok(result),
            Err(panic_info) => {
                let panic_msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = panic_info.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "Unknown panic".to_string()
                };
                Err(panic_msg)
            }
        }
    }

    /// **[REQ-AP-ERR-013]** Trigger an intentional panic for testing recovery
    #[allow(unreachable_code)]
    pub fn trigger_panic(message: &str) -> ! {
        panic!("{}", message);
        #[allow(clippy::empty_loop)]
        loop {} // Never reached, satisfies ! return type
    }
}

/// Event verification helpers for error handling tests
pub mod event_verification {
    use wkmp_common::WkmpEvent;
    use tokio::sync::broadcast;
    use tokio::time::{timeout, Duration};

    /// **[REQ-AP-EVENT-ERR-010]** Verify that an expected error event is emitted
    ///
    /// Returns the event if found within timeout, None otherwise
    pub async fn expect_error_event<F>(
        mut rx: broadcast::Receiver<WkmpEvent>,
        timeout_ms: u64,
        predicate: F,
    ) -> Option<WkmpEvent>
    where
        F: Fn(&WkmpEvent) -> bool,
    {
        let result = timeout(Duration::from_millis(timeout_ms), async {
            loop {
                match rx.recv().await {
                    Ok(event) => {
                        if predicate(&event) {
                            return Some(event);
                        }
                    }
                    Err(_) => return None,
                }
            }
        })
        .await;

        result.unwrap_or(None)
    }

    /// **[REQ-AP-EVENT-ERR-010]** Verify PassageDecodeFailed event
    pub async fn expect_decode_failed(
        rx: broadcast::Receiver<WkmpEvent>,
        timeout_ms: u64,
    ) -> Option<WkmpEvent> {
        expect_error_event(rx, timeout_ms, |event| {
            matches!(event, WkmpEvent::PassageDecodeFailed { .. })
        })
        .await
    }

    /// **[REQ-AP-EVENT-ERR-010]** Verify PassageUnsupportedCodec event
    pub async fn expect_unsupported_codec(
        rx: broadcast::Receiver<WkmpEvent>,
        timeout_ms: u64,
    ) -> Option<WkmpEvent> {
        expect_error_event(rx, timeout_ms, |event| {
            matches!(event, WkmpEvent::PassageUnsupportedCodec { .. })
        })
        .await
    }

    /// **[REQ-AP-EVENT-ERR-010]** Verify PassagePartialDecode event
    pub async fn expect_partial_decode(
        rx: broadcast::Receiver<WkmpEvent>,
        timeout_ms: u64,
    ) -> Option<WkmpEvent> {
        expect_error_event(rx, timeout_ms, |event| {
            matches!(event, WkmpEvent::PassagePartialDecode { .. })
        })
        .await
    }

    /// **[REQ-AP-EVENT-ERR-010]** Verify BufferUnderrun event
    pub async fn expect_buffer_underrun(
        rx: broadcast::Receiver<WkmpEvent>,
        timeout_ms: u64,
    ) -> Option<WkmpEvent> {
        expect_error_event(rx, timeout_ms, |event| {
            matches!(event, WkmpEvent::BufferUnderrun { .. })
        })
        .await
    }

    /// **[REQ-AP-EVENT-ERR-010]** Verify ResamplingFailed event
    pub async fn expect_resampling_failed(
        rx: broadcast::Receiver<WkmpEvent>,
        timeout_ms: u64,
    ) -> Option<WkmpEvent> {
        expect_error_event(rx, timeout_ms, |event| {
            matches!(event, WkmpEvent::ResamplingFailed { .. })
        })
        .await
    }

    /// **[REQ-AP-EVENT-ERR-010]** Verify FileHandleExhaustion event
    pub async fn expect_file_handle_exhaustion(
        rx: broadcast::Receiver<WkmpEvent>,
        timeout_ms: u64,
    ) -> Option<WkmpEvent> {
        expect_error_event(rx, timeout_ms, |event| {
            matches!(event, WkmpEvent::FileHandleExhaustion { .. })
        })
        .await
    }

    /// **[REQ-AP-EVENT-ERR-010]** Verify PositionDriftWarning event
    pub async fn expect_position_drift(
        rx: broadcast::Receiver<WkmpEvent>,
        timeout_ms: u64,
    ) -> Option<WkmpEvent> {
        expect_error_event(rx, timeout_ms, |event| {
            matches!(event, WkmpEvent::PositionDriftWarning { .. })
        })
        .await
    }
}

/// Logging verification helpers
pub mod logging_verification {
    use tracing::Level;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    use std::sync::{Arc, Mutex};

    /// Captured log record
    #[derive(Debug, Clone)]
    pub struct LogRecord {
        pub level: Level,
        pub message: String,
        pub target: String,
    }

    /// Log capture layer for testing
    pub struct LogCapture {
        records: Arc<Mutex<Vec<LogRecord>>>,
    }

    impl LogCapture {
        /// Create a new log capture
        pub fn new() -> Self {
            Self {
                records: Arc::new(Mutex::new(Vec::new())),
            }
        }

        /// Get captured log records
        pub fn records(&self) -> Vec<LogRecord> {
            self.records.lock().unwrap().clone()
        }

        /// **[REQ-AP-LOG-ERR-010]** Find logs at specific severity level
        pub fn find_level(&self, level: Level) -> Vec<LogRecord> {
            self.records()
                .into_iter()
                .filter(|r| r.level == level)
                .collect()
        }

        /// **[REQ-AP-LOG-ERR-010]** Find error-level logs
        pub fn find_errors(&self) -> Vec<LogRecord> {
            self.find_level(Level::ERROR)
        }

        /// **[REQ-AP-LOG-ERR-010]** Find warning-level logs
        pub fn find_warnings(&self) -> Vec<LogRecord> {
            self.find_level(Level::WARN)
        }

        /// **[REQ-AP-LOG-ERR-020]** Find logs containing specific text
        pub fn find_containing(&self, text: &str) -> Vec<LogRecord> {
            self.records()
                .into_iter()
                .filter(|r| r.message.contains(text))
                .collect()
        }
    }

    /// **[REQ-AP-LOG-ERR-010/020]** Initialize test logging with capture
    pub fn init_test_logging() -> LogCapture {
        let capture = LogCapture::new();

        // Note: Actual tracing layer implementation would go here
        // For now, this is a placeholder structure for the test framework

        capture
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_injection_builder_creation() {
        let builder = ErrorInjectionBuilder::new().unwrap();
        assert!(builder.temp_path().exists());
    }

    #[test]
    fn test_nonexistent_file_path() {
        let builder = ErrorInjectionBuilder::new().unwrap();
        let path = builder.nonexistent_file();
        assert!(!path.exists());
    }

    #[test]
    #[cfg(unix)]
    fn test_unreadable_file_creation() {
        let builder = ErrorInjectionBuilder::new().unwrap();
        let path = builder.unreadable_file().unwrap();
        assert!(path.exists());

        // Verify file is actually unreadable
        let result = std::fs::read(&path);
        assert!(result.is_err());
    }

    #[test]
    fn test_corrupted_file_creation() {
        let builder = ErrorInjectionBuilder::new().unwrap();
        let path = builder.corrupted_audio_file().unwrap();
        assert!(path.exists());

        // Verify file has FLAC header but corrupted content
        let content = std::fs::read(&path).unwrap();
        assert_eq!(&content[0..4], b"fLaC");
    }

    #[test]
    fn test_panic_catch() {
        let result = panic_injection::catch_panic(|| {
            panic!("Test panic");
        });

        assert!(result.is_err());
        let err_msg = result.unwrap_err();
        assert!(err_msg.contains("Test panic"));
    }

    #[test]
    fn test_panic_no_panic() {
        let result = panic_injection::catch_panic(|| {
            42
        });

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }
}
