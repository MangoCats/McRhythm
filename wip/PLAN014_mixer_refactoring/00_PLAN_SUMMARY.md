# PLAN014: Mixer Refactoring - PLAN SUMMARY

**Status:** Ready for Implementation (Phases 1-3 Complete)
**Created:** 2025-01-30
**Specification Source:** wip/mixer_architecture_review.md (Recommendations section, lines 390-424)
**Plan Location:** `wip/PLAN014_mixer_refactoring/`

---

## READ THIS FIRST

This plan implements the recommendations from the mixer architecture review to resolve architectural violations and eliminate code duplication in the wkmp-ap mixer implementations.

**For Implementation:**
- Read this summary (~450 lines)
- Review requirements index for specific requirement details
- Review test specifications for acceptance criteria
- Follow traceability matrix to ensure 100% requirement coverage

**Context Window Budget:**
- This summary: ~450 lines
- Requirements index: ~400 lines
- Test index + specific tests: ~200-300 lines
- **Total per increment:** ~650-850 lines (optimal for implementation)

---

## Executive Summary

### Problem Being Solved

**Primary Issue:** Two mixer implementations exist in wkmp-ap codebase with conflicting architectural approaches:

1. **Legacy mixer** (`wkmp-ap/src/playback/pipeline/mixer.rs`, 1,969 lines)
   - Applies fade curves at mix time (runtime calculation)
   - **Violates SPEC016 [DBD-MIX-042] architectural separation principle**
   - Complex state machine stores fade curves and durations
   - Featured but non-compliant

2. **Correct mixer** (`wkmp-ap/src/playback/mixer.rs`, 359 lines)
   - Reads pre-faded samples from buffers (simple addition)
   - **Compliant with SPEC016 [DBD-MIX-042]**
   - Simple design: sum overlapping samples, apply master volume
   - May be missing features (underrun detection, position events, resume fade)

**Impact:**
- **Code duplication:** 2,328 lines across two implementations
- **Architectural violation:** If legacy mixer is active, violates SPEC016
- **Developer confusion:** Unclear which mixer to use, when fades are applied
- **Specification ambiguity:** SPEC002 doesn't explicitly state fade application timing

### Solution Approach

**5-Phase Strategy:**

1. **Investigation** (REQ-MIX-001): Determine which mixer is active
2. **Cleanup** (REQ-MIX-002): Remove inactive mixer, eliminate duplication
3. **Documentation** (REQ-MIX-003/004/005/006): Clarify specifications, create ADR
4. **Feature Parity** (REQ-MIX-007/008/009): Port missing features if needed (conditional)
5. **Testing** (REQ-MIX-010/011/012): Verify architectural compliance

**Expected Outcome:**
- Single mixer implementation (~359-700 lines)
- 100% SPEC016 [DBD-MIX-042] compliance
- Clear architectural boundaries documented
- Comprehensive test coverage (unit + integration)

### Implementation Status

**Phases 1-3 Complete:**
- ✅ Phase 1: Scope Definition - 12 requirements extracted, dependencies mapped
- ✅ Phase 2: Specification Verification - 0 Critical, 2 High, 4 Medium, 1 Low issues identified
- ✅ Phase 3: Test Definition - 23 tests defined, 100% traceability coverage

**Phases 4-8 Status:** Week 2-3 implementation (not yet scheduled)

**Decision:** PROCEED to implementation - No critical blockers found

---

## Requirements Summary

**Total Requirements:** 12 (all P0-Critical or P1-High)

| Req ID | Type | Brief Description | Priority |
|--------|------|-------------------|----------|
| REQ-MIX-001 | Investigation | Determine which mixer is active | P0-Critical |
| REQ-MIX-002 | Technical Debt | Remove inactive mixer implementation | P0-Critical |
| REQ-MIX-003 | Documentation | Add [XFD-CURV-050] to SPEC002 | P1-High |
| REQ-MIX-004 | Documentation | Add [XFD-IMPL-025] to SPEC002 | P1-High |
| REQ-MIX-005 | Documentation | Clarify [DBD-MIX-040] resume-fade terminology | P1-High |
| REQ-MIX-006 | Documentation | Create ADR documenting mixer refactoring | P1-High |
| REQ-MIX-007 | Feature | Port buffer underrun detection (if needed) | P1-High |
| REQ-MIX-008 | Feature | Port position event emission (if needed) | P1-High |
| REQ-MIX-009 | Feature | Port resume fade-in support (if needed) | P1-High |
| REQ-MIX-010 | Testing | Unit tests verifying mixer reads pre-faded samples | P1-High |
| REQ-MIX-011 | Testing | Integration tests with Fader component | P1-High |
| REQ-MIX-012 | Testing | Crossfade overlap tests (simple addition verification) | P1-High |

