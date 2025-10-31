# PLAN014 Investigation Report: Active Mixer Determination

**Date:** 2025-01-30
**Tests Executed:** TC-INV-001-01, TC-INV-001-02
**Status:** Complete

---

## TC-INV-001-01 Results: Active Mixer Identified

**Active Mixer:** **Legacy** (CrossfadeMixer)
**Source File:** `wkmp-ap/src/playback/pipeline/mixer.rs`
**Instantiation Location:** `wkmp-ap/src/playback/engine.rs:238`

### Evidence

**Import Statement:**
```rust
// wkmp-ap/src/playback/engine.rs (line 1)
use crate::playback::pipeline::mixer::CrossfadeMixer;
```

**Instantiation Code:**
```rust
// wkmp-ap/src/playback/engine.rs:236-238
// Create mixer
// [SSD-MIX-010] Crossfade mixer for sample-accurate mixing
let mut mixer = CrossfadeMixer::new();
```

**Method Calls Observed:**
```rust
// wkmp-ap/src/playback/engine.rs:240-244
mixer.set_event_channel(position_event_tx.clone());
mixer.set_position_event_interval_ms(interval_ms);
```

**Mixer Type:** `CrossfadeMixer` from `playback::pipeline::mixer` module

### Conclusion

**Legacy mixer is active** (violates SPEC016 [DBD-MIX-042])

---

## TC-INV-001-02 Results: Mixer References Documented

### All References to CrossfadeMixer (Legacy)

**1. Primary Usage (Active):**
- `wkmp-ap/src/playback/engine.rs:238` - **ACTIVE INSTANTIATION**
  - Line 1: Import statement
  - Line 238: `CrossfadeMixer::new()`
  - Lines 240-244: Configuration methods called

**2. Test Usage (Legacy Mixer):**
- `wkmp-ap/src/playback/pipeline/mixer.rs:1080` - Test: `test_crossfade_mixer_creation()`
- `wkmp-ap/src/playback/pipeline/mixer.rs:1166` - Test: `test_crossfade_basic_overlap()`
- `wkmp-ap/src/playback/pipeline/mixer.rs:1497` - Test: `test_pause_mode_decay()`

### All References to Mixer (Correct)

**No active usage found.**

Search results:
```bash
$ grep -rn "Mixer::new()" src/ | grep -v "CrossfadeMixer"
(no results)
```

**Conclusion:** Correct mixer (`wkmp-ap/src/playback/mixer.rs`) is NOT instantiated anywhere in active code.

### Module Declarations

**Legacy Mixer:**
- Declared in: `wkmp-ap/src/playback/pipeline/mod.rs` (inferred)
- Path: `crate::playback::pipeline::mixer::CrossfadeMixer`

**Correct Mixer:**
- Declared in: `wkmp-ap/src/playback/mod.rs` (inferred)
- Path: `crate::playback::mixer::Mixer`
- **Status:** Unused (no imports found)

---

## Implications for PLAN014

### REQ-MIX-002 Scenario Determination

**Scenario A applies:** Legacy mixer is active, migration needed

**Migration Strategy Required:**
1. Legacy mixer (pipeline/mixer.rs, 1969 lines) is currently active
2. Correct mixer (mixer.rs, 359 lines) exists but is unused
3. Must migrate from legacy to correct mixer
4. **Cannot simply delete legacy mixer** (would break active code)

**Migration Steps (from PLAN014):**
1. Port missing features from legacy to correct mixer (if any)
2. Update PlaybackEngine to use correct mixer
3. Update imports
4. Verify all tests pass
5. Remove legacy mixer after migration verified

### REQ-MIX-007/008/009 Applicability

**Feature Porting NOT Needed** - Legacy mixer already has all features:
- ✅ REQ-MIX-007: Buffer underrun detection (legacy has it, lines 422-611)
- ✅ REQ-MIX-008: Position event emission (legacy has it, lines 614-642)
- ✅ REQ-MIX-009: Resume fade-in support (legacy has it, lines 644-671)

**However, correct mixer is missing these features:**
- ❌ Correct mixer lacks underrun detection
- ❌ Correct mixer lacks position event emission
- ❌ Correct mixer lacks resume fade-in support

**Decision Point:**
Two migration approaches:

**Option A: Port features FROM legacy TO correct (recommended per PLAN014)**
- Keep correct mixer as target (SPEC016 compliant)
- Port missing features while maintaining architectural separation
- Delete legacy mixer after migration
- **Pros:** Achieves SPEC016 compliance, simple architecture
- **Cons:** Porting effort required

**Option B: Fix legacy mixer to comply with SPEC016 (alternative)**
- Refactor legacy mixer to read pre-faded samples
- Remove fade curve application from mixer
- Delete correct mixer (unused)
- **Pros:** Less initial work (features already present)
- **Cons:** Complex refactoring, risk of breaking existing functionality

**Recommendation:** **Option A** per PLAN014 strategy (port features to correct mixer)

---

## Next Steps (PLAN014 Increments)

### Increment 1 (Remaining):
- ✅ TC-INV-001-01 Complete (active mixer identified)
- ✅ TC-INV-001-02 Complete (references documented)
- ⏭️ REQ-MIX-003: Add [XFD-CURV-050] to SPEC002
- ⏭️ REQ-MIX-004: Add [XFD-IMPL-025] to SPEC002
- ⏭️ REQ-MIX-006: Draft ADR (partial - defer Decision section to Increment 2)

### Increment 2 Decision:
- **Migration sub-plan required** (resolve HIGH-001 from specification issues)
- Cannot proceed with REQ-MIX-002 (cleanup) until migration complete
- ADR Decision section: Document Option A (port features to correct mixer)

### Adjusted Implementation Sequence:

1. **Increment 1:** Documentation improvements (in progress)
2. **Increment 2:** Migration sub-plan + ADR completion
3. **Increment 3:** Port features to correct mixer (REQ-MIX-007/008/009)
4. **Increment 4:** Update PlaybackEngine to use correct mixer
5. **Increment 5:** Unit tests (verify architectural compliance)
6. **Increment 6:** Integration tests
7. **Increment 7:** Crossfade tests
8. **Increment 8:** Remove legacy mixer (after all tests pass)

---

## Test Results Summary

| Test ID | Status | Result |
|---------|--------|--------|
| TC-INV-001-01 | ✅ PASS | Legacy mixer active (engine.rs:238) |
| TC-INV-001-02 | ✅ PASS | All references documented (1 active, 3 tests) |

**Investigation Phase Complete**
**Date:** 2025-01-30
