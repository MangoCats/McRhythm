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
/// Clamped to valid range: 100-5000ms
pub async fn load_position_event_interval(db: &Pool<Sqlite>) -> Result<u32> {
    load_clamped_setting(db, "position_event_interval_ms", 100, 5000, 1000).await
}

/// Load playback_progress_interval_ms from settings table
///
/// **[REV002]** Event-driven position tracking
/// **[ADDENDUM-interval_configurability]** Configurable playback progress interval
///
/// # Returns
/// Interval in milliseconds (default: 5000ms if not set)
/// Clamped to valid range: 1000-60000ms (1-60 seconds)
pub async fn load_progress_interval(db: &Pool<Sqlite>) -> Result<u64> {
    load_clamped_setting(db, "playback_progress_interval_ms", 1000, 60000, 5000).await
}

/// Load buffer_underrun_recovery_timeout_ms from settings table
///
/// **[REQ-AP-ERR-020]** Buffer underrun recovery timeout
/// **[ERH-BUF-015]** Configurable timeout for emergency refill
///
/// # Returns
/// Timeout in milliseconds (default: 2000ms)
/// Clamped to valid range: 100-5000ms (0.1-5 seconds per spec)
/// Note: Default is 2000ms (not spec's 500ms) for slow hardware (Pi Zero 2W)
pub async fn load_buffer_underrun_timeout(db: &Pool<Sqlite>) -> Result<u64> {
    load_clamped_setting(db, "buffer_underrun_recovery_timeout_ms", 100, 5000, 2000).await
}

/// Load audio_ring_buffer_grace_period_ms from settings table
///
/// **[SSD-RBUF-014]** Ring buffer startup grace period
///
/// # Returns
/// Grace period in milliseconds (default: 2000ms if not set)
/// Clamped to valid range: 0-10000ms (0-10 seconds)
pub async fn load_ring_buffer_grace_period(db: &Pool<Sqlite>) -> Result<u64> {
    load_clamped_setting(db, "audio_ring_buffer_grace_period_ms", 0, 10000, 2000).await
}

/// Mixer thread configuration parameters
///
/// These parameters control the behavior of the mixer thread that fills
/// the audio ring buffer. Tuned values prevent both underruns (audio gaps)
/// and overruns (wasted CPU cycles).
///
/// **Traceability:**
/// - [SSD-MIX-020] Mixer thread ring buffer filling strategy
#[derive(Debug, Clone)]
pub struct MixerThreadConfig {
    /// Check interval in microseconds (how often mixer checks if filling needed)
    /// Default: 10μs - Fast enough to keep buffer filled without busy-waiting
    pub check_interval_us: u64,

    /// Batch size when buffer < 50% (aggressive filling)
    /// Default: 8 frames - Fills moderately to catch up quickly
    pub batch_size_low: usize,

    /// Batch size when buffer 50-75% (conservative top-up)
    /// Default: 2 frames - Small batches to avoid overshooting optimal range
    pub batch_size_optimal: usize,
}

/// Load minimum buffer threshold for instant playback start
///
/// **[PERF-START-010]** Configurable minimum buffer threshold
///
/// # Returns
/// Minimum buffer duration in milliseconds before starting playback
/// Default: 3000ms (3 seconds) - Conservative for Raspberry Pi Zero2W
/// Clamped to valid range: 500-5000ms (0.5-5 seconds)
pub async fn load_minimum_buffer_threshold(db: &Pool<Sqlite>) -> Result<u64> {
    load_clamped_setting(db, "minimum_buffer_threshold_ms", 500, 5000, 3000).await
}

/// Set minimum buffer threshold
///
/// **[PERF-START-010]** Configure minimum buffer for playback start
///
/// # Arguments
/// * `threshold_ms` - Minimum buffer duration in milliseconds (will be clamped to 500-5000)
pub async fn set_minimum_buffer_threshold(db: &Pool<Sqlite>, threshold_ms: u64) -> Result<()> {
    let clamped = threshold_ms.clamp(500, 5000);
    set_setting(db, "minimum_buffer_threshold_ms", clamped).await
}

