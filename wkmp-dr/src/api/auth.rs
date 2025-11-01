//! Authentication middleware for wkmp-dr
//!
//! [REQ-DR-NF-030]: API authentication via timestamp + SHA-256 hash
//! Implements API-AUTH-025 through API-AUTH-031 per SPEC007

use axum::{
    body::Body,
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use tracing::warn;
use wkmp_common::api::auth::{validate_hash, validate_timestamp, ApiAuthError};

use crate::AppState;

/// Authentication request fields
///
/// Per API-AUTH-025: All API requests include timestamp and hash
#[derive(Debug, Deserialize)]
struct AuthFields {
    timestamp: i64,
    hash: String,
}

/// Authentication middleware
///
/// Validates timestamp and hash per REQ-DR-NF-030.
/// Returns 401 Unauthorized if validation fails.
///
/// **Note:** This is applied to protected routes only.
/// Health endpoint (/health) does NOT use this middleware.
pub async fn auth_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, AuthError> {
    // Special case: secret = 0 disables ALL auth checking [API-AUTH-028]
    if state.shared_secret == 0 {
        // Auth disabled - pass through without validation
        return Ok(next.run(request).await);
    }

    // Extract body for hash validation
    // [DR-SEC-050] Limit body size to 10MB to prevent DoS via memory exhaustion
    let (parts, body) = request.into_parts();
    let body_bytes = axum::body::to_bytes(body, 10 * 1024 * 1024)
        .await
        .map_err(|e| AuthError::ParseError(format!("Failed to read body: {}", e)))?;

    // Parse JSON to extract timestamp and hash
    let json_value: Value = serde_json::from_slice(&body_bytes)
        .map_err(|e| AuthError::ParseError(format!("Invalid JSON: {}", e)))?;

    let auth_fields: AuthFields = serde_json::from_value(json_value.clone())
        .map_err(|e| AuthError::MissingFields(format!("Missing auth fields: {}", e)))?;

    // Step 1: Validate timestamp [API-AUTH-029, API-AUTH-030]
    validate_timestamp(auth_fields.timestamp).map_err(|e| match e {
        ApiAuthError::InvalidTimestamp { reason, .. } => AuthError::InvalidTimestamp(reason),
        _ => AuthError::Other(e.to_string()),
    })?;

    // Step 2: Validate hash [API-AUTH-027]
    validate_hash(&auth_fields.hash, &json_value, state.shared_secret).map_err(|e| match e {
        ApiAuthError::InvalidHash { provided, calculated } => {
            warn!(
                "Hash validation failed: provided={}, calculated={}",
                provided, calculated
            );
            AuthError::InvalidHash
        }
        _ => AuthError::Other(e.to_string()),
    })?;

    // Reconstruct request with restored body for downstream handlers
    let request = Request::from_parts(parts, Body::from(body_bytes));

    // Authentication successful - proceed to handler
    Ok(next.run(request).await)
}

/// Authentication error types for HTTP responses
#[derive(Debug)]
pub enum AuthError {
    InvalidTimestamp(String),
    InvalidHash,
    MissingFields(String),
    ParseError(String),
    Other(String),
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AuthError::InvalidTimestamp(reason) => {
                (StatusCode::UNAUTHORIZED, format!("Invalid timestamp: {}", reason))
            }
            AuthError::InvalidHash => (StatusCode::UNAUTHORIZED, "Invalid hash".to_string()),
            AuthError::MissingFields(msg) => {
                (StatusCode::BAD_REQUEST, format!("Missing required fields: {}", msg))
            }
            AuthError::ParseError(msg) => {
                (StatusCode::BAD_REQUEST, format!("Parse error: {}", msg))
            }
            AuthError::Other(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Authentication error: {}", msg))
            }
        };

        let body = Json(json!({
            "error": message,
        }));

        (status, body).into_response()
    }
}
