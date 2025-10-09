# McRhythm Architecture

**🏗️ TIER 2 - DESIGN SPECIFICATION**

Defines HOW the system is structured. Derived from [requirements.md](requirements.md). See [Document Hierarchy](document_hierarchy.md).

> **Related Documentation:** [Database Schema](database_schema.md) | [Crossfade Design](crossfade.md) | [Musical Flavor](musical_flavor.md)| [Event System](event_system.md)

---

## Overview

McRhythm is a music player built on Rust, GStreamer, and SQLite that automatically selects music passages based on user-configured musical flavor preferences by time of day, using cooldown-based probability calculations and AcousticBrainz musical characterization data.

McRhythm implements a **microservices architecture** with multiple independent processes communicating via HTTP APIs and Server-Sent Events (SSE). This enables simplified maintenance, version flexibility, and independent module updates.

## Process Architecture

McRhythm consists of 4 independent processes, each with defined HTTP/SSE interfaces:

- **Module 1: Audio Player** - Core playback engine with queue management
- **Module 2: User Interface** - Polished web UI for end users
- **Module 3: Program Director** - Automatic passage selection
- **Module 4: File Ingest Interface** - New file import workflow (Full version only)

**Design Benefits:**
- **Simplifies maintenance**: Each module focuses on a single concern
- **Enables version flexibility**: Run more/fewer processes for Full/Lite/Minimal versions
- **Provides modularity**: Update one module without affecting others
- **Supports independent operation**: Audio Player and Program Director work without UI

### Process Communication Model

```
┌─────────────────────────────────────────────────────────────┐
│  Module 2: User Interface (HTTP + SSE Server)               │
│  Port: 8080 (configurable)                                  │
│  - Polished web UI for end users                            │
│  - Authentication, playback control, queue management       │
│  - Album art, lyrics, likes/dislikes, configuration         │
└───────────┬─────────────────────────────────────────────────┘
            │ HTTP API calls
            │ SSE subscriptions
    ┌───────┼────────┬────────────────────────┐
    │       │        │                        │
    ▼       ▼        ▼                        ▼
┌───────┐ ┌────────────────┐  ┌──────────────────────────────┐
│Module │ │  Module 1:     │  │  Module 3:                   │
│  4:   │ │  Audio Player  │  │  Program Director            │
│  New  │ │  Port: 8081    │  │  Port: 8082                  │
│ File  │ │                │◄─┤                              │
│Ingest │ │  - Minimal     │  │  - Minimal dev UI            │
│  UI   │ │    dev UI      │  │  - Selection API (for UI)    │
│       │ │  - Control API │  │  - Reads Audio Player status │
│(Full  │ │  - Status API  │  │  - Enqueues via Audio Player │
│ only) │ │  - SSE events  │  │                              │
│       │ │                │  │  SQLite Database (Shared)    │
│Port:  │ │                │  │  - Files, Passages, Songs    │
│ 8083  │ │                │  │  - Play History, Queue       │
└───────┘ └────────────────┘  └──────────────────────────────┘
            │                   │
            │ Direct HTTP API   │
            └───────────────────┘
               (No UI required)
```

### Version-Specific Process Configuration

| Version  | Module 1<br/>Audio Player | Module 2<br/>User Interface | Module 3<br/>Program Director | Module 4<br/>File Ingest |
|----------|---------------------------|----------------------------|------------------------------|--------------------------|
| **Full**     | ✅ Running | ✅ Running (Full-featured) | ✅ Running | ✅ Running |
| **Lite**     | ✅ Running | ✅ Running (De-featured)   | ✅ Running | ❌ Not included |
| **Minimal**  | ✅ Running | ✅ Running (De-featured)   | ❌ Not included | ❌ Not included |

## Module Specifications

### Module 1: Audio Player

**Process Type**: Independent HTTP server with minimal developer UI
**Port**: 8081 (configurable)
**Versions**: Full, Lite, Minimal

**Responsibilities:**
- Manages dual GStreamer pipelines for seamless crossfading
- Coordinates passage transitions based on lead-in/lead-out timing
- Implements three fade profiles (exponential, cosine, linear)
- Handles pause/resume with 0.5s exponential fade-in
- Manages volume control (user level + fade automation)
- Maintains playback queue with persistence

