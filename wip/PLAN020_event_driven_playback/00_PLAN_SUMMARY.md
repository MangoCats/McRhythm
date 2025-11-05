# PLAN020: Event-Driven Playback Orchestration - PLAN SUMMARY

**Status:** ✅ Phases 1-3 Complete - Ready for Implementation
**Created:** 2025-11-04
**Specification Source:** `wip/SPEC_event_driven_playback_refactor.md`
**Plan Location:** `wip/PLAN020_event_driven_playback/`

---

## READ THIS FIRST

This plan transforms WKMP's playback orchestration from polling-based to fully event-driven architecture while retaining a 100ms watchdog as safety mechanism.

**Current Status:** Phases 1-3 Complete
- ✅ Phase 1: Scope Definition
- ✅ Phase 2: Requirements Extraction and Specification Verification (0 critical issues)
- ✅ Phase 3: Acceptance Test Definition (11 tests, 100% requirement coverage)

**For Implementation:**
- Read this summary (~500 lines) for overview
- Read [requirements_index.md](requirements_index.md) for detailed requirements
- Read [02_test_specifications/](02_test_specifications/) for test cases
- Read [traceability_matrix.md](traceability_matrix.md) for requirements ↔ tests mapping

**Context Window Budget:** ~650-800 lines per increment during implementation

---

## Executive Summary

### Problem Being Solved

Current playback orchestration uses 100ms polling (`process_queue()`) for operations that should be event-driven:
- **Decode initiation:** Polled every 100ms instead of triggered on enqueue
- **Mixer startup:** Polled for buffer threshold instead of event-driven
- **Queue management:** Polled for state changes instead of event-driven

**Impact:**
- 0-100ms random latency for all operations
- Unnecessary CPU usage (polling when idle)
- Architectural inconsistency (crossfade/position already event-driven via markers)
- Technical debt from incomplete SUB-INC-4B migration

### Solution Approach

**Event-Driven Architecture with Watchdog Safety Net:**

1. **Eliminate Polling:**
   - Decode triggered immediately on `EnqueueEvent` and `QueueAdvanceEvent`
   - Mixer starts on `BufferThresholdReached` event
   - All operations <1ms latency (vs. current 0-100ms)

2. **Retain 100ms Loop as Watchdog:**
   - Detects "stuck" states (event system failures)
   - Logs **WARN** on intervention (indicates event bug)
   - Restores proper state (safety mechanism)
   - Refactor `process_queue()` → `watchdog_check()` (~70% code reduction)

3. **Test Strategy:**
   - Watchdog intervention in test = test failure
   - Event-driven paths fully unit tested
   - End-to-end event flow integration tests

4. **Configurable Parameters (Database Settings):**
   - `watchdog_interval_ms`: 100ms default (range: 10-2000ms)
   - `minimum_playback_buffer_ms`: 3000ms default (range: 100-12000ms)
   - Telemetry: `watchdog_interventions_total` counter metric

### Implementation Status

**Phase 1 Complete:**
- ✅ Specification analyzed (1180 lines)
- ✅ Scope identified (8 requirements: 4 functional, 4 non-functional)
- ✅ High-level approach confirmed (event-driven + watchdog)

**Phase 2 Complete:**
- ✅ All requirements extracted and documented ([requirements_index.md](requirements_index.md))
- ✅ Specification completeness verified ([01_specification_issues.md](01_specification_issues.md))
- ✅ **Result:** 0 critical issues, 0 high-priority issues, 2 minor clarifications (non-blocking)
- ✅ Specification quality: EXCELLENT - Production-Ready

**Phase 3 Complete:**
- ✅ 11 acceptance tests defined in BDD format ([02_test_specifications/](02_test_specifications/))
- ✅ Test categories: Event-Driven Decode (3), Event-Driven Mixer (2), Watchdog Detection (3), End-to-End (2), Isolation (1)
- ✅ Traceability matrix created ([traceability_matrix.md](traceability_matrix.md))
- ✅ **Coverage:** 100% of requirements have tests

---

## Requirements Summary

**Total Requirements:** 8 identified (4 functional, 4 non-functional)

### Functional Requirements

