//! Settings API endpoint
//!
//! **Traceability:** [APIK-UI-010], [APIK-UI-020], [APIK-UI-030]
//!
//! Provides POST /api/settings/acoustid_api_key for Web UI configuration

use crate::{ApiError, ApiResult, AppState};
use axum::{extract::State, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, warn};

/// Request payload for setting AcoustID API key
///
/// **Traceability:** [APIK-UI-010]
#[derive(Debug, Deserialize)]
pub struct SetApiKeyRequest {
    /// The AcoustID API key to configure
    pub api_key: String,
}

/// Response payload for API key configuration
///
/// **Traceability:** [APIK-UI-020]
#[derive(Debug, Serialize)]
pub struct SetApiKeyResponse {
    /// Whether the operation succeeded
    pub success: bool,
    /// Human-readable status message
    pub message: String,
}

/// POST /api/settings/acoustid_api_key handler
///
/// **Traceability:** [APIK-UI-010], [APIK-UI-020], [APIK-UI-030]
///
/// **Request:** `{"api_key": "your-acoustid-key"}`
/// **Response:** `{"success": true, "message": "..."}`
///
/// **Behavior:**
/// 1. Validate key (non-empty, non-whitespace)
/// 2. Write to database (authoritative)
/// 3. Sync to TOML (best-effort backup)
///
/// **Errors:**
/// - 400 Bad Request: Empty or whitespace-only key
/// - 500 Internal Server Error: Database write failure
///
/// **Note:** TOML write failures log warnings but do not fail the request
pub async fn set_acoustid_api_key(
    State(state): State<AppState>,
    Json(payload): Json<SetApiKeyRequest>,
) -> ApiResult<Json<SetApiKeyResponse>> {
    // Validate key (non-empty, non-whitespace)
    if !crate::config::is_valid_key(&payload.api_key) {
        return Err(ApiError::BadRequest(
            "API key cannot be empty or whitespace-only".to_string(),
        ));
    }

    // Write to database (authoritative)
    crate::db::settings::set_acoustid_api_key(&state.db, payload.api_key.clone())
        .await
        .map_err(|e| {
            ApiError::Internal(format!("Failed to save API key to database: {}", e))
        })?;

    info!("AcoustID API key configured via Web UI");

    // Sync to TOML (best-effort backup)
    let toml_path = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(|home| {
            std::path::PathBuf::from(home)
                .join(".config")
                .join("wkmp")
                .join("wkmp-ai.toml")
        })
        .unwrap_or_else(|_| std::path::PathBuf::from("wkmp-ai.toml"));

    let mut settings = HashMap::new();
    settings.insert("acoustid_api_key".to_string(), payload.api_key);

    match crate::config::sync_settings_to_toml(settings, &toml_path).await {
        Ok(()) => {
            info!("API key synced to TOML: {}", toml_path.display());
        }
        Err(e) => {
            warn!("TOML sync failed (database write succeeded): {}", e);
        }
    }

    Ok(Json(SetApiKeyResponse {
        success: true,
        message: "AcoustID API key configured successfully".to_string(),
    }))
}

/// Build settings routes
///
/// **Traceability:** [APIK-UI-010]
pub fn settings_routes() -> Router<AppState> {
    Router::new().route("/api/settings/acoustid_api_key", post(set_acoustid_api_key))
}
