# PLAN015 Increment 2 Analysis: Zero-Config & Health

**Analysis Date:** 2025-11-01
**Analysis Method:** `/think` Multi-Agent Workflow
**Analyst:** Claude Code
**Specification Source:** wip/PLAN015_database_review_wkmp_dr/00_PLAN_SUMMARY.md

---

## Executive Summary

### Objective

Implement WKMP's zero-configuration startup pattern and health monitoring for wkmp-dr module (Increment 2 of 9).

### Scope

**Deliverables:**
- 4-tier root folder resolution (CLI → ENV → TOML → Default)
- Directory initialization if missing
- Health endpoint (GET /health)
- Port 5725 binding with Axum HTTP server
- Tests: TC-I-NF010-01 through TC-I-NF010-04, TC-I-NF040-01, TC-I-NF050-01

**Success Criteria:**
- Module starts without configuration file
- Responds to health checks within 2 seconds
- All tests pass

### Key Findings

1. **Zero-config utilities already exist** - wkmp_common::config provides RootFolderResolver and RootFolderInitializer
2. **Health endpoint pattern established** - wkmp-ai provides reference implementation
3. **Axum server setup is straightforward** - Similar to wkmp-ai main.rs structure
4. **No authentication required for health endpoint** - Per REQ-DR-NF-040, health check is unauthenticated

### Implementation Status

**Current State (Post-Increment 1):**
- ✅ Database read-only connection working (wkmp-dr/src/db/mod.rs)
- ✅ Table listing functional (wkmp-dr/src/db/tables.rs)
- ✅ Basic main.rs with manual root folder resolution
- ❌ No Axum HTTP server
- ❌ No health endpoint
- ❌ Not using wkmp_common::config utilities

**After Increment 2:**
- ✅ Full zero-config with wkmp_common::config
- ✅ Axum HTTP server on port 5725
- ✅ Health endpoint GET /health
- ✅ 7 integration tests passing

---

## Requirements Analysis

### REQ-DR-NF-010: Zero-Config Startup

**Priority:** P0 (Must Have)
**Type:** Non-Functional
**Description:** Module SHALL implement WKMP's 4-tier root folder resolution without manual configuration.

**4-Tier Priority:**
1. **CLI args:** `--root-folder /path` or `--root /path`
2. **Environment:** `WKMP_ROOT_FOLDER` or `WKMP_ROOT`
3. **TOML config:** `~/.config/wkmp/wkmp-dr.toml` (Linux)
4. **Compiled default:** `~/Music` (Linux/macOS), `%USERPROFILE%\Music` (Windows)

**Implementation Pattern (MANDATORY from wkmp_common):**
```rust
use wkmp_common::config::{RootFolderResolver, RootFolderInitializer};

let resolver = RootFolderResolver::new("database-review");
let root_folder = resolver.resolve();

let initializer = RootFolderInitializer::new(root_folder);
initializer.ensure_directory_exists()?;

let db_path = initializer.database_path();  // root_folder/wkmp.db
```

**Current State:** main.rs has manual resolution (lines 68-90), needs replacement with wkmp_common pattern.

**Dependencies:** None (wkmp_common already available)

**Tests:**
- TC-I-NF010-01: Zero-config via CLI args
- TC-I-NF010-02: Zero-config via ENV vars
- TC-I-NF010-03: Zero-config via TOML config
- TC-I-NF010-04: Zero-config with compiled default

---

### REQ-DR-NF-040: Health Endpoint

**Priority:** P0 (Must Have)
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
- **Does NOT require authentication** (health checks are unauthenticated)

**Reference Implementation:** wkmp-ai/src/api/health.rs:20-28
```rust
pub async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        module: "wkmp-ai".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: 0,  // Optional
    })
}
```

**Adaptation for wkmp-dr:** Use simpler format per PLAN015 spec (no version/uptime fields required).

**Tests:**
- TC-I-NF040-01: Health endpoint response format
- TC-I-NF040-02: Health endpoint performance (<2s)

---

### REQ-DR-NF-050: Port Assignment

**Priority:** P0 (Must Have)
**Type:** Non-Functional (Architecture)
**Description:** Module SHALL bind to port 5725.

**Rationale:** Ports 5720-5724 already allocated:
- 5720: wkmp-ui
- 5721: wkmp-ap
- 5722: wkmp-pd
- 5723: wkmp-ai
- 5724: wkmp-le
- **5725: wkmp-dr** (new)

