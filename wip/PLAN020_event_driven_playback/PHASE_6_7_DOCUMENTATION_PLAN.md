# PLAN020 Phase 6-7: Validation & Documentation Plan

**Date:** 2025-11-04
**Status:** In Progress
**Context:** PLAN020 Event-Driven Playback - Final Phases

---

## Overview

Phases 1-5 are complete with all core functionality implemented and tested. Phase 6-7 focuses on validation, documentation, and preparation for archiving the plan.

**Completion Criteria:**
- All specification documents updated to reflect event-driven architecture
- Architecture diagrams created for event flows
- Design decisions documented
- Final regression testing performed
- Implementation artifacts ready for archiving

---

## Phase 6: Validation (Estimated: 2-3 hours)

### 6.1 Regression Testing

**Objective:** Verify no regressions introduced by event-driven refactoring

**Test Suites to Run:**
- ✅ PROJ001 chain assignment tests (already passing 7/7)
- ✅ Event-driven playback tests (already passing 5/5)
- ✅ wkmp-ap full test suite
- ❌ Manual smoke testing (enqueue, playback, skip, clear queue)

**Success Criteria:**
- All existing tests pass
- No behavioral changes observed in manual testing
- Watchdog intervention counter remains at 0 during normal operation

### 6.2 Performance Verification

**Objective:** Confirm event-driven architecture achieves <1ms latency goals

**Measurements:**
- Event-driven decode latency (enqueue → decode request)
- Event-driven mixer startup latency (buffer threshold → mixer start)
- Watchdog overhead (100ms loop execution time)

**Method:** Review existing log output from Phase 5 testing sessions

**Success Criteria:**
- Decode latency <1ms (vs. old 0-100ms)
- Mixer startup latency <1ms (vs. old 0-100ms)
- Watchdog loop overhead <10ms

---

## Phase 7: Documentation (Estimated: 4-6 hours)

### 7.1 SPEC028 Update (High Priority)

**File:** [docs/SPEC028-playback_orchestration.md](../../docs/SPEC028-playback_orchestration.md)

**Current State:** Describes old polling-based `process_queue()` architecture

**Required Changes:**

1. **Executive Summary Update**
   - Replace "queue management every 100ms" with "event-driven orchestration with 100ms watchdog"
   - Add event-driven architecture overview
   - Add watchdog safety net explanation

2. **Section 3: Rename process_queue() → watchdog_check()**
   - Explain watchdog detection-only pattern
   - Document intervention logging (WARN level)
   - Explain return type change (`Result<bool>`)

3. **New Section 4: Event-Driven Architecture**
   - **4.1 Event-Driven Decode**
     - EnqueueEvent triggering (immediate decode on enqueue)
     - QueueAdvanceEvent triggering (decode on passage promotion)
     - Priority mapping (Current→Immediate, Next→Next, Queued→Prefetch)
     - Code references to queue.rs implementation

   - **4.2 Event-Driven Mixer Startup**
     - BufferEvent::ReadyForStart emission (buffer threshold crossing)
     - Event handler in diagnostics.rs
     - Shared `start_mixer_for_current()` implementation
     - Code references to core.rs and diagnostics.rs

   - **4.3 Event System Infrastructure**
     - Event channels (tokio::broadcast)
     - Event handler registration
     - Event emission locations

4. **Section 5: Watchdog Safety Net**
   - Purpose: Detection-only, not proactive
   - Intervention scenarios (decode missing, mixer not started)
   - Logging behavior (WARN on intervention)
   - Telemetry (watchdog_interventions_total counter)
   - UI visibility (green/yellow/red indicator)

5. **Section 6: Architecture Diagrams**
   - Enqueue flow (event-driven decode path)
   - Queue advance flow (promotion + decode path)
   - Buffer threshold flow (mixer startup path)
   - Watchdog intervention flow (detection + recovery)

6. **Section 7: Migration Notes**
   - What changed from polling to event-driven
   - Backward compatibility guarantees
   - Performance improvements
   - Future enhancements (SSE for watchdog status)

**Estimated Effort:** 3-4 hours

---

### 7.2 Create Event Flow Diagrams (Medium Priority)

**New File:** [wip/PLAN020_event_driven_playback/EVENT_FLOW_DIAGRAMS.md](EVENT_FLOW_DIAGRAMS.md)

**Diagrams to Create (ASCII art or Mermaid):**

1. **Enqueue Event Flow**
   ```
   User Action (enqueue_file)
   → Determine queue position
   → Add to queue
   → Map position to DecodePriority
   → request_decode() immediate
   → Decoder worker starts decode
   ```

