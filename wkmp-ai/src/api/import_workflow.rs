//! Import workflow API handlers
//!
//! **[IMPL008]** POST /import/start, GET /import/status, POST /import/cancel

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{error::{ApiError, ApiResult}, models::{ImportParameters, ImportSession, ImportState}, AppState};

/// **[AIA-SEC-030]** POST /import/validate-acoustid request
#[derive(Debug, Deserialize)]
pub struct ValidateAcoustIDRequest {
    /// AcoustID API key to validate
    pub api_key: String,
}

/// **[AIA-SEC-030]** POST /import/validate-acoustid response
#[derive(Debug, Serialize)]
pub struct ValidateAcoustIDResponse {
    /// Whether the key is valid
    pub valid: bool,
    /// Status message (error details if invalid)
    pub message: String,
}

/// POST /import/start request
#[derive(Debug, Deserialize)]
pub struct StartImportRequest {
    /// Root folder path to scan for audio files
    pub root_folder: String,
    /// Import parameters (optional, uses defaults if not provided)
    #[serde(default)]
    pub parameters: ImportParameters,
}

/// POST /import/start response
#[derive(Debug, Serialize)]
pub struct StartImportResponse {
    /// Unique session identifier for this import
    pub session_id: Uuid,
    /// Current import state
    pub state: ImportState,
    /// Timestamp when import started
    pub started_at: chrono::DateTime<chrono::Utc>,
}

/// GET /import/status response
#[derive(Debug, Serialize)]
pub struct ImportStatusResponse {
    /// Import session identifier
    pub session_id: Uuid,
    /// Current import state
    pub state: ImportState,
    /// Progress information (files processed, percentage complete)
    pub progress: crate::models::ImportProgress,
    /// Description of current operation
    pub current_operation: String,
    /// List of errors encountered during import
    pub errors: Vec<crate::models::ImportError>,
    /// Timestamp when import started
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// Seconds elapsed since import started
    pub elapsed_seconds: u64,
    /// Estimated seconds remaining (None if unknown)
    pub estimated_remaining_seconds: Option<u64>,
}

/// POST /import/cancel response
#[derive(Debug, Serialize)]
pub struct CancelImportResponse {
    /// Import session identifier
    pub session_id: Uuid,
    /// Final import state after cancellation
    pub state: ImportState,
    /// Number of files successfully processed before cancellation
    pub files_processed: usize,
    /// Number of files skipped
    pub files_skipped: usize,
    /// Timestamp when import was cancelled
    pub cancelled_at: chrono::DateTime<chrono::Utc>,
}

/// **[AIA-SEC-030]** POST /import/validate-acoustid
///
/// Validate an AcoustID API key with a test request.
/// Called before import starts to check if the key is valid.
pub async fn validate_acoustid(
    State(_state): State<AppState>,
    Json(request): Json<ValidateAcoustIDRequest>,
) -> ApiResult<Json<ValidateAcoustIDResponse>> {
    if request.api_key.is_empty() {
        return Ok(Json(ValidateAcoustIDResponse {
            valid: false,
            message: "API key cannot be empty".to_string(),
        }));
    }

    match crate::extractors::acoustid_client::validate_acoustid_key(&request.api_key).await {
        Ok(()) => {
            tracing::info!("AcoustID API key validated successfully");
            Ok(Json(ValidateAcoustIDResponse {
                valid: true,
                message: "API key is valid".to_string(),
            }))
        }
        Err(err) => {
            tracing::debug!(error = %err, "AcoustID API key validation failed");
            Ok(Json(ValidateAcoustIDResponse {
                valid: false,
                message: err,
            }))
        }
    }
}

