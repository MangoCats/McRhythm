# Phase 4: Implementation Plan - wkmp-ai UI Specification Updates

**Plan:** PLAN006_wkmp_ai_ui_spec_updates
**Date:** 2025-10-28
**Total Effort:** 12-18 hours (1.5-2.5 days)
**Files to Update:** 6 files

---

## Execution Strategy

### Approach
1. **Sequential execution** - Update files in dependency order
2. **Before/after validation** - Review each change before committing
3. **Cross-reference verification** - Ensure links remain valid after updates
4. **Consistency checks** - Validate terminology after each file

### Tools Required
- Text editor with markdown support
- Markdown linter (optional but recommended)
- Git for tracking changes
- Line number references from Phase 1 & 2

---

## File Update Sequence

**Priority 1 (Foundation):**
1. CLAUDE.md - Defines "on-demand" pattern
2. REQ001-requirements.md - Establishes UI ownership table

**Priority 2 (Core Specifications):**
3. SPEC024-audio_ingest_architecture.md - Detailed wkmp-ai UI architecture
4. IMPL005-audio_file_segmentation.md - UI location for segmentation workflow

**Priority 3 (UI Details):**
5. SPEC009-ui_specification.md - Import UI and passage editor contexts

**Priority 4 (Minor Clarifications):**
6. SPEC008-library_management.md - WebUI terminology clarification

---

## File 1: CLAUDE.md

**Effort:** 1-2 hours
**Priority:** P0
**Dependencies:** None
**Acceptance Criteria:** AC4.1-AC4.4

### Update 1.1: Microservices Table (Line 246)

**Location:** Line 246 in microservices table

**BEFORE:**
```markdown
| Audio Ingest (wkmp-ai) | 5723 | File import, MusicBrainz/AcousticBrainz integration | Full (on-demand) |
```

**AFTER:**
```markdown
| Audio Ingest (wkmp-ai) | 5723 | Import wizard UI, file scanning, MusicBrainz identification | Full (on-demand) |
```

**Validation:**
- ✅ Description parallels wkmp-le pattern ("interface" terminology)
- ✅ Emphasizes UI provision
- ✅ Still mentions core functionality (scanning, MusicBrainz)

---

### Update 1.2: On-Demand Microservices Section (After Line 249)

**Location:** Insert after line 249 (after "**Communication:** HTTP REST APIs + Server-Sent Events (SSE) for real-time updates")

**BEFORE:** (nothing - new section)

**AFTER:**
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

**Validation:**
- ✅ Defines "on-demand" pattern clearly
- ✅ Establishes pattern for future specialized tools
- ✅ User flow documented
- ✅ Version differences explained

---

**File 1 Checklist:**
- [ ] Update line 246
- [ ] Insert on-demand section after line 249
- [ ] Verify markdown formatting
- [ ] Check line numbers for subsequent updates
- [ ] Validate cross-references (none in this update)

**Estimated Time:** 1-2 hours

---

## File 2: REQ001-requirements.md

**Effort:** 1-2 hours
**Priority:** P0
**Dependencies:** CLAUDE.md (references on-demand pattern)
**Acceptance Criteria:** AC5.1-AC5.6

### Update 2.1: Microservice UI Ownership Section (After Line 19)

**Location:** Insert after line 19 (after "**Architectural Note:** WKMP is implemented as a microservices architecture...")

**BEFORE:** (nothing - new section)

**AFTER:**
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

