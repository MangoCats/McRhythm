# TC-I-PAM-002: Quality Threshold for Partial Matches

**Test Type:** Integration Test
**Requirement:** REQ-PAM-005
**Priority:** P1

---

## Scope

Verify 80% quality threshold applies to partial matches

---

## Setup

- Mock boundary detection with controlled matches
- Partial match with 8 tracks expected

---

## Test Cases

### TC-I-PAM-002-A: Above Quality Threshold

**Given:**
- 8 tracks in partial match
- 7 of 8 tracks (87.5%) within 10s tolerance
- 1 track outside tolerance

**When:** Quality threshold is evaluated

**Then:**
- Match accepted (87.5% > 80%)
- Result `matched: true`

---

### TC-I-PAM-002-B: Below Quality Threshold

**Given:**
- 8 tracks in partial match
- 6 of 8 tracks (75%) within 10s tolerance
- 2 tracks outside tolerance

**When:** Quality threshold is evaluated

**Then:**
- Match rejected (75% < 80%)
- Result `matched: false` with reason "Quality threshold not met"

---

### TC-I-PAM-002-C: At Quality Threshold

**Given:**
- 10 tracks in partial match
- 8 of 10 tracks (80%) within 10s tolerance

**When:** Quality threshold is evaluated

**Then:**
- Match accepted (80% >= 80%)
- Result `matched: true`

---

## Pass Criteria

- 80% threshold enforced consistently
- Threshold applies to matched tracks only (not total album)

---

## Estimated Effort

Implementation: 20 minutes (uses existing quality scoring)
Test writing: 20 minutes
