# Specification Issues: PLAN014 Mixer Refactoring

**Plan:** PLAN014_mixer_refactoring
**Date:** 2025-01-30
**Phase:** 2 - Specification Completeness Verification

---

## Executive Summary

**Total Issues Found:** 8
- **CRITICAL:** 0
- **HIGH:** 3
- **MEDIUM:** 4
- **LOW:** 1

**Decision:** PROCEED - No critical blockers found. High-priority issues can be resolved during implementation.

**Key Findings:**
- Requirements are generally well-specified
- No critical information gaps that block implementation
- Some ambiguities exist around feature porting conditions
- Test specifications need more detail on specific assertions
- No conflicts or contradictions detected

---

## Analysis Methodology

Per /plan Phase 2 workflow, each requirement analyzed for:
1. **Completeness:** Inputs, outputs, behavior, constraints, error cases, dependencies specified?
2. **Ambiguity:** Vague language, unquantified requirements, undefined terms?
3. **Consistency:** Cross-requirement contradictions, resource conflicts?
4. **Testability:** Can compliance be objectively verified?
5. **Dependency Validation:** Do dependencies exist and have stable interfaces?

Requirements processed in 3 batches (REQ-MIX-001/002, REQ-MIX-003-006, REQ-MIX-007-012).

---

## Issues by Requirement

### Batch 1: Investigation and Cleanup (REQ-MIX-001, REQ-MIX-002)

#### REQ-MIX-001: Determine Active Mixer

**Issue:** None - Requirement is complete and testable

**Completeness Check:**
- ✅ Input: Codebase, PlaybackEngine source
- ✅ Output: Determination of which mixer is active + evidence
- ✅ Behavior: Code inspection, grep for mixer instantiation
- ✅ Constraints: None
- ✅ Error cases: N/A (investigation, not runtime)
- ✅ Dependencies: Access to wkmp-ap source code

**Ambiguity Check:**
- ✅ No vague language
- ✅ Quantified: "which mixer" (binary: legacy OR correct)
- ✅ Defined terms: "active" means instantiated by PlaybackEngine

**Testability:**
- ✅ Pass criteria: Code shows `CrossfadeMixer::new()` from one file or the other
- ✅ Fail criteria: No mixer instantiation found, or both instantiated
- ✅ Evidence: File path + line number of instantiation

**Status:** No issues

---

#### REQ-MIX-002: Remove Inactive Mixer

**Issue HIGH-001:** Removal strategy depends on REQ-MIX-001 but "Scenario A" may be complex

**Completeness Check:**
- ✅ Input: Result from REQ-MIX-001
- ✅ Output: Single mixer in codebase
- ⚠️ Behavior: **Scenario A** (migrate then remove) needs more specification
- ✅ Constraints: Must verify no active references
- ✅ Error cases: References found → update them
- ✅ Dependencies: REQ-MIX-001 completion

**Ambiguity Check:**
- ⚠️ **Ambiguous:** "Keep legacy mixer temporarily" - how long is "temporary"?
- ⚠️ **Ambiguous:** "after migration verified" - what constitutes verification?
- ✅ Scenario B (correct mixer active) is clear

**Testability:**
- ⚠️ **Scenario A:** Pass criteria undefined (when is migration complete?)
- ✅ **Scenario B:** Pass criteria clear (legacy mixer archived/removed)

**Issue Details:**
- **Severity:** HIGH
- **Category:** Missing specification
- **Impact:** Scenario A (migrate from legacy to correct) is underspecified
- **Resolution:**
  - Add requirement: REQ-MIX-013 - Migrate from Legacy to Correct Mixer
  - Define migration steps, verification criteria, rollback plan
  - Clarify "temporary" means "until REQ-MIX-007/008/009 complete"

**Recommendation:** Add migration specification before implementing Scenario A

---

### Batch 2: Documentation (REQ-MIX-003, REQ-MIX-004, REQ-MIX-005, REQ-MIX-006)

#### REQ-MIX-003: Add [XFD-CURV-050] to SPEC002

**Issue:** None - Requirement is complete and testable