**Implementation:**
```rust
let listener = tokio::net::TcpListener::bind("127.0.0.1:5725").await?;
info!("wkmp-dr listening on http://127.0.0.1:5725");
```

**Tests:**
- TC-I-NF050-01: Port 5725 binding successful

---

## Existing Patterns to Follow

### Pattern: wkmp-ai Axum Server Setup

**File:** wkmp-ai/src/main.rs:146-151

**Steps:**
1. Create AppState with database pool
2. Build router with health routes
3. Bind TcpListener to port
4. Start Axum server with `axum::serve()`

**Key Code:**
```rust
let state = AppState::new(db_pool, event_bus);
let app = build_router(state);
let listener = tokio::net::TcpListener::bind("127.0.0.1:5723").await?;
axum::serve(listener, app).await?;
```

### Pattern: wkmp-ai Health Endpoint

**File:** wkmp-ai/src/api/health.rs

**Structure:**
1. Health response struct with Serialize
2. Handler function returning Json<HealthResponse>
3. Router builder function

**Adaptation:** Simplify for wkmp-dr (no uptime tracking required).

### Pattern: wkmp_common Zero-Config

**File:** wkmp-common/src/config.rs:186-385

**Usage:**
1. RootFolderResolver::new("module-name")
2. resolver.resolve() → PathBuf
3. RootFolderInitializer::new(root_folder)
4. initializer.ensure_directory_exists()
5. initializer.database_path()

**Already tested** - wkmp-common/tests/config_tests.rs has comprehensive tests.

---

## Components to Implement

### 1. AppState Structure

**Location:** wkmp-dr/src/lib.rs (new file)

**Purpose:** Share database pool across HTTP handlers

**Content:**
```rust
#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
}
```

**Rationale:** Minimal state - only database pool needed (no event bus like wkmp-ai).

---

### 2. Health API Module

**Location:** wkmp-dr/src/api/health.rs (new file)

**Purpose:** Health check endpoint for monitoring

**Components:**
- HealthResponse struct
- health_check() handler
- health_routes() router builder

**Differences from wkmp-ai:**
- Simpler response (no version/uptime)
- No AppState dependency (stateless handler)

---

### 3. API Module

**Location:** wkmp-dr/src/api/mod.rs (new file)

**Purpose:** Export health routes for main router

**Content:**
```rust
pub mod health;
pub use health::health_routes;
```

---

### 4. Router Builder

**Location:** wkmp-dr/src/lib.rs

**Purpose:** Assemble application router

**Function:**
```rust
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .merge(health_routes())
        .with_state(state)
}
```

---

### 5. Updated main.rs

**Location:** wkmp-dr/src/main.rs

**Changes Required:**
1. **Replace manual root folder resolution** (lines 68-90) with wkmp_common pattern
2. **Add Axum server startup** after database connection
3. **Import new modules** (api, lib)

**Structure:**
```rust
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing (existing)

    // Zero-config with wkmp_common (REPLACES existing resolve_root_folder)
    let resolver = RootFolderResolver::new("database-review");
    let root_folder = resolver.resolve();
    let initializer = RootFolderInitializer::new(root_folder);
    initializer.ensure_directory_exists()?;
    let db_path = initializer.database_path();

    // Connect to database (existing)
    let pool = db::connect_readonly(&db_path).await?;

    // Create AppState and router (NEW)
    let state = AppState::new(pool);
    let app = build_router(state);

    // Start server (NEW)
    let listener = tokio::net::TcpListener::bind("127.0.0.1:5725").await?;
    info!("wkmp-dr listening on http://127.0.0.1:5725");
    info!("Health check: http://127.0.0.1:5725/health");
    axum::serve(listener, app).await?;

    Ok(())
}
```

**Remove:** resolve_root_folder() function (replaced by wkmp_common)

---

## Testing Strategy

### Integration Tests

**Test Coverage:** 7 integration tests for Increment 2

**Categories:**
1. **Zero-config (4 tests):** Each tier of priority system
2. **Health endpoint (2 tests):** Response format and performance
3. **Port binding (1 test):** Successful startup on 5725

**Test Approach:**
- Start wkmp-dr with different configurations
- Verify correct root folder resolution
- Make HTTP GET /health request
- Verify response format and timing

