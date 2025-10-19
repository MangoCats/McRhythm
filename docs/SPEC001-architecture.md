# WKMP Architecture

**ðŸ—ï¸ TIER 2 - DESIGN SPECIFICATION**

Defines HOW the system is structured. Derived from [requirements.md](REQ001-requirements.md). See [Document Hierarchy](GOV001-document_hierarchy.md), and [Requirements Enumeration](GOV002-requirements_enumeration.md).

> **Related Documentation:** [Database Schema](IMPL001-database_schema.md) | [Crossfade Design](SPEC002-crossfade.md) | [Musical Flavor](SPEC003-musical_flavor.md)| [Event System](SPEC011-event_system.md)

---

## Overview

WKMP is a music player built on Rust and SQLite that automatically selects music passages based on user-configured musical flavor preferences by time of day, using cooldown-based probability calculations and AcousticBrainz musical characterization data. Audio playback uses a custom single-stream architecture with sample-accurate crossfading powered by symphonia (decoding), rubato (resampling), and cpal (output).

WKMP implements a **microservices architecture with 5 core processes** communicating via HTTP APIs and Server-Sent Events (SSE). This enables simplified maintenance, version flexibility, and independent module updates.

### Extensibility Principle

**[ARCH-EXT-010]** The microservices architecture is designed to accommodate additional modules beyond the current 5 core services. The system's HTTP/SSE-based communication pattern, shared SQLite database, and module discovery mechanisms (via `module_config` table) enable future expansion without architectural changes.

**Current Core Modules (5):**
- **Audio Player (wkmp-ap)** - Port 5721 - Core playback engine with queue management
- **User Interface (wkmp-ui)** - Port 5720 - Polished web UI for end users
- **Program Director (wkmp-pd)** - Port 5722 - Automatic passage selection (Full and Lite versions only)
- **Audio File Ingest (wkmp-ai)** - Port 5723 - New file import workflow (Full version only, on-demand)
- **Lyric Editor (wkmp-le)** - Port 5724 - Standalone lyric editing tool (Full version only, on-demand)

**Potential Future Modules:**
- News and Weather Integration - Text-to-speech audio segments reading current local news and weather reports between passages
- Alternative UI Implementations - Specialized interfaces (mobile-optimized, accessibility-focused, minimal kiosk mode)
- Additional Content Sources - Streaming integration, podcast support, audiobook playback
- External Control Interfaces - MPD protocol compatibility, voice assistant integration, MQTT control

**Design Benefits:**
- **Simplifies maintenance**: Each module focuses on a single concern
- **Enables version flexibility**: Run more/fewer processes for Full/Lite/Minimal versions
- **Provides modularity**: Update one module without affecting others
- **Supports independent operation**: Audio Player and Program Director work without UI
- **Accommodates future expansion**: New modules can be added without modifying existing services

## Process Architecture

**[ARCH-SYS-010]** WKMP consists of **5 independent processes** (depending on version, Full deployments run all 5), each with defined HTTP/SSE interfaces:

- **Audio Player** - Core playback engine with queue management
- **User Interface** - Polished web UI for end users
- **Lyric Editor** - Standalone lyric editing tool (launched on-demand, Full version only)
- **Program Director** - Automatic passage selection (Full and Lite versions only)
- **Audio File Ingest** - New file import workflow (launched on-demand, Full version only)

**[ARCH-SYS-020]** Design Benefits:
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
    ┌───────"¼────────┬────────────────────────┬───────────────┐
    │       │        │                        │               │
    ▼       ▼        ▼                        ▼               ▼
┌───────┐ ┌────────────────┐  ┌──────────────────────────────┐ ┌─────────────┐
│ Audio │ │  Audio Player  │  │  Program Director            │ │Lyric Editor │
│ File  │ │  Port: 5721    │  │  Port: 5722                  │ │  Port: 5724 │
│Ingest │ │                │—„─┤ (Full and Lite only)         │ │ (Full only) │
│  UI   │ │  - Minimal     │  │  - Minimal dev UI            │ │  - Split UI │
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

**[ARCH-VER-010]** Process deployment by version:

| Version  | Audio Player | User Interface | Program Director | Audio Ingest | Lyric Editor |
|----------|--------------|----------------|------------------|--------------|--------------|
| **Full**     | ✅ Running | ✅ Running | ✅ Running | ✅ On-demand<br/><small><small>(invoked only during<br/>ingest sessions)</small></small>| ✅ On-demand<br/><small><small>(invoked only during<br/>lyric editing)</small></small> |
| **Lite**     | ✅ Running | ✅ Running | ✅ Running | âŒ Not included | âŒ Not included |
| **Minimal**  | ✅ Running | ✅ Running | âŒ Not included | âŒ Not included | âŒ Not included |

## Module Specifications

### Audio Player

**[ARCH-COMP-010]** Audio Player Module Specification:

**Process Type**: Independent HTTP server with minimal HTML developer UI (served via HTTP)
**Port**: 5721 (configurable)
**Versions**: Full, Lite, Minimal

**[ARCH-PC-010]** Responsibilities:
- Implements single-stream audio architecture with sample-accurate crossfading (~0.02ms precision)
- Decodes audio files using symphonia (MP3, FLAC, AAC, Vorbis in pure Rust; Opus via C library FFI)
- Buffers decoded PCM audio in memory with automatic fade application
- Coordinates passage transitions based on lead-in/lead-out timing
- Implements five fade curves (Linear, Logarithmic, Exponential, S-Curve, Equal-Power)
- Handles pause (immediate stop) and resume (configurable fade-in, default: 0.5s exponential)
- Manages volume control (user level + fade automation)
- Maintains playback queue with persistence
- Outputs audio via cpal (cross-platform: PulseAudio, ALSA, CoreAudio, WASAPI)

**HTTP Control API:**
- `POST /audio/device` - Set audio output device
- `POST /audio/volume` - Set volume level (0.0-1.0)
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
- `GET /files/browse?path=<dir>` - Browse filesystem for audio files (developer UI support)

**SSE Events** (Endpoint: `GET /events`):
- `VolumeChanged` - Volume level updated
- `QueueChanged` - Queue modified (add/remove/reorder)
- `PlaybackStateChanged` - Playing/Paused state changed
- `PlaybackProgress` - Position updates (configurable interval, default: 5 seconds)
- `PassageStarted` - New passage began playing
- `PassageCompleted` - Passage finished
- `CurrentSongChanged` - Within-passage song boundary crossed

**Developer UI** (Minimal HTML/JavaScript served via HTTP):
- Module status display
  - queue contents
  - playing/paused state
  - playback position in passage (elapsed/total)
  - volume level
  - audio output device
- Direct API testing interface
  - set audio device
  - set volume level
  - enqueue an audio file (with file browser modal for selecting files)
  - remove passage from queue
- File browser modal
  - browse directories within configured root folder
  - navigate subdirectories and parent directories
  - filter display to show only audio files (mp3, flac, ogg, wav, m4a, aac, opus, wma)
  - security: path traversal prevention (canonicalization + root folder constraint)
  - click file to auto-populate enqueue input field
- Event stream monitor
  (Configuration settings editor available only to authorized users.)

