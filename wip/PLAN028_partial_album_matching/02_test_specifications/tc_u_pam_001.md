# TC-U-PAM-001: Detect Partial Album Candidate

**Test Type:** Unit Test
**Requirement:** REQ-PAM-001
**Priority:** P0

---

## Scope

Function: `is_partial_album_candidate(file_duration: f64, edition_duration: f64) -> bool`

---

## Test Cases

### TC-U-PAM-001-A: Lower Bound (50%)

**Given:**
- File duration: 2000s
- Edition duration: 4000s
- Ratio: 50.0%

**When:** `is_partial_album_candidate(2000.0, 4000.0)`

**Then:** Returns `true`

**Pass Criteria:** Function returns true for exactly 50% ratio

---

### TC-U-PAM-001-B: Upper Bound (84.9%)

**Given:**
- File duration: 3396s
- Edition duration: 4000s
- Ratio: 84.9%

**When:** `is_partial_album_candidate(3396.0, 4000.0)`

**Then:** Returns `true`

**Pass Criteria:** Function returns true for just under 85%

---

### TC-U-PAM-001-C: Typical Partial (74.2% - Puppy case)

**Given:**
- File duration: 2995s
- Edition duration: 4039s
- Ratio: 74.2%

**When:** `is_partial_album_candidate(2995.0, 4039.0)`

**Then:** Returns `true`

**Pass Criteria:** Function returns true for Fluke/Puppy.mp3 scenario

---

## Estimated Effort

Implementation: 15 minutes
Test writing: 10 minutes
