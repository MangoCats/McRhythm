# Implementation Plan: Embedded MusicBrainz ID Matching (Stage 0)

**Plan ID:** PLAN-EMBID-001
**Specification:** SPEC-EMBID-001
**Status:** Ready for Implementation
**Date:** 2024-12-14
**Estimated Effort:** 8-12 hours
**Confidence:** HIGH (existing infrastructure, well-understood domain)

---

## Executive Summary

This plan implements a "Stage 0" matching strategy that prioritizes embedded MusicBrainz Recording IDs from ID3 tags, achieving **98.5% coverage** (up from 80.9%) with higher confidence and faster execution.

**Key Benefits:**
- 17.6% coverage improvement
- No API calls for 98% of files
- Higher confidence (user-approved tags)
- Works offline

**Implementation Approach:** Enhance existing `ID3Extractor` to extract ISRC and add confidence tier logic to `IdentityResolver`.

---

## Quick Navigation

| Document | Purpose | When to Read |
|----------|---------|--------------|
| This file | Overview | Always start here |
| [requirements_index.md](requirements_index.md) | All requirements | Planning |
| [01_specification_issues.md](01_specification_issues.md) | Spec gaps found | Before starting |
| [02_test_specifications/](02_test_specifications/) | Test definitions | When implementing |
| [04_increments/](04_increments/) | Implementation steps | During coding |

---

## Requirements Summary

| Category | Count | P0 | P1 | P2 |
|----------|-------|----|----|----|
| Functional | 8 | 4 | 3 | 1 |
| Non-Functional | 4 | 1 | 1 | 2 |
| **Total** | **12** | **5** | **4** | **3** |

**Existing Code Leverage:** 3 requirements already implemented (FR-01 partial, FR-03, FR-06)

---

## Specification Issues (Phase 2)

| Severity | Count | Resolution |
|----------|-------|------------|
| CRITICAL | 0 | - |
| HIGH | 1 | Add MB Release ID extraction for future use |
| MEDIUM | 2 | Error handling for corrupt tags, caching strategy |
| LOW | 1 | Logging format |

**Status:** No blocking issues. HIGH issue noted for future enhancement.

---

## Test Coverage Summary (Phase 3)

| Test Type | Count | Coverage |
|-----------|-------|----------|
| Unit | 8 | 100% of FR-01 through FR-04 |
| Integration | 4 | 100% of FR-05 through FR-08 |
| System | 2 | NFR-01 through NFR-04 |
| **Total** | **14** | **100%** |

**Traceability:** All 12 requirements have at least one test.

---

## Implementation Increments (Phase 5)

| # | Increment | Est. Hours | Dependencies | Tests |
|---|-----------|------------|--------------|-------|
| 1 | Add ISRC extraction to ID3Extractor | 1-2h | None | TC-U-001, TC-U-002 |
| 2 | Define ConfidenceTier enum and logic | 2-3h | I1 | TC-U-003, TC-U-004 |
| 3 | Update IdentityResolver for Stage 0 | 2-3h | I2 | TC-I-001, TC-I-002 |
| 4 | Add logging and metrics | 1-2h | I3 | TC-U-005, TC-I-003 |
| 5 | Integration testing and validation | 2-3h | I4 | TC-S-001, TC-S-002 |

**Total:** 8-13 hours (expected: 10 hours)
**Critical Path:** I1 → I2 → I3 → I4 → I5

---

## Approach Selected (Phase 4)

**Selected: Approach A - Enhance Existing Pipeline**

Modify `ID3Extractor` and `IdentityResolver` rather than creating new components.

**Risk-Based Justification:**
- Residual risk: LOW (uses proven, tested code)
- Alternative (new Stage 0 service) has MEDIUM risk due to integration complexity
- Effort comparable (~10h vs ~12h for new service)

---

## Risk Summary (Phase 7)

| Risk | Probability | Impact | Mitigation | Residual |
|------|-------------|--------|------------|----------|
| R-001: Corrupt ID3 tags cause panic | LOW | MAJOR | Add error handling | LOW |
| R-002: ISRC format variations | MEDIUM | MINOR | Normalize ISRC | LOW |
| R-003: Performance regression | LOW | MODERATE | Benchmark before/after | LOW |

**Overall Risk:** LOW - Well-understood domain with existing tested code.

---

## Estimates (Phase 6)

| Scenario | Hours | Probability |
|----------|-------|-------------|
| Optimistic | 8h | 20% |
| Expected | 10h | 60% |
| Pessimistic | 13h | 20% |

**Planning Estimate:** 10h + 2h buffer = **12 hours**
**Confidence:** HIGH (±25%)

---

## Success Criteria

| Metric | Target | Verification |
|--------|--------|--------------|
| Coverage | >= 98% | Run on test dataset |
| Tier 1 rate | >= 95% | Analyze match results |
| Speed | < 50ms/file Stage 0 | Benchmark |
| False positives | < 0.1% | Manual review sample |

---

## Checkpoints

1. **After Increment 2:** Tier logic complete, unit tests pass
2. **After Increment 4:** Full pipeline working, integration tests pass
3. **After Increment 5:** Validation complete, ready for production

---

## Next Steps

1. Review this plan summary
2. Read [04_increments/increment_01.md](04_increments/increment_01.md) to start
3. Implement increment by increment
4. Run tests after each increment
5. Use `/commit` after tests pass
