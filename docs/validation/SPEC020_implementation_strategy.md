# SPEC008 Developer UI Implementation Strategy

**Date:** 2025-10-20
**Objective:** Revise buffer chain monitor from card-based to table-based layout per SPEC008
**Risk Level:** MEDIUM (UI-only changes, but must verify no backend impact)

---

## Executive Summary

**Scope:** Revise wkmp-ap Developer UI buffer chain monitor to table-based layout with 7 columns per SPEC008-developer_ui_design.md

**Key Changes:**
1. Replace card-based layout with uniform table (all N chains visible)
2. Update column structure: Chain #, Queue Pos, Decoder, Resample, Fade, Buffer, Mixer
3. Fix queue position indexing (0-based per SPEC008 vs 1-based current implementation)
4. Remove redundant CSS and dead JavaScript code
5. Ensure >70% test coverage for at-risk backend components BEFORE UI changes

**Risk Mitigation:**
- UI-only changes (no backend API modifications)
- Existing test coverage: 177 unit tests + 6 integration tests passing
- Incremental approach: test after each phase

---

## Phase 1: Current Implementation Review

###

 Objective
Analyze existing developer_ui.html and identify all components affected by SPEC008 changes.

### Tasks

**Task 1.1:** Read current developer_ui.html implementation
- Location: `wkmp-ap/src/api/developer_ui.html`
- Identify all JavaScript functions related to buffer chain display
- Identify all CSS styles related to buffer chain display

**Task 1.2:** Identify discrepancies with SPEC008
- Current: Card-based layout (detailed view for 1-2, compact for 3-12)
- SPEC008: Table-based layout (uniform rows for all N chains)
- Current: 1-indexed queue positions (1=now playing, 2=next)
- SPEC008: 0-indexed queue positions (0=now playing, 1=next)

**Task 1.3:** Document current component inventory
- CSS classes in use
- JavaScript functions in use
- SSE event handlers
- Data transformation logic

**Deliverables:**
- Component inventory document
- Discrepancy list (current vs SPEC008)

---

## Phase 2: Risk Assessment & At-Risk Functionality

### Objective
Identify all components that could be affected by UI changes and assess risk level.

### Risk Classification

| Component | Risk Level | Rationale |
|-----------|------------|-----------|
| Buffer chain monitor UI | HIGH | Complete redesign (card → table) |
| get_buffer_chains() backend | LOW | No API changes needed |
| BufferChainInfo data model | MEDIUM | Queue position indexing change (1-based → 0-based) |
| SSE BufferChainStatus event | LOW | No changes needed |
| Queue manager | LOW | No changes (internal uses 0-based already) |
| Developer UI JavaScript | MEDIUM | Significant refactoring of rendering logic |
| CSS styles | HIGH | Complete replacement (cards → table) |

### At-Risk Components Requiring >70% Test Coverage

**Component 1: BufferChainInfo data model**
- Current test coverage: Structural tests exist (events.rs has no unit tests for data model)
- Required: Unit tests for queue_position field semantics
- Action: Add tests BEFORE changing indexing

**Component 2: get_buffer_chains() method**
- Current test coverage: ~85% (4 comprehensive tests added in Phase 5)
- Status: ✅ ALREADY >70% - No additional tests needed before changes

**Component 3: SSE event serialization**
- Current test coverage: Implicit (events.rs uses derive(Serialize))
- Required: Integration test for BufferChainStatus SSE event format
- Action: Add test BEFORE UI changes to ensure backward compatibility

### Components NOT at Risk

**No changes needed:**
1. BufferManager (buffer state logic unchanged)
2. DecoderPool (no UI dependencies)
3. QueueManager (internal queue logic unchanged)
4. Mixer (no UI dependencies)
5. Settings loading (maximum_decode_streams logic unchanged)

---

## Phase 3: Test Coverage Enhancement (Pre-Implementation)

### Objective
Ensure >70% test coverage for all at-risk components BEFORE making UI changes.

### Task 3.1: Add Unit Tests for Queue Position Semantics

