# WKMP Project Structure

**🏗️ TIER 3 - IMPLEMENTATION SPECIFICATION**

Defines the Rust workspace structure and organization for WKMP's microservices architecture.

> **Related Documentation:** [Architecture](SPEC001-architecture.md) | [Coding Conventions](IMPL002-coding_conventions.md) | [Implementation Order](EXEC001-implementation_order.md)

---

## Overview

WKMP uses a **Cargo workspace** to manage multiple binaries with shared common code. This structure enables:
- Code reuse across modules (database models, serialization, utilities)
- Unified dependency management
- Single build command for all modules
- Shared testing infrastructure

---

## Workspace Directory Structure

```
mcrhythm/
├── Cargo.toml                    # Workspace manifest
├── Cargo.lock                    # Unified dependency lock file
├── README.md
├── LICENSE
├── .gitignore
│
├── common/                       # Shared code library
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs               # Library root
│   │   ├── db/
│   │   │   ├── mod.rs
│   │   │   ├── schema.rs        # Database schema types
│   │   │   ├── models.rs        # Data models (File, Passage, Song, etc.)
│   │   │   ├── queries.rs       # Common database queries
│   │   │   └── migrations.rs    # Migration management
│   │   ├── events/
│   │   │   ├── mod.rs
│   │   │   └── types.rs         # WkmpEvent enum
│   │   ├── api/
│   │   │   ├── mod.rs
│   │   │   ├── types.rs         # API request/response types
│   │   │   └── client.rs        # HTTP client helpers
│   │   ├── config/
│   │   │   ├── mod.rs
│   │   │   └── module.rs        # Module config loading
│   │   ├── flavor/
│   │   │   ├── mod.rs
│   │   │   ├── types.rs         # FlavorVector, FlavorTarget
│   │   │   ├── distance.rs      # Distance calculations
│   │   │   └── centroid.rs      # Weighted centroid
│   │   ├── cooldown/
│   │   │   ├── mod.rs
│   │   │   └── calculator.rs    # Cooldown logic
│   │   ├── uuid.rs              # UUID utilities
│   │   ├── time.rs              # Timestamp helpers
│   │   └── error.rs             # Common error types
│   └── tests/
│       └── integration_tests.rs
│
├── wkmp-ap/                      # Audio Player binary
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs              # Binary entrypoint
│   │   ├── config.rs            # Module-specific config
│   │   ├── server.rs            # HTTP server setup
│   │   ├── api/
│   │   │   ├── mod.rs
│   │   │   ├── playback.rs      # Playback endpoints
│   │   │   ├── audio.rs         # Audio control endpoints
│   │   │   └── events.rs        # SSE endpoint
│   │   ├── playback/
│   │   │   ├── mod.rs
│   │   │   ├── engine/          # **[PLAN016]** Refactored modular engine
│   │   │   │   ├── mod.rs       # Public API re-exports
│   │   │   │   ├── core.rs      # Lifecycle, orchestration, process_queue (2,724 lines)
│   │   │   │   ├── queue.rs     # Queue operations (skip, enqueue, clear, remove) (511 lines)
│   │   │   │   └── diagnostics.rs # Monitoring, status, event handlers (1,019 lines)
│   │   │   ├── events.rs        # Internal PlaybackEvent types (not exposed via SSE)
│   │   │   ├── song_timeline.rs # Song boundary detection logic
│   │   │   ├── pipeline/        # Single-stream architecture
│   │   │   │   ├── mod.rs
│   │   │   │   ├── single_stream/ # Sample-accurate crossfading
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   ├── buffer.rs  # PCM buffer management
│   │   │   │   │   ├── mixer.rs   # Sample-accurate mixer with position events
│   │   │   │   │   └── curves.rs  # Fade curve algorithms
│   │   │   │   └── dual.rs       # Legacy dual-pipeline (archived)
│   │   │   ├── crossfade.rs     # Crossfade logic
│   │   │   └── queue_manager.rs # Queue manager
│   │   ├── audio/
│   │   │   ├── mod.rs
│   │   │   ├── device.rs        # Device management
│   │   │   └── volume.rs        # Volume control
│   │   ├── db/
│   │   │   ├── mod.rs
│   │   │   └── passage_songs.rs # Song timeline loading from database
│   │   ├── historian.rs         # Play history recorder
│   │   └── dev_ui/
│   │       └── templates/       # Minimal HTML developer UI
│   └── tests/
│       └── integration_tests.rs
│
├── wkmp-ui/                      # User Interface binary
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs
│   │   ├── config.rs
│   │   ├── server.rs
│   │   ├── api/
│   │   │   ├── mod.rs
│   │   │   ├── auth.rs          # Authentication endpoints
│   │   │   ├── proxy.rs         # Proxy to other modules
│   │   │   ├── library.rs       # Library browsing
│   │   │   └── events.rs        # SSE aggregation
│   │   ├── auth/
│   │   │   ├── mod.rs
│   │   │   ├── session.rs       # Session management
│   │   │   └── password.rs      # Password hashing
│   │   ├── proxy/
│   │   │   ├── mod.rs
│   │   │   ├── audio_player.rs  # Audio Player client
│   │   │   └── program_director.rs # Program Director client
│   │   └── static/              # Web UI assets
│   │       ├── index.html
│   │       ├── css/
│   │       ├── js/
│   │       └── images/
│   └── tests/
│       └── integration_tests.rs
│
├── wkmp-pd/                      # Program Director binary
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs
│   │   ├── config.rs
│   │   ├── server.rs
│   │   ├── api/
│   │   │   ├── mod.rs
│   │   │   ├── config.rs        # Configuration endpoints
│   │   │   ├── status.rs        # Status endpoints
│   │   │   └── events.rs        # SSE endpoint
│   │   ├── selection/
│   │   │   ├── mod.rs
│   │   │   ├── algorithm.rs     # Selection algorithm
│   │   │   ├── candidates.rs    # Candidate filtering
│   │   │   └── weights.rs       # Weight calculation
│   │   ├── timeslots/
│   │   │   ├── mod.rs
│   │   │   ├── manager.rs       # Timeslot management
│   │   │   └── calculator.rs    # Target flavor calculation
│   │   ├── monitor/
│   │   │   ├── mod.rs
│   │   │   └── queue.rs         # Queue monitoring
│   │   └── dev_ui/
│   │       └── templates/
│   └── tests/
│       └── integration_tests.rs
│
├── wkmp-ai/                      # Audio Ingest binary (Full only)
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs
│   │   ├── config.rs
│   │   ├── server.rs
│   │   ├── api/
│   │   │   ├── mod.rs
│   │   │   ├── scan.rs          # File scanning endpoints
│   │   │   ├── identify.rs      # MusicBrainz identification
│   │   │   ├── characterize.rs  # Flavor analysis
│   │   │   └── segment.rs       # Passage segmentation
│   │   ├── scanner/
│   │   │   ├── mod.rs
│   │   │   ├── filesystem.rs    # Directory scanning
│   │   │   └── metadata.rs      # Metadata extraction
│   │   ├── external/
│   │   │   ├── mod.rs
│   │   │   ├── musicbrainz.rs   # MusicBrainz client
│   │   │   ├── acousticbrainz.rs # AcousticBrainz client
│   │   │   ├── acoustid.rs      # AcoustID/Chromaprint
│   │   │   └── essentia.rs      # Essentia FFI bindings
│   │   ├── segmentation/
│   │   │   ├── mod.rs
│   │   │   ├── silence.rs       # Silence detection
│   │   │   └── boundaries.rs    # Boundary detection
│   │   └── workflow_ui/
│   │       └── templates/       # Guided workflow UI
│   └── tests/
│       └── integration_tests.rs
│
├── migrations/                   # Database migrations (shared)
│   ├── 001_initial_schema.sql
│   ├── 002_add_module_config.sql
│   └── ...
│
├── docs/                         # Documentation
│   ├── requirements.md
│   ├── architecture.md
│   ├── database_schema.md
│   ├── api_design.md
│   ├── deployment.md
│   ├── implementation_order.md
│   └── ...
│
├── scripts/                      # Build and utility scripts
│   ├── build_all.sh
│   ├── build_full.sh
│   ├── build_lite.sh
│   ├── build_minimal.sh
│   ├── run_tests.sh
│   └── setup_dev.sh
│
└── target/                       # Build artifacts (gitignored)
    ├── debug/
    └── release/
```

