# Scope Statement: PLAN014 Mixer Refactoring

**Plan:** PLAN014_mixer_refactoring
**Date:** 2025-01-30
**Source:** wip/mixer_architecture_review.md (Recommendations section)

---

## Problem Being Solved

**Primary Issue:** Two mixer implementations exist in wkmp-ap codebase with conflicting architectural approaches:

1. **Legacy mixer** (`wkmp-ap/src/playback/pipeline/mixer.rs`, 1,969 lines)
   - Applies fade curves at mix time (runtime calculation)
   - Violates SPEC016 [DBD-MIX-042] architectural separation principle
   - Complex state machine stores fade curves and durations

2. **Correct mixer** (`wkmp-ap/src/playback/mixer.rs`, 359 lines)
   - Reads pre-faded samples from buffers
   - Compliant with SPEC016 [DBD-MIX-042]
   - Simple design: sum overlapping samples, apply master volume

**Secondary Issues:**
- Specification ambiguity in SPEC002 regarding fade curve application timing
- Developer confusion about architectural boundaries (when/where fades are applied)
- No ADR documenting mixer refactoring rationale
- Incomplete test coverage for architectural separation principle

**Impact:**
- Code duplication (2,328 lines across two implementations)
- Architectural violations in active codebase (if legacy mixer is active)
- Confusion for future developers
- Risk of implementing features in wrong mixer

---

## Solution Approach

**High-Level Strategy:**

1. **Investigation Phase (REQ-MIX-001):**
   - Determine which mixer is currently active in PlaybackEngine
   - Identify all references to mixer implementations

2. **Cleanup Phase (REQ-MIX-002):**
   - Remove inactive mixer from codebase (archive or delete)
   - Eliminate code duplication

3. **Feature Parity Phase (REQ-MIX-007/008/009):**
   - If correct mixer is active and missing features:
     - Port buffer underrun detection (from legacy mixer lines 422-611)
     - Port position event emission (from legacy mixer lines 614-642)
     - Port resume fade-in support (from legacy mixer lines 644-671)
   - Maintain architectural separation during porting

4. **Documentation Phase (REQ-MIX-003/004/005/006):**
   - Add [XFD-CURV-050] to SPEC002 (fade application timing)
   - Add [XFD-IMPL-025] to SPEC002 (architectural note)
   - Clarify [DBD-MIX-040] resume-fade terminology
   - Create ADR documenting mixer refactoring decision

5. **Testing Phase (REQ-MIX-010/011/012):**
   - Unit tests: Verify mixer reads pre-faded samples
   - Integration tests: Verify Fader → Buffer → Mixer pipeline
   - Crossfade tests: Verify simple addition (no runtime fade calculations)

---

## In Scope

### ✅ Included in This Plan

**Investigation:**
- [x] Determine which mixer implementation is currently active (REQ-MIX-001)
- [x] Identify all references to mixer implementations in codebase
- [x] Analyze which features exist in legacy but missing in correct mixer

**Code Cleanup:**
- [x] Remove or archive inactive mixer implementation (REQ-MIX-002)
- [x] Update imports and references if needed
- [x] Verify no active code depends on removed mixer

**Feature Porting (Conditional):**
- [x] Port buffer underrun detection (if correct mixer active and missing) (REQ-MIX-007)
- [x] Port position event emission (if correct mixer active and missing) (REQ-MIX-008)
- [x] Port resume fade-in support (if correct mixer active and missing) (REQ-MIX-009)
- [x] Maintain architectural separation during porting (no fade curve knowledge in mixer)

**Documentation:**
- [x] Add [XFD-CURV-050] to SPEC002 after [XFD-CURV-040] (REQ-MIX-003)
- [x] Add [XFD-IMPL-025] to SPEC002 after [XFD-IMPL-020] (REQ-MIX-004)
- [x] Clarify [DBD-MIX-040] resume-fade terminology in SPEC016 (REQ-MIX-005)
- [x] Create ADR documenting mixer refactoring rationale (REQ-MIX-006)
- [x] Update SPEC002 and SPEC016 revision histories

