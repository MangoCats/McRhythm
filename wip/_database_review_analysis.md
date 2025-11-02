# Database Review Feature Analysis for WKMP

**Analysis Date:** 2025-11-01
**Analysis Method:** `/think` Multi-Agent Workflow (8-Phase Analysis)
**Analyst:** Claude Code
**Stakeholders:** WKMP Development Team
**Timeline:** Design phase (implementation via `/plan` if approved)

---

## Executive Summary

### Purpose
Evaluate approaches for adding database review capabilities to WKMP system for read-only inspection of wkmp.db content.

### Requirements (from user request)
1. Table-by-table raw content viewing
2. Filtered views (e.g., passages lacking MBID)
3. Search views (e.g., find passages by MusicBrainz Work ID)
4. Sorts and extensible views
5. Save favorite configurations/searches
6. User preference persistence across restarts

### Approaches Evaluated
- **Approach A:** Add to existing wkmp-ai module
- **Approach B:** Create new independent microservice (wkmp-db)
- **Approach C:** Integrate existing FOSS tools (sqlite-web, SQLPage, DB Browser)
- **Approach D:** Hybrid (FOSS tool + WKMP integration layer)
- **Approach E:** Minimal (browser-based viewer + file serving)

### Quick Navigation
- **Critical Findings:** See section 3
- **Detailed Approach Analysis:** See section 4
- **Risk-Based Comparison:** See section 5
- **Recommendation:** See section 6

---

## 1. Current State Assessment

### 1.1 Existing Database Access Patterns

**Shared Database:** `/home/sw/Dev/McRhythm/wkmp.db` (SQLite 3)
- Location determined by 4-tier resolution (CLI → ENV → TOML → Default)
- All 5 microservices connect to same database file
- Read-write access patterns in wkmp-ap (queue management, settings)
- Read-write access in wkmp-ai (import workflow, metadata)
- Read-mostly access in wkmp-ui (display playback state)

**Database Schema (from wkmp-common/src/db/init.rs):**
- 8 core tables: `users`, `settings`, `module_config`, `files`, `passages`, `queue`, `songs`, `artists`, `albums`, `works`
- 3 linking tables: `passage_songs`, `song_artists`, `passage_albums`
- 2 operational tables: `schema_version`, `acoustid_cache`

### 1.2 Existing UI Patterns in wkmp-ai

**Technology Stack:**
- **Backend:** Axum (async HTTP framework)
- **Database:** sqlx (compile-time verified queries, async)
- **Frontend:** Vanilla JavaScript (ES6+), no frameworks
- **Templates:** Inline HTML strings (no template engine)
- **Styling:** Inline CSS with custom properties
- **Real-time:** Server-Sent Events (SSE)

**Code Characteristics (from wkmp-ai/src research):**
- Module size: ~8,362 lines Rust (excluding tests)
- UI pages: 5 pages (root, import-progress, segment-editor, import-complete, settings)
- API pattern: Module-per-feature (api/settings.rs, api/import_workflow.rs, etc.)
- Database pattern: Module-per-table (db/files.rs, db/passages.rs, etc.)
- Static assets: `wkmp-ai/static/` served via tower-http

**Extensibility Assessment:** EASY (2/5 difficulty)
- Database access patterns well-established
- UI framework (inline HTML) follows existing patterns
- Modular routing with clear extension points

### 1.3 WKMP Architectural Constraints

**MANDATORY Requirements:**

1. **Zero-Config Startup** ([REQ-NF-030] through [REQ-NF-037])
   - Must use `wkmp_common::config::RootFolderResolver` and `RootFolderInitializer`
   - 4-tier priority: CLI args → ENV → TOML → Compiled default
   - Database path: `<root_folder>/wkmp.db` (never hardcoded)

2. **On-Demand Microservices Pattern** ([ARCH-OD-010])
   - Each provides own web UI on dedicated port
   - User accesses via browser (not embedded in wkmp-ui)
   - wkmp-ui provides launch points (links/buttons)
   - Specialized UIs for infrequent operations

3. **API Authentication** ([API-AUTH-025])
   - Timestamp + SHA-256 hash required for all HTTP requests
   - Shared secret from settings table
   - Secret embedded in served HTML (no separate GET endpoint)

4. **Health Monitoring** ([REQ-NF-050] through [REQ-NF-053])
   - GET /health endpoint required
   - JSON response: `{"status": "healthy", "module": "<name>"}`
   - Response within 2 seconds

5. **Version Packaging**
   - Full: All 5 binaries (wkmp-ap, wkmp-ui, wkmp-pd, wkmp-ai, wkmp-le)
   - Lite: 3 binaries (wkmp-ap, wkmp-ui, wkmp-pd)
   - Minimal: 2 binaries (wkmp-ap, wkmp-ui)
   - No conditional compilation (binaries differ only by packaging)

**Available Port Assignment:**
- Ports 5720-5724 currently allocated
- Port 5725 available for new module (wkmp-db)

---

## 2. FOSS Alternatives Research

### 2.1 Web-Based SQLite Viewers

