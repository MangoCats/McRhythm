# WKMP Architecture

**🏗️ TIER 2 - DESIGN SPECIFICATION**

Defines HOW the system is structured. Derived from [requirements.md](requirements.md). See [Document Hierarchy](document_hierarchy.md).

> **Related Documentation:** [Database Schema](database_schema.md) | [Crossfade Design](crossfade.md) | [Musical Flavor](musical_flavor.md)| [Event System](event_system.md)

---

## Overview

WKMP is a music player built on Rust, GStreamer, and SQLite that automatically selects music passages based on user-configured musical flavor preferences by time of day, using cooldown-based probability calculations and AcousticBrainz musical characterization data.

WKMP implements a **microservices architecture** with multiple independent processes communicating via HTTP APIs and Server-Sent Events (SSE). This enables simplified maintenance, version flexibility, and independent module updates.

## Process Architecture

WKMP consists of up to 5 independent processes (depending on version), each with defined HTTP/SSE interfaces:

- **Audio Player** - Core playback engine with queue management
- **User Interface** - Polished web UI for end users
- **Lyric Editor** - Standalone lyric editing tool (launched on-demand)
- **Program Director** - Automatic passage selection
- **Audio Ingest** - New file import workflow (Full version only)

**Design Benefits:**
- **Simplifies maintenance**: Each module focuses on a single concern
- **Enables version flexibility**: Run more/fewer processes for Full/Lite/Minimal versions
- **Provides modularity**: Update one module without affecting others
- **Supports independent operation**: Audio Player and Program Director work without UI

### Process Communication Model

```
┌─────────────────────────────────────────────────────────────┐
│  User Interface (HTTP + SSE Server)                         │
│  Port: 5720 (configurable)                                  │
│  - Polished web UI for end users                            │
│  - Authentication, playback control, queue management       │
│  - Album art, lyrics display, likes/dislikes, config        │
└───────────┬─────────────────────────────────────────────────┘
            │ HTTP API calls
            │ SSE subscriptions
    ┌───────┼────────┬────────────────────────┬───────────────┐
    │       │        │                        │               │
    ▼       ▼        ▼                        ▼               ▼
┌───────┐ ┌────────────────┐  ┌──────────────────────────────┐ ┌─────────────┐
│Audio  │ │  Audio Player  │  │  Program Director            │ │Lyric Editor │
│Ingest │ │  Port: 5721    │  │  Port: 5722                  │ │  Port: 5724 │
│  UI   │ │                │◄─┤                              │ │             │
│       │ │  - Minimal     │  │  - Minimal dev UI            │ │  - Split UI │
│(Full  │ │    dev UI      │  │  - Selection API (for UI)    │ │  - Editor + │
│ only) │ │  - Control API │  │  - Reads Audio Player status │ │    Browser  │
│       │ │  - Status API  │  │  - Enqueues via Audio Player │ │  - On-demand│
│Port:  │ │  - SSE events  │  │                              │ │    launch   │
│ 5723  │ │                │  │  SQLite Database (Shared)    │ │             │
│       │ │                │  │  - Files, Passages, Songs    │ │             │
│       │ │                │  │  - Play History, Queue       │ │             │
└───────┘ └────────────────┘  └──────────────────────────────┘ └─────────────┘
            │                   │
            │ Direct HTTP API   │
            └───────────────────┘
               (No UI required)
```

### Version-Specific Process Configuration

| Version  | Audio Player | User Interface | Lyric Editor | Program Director | Audio Ingest |
|----------|--------------|----------------|--------------|------------------|--------------|
| **Full**     | ✅ Running | ✅ Running (Full-featured) | ✅ On-demand | ✅ Running | ✅ Running |
| **Lite**     | ✅ Running | ✅ Running (De-featured)   | ❌ Not included | ✅ Running | ❌ Not included |
| **Minimal**  | ✅ Running | ✅ Running (De-featured)   | ❌ Not included | ❌ Not included | ❌ Not included |

## Module Specifications

### Audio Player

**Process Type**: Independent HTTP server with minimal HTML developer UI (served via HTTP)
**Port**: 5721 (configurable)
**Versions**: Full, Lite, Minimal

**Responsibilities:**
- Manages dual GStreamer pipelines for seamless crossfading
- Coordinates passage transitions based on lead-in/lead-out timing
- Implements three fade profiles (exponential, cosine, linear)
- Handles pause (immediate stop) and resume (configurable fade-in, default: 0.5s exponential)
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

**Developer UI** (Minimal HTML/JavaScript served via HTTP):
- Module status display
- Direct API testing interface
- Event stream monitor

**State:**
- Currently playing passage (position, duration, state)
- Next passage (pre-loaded, ready for crossfade)
- Queue contents (persisted to SQLite)
- User volume level (0-100)
- Playback state (Playing/Paused only - no "stopped" state)
- Initial state on app launch: Determined by `initial_play_state` setting (default: "playing")

**Key Design Notes:**
- **Operates independently**: Does not require User Interface to be running
- **Receives commands from**: User Interface, Program Director
- **Database access**: Direct SQLite access for queue persistence, passage metadata

### Queue and State Persistence

**[ARCH-QUEUE-PERSIST-010]** Queue Persistence Strategy:
- Queue contents written to SQLite immediately on every queue modification (enqueue/dequeue/reorder)
- Each queue entry stored with passage reference and timing specifications
- Queue changes are synchronous writes (blocking until persisted)
- Single database design (queue stored with library data)

**[ARCH-QUEUE-PERSIST-020]** Playback Position Persistence:
- Playback position transmitted via SSE at configurable interval (setting: `playback_progress_interval_ms`, default 5000ms)
- Also transmitted once when Pause initiated, once when Play initiated
- Playback position persisted **only on clean shutdown**
- On any queue change, `last_played_position` automatically reset to 0 in settings
- On startup: if `last_played_position` > 0, resume from that position; otherwise start from beginning
- No special crash detection needed - queue change reset handles both crash and normal operation

**[ARCH-QUEUE-PERSIST-030]** Database Backup Strategy (wkmp-ui responsibility):

**On Startup:**
1. Run `PRAGMA integrity_check` on wkmp.db
2. If integrity good:
   - Check time since last automatic backup
   - If ≥ `backup_minimum_interval_ms` (default: 2 weeks): Create backup
   - If < threshold: Skip backup (prevents excessive wear on frequent restarts)
3. If integrity bad:
   - Archive corrupted database with timestamp
   - Restore from most recent backup
   - Repeat integrity check on restored database
   - Continue until good database found or all backups exhausted
4. Display minimal UI showing backup/verification progress to connecting users

**Backup Process (Atomic):**
1. Copy wkmp.db → wkmp_backup_temp.db
2. Run `PRAGMA integrity_check` on temp
3. If good: Atomic rename → wkmp_backup_YYYY-MM-DD.db (timestamped if keeping multiple)
4. If bad: Delete temp, log error
5. Maintain `backup_retention_count` backups (default: 3), delete oldest when exceeded

**Periodic Backup:**
- Interval: `backup_interval_ms` (default: 3 months / ~7,776,000,000ms)
- Triggered by wkmp-ui background timer
- Same atomic process as startup backup

