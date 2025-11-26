# PLAN027: Album Matcher Simplicity-First Redesign

**Plan ID:** PLAN027
**Specification:** SPEC_optimal_album_matching_stages.md
**Target:** album_matcher_28.rs (copy from album_matcher_27.rs)
**Status:** Phase 1-3 Complete (Awaiting Approval)
**Created:** 2025-11-25

---

## Executive Summary

**Goal:** Redesign album matching pipeline with simplicity-first stage ordering while preserving all proven algorithms from Run 27. Target 98-100% automatic matching with 1.74x average performance improvement (115s vs 200s per album).

**Core Principle:** Try simple approaches before complex ones. Test default parameters first (Stage 1), then escalate to full 180-parameter sweep (Stage 2) only for albums that need it.

**Key Changes:**
- Add Stage 0: Pre-flight validation (reject invalid inputs)
- Add Stage 1: Default parameter silence detection (70-80% early exit)
- Preserve Stages 2-5 from Run 27 (180-param sweep, DP assembly, quiet spots, merging)
- Add Stage 6: Unmatchable classification (terminal diagnostic)
- Reorder execution: 0 → 1 → 2 → 3 → 4 → 5 → 6 (simplicity-first)

**Performance Target:**
- Current (Run 27): 200s average per album
- Proposed (Run 28): 115s average per album
- Speedup: 1.74x faster (75% faster for majority case, similar for complex cases)

**Success Rate Target:**
- Current: 93-99% automatic
- Proposed: 98-100% automatic
- Unmatchable rate: ≤2% (corrupt files, wrong MB data, non-standard formats)

---

## Requirements Summary

**Total Requirements:** 65 requirements across 7 stages + infrastructure

| Category | Count | Priority Distribution |
|----------|-------|----------------------|
| Stage 0 (Pre-flight) | 8 | P0: 6, P1: 2 |
| Stage 1 (Default params) | 9 | P0: 7, P1: 2 |
| Stage 2 (180-param sweep) | 12 | P0: 10, P1: 2 |
| Stage 3 (DP assembly) | 8 | P0: 6, P1: 2 |
| Stage 4 (Quiet spots) | 8 | P0: 6, P1: 2 |
| Stage 5 (Merging) | 7 | P0: 5, P1: 2 |
| Stage 6 (Unmatchable) | 6 | P0: 5, P1: 1 |
| Infrastructure | 7 | P0: 5, P1: 2 |

**P0 Requirements (Critical):** 51 (78%)
**P1 Requirements (Important):** 13 (20%)
**P2 Requirements (Nice-to-have):** 1 (2%)

---

## Scope Statement

### In Scope

**New Stages:**
- Stage 0: Pre-flight validation (file decodability, duration, MB candidates, single-track detection)
- Stage 1: Default parameter silence detection (single pass with -54dB, 0.6s)
- Stage 6: Unmatchable classification with diagnostic reporting

**Preserved Stages (UNCHANGED from Run 27):**
- Stage 2: Full 180-parameter adaptive sweep with WindowDbProfile caching
- Stage 3: Dynamic programming assembly of over-segmented candidates
- Stage 4: Edition-guided quiet spot detection with RMS profiling
- Stage 5: Adjacent track merging for perfect over-segmentation

**Stage Transition Logic:**
- Simplicity-first progression with explicit rejection criteria
- Early exit on 95% match (Stages 1-2) or 80% match (Stages 3-4)
- Confidence flags (High/Medium/Low) based on stage and match quality
- Fast-fail shortcuts to skip impossible stages

**Performance Optimization:**
- Avoid 180-parameter sweep for 70-80% of albums (early exit via Stage 1)
- Defer WindowDbProfile computation to Stage 2 only
- Maintain existing edition ranking and filtering

### Out of Scope

**NOT changing:**
- MusicBrainz search and filtering logic (Combination A, NDR ranking)
- Edition scoring and runtime filters (±25% duration)
- Cache infrastructure (PLAN026 MusicBrainz caching)
- Artist fallback algorithm
- Time-fit ranking adjustments
- Existing constants and thresholds (except stage-specific)

