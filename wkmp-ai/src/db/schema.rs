//! Database Schema Synchronization for WKMP-AI
//!
//! Implements automatic schema maintenance using SPEC031 SchemaSync.
//! Defines expected schemas for passages and files tables with PLAN024 extensions.
//!
//! # Implementation
//! - TASK-003: Database Schema Sync (PLAN024)
//! - REQ-AI-080-086: Passages table extensions (17 columns)
//! - REQ-AI-009-01: Files table extensions (7 columns, Amendment 8)
//!
//! # Architecture
//! Per SPEC031, schema definitions are the single source of truth.
//! SchemaSync automatically adds missing columns on module startup.

use sqlx::SqlitePool;
use wkmp_common::db::schema_sync::{ColumnDefinition, SchemaSync, TableSchema};
use wkmp_common::Result;

// ============================================================================
// Passages Table Schema
// ============================================================================

/// Passages table schema with PLAN024 extensions
///
/// **Base columns:** Existing passage fields (guid, file_id, timing, metadata)
/// **PLAN024 extensions (17 columns):** Source provenance, quality scores, validation
///
/// **Requirements:**
/// - REQ-AI-081: Flavor source provenance (2 columns)
/// - REQ-AI-082: Metadata source provenance (4 columns)
/// - REQ-AI-083: Identity resolution tracking (3 columns)
/// - REQ-AI-084: Quality scores (3 columns)
/// - REQ-AI-085: Validation flags (2 columns)
/// - REQ-AI-086: Import metadata (3 columns)
pub struct PassagesTableSchema;

impl TableSchema for PassagesTableSchema {
    fn table_name() -> &'static str {
        "passages"
    }

    fn expected_columns() -> Vec<ColumnDefinition> {
        vec![
            // ----------------------------------------------------------------
            // Base columns (existing)
            // ----------------------------------------------------------------
            ColumnDefinition::new("guid", "TEXT").primary_key(),
            ColumnDefinition::new("file_id", "TEXT").not_null(),
            ColumnDefinition::new("start_time_ticks", "INTEGER").not_null(),
            ColumnDefinition::new("fade_in_start_ticks", "INTEGER"),
            ColumnDefinition::new("lead_in_start_ticks", "INTEGER"),
            ColumnDefinition::new("lead_out_start_ticks", "INTEGER"),
            ColumnDefinition::new("fade_out_start_ticks", "INTEGER"),
            ColumnDefinition::new("end_time_ticks", "INTEGER").not_null(),
            ColumnDefinition::new("fade_in_curve", "TEXT"),
            ColumnDefinition::new("fade_out_curve", "TEXT"),
            ColumnDefinition::new("title", "TEXT"),
            ColumnDefinition::new("user_title", "TEXT"),
            ColumnDefinition::new("artist", "TEXT"),
            ColumnDefinition::new("album", "TEXT"),
            ColumnDefinition::new("musical_flavor_vector", "TEXT"),
            ColumnDefinition::new("import_metadata", "TEXT"),
            ColumnDefinition::new("additional_metadata", "TEXT"),
            ColumnDefinition::new("created_at", "TEXT"),
            ColumnDefinition::new("updated_at", "TEXT"),
            // ----------------------------------------------------------------
            // PLAN024 extensions (17 columns)
            // ----------------------------------------------------------------
            // [REQ-AI-081] Flavor source provenance
            ColumnDefinition::new("flavor_source_blend", "TEXT"),
            ColumnDefinition::new("flavor_confidence_map", "TEXT"),
            // [REQ-AI-082] Metadata source provenance
            ColumnDefinition::new("title_source", "TEXT"),
            ColumnDefinition::new("title_confidence", "REAL"),
            ColumnDefinition::new("artist_source", "TEXT"),
            ColumnDefinition::new("artist_confidence", "REAL"),
            // [REQ-AI-083] Identity resolution tracking
            ColumnDefinition::new("recording_mbid", "TEXT"),
            ColumnDefinition::new("identity_confidence", "REAL"),
            ColumnDefinition::new("identity_conflicts", "TEXT"),
            // [REQ-AI-084] Quality scores
            ColumnDefinition::new("overall_quality_score", "REAL"),
            ColumnDefinition::new("metadata_completeness", "REAL"),
            ColumnDefinition::new("flavor_completeness", "REAL"),
            // [REQ-AI-085] Validation flags
            ColumnDefinition::new("validation_status", "TEXT"),
            ColumnDefinition::new("validation_report", "TEXT"),
            // [REQ-AI-086] Import metadata
            ColumnDefinition::new("import_session_id", "TEXT"),
            ColumnDefinition::new("import_timestamp", "INTEGER"),
            ColumnDefinition::new("import_strategy", "TEXT"),
        ]
    }

    fn validate_schema(_pool: &SqlitePool) -> Result<()> {
        // Custom validation could check:
        // - Foreign key constraints
        // - Index existence
        // - Trigger presence
        // For now, rely on auto-sync only
        Ok(())
    }
}

