# Traceability Matrix: PLAN014 Mixer Refactoring

**Plan:** PLAN014_mixer_refactoring
**Date:** 2025-01-30
**Phase:** 3 - Acceptance Test Definition

---

## Purpose

This matrix provides bidirectional traceability between:
1. **Requirements** → **Tests** (forward traceability: every requirement has tests)
2. **Tests** → **Requirements** (backward traceability: every test traces to requirement)
3. **Requirements** → **Implementation** (forward traceability: every requirement is implemented)

---

## Complete Traceability Matrix

| Requirement | Tests | Implementation File(s) | Status | Coverage |
|-------------|-------|------------------------|--------|----------|
| **Investigation** |
| REQ-MIX-001 | TC-INV-001-01, TC-INV-001-02 | Investigation report (TBD) | Pending | Complete |
| **Code Cleanup** |
| REQ-MIX-002 | TC-CLN-002-01, TC-CLN-002-02 | Remove: mixer.rs OR pipeline/mixer.rs | Pending | Complete |
| **Documentation** |
| REQ-MIX-003 | TC-DOC-003-01 | docs/SPEC002-crossfade.md | Pending | Complete |
| REQ-MIX-004 | TC-DOC-004-01 | docs/SPEC002-crossfade.md | Pending | Complete |
| REQ-MIX-005 | TC-DOC-005-01 | docs/SPEC016-decoder_buffer_design.md | Pending | Complete |
| REQ-MIX-006 | TC-DOC-006-01 | docs/ADR-XXX-mixer_refactoring.md (TBD) | Pending | Complete |
| **Features (Conditional)** |
| REQ-MIX-007 | TC-F-007-01 | wkmp-ap/src/playback/mixer.rs (if needed) | Pending | Complete |
| REQ-MIX-008 | TC-F-008-01 | wkmp-ap/src/playback/mixer.rs (if needed) | Pending | Complete |
| REQ-MIX-009 | TC-F-009-01 | wkmp-ap/src/playback/mixer.rs (if needed) | Pending | Complete |
| **Unit Tests** |
| REQ-MIX-010 | TC-U-010-01, TC-U-010-02, TC-U-010-03, TC-U-010-04, TC-U-010-05, TC-U-010-06 | wkmp-ap/tests/mixer_unit_tests.rs (TBD) | Pending | Complete |
| **Integration Tests** |
| REQ-MIX-011 | TC-I-011-01, TC-I-011-02, TC-I-011-03, TC-I-011-04, TC-I-011-05, TC-I-011-06 | wkmp-ap/tests/mixer_integration_tests.rs (TBD) | Pending | Complete |
| REQ-MIX-012 | TC-X-012-01, TC-X-012-02, TC-X-012-03 | wkmp-ap/tests/mixer_crossfade_tests.rs (TBD) | Pending | Complete |

**Total:** 12 requirements, 23 tests, 100% coverage

---

## Requirement → Test Mapping (Detailed)

### REQ-MIX-001: Determine Active Mixer

**Tests:**
- **TC-INV-001-01:** Identify active mixer in PlaybackEngine
  - Type: Investigation (manual)
  - Verifies: Mixer instantiation located, source file identified
  - Pass: Exactly one mixer found, source clear
- **TC-INV-001-02:** Document all references to mixer implementations
  - Type: Investigation (manual)
  - Verifies: All imports, usages documented
  - Pass: Complete reference list, no ambiguity

**Coverage:** ✅ Complete (investigation + documentation)

---

### REQ-MIX-002: Remove Inactive Mixer

**Tests:**
- **TC-CLN-002-01:** Verify inactive mixer removed from codebase
  - Type: Manual verification
  - Verifies: Inactive mixer file deleted or archived
  - Pass: File not in active codebase
- **TC-CLN-002-02:** Verify no active references to removed mixer
  - Type: Manual verification (grep)
  - Verifies: No imports, calls, or mentions of removed mixer
  - Pass: Zero references found

**Coverage:** ✅ Complete (removal + reference check)

---

### REQ-MIX-003: Add [XFD-CURV-050] to SPEC002

**Tests:**
- **TC-DOC-003-01:** Verify [XFD-CURV-050] added to SPEC002
  - Type: Manual review
  - Verifies: New requirement added after [XFD-CURV-040]
  - Pass: Text matches specification, location correct, revision history updated

**Coverage:** ✅ Complete (documentation review)

---

### REQ-MIX-004: Add [XFD-IMPL-025] to SPEC002

**Tests:**
- **TC-DOC-004-01:** Verify [XFD-IMPL-025] added to SPEC002
  - Type: Manual review
  - Verifies: New requirement added after [XFD-IMPL-020]
  - Pass: Text matches specification, location correct, revision history updated

**Coverage:** ✅ Complete (documentation review)

---

### REQ-MIX-005: Clarify [DBD-MIX-040] Resume-Fade Terminology

**Tests:**
- **TC-DOC-005-01:** Verify [DBD-MIX-040] terminology clarified
  - Type: Manual review
  - Verifies: Text updated in SPEC016
  - Pass: "mixer-level fade" terminology added, distinction clear, revision history updated

