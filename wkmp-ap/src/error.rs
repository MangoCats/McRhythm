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

    /// Audio decoding errors
    #[error("Audio decode error: {0}")]
    Decode(String),

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
    #[error("Invalid timing: {0}")]
    InvalidTiming(String),

    /// Invalid state for operation
    #[error("Invalid state: {0}")]
    InvalidState(String),

    /// Missing or invalid passage
    #[error("Passage not found: {0}")]
    PassageNotFound(String),

    /// Resource not found
    #[error("Not found: {0}")]
    NotFound(String),

    /// Feature not yet implemented
    #[error("Not implemented: {0}")]
    NotImplemented(String),

    /// Invalid request
    #[error("Bad request: {0}")]
    BadRequest(String),

    /// Other errors
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Convenience Result type using wkmp-ap Error
pub type Result<T> = std::result::Result<T, Error>;