---

## Workspace Configuration

### Root `Cargo.toml`

```toml
[workspace]
resolver = "2"

members = [
    "common",
    "wkmp-ap",
    "wkmp-ui",
    "wkmp-le",
    "wkmp-pd",
    "wkmp-ai",
    "wkmp-dr",
]

# Shared dependencies across workspace
[workspace.dependencies]
tokio = { version = "1.35", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
uuid = { version = "1.6", features = ["v4", "serde"] }
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "sqlite", "uuid", "chrono", "json"] }
axum = "0.7"
tower = "0.4"
tower-http = { version = "0.5", features = ["fs", "cors", "trace"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
anyhow = "1.0"
thiserror = "1.0"
chrono = { version = "0.4", features = ["serde"] }
reqwest = { version = "0.11", features = ["json"] }

# Version differentiation is achieved by packaging different binaries
# No feature flags or conditional compilation required
[workspace.metadata.versions]
# Full version: Package all 6 binaries (wkmp-ap, wkmp-ui, wkmp-le, wkmp-pd, wkmp-ai, wkmp-dr)
# Lite version: Package 3 binaries (wkmp-ap, wkmp-ui, wkmp-pd)
# Minimal version: Package 2 binaries (wkmp-ap, wkmp-ui)
```

---

## Common Library (`common/Cargo.toml`)

