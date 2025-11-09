# WKMP Database Schema Management, Zero-Configuration Startup & Initialization Requirements

**Comprehensive Specification Summary**
Generated: 2025-11-09
Scope: All database schema, zero-configuration startup, and initialization requirements across WKMP codebase

---

## Executive Summary

WKMP implements a comprehensive zero-configuration startup pattern where:
- All 6 microservices start with zero configuration files
- Database is automatically created with full schema on first run
- Settings are initialized with sensible defaults
- Shared SQLite database (`wkmp.db`) serves as single source of truth
- Configuration resolution follows 4-tier priority system (CLI → ENV → TOML → defaults)

**Key Documents:**
- `/home/sw/Dev/McRhythm/docs/IMPL001-database_schema.md` (1,325 lines) - Complete schema definition
- `/home/sw/Dev/McRhythm/docs/REQ001-requirements.md` (679 lines) - Functional/non-functional requirements
- `/home/sw/Dev/McRhythm/docs/SPEC001-architecture.md` (556 lines) - System architecture and module design
- `/home/sw/Dev/McRhythm/docs/ADR-003-zero_configuration_strategy.md` - Zero-config decision record
- `/home/sw/Dev/McRhythm/docs/IMPL004-deployment.md` - Deployment specifications
- `/home/sw/Dev/McRhythm/docs/IMPL007-graceful_degradation_implementation.md` - Implementation details

---

## 1. Complete List of Requirement IDs

### Zero-Configuration Startup Requirements (REQ-NF-030 through REQ-NF-038)

| Requirement ID | Title | Scope | Status |
|---|---|---|---|
| **REQ-NF-030** | Configuration file graceful degradation | ALL MODULES | ✅ Implemented |
| **REQ-NF-031** | Missing TOML files SHALL NOT cause termination | ALL MODULES | ✅ Implemented |
| **REQ-NF-032** | Missing config → warning + defaults + startup | ALL MODULES | ✅ Implemented |
| **REQ-NF-033** | Root folder default location per platform | ALL MODULES | ✅ Implemented |
| **REQ-NF-034** | Default values for logging, static assets | ALL MODULES | ✅ Implemented |
| **REQ-NF-035** | Priority order for root folder resolution | ALL MODULES (MANDATORY) | ✅ Implemented |
| **REQ-NF-036** | Automatic directory/database creation | ALL MODULES | ✅ Implemented |
| **REQ-NF-037** | Implementation enforcement via wkmp_common | ALL MODULES (MANDATORY) | ✅ Implemented |
| **REQ-NF-038** | TOML Configuration Directory Auto-Creation | ALL MODULES | ✅ Implemented |

### Operational Monitoring Requirements (REQ-NF-050 through REQ-NF-054)

| Requirement ID | Title | Scope | Status |
|---|---|---|---|
| **REQ-NF-050** | Operational Monitoring | ALL MODULES | Partial |
| **REQ-NF-051** | Health check endpoints | ALL MODULES | Partial |
| **REQ-NF-052** | Health check response time (<2s) | ALL MODULES | Partial |
| **REQ-NF-053** | Health endpoint JSON format | ALL MODULES | Partial |
| **REQ-NF-054** | Future: Detailed diagnostics | ALL MODULES | Not Started |

### Architecture Requirements (ARCH-INIT-*)

| Requirement ID | Title | Scope | Status |
|---|---|---|---|
| **ARCH-INIT-003** | Tracing subscriber initialization | ALL MODULES | ✅ Implemented |
| **ARCH-INIT-004** | Build identification logging (REQUIRED immediately after tracing) | ALL MODULES | ✅ Implemented |
| **ARCH-INIT-005** | Root folder resolution algorithm | ALL MODULES | ✅ Implemented |
| **ARCH-INIT-010** | Module startup sequence | ALL MODULES | ✅ Implemented |
| **ARCH-INIT-015** | Missing configuration handling | ALL MODULES | ✅ Implemented |
| **ARCH-INIT-020** | Default value initialization behavior | ALL MODULES | ✅ Implemented |

### Deployment Requirements (DEP-*)

| Requirement ID | Title | Scope | Status |
|---|---|---|---|
| **DEP-DB-010** | SQLite database location at `{root_folder}/wkmp.db` | ALL MODULES | ✅ Implemented |
| **DEP-DB-011** | Automatic database initialization on first run | ALL MODULES | ✅ Implemented |
| **DEP-DB-020** | All modules use same root folder path | ALL MODULES | ✅ Implemented |
| **DEP-DB-030** | Auto-create root folder and database directory | ALL MODULES | ✅ Implemented |
| **DEP-CFG-031** | Graceful degradation for missing config files | ALL MODULES | ✅ Implemented |
| **DEP-CFG-035** | Module discovery via database | ALL MODULES | ✅ Implemented |
| **DEP-CFG-040** | Compiled default configuration values | ALL MODULES | ✅ Implemented |

---

## 2. Database Schema Management Specifications

