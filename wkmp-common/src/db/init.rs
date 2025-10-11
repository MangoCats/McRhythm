//! Database initialization

use crate::Result;
use sqlx::SqlitePool;
use std::path::Path;

/// Initialize database connection and create tables if needed
pub async fn init_database(db_path: &Path) -> Result<SqlitePool> {
    // Create parent directory if it doesn't exist
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Use sqlite options to create database if it doesn't exist
    let db_url = format!("sqlite://{}?mode=rwc", db_path.display());
    let pool = SqlitePool::connect(&db_url).await?;

    // Run migrations
    create_schema_version_table(&pool).await?;
    create_users_table(&pool).await?;
    create_settings_table(&pool).await?;
    create_module_config_table(&pool).await?;

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
            value TEXT NOT NULL,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Initialize default settings
    let defaults = vec![
        ("initial_play_state", "playing"),
        ("volume_level", "0.75"),
        ("global_crossfade_time", "2.0"),
        ("volume_fade_update_period", "10"),
        ("playback_progress_interval_ms", "5000"),
    ];

    for (key, value) in defaults {
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO settings (key, value)
            VALUES (?, ?)
            "#,
        )
        .bind(key)
        .bind(value)
        .execute(pool)
        .await?;
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
