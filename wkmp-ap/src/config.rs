//! Configuration loading and management
//!
//! Loads bootstrap configuration from TOML file and runtime settings from database.
//!
//! **Traceability:**
//! - CO-041 (Module configuration loading)
//! - Architecture: Database-first configuration strategy

use crate::error::{Error, Result};
use serde::Deserialize;
use std::path::PathBuf;
use sqlx::SqlitePool;

/// Bootstrap configuration loaded from TOML file
#[derive(Debug, Clone, Deserialize)]
pub struct TomlConfig {
    /// Database file path
    pub database_path: PathBuf,

    /// HTTP server port
    #[serde(default = "default_port")]
    pub port: u16,

    /// Root folder for audio files (optional, loaded from database if not specified)
    pub root_folder: Option<PathBuf>,
}

fn default_port() -> u16 {
    5721 // Default port for wkmp-ap from api_design.md
}

/// Runtime configuration combining TOML and database settings
#[derive(Debug, Clone)]
pub struct Config {
    /// Database file path
    pub database_path: PathBuf,

    /// HTTP server port
    pub port: u16,

    /// Root folder for audio files
    pub root_folder: Option<PathBuf>,

    /// Database connection pool
    pub db_pool: Option<SqlitePool>,
}

impl Config {
    /// Load configuration from TOML file with optional command-line overrides
    ///
    /// [ARCH-INIT-005] Root folder resolution priority:
    /// 1. --root-folder command-line argument
    /// 2. WKMP_ROOT_FOLDER environment variable
    /// 3. TOML config file root_folder field
    /// 4. Database settings.root_folder value
    /// 5. OS default (~/.local/share/wkmp on Linux, ~/Library/Application Support/wkmp on macOS)
    ///
    /// [ARCH-INIT-010] Module startup sequence
    /// [ISSUE-3] Complete initialization per requirements
    ///
    /// **Arguments:**
    /// - `config_path`: Path to TOML configuration file
    /// - `database_override`: Optional database path override
    /// - `port_override`: Optional port override
    /// - `root_folder_override`: Optional root folder override
    ///
    /// **Returns:** Configured Config instance with database connection pool
    ///
    /// **Traceability:** XFD-DB-030 (Global settings from database)
    pub async fn load(
        config_path: &PathBuf,
        database_override: Option<PathBuf>,
        port_override: Option<u16>,
        root_folder_override: Option<PathBuf>,
    ) -> Result<Self> {
        use tracing::info;

        // Load TOML configuration
        let toml_str = tokio::fs::read_to_string(config_path)
            .await
            .map_err(|e| Error::Config(format!("Failed to read config file: {}", e)))?;

        let toml_config: TomlConfig = toml::from_str(&toml_str)
            .map_err(|e| Error::Config(format!("Failed to parse TOML: {}", e)))?;

        // Apply overrides
        let database_path = database_override.unwrap_or(toml_config.database_path);
        let port = port_override.unwrap_or(toml_config.port);

        // Root folder resolution per ARCH-INIT-005
        // Priority: 1. CLI arg → 2. Env var → 3. TOML → 4. Database → 5. OS default
        let root_folder = if let Some(folder) = root_folder_override {
            info!("Root folder from command-line argument: {:?}", folder);
            Some(folder)
        } else if let Ok(env_folder) = std::env::var("WKMP_ROOT_FOLDER") {
            let folder = PathBuf::from(env_folder);
            info!("Root folder from WKMP_ROOT_FOLDER environment variable: {:?}", folder);
            Some(folder)
        } else if let Some(folder) = toml_config.root_folder {
            info!("Root folder from TOML config: {:?}", folder);
            Some(folder)
        } else {
            None // Will try database and OS default below
        };

        // Connect to database
        // [ISSUE-11] Add connection timeouts and retry configuration
        // [ARCH-ERRH-050] Database lock retry strategy
        // [ARCH-ERRH-070] Lock timeout configuration
        use sqlx::sqlite::SqlitePoolOptions;
        let db_url = format!("sqlite:{}?mode=rwc", database_path.display());
        let db_pool = SqlitePoolOptions::new()
            .max_connections(5)
            .acquire_timeout(std::time::Duration::from_secs(5))
            .idle_timeout(Some(std::time::Duration::from_secs(60)))
            .connect(&db_url)
            .await
            .map_err(|e| Error::Database(e))?;

        info!("Connected to database: {}", database_path.display());

        // Load root_folder from database if not specified yet
        let root_folder = if root_folder.is_none() {
            if let Some(db_folder) = Self::load_root_folder_from_db(&db_pool).await? {
                info!("Root folder from database: {:?}", db_folder);
                Some(db_folder)
            } else {
                // Use OS default
                let default_folder = Self::get_os_default_root_folder();
                info!("Root folder from OS default: {:?}", default_folder);
                Some(default_folder)
            }
        } else {
            root_folder
        };

        Ok(Config {
            database_path,
            port,
            root_folder,
            db_pool: Some(db_pool),
        })
    }

    /// Get OS-specific default root folder
    ///
    /// [ARCH-INIT-005] OS default root folder location
    /// [REQ-NF-033] Default root folder locations per platform
    ///
    /// Returns platform-appropriate default locations for music files:
    /// - **Linux**: `~/Music` (user's Music folder)
    /// - **macOS**: `~/Music` (user's Music folder)
    /// - **Windows**: `%USERPROFILE%\Music\wkmp` (user's Music folder with WKMP subfolder)
    /// - **Other**: `/tmp/wkmp` (fallback for unsupported platforms)
    ///
    /// **Rationale:** WKMP is a music player, so the default location should be
    /// where users typically store their music files, not application data folders.
    pub fn get_os_default_root_folder() -> PathBuf {
        #[cfg(target_os = "linux")]
        {
            // [REQ-NF-033] Linux default: ~/Music
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            PathBuf::from(home).join("Music")
        }

        #[cfg(target_os = "macos")]
        {
            // [REQ-NF-033] macOS default: ~/Music
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            PathBuf::from(home).join("Music")
        }

        #[cfg(target_os = "windows")]
        {
            // [REQ-NF-033] Windows default: %USERPROFILE%\Music\wkmp
            let userprofile = std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\".to_string());
            PathBuf::from(userprofile).join("Music").join("wkmp")
        }

        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            // Fallback for unsupported platforms
            PathBuf::from("/tmp/wkmp")
        }
    }

    /// Load root_folder from database settings table
    ///
    /// **Traceability:** Database schema - settings table (root_folder key)
    async fn load_root_folder_from_db(pool: &SqlitePool) -> Result<Option<PathBuf>> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT value FROM settings WHERE key = 'root_folder'"
        )
        .fetch_optional(pool)
        .await?;

        Ok(row.map(|(value,)| PathBuf::from(value)))
    }

    /// Get database connection pool
    pub fn db_pool(&self) -> &SqlitePool {
        self.db_pool.as_ref().expect("Database pool not initialized")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_default_port() {
        assert_eq!(default_port(), 5721);
    }
}
