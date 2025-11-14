//! Log Capture Utilities for Testing
//!
//! Provides tracing log capture and assertion utilities

use std::sync::{Arc, Mutex};
use tracing::Level;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

/// Captured log record
#[derive(Debug, Clone)]
pub struct LogRecord {
    pub level: Level,
    pub target: String,
    pub message: String,
}

/// Log capture layer for testing
#[derive(Clone)]
pub struct LogCapture {
    records: Arc<Mutex<Vec<LogRecord>>>,
}

impl LogCapture {
    /// Create new log capture
    pub fn new() -> Self {
        Self {
            records: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Get all captured log records
    pub fn records(&self) -> Vec<LogRecord> {
        self.records.lock().unwrap().clone()
    }

    /// Clear captured records
    pub fn clear(&self) {
        self.records.lock().unwrap().clear();
    }

    /// Check if any log message matches pattern
    pub fn contains(&self, pattern: &str) -> bool {
        self.records()
            .iter()
            .any(|r| r.message.contains(pattern))
    }

    /// Count log messages matching pattern
    pub fn count_matching(&self, pattern: &str) -> usize {
        self.records()
            .iter()
            .filter(|r| r.message.contains(pattern))
            .count()
    }

    /// Get all messages matching pattern
    pub fn matching(&self, pattern: &str) -> Vec<String> {
        self.records()
            .iter()
            .filter(|r| r.message.contains(pattern))
            .map(|r| r.message.clone())
            .collect()
    }

    /// Assert no logs match pattern
    pub fn assert_no_match(&self, pattern: &str) {
        let matches = self.matching(pattern);
        assert!(
            matches.is_empty(),
            "Expected no logs matching '{}', but found {} matches:\n{}",
            pattern,
            matches.len(),
            matches.join("\n")
        );
    }

    /// Assert at least one log matches pattern
    pub fn assert_contains(&self, pattern: &str) {
        assert!(
            self.contains(pattern),
            "Expected log matching '{}', but none found. All logs:\n{}",
            pattern,
            self.records()
                .iter()
                .map(|r| r.message.as_str())
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
}

impl Default for LogCapture {
    fn default() -> Self {
        Self::new()
    }
}

// Implement tracing Layer trait
impl<S> tracing_subscriber::Layer<S> for LogCapture
where
    S: tracing::Subscriber,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        use tracing::field::Visit;

        struct MessageVisitor {
            message: String,
        }

        impl Visit for MessageVisitor {
            fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
                if field.name() == "message" {
                    self.message = format!("{:?}", value);
                    // Remove surrounding quotes
                    if self.message.starts_with('"') && self.message.ends_with('"') {
                        self.message = self.message[1..self.message.len() - 1].to_string();
                    }
                }
            }
        }

        let mut visitor = MessageVisitor {
            message: String::new(),
        };
        event.record(&mut visitor);

        let record = LogRecord {
            level: *event.metadata().level(),
            target: event.metadata().target().to_string(),
            message: visitor.message,
        };

        self.records.lock().unwrap().push(record);
    }
}

/// Initialize test logging with log capture
///
/// Returns LogCapture instance that can be used to assert on logs
pub fn init_test_logging() -> LogCapture {
    let capture = LogCapture::new();

    // Try to initialize subscriber (may fail if already initialized in other tests)
    let _ = tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "wkmp_ai=debug".into()),
        )
        .with(capture.clone())
        .try_init();

    capture
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing::{debug, error, info, warn};

    #[test]
    fn test_log_capture_basic() {
        let capture = LogCapture::new();

        // Manually create log records for testing
        capture.records.lock().unwrap().push(LogRecord {
            level: Level::INFO,
            target: "test".to_string(),
            message: "Test message".to_string(),
        });

        assert_eq!(capture.records().len(), 1);
        assert!(capture.contains("Test message"));
    }

    #[test]
    fn test_log_capture_pattern_matching() {
        let capture = LogCapture::new();

        capture.records.lock().unwrap().push(LogRecord {
            level: Level::INFO,
            target: "test".to_string(),
            message: "Extracting metadata from 100 files".to_string(),
        });

        capture.records.lock().unwrap().push(LogRecord {
            level: Level::DEBUG,
            target: "test".to_string(),
            message: "Processing file 1 of 100".to_string(),
        });

        assert!(capture.contains("Extracting metadata"));
        assert_eq!(capture.count_matching("metadata"), 1);
        assert_eq!(capture.count_matching("file"), 1);

        let matches = capture.matching("from");
        assert_eq!(matches.len(), 1);
    }

    #[test]
    #[should_panic(expected = "Expected no logs matching")]
    fn test_assert_no_match_fails() {
        let capture = LogCapture::new();

        capture.records.lock().unwrap().push(LogRecord {
            level: Level::ERROR,
            target: "test".to_string(),
            message: "Batch extraction occurred".to_string(),
        });

        capture.assert_no_match("Batch extraction");
    }

    #[test]
    #[should_panic(expected = "Expected log matching")]
    fn test_assert_contains_fails() {
        let capture = LogCapture::new();
        capture.assert_contains("nonexistent pattern");
    }
}