#### sqlite-web (Python)
- **Language:** Python (Flask framework)
- **License:** MIT
- **Deployment:** Requires Python runtime + pip dependencies
- **Features:** Export JSON/CSV, import JSON/CSV, browse/edit/delete, password protection, SSL
- **Read-only mode:** Supported (`--readonly` flag)
- **Customization:** Moderate (Python/Jinja2 templates)
- **GitHub:** coleifer/sqlite-web (~4.7k stars)

#### SQLPage (Rust)
- **Language:** Rust (single executable)
- **License:** MIT
- **Deployment:** Single binary (Rust compiled)
- **Features:** SQL-driven web pages, plots, forms, file uploads, user management, HTTP/2, HTTPS
- **Read-only mode:** Not explicit (requires SQL-level permissions)
- **Customization:** High (write .sql files to define pages)
- **Crates.io:** crates.io/crates/sqlpage
- **Unique:** Declarative SQL-based UI definition

#### Adminer (PHP)
- **Language:** PHP (single file)
- **License:** Apache 2.0 / GPL 2
- **Deployment:** Requires PHP runtime + web server
- **Features:** Multi-database support (SQLite, MySQL, PostgreSQL, etc.), lightweight (single file)
- **Read-only mode:** Partial (UI-level, not enforced)
- **Customization:** Low (PHP editing required)

### 2.2 Desktop Applications

#### DB Browser for SQLite (DB4S)
- **Language:** C++ (Qt framework)
- **License:** MPL-2.0 / GPL-3.0
- **Deployment:** Desktop application (Windows, macOS, Linux)
- **Features:** Visual table editor, SQL query execution, plot creation, schema visualization
- **Integration:** No web interface (CLI automation possible)
- **Maturity:** Very high (~20k GitHub stars, active since 2014)

#### SQLiteStudio
- **Language:** C++ (Qt framework)
- **License:** GPL-3.0
- **Deployment:** Desktop application (cross-platform)
- **Features:** Plugin system, custom functions, data export
- **Integration:** No web interface (CLI available)

### 2.3 Browser-Based (Client-Side) Viewers

#### SQLite Viewer Web App (sqliteviewer.app)
- **Language:** JavaScript (WebAssembly via sql.js)
- **License:** Apache 2.0 (inloop/sqlite-viewer)
- **Deployment:** Static HTML + WASM (drag-and-drop .db file)
- **Features:** Fully client-side, no server required, table browsing, SQL queries
- **Privacy:** Complete (never leaves browser)
- **Limitations:** No server-side features (no saved preferences, no authentication)

#### SQLite Online (github.com/vwh/sqlite-online)
- **Language:** JavaScript (WebAssembly)
- **Deployment:** Static site
- **Features:** Similar to SQLite Viewer, client-side only
- **Limitations:** Same as above (no server-side persistence)

### 2.4 FOSS Analysis Summary

**Strengths:**
- Mature, well-tested codebases (DB4S, sqlite-web)
- Rich feature sets (query execution, export, visualization)
- Active communities and documentation

**Weaknesses for WKMP Integration:**
- **Zero-config compliance:** NONE support WKMP's 4-tier root folder resolution
- **API authentication:** NONE implement WKMP's timestamp+hash protocol
- **On-demand pattern:** NONE integrate with wkmp-ui launch mechanism
- **Version packaging:** NONE fit WKMP's binary packaging model
- **Runtime dependencies:** Python (sqlite-web), PHP (Adminer), Qt (desktop apps)
- **Custom features:** NONE support WKMP-specific views (passages lacking MBID, work ID searches)

**Integration Effort:**
- Wrapper development to handle zero-config and authentication
- Custom WKMP-specific query views require fork or plugin
- User preference persistence requires custom implementation

---

## 3. Critical Findings

### Finding 1: No FOSS Tool Satisfies WKMP Architectural Requirements Out-of-the-Box

**Evidence:**
- Reviewed sqlite-web, SQLPage, Adminer, DB Browser for SQLite, browser-based viewers
- NONE implement:
  - WKMP's 4-tier zero-config root folder resolution
  - WKMP's timestamp+hash API authentication
  - WKMP's on-demand microservice launch pattern
  - WKMP-specific filtered views (passages lacking MBID, work ID searches)

**Implication:**
- Any FOSS tool requires significant integration wrapper
- Integration effort may exceed custom implementation effort
- Custom implementation provides tighter architectural alignment

### Finding 2: wkmp-ai Provides Strong Extensibility Foundation

**Evidence (from wkmp-ai/src research):**
- Modular API routing: Add `src/api/database_viewer.rs`, merge routes in `lib.rs`
- Modular database access: Add query functions to `src/db/*.rs` modules
- Static asset serving: Already configured via tower-http
- Inline HTML templates: Consistent pattern for new pages
- Estimated effort for basic viewer: **8-13 hours**

**Implication:**
- Extending wkmp-ai is low-risk, low-effort approach
- Leverages existing patterns and infrastructure
- Maintains architectural consistency

### Finding 3: Database Review Fits On-Demand Pattern Perfectly

