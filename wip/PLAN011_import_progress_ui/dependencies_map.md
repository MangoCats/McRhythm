# PLAN011: Import Progress UI Enhancement - Dependencies Map

**Date:** 2025-10-30
**Phase:** 1 - Dependencies Identification

---

## Overview

This document catalogs all dependencies for the import progress UI enhancement, including existing code, external libraries, and integration points.

---

## Existing Code Dependencies

### Files to be Modified

| File Path | Current Status | Lines | Purpose | Changes Required |
|-----------|----------------|-------|---------|------------------|
| `wkmp-ai/src/models/import_session.rs` | ✅ Exists | ~182 | Import workflow state machine | Add new structs: PhaseProgress, PhaseStatus, SubTaskStatus. Extend ImportProgress struct. |
| `wkmp-common/src/events.rs` | ✅ Exists | ~912 | Event definitions for all modules | Extend ImportProgressUpdate event with phases and current_file fields. |
| `wkmp-ai/src/services/workflow_orchestrator.rs` | ✅ Exists | ~1000+ | Import workflow execution | Initialize phase tracking, update on transitions, track sub-task counters, broadcast enhanced events. |
| `wkmp-ai/src/api/ui.rs` | ✅ Exists | ~383 | Import progress HTML/CSS/JS | Complete UI redesign: checklist, sub-task status, enhanced progress display. |

**Total Files Modified:** 4
**Total Existing Lines:** ~2477 lines
**New Lines:** Estimated +400-600 lines

---

### Files to be Read (Not Modified)

| File Path | Purpose | Why Needed |
|-----------|---------|------------|
| `wkmp-ai/src/models/parameters.rs` | Import parameters model | Understand ImportParameters structure used in ImportSession |
| `wkmp-ai/src/db/sessions.rs` | Session persistence | Understand how sessions are saved/loaded (serialization) |
| `wkmp-ai/src/services/file_scanner.rs` | File scanning logic | Understand SCANNING phase sub-tasks |
| `wkmp-ai/src/services/metadata_extractor.rs` | Metadata extraction | Understand EXTRACTING phase sub-tasks |
| `wkmp-ai/src/services/fingerprinter.rs` | Audio fingerprinting | Understand Chromaprint sub-task |
| `wkmp-ai/src/services/acoustid_client.rs` | AcoustID API | Understand AcoustID sub-task |
| `wkmp-ai/src/services/musicbrainz_client.rs` | MusicBrainz API | Understand MusicBrainz sub-task |
| `wkmp-ai/src/services/essentia_client.rs` | Essentia analysis | Understand Essentia sub-task (FLAVORING phase) |

**Purpose:** Understand existing sub-task logic to add counter tracking

---

## External Library Dependencies

### Existing Dependencies (No Changes)

| Crate | Version | Purpose | Used For |
|-------|---------|---------|----------|
| `serde` | Latest | Serialization | Struct serialization/deserialization |
| `serde_json` | Latest | JSON support | SSE event JSON encoding |
| `chrono` | Latest | Date/time | Timestamps in events |
| `uuid` | Latest | IDs | Session IDs |
| `axum` | Latest | Web framework | HTTP server, SSE |
| `tokio` | Latest | Async runtime | Async/await support |
| `sqlx` | Latest | Database | SQLite operations |

**No New Dependencies Required:** All functionality uses existing libraries

---

### Browser API Dependencies (Frontend)

| API | Browser Support | Purpose | Fallback |
|-----|-----------------|---------|----------|
| Server-Sent Events (SSE) | Chrome 6+, Firefox 6+, Safari 5+ | Real-time progress updates | Long polling (not implemented) |
| `requestAnimationFrame` | All modern browsers | Smooth animations | setTimeout fallback |
| CSS Flexbox | IE 11+, all modern | Layout | Float-based fallback (not implemented) |
| ES6+ JavaScript | Chrome 51+, Firefox 54+, Safari 10+ | Modern JS features | Babel transpilation (not implemented) |

