# Deployment and Process Management

**🚀 TIER 3 - IMPLEMENTATION SPECIFICATION**

Defines deployment, process management, and operational configuration for McRhythm's microservices architecture. See [Document Hierarchy](document_hierarchy.md).

> **Related Documentation:** [Architecture](architecture.md) | [API Design](api_design.md) | [Requirements](requirements.md)

---

## Overview

**[DEP-OVR-010]** McRhythm consists of 4 independent processes that communicate via HTTP/REST APIs. This document specifies how to deploy, start, stop, configure, and monitor these processes across different operating systems and deployment scenarios.

**[DEP-OVR-020]** The version (Full, Lite, Minimal) determines which modules run:
- **Full Version**: Modules 1, 2, 3, 4
- **Lite Version**: Modules 1, 2, 3
- **Minimal Version**: Modules 1, 2

## 1. Module Binaries

**[DEP-BIN-010]** Each module is compiled as a separate executable binary:
- `mcrhythm-audio-player` - Module 1: Audio Player
- `mcrhythm-ui` - Module 2: User Interface
- `mcrhythm-program-director` - Module 3: Program Director
- `mcrhythm-file-ingest` - Module 4: File Ingest Interface

**[DEP-BIN-020]** Binaries shall be installed in a standard location:
- **Linux**: `/usr/local/bin/` or `/opt/mcrhythm/bin/`
- **macOS**: `/usr/local/bin/` or `/Applications/McRhythm.app/Contents/MacOS/`
- **Windows**: `C:\Program Files\McRhythm\bin\`

## 2. Configuration Files

### 2.1. Configuration File Location

**[DEP-CFG-010]** Each module reads its configuration from a TOML file located at:
- **Linux**: `~/.config/mcrhythm/<module-name>.toml`
- **macOS**: `~/Library/Application Support/McRhythm/<module-name>.toml`
- **Windows**: `%APPDATA%\McRhythm\<module-name>.toml`

**[DEP-CFG-020]** System-wide default configurations may be placed at:
- **Linux**: `/etc/mcrhythm/<module-name>.toml`
- **macOS**: `/Library/Application Support/McRhythm/<module-name>.toml`
- **Windows**: `C:\ProgramData\McRhythm\<module-name>.toml`

**[DEP-CFG-030]** User-specific configuration files override system-wide defaults.

### 2.2. Module 1: Audio Player Configuration

**[DEP-CFG-100]** Configuration file: `audio-player.toml`

```toml
[server]
# HTTP server listening port
port = 8081

# Bind address (use "0.0.0.0" for all interfaces, "127.0.0.1" for localhost only)
bind_address = "127.0.0.1"

[database]
# Path to shared SQLite database
path = "~/.local/share/mcrhythm/mcrhythm.db"

[audio]
# Default audio output device (empty = system default)
default_device = ""

# Default volume level (0-100)
default_volume = 50

[playback]
# Maximum queue size (number of passages)
max_queue_size = 100

# Crossfade duration in milliseconds
crossfade_ms = 3000

[logging]
# Log level: trace, debug, info, warn, error
level = "info"

# Log file path (empty = stdout only)
log_file = ""
```

### 2.3. Module 2: User Interface Configuration

**[DEP-CFG-200]** Configuration file: `ui.toml`

```toml
[server]
port = 8080
bind_address = "0.0.0.0"  # Accept connections from any IP

[database]
path = "~/.local/share/mcrhythm/mcrhythm.db"

[modules]
# URLs for other modules
audio_player_url = "http://localhost:8081"
program_director_url = "http://localhost:8082"
file_ingest_url = "http://localhost:8083"

[session]
# Session timeout in seconds (1 year default)
timeout_seconds = 31536000

# Secret key for session encryption (auto-generated if not provided)
secret_key = ""

[static]
# Path to static web assets (HTML, CSS, JS)
assets_path = "/usr/local/share/mcrhythm/ui/"

[logging]
level = "info"
log_file = ""
```

### 2.4. Module 3: Program Director Configuration

**[DEP-CFG-300]** Configuration file: `program-director.toml`

```toml
[server]
port = 8082
bind_address = "127.0.0.1"