### 2.1 Schema Versioning

**Document:** IMPL001-database_schema.md (lines 71-81, 1253-1283)

**Current Schema Version:** `0.1` (Development phase)

**Schema Version Table:**
```sql
CREATE TABLE schema_version (
    version INTEGER PRIMARY KEY,
    applied_at TIMESTAMP NOT NULL
);
```

**Migration Strategy (Post-Release):**
1. Schema version tracked in `schema_version` table
2. Migration scripts numbered sequentially (001_initial.sql, 002_add_works.sql, etc.)
3. On startup: Check current version and apply pending migrations
4. Each migration wrapped in transaction with automatic backup before applying
5. Databases cannot be downgraded (breaking changes require backup + rebuild)

**Current Status (Development Phase):**
- No migration scripts maintained during development
- Schema changes addressed by database deletion and rebuild from scratch
- Breaking changes explicitly documented (e.g., REQ-F-003: `duration` REAL → `duration_ticks` INTEGER)

**Existing Migrations:**
- `/home/sw/Dev/McRhythm/migrations/006_wkmp_ai_hybrid_fusion.sql` - PLAN023 extensions for wkmp-ai provenance tracking

### 2.2 Core Tables and Relationships

**Core Entity Tables:**
- `schema_version` - Schema version tracking
- `users` - User accounts and authentication (pre-populated with Anonymous user)
- `files` - Audio files discovered by scanner
- `passages` - Playable audio regions within files
- `songs` - MusicBrainz Recordings
- `artists` - MusicBrainz Artists
- `works` - Musical compositions (MusicBrainz Works)
- `albums` - Album/Release data (MusicBrainz)
- `images` - Images associated with songs, albums, artists, works, passages

**Relationship Tables (Many-to-Many):**
- `passage_songs` - Associates passages with songs (includes timing)
- `song_artists` - Associates songs with artists (includes weights)
- `passage_albums` - Associates passages with albums
- `song_works` - Associates songs with works (many-to-many)

**Playback & History:**
- `play_history` - Global playback event log (system-wide, not per-user)
- `song_play_history` - Detailed per-song play records (cooldown calculations)
- `likes_dislikes` - User-scoped taste preferences (user-specific data)
- `song_play_counts` (VIEW) - Optimized time-range play counts

**Configuration & Settings:**
- `settings` - Global key-value store for all runtime configuration
- `module_config` - Module network configuration (host/port discovery)

**Time-based Flavor System:**
- `timeslots` - 24-hour schedule defining musical flavor targets
- `timeslot_passages` - Passages defining flavor targets for each timeslot

**External API Caching:**
- `acoustid_cache` - Cached AcoustID responses
- `musicbrainz_cache` - Cached MusicBrainz metadata
- `acousticbrainz_cache` - Cached AcousticBrainz characterization data

### 2.3 Key Design Decisions

**UUID Primary Keys:**
- All entity tables use TEXT-based UUID (guid) instead of auto-incrementing integers
- Globally unique across all databases
- Enables database merging across versions (Full → Lite/Minimal)
- Format: lowercase hyphenated (e.g., `550e8400-e29b-41d4-a716-446655440000`)

**Global vs User-Scoped Data:**
- **Global (system-wide):** settings, queue, play_history, playback state
- **User-scoped:** likes_dislikes, user preferences, accounts
- Rationale: WKMP functions like shared hi-fi system (multiple users listening simultaneously)

**Relative Path Storage:**
- All `path` and `file_path` columns store paths relative to root folder
- Forward slashes (`/`) on all platforms for consistency
- Windows modules translate to backslashes when calling APIs
- Enables portability: move entire root folder to relocate complete library

**Tick-Based Timing (SPEC017 Integration):**
- All audio timing uses INTEGER ticks instead of REAL seconds
- Conversion: `ticks = seconds * 28,224,000` (1 tick ≈ 35.4 nanoseconds)
- Fields: `start_time_ticks`, `end_time_ticks`, `fade_in_start_ticks`, etc.
- Provides sample-accurate precision for crossfading
- See SPEC017-sample_rate_conversion.md for complete timing specification

**JSON Storage for Extensibility:**
- Musical flavor vectors stored as JSON in `passages.musical_flavor_vector`
- Import metadata stored as JSON in `passages.import_metadata`
- Additional metadata stored as JSON in `passages.additional_metadata`
- Queue entry timing overrides stored as JSON in `settings.queue_entry_timing_overrides`
- Allows schema-less design: add new parameters without database migrations

**Automatic Triggers:**
- `updated_at` timestamps automatically updated on row modification
- `last_played_at` timestamps automatically updated when passages played
- Cascading deletes via foreign key constraints

### 2.4 Settings Table (Database-First Configuration)

**Document:** IMPL001-database_schema.md (lines 799-891, 999-1046)

**Total Settings:** 27+ runtime configuration keys (all stored as TEXT, typed in application code)

