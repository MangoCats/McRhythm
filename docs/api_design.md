# WKMP API Design

**🌐 TIER 2 - DESIGN SPECIFICATION**

Defines REST API structure and Server-Sent Events interface. Derived from [requirements.md](requirements.md). See [Document Hierarchy](document_hierarchy.md).

> **Related Documentation:** [Requirements](requirements.md) | [Architecture](architecture.md) | [Event System](event_system.md)

---

## Overview

WKMP implements a **microservices architecture** with 5 independent HTTP servers, each exposing its own REST API and SSE endpoints. Modules communicate via HTTP APIs and share a common SQLite database.

### Module API Endpoints

| Module | Default Port | Base URL | Purpose |
|--------|--------------|----------|---------|
| **Audio Player** | 5721 | `http://localhost:5721` | Playback control, queue management |
| **User Interface** | 5720 | `http://localhost:5720/api` | User-facing API, authentication, library browsing |
| **Program Director** | 5722 | `http://localhost:5722` | Selection configuration, timeslots |
| **Audio Ingest** | 5723 | `http://localhost:5723` | File scanning, ingest workflow (Full only) |
| **Lyric Editor** | 5724 | `http://localhost:5724` | Lyric editing interface (Full only, launched on-demand) |

### API Communication Patterns

**End Users → User Interface:**
- User Interface serves as the primary API gateway for end users
- Proxies playback requests to Audio Player
- Proxies configuration requests to Program Director
- Handles authentication and session management
- Aggregates SSE events from Audio Player

**Program Director → Audio Player:**
- Direct communication for automatic enqueueing
- No user interface involvement required

**Audio Ingest → Database:**
- Direct SQLite access for new file insertion
- Independent operation

### Authentication

**User Interface API** handles all authentication:
1. Proceed as Anonymous user (shared UUID, no password)
2. Create new account (generates unique UUID, requires username/password)
3. Login to existing account (retrieves UUID, requires username/password)

Once authenticated, the browser stores the user UUID in localStorage with a rolling one-year expiration. See [User Identity and Authentication](user_identity.md) for complete flow.

**Internal Module APIs** (Audio Player, Program Director, Audio Ingest, Lyric Editor):
- No authentication required (assumed to be on trusted local network)
- Minimal HTML/JavaScript developer UIs for debugging (served via HTTP)
- Security relies on network isolation
- Lyric Editor is launched on-demand by User Interface when needed

**Content-Type:** `application/json` for all request/response bodies across all modules

---

## User Interface API

**Base URL:** `http://localhost:5720/api`
**Port:** 5720 (configurable)
**Purpose:** Primary API for end users, handles authentication, proxies to other modules

### Authentication Endpoints

These endpoints establish user identity and return a UUID for subsequent requests. All users must authenticate through one of these methods before accessing other endpoints.

### `POST /api/login`

Authenticate a user and retrieve their UUID.

**Request:**
```json
{
  "username": "string",
  "password": "string"
}
```

**Response (Success):**
```json
{
  "status": "ok",
  "user_id": "uuid"
}
```

**Response (Failure):**
```json
{
  "error": "invalid_credentials",
  "message": "Invalid username or password"
}
```

### `POST /api/register`

Create a new user account.

**Request:**
```json
{
  "username": "string",
  "password": "string"
}
```

**Response (Success):**
```json
{
  "status": "ok",
  "user_id": "uuid"
}
```

**Response (Failure):**
```json
{
  "error": "username_exists",
  "message": "Username is already taken"
}
```

### `POST /api/logout`

Log out the current user. This would invalidate the client-side token/UUID.

**Request:** Empty body

**Response:**
```json
{
  "status": "ok"
}
```

### `GET /api/current-user`

Retrieve information about the currently authenticated user.

**Request:** Includes user UUID from localStorage

**Response:**
```json
{
  "user_id": "uuid",
  "username": "string",
  "is_anonymous": false
}
```

