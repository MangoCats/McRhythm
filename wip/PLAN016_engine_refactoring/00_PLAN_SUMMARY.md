# PLAN016: Engine Refactoring - PLAN SUMMARY

**Status:** Ready for Implementation (Phases 1-8 Complete)
**Created:** 2025-11-01
**Specification Source:** wip/SPEC024-wkmp_ap_technical_debt_remediation.md (REQ-DEBT-QUALITY-002)
**Plan Location:** `wip/PLAN016_engine_refactoring/`

---

## READ THIS FIRST

This document provides a complete summary of PLAN016: Engine Refactoring. The plan decomposes the monolithic `engine.rs` file (4,251 lines) into a modular directory structure with 3 functional modules, each <1500 lines, while preserving the exact public API.

**For Implementation:**
- Read this summary (~450 lines)
- Review detailed requirements: [requirements_index.md](requirements_index.md)
- Review test specifications: [02_test_specifications/test_index.md](02_test_specifications/test_index.md)
- Follow traceability matrix: [02_test_specifications/traceability_matrix.md](02_test_specifications/traceability_matrix.md)
- See implementation increments: [04_increments/increments_summary.md](04_increments/increments_summary.md)

**Context Window Budget:**
- This summary: ~450 lines
- Increments summary: ~250 lines
- Test index: ~100 lines
- **Total for implementation:** ~800 lines (optimized for AI consumption)

---

## Executive Summary

### Problem Being Solved

**Current State:**
- `wkmp-ap/src/playback/engine.rs` contains 4,251 lines of code
- Monolithic file handles: state management, queue operations, diagnostics, lifecycle, buffer coordination, event emissions
- Difficult to navigate, understand, and maintain
- High cognitive load for developers

**Problems:**
1. **Maintainability:** Changes require understanding entire 4,251-line file
2. **Cognitive Overload:** Too much functionality in one file
3. **Code Review:** Difficult to review changes in context
4. **Technical Debt:** Violates single-responsibility principle

### Solution Approach

**Refactoring Strategy:** Functional Decomposition (Approach A)
- Convert `engine.rs` (single file) → `engine/` (directory with 4 modules)
- **Module Structure:**
  - `mod.rs` - Public API re-exports (interface) (~100 lines)
  - `core.rs` - State management and lifecycle (~1,380 lines)
  - `queue.rs` - Queue operations (~1,250 lines)
  - `diagnostics.rs` - Status queries and telemetry (~1,035 lines)
- **Constraints:**
  - Each module MUST be <1500 lines
  - Public API MUST remain unchanged (internal refactoring only)
  - All existing tests MUST pass without modification

**Approach:** Code migration with zero functional changes, verified by comprehensive test suite.

### Implementation Status

**All 8 Phases Complete:**
- ✅ Phase 1: Scope Definition - 3 requirements extracted, dependencies mapped
- ✅ Phase 2: Specification Verification - 8 issues identified (2 critical, resolved)
- ✅ Phase 3: Test Definition - 11 tests defined, 100% coverage
- ✅ Phase 4: Approach Selection - Functional decomposition selected (Low risk)
- ✅ Phase 5: Implementation Breakdown - 6 increments defined
- ✅ Phase 6: Effort Estimation - 8-12 hours (conservative: 15 hours)
- ✅ Phase 7: Risk Assessment - 12 risks identified, all mitigated (Overall: LOW)
- ✅ Phase 8: Final Documentation - Complete plan ready

**Current Status:** **READY FOR IMPLEMENTATION**

---

## Requirements Summary

**Total Requirements:** 3 (all P1 - High Priority)

| Req ID | Brief Description | Quantified Target | Status |
|--------|-------------------|-------------------|--------|
| REQ-DEBT-QUALITY-002-010 | Split into 3 modules | 3 functional modules + 1 interface | Defined |
| REQ-DEBT-QUALITY-002-020 | Line count limit | Each file <1500 lines | Defined |
| REQ-DEBT-QUALITY-002-030 | API stability | Zero public API changes | Defined |

**Full Requirements:** See [requirements_index.md](requirements_index.md)

---

## Scope

### ✅ In Scope

**What WILL Be Implemented:**

1. **Directory Structure Conversion**
   - Delete: `wkmp-ap/src/playback/engine.rs`
   - Create: `wkmp-ap/src/playback/engine/` directory
   - Files: `mod.rs`, `core.rs`, `queue.rs`, `diagnostics.rs`

2. **Code Migration**
   - Move ALL code from engine.rs to appropriate modules
   - Organize by functional responsibility (lifecycle/queue/diagnostics)
   - Preserve ALL functionality (no additions, no removals)

3. **Public API Preservation**
   - Re-export all `pub` items in `mod.rs`
   - Zero changes to function signatures, struct fields, or types
   - External callers (handlers, tests) compile without modification

