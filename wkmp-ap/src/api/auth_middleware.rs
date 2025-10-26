//! Authentication middleware for wkmp-ap API
//!
//! Validates timestamp and hash per SPEC007 API-AUTH-025
//! Uses wkmp_common authentication functions

use crate::state::AppState;
use axum::{
    body::Body,
    extract::{Request, State},
    http::{Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;
use wkmp_common::api::{
    types::AuthErrorResponse, validate_hash, validate_timestamp, ApiAuthError,
};

/// Axum middleware for API authentication
///
/// Per SPEC007 API-AUTH-025:
/// - GET requests: Extract timestamp/hash from query parameters
/// - POST/PUT/DELETE requests: Extract timestamp/hash from JSON body
///
/// Per API-AUTH-026: Authentication can be disabled by setting shared_secret = 0
///
/// # Examples
///
/// ```ignore
/// use axum::{Router, middleware};
///
/// let router = Router::new()
///     .route("/playback/play", post(handlers::play))
///     .layer(middleware::from_fn_with_state(state.clone(), auth_middleware));
/// ```
pub async fn auth_middleware(
    State(state): State<Arc<RwLock<AppState>>>,
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    // Get database pool from AppState
    let db = {
        let state_guard = state.read().await;
        state_guard.db_pool.clone()
    };

    // Load shared secret from database
    let shared_secret = match wkmp_common::api::load_shared_secret(&db).await {
        Ok(secret) => secret,
        Err(e) => {
            tracing::error!("Failed to load shared secret: {}", e);
            return Err(auth_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "database_error",
                "Failed to load authentication configuration",
            ));
        }
    };

    // Check if authentication is disabled (API-AUTH-026)
    if shared_secret == 0 {
        tracing::debug!("API authentication disabled (shared_secret = 0)");
        return Ok(next.run(request).await);
    }

    // Extract timestamp and hash based on HTTP method
    let (timestamp, hash, body_value, reconstructed_request) = match request.method() {
        &Method::GET | &Method::DELETE => {
            // Extract from query parameters
            let (ts, h, body) = extract_auth_from_query(&request)?;
            (ts, h, body, request)
        }
        &Method::POST | &Method::PUT => {
            // Extract from JSON body and reconstruct request
            extract_auth_from_body(request).await?
        }
        _ => {
            return Err(auth_error_response(
                StatusCode::METHOD_NOT_ALLOWED,
                "method_not_allowed",
                "HTTP method not supported",
            ))
        }
    };

    // Validate timestamp (API-AUTH-029, API-AUTH-030)
    if let Err(e) = validate_timestamp(timestamp) {
        return Err(map_auth_error_to_response(e));
    }

    // Validate hash (API-AUTH-027)
    let json_value = if let Some(body) = body_value {
        // POST/PUT: Use full body
        body
    } else {
        // GET/DELETE: Create minimal JSON with timestamp and hash
        json!({
            "timestamp": timestamp,
            "hash": &hash
        })
    };

    if let Err(e) = validate_hash(&hash, &json_value, shared_secret) {
        return Err(map_auth_error_to_response(e));
    }

    // Authentication successful - proceed to handler with reconstructed request
    Ok(next.run(reconstructed_request).await)
}

/// Extract timestamp and hash from query parameters (GET/DELETE)
fn extract_auth_from_query(
    request: &Request,
) -> Result<(i64, String, Option<serde_json::Value>), Response> {
    let query = request.uri().query().unwrap_or("");

    // Parse query string manually (simple approach)
    let mut timestamp: Option<i64> = None;
    let mut hash: Option<String> = None;

    for pair in query.split('&') {
        if let Some((key, value)) = pair.split_once('=') {
            match key {
                "timestamp" => {
                    timestamp = value.parse::<i64>().ok();
                }
                "hash" => {
                    hash = Some(value.to_string());
                }
                _ => {}
            }
        }
    }

    let timestamp = timestamp.ok_or_else(|| {
        auth_error_response(
            StatusCode::BAD_REQUEST,
            "missing_timestamp",
            "Query parameter 'timestamp' is required",
        )
    })?;

    let hash = hash.ok_or_else(|| {
        auth_error_response(
            StatusCode::BAD_REQUEST,
            "missing_hash",
            "Query parameter 'hash' is required",
        )
    })?;

    Ok((timestamp, hash, None))
}

/// Extract timestamp and hash from JSON body (POST/PUT)
async fn extract_auth_from_body(
    request: Request,
) -> Result<(i64, String, Option<serde_json::Value>, Request), Response> {
    // Destructure request to get parts and body
    let (parts, body) = request.into_parts();

    // Read body bytes
    let body_bytes = match axum::body::to_bytes(body, usize::MAX).await {
        Ok(bytes) => bytes,
        Err(e) => {
            tracing::error!("Failed to read request body: {}", e);
            return Err(auth_error_response(
                StatusCode::BAD_REQUEST,
                "invalid_body",
                "Failed to read request body",
            ));
        }
    };

    // Parse as JSON
    let json_value: serde_json::Value = match serde_json::from_slice(&body_bytes) {
        Ok(value) => value,
        Err(e) => {
            tracing::error!("Failed to parse request body as JSON: {}", e);
            return Err(auth_error_response(
                StatusCode::BAD_REQUEST,
                "invalid_json",
                "Request body must be valid JSON",
            ));
        }
    };

    // Extract timestamp and hash fields
    let timestamp = json_value
        .get("timestamp")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| {
            auth_error_response(
                StatusCode::BAD_REQUEST,
                "missing_timestamp",
                "JSON field 'timestamp' is required",
            )
        })?;

    let hash = json_value
        .get("hash")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            auth_error_response(
                StatusCode::BAD_REQUEST,
                "missing_hash",
                "JSON field 'hash' is required",
            )
        })?;

    // Reconstruct request with original body
    let reconstructed = Request::from_parts(parts, Body::from(body_bytes));

    Ok((timestamp, hash, Some(json_value), reconstructed))
}

