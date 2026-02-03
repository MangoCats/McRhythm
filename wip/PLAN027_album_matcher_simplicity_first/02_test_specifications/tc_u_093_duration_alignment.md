# Unit Tests: Total Duration Alignment Scoring (REQ-AM-093)

**Requirement:** REQ-AM-093 - Total Duration Alignment with Graduated Penalties
**Test Count:** 7 unit tests
**Priority:** P0 (Critical)

---

## TC-U-093-01: Duration Score <5% Difference (0.95)

**Test ID:** TC-U-093-01
**Test Type:** Unit Test
**Requirement:** REQ-AM-093
**Priority:** P0

### Given
- Detected total duration: 2,400,000ms (40 minutes)
- Edition total duration: 2,450,000ms (40.83 minutes)
- Percentage difference: 2.08% (within <5% band)

### When
- `calculate_total_duration_score(2_400_000, 2_450_000)` is called

### Then
- **Calculation:**
  ```
  diff_ms = 2,450,000 - 2,400,000 = 50,000ms
  diff_pct = (50,000 / 2,400,000) × 100 = 2.08%

  Since 2.08% < 5.0% → score = 0.95
  ```

### Verify
```rust
let score = calculate_total_duration_score(2_400_000, 2_450_000);
assert_eq!(score, 0.95);
```

### Pass Criteria
- Returns 0.95 for <5% difference

---

## TC-U-093-02: Duration Score 5-10% Difference (0.80)

**Test ID:** TC-U-093-02
**Test Type:** Unit Test
**Requirement:** REQ-AM-093
**Priority:** P0

### Given
- Detected: 2,400,000ms
- Edition: 2,580,000ms
- Percentage difference: 7.5% (within 5-10% band)

### When
- `calculate_total_duration_score(2_400_000, 2_580_000)` is called

### Then
- **Calculation:**
  ```
  diff_pct = (180,000 / 2,400,000) × 100 = 7.5%

  Since 5.0% ≤ 7.5% < 10.0% → score = 0.80
  ```

### Verify
```rust
let score = calculate_total_duration_score(2_400_000, 2_580_000);
assert_eq!(score, 0.80);
```

### Pass Criteria
- Returns 0.80 for 5-10% difference

---

## TC-U-093-03: Duration Score 10-15% Difference (0.60)

**Test ID:** TC-U-093-03
**Test Type:** Unit Test
**Requirement:** REQ-AM-093
**Priority:** P0

### Given
- Detected: 2,400,000ms
- Edition: 2,700,000ms
- Percentage difference: 12.5% (within 10-15% band)

### When
- `calculate_total_duration_score(2_400_000, 2_700_000)` is called

### Then
- **Calculation:**
  ```
  diff_pct = (300,000 / 2,400,000) × 100 = 12.5%

  Since 10.0% ≤ 12.5% < 15.0% → score = 0.60
  ```

### Verify
```rust
let score = calculate_total_duration_score(2_400_000, 2_700_000);
assert_eq!(score, 0.60);
```

### Pass Criteria
- Returns 0.60 for 10-15% difference

---

## TC-U-093-04: Duration Score 15-25% Difference (0.30)

**Test ID:** TC-U-093-04
**Test Type:** Unit Test
**Requirement:** REQ-AM-093
**Priority:** P0

### Given
- Detected: 2,400,000ms
- Edition: 2,880,000ms
- Percentage difference: 20.0% (within 15-25% band)

### When
- `calculate_total_duration_score(2_400_000, 2_880_000)` is called

### Then
- **Calculation:**
  ```
  diff_pct = (480,000 / 2,400,000) × 100 = 20.0%

  Since 15.0% ≤ 20.0% < 25.0% → score = 0.30
  ```

### Verify
```rust
let score = calculate_total_duration_score(2_400_000, 2_880_000);
assert_eq!(score, 0.30);
```

### Pass Criteria
- Returns 0.30 for 15-25% difference

---

## TC-U-093-05: Duration Score >25% Difference (0.05)

