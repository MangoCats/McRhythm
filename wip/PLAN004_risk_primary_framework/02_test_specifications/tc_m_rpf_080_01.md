# TC-M-RPF-080-01: /think Template Risk Assessment Section

**Test ID:** TC-M-RPF-080-01
**Test Type:** Manual Inspection
**Requirements:** REQ-RPF-080, REQ-RPF-090
**Priority:** CRITICAL
**Estimated Effort:** 10 minutes

---

## Test Specification

### Scope
Verify that .claude/commands/think.md comparison framework template has been restructured with Risk Assessment section first, including failure modes.

### Given
- .claude/commands/think.md file exists
- File contains comparison framework template section (originally around lines 430-450)
- File is readable

### When
- Open .claude/commands/think.md
- Locate comparison framework template section
- Review template structure

### Then
Template contains restructured format with Risk Assessment as FIRST section:

**Expected Template Structure:**

```markdown
APPROACH X: [Name/Description]

Risk Assessment:
  - Failure Risk: [Low/Medium/High]
  - Failure Modes:
    1. [Specific failure mode 1] - Probability: [%] - Impact: [description]
    2. [Specific failure mode 2] - Probability: [%] - Impact: [description]
  - Mitigation Strategies: [list]
  - Residual Risk After Mitigation: [Low/Medium/High]

Quality Characteristics:
  [content]

Implementation Considerations:
  [content]
```

### Verify

**Risk Assessment Section Checklist:**
- [ ] "Risk Assessment:" appears as first section in template
- [ ] Includes "Failure Risk:" field
- [ ] Includes "Failure Modes:" subsection
- [ ] Failure modes format: "[Mode] - Probability: [%] - Impact: [description]"
- [ ] At least 2 failure mode placeholders shown
- [ ] Includes "Mitigation Strategies:" field
- [ ] Includes "Residual Risk After Mitigation:" field
- [ ] Section appears BEFORE "Quality Characteristics" section
- [ ] Section appears BEFORE "Implementation Considerations" section

**Template Integration:**
- [ ] Template appears in "Phase 6: Options Comparison and Presentation" section
- [ ] Template replaces old format (not duplicated alongside)
- [ ] Template includes instruction to repeat for each approach
- [ ] Old template format (Advantages/Disadvantages/Effort/Risk side-by-side) removed or commented

### Pass Criteria

**PASS if:**
- All 9 Risk Assessment checklist items ✓
- All 4 Template Integration items ✓
- Risk Assessment clearly appears FIRST in approach comparison
- Failure modes format matches specification exactly

**PARTIAL if:**
- 11-12 total checklist items ✓
- Minor formatting variations (e.g., "Failure Scenarios" instead of "Failure Modes")
- Intent clear even if wording differs slightly

**FAIL if:**
- Risk Assessment not first section (Quality or Effort appears before it)
- Failure modes not included in Risk Assessment
- Old template format still present (not replaced)
- <10 total checklist items verified

### Fail Criteria

Risk Assessment missing, wrong order, or insufficient detail (no failure modes).

---

## Test Data

**Input:** .claude/commands/think.md at c:\Users\Mango Cat\Dev\McRhythm\.claude\commands\think.md

**Expected Location:** Phase 6 section (originally lines 430-450, may shift with edits)

**Expected Template Size:** ~15-20 lines for single approach template

**Reference:** wip/_deprioritize_effort_analysis_results.md lines 411-443

---

## Integration with Other Tests

**Prerequisites:**
- None (can test independently)

**Related Tests:**
- TC-M-RPF-100-01: Verifies Quality Characteristics section (second)
- TC-M-RPF-110-01: Verifies Implementation Considerations section (third)
- TC-M-RPF-120-01: Verifies RISK-BASED RANKING section added
- TC-I-RPF-002-01: Integration test - produces valid analysis output

**Execution Sequence:**
1. Run TC-M-RPF-080-01 (this test)
2. Run TC-M-RPF-100-01, TC-M-RPF-110-01, TC-M-RPF-120-01
3. Run TC-I-RPF-002-01 (verify template works end-to-end)

---

## Notes

**Why CRITICAL:**
This is the core structural change to /think workflow. If Risk Assessment isn't first, framework fails.

**Common Failure Modes:**
- Risk Assessment added but not placed first (appears after Effort)
- Failure modes missing from Risk Assessment section
- Old template not removed (duplication causes confusion)

**Verification Tip:**
Search for "APPROACH" in think.md - first structural element after approach name should be "Risk Assessment"

---

**Test Status:** Pending
**Last Updated:** 2025-10-25
