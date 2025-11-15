//! Import session database operations
//!
//! **[AIA-WF-020]** Import session state persistence

use sqlx::{Row, SqlitePool};
use uuid::Uuid;
use wkmp_common::Result;

use crate::models::{ImportSession, ImportState, ImportParameters, ImportProgress, ImportError};
use crate::utils::retry_on_lock;

/// Save import session to database
///
/// **[ARCH-DB-CONN-001]** Uses retry_on_lock to handle transient database lock contention
pub async fn save_session(pool: &SqlitePool, session: &ImportSession) -> Result<()> {
    // Prepare all data BEFORE acquiring database connection
    let session_id = session.session_id.to_string();
    let state = serde_json::to_string(&session.state)
        .map_err(|e| wkmp_common::Error::Internal(format!("Failed to serialize state: {}", e)))?;
    let parameters = serde_json::to_string(&session.parameters)
        .map_err(|e| wkmp_common::Error::Internal(format!("Failed to serialize parameters: {}", e)))?;
    let errors = serde_json::to_string(&session.errors)
        .map_err(|e| wkmp_common::Error::Internal(format!("Failed to serialize errors: {}", e)))?;
    let started_at = session.started_at.to_rfc3339();
    let ended_at = session.ended_at.map(|dt| dt.to_rfc3339());
    let progress_current = session.progress.current as i64;
    let progress_total = session.progress.total as i64;
    let progress_percentage = session.progress.percentage;
    let current_operation = session.progress.current_operation.clone();
    let root_folder = session.root_folder.clone();

    // Get max lock wait time from settings (default 5000ms)
    let max_wait_ms: i64 = sqlx::query_scalar(
        "SELECT CAST(value AS INTEGER) FROM settings WHERE key = 'ai_database_max_lock_wait_ms'"
    )
    .fetch_optional(pool)
    .await?
    .unwrap_or(5000);

    // Wrap in retry logic with unconstrained execution
    retry_on_lock(
        "save_session",
        max_wait_ms as u64,
        || async {
            sqlx::query(
                r#"
                INSERT INTO import_sessions (
                    session_id, state, root_folder, parameters,
                    progress_current, progress_total, progress_percentage,
                    current_operation, errors, started_at, ended_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(session_id) DO UPDATE SET
                    state = excluded.state,
                    progress_current = excluded.progress_current,
                    progress_total = excluded.progress_total,
                    progress_percentage = excluded.progress_percentage,
                    current_operation = excluded.current_operation,
                    errors = excluded.errors,
                    ended_at = excluded.ended_at
                "#,
            )
            .bind(&session_id)
            .bind(&state)
            .bind(&root_folder)
            .bind(&parameters)
            .bind(progress_current)
            .bind(progress_total)
            .bind(progress_percentage)
            .bind(&current_operation)
            .bind(&errors)
            .bind(&started_at)
            .bind(&ended_at)
            .execute(pool)
            .await
            .map_err(wkmp_common::Error::Database)?;

            Ok(())
        }
    )
    .await
}

/// Load import session from database
pub async fn load_session(pool: &SqlitePool, session_id: Uuid) -> Result<Option<ImportSession>> {
    let session_id_str = session_id.to_string();

    let row = sqlx::query(
        r#"
        SELECT session_id, state, root_folder, parameters,
               progress_current, progress_total, progress_percentage,
               current_operation, errors, started_at, ended_at
        FROM import_sessions
        WHERE session_id = ?
        "#,
    )
    .bind(session_id_str)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(row) => {
            let state: String = row.get("state");
            let state: ImportState = serde_json::from_str(&state)
                .map_err(|e| wkmp_common::Error::Internal(format!("Failed to deserialize state: {}", e)))?;

            let parameters: String = row.get("parameters");
            let parameters: ImportParameters = serde_json::from_str(&parameters)
                .map_err(|e| wkmp_common::Error::Internal(format!("Failed to deserialize parameters: {}", e)))?;

            let errors: String = row.get("errors");
            let errors: Vec<ImportError> = serde_json::from_str(&errors)
                .map_err(|e| wkmp_common::Error::Internal(format!("Failed to deserialize errors: {}", e)))?;

            let started_at: String = row.get("started_at");
            let started_at = chrono::DateTime::parse_from_rfc3339(&started_at)
                .map_err(|e| wkmp_common::Error::Internal(format!("Failed to parse started_at: {}", e)))?
                .with_timezone(&chrono::Utc);

            let ended_at: Option<String> = row.get("ended_at");
            let ended_at = ended_at
                .map(|s| chrono::DateTime::parse_from_rfc3339(&s))
                .transpose()
                .map_err(|e| wkmp_common::Error::Internal(format!("Failed to parse ended_at: {}", e)))?
                .map(|dt| dt.with_timezone(&chrono::Utc));

            let progress = ImportProgress {
                current: row.get::<i64, _>("progress_current") as usize,
                total: row.get::<i64, _>("progress_total") as usize,
                percentage: row.get("progress_percentage"),
                current_operation: row.get("current_operation"),
                elapsed_seconds: if let Some(end) = ended_at {
                    (end - started_at).num_seconds() as u64
                } else {
                    (chrono::Utc::now() - started_at).num_seconds() as u64
                },
                estimated_remaining_seconds: None, // Recalculated on demand
                // **[REQ-AIA-UI-001, REQ-AIA-UI-004]** Phase tracking not persisted to DB (runtime only)
                phases: Vec::new(),
                current_file: None,
            };

            Ok(Some(ImportSession {
                session_id,
                state,
                root_folder: row.get("root_folder"),
                parameters,
                progress,
                errors,
                started_at,
                ended_at,
            }))
        }
        None => Ok(None),
    }
}

