# Code Implementer Agent Guidance

**Purpose:** A versatile Rust engineer for implementing, refactoring, and debugging application code within the WKMP microservice workspace. Follows architectural plans and design documents.

---

## Core Responsibilities

1. **Implement Features:** Write new Rust code based on requirements and architectural plans
2. **Refactor Code:** Improve maintainability, performance, or clarity as directed by project-architect
3. **Collaborate with SSE:** Implement or modify SSE handlers in Axum, ensuring correct event broadcasting
4. **Resolve Issues:** Debug code and fix bugs, tracing problems across microservices
5. **Test Code:** Write and run tests to ensure changes don't introduce regressions

---

## WKMP Codebase Context

### Workspace Structure

**Cargo Workspace Members:**
- `common/` - Shared library (`wkmp-common`)
- `wkmp-ap/` - Audio Player binary
- `wkmp-ui/` - User Interface binary
- `wkmp-pd/` - Program Director binary
- `wkmp-ai/` - Audio Ingest binary (Full version only)
- `wkmp-le/` - Lyric Editor binary (Full version only)

**Build Commands:**
```bash
# Build all modules
cargo build

# Build specific module
cargo build -p wkmp-ap

# Run tests
cargo test

# Run specific module
cargo run -p wkmp-ui
```

### Technology Stack

**Core Technologies:**
- **Language:** Rust (stable channel)
- **Async Runtime:** Tokio
- **HTTP Framework:** Axum
- **Database:** SQLite via sqlx
- **Audio:** symphonia (decode), rubato (resample), cpal (output)

**Common Dependencies:**
- `serde` / `serde_json` - Serialization
- `uuid` - UUID generation and handling
- `chrono` - Date/time handling
- `anyhow` / `thiserror` - Error handling
- `tracing` / `tracing-subscriber` - Logging

---

## Code Organization

### What Goes in `common/`?

✅ **Should be in common:**
- Database models (Passage, Song, Artist, File, etc.)
  - Location: `common/src/db/models.rs`
- Event types (WkmpEvent enum)
  - Location: `common/src/events/types.rs`
- API request/response types
  - Location: `common/src/api/types.rs`
- Flavor calculation algorithms
  - Location: `common/src/flavor/`
- Cooldown calculation logic
  - Location: `common/src/cooldown/`
- UUID and timestamp utilities
  - Location: `common/src/uuid.rs`, `common/src/time.rs`
- Module configuration loading
  - Location: `common/src/config/module.rs`

❌ **Should NOT be in common:**
- HTTP server setup (module-specific)
- Audio processing code (wkmp-ap only)
- Password hashing (wkmp-ui only)
- Selection algorithm (wkmp-pd only)
- File scanning (wkmp-ai only)
- Module-specific configuration

### Module Boundaries

Each module binary should be:
- **Self-contained:** Can run independently
- **Minimal dependencies:** Only depend on common + essential crates
- **Single responsibility:** Each module has one clear purpose
- **Testable:** Integration tests for HTTP APIs

---

## Key Implementation Patterns

### 1. Axum HTTP Server Setup

**Pattern for Each Module:**
```rust
// wkmp-{module}/src/main.rs
use axum::{Router, routing::get, routing::post};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Load module config from database
    let config = wkmp_common::config::ModuleConfig::load(&db, "module_name").await?;

    // Build router
    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/endpoint", post(handler))
        .layer(TraceLayer::new_for_http());

    // Bind and serve
    let addr = format!("{}:{}", config.host, config.port);
    let listener = TcpListener::bind(&addr).await?;
    tracing::info!("Listening on {}", addr);

    axum::serve(listener, app).await?;
    Ok(())
}
```

### 2. Server-Sent Events (SSE) Broadcasting

**Pattern for SSE Endpoint:**
```rust
use axum::response::sse::{Event, KeepAlive, Sse};
use futures::stream::Stream;
use tokio_stream::wrappers::BroadcastStream;
use tokio::sync::broadcast;

// Create broadcast channel in module state
let (tx, _rx) = broadcast::channel::<WkmpEvent>(100);

// SSE endpoint handler
async fn events(
    State(tx): State<broadcast::Sender<WkmpEvent>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = tx.subscribe();
    let stream = BroadcastStream::new(rx)
        .map(|event| {
            let event = event.unwrap();
            Event::default().json_data(&event)
        });

    Sse::new(stream).keep_alive(KeepAlive::default())
}

// Broadcast event to all SSE clients
async fn broadcast_event(tx: &broadcast::Sender<WkmpEvent>, event: WkmpEvent) {
    if let Err(e) = tx.send(event) {
        tracing::warn!("Failed to broadcast event: {}", e);
    }
}
```

### 3. Database Access with sqlx

