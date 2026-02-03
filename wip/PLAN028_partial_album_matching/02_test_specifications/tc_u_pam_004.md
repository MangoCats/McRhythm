# TC-U-PAM-004: Calculate Cumulative Track Durations

**Test Type:** Unit Test
**Requirement:** REQ-PAM-002
**Priority:** P0

---

## Scope

Function: `calculate_cumulative_durations(track_durations: &[f64]) -> Vec<f64>`

---

## Test Cases

### TC-U-PAM-004-A: Basic Cumulative

**Given:**
- Track durations: [300, 250, 280, 320, 290]

**When:** `calculate_cumulative_durations(&[300.0, 250.0, 280.0, 320.0, 290.0])`

**Then:** Returns `[300.0, 550.0, 830.0, 1150.0, 1440.0]`

**Pass Criteria:** Each element is sum of all previous plus current

---

### TC-U-PAM-004-B: Single Track

**Given:**
- Track durations: [400]

**When:** `calculate_cumulative_durations(&[400.0])`

**Then:** Returns `[400.0]`

**Pass Criteria:** Single track returns single cumulative value

---

### TC-U-PAM-004-C: Empty List

**Given:**
- Track durations: []

**When:** `calculate_cumulative_durations(&[])`

**Then:** Returns `[]`

**Pass Criteria:** Empty input returns empty output

---

## Estimated Effort

Implementation: 10 minutes
Test writing: 10 minutes
