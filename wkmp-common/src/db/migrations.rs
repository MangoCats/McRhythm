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
const CURRENT_SCHEMA_VERSION: i32 = 4;

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

    if current_version < 4 {
        migrate_v4(pool).await?;
        set_schema_version(pool, 4).await?;
        info!("✓ Migration v4 completed");
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

/// Migration v3: Add duration_ticks column to files table
///
/// **Background:** The files table was originally created with `duration REAL` (f64 seconds).
/// Per IMPL001-database_schema.md and SPEC017, this was changed to `duration_ticks INTEGER`
/// for consistency with passage timing representation. This migration handles existing databases.
///
/// **Breaking Change:** REQ-F-003 documented in IMPL001-database_schema.md:146-148
///
/// **[REQ-NF-036]** Zero-configuration database self-repair
/// **[REQ-NF-037]** Modules create missing tables/columns automatically
/// **[ARCH-DB-MIG-030]** Idempotent implementation
async fn migrate_v3(pool: &SqlitePool) -> Result<()> {
    info!("Running migration v3: Add duration_ticks column to files");

    // Check if files table exists
    let table_exists: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM sqlite_master
            WHERE type='table' AND name='files'
        )
        "#
    )
    .fetch_one(pool)
    .await?;

    if !table_exists {
        // Table doesn't exist yet - will be created with correct schema
        info!("  Files table doesn't exist yet - skipping migration");
        return Ok(());
    }

    // Check if duration_ticks column already exists
    let has_duration_ticks: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('files') WHERE name = 'duration_ticks'"
    )
    .fetch_one(pool)
    .await?;

    if has_duration_ticks > 0 {
        info!("  duration_ticks column already exists - skipping");
        return Ok(());
    }

    // Check if old duration column exists
    let has_duration_real: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('files') WHERE name = 'duration'"
    )
    .fetch_one(pool)
    .await?;

    // Add duration_ticks column
    // Catch duplicate column error for concurrent initialization race conditions
    match sqlx::query("ALTER TABLE files ADD COLUMN duration_ticks INTEGER")
        .execute(pool)
        .await
    {
        Ok(_) => {
            info!("  ✓ Added duration_ticks column to files table");
        }
        Err(sqlx::Error::Database(db_err)) if db_err.message().contains("duplicate column") => {
            // Another thread beat us to it - that's fine
            info!("  duration_ticks column added by concurrent thread - skipping");
            return Ok(());
        }
        Err(e) => return Err(e.into())
    }

    // Migrate data from old duration column if it exists
    if has_duration_real > 0 {
        info!("  Migrating data from duration (REAL seconds) to duration_ticks (INTEGER)");

        // Convert: ticks = CAST(seconds * 28224000 AS INTEGER)
        // 28224000 = 44100 Hz * 640 ticks per sample (WKMP tick rate)
        sqlx::query(
            r#"
            UPDATE files
            SET duration_ticks = CAST(duration * 28224000 AS INTEGER)
            WHERE duration IS NOT NULL
            "#
        )
        .execute(pool)
        .await?;

        info!("  ✓ Migrated duration values to duration_ticks");

        // Note: We do NOT drop the old duration column for safety
        // Users can manually drop it after verifying migration succeeded
        warn!("  Old 'duration' column still exists - you may manually drop it after verification");
    }

    Ok(())
}

