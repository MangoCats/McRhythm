# McRhythm API Design

**🌐 TIER 2 - DESIGN SPECIFICATION**

Defines REST API structure and Server-Sent Events interface. Derived from [requirements.md](requirements.md). See [Document Hierarchy](document_hierarchy.md).

> **Related Documentation:** [Requirements](requirements.md) | [Architecture](architecture.md) | [Event System](event_system.md)

---

## Overview

McRhythm implements a **microservices architecture** with 4 independent HTTP servers, each exposing its own REST API and SSE endpoints. Modules communicate via HTTP APIs and share a common SQLite database.

### Module API Endpoints

| Module | Default Port | Base URL | Purpose |
|--------|--------------|----------|---------|
| **Module 1: Audio Player** | 8081 | `http://localhost:8081` | Playback control, queue management |
| **Module 2: User Interface** | 8080 | `http://localhost:8080/api` | User-facing API, authentication, library browsing |
| **Module 3: Program Director** | 8082 | `http://localhost:8082` | Selection configuration, timeslots |
| **Module 4: File Ingest** | 8083 | `http://localhost:8083` | File scanning, ingest workflow (Full only) |

### API Communication Patterns

**End Users → Module 2 (User Interface):**
- Module 2 serves as the primary API gateway for end users
- Proxies playback requests to Module 1
- Proxies configuration requests to Module 3
- Handles authentication and session management
- Aggregates SSE events from Module 1

**Module 3 (Program Director) → Module 1 (Audio Player):**
- Direct communication for automatic enqueueing
- No user interface involvement required

**Module 4 (File Ingest) → Database:**
- Direct SQLite access for new file insertion
- Independent operation

### Authentication

**User Interface API (Module 2)** handles all authentication:
1. Proceed as Anonymous user (shared UUID, no password)
2. Create new account (generates unique UUID, requires username/password)
3. Login to existing account (retrieves UUID, requires username/password)

Once authenticated, the browser stores the user UUID in localStorage with a rolling one-year expiration. See [User Identity and Authentication](user_identity.md) for complete flow.

**Internal Module APIs** (Modules 1, 3, 4):
- No authentication required (assumed to be on trusted local network)
- Minimal developer UIs for debugging
- Security relies on network isolation

**Content-Type:** `application/json` for all request/response bodies across all modules

---

## Module 2: User Interface API

**Base URL:** `http://localhost:8080/api`
**Port:** 8080 (configurable)
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
  "user_id": "550e8400-e29b-41d4-a716-446655440000",
  "username": "Anonymous",
  "is_anonymous": true
}
```

### Playback Control Endpoints (Proxied to Module 1)

**Note:** These endpoints are exposed by Module 2 at `/api/playback/*` and proxied to Module 1's HTTP API. All require user authentication (UUID from localStorage).

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
- Initial state on app launch: "playing"
- `state` reflects user-selected Play/Pause mode, independent of queue state
- When queue is empty, `passage` is `null` but `state` remains as user set it
- System in "playing" state with empty queue produces no audio output
- Enqueueing a passage while in "playing" state begins playback immediately
- Enqueueing a passage while in "paused" state queues it without starting playback
- State persists across app restarts: always resumes to "playing" on launch

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

Jump to specific position in current passage.

**Request:**
```json
{
  "position": 60.5
}
```

**Parameters:**
- `position`: Float, seconds from passage start

**Response:**
```json
{
  "status": "ok",
  "position": 60.5
}
```

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

### Lyrics (Full version only)

#### `GET /api/lyrics/:passage_id`

Retrieve lyrics for a passage.

**Parameters:**
- `passage_id`: UUID in URL path

**Response:**
```json
{
  "passage_id": "uuid",
  "lyrics": "string (plain UTF-8 text, may contain newlines)"
}
```

**Response (no lyrics):**
```json
{
  "passage_id": "uuid",
  "lyrics": null
}
```

#### `PUT /api/lyrics/:passage_id`

Update lyrics for a passage (Full version only).

**Parameters:**
- `passage_id`: UUID in URL path

**Request:**
```json
{
  "lyrics": "string (plain UTF-8 text)"
}
```

**Response:**
```json
{
  "status": "ok",
  "passage_id": "uuid"
}
```

**Edge Case:** Concurrent lyric submissions are handled via a "last write wins" strategy. See [Multi-User Coordination](multi_user_coordination.md#3-concurrent-lyric-editing) for details.

### Library Management (Full version only)

#### `POST /api/import`

Trigger library scan for new/changed audio files.

**Request:** Empty body or optional directory paths

**Request (with paths):**
```json
{
  "paths": ["/path/to/music/folder"]
}
```

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

## Module 1: Audio Player API

**Base URL:** `http://localhost:8081`
**Port:** 8081 (configurable)
**Purpose:** Direct playback control, queue management
**Authentication:** None (internal/trusted network only)

**Note:** End users typically access these endpoints via Module 2 (User Interface), which proxies requests. Module 3 (Program Director) calls these endpoints directly for automatic enqueueing.

### Control Endpoints

#### `POST /audio/device`
Set audio output device.

#### `POST /audio/volume`
Set volume level (0-100).

#### `POST /playback/enqueue`
Enqueue a passage for playback.

#### `DELETE /playback/queue/{passage_id}`
Remove passage from queue.

#### `POST /playback/play`
Resume playback (set state to Playing).

#### `POST /playback/pause`
Pause playback (set state to Paused).

### Status Endpoints

#### `GET /audio/device`
Get current audio output device.

#### `GET /audio/volume`
Get current volume level.

#### `GET /playback/queue`
Get queue contents.

#### `GET /playback/state`
Get Playing/Paused state.

#### `GET /playback/position`
Get current playback position in passage.

### SSE Events

#### `GET /events`
Server-Sent Events stream for real-time playback updates.

**Events emitted:**
- `VolumeChanged`
- `QueueChanged`
- `PlaybackStateChanged`
- `PlaybackProgress` (every 500ms)
- `PassageStarted`
- `PassageCompleted`
- `CurrentSongChanged`

---

## Module 3: Program Director API

**Base URL:** `http://localhost:8082`
**Port:** 8082 (configurable)
**Purpose:** Selection configuration, timeslot management
**Authentication:** None (internal/trusted network only)

**Note:** End users access these endpoints via Module 2 (User Interface), which proxies configuration requests.

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

## Module 4: File Ingest API (Full Version Only)

**Base URL:** `http://localhost:8083`
**Port:** 8083 (configurable)
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

#### `POST /ingest/finalize/{file_id}`
Complete ingest workflow and commit to library.

---

## Server-Sent Events (SSE)

### Module 2: `GET /api/events`

Real-time event stream for UI updates. Module 2 aggregates events from Module 1 and adds user-specific events.

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

**Note:** External API rate limits (AcoustID, MusicBrainz) handled internally by McRhythm, not exposed to API clients.

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
End of document - McRhythm API Design