**File:** `wkmp-common/src/events.rs`

**Tests to add:**
1. `test_buffer_chain_info_queue_position_semantics()` - Verify 0-based indexing
2. `test_buffer_chain_info_idle_constructor()` - Verify idle chain has queue_position: None
3. `test_buffer_chain_info_serialization()` - Verify JSON format for SSE

**Implementation:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_chain_info_queue_position_semantics() {
        // Position 0 = "now playing"
        let chain_0 = BufferChainInfo {
            queue_position: Some(0),
            // ... other fields
        };
        assert_eq!(chain_0.queue_position, Some(0), "Position 0 should be 'now playing'");

        // Position 1 = "up next"
        let chain_1 = BufferChainInfo {
            queue_position: Some(1),
            // ... other fields
        };
        assert_eq!(chain_1.queue_position, Some(1), "Position 1 should be 'up next'");

        // Idle chain has None
        let idle = BufferChainInfo::idle(5);
        assert_eq!(idle.queue_position, None, "Idle chain should have queue_position None");
    }

    #[test]
    fn test_buffer_chain_info_serialization() {
        let chain = BufferChainInfo {
            slot_index: 0,
            queue_position: Some(0),
            buffer_fill_percent: 65.5,
            // ... other fields
        };

        let json = serde_json::to_string(&chain).unwrap();
        assert!(json.contains("\"queue_position\":0"));
        assert!(json.contains("\"buffer_fill_percent\":65.5"));
    }
}
```

### Task 3.2: Add Integration Test for SSE Buffer Chain Event

**File:** `wkmp-ap/tests/sse_buffer_chain_tests.rs` (NEW FILE)

**Test:**
```rust
#[tokio::test]
async fn test_buffer_chain_status_sse_event_format() {
    // Start wkmp-ap server
    // Connect to /events SSE endpoint
    // Enqueue 3 passages
    // Wait for BufferChainStatus event
    // Verify JSON structure matches SPEC008
    // Verify queue_position values are 0-based (0, 1, 2)
}
```

### Success Criteria

- ✅ All new tests passing
- ✅ Test coverage for events.rs >70%
- ✅ Integration test validates SSE event format
- ✅ No breaking changes to existing 177 unit tests

---

## Phase 4: Backend Queue Position Indexing Fix

### Objective
Change queue_position from 1-based to 0-based per SPEC008 (if needed).

### Analysis

**Current Implementation** (engine.rs:~886-900):
```rust
for (slot_index, entry) in all_entries.iter().take(maximum_decode_streams).enumerate() {
    let queue_position = slot_index + 1;  // 1-INDEXED (current)
    // ...
}
```

**SPEC008 Requirement:**
```
Column 2: Queue Position
- Value: N/A for idle, 0 for Now Playing, 1 for Up Next, 2+ for queued
```

**Change Required:**
```rust
for (slot_index, entry) in all_entries.iter().take(maximum_decode_streams).enumerate() {
    let queue_position = slot_index;  // 0-INDEXED (SPEC008)
    // ...
}
```

### Impact Analysis

**Files affected:**
1. `wkmp-ap/src/playback/engine.rs` - get_buffer_chains() method
2. `wkmp-ap/tests/buffer_chain_monitoring_tests.rs` - Integration tests (assertions need updating)
3. `wkmp-ap/src/api/developer_ui.html` - Frontend display logic

**Backward compatibility:**
- ⚠️ **BREAKING CHANGE** for any external consumers of BufferChainStatus SSE event
- ✅ **NO IMPACT** on internal components (QueueManager already uses 0-based internally)

### Implementation Steps

**Step 4.1:** Update get_buffer_chains() method
- Change: `slot_index + 1` → `slot_index`
- Update comments to reflect 0-based semantics
- Update traceability tags

**Step 4.2:** Update integration tests
- File: `wkmp-ap/tests/buffer_chain_monitoring_tests.rs`
- Change all assertions from `Some(1)` → `Some(0)`, `Some(2)` → `Some(1)`, etc.
- Update test documentation

**Step 4.3:** Update unit tests
- File: `wkmp-ap/src/playback/engine.rs` (test module)
- Update test_buffer_chain_queue_position_tracking() assertions

**Step 4.4:** Run full test suite
```bash
cargo test -p wkmp-ap --lib
cargo test -p wkmp-ap --test buffer_chain_monitoring_tests
```

**Expected result:** All 183 tests pass with updated assertions

---

## Phase 5: UI Implementation - Table-Based Layout

### Objective
Replace card-based buffer chain monitor with table-based layout per SPEC008.

### Task 5.1: Remove Old CSS (Cards)

**File:** `wkmp-ap/src/api/developer_ui.html`

**CSS to remove:**
- `.buffer-chain-full` (detailed card style)
- `.buffer-chain-compact` (compact card style)
- `.compact-grid` (2-column grid for compact cards)
- `.pipeline-stage` (individual stage boxes)

**Estimated lines removed:** ~116 lines

### Task 5.2: Add New CSS (Table)

**New styles required:**

```css
/* Buffer Chain Monitor - Table Layout [SPEC008-MONITOR-020] */
.buffer-chain-table {
    width: 100%;
    border-collapse: collapse;
    font-family: 'Courier New', monospace;
    font-size: 12px;
}

