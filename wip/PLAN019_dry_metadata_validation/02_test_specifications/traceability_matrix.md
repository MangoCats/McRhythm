# PLAN019: Traceability Matrix

**Purpose:** Ensure 100% coverage - every requirement has tests, every test traces to requirements
**Status:** Complete - All 10 requirements have acceptance tests

---

## Complete Traceability Matrix

| Requirement | Unit Tests | Integration Tests | System Tests | Manual Tests | Implementation File(s) | Status | Coverage |
|-------------|-----------|-------------------|--------------|--------------|------------------------|--------|----------|
| REQ-DRY-010 | TC-U-010-01, TC-U-010-02 | - | - | - | wkmp-common/src/params.rs | Pending | Complete |
| REQ-DRY-020 | TC-U-020-01 | - | - | - | wkmp-common/src/params.rs | Pending | Complete |
| REQ-DRY-030 | TC-U-030-01, TC-U-030-02 | - | - | - | wkmp-common/src/params.rs | Pending | Complete |
| REQ-DRY-040 | - | TC-I-040-01 | - | - | wkmp-common/src/params.rs | Pending | Complete |
| REQ-DRY-050 | TC-U-050-01 | - | - | - | wkmp-common/src/params.rs | Pending | Complete |
| REQ-DRY-060 | - | TC-I-060-01, TC-I-060-02 | - | - | wkmp-ap/src/api/handlers.rs | Pending | Complete |
| REQ-DRY-070 | - | TC-I-070-01 | - | - | wkmp-ap/src/api/handlers.rs | Pending | Complete |
| REQ-DRY-080 | - | TC-I-080-01 | - | - | wkmp-ap/src/db/settings.rs | Pending | Complete |
| REQ-DRY-090 | TC-U-090-01, TC-U-090-02 | - | - | - | (all test files) | Pending | Complete |
| REQ-DRY-100 | - | - | - | TC-M-100-01 | (documentation) | Pending | Complete |

---

## Forward Traceability (Requirement → Tests)

**Purpose:** Verify every requirement has acceptance tests

### REQ-DRY-010: Create ParamMetadata Struct
**Tests:**
- TC-U-010-01: Struct has 6 fields ✓
- TC-U-010-02: 15 parameters defined ✓

**Coverage:** Complete (structure + content verified)

---

### REQ-DRY-020: Implement metadata() Accessor
**Tests:**
- TC-U-020-01: Returns static reference ✓

**Coverage:** Complete (accessor behavior verified)

---

### REQ-DRY-030: Add Validation Closures
**Tests:**
- TC-U-030-01: Volume validator example ✓
- TC-U-030-02: All 15 validators batch test ✓

**Coverage:** Complete (all validators tested)

---

### REQ-DRY-040: Refactor init_from_database()
**Tests:**
- TC-I-040-01: Database loading uses metadata ✓

**Coverage:** Complete (refactored behavior verified)

---

### REQ-DRY-050: Refactor Setter Methods
**Tests:**
- TC-U-050-01: Setters delegate to validators ✓

**Coverage:** Complete (delegation pattern verified)

---

### REQ-DRY-060: Add API Validation
**Tests:**
- TC-I-060-01: Single error validation ✓
- TC-I-060-02: Batch error reporting ✓

**Coverage:** Complete (both single and batch cases)

---

### REQ-DRY-070: Prevent Invalid Database Writes
**Tests:**
- TC-I-070-01: Database unchanged after failure ✓

**Coverage:** Complete (enforcement verified)

---

### REQ-DRY-080: Refactor Volume Functions
**Tests:**
- TC-I-080-01: Volume uses metadata ✓

**Coverage:** Complete (refactored behavior verified)

---

### REQ-DRY-090: Maintain Test Coverage
**Tests:**
- TC-U-090-01: 24 existing tests pass ✓
- TC-U-090-02: 10 new tests pass ✓

**Coverage:** Complete (regression and new tests)

---

