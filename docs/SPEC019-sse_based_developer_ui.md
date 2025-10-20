# SPEC019: SSE-Based Developer UI Communication

**Document Type:** Tier 2 - Design Specification
**Status:** Proposed
**Created:** 2025-10-20
**Parent Documents:** REQ001 (Developer UI), SPEC007 (API Design)
**Related:** SPEC001 (Architecture), IMPL003 (Project Structure)

---

## 1. Overview

### 1.1 Purpose

Replace the current polling-based developer UI with a **Server-Sent Events (SSE) only** communication pattern for all real-time updates. This eliminates the observed "stuck queue" bug and provides a unified, debuggable communication channel.

### 1.2 Problem Statement

**Current Issues:**
1. ✅ **Observed:** Queue display gets stuck, requiring page/server restart
2. ✅ **Observed:** No playback position updates (would require additional polling)
3. ✅ **Observed:** No volume change synchronization across browser tabs
4. ✅ **Developer need:** No visibility into system events for debugging

**Root Cause Analysis:**
The polling mechanism (`setInterval()` every 2 seconds) can fail silently if:
- JavaScript execution paused (browser throttling)
- Network request hangs/times out
- Error in fetch handler prevents retry
- Browser tab state issues

### 1.3 Goals

**Primary:**
1. Replace polling with SSE for queue updates
2. Add playback position updates via SSE (1/second)
3. Add volume change notifications via SSE
4. Add raw SSE event viewer for debugging

**Secondary:**
5. Eliminate all polling from developer UI
6. Provide single, unified communication channel
7. Make system behavior transparent and debuggable

### 1.4 Non-Goals

- Production multi-user UI (different requirements)
- WebSocket bidirectional communication (SSE sufficient)
- Guaranteed delivery semantics (best-effort acceptable for dev UI)

---

## 2. Requirements

### Functional Requirements

**[SSE-UI-010] Unified Event Stream**
- All real-time updates SHALL use single SSE connection at `/events`
- NO polling mechanisms SHALL be used for any dynamic data
- Connection SHALL auto-reconnect on disconnection

**[SSE-UI-020] Queue Updates**
- Queue changes SHALL trigger `QueueStateUpdate` event
- Event SHALL contain complete current queue state
- Updates SHALL be sent immediately on any queue modification

**[SSE-UI-030] Playback Position Updates**
- Position updates SHALL be sent via `PlaybackPosition` event
- Updates SHALL be sent every 1 second during playback
- Event SHALL include: position_ms, duration_ms, passage_id

**[SSE-UI-040] Volume Updates**
- Volume changes SHALL trigger `VolumeChanged` event
- Event SHALL include current volume level (0.0-1.0)
- Updates SHALL be broadcast to all connected clients

**[SSE-UI-050] Initial State on Connection**
- On SSE connection, server SHALL immediately send current state
- Initial events: QueueStateUpdate, PlaybackPosition, VolumeChanged
- Client SHALL NOT need to make separate HTTP requests for initial data

**[SSE-UI-060] Raw Event Viewer**
- Developer UI SHALL display all incoming SSE events
- Display SHALL show: timestamp, event type, raw JSON payload
- Display SHALL auto-scroll and support copy/paste for debugging

### Non-Functional Requirements

**[SSE-UI-NFR-010] Latency**
- Event delivery SHALL be <100ms from trigger to UI update
- Position updates SHALL have jitter <50ms

**[SSE-UI-NFR-020] Reconnection**
- SSE connection SHALL auto-reconnect on failure
- Reconnection SHALL trigger full state refresh
- Maximum reconnect delay: 5 seconds (exponential backoff)

**[SSE-UI-NFR-030] Memory**
- Event viewer SHALL limit history to last 100 events
- Older events SHALL be discarded automatically

**[SSE-UI-NFR-040] Debuggability**
- All events SHALL be logged to browser console in debug mode
- Event format SHALL be human-readable JSON
- Event types SHALL be clearly named and documented

---

## 3. Architecture

### 3.1 Event Types

**New Event Definitions** (add to `wkmp-common/src/events.rs`):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type")]
pub enum WkmpEvent {
    // Existing events...
    PassageStarted { /* ... */ },
    PassageCompleted { /* ... */ },
    CrossfadeStarted { /* ... */ },
    QueueChanged { timestamp: DateTime<Utc> },

    // NEW: Full queue state update
    QueueStateUpdate {
        timestamp: DateTime<Utc>,
        queue: Vec<QueueEntryInfo>,
    },

    // NEW: Playback position (sent every 1s during playback)
    PlaybackPosition {
        timestamp: DateTime<Utc>,
        passage_id: Uuid,
        position_ms: u64,
        duration_ms: u64,
        playing: bool,
    },

    // NEW: Volume change notification
    VolumeChanged {
        timestamp: DateTime<Utc>,
        volume: f32,  // 0.0 to 1.0
    },