.buffer-chain-table th {
    background-color: #1e293b;
    color: #f1f5f9;
    padding: 8px;
    text-align: left;
    border-bottom: 2px solid #475569;
}

.buffer-chain-table td {
    padding: 6px 8px;
    border-bottom: 1px solid #334155;
}

/* Queue Position Visual Treatment [SPEC008-MONITOR-050] */
.queue-pos-now-playing {
    font-weight: bold;
    color: #4ade80; /* Green */
}

.queue-pos-up-next {
    font-weight: bold;
    color: #fbbf24; /* Yellow */
}

.queue-pos-queued {
    color: #3b82f6; /* Blue */
}

.queue-pos-idle {
    color: #94a3b8; /* Gray */
}

/* Buffer Fill Visual Indicators [SPEC008-MONITOR-090] */
.buffer-fill-low {
    background-color: #7f1d1d; /* Red background <25% */
}

.buffer-fill-normal {
    /* No special styling 25-75% */
}

.buffer-fill-high {
    background-color: #14532d; /* Green background >75% */
}
```

**Estimated lines added:** ~80 lines

### Task 5.3: Remove Old JavaScript (Card Rendering)

**Functions to remove:**
- `renderChainFull(chain)` - Detailed card rendering
- `renderChainCompact(chain)` - Compact card rendering
- Card-specific DOM manipulation logic in `updateBufferChainDisplay()`

**Estimated lines removed:** ~174 lines

### Task 5.4: Add New JavaScript (Table Rendering)

**New function:**

```javascript
/**
 * Update buffer chain monitor table
 * [SPEC008-MONITOR-030] 7-column table layout
 */
function updateBufferChainDisplay(chains) {
    const container = document.getElementById('buffer-chain-monitor');

    // Ensure all N chains present (fill with idle if needed)
    const maxChains = chains[0]?.slot_index !== undefined ? 12 : chains.length;
    while (chains.length < maxChains) {
        chains.push(createIdleChain(chains.length));
    }

    // Sort by chain number (slot_index) - static 0 to N-1
    chains.sort((a, b) => a.slot_index - b.slot_index);

    // Build table HTML
    let html = '<table class="buffer-chain-table">';
    html += '<thead><tr>';
    html += '<th>Chain #</th>';          // Column 1
    html += '<th>Queue Pos</th>';        // Column 2
    html += '<th>Decoder</th>';          // Column 3
    html += '<th>Resample</th>';         // Column 4
    html += '<th>Fade</th>';             // Column 5
    html += '<th>Buffer</th>';           // Column 6
    html += '<th>Mixer</th>';            // Column 7
    html += '</tr></thead><tbody>';

    // Render all chains
    for (const chain of chains) {
        html += renderChainRow(chain);
    }

    html += '</tbody></table>';
    container.innerHTML = html;
}

