# SPEC027 - Database Review Module Design

> **Related Documentation:** [Requirements](REQ001-requirements.md) | [Architecture](SPEC001-architecture.md) | [API Design](SPEC007-api_design.md) | [Database Schema](IMPL001-database_schema.md) | [Requirements Enumeration](GOV002-requirements_enumeration.md)

---

**TIER 2 - DESIGN**

This document specifies the design of the Database Review module (wkmp-dr), a read-only database inspection tool for WKMP Full version.

---

## Overview

**[DR-OV-010] Purpose**

wkmp-dr provides read-only access to the WKMP SQLite database via:
- Direct table browsing with pagination
- Predefined query filters for common inspection tasks
- Search functionality for Work IDs and file paths
- User preference persistence via localStorage

**[DR-OV-020] Scope**

- Full version only (not included in Lite or Minimal)
- Standalone HTTP server on port 5725
- Zero database modification capabilities
- Accessed via web browser (not embedded in wkmp-ui)

**[DR-OV-030] Integration**

- Launched via button in wkmp-ai home page
- Opens in new browser tab
- Shares WKMP database via read-only connection

---

## Architecture

**[DR-ARCH-010] Technology Stack**

- Language: Rust
- Web framework: Axum
- Database: SQLx with SQLite (read-only mode)
- UI: Vanilla JavaScript + CSS (no frameworks)
- Static assets: Embedded via `include_str!` macro

**[DR-ARCH-020] Database Connection**

Read-only SQLite connection with immutable flag:
```
sqlite://path/to/wkmp.db?mode=ro&immutable=1
```

Prevents accidental writes and enables concurrent access without blocking WKMP modules.

**[DR-ARCH-030] Module Structure**

```
wkmp-dr/
├── src/
│   ├── main.rs          # Entry point, router setup
│   ├── lib.rs           # Router configuration
│   ├── pagination.rs    # Centralized pagination logic (v1.3)
│   ├── api/
│   │   ├── mod.rs       # API module exports
│   │   ├── auth.rs      # Authentication middleware
│   │   ├── table.rs     # Table browsing endpoints
│   │   ├── filters.rs   # Predefined query filters
│   │   ├── search.rs    # Search endpoints
│   │   ├── health.rs    # Health check endpoint
│   │   └── ui.rs        # Static UI serving
│   ├── db/
│   │   ├── mod.rs       # Database module exports
│   │   ├── connect.rs   # Read-only connection setup
│   │   └── tables.rs    # Table metadata queries
│   └── ui/
│       ├── index.html   # Main UI page
│       └── app.js       # Client-side application
├── tests/
│   ├── api_tests.rs     # API integration tests (v1.3)
│   └── security_tests.rs # Security tests for body limits (v1.3)
├── EXTENSIBILITY.md     # Guide for adding new filters
└── Cargo.toml
```

**[DR-ARCH-040] Zero-Configuration Startup**

Implements standard WKMP zero-config pattern:
1. Resolve root folder (4-tier priority: CLI → env → TOML → default)
2. Create directory if missing
3. Connect to database in read-only mode

---

## API Endpoints

### Health Check

**[DR-API-010] GET /health**

Health check endpoint for monitoring.

Response:
```json
{
  "status": "ok",
  "module": "wkmp-dr",
  "version": "0.1.0"
}
```

### Table Browsing

**[DR-API-020] GET /api/table/:name**

Browse table contents with pagination and sorting.

Query parameters:
- `page` (optional, default 1): Page number
- `sort` (optional, default primary key): Column to sort by
- `order` (optional, default "asc"): Sort order ("asc" or "desc")

Response:
```json
{
  "table_name": "songs",
  "columns": ["guid", "recording_mbid", "work_id", "created_at"],
  "rows": [
    ["uuid-here", "mbid-here", null, "2025-11-01T12:00:00Z"]
  ],
  "total_rows": 150,
  "page": 1,
  "total_pages": 2
}
```

Page size: 100 rows per page

Supported tables:
- songs
- passages
- files
- artists
- albums
- works

### Predefined Filters

**[DR-API-030] GET /api/filters/passages-without-mbid**

Returns passages lacking MusicBrainz Recording IDs.

Query parameters:
- `page` (optional, default 1): Page number

SQL query:
```sql
SELECT guid, file_id, start_time_ticks, end_time_ticks, title, created_at
FROM passages
WHERE guid NOT IN (
  SELECT DISTINCT passage_id FROM passage_songs
)
ORDER BY created_at DESC
LIMIT 100 OFFSET ?
```

