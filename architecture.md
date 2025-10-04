# McRhythm Architecture

> **Related Documentation:** [Requirements](requirements.md) | [Database Schema](database_schema.md) | [Implementation Order](implementation_order.md)

## Overview

McRhythm is a music player built on Rust, GStreamer, SQLite, and Tauri that automatically selects music passages based on user-configured musical flavor preferences by time of day, using cooldown-based probability calculations and AcousticBrainz musical characterization data.

## System Architecture

### Layered Architecture

```
┌────────────────────────────────────────────────────────────┐
│                  Presentation Layer                        │
│              (Tauri + Web UI)                              │
│         HTML/CSS/JavaScript Frontend                       │
│         Server-Sent Events for Real-time Updates           │
└────────────────────────────────────────────────────────────┘
                          ▼
┌────────────────────────────────────────────────────────────┐
│                    API Layer                               │
│              REST Endpoints + SSE Endpoint                 │
│    Request Validation & Command Queuing                    │
└────────────────────────────────────────────────────────────┘
                          ▼
┌────────────────────────────────────────────────────────────┐
│                Business Logic Layer                        │
│  ┌────────────────┐  ┌──────────────┐  ┌────────────────┐  │
│  │Program Director│  │Queue Manager │  │ Playback Ctrl  │  │
│  │ (Probability + │  │(Auto-fill +  │  │ (Crossfade +   │  │
│  │  Flavor Match) │  │ Persistence) │  │  Transitions)  │  │
│  └────────────────┘  └──────────────┘  └────────────────┘  │
│  ┌────────────────┐  ┌──────────────┐  ┌────────────────┐  │
│  │   Historian    │  │Library Mgr   │  │ Flavor Mgr     │  │
│  │ (Cooldowns +   │  │(Scan + Index)│  │ (Timeslots +   │  │
│  │  Last Play)    │  │              │  │  Distance Calc)│  │
│  └────────────────┘  └──────────────┘  └────────────────┘  │
└────────────────────────────────────────────────────────────┘
                          ▼
┌────────────────────────────────────────────────────────────┐
│                Audio Engine Layer                          │
│  ┌─────────────────────────────────────────────────────┐   │
│  │           GStreamer Pipeline Manager                │   │
│  │  ┌──────────────┐            ┌──────────────┐       │   │
│  │  │ Pipeline A   │            │  Pipeline B  │       │   │
│  │  │ (Current)    │───────────▶│  (Next)      │       │   │
│  │  └──────────────┘            └──────────────┘       │   │
│  │           │                          │              │   │
│  │           └──────────┬───────────────┘              │   │
│  │                      ▼                              │   │
│  │              ┌──────────────┐                       │   │
│  │              │ Audio Mixer  │                       │   │
│  │              │ (Crossfade)  │                       │   │
│  │              └──────────────┘                       │   │
│  │                      ▼                              │   │
│  │              ┌──────────────┐                       │   │
│  │              │Volume Control│                       │   │
│  │              │(Fade Profiles│                       │   │
│  │              │ + User Vol)  │                       │   │
│  │              └──────────────┘                       │   │
│  └─────────────────────────────────────────────────────┘   │
│                      ▼                                     │
│              OS Audio Output                               │
│         (ALSA/PulseAudio/CoreAudio/WASAPI)                 │
└────────────────────────────────────────────────────────────┘
                          ▼
┌────────────────────────────────────────────────────────────┐
│              Library Management Layer                      │
│  ┌────────────────┐  ┌──────────────┐  ┌────────────────┐  │
│  │  File Scanner  │  │   Metadata   │  │  Fingerprint   │  │
│  │  (Recursive +  │  │   Extractor  │  │   Generator    │  │
│  │Change Detect)  │  │  (ID3/Tags)  │  │ (Chromaprint)  │  │
│  └────────────────┘  └──────────────┘  └────────────────┘  │
└────────────────────────────────────────────────────────────┘
                          ▼
┌────────────────────────────────────────────────────────────┐
│           External Integration Layer                       │
│  ┌────────────────┐  ┌──────────────┐  ┌────────────────┐  │
│  │  MusicBrainz   │  │AcousticBrainz│  │ ListenBrainz   │  │
│  │    Client      │  │   Client +   │  │    Client      │  │
│  │  (Track/Artist │  │   Essentia   │  │  (Plays/Likes) │  │
│  │  Identification│  │(Local Flavor)│  │                │  │
│  └────────────────┘  └──────────────┘  └────────────────┘  │
│     Rate Limiting & Offline Fallback                       │
└────────────────────────────────────────────────────────────┘
                          ▼
┌────────────────────────────────────────────────────────────┐
│                    Data Layer                              │
│                  SQLite Database                           │
│  Files | Passages | Songs | Artists | Works | Albums       │
│  Play History | Queue State | Settings | Timeslots         │
│  Musical Flavor Vectors | Album Art File Paths             │
└────────────────────────────────────────────────────────────┘
```

