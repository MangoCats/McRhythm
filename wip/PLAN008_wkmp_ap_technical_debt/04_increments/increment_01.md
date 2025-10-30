# Increment 01: Implement POST Authentication

**Sprint:** 1 (Security & Critical)
**Estimated Effort:** 3 hours
**Dependencies:** None
**Risk Level:** HIGH (security-critical)

---

## Objectives

Implement JSON body-based authentication for POST requests to wkmp-ap HTTP API.

**What:** Extract `shared_secret` from request JSON body, validate against server secret
**Why:** Eliminate CRITICAL security vulnerability (POST requests currently bypass auth)
**Success:** POST requests without valid secret return HTTP 401

---

## Requirements Satisfied

- REQ-DEBT-SEC-001-010: Validate authentication for ALL POST requests
- REQ-DEBT-SEC-001-020: POST auth uses shared_secret mechanism
- REQ-DEBT-SEC-001-030: Extract credentials from JSON body field
- REQ-DEBT-SEC-001-040: Return HTTP 401 on auth failure

---

## Implementation Tasks

### Task 1: Modify auth_middleware.rs (2 hours)

**File:** `wkmp-ap/src/api/auth_middleware.rs`
**Lines:** 825-835

**Current Code:**
```rust
Method::POST | Method::PUT => {
    tracing::warn!("POST/PUT request bypassing authentication - not yet implemented");
    return Ok(Authenticated);  // ⚠️ ALLOWS ALL REQUESTS
}
```

**New Code:**
```rust
Method::POST | Method::PUT => {
    // Buffer request body for authentication
    let body_bytes = match hyper::body::to_bytes(req.body_mut()).await {
        Ok(bytes) => bytes,
        Err(e) => {
            tracing::error!("Failed to read request body: {}", e);
            return Err(auth_error_response(
                StatusCode::BAD_REQUEST,
                "invalid_request",
                "Failed to read request body"
            ));
        }
    };

    // Parse JSON
    let json: serde_json::Value = match serde_json::from_slice(&body_bytes) {
        Ok(v) => v,
        Err(e) => {
            tracing::debug!("Failed to parse JSON body: {}", e);
            return Err(auth_error_response(
                StatusCode::UNAUTHORIZED,
                "authentication_required",
                "Missing or invalid shared_secret"
            ));
        }
    };

    // Extract and validate shared_secret
    match json.get("shared_secret") {
        Some(serde_json::Value::String(secret)) => {
            // Validate against server secret
            let server_secret = state.read().await.shared_secret;
            if secret == &server_secret.to_string() {
                // Valid - reconstruct request with body for handler
                // TODO: Reconstruct request body (Axum middleware limitation)
                tracing::debug!("POST authentication successful");
                return Ok(Authenticated);
            } else {
                tracing::warn!("Invalid shared_secret in POST request");
                return Err(auth_error_response(
                    StatusCode::UNAUTHORIZED,
                    "invalid_credentials",
                    "Invalid shared_secret"
                ));
            }
        }
        Some(_) => {
            // shared_secret present but wrong type
            return Err(auth_error_response(
                StatusCode::UNAUTHORIZED,
                "authentication_required",
                "shared_secret must be string"
            ));
        }
        None => {
            // No shared_secret field
            return Err(auth_error_response(
                StatusCode::UNAUTHORIZED,
                "authentication_required",
                "Missing shared_secret field"
            ));
        }
    }
}
```

**Key Changes:**
1. Buffer request body asynchronously
2. Parse JSON with error handling
3. Extract `shared_secret` field
4. Validate type (must be string)
5. Compare against server secret
6. Return appropriate 401 errors

---

### Task 2: Write Unit Test (1 hour)

**File:** `wkmp-ap/tests/auth_post_test.rs` (new file)

**Test:** TC-SEC-001-01 (POST with valid secret succeeds)