**Test Location:** wkmp-dr/tests/integration_test.rs (new file)

---

## Technical Considerations

### 1. Error Handling

**Database Connection Failures:**
- Current: Logs error and returns Err (main.rs:56-59)
- Increment 2: Same behavior (fail fast if database missing)

**Port Binding Failures:**
- TcpListener::bind() returns Result
- Propagate error to main (fail fast if port already in use)

### 2. Logging

**Startup Sequence:**
```
INFO wkmp_dr: wkmp-dr (Database Review) v0.1.0
INFO wkmp_dr: Root folder: /home/user/Music (compiled default)
INFO wkmp_dr: Database path: /home/user/Music/wkmp.db
INFO wkmp_dr: ✓ Connected to database (read-only)
INFO wkmp_dr: wkmp-dr listening on http://127.0.0.1:5725
INFO wkmp_dr: Health check: http://127.0.0.1:5725/health
```

**Follow wkmp-ai pattern** (main.rs:146-148).

### 3. Dependencies

**New Cargo.toml dependencies:** None required (all already present)
- axum: ✅ Already in workspace
- tokio: ✅ Already in workspace
- tower: ✅ Already in workspace
- serde_json: ✅ Already in workspace

**wkmp_common usage:** Already a dependency.

### 4. Module Structure

**Before Increment 2:**
```
wkmp-dr/
└── src/
    ├── main.rs (90 lines)
    └── db/
        ├── mod.rs (76 lines)
        └── tables.rs (168 lines)
```

**After Increment 2:**
```
wkmp-dr/
├── src/
│   ├── main.rs (~100 lines, updated)
│   ├── lib.rs (~50 lines, NEW)
│   ├── db/
│   │   ├── mod.rs
│   │   └── tables.rs
│   └── api/
│       ├── mod.rs (~10 lines, NEW)
│       └── health.rs (~40 lines, NEW)
└── tests/
    └── integration_test.rs (~200 lines, NEW)
```

---

## Risk Assessment

### Implementation Risk: Low

**Rationale:**
1. **Proven patterns:** wkmp-ai provides working reference
2. **Utilities available:** wkmp_common handles complex zero-config logic
3. **Simple HTTP server:** Axum setup is straightforward
4. **No authentication:** Health endpoint is unauthenticated (simplifies Increment 2)

**Failure Modes:**
1. **Zero-config integration bugs** - Probability: Low - Impact: Medium
   - Mitigation: Follow exact wkmp_common pattern, comprehensive tests
2. **Port conflict (5725 already in use)** - Probability: Low - Impact: Low
   - Mitigation: Fail fast with clear error message
3. **Test flakiness** - Probability: Low - Impact: Low
   - Mitigation: Integration tests start/stop server cleanly

**Residual Risk:** Low

### Quality Characteristics

**Maintainability:** High
- Uses standard wkmp_common patterns
- Follows wkmp-ai reference implementation
- Clear module boundaries

**Test Coverage:** High
- 7 integration tests for 3 requirements
- All acceptance criteria covered
- Mirrors PLAN015 test specifications

**Architectural Alignment:** Strong
- Consistent with other WKMP modules
- No deviation from established patterns
- Proper use of wkmp_common utilities

---

## Recommendation

**Proceed with Increment 2 implementation** following wkmp-ai patterns and wkmp_common utilities.

**Key Success Factors:**
1. Use `wkmp_common::config::RootFolderResolver` (don't reinvent)
2. Mirror `wkmp-ai/src/api/health.rs` structure
3. Follow Axum server setup from `wkmp-ai/src/main.rs`
4. Write integration tests for all 3 requirements

**Estimated Effort:** 3-4 hours (per PLAN015)
- Zero-config integration: 1 hour
- Health endpoint + Axum server: 1 hour
- Integration tests: 1-2 hours

---

## Next Steps

**Implementation is ready to proceed.** All requirements are clear, patterns are established, and dependencies are available.

**To begin implementation:**
1. Create new files (lib.rs, api/mod.rs, api/health.rs)
2. Update main.rs with wkmp_common pattern and Axum server
3. Write integration tests
4. Verify all tests pass
5. Manually test health endpoint: `curl http://localhost:5725/health`

**Document Status:** Analysis complete, ready for execution
