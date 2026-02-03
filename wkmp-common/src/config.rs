//! Configuration loading and root folder resolution
//!
//! Implements graceful degradation for configuration as specified in:
//! - [REQ-NF-031] through [REQ-NF-037] - Zero-config startup with compiled defaults
//! - [ARCH-INIT-005] through [ARCH-INIT-020] - Module initialization sequence
//! - [DEP-CFG-031] through [DEP-CFG-040] - Deployment configuration
//!
//! # MANDATORY USAGE - ALL MODULES
//!
//! **[REQ-NF-037]** Per requirements, ALL WKMP modules MUST use this module's utilities:
//!
//! - **RootFolderResolver** - REQUIRED for all modules to resolve root folder path
//! - **RootFolderInitializer** - REQUIRED for all modules to initialize directories
//!
//! NO module may:
//! - Hardcode database paths (e.g., `PathBuf::from("wkmp.db")`)
//! - Implement custom root folder resolution logic
//! - Skip the 4-tier priority system (CLI → ENV → TOML → Defaults)
//!
//! This ensures consistent zero-configuration behavior across all 5 modules:
//! wkmp-ui, wkmp-ap, wkmp-pd, wkmp-ai, wkmp-le
//!
//! # Example Usage (REQUIRED PATTERN)
//!
//! ```rust
//! use wkmp_common::config::{RootFolderResolver, RootFolderInitializer};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Step 1: Resolve root folder (4-tier priority)
//! let resolver = RootFolderResolver::new("module-name");
//! let root_folder = resolver.resolve();
//!
//! // Step 2: Create directory if missing
//! let initializer = RootFolderInitializer::new(root_folder);
//! initializer.ensure_directory_exists()?;
//!
//! // Step 3: Get database path
//! let db_path = initializer.database_path();  // root_folder/wkmp.db
//! # Ok(())
//! # }
//! ```

use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

/// Module configuration from database
#[derive(Debug, Clone)]
pub struct ModuleConfig {
    /// Module name (e.g., "wkmp-ap", "wkmp-ui", "wkmp-pd")
    pub module_name: String,
    /// Host address (e.g., "127.0.0.1", "localhost")
    pub host: String,
    /// Port number (e.g., 5720, 5721, 5722)
    pub port: u16,
    /// Whether the module is enabled
    pub enabled: bool,
}

/// TOML configuration file structure
#[derive(Debug, Deserialize, Serialize)]
pub struct TomlConfig {
    /// Root folder path (optional - will use default if missing)
    pub root_folder: Option<PathBuf>,

    /// Logging configuration
    #[serde(default)]
    pub logging: LoggingConfig,

    /// Static assets path (optional)
    pub static_assets: Option<PathBuf>,

    /// AcoustID API key for audio fingerprinting (optional)
    /// Used by: wkmp-ai (Audio Ingest) only
    pub acoustid_api_key: Option<String>,
}

