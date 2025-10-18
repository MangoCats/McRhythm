# Deployment and Process Management

**ðŸš€ TIER 3 - IMPLEMENTATION SPECIFICATION**

Defines deployment, process management, and operational configuration for WKMP's microservices architecture. See [Document Hierarchy](GOV001-document_hierarchy.md).

> **Related Documentation:** [Architecture](SPEC001-architecture.md) | [API Design](SPEC007-api_design.md) | [Requirements](REQ001-requirements.md)

---

## Overview

**[DEP-OVR-010]** WKMP consists of **5 independent microservices** that communicate via HTTP/REST APIs. This document specifies how to deploy, start, stop, configure, and monitor these processes across different operating systems and deployment scenarios.

**[DEP-OVR-020]** The version (Full, Lite, Minimal) determines which of the **5 microservices** run:
- **Full Version** (all 5): Audio Player, User Interface, Lyric Editor, Program Director, Audio Ingest
- **Lite Version** (3 of 5): Audio Player, User Interface, Program Director
- **Minimal Version** (2 of 5): Audio Player, User Interface

## 1. Module Binaries

**[DEP-BIN-010]** Each module is compiled as a separate executable binary:
- `wkmp-ap` - Audio Player
- `wkmp-ui` - User Interface
- `wkmp-le` - Lyric Editor
- `wkmp-pd` - Program Director
- `wkmp-ai` - Audio Ingest

**[DEP-BIN-020]** Binaries shall be installed in a standard location:
- **Linux**: `/usr/local/bin/` or `/opt/wkmp/bin/`
- **macOS**: `/usr/local/bin/` or `/Applications/WKMP.app/Contents/MacOS/`
- **Windows**: `C:\Program Files\WKMP\bin\`

## 2. Configuration Files

### 2.1. Configuration File Location

**[DEP-CFG-010]** Each module reads its configuration from a TOML file located at:
- **Linux**: `~/.config/wkmp/<module-name>.toml`
- **macOS**: `~/Library/Application Support/WKMP/<module-name>.toml`
- **Windows**: `%APPDATA%\WKMP\<module-name>.toml`

**[DEP-CFG-020]** System-wide default configurations may be placed at:
- **Linux**: `/etc/wkmp/<module-name>.toml`
- **macOS**: `/Library/Application Support/WKMP/<module-name>.toml`
- **Windows**: `C:\ProgramData\WKMP\<module-name>.toml`

**[DEP-CFG-030]** User-specific configuration files override system-wide defaults.

**[DEP-CFG-031]** Graceful degradation for missing configuration files [REQ-NF-031, REQ-NF-032]:
- **Behavior**: If TOML configuration file does not exist, module SHALL NOT terminate with error
- **Action**: Log warning message indicating missing file, proceed with compiled default values
- **Example warning**: `WARN: Config file not found at ~/.config/wkmp/audio-player.toml, using default configuration`
- **Result**: Module starts successfully using compiled defaults (see [DEP-CFG-040] below)

### 2.1a. Module Discovery via Database

**[DEP-CFG-035]** Modules discover each other's network addresses through the shared SQLite database `module_config` table.

**On startup, each module:**
1. Resolves root folder path using priority order (CLI args > env vars > config file > compiled defaults) [REQ-NF-035]
2. Creates root folder directory if missing [REQ-NF-036]
3. Opens database (`wkmp.db`) in root folder, creating empty database if necessary [REQ-NF-036]
4. Queries `module_config` for its own host/port and other modules' addresses
5. Inserts defaults if entries missing (auto-initialization)
6. Binds to configured port and begins accepting connections

**Graceful first-run behavior** [REQ-NF-032, REQ-NF-036]:
- Missing config file → Log warning, use defaults
- Missing root folder → Create directory automatically
- Missing database → Initialize with default schema
- Missing tables/settings → Auto-create via idempotent initialization
- **Result**: Module starts successfully with zero manual configuration required

**Default module configuration (using first base port 5720):**

| Module | Host | Port | Notes |
|--------|------|------|-------|
| user_interface | 127.0.0.1 | 5720 | Configurable to 0.0.0.0 for network access |
| audio_player | 127.0.0.1 | 5721 | Internal service (localhost-only by default) |
| program_director | 127.0.0.1 | 5722 | Internal service (localhost-only by default) |
| audio_ingest | 0.0.0.0 | 5723 | Network accessible by default |
| lyric_editor | 0.0.0.0 | 5724 | Network accessible by default |

**Benefits:**
- No duplication of module URLs in TOML config files
- Configuration changes via database updates, not file edits
- wkmp-ui can launch modules even when configs missing (auto-insert defaults)
- All audio files and artwork stored within root folder tree

> **See Also:**
> - [Database Schema - module_config Table](IMPL001-database_schema.md#module_config) - Complete table definition, constraints, initialization behavior
> - [Section 13: HTTP Server Configuration](#13-http-server-configuration) - Port selection algorithm, fallback ports, duplicate instance detection
> - [Database Schema - File System Organization](IMPL001-database_schema.md#file-system-organization) - Root folder structure

### 2.2. Audio Player Configuration

**[DEP-CFG-100]** Configuration file: `audio-player.toml`

**Note:** This file is optional [REQ-NF-031]. If missing, module uses compiled defaults.

```toml
[root_folder]
# Path to the root folder containing the database and all audio/artwork files
# Default (if this file is missing): ~/Music (Linux/macOS), %USERPROFILE%\Music\wkmp (Windows)
path = "~/Music"
# Database file is located at: {root_folder}/wkmp.db

[logging]
# Log level: trace, debug, info, warn, error
# Default (if this file is missing): info
level = "info"

