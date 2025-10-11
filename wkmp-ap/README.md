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
- `GET /status` - Service status and version info

## Configuration

Root folder resolution priority order:
1. Command-line argument `--root-folder`
2. Environment variable `WKMP_ROOT_FOLDER`
3. TOML config file `root_folder` key
4. OS-dependent default path
