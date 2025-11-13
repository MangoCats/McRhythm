# Effort and Schedule Estimation: WKMP-AI Audio Import System Recode

**Plan:** PLAN024
**Created:** 2025-11-09
**Purpose:** Estimate effort, schedule, and resource allocation for implementation

**Phase:** Phase 6 - Effort and Schedule Estimation

---

## Summary

**Total Effort:** 59.5 developer-days (11.9 weeks at 5 days/week)
**Schedule:** 14 weeks (includes testing, buffer, documentation)
**Team Size:** 1 developer (ground-up recode per SPEC030)
**Start Date:** TBD (after plan approval)
**Target Completion:** 14 weeks from start

**Changes from original estimate:**
- Amendment 8: +3.5 days
  - Added TASK-000 (File-Level Import Tracking): +2 days
  - Updated TASK-019 (Workflow Orchestrator Phase -1/7): +1 day
  - Updated TASK-021 (User Approval API Endpoints): +0.5 days
- Amendment 9: +1 day
  - Updated TASK-019 (Workflow Orchestrator Discovery Phase): +1 day
- **Total additional effort:** +4.5 days (8% increase)
- **Schedule unchanged:** 14 weeks (buffer absorbs additional effort)

---

## Effort Breakdown by Phase

| Phase | Tasks | Days | % of Total |
|-------|-------|------|------------|
| Infrastructure (Weeks 1-2) | TASK-000 to TASK-004 | 10.5 | 18% |
| Tier 1 Extractors (Weeks 3-5) | TASK-005 to TASK-011 | 17 | 29% |
| Tier 2 Fusion (Weeks 6-8) | TASK-012 to TASK-015 | 11.5 | 19% |
| Tier 3 Validation (Weeks 9-10) | TASK-016 to TASK-018 | 5.5 | 9% |
| Orchestration (Weeks 11-12) | TASK-019 to TASK-021 | 12 | 20% |
| Integration & Testing (Weeks 13-14) | TASK-022 to TASK-025 | 9 | 15% |
| **Total** | **26 tasks** | **59.5 days** | **100%** |
| **Buffer (20%)** | - | **11.9 days** | - |
| **Contingency Total** | - | **71.4 days** | **~14 weeks** |

---

## Buffer Structure and Allocation

**Single-Level Buffer (20% overall, distributed by phase):**

The plan uses a **single 20% buffer** (11.9 days) distributed across phases based on risk level. There are NOT two separate buffers (phase-level + overall).

**How Buffer Works:**
- **Base Effort (tasks only):** 59.5 days
- **Overall Buffer (20% of base):** 11.9 days
- **Total Schedule with Buffer:** 71.4 days (~14 weeks at 5 days/week)

**Phase Buffer Allocation (portions of the 11.9-day total):**
- Infrastructure: 1 day (10% of phase - lower risk, established patterns)
- Tier 1 Extractors: 2.5 days (15% of phase - API integration risk)
- Tier 2 Fusion: 1.5 days (15% of phase - mathematical correctness critical)
- Tier 3 Validation: 0.5 days (10% of phase - low complexity)
- Orchestration: 2 days (20% of phase - highest integration complexity)
- Integration & Testing: 1 day (10% of phase - typically predictable)
- **Total Allocated:** ~8.5 days (remaining 3.2 days as general contingency)

**Important Clarifications:**
- The "Buffer" rows in detailed phase tables (below) show **portions of the overall 11.9-day buffer**, NOT additional buffers
- The "Days" column in the breakdown table (above) shows **task effort only** (no buffer included)
- The "Subtotal" rows in detailed tables show **tasks + allocated buffer portion**

**Example - Infrastructure Phase:**
- Tasks: 10.5 days (TASK-000 through TASK-004)
- Buffer allocation: 1 day (part of overall 11.9-day buffer)
- Subtotal: 11.5 days

This 1 day is NOT on top of the 11.9-day overall buffer—it's part of it, allocated to Infrastructure based on 10% risk buffer.

