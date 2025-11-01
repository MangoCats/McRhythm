# Requirements Index: Database Review Module (wkmp-dr)

**Plan:** PLAN015 - Database Review Module (wkmp-dr)
**Source:** wip/_database_review_analysis.md (Approach B)
**Created:** 2025-11-01
**Total Requirements:** 24 (10 P0, 9 P1, 5 P2)

---

## Requirements Summary

| Req ID | Type | Brief Description | Priority | Source |
|--------|------|-------------------|----------|--------|
| **REQ-DR-F-010** | Functional | Table-by-table raw content viewing | P0 | User requirement 1 |
| **REQ-DR-F-020** | Functional | Paginated table browsing (100 rows/page) | P0 | Derived |
| **REQ-DR-F-030** | Functional | Row count display per table | P0 | Derived |
| **REQ-DR-F-040** | Functional | Filtered views (passages lacking MBID) | P0 | User requirement 2 |
| **REQ-DR-F-050** | Functional | Filtered views (files without passages) | P1 | Derived |
| **REQ-DR-F-060** | Functional | Search by MusicBrainz Work ID | P0 | User requirement 3 |
| **REQ-DR-F-070** | Functional | Search by file path pattern | P1 | Derived |
| **REQ-DR-F-080** | Functional | Sort columns (ascending/descending) | P0 | User requirement 4 |
| **REQ-DR-F-090** | Functional | Save favorite searches | P0 | User requirement 5 |
| **REQ-DR-F-100** | Functional | User preference persistence (browser localStorage) | P0 | User requirement 6 |
| **REQ-DR-F-110** | Functional | Extensible view system (add new filters/searches) | P1 | User requirement 4 |
| **REQ-DR-NF-010** | Non-Functional | Zero-config startup (4-tier root folder resolution) | P0 | REQ-NF-030 through REQ-NF-037 |
| **REQ-DR-NF-020** | Non-Functional | Read-only database access (SQLite mode=ro) | P0 | Safety requirement |
| **REQ-DR-NF-030** | Non-Functional | API authentication (timestamp + SHA-256 hash) | P0 | API-AUTH-025 |
| **REQ-DR-NF-040** | Non-Functional | Health endpoint GET /health within 2s | P0 | REQ-NF-050 through REQ-NF-053 |
| **REQ-DR-NF-050** | Non-Functional | Port 5725 assignment | P0 | Architecture |
| **REQ-DR-NF-060** | Non-Functional | On-demand microservice pattern | P0 | ARCH-OD-010 |
| **REQ-DR-NF-070** | Non-Functional | wkmp-ui launch button integration | P1 | On-demand pattern |
| **REQ-DR-NF-080** | Non-Functional | Full version packaging only | P0 | Architecture decision |
| **REQ-DR-UI-010** | UI | Inline HTML templates (wkmp-ai pattern) | P1 | Consistency |
| **REQ-DR-UI-020** | UI | Vanilla JavaScript (no frameworks) | P1 | Consistency |
| **REQ-DR-UI-030** | UI | CSS custom properties (WKMP styling) | P1 | Consistency |
| **REQ-DR-UI-040** | UI | Mobile-responsive design | P2 | Usability |
| **REQ-DR-UI-050** | UI | Table rendering with pagination controls | P0 | Core functionality |

---

## Priority Definitions

- **P0 (Critical):** Must be implemented for MVP, blocks release if missing
- **P1 (High):** Should be implemented for full feature set, can defer to next release
- **P2 (Medium):** Nice to have, defer if time constrained

---

## Detailed Requirements

### REQ-DR-F-010: Table-by-Table Raw Content Viewing
**Priority:** P0
**Type:** Functional
**Description:** Module SHALL provide ability to select any database table and view all rows with all columns.
**Acceptance Criteria:**
- User can select table from dropdown listing all tables in wkmp.db
- Selected table displays all columns with proper data types
- All rows visible (with pagination per REQ-DR-F-020)
**Dependencies:** Database schema known, read-only connection established
**Source:** User requirement 1

---

### REQ-DR-F-020: Paginated Table Browsing
**Priority:** P0
**Type:** Functional
**Description:** Table views SHALL display maximum 100 rows per page with pagination controls.
**Rationale:** Large tables (5000+ passages) must not overwhelm browser or network.
**Acceptance Criteria:**
- Default page size: 100 rows
- Pagination controls: First, Previous, Next, Last, Page N of M
- Current page number displayed
- Total row count displayed
**Dependencies:** REQ-DR-F-030 (row count)
**Source:** Derived from performance requirements