**Evidence:**
- Specialized UI for data inspection/debugging
- Infrequent use (troubleshooting, data exploration)
- Complex visualization needs (table browsing, query execution)
- Matches wkmp-ai and wkmp-le characteristics

**Implication:**
- Should follow on-demand pattern if implemented as separate module
- Accessed via wkmp-ui launch button ("Database Review" in Tools menu)
- Opens in new browser tab (not embedded)

### Finding 4: Read-Only Mode is Critical for Safety

**Evidence:**
- wkmp.db is shared by all 5 microservices
- Concurrent writes can cause corruption (SQLite write locking)
- Database review is for inspection, not modification
- User preferences can be stored separately (settings table or local storage)

**Implication:**
- SQLite connection MUST use `?mode=ro` (read-only)
- Saved searches/preferences stored in browser localStorage or separate config file
- Eliminates risk of accidental database corruption

### Finding 5: Custom Implementation Provides Best Long-Term Value

**Rationale:**
- WKMP-specific filtered views (passages lacking MBID) require custom code anyway
- Integration wrappers for FOSS tools add complexity without reducing code
- Custom implementation maintains architectural consistency
- Estimated effort (8-13 hours for basic, 20-30 hours for full-featured) is manageable
- No external runtime dependencies (Python, PHP, Qt)

---

## 4. Detailed Approach Analysis

### Approach A: Add to Existing wkmp-ai Module

**Description:**
Extend wkmp-ai with database review pages and API endpoints, following existing patterns.

**Implementation Pattern:**
1. Create `wkmp-ai/src/api/database_viewer.rs` (routing + handlers)
2. Add query functions to `wkmp-ai/src/db/*.rs` (list_all, count_by, search_by)
3. Create HTML pages (inline templates or static files in `wkmp-ai/static/`)
4. Add CSS/JS for table rendering, filtering, searching
5. Merge routes in `wkmp-ai/src/lib.rs` via `.merge(api::database_viewer_routes())`
6. Add navigation link in existing wkmp-ai UI (root page or settings page)

**Architectural Fit:**
- ✅ **Zero-config:** Already implements WKMP pattern (wkmp-ai/src/main.rs:34-45)
- ✅ **API authentication:** Already implements timestamp+hash (shared via AppState)
- ✅ **Health endpoint:** Already exists (wkmp-ai/src/api/health.rs)
- ✅ **Port assignment:** Uses existing port 5723 (no new port needed)
- ✅ **Version packaging:** wkmp-ai included in Full version only

**Implementation Effort:**
- **Database queries:** 3-5 hours (list tables, count rows, filtered queries)
- **HTML/CSS/JS:** 6-10 hours (table rendering, filtering UI, search forms)
- **Testing:** 3-5 hours (integration tests, manual testing)
- **Total:** **12-20 hours**

**Maintainability:**
- **High:** Follows existing wkmp-ai patterns (same codebase, same conventions)
- **Risk:** Low (extends working module, no new architecture)
- **Code reuse:** High (shares database pool, authentication, routing infrastructure)

**User Experience:**
- Access: http://localhost:5723/database (from wkmp-ui link or direct)
- Navigation: Add "Database Review" link to wkmp-ai root page
- Consistency: Matches wkmp-ai UI styling (CSS custom properties)
- Learning curve: None (same UI patterns as import workflow)

**Pros:**
- ✅ Lowest implementation effort (reuses infrastructure)
- ✅ Zero new architectural components
- ✅ High code reuse (database pool, auth, routing)
- ✅ Consistent UI/UX with existing wkmp-ai pages
- ✅ Zero new runtime dependencies
- ✅ Automatic zero-config compliance (inherits from wkmp-ai)

**Cons:**
- ⚠️ wkmp-ai semantic scope expands ("Audio Ingest" now includes database review)
- ⚠️ Available only in Full version (wkmp-ai not in Lite/Minimal)
- ⚠️ Single port for two distinct functions (import vs. review)

**Risk Assessment:**
- **Failure Risk:** **Low**
- **Failure Modes:**
  1. Inline HTML becomes unwieldy for complex tables → Probability: Low, Impact: Low
     - Mitigation: Use static HTML files (like settings.html pattern)
  2. Database queries slow on large datasets → Probability: Low, Impact: Medium
     - Mitigation: Add pagination, row limits, indexes
- **Residual Risk:** **Low**

**Quality Characteristics:**
- **Maintainability:** High (single codebase, established patterns)
- **Test Coverage:** High (integration tests straightforward)
- **Architectural Alignment:** Strong (extends existing on-demand module)

---

### Approach B: Create New Independent Microservice (wkmp-db)

**Description:**
Develop new standalone microservice following wkmp-ai pattern with dedicated port (5725).

**Implementation Pattern:**
1. Create `wkmp-db/` crate structure (mirror wkmp-ai organization)
2. Implement zero-config startup (RootFolderResolver, RootFolderInitializer)
3. Implement API authentication (timestamp+hash, shared secret)
4. Implement health endpoint (GET /health)
5. Develop database viewer UI (HTML/CSS/JS)
6. Add launch button in wkmp-ui ("Database Review" in Tools menu)
7. Update version packaging scripts (include wkmp-db in Full version)