/// Get decoder resume hysteresis in samples
///
/// **[DBD-PARAM-085]** Configurable hysteresis gap prevents decoder pause/resume oscillation
/// **[DBD-BUF-050]** Resume when free_space >= decoder_resume_hysteresis_samples + playout_ringbuffer_headroom
///
/// This value determines the hysteresis gap between pause and resume thresholds.
/// The decoder pauses when free_space <= playout_ringbuffer_headroom (default: 4410 samples),
/// and resumes when free_space >= hysteresis + headroom (default: 44100 + 4410 = 48510 samples).
/// Using the sum prevents issues where headroom is inadvertently set larger than hysteresis.
///
/// # Returns
/// Hysteresis threshold in samples (frames)
/// Default: 44100 samples (1.0 second @ 44.1kHz) - Large gap prevents oscillation
/// Clamped to valid range: 882-88200 samples (0.02-2.0 seconds)
pub async fn get_decoder_resume_hysteresis(db: &Pool<Sqlite>) -> Result<usize> {
    Ok(load_clamped_setting(db, "decoder_resume_hysteresis_samples", 882u64, 88200u64, 44100u64).await? as usize)
}

/// Set decoder resume hysteresis in samples
///
/// **[DBD-BUF-050]** Configure decoder resume threshold
///
/// # Arguments
/// * `samples` - Hysteresis threshold in samples (will be clamped to 882-88200)
pub async fn set_decoder_resume_hysteresis(db: &Pool<Sqlite>, samples: usize) -> Result<()> {
    let clamped = (samples as u64).clamp(882, 88200);
    set_setting(db, "decoder_resume_hysteresis_samples", clamped).await
}

/// Load mixer thread configuration from settings table
///
/// **[SSD-MIX-020]** Mixer thread parameters
///
/// # Returns
/// Mixer thread configuration with validated defaults
pub async fn load_mixer_thread_config(db: &Pool<Sqlite>) -> Result<MixerThreadConfig> {
    // [DBD-PARAM-111] Load mixer check interval from database (in milliseconds)
    // Clamp to valid range: 1-100ms
    // Too low = async overhead dominates, too high = buffer underruns
    // Default: 10ms - Conservative value for VeryHigh stability confidence (empirically tuned)
    let check_interval_ms = load_clamped_setting(db, "mixer_check_interval_ms", 1u64, 100u64, 10u64).await?;

    // Convert milliseconds to microseconds for tokio::time::interval
    let check_interval_us = check_interval_ms * 1000;

    // Clamp to valid range: 16-1024 frames
    // Too low = slow recovery, too high = excessive lock hold time
    // Default: 512 frames - Tuned to keep output ring buffer filled under 2048-frame callback loads
    let batch_size_low = load_clamped_setting(db, "mixer_batch_size_low", 16usize, 1024usize, 512usize).await?;

    // Clamp to valid range: 16-512 frames
    // Too low = async overhead dominates, too high = overshooting
    // Default: 256 frames - Tuned to match audio callback drain rate (~441 frames/10ms)
    let batch_size_optimal = load_clamped_setting(db, "mixer_batch_size_optimal", 16usize, 512usize, 256usize).await?;

    Ok(MixerThreadConfig {
        check_interval_us,
        batch_size_low,
        batch_size_optimal,
    })
}

