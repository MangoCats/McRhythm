# TC-U-030-03: Track Count >3 Difference (Filtered)

**Test ID:** TC-U-030-03
**Test Type:** Unit Test
**Requirement:** REQ-EF-030 (Track Count Pre-Filtering)
**Priority:** High
**Estimated Effort:** 20 minutes (write + implement)

---

## Test Objective

Verify that `filter_by_track_count()` correctly filters editions where track count differs by >3 from detected boundaries.

---

## Test Specification

**Scope:** Function `filter_by_track_count(editions: &[Edition], detected: usize) -> Vec<Edition>`

**Given:**
- Detected track count: 11
- Edition list containing:
  1. Edition A: 11 tracks (exact match)
  2. Edition B: 13 tracks (within ±3)
  3. Edition C: 16 tracks (outside ±3, should be filtered)
  4. Edition D: 7 tracks (outside ±3, should be filtered)

**When:**
- Call `filter_by_track_count(editions, 11)`

**Then:**
- Returned list contains only Edition A and Edition B
- Edition C filtered (16 - 11 = 5 > 3)
- Edition D filtered (11 - 7 = 4 > 3)

**Verify:**
- `assert_eq!(filtered.len(), 2)`
- `assert!(filtered.iter().any(|e| e.title == "Edition A"))`
- `assert!(filtered.iter().any(|e| e.title == "Edition B"))`
- `assert!(!filtered.iter().any(|e| e.title == "Edition C"))`
- `assert!(!filtered.iter().any(|e| e.title == "Edition D"))`

---

## Pass Criteria

✅ Test passes if:
- Exactly 2 editions returned (A and B)
- Edition C and D filtered out
- No panics or errors

❌ Test fails if:
- Wrong number of editions returned
- Edition C or D not filtered
- Edition A or B incorrectly filtered

---

## Implementation Example

```rust
#[test]
fn test_track_count_filtering() {
    // Arrange
    let editions = vec![
        Edition {
            mbid: "a".to_string(),
            title: "Edition A".to_string(),
            track_count: 11,  // Exact match
            tracks: vec![],
        },
        Edition {
            mbid: "b".to_string(),
            title: "Edition B".to_string(),
            track_count: 13,  // Within ±3
            tracks: vec![],
        },
        Edition {
            mbid: "c".to_string(),
            title: "Edition C".to_string(),
            track_count: 16,  // Outside ±3 (16-11=5)
            tracks: vec![],
        },
        Edition {
            mbid: "d".to_string(),
            title: "Edition D".to_string(),
            track_count: 7,   // Outside ±3 (11-7=4)
            tracks: vec![],
        },
    ];
    let detected = 11;

    // Act
    let filtered = filter_by_track_count(&editions, detected);

    // Assert
    assert_eq!(filtered.len(), 2, "Should return exactly 2 editions");
    assert!(filtered.iter().any(|e| e.title == "Edition A"), "Edition A should pass");
    assert!(filtered.iter().any(|e| e.title == "Edition B"), "Edition B should pass");
    assert!(!filtered.iter().any(|e| e.title == "Edition C"), "Edition C should be filtered");
    assert!(!filtered.iter().any(|e| e.title == "Edition D"), "Edition D should be filtered");
}
```

---

## Test Data

**Input:**
- Detected: 11 tracks
- Editions:
  - A: 11 tracks (|11-11| = 0 ≤ 3) → PASS
  - B: 13 tracks (|13-11| = 2 ≤ 3) → PASS
  - C: 16 tracks (|16-11| = 5 > 3) → FILTERED
  - D: 7 tracks (|11-7| = 4 > 3) → FILTERED

**Expected Output:**
- Filtered list: [Edition A, Edition B]

---

## Edge Cases Covered

**Boundary Conditions:**
- Exact match (difference = 0) → passes
- Within tolerance (difference = 2) → passes
- Just outside tolerance (difference = 4) → filtered
- Far outside tolerance (difference = 5) → filtered

**See Also:**
- TC-U-030-01: Exact match case
- TC-U-030-02: Within ±3 case
- TC-U-030-04: Empty edition list
- TC-U-030-05: All editions filtered (fallback)

---

## Dependencies

- Function `filter_by_track_count()` must exist
- Edition struct must be defined
- No external dependencies

---

## Notes

**Rationale:** Imagine Dragons - Night Visions has 11 tracks but matched to 16-track deluxe edition. This test ensures editions with >3 track difference are filtered.

**Problem Album Context:**
- File: 11 tracks
- MusicBrainz editions: 11-track standard, 16-track deluxe
- Current behavior: Matches 16-track deluxe (wrong)
- Expected after fix: Filter 16-track deluxe, match 11-track standard

**Related Tests:**
- TC-I-030-01: Integration (filter before Stage 2)
- TC-S-030-01: Imagine Dragons system test
