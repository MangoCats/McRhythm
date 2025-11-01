//! Health check endpoint
//!
//! [REQ-DR-NF-040]: Health endpoint responding within 2 seconds

use axum::{routing::get, Json, Router};
use serde::Serialize;

use crate::AppState;

/// Health check response [REQ-DR-NF-040]
/// Per [DR-API-010], returns status, module name, and version
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub module: String,
    pub version: String,
}

/// GET /health
///
/// Health check endpoint for monitoring.
/// Does NOT require authentication per REQ-DR-NF-040.
pub async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        module: "wkmp-dr".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Build health check routes
pub fn health_routes() -> Router<AppState> {
    Router::new().route("/health", get(health_check))
}
