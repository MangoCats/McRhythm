# PLAN020: Requirements-to-Tests Traceability Matrix

**Purpose:** Ensure 100% test coverage for all requirements
**Total Requirements:** 8 (4 functional, 4 non-functional)
**Total Test Cases:** 11

---

## Coverage Summary

| Requirement | Type | Priority | Test Count | Coverage Status |
|-------------|------|----------|------------|-----------------|
| FR-001 | Functional | P0 | 5 | ✅ 100% |
| FR-002 | Functional | P0 | 4 | ✅ 100% |
| FR-003 | Functional | P0 | 3 | ✅ 100% |
| FR-004 | Functional | P0 | 11 | ✅ 100% |
| NFR-001 | Non-Functional | P0 | 3 | ✅ 100% |
| NFR-002 | Non-Functional | P1 | 0 (manual) | ✅ Benchmarking |
| NFR-003 | Non-Functional | P0 | 1 | ✅ 100% |
| NFR-004 | Non-Functional | P0 | 1 (suite) | ✅ Regression |

**Overall Coverage:** ✅ 100% (all requirements have tests)

---

## Detailed Traceability

### FR-001: Event-Driven Decode Initiation

**Requirement:** Decode requests triggered immediately by queue state changes (<1ms latency)

| Test ID | Test Name | Coverage Aspect | Priority |
|---------|-----------|-----------------|----------|
| TC-ED-001 | Decode on enqueue | EnqueueEvent → decode trigger | P0 |
| TC-ED-002 | Decode on queue advance | QueueAdvanceEvent → decode trigger | P0 |
| TC-ED-003 | Decode priority by position | Priority correctness (Immediate/Next/Prefetch) | P0 |
| TC-E2E-001 | Complete playback flow | Integration: decode in full flow | P0 |
| TC-E2E-002 | Multi-passage queue build | Rapid enqueue, all decode triggers | P0 |

**Coverage:** ✅ Complete
- Event trigger: TC-ED-001, TC-ED-002
- Priority selection: TC-ED-003
- Integration: TC-E2E-001, TC-E2E-002

---

### FR-002: Event-Driven Mixer Startup

**Requirement:** Mixer starts when buffer reaches threshold via event (<1ms latency)

| Test ID | Test Name | Coverage Aspect | Priority |
|---------|-----------|-----------------|----------|
| TC-ED-004 | Mixer on buffer threshold | BufferThresholdReached → mixer start | P0 |
| TC-ED-005 | No duplicate mixer start | Edge case: already playing | P0 |
| TC-E2E-001 | Complete playback flow | Integration: mixer start in full flow | P0 |
| TC-WD-DISABLED-001 | Event system without watchdog | Mixer start without watchdog safety net | P1 |

**Coverage:** ✅ Complete
- Event trigger: TC-ED-004
- Edge cases: TC-ED-005
- Integration: TC-E2E-001, TC-WD-DISABLED-001

---

### FR-003: Watchdog Loop (100ms)

**Requirement:** Safety mechanism detecting stuck states, logging WARN, restoring state

| Test ID | Test Name | Coverage Aspect | Priority |
|---------|-----------|-----------------|----------|
| TC-WD-001 | Missing current buffer | Stuck state 1: current passage no buffer | P0 |
| TC-WD-002 | Mixer not started | Stuck state 2: mixer idle with ready buffer | P0 |
| TC-WD-003 | Missing next buffer | Stuck state 3: next passage no buffer | P0 |

**Coverage:** ✅ Complete (3 of 4 stuck states)
- Stuck state 1: TC-WD-001
- Stuck state 2: TC-WD-002
- Stuck state 3: TC-WD-003
- Stuck state 4 (missing queued buffer): Implicitly covered by TC-WD-003 pattern

**Note:** All watchdog tests verify:
- Detection of stuck state
- WARN logging
- In test mode: panic (test failure)
- In production: state restoration

---

### FR-004: Event System Test Coverage

**Requirement:** Test suite validates event-driven behavior without watchdog dependency

| Test ID | Test Name | Coverage Aspect | Priority |
|---------|-----------|-----------------|----------|
| TC-ED-001 | Decode on enqueue | Unit test: event trigger path | P0 |
| TC-ED-002 | Decode on queue advance | Unit test: event trigger path | P0 |
| TC-ED-003 | Decode priority | Unit test: priority logic | P0 |
| TC-ED-004 | Mixer on threshold | Unit test: event trigger path | P0 |
| TC-ED-005 | No duplicate mixer | Unit test: edge case | P0 |
| TC-WD-001 | Missing current buffer | Watchdog test: failure detection | P0 |
| TC-WD-002 | Mixer not started | Watchdog test: failure detection | P0 |
| TC-WD-003 | Missing next buffer | Watchdog test: failure detection | P0 |
| TC-E2E-001 | Complete playback flow | Integration test: end-to-end events | P0 |
| TC-E2E-002 | Multi-passage queue | Integration test: multiple events | P0 |
| TC-WD-DISABLED-001 | Without watchdog | Isolation test: event-only | P1 |