# Log file path (empty = stdout only)
# Default (if this file is missing): "" (stdout only)
log_file = ""
```

**[DEP-CFG-040]** Compiled default configuration values [REQ-NF-033, REQ-NF-034]:

When TOML config file is missing, modules use these compiled defaults:

**Root folder location** [REQ-NF-033]:
- Linux: `~/Music`
- macOS: `~/Music`
- Windows: `%USERPROFILE%\Music\wkmp`

**Logging configuration** [REQ-NF-034]:
- Log level: `info`
- Log file: stdout only (no file logging)

**Static assets** [REQ-NF-034]:
- Linux: `/usr/local/share/wkmp/<module-name>/`
- macOS: `/Applications/WKMP.app/Contents/Resources/<module-name>/`
- Windows: `C:\Program Files\WKMP\share\<module-name>\`

**Configuration Source of Truth:**
- **Database `settings` table**: Volume level, audio output device, crossfade time, all playback and fade settings, queue limits, and ALL runtime configuration
- **Database `module_config` table**: Server port and bind address for all modules
- **TOML config file**: Root folder path, logging, static asset paths ONLY (NEVER runtime settings that belong in database)
- **Compiled defaults**: Used when TOML config file is missing [REQ-NF-032]

**Runtime settings in database:**
- `queue_max_size`: Maximum queue size (default: 100)
- See [database_schema.md - settings table](IMPL001-database_schema.md#settings) for complete list

**Precedence:** Database is the source of truth for ALL runtime settings. TOML files MUST NOT provide any values which are stored in database. TOML provides only bootstrap configuration (root folder path, logging, static asset paths). When database settings are missing, NULL, or the database does not exist, the application SHALL initialize them with built-in default values and write those defaults to the database.

### 2.3. User Interface Configuration

**[DEP-CFG-200]** Configuration file: `ui.toml`

**Note:** This file is optional [REQ-NF-031]. If missing, module uses compiled defaults.

```toml
[root_folder]
# Path to the root folder containing the database and all audio/artwork files
# Default (if this file is missing): ~/Music (Linux/macOS), %USERPROFILE%\Music\wkmp (Windows)
path = "~/Music"
# Database file is located at: {root_folder}/wkmp.db

[session]
# Secret key for session encryption (auto-generated if not provided)
secret_key = ""

[static]
# Path to static web assets (HTML, CSS, JS)
# Default (if this file is missing): /usr/local/share/wkmp/ui/ (Linux)
assets_path = "/usr/local/share/wkmp/ui/"

[logging]
# Default (if this file is missing): info
level = "info"
# Default (if this file is missing): "" (stdout only)
log_file = ""
```

**Runtime settings in database:**
- `session_timeout_seconds`: Session timeout duration (default: 31536000 = 1 year)
- See [database_schema.md - settings table](IMPL001-database_schema.md#settings) for complete list

**Note:** Server port and bind address are read from the `module_config` table in the database. Other module URLs are also read from the database, eliminating the need for `[server]` and `[modules]` sections in the config file.

### 2.4. Program Director Configuration

**[DEP-CFG-300]** Configuration file: `program-director.toml`

**Note:** This file is optional [REQ-NF-031]. If missing, module uses compiled defaults.

```toml
[root_folder]
# Path to the root folder containing the database and all audio/artwork files
# Default (if this file is missing): ~/Music (Linux/macOS), %USERPROFILE%\Music\wkmp (Windows)
path = "~/Music"
# Database file is located at: {root_folder}/wkmp.db

[logging]
# Default (if this file is missing): info
level = "info"
# Default (if this file is missing): "" (stdout only)
log_file = ""
```

**Runtime settings in database:**
- `queue_refill_threshold_passages`: Min passages before refill (default: 2)
- `queue_refill_threshold_seconds`: Min seconds before refill (default: 900)
- `queue_refill_request_throttle_seconds`: Min interval between requests (default: 10)
- `queue_max_enqueue_batch`: Maximum passages to enqueue at once (default: 5)
- See [database_schema.md - settings table](IMPL001-database_schema.md#settings) for complete list

**Note:** Server port, bind address, and Audio Player URL are read from the `module_config` table in the database. All queue management and refill behavior is configured via database settings table, not TOML.

### 2.5. Audio Ingest Configuration

**[DEP-CFG-400]** Configuration file: `file-ingest.toml`

**Note:** This file is optional [REQ-NF-031]. If missing, module uses compiled defaults.

```toml
[root_folder]
# Path to the root folder containing the database and all audio/artwork files
# Default (if this file is missing): ~/Music (Linux/macOS), %USERPROFILE%\Music\wkmp (Windows)
path = "~/Music"
# Database file is located at: {root_folder}/wkmp.db

[ingest]
# Temporary directory for file processing (within root folder)
# Default (if this file is missing): "temp/"
temp_path = "temp/"

[essentia]
# Path to Essentia extractor binary
# Default (if this file is missing): /usr/local/bin/essentia_streaming_extractor_music
extractor_path = "/usr/local/bin/essentia_streaming_extractor_music"

[logging]
# Default (if this file is missing): info
level = "info"
# Default (if this file is missing): "" (stdout only)
log_file = ""
```

**Runtime settings in database:**
- `ingest_max_concurrent_jobs`: Maximum concurrent file processing jobs (default: 4)
- See [database_schema.md - settings table](IMPL001-database_schema.md#settings) for complete list

**Note:** Server port and bind address are read from the `module_config` table in the database.

## 3. Database Location

**[DEP-DB-010]** The SQLite database file is shared by all modules and shall be located at `{root_folder}/wkmp.db`.

**Default root folder locations** [REQ-NF-033]:
- **Linux**: `~/Music/wkmp.db`
- **macOS**: `~/Music/wkmp.db`
- **Windows**: `%USERPROFILE%\Music\wkmp.db`

**Alternative deployment locations** (via config file or environment variable):
- **Linux system-wide**: `/var/lib/wkmp/wkmp.db`
- **Linux user XDG**: `~/.local/share/wkmp/wkmp.db`
- **macOS user**: `~/Library/Application Support/WKMP/wkmp.db`
- **Windows user AppData**: `%APPDATA%\WKMP\wkmp.db`

**[DEP-DB-011]** Database initialization [REQ-NF-036]:
- If database file does not exist, module creates it automatically with default schema
- All modules use idempotent table initialization (safe to run multiple times)
- First module to start creates initial tables, subsequent modules verify/add their tables

**Note:** These paths represent the default root folder locations. The database file `wkmp.db` is always located in the root folder, and all audio/artwork files are organized under the root folder tree.

**[DEP-DB-020]** All modules must be configured to use the same root folder path.

**[DEP-DB-030]** The root folder (and database directory) shall be created automatically if it does not exist.

## 4. Startup Order and Dependencies

**[DEP-START-010]** Modules have the following startup dependencies:

```
User Interface
  ├─ Depends on: Audio Player to play audio, but has independent non audio playing functionality
  └─ Optional: Program Director