```toml
[package]
name = "wkmp-common"
version = "0.1.0"
edition = "2021"

[dependencies]
# Use workspace dependencies
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
uuid = { workspace = true }
sqlx = { workspace = true }
chrono = { workspace = true }
anyhow = { workspace = true }
thiserror = { workspace = true }
tracing = { workspace = true }

# Common-specific dependencies
bincode = "1.3"

[dev-dependencies]
tokio-test = "0.4"
```

### Common Library Structure

**`common/src/lib.rs`:**
```rust
pub mod db;
pub mod events;
pub mod api;
pub mod config;
pub mod flavor;
pub mod cooldown;
pub mod uuid;
pub mod time;
pub mod error;

// Re-export commonly used types
pub use error::{Error, Result};
pub use uuid::generate_uuid;
```

**Key Shared Components:**

1. **Database Models** (`common/src/db/models.rs`)
   ```rust
   use serde::{Deserialize, Serialize};
   use uuid::Uuid;

   #[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
   pub struct Passage {
       pub guid: Uuid,
       pub file_id: Uuid,
       pub title: Option<String>,
       pub user_title: Option<String>,
       pub artist: Option<String>,
       pub album: Option<String>,
       pub musical_flavor_vector: Option<serde_json::Value>,
       // ... other fields
   }

   #[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
   pub struct Song {
       pub guid: Uuid,
       pub recording_mbid: String,
       pub base_probability: f64,
       pub min_cooldown: i64,
       pub ramping_cooldown: i64,
       pub last_played_at: Option<chrono::DateTime<chrono::Utc>>,
       // ... other fields
   }

   // ... other models
   ```

2. **Event Types** (`common/src/events/types.rs`)
   ```rust
   use serde::{Deserialize, Serialize};
   use uuid::Uuid;

   #[derive(Debug, Clone, Serialize, Deserialize)]
   #[serde(tag = "type", rename_all = "snake_case")]
   pub enum WkmpEvent {
       PassageStarted {
           passage_id: Uuid,
           timestamp: chrono::DateTime<chrono::Utc>,
       },
       PassageCompleted {
           passage_id: Uuid,
           timestamp: chrono::DateTime<chrono::Utc>,
       },
       PlaybackStateChanged {
           state: PlaybackState,
       },
       PlaybackProgress {
           position: f64,
           duration: f64,
       },
       QueueChanged {
           queue_length: usize,
       },
       VolumeChanged {
           level: u8,
       },
       CurrentSongChanged {
           song_id: Uuid,
       },
       TimeslotChanged {
           timeslot_id: Uuid,
           timeslot_name: String,
       },
       // ... other events
   }

   #[derive(Debug, Clone, Serialize, Deserialize)]
   #[serde(rename_all = "lowercase")]
   pub enum PlaybackState {
       Playing,
       Paused,
   }
   ```