[database]
path = "~/.local/share/mcrhythm/mcrhythm.db"

[modules]
audio_player_url = "http://localhost:8081"

[director]
# How often to check if queue needs refilling (milliseconds)
check_interval_ms = 5000

# Minimum passages to maintain in queue
min_queue_size = 3

# Maximum passages to enqueue at once
max_enqueue_batch = 5

[logging]
level = "info"
log_file = ""
```

### 2.5. Module 4: File Ingest Interface Configuration

**[DEP-CFG-400]** Configuration file: `file-ingest.toml`

```toml
[server]
port = 8083
bind_address = "127.0.0.1"

[database]
path = "~/.local/share/mcrhythm/mcrhythm.db"

[ingest]
# Temporary directory for file processing
temp_path = "~/.local/share/mcrhythm/temp/"

# Maximum concurrent file processing jobs
max_concurrent_jobs = 4

[essentia]
# Path to Essentia extractor binary
extractor_path = "/usr/local/bin/essentia_streaming_extractor_music"

[logging]
level = "info"
log_file = ""
```

## 3. Database Location

**[DEP-DB-010]** The SQLite database file is shared by all modules and shall be located at:
- **Linux**: `~/.local/share/mcrhythm/mcrhythm.db`
- **macOS**: `~/Library/Application Support/McRhythm/mcrhythm.db`
- **Windows**: `%APPDATA%\McRhythm\mcrhythm.db`

**[DEP-DB-020]** All modules must be configured to use the same database path.

**[DEP-DB-030]** The database directory shall be created automatically if it does not exist.

## 4. Startup Order and Dependencies

**[DEP-START-010]** Modules have the following startup dependencies:

```
Module 1: Audio Player
  └─ No dependencies (can start independently)

Module 2: User Interface
  ├─ Depends on: Module 1 (Audio Player)
  └─ Optional: Module 3 (Program Director)

Module 3: Program Director
  └─ Depends on: Module 1 (Audio Player)

Module 4: File Ingest Interface
  └─ No runtime dependencies (operates independently)
```

**[DEP-START-020]** Recommended startup order:
1. Module 1: Audio Player
2. Module 3: Program Director (if running)
3. Module 4: File Ingest Interface (if running)
4. Module 2: User Interface

**[DEP-START-030]** Modules shall implement startup retry logic with exponential backoff when dependencies are not yet available.

**[DEP-START-040]** Each module shall perform a health check on startup:
- Verify database accessibility
- Verify configuration validity
- Verify listening port availability
- Log startup status

## 5. Process Management on Linux (systemd)

### 5.1. Service Unit Files

**[DEP-LINUX-010]** systemd service unit files shall be installed at `/etc/systemd/system/`.

**[DEP-LINUX-020]** Module 1: `/etc/systemd/system/mcrhythm-audio-player.service`

```ini
[Unit]
Description=McRhythm Audio Player (Module 1)
After=network.target sound.target

[Service]
Type=simple
User=mcrhythm
Group=audio
ExecStart=/usr/local/bin/mcrhythm-audio-player
Restart=on-failure
RestartSec=5
StandardOutput=journal
StandardError=journal

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=read-only
ReadWritePaths=/home/mcrhythm/.local/share/mcrhythm /home/mcrhythm/.config/mcrhythm

[Install]
WantedBy=multi-user.target
```

**[DEP-LINUX-030]** Module 2: `/etc/systemd/system/mcrhythm-ui.service`

```ini
[Unit]
Description=McRhythm User Interface (Module 2)
After=network.target mcrhythm-audio-player.service
Requires=mcrhythm-audio-player.service

[Service]
Type=simple
User=mcrhythm
Group=mcrhythm
ExecStart=/usr/local/bin/mcrhythm-ui
Restart=on-failure
RestartSec=5
StandardOutput=journal
StandardError=journal

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=read-only
ReadWritePaths=/home/mcrhythm/.local/share/mcrhythm /home/mcrhythm/.config/mcrhythm

