//! Common error types for WKMP

use thiserror::Error;

/// Common result type for WKMP operations
pub type Result<T> = std::result::Result<T, Error>;

/// Common error types across WKMP microservices
#[derive(Error, Debug)]
pub enum Error {
    /// Database operation error (wraps sqlx::Error)
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    /// I/O operation error (wraps std::io::Error)
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Configuration loading or validation error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Requested resource not found
    #[error("Not found: {0}")]
    NotFound(String),

    /// Invalid user input or request parameter
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Internal server error
    #[error("Internal error: {0}")]
    Internal(String),
}
