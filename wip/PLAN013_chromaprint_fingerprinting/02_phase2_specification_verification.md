# PLAN013 Phase 2: Specification Completeness Verification

**Specification:** docs/IMPL012-acoustid_client.md
**Date:** 2025-10-30
**Status:** Phase 2 Complete

---

## Executive Summary

**Phase 2 Goal:** Investigate specification issues identified in Phase 1 and verify completeness.

**Critical Findings:**
1. **chromaprint-sys-next API verified** - Low-level FFI bindings, requires unsafe code
2. **Database caching NOT IMPLEMENTED** - Specification calls for caching, current code lacks it
3. **Database schema pattern confirmed** - Use wkmp-common/src/db/init.rs pattern
4. **AcoustIDClient partially implemented** - Has HTTP/rate limiting, missing caching layer

**Overall Assessment:** Specification is MOSTLY complete but has discrepancies with current implementation. Caching layer (REQ-CA-010 through REQ-CA-030) is specified but not implemented.

---

## Issue Resolution

### ISSUE 1: API Key Source Discrepancy ✅ RESOLVED

**Finding:** Specification says environment variable, current implementation uses database.

**Investigation:**
- Checked wkmp-ai/src/api/import_workflow.rs:209-228
- Confirmed: Loads from database via `get_acoustid_api_key(&state.db)`
- Falls back to None (with warning) if database load fails

**Resolution:** Specification should be updated to reflect database-first approach:

```markdown
## API Key Configuration **[CORRECTED]**

**Primary Source:** Database settings table (key: `acoustid_api_key`)
**Fallback:** Environment variable `ACOUSTID_API_KEY` (for testing/CI)
**TOML Sync:** Backup only (automatically synced from database)

**Loading Pattern:**
```rust
// Load from database (authoritative source)
let api_key = crate::db::settings::get_acoustid_api_key(&db).await?;

// Environment variable fallback for testing
let api_key = api_key.or_else(|| std::env::var("ACOUSTID_API_KEY").ok());
```

**Rationale:** Follows WKMP database-first configuration principle per REQ-NF-030 through REQ-NF-037.
```

**Priority:** Medium (documentation correction)
**Action:** Update IMPL012 specification section "API Key Configuration" (lines 615-621)

---

### ISSUE 2: Crate Name Ambiguity ✅ RESOLVED

**Finding:** Specification shows `use chromaprint::{Context, Algorithm}` but crate is `chromaprint-sys-next`.

**Investigation:**
- Ran `cargo info chromaprint-sys-next`
- Documentation: https://docs.rs/chromaprint-sys-next/1.6.0
- Fetched API documentation

**Findings:**
- chromaprint-sys-next is **low-level FFI bindings** to Chromaprint C library
- No high-level Rust wrapper exists (no `Context` or `Algorithm` types)
- Exposes C functions directly: `chromaprint_new`, `chromaprint_start`, `chromaprint_feed`, etc.
- Requires **unsafe code** for all calls

**Resolution:** Specification example code is INCORRECT. Should be:

```rust
use chromaprint_sys_next::*;  // Low-level C bindings

fn generate_fingerprint(&self, samples: &[f32]) -> Result<String, FingerprintError> {
    unsafe {
        // Allocate context
        let ctx = chromaprint_new(CHROMAPRINT_ALGORITHM_TEST2);
        if ctx.is_null() {
            return Err(FingerprintError::ChromaprintError("Context creation failed".to_string()));
        }

        // Start fingerprinting
        let ret = chromaprint_start(ctx, self.target_sample_rate as i32, 1);
        if ret != 1 {
            chromaprint_free(ctx);
            return Err(FingerprintError::ChromaprintError("Start failed".to_string()));
        }

        // Convert f32 to i16
        let samples_i16: Vec<i16> = samples.iter()
            .map(|&s| (s * 32767.0).clamp(-32768.0, 32767.0) as i16)
            .collect();

        // Feed samples
        let ret = chromaprint_feed(ctx, samples_i16.as_ptr(), samples_i16.len() as i32);
        if ret != 1 {
            chromaprint_free(ctx);
            return Err(FingerprintError::ChromaprintError("Feed failed".to_string()));
        }

        // Finish
        let ret = chromaprint_finish(ctx);
        if ret != 1 {
            chromaprint_free(ctx);
            return Err(FingerprintError::ChromaprintError("Finish failed".to_string()));
        }

        // Get fingerprint string
        let mut fp_ptr: *mut i8 = std::ptr::null_mut();
        let ret = chromaprint_get_fingerprint(ctx, &mut fp_ptr);
        if ret != 1 || fp_ptr.is_null() {
            chromaprint_free(ctx);
            return Err(FingerprintError::ChromaprintError("Get fingerprint failed".to_string()));
        }

        // Convert C string to Rust String
        let c_str = std::ffi::CStr::from_ptr(fp_ptr);
        let fingerprint = c_str.to_str()
            .map_err(|e| FingerprintError::ChromaprintError(e.to_string()))?
            .to_string();

        // Free resources
        chromaprint_dealloc(fp_ptr as *mut std::ffi::c_void);
        chromaprint_free(ctx);

        Ok(fingerprint)
    }
}
```

**Priority:** CRITICAL (implementation blocker - specification code won't compile)
**Action:** Replace IMPL012 specification section "Chromaprint Integration" (lines 206-231) with corrected unsafe FFI code

---

### ISSUE 3: Database Schema Not Defined ✅ RESOLVED

**Finding:** Specification references `acoustid_cache` table but never defines schema.

**Investigation:**
- Searched for `acoustid_cache` table definitions (SQL and Rust): NONE FOUND
- Read wkmp-common/src/db/init.rs to understand database initialization pattern
- Confirmed pattern: `CREATE TABLE IF NOT EXISTS` functions called from `init_database()`

**Current Pattern (lines 58-71):**
```rust
async fn create_schema_version_table(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY,
            applied_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}
```

**Resolution:** Add `create_acoustid_cache_table()` function to wkmp-common/src/db/init.rs:

```rust
async fn create_acoustid_cache_table(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS acoustid_cache (
            fingerprint_hash TEXT PRIMARY KEY,
            mbid TEXT NOT NULL,
            cached_at TEXT NOT NULL DEFAULT (datetime('now')),
            CHECK (length(fingerprint_hash) = 64)  -- SHA-256 hex
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create index for expiration queries (if we add cache expiration later)
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_acoustid_cache_cached_at ON acoustid_cache(cached_at)")
        .execute(pool)
        .await?;

    Ok(())
}
```

Then add call to `init_database()`:
```rust
pub async fn init_database(db_path: &Path) -> Result<SqlitePool> {
    // ... existing code ...

    create_schema_version_table(&pool).await?;
    create_users_table(&pool).await?;
    create_settings_table(&pool).await?;
    create_module_config_table(&pool).await?;
    create_files_table(&pool).await?;
    create_passages_table(&pool).await?;
    create_queue_table(&pool).await?;
    create_acoustid_cache_table(&pool).await?;  // NEW

    // ... rest of function ...
}
```

**Priority:** HIGH (implementation requirement)
**Action:**
1. Add `create_acoustid_cache_table()` to wkmp-common/src/db/init.rs
2. Update IMPL012 specification to document schema (add new section "Database Schema")

---

### ISSUE 4: Test Fixtures Not Present ✅ RESOLVED

**Finding:** Integration test references `fixtures/sample.mp3` which doesn't exist.

**Investigation:**
- Project has no fixtures/ directory
- No standard test audio files in repository

**Decision:** Multiple test strategies available:

**Option A: Skip Real Audio Tests (RECOMMENDED)**
- Use mock/stub for Symphonia decoder (returns synthetic PCM)
- Use mock for Chromaprint FFI (returns dummy fingerprint)
- Focus on unit tests for pipeline logic
- **Rationale:** Avoids copyright issues, fast tests, no large binary files in repo

**Option B: Use User's Music (Manual Testing Only)**
- Integration tests marked `#[ignore]` by default
- Documentation explains how to run with `--ignored` flag and real file path
- **Rationale:** Real-world testing without repo bloat

**Option C: Generate Synthetic Audio**
- Use `hound` crate to generate WAV files (sine wave, white noise)
- Test basic pipeline functionality
- **Rationale:** Reproducible, no copyright issues
- **Limitation:** Won't test real-world codec complexity

**Resolution:** Implement Option A for automated testing, Option B for manual validation.

**Test Structure:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    // Unit tests (always run)
    #[test]
    fn test_fingerprint_pipeline_stages() {
        // Test each stage with mocked dependencies
    }

    // Integration tests (manual only)
    #[test]
    #[ignore]
    fn test_real_audio_file() {
        // Requires user to set WKMP_TEST_AUDIO_FILE env var
        let path = std::env::var("WKMP_TEST_AUDIO_FILE")
            .expect("Set WKMP_TEST_AUDIO_FILE to run this test");
        // ... test with real file
    }
}
```

**Priority:** Medium (testing strategy)
**Action:** Update IMPL012 specification section "Testing" (lines 510-595) with test strategy

---

### ISSUE 5: Rate Limiting Clarification ✅ RESOLVED

**Finding:** Specification says "No explicit rate limiter needed" but contradicts with "AcoustID: 3 requests/second".

**Investigation:**
- Read wkmp-ai/src/services/acoustid_client.rs
- **Finding:** Rate limiter ALREADY IMPLEMENTED (lines 66-93)
- RateLimiter struct with 334ms minimum interval (3 req/sec)
- Called in `lookup()` at line 126

**Current Code:**
```rust
struct RateLimiter {
    last_request: Mutex<Option<Instant>>,
    min_interval: Duration,
}

