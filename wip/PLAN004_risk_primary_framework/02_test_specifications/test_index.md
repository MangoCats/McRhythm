# Test Index - Risk-Primary Decision Framework

**Plan:** PLAN004
**Date:** 2025-10-25
**Total Requirements:** 25
**Total Tests:** 29 (some requirements have multiple test cases)

---

## Quick Reference

| Test ID | Requirement(s) | Test Type | One-Line Description |
|---------|---------------|-----------|----------------------|
| TC-M-RPF-010-01 | REQ-RPF-010, REQ-RPF-020 | Manual | CLAUDE.md has Decision-Making Framework section with risk-first priorities |
| TC-M-RPF-030-01 | REQ-RPF-030 | Manual | CLAUDE.md framework states "MUST follow" for all decisions |
| TC-M-RPF-040-01 | REQ-RPF-040 | Manual | Framework requires failure mode identification with probability/impact |
| TC-M-RPF-050-01 | REQ-RPF-050, REQ-RPF-060, REQ-RPF-070 | Manual | Framework specifies risk→quality→effort prioritization sequence |
| TC-M-RPF-080-01 | REQ-RPF-080, REQ-RPF-090 | Manual | /think template includes Risk Assessment section with failure modes |
| TC-M-RPF-100-01 | REQ-RPF-100 | Manual | /think template includes Quality Characteristics section |
| TC-M-RPF-110-01 | REQ-RPF-110 | Manual | /think template includes Implementation Considerations (effort tertiary) |
| TC-M-RPF-120-01 | REQ-RPF-120 | Manual | /think template includes RISK-BASED RANKING section |
| TC-M-RPF-130-01 | REQ-RPF-130 | Manual | /think template recommendation includes risk-based justification |
| TC-M-RPF-140-01 | REQ-RPF-140, REQ-RPF-150 | Manual | /plan Phase 4 objective is "minimal failure risk; acknowledge effort" |
| TC-M-RPF-160-01 | REQ-RPF-160, REQ-RPF-170 | Manual | /plan Phase 4 ranks by residual risk and selects lowest |
| TC-M-RPF-180-01 | REQ-RPF-180, REQ-RPF-190 | Manual | /plan Phase 4 uses quality then effort as tiebreakers |
| TC-M-RPF-200-01 | REQ-RPF-200 | Manual | /plan Phase 4 requires ADR with risk-based justification |
| TC-M-RPF-210-01 | REQ-RPF-210, REQ-RPF-220 | Manual | templates/risk_assessment.md exists with Failure Modes table |
| TC-M-RPF-230-01 | REQ-RPF-230 | Manual | Risk template includes Mitigation Strategies table |
| TC-M-RPF-240-01 | REQ-RPF-240 | Manual | Risk template includes Overall Risk Assessment section |
| TC-M-RPF-250-01 | REQ-RPF-250 | Manual | /think command has at least 1 risk-first example |
| TC-M-RPF-250-02 | REQ-RPF-250 | Manual | /plan command has at least 1 risk-first example |
| TC-I-RPF-001-01 | REQ-RPF-010, REQ-RPF-020, REQ-RPF-030 | Integration | CLAUDE.md framework integrates with existing content seamlessly |
| TC-I-RPF-002-01 | REQ-RPF-080 through REQ-RPF-130 | Integration | /think template produces valid risk-first analysis output |
| TC-I-RPF-003-01 | REQ-RPF-140 through REQ-RPF-200 | Integration | /plan Phase 4 produces valid approach selection with ADR |
| TC-S-RPF-001-01 | All CRITICAL requirements | System | End-to-end: Use new /think workflow for decision analysis |
| TC-S-RPF-002-01 | All /plan requirements | System | End-to-end: Use new /plan workflow for approach selection |
| TC-S-RPF-003-01 | REQ-RPF-030 (enforcement) | System | Verify risk-first language appears in actual decision outputs |
| TC-V-RPF-001-01 | ISSUE-HIGH-001 resolution | Validation | "Equivalent risk" definition clear and unambiguous |
| TC-V-RPF-002-01 | ISSUE-HIGH-002 resolution | Validation | ADR format specified and consistently applied |
| TC-V-RPF-003-01 | All requirements | Validation | No regression: existing /think analyses still valid |
| TC-V-RPF-004-01 | All requirements | Validation | No regression: existing /plan workflows still functional |
| TC-V-RPF-005-01 | All requirements | Validation | Verbosity standards maintained (20-40% reduction target) |

