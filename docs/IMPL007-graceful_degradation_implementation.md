# Implementation Plan: Graceful Degradation for Configuration and Startup

**🛠️ TIER 3 - IMPLEMENTATION SPECIFICATION**

This document provides a detailed implementation plan for graceful degradation features specified in [REQ-NF-030] through [REQ-NF-036], [ARCH-INIT-005] through [ARCH-INIT-020], and [DEP-CFG-031] through [DEP-CFG-040].

> **Related Documentation:** [Requirements](REQ001-requirements.md) | [Architecture](SPEC001-architecture.md) | [Deployment](IMPL004-deployment.md) | [Database Schema](IMPL001-database_schema.md)

---

## Overview

### Purpose

Implement graceful degradation throughout WKMP's microservices architecture to enable zero-configuration startup, eliminate fatal errors from missing configuration files, and provide a seamless first-run experience.

### Goals

**[IMPL-GD-010]** Enable all modules to start successfully with no configuration files present:
- No TOML config files required
- No pre-existing database required
- No pre-existing root folder required
- Automatic creation of all necessary resources

**[IMPL-GD-020]** Provide clear, actionable feedback when using defaults:
- Log warnings (not errors) for missing config files
- Log informational messages for automatic initialization
- Include file paths in all log messages

**[IMPL-GD-030]** Maintain consistent behavior across all 5 modules:
- wkmp-ui (User Interface)
- wkmp-ap (Audio Player)
- wkmp-pd (Program Director)
- wkmp-ai (Audio Ingest)
- wkmp-le (Lyric Editor)

---

## Specification References

### Requirements

- **[REQ-NF-031]**: Missing TOML files SHALL NOT cause module termination
- **[REQ-NF-032]**: Missing config → warning log + compiled defaults + successful startup
- **[REQ-NF-033]**: Default root folder locations per platform
- **[REQ-NF-034]**: Default values for logging, static assets
- **[REQ-NF-035]**: Priority order for root folder resolution
- **[REQ-NF-036]**: Automatic directory/database creation on first run

### Architecture

- **[ARCH-INIT-005]**: Root folder location resolution algorithm
- **[ARCH-INIT-010]**: Module startup sequence
- **[ARCH-INIT-015]**: Missing configuration handling
- **[ARCH-INIT-020]**: Default value initialization behavior (database)

### Deployment

- **[DEP-CFG-031]**: Graceful degradation behavior specification
- **[DEP-CFG-035]**: Module discovery via database
- **[DEP-CFG-040]**: Compiled default configuration values

---

## Implementation Phases

### Phase 1: Shared Configuration Library (wkmp-common)

**Duration:** 1 week
**Dependencies:** None
**Deliverables:** Reusable configuration loading module

#### 1.1. Create Config Module Structure

**File:** `wkmp-common/src/config/mod.rs`

**Purpose:** Centralize all configuration resolution logic in a single, well-tested module that all microservices can use.

**Modules:**
```rust
pub mod defaults;      // Platform-specific compiled defaults
pub mod resolution;    // Priority-based config resolution
pub mod toml_loader;   // TOML file loading with fallback
pub mod validation;    // Config value validation
```

#### 1.2. Implement Platform-Specific Defaults

**File:** `wkmp-common/src/config/defaults.rs`

**Requirements:** [REQ-NF-033], [REQ-NF-034], [DEP-CFG-040]