4. **Verification**
   - 11 acceptance tests defined (unit, integration, system, manual)
   - 100% test coverage via traceability matrix
   - All baseline tests MUST pass

### ❌ Out of Scope

**What Will NOT Be Implemented:**

1. **Other Technical Debt Items from SPEC024**
   - DEBT-SEC-001 through DEBT-QUALITY-005 (separate plans)

2. **Functional Changes**
   - No new features, bug fixes, performance optimizations, or API enhancements

3. **Other Refactorings**
   - mixer.rs (already refactored in PLAN014)
   - handlers.rs (not required)

**Full Scope:** See [scope_statement.md](scope_statement.md)

---

## Specification Issues

**Phase 2 Analysis Results:**
- **CRITICAL Issues:** 2 (both resolved in Phases 3-4)
  - Module responsibility boundaries → Resolved by Phase 4 code analysis
  - Public API definition → Resolved by Phase 3 test extraction
- **HIGH Issues:** 3 (all resolved)
  - "3 modules" vs. 4 files → Clarified (3 functional + 1 interface)
  - Feasibility if modules exceed limit → Mitigation (allow further decomposition)
  - API test inadequate → Resolved (11 comprehensive tests defined)
- **MEDIUM Issues:** 2 (resolved)
- **LOW Issues:** 1 (resolved)

**Decision:** **PROCEED** - All critical issues resolved

**Full Analysis:** See [01_specification_issues.md](01_specification_issues.md)

---

## Approach Selection

**Approaches Evaluated:** 2

**Approach A: Functional Decomposition (SELECTED)**
- Split by functional responsibility (lifecycle, queue, diagnostics)
- Natural module boundaries identified through code analysis
- **Risk:** Low (after mitigation)
- **Effort:** 8-12 hours
- **Quality:** High maintainability, architectural alignment

**Approach B: Line-Count-First Split (REJECTED)**
- Split by equal-sized chunks (~1,400 lines each)
- **Risk:** Medium-High (arbitrary boundaries, poor maintainability)
- **Effort:** 6-8 hours (faster but wrong)
- **Quality:** Low maintainability, poor alignment

**Decision Rationale (Risk-First Framework):**
- Approach A: Low risk, High quality, 8-12 hours
- Approach B: Medium-High risk, Low quality, 6-8 hours
- **Per CLAUDE.md:** Choose lowest-risk approach (effort secondary)
- **Selected:** Approach A (Functional Decomposition)

**Full Analysis:** See [03_approach_selection.md](03_approach_selection.md)

---

## Implementation Roadmap

**Total Increments:** 6
**Estimated Effort:** 8-12 hours (conservative: 15 hours)
**Recommended Timeline:** 3 days (distributed, 3-4 hours per day)

| Inc | Name | Objective | Effort | Tests | Key Deliverable |
|-----|------|-----------|--------|-------|-----------------|
| 1 | Baseline & Analysis | Establish baseline, map code | 1-2h | 3 baseline | Code mapping + metrics |
| 2 | Module Structure | Create directory skeleton | 0.5h | 2 structure | Compiling skeleton |
| 3 | Extract Queue | Move queue operations | 2-3h | 3 tests | queue.rs <1500 lines |
| 4 | Extract Diagnostics | Move diagnostics | 2-3h | 3 tests | diagnostics.rs <1500 lines |
| 5 | Finalize Core | Clean up core.rs, mod.rs | 1h | 3 tests | core.rs <1500 lines |
| 6 | Verification | Delete engine.rs, verify | 1-2h | All 11 | 100% verification |

**Full Increments:** See [04_increments/increments_summary.md](04_increments/increments_summary.md)

---

## Test Coverage Summary

**Total Tests:** 11 (3 unit, 3 integration, 5 system/manual)
**Coverage:** 100% - All 3 requirements have acceptance tests

**Test Breakdown:**
- **Module Structure (REQ-010):** 4 tests
  - TC-U-010-01: Directory structure validation
  - TC-I-010-01: Module compilation
  - TC-S-010-01: File count verification
  - TC-S-010-02: Code organization review (manual)

- **Line Counts (REQ-020):** 2 tests
  - TC-U-020-01: Line count measurement (<1500 each)
  - TC-S-020-01: Total line preservation (≈4,251 ±5%)

- **API Stability (REQ-030):** 5 tests
  - TC-U-030-01: Public API surface check
  - TC-I-030-01: Handler compilation unchanged
  - TC-I-030-02: Test suite pass rate (100% baseline)
  - TC-S-030-01: API compatibility verification
  - TC-S-030-02: External caller review (manual)

**Traceability:** Complete matrix in [02_test_specifications/traceability_matrix.md](02_test_specifications/traceability_matrix.md)

---

## Effort Estimation