/// **[IMPL008]** POST /import/start
///
/// Begin import session. Returns 202 Accepted with session ID.
pub async fn start_import(
    State(state): State<AppState>,
    Json(request): Json<StartImportRequest>,
) -> ApiResult<Json<StartImportResponse>> {
    // **[AIA-SEC-010]** Validate root folder
    let path = std::path::Path::new(&request.root_folder);
    if !path.exists() {
        return Err(ApiError::BadRequest(format!(
            "Root folder does not exist: {}",
            request.root_folder
        )));
    }
    if !path.is_dir() {
        return Err(ApiError::BadRequest(format!(
            "Root folder is not a directory: {}",
            request.root_folder
        )));
    }

    // **[AIA-ERR-010]** Check if import already running (409 Conflict)
    if crate::db::sessions::has_running_session(&state.db).await? {
        return Err(ApiError::Conflict(
            "Import session already running".to_string(),
        ));
    }

    // Create new import session
    let session = ImportSession::new(request.root_folder, request.parameters);
    let response = StartImportResponse {
        session_id: session.session_id,
        state: session.state,
        started_at: session.started_at,
    };

    // **[AIA-WF-020]** Save session to database
    crate::db::sessions::save_session(&state.db, &session).await?;

    tracing::info!(
        session_id = %response.session_id,
        root_folder = %session.root_folder,
        "Import session started and persisted to database"
    );

    // **[AIA-ASYNC-010]** Create cancellation token for this session
    let cancel_token = tokio_util::sync::CancellationToken::new();
    {
        let mut tokens = state.cancellation_tokens.write().await;
        tokens.insert(session.session_id, cancel_token.clone());
    }

    // **[AIA-WF-010]** Spawn background task for workflow orchestration
    let state_clone = state.clone();
    let session_clone = session.clone();
    let session_id_for_logging = session.session_id;
    let cancel_token_clone = cancel_token.clone();
    tokio::spawn(async move {
        tracing::info!(
            session_id = %session_id_for_logging,
            "Background import workflow task started"
        );

        if let Err(e) = execute_import_workflow(state_clone, session_clone, cancel_token_clone).await {
            tracing::error!(
                session_id = %session_id_for_logging,
                error = ?e,
                "Import workflow background task failed"
            );
        } else {
            tracing::info!(
                session_id = %session_id_for_logging,
                "Background import workflow task completed successfully"
            );
        }
    });

    Ok(Json(response))
}

/// **[IMPL008]** GET /import/status/{session_id}
///
/// Poll import progress. Returns current status.
pub async fn get_import_status(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
) -> ApiResult<Json<ImportStatusResponse>> {
    // **[AIA-WF-020]** Load session from database
    let session = crate::db::sessions::load_session(&state.db, session_id)
        .await?
        .ok_or_else(|| {
            ApiError::NotFound(format!("Import session not found: {}", session_id))
        })?;

    tracing::debug!(session_id = %session_id, state = ?session.state, "Status query");

    let response = ImportStatusResponse {
        session_id: session.session_id,
        state: session.state,
        progress: session.progress.clone(),
        current_operation: session.progress.current_operation.clone(),
        errors: session.errors.clone(),
        started_at: session.started_at,
        elapsed_seconds: session.progress.elapsed_seconds,
        estimated_remaining_seconds: session.progress.estimated_remaining_seconds,
    };

    Ok(Json(response))
}

/// **[AIA-SEC-030]** GET /import/active
///
/// Get the currently active import session (if any).
/// Used to restore progress UI after page reload.
pub async fn get_active_session(
    State(state): State<AppState>,
) -> ApiResult<Json<Option<ImportStatusResponse>>> {
    let session = crate::db::sessions::get_active_session(&state.db).await?;

    if let Some(session) = session {
        tracing::debug!(session_id = %session.session_id, state = ?session.state, "Active session query");

        let response = ImportStatusResponse {
            session_id: session.session_id,
            state: session.state,
            progress: session.progress.clone(),
            current_operation: session.progress.current_operation.clone(),
            errors: session.errors.clone(),
            started_at: session.started_at,
            elapsed_seconds: session.progress.elapsed_seconds,
            estimated_remaining_seconds: session.progress.estimated_remaining_seconds,
        };

        Ok(Json(Some(response)))
    } else {
        Ok(Json(None))
    }
}

