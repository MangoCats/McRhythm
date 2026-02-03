# TC-U-PAM-002: Reject Below 50% Ratio

**Test Type:** Unit Test
**Requirement:** REQ-PAM-001
**Priority:** P0

---

## Scope

Function: `is_partial_album_candidate(file_duration: f64, edition_duration: f64) -> bool`

---

## Test Cases

### TC-U-PAM-002-A: Below Threshold (49%)

**Given:**
- File duration: 1960s
- Edition duration: 4000s
- Ratio: 49.0%

**When:** `is_partial_album_candidate(1960.0, 4000.0)`

**Then:** Returns `false`

**Pass Criteria:** Just below 50% is rejected

---

### TC-U-PAM-002-B: Very Low Ratio (25%)

**Given:**
- File duration: 1000s
- Edition duration: 4000s
- Ratio: 25.0%

**When:** `is_partial_album_candidate(1000.0, 4000.0)`

**Then:** Returns `false`

**Pass Criteria:** Very low ratios are rejected

---

## Estimated Effort

Implementation: 5 minutes (part of TC-U-PAM-001)
Test writing: 5 minutes