/**
 * Render single chain table row
 * [SPEC008-MONITOR-030] Row format
 */
function renderChainRow(chain) {
    // Column 1: Chain # (static 0-based) [SPEC008-MONITOR-040]
    const chainNum = chain.slot_index;

    // Column 2: Queue Pos [SPEC008-MONITOR-050]
    const queuePos = formatQueuePosition(chain.queue_position);
    const queuePosClass = getQueuePositionClass(chain.queue_position);

    // Column 3: Decoder [SPEC008-MONITOR-060]
    const decoderStatus = formatDecoderStatus(chain);

    // Column 4: Resample [SPEC008-MONITOR-070]
    const resampleStatus = formatResampleStatus(chain);

    // Column 5: Fade [SPEC008-MONITOR-080]
    const fadeStatus = formatFadeStatus(chain);

    // Column 6: Buffer [SPEC008-MONITOR-090]
    const bufferFill = formatBufferFill(chain);
    const bufferClass = getBufferFillClass(chain.buffer_fill_percent);

    // Column 7: Mixer [SPEC008-MONITOR-100]
    const mixerStatus = formatMixerStatus(chain);

    return `
        <tr>
            <td>${chainNum}</td>
            <td class="${queuePosClass}">${queuePos}</td>
            <td>${decoderStatus}</td>
            <td>${resampleStatus}</td>
            <td>${fadeStatus}</td>
            <td class="${bufferClass}">${bufferFill}</td>
            <td>${mixerStatus}</td>
        </tr>
    `;
}

/** Format queue position: N/A, 0, 1, 2, ... [SPEC008-MONITOR-050] */
function formatQueuePosition(queuePos) {
    return queuePos !== null && queuePos !== undefined ? queuePos : 'N/A';
}

/** Get CSS class for queue position styling */
function getQueuePositionClass(queuePos) {
    if (queuePos === null || queuePos === undefined) return 'queue-pos-idle';
    if (queuePos === 0) return 'queue-pos-now-playing';
    if (queuePos === 1) return 'queue-pos-up-next';
    return 'queue-pos-queued';
}

/** Format decoder status [SPEC008-MONITOR-060] */
function formatDecoderStatus(chain) {
    if (!chain.decoder_state) return 'N/A';

    let status = chain.decoder_state;
    if (chain.decode_progress_percent !== null) {
        status += ` ${chain.decode_progress_percent}%`;
    }
    return status;
}

/** Format resample status [SPEC008-MONITOR-070] */
function formatResampleStatus(chain) {
    if (!chain.resampler_active) return 'N/A';

    const source = chain.source_sample_rate || '?';
    const target = chain.target_sample_rate || 44100;
    return `${source} → ${target} Hz`;
}

/** Format fade status [SPEC008-MONITOR-080] */
function formatFadeStatus(chain) {
    if (!chain.fade_stage) return '-';
    return chain.fade_stage;
}

/** Format buffer fill [SPEC008-MONITOR-090] */
function formatBufferFill(chain) {
    return `${chain.buffer_fill_percent.toFixed(1)}%`;
}

/** Get CSS class for buffer fill visual indicator */
function getBufferFillClass(fillPercent) {
    if (fillPercent < 25) return 'buffer-fill-low';
    if (fillPercent > 75) return 'buffer-fill-high';
    return 'buffer-fill-normal';
}

/** Format mixer status [SPEC008-MONITOR-100] */
function formatMixerStatus(chain) {
    if (!chain.is_active_in_mixer) return 'Idle';

    let status = 'Feeding';
    if (chain.mixer_role && chain.mixer_role !== 'Idle') {
        status += ` [${chain.mixer_role}]`;
    }
    return status;
}

