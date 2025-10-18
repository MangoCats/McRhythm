# WKMP Configuration Examples

This directory contains example TOML configuration files for all WKMP modules.

## Important: Configuration Files are OPTIONAL

**[REQ-NF-031, REQ-NF-032]**: All WKMP modules can run without any configuration files. Missing config files will NOT cause errors - modules will log a warning and use compiled defaults.

## Zero-Configuration Quick Start

To run WKMP with zero configuration:

```bash
# Just run the module - no config file needed!
wkmp-ap

# Output:
# WARN: Config file not found at ~/.config/wkmp/audio-player.toml, using default configuration
# INFO: Root folder: /home/user/Music (compiled default)
# INFO: Creating root folder directory: /home/user/Music
# INFO: Initialized new database: /home/user/Music/wkmp.db
# INFO: Audio Player listening on 127.0.0.1:5721
```

All necessary directories and databases are created automatically with sensible defaults.

## Configuration Priority Order

WKMP uses a 4-tier priority system for root folder resolution [REQ-NF-035, ARCH-INIT-005]:

1. **Command-line argument** (highest priority)
   ```bash
   wkmp-ap --root-folder /custom/path
   ```

2. **Environment variable**
   ```bash
   export WKMP_ROOT=/custom/path
   wkmp-ap
   ```

3. **TOML configuration file**
   - Place config file at platform-specific location
   - Set `root_folder = "/custom/path"` in TOML

4. **Compiled default** (lowest priority - used when nothing else specified)
   - Linux: `~/Music`
   - macOS: `~/Music`
   - Windows: `%USERPROFILE%\Music\wkmp`

## Platform-Specific Config File Locations

Each module looks for its config file in platform-specific directories [DEP-CFG-031]:

### Linux
- **User config**: `~/.config/wkmp/<module-name>.toml`
- **System config**: `/etc/wkmp/<module-name>.toml` (not implemented yet)

Examples:
- `~/.config/wkmp/audio-player.toml`
- `~/.config/wkmp/ui.toml`
- `~/.config/wkmp/program-director.toml`

### macOS
- **User config**: `~/Library/Application Support/WKMP/<module-name>.toml`

Examples:
- `~/Library/Application Support/WKMP/audio-player.toml`
- `~/Library/Application Support/WKMP/ui.toml`

### Windows
- **User config**: `%APPDATA%\WKMP\<module-name>.toml`

Examples:
- `C:\Users\username\AppData\Roaming\WKMP\audio-player.toml`
- `C:\Users\username\AppData\Roaming\WKMP\ui.toml`

## Example Config Files

This directory contains example configuration files for each module:

- `audio-player.toml` - Audio Player (wkmp-ap) example config
- `ui.toml` - User Interface (wkmp-ui) example config
- `program-director.toml` - Program Director (wkmp-pd) example config
- `audio-ingest.toml` - Audio Ingest (wkmp-ai) example config (Full version)
- `lyric-editor.toml` - Lyric Editor (wkmp-le) example config (Full version)

## Using Example Configs

To use an example config file:

1. **Copy the example to the platform-specific location:**

   ```bash
   # Linux
   mkdir -p ~/.config/wkmp
   cp docs/examples/audio-player.toml ~/.config/wkmp/audio-player.toml

   # macOS
   mkdir -p ~/Library/Application\ Support/WKMP
   cp docs/examples/audio-player.toml ~/Library/Application\ Support/WKMP/audio-player.toml

   # Windows PowerShell
   New-Item -Path "$env:APPDATA\WKMP" -ItemType Directory -Force
   Copy-Item docs\examples\audio-player.toml "$env:APPDATA\WKMP\audio-player.toml"
   ```

2. **Edit the config file to customize settings:**

   ```bash
   # Linux/macOS
   nano ~/.config/wkmp/audio-player.toml

   # Windows
   notepad %APPDATA%\WKMP\audio-player.toml
   ```

3. **Run the module - it will now use your config:**

   ```bash
   wkmp-ap

   # Output:
   # INFO: Root folder: /custom/path (from config file)
   ```

## What Gets Configured Where

**TOML config files** configure only bootstrap parameters:
- Root folder path
- Logging level and file
- Static assets paths

**Database settings table** stores all runtime configuration:
- Volume level, crossfade time, audio device
- Queue settings, refill thresholds
- HTTP server ports, timeouts
- All playback and module behavior settings

See [IMPL001-database_schema.md](../IMPL001-database_schema.md) for complete database settings reference.

## Graceful Degradation Behavior

WKMP implements comprehensive graceful degradation [REQ-NF-031, REQ-NF-036]:

| Scenario | Behavior |
|----------|----------|
| Config file missing | Log warning, use compiled defaults, continue startup |
| Config file corrupted | Log error, use compiled defaults, continue startup |
| Root folder missing | Create directory automatically, continue startup |
| Database missing | Create database with default schema, continue startup |
| Database setting NULL | Reset to default value, log warning |

**Result**: WKMP can always start successfully with zero manual configuration.

## Testing Configuration

Test different configuration sources:

```bash
# Test 1: Zero configuration (delete all config files first)
rm ~/.config/wkmp/audio-player.toml
wkmp-ap

# Test 2: Environment variable override
export WKMP_ROOT=/tmp/wkmp-test
wkmp-ap

# Test 3: CLI argument override (highest priority)
wkmp-ap --root-folder /tmp/wkmp-cli-test

# Test 4: TOML config (create config file first)
mkdir -p ~/.config/wkmp
cp docs/examples/audio-player.toml ~/.config/wkmp/
# Edit root_folder in config file
wkmp-ap
```

## See Also

- [IMPL007-graceful_degradation_implementation.md](../IMPL007-graceful_degradation_implementation.md) - Complete implementation plan
- [IMPL004-deployment.md](../IMPL004-deployment.md) - Deployment specifications
- [REQ001-requirements.md](../REQ001-requirements.md) - Requirements [REQ-NF-030 through REQ-NF-036]
- [SPEC001-architecture.md](../SPEC001-architecture.md) - Architecture [ARCH-INIT-005 through ARCH-INIT-020]