**By Type:**
- Investigation: 1
- Technical Debt: 1
- Documentation: 4
- Feature: 3 (conditional on REQ-MIX-001 findings)
- Testing: 3

**Full Requirements:** See `requirements_index.md` (~400 lines)

---

## Scope

### ✅ In Scope

**Investigation:**
- Determine which mixer implementation is currently active in PlaybackEngine
- Identify all references to mixer implementations
- Analyze feature gaps between legacy and correct mixer

**Code Cleanup:**
- Remove or archive inactive mixer implementation
- Update imports and references
- Eliminate code duplication (reduce ~2,328 lines to ~359-700 lines)

**Documentation:**
- Add [XFD-CURV-050] to SPEC002 (fade application timing)
- Add [XFD-IMPL-025] to SPEC002 (architectural note)
- Clarify [DBD-MIX-040] in SPEC016 (resume-fade terminology)
- Create ADR documenting mixer refactoring rationale

**Feature Porting (Conditional):**
- Buffer underrun detection (if correct mixer active and missing)
- Position event emission (if correct mixer active and missing)
- Resume fade-in support (if correct mixer active and missing)
- **Constraint:** Maintain architectural separation during porting (no fade curve knowledge in mixer)

**Testing:**
- Unit tests: Mixer reads pre-faded samples (6 tests)
- Integration tests: Fader → Buffer → Mixer pipeline (6 tests)
- Crossfade tests: Simple addition verification (3 tests)
- Feature tests: Ported features (3 tests, conditional)

### ❌ Out of Scope

**Explicitly NOT Included:**
- New mixer functionality not present in either implementation
- Modifying Fader component
- Changing buffer management system
- Modifying crossfade timing algorithm
- Performance optimization (beyond existing)
- New fade curve types
- API endpoint changes
- PlaybackEngine refactoring (beyond mixer instantiation)

**Full Scope:** See `scope_statement.md` (~450 lines)

---

## Specification Issues

**Phase 2 Analysis Results:**
- **CRITICAL Issues:** 0
- **HIGH Issues:** 2
  - HIGH-001 (REQ-MIX-002): Scenario A (migration) underspecified
  - HIGH-002 (REQ-MIX-007): Underrun detection behavior not extracted
- **MEDIUM Issues:** 4
  - MEDIUM-001 (REQ-MIX-005): [DBD-MIX-040] line number not specified
  - MEDIUM-002 (REQ-MIX-006): ADR content depends on unknown (defer to Increment 2)
  - MEDIUM-003 (REQ-MIX-009): Resume fade behavior not extracted
  - MEDIUM-004 (REQ-MIX-012): [SSD-CLIP-010] clipping spec not provided
- **LOW Issues:** 1
  - LOW-001 (REQ-MIX-010): Compile-time test not runtime testable

**Decision:** PROCEED - No critical blockers

**Issue Resolution Strategy:**
- HIGH issues: Resolve during increment planning (before implementation)
- MEDIUM issues: Resolve during implementation (incremental fixes)
- LOW issues: Clarify in test specifications

**Full Analysis:** See `01_specification_issues.md` (~350 lines)

---

## Implementation Roadmap

**Note:** Phases 4-8 (detailed increments, estimates, risks) scheduled for Week 2-3 implementation. Below is high-level roadmap based on Phases 1-3.

### Increment 1: Investigation and Documentation Foundation (Estimated: 2-3 hours)

**Objective:** Identify active mixer, begin documentation improvements

**Deliverables:**
- Investigation report (TC-INV-001-01/002 complete)
- Draft ADR structure (REQ-MIX-006 partial - Context, Consequences sections)
- SPEC002 updates (REQ-MIX-003/004)

**Tests:** TC-INV-001-01, TC-INV-001-02, TC-DOC-003-01, TC-DOC-004-01

**Success Criteria:**
- Active mixer identified with evidence
- ADR drafted (Decision section pending investigation results)
- SPEC002 enhanced with [XFD-CURV-050] and [XFD-IMPL-025]

---

### Increment 2: Cleanup and ADR Completion (Estimated: 1-2 hours)

