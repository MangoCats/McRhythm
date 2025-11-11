//! Integration tests for wkmp-ai API endpoints
//!
//! **[IMPL008]** Integration testing for import workflow API

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::json;
use tower::util::ServiceExt;

/// Test helper: create test app with in-memory database
async fn create_test_app() -> (axum::Router, sqlx::SqlitePool) {
    use wkmp_common::events::EventBus;

    // Create in-memory database
    let pool = sqlx::SqlitePool::connect("sqlite::memory:")
        .await
        .expect("Failed to create in-memory database");

    // Initialize database schema
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT
        );

        CREATE TABLE IF NOT EXISTS import_sessions (
            session_id TEXT PRIMARY KEY,
            state TEXT NOT NULL,
            root_folder TEXT NOT NULL,
            parameters TEXT NOT NULL,
            progress_current INTEGER NOT NULL,
            progress_total INTEGER NOT NULL,
            progress_percentage REAL NOT NULL,
            current_operation TEXT NOT NULL,
            errors TEXT NOT NULL,
            started_at TEXT NOT NULL,
            ended_at TEXT
        );
        "#
    )
    .execute(&pool)
    .await
    .expect("Failed to initialize database schema");

    // Create event bus
    let event_bus = EventBus::new(100);

    // Create import event channel
    let (import_event_tx, _) = tokio::sync::broadcast::channel(100);

    // Create app state
    let state = wkmp_ai::AppState::new(pool.clone(), event_bus, import_event_tx);

    // Build router (wkmp-ai needs to have a lib.rs for this to work, or use main module)
    // Since wkmp-ai is a binary, we need to expose AppState and build_router
    let app = wkmp_ai::build_router(state);

    (app, pool)
}

/// Test helper: create temporary test directory with audio files
fn create_test_audio_files() -> tempfile::TempDir {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    // Create fake MP3 file
    let mp3_path = temp_dir.path().join("test.mp3");
    std::fs::write(&mp3_path, b"ID3\x03\x00\x00\x00\x00\x00\x00fake_mp3_data")
        .expect("Failed to write test MP3");

    temp_dir
}

#[tokio::test]
async fn test_health_endpoint() {
    let (app, _pool) = create_test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["status"], "ok");
    assert_eq!(json["module"], "wkmp-ai");
}

#[tokio::test]
async fn test_import_start_success() {
    let (app, _pool) = create_test_app().await;
    let temp_dir = create_test_audio_files();

    let request_body = json!({
        "root_folder": temp_dir.path().to_str().unwrap()
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/import/start")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(json["session_id"].is_string());
    assert_eq!(json["state"], "SCANNING");
    assert!(json["started_at"].is_string());
}

#[tokio::test]
async fn test_import_start_nonexistent_folder() {
    let (app, _pool) = create_test_app().await;

    let request_body = json!({
        "root_folder": "/nonexistent/path/that/does/not/exist"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/import/start")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_import_start_file_not_directory() {
    let (app, _pool) = create_test_app().await;
    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    std::fs::write(&file_path, b"test").unwrap();

    let request_body = json!({
        "root_folder": file_path.to_str().unwrap()
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/import/start")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_import_status_not_found() {
    let (app, _pool) = create_test_app().await;
    let fake_session_id = "00000000-0000-0000-0000-000000000000";

    let response = app
        .oneshot(
            Request::builder()
                .uri(&format!("/import/status/{}", fake_session_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_import_cancel_not_found() {
    let (app, _pool) = create_test_app().await;
    let fake_session_id = "00000000-0000-0000-0000-000000000000";

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&format!("/import/cancel/{}", fake_session_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_sse_endpoint_connection() {
    let (app, _pool) = create_test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/import/events")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        "text/event-stream"
    );
}

/// REQ-TD-003: Verify amplitude analysis endpoint has been removed (deferred to future release)
#[tokio::test]
async fn test_amplitude_analysis_endpoint_removed() {
    let (app, _pool) = create_test_app().await;
    let temp_dir = create_test_audio_files();
    let file_path = temp_dir.path().join("test.mp3");

    let request_body = json!({
        "file_path": file_path.to_str().unwrap(),
        "start_time": 0.0,
        "end_time": 10.0
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/analyze/amplitude")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // REQ-TD-003: Endpoint should return 404 (removed, not implemented)
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_parameters_get_and_update() {
    let (app, pool) = create_test_app().await;

    // Insert test parameter
    sqlx::query("INSERT INTO settings (key, value) VALUES ('global_params', '{\"min_silence_duration\":0.5}')")
        .execute(&pool)
        .await
        .unwrap();

    // Test GET /parameters/global
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/parameters/global")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    // Test POST /parameters/global
    let update_body = json!({
        "min_silence_duration": 0.3,
        "silence_threshold": -40.0
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/parameters/global")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&update_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
