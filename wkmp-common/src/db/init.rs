//! Database initialization
//!
//! Implements graceful degradation for database initialization as specified in:
//! - [REQ-NF-036]: Automatic database creation with default schema
//! - [ARCH-INIT-010]: Module startup sequence
//! - [ARCH-INIT-020]: Default value initialization behavior
//! - [DEP-DB-011]: Database initialization on first run

use crate::Result;
use sqlx::SqlitePool;
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
    let db_url = format!("sqlite://{}?mode=rwc", db_path.display());
    let pool = SqlitePool::connect(&db_url).await?;

    if newly_created {
        info!("Initialized new database: {}", db_path.display());
    } else {
        info!("Opened existing database: {}", db_path.display());
    }

    // Enable foreign keys
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await?;

    // Set busy timeout to 5 seconds [ARCH-ERRH-070]
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
    create_queue_table(&pool).await?;
    create_acoustid_cache_table(&pool).await?;

    // Initialize default settings [ARCH-INIT-020]
    init_default_settings(&pool).await?;

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

async fn create_settings_table(pool: &SqlitePool) -> Result<()> {
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

    // Validation service settings **[ARCH-AUTO-VAL-001]**
    ensure_setting(pool, "validation_enabled", "true").await?;
    ensure_setting(pool, "validation_interval_secs", "10").await?;
    ensure_setting(pool, "validation_tolerance_samples", "8192").await?;

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
        sqlx::query(
            "INSERT INTO settings (key, value) VALUES (?, ?)"
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

async fn create_files_table(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS files (
            guid TEXT PRIMARY KEY,
            path TEXT NOT NULL UNIQUE,
            hash TEXT NOT NULL,
            duration REAL,
            modification_time TIMESTAMP NOT NULL,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            CHECK (duration IS NULL OR duration > 0)
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

async fn create_passages_table(pool: &SqlitePool) -> Result<()> {
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
            musical_flavor_vector TEXT,
            decode_status TEXT DEFAULT 'pending' CHECK (decode_status IN ('pending', 'successful', 'unsupported_codec', 'failed')),
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
