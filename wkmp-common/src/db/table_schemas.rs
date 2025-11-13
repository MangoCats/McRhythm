//! Table Schema Definitions
//!
//! Single source of truth for database table schemas.
//! Each struct defines the expected schema for one table.
//!
//! **[ARCH-DB-SYNC-010]** Declarative schema definition
//!
//! # Usage
//!
//! ```rust,ignore
//! // Sync all table schemas on startup
//! sync_all_table_schemas(&pool).await?;
//! ```

use crate::db::schema_sync::{ColumnDefinition, SchemaSync, TableSchema};
use crate::Result;
use sqlx::SqlitePool;
use tracing::info;

/// Files table schema
///
/// Per IMPL001-database_schema.md lines 120-148
pub struct FilesTableSchema;

impl TableSchema for FilesTableSchema {
    fn table_name() -> &'static str {
        "files"
    }

    fn expected_columns() -> Vec<ColumnDefinition> {
        vec![
            ColumnDefinition::new("guid", "TEXT")
                .primary_key(),

            ColumnDefinition::new("path", "TEXT")
                .not_null()
                .unique(),

            ColumnDefinition::new("hash", "TEXT")
                .not_null(),

            // REQ-F-003: duration_ticks (added in migration v3)
            ColumnDefinition::new("duration_ticks", "INTEGER"),

            // Audio metadata columns (added in migration v4)
            ColumnDefinition::new("format", "TEXT"),
            ColumnDefinition::new("sample_rate", "INTEGER"),
            ColumnDefinition::new("channels", "INTEGER"),
            ColumnDefinition::new("file_size_bytes", "INTEGER"),

            ColumnDefinition::new("modification_time", "TIMESTAMP")
                .not_null(),

            // Import processing status (REQ-SPEC032-008, REQ-SPEC032-009)
            ColumnDefinition::new("status", "TEXT")
                .default("'PENDING'"),

            // JSON array of file UUIDs with matching hash (REQ-SPEC032-009)
            ColumnDefinition::new("matching_hashes", "TEXT"),

            ColumnDefinition::new("created_at", "TIMESTAMP")
                .not_null()
                .default("CURRENT_TIMESTAMP"),

            ColumnDefinition::new("updated_at", "TIMESTAMP")
                .not_null()
                .default("CURRENT_TIMESTAMP"),
        ]
    }
}

/// Synchronize all table schemas
///
/// **Phase 2 of database initialization** (after CREATE TABLE IF NOT EXISTS, before migrations)
///
/// Automatically adds missing columns to all tables.
pub async fn sync_all_table_schemas(pool: &SqlitePool) -> Result<()> {
    info!("=== Phase 2: Automatic Schema Synchronization ===");

    // Sync each table
    // Note: Only files table implemented as proof of concept
    // Future: Add all 30+ tables from IMPL001-database_schema.md
    SchemaSync::sync_table::<FilesTableSchema>(pool).await?;

    info!("=== Schema Synchronization Complete ===");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_test_db() -> SqlitePool {
        SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn test_files_table_schema_definition() {
        let columns = FilesTableSchema::expected_columns();

        // Verify key columns are present
        assert!(columns.iter().any(|c| c.name == "guid" && c.primary_key));
        assert!(columns.iter().any(|c| c.name == "path" && c.not_null && c.unique));
        assert!(columns.iter().any(|c| c.name == "hash" && c.not_null));
        assert!(columns.iter().any(|c| c.name == "duration_ticks"));
        assert!(columns.iter().any(|c| c.name == "format"));
        assert!(columns.iter().any(|c| c.name == "sample_rate"));
        assert!(columns.iter().any(|c| c.name == "channels"));
        assert!(columns.iter().any(|c| c.name == "file_size_bytes"));
    }

    #[tokio::test]
    async fn test_sync_files_table_fresh_database() {
        let pool = setup_test_db().await;

        // Create files table with OLD schema (missing format, sample_rate, channels, file_size_bytes)
        sqlx::query(
            r#"
            CREATE TABLE files (
                guid TEXT PRIMARY KEY,
                path TEXT NOT NULL UNIQUE,
                hash TEXT NOT NULL,
                duration_ticks INTEGER,
                modification_time TIMESTAMP NOT NULL,
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#
        )
        .execute(&pool)
        .await
        .unwrap();

        // Sync schema - should add missing columns
        SchemaSync::sync_table::<FilesTableSchema>(&pool)
            .await
            .unwrap();

        // Verify columns were added
        let columns: Vec<(String, String)> = sqlx::query_as(
            "SELECT name, type FROM pragma_table_info('files') ORDER BY cid"
        )
        .fetch_all(&pool)
        .await
        .unwrap();

        let column_names: Vec<String> = columns.iter().map(|(name, _)| name.clone()).collect();

        assert!(column_names.contains(&"guid".to_string()));
        assert!(column_names.contains(&"path".to_string()));
        assert!(column_names.contains(&"hash".to_string()));
        assert!(column_names.contains(&"duration_ticks".to_string()));
        assert!(column_names.contains(&"format".to_string()));
        assert!(column_names.contains(&"sample_rate".to_string()));
        assert!(column_names.contains(&"channels".to_string()));
        assert!(column_names.contains(&"file_size_bytes".to_string()));
    }

    #[tokio::test]
    async fn test_sync_files_table_idempotent() {
        let pool = setup_test_db().await;

        // Create files table with CURRENT schema (all columns present)
        sqlx::query(
            r#"
            CREATE TABLE files (
                guid TEXT PRIMARY KEY,
                path TEXT NOT NULL UNIQUE,
                hash TEXT NOT NULL,
                duration_ticks INTEGER,
                format TEXT,
                sample_rate INTEGER,
                channels INTEGER,
                file_size_bytes INTEGER,
                modification_time TIMESTAMP NOT NULL,
                created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#
        )
        .execute(&pool)
        .await
        .unwrap();

        // Sync schema - should make no changes
        SchemaSync::sync_table::<FilesTableSchema>(&pool)
            .await
            .unwrap();

        // Run sync again - should still make no changes
        SchemaSync::sync_table::<FilesTableSchema>(&pool)
            .await
            .unwrap();

        // Verify no duplicate columns
        let column_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM pragma_table_info('files')"
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(column_count, 13); // 13 columns total (includes status, matching_hashes)
    }
}
