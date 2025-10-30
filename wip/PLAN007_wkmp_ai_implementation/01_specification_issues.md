# PLAN007: Specification Issues Analysis

**Source:** [SPEC024-audio_ingest_architecture.md](../../docs/SPEC024-audio_ingest_architecture.md)
**Requirements Analyzed:** 26 (19 P0, 6 P1, 1 P3)
**Analysis Date:** 2025-10-28
**Methodology:** Systematic review per /plan Phase 2 checklist

---

## Executive Summary

**Specification Quality:** ✅ **High** - SPEC024 is well-written and comprehensive

**Issues Found:**
- **CRITICAL:** 0 issues (no blockers to implementation)
- **HIGH:** 3 issues (should resolve before implementation)
- **MEDIUM:** 5 issues (can address during implementation)
- **LOW:** 7 issues (minor clarifications)

**Total:** 15 issues identified across 26 requirements

**Recommendation:** ✅ **Proceed to Phase 3** - No critical gaps, high-priority issues are clarifications not blockers

---

## Issues by Severity

### CRITICAL Issues (Blocks Implementation)

**None identified.** ✅

SPEC024 contains sufficient detail to begin implementation. All P0 requirements have clear inputs, outputs, and behavior specifications.

---

### HIGH Priority Issues (Should Resolve Before Implementation)

#### **ISSUE-H01: Web UI Technology Stack Not Specified**

**Affected Requirements:** AIA-UI-010, AIA-UI-030

**Current State:**
- SPEC024 states "HTML/CSS/JavaScript served by wkmp-ai Axum server"
- Does not specify framework (React, Vue, vanilla JS, etc.)
- Does not specify build tooling (webpack, parcel, none)

**Ambiguity:**
- "vanilla JavaScript" could mean ES5, ES6+, TypeScript
- No guidance on bundling/minification
- No guidance on browser compatibility (modern browsers only? IE11?)

**Impact if Unresolved:**
- Implementation decisions made ad-hoc
- Potential rework if framework/tooling incompatible with requirements

**Recommended Resolution:**
- **Decision (PLAN007):** Vanilla ES6+ JavaScript, no framework, no build step
  - Rationale: Simplicity, no build dependencies, aligns with Decision 2 (client-side Canvas)
  - Browser target: Modern browsers only (Chrome 90+, Firefox 88+, Safari 14+)
  - Specify in implementation plan

**Testability:** ✅ Can test "does UI load and function in target browsers"

**Risk:** ⚠️ Medium - Framework choice affects development speed vs. deployment simplicity

---

#### **ISSUE-H02: Cancellation Mechanism Not Detailed**

**Affected Requirements:** AIA-WF-010 (workflow state machine)

**Current State:**
- Workflow diagram shows "Cancel available at any state → CANCELLED"
- No specification of HOW cancellation is triggered
- No specification of cleanup behavior on cancellation

**Ambiguity:**
- Is there a `/import/cancel` HTTP endpoint?
- Does UI provide "Cancel Import" button?
- What happens to partially imported files? (kept? rolled back?)
- Are background tasks gracefully terminated or forcibly killed?

**Impact if Unresolved:**
- Cancellation feature may be inconsistent or incomplete
- User may not know how to cancel an import
- Partial state cleanup unclear

**Recommended Resolution:**
- Add HTTP endpoint: `DELETE /import/session/{session_id}` (cancels import)
- UI: "Cancel Import" button visible during all import states
- Behavior: Set cancellation flag, wait for current file to complete, transition to CANCELLED state
- Cleanup: Keep already-imported passages (partial success), log cancellation reason

**Testability:** ✅ Can test "cancel during each workflow state, verify graceful termination"

**Risk:** ⚠️ Medium - Cancellation is expected UX feature, absence would frustrate users

---

#### **ISSUE-H03: Session Lifecycle Not Specified**

**Affected Requirements:** AIA-WF-020 (session state persistence)

**Current State:**
- Sessions persisted in-memory (UUID-based)
- No specification of when sessions are cleaned up
- No specification of session expiration

**Ambiguity:**
- How long do completed sessions remain in memory?
- Are failed/cancelled sessions cleaned up differently?
- What if user navigates away mid-import? (session orphaned?)
- Memory leak risk if sessions accumulate indefinitely

**Impact if Unresolved:**
- Memory leak from accumulated sessions
- `/import/status` may return stale data
- No guidance for when to stop polling after import complete

