# Traceability Matrix: SPEC017 Compliance Remediation

**Plan:** PLAN017
**Generated:** 2025-11-02
**Coverage Target:** 100%

---

## Requirements → Test Cases Mapping

### REQ-F-001: wkmp-dr Dual Time Display (HIGH)

**Acceptance Criteria:**
- Format: `{ticks} ({seconds}s)` e.g., `141120000 (5.000000s)`
- Applies to 6 timing columns
- Decimal precision: 6 places
- NULL values: display as `null`

**Test Coverage:**

| Test Case | Type | What It Verifies | Status |
|-----------|------|------------------|--------|
| TC-U-001 | Unit | JavaScript conversion function accuracy | ✅ Required |
| TC-I-002 | Integration | Full table rendering with dual format | ✅ Required |
| TC-A-001 | Acceptance | SRC-LAYER-011 compliance (visual + automated) | ✅ Required |

**Coverage:** ✅ **100%** (3 tests covering conversion, rendering, and specification compliance)

---

### REQ-F-002: API Timing Unit Documentation (MEDIUM)

**Acceptance Criteria:**
- Every API timing field has doc comment with unit
- Function parameters use unit suffixes
- SPEC017 updated with "API Layer Pragmatic Deviation" section
- Error messages reference correct units

**Test Coverage:**

| Test Case | Type | What It Verifies | Status |
|-----------|------|------------------|--------|
| TC-A-003 | Acceptance | API doc comments, SPEC017 update, error messages | ✅ Required |

**Coverage:** ✅ **100%** (1 comprehensive manual review test)

---

### REQ-F-003: File Duration Migration to Ticks (MEDIUM)

**Acceptance Criteria:**
- `AudioFile.duration: Option<f64>` → `duration_ticks: Option<i64>`
- Database schema: `duration REAL` → `duration_ticks INTEGER`
- Import converts via `seconds_to_ticks()`
- Breaking change documented

**Test Coverage:**

| Test Case | Type | What It Verifies | Status |
|-----------|------|------------------|--------|
| TC-U-002 | Unit | Roundtrip conversion accuracy | ✅ Required |
| TC-I-001 | Integration | End-to-end import stores ticks | ✅ Required |
| TC-A-002 | Acceptance | System-wide consistency + breaking change docs | ✅ Required |

**Coverage:** ✅ **100%** (3 tests covering conversion, integration, and system consistency)

---

### REQ-F-004: Variable Naming Clarity (LOW)

**Acceptance Criteria:**
- Variables use unit suffixes OR inline comments
- Applies to 2 specific files:
  - wkmp-ap/src/playback/pipeline/timing.rs
  - wkmp-ai/src/services/silence_detector.rs

**Test Coverage:**

| Test Case | Type | What It Verifies | Status |
|-----------|------|------------------|--------|
| Manual Code Review | Manual | Variable naming and comments in 2 files | ✅ Required |

**Coverage:** ✅ **100%** (manual review during implementation)

**Note:** No executable test case - verified via code review checklist during implementation.

---

### REQ-NF-001: Test Coverage (REQUIRED)

**Acceptance Criteria:**
- wkmp-dr: UI rendering test verifies dual display
- wkmp-ai: Database roundtrip test verifies tick storage
- wkmp-ai: File duration conversion test
- Integration test: end-to-end file import

**Test Coverage:**

| Test Case | Type | What It Verifies | Status |
|-----------|------|------------------|--------|
| TC-A-001 | Acceptance | Test coverage adequacy (meta-test) | ✅ Required |
| TC-U-001 | Unit | wkmp-dr conversion (satisfies criterion 1) | ✅ Required |
| TC-U-002 | Unit | File duration conversion (satisfies criterion 3) | ✅ Required |
| TC-I-001 | Integration | Database roundtrip (satisfies criterion 2) | ✅ Required |
| TC-I-002 | Integration | End-to-end import (satisfies criterion 4) | ✅ Required |

**Coverage:** ✅ **100%** (all 4 acceptance criteria satisfied by dedicated tests)

---

### REQ-NF-002: Documentation Updates (REQUIRED)

**Acceptance Criteria:**
- SPEC017 updated (API deviation section)
- IMPL001 updated (duration_ticks schema)
- Code documentation follows standards
- Migration notes written

**Test Coverage:**

| Test Case | Type | What It Verifies | Status |
|-----------|------|------------------|--------|
| TC-A-003 Part 3 | Acceptance | SPEC017 update (deviation section) | ✅ Required |
| TC-A-003 Part 5 | Acceptance | IMPL001 update (schema documentation) | ✅ Required |
| TC-A-003 Part 1-2 | Acceptance | Code documentation (API doc comments) | ✅ Required |
| TC-A-002 Part 3 | Acceptance | Migration notes documentation | ✅ Required |