---

## Test Coverage by Requirement

| Requirement | Manual Tests | Integration Tests | System Tests | Validation Tests | Total |
|-------------|--------------|-------------------|--------------|------------------|-------|
| REQ-RPF-010 | TC-M-010-01 | TC-I-001-01 | TC-S-001-01 | TC-V-003-01 | 4 |
| REQ-RPF-020 | TC-M-010-01 | TC-I-001-01 | TC-S-001-01 | - | 3 |
| REQ-RPF-030 | TC-M-030-01 | TC-I-001-01 | TC-S-001-01, TC-S-003-01 | - | 4 |
| REQ-RPF-040 | TC-M-040-01 | - | TC-S-001-01 | - | 2 |
| REQ-RPF-050 | TC-M-050-01 | - | TC-S-001-01 | TC-V-001-01 | 3 |
| REQ-RPF-060 | TC-M-050-01 | - | TC-S-001-01 | TC-V-001-01 | 3 |
| REQ-RPF-070 | TC-M-050-01 | - | TC-S-001-01 | TC-V-001-01 | 3 |
| REQ-RPF-080 | TC-M-080-01 | TC-I-002-01 | TC-S-001-01 | TC-V-003-01 | 4 |
| REQ-RPF-090 | TC-M-080-01 | TC-I-002-01 | TC-S-001-01 | - | 3 |
| REQ-RPF-100 | TC-M-100-01 | TC-I-002-01 | TC-S-001-01 | - | 3 |
| REQ-RPF-110 | TC-M-110-01 | TC-I-002-01 | TC-S-001-01 | - | 3 |
| REQ-RPF-120 | TC-M-120-01 | TC-I-002-01 | TC-S-001-01 | - | 3 |
| REQ-RPF-130 | TC-M-130-01 | TC-I-002-01 | TC-S-001-01 | - | 3 |
| REQ-RPF-140 | TC-M-140-01 | TC-I-003-01 | TC-S-002-01 | TC-V-004-01 | 4 |
| REQ-RPF-150 | TC-M-140-01 | TC-I-003-01 | TC-S-002-01 | - | 3 |
| REQ-RPF-160 | TC-M-160-01 | TC-I-003-01 | TC-S-002-01 | - | 3 |
| REQ-RPF-170 | TC-M-160-01 | TC-I-003-01 | TC-S-002-01 | - | 3 |
| REQ-RPF-180 | TC-M-180-01 | TC-I-003-01 | TC-S-002-01 | TC-V-001-01 | 4 |
| REQ-RPF-190 | TC-M-180-01 | TC-I-003-01 | TC-S-002-01 | TC-V-001-01 | 4 |
| REQ-RPF-200 | TC-M-200-01 | TC-I-003-01 | TC-S-002-01 | TC-V-002-01 | 4 |
| REQ-RPF-210 | TC-M-210-01 | - | TC-S-001-01 | - | 2 |
| REQ-RPF-220 | TC-M-210-01 | - | TC-S-001-01 | - | 2 |
| REQ-RPF-230 | TC-M-230-01 | - | TC-S-001-01 | - | 2 |
| REQ-RPF-240 | TC-M-240-01 | - | TC-S-001-01 | - | 2 |
| REQ-RPF-250 | TC-M-250-01, TC-M-250-02 | - | - | - | 2 |

**Coverage:** 100% (all 25 requirements have at least 2 tests)

---

## Test Types Summary

| Type | Count | Purpose |
|------|-------|---------|
| Manual (TC-M-*) | 18 | Verify documentation content and structure through inspection |
| Integration (TC-I-*) | 3 | Verify components work together (templates + content) |
| System (TC-S-*) | 3 | End-to-end workflow validation |
| Validation (TC-V-*) | 5 | Issue resolution verification and regression testing |
| **Total** | **29** | **Complete coverage with redundancy** |

---

## Test Execution Strategy

