# Test Coverage Analysis: AcoustID API Key Validation

**Date:** 2025-11-11
**Total Tests:** 244 passing
**Features Analyzed:** Pre-import AcoustID API key validation

---

## Executive Summary

**Current Status:** ⚠️ **PARTIAL COVERAGE**

The recently added AcoustID API key validation features have **minimal unit test coverage**. While the core validation logic in `acoustid_client.rs` has basic tests, the new API endpoints, database functions, and integration points are **not covered by automated tests**.

**Risk Assessment:**
- **Medium Risk:** Database operations and API endpoints lack test coverage
- **Low Risk:** Core validation logic has basic coverage
- **Manual Testing Required:** All integration points need manual verification

---

## Feature Coverage Breakdown

### 1. Backend Validation Logic

**File:** [wkmp-ai/src/extractors/acoustid_client.rs:108-185](wkmp-ai/src/extractors/acoustid_client.rs#L108-L185)

#### Functions Added:
- `validate_api_key()` (async method)
- `validate_acoustid_key()` (public helper)
- `is_invalid_api_key_error()` (utility)

#### Test Coverage: ⚠️ **PARTIAL**

**Existing Tests in acoustid_client.rs:**
```rust
#[test]
fn test_client_name() { ... }                    // ✅ Covered

#[test]
fn test_default_confidence() { ... }             // ✅ Covered

#[test]
fn test_custom_api_key() { ... }                 // ✅ Covered

#[test]
fn test_min_score_clamping() { ... }             // ✅ Covered

#[test]
fn test_score_to_confidence_mapping() { ... }    // ✅ Covered
```

**Missing Tests:**
- ❌ `validate_api_key()` - No unit tests
- ❌ `validate_acoustid_key()` - No unit tests
- ❌ `is_invalid_api_key_error()` - No unit tests
- ❌ Error code 3 handling (valid key, invalid fingerprint)
- ❌ Error code 5 handling (invalid API key)
- ❌ Error code 6 handling (invalid format)
- ❌ Network error handling
- ❌ JSON parsing error handling

**Rationale for Missing Tests:**
Validation functions require:
1. Network connectivity to AcoustID API
2. Mock HTTP client or integration test setup
3. Simulated API responses for different error codes

**Recommendation:** Add integration tests with mocked HTTP responses.

---

### 2. Settings API Endpoints

**File:** [wkmp-ai/src/api/settings.rs](wkmp-ai/src/api/settings.rs)

#### Functions Added:
- `get_acoustid_api_key()` (GET handler)
- `GetApiKeyResponse` (response struct)

#### Test Coverage: ❌ **NONE**

**No tests exist for:**
- ❌ GET /api/settings/acoustid_api_key endpoint
- ❌ Response serialization
- ❌ Database read error handling
- ❌ Empty/missing key scenarios

**Missing Test Scenarios:**
1. Key exists in database → Returns configured=true with key
2. Key doesn't exist → Returns configured=false with no key
3. Database read fails → Returns 500 error
4. Response JSON format correct

**Recommendation:** Add integration tests with test database.

---

### 3. Import Workflow API Endpoints

**File:** [wkmp-ai/src/api/import_workflow.rs](wkmp-ai/src/api/import_workflow.rs)

#### Functions Added:
- `validate_acoustid()` (POST /import/validate-acoustid)
- `ValidateAcoustIDRequest` / `ValidateAcoustIDResponse`
- Updated `update_acoustid_key()` with validation

#### Test Coverage: ❌ **NONE**

**No tests exist for:**
- ❌ POST /import/validate-acoustid endpoint
- ❌ Empty API key validation
- ❌ Invalid API key response
- ❌ Valid API key response
- ❌ Network error handling
- ❌ Request/response serialization

**Missing Test Scenarios:**
1. Empty API key → Returns valid=false
2. Invalid API key → Returns valid=false with error message
3. Valid API key → Returns valid=true
4. Network failure → Returns valid=false with network error
5. Malformed request → Returns 400 error

**Recommendation:** Add integration tests with mocked AcoustID client.

---

### 4. Database Settings Operations

**File:** [wkmp-ai/src/db/settings.rs](wkmp-ai/src/db/settings.rs)

#### Functions Used (No Changes):
- `get_acoustid_api_key()` (existing)
- `set_acoustid_api_key()` (existing)

#### Test Coverage: ❌ **NONE**

**No tests exist for:**
- ❌ get_acoustid_api_key() database query
- ❌ set_acoustid_api_key() database insert/update
- ❌ UPSERT conflict handling
- ❌ NULL/empty value handling

**Missing Test Scenarios:**
1. Key doesn't exist → Returns None
2. Key exists → Returns Some(key)
3. Set new key → Inserts into database
4. Update existing key → Updates value
5. Database connection failure → Returns error

**Recommendation:** Add database integration tests.

---

### 5. Database Sessions Operations

**File:** [wkmp-ai/src/db/sessions.rs](wkmp-ai/src/db/sessions.rs)

#### Functions Added:
- `get_active_session()` (query for active import)

#### Test Coverage: ❌ **NONE**

**No tests exist for:**
- ❌ get_active_session() query logic
- ❌ Filtering non-terminal states (COMPLETED, CANCELLED, FAILED)
- ❌ Ordering by started_at DESC
- ❌ Deserialization of ImportSession
- ❌ Multiple active sessions (should return most recent)
- ❌ No active sessions → Returns None

**Missing Test Scenarios:**
1. No sessions in database → Returns None
2. Only completed sessions → Returns None
3. One active session → Returns that session
4. Multiple active sessions → Returns most recent
5. Database error → Returns error

**Recommendation:** Add database integration tests.

---

### 6. Frontend JavaScript

**File:** [wkmp-ai/static/import-progress.js](wkmp-ai/static/import-progress.js)

#### Functions Added:
- `validateAcoustIDBeforeImport()`
- `promptForAcoustIDKey()`
- Updated `startImport()`

#### Test Coverage: ❌ **NONE**

**JavaScript Unit Tests:** Not present in codebase

**Manual Testing Required:**
- ❌ Pre-import validation flow
- ❌ Modal display when key invalid/missing
- ❌ Re-validation loop for invalid keys
- ❌ API key saved to database after validation
- ❌ Import proceeds after validation success
- ❌ Import cancelled if user closes modal

**Recommendation:** Manual testing only (no JS test framework in project).

---

## Test Coverage Summary Table

| Component | File | Functions Added | Unit Tests | Integration Tests | Manual Tests |
|-----------|------|----------------|------------|-------------------|--------------|
| **Validation Logic** | acoustid_client.rs | 3 functions | ⚠️ Partial (basic only) | ❌ None | Required |
| **Settings API** | api/settings.rs | 1 endpoint | ❌ None | ❌ None | Required |
| **Import API** | api/import_workflow.rs | 1 endpoint + updates | ❌ None | ❌ None | Required |
| **DB Settings** | db/settings.rs | 0 (reused existing) | ❌ None | ❌ None | Required |
| **DB Sessions** | db/sessions.rs | 1 function | ❌ None | ❌ None | Required |
| **Frontend JS** | static/import-progress.js | 2 functions + updates | ❌ None | ❌ None | Required |

**Overall Coverage:** ~10% (basic validation logic only)

---

## Risk Assessment

### High-Risk Gaps (No Coverage)

1. **API Endpoint Behavior**
   - No tests verify request/response serialization
   - No tests verify error handling (400, 500 codes)
   - **Risk:** Breaking changes undetected until runtime

2. **Database Operations**
   - No tests verify SQL queries execute correctly
   - No tests verify UPSERT conflict handling
   - **Risk:** Data corruption or query failures in production

3. **Integration Flow**
   - No tests verify end-to-end validation flow
   - No tests verify frontend ↔ backend communication
   - **Risk:** Integration failures only found during manual testing

### Medium-Risk Gaps (Partial Coverage)

4. **Validation Logic Error Handling**
   - Basic tests exist, but no error code scenarios tested
   - No mocked HTTP responses for different AcoustID errors
   - **Risk:** Incorrect error handling for specific error codes

### Low-Risk Gaps (Mitigated by Manual Testing)

5. **Frontend JavaScript**
   - No JS unit tests (project has no JS test framework)
   - **Risk:** Mitigated by manual testing (low complexity, UI-driven)

---

## Recommended Testing Strategy

### Phase 1: Critical Path Unit Tests (HIGH PRIORITY)

**Goal:** Cover validation logic error handling

**Tasks:**
1. Add unit tests for `validate_api_key()` with mocked HTTP client
   - Test error code 3 (valid key) → Ok(())
   - Test error code 5 (invalid key) → Err(...)
   - Test error code 6 (invalid format) → Err(...)
   - Test network errors → Err(...)
   - Test malformed JSON → Err(...)

2. Add unit tests for `is_invalid_api_key_error()`
   - Test with "invalid API key" message → true
   - Test with other error messages → false

**Effort:** 2-3 hours
**Value:** Catches validation logic bugs before production

---

### Phase 2: Database Integration Tests (MEDIUM PRIORITY)

**Goal:** Verify database operations work correctly

**Tasks:**
1. Add integration tests for `get_acoustid_api_key()`
   - Setup: In-memory SQLite database with test data
   - Test key exists → Returns Some(key)
   - Test key doesn't exist → Returns None

2. Add integration tests for `set_acoustid_api_key()`
   - Test insert new key → Key stored in database
   - Test update existing key → Key value updated

3. Add integration tests for `get_active_session()`
   - Test no sessions → Returns None
   - Test only completed sessions → Returns None
   - Test active session → Returns session
   - Test multiple active sessions → Returns most recent

**Effort:** 4-6 hours
**Value:** Prevents database schema mismatches and query bugs

---

### Phase 3: API Endpoint Integration Tests (MEDIUM PRIORITY)

**Goal:** Verify HTTP API behavior

**Tasks:**
1. Add integration tests for GET /api/settings/acoustid_api_key
   - Test key exists → Returns 200 with configured=true
   - Test key doesn't exist → Returns 200 with configured=false
   - Test database error → Returns 500

2. Add integration tests for POST /import/validate-acoustid
   - Test empty key → Returns valid=false
   - Test invalid key → Returns valid=false with error
   - Test valid key (mocked) → Returns valid=true
   - Test network error (mocked) → Returns valid=false

**Effort:** 4-6 hours
**Value:** Catches API contract violations and serialization bugs

---

### Phase 4: Manual Testing (REQUIRED)

**Goal:** Verify end-to-end user experience

**Test Scenarios:**
1. **No API Key Configured**
   - Start import → Modal appears
   - Enter invalid key → Error shown, retry
   - Enter valid key → Modal closes, import starts
   - Skip → Import starts without AcoustID

2. **Invalid API Key Configured**
   - Start import → Modal appears with error
   - Update key → Import proceeds
   - Skip → Import proceeds without AcoustID

3. **Valid API Key Configured**
   - Start import → No modal, import starts immediately

4. **Page Reload During Import**
   - Reload page → Progress UI restored automatically

**Effort:** 1-2 hours
**Value:** Ensures user-facing features work correctly

---

## Test Implementation Guidance

### Example: Unit Test for validate_api_key()

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mockito::{mock, server_url}; // Requires mockito crate

    #[tokio::test]
    async fn test_validate_api_key_valid_returns_error_code_3() {
        let _m = mock("POST", "/v2/lookup")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"status":"ok","error":{"code":3,"message":"invalid fingerprint"}}"#)
            .create();

        let client = AcoustIDClient::new("test_key".to_string());
        let result = client.validate_api_key().await;

        assert!(result.is_ok(), "Error code 3 should be treated as valid key");
    }

    #[tokio::test]
    async fn test_validate_api_key_invalid_returns_error_code_5() {
        let _m = mock("POST", "/v2/lookup")
            .with_status(400)
            .with_header("content-type", "application/json")
            .with_body(r#"{"status":"error","error":{"code":5,"message":"invalid API key"}}"#)
            .create();

        let client = AcoustIDClient::new("bad_key".to_string());
        let result = client.validate_api_key().await;

        assert!(result.is_err(), "Error code 5 should indicate invalid key");
        assert!(result.unwrap_err().to_string().contains("invalid"));
    }
}
```

**Dependencies Required:**
- `mockito = "1.2"` (HTTP mocking)
- Update ACOUSTID_API_URL to use mockito server in tests

---

### Example: Database Integration Test for get_acoustid_api_key()

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect(":memory:").await.unwrap();

        // Run migrations
        sqlx::query(
            "CREATE TABLE settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            )"
        )
        .execute(&pool)
        .await
        .unwrap();

        pool
    }

    #[tokio::test]
    async fn test_get_acoustid_api_key_exists() {
        let pool = setup_test_db().await;

        // Insert test key
        set_acoustid_api_key(&pool, "test_key_123".to_string())
            .await
            .unwrap();

        // Retrieve key
        let result = get_acoustid_api_key(&pool).await.unwrap();

        assert_eq!(result, Some("test_key_123".to_string()));
    }

    #[tokio::test]
    async fn test_get_acoustid_api_key_not_exists() {
        let pool = setup_test_db().await;

        let result = get_acoustid_api_key(&pool).await.unwrap();

        assert_eq!(result, None);
    }
}
```