/// Convert ApiAuthError to HTTP response
fn map_auth_error_to_response(error: ApiAuthError) -> Response {
    use ApiAuthError::*;

    match error {
        InvalidTimestamp {
            timestamp,
            now,
            reason,
        } => {
            let details = json!({
                "timestamp": timestamp,
                "server_time": now,
                "reason": reason
            });

            (
                StatusCode::UNAUTHORIZED,
                Json(AuthErrorResponse::with_details(
                    "timestamp_invalid",
                    "Request timestamp outside acceptable window",
                    details,
                )),
            )
                .into_response()
        }
        InvalidHash {
            provided,
            calculated,
        } => {
            let details = json!({
                "provided": provided,
                "calculated": calculated
            });

            (
                StatusCode::UNAUTHORIZED,
                Json(AuthErrorResponse::with_details(
                    "hash_invalid",
                    "Request hash does not match calculated value",
                    details,
                )),
            )
                .into_response()
        }
        MissingTimestamp => auth_error_response(
            StatusCode::BAD_REQUEST,
            "missing_timestamp",
            "Timestamp field is required",
        ),
        MissingHash => auth_error_response(
            StatusCode::BAD_REQUEST,
            "missing_hash",
            "Hash field is required",
        ),
        DatabaseError(msg) => {
            tracing::error!("Database error during auth: {}", msg);
            auth_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "database_error",
                "Failed to validate authentication",
            )
        }
        ParseError(msg) => {
            tracing::error!("Parse error during auth: {}", msg);
            auth_error_response(
                StatusCode::BAD_REQUEST,
                "parse_error",
                "Failed to parse authentication fields",
            )
        }
    }
}

/// Create auth error response
fn auth_error_response(
    status: StatusCode,
    error: &str,
    message: &str,
) -> Response {
    (
        status,
        Json(AuthErrorResponse::new(error, message)),
    )
        .into_response()
}
