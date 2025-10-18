# WKMP User Documentation

Welcome to the WKMP (Auto DJ Music Player) user documentation!

This directory contains all user-facing documentation for installing, configuring, operating, and troubleshooting WKMP.

---

## Quick Links

### Getting Started

- **[QUICKSTART.md](./QUICKSTART.md)** - Get WKMP running in under 5 minutes with zero configuration
  - Installation instructions (pre-built binaries and from source)
  - First run walkthrough
  - Basic usage and operations
  - Version differences (Full/Lite/Minimal)

### Problem Solving

- **[TROUBLESHOOTING.md](./TROUBLESHOOTING.md)** - Comprehensive guide for diagnosing and resolving issues
  - Configuration and startup issues
  - Database issues (corruption, NULL values, missing settings)
  - Audio playback problems
  - Network and port conflicts
  - Performance optimization
  - Migration guides
  - Complete diagnostic scripts

---

## What is WKMP?

WKMP is an automatic DJ music player with sample-accurate crossfading. It:

- **Automatically selects music** based on your preferences and time of day
- **Crossfades seamlessly** between songs with multiple fade curve types
- **Requires zero configuration** to get started (but is highly customizable)
- **Works on Linux, macOS, and Windows**
- **Provides a web interface** for control and visualization
- **Uses your existing music collection** with MusicBrainz/AcousticBrainz integration

---

## Documentation for Different Audiences

### End Users

**Start here:** [QUICKSTART.md](./QUICKSTART.md)

You just want to:
- ✅ Install WKMP and start listening to music
- ✅ Use the web interface to control playback
- ✅ Adjust volume and crossfade settings
- ✅ Add your music collection

### System Administrators

**Start here:** [QUICKSTART.md](./QUICKSTART.md) → Configuration section

You need to:
- ✅ Deploy WKMP on a server
- ✅ Configure root folders and storage locations
- ✅ Set up systemd/launchd services
- ✅ Troubleshoot deployment issues
- ✅ Manage backups and recovery

→ Also see: [TROUBLESHOOTING.md](./TROUBLESHOOTING.md) for diagnostic procedures

### Developers/Contributors

**See:** [../README.md](../README.md) for technical documentation

You want to:
- Understand WKMP's architecture
- Contribute code or features
- Review requirements and design specifications
- Follow implementation roadmaps

→ Start with [GOV001-document_hierarchy.md](../GOV001-document_hierarchy.md) to understand the technical documentation structure

---

## WKMP Versions

WKMP comes in three versions with different feature sets:

| Version | Binaries | Use Case |
|---------|----------|----------|
| **Full** | wkmp-ap, wkmp-ui, wkmp-pd, wkmp-ai, wkmp-le | Complete music server with file scanning and metadata extraction |
| **Lite** | wkmp-ap, wkmp-ui, wkmp-pd | Playback from pre-populated database (no file ingest) |
| **Minimal** | wkmp-ap, wkmp-ui | Manual queue management only (no automatic selection) |

All versions support:
- Sample-accurate crossfading
- Web user interface
- Configuration via database, TOML files, or environment variables

See [QUICKSTART.md](./QUICKSTART.md) for detailed version comparisons.

---

## Key Features

### Zero Configuration Startup

WKMP is designed to "just work" out of the box:

```bash
wkmp-ap  # That's it! No config files needed.
```

- Automatically creates necessary directories
- Initializes database with sensible defaults
- Starts HTTP server and playback engine
- Logs clear messages about what's happening

### Graceful Degradation

WKMP handles error conditions gracefully:

- **Missing config files?** Uses compiled defaults and logs a warning (not an error)
- **Database corruption?** Automatically restores from backup
- **NULL settings?** Resets to defaults automatically
- **Port in use?** Tries fallback ports automatically

You should rarely encounter fatal errors.

### Flexible Configuration

Configure WKMP your way:

1. **No configuration** - Use defaults (recommended for first-time users)
2. **Environment variables** - Quick overrides without files
3. **TOML config files** - Persistent custom settings
4. **Command-line arguments** - One-time overrides
5. **Database settings** - Runtime configuration via web UI or API

Priority order: CLI args > env vars > config file > database > compiled defaults

See [QUICKSTART.md](./QUICKSTART.md) for configuration examples.

---

## Common Tasks

### First-Time Setup

1. Install WKMP (see [QUICKSTART.md](./QUICKSTART.md))
2. Run `wkmp-ap` (creates database and starts server)
3. Run `wkmp-ui` (starts web interface)
4. Open browser to http://localhost:5720
5. Add music via web UI or API

### Adding Music

**Option 1: Manual (all versions)**
- Copy files to root folder
- Enqueue via API or web UI

**Option 2: Automatic (Full version only)**
- Run `wkmp-ai` (Audio Ingest module)
- Scan directories via web UI
- WKMP extracts metadata and creates passages automatically

### Troubleshooting

**Problem:** Something isn't working

**Solution:**
1. Check logs for error messages
2. Consult [TROUBLESHOOTING.md](./TROUBLESHOOTING.md) for your specific issue
3. Run diagnostic script (included in troubleshooting guide)
4. Report issue with logs if not resolved

Most "errors" are actually warnings - WKMP will continue with defaults.

---

## Getting Help

### Documentation

- **This directory (docs/user/)** - User guides and troubleshooting
- **docs/ (root)** - Technical documentation for developers
- **docs/examples/** - Configuration examples

### Community

- **GitHub Issues:** https://github.com/yourusername/wkmp/issues
- **GitHub Discussions:** https://github.com/yourusername/wkmp/discussions

### Reporting Issues

When reporting a problem, please include:

1. WKMP version (`wkmp-ap --version`)
2. Platform (Linux/macOS/Windows + version)
3. Logs (last 50-100 lines with debug logging)
4. Steps to reproduce
5. Expected vs actual behavior

See [TROUBLESHOOTING.md](./TROUBLESHOOTING.md) for instructions on collecting diagnostic information.

---

## Document Maintenance

**These user documents are maintained by:** Documentation Lead

**Update frequency:** Updated when requirements or specifications change

**Feedback:** User feedback is welcome! If something is unclear or missing, please open a GitHub issue.

**Principle:** User documentation is derived FROM technical requirements and specifications, ensuring accuracy and consistency. Changes to technical docs (especially [REQ001-requirements.md](../REQ001-requirements.md) and [SPEC###-*.md](../SPEC001-architecture.md)) may trigger updates to user docs.

---

## Next Steps

**New users:** Read [QUICKSTART.md](./QUICKSTART.md) to get WKMP running in under 5 minutes.

**Existing users:** Bookmark [TROUBLESHOOTING.md](./TROUBLESHOOTING.md) for quick problem resolution.

**Contributors:** See [../README.md](../README.md) for technical documentation and contribution guidelines.

---

**Welcome to WKMP - Enjoy your music with sample-accurate crossfading!**
