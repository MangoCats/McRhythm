# Phase 3: Acceptance Criteria - wkmp-ai UI Specification Updates

**Plan:** PLAN006_wkmp_ai_ui_spec_updates
**Date:** 2025-10-28

---

## Overview

This document defines acceptance criteria for specification updates to clarify wkmp-ai UI architecture. All criteria must be met before declaring the plan complete.

---

## AC1: SPEC024 Updates Complete

**File:** `docs/SPEC024-audio_ingest_architecture.md`
**Priority:** P0 (Critical)

### Acceptance Criteria:

✅ **AC1.1:** New "UI Architecture" section added after line 43
- Section header: `### UI Architecture`
- Contains [AIA-UI-010], [AIA-UI-020], [AIA-UI-030] requirements
- Explicitly states wkmp-ai provides its own web UI
- Defines access pattern (http://localhost:5723)
- Explains wkmp-ui integration (launch button)
- Describes return navigation pattern

✅ **AC1.2:** Line 38 integration table updated
- "Import wizard UI, progress display" changed to "Launch point for import wizard, progress monitoring"
- OR: Table restructured to separate UI provision from integration

✅ **AC1.3:** Line 43 clarification updated
- "orchestrates import workflow" changed to "provides launch point for wkmp-ai import workflow"
- No ambiguity about UI ownership

✅ **AC1.4:** Cross-references updated
- References to SPEC009, IMPL005, SPEC007 remain valid
- No broken links

✅ **AC1.5:** Consistency check
- No contradictory statements remain in document
- All UI references specify wkmp-ai as provider
- "on-demand" pattern referenced consistently

---

## AC2: IMPL005 Updates Complete

**File:** `docs/IMPL005-audio_file_segmentation.md`
**Priority:** P0 (Critical)

### Acceptance Criteria:

✅ **AC2.1:** New "UI Implementation" section added
- Located after line 8 (before "## 1. Overview")
- Contains [AFS-UI-010] and [AFS-UI-020] requirements
- Explicitly states UI provided by wkmp-ai
- Lists UI components (wizard pages, segment editor)
- Cross-references SPEC024

✅ **AC2.2:** Step 4 header updated
- "Step 4: User Review and Manual Adjustment" → "Step 4: User Review and Manual Adjustment (wkmp-ai UI)"
- Clarifies this is wkmp-ai UI, not wkmp-ui

✅ **AC2.3:** All step descriptions reviewed
- Each step clarified as wkmp-ai UI screen
- No ambiguity about which microservice provides UI

✅ **AC2.4:** Consistency check
- References to waveform display, draggable boundaries all attributed to wkmp-ai
- No statements suggesting wkmp-ui provides this UI

---

## AC3: SPEC009 Updates Complete

**File:** `docs/SPEC009-ui_specification.md`
**Priority:** P0 (Critical)

### Acceptance Criteria:

✅ **AC3.1:** Import Launch section added
- New [UI-IMPORT-005] requirement before existing [UI-IMPORT-010]
- Clarifies wkmp-ui provides launch button only
- States wkmp-ai provides full import UI
- Defines health check pattern

✅ **AC3.2:** Import View section restructured
- Section renamed to clarify UI ownership
- Added [UI-IMPORT-006] stating "Full import UI provided by wkmp-ai"
- Cross-references to SPEC024 and IMPL005

✅ **AC3.3:** Passage Editor contexts clarified
- New [UI-EDIT-005] added before existing content
- Clearly distinguishes Context A (wkmp-ai initial import) from Context B (wkmp-ui post-import)
- No ambiguity about which context applies when

✅ **AC3.4:** Existing requirements preserved
- [UI-EDIT-010] and following requirements remain intact
- Marked as "shared features between both contexts"
- No loss of specification detail

✅ **AC3.5:** Consistency check
- No contradictory statements
- All import UI features attributed to wkmp-ai
- Post-import editing features attributed to wkmp-ui

---

## AC4: CLAUDE.md Updates Complete

**File:** `CLAUDE.md`
**Priority:** P0 (Critical)

### Acceptance Criteria:

✅ **AC4.1:** Microservices table updated
- Line 246: wkmp-ai description changed from "File import, MusicBrainz/AcousticBrainz integration" to "Import wizard UI, file scanning, MusicBrainz identification"
- Description parallels wkmp-le ("Split-window lyric editing interface")
- Consistency between on-demand modules

✅ **AC4.2:** "On-Demand Microservices" section added
- Located after line 249 (after microservices table)
- Contains [ARCH-OD-010] requirement
- Defines "on-demand" pattern clearly
- Lists access methods for each on-demand module
- Provides user flow example
- Explains version availability

✅ **AC4.3:** Rationale documented
- Explains why specialized tools have dedicated UIs
- Justifies decoupling from main playback UI
- Establishes pattern for future specialized tools

✅ **AC4.4:** Consistency check
- No contradictory statements about wkmp-ai role
- wkmp-le pattern consistent with wkmp-ai
- Version packaging section remains accurate

---

## AC5: REQ001 Updates Complete

**File:** `docs/REQ001-requirements.md`
**Priority:** P0 (Critical)

### Acceptance Criteria:

✅ **AC5.1:** Microservice UI Ownership section added
- Located after line 19 (after Architectural Note)
- Contains [REQ-UI-010] table with all 5 microservices
- Contains [REQ-UI-020] "WebUI" terminology clarification
- Contains [REQ-UI-030] UI integration pattern description

✅ **AC5.2:** Table completeness
- All 5 microservices listed (wkmp-ui, wkmp-ai, wkmp-le, wkmp-ap, wkmp-pd)
- UI responsibility clearly stated for each
- Access method specified for each
- Version availability listed

✅ **AC5.3:** Line 308 updated
- [REQ-PI-080] clarified to specify wkmp-le (not generic "WebUI")
- Cross-reference to SPEC009 added
- No ambiguity about which module provides lyric editing UI

✅ **AC5.4:** Line 457 updated
- [REQ-NET-011] clarified to specify "wkmp-ai import UI"
- No ambiguity about which module displays internet status

✅ **AC5.5:** Cross-reference added
- Reference to SPEC024 UI architecture section
- Link from requirements to detailed design

✅ **AC5.6:** Consistency check
- All "WebUI" references reviewed and clarified
- No ambiguous module ownership statements
- Pattern consistent with other updated docs

---

## AC6: Documentation Consistency

**Priority:** P0 (Critical)

### Acceptance Criteria:

✅ **AC6.1:** No contradictions between documents
- SPEC024, IMPL005, SPEC009, CLAUDE.md, REQ001 all consistent
- UI ownership statements agree across all files
- "on-demand" pattern defined consistently

✅ **AC6.2:** Cross-references valid
- All cross-references point to correct documents/sections
- No broken links
- Section numbers/headers match

✅ **AC6.3:** Terminology consistent
- "on-demand" used consistently
- "wkmp-ui" vs. "wkmp-ai" vs. "WebUI" usage clear
- "Launch point" vs. "orchestration" vs. "provision" used correctly

✅ **AC6.4:** Patterns established
- UI ownership pattern clear for future specialized tools
- Integration pattern documented for wkmp-ui ↔ specialized tools
- Return navigation pattern consistent

---

## AC7: Ambiguous Files Clarified

**Priority:** P1 (Important)

### Acceptance Criteria:

✅ **AC7.1:** SPEC008 ambiguity resolved
- Line 36: "WebUI" clarified to "wkmp-ai WebUI (accessed from wkmp-ui)"
- No remaining ambiguous UI ownership statements

✅ **AC7.2:** All 4 ambiguous files from Phase 2 addressed
- SPEC024: ✅ Clarified
- SPEC009: ✅ Clarified
- SPEC008: ✅ Clarified
- REQ001: ✅ Clarified

---

## AC8: Supporting Evidence Preserved

**Priority:** P2 (Nice to have)

### Acceptance Criteria:

✅ **AC8.1:** IMPL003 consistency maintained
- `workflow_ui/templates/` directory reference remains
- No changes to project structure documentation (evidence preserved)

✅ **AC8.2:** SPEC007 consistency maintained
- wkmp-ai API descriptions remain accurate
- Authentication section references remain valid

✅ **AC8.3:** EXEC001 consistency maintained
- Implementation order references remain accurate
- No conflicts with updated specifications

---

## AC9: No Regression

**Priority:** P0 (Critical)

### Acceptance Criteria:

✅ **AC9.1:** No information loss
- All original specification details preserved
- Only clarifications added, nothing removed
- Technical requirements unchanged

✅ **AC9.2:** No API contract changes
- HTTP endpoints remain as specified
- SSE event formats unchanged
- Database schema references unchanged

✅ **AC9.3:** No implementation conflicts
- Existing wkmp-ai MVP implementation remains valid
- Updates don't contradict current code
- Future implementation path clear

---

## AC10: Review and Validation

**Priority:** P0 (Critical)

### Acceptance Criteria:

✅ **AC10.1:** Peer review complete
- All 5 updated files reviewed by stakeholder
- Feedback incorporated
- Sign-off obtained

✅ **AC10.2:** Downstream impact assessed
- No unintended consequences identified
- Implementation feasibility confirmed
- Effort estimates validated

✅ **AC10.3:** Documentation build successful
- All markdown files render correctly
- Links functional
- No formatting errors

---

## Success Metrics

### Quantitative Metrics

✅ **M1:** 5 files updated (SPEC024, IMPL005, SPEC009, CLAUDE.md, REQ001)
✅ **M2:** 1 file clarified (SPEC008)
✅ **M3:** 0 critical contradictions remaining
✅ **M4:** 100% of Phase 2 ambiguities addressed
✅ **M5:** 19 clean files remain unmodified (no regression)

### Qualitative Metrics

✅ **M6:** Developer can determine UI ownership for any feature in <30 seconds
✅ **M7:** No ambiguous "WebUI" references remain in core specifications
✅ **M8:** "on-demand" pattern clearly defined and usable for future tools
✅ **M9:** wkmp-ai and wkmp-le patterns consistent
✅ **M10:** Documentation supports implementation of wkmp-ai web UI (PLAN007)

---

## Completion Checklist

### Phase 1: Requirements ✅
- [x] R1: SPEC024 updates defined
- [x] R2: IMPL005 updates defined
- [x] R3: SPEC009 updates defined
- [x] R4: CLAUDE.md updates defined
- [x] R5: REQ001 updates defined
- [x] R6: Comprehensive review completed

### Phase 2: Review ✅
- [x] 23 files reviewed
- [x] 4 ambiguous files identified
- [x] 0 contradictions found
- [x] Issues log created

### Phase 3: Acceptance Criteria ✅
- [x] AC1-AC10 defined
- [x] Success metrics established
- [x] Completion checklist created

### Phase 4: Implementation (Pending)
- [ ] Detailed edit instructions created
- [ ] Before/after examples provided
- [ ] Effort estimates validated
- [ ] Execution sequence defined

---

## Sign-Off Criteria

**This plan is considered COMPLETE when:**

1. ✅ All P0 acceptance criteria met (AC1-AC6, AC9-AC10)
2. ✅ All P1 acceptance criteria met (AC7)
3. ✅ All quantitative metrics achieved (M1-M5)
4. ✅ All qualitative metrics validated (M6-M10)
5. ✅ Peer review sign-off obtained
6. ✅ No regressions introduced
7. ✅ Documentation builds successfully

**Total Acceptance Criteria:** 47 individual criteria
**Total Success Metrics:** 10 metrics

---

**END OF PHASE 3**
