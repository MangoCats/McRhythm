# Visible Developer Interface

**Part of:** [SPEC030 - Software Legibility Patterns](00_SUMMARY.md)

---

## 6.1 Overview

**Purpose:** HTTP-based runtime introspection exposing module structure and activity

**Access:** `http://localhost:<port>/dev/` (development builds only)

**Features:**
- Dashboard showing module status and recent activity
- Concept inspector exposing current state
- Sync monitor tracking rule activations
- Trace viewer visualizing provenance graphs
- Event stream providing real-time monitoring

**Implementation Strategy:**
- All dev interface routes gated by `#[cfg(debug_assertions)]`
- Same port as main API (no additional configuration)
- Read-only access (no state mutation via dev interface)

---

## 6.2 Dashboard

**Endpoint:** `GET /dev/`

**Purpose:** Entry point showing module overview and quick stats

### Response Format

```json
{
  "module": "wkmp-ap",
  "version": "0.1.0",
  "build_hash": "abc123",
  "uptime_seconds": 3600,
  "concepts": [
    {
      "name": "AudioPlayer",
      "uri": "wkmp-ap::AudioPlayer",
      "actions_count": 5,
      "queries_count": 8,
      "current_state_summary": {
        "playing": true,
        "current_passage": 42,
        "queue_depth": 3
      }
    },
    {
      "name": "CrossfadeEngine",
      "uri": "wkmp-ap::CrossfadeEngine",
      "actions_count": 2,
      "queries_count": 3,
      "current_state_summary": {
        "active_fade": true,
        "fade_curve": "equal_power"
      }
    }
  ],
  "synchronizations": [
    {
      "name": "AutoSelectWhenQueueLow",
      "activation_count": 42,
      "last_activated": 1699564800
    },
    {
      "name": "RecordPlaybackForCooldown",
      "activation_count": 156,
      "last_activated": 1699564900
    }
  ],
  "recent_activity": [
    {
      "timestamp": 1699564900,
      "type": "action",
      "concept": "AudioPlayer",
      "action": "play_passage",
      "flow_token": "abc123"
    },
    {
      "timestamp": 1699564850,
      "type": "sync",
      "sync_name": "AutoSelectWhenQueueLow",
      "triggered_by": "QueueDepthChanged"
    }
  ],
  "statistics": {
    "total_actions": 1234,
    "total_flows": 56,
    "avg_actions_per_flow": 22.0,
    "active_flows": 2
  }
}
```

### HTML View

```html
<!DOCTYPE html>
<html>
<head>
    <title>WKMP Developer Dashboard - wkmp-ap</title>
    <style>
        body { font-family: monospace; background: #1e1e1e; color: #d4d4d4; }
        .card { background: #252526; border: 1px solid #3e3e42; margin: 1em; padding: 1em; }
        .concept { background: #1e3a5f; }
        .sync { background: #3a1e5f; }
        .stat { display: inline-block; margin: 0.5em; padding: 0.5em; background: #2d2d30; }
    </style>
</head>
<body>
    <h1>WKMP Developer Dashboard - wkmp-ap</h1>

    <div class="card">
        <h2>Module Info</h2>
        <div class="stat">Version: 0.1.0 (abc123)</div>
        <div class="stat">Uptime: 1h 0m</div>
        <div class="stat">Total Actions: 1234</div>
        <div class="stat">Active Flows: 2</div>
    </div>

    <div class="card">
        <h2>Concepts (2)</h2>
        <div class="concept">
            <h3>AudioPlayer</h3>
            <p>URI: wkmp-ap::AudioPlayer</p>
            <p>Actions: 5 | Queries: 8</p>
            <p>State: Playing passage 42, queue depth 3</p>
            <a href="/dev/concepts/AudioPlayer">Inspect →</a>
        </div>
        <div class="concept">
            <h3>CrossfadeEngine</h3>
            <p>URI: wkmp-ap::CrossfadeEngine</p>
            <p>Actions: 2 | Queries: 3</p>
            <p>State: Active fade (equal_power curve)</p>
            <a href="/dev/concepts/CrossfadeEngine">Inspect →</a>
        </div>
    </div>

    <div class="card">
        <h2>Synchronizations (2)</h2>
        <div class="sync">
            <h3>AutoSelectWhenQueueLow</h3>
            <p>Activations: 42 | Last: 2 minutes ago</p>
            <a href="/dev/syncs/AutoSelectWhenQueueLow">Details →</a>
        </div>
        <div class="sync">
            <h3>RecordPlaybackForCooldown</h3>
            <p>Activations: 156 | Last: 1 minute ago</p>
            <a href="/dev/syncs/RecordPlaybackForCooldown">Details →</a>
        </div>
    </div>

    <div class="card">
        <h2>Quick Links</h2>
        <ul>
            <li><a href="/dev/traces">Action Traces →</a></li>
            <li><a href="/dev/events/stream">Live Event Stream →</a></li>
            <li><a href="/dev/api">API Documentation →</a></li>
        </ul>
    </div>
</body>
</html>
```

