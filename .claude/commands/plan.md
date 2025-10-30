# /plan - Implementation Planning Workflow

## Command Signature
```
/plan [specification_document_path]
```

## Purpose

Create systematic, specification-driven implementation plans that maximize probability of meeting all requirements on first implementation attempt. The workflow produces context-window-optimized, modular plans with explicit acceptance tests and incremental implementation steps.

**Key Features:**
- 8-phase systematic planning workflow
- Specification completeness verification (catch issues before coding)
- Acceptance test definition (tests before implementation)
- Context window management (modular output, <500 lines per document)
- Automatic /think integration for complex specifications
- Results permanently recorded in project documentation

## Input Parameters

- **specification_document_path** (required): Path to specification document
  - Should contain: Requirements (SHALL/MUST statements), design specifications, or both
  - May reference: Analysis documents from /think, architecture documents, existing code

## Prerequisites

Before running /plan:
- [ ] Requirements or design specifications documented
- [ ] Related /think analysis completed (if complex/novel features)
- [ ] Stakeholder expectations clear
- [ ] Technical feasibility understood

## Workflow Execution Phases

---

### PHASE 1: Input Validation and Scope Definition

**Objective:** Understand what needs to be implemented and establish clear boundaries

**Activities:**

1. **Read and Validate Input Document**
   - Locate and read specification document
   - Confirm document is current and approved
   - Identify document type (requirements, design, mixed)

2. **Extract Requirements Inventory**
   - Identify all SHALL/MUST statements
   - Assign or verify requirement IDs (REQ-XXX format per GOV002)
   - Create compact requirements index:
     ```markdown
     | Req ID | Type | Brief Description | Line # | Priority |
     |--------|------|-------------------|--------|----------|
     | REQ-CF-010 | Functional | Sample-accurate crossfade timing | 45 | High |
     | REQ-PD-015 | Functional | Musical flavor distance calculation | 78 | High |
     ```

3. **Define Scope Boundaries**
   - **In Scope:** What WILL be implemented (explicit list)
   - **Out of Scope:** What will NOT be implemented (explicit list)
   - **Assumptions:** Explicit statements of what is assumed true
   - **Constraints:** Technical, schedule, resource limitations

4. **Catalog Dependencies**
   - Existing code/modules required (wkmp-ap, wkmp-pd, wkmp-ui, wkmp-ai, wkmp-le)
   - External libraries needed (symphonia, rubato, cpal, axum, tokio)
   - Hardware/environment requirements
   - Dependencies on other features/systems

5. **Identify References**
   - Standards cited (Rust best practices, async patterns, etc.)
   - Related documents (architecture, analysis, design)
   - Prior art or similar implementations

**Context Window Management:**
- If specification >1500 lines: Extract requirements index only, don't load full spec repeatedly
- Requirements index typically 50-300 lines (compact representation)
- Full spec referenced by line number as needed

**Outputs:**
- `requirements_index.md` - Compact table of all requirements
- `scope_statement.md` - In/out of scope, assumptions, constraints
- `dependencies_map.md` - What exists, what's needed, what's external

**Success Criteria:**
- All requirements identified and cataloged
- Scope boundaries clear and unambiguous
- No confusion about what will/won't be implemented
- Dependencies identified and status known

**User Checkpoint:** Present scope summary, confirm understanding before proceeding

---

### PHASE 2: Specification Completeness Verification

**Objective:** Identify specification gaps, ambiguities, and conflicts BEFORE planning implementation

**Activities:**

1. **Completeness Check**

   For each requirement (or batch of 5-10 requirements), verify:
   - [ ] **Inputs specified:** What data/events trigger this requirement?
   - [ ] **Outputs specified:** What are the observable results?
   - [ ] **Behavior specified:** What processing/transformations occur?
   - [ ] **Constraints specified:** Timing, accuracy, resource limits?
   - [ ] **Error cases specified:** What happens when things go wrong?
   - [ ] **Dependencies specified:** What must exist for this to work?

   **Context Window Strategy:**
   - Read requirements_index.md (full context of all requirements)
   - Process requirements in batches of 5-10
   - For each batch: Read requirement text + context (±20 lines) from source
   - Record issues found, append to issues_found.md
   - Clear context, proceed to next batch
   - Never load full specification into context

2. **Ambiguity Check**

   For each requirement, identify:
   - Vague language ("appropriate," "reasonable," "good," "fast," "efficient")
   - Unquantified requirements ("quickly," "accurately," "minimal")
   - Undefined terms (jargon without definition)
   - Multiple interpretations possible

   **Test:** Could two reasonable engineers implement this differently and both claim compliance?
   - If YES → ambiguous, needs clarification
   - If NO → unambiguous, proceed

3. **Consistency Check**

   Cross-requirement analysis using compact requirements_index:
   - Do any requirements contradict each other?
   - Do timing budgets sum to more than available time?
   - Do resource allocations exceed available resources?
   - Are interface specifications consistent across components?
   - Do priorities create impossible conflicts?

4. **Testability Check**

   For each requirement:
   - Can compliance be objectively verified?
   - What test would prove this requirement is met?
   - What test would prove this requirement is violated?
   - Are test conditions achievable (equipment, data, environment)?

   **Critical Rule:** If can't define test → requirement not testable → needs refinement

5. **Dependency Validation**

   For each dependency identified in Phase 1:
   - Does the dependency exist?
   - Is the dependency's interface documented?
   - Is the dependency stable or changing?
   - Are there alternatives if primary unavailable?

6. **Issues Prioritization**

   Classify all issues found:
   - **CRITICAL:** Blocks implementation (missing essential information)
     - Example: Core requirement missing, undefined interfaces
   - **HIGH:** High risk of implementation failure without resolution
     - Example: Ambiguous requirements, timing not quantified
   - **MEDIUM:** Could cause problems, should resolve before implementation
     - Example: Undefined error handling, missing examples
   - **LOW:** Minor issues, can address during implementation
     - Example: Formatting inconsistencies, missing minor details

**Auto-/think Trigger:**

If Phase 2 discovers:
- 5+ Critical issues, OR
- 10+ High issues, OR
- Unclear architecture/approach, OR
- Novel/risky technical elements

**Then:**
1. STOP /plan execution
2. Inform user: "Specification complexity requires deeper analysis"
3. Formulate focused /think query based on issues found
4. Execute /think automatically
5. Present /think results to user
6. **CHECKPOINT:** User reviews /think analysis and approves approach
7. Resume /plan with /think insights informing decisions

**Outputs:**
- `01_specification_issues.md` - All issues found, prioritized by severity
- Issues grouped by: type (ambiguity, missing, conflict) and affected requirement

**Decision Point:**
- If CRITICAL or 5+ HIGH issues: **STOP** - Require specification updates before continuing
- If only MEDIUM/LOW issues: Note for tracking, continue to Phase 3
- Present issues report to user for review and decision

**Success Criteria:**
- Every requirement analyzed for completeness, ambiguity, testability
- Issues clearly documented with specific resolution recommendations
- User understands specification status and approves proceeding

---

### PHASE 3: Acceptance Test Definition

**Objective:** Define explicit, executable tests that will verify each requirement is met

**Philosophy:** Tests ARE executable specifications. If we can't define the test, the requirement is ambiguous.

**Activities:**

1. **Test Type Determination**

   For each requirement, determine test types needed:
   - **Unit Tests:** Individual components/functions
   - **Integration Tests:** Component interactions (e.g., HTTP API calls between microservices)
   - **System Tests:** End-to-end scenarios (e.g., full playback pipeline with crossfading)
   - **Manual Tests:** When automation not feasible (document procedure)

