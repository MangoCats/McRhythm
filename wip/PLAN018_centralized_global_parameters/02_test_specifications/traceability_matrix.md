# Traceability Matrix - PLAN018: Centralized Global Parameters

## Purpose

This matrix ensures:
1. **Forward Traceability:** Every requirement has acceptance tests
2. **Backward Traceability:** Every test traces to requirement(s)
3. **Implementation Traceability:** Every requirement maps to implementation files
4. **Coverage Verification:** No gaps in requirements → tests → implementation

---

## Requirements to Tests to Implementation

| Requirement ID | Priority | Tests | Implementation File(s) | Status | Coverage |
|----------------|----------|-------|------------------------|--------|----------|
| FR-001 | HIGH | TC-U-001-01, TC-U-001-02 | wkmp-common/src/params.rs | Pending | Complete |
| FR-002 | HIGH | TC-U-002-01, TC-U-002-02, TC-U-002-03 | wkmp-common/src/params.rs | Pending | Complete |
| FR-003 | CRITICAL | TC-U-003-01, TC-M-003-01 | All modified files | Pending | Complete |
| FR-004 | HIGH | TC-I-004-01, TC-I-004-02, TC-I-004-03, TC-I-004-04 | wkmp-common/src/params.rs (init_from_database) | Pending | Complete |
| FR-005 | MEDIUM | TC-U-005-01 | wkmp-common/src/params.rs (public API) | Pending | Complete |
| NFR-001 | HIGH | TC-U-101-01, TC-U-101-02 | wkmp-common/src/params.rs | Pending | Complete |
| NFR-002 | HIGH | TC-U-102-01 through TC-U-102-15 | wkmp-common/src/params.rs (setter methods) | Pending | Complete |
| NFR-003 | HIGH | TC-I-103-01, TC-I-103-02 | All test files | Pending | Complete |
| NFR-004 | MEDIUM | TC-M-104-01 | wkmp-common/src/params.rs (doc comments) | Pending | Complete |

**Coverage:** 100% (9/9 requirements have tests)

---

## Implementation Files by Requirement

### Primary Implementation

**wkmp-common/src/params.rs (NEW FILE)**
- **Requirements:** FR-001, FR-002, FR-004, NFR-001, NFR-002, NFR-004
- **Lines:** ~500-700 (estimated)
- **Key Components:**
  - GlobalParams struct definition (15 RwLock<T> fields)
  - Default trait implementation
  - init_from_database() method (database loading)
  - 15 setter methods with validation
  - Doc comments for all public items

**wkmp-common/src/lib.rs (MODIFIED)**
- **Requirements:** FR-001 (module visibility)
- **Change:** Add `pub mod params;` (+1 line)

**wkmp-ap/src/main.rs (MODIFIED)**
- **Requirements:** FR-004 (initialization call)
- **Change:** Add GlobalParams init after database pool creation (+2-3 lines)

### Per-Parameter Migration Files

