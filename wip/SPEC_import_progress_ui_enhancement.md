# Import Progress UI Enhancement Specification

**Document ID:** SPEC-AIA-UI-001
**Status:** Draft for /plan
**Created:** 2025-10-30
**Author:** Claude Code

---

## 1. Overview

### 1.1 Purpose

Enhance wkmp-ai import progress UI to provide detailed, multi-level progress visibility with checklist-style workflow tracking and sub-task success/failure indicators.

### 1.2 Current State Problems

**Problem 1: Limited Visibility**
- Current UI shows only: "State: SCANNING | Progress: 145/5709"
- No visibility into which workflow phase is active
- No indication of what phases are coming next
- No visibility into sub-task success/failure (e.g., AcoustID lookup failures)

**Problem 2: User Uncertainty**
- Long-running operations appear frozen
- Users don't know if import is progressing normally or stuck
- No indication of which phase is time-consuming
- No way to assess if external API calls are succeeding

**Problem 3: Error Opacity**
- Sub-task failures (fingerprint generation, API lookups) are invisible
- User only sees final failure, not which phase/task failed
- No way to diagnose partial successes (e.g., 90% of files fingerprinted successfully)

### 1.3 Proposed Solution

Multi-level progress UI with:
1. **Workflow Checklist:** Visual checklist showing all 6 phases with completion indicators
2. **Active Phase Progress:** Detailed progress for current phase (e.g., "Fingerprinting 145/5709")
3. **Sub-Task Indicators:** Success/failure counts for sub-tasks (e.g., "AcoustID: 120 found, 25 not found")
4. **Current File/Operation:** Show what's being processed right now
5. **Error Summary:** Inline warnings/errors without blocking progress

---

## 2. Workflow Analysis

### 2.1 Import Workflow Phases

From `wkmp-ai/src/models/import_session.rs`:

| Phase | Description | Key Sub-Tasks | Typical Duration |
|-------|-------------|---------------|------------------|
| **SCANNING** | File discovery | • Traverse directories<br>• Identify audio files<br>• Calculate file hashes<br>• Save to database | Fast (seconds) |
| **EXTRACTING** | Metadata extraction | • Read ID3/Vorbis tags<br>• Extract duration<br>• Save metadata | Medium (minutes) |
| **FINGERPRINTING** | Audio fingerprinting & ID | • Generate Chromaprint<br>• Query AcoustID<br>• Query MusicBrainz<br>• Save songs/artists/albums | Slow (10+ min) |
| **SEGMENTING** | Passage detection | • Silence detection<br>• Passage boundary creation<br>• Save passages | Medium (minutes) |
| **ANALYZING** | Amplitude analysis | • Lead-in/lead-out detection<br>• Amplitude profiling<br>• Save analysis data | Medium (minutes) |
| **FLAVORING** | Musical flavor extraction | • AcousticBrainz query<br>• Essentia analysis (fallback)<br>• Save flavor vectors | Medium (minutes) |

### 2.2 Sub-Task Success/Failure Patterns

**Fingerprinting Phase Sub-Tasks:**
- Chromaprint generation: Can fail (corrupted file, unsupported codec)
- AcoustID lookup: Can fail (network, rate limit, no match)
- MusicBrainz lookup: Can fail (network, no match, rate limit)

**Flavoring Phase Sub-Tasks:**
- AcousticBrainz lookup: Can fail (network, no data for recording)
- Essentia analysis: Can fail (installation missing, analysis error)

**Key Insight:** Sub-task failures are common and expected. UI should show:
- Success count (green)
- Failure count (yellow/red)
- Percentage for quick assessment

---

## 3. UI Design

### 3.1 Layout Mockup

