//! Configuration loading and root folder resolution

use crate::{Error, Result};
use std::path::PathBuf;

/// Module configuration from database
#[derive(Debug, Clone)]
pub struct ModuleConfig {
    pub module_name: String,
    pub host: String,
    pub port: u16,
    pub enabled: bool,
}

/// Root folder resolution following ARCH-INIT-005 priority order:
/// 1. Command-line argument (highest priority)
/// 2. Environment variable
/// 3. TOML config file
/// 4. OS-dependent compiled default (fallback)
pub fn resolve_root_folder(
    cli_arg: Option<&str>,
    env_var_name: &str,
    config_file_key: Option<&str>,
) -> Result<PathBuf> {
    // Priority 1: Command-line argument
    if let Some(path) = cli_arg {
        return Ok(PathBuf::from(path));
    }

    // Priority 2: Environment variable
    if let Ok(path) = std::env::var(env_var_name) {
        return Ok(PathBuf::from(path));
    }

    // Priority 3: TOML config file
    if config_file_key.is_some() {
        if let Ok(config_path) = load_config_file() {
            if let Ok(toml_content) = std::fs::read_to_string(&config_path) {
                if let Ok(config) = toml::from_str::<toml::Value>(&toml_content) {
                    if let Some(root_folder) = config.get("root_folder").and_then(|v| v.as_str()) {
                        return Ok(PathBuf::from(root_folder));
                    }
                }
            }
        }
    }

    // Priority 4: OS-dependent compiled default
    Ok(get_default_root_folder())
}

/// Get default configuration file path for the platform
fn load_config_file() -> Result<PathBuf> {
    let config_dir = if cfg!(target_os = "linux") {
        // Try ~/.config/wkmp/config.toml first, then /etc/wkmp/config.toml
        let user_config = dirs::config_dir()
            .map(|d| d.join("wkmp").join("config.toml"));
        let system_config = PathBuf::from("/etc/wkmp/config.toml");

        if let Some(path) = user_config {
            if path.exists() {
                return Ok(path);
            }
        }
        if system_config.exists() {
            return Ok(system_config);
        }
        return Err(Error::Config("No config file found".to_string()));
    } else if cfg!(target_os = "macos") {
        dirs::config_dir()
            .map(|d| d.join("wkmp").join("config.toml"))
            .ok_or_else(|| Error::Config("Could not determine config directory".to_string()))?
    } else if cfg!(target_os = "windows") {
        dirs::config_dir()
            .map(|d| d.join("wkmp").join("config.toml"))
            .ok_or_else(|| Error::Config("Could not determine config directory".to_string()))?
    } else {
        return Err(Error::Config("Unsupported platform".to_string()));
    };

    if config_dir.exists() {
        Ok(config_dir)
    } else {
        Err(Error::Config(format!("Config file not found: {:?}", config_dir)))
    }
}

/// Get OS-dependent default root folder path
fn get_default_root_folder() -> PathBuf {
    if cfg!(target_os = "linux") {
        // ~/.local/share/wkmp (or /var/lib/wkmp for system-wide)
        dirs::data_local_dir()
            .map(|d| d.join("wkmp"))
            .unwrap_or_else(|| PathBuf::from("/var/lib/wkmp"))
    } else if cfg!(target_os = "macos") {
        // ~/Library/Application Support/wkmp
        dirs::data_dir()
            .map(|d| d.join("wkmp"))
            .unwrap_or_else(|| PathBuf::from("/Library/Application Support/wkmp"))
    } else if cfg!(target_os = "windows") {
        // %LOCALAPPDATA%\wkmp
        dirs::data_local_dir()
            .map(|d| d.join("wkmp"))
            .unwrap_or_else(|| PathBuf::from("C:\\ProgramData\\wkmp"))
    } else {
        PathBuf::from("./wkmp_data")
    }
}

/// Load module configuration from database
pub async fn load_module_config(
    db: &sqlx::SqlitePool,
    module_name: &str,
) -> Result<ModuleConfig> {
    let record = sqlx::query_as::<_, (String, String, i64, i64)>(
        "SELECT module_name, host, port, enabled FROM module_config WHERE module_name = ?"
    )
    .bind(module_name)
    .fetch_one(db)
    .await?;

    Ok(ModuleConfig {
        module_name: record.0,
        host: record.1,
        port: record.2 as u16,
        enabled: record.3 != 0,
    })
}