**Configuration Philosophy:**
- Database-first: ALL runtime settings stored in `settings` table
- TOML files: Bootstrap only (root folder, logging, static assets)
- NULL values: Automatically initialized with built-in defaults and written back to database
- Source of truth: Database settings, never TOML

**Settings by Category:**

**Playback State (DB-SET-050):**
- `initial_play_state` - "playing" or "paused" on app launch
- `currently_playing_passage_id` - UUID of passage currently playing
- `last_played_passage_id` - UUID of last played passage
- `last_played_position_ticks` - Position in ticks (updated on clean shutdown only)

**Audio Configuration (DB-SET-060):**
- `volume_level` - 0.0-1.0 (HTTP/API use 0.0-1.0; UI displays 0-100)
- `audio_sink` - Selected audio output sink identifier

**Event Timing (DB-SET-070):**
- `position_event_interval_ms` - Mixer PositionUpdate event interval (default: 1000ms, range: 100-5000ms)
- `playback_progress_interval_ms` - SSE PlaybackProgress event interval (default: 5000ms, range: 1000-10000ms)

**Database Backup (DB-SET-080):**
- `backup_location` - Backup directory path
- `backup_interval_ms` - Periodic backup interval (default: 7776000000 = 90 days)
- `backup_minimum_interval_ms` - Minimum time between startup backups (default: 1209600000 = 14 days)
- `backup_retention_count` - Number of timestamped backups to keep (default: 3)
- `last_backup_timestamp_ms` - Unix milliseconds of last successful backup

**Crossfade (DB-SET-090):**
- `global_crossfade_time` - Global crossfade duration in seconds (default: 2.0)
- `global_fade_curve` - Fade curve pair (exponential_logarithmic, linear_linear, cosine_cosine)

**Pause/Resume (DB-SET-100):**
- `resume_from_pause_fade_in_duration` - Resume fade-in duration (default: 0.5s, range: 0.0-5.0s)
- `resume_from_pause_fade_in_curve` - Resume fade-in curve (linear, exponential, cosine)

**Volume Fade Updates (DB-SET-110):**
- `volume_fade_update_period` - Volume fade update period in milliseconds (default: 10, range: 1-100)

**Queue Management (DB-SET-120):**
- `queue_entry_timing_overrides` - JSON mapping queue entry guid → timing overrides
- `queue_refill_threshold_passages` - Min passages before refill (default: 2, Full/Lite only)
- `queue_refill_threshold_seconds` - Min seconds before refill (default: 900 = 15 minutes, Full/Lite only)
- `queue_refill_request_throttle_seconds` - Min interval between refill requests (default: 10)
- `queue_refill_acknowledgment_timeout_seconds` - Timeout for PD acknowledgment (default: 5)
- `queue_max_size` - Maximum queue size (default: 100)
- `queue_max_enqueue_batch` - Max passages to enqueue at once (default: 5, wkmp-pd, Full/Lite)

**Module Management (DB-SET-130):**
- `relaunch_delay` - Seconds between module relaunch attempts (default: 5)
- `relaunch_attempts` - Max relaunch attempts (default: 20)

**Session Management (DB-SET-140):**
- `session_timeout_seconds` - Session timeout (default: 31536000 = 1 year)

**File Ingest (DB-SET-150):**
- `ingest_max_concurrent_jobs` - Max concurrent file processing jobs (default: 4, Full only)

**Library (DB-SET-160):**
- `music_directories` - JSON array of directories to scan
- `temporary_flavor_override` - JSON with target flavor and expiration

**HTTP Server Configuration (DB-SET-170):**
- `http_base_ports` - JSON array of base port numbers
- `http_request_timeout_ms` - Request timeout (default: 30000)
- `http_keepalive_timeout_ms` - Keepalive timeout (default: 60000)
- `http_max_body_size_bytes` - Max request body size (default: 1048576)

**Program Director (DB-SET-180):**
- `playback_failure_threshold` - Failures before stopping auto selection (default: 3)
- `playback_failure_window_seconds` - Time window for failure counting (default: 60)

### 2.5 Module Configuration Table

**Document:** IMPL001-database_schema.md (lines 750-798)

**Purpose:** Module network configuration for inter-module discovery

**Default Initialization (First Run):**

| module_name | host | port | enabled | Notes |
|---|---|---|---|---|
| user_interface | 127.0.0.1 | 5720 | 1 | Can be changed to 0.0.0.0 for network access |
| audio_player | 127.0.0.1 | 5721 | 1 | Internal service (localhost-only) |
| program_director | 127.0.0.1 | 5722 | 1 | Internal service (localhost-only) |
| audio_ingest | 0.0.0.0 | 5723 | 1 | On-demand, network accessible |
| lyric_editor | 0.0.0.0 | 5724 | 1 | On-demand, network accessible |
| database_review | 0.0.0.0 | 5725 | 1 | On-demand (Full version only) |

