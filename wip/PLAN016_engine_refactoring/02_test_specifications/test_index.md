# Test Index: PLAN016 Engine Refactoring

**Plan:** PLAN016
**Feature:** Engine Refactoring
**Total Tests:** 11 (3 unit, 3 integration, 5 system/manual)
**Coverage:** 100% - All 3 requirements have acceptance tests

---

## Test Summary by Requirement

| Requirement | Unit Tests | Integration Tests | System Tests | Total | Coverage |
|-------------|------------|-------------------|--------------|-------|----------|
| REQ-DEBT-QUALITY-002-010 (Module Split) | 1 | 1 | 2 | 4 | Complete |
| REQ-DEBT-QUALITY-002-020 (Line Limits) | 1 | 0 | 1 | 2 | Complete |
| REQ-DEBT-QUALITY-002-030 (API Stability) | 1 | 2 | 2 | 5 | Complete |

**Total:** 11 tests covering 3 requirements

---

## All Tests Quick Reference

### Module Structure Tests (REQ-010)

| Test ID | Type | Description | Pass Criteria |
|---------|------|-------------|---------------|
| TC-U-010-01 | Unit | Directory structure validation | 4 files exist in correct locations |
| TC-I-010-01 | Integration | Module compilation | All modules compile without errors |
| TC-S-010-01 | System | File count verification | Exactly 4 files (mod.rs + 3 modules) |
| TC-S-010-02 | Manual | Code organization review | Logical grouping by responsibility |

### Line Count Tests (REQ-020)

| Test ID | Type | Description | Pass Criteria |
|---------|------|-------------|---------------|
| TC-U-020-01 | Unit | Line count measurement | All 4 files < 1500 lines |
| TC-S-020-01 | System | Total line preservation | Total lines ≈ original ±5% |

### API Stability Tests (REQ-030)

| Test ID | Type | Description | Pass Criteria |
|---------|------|-------------|---------------|
| TC-U-030-01 | Unit | Public API surface check | All pub items preserved |
| TC-I-030-01 | Integration | Handler compilation | handlers.rs compiles unchanged |
| TC-I-030-02 | Integration | Test suite pass rate | 100% baseline tests pass |
| TC-S-030-01 | System | API compatibility verification | Full public API exercised |
| TC-S-030-02 | Manual | External caller review | No code changes in callers |

---

## Test Execution Order

**Pre-Refactoring (Baseline):**
1. TC-S-020-01 - Measure original line count
2. TC-U-030-01 - Document public API surface
3. TC-I-030-02 - Run test suite, record pass/fail

**Post-Refactoring (Verification):**
1. TC-U-010-01 - Verify directory structure
2. TC-I-010-01 - Verify compilation
3. TC-U-020-01 - Verify line counts
4. TC-U-030-01 - Verify API surface unchanged
5. TC-I-030-01 - Verify handlers compile
6. TC-I-030-02 - Verify tests pass
7. TC-S-010-01 - Count files
8. TC-S-020-01 - Compare total lines
9. TC-S-030-01 - Exercise full API
10. TC-S-010-02 - Manual review (code organization)
11. TC-S-030-02 - Manual review (caller changes)

**Estimated Execution Time:** ~30 minutes (automated) + ~15 minutes (manual reviews)

---

## Test Coverage Matrix

| Requirement | Acceptance Criteria | Test IDs | Status |
|-------------|---------------------|----------|--------|
| REQ-DEBT-QUALITY-002-010 | 3 functional modules + 1 interface | TC-U-010-01, TC-I-010-01 | Defined |
| REQ-DEBT-QUALITY-002-010 | Logical code organization | TC-S-010-02 | Defined |
| REQ-DEBT-QUALITY-002-020 | Each file < 1500 lines | TC-U-020-01 | Defined |
| REQ-DEBT-QUALITY-002-020 | Total lines preserved | TC-S-020-01 | Defined |
| REQ-DEBT-QUALITY-002-030 | Public API unchanged | TC-U-030-01, TC-S-030-01 | Defined |
| REQ-DEBT-QUALITY-002-030 | Handlers compile unchanged | TC-I-030-01 | Defined |
| REQ-DEBT-QUALITY-002-030 | Tests pass unchanged | TC-I-030-02 | Defined |

**Coverage:** 100% - All acceptance criteria have tests

---

## Detailed Test Specifications

**See individual test files:**
- [tc_u_010_01.md](tc_u_010_01.md) - Directory structure
- [tc_i_010_01.md](tc_i_010_01.md) - Module compilation
- [tc_s_010_01.md](tc_s_010_01.md) - File count
- [tc_s_010_02.md](tc_s_010_02.md) - Code organization review
- [tc_u_020_01.md](tc_u_020_01.md) - Line count measurement
- [tc_s_020_01.md](tc_s_020_01.md) - Total line preservation
- [tc_u_030_01.md](tc_u_030_01.md) - Public API surface
- [tc_i_030_01.md](tc_i_030_01.md) - Handler compilation
- [tc_i_030_02.md](tc_i_030_02.md) - Test suite pass rate
- [tc_s_030_01.md](tc_s_030_01.md) - API compatibility
- [tc_s_030_02.md](tc_s_030_02.md) - External caller review

---

**Test Index Complete**
**Phase 3 Status:** Index created, individual tests next
