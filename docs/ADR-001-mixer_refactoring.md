# ADR-001: Migrate from Legacy Mixer to Correct Mixer Implementation

**Status:** Accepted
**Date:** 2025-01-30
**Context:** PLAN014 Mixer Refactoring
**Related Documents:** [Mixer Architecture Review](../wip/mixer_architecture_review.md) | [SPEC016](SPEC016-decoder_buffer_design.md) | [SPEC002](SPEC002-crossfade.md)

---

## Context

### Problem Statement

Two mixer implementations exist in wkmp-ap with conflicting architectural approaches:

**1. Legacy Mixer** (`wkmp-ap/src/playback/pipeline/mixer.rs`, 1,969 lines):
- **Current Status:** Active (instantiated in `engine.rs:238`)
- **Architecture:** Applies fade curves at mix time (runtime calculation)
- **Compliance:** **Violates SPEC016 [DBD-MIX-042]** architectural separation principle
- **State Complexity:** Stores fade curves, durations, and curve types in mixer state
- **API Design:** Accepts fade curve parameters in `start_passage()` and `start_crossfade()` methods
- **Features:** Complete (underrun detection, position events, resume fade-in)

**2. Correct Mixer** (`wkmp-ap/src/playback/mixer.rs`, 359 lines):
- **Current Status:** Inactive (exists but not instantiated anywhere)
- **Architecture:** Reads pre-faded samples from buffers (simple addition)
- **Compliance:** **Compliant with SPEC016 [DBD-MIX-042]** architectural separation
- **State Complexity:** Simple (master volume, pause state only)
- **API Design:** Does NOT accept fade curve parameters (enforced by type system)
- **Features:** Incomplete (missing underrun detection, position events, resume fade-in)

### Architectural Separation Principle (SPEC016 [DBD-MIX-042])

The specification defines clear separation of responsibilities:

**Fader Component** ([DBD-FADE-030]/[DBD-FADE-050]):
- Applies passage-specific fade-in/fade-out curves to samples **BEFORE buffering**
- Operates in decoder-resampler-fader-buffer chain
- Implementation: `wkmp-ap/src/playback/pipeline/fader.rs`

**Buffer Component** ([DBD-BUF-010]):
- Stores **pre-faded audio samples**
- No knowledge of fade curves

**Mixer Component** ([DBD-MIX-040]/[DBD-MIX-041]):
- Reads pre-faded samples from buffers
- Sums overlapping samples during crossfade (simple addition: `A + B`)
- Applies master volume
- **NO runtime fade curve calculations**

### Root Cause Analysis

Timeline reconstruction (from [Mixer Architecture Review](../wip/mixer_architecture_review.md)):

1. **Early Implementation:** Legacy mixer created before architectural separation was formalized
2. **Architectural Refinement:** SPEC016 enhanced with [DBD-MIX-041] and [DBD-MIX-042] requirements
3. **Correct Implementation:** New mixer (`mixer.rs`) created following [DBD-MIX-042]
4. **Current State:** Two mixers coexist; legacy mixer active, correct mixer unused