/// Migration v4: Add format, sample_rate, channels, and file_size_bytes columns to files table
///
/// **Background:** The files table was extended to store audio metadata extracted during import.
/// These columns support better file management and display in the UI.
///
/// **[REQ-NF-036]** Zero-configuration database self-repair
/// **[REQ-NF-037]** Modules create missing tables/columns automatically
/// **[ARCH-DB-MIG-030]** Idempotent implementation
async fn migrate_v4(pool: &SqlitePool) -> Result<()> {
    info!("Running migration v4: Add audio metadata columns to files table");

    // Check if files table exists
    let table_exists: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM sqlite_master
            WHERE type='table' AND name='files'
        )
        "#
    )
    .fetch_one(pool)
    .await?;

    if !table_exists {
        // Table doesn't exist yet - will be created with correct schema
        info!("  Files table doesn't exist yet - skipping migration");
        return Ok(());
    }

    // Add format column if missing
    let has_format: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('files') WHERE name = 'format'"
    )
    .fetch_one(pool)
    .await?;

    if has_format == 0 {
        match sqlx::query("ALTER TABLE files ADD COLUMN format TEXT")
            .execute(pool)
            .await
        {
            Ok(_) => {
                info!("  ✓ Added format column to files table");
            }
            Err(sqlx::Error::Database(db_err)) if db_err.message().contains("duplicate column") => {
                info!("  format column added by concurrent thread - skipping");
            }
            Err(e) => return Err(e.into())
        }
    } else {
        info!("  format column already exists - skipping");
    }

    // Add sample_rate column if missing
    let has_sample_rate: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('files') WHERE name = 'sample_rate'"
    )
    .fetch_one(pool)
    .await?;

    if has_sample_rate == 0 {
        match sqlx::query("ALTER TABLE files ADD COLUMN sample_rate INTEGER")
            .execute(pool)
            .await
        {
            Ok(_) => {
                info!("  ✓ Added sample_rate column to files table");
            }
            Err(sqlx::Error::Database(db_err)) if db_err.message().contains("duplicate column") => {
                info!("  sample_rate column added by concurrent thread - skipping");
            }
            Err(e) => return Err(e.into())
        }
    } else {
        info!("  sample_rate column already exists - skipping");
    }

    // Add channels column if missing
    let has_channels: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('files') WHERE name = 'channels'"
    )
    .fetch_one(pool)
    .await?;

    if has_channels == 0 {
        match sqlx::query("ALTER TABLE files ADD COLUMN channels INTEGER")
            .execute(pool)
            .await
        {
            Ok(_) => {
                info!("  ✓ Added channels column to files table");
            }
            Err(sqlx::Error::Database(db_err)) if db_err.message().contains("duplicate column") => {
                info!("  channels column added by concurrent thread - skipping");
            }
            Err(e) => return Err(e.into())
        }
    } else {
        info!("  channels column already exists - skipping");
    }

    // Add file_size_bytes column if missing
    let has_file_size: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM pragma_table_info('files') WHERE name = 'file_size_bytes'"
    )
    .fetch_one(pool)
    .await?;

    if has_file_size == 0 {
        match sqlx::query("ALTER TABLE files ADD COLUMN file_size_bytes INTEGER")
            .execute(pool)
            .await
        {
            Ok(_) => {
                info!("  ✓ Added file_size_bytes column to files table");
            }
            Err(sqlx::Error::Database(db_err)) if db_err.message().contains("duplicate column") => {
                info!("  file_size_bytes column added by concurrent thread - skipping");
            }
            Err(e) => return Err(e.into())
        }
    } else {
        info!("  file_size_bytes column already exists - skipping");
    }

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

    // ============================================================================
    // Migration v3 Tests (duration_ticks column)
    // ============================================================================

    #[tokio::test]
    async fn test_migrate_v3_no_table() {
        let pool = setup_test_db().await;

        // Should succeed even if files table doesn't exist
        migrate_v3(&pool).await.unwrap();
    }

    #[tokio::test]
    async fn test_migrate_v3_adds_column() {
        let pool = setup_test_db().await;

        // Create files table WITHOUT duration_ticks column (old schema)
        sqlx::query(
            r#"
            CREATE TABLE files (
                guid TEXT PRIMARY KEY,
                path TEXT NOT NULL UNIQUE,
                hash TEXT NOT NULL,
                duration REAL,
                modification_time TIMESTAMP NOT NULL
            )
            "#
        )
        .execute(&pool)
        .await
        .unwrap();

        // Run migration
        migrate_v3(&pool).await.unwrap();

        // Verify duration_ticks column was added
        let has_column: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM pragma_table_info('files') WHERE name = 'duration_ticks'"
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(has_column, 1);
    }

    #[tokio::test]
    async fn test_migrate_v3_migrates_data() {
        let pool = setup_test_db().await;

        // Create files table with old schema
        sqlx::query(
            r#"
            CREATE TABLE files (
                guid TEXT PRIMARY KEY,
                path TEXT NOT NULL UNIQUE,
                hash TEXT NOT NULL,
                duration REAL,
                modification_time TIMESTAMP NOT NULL
            )
            "#
        )
        .execute(&pool)
        .await
        .unwrap();

        // Insert test data with duration in seconds
        sqlx::query(
            r#"
            INSERT INTO files (guid, path, hash, duration, modification_time)
            VALUES
                ('file1', 'test1.mp3', 'hash1', 175.5, '2025-01-01 00:00:00'),
                ('file2', 'test2.mp3', 'hash2', 240.0, '2025-01-01 00:00:00'),
                ('file3', 'test3.mp3', 'hash3', NULL, '2025-01-01 00:00:00')
            "#
        )
        .execute(&pool)
        .await
        .unwrap();

        // Run migration
        migrate_v3(&pool).await.unwrap();

        // Verify data was migrated correctly
        // Conversion: ticks = CAST(seconds * 28224000 AS INTEGER)

        // File 1: 175.5 seconds
        let file1_ticks: Option<i64> = sqlx::query_scalar(
            "SELECT duration_ticks FROM files WHERE guid = 'file1'"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(file1_ticks, Some(4953312000)); // 175.5 * 28224000 = 4953312000

        // File 2: 240.0 seconds
        let file2_ticks: Option<i64> = sqlx::query_scalar(
            "SELECT duration_ticks FROM files WHERE guid = 'file2'"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(file2_ticks, Some(6773760000)); // 240.0 * 28224000 = 6773760000

        // File 3: NULL duration should remain NULL
        let file3_ticks: Option<i64> = sqlx::query_scalar(
            "SELECT duration_ticks FROM files WHERE guid = 'file3'"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(file3_ticks, None);
    }

    #[tokio::test]
    async fn test_migrate_v3_idempotent() {
        let pool = setup_test_db().await;

        // Create files table with old schema and data
        sqlx::query(
            r#"
            CREATE TABLE files (
                guid TEXT PRIMARY KEY,
                path TEXT NOT NULL UNIQUE,
                hash TEXT NOT NULL,
                duration REAL,
                modification_time TIMESTAMP NOT NULL
            )
            "#
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            r#"
            INSERT INTO files (guid, path, hash, duration, modification_time)
            VALUES ('file1', 'test.mp3', 'hash1', 100.0, '2025-01-01 00:00:00')
            "#
        )
        .execute(&pool)
        .await
        .unwrap();

        // Run migration first time
        migrate_v3(&pool).await.unwrap();

        let ticks_first: Option<i64> = sqlx::query_scalar(
            "SELECT duration_ticks FROM files WHERE guid = 'file1'"
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        // Run migration second time - should not fail
        migrate_v3(&pool).await.unwrap();

        let ticks_second: Option<i64> = sqlx::query_scalar(
            "SELECT duration_ticks FROM files WHERE guid = 'file1'"
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        // Values should be unchanged
        assert_eq!(ticks_first, ticks_second);
        assert_eq!(ticks_first, Some(2822400000)); // 100.0 * 28224000

        // Verify column exists only once
        let column_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM pragma_table_info('files') WHERE name = 'duration_ticks'"
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(column_count, 1);
    }

    #[tokio::test]
    async fn test_migrate_v3_preserves_old_column() {
        let pool = setup_test_db().await;

        // Create files table with old schema
        sqlx::query(
            r#"
            CREATE TABLE files (
                guid TEXT PRIMARY KEY,
                path TEXT NOT NULL UNIQUE,
                hash TEXT NOT NULL,
                duration REAL,
                modification_time TIMESTAMP NOT NULL
            )
            "#
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            r#"
            INSERT INTO files (guid, path, hash, duration, modification_time)
            VALUES ('file1', 'test.mp3', 'hash1', 150.0, '2025-01-01 00:00:00')
            "#
        )
        .execute(&pool)
        .await
        .unwrap();

        // Run migration
        migrate_v3(&pool).await.unwrap();

        // Verify old duration column still exists
        let has_duration: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM pragma_table_info('files') WHERE name = 'duration'"
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(has_duration, 1, "Old duration column should be preserved");

        // Verify old duration value is unchanged
        let duration: Option<f64> = sqlx::query_scalar(
            "SELECT duration FROM files WHERE guid = 'file1'"
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(duration, Some(150.0), "Old duration value should be unchanged");

        // Verify new duration_ticks column exists and has correct value
        let duration_ticks: Option<i64> = sqlx::query_scalar(
            "SELECT duration_ticks FROM files WHERE guid = 'file1'"
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(duration_ticks, Some(4233600000)); // 150.0 * 28224000
    }

    #[tokio::test]
    async fn test_migrate_v3_with_new_schema() {
        let pool = setup_test_db().await;

        // Create files table with NEW schema (already has duration_ticks)
        sqlx::query(
            r#"
            CREATE TABLE files (
                guid TEXT PRIMARY KEY,
                path TEXT NOT NULL UNIQUE,
                hash TEXT NOT NULL,
                duration_ticks INTEGER,
                modification_time TIMESTAMP NOT NULL
            )
            "#
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            r#"
            INSERT INTO files (guid, path, hash, duration_ticks, modification_time)
            VALUES ('file1', 'test.mp3', 'hash1', 5000000000, '2025-01-01 00:00:00')
            "#
        )
        .execute(&pool)
        .await
        .unwrap();

        // Run migration - should skip gracefully
        migrate_v3(&pool).await.unwrap();

        // Verify data is unchanged
        let ticks: Option<i64> = sqlx::query_scalar(
            "SELECT duration_ticks FROM files WHERE guid = 'file1'"
        )
        .fetch_one(&pool)
        .await
        .unwrap();

        assert_eq!(ticks, Some(5000000000));
    }

    // ============================================================================
    // Migration v4 Tests (audio metadata columns)
    // ============================================================================

    #[tokio::test]
    async fn test_migrate_v4_no_table() {
        let pool = setup_test_db().await;

        // Should succeed even if files table doesn't exist
        migrate_v4(&pool).await.unwrap();
    }

    #[tokio::test]
    async fn test_migrate_v4_adds_columns() {
        let pool = setup_test_db().await;

        // Create files table WITHOUT audio metadata columns (old schema from v3)
        sqlx::query(
            r#"
            CREATE TABLE files (
                guid TEXT PRIMARY KEY,
                path TEXT NOT NULL UNIQUE,
                hash TEXT NOT NULL,
                duration_ticks INTEGER,
                modification_time TIMESTAMP NOT NULL
            )
            "#
        )
        .execute(&pool)
        .await
        .unwrap();

        // Run migration
        migrate_v4(&pool).await.unwrap();

        // Verify all columns were added
        let has_format: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM pragma_table_info('files') WHERE name = 'format'"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(has_format, 1, "format column should be added");

        let has_sample_rate: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM pragma_table_info('files') WHERE name = 'sample_rate'"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(has_sample_rate, 1, "sample_rate column should be added");

        let has_channels: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM pragma_table_info('files') WHERE name = 'channels'"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(has_channels, 1, "channels column should be added");

        let has_file_size: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM pragma_table_info('files') WHERE name = 'file_size_bytes'"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(has_file_size, 1, "file_size_bytes column should be added");
    }

    #[tokio::test]
    async fn test_migrate_v4_idempotent() {
        let pool = setup_test_db().await;

        // Create files table with old schema
        sqlx::query(
            r#"
            CREATE TABLE files (
                guid TEXT PRIMARY KEY,
                path TEXT NOT NULL UNIQUE,
                hash TEXT NOT NULL,
                duration_ticks INTEGER,
                modification_time TIMESTAMP NOT NULL
            )
            "#
        )
        .execute(&pool)
        .await
        .unwrap();

        // Run migration twice - should not fail
        migrate_v4(&pool).await.unwrap();
        migrate_v4(&pool).await.unwrap();

        // Verify columns exist only once each
        let format_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM pragma_table_info('files') WHERE name = 'format'"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(format_count, 1);

        let sample_rate_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM pragma_table_info('files') WHERE name = 'sample_rate'"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(sample_rate_count, 1);

        let channels_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM pragma_table_info('files') WHERE name = 'channels'"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(channels_count, 1);

        let file_size_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM pragma_table_info('files') WHERE name = 'file_size_bytes'"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(file_size_count, 1);
    }

    #[tokio::test]
    async fn test_migrate_v4_with_new_schema() {
        let pool = setup_test_db().await;

        // Create files table with NEW schema (already has audio metadata columns)
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
                modification_time TIMESTAMP NOT NULL
            )
            "#
        )
        .execute(&pool)
        .await
        .unwrap();

        // Insert test data
        sqlx::query(
            r#"
            INSERT INTO files (guid, path, hash, duration_ticks, format, sample_rate, channels, file_size_bytes, modification_time)
            VALUES ('file1', 'test.flac', 'hash1', 5000000000, 'FLAC', 44100, 2, 1048576, '2025-01-01 00:00:00')
            "#
        )
        .execute(&pool)
        .await
        .unwrap();

        // Run migration - should skip gracefully
        migrate_v4(&pool).await.unwrap();

        // Verify data is unchanged
        let format: Option<String> = sqlx::query_scalar(
            "SELECT format FROM files WHERE guid = 'file1'"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(format, Some("FLAC".to_string()));

        let sample_rate: Option<i32> = sqlx::query_scalar(
            "SELECT sample_rate FROM files WHERE guid = 'file1'"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(sample_rate, Some(44100));

        let channels: Option<i32> = sqlx::query_scalar(
            "SELECT channels FROM files WHERE guid = 'file1'"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(channels, Some(2));

        let file_size: Option<i64> = sqlx::query_scalar(
            "SELECT file_size_bytes FROM files WHERE guid = 'file1'"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(file_size, Some(1048576));
    }

    #[tokio::test]
    async fn test_migrate_v4_partial_columns() {
        let pool = setup_test_db().await;

        // Create files table with some columns already present
        sqlx::query(
            r#"
            CREATE TABLE files (
                guid TEXT PRIMARY KEY,
                path TEXT NOT NULL UNIQUE,
                hash TEXT NOT NULL,
                duration_ticks INTEGER,
                format TEXT,
                modification_time TIMESTAMP NOT NULL
            )
            "#
        )
        .execute(&pool)
        .await
        .unwrap();

        // Run migration - should only add missing columns
        migrate_v4(&pool).await.unwrap();

        // Verify all columns exist
        let has_format: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM pragma_table_info('files') WHERE name = 'format'"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(has_format, 1);

        let has_sample_rate: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM pragma_table_info('files') WHERE name = 'sample_rate'"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(has_sample_rate, 1);

        let has_channels: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM pragma_table_info('files') WHERE name = 'channels'"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(has_channels, 1);

        let has_file_size: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM pragma_table_info('files') WHERE name = 'file_size_bytes'"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(has_file_size, 1);
    }
}
