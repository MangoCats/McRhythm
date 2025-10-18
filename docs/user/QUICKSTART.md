# WKMP Quick Start Guide

Get WKMP running in under 5 minutes with **zero configuration required**.

---

## What You Need

- **Linux, macOS, or Windows**
- **Pre-built WKMP binary** or Rust toolchain for building from source
- **No configuration files needed!** (WKMP uses sensible defaults)

---

## Installation

### Option 1: Pre-built Binaries (Recommended)

```bash
# Download release for your platform from GitHub releases
# Extract to standard location

# Linux
sudo tar -xzf wkmp-linux-x64.tar.gz -C /usr/local/bin/

# macOS
tar -xzf wkmp-macos-x64.tar.gz
sudo cp wkmp-* /usr/local/bin/

# Windows PowerShell (as Administrator)
Expand-Archive wkmp-windows-x64.zip -DestinationPath "C:\Program Files\WKMP\bin"
```

### Option 2: Build from Source

```bash
# Clone repository
git clone https://github.com/yourusername/wkmp.git
cd wkmp

# Build all modules (takes 5-10 minutes)
cargo build --release

# Binaries will be in target/release/
# - wkmp-ap (Audio Player)
# - wkmp-ui (User Interface)
# - wkmp-pd (Program Director)
# - wkmp-ai (Audio Ingest - Full version)
# - wkmp-le (Lyric Editor - Full version)
```

---

## First Run: Zero Configuration

WKMP is designed to "just work" with no configuration. Simply run the audio player:

```bash
wkmp-ap
```

**That's it!** You'll see output like this:

```
INFO: Starting WKMP Audio Player (wkmp-ap)
WARN: Config file not found at ~/.config/wkmp/audio-player.toml, using default configuration
INFO: Root folder: /home/user/Music (compiled default)
INFO: Creating root folder directory: /home/user/Music
INFO: Initialized new database: /home/user/Music/wkmp.db
INFO: Initialized setting 'volume_level' with default value: 0.5
INFO: Initialized setting 'global_crossfade_time' with default value: 2.0
... (25+ more settings initialized) ...
INFO: Audio Player listening on 127.0.0.1:5721
INFO: Playback engine started
INFO: Starting HTTP server on 0.0.0.0:5721
```

### What Just Happened?

1. **No config file found** → WKMP logged a warning (not an error!)
2. **Used default location** → `~/Music` on Linux/macOS, `%USERPROFILE%\Music\wkmp` on Windows
3. **Created directory** → Automatically created `~/Music` if it didn't exist
4. **Initialized database** → Created `wkmp.db` with 27+ default settings
5. **Started server** → Listening on `http://localhost:5721`

**No errors. No manual setup. Just works.**

---

## Verify Installation

### Check the Server is Running

```bash
# Test the health endpoint
curl http://localhost:5721/health

# Expected response:
{
  "status": "healthy",
  "module": "audio_player",
  "version": "1.0.0",
  "uptime_seconds": 42
}
```

### Check the Database Was Created

```bash
# Linux/macOS
ls -lh ~/Music/wkmp.db

# Expected output:
-rw-r--r-- 1 user user 68K Jan 18 12:34 /home/user/Music/wkmp.db

# Windows PowerShell
Get-Item "$env:USERPROFILE\Music\wkmp\wkmp.db"
```

---

## Basic Usage

### Start All Modules (Full Version)

```bash
# Terminal 1: Audio Player
wkmp-ap

# Terminal 2: User Interface
wkmp-ui

# Terminal 3: Program Director (automatic music selection)
wkmp-pd
```

### Access the Web UI

Open your browser to:
```
http://localhost:5720
```

The web interface provides:
- Playback controls (play, pause, skip)
- Queue management
- Volume and crossfade settings
- Library browsing

---

## Customizing Configuration (Optional)

WKMP works out of the box, but you can customize it if needed.

### Use a Different Root Folder

**Option 1: Environment Variable** (temporary)
```bash
export WKMP_ROOT=/mnt/music
wkmp-ap
```