**Completeness Check:**
- ✅ Input: SPEC002-crossfade.md, exact text to add
- ✅ Output: SPEC002 with new [XFD-CURV-050] requirement
- ✅ Behavior: Insert text after [XFD-CURV-040], update revision history
- ✅ Constraints: Must follow GOV001 review process
- ✅ Error cases: [XFD-CURV-040] not found → report error
- ✅ Dependencies: SPEC002 file exists

**Ambiguity Check:**
- ✅ Exact location specified ("after [XFD-CURV-040]")
- ✅ Exact content provided
- ✅ Formatting clear (uses SPEC002 existing style)

**Testability:**
- ✅ Pass: [XFD-CURV-050] exists in SPEC002, content matches specification
- ✅ Fail: [XFD-CURV-050] missing, content differs, or in wrong location

**Status:** No issues

---

#### REQ-MIX-004: Add [XFD-IMPL-025] to SPEC002

**Issue:** None - Requirement is complete and testable

**Completeness Check:**
- ✅ Input: SPEC002-crossfade.md, exact text to add
- ✅ Output: SPEC002 with new [XFD-IMPL-025] requirement
- ✅ Behavior: Insert text after [XFD-IMPL-020], update revision history
- ✅ Constraints: Must follow GOV001 review process
- ✅ Error cases: [XFD-IMPL-020] not found → report error
- ✅ Dependencies: SPEC002 file exists

**Ambiguity Check:**
- ✅ Exact location specified ("after [XFD-IMPL-020]")
- ✅ Exact content provided
- ✅ Formatting clear

**Testability:**
- ✅ Pass: [XFD-IMPL-025] exists in SPEC002, content matches specification
- ✅ Fail: [XFD-IMPL-025] missing, content differs, or in wrong location

**Status:** No issues

---

#### REQ-MIX-005: Clarify [DBD-MIX-040] Resume-Fade Terminology

**Issue MEDIUM-001:** Current text location not precisely specified

**Completeness Check:**
- ✅ Input: SPEC016-decoder_buffer_design.md
- ⚠️ Output: [DBD-MIX-040] updated, but exact line to change not specified
- ✅ Behavior: Replace text, update revision history
- ✅ Constraints: Must follow GOV001 review process
- ✅ Error cases: Text not found → report error
- ✅ Dependencies: SPEC016 file exists

**Ambiguity Check:**
- ⚠️ **Ambiguous:** "Line 608" mentioned in requirements_index, but requirement says "current text" without line reference
- ✅ Old and new text clearly specified

**Testability:**
- ⚠️ **Partial:** Hard to verify "found and replaced correct text" without line number
- ✅ Pass: New text exists in SPEC016 [DBD-MIX-040]
- ⚠️ Fail: How to know if old text was replaced vs. new text added elsewhere?

**Issue Details:**
- **Severity:** MEDIUM
- **Category:** Incomplete specification
- **Impact:** Implementer may update wrong location in [DBD-MIX-040]
- **Resolution:** Add line number reference to requirement
  - Search SPEC016 for "fading in after pause"
  - Specify exact line number to update

**Recommendation:** Grep SPEC016 for "fading in after pause" before implementing, document line number

---

#### REQ-MIX-006: Create ADR for Mixer Refactoring

**Issue MEDIUM-002:** ADR content depends on REQ-MIX-001 findings (circular dependency on unknown)

**Completeness Check:**
- ✅ Input: Mixer architecture review, REQ-MIX-001 findings, Nygard template
- ⚠️ Output: ADR document, but content varies based on investigation results
- ✅ Behavior: Create ADR following template
- ✅ Constraints: Must use Nygard format, reference mixer review analysis
- ✅ Error cases: N/A (documentation)
- ✅ Dependencies: REQ-MIX-001 complete (findings needed for "Decision" section)

**Ambiguity Check:**
- ⚠️ **Conditional:** "Decision" section content varies: "If legacy active..." vs. "If correct active..."
- ✅ Template structure clear
- ✅ Rationale requirements clear (reference mixer review)

**Testability:**
- ⚠️ **Conditional:** Pass criteria depends on which scenario applies
- ✅ General: ADR exists, follows Nygard template, references mixer review
- ⚠️ Specific: "Correct decision for actual scenario" - can't verify until REQ-MIX-001 complete