/// Load a clamped setting from the database with a default fallback
///
/// Generic helper for loading numeric settings that require validation via clamping.
/// This eliminates repetitive code across parameter loading functions.
///
/// # Type Parameters
/// - `T`: The type of the setting value (must implement FromStr, Ord, Copy)
///
/// # Arguments
/// - `db`: Database pool
/// - `key`: Settings table key name
/// - `min`: Minimum valid value (inclusive)
/// - `max`: Maximum valid value (inclusive)
/// - `default`: Default value if key doesn't exist
///
/// # Returns
/// The setting value, clamped to [min, max], or default if not set
///
/// **Traceability:** DB-SETTINGS-075 (DRY helper for clamped parameters)
pub async fn load_clamped_setting<T>(
    db: &Pool<Sqlite>,
    key: &str,
    min: T,
    max: T,
    default: T,
) -> Result<T>
where
    T: FromStr + Ord + Copy,
{
    match get_setting::<T>(db, key).await? {
        Some(value) => Ok(value.clamp(min, max)),
        None => Ok(default),
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

/// Load resume-from-pause fade-in duration (milliseconds)
///
/// **[XFD-PAUS-020]** Configurable resume fade-in duration
///
/// # Arguments
/// * `db` - Database connection pool
///
/// # Returns
/// Fade-in duration in milliseconds (default: 500ms = 0.5 seconds)
///
/// **Traceability:** XFD-PAUS-020
pub async fn load_resume_fade_in_duration(db: &Pool<Sqlite>) -> Result<u64> {
    match get_setting::<u64>(db, "resume_from_pause_fade_in_duration").await? {
        Some(duration) => Ok(duration),
        None => Ok(500), // Default: 0.5 seconds
    }
}

/// Load resume-from-pause fade-in curve
///
/// **[XFD-PAUS-020]** Configurable resume fade-in curve
///
/// # Arguments
/// * `db` - Database connection pool
///
/// # Returns
/// Fade curve name: "linear", "exponential", or "cosine" (default: "exponential")
///
/// **Traceability:** XFD-PAUS-020
pub async fn load_resume_fade_in_curve(db: &Pool<Sqlite>) -> Result<String> {
    match get_setting::<String>(db, "resume_from_pause_fade_in_curve").await? {
        Some(curve) => Ok(curve),
        None => Ok("exponential".to_string()), // Default: exponential curve
    }
}

/// Load maximum_decode_streams from settings table
///
/// **[DBD-PARAM-050]** Maximum number of decoder-resampler-fade-buffer chains
///
/// # Returns
/// Maximum decode streams (default: 12 if not set)
/// Clamped to valid range: 2-32 (minimum 2 for current+next passages)
///
/// **Traceability:** DBD-PARAM-050
pub async fn load_maximum_decode_streams(db: &Pool<Sqlite>) -> Result<usize> {
    load_clamped_setting(db, "maximum_decode_streams", 2usize, 32usize, 12usize).await
}

/// Load mixer minimum start level from database
///
/// **[DBD-PARAM-088]** Minimum buffer samples required before mixer starts playback
///
/// # Returns
/// Number of samples (default: 44100 = 1.0 second @ 44.1kHz)
/// Clamped to valid range: 8820-220500 samples (0.2-5.0 seconds @ 44.1kHz)
pub async fn load_mixer_min_start_level(db: &Pool<Sqlite>) -> Result<usize> {
    load_clamped_setting(db, "mixer_min_start_level", 8820usize, 220500usize, 44100usize).await
}

/// Load audio buffer size from database
///
/// **[DBD-PARAM-110]** Audio output buffer size in frames per callback
///
/// # Returns
/// Buffer size in frames (default: 2208 - empirically tuned for stability)
/// Clamped to valid range: 64-65536 frames
/// - Smaller buffers: Lower latency but higher CPU usage
/// - Larger buffers: Higher latency but more stable on slow systems
/// - Default (2208 frames = 50.1ms) provides VeryHigh stability confidence
pub async fn load_audio_buffer_size(db: &Pool<Sqlite>) -> Result<u32> {
    load_clamped_setting(db, "audio_buffer_size", 64, 65536, 2208).await
}

/// Load playout ring buffer capacity from database
///
/// **[DBD-PARAM-070]** Playout ring buffer capacity in stereo frames
/// - Default: 661,941 frames (15.01 seconds @ 44.1kHz stereo)
/// - Range: 88,200 to 2,646,000 frames (2-60 seconds @ 44.1kHz)
pub async fn load_playout_ringbuffer_capacity(db: &Pool<Sqlite>) -> Result<usize> {
    load_clamped_setting(db, "playout_ringbuffer_capacity", 88_200, 2_646_000, 661_941).await
}

/// Load playout ring buffer headroom threshold from database
///
/// **[DBD-PARAM-080]** Playout ring buffer headroom threshold in stereo frames
/// - Default: 4,410 frames (0.1 seconds @ 44.1kHz stereo)
/// - Range: 1,000 to 44,100 frames (0.023-1.0 seconds @ 44.1kHz)
pub async fn load_playout_ringbuffer_headroom(db: &Pool<Sqlite>) -> Result<usize> {
    load_clamped_setting(db, "playout_ringbuffer_headroom", 1_000, 44_100, 4_410).await
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

    /// **[DB-SETTINGS-020]** Test default volume on first run (empty database)
    #[tokio::test]
    async fn test_default_volume_on_first_run() {
        let db = setup_test_db().await;

        // Database is empty - no volume_level entry exists yet
        // First call to get_volume should return default 0.5 and persist it
        let vol = get_volume(&db).await.unwrap();
        assert_eq!(vol, 0.5, "Default volume should be 0.5");

        // Verify it was persisted to database
        let stored_vol: String = sqlx::query_scalar(
            "SELECT value FROM settings WHERE key = 'volume_level'"
        )
        .fetch_one(&db)
        .await
        .unwrap();

        assert_eq!(stored_vol, "0.5", "Volume should be persisted to database");

        // Subsequent calls should return the same value
        let vol2 = get_volume(&db).await.unwrap();
        assert_eq!(vol2, 0.5, "Volume should remain 0.5");
    }

    /// **[DB-SETTINGS-020]** Test volume persistence continues to work after errors
    /// Note: This tests best-effort persistence - errors are logged but don't fail the operation
    #[tokio::test]
    async fn test_volume_persistence_continues_after_errors() {
        let db = setup_test_db().await;

        // Set initial volume successfully
        set_volume(&db, 0.7).await.unwrap();
        let vol = get_volume(&db).await.unwrap();
        assert_eq!(vol, 0.7);

        // Note: We can't easily simulate database write failures in SQLite in-memory mode
        // without significant mocking infrastructure. The actual error handling is in the
        // API handler (handlers.rs:265) which logs errors but continues.

        // Verify that clamping still works even with extreme values
        set_volume(&db, 999.0).await.unwrap();
        let vol = get_volume(&db).await.unwrap();
        assert_eq!(vol, 1.0, "Volume should be clamped to 1.0");

        set_volume(&db, -999.0).await.unwrap();
        let vol = get_volume(&db).await.unwrap();
        assert_eq!(vol, 0.0, "Volume should be clamped to 0.0");
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

    #[tokio::test]
    async fn test_resume_fade_in_duration_default() {
        // [XFD-PAUS-020] Verify default resume fade-in duration
        let db = setup_test_db().await;

        // When setting doesn't exist, should return default 500ms
        let duration = load_resume_fade_in_duration(&db).await.unwrap();
        assert_eq!(duration, 500, "Default resume fade-in duration should be 500ms");
    }

    #[tokio::test]
    async fn test_resume_fade_in_duration_custom() {
        // [XFD-PAUS-020] Verify custom resume fade-in duration persists
        let db = setup_test_db().await;

        // Set custom duration (1000ms = 1 second)
        set_setting(&db, "resume_from_pause_fade_in_duration", 1000u64)
            .await
            .unwrap();

        // Should load custom value
        let duration = load_resume_fade_in_duration(&db).await.unwrap();
        assert_eq!(duration, 1000, "Custom resume fade-in duration should persist");

        // Set different custom duration (250ms)
        set_setting(&db, "resume_from_pause_fade_in_duration", 250u64)
            .await
            .unwrap();

        let duration = load_resume_fade_in_duration(&db).await.unwrap();
        assert_eq!(duration, 250, "Updated resume fade-in duration should persist");
    }

    #[tokio::test]
    async fn test_resume_fade_in_curve_default() {
        // [XFD-PAUS-020] Verify default resume fade-in curve
        let db = setup_test_db().await;

        // When setting doesn't exist, should return default "exponential"
        let curve = load_resume_fade_in_curve(&db).await.unwrap();
        assert_eq!(curve, "exponential", "Default resume fade-in curve should be exponential");
    }

    #[tokio::test]
    async fn test_resume_fade_in_curve_custom() {
        // [XFD-PAUS-020] Verify custom resume fade-in curve persists
        let db = setup_test_db().await;

        // Set custom curve to "linear"
        set_setting(&db, "resume_from_pause_fade_in_curve", "linear".to_string())
            .await
            .unwrap();

        // Should load custom value
        let curve = load_resume_fade_in_curve(&db).await.unwrap();
        assert_eq!(curve, "linear", "Custom resume fade-in curve should persist");

        // Change to exponential
        set_setting(&db, "resume_from_pause_fade_in_curve", "exponential".to_string())
            .await
            .unwrap();

        let curve = load_resume_fade_in_curve(&db).await.unwrap();
        assert_eq!(curve, "exponential", "Updated resume fade-in curve should persist");
    }

    /// **[DBD-PARAM-050]** Test default maximum_decode_streams loading
    #[tokio::test]
    async fn test_maximum_decode_streams_default() {
        let db = setup_test_db().await;

        // When setting doesn't exist, should return default 12
        let max_streams = load_maximum_decode_streams(&db).await.unwrap();
        assert_eq!(max_streams, 12, "Default maximum_decode_streams should be 12");
    }

    /// **[DBD-PARAM-050]** Test custom maximum_decode_streams loading
    #[tokio::test]
    async fn test_maximum_decode_streams_custom() {
        let db = setup_test_db().await;

        // Set custom value (8 streams)
        set_setting(&db, "maximum_decode_streams", 8usize)
            .await
            .unwrap();

        let max_streams = load_maximum_decode_streams(&db).await.unwrap();
        assert_eq!(max_streams, 8, "Custom maximum_decode_streams should persist");

        // Change to 16 streams
        set_setting(&db, "maximum_decode_streams", 16usize)
            .await
            .unwrap();

        let max_streams = load_maximum_decode_streams(&db).await.unwrap();
        assert_eq!(max_streams, 16, "Updated maximum_decode_streams should persist");
    }

    /// **[DBD-PARAM-050]** Test maximum_decode_streams clamping (2-32 range)
    #[tokio::test]
    async fn test_maximum_decode_streams_clamping() {
        let db = setup_test_db().await;

        // Test lower bound: 1 should clamp to 2
        set_setting(&db, "maximum_decode_streams", 1usize)
            .await
            .unwrap();
        let max_streams = load_maximum_decode_streams(&db).await.unwrap();
        assert_eq!(max_streams, 2, "maximum_decode_streams should clamp minimum to 2");

        // Test lower bound: 0 should clamp to 2
        set_setting(&db, "maximum_decode_streams", 0usize)
            .await
            .unwrap();
        let max_streams = load_maximum_decode_streams(&db).await.unwrap();
        assert_eq!(max_streams, 2, "maximum_decode_streams should clamp 0 to 2");

        // Test upper bound: 64 should clamp to 32
        set_setting(&db, "maximum_decode_streams", 64usize)
            .await
            .unwrap();
        let max_streams = load_maximum_decode_streams(&db).await.unwrap();
        assert_eq!(max_streams, 32, "maximum_decode_streams should clamp maximum to 32");

        // Test upper bound: 100 should clamp to 32
        set_setting(&db, "maximum_decode_streams", 100usize)
            .await
            .unwrap();
        let max_streams = load_maximum_decode_streams(&db).await.unwrap();
        assert_eq!(max_streams, 32, "maximum_decode_streams should clamp 100 to 32");

        // Test valid value within range: 10 should remain 10
        set_setting(&db, "maximum_decode_streams", 10usize)
            .await
            .unwrap();
        let max_streams = load_maximum_decode_streams(&db).await.unwrap();
        assert_eq!(max_streams, 10, "maximum_decode_streams should not clamp valid values");
    }

    /// **[DB-SETTINGS-075]** Test load_clamped_setting helper with u32 type
    #[tokio::test]
    async fn test_load_clamped_setting_u32() {
        let db = setup_test_db().await;

        // Test missing key (should return default)
        let value = load_clamped_setting(&db, "test_clamped_u32", 10u32, 100u32, 50u32).await.unwrap();
        assert_eq!(value, 50, "Missing key should return default value");

        // Test value within range (should return value unchanged)
        set_setting(&db, "test_clamped_u32", 75u32).await.unwrap();
        let value = load_clamped_setting(&db, "test_clamped_u32", 10u32, 100u32, 50u32).await.unwrap();
        assert_eq!(value, 75, "Value within range should be returned unchanged");

        // Test value below min (should clamp to min)
        set_setting(&db, "test_clamped_u32", 5u32).await.unwrap();
        let value = load_clamped_setting(&db, "test_clamped_u32", 10u32, 100u32, 50u32).await.unwrap();
        assert_eq!(value, 10, "Value below min should clamp to min");

        // Test value above max (should clamp to max)
        set_setting(&db, "test_clamped_u32", 150u32).await.unwrap();
        let value = load_clamped_setting(&db, "test_clamped_u32", 10u32, 100u32, 50u32).await.unwrap();
        assert_eq!(value, 100, "Value above max should clamp to max");

        // Test value at min boundary (should return min)
        set_setting(&db, "test_clamped_u32", 10u32).await.unwrap();
        let value = load_clamped_setting(&db, "test_clamped_u32", 10u32, 100u32, 50u32).await.unwrap();
        assert_eq!(value, 10, "Value at min boundary should return min");

        // Test value at max boundary (should return max)
        set_setting(&db, "test_clamped_u32", 100u32).await.unwrap();
        let value = load_clamped_setting(&db, "test_clamped_u32", 10u32, 100u32, 50u32).await.unwrap();
        assert_eq!(value, 100, "Value at max boundary should return max");
    }

    /// **[DB-SETTINGS-075]** Test load_clamped_setting helper with u64 type
    #[tokio::test]
    async fn test_load_clamped_setting_u64() {
        let db = setup_test_db().await;

        // Test missing key (should return default)
        let value = load_clamped_setting(&db, "test_clamped_u64", 1000u64, 60000u64, 5000u64).await.unwrap();
        assert_eq!(value, 5000, "Missing key should return default value");

        // Test value within range (should return value unchanged)
        set_setting(&db, "test_clamped_u64", 30000u64).await.unwrap();
        let value = load_clamped_setting(&db, "test_clamped_u64", 1000u64, 60000u64, 5000u64).await.unwrap();
        assert_eq!(value, 30000, "Value within range should be returned unchanged");

        // Test value below min (should clamp to min)
        set_setting(&db, "test_clamped_u64", 500u64).await.unwrap();
        let value = load_clamped_setting(&db, "test_clamped_u64", 1000u64, 60000u64, 5000u64).await.unwrap();
        assert_eq!(value, 1000, "Value below min should clamp to min");

        // Test value above max (should clamp to max)
        set_setting(&db, "test_clamped_u64", 100000u64).await.unwrap();
        let value = load_clamped_setting(&db, "test_clamped_u64", 1000u64, 60000u64, 5000u64).await.unwrap();
        assert_eq!(value, 60000, "Value above max should clamp to max");
    }

    /// **[DB-SETTINGS-075]** Test load_clamped_setting helper with usize type
    #[tokio::test]
    async fn test_load_clamped_setting_usize() {
        let db = setup_test_db().await;

        // Test missing key (should return default)
        let value = load_clamped_setting(&db, "test_clamped_usize", 2usize, 32usize, 12usize).await.unwrap();
        assert_eq!(value, 12, "Missing key should return default value");

        // Test value within range (should return value unchanged)
        set_setting(&db, "test_clamped_usize", 16usize).await.unwrap();
        let value = load_clamped_setting(&db, "test_clamped_usize", 2usize, 32usize, 12usize).await.unwrap();
        assert_eq!(value, 16, "Value within range should be returned unchanged");

        // Test value below min (should clamp to min)
        set_setting(&db, "test_clamped_usize", 1usize).await.unwrap();
        let value = load_clamped_setting(&db, "test_clamped_usize", 2usize, 32usize, 12usize).await.unwrap();
        assert_eq!(value, 2, "Value below min should clamp to min");

        // Test value above max (should clamp to max)
        set_setting(&db, "test_clamped_usize", 64usize).await.unwrap();
        let value = load_clamped_setting(&db, "test_clamped_usize", 2usize, 32usize, 12usize).await.unwrap();
        assert_eq!(value, 32, "Value above max should clamp to max");
    }
}