## Core Components

### 1. Playback Controller

**Responsibilities:**
- Manages dual GStreamer pipelines for seamless crossfading
- Coordinates passage transitions based on lead-in/lead-out timing
- Implements three fade profiles (exponential, cosine, linear)
- Handles pause/resume with 0.5s exponential fade-in
- Manages volume control (user level + fade automation)

**Key Operations:**
- Pre-load next passage in secondary pipeline
- Calculate crossfade start time based on lead-in/lead-out
- Apply volume curves during fade-in/out
- Switch primary/secondary pipelines on passage completion

**State:**
- Currently playing passage (position, duration, state)
- Next passage (pre-loaded, ready for crossfade)
- User volume level (0-100)
- Playback state (Playing/Paused/Stopped)

### 2. Program Director

**Responsibilities:**
- Calculate passage selection probabilities based on:
  - Base probability (song × artist)
  - Cooldown multipliers (song × artist × work)
  - Musical flavor distance from current target
- Implement weighted random selection algorithm
- Maintain time-of-day flavor targets

**Key Operations:**
- Filter to non-zero probability passages
- Calculate squared Euclidean distance from target flavor
- Sort by distance, take top 100
- Weighted random selection from candidates
- Handle edge cases (no candidates → Pause mode)

**Data Sources:**
- Current timeslot flavor target (or temporary override)
- Passage musical flavor vectors
- Song/artist/work last-play times
- User-configured base probabilities

### 3. Queue Manager

**Responsibilities:**
- Maintain playback queue (minimum 2 passages, 15+ minutes)
- Persist queue state to SQLite
- Handle manual user additions/removals
- Trigger automatic queue replenishment
- Enforce multi-user edge case rules

**Key Operations:**
- Add passage (append to queue)
- Remove passage (with concurrent operation handling)
- Auto-advance on passage completion
- Load/save queue on startup/shutdown
- Monitor queue depth and trigger selector

**Edge Cases:**
- Skip throttling (5-second window)
- Concurrent remove operations (ignore duplicates)
- Temporary override queue flush

### 4. Historian

**Responsibilities:**
- Record passage plays with timestamps
- Update last-play times for songs/artists/works
- Track completion status (played fully vs skipped)
- Calculate cooldown multipliers based on elapsed time

**Key Operations:**
- Record play event on passage start
- Update completion status on passage end/skip
- Query last-play time for cooldown calculation
- Calculate ramping multiplier (linear interpolation)

**Data Stored:**
- Passage ID, timestamp, duration played, completion status
- Last-play timestamps for songs, artists, works

### 5. Flavor Manager

**Responsibilities:**
- Manage 24-hour timeslot schedule
- Calculate flavor targets from selected passages
- Handle temporary flavor overrides
- Compute musical flavor distances

**Key Operations:**
- Determine current timeslot based on time-of-day
- Average passage flavor vectors for timeslot target
- Calculate squared Euclidean distance (binary + multi-dimensional)
- Apply temporary override (flush queue, skip current passage)

**Data:**
- Timeslot definitions (start time, passages)
- Computed flavor targets (averaged vectors)
- Active override (target, expiration time)

### 6. Audio Engine