3. **Module Configuration** (`common/src/config/module.rs`)
   ```rust
   use sqlx::SqlitePool;
   use uuid::Uuid;

   #[derive(Debug, Clone)]
   pub struct ModuleConfig {
       pub module_name: String,
       pub host: String,
       pub port: u16,
       pub enabled: bool,
   }

   impl ModuleConfig {
       /// Load module configuration from database
       pub async fn load(
           db: &SqlitePool,
           module_name: &str,
       ) -> Result<Self, sqlx::Error> {
           sqlx::query_as!(
               ModuleConfig,
               r#"
               SELECT module_name, host, port, enabled
               FROM module_config
               WHERE module_name = ?
               "#,
               module_name
           )
           .fetch_one(db)
           .await
       }

       /// Get URL for this module
       pub fn url(&self) -> String {
           format!("http://{}:{}", self.host, self.port)
       }
   }

   /// Load all module configurations
   pub async fn load_all_modules(db: &SqlitePool) -> Result<Vec<ModuleConfig>, sqlx::Error> {
       sqlx::query_as!(
           ModuleConfig,
           "SELECT module_name, host, port, enabled FROM module_config"
       )
       .fetch_all(db)
       .await
   }
   ```

4. **Flavor Calculations** (`common/src/flavor/distance.rs`)
   ```rust
   use serde_json::Value;

   /// Calculate squared Euclidean distance between two flavor vectors
   pub fn calculate_distance(
       flavor_a: &Value,
       flavor_b: &Value,
   ) -> f64 {
       // Implementation of squared Euclidean distance
       // See musical_flavor.md for complete algorithm
       todo!()
   }

   /// Calculate weighted centroid of multiple flavor vectors
   pub fn calculate_centroid(
       flavors: &[(Value, f64)], // (flavor, weight) pairs
   ) -> Value {
       // Implementation of weighted centroid
       todo!()
   }
   ```

5. **Cooldown Logic** (`common/src/cooldown/calculator.rs`)
   ```rust
   use chrono::{DateTime, Utc};

   pub struct CooldownConfig {
       pub min_cooldown: i64,      // seconds
       pub ramping_cooldown: i64,  // seconds
   }

   /// Calculate cooldown multiplier (0.0 to 1.0)
   pub fn calculate_multiplier(
       last_played_at: Option<DateTime<Utc>>,
       config: &CooldownConfig,
       now: DateTime<Utc>,
   ) -> f64 {
       // Implementation of cooldown calculation
       // See program_director.md for complete algorithm
       todo!()
   }
   ```

---

## Module Binary Structure

### Audio Player (`wkmp-ap/Cargo.toml`)

```toml
[package]
name = "wkmp-ap"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "wkmp-ap"
path = "src/main.rs"

[dependencies]
# Workspace dependencies
wkmp-common = { path = "../common" }
tokio = { workspace = true }
axum = { workspace = true }
tower = { workspace = true }
tower-http = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
sqlx = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
anyhow = { workspace = true }

# Module-specific dependencies
symphonia = "0.5"  # Audio decoding
rubato = "0.15"    # Sample rate conversion
cpal = "0.15"      # Audio output
toml = "0.8"
futures = "0.3"

[features]
# Optional features for version builds
default = []
```

### User Interface (`wkmp-ui/Cargo.toml`)

```toml
[package]
name = "wkmp-ui"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "wkmp-ui"
path = "src/main.rs"

[dependencies]
wkmp-common = { path = "../common" }
tokio = { workspace = true }
axum = { workspace = true }
tower = { workspace = true }
tower-http = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
sqlx = { workspace = true }
reqwest = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
anyhow = { workspace = true }

# UI-specific dependencies
argon2 = "0.5"  # Password hashing
toml = "0.8"
```

### Lyric Editor (`wkmp-le/Cargo.toml`)

```toml
[package]
name = "wkmp-le"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "wkmp-le"
path = "src/main.rs"

[dependencies]
wkmp-common = { path = "../common" }
tokio = { workspace = true }
axum = { workspace = true }
tower = { workspace = true }
tower-http = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
sqlx = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
anyhow = { workspace = true }

# Lyric Editor-specific dependencies
toml = "0.8"
# Platform-specific webview libraries (choose based on target):
# - webkit2gtk (Linux)
# - webview2-com (Windows)
# - cocoa-webkit (macOS)
```

