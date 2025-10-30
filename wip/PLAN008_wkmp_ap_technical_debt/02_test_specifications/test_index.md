# Test Index - wkmp-ap Technical Debt Remediation

**Plan:** PLAN008
**Total Tests:** 28 tests covering 37 requirements
**Coverage:** 100%

---

## Test Quick Reference

### Security Tests (6 tests)

| Test ID | Type | Requirement(s) | One-Line Description |
|---------|------|----------------|----------------------|
| TC-SEC-001-01 | Unit | SEC-001-010/020/030 | POST with valid secret succeeds |
| TC-SEC-001-02 | Unit | SEC-001-010/030/040 | POST with invalid secret returns 401 |
| TC-SEC-001-03 | Unit | SEC-001-010/030/040 | POST without secret returns 401 |
| TC-SEC-001-04 | Unit | SEC-001-010/020/030 | PUT with valid secret succeeds |
| TC-SEC-001-05 | Unit | SEC-001-030/040 | POST with malformed JSON returns 401 |
| TC-SEC-001-06 | Unit | SEC-001-030/040 | POST with wrong secret type returns 401 |

### Decoder Error Tests (3 tests)

| Test ID | Type | Requirement(s) | One-Line Description |
|---------|------|----------------|----------------------|
| TC-FUNC-001-01 | Unit | FUNC-001-010/020/030 | Decoder error includes file path |
| TC-FUNC-001-02 | Unit | FUNC-001-010/030 | Decode error for corrupt file includes path |
| TC-FUNC-001-03 | Unit | FUNC-001-010/030 | Multiple decoder errors show correct paths |

### Buffer Config Tests (4 tests)

| Test ID | Type | Requirement(s) | One-Line Description |
|---------|------|----------------|----------------------|
| TC-FUNC-002-01 | Unit | FUNC-002-010/020/040 | BufferManager reads custom settings from DB |
| TC-FUNC-002-02 | Unit | FUNC-002-030 | BufferManager uses defaults when settings NULL |
| TC-FUNC-002-03 | Unit | FUNC-002-030 | BufferManager validates and rejects invalid settings |
| TC-FUNC-002-04 | Integration | FUNC-002-010/020/030 | End-to-end buffer config flow |

### Telemetry Tests (4 tests)

| Test ID | Type | Requirement(s) | One-Line Description |
|---------|------|----------------|----------------------|
| TC-FUNC-003-01 | Unit | FUNC-003-010 | Buffer chain info includes decoder state |
| TC-FUNC-003-02 | Unit | FUNC-003-020 | Buffer chain info includes source sample rate |
| TC-FUNC-003-03 | Unit | FUNC-003-030 | Buffer chain info includes fade stage |
| TC-FUNC-003-04 | Integration | FUNC-003-010/020/030/040 | Complete telemetry in developer UI |

### Album Metadata Tests (3 tests)

| Test ID | Type | Requirement(s) | One-Line Description |
|---------|------|----------------|----------------------|
| TC-FUNC-004-01 | Unit | FUNC-004-030 | Query returns passage album UUIDs |
| TC-FUNC-004-02 | Integration | FUNC-004-010 | PassageStarted event includes albums |
| TC-FUNC-004-03 | Integration | FUNC-004-020 | PassageComplete event includes albums |

### Duration Tracking Tests (3 tests)

| Test ID | Type | Requirement(s) | One-Line Description |
|---------|------|----------------|----------------------|
| TC-FUNC-005-01 | Unit | FUNC-005-010 | Mixer tracks passage start time |
| TC-FUNC-005-02 | Unit | FUNC-005-020/030 | Duration calculated with millisecond precision |
| TC-FUNC-005-03 | Integration | FUNC-005-020/030/040 | PassageComplete reports accurate duration |

### Code Quality Tests (5 tests)

| Test ID | Type | Requirement(s) | One-Line Description |
|---------|------|----------------|----------------------|
| TC-QUALITY-001-01 | Unit | QUALITY-001-010/020 | Mutex lock errors propagate properly |
| TC-QUALITY-001-02 | Unit | QUALITY-001-030 | Event channel errors handled gracefully |
| TC-QUALITY-002-01 | Build | QUALITY-002-010/020/030 | engine.rs split maintains public API |
| TC-QUALITY-003-01 | Build | QUALITY-003-010/020/030 | Zero compiler warnings after fixes |
| TC-QUALITY-004-01 | Build | QUALITY-004-010/020/030 | Single config module exists |

---

## Test Coverage by Requirement

**Security (4 requirements):** 6 tests
- REQ-DEBT-SEC-001-010: TC-SEC-001-01 through TC-SEC-001-06
- REQ-DEBT-SEC-001-020: TC-SEC-001-01, TC-SEC-001-04
- REQ-DEBT-SEC-001-030: TC-SEC-001-01 through TC-SEC-001-06
- REQ-DEBT-SEC-001-040: TC-SEC-001-02, TC-SEC-001-03, TC-SEC-001-05, TC-SEC-001-06

**Decoder Errors (3 requirements):** 3 tests
- REQ-DEBT-FUNC-001-010: TC-FUNC-001-01, TC-FUNC-001-02, TC-FUNC-001-03
- REQ-DEBT-FUNC-001-020: TC-FUNC-001-01
- REQ-DEBT-FUNC-001-030: TC-FUNC-001-01, TC-FUNC-001-02, TC-FUNC-001-03