2. **Unit Test Specification**

   For each requirement needing unit tests, define using BDD format:

   ```markdown
   ### REQ-CF-XXX: [Requirement Title]

   **Unit Test: TC-U-XXX-01**
   - **Test Type:** Unit Test
   - **Scope:** [Component/Function under test]
   - **Given:** [Initial conditions/setup]
   - **When:** [Action/input]
   - **Then:** [Expected result]
   - **Verify:** [Specific assertions]
   - **Pass Criteria:** [Measurable success condition]
   - **Fail Criteria:** [What constitutes failure]

   **Estimated Effort:** [X minutes to write + implement]
   ```

3. **Integration Test Specification**

   For requirements involving multiple components:

   ```markdown
   **Integration Test: TC-I-XXX-01**
   - **Test Type:** Integration Test
   - **Scope:** [Microservices/interfaces under test]
   - **Setup:** [System configuration required]
   - **Given:** [Initial system state]
   - **When:** [Cross-component interaction (HTTP/SSE)]
   - **Then:** [Expected system behavior]
   - **Verify:** [Interface contracts, data flow verified]
   - **Pass Criteria:** [Measurable success]
   ```

4. **System Test Specification**

   For end-to-end requirements:

   ```markdown
   **System Test: TC-S-XXX-01**
   - **Test Type:** System/End-to-End Test
   - **Environment:** [Hardware/software configuration]
   - **Scenario:** [User/system interaction description]
   - **Given:** [System initial state]
   - **When:** [User actions or system events]
   - **Then:** [Observable system behavior]
   - **Verify:** [End-to-end validation]
   - **Pass Criteria:** [User-visible success]
   ```

5. **Edge Case and Error Condition Tests**

   For each requirement, define tests for:
   - **Boundary conditions:** Min/max values, empty sets, null inputs
   - **Error conditions:** Invalid inputs, resource exhaustion, external failures
   - **Concurrent access:** If applicable (tokio async contexts)
   - **Performance limits:** If specified in requirements

6. **Test Data Specification**

   For each test, identify:
   - Input data required (values, files, databases)
   - Expected output data (exact values or ranges)
   - Test data generation method (manual, generated, recorded)
   - Test data storage location

7. **Traceability Matrix Creation**

   Create comprehensive table linking requirements → tests → implementation:

   ```markdown
   | Requirement | Unit Tests | Integration Tests | System Tests | Implementation File(s) | Status | Coverage |
   |-------------|------------|-------------------|--------------|------------------------|--------|----------|
   | REQ-CF-010 | TC-U-010-01, TC-U-010-02 | TC-I-010-01 | TC-S-010-01 | wkmp-ap/src/crossfade.rs | Pending | Complete |
   | REQ-PD-015 | TC-U-015-01 | TC-I-015-01 | TC-S-015-01 | wkmp-pd/src/selector.rs | Pending | Complete |
   ```

   **Traceability Matrix Requirements:**
   - **Requirement ID:** Unique identifier (REQ-XXX format per GOV002)
   - **Tests:** All tests that verify this requirement (by ID)
   - **Implementation File(s):** Source files where requirement is implemented (can be "TBD" during planning)
   - **Status:** Pending / In Progress / Complete / Verified
   - **Coverage:** Complete (all acceptance criteria tested) / Partial / Incomplete

   **Purpose:** Enables verification that:
   1. Every requirement has tests (forward traceability)
   2. Every requirement is implemented (requirement → code)
   3. Every test traces to requirement (backward traceability)
   4. No orphaned tests or code

   **Usage During Implementation:**
   - Implementer updates "Implementation File(s)" column when implementing
   - Implementer updates "Status" as work progresses
   - Reviewer verifies matrix completeness before release

**Context Window Management for Tests:**

**Option A - Inline Tests (for small projects, <20 requirements):**
- All test specifications in single document
- Organized by requirement
- Target: <1500 lines total

**Option B - Modular Tests (for larger projects, ≥20 requirements):**
- Test index (compact table: test ID, requirement, one-line description)
- Individual test specification files: `tests/tc_u_001_01.md` (~50-100 lines each)
- Increment references tests by ID, reads only needed tests
- Context window: ~200 lines per test vs. 1500 lines for all tests

**Recommendation:** Use Option B (modular) for /plan output even on smaller projects - establishes good pattern

**Outputs:**
- `02_test_specifications/` folder containing:
  - `test_index.md` - Compact table of all tests
  - `tc_u_xxx_yy.md` - Individual unit test specifications
  - `tc_i_xxx_yy.md` - Individual integration test specifications
  - `tc_s_xxx_yy.md` - Individual system test specifications
  - `traceability_matrix.md` - Requirements ↔ Tests mapping

**Success Criteria:**
- Every requirement has at least one acceptance test
- Every test is specific and executable (not "verify it works")
- Tests cover normal operation, error cases, and edge cases
- Traceability matrix shows no gaps (100% coverage)

**User Checkpoint:** Present test coverage summary, confirm comprehensive before proceeding

---

### PHASE 4: Approach Selection [Week 2 Implementation]

**Status:** Specified in analysis, to be implemented Week 2

**Objective:** Choose implementation approach with minimal failure risk; acknowledge effort

**Process:**
1. **Identify 2-3 viable approaches** that could satisfy requirements
2. **For each approach:**
   a. Perform risk assessment (see templates/risk_assessment.md):
      - Identify failure modes with probability and impact
      - Define mitigation strategies
      - Calculate residual risk after mitigation
   b. Evaluate quality characteristics:
      - Maintainability
      - Test coverage achievable
      - Architectural alignment
   c. Document effort and dependencies:
      - Implementation effort estimate
      - Required dependencies
      - Technical complexity
3. **Rank approaches by residual risk** (after mitigation) - lowest risk = highest priority
4. **Select lowest-risk approach** as recommended approach
5. **If multiple approaches have equivalent risk:**
   - Use quality characteristics as tiebreaker
   - Choose approach with best maintainability/test coverage/architecture fit
6. **If multiple approaches have equivalent risk AND quality:**
   - Use effort as final tiebreaker
   - Choose lower-effort approach among equivalent-risk/quality options
7. **Document decision as ADR** with explicit risk-based justification:
   - Use Nygard ADR template format (Status, Date, Context, Decision, Consequences)
   - Justification MUST reference risk assessment
   - Include inline in `03_approach_selection.md`

**Output:** `03_approach_selection.md` including risk assessments and ADR

**Example Decision Justification:**
```
RECOMMENDATION: Approach B (Incremental Migration)

RISK-BASED JUSTIFICATION:
Approach B has lowest residual risk (Low) after mitigation:
- Failure modes identified: data loss (Low prob), partial migration (Low impact)
- Mitigations: Automated backups, validation checkpoints, rollback capability
- Residual risk: Low

Approach A (Big Bang Migration) has higher risk (Medium):
- Single point of failure, difficult rollback, higher impact if issues occur

Quality characteristics equivalent between A and B (both High maintainability).

Effort: Approach B requires 40 hours vs. Approach A's 25 hours.
The 15-hour effort differential is acceptable given risk reduction from Medium to Low.

Per CLAUDE.md Decision-Making Framework: Risk (primary) → Quality (secondary) → Effort (tertiary).
```

---

### PHASE 5: Implementation Breakdown [Week 2 Implementation]

**Status:** Specified in analysis, to be implemented Week 2

**Objective:** Decompose implementation into small, manageable, verifiable increments

**Brief:**
- Break system into logical components
- Define implementation increments (target 2-4 hours each)
- Sequence by dependency and risk
- Define checkpoints every 5-10 increments
- Each increment: objectives, deliverables, tests, verification

**Output:** `04_increments/` folder with individual increment files

---

### PHASE 6: Effort and Schedule Estimation [Week 3 Implementation]

**Status:** Specified in analysis, to be implemented Week 3

**Objective:** Provide realistic time estimates and identify resource needs

**Output:** `05_estimates.md`

---

### PHASE 7: Risk Assessment and Mitigation Planning [Week 3 Implementation]

**Status:** Specified in analysis, to be implemented Week 3

**Objective:** Identify risks to successful implementation and plan mitigations

**Output:** `06_risks.md`

---

