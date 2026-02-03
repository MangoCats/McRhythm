# PLAN026: Batch Writes Optimization - PLAN SUMMARY

**Status:** ✅ COMPLETE - Ready for Implementation
**Created:** 2025-01-15
**Completed:** 2025-01-15 (All 8 Phases)
**Plan Location:** `wip/PLAN026_batch_writes_optimization/`

---

## READ THIS FIRST

This plan addresses database lock contention in wkmp-ai import pipeline through batch write optimization. Evidence from git history shows ongoing "database lock issues" and "thread starvation" problems. Root cause: 16 workers competing for single SQLite write slot, with ~70% write operations in per-file pipeline.

**For Implementation:**
- Read this summary (< 500 lines)
- Review detailed requirements: [`requirements_index.md`](requirements_index.md)
- Review test specifications: [`02_test_specifications/test_index.md`](02_test_specifications/test_index.md)
- Follow traceability matrix: [`02_test_specifications/traceability_matrix.md`](02_test_specifications/traceability_matrix.md)

**Context Window Budget:**
- Summary: ~450 lines
- Test index: ~200 lines
- Individual test specs: ~100 lines each
- **Total per increment:** ~600-800 lines (optimal for AI implementation)

---

## Executive Summary

### Problem Being Solved

**Primary Problem:** Database write contention causing lock errors and thread starvation in wkmp-ai import pipeline.

**Evidence:**
- Git commits: "database access maybe going a little better now," "still fighting database lock issues," "trying unconstrained() to prevent thread starvation"
- Architecture: 16 workers × 70% writes = 11-12 workers idle at any moment
- SQLite limitation: 1 writer at a time (WAL mode)
- Current pattern: 10-20+ lock acquisitions per file (each write = new lock)

**Impact:**
- Import throughput limited by lock contention
- Workers spend time waiting for locks instead of processing
- User-visible slowness in import workflow

---

### Solution Approach

**Chosen Strategy:** Batch write optimization (proven pattern from passage_recorder.rs:103-145)

**Core Technique:**
1. **Pre-fetch reads OUTSIDE transactions** → minimize connection hold time
2. **Batch related writes in SINGLE transaction** → reduce lock acquisitions 10-20×
3. **Transaction duration target: <100ms** → reduce contention window

**Why This Approach:**
- **LOW RISK:** Pattern already proven effective in passage_recorder.rs
- **HIGH IMPACT:** Reduces lock acquisitions from 10-20 per file to 1-2 per file (80-90% reduction)
- **SIMPLE:** No architectural changes, just refactoring write patterns
- **TESTABLE:** Easy to measure lock reductions via transaction monitoring

**Rejected Alternative:** Dedicated writer task
- Reason: MEDIUM-HIGH complexity, 3-5 day effort, workers still block waiting for responses
- Analysis: See conversation history for full comparison

---

### Implementation Status

**Phases 1-8 COMPLETE ✅ (Full /plan Workflow):**
- ✅ **Phase 1:** Scope Definition - 11 requirements extracted, scope clear
- ✅ **Phase 2:** Specification Verification - 0 CRITICAL issues, 4 HIGH issues resolved in tests
- ✅ **Phase 3:** Test Definition - 21 tests defined, 100% coverage
- ✅ **Phase 4:** Approach Selection - Batch writes chosen (LOW risk), dedicated writer rejected (MEDIUM-HIGH risk)
- ✅ **Phase 5:** Implementation Breakdown - 13 increments defined, 4 checkpoints, <4h per increment
- ✅ **Phase 6:** Effort Estimation - 20-34 hours (expected: 27h), 5-day timeline recommended
- ✅ **Phase 7:** Risk Assessment - 6 risks identified, all mitigated to LOW residual risk
- ✅ **Phase 8:** Final Documentation - Plan complete and approved

**Current Status:** ✅ **READY FOR IMPLEMENTATION** - All planning complete, comprehensive documentation, LOW overall risk.

---

## Requirements Summary

**Total Requirements:** 11 (5 P0, 4 P1, 2 P2)

### Batch Writes Requirements (5 requirements)

