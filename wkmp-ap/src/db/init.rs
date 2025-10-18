///! Database initialization functions
///!
///! [ARCH-INIT-010] Module startup sequence
///! [ARCH-INIT-020] Default value initialization behavior
///! [ISSUE-3] Initialize required tables with defaults

use crate::error::Result;
use sqlx::{Pool, Sqlite};
use tracing::{info, warn};

/// Initialize settings table with default values
///
/// [ARCH-INIT-020] Default settings initialization
/// [ISSUE-3] Initialize settings if missing/NULL
pub async fn init_settings_defaults(pool: &Pool<Sqlite>) -> Result<()> {
    info!("Initializing default settings");

    // Settings with their default values
    let defaults = vec![
        // Queue refill thresholds
        ("queue_refill_threshold_passages", "2"),
        ("queue_refill_threshold_seconds", "900"), // 15 minutes
        ("queue_refill_request_throttle_seconds", "10"),
        ("queue_refill_acknowledgment_timeout_seconds", "5"),

        // Relaunch configuration
        ("relaunch_delay", "5"), // 5 seconds between relaunch attempts
        ("relaunch_attempts", "20"), // Maximum 20 attempts

        // Default volume (0.0 - 1.0)
        ("volume_level", "0.5"),

        // Default audio device
        ("audio_sink", "default"),

        // Crossfade defaults
        ("global_crossfade_time", "2.0"), // 2 seconds
        ("global_fade_curve", "exponential_logarithmic"),
    ];

    for (key, default_value) in defaults {
        // Check if setting exists
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM settings WHERE key = ?)"
        )
        .bind(key)
        .fetch_one(pool)
        .await?;

        if !exists {
            // Insert default value
            sqlx::query(
                "INSERT INTO settings (key, value) VALUES (?, ?)"
            )
            .bind(key)
            .bind(default_value)
            .execute(pool)
            .await?;

            info!("Initialized setting '{}' with default value: {}", key, default_value);
        }
    }

    Ok(())
}

/// Initialize module_config table with default configuration
///
/// [ARCH-INIT-010] Module configuration table
/// [ISSUE-3] Module config initialization
pub async fn init_module_config(pool: &Pool<Sqlite>) -> Result<()> {
    info!("Checking module_config table");

    // Check if module_config table exists
    let table_exists: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM sqlite_master
            WHERE type='table' AND name='module_config'
        )
        "#
    )
    .fetch_one(pool)
    .await?;

    if !table_exists {
        warn!("module_config table does not exist - creating with defaults");

        // Create module_config table
        sqlx::query(
            r#"
            CREATE TABLE module_config (
                module_name TEXT PRIMARY KEY,
                host TEXT NOT NULL,
                port INTEGER NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#
        )
        .execute(pool)
        .await?;

        // Insert default module configurations
        let modules = vec![
            ("audio_player", "127.0.0.1", 5721),
            ("user_interface", "127.0.0.1", 5720),
            ("program_director", "127.0.0.1", 5722),
            ("audio_ingest", "127.0.0.1", 5723),
            ("lyric_editor", "127.0.0.1", 5724),
        ];

        for (module, host, port) in modules {
            sqlx::query(
                "INSERT INTO module_config (module_name, host, port) VALUES (?, ?, ?)"
            )
            .bind(module)
            .bind(host)
            .bind(port)
            .execute(pool)
            .await?;
        }

        info!("Created module_config table with default configurations");
    }

    // Verify audio_player config exists
    let ap_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM module_config WHERE module_name = 'audio_player')"
    )
    .fetch_one(pool)
    .await?;

    if !ap_exists {
        warn!("audio_player config missing - inserting default");
        sqlx::query(
            "INSERT INTO module_config (module_name, host, port) VALUES ('audio_player', '127.0.0.1', 5721)"
        )
        .execute(pool)
        .await?;
    }

    Ok(())
}

/// Initialize all required database structures
///
/// [ARCH-INIT-010] Complete module initialization sequence
/// [ISSUE-3] Full database initialization
pub async fn initialize_database(pool: &Pool<Sqlite>) -> Result<()> {
    info!("Initializing database structures");

    // Initialize module config
    init_module_config(pool).await?;

    // Initialize settings defaults
    init_settings_defaults(pool).await?;

    info!("Database initialization complete");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_test_db() -> Pool<Sqlite> {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();

        // Create settings table
        sqlx::query(
            r#"
            CREATE TABLE settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    #[tokio::test]
    async fn test_init_settings_defaults() {
        let pool = setup_test_db().await;

        // Initialize defaults
        init_settings_defaults(&pool).await.unwrap();

        // Verify volume_level was set
        let volume: String = sqlx::query_scalar(
            "SELECT value FROM settings WHERE key = 'volume_level'"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(volume, "0.5");

        // Verify queue settings
        let threshold: String = sqlx::query_scalar(
            "SELECT value FROM settings WHERE key = 'queue_refill_threshold_passages'"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(threshold, "2");
    }

    #[tokio::test]
    async fn test_init_settings_idempotent() {
        let pool = setup_test_db().await;

        // Initialize defaults twice
        init_settings_defaults(&pool).await.unwrap();
        init_settings_defaults(&pool).await.unwrap();

        // Should still have correct values (not duplicated)
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM settings WHERE key = 'volume_level'"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, 1);
    }

    #[tokio::test]
    async fn test_init_module_config() {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();

        // Initialize module config (creates table)
        init_module_config(&pool).await.unwrap();

        // Verify audio_player config exists
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM module_config WHERE module_name = 'audio_player')"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(exists);

        // Verify port is correct
        let port: i32 = sqlx::query_scalar(
            "SELECT port FROM module_config WHERE module_name = 'audio_player'"
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(port, 5721);
    }
}