Audio Player
  └─ No dependencies (can start and run independently)

Program Director
  └─ Depends on: Audio Player, there's no point in selecting passsages for play if there's no audio player to enqueue them to.

Audio Ingest
  └─ No runtime dependencies (operates independently)
```

**[DEP-START-020]** Recommended startup order:
1. User Interface
2. Audio Player
3. Program Director (if running)

Note: Audio Ingest and Lyric Editor only start when called for (from User Interface)

**[DEP-START-030]** Modules shall implement startup retry logic when dependencies are not yet available.

**[DEP-START-040]** Each module shall perform a health check on startup:
- Verify database accessibility
- Verify configuration validity
- Verify listening port availability
- Log startup status

## 5. Process Management on Linux (systemd)

### 5.1. Service Unit File

**[DEP-LINUX-010]** Only the User Interface requires a systemd service file. All other modules are launched automatically by wkmp-ui or other modules as needed.

**[DEP-LINUX-020]** User Interface: `/etc/systemd/system/wkmp-ui.service`

```ini
[Unit]
Description=WKMP Music Player - User Interface
After=network.target sound.target
Documentation=https://github.com/mcrhythm/mcrhythm

[Service]
Type=simple
User=wkmp
Group=wkmp
ExecStart=/usr/local/bin/wkmp-ui
Restart=on-failure
RestartSec=5
StandardOutput=journal
StandardError=journal

# wkmp-ui will automatically launch other required modules:
# - wkmp-ap (Audio Player)
# - wkmp-pd (Program Director, Lite/Full only)
# - wkmp-ai (Audio Ingest, on-demand, Full only)
# - wkmp-le (Lyric Editor, on-demand, Full only)

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=read-only
ReadWritePaths=/home/wkmp/.local/share/wkmp /home/wkmp/.config/wkmp

[Install]
WantedBy=multi-user.target
```

**[DEP-LINUX-030]** ~~Audio Player: `/etc/systemd/system/wkmp-ap.service`~~ **REMOVED** - wkmp-ap is launched by wkmp-ui

**[DEP-LINUX-030]** ~~Audio Ingest: `/etc/systemd/system/wkmp-ai.service`~~ **REMOVED** - wkmp-ai is launched by wkmp-ui

### 5.2. systemd Commands

**[DEP-LINUX-040]** Enable and start the User Interface service:

```bash
sudo systemctl daemon-reload
sudo systemctl enable wkmp-ui
sudo systemctl start wkmp-ui
```

**[DEP-LINUX-050]** Check service status:

```bash
sudo systemctl status wkmp-ui
```

**[DEP-LINUX-060]** View logs for User Interface:

```bash
sudo journalctl -u wkmp-ui -f
```

**[DEP-LINUX-070]** View logs for all modules (wkmp-ui launches them as subprocesses):

```bash
# All modules log to journal via wkmp-ui parent process
sudo journalctl -u wkmp-ui -f --all
```

**[DEP-LINUX-080]** Stop and disable service:

```bash
sudo systemctl stop wkmp-ui
sudo systemctl disable wkmp-ui
```

**Note**: Stopping wkmp-ui will also stop all modules it launched (wkmp-ap, wkmp-pd, wkmp-ai, wkmp-le).

## 6. Process Management on macOS (launchd)

### 6.1. Launch Agent File

**[DEP-MACOS-010]** Only the User Interface requires a launchd property list file. All other modules are launched automatically by wkmp-ui or other modules as needed.

**[DEP-MACOS-020]** User Interface: `~/Library/LaunchAgents/com.wkmp.ui.plist`

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.wkmp.ui</string>

    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/wkmp-ui</string>
    </array>

    <key>RunAtLoad</key>
    <true/>

    <key>KeepAlive</key>
    <dict>
        <key>SuccessfulExit</key>
        <false/>
    </dict>

    <key>StandardOutPath</key>
    <string>/tmp/wkmp-ui.log</string>

    <key>StandardErrorPath</key>
    <string>/tmp/wkmp-ui-error.log</string>

    <key>ThrottleInterval</key>
    <integer>5</integer>

    <!-- wkmp-ui will automatically launch other required modules:
         - wkmp-ap (Audio Player)
         - wkmp-pd (Program Director, Lite/Full only)
         - wkmp-ai (Audio Ingest, Full only)
         - wkmp-le (Lyric Editor, on-demand, Full only) -->
</dict>
</plist>
```

**[DEP-MACOS-030]** ~~Audio Player: `~/Library/LaunchAgents/com.wkmp.audio-player.plist`~~ **REMOVED** - wkmp-ap is launched by wkmp-ui

**[DEP-MACOS-040]** ~~Program Director and Audio Ingest plist files~~ **REMOVED** - All modules are launched by wkmp-ui

### 6.2. launchctl Commands

**[DEP-MACOS-050]** Load and start the User Interface service:

```bash
launchctl load ~/Library/LaunchAgents/com.wkmp.ui.plist
```

**[DEP-MACOS-060]** Check service status:

```bash
launchctl list | grep wkmp
```

**[DEP-MACOS-070]** Stop and unload service:

```bash
launchctl unload ~/Library/LaunchAgents/com.wkmp.ui.plist
```

**Note**: Unloading wkmp-ui will also stop all modules it launched (wkmp-ap, wkmp-pd, wkmp-ai, wkmp-le).

## 7. Process Management on Windows

### 7.1. Windows Service

**[DEP-WIN-010]** Only the User Interface requires a Windows Service. All other modules are launched automatically by wkmp-ui or other modules as needed.

**[DEP-WIN-020]** Example using NSSM to install User Interface as a service:

```cmd
nssm install WKMPUI "C:\Program Files\WKMP\bin\wkmp-ui.exe"
nssm set WKMPUI AppDirectory "C:\Program Files\WKMP\bin"
nssm set WKMPUI DisplayName "WKMP Music Player - User Interface"
nssm set WKMPUI Description "WKMP User Interface (launches all other modules automatically)"
nssm set WKMPUI Start SERVICE_AUTO_START
nssm start WKMPUI
```

