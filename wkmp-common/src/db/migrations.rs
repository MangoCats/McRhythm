//! Database schema migrations
//!
//! Implements versioned schema migrations to allow seamless database upgrades
//! without requiring manual deletion or data loss.
//!
//! **[ARCH-DB-MIG-010]** Schema migration framework
//! **[ARCH-DB-MIG-020]** Migration tracking via schema_version table
//! **[ARCH-DB-MIG-030]** Idempotent migrations (safe to run multiple times)
//!
//! # Migration Guidelines
//!
//! 1. **Never modify existing migrations** - They must remain stable for users upgrading from older versions
//! 2. **Always add new migrations** - Create a new migration function for each schema change
//! 3. **Test migrations** - Verify they work on databases with old schema
//! 4. **Document breaking changes** - If a migration cannot preserve data, document it clearly
//! 5. **Use ALTER TABLE** - Prefer ALTER TABLE over DROP/CREATE to preserve data
//!
//! # Example Migration
//!
//! ```rust,ignore
//! async fn migrate_v2(pool: &SqlitePool) -> Result<()> {
//!     // Check if column already exists (idempotency)
//!     let has_column: i64 = sqlx::query_scalar(
//!         "SELECT COUNT(*) FROM pragma_table_info('passages') WHERE name = 'new_column'"
//!     )
//!     .fetch_one(pool)
//!     .await?;
//!
//!     if has_column == 0 {
//!         sqlx::query("ALTER TABLE passages ADD COLUMN new_column TEXT")
//!             .execute(pool)
//!             .await?;
//!         info!("Migration v2: Added new_column to passages table");
//!     }
//!     Ok(())
//! }
//! ```

use crate::Result;
use sqlx::SqlitePool;
use tracing::{info, warn};

/// Current schema version
///
/// **IMPORTANT:** Increment this when adding new migrations
const CURRENT_SCHEMA_VERSION: i32 = 3;

/// Get current schema version from database
///
/// Returns 0 if schema_version table doesn't exist or has no rows
async fn get_schema_version(pool: &SqlitePool) -> Result<i32> {
    // Check if schema_version table exists
    let table_exists: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM sqlite_master
            WHERE type='table' AND name='schema_version'
        )
        "#
    )
    .fetch_one(pool)
    .await?;

    if !table_exists {
        return Ok(0);
    }

    // Get latest version
    let version: Option<i32> = sqlx::query_scalar(
        "SELECT version FROM schema_version ORDER BY version DESC LIMIT 1"
    )
    .fetch_optional(pool)
    .await?;

    Ok(version.unwrap_or(0))
}

/// Set schema version in database
async fn set_schema_version(pool: &SqlitePool, version: i32) -> Result<()> {
    sqlx::query(
        "INSERT INTO schema_version (version) VALUES (?)"
    )
    .bind(version)
    .execute(pool)
    .await?;

    Ok(())
}

/// Run all pending migrations
///
/// **[ARCH-DB-MIG-010]** Migration framework
/// **[ARCH-DB-MIG-030]** Idempotent execution
pub async fn run_migrations(pool: &SqlitePool) -> Result<()> {
    let current_version = get_schema_version(pool).await?;

    if current_version == CURRENT_SCHEMA_VERSION {
        info!("Database schema is up to date (v{})", current_version);
        return Ok(());
    }

    if current_version > CURRENT_SCHEMA_VERSION {
        warn!(
            "Database schema version ({}) is newer than code version ({})",
            current_version, CURRENT_SCHEMA_VERSION
        );
        warn!("This may indicate a downgrade. Proceeding with caution.");
        return Ok(());
    }

    info!(
        "Running database migrations: v{} -> v{}",
        current_version, CURRENT_SCHEMA_VERSION
    );

    // Run migrations sequentially
    if current_version < 1 {
        migrate_v1(pool).await?;
        set_schema_version(pool, 1).await?;
        info!("✓ Migration v1 completed");
    }

    if current_version < 2 {
        migrate_v2(pool).await?;
        set_schema_version(pool, 2).await?;
        info!("✓ Migration v2 completed");
    }

    if current_version < 3 {
        migrate_v3(pool).await?;
        set_schema_version(pool, 3).await?;
        info!("✓ Migration v3 completed");
    }

    info!("All migrations completed successfully");
    Ok(())
}

