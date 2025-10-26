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