**FR-001: Event-Driven Decode Initiation**
- Decode requests triggered immediately by queue state changes
- Events: `EnqueueEvent`, `QueueAdvanceEvent`
- Latency: <1ms (vs. current 0-100ms)

**FR-002: Event-Driven Mixer Startup**
- Mixer starts when buffer reaches threshold via event
- Event: `BufferThresholdReached(queue_entry_id, threshold_ms)`
- Latency: <1ms (vs. current 0-100ms)

**FR-003: Watchdog Loop (100ms)**
- Safety mechanism detecting stuck states
- Logs WARN on intervention
- Detects: missing buffers, mixer not started, prefetch failures
- Configurable interval (database setting)

**FR-004: Event System Test Coverage**
- Tests verify event-driven behavior without watchdog
- Watchdog intervention = test failure
- Mock event system for failure testing

### Non-Functional Requirements

**NFR-001: Responsiveness**
- Event-driven operations: <1ms latency
- Watchdog check interval: 100ms (configurable)

**NFR-002: CPU Efficiency**
- Watchdog loop: Minimal checks (5-10 condition evaluations)
- No buffer reads, no database queries in watchdog

**NFR-003: Testability**
- Event triggers testable in isolation
- Watchdog testable separately
- Test failures distinguish event bugs from watchdog bugs

**NFR-004: Backward Compatibility**
- External behavior identical
- API unchanged
- Database schema unchanged
- Configuration unchanged (new settings additive)

**Full Requirements:** See [requirements_index.md](requirements_index.md) for complete details

---

## Scope

### ✅ In Scope

**Core Refactoring:**
- Rename `process_queue()` → `watchdog_check()` (reactive only, ~70% reduction)
- Add event system infrastructure (`EnqueueEvent`, `QueueAdvanceEvent`, `BufferThresholdReached`)
- Event-driven decode initiation (on enqueue, on queue advance)
- Event-driven mixer startup (buffer threshold detection in `BufferManager.push_samples()`)
- Extract `start_mixer_for_current()` method (called by event handler OR watchdog)

**Configuration:**
- Add `watchdog_interval_ms` to database settings table (100ms default, 10-2000ms range)
- Add `minimum_playback_buffer_ms` to database settings table (3000ms default, 100-12000ms range)
- Hot-reload support for both settings

**Telemetry:**
- Add `watchdog_interventions_total` counter metric
- Labels: `intervention_type`, `queue_entry_id`
- Alerting: >0 interventions = event system bug

**Testing:**
- Event-driven unit tests (verify each event trigger)
- Event-driven integration tests (verify end-to-end event flows)
- Watchdog unit tests (verify detection logic)
- Test mode: watchdog intervention = panic (test failure)

**Documentation:**
- Update SPEC028 with event-driven architecture
- Document watchdog purpose and logging format
- Test suite README
- Architecture diagrams

### ❌ Out of Scope

**Explicitly NOT Included:**
- Changes to marker-based crossfade/position system (already event-driven)
- Changes to database schema (settings table already exists)
- Changes to external APIs (HTTP/SSE unchanged)
- Performance optimization beyond event-driven conversion
- New features (only refactoring existing functionality)

**Full Scope:** All scope details integrated into this summary and requirements_index.md

---

## Specification Assessment

**Specification Quality:** ✅ **EXCELLENT - Production-Ready**

**Strengths:**
- Comprehensive current state analysis (Section 2)
- Clear requirements with quantified criteria (Section 3)
- Detailed design with code examples (Section 4, 400+ lines)
- Complete test strategy with test cases (Section 5, 150+ lines)
- Phased implementation plan (Section 6)
- Risk assessment with mitigations (Section 8)
- All open questions resolved with implementation details (Section 10)

**Phase 2 Formal Analysis Complete:**
- ✅ **0 critical issues** identified
- ✅ **0 high-priority issues** identified
- ✅ **2 minor clarifications** recommended (non-blocking)
- ✅ Specification completeness: 98% (only minor clarifications missing)
- ✅ Testability: 100% (all requirements verifiable)
- ✅ **Status:** APPROVED for implementation

**Details:** See [01_specification_issues.md](01_specification_issues.md)

---

## Implementation Roadmap

**Based on specification Section 6, 7 phases identified:**