**Architectural Fit:**
- ✅ **Zero-config:** Must implement manually (copy wkmp-ai pattern)
- ✅ **API authentication:** Must implement manually (copy wkmp-ai pattern)
- ✅ **Health endpoint:** Must implement (trivial)
- ✅ **Port assignment:** Uses new port 5725
- ✅ **Version packaging:** Included in Full version only (or all versions, TBD)
- ✅ **On-demand pattern:** Perfect match (dedicated specialized tool)

**Implementation Effort:**
- **Boilerplate:** 8-12 hours (crate setup, zero-config, auth, health)
- **Database queries:** 3-5 hours (same as Approach A)
- **HTML/CSS/JS:** 6-10 hours (same as Approach A)
- **wkmp-ui integration:** 2-3 hours (launch button, navigation link)
- **Testing:** 4-6 hours (more integration points)
- **Total:** **23-36 hours**

**Maintainability:**
- **Medium:** New codebase requires independent maintenance
- **Risk:** Medium (new module, more architectural surface area)
- **Code reuse:** Medium (uses wkmp-common utilities, but copies auth patterns)

**User Experience:**
- Access: http://localhost:5725 (from wkmp-ui link)
- Navigation: "Database Review" button in wkmp-ui Tools menu
- Consistency: Must replicate WKMP styling manually
- Learning curve: None (same patterns as wkmp-ai/wkmp-le)

**Pros:**
- ✅ Clear semantic separation (audio ingest ≠ database review)
- ✅ Independent deployment/scaling (not coupled to wkmp-ai)
- ✅ Dedicated port (no confusion with wkmp-ai functions)
- ✅ Can be included in Lite/Minimal versions (decision flexibility)
- ✅ Follows on-demand pattern explicitly

**Cons:**
- ❌ 2x implementation effort vs. Approach A (boilerplate duplication)
- ❌ New architectural component (more ports, more processes)
- ❌ More integration points (wkmp-ui launch, version packaging)
- ❌ Lower code reuse (must copy authentication, routing patterns)

**Risk Assessment:**
- **Failure Risk:** **Low-Medium**
- **Failure Modes:**
  1. Zero-config implementation differs from wkmp-ai → Probability: Low, Impact: Medium
     - Mitigation: Copy wkmp-ai main.rs pattern exactly
  2. API authentication implementation has bugs → Probability: Low, Impact: High
     - Mitigation: Extensive testing, copy wkmp-ai implementation verbatim
  3. Port conflicts with other software → Probability: Low, Impact: Low
     - Mitigation: Document port 5725 usage, make configurable
- **Residual Risk:** **Low-Medium**

**Quality Characteristics:**
- **Maintainability:** Medium (independent codebase, but more components)
- **Test Coverage:** High (requires more integration tests)
- **Architectural Alignment:** Strong (follows on-demand pattern explicitly)

---

### Approach C: Integrate Existing FOSS Tool

**Description:**
Deploy FOSS tool (sqlite-web, SQLPage, Adminer) with custom integration wrapper for WKMP compliance.

**Implementation Pattern (using sqlite-web as example):**
1. Package sqlite-web Python dependencies with WKMP
2. Create Rust wrapper binary (`wkmp-db-wrapper`) to:
   - Resolve root folder using WKMP's 4-tier priority
   - Locate wkmp.db
   - Generate sqlite-web configuration
   - Launch sqlite-web subprocess on port 5725
   - Proxy requests through WKMP authentication layer
3. Implement API authentication proxy (Axum middleware)
4. Add wkmp-ui launch button
5. Update version packaging

**Architectural Fit:**
- ⚠️ **Zero-config:** Requires custom wrapper (non-trivial)
- ⚠️ **API authentication:** Requires proxy layer (significant complexity)
- ✅ **Health endpoint:** Wrapper can implement
- ✅ **Port assignment:** Wrapper controls port
- ⚠️ **Version packaging:** Must bundle Python runtime + dependencies

**Implementation Effort:**
- **Wrapper development:** 10-15 hours (zero-config, subprocess management, config generation)
- **Authentication proxy:** 8-12 hours (Axum middleware, request forwarding, hash validation)
- **Python packaging:** 5-8 hours (pip dependencies, virtualenv, cross-platform)
- **WKMP-specific views:** 6-10 hours (custom SQL queries, UI customization)
- **wkmp-ui integration:** 2-3 hours (same as Approach B)
- **Testing:** 6-10 hours (wrapper, proxy, subprocess lifecycle)
- **Total:** **37-58 hours**

**Maintainability:**
- **Low:** Two-language codebase (Rust + Python), dependency management complexity
- **Risk:** High (wrapper bugs, subprocess lifecycle issues, Python version conflicts)
- **Code reuse:** Low (FOSS tool code, but high integration overhead)

**User Experience:**
- Access: http://localhost:5725 (proxied through wrapper)
- Navigation: Same as Approach B
- Consistency: ⚠️ sqlite-web UI styling differs from WKMP
- Feature richness: ✅ High (mature FOSS tool features)