**Coverage:** ✅ Complete (documentation review)

---

### REQ-MIX-006: Create ADR for Mixer Refactoring

**Tests:**
- **TC-DOC-006-01:** Verify ADR created with complete content
  - Type: Manual review
  - Verifies: ADR follows Nygard template, references mixer review, decision documented
  - Pass: All sections complete, rationale clear, traceability to mixer architecture review

**Coverage:** ✅ Complete (documentation review)

---

### REQ-MIX-007: Port Buffer Underrun Detection

**Tests:**
- **TC-F-007-01:** Buffer underrun detection functional
  - Type: Feature test (automated)
  - Verifies: Underrun detected when buffer empty, flatline output, auto-resume
  - Pass: All underrun behaviors verified
  - **Conditional:** Only if correct mixer active and feature missing

**Coverage:** ✅ Complete (conditional feature test)

---

### REQ-MIX-008: Port Position Event Emission

**Tests:**
- **TC-F-008-01:** Position event emission functional
  - Type: Feature test (automated)
  - Verifies: Events emitted periodically, accurate position, configurable interval
  - Pass: All event behaviors verified
  - **Conditional:** Only if correct mixer active and feature missing

**Coverage:** ✅ Complete (conditional feature test)

---

### REQ-MIX-009: Port Resume Fade-In Support

**Tests:**
- **TC-F-009-01:** Resume fade-in functional
  - Type: Feature test (automated)
  - Verifies: Fade starts at 0.0, reaches 1.0, correct curve, multiplicative application
  - Pass: All resume fade behaviors verified
  - **Conditional:** Only if correct mixer active and feature missing

**Coverage:** ✅ Complete (conditional feature test)

---

### REQ-MIX-010: Unit Tests for Pre-Faded Sample Reading

**Tests:**
- **TC-U-010-01:** Mixer API does not accept fade curve parameters
  - Type: Unit test (design verification)
  - Verifies: Compile-time enforcement via type system
  - Pass: API signatures do not include fade curve parameters
- **TC-U-010-02:** Mixer reads pre-faded samples (not applies curves)
  - Type: Unit test (automated)
  - Verifies: Output = buffer_sample * master_volume (no fade curves)
  - Pass: Linear relationship maintained, no non-linear transformation
- **TC-U-010-03:** Crossfade is simple addition (no fade calculations)
  - Type: Unit test (automated)
  - Verifies: Output = (buffer1 + buffer2) * master_volume
  - Pass: Sum matches expected, no fade curve calculations
- **TC-U-010-04:** Master volume applied after summing
  - Type: Unit test (automated)
  - Verifies: Master volume multiplier applied correctly
  - Pass: Output = samples * master_volume
- **TC-U-010-05:** Master volume clamping (0.0-1.0)
  - Type: Unit test (automated)
  - Verifies: Master volume out of range is clamped
  - Pass: Negative → 0.0, >1.0 → 1.0
- **TC-U-010-06:** Zero master volume outputs silence
  - Type: Unit test (automated)
  - Verifies: master_volume = 0.0 produces zero output
  - Pass: All samples = 0.0

**Coverage:** ✅ Complete (6 unit tests cover all aspects of REQ-MIX-010)

---

### REQ-MIX-011: Integration Tests with Fader Component

**Tests:**
- **TC-I-011-01:** Fader-Buffer-Mixer pipeline: fade-in
  - Type: Integration test (automated)
  - Verifies: Fader applies fade-in, Buffer stores, Mixer reads (no double-fading)
  - Pass: Output matches Fader output, not double-faded
- **TC-I-011-02:** Fader-Buffer-Mixer pipeline: fade-out
  - Type: Integration test (automated)
  - Verifies: Fader applies fade-out, Buffer stores, Mixer reads (no double-fading)
  - Pass: Output matches Fader output, not double-faded
- **TC-I-011-03:** Crossfade overlap with pre-fading (both passages)
  - Type: Integration test (automated)
  - Verifies: Both passages pre-faded, Mixer sums (simple addition)
  - Pass: Output = (A_faded + B_faded) * master_volume
- **TC-I-011-04:** No double-fading detection (negative test)
  - Type: Integration test (automated)
  - Verifies: Samples are NOT faded twice
  - Pass: Numeric values match single fade, not exponential²
- **TC-I-011-05:** Asymmetric fade durations (different A and B)
  - Type: Integration test (automated)
  - Verifies: Different fade durations handled correctly
  - Pass: Each passage faded per own duration, Mixer sums
- **TC-I-011-06:** Zero-duration fades (pass-through mode)
  - Type: Integration test (automated)
  - Verifies: Fader pass-through when fade duration = 0
  - Pass: Output = input (no fading), Mixer handles correctly

**Coverage:** ✅ Complete (6 integration tests cover all aspects of REQ-MIX-011)

---

### REQ-MIX-012: Crossfade Overlap Tests (Simple Addition Verification)

**Tests:**
- **TC-X-012-01:** Simple addition math (0.5 + 0.3 = 0.8)
  - Type: Integration test (automated)
  - Verifies: Mixer performs arithmetic sum
  - Pass: Output = 0.8 during overlap