/** Create idle chain placeholder */
function createIdleChain(slotIndex) {
    return {
        slot_index: slotIndex,
        queue_entry_id: null,
        queue_position: null,
        decoder_state: null,
        decode_progress_percent: null,
        is_actively_decoding: false,
        source_sample_rate: null,
        resampler_active: false,
        target_sample_rate: 44100,
        fade_stage: null,
        buffer_state: 'Idle',
        buffer_fill_percent: 0.0,
        buffer_fill_samples: 0,
        buffer_capacity_samples: 0,
        is_active_in_mixer: false,
        mixer_role: 'Idle'
    };
}
```

**Estimated lines added:** ~150 lines

### Net Code Change (Phase 5)

- **CSS removed:** ~116 lines
- **CSS added:** ~80 lines
- **JavaScript removed:** ~174 lines
- **JavaScript added:** ~150 lines
- **Net change:** -60 lines (code reduction + cleanup)

---

## Phase 6: Dead Code Removal & Cleanup

### Objective
Identify and remove redundant code and dead code paths exposed by UI redesign.

### Task 6.1: Identify Dead Code

**Candidates for removal:**

1. **Unused CSS classes**
   - `.pipeline-stage` (no longer used in table layout)
   - `.state-badge` (replaced by table cell styling)
   - Any gradient/shadow styles specific to cards

2. **Unused JavaScript helper functions**
   - Any functions exclusively called by removed card rendering code
   - Duplicate formatting logic (consolidate where possible)

3. **Commented-out code**
   - Search for `//` and `/* */` blocks in developer_ui.html
   - Remove if no longer relevant

### Task 6.2: Code Consolidation Opportunities

**Opportunity 1:** Consolidate CSS color variables
```css
:root {
    --color-now-playing: #4ade80;
    --color-up-next: #fbbf24;
    --color-queued: #3b82f6;
    --color-idle: #94a3b8;
    --color-buffer-low: #7f1d1d;
    --color-buffer-high: #14532d;
}
```

**Opportunity 2:** Extract common formatting functions
- Combine percentage formatting (buffer, decode progress)
- Consolidate N/A handling logic

### Success Criteria

- ✅ No unused CSS classes remaining
- ✅ No unreachable JavaScript code
- ✅ Code reduction: -60 lines minimum
- ✅ All tests still passing after cleanup

---

## Phase 7: Validation & Testing

### Objective
Verify all functionality works correctly and no regressions introduced.

### Task 7.1: Visual Validation (Manual)

**Checklist** (per SPEC008-VALIDATE-010):

- [ ] All N chains visible in table (verify for N=12)
- [ ] Chain numbers 0 to N-1 displayed (static, not dynamic)
- [ ] Queue position 0 shows bold green
- [ ] Queue position 1 shows bold yellow
- [ ] Queue position 2+ shows blue
- [ ] Idle chains show "N/A" in queue position (gray)
- [ ] Buffer fill < 25% shows red background
- [ ] Buffer fill > 75% shows green background
- [ ] Decoder status shows "N/A" or actual state
- [ ] Resample status shows "N/A" for 44.1kHz sources
- [ ] Fade status displays current stage
- [ ] Mixer status shows "Idle" when playback paused
- [ ] Table updates in real-time (1Hz SSE events)

### Task 7.2: Functional Testing

**Test scenarios** (per SPEC008-VALIDATE-020):

1. **Empty queue:** All chains idle, queue pos = N/A
2. **Single passage:** Chain 0 at position 0, others idle
3. **Three passages:** Chains 0-2 at positions 0-2, others idle
4. **Full queue (N=12):** All chains active, positions 0-11
5. **Queue advance (skip):** Position numbers shift correctly (0→removed, 1→0, 2→1)
6. **Pause state:** All mixer statuses change to "Idle"
7. **Resume state:** Mixer statuses restore (position 0 = "Feeding [Current]")
8. **SSE reconnect:** UI recovers, table rebuilds correctly

### Task 7.3: Unit Test Execution

**Run all tests:**
```bash
# Unit tests (should still have 177 passing after updates)
cargo test -p wkmp-ap --lib

# Integration tests (6 tests, updated assertions)
cargo test -p wkmp-ap --test buffer_chain_monitoring_tests

# New SSE event test
cargo test -p wkmp-ap --test sse_buffer_chain_tests

# Build verification
cargo build -p wkmp-ap
```