[Install]
WantedBy=multi-user.target
```

**[DEP-LINUX-040]** Module 3: `/etc/systemd/system/mcrhythm-program-director.service`

```ini
[Unit]
Description=McRhythm Program Director (Module 3)
After=network.target mcrhythm-audio-player.service
Requires=mcrhythm-audio-player.service

[Service]
Type=simple
User=mcrhythm
Group=mcrhythm
ExecStart=/usr/local/bin/mcrhythm-program-director
Restart=on-failure
RestartSec=5
StandardOutput=journal
StandardError=journal

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=read-only
ReadWritePaths=/home/mcrhythm/.local/share/mcrhythm /home/mcrhythm/.config/mcrhythm

[Install]
WantedBy=multi-user.target
```

**[DEP-LINUX-050]** Module 4: `/etc/systemd/system/mcrhythm-file-ingest.service`

```ini
[Unit]
Description=McRhythm File Ingest Interface (Module 4)
After=network.target

[Service]
Type=simple
User=mcrhythm
Group=mcrhythm
ExecStart=/usr/local/bin/mcrhythm-file-ingest
Restart=on-failure
RestartSec=5
StandardOutput=journal
StandardError=journal

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=read-only
ReadWritePaths=/home/mcrhythm/.local/share/mcrhythm /home/mcrhythm/.config/mcrhythm

[Install]
WantedBy=multi-user.target
```

### 5.2. systemd Commands

**[DEP-LINUX-060]** Enable and start all services (Full version):

```bash
sudo systemctl daemon-reload
sudo systemctl enable mcrhythm-audio-player mcrhythm-ui mcrhythm-program-director mcrhythm-file-ingest
sudo systemctl start mcrhythm-audio-player mcrhythm-ui mcrhythm-program-director mcrhythm-file-ingest
```

**[DEP-LINUX-070]** Check service status:

```bash
sudo systemctl status mcrhythm-audio-player
sudo systemctl status mcrhythm-ui
sudo systemctl status mcrhythm-program-director
sudo systemctl status mcrhythm-file-ingest
```

**[DEP-LINUX-080]** View logs:

```bash
sudo journalctl -u mcrhythm-audio-player -f
sudo journalctl -u mcrhythm-ui -f
```

**[DEP-LINUX-090]** Stop and disable services:

```bash
sudo systemctl stop mcrhythm-audio-player mcrhythm-ui mcrhythm-program-director mcrhythm-file-ingest
sudo systemctl disable mcrhythm-audio-player mcrhythm-ui mcrhythm-program-director mcrhythm-file-ingest
```

## 6. Process Management on macOS (launchd)

### 6.1. Launch Agent Files

**[DEP-MACOS-010]** launchd property list files shall be installed at `~/Library/LaunchAgents/` for user-level services.

**[DEP-MACOS-020]** Module 1: `~/Library/LaunchAgents/com.mcrhythm.audio-player.plist`

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.mcrhythm.audio-player</string>

    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/mcrhythm-audio-player</string>
    </array>

    <key>RunAtLoad</key>
    <true/>

    <key>KeepAlive</key>
    <dict>
        <key>SuccessfulExit</key>
        <false/>
    </dict>

    <key>StandardOutPath</key>
    <string>/tmp/mcrhythm-audio-player.log</string>

    <key>StandardErrorPath</key>
    <string>/tmp/mcrhythm-audio-player-error.log</string>

    <key>ThrottleInterval</key>
    <integer>5</integer>
</dict>
</plist>
```

**[DEP-MACOS-030]** Module 2: `~/Library/LaunchAgents/com.mcrhythm.ui.plist`

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.mcrhythm.ui</string>

    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/mcrhythm-ui</string>
    </array>

    <key>RunAtLoad</key>
    <true/>

    <key>KeepAlive</key>
    <dict>
        <key>SuccessfulExit</key>
        <false/>
    </dict>

    <key>StandardOutPath</key>
    <string>/tmp/mcrhythm-ui.log</string>

    <key>StandardErrorPath</key>
    <string>/tmp/mcrhythm-ui-error.log</string>

    <key>ThrottleInterval</key>
    <integer>5</integer>
