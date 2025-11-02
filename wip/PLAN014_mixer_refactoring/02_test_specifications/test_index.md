# Test Index: PLAN014 Mixer Refactoring

**Plan:** PLAN014_mixer_refactoring
**Date:** 2025-01-30
**Phase:** 3 - Acceptance Test Definition

---

## Test Summary

**Total Tests:** 23
- **Investigation Tests:** 2 (manual verification)
- **Code Cleanup Tests:** 2 (manual verification)
- **Documentation Tests:** 4 (manual review)
- **Unit Tests:** 6 (automated)
- **Integration Tests:** 6 (automated)
- **Feature Tests:** 3 (automated, conditional on REQ-MIX-001 findings)

**Coverage:** 100% of requirements have acceptance tests

---

## Test Catalog

| Test ID | Type | Requirement | Brief Description | Status |
|---------|------|-------------|-------------------|--------|
| **Investigation** |
| TC-INV-001-01 | Investigation | REQ-MIX-001 | Identify active mixer in PlaybackEngine | Pending |
| TC-INV-001-02 | Investigation | REQ-MIX-001 | Document all references to mixer implementations | Pending |
| **Code Cleanup** |
| TC-CLN-002-01 | Manual | REQ-MIX-002 | Verify inactive mixer removed from codebase | Pending |
| TC-CLN-002-02 | Manual | REQ-MIX-002 | Verify no active references to removed mixer | Pending |
| **Documentation** |
| TC-DOC-003-01 | Manual | REQ-MIX-003 | Verify [XFD-CURV-050] added to SPEC002 | Pending |
| TC-DOC-004-01 | Manual | REQ-MIX-004 | Verify [XFD-IMPL-025] added to SPEC002 | Pending |
| TC-DOC-005-01 | Manual | REQ-MIX-005 | Verify [DBD-MIX-040] terminology clarified | Pending |
| TC-DOC-006-01 | Manual | REQ-MIX-006 | Verify ADR created with complete content | Pending |
| **Unit Tests - Mixer API** |
| TC-U-010-01 | Unit | REQ-MIX-010 | Mixer API does not accept fade curve parameters | Pending |
| TC-U-010-02 | Unit | REQ-MIX-010 | Mixer reads pre-faded samples (not applies curves) | Pending |
| TC-U-010-03 | Unit | REQ-MIX-010 | Crossfade is simple addition (no fade calculations) | Pending |
| **Unit Tests - Master Volume** |
| TC-U-010-04 | Unit | REQ-MIX-010 | Master volume applied after summing | Pending |
| TC-U-010-05 | Unit | REQ-MIX-010 | Master volume clamping (0.0-1.0) | Pending |
| TC-U-010-06 | Unit | REQ-MIX-010 | Zero master volume outputs silence | Pending |
| **Integration Tests - Fader Pipeline** |
| TC-I-011-01 | Integration | REQ-MIX-011 | Fader-Buffer-Mixer pipeline: fade-in | Pending |
| TC-I-011-02 | Integration | REQ-MIX-011 | Fader-Buffer-Mixer pipeline: fade-out | Pending |
| TC-I-011-03 | Integration | REQ-MIX-011 | Crossfade overlap with pre-fading (both passages) | Pending |
| TC-I-011-04 | Integration | REQ-MIX-011 | No double-fading detection (samples not faded twice) | Pending |
| TC-I-011-05 | Integration | REQ-MIX-011 | Asymmetric fade durations (different A and B) | Pending |
| TC-I-011-06 | Integration | REQ-MIX-011 | Zero-duration fades (pass-through mode) | Pending |
| **Crossfade Overlap Tests** |
| TC-X-012-01 | Integration | REQ-MIX-012 | Simple addition math (0.5 + 0.3 = 0.8) | Pending |
| TC-X-012-02 | Integration | REQ-MIX-012 | Clipping behavior (sum > 1.0) | Pending |
| TC-X-012-03 | Integration | REQ-MIX-012 | Asymmetric crossfade durations (pre-faded samples) | Pending |
| **Feature Tests (Conditional)** |
| TC-F-007-01 | Feature | REQ-MIX-007 | Buffer underrun detection | Pending |
| TC-F-008-01 | Feature | REQ-MIX-008 | Position event emission | Pending |
| TC-F-009-01 | Feature | REQ-MIX-009 | Resume fade-in from pause | Pending |

---

## Test Organization

**Modular Structure:**
- Each test has individual specification file: `tc_<type>_<req>_<num>.md`
- Test type codes:
  - INV = Investigation
  - CLN = Cleanup
  - DOC = Documentation
  - U = Unit Test
  - I = Integration Test
  - X = Crossfade Test
  - F = Feature Test (conditional)

