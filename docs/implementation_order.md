# WKMP Implementation Order

**📋 TIER 4 - DOWNSTREAM EXECUTION PLAN**

This document aggregates all specifications to define the order in which features are built.

**Update Policy:** ✅ Always update when upstream docs change | ❌ NEVER update upstream docs from this

> **Architecture Note:** WKMP uses a microservices architecture with 5 independent HTTP servers (Audio Player, User Interface, Lyric Editor, Program Director, Audio Ingest). The Lyric Editor is launched on-demand but is still an independent process. This implementation plan reflects the module-based design. See [Architecture](architecture.md) for complete details.

> **Related Documentation:** [Database Schema](database_schema.md) | [API Design](api_design.md) | [Event System](event_system.md) | [Coding Conventions](coding_conventions.md)

---

## Phase 1: Foundation & Database

*Goal: Establish shared database schema, core infrastructure, and Cargo workspace structure.*

- **1.0. Workspace Setup:**
  - Create Cargo workspace structure (see `project_structure.md`)
  - Set up root `Cargo.toml` with workspace members and shared dependencies
  - Create `common/` library crate (`wkmp-common`) for shared code
  - Create binary crates: `wkmp-ap/`, `wkmp-ui/`, `wkmp-le/`, `wkmp-pd/`, `wkmp-ai/`
  - Set up packaging scripts for Full/Lite/Minimal versions:
    - Full: Package all 5 binaries (wkmp-ap, wkmp-ui, wkmp-le, wkmp-pd, wkmp-ai)
    - Lite: Package 3 binaries (wkmp-ap, wkmp-ui, wkmp-pd)
    - Minimal: Package 2 binaries (wkmp-ap, wkmp-ui)
  - Configure basic CI/CD pipeline (GitHub Actions or GitLab CI):
    - Automated builds on every commit (cargo build --release)
    - Build all workspace members (all 5 binaries + common library)
    - Basic compilation error detection
    - Branch protection for main branch
    - Run unit tests from Phase 1.2 onward (cargo test)
    - Add integration tests starting in Phase 2.0
    - Coverage tracking to be added later (Phase 12.5)
  - Set up development environment and initial project structure

- **1.1. Database Schema & Migrations:**
  - Define SQL schema for all tables from `database_schema.md`:
    - Core entities: files, passages, songs, artists, works, albums
    - Relationships: passage_songs, song_artists, passage_albums, song_works
    - Configuration: **module_config**, timeslots, timeslot_passages, settings, users
    - Playback: play_history, queue, likes_dislikes
    - Caching: acoustid_cache, musicbrainz_cache, acousticbrainz_cache
    - Images table for multi-entity image storage
  - Define triggers for automatic timestamp updates and cooldown tracking
  - **Note:** Database migration framework will be developed as-needed prior to release of a version that introduces breaking schema changes
  - **Note**: No standalone database initialization - each module creates its required tables on first startup (see Phase 1.2)

- **1.2. Common Library Foundation:**
  - Implement shared types in `common/` crate (`wkmp-common`):
    - Database models (Passage, Song, Artist, Work, Album, etc.)
    - Event types (`WkmpEvent` enum from `event_system.md`)
    - API request/response types
  - **Shared database initialization functions** (`wkmp-common/src/db/init.rs`):
    - Table creation functions for commonly used tables:
      - `init_module_config_table()` - Create and populate `module_config` with defaults
      - `init_settings_table()` - Create `settings` table
      - `init_users_table()` - Create `users` table and ensure Anonymous user exists
        - Create table if missing
        - Check if Anonymous user exists (guid: `00000000-0000-0000-0000-000000000001`)
        - If Anonymous user missing, insert with username "Anonymous", empty password_hash, empty password_salt
        - Idempotent: Safe to call on every startup
      - Each function checks if table exists before creating (idempotent operations)
    - Default module configurations:
      - user_interface: 127.0.0.1:5720
      - audio_player: 127.0.0.1:5721
      - program_director: 127.0.0.1:5722
      - audio_ingest: 127.0.0.1:5723
      - lyric_editor: 127.0.0.1:5724
    - Each module calls relevant init functions on startup (see module scaffolding phases)
  - **Module configuration loader** with error handling (`wkmp-common/src/db/config.rs`):
    - Read `module_config` table (calls `init_module_config_table()` if missing)
    - If own module's config is missing, insert default host/port and log warning
    - If other required modules' configs are missing, insert defaults and log warning
    - Return module network configuration for HTTP client setup
  - **Module launcher utility** (`wkmp-common/src/launcher.rs`):
    - Locate binary for module (e.g., `wkmp-pd`, `wkmp-ap`)
    - **Standard deployment**: Assume binaries in system PATH (no arguments needed)
    - **Non-standard deployment**: Pass optional `--binary-path` argument received by launching module
    - Launch subprocess with optional `--binary-path` argument (propagates to launched module)
    - No other command-line arguments required (modules configured via database)
    - Return process handle for monitoring
  - **Relaunch throttling logic** (`wkmp-common/src/launcher.rs`):
    - Read relaunch parameters from settings table:
      - `relaunch_delay` (default: 5 seconds) - Wait time between relaunch attempts
      - `relaunch_attempts` (default: 20) - Maximum relaunch attempts before giving up
    - Track relaunch attempt count per module
    - After failure, wait `relaunch_delay` seconds before next attempt
    - After `relaunch_attempts` exhausted, stop and return error to caller
    - Caller (e.g., wkmp-ui, wkmp-ap) displays error with user control to restart attempts
  - UUID generation helpers (`wkmp-common/src/uuid.rs`)
  - Timestamp utilities (`wkmp-common/src/time.rs`)
  - Common error types (`wkmp-common/src/error.rs`)
  - Shared serialization/deserialization helpers
  - All binary crates depend on `wkmp-common` for shared types and logic