```rust
#[tokio::test]
async fn test_post_with_valid_secret_succeeds() {
    // Setup
    let (server, secret) = spawn_test_server().await;
    let client = reqwest::Client::new();
    let url = format!("{}/api/queue", server.addr());

    // Execute
    let response = client
        .post(&url)
        .json(&json!({
            "passage_id": "550e8400-e29b-41d4-a716-446655440000",
            "shared_secret": secret
        }))
        .send()
        .await
        .expect("Failed to send request");

    // Verify
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Expected 200 OK for authenticated POST request"
    );
}
```

**Additional Tests (Increment 03):**
- TC-SEC-001-02: Invalid secret → 401
- TC-SEC-001-03: Missing secret → 401
- TC-SEC-001-05: Malformed JSON → 401
- TC-SEC-001-06: Wrong secret type → 401

---

## Testing Strategy

**Unit Tests:**
- Test happy path (valid secret)
- Test error paths (invalid, missing, malformed)

**Integration Tests:**
- Verify actual endpoint behavior end-to-end

**Manual Testing:**
```bash
# Start wkmp-ap
cargo run -p wkmp-ap

# Test POST without secret (should fail)
curl -X POST http://localhost:5721/api/queue \
  -H "Content-Type: application/json" \
  -d '{"passage_id": "test-uuid"}'
# Expected: 401 Unauthorized

# Test POST with valid secret (should succeed)
curl -X POST http://localhost:5721/api/queue \
  -H "Content-Type: application/json" \
  -d '{"passage_id": "test-uuid", "shared_secret": "your-secret-here"}'
# Expected: 200 OK (or appropriate success code)
```

---

## Verification Checklist

- [ ] Code compiles without errors
- [ ] TC-SEC-001-01 test passes
- [ ] Manual curl test: POST without secret returns 401
- [ ] Manual curl test: POST with invalid secret returns 401
- [ ] Manual curl test: POST with valid secret returns 200
- [ ] No regression: GET requests still work with query auth
- [ ] Logs show authentication success/failure appropriately
- [ ] Code review completed

---

## Commit Message

```
[PLAN008-01] Implement POST authentication via JSON body

- Extract shared_secret from request JSON body
- Validate against server secret
- Return 401 if invalid/missing
- Add TC-SEC-001-01 test for happy path

Fixes critical security vulnerability: POST requests previously
bypassed authentication, allowing any client to modify playback
state without credentials.

Refs: REQ-DEBT-SEC-001-010, REQ-DEBT-SEC-001-020,
      REQ-DEBT-SEC-001-030, REQ-DEBT-SEC-001-040
Tests: TC-SEC-001-01
```

---

## Known Issues / Limitations

**Body Reconstruction:**
After extracting JSON for authentication, the body is consumed. Axum middleware doesn't easily allow reconstructing the request with buffered body. Current implementation validates auth but handler may not receive body.

**Workaround Options:**
1. Use Axum extension to pass parsed JSON to handler
2. Accept that auth happens but body needs re-parsing in handler
3. Research Axum 0.7 middleware patterns for body preservation

**For this increment:** Implement basic auth validation, defer body reconstruction to handler if needed.

---

## Dependencies

**Before Starting:**
- Existing auth_middleware.rs infrastructure (shared_secret validation for GET)
- Axum 0.7 HTTP framework
- serde_json for JSON parsing

**After Completion:**
- Increment 02 (PUT authentication) can use same pattern
- Increment 03 (edge case tests) depends on this implementation

---

## Risk Mitigation

**Risk:** Breaking existing GET authentication
**Mitigation:** Only modify POST/PUT branch, leave GET unchanged, test GET still works

**Risk:** Body consumption prevents handler from reading
**Mitigation:** Document limitation, test with real handlers, add workaround if needed

**Risk:** Performance overhead from body buffering
**Mitigation:** Measure request latency before/after, expect <10ms impact

---

## Acceptance Criteria

**This increment is complete when:**
1. POST requests without secret return 401
2. POST requests with invalid secret return 401
3. POST requests with valid secret proceed to handler
4. TC-SEC-001-01 test passes
5. No regression in GET authentication
6. Code committed with reference to PLAN008-01

**Estimated Completion Time:** 3 hours
**Actual Completion Time:** ___ hours (fill in after completion)
