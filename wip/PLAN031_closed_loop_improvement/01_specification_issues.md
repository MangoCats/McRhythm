# PLAN031 Specification Issues

## Critical Issues (0)

None identified.

## High Priority Issues (3)

### ISSUE-H-001: "Consecutive failures" ambiguous
**Requirement:** SPEC031-BM-020 (Phase 1 stopping criteria)
**Issue:** §4.1 says "Stop after 3 consecutive failures indicate pattern" but doesn't clarify:
- Consecutive in processing order? Or consecutive same-category failures?
- Does a success reset the counter?
**Confidence:** CERTAIN - Multiple valid interpretations exist
**Impact:** Incorrect stopping behavior could cause premature stops or runaway batches
**Resolution:** Define as "3 failures in a row during processing" with counter reset on success

### ISSUE-H-002: Phase 3 "speed targets" undefined
**Requirement:** SPEC031-BM-020 (Phase 3 exit criteria)
**Issue:** §4.3 says Phase 3 exits when "Speed targets met" but no targets defined
**Confidence:** CERTAIN - No numeric targets in specification
**Impact:** Cannot determine when Phase 3 optimization is complete
**Resolution:** Add target: "Process 500 files in <10 minutes (avg <1.2s/file)"

### ISSUE-H-003: Ground truth reliability not addressed
**Requirement:** SPEC031-GT-010 (Ground truth management)
**Issue:** Specification assumes ground truth is always correct. What if:
- Embedded MBID tag is wrong (user error, bad tagger)
- Cross-validation produces false agreement
**Confidence:** HIGH - Real-world data quality issues likely
**Impact:** False negatives could poison improvement feedback loop
**Resolution:** Add confidence weighting per ground truth entry; flag low-confidence entries in reports

## Medium Priority Issues (4)

### ISSUE-M-001: Database transaction boundaries for hooks
**Requirement:** SPEC031-API-010 (Pipeline integration)
**Issue:** §9.2 shows hooks but doesn't specify:
- Should recording happen inside or outside existing transactions?
- What if hook fails - should it fail the import?
**Confidence:** HIGH - Transaction semantics affect correctness
**Resolution:** Hooks should be fire-and-forget, logged on failure, never fail import

### ISSUE-M-002: Batch processing order unspecified
**Requirement:** SPEC031-BM-010 (Batch management)
**Issue:** No specification for file ordering within batch:
- Random? Alphabetical? By ground truth source?
**Confidence:** MEDIUM - May affect reproducibility
**Resolution:** Default to deterministic order (sorted by file_hash) for reproducible results

### ISSUE-M-003: "Diminishing returns" not quantified
**Requirement:** SPEC031-BM-020 (Phase 3 exit)
**Issue:** One exit criterion is "diminishing returns" but not defined
**Confidence:** CERTAIN - No definition provided
**Resolution:** Define as "<0.5% accuracy improvement over 5 consecutive batches"

### ISSUE-M-004: Multiple test runs interaction
**Requirement:** Not explicitly covered
**Issue:** Can multiple test runs exist? What happens to old run data?
**Confidence:** MEDIUM - Common operational scenario
**Resolution:** Each run gets unique ID; old runs preserved unless explicitly deleted

## Low Priority Issues (2)

### ISSUE-L-001: Console vs JSON report format selection
**Requirement:** SPEC031-RP-010 (Batch reporting)
**Issue:** §8.1 shows console format but mentions "structured JSON output" without specifying when to use which
**Confidence:** LOW - Implementation can default sensibly
**Resolution:** CLI flag `--json` for machine-readable output, console default

### ISSUE-L-002: Improvement application verification
**Requirement:** SPEC031-FA-030 (Improvement suggestions)
**Issue:** `apply_improvement` API doesn't specify how to verify improvement was applied correctly
**Confidence:** LOW - Agent responsibility to verify
**Resolution:** Document as agent responsibility; add `--verify` flag to re-run failing files after applying

## Summary

| Severity | Count | Action |
|----------|-------|--------|
| Critical | 0 | N/A |
| High | 3 | Must resolve before implementation |
| Medium | 4 | Document resolution in plan |
| Low | 2 | Resolve during implementation |

**Recommendation:** Proceed to Phase 3 with documented resolutions for High/Medium issues.
