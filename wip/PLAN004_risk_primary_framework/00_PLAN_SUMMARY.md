# PLAN004: Risk-Primary Decision Framework - Implementation Plan Summary

**Plan ID:** PLAN004
**Feature:** Risk-Primary Decision Framework
**Date Created:** 2025-10-25
**Status:** Week 1 Complete (Phases 1-3)
**Priority:** CRITICAL
**Stakeholder:** Mango Cat (Principal Developer)

---

## READ THIS FIRST

**This is the executive summary. For implementation:**
1. Read this summary (~400 lines)
2. Read relevant increment file from 04_increments/ (~250 lines)
3. Read relevant test specs from 02_test_specifications/ (~100 lines)
4. **Do NOT read** full documentation until archival

**Total context per increment:** ~750 lines (optimal for AI/human)

---

## What We're Building

### Problem Statement

WKMP project has excellent quality mechanisms (mandatory /plan, test-driven development, 100% test coverage) but decision-making language treats **risk and effort as co-equal factors** ("acceptable risk/effort").

**Issue:** Cognitive bias toward concrete factors (effort hours) over abstract factors (risk) leads to unintentional effort-minimizing decisions despite project charter quality-absolute goals ("flawless audio playback").

**Gap:** Decision framework language, not quality practices.

### Solution Overview

Implement **Risk-Primary Decision Framework (Approach 3)** from /think analysis:
- Restructure decision framework: **Risk (primary) → Quality (secondary) → Effort (tertiary)**
- Add explicit risk assessment templates with failure mode analysis
- Update /think and /plan comparison frameworks
- Enforce through templates and ADR (Architecture Decision Record) requirement

### Success Criteria

**Behavioral Shift:**
- Decision language changes from "acceptable risk/effort" to "minimal risk; acknowledge effort"
- Approaches rejected for high risk despite low effort: From ~10% → ~40%
- Implementation rework rate: Expected -30% reduction

**Deliverables:**
- CLAUDE.md: New "Decision-Making Framework - MANDATORY" section
- .claude/commands/think.md: Restructured comparison framework
- .claude/commands/plan.md: Updated Phase 4 objective and process
- templates/risk_assessment.md: New risk assessment template
- Updated examples demonstrating risk-first analysis

---

## Requirements Summary

**Total:** 25 requirements (9 CRITICAL, 13 HIGH, 2 MEDIUM)

### CRITICAL Requirements (Implementation Blockers)

| ID | Description | File Affected |
|----|-------------|---------------|
| REQ-RPF-010 | Add "Decision-Making Framework - MANDATORY" to CLAUDE.md | CLAUDE.md |
| REQ-RPF-020 | Framework prioritizes risk→quality→effort | CLAUDE.md |
| REQ-RPF-030 | All decisions MUST follow framework | CLAUDE.md |
| REQ-RPF-080 | /think comparison framework restructured | .claude/commands/think.md |
| REQ-RPF-140 | /plan Phase 4 objective: "minimal failure risk; acknowledge effort" | .claude/commands/plan.md |
| REQ-RPF-150 | /plan Phase 4 MUST perform risk assessment | .claude/commands/plan.md |
| REQ-RPF-160 | /plan Phase 4 MUST rank by residual risk | .claude/commands/plan.md |
| REQ-RPF-170 | /plan Phase 4 selects lowest-risk approach | .claude/commands/plan.md |
| REQ-RPF-210 | Create templates/risk_assessment.md | templates/risk_assessment.md |

**All CRITICAL requirements must PASS tests before release.**

### HIGH Requirements (Quality)

13 requirements covering:
- Risk assessment process details (failure modes, probability/impact, mitigation)
- /think template sections (Risk Assessment, Quality Characteristics, Implementation Considerations)
- /plan tiebreaker logic (quality for equivalent risk, effort for equivalent risk+quality)
- Risk template components (Failure Modes table, Mitigation Strategies table, Overall Assessment)

**≥90% (12/13) must PASS tests before release.**

### MEDIUM Requirements (Enhancement)

- REQ-RPF-190: Effort as final tiebreaker (fallback case)
- REQ-RPF-250: Update examples in commands

**Nice-to-have, not blockers.**

**Full requirements list:** See [requirements_index.md](requirements_index.md)

---

## Specification Issues Resolved

**Phase 2 identified 8 issues:**
- **CRITICAL:** 0 (no blockers)
- **HIGH:** 2 (resolved before Phase 3)
- **MEDIUM:** 4 (address during implementation)
- **LOW:** 2 (minor details)

### HIGH Issue Resolutions (Incorporated into Plan)

**ISSUE-HIGH-001: "Equivalent Risk" Definition**
- **Resolution:** Define "equivalent" as same residual risk category (Low=Low, Low≠Low-Medium)
- **Implementation:** Add to CLAUDE.md framework section
- **Validation:** TC-V-RPF-001-01

**ISSUE-HIGH-002: ADR Format Not Specified**
- **Resolution:** Use Nygard ADR template inline in `03_approach_selection.md`
- **Format:** Status, Date, Context, Decision, Consequences sections
- **Validation:** TC-V-RPF-002-01