### Database Review (`wkmp-dr/Cargo.toml`)

```toml
[package]
name = "wkmp-dr"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "wkmp-dr"
path = "src/main.rs"

[lib]
path = "src/lib.rs"

[dependencies]
wkmp-common = { path = "../wkmp-common" }
tokio = { workspace = true }
axum = { workspace = true }
tower = { workspace = true }
tower-http = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
sqlx = { workspace = true }
uuid = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
anyhow = { workspace = true }
thiserror = { workspace = true }
```

### Program Director (`wkmp-pd/Cargo.toml`)

```toml
[package]
name = "wkmp-pd"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "wkmp-pd"
path = "src/main.rs"

[dependencies]
wkmp-common = { path = "../common" }
tokio = { workspace = true }
axum = { workspace = true }
reqwest = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
sqlx = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
anyhow = { workspace = true }

# Director-specific dependencies
rand = "0.8"  # Weighted random selection
toml = "0.8"
```

### Audio Ingest (`wkmp-ai/Cargo.toml`)

```toml
[package]
name = "wkmp-ai"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "wkmp-ai"
path = "src/main.rs"

[dependencies]
wkmp-common = { path = "../common" }
tokio = { workspace = true }
axum = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
sqlx = { workspace = true }
reqwest = { workspace = true }
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
anyhow = { workspace = true }

# Ingest-specific dependencies
walkdir = "2.4"
sha2 = "0.10"
id3 = "1.11"
metaflac = "0.2"
mp4ameta = "0.11"
chromaprint = "0.2"  # AcoustID fingerprinting
toml = "0.8"

# Essentia FFI (optional, full version only)
[dependencies.essentia]
version = "0.1"
optional = true

[features]
default = []
full = ["essentia"]  # Enable for Full version
```

---

## Building the Workspace

### Build All Modules

```bash
# Build all modules in debug mode
cargo build

# Build all modules in release mode
cargo build --release

# Build specific module
cargo build -p wkmp-ap
cargo build -p wkmp-ui --release
```

### Version-Specific Builds

**Full Version (all 6 modules):**
```bash
cargo build --release -p wkmp-ap -p wkmp-ui -p wkmp-pd -p wkmp-ai -p wkmp-le -p wkmp-dr --features wkmp-ai/full
```

**Lite Version (3 modules):**
```bash
cargo build --release -p wkmp-ap -p wkmp-ui -p wkmp-pd
```

**Minimal Version (2 modules):**
```bash
cargo build --release -p wkmp-ap -p wkmp-ui
```

### Build Scripts

All modules are built identically with no conditional compilation. Version differentiation is achieved by packaging different subsets of binaries.

**`scripts/build_all.sh`:**
```bash
#!/bin/bash
set -e

echo "Building all WKMP modules..."

# Build all binaries in release mode
cargo build --release

echo "All binaries available in target/release/"
ls -lh target/release/wkmp-*
```

**`scripts/package_full.sh`:**
```bash
#!/bin/bash
set -e

echo "Packaging WKMP Full version..."

# Build all binaries
./scripts/build_all.sh

# Create distribution directory
mkdir -p dist/full
cp target/release/wkmp-ap dist/full/
cp target/release/wkmp-ui dist/full/
cp target/release/wkmp-le dist/full/
cp target/release/wkmp-pd dist/full/
cp target/release/wkmp-ai dist/full/
cp target/release/wkmp-dr dist/full/

echo "Full version packaged in dist/full/"
```

**`scripts/package_lite.sh`:**
```bash
#!/bin/bash
set -e

echo "Packaging WKMP Lite version..."

# Build all binaries
./scripts/build_all.sh

# Create distribution directory
mkdir -p dist/lite
cp target/release/wkmp-ap dist/lite/
cp target/release/wkmp-ui dist/lite/
cp target/release/wkmp-pd dist/lite/

echo "Lite version packaged in dist/lite/"
```