---

## Phase 2: Audio Player Module

*Goal: Build the core playback engine as an independent HTTP server with minimal developer UI.*

- **2.1. Module Scaffolding:**
  - Implement `wkmp-ap` binary in workspace (already created in Phase 1.0)
  - Implement HTTP server (Axum or Actix-web) with module discovery:
    1. Read root folder path from config file or environment variable
    2. Open database file (`wkmp.db` in root folder)
    3. Initialize required database tables using shared common library functions:
       - Call `init_module_config_table()` - Creates `module_config` with all defaults
       - Call `init_settings_table()` - Creates `settings` table
       - Create Audio Player-specific tables: `queue`, `play_history`
       - All initialization is idempotent (safe to call multiple times)
    4. Read module configuration using shared config loader from Phase 1.2:
       - Loader calls `init_module_config_table()` if table missing
       - If own config missing, inserts default and logs warning
    5. Retrieve own host/port configuration (audio_player entry, default: 127.0.0.1:5721)
    6. Bind to configured address/port
    7. No other module dependencies (Audio Player is standalone)
  - Create minimal HTML developer UI (served via HTTP) for status display and event stream monitor
  - Implement SSE endpoint (`GET /events`) with basic broadcasting

- **2.2. Basic Playback Engine:**
  - Implement basic single-stream playback (without crossfading initially)
  - Create HTTP API endpoints:
    - `POST /playback/play` - Resume playback
    - `POST /playback/pause` - Pause playback
    - `POST /playback/seek` - Seek to position within current passage (0 to passage duration in seconds)
      - Seeking to passage duration is equivalent to skip
    - `GET /playback/state` - Get playing/paused state
    - `GET /playback/position` - Get current position
  - Emit basic events: `PassageStarted`, `PassageCompleted`, `PlaybackStateChanged`
  - Implement position updates (every 500ms)

- **2.3. Queue Management:**
  - **Testing Requirements:**
    - Unit tests for playback state transitions
    - Unit tests for queue operations (enqueue, dequeue, reorder)
    - Integration tests for HTTP API endpoints
    - Mock audio output for testing playback without actual audio
  - Implement queue persistence to database
  - Create HTTP API endpoints:
    - `POST /playback/enqueue` - Add passage to queue
    - `DELETE /playback/queue/{passage_id}` - Remove from queue
    - `GET /playback/queue` - Get queue contents
    - `POST /playback/skip` - Skip to next passage in queue
  - Implement auto-advance to next passage on completion
  - Emit `QueueChanged` events

- **2.4. Audio Control:**
  - Implement volume control:
    - `POST /audio/volume` - Set volume (0-100)
    - `GET /audio/volume` - Get current volume
    - Emit `VolumeChanged` events
  - Implement audio device selection:
    - `POST /audio/device` - Set output device
    - `GET /audio/device` - Get current device
  - Platform-specific sink detection (PulseAudio, ALSA, CoreAudio, WASAPI)

- **2.5. Historian:**
  - Record passage plays to `play_history` table
  - Update `last_played_at` timestamps (via database triggers)
  - Track duration_played and completion status

- **2.6. Single-Stream Crossfade Engine:**
  - **Complex task** - Implement single stream audio architecture with sample-accurate crossfading
  - Integrate audio decoder using `symphonia` crate:
    - Support MP3, FLAC, AAC, Vorbis, Opus, and other common formats
    - Handle sample rate conversion with `rubato`
    - Decode passages to PCM buffers in memory (interleaved stereo f32)
    - Seek to passage start position for accurate playback
  - Integrate audio output using `cpal` crate:
    - Ring buffer for smooth audio delivery to output device
    - Support PulseAudio, ALSA, CoreAudio, WASAPI backends
    - Handle buffer underruns gracefully
    - Mixer thread to keep ring buffer filled from CrossfadeMixer
  - Leverage existing single_stream components (already implemented with 28/28 tests passing):
    - Fade curve algorithms (`curves.rs`) - 5 curve types (Linear, Logarithmic, Exponential, S-Curve, Equal-Power) ✅
    - PCM buffer management (`buffer.rs`) - automatic fade application during sample read ✅
    - Sample-accurate crossfade mixer (`mixer.rs`) - per-sample mixing with automatic crossfade detection ✅
  - Calculate crossfade timing from passage lead-in/lead-out points (same logic as dual pipeline)
  - Support five fade curves: Linear, Logarithmic, Exponential, S-Curve, Equal-Power (already implemented)
  - Emit `CurrentSongChanged` events for multi-song passages
  - **Performance**: ~0.02ms crossfade precision (500-2500x better than GStreamer dual pipeline)
  - **Memory**: ~27 MB for 5 buffered passages (6x reduction vs dual pipeline)