---

### REQ-DR-F-030: Row Count Display Per Table
**Priority:** P0
**Type:** Functional
**Description:** For each table, SHALL display total row count.
**Acceptance Criteria:**
- Row count query executes in <500ms for tables with 100k rows
- Count displayed in table dropdown: "passages (5,234 rows)"
- Count updates on page refresh
**Dependencies:** Database connection
**Source:** Derived usability requirement

---

### REQ-DR-F-040: Filtered View - Passages Lacking MBID
**Priority:** P0
**Type:** Functional
**Description:** SHALL provide predefined filtered view showing passages without MusicBrainz Recording ID.
**Query Logic:** SELECT * FROM passages WHERE guid NOT IN (SELECT passage_guid FROM passage_songs)
**Acceptance Criteria:**
- "Passages without MBID" button in filter panel
- Displays passage GUID, file path, start/end times
- Shows count of unlinked passages
**Dependencies:** REQ-DR-F-010
**Source:** User requirement 2 (example filter)

---

### REQ-DR-F-050: Filtered View - Files Without Passages
**Priority:** P1
**Type:** Functional
**Description:** SHALL provide filtered view showing audio files not segmented into passages.
**Query Logic:** SELECT * FROM files WHERE guid NOT IN (SELECT file_id FROM passages)
**Acceptance Criteria:**
- "Files without passages" button in filter panel
- Displays file GUID, path, duration
- Shows count of unsegmented files
**Dependencies:** REQ-DR-F-010
**Source:** Derived (common troubleshooting need)

---

### REQ-DR-F-060: Search by MusicBrainz Work ID
**Priority:** P0
**Type:** Functional
**Description:** SHALL allow user to search for passages linked to a specific MusicBrainz Work ID.
**Query Logic:**
```sql
SELECT p.* FROM passages p
JOIN passage_songs ps ON p.guid = ps.passage_guid
JOIN songs s ON ps.song_guid = s.guid
WHERE s.work_mbid = ?
```
**Acceptance Criteria:**
- Search form accepts UUID input (validated format)
- Returns all matching passages with file paths
- Shows count of passages found
- Handles zero results gracefully
**Dependencies:** REQ-DR-F-010
**Source:** User requirement 3 (example search)

---

### REQ-DR-F-070: Search by File Path Pattern
**Priority:** P1
**Type:** Functional
**Description:** SHALL allow user to search files by path substring.
**Query Logic:** SELECT * FROM files WHERE path LIKE '%' || ? || '%'
**Acceptance Criteria:**
- Search form accepts text input
- Case-insensitive matching
- Returns matching files with metadata
**Dependencies:** REQ-DR-F-010
**Source:** Derived (common troubleshooting need)

---

### REQ-DR-F-080: Sort Columns
**Priority:** P0
**Type:** Functional
**Description:** User SHALL be able to sort any table column in ascending or descending order.
**Acceptance Criteria:**
- Click column header to sort ascending
- Click again to sort descending
- Visual indicator (arrow) shows sort direction
- Sort persists across pagination
**Dependencies:** REQ-DR-F-010
**Source:** User requirement 4

---

### REQ-DR-F-090: Save Favorite Searches
**Priority:** P0
**Type:** Functional
**Description:** User SHALL be able to save current search/filter configuration for reuse.
**Acceptance Criteria:**
- "Save this search" button available on all views
- User provides name for saved search
- Saved searches appear in favorites dropdown
- Selecting favorite restores exact search parameters
**Dependencies:** REQ-DR-F-100 (persistence)
**Source:** User requirement 5

---

### REQ-DR-F-100: User Preference Persistence
**Priority:** P0
**Type:** Functional
**Description:** User preferences (saved searches, default table, pagination settings) SHALL persist across application restarts.
**Implementation:** Browser localStorage (JSON)
**Acceptance Criteria:**
- Preferences stored in localStorage on change
- Preferences loaded on page load
- Preferences survive browser restart
- Clear preferences option available
**Dependencies:** None
**Source:** User requirement 6

---

### REQ-DR-F-110: Extensible View System
**Priority:** P1
**Type:** Functional
**Description:** Architecture SHALL support adding new predefined filters/searches without major refactoring.
**Acceptance Criteria:**
- New filter added by: (1) SQL query definition, (2) UI button configuration
- Filter system uses plugin-like pattern
- Documentation for adding custom filters
**Dependencies:** REQ-DR-F-010
**Source:** User requirement 4 + "frequently added features"

---