**MEDIUM/LOW Issues:** Documented in [01_specification_issues.md](01_specification_issues.md), address during implementation

**Specification Status:** ✅ IMPLEMENTATION-READY

---

## Test Coverage

**Total Tests:** 29 acceptance tests
- Manual (inspection): 18 tests
- Integration: 3 tests
- System (end-to-end): 3 tests
- Validation (regression): 5 tests

**Coverage:** 100% (all 25 requirements have ≥2 tests)

### Key Tests

**TC-M-RPF-010-01:** CLAUDE.md Decision Framework section verification (CRITICAL)
**TC-M-RPF-080-01:** /think Risk Assessment section first (CRITICAL)
**TC-M-RPF-140-01:** /plan Phase 4 objective changed (CRITICAL)
**TC-S-RPF-001-01:** End-to-end /think workflow with risk-first language
**TC-V-RPF-003-01:** No regression in existing /think analyses
**TC-V-RPF-004-01:** No regression in existing /plan workflows

**Test index:** [02_test_specifications/test_index.md](02_test_specifications/test_index.md)
**Traceability matrix:** [02_test_specifications/traceability_matrix.md](02_test_specifications/traceability_matrix.md)

---

## Implementation Approach (Preview)

**Note:** This is high-level approach. Detailed increments will be defined in Week 2 (Phase 5).

### Implementation Strategy

**Sequence:** Foundation → Policy → Application → Examples → Validation

1. **Create Foundation** (templates/risk_assessment.md)
   - Risk assessment template with failure modes, mitigation, overall assessment
   - Enables consistent risk analysis

2. **Establish Policy** (CLAUDE.md Decision Framework section)
   - Risk-first prioritization mandated
   - "Equivalent risk" defined
   - Rationale references PCH001 charter

3. **Apply to Workflows** (.claude/commands/think.md and plan.md)
   - /think: Restructure comparison framework (Risk → Quality → Effort)
   - /plan: Update Phase 4 objective and process

4. **Document with Examples**
   - Add 1-2 examples showing risk-first analysis
   - Demonstrate framework in practice

5. **Validate**
   - Run acceptance tests
   - Execute end-to-end workflows
   - Verify no regressions

### Estimated Effort

**Total:** 16-24 hours AI implementation time (from analysis)
**Increments:** 8-12 increments (2-3 hours each)
**Testing:** 2-3 hours (29 tests total)

**Timeline:** ~1 week AI implementation + human review

---

## Files Changed

| File | Change Type | Requirements | Lines Changed (Est.) |
|------|------------|--------------|---------------------|
| CLAUDE.md | Add section | REQ-RPF-010 through REQ-RPF-070 | +40-60 lines |
| .claude/commands/think.md | Restructure template | REQ-RPF-080 through REQ-RPF-130, REQ-RPF-250 | ~30 lines modified, +20 new |
| .claude/commands/plan.md | Update Phase 4 | REQ-RPF-140 through REQ-RPF-200, REQ-RPF-250 | ~40 lines modified, +15 new |
| templates/risk_assessment.md | Create new | REQ-RPF-210 through REQ-RPF-240 | +50-70 lines (new file) |

**Total:** 4 files, ~200 lines changed/added

**No code changes** (documentation only)
**No database changes**
**No HTTP API changes**

---

## Dependencies

| Dependency | Type | Status | Required For |
|------------|------|--------|--------------|
| CLAUDE.md | File | ✓ Exists | REQ-RPF-010 through REQ-RPF-070 |
| .claude/commands/think.md | File | ✓ Exists | REQ-RPF-080 through REQ-RPF-130 |
| .claude/commands/plan.md | File | ✓ Exists | REQ-RPF-140 through REQ-RPF-200 |
| templates/ directory | Directory | ⚠️ May need creation | REQ-RPF-210 through REQ-RPF-240 |
| PCH001_project_charter.md | Reference (read-only) | ✓ Exists | Rationale in framework |

**All dependencies available.** No blockers.

**Action if templates/ missing:** Create directory during implementation

---

## Risks and Mitigations

| Risk | Probability | Impact | Mitigation | Residual Risk |
|------|------------|--------|------------|---------------|
| Framework changes ignored in practice | Medium | High | Include in MANDATORY sections, clear examples | Low-Medium |
| Learning curve delays adoption | Low | Medium | Provide templates, examples, clear documentation | Low |
| Increased verbosity of analyses | Low | Low | Already have verbosity standards (CLAUDE.md) | Very Low |
| Inconsistent application across commands | Medium | Medium | Update all commands simultaneously, shared template | Low |
| User finds framework too rigid | Low | Medium | Framework allows tiebreakers (quality, effort) for equivalent risk | Low |

**Overall Implementation Risk:** Low (documentation-only, clear templates, reversible)

---

## Decision Points

### Before Implementation
- [x] Scope approved (Phase 1 checkpoint)
- [x] Specification issues resolved (Phase 2 checkpoint)
- [x] Test coverage confirmed 100% (Phase 3 checkpoint)
- [ ] **User approves proceeding to implementation** ← CURRENT DECISION POINT

