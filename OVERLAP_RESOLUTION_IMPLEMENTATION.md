# Stage 6 Overlap Resolution Implementation

**Date:** 2026-01-09
**Status:** Implemented and compiled successfully
**Test Status:** Pending (blocked by chromaprint linking issue)

---

## Problem Identified

### Eagles - The Long Run Case Study

**Original Track Pattern (before Stage 6):**
```
Track 8 (Teenage Jail):   Expected 223.99s, Detected 147.27s → -76.72s error
Track 9 (The Greeks...):  Expected 138.40s, Detected 209.70s → +71.31s error
```

**User Insight:**
> "The algorithm should simply move the single boundary between tracks 8 and 9 by approximately 74 seconds."

**Root Cause:**
Sequential application strategy prevented optimal solution:
1. Both cascade (tracks 8-9) and complementary pair (tracks 8-9) detected at same location
2. Cascade runs FIRST → moves wrong boundary (7-8 instead of 8-9)
3. Creates new error pattern
4. Complementary runs SECOND → tries to fix new pattern, fails validation
5. **Simple single-boundary fix never attempted**

---

## Solution: Option 2 - Overlap Resolution

**Implementation:** When cascade and complementary patterns overlap, try BOTH approaches in parallel and pick the better result.

### Algorithm Flow

#### 1. Overlap Detection (lines 858-883)

Identify when 2-track cascade overlaps with complementary pair:

```rust
for (cascade_idx, cascade) in cascade_patterns.iter().enumerate() {
    if cascade.count == 2 {
        let cascade_start = cascade.start_track;
        let cascade_end = cascade.start_track + 1;

        for (pair_idx, pair) in complementary_pairs.iter().enumerate() {
            // Check if tracks overlap
            if pair.track_index == cascade_start ||
               pair.track_index == cascade_end ||
               pair.track_index + 1 == cascade_start ||
               pair.track_index + 1 == cascade_end {
                overlapping_patterns.push((cascade_idx, pair_idx));
            }
        }
    }
}
```

#### 2. Parallel Evaluation (lines 892-980)

For each overlapping pattern pair:

1. **Try cascade refinement** → Get refined boundaries
2. **Try complementary refinement** → Get refined boundaries
3. **Calculate match percentage for each:**
   ```rust
   match_pct = (tracks_within_tolerance / total_tracks) * 100.0
   ```
4. **Pick the better approach:**
   - If complementary has higher match % → select complementary
   - Else if cascade has >0% match → select cascade
   - Else → neither approach worked
5. **Validate the winner** with full-album improvement check
6. **Track processed patterns** to prevent re-processing

#### 3. Sequential Fallback (lines 982-1036)

Apply remaining non-overlapping patterns using original sequential logic:
- Non-overlapping cascade patterns
- Non-overlapping complementary patterns

---

## Expected Behavior for Eagles

With the new overlap resolution:

**Before (sequential):**
1. Cascade detected at tracks 8-9 → Moves boundary 7-8 → New errors
2. Complementary detected at tracks 8-9 → Tries to fix, fails validation
3. **Result:** 70.0% match, no improvement

**After (overlap resolution):**
1. Overlap detected: cascade and complementary both at tracks 8-9
2. Try cascade: moves boundary 7-8 → Calculate match %
3. Try complementary: moves boundary 8-9 by ~74s → Calculate match %
4. **Complementary should win** (better match %)
5. **Expected result:** Significantly improved match percentage

---

## Code Changes

### Files Modified

#### 1. `wkmp-ai/src/matching/album_matcher.rs` (lines 852-1036)

**Changes:**
- Added overlap detection logic (lines 858-883)
- Added parallel evaluation for overlapping patterns (lines 892-980)
- Maintained sequential processing for non-overlapping patterns (lines 982-1036)
- Added `count_tracks_within_tolerance` to imports (line 37)