- **2.7. Queue Refill Request System:**
  - Monitor queue status continuously during playback
  - Calculate remaining queue time based on:
    - Current passage position and remaining duration
    - Durations of all queued passages
  - Trigger queue refill when below thresholds (configurable in settings table):
    - Default: < 2 passages (`queue_refill_threshold_passages`)
    - Default: < 15 minutes (`queue_refill_threshold_seconds` = 900)
    - Condition: Refill triggered when **either** threshold is met (< queue_refill_threshold_passages value passages **OR** < queue_refill_threshold_seconds remaining)
  - Send `POST /selection/request` to Program Director with:
    - Anticipated start time for new passage
    - Current queue state
    - **TODO:** Specify detailed request/response protocol including error handling for duplicate requests and Program Director crash scenarios (to be addressed prior to Program Director implementation)
  - Throttle requests while queue is underfilled:
    - Configurable interval in settings table (`queue_refill_request_throttle_seconds`)
    - Default: 10 seconds between requests
  - Handle Program Director unavailability with relaunch throttling:
    - Wait for acknowledgment with configurable timeout (`queue_refill_acknowledgment_timeout_seconds`)
    - Default timeout: 5 seconds
    - If no response, attempt to relaunch wkmp-pd using shared launcher utility:
      - **Binary location**: Assume `wkmp-pd` in system PATH (standard deployment)
      - **Non-standard deployment**: If wkmp-ap received `--binary-path` argument, pass it to wkmp-pd
      - **No other arguments**: wkmp-pd configured via database, no command-line config needed
      - Launch subprocess and get process handle
    - **Relaunch throttling** (using shared logic from wkmp-common):
      - Read `relaunch_delay` (default: 5 seconds) and `relaunch_attempts` (default: 20) from settings
      - Track relaunch attempt count for wkmp-pd
      - After launch failure, wait `relaunch_delay` seconds before next attempt
      - Maximum `relaunch_attempts` attempts before giving up
      - After exhausting attempts, display error in developer UI with "Retry" button
      - User can click "Retry" to reset attempt counter and resume relaunching
    - Continue playback normally even if refill and all relaunch attempts fail
    - Log all launch attempts, failures, and user-initiated retry actions
  - Note: Program Director may also enqueue passages proactively without requests

#### Request Deduplication Strategy

**Problem:** Network retries between Audio Player and Program Director may cause duplicate passage selection requests.

**Solution: UUID-Based Idempotency**

**Audio Player (wkmp-ap) Responsibilities:**
1. Generate unique `request_id = Uuid::new_v4()` for each refill request
2. Include `request_id` in `POST /selection/request` body
3. Cache sent requests: `HashMap<request_id, request_timestamp>`
4. On retry: Resend same `request_id` (not new UUID)
5. Cleanup: Remove cache entries after acknowledgment received

**Program Director (wkmp-pd) Responsibilities:**
1. Track in-flight requests: `HashMap<request_id, (timestamp, passage_id_result)>`
2. On request receive:
   - If `request_id` exists in map: Return cached acknowledgment immediately (no re-selection)
   - If `request_id` new: Perform passage selection, cache result with request_id, return acknowledgment
3. Cleanup: Remove map entries older than 5 minutes (stale request timeout)

**Benefits:**
- Idempotent operations: Safe to retry without duplicate selection
- Resource efficiency: No wasteful re-computation of selection algorithm
- Network resilience: Handles temporary connectivity issues gracefully

**Implementation Note:** This enhancement should be added during Phase 2.7 implementation. It does not affect initial prototype functionality but improves production robustness.

**Traceability:** ARCH-ERRH-010 (error recovery strategy)

---

## Phase 2A: Lyric Editor Module

*Goal: Build a standalone lyric editing tool launched on-demand by the User Interface.*

- **2A.1. Module Scaffolding:**
  - Implement `wkmp-le` binary in workspace
  - Implement HTTP server with module discovery:
    1. Read root folder path from config file or environment variable
    2. Open database file (`wkmp.db` in root folder)
    3. Initialize required database tables using shared common library functions:
       - Call `init_module_config_table()` - Creates `module_config` with all defaults
       - Create Lyric Editor-specific tables: `songs` table (if not already created)
       - All initialization is idempotent (safe to call multiple times)
    4. Read module configuration using shared config loader from Phase 1.2:
       - Loader calls `init_module_config_table()` if table missing
       - If own config missing, inserts default and logs warning
    5. Retrieve own host/port configuration (lyric_editor entry, default: 127.0.0.1:5724)
    6. Bind to configured address/port
    7. No other module dependencies (Lyric Editor is standalone)
  - Create split-window UI framework:
    - Left pane: Text editor for lyrics
    - Right pane: Embedded web browser
  - Implement SSE endpoint (`GET /events`) for status updates

- **2A.2. Lyric Editing Interface:**
  - Create HTTP API endpoints:
    - `POST /lyric_editor/open` - Launch editor with recording MBID
      - Parameters: `recording_mbid` (UUID), `title` (string), `artist` (string)
      - Load current lyrics from `songs.lyrics` column via recording_mbid
      - Initialize embedded browser with search for "song_title artist lyrics"
    - `GET /lyric_editor/lyrics/{recording_mbid}` - Get current lyrics for recording
    - `PUT /lyric_editor/lyrics/{recording_mbid}` - Save edited lyrics
      - Body: Plain UTF-8 text
      - Update `songs.lyrics` column for the specified recording_mbid
      - Return success/failure status
    - `POST /lyric_editor/close` - Close editor (no save)
  - Implement text editor component:
    - Load lyrics from database on open
    - Multi-line text editing with UTF-8 support
    - Save button to persist to database
    - Cancel/Exit button to close without saving
  - Implement embedded browser component:
    - **TODO:** Web view technology selection (WebKit/WebView2/Qt WebEngine) with platform-specific dependencies and build requirements will be specified prior to wkmp-le module implementation
    - Platform-specific web view (WebKit/WebView2/Qt WebEngine)
    - Initial navigation to lyrics search query
    - User can freely navigate to find lyrics sources
    - Copy-paste support from browser to editor
  - Implement last-write-wins concurrency:
    - No locking or conflict detection
    - Most recent PUT request overwrites existing lyrics
    - Emit `LyricsChanged` event on successful save

---

## Phase 3: User Interface Module

*Goal: Build the polished web UI as an HTTP server that proxies requests to other modules.*

