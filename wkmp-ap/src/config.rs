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
        // Load TOML configuration
        let toml_str = tokio::fs::read_to_string(config_path)
            .await
            .map_err(|e| Error::Config(format!("Failed to read config file: {}", e)))?;

        let toml_config: TomlConfig = toml::from_str(&toml_str)
            .map_err(|e| Error::Config(format!("Failed to parse TOML: {}", e)))?;

        // Apply overrides
        let database_path = database_override.unwrap_or(toml_config.database_path);
        let port = port_override.unwrap_or(toml_config.port);
        let root_folder = root_folder_override.or(toml_config.root_folder);

        // Connect to database
        let db_url = format!("sqlite:{}?mode=rwc", database_path.display());
        let db_pool = SqlitePool::connect(&db_url)
            .await
            .map_err(|e| Error::Database(e))?;

        // Load root_folder from database if not specified in config or command-line
        let root_folder = if root_folder.is_none() {
            Self::load_root_folder_from_db(&db_pool).await?
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
