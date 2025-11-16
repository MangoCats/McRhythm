//! Settings API endpoint
//!
//! **Traceability:** [APIK-UI-010], [APIK-UI-020], [APIK-UI-030]
//!
//! Provides POST /api/settings/acoustid_api_key for Web UI configuration

use crate::{ApiError, ApiResult, AppState};
use axum::{extract::State, routing::get, Json, Router};
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
#[derive(Debug, Serialize, Deserialize)]
pub struct SetApiKeyResponse {
    /// Whether the operation succeeded
    pub success: bool,
    /// Human-readable status message
    pub message: String,
}

/// Response payload for getting AcoustID API key
///
/// **Traceability:** [AIA-SEC-030]
#[derive(Debug, Serialize, Deserialize)]
pub struct GetApiKeyResponse {
    /// Whether an API key is configured
    pub configured: bool,
    /// The API key (if configured)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
}

/// GET /api/settings/acoustid_api_key handler
///
/// **Traceability:** [AIA-SEC-030]
///
/// **Response:** `{"configured": true, "api_key": "..."}`
///
/// **Behavior:**
/// 1. Check database for AcoustID API key
/// 2. Return whether key is configured and the key value
///
/// **Errors:**
/// - 500 Internal Server Error: Database read failure
pub async fn get_acoustid_api_key(
    State(state): State<AppState>,
) -> ApiResult<Json<GetApiKeyResponse>> {
    let api_key = crate::db::settings::get_acoustid_api_key(&state.db)
        .await
        .map_err(|e| {
            ApiError::Internal(format!("Failed to retrieve API key from database: {}", e))
        })?;

    Ok(Json(GetApiKeyResponse {
        configured: api_key.is_some(),
        api_key,
    }))
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
/// **Traceability:** [APIK-UI-010], [AIA-SEC-030]
pub fn settings_routes() -> Router<AppState> {
    Router::new().route(
        "/api/settings/acoustid_api_key",
        get(get_acoustid_api_key).post(set_acoustid_api_key),
    )
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use sqlx::SqlitePool;
    use tower::ServiceExt;

    /// Setup in-memory test database with settings table
    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();

        sqlx::query(
            "CREATE TABLE settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )"
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    /// Create test AppState with in-memory database
    fn create_test_state(pool: SqlitePool) -> AppState {
        use wkmp_common::events::EventBus;
        AppState::new(pool, EventBus::new(100))
    }

    #[tokio::test]
    async fn test_get_acoustid_api_key_configured() {
        let pool = setup_test_db().await;

        // Insert test key
        sqlx::query("INSERT INTO settings (key, value) VALUES ('acoustid_api_key', 'test_key_123')")
            .execute(&pool)
            .await
            .unwrap();

        let state = create_test_state(pool);
        let app = settings_routes().with_state(state);

        // Make GET request
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/settings/acoustid_api_key")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Parse response body
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: GetApiKeyResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(json.configured, true);
        assert_eq!(json.api_key, Some("test_key_123".to_string()));
    }

    #[tokio::test]
    async fn test_get_acoustid_api_key_not_configured() {
        let pool = setup_test_db().await;
        let state = create_test_state(pool);
        let app = settings_routes().with_state(state);

        // Make GET request
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/settings/acoustid_api_key")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Parse response body
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: GetApiKeyResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(json.configured, false);
        assert_eq!(json.api_key, None);
    }

    #[tokio::test]
    async fn test_post_acoustid_api_key_success() {
        let pool = setup_test_db().await;
        let state = create_test_state(pool.clone());
        let app = settings_routes().with_state(state);

        // Make POST request
        let request_body = serde_json::json!({
            "api_key": "new_valid_key"
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/settings/acoustid_api_key")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Parse response body
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: SetApiKeyResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(json.success, true);
        assert!(json.message.contains("successfully"));

        // Verify key was stored in database
        let stored_key = crate::db::settings::get_acoustid_api_key(&pool)
            .await
            .unwrap();
        assert_eq!(stored_key, Some("new_valid_key".to_string()));
    }

    #[tokio::test]
    async fn test_post_acoustid_api_key_empty() {
        let pool = setup_test_db().await;
        let state = create_test_state(pool);
        let app = settings_routes().with_state(state);

        // Make POST request with empty key
        let request_body = serde_json::json!({
            "api_key": ""
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/settings/acoustid_api_key")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_post_acoustid_api_key_whitespace_only() {
        let pool = setup_test_db().await;
        let state = create_test_state(pool);
        let app = settings_routes().with_state(state);

        // Make POST request with whitespace-only key
        let request_body = serde_json::json!({
            "api_key": "   "
        });

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/settings/acoustid_api_key")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
}