```rust
/// Compiled default configuration values for all modules
pub struct CompiledDefaults {
    /// Default root folder path (platform-specific)
    pub root_folder: PathBuf,

    /// Default logging level
    pub log_level: String,  // "info"

    /// Default log file path (empty = stdout only)
    pub log_file: Option<PathBuf>,  // None

    /// Default static assets path (platform-specific)
    pub static_assets_path: PathBuf,
}

impl CompiledDefaults {
    /// Get compiled defaults for the current platform
    pub fn for_current_platform() -> Self {
        #[cfg(target_os = "linux")]
        return Self::linux();

        #[cfg(target_os = "macos")]
        return Self::macos();

        #[cfg(target_os = "windows")]
        return Self::windows();
    }

    fn linux() -> Self {
        let home = env::var("HOME")
            .expect("HOME environment variable not set");

        Self {
            root_folder: PathBuf::from(home).join("Music"),
            log_level: "info".to_string(),
            log_file: None,
            static_assets_path: PathBuf::from("/usr/local/share/wkmp"),
        }
    }

    fn macos() -> Self {
        let home = env::var("HOME")
            .expect("HOME environment variable not set");

        Self {
            root_folder: PathBuf::from(home).join("Music"),
            log_level: "info".to_string(),
            log_file: None,
            static_assets_path: PathBuf::from("/Applications/WKMP.app/Contents/Resources"),
        }
    }

    fn windows() -> Self {
        let userprofile = env::var("USERPROFILE")
            .expect("USERPROFILE environment variable not set");

        Self {
            root_folder: PathBuf::from(userprofile).join("Music").join("wkmp"),
            log_level: "info".to_string(),
            log_file: None,
            static_assets_path: PathBuf::from("C:\\Program Files\\WKMP\\share"),
        }
    }
}
```

**Testing:**
- Unit tests for each platform's defaults
- Verify HOME/USERPROFILE fallback behavior
- Verify path construction on each OS

#### 1.3. Implement Root Folder Resolution

**File:** `wkmp-common/src/config/resolution.rs`

**Requirements:** [REQ-NF-035], [ARCH-INIT-005]

```rust
use std::path::PathBuf;
use std::env;
use super::defaults::CompiledDefaults;
use super::toml_loader::TomlConfig;

pub struct RootFolderResolver {
    /// Module name (for log messages and config file paths)
    module_name: String,
}

impl RootFolderResolver {
    pub fn new(module_name: &str) -> Self {
        Self {
            module_name: module_name.to_string(),
        }
    }

    /// Resolve root folder using 4-tier priority [REQ-NF-035]
    pub fn resolve(&self) -> Result<PathBuf, ConfigError> {
        // Priority 1: Command-line argument
        if let Some(path) = self.from_cli_args()? {
            log::info!("Root folder: {} (from command-line argument)", path.display());
            return Ok(path);
        }

        // Priority 2: Environment variable
        if let Some(path) = self.from_env_var()? {
            log::info!("Root folder: {} (from environment variable)", path.display());
            return Ok(path);
        }

        // Priority 3: TOML config file
        match self.from_toml_file()? {
            Some(path) => {
                log::info!("Root folder: {} (from config file)", path.display());
                return Ok(path);
            }
            None => {
                // Config file missing - this is expected and OK [REQ-NF-031]
                log::warn!(
                    "Config file not found at {}, using default configuration",
                    self.config_file_path().display()
                );
            }
        }

        // Priority 4: Compiled default [REQ-NF-032]
        let defaults = CompiledDefaults::for_current_platform();
        log::info!("Root folder: {} (compiled default)", defaults.root_folder.display());
        Ok(defaults.root_folder)
    }

    /// Try to get root folder from --root-folder or --root CLI argument
    fn from_cli_args(&self) -> Result<Option<PathBuf>, ConfigError> {
        let args: Vec<String> = env::args().collect();

        for i in 0..args.len() {
            if args[i] == "--root-folder" || args[i] == "--root" {
                if i + 1 < args.len() {
                    return Ok(Some(PathBuf::from(&args[i + 1])));
                } else {
                    return Err(ConfigError::InvalidArgument(
                        format!("{} requires a path argument", args[i])
                    ));
                }
            }

            // Also support --root-folder=/path/to/folder syntax
            if let Some(path) = args[i].strip_prefix("--root-folder=") {
                return Ok(Some(PathBuf::from(path)));
            }
            if let Some(path) = args[i].strip_prefix("--root=") {
                return Ok(Some(PathBuf::from(path)));
            }
        }

        Ok(None)
    }

    /// Try to get root folder from WKMP_ROOT_FOLDER or WKMP_ROOT env var
    fn from_env_var(&self) -> Result<Option<PathBuf>, ConfigError> {
        if let Ok(path) = env::var("WKMP_ROOT_FOLDER") {
            return Ok(Some(PathBuf::from(path)));
        }

        if let Ok(path) = env::var("WKMP_ROOT") {
            return Ok(Some(PathBuf::from(path)));
        }

        Ok(None)
    }

    /// Try to get root folder from TOML config file
    /// Returns None if file doesn't exist (not an error per [REQ-NF-031])
    fn from_toml_file(&self) -> Result<Option<PathBuf>, ConfigError> {
        let config_path = self.config_file_path();

        if !config_path.exists() {
            return Ok(None);  // Missing file is OK [REQ-NF-031]
        }

        let toml_config = TomlConfig::load(&config_path)?;
        Ok(toml_config.root_folder)
    }

    /// Get platform-specific config file path
    fn config_file_path(&self) -> PathBuf {
        #[cfg(target_os = "linux")]
        {
            let home = env::var("HOME").expect("HOME not set");
            PathBuf::from(home)
                .join(".config/wkmp")
                .join(format!("{}.toml", self.module_name))
        }

        #[cfg(target_os = "macos")]
        {
            let home = env::var("HOME").expect("HOME not set");
            PathBuf::from(home)
                .join("Library/Application Support/WKMP")
                .join(format!("{}.toml", self.module_name))
        }

        #[cfg(target_os = "windows")]
        {
            let appdata = env::var("APPDATA").expect("APPDATA not set");
            PathBuf::from(appdata)
                .join("WKMP")
                .join(format!("{}.toml", self.module_name))
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Invalid command-line argument: {0}")]
    InvalidArgument(String),

    #[error("Failed to load TOML config: {0}")]
    TomlLoadError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
```