**State:**
- Currently playing passage (position, duration, state)
- Next passage (pre-loaded, ready for crossfade)
- Queue contents (persisted to SQLite)
- User volume level (0-100)
- Playback state (Playing/Paused only - no "stopped" state)
- Initial state on app launch: Determined by `initial_play_state` setting (default: "playing")

**[ARCH-PC-020]** Key Design Notes:
- **Operates independently**: Does not require User Interface or Program Director to be running
  - Any application capable of communicating to the Control API can enqueue passages and control
    playback state, volume, output device, etc.  wkmp-ap otherwise plays these enqueued passages
    independently without need for communication from any other module.
- **Receives commands from**: User Interface, Program Director
  - wkmp-ap has no knowledge of user identity, the API is open (implicitly insecure) and accepts
    any valid control messages.
- **Database access**: Direct SQLite access for queue persistence, passage metadata

<a id="arch-queue-persist-030"></a>
<a id="queue-persistence"></a>
### Queue and State Persistence

**[ARCH-QP-010]** Queue Persistence Strategy:
- Queue contents written to SQLite immediately on every queue modification (enqueue/dequeue/reorder)
- Each queue entry stored with passage reference and timing specifications
- Queue changes are synchronous writes (blocking until persisted)
- Single database design (queue stored with library data)

**[ARCH-QP-020]** Playback Position Persistence:
- Playback position transmitted via SSE at configurable interval (setting: `playback_progress_interval_ms`, default 5000ms)
- Also transmitted once when Pause initiated, once when Play initiated
- Playback position persisted **on clean shutdown and when Pause or Play initiated**
- On any queue change, `last_played_position` automatically reset to 0 in settings
- On startup: if `last_played_position` > 0, resume first passage in the queue from that position; otherwise start the first passage in the queue from its start point
- No special crash detection needed - queue change reset handles both crash and normal operation

**[ARCH-BM-010]** Database Backup Strategy (wkmp-ui responsibility):

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

**[ARCH-BM-020]** Backup Configuration (settings table):
- `backup_location`: Path to backup directory (default: same folder as wkmp.db)
- `backup_interval_ms`: Time between periodic backups (default: 90 days)
- `backup_minimum_interval_ms`: Minimum time between startup backups (default: 14 days)
- `backup_retention_count`: Number of timestamped backups to keep (default: 3)
- `last_backup_timestamp_ms`: Unix milliseconds of last successful backup

**[ARCH-BM-030]** Backup Failure Handling:
- Network backup location unreachable: Fall back to local backup path, log warning
- Timeout: 30 seconds for network writes
- Startup never blocked by backup failure (only by integrity check and restore if needed)

### Initial Play State

**[ARCH-STRT-005]** Initial Play State Configuration:
- Setting: `initial_play_state` (string: "playing" or "paused", default: "playing")
- Determines playback state on app launch
- Current playback state is never persisted across restarts

