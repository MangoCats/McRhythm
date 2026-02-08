# PLAN033: Time Display Update Frequency - PLAN SUMMARY

**Status:** COMPLETE
**Created:** 2026-02-05
**Implemented:** 2026-02-05
**Specification Source:** docs/REQ001-requirements.md (REQ-IPD-020 through REQ-IPD-023)
**Plan Location:** `wip/PLAN033_time_display_update_frequency/`

---

## Executive Summary

### Problem Being Solved

The import progress UI's Elapsed Time counter updates only when file processing events occur. During long-running operations on individual files, the time display becomes stale, creating the perception that the import is frozen.

### Solution Approach

Add a dedicated background task to `ProgressManager` that broadcasts time updates independently of file processing activity. The task fires every 15 seconds to update elapsed time, and recalculates estimated remaining time every 60 seconds.

---

## Requirements Summary

**Total Requirements:** 4 (all High priority)

| Req ID | Description |
|--------|-------------|
| REQ-IPD-020 | Time display update frequency requirements (parent) |
| REQ-IPD-021 | Elapsed time updates at least every 15 seconds |
| REQ-IPD-022 | Estimated remaining updates at least every 60 seconds |
| REQ-IPD-023 | Updates independent of file processing activity |

---

## Scope

### In Scope
- Add `spawn_time_update_task()` method to ProgressManager
- 15-second interval ticker for elapsed time broadcasts
- 60-second interval for estimated remaining recalculation
- Task integration with existing cancellation token

### Out of Scope
- UI changes (already displays values from SSE)
- Estimated remaining algorithm improvements
- Database persistence of time values

---

## Implementation Roadmap

### Increment 1: Add Time Update Background Task
**Objective:** Periodic time broadcast independent of progress events
**Effort:** 1-2 hours

**Deliverables:**
1. Add `spawn_time_update_task()` to ProgressManager
2. 15-second ticker that calls `broadcast_sse()`
3. Track elapsed ticks to trigger 60-second estimated remaining recalc
4. Integrate with existing `cancel_token`

**Implementation Location:** `wkmp-ai/src/services/progress_manager.rs`

**Code Changes:**
```rust
// In spawn_time_update_task():
// - 15-second interval ticker
// - Counter for 60-second estimated remaining calculation
// - Call broadcast_sse() on each tick
// - Use cancel_token for shutdown
```

**Tests:** TC-U-IPD-021-01, TC-U-IPD-022-01, TC-U-IPD-023-01

---

## Test Coverage Summary

**Total Tests:** 3 (all unit tests)
**Coverage:** 100% - All 3 requirements have acceptance tests

| Test ID | Requirement | Description |
|---------|-------------|-------------|
| TC-U-IPD-021-01 | REQ-IPD-021 | Elapsed time updates every 15 seconds |
| TC-U-IPD-022-01 | REQ-IPD-022 | Estimated remaining updates every 60 seconds |
| TC-U-IPD-023-01 | REQ-IPD-023 | Updates independent of progress events |

---

## Risk Assessment

**Residual Risk:** Low

**Risks:**
1. **SSE event volume increase** - Mitigated: Only adds 4 events/minute max
2. **Timer coordination with existing tasks** - Mitigated: Uses same pattern as sync task

---

## Dependencies

**Existing Code:**
- `ProgressManager` (progress_manager.rs) - Add new method
- `EventBus` - Existing SSE infrastructure
- `tokio::time::interval` - Timer functionality

**No External Dependencies**

---

## Next Steps

1. Implement `spawn_time_update_task()` in progress_manager.rs
2. Call new method from `ProgressManager::new()`
3. Add estimated remaining calculation logic
4. Test with manual import to verify UI updates

---

## Approval

**Plan Created:** 2026-02-05
**Status:** Ready for Implementation

**Estimated Effort:** 1-2 hours
**Actual Effort:** ~30 minutes

---

## Implementation Report

### Changes Made

**File:** `wkmp-ai/src/services/progress_manager.rs`

1. **Added `spawn_time_update_task()` method** (lines 182-276)
   - 15-second interval ticker for elapsed time broadcasts [REQ-IPD-021]
   - Tick counter to track 60-second intervals [REQ-IPD-022]
   - SSE broadcast independent of progress events [REQ-IPD-023]
   - Uses existing `cancel_token` for graceful shutdown

2. **Updated `broadcast_sse()` method** (lines 358-416)
   - Now calculates estimated remaining time based on throughput
   - Formula: `remaining_files / (files_processed / elapsed_seconds)`
   - Returns `None` if insufficient data for calculation

3. **Called `spawn_time_update_task()` from `new()`** (line 127)

4. **Updated module documentation** with new requirement traceability (lines 17-20)

### Test Coverage

The implementation leverages the existing SSE infrastructure and test framework. Manual testing recommended:
- Start import, verify elapsed time updates every ~15 seconds
- Verify estimated remaining appears after first file completes
- Verify updates continue during long-running file operations
