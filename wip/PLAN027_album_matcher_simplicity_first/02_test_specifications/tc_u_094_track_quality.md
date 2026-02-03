# Unit Tests: Track Quality Graduated Scoring (REQ-AM-094)

**Requirement:** REQ-AM-094 - Track Match Quality with Graduated Scoring
**Test Count:** 6 unit tests
**Priority:** P0 (Critical)

---

## TC-U-094-01: Perfect Match (Quality = 1.0)

**Test ID:** TC-U-094-01
**Test Type:** Unit Test
**Requirement:** REQ-AM-094
**Priority:** P0

### Given
- Detected track durations: `[180.0, 210.0, 195.0]` seconds
- Edition track durations: `[180.0, 210.0, 195.0]` seconds
- Tolerance: 1.5 seconds

### When
- `calculate_track_quality_score(detected, edition, 1.5)` is called

### Then
- **Per-track quality:**
  ```
  Track 1: error = 0.0s → quality = 1.0 - (0.0 / 1.5) = 1.0
  Track 2: error = 0.0s → quality = 1.0 - (0.0 / 1.5) = 1.0
  Track 3: error = 0.0s → quality = 1.0 - (0.0 / 1.5) = 1.0
  ```
- **Average quality:**
  ```
  avg = (1.0 + 1.0 + 1.0) / 3 = 1.0
  ```

### Verify
```rust
let detected = vec![180.0, 210.0, 195.0];
let edition = vec![180.0, 210.0, 195.0];
let score = calculate_track_quality_score(&detected, &edition, 1.5);
assert_eq!(score, 1.0);
```

### Pass Criteria
- Returns 1.0 for perfect match (all errors = 0)

---

## TC-U-094-02: Linear Decay Within Tolerance

**Test ID:** TC-U-094-02
**Test Type:** Unit Test
**Requirement:** REQ-AM-094
**Priority:** P0

### Given
- Detected: `[180.0, 210.0, 195.0]` seconds
- Edition: `[180.5, 211.2, 194.3]` seconds
- Tolerance: 1.5 seconds

### When
- `calculate_track_quality_score(detected, edition, 1.5)` is called

### Then
- **Per-track quality (linear decay):**
  ```
  Track 1: error = 0.5s → quality = 1.0 - (0.5 / 1.5) = 0.6667
  Track 2: error = 1.2s → quality = 1.0 - (1.2 / 1.5) = 0.2000
  Track 3: error = 0.7s → quality = 1.0 - (0.7 / 1.5) = 0.5333
  ```
- **Average quality:**
  ```
  avg = (0.6667 + 0.2000 + 0.5333) / 3 = 0.4667
  ```

### Verify
```rust
let detected = vec![180.0, 210.0, 195.0];
let edition = vec![180.5, 211.2, 194.3];
let score = calculate_track_quality_score(&detected, &edition, 1.5);
assert_approx_eq!(score, 0.467, epsilon = 0.01);
```

### Pass Criteria
- Quality decreases linearly as error approaches tolerance
- Formula: `quality = 1.0 - (error / tolerance)` for error ≤ tolerance

---

## TC-U-094-03: Zero Quality Beyond Tolerance

**Test ID:** TC-U-094-03
**Test Type:** Unit Test
**Requirement:** REQ-AM-094
**Priority:** P0

### Given
- Detected: `[180.0, 210.0, 195.0]` seconds
- Edition: `[182.0, 215.0, 190.0]` seconds
- Tolerance: 1.5 seconds

### When
- `calculate_track_quality_score(detected, edition, 1.5)` is called

### Then
- **Per-track quality:**
  ```
  Track 1: error = 2.0s > 1.5s → quality = 0.0
  Track 2: error = 5.0s > 1.5s → quality = 0.0
  Track 3: error = 5.0s > 1.5s → quality = 0.0
  ```
- **Average quality:**
  ```
  avg = (0.0 + 0.0 + 0.0) / 3 = 0.0
  ```

### Verify
```rust
let detected = vec![180.0, 210.0, 195.0];
let edition = vec![182.0, 215.0, 190.0];
let score = calculate_track_quality_score(&detected, &edition, 1.5);
assert_eq!(score, 0.0);
```

### Pass Criteria
- Returns 0.0 when all errors exceed tolerance

### Fail Criteria
- Non-zero quality for errors beyond tolerance

---

## TC-U-094-04: Track Count Mismatch (Uses Min Length)

**Test ID:** TC-U-094-04
**Test Type:** Unit Test
**Requirement:** REQ-AM-094
**Priority:** P0

### Given
- Detected: `[180.0, 210.0, 195.0]` seconds (3 tracks)
- Edition: `[180.0, 210.5, 195.2, 220.0, 240.0]` seconds (5 tracks)
- Tolerance: 1.5 seconds

### When
- `calculate_track_quality_score(detected, edition, 1.5)` is called