</dict>
</plist>
```

**[DEP-MACOS-040]** Similar plist files for Module 3 and Module 4 follow the same pattern.

### 6.2. launchctl Commands

**[DEP-MACOS-050]** Load and start services:

```bash
launchctl load ~/Library/LaunchAgents/com.mcrhythm.audio-player.plist
launchctl load ~/Library/LaunchAgents/com.mcrhythm.ui.plist
launchctl load ~/Library/LaunchAgents/com.mcrhythm.program-director.plist
launchctl load ~/Library/LaunchAgents/com.mcrhythm.file-ingest.plist
```

**[DEP-MACOS-060]** Check service status:

```bash
launchctl list | grep mcrhythm
```

**[DEP-MACOS-070]** Stop and unload services:

```bash
launchctl unload ~/Library/LaunchAgents/com.mcrhythm.audio-player.plist
launchctl unload ~/Library/LaunchAgents/com.mcrhythm.ui.plist
launchctl unload ~/Library/LaunchAgents/com.mcrhythm.program-director.plist
launchctl unload ~/Library/LaunchAgents/com.mcrhythm.file-ingest.plist
```

## 7. Process Management on Windows

### 7.1. Windows Services

**[DEP-WIN-010]** On Windows, modules may be run as Windows Services using a service wrapper such as NSSM (Non-Sucking Service Manager) or WinSW.

**[DEP-WIN-020]** Example using NSSM to install Module 1 as a service:

```cmd
nssm install McRhythmAudioPlayer "C:\Program Files\McRhythm\bin\mcrhythm-audio-player.exe"
nssm set McRhythmAudioPlayer AppDirectory "C:\Program Files\McRhythm\bin"
nssm set McRhythmAudioPlayer DisplayName "McRhythm Audio Player"
nssm set McRhythmAudioPlayer Description "McRhythm Audio Player Module (Module 1)"
nssm set McRhythmAudioPlayer Start SERVICE_AUTO_START
nssm start McRhythmAudioPlayer
```

**[DEP-WIN-030]** Services can be managed via the Windows Services control panel (`services.msc`) or command line:

```cmd
# Start services
net start McRhythmAudioPlayer
net start McRhythmUI
net start McRhythmProgramDirector
net start McRhythmFileIngest

# Stop services
net stop McRhythmAudioPlayer
net stop McRhythmUI
net stop McRhythmProgramDirector
net stop McRhythmFileIngest

# Check status
sc query McRhythmAudioPlayer
```

### 7.2. Windows Service Dependencies

**[DEP-WIN-040]** Configure service dependencies using NSSM or sc command:

```cmd
# Module 2 depends on Module 1
sc config McRhythmUI depend= McRhythmAudioPlayer

# Module 3 depends on Module 1
sc config McRhythmProgramDirector depend= McRhythmAudioPlayer
```

## 8. Manual Startup (Development/Testing)

### 8.1. Command Line Execution

**[DEP-MANUAL-010]** For development or testing, modules can be started manually from the command line.

**[DEP-MANUAL-020]** Start modules in recommended order:

```bash
# Terminal 1: Audio Player
mcrhythm-audio-player

# Terminal 2: Program Director (optional)
mcrhythm-program-director

# Terminal 3: File Ingest Interface (optional, Full version only)
mcrhythm-file-ingest

# Terminal 4: User Interface
mcrhythm-ui
```

**[DEP-MANUAL-030]** Each module accepts command-line arguments to override configuration:

```bash
mcrhythm-audio-player --port 9081 --config /path/to/custom/config.toml
mcrhythm-ui --port 9080 --bind-address 0.0.0.0
```

**[DEP-MANUAL-040]** Use `--help` flag to see all available command-line options:

```bash
mcrhythm-audio-player --help
```

### 8.2. Environment Variables

**[DEP-MANUAL-050]** Modules support environment variable configuration:

```bash
export MCRHYTHM_AUDIO_PLAYER_PORT=9081
export MCRHYTHM_DATABASE_PATH=/custom/path/mcrhythm.db
export MCRHYTHM_LOG_LEVEL=debug