**Issue Details:**
- **Severity:** MEDIUM
- **Category:** Dependency on unknown
- **Impact:** ADR content can't be fully specified until investigation completes
- **Resolution:** Accept two-phase ADR creation:
  1. Draft ADR with placeholders (structure, context, consequences)
  2. Fill in "Decision" section after REQ-MIX-001 complete
- **Alternative:** Create ADR after REQ-MIX-001 (defer to later increment)

**Recommendation:** Defer REQ-MIX-006 until after REQ-MIX-001 investigation complete

---

### Batch 3: Features and Testing (REQ-MIX-007-012)

#### REQ-MIX-007: Port Buffer Underrun Detection

**Issue HIGH-002:** Feature porting specifications incomplete

**Completeness Check:**
- ⚠️ Input: Legacy mixer lines 422-611 (large code block, not fully analyzed)
- ⚠️ Output: Underrun detection in correct mixer, but exact behavior not specified
- ⚠️ Behavior: "Port" is vague - copy/paste? Refactor? Adapt to new architecture?
- ✅ Constraints: Maintain architectural separation (good)
- ⚠️ Error cases: What if porting reveals architectural issues?
- ✅ Dependencies: REQ-MIX-001 (correct mixer must be active and missing feature)

**Ambiguity Check:**
- ⚠️ **Ambiguous:** "Port" - does this mean exact copy or adapted implementation?
- ⚠️ **Ambiguous:** "Flatline output behavior" ([SSD-UND-017]) - reference not explained
- ⚠️ **Ambiguous:** "Auto-resume logic" ([SSD-UND-018]) - reference not explained
- ✅ Architectural constraint clear

**Testability:**
- ⚠️ **Underspecified:** "Tests verify underrun detection and recovery" - what specific tests?
- ⚠️ Pass criteria: "Underrun detection functional" - how is "functional" measured?
- ⚠️ Fail criteria: Not specified

**Issue Details:**
- **Severity:** HIGH
- **Category:** Underspecified feature
- **Impact:** Implementer doesn't know what "underrun detection functional" means
- **Resolution:**
  - Read legacy mixer lines 422-611 (underrun detection code)
  - Extract specific behaviors:
    - Detect when buffer empty but decode not complete
    - Output flatline frame (last valid frame repeated)
    - Auto-resume when buffer refills above threshold
  - Define specific acceptance tests (e.g., "Given buffer with 10 frames, when 11th frame requested, then flatline output")
- **Alternative:** Create separate detailed specification for underrun detection feature

**Recommendation:** Add detailed test specifications for REQ-MIX-007 before implementing

---

#### REQ-MIX-008: Port Position Event Emission

**Issue:** None - Requirement is reasonably complete

**Completeness Check:**
- ✅ Input: Legacy mixer lines 614-642
- ✅ Output: Position events via tokio::sync::mpsc
- ✅ Behavior: Periodic emission, configurable interval, frame counter
- ✅ Constraints: Non-blocking send (try_send)
- ✅ Error cases: Channel closed → log warning, continue playback
- ✅ Dependencies: REQ-MIX-001

**Ambiguity Check:**
- ✅ Event structure specified (queue_entry_id, position_ms)
- ✅ Default interval specified (1 second, 44100 frames)
- ✅ Configuration source specified (database setting)

**Testability:**
- ✅ Pass: Events emitted at configured interval, contain accurate position
- ✅ Fail: Events missing, wrong interval, or inaccurate position

**Note:** More detail than REQ-MIX-007 due to simpler feature

**Status:** No issues (adequate detail for implementation)

---

#### REQ-MIX-009: Port Resume Fade-In Support

**Issue MEDIUM-003:** Specification refers to source code lines but doesn't extract behavior

**Completeness Check:**
- ⚠️ Input: Legacy mixer lines 644-671 (reference, not specification)
- ✅ Output: Resume fade-in functionality
- ⚠️ Behavior: "ResumeState tracking" - what fields? What logic?
- ✅ Constraints: Multiplicative application (after other processing)
- ✅ Error cases: N/A (fade is optional enhancement)
- ✅ Dependencies: REQ-MIX-001