**[ARCH-STRT-010]** Cold Start Procedure:
1. Run database integrity check and backup (if wkmp-ui; see ARCH-QP-030)
2. Initialize audio device (see ARCH-AUDV-010 below)
3. Read `initial_play_state` from settings (default: "playing")
4. Set playback state according to setting
5. Read queue from database (ORDER BY play_order)
6. Read `last_played_passage_id` and `last_played_position` from settings
7. Determine action:
   - **Queue empty + Playing**: Wait in Playing state (plays immediately when passage enqueued)
     - User-facing state: "playing"
     - Internal audio state: Audio output thread continues but receives silence from empty mixer
     - See [single-stream-playback.md - Queue Empty Behavior](SPEC013-single_stream_playback.md#queue-empty-behavior) for implementation details
   - **Queue empty + Paused**: Wait silently
   - **Queue has passages + Playing**: Begin playback
   - **Queue has passages + Paused**: Load first passage but don't play
8. Starting position:
   - If `last_played_passage_id` matches first queue entry AND `last_played_position` > 0: Resume from position
   - Otherwise: Start from passage `start_time_ms`

**[ARCH-AUDV-010]** Audio Device Initialization:

On module startup, wkmp-ap must initialize an audio output device before playback can begin.

1. **Read persisted device setting:**
   - Query `settings` table for `audio_sink` value
   - If value is NULL or empty string: Proceed to step 2
   - If value exists: Proceed to step 3

2. **First-time startup (no persisted device):**
   - Use system default device (cpal default host/output device)
   - Query cpal to determine which actual device was selected
   - Write selected device_id to `settings.audio_sink` for future startups
   - Log: "Audio device initialized: [device_name] (system default)"

3. **Subsequent startup (device persisted):**
   - Query available audio devices from cpal host
   - Check if persisted `audio_sink` device_id exists in available devices list
   - **If device found**: Use persisted device, log: "Audio device restored: [device_name]"
   - **If device NOT found** (unplugged USB, disconnected Bluetooth, etc.):
     - Log warning: "Persisted audio device '[device_id]' not found, falling back to system default"
     - Use system default device (cpal default output)
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
   - `"default"`: Always uses cpal default output device (system default selection)
   - Specific device_id (e.g., `"alsa_output.usb-..."`, `"pulse_output.1"`): Uses exact cpal device

**[ARCH-STRT-020]** Queue Entry Validation:
- Validated lazily when scheduled for playback
- If file missing when playback attempted:
  1. Log error with passage ID and file path
  2. Emit `PassageCompleted(completed=false)` event
  3. Remove invalid entry from queue
  4. Advance to next passage
  5. Continue if in Playing state

**[ARCH-STRT-025]** Queue Lifecycle:
- Queue is forward-looking only (passages waiting to play)
- Currently playing passage tracked via `currently_playing_passage_id` setting
- Completed passages removed from queue immediately (FIFO)
- Play history stored separately in `song_play_history` table (single table for all songs)

**[ARCH-QM-010]** Play Order Management:
- New passages appended with `play_order = last_play_order + 64`
- Gaps enable insertion without renumbering (e.g., insert at 96 between 64 and 128)
- When inserting and no gap available (e.g., 128, 129), renumber tail: `UPDATE queue SET play_order = play_order + 8 WHERE play_order >= 128`
- Typical queue depth: 5-10 passages (graceful degradation up to 1000+, but performance not priority concern beyond that)

**[ARCH-QM-020]** Play Order Overflow Protection:
- `play_order` stored as signed 32-bit integer (max: 2,147,483,647)
- At typical usage (3 min/passage, +64 increment): 191 years until overflow
- If `play_order` exceeds 2,100,000,000: Trigger automatic renumbering
  - Renumber entire queue starting from 64 (64, 128, 192...)
  - Happens transparently during enqueue operation
  - Extremely rare (abuse/hack scenario only)

### Song Boundary Detection (CurrentSongChanged Event)

**[ARCH-SNGC-010]** Passage vs Song Relationship:

A **passage** is a continuous subset of an audio file played from its `start_time_ms` to `end_time_ms`.

Key characteristics:
- Passages are continuous playback regions within audio files
- Multiple passages can be defined within a single audio file
- Passages may overlap or have gaps between them
- The same region of an audio file can play in both lead-out of one passage and lead-in of next passage
- Each passage contains zero or more **songs** (defined in `passage_songs` table)
- Songs never overlap other songs within a passage, but they may have gaps
- an audio file with no passage_id and no defined lead or fade timing information is, by default, an 
  unidentified passage that plays from the start of the audio file through to the end with zero fade-in, 
  fade-out, lead-in and lead-out durations.
- when no passage_id is provided with an audio file for playback, no song_id can be determined, only the audio_file_path

**[ARCH-SNGC-020]** Song Timeline Construction:

The `passage_songs` table (also called a "cut list" in music production) defines which songs exist within each passage and their time boundaries.

When a passage starts playing:
1. Query `passage_songs` table for current passage: `SELECT * FROM passage_songs WHERE passage_id = ? ORDER BY start_time`
2. Build song timeline in memory: List of `{song_id, start_time_ms, end_time_ms}`
3. Store timeline for duration of passage playback
4. Timeline remains valid until passage completes (passages play continuously, timeline doesn't change)

**[ARCH-SNGC-030]** CurrentSongChanged Emission:

During playback, wkmp-ap uses **event-driven position tracking** to detect song boundary crossings:

1. **Position event generation:** Mixer emits `PositionUpdate` internal events
   - **Configurable interval**: Database setting `position_event_interval_ms` (default: 1000ms)
   - **Emission timing**: Every `(position_event_interval_ms / 1000.0) * sample_rate` frames
   - **Example**: At 44.1kHz with 1000ms interval → event every 44,100 frames (~1 second of audio)
   - Event contains: frame position, queue entry ID, sample rate, mixer state
   - Non-blocking emission via MPSC channel (capacity: 100 events)
   - **See [Database Schema](IMPL001-database_schema.md#event-timing-intervals---detailed-explanation)** for interval configuration details

2. **Event-driven boundary detection:**
   - Position event handler receives `PositionUpdate` events
   - Converts frame position to milliseconds using sample rate
   - Compares `position_ms` to each song's `[start_time_ms, end_time_ms]` range in timeline
   - Determines if position crossed into different song or gap since last event
   - **Detection latency**: 0 to `position_event_interval_ms` (configurable, default: 0-1000ms)

3. **Event emission:** Emit `CurrentSongChanged` SSE event when:
   - Passage starts, in a song or gap
   - Position crosses from one song to another
   - Position crosses from song to gap (no song at current position)
   - Position crosses from gap to song
   - Passage ends, in a song or gap
   
3.1 **Event emission, no passage_id:** Emit `CurrentSongChanged` when:
   - Passage starts, song_id and passage_id are None, audio_file_path contains path and filename of file to play
   - Passage ends, song_id, passage_id and audio_file_path are None

Do not emit `CurrentSongChanged` when passage starts or ends in a gap.


4. **Event payload:**
   ```rust
   CurrentSongChanged {
       song_id: Option<SongId>,          // Current song UUID, or None if in gap or song is unknown
       passage_id: PassageId,            // ALWAYS present during passage playback
       audio_file_path: Option<PathBuf>, // Current audio file's path, None at the end of passage event
       pipeline_id: PipelineId,          // Pipeline playing this passage (A or B)
       position_ms: u64,                 // Current position in passage (milliseconds)
       timestamp: SystemTime,            // When boundary was crossed
   }
   ```

**[ARCH-SNGC-031]** CurrentSongChanged Event - Passage Identity:
- `passage_id` (PassageId): ALWAYS present during passage playback
- For persistent passages: References `passages` table entry
- For ephemeral passages: Transient UUID for current session only
- See entity_definitions.md REQ-DEF-035 for ephemeral passage model
- `passage_id=None` only valid at system start (before first passage) or explicit stop

5. **Gap handling:**
   - If `current_position_ms` is not within any song's time range: `song_id = None`
   - Gaps between songs are normal (not errors)
   - UI should handle `None` gracefully (e.g., clear "now playing" song info, show passage info instead)

**[ARCH-SNGC-040]** Implementation Notes:

- Song timeline built **only once** per passage (on `PassageStarted`)
- No periodic re-reading of `passage_songs` table during playback
- Boundary checks use simple time range comparisons (no complex state machine)
- **Event-driven detection**: Boundary checks occur when mixer emits position update events (~1 event/second of audio)
  - Position events are tied to actual frame generation, not polling timers
  - Detection only triggers SSE emission when transition is detected
  - Typical latency: <50ms (determined by ring buffer size and event channel)
- First `CurrentSongChanged` emitted immediately on passage start (if passage begins within a song and not a gap)

**[ARCH-SNGC-041]** Song Timeline Data Structure:

The song timeline is stored as a **sorted Vec** in memory:

```rust
struct SongTimelineEntry {
    song_id: Option<Uuid>,       // None for gaps between songs
    start_time_ms: u64,          // Start time within passage
    end_time_ms: u64,            // End time within passage
}

struct SongTimeline {
    entries: Vec<SongTimelineEntry>,  // Sorted by start_time_ms ascending
    current_index: usize,              // Index of currently playing entry (cache)
}
```

**[ARCH-SNGC-042]** Efficient Boundary Detection Algorithm:

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
                song_id: entry.song_id,
                passage_id: self.current_passage_id,
                audio_file_path: self.audio_file_path,
                pipeline_id: self.pipeline_id,
                position_ms: current_position_ms,
                timestamp: SystemTime::now(),
            });
        }
    }

    // Position is in a gap (between songs or after last song)
    // Emit CurrentSongChanged with song_id=None
    Some(CurrentSongChanged {
        song_id: None,
        passage_id: self.current_passage_id,
        audio_file_path: if after_last_song { None } else { self.audio_file_path },
        pipeline_id: self.pipeline_id,
        position_ms: current_position_ms,
        timestamp: SystemTime::now(),
    })
}
```

**[ARCH-SNGC-044]** Songs Cannot Overlap Within Passage:

- The `passage_songs` table enforces non-overlapping songs within a single passage
- Database constraint: No two songs in the same passage may have overlapping time ranges
- This simplifies boundary detection (no ambiguity about which song is "current")

**[ARCH-SNGC-050]** Edge Cases:

- **Passage with no songs:** Emit `CurrentSongChanged` with `song_id=None` on passage start
- **Passage starts in gap:** Emit with `song_id=None`, then emit again when entering first song
- **Passage ends during gap:** Emit `CurrentSongChanged` with `song_id=None` and `audio_file_path=None` on passage end
- **Passage ends in Song:** Emit with `song_id=None` and `audio_file_path=None` on passage end
- **Songs with identical boundaries:** Emit `CurrentSongChanged` of the song that is starting
- **Seeking:** After seek, immediately check position against timeline and emit `CurrentSongChanged` if song changed

**[ARCH-SNGC-060]** Performance Considerations:

- Song timeline stored in memory (typically <100 songs per passage, minimal memory impact)
- Boundary checks are O(n) where n = songs in passage (acceptable for typical passage sizes)
  - If the future should bring large passages (>1000 songs), consider binary search on sorted timeline
- **Event-driven architecture**: Position events emitted only when frames are generated
  - **Position event frequency**: Configurable via `position_event_interval_ms` (default: 1000ms)
    - Lower values (100-500ms): More responsive boundary detection, higher CPU usage
    - Higher values (2000-5000ms): Lower CPU usage, delayed boundary detection
  - During playback: Events emitted at configured interval (tied to actual audio generation)
  - When paused: No events emitted (mixer stops generating frames)
  - **CPU overhead**: Proportional to event frequency
    - At 1000ms interval: <0.1% CPU (event emission + boundary checking)
    - At 100ms interval: ~1% CPU (10x event frequency)
  - **Memory overhead**: ~10KB (event channel buffer + song timeline)
  - **See [Database Schema - Event Timing Intervals](IMPL001-database_schema.md#event-timing-intervals---detailed-explanation)** for configuration guidance

### Volume Handling
<a name="volume-handling"></a>

**[ARCH-VOL-010]** Volume Scale:
- **System-wide** (API, storage, audio pipeline, SSE events): Double 0.0-1.0
- **UI Display Only**: UI components convert to integer 0-100 (percentage) for user display
- **Conversion** (UI layer only):
  - User input → API: `api_volume = user_input / 100.0`
  - API → User display: `user_display = round(api_volume * 100.0)`
- **Rationale**: Consistent 0.0-1.0 scale across all backend systems eliminates conversion errors

**Storage and Transmission:**
- Database: Store as double (0.0-1.0) in `settings.volume_level`
- HTTP API: Accept/return double (0.0-1.0) in JSON
- SSE Events: Transmit as double (0.0-1.0) for precision in real-time streams
- Audio pipeline: Use double (0.0-1.0) for volume multipliers

### User Interface

**[ARCH-COMP-020]** User Interface Module Specification:

**Process Type**: Polished HTTP server with full web UI
**Port**: 5720 (configurable)
**Versions**: Full, Lite, Minimal

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
- Events from Audio Player trigger user interface updates
- Adds user-specific events (session, likes/dislikes, manual passage enqueue)

**Web UI Features:**
- Authentication flow (Anonymous/Create Account/Login)
- Now Playing: Album art, song/artist/album, passage title, lyrics
- Playback controls: Play/Pause, Skip, volume slider
- Queue display and manual queue management
- Like/Dislike buttons (Full/Lite versions)
- Library browser for manual user selection of passages to enqueue
- Program Director configuration (timeslots, base probabilities, cooldowns)
- Network status indicators (local network)
- Responsive design for desktop and mobile

**[ARCH-COMP-021]** Lyrics Display Behavior:
- Implements fallback chain when displaying lyrics for currently playing Song:
  1. Check current Song's `lyrics` field - if non-empty, display these lyrics
  2. If empty, iterate through Song's `related_songs` array (most to least closely related)
  3. Display lyrics from first related Song with non-empty `lyrics` field
  4. If no Song in chain has lyrics, leave lyrics display area empty
- Read-only display in wkmp-ui (all editing via wkmp-le in Full version)

**Version Differences:**
- **Full**: All features enabled
- **Lite**: No links to file ingest or lyrics editing interfaces
- **Minimal**: No links to file ingest or lyrics editing interfaces, user always operates as Anonymous
  - UI elements for login and account creation are completely hidden
  - No authentication system present (hardcoded to Anonymous user)
  - No Like/Dislike features (Full and Lite only per [requirements.md#like-dislike](REQ001-requirements.md#like-dislike))

**[ARCH-COMP-022]** Key Design Notes:
- **Most users interact here**: Primary interface for controlling WKMP
- **Orchestration layer**: Coordinates between Audio Player and Program Director
- **Database access**: Direct SQLite access for user data, likes/dislikes, library browsing

---

### Configuration Interface Access Control

**[ARCH-USER-010]** Each microservice module (wkmp-ap, wkmp-ui, wkmp-pd, wkmp-ai, wkmp-le) SHALL provide an access-restricted configuration interface page that enables authorized users to both view and edit configuration settings that affect the module.

*(All â€œconfiguration settings editorâ€ mentions in module sections above refer to this unified interface which appears
  in each microservice's http interface, when the current user is authorized to access it.)*

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
- See [Database Schema - users table](IMPL001-database_schema.md#users) for column definition
- See [Deployment - Password Reset Tool](IMPL004-deployment.md) for command-line tool specification

---

### Lyric Editor

**[ARCH-COMP-030]** Lyric Editor Module Specification:

**Process Type**: Independent HTTP server with split-window UI (launched on-demand)
**Port**: 5724 (configurable)
**Versions**: Full only

**Responsibilities:**
- Provides dedicated interface for editing song lyrics
- Displays split window: text editor (left) + embedded browser (right)
- Loads and saves lyrics associated with MusicBrainz recordings, associates them with songs in the database.
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
  (Configuration settings editor available only to authorized users.)

**[ARCH-COMP-031]** Key Design Notes:
- **On-demand launching**: Typically started by the User Interface when a user requests lyric editing,
but may also be launched manually or by external tools if desired.
- **Standalone operation**: Fully independent process that can run and edit lyrics without any other
module active, provided it can access the shared database.
- **Optional UI integration**: The User Interface simply acts as a convenience launcher and
controller; once running, the Lyric Editor communicates directly with the shared database and
operates independently.
- **Read-only in UI**: User Interface displays lyrics, but all editing happens exclusively in Lyric Editor.
- **Simple concurrency**: Last write wins, no conflict resolution needed.
---

### Program Director

**[ARCH-COMP-040]** Program Director Module Specification:

**Process Type**: Independent HTTP server with minimal HTML developer UI (served via HTTP)
**Port**: 5722 (configurable)
**Versions**: Full, Lite (Minimal does not include automatic selection)

**[ARCH-PD-010]** Responsibilities:
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
  (Configuration settings editor available only to authorized users.)

**[ARCH-PD-020]** Automatic Queue Management:
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

**[ARCH-PD-030]** Key Operations:
- Determine target time for selection (provided in request from Audio Player)
- Filter to non-zero probability passages (passages with one or more songs only)
- Calculate squared Euclidean distance from target flavor
- Sort by distance, take top 100 candidates
- Weighted random selection from candidates
- Handle edge cases (no candidates → return error status)

**[ARCH-PD-040]** Key Design Notes:
- **Request-based, not polling**: Audio Player initiates refill requests
- **Operates independently**: Does not require User Interface to be running
- **May enqueue proactively**: Free to enqueue passages without requests (like users via UI)
- **Database access**: Direct SQLite access for passage metadata, timeslots, probabilities, play history

> **See [Program Director](SPEC005-program_director.md) for complete specification of selection algorithm, cooldown system, probability calculations, and timeslot handling.**

---

### Audio File Ingest

**[ARCH-COMP-050]** Audio File Ingest Module Specification:

**Process Type**: Polished HTTP server with guided workflow UI
**Port**: 5723 (configurable)
**Versions**: Full only

**[ARCH-LM-010]** Responsibilities:
- Present user-friendly interface for adding new audio files
- Guide user through ingest and characterization workflow
- Coordinate MusicBrainz/AcousticBrainz lookups
- Manage Essentia local flavor analysis
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

**[ARCH-LM-020]** Key Design Notes:
- **Full version only**: Not included in Lite or Minimal
- **On-demand invocation**: The module is launched only during ingest sessions, typically when a user initiates a new audio import via the User Interface.
- **Database access**: Direct SQLite access for file/passage/song insertion
- **External API integration**: MusicBrainz, AcousticBrainz, Chromaprint+AcoustID
- **Local analysis**: Essentia integration for offline flavor characterization

> **See [Library Management](SPEC008-library_management.md) for complete file scanning and metadata workflows.**

---

### Internal Components

The modules listed above are separate processes. Within each module, there are internal components that handle specific responsibilities. These are implementation details within each module:

**Audio Player Internal Components:**
- **Queue Manager**: Maintains playback queue, handles manual additions/removals, monitors queue levels, requests refills from Program Director
- **Queue Monitor**: Calculates remaining queue time, sends `POST /selection/request` to Program Director when below thresholds (< 2 passages or < 15 minutes), throttles requests to once per 10 seconds
- **Playback Controller**: Coordinates passage transitions, manages crossfade timing based on lead-in/lead-out points
- **Audio Decoder**: Decodes audio files using symphonia, handles sample rate conversion with rubato, fills PCM buffers
- **Crossfade Mixer**: Sample-accurate mixing engine with automatic fade curve application (5 curve types supported)
- **Audio Output**: cpal-based audio output with ring buffer management, handles platform-specific audio backends
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

**Audio File Ingest Internal Components:**
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
- **Chromaprint Client**: Song identification profiler
- **AcoustID Client**: Translates Chromaprint profiles to MusicBrainz recording MBIDs

---

## Inter-Process Communication

### HTTP/REST APIs

**[ARCH-COM-010]** Primary communication method between modules.

**[ARCH-COM-020]** Benefits:
- **Platform-independent**: Language-agnostic interfaces
- **Well-defined contracts**: Clear API boundaries between modules
- **Easy debugging**: Standard HTTP tools (curl, Postman) for testing
- **Independent deployment**: Modules can be updated separately
- **Local-only deployment**: All modules must run on same machine (require local SQLite database access)

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

**[ARCH-COM-030]** Real-time notification method from modules to clients for inter-process state synchronization.

**[ARCH-COM-040]** State Synchronization Role:
- **Primary mechanism** for keeping process state synchronized beyond database
- **Real-time updates** ensure all processes have current state without polling
- **Event-driven architecture** reduces coupling between processes
- **Automatic recovery**: SSE reconnects automatically if connection drops

**Event Flows:**
- Audio Player → User Interface: Playback state, queue changes, position updates, song changes
- Audio Player → Program Director: passage selection requests
- User Interface → Audio Player: user passage selections, volume changes, play/pause changes
- User Interface → Program Director: user program changes
- Program Director → Audio Player: passage selection events
- Program Director → User Interface: timed program changes
- Each module provides `/events` endpoint for SSE subscriptions

**Benefits:**
- **One-directional push**: Server-to-client notifications
- **Lightweight**: Built on HTTP, auto-reconnect
- **Multi-subscriber**: Multiple UIs can subscribe to same events
- **Loose coupling**: Event producers don't need to know consumers

### Database as Shared State

**[ARCH-DATA-010]** SQLite database serves as persistent shared state across all modules.

**[ARCH-DATA-020]** Access Patterns:
- Each module has direct SQLite access (embedded database, same file)
- Coordinated writes via HTTP API boundaries
- Read-heavy access for passage metadata, library browsing
- Triggers maintain consistency (e.g., last_played_at updates)
- **Module discovery**: Each module reads `module_config` table on startup to determine:
  - Its own binding address and port
  - Other modules' addresses for HTTP communication

**[ARCH-DATA-030]** Consistency Considerations:
- UUID primary keys enable database merging (Full → Lite → Minimal)
- Foreign key constraints maintain referential integrity
- Application-level coordination via HTTP APIs prevents conflicts
- Write serialization through module ownership (e.g., only Audio Player writes queue state)
- Centralized network configuration in `module_config` table eliminates config file synchronization issues

### Module Dependencies

```
User Interface
    ├── Depends on: Audio Player - optional, degrades gracefully
    ├── Depends on: Program Director - optional (not present in Minimal version)
    └── Depends on: SQLite database - required

