# Specification Issues - Risk-Primary Decision Framework

**Plan:** PLAN004
**Date:** 2025-10-25
**Phase:** Phase 2 - Specification Completeness Verification
**Requirements Analyzed:** 25 (in batches of 5-10)

---

## Executive Summary

**Total Issues Found:** 8 issues
- **CRITICAL:** 0 (no implementation blockers)
- **HIGH:** 2 (should resolve before implementation)
- **MEDIUM:** 4 (address during implementation)
- **LOW:** 2 (minor clarifications)

**Overall Assessment:** Specification is **IMPLEMENTATION-READY** with 2 HIGH issues recommended for clarification before proceeding.

**Recommendation:** Resolve HIGH issues, proceed with implementation addressing MEDIUM/LOW issues incrementally.

---

## Issues by Severity

### CRITICAL Issues (Implementation Blockers)

**None identified.** All requirements have sufficient detail to implement.

---

### HIGH Issues (Recommended Resolution Before Implementation)

#### ISSUE-HIGH-001: "Equivalent Risk" Definition Ambiguous

**Affected Requirements:**
- REQ-RPF-060: Quality evaluated among equivalent-risk approaches
- REQ-RPF-070: Effort considered only among equivalent risk+quality approaches
- REQ-RPF-180: /plan Phase 4 uses quality as tiebreaker for equivalent risk
- REQ-RPF-190: /plan Phase 4 uses effort as final tiebreaker for equivalent risk+quality

**Issue:**
"Equivalent risk" is not quantitatively defined. Could two engineers interpret differently?
- Does "Low" risk equal "Low" risk? (Yes, clearly)
- Does "Low-Medium" risk equal "Medium-Low" risk? (Ambiguous)
- Does "Low with mitigation" equal "Low inherent"? (Ambiguous)

**Impact:**
Tiebreaker logic may not trigger when it should, or may trigger inappropriately.

**Test for Ambiguity:**
Two approaches: One has "Low inherent risk," other has "Medium risk reduced to Low via mitigation"
- Are these "equivalent" for tiebreaker purposes?

**Recommendation:**
Add clarification to CLAUDE.md Decision Framework section:

```markdown
**Equivalent Risk Definition:**
Approaches have equivalent risk when their residual risk (after mitigation)
falls in the same category:
- Low = Low (equivalent)
- Low-Medium = Low-Medium (equivalent)
- Low ≠ Low-Medium (not equivalent, choose Low)

For borderline cases (e.g., "high-end Low" vs. "low-end Low-Medium"):
- Use engineering judgment
- Document rationale in ADR
- When in doubt, choose more conservative (lower) risk
```

**Severity Justification:**
HIGH because tiebreaker logic is core to framework functionality. Without clear definition, inconsistent application likely.

---

#### ISSUE-HIGH-002: ADR Format and Location Not Specified

**Affected Requirements:**
- REQ-RPF-200: /plan Phase 4 decision documented as ADR with risk-based justification

**Issue:**
ADR (Architecture Decision Record) format and storage location not specified:
- What sections should ADR contain?
- Where are ADRs stored (docs/adr/? wip/? plan folder?)?
- What filename convention (ADR-001.md? decision_*.md?)?
- Are ADRs committed separately or part of plan output?

**Impact:**
Implementer will make ad-hoc decisions about ADR format, leading to inconsistency across decisions.

**Recommendation:**
Either:

**Option A:** Use existing ADR standard (e.g., Nygard ADR template)
```markdown
# ADR-NNN: [Decision Title]

**Status:** Accepted | Rejected | Superseded
**Date:** YYYY-MM-DD
**Deciders:** [Names]

## Context
[Problem statement]

## Decision
[Chosen approach and rationale]

## Consequences
[Positive and negative outcomes]
```

**Option B:** Create WKMP-specific ADR format aligned with risk framework

Specify storage location:
- Recommend: `wip/PLAN###_*/03_approach_selection.md` includes ADR inline
- OR: `docs/adr/ADR-###.md` for permanent record

