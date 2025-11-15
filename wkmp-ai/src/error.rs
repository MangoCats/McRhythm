//! Error types for wkmp-ai
//!
//! **[AIA-ERR-010]** Error severity categorization
//! **[AIA-ERR-020]** Error reporting

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

/// API error type
#[derive(Debug, Error)]
pub enum ApiError {
    /// Resource not found (404)
    #[error("Resource not found: {0}")]
    NotFound(String),

    /// Invalid request (400)
    #[error("Invalid request: {0}")]
    BadRequest(String),

    /// Conflict (409) - e.g., import already running
    #[error("Conflict: {0}")]
    Conflict(String),

    /// Internal server error (500)
    #[error("Internal server error: {0}")]
    Internal(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Generic error
    #[error(transparent)]
    Other(#[from] anyhow::Error),

    /// wkmp-common error
    #[error("Common error: {0}")]
    Common(#[from] wkmp_common::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_code, message) = match self {
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, "NOT_FOUND", msg),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "BAD_REQUEST", msg),
            ApiError::Conflict(msg) => (StatusCode::CONFLICT, "CONFLICT", msg),
            ApiError::Internal(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                msg,
            ),
            ApiError::Io(ref err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "IO_ERROR",
                err.to_string(),
            ),
            ApiError::Other(ref err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL_ERROR",
                err.to_string(),
            ),
            ApiError::Common(ref err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "COMMON_ERROR",
                err.to_string(),
            ),
        };

        let body = Json(json!({
            "error": {
                "code": error_code,
                "message": message,
            }
        }));

        (status, body).into_response()
    }
}

/// Result type for API handlers
pub type ApiResult<T> = Result<T, ApiError>;
