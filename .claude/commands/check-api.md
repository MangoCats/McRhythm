# API Contract Validator

**Purpose:** Validate API implementation against SPEC007 API design specifications across all WKMP microservices

**Task:** Verify that implemented HTTP endpoints, request/response types, and error handling match documented API contracts.

---

## Instructions

You are validating API contract compliance across the 5 WKMP microservices. Compare actual Axum route implementations against SPEC007-api_design.md specifications.

---

## Background: WKMP Microservices APIs

**5 HTTP Microservices:**
1. **wkmp-ap** (Audio Player) - Port 5721
2. **wkmp-ui** (User Interface) - Port 5720
3. **wkmp-pd** (Program Director) - Port 5722
4. **wkmp-ai** (Audio Ingest) - Port 5723
5. **wkmp-le** (Lyric Editor) - Port 5724

**Communication:**
- HTTP REST APIs for inter-service calls
- Server-Sent Events (SSE) for real-time updates
- JSON request/response payloads

**API Design Specification:**
- Documented in `docs/SPEC007-api_design.md`
- Defines endpoints, methods, request/response schemas, error codes

---

## Execution Steps

### Step 1: Extract API Specification

Read SPEC007-api_design.md and catalog all documented endpoints:

**For each API endpoint in spec, extract:**
- Service name (wkmp-ap, wkmp-ui, etc.)
- HTTP method (GET, POST, PUT, DELETE, PATCH)
- Path pattern (e.g., `/api/v1/passages/:id`)
- Request body schema (if applicable)
- Response body schema
- Possible status codes (200, 400, 404, 500, etc.)
- Authentication requirements
- SSE endpoints (special handling)

**Output:** Structured API specification catalog

Example:
```
Service: wkmp-ap
Endpoint: POST /api/v1/queue/enqueue
Request: { passage_id: UUID, fade_in_curve: String, ... }
Response: { queue_position: u32, passage: Passage }
Status codes: 200 (success), 400 (invalid request), 404 (passage not found)
Auth: Required
```

---

### Step 2: Scan Implemented Routes

For each microservice, find all Axum route definitions:

**Search patterns:**
- Axum router setup: `.route("/path", METHOD(handler))`
- Route method helpers: `.get(handler)`, `.post(handler)`, `.put(handler)`, `.delete(handler)`
- Router merge: `Router::new().route(...).merge(...)`
- Nested routes: `.nest("/api/v1", routes())`

**Files to scan:**
- `wkmp-ap/src/**/*.rs` (likely `main.rs`, `routes.rs`, `handlers.rs`)
- `wkmp-ui/src/**/*.rs`
- `wkmp-pd/src/**/*.rs`
- `wkmp-ai/src/**/*.rs`
- `wkmp-le/src/**/*.rs`

**For each route found, extract:**
- Service name
- HTTP method
- Path pattern
- Handler function name
- File location (file:line)

**Output:** Catalog of all implemented routes

---

### Step 3: Extract Handler Signatures

For each handler function, analyze implementation:

**Find handler function definitions:**
```rust
async fn enqueue_passage(
    State(state): State<AppState>,
    Json(req): Json<EnqueueRequest>,
) -> Result<Json<EnqueueResponse>, ApiError>
```

**Extract:**
- Request type (from `Json<T>` extractor)
- Response type (from return type)
- Error type (from Result)
- Extractors used (State, Path, Query, Json, etc.)

**Cross-reference types:**
- Find struct definitions for request/response types
- Extract field names and types
- Check for serde derives (`#[derive(Deserialize)]`, `#[derive(Serialize)]`)

---

### Step 4: Compare Specification vs Implementation

For each endpoint in SPEC007:

**Check 1: Endpoint exists**
- ✅ Route implemented
- ❌ Route missing in code
- ⚠️ Route implemented but not in spec

**Check 2: HTTP method matches**
- ✅ Spec says POST, code uses POST
- ❌ Spec says POST, code uses GET

**Check 3: Path pattern matches**
- ✅ Exact match: `/api/v1/passages/:id`
- ⚠️ Close match: `/api/v1/passage/:id` (typo?)
- ❌ Completely different path

**Check 4: Request schema matches**
- ✅ All required fields present with correct types
- ❌ Required field missing
- ⚠️ Extra field in implementation (not in spec)
- ❌ Field type mismatch (spec: String, impl: u32)

**Check 5: Response schema matches**
- Same as request schema checks

**Check 6: Status codes documented**
- ✅ Handler returns documented status codes
- ⚠️ Handler returns undocumented status codes
- ❌ Spec promises 404, but handler never returns it

---

### Step 5: Validate Error Handling