**Pros:**
- ✅ Rich feature set (export, import, plot visualization)
- ✅ Mature, well-tested codebase (sqlite-web ~4.7k stars)
- ✅ Active community and documentation

**Cons:**
- ❌ **3x implementation effort vs. Approach A** (wrapper + proxy + packaging)
- ❌ Runtime dependency: Python + pip packages
- ❌ Two-language maintenance burden (Rust + Python)
- ❌ Architectural complexity (wrapper, proxy, subprocess)
- ❌ WKMP-specific features still require custom code
- ❌ UI styling inconsistency with WKMP
- ❌ Increased binary size (bundle Python runtime)

**Risk Assessment:**
- **Failure Risk:** **Medium-High**
- **Failure Modes:**
  1. Subprocess lifecycle bugs (crash, restart, zombie processes) → Probability: Medium, Impact: High
     - Mitigation: Robust process supervision, restart logic
  2. Python version conflicts across platforms → Probability: Medium, Impact: High
     - Mitigation: Bundle Python runtime, test on all platforms
  3. Authentication proxy bypass vulnerabilities → Probability: Low, Impact: Critical
     - Mitigation: Extensive security testing, penetration testing
  4. Wrapper zero-config logic diverges from WKMP → Probability: Medium, Impact: Medium
     - Mitigation: Use wkmp-common utilities, comprehensive testing
- **Residual Risk:** **Medium** (after extensive mitigation)

**Quality Characteristics:**
- **Maintainability:** Low (two languages, complex integration)
- **Test Coverage:** Medium (difficult to test subprocess interactions)
- **Architectural Alignment:** Weak (proxy layer, subprocess management)

---

### Approach D: Hybrid (FOSS Tool + Minimal Wrapper)

**Description:**
Use SQLPage (Rust-based) with thin WKMP integration layer for zero-config and authentication.

