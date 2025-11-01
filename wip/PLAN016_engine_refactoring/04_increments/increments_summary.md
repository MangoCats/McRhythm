# Implementation Increments: PLAN016 Engine Refactoring

**Plan:** PLAN016
**Total Increments:** 6
**Estimated Total Effort:** 8-12 hours
**Approach:** Functional Decomposition (Approach A)

---

## Increment Overview

| Inc | Name | Objective | Effort | Tests | Deliverable |
|-----|------|-----------|--------|-------|-------------|
| 1 | Baseline & Analysis | Establish baseline, analyze code | 1-2h | 3 baseline tests | Code mapping + baseline metrics |
| 2 | Module Structure | Create directory skeleton | 0.5h | 2 structure tests | Compiling skeleton |
| 3 | Extract Queue Module | Move queue operations to queue.rs | 2-3h | 3 tests | queue.rs <1500 lines, tests pass |
| 4 | Extract Diagnostics | Move diagnostics to diagnostics.rs | 2-3h | 3 tests | diagnostics.rs <1500 lines, tests pass |
| 5 | Finalize Core Module | Clean up core.rs, create mod.rs | 1h | 3 tests | core.rs <1500 lines, mod.rs re-exports |
| 6 | Verification & Cleanup | Delete engine.rs, verify all tests | 1-2h | All 11 tests | 100% verification complete |

---

## Increment 1: Baseline & Analysis

**Objective:** Establish test baseline and map code to modules

**Duration:** 1-2 hours

**Prerequisites:**
- Current engine.rs exists and compiles
- Test suite available

**Tasks:**

1. **Establish Test Baseline** (30 min)
   ```bash
   # Run baseline tests
   cargo test -p wkmp-ap 2>&1 | tee test_baseline.log

   # Extract metrics
   grep "test result" test_baseline.log > test_baseline.txt

   # Document results
   echo "Baseline: $(grep -oP '\d+ passed' test_baseline.txt)" >> baseline_metrics.txt
   ```

2. **Document Public API** (15 min)
   ```bash
   # Extract all pub fn from engine.rs
   grep -n "pub fn" wkmp-ap/src/playback/engine.rs > public_api_baseline.txt

   # Count public methods
   wc -l public_api_baseline.txt
   ```

3. **Measure Original Line Count** (5 min)
   ```bash
   wc -l wkmp-ap/src/playback/engine.rs > original_line_count.txt
   ```

4. **Create Code-to-Module Mapping** (30-45 min)

   Using analysis from Phase 4 (03_approach_selection.md), create detailed mapping:

   **core.rs** - Lines to extract:
   - Struct definition: Lines 71-178
   - new(): Lines 185-329
   - start(): Lines 371-831
   - stop(): Lines 837-873
   - play(): Lines 878-935
   - pause(): Lines 942-999
   - seek(): Lines 1172-1242
   - playback_loop(): Lines 2116-2142
   - process_queue(): Lines 2145-2725
   - assign_chain(), release_chain(): Lines 3104-3160
   - Helpers: Lines 2728-2886, 3060-3087

   **queue.rs** - Lines to extract:
   - skip_next(): Lines 1011-1094
   - clear_queue(): Lines 1103-1156
   - enqueue_file(): Lines 1673-1752
   - remove_queue_entry(): Lines 1760-1835
   - emit_queue_change_events(): Lines 1846-1872
   - complete_passage_removal(): Lines 1890-1923
   - reorder_queue_entry(): Lines 1609-1634
   - get_queue_entries(), queue_len(): Lines 1308-1337, 1298-1301

   **diagnostics.rs** - Lines to extract:
   - Status accessors: Lines 1250-1666
   - Monitoring config: Lines 3558-3571
   - Event handlers: Lines 2897-3057, 3169-3469
   - Emitters: Lines 3482-3616

   Document in `code_mapping.md`

**Deliverables:**
- `test_baseline.txt` - Baseline test results
- `public_api_baseline.txt` - List of public methods
- `original_line_count.txt` - Original file size
- `code_mapping.md` - Detailed line-by-line mapping

**Tests to Run:**
- TC-I-030-02 (baseline - pre-refactoring)
- TC-U-030-01 (baseline - document API)
- TC-S-020-01 (baseline - measure lines)

