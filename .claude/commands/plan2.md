# /plan2 - Enhanced Implementation Planning Workflow (Opus 4.5 Optimized)

## Command Signature
```
/plan2 [specification_document_path]
```

## Purpose

Create systematic, specification-driven implementation plans optimized for Opus 4.5's enhanced reasoning capabilities. This workflow maximizes probability of meeting all requirements on first implementation attempt through:

- **Extended thinking integration** at critical decision points
- **Semantic ambiguity detection** beyond keyword patterns
- **Self-verification loops** ensuring internal consistency
- **Pre-implementation simulation** catching hidden complexity
- **5-level confidence indicators** for transparent uncertainty communication
- **Adaptive checkpoints** based on plan complexity

**Key Difference from /plan:** This workflow explicitly leverages Opus 4.5's extended thinking mode and adds systematic verification steps that improve accuracy and reduce implementation bugs.

---

## Quick Reference: /plan2 vs /plan

| Feature | /plan | /plan2 |
|---------|-------|--------|
| Extended Thinking | Implicit | Explicit triggers at Phases 0, 2, 4, 7, 7.5 |
| Ambiguity Detection | Keyword patterns | Semantic analysis + implicit assumptions |
| Confidence Indicators | None | 5-level scale throughout |
| Self-Verification | None | After Phases 3, 5, and before 8 |
| Pattern Recognition | None | Phase 1.5 architectural analysis |
| Matrix Validation | Manual | Phase 3.5 automated validation |
| Pre-Implementation Sim | None | Phase 7.5 mental walkthrough |
| Checkpoints | Fixed | Adaptive based on complexity |
| Failure Mode Analysis | Basic | Structured with failure chains |

---

## Input Parameters

- **specification_document_path** (required): Path to specification document
  - Should contain: Requirements (SHALL/MUST statements), design specifications, or both
  - May reference: Analysis documents from /think, architecture documents, existing code

## Prerequisites

Before running /plan2:
- [ ] Requirements or design specifications documented
- [ ] Related /think analysis completed (if complex/novel features)
- [ ] Stakeholder expectations clear
- [ ] Technical feasibility understood

---

## 5-Level Confidence Scale

All findings, assessments, and estimates in /plan2 output use this standardized confidence scale:

| Level | Probability | Meaning | When to Use |
|-------|-------------|---------|-------------|
| **CERTAIN** | 95%+ | Objectively verifiable, documented evidence, logical necessity | Facts from specifications, mathematical certainty, explicit requirements |
| **HIGH** | 75-95% | Strong evidence, consistent patterns, high probability | Well-understood domains, similar past experience, clear precedent |
| **MEDIUM** | 50-75% | Reasonable inference, some supporting evidence | Logical deduction with assumptions, partial evidence |
| **LOW** | 25-50% | Limited evidence, speculative but plausible | Educated guesses, extrapolation from limited data |
| **SPECULATIVE** | <25% | Highly uncertain, edge case consideration | "What if" scenarios, novel situations, worst-case thinking |

**Usage Examples:**
```markdown
**Issue:** REQ-CF-010 does not specify crossfade curve type
**Confidence:** CERTAIN - The requirement text contains no curve specification

**Risk:** Database migration may corrupt existing data
**Confidence:** LOW - No evidence of corruption in similar migrations, but schema change is complex

**Estimate:** Increment 3 will take 2-4 hours
**Confidence:** HIGH (±20%) - Similar to increment completed last week
```

---

## Extended Thinking Guidelines

/plan2 explicitly triggers extended thinking at specific phases. When extended thinking is indicated:

**What Extended Thinking Provides:**
- Deeper analysis of complex relationships
- Systematic enumeration of possibilities
- Identification of non-obvious implications
- Reasoning about second-order effects
- Mental simulation of implementation

**Extended Thinking Triggers in /plan2:**
1. **Phase 0:** Build comprehensive mental model of specification
2. **Phase 2:** Detect semantic ambiguities and implicit assumptions
3. **Phase 4:** Compare approaches and enumerate failure modes
4. **Phase 7:** Analyze risk cascades and interactions
5. **Phase 7.5:** Simulate implementation to find hidden complexity

**Extended Thinking Format:**
When triggered, use explicit reasoning blocks:
```
<extended_thinking>
[Systematic analysis of the problem space]
[Enumeration of possibilities and implications]
[Reasoning about interactions and edge cases]
[Conclusions drawn from analysis]
</extended_thinking>
```

---

## Workflow Execution Phases

---

### PHASE 0: Extended Thinking Initialization (NEW)

**Objective:** Build comprehensive mental model of the specification before detailed analysis

**Trigger:** ALWAYS execute Phase 0 before Phase 1

**Activities:**

1. **Specification Comprehension**

   Use extended thinking to:
   ```
   <extended_thinking>
   Read the entire specification and build a mental model:

   1. What is the core problem being solved?
   2. What are the key entities and their relationships?
   3. What are the primary use cases?
   4. What technical constraints exist?
   5. What are the implicit assumptions?
   6. What is the expected scale/performance?
   7. How does this fit into the larger system?

   Identify potential complexity hotspots:
   - Areas where multiple requirements interact
   - Areas involving concurrency or timing
   - Areas touching external systems
   - Areas with implicit state management
   </extended_thinking>
   ```

2. **Complexity Assessment**

   Classify the specification:
   - **Simple:** <10 requirements, single component, well-understood domain
   - **Standard:** 10-30 requirements, 2-3 components, some novelty
   - **Complex:** >30 requirements, multiple components, novel elements

3. **Checkpoint Strategy Selection**

   Based on complexity assessment, select checkpoint strategy (see Adaptive Checkpoint System section)

**Output:**
- Mental model summary (3-5 sentences)
- Complexity classification (Simple/Standard/Complex)
- Selected checkpoint strategy
- Initial list of complexity hotspots

