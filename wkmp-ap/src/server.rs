//! HTTP server for wkmp-ap

use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use serde_json::json;
use sqlx::SqlitePool;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::info;

/// Application state
#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub root_folder: PathBuf,
}

/// Start the HTTP server
pub async fn start(bind_addr: &str, db: SqlitePool, root_folder: PathBuf) -> anyhow::Result<()> {
    let state = Arc::new(AppState { db, root_folder });

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/status", get(status))
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
    Json(json!({
        "service": "wkmp-ap",
        "version": env!("CARGO_PKG_VERSION"),
        "status": "running",
        "root_folder": state.root_folder.display().to_string(),
    }))
}
