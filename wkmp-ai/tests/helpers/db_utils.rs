//! Database Test Utilities
//!
//! Utilities for database testing and schema validation

use anyhow::Result;
use sqlx::SqlitePool;
use std::path::Path;
use tempfile::TempDir;

/// Column information from PRAGMA table_info
#[derive(Debug, sqlx::FromRow)]
pub struct ColumnInfo {
    pub cid: i32,
    pub name: String,
    pub r#type: String,
    pub notnull: i32,
    pub dflt_value: Option<String>,
    pub pk: i32,
}

/// Create temporary test database with migrations applied
///
/// Returns (TempDir, SqlitePool) - TempDir must be kept alive for duration of test
pub async fn create_test_db() -> Result<(TempDir, SqlitePool)> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test_wkmp.db");

    // Create database pool
    let db_url = format!("sqlite:{}?mode=rwc", db_path.display());
    let pool = SqlitePool::connect(&db_url).await?;

    // Run migrations
    sqlx::migrate!("../migrations").run(&pool).await?;

    Ok((temp_dir, pool))
}

/// Get table schema information
pub async fn get_table_columns(pool: &SqlitePool, table_name: &str) -> Result<Vec<ColumnInfo>> {
    let query = format!("PRAGMA table_info({})", table_name);
    let columns = sqlx::query_as::<_, ColumnInfo>(&query)
        .fetch_all(pool)
        .await?;
    Ok(columns)
}

/// Check if table has specific column
pub async fn has_column(pool: &SqlitePool, table_name: &str, column_name: &str) -> Result<bool> {
    let columns = get_table_columns(pool, table_name).await?;
    Ok(columns.iter().any(|c| c.name == column_name))
}

/// Assert table does NOT have column (for negative schema validation)
pub async fn assert_no_column(pool: &SqlitePool, table_name: &str, column_name: &str) {
    let has_col = has_column(pool, table_name, column_name).await.unwrap();
    assert!(
        !has_col,
        "Table '{}' should NOT have column '{}', but it exists",
        table_name,
        column_name
    );
}

/// Assert table HAS column (for positive schema validation)
pub async fn assert_has_column(pool: &SqlitePool, table_name: &str, column_name: &str) {
    let has_col = has_column(pool, table_name, column_name).await.unwrap();
    assert!(
        has_col,
        "Table '{}' should have column '{}', but it doesn't exist",
        table_name,
        column_name
    );
}

/// Get all table names in database
pub async fn get_table_names(pool: &SqlitePool) -> Result<Vec<String>> {
    let tables = sqlx::query_scalar::<_, String>(
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name"
    )
    .fetch_all(pool)
    .await?;
    Ok(tables)
}

/// Create test WorkflowOrchestrator
pub fn create_test_orchestrator(db_pool: sqlx::SqlitePool) -> wkmp_ai::services::WorkflowOrchestrator {
    use wkmp_ai::services::WorkflowOrchestrator;
    use wkmp_common::events::EventBus;

    let event_bus = EventBus::new(100); // Capacity for event broadcast
    let acoustid_api_key = None; // No API key for tests

    WorkflowOrchestrator::new(db_pool, event_bus, acoustid_api_key)
}

/// Seed database with test file records
pub async fn seed_test_files(
    pool: &SqlitePool,
    files: &[(String, String)], // (path, hash)
) -> Result<Vec<String>> {
    use wkmp_ai::db::files::AudioFile;

    let mut guids: Vec<String> = Vec::new();

    for (path, hash) in files {
        let file = AudioFile::new(path.clone(), hash.clone(), chrono::Utc::now());
        wkmp_ai::db::files::save_file(pool, &file).await?;
        guids.push(file.guid.to_string());
    }

    Ok(guids)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_test_db() {
        let result = create_test_db().await;
        assert!(result.is_ok());

        let (_temp_dir, pool) = result.unwrap();

        // Verify migrations ran
        let tables = get_table_names(&pool).await.unwrap();
        assert!(!tables.is_empty(), "Should have tables after migrations");
    }

    #[tokio::test]
    async fn test_get_table_columns() {
        let (_temp_dir, pool) = create_test_db().await.unwrap();

        let columns = get_table_columns(&pool, "files").await;
        assert!(columns.is_ok());

        let columns = columns.unwrap();
        assert!(!columns.is_empty(), "files table should have columns");

        // Verify expected columns exist
        let column_names: Vec<_> = columns.iter().map(|c| c.name.as_str()).collect();
        assert!(column_names.contains(&"guid"));
        assert!(column_names.contains(&"path"));
        assert!(column_names.contains(&"hash"));
    }

    #[tokio::test]
    async fn test_has_column() {
        let (_temp_dir, pool) = create_test_db().await.unwrap();

        let result = has_column(&pool, "files", "guid").await;
        assert!(result.is_ok());
        assert!(result.unwrap(), "files table should have guid column");

        let result = has_column(&pool, "files", "nonexistent_column").await;
        assert!(result.is_ok());
        assert!(
            !result.unwrap(),
            "files table should not have nonexistent_column"
        );
    }
}