- **3.1. Module Scaffolding:**
  - Implement `wkmp-ui` binary in workspace (already created in Phase 1.0)
  - Implement HTTP server with module discovery and service launching:
    1. Read root folder path from config file or environment variable
    2. Open database file (`wkmp.db` in root folder)
    3. Initialize required database tables using shared common library functions:
       - Call `init_module_config_table()` - Creates `module_config` with all defaults
       - Call `init_settings_table()` - Creates `settings` table
       - Call `init_users_table()` - Creates `users` table with Anonymous user
       - Create User Interface-specific tables: `likes_dislikes`
       - All initialization is idempotent (safe to call multiple times)
    4. Read module configuration using shared config loader from Phase 1.2:
       - Loader calls `init_module_config_table()` if table missing
       - If own config missing, inserts default and logs warning
       - If other required modules' configs missing, inserts defaults and logs warnings
    5. Retrieve own host/port configuration (user_interface entry, default: 127.0.0.1:5720)
    6. Bind to configured address/port
    7. Retrieve other modules' configurations for HTTP client setup:
       - Audio Player URL (for playback control proxying)
       - Program Director URL (for configuration proxying)
       - Audio Ingest URL (for file ingest, Full only)
       - Lyric Editor URL (for on-demand launch)
    8. Implement service launching logic using shared launcher utility (wkmp-ui is the primary entry point):
       - **Command-line argument handling**:
         - Accept optional `--binary-path <path>` argument for non-standard deployments
         - If provided, pass same argument to all launched modules
         - Standard deployment: No arguments needed, binaries in system PATH
       - Check if required services are running by attempting HTTP connection
       - If service not responding, launch using shared launcher utility from wkmp-common:
         - Launch `wkmp-ap` if not responding on configured port (all versions)
         - Launch `wkmp-pd` if not responding on configured port (Lite/Full only)
         - Launch `wkmp-ai` if not responding on configured port (Full only)
         - Launch `wkmp-le` on-demand when user requests lyric editing (Full only)
       - **Binary location**: Assume binaries in system PATH (wkmp-ap, wkmp-pd, etc.)
       - **Non-standard deployment**: Use `--binary-path` to specify directory
       - **No other arguments**: Modules configured via database, not command-line
       - **Relaunch on failure**: Use shared relaunch throttling logic
         - Read `relaunch_delay` and `relaunch_attempts` from settings
         - Display errors in web UI with "Retry" button after attempts exhausted
       - Log all module launches, failures, and retry actions
  - Set up static file serving for web UI assets
  - Implement SSE aggregation (subscribe to Audio Player events, forward to clients)

- **3.2. Authentication System:**
  - **Database Setup**:
    - Verify `users` table initialized by `init_users_table()` from Phase 1.2
    - Verify Anonymous user exists (guid: `00000000-0000-0000-0000-000000000001`)
    - On startup, if Anonymous user missing, call `init_users_table()` to create it
  - **Password Hashing Implementation** (lightweight security, not high-value account protection):
    - Generate random 64-bit integer salt
    - Encode salt to base64 UTF-8 using RFC 4648 encoding
    - Concatenate: `salt + user_guid + password`
    - Calculate hash using SHA3-256 hashing algorithm
    - Store both `password_salt` (base64 encoded) and `password_hash` (hex encoded) in `users` table
    - Note: Fast and lightweight design, protects password from discovery but not attempting high-security protection
  - **Session Token Generation**:
    - Generate session token as UUID v4
    - Store in browser localStorage with key `wkmp_session_token` (per CO-299)
    - Associate token with user_id in server-side session store (in-memory or database)
    - Set expiration: 1 year from last use (rolling expiration)
    - On each authenticated request, extend expiration by 1 year
  - **API Endpoints** (implementing `user_identity.md`):
    - `POST /api/register` - Create new user account
      - Validate username uniqueness
      - Generate salt, hash password, create user record
      - Return session token for immediate login
    - `POST /api/login` - Authenticate existing user
      - Look up user by username
      - Retrieve stored salt and hash
      - Hash provided password with stored salt
      - Compare hashes, return session token if match
    - `POST /api/logout` - End session
      - Invalidate session token
      - Remove from session store
    - `GET /api/current-user` - Get authenticated user info
      - Validate session token from request header or cookie
      - Return user guid, username (exclude password_hash/salt)
  - **localStorage Session Persistence**:
    - Store session token in localStorage (persists across browser restarts)
    - On page load, check for existing session token
    - Validate token with server via `/api/current-user`
    - If invalid, clear localStorage and show login
    - If valid, extend expiration and proceed as authenticated user

- **3.3. Playback Control Proxying:**
  - Implement proxy endpoints to Audio Player:
    - `/api/play`, `/api/pause`, `/api/skip`, `/api/seek`
    - `/api/volume`, `/api/output`
    - `/api/status` - Aggregate current state
  - Add user authentication to all endpoints (UUID from localStorage)
  - Handle Audio Player unavailability gracefully

- **3.4. Queue Management UI:**
  - Implement proxy endpoints:
    - `/api/queue` - Get queue contents
    - `/api/enqueue` - Add passage
    - `/api/remove` - Remove passage
  - Handle concurrent queue operations (multi-user coordination)

- **3.5. Web UI - Now Playing:**
  - Create responsive UI with desktop/mobile support
  - Display current passage: title, artist, album, album art
  - Playback controls: play/pause, skip, volume slider, seek bar
  - Real-time updates via SSE (`GET /api/events`)
  - Display queue with drag-to-reorder (future phase)

- **3.6. Library Browsing:**
  - Database queries for passages, songs, artists, albums
  - Implement search/filter functionality
  - Manual passage selection and enqueue
  - Album art display with priority handling

- **3.7. Likes/Dislikes (Full & Lite):**
  - Implement UI controls (thumbs up/down)
  - API endpoints:
    - `POST /api/like` - Record like for current passage
    - `POST /api/dislike` - Record dislike
  - Associate feedback with authenticated user UUID
  - Weight distribution across songs (per `like_dislike.md`)