/// Migration v1: Add import_metadata column to passages table
///
/// **Background:** The passages table was initially created without an
/// import_metadata column. This migration adds it to existing databases.
///
/// **[ARCH-DB-MIG-030]** Idempotent implementation
async fn migrate_v1(pool: &SqlitePool) -> Result<()> {
    info!("Running migration v1: Add import_metadata column to passages");

    // Check if passages table exists
    let table_exists: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM sqlite_master
            WHERE type='table' AND name='passages'
        )
        "#
    )
    .fetch_one(pool)
    .await?;

    if !table_exists {
        // Table doesn't exist yet - will be created with correct schema
        info!("  Passages table doesn't exist yet - skipping migration");
        return Ok(());
    }

    // Check if import_metadata column already exists
    let has_column: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('passages') WHERE name = 'import_metadata'"
    )
    .fetch_one(pool)
    .await?;

    if has_column > 0 {
        info!("  import_metadata column already exists - skipping");
        return Ok(());
    }

    // Add the column
    sqlx::query("ALTER TABLE passages ADD COLUMN import_metadata TEXT")
        .execute(pool)
        .await?;

    info!("  ✓ Added import_metadata column to passages table");
    Ok(())
}

/// Migration v2: Add title column to songs table
///
/// **Background:** The songs table was initially created without a title column.
/// MusicBrainz provides song titles, but they were not being stored in the database.
/// This migration adds the title column to enable storing MusicBrainz song titles.
///
/// **[ARCH-DB-MIG-030]** Idempotent implementation
async fn migrate_v2(pool: &SqlitePool) -> Result<()> {
    info!("Running migration v2: Add title column to songs");

    // Check if songs table exists
    let table_exists: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM sqlite_master
            WHERE type='table' AND name='songs'
        )
        "#
    )
    .fetch_one(pool)
    .await?;

    if !table_exists {
        // Table doesn't exist yet - will be created with correct schema
        info!("  Songs table doesn't exist yet - skipping migration");
        return Ok(());
    }

    // Check if title column already exists
    let has_column: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('songs') WHERE name = 'title'"
    )
    .fetch_one(pool)
    .await?;

    if has_column > 0 {
        info!("  title column already exists - skipping");
        return Ok(());
    }

    // Add the column
    // Catch duplicate column error for concurrent initialization race conditions
    match sqlx::query("ALTER TABLE songs ADD COLUMN title TEXT")
        .execute(pool)
        .await
    {
        Ok(_) => {
            info!("  ✓ Added title column to songs table");
            Ok(())
        }
        Err(sqlx::Error::Database(db_err)) if db_err.message().contains("duplicate column") => {
            // Another thread beat us to it - that's fine
            info!("  title column added by concurrent thread - skipping");
            Ok(())
        }
        Err(e) => Err(e.into())
    }
}