---

## 6.3 Concept Inspector

**Endpoint:** `GET /dev/concepts/<concept_name>`

**Purpose:** Expose current state and available operations for a concept

### Response Format

```json
{
  "concept": {
    "name": "AudioPlayer",
    "uri": "wkmp-ap::AudioPlayer",
    "description": "Core playback engine with queue management and crossfading"
  },
  "current_state": {
    "current_passage": {
      "passage_id": 42,
      "song_id": 5,
      "title": "Bohemian Rhapsody",
      "artist": "Queen",
      "position_seconds": 145.3,
      "duration_seconds": 354.0
    },
    "playback_state": "playing",
    "volume": 0.75,
    "queue": [
      {
        "passage_id": 43,
        "title": "Stairway to Heaven"
      },
      {
        "passage_id": 44,
        "title": "Hotel California"
      }
    ],
    "queue_depth": 2
  },
  "actions": [
    {
      "name": "play_passage",
      "parameters": [
        {"name": "passage_id", "type": "PassageId"}
      ],
      "returns": "Result<(), AudioPlayerError>",
      "description": "Play specified passage with crossfading"
    },
    {
      "name": "pause",
      "parameters": [],
      "returns": "Result<(), AudioPlayerError>",
      "description": "Pause current playback"
    },
    {
      "name": "enqueue",
      "parameters": [
        {"name": "passage_id", "type": "PassageId"}
      ],
      "returns": "Result<(), AudioPlayerError>",
      "description": "Add passage to playback queue"
    }
  ],
  "queries": [
    {
      "name": "get_current_passage",
      "parameters": [],
      "returns": "Option<PassageId>",
      "description": "Get currently playing passage ID"
    },
    {
      "name": "is_playing",
      "parameters": [],
      "returns": "bool",
      "description": "Check if playback active"
    },
    {
      "name": "get_queue_depth",
      "parameters": [],
      "returns": "usize",
      "description": "Get number of queued passages"
    }
  ],
  "recent_actions": [
    {
      "action": "play_passage",
      "timestamp": 1699564900,
      "inputs": {"passage_id": 42},
      "outputs": {"started_at": 1699564900},
      "flow_token": "abc123"
    },
    {
      "action": "enqueue",
      "timestamp": 1699564850,
      "inputs": {"passage_id": 43},
      "outputs": {"queued": true},
      "flow_token": "def456"
    }
  ]
}
```

### Try Action Form

**HTML:**
```html
<div class="card">
    <h3>Try Action: play_passage</h3>
    <form action="/dev/concepts/AudioPlayer/try_action" method="POST">
        <input type="hidden" name="action" value="play_passage">
        <label>passage_id (PassageId):</label>
        <input type="number" name="passage_id" value="42">
        <button type="submit">Execute</button>
    </form>
    <p class="note">NOTE: This creates a real action in the system. Use with caution.</p>
</div>
```

**Endpoint:** `POST /dev/concepts/<concept_name>/try_action`

**Purpose:** Execute action for testing (development only)

---

## 6.4 Sync Monitor

**Endpoint:** `GET /dev/syncs/<sync_name>`

**Purpose:** Inspect synchronization rule and activation history

### Response Format

