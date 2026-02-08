# Album Matcher 27 - Design Documentation

## Overview

`album_matcher_27.rs` enhances edition ranking to prefer match quality over track count when there's a significant match difference. Based on Run 26 (MusicBrainz API caching).

**Key innovation in Run 27:** Track count penalty that prevents compilations with extra tracks from beating standard editions with better match quality.

---

## Problem Statement

Run 26 introduced a regression where a 17-track compilation (76.5% match) was selected over a 12-track standard edition (100% match) for album A14. The more lenient "Combination A filter" allowed the compilation through, and the edition ranking logic didn't sufficiently penalize the 5 extra tracks.

**Root Cause:**
- Edition sorting used only match% and mean error
- Track count was considered during filtering/pre-sorting, but not in final winner selection
- Compilations with significantly more tracks could win with lower match quality

**Example (A14):**
| Edition | Tracks | Match % | Mean Error | Selected In |
|---------|--------|---------|------------|-------------|
| Standard Album | 12 | 100.0% | 3.23s | Run 23 ✓ |
| "Romantic Collection" | 17 | 76.5% | 6.64s | Run 26 ✗ |

The 17-track compilation has 5 extra tracks (41% more) and 23.5% worse match quality, yet was ranked higher in run 26.

---

## Solution: Adjusted Match Percentage with Track Count Penalty

### Algorithm Enhancement

Modified `find_best_edition_result_with_artist_check()` to calculate an **adjusted match percentage** that penalizes extra tracks:

```rust
adjusted_match_pct = raw_match_pct - (extra_tracks × TRACK_COUNT_PENALTY_PER_TRACK)

where:
  extra_tracks = max(0, edition_track_count - estimated_track_count)
  TRACK_COUNT_PENALTY_PER_TRACK = 4.0
```

### Rationale for Penalty Value (4.0%)

Setting penalty to 4.0% per extra track means:
- **5 extra tracks = 20% penalty**
- A compilation needs >20% better match to overcome the penalty
- Aligns with MATCH_QUALITY_THRESHOLD_PCT (20.0%)

### Sorting Logic

Editions are now sorted by:
1. **Primary:** Adjusted match% (higher is better)
   - Penalizes editions with track count > expected
   - No penalty for editions with track count ≤ expected
2. **Secondary:** Mean error (lower is better)

---

## Implementation Details

### Constants Added

```rust
// Run 27: Match quality threshold for edition ranking
// When match% differs by more than this, always prefer higher match% regardless of track count
// When match% differs by less than this, consider track count as tiebreaker
const MATCH_QUALITY_THRESHOLD_PCT: f64 = 20.0;

// Run 27: Track count penalty for edition ranking
// Each extra track beyond expected reduces effective match% by this amount
// Set to 4.0 so that 5 extra tracks = 20% penalty (needs >20% better match to overcome)
const TRACK_COUNT_PENALTY_PER_TRACK: f64 = 4.0;
```

### Function Signature Change

```rust
// OLD (Run 22/24/26):
fn find_best_edition_result_with_artist_check<'a>(
    edition_results: &'a [EditionTestResult],
    editions: &[Edition],
    source_artist: &str,
    source_album: &str,
    album_idx: usize,
) -> ...

// NEW (Run 27):
fn find_best_edition_result_with_artist_check<'a>(
    edition_results: &'a [EditionTestResult],
    editions: &[Edition],
    source_artist: &str,
    source_album: &str,
    estimated_track_count: Option<usize>,  // NEW PARAMETER
    album_idx: usize,
) -> ...
```

### Logging Enhancement

When track count penalties are applied, Run 27 logs details for the top 3 editions:

```
[A14]   Run 27: Track count penalties applied (expected: 12 tracks)
[A14]       17 tracks (5 extra) → 20.0% penalty, adjusted 76.5% → 56.5%
[A14]       15 tracks (3 extra) → 12.0% penalty, adjusted 82.1% → 70.1%
```