**Testing:**
- [x] Unit tests: Mixer reads pre-faded samples (no fade curve calculations) (REQ-MIX-010)
- [x] Integration tests: Fader → Buffer → Mixer pipeline (REQ-MIX-011)
- [x] Crossfade overlap tests: Simple addition verification (REQ-MIX-012)
- [x] Test coverage for all ported features (if any)

---

## Out of Scope

### ❌ Explicitly NOT Included

**New Features:**
- Implementing new mixer functionality not present in either implementation
- Adding new fade curve types
- Performance optimization (beyond what exists in legacy mixer)
- New mixing modes (e.g., volume normalization, ducking, EQ)

**Architecture Changes:**
- Modifying Fader component ([wkmp-ap/src/playback/pipeline/fader.rs](../wkmp-ap/src/playback/pipeline/fader.rs))
- Changing buffer management system
- Modifying crossfade timing algorithm (SPEC002 [XFD-IMPL-010])
- Changing fade curve calculation logic (FadeCurve enum in wkmp-common)

**Specification Changes (Beyond Clarifications):**
- Adding new requirements to SPEC002 or SPEC016
- Changing architectural principles
- Modifying [DBD-MIX-042] separation of concerns

**Other Modules:**
- PlaybackEngine refactoring (beyond mixer instantiation changes)
- BufferManager modifications
- DecoderChain modifications
- API endpoint changes

**Rationale:** This plan focuses on architectural cleanup and compliance, not new feature development. Scope limited to mixer refactoring, documentation clarification, and testing.

---

## Assumptions

### Explicit Assumptions

1. **SPEC016 [DBD-MIX-042] Is Authoritative:**
   - Architectural separation principle is correct
   - Fader applies curves BEFORE buffering
   - Mixer reads pre-faded samples
   - Mixer performs simple addition during crossfade
   - **Risk if False:** Would need to re-evaluate entire plan
   - **Validation:** Review with project stakeholders, confirm against project charter quality goals

2. **One Mixer Is Currently Active:**
   - Either legacy or correct mixer is instantiated by PlaybackEngine
   - Not both (no runtime switching between implementations)
   - **Risk if False:** Investigation would reveal dual usage, requiring different cleanup strategy
   - **Validation:** REQ-MIX-001 investigation will confirm

3. **Fader Component Is Functional:**
   - [wkmp-ap/src/playback/pipeline/fader.rs](../wkmp-ap/src/playback/pipeline/fader.rs) exists and works correctly
   - Applies fade curves to samples before buffering
   - **Risk if False:** Integration tests would fail, indicating Fader issues
   - **Validation:** REQ-MIX-011 integration tests will verify

4. **Buffer Management Provides Pre-Faded Samples:**
   - Buffers store samples that have already passed through Fader
   - No additional fading occurs between Fader and Mixer
   - **Risk if False:** Would indicate architectural issue in pipeline
   - **Validation:** REQ-MIX-011 integration tests will verify

5. **Test Infrastructure Is Available:**
   - tokio::test framework functional
   - Buffer mocking/test utilities exist or can be created
   - Test coverage tools available (cargo-tarpaulin or similar)
   - **Risk if False:** Would need to create test infrastructure first
   - **Validation:** Check existing test files in wkmp-ap/tests/

6. **Legacy Mixer Features Are Desirable:**
   - Buffer underrun detection is needed
   - Position event emission is needed
   - Resume fade-in is needed
   - **Risk if False:** Features may not need porting
   - **Validation:** Confirm with stakeholders during REQ-MIX-001 investigation

7. **No Breaking Changes to Public APIs:**
   - Mixer refactoring is internal implementation change
   - PlaybackEngine API remains stable
   - SSE events (if any) remain compatible
   - **Risk if False:** Would require API versioning or migration plan
   - **Validation:** Review PlaybackEngine public API surface

---

## Constraints

### Technical Constraints

1. **Architectural Separation Must Be Maintained:**
   - Mixer MUST NOT apply fade curves at mix time
   - Mixer MUST NOT store fade curve configuration in state
   - Mixer APIs MUST NOT accept fade curve parameters
   - **Source:** SPEC016 [DBD-MIX-042]

