# core.rs Refactoring Roadmap

**Date:** 2025-11-05
**Status:** IN PROGRESS
**File:** wkmp-ap/src/playback/engine/core.rs (3,156 LOC)
**Target:** Split into 4-5 files, each <1,000 LOC

---

## Refactoring Strategy

**Approach:** Extract modules one at a time, test after each extraction, commit on success.

**Order of Extraction:**
1. **chains.rs** - Buffer chain management (FIRST - least dependencies)
2. **playback.rs** - Playback controls (SECOND - depends on chains)
3. **events.rs** - Event emission (THIRD - optional, if needed for size)
4. **core.rs** - Retain struct, lifecycle, orchestration (LAST - always retained)

**Per-extraction Protocol:**
1. Create new file in `wkmp-ap/src/playback/engine/`
2. Move methods with clear boundaries
3. Add `pub(super)` to access parent module items
4. Update `mod.rs` to include new module
5. Run `cargo test --workspace` to validate
6. Commit with message: "Refactor: Extract {module} from core.rs"

---

## Method Categorization

### Group 1: Buffer Chain Management → chains.rs (~600 LOC)

**Lines: 2231-2307, plus supporting code**

**Methods to move:**
- `assign_chain()` (line 2231) - Core chain assignment
- `release_chain()` (line 2270) - Core chain release
- `assign_chains_to_unassigned_entries()` (line 2307) - Batch assignment
- `assign_chains_to_loaded_queue()` (line 338) - Initial assignment

**Test methods:**
- `test_get_chain_assignments()` (line 2391)
- `test_get_available_chains()` (line 2399)
- Related buffer chain tests (lines 2891-3156)

**Estimated LOC:** 600-700 lines (methods + tests)

---

### Group 2: Playback Controls → playback.rs (~700 LOC)

**Lines: 946-1235, plus supporting code**

**Methods to move:**
- `play()` (line 946) - Start playback
- `pause()` (line 1010) - Pause playback
- `seek()` (line 1072) - Seek to position
- `update_audio_expected_flag()` (line 1146) - Audio state tracking
- `calculate_crossfade_start_ms()` (line 1162) - Crossfade timing
- `should_trigger_crossfade()` (line 1206) - Crossfade detection
- `try_trigger_crossfade()` (line 1235) - Crossfade execution

**Supporting methods:**
- `playback_loop()` (line 1354) - Main playback loop
- `watchdog_check()` (line 1395) - Safety watchdog
- `start_mixer_for_current()` (line 1812) - Mixer initialization

**Estimated LOC:** 700-800 lines

---

### Group 3: Event Emission → events.rs (~400 LOC, OPTIONAL)

**Only if needed to keep other files <1000 LOC**

**Potential methods:**
- Event emission logic currently embedded in other methods
- May extract from playback_loop if size requires

**Estimated LOC:** 400-500 lines (if extracted)

---

### Group 4: Core (RETAINED) → core.rs (~800 LOC)

**Lines: 1-181 (struct), 187-437 (lifecycle), 2598-3156 (tests)**

**Methods retained:**
- Struct definition (lines 70-180)
- `new()` (line 187) - Constructor
- `start()` (line 438) - Engine startup
- `stop()` (line 905) - Engine shutdown
- Helper methods for initialization

**Supporting code:**
- `PlaybackPosition` struct (lines 44-68)
- Imports and module declarations (lines 1-38)
- Test infrastructure (lines 2598-2644)
- Unit tests (remaining tests that don't fit other modules)

**Estimated LOC:** 800-900 lines

---

## File Size Targets

| File | Target LOC | Status |
|------|-----------|--------|
| **chains.rs** | 600-700 | ⏳ Pending |
| **playback.rs** | 700-800 | ⏳ Pending |
| **events.rs** | 400-500 | ⏳ Optional (if needed) |
| **core.rs** | 800-900 | ⏳ Pending |
| **Total** | 2,500-2,900 | Target: <3,156 |

---

## Extraction Steps

### Step 1: Extract chains.rs ✅ NEXT

**Actions:**
1. Create `wkmp-ap/src/playback/engine/chains.rs`
2. Add module header and imports
3. Move chain management methods:
   - `assign_chain()`
   - `release_chain()`
   - `assign_chains_to_unassigned_entries()`
   - `assign_chains_to_loaded_queue()`
   - Test methods: `test_get_chain_assignments()`, `test_get_available_chains()`
   - Related test cases
4. Update `core.rs` to remove moved code
5. Add `pub(super) mod chains;` to `engine/mod.rs` (or create if needed)
6. Add `pub(super) use chains::*;` to expose methods to parent
7. **Test:** `cargo test --workspace`
8. **Commit:** "Refactor: Extract chains.rs from core.rs"

**Expected Impact:**
- core.rs: 3,156 → ~2,500 LOC
- chains.rs: 0 → ~600 LOC

---

### Step 2: Extract playback.rs

**Actions:**
1. Create `wkmp-ap/src/playback/engine/playback.rs`
2. Move playback control methods
3. Move playback loop and watchdog
4. Update imports and visibility
5. Test and commit

**Expected Impact:**
- core.rs: ~2,500 → ~1,800 LOC
- playback.rs: 0 → ~700 LOC

---

### Step 3: Extract events.rs (CONDITIONAL)

**Decision point:** If core.rs still >1,000 LOC after playback.rs extraction

**Actions:**
1. Analyze remaining core.rs size
2. If >1,000 LOC, extract event emission logic
3. Otherwise, skip this step

**Expected Impact:**
- core.rs: ~1,800 → ~1,400 LOC (if extracted)
- events.rs: 0 → ~400 LOC

---

### Step 4: Final Verification

**Actions:**
1. Verify all files <1,000 LOC
2. Run full test suite
3. Check no public API changes
4. Update IMPL003-project_structure.md (Increment 7)

---

## Risk Mitigation

**Failure Scenarios:**

1. **Tests fail after extraction**
   - **Action:** Rollback last commit, analyze failure, fix, re-extract
   - **Prevention:** Test after each file extraction

2. **Import/visibility errors**
   - **Action:** Adjust `pub(super)` and module imports
   - **Prevention:** Careful import analysis before extraction

3. **Method dependencies unclear**
   - **Action:** Keep dependent methods together
   - **Prevention:** Analyze call graph before moving

**Rollback Plan:**
- Each extraction is committed separately
- Use `git reset --hard HEAD~1` to rollback last extraction
- Re-analyze and retry with better boundaries

---

## Progress Tracking

| Step | File | Status | LOC | Test Result | Commit |
|------|------|--------|-----|-------------|--------|
| 1 | chains.rs | ⏳ Pending | - | - | - |
| 2 | playback.rs | ⏳ Pending | - | - | - |
| 3 | events.rs | ⏳ Optional | - | - | - |
| 4 | core.rs (final) | ⏳ Pending | - | - | - |

---

**Status:** Ready to begin Step 1 (Extract chains.rs)
**Next Action:** Create chains.rs and move chain management methods