// ============================================================================
// Files Table Schema
// ============================================================================

/// Files table schema with PLAN024 Amendment 8 extensions
///
/// **Base columns:** Existing file fields (guid, path, hash, format, etc.)
/// **Amendment 8 extensions (7 columns):** File-level import tracking
///
/// **Requirements:**
/// - REQ-AI-009-01: File-level import completion tracking
/// - REQ-AI-009-01: Import success confidence
/// - REQ-AI-009-01: Metadata import completion tracking
/// - REQ-AI-009-01: Metadata confidence
/// - REQ-AI-009-01: User approval timestamp
/// - REQ-AI-009-01: Re-import attempt counter
/// - REQ-AI-009-01: Last re-import attempt timestamp
pub struct FilesTableSchema;

impl TableSchema for FilesTableSchema {
    fn table_name() -> &'static str {
        "files"
    }

    fn expected_columns() -> Vec<ColumnDefinition> {
        vec![
            // ----------------------------------------------------------------
            // Base columns (existing)
            // ----------------------------------------------------------------
            ColumnDefinition::new("guid", "TEXT").primary_key(),
            ColumnDefinition::new("path", "TEXT").not_null().unique(),
            ColumnDefinition::new("hash", "TEXT"),
            ColumnDefinition::new("duration_ticks", "INTEGER"),
            ColumnDefinition::new("format", "TEXT"),
            ColumnDefinition::new("sample_rate", "INTEGER"),
            ColumnDefinition::new("channels", "INTEGER"),
            ColumnDefinition::new("file_size_bytes", "INTEGER"),
            ColumnDefinition::new("modification_time", "TEXT"),
            ColumnDefinition::new("created_at", "TEXT"),
            ColumnDefinition::new("updated_at", "TEXT"),
            // ----------------------------------------------------------------
            // Amendment 8 extensions (7 columns)
            // ----------------------------------------------------------------
            // [REQ-AI-009-01] File-level import tracking
            ColumnDefinition::new("import_completed_at", "INTEGER"),
            ColumnDefinition::new("import_success_confidence", "REAL"),
            ColumnDefinition::new("metadata_import_completed_at", "INTEGER"),
            ColumnDefinition::new("metadata_confidence", "REAL"),
            ColumnDefinition::new("user_approved_at", "INTEGER"),
            ColumnDefinition::new("reimport_attempt_count", "INTEGER")
                .not_null()
                .default("0"),
            ColumnDefinition::new("last_reimport_attempt_at", "INTEGER"),
        ]
    }

    fn validate_schema(_pool: &SqlitePool) -> Result<()> {
        // Custom validation could check:
        // - Foreign key constraints
        // - Index existence
        // - Unique constraint on path
        // For now, rely on auto-sync only
        Ok(())
    }
}

// ============================================================================
// Schema Initialization
// ============================================================================

