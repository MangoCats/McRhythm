//! Database access for wkmp-ai
//!
//! **[AIA-DB-010]** Shared SQLite database access

pub mod sessions;
pub mod parameters;

use anyhow::Result;
use sqlx::SqlitePool;
use std::path::Path;

/// Initialize database connection pool
///
/// **[AIA-DB-010]** Connects to shared wkmp.db in root folder
pub async fn init_database_pool(db_path: &Path) -> Result<SqlitePool> {
    // Ensure parent directory exists
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Use proper SQLite URI with mode=rwc (read, write, create)
    let db_url = format!("sqlite://{}?mode=rwc", db_path.display());
    tracing::debug!("Connecting to database: {}", db_url);

    let pool = SqlitePool::connect(&db_url).await?;

    // Run migrations for wkmp-ai specific tables
    init_tables(&pool).await?;

    Ok(pool)
}

/// Initialize wkmp-ai specific tables
///
/// Creates import_sessions and settings tables if they don't exist
async fn init_tables(pool: &SqlitePool) -> Result<()> {
    // Create settings table for parameter persistence
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create import_sessions table for session persistence
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS import_sessions (
            session_id TEXT PRIMARY KEY,
            state TEXT NOT NULL,
            root_folder TEXT NOT NULL,
            parameters TEXT NOT NULL,
            progress_current INTEGER NOT NULL DEFAULT 0,
            progress_total INTEGER NOT NULL DEFAULT 0,
            progress_percentage REAL NOT NULL DEFAULT 0.0,
            current_operation TEXT NOT NULL DEFAULT '',
            errors TEXT NOT NULL DEFAULT '[]',
            started_at TEXT NOT NULL,
            ended_at TEXT
        )
        "#,
    )
    .execute(pool)
    .await?;

    tracing::info!("Database tables initialized (settings, import_sessions)");

    Ok(())
}
