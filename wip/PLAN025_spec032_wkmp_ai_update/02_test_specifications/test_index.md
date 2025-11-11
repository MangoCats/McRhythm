# Test Index - PLAN025

**Plan:** SPEC032 wkmp-ai Implementation Update
**Date:** 2025-11-10

---

## Test Summary

**Total Tests:** 32 tests
- **Unit Tests:** 18
- **Integration Tests:** 10
- **System Tests:** 4

**Requirements Coverage:** 12/12 requirements (100%)

---

## Quick Reference Table

| Test ID | Type | Requirement | Brief Description | Priority |
|---------|------|-------------|-------------------|----------|
| **TC-U-PIPE-010-01** | Unit | REQ-PIPE-010 | Verify segmentation executes before fingerprinting | P0 |
| **TC-U-PIPE-020-01** | Unit | REQ-PIPE-020 | Verify 4 concurrent workers created | P0 |
| **TC-I-PIPE-020-01** | Integration | REQ-PIPE-020 | Verify per-file pipeline execution | P0 |
| **TC-U-PATT-010-01** | Unit | REQ-PATT-010 | Verify pattern analyzer accepts segment list | P1 |
| **TC-U-PATT-010-02** | Unit | REQ-PATT-010 | Verify pattern metadata output format | P1 |
| **TC-U-PATT-020-01** | Unit | REQ-PATT-020 | Verify track count detection | P2 |
| **TC-U-PATT-030-01** | Unit | REQ-PATT-030 | Verify gap pattern analysis (consistent) | P2 |
| **TC-U-PATT-030-02** | Unit | REQ-PATT-030 | Verify gap pattern analysis (variable) | P2 |
| **TC-U-PATT-040-01** | Unit | REQ-PATT-040 | Verify source media classification (CD) | P2 |
| **TC-U-PATT-040-02** | Unit | REQ-PATT-040 | Verify source media classification (Vinyl) | P2 |
| **TC-S-PATT-010-01** | System | REQ-PATT-010 | Verify pattern detection accuracy >80% | P1 |
| **TC-U-CTXM-010-01** | Unit | REQ-CTXM-010 | Verify contextual matcher input parsing | P1 |
| **TC-U-CTXM-010-02** | Unit | REQ-CTXM-010 | Verify match score output format | P1 |
| **TC-U-CTXM-020-01** | Unit | REQ-CTXM-020 | Verify single-segment matching logic | P1 |
| **TC-I-CTXM-020-01** | Integration | REQ-CTXM-020 | Verify single-segment MusicBrainz query | P1 |
| **TC-U-CTXM-030-01** | Unit | REQ-CTXM-030 | Verify multi-segment album detection | P1 |
| **TC-U-CTXM-030-02** | Unit | REQ-CTXM-030 | Verify alignment score calculation | P1 |
| **TC-I-CTXM-030-01** | Integration | REQ-CTXM-030 | Verify multi-segment MusicBrainz query | P1 |
| **TC-S-CTXM-010-01** | System | REQ-CTXM-010 | Verify contextual matching narrows to <10 candidates | P1 |
| **TC-U-CONF-010-01** | Unit | REQ-CONF-010 | Verify evidence combination (single-segment) | P1 |
| **TC-U-CONF-010-02** | Unit | REQ-CONF-010 | Verify evidence combination (multi-segment) | P1 |
| **TC-U-CONF-010-03** | Unit | REQ-CONF-010 | Verify decision thresholds (Accept/Review/Reject) | P1 |
| **TC-I-CONF-010-01** | Integration | REQ-CONF-010 | Verify confidence assessor integrates with matcher+fingerprinter | P1 |
| **TC-S-CONF-010-01** | System | REQ-CONF-010 | Verify >90% acceptance rate on known-good files | P1 |
| **TC-S-CONF-010-02** | System | REQ-CONF-010 | Verify <5% false positive rate | P1 |
| **TC-U-FING-010-01** | Unit | REQ-FING-010 | Verify per-segment PCM extraction | P1 |
| **TC-U-FING-010-02** | Unit | REQ-FING-010 | Verify per-segment fingerprint generation | P1 |
| **TC-I-FING-010-01** | Integration | REQ-FING-010 | Verify per-segment AcoustID queries | P1 |
| **TC-S-FING-010-01** | System | REQ-FING-010 | Verify per-segment more accurate than whole-file for albums | P1 |
| **TC-U-TICK-010-01** | Unit | REQ-TICK-010 | Verify seconds_to_ticks() conversion accuracy | P2 |
| **TC-U-TICK-010-02** | Unit | REQ-TICK-010 | Verify tick conversion applied to all timing fields | P2 |
| **TC-I-TICK-010-01** | Integration | REQ-TICK-010 | Verify tick-based timing in database writes | P2 |

---

## Tests by Requirement

### REQ-PIPE-010: Segmentation-First Pipeline

| Test ID | Type | Description |
|---------|------|-------------|
| TC-U-PIPE-010-01 | Unit | Verify segmentation before fingerprinting |
| TC-I-PIPE-020-01 | Integration | Verify complete pipeline order (integration overlap) |

**Coverage:** Complete

---

### REQ-PIPE-020: Per-File Pipeline

| Test ID | Type | Description |
|---------|------|-------------|
| TC-U-PIPE-020-01 | Unit | Verify 4 workers created |
| TC-I-PIPE-020-01 | Integration | Verify per-file processing (one file through all steps) |