**Option 2: Command-Line Argument** (one-time)
```bash
wkmp-ap --root-folder /mnt/music
```

**Option 3: Configuration File** (permanent)

Create config file at:
- Linux: `~/.config/wkmp/audio-player.toml`
- macOS: `~/Library/Application Support/WKMP/audio-player.toml`
- Windows: `%APPDATA%\WKMP\audio-player.toml`

```toml
# audio-player.toml
root_folder = "/mnt/music"

[logging]
level = "info"
log_file = ""  # Empty = stdout only
```

**Priority Order:**
1. Command-line argument (highest)
2. Environment variable
3. Config file
4. Compiled default (lowest)

### Change Logging Level

```bash
# More verbose logging
WKMP_LOG_LEVEL=debug wkmp-ap

# Only errors
WKMP_LOG_LEVEL=error wkmp-ap
```

---

## Adding Music

### Manual File Placement

1. Copy your music files to the root folder:
   ```bash
   cp -r /path/to/music/* ~/Music/
   ```

2. Enqueue files via API:
   ```bash
   curl -X POST http://localhost:5721/api/v1/playback/enqueue \
     -H "Content-Type: application/json" \
     -d '{
       "file_path": "artist/album/song.mp3",
       "start_time_ms": 0,
       "end_time_ms": 240000,
       "fade_in_point_ms": 8000,
       "lead_in_point_ms": 8000,
       "fade_out_point_ms": 232000,
       "lead_out_point_ms": 232000
     }'
   ```

### Using Audio Ingest (Full Version)

```bash
# Start the ingest module
wkmp-ai

# Scan a directory
curl -X POST http://localhost:5723/api/v1/ingest/scan \
  -H "Content-Type: application/json" \
  -d '{"directory": "/path/to/music"}'
```

The ingest module will:
- Scan all audio files
- Extract metadata (artist, album, title)
- Compute musical flavor vectors (via AcousticBrainz)
- Create passages with optimal crossfade points

---

## Common Operations

### Start Playback

```bash
curl -X POST http://localhost:5721/api/v1/playback/play
```

### Pause Playback

```bash
curl -X POST http://localhost:5721/api/v1/playback/pause
```

### Check Queue

```bash
curl http://localhost:5721/api/v1/queue
```

### Adjust Volume

```bash
curl -X POST http://localhost:5721/api/v1/playback/volume \
  -H "Content-Type: application/json" \
  -d '{"volume": 0.75}'
```

### Change Crossfade Time

```bash
curl -X POST http://localhost:5721/api/v1/playback/crossfade \
  -H "Content-Type: application/json" \
  -d '{"crossfade_seconds": 5.0}'
```

---

## Stopping WKMP

### Graceful Shutdown

Press `Ctrl+C` in each terminal running a module. WKMP will:
- Finish current requests
- Save playback position
- Save queue state
- Close database connections
- Exit cleanly

### Force Stop

```bash
# Linux/macOS
pkill wkmp-ap
pkill wkmp-ui
pkill wkmp-pd

# Windows PowerShell
Stop-Process -Name "wkmp-ap"
Stop-Process -Name "wkmp-ui"
Stop-Process -Name "wkmp-pd"
```

---

## Version Differences

WKMP comes in three versions:

### Full Version (All Features)
**Binaries:** wkmp-ap, wkmp-ui, wkmp-pd, wkmp-ai, wkmp-le

**Use case:** Personal music server with local file collection
- Audio playback with crossfading
- Web user interface
- Automatic music selection (Program Director)
- File scanning and metadata extraction (Audio Ingest)
- On-demand lyric editing (Lyric Editor)

### Lite Version (No File Ingest)
**Binaries:** wkmp-ap, wkmp-ui, wkmp-pd

**Use case:** Playback from pre-populated database
- Audio playback with crossfading
- Web user interface
- Automatic music selection
- Database must be populated manually or by Full version

### Minimal Version (Manual Control Only)
**Binaries:** wkmp-ap, wkmp-ui

**Use case:** Simple playback with manual queue management
- Audio playback with crossfading
- Web user interface
- No automatic music selection
- User manually enqueues all passages