**Testing:**
- Unit tests for each priority level
- Integration tests with various combinations
- Test missing config file (should not error)
- Test invalid CLI args (should error)

#### 1.4. Implement TOML Config Loader

**File:** `wkmp-common/src/config/toml_loader.rs`

**Requirements:** [REQ-NF-031], [REQ-NF-032]

```rust
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::fs;

#[derive(Debug, Deserialize)]
pub struct TomlConfig {
    pub root_folder: Option<PathBuf>,
    #[serde(default)]
    pub logging: LoggingConfig,
    #[serde(default)]
    pub static_assets: Option<PathBuf>,
}

#[derive(Debug, Deserialize, Default)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    pub log_file: Option<PathBuf>,
}

fn default_log_level() -> String {
    "info".to_string()
}

impl TomlConfig {
    /// Load TOML config from file
    /// Returns error only for corrupted files, not missing files [REQ-NF-031]
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let contents = fs::read_to_string(path)
            .map_err(|e| ConfigError::TomlLoadError(format!("Failed to read {}: {}", path.display(), e)))?;

        let config: TomlConfig = toml::from_str(&contents)
            .map_err(|e| ConfigError::TomlLoadError(format!("Failed to parse {}: {}", path.display(), e)))?;

        Ok(config)
    }
}
```

**Testing:**
- Test valid TOML parsing
- Test corrupted TOML (should error)
- Test missing file (caller handles this)
- Test partial configs (missing optional fields)

#### 1.5. Implement Root Folder Initialization

**File:** `wkmp-common/src/config/initialization.rs`

**Requirements:** [REQ-NF-036], [ARCH-INIT-010]

```rust
use std::path::{Path, PathBuf};
use std::fs;

/// Initialize root folder and database [REQ-NF-036]
pub struct RootFolderInitializer {
    root_folder: PathBuf,
}

impl RootFolderInitializer {
    pub fn new(root_folder: PathBuf) -> Self {
        Self { root_folder }
    }

    /// Create root folder directory if it doesn't exist [REQ-NF-036]
    pub fn ensure_directory_exists(&self) -> Result<(), InitError> {
        if !self.root_folder.exists() {
            log::info!("Creating root folder directory: {}", self.root_folder.display());
            fs::create_dir_all(&self.root_folder)
                .map_err(|e| InitError::DirectoryCreationFailed(
                    self.root_folder.clone(),
                    e
                ))?;
            log::info!("Root folder directory created successfully");
        } else {
            log::debug!("Root folder directory exists: {}", self.root_folder.display());
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

#[derive(Debug, thiserror::Error)]
pub enum InitError {
    #[error("Failed to create directory {0}: {1}")]
    DirectoryCreationFailed(PathBuf, std::io::Error),

    #[error("Failed to initialize database: {0}")]
    DatabaseInitFailed(String),
}
```

