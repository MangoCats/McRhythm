//! Database access for wkmp-ai
//!
//! **[AIA-DB-010]** Shared SQLite database access

pub mod albums;
pub mod artists;
pub mod files;
pub mod parameters;
pub mod passages;
pub mod schema;
pub mod sessions;
pub mod songs;
pub mod works;

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

    // Initialize complete database schema
    schema::initialize_schema(&pool).await?;

    Ok(pool)
}