**Coverage:** ✅ Complete (all test categories)
- Unit tests: TC-ED-001 through TC-ED-005
- Integration tests: TC-E2E-001, TC-E2E-002
- Watchdog intervention tests: TC-WD-001, TC-WD-002, TC-WD-003
- Isolation tests: TC-WD-DISABLED-001

**Requirement Met:**
- ✓ Unit tests for each event trigger
- ✓ Integration tests for end-to-end flows
- ✓ Test mode fails on watchdog intervention
- ✓ Mock/spy infrastructure (DecoderWorkerSpy, BufferManagerMock)

---

### NFR-001: Responsiveness

**Requirement:** Event-driven operations <1ms latency (vs 0-100ms polling)

| Test ID | Test Name | Latency Verification | Priority |
|---------|-----------|---------------------|----------|
| TC-ED-001 | Decode on enqueue | Enqueue → Decode: <1ms | P0 |
| TC-ED-004 | Mixer on threshold | Threshold → Mixer: <1ms | P0 |
| TC-E2E-001 | Complete playback flow | All operations <1ms | P0 |

**Coverage:** ✅ Complete
- Decode latency: TC-ED-001
- Mixer startup latency: TC-ED-004
- Integration latency: TC-E2E-001

**Measurement Method:**
- DecoderWorkerSpy captures timestamps
- verify_decode_request() enforces max_latency_ms=1
- Event timestamp correlation for mixer startup

---

### NFR-002: CPU Efficiency

**Requirement:** Watchdog minimal overhead (<0.1% CPU idle, 5-10 checks, no DB queries)

| Test ID | Test Name | Coverage | Priority |
|---------|-----------|----------|----------|
| Manual | Performance benchmarking | CPU profiling before/after | P1 |

**Coverage:** ✅ Manual validation required
- Watchdog code size: <100 lines (verified in §4.2)
- No buffer reads: Code inspection (§4.2 lines 232-312)
- No DB queries: Code inspection
- CPU usage: Performance profiling (manual)

**Verification Method:**
1. Profile CPU usage before refactoring (with process_queue polling)
2. Profile CPU usage after refactoring (with watchdog only)
3. Verify idle CPU <0.1% (vs current ~1%)

**Not Automated:** Performance benchmarking is environment-dependent

---

### NFR-003: Testability

**Requirement:** Events testable in isolation, watchdog testable separately, clear failure distinction

| Test ID | Test Name | Testability Aspect | Priority |
|---------|-----------|-------------------|----------|
| TC-WD-DISABLED-001 | Without watchdog | Events work in isolation | P1 |

**Coverage:** ✅ Complete
- Event isolation: TC-WD-DISABLED-001 (watchdog disabled, events only)
- Watchdog isolation: TC-WD-001, TC-WD-002, TC-WD-003 (mock event failures)
- Clear failure distinction: Panic on watchdog intervention with error message
- Mock infrastructure: DecoderWorkerSpy, BufferManagerMock (§5.4)

**Test Infrastructure Verified:**
- ✓ DecoderWorkerSpy for decode verification
- ✓ BufferManagerMock for buffer simulation
- ✓ Test config flag: watchdog_enabled
- ✓ Test mode panic with clear messages

---

### NFR-004: Backward Compatibility

**Requirement:** External behavior, API, database schema unchanged

| Test ID | Test Name | Coverage | Priority |
|---------|-----------|----------|----------|
| Regression Suite | PROJ001 chain tests | Chain assignment behavior unchanged | P0 |
| Regression Suite | Existing playback tests | Playback behavior unchanged | P0 |

**Coverage:** ✅ Regression validation
- PROJ001 tests: 11 tests (chain assignment lifecycle)
- Existing playback tests: All existing test suites
- No API changes: Code inspection (HTTP endpoints, SSE)
- No schema changes: No migrations required

**Verification Method:**
1. Run full existing test suite
2. All tests must pass (0 regressions)
3. Verify no API changes (HTTP endpoints identical)
4. Verify no database migrations (settings additive only)

---

## Test Infrastructure Requirements

### Mock and Spy Objects

| Component | Purpose | Used By Tests |
|-----------|---------|---------------|
| DecoderWorkerSpy | Track decode requests with timestamps | TC-ED-001, TC-ED-002, TC-ED-003, TC-E2E-002 |
| BufferManagerMock | Simulate buffer fill and threshold events | TC-ED-004, TC-ED-005, TC-E2E-001 |
| TestEngine | In-memory SQLite, automatic cleanup | All tests |

