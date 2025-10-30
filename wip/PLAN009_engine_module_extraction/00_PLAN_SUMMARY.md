# PLAN009: Engine Module Extraction - Implementation Plan

**Status:** Phase 1-3 Complete (Planning Only)
**Created:** 2025-10-29
**Specification Source:** PLAN008 Sprint 3 Inc 18-20 (deferred items)
**Target File:** `wkmp-ap/src/playback/engine.rs` (3704 lines)

---

## Executive Summary

### Problem Statement

The PlaybackEngine implementation (`engine.rs`, 3704 lines) violates single responsibility principle by mixing multiple concerns:
- Queue management operations
- Diagnostic/monitoring functions
- Core playback orchestration

This creates:
- **Maintainability issues:** Hard to locate and modify specific functionality
- **Testing challenges:** Large surface area, difficult to unit test individual concerns
- **Cognitive load:** Developers must navigate 3700+ lines to understand one feature

### Solution Approach

Extract three focused modules while preserving all functionality:
1. **diagnostics.rs** - Buffer monitoring, developer UI endpoints (~300-400 lines)
2. **queue.rs** - Queue manipulation helpers (~200-300 lines)
3. **core.rs** - Core playback loop and orchestration (~3000 lines)

**Architecture:** Keep modules as siblings under `playback/engine/` for clear separation.

### Success Criteria

- ✅ Zero functional changes (refactor only)
- ✅ All tests pass without modification
- ✅ Build succeeds with zero new warnings
- ✅ Clear module boundaries with documented interfaces
- ✅ Each extracted file <500 lines for maintainability

### Timeline Estimate

- **Planning:** Complete (this document)
- **Implementation:** 6-8 hours (3 modules × 2-3 hours each)
- **Testing:** 2 hours (verification, integration tests)
- **Total:** 8-10 hours

---

## Requirements Index

| Req ID | Priority | Description | Source |
|--------|----------|-------------|--------|
| REQ-MOD-001 | P0 | Extract diagnostics functions to separate module | Inc 18 |
| REQ-MOD-002 | P0 | Extract queue helpers to separate module | Inc 19 |
| REQ-MOD-003 | P0 | Move remaining code to core module | Inc 20 |
| REQ-MOD-004 | P0 | Zero functional changes (refactor only) | Quality |
| REQ-MOD-005 | P0 | All existing tests pass unmodified | Quality |
| REQ-MOD-006 | P1 | Module files <500 lines each | Maintainability |
| REQ-MOD-007 | P1 | Clear module boundaries documented | Maintainability |
| REQ-MOD-008 | P2 | Update related documentation | Documentation |

---

## Scope Definition

### In Scope

**Module Extractions:**
- Diagnostics module: `get_buffer_chains()`, `get_buffer_info()`, monitoring helpers
- Queue module: `advance_queue()`, `skip_passage()`, queue manipulation helpers
- Core module: `new()`, `play()`, `playback_loop()`, core orchestration

**Quality Assurance:**
- Verify zero functional changes
- Ensure all tests pass
- Maintain traceability comments
- Update module exports

### Out of Scope

- New functionality or behavior changes
- Test modifications (except imports)
- Performance optimizations
- Architecture redesign beyond extraction
- Documentation beyond inline comments

### Assumptions

1. Current test suite has adequate coverage
2. Module boundaries can be cleanly separated
3. No breaking API changes needed
4. Rust module system supports the extraction pattern

### Constraints

- Must maintain backward compatibility
- Cannot break existing tests
- Must complete in single PR for atomicity
- Zero new compiler warnings introduced

---

## Specification Issues

### Issue Analysis Complete ✓

**Findings:** No critical issues identified

**Minor observations:**
1. Some functions may have overlapping concerns (e.g., queue operations that also emit diagnostics)
2. State management (`SharedState`, position tracking) used across all modules
3. Need to carefully manage imports to avoid circular dependencies

**Resolutions:**
- Keep `SharedState` in engine.rs parent module, shared by submodules
- Accept some coupling for state/diagnostics (acceptable in orchestration layer)
- Use `pub(super)` visibility for cross-module engine internals