---

## Next Steps

**Now that WKMP is running:**

1. **Add music files** to your root folder (default: `~/Music`)
2. **Browse the web interface** at http://localhost:5720
3. **Read the API documentation** in `docs/SPEC007-api_design.md`
4. **Customize settings** via the web UI or database
5. **Set up Program Director** for automatic music selection (see `docs/REQ001-requirements.md`)

---

## Troubleshooting

### Audio Player Won't Start

**Problem:** `wkmp-ap` exits immediately

**Solution 1:** Check if port is already in use
```bash
# Linux/macOS
lsof -i :5721

# Windows PowerShell
Get-NetTCPConnection -LocalPort 5721

# Use different port if needed
wkmp-ap --port 15721
```

**Solution 2:** Check database permissions
```bash
# Linux/macOS
ls -l ~/Music/wkmp.db

# Should be readable/writable by your user
chmod 600 ~/Music/wkmp.db
```

**Solution 3:** Check audio device access
```bash
# Linux - ensure you're in audio group
groups | grep audio

# macOS - check System Preferences > Security > Microphone
# Windows - check Sound settings
```

### Config File Errors

**Problem:** `Failed to parse TOML`

**Solution:** Delete the config file and let WKMP use defaults
```bash
# Linux
rm ~/.config/wkmp/audio-player.toml

# macOS
rm ~/Library/Application\ Support/WKMP/audio-player.toml

# Windows PowerShell
Remove-Item "$env:APPDATA\WKMP\audio-player.toml"

# Now restart - WKMP will use compiled defaults
wkmp-ap
```

### Database Corruption

**Problem:** `PRAGMA integrity_check failed`

**Solution:** WKMP automatically creates backups. Restore from backup:
```bash
# Find backups (created every 14+ days)
ls -l ~/Music/wkmp-backup-*.db

# Restore latest backup
mv ~/Music/wkmp.db ~/Music/wkmp-corrupted.db
cp ~/Music/wkmp-backup-20250118.db ~/Music/wkmp.db

# Restart modules
wkmp-ap
```

### Can't Access Web UI

**Problem:** `Connection refused` when accessing http://localhost:5720

**Solution 1:** Check wkmp-ui is running
```bash
curl http://localhost:5720/health

# If no response, start wkmp-ui
wkmp-ui
```

**Solution 2:** Check firewall
```bash
# Linux
sudo ufw allow 5720/tcp

# macOS
# System Preferences > Security > Firewall > Allow wkmp-ui

# Windows PowerShell (as Administrator)
New-NetFirewallRule -DisplayName "WKMP UI" -Direction Inbound -LocalPort 5720 -Protocol TCP -Action Allow
```

### Permission Denied Creating Database

**Problem:** `Failed to create directory: Permission denied`

**Solution:** Use a writable location
```bash
# Use your home directory
export WKMP_ROOT=~/wkmp-data
wkmp-ap

# Or use /tmp for testing
export WKMP_ROOT=/tmp/wkmp-test
wkmp-ap
```

---

## Getting Help

**Documentation:**
- **Architecture:** `docs/SPEC001-architecture.md`
- **API Reference:** `docs/SPEC007-api_design.md`
- **Requirements:** `docs/REQ001-requirements.md`
- **Configuration Examples:** `docs/examples/README.md`

**Community:**
- GitHub Issues: https://github.com/yourusername/wkmp/issues
- Discussions: https://github.com/yourusername/wkmp/discussions

**Logs:**
```bash
# Increase log verbosity for debugging
WKMP_LOG_LEVEL=debug wkmp-ap 2>&1 | tee wkmp-debug.log

# Attach log file when reporting issues
```

---

## Success!

You now have WKMP running with zero manual configuration. The system:
- ✅ Created necessary directories automatically
- ✅ Initialized database with sensible defaults
- ✅ Started HTTP server and is ready for requests
- ✅ Has graceful degradation for all error conditions

**Welcome to WKMP - the Auto DJ Music Player with sample-accurate crossfading!**

For advanced configuration and features, see the complete documentation in the `docs/` directory.