**Files identified during migration (Step 2):**
- **Requirements:** FR-003 (replace hardcoded values)
- **Files:** TBD per parameter (e.g., mixer.rs, audio_output/mod.rs, engine.rs, decoder/*)
- **Changes:** Replace hardcoded literals with PARAMS access, remove Arc<RwLock> fields

### Test Files

**wkmp-common/src/params.rs (inline tests)**
- **Requirements:** All unit tests (TC-U-*)
- **Section:** `#[cfg(test)] mod tests { ... }`
- **Lines:** ~200-300 (estimated)

**wkmp-common/tests/params_integration.rs (NEW FILE)**
- **Requirements:** Integration tests (TC-I-*)
- **Lines:** ~150-200 (estimated)

---

## Tests to Requirements Mapping (Reverse Traceability)

| Test ID | Type | Requirement(s) | Description | Pass Criteria |
|---------|------|----------------|-------------|---------------|
| TC-U-001-01 | Unit | FR-001 | Verify 15 parameter fields exist | Count fields = 15 |
| TC-U-001-02 | Unit | FR-001 | Verify default values | All defaults match SPEC016 |
| TC-U-002-01 | Unit | FR-002 | Verify RwLock read | Read succeeds, no panic |
| TC-U-002-02 | Unit | FR-002 | Verify RwLock write | Write succeeds, value updates |
| TC-U-002-03 | Unit | FR-002 | Verify concurrent reads | 10 threads read simultaneously |
| TC-U-003-01 | Unit | FR-003 | Grep for hardcoded values | Zero matches for parameter defaults |
| TC-U-005-01 | Unit | FR-005 | Verify API unchanged | Compilation succeeds |
| TC-I-004-01 | Integration | FR-004 | Load from database | All 15 params loaded |
| TC-I-004-02 | Integration | FR-004 | Handle missing entries | Use defaults, log warnings |
| TC-I-004-03 | Integration | FR-004 | Handle type mismatch | Use defaults, log warnings |
| TC-I-004-04 | Integration | FR-004 | Handle out-of-range | Use defaults, log warnings |
| TC-U-101-01 | Unit | NFR-001 | Measure read performance | Median < 10ns |
| TC-U-101-02 | Unit | NFR-001 | Verify zero allocation | Allocations = 0 |
| TC-U-102-01 | Unit | NFR-002 | Validate volume_level | Range [0.0, 1.0] enforced |
| TC-U-102-02 | Unit | NFR-002 | Validate working_sample_rate | Range [8000, 192000] enforced |
| TC-U-102-03 | Unit | NFR-002 | Validate output_ringbuffer_size | Range [4410, 1000000] enforced |
| TC-U-102-04 | Unit | NFR-002 | Validate output_refill_period | Range [10, 1000] enforced |
| TC-U-102-05 | Unit | NFR-002 | Validate maximum_decode_streams | Range [1, 32] enforced |
| TC-U-102-06 | Unit | NFR-002 | Validate decode_work_period | Range [100, 60000] enforced |
| TC-U-102-07 | Unit | NFR-002 | Validate decode_chunk_size | Range [4410, 441000] enforced |
| TC-U-102-08 | Unit | NFR-002 | Validate playout_ringbuffer_size | Range [44100, 10000000] enforced |
| TC-U-102-09 | Unit | NFR-002 | Validate playout_ringbuffer_headroom | Range [2205, 88200] enforced |
| TC-U-102-10 | Unit | NFR-002 | Validate decoder_resume_hysteresis_samples | Range [2205, 441000] enforced |
| TC-U-102-11 | Unit | NFR-002 | Validate mixer_min_start_level | Range [2205, 88200] enforced |
| TC-U-102-12 | Unit | NFR-002 | Validate pause_decay_factor | Range [0.5, 0.99] enforced |
| TC-U-102-13 | Unit | NFR-002 | Validate pause_decay_floor | Range [0.00001, 0.001] enforced |
| TC-U-102-14 | Unit | NFR-002 | Validate audio_buffer_size | Range [512, 8192] enforced |
| TC-U-102-15 | Unit | NFR-002 | Validate mixer_check_interval_ms | Range [5, 100] enforced |
| TC-I-103-01 | Integration | NFR-003 | Full test suite after migration | `cargo test --workspace` passes |
| TC-I-103-02 | Integration | NFR-003 | Integration tests pass | `cargo test --test '*'` passes |
| TC-M-003-01 | Manual | FR-003 | Manual code review | No hardcoded values found |
| TC-M-104-01 | Manual | NFR-004 | Documentation review | All fields have doc comments |

**Total Tests:** 29 (21 unit, 6 integration, 2 manual)

---

## Parameter Coverage Matrix

Verification that all 15 parameters are migrated and tested:

| Parameter | DBD Tag | Risk Tier | Migration Tests | Validation Test | Implementation File | Status |
|-----------|---------|-----------|----------------|----------------|---------------------|--------|
| volume_level | DBD-PARAM-010 | Tier 1 | TC-U-003-01, TC-I-103-01 | TC-U-102-01 | TBD | Pending |
| working_sample_rate | DBD-PARAM-020 | Tier 3 | TC-U-003-01, TC-I-103-01, TC-M-003-01 | TC-U-102-02 | TBD | Pending |
| output_ringbuffer_size | DBD-PARAM-030 | Tier 2 | TC-U-003-01, TC-I-103-01 | TC-U-102-03 | TBD | Pending |
| output_refill_period | DBD-PARAM-040 | Tier 3 | TC-U-003-01, TC-I-103-01 | TC-U-102-04 | TBD | Pending |
| maximum_decode_streams | DBD-PARAM-050 | Tier 1 | TC-U-003-01, TC-I-103-01 | TC-U-102-05 | TBD | Pending |
| decode_work_period | DBD-PARAM-060 | Tier 1 | TC-U-003-01, TC-I-103-01 | TC-U-102-06 | TBD | Pending |
| decode_chunk_size | DBD-PARAM-065 | Tier 3 | TC-U-003-01, TC-I-103-01 | TC-U-102-07 | TBD | Pending |
| playout_ringbuffer_size | DBD-PARAM-070 | Tier 2 | TC-U-003-01, TC-I-103-01 | TC-U-102-08 | TBD | Pending |
| playout_ringbuffer_headroom | DBD-PARAM-080 | Tier 2 | TC-U-003-01, TC-I-103-01 | TC-U-102-09 | TBD | Pending |
| decoder_resume_hysteresis_samples | DBD-PARAM-085 | Tier 3 | TC-U-003-01, TC-I-103-01 | TC-U-102-10 | TBD | Pending |
| mixer_min_start_level | DBD-PARAM-088 | Tier 2 | TC-U-003-01, TC-I-103-01 | TC-U-102-11 | TBD | Pending |
| pause_decay_factor | DBD-PARAM-090 | Tier 1 | TC-U-003-01, TC-I-103-01 | TC-U-102-12 | TBD | Pending |
| pause_decay_floor | DBD-PARAM-100 | Tier 1 | TC-U-003-01, TC-I-103-01 | TC-U-102-13 | TBD | Pending |
| audio_buffer_size | DBD-PARAM-110 | Tier 3 | TC-U-003-01, TC-I-103-01 | TC-U-102-14 | TBD | Pending |
| mixer_check_interval_ms | DBD-PARAM-111 | Tier 3 | TC-U-003-01, TC-I-103-01 | TC-U-102-15 | TBD | Pending |

**Parameter Coverage:** 100% (15/15 parameters have tests)

---

## Gap Analysis

### Requirements Without Tests
**Count:** 0
**Status:** ✅ NO GAPS

### Tests Without Requirements
**Count:** 0
**Status:** ✅ NO ORPHANED TESTS

### Parameters Without Tests
**Count:** 0
**Status:** ✅ ALL PARAMETERS COVERED

### Parameters Without Implementation Files
**Count:** 15 (TBD during migration)
**Status:** ✅ EXPECTED - Files identified per-parameter during Step 2

---

## Coverage Verification Checklist

**Pre-Implementation:**
- [x] All requirements have test IDs
- [x] All tests trace to requirement IDs
- [x] All 15 parameters have validation tests
- [x] All error paths tested (missing, invalid, out-of-range)
- [x] Performance requirements tested (microbenchmark)
- [x] Safety requirements tested (validation)
- [x] Manual tests defined (code review, documentation)

**During Implementation:**
- [ ] Implementation files updated in matrix as discovered
- [ ] Test status updated (Pending → In Progress → Pass/Fail)
- [ ] Coverage percentage tracked per parameter

**Post-Implementation:**
- [ ] All tests executed and passed
- [ ] All implementation files documented
- [ ] No gaps remaining (requirements, tests, parameters)
- [ ] Traceability matrix 100% complete

---

## Usage

### For Implementers
1. Refer to this matrix to understand what tests verify each requirement
2. Update "Implementation File(s)" column when implementing each parameter
3. Update "Status" column as work progresses (Pending → In Progress → Pass)
4. Verify all tests for a requirement pass before marking requirement complete

### For Reviewers
1. Verify every requirement has tests (forward traceability)
2. Verify every test traces to requirement (backward traceability)
3. Check for gaps (untested requirements, orphaned tests)
4. Confirm implementation files correct for each requirement

### For Test Execution
1. Execute tests in order specified in test_index.md
2. Mark test status in matrix (Pass/Fail)
3. If test fails, file issue and link in matrix
4. Do not proceed to next parameter until all tests pass

---

## Document Status

**Traceability Matrix Status:** ✅ COMPLETE
**Coverage:** 100% requirements, 100% parameters
**Gaps:** NONE
**Ready for Implementation:** ✅ YES

**Note:** "Implementation File(s)" column will be updated during migration as files are identified.
