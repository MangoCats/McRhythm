# WKMP Troubleshooting Guide

Comprehensive guide for diagnosing and resolving common WKMP issues, with special focus on graceful degradation scenarios.

---

## Table of Contents

1. [Configuration and Startup Issues](#configuration-and-startup-issues)
2. [Database Issues](#database-issues)
3. [Audio Playback Issues](#audio-playback-issues)
4. [Network and Port Issues](#network-and-port-issues)
5. [Performance Issues](#performance-issues)
6. [Migration from Old Installations](#migration-from-old-installations)
7. [Diagnostic Commands](#diagnostic-commands)

---

## Configuration and Startup Issues

### Issue: "Config file not found" Warning

**Symptom:**
```
WARN: Config file not found at ~/.config/wkmp/audio-player.toml, using default configuration
```

**Diagnosis:**
This is **NORMAL and expected** if you haven't created a config file. WKMP is designed to work without configuration files.

**Resolution:**
✅ **No action needed.** This is [REQ-NF-031] working as designed - missing config files are gracefully handled.

The warning indicates:
- WKMP looked for a config file
- Didn't find one (which is fine!)
- Using compiled defaults instead
- Continuing startup normally

**To suppress the warning** (if desired):
Create an empty config file:
```bash
# Linux
mkdir -p ~/.config/wkmp
touch ~/.config/wkmp/audio-player.toml

# macOS
mkdir -p ~/Library/Application\ Support/WKMP
touch ~/Library/Application\ Support/WKMP/audio-player.toml

# Windows PowerShell
New-Item -Path "$env:APPDATA\WKMP" -ItemType Directory -Force
New-Item -Path "$env:APPDATA\WKMP\audio-player.toml" -ItemType File
```

---

### Issue: "Failed to parse TOML" Error

**Symptom:**
```
WARN: Failed to load config file at ~/.config/wkmp/audio-player.toml: expected a right bracket, found an identifier at line 5 column 1
INFO: Root folder: /home/user/Music (compiled default)
```

**Diagnosis:**
Config file exists but has syntax errors. WKMP logs a warning and uses defaults.

**Resolution:**

**Option 1: Fix the TOML syntax**
```bash
# Validate TOML syntax online: https://www.toml-lint.com/
# Or use a TOML validator:
cat ~/.config/wkmp/audio-player.toml | toml-lint
```

Common TOML errors:
```toml
# WRONG: Missing quotes
root_folder = ~/Music

# CORRECT: Use quotes
root_folder = "~/Music"

# WRONG: Invalid section
[logging
level = "info"

# CORRECT: Close section with ]
[logging]
level = "info"
```

**Option 2: Delete and start fresh**
```bash
# Backup corrupted file
mv ~/.config/wkmp/audio-player.toml ~/.config/wkmp/audio-player.toml.bak

# Copy example config
cp docs/examples/audio-player.toml ~/.config/wkmp/

# Or just delete - WKMP will use defaults
rm ~/.config/wkmp/audio-player.toml
```

---

### Issue: Wrong Default Root Folder Location

**Symptom:**
```
INFO: Root folder: ~/.local/share/wkmp (compiled default)
```

But you expected `~/Music`.

**Diagnosis:**
You may be running an older version of WKMP that used a different default path.

**Resolution:**

**Option 1: Set environment variable**
```bash
export WKMP_ROOT=~/Music
wkmp-ap
```

**Option 2: Create config file**
```bash
mkdir -p ~/.config/wkmp
cat > ~/.config/wkmp/audio-player.toml << EOF
root_folder = "$HOME/Music"
EOF
```

**Option 3: Use command-line argument**
```bash
wkmp-ap --root-folder ~/Music
```

**Priority order** [REQ-NF-035]:
1. `--root-folder` argument (highest)
2. `WKMP_ROOT_FOLDER` env var
3. `WKMP_ROOT` env var
4. TOML config file
5. Compiled default (lowest)

---

### Issue: Permission Denied Creating Root Folder

**Symptom:**
```
ERROR: Failed to initialize root folder: Failed to create directory /var/lib/wkmp: Permission denied
```

**Diagnosis:**
WKMP trying to create directory in protected location.

**Resolution:**

**Option 1: Use user-writable location**
```bash
# Use home directory (recommended)
export WKMP_ROOT=~/Music
wkmp-ap
```

**Option 2: Fix permissions**
```bash
# If you really need system location
sudo mkdir -p /var/lib/wkmp
sudo chown $USER:$USER /var/lib/wkmp
chmod 755 /var/lib/wkmp

wkmp-ap
```

**Option 3: Run as different user**
```bash
# Create wkmp user
sudo useradd -r -s /bin/false wkmp
sudo mkdir -p /var/lib/wkmp
sudo chown wkmp:wkmp /var/lib/wkmp

# Run as wkmp user
sudo -u wkmp wkmp-ap
```

---

## Database Issues

### Issue: "Initialized new database" Every Time

**Symptom:**
```
INFO: Initialized new database: /home/user/Music/wkmp.db
INFO: Initialized setting 'volume_level' with default value: 0.5
... (all settings initialized every time)
```

**Diagnosis:**
Database file being deleted or recreated each run. Likely using `/tmp` root folder.

**Resolution:**

**Check current root folder:**
```bash
# Look for this line in logs:
grep "Root folder:" wkmp.log

# If it shows /tmp, the database is ephemeral
INFO: Root folder: /tmp/wkmp-XXXXX (compiled default)
```

**Fix: Use persistent location**
```bash
# Set permanent root folder
mkdir -p ~/Music
export WKMP_ROOT=~/Music
wkmp-ap

# Or create config file for persistence
cat > ~/.config/wkmp/audio-player.toml << EOF
root_folder = "$HOME/Music"
EOF
```

---

### Issue: Database Corruption

**Symptom:**
```
ERROR: Database integrity check failed: database disk image is malformed
```

**Diagnosis:**
Database file corrupted (power loss, disk failure, etc.)

**Resolution:**

**Automatic Recovery** (WKMP does this automatically):
```
INFO: Database corruption detected
INFO: Archiving corrupted database: wkmp-corrupted-20250118.db
INFO: Restoring from backup: wkmp-backup-20250115.db
INFO: Integrity check passed on restored database
INFO: Continuing startup
```

**Manual Recovery** (if automatic fails):

```bash
# 1. Check for backups (created every 14+ days)
ls -lht ~/Music/wkmp-backup-*.db | head -5

# Output:
# -rw-r--r-- 1 user user 250K Jan 15 14:23 wkmp-backup-20250115.db
# -rw-r--r-- 1 user user 248K Jan 01 10:15 wkmp-backup-20250101.db

# 2. Archive corrupted database
mv ~/Music/wkmp.db ~/Music/wkmp-corrupted-$(date +%Y%m%d).db

# 3. Restore from latest backup
cp ~/Music/wkmp-backup-20250115.db ~/Music/wkmp.db

# 4. Verify integrity
sqlite3 ~/Music/wkmp.db "PRAGMA integrity_check"

# Expected output:
# ok

# 5. Restart modules
wkmp-ap
```

**If no backups exist:**
```bash
# 1. Archive corrupted database
mv ~/Music/wkmp.db ~/Music/wkmp-corrupted.db

# 2. Start fresh (database will be recreated)
wkmp-ap

# Expected output:
# INFO: Initialized new database: /home/user/Music/wkmp.db

# Data loss: play history, user settings, custom passages
# Solution: Re-ingest music files (Full version)
```

---

### Issue: NULL Values in Settings

**Symptom:**
```
WARN: Setting 'volume_level' was NULL, reset to default: 0.5
WARN: Setting 'global_crossfade_time' was NULL, reset to default: 2.0
```

**Diagnosis:**
Settings were manually set to NULL or database was partially corrupted. WKMP automatically resets to defaults [ARCH-INIT-020].

**Resolution:**
✅ **No action needed.** This is automatic recovery working correctly.

**Verification:**
```bash
# Check settings table
sqlite3 ~/Music/wkmp.db << EOF
SELECT key, value FROM settings WHERE key = 'volume_level';
SELECT key, value FROM settings WHERE key = 'global_crossfade_time';
EOF

# Should show non-NULL values:
# volume_level|0.5
# global_crossfade_time|2.0
```

**Prevent future NULL values:**
```sql
-- Make value column NOT NULL (if you want strict enforcement)
sqlite3 ~/Music/wkmp.db << EOF
-- This is not done by default to allow explicit NULL as "use default"
-- But you can enforce it if desired:
-- ALTER TABLE settings ALTER COLUMN value SET NOT NULL;
EOF
```

---

### Issue: Missing Default Settings

**Symptom:**
```
INFO: Initialized setting 'volume_level' with default value: 0.5
... (only 10 settings initialized, expected 27+)
```

**Diagnosis:**
Running old version of WKMP that doesn't know about new settings.

**Resolution:**

**Option 1: Update WKMP** (recommended)
```bash
# Rebuild from source
cd wkmp
git pull
cargo build --release

# Or download latest binary
```

**Option 2: Manually add missing settings**
```bash
sqlite3 ~/Music/wkmp.db << EOF
-- Add missing settings (example)
INSERT OR IGNORE INTO settings (key, value) VALUES
  ('http_base_ports', '[5720, 15720, 25720, 17200, 23400]'),
  ('http_request_timeout_ms', '30000'),
  ('backup_retention_count', '3');
EOF
```

**Verification:**
```bash
# Count settings
sqlite3 ~/Music/wkmp.db "SELECT COUNT(*) FROM settings"

# Should be 27+ for current version
```

---

## Audio Playback Issues

### Issue: No Audio Output

**Symptom:**
```
INFO: Playback engine started
INFO: Audio stream started successfully
```
But no sound is heard.

**Diagnosis:**
Wrong audio device or muted system volume.

**Resolution:**

**Check audio device:**
```bash
# Linux - list audio devices
aplay -l

# macOS - list audio devices
system_profiler SPAudioDataType

# Windows PowerShell
Get-AudioDevice -List
```

**Check WKMP audio device setting:**
```bash
sqlite3 ~/Music/wkmp.db "SELECT value FROM settings WHERE key = 'audio_sink'"

# Default: "default"
# To use specific device:
sqlite3 ~/Music/wkmp.db "UPDATE settings SET value = 'hw:0,0' WHERE key = 'audio_sink'"
```

**Check system volume:**
```bash
# Linux
amixer get Master

# macOS
osascript -e 'output volume of (get volume settings)'

# Windows PowerShell
(New-Object -ComObject WScript.Shell).SendKeys([char]174)  # Mute toggle
```

**Check WKMP volume:**
```bash
# Check database
sqlite3 ~/Music/wkmp.db "SELECT value FROM settings WHERE key = 'volume_level'"

# Should be between 0.0 and 1.0
# If 0.0, increase via API:
curl -X POST http://localhost:5721/api/v1/playback/volume \
  -H "Content-Type: application/json" \
  -d '{"volume": 0.75}'
```

---

### Issue: Audio Stuttering or Dropouts

**Symptom:**
```
WARN: Audio ring buffer underrun (total: 1000)
WARN: Audio ring buffer underrun (total: 2000)
```

**Diagnosis:**
CPU can't keep up with audio decoding or system is under high load.

**Resolution:**

**Check CPU usage:**
```bash
# Linux
top -p $(pgrep wkmp-ap)

# Should be < 20% per core
# If > 50%, system is too slow or other processes consuming CPU
```

**Reduce audio quality** (if needed):
```bash
# Edit config to reduce buffer size
cat > ~/.config/wkmp/audio-player.toml << EOF
[audio]
buffer_size = 4096  # Increase from default 2048
EOF
```

**Close other applications:**
```bash
# Linux - find CPU hogs
ps aux --sort=-%cpu | head -10

# Close unnecessary apps
```

**Increase process priority** (Linux):
```bash
# Run with higher priority
nice -n -10 wkmp-ap

# Or renice running process
sudo renice -n -10 -p $(pgrep wkmp-ap)
```

---

## Network and Port Issues

### Issue: "Address already in use"

**Symptom:**
```
ERROR: Failed to bind to port 5721: Address already in use
```

**Diagnosis:**
Another process (or another WKMP instance) is using port 5721.

**Resolution:**

**Find process using port:**
```bash
# Linux/macOS
lsof -i :5721

# Output:
# COMMAND   PID USER   FD   TYPE DEVICE SIZE/OFF NODE NAME
# wkmp-ap  1234 user   10u  IPv4  12345      0t0  TCP *:5721 (LISTEN)

# Windows PowerShell
Get-NetTCPConnection -LocalPort 5721 | Select-Object -Property OwningProcess
Get-Process -Id <PID>
```

**Option 1: Kill conflicting process**
```bash
# Linux/macOS
kill <PID>

# Windows PowerShell
Stop-Process -Id <PID>
```

**Option 2: Use different port**

WKMP automatically tries fallback ports [DEP-HTTP-040]:
- 5721 (primary)
- 15721 (fallback 1)
- 25721 (fallback 2)
- 17201 (fallback 3)
- 23401 (fallback 4)

Check logs:
```
INFO: Port 5721 in use, trying 15721
INFO: Audio Player listening on 127.0.0.1:15721
```

**Option 3: Configure custom port**
```bash
# Via database
sqlite3 ~/Music/wkmp.db << EOF
UPDATE module_config SET port = 9999 WHERE module_name = 'audio_player';
EOF

wkmp-ap
```

---

### Issue: Can't Access Web UI from Another Computer

**Symptom:**
Web UI works on `http://localhost:5720` but not `http://192.168.1.100:5720`.

**Diagnosis:**
wkmp-ui bound to localhost only (127.0.0.1) instead of all interfaces (0.0.0.0).

**Resolution:**

**Check bind address:**
```bash
sqlite3 ~/Music/wkmp.db "SELECT host, port FROM module_config WHERE module_name = 'user_interface'"

# Output:
# 127.0.0.1|5720

# Change to bind all interfaces:
sqlite3 ~/Music/wkmp.db << EOF
UPDATE module_config SET host = '0.0.0.0' WHERE module_name = 'user_interface';
EOF

# Restart wkmp-ui
pkill wkmp-ui
wkmp-ui
```

**Check firewall:**
```bash
# Linux (ufw)
sudo ufw allow 5720/tcp

# Linux (iptables)
sudo iptables -A INPUT -p tcp --dport 5720 -j ACCEPT

# macOS
# System Preferences > Security & Privacy > Firewall > Firewall Options
# Add wkmp-ui to allowed apps

# Windows PowerShell (as Administrator)
New-NetFirewallRule -DisplayName "WKMP UI" -Direction Inbound -LocalPort 5720 -Protocol TCP -Action Allow
```

---

## Performance Issues

### Issue: Slow Startup (> 5 seconds)

**Symptom:**
wkmp-ap takes 10+ seconds to show "Audio Player listening" message.

**Diagnosis:**
Large database with many settings or slow disk I/O.

**Resolution:**

**Check database size:**
```bash
ls -lh ~/Music/wkmp.db

# If > 100MB, optimize:
sqlite3 ~/Music/wkmp.db "VACUUM"

# Reduces database size by removing deleted records
```

**Check disk I/O:**
```bash
# Linux
iostat -x 1

# High %util or await > 100ms indicates slow disk
```

**Use SSD if available:**
```bash
# Move to faster storage
mkdir -p /mnt/ssd/wkmp
mv ~/Music/wkmp.db /mnt/ssd/wkmp/

export WKMP_ROOT=/mnt/ssd/wkmp
wkmp-ap
```

---

### Issue: High Memory Usage

**Symptom:**
wkmp-ap using > 500 MB RAM.

**Diagnosis:**
Large audio buffers or memory leak.

**Resolution:**

**Check actual usage:**
```bash
# Linux
ps aux | grep wkmp-ap

# macOS
top -pid $(pgrep wkmp-ap)

# Normal usage: 50-150 MB
# High usage: > 500 MB
```

**Restart periodically** (if memory leak):
```bash
# Create systemd service with restart policy
# Or use cron:
0 4 * * * killall wkmp-ap && sleep 5 && wkmp-ap
```

**Report issue** with memory profile:
```bash
# Run with memory profiler
valgrind --tool=massif wkmp-ap
```

---

## Migration from Old Installations

### Issue: Database Path Changed

**Problem:**
Old location: `~/.local/share/wkmp/wkmp.db`
New default: `~/Music/wkmp.db`

**Resolution:**

**Option 1: Move database to new location**
```bash
# Create new location
mkdir -p ~/Music

# Move database
mv ~/.local/share/wkmp/wkmp.db ~/Music/

# Move audio files (if stored with database)
mv ~/.local/share/wkmp/*.mp3 ~/Music/

# Start with new default
wkmp-ap
```

**Option 2: Keep old location**
```bash
# Use environment variable
export WKMP_ROOT=~/.local/share/wkmp
wkmp-ap

# Or create config file
cat > ~/.config/wkmp/audio-player.toml << EOF
root_folder = "$HOME/.local/share/wkmp"
EOF
```

---

### Issue: Config Format Changed

**Problem:**
Old config had different structure.

**Resolution:**
```bash
# Backup old config
mv ~/.config/wkmp/audio-player.toml ~/.config/wkmp/audio-player.toml.old

# Copy new example
cp docs/examples/audio-player.toml ~/.config/wkmp/

# Manually merge any customizations
```

---

## Diagnostic Commands

### Complete System Check

```bash
#!/bin/bash
# WKMP diagnostic script

echo "=== WKMP Diagnostics ==="
echo

echo "1. Environment Variables:"
env | grep WKMP
echo

echo "2. Config Files:"
ls -l ~/.config/wkmp/*.toml 2>/dev/null || echo "No config files found (OK)"
echo

echo "3. Root Folder:"
export WKMP_ROOT=${WKMP_ROOT:-~/Music}
ls -ld "$WKMP_ROOT" 2>/dev/null || echo "Root folder doesn't exist (will be created)"
echo

echo "4. Database:"
ls -lh "$WKMP_ROOT/wkmp.db" 2>/dev/null || echo "Database doesn't exist (will be created)"
echo

echo "5. Database Integrity:"
if [ -f "$WKMP_ROOT/wkmp.db" ]; then
  sqlite3 "$WKMP_ROOT/wkmp.db" "PRAGMA integrity_check" || echo "FAILED"
else
  echo "N/A (database doesn't exist)"
fi
echo

echo "6. Settings Count:"
if [ -f "$WKMP_ROOT/wkmp.db" ]; then
  sqlite3 "$WKMP_ROOT/wkmp.db" "SELECT COUNT(*) FROM settings" || echo "FAILED"
else
  echo "N/A"
fi
echo

echo "7. Module Configs:"
if [ -f "$WKMP_ROOT/wkmp.db" ]; then
  sqlite3 "$WKMP_ROOT/wkmp.db" "SELECT module_name, host, port FROM module_config" || echo "FAILED"
else
  echo "N/A"
fi
echo

echo "8. Running Processes:"
ps aux | grep wkmp | grep -v grep
echo

echo "9. Port Usage:"
lsof -i :5720,5721,5722,5723,5724 2>/dev/null || echo "No WKMP ports in use"
echo

echo "10. Audio Devices:"
aplay -l 2>/dev/null || echo "aplay not available"
echo

echo "=== End Diagnostics ==="
```

### Database Query Examples

```bash
# Check all settings
sqlite3 ~/Music/wkmp.db << EOF
.headers on
.mode column
SELECT key, value FROM settings ORDER BY key;
EOF

# Check module configurations
sqlite3 ~/Music/wkmp.db "SELECT * FROM module_config"

# Check users
sqlite3 ~/Music/wkmp.db "SELECT username, config_interface_access FROM users"

# Check queue
sqlite3 ~/Music/wkmp.db "SELECT file_path, play_order FROM queue ORDER BY play_order"

# Check for NULL values
sqlite3 ~/Music/wkmp.db "SELECT key FROM settings WHERE value IS NULL"
```

---

## Still Having Problems?

### Collect Debug Information

```bash
# Run with verbose logging
WKMP_LOG_LEVEL=debug wkmp-ap 2>&1 | tee wkmp-debug.log

# Generate diagnostic report
bash docs/diagnostic-script.sh > wkmp-diagnostics.txt

# Check database schema
sqlite3 ~/Music/wkmp.db ".schema" > wkmp-schema.sql
```

### Report an Issue

Include the following when reporting:

1. **Version:** `wkmp-ap --version`
2. **Platform:** `uname -a` (Linux/macOS) or `systeminfo` (Windows)
3. **Logs:** `wkmp-debug.log` (last 100 lines)
4. **Diagnostics:** `wkmp-diagnostics.txt`
5. **Steps to reproduce**
6. **Expected vs actual behavior**

### Community Support

- **GitHub Issues:** https://github.com/yourusername/wkmp/issues
- **Discussions:** https://github.com/yourusername/wkmp/discussions
- **Documentation:** `docs/` directory

---

**Remember:** WKMP is designed for graceful degradation. Most "errors" are actually warnings, and the system should continue to function with sensible defaults.