- **3.8. Lyrics Display & Integration (Full only):**
  - API endpoints:
    - `GET /api/lyrics/:song_guid` - Retrieve lyrics with fallback chain
      - Read-only proxy to database (no editing in wkmp-ui)
      - Implements lyrics fallback logic:
        1. Check current Song's `lyrics` field
        2. If empty, iterate through `related_songs` array in order
        3. Return lyrics from first Song with non-empty `lyrics` field
        4. Return null if no lyrics found in chain
    - `POST /api/lyrics/edit/:song_guid` - Launch wkmp-le for editing (Full only)
      - Check if wkmp-le is running, launch if needed
      - Forward song GUID, recording MBID, title, and artist to wkmp-le via `POST /lyric_editor/open`
      - Return success/failure status
  - UI for viewing and editing lyrics:
    - Display current lyrics for currently playing song/passage (read-only)
    - Lyrics are retrieved via fallback chain through related songs
    - "Edit Lyrics" button launches wkmp-le via `/api/lyrics/edit/:song_guid` (Full only)
    - Listen for `LyricsChanged` SSE events to refresh display when lyrics are saved
  - No direct editing in wkmp-ui (all editing via wkmp-le in Full version)

---

## Phase 4: Program Director Module

*Goal: Build the automatic passage selection engine as an independent HTTP server.*

- **4.1. Module Scaffolding:**
  - Implement `wkmp-pd` binary in workspace (already created in Phase 1.0)
  - Implement HTTP server with module discovery:
    1. Read root folder path from config file or environment variable
    2. Open database file (`wkmp.db` in root folder)
    3. Initialize required database tables using shared common library functions:
       - Call `init_module_config_table()` - Creates `module_config` with all defaults
       - Call `init_settings_table()` - Creates `settings` table
       - Create Program Director-specific tables: `timeslots`, `timeslot_passages`
       - All initialization is idempotent (safe to call multiple times)
    4. Read module configuration using shared config loader from Phase 1.2:
       - Loader calls `init_module_config_table()` if table missing
       - If own config missing, inserts default and logs warning
       - If Audio Player config missing, inserts default and logs warning
    5. Retrieve own host/port configuration (program_director entry, default: 127.0.0.1:5722)
    6. Bind to configured address/port
    7. Retrieve Audio Player URL for automatic enqueueing
  - Create minimal HTML developer UI (served via HTTP) for current timeslot display and last selection results
  - Implement SSE endpoint for selection events

- **4.2. Flavor Distance Calculation:**
  - Implement in `wkmp-common/src/flavor/distance.rs` (shared with other modules)
  - Squared Euclidean distance formula for flavor vectors
  - Calculate distances from target flavor for all passages
  - Sort by distance, take top 100 candidates
  - Handle passages with missing flavor data

- **4.3. Cooldown System:**
  - Implement in `wkmp-common/src/cooldown/calculator.rs` (shared logic)
  - Min/ramping cooldown logic for:
    - Songs (default: 7 days min, 14 days ramping)
    - Artists (default: 2 hours min, 4 hours ramping)
    - Works (default: 3 days min, 7 days ramping)
  - Calculate cooldown multiplier based on time since last play
  - Product of all entity cooldowns for final multiplier

- **4.4. Base Probability System:**
  - Load base probabilities from database (songs, artists, works)
  - Calculate passage final base probability (product of all entities)
  - Filter out zero-probability passages

- **4.5. Selection Algorithm:**
  - Implement weighted random selection:
    - Final weight = (1 / distance²) × cooldown_multiplier × base_probability
    - Normalize weights to sum to 1.0
    - Random selection from weighted distribution
  - Handle edge cases (no candidates → stop enqueueing)
  - Log selection decisions for debugging

- **4.6. Queue Refill Request Handler:**
  - Implement `POST /selection/request` endpoint to receive queue refill requests from Audio Player
  - Request includes anticipated start time for the new passage
  - Immediately acknowledge request to prevent Audio Player from relaunching:
    - Read acknowledgment timeout from settings table (`queue_refill_acknowledgment_timeout_seconds`)
    - Default: Must respond within 5 seconds
    - Audio Player uses this same setting to determine when to relaunch
  - Trigger passage selection asynchronously (may take longer than threshold interval)
  - Enqueue selected passage via Audio Player HTTP API
  - Handle edge cases:
    - No candidates available → return error status
    - Selection already in progress → acknowledge but don't duplicate work
  - Note: Audio Player initiates requests when queue drops below threshold (see Phase 2.7)

---

## Phase 5: Timeslot & Flavor Configuration

*Goal: Complete the time-of-day flavor system with UI configuration.*

- **5.1. Timeslot Management:**
  - User Interface proxies to Program Director:
    - `GET /config/timeslots` - Retrieve 24-hour schedule
    - `POST /config/timeslots` - Update timeslot configuration
  - Program Director implements:
    - Calculate active timeslot based on current time
    - Determine target flavor from timeslot passages (weighted centroid)
    - Emit `TimeslotChanged` events
  - UI for creating/editing timeslots with start times and names

- **5.2. Timeslot Passage Assignment:**
  - UI for assigning passages to timeslots
  - Calculate net timeslot flavor as weighted centroid
  - Visualize target flavor profile
  - Preview how passages match timeslot

- **5.3. Temporary Flavor Override:**
  - Program Director API:
    - `POST /selection/override` - Activate temporary override
    - `DELETE /selection/override` - Clear override
  - Implement override expiration
  - Emit `TemporaryFlavorOverride` and `OverrideExpired` events
  - Optional queue flush on override activation

---

## Phase 6: Audio Ingest Module (Full Version Only)

*Goal: Build the guided workflow for adding new audio files.*

