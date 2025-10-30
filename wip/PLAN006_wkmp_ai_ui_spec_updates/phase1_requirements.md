# Phase 1: Requirements - wkmp-ai UI Architecture Specification Updates

**Plan:** PLAN006_wkmp_ai_ui_spec_updates
**Date:** 2025-10-28
**Source:** `wip/_wkmp_ai_ui_architecture_clarification.md`

---

## Scope Definition

### In Scope
1. Update 5 core specification documents to clarify wkmp-ai UI architecture
2. Comprehensive review of all documentation for consistency
3. Define "on-demand" microservice pattern
4. Clarify UI ownership across all microservices
5. Ensure downstream documents align with corrected understanding

### Out of Scope
- Implementation of wkmp-ai web UI (separate plan - PLAN007)
- Code changes to wkmp-ai or wkmp-ui
- Changes to API contracts (only documentation)

---

## Problem Statement

**Issue:** Specifications contain ambiguous and contradictory statements about whether wkmp-ai provides its own web UI or relies on wkmp-ui for all user interaction.

**Impact:**
- Developer confusion about implementation requirements
- Risk of implementing wrong architecture
- Inconsistent mental models across documentation

**User Clarification:** wkmp-ai SHOULD provide its own dedicated web UI for complex one-time import operations, similar to wkmp-le pattern.

**Root Cause:** Specifications were written without explicitly choosing between two architectural options, leaving ambiguity throughout documentation.

---

## Requirements

### R1: Update SPEC024 (Audio Ingest Architecture)

**Priority:** P0 (Critical)
**Files:** `docs/SPEC024-audio_ingest_architecture.md`
**Effort:** 2-3 hours

**Current Problems:**
1. Line 42: "wkmp-ui orchestrates import workflow via wkmp-ai API" (suggests backend-only)
2. Line 332: "User review and manual adjustment UI" (mentions UI but no location)
3. No explicit statement about WHERE UI lives

**Required Changes:**

