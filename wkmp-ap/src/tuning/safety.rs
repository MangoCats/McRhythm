//! Settings backup and restore for safety
//!
//! **Purpose:** Preserve user settings before tuning and restore on abort/failure.
//!
//! **Traceability:**
//! - TUNE-SAFE-010: Settings backup and restore
//! - TUNE-OUT-020: Database update with backup

use crate::db::settings::{load_clamped_setting, set_setting};
use crate::error::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};
use std::path::PathBuf;

/// Backup of user settings before tuning
///
/// Captures mixer_check_interval_ms and audio_buffer_size values
/// for restoration if tuning is aborted or fails.
///
/// **Traceability:** TUNE-SAFE-010
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsBackup {
    /// Mixer check interval in milliseconds
    pub mixer_check_interval_ms: u64,

    /// Audio buffer size in frames
    pub audio_buffer_size: u32,

    /// When backup was created
    pub timestamp: DateTime<Utc>,
}

impl SettingsBackup {
    /// Get path to temporary backup file
    ///
    /// Uses system temp directory + predictable filename.
    ///
    /// # Returns
    /// PathBuf to backup file location
    fn backup_file_path() -> PathBuf {
        std::env::temp_dir().join("wkmp_tuning_backup.json")
    }
}

/// Backup current settings to memory and temp file
///
/// Reads both tuning-related parameters from database and stores them
/// in a SettingsBackup struct. Also writes to temp file as safety measure.
///
/// **Parameters Backed Up:**
/// - mixer_check_interval_ms (DBD-PARAM-111): Range 1-100ms, default 5ms
/// - audio_buffer_size (DBD-PARAM-110): Range 64-8192 frames, default 512
///
/// **Traceability:** TUNE-SAFE-010
///
/// # Arguments
/// - `db`: Database connection pool
///
/// # Returns
/// SettingsBackup with current parameter values
///
/// # Errors
/// Returns error if:
/// - Database read fails
/// - Temp file write fails (non-fatal - continues with in-memory backup)
pub async fn backup_settings(db: &Pool<Sqlite>) -> Result<SettingsBackup> {
    // Read current values from database (or use defaults)
    let mixer_check_interval_ms = load_clamped_setting(
        db,
        "mixer_check_interval_ms",
        1u64,
        100u64,
        5u64,
    )
    .await?;

    let audio_buffer_size = load_clamped_setting(
        db,
        "audio_buffer_size",
        64u32,
        8192u32,
        512u32,
    )
    .await?;

    let backup = SettingsBackup {
        mixer_check_interval_ms,
        audio_buffer_size,
        timestamp: Utc::now(),
    };

    // Write to temp file as safety measure (best effort - don't fail if this fails)
    if let Err(e) = write_backup_file(&backup) {
        tracing::warn!("Failed to write backup file (will use in-memory backup): {}", e);
    }

    Ok(backup)
}

/// Restore settings from backup
///
/// Writes backed-up parameter values back to database, overwriting any
/// values that may have been modified during tuning.
///
/// **Traceability:** TUNE-SAFE-010
///
/// # Arguments
/// - `db`: Database connection pool
/// - `backup`: SettingsBackup to restore from
///
/// # Returns
/// Ok(()) on success
///
/// # Errors
/// Returns error if database write fails
pub async fn restore_settings(db: &Pool<Sqlite>, backup: &SettingsBackup) -> Result<()> {
    // Restore both parameters
    set_setting(db, "mixer_check_interval_ms", backup.mixer_check_interval_ms).await?;
    set_setting(db, "audio_buffer_size", backup.audio_buffer_size).await?;

    // Clean up temp file
    cleanup_backup_file();

    Ok(())
}

/// Write backup to temporary file
///
/// Creates JSON file in system temp directory for recovery in case of
/// process crash or panic.
///
/// # Arguments
/// - `backup`: SettingsBackup to serialize
///
/// # Returns
/// Ok(()) on success
///
/// # Errors
/// Returns error if:
/// - JSON serialization fails
/// - File write fails (permissions, disk space, etc.)
fn write_backup_file(backup: &SettingsBackup) -> Result<()> {
    let path = SettingsBackup::backup_file_path();
    let json = serde_json::to_string_pretty(backup)
        .map_err(|e| crate::error::Error::Internal(format!("Failed to serialize backup: {}", e)))?;
    std::fs::write(path, json)?;
    Ok(())
}

/// Clean up temporary backup file
///
/// Removes backup file from temp directory. Called after successful
/// completion or restoration.
///
/// Does not return error - cleanup is best-effort.
fn cleanup_backup_file() {
    let path = SettingsBackup::backup_file_path();
    if path.exists() {
        if let Err(e) = std::fs::remove_file(&path) {
            tracing::warn!("Failed to clean up backup file at {:?}: {}", path, e);
        }
    }
}