2. **Queue Advance Event Flow**
   ```
   Passage Complete
   → complete_passage_removal()
   → Capture before state (next, queued[0])
   → Remove current from queue
   → Capture after state (current, next)
   → Detect promotions
   → Trigger decode for promoted passages
   ```

3. **Mixer Startup Event Flow**
   ```
   Decoder pushes samples
   → buffer_manager.notify_samples_appended()
   → Check if threshold crossed (3000ms)
   → Emit BufferEvent::ReadyForStart
   → Event handler in diagnostics.rs
   → start_mixer_for_current()
   → Mixer begins playback
   ```

4. **Watchdog Intervention Flow**
   ```
   100ms watchdog tick
   → Check current passage has buffer
   → If missing: WARN + request_decode()
   → Check mixer started
   → If not: WARN + start_mixer_for_current()
   → Increment intervention counter
   → Emit SSE event
   → Update UI indicator
   ```

**Format:** Mermaid sequence diagrams (better for GitHub rendering)

**Estimated Effort:** 1-2 hours

---

### 7.3 Design Decisions Document (Medium Priority)

**New File:** [wip/PLAN020_event_driven_playback/DESIGN_DECISIONS.md](DESIGN_DECISIONS.md)

**Decisions to Document:**

1. **Decision: Event-Driven vs. Polling**
   - Problem: 0-100ms random latency
   - Options considered: Pure event-driven, hybrid, polling optimization
   - Choice: Event-driven with watchdog safety net
   - Rationale: <1ms latency + reliability
   - Trade-offs: Added event infrastructure complexity

2. **Decision: Watchdog Detection-Only Pattern**
   - Problem: Watchdog intervention should be rare
   - Options: Disable in tests, hybrid mode, detection-only
   - Choice: Detection-only with WARN logging
   - Rationale: Always-on safety net, test failures indicate bugs
   - Trade-offs: Intervention counter complexity

3. **Decision: Queue Position Detection Before Enqueue**
   - Problem: Determine decode priority for new entry
   - Options: Detect after enqueue, detect before enqueue
   - Choice: Detect before enqueue
   - Rationale: Simpler logic, clearer intent
   - Trade-offs: None (equivalent result)

4. **Decision: Shared start_mixer_for_current() Implementation**
   - Problem: Duplicate mixer startup code in watchdog + event handler
   - Options: Duplicate, extract method, macro
   - Choice: Extract to pub(super) method
   - Rationale: DRY principle, easier maintenance
   - Trade-offs: Slightly longer call chain

5. **Decision: Deferred Test Infrastructure**
   - Problem: TC-ED-004/005 require complex buffer infrastructure
   - Options: Implement now, defer to integration tests, skip
   - Choice: Defer to future integration testing framework
   - Rationale: 4-6 hours effort for functionality already verified in production
   - Trade-offs: Lower test coverage (60% vs. 100%)

6. **Decision: Watchdog UI Visibility (Alternative to Tests)**
   - Problem: Verify event system working correctly
   - Options: Complex test infrastructure, production telemetry
   - Choice: Real-time UI indicator (green/yellow/red)
   - Rationale: 2 hours vs. 9-13 hours, real-world monitoring
   - Trade-offs: Manual verification instead of automated tests

7. **Decision: SSE-Driven Watchdog Updates**
   - Problem: 5-second polling wasteful (83% of requests when count=0)
   - Options: Keep polling, switch to SSE, hybrid
   - Choice: Hybrid (SSE + 30s polling fallback)
   - Rationale: <100ms latency + reliable fallback
   - Trade-offs: Added SSE event infrastructure

**Format:** Decision record template (Problem, Options, Choice, Rationale, Trade-offs)

**Estimated Effort:** 1-2 hours

---

### 7.4 Test Documentation Update (Low Priority)

**File:** [wkmp-ap/tests/README.md](../../wkmp-ap/tests/README.md) (if exists, else create)

**Content:**

1. **Test Suite Overview**
   - PROJ001: Chain assignment tests (7 tests)
   - Event-driven playback tests (5 tests)
   - Test infrastructure (TestEngine, helpers)

2. **Running Tests**
   ```bash
   # All tests
   cargo test -p wkmp-ap

   # PROJ001 chain assignment
   cargo test -p wkmp-ap --test chain_assignment_tests

   # Event-driven playback
   cargo test -p wkmp-ap --test event_driven_playback_tests
   ```

3. **Test Coverage**
   - FR-001 (Event-Driven Decode): TC-ED-001, TC-ED-002, TC-ED-003 ✅
   - FR-002 (Event-Driven Mixer): TC-ED-004, TC-ED-005 ⚠️ (deferred)
   - FR-003 (Watchdog): TC-WD-001/002/003 ⚠️ (deferred)
   - End-to-End: TC-E2E-001, TC-E2E-002 ✅