**Ambiguity Check:**
- ⚠️ **Ambiguous:** "Fade applied multiplicatively" - to what? Mixed output? Individual channels?
- ✅ Fade-in range specified (0.0 to 1.0)
- ✅ Curve types specified (linear, exponential)
- ⚠️ **Ambiguous:** "Configurable duration" - configured where? Database? API parameter?

**Testability:**
- ⚠️ **Underspecified:** "Correct curve shape" - how is correctness measured?
- ✅ Pass: Fade starts at 0.0, reaches 1.0
- ⚠️ Fail: "Incorrect curve shape" - what is incorrect?

**Issue Details:**
- **Severity:** MEDIUM
- **Category:** Incomplete specification
- **Impact:** Implementer must read legacy code to understand behavior
- **Resolution:**
  - Extract ResumeState struct definition from legacy mixer
  - Specify fade calculation: `output_frame * calculate_fade_in(position)`
  - Specify configuration source (API parameter vs. database)
- **Alternative:** Reference legacy mixer code as authoritative specification

**Recommendation:** Extract resume fade-in behavior from legacy mixer lines 644-671, document in requirement

---

#### REQ-MIX-010: Unit Tests for Pre-Faded Sample Reading

**Issue LOW-001:** Test assertions not fully specified

**Completeness Check:**
- ✅ Input: Test cases defined (3 tests)
- ✅ Output: Passing unit tests
- ✅ Behavior: Verify mixer reads pre-faded samples
- ✅ Constraints: None
- ⚠️ Error cases: Compile-time test (type system) may not be testable at runtime
- ✅ Dependencies: Mixer implementation, test infrastructure

**Ambiguity Check:**
- ✅ Test 1: "Mixer API Does Not Accept Fade Curves" - clear intent
- ⚠️ **Ambiguous:** "Compile-time verification (type system enforces this)" - how to test at compile time?
- ✅ Test 2: Clear (output matches buffer_sample * master_volume)
- ✅ Test 3: Clear (output = (buffer1 + buffer2) * master_volume)

**Testability:**
- ⚠️ Test 1: "Compile-time verification" - can't write runtime test for this
- ✅ Test 2: Pass/fail clear (numeric comparison)
- ✅ Test 3: Pass/fail clear (numeric comparison)

**Issue Details:**
- **Severity:** LOW
- **Category:** Test design ambiguity
- **Impact:** Test 1 may not be implementable as runtime test
- **Resolution:** Clarify Test 1:
  - Either: Document as "design verification" (code review confirms API signatures)
  - Or: Remove Test 1 from runtime test suite (covered by type system)
- **Alternative:** Keep Test 1 as documentation test (compile check only)

**Recommendation:** Clarify Test 1 as "API design verification" (code review), not runtime test

---

#### REQ-MIX-011: Integration Tests with Fader Component

**Issue:** None - Requirement is adequately specified

**Completeness Check:**
- ✅ Input: Fader + Buffer + Mixer pipeline
- ✅ Output: Passing integration tests
- ✅ Behavior: Verify end-to-end pipeline (Fader applies → Buffer stores → Mixer reads)
- ✅ Constraints: None
- ✅ Error cases: Double-fading detection (samples faded twice)
- ✅ Dependencies: Fader component functional, Buffer management functional

**Ambiguity Check:**
- ✅ Test 1: Clear (Fader-Buffer-Mixer pipeline verification)
- ✅ Test 2: Clear (Crossfade overlap with pre-fading)
- ✅ Assertions clear (output matches expected faded values)

**Testability:**
- ✅ Pass: Output matches expected (Fader applied curves, Mixer summed)
- ✅ Fail: Output doesn't match (indicates double-fading or incorrect pipeline)

**Status:** No issues (adequate for integration testing)

---

#### REQ-MIX-012: Crossfade Overlap Tests (Simple Addition Verification)

**Issue MEDIUM-004:** Test 2 (Volume Normalization) may have incorrect expected output

**Completeness Check:**
- ✅ Input: Test cases defined (3 tests)
- ✅ Output: Passing crossfade overlap tests
- ✅ Behavior: Verify simple addition (A + B)
- ✅ Constraints: Reference to [SSD-CLIP-010] (clipping spec)
- ⚠️ Error cases: Test 2 shows clipping, but clipping behavior not verified in requirement
- ✅ Dependencies: Mixer implementation, buffer management

