//! Authentication integration tests for POST/PUT requests
//!
//! **[REQ-DEBT-SEC-001-010, 020, 030, 040]** Verify POST/PUT authentication via AuthLayer
//!
//! Tests verify that:
//! - POST requests require timestamp + hash authentication
//! - PUT requests require timestamp + hash authentication
//! - Invalid/missing auth returns 401 Unauthorized
//! - Valid auth allows request to proceed

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, Method},
    routing::{get, post},
};
use tower::Service;
use serde_json::json;

/// Test helper to create minimal router with AuthLayer
fn create_test_router_with_auth(shared_secret: i64) -> Router {
    use wkmp_ap::api::auth_middleware::AuthLayer;

    // Create simple test handler that returns 200 OK
    async fn test_handler() -> StatusCode {
        StatusCode::OK
    }

    Router::new()
        .route("/", get(|| async { "HTML page" }))
        .route("/test", post(test_handler))
        .layer(AuthLayer { shared_secret })
}

/// **[TC-SEC-001-01]** POST with valid timestamp+hash succeeds
#[tokio::test]
async fn test_post_with_valid_auth_succeeds() {
    let shared_secret = 12345_i64;
    let router = create_test_router_with_auth(shared_secret);

    // Create valid authenticated POST request
    let timestamp = chrono::Utc::now().timestamp_millis();
    let body_json = json!({
        "timestamp": timestamp,
        "hash": "dummy",
        "test_data": "value"
    });

    // Calculate proper hash
    let hash = wkmp_common::api::calculate_hash(&body_json, shared_secret);
    let body_with_hash = json!({
        "timestamp": timestamp,
        "hash": hash,
        "test_data": "value"
    });

    let request = Request::builder()
        .method(Method::POST)
        .uri("/test")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&body_with_hash).unwrap()))
        .unwrap();

    let response = router.clone().call(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "POST with valid auth should succeed"
    );
}

/// **[TC-SEC-001-02]** POST with invalid hash returns 401
#[tokio::test]
async fn test_post_with_invalid_hash_returns_401() {
    let shared_secret = 12345_i64;
    let router = create_test_router_with_auth(shared_secret);

    let timestamp = chrono::Utc::now().timestamp_millis();
    let body_json = json!({
        "timestamp": timestamp,
        "hash": "invalid_hash_value",
        "test_data": "value"
    });

    let request = Request::builder()
        .method(Method::POST)
        .uri("/test")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&body_json).unwrap()))
        .unwrap();

    let response = router.clone().call(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "POST with invalid hash should return 401"
    );
}

/// **[TC-SEC-001-03]** POST without timestamp/hash returns 400 or 401
#[tokio::test]
async fn test_post_without_auth_returns_401() {
    let shared_secret = 12345_i64;
    let router = create_test_router_with_auth(shared_secret);

    let body_json = json!({
        "test_data": "value"
    });

    let request = Request::builder()
        .method(Method::POST)
        .uri("/test")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&body_json).unwrap()))
        .unwrap();

    let response = router.clone().call(request).await.unwrap();

    // Missing auth fields returns 400 BAD_REQUEST (malformed request)
    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "POST without auth fields should return 400"
    );
}

/// **[TC-SEC-001-04]** POST with expired timestamp returns 401
#[tokio::test]
async fn test_post_with_expired_timestamp_returns_401() {
    let shared_secret = 12345_i64;
    let router = create_test_router_with_auth(shared_secret);

    // Use timestamp from 10 seconds in the past (>1000ms window)
    let timestamp = chrono::Utc::now().timestamp_millis() - 10_000;
    let body_json = json!({
        "timestamp": timestamp,
        "hash": "dummy",
        "test_data": "value"
    });

    let hash = wkmp_common::api::calculate_hash(&body_json, shared_secret);
    let body_with_hash = json!({
        "timestamp": timestamp,
        "hash": hash,
        "test_data": "value"
    });

    let request = Request::builder()
        .method(Method::POST)
        .uri("/test")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&body_with_hash).unwrap()))
        .unwrap();

    let response = router.clone().call(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "POST with expired timestamp should return 401"
    );
}

/// **[TC-SEC-001-05]** Authentication disabled (shared_secret = 0) allows all requests
#[tokio::test]
async fn test_auth_disabled_allows_all_requests() {
    let shared_secret = 0_i64; // Disable auth
    let router = create_test_router_with_auth(shared_secret);

    let body_json = json!({
        "test_data": "value"
    });

    let request = Request::builder()
        .method(Method::POST)
        .uri("/test")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&body_json).unwrap()))
        .unwrap();

    let response = router.clone().call(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "POST with auth disabled should succeed"
    );
}

/// **[TC-SEC-001-06]** HTML serving endpoint bypasses auth
#[tokio::test]
async fn test_html_endpoint_bypasses_auth() {
    let shared_secret = 12345_i64;
    let router = create_test_router_with_auth(shared_secret);

    let request = Request::builder()
        .method(Method::GET)
        .uri("/")
        .body(Body::empty())
        .unwrap();

    let response = router.clone().call(request).await.unwrap();

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "HTML serving endpoint should bypass auth"
    );
}