**Buffer Config (4 requirements):** 4 tests
- REQ-DEBT-FUNC-002-010: TC-FUNC-002-01, TC-FUNC-002-04
- REQ-DEBT-FUNC-002-020: TC-FUNC-002-01, TC-FUNC-002-04
- REQ-DEBT-FUNC-002-030: TC-FUNC-002-02, TC-FUNC-002-03, TC-FUNC-002-04
- REQ-DEBT-FUNC-002-040: TC-FUNC-002-01

**Telemetry (4 requirements):** 4 tests
- REQ-DEBT-FUNC-003-010: TC-FUNC-003-01, TC-FUNC-003-04
- REQ-DEBT-FUNC-003-020: TC-FUNC-003-02, TC-FUNC-003-04
- REQ-DEBT-FUNC-003-030: TC-FUNC-003-03, TC-FUNC-003-04
- REQ-DEBT-FUNC-003-040: TC-FUNC-003-04

**Album Metadata (3 requirements):** 3 tests
- REQ-DEBT-FUNC-004-010: TC-FUNC-004-02
- REQ-DEBT-FUNC-004-020: TC-FUNC-004-03
- REQ-DEBT-FUNC-004-030: TC-FUNC-004-01

**Duration Tracking (4 requirements):** 3 tests
- REQ-DEBT-FUNC-005-010: TC-FUNC-005-01
- REQ-DEBT-FUNC-005-020: TC-FUNC-005-02, TC-FUNC-005-03
- REQ-DEBT-FUNC-005-030: TC-FUNC-005-02, TC-FUNC-005-03
- REQ-DEBT-FUNC-005-040: TC-FUNC-005-03

**Code Quality (10 requirements):** 5 tests
- REQ-DEBT-QUALITY-001-010: TC-QUALITY-001-01
- REQ-DEBT-QUALITY-001-020: TC-QUALITY-001-01
- REQ-DEBT-QUALITY-001-030: TC-QUALITY-001-02
- REQ-DEBT-QUALITY-002-010: TC-QUALITY-002-01
- REQ-DEBT-QUALITY-002-020: TC-QUALITY-002-01
- REQ-DEBT-QUALITY-002-030: TC-QUALITY-002-01
- REQ-DEBT-QUALITY-003-010: TC-QUALITY-003-01
- REQ-DEBT-QUALITY-003-020: TC-QUALITY-003-01
- REQ-DEBT-QUALITY-003-030: TC-QUALITY-003-01
- REQ-DEBT-QUALITY-004-010: TC-QUALITY-004-01

**Cleanup (3 requirements):** Verified by inspection (no automated tests needed)
- REQ-DEBT-QUALITY-004-020: Manual verification
- REQ-DEBT-QUALITY-004-030: File system check
- REQ-DEBT-QUALITY-005-010: File system check
- REQ-DEBT-QUALITY-005-020: Git history exists

**Future (1 requirement):** Manual verification (log inspection)
- REQ-DEBT-FUTURE-003-010: Manual log inspection during playback

---

## Test Types Distribution

| Type | Count | Percentage |
|------|-------|------------|
| Unit Tests | 18 | 64% |
| Integration Tests | 7 | 25% |
| Build Tests | 3 | 11% |
| Manual Verification | 2 | (not in count) |

**Total Automated Tests:** 28
**Coverage:** 100% of requirements have verification method

---

## Test Execution Order

**Sprint 1 Tests (11 tests):**
1. TC-SEC-001-01 through TC-SEC-001-06 (authentication)
2. TC-FUNC-001-01 through TC-FUNC-001-03 (file paths)
3. TC-FUNC-002-01 through TC-FUNC-002-04 (buffer config)

**Sprint 2 Tests (9 tests):**
1. TC-FUNC-003-01 through TC-FUNC-003-04 (telemetry)
2. TC-FUNC-004-01 through TC-FUNC-004-03 (albums)
3. TC-FUNC-005-01 through TC-FUNC-005-03 (duration)

**Sprint 3 Tests (8 tests):**
1. TC-QUALITY-001-01, TC-QUALITY-001-02 (.unwrap audit)
2. TC-QUALITY-002-01 (refactoring)
3. TC-QUALITY-003-01 (warnings)
4. TC-QUALITY-004-01 (config)
5. Manual verification (clipping logs)

---

## Reading Guide

**For Implementers:**
- Start with this test_index.md (quick reference)
- Read individual test specs as you implement each feature
- Tests are in files: `tc_[category]_[debt#]_[test#].md`
- Each test file is <100 lines (easy to load in context)

**Test File Naming Convention:**
- `tc_sec_001_01.md` = Test Case for Security debt item 001, test 01
- `tc_func_002_03.md` = Test Case for Functionality debt item 002, test 03
- `tc_quality_001_01.md` = Test Case for Quality debt item 001, test 01

---

## Next Steps

**Phase 3 Continuation:**
- Define individual test specifications (28 files)
- Each test follows BDD format (Given/When/Then)
- Include pass/fail criteria for each test
- Estimate effort for each test
