# Migration Guide: Graceful Degradation Update

**For existing WKMP installations upgrading to version with graceful degradation support**

**Date:** 2025-10-18
**Affects:** All installations with existing configuration files or databases
**Breaking Changes:** Default root folder location changed

---

## What Changed

### New Features

**[REQ-NF-031 through REQ-NF-036]:** WKMP now supports zero-configuration startup with graceful degradation:

✅ **Configuration files are now optional**
- Missing TOML files do NOT cause errors
- Automatic fallback to compiled defaults
- Warning logged (not error) for missing configs

✅ **Automatic resource initialization**
- Root folder created automatically if missing
- Database created with default schema
- All settings initialized with sensible defaults

✅ **4-tier configuration priority**
1. Command-line arguments (highest)
2. Environment variables
3. TOML configuration file
4. Compiled defaults (lowest)

### Breaking Changes

#### ⚠️ Default Root Folder Location Changed

**Old Default:**
- Linux: `~/.local/share/wkmp`
- macOS: `~/Library/Application Support/wkmp`
- Windows: `%LOCALAPPDATA%\wkmp`

**New Default:**
- Linux: `~/Music`
- macOS: `~/Music`
- Windows: `%USERPROFILE%\Music\wkmp`

**Reason:** User-facing Music folder is more intuitive than hidden application data folder.

---

## Who Needs to Migrate?

### You DON'T need to migrate if:

✅ You're a new user installing WKMP for the first time
✅ You already use a custom root folder via config file or environment variable
✅ You don't have any existing data (empty/new installation)

### You DO need to migrate if:

⚠️ You have an existing WKMP installation with:
- Existing database at old default location
- Existing music files at old default location
- No explicit root folder configuration

---

## Migration Options

Choose the option that best fits your needs.

### Option 1: Continue Using Old Location (Recommended)

**Simplest option - no file moves needed**

Create a config file pointing to your existing installation:

```bash
# Linux
mkdir -p ~/.config/wkmp

cat > ~/.config/wkmp/audio-player.toml << 'EOF'
root_folder = "~/.local/share/wkmp"
EOF

# Also create for other modules
cat > ~/.config/wkmp/ui.toml << 'EOF'
root_folder = "~/.local/share/wkmp"
EOF

cat > ~/.config/wkmp/program-director.toml << 'EOF'
root_folder = "~/.local/share/wkmp"
EOF

# macOS (adjust paths)
mkdir -p ~/Library/Application\ Support/WKMP

cat > ~/Library/Application\ Support/WKMP/audio-player.toml << 'EOF'
root_folder = "~/Library/Application Support/wkmp"
EOF

# Windows PowerShell
$configDir = "$env:APPDATA\WKMP"
New-Item -Path $configDir -ItemType Directory -Force

@"
root_folder = "$env:USERPROFILE\AppData\Local\wkmp"
"@ | Out-File -FilePath "$configDir\audio-player.toml"
```

**Verification:**
```bash
# Start module - should use old location
wkmp-ap

# Check logs for:
# INFO: Root folder: /home/user/.local/share/wkmp (from config file)
```

**Pros:**
- ✅ No file moves
- ✅ Zero risk of data loss
- ✅ Works immediately

**Cons:**
- ❌ Requires config file (less "zero-config")
- ❌ Doesn't follow new convention

---

### Option 2: Move to New Location

**Best for long-term - follows new convention**

Move your existing database and music files:

```bash
# === Linux ===

# 1. Stop all WKMP modules
pkill wkmp-ap
pkill wkmp-ui
pkill wkmp-pd

# 2. Create new location
mkdir -p ~/Music

# 3. Move database
mv ~/.local/share/wkmp/wkmp.db ~/Music/

# 4. Move music files (if stored with database)
# Check what's in old location:
ls -la ~/.local/share/wkmp/

# Move audio files:
find ~/.local/share/wkmp -type f \( -name "*.mp3" -o -name "*.flac" -o -name "*.m4a" \) -exec mv {} ~/Music/ \;

# Or move entire directory structure:
# rsync -av ~/.local/share/wkmp/ ~/Music/

# 5. Move backups
mv ~/.local/share/wkmp/wkmp-backup-*.db ~/Music/ 2>/dev/null || true

# 6. Verify
ls -lh ~/Music/wkmp.db
ls ~/Music/*.mp3 | head -5

# 7. Start modules - should use new location
wkmp-ap

# Check logs for:
# INFO: Root folder: /home/user/Music (compiled default)
# INFO: Opened existing database: /home/user/Music/wkmp.db

# === macOS ===

# 1-2. Stop modules and create location
pkill wkmp-ap wkmp-ui wkmp-pd
mkdir -p ~/Music

# 3-6. Move files
mv ~/Library/Application\ Support/wkmp/wkmp.db ~/Music/
# (adjust for music files as needed)

# 7. Start and verify
wkmp-ap

# === Windows PowerShell ===

# 1. Stop modules
Stop-Process -Name "wkmp-ap", "wkmp-ui", "wkmp-pd" -Force -ErrorAction SilentlyContinue

# 2. Create new location
$newLocation = "$env:USERPROFILE\Music\wkmp"
New-Item -Path $newLocation -ItemType Directory -Force

# 3. Move database
$oldLocation = "$env:LOCALAPPDATA\wkmp"
Move-Item -Path "$oldLocation\wkmp.db" -Destination "$newLocation\wkmp.db"

# 4. Move music files (adjust as needed)
Get-ChildItem -Path $oldLocation -Filter *.mp3 | Move-Item -Destination $newLocation

# 5. Start and verify
wkmp-ap.exe
```

**Verification:**
```bash
# Check database opened at new location:
grep "database" ~/.local/share/wkmp/logs/wkmp-ap.log

# Expected:
# INFO: Opened existing database: /home/user/Music/wkmp.db

# Check settings intact:
sqlite3 ~/Music/wkmp.db "SELECT COUNT(*) FROM settings"
# Should show 27+ settings

# Check queue preserved:
sqlite3 ~/Music/wkmp.db "SELECT COUNT(*) FROM queue"
# Should match old queue size
```

**Pros:**
- ✅ Follows new convention
- ✅ True zero-config (no TOML needed)
- ✅ User-facing location (easier to find files)

**Cons:**
- ⚠️ Requires file moves (small risk)
- ⚠️ Need to update external tools pointing to old location

**Risk Mitigation:**
```bash
# Backup before moving
tar -czf wkmp-backup-$(date +%Y%m%d).tar.gz ~/.local/share/wkmp

# Keep old directory for 30 days
# Delete only after verifying new location works
```

---

### Option 3: Use Environment Variable

**Good for testing or temporary setups**

```bash
# Set environment variable (temporary)
export WKMP_ROOT=~/.local/share/wkmp
wkmp-ap

# Or add to shell profile (permanent)
echo 'export WKMP_ROOT=~/.local/share/wkmp' >> ~/.bashrc
source ~/.bashrc

# macOS (use ~/.zshrc instead of ~/.bashrc)
echo 'export WKMP_ROOT=~/.local/share/wkmp' >> ~/.zshrc

# Windows (System Properties > Environment Variables)
# Add user variable: WKMP_ROOT = C:\Users\YourName\AppData\Local\wkmp
```

**Pros:**
- ✅ No file moves
- ✅ No config files
- ✅ Easy to change

**Cons:**
- ❌ Must set for every terminal/session
- ❌ Doesn't persist across reboots (unless added to profile)

---

## File Path Updates

If your application references absolute paths to old locations, update them:

### Database Connections

**Old code:**
```rust
let db_path = "~/.local/share/wkmp/wkmp.db";
```

**New code:**
```rust
// Use resolver to get correct path
use wkmp_common::config::RootFolderResolver;
let resolver = RootFolderResolver::new("audio-player");
let root = resolver.resolve();
let db_path = root.join("wkmp.db");
```