**Severity Justification:**
HIGH because ADR is explicitly required (REQ-RPF-200) but format undefined. Risk of inconsistent documentation.

**Suggested Resolution:**
Add to scope: "ADRs will be included inline in `03_approach_selection.md` following Nygard format"

---

### MEDIUM Issues (Address During Implementation)

#### ISSUE-MED-001: CLAUDE.md Section Placement Not Fully Specified

**Affected Requirements:**
- REQ-RPF-010: Add "Decision-Making Framework - MANDATORY" section to CLAUDE.md

**Issue:**
Scope statement says "After 'Implementation Workflow - MANDATORY' section" but:
- Should it be subsection or top-level section?
- Should it use # or ## heading level?
- Should it appear before or after "Document Generation Verbosity Standards"?

**Impact:**
Minor - affects document organization but not functionality.

**Recommendation:**
Place as top-level section (`#`) after "Implementation Workflow - MANDATORY", before "Document Generation Verbosity Standards"

Resulting structure:
```
# Implementation Workflow - MANDATORY
[existing content]

# Decision-Making Framework - MANDATORY  ← NEW SECTION
[new framework content]

# Document Generation Verbosity Standards
[existing content]
```

**Severity Justification:**
MEDIUM - affects document structure but implementer can make reasonable choice.

---

#### ISSUE-MED-002: /think Examples Not Fully Specified

**Affected Requirements:**
- REQ-RPF-250: Update examples in /think and /plan commands to reflect new framework

**Issue:**
"Update examples" is vague:
- Which specific examples need updating?
- How many examples (1, 3, 5)?
- Should new examples be created or existing modified?

**Impact:**
Medium - examples are valuable for learning but not strictly required for functionality.

**Recommendation:**
- Identify 2-3 existing examples in /think command that show approach comparison
- Update those examples to use new Risk Assessment → Quality → Effort structure
- If no suitable existing examples, create 1 new example demonstrating risk-first analysis

**Severity Justification:**
MEDIUM - good documentation practice but not a blocker.

---

#### ISSUE-MED-003: Risk Probability/Impact Scales Not Defined

**Affected Requirements:**
- REQ-RPF-040: Risk assessment MUST identify failure modes with probability and impact
- REQ-RPF-220: Risk assessment template includes Failure Modes table

**Issue:**
Template shows "Probability: Low/Med/High" and "Impact: [description]" but:
- Is "Low/Medium/High" qualitative scale sufficient, or should numeric ranges be defined?
- Should Impact use same Low/Med/High scale or remain descriptive?
- How is Severity calculated from Probability × Impact?

**Example Ambiguity:**
- Approach A: Probability=Low (10%), Impact=High (project failure)
- Approach B: Probability=High (60%), Impact=Low (1 hour rework)
- Which has higher severity?

**Impact:**
Medium - qualitative scales work for most cases, but edge cases may be unclear.

**Recommendation:**
Add guidance to risk_assessment.md template:

```markdown
**Probability Scale:**
- Low: <20% chance of occurring
- Medium: 20-60% chance
- High: >60% chance

**Impact Scale:**
- Low: <4 hours rework
- Medium: 4-24 hours rework or schedule delay
- High: >24 hours rework, project goal compromise, or rewrite required

**Severity = Probability × Impact:**
- Low × Low = Low
- Low × Medium = Low-Medium
- Low × High = Medium
- Medium × Medium = Medium
- Medium × High = High
- High × High = Critical
```

**Severity Justification:**
MEDIUM - qualitative scales are sufficient for most decisions, guidance reduces ambiguity.

---

#### ISSUE-MED-004: /plan Phase 4 Process Steps Integration Unclear

**Affected Requirements:**
- REQ-RPF-150 through REQ-RPF-190: /plan Phase 4 process steps

**Issue:**
New Phase 4 process (7 steps: identify → assess → evaluate → document → rank → select → ADR) is specified, but:
- How does this integrate with existing Phase 4 "Brief" (line 357-362 in plan.md)?
- Does this replace the brief or expand it?
- Current brief says "Evaluate each (advantages, disadvantages, risks, effort)" - is this superseded?

