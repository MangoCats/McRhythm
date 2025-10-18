//! Settings database access
//!
//! Read/write settings from the settings table (key-value store).
//! All settings are global/system-wide (not user-specific).
//!
//! **Traceability:**
//! - DB-SETTINGS-010 (Settings table schema)
//! - ARCH-CFG-020 (Database-first configuration)

use crate::error::{Error, Result};
use sqlx::{Pool, Sqlite};
use std::str::FromStr;
use uuid::Uuid;

/// Get volume setting (0.0-1.0)
///
/// **Traceability:** DB-SETTINGS-020
pub async fn get_volume(db: &Pool<Sqlite>) -> Result<f32> {
    match get_setting::<f32>(db, "volume_level").await? {
        Some(vol) => Ok(vol.clamp(0.0, 1.0)),
        None => {
            // Default volume is 0.5 (50%)
            set_volume(db, 0.5).await?;
            Ok(0.5)
        }
    }
}

/// Set volume setting (0.0-1.0)
///
/// **Traceability:** DB-SETTINGS-030
pub async fn set_volume(db: &Pool<Sqlite>, volume: f32) -> Result<()> {
    let clamped = volume.clamp(0.0, 1.0);
    set_setting(db, "volume_level", clamped).await
}

/// Get audio device/sink identifier
///
/// **Traceability:** DB-SETTINGS-040
pub async fn get_audio_device(db: &Pool<Sqlite>) -> Result<String> {
    match get_setting::<String>(db, "audio_sink").await? {
        Some(device) => Ok(device),
        None => {
            // Default to "default" device
            let default = "default".to_string();
            set_audio_device(db, default.clone()).await?;
            Ok(default)
        }
    }
}

/// Set audio device/sink identifier
///
/// **Traceability:** DB-SETTINGS-050
pub async fn set_audio_device(db: &Pool<Sqlite>, device: String) -> Result<()> {
    set_setting(db, "audio_sink", device).await
}

/// Crossfade default configuration
///
/// **Traceability:** XFD-IMPL-030
#[derive(Debug, Clone)]
pub struct CrossfadeDefaults {
    /// Global crossfade time in seconds
    pub crossfade_time_s: f64,
    /// Global fade curve (combined fade-in/fade-out)
    pub fade_curve: String,
}

/// Get crossfade default settings
///
/// **Traceability:** DB-SETTINGS-060
pub async fn get_crossfade_defaults(db: &Pool<Sqlite>) -> Result<CrossfadeDefaults> {
    let crossfade_time_s = match get_setting::<f64>(db, "global_crossfade_time").await? {
        Some(time) => time,
        None => {
            // Default crossfade time is 2.0 seconds
            set_setting(db, "global_crossfade_time", 2.0).await?;
            2.0
        }
    };

    let fade_curve = match get_setting::<String>(db, "global_fade_curve").await? {
        Some(curve) => curve,
        None => {
            // Default curve is exponential_logarithmic
            let default = "exponential_logarithmic".to_string();
            set_setting(db, "global_fade_curve", default.clone()).await?;
            default
        }
    };

    Ok(CrossfadeDefaults {
        crossfade_time_s,
        fade_curve,
    })
}

/// Save playback position to database
///
/// [REQ-PERS-011] Persist last played position
/// [ARCH-QP-020] Position persisted on Pause/Play
/// [ISSUE-2] Database persistence for playback state
pub async fn save_playback_position(db: &Pool<Sqlite>, position_ms: i64) -> Result<()> {
    set_setting(db, "last_played_position_ms", position_ms).await
}

/// Load last playback position from database
///
/// [REQ-PERS-011] Restore last played position
pub async fn load_playback_position(db: &Pool<Sqlite>) -> Result<Option<i64>> {
    get_setting::<i64>(db, "last_played_position_ms").await
}

/// Save last played passage/queue entry ID
///
/// [REQ-PERS-011] Persist last played passage
pub async fn save_last_passage_id(db: &Pool<Sqlite>, passage_id: Uuid) -> Result<()> {
    set_setting(db, "last_played_passage_id", passage_id.to_string()).await
}

/// Load last played passage/queue entry ID
///
/// [REQ-PERS-011] Restore last played passage
pub async fn load_last_passage_id(db: &Pool<Sqlite>) -> Result<Option<Uuid>> {
    match get_setting::<String>(db, "last_played_passage_id").await? {
        Some(id_str) => {
            Uuid::parse_str(&id_str)
                .map(Some)
                .map_err(|e| Error::Config(format!("Invalid UUID in last_played_passage_id: {}", e)))
        }
        None => Ok(None),
    }
}

