# PLAN011: Import Progress UI Enhancement - Requirements Index

**Source:** wip/SPEC_import_progress_ui_enhancement.md
**Total Requirements:** 9 (6 Functional, 3 Non-Functional)
**Date:** 2025-10-30

---

## Requirements Summary

| Req ID | Type | Priority | Brief Description | Line # |
|--------|------|----------|-------------------|--------|
| REQ-AIA-UI-001 | Functional | P0 (MUST) | Workflow Checklist Display | 250-253 |
| REQ-AIA-UI-002 | Functional | P0 (MUST) | Active Phase Progress | 255-258 |
| REQ-AIA-UI-003 | Functional | P0 (MUST) | Sub-Task Status Display | 260-264 |
| REQ-AIA-UI-004 | Functional | P0 (MUST) | Current File Display | 266-269 |
| REQ-AIA-UI-005 | Functional | P0 (MUST) | Time Estimates | 271-274 |
| REQ-AIA-UI-006 | Functional | P0 (MUST) | Error Visibility | 276-280 |
| REQ-AIA-UI-NF-001 | Non-Functional | P0 (MUST) | Performance | 284-287 |
| REQ-AIA-UI-NF-002 | Non-Functional | P0 (MUST) | Usability | 289-292 |
| REQ-AIA-UI-NF-003 | Non-Functional | P0 (MUST) | Maintainability | 294-297 |

---

## Detailed Requirements

### REQ-AIA-UI-001: Workflow Checklist Display

**Type:** Functional
**Priority:** P0 (MUST)
**Source:** SPEC line 250-253

**Requirements:**
- UI MUST display all 6 workflow phases in order
- Each phase MUST show status indicator (pending/active/complete/failed)
- Completed phases MUST show summary (e.g., "5709 files found")

**Workflow Phases:**
1. SCANNING - File discovery
2. EXTRACTING - Metadata extraction
3. FINGERPRINTING - Audio fingerprinting & identification
4. SEGMENTING - Passage detection
5. ANALYZING - Amplitude analysis
6. FLAVORING - Musical flavor extraction

**Status Indicators:**
- ○ Pending (not started)
- ⟳ In Progress (currently running)
- ✓ Completed (success)
- ✗ Failed (critical error)
- ⚠ Completed with Warnings (partial success)

---

### REQ-AIA-UI-002: Active Phase Progress

**Type:** Functional
**Priority:** P0 (MUST)
**Source:** SPEC line 255-258

**Requirements:**
- UI MUST show detailed progress for current phase
- Progress MUST include: count (N/M), percentage, progress bar
- Progress MUST update in real-time as SSE events arrive

**Example:** "Progress: 1420 / 5709 files (24.9%)" with visual progress bar

---

### REQ-AIA-UI-003: Sub-Task Status Display

**Type:** Functional
**Priority:** P0 (MUST)
**Source:** SPEC line 260-264

**Requirements:**
- UI MUST show success/failure counts for sub-tasks
- Sub-tasks MUST include: Chromaprint, AcoustID, MusicBrainz (Fingerprinting phase)
- Sub-tasks MUST show percentage success rate
- Color coding MUST indicate health:
  - Green: >95% success
  - Yellow: 85-95% success
  - Red: <85% success

**Example:**
```
Sub-Task Status:
  Chromaprint:    1420 generated, 15 failed (98.9% success) 🟢
  AcoustID:       1205 found, 215 not found (84.9% success) 🟡
  MusicBrainz:    1180 found, 25 failed (98.0% success) 🟢
```

---

### REQ-AIA-UI-004: Current File Display

**Type:** Functional
**Priority:** P0 (MUST)
**Source:** SPEC line 266-269

**Requirements:**
- UI MUST show the current file being processed
- Filename MUST be truncated if too long (show last N characters)
- Current file MUST update with each progress event

**Example:** "Currently Processing: /home/user/Music/Albums/Artist/Song.mp3"

---

### REQ-AIA-UI-005: Time Estimates

**Type:** Functional
**Priority:** P0 (MUST)
**Source:** SPEC line 271-274

**Requirements:**
- UI MUST show elapsed time
- UI SHOULD show estimated remaining time when available
- Time format: "Xm Ys" (e.g., "8m 32s")