**[DEP-WIN-030]** Service can be managed via the Windows Services control panel (`services.msc`) or command line:

```cmd
# Start service
net start WKMPUI

# Stop service
net stop WKMPUI

# Check status
sc query WKMPUI
```

**Note**: Stopping WKMPUI will also stop all modules it launched (wkmp-ap, wkmp-pd, wkmp-ai, wkmp-le).

### 7.2. ~~Windows Service Dependencies~~ **REMOVED**

No service dependencies needed - wkmp-ui launches all other modules as subprocesses.

## 8. Manual Startup (Development/Testing)

### 8.1. Command Line Execution

**[DEP-MANUAL-010]** For development or testing, modules can be started manually from the command line.

Note: modules will still attempt to launch other modules as they need them.

**[DEP-MANUAL-020]** Removed.

**[DEP-MANUAL-030]** Removed.

**[DEP-MANUAL-040]** Use `--help` flag to see all available command-line options:

```bash
wkmp-ap --help
```

### 8.2. Environment Variables

**[DEP-MANUAL-050]** Modules support environment variable configuration:

```bash
export WKMP_ROOT_FOLDER=/custom/path/wkmp
export WKMP_LOG_LEVEL=debug

wkmp-ap
```

**Note:** `WKMP_ROOT_FOLDER` specifies the root folder path. The database file (`wkmp.db`) is located within this folder.

**[DEP-MANUAL-060]** Configuration precedence by setting type:

**Runtime/Playback Settings** (volume, crossfade time, audio device, etc.):
1. Database settings table (source of truth)
2. Built-in defaults (if database value missing)

**Bootstrap Configuration** (root folder, logging, module discovery) [REQ-NF-035]:
1. Command-line arguments (highest priority)
2. Environment variables
3. User TOML configuration file
4. Built-in compiled defaults (lowest priority, graceful fallback) [REQ-NF-032]

**Module Network Configuration** (ports, bind addresses):
1. Database module_config table (source of truth)
2. Built-in defaults (auto-inserted if missing)

**[DEP-MANUAL-061]** Graceful degradation examples [REQ-NF-031, REQ-NF-032]:

**Example 1: Missing config file, no overrides**
```bash
# No config file at ~/.config/wkmp/audio-player.toml
# No environment variables set
# No command-line arguments

wkmp-ap

# Output:
# WARN: Config file not found at ~/.config/wkmp/audio-player.toml, using defaults
# INFO: Root folder: ~/Music
# INFO: Initialized new database: /home/user/Music/wkmp.db
# INFO: Audio Player listening on 127.0.0.1:5721
```

**Example 2: Environment variable override**
```bash
export WKMP_ROOT_FOLDER=/mnt/music
wkmp-ap

# Output:
# WARN: Config file not found at ~/.config/wkmp/audio-player.toml, using defaults
# INFO: Root folder: /mnt/music (from environment variable)
# INFO: Opened database: /mnt/music/wkmp.db
# INFO: Audio Player listening on 127.0.0.1:5721
```

**Example 3: Command-line override (highest priority)**
```bash
wkmp-ap --root-folder /opt/wkmp

# Output:
# WARN: Config file not found at ~/.config/wkmp/audio-player.toml, using defaults
# INFO: Root folder: /opt/wkmp (from command-line argument)
# INFO: Opened database: /opt/wkmp/wkmp.db
# INFO: Audio Player listening on 127.0.0.1:5721
```

**Design Philosophy:** Database is the persistent source of truth for all user-configurable runtime settings. TOML files only configure bootstrap parameters (paths, logging) and cannot override database settings.

## 9. Administrative Command-Line Tools

### 9.1. Password Reset Tool

**[DEP-ADMIN-010]** WKMP provides a command-line password reset tool for recovering access to user accounts and configuration interfaces.

**Tool Name**: `wkmp-passwd` (or `wkmp-password-reset`)

**Usage:**

```bash
# Reset password only (config_interface_access unchanged)
wkmp-passwd --user <username>

# Reset password AND enable config interface access
wkmp-passwd --user <username> --enable-config-access

# Enable config interface access for Anonymous user (no password change)
wkmp-passwd --user Anonymous --enable-config-access
```

**Functionality:**

1. **Password Reset**:
   - Prompts for new password (twice for confirmation)
   - Generates new salt
   - Computes new password_hash using same algorithm as web UI
   - Updates `users.password_hash` and `users.password_salt` in database
   - Does NOT change `config_interface_access` unless `--enable-config-access` flag is provided

2. **Configuration Interface Access Control**:
   - `--enable-config-access` flag: Sets `users.config_interface_access = 1` (enabled)
   - If flag NOT provided: `users.config_interface_access` remains unchanged
   - Works for any user, including Anonymous

3. **Anonymous User Special Handling**:
   - Tool MAY reset `config_interface_access` for Anonymous user
   - Tool MUST NOT set a password for Anonymous user
   - If user specifies `--user Anonymous` WITHOUT `--enable-config-access`: Error message explaining Anonymous cannot have a password
   - If user specifies `--user Anonymous` WITH `--enable-config-access`: Only updates `config_interface_access`, no password prompt

**Recovery Scenarios:**

- **Lost Password**: `wkmp-passwd --user myusername`
- **Configuration Interface Lockout**: `wkmp-passwd --user myusername --enable-config-access`
- **Anonymous Locked Out of Config**: `wkmp-passwd --user Anonymous --enable-config-access`
- **Both Password and Config Access Lost**: `wkmp-passwd --user myusername --enable-config-access`

**Database Location:**

The tool reads the root folder path using the same precedence as other modules:
1. Command-line argument: `--root-folder /path/to/wkmp`
2. Environment variable: `WKMP_ROOT_FOLDER`
3. User TOML configuration file
4. Built-in default

The database is located at `{root_folder}/wkmp.db`.

**Security Notes:**

- Requires filesystem access to the database file
- Should be run by the system administrator or user who owns the WKMP installation
- No network authentication required (direct database access)
- This is the single point of recovery for password and configuration interface access lockout scenarios