**No User Checkpoint:** Phase 0 is internal preparation

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
     | Req ID | Type | Brief Description | Line # | Priority | Complexity |
     |--------|------|-------------------|--------|----------|------------|
     | REQ-CF-010 | Functional | Sample-accurate crossfade timing | 45 | High | Medium |
     | REQ-PD-015 | Functional | Musical flavor distance calculation | 78 | High | High |
     ```
   - **NEW:** Add Complexity column (Low/Medium/High) based on Phase 0 analysis

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
- `requirements_index.md` - Compact table of all requirements with complexity ratings
- `scope_statement.md` - In/out of scope, assumptions, constraints
- `dependencies_map.md` - What exists, what's needed, what's external

**Success Criteria:**
- All requirements identified and cataloged
- Scope boundaries clear and unambiguous
- No confusion about what will/won't be implemented
- Dependencies identified and status known

**User Checkpoint (Standard/Complex plans only):** Present scope summary, confirm understanding before proceeding

---

### PHASE 1.5: Architectural Pattern Analysis (NEW)

**Objective:** Leverage knowledge of software patterns to inform implementation approach

**Trigger:** Execute after Phase 1, before Phase 2

**Activities:**

1. **Applicable Patterns Identification**

   Based on requirements, identify relevant patterns:

   ```markdown
   **Structural Patterns:**
   - [ ] Repository Pattern (data access abstraction)
   - [ ] Adapter Pattern (external service integration)
   - [ ] Facade Pattern (complex subsystem simplification)
   - [ ] Composite Pattern (hierarchical structures)

   **Behavioral Patterns:**
   - [ ] Observer Pattern (event-driven updates, SSE)
   - [ ] Command Pattern (action encapsulation)
   - [ ] State Pattern (state machine management)
   - [ ] Strategy Pattern (algorithm selection)

   **Concurrency Patterns (Rust/Tokio):**
   - [ ] Actor Pattern (message-passing concurrency)
   - [ ] Pipeline Pattern (staged processing)
   - [ ] Fan-out/Fan-in (parallel processing)
   - [ ] Circuit Breaker (fault tolerance)

   **WKMP-Specific Patterns:**
   - [ ] Microservice communication (HTTP REST + SSE)
   - [ ] Database-first configuration
   - [ ] Zero-config startup
   - [ ] Event broadcast (tokio::broadcast)
   ```

2. **Anti-Patterns to Avoid**

   Flag patterns that commonly cause problems:

   ```markdown
   **Code Smells to Watch For:**
   - God Object (single struct doing too much)
   - Feature Envy (excessive cross-module coupling)
   - Premature Optimization (complexity without measured need)
   - Magic Numbers (unexplained constants)

   **Concurrency Anti-Patterns:**
   - Blocking in async context
   - Shared mutable state without synchronization
   - Unbounded channels/queues
   - Missing cancellation handling

   **Architecture Anti-Patterns:**
   - Circular dependencies between modules
   - Leaky abstractions
   - Over-engineering (complexity beyond requirements)
   - Under-engineering (shortcuts that accumulate debt)
   ```

3. **Pattern Constraints Analysis**

   Consider what constrains pattern choice:
   - Existing WKMP architecture patterns (must align)
   - Rust/Tokio idioms (ownership, borrowing, async)
   - Microservices boundaries (HTTP/SSE communication)
   - Performance requirements (audio latency, real-time)

4. **Pattern Recommendations**

   For each complexity hotspot from Phase 0, recommend patterns:
   ```markdown
   **Hotspot:** Audio crossfade timing
   **Recommended Pattern:** Pipeline with lock-free ring buffer
   **Rationale:** Decouples decode from playback, maintains latency guarantees
   **Confidence:** HIGH - Pattern used successfully in similar audio systems

   **Hotspot:** Musical flavor calculation
   **Recommended Pattern:** Strategy + Memoization
   **Rationale:** Algorithm may change, results cacheable
   **Confidence:** MEDIUM - Reasonable fit, needs validation
   ```

**Output:**
- Pattern analysis section in plan documentation
- Recommended patterns per complexity hotspot
- Anti-patterns to explicitly avoid

**No User Checkpoint:** Phase 1.5 informs later phases

---

### PHASE 2: Specification Completeness Verification (Enhanced)

**Objective:** Identify specification gaps, ambiguities, and conflicts BEFORE planning implementation

**Extended Thinking Trigger:**

Before analyzing requirements, use extended thinking:
```
<extended_thinking>
For this specification, systematically consider:

1. IMPLICIT ASSUMPTIONS
   - What does this specification assume about the environment?
   - What error conditions are assumed to never happen?
   - What user behaviors are assumed?
   - What system states are assumed?

2. SEMANTIC CONSISTENCY
   - Are terms used consistently throughout?
   - Do examples match prose descriptions?
   - Are there contradictions between sections?

3. QUANTIFICATION GAPS
   - What timing requirements lack specific numbers?
   - What resource limits are unspecified?
   - What success criteria are unmeasurable?

4. INTERACTION EFFECTS
   - How do requirements interact with each other?
   - Are there emergent behaviors not explicitly addressed?
   - What happens when multiple requirements apply simultaneously?