**Testing:**
- Test directory creation when missing
- Test directory already exists (no-op)
- Test permission errors
- Test database path construction

---

### Phase 2: Database Initialization (wkmp-common)

**Duration:** 1 week
**Dependencies:** Phase 1
**Deliverables:** Shared database initialization functions

#### 2.1. Create Database Initialization Module

**File:** `wkmp-common/src/db/init.rs`

**Requirements:** [REQ-NF-036], [ARCH-INIT-010], [ARCH-INIT-020]

```rust
use rusqlite::{Connection, Result as SqliteResult};
use std::path::Path;

/// Initialize database with idempotent table creation [REQ-NF-036]
pub struct DatabaseInitializer<'a> {
    conn: &'a Connection,
}

impl<'a> DatabaseInitializer<'a> {
    pub fn new(conn: &'a Connection) -> Self {
        Self { conn }
    }

    /// Initialize all common tables shared by all modules
    /// Safe to call multiple times (idempotent) [ARCH-INIT-010]
    pub fn init_common_tables(&self) -> SqliteResult<()> {
        self.init_settings_table()?;
        self.init_module_config_table()?;
        self.init_users_table()?;
        Ok(())
    }

    /// Initialize settings table with default values [ARCH-INIT-020]
    fn init_settings_table(&self) -> SqliteResult<()> {
        // Create table if not exists
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT
            )",
            [],
        )?;

        // Insert default values if missing [ARCH-INIT-020]
        self.ensure_setting("volume_level", "0.5")?;
        self.ensure_setting("initial_play_state", "playing")?;
        self.ensure_setting("global_crossfade_time", "2.0")?;
        self.ensure_setting("queue_max_size", "100")?;
        self.ensure_setting("session_timeout_seconds", "31536000")?;
        self.ensure_setting("backup_interval_ms", "7776000000")?;
        self.ensure_setting("backup_minimum_interval_ms", "1209600000")?;
        self.ensure_setting("backup_retention_count", "3")?;
        self.ensure_setting("backup_location", "")?;  // Empty = same folder as db
        self.ensure_setting("http_base_ports", "[5720, 15720, 25720, 17200, 23400]")?;
        self.ensure_setting("http_request_timeout_ms", "30000")?;
        self.ensure_setting("http_keepalive_timeout_ms", "60000")?;
        self.ensure_setting("http_max_body_size_bytes", "1048576")?;

        log::info!("Settings table initialized with default values");
        Ok(())
    }

    /// Ensure a setting exists, insert default if missing [ARCH-INIT-020]
    fn ensure_setting(&self, key: &str, default_value: &str) -> SqliteResult<()> {
        let exists: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM settings WHERE key = ?1)",
            [key],
            |row| row.get(0),
        )?;

        if !exists {
            self.conn.execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)",
                [key, default_value],
            )?;
            log::info!("Initialized setting '{}' with default value: {}", key, default_value);
        }

        // Handle NULL values [ARCH-INIT-020]
        let is_null: bool = self.conn.query_row(
            "SELECT value IS NULL FROM settings WHERE key = ?1",
            [key],
            |row| row.get(0),
        )?;

        if is_null {
            self.conn.execute(
                "UPDATE settings SET value = ?2 WHERE key = ?1",
                [key, default_value],
            )?;
            log::warn!("Setting '{}' was NULL, reset to default: {}", key, default_value);
        }

        Ok(())
    }

    /// Initialize module_config table [DEP-CFG-035]
    fn init_module_config_table(&self) -> SqliteResult<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS module_config (
                module_name TEXT PRIMARY KEY,
                host TEXT NOT NULL,
                port INTEGER NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1
            )",
            [],
        )?;

        // Insert default module configurations if missing
        self.ensure_module_config("user_interface", "0.0.0.0", 5720)?;
        self.ensure_module_config("audio_player", "127.0.0.1", 5721)?;
        self.ensure_module_config("program_director", "127.0.0.1", 5722)?;
        self.ensure_module_config("audio_ingest", "0.0.0.0", 5723)?;
        self.ensure_module_config("lyric_editor", "0.0.0.0", 5724)?;

        log::info!("Module config table initialized with default values");
        Ok(())
    }

    /// Ensure a module config exists, insert default if missing
    fn ensure_module_config(&self, module_name: &str, default_host: &str, default_port: i32) -> SqliteResult<()> {
        let exists: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM module_config WHERE module_name = ?1)",
            [module_name],
            |row| row.get(0),
        )?;

        if !exists {
            self.conn.execute(
                "INSERT INTO module_config (module_name, host, port, enabled) VALUES (?1, ?2, ?3, 1)",
                rusqlite::params![module_name, default_host, default_port],
            )?;
            log::info!("Initialized module config '{}' -> {}:{}", module_name, default_host, default_port);
        }

        Ok(())
    }

    /// Initialize users table with Anonymous user [ARCH-INIT-010]
    fn init_users_table(&self) -> SqliteResult<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS users (
                user_id TEXT PRIMARY KEY,
                username TEXT UNIQUE NOT NULL,
                password_hash TEXT NOT NULL,
                password_salt TEXT NOT NULL,
                config_interface_access INTEGER NOT NULL DEFAULT 1
            )",
            [],
        )?;

        // Insert Anonymous user if not exists
        let anonymous_exists: bool = self.conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM users WHERE username = 'Anonymous')",
            [],
            |row| row.get(0),
        )?;

        if !anonymous_exists {
            let anonymous_uuid = uuid::Uuid::new_v4().to_string();
            self.conn.execute(
                "INSERT INTO users (user_id, username, password_hash, password_salt, config_interface_access)
                 VALUES (?1, 'Anonymous', '', '', 1)",
                [&anonymous_uuid],
            )?;
            log::info!("Created Anonymous user with UUID: {}", anonymous_uuid);
        }

        Ok(())
    }
}

/// Open or create database at specified path [REQ-NF-036]
pub fn open_or_create_database(db_path: &Path) -> SqliteResult<Connection> {
    let newly_created = !db_path.exists();

    let conn = Connection::open(db_path)?;

    if newly_created {
        log::info!("Initialized new database: {}", db_path.display());
    } else {
        log::debug!("Opened existing database: {}", db_path.display());
    }

    // Enable foreign keys
    conn.execute("PRAGMA foreign_keys = ON", [])?;

    // Set busy timeout [ARCH-ERRH-070]
    conn.busy_timeout(std::time::Duration::from_millis(5000))?;

    Ok(conn)
}
```