### Phase 1: Event Infrastructure
**Objective:** Add event types and routing
**Estimated Effort:** 4-6 hours
**Key Deliverables:**
- Add `PlaybackEvent` variants (Enqueued, QueueAdvanced, BufferThresholdReached)
- Implement event channel in BufferManager
- Add event handlers in PlaybackEngine
**Tests:** Event emission and routing tests

### Phase 2: Event-Driven Decode
**Objective:** Replace polling with events for decode initiation
**Estimated Effort:** 6-8 hours
**Key Deliverables:**
- Refactor `enqueue_file()` to trigger decode immediately
- Refactor queue advance to trigger decode for promoted passages
- Extract event handlers from `process_queue()`
**Tests:** TC-ED-001, TC-ED-002, TC-ED-003 (decode triggering tests)

### Phase 3: Event-Driven Mixer Startup
**Objective:** Replace polling with events for mixer startup
**Estimated Effort:** 6-8 hours
**Key Deliverables:**
- Add threshold detection to `BufferManager.push_samples()`
- Extract `start_mixer_for_current()` method
- Implement buffer threshold event handler
**Tests:** TC-ED-004, TC-ED-005 (mixer startup tests)

### Phase 4: Watchdog Refactoring
**Objective:** Convert `process_queue()` from proactive to reactive
**Estimated Effort:** 4-6 hours
**Key Deliverables:**
- Rename `process_queue()` → `watchdog_check()`
- Remove proactive operations (keep detection only)
- Add WARN logging for interventions
**Tests:** TC-WD-001, TC-WD-002, TC-WD-003 (watchdog detection tests)

### Phase 5: Test Infrastructure
**Objective:** Support test-mode failure on watchdog intervention
**Estimated Effort:** 3-4 hours
**Key Deliverables:**
- Add `watchdog_enabled` config flag
- Implement test-mode panic on intervention
- Create DecoderWorkerSpy and BufferManagerMock
**Tests:** TC-WD-DISABLED-001 (event system works without watchdog)

### Phase 6: Integration and Validation
**Objective:** End-to-end testing with event-driven system
**Estimated Effort:** 4-6 hours
**Key Deliverables:**
- End-to-end testing with real components
- Performance validation (latency measurements)
- Regression testing (existing test suite)
**Tests:** TC-E2E-001, TC-E2E-002 (complete playback flow tests)

### Phase 7: Documentation
**Objective:** Update specifications and guides
**Estimated Effort:** 3-4 hours
**Key Deliverables:**
- Update SPEC028 with event-driven architecture
- Document watchdog purpose and logging
- Update test suite README
- Add architecture diagrams

**Total Estimated Effort:** 30-42 hours over 2-3 weeks

---

## Test Coverage Summary

**Test Strategy:** Comprehensive BDD specifications for all requirements

**Test Categories and Counts:**

| Category | Tests | Priority | Files |
|----------|-------|----------|-------|
| Event-Driven Decode | 3 | P0 | TC-ED-001, TC-ED-002, TC-ED-003 |
| Event-Driven Mixer | 2 | P0 | TC-ED-004, TC-ED-005 |
| Watchdog Detection | 3 | P0 | TC-WD-001, TC-WD-002, TC-WD-003 |
| End-to-End Integration | 2 | P0 | TC-E2E-001, TC-E2E-002 |
| Isolation Testing | 1 | P1 | TC-WD-DISABLED-001 |
| **Total** | **11** | - | - |

**Test Infrastructure Required:**
- DecoderWorkerSpy (track decode requests with timestamps)
- BufferManagerMock (simulate buffer fill and threshold events)
- Test mode configuration (panic on watchdog intervention)

**Test Execution:**
- Target time: <3s total (CI-ready)
- All tests use BDD format (Given/When/Then)
- Test-first implementation approach (write tests before code)

**Traceability:** ✅ Complete - See [traceability_matrix.md](traceability_matrix.md)
- 100% requirement coverage (all 8 requirements have tests)
- FR-001: 5 tests, FR-002: 4 tests, FR-003: 3 tests, FR-004: all 11 tests
- NFR-001: 3 tests, NFR-003: 1 test, NFR-004: regression suite

**Test Specifications:** See [02_test_specifications/](02_test_specifications/) for BDD details

---

## Risk Assessment

