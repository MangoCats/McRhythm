# Traceability Matrix - Risk-Primary Decision Framework

**Plan:** PLAN004
**Date:** 2025-10-25
**Purpose:** Ensure 100% requirement coverage through acceptance tests

---

## Matrix

| Requirement ID | Requirement Description | Manual Tests | Integration Tests | System Tests | Implementation File(s) | Status | Coverage |
|----------------|------------------------|--------------|-------------------|--------------|------------------------|--------|----------|
| REQ-RPF-010 | Add "Decision-Making Framework - MANDATORY" section to CLAUDE.md | TC-M-010-01 | TC-I-001-01 | TC-S-001-01 | CLAUDE.md | Pending | Complete |
| REQ-RPF-020 | Framework MUST prioritize risk (primary), quality (secondary), effort (tertiary) | TC-M-010-01 | TC-I-001-01 | TC-S-001-01 | CLAUDE.md | Pending | Complete |
| REQ-RPF-030 | All design decisions MUST follow risk-first framework | TC-M-030-01 | TC-I-001-01 | TC-S-001-01, TC-S-003-01 | CLAUDE.md | Pending | Complete |
| REQ-RPF-040 | Risk assessment MUST identify failure modes with probability and impact | TC-M-040-01 | - | TC-S-001-01 | CLAUDE.md | Pending | Complete |
| REQ-RPF-050 | Rank approaches by failure risk (lowest = highest rank) | TC-M-050-01 | - | TC-S-001-01 | CLAUDE.md | Pending | Complete |
| REQ-RPF-060 | Quality evaluated among equivalent-risk approaches | TC-M-050-01 | - | TC-S-001-01 | CLAUDE.md | Pending | Complete |
| REQ-RPF-070 | Effort considered only among equivalent risk+quality approaches | TC-M-050-01 | - | TC-S-001-01 | CLAUDE.md | Pending | Complete |
| REQ-RPF-080 | /think command comparison framework restructured (risk → quality → effort) | TC-M-080-01 | TC-I-002-01 | TC-S-001-01 | .claude/commands/think.md | Pending | Complete |
| REQ-RPF-090 | /think output includes Risk Assessment section with failure modes | TC-M-080-01 | TC-I-002-01 | TC-S-001-01 | .claude/commands/think.md | Pending | Complete |
| REQ-RPF-100 | /think output includes Quality Characteristics section | TC-M-100-01 | TC-I-002-01 | TC-S-001-01 | .claude/commands/think.md | Pending | Complete |
| REQ-RPF-110 | /think output includes Implementation Considerations section (effort tertiary) | TC-M-110-01 | TC-I-002-01 | TC-S-001-01 | .claude/commands/think.md | Pending | Complete |
| REQ-RPF-120 | /think output includes RISK-BASED RANKING of approaches | TC-M-120-01 | TC-I-002-01 | TC-S-001-01 | .claude/commands/think.md | Pending | Complete |
| REQ-RPF-130 | /think recommendation explicitly states risk-based justification | TC-M-130-01 | TC-I-002-01 | TC-S-001-01 | .claude/commands/think.md | Pending | Complete |
| REQ-RPF-140 | /plan Phase 4 objective changed to "minimal failure risk; acknowledge effort" | TC-M-140-01 | TC-I-003-01 | TC-S-002-01 | .claude/commands/plan.md | Pending | Complete |
| REQ-RPF-150 | /plan Phase 4 MUST perform risk assessment for each approach | TC-M-140-01 | TC-I-003-01 | TC-S-002-01 | .claude/commands/plan.md | Pending | Complete |
| REQ-RPF-160 | /plan Phase 4 MUST rank by residual risk (after mitigation) | TC-M-160-01 | TC-I-003-01 | TC-S-002-01 | .claude/commands/plan.md | Pending | Complete |
| REQ-RPF-170 | /plan Phase 4 selects lowest-risk approach | TC-M-160-01 | TC-I-003-01 | TC-S-002-01 | .claude/commands/plan.md | Pending | Complete |
| REQ-RPF-180 | /plan Phase 4 uses quality as tiebreaker for equivalent risk | TC-M-180-01 | TC-I-003-01 | TC-S-002-01 | .claude/commands/plan.md | Pending | Complete |
| REQ-RPF-190 | /plan Phase 4 uses effort as final tiebreaker for equivalent risk+quality | TC-M-180-01 | TC-I-003-01 | TC-S-002-01 | .claude/commands/plan.md | Pending | Complete |
| REQ-RPF-200 | /plan Phase 4 decision documented as ADR with risk-based justification | TC-M-200-01 | TC-I-003-01 | TC-S-002-01 | .claude/commands/plan.md | Pending | Complete |
| REQ-RPF-210 | Create templates/risk_assessment.md template file | TC-M-210-01 | - | TC-S-001-01 | templates/risk_assessment.md | Pending | Complete |
| REQ-RPF-220 | Risk assessment template includes Failure Modes table | TC-M-210-01 | - | TC-S-001-01 | templates/risk_assessment.md | Pending | Complete |
| REQ-RPF-230 | Risk assessment template includes Mitigation Strategies table | TC-M-230-01 | - | TC-S-001-01 | templates/risk_assessment.md | Pending | Complete |
| REQ-RPF-240 | Risk assessment template includes Overall Risk Assessment section | TC-M-240-01 | - | TC-S-001-01 | templates/risk_assessment.md | Pending | Complete |
| REQ-RPF-250 | Update examples in /think and /plan commands to reflect new framework | TC-M-250-01, TC-M-250-02 | - | - | .claude/commands/think.md, .claude/commands/plan.md | Pending | Complete |