**Recommended Resolution:**
- Session TTL: 1 hour after completion/failure/cancellation (then purged)
- Active sessions (SCANNING, EXTRACTING, etc.): Never expire while running
- Orphaned sessions: If no status checks for 10 minutes, mark as ABANDONED, clean up after 1 hour
- Document in IMPL008 (API specification)

**Testability:** ✅ Can test "session cleanup after TTL expires"

**Risk:** ⚠️ Medium - Memory leak potential on long-running wkmp-ai instances

---

### MEDIUM Priority Issues (Address During Implementation)

#### **ISSUE-M01: Retry Logic for External APIs Not Specified**

**Affected Requirements:** AIA-COMP-010 (musicbrainz_client, acoustid_client, essentia_runner)

**Current State:**
- SPEC024 mentions rate limiting (1 req/s for MusicBrainz)
- Does not specify retry behavior on transient failures (503, timeout, network error)

**Ambiguity:**
- How many retries before giving up?
- Exponential backoff or fixed interval?
- Which HTTP status codes trigger retry? (503? 429? 500?)

**Impact if Unresolved:**
- Transient network failures cause unnecessary import failures
- Retry strategy implemented inconsistently across API clients

**Recommended Resolution:**
- Retry strategy (all external APIs):
  - Transient errors (503, 429, timeout): Retry up to 3 times
  - Exponential backoff: 1s, 2s, 4s
  - Permanent errors (404, 401, 400): No retry, log error
  - Document in IMPL011, IMPL012

**Testability:** ✅ Can test "simulate 503 response, verify 3 retries with backoff"

**Risk:** ✅ Low - Graceful degradation already specified (continue on API failure)

---

#### **ISSUE-M02: Essentia Error Handling Not Detailed**

**Affected Requirements:** AIA-COMP-010 (essentia_runner)

**Current State:**
- Dependencies map specifies Essentia required (Decision 1)
- SPEC024 mentions Essentia subprocess integration
- Error handling if Essentia binary missing or crashes not detailed

**Ambiguity:**
- What error message if Essentia not in PATH?
- What if Essentia crashes during analysis?
- What if Essentia JSON output is malformed?

**Impact if Unresolved:**
- Poor user experience if Essentia unavailable
- Unclear whether import should fail or continue without Musical Flavor

**Recommended Resolution:**
- Essentia binary detection at wkmp-ai startup:
  - If missing: Log ERROR, display banner in UI: "Essentia required for Musical Flavor extraction. Install: [link]"
  - Prevent import from starting (fail fast)
- Essentia crash during analysis:
  - Log warning, retry once
  - If still fails: Set Musical Flavor to NULL, log error, continue import
- Document in IMPL009 (amplitude analyzer might call Essentia)

**Testability:** ✅ Can test "Essentia not in PATH → clear error message with installation link"

**Risk:** ⚠️ Medium - User frustration if error messages unclear (Decision 1 requires Essentia)

---

#### **ISSUE-M03: Waveform Data Format Not Specified**

**Affected Requirements:** AIA-UI-010 (waveform editor route)

**Current State:**
- SPEC024 mentions "waveform editor for passage boundaries"
- IMPL005 describes "waveform display of audio file"
- Does not specify waveform data format sent to browser

**Ambiguity:**
- Peak data? RMS data? Raw PCM samples?
- How much downsampling? (e.g., 1 sample per 1000 samples?)
- JSON format for waveform data?
- How is waveform generated? (server-side or client-side from PCM?)

**Impact if Unresolved:**
- Waveform generation approach unclear
- Data size may be too large for browser rendering

**Recommended Resolution (Per Decision 2: Client-side Canvas):**
- Server sends downsampled peak/RMS data in JSON:
  ```json
  {
    "duration_seconds": 180.5,
    "sample_rate": 44100,
    "peaks": [0.1, 0.3, 0.5, ...],  // One value per 1000 samples
    "rms": [0.05, 0.15, 0.25, ...]  // Same granularity
  }
  ```
- Client renders using Canvas API
- Document in IMPL008 (API endpoint spec)

**Testability:** ✅ Can test "waveform renders correctly for 3-minute audio file"

**Risk:** ✅ Low - Implementation detail, multiple viable approaches

---

#### **ISSUE-M04: Parallelism Configuration Not Specified**

**Affected Requirements:** AIA-ASYNC-020 (parallel file processing)

**Current State:**
- Default parallelism: 4 concurrent workers
- States "user-configurable via `import_parallelism` parameter"
- Does not specify where this parameter is configured