### PHASE 8: Plan Documentation and Approval [Week 3 Implementation]

**Status:** Specified in analysis, to be implemented Week 3

**Objective:** Generate summary, consolidate full plan, obtain approval

**Output:** `00_PLAN_SUMMARY.md` and `FULL_PLAN.md`

---

#### 00_PLAN_SUMMARY.md Template

**Required Sections (<500 lines total):**

```markdown
# PLAN###: [Feature Name] - PLAN SUMMARY

**Status:** [Ready for Implementation / In Progress / Complete]
**Created:** [Date]
**Specification Source:** [Path to spec document]
**Plan Location:** `wip/PLAN###_[feature_name]/`

---

## READ THIS FIRST

[Brief paragraph: What this document provides, how to use it]

**For Implementation:**
- Read this summary
- Review detailed requirements: `requirements_index.md`
- Review test specifications: `02_test_specifications/test_index.md`
- Follow traceability matrix: `02_test_specifications/traceability_matrix.md`

**Context Window Budget:** [Estimated line counts for implementation]

---

## Executive Summary

### Problem Being Solved

[1-3 paragraphs: What problems are we addressing? Why is this work needed?]

### Solution Approach

[1-2 paragraphs: High-level overview of solution chosen]

### Implementation Status

**Phases 1-3 Complete:**
- ✅ Phase 1: Scope Definition - [N] requirements extracted
- ✅ Phase 2: Specification Verification - [N] Critical issues, [N] Medium, [N] Low
- ✅ Phase 3: Test Definition - [N] tests defined, [X]% coverage

**Phases 4-8 Status:** [Complete/Pending/N/A]

---

## Requirements Summary

**Total Requirements:** [N] ([N] P0, [N] P1, [N] P2)

[Table or list of key requirements]

**Full Requirements:** See `requirements_index.md`

---

## Scope

### ✅ In Scope

[Bulleted list of what WILL be implemented]

### ❌ Out of Scope

[Bulleted list of what will NOT be implemented]

**Full Scope:** See `scope_statement.md`

---

## Specification Issues

**Phase 2 Analysis Results:**
- **CRITICAL Issues:** [N]
- **HIGH Issues:** [N]
- **MEDIUM Issues:** [N]
- **LOW Issues:** [N]

**Decision:** [STOP - Critical issues / PROCEED - No blockers]

**Full Analysis:** See `01_specification_issues.md`

---

## Implementation Roadmap

[For each increment:]

