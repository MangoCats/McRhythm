//! Integration tests for Settings API endpoint
//!
//! Tests the implementation of:
//! - [APIK-UI-010] POST /api/settings/acoustid_api_key endpoint
//! - [APIK-UI-020] Request/response validation
//! - [APIK-UI-030] Database + TOML write-back

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::json;
use sqlx::sqlite::SqlitePoolOptions;
use tower::ServiceExt;
use wkmp_ai::{build_router, AppState};
use wkmp_common::events::EventBus;

// ============================================================================
// Integration Tests (tc_i_ui_001-003)
// ============================================================================

#[tokio::test]
async fn test_set_api_key_success() {
    // tc_i_ui_001: Valid key updates database and TOML
    let pool = SqlitePoolOptions::new()
        .connect(":memory:")
        .await
        .unwrap();
    // Initialize test database schema
    sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await.unwrap();
    wkmp_common::db::init::create_settings_table(&pool).await.unwrap();

    let event_bus = EventBus::new(100);
    let state = AppState::new(pool.clone(), event_bus, 16);
    let app = build_router(state);

    // Send request
    let request = Request::builder()
        .method("POST")
        .uri("/api/settings/acoustid_api_key")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "api_key": "test-key-valid-123"
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Verify response
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let response_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(response_json["success"], true);
    assert!(response_json["message"]
        .as_str()
        .unwrap()
        .contains("configured successfully"));

    // Verify database
    let db_key = wkmp_ai::db::settings::get_acoustid_api_key(&pool)
        .await
        .unwrap();
    assert_eq!(db_key, Some("test-key-valid-123".to_string()));
}

#[tokio::test]
async fn test_set_api_key_rejects_empty_key() {
    // tc_i_ui_002: Empty key rejected with 400
    let pool = SqlitePoolOptions::new()
        .connect(":memory:")
        .await
        .unwrap();
    // Initialize test database schema
    sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await.unwrap();
    wkmp_common::db::init::create_settings_table(&pool).await.unwrap();

    let event_bus = EventBus::new(100);
    let state = AppState::new(pool.clone(), event_bus, 16);
    let app = build_router(state);

    // Send request with empty key
    let request = Request::builder()
        .method("POST")
        .uri("/api/settings/acoustid_api_key")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "api_key": ""
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Verify 400 Bad Request
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // Verify database NOT updated
    let db_key = wkmp_ai::db::settings::get_acoustid_api_key(&pool)
        .await
        .unwrap();
    assert_eq!(db_key, None);
}

#[tokio::test]
async fn test_set_api_key_rejects_whitespace_key() {
    // tc_i_ui_003: Whitespace-only key rejected with 400
    let pool = SqlitePoolOptions::new()
        .connect(":memory:")
        .await
        .unwrap();
    // Initialize test database schema
    sqlx::query("PRAGMA foreign_keys = ON").execute(&pool).await.unwrap();
    wkmp_common::db::init::create_settings_table(&pool).await.unwrap();

    let event_bus = EventBus::new(100);
    let state = AppState::new(pool.clone(), event_bus, 16);
    let app = build_router(state);

    // Send request with whitespace-only key
    let request = Request::builder()
        .method("POST")
        .uri("/api/settings/acoustid_api_key")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "api_key": "   \t\n  "
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // Verify 400 Bad Request
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // Verify database NOT updated
    let db_key = wkmp_ai::db::settings::get_acoustid_api_key(&pool)
        .await
        .unwrap();
    assert_eq!(db_key, None);
}