---

## Dependencies Required for Testing

**Add to Cargo.toml [dev-dependencies]:**
```toml
[dev-dependencies]
mockito = "1.2"          # HTTP mocking for validation tests
tokio-test = "0.4"       # Tokio test utilities
```

**Already Available:**
- `tokio::test` macro (for async tests)
- `sqlx::SqlitePool::connect(":memory:")` (in-memory database)

---

## Conclusion

### Current State
- **244 tests passing** (unchanged from before validation features added)
- **0 new tests** added for validation features
- **~10% coverage** of new code (basic validation logic only)

### Gaps
- ❌ No API endpoint tests
- ❌ No database operation tests
- ❌ No integration flow tests
- ❌ No error handling tests for validation logic

### Recommendations

**Priority 1 (Essential):**
- Add unit tests for `validate_api_key()` with mocked HTTP responses
- Conduct comprehensive manual testing (all scenarios)

**Priority 2 (Important):**
- Add database integration tests (settings, sessions)
- Add API endpoint integration tests

**Priority 3 (Nice-to-have):**
- Add JavaScript unit tests (requires adding test framework)

### Risk Mitigation

**Current Approach:** Manual testing only
- ✅ Acceptable for initial release (low complexity features)
- ⚠️ Regressions may go undetected in future changes
- ❌ No automated verification of validation logic correctness

