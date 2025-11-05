//! Authentication middleware for wkmp-ap API
//!
//! Validates timestamp and hash per SPEC007 API-AUTH-025
//! Uses wkmp_common authentication functions
//!
//! **Implementation Note:** Uses custom extractor pattern instead of middleware
//! due to Axum 0.7 state handling complexity. See AUTHENTICATION_STATUS.md for details.

use crate::api::server::AppContext;
use axum::{
    async_trait,
    body::Body,
    extract::{FromRef, FromRequestParts, Request, State},
    http::{request::Parts, Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use tower::{Layer, Service};
use std::task::{Context, Poll};
use std::pin::Pin;
use std::future::Future;
use serde_json::json;
use wkmp_common::api::{
    types::AuthErrorResponse, validate_hash, validate_timestamp, ApiAuthError,
};

// ============================================================================
// Tower Layer Implementation (Recommended for Axum 0.7)
// ============================================================================

/// Tower layer for API authentication
///
/// This layer validates timestamp + hash authentication for all HTTP requests
/// except the "/" endpoint (which serves the HTML with embedded shared_secret).
///
/// Per SPEC007 API-AUTH-025: All API requests must include timestamp + hash
/// Per SPEC007 API-AUTH-026: Authentication can be disabled by setting shared_secret = 0
#[derive(Clone)]
pub struct AuthLayer {
    pub shared_secret: i64,
}

impl<S> Layer<S> for AuthLayer {
    type Service = AuthMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthMiddleware {
            inner,
            shared_secret: self.shared_secret,
        }
    }
}

/// Tower service that performs authentication validation
#[derive(Clone)]
pub struct AuthMiddleware<S> {
    inner: S,
    shared_secret: i64,
}

impl<S> Service<Request> for AuthMiddleware<S>
where
    S: Service<Request, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        let shared_secret = self.shared_secret;
        let mut inner = self.inner.clone();

        Box::pin(async move {
            // Skip authentication for HTML serving endpoint (bootstrapping mechanism)
            // Per SPEC007 API-AUTH-025-NOTE: HTML serving provides the shared_secret to client
            // Skip authentication for SSE endpoint (EventSource API cannot send custom headers)
            if request.uri().path() == "/" || request.uri().path() == "/events" {
                return inner.call(request).await;
            }

            // Check if authentication is disabled (API-AUTH-026)
            if shared_secret == 0 {
                tracing::debug!("API authentication disabled (shared_secret = 0)");
                return inner.call(request).await;
            }

            // Validate authentication based on method
            let validated_request = match request.method() {
                &Method::GET | &Method::DELETE => {
                    match validate_query_auth_tower(request, shared_secret) {
                        Ok(req) => req,
                        Err(response) => return Ok(response),
                    }
                }
                &Method::POST | &Method::PUT => {
                    match validate_body_auth_tower(request, shared_secret).await {
                        Ok(req) => req,
                        Err(response) => return Ok(response),
                    }
                }
                _ => {
                    let response = auth_error_response(
                        StatusCode::METHOD_NOT_ALLOWED,
                        "method_not_allowed",
                        "HTTP method not supported",
                    );
                    return Ok(response);
                }
            };

            // Authentication successful - proceed to inner service
            inner.call(validated_request).await
        })
    }
}

/// Validate authentication from query parameters (GET/DELETE) for Tower layer
fn validate_query_auth_tower(
    request: Request,
    shared_secret: i64,
) -> Result<Request, Response> {
    let query = request.uri().query().unwrap_or("");

    // Parse query string for auth fields
    let mut timestamp: Option<i64> = None;
    let mut hash: Option<String> = None;

    for pair in query.split('&') {
        if let Some((key, value)) = pair.split_once('=') {
            match key {
                "timestamp" => timestamp = value.parse::<i64>().ok(),
                "hash" => hash = Some(value.to_string()),
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

    // Validate timestamp
    if let Err(e) = validate_timestamp(timestamp) {
        return Err(map_auth_error_to_response(e));
    }

    // Validate hash
    let json_value = json!({
        "timestamp": timestamp,
        "hash": &hash
    });

    if let Err(e) = validate_hash(&hash, &json_value, shared_secret) {
        return Err(map_auth_error_to_response(e));
    }

    // Auth valid - return original request unchanged
    Ok(request)
}

/// Validate authentication from JSON body (POST/PUT) for Tower layer
async fn validate_body_auth_tower(
    request: Request,
    shared_secret: i64,
) -> Result<Request, Response> {
    use axum::body::to_bytes;

    // Decompose request into parts and body
    let (parts, body) = request.into_parts();

    // Buffer the body
    let bytes = to_bytes(body, usize::MAX)
        .await
        .map_err(|err| {
            tracing::error!("Failed to read request body: {}", err);
            auth_error_response(
                StatusCode::BAD_REQUEST,
                "invalid_body",
                "Failed to read request body",
            )
        })?;

    // Parse as JSON
    let json_value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|err| {
        tracing::error!("Failed to parse request body as JSON: {}", err);
        auth_error_response(
            StatusCode::BAD_REQUEST,
            "invalid_json",
            "Request body must be valid JSON",
        )
    })?;

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
        .ok_or_else(|| {
            auth_error_response(
                StatusCode::BAD_REQUEST,
                "missing_hash",
                "JSON field 'hash' is required",
            )
        })?;

    // Validate timestamp
    if let Err(e) = validate_timestamp(timestamp) {
        return Err(map_auth_error_to_response(e));
    }

    // Validate hash with full JSON body
    if let Err(e) = validate_hash(hash, &json_value, shared_secret) {
        return Err(map_auth_error_to_response(e));
    }

    // Auth valid - reconstruct request with original body
    let body = Body::from(bytes);
    Ok(Request::from_parts(parts, body))
}


// ============================================================================
// Helper Functions
// ============================================================================

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
