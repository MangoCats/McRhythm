//! Health check endpoint

use axum::{routing::get, Json, Router};
use serde::Serialize;

use crate::AppState;

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub module: String,
    pub version: String,
    pub uptime_seconds: u64,
}

/// GET /health
///
/// Health check endpoint for monitoring.
pub async fn health_check() -> Json<HealthResponse> {
    // TODO: Track actual uptime
    Json(HealthResponse {
        status: "ok".to_string(),
        module: "wkmp-ai".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: 0, // Stub
    })
}

/// Build health check routes
pub fn health_routes() -> Router<AppState> {
    Router::new().route("/health", get(health_check))
}
