# PLAN020: Requirements Index

**Source:** [wip/SPEC_event_driven_playback_refactor.md](../SPEC_event_driven_playback_refactor.md)
**Total Requirements:** 8 (4 functional, 4 non-functional)

---

## Requirements Table

| ID | Type | Priority | Description | Success Criteria | Source |
|----|------|----------|-------------|------------------|--------|
| FR-001 | Functional | P0 | Event-Driven Decode Initiation | Decode triggered <1ms on EnqueueEvent/QueueAdvanceEvent | §3.1 |
| FR-002 | Functional | P0 | Event-Driven Mixer Startup | Mixer starts <1ms on BufferThresholdReached event | §3.1 |
| FR-003 | Functional | P0 | Watchdog Loop (100ms safety net) | Detects stuck states, logs WARN, restores state | §3.1 |
| FR-004 | Functional | P0 | Event System Test Coverage | Tests verify event-driven behavior, fail on watchdog intervention | §3.1 |
| NFR-001 | Non-Functional | P0 | Responsiveness | Event operations <1ms latency (vs 0-100ms polling) | §3.2 |
| NFR-002 | Non-Functional | P1 | CPU Efficiency | Watchdog minimal checks (5-10 evaluations), no DB queries | §3.2 |
| NFR-003 | Non-Functional | P0 | Testability | Events testable in isolation, watchdog testable separately | §3.2 |
| NFR-004 | Non-Functional | P0 | Backward Compatibility | External behavior, API, database schema unchanged | §3.2 |

---

## Detailed Requirements

### FR-001: Event-Driven Decode Initiation

**Priority:** P0 (Critical)
**Source:** Specification §3.1 (lines 67-74)

**Description:**
Decode requests triggered immediately by queue state changes, not polled.

**Events:**
- `EnqueueEvent` → Trigger decode for newly enqueued passage
- `QueueAdvanceEvent` → Trigger decode for newly promoted next/queued passages

**Behavior:**
- Same priority system as current (Immediate, Next, Prefetch)
- Latency: <1ms (vs. current 0-100ms random latency)

**Success Criteria:**
- ✓ Decode request issued within 1ms of enqueue
- ✓ Decode request issued within 1ms of queue advance
- ✓ Priority correct by position (Current=Immediate, Next=Next, Queued=Prefetch)
- ✓ No watchdog interventions for decode initiation

**Test Coverage:** TC-ED-001, TC-ED-002, TC-ED-003

---

### FR-002: Event-Driven Mixer Startup

**Priority:** P0 (Critical)
**Source:** Specification §3.1 (lines 75-81)

**Description:**
Mixer starts when buffer reaches threshold via event, not polled.

**Event:**
- `BufferThresholdReached(queue_entry_id, threshold_ms)`
- Emitted by BufferManager when accumulated PCM ≥ threshold (3000ms default)

**Behavior:**
- Start mixer with markers (same as current)
- Emit PassageStarted event
- Latency: <1ms (vs. current 0-100ms random latency)

**Success Criteria:**
- ✓ BufferThresholdReached event emitted when buffer ≥ 3000ms
- ✓ Mixer startup initiated within 1ms of event
- ✓ PassageStarted event emitted
- ✓ No duplicate mixer starts if already playing
- ✓ No watchdog interventions for mixer startup

**Test Coverage:** TC-ED-004, TC-ED-005

---

### FR-003: Watchdog Loop (100ms)

**Priority:** P0 (Critical)
**Source:** Specification §3.1 (lines 82-100)

**Description:**
Safety mechanism detecting and recovering from event system failures.

**Behavior:**
- Check for "stuck" states every 100ms (configurable via database setting)
- Log **WARN** when intervention required
- Restore proper state
- **CRITICAL:** Intervention indicates event system bug

**Stuck States Detected:**
1. Current passage buffer missing (should decode)
2. Mixer idle with ready buffer (should start)
3. Next passage buffer missing with playing current (should prefetch)
4. Queued passage buffer missing with available decode slots (should prefetch)

**Logging Format:**
```
[WARN] [WATCHDOG] Event system failure detected: {reason}.
       Intervention: {action}. This indicates a bug in event-driven logic.
```

**Configuration (Database Settings):**
- Setting key: `watchdog_interval_ms`
- Default: 100ms
- Valid range: 10ms (fast recovery) to 2000ms (minimal overhead)
- Hot-reload: Support runtime changes without restart