**Example:** "Elapsed: 8m 32s | Estimated Remaining: 24m 15s"

---

### REQ-AIA-UI-006: Error Visibility

**Type:** Functional
**Priority:** P0 (MUST)
**Source:** SPEC line 276-280

**Requirements:**
- UI MUST provide access to detailed error list
- UI SHOULD show error count inline (e.g., "15 errors")
- Error list MUST show: filename, error type, error message
- Error list MUST be accessible via "[View Errors]" button

**Error List Format:**
- Filename with full path
- Error type (e.g., "Chromaprint generation failed")
- Detailed error message
- Timestamp (optional)

---

### REQ-AIA-UI-NF-001: Performance

**Type:** Non-Functional
**Priority:** P0 (MUST)
**Source:** SPEC line 284-287

**Requirements:**
- UI updates MUST NOT cause visible lag or jank
- Progress bar animations MUST be smooth (60fps)
- SSE event handling MUST be throttled if needed (max 10 updates/sec)

**Performance Targets:**
- UI update latency: <100ms
- Progress bar animation: 60fps (16.67ms per frame)
- SSE event processing: Max 10 events/sec displayed (throttle if backend sends faster)

---

### REQ-AIA-UI-NF-002: Usability

**Type:** Non-Functional
**Priority:** P0 (MUST)
**Source:** SPEC line 289-292

**Requirements:**
- UI MUST be readable on mobile screens (320px+ width)
- Critical information MUST be visible without scrolling on desktop
- Color indicators MUST have text labels (accessibility)

**Accessibility:**
- Color indicators supplemented with icons/text (not color-only)
- Mobile-responsive layout (320px minimum width)
- Desktop layout fits standard 1920x1080 without scrolling for critical info

---

### REQ-AIA-UI-NF-003: Maintainability

**Type:** Non-Functional
**Priority:** P0 (MUST)
**Source:** SPEC line 294-297

**Requirements:**
- UI code MUST be modular (separate components for checklist, progress, sub-tasks)
- Event handling MUST be centralized (single SSE listener)
- Data model MUST support future phase additions without breaking changes

**Modularity Goals:**
- Separate JavaScript functions for each UI section
- Single SSE event listener dispatches to handlers
- Data structures extensible (adding new phase types, sub-tasks)

---

## Requirement Categories

**By Type:**
- Functional: 6 requirements (67%)
- Non-Functional: 3 requirements (33%)

**By Priority:**
- P0 (MUST): 9 requirements (100%)
- P1 (SHOULD): 0 requirements (0%)
- P2 (MAY): 0 requirements (0%)

**By Component:**
- UI Display: 6 requirements (REQ-AIA-UI-001 through REQ-AIA-UI-006)
- Quality Attributes: 3 requirements (REQ-AIA-UI-NF-001 through REQ-AIA-UI-NF-003)

---

## Dependencies

**Backend Components:**
- `wkmp-ai/src/models/import_session.rs` - Data model changes
- `wkmp-ai/src/services/workflow_orchestrator.rs` - Phase tracking logic
- `wkmp-common/src/events.rs` - SSE event structure

**Frontend Components:**
- `wkmp-ai/src/api/ui.rs` - UI HTML, CSS, JavaScript

**External Dependencies:**
- None (all changes within wkmp-ai codebase)

---

## Acceptance Criteria Count

**From SPEC Section 7:**
- Visual Acceptance: 6 criteria (AC-001 through AC-006)
- Functional Acceptance: 6 criteria (AC-007 through AC-012)
- Error Handling Acceptance: 4 criteria (AC-013 through AC-016)

**Total:** 16 acceptance criteria (will be elaborated in Phase 3)

---

## Notes

**Backward Compatibility:**
- All new SSE event fields are additive (no breaking changes)
- Old UI can ignore new fields
- New UI uses enhanced fields for better display

**Out of Scope (per SPEC Section 8.3):**
- Pause/resume import functionality
- Per-phase detailed logs (expand phase to show file-level detail)
- Export error list to CSV
- Real-time throughput graph
- Notification on completion (browser notification API)

---

**Document Version:** 1.0
**Last Updated:** 2025-10-30
**Phase:** 1 - Input Validation and Scope Definition