#### R1.1: Add UI Architecture Section
**Location:** After line 43 (Microservices Integration section)
**Content:**
```markdown
### UI Architecture

**[AIA-UI-010]** wkmp-ai provides its own web-based UI for import workflows:
- **Access:** User navigates to http://localhost:5723 in browser
- **Launch:** wkmp-ui provides "Import Music" button that opens wkmp-ai in new tab/window
- **Technology:** HTML/CSS/JavaScript served by wkmp-ai Axum server
- **Pattern:** Similar to wkmp-le (on-demand specialized tool with dedicated UI)
- **Routes:**
  - `/` - Import wizard home page
  - `/import-progress` - Real-time progress display with SSE
  - `/segment-editor` - Waveform editor for passage boundaries (IMPL005 Step 4)
  - `/api/*` - REST API endpoints for programmatic access

**[AIA-UI-020]** wkmp-ui integration:
- wkmp-ui checks if wkmp-ai is running (via `/health` endpoint)
- If running, "Import Music" button enabled (opens http://localhost:5723)
- If not running, button shows "Install Full Version to enable import"
- No embedded import UI in wkmp-ui (wkmp-ai owns all import UX)

**[AIA-UI-030]** After import completion:
- wkmp-ai displays "Import Complete" with link back to wkmp-ui
- User returns to wkmp-ui (http://localhost:5720) to use library
- wkmp-ui detects new files via database watch or SSE event from wkmp-ai
```

#### R1.2: Update Line 42 Clarification
**Current:**
```
**Primary Integration:** wkmp-ui orchestrates import workflow via wkmp-ai API
```

**Updated:**
```
**Primary Integration:** wkmp-ui provides launch point for wkmp-ai; wkmp-ai owns import UX
```

**Acceptance Criteria:**
- ✅ UI architecture explicitly defined
- ✅ No ambiguity about UI ownership
- ✅ Launch mechanism clear
- ✅ Pattern consistency with wkmp-le

---

### R2: Update IMPL005 (Audio File Segmentation)

**Priority:** P0 (Critical)
**Files:** `docs/IMPL005-audio_file_segmentation.md`
**Effort:** 1-2 hours

**Current Problems:**
1. Describes complex UI workflow but doesn't state WHERE UI lives
2. Lines 57-68: "user is presented with review screen" (which microservice?)

**Required Changes:**

#### R2.1: Add UI Location Section
**Location:** After line 8 (before "## 1. Overview")
**Content:**
```markdown
## UI Implementation

**[AFS-UI-010]** The segmentation workflow UI is provided by **wkmp-ai** (not wkmp-ui):
- User accesses via http://localhost:5723 in web browser
- wkmp-ai serves HTML/CSS/JavaScript for all workflow screens
- wkmp-ui does NOT embed or proxy this UI
- Pattern: Standalone web application for specialized import tasks

**[AFS-UI-020]** UI Components:
- Step 1-3: Import wizard pages (source media selection, silence detection, MusicBrainz matching)
- Step 4: Interactive segment editor (waveform display, draggable boundaries)
- Step 5: Progress display and completion summary

**See:** [Audio Ingest Architecture - UI Architecture](SPEC024-audio_ingest_architecture.md#ui-architecture)
```

#### R2.2: Update Step 4 Header
**Current:**
```
### Step 4: User Review and Manual Adjustment
```

**Updated:**
```
### Step 4: User Review and Manual Adjustment (wkmp-ai UI)
```

**Acceptance Criteria:**
- ✅ UI location explicitly stated
- ✅ References to SPEC024 for consistency
- ✅ All steps clarified as wkmp-ai UI screens

---

### R3: Update SPEC009 (UI Specification)

**Priority:** P0 (Critical)
**Files:** `docs/SPEC009-ui_specification.md`
**Effort:** 2-3 hours

**Current Problems:**
1. Lines 517-535: Describes "Passage Editor" as if it's part of wkmp-ui
2. No distinction between initial import editing (wkmp-ai) vs. post-import editing (wkmp-ui)
3. Import View (lines 500-516) unclear about whether it's in wkmp-ui or wkmp-ai

**Required Changes:**

#### R3.1: Update Import View Section
**Location:** Lines 498-516
**Current:**
```markdown
### Import View

**[UI-IMPORT-010]** Library import interface displays:
- "Select Folders" button (opens folder picker)
- List of selected folders with paths
- "Import" button to start scan
- Progress indicator during scan
```

**Updated:**
```markdown
### Import Launch (wkmp-ui)

**[UI-IMPORT-005]** wkmp-ui provides import launcher:
- "Import Music" button in library view
- Checks wkmp-ai health (http://localhost:5723/health)
- If available: Opens wkmp-ai in new browser tab/window
- If unavailable: Shows "Full version required" message

**[UI-IMPORT-006]** Full import UI provided by wkmp-ai:
- See [Audio Ingest Architecture - UI Architecture](SPEC024-audio_ingest_architecture.md#ui-architecture)
- See [Audio File Segmentation](IMPL005-audio_file_segmentation.md#ui-implementation)
- wkmp-ui does NOT embed import wizard
- User navigates to http://localhost:5723 for import operations

### Import Progress Display (wkmp-ai)

**[UI-IMPORT-010]** wkmp-ai import progress interface:
- Current file being processed
- Files processed / Total files
- Percentage complete
- Cancel button (stops import, keeps partial results)
- Real-time updates via SSE

**[UI-IMPORT-020]** Import completion (wkmp-ai):
- Summary: "Added X files, Updated Y files"
- Show any errors encountered
- "Return to WKMP" button → Opens http://localhost:5720 (wkmp-ui)
```

#### R3.2: Clarify Passage Editor Ownership
**Location:** Lines 517-535
**Add before existing content:**
```markdown
### Passage Editor

**[UI-EDIT-005]** Passage editing available in TWO contexts:

**Context A: Initial Import (wkmp-ai only):**
- During audio file segmentation workflow ([IMPL005](IMPL005-audio_file_segmentation.md) Step 4)
- Accessed via wkmp-ai UI (http://localhost:5723/segment-editor)
- Full waveform display, silence threshold tuning, MusicBrainz matching
- Drag boundaries, add/delete passages, reassign songs
- Available: Full version only

**Context B: Post-Import Editing (wkmp-ui):**
- After files imported, user can edit existing passages
- Accessed via wkmp-ui library view
- Simplified editor: adjust boundaries, edit metadata
- Limited to editing existing passages (not segmentation workflow)
- Available: Full, Lite versions

**Implementation Note:** wkmp-ai and wkmp-ui may share waveform rendering code via wkmp-common, but UIs are separate applications.
```

**Keep existing [UI-EDIT-010] etc. as shared features between both contexts**

**Acceptance Criteria:**
- ✅ Clear distinction between import UI (wkmp-ai) and post-import UI (wkmp-ui)
- ✅ Passage editor contexts explicitly defined
- ✅ No ambiguity about which microservice owns which UI

---

### R4: Update CLAUDE.md (Project Overview)

**Priority:** P0 (Critical)
**Files:** `CLAUDE.md`
**Effort:** 1-2 hours

**Current Problems:**
1. Line 246: wkmp-ai described as "integration" (sounds like backend), wkmp-le described as "interface"
2. "on-demand" pattern never defined
3. No explanation of how user accesses wkmp-ai vs. wkmp-le

**Required Changes:**

#### R4.1: Update Microservices Table
**Location:** Line 246
**Current:**
```markdown
| Audio Ingest (wkmp-ai) | 5723 | File import, MusicBrainz/AcousticBrainz integration | Full (on-demand) |
```

**Updated:**
```markdown
| Audio Ingest (wkmp-ai) | 5723 | Import wizard UI, file scanning, MusicBrainz identification | Full (on-demand) |
```

#### R4.2: Define "On-Demand" Pattern
**Location:** After line 249 (after microservices table)
**Add:**
```markdown
### On-Demand Microservices

**[ARCH-OD-010]** wkmp-ai and wkmp-le are "on-demand" specialized tools:

**Architectural Pattern:**
- Each provides its own web UI served on dedicated port
- User accesses via browser (not embedded in wkmp-ui)
- wkmp-ui provides launch points (buttons/links to open in new tab)
- Decoupled specialized UIs for complex one-time operations

**Access Method:**
- **wkmp-ai:** http://localhost:5723 (import wizard, file segmentation)
- **wkmp-le:** http://localhost:5724 (lyric editor, split-window interface)
- **wkmp-ui:** http://localhost:5720 (main playback UI, provides links to above)

**Rationale:**
- Complex workflows benefit from dedicated UI (not cluttering main playback interface)
- Specialized visualization tools (waveforms, lyric sync timing)
- Infrequent use (import once, edit lyrics occasionally)
- Independent development and deployment

**User Flow Example (Import):**
1. User opens wkmp-ui (http://localhost:5720)
2. Clicks "Import Music" in library view
3. wkmp-ui opens http://localhost:5723 in new browser tab
4. User completes import workflow in wkmp-ai UI
5. Clicks "Return to WKMP" → Back to wkmp-ui tab

**Version Availability:**
- Full version: All 5 microservices (including wkmp-ai, wkmp-le)
- Lite version: wkmp-ui shows "Import Music" disabled with "Full version required" tooltip
- Minimal version: No import or lyric editing functionality
```

**Acceptance Criteria:**
- ✅ "on-demand" pattern clearly defined
- ✅ User access flow documented
- ✅ Consistency between wkmp-ai and wkmp-le
- ✅ Version differences clear

---

### R5: Update REQ001 (Requirements)

**Priority:** P0 (Critical)
**Files:** `docs/REQ001-requirements.md`
**Effort:** 1-2 hours

**Current Problems:**
1. Line 308: "WebUI provides interface..." implies all UI in wkmp-ui
2. No table of UI ownership across microservices
3. Ambiguous use of "WebUI" term (refers to wkmp-ui specifically or web-based generally?)

**Required Changes:**

#### R5.1: Add Microservice UI Ownership Section
**Location:** After line 19 (after Architectural Note)
**Add:**
```markdown
### Microservice UI Ownership

**[REQ-UI-010]** UI is distributed across microservices:

| Microservice | UI Responsibility | Access Method | Versions |
|--------------|------------------|---------------|----------|
| **wkmp-ui** | Main playback interface, library browser, preferences, user identity | http://localhost:5720 (primary access point) | All |
| **wkmp-ai** | Import wizard, file segmentation, passage boundary editor (initial import) | http://localhost:5723 (launched from wkmp-ui) | Full |
| **wkmp-le** | Lyric editor (split-window interface for timing sync) | http://localhost:5724 (launched from wkmp-ui) | Full |
| **wkmp-ap** | No UI (backend audio engine) | - | All |
| **wkmp-pd** | No UI (backend selection algorithm) | - | Full, Lite |

**[REQ-UI-020]** "WebUI" terminology clarification:
- "WebUI" in requirements refers to **browser-based UI** (HTML/CSS/JS), NOT exclusively wkmp-ui module
- wkmp-ui is PRIMARY UI (user starts here)
- wkmp-ai provides SPECIALIZED UI for import workflows
- wkmp-le provides SPECIALIZED UI for lyric editing
- All are web-based (accessed via web browser)

**[REQ-UI-030]** UI integration pattern:
- wkmp-ui is the **orchestrator**: provides launch points for specialized tools
- On-demand tools (wkmp-ai, wkmp-le) are **autonomous**: own their complete UX
- No UI embedding: wkmp-ui opens specialized tools in new browser tabs/windows
- Return navigation: specialized tools provide "Return to WKMP" links back to wkmp-ui

**See:** [Microservices Architecture](SPEC024-audio_ingest_architecture.md#ui-architecture) for detailed UI architecture
```

#### R5.2: Clarify Line 308 Reference
**Current:**
```markdown
**[REQ-PI-080]** WebUI provides interface to input/edit lyrics associated with a passage (Full version only)
```

**Updated:**
```markdown
**[REQ-PI-080]** Lyric editing UI provided by wkmp-le microservice (Full version only)
- Accessed via http://localhost:5724 (opened from wkmp-ui library view)
- See [UI Specification - Lyric Editor](SPEC009-ui_specification.md#lyric-editor) for details
```

**Acceptance Criteria:**
- ✅ UI ownership table complete and accurate
- ✅ "WebUI" term clarified
- ✅ Integration pattern documented
- ✅ No ambiguity about which microservice provides which UI

---

## Comprehensive Documentation Review (R6)

**Priority:** P0 (Critical)
**Effort:** 4-6 hours

### R6.1: Review All Specification Documents

**Files to Review:**
```
docs/SPEC*.md (all 19 specification documents)
```

**Search Patterns:**
- "wkmp-ai" - Check context (backend vs. UI)
- "wkmp-ui" - Check if incorrectly claiming wkmp-ai UI ownership
- "import" - Verify UI ownership clear
- "WebUI" - Ensure not ambiguous
- "user interface" - Check microservice assignment
- "on-demand" - Ensure defined or referenced

**Specific Files of Concern:**
1. **SPEC007-api_design.md** - May reference wkmp-ai API vs. UI endpoints
2. **SPEC019-sse_based_developer_ui.md** - Developer UI vs. user UI confusion?
3. **SPEC020-developer_ui_design.md** - May reference wkmp-ai incorrectly

### R6.2: Review Implementation Documents

**Files to Review:**
```
docs/IMPL*.md (all implementation documents)
```

**Focus:**
- IMPL008-audio_ingest_api.md - Likely needs review
- Any references to wkmp-ai UI components
- Database schema implications (no changes expected, but verify)

### R6.3: Review Requirements Documents

**Files:**
- REQ001-requirements.md (already addressed in R5)
- REQ002-entity_definitions.md (review for UI references)

### R6.4: Review Governance Documents

**Files:**
- GOV001-document_hierarchy.md
- GOV002-requirements_enumeration.md

**Check:** No UI architecture statements (unlikely but verify)

### R6.5: Review Workflow Documents

**Files:**
- workflows/DWI001_workflow_quickstart.md
- Any workflow docs referencing wkmp-ai

**Check:** Developer guidance about wkmp-ai role

### R6.6: Create Issues Log

**Output:** `wip/PLAN006_wkmp_ai_ui_spec_updates/documentation_issues_found.md`

**Format:**
```markdown
# Documentation Issues Found

## Critical Issues (Contradicts Clarified Architecture)
- [File:Line] Description of issue
- Recommended fix

## Minor Issues (Ambiguous but Inferable)
- [File:Line] Description
- Recommended clarification

## Non-Issues (Consistent with Clarified Architecture)
- Listed for completeness
```

---

## Requirements Summary

| ID | Requirement | Files | Priority | Effort |
|----|-------------|-------|----------|--------|
| R1 | Update SPEC024 | SPEC024-audio_ingest_architecture.md | P0 | 2-3h |
| R2 | Update IMPL005 | IMPL005-audio_file_segmentation.md | P0 | 1-2h |
| R3 | Update SPEC009 | SPEC009-ui_specification.md | P0 | 2-3h |
| R4 | Update CLAUDE.md | CLAUDE.md | P0 | 1-2h |
| R5 | Update REQ001 | REQ001-requirements.md | P0 | 1-2h |
| R6 | Comprehensive review | All docs (30+ files) | P0 | 4-6h |

**Total Effort:** 12-18 hours (1.5-2.5 days)

---

## Success Criteria

**Specifications Updated (R1-R5):**
- ✅ All 5 core documents explicitly state wkmp-ai provides its own web UI
- ✅ "on-demand" pattern defined and consistent
- ✅ No contradictory statements about UI ownership
- ✅ Clear user flow from wkmp-ui → wkmp-ai → back to wkmp-ui

**Comprehensive Review (R6):**
- ✅ All documentation files reviewed
- ✅ Issues log created with findings
- ✅ Recommended fixes for all critical issues
- ✅ No downstream documents contradict clarified architecture

**Consistency:**
- ✅ Terminology consistent across all docs
- ✅ Cross-references updated
- ✅ No orphaned statements from old understanding

---

## Next Steps

**Phase 2:** Comprehensive documentation review (execute R6)
**Phase 3:** Define acceptance tests for updates
**Phase 4:** Create implementation plan for executing updates
