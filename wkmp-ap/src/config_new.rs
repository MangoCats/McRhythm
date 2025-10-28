//! Configuration management for wkmp-ap Audio Player
//!
//! Implements two-tier configuration per SPEC001 Architecture:
//! 1. **TOML Bootstrap**: Database path, port, logging (static, bootstrap only)
//! 2. **Database Runtime**: All runtime settings from `settings` table
//!
//! # Configuration Philosophy
//!
//! - **Database-first**: All runtime configuration in `settings` table
//! - **TOML minimal**: Bootstrap only (cannot change while running)
//! - **NULL handling**: Missing/NULL settings initialized with built-in defaults
//! - **Built-in defaults**: Defined in code, not external files
//!
//! # Settings Sources Priority
//!
//! 1. Command-line arguments (--port, --database)
//! 2. Environment variables (WKMP_ROOT_FOLDER)
//! 3. TOML configuration file
//! 4. Database settings table
//! 5. Built-in defaults (code constants)
//!
//! See IMPL001-database_schema.md settings table for complete settings reference.

use crate::error::{AudioPlayerError, Result};
use serde::Deserialize;
use sqlx::SqlitePool;
use std::path::PathBuf;
use std::time::Duration;
use tracing::{error, info, warn};

/// Bootstrap configuration loaded from TOML file
///
/// These settings cannot change during runtime. Application must restart
/// to pick up changes to TOML file.
///
/// **Minimal by design** - per Architecture, only bootstrap concerns here.
#[derive(Debug, Clone, Deserialize)]
pub struct TomlConfig {
    /// Path to SQLite database file (relative or absolute)
    pub database_path: PathBuf,

    /// HTTP server port
    ///
    /// Default: 5721 (wkmp-ap standard port per SPEC007)
    #[serde(default = "default_port")]
    pub port: u16,

    /// Root folder for audio files (optional)
    ///
    /// If not specified, will attempt database → environment → OS default
    #[serde(default)]
    pub root_folder: Option<PathBuf>,

    /// Logging configuration (optional)
    #[serde(default)]
    pub logging: LoggingConfig,
}

/// Logging configuration
#[derive(Debug, Clone, Deserialize, Default)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub level: String,

    /// Log file path (optional, logs to stderr if not specified)
    #[serde(default)]
    pub file: Option<PathBuf>,
}

fn default_port() -> u16 {
    5721 // wkmp-ap port per SPEC007
}

fn default_log_level() -> String {
    "info".to_string()
}

/// Runtime settings loaded from database
///
/// All values have built-in defaults. NULL or missing database values
/// are automatically initialized with defaults and written back to database.
///
/// See IMPL001 settings table for complete documentation of each setting.
#[derive(Debug, Clone)]
pub struct RuntimeSettings {
    // === Playback State ===
    pub initial_play_state: PlayState,
    pub currently_playing_passage_id: Option<String>,
    pub volume_level: f64,
    pub audio_sink: String,

    // === Crossfade Configuration ===
    pub global_crossfade_time: f64,
    pub global_fade_curve: String,

    // === Event Timing ===
    pub position_event_interval_ms: u32,
    pub playback_progress_interval_ms: u32,

    // === Queue Management ===
    pub queue_max_size: usize,

    // === HTTP Server ===
    pub http_request_timeout_ms: u64,
    pub http_keepalive_timeout_ms: u64,
    pub http_max_body_size_bytes: usize,

    // === Error Handling (from SPEC021 fixes) ===
    pub buffer_underrun_recovery_timeout_ms: u32,

    // === Security (from SPEC007 fixes) ===
    pub api_shared_secret: Option<i64>,
}

/// Playback state enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayState {
    Playing,
    Paused,
}