**Testing:**
- Test database creation when missing
- Test opening existing database
- Test idempotent table creation (safe to run multiple times)
- Test default value insertion
- Test NULL value handling
- Test concurrent initialization (multiple modules starting simultaneously)

---

### Phase 3: Module-Specific Implementation

**Duration:** 2 weeks (all modules in parallel)
**Dependencies:** Phase 1, Phase 2
**Deliverables:** All 5 modules implement graceful degradation

#### 3.1. Audio Player (wkmp-ap)

**File:** `wkmp-ap/src/main.rs`

**Requirements:** All graceful degradation requirements

```rust
use wkmp_common::config::{RootFolderResolver, RootFolderInitializer, CompiledDefaults};
use wkmp_common::db::{open_or_create_database, DatabaseInitializer};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Step 1: Resolve root folder [ARCH-INIT-005]
    let resolver = RootFolderResolver::new("audio-player");
    let root_folder = resolver.resolve()?;

    // Step 2: Create root folder directory if missing [REQ-NF-036]
    let initializer = RootFolderInitializer::new(root_folder);
    initializer.ensure_directory_exists()?;

    // Step 3: Open or create database [REQ-NF-036]
    let db_path = initializer.database_path();
    let conn = open_or_create_database(&db_path)?;

    // Step 4: Initialize database tables [ARCH-INIT-010]
    let db_init = DatabaseInitializer::new(&conn);
    db_init.init_common_tables()?;
    init_audio_player_tables(&conn)?;  // Module-specific tables

    // Step 5: Read module configuration from database [DEP-CFG-035]
    let module_config = read_module_config(&conn, "audio_player")?;

    // Step 6: Bind to configured port [DEP-HTTP-040]
    let server = bind_server(&module_config)?;

    // Step 7: Start accepting connections
    log::info!("Audio Player listening on {}:{}", module_config.host, module_config.port);
    server.run().await?;

    Ok(())
}

fn init_audio_player_tables(conn: &Connection) -> SqliteResult<()> {
    // Create queue table
    conn.execute(
        "CREATE TABLE IF NOT EXISTS queue (
            queue_entry_id TEXT PRIMARY KEY,
            passage_id TEXT NOT NULL,
            play_order INTEGER NOT NULL,
            enqueued_at INTEGER NOT NULL,
            FOREIGN KEY (passage_id) REFERENCES passages(passage_id)
        )",
        [],
    )?;

    log::info!("Audio Player tables initialized");
    Ok(())
}
```

