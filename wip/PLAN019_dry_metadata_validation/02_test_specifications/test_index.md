# PLAN019: Test Index

**Purpose:** Quick reference table of all acceptance tests
**Total Tests:** 13 (7 unit, 5 integration, 0 system, 1 manual)
**Coverage:** 100% - All 10 requirements have acceptance tests

---

## Test Summary

| Test ID | Type | Requirement | Brief Description | Estimated Effort |
|---------|------|-------------|-------------------|------------------|
| TC-U-010-01 | Unit | REQ-DRY-010 | ParamMetadata struct has 6 fields | 10 min |
| TC-U-010-02 | Unit | REQ-DRY-010 | All 15 parameters defined in metadata | 15 min |
| TC-U-020-01 | Unit | REQ-DRY-020 | metadata() returns static reference | 10 min |
| TC-U-030-01 | Unit | REQ-DRY-030 | Volume level validator (example) | 20 min |
| TC-U-030-02 | Unit | REQ-DRY-030 | All 15 validators tested (batch) | 30 min |
| TC-I-040-01 | Integration | REQ-DRY-040 | Database loading uses metadata | 15 min |
| TC-U-050-01 | Unit | REQ-DRY-050 | Setters delegate to validators | 15 min |
| TC-I-060-01 | Integration | REQ-DRY-060 | API rejects invalid setting (single) | 20 min |
| TC-I-060-02 | Integration | REQ-DRY-060 | API batch error reporting (3 errors) | 20 min |
| TC-I-070-01 | Integration | REQ-DRY-070 | Database unchanged after validation fail | 20 min |
| TC-I-080-01 | Integration | REQ-DRY-080 | Volume functions use metadata | 15 min |
| TC-U-090-01 | Unit | REQ-DRY-090 | All 24 existing tests pass | 5 min (auto) |
| TC-U-090-02 | Unit | REQ-DRY-090 | All 10 new tests pass | 5 min (auto) |
| TC-M-100-01 | Manual | REQ-DRY-100 | Documentation review (3 locations) | 15 min |

**Total Estimated Effort:** 3.5 hours (test writing + implementation)

---

## Tests by Type

### Unit Tests (7)
- TC-U-010-01: ParamMetadata struct definition
- TC-U-010-02: All 15 parameters in metadata
- TC-U-020-01: metadata() accessor
- TC-U-030-01: Volume level validator
- TC-U-030-02: All 15 validators batch test
- TC-U-050-01: Setter delegation
- TC-U-090-01/02: Test suite verification

### Integration Tests (5)
- TC-I-040-01: Database loading with metadata
- TC-I-060-01/02: API validation (single + batch)
- TC-I-070-01: Database integrity
- TC-I-080-01: Volume refactor

### System Tests (0)
- None required (refactoring, not new features)

### Manual Tests (1)
- TC-M-100-01: Documentation review

---

## Tests by Requirement

| Requirement | Tests | Count |
|-------------|-------|-------|
| REQ-DRY-010 | TC-U-010-01, TC-U-010-02 | 2 |
| REQ-DRY-020 | TC-U-020-01 | 1 |
| REQ-DRY-030 | TC-U-030-01, TC-U-030-02 | 2 |
| REQ-DRY-040 | TC-I-040-01 | 1 |
| REQ-DRY-050 | TC-U-050-01 | 1 |
| REQ-DRY-060 | TC-I-060-01, TC-I-060-02 | 2 |
| REQ-DRY-070 | TC-I-070-01 | 1 |
| REQ-DRY-080 | TC-I-080-01 | 1 |
| REQ-DRY-090 | TC-U-090-01, TC-U-090-02 | 2 |
| REQ-DRY-100 | TC-M-100-01 | 1 |

---

## Test Execution Order

**Recommended order for test-first implementation:**

### Phase 1: Metadata Infrastructure
1. TC-U-010-01 (struct compiles)
2. TC-U-010-02 (15 parameters present)
3. TC-U-020-01 (accessor works)
4. TC-U-030-01 (example validator)
5. TC-U-030-02 (all validators)

**After Phase 1:** Metadata system complete and tested

### Phase 2: Refactoring
6. TC-I-040-01 (database loading)
7. TC-U-050-01 (setter delegation)
8. TC-I-080-01 (volume refactor)

**After Phase 2:** All duplication eliminated

### Phase 3: API Validation (CRITICAL)
9. TC-I-060-01 (API single error)
10. TC-I-060-02 (API batch errors)
11. TC-I-070-01 (database integrity)

**After Phase 3:** Database corruption prevented

### Phase 4: Verification
12. TC-U-090-01 (existing tests pass)
13. TC-U-090-02 (new tests pass)
14. TC-M-100-01 (documentation review)

**After Phase 4:** Complete and verified

---

## Test Files Location

All tests added to existing test modules:

**Unit Tests:**
- `wkmp-common/src/params.rs` - tests module (lines 552+)
- Add ~200 lines of test code

**Integration Tests:**
- `wkmp-ap/src/api/handlers.rs` - tests module (if exists) OR
- `wkmp-ap/tests/api_validation_tests.rs` - new integration test file
- Add ~150 lines of test code

**Total Test Code:** ~350 lines

---

## Pass Criteria Summary

**All tests must pass for plan to be considered complete:**

✅ **24 existing tests** - No regressions
✅ **13 new tests** - All validation paths covered
✅ **37 total tests** passing (100% coverage)

**Acceptance:**
- Run `cargo test -p wkmp-common` → 24+ tests pass
- Run `cargo test -p wkmp-ap` → Integration tests pass
- Manual doc review complete

---

## Test Data Requirements

### Unit Tests
- No external data required (in-memory test structs)

### Integration Tests
- **TC-I-040-01:** In-memory SQLite database (`:memory:`)
- **TC-I-060-01/02:** Mock AppContext with test database
- **TC-I-070-01:** Test database with pre-populated settings
- **TC-I-080-01:** Test database for volume functions

### Manual Tests
- **TC-M-100-01:** No data required (code review)

---

## Quick Reference: Test by Increment

### Increment 1: Metadata Infrastructure
Tests: TC-U-010-01, TC-U-010-02, TC-U-020-01, TC-U-030-01, TC-U-030-02 (5 tests)

### Increment 2: Database Loading
Tests: TC-I-040-01 (1 test)

### Increment 3: Setter Methods
Tests: TC-U-050-01 (1 test)

### Increment 4: API Validation
Tests: TC-I-060-01, TC-I-060-02, TC-I-070-01 (3 tests)

### Increment 5: Volume Refactor
Tests: TC-I-080-01 (1 test)

### Increment 6: Documentation + Verification
Tests: TC-U-090-01, TC-U-090-02, TC-M-100-01 (3 tests)

---

## Notes

**Test Execution Environment:**
- Rust stable channel
- `cargo test` with `serial_test` crate for database tests
- Use `#[serial_test::serial]` for tests that modify PARAMS singleton
- In-memory SQLite for fast integration tests

**Test Coverage Tools:**
- Optional: `cargo-tarpaulin` for coverage metrics
- Target: 100% coverage of new metadata code
- Existing coverage: 68 test modules (from critique)

**Continuous Integration:**
- All tests must pass before committing
- Run `cargo test --all` before each increment completion
