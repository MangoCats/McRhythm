# WKMP Audio Player (wkmp-ap)

GStreamer-based audio playback microservice for WKMP.

## Features

- ✅ Command-line argument parsing with clap
- ✅ Root folder resolution (CLI → env → config → OS default)
- ✅ SQLite database initialization
- ✅ Module configuration management
- ✅ HTTP server with health check endpoints
- ✅ GStreamer integration (ready for pipeline implementation)
- 🚧 Dual GStreamer pipeline for crossfading (pending)
- 🚧 Playback control (play/pause/skip) (pending)
- 🚧 Queue management (pending)

## Building

Requires GStreamer development libraries:

```bash
# Ubuntu/Debian
sudo apt-get install libgstreamer1.0-dev libgstreamer-plugins-base1.0-dev libglib2.0-dev

# Build
cargo build --package wkmp-ap
```

## Running

```bash
# With default root folder
./target/debug/wkmp-ap

# With custom root folder
./target/debug/wkmp-ap --root-folder /path/to/data

# With custom port
./target/debug/wkmp-ap --port 5999

# Show help
./target/debug/wkmp-ap --help
```

## API Endpoints

- `GET /health` - Health check (returns 200 OK)
- `GET /status` - Service status, version, playback state, queue size
- `GET /playback/state` - Current playback state (position, duration, passage ID)
- `POST /playback/play` - Start or resume playback
- `POST /playback/pause` - Pause playback
- `POST /playback/skip` - Skip to next track in queue
- `POST /playback/enqueue` - Add file to playback queue
- `GET /queue` - Get all queue entries

## Usage Examples

### Enqueue a file

**IMPORTANT:** File paths must be:
- Relative to the root folder (no leading `/`)
- Use forward slashes on all platforms

```bash
# Correct: relative path
curl -X POST http://localhost:5721/playback/enqueue \
  -H "Content-Type: application/json" \
  -d '{"file_path": "music/song.mp3"}'

# Wrong: absolute path (will fail)
curl -X POST http://localhost:5721/playback/enqueue \
  -H "Content-Type: application/json" \
  -d '{"file_path": "/music/song.mp3"}'
```

### Start playback

```bash
curl -X POST http://localhost:5721/playback/play
```

### Check current state

```bash
curl http://localhost:5721/playback/state | jq .
```

### Complete workflow

```bash
# 1. Start server with custom root folder
./target/debug/wkmp-ap --root-folder /path/to/music --port 5999

# 2. Enqueue files
curl -X POST http://localhost:5999/playback/enqueue \
  -H "Content-Type: application/json" \
  -d '{"file_path": "albums/artist/track01.mp3"}'

curl -X POST http://localhost:5999/playback/enqueue \
  -H "Content-Type: application/json" \
  -d '{"file_path": "albums/artist/track02.mp3"}'

# 3. Start playback
curl -X POST http://localhost:5999/playback/play

# 4. Check queue
curl http://localhost:5999/queue | jq .

# 5. Control playback
curl -X POST http://localhost:5999/playback/pause
curl -X POST http://localhost:5999/playback/play
curl -X POST http://localhost:5999/playback/skip
```

## Configuration

Root folder resolution priority order:
1. Command-line argument `--root-folder`
2. Environment variable `WKMP_ROOT_FOLDER`
3. TOML config file `root_folder` key
4. OS-dependent default path