**Ambiguity Check:**
- ✅ Test 1: Clear (0.5 + 0.3 = 0.8)
- ⚠️ **Ambiguous:** Test 2 expected output: "min(1.4 * 1.0, 1.0) = 1.0"
  - Is this correct? Should master volume be applied BEFORE or AFTER clipping?
  - Current: `(0.7 + 0.7) * 1.0 = 1.4` → clamp to 1.0
  - Alternative: `clamp(0.7 + 0.7, 1.0) * 1.0 = 1.0 * 1.0 = 1.0` (same result, different order)
- ⚠️ **Ambiguous:** "Verify clipping behavior is as specified in [SSD-CLIP-010]" - what does [SSD-CLIP-010] say?
- ✅ Test 3: Clear (asymmetric fade durations, pre-faded samples)

**Testability:**
- ✅ Test 1: Pass/fail clear
- ⚠️ Test 2: Pass criteria depends on [SSD-CLIP-010] specification (not provided)
- ✅ Test 3: Pass/fail clear

**Issue Details:**
- **Severity:** MEDIUM
- **Category:** Missing specification reference
- **Impact:** Can't verify Test 2 without [SSD-CLIP-010] clipping specification
- **Resolution:**
  - Look up [SSD-CLIP-010] in SPEC documentation
  - Document expected clipping behavior in test specification
  - Clarify order: (sum + clamp + master_volume) vs. (sum + master_volume + clamp)
- **Alternative:** Remove clipping verification from Test 2 (out of scope for this plan)

**Recommendation:** Clarify Test 2 expected behavior by referencing [SSD-CLIP-010] specification

---

## Issues Summary Table

| Issue ID | Severity | Category | Requirement | Description | Resolution |
|----------|----------|----------|-------------|-------------|------------|
| HIGH-001 | HIGH | Missing spec | REQ-MIX-002 | Scenario A (migration) underspecified | Add migration specification |
| HIGH-002 | HIGH | Underspecified | REQ-MIX-007 | Underrun detection behavior not extracted | Define specific behaviors + tests |
| MEDIUM-001 | MEDIUM | Incomplete | REQ-MIX-005 | [DBD-MIX-040] line number not specified | Grep SPEC016 for exact location |
| MEDIUM-002 | MEDIUM | Dependency | REQ-MIX-006 | ADR content depends on unknown | Defer until after REQ-MIX-001 |
| MEDIUM-003 | MEDIUM | Incomplete | REQ-MIX-009 | Resume fade behavior not extracted | Extract from legacy mixer code |
| MEDIUM-004 | MEDIUM | Missing ref | REQ-MIX-012 | [SSD-CLIP-010] not provided | Look up clipping specification |
| LOW-001 | LOW | Test design | REQ-MIX-010 | Compile-time test not runtime testable | Clarify as design verification |

---

## Cross-Requirement Analysis

### Consistency Check

**No contradictions found.** Requirements are internally consistent.

**Verified:**
- ✅ No conflicting priorities
- ✅ No resource conflicts (all work can proceed in sequence)
- ✅ No interface specification conflicts
- ✅ Timing budgets reasonable (no impossible deadlines)

### Dependency Validation

**All dependencies exist and have stable interfaces:**

1. **REQ-MIX-002 depends on REQ-MIX-001:** ✅ Valid dependency, sequencing clear
2. **REQ-MIX-006 depends on REQ-MIX-001:** ✅ Valid dependency, suggests deferral
3. **REQ-MIX-007/008/009 depend on REQ-MIX-001:** ✅ Valid conditional dependencies
4. **REQ-MIX-011 depends on Fader component:** ✅ Assumed functional (see assumptions in scope_statement.md)
5. **REQ-MIX-012 depends on buffer management:** ✅ Assumed functional

**No missing dependencies.**

---

## Recommendations by Priority

### Before Implementation (CRITICAL - None)

**No critical blockers.** Plan can proceed to Phase 3 (Test Definition).

### During Increment Planning (HIGH - 2 issues)

**HIGH-001 (REQ-MIX-002):**
- **Action:** If investigation (REQ-MIX-001) reveals legacy mixer is active:
  - Create separate increment: "Migrate from Legacy to Correct Mixer"
  - Define migration steps, verification criteria, rollback plan
  - This increment may be substantial (split into sub-increments)