/// Initialize all WKMP-AI database schemas
///
/// Syncs passages and files tables with expected schemas.
/// Per SPEC031, automatically adds missing columns via ALTER TABLE.
///
/// # Errors
/// Returns error if:
/// - Database connection fails
/// - Schema introspection fails
/// - Column addition fails
/// - Type/constraint mismatch detected (requires manual migration)
///
/// # Usage
/// Call once during module initialization (after database opened):
/// ```rust,ignore
/// use wkmp_ai::db::schema::init_schemas;
///
/// let pool = SqlitePool::connect(&db_path).await?;
/// init_schemas(&pool).await?;
/// ```
pub async fn init_schemas(pool: &SqlitePool) -> Result<()> {
    tracing::info!("Initializing WKMP-AI database schemas (SPEC031 auto-sync)");

    // Sync passages table (17 new columns)
    SchemaSync::sync_table::<PassagesTableSchema>(pool).await?;

    tracing::info!("✓ Passages table schema synchronized (17 PLAN024 columns)");

    // Sync files table (7 new columns)
    SchemaSync::sync_table::<FilesTableSchema>(pool).await?;

    tracing::info!("✓ Files table schema synchronized (7 Amendment 8 columns)");

    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_passages_table_column_count() {
        let columns = PassagesTableSchema::expected_columns();
        // 19 base + 17 PLAN024 = 36 total
        assert_eq!(columns.len(), 36, "Passages table should have 36 columns");
    }

    #[test]
    fn test_files_table_column_count() {
        let columns = FilesTableSchema::expected_columns();
        // 11 base + 7 Amendment 8 = 18 total
        assert_eq!(columns.len(), 18, "Files table should have 18 columns");
    }

    #[test]
    fn test_passages_table_name() {
        assert_eq!(PassagesTableSchema::table_name(), "passages");
    }

    #[test]
    fn test_files_table_name() {
        assert_eq!(FilesTableSchema::table_name(), "files");
    }

    #[test]
    fn test_passages_primary_key() {
        let columns = PassagesTableSchema::expected_columns();
        let guid_column = columns.iter().find(|c| c.name == "guid").unwrap();
        assert!(guid_column.primary_key, "guid should be primary key");
    }

    #[test]
    fn test_files_primary_key() {
        let columns = FilesTableSchema::expected_columns();
        let guid_column = columns.iter().find(|c| c.name == "guid").unwrap();
        assert!(guid_column.primary_key, "guid should be primary key");
    }

    #[test]
    fn test_files_path_unique() {
        let columns = FilesTableSchema::expected_columns();
        let path_column = columns.iter().find(|c| c.name == "path").unwrap();
        assert!(path_column.unique, "path should be unique");
        assert!(path_column.not_null, "path should be not null");
    }

    #[test]
    fn test_reimport_attempt_count_default() {
        let columns = FilesTableSchema::expected_columns();
        let counter_column = columns
            .iter()
            .find(|c| c.name == "reimport_attempt_count")
            .unwrap();
        assert!(counter_column.not_null, "reimport_attempt_count should be NOT NULL");
        assert_eq!(
            counter_column.default_value,
            Some("0".to_string()),
            "reimport_attempt_count should default to 0"
        );
    }

    #[test]
    fn test_plan024_columns_present() {
        let columns = PassagesTableSchema::expected_columns();
        let plan024_columns = vec![
            "flavor_source_blend",
            "flavor_confidence_map",
            "title_source",
            "title_confidence",
            "artist_source",
            "artist_confidence",
            "recording_mbid",
            "identity_confidence",
            "identity_conflicts",
            "overall_quality_score",
            "metadata_completeness",
            "flavor_completeness",
            "validation_status",
            "validation_report",
            "import_session_id",
            "import_timestamp",
            "import_strategy",
        ];

        for column_name in plan024_columns {
            assert!(
                columns.iter().any(|c| c.name == column_name),
                "PLAN024 column '{}' should be present",
                column_name
            );
        }
    }

    #[test]
    fn test_amendment8_columns_present() {
        let columns = FilesTableSchema::expected_columns();
        let amendment8_columns = vec![
            "import_completed_at",
            "import_success_confidence",
            "metadata_import_completed_at",
            "metadata_confidence",
            "user_approved_at",
            "reimport_attempt_count",
            "last_reimport_attempt_at",
        ];

        for column_name in amendment8_columns {
            assert!(
                columns.iter().any(|c| c.name == column_name),
                "Amendment 8 column '{}' should be present",
                column_name
            );
        }
    }
}