**Objective:** Remove inactive mixer, complete ADR

**Deliverables:**
- Inactive mixer removed or archived (REQ-MIX-002)
- ADR Decision section complete (depends on Increment 1 findings)
- SPEC016 [DBD-MIX-040] terminology clarified (REQ-MIX-005)

**Tests:** TC-CLN-002-01, TC-CLN-002-02, TC-DOC-005-01, TC-DOC-006-01

**Success Criteria:**
- Single mixer implementation in codebase
- ADR complete with decision rationale
- All specification clarifications done

**Note:** If investigation reveals legacy mixer active, may require migration sub-plan (resolve HIGH-001 issue)

---

### Increment 3: Unit Tests (Estimated: 2-3 hours)

**Objective:** Verify mixer reads pre-faded samples (no fade curve calculations)

**Deliverables:**
- 6 unit tests for mixer mixing logic (REQ-MIX-010)
- Test code + implementation in `wkmp-ap/tests/mixer_unit_tests.rs`

**Tests:** TC-U-010-01 through TC-U-010-06

**Success Criteria:**
- All 6 unit tests pass
- Test coverage ≥80% for mixer module
- Verified: Mixer does NOT apply fade curves, ONLY applies master volume

---

### Increment 4: Integration Tests (Estimated: 3-4 hours)

**Objective:** Verify Fader → Buffer → Mixer pipeline (architectural separation)

**Deliverables:**
- 6 integration tests for Fader-Buffer-Mixer pipeline (REQ-MIX-011)
- Test code + implementation in `wkmp-ap/tests/mixer_integration_tests.rs`

**Tests:** TC-I-011-01 through TC-I-011-06

**Success Criteria:**
- All 6 integration tests pass
- Verified: Fader applies curves BEFORE buffering, Mixer reads pre-faded samples
- No double-fading detected

---

### Increment 5: Crossfade Tests (Estimated: 2-3 hours)

**Objective:** Verify crossfade is simple addition (no runtime fade calculations)

**Deliverables:**
- 3 crossfade overlap tests (REQ-MIX-012)
- Test code + implementation in `wkmp-ap/tests/mixer_crossfade_tests.rs`

**Tests:** TC-X-012-01 through TC-X-012-03

**Success Criteria:**
- All 3 crossfade tests pass
- Verified: Output = (A + B) * master_volume (simple addition)
- Clipping behavior per [SSD-CLIP-010]

**Note:** Resolve MEDIUM-004 (clipping spec) before implementing TC-X-012-02

---

### Increment 6: Feature Porting (Conditional, Estimated: 3-5 hours)

**Objective:** Port missing features if correct mixer is active

**Deliverables (Conditional):**
- Buffer underrun detection (REQ-MIX-007, if needed)
- Position event emission (REQ-MIX-008, if needed)
- Resume fade-in support (REQ-MIX-009, if needed)
- 3 feature tests (TC-F-007-01, TC-F-008-01, TC-F-009-01)

**Tests:** TC-F-007-01, TC-F-008-01, TC-F-009-01 (conditional)

**Success Criteria:**
- All ported features functional
- Architectural separation maintained (no fade curve knowledge in mixer)
- All feature tests pass

**Note:** Resolve HIGH-002 and MEDIUM-003 (extract feature specs from legacy mixer) before implementing

**Conditional Execution:**
- **If investigation finds legacy mixer active:** Skip this increment (features already present)
- **If investigation finds correct mixer active AND missing features:** Execute this increment
- **If investigation finds correct mixer active AND features present:** Skip this increment

---

**Total Estimated Effort:** 13-20 hours (excluding migration if needed)

---

## Test Coverage Summary

**Total Tests:** 23
- Investigation: 2 (manual)
- Code Cleanup: 2 (manual verification)
- Documentation: 4 (manual review)
- Unit Tests: 6 (automated)
- Integration Tests: 6 (automated)
- Crossfade Tests: 3 (automated)
- Feature Tests: 3 (automated, conditional)

**Coverage:** 100% - All 12 requirements have acceptance tests

**Test Organization:**
- Test index: `02_test_specifications/test_index.md` (~250 lines)
- Individual test specs: `02_test_specifications/tc_*.md` (~50-150 lines each)
- Traceability matrix: `02_test_specifications/traceability_matrix.md` (~400 lines)

