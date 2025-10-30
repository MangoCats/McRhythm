//! Import operation results and errors
//!
//! **[AIA-ERR-010]** Error severity categorization
//! **[AIA-ERR-020]** Error reporting via SSE and completion summary

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// **[AIA-ERR-010]** Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ErrorSeverity {
    /// Warning: File skipped, import continues
    Warning,
    /// Skip: File cannot be processed, import continues
    Skip,
    /// Critical: Import cannot continue
    Critical,
}

/// **[AIA-ERR-020]** Import error details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportError {
    /// File path that caused the error
    pub file_path: String,

    /// Error code (e.g., "DECODE_ERROR", "NETWORK_ERROR")
    pub error_code: String,

    /// Human-readable error message
    pub error_message: String,

    /// Error severity
    pub severity: ErrorSeverity,

    /// When the error occurred
    pub occurred_at: DateTime<Utc>,
}

impl ImportError {
    /// Create new warning
    pub fn warning(file_path: String, error_code: String, error_message: String) -> Self {
        Self {
            file_path,
            error_code,
            error_message,
            severity: ErrorSeverity::Warning,
            occurred_at: Utc::now(),
        }
    }

    /// Create new skip error
    pub fn skip(file_path: String, error_code: String, error_message: String) -> Self {
        Self {
            file_path,
            error_code,
            error_message,
            severity: ErrorSeverity::Skip,
            occurred_at: Utc::now(),
        }
    }

    /// Create new critical error
    pub fn critical(file_path: String, error_code: String, error_message: String) -> Self {
        Self {
            file_path,
            error_code,
            error_message,
            severity: ErrorSeverity::Critical,
            occurred_at: Utc::now(),
        }
    }
}

/// Import completion result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    /// Total files discovered
    pub total_files: usize,

    /// Files successfully processed
    pub files_processed: usize,

    /// Files skipped due to errors
    pub files_skipped: usize,

    /// Errors encountered (categorized by severity)
    pub errors: Vec<ImportError>,

    /// Duration in seconds
    pub duration_seconds: u64,
}

impl ImportResult {
    /// Create new empty result
    pub fn new() -> Self {
        Self {
            total_files: 0,
            files_processed: 0,
            files_skipped: 0,
            errors: Vec::new(),
            duration_seconds: 0,
        }
    }

    /// Count errors by severity
    pub fn count_by_severity(&self, severity: ErrorSeverity) -> usize {
        self.errors.iter().filter(|e| e.severity == severity).count()
    }
}

impl Default for ImportResult {
    fn default() -> Self {
        Self::new()
    }
}