    // NEW: Initial state on connection (sent once per SSE client)
    InitialState {
        timestamp: DateTime<Utc>,
        queue: Vec<QueueEntryInfo>,
        position: Option<PlaybackPositionInfo>,
        volume: f32,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueEntryInfo {
    pub queue_entry_id: Uuid,
    pub passage_id: Option<Uuid>,
    pub file_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackPositionInfo {
    pub passage_id: Uuid,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub playing: bool,
}
```

### 3.2 SSE Server Architecture

**Modified SSE Handler** (`wkmp-ap/src/api/sse.rs`):

```rust
use axum::response::sse::{Event, Sse};
use futures::stream::{self, Stream};
use std::convert::Infallible;
use std::time::Duration;
use tokio_stream::StreamExt;

/// SSE endpoint with initial state injection
pub async fn event_stream(
    State(ctx): State<AppContext>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    // Subscribe to event broadcast
    let mut rx = ctx.state.subscribe_events();

    // Clone context for initial state emission
    let engine = ctx.engine.clone();
    let state_clone = ctx.state.clone();

    // Create stream that sends initial state, then ongoing events
    let stream = async_stream::stream! {
        // === INITIAL STATE EMISSION ===
        // [SSE-UI-050] Send current state immediately on connection

        // Get current queue
        let queue_entries = engine.get_queue_entries().await;
        let queue_info: Vec<QueueEntryInfo> = queue_entries.into_iter()
            .map(|e| QueueEntryInfo {
                queue_entry_id: e.queue_entry_id,
                passage_id: e.passage_id,
                file_path: e.file_path.to_string_lossy().to_string(),
            })
            .collect();

        // Get current playback position
        let position = engine.get_playback_position().await;
        let position_info = position.map(|p| PlaybackPositionInfo {
            passage_id: p.passage_id,
            position_ms: p.position_ms,
            duration_ms: p.duration_ms,
            playing: p.playing,
        });

        // Get current volume
        let volume = engine.get_volume().await;

        // Send InitialState event
        let initial_event = WkmpEvent::InitialState {
            timestamp: Utc::now(),
            queue: queue_info,
            position: position_info,
            volume,
        };

        let event_json = serde_json::to_string(&initial_event)
            .unwrap_or_else(|_| "{}".to_string());

        yield Ok(Event::default()
            .event("InitialState")
            .data(event_json));

        // === ONGOING EVENT STREAM ===
        // [SSE-UI-010] Stream all subsequent events

        loop {
            tokio::select! {
                // Heartbeat every 15 seconds
                _ = tokio::time::sleep(Duration::from_secs(15)) => {
                    yield Ok(Event::default().comment("heartbeat"));
                }

                // Broadcast events
                Ok(event) = rx.recv() => {
                    let event_type = match &event {
                        WkmpEvent::PassageStarted { .. } => "PassageStarted",
                        WkmpEvent::PassageCompleted { .. } => "PassageCompleted",
                        WkmpEvent::CrossfadeStarted { .. } => "CrossfadeStarted",
                        WkmpEvent::QueueChanged { .. } => "QueueChanged",
                        WkmpEvent::QueueStateUpdate { .. } => "QueueStateUpdate",
                        WkmpEvent::PlaybackPosition { .. } => "PlaybackPosition",
                        WkmpEvent::VolumeChanged { .. } => "VolumeChanged",
                        WkmpEvent::InitialState { .. } => "InitialState",
                    };

                    let event_json = serde_json::to_string(&event)
                        .unwrap_or_else(|_| "{}".to_string());

                    yield Ok(Event::default()
                        .event(event_type)
                        .data(event_json));
                }
            }
        }
    };

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("heartbeat")
    )
}
```

### 3.3 Event Emission Points

**Queue State Updates** - Emit `QueueStateUpdate` after:
1. `POST /playback/enqueue` (handlers.rs:309)
2. `DELETE /playback/queue/:id` (handlers.rs:344)
3. `POST /playback/queue/clear` (engine.rs:clear_queue)
4. `POST /playback/queue/reorder` (handlers.rs:613)
5. `queue.advance()` in engine (engine.rs:1476) ← **CRITICAL for crossfade completion**

**Implementation Pattern:**
```rust
// After any queue modification
let queue_entries = ctx.engine.get_queue_entries().await;
let queue_info = convert_to_queue_info(queue_entries);

ctx.state.broadcast_event(WkmpEvent::QueueStateUpdate {
    timestamp: Utc::now(),
    queue: queue_info,
});
```

**Playback Position Updates** - New periodic task in engine:
```rust
// In playback/engine.rs, new background task
async fn position_update_task(
    engine: Arc<PlaybackEngine>,
    state: Arc<AppState>,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(1));

    loop {
        interval.tick().await;

        // Get current position
        if let Some(position) = engine.get_playback_position().await {
            state.broadcast_event(WkmpEvent::PlaybackPosition {
                timestamp: Utc::now(),
                passage_id: position.passage_id,
                position_ms: position.position_ms,
                duration_ms: position.duration_ms,
                playing: position.playing,
            });
        }
    }
}
```

**Volume Updates** - Emit after volume change:
```rust
// In set_volume handler (handlers.rs:187)
pub async fn set_volume(
    State(ctx): State<AppContext>,
    Json(req): Json<VolumeRequest>,
) -> Result<StatusCode, (StatusCode, Json<StatusResponse>)> {
    ctx.engine.set_volume(req.volume).await;

    // NEW: Broadcast volume change
    ctx.state.broadcast_event(WkmpEvent::VolumeChanged {
        timestamp: Utc::now(),
        volume: req.volume,
    });

    Ok(StatusCode::NO_CONTENT)
}
```

### 3.4 Frontend Architecture

**New Developer UI Structure** (`wkmp-ap/src/api/developer_ui.html`):

```html
<!DOCTYPE html>
<html>
<head>
    <title>WKMP Audio Player - Developer Interface</title>
    <style>
        /* Existing styles... */