**Architecture:**
```
Pipeline A:                          Pipeline B:
┌─────────────┐                     ┌─────────────┐
│ filesrc     │                     │ filesrc     │
│ location=A  │                     │ location=B  │
└──────┬──────┘                     └──────┬──────┘
       │                                   │
┌──────▼──────┐                     ┌──────▼──────┐
│  decodebin  │                     │  decodebin  │
│ (auto codec)│                     │ (auto codec)│
└──────┬──────┘                     └──────┬──────┘
       │                                   │
┌──────▼──────┐                     ┌──────▼──────┐
│audioconvert │                     │audioconvert │
└──────┬──────┘                     └──────┬──────┘
       │                                   │
┌──────▼──────┐                     ┌──────▼──────┐
│audioresample│                     │audioresample│
└──────┬──────┘                     └──────┬──────┘
       │                                   │
       └────────────┬──────────────────────┘
                    │
            ┌───────▼────────┐
            │  audiomixer    │
            │  (crossfade)   │
            └───────┬────────┘
                    │
            ┌───────▼────────┐
            │    volume      │
            │ (controller)   │
            └───────┬────────┘
                    │
            ┌───────▼────────┐
            │   autoaudiosink│
            │ or manual sink │
            └────────────────┘
```

**Crossfade Timing Logic:**
```
Passage A: |lead-in]------------[lead-out|
Passage B:                       |lead-in]------------[lead-out|

If lead-out(A) < lead-in(B):
  Start B when A reaches lead-out point

If lead-out(A) > lead-in(B):
  Start B when A has lead-in(B) time remaining
```

### 7. Library Manager

**Responsibilities:**
- Scan directories for audio files (Full version only)
- Extract metadata from file tags
- Generate audio fingerprints (Chromaprint)
- Detect file changes (modified/deleted/added)
- Handle multi-passage file segmentation

**Key Operations:**
- Recursive directory scan with change detection (SHA-256 hashes)
- Parse ID3v2, Vorbis Comments, MP4 tags
- Silence detection for multi-passage segmentation
- Associate passages with MusicBrainz entities

**Data Stored:**
- File paths, modification times, hashes
- Extracted metadata (title, artist, album, etc.)
- Album art file paths (stored in same directory as audio files)
- Passage boundaries within files

### 8. External Integration Clients

**MusicBrainz Client:**
- Query: Recording/Release/Artist/Work IDs
- Fetch: Canonical names, dates, genres/tags
- Cache: All responses locally (indefinite retention)
- Offline: Continue with cached data

**AcousticBrainz Client:**
- Query: High-level musical characterization vectors
- Parse: Binary classifications + multi-dimensional genres/rhythms/moods
- Fallback: Essentia local analysis (Full version)
- Cache: All vectors in passage table

**ListenBrainz Client:**
- Submit: Play history, likes/dislikes (TBD)
- Fetch: Recommendations, taste profile (TBD)
- Effect: Inform selection algorithm (TBD)

**Rate Limiting:**
- AcoustID: 3 requests/second
- Network failures: 5s delay, 20 max retries

## Concurrency Model

### Threading Architecture

```
Main Thread:
  - Tauri event loop
  - UI coordination
  - Command dispatch

Audio Thread (GStreamer):
  - Pipeline execution
  - Crossfade timing
  - Volume automation
  - Isolated from blocking I/O

Program Director Thread (tokio async):
  - Passage selection algorithm
  - Distance calculations
  - Probability computations
  - Triggered by queue manager

Scanner Thread (tokio async):
  - File system scanning
  - Metadata extraction
  - Fingerprint generation
  - Full version only

API Thread Pool (tokio async):
  - HTTP request handling
  - SSE broadcasting
  - External API calls
  - Database queries
```

### Inter-component Communication

**Async Channels (tokio mpsc):**
- Playback commands: API → Playback Controller
- Selection requests: Queue Manager → Program Director
- State updates: Any → SSE Broadcaster
- Play events: Playback Controller → Historian

**Shared State (Arc<RwLock<T>>):**
- Current playback state (position, passage, status)
- Queue contents (read-heavy, write-light)
- Timeslot configuration (read-heavy)
- User settings (volume, preferences)

**GStreamer Bus:**
- Pipeline events (EOS, error, state change)
- Position queries (every 500ms)
- Crossfade triggers

**SSE Broadcaster:**
- Maintains connected client list
- Broadcasts state changes to all clients
- Non-blocking message delivery

## Data Model

### Core Entities

**Files:**
- `guid`, `path`, `hash`, `modification_time`, `created_at`, `updated_at`

**Passages:**
- `guid`, `file_id`, `start_time`, `fade_in_time`, `lead_in_time`, `lead_out_time`, `fade_out_time`, `end_time`
- `fade_profile` (exponential/cosine/linear)
- `musical_flavor_vector` (JSON blob or normalized columns)
- `title`, `artist`, `album` (from file tags)

