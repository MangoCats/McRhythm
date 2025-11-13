# Phase 2 Kickoff: File Organization

**Status:** READY TO IMPLEMENT
**Start Date:** 2025-11-10
**Estimated Duration:** 7-10 days (can complete incrementally)

---

## Phase Overview

**Objective:** Split 4 monolithic files into modular structures

| File | Current Lines | Target | Priority |
|------|--------------|--------|----------|
| workflow_orchestrator.rs | 2,253 | 7-8 modules (<650 lines each) | CRITICAL |
| events.rs | 1,711 | 3-4 modules (<600 lines each) | HIGH |
| params.rs | 1,450 | 4-5 modules (<400 lines each) | HIGH |
| api/ui.rs | 1,308 | 5-6 modules (<300 lines each) | MEDIUM |

**Success Criteria:**
- ✅ All 216 tests pass after each increment
- ✅ Backward compatibility maintained
- ✅ Public API unchanged
- ✅ All files <800 lines

---

## Increment 2.1: Workflow Orchestrator (CRITICAL)

**Duration:** 3-4 hours (can split across sessions)
**Current Size:** 2,253 lines (6.6% of codebase)
**Target:** 7-8 modules, largest <650 lines

### Sub-Increments (Test After Each!)

#### 2.1a: Create mod.rs Stub (15 min)
- Create `workflow_orchestrator/mod.rs`
- Copy struct definition and fields only
- No methods yet
- Test: `cargo build -p wkmp-ai`

#### 2.1b: Extract phase_scanning.rs (45 min)
- Move `phase_1_scan_files()` method (lines ~185-389)
- Move helper functions for file scanning
- Update mod.rs to call extracted function
- Test: `cargo test -p wkmp-ai`
- Commit: "Extract phase_scanning module"

#### 2.1c: Extract phase_extraction.rs (45 min)
- Move `phase_2_extract_metadata()` method (lines ~390-532)
- Move ID3 extraction helpers
- Test and commit

#### 2.1d: Extract phase_fingerprinting.rs (45 min)
- Move `phase_3_fingerprint()` method (lines ~533-906)
- Move Chromaprint/AcoustID logic
- Test and commit

#### 2.1e: Extract phase_segmenting.rs (45 min)
- Move `phase_4_segment()` method (lines ~907-1058)
- Move silence detection logic
- Test and commit

#### 2.1f: Extract phase_analyzing.rs (45 min)
- Move `phase_5_analyze()` method (lines ~1059-1156)
- Move amplitude analysis logic
- Test and commit

#### 2.1g: Extract phase_flavoring.rs (45 min)
- Move `phase_6_flavor()` method (lines ~1157-end of phases)
- Move AcousticBrainz/Essentia logic
- Test and commit

#### 2.1h: Extract entity_linking.rs (45 min)
- Move entity linking logic from PLAN024 (recent additions)
- Song/artist/album linking methods
- Test and commit

#### 2.1i: Finalize mod.rs (30 min)
- Clean up mod.rs
- Add public re-exports: `pub use self::WorkflowOrchestrator;`
- Verify phase modules are private/pub(crate)
- Verify line counts (<650 for largest)
- Test and commit: "Complete workflow_orchestrator refactoring"

### Key Considerations

**Public API Preservation:**
```rust
// External code should still work:
use wkmp_ai::services::workflow_orchestrator::WorkflowOrchestrator;
```

**Module Visibility:**
```rust
// mod.rs re-exports only public API
pub use self::WorkflowOrchestrator;

// Phase modules are internal
mod phase_scanning;     // NOT pub
mod phase_extraction;   // NOT pub
// etc.
```

**Testing Strategy:**
- Run `cargo test -p wkmp-ai` after EACH sub-increment
- If tests fail, debug immediately before proceeding
- Commit after each passing sub-increment

---

## Increment 2.2: Split events.rs (HIGH)

**Duration:** 1-2 hours
**Current Size:** 1,711 lines
**Target:** 3-4 modules, largest <600 lines

### Sub-Increments

#### 2.2a: Extract system_events.rs (30 min)
- Move system/config event types
- No dependencies on other events
- Test and commit

#### 2.2b: Extract playback_events.rs (30 min)
- Move audio playback event types
- Test and commit

#### 2.2c: Extract import_events.rs (45 min)
- Move import workflow event types
- Test and commit