2. **Sample-Accurate Timing Required:**
   - All timing calculations must be sample-accurate (~0.02ms precision @ 44.1kHz)
   - Frame counting must be precise
   - **Source:** SPEC002 [XFD-OV-010], SPEC016 [DBD-DEC-080]

3. **Backwards Compatibility:**
   - Must support all existing fade curves (exponential, cosine, linear)
   - Must maintain all existing SSE event contracts
   - Must not break existing queue management
   - **Source:** Project stability requirements

4. **Rust Language Constraints:**
   - Must compile on stable Rust channel
   - Must follow IMPL002 coding conventions
   - Must use tokio async runtime
   - **Source:** IMPL002-coding_conventions.md

5. **Zero-Configuration Startup:**
   - Must maintain [REQ-NF-030] through [REQ-NF-037] zero-config startup
   - No new required configuration parameters
   - **Source:** REQ001-requirements.md

### Process Constraints

1. **Documentation Review Required:**
   - All SPEC002 and SPEC016 changes require review
   - ADR must follow Nygard template format
   - Changes must be logged in revision history
   - **Source:** GOV001-document_hierarchy.md

2. **Code Removal Verification:**
   - Must verify no active references to removed mixer
   - Must check all imports, tests, documentation
   - **Source:** Standard refactoring practice

3. **Test-First Approach:**
   - Tests must be defined before implementation (this plan defines them)
   - Feature porting must include equivalent test coverage
   - **Source:** /plan workflow, CLAUDE.md

