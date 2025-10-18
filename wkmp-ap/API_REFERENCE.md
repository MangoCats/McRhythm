# WKMP Audio Player API Reference

**Module:** wkmp-ap (Audio Player)
**Default Port:** 5721
**Version:** 1.0.0
**Protocol:** HTTP REST + Server-Sent Events (SSE)

---

## Table of Contents

1. [Overview](#overview)
2. [Authentication](#authentication)
3. [Base URL](#base-url)
4. [Endpoints](#endpoints)
   - [Health](#health)
   - [Audio Devices](#audio-devices)
   - [Volume Control](#volume-control)
   - [Queue Management](#queue-management)
   - [Playback Control](#playback-control)
   - [Status Queries](#status-queries)
5. [Server-Sent Events (SSE)](#server-sent-events-sse)
6. [Error Handling](#error-handling)
7. [Examples](#examples)

---

## Overview

The WKMP Audio Player API provides REST endpoints for controlling music playback, managing queues, and monitoring state via Server-Sent Events.

**Core Features:**
- **Sample-Accurate Crossfading:** ~0.02ms precision, 5 fade curves
- **Multi-Format Support:** MP3, FLAC, AAC, Vorbis, Opus, WAV
- **Persistent Queue:** SQLite-backed queue with resume on restart
- **Real-Time Events:** SSE push notifications for all state changes
- **Event-Driven:** Minimal CPU usage (<1%), low latency (<50ms)

**Architecture:**
```
┌─────────────────────────────────────────┐
│   HTTP/SSE API (Port 5721)              │
├─────────────────────────────────────────┤
│   Playback Engine                       │
│   ├─ Queue Manager                      │
│   ├─ Decoder Pool (symphonia)          │
│   ├─ Crossfade Mixer                    │
│   └─ Audio Output (cpal)                │
├─────────────────────────────────────────┤
│   SQLite Database (shared)              │
└─────────────────────────────────────────┘
```

---

## Authentication

**Current Status:** No authentication required ⚠️

The Audio Player API is currently **open and insecure**. Any application with network access can control playback.

**Future:** Authentication will be handled by the User Interface (wkmp-ui) module via session tokens.

---

## Base URL

**Default:** `http://localhost:5721`
**Configurable:** Set via configuration file or environment variable

**Example:**
```bash
export WKMP_AP_PORT=5721
export WKMP_AP_HOST=0.0.0.0  # Bind to all interfaces
```

---

## Endpoints

### Health

#### GET /health

Health check endpoint for service discovery and monitoring.

**Response:**
```json
{
  "status": "healthy",
  "module": "audio_player",
  "version": "0.1.0"
}
```

**Example:**
```bash
curl http://localhost:5721/health
```

---

### Audio Devices

#### GET /audio/devices

List available audio output devices detected by cpal.

**Response:**
```json
{
  "devices": [
    "default",
    "PulseAudio",
    "HDA Intel PCH, ALC887-VD Analog"
  ]
}
```

**Example:**
```bash
curl http://localhost:5721/audio/devices
```

**Platform Notes:**
- **Linux:** PulseAudio, ALSA device names
- **macOS:** CoreAudio device names
- **Windows:** WASAPI device names

---

#### GET /audio/device

Get currently configured audio output device.

**Response:**
```json
{
  "device_name": "default"
}
```

**Example:**
```bash
curl http://localhost:5721/audio/device
```

---

#### POST /audio/device

Set audio output device.

**Request Body:**
```json
{
  "device_name": "PulseAudio"
}
```

**Response:** 200 OK (empty body)

**Example:**
```bash
curl -X POST http://localhost:5721/audio/device \
  -H "Content-Type: application/json" \
  -d '{"device_name":"PulseAudio"}'
```

**Note:** Device change requires audio output restart (not yet implemented). Setting is persisted for next restart.

---

### Volume Control

#### GET /audio/volume

Get current volume level (0-100 scale).

**Response:**
```json
{
  "volume": 75
}
```

**Example:**
```bash
curl http://localhost:5721/audio/volume
```

---

#### POST /audio/volume

Set volume level (0-100 scale).

**Request Body:**
```json
{
  "volume": 75
}
```

**Response:**
```json
{
  "volume": 75
}
```

**Example:**
```bash
curl -X POST http://localhost:5721/audio/volume \
  -H "Content-Type: application/json" \
  -d '{"volume":75}'
```

**Behavior:**
- Volume change takes effect immediately
- Emits `VolumeChanged` SSE event
- Persisted to database

**Validation:**
- Must be integer 0-100
- Values outside range return 400 Bad Request

---

### Queue Management

#### POST /playback/enqueue

Add audio file to playback queue.

**Request Body:**
```json
{
  "file_path": "albums/4_Non_Blondes/Bigger,_Better,_Faster,_More/03-What's_Up.mp3"
}
```

**Response:**
```json
{
  "status": "ok",
  "queue_entry_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

**Example:**
```bash
curl -X POST http://localhost:5721/playback/enqueue \
  -H "Content-Type: application/json" \
  -d '{"file_path":"albums/artist/song.mp3"}'
```

**File Path Requirements:**
- Must be relative to configured root folder
- Use forward slashes (/) as separator (even on Windows)
- Example: `albums/artist/album/song.mp3`

**Supported Formats:**
- MP3 (MPEG-1 Layer 3)
- FLAC (Free Lossless Audio Codec)
- AAC (Advanced Audio Coding)
- Vorbis (.ogg)
- Opus (.opus)
- WAV (PCM, uncompressed)

**Behavior:**
- Creates ephemeral passage if file not in database
- Automatically starts playback if queue was empty
- Emits `QueueChanged` SSE event

---

#### DELETE /playback/queue/:queue_entry_id

Remove specific entry from queue.

**Response:** 204 No Content

**Example:**
```bash
curl -X DELETE http://localhost:5721/playback/queue/550e8400-e29b-41d4-a716-446655440000
```

**Behavior:**
- Removes from both in-memory queue and database
- Cannot remove currently playing passage
- Emits `QueueChanged` SSE event

**Error Response:**
```json
{
  "status": "error: Queue entry not found"
}
```

---

#### POST /playback/queue/clear

Remove all entries from queue.

**Response:** 204 No Content

**Example:**
```bash
curl -X POST http://localhost:5721/playback/queue/clear
```

**Behavior:**
- Clears both in-memory queue and database
- Stops playback if currently playing
- Emits `QueueChanged` SSE event

---

#### POST /playback/queue/reorder

Move queue entry to new position.

**Request Body:**
```json
{
  "queue_entry_id": "550e8400-e29b-41d4-a716-446655440000",
  "new_position": 0
}
```

**Response:** 200 OK (empty body)

**Example:**
```bash
curl -X POST http://localhost:5721/playback/queue/reorder \
  -H "Content-Type: application/json" \
  -d '{"queue_entry_id":"550e8400-e29b-41d4-a716-446655440000","new_position":0}'
```

**Position Index:**
- 0-based index
- 0 = first in queue (plays next after current)
- Negative = count from end (-1 = last)

**Behavior:**
- Updates both in-memory queue and database
- Cannot reorder currently playing passage
- Emits `QueueChanged` SSE event

---

#### GET /playback/queue

Get all entries in queue.

**Response:**
```json
{
  "queue": [
    {
      "queue_entry_id": "550e8400-e29b-41d4-a716-446655440000",
      "passage_id": "7c9e6679-7425-40de-944b-e07fc1f90ae7",
      "file_path": "albums/artist/song1.mp3"
    },
    {
      "queue_entry_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
      "passage_id": null,
      "file_path": "albums/artist/song2.mp3"
    }
  ]
}
```

**Example:**
```bash
curl http://localhost:5721/playback/queue
```

**Response Fields:**
- `queue_entry_id`: Unique identifier for this queue entry
- `passage_id`: Passage UUID (null for ephemeral passages)
- `file_path`: Audio file path relative to root folder

**Order:** Entries are returned in playback order (first = next to play after current).

---

### Playback Control

#### POST /playback/play

Resume playback from paused state.

**Response:**
```json
{
  "status": "ok"
}
```

**Example:**
```bash
curl -X POST http://localhost:5721/playback/play
```

**Behavior:**
- Changes state from Paused → Playing
- If queue empty, has no effect
- Emits `PlaybackStateChanged` SSE event
- Emits `PlaybackProgress` SSE event with current position
- Persists position to database

---

#### POST /playback/pause

Pause playback immediately.

**Response:**
```json
{
  "status": "ok"
}
```

**Example:**
```bash
curl -X POST http://localhost:5721/playback/pause
```

**Behavior:**
- Changes state from Playing → Paused
- Position preserved for resume (no fade-out)
- Emits `PlaybackStateChanged` SSE event
- Emits `PlaybackProgress` SSE event with current position
- Persists position to database

---

#### POST /playback/next

Skip currently playing passage.

**Response:** 200 OK (empty body)

**Example:**
```bash
curl -X POST http://localhost:5721/playback/next
```

**Behavior:**
- Stops current passage immediately (no fade-out)
- Removes current from queue
- Starts next passage in queue
- Emits `PassageCompleted` SSE event (completed=false for skip)
- Emits `QueueChanged` SSE event

**Error Response:**
```json
{
  "status": "error: No passage to skip - queue is empty"
}
```

---

#### POST /playback/previous

Skip to previous passage (not implemented).

**Response:**
```json
{
  "status": "Previous playback not implemented"
}
```

**Status Code:** 501 Not Implemented

**Example:**
```bash
curl -X POST http://localhost:5721/playback/previous
# Returns 501 Not Implemented
```

**Note:** Previous/backwards playback is not implemented in current architecture.

---

#### POST /playback/seek

Seek to position in current passage.

**Request Body:**
```json
{
  "position_ms": 45000
}
```

**Response:** 200 OK (empty body)

**Example:**
```bash
curl -X POST http://localhost:5721/playback/seek \
  -H "Content-Type: application/json" \
  -d '{"position_ms":45000}'
```

**Behavior:**
- Immediately jumps to new position
- Preserves playing/paused state
- Position clamped to passage duration
- Seeking beyond end skips to next passage
- Emits `PlaybackProgress` SSE event with new position
- May emit `CurrentSongChanged` if crossing song boundary

---

### Status Queries

#### GET /playback/state

Get current playback state.

**Response:**
```json
{
  "state": "playing"
}
```

**Possible States:**
- `playing` - Audio is playing
- `paused` - Playback is paused

**Example:**
```bash
curl http://localhost:5721/playback/state
```

**Note:** WKMP has no "stopped" state. Paused with empty queue is functionally equivalent to stopped.

---

#### GET /playback/position

Get current playback position.

**Response:**
```json
{
  "passage_id": "7c9e6679-7425-40de-944b-e07fc1f90ae7",
  "position_ms": 45230,
  "duration_ms": 245000,
  "state": "playing"
}
```

**Example:**
```bash
curl http://localhost:5721/playback/position
```

**Response Fields:**
- `passage_id`: Currently playing passage UUID (null if nothing playing)
- `position_ms`: Current position in milliseconds
- `duration_ms`: Total passage duration in milliseconds
- `state`: Playback state (playing/paused)

**Position Tracking:**
- Updated via event-driven architecture (not polling)
- Typical latency: <50ms
- Sample-accurate: ~1ms precision

---

#### GET /playback/buffer_status

Get status of all passage buffers in memory.

**Response:**
```json
{
  "buffers": [
    {
      "passage_id": "7c9e6679-7425-40de-944b-e07fc1f90ae7",
      "status": "playing",
      "decode_progress_percent": null
    },
    {
      "passage_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
      "status": "decoding",
      "decode_progress_percent": 67.5
    },
    {
      "passage_id": "a3bb189e-8bf9-3888-9912-ace4e6543002",
      "status": "ready",
      "decode_progress_percent": null
    }
  ]
}
```

**Example:**
```bash
curl http://localhost:5721/playback/buffer_status
```

**Buffer States:**
- `decoding` - Buffer being filled by decoder (includes progress %)
- `ready` - Buffer fully decoded, ready for playback
- `playing` - Buffer currently being played
- `exhausted` - Buffer fully consumed

**Use Cases:**
- Monitor decode progress for long files
- Verify next passage ready before crossfade
- Debug playback issues

---

## Server-Sent Events (SSE)

### GET /events

Real-time event stream using Server-Sent Events protocol.

**Connection Type:** Long-lived HTTP connection
**Content-Type:** `text/event-stream`
**Encoding:** UTF-8

**Example (cURL):**
```bash
curl -N http://localhost:5721/events
```

**Example (JavaScript):**
```javascript
const eventSource = new EventSource('http://localhost:5721/events');

eventSource.addEventListener('PlaybackProgress', (event) => {
  const data = JSON.parse(event.data);
  console.log(`Position: ${data.position_ms}ms / ${data.duration_ms}ms`);
});

eventSource.addEventListener('CurrentSongChanged', (event) => {
  const data = JSON.parse(event.data);
  console.log(`Now playing song: ${data.song_id}`);
});
```

**Example (Rust):**
```rust
use eventsource_client::Client;

let client = Client::new("http://localhost:5721/events")?;
let stream = client.stream();

while let Some(event) = stream.next().await {
    match event {
        Ok(event) => {
            let event_type = event.event_type;
            let data: WkmpEvent = serde_json::from_str(&event.data)?;
            println!("Event: {:?}", data);
        }
        Err(e) => eprintln!("Error: {}", e),
    }
}
```

**Example (Python):**
```python
import sseclient
import json

response = requests.get('http://localhost:5721/events', stream=True)
client = sseclient.SSEClient(response)

for event in client.events():
    data = json.loads(event.data)
    print(f"Event: {data['type']}, {data}")
```

---

### Event Types

#### PlaybackStateChanged

Emitted when playback state changes (Playing ↔ Paused).

**Event Name:** `PlaybackStateChanged`

**Data:**
```json
{
  "type": "PlaybackStateChanged",
  "state": "playing",
  "timestamp": "2025-10-18T14:30:00.123Z"
}
```

**Fields:**
- `state`: New playback state ("playing", "paused", or "stopped")
- `timestamp`: ISO 8601 timestamp (UTC)

**Triggered By:**
- POST /playback/play
- POST /playback/pause

---

#### PassageStarted

Emitted when a new passage begins playing.

**Event Name:** `PassageStarted`

**Data:**
```json
{
  "type": "PassageStarted",
  "passage_id": "7c9e6679-7425-40de-944b-e07fc1f90ae7",
  "timestamp": "2025-10-18T14:30:00.123Z"
}
```

**Fields:**
- `passage_id`: UUID of passage that started
- `timestamp`: ISO 8601 timestamp (UTC)

**Triggered By:**
- Automatic queue advance
- POST /playback/next (skip)
- Queue becomes non-empty while playing

---

#### PassageCompleted

Emitted when a passage finishes.

**Event Name:** `PassageCompleted`

**Data:**
```json
{
  "type": "PassageCompleted",
  "passage_id": "7c9e6679-7425-40de-944b-e07fc1f90ae7",
  "completed": true,
  "timestamp": "2025-10-18T14:35:00.456Z"
}
```

**Fields:**
- `passage_id`: UUID of completed passage
- `completed`: `true` = finished naturally, `false` = skipped/interrupted
- `timestamp`: ISO 8601 timestamp (UTC)

**Triggered By:**
- Natural passage completion
- POST /playback/next (skip)

---

#### CurrentSongChanged

Emitted when playback crosses a song boundary within a passage.

**Event Name:** `CurrentSongChanged`

**Data:**
```json
{
  "type": "CurrentSongChanged",
  "passage_id": "7c9e6679-7425-40de-944b-e07fc1f90ae7",
  "song_id": "a3bb189e-8bf9-3888-9912-ace4e6543002",
  "position_ms": 120500,
  "timestamp": "2025-10-18T14:32:00.789Z"
}
```

**Fields:**
- `passage_id`: UUID of current passage
- `song_id`: UUID of new song (null = entered gap between songs)
- `position_ms`: Position where boundary crossed
- `timestamp`: ISO 8601 timestamp (UTC)

**Triggered By:**
- Crossing song boundary during playback
- POST /playback/seek (if seeking crosses boundary)

**Note:** Only emitted for multi-song passages (albums, compilations). Single-song passages never emit this event.

---

#### PlaybackProgress

Periodic position update during playback.

**Event Name:** `PlaybackProgress`

**Data:**
```json
{
  "type": "PlaybackProgress",
  "passage_id": "7c9e6679-7425-40de-944b-e07fc1f90ae7",
  "position_ms": 45000,
  "duration_ms": 245000,
  "timestamp": "2025-10-18T14:30:45.123Z"
}
```

**Fields:**
- `passage_id`: UUID of current passage
- `position_ms`: Current position in milliseconds
- `duration_ms`: Total passage duration
- `timestamp`: ISO 8601 timestamp (UTC)

**Frequency:** Every ~5 seconds (configurable via `playback_progress_interval_ms` setting)

**Also Triggered By:**
- POST /playback/play
- POST /playback/pause
- POST /playback/seek

---

#### QueueChanged

Emitted when queue is modified.

**Event Name:** `QueueChanged`

**Data:**
```json
{
  "type": "QueueChanged",
  "timestamp": "2025-10-18T14:30:00.123Z"
}
```

**Fields:**
- `timestamp`: ISO 8601 timestamp (UTC)

**Triggered By:**
- POST /playback/enqueue
- DELETE /playback/queue/:id
- POST /playback/queue/clear
- POST /playback/queue/reorder
- POST /playback/next (removes current)

**Note:** Event does not include queue contents. Call GET /playback/queue to fetch updated queue.

---

#### VolumeChanged

Emitted when volume level changes.

**Event Name:** `VolumeChanged`

**Data:**
```json
{
  "type": "VolumeChanged",
  "volume": 0.75,
  "timestamp": "2025-10-18T14:30:00.123Z"
}
```

**Fields:**
- `volume`: New volume level (0.0-1.0 scale, system internal format)
- `timestamp`: ISO 8601 timestamp (UTC)

**Triggered By:**
- POST /audio/volume

**Note:** Volume is in 0.0-1.0 system scale, not 0-100 user scale. Multiply by 100 for display.

---

## Error Handling

### HTTP Status Codes

| Code | Meaning | Typical Causes |
|------|---------|----------------|
| 200 OK | Request successful | Normal response |
| 204 No Content | Request successful, no body | DELETE operations |
| 400 Bad Request | Invalid input | Malformed JSON, invalid values |
| 404 Not Found | Resource not found | Queue entry doesn't exist |
| 500 Internal Server Error | Server error | Database error, decoder crash |
| 501 Not Implemented | Feature not implemented | POST /playback/previous |

### Error Response Format

```json
{
  "status": "error: <description>"
}
```

**Example:**
```json
{
  "status": "error: Queue entry not found"
}
```

### Common Error Scenarios

**Invalid Volume:**
```bash
curl -X POST http://localhost:5721/audio/volume \
  -H "Content-Type: application/json" \
  -d '{"volume":150}'
# Returns 400 Bad Request
```

**Queue Entry Not Found:**
```bash
curl -X DELETE http://localhost:5721/playback/queue/invalid-uuid
# Returns 404 Not Found
```

**Empty Queue Skip:**
```bash
curl -X POST http://localhost:5721/playback/next
# Returns 500 Internal Server Error
# {"status":"error: No passage to skip - queue is empty"}
```

---

## Examples

### Complete Workflow Example

```bash
# 1. Check module health
curl http://localhost:5721/health

# 2. List available audio devices
curl http://localhost:5721/audio/devices

# 3. Set volume to 75%
curl -X POST http://localhost:5721/audio/volume \
  -H "Content-Type: application/json" \
  -d '{"volume":75}'

# 4. Enqueue first song
curl -X POST http://localhost:5721/playback/enqueue \
  -H "Content-Type: application/json" \
  -d '{"file_path":"albums/artist/song1.mp3"}'

# 5. Enqueue second song
curl -X POST http://localhost:5721/playback/enqueue \
  -H "Content-Type: application/json" \
  -d '{"file_path":"albums/artist/song2.mp3"}'

# 6. Check queue
curl http://localhost:5721/playback/queue

# 7. Start playback
curl -X POST http://localhost:5721/playback/play

# 8. Monitor position
curl http://localhost:5721/playback/position

# 9. Seek to 30 seconds
curl -X POST http://localhost:5721/playback/seek \
  -H "Content-Type: application/json" \
  -d '{"position_ms":30000}'

# 10. Skip to next
curl -X POST http://localhost:5721/playback/next

# 11. Pause playback
curl -X POST http://localhost:5721/playback/pause
```

### JavaScript Web Application Example

```javascript
// Connect to SSE event stream
const eventSource = new EventSource('http://localhost:5721/events');

// Handle playback progress
eventSource.addEventListener('PlaybackProgress', (event) => {
  const data = JSON.parse(event.data);
  updateProgressBar(data.position_ms, data.duration_ms);
});

// Handle state changes
eventSource.addEventListener('PlaybackStateChanged', (event) => {
  const data = JSON.parse(event.data);
  updatePlayPauseButton(data.state);
});

// Handle queue changes
eventSource.addEventListener('QueueChanged', async (event) => {
  const queue = await fetch('http://localhost:5721/playback/queue')
    .then(r => r.json());
  updateQueueDisplay(queue);
});

// Control playback
async function play() {
  await fetch('http://localhost:5721/playback/play', { method: 'POST' });
}

async function pause() {
  await fetch('http://localhost:5721/playback/pause', { method: 'POST' });
}

async function skip() {
  await fetch('http://localhost:5721/playback/next', { method: 'POST' });
}

async function setVolume(percent) {
  await fetch('http://localhost:5721/audio/volume', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ volume: percent })
  });
}
```

### Rust Application Example

```rust
use reqwest::Client;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let base_url = "http://localhost:5721";

    // Enqueue a file
    let response = client
        .post(&format!("{}/playback/enqueue", base_url))
        .json(&json!({
            "file_path": "albums/artist/song.mp3"
        }))
        .send()
        .await?;

    let enqueue_response: EnqueueResponse = response.json().await?;
    println!("Enqueued: {}", enqueue_response.queue_entry_id);

    // Start playback
    client
        .post(&format!("{}/playback/play", base_url))
        .send()
        .await?;

    // Monitor events
    use eventsource_client::Client as SSEClient;
    let sse_client = SSEClient::new(&format!("{}/events", base_url))?;
    let mut stream = sse_client.stream();

    while let Some(event) = stream.next().await {
        match event {
            Ok(event) => {
                let data: WkmpEvent = serde_json::from_str(&event.data)?;
                match data {
                    WkmpEvent::PlaybackProgress { position_ms, duration_ms, .. } => {
                        println!("Position: {}ms / {}ms", position_ms, duration_ms);
                    }
                    WkmpEvent::CurrentSongChanged { song_id, .. } => {
                        println!("Song changed: {:?}", song_id);
                    }
                    _ => {}
                }
            }
            Err(e) => eprintln!("SSE error: {}", e),
        }
    }

    Ok(())
}
```

---

## Additional Resources

- **OpenAPI Specification:** See `openapi.yaml` for machine-readable API spec
- **Architecture Documentation:** See `OVERALL_IMPLEMENTATION_REVIEW.md`
- **Event-Driven Architecture:** See `ARCHITECTURAL_REVIEW-event_driven_implementation.md`
- **WKMP Project Documentation:** See `/docs` directory in repository root

---

**Last Updated:** 2025-10-18
**API Version:** 1.0.0
**Module:** wkmp-ap (Audio Player)
