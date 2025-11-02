# Increment 9: Integration and Recovery Tests

**Estimated Effort:** 3-4 hours
**Dependencies:** Increments 1-8 (all components)
**Risk:** LOW

---

## Objectives

Implement integration tests for database recovery, concurrency, and system workflows.

---

## Requirements Addressed

- [APIK-TOML-010] - Durable backup survives deletion
- [APIK-ATOMIC-020] - Prevent races
- [APIK-TEST-020] - Integration tests
- [APIK-TEST-030] - System tests

---

## Deliverables

### Integration Tests

**File: wkmp-ai/tests/integration/recovery_tests.rs** (new)

Tests for tc_i_recovery_001-002:

```rust
// tc_i_recovery_001: Database deletion recovers from TOML
#[tokio::test]
async fn test_database_deletion_recovers_from_toml() {
    // 1. Set key via UI (writes to DB + TOML)
    // 2. Delete database
    // 3. Restart wkmp-ai
    // 4. Verify key loaded from TOML
    // 5. Verify key migrated back to database
}

// tc_i_recovery_002: Database deletion with no TOML fails gracefully
#[tokio::test]
async fn test_database_deletion_no_toml_fails() {
    // 1. Delete database and TOML
    // 2. Attempt startup
    // 3. Verify clear error message
}
```

**File: wkmp-ai/tests/integration/concurrent_tests.rs** (new)

Tests for tc_i_concurrent_001:

```rust
// tc_i_concurrent_001: Multiple module startup TOML reads safe
#[tokio::test]
async fn test_concurrent_toml_reads_safe() {
    // 1. Write TOML file
    // 2. Spawn multiple tasks reading TOML concurrently
    // 3. Verify all reads succeed (no corruption)
}
```

---

### System Tests

**File: Manual testing**

Tests for tc_s_workflow_002-003:

```
tc_s_workflow_002: Developer uses ENV variable for CI/CD
- Set WKMP_ACOUSTID_API_KEY=test-key
- Start wkmp-ai
- Verify key migrated to database + TOML
- Verify can unset ENV (key persists in database)

tc_s_workflow_003: Database deletion recovery workflow
- Configure key via web UI
- Delete database file
- Restart wkmp-ai
- Verify key recovered from TOML
- Verify key migrated back to database
```

---

## Acceptance Criteria

- [ ] Database recovery tests pass (2 tests)
- [ ] Concurrency test passes (1 test)
- [ ] System workflow tests complete (2 manual tests)
- [ ] All integration tests pass (total 10 tests: e2e + UI + recovery + concurrent)

---

## Test Traceability

- tc_i_recovery_001-002: Database recovery
- tc_i_concurrent_001: Concurrency
- tc_s_workflow_002-003: User workflows

---

## Rollback Plan

Remove test files. No impact on implementation (tests only).
