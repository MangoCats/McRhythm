# Unit Tests: Multi-Factor Weighted Scoring (REQ-AM-092)

**Requirement:** REQ-AM-092 - Multi-Factor Weighted Scoring
**Test Count:** 5 unit tests
**Priority:** P0 (Critical)

---

## TC-U-092-01: Verify Correct Weight Application

**Test ID:** TC-U-092-01
**Test Type:** Unit Test
**Requirement:** REQ-AM-092
**Priority:** P0

### Given
- Duration score = 0.80 (80%)
- Track quality score = 0.90 (90%)
- Name similarity score = 0.70 (70%)
- Track count penalty = 1.00 (exact match)

### When
- `calculate_edition_score()` is called with these inputs

### Then
- **Expected base score:**
  ```
  base = (0.80 × 0.30) + (0.90 × 0.45) + (0.70 × 0.25)
       = 0.24 + 0.405 + 0.175
       = 0.820
  ```
- **Expected final score:**
  ```
  final = 0.820 × 1.00 = 0.820
  ```

### Verify
```rust
assert_approx_eq!(actual_score, 0.820, epsilon = 0.001);
```

### Pass Criteria
- Score calculation matches formula exactly (within 0.001 tolerance)

### Fail Criteria
- Score differs by more than 0.001

---

## TC-U-092-02: Verify Multiplicative Track Count Penalty

**Test ID:** TC-U-092-02
**Test Type:** Unit Test
**Requirement:** REQ-AM-092
**Priority:** P0

### Given
- Duration score = 0.95 (excellent)
- Track quality score = 0.85 (good)
- Name similarity score = 0.70 (acceptable)
- Track count penalty = 0.85 (±2 tracks difference)

### When
- `calculate_edition_score()` is called with these inputs

### Then
- **Expected base score:**
  ```
  base = (0.95 × 0.30) + (0.85 × 0.45) + (0.70 × 0.25)
       = 0.285 + 0.3825 + 0.175
       = 0.8425
  ```
- **Expected final score (with penalty):**
  ```
  final = 0.8425 × 0.85 = 0.716125
  ```

### Verify
```rust
assert_approx_eq!(actual_score, 0.716, epsilon = 0.001);
// Verify penalty is multiplicative (NOT additive)
assert!(actual_score < base_score);
assert_eq!(actual_score, base_score * track_count_penalty);
```

### Pass Criteria
- Score calculation includes multiplicative penalty
- Final score = base score × penalty (not base - penalty)

### Fail Criteria
- Penalty applied additively instead of multiplicatively

---

## TC-U-092-03: Edge Case - Empty Editions List

**Test ID:** TC-U-092-03
**Test Type:** Unit Test
**Requirement:** REQ-AM-092
**Priority:** P0

### Given
- `candidate_editions` = empty vector `[]`

### When
- `select_best_edition(candidate_editions)` is called

### Then
- Function returns `None`

### Verify
```rust
let result = select_best_edition(&[]);
assert!(result.is_none());
```

### Pass Criteria
- Returns `None` when editions list is empty
- No panic or error

### Fail Criteria
- Panics, crashes, or returns invalid result

---

## TC-U-092-04: Edge Case - All Scores ≤ 0.0

**Test ID:** TC-U-092-04
**Test Type:** Unit Test
**Requirement:** REQ-AM-092
**Priority:** P0

### Given
- Edition A: score = 0.0 (all factors zero)
- Edition B: score = -0.05 (negative due to extreme penalties)
- Edition C: score = 0.0

### When
- `select_best_edition(candidate_editions)` is called

### Then
- Function returns `None` (no acceptable matches)

### Verify
```rust
let editions = vec![
    EditionCandidate { id: "A", score: 0.0, ... },
    EditionCandidate { id: "B", score: -0.05, ... },
    EditionCandidate { id: "C", score: 0.0, ... },
];
let result = select_best_edition(&editions);
assert!(result.is_none());
```

### Pass Criteria
- Returns `None` when all scores ≤ 0.0
- Rejects editions with non-positive scores

### Fail Criteria
- Returns an edition despite all scores ≤ 0.0

---

## TC-U-092-05: Edge Case - Identical Scores (Tie-Breaking)

**Test ID:** TC-U-092-05
**Test Type:** Unit Test
**Requirement:** REQ-AM-092
**Priority:** P0

### Given
- Edition A: score = 0.850, name_similarity = 0.70, mbid = "zzz-123"
- Edition B: score = 0.850, name_similarity = 0.80, mbid = "aaa-456"
- Edition C: score = 0.850, name_similarity = 0.80, mbid = "mmm-789"

### When
- `select_best_edition(candidate_editions)` is called

### Then
- **Tie-breaking rules (in order):**
  1. Higher name_similarity wins (B and C beat A)
  2. Lexicographic MBID wins (B "aaa-456" beats C "mmm-789")
- **Expected winner:** Edition B

### Verify
```rust
let editions = vec![
    EditionCandidate { score: 0.850, name_sim: 0.70, mbid: "zzz-123" },
    EditionCandidate { score: 0.850, name_sim: 0.80, mbid: "aaa-456" },
    EditionCandidate { score: 0.850, name_sim: 0.80, mbid: "mmm-789" },
];
let result = select_best_edition(&editions).unwrap();
assert_eq!(result.mbid, "aaa-456");
```

### Pass Criteria
- Tie-breaking uses name_similarity first
- Lexicographic MBID used as final tiebreaker
- Consistent deterministic selection

### Fail Criteria
- Non-deterministic selection (returns different editions on repeated calls)
- Incorrect tie-breaking order

---

## Test Data Requirements

**Synthetic Edition Candidates:**
- Perfect scores (1.0 in all factors)
- Mixed scores (varying duration, quality, name)
- Zero scores (all factors = 0)
- Negative scores (extreme penalties)
- Identical scores (for tie-breaking tests)

**Expected Values:**
- Pre-calculated score values for verification
- Tolerance: ±0.001 for floating-point comparisons

---

## Implementation Notes

**Function Under Test:**
```rust
fn calculate_edition_score(
    duration_score: f64,
    quality_score: f64,
    name_score: f64,
    track_count_penalty: f64,
) -> f64 {
    let base = (duration_score * 0.30)
             + (quality_score * 0.45)
             + (name_score * 0.25);
    base * track_count_penalty
}

fn select_best_edition(
    candidates: &[EditionCandidate],
) -> Option<EditionCandidate> {
    if candidates.is_empty() {
        return None;
    }

    let valid_candidates: Vec<_> = candidates.iter()
        .filter(|c| c.score > 0.0)
        .collect();

    if valid_candidates.is_empty() {
        return None;
    }

    // Sort by score descending, then name_similarity descending, then MBID ascending
    valid_candidates.sort_by(|a, b| {
        match b.score.partial_cmp(&a.score) {
            Some(Ordering::Equal) => {
                match b.name_similarity.partial_cmp(&a.name_similarity) {
                    Some(Ordering::Equal) => a.mbid.cmp(&b.mbid),
                    other => other.unwrap(),
                }
            }
            other => other.unwrap(),
        }
    });

    Some(valid_candidates[0].clone())
}
```

**Location:** `wkmp-ai/src/matching/editions/scoring.rs` (new module)

---

**Test File Version:** 1.0
**Last Updated:** 2025-12-28
**Estimated Implementation Time:** 2-3 hours (all 5 tests + scoring module)