**Impact:**
Medium - implementer needs to reconcile new detailed process with existing brief.

**Recommendation:**
REPLACE existing Phase 4 brief with new detailed process. Old brief is superseded by risk-first approach.

New structure:
```markdown
### PHASE 4: Approach Selection

**Objective:** Choose implementation approach with minimal failure risk; acknowledge effort

**Process:** [7 detailed steps from REQ-RPF-150 through REQ-RPF-200]

**Output:** 03_approach_selection.md including ADR
```

**Severity Justification:**
MEDIUM - affects /plan command implementation clarity but intent is clear.

---

### LOW Issues (Minor Clarifications)

#### ISSUE-LOW-001: Templates Directory May Not Exist

**Affected Requirements:**
- REQ-RPF-210: Create templates/risk_assessment.md template file

**Issue:**
Scope statement notes "templates/ directory exists or can be created" but doesn't specify:
- Should implementation create directory if missing?
- What permissions/structure does directory need?

**Impact:**
Low - trivial to create directory if missing.

**Recommendation:**
If templates/ directory doesn't exist, create it during implementation. No special permissions needed.

**Severity Justification:**
LOW - minor operational detail, easily resolved.

---

#### ISSUE-LOW-002: Example Count Not Specified

**Affected Requirements:**
- REQ-RPF-250: Update examples in /think and /plan commands

**Issue:**
"Update examples" doesn't specify how many:
- Minimum 1?
- Target 2-3?
- Maximum?

**Impact:**
Low - any reasonable number (1-3) is acceptable.

**Recommendation:**
Update 1-2 examples in /think, 1 example in /plan. Sufficient to demonstrate pattern.

**Severity Justification:**
LOW - quantity is flexible, quality matters more.

---

## Consistency Check

**Cross-Requirement Analysis:**

✓ **No contradictions found** among 25 requirements

✓ **Terminology consistent:**
- "Risk" used consistently (residual risk after mitigation)
- "Quality" used consistently (maintainability, test coverage, architecture)
- "Effort" used consistently (implementation time)

✓ **Priorities aligned:**
- CRITICAL requirements are truly critical (framework structure, mandatory sections)
- HIGH requirements support CRITICAL (process details, templates)
- MEDIUM requirements are nice-to-have (examples, tiebreakers)

✓ **No timing conflicts:**
- All changes are documentation-only
- No dependencies on external systems or schedule-critical milestones

✓ **Resource allocations reasonable:**
- 16-24 hours AI time (within "AI implementation time is much less limited")
- Human review time required but not excessive

---

## Testability Check

**For each requirement, can compliance be objectively verified?**

| Requirement Type | Testable? | Verification Method |
|-----------------|-----------|---------------------|
| REQ-RPF-010 (Add section to CLAUDE.md) | ✓ YES | Inspection: Section exists with specified content |
| REQ-RPF-020 (Framework priorities) | ✓ YES | Inspection: Risk listed as primary, quality secondary, effort tertiary |
| REQ-RPF-030 (Mandatory compliance) | ✓ YES | Inspection: "MUST follow" language present |
| REQ-RPF-040 (Identify failure modes) | ✓ YES | Inspection: Failure modes with probability/impact in template |
| REQ-RPF-050 through REQ-RPF-070 | ✓ YES | Inspection: Process steps documented in order |
| REQ-RPF-080 (Restructure /think) | ✓ YES | Inspection: Risk → Quality → Effort order in template |
| REQ-RPF-090 through REQ-RPF-130 | ✓ YES | Inspection: Sections present in /think template |
| REQ-RPF-140 through REQ-RPF-200 | ✓ YES | Inspection: Phase 4 updated with specified process |
| REQ-RPF-210 through REQ-RPF-240 | ✓ YES | Inspection: Template file exists with tables |
| REQ-RPF-250 (Update examples) | ⚠️ PARTIAL | Inspection: Examples exist, but "update" is subjective quality |

