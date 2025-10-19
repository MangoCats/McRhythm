//! Integration tests for the WKMP Audio Player API
//!
//! Tests the complete API surface including:
//! - Health checks
//! - Playback control
//! - Queue management
//! - Audio device management
//!
//! Implements requirements from api_design.md - Audio Player API

use axum::http::StatusCode;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio;
use uuid::Uuid;

// Import the application modules
use wkmp_ap::api::server::AppContext;
use wkmp_ap::playback::engine::PlaybackEngine;
use wkmp_ap::state::{PlaybackState, SharedState};
use sqlx::sqlite::SqlitePoolOptions;

/// Test helper to create a test server
async fn setup_test_server() -> (axum::Router, Arc<SharedState>, Arc<PlaybackEngine>) {
    use axum::{Router, routing::{get, post, delete}};

    // Create in-memory test database
    let db_pool = SqlitePoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .expect("Failed to create test database");

    // Create minimal database schema for tests
    sqlx::query(
        r#"
        CREATE TABLE settings (
            key TEXT PRIMARY KEY,
            value TEXT
        )
        "#,
    )
    .execute(&db_pool)
    .await
    .expect("Failed to create settings table");

    sqlx::query(
        r#"
        CREATE TABLE queue (
            guid TEXT PRIMARY KEY,
            file_path TEXT NOT NULL,
            passage_guid TEXT,
            play_order INTEGER NOT NULL,
            start_time_ms INTEGER,
            end_time_ms INTEGER,
            lead_in_point_ms INTEGER,
            lead_out_point_ms INTEGER,
            fade_in_point_ms INTEGER,
            fade_out_point_ms INTEGER,
            fade_in_curve TEXT,
            fade_out_curve TEXT
        )
        "#,
    )
    .execute(&db_pool)
    .await
    .expect("Failed to create queue table");

    // Create shared state
    let state = Arc::new(SharedState::new());

    // Create playback engine
    let engine = Arc::new(
        PlaybackEngine::new(db_pool.clone(), Arc::clone(&state))
            .await
            .expect("Failed to create engine")
    );

    // Create application context
    // [ARCH-VOL-020] Get shared volume Arc from engine
    let volume = engine.get_volume_arc();
    let ctx = AppContext {
        state: Arc::clone(&state),
        engine: Arc::clone(&engine),
        db_pool: db_pool.clone(),
        volume,
    };

    // Build router with same structure as server.rs
    let router = Router::new()
        .route("/health", get(wkmp_ap::api::handlers::health))
        .route("/audio/volume", get(wkmp_ap::api::handlers::get_volume))
        .route("/audio/volume", post(wkmp_ap::api::handlers::set_volume))
        .route("/audio/devices", get(wkmp_ap::api::handlers::list_audio_devices))
        .route("/audio/device", get(wkmp_ap::api::handlers::get_audio_device))
        .route("/audio/device", post(wkmp_ap::api::handlers::set_audio_device))
        .route("/playback/enqueue", post(wkmp_ap::api::handlers::enqueue_passage))
        .route("/playback/queue/:queue_entry_id", delete(wkmp_ap::api::handlers::remove_from_queue))
        .route("/playback/queue/clear", post(wkmp_ap::api::handlers::clear_queue))
        .route("/playback/queue/reorder", post(wkmp_ap::api::handlers::reorder_queue_entry))
        .route("/playback/play", post(wkmp_ap::api::handlers::play))
        .route("/playback/pause", post(wkmp_ap::api::handlers::pause))
        .route("/playback/next", post(wkmp_ap::api::handlers::skip_next))
        .route("/playback/previous", post(wkmp_ap::api::handlers::skip_previous))
        .route("/playback/seek", post(wkmp_ap::api::handlers::seek))
        .route("/playback/queue", get(wkmp_ap::api::handlers::get_queue))
        .route("/playback/state", get(wkmp_ap::api::handlers::get_playback_state))
        .route("/playback/position", get(wkmp_ap::api::handlers::get_position))
        .route("/playback/buffer_status", get(wkmp_ap::api::handlers::get_buffer_status))
        .route("/files/browse", get(wkmp_ap::api::handlers::browse_files))
        .with_state(ctx);

    (router, state, engine)
}

