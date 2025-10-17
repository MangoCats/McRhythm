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
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio;
use uuid::Uuid;

// Import the application modules
use wkmp_ap::api::{create_router, AppState};
use wkmp_ap::playback::engine::{EnqueueRequest, PlaybackEngine};

/// Test helper to create a test server
async fn setup_test_server() -> (axum::Router, Arc<PlaybackEngine>) {
    let engine = Arc::new(
        PlaybackEngine::new(PathBuf::from("/tmp/test-music"))
            .await
            .expect("Failed to create engine")
    );

    let app_state = AppState {
        engine: Arc::clone(&engine),
        root_folder: "/tmp/test-music".to_string(),
        port: 5740,
    };

    let router = create_router(app_state);
    (router, engine)
}

/// Helper function to make HTTP requests to the test server
async fn make_request(
    app: &axum::Router,
    method: &str,
    path: &str,
    body: Option<Value>,
) -> (StatusCode, Option<Value>) {
    use tower::ServiceExt;
    use axum::body::Body;
    use http::{Request, Method};

    let method = match method {
        "GET" => Method::GET,
        "POST" => Method::POST,
        "DELETE" => Method::DELETE,
        _ => panic!("Unsupported method"),
    };

    let mut request = Request::builder()
        .method(method)
        .uri(path);

    if let Some(json_body) = body {
        request = request
            .header("content-type", "application/json");
    }

    let request = if let Some(json_body) = body {
        request.body(Body::from(json_body.to_string())).unwrap()
    } else {
        request.body(Body::empty()).unwrap()
    };

    let response = app.clone()
        .oneshot(request)
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
    let (app, _) = setup_test_server().await;

    let (status, body) = make_request(&app, "GET", "/health", None).await;

    assert_eq!(status, StatusCode::OK);
    let body = body.expect("Expected response body");
    assert_eq!(body["status"], "ok");
    assert_eq!(body["module"], "wkmp-ap");
    assert!(body["version"].is_string());
}

#[tokio::test]
async fn test_playback_state_endpoints() {
    let (app, _) = setup_test_server().await;

    // Check initial state (should be paused)
    let (status, body) = make_request(&app, "GET", "/api/v1/playback/state", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.unwrap()["state"], "paused");

    // Start playback
    let (status, body) = make_request(&app, "POST", "/api/v1/playback/play", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.unwrap()["state"], "playing");

    // Verify state changed
    let (status, body) = make_request(&app, "GET", "/api/v1/playback/state", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.unwrap()["state"], "playing");

    // Pause playback
    let (status, body) = make_request(&app, "POST", "/api/v1/playback/pause", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.unwrap()["state"], "paused");

    // Verify state changed back
    let (status, body) = make_request(&app, "GET", "/api/v1/playback/state", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.unwrap()["state"], "paused");
}

#[tokio::test]
async fn test_playback_position() {
    let (app, _) = setup_test_server().await;

    let (status, body) = make_request(&app, "GET", "/api/v1/playback/position", None).await;

    assert_eq!(status, StatusCode::OK);
    let body = body.unwrap();
    assert!(body["position_ms"].is_number());
    assert!(body["duration_ms"].is_number());
    assert!(body["state"].is_string());
    assert!(body.get("passage_id").is_some()); // Can be null
}

#[tokio::test]
async fn test_queue_management() {
    let (app, _) = setup_test_server().await;

    // Check initial queue (should be empty)
    let (status, body) = make_request(&app, "GET", "/api/v1/playback/queue", None).await;
    assert_eq!(status, StatusCode::OK);
    let queue = body.unwrap()["queue"].as_array().unwrap();
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
        "/api/v1/playback/enqueue",
        Some(enqueue_request)
    ).await;

    assert_eq!(status, StatusCode::OK);
    let body = body.unwrap();
    assert_eq!(body["status"], "ok");
    assert!(body["queue_entry_id"].is_string());
    assert!(body["play_order"].is_number());

    // Check queue now has one item
    let (status, body) = make_request(&app, "GET", "/api/v1/playback/queue", None).await;
    assert_eq!(status, StatusCode::OK);
    let queue = body.unwrap()["queue"].as_array().unwrap();
    assert_eq!(queue.len(), 1);
    assert_eq!(queue[0]["file_path"], "test.mp3");
}

