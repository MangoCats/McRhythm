# Effort and Schedule Estimation: PLAN016 Engine Refactoring

**Plan:** PLAN016
**Date:** 2025-11-01
**Estimation Method:** Bottom-up (task-based) with historical comparison
**Confidence Level:** Medium-High (70-80%)

---

## Executive Summary

**Total Estimated Effort:** 8-12 hours
**Conservative Estimate:** 15 hours (with 25% contingency)
**Recommended Timeline:** 2-3 working days (non-consecutive)
**Risk-Adjusted Effort:** 10-13 hours (most likely)

**Breakdown:**
- Implementation: 6.5-10 hours (75%)
- Testing & Verification: 1.5-2 hours (15%)
- Buffer/Contingency: 1-2 hours (10%)

---

## Detailed Effort Estimates

### Increment 1: Baseline & Analysis

| Task | Optimistic | Most Likely | Pessimistic | Estimate |
|------|------------|-------------|-------------|----------|
| Run baseline tests | 15 min | 30 min | 45 min | 30 min |
| Document public API | 10 min | 15 min | 30 min | 15 min |
| Measure line counts | 3 min | 5 min | 10 min | 5 min |
| Create code mapping | 30 min | 45 min | 90 min | 45 min |
| **Subtotal** | **58 min** | **1h 35min** | **2h 55min** | **1-2 hours** |

**Rationale:**
- Baseline tests: Standard `cargo test` run (~30 min for full suite)
- API documentation: Simple `grep` extraction
- Code mapping: Analysis already done in Phase 4, just formalize

**Dependencies:**
- None (can start immediately)

**Risks:**
- Test suite may take longer if many tests
- Code mapping may reveal unexpected complexity

---

### Increment 2: Module Structure Creation

| Task | Optimistic | Most Likely | Pessimistic | Estimate |
|------|------------|-------------|-------------|----------|
| Create directories/files | 3 min | 5 min | 10 min | 5 min |
| Write mod.rs skeleton | 5 min | 10 min | 20 min | 10 min |
| Write module skeletons | 10 min | 15 min | 30 min | 15 min |
| Verify compilation | 2 min | 5 min | 10 min | 5 min |
| **Subtotal** | **20 min** | **35 min** | **1h 10min** | **30 min** |

**Rationale:**
- Straightforward file creation and boilerplate
- Low complexity, minimal risk

**Dependencies:**
- Increment 1 complete (code mapping available)

**Risks:**
- Very low risk (boilerplate code)

---

### Increment 3: Extract Queue Module

| Task | Optimistic | Most Likely | Pessimistic | Estimate |
|------|------------|-------------|-------------|----------|
| Move queue operations (5 methods) | 45 min | 75 min | 120 min | 75 min |
| Move queue queries (2 methods) | 20 min | 30 min | 45 min | 30 min |
| Move queue helpers (2 functions) | 10 min | 15 min | 30 min | 15 min |
| Fix imports | 10 min | 15 min | 30 min | 15 min |
| Verify line count | 3 min | 5 min | 10 min | 5 min |
| Run tests | 5 min | 10 min | 20 min | 10 min |
| Debug issues (if any) | 0 min | 30 min | 90 min | 30 min |
| **Subtotal** | **1h 33min** | **3h** | **5h 45min** | **2-3 hours** |

**Rationale:**
- 9 methods/functions to move (~20-40 min each)
- Import fixing can be time-consuming (missing dependencies)
- Debug time budgeted for compilation errors

**Dependencies:**
- Increment 2 complete (skeleton exists)

**Risks:**
- Import errors (medium probability)
- Lifetime/borrow checker issues (low probability - no signature changes)

---

### Increment 4: Extract Diagnostics Module

| Task | Optimistic | Most Likely | Pessimistic | Estimate |
|------|------------|-------------|-------------|----------|
| Move status accessors (8 methods) | 30 min | 45 min | 75 min | 45 min |
| Move monitoring config (2 methods) | 5 min | 10 min | 15 min | 10 min |
| Move event handlers (4 functions) | 60 min | 90 min | 150 min | 90 min |
| Update handler spawns in start() | 10 min | 15 min | 30 min | 15 min |
| Fix imports | 10 min | 15 min | 30 min | 15 min |
| Verify line count | 3 min | 5 min | 10 min | 5 min |
| Run tests | 5 min | 10 min | 20 min | 10 min |
| Debug issues (if any) | 0 min | 30 min | 90 min | 30 min |
| **Subtotal** | **2h 3min** | **3h 40min** | **7h** | **2-3 hours** |

**Rationale:**
- Event handlers are large (160-300 lines each) but self-contained
- Handler spawning may require async lifetime adjustments
- Similar complexity to Increment 3

**Dependencies:**
- Increment 3 complete (queue.rs stable)

**Risks:**
- Async lifetime issues in handler spawns (medium probability)
- Handler tasks may fail to compile if dependencies unclear