- **6.1. Module Scaffolding:**
  - Implement `wkmp-ai` binary in workspace (already created in Phase 1.0)
  - Note: Audio Ingest is only included in Full version packaging (no conditional compilation needed)
  - Implement HTTP server with module discovery:
    1. Read root folder path from config file or environment variable
    2. Open database file (`wkmp.db` in root folder)
    3. Initialize required database tables using shared common library functions:
       - Call `init_module_config_table()` - Creates `module_config` with all defaults
       - Create Audio Ingest-specific tables: `files`, `passages`, `songs`, `artists`, `works`, `albums`, `images`
       - Create relationship tables: `passage_songs`, `song_artists`, `passage_albums`, `song_works`
       - Create cache tables: `acoustid_cache`, `musicbrainz_cache`, `acousticbrainz_cache`
       - All initialization is idempotent (safe to call multiple times)
    4. Read module configuration using shared config loader from Phase 1.2:
       - Loader calls `init_module_config_table()` if table missing
       - If own config missing, inserts default and logs warning
    5. Retrieve own host/port configuration (audio_ingest entry, default: 127.0.0.1:5723)
    6. Bind to configured address/port
    7. No other module dependencies (Audio Ingest is standalone)
  - Create guided workflow UI

- **6.2. File Scanner & Metadata:**
  - API endpoints:
    - `POST /ingest/scan` - Scan directory for audio files
    - `GET /ingest/pending` - List files pending ingest
  - Recursive directory scanning with change detection (SHA-256 hashes)
  - Parse basic metadata (ID3v2, Vorbis Comments, MP4 tags)
  - Store files in database with pending status

- **6.3. MusicBrainz Integration:**
  - Build API client for MusicBrainz queries
  - Implement AcoustID fingerprinting (via Chromaprint)
  - API endpoint: `POST /ingest/identify/{file_id}`
  - Match files to recordings, releases, artists, works
  - Cache all responses in `musicbrainz_cache` table
  - Handle multiple match candidates
  - Populate `related_songs` field:
    - Query MusicBrainz for other recordings of the same Work
    - Order by relationship closeness (same artist first, then other artists)
    - Store as JSON array in `songs.related_songs` field
    - API endpoint: `PUT /ingest/related_songs/{song_guid}` - User edit related songs list

- **6.4. AcousticBrainz Integration:**
  - Build API client for AcousticBrainz
  - API endpoint: `POST /ingest/characterize/{file_id}`
  - Fetch high-level musical flavor data
  - Cache in `acousticbrainz_cache` table
  - Add TODO comment for fallback to Essentia (implemented in Phase 6.5)
  - For now: If AcousticBrainz data unavailable, leave flavor vector empty (will be filled by Essentia in Phase 6.5)

- **6.5. Essentia Integration:**
  - **TODO:** Detailed Essentia integration requirements including version, build process, platform requirements, and conditional compilation strategy will be specified prior to wkmp-ai module implementation
  - **Very significant task** - Set up Rust FFI bindings for Essentia C++
  - Implement local analysis pipeline
  - Generate musical flavor vectors locally when AcousticBrainz data unavailable
  - Store in `musical_flavor_vector` JSON field
  - Complete fallback mechanism from Phase 6.4: Check for AcousticBrainz data first, use Essentia only if missing
  - Note: Part of wkmp-ai binary (Full version only)

- **6.6. Passage Segmentation:**
  - API endpoint: `POST /ingest/segment/{file_id}`
  - UI workflow for defining passage boundaries:
    - Source selection (MusicBrainz release track matching)
    - Silence detection for automatic suggestions
    - Manual boundary adjustment with waveform display
  - Set lead-in/lead-out points, fade curves
  - Support multiple passages per file

- **6.7. Metadata Editing & Finalization:**
  - API endpoints:
    - `PUT /ingest/metadata/{passage_id}` - Edit passage metadata
    - `POST /ingest/finalize/{file_id}` - Complete ingest workflow
  - Manual metadata correction UI
  - Commit finalized passages to library

---

## Phase 7: Multi-Song Passages & Advanced Features

*Goal: Support complex passage structures and album art handling.*