**Initialization Behavior:**
- **User Interface (wkmp-ui)** initializes entire database on first run (including module_config defaults)
- **Other modules** initialize required tables if missing on startup
- Each module verifies its entry exists, adds with defaults if not found
- Idempotent: safe to run multiple times

---

## 3. Zero-Configuration Startup Specification

### 3.1 Root Folder Resolution (4-Tier Priority System)

**Requirement:** REQ-NF-035, ARCH-INIT-005

**Priority Order (Highest to Lowest):**

1. **CLI Argument** (Tier 1 - Highest Priority)
   - `--root-folder /custom/path`
   - `--root /custom/path`

2. **Environment Variable** (Tier 2)
   - `WKMP_ROOT_FOLDER=/custom/path`
   - `WKMP_ROOT=/custom/path`

3. **TOML Config File** (Tier 3)
   - Linux: `~/.config/wkmp/<module-name>.toml`
   - macOS: `~/Library/Application Support/WKMP/<module-name>.toml`
   - Windows: `%APPDATA%\WKMP\<module-name>.toml`

4. **Compiled Default** (Tier 4 - Lowest Priority)
   - Linux: `~/Music`
   - macOS: `~/Music`
   - Windows: `%USERPROFILE%\Music` or `%USERPROFILE%\Music\wkmp`

**Key Features:**
- Mutually exclusive: each tier completely overrides lower tiers
- Graceful fallback: missing config files don't cause errors
- Logging: Resolution method logged at startup (CLI, ENV, TOML, or Default)
- Validation: Root folder created automatically if missing

### 3.2 Database Initialization Sequence

**Requirement:** REQ-NF-036, ARCH-INIT-010

**Module Startup Sequence (per ARCH-INIT-004 pattern):**

```
Step 0: Initialize tracing subscriber [ARCH-INIT-003]
  └─ Set up logging (stderr/file)

Step 1: Log build identification IMMEDIATELY [ARCH-INIT-004] ✅ REQUIRED
  └─ Format: "Starting WKMP [Module Name] v{version} [{git_hash}] built {timestamp} ({profile})"
  └─ Logged to stderr before database access

Step 2: Resolve root folder [ARCH-INIT-005]
  └─ Use RootFolderResolver::new("module-name")
  └─ Apply 4-tier priority system

Step 3: Create root folder if missing [ARCH-INIT-010]
  └─ Use RootFolderInitializer::ensure_directory_exists()
  └─ Log: "Created root folder: {path}"

Step 4: Initialize database [ARCH-INIT-010]
  └─ Call init_database() function
  └─ Creates wkmp.db if missing
  └─ Runs migrations if applicable
  └─ Idempotent: safe to run multiple times

Step 5: Initialize required tables [ARCH-INIT-010]
  └─ Module-specific table creation
  └─ Idempotent: checks if tables exist first
  └─ Each module creates only tables it requires

Step 6: Initialize default settings [ARCH-INIT-020]
  └─ Call ensure_setting_exists() for each key
  └─ Insert built-in default if key missing
  └─ NULL handling: reset NULL values to defaults
  └─ Per-setting logging: "Setting '{key}' initialized to '{value}'"

Step 7: Initialize module config [ARCH-INIT-010]
  └─ Each module inserts its own entry if missing
  └─ Uses defaults from module_config table specification

Result: Module starts successfully with zero configuration required
```

### 3.3 Graceful Degradation Behavior

**Requirements:** REQ-NF-031, REQ-NF-032

**Missing Configuration File Handling:**

```
IF config file does not exist THEN
  1. Log warning: "WARN: Config file not found at {path}, using default configuration"
  2. Use compiled default values for:
     - Root folder location
     - Logging level
     - Log file path
     - Static asset paths
  3. Continue startup normally (NO TERMINATION ERROR)
  4. Initialize database with defaults
  5. Startup succeeds
END IF
```

**Corrupted Configuration File Handling:**

```
IF config file exists but is malformed THEN
  1. Log error: "ERROR: Failed to parse config file at {path}: {parse_error}"
  2. Use compiled default values
  3. Continue startup (graceful degradation)
END IF
```

**Missing Database Handling:**

```
IF database file does not exist THEN
  1. Log info: "Database not found, creating at {db_path}"
  2. Create empty database with create()
  3. Run migrations (if any)
  4. Initialize all required tables
  5. Initialize all default settings
  6. Result: fully initialized database ready for use
END IF
```

**Missing Database Tables Handling:**

```
FOR EACH required table DO
  IF table does not exist THEN
    1. Create table with full schema
    2. Initialize with defaults if applicable
    3. Log: "Created table {table_name}"
  END IF
END FOR
```

**NULL Setting Values:**

```
FOR EACH setting in database DO
  IF value IS NULL THEN
    1. Reset to built-in default
    2. Write to database
    3. Log: "Setting '{key}' reset to default: '{value}'"
  END IF
END FOR
```

### 3.4 Configuration File Auto-Creation

**Requirement:** REQ-NF-038