```
┌─────────────────────────────────────────────────────────────────┐
│ Import Progress                                                  │
│                                                                  │
│ Workflow Checklist:                                             │
│   ✓ Scanning          (Completed - 5709 files found)           │
│   ✓ Extracting        (Completed - 5709/5709 processed)        │
│   ⟳ Fingerprinting    (In Progress - 1420/5709 processed)      │
│   ○ Segmenting        (Pending)                                 │
│   ○ Analyzing         (Pending)                                 │
│   ○ Flavoring         (Pending)                                 │
│                                                                  │
│ ─────────────────────────────────────────────────────────────── │
│                                                                  │
│ Current Phase: Fingerprinting                                   │
│                                                                  │
│ Progress: 1420 / 5709 files (24.9%)                            │
│ [████████░░░░░░░░░░░░░░░░░░░░░░░░] 24.9%                        │
│                                                                  │
│ Sub-Task Status:                                                │
│   Chromaprint:    1420 generated, 15 failed (98.9% success)    │
│   AcoustID:       1205 found, 215 not found (84.9% success)    │
│   MusicBrainz:    1180 found, 25 failed (98.0% success)        │
│                                                                  │
│ Currently Processing:                                            │
│   /home/user/Music/Albums/Artist/Song.mp3                      │
│                                                                  │
│ Elapsed: 8m 32s | Estimated Remaining: 24m 15s                 │
│                                                                  │
│ [View Errors] [Cancel Import]                                   │
└─────────────────────────────────────────────────────────────────┘
```

### 3.2 Status Indicators

**Phase Status Icons:**
- `○` Pending (not started)
- `⟳` In Progress (currently running)
- `✓` Completed (success)
- `✗` Failed (critical error)
- `⚠` Completed with Warnings (partial success)

**Color Coding:**
- Green: Success (>95% success rate)
- Yellow: Warnings (85-95% success rate)
- Red: Errors (<85% success rate)

### 3.3 Responsive Behavior

**Collapsible Sections:**
- Workflow checklist: Always visible
- Sub-task status: Collapsible (show/hide details)
- Error list: Hidden by default, shown via "[View Errors]" button

**Mobile/Small Screen:**
- Stack progress bar below phase name
- Hide "Currently Processing" filename (too long)
- Collapse sub-task status by default

---

## 4. Data Model

### 4.1 Extended ImportProgress Structure

Current structure (`wkmp-ai/src/models/import_session.rs`):
```rust
pub struct ImportProgress {
    pub current: usize,
    pub total: usize,
    pub percentage: f64,
    pub current_operation: String,
    pub elapsed_seconds: u64,
    pub estimated_remaining_seconds: Option<u64>,
}
```

**Proposed Extension:**
```rust
pub struct ImportProgress {
    // Existing fields
    pub current: usize,
    pub total: usize,
    pub percentage: f64,
    pub current_operation: String,
    pub elapsed_seconds: u64,
    pub estimated_remaining_seconds: Option<u64>,

    // NEW: Phase tracking
    pub phases: Vec<PhaseProgress>,

    // NEW: Current file being processed
    pub current_file: Option<String>,
}

pub struct PhaseProgress {
    pub phase: ImportState,           // SCANNING, EXTRACTING, etc.
    pub status: PhaseStatus,          // Pending, InProgress, Completed, Failed, CompletedWithWarnings
    pub progress_current: usize,      // Files processed in this phase
    pub progress_total: usize,        // Total files for this phase
    pub subtasks: Vec<SubTaskStatus>, // Sub-task success/failure counts
}

pub enum PhaseStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    CompletedWithWarnings,
}

pub struct SubTaskStatus {
    pub name: String,              // "Chromaprint", "AcoustID", "MusicBrainz"
    pub success_count: usize,      // How many succeeded
    pub failure_count: usize,      // How many failed
    pub skip_count: usize,         // How many were skipped
}
```

### 4.2 SSE Event Changes

**Current Event Structure:**
```rust
ImportProgressUpdate {
    session_id: Uuid,
    state: String,
    current: usize,
    total: usize,
    percentage: f32,
    current_operation: String,
    elapsed_seconds: u64,
    estimated_remaining_seconds: Option<u64>,
    timestamp: DateTime<Utc>,
}
```

**Proposed Enhancement:**
```rust
ImportProgressUpdate {
    // Existing fields
    session_id: Uuid,
    state: String,
    current: usize,
    total: usize,
    percentage: f32,
    current_operation: String,
    elapsed_seconds: u64,
    estimated_remaining_seconds: Option<u64>,
    timestamp: DateTime<Utc>,

    // NEW fields
    phases: Vec<PhaseProgress>,        // All phases with status
    current_file: Option<String>,      // Current file being processed
}
```