---

## Detailed Task Estimates

### Infrastructure (10.5 days)

| Task | Effort | Assumptions |
|------|--------|-------------|
| TASK-000: File-Level Import Tracking | 2 days | Skip logic decision tree, confidence aggregation, metadata merge |
| TASK-001: SPEC031 Verification | 2 days | If missing: 2 days implement; if exists: 0.5 days |
| TASK-002: Chromaprint FFI | 3 days | C library well-documented, FFI patterns known |
| TASK-003: Schema Sync | 2 days | 24 columns (17 passages + 7 files) |
| TASK-004: Base Traits | 2 days | Trait design clear from spec |
| Buffer (10%) | 1 day | Increased from 0.5 days due to additional complexity |
| **Subtotal** | **11.5 days** | **~2.5 weeks** |

### Tier 1 Extractors (17 days)

| Task | Effort | Assumptions |
|------|--------|-------------|
| TASK-005: ID3 Extractor | 1.5 days | `id3` crate exists, API simple |
| TASK-006: Chromaprint Analyzer | 2 days | FFI wrapper complete (TASK-002) |
| TASK-007: AcoustID Client | 2.5 days | API documented, rate limiting via `tower` |
| TASK-008: MusicBrainz Client | 3 days | XML parsing, User-Agent, rate limiting |
| TASK-009: Essentia Analyzer | 2.5 days | Command execution, JSON parsing |
| TASK-010: AudioDerived Extractor | 4 days | Custom DSP algorithms (most complex) |
| TASK-011: ID3 Genre Mapper | 1.5 days | HashMap-based, 50+ genres |
| Buffer (15%) | 2.5 days | Higher buffer for API integration risks |
| **Subtotal** | **19.5 days** | **~4 weeks** |

### Tier 2 Fusion (11.5 days)

| Task | Effort | Assumptions |
|------|--------|-------------|
| TASK-012: Identity Resolver | 4 days | Bayesian math verified via unit tests |
| TASK-013: Metadata Fuser | 2.5 days | Field-wise selection straightforward |
| TASK-014: Flavor Synthesizer | 3 days | Weighted averaging, normalization |
| TASK-015: Boundary Fuser | 2 days | Tick conversion available (SPEC017) |
| Buffer (15%) | 1.5 days | Mathematical correctness critical |
| **Subtotal** | **13 days** | **~2.5 weeks** |

### Tier 3 Validation (5.5 days)

| Task | Effort | Assumptions |
|------|--------|-------------|
| TASK-016: Consistency Validator | 2.5 days | Levenshtein via `strsim` crate |
| TASK-017: Completeness Scorer | 1.5 days | Simple formula implementation |
| TASK-018: Quality Scorer | 1.5 days | Weighted average |
| Buffer (10%) | 0.5 days | Low complexity, low risk |
| **Subtotal** | **6 days** | **~1.5 weeks** |

### Orchestration (12 days)

| Task | Effort | Assumptions |
|------|--------|-------------|
| TASK-019: Workflow Orchestrator | 7 days | Discovery Phase + Phase -1 + Phase 0-7 (was 5 days originally, +1d Amendment 8, +1d Amendment 9) |
| TASK-020: SSE Event System | 2.5 days | Axum SSE support exists, Discovery events added |
| TASK-021: HTTP API Endpoints | 2.5 days | Standard routes + user approval endpoints + Discovery request format (was 2 days) |
| Buffer (20%) | 2.4 days | Integration complexity (20% of 12 days) |
| **Subtotal** | **14.4 days** | **~3 weeks** |

### Integration & Testing (9 days)

| Task | Effort | Assumptions |
|------|--------|-------------|
| TASK-022: Integration Tests | 3 days | Acceptance tests defined (Phase 3) |
| TASK-023: System Tests | 2 days | Test data prepared |
| TASK-024: Performance Testing | 2 days | Benchmarking straightforward |
| TASK-025: Documentation | 2 days | IMPL docs, inline comments |
| Buffer (10%) | 1 days | Testing typically predictable |
| **Subtotal** | **10 days** | **~2 weeks** |

