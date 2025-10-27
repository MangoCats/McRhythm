# PLAN002: Fix Remove Currently Playing Passage Bug - EXECUTIVE SUMMARY

**Status:** Planning Complete (Phases 1-3)
**Date:** 2025-10-27
**Estimated Implementation:** 4-8 hours (including investigation)
**Priority:** High (Critical bug affecting core playback)

---

## READ THIS FIRST

**This is the only document you need to read to understand the entire plan.**
- Summary: ~400 lines (this file)
- For detailed test specs: See `02_test_specifications/` (read only when implementing tests)
- For full consolidated plan: See `FULL_PLAN.md` (archival only - don't read during implementation)

---

## The Problem

**Bug:** Removing the currently playing passage corrupts playback state. When a new passage is enqueued afterward, the REMOVED passage resumes playing instead of the new passage starting.

**Impact:**
- User cannot safely remove passages from queue
- Playback behavior unpredictable
- Violates user expectations

**Root Cause:**
- Queue state updated correctly
- BUT: Decoder chain and mixer NOT notified
- Stale buffer data remains, gets replayed

---

## The Fix (High Level)

**Approach:** Add "currently playing passage lifecycle management" to `remove_queue_entry()`.

**Key Changes:**
1. Detect if removed entry is currently playing
2. If yes: Clear decoder chain (stop decoding, release resources)
3. If yes: Clear mixer state (stop audio output)
4. Update queue structure (already working)
5. Start next passage if queue non-empty

**Files to Modify:**
- `wkmp-ap/src/playback/engine.rs` - Add lifecycle coordination
- `wkmp-ap/src/playback/decoder_worker.rs` - Add stop command handling (if needed)
- `wkmp-ap/src/playback/pipeline/mixer.rs` - Add clear mechanism (if needed)
- `wkmp-ap/src/playback/queue_manager.rs` - May need to return "was current?" info

---

## Requirements (8 Total)

| Req ID | Description | Priority |
|--------|-------------|----------|
| REQ-FIX-010 | Stop playback immediately when removing current passage | Critical |
| REQ-FIX-020 | Release decoder chain resources | Critical |
| REQ-FIX-030 | Clear mixer state | Critical |
| REQ-FIX-040 | Update queue structure (already working) | High |
| REQ-FIX-050 | Start next passage if queue non-empty | Critical |
| REQ-FIX-060 | No disruption when removing non-current passage | High |
| REQ-FIX-070 | Prevent removed passage from resuming | Critical |
| REQ-FIX-080 | New passage starts correctly after removal | Critical |

**All requirements have tests** (100% coverage)

---

## Test Suite (8 Tests)

### System Tests (4) - End-to-End Scenarios
- **TC-S-001:** Remove current (empty queue after) → verify stop & cleanup
- **TC-S-002:** Remove current (queue non-empty) → verify next starts
- **TC-S-003:** Remove current, enqueue new (THE BUG SCENARIO) → verify new plays, not old
- **TC-S-004:** Remove non-current → verify no disruption

### Integration Tests (2) - Component Interactions
- **TC-I-001:** Decoder chain cleanup mechanism
- **TC-I-002:** Mixer state clearing mechanism

### Unit Tests (2) - Component Behavior
- **TC-U-001:** Queue detects current vs. non-current
- **TC-U-002:** Next passage determination logic

**Critical Test:** TC-S-003 directly reproduces the reported bug. If this passes, bug is fixed.

---

## Investigation Phase (Required Before Implementation)

**Before coding, must answer 4 questions:**

### Q1: How does decoder worker receive commands?
- **Investigate:** `wkmp-ap/src/playback/decoder_worker.rs`
- **Find:** Command channel, message types
- **Need:** Add "stop chain" command type
- **Time:** 30-60 minutes

### Q2: How to clear mixer state?
- **Investigate:** `wkmp-ap/src/playback/pipeline/mixer.rs`
- **Find:** Mixer state structure, stop/reset methods
- **Need:** Method to signal "stop current"
- **Time:** 30-60 minutes

### Q3: How to identify decoder chain for passage?
- **Investigate:** `wkmp-ap/src/playback/engine.rs`
- **Find:** Chain assignment tracking (map from queue_entry_id → chain_id)
- **Need:** Query and clear chain assignment
- **Time:** 20-30 minutes

### Q4: How does natural passage end trigger next start?
- **Investigate:** Playback loop in `engine.rs`
- **Find:** Logic for "current finished → start next"
- **Need:** Reuse same mechanism for "removed current → start next"
- **Time:** 30 minutes

**Total Investigation:** 2-3 hours

---

## Specification Issues (From Phase 2)

### HIGH Issues (4) - Resolved via Investigation
All HIGH issues are questions about existing mechanisms. Investigation phase will answer these.

### MEDIUM Issues (3) - Design Decisions Needed

**ISSUE-M1:** Crossfade abort behavior
- **Decision:** Abort crossfade immediately, start next (no fade)
- **Rationale:** User explicitly removed passage, expect immediate stop

**ISSUE-M2:** Ring buffer handling
- **Decision:** Flush buffer immediately (hard stop)
- **Rationale:** "Immediate stop" requirement

**ISSUE-M3:** Remove during pause
- **Decision:** Clear without unpause
- **Rationale:** Don't play removed passage

### LOW Issues (1) - Implementation Detail
- SSE event timing - emit after cleanup complete

**No CRITICAL blockers** - Proceed with implementation

---

## Implementation Strategy

### Phase A: Investigation (2-3 hours)
1. Answer Q1-Q4 above
2. Document findings
3. Identify minimal changes needed

### Phase B: Unit Tests (1 hour)
1. Write TC-U-001 (queue current detection)
2. Write TC-U-002 (next passage selection)
3. Tests FAIL initially (expected)

### Phase C: Core Fix (2-3 hours)
1. Modify `engine.rs::remove_queue_entry()`:
   - Check if removed entry is current
   - Clear decoder chain
   - Clear mixer state
   - Start next or enter stopped state
2. Add decoder stop command (if needed)
3. Add mixer clear method (if needed)
4. Run unit tests → PASS

### Phase D: Integration Tests (1 hour)
1. Write TC-I-001 (decoder cleanup)
2. Write TC-I-002 (mixer clearing)
3. Run integration tests → PASS

### Phase E: System Tests (1-2 hours)
1. Write TC-S-001 through TC-S-004
2. Create test audio files
3. Run system tests → ALL PASS
4. **Key:** TC-S-003 must pass (the bug scenario)

### Phase F: Verification & Cleanup (30 min)
1. Manual testing with UI
2. Check for resource leaks
3. Update documentation
4. Commit with reference to BUG001

**Total:** 7.5-11.5 hours (with investigation)

---

## Success Criteria

**Fix is complete when:**
1. ✓ All 8 tests pass
2. ✓ TC-S-003 specifically passes (bug scenario)
3. ✓ Manual testing confirms fix
4. ✓ No resource leaks (file handles, memory)
5. ✓ No regressions in existing playback
6. ✓ Code reviewed and commented
7. ✓ Committed with BUG001 reference

---

## Scope

### In Scope
- Detect removal of currently playing passage
- Clear decoder chain resources
- Clear mixer state
- Handle queue state transitions (empty vs. non-empty)
- Prevent removed passage from resuming
- Test coverage for all scenarios

### Out of Scope
- "Clear Queue" verification (separate investigation)
- "Skip Next" verification (separate investigation)
- General playback lifecycle refactoring (future work)
- Performance optimization
- UI changes

---

## Dependencies

### Code Dependencies (Exist, Will Modify)
- `wkmp-ap/src/playback/engine.rs:1465` - `remove_queue_entry()`
- `wkmp-ap/src/playback/queue_manager.rs:307` - `QueueManager::remove()`
- `wkmp-ap/src/playback/decoder_worker.rs` - Command handling
- `wkmp-ap/src/playback/pipeline/mixer.rs` - State management

### No New External Dependencies
- Uses tokio (async runtime) - already in use
- Uses existing WKMP infrastructure

---

## Risks

### Primary Risks
1. **Decoder/mixer mechanisms unclear** (HIGH issues)
   - **Mitigation:** Investigation phase resolves this
   - **Fallback:** Design minimal mechanism if none exists

2. **State synchronization issues**
   - **Mitigation:** Use existing async patterns
   - **Fallback:** Add necessary locking

3. **Regression in working scenarios**
   - **Mitigation:** Comprehensive test coverage
   - **Fallback:** Rollback if tests fail

### Risk Acceptance
- Bug is critical, affects core functionality
- Risk of fix > risk of leaving bug unfixed
- Changes are localized and testable

---

## How to Use This Plan

### For Implementation:

**Step 1:** Read this summary (you're doing it!)

**Step 2:** Investigation phase
- Answer Q1-Q4 (see "Investigation Phase" section above)
- Document findings

**Step 3:** For each test
- Read test spec: `02_test_specifications/tc_X_YYY.md`
- Implement to pass that test
- ~600 lines context: summary + current test spec

**Step 4:** Iterate through implementation strategy (Phases B-F)

**Step 5:** Verify all tests pass

### DO NOT READ:
- `FULL_PLAN.md` during implementation (context overload)
- All test specs at once (read one at a time)

### DO READ:
- This summary (400 lines)
- Current test spec (~100 lines)
- Requirements index if needed (~100 lines)

**Total context: ~600 lines** (optimal for AI/human)

---

## Key Insights from Planning

### What We Learned (Phase 2)
1. Bug is state synchronization issue, not logic error
2. Queue management works, coordination doesn't
3. Natural passage end works (proves mechanisms exist)
4. Fix is targeted - touch 1-4 files only

### Critical Success Factors
1. **Investigation first** - understand mechanisms before coding
2. **Test-driven** - write tests, then make them pass
3. **TC-S-003 is key** - direct reproduction of bug
4. **Resource cleanup** - prevent leaks (file handles)

### Confidence Level
- **High confidence** bug can be fixed
- **Medium confidence** on effort estimate (depends on investigation)
- **High confidence** approach is correct

---

## Related Documents

### In This Plan Folder
- **requirements_index.md** - Complete requirements list
- **scope_statement.md** - Detailed scope boundaries
- **dependencies_map.md** - What exists, what's needed
- **01_specification_issues.md** - Issues analysis (HIGH/MEDIUM/LOW)
- **02_test_specifications/** - Individual test specs
  - test_index.md - Test quick reference
  - tc_s_001.md through tc_s_004.md - System tests
  - tc_i_001.md, tc_i_002.md - Integration tests
  - tc_u_001.md, tc_u_002.md - Unit tests
  - traceability_matrix.md - Requirements ↔ Tests ↔ Code

### External References
- **wip/BUG001_remove_currently_playing.md** - Original bug report
- **docs/SPEC001-architecture.md** - WKMP architecture (single-stream design)
- **docs/REQ001-requirements.md** - Original requirements (REQ-QUE-070, REQ-PB-010, REQ-PB-040)

---

## Questions or Issues?

**If unclear on requirements:**
- Read: `requirements_index.md`
- Read: Source bug report (BUG001)

**If unclear on a test:**
- Read: `02_test_specifications/tc_X_YYY.md` for that specific test
- See: `traceability_matrix.md` for requirement-to-test mapping

**If unclear on scope:**
- Read: `scope_statement.md`

**If unclear on dependencies:**
- Read: `dependencies_map.md`

**If implementation blocked:**
- Check: `01_specification_issues.md` - see if your question is a known issue
- Investigate: Follow investigation phase guidance

---

## Status Summary

**✅ PLANNING COMPLETE (Phases 1-3)**

- ✓ Phase 1: Scope defined, requirements extracted
- ✓ Phase 2: Specification verified, issues documented
- ✓ Phase 3: Tests defined, 100% coverage

**⏳ READY FOR IMPLEMENTATION**

Next step: Begin investigation phase (Q1-Q4)

---

## Line Count: ~425 lines

**Target: <500 lines** ✓

This summary provides everything needed to start implementation without reading thousands of lines of documentation.

**Start implementing:** Begin with investigation phase, then follow implementation strategy (Phases A-F).

**Good luck!** 🚀
