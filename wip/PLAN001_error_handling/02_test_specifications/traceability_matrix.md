# Traceability Matrix - Error Handling Tests

**Plan:** PLAN001_error_handling
**Purpose:** Verify 100% test coverage for all 19 requirements
**Date:** 2025-10-26

---

## Coverage Summary

| Requirement Category | Requirements | Tests | Coverage |
|----------------------|--------------|-------|----------|
| Error Handling | 13 | 27 | 100% ✓ |
| Degradation | 3 | 6 | 100% ✓ |
| Events | 2 | 4 | 100% ✓ |
| Logging | 2 | 4 | 100% ✓ |
| **TOTAL** | **19** | **47** | **100% ✓** |

---

## Complete Traceability Matrix

| Requirement | Unit Tests | Integration Tests | System Tests | Implementation | Status | Coverage |
|-------------|------------|-------------------|--------------|----------------|--------|----------|
| **REQ-AP-ERR-010** | TC-U-ERR-010-01 (file not found)<br>TC-U-ERR-010-02 (permission denied) | TC-I-ERR-010-01 (skip passage) | TC-S-RECOVERY-001 | TBD | Pending | ✓ Complete |
| **REQ-AP-ERR-011** | TC-U-ERR-011-01 (codec detection) | TC-I-ERR-011-01 (mark in DB) | TC-S-RECOVERY-001 | TBD | Pending | ✓ Complete |
| **REQ-AP-ERR-012** | TC-U-ERR-012-01 (≥50% plays)<br>TC-U-ERR-012-02 (<50% skips) | — | TC-S-RECOVERY-001 | TBD | Pending | ✓ Complete |
| **REQ-AP-ERR-013** | TC-U-ERR-013-01 (panic recovery) | — | TC-S-RECOVERY-001 | TBD | Pending | ✓ Complete |
| **REQ-AP-ERR-020** | TC-U-ERR-020-01 (underrun detect) | TC-I-ERR-020-01 (emergency refill)<br>TC-I-ERR-020-02 (timeout fallback) | TC-S-RECOVERY-001 | TBD | Pending | ✓ Complete |
| **REQ-AP-ERR-030** | TC-U-ERR-030-01 (disconnect detect) | TC-I-ERR-030-01 (retry sequence)<br>TC-I-ERR-030-02 (30s timeout) | TC-S-RECOVERY-001<br>TC-S-DEGRADE-001 | TBD | Pending | ✓ Complete |
| **REQ-AP-ERR-031** | TC-U-ERR-031-01 (config error detect) | TC-I-ERR-031-01 (4 fallbacks)<br>TC-I-ERR-031-02 (all fail) | TC-S-RECOVERY-001<br>TC-S-DEGRADE-001 | TBD | Pending | ✓ Complete |
| **REQ-AP-ERR-040** | TC-U-ERR-040-01 (invalid detect) | TC-I-ERR-040-01 (auto-remove)<br>TC-I-ERR-040-02 (validation on load) | TC-S-RECOVERY-001 | TBD | Pending | ✓ Complete |
| **REQ-AP-ERR-050** | TC-U-ERR-050-01 (init fail detect) | TC-I-ERR-050-01 (bypass if same rate) | TC-S-RECOVERY-001 | TBD | Pending | ✓ Complete |
| **REQ-AP-ERR-051** | TC-U-ERR-051-01 (runtime error detect) | TC-I-ERR-051-01 (skip passage) | TC-S-RECOVERY-001 | TBD | Pending | ✓ Complete |
| **REQ-AP-ERR-060** | TC-U-ERR-060-01 (<100 samples log)<br>TC-U-ERR-060-02 (≥100 resync)<br>TC-U-ERR-060-03 (>1s skip) | — | TC-S-RECOVERY-001 | TBD | Pending | ✓ Complete |
| **REQ-AP-ERR-070** | TC-U-ERR-070-01 (OOM detect+cleanup) | TC-I-ERR-070-01 (retry after cleanup) | TC-S-RECOVERY-001 | TBD | Pending | ✓ Complete |
| **REQ-AP-ERR-071** | TC-U-ERR-071-01 (handle exhaustion detect) | TC-I-ERR-071-01 (chain reduction) | TC-S-RECOVERY-001 | TBD | Pending | ✓ Complete |
| **REQ-AP-DEGRADE-010** | — | TC-I-DEGRADE-010-01 (queue after decode error)<br>TC-I-DEGRADE-010-02 (queue after device error) | TC-S-DEGRADE-001 | TBD | Pending | ✓ Complete |
| **REQ-AP-DEGRADE-020** | — | TC-I-DEGRADE-020-01 (position after underrun)<br>TC-I-DEGRADE-020-02 (position after reconnect) | TC-S-DEGRADE-001 | TBD | Pending | ✓ Complete |
| **REQ-AP-DEGRADE-030** | — | TC-I-DEGRADE-030-01 (controls in degraded mode) | TC-S-DEGRADE-001 | TBD | Pending | ✓ Complete |
| **REQ-AP-EVENT-ERR-010** | TC-U-EVENT-010-01 (all error types emit) | TC-I-EVENT-010-01 (events reach SSE) | TC-S-EVENT-001<br>TC-S-RECOVERY-001 | TBD | Pending | ✓ Complete |
| **REQ-AP-EVENT-ERR-020** | TC-U-EVENT-020-01 (field completeness) | — | TC-S-EVENT-001<br>TC-S-RECOVERY-001 | TBD | Pending | ✓ Complete |
| **REQ-AP-LOG-ERR-010** | TC-U-LOG-010-01 (log levels per type)<br>TC-U-LOG-010-02 (configurable levels) | — | TC-S-LOG-001 | TBD | Pending | ✓ Complete |
| **REQ-AP-LOG-ERR-020** | TC-U-LOG-020-01 (structured format) | — | TC-S-LOG-001 | TBD | Pending | ✓ Complete |