**Success Criteria:**
- ✓ Watchdog detects all 4 stuck state types
- ✓ WARN logged on every intervention with correct format
- ✓ State restored after intervention
- ✓ Configurable interval via database settings (10-2000ms range)
- ✓ In test mode: panic on intervention (test failure)
- ✓ In production: recover gracefully

**Test Coverage:** TC-WD-001, TC-WD-002, TC-WD-003

---

### FR-004: Event System Test Coverage

**Priority:** P0 (Critical)
**Source:** Specification §3.1 (lines 102-108)

**Description:**
Test suite validates event-driven behavior without watchdog dependency.

**Requirements:**
1. Unit tests for each event trigger path
2. Integration tests verify end-to-end event flows
3. Test environment disables watchdog interventions OR fails test if watchdog intervenes
4. Mock event system for failure testing

**Success Criteria:**
- ✓ All event triggers have dedicated unit tests
- ✓ End-to-end event flows validated
- ✓ Watchdog intervention in test = test failure (panic)
- ✓ Mock/spy infrastructure for event verification
- ✓ Test suite execution time <3s

**Test Coverage:** All TC-ED-*, TC-WD-*, TC-E2E-* tests

---

### NFR-001: Responsiveness

**Priority:** P0 (Critical)
**Source:** Specification §3.2 (lines 112-115)

**Description:**
Minimize latency for all playback orchestration operations.

**Requirements:**
- Event-driven operations: <1ms latency (vs. current 0-100ms polling)
- Watchdog check interval: 100ms (configurable 10-2000ms)
- No perceptible delay for user operations

**Success Criteria:**
- ✓ Enqueue → Decode start: <1ms (measured)
- ✓ Buffer ready → Mixer start: <1ms (measured)
- ✓ Queue advance → Decode for promoted entries: <1ms (measured)
- ✓ Watchdog check overhead: <0.1ms per iteration

**Test Coverage:** TC-E2E-001, TC-E2E-002 (with timing verification)

---

### NFR-002: CPU Efficiency

**Priority:** P1 (High)
**Source:** Specification §3.2 (lines 117-120)

**Description:**
Minimize CPU overhead for watchdog loop.

**Requirements:**
- Watchdog loop: Minimal checks (5-10 condition evaluations)
- No buffer reads in watchdog (use cached state)
- No database queries in watchdog
- CPU usage during idle playback: <0.1% (vs. current ~1%)

**Success Criteria:**
- ✓ Watchdog code <100 lines (vs. current 350 lines)
- ✓ No buffer content reads (only metadata checks)
- ✓ No database queries (in-memory state only)
- ✓ CPU profiling shows <0.1% usage during idle playback

**Test Coverage:** Performance benchmarking (manual validation)

---

### NFR-003: Testability

**Priority:** P0 (Critical)
**Source:** Specification §3.2 (lines 122-126)

**Description:**
Enable isolated testing of event system and watchdog.

**Requirements:**
- Event triggers testable in isolation
- Watchdog testable separately from event system
- Test environment can disable watchdog
- Test failures distinguish event bugs from watchdog bugs

**Success Criteria:**
- ✓ DecoderWorkerSpy allows verification of decode requests
- ✓ BufferManagerMock simulates buffer fill and threshold events
- ✓ Test config flag: `watchdog_enabled: false`
- ✓ Test mode panic on watchdog intervention with clear error message
- ✓ All event paths have dedicated unit tests

**Test Coverage:** TC-WD-DISABLED-001, all mock infrastructure

---

### NFR-004: Backward Compatibility

**Priority:** P0 (Critical)
**Source:** Specification §3.2 (lines 128-132)

**Description:**
Maintain external behavior and interfaces.

**Requirements:**
- External behavior identical to current implementation
- API unchanged (HTTP endpoints, SSE events)
- Database schema unchanged
- Configuration unchanged (new settings additive only)

**Success Criteria:**
- ✓ All existing tests pass (no regressions)
- ✓ No API changes (HTTP/SSE interfaces identical)
- ✓ No database migrations (except additive settings)
- ✓ No changes to external configuration files (TOML, etc.)
- ✓ Playback timing, crossfades, queue behavior identical