**Total Estimated Effort:** 8-12 hours
**Confidence Level:** 70-75% (Medium-High)
**Conservative Estimate:** 15 hours (with 25% contingency)

**Breakdown:**
- Implementation: 6.5-10 hours (75%)
- Testing & Verification: 1.5-2 hours (15%)
- Buffer/Contingency: 1-2 hours (10%)

**Recommended Timeline:**
- **Option B (Recommended):** 3 days distributed (3-4 hours per day)
  - Day 1: Increments 1-2
  - Day 2: Increments 3-4
  - Day 3: Increments 5-6

**Full Estimates:** See [05_estimates.md](05_estimates.md)

---

## Risk Assessment

**Total Risks Identified:** 12
- **HIGH:** 0
- **MEDIUM:** 3 (all mitigated to Low-Medium residual)
- **LOW:** 6
- **VERY LOW:** 3

**Overall Risk Rating:** **LOW** (acceptable for implementation)

**Top 3 Risks:**
1. **RISK-004:** Import/Dependency Errors (Medium → Low-Med after mitigation)
   - Mitigation: Incremental compilation, clear import hierarchy
2. **RISK-005:** Async Handler Lifetime Issues (Medium → Low-Med after mitigation)
   - Mitigation: Keep spawning pattern, fallback option available
3. **RISK-007:** Unexpected Test Failures (Medium → Low after mitigation)
   - Mitigation: Baseline comparison, comprehensive tests

**Risk Acceptance:** All medium risks have effective mitigations. Residual risk acceptable.

**Recommendation:** **PROCEED WITH IMPLEMENTATION**

**Full Risk Assessment:** See [06_risks.md](06_risks.md)

---

## Module Responsibility Matrix

**From Phase 4 Code Analysis:**

**CORE.RS (~1,380 lines)** - State Management & Lifecycle
- PlaybackEngine struct (107 lines)
- Lifecycle: new(), start(), stop(), play(), pause(), seek()
- Orchestration: playback_loop(), process_queue() (580 lines)
- Chain allocation: assign_chain(), release_chain()
- Utilities: clone_handles()

**QUEUE.RS (~1,250 lines)** - Queue Operations
- Queue operations: skip_next(), clear_queue(), enqueue_file(), remove_queue_entry()
- Queue queries: get_queue_entries(), queue_len(), reorder_queue_entry()
- Helpers: emit_queue_change_events(), complete_passage_removal()

**DIAGNOSTICS.RS (~1,035 lines)** - Monitoring & Status
- Status accessors: get_buffer_chains(), get_metrics(), verify_queue_sync()
- Monitoring config: set_buffer_monitor_rate(), trigger_buffer_monitor_update()
- Event handlers: position_event_handler(), buffer_event_handler() (700 lines)
- Emitters: buffer_chain_status_emitter(), playback_position_emitter()

**Full Mapping:** See [03_approach_selection.md](03_approach_selection.md) Module Responsibility Matrix

---

## Technical Debt and Known Issues

**Status:** Not applicable - plan not yet implemented

**Note:** After implementation completes, Phase 9 (Post-Implementation Review) will systematically discover and document:
- Known bugs and limitations
- Test coverage gaps
- Performance concerns
- Security issues
- Deferred requirements

See Phase 9 section of `/plan` workflow for 7-step technical debt discovery process.

---

## Success Metrics

**Quantitative:**
- ✅ All 4 module files < 1500 lines (TC-U-020-01)
- ✅ Total lines ≈ original ±5% (TC-S-020-01)
- ✅ 100% baseline tests pass (TC-I-030-02)
- ✅ Zero API changes (TC-U-030-01, TC-S-030-01)

**Qualitative:**
- ✅ Logical code organization (TC-S-010-02)
- ✅ Improved maintainability
- ✅ Easier navigation and comprehension
- ✅ No external caller changes (TC-S-030-02)

---

## Dependencies

**Existing Documents (Read-Only):**
- wkmp-ap/src/playback/engine.rs (4,251 lines) - Current implementation
- wkmp-ap/src/api/handlers.rs (1,305 lines) - API callers
- wkmp-ap/tests/**/*.rs - Test suite

**Integration Points:**
- handlers.rs → PlaybackEngine (MUST compile unchanged)
- Tests → PlaybackEngine (MUST pass unchanged)
- Internal modules (mixer, buffer_manager, queue_manager) - Unchanged

**No External Dependencies** (no new crates)

**Full Dependencies:** See [dependencies_map.md](dependencies_map.md)

---

## Constraints

**Technical:**
- Rust module system (pub, pub(crate), pub(super) visibility)
- API stability (REQ-030) - no public interface changes
- Line count limit (REQ-020) - each file <1500 lines
- No code duplication (DRY principle)

**Process:**
- Test-first refactoring (run tests before/after)
- Incremental commits (enable easy rollback)
- WKMP coding conventions (IMPL002)

