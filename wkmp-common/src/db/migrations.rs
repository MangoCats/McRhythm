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
const CURRENT_SCHEMA_VERSION: i32 = 2;

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
    sqlx::query("ALTER TABLE songs ADD COLUMN title TEXT")
        .execute(pool)
        .await?;

    info!("  ✓ Added title column to songs table");
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