### Then
- **Only first 3 tracks compared (min length = 3):**
  ```
  Track 1: error = 0.0s → quality = 1.0
  Track 2: error = 0.5s → quality = 1.0 - (0.5 / 1.5) = 0.6667
  Track 3: error = 0.2s → quality = 1.0 - (0.2 / 1.5) = 0.8667
  ```
- **Tracks 4-5 ignored** (beyond min length)
- **Average quality:**
  ```
  avg = (1.0 + 0.6667 + 0.8667) / 3 = 0.8444
  ```

### Verify
```rust
let detected = vec![180.0, 210.0, 195.0];
let edition = vec![180.0, 210.5, 195.2, 220.0, 240.0];
let score = calculate_track_quality_score(&detected, &edition, 1.5);
assert_approx_eq!(score, 0.844, epsilon = 0.01);
```

### Pass Criteria
- Uses `min(detected_count, edition_count)` for comparison
- Extra tracks beyond minimum ignored
- Quality calculated only on matched portion

### Fail Criteria
- Attempts to compare unmatched tracks
- Panics due to index out of bounds

---

## TC-U-094-05: Edge Case - Both Arrays Empty

**Test ID:** TC-U-094-05
**Test Type:** Unit Test
**Requirement:** REQ-AM-094
**Priority:** P0

### Given
- Detected: `[]` (zero tracks)
- Edition: `[]` (zero tracks)
- Tolerance: 1.5 seconds

### When
- `calculate_track_quality_score(&[], &[], 1.5)` is called

### Then
- Function returns 0.0 (no tracks to match)

### Verify
```rust
let score = calculate_track_quality_score(&[], &[], 1.5);
assert_eq!(score, 0.0);
```

### Pass Criteria
- Returns 0.0 without panicking

### Fail Criteria
- Panics due to division by zero

---

## TC-U-094-06: Edge Case - Zero Tolerance

**Test ID:** TC-U-094-06
**Test Type:** Unit Test
**Requirement:** REQ-AM-094
**Priority:** P0

### Given
- Detected: `[180.0, 210.0]` seconds
- Edition: `[180.0, 210.0]` seconds
- Tolerance: 0.0 seconds (invalid)

### When
- `calculate_track_quality_score(detected, edition, 0.0)` is called

### Then
- **Option A (Defensive):** Function returns 0.0 (invalid tolerance)
- **Option B (Assertion):** Function panics with assertion message "tolerance must be > 0"

### Verify
```rust
// Option A (Defensive):
let score = calculate_track_quality_score(&[180.0, 210.0], &[180.0, 210.0], 0.0);
assert_eq!(score, 0.0);

// Option B (Assertion):
#[should_panic(expected = "tolerance must be > 0")]
fn test_zero_tolerance_panics() {
    calculate_track_quality_score(&[180.0, 210.0], &[180.0, 210.0], 0.0);
}
```

### Pass Criteria
- Either returns 0.0 OR panics with clear assertion message

### Fail Criteria
- Division by zero without handling

**Note:** Recommend Option A (defensive) for production robustness

---

## Implementation Notes

**Function Under Test:**
```rust
fn calculate_track_quality_score(
    detected_durations: &[f64],
    edition_durations: &[f64],
    tolerance_secs: f64,
) -> f64 {
    // Edge case: empty arrays
    if detected_durations.is_empty() || edition_durations.is_empty() {
        return 0.0;
    }

    // Edge case: zero tolerance
    if tolerance_secs <= 0.0 {
        return 0.0;  // Option A: defensive
        // panic!("tolerance must be > 0");  // Option B: assertion
    }

    let track_count = detected_durations.len().min(edition_durations.len());
    let mut total_quality = 0.0;

    for i in 0..track_count {
        let error = (detected_durations[i] - edition_durations[i]).abs();
        let track_quality = if error <= tolerance_secs {
            1.0 - (error / tolerance_secs)
        } else {
            0.0
        };
        total_quality += track_quality;
    }

    total_quality / track_count as f64
}
```

**Location:** `wkmp-ai/src/matching/editions/scoring.rs`

---

## Test Data Summary

| Test ID | Detected Count | Edition Count | Match Type | Expected Score |
|---------|----------------|---------------|------------|----------------|
| TC-U-094-01 | 3 | 3 | Perfect (0.0s errors) | 1.0 |
| TC-U-094-02 | 3 | 3 | Good (0.5-1.2s errors) | 0.467 |
| TC-U-094-03 | 3 | 3 | Poor (>1.5s errors) | 0.0 |
| TC-U-094-04 | 3 | 5 | Mismatch (uses min=3) | 0.844 |
| TC-U-094-05 | 0 | 0 | Empty arrays | 0.0 |
| TC-U-094-06 | 2 | 2 | Zero tolerance | 0.0 or panic |

---

**Test File Version:** 1.0
**Last Updated:** 2025-12-28
**Estimated Implementation Time:** 2-3 hours (all 6 tests + function)