**Test Coverage:** Full regression test suite (PROJ001 chain tests, existing playback tests)

---

## Configuration Requirements

**Source:** Specification §10 (lines 1052-1118)

### Database Settings Parameters

Following DBD-PARAM-### pattern for consistency with existing parameters.

| Setting Key | Default | Valid Range | Type | Hot-Reload | Source |
|-------------|---------|-------------|------|------------|--------|
| `watchdog_interval_ms` | 100ms | 10-2000ms | u64 | Yes | §10 Q2/A2 |
| `minimum_playback_buffer_ms` | 3000ms | 100-12000ms | u64 | Yes | §10 Q3/A3 |

**Requirements:**
- Validation: Clamp to range, log warning if out of bounds
- Storage: SQLite settings table (existing infrastructure)
- Retrieval: Hot-reload support (no restart required)

---

## Telemetry Requirements

**Source:** Specification §10 (lines 1052-1078)

### Watchdog Intervention Metrics

**Metric Name:** `watchdog_interventions_total`
**Type:** Counter (increments on each intervention)

**Labels/Dimensions:**
- `intervention_type`: `missing_current_buffer`, `mixer_not_started`, `missing_next_buffer`, `missing_queued_buffer`
- `queue_entry_id`: UUID of affected entry (optional, for detailed debugging)

**Exposure:**
- Internal metrics endpoint (for monitoring dashboards)
- Log correlation via request ID/timestamp

**Alerting:**
- Alert threshold: >0 interventions in production (indicates event system bug)
- Severity: Warning (system recovers, but bug needs investigation)

**Requirements:**
- ✓ Counter increments on every watchdog intervention
- ✓ Labels correctly categorize intervention type
- ✓ Metric exposed via internal metrics endpoint
- ✓ Zero interventions expected in normal operation

---

## Implementation Requirements Summary

**Code Changes Required:**

1. **Event System (§4.1):**
   - Add PlaybackEvent variants: `Enqueued`, `QueueAdvanced`, `BufferThresholdReached`
   - Implement event channel in BufferManager
   - Add event handlers in PlaybackEngine

2. **Event-Driven Operations (§4.3):**
   - Refactor `enqueue_file()` to trigger decode immediately (§4.3.1)
   - Add threshold detection to `BufferManager.push_samples()` (§4.3.2)
   - Extract `start_mixer_for_current()` method (§4.3.3)
   - Refactor queue advance to return promotion info (§4.3.4)

3. **Watchdog Refactoring (§4.2):**
   - Rename `process_queue()` → `watchdog_check()`
   - Remove proactive operations (~350 lines → ~100 lines, 70% reduction)
   - Add WARN logging on intervention
   - Add database settings support (watchdog_interval_ms)

4. **Test Infrastructure (§5):**
   - Create DecoderWorkerSpy for decode request verification
   - Create BufferManagerMock for buffer simulation
   - Add test mode panic on watchdog intervention
   - Add `watchdog_enabled` config flag

5. **Configuration (§10):**
   - Add `watchdog_interval_ms` setting to database (default: 100ms, range: 10-2000ms)
   - Add `minimum_playback_buffer_ms` setting to database (default: 3000ms, range: 100-12000ms)
   - Implement hot-reload for both settings

6. **Telemetry (§10):**
   - Add `watchdog_interventions_total` counter metric
   - Implement labels: intervention_type, queue_entry_id
   - Expose via internal metrics endpoint

---

## Traceability Notes

**All requirements derived from specification sections:**
- §3.1: Functional Requirements (FR-001 through FR-004)
- §3.2: Non-Functional Requirements (NFR-001 through NFR-004)
- §10: Configuration and Telemetry requirements

**Test specifications will reference:**
- §5.3: Test Cases (TC-ED-*, TC-WD-*, TC-E2E-*)
- §5.4: Test Mocks and Spies (DecoderWorkerSpy, BufferManagerMock)

**Design references:**
- §4.1: Event System Architecture
- §4.2: Watchdog refactoring from process_queue()
- §4.3: Event-driven implementations
- §4.4: Playback loop changes

---

## Next Steps

1. **Phase 2 Verification:** Analyze specification for completeness, ambiguity, consistency
2. **Phase 3 Test Definition:** Create BDD test specifications for each requirement
3. **Traceability Matrix:** Map all requirements to test cases (100% coverage)