**Based on specification Section 8:**

### Risk 1: Event System Complexity
- **Residual Risk:** Low (watchdog catches all failure modes)
- **Mitigation:** Comprehensive unit tests, watchdog safety net, test mode fails fast

### Risk 2: Watchdog False Positives
- **Residual Risk:** Low (can tune watchdog checks based on production data)
- **Mitigation:** Careful state checking, thorough testing, production log monitoring

### Risk 3: Performance Regression
- **Residual Risk:** Very Low (event-driven should be faster than polling)
- **Mitigation:** Performance benchmarking before/after, lightweight events

### Risk 4: Test Suite Complexity
- **Residual Risk:** Low-Medium (typical for event-driven testing)
- **Mitigation:** Reasonable timing thresholds, test against real components, watchdog disabled tests

**Full Risk Analysis:** See specification Section 8

---

## Dependencies

**Existing Code (Read-Only Reference):**
- [SPEC028: Playback Loop Orchestration](../docs/SPEC028-playback_orchestration.md) - Current implementation (~200 lines relevant)
- [core.rs:1292-1680](../wkmp-ap/src/playback/engine/core.rs) - `playback_loop()` and `process_queue()` (~400 lines)
- [queue.rs](../wkmp-ap/src/playback/engine/queue.rs) - Queue management (focus on `enqueue_file()`)
- [buffer_manager.rs](../wkmp-ap/src/playback/buffer_manager.rs) - Buffer lifecycle (focus on `push_samples()`)
- [decoder_worker.rs](../wkmp-ap/src/playback/decoder_worker.rs) - Decoder priority selection

**Integration Points (Will Be Modified):**
- `wkmp-ap/src/playback/engine/core.rs` - Watchdog refactoring, event handlers
- `wkmp-ap/src/playback/engine/queue.rs` - Event-driven enqueue
- `wkmp-ap/src/playback/buffer_manager.rs` - Buffer threshold detection
- `wkmp-ap/src/playback/queue_manager.rs` - Queue advance event

**No External Dependencies** - All changes within wkmp-ap

---

## Constraints

**Technical:**
- Must maintain backward compatibility (external behavior unchanged)
- Must work with existing microservices architecture (HTTP APIs, SSE events)
- Must follow Rust async patterns (tokio, async/await)
- Must integrate with existing event system (wkmp_common::events)

**Process:**
- Test-first implementation (write tests before code)
- All tests must pass before marking increment complete
- Watchdog intervention in test = test failure (no exceptions)

**Timeline:**
- Estimated: 30-42 hours over 2-3 weeks
- Checkpoint every 5-10 increments (verify tests pass, review progress)

---

## Success Metrics

**Quantitative:**
- ✅ Enqueue → Decode start: <1ms (vs. current 0-100ms)
- ✅ Buffer ready → Mixer start: <1ms (vs. current 0-100ms)
- ✅ CPU usage reduced (no unnecessary polling)
- ✅ Code complexity reduced (~250 lines eliminated from process_queue)
- ✅ Test suite passes (100% of acceptance tests)
- ✅ No watchdog interventions in test suite (0 expected)

**Qualitative:**
- ✅ Architecture consistency (fully event-driven)
- ✅ Improved responsiveness (instant operations)
- ✅ Better testability (event paths testable in isolation)
- ✅ Clear safety mechanism (watchdog provides diagnostics)

---

## Next Steps

### Immediate: Ready for Implementation ✅

**Planning Complete (Phases 1-3):**
- ✅ Phase 1: Scope Definition complete
- ✅ Phase 2: Specification Verification complete (0 critical issues)
- ✅ Phase 3: Acceptance Test Definition complete (11 tests, 100% coverage)

**Implementation Ready:**
All planning artifacts created and ready for use:
- [requirements_index.md](requirements_index.md) - All 8 requirements detailed
- [01_specification_issues.md](01_specification_issues.md) - Verification results (EXCELLENT rating)
- [02_test_specifications/](02_test_specifications/) - 11 BDD test specifications
- [traceability_matrix.md](traceability_matrix.md) - Requirements ↔ Tests mapping (100% coverage)

### Implementation Sequence