Response format: FilterResponse (see DR-API-050)

**[DR-API-040] GET /api/filters/files-without-passages**

Returns files that have no associated passages.

Query parameters:
- `page` (optional, default 1): Page number

SQL query:
```sql
SELECT guid, path, duration, hash, created_at
FROM files
WHERE guid NOT IN (
  SELECT DISTINCT file_id FROM passages
)
ORDER BY created_at DESC
LIMIT 100 OFFSET ?
```

**[DR-API-050] FilterResponse Format**

Common response format for all filters:
```json
{
  "filter_name": "passages-without-mbid",
  "description": "Passages without MusicBrainz Recording IDs",
  "columns": ["guid", "file_id", "start_time_ticks", "end_time_ticks", "title", "created_at"],
  "rows": [
    ["uuid-1", "file-uuid", 0, 8820000, "Example Title", "2025-11-01T12:00:00Z"]
  ],
  "total_results": 42,
  "page": 1,
  "total_pages": 1
}
```

### Search Functions

**[DR-API-060] GET /api/search/by-work-id**

Search passages by MusicBrainz Work ID.

Query parameters:
- `work_id` (required): MusicBrainz Work UUID
- `page` (optional, default 1): Page number

Validation:
- work_id must be valid UUID format
- Returns 400 Bad Request if invalid

SQL query:
```sql
SELECT p.guid, p.file_id, p.start_time_ticks, p.end_time_ticks,
       s.recording_mbid, s.work_id
FROM passages p
JOIN passage_songs ps ON p.guid = ps.passage_id
JOIN songs s ON ps.song_id = s.guid
WHERE s.work_id = ?
ORDER BY p.created_at DESC
LIMIT 100 OFFSET ?
```

**[DR-API-070] GET /api/search/by-path**

Search files by path pattern using SQL LIKE.

Query parameters:
- `pattern` (required): SQL LIKE pattern (e.g., `%.mp3`, `/music/%`)
- `page` (optional, default 1): Page number

SQL query:
```sql
SELECT guid, path, duration, hash, created_at
FROM files
WHERE path LIKE ?
ORDER BY path ASC
LIMIT 100 OFFSET ?
```

---

## User Interface

**[DR-UI-010] Page Structure**

Single-page application with sections:
- Header (module name, subtitle)
- Saved Searches (favorites bar)
- Controls (view type, parameters, actions)
- Results (table + pagination)
- Status messages

**[DR-UI-020] View Types**

Dropdown selector with options:
- Browse Table (direct table access)
- Filter: Passages without MBID
- Filter: Files without passages
- Search: By Work ID
- Search: By File Path

**[DR-UI-030] Dynamic Controls**

Controls shown/hidden based on view type:
- Table browsing: table selector
- Work ID search: UUID input field
- Path search: pattern input field

**[DR-UI-040] Pagination**

- Previous/Next buttons
- Page info: "Page X of Y (N total results)"
- Disabled state when at first/last page

**[DR-UI-050] Table Rendering**

Responsive table with:
- Header row (column names)
- Data rows (JSON values)
- NULL values displayed as italic gray "null"
- Mobile-responsive design (tablet @ 768px, mobile @ 480px)

**[DR-UI-060] Status Messages**

Toast-style messages:
- Success (green): "Saved search 'name'"
- Error (red): HTTP errors, validation failures
- Auto-hide after 3 seconds

---

## User Preferences

**[DR-PREF-010] localStorage Persistence**

User preferences stored in browser localStorage:
```json
{
  "savedSearches": [
    {
      "id": "1730468923456",
      "name": "User-defined name",
      "view": {
        "viewType": "search-work",
        "workId": "uuid-here"
      },
      "savedAt": "2025-11-01T12:00:00Z"
    }
  ],
  "lastView": { /* view config */ },
  "version": 1
}
```

**[DR-PREF-020] Saved Searches**

Features:
- Save current search with user-provided name
- Quick-access buttons in favorites bar
- Duplicate name handling (overwrite)
- Export to JSON file
- Import from JSON file
- Clear all saved searches

**[DR-PREF-030] Last View Restoration**

On page load:
- Restore last viewed configuration
- Pre-fill form controls
- Do NOT auto-execute query (requires user click)

---

## Extensibility

**[DR-EXT-010] Adding New Filters**

Modular filter system allows easy additions. See `EXTENSIBILITY.md` for complete guide.

Steps:
1. Add filter function to `api/filters.rs`
2. Export from `api/mod.rs`
3. Add route to router in `lib.rs`
4. Add UI dropdown option in `index.html`
5. Add JavaScript handler in `app.js`

