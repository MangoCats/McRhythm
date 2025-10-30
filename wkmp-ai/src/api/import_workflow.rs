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

/// POST /import/start request
#[derive(Debug, Deserialize)]
pub struct StartImportRequest {
    pub root_folder: String,
    #[serde(default)]
    pub parameters: ImportParameters,
}

/// POST /import/start response
#[derive(Debug, Serialize)]
pub struct StartImportResponse {
    pub session_id: Uuid,
    pub state: ImportState,
    pub started_at: chrono::DateTime<chrono::Utc>,
}

/// GET /import/status response
#[derive(Debug, Serialize)]
pub struct ImportStatusResponse {
    pub session_id: Uuid,
    pub state: ImportState,
    pub progress: crate::models::ImportProgress,
    pub current_operation: String,
    pub errors: Vec<crate::models::ImportError>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub elapsed_seconds: u64,
    pub estimated_remaining_seconds: Option<u64>,
}

/// POST /import/cancel response
#[derive(Debug, Serialize)]
pub struct CancelImportResponse {
    pub session_id: Uuid,
    pub state: ImportState,
    pub files_processed: usize,
    pub files_skipped: usize,
    pub cancelled_at: chrono::DateTime<chrono::Utc>,
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

    // **[AIA-WF-010]** Spawn background task for workflow orchestration
    let state_clone = state.clone();
    let session_clone = session.clone();
    let session_id_for_logging = session.session_id;
    tokio::spawn(async move {
        tracing::info!(
            session_id = %session_id_for_logging,
            "Background import workflow task started"
        );

        if let Err(e) = execute_import_workflow(state_clone, session_clone).await {
            tracing::error!(
                session_id = %session_id_for_logging,
                error = %e,
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

    // TODO: Signal background task to cancel (AIA-ASYNC-010)

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
async fn execute_import_workflow(state: AppState, session: ImportSession) -> anyhow::Result<()> {
    use crate::services::WorkflowOrchestrator;

    let session_id = session.session_id;
    tracing::info!(session_id = %session_id, "Starting import workflow orchestration");

    // Create workflow orchestrator with event bus for SSE broadcasting
    let orchestrator = WorkflowOrchestrator::new(state.db.clone(), state.event_bus.clone());

    // Execute workflow with error handling
    match orchestrator.execute_import(session).await {
        Ok(final_session) => {
            tracing::info!(
                session_id = %session_id,
                state = ?final_session.state,
                "Import workflow completed"
            );
            Ok(())
        }
        Err(e) => {
            tracing::error!(
                session_id = %session_id,
                error = %e,
                "Import workflow failed"
            );

            // Load session and mark as failed
            // **[AIA-ERR-020]** Ensure session transitions to Failed state even if error handling fails
            match crate::db::sessions::load_session(&state.db, session_id).await {
                Ok(Some(session)) => {
                    if let Err(failure_error) = orchestrator.handle_failure(session, &e).await {
                        tracing::error!(
                            session_id = %session_id,
                            error = %failure_error,
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
                        error = %db_error,
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

            Err(e)
        }
    }
}

/// Format bytes for human-readable display
fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

/// Build import workflow routes
pub fn import_routes() -> Router<AppState> {
    Router::new()
        .route("/import/start", post(start_import))
        .route("/import/status/:session_id", get(get_import_status))
        .route("/import/cancel/:session_id", post(cancel_import))
}