**`scripts/package_minimal.sh`:**
```bash
#!/bin/bash
set -e

echo "Packaging WKMP Minimal version..."

# Build all binaries
./scripts/build_all.sh

# Create distribution directory
mkdir -p dist/minimal
cp target/release/wkmp-ap dist/minimal/
cp target/release/wkmp-ui dist/minimal/

echo "Minimal version packaged in dist/minimal/"
```

---

## Running the Application

### Development Mode

```bash
# Terminal 1: Audio Player
cargo run -p wkmp-ap

# Terminal 2: Program Director
cargo run -p wkmp-pd

# Terminal 3: Audio Ingest (Full only)
cargo run -p wkmp-ai

# Terminal 4: User Interface
cargo run -p wkmp-ui
```

### Using Configuration Files

Each module reads its config file from standard location:
```bash
# Linux
~/.config/wkmp/audio-player.toml
~/.config/wkmp/user-interface.toml
~/.config/wkmp/program-director.toml
~/.config/wkmp/audio-ingest.toml
~/.config/wkmp/lyrics-editor.toml

# Override with environment variable
WKMP_ROOT_FOLDER=/custom/path cargo run -p wkmp-ap
```

---

## Testing

### Run All Tests

```bash
# Run all tests in workspace
cargo test

# Run tests for specific module
cargo test -p wkmp-common
cargo test -p wkmp-ap

# Run integration tests only
cargo test --test integration_tests
```

### Test Organization

**Unit tests** in each module:
```rust
// wkmp-pd/src/selection/algorithm.rs
pub fn select_passage(candidates: &[Candidate]) -> Option<Uuid> {
    // Implementation
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selection_with_empty_candidates() {
        assert_eq!(select_passage(&[]), None);
    }
}
```

**Integration tests** in `tests/` directory:
```rust
// wkmp-ap/tests/integration_tests.rs
use wkmp_ap;

#[tokio::test]
async fn test_playback_api() {
    // Test HTTP endpoints
}
```

**Common library tests:**
```rust
// common/tests/integration_tests.rs
use wkmp_common::flavor::distance;

#[test]
fn test_flavor_distance_calculation() {
    // Test flavor distance
}
```

---

## Dependency Management

### Adding Dependencies

**Workspace-wide dependency:**
```toml
# Edit root Cargo.toml
[workspace.dependencies]
new-crate = "1.0"

# Use in module
[dependencies]
new-crate = { workspace = true }
```

**Module-specific dependency:**
```toml
# Edit module Cargo.toml (e.g., wkmp-ap/Cargo.toml)
[dependencies]
symphonia = "0.5"  # Only needed by Audio Player (audio decoding)
rubato = "0.15"    # Only needed by Audio Player (sample rate conversion)
cpal = "0.15"      # Only needed by Audio Player (audio output)
```

### Updating Dependencies

```bash
# Update all dependencies
cargo update

# Update specific crate
cargo update serde

# Check for outdated dependencies
cargo outdated
```

---

## Code Organization Guidelines

### What Goes in `common/`?

✅ **Should be in common:**
- Database models (Passage, Song, Artist, etc.)
- Event types (WkmpEvent enum)
- API request/response types
- Flavor calculation algorithms
- Cooldown calculation logic
- UUID and timestamp utilities
- Error types used across modules
- Module configuration loading

❌ **Should NOT be in common:**
- HTTP server setup (module-specific)
- Audio pipeline code (Audio Player only)
- Password hashing (User Interface only)
- Selection algorithm (Program Director only)
- File scanning (Audio Ingest only)
- Module-specific configuration

### Module Boundaries

Each module binary should be:
- **Self-contained**: Can run independently
- **Minimal dependencies**: Only depend on common + essential crates
- **Single responsibility**: Each module has one clear purpose
- **Testable**: Integration tests for HTTP APIs

---

## Migration Management

Database migrations are **shared** across all modules:

```
migrations/
├── 001_initial_schema.sql
├── 002_add_module_config.sql
├── 003_add_users_table.sql
└── ...
```

Each module runs migrations on startup:
```rust
// In each module's main.rs
use sqlx::SqlitePool;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let database_url = std::env::var("DATABASE_URL")?;
    let pool = SqlitePool::connect(&database_url).await?;

    // Run migrations
    sqlx::migrate!("../migrations")
        .run(&pool)
        .await?;

    // Start module...
}
```