This helps debug and tune the penalty value across different albums.

---

## Expected Behavior Changes

### A14 (12-track album) - Primary Test Case

**Run 26 Result:**
- Selected: 17-track "Romantic Collection" (76.5% match, 6.64s error)
- Reason: Highest raw match% among tested editions

**Run 27 Expected Result:**
- Selected: 12-track standard album (100.0% match, 3.23s error)
- Reason: No extra tracks (no penalty), higher match quality
- 17-track compilation: Adjusted 76.5% - 20% = **56.5%** (loses to 100%)

### Other Albums

For albums where the best edition has the correct track count:
- **No penalty applied** (extra_tracks = 0)
- **Identical results to Run 26** (A17, A30, A53, A83, A133, A154, A174, A190)

For albums where a compilation won due to better match despite extra tracks:
- Penalty may cause standard edition to win instead
- Only happens when standard edition has >20% better match after penalty adjustment

---

## Edge Cases

### 1. No Estimated Track Count Available

If `estimated_track_count` is `None`:
- No penalty applied (falls back to raw match% sorting)
- Behavior identical to Run 26

### 2. Edition Has Fewer Tracks Than Expected

```rust
extra_tracks = max(0, edition_track_count - estimated_track_count)
```

Only editions with **more** tracks than expected are penalized. Shorter editions (e.g., "Best Of" collections) receive no penalty.

### 3. Multiple Editions With Same Adjusted Match%

Tiebreaker: Mean error (lower is better)

### 4. Estimated Track Count Is Wrong

If the estimated track count from ID3 comments is incorrect (e.g., file says "10 tracks" but is actually 12):
- May penalize correct edition incorrectly
- Mitigation: Penalty is conservative (4% per track), so incorrect estimate needs to be off by 5+ tracks to cause >20% penalty
- Future improvement: Use detected silence gap count as fallback for track count estimation

---

## Performance Impact

**Negligible.** Additional computation per edition:
- 1 integer comparison
- 1 integer subtraction
- 1 floating-point multiplication
- 1 floating-point subtraction

Total overhead: <0.1% of edition testing time.

---

## Future Enhancements

1. **Adaptive Penalty:** Adjust penalty based on music genre (prog rock albums have fewer, longer tracks)
2. **Confidence Weighting:** Reduce penalty when estimated track count confidence is low
3. **Gap Count Fallback:** Use detected silence gap count when ID3 track count unavailable
4. **Bidirectional Penalty:** Penalize editions with significantly fewer tracks (may indicate truncated album)

---

## Validation Plan

### Test Albums

Run 27 should be tested against:
- **A14** (primary regression case): Expect 12-track standard (100%) to beat 17-track compilation (76.5%)
- **A17, A30, A53, A83** (no extra tracks): Expect identical results to Run 26
- **A133, A154, A174, A190** (various track counts): Verify no unintended regressions

### Success Criteria

1. ✅ A14 selects 12-track standard edition (100% match)
2. ✅ No new failures compared to Run 26
3. ✅ At least 8/11 run 24d problem albums show identical results to Run 23
4. ✅ Penalty logging appears for albums with extra-track editions

---

## Version History

| Run | Changes |
|-----|---------|
| 27 | Track count penalty in edition ranking (prefer match quality over track count) |
| 26 | MusicBrainz API caching (3 modes: Disabled, ReadWrite, ReadOnly) |
| 25 | Combination A filter (weighted<42% OR various/album<35%) |
| 24 | Aggressive ratio filter (artist<50% OR album<50%) - caused regressions |
| 23 | Single-track discriminator (detection and logging only) |
| 22 | Artist fallback algorithm (prevents wrong-artist matches) |

---

## References

- Run 26 Analysis: See `album_matcher_output_run26.txt`
- Run 23 Baseline: See `album_matcher_output_run23.txt`
- Design Document: This file
- Implementation: `album_matcher_27.rs`