**Ambiguity:**
- Is `import_parallelism` in settings table?
- Is it a per-import parameter (POST /import/start)?
- What are valid values? (1-32? 1-cpu_count?)

**Impact if Unresolved:**
- Parameter location unclear
- Validation bounds undefined

**Recommended Resolution:**
- Location: `settings` table, key="import_parallelism", default=4
- Valid range: 1-16 (cap to prevent resource exhaustion)
- Configurable via wkmp-ui preferences (global setting)
- Document in IMPL010 (parameter management)

**Testability:** ✅ Can test "set parallelism=2, verify only 2 files processed concurrently"

**Risk:** ✅ Low - Default value (4) is reasonable, configuration optional

---

#### **ISSUE-M05: API Key Storage Location Ambiguous**

**Affected Requirements:** AIA-SEC-020 (API key secure storage)

**Current State:**
- "API keys stored in credentials table"
- AIA-SEC-020 mentions "Environment variable or config file"
- Contradiction: database vs. environment variable

**Ambiguity:**
- Which takes precedence? (database or environment variable?)
- Is `credentials` table defined in IMPL001?

**Verification:**
- Check IMPL001-database_schema.md for `credentials` table definition

**Impact if Unresolved:**
- Implementation may use wrong storage location

**Recommended Resolution:**
- Priority: Environment variable > database > hardcoded default (for development only)
- `credentials` table for user-configurable keys (via wkmp-ui preferences)
- Document in IMPL001 (database schema) and IMPL010 (parameter management)

**Testability:** ✅ Can test "set ACOUSTID_API_KEY env var, verify used before database value"

**Risk:** ✅ Low - Security best practice clear (no hardcoding)

---

### LOW Priority Issues (Minor Clarifications)

#### **ISSUE-L01: Health Endpoint Response Format Not Specified**

**Affected Requirements:** AIA-UI-020 (wkmp-ui health check)

**Current State:**
- wkmp-ui checks `/health` endpoint
- Response format not specified

**Recommended Resolution:**
- Standard health check response:
  ```json
  {
    "status": "ok",
    "version": "1.0.0",
    "dependencies": {
      "essentia": "available",  // or "missing"
      "database": "ok"
    }
  }
  ```
- Document in IMPL008 (API spec)

**Testability:** ✅ Trivial

**Risk:** ✅ Minimal - Straightforward implementation

---

#### **ISSUE-L02: SSE Reconnection Behavior Not Detailed**

**Affected Requirements:** AIA-SSE-010 (Server-Sent Events)

**Current State:**
- "Client may disconnect/reconnect, missed events available via `/import/status` polling"
- Reconnection strategy not specified

**Recommended Resolution:**
- SSE EventSource API auto-reconnects (browser default)
- No custom reconnection logic needed
- Client reads `/import/status` on reconnect to catch up

**Testability:** ✅ Can test "disconnect during import, reconnect, verify catch-up"

**Risk:** ✅ Minimal - Browser handles reconnection

---

#### **ISSUE-L03: Database Transaction Scope Unclear**

**Affected Requirements:** AIA-DB-010 (database writes), AIA-PERF-020 (batch inserts)

**Current State:**
- "Transaction handling for atomic operations"
- "Batch inserts (100 at a time)"
- Does not specify transaction boundaries

**Recommended Resolution:**
- Per-file transaction: All writes for single file in one transaction (passages, songs, etc.)
- Batch inserts: Commit every 100 passages (not per-passage)
- Rollback behavior: If file fails, roll back its transaction, continue with next file
- Document in IMPL014 (database queries)

**Testability:** ✅ Can test "corrupt file mid-batch, verify previous files committed"

**Risk:** ✅ Minimal - Implementation detail

---

#### **ISSUE-L04: Silence Detection Parameters Not Fully Quantified**

**Affected Requirements:** AIA-INT-020 (IMPL005 segmentation workflow)

**Current State:**
- IMPL005 specifies silence threshold per source media type (CD: -80dB, Vinyl: -60dB, etc.)
- Minimum silence duration: 0.5 seconds
- Does not specify RMS window size for silence detection

**Recommended Resolution:**
- RMS window size: 100ms (typical for silence detection)
- Document in IMPL005 or SPEC025 (amplitude analysis)

**Testability:** ✅ Can test "silence detection with 100ms window"

**Risk:** ✅ Minimal - Standard audio processing value

---

#### **ISSUE-L05: MusicBrainz Rate Limiter Implementation Not Specified**