/// Save queue state (current passage being played)
///
/// [ARCH-QP-020] Queue state persistence
pub async fn save_queue_state(db: &Pool<Sqlite>, current_id: Option<Uuid>) -> Result<()> {
    match current_id {
        Some(id) => set_setting(db, "queue_current_id", id.to_string()).await,
        None => {
            // Delete the setting if no current passage
            sqlx::query("DELETE FROM settings WHERE key = 'queue_current_id'")
                .execute(db)
                .await?;
            Ok(())
        }
    }
}

/// Load queue state
///
/// [ARCH-QP-020] Queue state restoration
pub async fn load_queue_state(db: &Pool<Sqlite>) -> Result<Option<Uuid>> {
    match get_setting::<String>(db, "queue_current_id").await? {
        Some(id_str) => {
            Uuid::parse_str(&id_str)
                .map(Some)
                .map_err(|e| Error::Config(format!("Invalid UUID in queue_current_id: {}", e)))
        }
        None => Ok(None),
    }
}

/// Load position_event_interval_ms from settings table
///
/// **[REV002]** Event-driven position tracking
/// **[ADDENDUM-interval_configurability]** Configurable position event interval
///
/// # Returns
/// Interval in milliseconds (default: 1000ms if not set)
pub async fn load_position_event_interval(db: &Pool<Sqlite>) -> Result<u32> {
    match get_setting::<u32>(db, "position_event_interval_ms").await? {
        Some(interval) => {
            // Clamp to valid range: 100-5000ms
            Ok(interval.clamp(100, 5000))
        }
        None => {
            // Default: 1000ms (1 second)
            Ok(1000)
        }
    }
}

/// Load playback_progress_interval_ms from settings table
///
/// **[REV002]** Event-driven position tracking
/// **[ADDENDUM-interval_configurability]** Configurable playback progress interval
///
/// # Returns
/// Interval in milliseconds (default: 5000ms if not set)
pub async fn load_progress_interval(db: &Pool<Sqlite>) -> Result<u64> {
    match get_setting::<u64>(db, "playback_progress_interval_ms").await? {
        Some(interval) => {
            // Clamp to valid range: 1000-60000ms (1-60 seconds)
            Ok(interval.clamp(1000, 60000))
        }
        None => {
            // Default: 5000ms (5 seconds)
            Ok(5000)
        }
    }
}

/// Load audio_ring_buffer_grace_period_ms from settings table
///
/// **[SSD-RBUF-014]** Ring buffer startup grace period
///
/// # Returns
/// Grace period in milliseconds (default: 2000ms if not set)
pub async fn load_ring_buffer_grace_period(db: &Pool<Sqlite>) -> Result<u64> {
    match get_setting::<u64>(db, "audio_ring_buffer_grace_period_ms").await? {
        Some(grace_ms) => {
            // Clamp to valid range: 0-10000ms (0-10 seconds)
            // 0 = no grace period, 10s = maximum
            Ok(grace_ms.clamp(0, 10000))
        }
        None => {
            // Default: 2000ms (2 seconds)
            Ok(2000)
        }
    }
}

