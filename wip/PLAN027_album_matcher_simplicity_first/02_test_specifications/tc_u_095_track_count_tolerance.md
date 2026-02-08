# Unit Tests: Graduated Track Count Tolerance (REQ-AM-095)

**Requirement:** REQ-AM-095 - Graduated Track Count Tolerance
**Test Count:** 6 unit tests
**Priority:** P0 (Critical)

---

## TC-U-095-01: Exact Match (Penalty = 1.00)

**Test ID:** TC-U-095-01
**Test Type:** Unit Test
**Requirement:** REQ-AM-095
**Priority:** P0

### Given
- Detected track count: 11
- Edition track count: 11
- Difference: 0 tracks

### When
- `calculate_track_count_penalty(11, 11)` is called

### Then
- Penalty = 1.00 (no penalty for exact match)

### Verify
```rust
let penalty = calculate_track_count_penalty(11, 11);
assert_eq!(penalty, 1.00);
```

### Pass Criteria
- Returns 1.00 for exact match (difference = 0)

---

## TC-U-095-02: ±1 Track (Penalty = 0.95)

**Test ID:** TC-U-095-02
**Test Type:** Unit Test
**Requirement:** REQ-AM-095
**Priority:** P0

### Given
- **Case A:** Detected = 11, Edition = 12 (difference = 1)
- **Case B:** Detected = 12, Edition = 11 (difference = 1)

### When
- `calculate_track_count_penalty(11, 12)` is called
- `calculate_track_count_penalty(12, 11)` is called

### Then
- Both cases: Penalty = 0.95 (minimal penalty)

### Verify
```rust
let penalty_a = calculate_track_count_penalty(11, 12);
let penalty_b = calculate_track_count_penalty(12, 11);
assert_eq!(penalty_a, 0.95);
assert_eq!(penalty_b, 0.95);
```