### Audio File Paths

**Old absolute paths in database:**
```sql
-- Check for absolute paths
SELECT file_path FROM files WHERE file_path LIKE '/home/%/.local/share/wkmp/%';
```

**Update to relative paths:**
```sql
-- Convert absolute to relative (if all files under root)
UPDATE files
SET file_path = REPLACE(file_path, '/home/user/.local/share/wkmp/', '')
WHERE file_path LIKE '/home/user/.local/share/wkmp/%';
```

---

## Testing Migration

### Before Migration

```bash
# 1. Test current installation works
wkmp-ap &
sleep 2
curl http://localhost:5721/health
pkill wkmp-ap

# 2. Backup database
cp ~/.local/share/wkmp/wkmp.db ~/wkmp-backup-pre-migration.db

# 3. Document current state
sqlite3 ~/.local/share/wkmp/wkmp.db << EOF
SELECT COUNT(*) as setting_count FROM settings;
SELECT COUNT(*) as queue_count FROM queue;
SELECT COUNT(*) as file_count FROM files;
EOF

# Save output for comparison
```

### After Migration

```bash
# 1. Test new location works
wkmp-ap &
sleep 2
curl http://localhost:5721/health
pkill wkmp-ap

# 2. Verify data migrated
sqlite3 ~/Music/wkmp.db << EOF
SELECT COUNT(*) as setting_count FROM settings;
SELECT COUNT(*) as queue_count FROM queue;
SELECT COUNT(*) as file_count FROM files;
EOF

# Compare with pre-migration counts

# 3. Test playback
curl -X POST http://localhost:5721/api/v1/playback/play
# Should start playing from queue

# 4. Test web UI
curl http://localhost:5720/
# Should return HTML
```

---

## Rollback Plan

If migration causes issues:

### Rollback from Option 2 (File Move)

```bash
# 1. Stop modules
pkill wkmp-ap wkmp-ui wkmp-pd

# 2. Restore from backup
tar -xzf wkmp-backup-$(date +%Y%m%d).tar.gz -C ~/

# 3. Use environment variable to force old location
export WKMP_ROOT=~/.local/share/wkmp

# 4. Start modules
wkmp-ap
```

### Rollback from Option 1 (Config File)

```bash
# Just delete config files
rm ~/.config/wkmp/*.toml

# System will use new default
# (but won't find old database - use env var instead)
export WKMP_ROOT=~/.local/share/wkmp
wkmp-ap
```

---

## Automated Migration Script