impl AcoustIDClient {
    pub async fn lookup(&self, fingerprint: &str, duration_seconds: u64) -> Result<...> {
        // Rate limit
        self.rate_limiter.wait().await;  // ← RATE LIMITING HERE

        // ... API call
    }
}
```

**Resolution:** Specification statement is **INCORRECT**. Rate limiter IS needed and IS implemented.

**Corrected Specification Text:**
```markdown
### API Rate Limiting **[CORRECTED]**

**AcoustID Limit:** 3 requests/second
**Implementation:** RateLimiter with 334ms minimum interval between requests
**Location:** wkmp-ai/src/services/acoustid_client.rs (lines 66-93)

**Rationale:** While MusicBrainz (1 req/s) is slower overall, parallel processing could cause burst traffic to AcoustID. Rate limiter prevents exceeding 3 req/s limit.
```

**Priority:** Low (already correctly implemented, just specification error)
**Action:** Update IMPL012 specification section "Performance Considerations" (lines 606-609)

---

## Current Implementation State

### What EXISTS and is CORRECT

**wkmp-ai/src/services/acoustid_client.rs (268 lines) ✅**
- HTTP client configuration with 30s timeout
- Rate limiter (334ms interval, 3 req/sec)
- POST to https://api.acoustid.org/v2/lookup with form params
- Response parsing (AcoustIDResponse, AcoustIDResult, AcoustIDRecording structs)
- Best MBID extraction (first recording from first result)
- Error handling (NetworkError, ApiError, ParseError, InvalidApiKey, NoMatches)
- Unit tests (rate limiter, MBID extraction, error cases)

**wkmp-ai/src/services/fingerprinter.rs (partial) ✅**
- Struct skeleton exists
- Placeholder implementations (return dummy data)
- Error types defined (FingerprintError)

**wkmp-common/src/db/init.rs ✅**
- Database initialization pattern established
- CREATE TABLE IF NOT EXISTS pattern
- Example tables (users, settings, files, passages, queue)

**wkmp-ai/Cargo.toml ✅**
- All required dependencies present
- chromaprint-sys-next, symphonia, rubato, reqwest, sha2

### What is MISSING or INCORRECT

**wkmp-ai/src/services/acoustid_client.rs ❌ MISSING CACHING**
- No database pool field
- No `get_cached_mbid()` function
- No `cache_mbid()` function
- No `hash_fingerprint()` function
- Does not follow IMPL012 specification lines 398-440

**wkmp-ai/src/services/fingerprinter.rs ❌ NOT IMPLEMENTED**
- `decode_audio()` returns dummy PCM data (line 61-89)
- `generate_chromaprint()` returns dummy bytes (line 91-103)
- No Symphonia integration
- No Rubato integration
- No Chromaprint FFI calls

**wkmp-common/src/db/init.rs ❌ MISSING TABLE**
- No `create_acoustid_cache_table()` function
- No call in `init_database()`

**IMPL012 specification ❌ CONTAINS ERRORS**
- Lines 206-231: Incorrect Rust API (shows high-level API that doesn't exist)
- Lines 615-621: Incorrect API key configuration (shows env var instead of database)
- Lines 606-609: Incorrect rate limiting statement (claims no limiter needed)
- Missing: Database schema definition section

---

## Specification Gap Analysis

### Requirements Coverage

| Requirement ID | Specification | Current Implementation | Status |
|---|---|---|---|
| REQ-FP-010 | Audio decoding | Placeholder only | ❌ NOT IMPLEMENTED |
| REQ-FP-020 | Audio resampling | Placeholder only | ❌ NOT IMPLEMENTED |
| REQ-FP-030 | Chromaprint fingerprinting | Placeholder only | ❌ NOT IMPLEMENTED |
| REQ-FP-040 | Error handling | Error types defined | ✅ COMPLETE |
| REQ-AC-010 | API communication | Fully implemented | ✅ COMPLETE |
| REQ-AC-020 | Response parsing | Fully implemented | ✅ COMPLETE |
| REQ-AC-030 | MBID selection | Fully implemented | ✅ COMPLETE |
| REQ-AC-040 | Error handling | Fully implemented | ✅ COMPLETE |
| REQ-CA-010 | Cache lookup | Not implemented | ❌ MISSING |
| REQ-CA-020 | Cache storage | Not implemented | ❌ MISSING |
| REQ-CA-030 | Fingerprint hashing | Not implemented | ❌ MISSING |
| REQ-BLD-010 | LLVM/Clang dependency | Documented | ✅ COMPLETE |
| REQ-PERF-010 | Processing time | Not yet testable | ⚠️ BLOCKED |
| REQ-PERF-020 | Memory usage | Not yet testable | ⚠️ BLOCKED |
| REQ-PERF-030 | Rate limiting | Fully implemented | ✅ COMPLETE |
| REQ-PERF-040 | Cache effectiveness | Depends on caching | ⚠️ BLOCKED |
| REQ-TEST-010 | Unit tests | Partial (acoustid_client only) | ⚠️ INCOMPLETE |
| REQ-TEST-020 | Integration tests | Not implemented | ❌ MISSING |

**Summary:**
- ✅ COMPLETE: 7/18 (39%)
- ❌ NOT IMPLEMENTED: 7/18 (39%)
- ⚠️ BLOCKED/INCOMPLETE: 4/18 (22%)

---

## Specification Corrections Required

### CRITICAL: Update Chromaprint Integration Section

**File:** docs/IMPL012-acoustid_client.md
**Lines:** 206-231
**Issue:** Shows non-existent high-level API (`Context`, `Algorithm` types)

**Required Changes:**
1. Replace with unsafe FFI code using chromaprint-sys-next
2. Add safety warnings about unsafe code
3. Document error handling for all FFI calls
4. Show proper resource cleanup (chromaprint_free, chromaprint_dealloc)

### HIGH: Add Database Schema Section

**File:** docs/IMPL012-acoustid_client.md
**Location:** After line 240 (before "AcoustID API Client" section)

**Required Content:**
```markdown
## Database Schema