- **TC-X-012-02:** Clipping behavior (sum > 1.0)
  - Type: Integration test (automated)
  - Verifies: Master volume applied, then clamp per [SSD-CLIP-010]
  - Pass: Output clamped to 1.0 after master volume
- **TC-X-012-03:** Asymmetric crossfade durations (pre-faded samples)
  - Type: Integration test (automated)
  - Verifies: Different fade durations, Mixer sums pre-faded samples
  - Pass: Simple addition regardless of asymmetric durations

**Coverage:** ✅ Complete (3 crossfade tests cover all aspects of REQ-MIX-012)

---

## Test → Requirement Mapping (Backward Traceability)

| Test ID | Requirement | Verification Focus |
|---------|-------------|--------------------|
| TC-INV-001-01 | REQ-MIX-001 | Active mixer identification |
| TC-INV-001-02 | REQ-MIX-001 | Reference documentation |
| TC-CLN-002-01 | REQ-MIX-002 | Inactive mixer removed |
| TC-CLN-002-02 | REQ-MIX-002 | No orphaned references |
| TC-DOC-003-01 | REQ-MIX-003 | [XFD-CURV-050] in SPEC002 |
| TC-DOC-004-01 | REQ-MIX-004 | [XFD-IMPL-025] in SPEC002 |
| TC-DOC-005-01 | REQ-MIX-005 | [DBD-MIX-040] terminology |
| TC-DOC-006-01 | REQ-MIX-006 | ADR complete |
| TC-F-007-01 | REQ-MIX-007 | Underrun detection ported |
| TC-F-008-01 | REQ-MIX-008 | Position events ported |
| TC-F-009-01 | REQ-MIX-009 | Resume fade-in ported |
| TC-U-010-01 | REQ-MIX-010 | API does not accept fade curves |
| TC-U-010-02 | REQ-MIX-010 | Reads pre-faded samples |
| TC-U-010-03 | REQ-MIX-010 | Crossfade is simple addition |
| TC-U-010-04 | REQ-MIX-010 | Master volume applied |
| TC-U-010-05 | REQ-MIX-010 | Master volume clamping |
| TC-U-010-06 | REQ-MIX-010 | Zero master volume = silence |
| TC-I-011-01 | REQ-MIX-011 | Pipeline: fade-in |
| TC-I-011-02 | REQ-MIX-011 | Pipeline: fade-out |
| TC-I-011-03 | REQ-MIX-011 | Crossfade with pre-fading |
| TC-I-011-04 | REQ-MIX-011 | No double-fading |
| TC-I-011-05 | REQ-MIX-011 | Asymmetric fade durations |
| TC-I-011-06 | REQ-MIX-011 | Zero-duration fades |
| TC-X-012-01 | REQ-MIX-012 | Simple addition (0.5 + 0.3) |
| TC-X-012-02 | REQ-MIX-012 | Clipping behavior |
| TC-X-012-03 | REQ-MIX-012 | Asymmetric crossfade durations |

**Total:** 23 tests, all trace to requirements (no orphaned tests)

---

## Implementation Tracking

**Implementation files will be updated during implementation. Format:**

```markdown
| Requirement | Implementation File(s) | Lines | Status |
|-------------|------------------------|-------|--------|
| REQ-MIX-001 | Investigation report | N/A | Complete |
| REQ-MIX-002 | (Removed file) | N/A | Complete |
| REQ-MIX-003 | docs/SPEC002-crossfade.md | 215-220 | Complete |
| ... | ... | ... | ... |
```

**Status Values:**
- Pending: Not yet implemented
- In Progress: Implementation underway
- Complete: Implementation done, tests pass
- Verified: Reviewed and approved

---

## Coverage Gaps (NONE)

**Forward Traceability:** ✅ All 12 requirements have acceptance tests
**Backward Traceability:** ✅ All 23 tests trace to requirements
**Coverage:** ✅ 100% (no gaps detected)

**Verification:**
- REQ-MIX-001 through REQ-MIX-012: All have tests
- TC-INV-001-01 through TC-X-012-03: All trace to requirements
- No orphaned requirements (requirements without tests)
- No orphaned tests (tests without requirements)

---

## Usage During Implementation

**For Implementers:**
1. Select requirement to implement (e.g., REQ-MIX-003)
2. Find tests in matrix (e.g., TC-DOC-003-01)
3. Read test specification file (e.g., `tc_doc_003_01.md`)
4. Implement to pass test
5. Update "Implementation File(s)" and "Status" columns in this matrix
6. Mark test as Complete when passed

**For Reviewers:**
1. Check matrix for 100% coverage
2. Verify all "Complete" items have implementation files documented
3. Verify no orphaned tests or requirements

**For Project Managers:**
1. Track progress via "Status" column
2. % Complete = (Complete rows / Total rows) * 100
3. Identify blockers (Pending items that should be In Progress)

---

**Traceability Matrix Complete**
**Date:** 2025-01-30
**Coverage:** 100% (12 requirements, 23 tests, 0 gaps)