- **Timeline:** Address during Increment 2 planning (after investigation)

**HIGH-002 (REQ-MIX-007):**
- **Action:** Before implementing underrun detection porting:
  - Read legacy mixer lines 422-611
  - Extract underrun detection behaviors into detailed specification
  - Define specific test cases (given/when/then format)
  - Update test_specifications/ with detailed underrun tests
- **Timeline:** Address during Increment 4 planning (before feature porting)

### During Implementation (MEDIUM - 4 issues)

**MEDIUM-001 (REQ-MIX-005):**
- **Action:** Before updating SPEC016:
  - Grep SPEC016 for "fading in after pause"
  - Document exact line number in implementation notes
  - Verify text match before replacement
- **Timeline:** Increment 3 (SPEC016 updates)

**MEDIUM-002 (REQ-MIX-006):**
- **Action:** Create ADR in two phases:
  1. Increment 1: Draft ADR structure (Context, Consequences)
  2. Increment 2: Fill in Decision section (after investigation complete)
- **Alternative:** Defer entire ADR to Increment 2 (after investigation)
- **Timeline:** Increment 1 (partial) + Increment 2 (completion)

**MEDIUM-003 (REQ-MIX-009):**
- **Action:** Before implementing resume fade-in porting:
  - Read legacy mixer lines 644-671
  - Extract ResumeState struct and logic
  - Document fade calculation algorithm
  - Clarify configuration source
  - Update test_specifications/ with detailed resume fade tests
- **Timeline:** Increment 4 (before feature porting)

**MEDIUM-004 (REQ-MIX-012):**
- **Action:** Before implementing Test 2:
  - Grep SPEC documentation for [SSD-CLIP-010]
  - Document expected clipping behavior
  - Clarify master volume application order
  - Update test specification with correct expected output
- **Alternative:** Remove clipping verification from Test 2 (defer to separate test)
- **Timeline:** Increment 5 (before crossfade tests)

### Low Priority (LOW - 1 issue)

**LOW-001 (REQ-MIX-010):**
- **Action:** Clarify Test 1 as "API Design Verification":
  - Document in test specification: "Verified via code review + type system"
  - No runtime test needed (compile-time enforcement)
  - Include in acceptance criteria but not in test suite
- **Timeline:** Increment 5 (before unit tests)

---

## Auto-/think Trigger Evaluation

**Criteria for /think (from workflow):**
- 5+ Critical issues, OR
- 10+ High issues, OR
- Unclear architecture/approach, OR
- Novel/risky technical elements

**Current Status:**
- 0 Critical issues
- 2 High issues
- Architecture is clear (SPEC016 [DBD-MIX-042] defines approach)
- Technical elements are not novel (code porting, documentation updates, testing)

**Decision:** /think NOT required

**Rationale:**
- High issues are specification gaps, not architectural ambiguities
- Issues can be resolved during implementation (read legacy code, grep specs)
- No fundamental uncertainty about approach

---

## Decision Point

**Phase 2 Complete: Proceed to Phase 3?**

**Assessment:**
- ✅ All requirements analyzed for completeness, ambiguity, testability
- ✅ Issues clearly documented with specific resolutions
- ✅ No CRITICAL or 5+ HIGH issues (would block implementation)
- ✅ 2 HIGH + 4 MEDIUM + 1 LOW issues are resolvable during implementation
- ✅ Issues prioritized (before implementation, during increment planning, during implementation)

**Recommendation:** PROCEED to Phase 3 (Acceptance Test Definition)

**Rationale:**
- No critical blockers
- High-priority issues are specification gaps that can be filled during increment planning
- Medium/Low issues are implementation details that can be resolved incrementally
- Test definitions (Phase 3) will further clarify requirements

**User Approval Requested:** Confirm proceeding to Phase 3 with understanding that:
1. REQ-MIX-002 may require migration sub-plan if legacy mixer is active
2. REQ-MIX-007 needs detailed underrun detection specification before implementation
3. Other issues are minor and resolvable during implementation

---

**Phase 2 Verification Complete**
**Date:** 2025-01-30
**Recommendation:** PROCEED to Phase 3