**Coverage:** Complete

---

### REQ-PATT-010/020/030/040: Pattern Analyzer

| Test ID | Type | Description |
|---------|------|-------------|
| TC-U-PATT-010-01 | Unit | Input acceptance |
| TC-U-PATT-010-02 | Unit | Output format |
| TC-U-PATT-020-01 | Unit | Track count detection |
| TC-U-PATT-030-01 | Unit | Gap pattern (consistent) |
| TC-U-PATT-030-02 | Unit | Gap pattern (variable) |
| TC-U-PATT-040-01 | Unit | Source media (CD) |
| TC-U-PATT-040-02 | Unit | Source media (Vinyl) |
| TC-S-PATT-010-01 | System | Accuracy >80% on test dataset |

**Coverage:** Complete (functional + accuracy)

---

### REQ-CTXM-010/020/030: Contextual Matcher

| Test ID | Type | Description |
|---------|------|-------------|
| TC-U-CTXM-010-01 | Unit | Input parsing |
| TC-U-CTXM-010-02 | Unit | Output format |
| TC-U-CTXM-020-01 | Unit | Single-segment logic |
| TC-I-CTXM-020-01 | Integration | Single-segment MB query |
| TC-U-CTXM-030-01 | Unit | Multi-segment detection |
| TC-U-CTXM-030-02 | Unit | Alignment score calc |
| TC-I-CTXM-030-01 | Integration | Multi-segment MB query |
| TC-S-CTXM-010-01 | System | Narrows to <10 candidates |

**Coverage:** Complete (logic + integration + accuracy)

---

### REQ-CONF-010: Confidence Assessor

| Test ID | Type | Description |
|---------|------|-------------|
| TC-U-CONF-010-01 | Unit | Single-segment evidence combination |
| TC-U-CONF-010-02 | Unit | Multi-segment evidence combination |
| TC-U-CONF-010-03 | Unit | Decision thresholds |
| TC-I-CONF-010-01 | Integration | Integration with matcher+fingerprinter |
| TC-S-CONF-010-01 | System | >90% acceptance rate |
| TC-S-CONF-010-02 | System | <5% false positive rate |

**Coverage:** Complete (algorithm + integration + accuracy)

---

### REQ-FING-010: Per-Segment Fingerprinting

| Test ID | Type | Description |
|---------|------|-------------|
| TC-U-FING-010-01 | Unit | Per-segment PCM extraction |
| TC-U-FING-010-02 | Unit | Per-segment fingerprint generation |
| TC-I-FING-010-01 | Integration | Per-segment AcoustID queries |
| TC-S-FING-010-01 | System | More accurate than whole-file |

**Coverage:** Complete (unit + integration + accuracy)

---

### REQ-TICK-010: Tick-Based Timing

| Test ID | Type | Description |
|---------|------|-------------|
| TC-U-TICK-010-01 | Unit | Conversion accuracy |
| TC-U-TICK-010-02 | Unit | Applied to all fields |
| TC-I-TICK-010-01 | Integration | Database writes |

**Coverage:** Complete

---

## Test Execution Order

### Phase 1: Unit Tests (Development)
Execute during component development to verify logic:
1. Pattern Analyzer units (TC-U-PATT-*)
2. Contextual Matcher units (TC-U-CTXM-*)
3. Confidence Assessor units (TC-U-CONF-*)
4. Fingerprinter units (TC-U-FING-*)
5. Tick conversion units (TC-U-TICK-*)
6. Pipeline units (TC-U-PIPE-*)

### Phase 2: Integration Tests (Component Integration)
Execute after components complete to verify interactions:
1. Pipeline integration (TC-I-PIPE-020-01)
2. Contextual Matcher integration (TC-I-CTXM-*)
3. Confidence Assessor integration (TC-I-CONF-010-01)
4. Fingerprinter integration (TC-I-FING-010-01)
5. Tick conversion integration (TC-I-TICK-010-01)

### Phase 3: System Tests (End-to-End)
Execute after all integration tests pass:
1. Pattern accuracy (TC-S-PATT-010-01)
2. Contextual matching effectiveness (TC-S-CTXM-010-01)
3. Confidence assessment accuracy (TC-S-CONF-010-01, TC-S-CONF-010-02)
4. Fingerprinting accuracy (TC-S-FING-010-01)

---

## Test Data Requirements

### Test Dataset (Required for System Tests)

**Minimum Dataset:**
- 50 single-track files (known artist/title/MBID)
- 10 full album files (12+ tracks, known track list, MBIDs)
- 10 edge cases (no tags, ambiguous metadata, etc.)

**Total:** 70 test files minimum

**Storage:** `wkmp-ai/tests/fixtures/` (NOT committed to git due to size/licensing)

**Documentation:** `tests/fixtures/README.md` describes how to obtain/prepare test data

---

## Acceptance Criteria Summary

**All tests must pass for plan completion:**
- ✅ All 18 unit tests pass
- ✅ All 10 integration tests pass
- ✅ All 4 system tests pass
- ✅ Test coverage >80% (measured by cargo-tarpaulin)
- ✅ No regressions in existing tests

---

**END OF TEST INDEX**

**Next:** Individual test specification files provide detailed Given/When/Then for each test