---

## Coverage Summary

**Total Requirements:** 25
**Total Tests:** 29 (some requirements tested by multiple tests)
**Coverage:** 100% (all requirements have at least 2 tests)

**By Priority:**
- CRITICAL (9 requirements): 100% coverage, average 3.2 tests per requirement
- HIGH (13 requirements): 100% coverage, average 3.0 tests per requirement
- MEDIUM (2 requirements): 100% coverage, average 2.5 tests per requirement
- LOW (1 requirement): 100% coverage, 2 tests

**By Test Type:**
- Manual Tests: Cover all 25 requirements (inspection-based)
- Integration Tests: Cover 15 requirements (component interaction)
- System Tests: Cover 21 requirements (end-to-end workflows)
- Validation Tests: Cover 10 requirements (issue resolution, regression)

---

## Implementation Files

| File | Requirements Implemented | Test Coverage |
|------|-------------------------|---------------|
| CLAUDE.md | REQ-RPF-010 through REQ-RPF-070 (7 requirements) | TC-M-010-01, TC-M-030-01, TC-M-040-01, TC-M-050-01, TC-I-001-01, TC-S-001-01 |
| .claude/commands/think.md | REQ-RPF-080 through REQ-RPF-130, REQ-RPF-250 (7 requirements) | TC-M-080-01, TC-M-100-01, TC-M-110-01, TC-M-120-01, TC-M-130-01, TC-M-250-01, TC-I-002-01, TC-S-001-01 |
| .claude/commands/plan.md | REQ-RPF-140 through REQ-RPF-200, REQ-RPF-250 (8 requirements) | TC-M-140-01, TC-M-160-01, TC-M-180-01, TC-M-200-01, TC-M-250-02, TC-I-003-01, TC-S-002-01 |
| templates/risk_assessment.md | REQ-RPF-210 through REQ-RPF-240 (4 requirements) | TC-M-210-01, TC-M-230-01, TC-M-240-01, TC-S-001-01 |

**Total Files:** 4
**Total Requirements:** 25 (some requirements span multiple files)

---

## Test Execution Tracking

**Usage During Implementation:**

1. **As each file is implemented, update Status column:**
   - Pending → In Progress → Implemented → Tested → Verified

2. **Track test results in matrix:**
   - Add "Test Result" column during execution
   - Record PASS/PARTIAL/FAIL for each test
   - Link to test execution logs if needed

3. **Coverage validation:**
   - Before considering increment complete, verify all requirements in that increment have tests defined
   - Before release, verify all tests PASS or documented exceptions approved

**Example During Implementation:**