4. **Traceability Required:**
   - All changes must reference requirements (REQ-MIX-###)
   - ADR must reference mixer architecture review analysis
   - **Source:** GOV002-requirements_enumeration.md

### Timeline Constraints

1. **P0-Critical Requirements First:**
   - REQ-MIX-001 (investigation) must complete before REQ-MIX-002 (cleanup)
   - REQ-MIX-002 (cleanup) must complete before feature porting
   - **Rationale:** Can't remove mixer until we know which is active

2. **Documentation Can Proceed in Parallel:**
   - REQ-MIX-003/004/005/006 (documentation) can start immediately
   - Documentation does not depend on code investigation
   - **Rationale:** Specification clarifications are independent of implementation status

3. **Testing After Implementation:**
   - REQ-MIX-010/011/012 (testing) requires mixer implementation finalized
   - Tests verify correct behavior, so implementation must be stable
   - **Rationale:** Test-driven development (tests defined now, implemented after code)

4. **No External Dependencies on Timeline:**
   - This plan does not block other development work
   - This plan is not blocked by other development work
   - **Rationale:** Mixer refactoring is internal cleanup

---

## Dependencies

### Existing Code (Read-Only - No Modifications)

**Required to Understand:**
1. **wkmp-ap/src/playback/pipeline/mixer.rs** (1,969 lines)
   - Legacy mixer implementation
   - Source for feature porting (if needed)
   - Will be removed or archived after plan complete

2. **wkmp-ap/src/playback/mixer.rs** (359 lines)
   - Correct mixer implementation
   - May need feature additions (underrun, position events, resume fade)

3. **wkmp-ap/src/playback/pipeline/fader.rs**
   - Fader component that applies curves before buffering
   - Integration tests will verify interaction with mixer

4. **wkmp-ap/src/playback/buffer_manager.rs**
   - Buffer management system
   - Provides pre-faded samples to mixer

5. **wkmp-ap/src/playback/playback_engine.rs** (estimated location)
   - PlaybackEngine that instantiates mixer
   - Investigation will check which mixer is used

6. **wkmp-common/src/fade_curve.rs** (estimated location)
   - FadeCurve enum (exponential, cosine, linear)
   - Used by Fader, not by Mixer (per [DBD-MIX-042])

### Specifications (Will Be Modified)

1. **docs/SPEC002-crossfade.md**
   - Will add [XFD-CURV-050] and [XFD-IMPL-025]
   - Current lines: ~450 (estimated)

2. **docs/SPEC016-decoder_buffer_design.md**
   - Will clarify [DBD-MIX-040] resume-fade terminology
   - Current lines: ~730 (estimated)

### External Libraries (No Changes)

1. **tokio** - Async runtime (existing)
2. **symphonia** - Audio decoding (existing)
3. **rubato** - Resampling (existing)
4. **cpal** - Audio output (existing)

### Test Infrastructure

1. **cargo test** - Unit test framework (existing)
2. **tokio::test** - Async test support (existing)
3. **Buffer mocking** - May need to create test utilities
4. **cargo-tarpaulin** (optional) - Coverage analysis

---

## Success Metrics

### Quantitative Metrics

1. **Code Duplication Eliminated:**
   - **Before:** 2,328 lines (1,969 + 359) across two mixers
   - **After:** ~359-700 lines (single mixer, may grow with ported features)
   - **Target:** <700 lines total

2. **Test Coverage:**
   - **Before:** Unknown (investigation will establish baseline)
   - **After:** ≥80% coverage for mixer module
   - **Target:** 100% coverage for mixing logic (REQ-MIX-010/011/012)

3. **Specification Clarity:**
   - **Before:** 0 explicit statements in SPEC002 about fade application timing
   - **After:** 2 new requirements ([XFD-CURV-050], [XFD-IMPL-025]) added
   - **Target:** Zero ambiguity about architectural boundaries

4. **Architectural Violations:**
   - **Before:** 1 mixer violates [DBD-MIX-042] (if legacy is active)
   - **After:** 0 mixers violate [DBD-MIX-042]
   - **Target:** 100% compliance with architectural separation

### Qualitative Metrics

1. **Developer Understanding:**
   - **Before:** Confusion about which mixer to use, when fades are applied
   - **After:** Single mixer, clear architectural boundaries, ADR documenting rationale
   - **Target:** New developers can understand mixer architecture in <15 minutes

2. **Specification Clarity:**
   - **Before:** SPEC002 ambiguous about fade application timing
   - **After:** Explicit statements in SPEC002 and SPEC016
   - **Target:** No reasonable misinterpretation possible

3. **Code Maintainability:**
   - **Before:** Two implementations, architectural violations, complex state
   - **After:** Single implementation, architectural compliance, simple state
   - **Target:** Mixer code is simple enough for junior developer to understand

4. **Test Confidence:**
   - **Before:** Unknown test coverage, no tests for architectural separation
   - **After:** Comprehensive tests verify architectural separation
   - **Target:** High confidence that mixer reads pre-faded samples (not applies curves)

---

## Risks and Assumptions

*Note: Detailed risk assessment in Phase 7 (Week 3 implementation). Key risks identified here for context.*

**Key Risks:**

1. **Risk:** Legacy mixer is active and has undocumented features
   - **Mitigation:** Thorough investigation (REQ-MIX-001) before removal
   - **Residual Risk:** Low (investigation will discover features)

2. **Risk:** Specification ambiguity not fully resolved by clarifications
   - **Mitigation:** Multiple clarifications (REQ-MIX-003/004/005) address different aspects
   - **Residual Risk:** Low-Medium (stakeholder review required)

3. **Risk:** Feature porting introduces architectural violations
   - **Mitigation:** Explicit constraint (maintain separation), tests verify (REQ-MIX-010/011/012)
   - **Residual Risk:** Low (tests will catch violations)

4. **Risk:** Integration tests reveal Fader component issues
   - **Mitigation:** Out of scope for this plan, would create separate plan
   - **Residual Risk:** Medium (Fader assumed functional)

**See full risk assessment in Phase 7 documentation (Week 3).**

---

## Scope Summary

**In Scope:** Mixer refactoring, documentation clarification, testing
**Out of Scope:** New features, architecture changes, other modules
**Assumptions:** SPEC016 authoritative, Fader functional, one mixer active
**Constraints:** Architectural separation, sample-accurate timing, backwards compatibility

**Scope Definition Complete**
**Date:** 2025-01-30
