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

use crate::playback::PlaybackEngine;

/// Application state
#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub root_folder: PathBuf,
    pub engine: Arc<RwLock<PlaybackEngine>>,
}

/// Start the HTTP server
pub async fn start(
    bind_addr: &str,
    db: SqlitePool,
    root_folder: PathBuf,
    engine: PlaybackEngine,
) -> anyhow::Result<()> {
    let state = Arc::new(AppState {
        db,
        root_folder,
        engine: Arc::new(RwLock::new(engine)),
    });

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/status", get(status))
        .route("/playback/state", get(get_playback_state))
        .route("/playback/play", post(play))
        .route("/playback/pause", post(pause))
        .route("/playback/skip", post(skip))
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
