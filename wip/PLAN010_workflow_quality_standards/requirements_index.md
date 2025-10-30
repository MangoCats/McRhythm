# Requirements Index: Workflow Quality Standards Enhancement

**Source:** wip/_attitude_adjustment_analysis_results.md (Approach 2 Recommendation)
**Plan:** PLAN010
**Date:** 2025-10-30

---

## Requirements Summary

**Total Requirements:** 12
- P0 (Critical): 4
- P1 (High): 6
- P2 (Medium): 2

---

## Requirements Table

| Req ID | Priority | Category | Brief Description | Source Line # |
|--------|----------|----------|-------------------|---------------|
| **Value 1: Anti-Sycophancy (Professional Objectivity)** |
| REQ-WQ-001 | P0 | Standards | Add "Professional Objectivity" section to CLAUDE.md | Analysis 191-264 |
| REQ-WQ-002 | P1 | Standards | Define fact vs. opinion standards | Analysis Appendix A1 |
| REQ-WQ-003 | P1 | Standards | Document respectful disagreement protocol | Analysis Appendix A1 |
| **Value 2: Anti-Laziness (Plan Execution Completion)** |
| REQ-WQ-004 | P0 | Standards | Add "Plan Execution and Completion Standards" to plan.md | Analysis 266-336 |
| REQ-WQ-005 | P1 | Enforcement | Require explicit user approval before skipping increments | Analysis 266-336 |
| REQ-WQ-006 | P1 | Reporting | Mandatory completion report with planned vs. actual comparison | Analysis 266-336 |
| REQ-WQ-007 | P1 | Standards | Define "No-Shortcut Implementation" subsection | Analysis 266-336 |
| **Value 3: Anti-Hurry (No Shortcuts)** |
| REQ-WQ-008 | P2 | Standards | Document shortcut vs. phased delivery distinction | Analysis 338-381 |
| **Value 4: Problem Transparency (Technical Debt Reporting)** |
| REQ-WQ-009 | P0 | Standards | Add "Phase 9: Post-Implementation Review" to plan.md | Analysis 383-484 |
| REQ-WQ-010 | P0 | Process | Define technical debt discovery process (7 steps) | Analysis Appendix A3 |
| REQ-WQ-011 | P1 | Reporting | Mandatory technical debt section in final reports | Analysis Appendix A3 |
| REQ-WQ-012 | P2 | Standards | Update Phase 8 plan template with technical debt section | Analysis line 40 |

---

## Requirements by Value

### Value 1: Anti-Sycophancy (Professional Objectivity)

**REQ-WQ-001: Add Professional Objectivity Section to CLAUDE.md** [P0]
- **Location:** CLAUDE.md after line 127 (after Equivalent Risk Definition)
- **Content:** ~75 lines
- **Must Include:**
  - Core principle statement
  - Fact vs. opinion standards
  - Respectful disagreement protocol
  - Bias awareness guidance
  - Evidence standards
  - 2-3 concrete examples
- **Source:** Analysis Gap 1 (lines 191-264)

**REQ-WQ-002: Define Fact vs. Opinion Standards** [P1]
- **Specifics:**
  - State facts objectively with citations
  - Label opinions/judgments clearly
  - Distinguish evidence-based conclusions from preferences
- **Source:** Analysis Appendix A1

**REQ-WQ-003: Document Respectful Disagreement Protocol** [P1]
- **Specifics:**
  - Disagree when technical correctness requires it
  - Provide clear rationale and evidence
  - Offer alternative approaches
- **Source:** Analysis Appendix A1

---

### Value 2: Anti-Laziness (Plan Execution Completion)

**REQ-WQ-004: Add Plan Execution and Completion Standards** [P0]
- **Location:** plan.md after line 727
- **Content:** ~250 lines
- **Must Include:**
  - Mandatory execution requirements (3 subsections)
  - Completion report standard
  - Examples and templates
  - Enforcement mechanisms
- **Source:** Analysis Gap 2 (lines 266-336)

**REQ-WQ-005: Require Explicit User Approval Before Skipping** [P1]
- **Specifics:**
  - No increment may be marked "skipped" without user approval
  - Before skipping: STOP, explain reason, get explicit approval
  - Document all approved skips in final report
- **Source:** Analysis Appendix A2

**REQ-WQ-006: Mandatory Completion Report Standard** [P1]
- **Specifics:**
  - Planned vs. Actual comparison table
  - Clear statement of completion or skip reason for each increment
  - No ambiguous status allowed
  - "What Was NOT Done" explicit section
- **Source:** Analysis Appendix A2

**REQ-WQ-007: Define No-Shortcut Implementation** [P1]
- **Specifics:**
  - Definition: Shortcut = omitting planned functionality to deliver faster
  - ALL planned functionality MUST be delivered OR plan revised
  - Distinction between phased delivery vs. shortcuts
- **Source:** Analysis Appendix A2

---