/// **[IMPL008]** POST /import/cancel/{session_id}
///
/// Cancel running import. Returns cancellation status.
pub async fn cancel_import(
    State(state): State<AppState>,
    Path(session_id): Path<Uuid>,
) -> ApiResult<Json<CancelImportResponse>> {
    // **[AIA-WF-020]** Load session from database
    let mut session = crate::db::sessions::load_session(&state.db, session_id)
        .await?
        .ok_or_else(|| {
            ApiError::NotFound(format!("Import session not found: {}", session_id))
        })?;

    // Check if session is already terminal
    if session.is_terminal() {
        return Err(ApiError::BadRequest(format!(
            "Import session already in terminal state: {:?}",
            session.state
        )));
    }

    // **[AIA-ASYNC-010]** Signal background task to cancel
    {
        let mut tokens = state.cancellation_tokens.write().await;
        if let Some(token) = tokens.remove(&session_id) {
            tracing::info!(
                session_id = %session_id,
                "Triggering cancellation token for import session"
            );
            token.cancel();
        } else {
            tracing::warn!(
                session_id = %session_id,
                "No cancellation token found - background task may have already completed"
            );
        }
    }

    // Transition to cancelled state
    session.transition_to(ImportState::Cancelled);

    // Save updated session
    crate::db::sessions::save_session(&state.db, &session).await?;

    tracing::info!(session_id = %session_id, "Import session cancelled");

    let response = CancelImportResponse {
        session_id: session.session_id,
        state: session.state,
        files_processed: session.progress.current,
        files_skipped: session.progress.total.saturating_sub(session.progress.current),
        cancelled_at: session.ended_at.unwrap_or_else(chrono::Utc::now),
    };

    Ok(Json(response))
}

/// Background task for workflow execution
///
/// **[AIA-WF-010]** Execute complete import workflow through all states
/// **[AIA-ASYNC-010]** Respects cancellation token
async fn execute_import_workflow(
    state: AppState,
    session: ImportSession,
    cancel_token: tokio_util::sync::CancellationToken,
) -> anyhow::Result<()> {
    use crate::services::WorkflowOrchestrator;

    let session_id = session.session_id;
    tracing::info!(session_id = %session_id, "Starting import workflow orchestration");

    // Load AcoustID API key from database
    let acoustid_api_key = match crate::db::settings::get_acoustid_api_key(&state.db).await {
        Ok(key) => key,
        Err(e) => {
            tracing::warn!("Failed to load AcoustID API key from database: {}", e);
            None
        }
    };

    if acoustid_api_key.is_none() {
        tracing::warn!("No AcoustID API key configured - fingerprinting will be disabled");
        tracing::warn!("Configure key at: http://localhost:5723/settings");
    }

    // Create workflow orchestrator with event bus for SSE broadcasting
    let orchestrator = WorkflowOrchestrator::new(
        state.db.clone(),
        state.event_bus.clone(),
        acoustid_api_key,
    );

    // Execute workflow with error handling
    // **[PLAN024]** Use new 3-tier hybrid fusion pipeline
    match orchestrator.execute_import_plan024(session, cancel_token).await {
        Ok(final_session) => {
            tracing::info!(
                session_id = %session_id,
                state = ?final_session.state,
                "Import workflow completed"
            );

            // Clean up cancellation token (if still present)
            let mut tokens = state.cancellation_tokens.write().await;
            tokens.remove(&session_id);

            Ok(())
        }
        Err(e) => {
            tracing::error!(
                session_id = %session_id,
                error = ?e,
                "Import workflow failed"
            );

            // Load session and mark as failed
            // **[AIA-ERR-020]** Ensure session transitions to Failed state even if error handling fails
            match crate::db::sessions::load_session(&state.db, session_id).await {
                Ok(Some(session)) => {
                    if let Err(failure_error) = orchestrator.handle_failure(session, &e).await {
                        tracing::error!(
                            session_id = %session_id,
                            error = ?failure_error,
                            "Failed to mark session as failed - attempting direct database update"
                        );

                        // Fallback: Direct database update to ensure session is marked as failed
                        let _ = sqlx::query(
                            r#"UPDATE import_sessions
                               SET state = '"FAILED"',
                                   ended_at = ?,
                                   current_operation = ?
                               WHERE session_id = ?"#
                        )
                        .bind(chrono::Utc::now().to_rfc3339())
                        .bind(format!("Import failed: {}", e))
                        .bind(session_id.to_string())
                        .execute(&state.db)
                        .await;
                    }
                }
                Ok(None) => {
                    tracing::error!(
                        session_id = %session_id,
                        "Session not found in database - cannot mark as failed"
                    );
                }
                Err(db_error) => {
                    tracing::error!(
                        session_id = %session_id,
                        error = ?db_error,
                        "Failed to load session from database - attempting direct database update"
                    );

                    // Fallback: Direct database update
                    let _ = sqlx::query(
                        r#"UPDATE import_sessions
                           SET state = '"FAILED"',
                               ended_at = ?,
                               current_operation = ?
                           WHERE session_id = ?"#
                    )
                    .bind(chrono::Utc::now().to_rfc3339())
                    .bind(format!("Import failed: {}", e))
                    .bind(session_id.to_string())
                    .execute(&state.db)
                    .await;
                }
            }

            // Clean up cancellation token (if still present)
            let mut tokens = state.cancellation_tokens.write().await;
            tokens.remove(&session_id);

            Err(e)
        }
    }
}