```json
{
  "synchronization": {
    "name": "AutoSelectWhenQueueLow",
    "when": {
      "event_type": "QueueDepthChanged",
      "bindings": [
        {"var": "depth", "type": "free"}
      ]
    },
    "where": {
      "type": "and",
      "conditions": [
        {
          "concept": "Settings",
          "query": "is_auto_play_enabled",
          "expectation": "affirm"
        },
        {
          "concept": "AudioPlayer",
          "query": "get_queue_depth",
          "expectation": {"equals": 1}
        }
      ]
    },
    "then": [
      {
        "invoke": {
          "concept": "Timeslot",
          "action": "get_target_flavor"
        }
      },
      {
        "invoke": {
          "concept": "PassageSelection",
          "action": "select_next",
          "parameters": [{"name": "flavor", "type": "bound"}]
        }
      },
      {
        "invoke": {
          "concept": "AudioPlayer",
          "action": "enqueue",
          "parameters": [{"name": "passage_id", "type": "bound"}]
        }
      }
    ]
  },
  "statistics": {
    "total_activations": 42,
    "successful_activations": 40,
    "failed_activations": 2,
    "where_clause_rejections": 15,
    "avg_execution_time_ms": 12.5,
    "last_activated": 1699564800
  },
  "recent_activations": [
    {
      "timestamp": 1699564800,
      "triggered_by_event": {
        "type": "QueueDepthChanged",
        "data": {"depth": 1}
      },
      "where_result": true,
      "actions_executed": [
        {
          "concept": "Timeslot",
          "action": "get_target_flavor",
          "result": {"flavor": {"valence": 0.7, "energy": 0.6}}
        },
        {
          "concept": "PassageSelection",
          "action": "select_next",
          "result": {"passage_id": 43}
        },
        {
          "concept": "AudioPlayer",
          "action": "enqueue",
          "result": {"success": true}
        }
      ],
      "flow_token": "abc123",
      "execution_time_ms": 11
    }
  ]
}
```

### Visualization

**Sync Activation Timeline:**
```
Timeline: AutoSelectWhenQueueLow (last 1 hour)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  :00        :15        :30        :45        :00
   │          │          │          │          │
   ✓          ✓          ✗          ✓          ✓

Legend: ✓ = activated  ✗ = where clause rejected  · = no trigger
```

---

## 6.5 Trace Viewer

**Endpoint:** `GET /dev/traces?flow_token=<uuid>`

**Purpose:** Visualize action trace as directed acyclic graph

### Response Format (JSON)

```json
{
  "flow_token": "abc123",
  "root_action_id": "def456",
  "created_at": 1699564800,
  "actions": [
    {
      "id": "def456",
      "concept_uri": "wkmp-ui::Web",
      "action": "handle_request",
      "inputs": {"method": "POST", "path": "/api/play"},
      "outputs": {"status": 200},
      "timestamp": 1699564800
    },
    {
      "id": "ghi789",
      "concept_uri": "wkmp-ap::AudioPlayer",
      "action": "play_passage",
      "inputs": {"passage_id": 42},
      "outputs": {"started_at": 1699564801},
      "timestamp": 1699564801
    }
  ],
  "provenance_edges": [
    {
      "from_action": "def456",
      "to_action": "ghi789",
      "synchronization": "OnWebRequest"
    }
  ]
}
```

### HTML Visualization

**ASCII Graph:**
```
┌──────────────────────────────────────┐
│ Flow Token: abc123                   │
│ Started: 2024-11-09 14:23:00 UTC     │
│ Duration: 45ms                       │
└──────────────────────────────────────┘

[Web::handle_request]
    inputs: {"method": "POST", "path": "/api/play"}
    outputs: {"status": 200}
    timestamp: 14:23:00.000
    │
    ├──(sync: OnWebRequest)──> [Auth::validate_token]
    │                              inputs: {"token": "..."}
    │                              outputs: {"valid": true, "user_id": 42}
    │                              timestamp: 14:23:00.005
    │                              duration: 5ms
    │
    └──(sync: OnAuthSuccess)──> [AudioPlayer::play_passage]
                                   inputs: {"passage_id": 42}
                                   outputs: {"started_at": 1699564801}
                                   timestamp: 14:23:00.010
                                   duration: 35ms
```

**Interactive SVG Graph:**
```html
<svg width="800" height="600" xmlns="http://www.w3.org/2000/svg">
  <!-- Nodes -->
  <g id="node-def456" class="action-node">
    <rect x="100" y="50" width="200" height="80" fill="#1e3a5f" stroke="#4a90e2"/>
    <text x="200" y="80" text-anchor="middle" fill="white">Web::handle_request</text>
    <text x="200" y="100" text-anchor="middle" fill="#aaa" font-size="12">14:23:00.000</text>
  </g>

  <g id="node-ghi789" class="action-node">
    <rect x="100" y="200" width="200" height="80" fill="#1e3a5f" stroke="#4a90e2"/>
    <text x="200" y="230" text-anchor="middle" fill="white">AudioPlayer::play_passage</text>
    <text x="200" y="250" text-anchor="middle" fill="#aaa" font-size="12">14:23:00.010 (35ms)</text>
  </g>

  <!-- Edges -->
  <g id="edge-1" class="provenance-edge">
    <line x1="200" y1="130" x2="200" y2="200" stroke="#4a90e2" stroke-width="2" marker-end="url(#arrowhead)"/>
    <text x="210" y="165" fill="#4a90e2" font-size="12">OnWebRequest</text>
  </g>

  <!-- Arrowhead marker -->
  <defs>
    <marker id="arrowhead" markerWidth="10" markerHeight="10" refX="5" refY="5" orient="auto">
      <polygon points="0 0, 10 5, 0 10" fill="#4a90e2"/>
    </marker>
  </defs>
</svg>
```