**Backup Configuration (settings table):**
- `backup_location`: Path to backup directory (default: same folder as wkmp.db)
- `backup_interval_ms`: Time between periodic backups (default: 90 days)
- `backup_minimum_interval_ms`: Minimum time between startup backups (default: 14 days)
- `backup_retention_count`: Number of timestamped backups to keep (default: 3)
- `last_backup_timestamp_ms`: Unix milliseconds of last successful backup

**Backup Failure Handling:**
- Network backup location unreachable: Fall back to local backup path, log warning
- Timeout: 30 seconds for network writes
- Startup never blocked by backup failure (only by integrity check and restore if needed)

### Initial Play State

**[ARCH-STARTUP-005]** Initial Play State Configuration:
- Setting: `initial_play_state` (string: "playing" or "paused", default: "playing")
- Determines playback state on app launch
- Current playback state is never persisted across restarts

**[ARCH-STARTUP-010]** Cold Start Procedure:
1. Run database integrity check and backup (if wkmp-ui; see ARCH-QUEUE-PERSIST-030)
2. Initialize audio device (see ARCH-AUDIO-DEVICE-010 below)
3. Read `initial_play_state` from settings (default: "playing")
4. Set playback state according to setting
5. Read queue from database (ORDER BY play_order)
6. Read `last_played_passage_id` and `last_played_position` from settings
7. Determine action:
   - **Queue empty + Playing**: Wait in Playing state (plays immediately when passage enqueued)
     - User-facing state: "playing"
     - Internal GStreamer state: Use whichever GStreamer state most naturally provides silence and smooth pop-free transition to playing when passage becomes available (NULL or PAUSED)
     - See [gstreamer_design.md - Empty Queue Behavior](gstreamer_design.md#44-empty-queue-behavior) for implementation details
   - **Queue empty + Paused**: Wait silently
   - **Queue has passages + Playing**: Begin playback
   - **Queue has passages + Paused**: Load first passage but don't play
8. Starting position:
   - If `last_played_passage_id` matches first queue entry AND `last_played_position` > 0: Resume from position
   - Otherwise: Start from passage `start_time_ms`

**[ARCH-AUDIO-DEVICE-010]** Audio Device Initialization:

On module startup, wkmp-ap must initialize an audio output device before playback can begin.

1. **Read persisted device setting:**
   - Query `settings` table for `audio_sink` value
   - If value is NULL or empty string: Proceed to step 2
   - If value exists: Proceed to step 3

2. **First-time startup (no persisted device):**
   - Use system default device (GStreamer `autoaudiosink`)
   - Query GStreamer to determine which actual device was selected
   - Write selected device_id to `settings.audio_sink` for future startups
   - Log: "Audio device initialized: [device_name] (system default)"

3. **Subsequent startup (device persisted):**
   - Query available audio devices from GStreamer
   - Check if persisted `audio_sink` device_id exists in available devices list
   - **If device found**: Use persisted device, log: "Audio device restored: [device_name]"
   - **If device NOT found** (unplugged USB, disconnected Bluetooth, etc.):
     - Log warning: "Persisted audio device '[device_id]' not found, falling back to system default"
     - Use system default device (GStreamer `autoaudiosink`)
     - Update `settings.audio_sink` to `"default"` to record fallback
     - Continue startup normally

4. **Device initialization failure:**
   - If system default device also fails to initialize:
     - Log error: "Failed to initialize any audio output device"
     - Set module health status to "unhealthy"
     - `GET /health` returns `503 Service Unavailable` with `audio_device: "failed"`
     - Module continues running but playback cannot start
     - User can attempt to set different device via `POST /audio/device`

5. **Special device_id values:**
   - `"default"`: Always uses GStreamer `autoaudiosink` (system default selection)
   - Specific device_id (e.g., `"pulse-sink-1"`): Uses exact GStreamer sink

**[ARCH-STARTUP-020]** Queue Entry Validation:
- Validated lazily when scheduled for playback
- If file missing when playback attempted:
  1. Log error with passage ID and file path
  2. Emit `PassageCompleted(completed=false)` event
  3. Remove invalid entry from queue
  4. Advance to next passage
  5. Continue if in Playing state

**[ARCH-STARTUP-025]** Queue Lifecycle:
- Queue is forward-looking only (passages waiting to play)
- Currently playing passage tracked via `currently_playing_passage_id` setting
- Completed passages removed from queue immediately (FIFO)
- Play history stored separately in `song_play_history` table (single table for all songs)

**[ARCH-QUEUE-ORDER-010]** Play Order Management:
- New passages appended with `play_order = last_play_order + 10`
- Gaps enable insertion without renumbering (e.g., insert at 25 between 20 and 30)
- When inserting and no gap available (e.g., 20, 21), renumber tail: `UPDATE queue SET play_order = play_order + 10 WHERE play_order >= 20`
- Typical queue depth: 5-10 passages (graceful degradation up to 1000+, but performance not priority concern beyond that)

**Play Order Overflow Protection:**
- `play_order` stored as signed 32-bit integer (max: 2,147,483,647)
- At typical usage (3 min/passage, +10 increment): 1,225 years until overflow
- If `play_order` exceeds 2,000,000,000: Trigger automatic renumbering
  - Renumber entire queue starting from 10 (10, 20, 30...)
  - Happens transparently during enqueue operation
  - Extremely rare (abuse/hack scenario only)

### Song Boundary Detection (CurrentSongChanged Event)

**[ARCH-SONG-CHANGE-010]** Passage vs Song Relationship:

A **passage** is a continuous subset of an audio file played from its `start_time_ms` to `end_time_ms`. A passage plays continuously without any transitions except at its start and end points.

Key characteristics:
- Passages are continuous playback regions within audio files
- Multiple passages can be defined within a single audio file
- Passages may overlap or have gaps between them
- The same audio region can play in both lead-out of one passage and lead-in of next passage
- Each passage contains zero or more **songs** (defined in `passage_songs` table)

**[ARCH-SONG-CHANGE-020]** Song Timeline Construction:

The `passage_songs` table (also called a "cut list" in music production) defines which songs exist within each passage and their time boundaries.

When a passage starts playing:
1. Query `passage_songs` table for current passage: `SELECT * FROM passage_songs WHERE passage_id = ? ORDER BY start_time`
2. Build song timeline in memory: List of `{song_id, start_time_ms, end_time_ms, albums[]}`
3. Store timeline for duration of passage playback
4. Timeline remains valid until passage completes (passages play continuously, timeline doesn't change)

**[ARCH-SONG-CHANGE-030]** CurrentSongChanged Emission:

During playback, wkmp-ap monitors playback position to detect song boundary crossings:

1. **Position monitoring:** Check current position against song timeline every 500ms
   - Use same position query mechanism as `PlaybackProgress` event
   - Separate 500ms timer for song boundary detection

2. **Boundary detection:**
   - Compare `current_position_ms` to each song's `[start_time_ms, end_time_ms]` range
   - Determine if position crossed into different song since last check

3. **Event emission:** Emit `CurrentSongChanged` when:
   - Position crosses from one song to another
   - Position crosses from song to gap (no song at current position)
   - Position crosses from gap to song

4. **Event payload:**
   ```rust
   CurrentSongChanged {
       passage_id: PassageId,           // Current passage UUID
       song_id: Option<SongId>,         // Current song UUID, or None if in gap
       song_albums: Vec<AlbumId>,       // All albums for current song (empty if None)
       position_ms: u64,                // Current position in passage (milliseconds)
       timestamp: SystemTime,           // When boundary was crossed
   }
   ```

5. **Gap handling:**
   - If `current_position_ms` is not within any song's time range: `song_id = None`
   - Gaps between songs are normal (not errors)
   - UI should handle `None` gracefully (e.g., clear "now playing" song info, show passage info instead)

**[ARCH-SONG-CHANGE-040]** Implementation Notes:

- Song timeline built **only once** per passage (on `PassageStarted`)
- No periodic re-reading of `passage_songs` table during playback
- Boundary checks use simple time range comparisons (no complex state machine)
- 500ms detection interval provides smooth UI updates without excessive CPU usage
- First `CurrentSongChanged` emitted immediately on passage start (if passage begins within a song)

**[ARCH-SONG-CHANGE-041]** Song Timeline Data Structure:

The song timeline is stored as a **sorted Vec** in memory:

```rust
struct SongTimelineEntry {
    song_id: Option<Uuid>,      // None for gaps between songs
    start_time_ms: u64,          // Start time within passage
    end_time_ms: u64,            // End time within passage
    albums: Vec<Uuid>,           // Album IDs for this song (empty for gaps)
}

struct SongTimeline {
    entries: Vec<SongTimelineEntry>,  // Sorted by start_time_ms ascending
    current_index: usize,              // Index of currently playing entry (cache)
}
```

**[ARCH-SONG-CHANGE-042]** Efficient Boundary Detection Algorithm:

```rust
fn check_song_boundaries(&mut self, current_position_ms: u64) -> Option<CurrentSongChanged> {
    let timeline = &mut self.song_timeline;
    let current_entry = &timeline.entries[timeline.current_index];

    // Fast path: Position still within current entry's time range
    if current_position_ms >= current_entry.start_time_ms
       && current_position_ms < current_entry.end_time_ms {
        return None;  // No boundary crossed, no signal to emit
    }

    // Boundary crossed: Find new current entry
    // Typical passages have 1-10 songs, up to 100 in rare cases
    // Linear search is acceptable for these sizes
    for (index, entry) in timeline.entries.iter().enumerate() {
        if current_position_ms >= entry.start_time_ms
           && current_position_ms < entry.end_time_ms {
            // Found new current entry
            timeline.current_index = index;

            return Some(CurrentSongChanged {
                passage_id: self.current_passage_id,
                song_id: entry.song_id,
                song_albums: entry.albums.clone(),
                position_ms: current_position_ms,
                timestamp: SystemTime::now(),
            });
        }
    }

    // Position is in a gap (between songs or after last song)
    // Emit CurrentSongChanged with song_id=None
    Some(CurrentSongChanged {
        passage_id: self.current_passage_id,
        song_id: None,
        song_albums: vec![],
        position_ms: current_position_ms,
        timestamp: SystemTime::now(),
    })
}
```

**[ARCH-SONG-CHANGE-043]** Gap Handling:

- **Gaps between songs**: When `current_position_ms` is not within any song's time range
  - Emit `CurrentSongChanged` with `song_id = None` and empty `song_albums`
  - Song-end signal happens when leaving the previous song
  - Next song-start signal happens when entering the next song
  - Gaps are normal and expected (not errors)

**[ARCH-SONG-CHANGE-044]** Songs Cannot Overlap Within Passage:

- The `passage_songs` table enforces non-overlapping songs within a single passage
- Database constraint: No two songs in the same passage may have overlapping time ranges
- This simplifies boundary detection (no ambiguity about which song is "current")

**[ARCH-SONG-CHANGE-045]** Crossfade Song Boundary Behavior:

During passage crossfade (when both Pipeline A and B are playing simultaneously):

- **Song-start of following passage may occur before song-end of ending passage**
- Example timeline:
  ```
  Time:     0s ─────── 3s ─────── 5s ─────── 8s
  Pipeline A: [Song X playing────────X ends]
  Pipeline B:           [Song Y──starts──────playing]
  Events:               ↑                    ↑
                   Song Y Start          Song X End
  ```
- **Signal receivers must handle this ordering**: Song-start before previous song-end
- This is expected behavior, not an error condition
- UI should update album art and song info immediately on Song-start, even if previous song hasn't signaled end yet

**[ARCH-SONG-CHANGE-050]** Edge Cases:

- **Passage with no songs:** Emit `CurrentSongChanged` with `song_id=None` on passage start
- **Passage starts in gap:** Emit with `song_id=None`, then emit again when entering first song
- **Passage ends during song:** No special handling needed, `PassageCompleted` marks end of passage
- **Songs with identical boundaries:** Both songs considered "current" (implementation may choose first song or emit multiple events)
- **Seeking:** After seek, immediately check position against timeline and emit `CurrentSongChanged` if song changed

**[ARCH-SONG-CHANGE-060]** Performance Considerations:

- Song timeline stored in memory (typically <100 songs per passage, minimal memory impact)
- Boundary checks are O(n) where n = songs in passage (acceptable for typical passage sizes)
- For large passages (>1000 songs), consider binary search on sorted timeline
- Detection timer runs only during playback (paused = no checks)

### Volume Handling
<a name="volume-handling"></a>

**[ARCH-VOLUME-010]** Volume Scale Conversion:
- **User-facing** (UI, API): Integer 0-100 (percentage)
- **Backend** (storage, GStreamer): Double 0.0-1.0
- **Conversion**:
  - User → System: `system_volume = user_volume / 100.0`
  - System → User: `user_volume = ceil(system_volume * 100.0)`
- **Rationale for ceiling**: Ensures non-zero audio never displays as 0%

**Storage:**
- Database: Store as double (0.0-1.0) in `settings.volume_level`
- API: Accept/return integer (0-100)
- Events: Transmit as float (0.0-1.0) for precision in real-time streams

### User Interface

**Process Type**: Polished HTTP server with full web UI
**Port**: 5720 (configurable)
**Versions**: Full, Lite (de-featured), Minimal (de-featured)

**Responsibilities:**
- Present polished web interface for end users
- Proxy/orchestrate requests to Audio Player and Program Director
- Handle user authentication and session management
- Display album art, lyrics, and playback information
- Provide configuration interface for Program Director parameters
- Aggregate SSE events from Audio Player for UI updates

**HTTP API** (User-facing):
- Authentication endpoints: `/api/login`, `/api/create-account`, `/api/current-user`
- Playback control: `/api/playback/*` (proxied to Audio Player)
- Queue management: `/api/queue/*` (proxied to Audio Player)
- Like/Dislike: `/api/passages/{id}/like`, `/api/passages/{id}/dislike`
- Program Director config: Proxied to Program Director
- Manual passage selection: Browse library, enqueue to Audio Player
- Volume control: Proxied to Audio Player
- Audio device selection: Proxied to Audio Player

**SSE Events** (Endpoint: `GET /api/events`):
- Aggregates and forwards events from Audio Player
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

**Lyrics Display Behavior:**
- Implements fallback chain when displaying lyrics for currently playing Song:
  1. Check current Song's `lyrics` field - if non-empty, display these lyrics
  2. If empty, iterate through Song's `related_songs` array (most to least closely related)
  3. Display lyrics from first related Song with non-empty `lyrics` field
  4. If no Song in chain has lyrics, leave lyrics display area empty
- Read-only display in wkmp-ui (all editing via wkmp-le in Full version)

**Version Differences:**
- **Full**: All features enabled
- **Lite**: No file ingest, limited configuration options
- **Minimal**: No file ingest, no likes/dislikes, no advanced configuration

**Key Design Notes:**
- **Most users interact here**: Primary interface for controlling WKMP
- **Orchestration layer**: Coordinates between Audio Player and Program Director
- **Database access**: Direct SQLite access for user data, likes/dislikes, library browsing

---

### Configuration Interface Access Control

**[ARCH-CONFIG-ACCESS-010]** Each microservice module (wkmp-ap, wkmp-ui, wkmp-pd, wkmp-ai, wkmp-le) SHALL provide an access-restricted configuration interface page that enables authorized users to both view and edit configuration settings that affect the module.

**Access Control Rules:**

1. **Per-User Configuration**: Configuration interface access is individually configurable per user via the `users.config_interface_access` database column (BOOLEAN).

2. **Not Per-Module**: Configuration interface access applies uniformly to ALL microservice modules. There is no per-module access control (e.g., cannot grant access to wkmp-ui config but deny access to wkmp-ap config).

3. **UI Behavior When Access Denied**:
   - When the current user does NOT have configuration interface access (config_interface_access = 0):
     - All indications that configuration interfaces exist are removed from the UI of every microservice module
     - Configuration links, buttons, menu items, and pages are hidden
     - Direct navigation to configuration URLs returns 403 Forbidden
   - When the current user DOES have configuration interface access (config_interface_access = 1):
     - Configuration interfaces are visible and functional in all modules

4. **Default Access**:
   - **Anonymous user**: config_interface_access = 1 (enabled by default)
   - **New user accounts**: Inherit the Anonymous user's current config_interface_access value at account creation time
   - **NULL handling**: If config_interface_access is NULL when read, reset to 1 (enabled) and store

5. **Managing Access**:
   - Users with configuration interface access MAY enable or disable configuration interface access for any user, including:
     - Anonymous user
     - Other registered users
     - Themselves (WARNING: Self-lockout requires command-line password reset tool recovery)

6. **Command-Line Password Reset Tool**:
   - The password reset tool SHALL include an option to set config_interface_access = 1 (enabled) for the target user at the same time as password is reset
   - If this option is NOT selected, the user's config_interface_access remains unchanged
   - The tool MAY re-enable config_interface_access for the Anonymous user
   - The tool MUST NOT set a password for the Anonymous user (password_hash and password_salt remain empty strings)

**Rationale:**

Configuration interface access restriction is provided to prevent inexperienced or unknowledgeable users from accidentally misconfiguring settings. It is NOT intended to "secure" the system from malicious users. The design operates on the principle of **accessibility first**:

- Default-enabled for all users (including Anonymous)
- Simple recovery path via command-line tool
- No complicated logic to prevent self-lockout scenarios
- Single point of recovery for both lost passwords and configuration access lockout

**Implementation Notes:**

- Configuration interface access check occurs at HTTP request time (middleware or route handler)
- Session must include user's config_interface_access flag for efficient checking
- UI templates/components conditionally render configuration elements based on session flag
- See [Database Schema - users table](database_schema.md#users) for column definition
- See [Deployment - Password Reset Tool](deployment.md) for command-line tool specification

---

### Lyric Editor

**Process Type**: Independent HTTP server with split-window UI (launched on-demand)
**Port**: 5724 (configurable)
**Versions**: Full only

**Responsibilities:**
- Provides dedicated interface for editing song lyrics
- Displays split window: text editor (left) + embedded browser (right)
- Loads and saves lyrics associated with MusicBrainz recordings
- Facilitates finding and copying lyrics from web sources

**HTTP Control API:**
- `POST /lyric_editor/open` - Launch editor with recording MBID, title, artist
- `GET /lyric_editor/lyrics/{recording_mbid}` - Get current lyrics for recording
- `PUT /lyric_editor/lyrics/{recording_mbid}` - Save edited lyrics to database
- `POST /lyric_editor/close` - Close editor without saving

**SSE Event Stream** (`GET /events`):
- `LyricsChanged` - Emitted when lyrics are saved to database

**Database Access:**
- **Read/Write**: `songs.lyrics` - Lyrics associated with recordings (via recording_mbid)
- Uses last-write-wins concurrency model (no locking)

**User Interface:**
- **Left pane**: Multi-line text editor pre-loaded with current lyrics from database
- **Right pane**: Embedded web browser initially searching for song lyrics
- **Save button**: Writes edited lyrics to database via `PUT /lyric_editor/lyrics/{recording_mbid}`
- **Cancel/Exit button**: Closes editor without saving changes

**Key Design Notes:**
- **On-demand launching**: Started by User Interface when user requests lyric editing
- **Standalone operation**: No dependencies on other modules (except shared database)
- **Read-only in UI**: User Interface displays lyrics but all editing happens in Lyric Editor
- **Simple concurrency**: Last write wins, no conflict resolution needed

---

### Program Director

**Process Type**: Independent HTTP server with minimal HTML developer UI (served via HTTP)
**Port**: 5722 (configurable)
**Versions**: Full, Lite (Minimal does not include automatic selection)

**Responsibilities:**
- Calculate passage selection probabilities based on multiple factors
- Implement weighted random selection algorithm
- Maintain time-of-day flavor targets
- Handle timeslot transitions
- Respond to temporary flavor overrides
- Receive queue refill requests from Audio Player and enqueue passages

**HTTP API for User Interface**:
- `GET /config/timeslots` - Retrieve timeslot configuration
- `POST /config/timeslots` - Update timeslot configuration
- `GET /config/probabilities` - Get base probabilities for songs/artists/works
- `PUT /config/probabilities/{entity_type}/{id}` - Set base probability
- `GET /config/cooldowns` - Get cooldown settings
- `PUT /config/cooldowns` - Update cooldown settings
- `POST /selection/override` - Temporary flavor override
- `DELETE /selection/override` - Clear temporary override

**HTTP API for Audio Player**:
- `POST /selection/request` - Request passage selection (called by Audio Player when queue is low)

**HTTP Status API:**
- `GET /status` - Module status, current timeslot, target flavor
- `GET /selection/candidates` - Last selection candidates (debugging)

**SSE Events** (Endpoint: `GET /events`):
- `TimeslotChanged` - New timeslot became active
- `TemporaryFlavorOverride` - Temporary override activated
- `OverrideExpired` - Temporary override ended
- `SelectionFailed` - No candidates available

**Developer UI** (Minimal HTML/JavaScript served via HTTP):
- Module status display
- Current timeslot and target flavor
- Last selection results

**Automatic Queue Management:**
- Receives queue refill requests from Audio Player via `POST /selection/request`
- Audio Player sends requests when queue drops below configurable thresholds:
  - Default: < 2 passages OR < 15 minutes remaining
  - Configured in settings table: `queue_refill_threshold_passages`, `queue_refill_threshold_seconds`
- Request includes anticipated start time for the new passage
- Program Director responds immediately to acknowledge request:
  - Timeout configured in settings table: `queue_refill_acknowledgment_timeout_seconds` (default: 5 seconds)
  - If no response within timeout, Audio Player may relaunch Program Director
- Selection happens asynchronously (may take longer than throttle interval):
  1. Determine target time from request (anticipated start time)
  2. Calculate selection probabilities
  3. Select passage via weighted random algorithm
  4. Enqueue to Audio Player via HTTP API
- Audio Player throttles requests while queue is underfilled:
  - Interval configured in settings table: `queue_refill_request_throttle_seconds` (default: 10 seconds)

**Key Operations:**
- Determine target time for selection (provided in request from Audio Player)
- Filter to non-zero probability passages (passages with one or more songs only)
- Calculate squared Euclidean distance from target flavor
- Sort by distance, take top 100 candidates
- Weighted random selection from candidates
- Handle edge cases (no candidates → return error status)

**Key Design Notes:**
- **Request-based, not polling**: Audio Player initiates refill requests
- **Operates independently**: Does not require User Interface to be running
- **May enqueue proactively**: Free to enqueue passages without requests (like users via UI)
- **Database access**: Direct SQLite access for passage metadata, timeslots, probabilities, play history

> **See [Program Director](program_director.md) for complete specification of selection algorithm, cooldown system, probability calculations, and timeslot handling.**

---

### Audio Ingest

**Process Type**: Polished HTTP server with guided workflow UI
**Port**: 5723 (configurable)
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
- `PUT /ingest/related_songs/{song_guid}` - Edit related songs list
- `POST /ingest/finalize/{file_id}` - Complete ingest workflow

**Web UI Workflow:**
1. **File Discovery**: Select directories to scan for new audio files
2. **File Review**: Preview detected files, confirm additions
3. **Identification**: Match files to MusicBrainz recordings (fingerprinting)
4. **Related Songs Population**: Query MusicBrainz for other recordings of the same Work, populate `related_songs` field
5. **Characterization**: Retrieve AcousticBrainz data or run local Essentia analysis
6. **Passage Definition**: Define passage boundaries, timing points, metadata
7. **Finalization**: Review and commit to library

**Key Design Notes:**
- **Full version only**: Not included in Lite or Minimal
- **Database access**: Direct SQLite access for file/passage/song insertion
- **External API integration**: MusicBrainz, AcousticBrainz, Chromaprint
- **Local analysis**: Essentia integration for offline flavor characterization

> **See [Library Management](library_management.md) for complete file scanning and metadata workflows.**

---

### Internal Components

The modules listed above are separate processes. Within each module, there are internal components that handle specific responsibilities. These are implementation details within each module:

**Audio Player Internal Components:**
- **Queue Manager**: Maintains playback queue, handles manual additions/removals, monitors queue levels, requests refills from Program Director
- **Queue Monitor**: Calculates remaining queue time, sends `POST /selection/request` to Program Director when below thresholds (< 2 passages or < 15 minutes), throttles requests to once per 10 seconds
- **Playback Controller**: Manages dual GStreamer pipelines for crossfading, coordinates passage transitions
- **Audio Engine**: GStreamer pipeline manager with dual pipelines, audio mixer, volume control
- **Historian**: Records passage plays with timestamps, updates last-play times for cooldown calculations

**User Interface Internal Components:**
- **Authentication Handler**: User session management, Anonymous/Create/Login flows
- **API Proxy**: Forwards requests to Audio Player and Program Director
- **Event Aggregator**: Subscribes to Audio Player SSE events, forwards to web UI clients
- **Library Browser**: Database queries for passage/song/artist/album browsing

**Program Director Internal Components:**
- **Flavor Manager**: Manages 24-hour timeslot schedule, calculates flavor targets, handles temporary overrides
- **Selection Engine**: Implements weighted random selection algorithm with flavor distance calculations
- **Request Handler**: Receives queue refill requests from Audio Player, acknowledges immediately, triggers asynchronous selection

**Audio Ingest Internal Components:**
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
- ~~3. Queue Manager~~ - Now part of Audio Player
- ~~4. Historian~~ - Now part of Audio Player
- ~~5. Flavor Manager~~ - Now part of Program Director
- ~~6. Audio Engine~~ - Now part of Audio Player
- ~~7. Library Manager~~ - Now part of Audio Ingest
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
- **Module discovery**: Each module reads `module_config` table on startup to determine:
  - Its own binding address and port
  - Other modules' addresses for HTTP communication

**Consistency Considerations:**
- UUID primary keys enable database merging (Full → Lite → Minimal)
- Foreign key constraints maintain referential integrity
- Application-level coordination via HTTP APIs prevents conflicts
- Write serialization through module ownership (e.g., only Audio Player writes queue state)
- Centralized network configuration in `module_config` table eliminates config file synchronization issues

### Module Dependencies

```
User Interface
    ├── Depends on: Audio Player - optional, degrades gracefully
    ├── Depends on: Program Director - optional (Minimal version)
    └── Depends on: SQLite database - required

Program Director
    ├── Depends on: Audio Player - required for enqueueing
    └── Depends on: SQLite database - required

Audio Player
    └── Depends on: SQLite database - required

Audio Ingest (Full only)
    └── Depends on: SQLite database - required
```

**Service Launching:**
- **Only wkmp-ui has a system service file**: Users configure their OS to auto-start wkmp-ui (systemd, launchd, Windows Service)
- **wkmp-ui is the primary entry point**: Launches other modules as needed (wkmp-ap, wkmp-pd, wkmp-ai, wkmp-le)
- **Modules can launch other modules**: Any module can launch peer modules if it detects they're not running
  - Example: wkmp-ap can relaunch wkmp-pd if it's not responding to queue refill requests
  - Example: Any module can launch wkmp-ui if it needs the orchestration layer
- **Module launching process** (using shared launcher utility from wkmp-common):
  - **Binary location**:
    - **Standard deployment**: Binaries in system PATH (wkmp-ap, wkmp-pd, wkmp-ui, wkmp-ai, wkmp-le)
    - **Non-standard deployment**: Optional `--binary-path <path>` argument specifies directory
    - Argument propagates: Launching module passes `--binary-path` to launched module
  - **Command-line arguments**:
    - Modules accept optional `--binary-path` for non-standard deployments
    - No other arguments required (all configuration via database)
    - Standard deployment: No arguments needed
  - **Launch procedure**:
    - Check if module is responding via HTTP health check
    - If not responding, launch subprocess using shared launcher utility
    - Pass `--binary-path` argument if received by launching module
    - Get process handle for monitoring
  - **Relaunch throttling** (configurable via settings table):
    - `relaunch_delay` (default: 5 seconds) - Wait time between attempts
    - `relaunch_attempts` (default: 20) - Maximum attempts before giving up
    - After failure, wait `relaunch_delay` before next attempt
    - Track attempt count per module
    - After exhausting attempts, display error with user "Retry" button
    - User can reset counter and resume relaunching
  - **Logging**: All launch attempts, failures, and user retry actions logged
- **All modules self-initialize**: Each module creates its required database tables on first startup (no central database initialization)

**Root Folder Location Resolution:**

**[ARCH-INIT-005]** On module startup, the root folder path is determined using the following priority order:

1. **Command-line argument** (highest priority): `--root-folder <path>` or `--root <path>`
   - When present, this value is used as the root folder location
   - Overrides all other sources
2. **Environment variable** (second priority): `WKMP_ROOT_FOLDER` or `WKMP_ROOT`
   - When present and command-line argument is absent, this value is used
   - Enables deployment-specific configuration without code changes
3. **TOML config file** (third priority): `root_folder` key in config file
   - Default config file locations:
     - Linux: `~/.config/wkmp/config.toml` or `/etc/wkmp/config.toml`
     - macOS: `~/Library/Application Support/wkmp/config.toml`
     - Windows: `%APPDATA%\wkmp\config.toml`
   - When present and higher-priority sources are absent, this value is used
4. **OS-dependent compiled default** (lowest priority, fallback):
   - Linux: `~/.local/share/wkmp` (or `/var/lib/wkmp` for system-wide installation)
   - macOS: `~/Library/Application Support/wkmp`
   - Windows: `%LOCALAPPDATA%\wkmp`
   - Used when no other source provides a value
   - Ensures module can always start with a valid root folder path

**Module Startup Sequence:**
1. Module determines root folder path using resolution priority order above
2. Module opens database file (`wkmp.db`) in root folder
3. Module initializes its required database tables using shared initialization functions from `wkmp-common`:
   - Commonly used tables: `module_config`, `settings`, `users` (via shared functions in `wkmp-common/src/db/init.rs`)
   - Module-specific tables: Created directly by each module (e.g., `queue` for Audio Player, `timeslots` for Program Director)
   - All initialization is idempotent (safe to call multiple times, checks if table exists before creating)
4. Module reads `module_config` table using shared config loader from `wkmp-common`:
   - Shared config loader calls `init_module_config_table()` if table missing
   - If own module's config is missing, inserts default host/port and logs warning
   - If other required modules' configs are missing, inserts defaults and logs warnings
   - Default configurations: user_interface (127.0.0.1:5720), audio_player (127.0.0.1:5721), program_director (127.0.0.1:5722), audio_ingest (127.0.0.1:5723), lyric_editor (127.0.0.1:5724)
5. Module retrieves its own `host` and `port` configuration
6. Module binds to configured address/port
7. Module retrieves other modules' configurations for HTTP client setup (if needed for making requests to peer modules)
8. Module begins accepting connections and making requests to peer modules

**Default Value Initialization Behavior:**

**[ARCH-INIT-020]** When the application reads a configuration value from the database, it SHALL handle missing, NULL, or nonexistent values according to the following rules:

1. **Database Does Not Exist**:
   - Module creates database file and all required tables
   - All settings are initialized with built-in default values
   - Defaults are written to the database immediately

2. **Settings Table Missing**:
   - Module creates `settings` table
   - All settings are initialized with built-in default values
   - Defaults are written to the database immediately

3. **Setting Key Does Not Exist**:
   - Module inserts the setting key with built-in default value
   - Default is written to the database immediately
   - Logged as INFO: "Initialized setting 'key_name' with default value: X"

4. **Setting Value is NULL**:
   - Module treats NULL as missing value
   - Module replaces NULL with built-in default value
   - Default is written to the database immediately (UPDATE operation)
   - Logged as WARNING: "Setting 'key_name' was NULL, reset to default: X"

5. **Built-in Default Values**:
   - All default values are defined in application code, NOT in TOML config files
   - Defaults are version-controlled with the application source code
   - See [Database Schema - settings table](database_schema.md#settings) for complete list of settings and their defaults

**Rationale:**

- **Database is source of truth**: All runtime configuration lives in database, never in TOML files
- **TOML only for bootstrap**: TOML files provide only root folder path, logging config, and static asset paths
- **Self-healing**: NULL values are automatically corrected on read
- **Predictable behavior**: Missing values always get the same built-in defaults
- **Migration-friendly**: Schema changes can add new settings with proper defaults
- **No TOML/database conflicts**: TOML never contains runtime settings, eliminating precedence questions

**Example Default Values** (see database_schema.md for authoritative list):
- `volume_level`: 0.5 (50%)
- `global_crossfade_time`: 2.0 seconds
- `queue_max_size`: 100 passages
- `session_timeout_seconds`: 31536000 (1 year)
- `config_interface_access` (users table): 1 (enabled)

**Module Launch Responsibilities:**
- **User Interface (wkmp-ui)**:
  - Launched by: OS service manager (systemd/launchd/Windows Service)
  - Launches: wkmp-ap (on startup), wkmp-pd (Lite/Full only), wkmp-ai (Full only), wkmp-le (on-demand, Full only)
- **Audio Player (wkmp-ap)**:
  - Launched by: wkmp-ui on startup
  - Can launch: wkmp-pd (if not responding to queue refill requests), wkmp-ui (if needed)
- **Program Director (wkmp-pd)**:
  - Launched by: wkmp-ui on startup (Lite/Full versions only)
  - Can launch: wkmp-ap (if needed for enqueueing), wkmp-ui (if needed)
- **Audio Ingest (wkmp-ai)**:
  - Launched by: wkmp-ui on startup (Full version only)
  - Can launch: wkmp-ui (if needed)
- **Lyric Editor (wkmp-le)**:
  - Launched by: wkmp-ui on-demand when user requests lyric editing (Full version only)
  - Can launch: wkmp-ui (if needed)

---

## Component Implementation Details

This architecture implements the requirements specified in [requirements.md](requirements.md).

Detailed design specifications for each subsystem:
- **Crossfade System**: See [Crossfade Design](crossfade.md)
- **Musical Flavor System**: See [Musical Flavor](musical_flavor.md)
- **Event-Driven Communication**: See [Event System](event_system.md)
- **Data Model**: See [Database Schema](database_schema.md)
- **Project Structure**: See [Project Structure](project_structure.md)
- **Code Organization**: See [Coding Conventions](coding_conventions.md)

## Concurrency Model

### Per-Module Threading

Each module is an independent process with its own threading model:

**Audio Player:**
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

**User Interface:**
```
HTTP Server Thread Pool (tokio async):
  - Web UI serving
  - API request handling
  - SSE aggregation and forwarding
  - Proxy requests to Audio Player and Program Director

Database Query Pool (tokio async):
  - Library browsing queries
  - User data (likes/dislikes)
  - Session management
```

**Program Director:**
```
HTTP Server Thread Pool (tokio async):
  - API request handling
  - SSE broadcasting

Selection Thread (tokio async):
  - Passage selection algorithm
  - Distance calculations
  - Probability computations

Request Handler Thread (tokio async):
  - Receives queue refill requests from Audio Player
  - Acknowledges requests immediately
  - Triggers asynchronous selection
```

**Audio Ingest:**
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

WKMP uses SQLite with UUID-based primary keys for all entities. The complete schema includes:

**Core Entities:** files, passages, songs, artists, works, albums
**Relationships:** passage_songs, passage_albums, song_works
**Playback:** play_history, likes_dislikes, queue
**Configuration:** module_config, timeslots, timeslot_passages, settings
**Caching:** acoustid_cache, musicbrainz_cache, acousticbrainz_cache

See [Database Schema](database_schema.md) for complete table definitions, constraints, indexes, and triggers.

### Key Design Decisions

- **UUID primary keys**: Enable database merging across Full/Lite/Minimal versions
- **Musical flavor vectors**: Stored as JSON in `passages.musical_flavor_vector` for flexibility and SQLite JSON1 integration
- **Automatic triggers**: Update `last_played_at` timestamps on playback for cooldown calculations
- **Foreign key cascades**: Simplify cleanup when files/passages deleted
- **No binary blobs**: Album art stored as files (in root folder tree), database stores relative paths only
- **Event-driven architecture**: Uses `tokio::broadcast` for one-to-many event distribution, avoiding tight coupling between components while staying idiomatic to Rust async ecosystem. See [Event System](event_system.md) for details.
- **Hybrid communication**: Events for notifications, channels for commands, shared state for reads—each pattern chosen for specific use cases

## Version Differentiation

WKMP is built in three versions (Full, Lite, Minimal) by **packaging different combinations of modules**. See [Requirements - Three Versions](requirements.md#three-versions) for detailed feature comparison and resource profiles.

**Implementation approach:**
- **Process-based differentiation**: Different modules are deployed in each version
- **No conditional compilation**: Each module is built identically; versions differ only by which binaries are packaged
- **Packaging strategy**: Version-specific installer packages include different module subsets
- **Database compatibility**: UUID-based schema enables database export/import across versions

**Version Configuration:**

| Version  | Modules Deployed | Features |
|----------|------------------|----------|
| **Full** | wkmp-ap, wkmp-ui, wkmp-le, wkmp-pd, wkmp-ai | All features, lyric editing, local Essentia analysis, file ingest |
| **Lite** | wkmp-ap, wkmp-ui, wkmp-pd | No file ingest, no lyric editing, automatic selection, configuration UI |
| **Minimal** | wkmp-ap, wkmp-ui | Playback only, no lyric editing, manual queue management, no automatic selection |

**Packaging Details:**
- All modules are built with `cargo build --release` (no feature flags)
- Full version packages all 5 binaries (wkmp-ap, wkmp-ui, wkmp-le, wkmp-pd, wkmp-ai)
- Lite version packages 3 binaries (wkmp-ap, wkmp-ui, wkmp-pd)
- Minimal version packages 2 binaries (wkmp-ap, wkmp-ui)
- See [Implementation Order - Version Packaging](implementation_order.md#phase-9-version-packaging--module-integration-25-weeks) for packaging details

## Technology Stack

### Core Technologies

**Programming Language:**
- Rust (stable channel)
- Async runtime: Tokio

**HTTP Server Framework (all modules):**
- Axum or Actix-web (to be determined during implementation)
- Server-Sent Events (SSE) support
- JSON request/response handling

**Audio Processing (Audio Player only):**
- GStreamer 1.x
- Rust bindings: gstreamer-rs

**Database:**
- SQLite 3.x (embedded in each module)
- rusqlite crate for Rust bindings
- JSON1 extension for flavor vector storage

**External API Clients:**
- reqwest for HTTP clients
- MusicBrainz, AcousticBrainz, Chromaprint/AcoustID

**Local Audio Analysis (Audio Ingest, Full version only):**
- Essentia C++ library
- Rust FFI bindings (custom or via existing crate)

**Web UI (User Interface and Audio Ingest):**
- HTML/CSS/JavaScript (framework TBD - React, Vue, or Svelte)
- SSE client for real-time updates
- Responsive design framework (TailwindCSS or similar)

**Configuration:**
- **Database first**: ALL runtime settings stored in database (`settings` and `module_config` tables)
- **TOML files**: Bootstrap configuration ONLY (root folder path, logging, static asset paths)
- **Default value initialization**: When database settings are missing/NULL, application initializes with built-in defaults and writes to database
- Database and all files contained in root folder tree for portability

**Build System:**
- Cargo workspaces for multi-module project (see [Project Structure](project_structure.md))
  - `common/` - Shared library crate (`wkmp-common`)
  - `wkmp-ap/`, `wkmp-ui/`, `wkmp-pd/`, `wkmp-ai/` - Binary crates
- Separate binaries for each module
- Version differentiation via selective packaging (no conditional compilation)
- Shared dependencies managed at workspace level

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
- Root folder: User-configurable (default: platform-specific app data directory)
- Database: `wkmp.db` in root folder
- Audio/artwork files: Organized under root folder tree
- Settings: Platform-specific config directory (stores root folder path)
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
- Relative file paths only (no file contents stored in database)
- All paths relative to root folder for portability
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
- Network failures → Retry with fixed 5-second delay (see Network Error Handling below)
- Missing files → Skip, remove from queue, log
- Database lock → Retry with exponential backoff (see Database Lock Timeout below)
- Decode errors → Skip to next passage (see GStreamer Pipeline Errors below)
- Program Director timeout → Continue with existing queue, retry on next threshold

### Error Recovery Strategies

This section specifies the detailed recovery procedures for common error scenarios in wkmp-ap.

#### GStreamer Pipeline Errors

**[ARCH-ERR-PLAYBACK-010]** GStreamer Pipeline Error Recovery:

When a GStreamer pipeline enters ERROR state (file not found, decode failure, audio device unavailable, etc.), the following recovery procedure is executed:

1. **Log error** with pipeline state and error details
2. **Handle as skip event**: From this point, any playback failure is treated identically to a user-initiated skip:
   - Emit `PassageCompleted(completed=false)` event with appropriate reason:
     - `reason: "playback_error"` if decode or pipeline failure
     - `reason: "queue_removed"` if file not found or inaccessible
   - Remove failed passage from queue
3. **Advance to next passage**:
   - If next passage already started (crossfade in progress): Continue from its current position
   - If next passage not yet started: Begin playback from beginning
   - Fade-in curve of next passage is **unaffected** by the skip (continues as if previous passage had played normally)
4. **Continue until queue exhausted**:
   - Multiple consecutive failures continue to log errors and skip passages
   - When queue becomes empty: Audio player enters idle state (same as normal empty queue behavior)
   - Player produces no audio until a new passage is enqueued

**[ARCH-ERR-PLAYBACK-020]** Crossfade Behavior During Pipeline Error:

- If error occurs during crossfade (both pipelines active):
  - Failed pipeline stops immediately
  - Surviving pipeline continues playing without interruption
  - No fade adjustment applied (surviving pipeline maintains its current fade curve)
- If error occurs in idle pipeline (pre-loading next passage):
  - Current pipeline continues playing normally
  - Failed pre-load logged as error
  - Next passage skip logic applies when current passage completes

**[ARCH-ERR-PLAYBACK-030]** Automatic Queue Refill Throttling (wkmp-pd responsibility):

**Note:** This mechanism is implemented in wkmp-pd (Program Director), not wkmp-ap (Audio Player). Documented here for completeness.

- wkmp-pd monitors `PassageCompleted(completed=false)` events
- After 3 playback failures within 60 seconds (configurable):
  - wkmp-pd stops automatic passage enqueueing
  - User notification displayed in UI
  - Manual intervention required to resume automatic selection
- Default settings (configurable in settings table):
  - `playback_failure_threshold`: 3 failures
  - `playback_failure_window_seconds`: 60 seconds

#### Database Lock Timeout

**[ARCH-ERR-DB-010]** Database Lock Retry Strategy:

When a database operation fails due to lock timeout (SQLite `SQLITE_BUSY` error), the following retry logic applies:

1. **Retry with exponential backoff**:
   - Attempt 1: Immediate (no delay)
   - Attempt 2: Wait 10ms, retry
   - Attempt 3: Wait 20ms, retry
   - Attempt 4: Wait 40ms, retry
   - Attempt 5: Wait 80ms, retry
   - Attempt 6: Wait 160ms, retry
   - Attempt 7: Wait 320ms, retry
   - Attempt 8: Wait 640ms, retry (final attempt)

2. **Maximum 7 retries** (8 total attempts including initial)

3. **Total maximum wait time**: 1,270ms (10+20+40+80+160+320+640)

4. **If all retries fail**:
   - Log error with operation details and retry count
   - Continue with cached state (if applicable)
   - For critical operations (queue writes): Return error to caller, trigger UI notification

**[ARCH-ERR-DB-020]** Cached State Fallback:

Operations that can use cached state on lock timeout:
- Queue read: Use last successfully read queue (may be stale)
- Settings read: Use last successfully read settings
- Module config read: Use last known configuration

Operations that **cannot** use cached state (require retry or failure):
- Queue write: Must succeed or return error
- Settings write: Must succeed or return error
- Playback position persistence: Failure is acceptable (position lost on crash)

**[ARCH-ERR-DB-030]** Lock Timeout Configuration:

SQLite busy timeout is set to 5000ms (5 seconds) at connection initialization:
```rust
connection.busy_timeout(Duration::from_millis(5000))?;
```

This timeout applies **before** the exponential backoff retry logic. The exponential backoff provides additional resilience for transient lock contention.

#### Program Director Timeout

**[ARCH-ERR-PD-010]** Program Director Timeout Handling:

When wkmp-ap sends a queue refill request to wkmp-pd (Program Director) and does not receive acknowledgment within the timeout period:

1. **Timeout detection**:
   - Timeout configured in settings table: `queue_refill_acknowledgment_timeout_seconds` (default: 5 seconds)
   - Timer starts when `POST /selection/request` is sent to wkmp-pd
   - Timeout triggered if no response received within configured duration

2. **Log warning**:
   - Log message: `"Program Director timeout after {timeout}s, continuing with existing queue"`
   - Include request details (anticipated start time, current queue status)

3. **Continue playback**:
   - Do **not** halt playback
   - Continue playing existing queue passages
   - Do **not** attempt to relaunch wkmp-pd (that is wkmp-ui's responsibility)

4. **Retry on next threshold trigger**:
   - When queue drops below refill threshold again, send new request to wkmp-pd
   - Fresh timeout timer starts for new request
   - No cumulative failure tracking in wkmp-ap (wkmp-ui handles module health monitoring)

**[ARCH-ERR-PD-020]** Request Throttling:

To prevent request spam during wkmp-pd unavailability:
- Minimum interval between refill requests: `queue_refill_request_throttle_seconds` (default: 10 seconds)
- Throttle applies even after timeout
- If queue drops below threshold during throttle period: Wait until throttle expires, then send request

**[ARCH-ERR-PD-030]** Empty Queue Behavior:

If queue becomes empty while wkmp-pd is unresponsive:
- Audio player enters idle state (no audio output)
- Continues attempting refill requests at throttle interval
- Resumes playback automatically when wkmp-pd responds with passage

**[ARCH-ERR-PD-040]** Module Health Monitoring (wkmp-ui responsibility):

**Note:** wkmp-ui (User Interface) is responsible for detecting unresponsive modules and relaunching them. wkmp-ap only logs timeouts and continues operation.

### Network Error Handling

**[ARCH-NET-010]** WKMP requires two distinct types of network access with different error handling strategies:

**Internet Access (External APIs - Full version only):**

Used for:
- MusicBrainz metadata lookup during library import
- AcousticBrainz musical flavor data retrieval
- Cover art fetching
- Future ListenBrainz integration (Phase 2)

**[ARCH-NET-020]** Retry Algorithm:
- **Fixed delay**: Wait exactly 5 seconds between each retry attempt (not exponential backoff)
- **Retry limit**: Maximum of 20 consecutive retry attempts
- After 20 failures, stop attempting until user triggers reconnection
- Reconnection triggers:
  - User clicks any UI control that requires internet (Import, metadata refresh, etc.)
  - User explicitly clicks "Retry Connection" button
  - Counter resets to 20 attempts on each user-triggered reconnection

**Rationale for fixed delay:**
- Simplicity: No complex backoff calculation needed
- Predictability: User knows exactly when next attempt occurs
- Resource efficiency: 5-second intervals are reasonable for external API failures
- Total duration: 20 retries × 5s = 100 seconds maximum before requiring user intervention

**Example retry sequence:**
- Attempt 1: Fail → wait 5s
- Attempt 2: Fail → wait 5s
- Attempt 3: Fail → wait 5s
- ...
- Attempt 20: Fail → stop, display "Connection Failed" message

**[ARCH-NET-030]** Playback Impact:
- No impact on playback: Music continues playing during internet outages
- Playback uses only local database and audio files (no internet required)

**Local Network Access (WebUI Server):**

Used for:
- Serving WebUI on `http://localhost:5720`
- Server-Sent Events (SSE) for real-time UI updates
- REST API endpoints for playback control

**[ARCH-NET-040]** Error Handling:
- HTTP server binds to localhost:5720 on startup
- If port binding fails: Log error and exit (critical failure)
- Once running, server continues indefinitely

**[ARCH-NET-050]** Access Requirements:
- Localhost access: Always available (no network required)
- Remote access: Requires local network connectivity
  - User responsible for network configuration (router, firewall, etc.)
  - No internet required (local network only)

**[ARCH-NET-060]** Playback Impact:
- Automatic playback: Works without any network access
  - System auto-starts on boot
  - Auto-selects and plays passages
  - No WebUI access needed for basic operation
- Manual control: Requires WebUI access (localhost or remote)

> **See:** [UI Specification - Network Status Indicators](ui_specification.md#network-status-indicators) for user-facing status display
> **See:** [Requirements - Network Error Handling](requirements.md#network-error-handling) for complete requirements

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
- SQLite `module_config` table for network configuration (host/port for each module)
- SQLite `settings` table for user preferences
- File-based config for deployment settings (root folder path, logging, application-specific settings)
- Sane defaults for all optional settings
- Centralized network configuration eliminates the need to duplicate module URLs across config files

### Distribution
- **Multiple binaries per version**: Each module is a separate binary
- **Version-specific packaging**:
  - Full: 5 binaries (Audio Player, User Interface, Lyric Editor, Program Director, Audio Ingest)
  - Lite: 3 binaries (Audio Player, User Interface, Program Director)
  - Minimal: 2 binaries (Audio Player, User Interface)
- **Bundled dependencies**: GStreamer (Audio Player only), SQLite (all modules)
- **Installer packages**: deb, rpm, msi, dmg with systemd/launchd service files
- **Process management**: System service manager or manual startup scripts
- **Configuration files**: Default ports, module URLs, root folder path

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
End of document - WKMP Architecture
