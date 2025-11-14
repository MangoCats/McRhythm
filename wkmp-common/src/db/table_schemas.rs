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

            // Extracted tag metadata (REQ-SPEC032-010)
            ColumnDefinition::new("artist", "TEXT"),
            ColumnDefinition::new("title", "TEXT"),
            ColumnDefinition::new("album", "TEXT"),
            ColumnDefinition::new("track_number", "INTEGER"),
            ColumnDefinition::new("year", "INTEGER"),

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

/// Passages table schema
///
/// Per PLAN024 requirements for Phase 7-9 (Recording, Amplitude, Flavoring)
pub struct PassagesTableSchema;

impl TableSchema for PassagesTableSchema {
    fn table_name() -> &'static str {
        "passages"
    }

    fn expected_columns() -> Vec<ColumnDefinition> {
        vec![
            ColumnDefinition::new("guid", "TEXT")
                .primary_key(),

            ColumnDefinition::new("file_id", "TEXT")
                .not_null(),

            // PLAN024 Phase 7: Recording - Song association
            ColumnDefinition::new("song_id", "TEXT"),

            // Timing columns (SPEC017 tick-based)
            ColumnDefinition::new("start_time_ticks", "INTEGER")
                .not_null()
                .default("0"),

            ColumnDefinition::new("end_time_ticks", "INTEGER")
                .not_null(),

            // PLAN024 Phase 4: Segmentation - Passage boundaries
            ColumnDefinition::new("start_ticks", "INTEGER"),
            ColumnDefinition::new("end_ticks", "INTEGER"),

            // PLAN024 Phase 8: Amplitude analysis - Lead-in/lead-out timing
            ColumnDefinition::new("lead_in_start_ticks", "INTEGER"),
            ColumnDefinition::new("lead_out_start_ticks", "INTEGER"),

            // Crossfade points
            ColumnDefinition::new("fade_in_start_ticks", "INTEGER"),
            ColumnDefinition::new("fade_out_start_ticks", "INTEGER"),
            ColumnDefinition::new("fade_in_curve", "TEXT"),
            ColumnDefinition::new("fade_out_curve", "TEXT"),

            // PLAN024 Phase 8: Processing status
            ColumnDefinition::new("status", "TEXT")
                .default("'PENDING'"),

            // Metadata columns
            ColumnDefinition::new("title", "TEXT"),
            ColumnDefinition::new("user_title", "TEXT"),
            ColumnDefinition::new("artist", "TEXT"),
            ColumnDefinition::new("album", "TEXT"),
            ColumnDefinition::new("recording_mbid", "TEXT"),

            // PLAN024 Phase 9: Musical flavor
            ColumnDefinition::new("musical_flavor_vector", "TEXT"),
            ColumnDefinition::new("flavor_source_blend", "TEXT"),
            ColumnDefinition::new("flavor_confidence_map", "TEXT"),
            ColumnDefinition::new("flavor_completeness", "REAL"),

            // Metadata fusion confidence scores
            ColumnDefinition::new("title_source", "TEXT"),
            ColumnDefinition::new("title_confidence", "REAL"),
            ColumnDefinition::new("artist_source", "TEXT"),
            ColumnDefinition::new("artist_confidence", "REAL"),
            ColumnDefinition::new("album_source", "TEXT"),
            ColumnDefinition::new("album_confidence", "REAL"),
            ColumnDefinition::new("mbid_source", "TEXT"),
            ColumnDefinition::new("mbid_confidence", "REAL"),
            ColumnDefinition::new("identity_confidence", "REAL"),
            ColumnDefinition::new("identity_posterior_probability", "REAL"),
            ColumnDefinition::new("identity_conflicts", "TEXT"),
            ColumnDefinition::new("overall_quality_score", "REAL"),
            ColumnDefinition::new("metadata_completeness", "REAL"),

            // Validation
            ColumnDefinition::new("validation_status", "TEXT"),
            ColumnDefinition::new("validation_report", "TEXT"),
            ColumnDefinition::new("validation_issues", "TEXT"),

            // Import provenance
            ColumnDefinition::new("import_metadata", "TEXT"),
            ColumnDefinition::new("additional_metadata", "TEXT"),
            ColumnDefinition::new("import_session_id", "TEXT"),
            ColumnDefinition::new("import_timestamp", "TIMESTAMP"),
            ColumnDefinition::new("import_strategy", "TEXT"),

            // Decode status
            ColumnDefinition::new("decode_status", "TEXT")
                .default("'pending'"),

            ColumnDefinition::new("created_at", "TIMESTAMP")
                .not_null()
                .default("CURRENT_TIMESTAMP"),

            ColumnDefinition::new("updated_at", "TIMESTAMP")
                .not_null()
                .default("CURRENT_TIMESTAMP"),
        ]
    }
}

/// Songs table schema
///
/// Per PLAN024 Phase 7 (Recording) and Phase 9 (Flavoring)
pub struct SongsTableSchema;

impl TableSchema for SongsTableSchema {
    fn table_name() -> &'static str {
        "songs"
    }

    fn expected_columns() -> Vec<ColumnDefinition> {
        vec![
            ColumnDefinition::new("guid", "TEXT")
                .primary_key(),

            ColumnDefinition::new("recording_mbid", "TEXT")
                .not_null()
                .unique(),

            // PLAN024 Phase 7: Song metadata
            ColumnDefinition::new("title", "TEXT"),
            ColumnDefinition::new("artist_name", "TEXT"),

            // PLAN024 Phase 9: Flavor vector
            ColumnDefinition::new("flavor_vector", "TEXT"),
            ColumnDefinition::new("flavor_source_blend", "TEXT"),
            ColumnDefinition::new("status", "TEXT")
                .default("'PENDING'"),

            // Program Director parameters
            ColumnDefinition::new("work_id", "TEXT"),
            ColumnDefinition::new("related_songs", "TEXT"),
            ColumnDefinition::new("lyrics", "TEXT"),
            ColumnDefinition::new("base_probability", "REAL")
                .not_null()
                .default("1.0"),
            ColumnDefinition::new("min_cooldown", "INTEGER")
                .not_null()
                .default("604800"),
            ColumnDefinition::new("ramping_cooldown", "INTEGER")
                .not_null()
                .default("1209600"),
            ColumnDefinition::new("last_played_at", "TIMESTAMP"),

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
    SchemaSync::sync_table::<FilesTableSchema>(pool).await?;
    SchemaSync::sync_table::<PassagesTableSchema>(pool).await?;
    SchemaSync::sync_table::<SongsTableSchema>(pool).await?;

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

        assert_eq!(column_count, 18); // 18 columns total (includes artist, title, album, track_number, year, status, matching_hashes)
    }
}