**Sample Test Specifications Provided:**
- `tc_inv_001_01.md` - Investigation: Identify active mixer
- `tc_u_010_02.md` - Unit test: Mixer reads pre-faded samples
- `tc_i_011_01.md` - Integration test: Fader-Buffer-Mixer pipeline (fade-in)

**Traceability:** Complete matrix in `02_test_specifications/traceability_matrix.md`
- Forward: Every requirement → tests
- Backward: Every test → requirement
- No gaps, no orphaned tests

---

## Risk Assessment

**Residual Risk:** Low-Medium (after mitigation)

**Top Risks:**

1. **Risk:** Legacy mixer is active, migration is complex
   - **Mitigation:** Thorough investigation (REQ-MIX-001), incremental migration if needed
   - **Residual Risk:** Low-Medium

2. **Risk:** Specification ambiguity not fully resolved by clarifications
   - **Mitigation:** Multiple clarifications (REQ-MIX-003/004/005), stakeholder review
   - **Residual Risk:** Low

3. **Risk:** Feature porting introduces architectural violations
   - **Mitigation:** Explicit constraints (maintain separation), comprehensive tests (REQ-MIX-010/011/012)
   - **Residual Risk:** Low

4. **Risk:** Integration tests reveal Fader component issues
   - **Mitigation:** Out of scope for this plan, would create separate plan
   - **Residual Risk:** Medium (Fader assumed functional per assumptions)

**Full Risk Analysis:** Detailed assessment in Phase 7 (Week 3 implementation)

---

## Technical Debt and Known Issues

**Status:** Not applicable - plan not yet implemented

**Note:** After implementation completes, Phase 9 (Post-Implementation Review) will systematically discover and document:
- Known bugs and limitations
- Test coverage gaps
- Performance concerns
- Security issues
- Deferred requirements

See Phase 9 section of /plan workflow for 7-step technical debt discovery process.

**Mandatory:** Do NOT mark plan complete or archive until Phase 9 technical debt report is generated and attached.

---

## Success Metrics

**Quantitative:**
- ✅ Code duplication eliminated: 2,328 lines → ~359-700 lines (69-87% reduction)
- ✅ Test coverage: ≥80% for mixer module, 100% for mixing logic
- ✅ Specification clarity: 2 new SPEC002 requirements added ([XFD-CURV-050], [XFD-IMPL-025])
- ✅ Architectural violations: 1 mixer violates [DBD-MIX-042] → 0 violations (100% compliance)

**Qualitative:**
- ✅ Developer understanding: Single mixer, clear architectural boundaries, ADR documenting rationale
- ✅ Specification clarity: No reasonable misinterpretation possible (fade application timing explicit)
- ✅ Code maintainability: Simple enough for junior developer to understand
- ✅ Test confidence: High confidence mixer reads pre-faded samples (not applies curves)

---

## Dependencies

**Existing Documents (Read-Only):**
- wkmp-ap/src/playback/pipeline/mixer.rs (1,969 lines) - Legacy mixer (source for feature porting if needed)
- wkmp-ap/src/playback/mixer.rs (359 lines) - Correct mixer (may need feature additions)
- wkmp-ap/src/playback/pipeline/fader.rs - Fader component (integration tests verify interaction)
- docs/SPEC002-crossfade.md (~450 lines estimated) - Will be modified (add [XFD-CURV-050], [XFD-IMPL-025])
- docs/SPEC016-decoder_buffer_design.md (~730 lines estimated) - Will be modified (clarify [DBD-MIX-040])

**Integration Points:**
- PlaybackEngine mixer instantiation (investigation will locate)
- Fader → Buffer → Mixer pipeline (integration tests verify)

**No External Dependencies** (all work contained within wkmp-ap)

---

## Constraints

**Technical:**
- Must maintain architectural separation per SPEC016 [DBD-MIX-042]
- Mixer MUST NOT apply fade curves at mix time
- Mixer MUST NOT store fade curve configuration in state
- Mixer APIs MUST NOT accept fade curve parameters
- Sample-accurate timing required (~0.02ms precision @ 44.1kHz)

**Process:**
- Documentation changes require review per GOV001
- Code removal requires verification of no active references
- Feature porting requires equivalent test coverage
- ADR requires Nygard template format

**Timeline:**
- P0-Critical requirements (REQ-MIX-001/002) first
- Documentation can proceed in parallel with code
- Testing after implementation finalized

---

## Next Steps

### Immediate (Ready Now)