### AcoustID Cache Table

**Purpose:** Cache fingerprint → MBID mappings to reduce API calls

**Schema:**
```sql
CREATE TABLE IF NOT EXISTS acoustid_cache (
    fingerprint_hash TEXT PRIMARY KEY,
    mbid TEXT NOT NULL,
    cached_at TEXT NOT NULL DEFAULT (datetime('now')),
    CHECK (length(fingerprint_hash) = 64)
);

CREATE INDEX IF NOT EXISTS idx_acoustid_cache_cached_at
    ON acoustid_cache(cached_at);
```

**Columns:**
- `fingerprint_hash`: SHA-256 hash of Chromaprint fingerprint (64 hex chars)
- `mbid`: MusicBrainz Recording MBID (UUID string)
- `cached_at`: Timestamp of cache entry creation (ISO 8601)

**Rationale:**
- Full fingerprint strings are large (~1-5 KB) - hash saves space
- SHA-256 provides negligible collision probability
- Index on cached_at supports future cache expiration feature
```

### MEDIUM: Correct API Key Configuration

**File:** docs/IMPL012-acoustid_client.md
**Lines:** 615-621

**Replace:**
```markdown
**Environment Variable:** `ACOUSTID_API_KEY`
```

**With:**
```markdown
**Primary Source:** Database settings table
- Key: `acoustid_api_key`
- Loaded via `wkmp-ai::db::settings::get_acoustid_api_key(&db)`
- Synced to TOML for backup