**Module-specific default settings:**
- `audio_sink`: `"default"` (use cpal default output device)
- `position_event_interval_ms`: `1000`
- `playback_progress_interval_ms`: `5000`
- `queue_refill_threshold_passages`: `2`
- `queue_refill_threshold_seconds`: `900`

#### 3.2. User Interface (wkmp-ui)

Similar implementation structure:
1. Resolve root folder
2. Create directory
3. Open/create database
4. Initialize common + UI-specific tables
5. Read module config
6. Launch other modules as needed
7. Bind and start server

**Module-specific default settings:**
- `session_timeout_seconds`: `31536000` (1 year)
- `backup_interval_ms`: `7776000000` (90 days)

#### 3.3. Program Director (wkmp-pd)

Same implementation pattern.

**Module-specific default settings:**
- `queue_refill_request_throttle_seconds`: `10`
- `queue_refill_acknowledgment_timeout_seconds`: `5`
- `queue_max_enqueue_batch`: `5`

#### 3.4. Audio Ingest (wkmp-ai)

Same implementation pattern.

**Module-specific default settings:**
- `ingest_max_concurrent_jobs`: `4`

#### 3.5. Lyric Editor (wkmp-le)

Same implementation pattern.

**Module-specific default settings:** None (uses only common settings)

---

### Phase 4: Testing Strategy

**Duration:** 1 week
**Dependencies:** Phase 3
**Deliverables:** Comprehensive test coverage

#### 4.1. Unit Tests

**Test Coverage:**

**Configuration Resolution Tests** (`wkmp-common/tests/config_resolution.rs`):
```rust
#[test]
fn test_missing_config_file_uses_default() {
    // Ensure no config file exists
    // Resolve root folder
    // Assert compiled default is used
    // Assert warning logged
}

#[test]
fn test_cli_arg_overrides_all() {
    // Set env var and create config file
    // Pass CLI argument
    // Assert CLI value used
}

#[test]
fn test_env_var_overrides_toml() {
    // Create config file
    // Set env var
    // Assert env var used
}

#[test]
fn test_toml_used_when_no_overrides() {
    // Create config file
    // No CLI args or env vars
    // Assert TOML value used
}
```

**Database Initialization Tests** (`wkmp-common/tests/db_init.rs`):
```rust
#[test]
fn test_database_created_when_missing() {
    // Delete database if exists
    // Call open_or_create_database
    // Assert database created
    // Assert log message
}

#[test]
fn test_tables_created_idempotently() {
    // Create database
    // Call init_common_tables twice
    // Assert tables exist
    // Assert no errors
}

#[test]
fn test_default_settings_inserted() {
    // Create database
    // Call init_common_tables
    // Query settings table
    // Assert all defaults present
}

#[test]
fn test_null_values_reset_to_defaults() {
    // Create database
    // Insert NULL setting value
    // Call init_common_tables
    // Assert setting reset to default
    // Assert warning logged
}
```