**HTTP Control API:**
- `POST /audio/device` - Set audio output device
- `POST /audio/volume` - Set volume level (0-100)
- `POST /playback/enqueue` - Enqueue a passage
- `DELETE /playback/queue/{passage_id}` - Remove passage from queue
- `POST /playback/play` - Resume playback
- `POST /playback/pause` - Pause playback

**HTTP Status API:**
- `GET /audio/device` - Current audio output device
- `GET /audio/volume` - Current volume level
- `GET /playback/queue` - Queue contents
- `GET /playback/state` - Playing/Paused state
- `GET /playback/position` - Current playback position in passage

**SSE Events** (Endpoint: `GET /events`):
- `VolumeChanged` - Volume level updated
- `QueueChanged` - Queue modified (add/remove/reorder)
- `PlaybackStateChanged` - Playing/Paused state changed
- `PlaybackProgress` - Position updates (every 500ms)
- `PassageStarted` - New passage began playing
- `PassageCompleted` - Passage finished
- `CurrentSongChanged` - Within-passage song boundary crossed

**Developer UI** (Minimal):
- Module status display
- Direct API testing interface
- Event stream monitor

**State:**
- Currently playing passage (position, duration, state)
- Next passage (pre-loaded, ready for crossfade)
- Queue contents (persisted to SQLite)
- User volume level (0-100)
- Playback state (Playing/Paused only - no "stopped" state)
- Initial state on app launch: Playing

**Key Design Notes:**
- **Operates independently**: Does not require Module 2 (User Interface) to be running
- **Receives commands from**: Module 2 (User Interface), Module 3 (Program Director)
- **Database access**: Direct SQLite access for queue persistence, passage metadata

### Module 2: User Interface

**Process Type**: Polished HTTP server with full web UI
**Port**: 8080 (configurable)
**Versions**: Full, Lite (de-featured), Minimal (de-featured)

**Responsibilities:**
- Present polished web interface for end users
- Proxy/orchestrate requests to Module 1 (Audio Player) and Module 3 (Program Director)
- Handle user authentication and session management
- Display album art, lyrics, and playback information
- Provide configuration interface for Program Director parameters
- Aggregate SSE events from Audio Player for UI updates

**HTTP API** (User-facing):
- Authentication endpoints: `/api/login`, `/api/create-account`, `/api/current-user`
- Playback control: `/api/playback/*` (proxied to Module 1)
- Queue management: `/api/queue/*` (proxied to Module 1)
- Like/Dislike: `/api/passages/{id}/like`, `/api/passages/{id}/dislike`
- Program Director config: Proxied to Module 3
- Manual passage selection: Browse library, enqueue to Module 1
- Volume control: Proxied to Module 1
- Audio device selection: Proxied to Module 1

**SSE Events** (Endpoint: `GET /api/events`):
- Aggregates and forwards events from Module 1 (Audio Player)
- Adds user-specific events (session, likes/dislikes)

**Web UI Features:**
- Authentication flow (Anonymous/Create Account/Login)
- Now Playing: Album art, song/artist/album, passage title, lyrics
- Playback controls: Play/Pause, Skip, volume slider
- Queue display and manual queue management
- Like/Dislike buttons (Full/Lite versions)
- Program Director configuration (timeslots, base probabilities, cooldowns)
- Network status indicators (internet and local network)
- Responsive design for desktop and mobile

**Version Differences:**
- **Full**: All features enabled
- **Lite**: No file ingest, limited configuration options
- **Minimal**: No file ingest, no likes/dislikes, no advanced configuration

**Key Design Notes:**
- **Most users interact here**: Primary interface for controlling McRhythm
- **Orchestration layer**: Coordinates between Audio Player and Program Director
- **Database access**: Direct SQLite access for user data, likes/dislikes, library browsing

---

### Module 3: Program Director

**Process Type**: Independent HTTP server with minimal developer UI
**Port**: 8082 (configurable)
**Versions**: Full, Lite (Minimal does not include automatic selection)

**Responsibilities:**
- Calculate passage selection probabilities based on multiple factors
- Implement weighted random selection algorithm
- Maintain time-of-day flavor targets
- Handle timeslot transitions
- Respond to temporary flavor overrides
- Monitor Audio Player queue and automatically enqueue passages