**Coverage:** ✅ **100%** (all 4 documentation types verified)

---

### REQ-NF-003: Backward Compatibility (REQUIRED)

**Acceptance Criteria:**
- Database rebuild instructions provided
- Users informed (existing databases incompatible)
- No automated migration (acceptable)

**Test Coverage:**

| Test Case | Type | What It Verifies | Status |
|-----------|------|------------------|--------|
| TC-A-002 Part 3 | Acceptance | Breaking change documentation | ✅ Required |
| TC-A-002 Part 1 | Acceptance | Schema incompatibility (old field removed) | ✅ Required |

**Coverage:** ✅ **100%** (breaking change documented and verified)

---

## Summary

### Requirements Coverage

| Requirement | Priority | Test Cases | Coverage |
|-------------|----------|------------|----------|
| REQ-F-001 | HIGH | 3 (TC-U-001, TC-I-002, TC-A-001) | ✅ 100% |
| REQ-F-002 | MEDIUM | 1 (TC-A-003) | ✅ 100% |
| REQ-F-003 | MEDIUM | 3 (TC-U-002, TC-I-001, TC-A-002) | ✅ 100% |
| REQ-F-004 | LOW | Manual review | ✅ 100% |
| REQ-NF-001 | REQUIRED | 5 (all unit + integration tests) | ✅ 100% |
| REQ-NF-002 | REQUIRED | 2 (TC-A-003, TC-A-002) | ✅ 100% |
| REQ-NF-003 | REQUIRED | 1 (TC-A-002) | ✅ 100% |

**Total:** 7/7 requirements (100% coverage)

---

### Test Cases Coverage

| Test Case | Type | Requirements Covered | Priority |
|-----------|------|---------------------|----------|
| TC-U-001 | Unit | REQ-F-001, REQ-NF-001 | HIGH |
| TC-U-002 | Unit | REQ-F-003, REQ-NF-001 | MEDIUM |
| TC-I-001 | Integration | REQ-F-003, REQ-NF-001 | MEDIUM |
| TC-I-002 | Integration | REQ-F-001, REQ-NF-001 | HIGH |
| TC-A-001 | Acceptance | REQ-F-001, REQ-NF-001 | HIGH |
| TC-A-002 | Acceptance | REQ-F-003, REQ-NF-003 | MEDIUM |
| TC-A-003 | Acceptance | REQ-F-002, REQ-NF-002 | MEDIUM |
| Manual Review | Manual | REQ-F-004 | LOW |

**Total:** 7 test cases + 1 manual review (100% coverage)

---

## Test Execution Dependency Graph

```
Unit Tests (Independent, run first):
├─ TC-U-001: wkmp-dr conversion
└─ TC-U-002: File duration roundtrip

Integration Tests (Require database):
├─ TC-I-001: File import ← depends on TC-U-002 passing
└─ TC-I-002: Display rendering ← depends on TC-U-001 passing

Acceptance Tests (Require full system):
├─ TC-A-001: Developer UI compliance ← depends on TC-U-001, TC-I-002
├─ TC-A-002: File duration consistency ← depends on TC-U-002, TC-I-001
└─ TC-A-003: API documentation ← independent (code review)

Manual Review:
└─ REQ-F-004: Variable naming ← during implementation
```

---

## Gap Analysis

**Missing Coverage:** ❌ **NONE** - All requirements have test coverage

**Redundant Coverage:** ✅ **Intentional**
- REQ-F-001 has 3 tests (unit, integration, acceptance) for defense-in-depth
- REQ-F-003 has 3 tests (unit, integration, acceptance) for system-wide verification

**Rationale:** HIGH and MEDIUM priority requirements receive multi-layer testing to reduce risk.

---

## Verification Sign-Off

**Upon successful execution of all tests:**

> "All 7 requirements in SPEC017 Compliance Remediation have been verified through 7 test cases plus manual review. Coverage: 100%. All acceptance criteria met."

**Test Lead:** ___________
**Date:** ___________
**Signature:** ___________

---

## Notes

- **100% coverage achieved:** All 7 requirements tested
- **Test pyramid maintained:** 2 unit, 2 integration, 3 acceptance, 1 manual
- **Automation level:** 7/8 tests automated (87.5%)
- **Risk mitigation:** HIGH priority requirements have 3-layer testing
- **Traceability:** Bidirectional (requirements → tests, tests → requirements)