**Success criteria:**
- ✅ 177+ unit tests passing (may add more)
- ✅ 6 integration tests passing (with updated 0-based assertions)
- ✅ 1 new SSE event test passing
- ✅ Clean build (no new warnings)

### Task 7.4: Performance Validation

**Performance tests** (per SPEC008-VALIDATE-030):

1. **Update latency:** SSE event to DOM update < 100ms
2. **CPU usage:** < 15% during 1Hz updates (12 chains)
3. **Memory:** No leaks after 1000 table refreshes
4. **High chain count:** N=32 chains (maximum_decode_streams upper limit)

**Measurement tools:**
- Browser DevTools Performance profiler
- Chrome Task Manager (memory monitoring)
- Manual stopwatch for latency (event timestamp vs DOM update)

---

## Phase 8: Documentation Updates

### Objective
Update all documentation to reflect table-based implementation.

### Task 8.1: Update SPEC016 Findings Report

**File:** `docs/validation/SPEC016_buffer_chain_implementation_findings.md`

**Changes:**
- Add section on SPEC008 compliance
- Update UI implementation description (cards → table)
- Update screenshot/example to show table layout
- Note queue position indexing change (1-based → 0-based)

### Task 8.2: Create Implementation Log

**File:** `docs/validation/SPEC008_implementation_log.json`

**Content:**
```json
{
  "document": "SPEC008_implementation_log",
  "version": "1.0",
  "date": "2025-10-20",
  "phases": [
    {
      "phase": 1,
      "name": "Current Implementation Review",
      "status": "complete",
      "findings": "..."
    },
    // ... all phases
  ],
  "test_coverage": {
    "before": "177 unit + 6 integration",
    "after": "180+ unit + 7 integration",
    "target": ">70% for at-risk components",
    "achieved": true
  },
  "code_metrics": {
    "css_removed": 116,
    "css_added": 80,
    "js_removed": 174,
    "js_added": 150,
    "net_change": -60
  }
}
```

### Task 8.3: Update Developer UI Comments

**File:** `wkmp-ap/src/api/developer_ui.html`

**Add traceability comments:**
```html
<!-- Buffer Chain Monitor Table Layout -->
<!-- [SPEC008-MONITOR-020] Table-based design for all N chains -->
<!-- [SPEC008-MONITOR-030] 7-column structure: Chain#, Queue Pos, Decoder, Resample, Fade, Buffer, Mixer -->
```

---

## Implementation Timeline

### Estimated Effort

| Phase | Estimated Time | Dependencies |
|-------|----------------|--------------|
| Phase 1: Review | 1 hour | None |
| Phase 2: Risk Assessment | 30 minutes | Phase 1 |
| Phase 3: Test Coverage | 2 hours | Phase 2 |
| Phase 4: Backend Indexing Fix | 1 hour | Phase 3 complete |
| Phase 5: UI Implementation | 3 hours | Phase 4 complete |
| Phase 6: Dead Code Cleanup | 1 hour | Phase 5 complete |
| Phase 7: Validation | 2 hours | Phase 6 complete |
| Phase 8: Documentation | 1 hour | Phase 7 complete |
| **Total** | **11.5 hours** | Sequential |

### Parallelization Opportunities

**Cannot parallelize:**
- Phases 3-4-5 (must ensure tests pass before changes)
- Phases 5-6-7 (must implement before validation)

**Can parallelize:**
- Phase 8 (documentation) can start during Phase 7 (validation)

---

## Risk Mitigation

### Rollback Plan

**If implementation fails validation:**

1. **Revert UI changes:** Git checkout previous developer_ui.html
2. **Revert indexing change:** Git checkout previous engine.rs get_buffer_chains()
3. **Revert test updates:** Git checkout previous test assertions
4. **Verify rollback:** Run full test suite (should return to 177 passing)