**HTTP API for User Interface** (Module 2):
- `GET /config/timeslots` - Retrieve timeslot configuration
- `POST /config/timeslots` - Update timeslot configuration
- `GET /config/probabilities` - Get base probabilities for songs/artists/works
- `PUT /config/probabilities/{entity_type}/{id}` - Set base probability
- `GET /config/cooldowns` - Get cooldown settings
- `PUT /config/cooldowns` - Update cooldown settings
- `POST /selection/override` - Temporary flavor override
- `DELETE /selection/override` - Clear temporary override

**HTTP Status API:**
- `GET /status` - Module status, current timeslot, target flavor
- `GET /selection/candidates` - Last selection candidates (debugging)

**SSE Events** (Endpoint: `GET /events`):
- `TimeslotChanged` - New timeslot became active
- `TemporaryFlavorOverride` - Temporary override activated
- `OverrideExpired` - Temporary override ended
- `SelectionFailed` - No candidates available

**Developer UI** (Minimal):
- Module status display
- Current timeslot and target flavor
- Last selection results

**Automatic Queue Management:**
- Polls Module 1 (Audio Player) queue status periodically
- When queue drops below threshold (< 2 passages or < 15 minutes):
  1. Determine target time (end of last queued passage)
  2. Calculate selection probabilities
  3. Select passage via weighted random algorithm
  4. Enqueue to Module 1 via HTTP API

**Key Operations:**
- Determine target time for selection (end time of last queued passage)
- Filter to non-zero probability passages (passages with one or more songs only)
- Calculate squared Euclidean distance from target flavor
- Sort by distance, take top 100 candidates
- Weighted random selection from candidates
- Handle edge cases (no candidates → stop automatic enqueueing)

**Key Design Notes:**
- **Operates independently**: Does not require Module 2 (User Interface) to be running
- **Communicates with Module 1 only**: Reads queue status, enqueues passages
- **Database access**: Direct SQLite access for passage metadata, timeslots, probabilities, play history

> **See [Program Director](program_director.md) for complete specification of selection algorithm, cooldown system, probability calculations, and timeslot handling.**

---

### Module 4: File Ingest Interface

**Process Type**: Polished HTTP server with guided workflow UI
**Port**: 8083 (configurable)
**Versions**: Full only

**Responsibilities:**
- Present user-friendly interface for adding new audio files
- Guide user through ingest and characterization workflow
- Coordinate MusicBrainz/AcousticBrainz lookups
- Manage Essentia local flavor analysis (Full version)
- Support passage segmentation and metadata editing

**HTTP API:**
- `POST /ingest/scan` - Scan directory for new files
- `GET /ingest/pending` - List files pending ingest
- `POST /ingest/identify/{file_id}` - Trigger MusicBrainz lookup
- `POST /ingest/characterize/{file_id}` - Trigger flavor analysis
- `POST /ingest/segment/{file_id}` - Define passages within file
- `PUT /ingest/metadata/{passage_id}` - Edit passage metadata
- `POST /ingest/finalize/{file_id}` - Complete ingest workflow

**Web UI Workflow:**
1. **File Discovery**: Select directories to scan for new audio files
2. **File Review**: Preview detected files, confirm additions
3. **Identification**: Match files to MusicBrainz recordings (fingerprinting)
4. **Characterization**: Retrieve AcousticBrainz data or run local Essentia analysis
5. **Passage Definition**: Define passage boundaries, timing points, metadata
6. **Finalization**: Review and commit to library

**Key Design Notes:**
- **Full version only**: Not included in Lite or Minimal
- **Database access**: Direct SQLite access for file/passage/song insertion
- **External API integration**: MusicBrainz, AcousticBrainz, Chromaprint
- **Local analysis**: Essentia integration for offline flavor characterization

> **See [Library Management](library_management.md) for complete file scanning and metadata workflows.**

---

### Internal Components

The modules listed above are separate processes. Within each module, there are internal components that handle specific responsibilities. These are implementation details within each module:

**Module 1 (Audio Player) Internal Components:**
- **Queue Manager**: Maintains playback queue (minimum 2 passages, 15+ minutes), handles manual additions/removals, triggers automatic queue replenishment
- **Playback Controller**: Manages dual GStreamer pipelines for crossfading, coordinates passage transitions
- **Audio Engine**: GStreamer pipeline manager with dual pipelines, audio mixer, volume control
- **Historian**: Records passage plays with timestamps, updates last-play times for cooldown calculations

**Module 2 (User Interface) Internal Components:**
- **Authentication Handler**: User session management, Anonymous/Create/Login flows
- **API Proxy**: Forwards requests to Module 1 and Module 3
- **Event Aggregator**: Subscribes to Module 1 SSE events, forwards to web UI clients
- **Library Browser**: Database queries for passage/song/artist/album browsing