```bash
#!/bin/bash
# migrate-wkmp-root.sh - Automated migration to new default location

set -e

OLD_ROOT="$HOME/.local/share/wkmp"
NEW_ROOT="$HOME/Music"

echo "=== WKMP Migration Script ==="
echo "Old location: $OLD_ROOT"
echo "New location: $NEW_ROOT"
echo

# Check old location exists
if [ ! -d "$OLD_ROOT" ]; then
  echo "ERROR: Old location doesn't exist. No migration needed."
  exit 0
fi

# Check database exists
if [ ! -f "$OLD_ROOT/wkmp.db" ]; then
  echo "ERROR: No database found at old location. No migration needed."
  exit 0
fi

# Backup
echo "Creating backup..."
BACKUP_FILE="$HOME/wkmp-backup-$(date +%Y%m%d-%H%M%S).tar.gz"
tar -czf "$BACKUP_FILE" "$OLD_ROOT"
echo "Backup created: $BACKUP_FILE"
echo

# Stop modules
echo "Stopping WKMP modules..."
pkill wkmp-ap wkmp-ui wkmp-pd || true
sleep 2
echo

# Create new location
echo "Creating new location..."
mkdir -p "$NEW_ROOT"
echo

# Move database
echo "Moving database..."
mv "$OLD_ROOT/wkmp.db" "$NEW_ROOT/wkmp.db"
echo "✓ Database moved"

# Move backups
echo "Moving backups..."
mv "$OLD_ROOT"/wkmp-backup-*.db "$NEW_ROOT/" 2>/dev/null || true
echo "✓ Backups moved"

# Move music files (adjust extensions as needed)
echo "Moving music files..."
MUSIC_COUNT=$(find "$OLD_ROOT" -type f \( -name "*.mp3" -o -name "*.flac" -o -name "*.m4a" -o -name "*.ogg" -o -name "*.opus" \) | wc -l)
if [ "$MUSIC_COUNT" -gt 0 ]; then
  find "$OLD_ROOT" -type f \( -name "*.mp3" -o -name "*.flac" -o -name "*.m4a" -o -name "*.ogg" -o -name "*.opus" \) -exec mv {} "$NEW_ROOT/" \;
  echo "✓ Moved $MUSIC_COUNT music files"
else
  echo "  No music files found in old location"
fi
echo

# Verify
echo "Verifying migration..."
if [ -f "$NEW_ROOT/wkmp.db" ]; then
  SETTINGS_COUNT=$(sqlite3 "$NEW_ROOT/wkmp.db" "SELECT COUNT(*) FROM settings" 2>/dev/null || echo "0")
  echo "✓ Database accessible: $SETTINGS_COUNT settings"
else
  echo "ERROR: Database not found at new location!"
  exit 1
fi
echo

# Clean up (keep old directory for now)
echo "Keeping old directory for safety: $OLD_ROOT"
echo "You can delete it manually after verifying everything works."
echo

echo "=== Migration Complete ==="
echo
echo "Next steps:"
echo "1. Start WKMP: wkmp-ap"
echo "2. Verify logs show: 'Root folder: $NEW_ROOT (compiled default)'"
echo "3. Test playback works"
echo "4. After 30 days, delete old directory: rm -rf $OLD_ROOT"
echo
echo "To rollback: tar -xzf $BACKUP_FILE -C ~/"
```

**Usage:**
```bash
chmod +x migrate-wkmp-root.sh
./migrate-wkmp-root.sh
```

---

## FAQ

### Q: Will my playlists/queue be preserved?

**A:** Yes, if you migrate the database file, all queue entries are preserved. The queue is stored in the database.

### Q: Will my playback history be lost?

**A:** No, playback history is stored in the database. Migration preserves all data.

### Q: Can I use both old and new locations?

**A:** No, WKMP uses a single root folder. You must choose one location.

### Q: What if I have music files in multiple locations?

**A:** Use symlinks:
```bash
ln -s /mnt/music1 ~/Music/collection1
ln -s /mnt/music2 ~/Music/collection2
```

Or use the old location (which may already have symlinks).

### Q: Do I need to update all modules?

**A:** If using config files, yes - create config for each module. If using environment variable, set once in shell profile.

### Q: Can I test migration without affecting production?

**A:** Yes, use a copy:
```bash
cp -r ~/.local/share/wkmp /tmp/wkmp-test
export WKMP_ROOT=/tmp/wkmp-test
wkmp-ap
# Test, then delete /tmp/wkmp-test when done
```

---

## Support

If you encounter issues during migration:

1. **Stop modules immediately**
2. **Restore from backup** (created before migration)
3. **Report issue** on GitHub with:
   - Migration option chosen
   - Error messages
   - Platform (Linux/macOS/Windows)
   - Database size: `ls -lh wkmp.db`

**GitHub Issues:** https://github.com/yourusername/wkmp/issues

---

## Summary

**Recommended migration path:**

1. **Backup everything** (database + music files)
2. **Choose Option 1** (config file) for zero risk
3. **Test thoroughly** for 1-2 weeks
4. **Optionally switch to Option 2** later (file move) if desired

**Remember:** Both old and new locations work perfectly. The choice is about:
- **Convenience** (new location more discoverable)
- **Convention** (new location matches user expectations)
- **Risk** (old location requires no changes)

Choose what works best for your setup!
