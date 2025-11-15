//! Database initialization
//!
//! Implements graceful degradation for database initialization as specified in:
//! - [REQ-NF-036]: Automatic database creation with default schema
//! - [ARCH-INIT-010]: Module startup sequence
//! - [ARCH-INIT-020]: Default value initialization behavior
//! - [DEP-DB-011]: Database initialization on first run

use crate::Result;
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
use std::path::Path;
use tracing::{info, warn};

/// Initialize database connection and create tables if needed [REQ-NF-036]
pub async fn init_database(db_path: &Path) -> Result<SqlitePool> {
    let newly_created = !db_path.exists();

    // Create parent directory if it doesn't exist
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Use sqlite options to create database if it doesn't exist
    // **[ARCH-PERF-020]** Increase connection pool size for concurrent write operations
    // Default is 10, increasing to 20 to reduce lock contention during parallel import
    let db_url = format!("sqlite://{}?mode=rwc", db_path.display());
    let pool = SqlitePoolOptions::new()
        .max_connections(20)
        .min_connections(5)
        .connect(&db_url)
        .await?;

    if newly_created {
        info!("Initialized new database: {}", db_path.display());
    } else {
        info!("Opened existing database: {}", db_path.display());
    }

    // Enable foreign keys
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await?;

    // **[ARCH-PERF-010]** Enable WAL mode for better write concurrency
    // WAL (Write-Ahead Logging) allows concurrent readers with one writer
    // Critical for multi-threaded import workflow with multiple worker threads
    sqlx::query("PRAGMA journal_mode = WAL")
        .execute(&pool)
        .await?;

    // Set busy timeout [ARCH-ERRH-070]
    // Read from settings table after it's created, or use default 5000ms
    // This will be re-applied after init_default_settings() creates the setting
    sqlx::query("PRAGMA busy_timeout = 5000")
        .execute(&pool)
        .await?;

    // Run migrations (idempotent - safe to call multiple times)
    create_schema_version_table(&pool).await?;
    create_users_table(&pool).await?;
    create_settings_table(&pool).await?;
    create_module_config_table(&pool).await?;
    create_files_table(&pool).await?;
    create_passages_table(&pool).await?;
    create_import_provenance_table(&pool).await?;
    create_queue_table(&pool).await?;
    create_acoustid_cache_table(&pool).await?;

    // MusicBrainz entity tables (used by wkmp-ai, wkmp-pd)
    create_songs_table(&pool).await?;
    create_artists_table(&pool).await?;
    create_works_table(&pool).await?;
    create_albums_table(&pool).await?;
    create_images_table(&pool).await?;

    // Linking tables
    create_passage_songs_table(&pool).await?;
    create_song_artists_table(&pool).await?;
    create_passage_albums_table(&pool).await?;

    // Audio Ingest workflow tables (wkmp-ai specific)
    create_import_sessions_table(&pool).await?;
    create_temp_file_songs_table(&pool).await?;
    create_temp_file_albums_table(&pool).await?;

    // Phase 2: Automatic Schema Synchronization [ARCH-DB-SYNC-020]
    // Automatically add missing columns to existing tables
    // This runs AFTER CREATE TABLE IF NOT EXISTS and BEFORE manual migrations
    crate::db::table_schemas::sync_all_table_schemas(&pool).await?;

    // Phase 3: Manual Migrations [ARCH-DB-MIG-010]
    // Complex transformations (type changes, data migration, etc.)
    // This must run AFTER auto-sync to handle edge cases
    crate::db::migrations::run_migrations(&pool).await?;

    // Phase 4: Initialize default settings [ARCH-INIT-020]
    init_default_settings(&pool).await?;

    // Apply configurable busy timeout from settings [ARCH-ERRH-070]
    // Use ai_database_lock_retry_ms (default 250ms) for SQLite busy_timeout
    // This allows shorter lock waits before returning errors, enabling retry logic
    // to handle contention with exponential backoff up to ai_database_max_lock_wait_ms
    let timeout_ms: i64 = sqlx::query_scalar(
        "SELECT CAST(value AS INTEGER) FROM settings WHERE key = 'ai_database_lock_retry_ms'"
    )
    .fetch_optional(&pool)
    .await?
    .unwrap_or(250);

    let pragma_sql = format!("PRAGMA busy_timeout = {}", timeout_ms);
    sqlx::query(&pragma_sql)
        .execute(&pool)
        .await?;

    info!("Database busy timeout set to {} ms", timeout_ms);

    Ok(pool)
}

