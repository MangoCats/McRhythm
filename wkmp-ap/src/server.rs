//! HTTP server for wkmp-ap

use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde_json::json;
use sqlx::SqlitePool;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use crate::api::{EnqueueRequest, EnqueueResponse, VolumeRequest, SeekRequest};
use crate::playback::{PlaybackEngine, start_monitoring};
use crate::sse::SseBroadcaster;

/// Application state
#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub root_folder: PathBuf,
    pub engine: Arc<RwLock<PlaybackEngine>>,
    pub sse_broadcaster: SseBroadcaster,
}

/// Start the HTTP server
pub async fn start(
    bind_addr: &str,
    db: SqlitePool,
    root_folder: PathBuf,
    mut engine: PlaybackEngine,
) -> anyhow::Result<()> {
    // Create SSE broadcaster
    let sse_broadcaster = SseBroadcaster::new(100);

    // Set broadcaster on engine
    engine.set_sse_broadcaster(sse_broadcaster.clone());

    let engine_arc = Arc::new(RwLock::new(engine));

    // Start background monitoring tasks
    start_monitoring(engine_arc.clone(), sse_broadcaster.clone());

    let state = Arc::new(AppState {
        db,
        root_folder,
        engine: engine_arc,
        sse_broadcaster,
    });

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/status", get(status))
        .route("/events", get(sse_handler))
        .route("/playback/state", get(get_playback_state))
        .route("/playback/play", post(play))
        .route("/playback/pause", post(pause))
        .route("/playback/skip", post(skip))
        .route("/playback/volume", post(set_volume))
        .route("/playback/seek", post(seek))
        .route("/playback/enqueue", post(enqueue))
        .route("/queue", get(get_queue))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    info!("HTTP server listening on {}", bind_addr);

    axum::serve(listener, app).await?;

    Ok(())
}

/// Health check endpoint
async fn health_check() -> StatusCode {
    StatusCode::OK
}

/// Status endpoint
async fn status(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let engine = state.engine.read().await;
    let playback_state = engine.get_state().await;
    let queue_size = engine.queue().size().await;

    Json(json!({
        "service": "wkmp-ap",
        "version": env!("CARGO_PKG_VERSION"),
        "status": "running",
        "root_folder": state.root_folder.display().to_string(),
        "playback_state": playback_state.to_string(),
        "queue_size": queue_size,
    }))
}

/// Get playback state
async fn get_playback_state(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let engine = state.engine.read().await;
    let playback_state = engine.get_state().await;
    let (position_ms, duration_ms) = engine.shared_state().get_position().await;
    let currently_playing = engine.shared_state().get_currently_playing().await;

    Json(json!({
        "state": playback_state.to_string(),
        "currently_playing_passage_id": currently_playing,
        "position_ms": position_ms,
        "duration_ms": duration_ms,
    }))
}

/// Play endpoint
async fn play(State(state): State<Arc<AppState>>) -> Result<StatusCode, StatusCode> {
    let mut engine = state.engine.write().await;
    engine.play().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::OK)
}

/// Pause endpoint
async fn pause(State(state): State<Arc<AppState>>) -> Result<StatusCode, StatusCode> {
    let mut engine = state.engine.write().await;
    engine.pause().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::OK)
}

/// Skip endpoint
async fn skip(State(state): State<Arc<AppState>>) -> Result<StatusCode, StatusCode> {
    let mut engine = state.engine.write().await;
    engine.skip().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::OK)
}

/// Get queue
async fn get_queue(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let engine = state.engine.read().await;
    let queue = engine.queue().get_all().await;

    Json(json!({
        "entries": queue,
        "count": queue.len(),
    }))
}

/// Enqueue endpoint
async fn enqueue(
    State(state): State<Arc<AppState>>,
    Json(req): Json<EnqueueRequest>,
) -> Result<Json<EnqueueResponse>, StatusCode> {
    let engine = state.engine.read().await;
    let queue_manager = engine.queue();

    // Resolve timing parameters using precedence rules
    let timing = queue_manager
        .resolve_timing(
            &req.file_path,
            req.passage_guid.as_deref(),
            req.start_time_ms,
            req.end_time_ms,
            req.lead_in_point_ms,
            req.lead_out_point_ms,
            req.fade_in_point_ms,
            req.fade_out_point_ms,
            req.fade_in_curve.as_deref(),
            req.fade_out_curve.as_deref(),
        )
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    // Enqueue with resolved timing
    let guid = queue_manager
        .enqueue(
            req.file_path,
            req.passage_guid,
            Some(timing.start_time_ms),
            Some(timing.end_time_ms),
            Some(timing.lead_in_point_ms),
            Some(timing.lead_out_point_ms),
            Some(timing.fade_in_point_ms),
            Some(timing.fade_out_point_ms),
            Some(timing.fade_in_curve),
            Some(timing.fade_out_curve),
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let position = queue_manager.size().await;

    Ok(Json(EnqueueResponse { guid, position }))
}

/// Set volume endpoint
async fn set_volume(
    State(state): State<Arc<AppState>>,
    Json(req): Json<VolumeRequest>,
) -> Result<StatusCode, StatusCode> {
    // Convert from 0-100 (user-facing) to 0.0-1.0 (internal)
    let volume = (req.volume.clamp(0, 100) as f64) / 100.0;

    let engine = state.engine.read().await;
    engine
        .set_volume(volume)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::OK)
}

/// Seek endpoint
async fn seek(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SeekRequest>,
) -> Result<StatusCode, StatusCode> {
    let engine = state.engine.read().await;
    engine
        .seek(req.position_ms)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::OK)
}

/// SSE endpoint for real-time event streaming
async fn sse_handler(
    State(state): State<Arc<AppState>>,
) -> impl axum::response::IntoResponse {
    state.sse_broadcaster.handle_sse_connection()
}