/// Migration v3: PLAN023 - Add import provenance columns and table
///
/// **Background:** PLAN023 implements 3-tier hybrid fusion for audio import.
/// This migration adds 21 columns to the passages table to store identity resolution,
/// metadata provenance, musical flavor synthesis, and validation results.
/// It also creates an import_provenance table for detailed per-field tracking.
///
/// **Requirements:** REQ-AI-081 through REQ-AI-087
/// **Architecture:** 3-Tier Hybrid Fusion (PLAN023)
///
/// **[ARCH-DB-MIG-030]** Idempotent implementation
async fn migrate_v3(pool: &SqlitePool) -> Result<()> {
    info!("Running migration v3: PLAN023 Import Provenance (21 columns + import_provenance table)");

    // Check if passages table exists
    let table_exists: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM sqlite_master
            WHERE type='table' AND name='passages'
        )
        "#
    )
    .fetch_one(pool)
    .await?;

    if !table_exists {
        info!("  Passages table doesn't exist yet - skipping migration");
        return Ok(());
    }

    // Define all 21 columns to add
    let columns_to_add = vec![
        // Identity Resolution (REQ-AI-083) [3 columns]
        ("recording_mbid", "TEXT"),
        ("identity_confidence", "REAL"),
        ("identity_conflicts", "TEXT"),
        // Metadata Provenance (REQ-AI-082) [6 columns]
        ("title_source", "TEXT"),
        ("title_confidence", "REAL"),
        ("artist_source", "TEXT"),
        ("artist_confidence", "REAL"),
        ("album_source", "TEXT"),
        ("album_confidence", "REAL"),
        // Musical Flavor Synthesis (REQ-AI-081) [2 columns]
        ("flavor_source_blend", "TEXT"),
        ("flavor_confidence_map", "TEXT"),
        // Quality Validation (REQ-AI-084, REQ-AI-085) [5 columns]
        ("overall_quality_score", "REAL"),
        ("metadata_completeness", "REAL"),
        ("flavor_completeness", "REAL"),
        ("validation_status", "TEXT"),
        ("validation_report", "TEXT"),
        // Import Metadata (REQ-AI-086) [5 columns]
        ("import_session_id", "TEXT"),
        ("import_timestamp", "INTEGER"),
        ("import_strategy", "TEXT"),
        ("import_duration_ms", "INTEGER"),
        ("import_version", "TEXT"),
    ];

    // Add each column (idempotent - check if exists first)
    let mut added_count = 0;
    for (column_name, column_type) in columns_to_add {
        let has_column: i64 = sqlx::query_scalar(
            &format!("SELECT COUNT(*) FROM pragma_table_info('passages') WHERE name = '{}'", column_name)
        )
        .fetch_one(pool)
        .await?;

        if has_column == 0 {
            match sqlx::query(&format!("ALTER TABLE passages ADD COLUMN {} {}", column_name, column_type))
                .execute(pool)
                .await
            {
                Ok(_) => {
                    added_count += 1;
                }
                Err(sqlx::Error::Database(db_err)) if db_err.message().contains("duplicate column") => {
                    // Another thread beat us to it - that's fine
                    info!("  {} column added by concurrent thread - skipping", column_name);
                }
                Err(e) => return Err(e.into())
            }
        }
    }

    if added_count > 0 {
        info!("  ✓ Added {} columns to passages table", added_count);
    } else {
        info!("  All 21 columns already exist in passages table - skipping column additions");
    }

    // Create import_provenance table (REQ-AI-087)
    let provenance_table_exists: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM sqlite_master
            WHERE type='table' AND name='import_provenance'
        )
        "#
    )
    .fetch_one(pool)
    .await?;

    if !provenance_table_exists {
        sqlx::query(
            r#"
            CREATE TABLE import_provenance (
                id TEXT PRIMARY KEY,
                passage_id TEXT NOT NULL,
                source_type TEXT NOT NULL,
                data_extracted TEXT,
                confidence REAL,
                timestamp INTEGER,
                FOREIGN KEY (passage_id) REFERENCES passages(guid) ON DELETE CASCADE
            )
            "#
        )
        .execute(pool)
        .await?;

        // Create indexes
        sqlx::query("CREATE INDEX idx_import_provenance_passage_id ON import_provenance(passage_id)")
            .execute(pool)
            .await?;

        sqlx::query("CREATE INDEX idx_import_provenance_source_type ON import_provenance(source_type)")
            .execute(pool)
            .await?;

        sqlx::query("CREATE INDEX idx_import_provenance_timestamp ON import_provenance(timestamp)")
            .execute(pool)
            .await?;

        info!("  ✓ Created import_provenance table with 3 indexes");
    } else {
        info!("  import_provenance table already exists - skipping table creation");
    }

    info!("  ✓ Migration v3 complete: PLAN023 import provenance support added");
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
    async fn test_get_schema_version_no_table() {
        let pool = setup_test_db().await;
        let version = get_schema_version(&pool).await.unwrap();
        assert_eq!(version, 0);
    }

    #[tokio::test]
    async fn test_get_schema_version_empty_table() {
        let pool = setup_test_db().await;

        // Create empty schema_version table
        sqlx::query(
            "CREATE TABLE schema_version (version INTEGER PRIMARY KEY, applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP)"
        )
        .execute(&pool)
        .await
        .unwrap();

        let version = get_schema_version(&pool).await.unwrap();
        assert_eq!(version, 0);
    }

    #[tokio::test]
    async fn test_set_and_get_schema_version() {
        let pool = setup_test_db().await;

        // Create schema_version table
        sqlx::query(
            "CREATE TABLE schema_version (version INTEGER PRIMARY KEY, applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP)"
        )
        .execute(&pool)
        .await
        .unwrap();

        set_schema_version(&pool, 1).await.unwrap();
        let version = get_schema_version(&pool).await.unwrap();
        assert_eq!(version, 1);
    }

    #[tokio::test]
    async fn test_migrate_v1_no_table() {
        let pool = setup_test_db().await;

        // Should succeed even if passages table doesn't exist
        migrate_v1(&pool).await.unwrap();
    }

    #[tokio::test]
    async fn test_migrate_v1_adds_column() {
        let pool = setup_test_db().await;

        // Create passages table WITHOUT import_metadata column
        sqlx::query(
            r#"
            CREATE TABLE passages (
                guid TEXT PRIMARY KEY,
                file_id TEXT NOT NULL,
                start_time_ticks INTEGER NOT NULL,
                end_time_ticks INTEGER NOT NULL
            )
            "#
        )
        .execute(&pool)
        .await
        .unwrap();

        // Run migration
        migrate_v1(&pool).await.unwrap();

        // Verify column was added
        let has_column: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM pragma_table_info('passages') WHERE name = 'import_metadata'"
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(has_column, 1);
    }

    #[tokio::test]
    async fn test_migrate_v1_idempotent() {
        let pool = setup_test_db().await;

        // Create passages table WITH import_metadata column
        sqlx::query(
            r#"
            CREATE TABLE passages (
                guid TEXT PRIMARY KEY,
                file_id TEXT NOT NULL,
                start_time_ticks INTEGER NOT NULL,
                end_time_ticks INTEGER NOT NULL,
                import_metadata TEXT
            )
            "#
        )
        .execute(&pool)
        .await
        .unwrap();

        // Run migration twice - should not fail
        migrate_v1(&pool).await.unwrap();
        migrate_v1(&pool).await.unwrap();

        // Verify column exists only once
        let column_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM pragma_table_info('passages') WHERE name = 'import_metadata'"
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(column_count, 1);
    }

    #[tokio::test]
    async fn test_run_migrations_complete_flow() {
        let pool = setup_test_db().await;

        // Create schema_version table
        sqlx::query(
            "CREATE TABLE schema_version (version INTEGER PRIMARY KEY, applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP)"
        )
        .execute(&pool)
        .await
        .unwrap();

        // Create passages table without import_metadata
        sqlx::query(
            r#"
            CREATE TABLE passages (
                guid TEXT PRIMARY KEY,
                file_id TEXT NOT NULL,
                start_time_ticks INTEGER NOT NULL,
                end_time_ticks INTEGER NOT NULL
            )
            "#
        )
        .execute(&pool)
        .await
        .unwrap();

        // Run migrations
        run_migrations(&pool).await.unwrap();

        // Verify version was set to current version
        let version = get_schema_version(&pool).await.unwrap();
        assert_eq!(version, CURRENT_SCHEMA_VERSION);

        // Verify v1 column was added
        let has_column: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM pragma_table_info('passages') WHERE name = 'import_metadata'"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(has_column, 1);
    }
}