**Module 3 (Program Director) Internal Components:**
- **Flavor Manager**: Manages 24-hour timeslot schedule, calculates flavor targets, handles temporary overrides
- **Selection Engine**: Implements weighted random selection algorithm with flavor distance calculations
- **Queue Monitor**: Polls Module 1 for queue status, triggers selection when needed

**Module 4 (File Ingest) Internal Components:**
- **File Scanner**: Recursive directory scan with change detection (SHA-256 hashes)
- **Metadata Extractor**: Parse ID3v2, Vorbis Comments, MP4 tags
- **Fingerprint Generator**: Chromaprint for MusicBrainz identification
- **External Integration Clients**: MusicBrainz, AcousticBrainz, Essentia

### Shared Infrastructure Components

These components are used across multiple modules:

**SQLite Database:**
- Embedded in each module process (same database file)
- Files, Passages, Songs, Artists, Works, Albums
- Play History, Queue State, Settings, Timeslots
- Musical Flavor Vectors, Album Art File Paths

**External API Clients:**
- **MusicBrainz Client**: Recording/Release/Artist/Work identification, all responses cached locally
- **AcousticBrainz Client**: High-level musical characterization vectors, fallback to Essentia (Full version)
- **ListenBrainz Client** (Phase 2): Play history submission, recommendations (TBD)

### Implementation Details Removed From This Section

The following subsections previously described monolithic components. They have been replaced by the module-based architecture above:
- ~~3. Queue Manager~~ - Now part of Module 1 (Audio Player)
- ~~4. Historian~~ - Now part of Module 1 (Audio Player)
- ~~5. Flavor Manager~~ - Now part of Module 3 (Program Director)
- ~~6. Audio Engine~~ - Now part of Module 1 (Audio Player)
- ~~7. Library Manager~~ - Now part of Module 4 (File Ingest)
- ~~8. External Integration Clients~~ - Shared across modules

---

## Inter-Process Communication

### HTTP/REST APIs

**Primary communication method** between modules.

**Benefits:**
- **Platform-independent**: Language-agnostic interfaces
- **Well-defined contracts**: Clear API boundaries between modules
- **Easy debugging**: Standard HTTP tools (curl, Postman) for testing
- **Independent deployment**: Modules can be updated separately
- **Network transparency**: Modules can run on same machine or distributed

**Request/Response Patterns:**
- User Interface → Audio Player: Playback commands, queue management
- User Interface → Program Director: Configuration updates
- Program Director → Audio Player: Automatic enqueueing
- File Ingest → Database: New file/passage insertion

**Error Handling:**
- HTTP status codes for success/failure
- JSON error responses with details
- Retry logic for transient failures
- Graceful degradation when modules unavailable

### Server-Sent Events (SSE)

**Real-time notification method** from modules to clients.

**Event Flows:**
- Audio Player → User Interface: Playback state, queue changes, position updates
- Program Director → User Interface: Timeslot changes, selection events
- Each module provides `/events` endpoint for SSE subscriptions

**Benefits:**
- **One-directional push**: Server-to-client notifications
- **Lightweight**: Built on HTTP, auto-reconnect
- **Multi-subscriber**: Multiple UIs can subscribe to same events
- **Loose coupling**: Event producers don't need to know consumers

### Database as Shared State

**SQLite database** serves as persistent shared state across all modules.

**Access Patterns:**
- Each module has direct SQLite access (embedded database, same file)
- Coordinated writes via HTTP API boundaries
- Read-heavy access for passage metadata, library browsing
- Triggers maintain consistency (e.g., last_played_at updates)

**Consistency Considerations:**
- UUID primary keys enable database merging (Full → Lite → Minimal)
- Foreign key constraints maintain referential integrity
- Application-level coordination via HTTP APIs prevents conflicts
- Write serialization through module ownership (e.g., only Audio Player writes queue state)

### Module Dependencies

```
Module 2: User Interface
    ├── Depends on: Module 1 (Audio Player) - optional, degrades gracefully
    ├── Depends on: Module 3 (Program Director) - optional (Minimal version)
    └── Depends on: SQLite database - required

Module 3: Program Director
    ├── Depends on: Module 1 (Audio Player) - required for enqueueing
    └── Depends on: SQLite database - required

Module 1: Audio Player
    └── Depends on: SQLite database - required

Module 4: File Ingest (Full only)
    └── Depends on: SQLite database - required
```