### REQ-DR-NF-010: Zero-Config Startup
**Priority:** P0
**Type:** Non-Functional
**Description:** Module SHALL implement WKMP's 4-tier root folder resolution without manual configuration.
**Implementation:**
```rust
let resolver = wkmp_common::config::RootFolderResolver::new("database-review");
let root_folder = resolver.resolve();
let initializer = wkmp_common::config::RootFolderInitializer::new(root_folder);
initializer.ensure_directory_exists()?;
let db_path = initializer.database_path();
```
**Priority Order:**
1. CLI: `--root-folder /path` or `--root /path`
2. ENV: `WKMP_ROOT_FOLDER` or `WKMP_ROOT`
3. TOML: `~/.config/wkmp/wkmp-dr.toml`
4. Default: `~/Music` (Linux/macOS), `%USERPROFILE%\Music` (Windows)
**Acceptance Criteria:**
- No hardcoded paths in code
- Module starts without configuration file
- All 4 tiers tested
**Dependencies:** wkmp_common::config utilities
**Source:** REQ-NF-030 through REQ-NF-037

---

### REQ-DR-NF-020: Read-Only Database Access
**Priority:** P0
**Type:** Non-Functional (Safety)
**Description:** Database connection SHALL be read-only to prevent accidental corruption.
**Implementation:** SQLite connection string: `sqlite://{path}?mode=ro`
**Acceptance Criteria:**
- Any INSERT/UPDATE/DELETE attempt fails
- Test: Attempt write operation, verify error
- Log connection mode on startup
**Dependencies:** Database connection initialization
**Source:** Safety requirement (shared database)

---

### REQ-DR-NF-030: API Authentication
**Priority:** P0
**Type:** Non-Functional (Security)
**Description:** All HTTP endpoints SHALL require timestamp + SHA-256 hash authentication per WKMP standard.
**Implementation:**
- Shared secret: i64 from `settings` table (key: `shared_secret`)
- Timestamp: Unix epoch milliseconds
- Hash: SHA-256(canonical_json + shared_secret)
- Window: ≤1000ms past, ≤1ms future
**Acceptance Criteria:**
- Request without auth → HTTP 401
- Request with invalid hash → HTTP 401
- Request with expired timestamp → HTTP 401
- Valid request → HTTP 200
**Dependencies:** wkmp_common auth utilities
**Source:** API-AUTH-025

---

### REQ-DR-NF-040: Health Endpoint
**Priority:** P0
**Type:** Non-Functional (Operational)
**Description:** GET /health endpoint SHALL respond within 2 seconds with JSON status.
**Response Format:**
```json
{
  "status": "healthy",
  "module": "wkmp-dr"
}
```
**Acceptance Criteria:**
- Response time <2s (99th percentile)
- HTTP 200 status code
- JSON content-type header
- Does NOT require authentication
**Dependencies:** None
**Source:** REQ-NF-050 through REQ-NF-053

---

### REQ-DR-NF-050: Port Assignment
**Priority:** P0
**Type:** Non-Functional (Architecture)
**Description:** Module SHALL bind to port 5725.
**Rationale:** Ports 5720-5724 already allocated to existing modules.
**Acceptance Criteria:**
- Binds to port 5725 on startup
- Logs port on startup: "wkmp-dr listening on http://localhost:5725"
- Port configurable via TOML (optional)
**Dependencies:** None
**Source:** Architecture decision

---

### REQ-DR-NF-060: On-Demand Microservice Pattern
**Priority:** P0
**Type:** Non-Functional (Architecture)
**Description:** Module SHALL follow WKMP on-demand microservice pattern.
**Pattern Requirements:**
- Own web UI served on dedicated port
- User accesses via browser (not embedded in wkmp-ui)
- Specialized UI for infrequent use (database inspection)
**Acceptance Criteria:**
- Module runs independently of other modules
- No embedded iframe in wkmp-ui
- "Return to WKMP" link present in UI
**Dependencies:** None
**Source:** ARCH-OD-010

---

### REQ-DR-NF-070: wkmp-ui Launch Button
**Priority:** P1
**Type:** Non-Functional (Integration)
**Description:** wkmp-ui SHALL provide launch button/link for database review module.
**Location:** Tools menu or Library view
**Behavior:** Opens http://localhost:5725 in new browser tab
**Acceptance Criteria:**
- Button labeled "Database Review"
- Opens in new tab (target="_blank")
- Tooltip: "Inspect database contents (read-only)"
**Dependencies:** REQ-DR-NF-050 (port)
**Source:** On-demand pattern integration

---