**Rollback trigger:** Any of:
- >10% of tests failing after Phase 4
- UI completely broken (table not rendering)
- Performance regression >50ms update latency

### Incremental Validation

**After each phase:**
1. Run relevant test subset
2. Visual check (for UI phases)
3. Git commit with descriptive message
4. Proceed to next phase only if current phase validated

**Git commit messages:**
```
Phase 3: Add queue position semantics tests (3 new tests)
Phase 4: Fix queue position indexing (1-based → 0-based)
Phase 5: Implement table-based buffer chain monitor (SPEC008)
Phase 6: Remove dead code from card-based layout (-60 lines)
Phase 7: Validation complete - all tests passing
Phase 8: Update documentation for SPEC008 implementation
```

---

## Success Criteria

### Definition of Done

**All criteria must be met:**

1. ✅ SPEC008 table-based layout fully implemented
2. ✅ All 7 columns present and correctly formatted
3. ✅ Queue position 0-indexed (0=now playing, 1=up next)
4. ✅ All N chains visible (no hidden chains)
5. ✅ 177+ unit tests passing
6. ✅ 6+ integration tests passing (with updated assertions)
7. ✅ >70% test coverage for at-risk components maintained
8. ✅ Code reduction achieved (target: -60 lines minimum)
9. ✅ No new compiler warnings introduced
10. ✅ Performance targets met (<100ms update latency)
11. ✅ Documentation updated (SPEC016 findings, implementation log)
12. ✅ Manual validation checklist 100% complete

---

## Appendix: Multi-Agent Deployment Strategy

### Agent Roles

**Agent 1: Test Engineer**
- **Responsibility:** Phase 3 (Test Coverage Enhancement)
- **Deliverables:** 3 new unit tests, 1 new integration test
- **Success metric:** >70% coverage for events.rs

**Agent 2: Backend Engineer**
- **Responsibility:** Phase 4 (Queue Position Indexing Fix)
- **Deliverables:** Updated get_buffer_chains(), updated tests
- **Success metric:** All 177+ tests passing with 0-based indexing

**Agent 3: Frontend Engineer**
- **Responsibility:** Phase 5 (UI Implementation)
- **Deliverables:** Table-based buffer chain monitor (SPEC008-compliant)
- **Success metric:** Visual validation checklist 100% complete

**Agent 4: Code Quality Engineer**
- **Responsibility:** Phase 6 (Dead Code Removal)
- **Deliverables:** Cleanup report, -60 lines minimum reduction
- **Success metric:** No unused CSS/JS code remaining

**Agent 5: QA Engineer**
- **Responsibility:** Phase 7 (Validation)
- **Deliverables:** Test execution report, performance metrics
- **Success metric:** All functional tests passing, latency <100ms

**Agent 6: Documentation Engineer**
- **Responsibility:** Phase 8 (Documentation)
- **Deliverables:** Updated SPEC016 findings, implementation log
- **Success metric:** All traceability comments present

### Parallel Execution Plan

**Sequential groups:**

**Group 1:** Phase 1-2 (Review & Risk Assessment) - Single executor
**Group 2:** Phase 3 (Test Coverage) - Agent 1 (Test Engineer)
**Group 3:** Phase 4 (Backend Fix) - Agent 2 (Backend Engineer)
**Group 4:** Phase 5-6 (UI + Cleanup) - Agent 3 + Agent 4 (parallel possible after Phase 5 complete)
**Group 5:** Phase 7 (Validation) - Agent 5 (QA Engineer)
**Group 6:** Phase 8 (Documentation) - Agent 6 (parallel with Phase 7)

**Dependencies:**
- Group 2 → Group 3 (tests must exist before backend changes)
- Group 3 → Group 4 (backend must be stable before UI changes)
- Group 4 → Group 5 (implementation complete before validation)
- Group 5 ← Group 6 (documentation can start during validation)

---

**Document Version:** 1.0
**Author:** Implementation Strategy Lead
**Status:** Ready for Execution
**Approval:** Pending technical review