mcrhythm-audio-player
```

**[DEP-MANUAL-060]** Configuration precedence (highest to lowest):
1. Command-line arguments
2. Environment variables
3. User configuration file
4. System configuration file
5. Built-in defaults

## 9. Health Checks and Monitoring

### 9.1. Health Check Endpoints

**[DEP-HEALTH-010]** Each module exposes a health check endpoint at `GET /health` that returns:
- HTTP 200 OK if the module is healthy
- HTTP 503 Service Unavailable if the module is unhealthy

**[DEP-HEALTH-020]** Health check response format (JSON):

```json
{
  "status": "healthy",
  "module": "audio-player",
  "version": "1.0.0",
  "uptime_seconds": 3600,
  "checks": {
    "database": "ok",
    "audio_device": "ok",
    "dependencies": "ok"
  }
}
```

**[DEP-HEALTH-030]** A module is considered unhealthy if:
- Database connection fails
- Required dependencies (other modules) are unreachable
- Critical resources are unavailable (e.g., audio device for Module 1)

### 9.2. Monitoring

**[DEP-MON-010]** Operators should monitor:
- Module process status (running/stopped)
- HTTP health check endpoints
- Log files for errors and warnings
- CPU and memory usage
- Database size and growth rate
- Queue depth (Module 1)
- Request latency (all modules)

**[DEP-MON-020]** Recommended monitoring tools:
- **Linux**: systemd status, journalctl, prometheus + node_exporter
- **macOS**: launchctl, Console.app, Activity Monitor
- **Windows**: Services control panel, Event Viewer, Performance Monitor

**[DEP-MON-030]** Log rotation should be configured to prevent disk space exhaustion:
- **Linux**: Use logrotate with systemd journal
- **macOS**: Use newsyslog or manual log rotation
- **Windows**: Configure log file size limits in application configuration

## 10. Graceful Shutdown

**[DEP-SHUTDOWN-010]** All modules shall handle SIGTERM (Linux/macOS) or SERVICE_CONTROL_STOP (Windows) signals for graceful shutdown.

**[DEP-SHUTDOWN-020]** On receiving a shutdown signal, each module shall:
1. Stop accepting new HTTP requests
2. Complete in-flight requests (with timeout)
3. For Module 1: Save playback position and queue state
4. Close database connections
5. Release system resources (audio devices, file handles)
6. Log shutdown completion
7. Exit with status code 0

**[DEP-SHUTDOWN-030]** Shutdown timeout: 30 seconds. If graceful shutdown does not complete within 30 seconds, the process may be forcibly terminated.

**[DEP-SHUTDOWN-040]** Recommended shutdown order (reverse of startup):
1. Module 2: User Interface
2. Module 3: Program Director
3. Module 4: File Ingest Interface
4. Module 1: Audio Player

## 11. Backup and Recovery

### 11.1. Database Backup

**[DEP-BACKUP-010]** The SQLite database should be backed up regularly using one of these methods:
- `sqlite3` command-line tool: `sqlite3 mcrhythm.db ".backup mcrhythm-backup.db"`
- `VACUUM INTO` SQL command: `VACUUM INTO '/path/to/backup.db'`
- File-level copy (only when all modules are stopped)

**[DEP-BACKUP-020]** Backup schedule recommendations:
- **Daily**: Automated backup during low-usage hours
- **Before major operations**: Before file ingest, before software updates
- **On-demand**: User-initiated backup via UI or CLI tool

### 11.2. Configuration Backup

**[DEP-BACKUP-030]** Configuration files should be backed up separately from the database, as they are small and change infrequently.

**[DEP-BACKUP-040]** Backup locations should be outside the application data directory to survive application reinstallation.

### 11.3. Recovery

**[DEP-RECOVERY-010]** To restore from backup:
1. Stop all McRhythm modules
2. Replace `mcrhythm.db` with backup file
3. Verify database integrity: `sqlite3 mcrhythm.db "PRAGMA integrity_check"`
4. Restart modules in recommended startup order

**[DEP-RECOVERY-020]** If database is corrupted and no backup exists:
1. Create a new empty database by starting Module 1 (it will initialize the schema)
2. Re-ingest music files using Module 4
3. User likes/dislikes and taste profiles will be lost

## 12. Version-Specific Deployments

### 12.1. Full Version

**[DEP-VER-FULL-010]** Deploy and enable all 4 modules:
- Module 1: Audio Player (required)
- Module 2: User Interface (required)
- Module 3: Program Director (required)
- Module 4: File Ingest Interface (required)

**[DEP-VER-FULL-020]** Recommended for:
- Personal music servers with local file collections
- Installations requiring file ingest and characterization
- Full-featured deployments

### 12.2. Lite Version

**[DEP-VER-LITE-010]** Deploy modules 1, 2, and 3 only:
- Module 1: Audio Player (required)
- Module 2: User Interface (required)
- Module 3: Program Director (required)

**[DEP-VER-LITE-020]** File ingest must be performed by a separate Full version installation or manual database population.

**[DEP-VER-LITE-030]** Recommended for:
- Playback-only installations
- Embedded devices with limited resources
- Remote players accessing a centralized database

### 12.3. Minimal Version

**[DEP-VER-MIN-010]** Deploy modules 1 and 2 only:
- Module 1: Audio Player (required)
- Module 2: User Interface (required)

**[DEP-VER-MIN-020]** No automatic passage selection; user manually enqueues all passages via UI.

**[DEP-VER-MIN-030]** Recommended for:
- Simple playback scenarios
- Testing and development
- Minimal resource environments

## 13. Security Considerations

**[DEP-SEC-010]** Module 1 (Audio Player), Module 3 (Program Director), and Module 4 (File Ingest) should only bind to `127.0.0.1` (localhost) by default, as they are not designed for direct external access.

**[DEP-SEC-020]** Module 2 (User Interface) may bind to `0.0.0.0` (all interfaces) for remote access, but should be protected by:
- HTTPS/TLS encryption (reverse proxy recommended)
- Authentication (enforced by application)
- Firewall rules limiting access to trusted networks

**[DEP-SEC-030]** Consider deploying a reverse proxy (nginx, Apache, Caddy) in front of Module 2 for:
- TLS termination
- Rate limiting
- IP-based access control
- Static asset caching

**[DEP-SEC-040]** Database file permissions should be restricted to the user account running McRhythm modules (chmod 600 or equivalent).

**[DEP-SEC-050]** Configuration files containing sensitive data (session secret keys) should be protected with restrictive permissions (chmod 600 or equivalent).

## 14. Network Architecture

**[DEP-NET-010]** Typical deployment network diagram:

```
┌─────────────────────────────────────────────────────────────┐
│                         Host System                          │
│                                                               │
│  ┌─────────────┐                                             │
│  │  Module 2:  │ ← External HTTP requests (port 8080)        │
│  │     UI      │                                             │
│  └──────┬──────┘                                             │
│         │                                                     │
│         ├──────→ Module 1: Audio Player (localhost:8081)     │
│         ├──────→ Module 3: Program Director (localhost:8082) │
│         └──────→ Module 4: File Ingest (localhost:8083)      │
│                                                               │
│  ┌────────────────────────────────────────┐                  │
│  │  Module 3: Program Director            │                  │
│  │      ↓                                  │                  │
│  │  Module 1: Audio Player (direct call)  │                  │
│  └────────────────────────────────────────┘                  │
│                                                               │
│  ┌──────────────────────────────────────────────────────┐    │
│  │          Shared SQLite Database                       │    │
│  │  ~/.local/share/mcrhythm/mcrhythm.db                 │    │
│  └──────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
```

**[DEP-NET-020]** Port configuration must be consistent across all module configuration files. If Module 2 is configured to call Module 1 at `http://localhost:9081`, then Module 1 must be configured to listen on port 9081.

**[DEP-NET-030]** For distributed deployments (modules on separate hosts), update the `[modules]` section URLs in configuration files to use appropriate hostnames or IP addresses instead of `localhost`.

---

End of document - Deployment and Process Management
