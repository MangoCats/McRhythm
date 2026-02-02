# TC-U-010-01: Deluxe Keyword Detection (Lowercase)

**Test ID:** TC-U-010-01
**Test Type:** Unit Test
**Requirement:** REQ-EF-010 (Edition Preference Scoring - Deluxe Penalty)
**Priority:** High
**Estimated Effort:** 15 minutes (write + implement)

---

## Test Objective

Verify that `score_edition_preference()` correctly detects "deluxe" keyword in edition titles (lowercase variant).

---

## Test Specification

**Scope:** Function `score_edition_preference(edition: &Edition, detected_track_count: usize) -> f64`

**Given:**
- Edition with title containing "deluxe" (lowercase)
- Example: "Surrender (deluxe edition)"
- Detected track count: 11 (arbitrary, not used in this test)

**When:**
- Call `score_edition_preference(edition, 11)`

**Then:**
- Function returns score with 0.7× multiplier applied
- Base score = 1.0 → returned score = 0.7

**Verify:**
- `assert_eq!(score, 0.7)`
- Score is exactly 0.7 (not 1.0, not other value)

---

## Pass Criteria

✅ Test passes if score = 0.7 when edition title contains "deluxe"

❌ Test fails if:
- Score != 0.7
- "deluxe" keyword not detected
- Function panics or returns error

---

## Implementation Example

```rust
#[test]
fn test_deluxe_lowercase_detection() {
    // Arrange
    let edition = Edition {
        mbid: "test-mbid".to_string(),
        title: "Surrender (deluxe edition)".to_string(),
        track_count: 28,
        tracks: vec![],
    };
    let detected_track_count = 11;

    // Act
    let score = score_edition_preference(&edition, detected_track_count);

    // Assert
    assert_eq!(score, 0.7, "Deluxe edition should receive 0.7× multiplier");
}
```

---

## Test Data

**Input:**
- Edition title: "Surrender (deluxe edition)"
- Track count: 11 (arbitrary)

**Expected Output:**
- Score: 0.7

---

## Edge Cases Covered

- Lowercase "deluxe" (this test)
- See TC-U-010-02 for mixed case ("Deluxe", "DELUXE")
- See TC-U-010-03 for "expanded" keyword

---

## Dependencies

- Function `score_edition_preference()` must exist
- Edition struct must be defined
- No external dependencies (pure unit test)

---

## Notes

**Rationale:** Chemical Brothers - Surrender has "deluxe" in title and matched incorrectly. This test ensures deluxe penalty is applied.

**Related Tests:**
- TC-U-010-02 (mixed case)
- TC-I-010-01 (integration with matching)
- TC-S-010-01 (Chemical Brothers system test)