Program Director
    ├── Depends on: Audio Player - required for enqueueing
    └── Depends on: SQLite database - required

Audio Player
    └── Depends on: SQLite database - required

Audio File Ingest (Full only)
    └── Depends on: SQLite database - required

Lyric Editor (Full only)
    └── Depends on: SQLite database - required
```

**[ARCH-INT-010]** Service Launching:
- **Only wkmp-ui has a system service file**: Users configure their OS to auto-start wkmp-ui (systemd, launchd, Windows Service)
- **wkmp-ui is the primary entry point**: Launches other modules as needed (wkmp-ap, wkmp-pd, wkmp-ai, wkmp-le)
- **Modules can launch other modules**: Any module can launch peer modules if it detects they're not running
  - Example: wkmp-ap can relaunch wkmp-pd if it's not responding to queue refill requests
  - Example: wkmp-ui can launch wkmp-le when the user wants to edit lyrics
  - Example: wkmp-ui can launch wkmp-ap when the Playing mode is engaged, but wkmp-ap is not responsive
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
    - **[ARCH-INIT-005]** Module Health Check Strategy:
      - Current implementation: Basic health check (HTTP 200 = alive, 503 = not ready)
      - Initial startup: Any HTTP response (even error codes) indicates service is alive
      - Launch detection: Service considered ready when HTTP server responds
      - Future enhancement: Detailed health checks (database connectivity, audio device availability, audio subsystem status)
      - API structure: See api_design.md `/health` endpoint (reserves fields for future detailed diagnostics)
      - Traceability: REQ-NF-041, REQ-NF-042, REQ-NF-043, REQ-NF-044 (operational monitoring)
    - **Health check**: Send HTTP GET request to module's `/health` endpoint
      - Expected response: HTTP 200 with JSON `{"status": "healthy", "module": "<module-name>"}`
      - Timeout: 2 seconds for health check response
    - **Subprocess spawning**: If health check fails (timeout, connection refused, or non-200 response):
      - Parent process spawns child subprocess using system ProcessBuilder/spawn mechanism
      - Pass `--binary-path` argument if received by launching module
      - Get process handle for monitoring and potential future termination
    - **Crash detection and respawning**:
      - When a process needs another process for any reason (e.g., to present another interface to the user)
      - First checks health via `/health` endpoint
      - If non-responsive, initiates respawn procedure with rate limiting
      - Respawn rate: Configurable via settings table `relaunch_delay` (default: 5000 milliseconds)
      - Retry count limit: Configurable via settings table `relaunch_attempts` (default: 20)
      - Respawn retries are equal-time spaced (wait `relaunch_delay` between each attempt)
      - When retries exhausted: User notification displayed with manual retry option
    - **[ARCH-INT-020]** Duplicate instance detection:
      - **Lock-file based IPC strategy**: Each module creates a lock file on startup
      - Lock file location: `{root_folder}/.wkmp/locks/{module_name}.lock` (e.g., `wkmp-ap.lock`)
      - Lock file contains: PID, port number, start timestamp
      - On startup, module checks if lock file exists:
        - If exists: Read PID and check if process is still running
        - If process running: Attempt health check on specified port
        - If health check succeeds: Exit with error "Instance already running"
        - If health check fails or process not running: Remove stale lock file and proceed
      - On shutdown: Module removes its lock file
      - This prevents multiple instances when module is healthy but network issues prevent health check
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
     - Linux: `~/.config/wkmp/<module-name>.toml` or `/etc/wkmp/<module-name>.toml`
     - macOS: `~/Library/Application Support/WKMP/<module-name>.toml`
     - Windows: `%APPDATA%\WKMP\<module-name>.toml`
   - When present and higher-priority sources are absent, this value is used
   - **[REQ-NF-031]** If config file is missing, module SHALL NOT error - proceeds to compiled default
4. **OS-dependent compiled default** (lowest priority, graceful fallback):
   - **[REQ-NF-033]** Default locations in user's Music folder:
     - Linux: `~/Music`
     - macOS: `~/Music`
     - Windows: `%USERPROFILE%\Music`
   - Used when no other source provides a value
   - Ensures module can always start with a valid root folder path
   - **[REQ-NF-036]** System automatically creates directory if missing

**Module Startup Sequence:**

**[ARCH-INIT-010]** Each module follows this initialization sequence:

1. **Resolve root folder path** using resolution priority order above [ARCH-INIT-005]
   - **[REQ-NF-032]** If TOML config file is missing: Log warning, use compiled default
   - No error termination for missing config files
2. **Verify/create root folder directory** [REQ-NF-036]
   - If directory does not exist, create it automatically
   - Log informational message about directory creation
3. **Open database file** (`wkmp.db`) in root folder
   - **[REQ-NF-036]** If database does not exist, create empty database with default schema
   - Log informational message about database initialization
4. **Initialize database tables** using shared initialization functions from `wkmp-common`:
   - Commonly used tables: `module_config`, `settings`, `users` (via shared functions in `wkmp-common/src/db/init.rs`)
   - Module-specific tables: Created directly by each module (e.g., `queue` for Audio Player, `timeslots` for Program Director)
   - All initialization is idempotent (safe to call multiple times, checks if table exists before creating)
5. **Read module configuration** from `module_config` table using shared config loader from `wkmp-common`:
   - Shared config loader calls `init_module_config_table()` if table missing
   - If own module's config is missing, inserts default host/port and logs warning
   - If other required modules' configs are missing, inserts defaults and logs warnings
   - Default configurations: user_interface (127.0.0.1:5720), audio_player (127.0.0.1:5721), program_director (127.0.0.1:5722), audio_ingest (0.0.0.0:5723), lyric_editor (0.0.0.0:5724)
6. **Retrieve own host and port** configuration
7. **Bind to configured address/port**
8. **Retrieve other modules' configurations** for HTTP client setup (if needed for making requests to peer modules)
9. **Begin accepting connections** and making requests to peer modules

**Graceful Degradation Behavior:**

**[ARCH-INIT-015]** Missing configuration handling [REQ-NF-031, REQ-NF-032]:

- **Missing TOML config file**: Log warning, use compiled defaults, proceed with startup
  - Example: `WARN: Config file not found at ~/.config/wkmp/audio-player.toml, using defaults`
- **Missing root folder directory**: Create directory automatically, log info message
  - Example: `INFO: Created root folder directory: ~/Music`
- **Missing database file**: Create new database with default schema, log info message
  - Example: `INFO: Initialized new database: ~/Music/wkmp.db`
- **Missing database tables**: Create tables automatically via idempotent initialization functions
- **Missing settings values**: Insert compiled defaults into database
- **Result**: Module starts successfully with default configuration, no user intervention required

**Default Value Initialization Behavior:**

**[ARCH-INIT-020]** When the application reads a configuration value from the database, it SHALL handle missing, NULL, or nonexistent values according to the following rules:

1. **Database Does Not Exist**:
   - Module creates database file and all tables required by the module
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
   - See [Database Schema - settings table](IMPL001-database_schema.md#settings) for complete list of settings and their defaults

**Rationale:**

- **Database is source of truth**: All runtime configuration lives in database, TOML files do not contain parameters that are found in the database
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

**[ARCH-INT-030]** Module Launch Responsibilities:
- **User Interface (wkmp-ui)**:
  - Launched by: OS service manager (systemd/launchd/Windows Service)
  - Launches: wkmp-ap (on startup), wkmp-pd (Lite/Full only), wkmp-ai (on-demand, Full only), wkmp-le (on-demand, Full only)
- **Audio Player (wkmp-ap)**:
  - Launched by: wkmp-ui on startup
  - Can launch: wkmp-pd (if not responding to queue refill requests), wkmp-ui (if needed)
- **Program Director (wkmp-pd)**:
  - Launched by: wkmp-ui on startup (Lite/Full versions only)
  - Can launch: wkmp-ap (if needed for enqueueing), wkmp-ui (if needed)
- **Audio File Ingest (wkmp-ai)**:
  - Launched by: wkmp-ui when user initiates audio file ingest (Full version only)
  - Can launch: wkmp-ui (if needed)
- **Lyric Editor (wkmp-le)**:
  - Launched by: wkmp-ui on-demand when user requests lyric editing (Full version only)
  - Can launch: wkmp-ui (if needed)

---

## Component Implementation Details

This architecture implements the requirements specified in [requirements.md](REQ001-requirements.md).

Detailed design specifications for each subsystem:
- **Crossfade System**: See [Crossfade Design](SPEC002-crossfade.md)
- **Musical Flavor System**: See [Musical Flavor](SPEC003-musical_flavor.md)
- **Event-Driven Communication**: See [Event System](SPEC011-event_system.md)
- **Data Model**: See [Database Schema](IMPL001-database_schema.md)
- **Project Structure**: See [Project Structure](IMPL003-project_structure.md)
- **Code Organization**: See [Coding Conventions](IMPL002-coding_conventions.md)

## Concurrency Model

### Per-Module Threading

**[ARCH-CONC-010]** Each module is an independent process with its own threading model:

**Audio Player:**
```
HTTP Server Thread Pool (tokio async):
  - API request handling
  - SSE broadcasting to clients