**Startup Requirements:**
- Module 1 (Audio Player) can start standalone
- Module 3 (Program Director) requires Module 1 to be running
- Module 2 (User Interface) can start without other modules (degrades features)
- Module 4 (File Ingest) can start standalone

---

## Component Implementation Details

This architecture implements the requirements specified in [requirements.md](requirements.md).

Detailed design specifications for each subsystem:
- **Crossfade System**: See [Crossfade Design](crossfade.md)
- **Musical Flavor System**: See [Musical Flavor](musical_flavor.md)
- **Event-Driven Communication**: See [Event System](event_system.md)
- **Data Model**: See [Database Schema](database_schema.md)
- **Code Organization**: See [Coding Conventions](coding_conventions.md)

## Concurrency Model

### Per-Module Threading

Each module is an independent process with its own threading model:

**Module 1 (Audio Player):**
```
HTTP Server Thread Pool (tokio async):
  - API request handling
  - SSE broadcasting to clients

Audio Thread (GStreamer):
  - Pipeline execution
  - Crossfade timing
  - Volume automation
  - Isolated from blocking I/O

Queue Manager Thread (tokio async):
  - Queue persistence
  - Passage loading
  - Database queries

GStreamer Bus Handler:
  - Pipeline events (EOS, error, state change)
  - Position queries (every 500ms)
  - Crossfade triggers
```

**Module 2 (User Interface):**
```
HTTP Server Thread Pool (tokio async):
  - Web UI serving
  - API request handling
  - SSE aggregation and forwarding
  - Proxy requests to Modules 1 and 3

Database Query Pool (tokio async):
  - Library browsing queries
  - User data (likes/dislikes)
  - Session management
```

**Module 3 (Program Director):**
```
HTTP Server Thread Pool (tokio async):
  - API request handling
  - SSE broadcasting

Selection Thread (tokio async):
  - Passage selection algorithm
  - Distance calculations
  - Probability computations

Queue Monitor Thread (tokio async):
  - Polls Module 1 queue status
  - Triggers selection when needed
```

**Module 4 (File Ingest):**
```
HTTP Server Thread Pool (tokio async):
  - API request handling
  - Web UI serving

Scanner Thread (tokio async):
  - File system scanning
  - Metadata extraction
  - Fingerprint generation

External API Pool (tokio async):
  - MusicBrainz queries
  - AcousticBrainz queries
  - Essentia local analysis
```

### Internal Communication Patterns

Within each module, components use standard Rust async patterns:

**Event Broadcasting (tokio::broadcast):**
- One-to-many notification pattern within a module
- Playback events, queue events, system events
- Enables loose coupling between internal components

**Command Channels (tokio::mpsc):**
- Request-response pattern with single handler
- Clear ownership and error propagation

**Shared State (Arc<RwLock<T>>):**
- Read-heavy access to current state
- Current playback state, queue contents, configuration

**Watch Channels (tokio::sync::watch):**
- Latest-value semantics for single-value updates
- Volume level changes, position updates

> **See [Event System](event_system.md) for complete event-driven architecture specification within modules.**

## Data Model

McRhythm uses SQLite with UUID-based primary keys for all entities. The complete schema includes:

**Core Entities:** files, passages, songs, artists, works, albums
**Relationships:** passage_songs, passage_albums, song_works
**Playback:** play_history, likes_dislikes, queue
**Configuration:** timeslots, timeslot_passages, settings
**Caching:** acoustid_cache, musicbrainz_cache, acousticbrainz_cache

See [Database Schema](database_schema.md) for complete table definitions, constraints, indexes, and triggers.

### Key Design Decisions

- **UUID primary keys**: Enable database merging across Full/Lite/Minimal versions
- **Musical flavor vectors**: Stored as JSON in `passages.musical_flavor_vector` for flexibility and SQLite JSON1 integration
- **Automatic triggers**: Update `last_played_at` timestamps on playback for cooldown calculations
- **Foreign key cascades**: Simplify cleanup when files/passages deleted
- **No binary blobs**: Album art stored as files, database stores paths only
- **Event-driven architecture**: Uses `tokio::broadcast` for one-to-many event distribution, avoiding tight coupling between components while staying idiomatic to Rust async ecosystem. See [Event System](event_system.md) for details.
- **Hybrid communication**: Events for notifications, channels for commands, shared state for reads—each pattern chosen for specific use cases