**Minimum Browser Versions:**
- Chrome 90+
- Firefox 88+
- Safari 14+

**Note:** Older browser support not in scope (see scope_statement.md)

---

## Integration Points

### 1. SSE Event Broadcasting

**Integration:** wkmp-ai workflow orchestrator → wkmp-common event bus → HTTP SSE stream → browser

**Current Flow:**
1. `WorkflowOrchestrator.broadcast_progress()` emits `ImportProgressUpdate`
2. Event goes to `EventBus` (wkmp-common)
3. SSE handler streams to connected clients
4. Browser receives event via `EventSource`

**Changes Required:**
- Extend `ImportProgressUpdate` with new fields (wkmp-common/src/events.rs)
- Update `WorkflowOrchestrator.broadcast_progress()` to include phases and current_file
- Update browser JavaScript to parse new fields

**Risk:** Medium - Must maintain backward compatibility

---

### 2. Phase State Tracking

**Integration:** ImportSession state machine → Phase tracking array

**Current:** ImportSession has single `state: ImportState` enum

**New:** ImportSession adds `phases: Vec<PhaseProgress>` array

**Initialization:**
```rust
// On ImportSession::new()
phases: vec![
    PhaseProgress { phase: ImportState::Scanning, status: PhaseStatus::Pending, ... },
    PhaseProgress { phase: ImportState::Extracting, status: PhaseStatus::Pending, ... },
    PhaseProgress { phase: ImportState::Fingerprinting, status: PhaseStatus::Pending, ... },
    PhaseProgress { phase: ImportState::Segmenting, status: PhaseStatus::Pending, ... },
    PhaseProgress { phase: ImportState::Analyzing, status: PhaseStatus::Pending, ... },
    PhaseProgress { phase: ImportState::Flavoring, status: PhaseStatus::Pending, ... },
]
```

**Update Points:**
- On `session.transition_to()`: Update corresponding phase status
- During phase execution: Increment sub-task counters

**Risk:** Low - Additive change, does not affect existing state machine

---

### 3. Sub-Task Counter Tracking

**Integration:** Sub-task execution points → SubTaskStatus counters

**Tracking Points in workflow_orchestrator.rs:**

**FINGERPRINTING Phase:**
- Line ~400: `self.fingerprinter.fingerprint_file()` → Chromaprint counter
- Line ~413: `acoustid.lookup()` → AcoustID counter
- Line ~421: `mb.lookup_recording()` → MusicBrainz counter

**FLAVORING Phase:**
- Line ~710+: AcousticBrainz/Essentia lookup → Flavor counter

**Implementation Pattern:**
```rust
match operation() {
    Ok(_) => sub_task_status.success_count += 1,
    Err(_) => sub_task_status.failure_count += 1,
}
```

**Risk:** Low - Straightforward counter increments

---

### 4. UI DOM Update

**Integration:** SSE event → JavaScript handler → DOM updates

**Update Flow:**
1. `EventSource.addEventListener('ImportProgressUpdate', handler)`
2. Parse event.data JSON
3. Update DOM elements:
   - Checklist: `document.getElementById('phase-N').update()`
   - Sub-tasks: `document.getElementById('subtask-X').update()`
   - Progress bar: `document.getElementById('progress-bar').style.width`
   - Current file: `document.getElementById('current-file').textContent`

**Throttling:**
- Use `requestAnimationFrame` to batch DOM updates
- Track last update timestamp, skip if <100ms since last update

**Risk:** Medium - DOM performance critical, must test throttling

---

## Environment Dependencies

### Development Environment

**Required:**
- Rust toolchain (stable channel, 1.70+)
- Cargo package manager
- SQLite 3.35+
- Git

**Optional (for testing):**
- Test audio files (MP3, FLAC, OGG)
- MusicBrainz API key (for integration tests)
- AcoustID API key (for integration tests)