**Fallback:** Environment variable `ACOUSTID_API_KEY` (testing/CI only)

**Follows:** Database-first configuration per REQ-NF-030 through REQ-NF-037
```

### MEDIUM: Correct Rate Limiting Statement

**File:** docs/IMPL012-acoustid_client.md
**Lines:** 606-609

**Replace:**
```markdown
- Implementation: No explicit rate limiter needed (MusicBrainz 1/s is bottleneck)
```

**With:**
```markdown
- Implementation: RateLimiter with 334ms interval (prevents bursts during parallel processing)
- Location: wkmp-ai/src/services/acoustid_client.rs lines 66-93
```

### MEDIUM: Update Testing Section

**File:** docs/IMPL012-acoustid_client.md
**Lines:** 510-595

**Add:**
```markdown
### Test Strategy

**Unit Tests (Automated):**
- Mock Symphonia decoder for audio processing tests
- Mock Chromaprint FFI for fingerprint generation tests
- Test error handling with synthetic failures

**Integration Tests (Manual):**
- Mark with `#[ignore]` attribute
- Require `WKMP_TEST_AUDIO_FILE` environment variable
- Run with `cargo test --ignored`
- Document in test function docstrings

**Rationale:** Avoids copyright issues and large binary files in repository.
```

---

## Dependency Verification

### Rust Crates ✅ ALL PRESENT

Verified in wkmp-ai/Cargo.toml:
- chromaprint-sys-next = "1.6" (line 36)
- symphonia = { version = "0.5", features = ["all"] } (line 41)
- rubato = "0.15" (line 42)
- reqwest = { version = "0.11", features = ["json"] } (line 30)
- sha2 = "0.10" (line 39)
- base64 = "0.22" (line 37)
- serde/serde_json (lines 18-19)
- sqlx (line 26)

**Action:** No dependency changes needed

### System Dependencies ⚠️ BUILD-TIME REQUIREMENT

**LLVM/Clang** required for chromaprint-sys-next bindgen:
- Windows: https://releases.llvm.org/ (add to PATH)
- Linux: `sudo apt-get install llvm-dev libclang-dev clang`
- macOS: `brew install llvm`

**Action:** Document in project README.md build requirements section

---

## Implementation Scope Refinement

Based on current state analysis, implementation must:

### 1. Implement Fingerprinting Pipeline (fingerprinter.rs)
- Replace placeholder `decode_audio()` with Symphonia integration
- Replace placeholder `resample_to_44100()` with Rubato integration
- Replace placeholder `generate_chromaprint()` with chromaprint-sys-next FFI
- Add helper `convert_to_mono_f32()` for channel mixing
- Add proper error handling for all stages

### 2. Add Caching Layer (acoustid_client.rs)
- Add `db: SqlitePool` field to AcoustIDClient
- Implement `hash_fingerprint()` using sha2 crate
- Implement `get_cached_mbid()` with database query
- Implement `cache_mbid()` with UPSERT query
- Update `lookup()` to check cache first

### 3. Add Database Table (wkmp-common/src/db/init.rs)
- Implement `create_acoustid_cache_table()`
- Add call to `init_database()`

### 4. Update Specification (docs/IMPL012-acoustid_client.md)
- Correct Chromaprint integration section (unsafe FFI code)
- Add database schema section
- Correct API key configuration section
- Correct rate limiting statement
- Update testing strategy section

### 5. Write Tests
- Unit tests for fingerprint hashing (determinism, length)
- Unit tests for MBID extraction (already exist, verify coverage)
- Unit tests for caching logic (cache hit, cache miss, UPSERT)
- Integration tests (manual only, documented with #[ignore])

---

## Risk Update

### New Risks Identified

**RISK-007: Unsafe FFI Code Safety**
- **Description:** chromaprint-sys-next requires extensive unsafe code blocks
- **Probability:** Medium (FFI is inherently unsafe)
- **Impact:** High (crashes, memory leaks, undefined behavior)
- **Mitigation:**
  - Wrap all FFI calls in unsafe blocks
  - Always check return values (chromaprint functions return 0/1 for failure/success)
  - Always call chromaprint_free() even on error paths
  - Always call chromaprint_dealloc() for fingerprint strings
  - Use RAII pattern (implement Drop trait) for context cleanup
- **Residual Risk:** Low-Medium (with careful implementation)

**RISK-008: Specification vs Implementation Divergence**
- **Description:** Current implementation already diverges from specification
- **Probability:** High (already confirmed)
- **Impact:** Medium (confusion, wasted effort following wrong spec)
- **Mitigation:** Update specification BEFORE implementing remaining features
- **Residual Risk:** None (will be resolved in this plan)

### Risk Summary

- RISK-001 (FFI Complexity): Elevated to High priority due to unsafe code discovery
- RISK-002 (Audio Format Compatibility): Unchanged (Low-Medium residual)
- RISK-003 (Resampling Quality): Unchanged (Very Low residual)
- RISK-004 (API Key Confusion): Unchanged (Low residual)
- RISK-005 (Cache Collisions): Unchanged (Very Low residual)
- RISK-006 (Missing Schema): RESOLVED (will add table creation)
- RISK-007 (Unsafe FFI Safety): NEW - High priority
- RISK-008 (Spec Divergence): NEW - Blocking issue for implementation

---

## Phase 2 Completeness Checklist

- [✅] chromaprint-sys-next API verified (unsafe FFI bindings confirmed)
- [✅] Database schema pattern identified (wkmp-common/src/db/init.rs)
- [✅] Current implementation state analyzed (7/18 requirements complete)
- [✅] Specification errors cataloged (3 critical corrections needed)
- [✅] Missing features identified (caching layer, fingerprinting pipeline)
- [✅] Dependencies verified (all present in Cargo.toml)
- [✅] Test strategy defined (unit tests with mocks, manual integration tests)
- [✅] Risks updated (2 new risks identified and mitigated)

---

## Next Steps

**Phase 2 Status:** ✅ COMPLETE

**Before Phase 3:**
1. ⚠️ **BLOCKER**: Update IMPL012 specification with corrections
   - Critical: Chromaprint integration section (unsafe FFI code)
   - High: Add database schema section
   - Medium: API key configuration, rate limiting, test strategy
2. Review updated specification with user for approval

**Phase 3 Actions:**
1. Define acceptance tests for each requirement
2. Map requirements to test cases (traceability matrix)
3. Define test-first workflow (write test → implement → verify)
4. Identify integration test checkpoints

**Phase 4 Actions:**
1. Break implementation into increments
2. Define increment boundaries (working intermediate states)
3. Assign requirements to increments
4. Create test checkpoints per increment

---

**Phase 2 Complete:** Specification verified, gaps identified, ready for test definition.
