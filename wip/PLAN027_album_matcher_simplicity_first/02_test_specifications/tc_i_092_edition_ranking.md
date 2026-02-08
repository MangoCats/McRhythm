# Integration Test: Edition Ranking (REQ-AM-092)

**Requirement:** REQ-AM-092 - Multi-Factor Weighted Scoring
**Test Count:** 1 integration test
**Priority:** P0 (Critical)

---

## TC-I-092-01: Edition Ranking with Multiple Candidates

**Test ID:** TC-I-092-01
**Test Type:** Integration Test
**Requirement:** REQ-AM-092
**Priority:** P0
**Scope:** Complete edition selection pipeline with all scoring factors

### Setup
Create 5 candidate editions with realistic score variations:

**Edition A: Standard (Ideal Match)**
- Total duration: 2,400,000ms (detected: 2,410,000ms → <5% diff)
- Track count: 11 (detected: 11 → exact match)
- Track durations: Near-perfect alignment (avg error 0.5s)
- Name similarity: 0.85

**Edition B: Deluxe (Extra Tracks)**
- Total duration: 3,200,000ms (detected: 2,410,000ms → >25% diff)
- Track count: 17 (detected: 11 → ±6 tracks)
- Track durations: Good alignment on first 11 tracks (avg error 0.8s)
- Name similarity: 0.80

**Edition C: Japanese Edition (±1 Track)**
- Total duration: 2,450,000ms (detected: 2,410,000ms → <5% diff)
- Track count: 12 (detected: 11 → ±1 track)
- Track durations: Excellent alignment on first 11 tracks (avg error 0.3s)
- Name similarity: 0.75

**Edition D: Remaster (Good Duration, Poor Quality)**
- Total duration: 2,420,000ms (detected: 2,410,000ms → <5% diff)
- Track count: 11 (detected: 11 → exact match)
- Track durations: Poor alignment (avg error 2.5s)
- Name similarity: 0.90

**Edition E: Box Set (Many Extra Tracks)**
- Total duration: 8,500,000ms (detected: 2,410,000ms → >25% diff)
- Track count: 147 (detected: 11 → ±136 tracks)
- Track durations: Unknown alignment (outside tolerance)
- Name similarity: 0.60

### When
- Multi-factor scoring algorithm evaluates all 5 editions
- Each edition scored using REQ-AM-092 formula:
  ```
  score = [(duration × 0.30) + (quality × 0.45) + (name × 0.25)] × track_count_penalty
  ```

### Then

**Expected Score Calculations:**

**Edition A (Standard):**
```
duration_score = 0.95 (detected 2,410,000 vs edition 2,400,000 → <5%)
quality_score = 0.90 (avg error 0.5s, tolerance 1.5s → 1.0 - 0.5/1.5)
name_score = 0.85
track_count_penalty = 1.00 (exact match)

base = (0.95 × 0.30) + (0.90 × 0.45) + (0.85 × 0.25)
     = 0.285 + 0.405 + 0.2125
     = 0.9025

final = 0.9025 × 1.00 = 0.9025
```

**Edition C (Japanese):**
```
duration_score = 0.95 (<5%)
quality_score = 0.96 (avg error 0.3s → 1.0 - 0.3/1.5)
name_score = 0.75
track_count_penalty = 0.95 (±1 track)

base = (0.95 × 0.30) + (0.96 × 0.45) + (0.75 × 0.25)
     = 0.285 + 0.432 + 0.1875
     = 0.9045

final = 0.9045 × 0.95 = 0.859
```

**Edition D (Remaster):**
```
duration_score = 0.95 (<5%)
quality_score = 0.40 (avg error 2.5s → tolerance exceeded)
name_score = 0.90
track_count_penalty = 1.00

base = (0.95 × 0.30) + (0.40 × 0.45) + (0.90 × 0.25)
     = 0.285 + 0.18 + 0.225
     = 0.69

final = 0.69 × 1.00 = 0.69
```