**Recommended Approach:** Manual + automated tests
- ✅ Catches regressions automatically
- ✅ Verifies validation logic correctness
- ✅ Safer for long-term maintenance

---

## Manual Testing Checklist

Use this checklist to verify all features work correctly:

### Pre-Import Validation

- [ ] **No API key configured**
  - [ ] Modal appears when clicking "Start Import"
  - [ ] Error message shows "No AcoustID API key configured"
  - [ ] Can enter API key in modal
  - [ ] Can click "Skip AcoustID"

- [ ] **Invalid API key entered**
  - [ ] Error shown: "Invalid API key: ..."
  - [ ] Modal stays open
  - [ ] Can retry with different key
  - [ ] Button text changes to "Validating..." during check

- [ ] **Valid API key entered**
  - [ ] Modal closes automatically
  - [ ] Import starts
  - [ ] API key saved to database (persists for next import)

- [ ] **Skip AcoustID**
  - [ ] Modal closes
  - [ ] Import starts
  - [ ] AcoustID functionality disabled (check DEBUG logs)

- [ ] **Valid API key configured (pre-existing)**
  - [ ] No modal appears
  - [ ] Import starts immediately
  - [ ] Validation happens silently in background

### Page Reload During Import

- [ ] **Active import in progress**
  - [ ] Reload page
  - [ ] Progress UI restores automatically
  - [ ] Shows current phase and progress
  - [ ] SSE reconnects and receives updates

- [ ] **No active import**
  - [ ] Reload page
  - [ ] Setup form shown (root folder input)

### DEBUG Logging

- [ ] **AcoustID skipped (user choice)**
  - [ ] Log: "AcoustID extraction skipped (user chose to skip AcoustID functionality)"
  - [ ] Appears for each passage processed

- [ ] **AcoustID skipped (no key)**
  - [ ] Log: "AcoustID extraction skipped (no API key configured)"
  - [ ] Appears for each passage processed

---

**Test Results:**
- Date tested: ___________
- Tester: ___________
- All scenarios passed: ☐ Yes ☐ No
- Issues found: ___________________________________________