**[DR-EXT-020] Filter Naming Conventions**

- Route: `/api/filters/{filter-name-kebab-case}`
- Function: `{filter_name_snake_case}`
- UI dropdown value: `filter-{shortname}`
- Response filter_name: `{filter-name-kebab-case}`

**[DR-EXT-030] Common Filter Patterns**

Orphaned records:
```sql
SELECT * FROM table1
WHERE guid NOT IN (SELECT DISTINCT table1_id FROM junction_table)
```

Missing optional fields:
```sql
SELECT * FROM table WHERE optional_field IS NULL
```

Specific criteria:
```sql
SELECT * FROM table WHERE condition = true
```

---

## Configuration

**[DR-CONF-010] Port**

Default: 5725
Configurable via environment: `WKMP_DR_PORT=5725`

**[DR-CONF-020] Database Path**

Resolved via standard WKMP root folder resolution (4-tier priority).
Database file: `{root_folder}/wkmp.db`

**[DR-CONF-030] Static Assets**

Embedded in binary via `include_str!` macro:
- No external files required
- Zero-config deployment
- Version-locked UI assets

---

## Security

**[DR-SEC-010] Read-Only Database**

SQLite connection flags enforce read-only access:
- `mode=ro`: Read-only mode
- `immutable=1`: Treat database as immutable

Prevents accidental modification even with malicious requests.

**[DR-SEC-020] SQL Injection Protection**

All queries use SQLx prepared statements with parameter binding.
User input never concatenated into SQL strings.

**[DR-SEC-030] Input Validation**

- Work ID: UUID format validation
- Table names: Whitelist validation
- Sort columns: Whitelist validation
- Sort order: Enum validation ("asc" or "desc")
- Pattern: No validation (SQL LIKE syntax allows any pattern)

**[DR-SEC-040] Authentication Behavior**

wkmp-dr inherits the WKMP shared secret authentication system but operates in **authentication-disabled mode by default**.

**Authentication States:**

1. **Disabled (Default):** When `shared_secret == 0` (database setting)
   - ALL API endpoints are publicly accessible without credentials
   - No timestamp validation performed
   - No hash validation performed
   - Middleware immediately passes requests through
   - This is the standard localhost development configuration

2. **Enabled:** When `shared_secret != 0` (database setting)
   - Protected API routes require timestamp + hash validation per API-AUTH-025 through API-AUTH-031
   - Timestamp must be within 300 seconds of current time
   - Hash must match SHA-256 HMAC of request body with shared secret
   - 401 Unauthorized returned on validation failure

**Protected vs Public Endpoints:**

- **Public (no auth ever):** `/health`
- **Protected (auth when enabled):** All `/api/*` routes (tables, filters, searches)

**Implementation Reference:**
- Auth middleware: `wkmp-dr/src/api/auth.rs:42-46`
- Bypass logic: `if state.shared_secret == 0 { return Ok(next.run(request).await); }`
- Standard WKMP pattern: Consistent with wkmp-ui, wkmp-pd, wkmp-ai, wkmp-le

**Reasoning for Default-Disabled:**
- Runs on localhost only
- Read-only access to local database
- Trusted local environment
- Simplifies browser-based access (no auth context needed from wkmp-ai button)

**Warning:** Do not expose wkmp-dr to network. Localhost use only. If remote access is required, set `shared_secret != 0` in database and ensure all API clients provide valid auth credentials.

---

## Testing

**[DR-TEST-010] Automated Tests**

Node.js test script validates:
- Health endpoint returns 200
- Table browsing endpoints return correct structure
- Filter endpoints return correct format
- Search endpoints validate input
- Static assets load correctly

**[DR-TEST-020] Manual Testing**

Browser-based testing:
- UI rendering and responsive design
- Pagination navigation
- Saved searches functionality
- localStorage persistence
- Export/import features

**[DR-TEST-030] Database Scenarios**

Test with varying database states:
- Empty database (0 rows)
- Small database (< 100 rows per table)
- Large database (> 1000 rows, multiple pages)
- Missing optional fields (NULL values)

---

## Performance

**[DR-PERF-010] Pagination**

100 rows per page balances:
- Responsive UI rendering
- Network transfer size
- User browsing efficiency

**[DR-PERF-020] Query Optimization**

- `COUNT(*)` queries executed separately from data queries
- Indexes utilized where available (primary keys, foreign keys)
- LIMIT/OFFSET for efficient pagination

**[DR-PERF-030] Static Asset Serving**

