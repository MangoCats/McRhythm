//! Configuration loading and root folder resolution
//!
//! Implements graceful degradation for configuration as specified in:
//! - [REQ-NF-031] through [REQ-NF-036]
//! - [ARCH-INIT-005] through [ARCH-INIT-020]
//! - [DEP-CFG-031] through [DEP-CFG-040]

use crate::{Error, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// Module configuration from database
#[derive(Debug, Clone)]
pub struct ModuleConfig {
    pub module_name: String,
    pub host: String,
    pub port: u16,
    pub enabled: bool,
}

/// TOML configuration file structure
#[derive(Debug, Deserialize)]
pub struct TomlConfig {
    /// Root folder path (optional - will use default if missing)
    pub root_folder: Option<PathBuf>,

    /// Logging configuration
    #[serde(default)]
    pub logging: LoggingConfig,

    /// Static assets path (optional)
    pub static_assets: Option<PathBuf>,
}

/// Logging configuration section
#[derive(Debug, Deserialize, Default)]
pub struct LoggingConfig {
    /// Log level (default: "info")
    #[serde(default = "default_log_level")]
    pub level: String,

    /// Log file path (None = stdout only)
    pub log_file: Option<PathBuf>,
}

fn default_log_level() -> String {
    "info".to_string()
}

/// Compiled default configuration values [REQ-NF-033, REQ-NF-034, DEP-CFG-040]
#[derive(Debug, Clone)]
pub struct CompiledDefaults {
    /// Default root folder path (platform-specific)
    pub root_folder: PathBuf,

    /// Default logging level
    pub log_level: String,

    /// Default log file path (None = stdout only)
    pub log_file: Option<PathBuf>,

    /// Default static assets path (platform-specific)
    pub static_assets_path: PathBuf,
}

impl CompiledDefaults {
    /// Get compiled defaults for the current platform [REQ-NF-033, REQ-NF-034]
    pub fn for_current_platform() -> Self {
        #[cfg(target_os = "linux")]
        return Self::linux();

        #[cfg(target_os = "macos")]
        return Self::macos();

        #[cfg(target_os = "windows")]
        return Self::windows();

        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        return Self::fallback();
    }

    #[cfg(target_os = "linux")]
    fn linux() -> Self {
        let home = std::env::var("HOME")
            .unwrap_or_else(|_| "/home/user".to_string());

        Self {
            root_folder: PathBuf::from(home).join("Music"),
            log_level: "info".to_string(),
            log_file: None,
            static_assets_path: PathBuf::from("/usr/local/share/wkmp"),
        }
    }

    #[cfg(target_os = "macos")]
    fn macos() -> Self {
        let home = std::env::var("HOME")
            .unwrap_or_else(|_| "/Users/user".to_string());

        Self {
            root_folder: PathBuf::from(home).join("Music"),
            log_level: "info".to_string(),
            log_file: None,
            static_assets_path: PathBuf::from("/Applications/WKMP.app/Contents/Resources"),
        }
    }

    #[cfg(target_os = "windows")]
    fn windows() -> Self {
        let userprofile = std::env::var("USERPROFILE")
            .unwrap_or_else(|_| "C:\\Users\\user".to_string());

        // [REQ-NF-033] Windows default: %USERPROFILE%\Music (amended - removed \wkmp subfolder)
        Self {
            root_folder: PathBuf::from(userprofile).join("Music"),
            log_level: "info".to_string(),
            log_file: None,
            static_assets_path: PathBuf::from("C:\\Program Files\\WKMP\\share"),
        }
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    fn fallback() -> Self {
        Self {
            root_folder: PathBuf::from("./wkmp_data"),
            log_level: "info".to_string(),
            log_file: None,
            static_assets_path: PathBuf::from("./wkmp_assets"),
        }
    }
}

/// Get the default root folder for the current platform [REQ-NF-033]
///
/// Returns platform-appropriate default locations for music files:
/// - **Linux**: `~/Music`
/// - **macOS**: `~/Music`
/// - **Windows**: `%USERPROFILE%\Music`
/// - **Other**: `./wkmp_data`
pub fn get_default_root_folder() -> PathBuf {
    CompiledDefaults::for_current_platform().root_folder
}

/// Root folder resolver following ARCH-INIT-005 priority order
pub struct RootFolderResolver {
    module_name: String,
}

impl RootFolderResolver {
    /// Create a new resolver for the specified module
    pub fn new(module_name: impl Into<String>) -> Self {
        Self {
            module_name: module_name.into(),
        }
    }

    /// Resolve root folder using 4-tier priority [REQ-NF-035, ARCH-INIT-005]
    ///
    /// Priority order:
    /// 1. Command-line argument (--root-folder or --root)
    /// 2. Environment variable (WKMP_ROOT_FOLDER or WKMP_ROOT)
    /// 3. TOML configuration file
    /// 4. Compiled default (platform-specific)
    pub fn resolve(&self) -> PathBuf {
        // Priority 1: Command-line argument
        if let Some(path) = self.from_cli_args() {
            info!("Root folder: {} (from command-line argument)", path.display());
            return path;
        }

        // Priority 2: Environment variable
        if let Some(path) = self.from_env_var() {
            info!("Root folder: {} (from environment variable)", path.display());
            return path;
        }

        // Priority 3: TOML config file
        match self.from_toml_file() {
            Ok(Some(path)) => {
                info!("Root folder: {} (from config file)", path.display());
                return path;
            }
            Ok(None) => {
                // Config file missing - this is expected and OK [REQ-NF-031]
                warn!(
                    "Config file not found at {}, using default configuration",
                    self.config_file_path().display()
                );
            }
            Err(e) => {
                // Config file exists but is corrupted - this IS an error
                warn!(
                    "Failed to load config file at {}: {}. Using default configuration.",
                    self.config_file_path().display(),
                    e
                );
            }
        }

        // Priority 4: Compiled default [REQ-NF-032]
        let defaults = CompiledDefaults::for_current_platform();
        info!("Root folder: {} (compiled default)", defaults.root_folder.display());
        defaults.root_folder
    }

    /// Try to get root folder from --root-folder or --root CLI argument
    fn from_cli_args(&self) -> Option<PathBuf> {
        let args: Vec<String> = std::env::args().collect();

        for i in 0..args.len() {
            // Check for --root-folder <path> or --root <path>
            if args[i] == "--root-folder" || args[i] == "--root" {
                if i + 1 < args.len() {
                    return Some(PathBuf::from(&args[i + 1]));
                }
            }

            // Check for --root-folder=<path> or --root=<path>
            if let Some(path) = args[i].strip_prefix("--root-folder=") {
                return Some(PathBuf::from(path));
            }
            if let Some(path) = args[i].strip_prefix("--root=") {
                return Some(PathBuf::from(path));
            }
        }

        None
    }

    /// Try to get root folder from WKMP_ROOT_FOLDER or WKMP_ROOT env var
    fn from_env_var(&self) -> Option<PathBuf> {
        if let Ok(path) = std::env::var("WKMP_ROOT_FOLDER") {
            return Some(PathBuf::from(path));
        }

        if let Ok(path) = std::env::var("WKMP_ROOT") {
            return Some(PathBuf::from(path));
        }

        None
    }

    /// Try to get root folder from TOML config file
    /// Returns None if file doesn't exist (not an error per [REQ-NF-031])
    fn from_toml_file(&self) -> Result<Option<PathBuf>> {
        let config_path = self.config_file_path();

        if !config_path.exists() {
            return Ok(None);  // Missing file is OK [REQ-NF-031]
        }

        let toml_config = self.load_toml_file(&config_path)?;
        Ok(toml_config.root_folder)
    }

    /// Load TOML config from file [DEP-CFG-031]
    /// Returns error only for corrupted files, not missing files
    fn load_toml_file(&self, path: &Path) -> Result<TomlConfig> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| Error::Config(format!("Failed to read {}: {}", path.display(), e)))?;

        let config: TomlConfig = toml::from_str(&contents)
            .map_err(|e| Error::Config(format!("Failed to parse {}: {}", path.display(), e)))?;

        Ok(config)
    }

    /// Get platform-specific config file path [DEP-CFG-031]
    /// Format: ~/.config/wkmp/<module-name>.toml
    fn config_file_path(&self) -> PathBuf {
        #[cfg(target_os = "linux")]
        {
            let home = std::env::var("HOME")
                .unwrap_or_else(|_| "/home/user".to_string());
            PathBuf::from(home)
                .join(".config/wkmp")
                .join(format!("{}.toml", self.module_name))
        }

        #[cfg(target_os = "macos")]
        {
            let home = std::env::var("HOME")
                .unwrap_or_else(|_| "/Users/user".to_string());
            PathBuf::from(home)
                .join("Library/Application Support/WKMP")
                .join(format!("{}.toml", self.module_name))
        }

        #[cfg(target_os = "windows")]
        {
            let appdata = std::env::var("APPDATA")
                .unwrap_or_else(|_| "C:\\Users\\user\\AppData\\Roaming".to_string());
            PathBuf::from(appdata)
                .join("WKMP")
                .join(format!("{}.toml", self.module_name))
        }

        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            PathBuf::from(".").join(format!("{}.toml", self.module_name))
        }
    }
}

/// Root folder initializer [REQ-NF-036, ARCH-INIT-010]
pub struct RootFolderInitializer {
    root_folder: PathBuf,
}

impl RootFolderInitializer {
    /// Create a new initializer for the specified root folder
    pub fn new(root_folder: PathBuf) -> Self {
        Self { root_folder }
    }

    /// Create root folder directory if it doesn't exist [REQ-NF-036]
    pub fn ensure_directory_exists(&self) -> Result<()> {
        if !self.root_folder.exists() {
            info!("Creating root folder directory: {}", self.root_folder.display());
            std::fs::create_dir_all(&self.root_folder)
                .map_err(|e| Error::Config(format!(
                    "Failed to create directory {}: {}",
                    self.root_folder.display(),
                    e
                )))?;
            info!("Root folder directory created successfully");
        } else {
            info!("Root folder directory exists: {}", self.root_folder.display());
        }

        Ok(())
    }

    /// Get database file path
    pub fn database_path(&self) -> PathBuf {
        self.root_folder.join("wkmp.db")
    }

    /// Check if database exists
    pub fn database_exists(&self) -> bool {
        self.database_path().exists()
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
