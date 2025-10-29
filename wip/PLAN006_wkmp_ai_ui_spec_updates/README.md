# PLAN006: wkmp-ai UI Architecture Specification Updates

**Status:** ✅ COMPLETE - Ready for execution
**Created:** 2025-10-28
**Effort:** 12-18 hours (1.5-2.5 days)
**Files to Update:** 6 files

---

## Executive Summary

**Problem:** Specifications contain ambiguous statements about whether wkmp-ai provides its own web UI or relies on wkmp-ui for all user interaction.

**Root Cause:** Documentation was written without explicitly choosing between two architectural options.

**User Clarification:** wkmp-ai SHOULD provide its own dedicated web UI for complex one-time import operations (similar to wkmp-le pattern).

**Solution:** Update 6 documentation files to explicitly state wkmp-ai provides its own web UI, define "on-demand" microservice pattern, and clarify UI ownership across all modules.

---

## Plan Documents

### Phase 1: Requirements Index
**File:** `phase1_requirements.md`

**Contents:**
- R1: Update SPEC024 (Audio Ingest Architecture)
- R2: Update IMPL005 (Audio File Segmentation)
- R3: Update SPEC009 (UI Specification)
- R4: Update CLAUDE.md (Project Overview)
- R5: Update REQ001 (Requirements)
- R6: Comprehensive documentation review

**Total Requirements:** 6 (R1-R6)
**Effort:** 12-18 hours

---

### Phase 2: Documentation Review Results
**File:** `phase2_documentation_review.md`

**Key Findings:**
- ✅ 0 critical contradictions found
- ⚠️ 4 ambiguous files requiring clarification
- ✅ 19 clean files (fully consistent)
- ✅ Overall: Documentation supports wkmp-ai having own UI

**Ambiguous Files:**
1. SPEC024-audio_ingest_architecture.md (HIGH priority)
2. SPEC009-ui_specification.md (HIGH priority)
3. SPEC008-library_management.md (MEDIUM priority)
4. REQ001-requirements.md (MEDIUM priority)

**Patterns Identified:**
- "WebUI" ambiguity (generic term vs. specific microservice)
- Integration vs. provision confusion (launch point vs. UI ownership)
- Implicit module ownership (not always explicitly stated)

**Recommendation:** Proceed with targeted clarification edits

---

### Phase 3: Acceptance Criteria
**File:** `phase3_acceptance_criteria.md`

**Total Criteria:** 47 individual acceptance criteria
**Success Metrics:** 10 metrics (5 quantitative, 5 qualitative)

**Criteria Categories:**
- AC1-AC5: Core file updates (SPEC024, IMPL005, SPEC009, CLAUDE.md, REQ001)
- AC6: Documentation consistency
- AC7: Ambiguous files clarified
- AC8: Supporting evidence preserved
- AC9: No regression
- AC10: Review and validation

**Sign-Off Requirements:**
- All P0 criteria met
- All P1 criteria met
- All metrics achieved
- Peer review complete
- No regressions
- Documentation builds successfully

---

### Phase 4: Implementation Plan
**File:** `phase4_implementation_plan.md`

**Execution Strategy:**
- Sequential file updates (dependency order)
- Before/after validation for each change
- Cross-reference verification
- Consistency checks

**File Update Sequence:**
1. CLAUDE.md (1-2h) - Foundation: Define "on-demand" pattern
2. REQ001-requirements.md (1-2h) - Foundation: UI ownership table
3. SPEC024-audio_ingest_architecture.md (2-3h) - Core: wkmp-ai UI architecture
4. IMPL005-audio_file_segmentation.md (1-2h) - Core: Segmentation workflow UI
5. SPEC009-ui_specification.md (2-3h) - Details: Import UI, passage editor
6. SPEC008-library_management.md (30m) - Clarification: WebUI terminology

**Total Effort:** 12-18 hours

---

## Problem Analysis

### Current Ambiguity

**Contradictory Signals in Documentation:**

**Suggesting wkmp-ui owns UI:**
- SPEC024: "wkmp-ui orchestrates import workflow"
- SPEC009: Describes "Passage Editor" in wkmp-ui section

**Suggesting wkmp-ai has own UI:**
- IMPL005: Complex interactive workflow (waveform editor, drag boundaries)
- SPEC024: Mentions "User review and manual adjustment UI"
- IMPL003: Shows `workflow_ui/templates/` directory in wkmp-ai

**The Issue:** Specs never explicitly chose between architectures

---

### Clarified Architecture