**Affected Requirements:** AIA-PERF-020 (rate limiting 1 req/s)

**Current State:**
- "Rate limiting (1 req/s for MusicBrainz per API terms)"
- Does not specify implementation approach

**Recommended Resolution:**
- Token bucket algorithm: 1 token/second, burst of 1
- Implemented via `tokio::time::sleep` between requests
- Document in IMPL011 (MusicBrainz client)

**Testability:** ✅ Can test "10 requests take ≥10 seconds"

**Risk:** ✅ Minimal - Standard rate limiting pattern

---

#### **ISSUE-L06: Error Codes Not Enumerated**

**Affected Requirements:** AIA-ERR-020 (error reporting)

**Current State:**
- "Error code (e.g., `DECODE_ERROR`, `MBID_LOOKUP_FAILED`)"
- Full list of error codes not provided

**Recommended Resolution:**
- Enumerate all error codes in IMPL008 or separate error code registry
- Examples:
  - `DECODE_ERROR` - Audio decoding failed
  - `MBID_LOOKUP_FAILED` - MusicBrainz lookup failed
  - `FINGERPRINT_ERROR` - Chromaprint generation failed
  - `ESSENTIA_MISSING` - Essentia binary not found
  - `ESSENTIA_CRASH` - Essentia subprocess crashed
  - `DB_WRITE_ERROR` - Database write failed
  - (etc.)

**Testability:** ✅ Each error code testable independently

**Risk:** ✅ Minimal - Can enumerate during implementation

---

#### **ISSUE-L07: Browser Compatibility Not Specified**

**Affected Requirements:** AIA-UI-010 (web UI)

**Current State:**
- "HTML/CSS/JavaScript served by wkmp-ai Axum server"
- No browser compatibility specified

**Recommended Resolution (Per ISSUE-H01):**
- Target modern browsers only: Chrome 90+, Firefox 88+, Safari 14+ (2021+)
- Rationale: Simplifies development, Pi Zero2W users likely have recent browsers
- No IE11 support (EOL 2022)
- Document in README

**Testability:** ✅ Manual testing on target browsers

**Risk:** ✅ Minimal - Modern browser baseline reasonable

---

## Issues by Requirement

| Req ID | Issues | Severity | Resolution |
|--------|--------|----------|------------|
| AIA-OV-010 | None | - | ✅ Complete |
| AIA-MS-010 | None | - | ✅ Complete |
| AIA-UI-010 | ISSUE-H01, ISSUE-L07 | HIGH, LOW | Specify vanilla ES6+ JS, modern browsers |
| AIA-UI-020 | ISSUE-L01 | LOW | Specify health endpoint JSON format |
| AIA-UI-030 | None (covered by ISSUE-H01) | - | ✅ Complete |
| AIA-DB-010 | ISSUE-L03 | LOW | Specify transaction boundaries |
| AIA-COMP-010 | ISSUE-M01, ISSUE-M02 | MEDIUM | Retry logic, Essentia error handling |
| AIA-WF-010 | ISSUE-H02 | HIGH | Specify cancellation mechanism |
| AIA-WF-020 | ISSUE-H03 | HIGH | Specify session lifecycle/TTL |
| AIA-ASYNC-010 | None | - | ✅ Complete |
| AIA-ASYNC-020 | ISSUE-M04 | MEDIUM | Specify parallelism configuration location |
| AIA-SSE-010 | ISSUE-L02 | LOW | Clarify reconnection behavior |
| AIA-POLL-010 | None | - | ✅ Complete |
| AIA-INT-010 | None | - | ✅ Complete |
| AIA-INT-020 | ISSUE-L04, ISSUE-M03 | LOW, MEDIUM | RMS window size, waveform data format |
| AIA-INT-030 | None | - | ✅ Complete |
| AIA-ERR-010 | None | - | ✅ Complete |
| AIA-ERR-020 | ISSUE-L06 | LOW | Enumerate error codes |
| AIA-PERF-010 | None | - | ✅ Complete |
| AIA-PERF-020 | ISSUE-L05 | LOW | Specify rate limiter implementation |
| AIA-SEC-010 | None | - | ✅ Complete |
| AIA-SEC-020 | ISSUE-M05 | MEDIUM | Clarify API key storage priority (env var > db) |
| AIA-TEST-010 | None | - | ✅ Complete |
| AIA-TEST-020 | None | - | ✅ Complete |
| AIA-TEST-030 | None | - | ✅ Complete |
| AIA-FUTURE-010 | None | - | ✅ Complete (out of scope) |