**Implementation Notes:**

- Tool is a standalone binary (can be separate from main modules)
- May be part of `wkmp-common` utilities or a dedicated `wkmp-tools` package
- Direct SQLite database access (does not require any WKMP modules to be running)
- Validates that user exists before prompting for password
- Returns appropriate exit codes for scripting (0 = success, non-zero = error)

See [Architecture - Configuration Interface Access Control](SPEC001-architecture.md#configuration-interface-access-control) for complete access control specification.

## 10. Health Checks and Monitoring

### 10.1. Health Check Endpoints

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
- Critical resources are unavailable (e.g., audio device for Audio Player)

### 10.2. Monitoring

**[DEP-MON-010]** Operators should monitor:
- Module process status (running/stopped)
- HTTP health check endpoints
- Log files for errors and warnings
- CPU and memory usage
- Database size and growth rate
- Queue depth (Audio Player)
- Request latency (all modules)

**[DEP-MON-020]** Recommended monitoring tools:
- **Linux**: systemd status, journalctl, prometheus + node_exporter
- **macOS**: launchctl, Console.app, Activity Monitor
- **Windows**: Services control panel, Event Viewer, Performance Monitor

**[DEP-MON-030]** Log rotation should be configured to prevent disk space exhaustion:
- **Linux**: Use logrotate with systemd journal
- **macOS**: Use newsyslog or manual log rotation
- **Windows**: Configure log file size limits in application configuration

## 11. Graceful Shutdown

**[DEP-SHUTDOWN-010]** All modules shall handle SIGTERM (Linux/macOS) or SERVICE_CONTROL_STOP (Windows) signals for graceful shutdown.

**[DEP-SHUTDOWN-020]** On receiving a shutdown signal, each module shall:
1. Stop accepting new HTTP requests
2. Complete in-flight requests (with timeout)
3. For Audio Player: Save playback position and queue state
4. Close database connections
5. Release system resources (audio devices, file handles)
6. Log shutdown completion
7. Exit with status code 0

**[DEP-SHUTDOWN-030]** Shutdown timeout: 30 seconds. If graceful shutdown does not complete within 30 seconds, the process may be forcibly terminated.

**[DEP-SHUTDOWN-040]** Recommended shutdown order:
1. Program Director
2. Audio Player
3. Audio Ingest
4. Lyric Editor
5. User Interface

## 12. Database Backup and Recovery

### 12.1. Automatic Backup Strategy

**[DEP-BACKUP-010]** WKMP implements an **automatic database backup system** managed by wkmp-ui. See [Architecture - Database Backup Strategy](SPEC001-architecture.md#arch-queue-persist-030) for complete specification.

**Key Features:**
- **On Startup**: Integrity check + conditional backup (throttled to prevent excessive wear)
- **Periodic**: Automated backup every 3 months (configurable)
- **Atomic Process**: Temp file → integrity check → atomic rename
- **Retention**: Keeps 3 timestamped backups (configurable)
- **Network Support**: Configurable backup location (local or network drive with fallback)

**Configuration (settings table):**
- `backup_location`: Backup directory path (default: same folder as wkmp.db)
- `backup_interval_ms`: Periodic backup interval (default: 90 days)
- `backup_minimum_interval_ms`: Minimum between startup backups (default: 14 days)
- `backup_retention_count`: Number of backups to keep (default: 3)
- `last_backup_timestamp_ms`: Last successful backup timestamp

> See [Architecture - ARCH-QUEUE-PERSIST-030](SPEC001-architecture.md#arch-queue-persist-030) for detailed backup algorithm, integrity checking, and failure handling.

### 12.2. Manual Backup

**[DEP-BACKUP-020]** Operators may also perform manual backups using standard SQLite tools:

**Recommended method (online backup, safe while modules running):**
```bash
sqlite3 /path/to/wkmp.db ".backup /path/to/wkmp-manual-backup.db"
```

**Alternative (VACUUM INTO, also online-safe):**
```sql
VACUUM INTO '/path/to/wkmp-manual-backup.db';
```

**File copy (only when all modules stopped):**
```bash
# Stop all modules first!
cp /path/to/wkmp.db /path/to/wkmp-backup.db
```

**[DEP-BACKUP-030]** Manual backups recommended before:
- Major software updates
- Database schema migrations
- Large-scale file ingest operations
- Manual database modifications

### 12.3. Configuration Backup

**[DEP-BACKUP-040]** Configuration files should be backed up separately:
- Located in user config directories (see section 2.1)
- Small and change infrequently
- Use standard file backup tools (rsync, tar, etc.)

**[DEP-BACKUP-050]** Backup locations should be outside the application data directory to survive application reinstallation.

### 12.4. Recovery from Backup

**[DEP-RECOVERY-010]** Automatic recovery (wkmp-ui handles this on startup):
1. wkmp-ui runs `PRAGMA integrity_check` on database
2. If corrupted: Archives corrupted DB, restores from most recent backup
3. Repeats integrity check on restored database
4. Displays progress UI to connecting users during recovery
5. Once good database verified, proceeds with normal startup

> See [Architecture - ARCH-QUEUE-PERSIST-030](SPEC001-architecture.md#arch-queue-persist-030) for automatic recovery details.

**[DEP-RECOVERY-020]** Manual recovery procedure:
1. Stop all WKMP modules
2. Verify backup integrity: `sqlite3 wkmp-backup.db "PRAGMA integrity_check"`
3. If backup is good, replace database:
   ```bash
   mv wkmp.db wkmp-corrupted-$(date +%Y%m%d).db  # Archive corrupted DB
   cp wkmp-backup.db wkmp.db                      # Restore from backup
   ```
4. Restart modules in recommended startup order (wkmp-ui first)

**[DEP-RECOVERY-030]** If database is corrupted and no backup exists:
1. Archive corrupted database: `mv wkmp.db wkmp-corrupted.db`
2. Start wkmp-ui (will initialize new empty database with schema)
3. Re-ingest music files using Audio Ingest module
4. **Data loss**: User likes/dislikes, play history, and custom passage edits will be lost

**[DEP-RECOVERY-040]** Recovery from network backup location:
- If network drive unreachable during automatic recovery, wkmp-ui falls back to local backup
- Manual recovery may require mounting network drive first
- See `backup_location` setting in database

## 13. Version-Specific Deployments

### 13.1. Full Version

**[DEP-VER-FULL-010]** Deploy and enable all 5 modules:
- Audio Player (required)
- User Interface (required)
- Lyric Editor (on-demand)
- Program Director (required)
- Audio Ingest (required)

**Note:** Lyric Editor (wkmp-le) is launched on-demand by User Interface when user requests lyric editing, not automatically at startup.

**[DEP-VER-FULL-020]** Recommended for:
- Personal music servers with local file collections
- Installations requiring file ingest and characterization
- Full-featured deployments

### 13.2. Lite Version

**[DEP-VER-LITE-010]** Deploy Audio Player, User Interface, and Program Director only:
- Audio Player (required)
- User Interface (required)
- Program Director (required)

**[DEP-VER-LITE-020]** File ingest must be performed by a separate Full version installation or manual database population.

**[DEP-VER-LITE-030]** Recommended for:
- Playback-only installations
- Embedded devices with limited resources
- Remote players accessing a centralized database

### 13.3. Minimal Version

**[DEP-VER-MIN-010]** Deploy Audio Player and User Interface only:
- Audio Player (required)
- User Interface (required)

**[DEP-VER-MIN-020]** No automatic passage selection; user manually enqueues all passages via UI.

**[DEP-VER-MIN-030]** Recommended for:
- Simple playback scenarios
- Full manual control of passage selection from the user interface
- Testing and development
- Minimal resource environments

## 13a. Runtime Dependencies

### 13a.1. Audio Runtime Dependencies (Audio Player)

**[DEP-DEP-010]** Audio Player (wkmp-ap) uses single-stream architecture with pure Rust audio processing libraries.

**Audio Processing Libraries:**

**[DEP-DEP-020]** Audio decoding, resampling, and output libraries are **statically compiled into the binary**:
- **symphonia 0.5.x**: Audio decoding (MP3, FLAC, AAC, Vorbis, Opus, WAV, AIFF, etc.)
- **rubato 0.15.x**: High-quality sample rate conversion
- **cpal 0.15.x**: Cross-platform audio output abstraction
- No separate runtime libraries or plugins needed - all code compiles into wkmp-ap binary

**Supported Audio Formats:**

**[DEP-DEP-030]** Symphonia provides built-in support for all popular audio formats:
- **MP3**: MPEG-1/2 Layer 3 audio
- **FLAC**: Free Lossless Audio Codec
- **Ogg Vorbis**: Ogg container with Vorbis codec
- **Opus**: Modern low-latency codec
- **AAC/M4A**: Advanced Audio Coding (MP4/M4A containers)
- **WAV**: Waveform Audio File Format
- **AIFF**: Audio Interchange File Format
- **WavPack**: Hybrid lossless compression
- **ALAC**: Apple Lossless Audio Codec
- **APE**: Monkey's Audio
- Additional formats via symphonia feature flags

**Platform-Specific Audio Output:**

**[DEP-DEP-050]** cpal provides unified audio output across platforms (no bundling required):
- **Linux**: PulseAudio (primary), ALSA (fallback) - requires system libraries
- **macOS**: CoreAudio - built into OS
- **Windows**: WASAPI - built into OS
- cpal automatically selects appropriate backend at runtime

**System Audio Library Requirements:**

**[DEP-DEP-060]** Audio Player requires only platform-standard audio libraries (already present on target systems):

**Linux:**
- `libasound2` (ALSA library) - typically pre-installed
- `libpulse0` (PulseAudio client library) - typically pre-installed on desktop systems

**macOS:**
- CoreAudio framework - built into macOS, no additional libraries needed

**Windows:**
- WASAPI (Windows Audio Session API) - built into Windows Vista and later, no additional libraries needed

**Deployment Simplicity:**

**[DEP-DEP-070]** Single-stream architecture simplifies deployment:
- **Single binary**: All audio processing code compiled into wkmp-ap executable
- **No plugin directories**: No need to bundle separate plugin libraries
- **No environment variables**: No `GST_PLUGIN_PATH` or similar configuration required
- **Smaller distribution**: ~15-20 MB binary vs ~100+ MB with GStreamer bundling
- **Faster startup**: No plugin discovery or registry loading
- **Consistent behavior**: Same code on all platforms (no plugin version mismatches)

### 13a.2. SQLite Bundling

**[DEP-DEP-080]** All modules require SQLite:
- Bundle SQLite library with each module binary
- Use `rusqlite` crate with bundled feature: `rusqlite = { version = "0.30", features = ["bundled"] }`
- No separate SQLite installation required

**[DEP-DEP-090]** SQLite extensions required:
- JSON1 extension (for musical flavor vector queries)
- Enabled by default in bundled SQLite

## 14. HTTP Server Configuration

**[DEP-HTTP-010]** All microservices implement HTTP servers with consistent configuration behavior. This section applies to all modules: wkmp-ui, wkmp-ap, wkmp-pd, wkmp-ai, wkmp-le.

### 14.1. Port Configuration and Selection

**[DEP-HTTP-020]** Port numbers are managed through a base port list in the settings table plus per-module offsets:

**Module Port Offsets:**
- wkmp-ui (User Interface): offset = 0
- wkmp-ap (Audio Player): offset = 1
- wkmp-pd (Program Director): offset = 2
- wkmp-ai (Audio Ingest): offset = 3
- wkmp-le (Lyric Editor): offset = 4

**[DEP-HTTP-030]** Default base port list (stored in `settings.http_base_ports` as JSON array):
```json
[5720, 15720, 25720, 17200, 23400]
```

**[DEP-HTTP-040]** Port selection algorithm on module startup:

1. **Read configuration:**
   - Read module's last successful port from `module_config.port` (if exists)

2. **Try last known port first:**
   - If module has a previously successful port in `module_config.port`, try that port first
   - Send identity request: `GET /health` to `http://localhost:{port}`
   - If response indicates same module type already running:
     - Log to stderr: `"[module_name] already running on port {port}. Multiple instances not allowed."`
     - Exit with status code 1
   - If port unavailable for other reason (no response, different service):
     - Continue to next step

3. **Try all base+offset ports:**
   - Read `http_base_ports` list from settings table (default: `[5720, 15720, 25720, 17200, 23400]`)
   - For each base port in `http_base_ports` list:
     - Calculate: `candidate_port = base_port + module_offset`
     - Skip if `candidate_port` equals the already-tried last known port
     - Attempt to bind to `candidate_port`
     - If bind succeeds:
       - Update `module_config.port` to `candidate_port`
       - Log to stdout: `"[module_name] listening on port {candidate_port}"`
       - Proceed with startup
     - If bind fails (port in use), continue to next base port

4. **All ports exhausted:**
   - If all base+offset ports fail to bind:
     - Log to stderr: `"[module_name] unable to bind to any configured port. Tried: {list_of_attempted_ports}"`
     - Exit with status code 1

**[DEP-HTTP-050]** Port persistence behavior:
- Once a module successfully binds to a port, that port is stored in `module_config.port`
- On subsequent restarts, the module tries its last successful port first
- Only tries fallback ports if last known port is unavailable
- This prevents unnecessary port hopping on normal restarts

**[DEP-HTTP-060]** Example port allocation scenario:
```
Startup attempt 1:
- wkmp-ui tries 5720 → success, stores 5720 in module_config
- wkmp-ap tries 5721 → blocked by another process
- wkmp-ap tries 15721 → success, stores 15721 in module_config
- wkmp-pd tries 5722 → success, stores 5722 in module_config

Startup attempt 2 (after restart):
- wkmp-ui tries 5720 (last known) → success
- wkmp-ap tries 15721 (last known) → success
- wkmp-pd tries 5722 (last known) → success
(No fallback port search needed)
```

### 14.2. Duplicate Instance Detection

**[DEP-HTTP-070]** Health endpoint identity response:

Each module's `GET /health` endpoint includes module identity:
```json
{
  "status": "healthy",
  "module": "audio_player",
  "version": "1.0.0",
  "uptime_seconds": 3600
}
```

**[DEP-HTTP-080]** When a module finds a port occupied, it sends `GET /health` to determine if it's another instance:
- If `module` field matches the attempting module's name → duplicate instance detected → exit
- If no response or different `module` value → port occupied by different service → try next port

### 14.3. Bind Address Configuration

**[DEP-HTTP-090]** Each module's bind address is configured in `module_config.host`:

**Default bind addresses:**
- wkmp-ui: `0.0.0.0` (accessible from network)
- wkmp-ap: `127.0.0.1` (localhost-only)
- wkmp-pd: `127.0.0.1` (localhost-only)
- wkmp-ai: `0.0.0.0` (accessible from network)
- wkmp-le: `0.0.0.0` (accessible from network)

**[DEP-HTTP-100]** Bind address semantics:
- `127.0.0.1`: Listen on localhost only (not accessible from network)
- `0.0.0.0`: Listen on all network interfaces (accessible from network)
- Specific IP (e.g., `192.168.1.10`): Listen on specific interface

**[DEP-HTTP-110]** Security rationale:
- Only wkmp-ui, -ai, -le need network access (user facing web UI)
- Internal microservices (ap, pd) should not be exposed externally
- User can configure any module to use `0.0.0.0` if needed (advanced use cases)
- User can configure any module to use `127.0.0.1` if desired (enhanced security)

### 14.4. HTTP Server Timeouts

**[DEP-HTTP-120]** Request timeout: 30 seconds
- If request processing exceeds 30 seconds, connection is closed
- Applies to all HTTP endpoints
- Prevents resource exhaustion from hung requests

**[DEP-HTTP-130]** Connection keepalive timeout: 60 seconds
- Idle connections closed after 60 seconds
- Reduces resource usage for inactive clients

### 14.5. Request Body Limits

**[DEP-HTTP-140]** Maximum request body size: 1 MB (1,048,576 bytes)
- Applies to all POST/PUT/PATCH requests
- Prevents memory exhaustion from malicious large payloads
- Sufficient for all WKMP API operations (JSON payloads are small)

**[DEP-HTTP-150]** Exceeding request body limit:
- Return HTTP 413 Payload Too Large
- Close connection immediately
- Log warning with client IP and attempted size

### 14.6. CORS Configuration

**[DEP-HTTP-160]** CORS (Cross-Origin Resource Sharing) disabled by default on all modules except wkmp-le.

**[DEP-HTTP-170]** wkmp-ui, wkmp-ai CORS policy:
- CORS disabled (same-origin only)
- Web UI assets served from same origin as API
- No cross-origin requests needed for normal operation

**[DEP-HTTP-180]** wkmp-ap, wkmp-pd CORS policy:
- CORS disabled (same-origin only)
- These modules are internal services accessed only by wkmp-ui and each other
- No browser-based cross-origin access needed

**[DEP-HTTP-190]** wkmp-le CORS policy:
- CORS **enabled** with permissive policy
- Allows cross-origin requests from any origin (`Access-Control-Allow-Origin: *`)
- Required for lyric editor workflow:
  - User opens wkmp-le interface in browser
  - User navigates to external lyric websites
  - User copies/pastes lyrics back to wkmp-le interface
  - Some lyric sites may be embedded as iframes or fetched via AJAX

**[DEP-HTTP-200]** wkmp-le CORS headers:
```
Access-Control-Allow-Origin: *
Access-Control-Allow-Methods: GET, POST, PUT, DELETE, OPTIONS
Access-Control-Allow-Headers: Content-Type, Authorization
Access-Control-Max-Age: 86400
```

### 14.7. HTTP Server Implementation

**[DEP-HTTP-210]** Server framework: Use async Rust HTTP server (e.g., `axum`, `actix-web`, `warp`)

**[DEP-HTTP-220]** Server configuration consistency:
- All modules use same HTTP server framework for consistency
- All modules implement same timeout, body limit, and error handling behavior
- All modules use same JSON serialization library

**[DEP-HTTP-230]** Graceful shutdown on SIGTERM/SIGINT:
1. Stop accepting new connections
2. Wait for in-flight requests to complete (max 5 seconds)
3. Force-close any remaining connections
4. Proceed with module shutdown sequence

### 14.8. Settings Table Schema Addition

**[DEP-HTTP-240]** Add to `settings` table common keys:

```markdown
**HTTP Server Configuration:**
- `http_base_ports`: JSON array of base port numbers (default: `[5720, 15720, 25720, 17200, 23400]`)
- `http_request_timeout_ms`: Request timeout in milliseconds (default: 30000)
- `http_keepalive_timeout_ms`: Keepalive timeout in milliseconds (default: 60000)
- `http_max_body_size_bytes`: Maximum request body size (default: 1048576)
```

**[DEP-HTTP-250]** Module-specific configuration stored in `module_config` table:
- `host`: Bind address (e.g., `127.0.0.1`, `0.0.0.0`)
- `port`: Actual bound port (updated by module on successful bind)

### 14.9. Example Configurations

**[DEP-HTTP-260]** Default configuration (single-host deployment):
```sql
-- Settings table
INSERT INTO settings (key, value) VALUES
  ('http_base_ports', '[5720, 15720, 25720, 17200, 23400]'),
  ('http_request_timeout_ms', '30000'),
  ('http_keepalive_timeout_ms', '60000'),
  ('http_max_body_size_bytes', '1048576');

-- Module config table
INSERT INTO module_config (module_name, host, port, enabled) VALUES
  ('user_interface', '0.0.0.0', 5720, 1),
  ('audio_player', '127.0.0.1', 5721, 1),
  ('program_director', '127.0.0.1', 5722, 1),
  ('audio_ingest', '127.0.0.1', 5723, 1),
  ('lyric_editor', '127.0.0.1', 5724, 1);
```

**[DEP-HTTP-270]** Remote access configuration:
```sql
-- Allow wkmp-ui to be accessed from network
UPDATE module_config SET host = '0.0.0.0' WHERE module_name = 'user_interface';

-- Keep internal services on localhost
UPDATE module_config SET host = '127.0.0.1'
  WHERE module_name IN ('audio_player', 'program_director', 'audio_ingest', 'lyric_editor');
```

**[DEP-HTTP-280]** Custom port list for restricted environments:
```sql
-- Use higher port numbers to avoid conflicts
UPDATE settings SET value = '[8080, 8090, 8100, 8110, 8120]'
  WHERE key = 'http_base_ports';
```

## 15. Security Considerations

### 15.1. Network Access Control

**[DEP-SEC-010]** Audio Player and Program Director should only bind to `127.0.0.1` (localhost) by default, as they are not designed for direct external access.

**[DEP-SEC-020]** User Interface, Audio Ingest, and Lyric Editor may bind to `0.0.0.0` (all interfaces) for remote access, but should be protected by:
- HTTPS/TLS encryption (reverse proxy strongly recommended)
- Authentication (enforced by application)
- Firewall rules limiting access to trusted networks

### 15.2. Password Protection Over HTTP

**[DEP-SEC-030]** WKMP implements **challenge-response authentication** to protect passwords when transmitted over insecure HTTP connections.

**Key Security Features:**
- Passwords never transmitted in cleartext
- Client-side SHA-256 hashing before transmission
- Single-use challenges with 60-second expiration (prevents replay attacks)
- Server never sees actual password, only `client_hash`

> See [User Identity - Password Transmission Protection](SPEC010-user_identity.md#43-password-transmission-protection) for complete protocol specification.

**[DEP-SEC-031]** Removed.

### 15.3. Removed.

**[DEP-SEC-040]** Removed.

### 15.4. Localhost-Only Deployment

**[DEP-SEC-050]** For localhost-only deployments (single-user workstations), HTTP is acceptable:
- All traffic stays on `127.0.0.1` (loopback interface)
- No network transmission, no interception risk
- Challenge-response still provides defense-in-depth
- Simplifies deployment (no certificates needed)

**[DEP-SEC-051]** Localhost deployment configuration:
```sql
-- Bind wkmp-ui to localhost only
UPDATE module_config SET host = '127.0.0.1' WHERE module_name = 'user_interface';
```

Access via: `http://localhost:5720`

### 15.5. File System Security

**[DEP-SEC-060]** Database file permissions should be restricted to the user account running WKMP modules:
```bash
chmod 600 ~/.local/share/wkmp/wkmp.db
chown wkmp:wkmp ~/.local/share/wkmp/wkmp.db
```

**[DEP-SEC-070]** Configuration files containing sensitive data should be protected with restrictive permissions:
```bash
chmod 600 ~/.config/wkmp/*.toml
```

### 15.6. Additional Security Measures

**[DEP-SEC-080]** Do not create public-facing deployments, WKMP is a local audio player, distant users have few if any reasons to access it.

## 16. Network Architecture

**[DEP-NET-010]** Typical deployment network diagram:

```
┌─────────────────────────────────────────────────────────────┐
│                         Host System                          │
│                                                               │
│  ┌─────────────┐                                             │
│  │    User     │ â† External HTTP requests (port 5720)        │
│  │  Interface  │                                             │
│  └──────┬──────┘                                             │
│         │                                                     │
│         ├──────→ Audio Player (localhost:5721)               │
│         ├──────→ Program Director (localhost:5722)           │
│         └──────→ Audio Ingest (localhost:5723)               │
│                                                               │
│  ┌────────────────────────────────────────┐                  │
│  │  Program Director                      │                  │
│  │      â†“                                  │                  │
│  │  Audio Player (direct call)            │                  │
│  └────────────────────────────────────────┘                  │
│                                                               │
│  ┌──────────────────────────────────────────────────────┐    │
│  │          Shared SQLite Database                       │    │
│  │  ~/.local/share/wkmp/wkmp.db                          │    │
│  └──────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
```

**[DEP-NET-020]** Port configuration is centralized in the `module_config` database table. All modules read their binding configuration and peer module addresses from this single source of truth.

**[DEP-NET-030]** WKMP is not designed for distributed deployments (modules on separate hosts).
- Remote network access to the SQLite database is less than ideal.
- Computational resource loads do not merit separation of the microservices on different servers

---

End of document - Deployment and Process Management