**Edition B (Deluxe):**
```
duration_score = 0.05 (>25%)
quality_score = 0.85 (good alignment on matched tracks)
name_score = 0.80
track_count_penalty = 0.20 (±6 tracks)

base = (0.05 × 0.30) + (0.85 × 0.45) + (0.80 × 0.25)
     = 0.015 + 0.3825 + 0.20
     = 0.5975

final = 0.5975 × 0.20 = 0.1195
```

**Edition E (Box Set):**
```
duration_score = 0.05 (>25%)
quality_score = 0.10 (poor alignment)
name_score = 0.60
track_count_penalty = 0.20 (±136 tracks)

base = (0.05 × 0.30) + (0.10 × 0.45) + (0.60 × 0.25)
     = 0.015 + 0.045 + 0.15
     = 0.21

final = 0.21 × 0.20 = 0.042
```

**Expected Ranking (Descending by Score):**
1. Edition A (Standard): 0.9025 ✅ **Winner**
2. Edition C (Japanese): 0.859
3. Edition D (Remaster): 0.69
4. Edition B (Deluxe): 0.1195
5. Edition E (Box Set): 0.042

### Verify
```rust
let editions = vec![edition_a, edition_b, edition_c, edition_d, edition_e];
let ranked = rank_editions(&editions, &detected_tracks);

// Verify correct ranking
assert_eq!(ranked[0].id, "edition-a");  // Standard wins
assert_eq!(ranked[1].id, "edition-c");  // Japanese second
assert_eq!(ranked[2].id, "edition-d");  // Remaster third

// Verify score values
assert_approx_eq!(ranked[0].score, 0.9025, epsilon = 0.01);
assert_approx_eq!(ranked[1].score, 0.859, epsilon = 0.01);

// Verify score ordering
assert!(ranked[0].score > ranked[1].score);
assert!(ranked[1].score > ranked[2].score);
assert!(ranked[2].score > ranked[3].score);
assert!(ranked[3].score > ranked[4].score);

// Verify standard edition preferred over box set
assert!(ranked[0].score > ranked[4].score * 2.0);
```

### Pass Criteria
- Standard edition (A) ranks first (highest score)
- Japanese edition (C) ranks second (±1 track penalty acceptable)
- Deluxe (B) and Box Set (E) rank low (large duration/count mismatches)
- Score ordering is strictly descending

### Fail Criteria
- Box set or deluxe edition ranks first (wrong edition selected)
- Score ordering not strictly descending
- Remaster with poor quality scores higher than Japanese with ±1 track

---

## Test Data

**Mock MusicBrainz Editions:**
```rust
struct EditionTestData {
    id: String,
    title: String,
    total_duration_ms: u64,
    track_count: usize,
    track_durations_secs: Vec<f64>,
}
```

**Detected Album Data:**
```rust
struct DetectedAlbum {
    total_duration_ms: u64,
    track_count: usize,
    track_durations_secs: Vec<f64>,
}
```

**Test Dataset Location:** `wkmp-ai/tests/data/edition_ranking/`

---

## Integration Points Tested

1. **Duration Alignment (REQ-AM-093):** All 5 penalty bands exercised
2. **Track Quality (REQ-AM-094):** Various alignment qualities tested
3. **Track Count Tolerance (REQ-AM-095):** Exact, ±1, ±6, ±136 tested
4. **Multi-Factor Scoring (REQ-AM-092):** Weight application and penalty multiplication

---

## Success Criteria

**Primary:**
- Standard edition (best match on all factors) ranks first
- Japanese edition (±1 track, excellent quality) ranks second
- Box set (extreme mismatch) ranks last

**Secondary:**
- All score values within 10% of calculated expected values
- Ranking order matches predicted order based on factor analysis

**Quality Metric:**
- Test represents real-world Aqualung scenario (11-track standard vs 147-track box)

---

**Test File Version:** 1.0
**Last Updated:** 2025-12-28
**Estimated Implementation Time:** 3-4 hours (integration + test data)