---

## Ambiguity Analysis

### Vague Language Detected

**None significant.** SPEC024 uses precise language throughout:
- "SHALL", "MUST" for requirements
- Quantified values where needed (1 req/s, 4 workers, 28,224,000 ticks/sec)
- Specific error severity levels (Warning, Skip File, Critical)

### Undefined Terms

**None significant.** All domain terms defined:
- Passage, Song, Artist, Work (REQ002-entity_definitions.md)
- Ticks (IMPL001-database_schema.md)
- Musical Flavor (SPEC008)
- Essentia, Chromaprint, MusicBrainz (external tools, widely known)

---

## Consistency Check

### Conflicts Between Requirements

**No conflicts detected.** ✅

All requirements are internally consistent:
- State machine transitions well-defined
- Database tables align with workflow
- Performance targets realistic for hardware constraints
- Error handling consistent across components

### Cross-Document Conflicts

**No conflicts detected.** ✅

SPEC024 aligns with:
- SPEC008 (Library Management workflows)
- IMPL005 (Audio File Segmentation)
- IMPL001 (Database schema)
- IMPL008-014 (Implementation specs created in PLAN004)

---

## Testability Assessment

**All 26 requirements are testable.** ✅

For each requirement, a pass/fail test can be defined:
- Functional requirements: Input → expected output verification
- Performance requirements: Measured timing/throughput
- Integration requirements: Cross-module communication verified
- Security requirements: Input validation boundary testing

**Test Coverage Achievable:** >80% per AIA-TEST-010

---

## Dependency Validation

**All dependencies exist and are accessible.** ✅

- Internal modules: wkmp-common, wkmp-ui (exist)
- Rust crates: tokio, axum, symphonia, lofty, reqwest, chromaprint-sys-next (all available)
- External APIs: MusicBrainz, AcoustID (operational), AcousticBrainz (shut down, Essentia fallback approved)
- Database tables: Defined in IMPL001
- External binaries: Essentia (required per Decision 1, installation documented)

---

## /think Analysis Trigger Assessment

**Is /think needed for this plan?**

**Decision:** ❌ **No** - /think not needed

**Rationale:**
- Only 15 issues found, 0 CRITICAL
- 3 HIGH issues are clarifications, not complex unknowns
- 5 MEDIUM issues are implementation details
- 7 LOW issues are trivial clarifications
- No novel/risky technical elements requiring deep analysis
- Architecture well-defined in SPEC024 (created in PLAN004)

**Threshold for /think trigger (per /plan workflow):**
- 5+ Critical issues, OR
- 10+ High issues, OR
- Unclear architecture/approach, OR
- Novel/risky technical elements

**Current State:** 0 Critical, 3 High, 5 Medium, 7 Low - **Below threshold**

---

## Recommendations

### Before Phase 3 (Acceptance Test Definition)

**Resolve HIGH priority issues:**
1. **ISSUE-H01:** Confirm vanilla ES6+ JavaScript, no framework (per Decision 2)
2. **ISSUE-H02:** Add cancellation specification to IMPL008
3. **ISSUE-H03:** Add session lifecycle specification to IMPL008

**Action:** Update IMPL008-audio_ingest_api.md (or create supplement) with:
- Cancellation endpoint: `DELETE /import/session/{session_id}`
- Session TTL: 1 hour after completion, 10 min inactivity timeout
- Web UI technology: Vanilla ES6+ JS, modern browsers only

### During Implementation

**Address MEDIUM priority issues as encountered:**
- Retry logic (ISSUE-M01)
- Essentia error handling (ISSUE-M02)
- Waveform data format (ISSUE-M03)
- Parallelism configuration (ISSUE-M04)
- API key storage priority (ISSUE-M05)

**Address LOW priority issues opportunistically:**
- Health endpoint format (ISSUE-L01)
- SSE reconnection (ISSUE-L02)
- Transaction scope (ISSUE-L03)
- Silence detection window (ISSUE-L04)
- Rate limiter implementation (ISSUE-L05)
- Error code enumeration (ISSUE-L06)
- Browser compatibility (ISSUE-L07)

### Approval to Proceed

✅ **Phase 2 Complete - Approved to proceed to Phase 3**

**Justification:**
- Specification quality is high (no critical blockers)
- HIGH issues are clarifications, easily resolved in planning
- MEDIUM/LOW issues are implementation details, don't block test definition
- All requirements testable (100% coverage achievable)

---

**End of Specification Issues Analysis**
