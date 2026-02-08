# TC-U-PAM-006: Enforce 60% Minimum Track Coverage

**Test Type:** Unit Test
**Requirement:** REQ-PAM-004
**Priority:** P1

---

## Scope

Function: `is_partial_match_acceptable(matched_tracks: usize, total_tracks: usize) -> bool`

---

## Test Cases

### TC-U-PAM-006-A: 11-Track Album at Minimum (7/11 = 63.6%)

**Given:**
- Matched tracks: 7
- Total tracks: 11

**When:** `is_partial_match_acceptable(7, 11)`

**Then:** Returns `true`

**Pass Criteria:** 63.6% coverage accepted (above 60%)

---

### TC-U-PAM-006-B: 11-Track Album Below Minimum (6/11 = 54.5%)

**Given:**
- Matched tracks: 6
- Total tracks: 11

**When:** `is_partial_match_acceptable(6, 11)`

**Then:** Returns `false`

**Pass Criteria:** 54.5% coverage rejected (below 60%)

---

### TC-U-PAM-006-C: Puppy Case (8/11 = 72.7%)

**Given:**
- Matched tracks: 8
- Total tracks: 11

**When:** `is_partial_match_acceptable(8, 11)`

**Then:** Returns `true`

**Pass Criteria:** 72.7% coverage accepted

---

### TC-U-PAM-006-D: 10-Track Album Minimum (6/10 = 60%)

**Given:**
- Matched tracks: 6
- Total tracks: 10

**When:** `is_partial_match_acceptable(6, 10)`

**Then:** Returns `true`

**Pass Criteria:** Exactly 60% is accepted

---

### TC-U-PAM-006-E: Below 60% (5/10 = 50%)

**Given:**
- Matched tracks: 5
- Total tracks: 10

**When:** `is_partial_match_acceptable(5, 10)`

**Then:** Returns `false`

**Pass Criteria:** 50% rejected (below 60% threshold)

---

## Estimated Effort

Implementation: 10 minutes
Test writing: 15 minutes