**Songs:**
- `guid`, `recording_mbid`, `primary_artist_mbid`
- `base_probability` (default 1.0, range 0.0-1000.0)
- `min_cooldown`, `ramping_cooldown`
- `last_played_at`

**Artists:**
- `guid`, `artist_mbid`, `name`
- `base_probability`
- `min_cooldown`, `ramping_cooldown`
- `last_played_at`

**Works:**
- `guid`, `work_mbid`, `title`
- `min_cooldown`, `ramping_cooldown` (TBD)
- `last_played_at`

**PassageSongs (junction):**
- `passage_id`, `song_id`

**PlayHistory:**
- `guid`, `passage_id`, `timestamp`, `duration_played`, `completed`

**Queue:**
- `position`, `passage_id`, `created_at`

**Timeslots:**
- `guid`, `start_hour`, `start_minute`, `name`

**TimeslotPassages (junction):**
- `timeslot_id`, `passage_id`

**Settings:**
- `key`, `value` (JSON)
- Global settings, preferences, temporary overrides

**Albums:**
- `guid`, `album_mbid`, `title`, `release_date`
- `front_art_path`, `back_art_path` (file paths to image files)

### Indexes

- `passages.file_id`
- `songs.last_played_at`
- `artists.last_played_at`
- `works.last_played_at`
- `play_history.timestamp`
- `play_history.passage_id`

## Version Differentiation

### Full Version
**Features:**
- All components enabled
- File scanning and library management
- Essentia local analysis for musical flavor
- Database building and maintenance
- Preference editing
- MusicBrainz/AcousticBrainz/ListenBrainz integration

**Target Platforms:**
- Linux desktop
- Windows
- macOS

**Resource Profile:**
- Higher CPU (Essentia analysis)
- Higher disk I/O (file scanning)
- Network access required for initial setup

### Lite Version
**Features:**
- Player + selector + preference editing
- Uses pre-built static database from Full version
- Read-only external data (MusicBrainz/AcousticBrainz cached)
- Timeslot and probability editing allowed
- No file scanning, no Essentia

**Target Platforms:**
- Raspberry Pi Zero2W
- Linux desktop
- Windows, macOS

**Resource Profile:**
- Moderate CPU (playback + selection only)
- Low disk I/O (no scanning)
- Optional network (for ListenBrainz sync)

### Minimal Version
**Features:**
- Player + selector only
- Read-only database and preferences
- No editing capabilities
- Pre-configured timeslots and probabilities
- Smallest memory footprint

**Target Platforms:**
- Raspberry Pi Zero2W
- Embedded systems
- Resource-constrained devices

**Resource Profile:**
- Minimal CPU (playback + basic selection)
- Minimal disk I/O (read-only database)
- No network access required

### Version Build Strategy

**Rust Feature Flags:**
```toml
[features]
default = ["minimal"]
minimal = []
lite = ["minimal", "preference-editing"]
full = ["lite", "library-management", "essentia-analysis"]
```

**Conditional Compilation:**
```rust
#[cfg(feature = "full")]
mod library_scanner;

#[cfg(feature = "lite")]
mod preference_editor;

#[cfg(not(feature = "minimal"))]
fn allow_editing() { /* ... */ }
```

**Database Export/Import:**
- Full version exports complete database snapshot
- Lite/Minimal versions import read-only database
- Migration tools for version upgrades

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
- Linux: PulseAudio → ALSA
- macOS: CoreAudio
- Windows: WASAPI

**Manual Override:**
- User can select specific sink
- User can choose specific output device
- Settings persisted in database

### System Integration

**Auto-start:**
- Linux: systemd service unit
- Windows: Task Scheduler XML
- macOS: launchd plist

**File Paths:**
- Database: Platform-specific app data directory
- Settings: Platform-specific config directory
- Logs: Platform-specific log directory

## Security Considerations

### Web UI
- HTTP only (no authentication) on `localhost:5720`
- CORS restricted to localhost
- No external network exposure
- User responsible for network security

### Database
- SQLite with file permissions (user-only read/write)
- No sensitive data stored
- File paths only (no file contents stored in database)

### External APIs
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
- Single binary per platform/version
- Bundled dependencies (GStreamer, SQLite)
- Installer packages (deb, rpm, msi, dmg)
- Tauri auto-updater for desktop versions

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