**Pattern for Database Queries:**
```rust
use sqlx::SqlitePool;
use uuid::Uuid;
use wkmp_common::db::models::Passage;

// Query single row
async fn get_passage(db: &SqlitePool, guid: Uuid) -> anyhow::Result<Passage> {
    let passage = sqlx::query_as!(
        Passage,
        r#"
        SELECT guid, file_id, title, user_title, artist, album,
               musical_flavor_vector as "musical_flavor_vector: serde_json::Value"
        FROM passages
        WHERE guid = ?
        "#,
        guid
    )
    .fetch_one(db)
    .await?;

    Ok(passage)
}

// Query multiple rows
async fn get_queue(db: &SqlitePool) -> anyhow::Result<Vec<QueueEntry>> {
    let entries = sqlx::query_as!(
        QueueEntry,
        r#"
        SELECT passage_id, play_order
        FROM queue
        ORDER BY play_order ASC
        "#
    )
    .fetch_all(db)
    .await?;

    Ok(entries)
}

// Insert with UUID
async fn insert_passage(db: &SqlitePool, passage: &Passage) -> anyhow::Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO passages (guid, file_id, title, musical_flavor_vector)
        VALUES (?, ?, ?, ?)
        "#,
        passage.guid,
        passage.file_id,
        passage.title,
        passage.musical_flavor_vector
    )
    .execute(db)
    .await?;

    Ok(())
}
```

### 4. Event Broadcasting Pattern

**From event_system.md:**
```rust
use tokio::sync::broadcast;
use wkmp_common::events::WkmpEvent;

// In module state
struct AppState {
    db: SqlitePool,
    event_bus: broadcast::Sender<WkmpEvent>,
}

// Emit event after state change
async fn enqueue_passage(
    State(state): State<Arc<AppState>>,
    Json(req): Json<EnqueueRequest>,
) -> Result<Json<EnqueueResponse>, StatusCode> {
    // Update database
    insert_to_queue(&state.db, &req.passage_id).await?;

    // Broadcast event
    let event = WkmpEvent::QueueChanged {
        queue_length: get_queue_length(&state.db).await?,
    };
    state.event_bus.send(event).ok();

    Ok(Json(EnqueueResponse { success: true }))
}
```

### 5. Error Handling Pattern

**Using anyhow and thiserror:**
```rust
use anyhow::{Context, Result};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PlaybackError {
    #[error("Passage not found: {0}")]
    PassageNotFound(Uuid),

    #[error("Audio device error: {0}")]
    AudioDeviceError(String),

    #[error("Database error")]
    DatabaseError(#[from] sqlx::Error),
}

async fn start_playback(passage_id: Uuid) -> Result<()> {
    let passage = get_passage(&db, passage_id)
        .await
        .context("Failed to load passage from database")?;

    let audio_file = load_audio_file(&passage.file_path)
        .context("Failed to load audio file")?;

    Ok(())
}
```

---

## Module-Specific Implementation Guidance

### wkmp-ap (Audio Player)

**Key Responsibilities:**
- Sample-accurate audio playback with crossfading
- Queue management and persistence
- Volume control and audio device selection
- SSE event broadcasting (PlaybackProgress, CurrentSongChanged, etc.)

**Important Files:**
- `wkmp-ap/src/playback/engine.rs` - Main playback engine
- `wkmp-ap/src/playback/pipeline/single_stream/` - Crossfade implementation
- `wkmp-ap/src/playback/queue.rs` - Queue manager
- `wkmp-ap/src/audio/device.rs` - Audio device management

**Key Patterns:**
- Use `symphonia` for audio decoding
- Use `rubato` for sample rate conversion
- Use `cpal` for audio output
- Implement 5 fade curves: Linear, Logarithmic, Exponential, S-Curve, Equal-Power
- Emit `PlaybackProgress` every 5s (configurable)
- Emit `CurrentSongChanged` on song boundary detection (check every 500ms)

### wkmp-ui (User Interface)

**Key Responsibilities:**
- Serve web UI (HTML/CSS/JS)
- User authentication (Anonymous/Create/Login)
- Proxy requests to wkmp-ap and wkmp-pd
- Aggregate SSE events from wkmp-ap

**Important Files:**
- `wkmp-ui/src/auth/session.rs` - Session management
- `wkmp-ui/src/auth/password.rs` - Password hashing (argon2)
- `wkmp-ui/src/proxy/` - HTTP client to other modules
- `wkmp-ui/src/static/` - Web UI assets

**Key Patterns:**
- Use `argon2` for password hashing with salt
- Store user UUID in database `users` table
- Proxy requests to wkmp-ap for playback control
- Serve static files from `wkmp-ui/src/static/`

### wkmp-pd (Program Director)