Audio Thread (cpal callback):
  - Audio output callback execution
  - Reads from ring buffer
  - Low-latency audio delivery
  - Isolated from blocking I/O

Mixer Thread (single-stream):
  - Reads decoded samples from PassageBuffers
  - Applies fade curves per-sample
  - Performs crossfade mixing
  - Fills ring buffer for audio thread

Queue Manager Thread (tokio async):
  - Queue persistence
  - Passage loading and decoding
  - Database queries

Decoder Thread Pool:
  - Decodes audio files using symphonia
  - Resamples using rubato
  - Fills PassageBuffers with PCM data
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

**Audio File Ingest:**
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
  - Chromaprint queries
  - AcoustID queries
  - Essentia local analysis
```

### Internal Communication Patterns

**[ARCH-CONC-020]** Within each module, components use standard Rust async patterns:

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

> **See [Event System](SPEC011-event_system.md) for complete event-driven architecture specification within modules.**

## Data Model

WKMP uses SQLite with UUID-based primary keys for all entities. The complete schema includes:

**Core Entities:** files, passages, songs, artists, works, albums
**Relationships:** passage_songs, passage_albums, song_works
**Playback:** play_history, likes_dislikes, queue
**Configuration:** module_config, timeslots, timeslot_passages, settings
**Caching:** acoustid_cache, musicbrainz_cache, acousticbrainz_cache

See [Database Schema](IMPL001-database_schema.md) for complete table definitions, constraints, indexes, and triggers.

### Key Design Decisions

**[ARCH-DES-010]** Key architectural decisions:

- **UUID primary keys**: Enable database merging across Full/Lite/Minimal versions
- **Musical flavor vectors**: Stored as JSON in `passages.musical_flavor_vector` for flexibility and SQLite JSON1 integration
- **Automatic triggers**: Update `last_played_at` timestamps on playback for cooldown calculations
- **Foreign key cascades**: Simplify cleanup when files/passages deleted
- **No binary blobs**: Album art stored as files (in root folder tree), database stores relative paths only
- **Event-driven architecture**: Uses `tokio::broadcast` for one-to-many event distribution, avoiding tight coupling between components while staying idiomatic to Rust async ecosystem. See [Event System](SPEC011-event_system.md) for details.
- **Hybrid communication**: Events for notifications, channels for commands, shared state for readsâ€”each pattern chosen for specific use cases

## Version Differentiation

WKMP is built in three versions (Full, Lite, Minimal) by **packaging different combinations of modules**. See [Requirements - Three Versions](REQ001-requirements.md#three-versions) for detailed feature comparison and resource profiles.

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
- See [Implementation Order - Version Packaging](EXEC001-implementation_order.md#phase-9-version-packaging--module-integration-25-weeks) for packaging details

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
- Audio decoding: symphonia 0.5.x (primarily pure Rust)
  - Pure Rust codecs: MP3, FLAC, AAC (M4A), Vorbis (OGG), WAV (PCM), ADPCM
  - FFI-based codecs: Opus (via symphonia-adapter-libopus + libopus C library)
  - [REQ-TECH-022A]: Opus exception approved for C library integration (see REV003)
- Sample rate conversion: rubato 0.15.x (high-quality resampling)
- Audio output: cpal 0.15.x (cross-platform, supports PulseAudio, ALSA, CoreAudio, WASAPI)
- Crossfading: Custom single-stream implementation with sample-accurate mixing

**Database:**
- SQLite 3.x (embedded in each module)
- rusqlite crate for Rust bindings
- JSON1 extension for flavor vector storage

**External API Clients:**
- reqwest for HTTP clients
- MusicBrainz, AcousticBrainz, AcoustID

**Local Audio Analysis (Audio File Ingest, Full version only):**
- Essentia C++ library
- Chromaprint C library
- Rust FFI bindings (custom or via existing crate)

**Web UI (User Interface and Audio File Ingest):**
- HTML/CSS/JavaScript (framework TBD - React, Vue, or Svelte)
- SSE client for real-time updates
- Responsive design framework (TailwindCSS or similar)

**Configuration:**
- **Database first**: ALL runtime settings stored in database (`settings` and `module_config` tables)
- **TOML files**: Bootstrap configuration ONLY (root folder path, logging, static asset paths)
- **Default value initialization**: When database settings are missing/NULL, application initializes with built-in defaults and writes to database
- Database and all files contained in root folder tree for portability

**Build System:**
- Cargo workspaces for multi-module project (see [Project Structure](IMPL003-project_structure.md))
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

**[ARCH-PLAT-010]** Audio output platform abstraction:
```
┌──────────────────────┐
│  Platform Detector   │
│  (Runtime detection) │
└──────────┬───────────┘
           │
    ┌──────"´──────┬──────────┬──────────┐
    │             │          │          │