**NOT adding:**
- New matching algorithms (preserve all Run 27 algorithms)
- New edition ranking strategies
- New cache mechanisms
- GUI changes or user-facing features

### Assumptions

1. **Default parameters are optimal:** -54dB threshold, 0.6s min duration (proven in Run 27)
2. **Stage 1 success rate:** 70-80% of albums match with defaults (to be validated)
3. **Top 3 editions:** 92% of wins occur in top 3 by NDR (from Run 23 data)
4. **180-parameter sweep cost:** ~60-90s including WindowDbProfile computation (empirical)
5. **Preserved algorithm behavior:** Stages 2-5 produce identical results to Run 27
6. **Deterministic matching:** 98-100% of valid albums are algorithmically matchable

### Constraints

1. **Algorithm preservation:** Stages 2-5 must remain UNCHANGED (identical logic and thresholds)
2. **Performance target:** Must achieve ≥1.5x speedup (≤133s avg) to justify 26% code increase
3. **Success rate:** Must achieve ≥98% automatic matching (no regression from Run 27)
4. **Code complexity:** Stage additions justified by performance and success rate gains
5. **Testing coverage:** 100% requirement traceability via acceptance tests

---

## Dependencies

### Existing Code to Preserve

**From album_matcher_27.rs (7324 lines):**
- Stage 2 (180-param sweep): Lines ~2000-3200 (WindowDbProfile, adaptive sweep logic)
- Stage 3 (DP assembly): Lines ~3200-3600 (dp algorithm, assembly testing)
- Stage 4 (Quiet spots): Lines ~3600-3900 (RMS profiling, guided detection)
- Stage 5 (Merging): Lines ~3900-4100 (adjacent track merging)
- MusicBrainz API client: Lines ~380-600 (HTTP requests, rate limiting, caching)
- Edition discovery: Lines ~4500-5500 (search, filter, rank, fetch details)
- Constants: Lines ~610-843 (all tuning parameters)

**Infrastructure dependencies:**
- Symphonia: Audio decoding
- lofty: ID3 tag reading
- Chromaprint: Fingerprinting (for single-track detection)
- Rayon: Parallel edition processing
- Tokio: Async MusicBrainz API
- sha2: Cache key hashing

### New Code to Add

**Stage 0 (~150 lines):**
- File decodability check (decode first 10s)
- Duration validation (reject <60s)
- MusicBrainz candidate validation (reject if 0 candidates)
- Edition runtime filter validation (reject if all outside ±25%)
- Single-track detection (filename patterns, ID3 tags, silence gaps, duration heuristics)

**Stage 1 (~200 lines):**
- Single-pass silence detection with DEFAULT_THRESHOLD_DB, DEFAULT_MIN_DURATION_SECS
- Test against top 3 editions (NDR ranks 1-3)
- Duration comparison with ±1.5s tolerance
- Early exit on ≥95% match
- Rejection criteria: <50% or >150% detected/expected ratio, <95% match

**Stage 6 (~200 lines):**
- Comprehensive diagnostic report generation
- Unmatchable reason classification (corrupt, wrong MB, non-standard, edge case)
- Log all stage results and best attempt
- Suggest user actions (override edition, report MB issue, accept unmatchable)

**Control Flow Refactoring (~100 lines):**
- Main match_album() function restructure for 0→6 progression
- Stage transition logic with explicit rejection criteria
- Confidence flag assignment (High/Medium/Low)
- Fast-fail shortcuts (skip stages based on prior results)

**Total New Code:** ~650 lines (+8.9% vs Run 27)

---

## Specification Completeness Analysis

**Phase 2 Results:** 18 specification issues identified

| Severity | Count | Action Required |
|----------|-------|-----------------|
| CRITICAL | 3 | Must resolve before implementation |
| HIGH | 6 | Should resolve, may proceed with caution |
| MEDIUM | 7 | Document decisions, address during implementation |
| LOW | 2 | Minor clarifications, address as needed |