**Test Files:**
- Investigation: `tc_inv_001_01.md`, `tc_inv_001_02.md`
- Cleanup: `tc_cln_002_01.md`, `tc_cln_002_02.md`
- Documentation: `tc_doc_003_01.md`, `tc_doc_004_01.md`, `tc_doc_005_01.md`, `tc_doc_006_01.md`
- Unit Tests: `tc_u_010_01.md` through `tc_u_010_06.md`
- Integration Tests: `tc_i_011_01.md` through `tc_i_011_06.md`
- Crossfade Tests: `tc_x_012_01.md` through `tc_x_012_03.md`
- Feature Tests: `tc_f_007_01.md`, `tc_f_008_01.md`, `tc_f_009_01.md`

---

## Test Dependencies

**Sequencing:**

1. **Phase 1: Investigation** (TC-INV-001-01, TC-INV-001-02)
   - Must complete first
   - Determines which mixer is active
   - Informs subsequent increments

2. **Phase 2: Cleanup** (TC-CLN-002-01, TC-CLN-002-02)
   - Depends on Phase 1 completion
   - May require migration sub-plan if legacy mixer active

3. **Phase 3: Documentation** (TC-DOC-003-01 through TC-DOC-006-01)
   - Can proceed in parallel with code changes
   - No dependencies on investigation results (except TC-DOC-006-01 ADR decision section)

4. **Phase 4: Unit Tests** (TC-U-010-01 through TC-U-010-06)
   - Depends on mixer implementation finalized
   - Can run on either mixer (verifies architectural compliance)

5. **Phase 5: Integration Tests** (TC-I-011-01 through TC-I-011-06)
   - Depends on Phase 4 completion
   - Requires Fader + Buffer + Mixer all functional

6. **Phase 6: Crossfade Tests** (TC-X-012-01 through TC-X-012-03)
   - Depends on Phase 5 completion
   - Verifies simple addition (no double-fading)

7. **Phase 7: Feature Tests (Conditional)** (TC-F-007-01, TC-F-008-01, TC-F-009-01)
   - Only execute if correct mixer active and features missing
   - Depends on feature porting complete

---

## Coverage Verification

**Requirement → Test Mapping:**

| Requirement | Tests | Coverage |
|-------------|-------|----------|
| REQ-MIX-001 | TC-INV-001-01, TC-INV-001-02 | Complete ✓ |
| REQ-MIX-002 | TC-CLN-002-01, TC-CLN-002-02 | Complete ✓ |
| REQ-MIX-003 | TC-DOC-003-01 | Complete ✓ |
| REQ-MIX-004 | TC-DOC-004-01 | Complete ✓ |
| REQ-MIX-005 | TC-DOC-005-01 | Complete ✓ |
| REQ-MIX-006 | TC-DOC-006-01 | Complete ✓ |
| REQ-MIX-007 | TC-F-007-01 (conditional) | Complete ✓ |
| REQ-MIX-008 | TC-F-008-01 (conditional) | Complete ✓ |
| REQ-MIX-009 | TC-F-009-01 (conditional) | Complete ✓ |
| REQ-MIX-010 | TC-U-010-01 through TC-U-010-06 | Complete ✓ |
| REQ-MIX-011 | TC-I-011-01 through TC-I-011-06 | Complete ✓ |
| REQ-MIX-012 | TC-X-012-01 through TC-X-012-03 | Complete ✓ |

**Total Coverage:** 12/12 requirements (100%)

---

## Test Execution Plan

**Estimated Effort:**
- Investigation tests: 30-60 minutes (code inspection)
- Cleanup tests: 15-30 minutes (verification)
- Documentation tests: 30-45 minutes (review)
- Unit tests: 2-3 hours (write + implement)
- Integration tests: 3-4 hours (write + implement + debug)
- Crossfade tests: 2-3 hours (write + implement)
- Feature tests: 3-5 hours (conditional, if needed)

**Total:** 11-19 hours for all tests

---

## Success Criteria

**All Tests Pass:**
- Investigation: Active mixer identified, references documented
- Cleanup: Inactive mixer removed, no orphaned references
- Documentation: All 4 specification updates complete
- Unit: 6/6 tests pass (mixer reads pre-faded, no fade calculations)
- Integration: 6/6 tests pass (Fader → Buffer → Mixer pipeline verified)
- Crossfade: 3/3 tests pass (simple addition verified)
- Features: N/N tests pass (where N depends on findings)

**Traceability Matrix Complete:**
- Every requirement has tests
- Every test traces to requirement
- No orphaned tests

**Coverage Target:**
- ≥80% code coverage for mixer module
- 100% coverage for mixing logic (sum samples, apply master volume)

---

**Test Index Complete**
**Date:** 2025-01-30