1. Implement Phase 1 (Event Infrastructure)
2. Implement Phase 2 (Event-Driven Decode)
3. Implement Phase 3 (Event-Driven Mixer Startup)
4. Implement Phase 4 (Watchdog Refactoring)
5. Implement Phase 5 (Test Infrastructure)
6. Implement Phase 6 (Integration and Validation)
7. Implement Phase 7 (Documentation)

### After Implementation

1. Execute Phase 9: Post-Implementation Review (MANDATORY)
2. Generate technical debt report
3. Run all acceptance tests
4. Verify traceability matrix 100% complete
5. Create final implementation report
6. Archive plan using `/archive-plan PLAN020`

---

## Document Navigation

**Start Here:** This file (00_PLAN_SUMMARY.md)

**Detailed Planning (Created):**
- [requirements_index.md](requirements_index.md) - All 8 requirements with priorities, success criteria
- [01_specification_issues.md](01_specification_issues.md) - Phase 2 verification (EXCELLENT - 0 critical issues)
- [02_test_specifications/](02_test_specifications/) - Modular test specifications folder
  - [test_index.md](02_test_specifications/test_index.md) - All 11 tests quick reference
  - [tc_ed_001_decode_on_enqueue.md](02_test_specifications/tc_ed_001_decode_on_enqueue.md) - Detailed BDD spec
  - [tc_ed_002_decode_on_advance.md](02_test_specifications/tc_ed_002_decode_on_advance.md) - Detailed BDD spec
  - [tc_ed_003_to_tc_e2e_002.md](02_test_specifications/tc_ed_003_to_tc_e2e_002.md) - 9 remaining tests (consolidated)
- [traceability_matrix.md](traceability_matrix.md) - Requirements ↔ Tests mapping (100% coverage)

**For Implementation:**
- Read this summary (~400 lines) - overview
- Read current increment specification (~250 lines) - details
- Read relevant test specs (~100-150 lines) - acceptance criteria
- **Total context:** ~650-800 lines per increment

---

## Plan Status

**Phase 1 Status:** ✅ Complete (Scope Definition)
**Phase 2 Status:** ✅ Complete (Requirements Extraction, Specification Verification - 0 critical issues)
**Phase 3 Status:** ✅ Complete (Acceptance Test Definition - 11 tests, 100% coverage)
**Phases 4-8 Status:** N/A (implementation phases, not planning)

**Current Status:** ✅ Planning Complete - Ready for Implementation

**Specification Quality:** ✅ EXCELLENT (0 critical issues, 98% completeness, 100% testability)

---

## Implementation Instructions

**Planning is complete.** To begin implementation:

1. **Review Plan Artifacts:**
   - Read [00_PLAN_SUMMARY.md](00_PLAN_SUMMARY.md) (this file) - Overview
   - Read [requirements_index.md](requirements_index.md) - Detailed requirements
   - Read [02_test_specifications/](02_test_specifications/) - Test cases (BDD format)
   - Read [traceability_matrix.md](traceability_matrix.md) - Coverage verification

2. **Follow Test-First Approach:**
   - Implement tests before code (per traceability_matrix.md implementation order)
   - Start with Phase 1: Test Infrastructure (DecoderWorkerSpy, BufferManagerMock)
   - Continue with Phases 2-7 per Implementation Roadmap section

3. **Reference Specification:**
   - Design details: [SPEC_event_driven_playback_refactor.md](../../SPEC_event_driven_playback_refactor.md)
   - Code examples in §4 (400+ lines of implementation guidance)

**Plan Status:** ✅ Complete - No further planning needed

---

## Approval and Sign-Off

**Plan Created:** 2025-11-04
**Planning Completed:** 2025-11-04
**Status:** ✅ All Planning Phases Complete (Phases 1-3)

**Deliverables:**
- ✅ Requirements Index (8 requirements)
- ✅ Specification Verification (0 critical issues)
- ✅ Test Specifications (11 tests, BDD format)
- ✅ Traceability Matrix (100% coverage)

**Specification Source:** [wip/SPEC_event_driven_playback_refactor.md](../SPEC_event_driven_playback_refactor.md) (1180 lines)
**Specification Quality:** ✅ EXCELLENT (98% completeness, 100% testability)

**Ready for Implementation:** ✅ Yes - All planning artifacts complete
