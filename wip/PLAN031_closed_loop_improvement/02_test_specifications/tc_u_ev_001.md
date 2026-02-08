# TC-U-EV-001: Classify TRUE_POSITIVE correctly

**Test Type:** Unit Test
**Requirement:** SPEC031-EV-020
**Scope:** `EvaluationEngine::classify_result()`
**Category:** Happy Path

## Test Specification

**Given:**
- Ground truth entry with expected_mbid = "abc123"
- Import result with assigned_mbid = "abc123"

**When:**
- `classify_result(ground_truth, import_result)` is called

**Then:**
- Returns `Classification::TruePositive`

**Verify:**
```rust
let gt = GroundTruth { expected_mbid: Some("abc123".into()), .. };
let result = ImportResult { assigned_mbid: Some("abc123".into()), .. };
assert_eq!(engine.classify_result(&gt, &result), Classification::TruePositive);
```

**Pass Criteria:** Classification equals TruePositive
**Fail Criteria:** Any other classification returned
**Confidence:** HIGH - Direct requirement mapping

**Estimated Effort:** 15 minutes