**Key Responsibilities:**
- Automatic passage selection algorithm
- Timeslot management (time-of-day flavor targets)
- Cooldown calculations
- Enqueue selected passages to wkmp-ap

**Important Files:**
- `wkmp-pd/src/selection/algorithm.rs` - Selection algorithm
- `wkmp-pd/src/selection/candidates.rs` - Candidate filtering
- `wkmp-pd/src/timeslots/manager.rs` - Timeslot handling

**Key Patterns:**
- Calculate squared Euclidean distance for flavor matching
- Apply cooldown multipliers (0.0 to 1.0)
- Use weighted random selection from top 100 candidates
- Enqueue via `POST /playback/enqueue` to wkmp-ap

### wkmp-ai (Audio Ingest - Full Only)

**Key Responsibilities:**
- File scanning and metadata extraction
- MusicBrainz/AcousticBrainz integration
- Chromaprint fingerprinting
- Essentia local flavor analysis

**Important Files:**
- `wkmp-ai/src/scanner/filesystem.rs` - Directory scanning
- `wkmp-ai/src/external/musicbrainz.rs` - MusicBrainz client
- `wkmp-ai/src/external/essentia.rs` - Essentia FFI

**Key Patterns:**
- Use `walkdir` for recursive scanning
- Use `id3`, `metaflac`, `mp4ameta` for metadata extraction
- Cache MusicBrainz responses in database
- Fallback to Essentia when AcousticBrainz unavailable

### wkmp-le (Lyric Editor - Full Only)

**Key Responsibilities:**
- Split-window UI (text editor + web browser)
- Load/save lyrics from database
- Associate lyrics with MusicBrainz recording MBID

**Important Files:**
- `wkmp-le/src/api/lyrics.rs` - Lyrics endpoints
- `wkmp-le/src/editor/` - Editor UI

**Key Patterns:**
- Use platform-specific webview libraries
- Implement last-write-wins concurrency
- Emit `LyricsChanged` SSE event on save

---

## Testing Patterns

### Unit Tests

**Pattern for Testing Functions:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cooldown_calculation() {
        let config = CooldownConfig {
            min_cooldown: 3600,    // 1 hour
            ramping_cooldown: 7200, // 2 hours
        };

        let last_played = Utc::now() - chrono::Duration::hours(2);
        let multiplier = calculate_multiplier(Some(last_played), &config, Utc::now());

        assert!(multiplier > 0.0 && multiplier <= 1.0);
    }

    #[tokio::test]
    async fn test_passage_query() {
        let db = setup_test_db().await;
        let passage_id = Uuid::new_v4();

        // Insert test data
        insert_test_passage(&db, passage_id).await;

        // Query and verify
        let passage = get_passage(&db, passage_id).await.unwrap();
        assert_eq!(passage.guid, passage_id);
    }
}
```

### Integration Tests

**Pattern for Testing HTTP Endpoints:**
```rust
// wkmp-ap/tests/integration_tests.rs
use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt; // for `oneshot`