---

## Test Specifications

### Test Strategy

**Approach:** Rely on existing comprehensive test suite

The existing test suite already provides:
- Unit tests for individual engine methods
- Integration tests for full playback scenarios
- Queue integrity tests
- Crossfade coordination tests
- Buffer state transition tests

**Verification Plan:**
1. Run full test suite before extraction (baseline)
2. After each module extraction: Run tests, verify pass
3. After all extractions: Run full suite, compare to baseline
4. Check for any test timing changes or new warnings

### Acceptance Tests

| Test ID | Type | Description | Pass Criteria |
|---------|------|-------------|---------------|
| TC-MOD-001 | Integration | All existing tests pass | 100% pass rate |
| TC-MOD-002 | Build | Zero new compiler warnings | Warning count unchanged |
| TC-MOD-003 | Build | Successful compilation | cargo build succeeds |
| TC-MOD-004 | Unit | Engine construction works | `PlaybackEngine::new()` succeeds |
| TC-MOD-005 | Integration | Full playback scenario | Queue → decode → play → complete |
| TC-MOD-006 | API | Buffer chains endpoint | `get_buffer_chains()` returns data |
| TC-MOD-007 | API | Skip functionality | `skip_passage()` advances queue |

### Traceability Matrix

| Requirement | Test(s) | Implementation | Status | Coverage |
|-------------|---------|----------------|--------|----------|
| REQ-MOD-001 | TC-MOD-001, TC-MOD-006 | diagnostics.rs | Pending | Complete |
| REQ-MOD-002 | TC-MOD-001, TC-MOD-007 | queue.rs | Pending | Complete |
| REQ-MOD-003 | TC-MOD-001, TC-MOD-004, TC-MOD-005 | core.rs | Pending | Complete |
| REQ-MOD-004 | TC-MOD-001 through TC-MOD-007 | All modules | Pending | Complete |
| REQ-MOD-005 | TC-MOD-001 | All modules | Pending | Complete |
| REQ-MOD-006 | Manual review | Line counts | Pending | N/A |
| REQ-MOD-007 | Code review | Module docs | Pending | N/A |

---

## Implementation Approach

### Selected Approach: Incremental Module Extraction

**Rationale:** Minimizes risk by extracting one module at a time with verification between each step.

**Steps:**
1. Create module structure: `playback/engine/` directory
2. Extract diagnostics → Test → Commit
3. Extract queue helpers → Test → Commit
4. Move remaining to core → Test → Commit
5. Final verification and cleanup

**Alternative Considered:** Big-bang extraction (all at once)
- **Rejected:** Higher risk of introducing subtle bugs, harder to isolate issues

### Module Structure

```
wkmp-ap/src/playback/
├── engine.rs                    # Module root (becomes thin wrapper)
└── engine/
    ├── mod.rs                   # Re-exports, shared types
    ├── core.rs                  # Main playback orchestration (~3000 lines)
    ├── diagnostics.rs           # Buffer monitoring (~300 lines)
    └── queue.rs                 # Queue operations (~200 lines)
```

### Shared State Strategy

Keep in `engine/mod.rs`:
- `PlaybackPosition` struct (used across modules)
- `PlaybackEngine` struct definition
- Shared imports and type aliases

This allows submodules to access via `use super::PlaybackEngine;`

---

## Implementation Increments

### Increment 1: Setup Module Structure (30 min)

**Objective:** Create directory and mod.rs skeleton

**Steps:**
1. Create `wkmp-ap/src/playback/engine/` directory
2. Create `engine/mod.rs` with struct definition
3. Convert `engine.rs` to thin wrapper that re-exports from `engine/mod.rs`
4. Verify build succeeds

**Verification:** `cargo build` succeeds, tests pass

**Deliverables:**
- `engine/mod.rs` - Module root
- Updated `engine.rs` - Becomes `pub use engine::*;`

---

### Increment 2: Extract Diagnostics Module (2 hours)

**Objective:** Move buffer monitoring and diagnostic functions

