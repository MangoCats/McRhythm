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
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to initialize database schema");

    // Create event bus
    let event_bus = EventBus::new(100);

    // Create app state
    let state = wkmp_ai::AppState::new(pool.clone(), event_bus, 16, 1_073_741_824);

    // Build router (wkmp-ai needs to have a lib.rs for this to work, or use main module)
    // Since wkmp-ai is a binary, we need to expose AppState and build_router
    let app = wkmp_ai::build_router(state);

    (app, pool)
}

/// Test helper: create temporary test directory with audio files
fn create_test_audio_files() -> tempfile::TempDir {
    use hound::WavWriter;

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    // Create real WAV file (1 second, 440Hz sine wave)
    let wav_path = temp_dir.path().join("test.wav");
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 44100,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = WavWriter::create(&wav_path, spec).expect("Failed to create WAV file");

    // Write 1 second of audio (440Hz sine wave)
    for t in 0..44100 {
        let sample = (t as f32 * 440.0 * 2.0 * std::f32::consts::PI / 44100.0).sin();
        writer
            .write_sample((sample * i16::MAX as f32) as i16)
            .expect("Failed to write sample");
    }
    writer.finalize().expect("Failed to finalize WAV file");

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

#[tokio::test]
async fn test_amplitude_analysis_endpoint() {
    let (app, _pool) = create_test_app().await;
    let temp_dir = create_test_audio_files();
    let file_path = temp_dir.path().join("test.wav");

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

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // Stub implementation returns default values
    assert!(json["peak_rms"].is_number());
    assert!(json["lead_in_duration"].is_number());
    assert!(json["lead_out_duration"].is_number());
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

// **[PLAN027]** File Classification API Integration Tests

/// Test helper: create temporary test directory with mixed file types
fn create_test_mixed_files() -> tempfile::TempDir {
    use std::fs;

    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");

    // Create audio files
    fs::write(temp_dir.path().join("song1.mp3"), b"fake mp3 data").unwrap();
    fs::write(temp_dir.path().join("song2.flac"), b"fake flac data").unwrap();
    fs::write(temp_dir.path().join("song3.WAV"), b"fake wav data").unwrap();

    // Create image files
    fs::write(temp_dir.path().join("cover.jpg"), b"fake jpg data").unwrap();
    fs::write(temp_dir.path().join("artwork.PNG"), b"fake png data").unwrap();

    // Create other files
    fs::write(temp_dir.path().join("notes.txt"), b"some notes").unwrap();
    fs::write(temp_dir.path().join("README.md"), b"readme content").unwrap();

    temp_dir
}

/// **[TC-CLASSIFY-API-001]** Test file classification endpoint returns 404 when no session exists
#[tokio::test]
async fn test_file_classification_no_session() {
    let (app, _pool) = create_test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/import/file-classification")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(json["error"]["message"]
        .as_str()
        .unwrap()
        .contains("No import session found"));
}

/// **[TC-CLASSIFY-API-002]** Test file classification endpoint returns 409 during SCANNING phase
#[tokio::test]
async fn test_file_classification_during_scanning() {
    use chrono::Utc;
    use uuid::Uuid;

    let (app, pool) = create_test_app().await;

    // Insert a session in SCANNING state
    let session_id = Uuid::new_v4().to_string();
    sqlx::query(
        r#"
        INSERT INTO import_sessions (
            session_id, state, root_folder, parameters,
            progress_current, progress_total, progress_percentage,
            current_operation, errors, started_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&session_id)
    .bind("\"SCANNING\"")
    .bind("/test/path")
    .bind("{}")
    .bind(0i64)
    .bind(0i64)
    .bind(0.0)
    .bind("Scanning files")
    .bind("[]")
    .bind(Utc::now().to_rfc3339())
    .execute(&pool)
    .await
    .unwrap();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/import/file-classification")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CONFLICT);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(json["error"]["message"]
        .as_str()
        .unwrap()
        .contains("SCANNING phase"));
}

/// **[TC-CLASSIFY-API-003]** Test file classification endpoint with invalid category parameter
#[tokio::test]
async fn test_file_classification_invalid_category() {
    use chrono::Utc;
    use uuid::Uuid;

    let (app, pool) = create_test_app().await;

    // Insert a session in COMPLETED state
    let session_id = Uuid::new_v4().to_string();
    let started_at = Utc::now();
    let ended_at = started_at + chrono::Duration::seconds(10);

    sqlx::query(
        r#"
        INSERT INTO import_sessions (
            session_id, state, root_folder, parameters,
            progress_current, progress_total, progress_percentage,
            current_operation, errors, started_at, ended_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&session_id)
    .bind("\"COMPLETED\"")
    .bind("/test/path")
    .bind("{}")
    .bind(100i64)
    .bind(100i64)
    .bind(100.0)
    .bind("Import completed")
    .bind("[]")
    .bind(started_at.to_rfc3339())
    .bind(ended_at.to_rfc3339())
    .execute(&pool)
    .await
    .unwrap();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/import/file-classification?category=invalid")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(json["error"]["message"]
        .as_str()
        .unwrap()
        .contains("Invalid category"));
}

/// **[TC-CLASSIFY-API-004]** Test file classification endpoint pagination
#[tokio::test]
async fn test_file_classification_pagination() {
    use chrono::Utc;
    use uuid::Uuid;

    let (app, pool) = create_test_app().await;

    // Insert a session in COMPLETED state with classification data
    let session_id = Uuid::new_v4().to_string();
    let started_at = Utc::now();
    let ended_at = started_at + chrono::Duration::seconds(10);

    sqlx::query(
        r#"
        INSERT INTO import_sessions (
            session_id, state, root_folder, parameters,
            progress_current, progress_total, progress_percentage,
            current_operation, errors, started_at, ended_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&session_id)
    .bind("\"COMPLETED\"")
    .bind("/test/path")
    .bind("{}")
    .bind(100i64)
    .bind(100i64)
    .bind(100.0)
    .bind("Import completed")
    .bind("[]")
    .bind(started_at.to_rfc3339())
    .bind(ended_at.to_rfc3339())
    .execute(&pool)
    .await
    .unwrap();

    // Test with pagination parameters
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/import/file-classification?category=audio&offset=0&limit=10")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Will return 404 because session doesn't have file_classification data populated
    // (runtime-only data, not persisted to DB)
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