---

### Increment 5: Finalize Core Module

| Task | Optimistic | Most Likely | Pessimistic | Estimate |
|------|------------|-------------|-------------|----------|
| Move remaining code to core.rs | 30 min | 45 min | 75 min | 45 min |
| Update mod.rs re-exports | 10 min | 15 min | 30 min | 15 min |
| Fix imports | 5 min | 10 min | 20 min | 10 min |
| Verify line count | 3 min | 5 min | 10 min | 5 min |
| Verify compilation | 2 min | 5 min | 10 min | 5 min |
| **Subtotal** | **50 min** | **1h 20min** | **2h 25min** | **1 hour** |

**Rationale:**
- Most code already moved (queue, diagnostics)
- Core.rs is "what's left" - simpler than selective extraction
- Minimal risk (final cleanup)

**Dependencies:**
- Increments 3 & 4 complete

**Risks:**
- Low risk (mostly cleanup)

---

### Increment 6: Verification & Cleanup

| Task | Optimistic | Most Likely | Pessimistic | Estimate |
|------|------------|-------------|-------------|----------|
| Delete engine.rs | 1 min | 2 min | 5 min | 2 min |
| Verify compilation | 2 min | 5 min | 10 min | 5 min |
| Run full test suite | 5 min | 10 min | 20 min | 10 min |
| Compare test results | 3 min | 5 min | 10 min | 5 min |
| Run 11 acceptance tests | 20 min | 30 min | 60 min | 30 min |
| Manual code review | 10 min | 15 min | 30 min | 15 min |
| Manual caller review | 5 min | 10 min | 20 min | 10 min |
| Update traceability matrix | 5 min | 10 min | 20 min | 10 min |
| Create implementation report | 10 min | 15 min | 30 min | 15 min |
| Fix any issues found | 0 min | 30 min | 120 min | 30 min |
| **Subtotal** | **1h 1min** | **2h 12min** | **5h 25min** | **1-2 hours** |

**Rationale:**
- Most time in running tests and reviews
- Contingency for fixing issues discovered during final verification
- Implementation report is documentation (low effort)

**Dependencies:**
- All previous increments complete

**Risks:**
- May discover issues requiring rework (medium probability)
- Test failures would require debugging (budgeted in "fix issues")

---

## Total Effort Summary

| Increment | Optimistic | Most Likely | Pessimistic | Estimate Range |
|-----------|------------|-------------|-------------|----------------|
| 1. Baseline | 58 min | 1h 35min | 2h 55min | 1-2 hours |
| 2. Structure | 20 min | 35 min | 1h 10min | 30 min |
| 3. Queue | 1h 33min | 3h | 5h 45min | 2-3 hours |
| 4. Diagnostics | 2h 3min | 3h 40min | 7h | 2-3 hours |
| 5. Core | 50 min | 1h 20min | 2h 25min | 1 hour |
| 6. Verification | 1h 1min | 2h 12min | 5h 25min | 1-2 hours |
| **Total** | **6h 25min** | **12h 22min** | **24h 40min** | **8-12 hours** |

**Weighted Average (PERT):**
- Formula: (Optimistic + 4×Most Likely + Pessimistic) / 6
- Calculation: (6.42 + 4×12.37 + 24.67) / 6 = **13.2 hours**

**Recommended Estimate:** **10-13 hours**

**With 25% Contingency:** **12.5-16 hours**
**Conservative Estimate:** **15 hours**

---

## Schedule Estimate

### Timeline Options

**Option A: Focused Sprint (2 days)**
- Day 1 (8 hours): Increments 1-4
- Day 2 (4-5 hours): Increments 5-6
- **Total:** 2 consecutive days, 12-13 hours

**Option B: Distributed (3 days)**
- Day 1 (4 hours): Increments 1-2
- Day 2 (4-5 hours): Increments 3-4
- Day 3 (3-4 hours): Increments 5-6
- **Total:** 3 non-consecutive days, 11-13 hours

**Option C: Conservative (4 days)**
- Day 1 (2-3 hours): Increments 1-2
- Day 2 (3-4 hours): Increment 3
- Day 3 (3-4 hours): Increment 4
- Day 4 (3-4 hours): Increments 5-6
- **Total:** 4 non-consecutive days, 11-15 hours

**Recommended:** Option B (Distributed) - Balances progress with natural break points

---

## Resource Requirements

**Personnel:**
- 1 developer (primary implementer)
- Optional: 1 reviewer (code review at checkpoints)

**Tools:**
- Rust toolchain (cargo, rustc)
- Text editor / IDE
- Git (version control)
- Terminal (bash scripts for tests)

**Environment:**
- Development machine with wkmp-ap crate
- Test environment (same machine)

**No External Resources Required**

---

## Dependencies and Blockers

**Internal Dependencies:**
- Access to wkmp-ap codebase
- Working test suite (verified in Increment 1)