**See:** [On-Demand Microservices](../CLAUDE.md#on-demand-microservices) for detailed pattern definition
```

**Validation:**
- ✅ All 5 microservices listed
- ✅ UI responsibility clear for each
- ✅ Access methods specified
- ✅ Version availability listed
- ✅ "WebUI" term clarified
- ✅ Cross-reference to CLAUDE.md

---

### Update 2.2: Line 308 Clarification

**Location:** Line 308

**BEFORE:**
```markdown
**[REQ-PI-080]** WebUI provides interface to input/edit lyrics associated with a passage (Full version only)
```

**AFTER:**
```markdown
**[REQ-PI-080]** Lyric editing UI provided by wkmp-le microservice (Full version only)
- Accessed via http://localhost:5724 (opened from wkmp-ui library view)
- See [UI Specification - Lyric Editor](SPEC009-ui_specification.md#lyric-editor) for details
```

**Validation:**
- ✅ Specifies wkmp-le (not generic "WebUI")
- ✅ Access method documented
- ✅ Cross-reference added

---

### Update 2.3: Line 457 Clarification

**Location:** Line 457

**BEFORE:**
```markdown
- **[REQ-NET-011]** Display internet connection status in library management / import UI
```

**AFTER:**
```markdown
- **[REQ-NET-011]** Display internet connection status in wkmp-ai import UI
```

**Validation:**
- ✅ Specifies wkmp-ai
- ✅ Removes ambiguous "library management / import UI"

---

**File 2 Checklist:**
- [ ] Insert UI ownership section after line 19
- [ ] Update line 308 (REQ-PI-080)
- [ ] Update line 457 (REQ-NET-011)
- [ ] Verify table formatting
- [ ] Check cross-references to CLAUDE.md and SPEC009
- [ ] Validate line numbers for subsequent updates

**Estimated Time:** 1-2 hours

---

## File 3: SPEC024-audio_ingest_architecture.md

**Effort:** 2-3 hours
**Priority:** P0
**Dependencies:** REQ001 (references UI ownership), CLAUDE.md (on-demand pattern)
**Acceptance Criteria:** AC1.1-AC1.5

### Update 3.1: Integration Table (Line 38)

**Location:** Line 38 in Microservices Integration table

**BEFORE:**
```markdown
| **wkmp-ui** | 5720 | Import wizard UI, progress display | HTTP REST + SSE |
```

**AFTER:**
```markdown
| **wkmp-ui** | 5720 | Launch point for import wizard, progress monitoring | HTTP REST + SSE |
```

**Validation:**
- ✅ Clarifies wkmp-ui provides launch point, not UI itself
- ✅ "Monitoring" (via SSE) vs. "Display" (UI rendering)

---

### Update 3.2: Primary Integration Statement (Line 43)

**Location:** Line 43

**BEFORE:**
```markdown
**Primary Integration:** wkmp-ui orchestrates import workflow via wkmp-ai API
```

**AFTER:**
```markdown
**Primary Integration:** wkmp-ui provides launch point for wkmp-ai import workflow; wkmp-ai owns import UX
```

**Validation:**
- ✅ Clarifies separation of concerns
- ✅ Removes ambiguous "orchestrates"

---

### Update 3.3: UI Architecture Section (After Line 43)

**Location:** Insert new section after line 43 (after "**Primary Integration:**...")

**BEFORE:** (nothing - new section)

**AFTER:**
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
  - `/segment-editor` - Waveform editor for passage boundaries ([IMPL005](IMPL005-audio_file_segmentation.md) Step 4)
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

**See:** [On-Demand Microservices](../CLAUDE.md#on-demand-microservices) for architectural pattern
```

**Validation:**
- ✅ Explicitly states wkmp-ai provides web UI
- ✅ Access pattern documented
- ✅ wkmp-ui integration clarified
- ✅ Return navigation described
- ✅ Cross-references to IMPL005 and CLAUDE.md

---

**File 3 Checklist:**
- [ ] Update line 38 (integration table)
- [ ] Update line 43 (primary integration)
- [ ] Insert UI Architecture section after line 43
- [ ] Verify cross-references (IMPL005, CLAUDE.md)
- [ ] Check markdown formatting
- [ ] Validate line numbers for subsequent sections

**Estimated Time:** 2-3 hours

---

## File 4: IMPL005-audio_file_segmentation.md

**Effort:** 1-2 hours
**Priority:** P0
**Dependencies:** SPEC024 (UI architecture defined)
**Acceptance Criteria:** AC2.1-AC2.4

### Update 4.1: UI Implementation Section (After Line 8)

**Location:** Insert after line 8 (before "## 1. Overview")

**BEFORE:** (nothing - new section)

**AFTER:**
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

**Validation:**
- ✅ Explicitly states wkmp-ai provides UI
- ✅ Lists UI components
- ✅ Cross-references SPEC024

---

### Update 4.2: Step 4 Header (Line 57)

**Location:** Line 57

**BEFORE:**
```markdown
### Step 4: User Review and Manual Adjustment
```

**AFTER:**
```markdown
### Step 4: User Review and Manual Adjustment (wkmp-ai UI)
```

**Validation:**
- ✅ Clarifies this step is wkmp-ai UI screen
- ✅ Minimal change, preserves content

---

**File 4 Checklist:**
- [ ] Insert UI Implementation section after line 8
- [ ] Update line 57 (Step 4 header)
- [ ] Verify cross-reference to SPEC024
- [ ] Check markdown formatting
- [ ] Validate all steps still numbered correctly

**Estimated Time:** 1-2 hours

---

## File 5: SPEC009-ui_specification.md

**Effort:** 2-3 hours
**Priority:** P0
**Dependencies:** SPEC024 (UI architecture), IMPL005 (segmentation workflow)
**Acceptance Criteria:** AC3.1-AC3.5

### Update 5.1: Import Launch Section (Before Line 500)

**Location:** Insert new section before line 500 (before "### Import View")

**BEFORE:** (section header "### Import View")

**AFTER:**
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
```

**Validation:**
- ✅ New [UI-IMPORT-005] and [UI-IMPORT-006] requirements
- ✅ Clarifies wkmp-ui provides launch only
- ✅ Cross-references to SPEC024 and IMPL005
- ✅ Section renamed to "Import Progress Display (wkmp-ai)"

---

### Update 5.2: Rename Section Header (Line 500)

**Location:** Line 500

**BEFORE:**
```markdown
### Import View

**[UI-IMPORT-010]** Library import interface displays:
```

**AFTER:**
```markdown
**[UI-IMPORT-010]** wkmp-ai import progress interface:
```

**Validation:**
- ✅ Clarifies wkmp-ai provides this interface
- ✅ Preserves requirement ID
- ✅ Section header already changed in Update 5.1

---

### Update 5.3: Import Completion (Line 512)

**Location:** Line 512

**BEFORE:**
```markdown
**[UI-IMPORT-030]** Import completion:
- Summary: "Added X files, Updated Y files"
- Show any errors encountered
- "View Library" button
```

**AFTER:**
```markdown
**[UI-IMPORT-020]** Import completion (wkmp-ai):
- Summary: "Added X files, Updated Y files"
- Show any errors encountered
- "Return to WKMP" button → Opens http://localhost:5720 (wkmp-ui)
```

**Validation:**
- ✅ Requirement ID changed to UI-IMPORT-020 (was 030, to align with new numbering)
- ✅ Clarifies wkmp-ai provides this screen
- ✅ Return navigation specified

---

### Update 5.4: Passage Editor Contexts (Before Line 517)

**Location:** Insert before line 517 (before "### Passage Editor")

**BEFORE:** (section header "### Passage Editor")

**AFTER:**
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

**Validation:**
- ✅ New [UI-EDIT-005] requirement
- ✅ Two contexts clearly distinguished
- ✅ Cross-reference to IMPL005
- ✅ Version availability specified

---

### Update 5.5: Keep Existing Requirements (After Line 524)

**Location:** After [UI-EDIT-005] context descriptions

**BEFORE:**
```markdown
**[UI-EDIT-010]** Passage boundary editing interface:
```

**AFTER:**
```markdown
**[UI-EDIT-010]** Passage boundary editing interface (both contexts):
```

**Validation:**
- ✅ Clarifies [UI-EDIT-010] and following apply to both contexts
- ✅ Preserves all existing requirements
- ✅ Minimal change

---

**File 5 Checklist:**
- [ ] Insert Import Launch section before line 500
- [ ] Rename "Import View" to "Import Progress Display (wkmp-ai)"
- [ ] Update [UI-IMPORT-010] text
- [ ] Update [UI-IMPORT-030] to [UI-IMPORT-020] and add return navigation
- [ ] Insert Passage Editor contexts before line 517
- [ ] Add "(both contexts)" to [UI-EDIT-010]
- [ ] Verify all cross-references (SPEC024, IMPL005)
- [ ] Check requirement ID sequence

**Estimated Time:** 2-3 hours

---

## File 6: SPEC008-library_management.md

**Effort:** 30 minutes
**Priority:** P1
**Dependencies:** None (simple clarification)
**Acceptance Criteria:** AC7.1

### Update 6.1: WebUI Reference (Line 36)

**Location:** Line 36

**BEFORE:**
```markdown
**Trigger:** User initiates import via WebUI or CLI
```

**AFTER:**
```markdown
**Trigger:** User initiates import via wkmp-ai WebUI (accessed from wkmp-ui launch button) or CLI
```

**Validation:**
- ✅ Clarifies "WebUI" means wkmp-ai
- ✅ Notes access method (from wkmp-ui)
- ✅ CLI option preserved

---

**File 6 Checklist:**
- [ ] Update line 36
- [ ] Verify no other ambiguous "WebUI" references in file
- [ ] Check markdown formatting

**Estimated Time:** 30 minutes

---

## Execution Checklist

### Pre-Execution
- [ ] Back up all files to be modified
- [ ] Create git branch for updates (`git checkout -b spec-updates-wkmp-ai-ui`)
- [ ] Note starting line numbers for all planned edits
- [ ] Prepare markdown linter/validator

### Execution Order
1. [ ] File 1: CLAUDE.md (1-2 hours)
2. [ ] File 2: REQ001-requirements.md (1-2 hours)
3. [ ] File 3: SPEC024-audio_ingest_architecture.md (2-3 hours)
4. [ ] File 4: IMPL005-audio_file_segmentation.md (1-2 hours)
5. [ ] File 5: SPEC009-ui_specification.md (2-3 hours)
6. [ ] File 6: SPEC008-library_management.md (30 minutes)

### Post-Execution
- [ ] Verify all cross-references functional
- [ ] Check markdown rendering (build docs if applicable)
- [ ] Search for remaining "WebUI" ambiguities
- [ ] Search for "wkmp-ui" + "import" to verify no conflicting statements
- [ ] Review git diff for unintended changes
- [ ] Run markdown linter
- [ ] Commit changes with descriptive message

---

## Validation Process

### After Each File
1. **Read updated file in full** - Ensure changes integrate smoothly
2. **Check cross-references** - All links still valid?
3. **Verify line numbers** - Subsequent updates may need line number adjustments
4. **Test markdown rendering** - No formatting errors?

### After All Files
1. **Cross-file consistency check**
   - Search all 6 files for "wkmp-ai"
   - Verify all references consistent with clarified architecture
   - No contradictory statements

2. **Terminology audit**
   - Search for "WebUI" (should all be clarified or removed)
   - Search for "orchestrate" (should be "launch point")
   - Search for "on-demand" (should reference CLAUDE.md definition)

3. **Cross-reference validation**
   - All links point to correct documents/sections
   - All section headers match cross-references
   - No broken links

---

## Effort Summary

| File | Effort | Priority | Complexity |
|------|--------|----------|------------|
| CLAUDE.md | 1-2h | P0 | Low (new section) |
| REQ001-requirements.md | 1-2h | P0 | Medium (table + 2 edits) |
| SPEC024-audio_ingest_architecture.md | 2-3h | P0 | Medium (new section + 2 edits) |
| IMPL005-audio_file_segmentation.md | 1-2h | P0 | Low (new section + header) |
| SPEC009-ui_specification.md | 2-3h | P0 | High (multiple sections, renumbering) |
| SPEC008-library_management.md | 30m | P1 | Low (single word change) |

**Total:** 12-18 hours (1.5-2.5 days)

---

## Risk Mitigation

### Risk 1: Line Number Shifts
**Mitigation:**
- Update line numbers after each file edit
- Use text search to relocate sections if needed
- Validate before proceeding to next file

### Risk 2: Broken Cross-References
**Mitigation:**
- Test all links after updates
- Use markdown linter to detect broken links
- Create cross-reference validation checklist

### Risk 3: Requirement ID Conflicts
**Mitigation:**
- Carefully track requirement ID changes (e.g., UI-IMPORT-030 → UI-IMPORT-020)
- Search for old IDs to ensure no orphaned references
- Document ID changes in commit message

### Risk 4: Unintended Meaning Changes
**Mitigation:**
- Review git diff carefully
- Have peer review updated files
- Validate against acceptance criteria

---

## Success Criteria

**This implementation is successful when:**
1. ✅ All 6 files updated as specified
2. ✅ All 47 acceptance criteria met (Phase 3)
3. ✅ All cross-references functional
4. ✅ No markdown errors
5. ✅ Git diff shows only intended changes
6. ✅ Peer review sign-off obtained
7. ✅ Documentation builds successfully

---

**END OF PHASE 4 - IMPLEMENTATION PLAN**