4. **Deferred Tests**
   - Explanation of why TC-ED-004/005 deferred
   - Reference to DEFERRED_TESTS_ANALYSIS.md
   - Alternative verification (UI visibility)

**Estimated Effort:** 30 minutes

---

### 7.5 Update IMPLEMENTATION_PROGRESS.md (High Priority)

**File:** [wip/PLAN020_event_driven_playback/IMPLEMENTATION_PROGRESS.md](IMPLEMENTATION_PROGRESS.md)

**Changes:**

1. **Update status header**
   - Change from "Phase 5 COMPLETE" to "Phases 1-7 COMPLETE"
   - Update effort completed (17 hours → 21-23 hours)

2. **Add Phase 6-7 section**
   ```markdown
   ### ✅ Phase 6-7: Validation & Documentation (100% Complete)

   **Regression Testing:**
   - ✅ PROJ001 tests: 7/7 passing
   - ✅ Event-driven tests: 5/5 passing
   - ✅ Full wkmp-ap test suite: All passing
   - ✅ Manual smoke testing: No issues observed

   **Documentation Updates:**
   - ✅ SPEC028 updated with event-driven architecture
   - ✅ Event flow diagrams created
   - ✅ Design decisions documented
   - ✅ Test README updated

   **Deliverables:**
   - [SPEC028-playback_orchestration.md](../../docs/SPEC028-playback_orchestration.md) - Updated
   - [EVENT_FLOW_DIAGRAMS.md](EVENT_FLOW_DIAGRAMS.md) - New
   - [DESIGN_DECISIONS.md](DESIGN_DECISIONS.md) - New
   - [PHASE_6_7_DOCUMENTATION_PLAN.md](PHASE_6_7_DOCUMENTATION_PLAN.md) - This file

   **Effort:** 6 hours
   ```

3. **Update pending work section**
   - Remove Phase 6-7 from pending
   - Add note about plan archiving

**Estimated Effort:** 15 minutes

---

## Implementation Order

**Priority sequence for efficient completion:**

1. **Update IMPLEMENTATION_PROGRESS.md** (15 min) - Mark Phase 6-7 in progress
2. **Run regression tests** (30 min) - Verify all tests passing
3. **Update SPEC028** (3-4 hours) - Critical documentation artifact
4. **Create EVENT_FLOW_DIAGRAMS** (1-2 hours) - Visual reference
5. **Create DESIGN_DECISIONS** (1-2 hours) - Capture rationale
6. **Update test README** (30 min) - Developer reference
7. **Final IMPLEMENTATION_PROGRESS update** (15 min) - Mark complete

**Total Estimated Effort:** 6-8 hours

---

## Success Criteria

**Phase 6-7 is complete when:**

- ✅ All regression tests passing (PROJ001 + event-driven tests)
- ✅ SPEC028 updated with event-driven architecture
- ✅ Event flow diagrams created (Mermaid sequence diagrams)
- ✅ Design decisions documented (7 key decisions)
- ✅ Test documentation updated (README with coverage matrix)
- ✅ IMPLEMENTATION_PROGRESS.md reflects Phase 6-7 completion

**Ready for plan archiving when:**
- All Phase 6-7 success criteria met
- No pending TODOs in implementation
- All deliverables committed to git
- Change history updated via `/commit`

---

## References

**Plan Documents:**
- [00_PLAN_SUMMARY.md](00_PLAN_SUMMARY.md) - Plan overview
- [IMPLEMENTATION_PROGRESS.md](IMPLEMENTATION_PROGRESS.md) - Current status
- [DEFERRED_TESTS_ANALYSIS.md](DEFERRED_TESTS_ANALYSIS.md) - Deferred test rationale
- [WATCHDOG_VISIBILITY_FEATURE.md](WATCHDOG_VISIBILITY_FEATURE.md) - UI monitoring
- [WATCHDOG_SSE_ENHANCEMENT.md](WATCHDOG_SSE_ENHANCEMENT.md) - Real-time updates

**Specification:**
- [SPEC_event_driven_playback_refactor.md](../SPEC_event_driven_playback_refactor.md) - Original design
- [SPEC028-playback_orchestration.md](../../docs/SPEC028-playback_orchestration.md) - To be updated

---

## Next Steps

1. Mark Phase 6-7 as in_progress in IMPLEMENTATION_PROGRESS.md
2. Run full regression test suite
3. Begin SPEC028 updates (highest priority)
4. Create supporting documentation (diagrams, decisions)
5. Final validation and completion