1. **Review this plan summary** (confirm understanding)
2. **Review specification issues** (`01_specification_issues.md`) - understand HIGH/MEDIUM issues
3. **Review test specifications** (`02_test_specifications/test_index.md`) - understand acceptance criteria
4. **Verify assumptions** (see `scope_statement.md` assumptions section)

### Implementation Sequence

1. **Increment 1:** Investigation + Documentation Foundation
   - Execute TC-INV-001-01/002 (identify active mixer)
   - Implement REQ-MIX-003/004 (SPEC002 updates)
   - Draft ADR (REQ-MIX-006 partial)

2. **Increment 2:** Cleanup + ADR Completion
   - Execute TC-CLN-002-01/002 (remove inactive mixer)
   - Complete ADR (REQ-MIX-006)
   - Implement REQ-MIX-005 (SPEC016 update)

3. **Increment 3:** Unit Tests
   - Implement TC-U-010-01 through TC-U-010-06
   - Verify mixer reads pre-faded samples

4. **Increment 4:** Integration Tests
   - Implement TC-I-011-01 through TC-I-011-06
   - Verify Fader → Buffer → Mixer pipeline

5. **Increment 5:** Crossfade Tests
   - Implement TC-X-012-01 through TC-X-012-03
   - Verify simple addition (no runtime fade calculations)

6. **Increment 6 (Conditional):** Feature Porting
   - If needed: Implement REQ-MIX-007/008/009
   - Execute TC-F-007-01, TC-F-008-01, TC-F-009-01

### After Implementation

1. Execute Phase 9: Post-Implementation Review (MANDATORY)
2. Generate technical debt report
3. Run all 23 tests
4. Verify traceability matrix 100% complete
5. Create final implementation report
6. Archive plan using `/archive-plan PLAN014`

---

## Document Navigation

**Start Here:** This file (00_PLAN_SUMMARY.md) - ~450 lines

**Detailed Planning:**
- `requirements_index.md` - All requirements with detailed acceptance criteria (~400 lines)
- `scope_statement.md` - In/out scope, assumptions, constraints (~450 lines)
- `01_specification_issues.md` - Phase 2 analysis, issues by severity (~350 lines)

**Test Specifications:**
- `02_test_specifications/test_index.md` - All tests quick reference (~250 lines)
- `02_test_specifications/traceability_matrix.md` - Requirements ↔ Tests mapping (~400 lines)
- `02_test_specifications/tc_*.md` - Individual test specs (~50-150 lines each)

**For Implementation:**
- Read this summary (~450 lines)
- Read current increment requirements from `requirements_index.md` (~50-100 lines)
- Read relevant test specs (~100-200 lines)
- **Total context:** ~600-750 lines per increment

---

## Plan Status

**Phase 1-3 Status:** Complete (2025-01-30)
- ✅ Phase 1: Scope Definition - 12 requirements extracted, dependencies mapped
- ✅ Phase 2: Specification Verification - 7 issues identified, prioritized, resolutions documented
- ✅ Phase 3: Test Definition - 23 tests defined, 100% traceability

**Phases 4-8 Status:** Pending (Week 2-3 implementation)
- Week 2: Phases 4-5 (Approach Selection, Implementation Breakdown)
- Week 3: Phases 6-8 (Estimates, Risks, Final Documentation)

**Current Status:** Ready for Implementation Review
**Estimated Timeline:** 13-20 hours over 2-3 weeks (excluding migration if needed)

---

## Approval and Sign-Off

**Plan Created:** 2025-01-30
**Plan Status:** Ready for Implementation Review

**Phase 1-3 Verification:**
- ✅ All requirements extracted and documented
- ✅ Scope boundaries clear (in/out, assumptions, constraints)
- ✅ Specification issues identified and prioritized
- ✅ No critical blockers (0 CRITICAL issues)
- ✅ All requirements have acceptance tests (100% coverage)
- ✅ Traceability matrix complete (no gaps)
- ✅ Modular output structure (context-window optimized)

**Next Action:** Review plan, confirm understanding, approve to proceed with Increment 1 (Investigation + Documentation Foundation)

**Key Decision Points:**
1. After Increment 1: Confirm investigation findings before cleanup
2. After Increment 2: Confirm all specifications updated before testing
3. After Increment 5: Determine if Increment 6 (feature porting) needed
4. After all increments: Execute Phase 9 (technical debt assessment) before archiving

---

**PLAN014 Summary Complete**
**Date:** 2025-01-30
**Ready for Implementation**