┌───▼────┐  ┌────▼────┐ ┌───▼────┐ ┌───▼────┐
│ ALSA   │  │PulseAudio│ │CoreAudio│ │WASAPI │
│(Linux) │  │ (Linux) │ │ (macOS) │ │(Windows)│
└────────┘  └─────────┘ └────────┘ └────────┘
```

**[ARCH-AUDV-020]** Auto-detection Priority:
- Linux: PulseAudio → ALSA (Phase 1)
- Windows: WASAPI (Phase 1)
- macOS: CoreAudio (Phase 2)

**[ARCH-AUDV-030]** Manual Override:
- User can select specific sink
- User can choose specific output device
- Settings persisted in database

### System Integration

**[ARCH-INT-040]** Auto-start:
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

**[ARCH-SEC-010]** Local Network Access:
- HTTP only on `localhost:5720`
- Binds to localhost on startup (critical failure if port unavailable)
- Accessible via:
  - Localhost: `http://localhost:5720` (no network required)
  - Local network: `http://<machine-ip>:5720` (requires local network)
  - Remote: User must configure port forwarding (not recommended)

**[ARCH-SEC-020]** Authentication:
- User authentication system with three modes:
  - Anonymous access (shared UUID, no password)
  - Account creation (unique UUID, username/password)
  - Account login (existing credentials)