**Test ID:** TC-U-093-05
**Test Type:** Unit Test
**Requirement:** REQ-AM-093
**Priority:** P0

### Given
- Detected: 2,400,000ms (40 minutes, 11 tracks)
- Edition: 8,500,000ms (141.67 minutes, 147 tracks - box set)
- Percentage difference: 254.2% (>25% band)

### When
- `calculate_total_duration_score(2_400_000, 8_500_000)` is called

### Then
- **Calculation:**
  ```
  diff_pct = (6,100,000 / 2,400,000) × 100 = 254.2%

  Since 254.2% ≥ 25.0% → score = 0.05
  ```

### Verify
```rust
let score = calculate_total_duration_score(2_400_000, 8_500_000);
assert_eq!(score, 0.05);
```

### Pass Criteria
- Returns 0.05 for >25% difference

**Note:** This test represents Aqualung box set scenario (147 tracks vs 11 detected)

---

## TC-U-093-06: Edge Case - Zero Detected Duration

**Test ID:** TC-U-093-06
**Test Type:** Unit Test
**Requirement:** REQ-AM-093
**Priority:** P0

### Given
- Detected total duration: 0ms (invalid/corrupt file)
- Edition total duration: 2,400,000ms

### When
- `calculate_total_duration_score(0, 2_400_000)` is called

### Then
- Function returns 0.05 (very poor alignment)
- **Rationale:** Division by zero would panic; instead return worst score

### Verify
```rust
let score = calculate_total_duration_score(0, 2_400_000);
assert_eq!(score, 0.05);
```

### Pass Criteria
- Returns 0.05 without panicking

### Fail Criteria
- Panics due to division by zero

---

## TC-U-093-07: Edge Case - Zero Edition Duration

**Test ID:** TC-U-093-07
**Test Type:** Unit Test
**Requirement:** REQ-AM-093
**Priority:** P0

### Given
- Detected total duration: 2,400,000ms
- Edition total duration: 0ms (invalid MusicBrainz data)

### When
- `calculate_total_duration_score(2_400_000, 0)` is called

### Then
- Function returns 0.05 (very poor alignment)

### Verify
```rust
let score = calculate_total_duration_score(2_400_000, 0);
assert_eq!(score, 0.05);
```

### Pass Criteria
- Returns 0.05 without panicking

---

## Test Data Summary

| Test ID | Detected (ms) | Edition (ms) | Diff (%) | Expected Score |
|---------|---------------|--------------|----------|----------------|
| TC-U-093-01 | 2,400,000 | 2,450,000 | 2.08% | 0.95 |
| TC-U-093-02 | 2,400,000 | 2,580,000 | 7.5% | 0.80 |
| TC-U-093-03 | 2,400,000 | 2,700,000 | 12.5% | 0.60 |
| TC-U-093-04 | 2,400,000 | 2,880,000 | 20.0% | 0.30 |
| TC-U-093-05 | 2,400,000 | 8,500,000 | 254.2% | 0.05 |
| TC-U-093-06 | 0 | 2,400,000 | N/A | 0.05 |
| TC-U-093-07 | 2,400,000 | 0 | N/A | 0.05 |

---

## Implementation Notes

**Function Under Test:**
```rust
fn calculate_total_duration_score(
    detected_total_ms: u64,
    edition_total_ms: u64,
) -> f64 {
    // Edge case: zero duration
    if detected_total_ms == 0 || edition_total_ms == 0 {
        return 0.05;
    }

    let diff_ms = detected_total_ms.abs_diff(edition_total_ms);
    let diff_pct = (diff_ms as f64 / detected_total_ms as f64) * 100.0;

    if diff_pct < 5.0 {
        0.95
    } else if diff_pct < 10.0 {
        0.80
    } else if diff_pct < 15.0 {
        0.60
    } else if diff_pct < 25.0 {
        0.30
    } else {
        0.05
    }
}
```

**Location:** `wkmp-ai/src/matching/editions/scoring.rs`

---

**Test File Version:** 1.0
**Last Updated:** 2025-12-28
**Estimated Implementation Time:** 1-2 hours (all 7 tests + function)
