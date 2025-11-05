# PLAN020: Test Specifications Index

**Total Tests:** 11 functional tests
**Source:** [SPEC_event_driven_playback_refactor.md](../../SPEC_event_driven_playback_refactor.md) §5.3

---

## Test Categories

| Category | Count | Priority | Execution Time Target |
|----------|-------|----------|----------------------|
| Event-Driven Decode | 3 | P0 | <500ms |
| Event-Driven Mixer | 2 | P0 | <500ms |
| Watchdog Detection | 3 | P0 | <500ms |
| End-to-End Flows | 2 | P0 | <1000ms |
| Watchdog Disabled | 1 | P1 | <500ms |
| **Total** | **11** | - | **<3000ms** |

---

## Test Quick Reference

### Event-Driven Decode Tests (TC-ED-001 to TC-ED-003)

| Test ID | Description | Requirements | Specification File |
|---------|-------------|--------------|-------------------|
| TC-ED-001 | Decode triggered on enqueue | FR-001, NFR-001 | [tc_ed_001_decode_on_enqueue.md](tc_ed_001_decode_on_enqueue.md) |
| TC-ED-002 | Decode triggered on queue advance | FR-001, NFR-001 | [tc_ed_002_decode_on_advance.md](tc_ed_002_decode_on_advance.md) |
| TC-ED-003 | Decode priority correct by position | FR-001 | [tc_ed_003_decode_priority.md](tc_ed_003_decode_priority.md) |

### Event-Driven Mixer Tests (TC-ED-004 to TC-ED-005)

| Test ID | Description | Requirements | Specification File |
|---------|-------------|--------------|-------------------|
| TC-ED-004 | Mixer starts on buffer threshold | FR-002, NFR-001 | [tc_ed_004_mixer_on_threshold.md](tc_ed_004_mixer_on_threshold.md) |
| TC-ED-005 | Mixer already playing - no duplicate start | FR-002 | [tc_ed_005_mixer_no_duplicate.md](tc_ed_005_mixer_no_duplicate.md) |

### Watchdog Detection Tests (TC-WD-001 to TC-WD-003)

| Test ID | Description | Requirements | Specification File |
|---------|-------------|--------------|-------------------|
| TC-WD-001 | Watchdog detects missing current buffer | FR-003, FR-004 | [tc_wd_001_missing_current_buffer.md](tc_wd_001_missing_current_buffer.md) |
| TC-WD-002 | Watchdog detects mixer not started | FR-003, FR-004 | [tc_wd_002_mixer_not_started.md](tc_wd_002_mixer_not_started.md) |
| TC-WD-003 | Watchdog detects missing next buffer | FR-003, FR-004 | [tc_wd_003_missing_next_buffer.md](tc_wd_003_missing_next_buffer.md) |

### End-to-End Tests (TC-E2E-001 to TC-E2E-002)

| Test ID | Description | Requirements | Specification File |
|---------|-------------|--------------|-------------------|
| TC-E2E-001 | Complete playback flow (event-driven) | FR-001, FR-002, FR-004, NFR-001 | [tc_e2e_001_complete_playback.md](tc_e2e_001_complete_playback.md) |
| TC-E2E-002 | Multi-passage queue build (event-driven) | FR-001, FR-004 | [tc_e2e_002_multi_passage_queue.md](tc_e2e_002_multi_passage_queue.md) |

### Watchdog Disabled Tests (TC-WD-DISABLED-001)

| Test ID | Description | Requirements | Specification File |
|---------|-------------|--------------|-------------------|
| TC-WD-DISABLED-001 | Event system works without watchdog | FR-001, FR-002, FR-004, NFR-003 | [tc_wd_disabled_001_event_system_only.md](tc_wd_disabled_001_event_system_only.md) |

---

## Test Infrastructure Requirements

### Test Helpers (Existing - from PROJ001)

**Location:** [wkmp-ap/tests/test_engine.rs](../../../wkmp-ap/tests/test_engine.rs)

- `TestEngine` wrapper (in-memory SQLite, automatic cleanup)
- Test asset support (100ms silence MP3)
- Helper methods for state inspection

### New Test Infrastructure Needed

**DecoderWorkerSpy** (§5.4, lines 789-826):
- Track decode requests with timestamps
- Verify decode priority
- Measure latency (<1ms verification)

**BufferManagerMock** (§5.4, lines 829-847):
- Simulate buffer fill
- Emit threshold events
- Track event emission timing

**Test Configuration:**
- `watchdog_enabled` flag (disable for event-only tests)
- Test mode panic on watchdog intervention

---

## Success Criteria Summary

**All tests must:**
1. ✓ Execute in <3s total (CI-ready)
2. ✓ Use BDD format (Given/When/Then)
3. ✓ Verify event-driven behavior (no polling)
4. ✓ Fail if watchdog intervenes (panic in test mode)
5. ✓ Measure latency where applicable (<1ms)

**Test Suite Characteristics:**
- Location: `wkmp-ap/tests/event_driven_playback_tests.rs`
- Pattern: Follow PROJ001 chain_assignment_tests.rs structure
- Isolation: Each test independent, no shared state
- Cleanup: Automatic via TestEngine wrapper

---

## Traceability Matrix Preview

**Full matrix:** [traceability_matrix.md](../traceability_matrix.md)

**Coverage Summary:**
- FR-001: 5 tests (TC-ED-001, TC-ED-002, TC-ED-003, TC-E2E-001, TC-E2E-002)
- FR-002: 4 tests (TC-ED-004, TC-ED-005, TC-E2E-001, TC-WD-DISABLED-001)
- FR-003: 3 tests (TC-WD-001, TC-WD-002, TC-WD-003)
- FR-004: All 11 tests
- NFR-001: 3 tests (TC-ED-001, TC-ED-004, TC-E2E-001)
- NFR-003: 1 test (TC-WD-DISABLED-001)
- NFR-004: Regression test suite (PROJ001, existing playback tests)

**100% requirement coverage** - all requirements have defined tests

---

## Test Execution Order

**Recommended sequence for implementation:**

1. **Phase 1: Event Infrastructure Tests**
   - Verify event emission (not full test suite, just infrastructure)

2. **Phase 2: Event-Driven Decode Tests**
   - TC-ED-001, TC-ED-002, TC-ED-003

3. **Phase 3: Event-Driven Mixer Tests**
   - TC-ED-004, TC-ED-005

4. **Phase 4: Watchdog Tests**
   - TC-WD-001, TC-WD-002, TC-WD-003

5. **Phase 5: Watchdog Disabled Test**
   - TC-WD-DISABLED-001

6. **Phase 6: End-to-End Tests**
   - TC-E2E-001, TC-E2E-002

7. **Phase 7: Regression Validation**
   - Run existing test suite (PROJ001, etc.)

---

## Next Steps

1. Create individual test specification files (tc_*.md)
2. Create traceability matrix
3. Update plan summary with test coverage
4. Begin implementation (test-first approach)