/// Helper function to make HTTP requests to the test server
async fn make_request(
    app: &axum::Router,
    method: &str,
    path: &str,
    body: Option<Value>,
) -> (StatusCode, Option<Value>) {
    use axum::body::Body;
    use http::{Request, Method};
    use tower::Service;

    let method = match method {
        "GET" => Method::GET,
        "POST" => Method::POST,
        "DELETE" => Method::DELETE,
        _ => panic!("Unsupported method"),
    };

    let mut request = Request::builder()
        .method(method)
        .uri(path);

    if body.is_some() {
        request = request
            .header("content-type", "application/json");
    }

    let request = if let Some(json_body) = body {
        request.body(Body::from(json_body.to_string())).unwrap()
    } else {
        request.body(Body::empty()).unwrap()
    };

    // Clone the app to get an owned instance for oneshot
    let response = app.clone()
        .call(request)
        .await
        .unwrap();

    let status = response.status();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();

    let json_body = if !body.is_empty() {
        Some(serde_json::from_slice(&body).unwrap())
    } else {
        None
    };

    (status, json_body)
}

#[tokio::test]
async fn test_health_endpoint() {
    let (app, _state, _engine) = setup_test_server().await;

    let (status, body) = make_request(&app, "GET", "/health", None).await;

    assert_eq!(status, StatusCode::OK);
    let body = body.expect("Expected response body");
    assert_eq!(body["module"], "wkmp-ap");
    assert!(body["version"].is_string());
}

#[tokio::test]
async fn test_playback_state_endpoints() {
    let (app, _state, _engine) = setup_test_server().await;

    // Check initial state (should be paused)
    let (status, body) = make_request(&app, "GET", "/playback/state", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.unwrap()["state"], "paused");

    // Start playback
    let (status, body) = make_request(&app, "POST", "/playback/play", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.unwrap()["state"], "playing");

    // Verify state changed
    let (status, body) = make_request(&app, "GET", "/playback/state", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.unwrap()["state"], "playing");

    // Pause playback
    let (status, body) = make_request(&app, "POST", "/playback/pause", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.unwrap()["state"], "paused");

    // Verify state changed back
    let (status, body) = make_request(&app, "GET", "/playback/state", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.unwrap()["state"], "paused");
}

#[tokio::test]
async fn test_playback_position() {
    let (app, _state, _engine) = setup_test_server().await;

    let (status, body) = make_request(&app, "GET", "/playback/position", None).await;

    assert_eq!(status, StatusCode::OK);
    let body = body.unwrap();
    assert!(body["position_ms"].is_number());
    assert!(body["duration_ms"].is_number());
    assert!(body["state"].is_string());
    assert!(body.get("passage_id").is_some()); // Can be null
}

#[tokio::test]
async fn test_queue_management() {
    let (app, _state, _engine) = setup_test_server().await;

    // Check initial queue (should be empty)
    let (status, body) = make_request(&app, "GET", "/playback/queue", None).await;
    assert_eq!(status, StatusCode::OK);
    let body = body.unwrap();
    let queue = body["queue"].as_array().unwrap();
    assert_eq!(queue.len(), 0);

    // Enqueue a track
    let enqueue_request = json!({
        "file_path": "test.mp3",
        "start_time_ms": 1000,
        "end_time_ms": 5000,
        "fade_in_point_ms": 1500,
        "fade_out_point_ms": 4500
    });

    let (status, body) = make_request(
        &app,
        "POST",
        "/playback/enqueue",
        Some(enqueue_request)
    ).await;

    assert_eq!(status, StatusCode::OK);
    let body = body.unwrap();
    assert!(body["queue_entry_id"].is_string());
    assert!(body["play_order"].is_number());

    // Check queue now has one item
    let (status, body) = make_request(&app, "GET", "/playback/queue", None).await;
    assert_eq!(status, StatusCode::OK);
    let body = body.unwrap();
    let queue = body["queue"].as_array().unwrap();
    assert_eq!(queue.len(), 1);
    assert_eq!(queue[0]["file_path"], "test.mp3");
}

