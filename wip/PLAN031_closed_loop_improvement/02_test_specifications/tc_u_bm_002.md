# TC-U-BM-002: Batch stops at failure threshold

**Test Type:** Unit Test
**Requirement:** SPEC031-BM-010
**Scope:** `BatchOrchestrator::process_batch()`
**Category:** Boundary

## Test Specification

**Given:**
- BatchConfig with failure_threshold = 3
- 10 files to process
- Files 3, 5, 6 will fail (consecutive at 5, 6)

**When:**
- `process_batch()` is called

**Then:**
- Processing stops after file 6 (3rd failure, 2 consecutive)
- Wait, need to clarify: Per ISSUE-H-001 resolution, consecutive means "in a row"
- Files 5, 6 are consecutive failures (count = 2)
- Need file 7 to also fail to hit threshold

**Revised Given:**
- Files 5, 6, 7 will fail (3 consecutive)

**Revised Then:**
- Processing stops after file 7
- BatchReport shows 7 files processed, not 10
- BatchReport shows stop_reason = "failure_threshold_reached"

**Verify:**
```rust
let report = orchestrator.process_batch(&config).await?;
assert_eq!(report.files_processed, 7);
assert_eq!(report.stop_reason, StopReason::FailureThreshold);
```

**Pass Criteria:** Batch stops exactly at 3rd consecutive failure
**Fail Criteria:** Processes all 10 files OR stops too early
**Confidence:** HIGH - Clear boundary behavior

**Estimated Effort:** 30 minutes (needs test fixtures)