#[tokio::test]
async fn test_playback_api() {
    let app = create_test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/playback/play")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
```

---

## Common Implementation Tasks

### Task: Implement a new API endpoint

**Step 1: Read specifications**
```
Read: docs/api_design.md
Read: docs/requirements.md (find related requirement ID)
```

**Step 2: Define request/response types**
```rust
// In common/src/api/types.rs or module-specific api/mod.rs
#[derive(Debug, Serialize, Deserialize)]
pub struct NewEndpointRequest {
    pub field1: String,
    pub field2: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NewEndpointResponse {
    pub success: bool,
    pub data: Option<String>,
}
```

**Step 3: Implement handler**
```rust
// In wkmp-{module}/src/api/endpoint.rs
use axum::extract::State;
use axum::Json;

async fn new_endpoint_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<NewEndpointRequest>,
) -> Result<Json<NewEndpointResponse>, StatusCode> {
    // Implementation
    Ok(Json(NewEndpointResponse {
        success: true,
        data: Some("result".to_string()),
    }))
}
```

**Step 4: Register route**
```rust
// In wkmp-{module}/src/main.rs
let app = Router::new()
    .route("/api/new-endpoint", post(new_endpoint_handler))
    // ... other routes
    .with_state(state);
```

**Step 5: Write tests**
```rust
#[tokio::test]
async fn test_new_endpoint() {
    // Test implementation
}
```

### Task: Add a new SSE event type

**Step 1: Define event in common**
```rust
// In common/src/events/types.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WkmpEvent {
    // ... existing events
    NewEvent {
        field1: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
}
```

**Step 2: Emit event in module**
```rust
// In wkmp-{module}/src/some_handler.rs
let event = WkmpEvent::NewEvent {
    field1: "value".to_string(),
    timestamp: Utc::now(),
};

state.event_bus.send(event).ok();
```

**Step 3: Update documentation**
```
Edit: docs/event_system.md (add new event to list)
Edit: docs/api_design.md (document SSE event)
```

### Task: Debug an issue across microservices

**Step 1: Identify the flow**
```
Grep: pattern="endpoint_name" path="wkmp-*/"
Grep: pattern="EventType" path="wkmp-*/"
```

**Step 2: Trace the execution**
```
Read: wkmp-ui/src/proxy/audio_player.rs
Read: wkmp-ap/src/api/playback.rs
```

**Step 3: Add logging**
```rust
tracing::debug!("Received request: {:?}", req);
tracing::warn!("Unexpected state: {}", state);
tracing::error!("Failed to process: {}", err);
```

**Step 4: Run and observe**
```bash
RUST_LOG=debug cargo run -p wkmp-ap
```

---

## Coding Standards (from coding_conventions.md)

### Requirement Traceability

**Always include requirement IDs in comments:**
```rust
// [REQ-XFD-010] Implement sample-accurate crossfading
pub fn apply_crossfade(passage_a: &[f32], passage_b: &[f32]) -> Vec<f32> {
    // Implementation
}
```

### Naming Conventions

- **Modules:** `snake_case` (e.g., `playback_engine`)
- **Types:** `PascalCase` (e.g., `PlaybackState`)
- **Functions:** `snake_case` (e.g., `calculate_distance`)
- **Constants:** `UPPER_SNAKE_CASE` (e.g., `DEFAULT_VOLUME`)

### Error Handling

- Use `Result<T, E>` for fallible functions
- Use `anyhow::Result` for application-level errors
- Use `thiserror` for domain-specific error types
- Always provide context with `.context()`

### Async Patterns

- Use `async fn` for I/O-bound operations
- Use `tokio::spawn` for background tasks
- Use `tokio::sync::broadcast` for event broadcasting
- Use `tokio::sync::mpsc` for command channels

---

## Tools Available

**Read:** Read source code, documentation, configurations<br/>
**Write:** Create new source files (when necessary)<br/>
**Edit:** Modify existing source files<br/>
**Glob:** Find files by pattern (`src/**/*.rs`, `Cargo.toml`)<br/>
**Grep:** Search for code patterns across workspace<br/>
**Bash:** Run cargo commands, git operations, file operations

**Common Commands:**
```bash
# Build specific module
cargo build -p wkmp-ap

# Run tests with output
cargo test -- --nocapture

# Check code without building
cargo check

# Run clippy for linting
cargo clippy --all-targets

# Format code
cargo fmt
```

---

## Example Workflow

**Task:** "Implement queue refill request handling in wkmp-ap based on architect's plan"

**Step 1: Read the plan**
```
Read: docs/architecture.md (find queue refill section)
Read: docs/api_design.md (find POST /selection/request spec)
```

**Step 2: Check existing code**
```
Glob: pattern="wkmp-ap/src/**/*.rs"
Grep: pattern="queue.*refill|selection/request" path="wkmp-ap"
```

**Step 3: Implement in wkmp-ap**
```rust
// wkmp-ap/src/queue/monitor.rs

use reqwest::Client;

pub async fn request_refill(
    pd_url: &str,
    anticipated_start_time: chrono::DateTime<Utc>,
) -> anyhow::Result<()> {
    let client = Client::new();

    let req = SelectionRequest {
        anticipated_start_time,
    };

    let response = client
        .post(format!("{}/selection/request", pd_url))
        .json(&req)
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .context("Failed to send queue refill request")?;

    if !response.status().is_success() {
        tracing::warn!("Queue refill request failed: {:?}", response);
    }

    Ok(())
}
```

**Step 4: Wire into queue monitor**
```
Edit: wkmp-ap/src/playback/queue.rs (add refill trigger logic)
```

**Step 5: Test**
```bash
cargo test -p wkmp-ap
cargo run -p wkmp-ap
```

**Step 6: Verify integration**
- Start wkmp-pd
- Start wkmp-ap
- Observe logs for refill requests

---

## Success Criteria

A successful code implementation:
- ✅ Satisfies requirements from requirements.md
- ✅ Follows architecture from architecture.md
- ✅ Adheres to coding_conventions.md
- ✅ Includes requirement ID comments for traceability
- ✅ Passes all tests (`cargo test`)
- ✅ Compiles without warnings (`cargo clippy`)
- ✅ Is properly formatted (`cargo fmt`)
- ✅ Integrates correctly with other modules

Remember: **Code is the implementation of design, not the driver of it.** If the design is unclear, consult project-architect. If requirements are missing, flag for docs-specialist and formal change control.