#[tokio::test]
async fn test_audio_devices() {
    let (app, _state, _engine) = setup_test_server().await;

    // List devices
    let (status, body) = make_request(&app, "GET", "/audio/devices", None).await;
    assert_eq!(status, StatusCode::OK);
    let body = body.unwrap();
    let devices = body["devices"].as_array().unwrap();
    assert!(devices.len() > 0);
    assert_eq!(devices[0]["id"], "default");
    assert_eq!(devices[0]["name"], "System Default");

    // Get current device
    let (status, body) = make_request(&app, "GET", "/audio/device", None).await;
    assert_eq!(status, StatusCode::OK);
    let body = body.unwrap();
    assert_eq!(body["device_id"], "default");

    // Set device
    let set_request = json!({
        "device_id": "default"
    });
    let (status, body) = make_request(
        &app,
        "POST",
        "/audio/device",
        Some(set_request)
    ).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.unwrap()["status"], "ok");
}

#[tokio::test]
async fn test_volume_control() {
    let (app, _state, _engine) = setup_test_server().await;

    // Get volume
    let (status, body) = make_request(&app, "GET", "/audio/volume", None).await;
    assert_eq!(status, StatusCode::OK);
    let body = body.unwrap();
    assert!(body["volume"].is_number());

    // Set volume (using 0.0-1.0 floating-point scale)
    let set_request = json!({
        "volume": 0.80
    });
    let (status, body) = make_request(
        &app,
        "POST",
        "/audio/volume",
        Some(set_request)
    ).await;
    assert_eq!(status, StatusCode::OK);
    let body = body.unwrap();
    assert_eq!(body["volume"], 0.80);

    // Test invalid volume (> 1.0)
    let invalid_request = json!({
        "volume": 1.5
    });
    let (status, _) = make_request(
        &app,
        "POST",
        "/audio/volume",
        Some(invalid_request)
    ).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_invalid_endpoints() {
    let (app, _state, _engine) = setup_test_server().await;

    // Test non-existent endpoint
    let (status, _) = make_request(&app, "GET", "/nonexistent", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Test wrong method
    let (status, _) = make_request(&app, "GET", "/playback/play", None).await;
    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn test_concurrent_state_changes() {
    let (app, state, _engine) = setup_test_server().await;

    // Start multiple play/pause operations concurrently
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let app = app.clone();
            tokio::spawn(async move {
                if i % 2 == 0 {
                    make_request(&app, "POST", "/playback/play", None).await
                } else {
                    make_request(&app, "POST", "/playback/pause", None).await
                }
            })
        })
        .collect();

    // Wait for all operations to complete
    for handle in handles {
        let (status, _) = handle.await.unwrap();
        assert_eq!(status, StatusCode::OK);
    }

    // Verify final state is consistent
    let (status, body) = make_request(&app, "GET", "/playback/state", None).await;
    assert_eq!(status, StatusCode::OK);
    let body = body.unwrap();
    let state_str = body["state"].as_str().unwrap();
    assert!(state_str == "playing" || state_str == "paused");

    // Verify state matches API response
    let playback_state = state.get_playback_state().await;
    let expected = match playback_state {
        PlaybackState::Playing => "playing",
        PlaybackState::Paused => "paused",
    };
    assert_eq!(state_str, expected);
}

#[tokio::test]
async fn test_queue_ordering() {
    let (app, _state, _engine) = setup_test_server().await;

    // Add multiple tracks to queue
    for i in 0..5 {
        let request = json!({
            "file_path": format!("track_{}.mp3", i),
        });

        let (status, _) = make_request(
            &app,
            "POST",
            "/playback/enqueue",
            Some(request)
        ).await;
        assert_eq!(status, StatusCode::OK);
    }

    // Verify queue order
    let (status, body) = make_request(&app, "GET", "/playback/queue", None).await;
    assert_eq!(status, StatusCode::OK);
    let body = body.unwrap();
    let queue = body["queue"].as_array().unwrap();
    assert_eq!(queue.len(), 5);

    // Check play_order is increasing
    let mut prev_order = 0;
    for item in queue {
        let play_order = item["play_order"].as_u64().unwrap();
        assert!(play_order > prev_order);
        prev_order = play_order;
    }
}

// ============================================================================
// Volume API Tests (Priority 2)
// ============================================================================

#[tokio::test]
async fn test_get_volume_response_format() {
    let (app, _state, _engine) = setup_test_server().await;

    // Test GET /audio/volume returns correct JSON structure
    let (status, body) = make_request(&app, "GET", "/audio/volume", None).await;

    assert_eq!(status, StatusCode::OK);
    let body = body.expect("Expected response body");

    // Verify volume field exists and is a number
    assert!(body["volume"].is_number(), "volume should be a number");

    // Verify volume is in valid range (0.0-1.0)
    let volume = body["volume"].as_f64().expect("volume should be f64");
    assert!(volume >= 0.0 && volume <= 1.0, "volume should be between 0.0 and 1.0");
}

#[tokio::test]
async fn test_post_volume_boundary_values() {
    let (app, _state, _engine) = setup_test_server().await;

    // Test 0.0 (minimum)
    let request = json!({"volume": 0.0});
    let (status, body) = make_request(&app, "POST", "/audio/volume", Some(request)).await;
    assert_eq!(status, StatusCode::OK);
    let body = body.unwrap();
    assert_eq!(body["volume"], 0.0);

    // Verify it persisted
    let (status, body) = make_request(&app, "GET", "/audio/volume", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.unwrap()["volume"], 0.0);

    // Test 0.5 (middle)
    let request = json!({"volume": 0.5});
    let (status, body) = make_request(&app, "POST", "/audio/volume", Some(request)).await;
    assert_eq!(status, StatusCode::OK);
    let body = body.unwrap();
    assert_eq!(body["volume"], 0.5);

    // Test 1.0 (maximum)
    let request = json!({"volume": 1.0});
    let (status, body) = make_request(&app, "POST", "/audio/volume", Some(request)).await;
    assert_eq!(status, StatusCode::OK);
    let body = body.unwrap();
    assert_eq!(body["volume"], 1.0);

    // Verify it persisted
    let (status, body) = make_request(&app, "GET", "/audio/volume", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.unwrap()["volume"], 1.0);
}

#[tokio::test]
async fn test_post_volume_negative_rejection() {
    let (app, _state, _engine) = setup_test_server().await;

    // Test volume < 0.0 returns 400 Bad Request
    let request = json!({"volume": -0.5});
    let (status, _) = make_request(&app, "POST", "/audio/volume", Some(request)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // Test volume < 0.0 (edge case: -0.001)
    let request = json!({"volume": -0.001});
    let (status, _) = make_request(&app, "POST", "/audio/volume", Some(request)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // Test volume > 1.0 returns 400 Bad Request
    let request = json!({"volume": 1.001});
    let (status, _) = make_request(&app, "POST", "/audio/volume", Some(request)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    // Test volume significantly > 1.0
    let request = json!({"volume": 2.0});
    let (status, _) = make_request(&app, "POST", "/audio/volume", Some(request)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_volume_persistence_flow() {
    let (app, _state, _engine) = setup_test_server().await;

    // Set a specific volume value
    let test_volume = 0.73;
    let request = json!({"volume": test_volume});
    let (status, _) = make_request(&app, "POST", "/audio/volume", Some(request)).await;
    assert_eq!(status, StatusCode::OK);

    // Read it back via GET to verify SharedState was updated
    let (status, body) = make_request(&app, "GET", "/audio/volume", None).await;
    assert_eq!(status, StatusCode::OK);
    let body = body.unwrap();

    // Allow for floating point comparison with tolerance
    let returned_volume = body["volume"].as_f64().unwrap();
    assert!((returned_volume - test_volume).abs() < 0.0001);

    // Set another value to test persistence chain
    let test_volume2 = 0.25;
    let request = json!({"volume": test_volume2});
    let (status, _) = make_request(&app, "POST", "/audio/volume", Some(request)).await;
    assert_eq!(status, StatusCode::OK);

    // Verify the new value
    let (status, body) = make_request(&app, "GET", "/audio/volume", None).await;
    assert_eq!(status, StatusCode::OK);
    let returned_volume = body.unwrap()["volume"].as_f64().unwrap();
    assert!((returned_volume - test_volume2).abs() < 0.0001);
}

/// **[ARCH-VOL-020]** Test that VolumeChanged event is broadcast
#[tokio::test]
async fn test_volume_changed_event() {
    let (app, state, _engine) = setup_test_server().await;

    // Subscribe to events before making change
    let mut event_rx = state.subscribe_events();

    // Change volume via API
    let request = json!({"volume": 0.65});
    let (status, _) = make_request(&app, "POST", "/audio/volume", Some(request)).await;
    assert_eq!(status, StatusCode::OK);

    // Wait for event (with timeout)
    let event_result = tokio::time::timeout(
        std::time::Duration::from_millis(100),
        event_rx.recv()
    ).await;

    assert!(event_result.is_ok(), "Should receive event within timeout");
    let event = event_result.unwrap();
    assert!(event.is_ok(), "Event should not be an error");

    // Verify it's a VolumeChanged event with correct value
    let event = event.unwrap();
    match event {
        wkmp_common::events::WkmpEvent::VolumeChanged { volume, .. } => {
            assert!((volume - 0.65).abs() < 0.0001, "Event should contain correct volume");
        }
        _ => panic!("Expected VolumeChanged event, got {:?}", event),
    }
}

/// **[ARCH-VOL-020]** Test concurrent volume updates (thread safety)
#[tokio::test]
async fn test_concurrent_volume_updates() {
    let (app, _state, engine) = setup_test_server().await;

    // Get volume Arc directly from engine
    let volume_arc = engine.get_volume_arc();

    // Spawn multiple tasks updating volume concurrently
    let mut handles = vec![];

    for i in 0..10 {
        let volume_clone = Arc::clone(&volume_arc);
        let handle = tokio::spawn(async move {
            let value = (i as f32) / 10.0; // 0.0, 0.1, 0.2, ..., 0.9
            *volume_clone.lock().unwrap() = value;
            tokio::time::sleep(tokio::time::Duration::from_micros(10)).await;
            *volume_clone.lock().unwrap()
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    let results: Vec<f32> = futures::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.unwrap())
        .collect();

    // Verify no panics occurred (all tasks completed)
    assert_eq!(results.len(), 10, "All concurrent tasks should complete");

    // Verify final value is valid (one of the set values)
    let final_value = *volume_arc.lock().unwrap();
    assert!(final_value >= 0.0 && final_value <= 1.0, "Final value should be in valid range");

    // Test concurrent API updates
    let mut api_handles = vec![];
    for i in 0..5 {
        let app_clone = app.clone();
        let handle = tokio::spawn(async move {
            let value = (i as f64 + 1.0) / 10.0; // 0.1, 0.2, 0.3, 0.4, 0.5
            let request = json!({"volume": value});
            make_request(&app_clone, "POST", "/audio/volume", Some(request)).await
        });
        api_handles.push(handle);
    }

    // Wait for all API requests
    let api_results = futures::future::join_all(api_handles).await;

    // Verify all requests succeeded
    for result in api_results {
        let (status, _body) = result.unwrap();
        assert_eq!(status, StatusCode::OK, "All concurrent API requests should succeed");
    }

    // Verify final state is consistent
    let (status, body) = make_request(&app, "GET", "/audio/volume", None).await;
    assert_eq!(status, StatusCode::OK);
    let final_api_volume = body.unwrap()["volume"].as_f64().unwrap();
    assert!(final_api_volume >= 0.0 && final_api_volume <= 1.0, "Final API volume should be valid");
}