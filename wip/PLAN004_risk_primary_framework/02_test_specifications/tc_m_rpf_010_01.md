# TC-M-RPF-010-01: CLAUDE.md Decision-Making Framework Section

**Test ID:** TC-M-RPF-010-01
**Test Type:** Manual Inspection
**Requirements:** REQ-RPF-010, REQ-RPF-020
**Priority:** CRITICAL
**Estimated Effort:** 5 minutes

---

## Test Specification

### Scope
Verify that CLAUDE.md contains the "Decision-Making Framework - MANDATORY" section with correct priority structure (risk → quality → effort).

### Given
- CLAUDE.md file exists in project root
- File is readable and properly formatted

### When
- Open CLAUDE.md
- Search for "Decision-Making Framework - MANDATORY" section
- Review section content

### Then
Section exists with ALL of the following elements:

1. **Section Header:**
   - Exact text: "# Decision-Making Framework - MANDATORY" (or "## Decision-Making Framework - MANDATORY")
   - Appears after "# Implementation Workflow - MANDATORY" section
   - Appears before "# Document Generation Verbosity Standards" section

2. **Mandatory Statement:**
   - Contains: "All design and implementation decisions MUST follow this framework"
   - OR similar language with "MUST" requirement

3. **Risk Assessment (Primary Criterion):**
   - Listed as item 1 or labeled "Primary"
   - Includes: "Identify failure modes"
   - Includes: "Quantify probability and impact"
   - Includes: "Evaluate residual risk after mitigation"
   - Includes: "Rank approaches by failure risk"

4. **Quality Characteristics (Secondary Criterion):**
   - Listed as item 2 or labeled "Secondary"
   - Includes: "Among approaches with equivalent risk"
   - Mentions factors: "Maintainability, test coverage, architectural alignment" (or similar)

5. **Implementation Effort (Tertiary Consideration):**
   - Listed as item 3 or labeled "Tertiary"
   - Includes: "Among approaches with equivalent risk and quality, consider effort"
   - Includes: "Effort is acknowledged but NOT a decision factor" (or similar phrasing)

6. **Rationale:**
   - References PCH001 project charter
   - Mentions quality-absolute goals (e.g., "flawless audio playback")
   - States: "Risk of failure to achieve goals outweighs implementation time" (or similar)

7. **Equivalent Risk Definition (from ISSUE-HIGH-001 resolution):**
   - Defines "equivalent risk" as same residual risk category
   - Provides Low/Medium/High category examples
   - Explains borderline case handling

### Verify

**Checklist:**
- [ ] Section exists with "MANDATORY" in title
- [ ] Section placed after "Implementation Workflow" section
- [ ] "MUST follow" language present
- [ ] Risk Assessment listed first (primary)
- [ ] Quality Characteristics listed second (secondary)
- [ ] Implementation Effort listed third (tertiary)
- [ ] Rationale references PCH001 charter
- [ ] Rationale mentions quality-absolute goals
- [ ] "Equivalent risk" defined clearly
- [ ] Section integrates smoothly with existing content (no orphaned references)

### Pass Criteria

**PASS if:**
- All 10 checklist items verified ✓
- Section content matches specification in Approach 3 (lines 386-409 of analysis)
- No ambiguities in priority ordering (risk clearly primary)

**PARTIAL if:**
- 8-9 checklist items verified ✓
- Minor formatting differences but intent clear
- Example: "Quality" instead of "Quality Characteristics" (acceptable terminology variation)

**FAIL if:**
- Section does not exist
- Section exists but priorities unclear or wrong order (e.g., effort listed before risk)
- "Equivalent risk" not defined
- <7 checklist items verified

### Fail Criteria

Section missing, priorities inverted, or insufficient detail to use framework.

---

## Test Data

**Input:** CLAUDE.md file at c:\Users\Mango Cat\Dev\McRhythm\CLAUDE.md

**Expected Output:** Section content approximately 30-50 lines including:
- Framework structure (3 prioritized criteria)
- Rationale paragraph (3-5 lines)
- Equivalent risk definition (5-10 lines)

**Reference:** wip/_deprioritize_effort_analysis_results.md lines 386-409

---

## Notes

**Why Manual Test:**
Documentation inspection test - no automated verification needed for content quality and integration.

**Relationship to Other Tests:**
- TC-M-RPF-030-01 verifies mandatory language specifically
- TC-I-RPF-001-01 verifies integration with existing CLAUDE.md content
- TC-S-RPF-001-01 verifies framework actually used in /think workflow

**Execution Timing:**
Run immediately after CLAUDE.md updated in implementation.

**Typical Issues:**
- Section placed in wrong location (before Implementation Workflow)
- Equivalent risk definition missing (ISSUE-HIGH-001 not resolved)
- Rationale doesn't reference PCH001

---

**Test Status:** Pending
**Last Updated:** 2025-10-25