**Success Criteria:**
- Baseline tests documented
- Public API cataloged (expect ~28 public methods)
- Original line count recorded (4,251 lines)
- Code mapping created with line number ranges

---

## Increment 2: Module Structure Creation

**Objective:** Create compiling module skeleton

**Duration:** 30 minutes

**Prerequisites:**
- Increment 1 complete
- Code mapping available

**Tasks:**

1. **Create Directory Structure** (5 min)
   ```bash
   # Create engine/ directory
   mkdir wkmp-ap/src/playback/engine

   # Create empty module files
   touch wkmp-ap/src/playback/engine/mod.rs
   touch wkmp-ap/src/playback/engine/core.rs
   touch wkmp-ap/src/playback/engine/queue.rs
   touch wkmp-ap/src/playback/engine/diagnostics.rs
   ```

2. **Create mod.rs Skeleton** (10 min)
   ```rust
   // mod.rs
   //! Playback engine module
   //!
   //! **[PLAN016]** Refactored from monolithic engine.rs (4,251 lines)
   //! into functional modules for improved maintainability.

   mod core;
   mod queue;
   mod diagnostics;

   // Re-export public API
   pub use core::PlaybackEngine;

   // All public methods available via impl blocks in submodules
   ```

3. **Create Module Skeletons** (15 min)

   **core.rs:**
   ```rust
   //! Core playback engine - State management & lifecycle
   //!
   //! **[PLAN016]** Contains:
   //! - PlaybackEngine struct definition
   //! - Lifecycle methods (new, start, stop, play, pause)
   //! - Orchestration (playback_loop, process_queue)
   //! - Chain allocation

   use super::*; // Import from parent (engine/)

   // TODO: Move PlaybackEngine struct here
   // TODO: Move impl PlaybackEngine (lifecycle methods)
   ```

   **queue.rs:**
   ```rust
   //! Queue operations
   //!
   //! **[PLAN016]** Contains:
   //! - Queue manipulation (enqueue, remove, skip, clear)
   //! - Queue queries (get_entries, queue_len, reorder)

   use super::*;

   // TODO: Move impl PlaybackEngine (queue methods)
   ```

   **diagnostics.rs:**
   ```rust
   //! Status queries and monitoring
   //!
   //! **[PLAN016]** Contains:
   //! - Status accessors (get_buffer_chains, get_metrics)
   //! - Event handlers (position_event_handler, buffer_event_handler)
   //! - Periodic emitters (buffer_chain_status, position)

   use super::*;

   // TODO: Move impl PlaybackEngine (diagnostic methods)
   // TODO: Move event handler functions
   ```

4. **Update playback/mod.rs** (no change needed)

   Rust automatically recognizes `engine/mod.rs` as the module.
   Existing `pub mod engine;` works unchanged.

5. **Verify Compilation** (5 min)
   ```bash
   cargo check -p wkmp-ap
   # Should compile (empty modules okay)
   ```

**Deliverables:**
- `engine/` directory created
- 4 module files with skeleton code
- Compiles without errors

**Tests to Run:**
- TC-U-010-01 (directory structure validation)
- TC-I-010-01 (module compilation)

**Success Criteria:**
- Directory structure matches SPEC024 design
- All 4 files exist
- `cargo check -p wkmp-ap` succeeds
- No code moved yet (skeleton only)

---

## Increment 3: Extract Queue Module

**Objective:** Move all queue-related code to queue.rs

**Duration:** 2-3 hours

**Prerequisites:**
- Increment 2 complete
- Code mapping from Increment 1

**Tasks:**

1. **Move Queue Operation Methods** (60-90 min)

   From engine.rs, move to queue.rs:
   - `skip_next()` (lines 1011-1094)
   - `clear_queue()` (lines 1103-1156)
   - `enqueue_file()` (lines 1673-1752)
   - `remove_queue_entry()` (lines 1760-1835)
   - `reorder_queue_entry()` (lines 1609-1634)

   **Process for each method:**
   ```bash
   # 1. Copy method from engine.rs to queue.rs
   # 2. Wrap in: impl PlaybackEngine { ... }
   # 3. Update visibility if needed (keep pub)
   # 4. Comment out in engine.rs (don't delete yet)
   # 5. Compile: cargo check -p wkmp-ap
   # 6. Fix imports in queue.rs
   # 7. If compiles, proceed to next method
   ```