### REQ-DRY-100: Documentation
**Tests:**
- TC-M-100-01: 3 locations documented ✓

**Coverage:** Complete (all documentation locations)

---

## Backward Traceability (Test → Requirement)

**Purpose:** Verify no orphaned tests (every test traces to requirement)

| Test ID | Requirement | Trace Valid? |
|---------|-------------|--------------|
| TC-U-010-01 | REQ-DRY-010 | ✓ |
| TC-U-010-02 | REQ-DRY-010 | ✓ |
| TC-U-020-01 | REQ-DRY-020 | ✓ |
| TC-U-030-01 | REQ-DRY-030 | ✓ |
| TC-U-030-02 | REQ-DRY-030 | ✓ |
| TC-I-040-01 | REQ-DRY-040 | ✓ |
| TC-U-050-01 | REQ-DRY-050 | ✓ |
| TC-I-060-01 | REQ-DRY-060 | ✓ |
| TC-I-060-02 | REQ-DRY-060 | ✓ |
| TC-I-070-01 | REQ-DRY-070 | ✓ |
| TC-I-080-01 | REQ-DRY-080 | ✓ |
| TC-U-090-01 | REQ-DRY-090 | ✓ |
| TC-U-090-02 | REQ-DRY-090 | ✓ |
| TC-M-100-01 | REQ-DRY-100 | ✓ |

**Result:** ✓ No orphaned tests - all 13 tests trace to requirements

---

## Implementation Traceability

**Purpose:** Map requirements to implementation files

### wkmp-common/src/params.rs (Primary File)
**Requirements Implemented:**
- REQ-DRY-010: ParamMetadata struct (+50 lines)
- REQ-DRY-020: metadata() accessor (+20 lines)
- REQ-DRY-030: Validation closures (+100 lines)
- REQ-DRY-040: Refactored init_from_database() (~0 net, replacement)
- REQ-DRY-050: Refactored 15 setters (~-80 lines)

**Net Change:** +90 lines (eliminate ~80, add ~170)

---

### wkmp-ap/src/api/handlers.rs (API Validation)
**Requirements Implemented:**
- REQ-DRY-060: API validation logic (+30 lines)
- REQ-DRY-070: Database write prevention (enforcement, part of 060)
- REQ-DRY-100: API handler comments (+10 lines docs)

**Net Change:** +40 lines

---

### wkmp-ap/src/db/settings.rs (Volume Refactor)
**Requirements Implemented:**
- REQ-DRY-080: Refactored volume functions (-10 lines duplication)
- REQ-DRY-100: Function-level comments (optional, +5 lines)

**Net Change:** -5 lines (eliminate duplication)

---

### Test Files
**Requirements Implemented:**
- REQ-DRY-090: Test coverage (+200 lines unit tests, +150 lines integration tests)

**Net Change:** +350 lines tests

---

### Documentation
**Requirements Implemented:**
- REQ-DRY-100: Module/struct/API documentation (+30 lines Rustdoc)

**Net Change:** +30 lines docs

---

## Coverage Metrics

### By Requirement Priority

**Critical (2):**
- REQ-DRY-060: 2 tests (TC-I-060-01, TC-I-060-02) ✓
- REQ-DRY-070: 1 test (TC-I-070-01) ✓

**High (6):**
- REQ-DRY-010: 2 tests ✓
- REQ-DRY-020: 1 test ✓
- REQ-DRY-030: 2 tests ✓
- REQ-DRY-040: 1 test ✓
- REQ-DRY-050: 1 test ✓
- REQ-DRY-090: 2 tests ✓

**Medium (2):**
- REQ-DRY-080: 1 test ✓
- REQ-DRY-100: 1 test ✓

**Result:** 100% coverage across all priority levels

---

### By Implementation File

**wkmp-common/src/params.rs:**
- 7 unit tests
- 1 integration test
- **Total: 8 tests**

**wkmp-ap/src/api/handlers.rs:**
- 3 integration tests
- **Total: 3 tests**