/// Logging configuration section
#[derive(Debug, Deserialize, Serialize, Default)]
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
        if let Some(path) = self.try_cli_args() {
            info!("Root folder: {} (from command-line argument)", path.display());
            return path;
        }

        // Priority 2: Environment variable
        if let Some(path) = self.try_env_var() {
            info!("Root folder: {} (from environment variable)", path.display());
            return path;
        }

        // Priority 3: TOML config file
        match self.try_toml_file() {
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
    fn try_cli_args(&self) -> Option<PathBuf> {
        let args: Vec<String> = std::env::args().collect();

        for i in 0..args.len() {
            // Check for --root-folder <path> or --root <path>
            if (args[i] == "--root-folder" || args[i] == "--root")
                && i + 1 < args.len() {
                    return Some(PathBuf::from(&args[i + 1]));
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
    fn try_env_var(&self) -> Option<PathBuf> {
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
    fn try_toml_file(&self) -> Result<Option<PathBuf>> {
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

// ============================================================================
// TOML Atomic Write Utilities
// ============================================================================

/// Write TOML config to file atomically with permissions
///
/// **Traceability:** [APIK-ATOMIC-010], [APIK-SEC-010]
///
/// Atomic write steps:
/// 1. Serialize config to TOML string
/// 2. Write to temporary file (.toml.tmp)
/// 3. Set Unix permissions 0600 (if Unix)
/// 4. Rename temp file to target (atomic)
///
/// **Returns:** Ok(()) on success, Err on any failure
pub fn write_toml_config(
    config: &TomlConfig,
    target_path: &Path,
) -> Result<()> {
    // Step 0: Ensure parent directory exists **[REQ-NF-038]**
    if let Some(parent) = target_path.parent() {
        if !parent.exists() {
            tracing::info!("Creating config directory: {}", parent.display());
            fs::create_dir_all(parent)
                .map_err(|e| Error::Config(format!("Directory creation failed: {}", e)))?;

            // Set secure permissions on directory (Unix only: 0700 user-only)
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let perms = fs::Permissions::from_mode(0o700);
                fs::set_permissions(parent, perms)
                    .map_err(|e| Error::Config(format!("Directory permissions failed: {}", e)))?;
            }
        }
    }

    // Step 1: Serialize to TOML
    let toml_string = toml::to_string_pretty(config)
        .map_err(|e| Error::Config(format!("TOML serialization failed: {}", e)))?;

    // Step 2: Create temp file
    let temp_path = target_path.with_extension("toml.tmp");
    let mut temp_file = fs::File::create(&temp_path)
        .map_err(|e| Error::Config(format!("Temp file create failed: {}", e)))?;

    temp_file.write_all(toml_string.as_bytes())
        .map_err(|e| Error::Config(format!("Temp file write failed: {}", e)))?;

    // Ensure data is flushed to disk before rename
    temp_file.sync_all()
        .map_err(|e| Error::Config(format!("Temp file sync failed: {}", e)))?;

    drop(temp_file); // Close file before setting permissions

    // Step 3: Set permissions (Unix only)
    set_unix_permissions_0600(&temp_path)?;

    // Step 4: Atomic rename
    fs::rename(&temp_path, target_path)
        .map_err(|e| Error::Config(format!("Atomic rename failed: {}", e)))?;

    Ok(())
}

/// Set Unix file permissions to 0600 (rw-------)
///
/// **Traceability:** [APIK-SEC-010], [APIK-SEC-020]
///
/// **Unix:** Sets permissions to 0600 (owner read/write only)
/// **Windows:** No-op (returns Ok, relies on NTFS default permissions)
#[cfg(unix)]
pub fn set_unix_permissions_0600(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut perms = fs::metadata(path)
        .map_err(|e| Error::Config(format!("Get metadata failed: {}", e)))?
        .permissions();

    perms.set_mode(0o600);

    fs::set_permissions(path, perms)
        .map_err(|e| Error::Config(format!("Set permissions failed: {}", e)))?;

    Ok(())
}

#[cfg(not(unix))]
pub fn set_unix_permissions_0600(_path: &Path) -> Result<()> {
    // Windows: No-op (best-effort approach)
    // NTFS default permissions (user-only access) are acceptable
    Ok(())
}

/// Check if TOML file has loose permissions (Unix only)
///
/// **Traceability:** [APIK-SEC-040]
///
/// **Returns:** true if permissions are looser than 0600 (world/group readable)
#[cfg(unix)]
pub fn check_toml_permissions_loose(path: &Path) -> Result<bool> {
    use std::os::unix::fs::PermissionsExt;

    if !path.exists() {
        return Ok(false); // File doesn't exist, no permission issue
    }

    let metadata = fs::metadata(path)
        .map_err(|e| Error::Config(format!("Get metadata failed: {}", e)))?;

    let mode = metadata.permissions().mode();

    // Loose if group or others have any access (bits 077)
    Ok((mode & 0o077) != 0)
}

#[cfg(not(unix))]
pub fn check_toml_permissions_loose(_path: &Path) -> Result<bool> {
    // Windows: Cannot reliably check NTFS ACLs, return false
    Ok(false)
}
