# PLAN012 - Test Index

**Specification:** SPEC025-api_key_configuration.md
**Plan Version:** 1.0
**Date:** 2025-10-30

---

## Test Summary

**Total Test Cases:** 47
**Unit Tests:** 28
**Integration Tests:** 10
**System Tests:** 3
**Manual Tests:** 6

**Coverage:** 100% (all 62 requirements traced)

---

## Test Categories

### Unit Tests (28 tests)

**Multi-Tier Resolution (8 tests):**
- tc_u_res_001: Database priority (database key overrides ENV and TOML)
- tc_u_res_002: ENV fallback (database empty, ENV provides key)
- tc_u_res_003: TOML fallback (database and ENV empty, TOML provides key)
- tc_u_res_004: Error on no key (all sources empty)
- tc_u_res_005: Database ignores ENV when present
- tc_u_res_006: Database ignores TOML when present
- tc_u_res_007: ENV ignores TOML when present
- tc_u_res_008: Multiple sources warning logged

**Write-Back Behavior (6 tests):**
- tc_u_wb_001: ENV to database write-back
- tc_u_wb_002: ENV to TOML write-back
- tc_u_wb_003: TOML to database write-back (no TOML write)
- tc_u_wb_004: UI update to database write-back
- tc_u_wb_005: UI update to TOML write-back
- tc_u_wb_006: TOML write failure graceful degradation

**Validation (3 tests):**
- tc_u_val_001: Empty key rejected
- tc_u_val_002: Whitespace-only key rejected
- tc_u_val_003: NULL key rejected

**TOML Utilities (7 tests):**
- tc_u_toml_001: Atomic write creates temp file
- tc_u_toml_002: Atomic write renames to target
- tc_u_toml_003: Atomic write preserves existing fields
- tc_u_toml_004: Atomic write sets permissions 0600 (Unix)
- tc_u_toml_005: Atomic write graceful on Windows (no chmod)
- tc_u_toml_006: Round-trip serialization preserves data
- tc_u_toml_007: Corrupt temp file does not overwrite target

**Database Accessors (2 tests):**
- tc_u_db_001: get_acoustid_api_key returns value
- tc_u_db_002: set_acoustid_api_key writes value

**Security (2 tests):**
- tc_u_sec_001: Permission check detects loose permissions (Unix)
- tc_u_sec_002: Permission warning logged for loose permissions

### Integration Tests (10 tests)

**End-to-End Resolution (4 tests):**
- tc_i_e2e_001: wkmp-ai startup with database key
- tc_i_e2e_002: wkmp-ai startup with ENV migration
- tc_i_e2e_003: wkmp-ai startup with TOML migration
- tc_i_e2e_004: wkmp-ai startup error on no key

**Web UI Endpoint (3 tests):**
- tc_i_ui_001: POST /api/settings/acoustid_api_key success
- tc_i_ui_002: POST with empty key returns error
- tc_i_ui_003: POST writes to database and TOML

**Database Recovery (2 tests):**
- tc_i_recovery_001: Database deletion recovers from TOML
- tc_i_recovery_002: Database deletion with no TOML fails gracefully

**Concurrency (1 test):**
- tc_i_concurrent_001: Multiple module startup TOML reads safe

### System Tests (3 tests)

**User Workflows (3 tests):**
- tc_s_workflow_001: New user configures key via web UI
- tc_s_workflow_002: Developer uses ENV variable for CI/CD
- tc_s_workflow_003: Database deletion recovery workflow

### Manual Tests (6 tests)

**Migration Scenarios (3 tests):**
- tc_m_migration_001: ENV → Database + TOML migration
- tc_m_migration_002: TOML → Database migration
- tc_m_migration_003: Web UI save and verification

**Failure Scenarios (3 tests):**
- tc_m_failure_001: Read-only TOML filesystem
- tc_m_failure_002: Permission warnings on loose permissions
- tc_m_failure_003: Invalid key error messages

---

## Test Files

### Unit Test Files (tc_u_*.md)

**Resolution Tests:**
- tc_u_res_001_to_008.md (8 tests, multi-tier resolution)

**Write-Back Tests:**
- tc_u_wb_001_to_006.md (6 tests, write-back behavior)

**Validation Tests:**
- tc_u_val_001_to_003.md (3 tests, key validation)

**TOML Utility Tests:**
- tc_u_toml_001_to_007.md (7 tests, atomic write and preservation)

**Database Tests:**
- tc_u_db_001_to_002.md (2 tests, accessor functions)

**Security Tests:**
- tc_u_sec_001_to_002.md (2 tests, permission checks)

### Integration Test Files (tc_i_*.md)

- tc_i_e2e_001_to_004.md (4 tests, end-to-end startup)
- tc_i_ui_001_to_003.md (3 tests, web UI endpoint)
- tc_i_recovery_001_to_002.md (2 tests, database recovery)
- tc_i_concurrent_001.md (1 test, concurrency)

### System Test Files (tc_s_*.md)

- tc_s_workflow_001_to_003.md (3 tests, user workflows)

### Manual Test Files (tc_m_*.md)