**Response (Anonymous user):**
```json
{
  "user_id": "00000000-0000-0000-0000-000000000001",
  "username": "Anonymous",
  "is_anonymous": true
}
```
*Note: The Anonymous user GUID is fixed as defined in [Database Schema](database_schema.md#users)*

### Playback Control Endpoints (Proxied to Audio Player)

**Note:** These endpoints are exposed by User Interface at `/api/playback/*` and proxied to Audio Player's HTTP API. All require user authentication (UUID from localStorage).

#### `GET /api/status`

Get current playback state.

**Response:**
```json
{
  "state": "playing" | "paused",
  "passage": {
    "id": "uuid",
    "title": "string",
    "artist": "string",
    "album": "string",
    "duration": 180.5
  },
  "position": 42.3,
  "volume": 75,
  "queue_length": 3
}
```

**Response when queue is empty:**
```json
{
  "state": "playing" | "paused",
  "passage": null,
  "position": 0.0,
  "volume": 75,
  "queue_length": 0
}
```

**Notes:**
- System has two states: "playing" or "paused" (no "stopped" state)
- Initial state on app launch: "playing" (always resumes to playing)
- `state` reflects user-selected Play/Pause mode, independent of queue state
- When queue is empty, `passage` is `null` but `state` remains as user set it
- System in "playing" state with empty queue produces no audio output
- Enqueueing a passage while in "playing" state begins playback immediately
- Enqueueing a passage while in "paused" state queues it without starting playback
- State NOT persisted across app restarts - see [Architecture](architecture.md#state-persistence) for details

#### `POST /api/play`

Start playback of current passage.

**Request:** Empty body

**Response:**
```json
{
  "status": "ok"
}
```

#### `POST /api/pause`

Pause playback (maintain position).

**Request:** Empty body

**Response:**
```json
{
  "status": "ok"
}
```

#### `POST /api/skip`

Skip to next passage in queue.

**Request:** Empty body

**Response:**
```json
{
  "status": "ok"
}
```

**Edge Case:** Skip requests within 5 seconds of a previous skip are ignored. See [Multi-User Coordination](multi_user_coordination.md#1-skip-throttling) for details.

#### `POST /api/volume`

Set master volume level.

**Request:**
```json
{
  "level": 75
}
```

**Parameters:**
- `level`: Integer 0-100 (percentage)

**Response:**
```json
{
  "status": "ok",
  "volume": 75
}
```

#### `POST /api/seek`

Seek to a specific position within the currently playing passage. Allows users to rewind or fast-forward.

**Request:**
```json
{
  "position": 60.5
}
```

**Parameters:**
- `position`: Float, seconds from passage start
  - Valid range: `[0, passage_duration]`
  - Seeking to `passage_duration` has the same effect as skip

**Response:**
```json
{
  "status": "ok",
  "position": 60.5
}
```

**Notes:**
- Seek only applies to the current passage
- Position is clamped to valid range if out of bounds
- During crossfade, seek applies to the currently playing (fading out) passage
- The crossfading (next) passage seeks along with the current passage, adjusting by the same time offset to maintain crossfade synchronization

### Queue Management

#### `GET /api/queue`

Get upcoming passages in queue.

**Response:**
```json
{
  "queue": [
    {
      "id": "uuid",
      "title": "string",
      "artist": "string",
      "duration": 180.5,
      "position": 0
    },
    ...
  ]
}
```

#### `POST /api/enqueue`

Add passage to end of queue.

**Request:**
```json
{
  "passage_id": "uuid"
}
```

**Response:**
```json
{
  "status": "ok",
  "queue_position": 3
}
```

**Note:** User-enqueued passages may have zero songs (manual selection only)

#### `POST /api/remove`

Remove passage from queue.

**Request:**
```json
{
  "passage_id": "uuid"
}
```

**Response:**
```json
{
  "status": "ok"
}
```

**Edge Case:** Multiple concurrent remove requests for the same passage are handled gracefully. See [Multi-User Coordination](multi_user_coordination.md#2-concurrent-queue-removal) for details.

### User Feedback (Full and Lite versions only)

#### `POST /api/like`

Record a like for the currently playing passage.

**Request:** Empty body (user UUID automatically included from session)

**Response:**
```json
{
  "status": "ok",
  "passage_id": "uuid",
  "user_id": "uuid",
  "timestamp": "2025-10-06T14:23:45Z"
}
```

**Notes:**
- Like is recorded against the authenticated user's UUID
- Anonymous users share likes (all recorded to same Anonymous UUID)
- Used to build user-specific taste profile for passage selection

#### `POST /api/dislike`

Record a dislike for the currently playing passage.

**Request:** Empty body (user UUID automatically included from session)

**Response:**
```json
{
  "status": "ok",
  "passage_id": "uuid",
  "user_id": "uuid",
  "timestamp": "2025-10-06T14:23:45Z"
}
```

**Notes:**
- Dislike is recorded against the authenticated user's UUID
- Anonymous users share dislikes (all recorded to same Anonymous UUID)
- Used to build user-specific taste profile for passage selection

### Lyrics

#### `GET /api/lyrics/:song_guid`

Retrieve lyrics for a song with fallback chain (read-only proxy to database).

**Parameters:**
- `song_guid`: Song GUID (UUID) in URL path

**Response:**
```json
{
  "song_guid": "abc123...",
  "lyrics": "string (plain UTF-8 text, may contain newlines)",
  "lyrics_source_song_guid": "abc123...",
  "last_modified": "2025-10-09T12:34:56Z"
}
```

**Response (no lyrics):**
```json
{
  "song_guid": "abc123...",
  "lyrics": null,
  "lyrics_source_song_guid": null,
  "last_modified": null
}
```

**Lyrics Fallback Logic:**
1. Check the specified Song's `lyrics` field
2. If empty, iterate through the Song's `related_songs` array (ordered from most to least closely related)
3. Return lyrics from the first related Song with a non-empty `lyrics` field
4. If no Song in the chain has lyrics, return null
5. The `lyrics_source_song_guid` indicates which Song's lyrics are being displayed

**Note:** User Interface provides read-only access to lyrics. All editing is done via the Lyric Editor module (Full version only).

#### `POST /api/lyrics/edit/:song_guid`

Launch the Lyric Editor for a specific song (Full version only).

**Parameters:**
- `song_guid`: Song GUID (UUID) in URL path

**Request:**
```json
{
  "recording_mbid": "5e46c5b4-7f91-4d86-a97e-5a3c8e3f5c4f",
  "title": "Bohemian Rhapsody",
  "artist": "Queen"
}
```

**Response:**
```json
{
  "status": "ok",
  "message": "Lyric editor launched"
}
```

**Behavior:**
- Checks if wkmp-le is running on configured port (default: 5724)
- Launches wkmp-le if not running (Full version only)
- Forwards song_guid, recording_mbid, title, and artist to `POST /lyric_editor/open`
- Returns success/failure status

**Edge Case:** Concurrent lyric submissions are handled via a "last write wins" strategy in the Lyric Editor. See [Multi-User Coordination](multi_user_coordination.md#3-concurrent-lyric-editing) for details.

### Library Management (Full version only)

#### `POST /api/import`

Trigger library scan for new/changed audio files.

**Request:** Empty body or optional directory paths

**Request (with paths):**
```json
{
  "paths": ["albums/rock", "new_imports"]
}
```

**Note:**
- If no paths provided, scans the entire root folder
- Paths are relative to the root folder (e.g., "albums/rock" scans `{root_folder}/albums/rock`)
- All discovered audio files must be within the root folder tree
- File paths are stored in the database relative to the root folder for portability

**Response:**
```json
{
  "status": "ok",
  "scan_id": "uuid"
}
```

**Note:** Scan runs asynchronously. Progress updates via SSE (LibraryScanCompleted event)

### Audio Output

#### `POST /api/output`

Select audio output device.

**Request:**
```json
{
  "sink": "auto" | "pulseaudio" | "alsa" | "coreaudio" | "wasapi",
  "device": "optional-device-id"
}
```

**Parameters:**
- `sink`: Audio sink type (auto-detect recommended)
- `device`: Optional specific device ID (platform-specific)

**Response:**
```json
{
  "status": "ok",
  "sink": "pulseaudio",
  "device": "alsa_output.pci-0000_00_1f.3.analog-stereo"
}
```

---

## Lyric Editor API

**Base URL:** `http://localhost:5724`
**Port:** 5724 (configurable)
**Purpose:** Dedicated lyric editing interface
**Authentication:** None (internal/trusted network only)
**Launch Model:** On-demand (started by User Interface when needed)

The Lyric Editor provides a standalone interface for editing song lyrics associated with MusicBrainz recordings. It displays a split window with a text editor on the left and an embedded web browser on the right to facilitate finding and copying lyrics from web sources.

### POST /lyric_editor/open

Opens the lyric editor window with a specific recording.

**Request Body:**
```json
{
  "recording_mbid": "5e46c5b4-7f91-4d86-a97e-5a3c8e3f5c4f",
  "title": "Bohemian Rhapsody",
  "artist": "Queen"
}
```

**Response:**
```json
{
  "status": "ok",
  "message": "Lyric editor opened"
}
```

**Behavior:**
- Opens split-window UI
- Left pane: Loads current lyrics from `songs.lyrics` (empty if none exist)
- Right pane: Launches embedded browser searching for "{title} {artist} lyrics"
- User can edit lyrics in left pane and browse for sources in right pane

### GET /lyric_editor/lyrics/:recording_mbid

Retrieves current lyrics for a recording.

**Response:**
```json
{
  "recording_mbid": "5e46c5b4-7f91-4d86-a97e-5a3c8e3f5c4f",
  "lyrics": "Is this the real life?\nIs this just fantasy?...",
  "last_modified": "2025-10-09T12:34:56Z"
}
```

### PUT /lyric_editor/lyrics/:recording_mbid

Saves edited lyrics to the database.

**Request Body:**
```json
{
  "lyrics": "Is this the real life?\nIs this just fantasy?..."
}
```

**Response:**
```json
{
  "status": "ok",
  "recording_mbid": "5e46c5b4-7f91-4d86-a97e-5a3c8e3f5c4f",
  "saved_at": "2025-10-09T12:35:00Z"
}
```

**Behavior:**
- Updates `songs.lyrics` column for the specified recording_mbid
- Uses last-write-wins concurrency (no locking)
- Emits `LyricsChanged` SSE event to notify other clients

### POST /lyric_editor/close

Closes the lyric editor without saving.

**Response:**
```json
{
  "status": "ok",
  "message": "Lyric editor closed"
}
```

### SSE Event Stream

**Endpoint:** `GET /events`

**Events:**
- `LyricsChanged` - Emitted when lyrics are saved
  ```json
  {
    "event": "LyricsChanged",
    "recording_mbid": "5e46c5b4-7f91-4d86-a97e-5a3c8e3f5c4f",
    "timestamp": "2025-10-09T12:35:00Z"
  }
  ```

---

## Audio Player API

**Base URL:** `http://localhost:5721`
**Port:** 5721 (configurable)
**Purpose:** Direct playback control, queue management
**Authentication:** None (internal/trusted network only)

**Note:** End users typically access these endpoints via User Interface, which proxies requests. Program Director calls these endpoints directly for automatic enqueueing.

### Control Endpoints

#### `GET /audio/devices`
List available audio output devices.

**Response (200 OK):**
```json
{
  "devices": [
    {"id": "default", "name": "System Default", "default": true},
    {"id": "pulse-sink-0", "name": "Built-in Audio Analog Stereo", "default": false},
    {"id": "pulse-sink-1", "name": "HDMI Audio", "default": false}
  ]
}
```

**Field Details:**
- `id` (string): Platform-specific device identifier (see Device Identifier Format below)
- `name` (string): Human-readable device name from audio system
- `default` (boolean): True if this is the current system default device

**Device Identifier Format:**
- **Linux (PulseAudio):** `pulse-sink-N` where N is the sink index (e.g., `pulse-sink-0`, `pulse-sink-1`)
- **Linux (ALSA):** `alsa-hw-N-M` where N is card number, M is device number (e.g., `alsa-hw-0-0`)
- **macOS (CoreAudio):** `coreaudio-N` where N is the device UID (e.g., `coreaudio-AppleHDAEngineOutput:1B,0,1,0:0`)
- **Windows (WASAPI):** `wasapi-{GUID}` where GUID is the device endpoint identifier
- **Special:** `default` always represents the system default audio output device

**Error Responses:**
- **500 Internal Server Error**: No audio output devices available (extremely rare, indicates system misconfiguration)
  ```json
  {"error": "no_devices_found", "message": "No audio output devices detected"}
  ```

**Notes:**
- Device list is queried dynamically from the audio subsystem at request time
- Device availability may change (headphones plugged/unplugged, USB devices connected/disconnected)
- See [single-stream-design.md](single-stream-design.md) for audio architecture implementation details

#### `POST /audio/device`
Set audio output device.

**Request:**
```json
{"device_id": "pulse-sink-1"}
```

**Response (200 OK):**
```json
{"status": "ok", "device_id": "pulse-sink-1", "device_name": "HDMI Audio"}
```

**Error Responses:**
- **404 Not Found**: Specified device_id does not exist or is no longer available
  ```json
  {"error": "device_not_found", "device_id": "pulse-sink-1"}
  ```
- **500 Internal Server Error**: Device exists but cannot be opened (in use, permission denied, etc.)
  ```json
  {"error": "device_unavailable", "device_id": "pulse-sink-1", "reason": "Device in use by another application"}
  ```

**Behavior:**
- If playback is active, audio switches to new device seamlessly (may cause brief interruption)
- If no playback is active, new device becomes active for next playback
- Device selection persists in settings table (`audio_sink` key)
- Using `device_id: "default"` delegates device selection to system defaults

**Notes:**
- Device must be one of the IDs returned by `GET /audio/devices`
- See [single-stream-design.md](single-stream-design.md) for audio device configuration details

#### `POST /audio/volume`
Set volume level (0-100 integer, user-facing scale).

**Request:**
```json
{"volume": 75}
```

**Response (200 OK):**
```json
{"status": "ok", "volume": 75}
```

**Error Responses:**
- **400 Bad Request**: Invalid volume value (out of range 0-100)
  ```json
  {"error": "invalid_volume", "volume": 150, "valid_range": "0-100"}
  ```

**Notes:**
- User-facing scale: 0-100 (integer percentage)
- Backend converts to 0.0-1.0 (double) for audio system: `system_volume = user_volume / 100.0`
- Conversion back: `user_volume = ceil(system_volume * 100.0)`
- Volume change persists to database (`settings.volume_level`)
- VolumeChanged SSE event emitted

#### `POST /playback/enqueue`
Enqueue a passage for playback.

**Request Body:**
```json
{
  "file_path": "music/albums/album_name/track.mp3",
  "start_time_ms": 0,
  "end_time_ms": 234500,
  "lead_in_point_ms": 0,
  "lead_out_point_ms": 234500,
  "fade_in_point_ms": 0,
  "fade_out_point_ms": 234500,
  "fade_in_curve": "cosine",
  "fade_out_curve": "cosine",
  "passage_guid": "uuid-string",
  "position": {
    "type": "after",
    "reference_guid": "uuid-123"
  }
}
```

**Field Details:**
- `file_path` (required): Path relative to root folder (not including root folder path itself)
  - Example: If root_folder is `/home/user/wkmp` and audio file is at `/home/user/wkmp/music/albums/track.mp3`, use `file_path: "music/albums/track.mp3"`
  - Must match a path in the `files` table (see [database_schema.md - File System Organization](database_schema.md#file-system-organization))
  - Forward slashes (`/`) used as path separator on all platforms
- `start_time_ms` (optional): Passage start time in milliseconds
- `end_time_ms` (optional): Passage end time in milliseconds
- `lead_in_point_ms` (optional): Lead-in point relative to start_time_ms
- `lead_out_point_ms` (optional): Lead-out point relative to start_time_ms
- `fade_in_point_ms` (optional): Fade-in point relative to start_time_ms
- `fade_out_point_ms` (optional): Fade-out point relative to start_time_ms
- `fade_in_curve` (optional): Fade-in curve type ("linear", "exponential", "cosine")
- `fade_out_curve` (optional): Fade-out curve type ("linear", "logarithmic", "cosine")
- `passage_guid` (optional): UUID for song identification features and passage timing defaults
- `position` (optional): Where to insert in queue
  - `type`: "after", "before", "at_order", or "append" (default)
  - `reference_guid`: Required if type is "after" or "before"

**Timing Parameter Precedence:**

When determining timing values for the enqueued passage, the following precedence order applies:

1. **Explicit timing override fields** (when provided and valid in request): Use these values
2. **Passage defaults** (when `passage_guid` is provided and timing field is missing/invalid): Read from `passages` table for the specified passage
3. **System defaults** (when timing field is still missing/invalid after steps 1-2):
   - `start_time_ms`: 0 (start of audio file)
   - `end_time_ms`: End of audio file (from file metadata)
   - `lead_in_point_ms`: Same as `start_time_ms` (zero lead-in duration)
   - `fade_in_point_ms`: Same as `start_time_ms` (zero fade-in duration)
   - `fade_in_curve`: "exponential"
   - `lead_out_point_ms`: Same as `end_time_ms` (zero lead-out duration)
   - `fade_out_point_ms`: Same as `end_time_ms` (zero fade-out duration)
   - `fade_out_curve`: "logarithmic"

**Examples:**
- **No passage_guid, no timing overrides**: All system defaults used
- **passage_guid only**: All timing from passage definition, system defaults for any NULL passage fields
- **passage_guid + partial overrides**: Override fields take precedence, passage defaults fill gaps, system defaults for remaining gaps
- **No passage_guid + partial overrides**: Override fields used, system defaults for missing fields

**Response (201 Created):**
```json
{
  "status": "ok",
  "queue_entry_id": "uuid-of-queue-entry",
  "play_order": 30
}
```

**Error Responses:**
- **400 Bad Request**: Invalid timing parameters (Phase 1 validation)
  ```json
  {
    "error": "Invalid passage timing",
    "details": {
      "validation_failures": [
        "start_time_ms (5000) >= end_time_ms (3000)",
        "fade_in_point_ms (200) < start_time_ms (5000)"
      ]
    }
  }
  ```
  Validation follows XFD-IMPL-040 through XFD-IMPL-043.
- **404 Not Found**: Audio file does not exist at specified path
  ```json
  {"error": "file_not_found", "file_path": "music/album/track.mp3"}
  ```
- **400 Bad Request**: Timing points are inconsistent or invalid
  ```json
  {"error": "invalid_timing", "message": "end_time_ms must be greater than start_time_ms"}
  ```
- **409 Conflict**: Queue is full (at queue_max_size limit)
  ```json
  {"error": "queue_full", "current_size": 100, "max_size": 100}
  ```
- **404 Not Found**: Position reference_guid does not exist in queue
  ```json
  {"error": "reference_not_found", "reference_guid": "uuid-123"}
  ```

**Ephemeral Passage Creation:**

When only `file_path` is provided (without `passage_guid`), the Audio Player creates an ephemeral passage definition with default timing:
- start_time = 0
- end_time = audio file duration (detected during decode)
- All lead/fade points = 0 (no crossfade)
- fade curves = system defaults

The ephemeral passage exists only for the current playback session and is not persisted to the database. It behaves identically to persistent passages during playback.

See entity_definitions.md REQ-DEF-035 for ephemeral passage specification.

**Timing Override Behavior:**
- When timing fields are provided in request: Override persists to `settings.queue_entry_timing_overrides` as JSON
- Override keyed by generated `queue_entry_id` (not `passage_guid`)
- Override applied when passage plays; passage defaults used if no override
- Override deleted when queue entry removed from queue
- Partial overrides supported (e.g., override only `end_time_ms` while using passage defaults for other fields)
- See [database_schema.md - Queue Entry Timing Overrides](database_schema.md#queue-entry-timing-overrides-json-schema) for complete JSON schema

**Notes:**
- All timing values are unsigned integer milliseconds
- When timing points omitted in request, values from passage definition or global settings are used
- See [Crossfade Design](crossfade.md) for timing point semantics
- `passage_guid` parameter is optional and used for song identification features (CurrentSongChanged events)

#### `DELETE /playback/queue/{passage_id}`
Remove passage from queue by queue entry GUID.

**URL Parameters:**
- `passage_id` (string): Queue entry GUID (not passage GUID)

**Response (200 OK):**
```json
{
  "status": "ok",
  "removed": true,
  "queue_entry_id": "uuid-of-removed-entry"
}
```

**Error Responses:**
- **404 Not Found**: Queue entry with specified GUID does not exist
  ```json
  {"error": "queue_entry_not_found", "queue_entry_id": "uuid"}
  ```

**Behavior:**
- If removing currently playing passage: Skip to next passage (or stop if queue becomes empty)
- If removing future passage: Queue is updated, no playback interruption
- Removal persists to database immediately
- Timing override deleted from `settings.queue_entry_timing_overrides` if present
- QueueChanged SSE event emitted with trigger `"user_dequeue"`
- If currently playing passage removed: PassageCompleted SSE event emitted with reason `"queue_removed"`

**Notes:**
- Parameter name is `passage_id` but expects queue entry GUID for deletion
- To find queue entry GUID for a passage, use `GET /playback/queue`
- Cleanup: Removes entry from both `queue` table and `queue_entry_timing_overrides` JSON

#### `POST /playback/play`
Resume playback (transition to Playing state).

**Request Body:** None (empty POST)

**Response (200 OK):**
```json
{
  "status": "ok",
  "state": "playing"
}
```

**Behavior:**
- If state is already Playing: No-op, returns 200 OK
- If state is Paused with passage loaded: Resume with configurable fade-in
- If queue is empty: Set state to Playing, wait for passage to be enqueued (plays immediately when enqueued)
- PlaybackStateChanged SSE event emitted (old_state: "paused", new_state: "playing")
- PlaybackProgress SSE event emitted immediately after state change

**Resume Fade-In Details:**
- Duration: `resume_from_pause_fade_in_duration` setting (default: 0.5 seconds)
- Curve: `resume_from_pause_fade_in_curve` setting (default: "exponential")
- Available curves: "linear", "exponential" (v(t) = t²), "cosine"
- Applies multiplicatively with any active crossfade curves
- See [crossfade.md - Pause and Resume Behavior](crossfade.md#pause-and-resume-behavior) for complete specification

**Notes:**
- Idempotent operation (safe to call multiple times)
- State persists to `settings.initial_play_state` if configured

#### `POST /playback/pause`
Pause playback (transition to Paused state).

**Request Body:** None (empty POST)

**Response (200 OK):**
```json
{
  "status": "ok",
  "state": "paused"
}
```

**Behavior:**
- If state is already Paused: No-op, returns 200 OK
- If state is Playing: Pause immediately (no fade-out)
- Current playback position preserved during pause
- PlaybackStateChanged SSE event emitted (old_state: "playing", new_state: "paused")
- PlaybackProgress SSE event emitted immediately after state change (captures paused position)

**Pause Details:**
- No fade-out: Immediate stop (volume set to 0.0 instantly)
- Audio stream continues internally (muted) to maintain position accuracy
- Rationale: Pause is an immediate stop; users expect instant response
- See [crossfade.md - Pause and Resume Behavior](crossfade.md#pause-and-resume-behavior) for complete specification

**Notes:**
- Idempotent operation (safe to call multiple times)
- Position not persisted to database until clean shutdown

### Status Endpoints

#### `GET /audio/device`
Get current audio output device.

**Response (200 OK):**
```json
{
  "device_id": "pulse-sink-1",
  "device_name": "HDMI Audio"
}
```

**Special Cases:**
- If using system default device:
  ```json
  {
    "device_id": "default",
    "device_name": "System Default"
  }
  ```
- If no device configured (first startup):
  ```json
  {
    "device_id": "default",
    "device_name": "System Default"
  }
  ```

**Notes:**
- Returns device currently in use by audio output system
- Device persisted in `settings.audio_sink` database key

#### `GET /audio/volume`
Get current volume level (0-100 integer).

**Response (200 OK):**
```json
{"volume": 75}
```

**Notes:**
- Returns user-facing 0-100 integer scale (converted from internal 0.0-1.0)
- Conversion: `user_volume = ceil(system_volume * 100.0)`

#### `GET /playback/queue`
Get queue contents in play order.

**Response (200 OK):**
```json
{
  "queue": [
    {
      "queue_entry_id": "uuid-1",
      "passage_id": "passage-uuid-1",
      "play_order": 10,
      "file_path": "music/album/track1.mp3",
      "timing_override": null
    },
    {
      "queue_entry_id": "uuid-2",
      "passage_id": "passage-uuid-2",
      "play_order": 20,
      "file_path": "music/album/track2.mp3",
      "timing_override": {
        "start_time_ms": 5000,
        "end_time_ms": 180000,
        "fade_in_curve": "linear"
      }
    }
  ]
}
```

**Field Details:**
- `queue_entry_id` (string): Unique ID for this queue entry (use for deletion)
- `passage_id` (string): Passage GUID from database
- `play_order` (integer): Play order (ascending, gaps allowed)
- `file_path` (string): Audio file path relative to root folder
- `timing_override` (object or null): Timing overrides for this queue entry
  - If null: Uses passage timing from database
  - If object: Contains override values (only overridden fields included)

**Empty Queue Response:**
```json
{"queue": []}
```

**Notes:**
- Queue entries ordered by `play_order` ascending
- Currently playing passage is NOT included (only future passages)
- Timing override format matches `POST /playback/enqueue` request format

#### `GET /playback/state`
Get current playback state (Playing or Paused).

**Response (200 OK):**
```json
{"state": "playing"}
```

**Possible Values:**
- `"playing"`: Playback is active (or waiting for queue to be populated)
- `"paused"`: Playback is paused

**Notes:**
- WKMP uses two-state model only (no "stopped" state)
- State reflects intent, not whether audio is currently audible
- If state is "playing" but queue is empty, audio plays immediately when passage enqueued

#### `GET /playback/position`
Get current playback position within currently playing passage.

**Response (200 OK):**
```json
{
  "passage_id": "uuid-string",
  "position_ms": 45200,
  "duration_ms": 234500,
  "state": "playing"
}
```

**No Passage Playing Response (200 OK):**
```json
{
  "passage_id": null,
  "position_ms": 0,
  "duration_ms": 0,
  "state": "paused"
}
```

**Field Details:**
- `passage_id` (string or null): UUID of currently playing passage, or null if none
- `position_ms` (integer): Current position in milliseconds (0 if no passage)
- `duration_ms` (integer): Total passage duration in milliseconds (0 if no passage)
- `state` (string): Current playback state ("playing" or "paused")

**Notes:**
- Position updates in real-time during playback
- Position query uses audio system's playback position tracking
- During crossfade, returns position of currently audible passage (not next passage)

#### `GET /playback/buffer_status`

Retrieve current buffer decode/playback status for all passages in queue.

**Authentication:** None

**Request:** No parameters

**Response (200 OK):**
```json
{
  "buffers": [
    {
      "passage_id": "550e8400-e29b-41d4-a716-446655440000",
      "status": "Ready",
      "decode_progress_percent": null,
      "sample_count": 1234567,
      "sample_rate": 44100
    },
    {
      "passage_id": "550e8400-e29b-41d4-a716-446655440001",
      "status": "Decoding",
      "decode_progress_percent": 45.2,
      "sample_count": 567890,
      "sample_rate": 44100
    }
  ]
}
```

**Response Fields:**
- `buffers`: Array of buffer status objects, ordered by queue position
- `passage_id`: UUID of passage
- `status`: One of: "Decoding", "Ready", "Playing", "Exhausted"
- `decode_progress_percent`: 0.0-100.0 (only present for "Decoding" status)
- `sample_count`: Number of samples currently buffered
- `sample_rate`: Sample rate (Hz) after resampling

**Error Responses:**
- 503 Service Unavailable: Audio playback subsystem not initialized

**Traceability:** SSD-BUF-010 (Buffer state visibility)

### Health Endpoint

#### `GET /health`
Health check endpoint for monitoring and duplicate instance detection. **Required for all modules** (wkmp-ap, wkmp-ui, wkmp-pd, wkmp-le, wkmp-ai).

See [architecture.md - Health check and respawning](architecture.md#launch-procedure) for detailed subprocess health monitoring behavior.

**Request Format:**
- Method: `GET`
- Path: `/health`
- Headers: None required
- Body: None
- This is a minimal health check that simply ensures the server is responsive

**Health Check Behavior:**
- **Timeout**: If no response within 2 seconds, the health check is considered failed
- **Timeout triggers relaunch**: A timeout or connection refused triggers the subprocess relaunch procedure
- **Any response indicates health**: Currently, any HTTP response (even error codes) indicates the service is alive
- Additional health parameters with more nuanced information may be added in the future

**Response (200 OK):**
```json
{
  "status": "healthy",
  "module": "audio_player",
  "version": "1.0.0",
  "uptime_seconds": 3600,
  "checks": {
    "database": "ok",
    "audio_device": "ok",
    "audio_subsystem": "ok"
  }
}
```

**Response (503 Service Unavailable):**
```json
{
  "status": "unhealthy",
  "module": "audio_player",
  "version": "1.0.0",
  "uptime_seconds": 120,
  "checks": {
    "database": "ok",
    "audio_device": "failed",
    "audio_subsystem": "ok"
  }
}
```

**Field Details:**
- `status` (string): "healthy" or "unhealthy"
- `module` (string): Module name ("audio_player", "user_interface", "program_director", etc.)
- `version` (string): Module version string
- `uptime_seconds` (integer): Seconds since module started
- `checks` (object): Health status of critical subsystems
  - `database` (string): "ok" if database accessible, "failed" otherwise
  - `audio_device` (string): "ok" if audio output device working, "failed" if unavailable
  - `audio_subsystem` (string): "ok" if audio subsystem initialized, "failed" otherwise

**Unhealthy Conditions:**
- Database connection fails
- Audio output device cannot be opened
- Audio subsystem cannot be initialized

**Notes:**
- Used for duplicate instance detection during startup (see deployment.md)
- Used for health monitoring by operators
- Should respond quickly (<100ms) even under load

### SSE Events

#### `GET /events`
Server-Sent Events stream for real-time playback updates.

**Events emitted:**
- `VolumeChanged`
- `QueueChanged`
- `PlaybackStateChanged`
- `PlaybackProgress` (interval configurable via `playback_progress_interval_ms` setting, default 5000ms)
- `PassageStarted`
- `PassageCompleted`
- `CurrentSongChanged`

**Event Format:** All events follow standard SSE format with `event:` and `data:` fields. The `data:` field contains JSON.

---

## SSE Event Formats

This section defines the JSON wire format for Server-Sent Events transmitted by the Audio Player (`GET /events` endpoint).

**Note on Volume Scale:** SSE events use the system-level volume scale (0.0-1.0 floating-point) rather than the user-facing scale (0-100 integer). This allows for precise volume representation in real-time event streams. User interface components should convert to 0-100 integer scale for display (see [architecture.md - Volume Handling](architecture.md#volume-handling) for conversion formulas).

### PlaybackProgress

Emitted periodically during playback at configurable interval (`playback_progress_interval_ms` setting, default 5000ms). Also emitted once when Pause initiated and once when Play initiated.

**SSE Format:**
```
event: PlaybackProgress
data: {"passage_id":"uuid-string","position_ms":45200,"duration_ms":234500,"timestamp":"2025-10-10T14:30:00Z"}
```

**JSON Structure:**
```json
{
  "passage_id": "uuid-string",
  "position_ms": 45200,
  "duration_ms": 234500,
  "timestamp": "2025-10-10T14:30:00Z"
}
```

**Fields:**
- `passage_id` (string): UUID of currently playing passage
- `position_ms` (integer): Current playback position in milliseconds
- `duration_ms` (integer): Total passage duration in milliseconds
- `timestamp` (string): ISO 8601 timestamp (UTC) when event was generated

### VolumeChanged

Emitted when volume changes via `POST /audio/volume` or other volume control mechanisms.

**SSE Format:**
```
event: VolumeChanged
data: {"old_volume":0.75,"new_volume":0.85,"timestamp":"2025-10-10T14:30:00Z"}
```

**JSON Structure:**
```json
{
  "old_volume": 0.75,
  "new_volume": 0.85,
  "timestamp": "2025-10-10T14:30:00Z"
}
```

**Fields:**
- `old_volume` (float): Previous volume level (0.0 = mute, 1.0 = maximum)
- `new_volume` (float): New volume level (0.0 = mute, 1.0 = maximum)
- `timestamp` (string): ISO 8601 timestamp (UTC) when volume changed

**Conversion:** System volume (0.0-1.0) → User display (0-100): `user_volume = ceil(system_volume × 100.0)`

### QueueChanged

Emitted when queue contents change (passage added, removed, reordered, or cleared).

**SSE Format:**
```
event: QueueChanged
data: {"queue":["uuid1","uuid2","uuid3"],"trigger":"user_enqueue","timestamp":"2025-10-10T14:30:00Z"}
```

**JSON Structure:**
```json
{
  "queue": ["uuid1", "uuid2", "uuid3"],
  "trigger": "user_enqueue",
  "timestamp": "2025-10-10T14:30:00Z"
}
```

**Fields:**
- `queue` (array of strings): Ordered list of passage UUIDs in the queue (play_order ascending)
- `trigger` (string): What caused the queue change
- `timestamp` (string): ISO 8601 timestamp (UTC) when queue changed

**Trigger Values:**
- `"user_enqueue"`: User manually added passage via UI
- `"program_director"`: Program Director automatically added passage
- `"user_dequeue"`: User removed passage via UI
- `"passage_completed"`: Passage finished playing and was removed
- `"queue_cleared"`: User cleared entire queue
- `"queue_reordered"`: User reordered queue entries

### PlaybackStateChanged

Emitted when playback state transitions between Playing and Paused.

**SSE Format:**
```
event: PlaybackStateChanged
data: {"old_state":"paused","new_state":"playing","timestamp":"2025-10-10T14:30:00Z"}
```

**JSON Structure:**
```json
{
  "old_state": "paused",
  "new_state": "playing",
  "timestamp": "2025-10-10T14:30:00Z"
}
```

**Fields:**
- `old_state` (string): Previous playback state (`"playing"` or `"paused"`)
- `new_state` (string): New playback state (`"playing"` or `"paused"`)
- `timestamp` (string): ISO 8601 timestamp (UTC) when state changed

### PassageStarted

Emitted when a new passage begins playing (first passage in queue or after previous passage completes/skips).

**SSE Format:**
```
event: PassageStarted
data: {"passage_id":"uuid-string","file_path":"music/albums/album_name/track.mp3","start_time_ms":0,"end_time_ms":234500,"timestamp":"2025-10-10T14:30:00Z"}
```

**JSON Structure:**
```json
{
  "passage_id": "uuid-string",
  "file_path": "music/albums/album_name/track.mp3",
  "start_time_ms": 0,
  "end_time_ms": 234500,
  "timestamp": "2025-10-10T14:30:00Z"
}
```

**Fields:**
- `passage_id` (string): UUID of passage that started
- `file_path` (string): Audio file path relative to root folder (see [database_schema.md - File System Organization](database_schema.md#file-system-organization))
- `start_time_ms` (integer): Passage start time in milliseconds
- `end_time_ms` (integer): Passage end time in milliseconds
- `timestamp` (string): ISO 8601 timestamp (UTC) when passage started

### PassageCompleted

Emitted when a passage finishes playing (reached end) or is skipped.

**SSE Format:**
```
event: PassageCompleted
data: {"passage_id":"uuid-string","completed":true,"reason":"natural","timestamp":"2025-10-10T14:30:00Z"}
```

**JSON Structure:**
```json
{
  "passage_id": "uuid-string",
  "completed": true,
  "reason": "natural",
  "timestamp": "2025-10-10T14:30:00Z"
}
```

**Fields:**
- `passage_id` (string): UUID of passage that completed
- `completed` (boolean): `true` if played to end, `false` if skipped or failed
- `reason` (string): Why passage completed
- `timestamp` (string): ISO 8601 timestamp (UTC) when passage completed

**Reason Values:**
- `"natural"`: Passage played to its end_time_ms
- `"user_skip"`: User pressed skip button
- `"playback_error"`: Audio file could not be decoded or played
- `"queue_removed"`: Passage removed from queue while playing

### CurrentSongChanged

Emitted when the currently playing song within a passage changes (when passage contains multiple songs or gaps).

**SSE Format:**
```
event: CurrentSongChanged
data: {"passage_id":"uuid-string","song_id":"song-uuid","song_albums":["album-uuid-1","album-uuid-2"],"position_ms":45200,"timestamp":"2025-10-10T14:30:00Z"}
```

**JSON Structure:**
```json
{
  "passage_id": "uuid-string",
  "song_id": "song-uuid",
  "song_albums": ["album-uuid-1", "album-uuid-2"],
  "position_ms": 45200,
  "timestamp": "2025-10-10T14:30:00Z"
}
```

**Fields:**
- `passage_id` (string): UUID of currently playing passage
- `song_id` (string or null): UUID of current song, or `null` if in gap between songs
- `song_albums` (array of strings): UUIDs of all albums associated with this song (empty array if song_id is null)
- `position_ms` (integer): Current playback position in passage (milliseconds)
- `timestamp` (string): ISO 8601 timestamp (UTC) when song changed

**Use Cases:**
- Update album art when song changes within multi-song passage
- Display "now playing" song information
- Track song-level play history

**Note:** This event is distinct from `PassageStarted`. `PassageStarted` fires when a new queue entry begins playing. `CurrentSongChanged` fires when crossing song boundaries within a single passage.

---

## Program Director API

**Base URL:** `http://localhost:5722`
**Port:** 5722 (configurable)
**Purpose:** Selection configuration, timeslot management
**Authentication:** None (internal/trusted network only)

**Note:** End users access these endpoints via User Interface, which proxies configuration requests.

### Configuration Endpoints

#### `GET /config/timeslots`
Retrieve timeslot configuration for 24-hour schedule.

#### `POST /config/timeslots`
Update timeslot configuration.

#### `GET /config/probabilities`
Get base probabilities for songs/artists/works.

#### `PUT /config/probabilities/{entity_type}/{id}`
Set base probability for specific entity.

#### `GET /config/cooldowns`
Get cooldown settings (minimum and ramping periods).

#### `PUT /config/cooldowns`
Update cooldown settings.

#### `POST /selection/override`
Activate temporary flavor override.

#### `DELETE /selection/override`
Clear temporary flavor override.

#### `POST /selection/request`
Request passage selection (called by Audio Player when queue is low).

**Called by:** Audio Player (wkmp-ap)

**Request:**
```json
{
  "anticipated_start_time": "2025-10-09T15:30:45Z",
  "current_queue_passages": 1,
  "current_queue_duration_seconds": 240
}
```

**Response (immediate acknowledgment):**
```json
{
  "status": "acknowledged",
  "request_id": "uuid"
}
```

**Behavior:**
- Program Director responds immediately to acknowledge request:
  - Timeout configured in settings table: `queue_refill_acknowledgment_timeout_seconds` (default: 5 seconds)
  - Audio Player uses this timeout to determine when to relaunch Program Director
- Selection happens asynchronously (may take longer than throttle interval)
- If no candidates available, selection fails but acknowledgment still sent
- Selected passage is enqueued to Audio Player via `POST /playback/enqueue`
- Audio Player throttles requests while queue is underfilled:
  - Interval configured in settings table: `queue_refill_request_throttle_seconds` (default: 10 seconds)

### Status Endpoints

#### `GET /status`
Get module status, current timeslot, target flavor.

#### `GET /selection/candidates`
Get last selection candidates (debugging).

### SSE Events

#### `GET /events`
Server-Sent Events stream for selection updates.

**Events emitted:**
- `TimeslotChanged`
- `TemporaryFlavorOverride`
- `OverrideExpired`
- `SelectionFailed`

---

## Audio Ingest API (Full Version Only)

**Base URL:** `http://localhost:5723`
**Port:** 5723 (configurable)
**Purpose:** File scanning, ingest workflow
**Authentication:** None (internal/trusted network only)

### Ingest Endpoints

#### `POST /ingest/scan`
Scan directory for new audio files.

#### `GET /ingest/pending`
List files pending ingest.

#### `POST /ingest/identify/{file_id}`
Trigger MusicBrainz lookup for file identification.

#### `POST /ingest/characterize/{file_id}`
Trigger flavor analysis (AcousticBrainz or Essentia).

#### `POST /ingest/segment/{file_id}`
Define passages within file.

#### `PUT /ingest/metadata/{passage_id}`
Edit passage metadata.

#### `PUT /ingest/related_songs/{song_guid}`
Edit the related songs list for a song.

**Request:**
```json
{
  "related_songs": ["song-guid-1", "song-guid-2", "song-guid-3"]
}
```

**Response:**
```json
{
  "status": "ok",
  "song_guid": "abc123..."
}
```

**Behavior:**
- Accepts a JSON array of song GUIDs, ordered from most to least closely related
- Updates the `songs.related_songs` field for the specified song
- Related songs are typically other recordings of the same Work
- This list is initially populated during MusicBrainz identification, but can be user-edited

#### `POST /ingest/finalize/{file_id}`
Complete ingest workflow and commit to library.

---

## Server-Sent Events (SSE)

### User Interface: `GET /api/events`

Real-time event stream for UI updates. User Interface aggregates events from Audio Player and adds user-specific events.

**Connection:** Keep-alive HTTP connection with `text/event-stream` content type

**Event Format:**
```
event: <event_type>
data: <json_payload>

```

**Event Types:**

See [Event System](event_system.md) for complete event enumeration and payloads.

**Key Events for UI:**
- `passage_started` - New passage began playing
- `passage_completed` - Passage finished or skipped
- `playback_state_changed` - Playing/Paused/Stopped transition
- `position_update` - Playback position update (every 500ms)
- `volume_changed` - Volume level changed
- `queue_changed` - Queue contents modified
- `user_action` - Another user performed an action (for multi-user sync)
- `network_status_changed` - Network connectivity status

**Example Event:**
```
event: passage_started
data: {"passage_id": "550e8400-e29b-41d4-a716-446655440000", "timestamp": "2025-10-06T14:23:45Z", "queue_position": 0}

```

**Client Reconnection:**
- Clients should implement automatic reconnection on disconnect
- No event replay on reconnection (client fetches current state via GET /api/status)

**Multi-user Synchronization:**

All connected clients receive the same event stream, ensuring synchronized UI state across desktop and mobile browsers.

> **Implements:** [Requirements - Real-time UI Updates](requirements.md#core-features)

## Error Responses

All endpoints may return error responses:

**Format:**
```json
{
  "error": "error_code",
  "message": "Human-readable error description"
}
```

**Common Error Codes:**
- `invalid_request` - Malformed request body or parameters
- `not_found` - Passage/resource not found
- `internal_error` - Server-side error
- `version_restricted` - Feature not available in current version (Lite/Minimal)

**HTTP Status Codes:**
- `200 OK` - Success
- `400 Bad Request` - Invalid request
- `404 Not Found` - Resource not found
- `500 Internal Server Error` - Server error
- `503 Service Unavailable` - Feature not available in this version

## Network Requirements

**Local Network Access (WebUI Server):**
- REST API and SSE endpoint require HTTP server running on `localhost:5720`
- Accessible via localhost (no network) or local network (LAN required)
- No internet connection required for API operation

**Internet Access (External Data):**
- Not required for any API endpoint
- Library import/update operations may trigger internet requests internally
- Internet failures do not affect API availability or playback control

## CORS Policy

**Allowed Origins:** `http://localhost:*`

**Rationale:** Local-only access, no external network exposure. User responsible for network security.

## Rate Limiting

No rate limiting on local API endpoints.

**Note:** External API rate limits (AcoustID, MusicBrainz) handled internally by WKMP, not exposed to API clients.

## API Versioning

**Current Version:** v1 (implicit, no version in URL)

**Future Versioning:** If breaking changes needed, introduce `/api/v2/...` endpoints while maintaining v1 compatibility.

## Implementation Notes

### API Layer Architecture

See [Architecture - API Layer](architecture.md#layered-architecture) for component structure.

**Request Flow:**
1. HTTP request received by Tauri/Axum web server
2. Request validation
3. Command dispatch via `tokio::mpsc` channels to appropriate component
4. Response from component (may be async)
5. JSON response to client

**SSE Broadcasting:**

SSE endpoint subscribes to EventBus (see [Event System](event_system.md)) and forwards all events to connected clients.

### Testing

API endpoints should have integration tests covering:
- Request validation
- Multi-user edge cases (skip throttling, concurrent operations)
- Error handling
- Version-specific endpoint availability

----
End of document - WKMP API Design

**Document Version:** 1.1
**Last Updated:** 2025-10-17

**Change Log:**
- v1.1 (2025-10-17): Added buffer status endpoint and timing validation specifications
  - Added `GET /playback/buffer_status` endpoint with buffer state and progress reporting
  - Added Phase 1 validation error response to `POST /playback/enqueue` endpoint
  - Added Ephemeral Passage Creation section explaining transient passage behavior
  - Supports architectural decisions from wkmp-ap design review (ISSUE-1, ISSUE-2, ISSUE-4)