**Key Implementation Details:**
```rust
// Calculate match percentages for both approaches
let cascade_match = if let Some(ref refined) = cascade_result {
    let within = count_tracks_within_tolerance(
        refined, &expected_durations, sample_rate,
        self.config.match_tolerance_secs
    );
    (within as f64 / expected_durations.len() as f64) * 100.0
} else {
    0.0
};

let complementary_match = if let Some(ref refined) = complementary_result {
    let within = count_tracks_within_tolerance(
        refined, &expected_durations, sample_rate,
        self.config.match_tolerance_secs
    );
    (within as f64 / expected_durations.len() as f64) * 100.0
} else {
    0.0
};

// Pick the better approach
let (best_approach, best_refined, best_match) =
    if complementary_match > cascade_match {
        ("complementary", complementary_result, complementary_match)
    } else if cascade_match > 0.0 {
        ("cascade", cascade_result, cascade_match)
    } else {
        ("none", None, 0.0)
    };
```

#### 2. `wkmp-ai/src/matching/stage6_helpers.rs` (line 248)

**Changes:**
- Made `count_tracks_within_tolerance()` public for use in overlap resolution

```rust
// Before:
fn count_tracks_within_tolerance(

// After:
pub fn count_tracks_within_tolerance(
```

---

## Build Status

**Library:** ✅ Compiles successfully with 113 warnings (documentation only)

```
Finished `release` profile [optimized] target(s) in 1.39s
```

**Tests:** ❌ Blocked by pre-existing chromaprint linking issue (unrelated to overlap resolution)

```
error LNK2019: unresolved external symbol __imp_chromaprint_new
error LNK1120: 8 unresolved externals
```

---

## Debug Logging

Enhanced debug output to track overlap resolution decisions:

```
Overlap detected: cascade at tracks 8-9 and complementary pair at tracks 8-9
Resolving overlap: trying both cascade (tracks 8) and complementary (tracks 8-9) approaches
Overlap resolution: complementary approach selected (match: 80.0% vs 70.0%), tracks 8-9
```

---

## Next Steps

1. **Resolve chromaprint linking issue** to enable testing
2. **Run targeted test on Eagles - The Long Run** to verify complementary approach is selected
3. **Run full 200-album test** to compare results with overlap resolution
4. **Analyze improvements** across all albums with overlapping patterns

---

## Design Rationale

### Why Parallel Evaluation?

**Problem with Sequential Application:**
- First pattern "claims" the tracks
- Second pattern operates on modified boundaries
- Optimal single-boundary fix may never be attempted

**Parallel Evaluation Advantages:**
- Both approaches start from same baseline (current boundaries)
- Direct comparison of results (match percentage)
- Always finds the better approach when patterns overlap
- Zero regressions (full-album validation required for acceptance)

### Match Percentage as Decision Criterion

Using `tracks_within_tolerance / total_tracks` as the metric ensures:
- Objective comparison (not subjective heuristics)
- Aligns with ultimate goal (maximize tracks within tolerance)
- Simple, transparent decision logic
- Works for albums of any size

### Conservative Validation

Even the "winner" must pass full-album validation:
```rust
if validate_full_album_improvement(...) {
    // Accept refinement
} else {
    debug!("Overlap resolution: both approaches failed validation");
}
```

This prevents regressions even when one approach is "better" than the other.

---

## Expected Impact

Based on Stage6_Full_Library_Analysis.md, overlap resolution should improve:

1. **Eagles - The Long Run** (70.0% match)
   - Complementary approach should correctly move boundary 8-9
   - Expected improvement: +10-20 percentage points

2. **Other Albums with Moderate Errors**
   - 22 albums in 70-89% range
   - Many likely have overlapping patterns
   - Potential improvements where cascade was suboptimal

3. **Zero Regressions Expected**
   - Full-album validation still required
   - Sequential fallback for non-overlapping patterns
   - Conservative acceptance criteria maintained

---

## Conclusion

The overlap resolution implementation addresses the design flaw identified by the user:
- ✅ Both approaches are now tried when patterns overlap
- ✅ Better approach is selected based on match percentage
- ✅ Full-album validation prevents regressions
- ✅ Sequential fallback maintains original behavior for non-overlapping patterns

**Ready for testing** once chromaprint linking issue is resolved.