#### 2.2d: Extract sse_formatting.rs (45 min)
- Move SSE serialization functions
- May reference event types from other modules
- Test and commit

#### 2.2e: Create mod.rs with re-exports (15 min)
```rust
pub use self::import_events::*;
pub use self::playback_events::*;
pub use self::system_events::*;
pub use self::sse_formatting::*;
```
- Test and commit: "Complete events.rs split"

---

## Increment 2.3: Split params.rs (HIGH)

**Duration:** 1-2 hours
**Current Size:** 1,450 lines
**Target:** 4-5 modules, largest <400 lines

### Sub-Increments (45 min each)

1. Extract `crossfade_params.rs` (CrossfadeParams, FadeCurve, etc.)
2. Extract `selector_params.rs` (SelectorParams, cooldown settings)
3. Extract `timing_params.rs` (TimingParams, duration settings)
4. Extract `flavor_params.rs` (FlavorParams, musical flavor weights)
5. Extract `system_params.rs` (SystemParams, general config)
6. Create mod.rs with wildcard re-exports

**Pattern:** Same as events.rs (pure data types, wildcard re-exports)

---

## Increment 2.4: Reorganize api/ui.rs (MEDIUM)

**Duration:** 2-3 hours
**Current Size:** 1,308 lines
**Target:** 5-6 modules, largest <300 lines

### Sub-Increments (30-45 min each)

1. Extract `dashboard_page.rs` (main dashboard handler)
2. Extract `settings_page.rs` (settings form handler)
3. Extract `library_page.rs` (library view handler)
4. Extract `import_page.rs` (import wizard handler)
5. Extract `components.rs` (shared HTML generation functions)
6. Create mod.rs with route registration

**Testing:** Test HTTP endpoints return correct HTML

---

## Session Management

### For Each Sub-Increment

**Before:**
- [ ] Read current code section
- [ ] Identify extraction boundaries
- [ ] Plan minimal changes

**During:**
- [ ] Extract code to new module
- [ ] Update imports
- [ ] Update mod.rs

**After:**
- [ ] Build: `cargo build -p wkmp-ai` (or -p wkmp-common)
- [ ] Test: `cargo test -p wkmp-ai --lib --quiet`
- [ ] Verify: All 216 tests pass
- [ ] Commit with descriptive message

### Stopping Points

**Can pause after any sub-increment** - each is independently valuable:
- After 2.1a-h: Orchestrator partially split (any sub-increment)
- After 2.1i: Orchestrator complete
- After 2.2e: Events complete
- After 2.3: Params complete
- After 2.4: UI complete

### Recovery Strategy

**If tests fail:**
1. Check build errors first
2. Review imports/visibility
3. Check for moved but not updated function calls
4. If stuck >15 min, revert sub-increment and try different approach

**If time runs out:**
- Commit current progress
- Document next sub-increment in commit message
- Can resume from any sub-increment

---

## Verification Checklist (After Each Increment)

- [ ] `cargo build -p wkmp-ai -p wkmp-common` succeeds
- [ ] Zero compiler warnings
- [ ] `cargo test -p wkmp-ai -p wkmp-common --lib --quiet` passes (216 tests)
- [ ] `wc -l module/*.rs` shows all files meet size targets
- [ ] Git commit created with passing tests
- [ ] Public API verified unchanged (backward compatibility)

---

## Implementation Order

**Recommended:** Complete in sequence (2.1 → 2.2 → 2.3 → 2.4)

**Rationale:**
- 2.1 is most critical (largest file, most complex)
- 2.2 and 2.3 are simpler (pure data types)
- 2.4 is lowest priority (UI code, medium priority)

**Can Adjust:** If 2.1 takes too long, can skip to 2.2/2.3 (simpler) and return to 2.1 later

---

## Success Metrics

**After Phase 2 Complete:**
- ✅ Largest file <800 lines (currently 2,253)
- ✅ workflow_orchestrator/ has 7-8 modules, largest <650 lines
- ✅ events/ has 3-4 modules, largest <600 lines
- ✅ params/ has 4-5 modules, largest <400 lines
- ✅ api/ui/ has 5-6 modules, largest <300 lines
- ✅ All 216 tests passing
- ✅ Zero breaking changes
- ✅ 10-15 commits (one per sub-increment)

---

## Ready to Begin!

**Next Command:** Start Increment 2.1a (Create mod.rs stub)