---

## Development Workflow

### Initial Setup

```bash
# Clone repository
git clone https://github.com/username/mcrhythm
cd mcrhythm

# Install dependencies
cargo build

# Set up database
# For development, database is in current directory
# In production, database is in the configured root folder
export DATABASE_URL="sqlite://wkmp.db"
cargo install sqlx-cli --no-default-features --features sqlite
sqlx database create
sqlx migrate run --source migrations

# Run tests
cargo test

# Start development servers (separate terminals)
cargo run -p wkmp-ap
cargo run -p wkmp-ui
cargo run -p wkmp-pd
```

### Development Iteration

```bash
# Make changes to common library
# Rebuild all modules that depend on it
cargo build

# Make changes to specific module
# Only that module rebuilds
cargo build -p wkmp-ap

# Run tests frequently
cargo test -p wkmp-common
cargo test -p wkmp-ap
```

### Code Formatting and Linting

```bash
# Format all code
cargo fmt

# Check formatting without modifying
cargo fmt --check

# Run Clippy (linter)
cargo clippy --all-targets --all-features

# Fix Clippy warnings automatically
cargo clippy --fix
```

---

## Continuous Integration

### GitHub Actions Workflow

**.github/workflows/ci.yml:**
```yaml
name: CI

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Cache dependencies
        uses: actions/cache@v3
        with:
          path: ~/.cargo
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}

      - name: Run tests
        run: cargo test --all

      - name: Check formatting
        run: cargo fmt --check

      - name: Run Clippy
        run: cargo clippy --all-targets --all-features -- -D warnings

  build:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        version: [full, lite, minimal]
    steps:
      - uses: actions/checkout@v3

      - name: Build ${{ matrix.version }}
        run: ./scripts/build_${{ matrix.version }}.sh

      - name: Upload artifacts
        uses: actions/upload-artifact@v3
        with:
          name: wkmp-${{ matrix.version }}-linux
          path: target/release/wkmp-*
```

---

## Binary Distribution

After building, distribute binaries with appropriate files:

**Full version:**
```
wkmp-full-v0.1.0-linux/
├── bin/
│   ├── wkmp-ap
│   ├── wkmp-ui
│   ├── wkmp-pd
│   ├── wkmp-le
│   ├── wkmp-ai
│   └── wkmp-dr
├── migrations/
│   └── *.sql
├── README.md
└── LICENSE
```

**Lite version:**
```
wkmp-lite-v0.1.0-linux/
├── bin/
│   ├── wkmp-ap
│   ├── wkmp-ui
│   └── wkmp-pd
├── migrations/
│   └── *.sql
├── README.md
└── LICENSE
```

**Minimal version:**
```
wkmp-minimal-v0.1.0-linux/
├── bin/
│   ├── wkmp-ap
│   └── wkmp-ui
├── migrations/
│   └── *.sql
├── README.md
└── LICENSE
```

---

## Performance Considerations

### Compilation Time

- **Common changes**: Trigger rebuild of all dependent modules
- **Module-specific changes**: Only that module rebuilds
- **Parallel compilation**: Cargo builds modules in parallel by default

### Binary Size

```bash
# Check binary sizes
ls -lh target/release/wkmp-*

# Strip debug symbols (smaller binaries)
strip target/release/wkmp-*

# Use LTO (Link-Time Optimization) for smaller, faster binaries
# Add to root Cargo.toml:
[profile.release]
lto = true
codegen-units = 1
```

### Runtime Performance

- Shared code in `common/` has no runtime overhead (inlined by compiler)
- Each module is a separate process (no shared memory)
- HTTP communication between modules (~2-5ms latency)

---

## Summary

The Cargo workspace structure provides:

✅ **Code reuse**: Database models, events, utilities shared via `common/`
✅ **Unified builds**: Single `cargo build` command
✅ **Dependency management**: Workspace-level dependency versions
✅ **Modular binaries**: Each module is independent
✅ **Version flexibility**: Build Full/Lite/Minimal from same codebase
✅ **Testability**: Unit and integration tests for all components

This structure supports WKMP's microservices architecture while maintaining a clean, maintainable codebase.

----
End of document - WKMP Project Structure