**Timeline:**
- Estimated: 8-12 hours
- Conservative: 15 hours (with contingency)
- Recommended: 3 days distributed

**Full Constraints:** See [scope_statement.md](scope_statement.md)

---

## Next Steps

### Immediate (Ready Now)

**Begin Implementation:**
1. Start with Increment 1: Baseline & Analysis
2. Follow [04_increments/increments_summary.md](04_increments/increments_summary.md)
3. Run tests at each checkpoint
4. Use traceability matrix to verify coverage

**Quick Start:**
```bash
# Increment 1: Establish Baseline
cd wkmp-ap
cargo test -p wkmp-ap 2>&1 | tee test_baseline.log
grep "pub fn" src/playback/engine.rs > public_api_baseline.txt
wc -l src/playback/engine.rs > original_line_count.txt
```

### During Implementation

**At Each Increment:**
- Follow tasks in increments_summary.md
- Run specified tests
- Verify success criteria before proceeding

**At Checkpoints (After Increments 3, 4, 5):**
- Review progress
- Verify tests pass
- Decide: Proceed / Pause / Rollback

### After Implementation

1. Execute Phase 9: Post-Implementation Review (MANDATORY)
2. Generate technical debt report
3. Run all 11 tests
4. Verify traceability matrix 100% complete
5. Create final implementation report
6. Archive plan using `/archive-plan PLAN016`

---

## Document Navigation

**Start Here:** This file (00_PLAN_SUMMARY.md)

**Planning Documents:**
- [requirements_index.md](requirements_index.md) - All 3 requirements (~150 lines)
- [scope_statement.md](scope_statement.md) - In/out scope, assumptions, constraints
- [dependencies_map.md](dependencies_map.md) - Dependencies catalog
- [01_specification_issues.md](01_specification_issues.md) - Phase 2 analysis
- [03_approach_selection.md](03_approach_selection.md) - Approach A selected (ADR)

**Implementation Documents:**
- [04_increments/increments_summary.md](04_increments/increments_summary.md) - 6 increments (~250 lines)
- [05_estimates.md](05_estimates.md) - Effort estimates (8-12 hours)
- [06_risks.md](06_risks.md) - 12 risks assessed (Overall: LOW)

**Test Specifications:**
- [02_test_specifications/test_index.md](02_test_specifications/test_index.md) - 11 tests (~200 lines)
- [02_test_specifications/traceability_matrix.md](02_test_specifications/traceability_matrix.md) - Requirements ↔ Tests
- [02_test_specifications/tc_*.md](02_test_specifications/) - Individual test specs

**For Implementation (Recommended Reading Order):**
1. This summary (~450 lines)
2. Increments summary (~250 lines)
3. Test index (~200 lines)
4. **Total context:** ~900 lines (optimized for AI consumption)

**NOT Required for Implementation:**
- Full scope/dependencies/issues documents (reference only)
- Detailed risk analysis (summary in this document sufficient)

---

## Plan Status

**All Phases Complete:**
- ✅ Phase 1: Scope Definition
- ✅ Phase 2: Specification Verification
- ✅ Phase 3: Test Definition
- ✅ Phase 4: Approach Selection
- ✅ Phase 5: Implementation Breakdown
- ✅ Phase 6: Effort Estimation
- ✅ Phase 7: Risk Assessment
- ✅ Phase 8: Final Documentation

**Current Status:** **READY FOR IMPLEMENTATION**

**Approval Status:** Awaiting user review and approval

---

## Approval and Sign-Off

**Plan Created:** 2025-11-01
**Plan Status:** Complete - Ready for Implementation

**Phases 1-8 Deliverables:**
- ✅ Requirements extracted and cataloged (3 requirements)
- ✅ Specification issues identified and resolved (8 issues)
- ✅ Acceptance tests defined (11 tests, 100% coverage)
- ✅ Traceability matrix complete (0 gaps)
- ✅ Code analyzed, approach selected (Functional Decomposition)
- ✅ Implementation broken down (6 increments)
- ✅ Effort estimated (8-12 hours, confidence 70-75%)
- ✅ Risks assessed and mitigated (Overall: LOW)
- ✅ Context-window-optimized documentation (<500 line summary)

**Estimated Effort:** 8-12 hours (conservative: 15 hours)
**Estimated Timeline:** 3 days distributed
**Overall Risk:** LOW (all medium risks mitigated)

**Recommendation:** **PROCEED WITH IMPLEMENTATION**

**Next Action:** Begin Increment 1 (Baseline & Analysis)

---

**PLAN016 Summary Complete**
**Document Version:** 2.0 (All 8 Phases Complete)
**Last Updated:** 2025-11-01
**Total Lines:** ~470 (within <500 line target for summaries)