## Version Differentiation

McRhythm is built in three versions (Full, Lite, Minimal) by running different combinations of modules. See [Requirements - Three Versions](requirements.md#three-versions) for detailed feature comparison and resource profiles.

**Implementation approach:**
- **Process-based differentiation**: Different modules run in each version
- **Per-module feature flags**: Each module binary may have conditional features
- **Configuration files**: Specify which modules to start for each version
- **Database compatibility**: UUID-based schema enables database export/import across versions

**Version Configuration:**

| Version  | Modules Running | Features |
|----------|-----------------|----------|
| **Full** | 1, 2, 3, 4 | All features, local Essentia analysis, file ingest |
| **Lite** | 1, 2, 3 | No file ingest, automatic selection, limited config UI |
| **Minimal** | 1, 2 | Playback only, manual queue management, no automatic selection |

**Module Binary Build Variants:**
- Each module may be compiled with version-specific features using Rust feature flags
- Example: Module 4 only compiled for Full version
- Module 2 UI may have conditional features for Full/Lite/Minimal
- See [Implementation Order - Version Builds](implementation_order.md#27-version-builds-fulliteminimal) for build details

## Technology Stack

### Core Technologies

**Programming Language:**
- Rust (stable channel)
- Async runtime: Tokio

**HTTP Server Framework (all modules):**
- Axum or Actix-web (to be determined during implementation)
- Server-Sent Events (SSE) support
- JSON request/response handling

**Audio Processing (Module 1 only):**
- GStreamer 1.x
- Rust bindings: gstreamer-rs

**Database:**
- SQLite 3.x (embedded in each module)
- rusqlite crate for Rust bindings
- JSON1 extension for flavor vector storage

**External API Clients:**
- reqwest for HTTP clients
- MusicBrainz, AcousticBrainz, Chromaprint/AcoustID

**Local Audio Analysis (Module 4, Full version only):**
- Essentia C++ library
- Rust FFI bindings (custom or via existing crate)

**Web UI (Module 2 and Module 4):**
- HTML/CSS/JavaScript (framework TBD - React, Vue, or Svelte)
- SSE client for real-time updates
- Responsive design framework (TailwindCSS or similar)

**Configuration:**
- TOML or JSON configuration files
- Environment variables for deployment settings

**Build System:**
- Cargo workspaces for multi-module project
- Separate binaries for each module
- Feature flags for version differentiation

### Removed Technologies

**Tauri** - Previously planned for monolithic desktop app, no longer needed:
- Replaced by: Standalone HTTP servers with web UIs
- Benefit: Simpler deployment, network-accessible, no desktop framework dependency

---

## Platform Abstraction

### Audio Output
```
┌──────────────────────┐
│  Platform Detector   │
│  (Runtime detection) │
└──────────┬───────────┘
           │
    ┌──────┴──────┬──────────┬──────────┐
    │             │          │          │
┌───▼────┐  ┌────▼────┐ ┌───▼────┐ ┌───▼────┐
│ ALSA   │  │PulseAudio│ │CoreAudio│ │WASAPI │
│(Linux) │  │ (Linux) │ │ (macOS) │ │(Windows)│
└────────┘  └─────────┘ └────────┘ └────────┘
```

**Auto-detection Priority:**
- Linux: PulseAudio → ALSA (Phase 1)
- Windows: WASAPI (Phase 1)
- macOS: CoreAudio (Phase 2)

**Manual Override:**
- User can select specific sink
- User can choose specific output device
- Settings persisted in database

### System Integration

**Auto-start:**
- Linux: systemd service unit (Phase 1)
- Windows: Task Scheduler XML (Phase 1)
- macOS: launchd plist (Phase 2)

**File Paths:**
- Database: Platform-specific app data directory
- Settings: Platform-specific config directory
- Logs: Platform-specific log directory

## Security Considerations

### Web UI

**Local Network Access:**
- HTTP only on `localhost:5720`
- Binds to localhost on startup (critical failure if port unavailable)
- Accessible via:
  - Localhost: `http://localhost:5720` (no network required)
  - Local network: `http://<machine-ip>:5720` (requires local network)
  - Remote: User must configure port forwarding (not recommended)

**Authentication:**
- User authentication system with three modes:
  - Anonymous access (shared UUID, no password)
  - Account creation (unique UUID, username/password)
  - Account login (existing credentials)
- Session persistence via browser localStorage (one-year rolling expiration)

**Security:**
- CORS restricted to localhost
- No external network exposure by default
- User responsible for network security if exposing to local network
- No internet access required for WebUI operation (local network only)

### Database
- SQLite with file permissions (user-only read/write)
- Passwords stored as salted hashes (never plaintext)
- Salt incorporates user UUID for additional security
- File paths only (no file contents stored in database)
- User taste data (likes/dislikes) considered non-sensitive
- Anonymous user data is shared across all anonymous sessions

### Internet Access (External APIs)

**Purpose:**
- MusicBrainz metadata lookup
- AcousticBrainz flavor data retrieval
- Cover art fetching
- Future ListenBrainz integration (Phase 2)

**Connectivity:**
- Required only during library import/update (Full version)
- Not required for playback or local database operations
- Retry logic: 20 attempts with 5-second delays
- User-triggered reconnection after retry exhaustion

**Error Handling:**
- Network failures do not impact playback
- Cached data used when internet unavailable
- User notified of degraded functionality
- Local Essentia analysis used when AcousticBrainz unavailable (Full version)

**Version Differences:**
- **Full version**: Internet required for initial setup, optional thereafter
- **Lite version**: Internet optional (ListenBrainz sync only, Phase 2)
- **Minimal version**: No internet access at all

**Security:**
- HTTPS for all external requests
- API keys in environment variables (if required)
- Rate limiting to respect service terms
- Offline fallback for all features

## Performance Targets

### Raspberry Pi Zero2W (Lite/Minimal)
- Startup time: < 5 seconds
- Memory usage: < 256MB
- Selection time: < 500ms for 10k passage library
- Crossfade latency: < 50ms gap
- UI responsiveness: < 100ms for user actions

### Desktop (Full)
- Startup time: < 3 seconds
- Memory usage: < 512MB
- Essentia analysis: < 30 seconds per passage
- Concurrent scan: 100+ files/second
- Selection time: < 200ms for 100k passage library

## Error Handling Strategy

### Categories

**Recoverable Errors:**
- Network failures → Retry with exponential backoff
- Missing files → Skip, remove from queue, log
- Database lock → Retry with timeout
- Decode errors → Skip to next passage

**Non-recoverable Errors:**
- Database corruption → Alert user, attempt repair
- Configuration errors → Reset to defaults, warn user
- Critical GStreamer failures → Restart pipeline

### Logging

**Levels:**
- ERROR: System failures, data corruption
- WARN: Recoverable issues, missing data
- INFO: State changes, significant events
- DEBUG: Detailed operation info
- TRACE: Fine-grained execution flow

**Output:**
- stdout/stderr with timestamps
- File rotation (max 10MB per file, keep 5)
- Structured logging with `tracing` crate
- File:line identification in all messages

## Deployment

### Database Migrations
- Version tracking in `schema_version` table
- Forward-only migrations
- Automatic on startup (with backup)
- Rollback support for critical failures

### Configuration
- Environment variables for system paths
- SQLite settings table for user preferences
- File-based config for deployment settings
- Sane defaults for all optional settings

### Distribution
- **Multiple binaries per version**: Each module is a separate binary
- **Version-specific packaging**:
  - Full: 4 binaries (modules 1, 2, 3, 4)
  - Lite: 3 binaries (modules 1, 2, 3)
  - Minimal: 2 binaries (modules 1, 2)
- **Bundled dependencies**: GStreamer (Module 1 only), SQLite (all modules)
- **Installer packages**: deb, rpm, msi, dmg with systemd/launchd service files
- **Process management**: System service manager or manual startup scripts
- **Configuration files**: Default ports, module URLs, database path

## Future Architecture Considerations

### Scalability
- Current design: single-user, local database
- Future: Multi-user with centralized database
- Future: Cloud sync for preferences/history
- Future: Collaborative playlists and flavor sharing

### Mobile (Flutter Rewrite)
- Shared Rust core via FFI
- Flutter UI layer
- Platform-specific audio engines
- Background playback support
- Offline-first architecture

### Advanced Features
- Machine learning for preference inference
- Real-time collaborative listening
- Plugin system for custom selectors
- External player control protocols (MPD, etc.)

----
End of document - McRhythm Architecture