**External Dependencies:**
- None

**Potential Blockers:**
- Test suite broken (discovered in Increment 1) - Would require fixing before refactoring
- Concurrent changes to engine.rs by other developers - Requires coordination

**Mitigation:**
- Run baseline tests BEFORE starting (Increment 1 catches test issues early)
- Check for active branches touching engine.rs before starting

---

## Variance Analysis

**Historical Comparison:**

Similar refactoring (PLAN014 - Mixer Refactoring):
- Original estimate: 8-12 hours
- Actual effort: ~11 hours (Sub-Increments 4a, 4b, 4c combined)
- Variance: Within range (good estimate)

**Complexity Comparison:**

| Factor | Mixer (PLAN014) | Engine (PLAN016) | Impact |
|--------|-----------------|------------------|--------|
| Original size | 1,933 lines | 4,251 lines | +2.2x |
| Modules created | 1 file → 1 file | 1 file → 3 modules | +Complexity |
| Public API changes | Some (refactor) | None (internal only) | -Risk |
| Test coverage | 42 new tests | 11 acceptance tests | Similar |

**Adjustment:**
- Engine is 2.2x larger → Estimate 2x effort: 16-24 hours
- BUT: No API changes (lower risk) → Reduce 30%: 11-17 hours
- AND: Clear boundaries identified (Phase 4 analysis) → Reduce 10%: 10-15 hours

**Final Adjustment:** 8-12 hour estimate validated by comparison

---

## Confidence Levels

| Increment | Confidence | Rationale |
|-----------|------------|-----------|
| 1. Baseline | 90% | Standard tasks, no unknowns |
| 2. Structure | 95% | Boilerplate only |
| 3. Queue | 70% | Import issues possible |
| 4. Diagnostics | 65% | Handler async lifetimes uncertain |
| 5. Core | 75% | Cleanup phase, lower risk |
| 6. Verification | 80% | Known tests, may find issues |

**Overall Confidence:** 70-75% (Medium-High)

**Reasoning:**
- Well-defined scope (Phases 1-4 complete)
- Clear module boundaries (Phase 4 analysis)
- Comprehensive tests (Phase 3)
- Uncertainty in async handler lifetimes (Increment 4)

---

## Contingency Planning

**Contingency Budget:** 25% (3-4 hours)

**Allocated to:**
- Import/dependency issues (1 hour)
- Async lifetime debugging (1 hour)
- Test failures requiring investigation (1 hour)
- Unexpected compilation errors (30 min)
- Buffer for breaks/context switching (30 min)

**Burn Rate Monitoring:**

Track actual vs. estimated at each increment:
- After Increment 1: On track if ≤2 hours
- After Increment 3: On track if ≤5 hours
- After Increment 5: On track if ≤9 hours

**If Over Budget:**
- Review remaining work
- Identify cause (complexity underestimated? unexpected issues?)
- Adjust estimates for remaining increments
- Consider pausing for additional planning

---

## Effort Distribution

**By Activity Type:**

| Activity | Estimated Hours | Percentage |
|----------|-----------------|------------|
| Code migration | 6.5 hours | 54% |
| Import/compile fixing | 2 hours | 17% |
| Testing & verification | 2 hours | 17% |
| Documentation | 0.5 hours | 4% |
| Contingency | 1 hour | 8% |
| **Total** | **12 hours** | **100%** |

**By Phase:**

| Phase | Estimated Hours | Percentage |
|-------|-----------------|------------|
| Setup (Inc 1-2) | 1.5 hours | 13% |
| Implementation (Inc 3-5) | 6.5 hours | 54% |
| Verification (Inc 6) | 2 hours | 17% |
| Contingency | 2 hours | 17% |
| **Total** | **12 hours** | **100%** |

---

## Recommendations

**1. Time Allocation:**
- Reserve 12-15 hours for implementation
- Schedule in 3-4 hour blocks (natural increment boundaries)
- Avoid context switching mid-increment

**2. Checkpoints:**
- Stop after Increment 3 to verify queue.rs (natural break)
- Stop after Increment 4 to verify diagnostics.rs (natural break)
- Run full verification in Increment 6 (do not skip)

**3. Risk Management:**
- If any increment takes 2x estimated time: STOP, reassess
- If tests fail in Increment 6: DO NOT PROCEED, debug first
- Keep engine.rs until Increment 6 (fallback available)

**4. Schedule Flexibility:**
- Build in 1-2 hour buffer between days (for context switching)
- If Option B (3 days): Allow 4th day if needed
- Do not compress into single day (fatigue risk)

---

**Effort and Schedule Estimation Complete**
**Phase 6 Status:** ✓ Estimates defined with confidence levels
**Recommended Effort:** 10-13 hours (conservative: 15 hours)
**Recommended Timeline:** 3 days (distributed)
**Next Phase:** Risk Assessment (Phase 7)