impl RuntimeSettings {
    /// Load runtime settings from database
    ///
    /// For each setting:
    /// 1. Try to read from database
    /// 2. If NULL or missing, use built-in default
    /// 3. Write default back to database for consistency
    ///
    /// Per IMPL001 settings table philosophy: "Database-first configuration"
    pub async fn load(pool: &SqlitePool) -> Result<Self> {
        // Helper to get setting with default
        async fn get_setting<T>(
            pool: &SqlitePool,
            key: &str,
            default: T,
            parse: fn(&str) -> Result<T>,
        ) -> Result<T>
        where
            T: ToString + Clone,
        {
            let value_opt: Option<(String,)> = sqlx::query_as(
                "SELECT value FROM settings WHERE key = ?"
            )
            .bind(key)
            .fetch_optional(pool)
            .await
            .map_err(AudioPlayerError::Database)?;

            match value_opt {
                Some((value,)) => {
                    // Parse existing value
                    parse(&value)
                }
                None => {
                    // NULL or missing: use default and write back
                    info!("Setting '{}' not found in database, using default: {}", key, default.to_string());
                    let default_str = default.to_string();
                    sqlx::query(
                        "INSERT OR REPLACE INTO settings (key, value, updated_at) VALUES (?, ?, CURRENT_TIMESTAMP)"
                    )
                    .bind(key)
                    .bind(&default_str)
                    .execute(pool)
                    .await
                    .map_err(AudioPlayerError::Database)?;

                    Ok(default)
                }
            }
        }

        // Parse helpers
        let parse_string = |s: &str| Ok(s.to_string());
        let parse_f64 = |s: &str| s.parse::<f64>()
            .map_err(|e| AudioPlayerError::Configuration(format!("Invalid f64: {}", e)));
        let parse_u32 = |s: &str| s.parse::<u32>()
            .map_err(|e| AudioPlayerError::Configuration(format!("Invalid u32: {}", e)));
        let parse_u64 = |s: &str| s.parse::<u64>()
            .map_err(|e| AudioPlayerError::Configuration(format!("Invalid u64: {}", e)));
        let parse_usize = |s: &str| s.parse::<usize>()
            .map_err(|e| AudioPlayerError::Configuration(format!("Invalid usize: {}", e)));

        // Load all settings with defaults per IMPL001 table
        let settings = Self {
            // Playback State
            initial_play_state: {
                let s = get_setting(pool, "initial_play_state", "playing".to_string(), parse_string).await?;
                match s.as_str() {
                    "playing" => PlayState::Playing,
                    "paused" => PlayState::Paused,
                    _ => {
                        warn!("Invalid initial_play_state '{}', using default 'playing'", s);
                        PlayState::Playing
                    }
                }
            },
            currently_playing_passage_id: {
                let value_opt: Option<(String,)> = sqlx::query_as(
                    "SELECT value FROM settings WHERE key = 'currently_playing_passage_id'"
                )
                .fetch_optional(pool)
                .await
                .map_err(AudioPlayerError::Database)?;
                value_opt.map(|(v,)| v)
            },
            volume_level: get_setting(pool, "volume_level", 0.5, parse_f64).await?,
            audio_sink: get_setting(pool, "audio_sink", "default".to_string(), parse_string).await?,

            // Crossfade
            global_crossfade_time: get_setting(pool, "global_crossfade_time", 2.0, parse_f64).await?,
            global_fade_curve: get_setting(pool, "global_fade_curve", "exponential_logarithmic".to_string(), parse_string).await?,

            // Event Timing
            position_event_interval_ms: get_setting(pool, "position_event_interval_ms", 1000, parse_u32).await?,
            playback_progress_interval_ms: get_setting(pool, "playback_progress_interval_ms", 5000, parse_u32).await?,

            // Queue Management
            queue_max_size: get_setting(pool, "queue_max_size", 100, parse_usize).await?,

            // HTTP Server
            http_request_timeout_ms: get_setting(pool, "http_request_timeout_ms", 30000, parse_u64).await?,
            http_keepalive_timeout_ms: get_setting(pool, "http_keepalive_timeout_ms", 60000, parse_u64).await?,
            http_max_body_size_bytes: get_setting(pool, "http_max_body_size_bytes", 1048576, parse_usize).await?,

            // Error Handling (SPEC021 ERH-BUF-015: configurable buffer underrun timeout)
            buffer_underrun_recovery_timeout_ms: get_setting(pool, "buffer_underrun_recovery_timeout_ms", 500, parse_u32).await?,

            // Security (SPEC007 API-AUTH-028: shared secret for hash authentication)
            api_shared_secret: {
                // Use wkmp_common function to load or initialize shared secret
                match wkmp_common::api::load_shared_secret(pool).await {
                    Ok(secret) => {
                        if secret == 0 {
                            info!("API authentication disabled (shared_secret = 0)");
                            Some(0)
                        } else {
                            info!("Loaded API shared secret from database");
                            Some(secret)
                        }
                    }
                    Err(e) => {
                        error!("Failed to load API shared secret: {}", e);
                        None
                    }
                }
            },
        };

        info!("Loaded runtime settings from database");
        Ok(settings)
    }

    /// Get buffer underrun recovery timeout as Duration
    ///
    /// Per SPEC021 ERH-BUF-015, timeout is configurable (default 500ms)
    pub fn buffer_underrun_timeout(&self) -> Duration {
        Duration::from_millis(self.buffer_underrun_recovery_timeout_ms as u64)
    }

