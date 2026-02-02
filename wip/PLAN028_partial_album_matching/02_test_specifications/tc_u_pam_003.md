# TC-U-PAM-003: Skip Above 85% Ratio (Use Full Match)

**Test Type:** Unit Test
**Requirement:** REQ-PAM-001
**Priority:** P0

---

## Scope

Function: `is_partial_album_candidate(file_duration: f64, edition_duration: f64) -> bool`

---

## Test Cases

### TC-U-PAM-003-A: At Threshold (85%)

**Given:**
- File duration: 3400s
- Edition duration: 4000s
- Ratio: 85.0%

**When:** `is_partial_album_candidate(3400.0, 4000.0)`

**Then:** Returns `false`

**Rationale:** At 85%, existing full-match algorithm should be used

**Pass Criteria:** Exactly 85% uses full match path

---

### TC-U-PAM-003-B: Above Threshold (95%)

**Given:**
- File duration: 3800s
- Edition duration: 4000s
- Ratio: 95.0%

**When:** `is_partial_album_candidate(3800.0, 4000.0)`

**Then:** Returns `false`

**Pass Criteria:** Above 85% always uses full match path

---

## Estimated Effort

Implementation: 5 minutes (part of TC-U-PAM-001)
Test writing: 5 minutes