- tc_m_migration_001_to_003.md (3 tests, migration scenarios)
- tc_m_failure_001_to_003.md (3 tests, failure scenarios)

---

## Traceability Matrix Preview

**Requirement Categories → Test Coverage:**

| Category | Requirements | Unit Tests | Integration Tests | System Tests | Manual Tests | Total Coverage |
|----------|--------------|------------|-------------------|--------------|--------------|----------------|
| Priority Resolution | 5 | 8 | 4 | 1 | 2 | 100% |
| Write-Back Behavior | 4 | 6 | 1 | 0 | 1 | 100% |
| TOML Persistence | 5 | 7 | 0 | 0 | 1 | 100% |
| Validation | 2 | 3 | 1 | 0 | 0 | 100% |
| Error Handling | 3 | 1 | 1 | 0 | 1 | 100% |
| Architecture | 3 | 2 | 1 | 1 | 0 | 100% |
| Generic Sync | 3 | 2 | 1 | 0 | 0 | 100% |
| AcoustID Specific | 4 | 2 | 4 | 2 | 3 | 100% |
| TOML Schema | 2 | 3 | 0 | 0 | 0 | 100% |
| Database Storage | 3 | 2 | 2 | 0 | 0 | 100% |
| Atomic TOML Write | 2 | 7 | 0 | 0 | 1 | 100% |
| Security | 9 | 2 | 0 | 0 | 2 | 100% |
| Web UI Integration | 6 | 0 | 3 | 1 | 1 | 100% |
| Logging | 4 | 0 | 4 | 1 | 1 | 100% |
| Testing Reqs | 3 | N/A | N/A | N/A | N/A | Meta |
| Migration | 3 | 0 | 2 | 1 | 2 | 100% |
| Future Extensions | 5 | N/A | N/A | N/A | N/A | Out of scope |

**Total Coverage:** 100% (all in-scope requirements have at least one test)

---

## Test Execution Order

**Phase 1: Unit Tests (Foundation)**
1. TOML utilities (tc_u_toml_*)
2. Database accessors (tc_u_db_*)
3. Validation (tc_u_val_*)
4. Resolution logic (tc_u_res_*)
5. Write-back logic (tc_u_wb_*)
6. Security checks (tc_u_sec_*)

**Phase 2: Integration Tests**
1. End-to-end startup (tc_i_e2e_*)
2. Web UI endpoint (tc_i_ui_*)
3. Database recovery (tc_i_recovery_*)
4. Concurrency (tc_i_concurrent_*)

**Phase 3: System Tests**
1. User workflows (tc_s_workflow_*)

**Phase 4: Manual Tests**
1. Migration scenarios (tc_m_migration_*)
2. Failure scenarios (tc_m_failure_*)

---

## Test Data Requirements

**Test Databases:**
- In-memory SQLite for unit tests (`:memory:`)
- Temporary file databases for integration tests
- Real database for manual tests (can be deleted)

**Test TOML Files:**
- Temporary directory for unit/integration tests
- Real config file for manual tests (~/.config/wkmp/wkmp-ai.toml)

**Test Environment Variables:**
- Set/unset WKMP_ACOUSTID_API_KEY in test fixtures
- Cleanup after each test

**Test API Keys:**
- Mock keys for unit/integration tests ("test-key-123")
- Real AcoustID key for manual tests (obtain from acoustid.org)

---

## Acceptance Criteria Mapping

**All 21 acceptance criteria covered by tests:**

1. Multi-tier resolution works → tc_u_res_001-008, tc_i_e2e_001-004
2. Database is authoritative → tc_u_res_005-006
3. Auto-migration ENV → tc_u_wb_001-002, tc_i_e2e_002, tc_m_migration_001
4. Auto-migration TOML → tc_u_wb_003, tc_i_e2e_003, tc_m_migration_002
5. TOML write-back works → tc_u_wb_004-005, tc_i_ui_003
6. TOML write is atomic → tc_u_toml_001-002, tc_u_toml_007
7. TOML preserves fields → tc_u_toml_003, tc_u_toml_006
8. TOML permissions 0600 → tc_u_toml_004, tc_m_failure_002
9. TOML write graceful → tc_u_wb_006, tc_m_failure_001
10. Generic sync works → tc_u_wb_004-005, tc_i_ui_003
11. Web UI endpoint works → tc_i_ui_001-003
12. Web UI page works → tc_s_workflow_001, tc_m_migration_003
13. Unit tests pass → All tc_u_* tests
14. Integration tests pass → All tc_i_* tests
15. Manual testing complete → All tc_m_* tests
16. Documentation updated → (Checked in implementation phase)
17. Logging observable → tc_i_e2e_001-004, tc_m_migration_001-003
18. Error messages clear → tc_i_e2e_004, tc_m_failure_003
19. Security warnings work → tc_u_sec_002, tc_m_failure_002
20. Backward compatible → tc_i_e2e_001 (existing database/TOML unchanged)
21. Extensibility demonstrated → tc_u_wb_004-005 (HashMap interface)

**Coverage:** 100% (all acceptance criteria → tests)

---

**Test Index:** Complete
**Next Step:** Create individual test specification files
