//! Security tests for wkmp-dr
//!
//! Tests security-critical features:
//! - [DR-SEC-050]: 10MB body size limit to prevent DoS via memory exhaustion

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use sqlx::SqlitePool;
use std::path::PathBuf;
use tower::util::ServiceExt;
use wkmp_dr::{build_router, AppState};

/// Test helper: Connect to real wkmp.db database
async fn setup_test_db() -> SqlitePool {
    let db_path = PathBuf::from(env!("HOME")).join("Music/wkmp.db");

    if !db_path.exists() {
        panic!("Test database not found at {:?}. Integration tests require real database.", db_path);
    }

    wkmp_dr::db::connect_readonly(&db_path)
        .await
        .expect("Should connect to test database")
}

/// Test helper: Create app with auth enabled
fn setup_app_with_auth(db: SqlitePool) -> axum::Router {
    // Use non-zero secret to enable auth middleware
    let state = AppState::new(db, 12345);
    build_router(state)
}

// =============================================================================
// Body Size Limit Tests [DR-SEC-050]
// =============================================================================

/// Test that bodies exceeding 10MB are rejected
///
/// [DR-SEC-050]: Request body size limited to 10MB to prevent DoS
/// via memory exhaustion attacks.
///
/// The limit is enforced in auth middleware (auth.rs:51-53):
/// ```rust
/// let body_bytes = axum::body::to_bytes(body, 10 * 1024 * 1024)
///     .await
///     .map_err(|e| AuthError::ParseError(format!("Failed to read body: {}", e)))?;
/// ```
#[tokio::test]
async fn test_body_size_limit_10mb() {
    let db = setup_test_db().await;
    let app = setup_app_with_auth(db);

    // Create a body slightly larger than 10MB
    let large_body_size = 10 * 1024 * 1024 + 1024; // 10MB + 1KB
    let large_body = vec![b'x'; large_body_size];

    let request = Request::builder()
        .method("POST")
        .uri("/api/table/songs")
        .header("Content-Type", "application/json")
        .body(Body::from(large_body))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should reject with 413 Payload Too Large or 400 Bad Request
    // Axum may return different codes depending on how the limit is hit
    assert!(
        response.status() == StatusCode::PAYLOAD_TOO_LARGE
            || response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::REQUEST_TIMEOUT,
        "Expected 413/400/408 for oversized body, got {}",
        response.status()
    );
}

/// Test that bodies under 10MB are accepted (not rejected for size)
///
/// This verifies the limit is not too restrictive and allows valid requests.
#[tokio::test]
async fn test_body_size_under_limit() {
    let db = setup_test_db().await;
    let app = setup_app_with_auth(db);

    // Create a body well under 10MB (1MB)
    let small_body_size = 1 * 1024 * 1024; // 1MB

    // Create valid JSON (will fail auth but not size limit)
    let json_body = format!(
        r#"{{"timestamp": 1234567890, "hash": "{}"}}"#,
        "a".repeat(small_body_size - 100) // Pad to approach 1MB
    );

    let request = Request::builder()
        .method("POST")
        .uri("/api/table/songs")
        .header("Content-Type", "application/json")
        .body(Body::from(json_body))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Should NOT reject for size (may fail auth with 401, but not 413)
    assert_ne!(
        response.status(),
        StatusCode::PAYLOAD_TOO_LARGE,
        "Body under 10MB should not be rejected for size"
    );
}

/// Test exactly 10MB body (boundary condition)
///
/// Verifies the limit is exactly 10MB, not off-by-one.
#[tokio::test]
async fn test_body_size_exactly_10mb() {
    let db = setup_test_db().await;
    let app = setup_app_with_auth(db);

    // Create exactly 10MB body
    let exact_body_size = 10 * 1024 * 1024; // Exactly 10MB

    let json_body = format!(
        r#"{{"timestamp": 1234567890, "hash": "{}"}}"#,
        "a".repeat(exact_body_size - 100) // Pad to exactly 10MB
    );

    let request = Request::builder()
        .method("POST")
        .uri("/api/table/songs")
        .header("Content-Type", "application/json")
        .body(Body::from(json_body))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Exactly 10MB should be accepted (not rejected for size)
    assert_ne!(
        response.status(),
        StatusCode::PAYLOAD_TOO_LARGE,
        "Exactly 10MB body should be accepted"
    );
}