2. **Move Queue Query Methods** (30 min)
   - `get_queue_entries()` (lines 1308-1337)
   - `queue_len()` (lines 1298-1301)

3. **Move Queue Helper Functions** (15 min)
   - `emit_queue_change_events()` (lines 1846-1872)
   - `complete_passage_removal()` (lines 1890-1923)

4. **Add Imports to queue.rs** (15 min)
   ```rust
   use crate::playback::{
       buffer_manager::BufferManager,
       queue_manager::QueueManager,
       // ... other imports
   };
   use sqlx::SqlitePool;
   use tokio::sync::RwLock;
   use std::sync::Arc;
   // ... complete import list
   ```

5. **Update core.rs Imports** (10 min)

   If core.rs calls queue methods (e.g., skip_next()):
   - Methods available via &self receiver (no import needed)

6. **Verify Line Count** (5 min)
   ```bash
   wc -l wkmp-ap/src/playback/engine/queue.rs
   # Expect: ~1,250 lines
   # Max: 1,499 lines
   ```

7. **Run Tests** (10 min)
   ```bash
   cargo test -p wkmp-ap
   # All baseline tests should still pass
   ```

**Deliverables:**
- queue.rs contains all queue operations (~1,250 lines)
- Code commented out in engine.rs (not deleted yet)
- All tests pass

**Tests to Run:**
- TC-I-010-01 (compilation)
- TC-U-020-01 (queue.rs line count)
- TC-I-030-02 (test suite pass rate)

**Success Criteria:**
- queue.rs <1500 lines
- All queue methods compile in queue.rs
- 100% of baseline tests pass
- No compilation errors

**Checkpoint:** If tests fail, revert queue.rs changes and debug before proceeding.

---

## Increment 4: Extract Diagnostics Module

**Objective:** Move diagnostics code to diagnostics.rs

**Duration:** 2-3 hours

**Prerequisites:**
- Increment 3 complete and verified
- queue.rs stable

**Tasks:**

1. **Move Status Accessor Methods** (45 min)
   - `get_volume_arc()` (lines 1250-1252)
   - `get_buffer_manager()` (lines 1264-1266)
   - `is_audio_expected()` (lines 1275-1277)
   - `get_callback_stats()` (lines 1285-1288)
   - `get_buffer_chains()` (lines 1338-1527)
   - `verify_queue_sync()` (lines 1546-1600)
   - `get_buffer_statuses()` (lines 1664-1666)
   - `get_pipeline_metrics()` (lines 3628-3655)

2. **Move Monitoring Config Methods** (10 min)
   - `set_buffer_monitor_rate()` (lines 3558-3561)
   - `trigger_buffer_monitor_update()` (lines 3568-3571)

3. **Move Event Handler Functions** (90-120 min)

   These are NOT impl methods - standalone async functions:
   - `position_event_handler()` (lines 2897-3057, 160 lines)
   - `buffer_event_handler()` (lines 3169-3469, 300 lines)
   - `buffer_chain_status_emitter()` (lines 3482-3550, 68 lines)
   - `playback_position_emitter()` (lines 3579-3616, 37 lines)

   **Note:** These functions are called in start() method (core.rs).
   Update calls:
   ```rust
   // In core.rs start() method:
   tokio::spawn(diagnostics::position_event_handler(...));
   tokio::spawn(diagnostics::buffer_event_handler(...));
   ```

4. **Add Imports to diagnostics.rs** (15 min)
   ```rust
   use crate::playback::{
       events::PlaybackEvent,
       buffer_manager::BufferManager,
       // ... complete imports
   };
   use tokio::sync::broadcast;
   // ... etc.
   ```

5. **Make Handlers Public** (5 min)
   ```rust
   // In diagnostics.rs
   pub async fn position_event_handler(...) { }
   pub async fn buffer_event_handler(...) { }
   pub async fn buffer_chain_status_emitter(...) { }
   pub async fn playback_position_emitter(...) { }
   ```

6. **Update core.rs start() Method** (15 min)

   Change handler spawns from local functions to module functions:
   ```rust
   // Old (in engine.rs):
   tokio::spawn(position_event_handler(...));

   // New (in core.rs):
   tokio::spawn(super::diagnostics::position_event_handler(...));
   ```