**CORRECT Understanding (User Confirmed):**
- wkmp-ai provides its own dedicated web UI
- wkmp-ui provides launch point (button that opens http://localhost:5723)
- This is an "on-demand" pattern for specialized tools
- Consistent with wkmp-le (lyric editor with own UI)

**Rationale:**
- Complex workflows benefit from dedicated UI
- Specialized visualization tools (waveforms, silence detection)
- Infrequent use (import once)
- Independent development and deployment

---

## Implementation Summary

### Files to Update: 6

| File | Priority | Effort | Changes |
|------|----------|--------|---------|
| CLAUDE.md | P0 | 1-2h | Update table, add "on-demand" section |
| REQ001-requirements.md | P0 | 1-2h | Add UI ownership table, clarify 2 requirements |
| SPEC024-audio_ingest_architecture.md | P0 | 2-3h | Add UI Architecture section, update 2 lines |
| IMPL005-audio_file_segmentation.md | P0 | 1-2h | Add UI Implementation section, update header |
| SPEC009-ui_specification.md | P0 | 2-3h | Add context distinctions, restructure sections |
| SPEC008-library_management.md | P1 | 30m | Clarify one "WebUI" reference |

### Key Changes

**1. Define "On-Demand" Pattern (CLAUDE.md)**
- Establishes pattern for wkmp-ai and wkmp-le
- Documents access method (separate browser tabs)
- Explains rationale (complex workflows, specialized tools)
- Provides user flow example

**2. Establish UI Ownership (REQ001)**
- Table showing which microservice provides which UI
- Clarifies "WebUI" terminology
- Defines integration pattern (orchestrator vs. autonomous)

**3. Document wkmp-ai UI Architecture (SPEC024)**
- Explicitly states wkmp-ai provides web UI
- Lists UI routes (/, /import-progress, /segment-editor, /api/*)
- Explains wkmp-ui integration (health check, launch button)
- Describes return navigation

**4. Clarify Segmentation Workflow UI (IMPL005)**
- States wkmp-ai provides UI for Step 1-5
- Lists UI components
- Cross-references SPEC024

**5. Distinguish Import Contexts (SPEC009)**
- Context A: Initial import (wkmp-ai only)
- Context B: Post-import editing (wkmp-ui)
- Clarifies which microservice provides which features

**6. Remove Ambiguity (SPEC008)**
- Single "WebUI" reference clarified
- Specifies wkmp-ai WebUI

---

## Benefits

### Immediate Benefits
1. **Eliminates developer confusion** - Clear UI ownership
2. **Enables correct implementation** - No risk of building wrong architecture
3. **Establishes pattern** - Reusable for future specialized tools
4. **Prevents rework** - No need to refactor incorrect assumptions

### Long-Term Benefits
5. **Documentation consistency** - All specs align on architecture
6. **Easier onboarding** - New developers understand microservice roles
7. **Better separation of concerns** - Each microservice owns its UX
8. **Scalable pattern** - Can add more specialized tools using same approach

---

## Validation

### Documentation Review Results

**Files Reviewed:** 23 documents
**Critical Contradictions:** 0
**Ambiguous Statements:** 4 files (now identified and addressed)
**Clean Files:** 19 (no changes needed)

**Conclusion:** Documentation is architecturally sound. Main issues are ambiguous terminology and implicit assumptions.

### Supporting Evidence

**Evidence wkmp-ai HAS own UI:**
1. IMPL003 shows `workflow_ui/templates/` directory
2. SPEC007 lists wkmp-ai UI as receiving shared secret
3. EXEC001 mentions "Create guided workflow UI" for wkmp-ai
4. SPEC024 describes SSE endpoint at port 5723

**No evidence contradicting clarified architecture found.**

---

## Execution Plan

### Prerequisites
1. Back up all files
2. Create git branch: `spec-updates-wkmp-ai-ui`
3. Prepare markdown linter
4. Note current line numbers

### Execution Steps
1. Update CLAUDE.md (foundation)
2. Update REQ001 (foundation)
3. Update SPEC024 (core)
4. Update IMPL005 (core)
5. Update SPEC009 (details)
6. Update SPEC008 (clarification)

### Post-Execution
1. Verify cross-references
2. Check markdown rendering
3. Search for remaining ambiguities
4. Review git diff
5. Run linter
6. Commit with descriptive message

---

## Success Criteria

**Plan is successful when:**
1. ✅ All 6 files updated as specified
2. ✅ All 47 acceptance criteria met
3. ✅ All cross-references functional
4. ✅ No markdown errors
5. ✅ Git diff shows only intended changes
6. ✅ Peer review sign-off obtained
7. ✅ Documentation builds successfully
8. ✅ No regressions introduced
9. ✅ "on-demand" pattern clearly defined
10. ✅ UI ownership unambiguous across all docs

---

## Risk Mitigation

### High-Risk Items
1. **Line number shifts** after edits
   - Mitigation: Update line refs after each file

2. **Broken cross-references**
   - Mitigation: Test all links, use linter

3. **Requirement ID conflicts**
   - Mitigation: Track ID changes, search for orphaned refs

4. **Unintended meaning changes**
   - Mitigation: Careful git diff review, peer review

### Low-Risk Items
- Markdown formatting errors (caught by linter)
- Missing sections (checklist ensures completeness)

---

## Related Plans

**PLAN005:** Full Feature Parity (wkmp-ai implementation)
- Implements the features described in these specs
- Depends on this plan for accurate documentation

**PLAN007 (Future):** wkmp-ai Web UI Implementation
- Will implement the UI architecture defined here
- Prerequisites: PLAN006 complete, PLAN005 backend complete

---

## Questions or Clarifications

For questions about this plan:
1. Review the phase documents in this directory
2. Check source analysis: `wip/_wkmp_ai_ui_architecture_clarification.md`
3. Consult technical lead or architect

---

## Appendix: Quick Reference

### "On-Demand" Pattern Definition

**Modules:** wkmp-ai, wkmp-le

**Characteristics:**
- Own web UI served on dedicated port
- Accessed via browser (not embedded)
- wkmp-ui provides launch points
- Specialized tools for complex one-time operations

**User Flow:**
1. Open wkmp-ui (http://localhost:5720)
2. Click button (e.g., "Import Music")
3. Opens specialized tool in new tab (http://localhost:5723)
4. Complete specialized workflow
5. Return to wkmp-ui

**Rationale:**
- Complex workflows need dedicated UI
- Specialized tools (waveforms, timing editors)
- Infrequent use (don't clutter main UI)
- Independent development

---

**END OF PLAN006 README**