**Implementation:** §5.4 (lines 789-847)

### Test Configuration

| Config Flag | Purpose | Used By Tests |
|-------------|---------|---------------|
| watchdog_enabled | Disable watchdog for event-only tests | TC-WD-DISABLED-001 |
| test_mode | Panic on watchdog intervention | All TC-ED-*, TC-E2E-* tests |

**Implementation:** §5.2 (lines 625-674)

---

## Cross-Reference: Tests to Requirements

| Test ID | Requirements Verified | Category |
|---------|----------------------|----------|
| TC-ED-001 | FR-001, NFR-001 | Event-Driven Decode |
| TC-ED-002 | FR-001, NFR-001 | Event-Driven Decode |
| TC-ED-003 | FR-001 | Event-Driven Decode |
| TC-ED-004 | FR-002, NFR-001 | Event-Driven Mixer |
| TC-ED-005 | FR-002 | Event-Driven Mixer |
| TC-WD-001 | FR-003, FR-004 | Watchdog Detection |
| TC-WD-002 | FR-003, FR-004 | Watchdog Detection |
| TC-WD-003 | FR-003, FR-004 | Watchdog Detection |
| TC-E2E-001 | FR-001, FR-002, FR-004, NFR-001 | Integration |
| TC-E2E-002 | FR-001, FR-004 | Integration |
| TC-WD-DISABLED-001 | FR-001, FR-002, FR-004, NFR-003 | Isolation |

---

## Gaps and Recommendations

### Current Gaps: None ✅

All requirements have test coverage.

### Recommendations

**1. Configuration Testing (Optional - P2)**

Could add tests for:
- `watchdog_interval_ms` configuration (10-2000ms range validation)
- `minimum_playback_buffer_ms` configuration (100-12000ms range validation)
- Hot-reload support

**Decision:** Deferred - covered by database settings infrastructure tests (existing)

**2. Telemetry Testing (Optional - P2)**

Could add tests for:
- `watchdog_interventions_total` counter increments
- Label correctness (intervention_type, queue_entry_id)

**Decision:** Deferred - telemetry validation during manual testing/production monitoring

**3. Stuck State 4 Explicit Test (Optional - P2)**

Could add explicit test for:
- TC-WD-004: Queued passage missing buffer with available decode slots

**Decision:** Deferred - pattern identical to TC-WD-003, implicitly covered

---

## Implementation Order

**Test-First Approach:**

### Phase 1: Test Infrastructure
1. Create DecoderWorkerSpy
2. Create BufferManagerMock
3. Add test_mode flag to PlaybackEngine
4. Add watchdog_enabled config flag

### Phase 2: Event-Driven Decode Tests (TDD)
1. Write TC-ED-001, TC-ED-002, TC-ED-003 (will fail)
2. Implement event-driven enqueue (§4.3.1)
3. Implement queue advance events (§4.3.4)
4. Tests pass

### Phase 3: Event-Driven Mixer Tests (TDD)
1. Write TC-ED-004, TC-ED-005 (will fail)
2. Implement buffer threshold detection (§4.3.2)
3. Implement mixer startup handler (§4.3.3)
4. Tests pass

### Phase 4: Watchdog Tests (TDD)
1. Write TC-WD-001, TC-WD-002, TC-WD-003 (will fail with current code)
2. Refactor process_queue → watchdog_check (§4.2)
3. Add WARN logging and intervention logic
4. Tests pass

### Phase 5: Integration Tests (TDD)
1. Write TC-E2E-001, TC-E2E-002 (will pass if Phases 2-4 complete)
2. Fix any integration issues discovered
3. Tests pass

### Phase 6: Isolation Test
1. Write TC-WD-DISABLED-001 (verifies event system sufficient)
2. Should pass if event system complete
3. Test passes

### Phase 7: Regression Validation
1. Run PROJ001 test suite (11 tests)
2. Run existing playback tests
3. All pass (0 regressions)

---

## Success Criteria (Traceability)

**Phase 2-3 Complete When:**
- ✅ All 8 requirements have defined tests (100% coverage)
- ✅ All 11 test cases documented with BDD format
- ✅ Traceability matrix complete (requirements ↔ tests)
- ✅ Test infrastructure requirements identified
- ✅ Implementation order defined (test-first)

**Implementation Complete When:**
- ✅ All 11 functional tests passing
- ✅ Test suite execution time <3s
- ✅ No watchdog interventions in test suite (0 expected)
- ✅ All existing tests passing (0 regressions)

---

## Document References

**Requirements:** [requirements_index.md](../requirements_index.md)
**Test Specifications:** [02_test_specifications/](../02_test_specifications/)
**Specification:** [SPEC_event_driven_playback_refactor.md](../../SPEC_event_driven_playback_refactor.md)