Embedded assets served from memory:
- No disk I/O for static files
- Fast response times
- No caching headers needed (assets never change within version)

---

## Version Support

**[DR-VER-010] Full Version Only**

wkmp-dr included only in Full version packaging.
Binary not built for Lite or Minimal versions.

**[DR-VER-020] Database Compatibility**

Compatible with any WKMP database schema version.
Queries target core tables present in all schema versions.

Future schema changes may require filter updates if:
- Core tables renamed
- Core columns removed
- Junction tables restructured

---

## Integration Points

**[DR-INT-010] wkmp-ai Integration**

wkmp-ai home page includes "Database Review" button:
```html
<a href="http://localhost:5725/" target="_blank" class="button">Database Review</a>
```

Opens wkmp-dr in new browser tab.

**[DR-INT-020] Shared Database**

All WKMP modules share single SQLite database file.
Read-only connection prevents interference with other modules.

**[DR-INT-030] No Direct Module Communication**

wkmp-dr does not:
- Send HTTP requests to other modules
- Subscribe to SSE event streams
- Modify shared database

Fully decoupled inspection tool.

---

## Future Enhancements

**[DR-FUT-010] Potential Filters**

- Songs without albums
- Passages with multiple songs
- Duplicate files (by path or content hash)
- Large files (> X MB)
- Old files (created before date)

**[DR-FUT-020] Search Enhancements**

- Full-text search across all tables
- Date range filtering
- Advanced query builder UI
- Export results to CSV/JSON

**[DR-FUT-030] Visualization**

- Database statistics dashboard
- Table relationship diagrams
- Growth charts over time

---

## Revision History

**Version:** 1.3
**Status:** Current
**Last Updated:** 2025-11-01
**Author:** Claude Code

**Change Log:**
- v1.3 (2025-11-01): Technical debt remediation and test coverage improvements
  - **Code Quality:**
    - Extracted pagination helper module (`wkmp-dr/src/pagination.rs`) eliminating 24 lines of duplicated logic
    - Centralized `PAGE_SIZE` constant and `calculate_pagination()` function with bounds checking
    - Removed unused exports while preserving `list_tables()` function for future extensibility
  - **Test Coverage:**
    - Added 14 API integration tests covering all core endpoints (`wkmp-dr/tests/api_tests.rs`)
    - Added 3 security tests verifying 10MB body size limit enforcement (`wkmp-dr/tests/security_tests.rs`)
    - All tests use `shared_secret=0` auth bypass for simplified testing
    - Tests verify: health, table viewing, pagination, sorting, filters, search, error handling
  - **Module Structure:**
    - Updated architecture diagram to include `pagination.rs`
    - Enabled tower `util` feature in workspace Cargo.toml for test infrastructure
  - **Implementation Quality:**
    - Zero compiler warnings
    - 100% test pass rate (17 tests total)
    - Comprehensive edge case coverage (pagination boundaries, invalid inputs, large bodies)
- v1.2 (2025-11-01): Database field name corrections
  - **[DR-API-030]** Updated passages filter to use actual field names: `start_time_ticks, end_time_ticks, title` (not `start_seconds, end_seconds`)
  - **[DR-API-040]** Updated files filter to use actual field names: `path, duration, hash` (not `full_path, file_size_bytes`)
  - **[DR-API-050]** Updated FilterResponse example to match corrected field names
  - **[DR-API-060]** Updated work_id search to use `start_time_ticks, end_time_ticks, work_id` (not `start_seconds, end_seconds, created_at`)
  - **[DR-API-070]** Updated path search to use `path, duration, hash` and sort by `path ASC` (not `full_path, file_size_bytes` sorted by `created_at DESC`)
  - All changes reflect actual database schema per IMPL001; implementation was correct, specification was outdated
- v1.1 (2025-11-01): Authentication documentation correction
  - **[DR-SEC-040]** Corrected authentication behavior documentation
  - Clarified that wkmp-dr inherits WKMP shared secret system but defaults to disabled (shared_secret == 0)
  - Documented two authentication states: Disabled (default) vs Enabled (shared_secret != 0)
  - Confirmed that when shared_secret == 0, NO timestamp or hash validation is performed
  - Added implementation references to auth.rs bypass logic
  - Explained protected vs public endpoint distinction
- v1.0 (2025-11-01): Initial specification
  - Documented wkmp-dr design and implementation
  - Established DR-xxx requirement IDs
  - Created comprehensive API, UI, and architecture specifications

---
End of document - WKMP Database Review Module Design Specification