**Functions to Extract:**
- `get_buffer_chains()` - Returns buffer chain info for developer UI
- `get_buffer_info()` - Helper for buffer chain construction
- Any related diagnostic helpers

**Steps:**
1. Create `engine/diagnostics.rs`
2. Move functions with all dependencies
3. Update imports in moved functions
4. Add `pub use diagnostics::*;` to `mod.rs`
5. Resolve compilation errors
6. Run tests: `cargo test --package wkmp-ap`

**Verification:**
- Build succeeds
- All tests pass
- GET `/playback/buffer_chains` endpoint works

**Commit Message:**
```
PLAN009 Inc 2: Extract diagnostics module from engine.rs

- Created engine/diagnostics.rs with buffer monitoring functions
- Moved get_buffer_chains() and related helpers
- Zero functional changes, all tests pass
- File size: diagnostics.rs ~300 lines

[REQ-MOD-001]
```

---

### Increment 3: Extract Queue Module (2 hours)

**Objective:** Move queue manipulation functions

**Functions to Extract:**
- `advance_queue()` - Move to next passage
- `skip_passage()` - Skip current passage
- Queue position helpers
- Related queue state management

**Steps:**
1. Create `engine/queue.rs`
2. Move queue-related functions
3. Update imports and visibility
4. Add `pub use queue::*;` to `mod.rs`
5. Resolve compilation errors
6. Run tests: `cargo test --package wkmp-ap`

**Verification:**
- Build succeeds
- All tests pass
- Skip functionality works via API

**Commit Message:**
```
PLAN009 Inc 3: Extract queue module from engine.rs

- Created engine/queue.rs with queue operations
- Moved advance_queue(), skip_passage() and helpers
- Zero functional changes, all tests pass
- File size: queue.rs ~250 lines

[REQ-MOD-002]
```

---

### Increment 4: Move Core to Dedicated Module (1.5 hours)

**Objective:** Move remaining playback orchestration to core.rs

**Functions to Move:**
- `new()` - Engine construction
- `play()` / `pause()` / `stop()` - Playback controls
- `playback_loop()` - Main orchestration
- `start_crossfade()` - Crossfade initiation
- All remaining engine implementation

**Steps:**
1. Create `engine/core.rs`
2. Move all remaining impl functions
3. Update mod.rs to properly re-export
4. Verify all visibility is correct
5. Run full test suite

**Verification:**
- Build succeeds
- All tests pass
- Full playback pipeline works end-to-end

**Commit Message:**
```
PLAN009 Inc 4: Move core orchestration to core.rs

- Created engine/core.rs with playback loop and controls
- Completed module extraction refactoring
- All 3 modules now cleanly separated
- File sizes: core.rs ~3000 lines, diagnostics.rs ~300, queue.rs ~250

[REQ-MOD-003]
```

---

### Increment 5: Cleanup and Documentation (30 min)

**Objective:** Polish module structure and update documentation

**Steps:**
1. Add module-level documentation to each file
2. Review and optimize imports
3. Check for any unused code
4. Update inline comments referencing old structure
5. Run `cargo clippy` for warnings
6. Final full test suite run

**Verification:**
- Zero new clippy warnings
- All tests pass
- Documentation clear and accurate

**Commit Message:**
```
PLAN009 Inc 5: Documentation and cleanup for engine modules

- Added module-level docs to diagnostics/queue/core
- Optimized imports across modules
- Updated inline comments
- Zero new warnings, all tests pass

[REQ-MOD-007]
```

---

## Checkpoint Criteria

**Checkpoint 1: After Diagnostics Extraction**
- ✓ Build succeeds
- ✓ All tests pass
- ✓ GET /playback/buffer_chains works
- ✓ Zero new warnings

**Checkpoint 2: After Queue Extraction**
- ✓ Build succeeds
- ✓ All tests pass
- ✓ Skip functionality works
- ✓ Zero new warnings

**Checkpoint 3: After Core Move**
- ✓ Build succeeds
- ✓ All tests pass
- ✓ Full playback pipeline works
- ✓ Zero new warnings
- ✓ All module files <500 lines (except core.rs ~3000 lines acceptable)