---

## Schedule Timeline

### Week-by-Week Breakdown

**Weeks 1-2: Infrastructure**
- Week 1: File-Level Import Tracking (TASK-000, 2d), SPEC031 verification, Chromaprint FFI start (1d/3d)
- Week 2: Chromaprint FFI complete (2d), Schema sync with 24 columns (2d), Base traits (1d)

**Weeks 3-5: Tier 1 Extractors**
- Week 3: ID3 (1.5d), Chromaprint Analyzer (2d), AudioDerived start (1.5d/4d)
- Week 4: AudioDerived complete (2.5d), AcoustID Client (2.5d)
- Week 5: MusicBrainz Client (3d), Essentia (2.5d), Genre Mapper (1.5d) - overlap

**Weeks 6-8: Tier 2 Fusion**
- Week 6: Identity Resolver (4d), Metadata Fuser start (1d/2.5d)
- Week 7: Metadata Fuser complete (1.5d), Flavor Synthesizer (3d)
- Week 8: Boundary Fuser (2d), buffer (3d)

**Weeks 9-10: Tier 3 Validation**
- Week 9: Consistency Validator (2.5d), Completeness Scorer (1.5d), Quality Scorer (1d/1.5d)
- Week 10: Quality Scorer complete (0.5d), buffer (4d)

**Weeks 11-12: Orchestration**
- Week 11: Workflow Orchestrator with Discovery Phase + Phase -1/7 (7d)
- Week 12: SSE Event System with Discovery events (2.5d), HTTP API with approval + Discovery endpoints (2.5d), buffer (2d)

**Weeks 13-14: Integration & Testing**
- Week 13: Integration Tests (3d), System Tests (2d)
- Week 14: Performance Testing (2d), Documentation (2d), final buffer (1d)

---

## Critical Path Analysis

**Critical Path:** 57.5 days (sequential dependencies, +1.5 days from Amendment 8, +1 day from Amendment 9)

**Tasks on Critical Path:**
1. SPEC031 Verification (TASK-001) - blocks schema sync
2. Schema Sync (TASK-003) - blocks parameter loading
3. Base Traits (TASK-004) - blocks all extractors
4. Chromaprint FFI (TASK-002) → Chromaprint Analyzer (TASK-006) → AcoustID (TASK-007) - sequential chain
5. All Tier 1 → Identity Resolver (TASK-012) - needs all sources
6. All Tier 2 → Workflow Orchestrator (TASK-019, 7d) - needs all fusion, includes Discovery Phase
7. Workflow → SSE → HTTP API (TASK-021, 2.5d) - sequential
8. All Implementation → Integration Tests - sequential

**Note:** TASK-000 (File-Level Import Tracking) coding parallelizes with TASK-001, but integration testing requires TASK-003 complete (database schema with 7 new file columns). Does not extend critical path.

**Parallelization Gains:**
- Tier 1 extractors: 7 tasks, but some sequential (Chromaprint chain) = ~15% time savings vs pure sequential
- Tier 3 validators: 3 tasks, fully parallel = ~30% time savings

**Without Parallelization:** 69.5 days (+3.5 days from Amendment 8, +1 day from Amendment 9)
**With Parallelization:** 57.5 days (+1.5 days on critical path from Amendment 8, +1 day from Amendment 9)
**Efficiency Gain:** 17.6%

---

## Resource Allocation

**Developer:** 1 full-time (per SPEC030 ground-up recode guidance)

**Workload Distribution:**
- 50% coding (27.5 days)
- 25% testing (13.75 days)
- 15% debugging (8.25 days)
- 10% documentation (5.5 days)

**Peak Complexity Weeks:**
- Week 6: Identity Resolver (Bayesian math)
- Week 11: Workflow Orchestrator (integration complexity)