### Value 3: Anti-Hurry (No Shortcuts)

**REQ-WQ-008: Document Shortcut vs. Phased Delivery Distinction** [P2]
- **Specifics:**
  - Clear examples of acceptable phased delivery
  - Clear examples of unacceptable shortcuts
  - Guidance on when phased delivery is appropriate
- **Source:** Analysis Gap 3 (lines 338-381)

---

### Value 4: Problem Transparency (Technical Debt Reporting)

**REQ-WQ-009: Add Phase 9 Post-Implementation Review** [P0]
- **Location:** plan.md after Phase 8 section
- **Content:** ~300 lines
- **Must Include:**
  - Technical debt definition
  - Technical debt discovery process (MANDATORY checklist)
  - Technical debt reporting standard
  - Known problems catalog
  - Examples and guidance
- **Source:** Analysis Gap 4 (lines 383-484) - CRITICAL gap

**REQ-WQ-010: Define Technical Debt Discovery Process** [P0]
- **Specifics:** 7-step mandatory process:
  1. Code review (TODO, FIXME, HACK comments, compiler warnings)
  2. Test coverage review
  3. Quality review (duplication, complexity, documentation)
  4. Known problems catalog
  5. Error handling completeness check
  6. Performance bottleneck identification
  7. Security concern review
- **Source:** Analysis Appendix A3

**REQ-WQ-011: Mandatory Technical Debt Section in Final Reports** [P1]
- **Specifics:**
  - High/Medium/Low priority categorization
  - Each item: description, location, why deferred, effort estimate
  - Known problems section
  - Test coverage gaps section
  - "NONE FOUND" only if genuinely zero issues (rare)
- **Source:** Analysis Appendix A3

**REQ-WQ-012: Update Phase 8 Plan Template** [P2]
- **Specifics:**
  - Add mandatory technical debt section to final report template
  - ~20 lines addition to Phase 8 documentation
- **Source:** Analysis line 40 (Approach 2 description)

---

## Requirements Coverage by Document

### CLAUDE.md Changes
- REQ-WQ-001: New section "Professional Objectivity" (~75 lines)
- REQ-WQ-002: Within REQ-WQ-001
- REQ-WQ-003: Within REQ-WQ-001

**Total CLAUDE.md additions:** ~75 lines

### plan.md Changes
- REQ-WQ-004: New section "Plan Execution and Completion Standards" (~250 lines)
- REQ-WQ-005: Within REQ-WQ-004
- REQ-WQ-006: Within REQ-WQ-004
- REQ-WQ-007: Within REQ-WQ-004
- REQ-WQ-008: Within REQ-WQ-004 (shortcut examples)
- REQ-WQ-009: New section "Phase 9: Post-Implementation Review" (~300 lines)
- REQ-WQ-010: Within REQ-WQ-009
- REQ-WQ-011: Within REQ-WQ-009
- REQ-WQ-012: Update to existing Phase 8 (~50 lines)

**Total plan.md additions:** ~600 lines (250 + 300 + 50)

---

## Dependencies

**Existing Documents (Must Preserve):**
- CLAUDE.md (current 400+ lines)
- .claude/commands/plan.md (current 888 lines)

**Integration Points:**
- REQ-WQ-001 references existing Risk-First Framework (CLAUDE.md lines 61-127)
- REQ-WQ-004 references existing completion checklist (plan.md lines 697-709)
- REQ-WQ-009 references Phase 8 plan documentation (plan.md Phase 8 section)

**No External Dependencies** - Pure documentation update

---

## Constraints

1. **Preserve Existing Content:** All additions must not modify existing working standards
2. **Cross-Reference Existing Standards:** New sections must reference related existing content
3. **Size Limits:** Follow CLAUDE.md verbosity standards (20-40% reduction from comprehensive draft)
4. **Backward Compatibility:** Existing workflows must continue to work
5. **Timeline:** Estimated 15-25 hours effort (per analysis recommendation)

---

## Assumptions

1. Current CLAUDE.md Risk-First Framework is effective and should be preserved
2. Current plan.md structure is sound and should be extended, not replaced
3. Users will read new standards and apply them
4. Standards enforcement will be primarily through documentation and review
5. Technical debt reporting can be made mandatory without overwhelming users

---

## Success Metrics

**Quantitative:**
- All 12 requirements implemented
- CLAUDE.md increased by ~75 lines
- plan.md increased by ~600 lines
- Zero existing content modified (additions only)
- Zero new compiler/linter warnings

**Qualitative:**
- Standards are clear and unambiguous
- Examples provided for each standard
- Integration with existing frameworks is seamless
- User feedback indicates standards are actionable

---

## Document Status

**Phase 1 Complete:** Requirements extracted and cataloged
**Total Requirements:** 12 (4 P0, 6 P1, 2 P2)
**Requirements Source:** Approach 2 from _attitude_adjustment_analysis_results.md
**Next Phase:** Specification completeness verification