#[tokio::test]
async fn test_audio_devices() {
    let (app, _) = setup_test_server().await;

    // List devices
    let (status, body) = make_request(&app, "GET", "/api/v1/audio/devices", None).await;
    assert_eq!(status, StatusCode::OK);
    let body = body.unwrap();
    let devices = body["devices"].as_array().unwrap();
    assert!(devices.len() > 0);
    assert_eq!(devices[0]["id"], "default");
    assert_eq!(devices[0]["name"], "System Default");

    // Get current device
    let (status, body) = make_request(&app, "GET", "/api/v1/audio/device", None).await;
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
        "/api/v1/audio/device",
        Some(set_request)
    ).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body.unwrap()["status"], "ok");
}

#[tokio::test]
async fn test_volume_control() {
    let (app, _) = setup_test_server().await;

    // Get volume
    let (status, body) = make_request(&app, "GET", "/api/v1/audio/volume", None).await;
    assert_eq!(status, StatusCode::OK);
    let body = body.unwrap();
    assert!(body["volume"].is_number());

    // Set volume
    let set_request = json!({
        "volume": 80
    });
    let (status, body) = make_request(
        &app,
        "POST",
        "/api/v1/audio/volume",
        Some(set_request)
    ).await;
    assert_eq!(status, StatusCode::OK);
    let body = body.unwrap();
    assert_eq!(body["status"], "ok");
    assert_eq!(body["volume"], 80);

    // Test invalid volume (> 100)
    let invalid_request = json!({
        "volume": 150
    });
    let (status, _) = make_request(
        &app,
        "POST",
        "/api/v1/audio/volume",
        Some(invalid_request)
    ).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_invalid_endpoints() {
    let (app, _) = setup_test_server().await;

    // Test non-existent endpoint
    let (status, _) = make_request(&app, "GET", "/api/v1/nonexistent", None).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Test wrong method
    let (status, _) = make_request(&app, "GET", "/api/v1/playback/play", None).await;
    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn test_concurrent_state_changes() {
    let (app, engine) = setup_test_server().await;

    // Start multiple play/pause operations concurrently
    let handles: Vec<_> = (0..10)
        .map(|i| {
            let app = app.clone();
            tokio::spawn(async move {
                if i % 2 == 0 {
                    make_request(&app, "POST", "/api/v1/playback/play", None).await
                } else {
                    make_request(&app, "POST", "/api/v1/playback/pause", None).await
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
    let (status, body) = make_request(&app, "GET", "/api/v1/playback/state", None).await;
    assert_eq!(status, StatusCode::OK);
    let state = body.unwrap()["state"].as_str().unwrap();
    assert!(state == "playing" || state == "paused");

    // Verify engine state matches API response
    let engine_state = engine.get_state().await;
    let expected = match engine_state {
        wkmp_ap::playback::engine::PlaybackState::Playing => "playing",
        wkmp_ap::playback::engine::PlaybackState::Paused => "paused",
    };
    assert_eq!(state, expected);
}

#[tokio::test]
async fn test_queue_ordering() {
    let (app, _) = setup_test_server().await;

    // Add multiple tracks to queue
    for i in 0..5 {
        let request = json!({
            "file_path": format!("track_{}.mp3", i),
        });

        let (status, _) = make_request(
            &app,
            "POST",
            "/api/v1/playback/enqueue",
            Some(request)
        ).await;
        assert_eq!(status, StatusCode::OK);
    }

    // Verify queue order
    let (status, body) = make_request(&app, "GET", "/api/v1/playback/queue", None).await;
    assert_eq!(status, StatusCode::OK);
    let queue = body.unwrap()["queue"].as_array().unwrap();
    assert_eq!(queue.len(), 5);

    // Check play_order is increasing
    let mut prev_order = 0;
    for item in queue {
        let play_order = item["play_order"].as_u64().unwrap();
        assert!(play_order > prev_order);
        prev_order = play_order;
    }
}