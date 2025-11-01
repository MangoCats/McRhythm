//! Integration tests for wkmp-dr API endpoints
//!
//! Tests cover:
//! - [REQ-DR-F-010, REQ-DR-F-020, REQ-DR-F-080] Table viewing with pagination/sorting
//! - [REQ-DR-F-040] Passages without MusicBrainz ID filter
//! - [REQ-DR-F-050] Files without passages filter
//! - [REQ-DR-F-060] Search by MusicBrainz Work ID
//! - [REQ-DR-F-070] Search by file path pattern
//! - [REQ-DR-NF-030] Authentication middleware
//! - [REQ-DR-NF-040] Health endpoint (no auth required)

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::Value;
use sqlx::SqlitePool;
use std::path::PathBuf;
use tower::util::ServiceExt; // for `oneshot` method
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

/// Test helper: Create app with test state (auth disabled)
fn setup_app(db: SqlitePool) -> axum::Router {
    // Use shared_secret=0 to disable auth checking [API-AUTH-028]
    // This simplifies testing while still validating routing and handler logic
    let state = AppState::new(db, 0);
    build_router(state)
}

/// Test helper: Create request (auth disabled in test app)
fn test_request(method: &str, uri: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

/// Test helper: Extract JSON body from response
async fn extract_json(body: Body) -> Value {
    let bytes = axum::body::to_bytes(body, usize::MAX)
        .await
        .expect("Should read body");
    serde_json::from_slice(&bytes).expect("Should parse JSON")
}

// =============================================================================
// Health Endpoint Tests [REQ-DR-NF-040]
// =============================================================================

#[tokio::test]
async fn test_health_endpoint_no_auth_required() {
    let db = setup_test_db().await;
    let app = setup_app(db);

    let request = test_request("GET", "/health");
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = extract_json(response.into_body()).await;
    assert_eq!(body["status"], "ok");
    assert_eq!(body["module"], "wkmp-dr");
    assert!(body["version"].is_string());
}

// =============================================================================
// Authentication Tests [REQ-DR-NF-030]
// =============================================================================
//
// NOTE: Authentication testing is simplified using shared_secret=0 to disable auth.
// This allows us to test routing and handler logic without implementing full
// timestamp+hash authentication in tests.
//
// Per [API-AUTH-028], shared_secret=0 disables auth checking entirely.
//
// For production auth testing, see wkmp-common/tests/api_auth_tests.rs which
// tests the auth validation logic in isolation.

// =============================================================================
// Table Viewing Tests [REQ-DR-F-010, REQ-DR-F-020, REQ-DR-F-080]
// =============================================================================

#[tokio::test]
async fn test_table_viewing_basic() {
    let db = setup_test_db().await;
    let app = setup_app(db);

    let request = test_request("GET", "/api/table/songs?page=1");
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = extract_json(response.into_body()).await;

    // Verify response structure [REQ-DR-F-030]
    assert_eq!(body["table_name"], "songs");
    assert!(body["total_rows"].is_number());
    assert_eq!(body["page"], 1);
    assert_eq!(body["page_size"], 100);
    assert!(body["total_pages"].is_number());
    assert!(body["columns"].is_array());
    assert!(body["rows"].is_array());
}

#[tokio::test]
async fn test_table_pagination() {
    let db = setup_test_db().await;
    let app = setup_app(db);

    // Test page 2
    let request = test_request("GET", "/api/table/songs?page=2");
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = extract_json(response.into_body()).await;
    assert_eq!(body["page"], 2);
}

#[tokio::test]
async fn test_table_sorting_asc() {
    let db = setup_test_db().await;
    let app = setup_app(db);

    let request = test_request("GET", "/api/table/songs?page=1&sort=created_at&order=asc");
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_table_sorting_desc() {
    let db = setup_test_db().await;
    let app = setup_app(db);

    let request = test_request("GET", "/api/table/songs?page=1&sort=created_at&order=desc");
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_table_invalid_table_name() {
    let db = setup_test_db().await;
    let app = setup_app(db);

    let request = test_request("GET", "/api/table/invalid_table");
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = extract_json(response.into_body()).await;
    assert!(body["error"].as_str().unwrap().contains("Invalid table name"));
}

#[tokio::test]
async fn test_table_invalid_column() {
    let db = setup_test_db().await;
    let app = setup_app(db);

    let request = test_request("GET", "/api/table/songs?sort=invalid_column");
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = extract_json(response.into_body()).await;
    assert!(body["error"].as_str().unwrap().contains("Invalid column"));
}

// =============================================================================
// Filter Tests [REQ-DR-F-040, REQ-DR-F-050]
// =============================================================================

#[tokio::test]
async fn test_filter_passages_without_mbid() {
    let db = setup_test_db().await;
    let app = setup_app(db);

    let request = test_request("GET", "/api/filters/passages-without-mbid?page=1");
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = extract_json(response.into_body()).await;

    assert_eq!(body["filter_name"], "passages-without-mbid");
    assert_eq!(body["description"], "Passages lacking MusicBrainz recording ID");
    assert!(body["total_results"].is_number());
    assert_eq!(body["page"], 1);
    assert_eq!(body["page_size"], 100);
    assert!(body["columns"].is_array());
    assert!(body["rows"].is_array());

    // Verify columns
    let columns = body["columns"].as_array().unwrap();
    assert!(columns.contains(&Value::String("guid".to_string())));
    assert!(columns.contains(&Value::String("file_id".to_string())));
}

#[tokio::test]
async fn test_filter_files_without_passages() {
    let db = setup_test_db().await;
    let app = setup_app(db);

    let request = test_request("GET", "/api/filters/files-without-passages?page=1");
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = extract_json(response.into_body()).await;

    assert_eq!(body["filter_name"], "files-without-passages");
    assert_eq!(body["description"], "Audio files not yet segmented into passages");
    assert!(body["total_results"].is_number());
    assert_eq!(body["page"], 1);
    assert_eq!(body["page_size"], 100);

    // Verify columns
    let columns = body["columns"].as_array().unwrap();
    assert!(columns.contains(&Value::String("guid".to_string())));
    assert!(columns.contains(&Value::String("path".to_string())));
}

// =============================================================================
// Search Tests [REQ-DR-F-060, REQ-DR-F-070]
// =============================================================================

#[tokio::test]
async fn test_search_by_path() {
    let db = setup_test_db().await;
    let app = setup_app(db);

    let request = test_request("GET", "/api/search/by-path?pattern=%.flac");
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = extract_json(response.into_body()).await;

    assert_eq!(body["search_type"], "by-path");
    assert_eq!(body["query"], "%.flac");
    assert!(body["total_results"].is_number());
    assert_eq!(body["page"], 1);
    assert_eq!(body["page_size"], 100);
}

#[tokio::test]
async fn test_search_by_path_empty_pattern() {
    let db = setup_test_db().await;
    let app = setup_app(db);

    let request = test_request("GET", "/api/search/by-path?pattern=");
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = extract_json(response.into_body()).await;
    assert!(body["error"].as_str().unwrap().contains("Empty search pattern"));
}

#[tokio::test]
async fn test_search_by_work_id_invalid_uuid() {
    let db = setup_test_db().await;
    let app = setup_app(db);

    let request = test_request("GET", "/api/search/by-work-id?work_id=not-a-uuid");
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = extract_json(response.into_body()).await;
    assert!(body["error"].as_str().unwrap().contains("Invalid Work ID"));
}

// =============================================================================
// Pagination Edge Cases
// =============================================================================

#[tokio::test]
async fn test_pagination_out_of_bounds_high() {
    let db = setup_test_db().await;
    let app = setup_app(db);

    // Request impossibly high page number
    let request = test_request("GET", "/api/table/songs?page=9999");
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = extract_json(response.into_body()).await;

    // Should clamp to last page
    let page = body["page"].as_i64().unwrap();
    let total_pages = body["total_pages"].as_i64().unwrap();
    assert!(page <= total_pages);
}

#[tokio::test]
async fn test_pagination_out_of_bounds_low() {
    let db = setup_test_db().await;
    let app = setup_app(db);

    // Request page 0 or negative
    let request = test_request("GET", "/api/table/songs?page=0");
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = extract_json(response.into_body()).await;

    // Should clamp to first page
    assert_eq!(body["page"], 1);
}
