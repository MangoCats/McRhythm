# SPEC001: WKMP System Architecture

**Tier:** 2 (Design Specification)
**Status:** Active
**Version:** 1.0
**Last Updated:** 2025-11-02

**Purpose:** Defines the high-level system architecture, microservices design, communication patterns, and key architectural decisions for the WKMP (Auto DJ Music Player) project.

**Related Documents:**
- [PCH001: Project Charter](PCH001_project_charter.md) - Project goals and principles
- [REQ001: Requirements](REQ001-requirements.md) - Functional and non-functional requirements
- [REQ002: Entity Definitions](REQ002-entity_definitions.md) - Core domain concepts
- [ADR-003: Zero-Configuration Strategy](ADR-003-zero_configuration_strategy.md) - Config resolution pattern
- [SPEC007: API Design](SPEC007-api_design.md) - REST API and SSE specifications
- [IMPL001: Database Schema](IMPL001-database_schema.md) - Data model implementation

---

## Table of Contents

1. [System Overview](#1-system-overview)
2. [Microservices Architecture](#2-microservices-architecture)
3. [Communication Patterns](#3-communication-patterns)
4. [Data Architecture](#4-data-architecture)
5. [Audio Pipeline Architecture](#5-audio-pipeline-architecture)
6. [Zero-Configuration Startup](#6-zero-configuration-startup)
7. [Version Architecture](#7-version-architecture)
8. [Architectural Principles](#8-architectural-principles)

---

## 1. System Overview

### 1.1 Architecture Style

**WKMP uses a microservices architecture** with the following characteristics:

- **6 independent HTTP servers** (microservices)
- **1 shared library** (wkmp-common) for cross-module code
- **SQLite shared database** (single source of truth)
- **HTTP REST + Server-Sent Events** for inter-module communication
- **Zero-configuration startup** (works out-of-the-box for 95% of users)

### 1.2 Design Philosophy

**Per [PCH001: Project Charter](PCH001_project_charter.md):**

1. **Quality-Absolute Goals:**
   - Flawless audio playback (zero dropouts, clicks, pops)
   - Listener experience reminiscent of 1970s FM radio
   - Sample-accurate crossfading (~0.02ms precision)

2. **Risk-First Decision Framework:**
   - Risk reduction prioritized over implementation speed
   - Lowest residual risk approach always chosen
   - Quality takes precedence over development effort

3. **Architectural Separation:**
   - Clear component boundaries with single responsibilities
   - Audio processing separated from control logic
   - Fading separated from mixing (see [Section 5.2](#52-audio-pipeline-components))

---

## 2. Microservices Architecture

### 2.1 Module Overview

WKMP consists of 6 independent microservices + 1 shared library:

| Module | Port | Purpose | Versions | Auto-Start |
|--------|------|---------|----------|------------|
| **wkmp-ap** | 5721 | Core playback, crossfading, queue management | All | Yes |
| **wkmp-ui** | 5720 | Web UI, authentication, orchestration | All | Yes |
| **wkmp-pd** | 5722 | Automatic passage selection algorithm | Full, Lite | Yes |
| **wkmp-ai** | 5723 | Import wizard, file scanning, MusicBrainz | Full | On-demand |
| **wkmp-le** | 5724 | Lyric editor, split-window interface | Full | On-demand |
| **wkmp-dr** | 5725 | Read-only database inspection tool | Full | On-demand |
| **wkmp-common** | N/A | Shared library (models, events, utilities) | All | N/A |

**Auto-Start vs On-Demand:**
- **Auto-Start:** Always running when WKMP starts
- **On-Demand:** User launches via browser when needed (opens in new tab)

### 2.2 Module Responsibilities

#### wkmp-ap: Audio Player (Core Playback Engine)

**Responsibilities:**
- Audio decoding (symphonia)
- Sample rate conversion (rubato)
- Sample-accurate crossfading (custom implementation)
- Playback queue management
- Buffer management (pre-decoded PCM)
- Audio output (cpal)
- Pause/resume with exponential decay
- Master volume control

**Key Capabilities:**
- Sample-accurate crossfade timing (~0.02ms precision - [REQ-CF-070])
- 5 fade curve types (Linear, EqualPower, Logarithmic, Exponential, SCurve - [REQ-CF-080])
- Marker-based event system for precise triggering
- Ring buffer for lock-free audio thread communication

**Port:** 5721
**Technology:** Rust, Tokio, Axum, symphonia, rubato, cpal

#### wkmp-ui: User Interface (Web Frontend & Orchestration)

**Responsibilities:**
- Serve web UI (HTML/CSS/JavaScript)
- User authentication and session management
- API request proxying to other modules

**Key Capabilities:**
- SSE (Server-Sent Events) for real-time updates
- Multi-user coordination
- Developer UI with API testing tools

**Port:** 5720
**Technology:** Rust, Tokio, Axum, HTML/CSS/JS

**See:**
- [SPEC009: UI Specification](SPEC009-ui_specification.md) - User interface design
- [SPEC012: Multi-User Coordination](SPEC012-multi_user_coordination.md) - Multi-user patterns

#### wkmp-pd: Program Director (Automatic Selection)

**Responsibilities:**
- Automatic passage selection based on musical flavor distance, cooldowns, and probabilities
- Timeslot management (time-of-day scheduling)

**Key Capabilities:**
- Euclidean distance calculation in musical flavor space
- Multi-level cooldowns (song: 14d, artist: 30min, work: 2d)
- Weighted random selection with flavor distance scoring

**Port:** 5722
**Technology:** Rust, Tokio, Axum

**See:** [SPEC005: Program Director](SPEC005-program_director.md) for selection algorithm details

#### wkmp-ai: Audio Ingest (File Import & Identification)

**Responsibilities:**
- Import wizard UI (browser-based)
- File system scanning and passage boundary detection
- MusicBrainz lookup and metadata matching

**Key Capabilities:**
- Multi-phase import workflow with progress tracking
- Cancellable long-running operations
- MusicBrainz API integration with rate limiting

**Port:** 5723 (on-demand)
**Technology:** Rust, Tokio, Axum, symphonia

**See:** [SPEC032: Audio Ingest Architecture](SPEC032-audio_ingest_architecture.md) for import workflow details

#### wkmp-le: Lyric Editor (Lyric Timing Editor)

**Responsibilities:**
- Split-window lyric editing interface
- Lyric-to-audio synchronization
- Timing adjustment tools
- Visual feedback for lyric boundaries

**Port:** 5724 (on-demand)
**Technology:** Rust, Tokio, Axum

#### wkmp-dr: Database Review (Inspection Tool)

**Responsibilities:**
- Read-only database browsing
- Table inspection with pagination
- Query execution with results display
- Data export

**Port:** 5725
**Technology:** Rust, Tokio, Axum, SQLite

#### wkmp-common: Shared Library

**Responsibilities:**
- Database models (File, Passage, Song, Artist, Work, Album, etc.)
- Event system (EventBus, WkmpEvent enum)
- Configuration utilities (RootFolderResolver, RootFolderInitializer)
- Timing utilities (ms_to_ticks, ticks_to_ms)
- Database initialization (migrations, table creation)

**Technology:** Rust, SQLx, chrono, serde

---

## 3. Communication Patterns

### 3.1 Inter-Module Communication

**HTTP REST APIs:**
- All modules expose HTTP REST endpoints ([SPEC007](SPEC007-api_design.md))
- Request/response pattern for commands
- JSON payloads for structured data
- Standard HTTP status codes (200, 400, 404, 500)

**Server-Sent Events (SSE):**
- Real-time updates pushed from modules to UI
- One-way communication (server → client)
- Event types: PassageStarted, CrossfadeStarted, CurrentSongChanged, QueueUpdated, etc.
- See [SPEC011: Event System](SPEC011-event_system.md)

**Example Flow:**
```
User (Browser)
    ↓ HTTP GET /queue
wkmp-ui (5720)
    ↓ HTTP GET http://localhost:5721/queue
wkmp-ap (5721)
    ↓ SQL SELECT * FROM queue_entries
SQLite Database
    ↑ Results
wkmp-ap
    ↑ JSON response
wkmp-ui
    ↑ JSON response
User (Browser)
```

### 3.2 Event Broadcasting

**EventBus Architecture:**
- `tokio::broadcast` channel for one-to-many distribution
- Each module creates EventBus instance
- SSE handlers subscribe to event stream
- Automatic reconnection on connection loss

**Event Flow:**
```
wkmp-ap (audio thread)
    ↓ Marker reached (e.g., PassageComplete)
EventBus (broadcast channel)
    ↓ Clone to all subscribers
SSE Handler 1 (User A)
SSE Handler 2 (User B)
SSE Handler 3 (Developer UI)
```

---

## 4. Data Architecture

### 4.1 Shared Database

**Technology:** SQLite 3 with JSON1 extension

**Location:** `<root_folder>/wkmp.db` (see [Section 6](#6-zero-configuration-startup))

**Characteristics:**
- **Single source of truth** for all modules
- **UUID primary keys** for all entities (enables database merging)
- **Foreign key cascades** for automatic cleanup
- **Automatic triggers** for last_played_at timestamps
- **JSON storage** for musical flavor vectors

**Schema:** See [IMPL001: Database Schema](IMPL001-database_schema.md)

### 4.2 Key Entities

| Entity | Purpose | Key Relationships |
|--------|---------|-------------------|
| **File** | Audio file on disk | → Passages (1:N) |
| **Passage** | Playable region within file | → File (N:1), → Songs (N:M) |
| **Song** | MusicBrainz Recording | → Passages (M:N), → Artists (M:N) |
| **Artist** | MusicBrainz Artist | → Songs (M:N) |
| **Work** | Musical composition | → Songs (M:N) |
| **Album** | MusicBrainz Release | → Songs (M:N) |
| **QueueEntry** | Playback queue item | → Passage (N:1) |
| **Timeslot** | Time-of-day schedule | N/A (stores target flavor) |

See [REQ002: Entity Definitions](REQ002-entity_definitions.md) for detailed entity semantics.

### 4.3 Data Access Patterns

**In-Memory Operations (High Frequency):**
- Queue state checks (every 100ms in playback loop - in-memory only, no DB queries)
- Buffer status checks (continuous in mixer thread)
- Playback position tracking (atomic updates, ~100Hz)

**Database Reads (Event-Driven):**
- Passage metadata (triggered on queue entry load)
- Song/artist cooldown checks (triggered by Program Director selection)
- Settings retrieval (on module startup or setting change)
- Album UUID lookups (triggered on passage events)

**Database Writes (Infrequent):**
- Queue persistence (on queue modifications - enqueue/dequeue)
- Last played timestamps (on passage completion)
- Import operations (batch inserts during audio ingest)
- Settings updates (on user configuration changes)

**Concurrency:**
- SQLite WAL mode for concurrent reads
- Write serialization handled by SQLite
- No application-level locking required
- Most playback coordination uses in-memory structures (QueueManager, BufferManager)

**See:** [SPEC028: Playback Orchestration](SPEC028-playback_orchestration.md) for detailed explanation of the 100ms playback loop and event-driven architecture

---

## 5. Audio Pipeline Architecture

### 5.1 Single-Stream Design

**WKMP uses a single-stream audio architecture** with sample-accurate crossfading.

**Key Principle:** "One audio stream to the hardware at all times" ([SPEC013](SPEC013-single_stream_playback.md))

**Benefits:**
- Eliminates hardware context switching
- Guarantees glitch-free crossfades
- Sample-accurate fade timing control
- Predictable latency characteristics

### 5.2 Audio Pipeline Components

**Pipeline:** Decoder → Resampler → Fader → Buffer → Mixer → Output

```
┌──────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐
│ Decoder  │ --> │Resampler │ --> │  Fader   │ --> │  Buffer  │ --> │  Mixer   │ --> │  Output  │
│ (symphon)│     │ (rubato) │     │ (Worker) │     │  (Mgr)   │     │  (Main)  │     │  (cpal)  │
└──────────┘     └──────────┘     └──────────┘     └──────────┘     └──────────┘     └──────────┘
```

**High-Level Summary:**
- **Decoder:** symphonia decodes compressed audio (MP3, FLAC, Opus) to PCM at native sample rate
- **Resampler:** rubato converts to standard 44.1kHz (stateful, preserves phase across chunks)
- **Fader:** Applies fade curves (Linear, EqualPower, Logarithmic, Exponential, SCurve) BEFORE buffering
- **Buffer:** Stores pre-decoded, pre-faded PCM (10s capacity, lock-free access)
- **Mixer:** Simple addition of pre-faded samples (no runtime fade calculations)
- **Output:** cpal writes to hardware (platform-dependent latency)

**See detailed specifications:**
- [SPEC013: Single-Stream Playback](SPEC013-single_stream_playback.md) - Overall audio pipeline architecture
- [SPEC016: Decoder & Buffer Design](SPEC016-decoder_buffer_design.md) - Decoder and buffer implementation
- [SPEC017: Sample Rate Conversion](SPEC017-sample_rate_conversion.md) - Resampling strategy and rubato integration
- [SPEC002: Crossfade](SPEC002-crossfade.md) - Fade curve algorithms and timing
- [SPEC028: Playback Orchestration](SPEC028-playback_orchestration.md) - Pipeline coordination and threading

### 5.3 Marker-Based Event System

**Problem:** How to trigger crossfades at precise sample positions?

**Solution:** Marker system with tick-based positioning

**Marker Types:**
- `StartCrossfade { next_passage_id }` - Trigger crossfade (at fade-out point - 5 sec)
- `PassageComplete` - Passage finished playing
- `SongBoundary { song_id, position_ms }` - Song changed within passage

**Marker Registration:**
```rust
mixer.add_marker(PositionMarker {
    tick: fade_out_start_tick,           // Calculated from fade_out_point
    passage_id: current_passage_id,
    event_type: MarkerEvent::StartCrossfade { next_passage_id },
});
```

**Marker Triggering:**
- Mixer checks `current_tick >= marker.tick` during playback
- Emits SSE event when marker reached
- State transitions coordinated in main thread (not audio thread)

**See:** [SPEC028: Playback Orchestration](SPEC028-playback_orchestration.md) for detailed explanation of marker processing, event flow, and threading model

---

## 6. Zero-Configuration Startup

**Requirement:** [REQ-NF-030] through [REQ-NF-037] - All modules must start without configuration files

**Implementation:** 4-tier priority system for root folder resolution

**See:** [ADR-003: Zero-Configuration Strategy](ADR-003-zero_configuration_strategy.md) for detailed decision rationale

### 6.1 Root Folder Resolution

**Priority Order:**
1. **CLI argument:** `--root-folder /custom/path` or `--root /custom/path`
2. **Environment variable:** `WKMP_ROOT_FOLDER=/custom/path` or `WKMP_ROOT=/custom/path`
3. **TOML config:** `~/.config/wkmp/<module-name>.toml`
4. **Compiled default:** `~/Music` (Linux/macOS), `%USERPROFILE%\Music` (Windows)

**Result:** 95% of users get zero-config startup, power users can override

### 6.2 Startup Sequence (Per Module)

**High-Level Steps:**
1. Initialize tracing subscriber [ARCH-INIT-003]
2. Log build identification [ARCH-INIT-004]
3. Resolve root folder (4-tier priority via `RootFolderResolver`)
4. Create directory if missing (via `RootFolderInitializer`)
5. Connect to database (`<root_folder>/wkmp.db`)
6. Initialize database schema (migrations via `wkmp_common::db::init`)
7. Start HTTP server (Axum on module-specific port)

**Implementation Utilities:**
- `wkmp_common::config::RootFolderResolver` - 4-tier priority resolution
- `wkmp_common::config::RootFolderInitializer` - Directory creation and database path
- `wkmp_common::db::init::init_database()` - Schema initialization and migrations

**See:** [ADR-003: Zero-Configuration Strategy](ADR-003-zero_configuration_strategy.md) for complete implementation example and rationale

### 6.3 Database Initialization

**Automatic on first start:**
- SQLite database created at `<root_folder>/wkmp.db`
- Migrations applied via `wkmp_common::db::init::init_database()`
- Empty tables initialized with schema from [IMPL001](IMPL001-database_schema.md)

**Subsequent starts:**
- Existing database opened
- Migrations applied if schema version mismatch
- No data loss

---

## 7. Version Architecture

**WKMP ships in 3 versions:** Full, Lite, Minimal

**Packaging Strategy:** No conditional compilation - versions differ only by which binaries are packaged.

| Version | Binaries Included | Use Case |
|---------|-------------------|----------|
| **Full** | All 6 modules | Complete experience (import, edit, inspect) |
| **Lite** | wkmp-ap, wkmp-ui, wkmp-pd | Playback + auto-selection (no import) |
| **Minimal** | wkmp-ap, wkmp-ui | Manual queue management only |

**UI Behavior:**
- **Full:** All features enabled
- **Lite:** "Import Music" button disabled with "Full version required" tooltip
- **Minimal:** "Import Music" and "Auto-Select" disabled

**Benefit:** Single codebase, single build, simple packaging scripts

---

## 8. Architectural Principles

### 8.1 Design Principles (from [PCH001](PCH001_project_charter.md))

**Risk-First Framework:**
- Evaluate failure risk FIRST for every design decision
- Choose approach with lowest residual risk
- Quality over implementation speed

**Separation of Concerns:**
- Fading separated from mixing ([DBD-MIX-042])
- Control logic separated from audio processing
- State management separated from rendering

**Sample-Accurate Timing:**
- Tick-based position tracking (44.1 kHz = 1 tick = ~0.023ms)
- Marker system for precise event triggering
- Pre-calculated fade-in/fade-out points

### 8.2 Technology Choices

**Rust:**
- Memory safety without garbage collection
- Zero-cost abstractions
- Fearless concurrency
- Strong type system

**Tokio:**
- Async runtime for concurrent I/O
- Task spawning for background workers
- Channel-based communication

**Axum:**
- Type-safe HTTP framework
- Tower middleware ecosystem
- SSE (Server-Sent Events) support

**SQLite:**
- Zero-configuration embedded database
- ACID transactions
- JSON1 extension for flexible schemas
- Cross-platform compatibility

**symphonia:**
- Pure Rust audio decoding
- Format detection
- Metadata extraction

**rubato:**
- High-quality sample rate conversion
- Configurable resampling algorithms

**cpal:**
- Cross-platform audio output
- Low-latency support
- Platform-native backends

### 8.3 Constraints

**Single Database:**
- All modules share one SQLite database
- No database replication or sharding
- Suitable for single-user installations

**Localhost Only:**
- All modules bind to 127.0.0.1
- No remote access by default
- Security through local-only binding

**Sequential Playback:**
- One passage playing at a time (during crossfade: 2)
- No parallel audio streams
- No mixing arbitrary tracks

---

## Appendix A: Port Assignments

| Port | Module | Auto-Start | Access Pattern |
|------|--------|------------|----------------|
| 5720 | wkmp-ui | Yes | User opens http://localhost:5720 |
| 5721 | wkmp-ap | Yes | wkmp-ui proxies requests |
| 5722 | wkmp-pd | Yes | wkmp-ui proxies requests |
| 5723 | wkmp-ai | On-demand | User clicks "Import" → opens in new tab |
| 5724 | wkmp-le | On-demand | User clicks "Edit Lyrics" → opens in new tab |
| 5725 | wkmp-dr | On-demand | User clicks "Database" → opens in new tab |

---

## Appendix B: Key Architectural Decisions

| Decision | Document | Rationale |
|----------|----------|-----------|
| Microservices over monolith | This document | Modularity, independent deployment |
| Single-stream audio | [SPEC013](SPEC013-single_stream_playback.md) | Glitch-free crossfades |
| Zero-configuration startup | [ADR-003](ADR-003-zero_configuration_strategy.md) | User experience, 95% out-of-box |
| Marker-based events | [wkmp-ap/core.rs:1245](../wkmp-ap/src/playback/engine/core.rs#L1245) | Sample-accurate timing |
| Fading before buffering | [Mixer review](../wip/mixer_architecture_review.md) | Architectural separation |
| SQLite shared database | [IMPL001](IMPL001-database_schema.md) | Simplicity, zero-config |
| HTTP REST + SSE | [SPEC007](SPEC007-api_design.md) | Platform independence, real-time updates |

---

**Document Status:** Active
**Next Review:** When adding new modules or major architectural changes
**Maintained By:** WKMP Development Team
