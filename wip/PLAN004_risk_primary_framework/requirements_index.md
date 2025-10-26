# Requirements Index - Risk-Primary Decision Framework

**Source:** [wip/_deprioritize_effort_analysis_results.md](../_deprioritize_effort_analysis_results.md) - Approach 3
**Date:** 2025-10-25
**Feature:** Risk-Primary Decision Framework Implementation

---

## Requirements Summary

| Req ID | Type | Brief Description | Source Line | Priority |
|--------|------|-------------------|-------------|----------|
| REQ-RPF-010 | Documentation | Add "Decision-Making Framework - MANDATORY" section to CLAUDE.md | 386-409 | CRITICAL |
| REQ-RPF-020 | Documentation | Framework MUST prioritize risk (primary), quality (secondary), effort (tertiary) | 390-405 | CRITICAL |
| REQ-RPF-030 | Process | All design decisions MUST follow risk-first framework | 390 | CRITICAL |
| REQ-RPF-040 | Process | Risk assessment MUST identify failure modes with probability and impact | 392-394 | HIGH |
| REQ-RPF-050 | Process | Rank approaches by failure risk (lowest = highest rank) | 396 | HIGH |
| REQ-RPF-060 | Process | Quality evaluated among equivalent-risk approaches | 398-400 | HIGH |
| REQ-RPF-070 | Process | Effort considered only among equivalent risk+quality approaches | 402-405 | HIGH |
| REQ-RPF-080 | Documentation | /think command comparison framework restructured (risk → quality → effort) | 411-443 | CRITICAL |
| REQ-RPF-090 | Template | /think output includes Risk Assessment section with failure modes | 415-421 | HIGH |
| REQ-RPF-100 | Template | /think output includes Quality Characteristics section | 423-426 | HIGH |
| REQ-RPF-110 | Template | /think output includes Implementation Considerations section (effort tertiary) | 428-431 | HIGH |
| REQ-RPF-120 | Template | /think output includes RISK-BASED RANKING of approaches | 435-438 | HIGH |
| REQ-RPF-130 | Template | /think recommendation explicitly states risk-based justification | 440-442 | HIGH |
| REQ-RPF-140 | Documentation | /plan Phase 4 objective changed to "minimal failure risk; acknowledge effort" | 445-460 | CRITICAL |
| REQ-RPF-150 | Process | /plan Phase 4 MUST perform risk assessment for each approach | 451-452 | CRITICAL |
| REQ-RPF-160 | Process | /plan Phase 4 MUST rank by residual risk (after mitigation) | 455 | CRITICAL |
| REQ-RPF-170 | Process | /plan Phase 4 selects lowest-risk approach | 456 | CRITICAL |
| REQ-RPF-180 | Process | /plan Phase 4 uses quality as tiebreaker for equivalent risk | 457 | HIGH |
| REQ-RPF-190 | Process | /plan Phase 4 uses effort as final tiebreaker for equivalent risk+quality | 458 | MEDIUM |
| REQ-RPF-200 | Documentation | /plan Phase 4 decision documented as ADR with risk-based justification | 459 | HIGH |
| REQ-RPF-210 | Template | Create templates/risk_assessment.md template file | 462-487 | CRITICAL |
| REQ-RPF-220 | Template | Risk assessment template includes Failure Modes table | 468-473 | HIGH |
| REQ-RPF-230 | Template | Risk assessment template includes Mitigation Strategies table | 475-480 | HIGH |
| REQ-RPF-240 | Template | Risk assessment template includes Overall Risk Assessment section | 482-486 | HIGH |
| REQ-RPF-250 | Documentation | Update examples in /think and /plan commands to reflect new framework | 521 | MEDIUM |

---

## Requirements by Category

### CRITICAL (Implementation Blockers)
- REQ-RPF-010: CLAUDE.md decision framework section
- REQ-RPF-020: Risk/Quality/Effort prioritization
- REQ-RPF-030: Mandatory framework compliance
- REQ-RPF-080: /think framework restructure
- REQ-RPF-140: /plan Phase 4 objective change
- REQ-RPF-150: /plan risk assessment requirement
- REQ-RPF-160: /plan risk ranking requirement
- REQ-RPF-170: /plan lowest-risk selection
- REQ-RPF-210: Risk assessment template creation

**Total CRITICAL:** 9 requirements

### HIGH (Quality Requirements)
- REQ-RPF-040 through REQ-RPF-070: Process requirements for risk/quality/effort evaluation
- REQ-RPF-090 through REQ-RPF-130: /think template requirements
- REQ-RPF-180, REQ-RPF-200: /plan tiebreaker and ADR requirements
- REQ-RPF-220 through REQ-RPF-240: Risk template components

**Total HIGH:** 13 requirements

### MEDIUM (Enhancement)
- REQ-RPF-190: Effort tiebreaker (fallback)
- REQ-RPF-250: Example updates

**Total MEDIUM:** 2 requirements

---

## Total Requirements: 25

**Breakdown:**
- Documentation changes: 7 requirements
- Process changes: 10 requirements
- Template creation: 8 requirements

---

## Traceability to Analysis

All requirements extracted from:
- **Source Document:** wip/_deprioritize_effort_analysis_results.md
- **Section:** Approach 3: Risk-Primary Decision Framework (Recommended)
- **Lines:** 379-524
- **Recommendation Status:** RECOMMENDED approach by /think analysis