**Lower Complexity Weeks:**
- Week 9-10: Validation modules (straightforward)
- Week 14: Documentation

---

## Assumptions

**Development Environment:**
- Rust toolchain stable (already installed)
- Chromaprint library available via system package manager
- Essentia optional (graceful degradation)
- Test data prepared per 03_acceptance_tests.md

**Dependencies:**
- SPEC031 exists in wkmp-common (2-day contingency if missing)
- SPEC017 tick utilities available (assumed yes)
- External APIs (AcoustID, MusicBrainz, AcousticBrainz) remain stable

**Developer Skill Level:**
- Experienced Rust developer (familiar with tokio, axum)
- Familiar with FFI (Chromaprint wrapper feasible)
- Familiar with WKMP codebase

**Working Hours:**
- 5 days/week, 8 hours/day
- No major holidays in 14-week window
- No other concurrent projects

---

## Risks to Schedule

**High Risk:**
- SPEC031 not implemented (+2 days if missing)
- Bayesian algorithm bugs (+2-3 days for debugging/correction)
- AudioDerived algorithm performance issues (+1-2 days optimization)

**Medium Risk:**
- API rate limiting more strict than documented (+1 day adjustment)
- Chromaprint FFI memory leaks (+1 day debugging)
- Test data acquisition delays (+0.5 days)

**Mitigation:**
- 20% buffer included (11.9 days) covers most risks
- Early verification of SPEC031 (Week 1) minimizes discovery delay
- Mathematical unit tests for Bayesian algorithm (catch bugs early)

---

## Milestones

| Milestone | Week | Deliverable | Success Criteria |
|-----------|------|-------------|------------------|
| M1: Infrastructure Complete | Week 2 | SPEC031 verified, FFI wrapper working, file tracking implemented | Chromaprint generates fingerprints, skip logic works |
| M2: Tier 1 Extractors Complete | Week 5 | All 7 extractors implemented | Unit tests >90% coverage per module |
| M3: Tier 2 Fusion Complete | Week 8 | Identity, metadata, flavor fusion working | Integration tests passing |
| M4: Tier 3 Validation Complete | Week 10 | Quality scoring working | Quality scores computed correctly |
| M5: Orchestration Complete | Week 12 | Full pipeline working (Discovery Phase through Phase 7) | End-to-end import with file discovery, skip logic, and user approval works |
| M6: Testing Complete | Week 14 | All acceptance tests passing | >90% total coverage, performance met |

---

## Contingency Plans

**If Schedule Slips (>2 weeks behind):**
1. Reduce scope: Skip Essentia integration (use AudioDerived only) - saves 2.5 days
2. Reduce scope: Skip ID3 Genre Mapper - saves 1.5 days
3. Reduce validation: Skip genre-flavor alignment check - saves 0.5 days
4. Total contingency: 4.5 days recoverable

**If Ahead of Schedule:**
1. Implement additional extractors (AcousticBrainz direct if time permits)
2. Enhance AudioDerived with more sophisticated algorithms
3. Improve test coverage beyond 90%

---

**Document Version:** 3.0 (Updated for Amendment 9)
**Last Updated:** 2025-11-09
**Phase 6 Status:** ✅ COMPLETE (Updated with Amendments 8 and 9)

**Amendment 9 Updates:**
- Base Effort: 58.5 days → 59.5 days (+1 day)
- Buffer: 11.7 days → 11.9 days (20% of base)
- Total Schedule: 70.2 days → 71.4 days (~14 weeks, unchanged)
- Critical Path: 56.5 days → 57.5 days (+1 day on critical path)
- Orchestration Phase: 11 days → 12 days (TASK-019: 6d → 7d for Discovery Phase)
- Milestone M5 updated to include Discovery Phase

**MED Clarifications Applied (Amendment 8):**
- MED-003: Added buffer structure clarification (single 20% buffer distributed by phase)
- MED-001: Clarified TASK-000 dependency (coding parallelizes, integration testing requires TASK-003)