---

## Test Distribution by Requirement

### High Test Density (≥4 tests)

| Requirement | Test Count | Rationale |
|-------------|------------|-----------|
| REQ-AP-ERR-020 | 4 | Buffer underrun is complex (detect, refill, timeout, recovery) |
| REQ-AP-ERR-030 | 5 | Device disconnect critical for user experience |
| REQ-AP-ERR-031 | 5 | Device config fallback sequence complex |
| REQ-AP-ERR-060 | 4 | Position drift has 3 threshold levels |
| REQ-AP-EVENT-ERR-010 | 4 | 25 event variants to verify |

### Standard Test Density (2-3 tests)

Most requirements: Adequate coverage with unit + integration + system tests

### Low Test Density (1-2 tests)

| Requirement | Test Count | Rationale |
|-------------|------------|-----------|
| REQ-AP-ERR-012 | 3 | Binary threshold (≥50% vs <50%) |
| REQ-AP-ERR-013 | 2 | Panic recovery is straightforward |

---

## Test Types by Requirement Category

### Decode Errors (REQ-AP-ERR-010 through REQ-AP-ERR-013)

- **Unit Tests:** 6 tests (detection of file errors, codec errors, truncation, panics)
- **Integration Tests:** 2 tests (skip passage flow, database marking)
- **System Tests:** 1 test (TC-S-RECOVERY-001 covers all)
- **Total:** 9 tests

**Coverage Justification:** Each error type has detection test + at least one recovery test.

### Buffer Errors (REQ-AP-ERR-020)

- **Unit Tests:** 1 test (underrun detection)
- **Integration Tests:** 2 tests (emergency refill, timeout fallback)
- **System Tests:** 1 test (TC-S-RECOVERY-001)
- **Total:** 4 tests

**Coverage Justification:** Complex recovery sequence requires multiple integration tests.

### Device Errors (REQ-AP-ERR-030, REQ-AP-ERR-031)

- **Unit Tests:** 2 tests (disconnect detection, config error detection)
- **Integration Tests:** 4 tests (retry sequence, timeout, 4 fallbacks, all fail)
- **System Tests:** 2 tests (TC-S-RECOVERY-001, TC-S-DEGRADE-001)
- **Total:** 8 tests

**Coverage Justification:** Device handling is user-facing critical path.

### Queue Errors (REQ-AP-ERR-040)

- **Unit Tests:** 1 test (invalid detection)
- **Integration Tests:** 2 tests (auto-remove, validation on load)
- **System Tests:** 1 test (TC-S-RECOVERY-001)
- **Total:** 4 tests

**Coverage Justification:** Queue integrity is critical for system stability.

