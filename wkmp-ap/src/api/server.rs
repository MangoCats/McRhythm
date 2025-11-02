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
///
/// **Note:** AppContext implements Clone, which gives us `FromRef<AppContext>` for free
/// via Axum's blanket implementation. This allows custom extractors to access state.
#[derive(Clone)]
pub struct AppContext {
    pub state: Arc<SharedState>,
    pub engine: Arc<PlaybackEngine>,
    pub db_pool: Pool<Sqlite>,
    /// Shared volume control Arc (synchronized with AudioOutput)
    /// **[ARCH-VOL-020]** Direct access for API handlers
    pub volume: Arc<Mutex<f32>>,
    /// Shared secret for API authentication
    /// Loaded once at startup to avoid async issues with thread_rng()
    /// Per SPEC007 API-AUTH-026: Value of 0 means authentication disabled
    pub shared_secret: i64,
}

/// Run HTTP API server
///
/// **Arguments:**
/// - `config`: Application configuration
/// - `state`: Shared playback state
/// - `engine`: Playback engine reference
/// - `db_pool`: Database connection pool
/// - `shared_secret`: API authentication secret (loaded from database)
///
/// **Traceability:** API Design - Port 5721 (configurable)
pub async fn run(
    config: Config,
    state: Arc<SharedState>,
    engine: Arc<PlaybackEngine>,
    db_pool: Pool<Sqlite>,
    shared_secret: i64,
) -> Result<()> {
    // Shared secret passed as parameter (loaded in main.rs before spawning)
    // Per SPEC007 API-AUTH-028-A: shared_secret must be embedded in served HTML
    // Per SPEC007 API-AUTH-026: Value of 0 means authentication disabled

    // [ARCH-VOL-020] Get shared volume Arc from engine
    let volume = engine.get_volume_arc();

    // Create application context with shared secret
    let ctx = AppContext {
        state,
        engine,
        db_pool,
        volume,
        shared_secret,
    };

    // Prepare HTML with embedded secret
    let html_template = include_str!("developer_ui.html");
    let html_with_secret = html_template.replace("{{SHARED_SECRET}}", &shared_secret.to_string());

    // Build router with all routes
    // Authentication uses body reconstruction pattern for POST/PUT support
    // Reference: axum/examples/consume-body-in-extractor-or-middleware
    let app = Router::new()
        // Developer UI (HTML serving - embedded shared_secret)
        .route("/", get(|| async move { axum::response::Html(html_with_secret.clone()) }))

        // Health endpoint
        .route("/health", get(super::handlers::health))

        // Audio device management
        .route("/audio/devices", get(super::handlers::list_audio_devices))
        .route("/audio/device", get(super::handlers::get_audio_device))
        .route("/audio/device", post(super::handlers::set_audio_device))
        .route("/audio/volume", get(super::handlers::get_volume))
        .route("/audio/volume", post(super::handlers::set_volume))

        // Playback control
        .route("/playback/enqueue", post(super::handlers::enqueue_passage))
        .route("/playback/enqueue-folder", post(super::handlers::enqueue_folder))
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
        .route("/playback/callback_stats", get(super::handlers::get_callback_stats))

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

        // Attach application context
        .with_state(ctx)

        // Apply authentication layer (Tower pattern for Axum 0.7 compatibility)
        // This validates timestamp + hash for all requests except "/"
        // Per SPEC007 API-AUTH-025: All API requests must include timestamp + hash
        // Per SPEC007 API-AUTH-026: Authentication can be disabled by setting shared_secret = 0
        .layer(super::auth_middleware::AuthLayer { shared_secret })

        // Enable CORS for local access
        .layer(CorsLayer::permissive());

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