### Increment N: [Name]
**Objective:** [What this increment achieves]
**Effort:** [X-Y hours]
**Deliverables:**
- [Deliverable 1]
- [Deliverable 2]
**Tests:** [Test IDs]
**Success Criteria:** [How to know it's done]

[Repeat for all increments]

**Total Estimated Effort:** [X-Y hours]

---

## Test Coverage Summary

**Total Tests:** [N] ([N] unit, [N] integration, [N] system, [N] manual)
**Coverage:** [X]% - All [N] requirements have acceptance tests

**Traceability:** Complete matrix in `02_test_specifications/traceability_matrix.md`

---

## Risk Assessment

**Residual Risk:** [Low / Low-Medium / Medium / etc.]

**Top Risks:**
1. [Risk 1] - [Mitigation]
2. [Risk 2] - [Mitigation]

**Full Risk Analysis:** See `06_risks.md` (if Phase 7 complete)

---

## Technical Debt and Known Issues

**MANDATORY: This section documents technical debt discovered during implementation (Phase 9).**

**If plan NOT yet implemented:**
```markdown
### Technical Debt and Known Issues

**Status:** Not applicable - plan not yet implemented

**Note:** After implementation completes, Phase 9 (Post-Implementation Review) will systematically discover and document:
- Known bugs and limitations
- Test coverage gaps
- Performance concerns
- Security issues
- Deferred requirements

See Phase 9 section of plan.md for 7-step technical debt discovery process.
```

**If plan implementation COMPLETE:**
```markdown
### Technical Debt and Known Issues

**Phase 9 Review Complete:** [Date]
**Full Report:** See `phase9_technical_debt_report.md`

**Executive Summary:**
- Total Items: [N] ([N] critical, [N] high, [N] medium, [N] low)
- Estimated Remediation: [X-Y] hours
- Immediate Action Required: [Yes/No]

**Critical Items Requiring Immediate Attention:**
1. [CRITICAL-001 brief description]
2. [CRITICAL-002 brief description]

**Test Coverage:** [X]% (Target: 80%+)
**Performance:** [N] known bottlenecks documented
**Security:** [N] concerns identified

**Recommendations:**
- Immediate: [Action items for critical issues]
- Next Release: [Action items for high-priority issues]

**Note:** Do NOT mark plan complete or archive until Phase 9 technical debt report is generated and attached.
```

---

## Success Metrics

**Quantitative:**
- ✅ [Metric 1]
- ✅ [Metric 2]

**Qualitative:**
- ✅ [Criterion 1]
- ✅ [Criterion 2]

---

## Dependencies

**Existing Documents (Read-Only):**
- [Document 1] ([Current lines])
- [Document 2] ([Current lines])

**Integration Points:**
- [Where changes will be made]

**No External Dependencies** OR **External Dependencies:**
- [List any external dependencies]

---

## Constraints

**Technical:**
- [Constraint 1]
- [Constraint 2]

**Process:**
- [Constraint 1]
- [Constraint 2]

**Timeline:**
- [Estimated duration]
- [Expected timeframe]

---

## Next Steps

### Immediate (Ready Now)
1. [Step 1]
2. [Step 2]

### Implementation Sequence
1. [Increment 1 description]
2. [Increment 2 description]
...

### After Implementation
1. Execute Phase 9: Post-Implementation Review (MANDATORY)
2. Generate technical debt report
3. Run all [N] tests
4. Verify traceability matrix 100% complete
5. Create final implementation report
6. Archive plan using `/archive-plan PLAN###`

---

## Document Navigation

**Start Here:** This file (00_PLAN_SUMMARY.md)

**Detailed Planning:**
- `requirements_index.md` - All requirements with priorities
- `scope_statement.md` - In/out scope, assumptions, constraints
- `01_specification_issues.md` - Phase 2 analysis

**Test Specifications:**
- `02_test_specifications/test_index.md` - All tests quick reference
- `02_test_specifications/traceability_matrix.md` - Requirements ↔ Tests mapping

**For Implementation:**
- Read this summary (~400 lines)
- Read current increment specification (~250 lines)
- Read relevant test specs (~100-150 lines)
- **Total context:** ~650-800 lines per increment

---

## Plan Status

**Phase 1-3 Status:** [Complete/In Progress/Pending]
**Phases 4-8 Status:** [Complete/In Progress/Pending/N/A]
**Current Status:** [Ready for Implementation / In Progress / Complete]
**Estimated Timeline:** [X-Y hours over Z weeks]

---

## Approval and Sign-Off

**Plan Created:** [Date]
**Plan Status:** [Ready for Implementation Review / Approved / Complete]

**Next Action:** [What needs to happen next]
```

**Critical Rule:** The "Technical Debt and Known Issues" section is MANDATORY in all 00_PLAN_SUMMARY.md files generated by Phase 8. This section MUST reference Phase 9 and include technical debt reporting when implementation is complete.

---

### PHASE 9: Post-Implementation Review and Technical Debt Assessment

**Purpose:** Systematically discover and document technical debt, known problems, and implementation shortcuts taken during plan execution. Ensure transparency about what was deferred, what assumptions were made, and what issues remain.

**When Executed:** AFTER completing all planned increments, BEFORE marking plan complete and archiving.

**Critical Rule:** Phase 9 is MANDATORY for ALL implementation plans. No exceptions.

---

#### Rationale

**Problem:** Without systematic technical debt discovery, problems remain hidden until they cause failures. Teams unknowingly inherit undocumented issues, assumptions, and incomplete implementations.

**Solution:** Structured discovery process forces explicit identification and documentation of all known issues, deferred work, and assumptions. Makes technical debt visible and manageable.

**Benefit:** Future developers understand complete system state (including limitations), reducing surprise failures and enabling informed maintenance decisions.

---

#### Technical Debt Definition

Technical debt is ANY decision, implementation, or omission that increases future development or maintenance costs. Categories:

**1. Code Quality Issues:**
- Code duplication
- Overly complex functions (>200 lines, cyclomatic complexity >10)
- Unclear variable/function names
- Missing or inadequate code documentation
- Inconsistent coding style

**2. Incomplete Implementations:**
- Edge cases not handled
- Error handling missing or incomplete
- Input validation insufficient
- Assumptions not validated at runtime

**3. Test Coverage Gaps:**
- Untested code paths
- Missing edge case tests
- Integration tests skipped
- Flaky or skipped tests

**4. Performance Issues:**
- Known bottlenecks not optimized
- Inefficient algorithms used (deferred optimization)
- Resource leaks possible
- Scalability concerns unaddressed

**5. Workarounds and Hacks:**
- TODO/FIXME/HACK comments in code
- Temporary solutions that became permanent
- Dependencies on external system assumptions
- Manual steps required for operations

**6. Documentation Debt:**
- Missing API documentation
- Outdated specifications
- Unclear system behavior
- No operational runbooks

**7. Dependency Issues:**
- Outdated library versions
- Security vulnerabilities in dependencies
- Deprecated API usage
- License compliance concerns

**8. Deferred Requirements:**
- Requirements explicitly deferred (with user approval)
- Requirements partially implemented
- Non-functional requirements not met (performance, security, accessibility)

---

#### 7-Step Technical Debt Discovery Process

**Execute ALL 7 steps in sequence. Skip NONE. Checkbox format MANDATORY.**

---

**Step 1: Code Review for Markers**

Search ALL modified code files for technical debt markers:

**Actions:**
- [ ] Search for comments: `TODO`, `FIXME`, `HACK`, `XXX`, `NOTE`, `WORKAROUND`
- [ ] Review all compiler/linter warnings (must be zero or documented)
- [ ] Check error handling: Are all `Result<>` types properly handled? Any `.unwrap()` calls?
- [ ] Identify temporary/placeholder implementations

**Document:** List all markers found with file:line references

**Output Format:**
```markdown
### Code Markers Found

**TODO Comments (N total):**
- file.rs:45 - TODO: Add validation for negative values
- file.rs:123 - TODO: Implement retry logic

**Compiler Warnings (N total):**
- file.rs:67 - unused variable `foo` (acceptable: debugging aid)
- file.rs:89 - unreachable code (MUST FIX: logic error)

**Unwrap Calls (N total):**
- file.rs:12 - .unwrap() acceptable: config file required for startup
- file.rs:34 - .unwrap() CONCERN: user input, should return error
```

---

**Step 2: Test Coverage Analysis**

Evaluate testing completeness for ALL implemented code:

**Actions:**
- [ ] Run test coverage tool (cargo-tarpaulin for Rust, coverage.py for Python, etc.)
- [ ] Identify code paths with zero test coverage
- [ ] List skipped tests (tests marked with `#[ignore]` or similar)
- [ ] Document flaky tests (tests that pass/fail inconsistently)
- [ ] Identify missing test categories (unit/integration/system)

**Document:** Coverage metrics + gaps

**Output Format:**
```markdown
### Test Coverage Analysis

**Overall Coverage:** X% (Target: 80%+)

**Uncovered Code Paths:**
- file.rs:45-67 - Error recovery logic (23 lines uncovered)
- file.rs:89-92 - Edge case: empty input (4 lines uncovered)

**Skipped Tests (N total):**
- test_name (file_test.rs:12) - Reason: Requires external service

**Flaky Tests (N total):**
- test_concurrent_access - Passes 90% of time, timing-dependent

**Missing Test Categories:**
- Integration tests: Module A + Module B interaction not tested
- System tests: End-to-end workflow not tested
```

---

**Step 3: Quality Review**

Assess code quality issues that increase maintenance costs:

**Actions:**
- [ ] Identify code duplication (>10 lines repeated, DRY violations)
- [ ] Measure function complexity (identify functions >200 lines or cyclomatic complexity >10)
- [ ] Check for magic numbers/strings (hardcoded values without explanation)
- [ ] Review variable/function naming clarity
- [ ] Identify missing function-level documentation

**Document:** Quality issues found

**Output Format:**
```markdown
### Code Quality Issues

**Code Duplication (N instances):**
- Lines 45-67 in file1.rs duplicated in file2.rs:89-111 (23 lines)
  - Impact: Change requires editing 2+ locations
  - Recommendation: Extract to shared function

**Complex Functions (N functions):**
- `calculate_flavor_distance()` (file.rs:123) - 245 lines, complexity 15
  - Recommendation: Split into subfunctions

**Magic Numbers (N instances):**
- file.rs:45 - `0.7` hardcoded (threshold for what?)
  - Recommendation: Extract to named constant

**Unclear Naming (N instances):**
- Variable `x` (file.rs:67) - Unclear purpose
  - Recommendation: Rename to `crossfade_duration_ms`
```

---

**Step 4: Known Problems Catalog**

Explicitly list ALL known bugs, limitations, and edge cases not handled:

**Actions:**
- [ ] List all known bugs (reproducible defects)
- [ ] List all edge cases not handled (documented limitations)
- [ ] List all assumptions that may not hold in all environments
- [ ] List all "works in practice but not guaranteed" behaviors

**Document:** Complete problem catalog

**Output Format:**
```markdown
### Known Problems

**Known Bugs (N total):**
1. **Bug:** Import fails if filename contains UTF-8 emoji
   - Severity: Medium
   - Workaround: Rename file before import
   - Root Cause: Path parsing library limitation
   - Fix Estimate: 2-3 hours

**Unhandled Edge Cases (N total):**
1. **Case:** Root folder path >4096 characters
   - Impact: Program panics
   - Frequency: Rare (PATH_MAX constraint)
   - Recommendation: Add validation, return error

**Assumptions (N total):**
1. **Assumption:** SQLite database is always writable
   - Risk: If filesystem readonly, application fails
   - Mitigation: Check write permission on startup

**"Works But Not Guaranteed" (N total):**
1. **Behavior:** Crossfade timing accurate to ~0.02ms on test hardware
   - Concern: May drift on different hardware/OS
   - Recommendation: Add runtime timing validation
```

---

**Step 5: Error Handling Completeness**

Verify ALL error paths are properly handled:

**Actions:**
- [ ] Verify all `Result<>` types are handled (no `.unwrap()` on user input or I/O)
- [ ] Check for ignored errors (empty `catch` blocks, `let _ = ...`)
- [ ] Validate error messages are clear and actionable
- [ ] Confirm all errors are properly logged
- [ ] Identify error conditions not handled at all

**Document:** Error handling gaps

**Output Format:**
```markdown
### Error Handling Completeness

**Unhandled Error Paths (N total):**
- file.rs:45 - Network timeout not handled (assumes network always available)
- file.rs:67 - Disk full not handled (write may fail silently)

**Ignored Errors (N instances):**
- file.rs:89 - `let _ = update_cache()` - Cache update failure ignored
  - Recommendation: Log error, continue execution

**Unclear Error Messages (N instances):**
- "Operation failed" (file.rs:123) - Does not specify what failed or why
  - Recommendation: Include context (file path, operation type, error code)

**Missing Logging (N instances):**
- file.rs:145 - Database error caught but not logged
  - Recommendation: Add error! macro call
```

---

**Step 6: Performance Assessment**

Document known performance issues and deferred optimizations:

**Actions:**
- [ ] Identify observed bottlenecks (profiling data if available)
- [ ] List deferred optimizations (documented as "optimize later")
- [ ] Document performance constraints (memory usage, CPU usage, latency)
- [ ] Note scalability concerns (breaks at what scale?)

**Document:** Performance technical debt

**Output Format:**
```markdown
### Performance Assessment

**Observed Bottlenecks (N total):**
1. **Bottleneck:** Musical flavor distance calculation takes 15ms per passage
   - Impact: Limits selection pool to ~66 passages per second
   - Current Scale: Acceptable for 1000-passage library
   - Concern: May not scale to 100,000-passage library
   - Optimization Identified: Use spatial indexing (k-d tree)

**Deferred Optimizations (N total):**
1. **Area:** String parsing in import loop
   - Current: ~50ms per file
   - Estimated Improvement: 2x faster with zero-copy parsing
   - Reason Deferred: Current performance acceptable (<100k files)

**Resource Constraints (N identified):**
- Memory: Holds all passages in RAM (~1MB per 1000 passages)
- Disk I/O: 100 IOPS required during import
```

---

**Step 7: Security and Dependency Review**

Identify security concerns and dependency issues:

**Actions:**
- [ ] Note any security concerns (authentication, authorization, injection risks)
- [ ] Document trust boundaries and assumptions
- [ ] List areas needing security review
- [ ] Check for dependency vulnerabilities (`cargo audit` for Rust, `npm audit` for Node)
- [ ] Identify deprecated API usage
- [ ] Note license compliance concerns

**Document:** Security/dependency debt

**Output Format:**
```markdown
### Security and Dependency Review

**Security Concerns (N total):**
1. **Concern:** Root folder path not validated for directory traversal
   - Risk: User could specify `/etc` and expose system files
   - Severity: Medium
   - Mitigation: Add path validation, restrict to user home directory

**Trust Boundary Assumptions (N total):**
- Assumes MusicBrainz API returns safe data (no sanitization)
- Assumes local filesystem is trusted (no permission checks)

**Dependency Vulnerabilities (N total):**
- [None identified by cargo audit as of YYYY-MM-DD]
OR
- reqwest 0.11.4 - Known CVE-2023-XXXX (DoS vulnerability)
  - Mitigation: Upgrade to 0.11.18+

**Deprecated APIs (N instances):**
- Uses `std::sync::mpsc` (deprecated in favor of `tokio::sync::mpsc`)
  - Impact: May be removed in future Rust versions
  - Recommendation: Migrate to tokio equivalent
```

---

#### Technical Debt Reporting Standard

**After completing all 7 steps, generate comprehensive technical debt report:**

---

**Report Template:**

```markdown
# Technical Debt Report: PLANxxx

**Plan:** [Plan Name]
**Implementation Complete:** [Date]
**Review Date:** [Date]
**Reviewed By:** [Name/Agent]

---

## Executive Summary

**Total Technical Debt Items:** [N]
- Critical (MUST address before next release): [N]
- High (Should address soon): [N]
- Medium (Defer to future release): [N]
- Low (Monitor, no action needed): [N]

**Estimated Remediation Effort:** [X-Y hours]

**Recommended Next Actions:**
1. [Action 1 with priority]
2. [Action 2 with priority]
3. [Action 3 with priority]

---

## Discovery Process Verification

**7-Step Process Completion:**
- [x] Step 1: Code Review for Markers - [N] items found
- [x] Step 2: Test Coverage Analysis - [X]% coverage, [N] gaps
- [x] Step 3: Quality Review - [N] issues identified
- [x] Step 4: Known Problems Catalog - [N] problems documented
- [x] Step 5: Error Handling Completeness - [N] gaps found
- [x] Step 6: Performance Assessment - [N] concerns noted
- [x] Step 7: Security and Dependency Review - [N] items identified

---

## Technical Debt Inventory

### Critical Items (MUST Address)

[Items that MUST be fixed before next release or before production deployment]

**CRITICAL-001: [Brief Description]**
- **Category:** [Code Quality / Incomplete / Test Gap / Performance / etc.]
- **Location:** [file:line or component]
- **Issue:** [Detailed description]
- **Impact:** [What breaks or degrades]
- **Remediation:** [How to fix]
- **Effort:** [Estimate in hours]
- **Discovered:** Step [N]

### High Priority Items (Should Address Soon)

[Items that should be addressed in next 1-2 releases]

**HIGH-001: [Brief Description]**
- **Category:** [...]
- **Location:** [...]
- **Issue:** [...]
- **Impact:** [...]
- **Remediation:** [...]
- **Effort:** [...]
- **Discovered:** Step [N]

### Medium Priority Items (Defer to Future)

[Items that can wait but should be tracked]

**MEDIUM-001: [Brief Description]**
- **Category:** [...]
- **Location:** [...]
- **Issue:** [...]
- **Impact:** [...]
- **Remediation:** [...]
- **Effort:** [...]
- **Discovered:** Step [N]

### Low Priority Items (Monitor)

[Items noted for awareness, no immediate action needed]

**LOW-001: [Brief Description]**
- **Category:** [...]
- **Location:** [...]
- **Issue:** [...]
- **Impact:** [...]
- **Remediation:** [...]
- **Effort:** [...]
- **Discovered:** Step [N]

---

## Deferred Requirements

[Requirements explicitly deferred with user approval during implementation]

**DEF-REQ-001: [Requirement Description]**
- **Original Requirement:** [REQ-XXX-YYY]
- **Reason Deferred:** [Technical blocker / Changed priority / User decision]
- **Approved By:** [User, Date]
- **Impact:** [What functionality is missing]
- **Plan to Address:** [Future release plan or "wontfix"]

---

## Test Coverage Summary

**Overall Coverage:** [X]%
**Target:** 80%+
**Gap:** [X]% below target OR "Exceeds target ✓"

**Significant Coverage Gaps:**
- [Component/module]: [X]% coverage, [N] lines uncovered
- [Component/module]: [X]% coverage, [N] lines uncovered

---

## Performance Baseline

**Benchmarks Established:**
- [Operation]: [X]ms average, [Y]ms p99
- [Operation]: [X] operations/second throughput

**Performance Debt:**
- [N] known bottlenecks documented
- [N] deferred optimizations tracked

---

## Security Posture

**Vulnerabilities:** [N] identified ([N] critical, [N] high, [N] medium, [N] low)
**Dependency Audit:** [Clean / N issues found]
**Trust Boundaries:** [N] assumptions documented

---

## Recommendations

**Immediate Action Required (CRITICAL items):**
1. [Action 1]
2. [Action 2]

**Next Release (HIGH items):**
1. [Action 1]
2. [Action 2]

**Future Consideration (MEDIUM/LOW items):**
1. [Action 1]
2. [Action 2]

**Monitoring:**
- [Metric to watch]
- [Condition that requires action]

---

## Sign-Off

**Technical Debt Discovery Complete:** [Date]
**Report Reviewed By:** [Name]
**Status:** Ready for Archive

**Next Review:** [After next major release / In 6 months / etc.]
```

---

#### Integration with Plan Completion Report

**Phase 9 technical debt report is referenced in Plan Execution Completion Report (Section 2 of Plan Execution Standards).**

In the "Known Issues and Technical Debt" section of completion report:

```markdown
### Known Issues and Technical Debt

See Phase 9 Technical Debt Report for complete analysis: [link to report file]

**Executive Summary:**
- Total Items: [N] ([N] critical, [N] high, [N] medium, [N] low)
- Estimated Remediation: [X-Y] hours
- Immediate Action Required: [Yes/No]

**Critical Items Requiring Immediate Attention:**
1. [CRITICAL-001 brief description]
2. [CRITICAL-002 brief description]
```

---

#### Enforcement

**Phase 9 is MANDATORY:**
- Execute all 7 steps (no skipping)
- Generate complete technical debt report
- Include executive summary in plan completion report
- Archive report with plan documentation

**Acceptance Criteria:**
- [ ] All 7 discovery steps completed (checkboxes)
- [ ] Technical debt report generated using template
- [ ] All items categorized by severity
- [ ] Remediation estimates provided
- [ ] Completion report references technical debt report

**If Phase 9 skipped or incomplete:**
- Plan is NOT complete
- Do NOT mark plan as ready for archive
- Do NOT proceed to next plan without completing Phase 9

---

## Plan Document Structure (Output)

### Modular Folder Architecture

**Location:** All plan working folders are created in `wip/` for easy batch archival when complete.

```
wip/PLAN###_[feature_name]/
├── 00_PLAN_SUMMARY.md                 # <500 lines - READ THIS FIRST
├── 01_specification_issues.md          # Phase 2 output
├── 02_test_specifications/             # Phase 3 output (modular)
│   ├── test_index.md                   # Quick reference table
│   ├── tc_u_001_01.md                  # Individual test specs
│   ├── tc_u_001_02.md
│   ├── ...
│   └── traceability_matrix.md
├── 03_approach_selection.md            # Phase 4 output (Week 2)
├── 04_increments/                      # Phase 5 output (Week 2)
│   ├── increment_01.md                 # <300 lines each
│   ├── increment_02.md
│   ├── ...
│   └── checkpoints.md
├── 05_estimates.md                     # Phase 6 output (Week 3)
├── 06_risks.md                         # Phase 7 output (Week 3)
├── requirements_index.md               # Phase 1 output
└── FULL_PLAN.md                        # Phase 8 consolidated (archival)
```

**Folder Naming Convention:**
- Format: `PLAN###_[descriptive_feature_name]`
- Plan number assigned from workflows/REG001_number_registry.md
- Example: `PLAN005_crossfade_implementation`

**Lifecycle Management:**
- Plan folders are created in `wip/` as work-in-progress
- After plan implementation is complete, use `/archive-plan PLAN###` to archive
- Archival removes folder from wip/ while preserving in archive branch
- See `.claude/commands/archive-plan.md` for archival workflow

### Document Size Targets (Context Window Management)

| Document | Target Size | Purpose | When to Read |
|----------|-------------|---------|--------------|
| 00_PLAN_SUMMARY.md | <500 lines | Overview + roadmap | Always start here |
| 01_specification_issues.md | Varies | Issues found + resolutions | When clarifying specs |
| 02_test_specifications/test_index.md | <200 lines | Test quick reference | Planning, verification |
| 02_test_specifications/tc_*.md | <100 lines | Individual test details | When implementing that test |
| 04_increments/increment_XX.md | <300 lines | Single increment details | When implementing that increment |
| 00_PLAN_SUMMARY.md + increment | <800 lines | Everything needed to implement | Typical AI context |
| FULL_PLAN.md | >2000 lines | Complete consolidated | Archival/review only |

**Key Principle:** Implementer (AI or human) reads ONLY:
1. Plan summary (~400 lines)
2. Current increment (~250 lines)
3. Relevant test specs (~100-200 lines)

**Total context:** ~600-850 lines, not 2000+

### Alignment with CLAUDE.md Standards

**This workflow implements CLAUDE.md standards:**
- Plan summary: <500 lines (specified line 457)
- Test specs: <100 lines each (specified line 462)
- Increments: <300 lines each (specified line 463)
- Verbosity: 20-40% reduction target (from CLAUDE.md)
- Reading protocol: Summary + increment only (~600-850 lines, not 2000+) (specified line 471)

**These targets ensure optimal context window usage during implementation.**

**MANDATORY Two-Tier Output Structure:**

All /plan outputs MUST provide:

1. **Executive Summary** (<500 lines) - 00_PLAN_SUMMARY.md
   - Problems being solved
   - Solution approach overview
   - Implementation timeline
   - Key decisions required
   - Success metrics

2. **Detailed Content** - Modular sections
   - Each section <300 lines (increments, test specs, risks)
   - Progressive disclosure: Read only what's needed
   - Full plan (FULL_PLAN.md) for archival/review only

**Enforcement:**
- Before writing plan: Count lines in 00_PLAN_SUMMARY.md
- If >500 lines: Compress to bullet points + references to detail sections
- Implementer reads: Summary + current increment (<800 lines total)
- DO NOT read FULL_PLAN.md during implementation (context overload)

---

## Workflow Constraints

### MUST DO:
- Read and understand complete specification document
- Extract ALL requirements systematically
- Verify specification completeness before planning implementation
- Define acceptance tests for every requirement
- Create modular, context-window-optimized output structure
- Present specification issues to user with clear severity levels
- Stop at checkpoints for user review and approval
- Preserve original specification document unchanged
- Reference WKMP documentation hierarchy (GOV001) and requirement numbering (GOV002)
- Apply rigorous software engineering standards
- Consider microservices architecture implications (HTTP APIs, SSE events)

### MUST NOT DO:
- Skip specification verification to jump to planning
- Assume missing information without flagging it
- Create monolithic plan documents >2000 lines
- Define requirements - only work with specifications provided
- Proceed with CRITICAL specification issues unresolved
- Modify original specification document
- Begin implementation (that's separate from planning)
- Make architectural decisions without evaluation of alternatives (Phase 4)

### Context Window Management Rules:
- When processing large specifications (>1500 lines): Use chunked analysis approach
- When generating plans: Create modular folder structure, not monolithic files
- When defining tests: Individual test files, not one large document
- When writing increments: One file per increment, <300 lines
- Always provide consumption guidance: "Read ONLY X and Y, not Z"

---

## Integration with Existing Workflows

### Recommended Sequence:

```
1. /think [requirements_doc]
   ↓ [Analysis of requirements and approaches]

2. Review /think analysis
   ↓ [Decision on approach feasibility]

3. /plan [specifications_doc]
   ↓ Phase 1-2: Scope + Specification Verification
   ↓ CHECKPOINT: Resolve critical specification issues
   ↓ Phase 3: Acceptance Test Definition
   ↓ CHECKPOINT: Review test coverage
   ↓ [Week 2: Phases 4-5 - Approach + Implementation Breakdown]
   ↓ [Week 3: Phases 6-8 - Estimates + Risks + Documentation]

4. Review /plan output (read 00_PLAN_SUMMARY.md)
   ↓ [Approval to proceed]

5. Implement following plan, increment by increment
   ↓ For each increment:
   ↓   - Read increment_XX.md + relevant tests
   ↓   - Implement to pass tests
   ↓   - Commit after passing tests

6. At checkpoints: Review progress, verify tests pass

7. After implementation: Comprehensive verification

8. Implementation Complete
```

### Decision Points:

| Stage | Decision | Required Before Proceeding |
|-------|----------|----------------------------|
| After Phase 1 | Scope correct? | User confirms understanding |
| After Phase 2 | Specs adequate? | Critical/High issues resolved |
| After Phase 3 | Tests comprehensive? | 100% requirement coverage |
| After Plan | Approve plan? | Plan reviewed, risks acceptable |
| During implementation | Next increment? | Previous increment tests pass |
| At checkpoints | Continue? | Checkpoint criteria met |
| After implementation | Release? | All acceptance tests pass |

---

## Quality Standards

### Completeness:
- ALL requirements from specifications included in requirements_index
- ALL requirements have acceptance tests defined
- ALL scope boundaries explicitly stated
- ALL dependencies identified and status known
- NO topics left unaddressed without explicit reason

### Accuracy:
- Findings verified against specification document
- No assumptions stated as facts
- Uncertainties clearly identified as issues
- Test specifications match requirements exactly

### Traceability:
- Every requirement has unique ID (per GOV002)
- Every test traces to specific requirement(s)
- Traceability matrix complete with no gaps
- Easy to follow: requirement → test → implementation

### Context Window Efficiency:
- No document >500 lines except FULL_PLAN.md (archival only)
- Modular structure: read only what's needed
- Clear consumption guidance provided
- Increment files self-contained (<300 lines)

### WKMP Architecture Alignment:
- Specification issues prevent implementation risks
- Test specifications support microservices architecture
- Traceability supports WKMP documentation hierarchy (GOV001)
- Integration tests verify HTTP/SSE communication patterns
- Complete documentation aligns with EXEC001 implementation order

---

## Success Criteria

The `/plan` workflow is successful when:

**Phase 1-3 Complete (Week 1):**
1. ✓ Specification document analyzed systematically
2. ✓ All requirements extracted into compact index
3. ✓ Scope boundaries clear (in/out, assumptions, constraints)
4. ✓ Specification issues identified and prioritized
5. ✓ Critical issues flagged for resolution before implementation
6. ✓ Every requirement has acceptance test(s) defined
7. ✓ Test coverage 100% (traceability matrix complete)
8. ✓ Modular output structure created (context-window optimized)
9. ✓ User checkpoints completed with approval to proceed
10. ✗ NO implementation begun (planning only at this stage)

**Full Workflow Complete (Week 3):**
- All above, plus:
- ✓ Multiple implementation approaches evaluated (Phase 4)
- ✓ Implementation broken into sized increments (Phase 5)
- ✓ Effort estimated with contingency (Phase 6)
- ✓ Risks identified with mitigations (Phase 7)
- ✓ Plan summary and full documentation generated (Phase 8)
- ✓ Plan approved and ready for implementation

## Workflow Execution Verification Checklist

**Before marking /plan workflow complete, verify ALL 8 phases executed:**

**Planning Only (Phases 1-3):**
- [ ] Phase 1: Scope Definition - requirements index, scope statement, dependencies created
- [ ] Phase 2: Specification Completeness Verification - issues identified, categorized by severity
- [ ] Phase 3: Test Specifications - ALL tests defined with traceability matrix 100% complete
- [ ] User checkpoint: Approved to proceed OR stopped here for specification fixes

**Full Planning (Phases 4-8):**
- [ ] Phase 4: Approach Selection - multiple approaches evaluated, one selected with rationale
- [ ] Phase 5: Implementation Breakdown - sized increments defined (<2 days each)
- [ ] Phase 6: Effort Estimation - estimates with contingency, realistic timelines
- [ ] Phase 7: Risk Assessment - risks identified with likelihood/impact/mitigation
- [ ] Phase 8: Plan Documentation - 00_PLAN_SUMMARY.md + full documentation created

**Common Skipped Elements:**
- ❌ Traceability matrix (Phase 3) - Must be 100% complete
- ❌ User checkpoints - Must pause and get explicit approval
- ❌ 00_PLAN_SUMMARY.md (Phase 8) - Required executive summary
- ❌ Specification issues (Phase 2) - Must document even if "none found"

**Verification:**
1. Count files created vs. expected outputs
2. Check traceability matrix for gaps
3. Verify no implementation begun (only planning)
4. Confirm user approved to proceed at checkpoints

**If any phase incomplete:**
1. Complete missing elements before proceeding
2. Document deviation in workflow execution log
3. Update workflow if pattern issue identified

---

## Plan Execution and Completion Standards

**Purpose:** Ensure all planned work is completed as specified, with transparent reporting of deviations.

**Scope:** Applies to ALL implementation plans created via /plan workflow.

---

### 1. Mandatory Execution Requirements

When executing an implementation plan (PLANxxx), the following are REQUIRED:

**Rule 1: Execute All Increments OR Get User Approval to Skip**
- ALL increments in the plan MUST be executed as specified
- If an increment needs to be skipped, deferred, or modified:
  - STOP implementation
  - Explain reason for deviation (technical blocker, changed requirements, etc.)
  - Request explicit user approval: "Skip increment N? (Reason: ...)"
  - Document deviation in completion report (see Section 2)
- NEVER silently skip increments
- NEVER decide unilaterally that "this increment isn't needed"

**Rule 2: Follow Test-First Approach**
- Read acceptance tests BEFORE implementing each increment
- Implement to pass tests (tests define "done")
- Verify tests pass before marking increment complete
- If tests cannot pass, document as deviation (requires user approval)

**Rule 3: No Shortcuts (see Section 3)**
- Implement complete functionality per specification
- Do NOT implement "simplified version" unless explicitly approved
- Do NOT defer error handling, edge cases, or testing
- Phased delivery (shipping subset first) is acceptable; partial implementation is not

**Enforcement:**
- Mark increment complete ONLY when all acceptance tests pass
- Update todo list in real-time (one increment in_progress at a time)
- Report deviations immediately when discovered

---

### 2. Completion Report Standard

**When Required:**
- At end of each increment (brief: 1-3 sentences)
- At end of entire plan execution (comprehensive: full report)

**End-of-Increment Report (Brief):**

After completing each increment, provide:
1. Increment name/number
2. Tests executed and results (e.g., "TC-M-001-01: PASS, TC-M-001-02: PASS")
3. Deviations from plan (if any): "Planned X, actually did Y because Z"
4. Files modified (list with line ranges)

**Example:**
```
Increment 1 Complete: Professional Objectivity Section (CLAUDE.md)

Tests:
- TC-M-001-01: PASS (section exists at line 125)
- TC-M-001-02: PASS (6 elements present)
- TC-M-002-01: PASS (fact/opinion standards documented)
- TC-M-003-01: PASS (disagreement protocol documented)

Deviations: None

Files Modified:
- CLAUDE.md lines 125-174 (+52 lines, target was ~75 lines - slightly under but complete per tests)
```

**End-of-Plan Report (Comprehensive):**

After completing ALL increments, provide:

```markdown
## Plan Execution Report: PLANxxx

**Plan:** [Plan Name]
**Execution Start:** [Date]
**Execution Complete:** [Date]
**Total Time:** [Actual hours]

### Summary
- Total Increments: [Planned] / [Completed]
- All Tests: [Passed] / [Total]
- Deviations: [Count] (see details below)

### Increments

**Increment 1: [Name]**
- Status: ✅ Complete / ⚠️ Partial / ❌ Skipped
- Tests: [Pass/Total]
- Effort: [Estimated] → [Actual]
- Deviations: [None / Description]

**Increment 2: [Name]**
...

### Deviations from Plan

**Deviation 1: [Brief description]**
- Increment: [N]
- Planned: [What was originally planned]
- Actually Did: [What was actually implemented]
- Reason: [Technical blocker / Changed requirements / User approved simplification]
- Approved By: [User / N/A]
- Impact: [Describe impact on functionality, timeline, or quality]

### Test Results

| Test ID | Status | Notes |
|---------|--------|-------|
| TC-M-001-01 | PASS | ... |
| TC-M-001-02 | PASS | ... |
| ... | | |

**Coverage:** [X/Y tests passed] ([percentage]%)

### Files Modified

- file1.md: lines X-Y (+N lines)
- file2.rs: lines A-B (~M lines modified)
...

### Effort Analysis

- Estimated Total: [X-Y hours]
- Actual Total: [Z hours]
- Variance: [+/-N hours] ([percentage]%)
- Reason for Variance: [Explain if >20% variance]

### Known Issues and Technical Debt

(See Phase 9: Post-Implementation Review for technical debt discovered during implementation)

### Sign-Off

- [ ] All planned increments completed OR deviations approved
- [ ] All acceptance tests pass
- [ ] Traceability matrix 100% complete
- [ ] Technical debt documented (Phase 9)
- [ ] Plan ready for archive

**Completed By:** [Name/Agent]
**Date:** [YYYY-MM-DD]
```

**Critical Rule:** Completion report is MANDATORY. Do NOT mark plan complete without generating this report.

---

### 3. No-Shortcut Implementation

**Definition:** A "shortcut" is implementing less than specified functionality with the intent of shipping it as complete.

**Shortcuts vs. Phased Delivery:**

**Shortcut (NOT Allowed):**
- Implement partial functionality and claim it's "done"
- Skip error handling, edge cases, or tests
- Implement "good enough for now" with no plan to complete
- Defer critical requirements indefinitely

**Phased Delivery (Acceptable):**
- Ship working subset first, remaining features later
- Each phase is complete within its scope
- Each phase passes all tests for its scope
- Clear plan exists for subsequent phases

**The Difference:** Shortcuts deliver incomplete work; phased delivery delivers complete subsets.

---

#### Examples: Shortcut vs. Phased Delivery

**Example 1: API Error Handling**

❌ **Shortcut (NOT Allowed):**
- Implement happy path only
- Mark increment "complete"
- Claim: "Error handling can be added later if needed"
- **Problem:** Incomplete functionality shipped as complete

✅ **Phased Delivery (Acceptable):**
- **Phase 1:** Implement happy path + basic error handling (4xx/5xx responses)
- **Phase 2:** Add detailed error codes, recovery strategies
- Each phase complete within scope, with tests
- **Key:** Phase 1 is production-ready, not a prototype

✅ **Complete Implementation (Best):**
- Implement happy path + comprehensive error handling in one increment
- All tests pass
- Production-ready immediately

---

**Example 2: Feature with Complex Edge Cases**

❌ **Shortcut (NOT Allowed):**
- Implement common case only
- Document edge cases as "known limitations"
- No plan to address edge cases
- Mark feature "complete"
- **Problem:** Partially implemented feature

✅ **Phased Delivery (Acceptable):**
- **Phase 1:** Implement common case (e.g., 80% of usage)
- Explicitly document out-of-scope edge cases with tracking issues
- **Phase 2:** Implement remaining edge cases
- Each phase has clear scope and passes all tests for that scope
- **Key:** Phase 1 scope is intentionally limited but complete

✅ **Complete Implementation (Best):**
- Implement all cases (common + edge cases) in one increment
- All tests pass
- No known limitations

---

**Example 3: Database Schema Migration**

❌ **Shortcut (NOT Allowed):**
- Implement schema changes
- Skip migration script (manual SQL needed)
- Skip rollback plan
- Claim: "Migration is complete, ops team can handle deployment"
- **Problem:** Incomplete implementation (not deployable)

✅ **Phased Delivery (Acceptable):**
- **Phase 1:** Implement backward-compatible schema changes only
- Full migration + rollback scripts included
- **Phase 2:** Remove deprecated columns after cutover
- Each phase is fully deployable
- **Key:** Phase 1 deployment does not break existing code

✅ **Complete Implementation (Best):**
- Implement full schema change
- Migration scripts (up + down)
- Tested rollback procedure
- Deployment runbook included
- Fully deployable immediately

---

#### Enforcement

**During Implementation:**
1. If you find yourself thinking "I'll skip X for now", STOP
2. Ask: "Is this a shortcut (incomplete) or phased delivery (complete subset)?"
3. If shortcut: Implement completely OR request user approval to narrow scope
4. Document decision in completion report

**During Code Review:**
1. Verify all requirements in increment scope are implemented
2. Check for deferred error handling, missing tests, incomplete edge cases
3. Distinguish shortcuts (reject) from phased delivery (acceptable)

**Test-Based Verification:**
- If acceptance tests pass → increment is complete (by definition)
- If acceptance tests cannot pass due to missing functionality → increment is incomplete (shortcut)
- If scope was narrowed with user approval → update tests to match new scope

---

## Error Handling

**Specification Not Found:**
- State file does not exist clearly
- Prompt for correct path
- Offer to list similar filenames in workspace

**Incomplete Specification:**
- Explicitly state what information is missing (Phase 2 issues)
- Classify as CRITICAL if blocks implementation
- Recommend specification updates needed
- Do NOT proceed to implementation planning with CRITICAL issues

**No Requirements Found:**
- Report: "No SHALL/MUST statements found in document"
- Verify this is correct specification document
- May indicate informal/incomplete specification
- Cannot proceed - planning requires explicit requirements

**Ambiguous Requirements:**
- Document as HIGH severity issues in Phase 2
- Provide specific examples of ambiguity
- Recommend clarification language
- If 5+ ambiguous requirements: Consider /think for analysis

**Context Window Exceeded (Large Specification):**
- Automatically switch to chunked analysis mode
- Extract requirements index first pass
- Process requirements in batches
- Inform user: "Large specification detected, using chunked analysis"

**Complex/Novel Technical Elements:**
- Trigger automatic /think integration
- Present /think analysis to user
- Wait for user review before continuing
- Use /think insights in approach selection (Phase 4)

---

## Integration with Project Standards

This command operates within project context:
- Respects WKMP documentation hierarchy (GOV001)
- Applies requirement numbering scheme (GOV002)
- Aligns with EXEC001 implementation order
- References WKMP microservices architecture (SPEC001)
- Considers Rust/Tokio/Axum technology stack
- Maintains focus on quality and traceability
- Follows DRY principle in specification analysis
- Uses established test-driven development practices
- Context window management prioritized throughout
- Modular output structure matches project documentation standards

---

## Example Invocations

```bash
# Basic invocation
/plan docs/SPEC002-crossfade.md

# After /think analysis provides architectural guidance
/think docs/SPEC015-musical_flavor.md
# Review /think results
/plan docs/SPEC015-musical_flavor.md

# Planning from requirements document
/plan docs/REQ001-requirements.md
```

---

## Output Message Template

```markdown
Implementation planning initiated for: [Feature Name]

**Input:** [specification_document_path]
**Requirements Found:** [count]
**Plan Location:** wip/PLAN###_[feature_name]/

**Phase 1: Scope Definition** ✓
- [X] requirements extracted
- Scope boundaries defined
- [Y] dependencies identified

**Phase 2: Specification Verification** [Status]
- [Analyzing... / Complete]
- Issues found: [count by severity]
- [CRITICAL issues BLOCK implementation]

[If CRITICAL/HIGH issues found:]
**⚠️ SPECIFICATION ISSUES REQUIRE RESOLUTION**
See: [01_specification_issues.md]
Please review and update specifications before continuing.

[If /think triggered:]
**🤔 COMPLEXITY DETECTED - Automatic /think Analysis**
Formulating analysis query for: [topic]
[Execute /think]
[Present results for user review]

**Phase 3: Test Definition** [Status]
- [Defining... / Complete]
- Tests defined: [count]
- Coverage: [percentage]%

**✅ PLANNING COMPLETE (Phases 1-3)**

**Start Here:** [00_PLAN_SUMMARY.md] (400 lines)

**Next Actions:**
1. Review specification issues (if any)
2. Confirm test coverage complete
3. [If approved:] Begin implementation with Increment 1

**Implementation Guide:**
- Read plan summary first (quick overview)
- Then read [04_increments/increment_01.md] (~250 lines)
- Do NOT read FULL_PLAN.md (for archival only)

**Context Window Budget:**
- Summary + Increment: ~650 lines (optimal for AI/human)
- Full plan: [XXXX] lines (use only for archival/review)
```

---

## Version and Status

**Version:** 1.0 (Week 1 Deliverable - Phases 1-3)
**Status:** Production Ready for Phases 1-3
**Next Enhancement:** Week 2 - Add Phases 4-5 (Approach Selection + Implementation Breakdown)
**Author:** WKMP Music Player Development Team
**Date:** 2025-10-25

**Related Commands:**
- `/think` - For deep analysis before planning
- `/archive-plan` - For archiving completed plans

---

## Week 1 Implementation Notes

**What's Included:**
- ✅ Phase 1: Input Validation and Scope Definition
- ✅ Phase 2: Specification Completeness Verification
- ✅ Phase 3: Acceptance Test Definition
- ✅ Context window management throughout
- ✅ Modular output structure
- ✅ Automatic /think integration
- ✅ User checkpoints and approval gates

**What's Coming:**
- Week 2: Phases 4-5 (Approach selection, Implementation breakdown)
- Week 3: Phases 6-8 (Estimates, Risks, Final documentation)

**Current Capability:**
Can analyze specifications, identify issues, and define comprehensive test coverage - the most critical phase for preventing implementation failures.