Check error response consistency:

**Scan for error type definitions:**
```rust
enum ApiError {
    NotFound(String),
    BadRequest(String),
    InternalError(String),
}
```

**Validate:**
- All error variants map to appropriate HTTP status codes
- Error responses have consistent JSON schema
- Errors documented in SPEC007

**Common patterns:**
- 400 Bad Request → validation errors
- 404 Not Found → resource doesn't exist
- 500 Internal Server Error → unexpected failures

---

### Step 6: Check Authentication/Authorization

If SPEC007 specifies auth requirements:

**Search for auth middleware:**
- `require_auth` middleware
- `Auth` extractor in handlers
- JWT/session validation

**Validate:**
- Protected endpoints have auth middleware
- Public endpoints don't require auth
- Auth scheme matches spec (bearer token, session, etc.)

---

### Step 7: Validate SSE Endpoints

For Server-Sent Events endpoints:

**Spec check:**
- Path: `/api/v1/events` or similar
- Method: GET
- Response: `text/event-stream`

**Implementation check:**
- Handler returns `Sse<impl Stream>`
- Event format matches spec (data:, event:, id: fields)
- Event types documented

---

## Output Format

Generate validation report: `wip/api_contract_validation_YYYY-MM-DD.md`

### Executive Summary

```
🔌 WKMP API Contract Validation Report

Services scanned: 5
Endpoints in spec: 42
Endpoints implemented: 45

✅ Fully compliant: 38 (90%)
⚠️  Minor issues: 4 (10%)
❌ Contract violations: 3 (7%)
⚠️  Undocumented endpoints: 3 (7%)

Overall compliance: 90% (38/42)
```

### Contract Violations (Must Fix)

```
❌ wkmp-ap: POST /api/v1/queue/enqueue
   Spec: wkmp-ap/src/handlers/queue.rs:45
   Issue: Response missing field `estimated_start_time`
   Spec requires: { queue_position: u32, passage: Passage, estimated_start_time: f64 }
   Implementation: { queue_position: u32, passage: Passage }
   Severity: HIGH - Breaking change for clients
   Action: Add `estimated_start_time` field to EnqueueResponse struct

❌ wkmp-pd: GET /api/v1/select/next
   Spec: wkmp-pd/src/handlers/selection.rs:123
   Issue: Response type mismatch
   Spec requires: { passage_id: UUID, score: f64 }
   Implementation: { passage: Passage }
   Severity: HIGH - Incompatible response schema
   Action: Change response to match spec or update SPEC007

❌ wkmp-ui: DELETE /api/v1/passages/:id
   Spec: Endpoint documented in SPEC007:234
   Issue: Route not implemented
   Severity: MEDIUM - Missing functionality
   Action: Implement DELETE handler in wkmp-ui
```

### Minor Issues (Review Recommended)

```
⚠️  wkmp-ap: GET /api/v1/queue/current
   Spec: wkmp-ap/src/handlers/queue.rs:78
   Issue: Extra field in response
   Spec requires: { passage: Passage }
   Implementation: { passage: Passage, next_passage: Option<Passage> }
   Severity: LOW - Backward compatible (extra field)
   Action: Update SPEC007 to document `next_passage` field

⚠️  wkmp-ai: POST /api/v1/import/file
   Spec: wkmp-ai/src/handlers/import.rs:234
   Issue: Undocumented status code 409 (Conflict)
   Spec documents: 200, 400, 500
   Implementation: Also returns 409 for duplicate files
   Severity: LOW - Client should handle gracefully
   Action: Document 409 status code in SPEC007
```

### Undocumented Endpoints

```
⚠️  wkmp-ui: GET /api/v1/health
   Implementation: wkmp-ui/src/handlers/health.rs:12
   Issue: Not documented in SPEC007
   Response: { status: String, uptime: u64 }
   Severity: LOW - Health check endpoint
   Action: Add to SPEC007 or document as internal-only

⚠️  wkmp-ap: GET /api/v1/debug/buffer-stats
   Implementation: wkmp-ap/src/handlers/debug.rs:45
   Issue: Not documented in SPEC007
   Severity: LOW - Debug endpoint (likely not for production)
   Action: Document as debug-only in SPEC007 or remove from production builds
```

### Compliance by Service

```
| Service | Endpoints (Spec) | Implemented | Compliant | Issues |
|---------|------------------|-------------|-----------|--------|
| wkmp-ap | 12               | 13          | 10 (83%)  | 2      |
| wkmp-ui | 15               | 16          | 14 (93%)  | 1      |
| wkmp-pd | 6                | 6           | 5 (83%)   | 1      |
| wkmp-ai | 5                | 5           | 5 (100%)  | 0      |
| wkmp-le | 4                | 5           | 4 (100%)  | 1 (undoc) |
| Total   | 42               | 45          | 38 (90%)  | 5      |
```