---

### Runtime Environment

**Server:**
- Linux/macOS/Windows (Rust cross-platform)
- 2GB+ RAM (for 5000+ file import)
- Network access (for MusicBrainz/AcoustID APIs)

**Client:**
- Modern web browser (Chrome 90+, Firefox 88+, Safari 14+)
- JavaScript enabled
- 320px+ screen width (mobile minimum)

---

## No Dependencies (Explicitly)

The following are NOT dependencies (will not be used):

### ❌ Not Using

- React, Vue, Angular (vanilla JavaScript only)
- CSS frameworks (Bootstrap, Tailwind) - custom CSS only
- Chart.js, D3.js (no graphs in scope)
- WebSockets (SSE sufficient)
- Service Workers (not needed)
- IndexedDB (no client-side persistence)
- Web Workers (no heavy client-side computation)

**Rationale:** Keep frontend simple, leverage existing wkmp-ai architecture

---

## Dependency Risks

### Risk 1: SSE Browser Compatibility

**Risk:** Older browsers may not support SSE
**Likelihood:** Low (targeting modern browsers only)
**Impact:** High (no progress updates at all)
**Mitigation:** Document minimum browser versions, no fallback in scope

---

### Risk 2: JSON Serialization Breaking

**Risk:** Adding fields to ImportProgressUpdate breaks serialization
**Likelihood:** Low (serde handles additive fields)
**Impact:** High (no events transmitted)
**Mitigation:** Add explicit serialization tests, verify backward compatibility

---

### Risk 3: DOM Performance Degradation

**Risk:** Rapid SSE events cause UI jank
**Likelihood:** Medium (5000+ files = many events)
**Impact:** Medium (poor UX, but functional)
**Mitigation:** Implement throttling (max 10 updates/sec), batch DOM writes

---

### Risk 4: Phase Counter Synchronization

**Risk:** Phase counters get out of sync with actual state
**Likelihood:** Low (counters updated immediately)
**Impact:** Medium (incorrect percentages displayed)
**Mitigation:** Careful state management, add assertions, test with real imports

---

## Dependency Change Control

**Policy:** No new external dependencies will be added without approval.

**If New Dependency Needed:**
1. Document justification (why existing libraries insufficient)
2. Evaluate alternatives
3. Assess licensing, maintenance status, performance
4. Request approval before adding

**Current Status:** No new dependencies required ✅

---

## Testing Dependencies

### Unit Tests

**Framework:** Rust built-in test framework (`#[test]`)
**No External:** No additional test libraries needed

**Test Data:**
- Synthetic ImportSession structs
- Mock SSE events
- Hardcoded test values

---

### Integration Tests

**Framework:** Rust built-in integration tests (`tests/` folder)
**External:** May need mock HTTP server (existing in wkmp-ai tests)

**Test Data:**
- Sample audio files (3-5 files, small sizes)
- Mock API responses (AcoustID, MusicBrainz)

---

### Manual Tests

**Browser:**
- Chrome 90+ (primary)
- Firefox 88+ (secondary)
- Safari 14+ (tertiary)

**Tools:**
- Browser DevTools (inspect elements, network tab)
- Responsive design mode (mobile testing)

**Test Data:**
- Real audio library (5000+ files recommended for stress testing)

---

## Dependency Resolution Strategy

**For Build Failures:**
1. Check Cargo.lock for version conflicts
2. Run `cargo update` if needed
3. Verify Rust toolchain version (stable channel)

**For Runtime Failures:**
1. Check browser console for JavaScript errors
2. Verify SSE connection established (network tab)
3. Check server logs for event broadcasting

**For Test Failures:**
1. Verify test data present
2. Check mock setups correct
3. Run single test in isolation for debugging

---

**Document Version:** 1.0
**Last Updated:** 2025-10-30
**Phase:** 1 - Dependencies Identification