**TOML Configuration Directory Auto-Creation:**
- When module needs to write config file (e.g., auto-migration)
- Parent directory is created automatically if missing
- Per-platform paths created with proper separators
- Logging: "Created config directory: {path}"

**Benefits:**
- Users don't need to manually create `~/.config/wkmp/` on Linux
- Automatic migration from ENV to TOML works seamlessly
- Write-operations always succeed

---

## 4. Implementation Details

### 4.1 Shared Configuration Library (wkmp_common::config)

**Files:**
- `wkmp-common/src/config/mod.rs` - Main module
- `wkmp-common/src/config/defaults.rs` - Platform-specific defaults
- `wkmp-common/src/config/resolution.rs` - 4-tier priority resolution
- `wkmp-common/src/config/toml_loader.rs` - TOML file loading
- `wkmp-common/src/config/validation.rs` - Config value validation

**Key Types:**

```rust
pub struct RootFolderResolver {
    module_name: String,
}

impl RootFolderResolver {
    pub fn new(module_name: &str) -> Self { ... }
    pub fn resolve(&self) -> PathBuf { ... }
}

pub struct RootFolderInitializer {
    root_folder: PathBuf,
}

impl RootFolderInitializer {
    pub fn new(root_folder: PathBuf) -> Self { ... }
    pub fn ensure_directory_exists(&self) -> Result<()> { ... }
    pub fn database_path(&self) -> PathBuf { ... }
}
```

**Mandatory Usage (REQ-NF-037):**
- ALL modules MUST use these utilities
- NO module may implement custom root folder resolution
- NO module may hardcode database paths
- Enforcement: Code review + compiler cannot prevent, but documented requirement

### 4.2 Database Initialization Functions

**File:** `wkmp-common/src/db/init.rs`

**Key Functions:**

```rust
/// Initialize database: create if missing, run migrations, create tables
pub async fn init_database(db_path: &Path) -> Result<SqlitePool> { ... }

/// Idempotent: create table only if not exists
pub async fn create_table_if_not_exists(pool: &SqlitePool, table_name: &str, schema: &str) { ... }

/// Initialize settings table with defaults
pub async fn init_settings(pool: &SqlitePool) -> Result<()> { ... }

/// Ensure a setting exists, insert default if missing
pub async fn ensure_setting(pool: &SqlitePool, key: &str, default: &str) -> Result<()> { ... }

/// Handle NULL values: reset to defaults
pub async fn reset_null_settings(pool: &SqlitePool) -> Result<()> { ... }

/// Initialize users table with Anonymous user
pub async fn init_users(pool: &SqlitePool) -> Result<()> { ... }

/// Initialize module_config with defaults
pub async fn init_module_config(pool: &SqlitePool) -> Result<()> { ... }
```

**Idempotency Guarantees:**
- All functions safe to call multiple times
- No errors if resources already exist
- Concurrent calls from multiple modules supported (via SQLite locking)

### 4.3 Per-Module Main Function Pattern

**Required Pattern (All 6 Modules - MANDATORY):**

```rust
#[tokio::main]
async fn main() -> Result<()> {
    // Step 0: Initialize tracing subscriber [ARCH-INIT-003]
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "module_name=debug,wkmp_common=info".into()),
        )
        .with(tracing_subscriber::fmt::layer().with_target(true).with_file(true).with_line_number(true))
        .init();

    // **[ARCH-INIT-004]** Log build identification IMMEDIATELY after tracing init
    // REQUIRED for all modules - provides instant startup feedback before database delays
    info!(
        "Starting WKMP [Module Name] (module-id) v{} [{}] built {} ({})",
        env!("CARGO_PKG_VERSION"),
        env!("GIT_HASH"),
        env!("BUILD_TIMESTAMP"),
        env!("BUILD_PROFILE")
    );

    // Step 1: Resolve root folder (4-tier priority)
    let resolver = wkmp_common::config::RootFolderResolver::new("module-name");
    let root_folder = resolver.resolve();

    // Step 2: Create directory if missing
    let initializer = wkmp_common::config::RootFolderInitializer::new(root_folder);
    initializer.ensure_directory_exists()?;

    // Step 3: Get database path
    let db_path = initializer.database_path();  // root_folder/wkmp.db

    // Step 4: Initialize database
    let db_pool = wkmp_common::db::init::init_database(&db_path).await?;

    // Step 5: Initialize module-specific tables
    // (Each module creates only tables it requires)

    // Step 6: Initialize default settings
    // (Using ensure_setting() for each key)

    // Step 7: Start HTTP server on configured port
    // (Read port from module_config table in database)

    Ok(())
}
```

---

## 5. Design Patterns and Constraints

### 5.1 Database as Source of Truth

**Principle:** DATABASE-FIRST CONFIGURATION

- **Runtime settings** → `settings` table (ALL configuration)
- **Module discovery** → `module_config` table (host/port for all modules)
- **TOML files** → Bootstrap ONLY (root folder, logging, static assets)
- **Compiled defaults** → Fallback when TOML missing (graceful degradation)