#### 4.2. Integration Tests

**End-to-End Startup Tests** (`wkmp-ap/tests/graceful_startup.rs`):
```rust
#[test]
fn test_zero_config_startup() {
    // Clean environment (no config, no db, no root folder)
    // Start wkmp-ap
    // Assert successful startup
    // Assert root folder created
    // Assert database created
    // Assert module listening on default port
}

#[test]
fn test_startup_with_custom_root_folder() {
    // Pass --root-folder argument
    // Start wkmp-ap
    // Assert custom root folder used
    // Assert database in custom location
}

#[test]
fn test_multiple_module_concurrent_init() {
    // Clean environment
    // Start all modules concurrently
    // Assert all start successfully
    // Assert single database created
    // Assert no race conditions
}
```

#### 4.3. Manual Testing Checklist

**Test Scenarios:**

1. **Fresh Install (No Prior Configuration)**
   - [ ] Delete all TOML config files
   - [ ] Delete root folder and database
   - [ ] Start wkmp-ui
   - [ ] Verify: Modules start successfully
   - [ ] Verify: Root folder created at default location
   - [ ] Verify: Database created with default schema
   - [ ] Verify: Warning logged for missing config files
   - [ ] Verify: Info logs for directory/database creation

2. **Custom Root Folder via CLI**
   - [ ] Run `wkmp-ap --root-folder /tmp/wkmp-test`
   - [ ] Verify: Root folder created at `/tmp/wkmp-test`
   - [ ] Verify: Database at `/tmp/wkmp-test/wkmp.db`
   - [ ] Verify: Log indicates CLI argument used

3. **Custom Root Folder via Environment Variable**
   - [ ] Set `export WKMP_ROOT_FOLDER=/tmp/wkmp-env-test`
   - [ ] Run `wkmp-ap`
   - [ ] Verify: Root folder at `/tmp/wkmp-env-test`
   - [ ] Verify: Log indicates environment variable used

4. **Custom Root Folder via TOML Config**
   - [ ] Create `~/.config/wkmp/audio-player.toml` with custom root
   - [ ] Run `wkmp-ap`
   - [ ] Verify: Root folder matches TOML config
   - [ ] Verify: Log indicates config file used

5. **Corrupted TOML Config**
   - [ ] Create invalid TOML file
   - [ ] Run `wkmp-ap`
   - [ ] Verify: Module fails to start
   - [ ] Verify: Error message explains TOML parsing failure

6. **Concurrent Module Startup**
   - [ ] Clean environment
   - [ ] Start all 5 modules simultaneously
   - [ ] Verify: All modules start successfully
   - [ ] Verify: Single database created
   - [ ] Verify: No database corruption
   - [ ] Verify: All modules' configs in module_config table

---

### Phase 5: Documentation Updates

**Duration:** 2 days
**Dependencies:** Phase 4
**Deliverables:** Updated user and developer documentation

#### 5.1. User Documentation

**Update:** `README.md`, `docs/USER_GUIDE.md`

**Topics:**
- Quick start with zero configuration
- How to customize root folder location
- How to use environment variables
- How to create TOML config files (optional)
- Troubleshooting startup issues

#### 5.2. Developer Documentation

**Update:** `docs/IMPL002-coding_conventions.md`

**Topics:**
- How to use wkmp-common config module
- How to add module-specific default settings
- How to add module-specific tables
- Testing requirements for new modules

---

## Implementation Checklist

### Phase 1: Shared Configuration Library
- [ ] 1.1: Create config module structure
- [ ] 1.2: Implement platform-specific defaults
- [ ] 1.3: Implement root folder resolution
- [ ] 1.4: Implement TOML config loader
- [ ] 1.5: Implement root folder initialization
- [ ] Unit tests for Phase 1

### Phase 2: Database Initialization
- [ ] 2.1: Create database initialization module
- [ ] Unit tests for Phase 2