        /* NEW: Event viewer styles */
        #event-viewer {
            background: #1a1a1a;
            border: 1px solid #333;
            border-radius: 4px;
            height: 300px;
            overflow-y: auto;
            font-family: 'Courier New', monospace;
            font-size: 12px;
            padding: 10px;
        }

        .event-entry {
            margin: 4px 0;
            padding: 8px;
            background: #222;
            border-left: 3px solid #555;
            border-radius: 2px;
        }

        .event-entry.InitialState { border-left-color: #4CAF50; }
        .event-entry.QueueStateUpdate { border-left-color: #2196F3; }
        .event-entry.PlaybackPosition { border-left-color: #FF9800; }
        .event-entry.VolumeChanged { border-left-color: #9C27B0; }
        .event-entry.PassageStarted { border-left-color: #00BCD4; }
        .event-entry.PassageCompleted { border-left-color: #F44336; }

        .event-timestamp {
            color: #888;
            font-size: 10px;
        }

        .event-type {
            color: #FFA726;
            font-weight: bold;
        }

        .event-data {
            color: #CCC;
            white-space: pre-wrap;
            margin-top: 4px;
        }

        .sse-status {
            padding: 4px 8px;
            border-radius: 3px;
            font-size: 12px;
            display: inline-block;
        }

        .sse-status.connected {
            background: #4CAF50;
            color: white;
        }

        .sse-status.disconnected {
            background: #F44336;
            color: white;
        }

        .sse-status.connecting {
            background: #FF9800;
            color: white;
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>WKMP Audio Player - Developer Interface</h1>

        <!-- SSE Connection Status -->
        <div style="margin-bottom: 20px;">
            <strong>SSE Connection:</strong>
            <span id="sse-status" class="sse-status disconnected">Disconnected</span>
            <span id="sse-reconnect-info" style="margin-left: 10px; color: #888;"></span>
        </div>

        <!-- Existing UI sections... -->
        <div class="section">
            <h2>Playback Status</h2>
            <!-- Updated by SSE PlaybackPosition events -->
            <div id="status-display">
                <div>Playing: <span id="playing-status">-</span></div>
                <div>Current Passage: <span id="current-passage">-</span></div>
                <div>Position: <span id="position">-</span></div>
                <div>Duration: <span id="duration">-</span></div>
            </div>
        </div>

        <div class="section">
            <h2>Volume</h2>
            <!-- Updated by SSE VolumeChanged events -->
            <div>
                <input type="range" id="volume-slider" min="0" max="100" value="50">
                <span id="volume-display">50%</span>
            </div>
        </div>

        <div class="section">
            <h2>Queue Contents</h2>
            <!-- Updated by SSE QueueStateUpdate events -->
            <div id="queue-display" class="queue-empty">No queue data</div>
        </div>

        <!-- NEW: Raw Event Viewer -->
        <div class="section">
            <h2>SSE Event Log (Raw)</h2>
            <div style="margin-bottom: 10px;">
                <button onclick="clearEventLog()">Clear Log</button>
                <button onclick="exportEventLog()">Export to Clipboard</button>
                <label>
                    <input type="checkbox" id="auto-scroll" checked>
                    Auto-scroll
                </label>
            </div>
            <div id="event-viewer"></div>
        </div>
    </div>

    <script>
        const API_BASE = 'http://localhost:5721';
        let eventSource = null;
        let reconnectAttempts = 0;
        let eventLog = [];
        const MAX_LOG_ENTRIES = 100;

        // === SSE CONNECTION MANAGEMENT ===

        function connectSSE() {
            updateSSEStatus('connecting');

            eventSource = new EventSource(`${API_BASE}/events`);

            eventSource.onopen = () => {
                console.log('[SSE] Connection opened');
                reconnectAttempts = 0;
                updateSSEStatus('connected');
            };

            eventSource.onerror = (error) => {
                console.error('[SSE] Connection error:', error);
                updateSSEStatus('disconnected');

                // Auto-reconnect with exponential backoff
                const delay = Math.min(5000, 1000 * Math.pow(2, reconnectAttempts));
                reconnectAttempts++;

                document.getElementById('sse-reconnect-info').textContent =
                    `Reconnecting in ${delay/1000}s... (attempt ${reconnectAttempts})`;

                setTimeout(() => {
                    console.log(`[SSE] Reconnecting (attempt ${reconnectAttempts})...`);
                    connectSSE();
                }, delay);
            };

            // === EVENT HANDLERS ===

            // Initial state (sent once on connection)
            eventSource.addEventListener('InitialState', (e) => {
                const data = JSON.parse(e.data);
                logEvent('InitialState', data);

                // Update all UI components with initial state
                if (data.queue) {
                    renderQueue(data.queue);
                }
                if (data.position) {
                    updatePosition(data.position);
                }
                if (data.volume !== undefined) {
                    updateVolume(data.volume);
                }
            });

            // Queue state updates
            eventSource.addEventListener('QueueStateUpdate', (e) => {
                const data = JSON.parse(e.data);
                logEvent('QueueStateUpdate', data);
                renderQueue(data.queue);
            });

            // Playback position (every 1 second)
            eventSource.addEventListener('PlaybackPosition', (e) => {
                const data = JSON.parse(e.data);
                logEvent('PlaybackPosition', data, false); // Don't log every position update (too noisy)
                updatePosition(data);
            });

            // Volume changes
            eventSource.addEventListener('VolumeChanged', (e) => {
                const data = JSON.parse(e.data);
                logEvent('VolumeChanged', data);
                updateVolume(data.volume);
            });

            // Passage lifecycle events
            eventSource.addEventListener('PassageStarted', (e) => {
                const data = JSON.parse(e.data);
                logEvent('PassageStarted', data);
            });

            eventSource.addEventListener('PassageCompleted', (e) => {
                const data = JSON.parse(e.data);
                logEvent('PassageCompleted', data);
            });

            eventSource.addEventListener('CrossfadeStarted', (e) => {
                const data = JSON.parse(e.data);
                logEvent('CrossfadeStarted', data);
            });

            // Generic message handler (fallback)
            eventSource.onmessage = (e) => {
                try {
                    const data = JSON.parse(e.data);
                    logEvent(data.event_type || 'Unknown', data);
                } catch (err) {
                    console.warn('[SSE] Failed to parse event:', e.data);
                }
            };
        }

        function updateSSEStatus(status) {
            const statusEl = document.getElementById('sse-status');
            statusEl.className = `sse-status ${status}`;
            statusEl.textContent = status.charAt(0).toUpperCase() + status.slice(1);

            if (status === 'connected') {
                document.getElementById('sse-reconnect-info').textContent = '';
            }
        }

        // === EVENT LOGGING ===

        function logEvent(eventType, data, display = true) {
            const timestamp = new Date().toISOString();
            const entry = { timestamp, eventType, data };

            // Add to log (with size limit)
            eventLog.push(entry);
            if (eventLog.length > MAX_LOG_ENTRIES) {
                eventLog.shift();
            }

            // Display in event viewer (if enabled)
            if (display) {
                displayEvent(entry);
            }

            // Console log in debug mode
            console.log(`[SSE Event] ${eventType}:`, data);
        }

        function displayEvent(entry) {
            const viewer = document.getElementById('event-viewer');

            const eventDiv = document.createElement('div');
            eventDiv.className = `event-entry ${entry.eventType}`;

            const timestampSpan = document.createElement('div');
            timestampSpan.className = 'event-timestamp';
            timestampSpan.textContent = entry.timestamp.split('T')[1].split('.')[0];

            const typeSpan = document.createElement('div');
            typeSpan.className = 'event-type';
            typeSpan.textContent = entry.eventType;

            const dataDiv = document.createElement('div');
            dataDiv.className = 'event-data';
            dataDiv.textContent = JSON.stringify(entry.data, null, 2);

            eventDiv.appendChild(timestampSpan);
            eventDiv.appendChild(typeSpan);
            eventDiv.appendChild(dataDiv);

            viewer.appendChild(eventDiv);

            // Auto-scroll if enabled
            if (document.getElementById('auto-scroll').checked) {
                viewer.scrollTop = viewer.scrollHeight;
            }

            // Limit DOM size (keep last 100 entries)
            while (viewer.children.length > MAX_LOG_ENTRIES) {
                viewer.removeChild(viewer.firstChild);
            }
        }

        function clearEventLog() {
            eventLog = [];
            document.getElementById('event-viewer').innerHTML = '';
        }

        function exportEventLog() {
            const logText = JSON.stringify(eventLog, null, 2);
            navigator.clipboard.writeText(logText).then(() => {
                alert('Event log copied to clipboard!');
            }).catch(err => {
                console.error('Failed to copy log:', err);
                alert('Failed to copy log. Check console.');
            });
        }

        // === UI UPDATE FUNCTIONS ===

        function renderQueue(queue) {
            const queueEl = document.getElementById('queue-display');

            if (!queue || queue.length === 0) {
                queueEl.innerHTML = '<div class="queue-empty">Queue is empty</div>';
                return;
            }

            queueEl.innerHTML = queue.map(item => `
                <div class="queue-item">
                    <div>${item.file_path || 'Unknown'}</div>
                    <div class="queue-item-id">ID: ${item.queue_entry_id || 'N/A'}</div>
                </div>
            `).join('');
        }

        function updatePosition(position) {
            document.getElementById('playing-status').textContent =
                position.playing ? 'Yes' : 'No';
            document.getElementById('current-passage').textContent =
                position.passage_id ? position.passage_id.substring(0, 8) + '...' : 'None';

            const posSeconds = Math.floor(position.position_ms / 1000);
            const durSeconds = Math.floor(position.duration_ms / 1000);

            document.getElementById('position').textContent =
                `${formatTime(posSeconds)} / ${formatTime(durSeconds)}`;
            document.getElementById('duration').textContent =
                formatTime(durSeconds);
        }

        function updateVolume(volume) {
            const percent = Math.round(volume * 100);
            document.getElementById('volume-slider').value = percent;
            document.getElementById('volume-display').textContent = `${percent}%`;
        }

        function formatTime(seconds) {
            const mins = Math.floor(seconds / 60);
            const secs = seconds % 60;
            return `${mins}:${secs.toString().padStart(2, '0')}`;
        }

        // === VOLUME CONTROL ===

        document.getElementById('volume-slider').addEventListener('change', async (e) => {
            const volume = parseInt(e.target.value) / 100;

            try {
                await fetch(`${API_BASE}/audio/volume`, {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({ volume })
                });
                // UI will update via SSE VolumeChanged event
            } catch (err) {
                console.error('Failed to set volume:', err);
            }
        });

        // === INITIALIZATION ===

        window.addEventListener('load', () => {
            console.log('[WKMP] Developer UI loaded');
            connectSSE();
        });

        // Cleanup on page unload
        window.addEventListener('beforeunload', () => {
            if (eventSource) {
                eventSource.close();
            }
        });
    </script>
</body>
</html>
```

---

## 4. Implementation Plan

### Phase 1: Event Type Definitions (30 min)

**File:** `wkmp-common/src/events.rs`

- [ ] Add `QueueStateUpdate` event variant
- [ ] Add `PlaybackPosition` event variant
- [ ] Add `VolumeChanged` event variant
- [ ] Add `InitialState` event variant
- [ ] Add helper structs: `QueueEntryInfo`, `PlaybackPositionInfo`

### Phase 2: SSE Server Enhancement (2 hours)

**File:** `wkmp-ap/src/api/sse.rs`

- [ ] Modify `event_stream()` to send `InitialState` on connection
- [ ] Add event type name extraction for all new events
- [ ] Test SSE connection with `curl` or EventSource

**New Engine Methods:**

**File:** `wkmp-ap/src/playback/engine.rs`

- [ ] Add `get_playback_position()` → `Option<PlaybackPositionInfo>`
- [ ] Add `get_volume()` → `f32`
- [ ] Create `position_update_task()` background task (1/second emission)

### Phase 3: Event Emission Points (2 hours)

**Queue Updates:**

**File:** `wkmp-ap/src/api/handlers.rs`

- [ ] `enqueue_passage()` → emit `QueueStateUpdate` (line 309)
- [ ] `remove_from_queue()` → emit `QueueStateUpdate` (line 344)
- [ ] `reorder_queue_entry()` → emit `QueueStateUpdate` (line 613)

**File:** `wkmp-ap/src/playback/engine.rs`

- [ ] `clear_queue()` → emit `QueueStateUpdate`
- [ ] `queue.advance()` → emit `QueueStateUpdate` (line 1476 - **critical for crossfade**)

**Volume Updates:**

**File:** `wkmp-ap/src/api/handlers.rs`

- [ ] `set_volume()` → emit `VolumeChanged` (line 187)

**Position Updates:**

**File:** `wkmp-ap/src/playback/engine.rs`

- [ ] Spawn `position_update_task()` in `start()` method
- [ ] Emit `PlaybackPosition` every 1 second during playback

### Phase 4: Frontend Rewrite (3 hours)

**File:** `wkmp-ap/src/api/developer_ui.html`

- [ ] Remove all `setInterval()` polling code
- [ ] Implement SSE connection management with auto-reconnect
- [ ] Add event handlers for all event types
- [ ] Implement event log viewer UI
- [ ] Add export/clear functionality for event log
- [ ] Style event viewer with color-coded event types

### Phase 5: Testing (2 hours)

**Manual Tests:**

- [ ] SSE connection establishes and sends `InitialState`
- [ ] Queue updates reflect immediately when enqueuing/removing
- [ ] Position updates every 1 second during playback
- [ ] Volume changes propagate to all open browser tabs
- [ ] Event log displays all events with correct formatting
- [ ] Reconnection works after network disconnect
- [ ] Multiple browser tabs receive same events

**Edge Case Tests:**

- [ ] SSE connection drop during playback
- [ ] Browser tab backgrounded/foregrounded
- [ ] Rapid queue changes (add 10 files quickly)
- [ ] Volume slider drag (many rapid changes)
- [ ] Server restart while UI open

### Phase 6: Documentation (1 hour)

- [ ] Update SPEC007 (API Design) with new event types
- [ ] Document event viewer usage for developers
- [ ] Add troubleshooting section for SSE connection issues

**Total Estimated Time:** ~10.5 hours

---

## 5. Event Format Examples

### InitialState Event

```json
{
  "event_type": "InitialState",
  "timestamp": "2025-10-20T17:30:00.123Z",
  "queue": [
    {
      "queue_entry_id": "12e7b586-fb05-44fc-8900-2deca3584445",
      "passage_id": "a1b2c3d4-e5f6-7890-1234-567890abcdef",
      "file_path": "/home/user/Music/song1.mp3"
    },
    {
      "queue_entry_id": "865a02f9-1ec6-4f17-8cfc-aec7145e6128",
      "passage_id": "b2c3d4e5-f678-9012-3456-7890abcdef12",
      "file_path": "/home/user/Music/song2.mp3"
    }
  ],
  "position": {
    "passage_id": "a1b2c3d4-e5f6-7890-1234-567890abcdef",
    "position_ms": 45230,
    "duration_ms": 182000,
    "playing": true
  },
  "volume": 0.65
}
```

### QueueStateUpdate Event

```json
{
  "event_type": "QueueStateUpdate",
  "timestamp": "2025-10-20T17:30:15.456Z",
  "queue": [
    {
      "queue_entry_id": "865a02f9-1ec6-4f17-8cfc-aec7145e6128",
      "passage_id": "b2c3d4e5-f678-9012-3456-7890abcdef12",
      "file_path": "/home/user/Music/song2.mp3"
    },
    {
      "queue_entry_id": "99f3a7b2-4c1d-4e8f-9a3b-2c1d4e8f9a3b",
      "passage_id": "c3d4e5f6-7890-1234-5678-90abcdef1234",
      "file_path": "/home/user/Music/song3.mp3"
    }
  ]
}
```

### PlaybackPosition Event (sent every 1s)

```json
{
  "event_type": "PlaybackPosition",
  "timestamp": "2025-10-20T17:30:16.789Z",
  "passage_id": "a1b2c3d4-e5f6-7890-1234-567890abcdef",
  "position_ms": 46230,
  "duration_ms": 182000,
  "playing": true
}
```

### VolumeChanged Event

```json
{
  "event_type": "VolumeChanged",
  "timestamp": "2025-10-20T17:30:20.123Z",
  "volume": 0.75
}
```

---

## 6. Benefits

### 6.1 Fixes Observed Bug

**Problem:** Queue display gets stuck, requires page refresh
**Solution:** SSE pushes updates immediately when queue changes
**Impact:** No polling means no stuck state - queue always reflects server reality

### 6.2 Unified Communication

**Before:** Multiple mechanisms (polling, SSE for events, future needs)
**After:** Single SSE stream for everything
**Impact:** Simpler architecture, easier debugging, less code

### 6.3 Developer Visibility

**New Feature:** Raw event viewer shows all system activity
**Use Cases:**
- Debug timing issues (see exact event sequence)
- Understand crossfade flow (see completion events)
- Monitor position updates (verify 1/second timing)
- Export event log for bug reports

**Example Debugging Session:**
```
17:30:00.123  InitialState       {"queue": [...], "position": {...}}
17:30:01.456  PlaybackPosition   {"position_ms": 45230, "playing": true}
17:30:02.789  PlaybackPosition   {"position_ms": 46230, "playing": true}
17:30:05.234  QueueStateUpdate   {"queue": [removed first entry]}
17:30:06.567  PlaybackPosition   {"position_ms": 48230, "playing": true}
                                  ↑ Clear 1-second interval
```

### 6.4 Multi-Tab Synchronization

**Feature:** All open browser tabs receive same events
**Use Case:** Developer has UI open in 2 monitors
- Change volume in Tab A
- Tab B updates immediately
- See position updates in sync across tabs

### 6.5 No Polling Overhead

**Eliminated:**
- No timers firing in JavaScript
- No periodic HTTP requests
- No potential for timer drift/throttling

**Result:** Zero overhead when nothing changing

---

## 7. Edge Cases and Handling

### 7.1 SSE Connection Loss

**Scenario:** WiFi drops for 30 seconds during playback

**Handling:**
1. EventSource automatically attempts reconnect
2. Frontend shows "Disconnected" status
3. Reconnection triggers new `InitialState` event
4. UI refreshes with current state
5. Event log shows: `[Disconnected] ... [Reconnecting] ... [Connected] InitialState`

**User sees:** Brief disconnection notice, then immediate recovery with correct state

### 7.2 Browser Tab Backgrounding

**Scenario:** User switches to another tab for 10 minutes

**Potential Issue:** Some browsers throttle background tabs

**Mitigation:**
- SSE connections typically NOT throttled (unlike timers)
- Page Visibility API can detect when tab foregrounded
- On foreground: Check event log timestamp, warn if stale

**Code:**
```javascript
document.addEventListener('visibilitychange', () => {
    if (!document.hidden) {
        // Tab now visible - check if events recent
        const lastEventTime = eventLog[eventLog.length - 1]?.timestamp;
        if (lastEventTime) {
            const age = Date.now() - new Date(lastEventTime);
            if (age > 60000) { // 1 minute old
                console.warn('[SSE] Events may be stale, connection age:', age);
            }
        }
    }
});
```

### 7.3 Rapid Queue Changes

**Scenario:** User enqueues 20 files in quick succession

**Handling:**
- Each enqueue emits `QueueStateUpdate` with full queue
- 20 events sent in ~1 second
- Frontend debounces rendering (update DOM max once per 100ms)

**Code:**
```javascript
let queueUpdateTimeout = null;
let pendingQueueData = null;

eventSource.addEventListener('QueueStateUpdate', (e) => {
    const data = JSON.parse(e.data);
    pendingQueueData = data.queue;

    // Debounce rendering
    if (queueUpdateTimeout) clearTimeout(queueUpdateTimeout);
    queueUpdateTimeout = setTimeout(() => {
        renderQueue(pendingQueueData);
        pendingQueueData = null;
    }, 100);
});
```

### 7.4 Volume Slider Dragging

**Scenario:** User drags volume slider continuously

**Handling:**
- Slider `change` event (not `input`) only fires on release
- Single volume update per drag
- SSE broadcasts to other tabs

**Alternative (if input events desired):**
```javascript
let volumeUpdateTimeout = null;

volumeSlider.addEventListener('input', (e) => {
    // Debounce to avoid flooding server
    if (volumeUpdateTimeout) clearTimeout(volumeUpdateTimeout);
    volumeUpdateTimeout = setTimeout(() => {
        updateVolumeOnServer(e.target.value);
    }, 300);
});
```

### 7.5 Server Restart

**Scenario:** Developer restarts wkmp-ap server

**Handling:**
1. All SSE connections close
2. Frontend enters reconnection loop
3. Server starts, SSE endpoint available
4. Connections reestablish
5. `InitialState` sent with fresh data
6. UI reflects new server state

**User Experience:**
- "Disconnected" status for ~5 seconds
- "Connected" status when recovered
- Queue/position reflect server restart

---

## 8. Testing Strategy

### 8.1 Unit Tests

**SSE Event Emission:**
```rust
#[tokio::test]
async fn test_queue_update_emits_event() {
    let ctx = create_test_context().await;
    let mut rx = ctx.state.subscribe_events();

    // Enqueue passage
    ctx.engine.enqueue_file(PathBuf::from("test.mp3")).await.unwrap();

    // Verify event received
    let event = rx.recv().await.unwrap();
    match event {
        WkmpEvent::QueueStateUpdate { queue, .. } => {
            assert_eq!(queue.len(), 1);
        }
        _ => panic!("Expected QueueStateUpdate event"),
    }
}
```

**Position Update Task:**
```rust
#[tokio::test]
async fn test_position_updates_every_second() {
    let ctx = create_test_context().await;
    let mut rx = ctx.state.subscribe_events();

    // Start playback
    ctx.engine.enqueue_file(PathBuf::from("test.mp3")).await.unwrap();

    // Collect position events
    let mut positions = Vec::new();
    for _ in 0..3 {
        if let Ok(event) = tokio::time::timeout(
            Duration::from_millis(1100),
            rx.recv()
        ).await {
            if let WkmpEvent::PlaybackPosition { position_ms, .. } = event.unwrap() {
                positions.push(position_ms);
            }
        }
    }

    // Verify ~1 second intervals
    assert_eq!(positions.len(), 3);
    assert!((positions[1] - positions[0]) >= 900); // Allow 10% jitter
    assert!((positions[1] - positions[0]) <= 1100);
}
```

### 8.2 Integration Tests

**SSE Connection and InitialState:**
```rust
#[tokio::test]
async fn test_sse_sends_initial_state() {
    let server = TestServer::start().await;

    // Enqueue files
    server.enqueue_file("file1.mp3").await;
    server.enqueue_file("file2.mp3").await;

    // Connect SSE
    let mut events = server.connect_sse().await;

    // First event should be InitialState
    let event = events.next().await.unwrap();
    match event {
        WkmpEvent::InitialState { queue, position, volume } => {
            assert_eq!(queue.len(), 2);
            assert!(position.is_some());
            assert_eq!(volume, 0.5); // Default
        }
        _ => panic!("Expected InitialState as first event"),
    }
}
```

### 8.3 Manual Tests

**Checklist:**

- [ ] Open developer UI → See queue/position/volume immediately
- [ ] Enqueue file → Queue updates in <100ms
- [ ] Remove file → Queue updates immediately
- [ ] Start playback → Position updates every ~1 second
- [ ] Change volume → Volume displays new value instantly
- [ ] Open 2nd browser tab → Both tabs show same data
- [ ] Change volume in Tab A → Tab B updates
- [ ] Event log shows all events with timestamps
- [ ] Export event log → JSON copied to clipboard
- [ ] Disconnect WiFi → "Disconnected" status
- [ ] Reconnect WiFi → "Connected" status + data refreshed
- [ ] Restart server → UI reconnects and shows fresh state

---

## 9. Migration Plan

### 9.1 Backward Compatibility

**HTTP Endpoints:** Keep existing for external tools
- `GET /playback/queue` - Still works (for scripts/tests)
- `GET /playback/state` - Still works
- `GET /playback/position` - Still works

**SSE Endpoint:** Enhanced, not replaced
- Existing SSE subscribers continue to work
- New event types added (backward compatible)

### 9.2 Deployment Steps

1. **Deploy new backend** (with enhanced SSE)
2. **Test SSE endpoints** with curl/manual verification
3. **Deploy new frontend** (SSE-only developer UI)
4. **Monitor event logs** for any issues
5. **Remove old polling code** after 1 week of stable operation

### 9.3 Rollback Plan

If SSE approach has issues:
1. Revert to previous `developer_ui.html` (polling version)
2. New backend still works with old frontend
3. Fix issues in SSE implementation
4. Redeploy when ready

---

## 10. Performance Analysis

### 10.1 Network Traffic

**Current (Polling):**
```
Queue polling: 30 req/min × 800 bytes = 24 KB/min = 1.44 MB/hour
Position polling (if added): 60 req/min × 200 bytes = 12 KB/min = 0.72 MB/hour
Volume polling (if added): 30 req/min × 100 bytes = 3 KB/min = 0.18 MB/hour
Total: 2.34 MB/hour per client
```

**Proposed (SSE):**
```
InitialState: 1 event × 1 KB = 1 KB
Queue changes: ~5/hour × 800 bytes = 4 KB
Position updates: 3600/hour × 200 bytes = 720 KB
Volume changes: ~2/hour × 100 bytes = 0.2 KB
Keepalive: 240/hour × 10 bytes = 2.4 KB
Total: ~727 KB/hour per client
```

**Savings:** ~69% reduction (2.34 MB → 0.73 MB)

**Note:** Position updates dominate traffic (720 KB/hour). Acceptable for 1/second precision.

### 10.2 Server CPU

**Current (Polling - if all features added):**
```
Queue: 30/min × 5ms = 150ms/min = 2.5ms/sec
Position: 60/min × 2ms = 120ms/min = 2ms/sec
Volume: 30/min × 1ms = 30ms/min = 0.5ms/sec
Total: ~5ms/sec = 0.5% CPU
```

**Proposed (SSE):**
```
Queue: 5/hour × 2ms = 0.00003ms/sec = negligible
Position: 3600/hour × 0.5ms = 0.5ms/sec = 0.05% CPU
Volume: 2/hour × 0.5ms = negligible
Broadcast overhead: ~0.1ms/event
Total: ~0.1% CPU
```

**Savings:** ~80% reduction (0.5% → 0.1%)

### 10.3 Memory

**Current:** 0 bytes (stateless)

**Proposed:**
- SSE connection state: ~5 KB per client
- Event log buffer: ~10 KB per client (100 events)
- Total: ~15 KB per client

**Impact:** 10 clients = 150 KB. Negligible.

---

## 11. Future Enhancements

### 11.1 Event Filtering

**Feature:** Allow clients to subscribe to specific event types

**API:**
```
GET /events?filter=QueueStateUpdate,PlaybackPosition
```

**Use Case:** External monitoring tools that only need certain events

### 11.2 Event Replay

**Feature:** Request last N events on reconnection

**API:**
```
GET /events?replay_last=10
```

**Use Case:** Catch up on missed events after disconnect

### 11.3 Compressed Events

**Feature:** Optional gzip compression for large queue events

**Benefit:** Reduce 50-entry queue from 7.5 KB → ~1 KB

### 11.4 Event Rate Limiting

**Feature:** Throttle position updates if client falling behind

**Logic:** Skip position events if previous event not processed

---

## 12. Success Criteria

**Must Have:**
- ✅ No polling anywhere in developer UI
- ✅ Queue updates appear within 100ms of change
- ✅ Position updates every 1 second ±50ms
- ✅ Volume changes propagate to all tabs
- ✅ Event log shows all events with export capability
- ✅ SSE reconnection works reliably

**Nice to Have:**
- ✅ Event log color-coded by type
- ✅ Auto-scroll option for event log
- ✅ Connection status indicator
- ✅ Reconnection attempt counter

**Performance:**
- ✅ Network usage <1 MB/hour per client
- ✅ CPU usage <0.5% for SSE broadcasting
- ✅ Memory usage <50 KB per client

---

## 13. Conclusion

The SSE-based developer UI eliminates the observed "stuck queue" bug by removing all polling in favor of server-pushed updates. The unified event stream provides a single, debuggable communication channel for all real-time data (queue, position, volume). The raw event viewer gives developers unprecedented visibility into system behavior, making debugging and understanding much easier.

**Key Benefits:**
1. ✅ Fixes stuck queue bug (no more polling failures)
2. ✅ Real-time updates (<100ms latency)
3. ✅ Complete system visibility (event log)
4. ✅ Multi-tab synchronization (all tabs see same events)
5. ✅ Reduced network traffic (~69% savings)
6. ✅ Simpler architecture (one mechanism for everything)

**Implementation Effort:** ~10.5 hours

**Recommendation:** ✅ **IMPLEMENT** - This design directly addresses your observed issues and provides significant developer UX improvements.