---

## 6.6 Event Stream

**Endpoint:** `GET /dev/events/stream`

**Purpose:** Real-time server-sent events stream for monitoring

### SSE Format

```
event: action
data: {"concept": "AudioPlayer", "action": "play_passage", "inputs": {"passage_id": 42}, "flow_token": "abc123", "timestamp": 1699564800}

event: sync_activated
data: {"sync_name": "AutoSelectWhenQueueLow", "triggered_by": "QueueDepthChanged", "where_result": true, "timestamp": 1699564850}

event: sync_rejected
data: {"sync_name": "EnforceAuthenticationOnPlayback", "triggered_by": "handle_play_request", "where_result": false, "reason": "is_authenticated returned false", "timestamp": 1699564900}
```

### HTML Consumer

```html
<!DOCTYPE html>
<html>
<head>
    <title>Live Event Stream - wkmp-ap</title>
    <style>
        #events { font-family: monospace; background: #1e1e1e; color: #d4d4d4; padding: 1em; max-height: 600px; overflow-y: auto; }
        .event { margin: 0.5em 0; padding: 0.5em; background: #252526; border-left: 3px solid #4a90e2; }
        .event.action { border-left-color: #1e3a5f; }
        .event.sync { border-left-color: #3a1e5f; }
        .timestamp { color: #888; }
    </style>
</head>
<body>
    <h1>Live Event Stream - wkmp-ap</h1>
    <button onclick="clearEvents()">Clear</button>
    <button onclick="togglePause()">Pause/Resume</button>
    <div id="events"></div>

    <script>
        const eventsDiv = document.getElementById('events');
        let paused = false;

        const eventSource = new EventSource('/dev/events/stream');

        eventSource.addEventListener('action', (e) => {
            if (paused) return;
            const data = JSON.parse(e.data);
            const div = document.createElement('div');
            div.className = 'event action';
            div.innerHTML = `
                <span class="timestamp">${new Date(data.timestamp * 1000).toISOString()}</span>
                <strong>ACTION:</strong> ${data.concept}::${data.action}
                <pre>${JSON.stringify(data.inputs, null, 2)}</pre>
            `;
            eventsDiv.insertBefore(div, eventsDiv.firstChild);
        });

        eventSource.addEventListener('sync_activated', (e) => {
            if (paused) return;
            const data = JSON.parse(e.data);
            const div = document.createElement('div');
            div.className = 'event sync';
            div.innerHTML = `
                <span class="timestamp">${new Date(data.timestamp * 1000).toISOString()}</span>
                <strong>SYNC ACTIVATED:</strong> ${data.sync_name}
                <span>Triggered by: ${data.triggered_by}</span>
            `;
            eventsDiv.insertBefore(div, eventsDiv.firstChild);
        });

        function clearEvents() {
            eventsDiv.innerHTML = '';
        }

        function togglePause() {
            paused = !paused;
        }
    </script>
</body>
</html>
```

---

## 6.7 Implementation in Axum

**Router Setup:**
```rust
#[cfg(debug_assertions)]
fn dev_routes() -> Router {
    Router::new()
        .route("/dev/", get(dashboard))
        .route("/dev/concepts", get(list_concepts))
        .route("/dev/concepts/:concept_name", get(inspect_concept))
        .route("/dev/concepts/:concept_name/try_action", post(try_action))
        .route("/dev/syncs", get(list_syncs))
        .route("/dev/syncs/:sync_name", get(inspect_sync))
        .route("/dev/traces", get(list_traces))
        .route("/dev/traces/:flow_token", get(view_trace))
        .route("/dev/events/stream", get(event_stream))
}

#[cfg(not(debug_assertions))]
fn dev_routes() -> Router {
    // No-op in release builds
    Router::new()
}

pub fn create_app() -> Router {
    Router::new()
        .route("/api/play", post(handle_play))
        .route("/api/pause", post(handle_pause))
        // ... other API routes
        .merge(dev_routes())  // Conditionally includes dev interface
}
```

---

## Navigation

**Previous:** [05_action_traces.md](05_action_traces.md) - Action trace architecture
**Next:** [07_wkmp_examples.md](07_wkmp_examples.md) - WKMP application examples

**Back to Summary:** [00_SUMMARY.md](00_SUMMARY.md)

---

**END OF SECTION 06**
