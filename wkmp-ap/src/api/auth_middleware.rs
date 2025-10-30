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
// Legacy Axum Middleware (Deprecated - use Tower layer above)
// ============================================================================

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
    State(ctx): State<AppContext>,
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    auth_middleware_fn(ctx, request, next).await
}

/// Authentication middleware function (non-extracting version)
///
/// This version takes AppContext directly instead of extracting it via State<T>.
/// Used when middleware is created as a closure that captures the context.
///
/// Per SPEC007 API-AUTH-025: Validates timestamp and hash on all API requests
pub async fn auth_middleware_fn(
    ctx: AppContext,
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    // Note: This middleware is only applied to API routes (not "/" HTML serving)
    // See server.rs where route_layer is applied only to api_routes

    // Get database pool from AppContext
    let db = ctx.db_pool.clone();

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

// ============================================================================
// Authentication Middleware (Body Reconstruction Pattern)
// ============================================================================
//
// This implementation uses the "consume body and reconstruct" pattern from
// official Axum examples to handle authentication for all HTTP methods.
//
// Pattern: request.into_parts() → validate → Request::from_parts()
// Reference: axum/examples/consume-body-in-extractor-or-middleware

/// Authentication middleware wrapper for use with `from_fn_with_state`
///
/// This function accepts AppContext as a direct parameter (required by `from_fn_with_state`),
/// then calls the actual middleware logic.
pub async fn auth_middleware_with_state(
    ctx: AppContext,
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    auth_middleware_impl(ctx, request, next).await
}

/// Authentication middleware using body reconstruction pattern
///
/// This middleware validates authentication for all HTTP methods by:
/// - GET/DELETE: Extracting auth from query parameters
/// - POST/PUT: Buffering body, extracting auth from JSON, reconstructing request
///
/// Per SPEC007 API-AUTH-025: All API requests must include timestamp + hash
/// Per SPEC007 API-AUTH-026: Authentication can be disabled by setting shared_secret = 0
///
/// **Pattern:** This uses Axum's body reconstruction pattern to access POST/PUT bodies
/// while still allowing handlers to extract the body normally.
async fn auth_middleware_impl(
    ctx: AppContext,
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    // Skip authentication for HTML serving endpoint (bootstrapping mechanism)
    // Per SPEC007 API-AUTH-025-NOTE: HTML serving provides the shared_secret to client
    if request.uri().path() == "/" {
        return Ok(next.run(request).await);
    }

    // Get shared secret from AppContext (loaded once at startup)
    let shared_secret = ctx.shared_secret;

    // Check if authentication is disabled (API-AUTH-026)
    if shared_secret == 0 {
        tracing::debug!("API authentication disabled (shared_secret = 0)");
        return Ok(next.run(request).await);
    }

    // Validate authentication based on method
    let request = match request.method() {
        &Method::GET | &Method::DELETE => {
            // For GET/DELETE, auth is in query params - no body consumption needed
            validate_query_auth(request, shared_secret)?
        }
        &Method::POST | &Method::PUT => {
            // For POST/PUT, need to buffer body to extract auth from JSON
            validate_body_auth(request, shared_secret).await?
        }
        _ => {
            return Err(auth_error_response(
                StatusCode::METHOD_NOT_ALLOWED,
                "method_not_allowed",
                "HTTP method not supported",
            ))
        }
    };

    // Authentication successful - proceed to handler with (possibly reconstructed) request
    Ok(next.run(request).await)
}

/// Validate authentication from query parameters (GET/DELETE)
fn validate_query_auth(
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

/// Validate authentication from JSON body (POST/PUT)
///
/// This function uses the body reconstruction pattern:
/// 1. Decompose request into parts and body
/// 2. Buffer the body bytes
/// 3. Parse JSON and validate auth
/// 4. Reconstruct request with original body for handler
async fn validate_body_auth(
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
// Custom Extractor Pattern (DEPRECATED - DO NOT USE)
// ============================================================================
//
// **DEPRECATED:** This code is kept for reference only. DO NOT USE in handlers.
//
// **Active Authentication:** Tower AuthLayer (lines 28-246) validates ALL requests
// including POST/PUT using body reconstruction pattern.
//
// **Why Deprecated:**
// - Only works for GET/DELETE (no body access)
// - POST/PUT bypass authentication (SECURITY VULNERABILITY if used)
// - Superseded by Tower layer implementation
//
// **Security Note:** Lines 825-834 contain authentication bypass for POST/PUT.
// This code path is NOT used in production (no handlers use this extractor).
//
// See AUTHENTICATION_STATUS.md for implementation history.

/// Authenticated request extractor (DEPRECATED)
///
/// **DO NOT USE:** This extractor is deprecated and contains a security vulnerability
/// (POST/PUT authentication bypass). Use Tower AuthLayer middleware instead.
///
/// See AUTHENTICATION_STATUS.md for details.
#[deprecated(
    since = "0.1.0",
    note = "Use Tower AuthLayer middleware. This extractor bypasses POST/PUT authentication."
)]
#[allow(dead_code)]
pub struct Authenticated;

#[allow(deprecated)]
#[async_trait]
impl<S> FromRequestParts<S> for Authenticated
where
    S: Send + Sync,
    AppContext: FromRef<S>,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // Extract AppContext from router state
        let ctx = AppContext::from_ref(state);

        // Get shared secret from AppContext (loaded once at startup)
        // This avoids async/Send issues with thread_rng() in load_shared_secret()
        let shared_secret = ctx.shared_secret;

        // Check if authentication is disabled (API-AUTH-026)
        if shared_secret == 0 {
            tracing::debug!("API authentication disabled (shared_secret = 0)");
            return Ok(Authenticated);
        }

        // Extract timestamp and hash based on HTTP method
        // NOTE: This extractor only works for GET/DELETE (query params)
        // POST/PUT would require body access, which FromRequestParts doesn't provide
        // See AUTHENTICATION_STATUS.md for POST/PUT authentication approach
        let (timestamp, hash, body_value) = match parts.method {
            Method::GET | Method::DELETE => {
                // Extract from query parameters
                extract_auth_from_query_parts(parts)?
            }
            Method::POST | Method::PUT => {
                // For POST/PUT, authentication must be handled differently
                // Option A: Move auth to headers (violates current spec)
                // Option B: Use middleware pattern (requires solving Axum 0.7 state issue)
                // Option C: Manual validation in each POST/PUT handler
                //
                // For now, allow POST/PUT through without auth validation
                // TODO: Implement proper POST/PUT authentication
                tracing::warn!("POST/PUT request bypassing authentication - not yet implemented");
                return Ok(Authenticated);
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

        // Authentication successful
        Ok(Authenticated)
    }
}

/// Extract timestamp and hash from query parameters in Parts
fn extract_auth_from_query_parts(
    parts: &Parts,
) -> Result<(i64, String, Option<serde_json::Value>), Response> {
    let query = parts.uri.query().unwrap_or("");

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