7. **Verify Line Count** (5 min)
   ```bash
   wc -l wkmp-ap/src/playback/engine/diagnostics.rs
   # Expect: ~1,035 lines
   ```

8. **Run Tests** (10 min)
   ```bash
   cargo test -p wkmp-ap
   ```

**Deliverables:**
- diagnostics.rs contains all diagnostic code (~1,035 lines)
- Code commented out in engine.rs
- All tests pass

**Tests to Run:**
- TC-I-010-01 (compilation)
- TC-U-020-01 (diagnostics.rs line count)
- TC-I-030-02 (test suite pass rate)

**Success Criteria:**
- diagnostics.rs <1500 lines
- All handlers compile and spawn correctly
- 100% of baseline tests pass

**Checkpoint:** If handlers fail to spawn, debug async lifetime issues before proceeding.

---

## Increment 5: Finalize Core Module

**Objective:** Complete core.rs and create mod.rs re-exports

**Duration:** 1 hour

**Prerequisites:**
- Increment 4 complete
- queue.rs and diagnostics.rs verified

**Tasks:**

1. **Move Remaining Code to core.rs** (30 min)

   All code NOT in queue.rs or diagnostics.rs goes to core.rs:
   - PlaybackEngine struct (lines 71-178)
   - Lifecycle methods (new, start, stop, play, pause, seek)
   - Orchestration (playback_loop, process_queue)
   - Chain allocation (assign_chain, release_chain)
   - Helpers (clone_handles, etc.)

   **Important:** Copy struct definition to core.rs, then add:
   ```rust
   // At top of core.rs
   pub struct PlaybackEngine {
       // ... fields
   }
   ```

2. **Update mod.rs with Re-Exports** (15 min)
   ```rust
   // engine/mod.rs
   mod core;
   mod queue;
   mod diagnostics;

   // Re-export PlaybackEngine struct
   pub use core::PlaybackEngine;

   // Note: Public methods auto-available via impl blocks
   // No need to re-export individual methods
   ```

3. **Add Import Statements** (10 min)

   At top of core.rs, queue.rs, diagnostics.rs:
   ```rust
   use crate::playback::PlaybackEngine; // If needed
   ```

4. **Verify Line Count for core.rs** (5 min)
   ```bash
   wc -l wkmp-ap/src/playback/engine/core.rs
   # Expect: ~1,380 lines
   # Max: 1,499 lines
   ```

5. **Verify All Modules Compile** (5 min)
   ```bash
   cargo check -p wkmp-ap
   ```

**Deliverables:**
- core.rs complete (~1,380 lines)
- mod.rs with proper re-exports (~100 lines)
- All modules compile

**Tests to Run:**
- TC-I-010-01 (compilation)
- TC-U-020-01 (all 4 files line counts)
- TC-I-030-01 (handlers compile unchanged)

**Success Criteria:**
- core.rs <1500 lines
- mod.rs <100 lines
- All modules compile
- No errors

---

## Increment 6: Verification & Cleanup

**Objective:** Delete old file, verify all tests

**Duration:** 1-2 hours

**Prerequisites:**
- Increment 5 complete
- All modules verified

**Tasks:**

1. **Delete Original engine.rs** (2 min)
   ```bash
   # Double-check first!
   ls -la wkmp-ap/src/playback/engine.rs
   ls -la wkmp-ap/src/playback/engine/

   # Delete
   rm wkmp-ap/src/playback/engine.rs
   ```

2. **Verify Compilation After Deletion** (2 min)
   ```bash
   cargo check -p wkmp-ap
   # Must succeed
   ```

3. **Run Full Test Suite** (10 min)
   ```bash
   cargo test -p wkmp-ap 2>&1 | tee refactored_tests.log
   ```

4. **Compare Test Results** (5 min)
   ```bash
   diff <(grep "test result" test_baseline.log) <(grep "test result" refactored_tests.log)
   # Should show no differences
   ```

5. **Run All 11 Acceptance Tests** (30-45 min)

   Execute each test from 02_test_specifications/:
   - TC-U-010-01: Directory structure ✓
   - TC-I-010-01: Module compilation ✓
   - TC-S-010-01: File count (should be 4) ✓
   - TC-U-020-01: Line counts (all <1500) ✓
   - TC-S-020-01: Total lines (≈4,251 ±5%) ✓
   - TC-U-030-01: Public API surface unchanged ✓
   - TC-I-030-01: Handlers compile unchanged ✓
   - TC-I-030-02: Test suite pass rate (100%) ✓
   - TC-S-030-01: API compatibility ✓
   - TC-S-010-02: Manual code organization review ✓
   - TC-S-030-02: Manual external caller review ✓

