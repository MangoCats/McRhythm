//! Build information API endpoint
//!
//! Provides version and build metadata for display in UI

use axum::{response::Json, extract::State};
use serde::Serialize;

use crate::AppState;

/// Build information response
#[derive(Debug, Serialize)]
pub struct BuildInfo {
    pub version: String,
    pub git_hash: String,
    pub build_timestamp: String,
    pub build_profile: String,
}

/// GET /api/buildinfo
///
/// Returns build identification information for UI display
pub async fn get_build_info(State(_state): State<AppState>) -> Json<BuildInfo> {
    Json(BuildInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        git_hash: env!("GIT_HASH").to_string(),
        build_timestamp: env!("BUILD_TIMESTAMP").to_string(),
        build_profile: env!("BUILD_PROFILE").to_string(),
    })
}