- **7.1. Multi-Song Passage Support:**
  - Implement passage-to-song many-to-many relationships
  - Implement weighted centroid in `wkmp-common/src/flavor/centroid.rs`:
    - Calculate passage flavor as weighted centroid of constituent songs
    - Songs weighted by duration within passage (not total song duration)
    - For multi-song passages: weight = (song's time span in passage) / (total passage duration)
    - For single-song passages: weight = 1.0
    - See [musical_flavor.md](musical_flavor.md) for complete algorithm details
  - Handle `CurrentSongChanged` events during playback
  - Display current song within passage in UI

- **7.2. Album Art Handling:**
  - Extract embedded cover art from audio files
  - Store in filesystem alongside audio files:
    - Storage location: Same folder as the audio file the artwork relates to
    - Naming convention: Same filename as source file art is extracted from
    - Conflict resolution: Append ISO8601 timestamp before extension (e.g., `cover_2025-10-09T12:34:56Z.jpg`)
    - For artwork related to multiple audio files in different folders, store with first related audio file (rare)
  - Store relative file paths in `images` table (relative to root folder) (see [Database Schema](database_schema.md#images))
  - Support multiple image types (front, back, liner, artist, work)
  - Priority-based selection for display
  - UI displays art based on currently playing song
  - Rotate multiple albums every 15 seconds
  - See [Library Management](library_management.md) for complete artwork extraction workflows

- **7.3. Multi-Album Passage Support:**
  - Handle passages spanning multiple albums (compilations)
  - Album art switches based on current song
  - UI logic per `ui_specification.md`

- **7.4. Works Support:**
  - Implement song-to-work many-to-many relationships
  - Handle mashups (multiple works per song)
  - Work cooldown multiplier = product of all associated works
  - All works must be out of cooldown for song to be selectable

---

## Phase 8: Configuration UI & Base Probabilities

*Goal: Complete the configuration interfaces for controlling selection behavior.*

- **8.1. Base Probability Editing:**
  - User Interface proxies to Program Director:
    - `GET /config/probabilities` - Get base probabilities
    - `PUT /config/probabilities/{entity_type}/{id}` - Set base probability
  - UI for adjusting probabilities (0.0-1000.0) for:
    - Individual songs
    - Individual artists
    - Individual works
  - Visualize impact on selection likelihood

- **8.2. Cooldown Configuration:**
  - User Interface proxies to Program Director:
    - `GET /config/cooldowns` - Get cooldown settings
    - `PUT /config/cooldowns` - Update cooldown settings
  - UI for editing min/ramping periods:
    - Per-song custom cooldowns
    - Per-artist custom cooldowns
    - Per-work custom cooldowns
    - Global defaults

- **8.3. Program Director Status:**
  - User Interface display of:
    - Current timeslot and target flavor
    - Last selection candidates (debugging)
    - Selection success/failure events
  - SSE events from Program Director forwarded to UI

---

## Phase 9: Version Packaging & Module Integration

*Goal: Create version-specific distribution packages and ensure modules work together.*

- **9.1. Version Packaging Strategy:**
  - **Key principle**: Versions are differentiated by **which modules are deployed**, not by conditional compilation
  - Each module is built identically; no feature flags or conditional compilation required
  - Create packaging scripts for each version:
    - **Full version**: Package all 5 binaries
      - `wkmp-ap` (Audio Player)
      - `wkmp-ui` (User Interface)
      - `wkmp-le` (Lyric Editor - launched on-demand)
      - `wkmp-pd` (Program Director)
      - `wkmp-ai` (Audio Ingest)
    - **Lite version**: Package 3 binaries
      - `wkmp-ap` (Audio Player)
      - `wkmp-ui` (User Interface)
      - `wkmp-pd` (Program Director)
    - **Minimal version**: Package 2 binaries
      - `wkmp-ap` (Audio Player)
      - `wkmp-ui` (User Interface)
  - Document deployment configurations in `deployment.md`

- **9.2. Build Scripts & Distribution:**
  - Create unified build script: `cargo build --release` (builds all binaries)
  - Create distribution packaging scripts:
    - `package-full.sh` - Packages 5 binaries + dependencies
    - `package-lite.sh` - Packages 3 binaries + dependencies
    - `package-minimal.sh` - Packages 2 binaries + dependencies
  - Bundle runtime dependencies:
    - **Audio libraries** (with wkmp-ap only): System audio libraries (PulseAudio/ALSA/CoreAudio/WASAPI)
    - **SQLite** (with all modules): Use `rusqlite` with `bundled` feature (includes JSON1 extension)
    - Note: symphonia, rubato, and cpal are compiled into the wkmp-ap binary (no external runtime dependencies except system audio)
  - Create installer packages (.deb, .rpm, .dmg, .msi) per version
  - Include default database with pre-populated `module_config` table

- **9.3. Module Integration Testing:**
  - Test all HTTP API communication between modules
  - Verify module discovery via `module_config` table
  - Test graceful degradation when modules unavailable
  - Test SSE event flow from Audio Player → User Interface
  - Test Program Director → Audio Player enqueueing

- **9.4. Database Export/Import:**
  - Simple SQLite database file copy workflow
  - Full version: Copy `wkmp.db` file to share with Lite/Minimal installations
  - Lite/Minimal versions: Copy received `wkmp.db` file to their root folder
  - UUID-based primary keys enable seamless merging (no ID conflicts)
  - Document workflow for users:
    - Full version users: Locate and copy `wkmp.db` from root folder
    - Lite/Minimal users: Place received `wkmp.db` in their root folder
  - Note: Simple file copy operation justifies 0.5 week estimate for implementation and documentation

---

## Phase 10: Platform Support & Deployment

*Goal: Ensure cross-platform operation and proper system integration.*

- **10.1. Linux Deployment:**
  - Create systemd service file for User Interface (see `deployment.md`):
    - `wkmp-ui.service` - Only service file needed
    - wkmp-ui launches other modules (wkmp-ap, wkmp-pd, wkmp-ai, wkmp-le) automatically as needed
    - Other modules can also launch wkmp-ui if they detect it's not running
  - Configure auto-start on boot (enable wkmp-ui.service)
  - Test startup/shutdown sequences
  - Test module launching and relaunching logic
  - Create installation scripts (.deb, .rpm)

- **10.2. macOS Deployment:**
  - Create launchd plist file for User Interface:
    - `com.wkmp.ui.plist` - Only plist needed
    - wkmp-ui launches other modules automatically as needed
  - Configure auto-start on login
  - Test with macOS audio system (CoreAudio)
  - Test module launching and relaunching logic
  - Create .dmg installer package
  - Code signing for Gatekeeper

- **10.3. Windows Deployment:**
  - Create Windows Service wrapper for User Interface (using NSSM or native API):
    - `WKMP-UI` service - Only service needed
    - wkmp-ui launches other modules automatically as needed
  - Configure auto-start on boot
  - Test with Windows audio (WASAPI)
  - Test module launching and relaunching logic
  - Create .msi installer package

---

## Phase 11: Distributed Deployment Support

*Goal: Enable modules to run on separate hosts.*

- **11.1. Network Configuration:**
  - Document updating `module_config` table for distributed setups
  - Test modules communicating across network
  - Security considerations for exposed HTTP ports
  - Firewall configuration guidance

- **11.2. Multi-Host Testing:**
  - Test with Audio Player on one machine, User Interface on another
  - Verify SSE works across network
  - Test reconnection logic when modules restart
  - Document deployment topologies

---

## Phase 12: Polish, Optimization & Testing

*Goal: Harden the application, improve performance, and ensure quality.*

- **12.1. Raspberry Pi Zero2W Optimization:**
  - Profile memory and CPU usage on target device (Lite/Minimal versions)
  - Optimize audio buffer management for low-power devices (reduce pre-buffered passages if needed)
  - Optimize database queries (indexes, query plans)
  - Reduce binary sizes
  - Target: < 256MB memory, < 5s startup time

- **12.2. Error Handling & Recovery:**
  - Comprehensive error handling in all modules
  - Graceful degradation when dependencies unavailable
  - Retry logic for transient failures (network, database locks)
  - User-friendly error messages in UI
  - Logging strategy (file rotation, log levels)

- **12.3. Multi-User Edge Cases:**
  - Test and harden skip throttling (< 5 seconds)
  - Test concurrent queue removal
  - Test concurrent lyric editing
  - Verify user identity isolation
  - Test Anonymous user shared state

- **12.4. UI/UX Refinements:**
  - Loading states for asynchronous operations
  - Responsive design polish (mobile/desktop)
  - Accessibility (keyboard navigation, screen readers)
  - Visual polish (transitions, animations)
  - Error state displays

- **12.5. Comprehensive Testing:**
  - **Test Framework**: Rust `cargo test` with standard test harness
  - **Test Coverage Targets** (per CO-093, measured with `cargo-tarpaulin` or `cargo-llvm-cov`):
    - **wkmp-ap (Audio Player)**: 80% coverage
      - Critical: Playback engine, queue management, crossfade logic, single-stream mixing coordination
      - Important: Volume control, audio device selection, play history recording, PCM buffer management
      - Excluded: Minimal HTML developer UI
    - **wkmp-pd (Program Director)**: 80% coverage
      - Critical: Selection algorithm, cooldown calculations, flavor distance calculations, weighted random selection
      - Important: Timeslot management, request handling, probability calculations
      - Excluded: Minimal HTML developer UI
    - **wkmp-ui (User Interface)**: 70% coverage
      - Less critical (mostly proxy logic and orchestration)
      - Important: Authentication, session management, module launching logic
      - Excluded: Static web UI assets (HTML/CSS/JavaScript files)
    - **wkmp-ai (Audio Ingest)**: 70% coverage (Full version only)
      - Complex workflows but fewer edge cases than playback/selection
      - Important: File scanning, MusicBrainz/AcousticBrainz integration, passage segmentation
      - Excluded: Web UI assets
    - **wkmp-le (Lyric Editor)**: 60% coverage (Full version only)
      - Simple CRUD operations, lower complexity
      - Important: Database read/write, HTTP API endpoints
      - Excluded: Embedded browser component, UI framework
    - **wkmp-common (Shared Library)**: 80% coverage
      - Critical shared logic used by all modules
      - Important: Database initialization, config loader, flavor calculations, event types
  - **Unit Tests**:
    - Core algorithms (distance, cooldown, selection)
    - Weighted centroid calculations
    - Queue management logic
    - Configuration loader with error handling
  - **Integration Tests**:
    - HTTP API endpoints for all modules
    - Module-to-module communication
    - Database operations and migrations
    - SSE event stream delivery
  - **End-to-End Tests**:
    - Complete user workflows (playback, queue management, library ingest)
    - Multi-module coordination scenarios
    - Error recovery and graceful degradation
  - **Performance Benchmarks**:
    - Selection algorithm performance with large libraries
    - Database query optimization
    - Concurrent request handling
  - **Cross-Platform Testing**:
    - Linux, macOS, Windows compatibility
    - Different audio sink configurations
  - **Load Testing**:
    - Concurrent users (multiple UI connections)
    - Large library sizes (10k+ songs)
    - Sustained playback over extended periods
  - **CI/CD Enhancement** (builds on Phase 1.0 basic build pipeline):
    - Add automated test execution on every commit (extends existing build-only pipeline)
    - Add coverage reporting and enforcement (80% threshold for critical modules)
    - Add platform-specific test runners (Linux, macOS, Windows)
    - Add test result reporting in pull requests
    - Add quality gates (tests must pass before merge)
    - Add benchmark tracking over time
    - Integration with code review process
    - Note: Phase 1.0 provided basic build automation; this phase adds comprehensive testing automation

---

## Phase 13: Documentation & Developer Tools

*Goal: Complete documentation and create developer-friendly tools.*

- **13.1. User Documentation:**
  - Installation guides for all platforms
  - Configuration guide (module_config, settings)
  - User manual for core features
  - Troubleshooting guide

- **13.2. API Documentation:**
  - OpenAPI/Swagger specs for all module APIs
  - API client examples (curl, JavaScript)
  - SSE event documentation

- **13.3. Developer Documentation:**
  - Architecture overview
  - Module communication patterns
  - Database schema documentation
  - Build and development setup

- **13.4. Developer Tools:**
  - CLI tool for database inspection
  - CLI tool for testing module communication
  - Mock servers for development/testing
  - Debugging utilities

---

## Future Phases

The following features are specified but not yet estimated:

- **Musical Taste System:**
  - Use likes/dislikes to influence selection
  - User-specific taste profiles
  - Taste profile visualization

- **Advanced Queue Management:**
  - Drag-to-reorder in UI
  - Save/load queue presets
  - Queue sharing between users

- **Mobile App (Flutter Rewrite):**
  - Shared Rust core via FFI
  - Platform-specific audio engines
  - Background playback
  - Offline-first architecture

----
End of document - WKMP Implementation Order & Timeline

**Document Version:** 1.1
**Last Updated:** 2025-10-17

**Change Log:**
- v1.1 (2025-10-17): Added request deduplication strategy for Audio Player / Program Director communication
  - Added "Request Deduplication Strategy" section in Phase 2.7
  - Specified UUID-based idempotency approach for queue refill requests
  - Defined responsibilities for both Audio Player (wkmp-ap) and Program Director (wkmp-pd)
  - Added benefits and implementation notes for production robustness
  - Supports architectural decision from wkmp-ap design review (ISSUE-10)