### Resampling Errors (REQ-AP-ERR-050, REQ-AP-ERR-051)

- **Unit Tests:** 2 tests (init failure, runtime error)
- **Integration Tests:** 2 tests (bypass if same rate, skip passage)
- **System Tests:** 1 test (TC-S-RECOVERY-001)
- **Total:** 5 tests

**Coverage Justification:** Resampling failures need bypass logic verification.

### Timing Errors (REQ-AP-ERR-060)

- **Unit Tests:** 3 tests (three threshold levels: <100, ≥100, >1s)
- **Integration Tests:** 0 (unit tests sufficient)
- **System Tests:** 1 test (TC-S-RECOVERY-001)
- **Total:** 4 tests

**Coverage Justification:** Each threshold level has distinct behavior.

### Resource Errors (REQ-AP-ERR-070, REQ-AP-ERR-071)

- **Unit Tests:** 2 tests (OOM detection, handle exhaustion)
- **Integration Tests:** 2 tests (OOM retry, chain reduction)
- **System Tests:** 1 test (TC-S-RECOVERY-001)
- **Total:** 5 tests

**Coverage Justification:** Resource management requires cleanup + retry verification.

### Degradation (REQ-AP-DEGRADE-010/020/030)

- **Unit Tests:** 0 (integration-level behavior)
- **Integration Tests:** 5 tests (queue integrity, position preservation, controls)
- **System Tests:** 1 test (TC-S-DEGRADE-001)
- **Total:** 6 tests

**Coverage Justification:** Degradation is system-wide behavior requiring integration tests.

### Events (REQ-AP-EVENT-ERR-010/020)

- **Unit Tests:** 2 tests (emission verification, field completeness)
- **Integration Tests:** 1 test (SSE delivery)
- **System Tests:** 2 tests (TC-S-EVENT-001, TC-S-RECOVERY-001)
- **Total:** 5 tests

**Coverage Justification:** Event system spans all error types - comprehensive verification needed.

### Logging (REQ-AP-LOG-ERR-010/020)

- **Unit Tests:** 3 tests (log levels, configurability, structured format)
- **Integration Tests:** 0 (logging is unit-testable)
- **System Tests:** 1 test (TC-S-LOG-001)
- **Total:** 4 tests

**Coverage Justification:** Logging structure and levels verified per error type.

---

## Uncovered Scenarios (Intentional Gaps)

### None Identified

All 19 requirements have comprehensive test coverage across unit, integration, and system test levels.

---

## Test Dependency Graph

```
Unit Tests (run first - no dependencies)
    ↓
Integration Tests (require components from Phases 1-6)
    ↓
System Tests (require complete wkmp-ap stack)
```

**Critical Path:**
1. Error detection unit tests MUST pass before integration tests
2. Individual error recovery integration tests MUST pass before system stress tests
3. All tests MUST pass before Phase 7 considered complete

---

## Implementation Tracking

**Test Implementation Progress:** 0/47 (0%)

**Test Status Legend:**
- **Defined:** Test specification complete (current status for all tests)
- **Implemented:** Test code written
- **Passing:** Test executes and passes
- **Verified:** Test independently reviewed and confirmed correct

**Implementation Order:**
1. Unit tests (TC-U-*) - 23 tests
2. Integration tests (TC-I-*) - 19 tests
3. System tests (TC-S-*) - 5 tests

**Estimated Implementation Time:**
- Unit tests: ~6 hours (23 tests × 15 min avg)
- Integration tests: ~9.5 hours (19 tests × 30 min avg)
- System tests: ~5 hours (5 tests × 60 min avg)
- **Total:** ~20.5 hours

---

## Verification Checklist

Before marking Phase 7 complete, verify:

- [ ] All 47 tests implemented
- [ ] All tests passing
- [ ] 100% requirement coverage maintained (traceability matrix)
- [ ] No untested error paths
- [ ] All 25 event types verified
- [ ] All log levels and formats verified
- [ ] Error injection framework functional
- [ ] Test data available (valid/invalid audio files)
- [ ] Test execution automated (CI integration)
- [ ] Test results documented

---

**Matrix Version:** 1.0
**Last Updated:** 2025-10-26
**Coverage Status:** ✓ 100% Complete (all 19 requirements covered)
**Implementation Status:** Pending (0/47 tests implemented)
