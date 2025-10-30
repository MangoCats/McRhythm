//! Error types for wkmp-ap
//!
//! Defines module-specific error types using thiserror for clear error propagation.
//!
//! **Traceability:** CO-162 (Custom error types using thiserror)

use thiserror::Error;

/// Main error type for wkmp-ap module
#[derive(Error, Debug)]
pub enum Error {
    /// Configuration file loading errors
    #[error("Configuration error: {0}")]
    Config(String),

    /// Database connection or query errors
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    /// HTTP server errors
    #[error("HTTP server error: {0}")]
    Http(String),

    /// Audio decoding errors (general)
    #[error("Audio decode error: {0}")]
    Decode(String),

    /// File read error during decode
    /// **[REQ-AP-ERR-010]** Decode errors skip passage, continue with next
    #[error("File read error: {path}: {source}")]
    FileReadError {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Unsupported codec
    /// **[REQ-AP-ERR-011]** Unsupported codecs marked to prevent re-queue
    #[error("Unsupported codec: {path}: {codec}")]
    UnsupportedCodec {
        path: std::path::PathBuf,
        codec: String,
    },

    /// Partial decode (truncated file)
    /// **[REQ-AP-ERR-012]** Partial decode ≥50% allows playback
    ///
    /// **Phase 4:** PartialDecode error reserved for future decoder diagnostics
    #[allow(dead_code)]
    #[error("Partial decode: {path}: expected {expected_duration_ms}ms, got {actual_duration_ms}ms")]
    PartialDecode {
        path: std::path::PathBuf,
        expected_duration_ms: u64,
        actual_duration_ms: u64,
    },

    /// Decoder panic
    /// **[REQ-AP-ERR-013]** Decoder panics caught and recovered
    #[error("Decoder panic: {path}: {message}")]
    DecoderPanic {
        path: std::path::PathBuf,
        message: String,
    },

    /// Resampling initialization failure
    /// **[REQ-AP-ERR-050]** Resampling init errors skip passage or bypass if rates match
    #[error("Resampling init failed: {source_rate}Hz -> {target_rate}Hz: {message}")]
    ResamplingInitFailed {
        source_rate: u32,
        target_rate: u32,
        message: String,
    },

    /// Resampling runtime error
    /// **[REQ-AP-ERR-051]** Resampling runtime errors skip passage
    #[error("Resampling runtime error at {position_ms}ms: {message}")]
    ResamplingRuntimeError {
        position_ms: u64,
        message: String,
    },

    /// File handle exhaustion
    /// **[REQ-AP-ERR-071]** Too many open files - OS descriptor limit reached
    #[error("File handle exhaustion: cannot open {path}: too many open files")]
    FileHandleExhaustion {
        path: std::path::PathBuf,
    },

    /// Position drift warning
    /// **[REQ-AP-ERR-060]** Sample position mismatch detected (moderate drift)
    #[error("Position drift: expected {expected_frames} frames, actual {actual_frames} frames, drift {drift_frames} frames ({drift_ms}ms)")]
    PositionDrift {
        expected_frames: usize,
        actual_frames: usize,
        drift_frames: usize,
        drift_ms: u64,
    },

    /// Audio output device errors
    #[error("Audio output error: {0}")]
    AudioOutput(String),

    /// Playback engine errors
    #[error("Playback error: {0}")]
    Playback(String),

    /// Queue management errors
    #[error("Queue error: {0}")]
    Queue(String),

    /// File I/O errors
    #[error("File I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Invalid timing parameters
    ///
    /// **Phase 4:** InvalidTiming error reserved for stricter timing validation
    #[allow(dead_code)]
    #[error("Invalid timing: {0}")]
    InvalidTiming(String),

    /// Invalid state for operation
    #[error("Invalid state: {0}")]
    InvalidState(String),

    /// Missing or invalid passage
    #[error("Passage not found: {0}")]
    PassageNotFound(String),

    /// Resource not found
    ///
    /// **Phase 4:** NotFound error reserved for REST API 404 responses
    #[allow(dead_code)]
    #[error("Not found: {0}")]
    NotFound(String),

    /// Feature not yet implemented
    ///
    /// **Phase 4:** NotImplemented error reserved for phased API development
    #[allow(dead_code)]
    #[error("Not implemented: {0}")]
    NotImplemented(String),

    /// Invalid request
    ///
    /// **Phase 4:** BadRequest error reserved for REST API 400 responses
    #[allow(dead_code)]
    #[error("Bad request: {0}")]
    BadRequest(String),

    /// Other errors
    ///
    /// **Phase 4:** Internal error reserved for catch-all error handling
    #[allow(dead_code)]
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Convenience Result type using wkmp-ap Error
pub type Result<T> = std::result::Result<T, Error>;