</extended_thinking>
```

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

2. **Semantic Ambiguity Analysis (Enhanced)**

   Beyond keyword detection, analyze semantics:

   **Keyword Ambiguity (Traditional):**
   - Vague language ("appropriate," "reasonable," "good," "fast," "efficient")
   - Unquantified requirements ("quickly," "accurately," "minimal")
   - Undefined terms (jargon without definition)

   **Semantic Ambiguity (New):**
   - **Implicit Assumptions:** What is assumed but not stated?
     - Confidence: [CERTAIN/HIGH/MEDIUM/LOW/SPECULATIVE]
   - **Interpretation Variance:** Could two engineers implement differently?
     - Confidence: [CERTAIN/HIGH/MEDIUM/LOW/SPECULATIVE]
   - **Context Dependency:** Does meaning change based on context?
     - Confidence: [CERTAIN/HIGH/MEDIUM/LOW/SPECULATIVE]
   - **Boundary Ambiguity:** Are edge cases clearly defined?
     - Confidence: [CERTAIN/HIGH/MEDIUM/LOW/SPECULATIVE]

   **Ambiguity Test:** Could two reasonable engineers implement this differently and both claim compliance?
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

6. **Issues Prioritization with Confidence**

   Classify all issues found with confidence levels:

   | Severity | Confidence | Example |
   |----------|------------|---------|
   | **CRITICAL** | CERTAIN | Core requirement missing - document explicitly lacks information |
   | **CRITICAL** | HIGH | Undefined interface - strongly implied but not documented |
   | **HIGH** | CERTAIN | Ambiguous requirement - multiple valid interpretations exist |
   | **HIGH** | MEDIUM | Timing constraint may be unachievable - needs verification |
   | **MEDIUM** | HIGH | Error handling undefined - common pattern suggests gap |
   | **MEDIUM** | LOW | Possible edge case not covered - may not be relevant |
   | **LOW** | Any | Minor formatting, missing minor details |

**Auto-/think Trigger:**

If Phase 2 discovers:
- 5+ Critical issues, OR
- 10+ High issues, OR
- Unclear architecture/approach, OR
- Novel/risky technical elements

**Then:**
1. STOP /plan2 execution
2. Inform user: "Specification complexity requires deeper analysis"
3. Formulate focused /think query based on issues found
4. Execute /think automatically
5. Present /think results to user
6. **CHECKPOINT:** User reviews /think analysis and approves approach
7. Resume /plan2 with /think insights informing decisions

**Outputs:**
- `01_specification_issues.md` - All issues found, prioritized by severity with confidence levels
- Issues grouped by: type (ambiguity, missing, conflict) and affected requirement

**Decision Point:**
- If CRITICAL or 5+ HIGH issues: **STOP** - Require specification updates before continuing
- If only MEDIUM/LOW issues: Note for tracking, continue to Phase 3
- Present issues report to user for review and decision

**Success Criteria:**
- Every requirement analyzed for completeness, ambiguity, testability
- Issues clearly documented with specific resolution recommendations
- Confidence levels assigned to all findings
- User understands specification status and approves proceeding

---

### PHASE 3: Acceptance Test Definition (Enhanced)

**Objective:** Define explicit, executable tests that will verify each requirement is met

**Philosophy:** Tests ARE executable specifications. If we can't define the test, the requirement is ambiguous.

**Activities:**

1. **Test Type Determination**

   For each requirement, determine test types needed:
   - **Unit Tests:** Individual components/functions
   - **Integration Tests:** Component interactions (e.g., HTTP API calls between microservices)
   - **System Tests:** End-to-end scenarios (e.g., full playback pipeline with crossfading)
   - **Manual Tests:** When automation not feasible (document procedure)

2. **Enhanced Test Specification with Opus 4.5 Reasoning**

   For each requirement, apply systematic test generation:

   **A. Boundary Analysis:**
   ```markdown
   For input X:
   - Minimum valid value: [value] → Test: TC-U-XXX-01
   - Maximum valid value: [value] → Test: TC-U-XXX-02
   - Just below minimum: [value] → Test: TC-U-XXX-03 (expect rejection)
   - Just above maximum: [value] → Test: TC-U-XXX-04 (expect rejection)
   - Empty/null/zero: [value] → Test: TC-U-XXX-05
   ```

   **B. State Transition Coverage:**
   ```markdown
   System states: [S1, S2, S3, ...]
   Valid transitions:
   - S1 → S2: Condition [X] → Test: TC-I-XXX-01
   - S2 → S3: Condition [Y] → Test: TC-I-XXX-02
   Invalid transitions:
   - S1 → S3: Should fail → Test: TC-I-XXX-03 (expect error)
   ```

   **C. Concurrency Edge Cases (Rust/Tokio):**
   ```markdown
   Concurrent access scenarios:
   - Two tasks access same resource: → Test: TC-I-XXX-04
   - Operation cancelled mid-execution: → Test: TC-I-XXX-05
   - Dependent service unavailable: → Test: TC-I-XXX-06
   - Race condition: [describe] → Test: TC-I-XXX-07
   ```

   **D. Resource Exhaustion:**
   ```markdown
   Resource limits:
   - Memory pressure: → Test: TC-S-XXX-01 (graceful degradation)
   - Disk full: → Test: TC-S-XXX-02 (appropriate error)
   - Network timeout: → Test: TC-S-XXX-03 (retry/fail gracefully)
   - Connection pool exhausted: → Test: TC-S-XXX-04
   ```

3. **Unit Test Specification (BDD Format)**

   ```markdown
   ### REQ-XXX: [Requirement Title]

   **Unit Test: TC-U-XXX-01**
   - **Test Type:** Unit Test
   - **Scope:** [Component/Function under test]
   - **Category:** [Happy Path / Boundary / Error / Edge Case]
   - **Given:** [Initial conditions/setup]
   - **When:** [Action/input]
   - **Then:** [Expected result]
   - **Verify:** [Specific assertions]
   - **Pass Criteria:** [Measurable success condition]
   - **Fail Criteria:** [What constitutes failure]
   - **Confidence:** [CERTAIN/HIGH/MEDIUM] that test covers requirement

   **Estimated Effort:** [X minutes to write + implement]
   ```

4. **Integration Test Specification**

   ```markdown
   **Integration Test: TC-I-XXX-01**
   - **Test Type:** Integration Test
   - **Scope:** [Microservices/interfaces under test]
   - **Category:** [Communication / State Sync / Error Propagation]
   - **Setup:** [System configuration required]
   - **Given:** [Initial system state]
   - **When:** [Cross-component interaction (HTTP/SSE)]
   - **Then:** [Expected system behavior]
   - **Verify:** [Interface contracts, data flow verified]
   - **Pass Criteria:** [Measurable success]
   - **Confidence:** [CERTAIN/HIGH/MEDIUM] that test covers requirement
   ```

5. **System Test Specification**

   ```markdown
   **System Test: TC-S-XXX-01**
   - **Test Type:** System/End-to-End Test
   - **Environment:** [Hardware/software configuration]
   - **Scenario:** [User/system interaction description]
   - **Category:** [User Flow / Performance / Reliability]
   - **Given:** [System initial state]
   - **When:** [User actions or system events]
   - **Then:** [Observable system behavior]
   - **Verify:** [End-to-end validation]
   - **Pass Criteria:** [User-visible success]
   - **Confidence:** [CERTAIN/HIGH/MEDIUM] that test covers requirement
   ```

6. **Test Data Specification**

   For each test, identify:
   - Input data required (values, files, databases)
   - Expected output data (exact values or ranges)
   - Test data generation method (manual, generated, recorded)
   - Test data storage location

7. **Traceability Matrix Creation**

   Create comprehensive table linking requirements → tests → implementation:

   ```markdown
   | Requirement | Unit Tests | Integration Tests | System Tests | Implementation File(s) | Status | Coverage | Confidence |
   |-------------|------------|-------------------|--------------|------------------------|--------|----------|------------|
   | REQ-CF-010 | TC-U-010-01, TC-U-010-02, TC-U-010-03 | TC-I-010-01 | TC-S-010-01 | wkmp-ap/src/crossfade.rs | Pending | Complete | HIGH |
   | REQ-PD-015 | TC-U-015-01 | TC-I-015-01 | TC-S-015-01 | wkmp-pd/src/selector.rs | Pending | Partial | MEDIUM |
   ```

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

**Recommendation:** Use Option B (modular) for /plan2 output - establishes good pattern

**Outputs:**
- `02_test_specifications/` folder containing:
  - `test_index.md` - Compact table of all tests with categories
  - `tc_u_xxx_yy.md` - Individual unit test specifications
  - `tc_i_xxx_yy.md` - Individual integration test specifications
  - `tc_s_xxx_yy.md` - Individual system test specifications
  - `traceability_matrix.md` - Requirements ↔ Tests mapping with confidence

**Success Criteria:**
- Every requirement has at least one acceptance test
- Every test is specific and executable (not "verify it works")
- Tests cover: happy path, boundaries, error cases, edge cases, concurrency
- Traceability matrix shows no gaps (100% coverage)
- Confidence levels assigned to all coverage assessments

---

### PHASE 3.5: Traceability Matrix Validation (NEW)

**Objective:** Systematically verify the traceability matrix is complete and well-formed

**Trigger:** Execute immediately after Phase 3, before user checkpoint

**Activities:**

1. **Completeness Check**

   ```markdown
   **Validation: Every Requirement Has Tests**

   | Requirement | Has Unit Test? | Has Integration/System? | Status |
   |-------------|----------------|------------------------|--------|
   | REQ-CF-010 | ✅ TC-U-010-01 | ✅ TC-I-010-01 | PASS |
   | REQ-PD-015 | ✅ TC-U-015-01 | ❌ None | FAIL - Add integration test |

   **Result:** [N]/[Total] requirements fully covered
   **Action Required:** [List requirements needing additional tests]
   ```

2. **Redundancy Check**

   ```markdown
   **Validation: Test Coverage Efficiency**

   **Over-Tested Requirements (>5 tests):**
   - REQ-XXX: 7 tests - Review for redundancy
     - Recommendation: [Keep/Consolidate]

   **Broad Tests (covering >3 requirements):**
   - TC-S-001-01: Covers REQ-001, REQ-002, REQ-003, REQ-004
     - Risk: Failure doesn't pinpoint which requirement is broken
     - Recommendation: Split into focused tests

   **Result:** [N] over-tested, [M] overly-broad tests identified
   ```

3. **Coverage Quality Check**

   ```markdown
   **Validation: Test Category Coverage**

   | Requirement | Happy Path | Boundary | Error | Edge | Concurrency | Quality |
   |-------------|------------|----------|-------|------|-------------|---------|
   | REQ-CF-010 | ✅ | ✅ | ✅ | ✅ | ✅ | EXCELLENT |
   | REQ-PD-015 | ✅ | ✅ | ❌ | ❌ | N/A | ADEQUATE |
   | REQ-UI-020 | ✅ | ❌ | ❌ | ❌ | N/A | POOR - Add tests |

   **Coverage Summary:**
   - Excellent (all categories): [N] requirements
   - Adequate (happy + boundaries): [M] requirements
   - Poor (happy path only): [P] requirements

   **Action Required:** Improve coverage for POOR requirements
   ```

4. **Implementation Column Validation**

   ```markdown
   **Validation: Implementation Locations**

   | Requirement | Implementation File | File Exists? | Plausible? |
   |-------------|---------------------|--------------|------------|
   | REQ-CF-010 | wkmp-ap/src/crossfade.rs | ✅ Yes | ✅ Yes |
   | REQ-PD-015 | wkmp-pd/src/selector.rs | ✅ Yes | ✅ Yes |
   | REQ-NEW-001 | TBD | N/A | ⚠️ Needs assignment |

   **TBD Entries:** [N] - Must be resolved by Phase 5
   **Non-Existent Files:** [M] - Will be created during implementation
   ```

5. **Self-Verification Summary**

   ```markdown
   ## Phase 3.5 Self-Verification Results

   **Completeness:** [PASS/FAIL] - [N]/[Total] requirements have tests
   **Redundancy:** [PASS/WARN] - [N] potential issues identified
   **Quality:** [PASS/WARN/FAIL] - [N] POOR coverage requirements
   **Implementation:** [PASS/WARN] - [N] TBD entries remaining

   **Overall Status:** [READY TO PROCEED / NEEDS REMEDIATION]

   **If NEEDS REMEDIATION:**
   - Add [N] missing tests before proceeding
   - Review [M] redundant tests
   - Improve [P] poor-quality test suites
   ```

**Output:**
- `02_test_specifications/validation_report.md` - Validation results
- Updated `traceability_matrix.md` if issues found and corrected

**Decision Point:**
- If validation PASS: Continue to checkpoint
- If validation FAIL: Remediate issues, re-run validation

**User Checkpoint:** Present test coverage summary with validation results, confirm comprehensive before proceeding

---

### PHASE 4: Approach Selection (Enhanced)

**Objective:** Choose implementation approach with minimal failure risk using structured analysis

**Extended Thinking Trigger:**

Before comparing approaches, use extended thinking:
```
<extended_thinking>
For each candidate approach, systematically analyze:

1. FAILURE MODE ENUMERATION
   - What can go wrong with this approach?
   - What are the technical risks?
   - What dependencies could fail?
   - What assumptions might be wrong?

2. CASCADING FAILURE ANALYSIS
   - If component X fails, what else fails?
   - What is the blast radius of each failure mode?
   - Are failures contained or do they propagate?

3. APPROACH INTERACTIONS
   - How does this approach interact with existing architecture?
   - Does it introduce new coupling?
   - Does it simplify or complicate future changes?

4. HIDDEN COMPLEXITY
   - What looks simple but is actually hard?
   - What edge cases does this approach handle poorly?
   - What maintenance burden does this create?
</extended_thinking>
```

**Activities:**

1. **Identify Viable Approaches**

   Identify 2-3 approaches that could satisfy requirements:
   ```markdown
   **Approach A: [Name]**
   - Description: [How it works]
   - Key characteristics: [Strengths]

   **Approach B: [Name]**
   - Description: [How it works]
   - Key characteristics: [Strengths]

   **Approach C: [Name]** (if applicable)
   - Description: [How it works]
   - Key characteristics: [Strengths]
   ```

2. **Structured Failure Mode Analysis (Enhanced)**

   For each approach, complete this analysis:

   ```markdown
   ### Approach A: Failure Mode Analysis

   **FM-A-01: [Failure Mode Name]**
   - **What Fails:** [Specific component/behavior]
   - **How It Manifests:** [Silent / Error message / Crash / Corruption]
   - **Trigger Conditions:** [What causes this failure]
   - **Probability:** [CERTAIN/HIGH/MEDIUM/LOW/SPECULATIVE] - [Rationale]
   - **Impact:** [Catastrophic / Major / Moderate / Minor]
   - **Blast Radius:** [What else fails if this fails]
   - **Detection:** [How quickly detected, by what mechanism]
   - **Recovery:** [Automatic / Manual, time to recover, data loss?]
   - **Prevention:** [Design choices that prevent this]
   - **Mitigation:** [How to reduce impact if it occurs]
   - **Residual Risk:** [After mitigation: LOW/MEDIUM/HIGH]

   **FM-A-02: [Next Failure Mode]**
   ...
   ```

3. **Approach Risk Summary**

   ```markdown
   | Approach | Failure Modes | Highest Risk | Residual Risk | Confidence |
   |----------|---------------|--------------|---------------|------------|
   | A | FM-A-01, FM-A-02, FM-A-03 | FM-A-01 (Major) | MEDIUM | HIGH |
   | B | FM-B-01, FM-B-02 | FM-B-02 (Moderate) | LOW | HIGH |
   | C | FM-C-01 | FM-C-01 (Minor) | LOW | MEDIUM |
   ```

4. **Quality Characteristics Comparison**

   ```markdown
   | Quality Attribute | Approach A | Approach B | Approach C |
   |-------------------|------------|------------|------------|
   | Maintainability | Medium | High | High |
   | Test Coverage Achievable | 80% | 95% | 90% |
   | Architecture Alignment | Poor | Good | Good |
   | Code Complexity | High | Medium | Low |
   | Future Extensibility | Limited | Good | Excellent |
   ```

5. **Effort Comparison**

   ```markdown
   | Effort Dimension | Approach A | Approach B | Approach C |
   |------------------|------------|------------|------------|
   | Implementation Hours | 20-25 | 35-45 | 40-50 |
   | Test Hours | 10-15 | 8-12 | 10-15 |
   | Documentation Hours | 3-5 | 5-8 | 5-8 |
   | **Total** | **33-45** | **48-65** | **55-73** |
   | Confidence | MEDIUM (±40%) | HIGH (±25%) | MEDIUM (±35%) |
   ```

6. **Decision and Justification (ADR Format)**

   ```markdown
   ## Architecture Decision Record: [Feature] Implementation Approach

   **Status:** Proposed
   **Date:** [YYYY-MM-DD]

   **Context:**
   [Why this decision is needed, what problem we're solving]

   **Decision:**
   Recommend **Approach B** ([Name])

   **Risk-Based Justification:**

   Approach B has lowest residual risk (LOW) after mitigation:
   - Failure modes: [List with mitigations]
   - Residual risk after mitigation: LOW
   - Confidence in assessment: HIGH

   Approach A has higher residual risk (MEDIUM):
   - [Specific failure modes that couldn't be fully mitigated]

   Approach C has equivalent risk (LOW) but:
   - Lower confidence in assessment (MEDIUM)
   - Higher effort without proportional benefit

   **Quality Assessment:**
   Approach B scores highest on maintainability and architecture alignment.

   **Effort Acknowledgment:**
   Approach B requires ~50 hours vs. Approach A's ~40 hours.
   The ~10-hour effort differential is acceptable given risk reduction.

   Per CLAUDE.md Decision-Making Framework: Risk (primary) → Quality (secondary) → Effort (tertiary).

   **Consequences:**
   - Positive: [Benefits of chosen approach]
   - Negative: [Tradeoffs accepted]
   - Risks: [Remaining risks and monitoring approach]
   ```

**Output:** `03_approach_selection.md` including:
- All approaches described
- Complete failure mode analysis for each
- Risk comparison table
- Quality comparison table
- Effort comparison with confidence ranges
- ADR with risk-based justification

**User Checkpoint (Standard/Complex plans):** Present approach comparison, confirm selection before proceeding

---

### PHASE 5: Implementation Breakdown (Enhanced)

**Objective:** Decompose implementation into small, manageable, verifiable increments

**Activities:**

1. **Component Decomposition**

   Break system into logical components based on:
   - Functional boundaries
   - Dependency relationships
   - Risk containment (high-risk in early increments)
   - Test isolation

2. **Increment Definition**

   Define increments targeting 2-4 hours each:

   ```markdown
   # Increment N: [Name]

   **Increment:** N of M
   **Phase:** [Implementation/Testing/Integration]
   **Estimated Effort:** X-Y hours
   **Confidence:** [HIGH ±20% / MEDIUM ±40% / LOW ±60%]
   **Prerequisites:** [Previous increments, setup required]

   ## Objective
   [What this increment achieves - single clear goal]

   ## Deliverables
   1. [Deliverable 1 - specific file or component]
   2. [Deliverable 2]

   ## Requirements Covered
   - REQ-XXX-01 (partial/complete)
   - REQ-XXX-02 (partial/complete)

   ## Tests to Pass
   - TC-U-XXX-01: [Brief description]
   - TC-U-XXX-02: [Brief description]

   ## Acceptance Criteria
   - [ ] [Specific, verifiable criterion 1]
   - [ ] [Specific, verifiable criterion 2]
   - [ ] All listed tests pass

   ## Implementation Notes
   [Guidance, patterns to use, pitfalls to avoid]

   ## Dependencies
   - Requires: [Increments that must complete first]
   - Enables: [Increments that can start after this]
   ```

3. **Dependency Analysis**

   ```markdown
   ## Increment Dependencies

   ```mermaid
   graph TD
       I1[Increment 1: Foundation] --> I2[Increment 2: Core Logic]
       I1 --> I3[Increment 3: Data Layer]
       I2 --> I4[Increment 4: Integration]
       I3 --> I4
       I4 --> I5[Increment 5: Testing]
   ```

   **Critical Path:** I1 → I2 → I4 → I5 (16-22 hours)
   **Parallelizable:** I2 and I3 can run in parallel after I1
   ```

4. **Checkpoint Placement**

   Define checkpoints per Adaptive Checkpoint System:
   ```markdown
   **Checkpoint 1:** After Increment 3
   - Verify: Foundation complete, core components functional
   - Tests passing: TC-U-001-* through TC-U-015-*
   - Decision: Continue / Adjust plan / Stop

   **Checkpoint 2:** After Increment 7
   - Verify: Integration complete, end-to-end flow works
   - Tests passing: All unit + integration tests
   - Decision: Continue to system testing / Fix issues
   ```

5. **Self-Verification: Increment Coverage**

   ```markdown
   ## Phase 5 Self-Verification

   **Requirements → Increments Mapping:**

   | Requirement | Increment(s) | Coverage |
   |-------------|--------------|----------|
   | REQ-CF-010 | I2, I4 | Complete |
   | REQ-PD-015 | I3, I4, I5 | Complete |
   | REQ-UI-020 | I6 | Complete |

   **Verification Results:**
   - All requirements mapped: [YES/NO]
   - Orphan increments (no requirements): [List or None]
   - Requirements without increments: [List or None]

   **Increment Size Validation:**
   | Increment | Estimated Hours | Within Target (2-4h)? |
   |-----------|-----------------|----------------------|
   | I1 | 3-4 | ✅ Yes |
   | I2 | 4-6 | ⚠️ Slightly over - consider splitting |
   | I3 | 2-3 | ✅ Yes |

   **Dependency Validation:**
   - Circular dependencies: [None / List]
   - Missing dependencies: [None / List]
   ```

**Output:** `04_increments/` folder with:
- `increment_index.md` - Overview table
- `increment_01.md` through `increment_NN.md` - Individual increment specs (<300 lines each)
- `dependencies.md` - Dependency graph and analysis
- `checkpoints.md` - Checkpoint criteria

**Success Criteria:**
- Every requirement covered by at least one increment
- No circular dependencies
- All increments 2-4 hours (or justified exceptions)
- Clear checkpoint criteria defined
- Self-verification passed

---

### PHASE 6: Effort and Schedule Estimation (Enhanced)

**Objective:** Provide realistic estimates with explicit confidence ranges

**Activities:**

1. **Per-Increment Estimation**

   ```markdown
   | Increment | Low Estimate | Expected | High Estimate | Confidence | Basis |
   |-----------|--------------|----------|---------------|------------|-------|
   | I1 | 2h | 3h | 4h | HIGH (±25%) | Similar to past work |
   | I2 | 3h | 4.5h | 7h | MEDIUM (±40%) | Some unknowns |
   | I3 | 4h | 6h | 10h | LOW (±50%) | Novel component |
   ```

2. **Aggregate Estimation**

   ```markdown
   **Total Effort Summary:**

   | Scenario | Hours | Probability |
   |----------|-------|-------------|
   | Optimistic (all goes well) | 25h | 10% |
   | Expected (normal issues) | 35h | 60% |
   | Pessimistic (significant issues) | 50h | 25% |
   | Worst Case (major problems) | 70h | 5% |

   **Recommended Buffer:** 30% contingency on expected
   **Planning Estimate:** 35h + 10h buffer = 45h
   ```

3. **Risk-Adjusted Estimation**

   For each HIGH/CRITICAL risk from Phase 7:
   ```markdown
   **Risk Impact on Estimates:**

   | Risk | Probability | Impact if Occurs | Expected Impact |
   |------|-------------|------------------|-----------------|
   | R-001 | 20% | +15h | +3h |
   | R-002 | 10% | +8h | +0.8h |
   | R-003 | 30% | +5h | +1.5h |

   **Total Risk Adjustment:** +5.3h
   **Risk-Adjusted Estimate:** 45h + 5h = 50h
   ```

4. **Resource Requirements**

   ```markdown
   **Resource Needs:**

   - Developer time: 50h (1 FTE × 1.5 weeks)
   - Test environment: [Requirements]
   - External dependencies: [List with lead times]
   - Review/approval time: ~5h
   ```

**Output:** `05_estimates.md` with:
- Per-increment estimates with confidence
- Aggregate estimates with scenarios
- Risk adjustments
- Resource requirements

---

### PHASE 7: Risk Assessment and Mitigation Planning (Enhanced)

**Objective:** Systematically identify and plan for implementation risks

**Extended Thinking Trigger:**

Use extended thinking for risk cascade analysis:
```
<extended_thinking>
For each identified risk, analyze the failure chain:

1. PRIMARY FAILURE
   - What is the direct consequence of this risk materializing?
   - Who/what is immediately affected?

2. SECONDARY EFFECTS
   - What other components depend on the failed component?
   - How does the failure propagate through the system?
   - What deadlines or commitments are affected?

3. TERTIARY EFFECTS
   - What downstream processes are blocked?
   - What workarounds become necessary?
   - What technical debt is incurred?

4. RECOVERY ANALYSIS
   - What's the minimum recovery path?
   - What's lost that can't be recovered?
   - How long until normal operation resumes?
</extended_thinking>
```

**Activities:**

1. **Risk Identification**

   Categories to consider:
   - **Technical Risks:** Architecture, complexity, dependencies
   - **Schedule Risks:** Estimate uncertainty, external dependencies
   - **Resource Risks:** Skills, availability, tools
   - **Integration Risks:** Existing systems, data migration
   - **Quality Risks:** Test coverage, performance, security

2. **Risk Analysis with Failure Chains**

   ```markdown
   ### RISK-001: [Risk Name]

   **Category:** [Technical / Schedule / Resource / Integration / Quality]
   **Description:** [What could go wrong]

   **Probability Assessment:**
   - Level: [CERTAIN / HIGH / MEDIUM / LOW / SPECULATIVE]
   - Rationale: [Why this probability]
   - Confidence in assessment: [HIGH / MEDIUM / LOW]

   **Impact Assessment:**
   - Level: [Catastrophic / Major / Moderate / Minor]
   - Primary Impact: [Direct consequence]
   - Blast Radius: [What else is affected]

   **Failure Chain Analysis:**
   ```
   [Component A fails]
        ↓
   [Component B receives bad data]
        ↓
   [Component C makes wrong decision]
        ↓
   [User sees incorrect behavior]
        ↓
   [Trust in system degraded]
   ```

   **Detection:**
   - How detected: [Monitoring / Test failure / User report]
   - Detection latency: [Immediate / Minutes / Hours / Days]

   **Mitigation Strategy:**
   - Prevention: [How to prevent occurrence]
   - Detection: [How to detect early]
   - Response: [What to do if it occurs]
   - Recovery: [How to restore normal operation]

   **Residual Risk:**
   - After mitigation: [LOW / LOW-MEDIUM / MEDIUM / MEDIUM-HIGH / HIGH]
   - Confidence: [HIGH / MEDIUM / LOW]

   **Monitoring:**
   - Indicators to watch: [Metrics, logs, alerts]
   - Escalation threshold: [When to escalate]
   ```

3. **Risk Matrix**

   ```markdown
   ## Risk Matrix

   |            | Minor | Moderate | Major | Catastrophic |
   |------------|-------|----------|-------|--------------|
   | CERTAIN    |       |          |       |              |
   | HIGH       |       | R-003    | R-001 |              |
   | MEDIUM     | R-005 | R-002    |       |              |
   | LOW        | R-006 |          | R-004 |              |
   | SPECULATIVE|       |          |       |              |

   **Critical Risks (require immediate mitigation):** R-001
   **High Priority Risks:** R-002, R-003
   **Monitor Risks:** R-004, R-005, R-006
   ```

4. **Mitigation Plan Summary**

   ```markdown
   ## Mitigation Actions

   | Risk | Mitigation Action | Owner | Deadline | Status |
   |------|-------------------|-------|----------|--------|
   | R-001 | Add retry logic with exponential backoff | Dev | I3 | Planned |
   | R-002 | Create fallback data source | Dev | I2 | Planned |
   | R-003 | Add comprehensive input validation | Dev | I1 | Planned |
   ```

**Output:** `06_risks.md` with:
- Complete risk inventory
- Failure chain analysis for each significant risk
- Risk matrix visualization
- Mitigation plan with ownership
- Monitoring recommendations

---

### PHASE 7.5: Pre-Implementation Simulation (NEW)

**Objective:** Mentally simulate the implementation to discover hidden complexity before coding

**Extended Thinking Trigger:**

Use extended thinking to simulate implementation:
```
<extended_thinking>
For each increment, mentally walk through the implementation:

1. CODE FLOW SIMULATION
   - What files will I create/modify?
   - What functions will I write?
   - What data structures do I need?
   - Where will I need to make decisions not covered by the spec?

2. INTEGRATION SIMULATION
   - How does this connect to existing code?
   - What interfaces need to change?
   - What backward compatibility concerns exist?
   - How do I handle the transition?

3. TEST SIMULATION
   - Can I actually write the tests as specified?
   - What test infrastructure is needed?
   - What mocking/stubbing is required?
   - Are the test assertions verifiable?

4. DIFFICULTY ASSESSMENT
   - What looks simple but is actually hard?
   - What requires expertise I don't have?
   - What requires coordination with others?
   - What has been underestimated?
</extended_thinking>
```

**Activities:**

1. **Increment-by-Increment Simulation**

   For each increment, document:

   ```markdown
   ### Increment N: Pre-Implementation Simulation

   **Code Changes Anticipated:**
   - Files to create: [List]
   - Files to modify: [List with estimated line changes]
   - Files to delete/deprecate: [List]

   **Implementation Decisions Required:**
   | Decision Point | Options | Recommendation | Confidence |
   |----------------|---------|----------------|------------|
   | Error handling strategy | Result vs panic | Result | HIGH |
   | Data structure for X | Vec vs HashMap | HashMap (O(1) lookup) | MEDIUM |

   **Integration Concerns:**
   - [Concern 1 and mitigation]
   - [Concern 2 and mitigation]

   **Test Infrastructure Needs:**
   - [Mock/stub requirements]
   - [Test data requirements]
   - [Environment requirements]

   **Difficulty Assessment:**
   - Estimated difficulty: [Easy / Moderate / Hard / Very Hard]
   - Confidence in estimate: [HIGH / MEDIUM / LOW]
   - "Harder than it looks" factors: [List any]
   ```

2. **Hidden Complexity Catalog**

   ```markdown
   ## Hidden Complexity Discovery

   **Items Flagged as "Harder Than They Look":**

   | Item | Why It's Hard | Impact | Mitigation |
   |------|---------------|--------|------------|
   | REQ-CF-010 | Requires lock-free ring buffer | +8h | Use proven library |
   | Increment 4 | Touches 5 existing modules | +4h | Careful coordination |
   | TC-I-015-01 | Requires async test harness | +2h | Research upfront |

   **Specification Gaps Discovered:**
   | Gap | Location | Impact | Resolution |
   |-----|----------|--------|------------|
   | Unspecified timeout | REQ-PD-015 | May block indefinitely | Add 30s default |
   | Missing error code | REQ-UI-020:45 | Client can't distinguish errors | Add specific codes |
   ```

3. **Estimate Revision**

   Based on simulation, revise estimates if needed:
   ```markdown
   ## Estimate Revisions from Simulation

   | Increment | Original | Revised | Change | Reason |
   |-----------|----------|---------|--------|--------|
   | I3 | 3-4h | 5-7h | +2h | Hidden complexity in error handling |
   | I5 | 2-3h | 2-3h | No change | Straightforward as expected |

   **Total Adjustment:** +X hours
   **Revised Total Estimate:** Y hours (was Z hours)
   ```

4. **Self-Verification: Plan Coherence**

   ```markdown
   ## Cross-Phase Coherence Check

   **Phase 1 Requirements → Phase 3 Tests → Phase 5 Increments:**
   - All requirements have tests: [YES/NO]
   - All requirements have increments: [YES/NO]
   - All tests have increments that implement them: [YES/NO]

   **Phase 4 Approach → Phase 7 Risks:**
   - Selected approach risks identified in Phase 7: [YES/NO]
   - Risk mitigations incorporated in increments: [YES/NO]

   **Phase 6 Estimates → Phase 7.5 Revisions:**
   - Estimates revised based on simulation: [YES/NO]
   - Major discrepancies explained: [YES/NO]

   **Coherence Status:** [COHERENT / NEEDS RECONCILIATION]
   ```

**Output:**
- `07_simulation.md` - Pre-implementation simulation results
- Updated estimates if significant changes discovered
- List of specification gaps for user review

**Decision Point:**
- If significant gaps discovered: Present to user, may need specification update
- If estimate changed >30%: Highlight and explain
- If coherence check fails: Reconcile before proceeding

---

### PHASE 8: Plan Documentation and Approval (Enhanced)

**Objective:** Generate comprehensive yet scannable plan documentation

**Activities:**

1. **Generate 00_PLAN_SUMMARY.md**

   Use the enhanced template (see Plan Document Structure section)
   - Include all confidence levels
   - Include self-verification results
   - Include simulation findings

2. **Generate FULL_PLAN.md**

   Consolidate all sections for archival reference
   - Include phase outputs inline
   - Add cross-references
   - Mark as "archival only - do not use for implementation"

3. **Final Self-Verification**

   ```markdown
   ## Final Plan Verification Checklist

   **Phase Completion:**
   - [x] Phase 0: Extended Thinking Initialization
   - [x] Phase 1: Input Validation and Scope Definition
   - [x] Phase 1.5: Architectural Pattern Analysis
   - [x] Phase 2: Specification Completeness Verification
   - [x] Phase 3: Acceptance Test Definition
   - [x] Phase 3.5: Traceability Matrix Validation
   - [x] Phase 4: Approach Selection
   - [x] Phase 5: Implementation Breakdown
   - [x] Phase 6: Effort and Schedule Estimation
   - [x] Phase 7: Risk Assessment and Mitigation
   - [x] Phase 7.5: Pre-Implementation Simulation
   - [x] Phase 8: Plan Documentation

   **Quality Gates:**
   - [x] All requirements have tests (100% traceability)
   - [x] All requirements have increments
   - [x] No CRITICAL specification issues unresolved
   - [x] Risk mitigations incorporated
   - [x] Estimates include confidence ranges
   - [x] Cross-phase coherence verified

   **Document Quality:**
   - [x] 00_PLAN_SUMMARY.md < 500 lines
   - [x] Individual increments < 300 lines
   - [x] Test specs modular (< 100 lines each)
   ```

**Output:**
- `00_PLAN_SUMMARY.md` - Executive summary (< 500 lines)
- `FULL_PLAN.md` - Complete consolidated plan (archival only)

**User Checkpoint:** Present final plan for approval

---

### PHASE 9: Post-Implementation Review and Technical Debt Assessment

**Purpose:** Systematically discover and document technical debt after implementation

**Note:** Phase 9 is inherited from /plan without modification. See /plan documentation for complete Phase 9 specification including:
- 7-step technical debt discovery process
- Technical debt report template
- Deferred requirements tracking
- Integration with plan completion report

**Critical Rule:** Phase 9 is MANDATORY for ALL implementation plans. No exceptions.

---

## Adaptive Checkpoint System

Checkpoints are adjusted based on plan complexity:

### Simple Plans (<10 requirements, no CRITICAL issues)

```markdown
**Checkpoint Strategy: MINIMAL**

Checkpoints:
1. After Phase 3.5 (combined Phases 1-3 review)
   - Scope correct?
   - Tests comprehensive?
   - Ready to proceed?

2. After Phase 8 (combined Phases 4-8 review)
   - Approach acceptable?
   - Plan approved?

Skipped Checkpoints:
- Phase 1 (scope) - combined into Checkpoint 1
- Phase 2 (issues) - combined into Checkpoint 1
- Phase 4 (approach) - combined into Checkpoint 2
```

### Standard Plans (10-30 requirements, ≤5 HIGH issues)

```markdown
**Checkpoint Strategy: STANDARD**

Checkpoints:
1. After Phase 2: Specification issues review
   - Are issues acceptable?
   - Need specification updates?

2. After Phase 3.5: Test coverage review
   - Coverage complete?
   - Quality adequate?

3. After Phase 8: Final plan approval
   - Plan approved for implementation?

Skipped Checkpoints:
- Phase 1 (proceed if no obvious issues)
- Phase 4 (proceed if approach selection is clear)
```

### Complex Plans (>30 requirements OR >5 HIGH issues OR CRITICAL issues)

```markdown
**Checkpoint Strategy: COMPREHENSIVE**

Checkpoints:
1. After Phase 1: Scope validation
2. After Phase 2: Issue resolution
3. After Phase 3.5: Test coverage validation
4. After Phase 4: Approach approval
5. After Phase 5: Increment review
6. After Phase 7.5: Simulation findings review
7. After Phase 8: Final approval

No Skipped Checkpoints - full review at each phase

Additional Measures:
- Consider breaking into sub-plans
- Add mid-increment checkpoints for high-risk increments
- Require explicit approval for each phase
```

---

## Plan Document Structure (Output)

### Modular Folder Architecture

**Location:** `wip/PLAN###_[feature_name]/`

```
wip/PLAN###_[feature_name]/
├── 00_PLAN_SUMMARY.md                  # <500 lines - READ THIS FIRST
├── 01_specification_issues.md           # Phase 2 output with confidence levels
├── 02_test_specifications/              # Phase 3 output (modular)
│   ├── test_index.md                    # Quick reference with categories
│   ├── tc_u_001_01.md                   # Individual test specs
│   ├── tc_i_001_01.md
│   ├── tc_s_001_01.md
│   ├── traceability_matrix.md           # With confidence column
│   └── validation_report.md             # Phase 3.5 output
├── 03_approach_selection.md             # Phase 4 with failure mode analysis
├── 04_increments/                       # Phase 5 output
│   ├── increment_index.md
│   ├── increment_01.md                  # <300 lines each
│   ├── increment_02.md
│   ├── dependencies.md
│   └── checkpoints.md
├── 05_estimates.md                      # Phase 6 with confidence ranges
├── 06_risks.md                          # Phase 7 with failure chains
├── 07_simulation.md                     # Phase 7.5 output
├── requirements_index.md                # Phase 1 output with complexity
├── scope_statement.md                   # Phase 1 output
├── dependencies_map.md                  # Phase 1 output
├── pattern_analysis.md                  # Phase 1.5 output
├── phase9_technical_debt_report.md      # Phase 9 output (post-implementation)
└── FULL_PLAN.md                         # Phase 8 consolidated (archival only)
```

### Document Size Targets

| Document | Target Size | When to Read |
|----------|-------------|--------------|
| 00_PLAN_SUMMARY.md | <500 lines | Always start here |
| Individual increment_XX.md | <300 lines | When implementing that increment |
| Individual tc_*.md | <100 lines | When implementing that test |
| traceability_matrix.md | <200 lines | Planning, verification |
| **Summary + increment** | **<800 lines** | **Typical implementation context** |
| FULL_PLAN.md | >2000 lines | Archival/review only |

---

## Integration with Existing Workflows

### Recommended Sequence

```
1. /think [requirements_doc] (if complex/novel)
   ↓ Analysis of requirements and approaches

2. Review /think analysis
   ↓ Decision on approach feasibility

3. /plan2 [specifications_doc]
   ↓ Phase 0: Extended thinking initialization
   ↓ Phases 1-2: Scope + Specification Verification
   ↓ CHECKPOINT: Resolve critical specification issues
   ↓ Phases 3-3.5: Test Definition + Validation
   ↓ CHECKPOINT: Review test coverage
   ↓ Phases 4-5: Approach + Implementation Breakdown
   ↓ Phases 6-7.5: Estimates + Risks + Simulation
   ↓ Phase 8: Documentation
   ↓ CHECKPOINT: Final approval

4. Review /plan2 output (read 00_PLAN_SUMMARY.md)
   ↓ Approval to proceed

5. Implement following plan, increment by increment
   ↓ For each increment:
   ↓   - Read increment_XX.md + relevant tests
   ↓   - Implement to pass tests
   ↓   - /commit after passing tests

6. At checkpoints: Review progress, verify tests pass

7. After implementation: Phase 9 technical debt discovery

8. /archive-plan PLAN### to clean up
```

---

## Workflow Constraints

### MUST DO:
- Execute Phase 0 extended thinking before analysis
- Use 5-level confidence scale throughout
- Complete self-verification at Phases 3.5, 5, and 7.5
- Apply semantic ambiguity analysis (not just keywords)
- Perform failure chain analysis for significant risks
- Run pre-implementation simulation
- Adapt checkpoints to plan complexity
- Maintain cross-phase coherence

### MUST NOT DO:
- Skip extended thinking triggers
- Omit confidence levels from findings
- Bypass self-verification steps
- Use only keyword-based ambiguity detection
- Skip failure chain analysis for HIGH/CRITICAL risks
- Finalize plan without simulation
- Use fixed checkpoints regardless of complexity

---

## Version and Status

**Version:** 1.0
**Status:** Production Ready
**Optimized For:** Claude Opus 4.5 with extended thinking
**Based On:** /plan workflow v1.0 with 10 optimizations
**Author:** WKMP Music Player Development Team
**Date:** 2025-12-13

**Key Enhancements Over /plan:**
1. Extended thinking integration (Phases 0, 2, 4, 7, 7.5)
2. Semantic ambiguity detection
3. Self-verification loops
4. Pre-implementation simulation
5. Failure mode analysis with chains
6. Architectural pattern recognition
7. Adaptive checkpoint system
8. Enhanced test generation
9. Traceability matrix validation
10. 5-level confidence indicators

**Related Commands:**
- `/plan` - Original planning workflow (unchanged)
- `/think` - Deep analysis before planning
- `/archive-plan` - Archive completed plans