### Detailed Endpoint Comparison

Table format (excerpt):
```
| Service | Method | Path | Spec | Impl | Status |
|---------|--------|------|------|------|--------|
| wkmp-ap | POST | /api/v1/queue/enqueue | ✅ | ✅ | ❌ Response mismatch |
| wkmp-ap | GET | /api/v1/queue/current | ✅ | ✅ | ⚠️ Extra field |
| wkmp-ap | GET | /api/v1/queue/list | ✅ | ✅ | ✅ Compliant |
| wkmp-pd | GET | /api/v1/select/next | ✅ | ✅ | ❌ Schema mismatch |
| wkmp-ui | DELETE | /api/v1/passages/:id | ✅ | ❌ | ❌ Not implemented |
```

### Actionable Recommendations

**Immediate (High Priority):**
1. Fix response schema mismatch: wkmp-ap `/queue/enqueue` (add `estimated_start_time`)
2. Fix response schema mismatch: wkmp-pd `/select/next` (align spec and impl)
3. Implement missing endpoint: wkmp-ui `DELETE /passages/:id`

**Short-term (Medium Priority):**
1. Document extra field in wkmp-ap `/queue/current` response
2. Document 409 status code for wkmp-ai `/import/file`
3. Document or remove undocumented endpoints (health, debug)

**Long-term (Best Practices):**
1. Establish contract-first development: Update SPEC007 before changing APIs
2. Add contract testing: Generate tests from SPEC007 automatically
3. Version API endpoints: Use `/api/v1/` prefix consistently
4. Consider OpenAPI/Swagger generation from Rust types

---

## Display to User

Show:
- Executive summary
- All contract violations (full list)
- Compliance by service table
- Link to full report

Example:
```
Found 3 contract violations across 42 endpoints (90% compliant):
1. wkmp-ap POST /queue/enqueue - Response missing field
2. wkmp-pd GET /select/next - Schema mismatch
3. wkmp-ui DELETE /passages/:id - Not implemented

See wip/api_contract_validation_2025-10-28.md for full report.
```

---

## Advanced Features

### Type-Level Validation

If time permits, validate Rust type definitions:

**Check serde attributes:**
```rust
#[derive(Serialize, Deserialize)]
struct EnqueueRequest {
    #[serde(rename = "passageId")]  // Matches spec: camelCase
    passage_id: Uuid,
}
```

**Validate:**
- Field rename attributes match spec naming convention
- Required vs optional fields (`Option<T>`)
- Default values match spec

### Version Detection

Check for API versioning:
- `/api/v1/` vs `/api/v2/`
- Document which spec version each implementation matches

### Breaking Change Detection

Compare against previous validation run:
- New contract violations introduced
- Fixed violations
- New endpoints added

---

## Performance Optimization

**Efficient scanning:**
- Use Grep for route definitions first (fast)
- Read only files with route definitions (targeted)
- Parse handler signatures on-demand
- Cache struct definitions to avoid re-parsing

**Parallel execution:**
- Scan all 5 services in parallel
- Grep operations in parallel per service

---

## Error Handling

**SPEC007 not found:**
- Report error: "Cannot validate without SPEC007-api_design.md"
- Suggest: Create SPEC007 from current implementation

**No routes found:**
- Report: "No Axum routes found in [service]"
- Check: Is service implemented yet?

**Ambiguous route patterns:**
- Warn: "Multiple routes match pattern /api/v1/passages/:id"
- List all matches for manual review

---

## Integration with Development Workflow

**Before commits:**
- Run `/check-api` to ensure no contract regressions

**During API changes:**
1. Update SPEC007 first
2. Implement changes in code
3. Run `/check-api` to verify compliance
4. Fix violations before committing

**During code review:**
- Include API compliance report
- Reviewer checks spec alignment

---

## Success Criteria

✅ SPEC007 API specifications extracted
✅ All 5 microservices scanned for routes
✅ Handler signatures analyzed
✅ Request/response schemas compared
✅ Contract violations identified with severity
✅ Undocumented endpoints flagged
✅ Compliance percentage calculated
✅ Actionable recommendations provided
✅ Comprehensive report generated

---

**Expected runtime:** 45-120 seconds (depends on codebase size)
**Output file:** `wip/api_contract_validation_YYYY-MM-DD.md`
**Frequency:** Run before commits that modify APIs, weekly for continuous monitoring