**Precedence Order:**
1. Database `settings` table (source of truth if present)
2. TOML config file (bootstrap configuration)
3. Environment variables (deployment overrides)
4. Compiled defaults (final fallback)

**NULL Value Handling:**
- If setting is NULL in database: Reset to built-in default and write back
- If setting missing from table: Insert with default value
- If config file missing: Use compiled default
- Result: Database always has valid values

### 5.2 Idempotency Guarantees

**All initialization functions are idempotent:**

```
idempotent(create_table_if_not_exists)
  ✓ First call: Creates table with full schema
  ✓ Second call: Checks if exists, returns OK
  ✓ Nth call: Always returns OK

idempotent(init_settings)
  ✓ First call: Inserts all default settings
  ✓ Second call: Finds existing settings, skips insert
  ✓ Nth call: Always returns OK

idempotent(init_database)
  ✓ First call: Creates database, runs migrations, creates tables
  ✓ Second call: Opens existing database, applies any pending migrations
  ✓ Nth call: Always returns OK
```

**Enables:**
- Multiple modules starting simultaneously
- Restart without cleanup
- Safe concurrent initialization
- No race conditions (SQLite handles locking)

### 5.3 Separation of Concerns

**Layers:**

| Layer | Responsibility | Technology | Files |
|---|---|---|---|
| **Application** | Business logic, UI, audio processing | Rust/Tokio/Axum | wkmp-{ap,ui,pd,ai,le} |
| **Data Access** | Database queries, ORM | SQLx | wkmp-common/db/ |
| **Database** | Schema, migrations, data storage | SQLite | wkmp.db, migrations/*.sql |
| **Configuration** | Root folder resolution, TOML loading | wkmp_common::config | wkmp-common/config/ |

**No Cross-Module Dependencies:**
- wkmp-ap doesn't depend on wkmp-ui
- wkmp-pd doesn't depend on wkmp-ai
- All communication via HTTP REST + SSE
- Database is shared but each module owns its tables

---

## 6. Current Implementation Status

### 6.1 Completed Features

✅ **Zero-Configuration Startup (REQ-NF-030 through REQ-NF-037):**
- 4-tier root folder resolution implemented
- Database auto-creation working
- All modules use shared RootFolderResolver/RootFolderInitializer
- Graceful degradation for missing config files
- Default values initialization for 27+ settings

✅ **Architecture Requirements (ARCH-INIT-*):**
- Tracing subscriber initialization [ARCH-INIT-003]
- Build identification logging [ARCH-INIT-004]
- Root folder resolution [ARCH-INIT-005]
- Module startup sequence [ARCH-INIT-010]
- Default value initialization [ARCH-INIT-020]

✅ **Database Schema:**
- 30+ tables fully defined and documented
- UUID primary keys for all entities
- Foreign key cascades for cleanup
- Automatic triggers for timestamps and cooldowns
- JSON storage for extensibility

✅ **Migrations:**
- 006_wkmp_ai_hybrid_fusion.sql implemented for PLAN023
- Migration framework in place (sqlx::migrate!)
- Idempotent table creation functions

✅ **Testing:**
- 27+ tests written for zero-config startup
- Unit tests for each 4-tier resolution priority
- Integration tests for database initialization
- Platform-specific tests (Linux, macOS, Windows)

### 6.2 Partial Features

⚠️ **Health Check Endpoints (REQ-NF-050 through REQ-NF-053):**
- Partial implementation
- `/health` endpoint present in some modules
- Response time compliance not fully verified
- Status field present but minimal diagnostics

### 6.3 Not Yet Implemented

❌ **Health Check Diagnostics (REQ-NF-054):**
- Detailed subsystem diagnostics not yet included
- Database connectivity checks not in response
- Audio device availability checks not included
- Future enhancement for v2.0

---

## 7. Key Constraints and Design Decisions

### 7.1 SQLite Shared Database Constraint

**Why SQLite, not PostgreSQL/MySQL?**

**Decision Rationale:**
1. **Zero-configuration goal** - SQLite file-based, no server required
2. **No external dependencies** - Single file (`wkmp.db`), no service to install/configure
3. **Portability** - Move entire library by moving one directory
4. **Simplicity** - Perfect for desktop/embedded use cases
5. **Sufficient performance** - 95% of WKMP operations not heavy I/O

**Trade-offs:**
- Limited concurrent writes (acceptable for WKMP: single queue, not multi-user writes)
- No full-text search (not required for WKMP data types)
- Limited data types (mitigated by JSON columns)

**Consequence:** All modules share single SQLite connection pool, use database for coordination

### 7.2 UUID Primary Keys Constraint

**Why UUIDs instead of auto-increment integers?**

**Decision Rationale:**
1. **Database merging** - Essential for version upgrades (Minimal → Lite → Full)
2. **Global uniqueness** - Can merge data from different databases
3. **No coordination needed** - Each module can generate IDs independently
4. **Future distribution** - Enables multi-node deployments

**Implementation:**
- UUID v4 (random) generated in application code
- Stored as TEXT in lowercase hyphenated format
- Indexes on all foreign key references

### 7.3 Global vs User-Scoped Data Constraint

**Why playback state is global, not per-user?**

**Decision Rationale:**
- WKMP models shared hi-fi system (family in living room)
- Multiple users listening simultaneously to same audio
- Pause affects everyone
- Queue affects everyone
- Only individual taste (likes/dislikes) is per-user

**Consequence:**
- `settings`, `queue`, `play_history` are system-wide
- `likes_dislikes` is user-scoped (individual taste preferences)
- No user-specific playback state

### 7.4 Tick-Based Timing Constraint

**Why ticks instead of milliseconds for audio timing?**

**Decision Rationale (SPEC017):**
1. **Sample-accurate precision** - ~0.02ms precision for crossfading
2. **Lossless conversion** - Seconds → ticks is bijective
3. **Consistency** - One timing system throughout audio pipeline
4. **Database storage** - INTEGER (not REAL) avoids floating-point precision issues

**Conversion:** `ticks = seconds * 28,224,000` (1 tick ≈ 35.4 nanoseconds)

**Fields:**
- Timing fields in `passages` table: `start_time_ticks`, `end_time_ticks`, `fade_in_start_ticks`, etc.
- Audio durations in `song_play_history`: `audio_played_ticks`, `pause_duration_ticks`
- Position tracking: `last_played_position_ticks`
- Queue entry overrides: JSON with `*_ticks` fields

---

## 8. Testing and Verification

### 8.1 Test Coverage for Zero-Config

**Test Categories:**

1. **Root Folder Resolution (5 tests)**
   - CLI argument overrides all
   - ENV var overrides TOML
   - TOML overrides defaults
   - Default fallback when all absent
   - Platform-specific defaults (Linux, macOS, Windows)

2. **Configuration Loading (4 tests)**
   - Missing TOML files don't error
   - Missing config → warning log
   - Corrupted TOML → parse error + default fallback
   - Environment variables respected

3. **Database Initialization (5 tests)**
   - Database created when missing
   - Tables created with full schema
   - Migrations applied if applicable
   - Settings initialized with defaults
   - NULL values reset to defaults

4. **Module-Specific Startup (3+ tests per module)**
   - Module starts with zero config
   - Module discovers other modules via database
   - Module binds to correct port

5. **Idempotency (3 tests)**
   - init_database() safe to call multiple times
   - init_settings() safe to call multiple times
   - Multiple modules starting simultaneously

### 8.2 Integration Testing

**End-to-End Scenarios:**

```
Scenario 1: First Run (No Config, No Database)
  1. User runs: wkmp-ui
  2. Result: Starts with zero configuration
           Creates ~/Music/wkmp.db
           Initializes all tables
           Initializes all settings
           Binds to 5720

Scenario 2: Config File Override
  1. User sets: WKMP_ROOT_FOLDER=/custom/path
  2. User runs: wkmp-ap
  3. Result: Uses /custom/path as root folder
           Creates database at /custom/path/wkmp.db

Scenario 3: Multi-Module Startup (Concurrent)
  1. User runs: wkmp-ui (auto-starts wkmp-ap and wkmp-pd)
  2. All 3 modules start simultaneously
  3. Each module:
     - Resolves root folder independently
     - Initializes required tables
     - Queries module_config for other modules' addresses
     - Binds to port
  4. Result: All 3 modules running, fully coordinated

Scenario 4: Version Upgrade (Minimal → Lite → Full)
  1. User runs Minimal version
     - Creates database with minimal tables
  2. User upgrades to Lite version
     - wkmp-pd module starts
     - Creates additional required tables
     - Database still works, no migration needed
  3. User upgrades to Full version
     - wkmp-ai module starts
     - Creates additional tables for file import
     - Previous data preserved
```

---

## 9. Gaps and Recommendations

### 9.1 Identified Gaps

**Gap 1: Health Check Endpoints Incomplete**
- Requirement: REQ-NF-050 through REQ-NF-054
- Status: Partial implementation
- **Gap:** `/health` endpoints present but missing detailed diagnostics
- **Recommendation:** Add database connectivity checks to response
- **Effort:** 2-3 hours per module

**Gap 2: Database Backup Implementation**
- Requirement: Settings table supports backup configuration
- Status: Schema ready, backup functions not yet implemented
- **Gap:** No automatic backup execution
- **Recommendation:** Implement backup_handler in wkmp-ui module
- **Effort:** 1-2 days

**Gap 3: Migration Path from Old Installations**
- Requirement: REQ-NF-033 changed default root folder location
- Status: Documentation exists, tool not provided
- **Gap:** Users with ~/Music/wkmp.db need manual migration
- **Recommendation:** Provide automated migration script or wizard
- **Effort:** 1 day

**Gap 4: Configuration Validation**
- Requirement: REQ-NF-035 priority order is enforced
- Status: Implemented
- **Gap:** Silent fallback if user typos ENV var (e.g., `WKMP_ROOT_FOLDER=/opt/music` typo silently falls back to default)
- **Recommendation:** Add verbose logging option to show all resolution attempts
- **Effort:** 2-3 hours

### 9.2 Future Enhancements

**Enhancement 1: Full XDG Compliance (Linux)**
- Currently uses `~/.config/wkmp/` for config
- Could expand to use `XDG_CONFIG_HOME`, `XDG_DATA_HOME`, `XDG_CACHE_HOME`
- **Priority:** Medium (affects Linux desktop users)
- **Effort:** 1-2 days

**Enhancement 2: Windows Registry Integration**
- Currently uses ENV vars for overrides
- Could check `HKCU\Software\WKMP\RootFolder`
- **Priority:** Low (ENV vars sufficient for power users)
- **Effort:** 1 day

**Enhancement 3: Configuration Migration from Previous Versions**
- Detect old `~/.wkmp/wkmp.db` location
- Offer interactive migration to new location
- **Priority:** Medium (affects existing users)
- **Effort:** 1-2 days

**Enhancement 4: Automated Schema Repair**
- Detect corrupted tables
- Offer repair or rebuild options
- **Priority:** Low (SQLite corruption rare)
- **Effort:** 2-3 days

---

## 10. Reference Documents and Line Ranges

### Core Specifications

| Document | Path | Key Sections | Lines |
|---|---|---|---|
| **Database Schema** | docs/IMPL001-database_schema.md | Schema versioning (71-81), Settings (799-891), Migration strategy (1253-1283) | 1,325 |
| **Requirements** | docs/REQ001-requirements.md | REQ-NF-030 through REQ-NF-038 (256-296) | 679 |
| **Architecture** | docs/SPEC001-architecture.md | Zero-config startup (382-426), Module responsibilities (87-193) | 556 |
| **Zero-Config ADR** | docs/ADR-003-zero_configuration_strategy.md | Decision rationale (23-65), Implementation pattern (34-51) | 406 |
| **Deployment** | docs/IMPL004-deployment.md | Database location (250-274), Config locations (35-248), Module config (56-94) | 600+ |
| **Graceful Degradation** | docs/IMPL007-graceful_degradation_implementation.md | Implementation phases (72-650+), Testing strategy (309-338) | 1,200+ |
| **Test Summary** | docs/IMPL007-TEST_SUMMARY.md | Zero-config verification (162-187), Test categories (172-406) | 600+ |

### Supporting Documents

| Document | Purpose | Key Sections |
|---|---|---|
| docs/SPEC017-sample_rate_conversion.md | Tick-based timing specification | Database storage section |
| docs/SPEC002-crossfade.md | Crossfade timing and curves | Database storage section |
| docs/IMPL002-coding_conventions.md | Coding standards | CO-175 schema versioning |
| docs/IMPL003-project_structure.md | Project layout | Migrations directory, wkmp_common/config |
| docs/IMPL014-database_queries.md | Database access patterns | Query examples |

---

## 11. Quick Reference Tables

### Requirement Implementation Status

| Category | Count | Status | Coverage |
|---|---|---|---|
| Zero-Config Startup | 8 | ✅ Implemented | 100% |
| Architecture Init | 6 | ✅ Implemented | 100% |
| Deployment | 7 | ✅ Implemented | 100% |
| Health Checks | 5 | ⚠️ Partial | 60% |
| **Total** | **26** | **✅ 85%** | |

### Module Compliance (REQ-NF-037)

| Module | Uses RootFolderResolver | Uses RootFolderInitializer | Custom Path Handling | Status |
|---|---|---|---|---|
| wkmp-ap | ✅ | ✅ | ❌ | ✅ Compliant |
| wkmp-ui | ✅ | ✅ | ❌ | ✅ Compliant |
| wkmp-pd | ✅ | ✅ | ❌ | ✅ Compliant |
| wkmp-ai | ✅ | ✅ | ❌ | ✅ Compliant |
| wkmp-le | ✅ | ✅ | ❌ | ✅ Compliant |
| wkmp-dr | ✅ | ✅ | ❌ | ✅ Compliant |

### Settings by Module

| Module | Settings Keys | Status |
|---|---|---|
| wkmp-ap | 13 (playback, audio, events, crossfade, volume, pause/resume, queue) | ✅ Implemented |
| wkmp-ui | 3 (session, backup) | ✅ Implemented |
| wkmp-pd | 5 (queue refill, failure handling) | ✅ Implemented |
| wkmp-ai | 1 (concurrent jobs) | ✅ Implemented |
| All | 27+ total keys | ✅ All implemented |

---

**Document Generated:** 2025-11-09
**Last Specification Review:** 2025-11-02
**Coverage:** All database schema, zero-configuration, and initialization specifications
**Status:** Comprehensive - covers all identified requirements and design decisions