### Pass Criteria
- Returns 0.95 for ±1 track difference
- Symmetric (direction doesn't matter)

**Note:** Represents Japanese edition with bonus track scenario

---

## TC-U-095-03: ±2 Tracks (Penalty = 0.85)

**Test ID:** TC-U-095-03
**Test Type:** Unit Test
**Requirement:** REQ-AM-095
**Priority:** P0

### Given
- Detected = 11, Edition = 13 (difference = 2)

### When
- `calculate_track_count_penalty(11, 13)` is called

### Then
- Penalty = 0.85

### Verify
```rust
let penalty = calculate_track_count_penalty(11, 13);
assert_eq!(penalty, 0.85);
```

### Pass Criteria
- Returns 0.85 for ±2 track difference

**Note:** Acceptable but needs good other parameters to compete

---

## TC-U-095-04: ±3 Tracks (Penalty = 0.70)

**Test ID:** TC-U-095-04
**Test Type:** Unit Test
**Requirement:** REQ-AM-095
**Priority:** P0

### Given
- Detected = 11, Edition = 14 (difference = 3)

### When
- `calculate_track_count_penalty(11, 14)` is called

### Then
- Penalty = 0.70

### Verify
```rust
let penalty = calculate_track_count_penalty(11, 14);
assert_eq!(penalty, 0.70);
```

### Pass Criteria
- Returns 0.70 for ±3 track difference

**Note:** Moderate penalty, requires excellent other parameters

---

## TC-U-095-05: ±4-5 Tracks (Penalty = 0.50)

**Test ID:** TC-U-095-05
**Test Type:** Unit Test
**Requirement:** REQ-AM-095
**Priority:** P0

### Given
- **Case A:** Detected = 11, Edition = 15 (difference = 4)
- **Case B:** Detected = 11, Edition = 16 (difference = 5)

### When
- `calculate_track_count_penalty(11, 15)` is called (difference = 4)
- `calculate_track_count_penalty(11, 16)` is called (difference = 5)

### Then
- Both cases: Penalty = 0.50

### Verify
```rust
let penalty_4 = calculate_track_count_penalty(11, 15);
let penalty_5 = calculate_track_count_penalty(11, 16);
assert_eq!(penalty_4, 0.50);
assert_eq!(penalty_5, 0.50);
```

### Pass Criteria
- Returns 0.50 for both ±4 and ±5 track differences

**Note:** Significant penalty, unlikely to win unless other factors perfect

---

## TC-U-095-06: ±6+ Tracks (Penalty = 0.20)

**Test ID:** TC-U-095-06
**Test Type:** Unit Test
**Requirement:** REQ-AM-095
**Priority:** P0

### Given
- **Case A:** Detected = 11, Edition = 17 (difference = 6)
- **Case B:** Detected = 11, Edition = 147 (difference = 136, box set)

### When
- `calculate_track_count_penalty(11, 17)` is called (difference = 6)
- `calculate_track_count_penalty(11, 147)` is called (difference = 136)

### Then
- Both cases: Penalty = 0.20 (severe penalty)

### Verify
```rust
let penalty_6 = calculate_track_count_penalty(11, 17);
let penalty_136 = calculate_track_count_penalty(11, 147);
assert_eq!(penalty_6, 0.20);
assert_eq!(penalty_136, 0.20);
```

### Pass Criteria
- Returns 0.20 for ±6 or more track differences
- Same penalty for 6 tracks and 136 tracks (floor)

**Note:** Represents Aqualung box set scenario (147 vs 11 tracks)

---

## Implementation Notes

**Function Under Test:**
```rust
fn calculate_track_count_penalty(
    detected_count: usize,
    edition_count: usize,
) -> f64 {
    let diff = detected_count.abs_diff(edition_count);

    match diff {
        0 => 1.00,       // Exact match
        1 => 0.95,       // ±1 track
        2 => 0.85,       // ±2 tracks
        3 => 0.70,       // ±3 tracks
        4..=5 => 0.50,   // ±4-5 tracks
        _ => 0.20,       // ±6+ tracks
    }
}
```

**Location:** `wkmp-ai/src/matching/editions/scoring.rs`

---

## Test Data Summary

| Test ID | Detected | Edition | Difference | Expected Penalty |
|---------|----------|---------|------------|------------------|
| TC-U-095-01 | 11 | 11 | 0 | 1.00 |
| TC-U-095-02 | 11 | 12 | ±1 | 0.95 |
| TC-U-095-03 | 11 | 13 | ±2 | 0.85 |
| TC-U-095-04 | 11 | 14 | ±3 | 0.70 |
| TC-U-095-05 | 11 | 15-16 | ±4-5 | 0.50 |
| TC-U-095-06 | 11 | 17, 147 | ±6+ | 0.20 |

---

## Graduated Penalty Visualization

```
1.00 ┤■
     │
0.95 ┤ ■
     │
0.85 ┤  ■
     │
0.70 ┤   ■
     │
0.50 ┤    ■■
     │
0.20 ┤      ■■■■■■■■■■■■■■■■■■■■ (floor at 6+)
     │
     └──────────────────────────────────────
      0  1  2  3  4  5  6  7  8 ... 136
            Track Count Difference
```

---

## Integration with Multi-Factor Scoring

**Example: Japanese Edition vs Standard Edition**

**Standard Edition:**
- Duration: 0.95, Quality: 0.90, Name: 0.85
- Track count: Exact match (11 = 11) → penalty = 1.00
- **Score:** `[(0.95 × 0.30) + (0.90 × 0.45) + (0.85 × 0.25)] × 1.00 = 0.9025`

**Japanese Edition (±1 Track):**
- Duration: 0.95, Quality: 0.96, Name: 0.75
- Track count: ±1 (11 vs 12) → penalty = 0.95
- **Score:** `[(0.95 × 0.30) + (0.96 × 0.45) + (0.75 × 0.25)] × 0.95 = 0.859`

**Result:** Standard wins (0.9025 > 0.859), but Japanese competitive (16% difference)

**Deluxe Edition (±6 Tracks):**
- Duration: 0.60, Quality: 0.85, Name: 0.80
- Track count: ±6 (11 vs 17) → penalty = 0.20
- **Score:** `[(0.60 × 0.30) + (0.85 × 0.45) + (0.80 × 0.25)] × 0.20 = 0.132`

**Result:** Deluxe loses significantly (0.9025 vs 0.132 = 85% difference)

---

**Test File Version:** 1.0
**Last Updated:** 2025-12-28
**Estimated Implementation Time:** 1 hour (all 6 tests + function - straightforward)
