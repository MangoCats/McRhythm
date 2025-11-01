# wkmp-dr - Database Review Module

Read-only SQLite database inspection tool for WKMP Full version.

## Overview

wkmp-dr provides web-based access to the WKMP database for inspection and troubleshooting:
- Browse tables with pagination and sorting
- Predefined filters for common queries
- Search by Work ID or file path
- Zero write capabilities (read-only connection)

## Documentation

**Complete specification:** [docs/SPEC027-database_review.md](../docs/SPEC027-database_review.md)

**Extensibility guide:** [EXTENSIBILITY.md](EXTENSIBILITY.md)

## Quick Start

```bash
# Build
cargo build -p wkmp-dr

# Run (defaults to ~/Music/wkmp.db)
cargo run -p wkmp-dr

# Custom database location
cargo run -p wkmp-dr -- --root-folder /path/to/music

# Access UI
open http://localhost:5725
```

## Architecture

- **Language:** Rust
- **Framework:** Axum (HTTP server)
- **Database:** SQLx with SQLite (read-only mode)
- **UI:** Vanilla JavaScript + CSS
- **Port:** 5725

## Features

### Table Viewing
- Browse any WKMP table (songs, passages, files, etc.)
- Pagination (100 rows/page)
- Column sorting (ascending/descending)

### Filters
- Passages lacking MusicBrainz recording ID
- Files not yet segmented into passages

### Search
- Find songs by MusicBrainz Work ID
- Search files by path pattern

## Module Structure

```
wkmp-dr/
├── src/
│   ├── main.rs          # Entry point
│   ├── lib.rs           # Router configuration
│   ├── pagination.rs    # Centralized pagination logic
│   ├── api/             # API endpoints
│   │   ├── auth.rs      # Authentication middleware
│   │   ├── table.rs     # Table viewing
│   │   ├── filters.rs   # Predefined filters
│   │   ├── search.rs    # Search functions
│   │   ├── health.rs    # Health check
│   │   └── ui.rs        # Static UI serving
│   └── db/              # Database utilities
├── tests/
│   ├── api_tests.rs     # API integration tests
│   └── security_tests.rs # Security tests
├── EXTENSIBILITY.md     # Guide for adding filters
└── README.md           # This file
```

## Testing

```bash
# Run all tests (18 total)
cargo test -p wkmp-dr

# Run specific test suite
cargo test -p wkmp-dr --test api_tests
cargo test -p wkmp-dr --test security_tests
```

**Test Coverage:**
- 14 API integration tests
- 3 security tests (10MB body limit)
- 1 documentation test
- 100% pass rate

## Configuration

### Root Folder (4-tier priority)
1. CLI argument: `--root-folder /path` or `--root /path`
2. Environment: `WKMP_ROOT_FOLDER=/path` or `WKMP_ROOT=/path`
3. TOML config: `~/.config/wkmp/wkmp-dr.toml`
4. Default: `~/Music` (Linux/macOS), `%USERPROFILE%\Music` (Windows)

### Authentication
Inherits WKMP shared secret system, defaults to disabled (shared_secret == 0).

## Development

### Adding New Filters
See [EXTENSIBILITY.md](EXTENSIBILITY.md) for step-by-step guide.

### Code Quality
- Zero compiler warnings
- Centralized pagination logic (DRY principle)
- Input validation (whitelist for table names)
- Security: 10MB body limit, read-only DB connection

## Integration

- **Full version only** (not in Lite or Minimal)
- Launched via button in wkmp-ai home page
- Opens in new browser tab (port 5725)
- Shares WKMP database via read-only connection

## Version

**Current:** 1.3 (2025-11-01)

See [SPEC027 Revision History](../docs/SPEC027-database_review.md#revision-history) for changelog.

## License

Part of the WKMP project.