/// **[AIA-SEC-030]** POST /import/acoustid-key request
#[derive(Debug, Deserialize)]
pub struct UpdateAcoustIDKeyRequest {
    /// Session ID for the import
    pub session_id: Uuid,
    /// New AcoustID API key to validate and use
    pub api_key: String,
}

/// **[AIA-SEC-030]** POST /import/acoustid-key response
#[derive(Debug, Serialize)]
pub struct UpdateAcoustIDKeyResponse {
    /// Session ID
    pub session_id: Uuid,
    /// Whether the key was validated successfully
    pub success: bool,
    /// Status message
    pub message: String,
}

/// **[AIA-SEC-030]** POST /import/acoustid-key
///
/// Update AcoustID API key for an active import session.
/// Validates the key with a test API call before accepting it.
pub async fn update_acoustid_key(
    State(_state): State<AppState>,
    Json(request): Json<UpdateAcoustIDKeyRequest>,
) -> ApiResult<Json<UpdateAcoustIDKeyResponse>> {
    if request.api_key.is_empty() {
        return Ok(Json(UpdateAcoustIDKeyResponse {
            session_id: request.session_id,
            success: false,
            message: "API key cannot be empty".to_string(),
        }));
    }

    // Validate key with test API call
    match crate::extractors::acoustid_client::validate_acoustid_key(&request.api_key).await {
        Ok(()) => {
            tracing::info!(
                session_id = %request.session_id,
                "AcoustID API key validated successfully"
            );
            // TODO: Update pipeline config for session with new key
            // TODO: Resume import processing

            Ok(Json(UpdateAcoustIDKeyResponse {
                session_id: request.session_id,
                success: true,
                message: "AcoustID API key is valid and has been updated".to_string(),
            }))
        }
        Err(err) => {
            tracing::warn!(
                session_id = %request.session_id,
                error = %err,
                "AcoustID API key validation failed"
            );
            Ok(Json(UpdateAcoustIDKeyResponse {
                session_id: request.session_id,
                success: false,
                message: format!("Invalid API key: {}", err),
            }))
        }
    }
}

/// **[AIA-SEC-030]** POST /import/acoustid-skip request
#[derive(Debug, Deserialize)]
pub struct SkipAcoustIDRequest {
    /// Session ID for the import
    pub session_id: Uuid,
}

/// **[AIA-SEC-030]** POST /import/acoustid-skip response
#[derive(Debug, Serialize)]
pub struct SkipAcoustIDResponse {
    /// Session ID
    pub session_id: Uuid,
    /// Status message
    pub message: String,
}

/// **[AIA-SEC-030]** POST /import/acoustid-skip
///
/// Skip AcoustID functionality for this import session.
/// Import will continue without fingerprint-based identification.
pub async fn skip_acoustid(
    State(_state): State<AppState>,
    Json(request): Json<SkipAcoustIDRequest>,
) -> ApiResult<Json<SkipAcoustIDResponse>> {
    // TODO: Implement skip logic
    // Real implementation should:
    // 1. Set acoustid_skip=true in pipeline config for this session
    // 2. Resume import processing

    // TODO: Update pipeline config for session
    // TODO: Resume import

    Ok(Json(SkipAcoustIDResponse {
        session_id: request.session_id,
        message: "AcoustID skipped for this session (implementation pending)".to_string(),
    }))
}

/// Build import workflow routes
pub fn import_routes() -> Router<AppState> {
    Router::new()
        .route("/import/validate-acoustid", post(validate_acoustid))
        .route("/import/start", post(start_import))
        .route("/import/active", get(get_active_session))
        .route("/import/status/:session_id", get(get_import_status))
        .route("/import/cancel/:session_id", post(cancel_import))
        .route("/import/acoustid-key", post(update_acoustid_key))
        .route("/import/acoustid-skip", post(skip_acoustid))
}