### Phase 1: Manual Tests (During Implementation)
Execute manual tests as each component is implemented:
- After CLAUDE.md update: Run TC-M-RPF-010-01, TC-M-RPF-030-01, TC-M-RPF-040-01, TC-M-RPF-050-01
- After /think update: Run TC-M-RPF-080-01 through TC-M-RPF-130-01, TC-M-RPF-250-01
- After /plan update: Run TC-M-RPF-140-01 through TC-M-RPF-200-01, TC-M-RPF-250-02
- After template creation: Run TC-M-RPF-210-01 through TC-M-RPF-240-01

### Phase 2: Integration Tests (After Component Completion)
- After all CLAUDE.md/think/plan changes: Run TC-I-RPF-001-01, TC-I-RPF-002-01, TC-I-RPF-003-01

### Phase 3: System Tests (After All Implementation)
- Run TC-S-RPF-001-01: Execute actual /think with risk-first framework
- Run TC-S-RPF-002-01: Execute actual /plan with new Phase 4
- Run TC-S-RPF-003-01: Verify risk-first language in real outputs

### Phase 4: Validation Tests (Final Verification)
- Run TC-V-RPF-001-01 through TC-V-RPF-005-01
- Verify issue resolutions and no regressions

---

## Pass/Fail Criteria

### Overall Plan Success
- **PASS:** All 9 CRITICAL requirement tests pass + No regressions (TC-V-003-01, TC-V-004-01 pass)
- **PARTIAL:** All CRITICAL pass but 1-2 HIGH requirement tests fail (acceptable with documentation)
- **FAIL:** Any CRITICAL requirement test fails OR >2 HIGH requirement tests fail OR regressions detected

### Test Result Tracking
Each test file contains explicit pass criteria. Track results in traceability matrix.

---

## Test Files

Detailed test specifications in individual files:
- `tc_m_rpf_010_01.md` - CLAUDE.md framework section verification
- `tc_m_rpf_030_01.md` - Mandatory compliance language verification
- `tc_m_rpf_040_01.md` - Failure mode requirement verification
- `tc_m_rpf_050_01.md` - Prioritization sequence verification
- `tc_m_rpf_080_01.md` - /think Risk Assessment section verification
- `tc_m_rpf_100_01.md` - /think Quality Characteristics section verification
- `tc_m_rpf_110_01.md` - /think Implementation Considerations verification
- `tc_m_rpf_120_01.md` - /think RISK-BASED RANKING verification
- `tc_m_rpf_130_01.md` - /think recommendation justification verification
- `tc_m_rpf_140_01.md` - /plan Phase 4 objective verification
- `tc_m_rpf_160_01.md` - /plan Phase 4 ranking and selection verification
- `tc_m_rpf_180_01.md` - /plan Phase 4 tiebreaker logic verification
- `tc_m_rpf_200_01.md` - /plan Phase 4 ADR requirement verification
- `tc_m_rpf_210_01.md` - Risk template Failure Modes table verification
- `tc_m_rpf_230_01.md` - Risk template Mitigation Strategies table verification
- `tc_m_rpf_240_01.md` - Risk template Overall Assessment section verification
- `tc_m_rpf_250_01.md` - /think examples verification
- `tc_m_rpf_250_02.md` - /plan examples verification
- `tc_i_rpf_001_01.md` - CLAUDE.md integration test
- `tc_i_rpf_002_01.md` - /think template integration test
- `tc_i_rpf_003_01.md` - /plan Phase 4 integration test
- `tc_s_rpf_001_01.md` - End-to-end /think workflow test
- `tc_s_rpf_002_01.md` - End-to-end /plan workflow test
- `tc_s_rpf_003_01.md` - Risk-first language enforcement test
- `tc_v_rpf_001_01.md` - ISSUE-HIGH-001 resolution validation
- `tc_v_rpf_002_01.md` - ISSUE-HIGH-002 resolution validation
- `tc_v_rpf_003_01.md` - No regression: existing /think analyses
- `tc_v_rpf_004_01.md` - No regression: existing /plan workflows
- `tc_v_rpf_005_01.md` - Verbosity standards compliance validation

---

**Test Index Status:** Complete
**Next:** Create individual test specification files
