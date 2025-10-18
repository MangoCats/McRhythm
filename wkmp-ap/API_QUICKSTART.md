# WKMP Audio Player API - Quick Start Guide

Get up and running with the WKMP Audio Player API in 5 minutes.

---

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [First Request](#first-request)
3. [Basic Playback](#basic-playback)
4. [Monitoring Events](#monitoring-events)
5. [Common Tasks](#common-tasks)
6. [Next Steps](#next-steps)

---

## Prerequisites

**Required:**
- WKMP Audio Player (wkmp-ap) running on port 5721
- Audio files in configured root folder
- Command-line tools: `curl` and `jq`

**Optional:**
- JavaScript runtime for web examples
- Rust toolchain for Rust examples
- Python with `requests` and `sseclient-py` for Python examples

**Start the Audio Player:**
```bash
# Default configuration
./wkmp-ap

# Custom port
WKMP_AP_PORT=5721 ./wkmp-ap

# Check if running
curl http://localhost:5721/health
```

---

## First Request

### Health Check

Verify the Audio Player is running:

```bash
curl http://localhost:5721/health | jq
```

**Expected Output:**
```json
{
  "status": "healthy",
  "module": "audio_player",
  "version": "0.1.0"
}
```

✅ **Success!** You can communicate with the Audio Player.

---

## Basic Playback

### Step 1: Enqueue an Audio File

```bash
curl -X POST http://localhost:5721/playback/enqueue \
  -H "Content-Type: application/json" \
  -d '{"file_path":"test-10s.mp3"}' \
  | jq
```

**Response:**
```json
{
  "status": "ok",
  "queue_entry_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

**Notes:**
- File path is relative to configured root folder
- Use forward slashes (/) even on Windows
- Audio Player creates ephemeral passage if file not in database

### Step 2: Start Playback

```bash
curl -X POST http://localhost:5721/playback/play | jq
```

**Response:**
```json
{
  "status": "ok"
}
```

🎵 **Audio should now be playing!**

### Step 3: Check Playback Position

```bash
curl http://localhost:5721/playback/position | jq
```

**Response:**
```json
{
  "passage_id": "7c9e6679-7425-40de-944b-e07fc1f90ae7",
  "position_ms": 2345,
  "duration_ms": 10000,
  "state": "playing"
}
```

### Step 4: Pause Playback

```bash
curl -X POST http://localhost:5721/playback/pause | jq
```

🔇 **Audio should now be paused.**

---

## Monitoring Events

### Using cURL (Command Line)

Open a terminal and run:

```bash
curl -N http://localhost:5721/events
```

**Expected Output (streaming):**
```
event: PlaybackStateChanged
data: {"type":"PlaybackStateChanged","state":"playing","timestamp":"2025-10-18T14:30:00Z"}

event: PlaybackProgress
data: {"type":"PlaybackProgress","passage_id":"...","position_ms":5000,"duration_ms":10000,"timestamp":"..."}

event: PlaybackProgress
data: {"type":"PlaybackProgress","passage_id":"...","position_ms":10000,"duration_ms":10000,"timestamp":"..."}

event: PassageCompleted
data: {"type":"PassageCompleted","passage_id":"...","completed":true,"timestamp":"..."}
```

**Tip:** Keep this terminal open while testing other commands to see events in real-time.

### Using JavaScript (Browser)

```html
<!DOCTYPE html>
<html>
<head>
    <title>WKMP Event Monitor</title>
</head>
<body>
    <h1>WKMP Audio Player Events</h1>
    <div id="events"></div>

    <script>
        const eventSource = new EventSource('http://localhost:5721/events');
        const eventsDiv = document.getElementById('events');

        // Listen to all event types
        ['PlaybackStateChanged', 'PlaybackProgress', 'PassageStarted',
         'PassageCompleted', 'CurrentSongChanged', 'QueueChanged',
         'VolumeChanged'].forEach(eventType => {
            eventSource.addEventListener(eventType, (event) => {
                const data = JSON.parse(event.data);
                const div = document.createElement('div');
                div.style.padding = '10px';
                div.style.margin = '5px';
                div.style.border = '1px solid #ccc';
                div.innerHTML = `
                    <strong>${data.type}</strong><br>
                    <pre>${JSON.stringify(data, null, 2)}</pre>
                `;
                eventsDiv.prepend(div);
            });
        });

        eventSource.onerror = (error) => {
            console.error('SSE Error:', error);
        };
    </script>
</body>
</html>
```

**Save as `event-monitor.html` and open in browser.**

### Using Python

```python
import requests
import sseclient
import json

# Connect to event stream
response = requests.get('http://localhost:5721/events', stream=True)
client = sseclient.SSEClient(response)

# Process events
print("Monitoring WKMP events (Ctrl+C to stop)...")
for event in client.events():
    data = json.loads(event.data)
    print(f"\n[{data['type']}]")
    print(json.dumps(data, indent=2))
```

**Install dependencies:**
```bash
pip install requests sseclient-py
python event-monitor.py
```

---

## Common Tasks

### Set Volume to 75%

```bash
curl -X POST http://localhost:5721/audio/volume \
  -H "Content-Type: application/json" \
  -d '{"volume":75}' \
  | jq
```

### Get Current Volume

```bash
curl http://localhost:5721/audio/volume | jq
```

### Enqueue Multiple Files

```bash
# Song 1
curl -X POST http://localhost:5721/playback/enqueue \
  -H "Content-Type: application/json" \
  -d '{"file_path":"albums/artist/song1.mp3"}'

# Song 2
curl -X POST http://localhost:5721/playback/enqueue \
  -H "Content-Type: application/json" \
  -d '{"file_path":"albums/artist/song2.mp3"}'

# Song 3
curl -X POST http://localhost:5721/playback/enqueue \
  -H "Content-Type: application/json" \
  -d '{"file_path":"albums/artist/song3.mp3"}'
```

### View Queue

```bash
curl http://localhost:5721/playback/queue | jq
```

**Example Output:**
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

### Skip to Next Song

```bash
curl -X POST http://localhost:5721/playback/next
```

### Clear Entire Queue

```bash
curl -X POST http://localhost:5721/playback/queue/clear
```

### Remove Specific Queue Entry

```bash
# Get queue first to find queue_entry_id
curl http://localhost:5721/playback/queue | jq

# Remove by ID
curl -X DELETE http://localhost:5721/playback/queue/550e8400-e29b-41d4-a716-446655440000
```

### Seek to 30 Seconds

```bash
curl -X POST http://localhost:5721/playback/seek \
  -H "Content-Type: application/json" \
  -d '{"position_ms":30000}'
```

### List Available Audio Devices

```bash
curl http://localhost:5721/audio/devices | jq
```

**Example Output:**
```json
{
  "devices": [
    "default",
    "PulseAudio",
    "HDA Intel PCH, ALC887-VD Analog"
  ]
}
```

### Set Audio Device

```bash
curl -X POST http://localhost:5721/audio/device \
  -H "Content-Type: application/json" \
  -d '{"device_name":"PulseAudio"}'
```

**Note:** Device change requires restart (not yet implemented).

---

## Shell Script Examples

### Complete Playback Script

```bash
#!/bin/bash
# play-album.sh - Play all songs in an album

BASE_URL="http://localhost:5721"
ALBUM_PATH="albums/4_Non_Blondes/Bigger,_Better,_Faster,_More"

# Clear existing queue
echo "Clearing queue..."
curl -X POST $BASE_URL/playback/queue/clear

# Enqueue all songs
echo "Enqueueing songs..."
for song in "$ALBUM_PATH"/*.mp3; do
    echo "  Adding: $song"
    curl -X POST $BASE_URL/playback/enqueue \
        -H "Content-Type: application/json" \
        -d "{\"file_path\":\"$song\"}" \
        -s > /dev/null
done

# Start playback
echo "Starting playback..."
curl -X POST $BASE_URL/playback/play -s > /dev/null

# Show queue
echo ""
echo "Queue:"
curl $BASE_URL/playback/queue | jq '.queue[] | .file_path'
```

### Position Monitor Script

```bash
#!/bin/bash
# monitor-position.sh - Display current position every second

BASE_URL="http://localhost:5721"

while true; do
    RESPONSE=$(curl -s $BASE_URL/playback/position)

    POSITION=$(echo $RESPONSE | jq -r '.position_ms')
    DURATION=$(echo $RESPONSE | jq -r '.duration_ms')
    STATE=$(echo $RESPONSE | jq -r '.state')

    # Convert milliseconds to MM:SS
    POS_SEC=$((POSITION / 1000))
    DUR_SEC=$((DURATION / 1000))

    POS_MIN=$((POS_SEC / 60))
    POS_REM=$((POS_SEC % 60))

    DUR_MIN=$((DUR_SEC / 60))
    DUR_REM=$((DUR_SEC % 60))

    printf "\r[$STATE] %02d:%02d / %02d:%02d" \
        $POS_MIN $POS_REM $DUR_MIN $DUR_REM

    sleep 1
done
```

**Usage:**
```bash
chmod +x monitor-position.sh
./monitor-position.sh
```

---

## JavaScript Web Player Example

Create a simple web-based player:

```html
<!DOCTYPE html>
<html>
<head>
    <title>WKMP Web Player</title>
    <style>
        body { font-family: Arial, sans-serif; padding: 20px; }
        .controls button { padding: 10px 20px; margin: 5px; }
        #position { margin: 20px 0; }
        #queue { margin-top: 20px; }
        .queue-item { padding: 5px; border-bottom: 1px solid #ccc; }
    </style>
</head>
<body>
    <h1>WKMP Web Player</h1>

    <div class="controls">
        <button onclick="play()">▶ Play</button>
        <button onclick="pause()">⏸ Pause</button>
        <button onclick="skip()">⏭ Skip</button>
        <button onclick="setVolume(50)">🔉 50%</button>
        <button onclick="setVolume(75)">🔊 75%</button>
        <button onclick="setVolume(100)">🔊 100%</button>
    </div>

    <div id="position">
        <strong>Position:</strong> <span id="pos">00:00</span> / <span id="dur">00:00</span>
        <br><strong>State:</strong> <span id="state">paused</span>
    </div>

    <div id="queue">
        <h2>Queue</h2>
        <div id="queue-items"></div>
    </div>

    <script>
        const BASE_URL = 'http://localhost:5721';

        // SSE connection
        const eventSource = new EventSource(`${BASE_URL}/events`);

        eventSource.addEventListener('PlaybackProgress', (event) => {
            const data = JSON.parse(event.data);
            updatePosition(data.position_ms, data.duration_ms);
        });

        eventSource.addEventListener('PlaybackStateChanged', (event) => {
            const data = JSON.parse(event.data);
            document.getElementById('state').textContent = data.state;
        });

        eventSource.addEventListener('QueueChanged', () => {
            loadQueue();
        });

        // Control functions
        async function play() {
            await fetch(`${BASE_URL}/playback/play`, { method: 'POST' });
        }

        async function pause() {
            await fetch(`${BASE_URL}/playback/pause`, { method: 'POST' });
        }

        async function skip() {
            await fetch(`${BASE_URL}/playback/next`, { method: 'POST' });
        }

        async function setVolume(percent) {
            await fetch(`${BASE_URL}/audio/volume`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ volume: percent })
            });
        }

        async function loadQueue() {
            const response = await fetch(`${BASE_URL}/playback/queue`);
            const data = await response.json();

            const items = data.queue.map(item => `
                <div class="queue-item">
                    ${item.file_path}
                    <button onclick="removeFromQueue('${item.queue_entry_id}')">✕</button>
                </div>
            `).join('');

            document.getElementById('queue-items').innerHTML = items || '<p>Queue is empty</p>';
        }

        async function removeFromQueue(id) {
            await fetch(`${BASE_URL}/playback/queue/${id}`, { method: 'DELETE' });
        }

        function updatePosition(posMs, durMs) {
            document.getElementById('pos').textContent = formatTime(posMs);
            document.getElementById('dur').textContent = formatTime(durMs);
        }

        function formatTime(ms) {
            const sec = Math.floor(ms / 1000);
            const min = Math.floor(sec / 60);
            const rem = sec % 60;
            return `${min.toString().padStart(2, '0')}:${rem.toString().padStart(2, '0')}`;
        }

        // Initial load
        loadQueue();
    </script>
</body>
</html>
```

**Save as `web-player.html` and open in browser.**

---

## Testing the Crossfade

The Audio Player features sample-accurate crossfading (~0.02ms precision). Test it:

```bash
# Enqueue 3 songs with crossfade (requires database passages)
# If you have passages with configured fade points, they will crossfade automatically

# For testing, enqueue 3 short files
curl -X POST http://localhost:5721/playback/enqueue \
  -H "Content-Type: application/json" \
  -d '{"file_path":"test-10s.mp3"}'

curl -X POST http://localhost:5721/playback/enqueue \
  -H "Content-Type: application/json" \
  -d '{"file_path":"test-10s.mp3"}'

curl -X POST http://localhost:5721/playback/enqueue \
  -H "Content-Type: application/json" \
  -d '{"file_path":"test-10s.mp3"}'

# Start playback
curl -X POST http://localhost:5721/playback/play

# Monitor events to see PassageStarted/PassageCompleted
curl -N http://localhost:5721/events
```

**Crossfade Features:**
- 5 fade curves: Linear, Logarithmic, Exponential, S-Curve, Equal-Power
- Configurable fade-in/fade-out points per passage
- Sample-accurate timing (~0.02ms precision)
- Automatic fade application during playback

---

## Troubleshooting

### Connection Refused

**Problem:** `curl: (7) Failed to connect to localhost port 5721`

**Solution:**
- Check if wkmp-ap is running: `ps aux | grep wkmp-ap`
- Verify port: `netstat -tlnp | grep 5721`
- Check logs for startup errors

### File Not Found

**Problem:** `{"status":"error: File not found"}`

**Solution:**
- Verify file path is relative to root folder
- Use forward slashes (/) even on Windows
- Check file exists: `ls -la /path/to/root/folder/albums/artist/song.mp3`

### No Audio Output

**Problem:** Commands succeed but no audio

**Solution:**
- List devices: `curl http://localhost:5721/audio/devices`
- Check volume: `curl http://localhost:5721/audio/volume`
- Verify system audio: `speaker-test -t wav -c 2`
- Check Audio Player logs for cpal errors

### SSE Connection Drops

**Problem:** Event stream disconnects frequently

**Solution:**
- SSE connections auto-reconnect by design
- Check network stability
- Verify no firewall blocking long-lived connections
- Monitor Audio Player logs for errors

---

## Next Steps

### Read Full Documentation

- **API Reference:** `API_REFERENCE.md` - Complete endpoint documentation
- **OpenAPI Spec:** `openapi.yaml` - Machine-readable API specification
- **Architecture Review:** `OVERALL_IMPLEMENTATION_REVIEW.md` - System design

### Explore Advanced Features

- **Song Boundary Detection:** Multi-song passages emit `CurrentSongChanged` events
- **Buffer Status:** Monitor decode progress with `/playback/buffer_status`
- **Queue Reordering:** Rearrange playback order with `/playback/queue/reorder`
- **Configurable Intervals:** Database settings for event frequencies

### Build Applications

- **Web Player:** Full-featured browser-based player
- **Desktop App:** Native application using Rust/Tauri
- **Mobile App:** React Native or Flutter with SSE support
- **CLI Tool:** Command-line music player using cURL/jq

### Integrate with Other Modules

- **User Interface (wkmp-ui):** Polished web UI for end users
- **Program Director (wkmp-pd):** Automatic passage selection
- **Audio Ingest (wkmp-ai):** File scanning and metadata import

---

## Need Help?

- **Documentation:** Check `API_REFERENCE.md` for detailed endpoint docs
- **Examples:** See JavaScript/Rust/Python examples in this guide
- **Issues:** Report bugs in project issue tracker
- **Community:** Join WKMP project discussions

---

**Happy Hacking! 🎵**

**Last Updated:** 2025-10-18
**API Version:** 1.0.0
**Module:** wkmp-ap (Audio Player)