/// Load backup from temporary file
///
/// Attempts to recover backup from temp file in case of process crash.
/// Used for manual recovery or panic handlers.
///
/// # Returns
/// Some(SettingsBackup) if file exists and is valid, None otherwise
pub fn load_backup_from_file() -> Option<SettingsBackup> {
    let path = SettingsBackup::backup_file_path();
    if !path.exists() {
        return None;
    }

    match std::fs::read_to_string(&path) {
        Ok(json) => match serde_json::from_str::<SettingsBackup>(&json) {
            Ok(backup) => Some(backup),
            Err(e) => {
                tracing::error!("Failed to parse backup file: {}", e);
                None
            }
        },
        Err(e) => {
            tracing::error!("Failed to read backup file: {}", e);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    /// Create in-memory test database
    async fn setup_test_db() -> Pool<Sqlite> {
        let db = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .expect("Failed to create test database");

        // Create settings table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )
            "#,
        )
        .execute(&db)
        .await
        .expect("Failed to create settings table");

        db
    }

    #[tokio::test]
    async fn test_backup_settings() {
        let db = setup_test_db().await;

        // Set initial values
        set_setting(&db, "mixer_check_interval_ms", 10u64)
            .await
            .unwrap();
        set_setting(&db, "audio_buffer_size", 1024u32)
            .await
            .unwrap();

        // Backup
        let backup = backup_settings(&db).await.unwrap();

        assert_eq!(backup.mixer_check_interval_ms, 10);
        assert_eq!(backup.audio_buffer_size, 1024);
        assert!(backup.timestamp <= Utc::now());
    }

    #[tokio::test]
    async fn test_backup_with_defaults() {
        let db = setup_test_db().await;

        // No settings in database - should use defaults
        let backup = backup_settings(&db).await.unwrap();

        assert_eq!(backup.mixer_check_interval_ms, 5); // Default
        assert_eq!(backup.audio_buffer_size, 512); // Default
    }

    #[tokio::test]
    async fn test_restore_settings() {
        let db = setup_test_db().await;

        // Set initial values
        set_setting(&db, "mixer_check_interval_ms", 5u64)
            .await
            .unwrap();
        set_setting(&db, "audio_buffer_size", 512u32)
            .await
            .unwrap();

        // Backup
        let backup = backup_settings(&db).await.unwrap();

        // Modify settings (simulate tuning)
        set_setting(&db, "mixer_check_interval_ms", 20u64)
            .await
            .unwrap();
        set_setting(&db, "audio_buffer_size", 2048u32)
            .await
            .unwrap();

        // Verify modified
        let modified_interval: u64 = load_clamped_setting(&db, "mixer_check_interval_ms", 1, 100, 5)
            .await
            .unwrap();
        let modified_buffer: u32 = load_clamped_setting(&db, "audio_buffer_size", 64, 8192, 512)
            .await
            .unwrap();
        assert_eq!(modified_interval, 20);
        assert_eq!(modified_buffer, 2048);

        // Restore
        restore_settings(&db, &backup).await.unwrap();

        // Verify restoration
        let restored_interval: u64 = load_clamped_setting(&db, "mixer_check_interval_ms", 1, 100, 5)
            .await
            .unwrap();
        let restored_buffer: u32 = load_clamped_setting(&db, "audio_buffer_size", 64, 8192, 512)
            .await
            .unwrap();

        assert_eq!(restored_interval, 5);
        assert_eq!(restored_buffer, 512);
    }

    /// Test backup file write/read operations
    ///
    /// **Note:** Uses `#[serial]` to prevent test isolation issues.
    /// Both this test and `test_load_nonexistent_backup` access the same
    /// backup file in the system temp directory.
    #[test]
    #[serial_test::serial]
    fn test_backup_file_operations() {
        // Ensure clean state - remove any stale backup file from previous test runs
        cleanup_backup_file();

        // Small delay to ensure OS completes file deletion
        std::thread::sleep(std::time::Duration::from_millis(10));

        let backup = SettingsBackup {
            mixer_check_interval_ms: 10,
            audio_buffer_size: 1024,
            timestamp: Utc::now(),
        };

        // Write backup file
        write_backup_file(&backup).unwrap();

        // Verify file exists
        let path = SettingsBackup::backup_file_path();
        assert!(path.exists());

        // Load from file
        let loaded = load_backup_from_file().expect("Should load backup");
        assert_eq!(loaded.mixer_check_interval_ms, 10);
        assert_eq!(loaded.audio_buffer_size, 1024);

        // Cleanup
        cleanup_backup_file();
        assert!(!path.exists());
    }

    /// Test loading backup when file doesn't exist
    ///
    /// **Note:** Uses `#[serial]` to prevent race condition with
    /// `test_backup_file_operations` accessing the same temp file.
    #[test]
    #[serial_test::serial]
    fn test_load_nonexistent_backup() {
        // Ensure no backup file exists
        cleanup_backup_file();

        let result = load_backup_from_file();
        assert!(result.is_none());
    }
}