**Backward Compatibility:**
- All new fields are additional (not replacing existing)
- Old UI can ignore new fields
- New UI uses new fields for enhanced display

---

## 5. Requirements

### 5.1 Functional Requirements

**REQ-AIA-UI-001: Workflow Checklist Display**
- UI MUST display all 6 workflow phases in order
- Each phase MUST show status indicator (pending/active/complete/failed)
- Completed phases MUST show summary (e.g., "5709 files found")

**REQ-AIA-UI-002: Active Phase Progress**
- UI MUST show detailed progress for current phase
- Progress MUST include: count (N/M), percentage, progress bar
- Progress MUST update in real-time as SSE events arrive

**REQ-AIA-UI-003: Sub-Task Status Display**
- UI MUST show success/failure counts for sub-tasks
- Sub-tasks MUST include: Chromaprint, AcoustID, MusicBrainz (Fingerprinting phase)
- Sub-tasks MUST show percentage success rate
- Color coding MUST indicate health (green >95%, yellow 85-95%, red <85%)

**REQ-AIA-UI-004: Current File Display**
- UI MUST show the current file being processed
- Filename MUST be truncated if too long (show last N characters)
- Current file MUST update with each progress event

**REQ-AIA-UI-005: Time Estimates**
- UI MUST show elapsed time
- UI SHOULD show estimated remaining time when available
- Time format: "Xm Ys" (e.g., "8m 32s")

**REQ-AIA-UI-006: Error Visibility**
- UI MUST provide access to detailed error list
- UI SHOULD show error count inline (e.g., "15 errors")
- Error list MUST show: filename, error type, error message
- Error list MUST be accessible via "[View Errors]" button

### 5.2 Non-Functional Requirements

**REQ-AIA-UI-NF-001: Performance**
- UI updates MUST NOT cause visible lag or jank
- Progress bar animations MUST be smooth (60fps)
- SSE event handling MUST be throttled if needed (max 10 updates/sec)

**REQ-AIA-UI-NF-002: Usability**
- UI MUST be readable on mobile screens (320px+ width)
- Critical information MUST be visible without scrolling on desktop
- Color indicators MUST have text labels (accessibility)

**REQ-AIA-UI-NF-003: Maintainability**
- UI code MUST be modular (separate components for checklist, progress, sub-tasks)
- Event handling MUST be centralized (single SSE listener)
- Data model MUST support future phase additions without breaking changes

---

## 6. Technical Approach

### 6.1 Backend Changes

**File:** `wkmp-ai/src/models/import_session.rs`
- Add `PhaseProgress` struct
- Add `PhaseStatus` enum
- Add `SubTaskStatus` struct
- Extend `ImportProgress` with new fields

**File:** `wkmp-ai/src/services/workflow_orchestrator.rs`
- Initialize phase tracking in `execute_import()`
- Update phase status on transitions
- Track sub-task success/failure counts
- Include current filename in progress updates
- Broadcast enhanced progress events

**File:** `wkmp-common/src/events.rs`
- Extend `ImportProgressUpdate` event with new fields
- Maintain backward compatibility

### 6.2 Frontend Changes

**File:** `wkmp-ai/src/api/ui.rs`
- Redesign progress page HTML with new layout
- Add CSS for checklist, sub-task indicators, color coding
- Enhance JavaScript SSE handler to process new fields
- Add DOM update functions for each UI section
- Add throttling to prevent UI jank from rapid updates

### 6.3 Implementation Phases

**Phase 1: Data Model (Backend)**
- Define new structs
- Extend ImportProgress
- Update SSE event structure

**Phase 2: Tracking Logic (Backend)**
- Initialize phase tracking
- Update phase status on transitions
- Track sub-task counts
- Include current file in updates

**Phase 3: UI Structure (Frontend)**
- HTML layout for checklist
- HTML layout for sub-task status
- CSS styling and color coding

**Phase 4: Event Handling (Frontend)**
- Parse new SSE event fields
- Update checklist DOM
- Update sub-task DOM
- Update current file display

