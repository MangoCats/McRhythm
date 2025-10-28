//! HTTP server setup and routing
//!
//! Sets up Axum HTTP server with routes for control endpoints and SSE.
//!
//! **Traceability:**
//! - API Design - Audio Player API (Base URL: http://localhost:5721)
//! - CO-144 (Tokio channel types)

use crate::config::Config;
use crate::error::{Error, Result};
use crate::playback::engine::PlaybackEngine;
use crate::state::SharedState;
use axum::{
    Router,
    routing::{get, post, delete},
};
use sqlx::{Pool, Sqlite};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tower_http::cors::CorsLayer;
use tracing::info;

/// Shared application context passed to all handlers
#[derive(Clone)]
pub struct AppContext {
    pub state: Arc<SharedState>,
    pub engine: Arc<PlaybackEngine>,
    pub db_pool: Pool<Sqlite>,
    /// Shared volume control Arc (synchronized with AudioOutput)
    /// **[ARCH-VOL-020]** Direct access for API handlers
    pub volume: Arc<Mutex<f32>>,
}

/// Run HTTP API server
///
/// **Arguments:**
/// - `config`: Application configuration
/// - `state`: Shared playback state
/// - `engine`: Playback engine reference
/// - `db_pool`: Database connection pool
///
/// **Traceability:** API Design - Port 5721 (configurable)
pub async fn run(config: Config, state: Arc<SharedState>, engine: Arc<PlaybackEngine>, db_pool: Pool<Sqlite>) -> Result<()> {
    // [ARCH-VOL-020] Get shared volume Arc from engine
    let volume = engine.get_volume_arc();
    let ctx = AppContext { state, engine, db_pool, volume };
    // Build router with all endpoints
    let app = Router::new()
        // Developer UI (served at root)
        .route("/", get(super::handlers::developer_ui))

        // Health endpoint (required for all modules)
        .route("/health", get(super::handlers::health))

        // Audio device management
        .route("/audio/devices", get(super::handlers::list_audio_devices))
        .route("/audio/device", get(super::handlers::get_audio_device))
        .route("/audio/device", post(super::handlers::set_audio_device))
        .route("/audio/volume", get(super::handlers::get_volume))
        .route("/audio/volume", post(super::handlers::set_volume))

        // Playback control
        .route("/playback/enqueue", post(super::handlers::enqueue_passage))
        .route("/playback/queue/:queue_entry_id", delete(super::handlers::remove_from_queue))
        .route("/playback/queue/clear", post(super::handlers::clear_queue))
        .route("/playback/queue/reorder", post(super::handlers::reorder_queue_entry))
        .route("/playback/play", post(super::handlers::play))
        .route("/playback/pause", post(super::handlers::pause))
        .route("/playback/next", post(super::handlers::skip_next))
        .route("/playback/previous", post(super::handlers::skip_previous))
        .route("/playback/seek", post(super::handlers::seek))
        .route("/playback/queue", get(super::handlers::get_queue))
        .route("/playback/state", get(super::handlers::get_playback_state))
        .route("/playback/position", get(super::handlers::get_position))
        .route("/playback/buffer_status", get(super::handlers::get_buffer_status))
        .route("/playback/buffer_chains", get(super::handlers::get_buffer_chains))
        .route("/playback/buffer_monitor/rate", post(super::handlers::set_buffer_monitor_rate))
        .route("/playback/buffer_monitor/update", post(super::handlers::trigger_buffer_monitor_update))

        // Pipeline diagnostics
        .route("/playback/diagnostics", get(super::handlers::get_pipeline_diagnostics))

        // SSE event stream
        .route("/events", get(super::sse::event_stream))

        // Build information
        .route("/build_info", get(super::handlers::get_build_info))

        // File browser
        .route("/files/browse", get(super::handlers::browse_files))

        // Settings management
        .route("/settings/all", get(super::handlers::get_all_settings))
        .route("/settings/bulk_update", post(super::handlers::bulk_update_settings))

        // Enable CORS for local access
        .layer(CorsLayer::permissive())

        // Attach application context
        .with_state(ctx);

    // Bind to configured port
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    info!("Starting HTTP server on {}", addr);

    // Start server
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| Error::Http(format!("Failed to bind to {}: {}", addr, e)))?;

    axum::serve(listener, app)
        .await
        .map_err(|e| Error::Http(format!("Server error: {}", e)))?;

    Ok(())
}