6. **Verify Handlers Compile Unchanged** (5 min)
   ```bash
   cargo check -p wkmp-ap --lib
   # Specifically check handlers.rs compiles
   ```

7. **Manual Code Organization Review** (15-20 min)

   Review each module for:
   - Logical grouping (functions related?)
   - Clear responsibilities (single purpose?)
   - No code duplication
   - Proper imports

8. **Manual External Caller Review** (10 min)

   Check that no external code needed changes:
   - handlers.rs - no changes
   - tests - no changes
   - main.rs - no changes

9. **Update Traceability Matrix** (10 min)

   In 02_test_specifications/traceability_matrix.md:
   - Update "Status" column for all requirements (Pending → Complete)
   - Update "Test Results" table with pass/fail
   - Record date of verification

10. **Create Implementation Report** (15-20 min)

    Document:
    - Actual vs. estimated effort
    - Any deviations from plan
    - Issues encountered
    - Final metrics (line counts, test results)

**Deliverables:**
- engine.rs deleted
- All 11 tests pass
- Traceability matrix updated
- Implementation report created

**Tests to Run:**
- ALL 11 acceptance tests (complete suite)

**Success Criteria:**
- ✅ engine.rs file does NOT exist
- ✅ engine/ directory exists with 4 files
- ✅ All files <1500 lines
- ✅ Total lines ≈4,251 ±5%
- ✅ 100% baseline tests pass
- ✅ Handlers compile unchanged
- ✅ All 11 acceptance tests pass
- ✅ Traceability matrix 100% complete

**Final Checkpoint:** If ANY test fails, investigate and fix before marking refactoring complete.

---

## Increment Dependencies

```
Increment 1 (Baseline)
    ↓
Increment 2 (Structure)
    ↓
Increment 3 (Queue) ←─────┐
    ↓                      │
Increment 4 (Diagnostics) ─┤
    ↓                      │
Increment 5 (Core) ←───────┘
    ↓
Increment 6 (Verification)
```

**Critical Path:** All increments are sequential. Each must complete successfully before proceeding to next.

---

## Rollback Plan

**If any increment fails:**

1. **Identify Failure Point:**
   - Which increment?
   - Which task?
   - What error?

2. **Rollback Options:**

   **Option A: Revert Last Increment**
   ```bash
   git reset --hard HEAD~1
   # OR
   git checkout <previous_commit>
   ```

   **Option B: Fix Forward**
   - If error is simple (import, typo): Fix and continue
   - If error is complex: Revert and rethink approach

3. **Re-verify Before Continuing:**
   ```bash
   cargo test -p wkmp-ap
   # Ensure tests pass before retrying
   ```

**Safe Points for Rollback:**
- After Increment 1: Baseline established, no code changes
- After Increment 2: Skeleton only, easily deleted
- After Increment 3: queue.rs isolated, can revert
- After Increment 4: diagnostics.rs isolated, can revert
- After Increment 5: All code moved but engine.rs still exists (fallback available)

**Point of No Return:** Increment 6 (engine.rs deleted) - ensure ALL tests pass in Increment 5 before proceeding.

---

## Checkpoints

**Checkpoint 1 (After Increment 3):**
- Verify: queue.rs compiles, tests pass
- Decision: Proceed to Increment 4 OR pause to address issues

**Checkpoint 2 (After Increment 4):**
- Verify: diagnostics.rs compiles, handlers spawn correctly
- Decision: Proceed to Increment 5 OR pause

**Checkpoint 3 (After Increment 5):**
- Verify: All 3 modules compile, tests pass, line counts <1500
- Decision: Proceed to delete engine.rs (Increment 6) OR pause for review

**Checkpoint 4 (After Increment 6):**
- Verify: All 11 tests pass, traceability complete
- Decision: Mark refactoring complete OR fix remaining issues

---

**Implementation Breakdown Complete**
**Phase 5 Status:** ✓ 6 increments defined with detailed tasks
**Estimated Total Effort:** 8-12 hours
**Next Phase:** Effort Estimation (Phase 6)