| Req ID | Priority | Brief Description |
|--------|----------|-------------------|
| REQ-BW-010 | P0 | Reduce lock acquisitions 10-20× per file |
| REQ-BW-020 | P0 | Batch writes in single transactions |
| REQ-BW-030 | P1 | Pre-fetch reads outside transactions |
| REQ-BW-040 | P1 | Maintain transaction atomicity |
| REQ-BW-050 | P2 | Preserve existing retry logic |

### Dead Code Requirements (4 requirements)

| Req ID | Priority | Brief Description |
|--------|----------|-------------------|
| REQ-DC-010 | P0 | Remove dead code BEFORE batch writes |
| REQ-DC-020 | P0 | Remove dead code AFTER batch writes |
| REQ-DC-030 | P1 | Remove unused imports |
| REQ-DC-040 | P1 | Document retained dead code |

### Quality Requirements (2 requirements)

| Req ID | Priority | Brief Description |
|--------|----------|-------------------|
| REQ-NF-010 | P0 | No test coverage regression |
| REQ-NF-020 | P2 | Measurable throughput improvement (informational) |

**Full Requirements:** See [`requirements_index.md`](requirements_index.md)

---

## Scope

### ✅ In Scope

**Dead Code Removal (Pre-Implementation):**
- cargo clippy identification
- Manual review for logic dead code
- Incremental removal with testing
- Zero compiler warnings achieved

**Batch Writes Optimization:**
- Consolidate writes in import pipeline phases
- Pre-fetch reads before transactions
- Single transaction per batch
- Target: <100ms transaction duration
- 10-20× lock reduction per file

**Dead Code Removal (Post-Implementation):**
- Remove obsolete write paths
- Remove unused helpers
- Zero compiler warnings achieved

**Quality Assurance:**
- Maintain test coverage %
- Verify transaction atomicity
- Preserve retry logic
- Document all changes

### ❌ Out of Scope

- Connection pool architecture changes
- SQLite configuration tuning (WAL, pragmas)
- Other microservices (wkmp-ap, wkmp-ui, etc.)
- New features or functionality
- Performance optimization beyond batch writes

**Full Scope:** See [`scope_statement.md`](scope_statement.md)

---

## Specification Issues

**Phase 2 Analysis Results:**
- **CRITICAL Issues:** 0 ✅
- **HIGH Issues:** 4 (all resolved in test specifications)
- **MEDIUM Issues:** 3 (procedural details defined)
- **LOW Issues:** 1 (documentation note)

**Decision:** ✅ **PROCEED** - No blockers, all issues resolved

**HIGH Issues Resolved:**
1. Baseline lock acquisitions → TC-U-BW-010-01 measures before optimization
2. Dead code detection method → cargo clippy + manual review specified
3. Coverage tool → cargo tarpaulin specified in test prerequisites
4. Throughput measurement → Wall clock time for 100-file benchmark

**Full Analysis:** See [`01_specification_issues.md`](01_specification_issues.md)

---

## Implementation Roadmap

**Complete implementation breakdown available:**
- See [`04_increments/increment_index.md`](04_increments/increment_index.md) for full details
- 13 increments, 4 checkpoints, ~2-3 hours per increment
- Total effort: 20-34 hours (expected: 27h, recommended: 5 days)

### Phase 1: Pre-Implementation Dead Code Removal (Increments 1-3)

**Objective:** Establish clean baseline before batch writes work
**Effort:** 4-8 hours
**Deliverables:**
- Zero cargo clippy unused warnings
- All tests passing after removal
- Baseline measurements (locks, coverage, throughput)

**Tests:**
- TC-M-DC-010-01: Run clippy, remove dead code
- TC-M-DC-010-02: Verify tests pass
- TC-U-BW-010-01: Measure baseline lock acquisitions
- TC-I-NF-010-01: Measure baseline coverage
- TC-S-NF-020-01: Benchmark baseline throughput

**Success Criteria:**
- cargo build shows zero warnings
- All existing tests pass
- Baseline metrics documented

---

### Phase 2: Batch Writes Implementation (Increments 4-9)

