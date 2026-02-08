# PLAN031: Closed-Loop Algorithm Improvement System

**Status:** Ready for Implementation
**Created:** 2025-12-13
**Specification:** [SPEC031_closed_loop_algorithm_improvement.md](../SPEC031_closed_loop_algorithm_improvement.md)
**Complexity:** STANDARD (14 requirements)
**Total Effort:** 32-36 hours

---

## Executive Summary

PLAN031 implements a test-driven feedback system for improving MBID assignment accuracy. The system:

1. **Loads ground truth** from embedded MBID tags or curated JSON files
2. **Runs test batches** against the existing import pipeline
3. **Evaluates accuracy** with TP/FP/TN/FN classification
4. **Analyzes failures** to identify patterns and root causes
5. **Generates improvement suggestions** for the AI agent to implement
6. **Tracks progress** across batches until >90% accuracy achieved

**Target:** Enable iterative algorithm improvement achieving >90% MBID accuracy.

---

## Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Integration Approach | Optional Evaluator Pattern | Lowest risk, matches existing patterns |
| Hook Strategy | Fire-and-forget with logging | Non-invasive, never breaks imports |
| Ground Truth Priority | Embedded tags > JSON > Cross-validation | Reliability ordering |
| Phase Progression | 3 phases (Small → Medium → Large) | Balance iteration speed with validation |

---

## Requirements Summary

| Priority | Count | Coverage |
|----------|-------|----------|
| P0 (Must Have) | 7 | 100% tested |
| P1 (Should Have) | 6 | 100% tested |
| P2 (Nice to Have) | 1 | Minimal testing |
| **Total** | **14** | **100%** |

---

## Implementation Plan

### Increments (9 total)

| # | Name | Effort | Status |
|---|------|--------|--------|
| 1 | Testing Module Foundation | 2-3h | Pending |
| 2 | Ground Truth Schema & Store | 2-3h | Pending |
| 3 | Evaluation Engine | 3-4h | Pending |
| 4 | Batch Orchestrator | 3-4h | Pending |
| 5 | Pipeline Integration Hooks | 2-3h | Pending |
| 6 | Failure Analyzer | 3-4h | Pending |
| 7 | Database Reset | 2h | Pending |
| 8 | Reporting & CLI | 3-4h | Pending |
| 9 | Integration Testing | 2-3h | Pending |

### Critical Path

**I1 → I2 → I3 → I4 → I8 → I9** (15-21 hours)

### Checkpoints

1. **After I3:** Foundation complete, evaluation working
2. **After I7:** Core system complete, all unit tests pass
3. **After I8:** CLI ready, reports working
4. **After I9:** Integration verified, system complete

---

## Effort Estimate

| Scenario | Hours | Probability |
|----------|-------|-------------|
| Expected | 26h | 55% |
| Risk-Adjusted | 36h | - |
| With Buffer | 36h | Recommended |

---

## Risk Summary

| Risk | Probability | Residual | Mitigation |
|------|-------------|----------|------------|
| Insufficient ground truth | MEDIUM | LOW-MEDIUM | Scan for embedded tags first |
| Pipeline hook side effects | LOW | LOW | Fire-and-forget pattern |
| API rate limiting | HIGH | LOW | Use cache, add backoff |
| False improvement detection | MEDIUM | LOW-MEDIUM | Validation set holdout |

---

## Specification Issues Resolved

| Issue | Resolution |
|-------|------------|
| "Consecutive failures" ambiguous | 3 failures in a row, reset on success |
| Phase 3 "speed targets" undefined | <1.2s/file for 500+ file batches |
| Ground truth reliability | Confidence weighting per entry |

---

## Files to Create

```
wkmp-ai/src/testing/
├── mod.rs           # Module exports
├── types.rs         # Core types (enums, structs)
├── ground_truth.rs  # Ground truth store
├── evaluation.rs    # Evaluation engine
├── batch.rs         # Batch orchestrator
├── failure.rs       # Failure analyzer
├── reset.rs         # Database reset
├── reporting.rs     # Report generation
└── cli.rs           # CLI commands

migrations/
└── YYYYMMDDHHMMSS_ground_truth.sql
```

---

## Success Criteria

- [ ] All 28 tests pass
- [ ] Can run `cargo run -p wkmp-ai -- test-batch --size 10`
- [ ] Batch report displays accuracy metrics
- [ ] Failure patterns identified automatically
- [ ] Normal imports unaffected when testing disabled

---

## Next Steps

1. Review this plan summary
2. Begin Increment 1: Testing Module Foundation
3. Follow checkpoints for verification
4. After I9 complete: Execute Phase 9 (Technical Debt Review)

---

## Document Index

| Document | Purpose |
|----------|---------|
| [requirements_index.md](requirements_index.md) | All 14 requirements |
| [scope_statement.md](scope_statement.md) | In/out of scope |
| [01_specification_issues.md](01_specification_issues.md) | Resolved issues |
| [02_test_specifications/](02_test_specifications/) | All test cases |
| [03_approach_selection.md](03_approach_selection.md) | Architecture decision |
| [04_increments/](04_increments/) | Implementation details |
| [05_estimates.md](05_estimates.md) | Effort breakdown |
| [06_risks.md](06_risks.md) | Risk analysis |