**Critical Issues:**
1. **CRIT-01:** Stage 1 success rate assumption (70-80%) unvalidated - requires empirical test
2. **CRIT-02:** Default parameters (-54dB, 0.6s) may not be from configuration constants - verify source
3. **CRIT-03:** WindowDbProfile computation cost (60-90s) conflicts with Stage 2 total cost - clarify breakdown

See [01_specification_issues.md](01_specification_issues.md) for full analysis.

---

## Test Coverage Summary

**Total Test Cases:** 78 tests covering 65 requirements

| Test Type | Count | Coverage |
|-----------|-------|----------|
| Unit Tests | 48 | 73.8% of tests |
| Integration Tests | 24 | 30.8% of tests |
| System Tests | 6 | 7.7% of tests |

**Traceability:** 100% requirement coverage (every requirement has ≥1 test)

**Key Test Scenarios:**
- Stage 0: Invalid file rejection (corrupt, too short, no MB matches, single tracks)
- Stage 1: Clear-gap albums with default parameters (early exit verification)
- Stage 2: Parameter-sensitive albums requiring adaptive sweep
- Stage 3: Over-segmented albums requiring DP assembly
- Stage 4: Continuous audio requiring quiet spot detection
- Stage 5: Perfect over-segmentation requiring merging
- Stage 6: Unmatchable classification and diagnostic quality
- Performance: Weighted average timing validation (≤115s target)

See [02_test_specifications/test_index.md](02_test_specifications/test_index.md) for full test catalog.

---

## Implementation Phases

**Phase 1: Validation (1-2 weeks)**
- Extract Stage 1 default parameters from Run 27 configuration
- Implement Stage 1 alongside Run 27 (parallel execution)
- Run on existing album dataset (179 albums from Run 27)
- Validate 70-80% success rate
- If <70%, adjust parameters or add 2-3 default sets

**Phase 2: Stage 0 Implementation (1 week)**
- Extract existing validation logic into explicit Stage 0
- Add single-track detection consolidation
- Test on invalid files (corrupt, too short, singles)
- Validate 5-10% rejection rate

**Phase 3: Stage 1 Integration (1-2 weeks)**
- Add Stage 1 as first matching attempt
- Implement rejection criteria and Stage 2 transition
- Test early exit on clear-gap albums
- Validate no regression vs Run 27

**Phase 4: Stage 6 Implementation (1 week)**
- Implement diagnostic report generation
- Add unmatchable reason classification
- Test on Run 27 failures
- Validate diagnostic quality

**Phase 5: Control Flow Refactor (1-2 weeks)**
- Restructure main loop for 0→6 progression
- Implement confidence flags and fast-fail shortcuts
- Preserve Stages 2-5 logic (no changes)
- Full regression testing vs Run 27

**Phase 6: Performance Validation (1 week)**
- Full dataset run (179 albums)
- Measure weighted average timing (target ≤115s)
- Measure automatic success rate (target ≥98%)
- Measure unmatchable rate (target ≤2%)
- A/B comparison vs Run 27

**Total Timeline:** 6-9 weeks

---

## Success Criteria