/// Generic setting getter
///
/// Returns None if key doesn't exist in database.
/// Parses value from string using FromStr trait.
///
/// **Traceability:** DB-SETTINGS-070
pub async fn get_setting<T: FromStr>(db: &Pool<Sqlite>, key: &str) -> Result<Option<T>> {
    let value: Option<String> = sqlx::query_scalar("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(db)
        .await?;

    match value {
        Some(s) => match s.parse::<T>() {
            Ok(parsed) => Ok(Some(parsed)),
            Err(_) => Err(Error::Config(format!(
                "Failed to parse setting '{}' value: {}",
                key, s
            ))),
        },
        None => Ok(None),
    }
}

/// Generic setting setter
///
/// Inserts or updates setting in database.
///
/// **Traceability:** DB-SETTINGS-080
pub async fn set_setting<T: ToString>(db: &Pool<Sqlite>, key: &str, value: T) -> Result<()> {
    let value_str = value.to_string();

    sqlx::query(
        r#"
        INSERT INTO settings (key, value)
        VALUES (?, ?)
        ON CONFLICT(key) DO UPDATE SET value = excluded.value
        "#,
    )
    .bind(key)
    .bind(value_str)
    .execute(db)
    .await?;

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
    async fn test_volume_get_set() {
        let db = setup_test_db().await;

        // Default volume should be 0.5
        let vol = get_volume(&db).await.unwrap();
        assert_eq!(vol, 0.5);

        // Set new volume
        set_volume(&db, 0.75).await.unwrap();
        let vol = get_volume(&db).await.unwrap();
        assert_eq!(vol, 0.75);

        // Volume should be clamped
        set_volume(&db, 1.5).await.unwrap();
        let vol = get_volume(&db).await.unwrap();
        assert_eq!(vol, 1.0);

        set_volume(&db, -0.5).await.unwrap();
        let vol = get_volume(&db).await.unwrap();
        assert_eq!(vol, 0.0);
    }

    #[tokio::test]
    async fn test_audio_device_get_set() {
        let db = setup_test_db().await;

        // Default should be "default"
        let device = get_audio_device(&db).await.unwrap();
        assert_eq!(device, "default");

        // Set new device
        set_audio_device(&db, "hw:0,0".to_string())
            .await
            .unwrap();
        let device = get_audio_device(&db).await.unwrap();
        assert_eq!(device, "hw:0,0");
    }

    #[tokio::test]
    async fn test_crossfade_defaults() {
        let db = setup_test_db().await;

        // Get defaults (should initialize if missing)
        let defaults = get_crossfade_defaults(&db).await.unwrap();
        assert_eq!(defaults.crossfade_time_s, 2.0);
        assert_eq!(defaults.fade_curve, "exponential_logarithmic");

        // Modify and re-read
        set_setting(&db, "global_crossfade_time", 3.5)
            .await
            .unwrap();
        let defaults = get_crossfade_defaults(&db).await.unwrap();
        assert_eq!(defaults.crossfade_time_s, 3.5);
    }

    #[tokio::test]
    async fn test_generic_setting_get_set() {
        let db = setup_test_db().await;

        // Set an integer setting
        set_setting(&db, "test_int", 42).await.unwrap();
        let value: Option<i32> = get_setting(&db, "test_int").await.unwrap();
        assert_eq!(value, Some(42));

        // Set a string setting
        set_setting(&db, "test_str", "hello".to_string())
            .await
            .unwrap();
        let value: Option<String> = get_setting(&db, "test_str").await.unwrap();
        assert_eq!(value, Some("hello".to_string()));

        // Non-existent key should return None
        let value: Option<String> = get_setting(&db, "nonexistent").await.unwrap();
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_setting_update() {
        let db = setup_test_db().await;

        // Set initial value
        set_setting(&db, "test_key", "value1".to_string())
            .await
            .unwrap();
        let value: Option<String> = get_setting(&db, "test_key").await.unwrap();
        assert_eq!(value, Some("value1".to_string()));

        // Update value (should use UPSERT)
        set_setting(&db, "test_key", "value2".to_string())
            .await
            .unwrap();
        let value: Option<String> = get_setting(&db, "test_key").await.unwrap();
        assert_eq!(value, Some("value2".to_string()));
    }

    #[tokio::test]
    async fn test_playback_position_persistence() {
        let db = setup_test_db().await;

        // Initially should be None
        let pos = load_playback_position(&db).await.unwrap();
        assert_eq!(pos, None);

        // Save position
        save_playback_position(&db, 12345).await.unwrap();
        let pos = load_playback_position(&db).await.unwrap();
        assert_eq!(pos, Some(12345));

        // Update position
        save_playback_position(&db, 54321).await.unwrap();
        let pos = load_playback_position(&db).await.unwrap();
        assert_eq!(pos, Some(54321));
    }

    #[tokio::test]
    async fn test_last_passage_id_persistence() {
        let db = setup_test_db().await;

        // Initially should be None
        let id = load_last_passage_id(&db).await.unwrap();
        assert_eq!(id, None);

        // Save passage ID
        let test_id = Uuid::new_v4();
        save_last_passage_id(&db, test_id).await.unwrap();
        let loaded_id = load_last_passage_id(&db).await.unwrap();
        assert_eq!(loaded_id, Some(test_id));
    }

    #[tokio::test]
    async fn test_queue_state_persistence() {
        let db = setup_test_db().await;

        // Initially should be None
        let state = load_queue_state(&db).await.unwrap();
        assert_eq!(state, None);

        // Save queue state with current ID
        let test_id = Uuid::new_v4();
        save_queue_state(&db, Some(test_id)).await.unwrap();
        let loaded_state = load_queue_state(&db).await.unwrap();
        assert_eq!(loaded_state, Some(test_id));

        // Clear queue state (None)
        save_queue_state(&db, None).await.unwrap();
        let loaded_state = load_queue_state(&db).await.unwrap();
        assert_eq!(loaded_state, None);
    }
}