**Overall Testability:** 96% (24/25 requirements objectively testable, 1 requires judgment)

**Note on REQ-RPF-250:**
"Update examples" testable as: "At least 1 example in /think and 1 in /plan demonstrate risk-first framework"

---

## Dependency Validation

**Dependencies from Scope Statement:**

| Dependency | Status | Interface | Stability | Alternative |
|------------|--------|-----------|-----------|-------------|
| CLAUDE.md | ✓ Exists | File read/write | Stable | N/A |
| .claude/commands/think.md | ✓ Exists | File read/write | Stable | N/A |
| .claude/commands/plan.md | ✓ Exists | File read/write | Stable | N/A |
| templates/ directory | ⚠️ May need creation | Directory creation | N/A | Create if missing |
| PCH001_project_charter.md | ✓ Exists (read-only) | File read | Stable | N/A |

**All dependencies available.** No blockers.

---

## Auto-/think Trigger Evaluation

**Trigger Conditions:**
- 5+ Critical issues: ❌ No (0 critical issues)
- 10+ High issues: ❌ No (2 high issues)
- Unclear architecture/approach: ❌ No (approach defined by /think analysis)
- Novel/risky technical elements: ❌ No (documentation changes only)

**Result:** Auto-/think trigger **NOT activated**. Specification is sufficiently clear.

---

## Recommendations

### Immediate Actions (Before Proceeding to Phase 3)

1. **Resolve ISSUE-HIGH-001 (Equivalent Risk Definition)**
   - Add clarification to CLAUDE.md framework section
   - Define "equivalent" as same residual risk category
   - Document borderline case handling

2. **Resolve ISSUE-HIGH-002 (ADR Format)**
   - Specify ADR will be inline in `03_approach_selection.md`
   - Use Nygard ADR template format
   - Include Status, Date, Context, Decision, Consequences sections

### During Implementation (MEDIUM Issues)

3. **Address ISSUE-MED-001 through ISSUE-MED-004**
   - Document structure decisions as encountered
   - Use engineering judgment for placement details
   - Maintain consistency with existing documentation style

### Post-Implementation (LOW Issues)

4. **Address ISSUE-LOW-001 and ISSUE-LOW-002**
   - Create templates/ directory if needed
   - Add 1-2 examples demonstrating framework

---

## Decision Point

**Specification Status:** READY FOR IMPLEMENTATION with 2 HIGH clarifications recommended

**Options:**

**Option A: Proceed with Clarifications**
- Incorporate ISSUE-HIGH-001 and ISSUE-HIGH-002 resolutions into requirements
- Continue to Phase 3 (Acceptance Test Definition)
- Address MEDIUM/LOW issues during implementation

**Option B: Defer for Specification Updates**
- Request stakeholder to review and approve HIGH issue resolutions
- Update analysis document with clarifications
- Resume planning after confirmation

**Recommendation:** **Option A (Proceed with Clarifications)**

**Rationale:**
- HIGH issues are minor clarifications, not fundamental gaps
- Proposed resolutions are straightforward and low-risk
- Deferring would delay CRITICAL priority implementation unnecessarily
- Clarifications can be incorporated during Phase 3 test definition

---

## Phase 2 Verification Checklist

- [x] Every requirement analyzed for completeness (inputs, outputs, behavior, constraints, errors, dependencies)
- [x] Ambiguity check performed (equivalent risk definition flagged)
- [x] Consistency check performed (no contradictions found)
- [x] Testability check performed (96% objectively testable)
- [x] Dependency validation performed (all dependencies available)
- [x] Issues prioritized by severity (0 CRITICAL, 2 HIGH, 4 MEDIUM, 2 LOW)
- [x] Auto-/think trigger evaluated (not activated)
- [x] Recommendations provided with clear reasoning

**Phase 2 Status:** ✅ COMPLETE

**Next Phase:** Phase 3 - Acceptance Test Definition (with HIGH issue clarifications incorporated)