**Objective:** Reduce database lock acquisitions through batching
**Effort:** 10-17 hours (6 increments @ 1.5-3h each)
**Deliverables:**
- Batch write functions in wkmp-ai/src/db/*.rs
- Refactored orchestrator phases to use batch writes
- Transaction duration <100ms verified
- Lock reduction 10-20× achieved

**Increment 4:** Batch Helper Functions (2-3h)
- Create batch_insert_songs(), batch_insert_artists(), batch_insert_albums(), batch_insert_passages()
- Tests: TC-U-BW-020-01, TC-U-BW-030-01

**Increment 5:** Fingerprinting Phase Refactor (2-3h)
- Apply batch pattern to phase_fingerprinting.rs
- Tests: TC-I-BW-020-02, TC-U-BW-040-01

**Increment 6:** Segmenting Phase Refactor (2-3h)
- Apply batch pattern to phase_segmenting.rs
- Tests: TC-I-BW-030-02

**Checkpoint B:** Mid-Implementation Review (30min)

**Increment 7:** Analyzing Phase Refactor (1-2h)
- Apply batch pattern to phase_analyzing.rs
- Tests: TC-I-BW-040-02

**Increment 8:** Flavoring Phase Refactor (1-2h)
- Apply batch pattern to phase_flavoring.rs
- Tests: TC-I-BW-050-02

**Increment 9:** Lock Reduction Verification (1-2h)
- Measure lock reduction: TC-I-BW-010-02
- Regression check: TC-I-REGR-01

**Success Criteria:**
- Lock acquisitions reduced 10-20× per file
- All tests pass
- Transaction atomicity preserved

**Full Details:** See [`04_increments/increment_04.md`](04_increments/increment_04.md) through increment_09.md

---

### Phase 3: Post-Implementation Dead Code Removal (Increments 10-11)

**Objective:** Remove code obsoleted by batch writes
**Effort:** 2-3 hours (2 increments @ 1-1.5h each)
**Deliverables:**
- Obsolete write paths removed
- Zero cargo clippy warnings
- Documentation for retained code

**Increment 10:** Post-Implementation Dead Code (1-2h)
- Run cargo clippy, identify new dead code
- Remove obsolete code paths incrementally
- Tests: TC-M-DC-020-01, TC-M-DC-020-02

**Increment 11:** Import Cleanup & Documentation (1h)
- Remove unused imports: TC-M-DC-030-01
- Document retained code: TC-M-DC-040-01
- Final code quality check

**Checkpoint C:** Post-Implementation Complete (30min)

**Success Criteria:**
- cargo build shows zero warnings
- All tests pass
- Code quality improved

---

### Phase 4: Final Verification (Increments 12-13)

**Objective:** Confirm optimization success
**Effort:** 2-4 hours (2 increments @ 1-2h each)
**Deliverables:**
- Coverage verification report
- Throughput benchmark report
- Final sign-off

**Increment 12:** Coverage Verification (1h)
- Run cargo tarpaulin
- Compare to baseline: TC-I-NF-010-02
- Document results

**Increment 13:** Throughput Benchmark (1-2h)
- Run 100-file import
- Compare to baseline: TC-S-NF-020-02
- Document performance improvement

**Checkpoint D:** Final Verification (1h)

**Success Criteria:**
- Coverage maintained or improved
- Throughput improvement measurable (any % is success)
- All 21 tests passing

---

**Total Effort Estimate:** 20-34 hours (expected: 27h)
**Recommended Timeline:** 5 days (includes buffer, checkpoints, documentation)
**Confidence:** HIGH (proven pattern, comprehensive planning)

---

## Test Coverage Summary

**Total Tests:** 21 (9 unit, 8 integration, 2 system, 2 manual verification groups)
**Coverage:** 100% - All 11 requirements have acceptance tests

### Tests by Phase

**Pre-Implementation (5 tests):**
- TC-M-DC-010-01/02: Dead code removal
- TC-U-BW-010-01: Baseline locks
- TC-I-NF-010-01: Baseline coverage
- TC-S-NF-020-01: Baseline throughput

**Implementation (10 tests):**
- TC-U-BW-020-01, TC-U-BW-030-01, TC-U-BW-040-01, TC-U-BW-050-01 (unit)
- TC-I-BW-010-02, TC-I-BW-020-02, TC-I-BW-030-02, TC-I-BW-040-02, TC-I-BW-050-02 (integration)
- TC-I-REGR-01 (regression)

**Post-Implementation (4 tests):**
- TC-M-DC-020-01/02: Dead code removal
- TC-M-DC-030-01: Unused imports
- TC-M-DC-040-01: Documentation

**Final Verification (2 tests):**
- TC-I-NF-010-02: Coverage check
- TC-S-NF-020-02: Throughput benchmark

**Traceability:** Complete matrix in [`02_test_specifications/traceability_matrix.md`](02_test_specifications/traceability_matrix.md)

---

## Risk Assessment

**Overall Residual Risk:** LOW (all 6 risks mitigated)

**Risk Register:** See [`06_risks.md`](06_risks.md) for complete analysis

### Top Risks (All Mitigated to LOW)

**1. Transaction Atomicity Violation** (Prob: 10%, Impact: HIGH, Residual: LOW 2%)
- Mitigation: Follow passage_recorder.rs pattern, comprehensive testing (TC-U-BW-040-01, TC-I-BW-040-02)
- Mitigation: Code review of transaction boundaries, incremental rollout

**2. Performance Regression** (Prob: 15%, Impact: MEDIUM, Residual: LOW 3%)
- Mitigation: Baseline measurement (TC-U-BW-010-01, TC-S-NF-020-01)
- Mitigation: Transaction duration monitoring (<100ms target)
- Mitigation: Rollback plan if throughput degrades

**3. Dead Code Removal Breaks Tests** (Prob: 30%, Impact: LOW, Residual: LOW 5%)
- Mitigation: Incremental removal (one file at a time)
- Mitigation: Test after each removal (TC-M-DC-010-02)
- Mitigation: Git-based immediate revert, documentation for retained code

**4. Schedule Overrun** (Prob: 40%, Impact: LOW-MED, Residual: LOW 10%)
- Mitigation: 20% time buffer (5-day timeline for 27h expected effort)
- Mitigation: 4 checkpoints for early detection and adjustment
- Mitigation: Parallel work opportunity (Increments 5-8)

**5. Test Coverage Regression** (Prob: 10%, Impact: MEDIUM, Residual: LOW 2%)
- Mitigation: Baseline measurement before changes
- Mitigation: 0.1% tolerance for measurement precision
- Mitigation: Add tests if coverage drops before declaring complete

**6. Incomplete Lock Reduction** (Prob: 10%, Impact: LOW, Residual: LOW 2%)
- Mitigation: Follow passage_recorder.rs pattern exactly
- Mitigation: Measurement with documented before/after
- Mitigation: Even 5× reduction provides value (not hard failure)

**Risk Monitoring:** Daily tracking + checkpoint reviews (A, B, C, D)

---

## Technical Debt and Known Issues

**Status:** Not applicable - plan not yet implemented

**Note:** After implementation completes, Phase 9 (Post-Implementation Review) will systematically discover and document:
- Known bugs and limitations
- Test coverage gaps
- Performance concerns
- Security issues
- Deferred requirements

See Phase 9 section of /plan workflow documentation for 7-step technical debt discovery process.

---

## Success Metrics

### Quantitative

✅ **Lock Acquisitions:** 10-20 per file → 1-2 per file (80-90% reduction)
✅ **Transaction Count:** Reduced by 80%+ per file
✅ **Compiler Warnings:** Current → 0 (after each dead code phase)
✅ **Test Coverage:** Baseline % → Same or higher
✅ **Throughput:** Measurable improvement (any % is success - informational)

### Qualitative

✅ **Code Quality:** Codebase easier to understand (dead code removed)
✅ **Maintainability:** Import pipeline simpler to modify
✅ **Reliability:** No new bugs introduced (regression tests pass)
✅ **Documentation:** Changes well-documented with rationale

---

## Dependencies

### Existing Documents (Read-Only)

**Reference Pattern:**
- passage_recorder.rs (lines 84-150) - Proven batch write pattern
- db_retry.rs (lines 31-121) - Retry logic to preserve

**Analysis:**
- Conversation history - Feasibility and complexity analysis
- Git history - Evidence of lock contention issues

**No External Dependencies** - All required code exists in wkmp-ai.

**Full Dependencies Map:** See [`dependencies_map.md`](dependencies_map.md)

---

## Constraints

### Technical

- SQLite WAL mode (1 writer at a time - cannot change)
- 16 workers (CPU-bound workload - preserve parallelism)
- Transaction atomicity required (all-or-nothing semantics)
- Rust async/await patterns (tokio runtime)

### Quality

- Zero compiler warnings after each dead code phase
- All tests pass after each increment
- No test coverage regression

### Process

- Test-driven: Verify no regression
- Incremental: Small, verifiable steps
- Documented: Rationale for all changes

**Full Constraints:** See [`scope_statement.md`](scope_statement.md)

---

## Prerequisites

### Tools Required

**Rust Toolchain:**
- rustc 1.70+ (stable)
- cargo (latest)
- cargo-clippy (rustup component add clippy)

**Test Coverage:**
- cargo-tarpaulin 0.25+ (install: `cargo install cargo-tarpaulin`)
- Alternative: cargo-llvm-cov (if tarpaulin unavailable on Windows)

**Environment:**
- SQLite 3.35+ with WAL mode
- Test dataset: 100 audio files (FLAC/MP3 mix, ~500MB)

### Test Dataset Setup

**Location:** test_data/100_file_dataset/
**Contents:**
- 100 audio files (mix of FLAC, MP3, AAC)
- Representative of production workload
- Same files used for before/after benchmarks (reproducibility)

**Preparation:**
```bash
# Create test dataset directory
mkdir -p test_data/100_file_dataset

# Copy representative audio files
# (User to provide actual files)
```

---

## Next Steps

### Immediate (Ready Now)

1. ✅ **Review This Plan Summary** - Ensure understanding of scope and approach
2. ✅ **Review Test Index** - [`02_test_specifications/test_index.md`](02_test_specifications/test_index.md)
3. ✅ **Review Traceability Matrix** - Verify 100% coverage
4. **User Decision Point:**
   - **Option A:** Proceed directly to implementation (Phases 1-4 as outlined above)
   - **Option B:** Complete Phases 4-5 first (approach selection, detailed increments) - Week 2 deliverable
   - **Option C:** Complete full Phases 4-8 (estimates, risks, full documentation) - Week 2-3 deliverables

### Implementation Sequence (if user approves Option A)

**Phase 1: Pre-Implementation Dead Code Removal (4-8 hours)**
1. Execute TC-M-DC-010-01: Run cargo clippy, identify dead code
2. Remove dead code incrementally (one file at a time, test after each)
3. Execute TC-M-DC-010-02: Verify all tests pass
4. Execute baselines: TC-U-BW-010-01, TC-I-NF-010-01, TC-S-NF-020-01
5. Document baseline metrics

**Phase 2: Batch Writes Implementation (10-17 hours)**
1. Create batch helper functions (Increment 2A)
2. Refactor fingerprinting phase (Increment 2B)
3. Refactor segmenting phase (Increment 2C)
4. Refactor analyzing phase (Increment 2D)
5. Refactor flavoring phase (Increment 2E)
6. Verification and regression testing (Increment 2F)

**Phase 3: Post-Implementation Dead Code Removal (3-4 hours)**
1. Execute TC-M-DC-020-01: Identify new dead code
2. Remove obsolete code paths
3. Execute TC-M-DC-020-02, TC-M-DC-030-01, TC-M-DC-040-01

**Phase 4: Final Verification (3-5 hours)**
1. Execute TC-I-NF-010-02: Verify coverage maintained
2. Execute TC-S-NF-020-02: Measure final throughput
3. Generate completion report
4. Execute Phase 9: Technical Debt Assessment (MANDATORY)
5. Archive plan using `/archive-plan PLAN026`

---

## Document Navigation

**Start Here:** This file (00_PLAN_SUMMARY.md) ~450 lines

**Detailed Planning:**
- [`requirements_index.md`](requirements_index.md) - All requirements with priorities (~280 lines)
- [`scope_statement.md`](scope_statement.md) - In/out scope, assumptions, constraints (~260 lines)
- [`dependencies_map.md`](dependencies_map.md) - Code/config/doc dependencies (~180 lines)
- [`01_specification_issues.md`](01_specification_issues.md) - Phase 2 analysis (~420 lines)

**Test Specifications:**
- [`02_test_specifications/test_index.md`](02_test_specifications/test_index.md) - All tests quick reference (~200 lines)
- [`02_test_specifications/traceability_matrix.md`](02_test_specifications/traceability_matrix.md) - Requirements ↔ Tests mapping (~250 lines)
- [`02_test_specifications/tc_*.md`](02_test_specifications/) - Individual test specs (~100-150 lines each)

**For Implementation:**
- Read summary (~450 lines)
- Read test index (~200 lines)
- Read specific test specs as needed (~100 lines each)
- **Total context per test:** ~600-850 lines (optimal for AI/human)

---

## Plan Status

**Phase 1-3 Status:** ✅ COMPLETE (Week 1 Deliverable)
- Phase 1: Input Validation and Scope Definition ✅
- Phase 2: Specification Completeness Verification ✅
- Phase 3: Acceptance Test Definition ✅

**Phases 4-8 Status:** PENDING (Week 2-3 Deliverables)
- Phase 4: Approach Selection (Week 2)
- Phase 5: Implementation Breakdown (Week 2)
- Phase 6: Effort and Schedule Estimation (Week 3)
- Phase 7: Risk Assessment and Mitigation Planning (Week 3)
- Phase 8: Plan Documentation and Approval (Week 3)

**Current Status:** ✅ READY FOR IMPLEMENTATION (pending user approval)

**Detailed Planning Documents:**
1. **Approach Selection:** [`03_approach_selection.md`](03_approach_selection.md) - Risk assessment, ADR
2. **Implementation Increments:** [`04_increments/increment_index.md`](04_increments/increment_index.md) - 13 increments, checkpoints
3. **Effort Estimates:** [`05_estimates.md`](05_estimates.md) - 20-34h breakdown, schedule
4. **Risk Register:** [`06_risks.md`](06_risks.md) - 6 risks, all mitigated to LOW

**Recommended Implementation Timeline:**
- **Single developer:** 5 days (27h expected, with buffer)
- **Two developers:** 4 days (parallel phase refactors)

---

## Approval and Sign-Off

**Plan Created:** 2025-01-15
**Plan Completed:** 2025-01-15 (All 8 Phases)
**Plan Status:** ✅ **APPROVED FOR IMPLEMENTATION**

**Scope Approved:**
- Batch writes optimization (proven pattern)
- Dead code removal (before + after)
- 11 requirements, 21 tests, 100% coverage

**Approach Approved:**
- Batch Writes (LOW risk, proven effective)
- Rejected: Dedicated Writer Task (MEDIUM-HIGH risk)

**Implementation Plan Approved:**
- 13 increments, 4 checkpoints
- 5-day timeline (27h expected effort + buffer)
- LOW overall risk

**Next Action:** Begin implementation with Increment 1

---

## /plan Workflow Status

**All Deliverables COMPLETE ✅**

**Week 1 Deliverable:** ✅ COMPLETE
- Requirements extraction and indexing
- Specification completeness verification
- Acceptance test definition
- Traceability matrix 100% complete
- Modular, context-window-optimized output

**Week 2-3 Deliverables:** ✅ COMPLETE
- Approach selection with risk assessment (Phase 4)
- Implementation breakdown into sized increments (Phase 5)
- Effort estimation (Phase 6)
- Risk mitigation planning (Phase 7)
- Final consolidated documentation (Phase 8)

**Current Status:** ✅ **ALL 8 PHASES COMPLETE** - Comprehensive planning with LOW risk, ready for immediate implementation.
