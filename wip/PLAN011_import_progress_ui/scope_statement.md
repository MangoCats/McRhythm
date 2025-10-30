# PLAN011: Import Progress UI Enhancement - Scope Statement

**Date:** 2025-10-30
**Plan Status:** Phase 1 - Scope Definition

---

## In Scope

The following features WILL be implemented as part of this plan:

### 1. Backend Data Model Enhancements

**✅ IN SCOPE:**
- Add `PhaseProgress` struct to track individual phase state
- Add `PhaseStatus` enum (Pending, InProgress, Completed, Failed, CompletedWithWarnings)
- Add `SubTaskStatus` struct to track sub-task success/failure counts
- Extend `ImportProgress` struct with:
  - `phases: Vec<PhaseProgress>` field
  - `current_file: Option<String>` field
- Extend `ImportProgressUpdate` SSE event with new fields
- Maintain backward compatibility (additive changes only)

**Files Modified:**
- `wkmp-ai/src/models/import_session.rs`
- `wkmp-common/src/events.rs`

---

### 2. Backend Phase Tracking Logic

**✅ IN SCOPE:**
- Initialize phase tracking array on import start (6 phases)
- Update phase status on state transitions (Pending → InProgress → Completed)
- Track sub-task counters during workflow execution:
  - Chromaprint: success/failure counts
  - AcoustID: found/not found counts
  - MusicBrainz: found/failure counts
  - Essentia: success/failure counts
- Include current filename in progress broadcasts
- Update phase summaries on completion

**Files Modified:**
- `wkmp-ai/src/services/workflow_orchestrator.rs`

---

### 3. Frontend UI Complete Redesign

**✅ IN SCOPE:**
- Workflow checklist section (6 phases with status indicators)
- Active phase progress section (count, percentage, progress bar)
- Sub-task status section (success/failure counts, color coding)
- Current file display section (truncated filename)
- Time estimates section (elapsed, estimated remaining)
- Error visibility (inline count, "[View Errors]" button, error list modal)
- Mobile-responsive CSS (320px+ width support)
- Smooth progress bar animations (60fps)
- Color coding system (green >95%, yellow 85-95%, red <85%)
- Accessibility features (text labels for colors)

**Files Modified:**
- `wkmp-ai/src/api/ui.rs` (HTML, CSS, JavaScript)

---

### 4. Event Handling Enhancements

**✅ IN SCOPE:**
- Parse new SSE event fields (`phases`, `current_file`)
- Update checklist DOM elements on phase changes
- Update sub-task DOM elements on counter increments
- Update current file display on each event
- Throttle UI updates to max 10/sec (prevent jank)
- Centralized SSE event listener with dispatching

**Files Modified:**
- `wkmp-ai/src/api/ui.rs` (JavaScript section)

---

### 5. Testing

**✅ IN SCOPE:**
- Unit tests for data model changes (PhaseProgress, SubTaskStatus)
- Unit tests for SSE event serialization/deserialization
- Integration tests for phase tracking logic
- Integration tests for sub-task counter increments
- Manual UI testing (visual inspection, mobile, browser compatibility)
- Performance testing (SSE event rate, UI responsiveness)

**Test Locations:**
- `wkmp-ai/tests/` (unit and integration tests)
- Manual test plan document

---

### 6. Documentation

**✅ IN SCOPE:**
- Inline code documentation (Rust doc comments)
- JavaScript function documentation
- Implementation notes in this plan

---

## Out of Scope

The following features will NOT be implemented (explicitly excluded):

### 1. Future Enhancements (Deferred)

**❌ OUT OF SCOPE:**
- Pause/resume import functionality
- Per-phase detailed logs (expand phase to show file-level detail)
- Export error list to CSV
- Real-time throughput graph (files/sec over time)
- Notification on completion (browser notification API)
- Customizable color thresholds (hardcoded to >95%, 85-95%, <85%)
- User preference storage (checklist collapsed/expanded state)

**Reason:** These are nice-to-have features not required for core functionality. Can be added in future iterations.

---

### 2. Database Schema Changes

**❌ OUT OF SCOPE:**
- Persistent storage of phase progress (in-memory during import only)
- Historical import session analysis
- Phase timing statistics storage

**Reason:** Phase tracking is runtime state, not persisted. Database only stores final session state (COMPLETED/FAILED/CANCELLED).

---

### 3. Other Modules

**❌ OUT OF SCOPE:**
- Changes to wkmp-ap, wkmp-ui, wkmp-pd, wkmp-le
- Changes to database migrations
- Changes to shared wkmp-common utilities (except events.rs)

**Reason:** This enhancement is isolated to wkmp-ai import progress display.

---

### 4. Backend Workflow Changes

**❌ OUT OF SCOPE:**
- Changing import workflow logic (phase order, sub-task execution)
- Adding new phases beyond the existing 6
- Modifying error handling strategies
- Performance optimizations to workflow orchestrator (separate effort)

**Reason:** This plan focuses on UI visibility improvements, not backend algorithm changes.

---

## Assumptions

The following are assumed to be true:

### 1. Existing Infrastructure

**Assumption:** Existing wkmp-ai import workflow is stable and functional
- 6 phases execute in order: SCANNING → EXTRACTING → FINGERPRINTING → SEGMENTING → ANALYZING → FLAVORING
- SSE event broadcasting works reliably
- Database models support serialization/deserialization
- Phase transitions are deterministic

**Risk if False:** May need to fix underlying workflow issues before adding UI enhancements

---