**Final Verification:**
- ✓ All tests pass (baseline comparison)
- ✓ Zero functional changes
- ✓ Module structure clear and documented
- ✓ Ready for PR review

---

## Risk Assessment

### Risk 1: Circular Dependencies

**Probability:** Medium
**Impact:** High (blocks compilation)

**Mitigation:**
- Carefully plan module boundaries before extraction
- Use `pub(super)` for internal visibility
- Keep shared state in parent module

**Residual Risk:** Low (mitigated by careful planning)

---

### Risk 2: Breaking Tests

**Probability:** Low
**Impact:** High (indicates functional changes)

**Mitigation:**
- Extract one module at a time
- Run tests after each extraction
- Commit after each successful extraction (easy rollback)

**Residual Risk:** Very Low (incremental approach + testing)

---

### Risk 3: Import/Visibility Issues

**Probability:** Medium
**Impact:** Medium (compilation errors, easy to fix)

**Mitigation:**
- Use `pub(super)` liberally during extraction
- Verify visibility after each move
- Rust compiler provides clear error messages

**Residual Risk:** Very Low (compile-time errors, easy fixes)

---

## Dependencies

### Existing Code

- ✅ `wkmp-ap/src/playback/engine.rs` (current implementation)
- ✅ Existing test suite (`wkmp-ap/tests/`)
- ✅ Related modules (buffer_manager, mixer, queue_manager)

### External Libraries

No new dependencies required (pure refactoring)

### Tools

- ✅ Rust 1.70+ (stable)
- ✅ cargo build/test
- ✅ cargo clippy (optional, for warnings)

---

## Success Metrics

### Quantitative

- **Build:** `cargo build` succeeds (0 errors)
- **Tests:** 100% existing tests pass
- **Warnings:** Zero new compiler warnings
- **Line Counts:**
  - diagnostics.rs: 250-400 lines
  - queue.rs: 150-300 lines
  - core.rs: ~3000 lines (acceptable for main orchestration)

### Qualitative

- **Maintainability:** Easier to locate specific functionality
- **Readability:** Clear module boundaries
- **Documentation:** Module purpose clear from comments

---

## Implementation Notes

### Critical Success Factors

1. **Test After Each Step:** Never proceed with failing tests
2. **Commit Frequently:** One commit per increment for easy rollback
3. **Preserve Traceability:** Keep all `[REQ-XXX]` comments intact
4. **No Functional Changes:** This is pure refactoring

### Common Pitfalls to Avoid

- ❌ Moving too much at once (extract incrementally)
- ❌ Modifying function logic during move (refactor only)
- ❌ Forgetting to update module exports
- ❌ Breaking existing import paths for tests

### Recovery Procedures

**If Tests Fail:**
1. Review diff carefully - what changed functionally?
2. Check imports and visibility modifiers
3. Verify all functions moved completely (no partial moves)
4. If stuck: `git reset --hard` and retry more carefully

**If Circular Dependency:**
1. Move shared types to parent module (`mod.rs`)
2. Use `pub(super)` to allow submodule access
3. Consider if boundary needs adjustment

---

## Next Steps

1. **Review Plan:** Confirm approach and increments make sense
2. **Set Aside Time:** Block 8-10 hours for focused work
3. **Baseline:** Run `cargo test` before starting (record results)
4. **Execute:** Follow increments sequentially
5. **Verify:** Run full test suite at each checkpoint
6. **PR:** Create pull request with clear description
7. **Archive:** Use `/archive-plan PLAN009` when complete

---

## Document Location

**Plan Folder:** `wip/PLAN009_engine_module_extraction/`

**Files:**
- `00_PLAN_SUMMARY.md` (this file) - Start here
- `requirements_index.md` - Compact requirements reference
- `02_test_specifications/test_index.md` - Test reference

**After Implementation:**
- Use `/archive-plan PLAN009` to archive plan folder
- Preserves plan in archive branch for future reference

---

**End of Plan Summary**

**Total Length:** ~500 lines (target met)