async fn create_schema_version_table(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY,
            applied_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn create_users_table(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            guid TEXT PRIMARY KEY,
            username TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            password_salt TEXT NOT NULL,
            config_interface_access INTEGER NOT NULL DEFAULT 1,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create Anonymous user if it doesn't exist
    sqlx::query(
        r#"
        INSERT OR IGNORE INTO users (guid, username, password_hash, password_salt, config_interface_access)
        VALUES ('00000000-0000-0000-0000-000000000001', 'Anonymous', '', '', 1)
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Create the settings table
///
/// Stores application configuration key-value pairs.
pub async fn create_settings_table(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Initialize or update default settings [ARCH-INIT-020]
///
/// This function ensures all required settings exist with default values.
/// It also handles NULL values by resetting them to defaults.
async fn init_default_settings(pool: &SqlitePool) -> Result<()> {
    // Core playback settings
    ensure_setting(pool, "initial_play_state", "playing").await?;
    ensure_setting(pool, "volume_level", "0.5").await?;
    ensure_setting(pool, "global_crossfade_time", "2.0").await?;
    ensure_setting(pool, "volume_fade_update_period", "10").await?;

    // Audio Player settings
    ensure_setting(pool, "audio_sink", "default").await?;
    ensure_setting(pool, "position_event_interval_ms", "1000").await?;
    ensure_setting(pool, "playback_progress_interval_ms", "5000").await?;

    // Queue management settings
    ensure_setting(pool, "queue_max_size", "100").await?;
    ensure_setting(pool, "queue_refill_threshold_passages", "2").await?;
    ensure_setting(pool, "queue_refill_threshold_seconds", "900").await?;  // 15 minutes
    ensure_setting(pool, "queue_refill_request_throttle_seconds", "10").await?;
    ensure_setting(pool, "queue_refill_acknowledgment_timeout_seconds", "5").await?;
    ensure_setting(pool, "queue_max_enqueue_batch", "5").await?;

    // Session and authentication settings
    ensure_setting(pool, "session_timeout_seconds", "31536000").await?;  // 1 year

    // Backup settings
    ensure_setting(pool, "backup_interval_ms", "7776000000").await?;  // 90 days
    ensure_setting(pool, "backup_minimum_interval_ms", "1209600000").await?;  // 14 days
    ensure_setting(pool, "backup_retention_count", "3").await?;
    ensure_setting(pool, "backup_location", "").await?;  // Empty = same folder as db
    ensure_setting(pool, "last_backup_timestamp_ms", "0").await?;

    // HTTP server settings
    ensure_setting(pool, "http_base_ports", "[5720, 15720, 25720, 17200, 23400]").await?;
    ensure_setting(pool, "http_request_timeout_ms", "30000").await?;
    ensure_setting(pool, "http_keepalive_timeout_ms", "60000").await?;
    ensure_setting(pool, "http_max_body_size_bytes", "1048576").await?;

    // Module launch/relaunch settings
    ensure_setting(pool, "relaunch_delay", "5000").await?;  // 5 seconds
    ensure_setting(pool, "relaunch_attempts", "20").await?;

    // Error handling settings
    ensure_setting(pool, "playback_failure_threshold", "3").await?;
    ensure_setting(pool, "playback_failure_window_seconds", "60").await?;

    // Audio Ingest settings (Full version)
    ensure_setting(pool, "ingest_max_concurrent_jobs", "4").await?;
    ensure_setting(pool, "ai_database_max_lock_wait_ms", "5000").await?;
    ensure_setting(pool, "ai_database_lock_retry_ms", "250").await?;

    // Validation service settings **[ARCH-AUTO-VAL-001]**
    ensure_setting(pool, "validation_enabled", "true").await?;              // [DBD-PARAM-130]
    ensure_setting(pool, "validation_interval_secs", "10").await?;          // [DBD-PARAM-131]
    ensure_setting(pool, "validation_tolerance_samples", "8192").await?;    // [DBD-PARAM-132]

    // GlobalParams defaults **[PLAN018]** - All 15 database-backed parameters
    // These match the defaults in wkmp-common/src/params.rs
    ensure_setting(pool, "working_sample_rate", "44100").await?;              // [DBD-PARAM-020]
    ensure_setting(pool, "output_ringbuffer_size", "8192").await?;            // [DBD-PARAM-030]
    ensure_setting(pool, "maximum_decode_streams", "12").await?;              // [DBD-PARAM-050]
    ensure_setting(pool, "decode_work_period", "5000").await?;                // [DBD-PARAM-060]
    ensure_setting(pool, "chunk_duration_ms", "1000").await?;                 // [DBD-PARAM-065]
    ensure_setting(pool, "playout_ringbuffer_size", "661941").await?;         // [DBD-PARAM-070]
    ensure_setting(pool, "playout_ringbuffer_headroom", "4410").await?;       // [DBD-PARAM-080]
    ensure_setting(pool, "decoder_resume_hysteresis_samples", "44100").await?; // [DBD-PARAM-085]
    ensure_setting(pool, "mixer_min_start_level", "22050").await?;            // [DBD-PARAM-088]
    ensure_setting(pool, "pause_decay_factor", "0.95").await?;                // [DBD-PARAM-090]
    ensure_setting(pool, "pause_decay_floor", "0.0001778").await?;            // [DBD-PARAM-100]
    ensure_setting(pool, "audio_buffer_size", "2208").await?;                 // [DBD-PARAM-110]
    ensure_setting(pool, "mixer_check_interval_ms", "10").await?;             // [DBD-PARAM-111]
    // Note: volume_level (DBD-PARAM-010) already set above in "Core playback settings"

    info!("Default settings initialized");
    Ok(())
}

/// Ensure a setting exists with the specified default value [ARCH-INIT-020]
///
/// If the setting doesn't exist, it will be created with the default.
/// If the setting exists but has a NULL value, it will be reset to the default.
async fn ensure_setting(pool: &SqlitePool, key: &str, default_value: &str) -> Result<()> {
    // Check if setting exists
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM settings WHERE key = ?)"
    )
    .bind(key)
    .fetch_one(pool)
    .await?;

    if !exists {
        // Setting doesn't exist - create it
        // Use INSERT OR IGNORE to handle concurrent initialization race conditions
        // Multiple threads may pass the exists check simultaneously
        sqlx::query(
            "INSERT OR IGNORE INTO settings (key, value) VALUES (?, ?)"
        )
        .bind(key)
        .bind(default_value)
        .execute(pool)
        .await?;

        info!("Initialized setting '{}' with default value: {}", key, default_value);
        return Ok(());
    }

    // Check if value is NULL [ARCH-INIT-020 rule 4]
    let value: Option<String> = sqlx::query_scalar(
        "SELECT value FROM settings WHERE key = ?"
    )
    .bind(key)
    .fetch_one(pool)
    .await?;

    if value.is_none() {
        // Value is NULL - reset to default
        sqlx::query(
            "UPDATE settings SET value = ? WHERE key = ?"
        )
        .bind(default_value)
        .bind(key)
        .execute(pool)
        .await?;

        warn!("Setting '{}' was NULL, reset to default: {}", key, default_value);
    }

    Ok(())
}

async fn create_module_config_table(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS module_config (
            module_name TEXT PRIMARY KEY CHECK (module_name IN ('audio_player', 'user_interface', 'program_director', 'audio_ingest', 'lyric_editor')),
            host TEXT NOT NULL,
            port INTEGER NOT NULL CHECK (port > 0 AND port <= 65535),
            enabled INTEGER NOT NULL DEFAULT 1,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Initialize default module configurations
    let defaults = vec![
        ("user_interface", "127.0.0.1", 5720),
        ("audio_player", "127.0.0.1", 5721),
        ("program_director", "127.0.0.1", 5722),
        ("audio_ingest", "0.0.0.0", 5723),
        ("lyric_editor", "0.0.0.0", 5724),
    ];

    for (module_name, host, port) in defaults {
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO module_config (module_name, host, port, enabled)
            VALUES (?, ?, ?, 1)
            "#,
        )
        .bind(module_name)
        .bind(host)
        .bind(port)
        .execute(pool)
        .await?;
    }

    Ok(())
}

/// Create the files table
///
/// Stores audio file metadata including duration in ticks (SPEC017).
///
/// **[REQ-F-003]** File duration uses tick-based representation for consistency.
pub async fn create_files_table(pool: &SqlitePool) -> Result<()> {
    // REQ-F-003: File duration migration to ticks (BREAKING CHANGE)
    // Changed from `duration REAL` (f64 seconds) to `duration_ticks INTEGER` (i64 ticks)
    // Per SPEC017: All passage timing uses tick-based representation for consistency
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS files (
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
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            CHECK (duration_ticks IS NULL OR duration_ticks > 0),
            CHECK (sample_rate IS NULL OR sample_rate > 0),
            CHECK (channels IS NULL OR (channels > 0 AND channels <= 32)),
            CHECK (file_size_bytes IS NULL OR file_size_bytes >= 0)
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create indexes
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_files_path ON files(path)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_files_hash ON files(hash)")
        .execute(pool)
        .await?;

    Ok(())
}

/// Create the passages table
///
/// Stores passage metadata including timing, crossfade points, and musical flavor.
/// All timing values use tick-based representation (SPEC017).
pub async fn create_passages_table(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS passages (
            guid TEXT PRIMARY KEY,
            file_id TEXT NOT NULL REFERENCES files(guid) ON DELETE CASCADE,
            start_time_ticks INTEGER NOT NULL DEFAULT 0,
            fade_in_start_ticks INTEGER,
            lead_in_start_ticks INTEGER,
            lead_out_start_ticks INTEGER,
            fade_out_start_ticks INTEGER,
            end_time_ticks INTEGER NOT NULL,
            fade_in_curve TEXT CHECK (fade_in_curve IS NULL OR fade_in_curve IN ('exponential', 'cosine', 'linear', 'logarithmic', 'equal_power')),
            fade_out_curve TEXT CHECK (fade_out_curve IS NULL OR fade_out_curve IN ('exponential', 'cosine', 'linear', 'logarithmic', 'equal_power')),
            title TEXT,
            user_title TEXT,
            artist TEXT,
            album TEXT,
            recording_mbid TEXT,
            musical_flavor_vector TEXT,
            import_metadata TEXT,
            additional_metadata TEXT,
            decode_status TEXT DEFAULT 'pending' CHECK (decode_status IN ('pending', 'successful', 'unsupported_codec', 'failed')),
            flavor_source_blend TEXT,
            flavor_confidence_map TEXT,
            flavor_completeness REAL,
            title_source TEXT,
            title_confidence REAL,
            artist_source TEXT,
            artist_confidence REAL,
            album_source TEXT,
            album_confidence REAL,
            mbid_source TEXT,
            mbid_confidence REAL,
            identity_confidence REAL,
            identity_posterior_probability REAL,
            identity_conflicts TEXT,
            overall_quality_score REAL,
            metadata_completeness REAL,
            validation_status TEXT,
            validation_report TEXT,
            validation_issues TEXT,
            import_session_id TEXT,
            import_timestamp TIMESTAMP,
            import_strategy TEXT,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            CHECK (start_time_ticks >= 0),
            CHECK (end_time_ticks > start_time_ticks),
            CHECK (fade_in_start_ticks IS NULL OR (fade_in_start_ticks >= start_time_ticks AND fade_in_start_ticks <= end_time_ticks)),
            CHECK (lead_in_start_ticks IS NULL OR (lead_in_start_ticks >= start_time_ticks AND lead_in_start_ticks <= end_time_ticks)),
            CHECK (lead_out_start_ticks IS NULL OR (lead_out_start_ticks >= start_time_ticks AND lead_out_start_ticks <= end_time_ticks)),
            CHECK (fade_out_start_ticks IS NULL OR (fade_out_start_ticks >= start_time_ticks AND fade_out_start_ticks <= end_time_ticks)),
            CHECK (fade_in_start_ticks IS NULL OR fade_out_start_ticks IS NULL OR fade_in_start_ticks <= fade_out_start_ticks),
            CHECK (lead_in_start_ticks IS NULL OR lead_out_start_ticks IS NULL OR lead_in_start_ticks <= lead_out_start_ticks)
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create indexes per IMPL001
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_passages_file_id ON passages(file_id)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_passages_title ON passages(title)")
        .execute(pool)
        .await?;

    // Create index for decode_status queries (REQ-AP-ERR-011)
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_passages_decode_status ON passages(decode_status)")
        .execute(pool)
        .await?;

    Ok(())
}

/// Create the import_provenance table
///
/// Tracks the origin and confidence of data extracted during passage import.
/// Used for debugging import quality and providing audit trails.
pub async fn create_import_provenance_table(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS import_provenance (
            id TEXT PRIMARY KEY,
            passage_id TEXT NOT NULL REFERENCES passages(guid) ON DELETE CASCADE,
            source_type TEXT NOT NULL,
            data_extracted TEXT,
            confidence REAL,
            timestamp INTEGER,
            FOREIGN KEY (passage_id) REFERENCES passages(guid) ON DELETE CASCADE
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create index for querying all extractions for a passage
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_import_provenance_passage_id ON import_provenance(passage_id)")
        .execute(pool)
        .await?;

    Ok(())
}

async fn create_queue_table(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS queue (
            guid TEXT PRIMARY KEY,
            file_path TEXT NOT NULL,
            passage_guid TEXT,
            play_order INTEGER NOT NULL,
            start_time_ms INTEGER,
            end_time_ms INTEGER,
            lead_in_point_ms INTEGER,
            lead_out_point_ms INTEGER,
            fade_in_point_ms INTEGER,
            fade_out_point_ms INTEGER,
            fade_in_curve TEXT CHECK (fade_in_curve IS NULL OR fade_in_curve IN ('linear', 'exponential', 'logarithmic', 'cosine', 'equal_power')),
            fade_out_curve TEXT CHECK (fade_out_curve IS NULL OR fade_out_curve IN ('linear', 'exponential', 'logarithmic', 'cosine', 'equal_power')),
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create indexes
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_queue_order ON queue(play_order)")
        .execute(pool)
        .await?;

    Ok(())
}

async fn create_acoustid_cache_table(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS acoustid_cache (
            fingerprint_hash TEXT PRIMARY KEY,
            mbid TEXT NOT NULL,
            cached_at TEXT NOT NULL DEFAULT (datetime('now')),
            CHECK (length(fingerprint_hash) = 64)
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create index for cache expiration queries (future feature)
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_acoustid_cache_cached_at ON acoustid_cache(cached_at)")
        .execute(pool)
        .await?;

    Ok(())
}

async fn create_songs_table(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS songs (
            guid TEXT PRIMARY KEY,
            recording_mbid TEXT NOT NULL UNIQUE,
            work_id TEXT,
            related_songs TEXT,
            lyrics TEXT,
            base_probability REAL NOT NULL DEFAULT 1.0,
            min_cooldown INTEGER NOT NULL DEFAULT 604800,
            ramping_cooldown INTEGER NOT NULL DEFAULT 1209600,
            last_played_at TIMESTAMP,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            CHECK (base_probability >= 0.0 AND base_probability <= 1000.0),
            CHECK (min_cooldown >= 0),
            CHECK (ramping_cooldown >= 0)
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_songs_recording_mbid ON songs(recording_mbid)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_songs_last_played ON songs(last_played_at)")
        .execute(pool)
        .await?;

    Ok(())
}

/// Create the artists table
///
/// Stores artist metadata including cooldown periods and selection probabilities.
pub async fn create_artists_table(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS artists (
            guid TEXT PRIMARY KEY,
            artist_mbid TEXT NOT NULL UNIQUE,
            name TEXT NOT NULL,
            base_probability REAL NOT NULL DEFAULT 1.0,
            min_cooldown INTEGER NOT NULL DEFAULT 7200,
            ramping_cooldown INTEGER NOT NULL DEFAULT 14400,
            last_played_at TIMESTAMP,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            CHECK (base_probability >= 0.0 AND base_probability <= 1000.0),
            CHECK (min_cooldown >= 0),
            CHECK (ramping_cooldown >= 0)
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_artists_mbid ON artists(artist_mbid)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_artists_last_played ON artists(last_played_at)")
        .execute(pool)
        .await?;

    Ok(())
}

/// Create the works table
///
/// Stores musical work metadata including cooldown periods for automatic selection.
pub async fn create_works_table(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS works (
            guid TEXT PRIMARY KEY,
            work_mbid TEXT NOT NULL UNIQUE,
            title TEXT NOT NULL,
            base_probability REAL NOT NULL DEFAULT 1.0,
            min_cooldown INTEGER NOT NULL DEFAULT 259200,
            ramping_cooldown INTEGER NOT NULL DEFAULT 604800,
            last_played_at TIMESTAMP,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            CHECK (base_probability >= 0.0 AND base_probability <= 1000.0),
            CHECK (min_cooldown >= 0),
            CHECK (ramping_cooldown >= 0)
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_works_mbid ON works(work_mbid)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_works_last_played ON works(last_played_at)")
        .execute(pool)
        .await?;

    Ok(())
}

/// Create the albums table
///
/// Stores album metadata from MusicBrainz.
pub async fn create_albums_table(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS albums (
            guid TEXT PRIMARY KEY,
            album_mbid TEXT NOT NULL UNIQUE,
            title TEXT NOT NULL,
            release_date TEXT,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_albums_mbid ON albums(album_mbid)")
        .execute(pool)
        .await?;

    Ok(())
}

async fn create_images_table(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS images (
            guid TEXT PRIMARY KEY,
            file_path TEXT NOT NULL,
            image_type TEXT NOT NULL,
            entity_id TEXT NOT NULL,
            priority INTEGER NOT NULL DEFAULT 100,
            width INTEGER,
            height INTEGER,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            CHECK (image_type IN ('album_front', 'album_back', 'album_liner', 'song', 'passage', 'artist', 'work', 'logo')),
            CHECK (priority >= 0)
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_images_entity ON images(entity_id, image_type, priority)",
    )
    .execute(pool)
    .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_images_type ON images(image_type)")
        .execute(pool)
        .await?;

    Ok(())
}

async fn create_passage_songs_table(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS passage_songs (
            passage_id TEXT NOT NULL REFERENCES passages(guid) ON DELETE CASCADE,
            song_id TEXT NOT NULL REFERENCES songs(guid) ON DELETE CASCADE,
            start_time_ticks INTEGER NOT NULL,
            end_time_ticks INTEGER NOT NULL,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY (passage_id, song_id),
            CHECK (start_time_ticks >= 0),
            CHECK (end_time_ticks > start_time_ticks)
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_passage_songs_passage ON passage_songs(passage_id)",
    )
    .execute(pool)
    .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_passage_songs_song ON passage_songs(song_id)")
        .execute(pool)
        .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_passage_songs_timing ON passage_songs(passage_id, start_time_ticks)",
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn create_song_artists_table(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS song_artists (
            song_id TEXT NOT NULL REFERENCES songs(guid) ON DELETE CASCADE,
            artist_id TEXT NOT NULL REFERENCES artists(guid) ON DELETE CASCADE,
            weight REAL NOT NULL,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY (song_id, artist_id),
            CHECK (weight > 0.0 AND weight <= 1.0)
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_song_artists_song ON song_artists(song_id)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_song_artists_artist ON song_artists(artist_id)")
        .execute(pool)
        .await?;

    Ok(())
}

async fn create_passage_albums_table(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS passage_albums (
            passage_id TEXT NOT NULL REFERENCES passages(guid) ON DELETE CASCADE,
            album_id TEXT NOT NULL REFERENCES albums(guid) ON DELETE CASCADE,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY (passage_id, album_id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_passage_albums_passage ON passage_albums(passage_id)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_passage_albums_album ON passage_albums(album_id)")
        .execute(pool)
        .await?;

    Ok(())
}

async fn create_import_sessions_table(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS import_sessions (
            session_id TEXT PRIMARY KEY,
            state TEXT NOT NULL,
            root_folder TEXT NOT NULL,
            parameters TEXT NOT NULL,
            progress_current INTEGER NOT NULL,
            progress_total INTEGER NOT NULL,
            progress_percentage REAL NOT NULL,
            current_operation TEXT NOT NULL,
            errors TEXT NOT NULL,
            started_at TEXT NOT NULL,
            ended_at TEXT
        )
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn create_temp_file_songs_table(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS temp_file_songs (
            file_id TEXT PRIMARY KEY REFERENCES files(guid) ON DELETE CASCADE,
            song_id TEXT NOT NULL REFERENCES songs(guid) ON DELETE CASCADE,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

async fn create_temp_file_albums_table(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS temp_file_albums (
            file_id TEXT NOT NULL REFERENCES files(guid) ON DELETE CASCADE,
            album_id TEXT NOT NULL REFERENCES albums(guid) ON DELETE CASCADE,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            PRIMARY KEY (file_id, album_id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}