### 2. Browser Compatibility

**Assumption:** Target browsers support required features
- Server-Sent Events (SSE) API
- CSS Flexbox/Grid for layout
- JavaScript ES6+ features
- requestAnimationFrame for smooth animations

**Risk if False:** May need polyfills or fallback implementations

**Mitigation:** Modern browsers (Chrome 90+, Firefox 88+, Safari 14+) support all required features. Document minimum browser versions.

---

### 3. Performance Characteristics

**Assumption:** Import workflow emits SSE events at manageable rate
- Current: 1 event per file processed
- For 5709 files: ~1-5 events/sec during active processing
- Peak burst: <50 events/sec (rare)

**Risk if False:** UI throttling may need to be more aggressive

**Mitigation:** Implement configurable throttling (currently max 10 updates/sec, can be adjusted)

---

### 4. Mobile Usage

**Assumption:** Users will primarily use desktop browsers for import
- Mobile support required for accessibility (320px+ width)
- But primary use case is desktop (1920x1080 common resolution)
- Mobile users may have limited screen real estate (acceptable)

**Risk if False:** May need more aggressive UI simplification for mobile

**Mitigation:** Responsive design adapts to screen size, critical info prioritized

---

### 5. Error Rates

**Assumption:** Sub-task failure rates are reasonable
- Chromaprint generation: <5% failure rate
- AcoustID lookup: 10-30% not found (expected)
- MusicBrainz lookup: <5% failure rate
- Essentia analysis: <10% failure rate (if installed)

**Risk if False:** UI may be dominated by error indicators (mostly red)

**Mitigation:** Color thresholds calibrated for expected failure rates

---

### 6. Development Environment

**Assumption:** Development environment has necessary tools
- Rust toolchain (stable channel)
- Cargo for building/testing
- SQLite for database
- Test audio files available

**Risk if False:** Cannot test implementation adequately

**Mitigation:** Document environment setup requirements

---

## Constraints

The following limitations apply to this implementation:

### 1. Technical Constraints

**Backward Compatibility:**
- MUST maintain compatibility with old SSE event structure
- New fields are additive only (no removals, no type changes)
- Old UI code (if any) must continue to work

**Performance:**
- UI updates MUST NOT block SSE event processing
- Progress bar animations MUST maintain 60fps
- Memory usage MUST NOT grow unbounded (phase tracking ~1-2KB per import)

**Browser Support:**
- MUST work on Chrome 90+, Firefox 88+, Safari 14+
- Should degrade gracefully on older browsers (SSE fallback)

---

### 2. Process Constraints

**No Breaking Changes:**
- Cannot modify existing API contracts (SSE event types)
- Cannot change database schema (phase tracking in-memory only)
- Cannot modify import workflow phase order

**Test Coverage:**
- MUST achieve 80%+ code coverage for new Rust code
- MUST have manual test plan for UI (automation not required)

---

### 3. Resource Constraints

**Development Time:**
- Estimated 12-16 hours total effort (per SPEC Section 10)
- Phased implementation: Data Model (2-3h) → Tracking Logic (3-4h) → UI (6-8h)

**Dependencies:**
- No external library additions allowed (use existing stack)
- No new microservices or processes

---

### 4. Timeline Constraints

**Implementation Order:**
- MUST implement backend changes before frontend changes
- Data model first, tracking logic second, UI third
- Cannot test UI without backend changes complete

**Testing:**
- MUST verify backend changes with unit tests before UI work
- Manual UI testing requires real import workflow (5000+ files recommended)

---

## Dependencies

See `dependencies_map.md` for detailed dependency analysis.

**Summary:**
- Existing code: 4 files modified
- External libraries: None added
- Test infrastructure: Existing Rust test framework
- Development tools: Standard Rust/Cargo toolchain

---

## Success Criteria

This plan is successful when:

### Functional Success

- ✅ All 9 requirements (REQ-AIA-UI-001 through REQ-AIA-UI-NF-003) implemented
- ✅ All 16 acceptance criteria (AC-001 through AC-016) verified passing
- ✅ Import workflow displays enhanced progress UI in real-time
- ✅ Sub-task success/failure visible and color-coded correctly
- ✅ Mobile-responsive UI works on 320px+ screens
- ✅ Error list accessible and displays useful information

### Quality Success

- ✅ Zero compiler warnings introduced
- ✅ All unit tests pass (80%+ code coverage)
- ✅ All integration tests pass
- ✅ Manual UI tests pass on Chrome, Firefox, Safari
- ✅ Performance requirements met (60fps animations, <100ms latency)
- ✅ Backward compatibility maintained (old SSE events still work)

### Documentation Success

- ✅ All new Rust code has doc comments
- ✅ JavaScript functions documented
- ✅ Implementation plan complete with traceability matrix

---

## Risk Summary

**Low Risk Areas:**
- Data model changes (additive, well-defined)
- SSE event structure (backward compatible)
- UI HTML/CSS structure (isolated changes)

**Medium Risk Areas:**
- Sub-task counter tracking (requires careful state management)
- UI throttling implementation (must balance responsiveness vs. performance)
- Mobile responsiveness (many screen sizes to support)

**Mitigation Strategies:**
- Incremental implementation (data model → logic → UI)
- Comprehensive testing at each phase
- Manual testing on real import workflows

**Overall Risk:** Low-Medium (well-scoped, clear requirements, no unknowns)

---

**Document Version:** 1.0
**Last Updated:** 2025-10-30
**Phase:** 1 - Scope Definition