- Session persistence via browser localStorage (one-year rolling expiration)

**[ARCH-SEC-030]** Security:
- CORS restricted to localhost (except Lyric Editor)
- No external network exposure by default
- User responsible for network security if exposing to local network
- No internet access required for WebUI operation (local network only)

### Database

**[ARCH-SEC-040]** Database security:
- SQLite with file permissions (user-only read/write)
- Passwords stored as salted hashes (never plaintext)
- Salt incorporates user UUID for additional security
- Relative file paths only (no file contents stored in database)
- All paths relative to root folder for portability
- User taste data (likes/dislikes) considered non-sensitive
- Anonymous user data is shared across all anonymous sessions

### Internet Access (External APIs)

**Purpose:**
- AcoustID recording identification
- MusicBrainz metadata lookup
- AcousticBrainz flavor data retrieval
- Cover art fetching

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
- **Lite version**: Internet not required for core functionality
  - *Future Enhancement*: News and Weather features may utilize internet when available (not part of initial implementation)
- **Minimal version**: Internet not required

**Security:**
- HTTPS for all external requests
- API keys in environment variables (if required)
- Rate limiting to respect service terms
- Offline fallback for all features

## Performance Targets

**[ARCH-PERF-010]** Raspberry Pi Zero2W (Lite/Minimal):
- Startup time: < 5 seconds
- Memory usage: < 256MB
- Selection time: < 500ms for 10k passage library
- Crossfade latency: < 50ms gap
- UI responsiveness: < 100ms for user actions

**[ARCH-PERF-020]** Desktop (Full):
- Startup time: < 3 seconds
- Memory usage: < 512MB
- Essentia analysis: < 30 seconds per passage
- Concurrent scan: 100+ files/second
- Selection time: < 200ms for 100k passage library

## Error Handling Strategy

### Categories

**[ARCH-CAT-010]** Recoverable Errors:
- Network failures → Retry with fixed 5-second delay (see Network Error Handling below)
- Missing files → Skip, remove from queue, log
- Database lock → Retry with exponential backoff (see Database Lock Timeout below)
- Decode errors → Skip to next passage (see Audio Playback Errors below)
- Program Director timeout → Continue with existing queue, retry on next threshold

### Error Recovery Strategies

This section specifies the detailed recovery procedures for common error scenarios in wkmp-ap.

#### Audio Playback Errors

**[ARCH-ERRH-010]** Audio Playback Error Recovery:

When an audio playback error occurs (file not found, decode failure using symphonia, audio device unavailable via cpal, etc.), the following recovery procedure is executed:

1. **Log error** with playback state and error details
2. **Handle as skip event**: From this point, any playback failure is treated identically to a user-initiated skip:
   - Emit `PassageCompleted(completed=false)` event with appropriate reason:
     - `reason: "playback_error"` if decode or audio output failure
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

**[ARCH-ERRH-020]** Crossfade Behavior During Pipeline Error:

- If error occurs during crossfade (both pipelines active):
  - Failed pipeline stops immediately
  - Surviving pipeline continues playing without interruption
  - No fade adjustment applied (surviving pipeline maintains its current fade curve)
