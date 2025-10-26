# TC-S-RPF-001-01: End-to-End /think Workflow with Risk-First Framework

**Test ID:** TC-S-RPF-001-01
**Test Type:** System / End-to-End Test
**Requirements:** All CRITICAL requirements (REQ-RPF-010, 020, 030, 080, 140, 150, 160, 170, 210)
**Priority:** HIGH
**Estimated Effort:** 30 minutes

---

## Test Specification

### Scope
Execute complete /think workflow using new risk-first framework to verify end-to-end functionality and verify risk-first language appears in actual outputs.

### Test Scenario
**Scenario:** Use /think to analyze a real decision with multiple approaches (e.g., documentation format choice, architectural pattern selection, etc.)

**Environment:**
- WKMP project with all framework changes implemented
- Real or synthetic problem requiring approach comparison
- Multiple viable approaches (minimum 2, ideally 3)

### Given
- CLAUDE.md Decision-Making Framework section exists and is correct (TC-M-RPF-010-01 passed)
- /think command template updated with risk-first structure (TC-M-RPF-080-01 passed)
- templates/risk_assessment.md exists (TC-M-RPF-210-01 passed)
- Problem document created with: question to answer, approaches to compare, context

### When
1. Execute `/think [problem_document]`
2. Allow /think workflow to complete all 8 phases
3. Read generated analysis output document

### Then
Analysis output demonstrates risk-first framework usage:

**Phase 5 Output (Analysis and Synthesis):**
- [ ] Each approach has "Risk Assessment" section FIRST
- [ ] Risk Assessment includes specific failure modes (not vague)
- [ ] Failure modes include probability and impact estimates
- [ ] Mitigation strategies documented
- [ ] Residual risk (after mitigation) stated

**Phase 6 Output (Options Comparison):**
- [ ] Approaches presented with Risk Assessment → Quality → Effort ordering
- [ ] Risk-based ranking included (lowest risk ranked first)
- [ ] Comparison emphasizes risk differences before effort differences

**Phase 7 Output (Executive Summary):**
- [ ] Recommendation justification states risk-based reasoning
- [ ] If lowest-risk approach has higher effort, recommendation acknowledges this but still chooses lowest-risk
- [ ] Language like "due to lowest failure risk" or "minimal risk justifies effort"

### Verify

**Risk-First Language Indicators:**
- [ ] "Risk" appears before "Effort" in approach descriptions
- [ ] Phrases like "lowest residual risk," "failure risk," "mitigation"
- [ ] Recommendation explicitly references risk as primary decision factor
- [ ] Effort mentioned but framed as secondary ("effort differential is acceptable given risk reduction")

**Contrast with Old Pattern:**
- [ ] ❌ Does NOT say "acceptable risk/effort" (co-equal language)
- [ ] ❌ Does NOT lead with effort comparison ("Approach A is faster but...")
- [ ] ❌ Does NOT recommend based primarily on effort

**Framework Compliance:**
- [ ] Decision follows: Risk (primary) → Quality (secondary) → Effort (tertiary)
- [ ] If multiple approaches have equivalent risk, quality used as tiebreaker
- [ ] If equivalent risk AND quality, effort used as final tiebreaker

### Pass Criteria

**PASS if:**
- All risk-first language indicators present (8/8 ✓)
- All contrast checks pass (old patterns absent)
- Framework compliance verified
- Analysis output is usable (clear, well-structured, actionable)

**PARTIAL if:**
- 6-7/8 risk-first indicators present
- Minor instances of old language pattern (1-2 occurrences)
- Framework mostly followed but one section unclear

**FAIL if:**
- <6 risk-first indicators
- Recommendation based primarily on effort despite risk differences
- Analysis reverts to old "acceptable risk/effort" language
- Output unusable or confusing

### Fail Criteria

Analysis does not demonstrate risk-first decision-making or reverts to effort-first language.

---

## Test Data

**Input Problem Document:**
Create test problem document with:
- **Question:** "What approach should we use for [specific decision]?"
- **Approach A:** Lower effort (10 hours), higher risk (Medium - novel technique, untested)
- **Approach B:** Higher effort (20 hours), lower risk (Low - proven pattern, well-documented)
- **Approach C:** Medium effort (15 hours), medium risk (Low-Medium - hybrid approach)

**Expected Output:**
- Risk-based ranking: B (Low risk) > C (Low-Medium risk) > A (Medium risk)
- Recommendation: Approach B
- Justification: "Lowest residual risk. Effort differential (10 hours vs. 20 hours) is secondary to risk reduction."

**Actual Test Problem (Example):**
Use real WKMP decision if available, or create synthetic problem like:
- "Document storage: Monolithic vs. Modular structure for large specifications"
- "API error handling: Custom error types vs. Standard HTTP codes"
- "Configuration: TOML vs. Database-first for runtime settings"

---

## Execution Procedure

1. **Setup:**
   - Create problem document in wip/ directory
   - Define 2-3 approaches with different risk/effort profiles
   - Ensure at least one approach has higher effort but lower risk

2. **Execute:**
   - Run `/think [problem_document]`
   - Do NOT interrupt or guide the analysis
   - Let workflow complete naturally

3. **Inspect Output:**
   - Open generated analysis results document
   - Use verification checklist above
   - Document which indicators are present/absent

4. **Record Results:**
   - PASS/PARTIAL/FAIL determination
   - Specific examples of risk-first language found
   - Any deviations from framework noted

---

## Success Examples

**Good Risk-First Language:**
- "Approach B has lowest residual risk (Low) after mitigation through comprehensive testing"
- "Recommend Approach B due to minimal failure risk. Effort differential (10 additional hours) is acceptable given risk reduction from Medium to Low"
- "Risk-based ranking: B > C > A (ordered by residual risk ascending)"

**Bad Old-Pattern Language (Should NOT Appear):**
- "Approach A is faster (10 hours vs. 20) so recommended despite higher risk"
- "Balancing acceptable risk and effort, Approach A is preferred"
- "While Approach B is lower risk, the effort savings of Approach A (50% faster) justify selection"

---

## Integration with Other Tests

**Prerequisites (Must PASS First):**
- TC-M-RPF-010-01: CLAUDE.md framework exists
- TC-M-RPF-080-01: /think template Risk Assessment first
- TC-M-RPF-120-01: /think RISK-BASED RANKING section
- TC-M-RPF-130-01: /think recommendation risk-based justification
- TC-I-RPF-002-01: /think template produces valid output

**Validates:**
- REQ-RPF-030: All decisions MUST follow risk-first framework
- REQ-RPF-050: Rank approaches by failure risk
- REQ-RPF-060: Quality evaluated among equivalent-risk approaches
- REQ-RPF-070: Effort considered only among equivalent risk+quality

**Follow-Up Tests:**
- TC-S-RPF-003-01: Verify risk-first language in multiple analyses (pattern consistency)
- TC-V-RPF-001-01: Validate "equivalent risk" definition works in practice

---

## Notes

**Why System Test:**
End-to-end validation that framework changes produce desired behavioral outcome (risk-first decisions).

**Test Execution Timing:**
Run after ALL implementation complete and integration tests pass. This is final validation before release.

**Typical Issues:**
- Risk-first language in template but not in actual output (template not actually used)
- Recommendation says "lowest risk" but justification emphasizes effort
- Framework followed partially but mixed with old patterns

**Debugging Failed Test:**
If test fails:
1. Check which language indicators missing
2. Review /think command execution log
3. Verify templates actually loaded during execution
4. Check if old cached templates used instead of new

---

**Test Status:** Pending
**Last Updated:** 2025-10-25