### REQ-DR-NF-080: Full Version Packaging
**Priority:** P0
**Type:** Non-Functional (Distribution)
**Description:** wkmp-dr binary SHALL be included in Full version only.
**Version Matrix:**
- Full: ✅ Included (wkmp-ap, wkmp-ui, wkmp-pd, wkmp-ai, wkmp-le, wkmp-dr)
- Lite: ❌ Not included
- Minimal: ❌ Not included
**Acceptance Criteria:**
- Build scripts package wkmp-dr with Full version
- README documents wkmp-dr as Full-only feature
**Dependencies:** Build/packaging infrastructure
**Source:** Architecture decision (user confirmed)

---

### REQ-DR-UI-010: Inline HTML Templates
**Priority:** P1
**Type:** UI (Consistency)
**Description:** UI pages SHALL use inline HTML strings following wkmp-ai pattern.
**Rationale:** Consistency with existing WKMP modules, zero build dependencies.
**Acceptance Criteria:**
- HTML defined as Rust string literals or `format!()` macros
- No external template engine (Tera, Askama, etc.)
- Matches wkmp-ai UI style
**Dependencies:** None
**Source:** Architectural consistency

---

### REQ-DR-UI-020: Vanilla JavaScript
**Priority:** P1
**Type:** UI (Consistency)
**Description:** Frontend SHALL use vanilla JavaScript (ES6+) without frameworks.
**Rationale:** Zero npm dependencies, fast loading, simple debugging.
**Acceptance Criteria:**
- No React, Vue, Angular, or similar frameworks
- No npm package.json
- No build step required
**Dependencies:** None
**Source:** Architectural consistency

---

### REQ-DR-UI-030: CSS Custom Properties
**Priority:** P1
**Type:** UI (Consistency)
**Description:** Styling SHALL use CSS custom properties matching WKMP theme.
**Variables:** `:root` variables for colors, spacing, typography
**Acceptance Criteria:**
- UI visually consistent with wkmp-ai pages
- Reuses WKMP color scheme
- System font stack (no external fonts)
**Dependencies:** None
**Source:** UI/UX consistency

---

### REQ-DR-UI-040: Mobile-Responsive Design
**Priority:** P2
**Type:** UI (Usability)
**Description:** UI SHOULD adapt to mobile screen sizes (≥375px width).
**Acceptance Criteria:**
- Tables scroll horizontally on small screens
- Touch-friendly controls (≥44px tap targets)
- Readable text (≥14px font size)
**Dependencies:** None
**Source:** Usability best practice

---

### REQ-DR-UI-050: Table Rendering with Pagination
**Priority:** P0
**Type:** UI (Core)
**Description:** Table view SHALL render rows with pagination controls.
**Components:**
- Data grid (HTML table with CSS styling)
- Pagination bar (First, Prev, Next, Last, Page N of M)
- Row count display
**Acceptance Criteria:**
- Renders 100 rows smoothly (<100ms)
- Pagination controls functional
- Keyboard navigation supported (Tab, Enter)
**Dependencies:** REQ-DR-F-020
**Source:** Core UI functionality

---

## Requirement Traceability

**User Requirements → Derived Requirements:**
1. Table viewing → REQ-DR-F-010, REQ-DR-F-020, REQ-DR-F-030, REQ-DR-UI-050
2. Filtered views → REQ-DR-F-040, REQ-DR-F-050
3. Search views → REQ-DR-F-060, REQ-DR-F-070
4. Sorts/extensible → REQ-DR-F-080, REQ-DR-F-110
5. Save favorites → REQ-DR-F-090
6. Persistence → REQ-DR-F-100

**WKMP Architecture → Non-Functional Requirements:**
- REQ-NF-030 through REQ-NF-037 → REQ-DR-NF-010
- ARCH-OD-010 → REQ-DR-NF-060
- API-AUTH-025 → REQ-DR-NF-030
- REQ-NF-050 through REQ-NF-053 → REQ-DR-NF-040

---

## Requirements Coverage

| Category | Count | Percentage |
|----------|-------|------------|
| Functional (F) | 11 | 45.8% |
| Non-Functional (NF) | 8 | 33.3% |
| UI | 5 | 20.8% |
| **Total** | **24** | **100%** |

| Priority | Count | Percentage |
|----------|-------|------------|
| P0 (Critical) | 16 | 66.7% |
| P1 (High) | 6 | 25.0% |
| P2 (Medium) | 2 | 8.3% |
| **Total** | **24** | **100%** |

---

**Index Complete**
**Next:** See scope_statement.md for in/out of scope boundaries