**Contributing Factors:**
- SPEC002 did not explicitly state fade curves are pre-applied (addressed in v1.2)
- Late clarification of architectural separation ([DBD-MIX-041]/[DBD-MIX-042] added after initial implementation)
- Resume-from-pause fade caused confusion (mixer DOES apply this, but it's mixer-level, not passage-level)
- Code duplication without clear migration plan

### Investigation Findings

**Investigation Report (2025-01-30):**
- **Active Mixer:** Legacy (CrossfadeMixer)
- **Instantiation:** `wkmp-ap/src/playback/engine.rs:238`
- **Import:** `use crate::playback::pipeline::mixer::CrossfadeMixer;`
- **Usage:** 1 active instantiation, 3 test usages
- **Correct Mixer:** 0 active instantiations (unused)

**Feature Analysis:**
- Legacy mixer: All features present (underrun detection, position events, resume fade-in)
- Correct mixer: Missing features (requires porting)

---

## Decision

**Selected Approach:** **Option A - Migrate to Correct Mixer with Feature Porting**

### Migration Strategy

**Phase 1: Preparation** (PLAN014 Increment 1-2)
1. Document investigation findings
2. Update specifications (SPEC002, SPEC016) to clarify architectural boundaries
3. Create this ADR
4. Define migration sub-plan

**Phase 2: Feature Porting** (PLAN014 Increment 3)
1. Port buffer underrun detection from legacy mixer (lines 422-611)
   - Maintain architectural separation (no fade curve knowledge)
   - Detect when buffer empty but decode not complete
   - Output flatline frame (last valid frame repeated)
   - Auto-resume when buffer refills

2. Port position event emission from legacy mixer (lines 614-642)
   - Periodic PositionUpdate events via `tokio::sync::mpsc`
   - Configurable interval (default 1 second)
   - Non-blocking send (`try_send`)

3. Port resume fade-in support from legacy mixer (lines 644-671)
   - ResumeState tracking (fade duration, curve, frames since resume)
   - Applied **multiplicatively AFTER mixing** (mixer-level fade)
   - Orthogonal to passage-level fades (no interference with Fader component)

**Phase 3: Migration** (PLAN014 Increment 4)
1. Update `wkmp-ap/src/playback/engine.rs`:
   - Change import: `use crate::playback::mixer::Mixer;`
   - Change instantiation: `let mut mixer = Mixer::new(master_volume);`
   - Update method calls (no fade curve parameters)

2. Verify Fader component applies curves correctly (integration tests)

**Phase 4: Testing** (PLAN014 Increments 5-7)
1. Unit tests: Verify mixer reads pre-faded samples (TC-U-010-01 through TC-U-010-06)
2. Integration tests: Verify Fader → Buffer → Mixer pipeline (TC-I-011-01 through TC-I-011-06)
3. Crossfade tests: Verify simple addition (TC-X-012-01 through TC-X-012-03)

**Phase 5: Cleanup** (PLAN014 Increment 8)
1. All tests pass (23/23)
2. Archive or delete legacy mixer (`pipeline/mixer.rs`)
3. Remove legacy mixer tests
4. Update documentation references

### Alternative Approaches Considered

**Option B - Fix Legacy Mixer to Comply with SPEC016:**
- Refactor legacy mixer to read pre-faded samples
- Remove fade curve application from mixer
- Delete correct mixer (unused)
- **Rejected:** Complex refactoring of 1,969 lines, higher risk of breaking existing functionality

**Option C - Keep Both Mixers:**
- Document each mixer's purpose
- Use legacy for features, correct for simplicity
- **Rejected:** Violates DRY principle, maintains architectural violation, confusing for developers

**Rationale for Option A:**
- Achieves SPEC016 compliance (primary goal)
- Simpler target architecture (359 lines vs 1,969 lines)
- Lower residual risk (port proven features to simple base)
- Feature parity maintained (no functionality lost)
- Test-driven migration (comprehensive test coverage)

---

## Consequences

### Positive

**1. Architectural Compliance:**
- ✅ 100% compliance with SPEC016 [DBD-MIX-042] architectural separation
- ✅ Mixer does NOT apply passage-level fade curves (enforced by architecture)
- ✅ Clear separation: Fader applies curves → Buffer stores → Mixer reads and sums
- ✅ Eliminates architectural violation from active codebase

**2. Code Simplification:**
- ✅ Single mixer implementation (359-700 lines, down from 2,328 total)
- ✅ 69-87% reduction in code duplication
- ✅ Simpler state machine (no fade curve storage)
- ✅ Smaller API surface (no fade curve parameters)

**3. Developer Clarity:**
- ✅ No ambiguity about which mixer to use
- ✅ Clear architectural boundaries (when/where fades are applied)
- ✅ Specifications clarified (SPEC002 v1.2, SPEC016 updates)
- ✅ ADR documents rationale for future developers

**4. Maintainability:**
- ✅ Simpler code easier to understand and modify
- ✅ Reduced cognitive load (one mixer to maintain, not two)
- ✅ Test coverage enforces architectural compliance
- ✅ Future features built on compliant foundation

**5. Test Coverage:**
- ✅ Comprehensive test suite (23 tests, 100% requirement coverage)
- ✅ Unit tests verify no fade curve application
- ✅ Integration tests verify end-to-end pipeline
- ✅ High confidence in correctness

### Negative

**1. Implementation Effort:**
- ⚠️ Feature porting required (underrun detection, position events, resume fade-in)
- ⚠️ Estimated 3-5 hours for feature porting
- ⚠️ Integration testing required (3-4 hours)
- ⚠️ Total estimated effort: 13-20 hours

**2. Migration Risk:**
- ⚠️ Risk of breaking existing playback functionality during migration
- ⚠️ **Mitigation:** Comprehensive test suite before migration
- ⚠️ **Mitigation:** Feature porting with tests before PlaybackEngine update
- ⚠️ **Mitigation:** Incremental approach (port features → test → migrate → test → cleanup)
- ⚠️ **Residual Risk:** Low (tests will catch regressions)

**3. Temporary Complexity:**
- ⚠️ During migration, both mixers coexist in codebase
- ⚠️ Must maintain clear status (which is active, which is being deprecated)
- ⚠️ **Mitigation:** Investigation report documents active mixer
- ⚠️ **Mitigation:** Clear commit messages during migration
- ⚠️ **Duration:** Temporary (resolved in PLAN014 Increment 8 cleanup)

**4. Testing Overhead:**
- ⚠️ Comprehensive testing required (23 tests)
- ⚠️ Test infrastructure may need creation (buffer mocks, etc.)
- ⚠️ **Mitigation:** Tests provide long-term value (prevent regressions)
- ⚠️ **Mitigation:** Modular test specifications simplify implementation

### Trade-offs Accepted

**1. Effort vs. Risk:**
- Accept 15-hour effort differential (Option A vs. Option B quick fix)
- Rationale: Lower residual risk outweighs implementation time
- Aligns with project charter quality-absolute goals (flawless audio playback)

**2. Temporary Complexity vs. Long-term Simplicity:**
- Accept temporary coexistence of two mixers during migration
- Rationale: Short-term complexity for long-term maintainability
- Clear migration plan minimizes confusion

**3. Test Coverage vs. Speed:**
- Accept comprehensive testing overhead (23 tests, 5-7 hours)
- Rationale: High confidence in correctness worth the time investment
- Tests prevent regressions, document expected behavior

---

## Related Decisions

**Specification Updates:**
- SPEC002 v1.2: Added [XFD-CURV-050] (fade application timing) and [XFD-IMPL-025] (architectural note)
- SPEC016: Pending update to [DBD-MIX-040] (clarify resume-fade terminology)

**Implementation Decisions:**
- PLAN014: Follow incremental migration approach (8 increments)
- Feature porting maintains architectural separation (no fade curve knowledge in mixer)
- Comprehensive test suite required before cleanup

**Future Impact:**
- All future mixer features must maintain architectural separation
- Fader component responsible for ALL passage-level fade curve application
- Mixer responsible ONLY for summing pre-faded samples + master volume

---

## References

**Documents:**
- [Mixer Architecture Review](../wip/mixer_architecture_review.md) - Analysis of two mixer implementations
- [PLAN014 Investigation Report](../wip/PLAN014_mixer_refactoring/investigation_report.md) - Active mixer determination
- [SPEC016 Decoder Buffer Design](SPEC016-decoder_buffer_design.md) - Architectural separation principle
- [SPEC002 Crossfade Design](SPEC002-crossfade.md) - Crossfade timing (not fade application)
- [PLAN014 Summary](../wip/PLAN014_mixer_refactoring/00_PLAN_SUMMARY.md) - Implementation plan

**Requirements:**
- SPEC016 [DBD-MIX-040]: Mixer in play mode (reads pre-faded samples)
- SPEC016 [DBD-MIX-041]: Crossfade mixing operation (simple addition)
- SPEC016 [DBD-MIX-042]: Architectural separation of concerns
- SPEC002 [XFD-CURV-050]: Fade curve application timing
- SPEC002 [XFD-IMPL-025]: Architectural note (timing vs. application)

**Code Locations:**
- Legacy mixer: `wkmp-ap/src/playback/pipeline/mixer.rs` (1,969 lines) - **To be removed**
- Correct mixer: `wkmp-ap/src/playback/mixer.rs` (359 lines) - **Target implementation**
- Fader component: `wkmp-ap/src/playback/pipeline/fader.rs` - Applies curves before buffering
- PlaybackEngine: `wkmp-ap/src/playback/engine.rs:238` - Mixer instantiation

---

**ADR Complete**
**Date:** 2025-01-30
**Status:** Accepted (migration in progress)
