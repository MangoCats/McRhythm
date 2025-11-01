//! Health check endpoint
//!
//! [REQ-DR-NF-040]: Health endpoint responding within 2 seconds

use axum::{routing::get, Json, Router};
use serde::Serialize;

use crate::AppState;

/// Health check response [REQ-DR-NF-040]
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub module: String,
}

/// GET /health
///
/// Health check endpoint for monitoring.
/// Does NOT require authentication per REQ-DR-NF-040.
pub async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        module: "wkmp-dr".to_string(),
    })
}

/// Build health check routes
pub fn health_routes() -> Router<AppState> {
    Router::new().route("/health", get(health_check))
}