**Implementation Pattern:**
1. Add SQLPage as Cargo dependency (`crates.io/crates/sqlpage`)
2. Create `wkmp-db` binary that:
   - Resolves root folder (WKMP's 4-tier priority)
   - Generates SQLPage configuration pointing to wkmp.db
   - Embeds authentication middleware (Axum layer before SQLPage)
   - Launches SQLPage programmatically
3. Write `.sql` files for WKMP-specific views (passages lacking MBID, work ID searches)
4. Add wkmp-ui launch button
5. Update version packaging

**Architectural Fit:**
- ✅ **Zero-config:** Wrapper implements WKMP pattern
- ⚠️ **API authentication:** Requires middleware layer (moderate complexity)
- ✅ **Health endpoint:** Wrapper implements
- ✅ **Port assignment:** Wrapper controls port
- ✅ **Version packaging:** SQLPage is Rust (single binary linkable)

**Implementation Effort:**
- **Wrapper development:** 6-10 hours (zero-config, SQLPage config generation)
- **Authentication middleware:** 6-10 hours (Axum layer integration)
- **SQLPage .sql files:** 8-12 hours (learn SQLPage DSL, write views)
- **WKMP-specific views:** 4-6 hours (custom queries)
- **wkmp-ui integration:** 2-3 hours (same as Approach B)
- **Testing:** 5-8 hours (middleware, SQLPage integration)
- **Total:** **31-49 hours**

**Maintainability:**
- **Medium:** Single language (Rust), but SQLPage DSL learning curve
- **Risk:** Medium (SQLPage API stability unknown, DSL complexity)
- **Code reuse:** Medium (SQLPage handles UI, wrapper handles integration)

**User Experience:**
- Access: http://localhost:5725
- Navigation: Same as Approach B
- Consistency: ⚠️ SQLPage UI styling (may differ from WKMP)
- Feature richness: ✅ High (SQLPage supports plots, forms, uploads)
- Learning curve: ⚠️ SQLPage .sql DSL for customization

**Pros:**
- ✅ Rust-only ecosystem (no Python/PHP dependencies)
- ✅ SQLPage handles UI complexity (plots, forms, tables)
- ✅ Declarative .sql file approach (maintainable)
- ✅ Single binary deployment

**Cons:**
- ❌ **2.5x implementation effort vs. Approach A**
- ❌ SQLPage learning curve (.sql DSL, configuration model)
- ❌ SQLPage maturity unknown (less proven than sqlite-web)
- ❌ UI styling inconsistency with WKMP
- ❌ Authentication middleware adds complexity

**Risk Assessment:**
- **Failure Risk:** **Medium**
- **Failure Modes:**
  1. SQLPage API changes break integration → Probability: Low, Impact: High
     - Mitigation: Pin SQLPage version, monitor updates
  2. SQLPage .sql DSL limitations prevent desired features → Probability: Medium, Impact: Medium
     - Mitigation: Prototype critical views early, fallback to custom UI
  3. Authentication middleware has vulnerabilities → Probability: Low, Impact: High
     - Mitigation: Security review, testing
- **Residual Risk:** **Low-Medium**

**Quality Characteristics:**
- **Maintainability:** Medium (Rust-only, but SQLPage DSL complexity)
- **Test Coverage:** Medium (SQLPage integration testing)
- **Architectural Alignment:** Moderate (wrapper layer, external DSL)

---

### Approach E: Minimal (Browser-Based Viewer + File Serving)

**Description:**
Serve static browser-based viewer (SQLite Viewer Web App) with wkmp.db file access.

**Implementation Pattern:**
1. Bundle SQLite Viewer Web App static files (HTML/JS/WASM) in `wkmp-ui/static/`
2. Add file serving endpoint in wkmp-ui: `GET /database/download` → streams wkmp.db (read-only copy)
3. Add "Database Review" button in wkmp-ui → opens `/static/sqlite-viewer.html`
4. User drags downloaded wkmp.db into browser viewer
5. All analysis happens client-side (JavaScript + WASM)

**Architectural Fit:**
- ✅ **Zero-config:** No new module (uses wkmp-ui)
- ✅ **API authentication:** wkmp-ui already implements
- ✅ **Health endpoint:** N/A (no new endpoint)
- ✅ **Port assignment:** Uses existing port 5720 (wkmp-ui)
- ✅ **Version packaging:** Static files (no new binary)

**Implementation Effort:**
- **Bundle static viewer:** 2-3 hours (download, integrate into wkmp-ui)
- **File download endpoint:** 3-5 hours (stream wkmp.db, authentication)
- **UI integration:** 2-3 hours (button, instructions)
- **Testing:** 2-3 hours (file download, viewer functionality)
- **Total:** **9-14 hours**

**Maintainability:**
- **High:** Minimal code (static files + file serving endpoint)
- **Risk:** Low (no complex integration)
- **Code reuse:** Very high (leverages wkmp-ui, no new components)

**User Experience:**
- Access: Click "Database Review" in wkmp-ui → Downloads wkmp.db → Drag to viewer
- Navigation: Embedded in wkmp-ui (no new tab)
- Consistency: ⚠️ SQLite Viewer styling (external tool)
- Feature richness: ⚠️ Basic (table browsing, SQL queries, no saved preferences)
- Learning curve: ⚠️ Manual file download + drag-and-drop (extra steps)

**Pros:**
- ✅ Lowest implementation effort (9-14 hours)
- ✅ Zero new architectural components
- ✅ Zero runtime dependencies
- ✅ Client-side processing (no server load)
- ✅ Available in all versions (wkmp-ui in all versions)

**Cons:**
- ❌ **No saved preferences** (client-side tool has no server storage)
- ❌ **No WKMP-specific views** (generic SQLite viewer)
- ❌ **Manual workflow** (download → drag-and-drop)
- ❌ **Read-only snapshot** (not live database view)
- ❌ UI styling inconsistency
- ❌ Limited feature set (no plots, no complex filtering UI)

**Risk Assessment:**
- **Failure Risk:** **Very Low**
- **Failure Modes:**
  1. File download fails for large databases → Probability: Low, Impact: Low
     - Mitigation: Stream with proper headers, test with large files
  2. Browser WASM support missing → Probability: Very Low, Impact: Low
     - Mitigation: Document browser requirements (Chrome/Firefox/Safari)
- **Residual Risk:** **Very Low**

**Quality Characteristics:**
- **Maintainability:** Very high (minimal code)
- **Test Coverage:** High (simple integration)
- **Architectural Alignment:** Strong (leverages existing wkmp-ui)

---

## 5. Risk-Based Comparison

### 5.1 Failure Risk Ranking

| Approach | Residual Risk | Rationale |
|----------|---------------|-----------|
| **E (Minimal)** | **Very Low** | Minimal code, no new components, simple file serving |
| **A (Extend wkmp-ai)** | **Low** | Reuses proven patterns, low complexity, well-understood codebase |
| **B (New microservice)** | **Low-Medium** | New module adds complexity, but follows established pattern |
| **D (SQLPage hybrid)** | **Low-Medium** | SQLPage maturity unknown, DSL complexity, middleware layer |
| **C (FOSS wrapper)** | **Medium** | Subprocess management, Python dependencies, proxy complexity |

### 5.2 Quality Characteristics Comparison

| Approach | Maintainability | Test Coverage | Architectural Alignment | Code Reuse |
|----------|----------------|---------------|------------------------|------------|
| **E** | Very High | High | Strong | Very High |
| **A** | High | High | Strong | High |
| **B** | Medium | High | Strong | Medium |
| **D** | Medium | Medium | Moderate | Medium |
| **C** | Low | Medium | Weak | Low |

### 5.3 Implementation Effort Comparison

| Approach | Effort (hours) | Effort Level | Boilerplate | Custom Features |
|----------|----------------|--------------|-------------|-----------------|
| **E** | 9-14 | Very Low | Minimal | None |
| **A** | 12-20 | Low | None (reuses) | Easy |
| **B** | 23-36 | Medium | High | Easy |
| **D** | 31-49 | Medium-High | Medium | Moderate |
| **C** | 37-58 | High | Very High | Moderate |

### 5.4 Feature Completeness Comparison

| Requirement | E | A | B | D | C |
|-------------|---|---|---|---|---|
| **Table viewing** | ✅ | ✅ | ✅ | ✅ | ✅ |
| **Filtered views** | ❌ | ✅ | ✅ | ✅ | ⚠️ |
| **Search views** | ⚠️ | ✅ | ✅ | ✅ | ✅ |
| **Sorts/extensible** | ⚠️ | ✅ | ✅ | ✅ | ✅ |
| **Save favorites** | ❌ | ✅ | ✅ | ✅ | ⚠️ |
| **Preference persistence** | ❌ | ✅ | ✅ | ✅ | ⚠️ |
| **WKMP-specific views** | ❌ | ✅ | ✅ | ⚠️ | ⚠️ |
| **Zero-config** | ✅ | ✅ | ✅ | ✅ | ⚠️ |
| **Read-only safe** | ✅ | ✅ | ✅ | ✅ | ✅ |

**Legend:** ✅ Full support, ⚠️ Partial support, ❌ Not supported

### 5.5 User Experience Comparison

| Aspect | E | A | B | D | C |
|--------|---|---|---|---|---|
| **Access simplicity** | ⚠️ (download+drag) | ✅ (one click) | ✅ (one click) | ✅ (one click) | ✅ (one click) |
| **UI consistency** | ❌ (external) | ✅ (WKMP style) | ✅ (WKMP style) | ⚠️ (SQLPage) | ⚠️ (sqlite-web) |
| **Learning curve** | ⚠️ (manual steps) | ✅ (familiar) | ✅ (familiar) | ⚠️ (DSL) | ✅ (familiar) |
| **Feature richness** | ⚠️ (basic) | ✅ (customized) | ✅ (customized) | ✅ (rich) | ✅ (rich) |
| **Performance** | ✅ (client-side) | ✅ (server) | ✅ (server) | ✅ (server) | ✅ (server) |

---

## 6. Recommendation

### 6.1 Primary Recommendation: **Approach A (Extend wkmp-ai)**

**Rationale:**

**Risk-First Analysis:**
- **Lowest risk among fully-featured approaches** (Low residual risk)
- Only Approach E has lower risk, but fails to meet requirements (no saved preferences, no WKMP-specific views)
- Reuses proven wkmp-ai patterns (inline HTML, modular routing, sqlx queries)
- Zero new architectural components (no new ports, no new binaries)
- Well-understood failure modes with straightforward mitigations

**Quality Characteristics:**
- **Maintainability:** High (single codebase, established patterns)
- **Test Coverage:** High (straightforward integration testing)
- **Architectural Alignment:** Strong (extends existing on-demand module)

**Implementation Effort:**
- **12-20 hours** for fully-featured implementation
- Lowest effort among approaches meeting all requirements
- No boilerplate duplication (reuses wkmp-ai infrastructure)

**Feature Completeness:**
- Meets all 6 user requirements
- Supports WKMP-specific filtered views (passages lacking MBID, work ID searches)
- User preference persistence via settings table or browser localStorage

**Effort Justification:**
- Approach E saves 3-6 hours but fails 3 of 6 requirements (50% failure rate)
- Approaches B/D/C require 1.5x-3x effort with higher risk
- **Risk reduction justifies implementation over shortcuts**

### 6.2 Alternative Recommendation: **Approach E (Minimal) as Phase 1**

**If immediate need exists and full implementation can wait:**

**Two-Phase Strategy:**
1. **Phase 1 (Immediate):** Implement Approach E (9-14 hours)
   - Provides basic database inspection capability quickly
   - Minimal risk, zero architectural changes
   - Available in all WKMP versions

2. **Phase 2 (When resources available):** Implement Approach A (12-20 hours)
   - Adds full feature set (saved preferences, WKMP-specific views)
   - Replaces Phase 1 viewer with integrated solution
   - Deprecate Phase 1 after Phase 2 deployment

**Rationale:**
- If troubleshooting need is urgent, Approach E delivers 70% value in 50% time
- Allows requirements refinement based on real usage
- Low switching cost (Phase 1 is simple file serving endpoint)

### 6.3 Not Recommended: Approaches B, C, D

**Approach B (New Microservice):**
- 2x implementation effort vs. Approach A
- Higher architectural complexity (new port, new binary)
- No significant benefit over Approach A
- **Verdict:** Effort not justified by benefits

**Approach C (FOSS Wrapper):**
- 3x implementation effort vs. Approach A
- Medium-High risk (subprocess, Python dependencies, proxy)
- Two-language maintenance burden
- WKMP-specific features still require custom code
- **Verdict:** Complexity exceeds value

**Approach D (SQLPage Hybrid):**
- 2.5x implementation effort vs. Approach A
- SQLPage maturity/API stability unknown
- DSL learning curve
- UI styling inconsistency
- **Verdict:** Unproven dependency, higher risk

---

## 7. Implementation Guidance (Approach A)

### 7.1 High-Level Architecture

**Module Structure:**
```
wkmp-ai/src/
├── api/
│   └── database_viewer.rs    # NEW: Routes + handlers
├── db/
│   ├── files.rs              # ADD: list_all_files(), count_files()
│   ├── passages.rs           # ADD: list_passages_without_mbid()
│   ├── songs.rs              # ADD: find_passages_by_work_id()
│   └── database_viewer.rs    # NEW: Cross-table queries
└── static/
    ├── database_viewer.html  # NEW: Main UI page
    ├── database_viewer.css   # NEW: Styling
    └── database_viewer.js    # NEW: Table rendering, filtering, searching
```

**Database Connection:**
```rust
// Read-only connection (CRITICAL for safety)
let db_url = format!("sqlite://{}?mode=ro", db_path.display());
let pool = SqlitePool::connect(&db_url).await?;
```

### 7.2 Core Features (MVP)

**Priority 1: Table Viewing**
- Endpoint: `GET /database/tables` → List all tables with row counts
- Endpoint: `GET /database/table/:name` → Paginated rows (limit 100, offset parameter)
- UI: Dropdown to select table, paginated data grid

**Priority 2: Filtered Views**
- Endpoint: `GET /database/passages/without_mbid` → Passages lacking MusicBrainz Recording ID
- Endpoint: `GET /database/files/unlinked` → Audio files with no passages
- UI: Predefined filter buttons

**Priority 3: Search Views**
- Endpoint: `GET /database/search/by_work_id?mbid=<uuid>` → All passages linked to MusicBrainz Work
- Endpoint: `GET /database/search/by_path?query=<string>` → Files matching path pattern
- UI: Search form with query type dropdown

**Priority 4: Saved Searches**
- Storage: Browser localStorage (JSON array of search configs)
- UI: "Save this search" button, favorites dropdown

### 7.3 Testing Strategy

**Unit Tests:**
- Database query functions (`list_all_files()`, `count_passages_without_mbid()`)
- Read-only connection validation

**Integration Tests:**
- API endpoints with test database
- Authentication (timestamp+hash validation)
- Pagination logic

**Manual Testing:**
- Large database performance (1000+ files, 5000+ passages)
- UI responsiveness
- Browser compatibility (Chrome, Firefox, Safari)

### 7.4 Security Considerations

**Read-Only Enforcement:**
- SQLite connection MUST use `?mode=ro`
- Test: Attempt INSERT/UPDATE/DELETE, verify failure

**API Authentication:**
- All endpoints require timestamp+hash
- Test: Request without auth → HTTP 401

**Data Sanitization:**
- SQL injection prevention (sqlx parameterized queries)
- XSS prevention (HTML escaping in UI)

---

## 8. Next Steps

### 8.1 Decision Required

**Stakeholder must decide:**
1. **Approve Approach A (Extend wkmp-ai)** as primary implementation?
2. **If urgent need:** Approve two-phase strategy (Approach E → Approach A)?
3. **Version availability:** Full version only, or include in Lite/Minimal?
4. **Feature priority:** MVP (table viewing + basic filters) or full feature set?

### 8.2 To Proceed with Implementation

**Use `/plan` workflow to create detailed implementation plan:**

```bash
/plan wip/_database_review_analysis.md
```

This will:
- Generate requirements analysis (Phase 2)
- Create acceptance test specifications (Phase 3)
- Develop increment breakdown with tasks (Phase 4)
- Produce traceability matrix (Phase 5)

**Estimated timeline (Approach A, full feature set):**
- Phase 1 (Database queries): 1-2 days
- Phase 2 (API endpoints): 1-2 days
- Phase 3 (UI development): 2-3 days
- Phase 4 (Testing + polish): 1-2 days
- **Total: 5-9 days** (assuming 2-3 hours/day development)

---

## 9. Appendix: Research References

### 9.1 Codebase Research
- wkmp-ai architecture: `/home/sw/Dev/McRhythm/wkmp-ai/src/` (researched via Explore agent)
- wkmp-common utilities: `/home/sw/Dev/McRhythm/wkmp-common/src/` (researched via Explore agent)
- WKMP architectural docs: `CLAUDE.md`, `docs/REQ001-requirements.md`, `docs/SPEC007-api_design.md`

### 9.2 FOSS Tools Research
- **sqlite-web:** github.com/coleifer/sqlite-web (~4.7k stars, Python/Flask, MIT license)
- **SQLPage:** crates.io/crates/sqlpage (Rust, MIT license, SQL-driven web pages)
- **Adminer:** adminer.org (PHP, Apache 2.0/GPL 2, single file)
- **DB Browser for SQLite:** sqlitebrowser.org (~20k stars, C++/Qt, MPL-2.0/GPL-3.0)
- **SQLite Viewer Web App:** sqliteviewer.app (JavaScript/WASM, Apache 2.0, client-side)

### 9.3 WKMP Requirements References
- [REQ-NF-030] through [REQ-NF-037]: Zero-configuration startup (REQ001-requirements.md:256-283)
- [ARCH-OD-010]: On-demand microservices pattern (CLAUDE.md:332-370)
- [API-AUTH-025]: Timestamp+hash authentication protocol (SPEC007-api_design.md:57-179)
- [REQ-NF-050] through [REQ-NF-053]: Health endpoint requirements (REQ001-requirements.md:296-302)

---

**Analysis Complete**
**Document Status:** Ready for stakeholder review and decision