**wkmp-ap/src/db/settings.rs:**
- 1 integration test
- **Total: 1 test**

**Test suite verification:**
- 2 unit tests (verify all pass)

**Documentation:**
- 1 manual test

---

## Test Execution Checklist

**For each requirement, verify:**
- [ ] REQ-DRY-010: Struct compiles, 15 parameters present
- [ ] REQ-DRY-020: Accessor returns static reference
- [ ] REQ-DRY-030: All validators accept defaults, reject out-of-range
- [ ] REQ-DRY-040: Database loading uses validators (no duplication)
- [ ] REQ-DRY-050: Setters delegate to validators
- [ ] REQ-DRY-060: API rejects invalid values (single + batch)
- [ ] REQ-DRY-070: Database unchanged after validation failure
- [ ] REQ-DRY-080: Volume functions use metadata (no .clamp())
- [ ] REQ-DRY-090: All 24 existing + 13 new tests pass
- [ ] REQ-DRY-100: Documentation in 3 locations

---

## Regression Prevention

**Existing Tests (24) Must Continue Passing:**

From `wkmp-common/src/params.rs`:
- test_global_params_has_all_fields
- test_global_params_volume_accessor
- test_global_params_working_sample_rate_accessor
- test_set_volume_level_valid
- test_set_volume_level_clamps
- test_init_from_database_with_all_values
- test_init_from_database_with_missing_values
- test_init_from_database_with_out_of_range_values
- test_init_from_database_with_type_mismatch
- test_init_from_database_with_null_values
- test_init_from_database_partial_values
- test_volume_level_clamping_from_database
- test_set_output_ringbuffer_size_valid
- test_set_output_ringbuffer_size_out_of_range
- test_set_pause_decay_factor_valid
- test_set_pause_decay_factor_out_of_range
- test_set_audio_buffer_size_valid
- test_set_audio_buffer_size_out_of_range
- test_set_maximum_decode_streams_valid
- test_set_maximum_decode_streams_out_of_range
- (24 total tests from previous implementation)

**Regression Check:** Run `cargo test -p wkmp-common params` after each increment

---

## Implementation Status Tracking

**During implementation, update this matrix:**

| Requirement | Status | Tests Passing | Files Modified | Notes |
|-------------|--------|---------------|----------------|-------|
| REQ-DRY-010 | Pending | 0/2 | - | Not started |
| REQ-DRY-020 | Pending | 0/1 | - | Not started |
| REQ-DRY-030 | Pending | 0/2 | - | Not started |
| REQ-DRY-040 | Pending | 0/1 | - | Not started |
| REQ-DRY-050 | Pending | 0/1 | - | Not started |
| REQ-DRY-060 | Pending | 0/2 | - | Not started |
| REQ-DRY-070 | Pending | 0/1 | - | Not started |
| REQ-DRY-080 | Pending | 0/1 | - | Not started |
| REQ-DRY-090 | Pending | 0/2 | - | Not started |
| REQ-DRY-100 | Pending | 0/1 | - | Not started |

**Progress:** 0% (0/13 tests passing)

---

## Final Verification Checklist

**Before marking plan complete:**
- [ ] All 10 requirements implemented
- [ ] All 37 tests passing (24 existing + 13 new)
- [ ] Traceability matrix 100% complete (no gaps)
- [ ] Implementation files match planned files
- [ ] Documentation complete in 3 locations
- [ ] Code reviewed and committed
- [ ] Phase 9 technical debt report generated

---

## Matrix Completeness Verification

**Forward Traceability:** ✓ Complete
- All 10 requirements → tests mapping verified
- No requirements without tests

**Backward Traceability:** ✓ Complete
- All 13 tests → requirement mapping verified
- No orphaned tests

**Implementation Mapping:** ✓ Complete
- All requirements → files mapping defined
- Implementation files identified

**Coverage:** ✓ 100%
- All requirements have acceptance tests
- All tests trace to requirements
- No gaps in traceability

---

**Matrix Status:** ✅ COMPLETE - Ready for implementation