**Must Have (Go/No-Go):**
- [ ] Automatic success rate ≥98% (no regression from Run 27's 93-99%)
- [ ] Average time ≤150s per album (≥1.33x speedup)
- [ ] Stage 1 success rate ≥70% (validates simplicity-first approach)
- [ ] 100% test coverage per traceability matrix
- [ ] All CRITICAL specification issues resolved

**Should Have (Quality Gates):**
- [ ] Automatic success rate ≥99% (stretch goal)
- [ ] Average time ≤120s per album (≥1.67x speedup)
- [ ] Stage 1 success rate ≥80% (strong simplicity-first)
- [ ] Unmatchable rate ≤1% (ideal)
- [ ] All HIGH specification issues resolved

**Could Have (Aspirational):**
- [ ] Average time ≤115s per album (1.74x speedup per spec)
- [ ] False positive rate ≤1% (high confidence)
- [ ] False negative rate = 0% (no missed deterministic matches)

---

## Risk Assessment

**Risk 1: Stage 1 Success Rate Below Target (MEDIUM)**
- **Impact:** Less performance gain than expected (<1.5x speedup)
- **Mitigation:** Empirical validation in Phase 1, adjust parameters if needed
- **Residual:** LOW (Stage 2 provides comprehensive fallback)

**Risk 2: Performance Regression for Complex Albums (LOW)**
- **Impact:** Albums requiring Stage 2+ take longer (Stage 1 overhead + Stage 2 time)
- **Mitigation:** Fast-fail rejection criteria, skip Stage 1 for extreme cases
- **Residual:** LOW (overall average still improves due to 70-80% early exit)

**Risk 3: Code Complexity Increase (LOW)**
- **Impact:** +650 lines (+8.9%) more code to maintain
- **Mitigation:** Stage 0 consolidates existing logic, Stage 1 is simple, Stages 2-5 unchanged
- **Residual:** LOW (complexity increase is minimal and well-structured)

**Risk 4: Specification Assumptions Invalid (HIGH)**
- **Impact:** Critical assumptions (70-80% Stage 1 success, 60-90s sweep cost) may be wrong
- **Mitigation:** Phase 1 empirical validation BEFORE full implementation
- **Residual:** MEDIUM (dependent on Phase 1 results)

---

## Approval Required

**Stakeholder Decision Points:**

1. **Proceed with Phase 1 validation?** (Extract and test Stage 1 on existing dataset)
   - [ ] APPROVED
   - [ ] REJECTED
   - [ ] DEFER (needs more analysis)

2. **Accept 26% code increase (+550 lines)?** (Justified by 1.74x speedup and 98% success rate)
   - [ ] APPROVED
   - [ ] REJECTED
   - [ ] DEFER (needs cost-benefit review)

3. **Proceed with CRITICAL issue resolutions?** (3 critical issues require resolution before implementation)
   - [ ] APPROVED (resolve during Phase 1)
   - [ ] REJECTED (specification needs revision)
   - [ ] DEFER (needs technical review)

**Next Steps After Approval:**
- Resolve CRITICAL specification issues (see 01_specification_issues.md)
- Begin Phase 1 validation (extract Stage 1, test on dataset)
- Report Phase 1 results (success rate, timing, parameter validation)
- Proceed to Phase 2+ only if Phase 1 meets success criteria

---

## Document Inventory

This plan consists of the following documents:

1. **00_PLAN_SUMMARY.md** (this document) - Executive overview and decision framework
2. **requirements_index.md** - Compact requirements table (65 requirements)
3. **scope_statement.md** - Detailed in/out scope, assumptions, constraints
4. **dependencies_map.md** - Existing code preservation and new code additions
5. **01_specification_issues.md** - Phase 2 completeness analysis (18 issues)
6. **02_test_specifications/test_index.md** - Test catalog (78 tests)
7. **02_test_specifications/traceability_matrix.md** - Requirements ↔ tests mapping

**Plan Size:** ~1200 lines total (meets <1500 line target for modular plans)

---

## References

- **Specification:** SPEC_optimal_album_matching_stages.md (947 lines)
- **Current Implementation:** album_matcher_27.rs (7324 lines)
- **Empirical Data:** Run 27 results (179 albums, 200s average, 93-99% success)
- **Edition Ranking Analysis:** album_matcher_25_designs.md (Run 23 edition data)
- **Cache Implementation:** PLAN026 (MusicBrainz API caching)

---

**Plan Status:** Phase 1-3 Complete, Awaiting Stakeholder Approval
**Next Action:** Review specification issues, approve Phase 1 validation
**Created:** 2025-11-25
**Last Updated:** 2025-11-25