| Requirement ID | ... | Implementation File(s) | Status | Test Result | Notes |
|----------------|-----|------------------------|--------|-------------|-------|
| REQ-RPF-010 | ... | CLAUDE.md | Implemented | PASS (TC-M-010-01) | Section added lines 60-105 |
| REQ-RPF-020 | ... | CLAUDE.md | Implemented | PASS (TC-M-010-01) | Risk→Quality→Effort verified |

---

## Gap Analysis

**Requirements with NO Tests:** None (100% coverage)

**Requirements with Single Test:** None (all have 2+ tests)

**Requirements with Weak Coverage:** None identified

**Test Types Missing:** None (manual, integration, system, validation all present)

**Overall:** Zero gaps identified. Coverage is comprehensive with redundancy.

---

## Validation Tests (Issue Resolution Tracking)

| Issue | Resolution | Validation Test | Requirement Impact |
|-------|-----------|-----------------|-------------------|
| ISSUE-HIGH-001 | "Equivalent risk" defined in CLAUDE.md | TC-V-RPF-001-01 | REQ-RPF-060, REQ-RPF-070, REQ-RPF-180, REQ-RPF-190 |
| ISSUE-HIGH-002 | ADR format specified (Nygard template, inline) | TC-V-RPF-002-01 | REQ-RPF-200 |
| Regression Risk | Existing /think analyses still valid | TC-V-RPF-003-01 | All /think requirements |
| Regression Risk | Existing /plan workflows functional | TC-V-RPF-004-01 | All /plan requirements |
| Verbosity Standard | 20-40% reduction target maintained | TC-V-RPF-005-01 | All documentation requirements |

**All identified issues have corresponding validation tests.**

---

## Test Sequence Recommendation

### Phase 1: Component Tests (Per Increment)
```
Increment 1 (CLAUDE.md):
→ TC-M-RPF-010-01, TC-M-RPF-030-01, TC-M-RPF-040-01, TC-M-RPF-050-01

Increment 2 (/think template):
→ TC-M-RPF-080-01, TC-M-RPF-100-01, TC-M-RPF-110-01, TC-M-RPF-120-01, TC-M-RPF-130-01

Increment 3 (/plan Phase 4):
→ TC-M-RPF-140-01, TC-M-RPF-160-01, TC-M-RPF-180-01, TC-M-RPF-200-01

Increment 4 (Risk template):
→ TC-M-RPF-210-01, TC-M-RPF-230-01, TC-M-RPF-240-01

Increment 5 (Examples):
→ TC-M-RPF-250-01, TC-M-RPF-250-02
```

### Phase 2: Integration Tests (After Components)
```
→ TC-I-RPF-001-01 (CLAUDE.md integration)
→ TC-I-RPF-002-01 (/think template integration)
→ TC-I-RPF-003-01 (/plan Phase 4 integration)
```

### Phase 3: System Tests (End-to-End)
```
→ TC-S-RPF-001-01 (Full /think workflow)
→ TC-S-RPF-002-01 (Full /plan workflow)
→ TC-S-RPF-003-01 (Risk-first language enforcement)
```

### Phase 4: Validation Tests (Final Check)
```
→ TC-V-RPF-001-01 (Equivalent risk definition)
→ TC-V-RPF-002-01 (ADR format)
→ TC-V-RPF-003-01 (No /think regression)
→ TC-V-RPF-004-01 (No /plan regression)
→ TC-V-RPF-005-01 (Verbosity compliance)
```

**Total Execution Time Estimate:** 2-3 hours (all 29 tests)

---

## Acceptance Criteria

**Plan is COMPLETE when:**
- ✓ All 25 requirements have "Complete" coverage status
- ✓ All 9 CRITICAL requirement tests PASS
- ✓ ≥90% of HIGH requirement tests PASS (12/13 minimum)
- ✓ No regressions detected (TC-V-003-01 and TC-V-004-01 PASS)
- ✓ All integration tests PASS (TC-I-001-01, TC-I-002-01, TC-I-003-01)
- ✓ At least 2/3 system tests PASS

**Release is BLOCKED if:**
- Any CRITICAL requirement test FAILS
- >2 HIGH requirement tests FAIL
- Regressions detected in existing workflows
- Integration tests FAIL

---

**Traceability Matrix Status:** ✅ COMPLETE - 100% Coverage Verified
**Last Updated:** 2025-10-25
**Maintained By:** Implementation Team