**Phase 5: Polish**
- Animations and transitions
- Mobile responsiveness
- Error list modal
- Testing and refinement

---

## 7. Acceptance Criteria

### 7.1 Visual Acceptance

**AC-001:** When import starts, workflow checklist shows all 6 phases with SCANNING in progress, others pending
**AC-002:** When SCANNING completes, SCANNING shows ✓ and summary, EXTRACTING shows ⟳
**AC-003:** During FINGERPRINTING, sub-task status shows success/failure counts that update in real-time
**AC-004:** Current file display updates continuously showing filename being processed
**AC-005:** Progress bar advances smoothly without jank or freezing
**AC-006:** Color indicators change based on success rate thresholds (green/yellow/red)

### 7.2 Functional Acceptance

**AC-007:** Progress updates arrive via SSE within 1 second of backend emit
**AC-008:** Sub-task counters accurately reflect backend tracking
**AC-009:** Estimated time remaining recalculates based on current phase progress
**AC-010:** Error list button shows count of errors and opens detailed list
**AC-011:** UI remains responsive on 320px wide screens (mobile)
**AC-012:** All phases complete successfully with ✓ indicator, followed by "Completed" status

### 7.3 Error Handling Acceptance

**AC-013:** When AcoustID lookup fails, counter increments and phase continues
**AC-014:** When phase fails critically, phase shows ✗ and import stops
**AC-015:** When 20% of files fail fingerprinting, phase shows ⚠ (completed with warnings)
**AC-016:** Error list shows clear, actionable error messages with file paths

---

## 8. Implementation Considerations

### 8.1 Performance Optimization

**SSE Event Rate:**
- Current: 1 event per file (5709 events for 5709 files)
- Problem: May overwhelm UI with rapid updates
- Solution: Throttle UI updates to max 10/sec, batch backend events

**DOM Updates:**
- Use `requestAnimationFrame` for smooth progress bar updates
- Use `textContent` instead of `innerHTML` where possible
- Minimize reflows by batching DOM writes

### 8.2 Backward Compatibility

**Event Structure:**
- Add new fields to `ImportProgressUpdate` without removing old
- Old UI ignores new fields (no breaking change)
- New UI uses enhanced fields for better display

**Database Schema:**
- No database changes required (all tracked in memory during import)
- Session serialization includes new progress structure

### 8.3 Future Enhancements

**Possible Additions (Not in Scope):**
- Pause/resume import functionality
- Per-phase detailed logs (expand phase to show file-level detail)
- Export error list to CSV
- Real-time throughput graph (files/sec over time)
- Notification on completion (browser notification API)

### 8.4 Testing Strategy

**Unit Tests:**
- PhaseProgress tracking logic
- SubTaskStatus counter increments
- SSE event serialization/deserialization

**Integration Tests:**
- Full import workflow with phase transitions
- Sub-task failure scenarios (mock API failures)
- Progress percentage calculations

**Manual Tests:**
- Visual inspection of UI during real import
- Mobile responsiveness on various screen sizes
- Error list functionality
- Browser compatibility (Chrome, Firefox, Safari)

---

## 9. Open Questions

**Q1:** Should sub-task status be collapsible to save screen space?
**A:** Recommend collapsible with default expanded for first import, collapsed for subsequent

**Q2:** How to handle rate limiting delays (AcoustID has rate limits)?
**A:** Show "Waiting for rate limit..." message, don't count as failure

**Q3:** Should error list be modal or inline?
**A:** Recommend modal (overlay) to avoid disrupting main progress view

**Q4:** Display filename as full path or just basename?
**A:** Recommend basename with tooltip showing full path on hover

---

## 10. Specification Approval

**Status:** Draft - Ready for /plan
**Next Steps:**
1. Review with stakeholder
2. Run `/plan wip/SPEC_import_progress_ui_enhancement.md` to create implementation plan
3. Implement in phases per plan
4. Test against acceptance criteria
5. Deploy to wkmp-ai

**Estimated Effort:** 12-16 hours (based on /plan analysis)

---

**Document Version:** 1.0
**Last Updated:** 2025-10-30
**Generated by:** Claude Code (ultrathink mode)