/// Delete import session from database
pub async fn delete_session(pool: &SqlitePool, session_id: Uuid) -> Result<()> {
    let session_id_str = session_id.to_string();

    sqlx::query("DELETE FROM import_sessions WHERE session_id = ?")
        .bind(session_id_str)
        .execute(pool)
        .await?;

    Ok(())
}

/// Check if any import session is currently running
pub async fn has_running_session(pool: &SqlitePool) -> Result<bool> {
    let count: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*)
        FROM import_sessions
        WHERE state NOT IN ('"COMPLETED"', '"CANCELLED"', '"FAILED"')
        "#,
    )
    .fetch_one(pool)
    .await?;

    Ok(count > 0)
}

/// **[AIA-SEC-030]** Get the currently active import session (if any)
///
/// Returns the most recently started session that is not in a terminal state.
/// Used to restore progress UI after page reload.
pub async fn get_active_session(pool: &SqlitePool) -> Result<Option<ImportSession>> {
    let row = sqlx::query(
        r#"
        SELECT session_id, state, root_folder, parameters,
               progress_current, progress_total, progress_percentage,
               current_operation, errors, started_at, ended_at
        FROM import_sessions
        WHERE state NOT IN ('"COMPLETED"', '"CANCELLED"', '"FAILED"')
        ORDER BY started_at DESC
        LIMIT 1
        "#,
    )
    .fetch_optional(pool)
    .await?;

    match row {
        Some(row) => {
            let session_id_str: String = row.get("session_id");
            let session_id = Uuid::parse_str(&session_id_str)
                .map_err(|e| wkmp_common::Error::Internal(format!("Failed to parse session_id: {}", e)))?;

            let state: String = row.get("state");
            let state: ImportState = serde_json::from_str(&state)
                .map_err(|e| wkmp_common::Error::Internal(format!("Failed to deserialize state: {}", e)))?;

            let parameters: String = row.get("parameters");
            let parameters: ImportParameters = serde_json::from_str(&parameters)
                .map_err(|e| wkmp_common::Error::Internal(format!("Failed to deserialize parameters: {}", e)))?;

            let errors: String = row.get("errors");
            let errors: Vec<ImportError> = serde_json::from_str(&errors)
                .map_err(|e| wkmp_common::Error::Internal(format!("Failed to deserialize errors: {}", e)))?;

            let started_at: String = row.get("started_at");
            let started_at = chrono::DateTime::parse_from_rfc3339(&started_at)
                .map_err(|e| wkmp_common::Error::Internal(format!("Failed to parse started_at: {}", e)))?
                .with_timezone(&chrono::Utc);

            let ended_at: Option<String> = row.get("ended_at");
            let ended_at = ended_at
                .map(|s| chrono::DateTime::parse_from_rfc3339(&s))
                .transpose()
                .map_err(|e| wkmp_common::Error::Internal(format!("Failed to parse ended_at: {}", e)))?
                .map(|dt| dt.with_timezone(&chrono::Utc));

            let progress = ImportProgress {
                current: row.get::<i64, _>("progress_current") as usize,
                total: row.get::<i64, _>("progress_total") as usize,
                percentage: row.get("progress_percentage"),
                current_operation: row.get("current_operation"),
                elapsed_seconds: if let Some(end) = ended_at {
                    (end - started_at).num_seconds() as u64
                } else {
                    (chrono::Utc::now() - started_at).num_seconds() as u64
                },
                estimated_remaining_seconds: None, // Recalculated on demand
                // **[REQ-AIA-UI-001, REQ-AIA-UI-004]** Phase tracking not persisted to DB (runtime only)
                phases: Vec::new(),
                current_file: None,
            };

            Ok(Some(ImportSession {
                session_id,
                state,
                root_folder: row.get("root_folder"),
                parameters,
                progress,
                errors,
                started_at,
                ended_at,
            }))
        }
        None => Ok(None),
    }
}

/// Cleanup stale import sessions on startup
///
/// **[AIA-INIT-010]** Any session not in a terminal state when wkmp-ai starts
/// is from a previous run and will never complete. Mark these as CANCELLED.
///
/// **Rationale:**
/// - Import workflow runs in background task that dies when process stops
/// - No background task = no workflow = session will never progress
/// - User may have changed files/folders while wkmp-ai was down
/// - New import should start fresh to handle all changes
pub async fn cleanup_stale_sessions(pool: &SqlitePool) -> Result<usize> {
    let result = sqlx::query(
        r#"
        UPDATE import_sessions
        SET state = '"CANCELLED"',
            ended_at = ?,
            current_operation = 'Import cancelled - wkmp-ai was restarted'
        WHERE state NOT IN ('"COMPLETED"', '"CANCELLED"', '"FAILED"')
        "#,
    )
    .bind(chrono::Utc::now().to_rfc3339())
    .execute(pool)
    .await?;

    Ok(result.rows_affected() as usize)
}
