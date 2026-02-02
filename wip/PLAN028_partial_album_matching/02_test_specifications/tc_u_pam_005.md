# TC-U-PAM-005: Find Best N Tracks Matching File Duration

**Test Type:** Unit Test
**Requirement:** REQ-PAM-002
**Priority:** P0

---

## Scope

Function: `find_partial_track_count(file_duration: f64, track_durations: &[f64], tolerance: f64) -> Option<usize>`

---

## Test Cases

### TC-U-PAM-005-A: Puppy.mp3 Scenario

**Given:**
- File duration: 2995s
- Track durations (Fluke - Puppy, 11 tracks):
  - Track 1: 373s (Absurd)
  - Track 2: 362s (Atom Bomb)
  - Track 3: 383s (Bullet)
  - Track 4: 365s (Squirt)
  - Track 5: 402s (Play Thing)
  - Track 6: 333s (Kitten Moon)
  - Track 7: 349s (Switch/Twitch)
  - Track 8: 422s (YKK)
  - Track 9: 348s (Goodnight Lover)
  - Track 10: 362s (Reeferendrum)
  - Track 11: 340s (My Spine)
- Cumulative durations:
  - 1-8: 2989s
  - 1-9: 3337s
- Tolerance: 2%

**When:** `find_partial_track_count(2995.0, &track_durations, 0.02)`

**Then:** Returns `Some(8)`

**Verify:**
- `|2995 - 2989| / 2989 = 0.2%` (within 2% tolerance)
- `|2995 - 3337| / 3337 = 10.2%` (exceeds tolerance, 9 tracks rejected)

**Pass Criteria:** Returns 8 tracks (not 7, not 9)

---

### TC-U-PAM-005-B: No Match Within Tolerance

**Given:**
- File duration: 1500s (doesn't match any cumulative)
- Track durations: [400, 400, 400, 400, 400] (cumulative: 400, 800, 1200, 1600, 2000)
- Tolerance: 2%

**When:** `find_partial_track_count(1500.0, &track_durations, 0.02)`

**Then:** Returns `None`

**Pass Criteria:** Returns None when no cumulative matches file within tolerance

---

### TC-U-PAM-005-C: Exact Match

**Given:**
- File duration: 1200s
- Track durations: [400, 400, 400, 400, 400]
- Tolerance: 2%

**When:** `find_partial_track_count(1200.0, &track_durations, 0.02)`

**Then:** Returns `Some(3)`

**Pass Criteria:** Exact cumulative match returns correct track count

---

## Estimated Effort

Implementation: 30 minutes
Test writing: 20 minutes
