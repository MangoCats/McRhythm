//! REST API implementation for Audio Player
//!
//! Implements the Audio Player API endpoints as specified in api_design.md

pub mod handlers;

use axum::{
    Router,
    routing::{get, post, delete},
    extract::State,
    response::Json,
};
use std::sync::Arc;
use serde_json::json;

use crate::playback::engine::PlaybackEngine;

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    /// Playback engine
    pub engine: Arc<PlaybackEngine>,
    /// Root folder path
    pub root_folder: String,
    /// Server port
    pub port: u16,
}

/// Create the API router
pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Health check (no prefix for health endpoint)
        .route("/health", get(health_check))

        // API v1 routes
        .nest("/api/v1", Router::new()
            // Audio device endpoints
            .route("/audio/devices", get(handlers::get_audio_devices))
            .route("/audio/device", get(handlers::get_current_device))
            .route("/audio/device", post(handlers::set_audio_device))

            // Volume endpoints
            .route("/audio/volume", get(handlers::get_volume))
            .route("/audio/volume", post(handlers::set_volume))

            // Playback control endpoints
            .route("/playback/play", post(handlers::play))
            .route("/playback/pause", post(handlers::pause))
            .route("/playback/status", get(handlers::get_state))  // Also map status to state
            .route("/playback/state", get(handlers::get_state))
            .route("/playback/position", get(handlers::get_position))

            // Queue management endpoints
            .route("/playback/queue", get(handlers::get_queue))
            .route("/playback/enqueue", post(handlers::enqueue))
            .route("/playback/queue/:passage_id", delete(handlers::dequeue))

            // SSE events
            .route("/events", get(handlers::sse_handler))
        )
        .with_state(state)
}

/// Health check endpoint
async fn health_check(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok",
        "module": "wkmp-ap",
        "version": env!("CARGO_PKG_VERSION"),
        "port": state.port,
        "root_folder": state.root_folder
    }))
}