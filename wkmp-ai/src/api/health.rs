//! Health check endpoint
//!
//! **[HIGH-005]** Real uptime tracking and diagnostics

use axum::{extract::State, routing::get, Json, Router};
use chrono::Utc;
use serde::Serialize;

use crate::AppState;

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    /// Service status (e.g., "ok", "degraded", "error")
    pub status: String,
    /// Module name ("wkmp-ai")
    pub module: String,
    /// Crate version from Cargo.toml
    pub version: String,
    /// Seconds since service started
    pub uptime_seconds: u64,
    /// Last error message if any (for diagnostics)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

/// GET /health
///
/// Health check endpoint for monitoring.
/// **[HIGH-005]** Returns real uptime and last error for diagnostics
pub async fn health_check(State(state): State<AppState>) -> Json<HealthResponse> {
    // Calculate uptime from startup timestamp
    let uptime = Utc::now().signed_duration_since(state.startup_time);
    let uptime_seconds = uptime.num_seconds().max(0) as u64;

    // Get last error if any
    let last_error = state.last_error.read().await.clone();

    Json(HealthResponse {
        status: "ok".to_string(),
        module: "wkmp-ai".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds,
        last_error,
    })
}

/// Build health check routes
pub fn health_routes() -> Router<AppState> {
    Router::new().route("/health", get(health_check))
}