- If error occurs in idle pipeline (pre-loading next passage):
  - Current pipeline continues playing normally
  - Failed pre-load logged as error
  - Next passage skip logic applies when current passage completes

**[ARCH-ERRH-030]** Automatic Queue Refill Throttling (wkmp-pd responsibility):

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

**[ARCH-ERRH-050]** Database Lock Retry Strategy:

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

**[ARCH-ERRH-060]** Cached State Fallback:

Operations that can use cached state on lock timeout:
- Queue read: Use last successfully read queue (may be stale)
- Settings read: Use last successfully read settings
- Module config read: Use last known configuration

Operations that **cannot** use cached state (require retry or failure):
- Queue write: Must succeed or return error
- Settings write: Must succeed or return error
- Playback position persistence: Failure is acceptable (position lost on crash)

**[ARCH-ERRH-070]** Lock Timeout Configuration:

SQLite busy timeout is set to 5000ms (5 seconds) at connection initialization:
```rust
connection.busy_timeout(Duration::from_millis(5000))?;
```

This timeout applies **before** the exponential backoff retry logic. The exponential backoff provides additional resilience for transient lock contention.

#### Program Director Timeout

**[ARCH-ERRH-100]** Program Director Timeout Handling:

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

**[ARCH-ERRH-110]** Request Throttling:

To prevent request spam during wkmp-pd unavailability:
- Minimum interval between refill requests: `queue_refill_request_throttle_seconds` (default: 10 seconds)
- Throttle applies even after timeout
- If queue drops below threshold during throttle period: Wait until throttle expires, then send request

**[ARCH-ERRH-120]** Empty Queue Behavior:

If queue becomes empty while wkmp-pd is unresponsive:
- Audio player enters idle state (no audio output)
- Continues attempting refill requests at throttle interval
- Resumes playback automatically when wkmp-pd responds with passage

**[ARCH-ERRH-130]** Module Health Monitoring (wkmp-ui responsibility):

**Note:** wkmp-ui (User Interface) is responsible for detecting unresponsive modules and relaunching them. wkmp-ap only logs timeouts and continues operation.

### Network Error Handling

**[ARCH-ERRH-150]** WKMP requires two distinct types of network access with different error handling strategies:

**Internet Access (External APIs - Full version only):**

Used for:
- MusicBrainz metadata lookup during library import
- AcousticBrainz musical flavor data retrieval
- Cover art fetching

**[ARCH-ERRH-160]** Retry Algorithm:
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
- Total duration: 20 retries Ã— 5s = 100 seconds maximum before requiring user intervention

**Example retry sequence:**
- Attempt 1: Fail → wait 5s
- Attempt 2: Fail → wait 5s
- Attempt 3: Fail → wait 5s
- ...
- Attempt 20: Fail → stop, display "Connection Failed" message

**[ARCH-ERRH-170]** Playback Impact:
- No impact on playback: Music continues playing during internet outages
- Playback uses only local database and audio files (no internet required)

**Local Network Access (WebUI Server):**

Used for:
- Serving WebUI on `http://localhost:5720`
- Server-Sent Events (SSE) for real-time UI updates
- REST API endpoints for playback control

**[ARCH-ERRH-180]** Error Handling:
- HTTP server binds to localhost:5720 on startup
- If port binding fails: Log error and exit (critical failure)
- Once running, server continues indefinitely

**[ARCH-ERRH-190]** Access Requirements:
- Localhost access: Always available (no network required)
- Remote access: Requires local network connectivity
  - User responsible for network configuration (router, firewall, etc.)
  - No internet required (local network only)

**[ARCH-ERRH-200]** Playback Impact:
- Automatic playback: Works without any network access
  - System auto-starts on boot
  - Auto-selects and plays passages
  - No WebUI access needed for basic operation
- Manual control: Requires WebUI access (localhost or remote)

> **See:** [UI Specification - Network Status Indicators](SPEC009-ui_specification.md#network-status-indicators) for user-facing status display
> **See:** [Requirements - Network Error Handling](REQ001-requirements.md#network-error-handling) for complete requirements

**[ARCH-CAT-020]** Non-recoverable Errors:
- Database corruption → Alert user, attempt repair
- Configuration errors → Reset to defaults, warn user
- Critical audio system failures → Restart audio output subsystem

### Logging

**[ARCH-LOG-010]** Levels:
- ERROR: System failures, data corruption
- WARN: Recoverable issues, missing data
- INFO: State changes, significant events
- DEBUG: Detailed operation info
- TRACE: Fine-grained execution flow

**[ARCH-LOG-020]** Output:
- stdout/stderr with timestamps
- File rotation (max 10MB per file, keep 5)
- Structured logging with `tracing` crate
- File:line identification in all messages

## Deployment

### Database Migrations

**[ARCH-MIG-010]** Migration strategy:
- Version tracking in `schema_version` table
- Forward-only migrations
- Automatic on startup (with backup)
- Rollback support for critical failures

### Configuration

**[ARCH-CONF-010]** Configuration sources:
- Environment variables for system paths
- SQLite `module_config` table for network configuration (host/port for each module)
- SQLite `settings` table for user preferences
- File-based config for deployment settings which are not found in the database settings table (root folder path, logging configuration)
- Sane defaults for all optional settings
- Centralized network configuration eliminates the need to duplicate module URLs across config files

### Distribution

**[ARCH-DIST-010]** Distribution packaging:
- **Multiple binaries per version**: Each module is a separate binary
- **Version-specific packaging**:
  - Full: 5 binaries (Audio Player, User Interface, Lyric Editor, Program Director, Audio File Ingest)
  - Lite: 3 binaries (Audio Player, User Interface, Program Director)
  - Minimal: 2 binaries (Audio Player, User Interface)
- **Bundled dependencies**: SQLite (all modules)
- **Installer packages**: deb, rpm, msi, dmg with systemd/launchd service files
- **Process management**: System service manager or manual startup scripts
- **Configuration files**: Default ports, module URLs, root folder path

## Future Architecture Considerations

**[ARCH-FUT-010]** Scalability:
- Current design: single-user, local database
- Future: Multi-user with centralized database
- Future: Cloud sync for preferences/history
- Future: Collaborative playlists and flavor sharing

**[ARCH-FUT-020]** Mobile (Flutter Rewrite):
- Shared Rust core via FFI
- Flutter UI layer
- Platform-specific audio engines
- Background playback support
- Offline-first architecture

**[ARCH-FUT-030]** Advanced Features:
- Machine learning for preference inference
- Real-time collaborative listening
- Plugin system for custom selectors
- External player control protocols (MPD, etc.)

----
End of document - WKMP Architecture

**Document Version:** 1.1
**Last Updated:** 2025-10-17

**Change Log:**
- v1.1 (2025-10-17): Updated CurrentSongChanged event and health check specifications
  - Updated [ARCH-SNGC-030] to clarify passage_id is always present during playback
  - Added ephemeral passage support to CurrentSongChanged event specification
  - Updated event payload structure to reflect passage_id as non-optional (PassageId not Option<PassageId>)
  - Added [ARCH-INIT-005] Module Health Check Strategy specification
  - Clarified basic health check (initial) vs. detailed health check (future enhancement)
  - Supports architectural decisions from wkmp-ap design review (ISSUE-4, ISSUE-9)