    /// Get HTTP request timeout as Duration
    pub fn http_request_timeout(&self) -> Duration {
        Duration::from_millis(self.http_request_timeout_ms)
    }

    /// Get HTTP keepalive timeout as Duration
    pub fn http_keepalive_timeout(&self) -> Duration {
        Duration::from_millis(self.http_keepalive_timeout_ms)
    }
}

/// Complete application configuration
///
/// Combines bootstrap (TOML) and runtime (database) configuration
#[derive(Debug, Clone)]
pub struct Config {
    /// Database file path
    pub database_path: PathBuf,

    /// HTTP server port
    pub port: u16,

    /// Root folder for audio files (optional)
    pub root_folder: Option<PathBuf>,

    /// Database connection pool
    pub db_pool: SqlitePool,

    /// Runtime settings from database
    pub runtime: RuntimeSettings,
}

impl Config {
    /// Load complete configuration from TOML and database
    ///
    /// # Arguments
    ///
    /// - `toml_path`: Path to TOML configuration file
    /// - `cli_overrides`: Optional command-line overrides
    ///
    /// # Configuration Priority
    ///
    /// 1. Command-line arguments (highest priority)
    /// 2. Environment variables (WKMP_ROOT_FOLDER)
    /// 3. TOML configuration file
    /// 4. Database settings table
    /// 5. Built-in defaults (lowest priority)
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - TOML file cannot be read or parsed
    /// - Database connection fails
    /// - Database settings cannot be loaded
    pub async fn load(
        toml_path: &PathBuf,
        cli_overrides: ConfigOverrides,
    ) -> Result<Self> {
        // Load TOML bootstrap configuration
        let toml_str = tokio::fs::read_to_string(toml_path)
            .await
            .map_err(|e| AudioPlayerError::Configuration(
                format!("Failed to read config file {:?}: {}", toml_path, e)
            ))?;

        let toml_config: TomlConfig = toml::from_str(&toml_str)
            .map_err(|e| AudioPlayerError::Configuration(
                format!("Failed to parse TOML: {}", e)
            ))?;

        info!("Loaded TOML configuration from {:?}", toml_path);

        // Apply CLI overrides
        let database_path = cli_overrides.database_path.unwrap_or(toml_config.database_path);
        let port = cli_overrides.port.unwrap_or(toml_config.port);

        // Connect to database
        let db_url = format!("sqlite:{}?mode=rwc", database_path.display());
        let db_pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(5))
            .idle_timeout(Some(Duration::from_secs(60)))
            .connect(&db_url)
            .await
            .map_err(AudioPlayerError::Database)?;

        info!("Connected to database: {:?}", database_path);

        // Load runtime settings from database
        let runtime = RuntimeSettings::load(&db_pool).await?;

        // Resolve root folder using wkmp-common's resolver
        // Handles priority: CLI > env > TOML > OS default
        let resolver = wkmp_common::config::RootFolderResolver::new("wkmp-ap");
        let root_folder = if let Some(override_path) = cli_overrides.root_folder {
            // CLI override takes highest priority
            info!("Root folder: {:?} (from CLI override)", override_path);
            Some(override_path)
        } else if toml_config.root_folder.is_some() {
            // TOML config has root_folder, use RootFolderResolver to handle env priority
            let resolved = resolver.resolve();
            info!("Root folder: {:?} (resolved)", resolved);
            Some(resolved)
        } else {
            // No TOML config, let resolver handle env + defaults
            let resolved = resolver.resolve();
            info!("Root folder: {:?} (resolved)", resolved);
            Some(resolved)
        };

        Ok(Config {
            database_path,
            port,
            root_folder,
            db_pool,
            runtime,
        })
    }

    // os_default_root_folder() removed - use wkmp_common::config::get_default_root_folder()
}

/// Command-line configuration overrides
#[derive(Debug, Clone, Default)]
pub struct ConfigOverrides {
    pub database_path: Option<PathBuf>,
    pub port: Option<u16>,
    pub root_folder: Option<PathBuf>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_port() {
        assert_eq!(default_port(), 5721);
    }

    #[test]
    fn test_default_log_level() {
        assert_eq!(default_log_level(), "info");
    }

    #[test]
    fn test_default_root_folder() {
        // Now uses wkmp-common's get_default_root_folder()
        let folder = wkmp_common::config::get_default_root_folder();
        // Just verify it returns a valid path
        assert!(!folder.as_os_str().is_empty());
    }

    #[test]
    fn test_play_state() {
        assert_eq!(PlayState::Playing, PlayState::Playing);
        assert_ne!(PlayState::Playing, PlayState::Paused);
    }
}