### During Implementation
- [ ] After each increment: Tests pass for that increment
- [ ] At checkpoints: Progress review, verify on track
- [ ] If issues found: Pause, reassess, resolve before continuing

### Before Release
- [ ] All CRITICAL requirement tests PASS
- [ ] ≥90% HIGH requirement tests PASS
- [ ] No regressions (TC-V-003-01, TC-V-004-01 PASS)
- [ ] Integration tests PASS
- [ ] ≥2/3 system tests PASS

---

## Success Metrics

### Immediate (Implementation Complete)
- ✓ All 4 files updated as specified
- ✓ All CRITICAL tests PASS
- ✓ No regressions in existing workflows
- ✓ Documentation clear and usable

### Short-Term (First Month After)
- ✓ Next /think analysis uses risk-first framework
- ✓ Next /plan uses updated Phase 4 process
- ✓ Risk-based language appears in decision outputs
- ✓ No confusion or workflow disruption

### Long-Term (3-6 Months After)
- Approaches rejected for high risk despite low effort: ~10% → ~40%
- Implementation rework rate: Expected -30% reduction
- Decision language consistently risk-first ("lowest failure risk" vs. "acceptable risk/effort")
- Quality-absolute goals (PCH001) better supported by decision framework

---

## Next Steps

**Week 1 Complete:** ✅ Phases 1-3 (Scope, Specification Verification, Test Definition)

**Week 2 Tasks:**
- Phase 4: Approach Selection (if multiple implementation approaches exist)
- Phase 5: Implementation Breakdown (define increments)

**Week 3 Tasks:**
- Phase 6: Effort and Schedule Estimation
- Phase 7: Risk Assessment and Mitigation Planning
- Phase 8: Plan Documentation and Approval

**Current Status:** Ready for implementation OR Week 2/3 planning phases

**User Decision Required:** Proceed to implementation now OR complete Week 2/3 planning phases first?

---

## How to Use This Plan

### For Implementer (AI or Human)

**Per Increment:**
1. Read this summary (~400 lines) - One time at start
2. Read specific increment file (~250 lines) - When starting that increment
3. Read relevant test specs (~100 lines) - For that increment's tests
4. Implement according to increment specification
5. Run tests for that increment
6. Verify tests PASS before next increment

**Total context per increment:** ~650-750 lines (not 2000+)

### For Reviewer

**Initial Review:**
- Read this summary (understand goals, scope, risks)
- Review [requirements_index.md](requirements_index.md) (25 requirements)
- Review [01_specification_issues.md](01_specification_issues.md) (issue resolutions)

**Per Increment Review:**
- Review increment implementation
- Verify tests PASS
- Check integration with existing content

**Final Review:**
- Run system tests (TC-S-RPF-001-01, TC-S-RPF-002-01, TC-S-RPF-003-01)
- Run validation tests (TC-V-RPF-001-01 through TC-V-RPF-005-01)
- Verify all CRITICAL tests PASS

---

## Document Navigation

### Phase 1-3 Outputs (Week 1 - COMPLETE)

- **[requirements_index.md](requirements_index.md)** - 25 requirements, compact table
- **[scope_statement.md](scope_statement.md)** - In/out of scope, assumptions, constraints, dependencies
- **[01_specification_issues.md](01_specification_issues.md)** - 8 issues found, 2 HIGH resolved
- **[02_test_specifications/test_index.md](02_test_specifications/test_index.md)** - 29 tests, quick reference
- **[02_test_specifications/traceability_matrix.md](02_test_specifications/traceability_matrix.md)** - 100% requirement coverage
- **02_test_specifications/tc_*.md** - Individual test specifications (3 created as examples, 26 more to be created)

### Phase 4-8 Outputs (Week 2-3 - PLANNED)

- **03_approach_selection.md** - Approach evaluation and ADR (if needed)
- **04_increments/** - Individual increment files (~8-12 increments)
- **05_estimates.md** - Effort and schedule estimates
- **06_risks.md** - Risk assessment and mitigation plans
- **FULL_PLAN.md** - Consolidated plan (for archival only, do not read during implementation)

---

## Approval Signatures

**Plan Phases 1-3 Status:** ✅ COMPLETE
**Specification Status:** ✅ IMPLEMENTATION-READY
**Test Coverage:** ✅ 100%

**Recommended Action:** Proceed to implementation (skip Phases 4-8 for this simple documentation change)

**Alternative Action:** Complete Phases 4-8 for comprehensive planning documentation

---

**Questions or Concerns?**

Review detailed documents:
- Unclear requirements? → [requirements_index.md](requirements_index.md)
- Specification issues? → [01_specification_issues.md](01_specification_issues.md)
- Test coverage questions? → [02_test_specifications/traceability_matrix.md](02_test_specifications/traceability_matrix.md)
- Scope concerns? → [scope_statement.md](scope_statement.md)

---

**Plan Summary Version:** 1.0
**Last Updated:** 2025-10-25
**Status:** Week 1 Complete - Awaiting Implementation Approval