### Phase 3: Module-Specific Implementation
- [ ] 3.1: Implement graceful degradation in wkmp-ap
- [ ] 3.2: Implement graceful degradation in wkmp-ui
- [ ] 3.3: Implement graceful degradation in wkmp-pd
- [ ] 3.4: Implement graceful degradation in wkmp-ai
- [ ] 3.5: Implement graceful degradation in wkmp-le

### Phase 4: Testing
- [ ] 4.1: Unit tests
- [ ] 4.2: Integration tests
- [ ] 4.3: Manual testing checklist completion

### Phase 5: Documentation
- [ ] 5.1: User documentation updates
- [ ] 5.2: Developer documentation updates

---

## Risk Assessment

### High-Risk Items

**[RISK-GD-010]** Database race conditions during concurrent module startup:
- **Mitigation:** SQLite busy timeout + idempotent initialization
- **Testing:** Concurrent startup integration tests

**[RISK-GD-020]** Platform-specific path construction errors:
- **Mitigation:** Comprehensive unit tests for each platform
- **Testing:** Test on Linux, macOS, Windows

**[RISK-GD-030]** Backward compatibility with existing configurations:
- **Mitigation:** Graceful handling of existing TOML files
- **Testing:** Test with pre-existing valid configs

### Medium-Risk Items

**[RISK-GD-040]** Log message consistency across modules:
- **Mitigation:** Centralized logging utilities in wkmp-common
- **Testing:** Manual review of all log output

**[RISK-GD-050]** NULL value handling in database settings:
- **Mitigation:** Explicit NULL checks in initialization code
- **Testing:** Unit tests for NULL reset logic

---

## Success Criteria

### Functional Requirements

- [ ] All 5 modules start successfully with no config files present
- [ ] Root folder created automatically at default location
- [ ] Database created automatically with default schema
- [ ] Warning logged (not error) for missing config files
- [ ] CLI arguments override all other config sources
- [ ] Environment variables override TOML files
- [ ] TOML files override compiled defaults
- [ ] Compiled defaults used when no other source available

### Non-Functional Requirements

- [ ] Startup time < 2 seconds on typical hardware
- [ ] No performance regression from current implementation
- [ ] All error messages include file paths and actionable guidance
- [ ] All log messages use appropriate levels (WARN for missing config, INFO for initialization)

### Testing Requirements

- [ ] >90% code coverage for config resolution logic
- [ ] >90% code coverage for database initialization logic
- [ ] All integration tests passing
- [ ] All manual test scenarios verified

### Documentation Requirements

- [ ] User guide includes "Quick Start" section with zero-config example
- [ ] Developer guide explains how to add new modules
- [ ] All specification documents updated with implementation details

---

## Timeline

**Total Duration:** 5 weeks

| Phase | Duration | Start | End | Dependencies |
|-------|----------|-------|-----|--------------|
| Phase 1 | 1 week | Week 1 | Week 1 | None |
| Phase 2 | 1 week | Week 2 | Week 2 | Phase 1 |
| Phase 3 | 2 weeks | Week 3 | Week 4 | Phase 1, 2 |
| Phase 4 | 1 week | Week 5 | Week 5 | Phase 3 |
| Phase 5 | 2 days | Week 5 | Week 5 | Phase 4 |

---

## Future Enhancements

### Post-Initial Implementation

**[ENHANCE-GD-010]** Configuration validation and migration:
- Detect outdated TOML config files
- Automatically migrate to new format
- Warn about deprecated settings

**[ENHANCE-GD-020]** Configuration UI:
- Web-based configuration editor
- Export/import configuration profiles
- Configuration validation with helpful error messages

**[ENHANCE-GD-030]** Advanced logging:
- Structured logging (JSON format)
- Log aggregation for multi-module debugging
- Log level configuration via database

---

End of document - Graceful Degradation Implementation Plan

**Document Version:** 1.0
**Last Updated:** 2025-10-18
**Status:** Draft

**Change Log:**
- v1.0 (2025-10-18): Initial implementation plan created based on REQ-NF-030 through REQ-NF-036, ARCH-INIT-005 through ARCH-INIT-020, and DEP-CFG-031 through DEP-CFG-040
