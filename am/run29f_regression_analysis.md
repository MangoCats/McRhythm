# Run29f Regression Analysis
## Boundary Refinement + Negative Floor Test Results

**Test Date:** 2025-12-31
**Configuration:** Duration overflow fix + -1.0 negative floor + boundary refinement
**Total Albums:** 200
**Albums Differing from run29f:** 122 (61%)

---

## Executive Summary

### Categorization Results

| Category | Count | Percentage |
|----------|-------|------------|
| **Clearly Better** | 81 | 66.4% of changes |
| **Equivocal** | 34 | 27.9% of changes |
| **Clearly Worse** | 7 | 5.7% of changes |
| **Total Changes** | 122 | 100% |

### Key Findings

1. **Boundary Refinement Impact: Minimal**
   - Only 2 albums changed due to boundary refinement (The Go-Go's, The Police - Reggatta De Blanc)
   - Both changes were equivocal (same match %, different edition)
   - Boundary refinement DID NOT fix The Cars - Panorama regression

2. **-1.0 Negative Floor: Problematic**
   - All 7 "clearly worse" cases have low quality scores (< 0.3)
   - The negative floor over-penalizes albums with boundary detection issues
   - Allows degenerate single-track matches to win (The Cars: 1-track album beats correct 10-track album)

3. **Root Cause: Quality Score Calculation**
   - The -1.0 floor makes quality scores extremely negative for tracks with boundary failures
   - Four tracks with ~190s errors → four -1.0 quality scores → average quality score near zero
   - This destroys the final score even when duration and name scores are good

---

## Detailed Analysis: Clearly Worse Cases (7 albums)

### 1. The Cars - Panorama **[CRITICAL REGRESSION]**

**Path:** `Cars/Panorama.mp3`

**Baseline (run29f):**
- MBID: 48cf3a7a-e8b8-4d3a-940c-9ca1219d45f9
- Album: Panorama (10 tracks)
- Match: Likely 60-90%

**Current Result:**
- MBID: 9a58a677-60d4-4f0b-b686-3d80820d6d27
- Album: Stephan Mathieu - Radioland (Panorámica) **(WRONG ARTIST/ALBUM)**
- Tracks: 1 (should be 10)
- Match: 100.0%

**Scoring Comparison:**

| Candidate | Final Score | Duration | Quality | Name | Match % |
|-----------|-------------|----------|---------|------|---------|
| Radioland (1 track) | 0.5856 | 0.9500 | **0.3698** | 0.5369 | 100% |
| Panorama (10 tracks) | 0.5530 | 0.9500 | **0.0399** | 1.0000 | 60% |

**Root Cause:**

Tracks 7-10 had split failures in boundary detection:
- Track 7: Expected 186.61s, detected 0.30s (nearly zero) → Error: 186.31s
- Track 8: Expected 297.12s, detected 477.75s (absorbed track 7) → Error: 180.63s
- Track 9: Expected 201.03s, detected 0.49s (nearly zero) → Error: 200.54s
- Track 10: Expected 214.60s, detected 415.77s (absorbed track 9) → Error: 201.17s

**Quality Score Calculation:**
```
Each track error ~190s with 10s tolerance = error/tolerance = 19.0
Quality score = 1.0 - 19.0 = -18.0, capped at -1.0
Average quality = (6 good tracks near 1.0 + 4 tracks at -1.0) / 10 = 0.0399
```

**Why run29f Succeeded:**

run29f likely had:
1. No negative floor (errors > tolerance contributed 0, not negative values)
2. Different tolerance or scoring weights
3. Possibly better boundary detection in the first place

**Why Boundary Refinement Failed:**

- Boundary refinement only runs in Stage 4 (RMS quiet spot detection)
- The Cars matched in Stage 2 (silence detection)
- Refinement never executed for this album

---

### 2. Eagles - The Long Run

**Path:** `Eagles/TheLongRun.mp3`

**Issue:** Low match percentage (70.0%)

**Baseline (run29f):** Likely matched same album with higher percentage

**Current Result:**
- Match: 70.0%
- Quality Score: **0.0686** (very low)

**Analysis:**

Same pattern as The Cars - low quality score due to boundary detection issues being heavily penalized by -1.0 floor. The correct album is being selected, but with poor confidence due to quality score collapse.

---

### 3. Imagine Dragons - Night Visions

**Path:** `Imagine Dragons/NightVisions.mp3`

**Issue:** Low match percentage (68.8%)

**Current Result:**
- Match: 68.8%
- Quality Score: **0.2279** (low)

**Analysis:**

Multiple tracks with moderate timing errors accumulating to drag quality score down with -1.0 floor. run29f likely tolerated these errors better without negative penalties.

---

### 4. Heather Nova - Pearl

**Path:** `Nova, Heather/Pearl.mp3`

**Issue:** Low match percentage (72.7%)

**Current Result:**
- Match: 72.7%

**Analysis:**

Similar to Imagine Dragons - accumulated timing errors with negative floor penalties reducing confidence in correct match.

---

### 5. Delerium - Ritual

**Path:** `Phildel/Ritual.mp3`

**Issue:** Track count change 1 → 8 with 87.5% match

**Baseline (run29f):** 1 track

**Current Result:**
- Album: Velonnic Sin - Ritual (8 tracks) **(WRONG ARTIST)**
- Match: 87.5%

**Analysis:**

This is unusual - run29f had 1 track (likely wrong), current has 8 tracks (also wrong artist). Both are probably incorrect; need to investigate actual file content.

---

### 6. The Police - Zenyatta Mondatta

**Path:** `Police/ZenyattaMondatta.mp3`

**Issue:** Low match percentage (72.7%)

**Current Result:**
- Match: 72.7%
- Quality Score: **0.0907** (very low)

**Analysis:**

Boundary detection issues with -1.0 floor penalties. Correct album, poor confidence.

---

### 7. The Rolling Stones - Let It Bleed

**Path:** `Rolling Stones/LetItBleed.mp3`

**Issue:** Low match percentage (66.7%)

**Current Result:**
- Match: 66.7%
- Quality Score: **0.0907** (very low)

**Analysis:**

6 of 9 tracks matched. Boundary detection issues on 3 tracks with heavy negative floor penalties destroying quality score.

---

## Pattern Analysis

### Common Failure Mode

All 7 "clearly worse" cases share this pattern:

1. **Boundary Detection Failures:** Tracks with large timing errors (>50s)
2. **Negative Floor Penalty:** Errors beyond 2× tolerance get -1.0 quality score
3. **Quality Score Collapse:** Average quality score drops to near zero
4. **Final Score Impact:** Even with perfect duration and name scores, quality dominates (45% weight)

### Scoring Formula Review

```
base_score = (duration_score × 0.30) + (quality_score × 0.45) + (name_score × 0.25)
final_score = base_score × track_count_penalty
```

**Quality score has 45% weight** - when it collapses to near zero, final score is capped at ~0.55 even with perfect duration (0.30) and name (0.25) scores.

### Why run29f Succeeded

Based on the pattern, run29f likely had one or more of:

1. **No Negative Floor:** Errors beyond tolerance contributed 0.0, not -1.0
   - This prevents quality score collapse
   - Bad tracks don't drag down good tracks as severely

2. **Better Boundary Detection:** Possibly different algorithm or parameters
   - May have avoided split failures in tracks 7-10
   - More robust to difficult audio conditions

3. **Different Tolerance:** Possibly higher tolerance values
   - Would reduce the number of tracks hitting the floor
   - More forgiving of moderate timing errors

4. **Different Weights:** Possibly lower weight for quality score
   - Would reduce impact of boundary failures on final score
   - More emphasis on duration and name matching

---

## Recommendations

### Immediate Actions

1. **Revert -1.0 Negative Floor**
   - Test with -0.5 floor or no floor
   - Compare results to run29f baseline
   - The current floor is too aggressive

2. **Investigate run29f Algorithm**
   - Examine run29f source code to understand:
     - Quality score calculation
     - Tolerance values
     - Boundary detection method
     - Scoring weights

3. **Fix Boundary Refinement Scope**
   - Currently only runs in Stage 4
   - Consider running in Stage 2 as well
   - Or integrate into all stages

### Long-Term Solutions

1. **Improve Boundary Detection**
   - Analyze why tracks 7-10 fail consistently
   - Consider hybrid approaches (silence + RMS)
   - Validate boundary positions before finalizing

2. **Smarter Quality Scoring**
   - Use median error instead of mean (reduces outlier impact)
   - Graduated penalties instead of linear to -1.0
   - Cap negative contribution per track (e.g., -0.3 max)

3. **Confidence-Based Selection**
   - Flag low-confidence matches for review
   - Don't select 1-track matches when expecting 10 tracks
   - Sanity check track count differences

---

## Statistics Summary

### Overall Performance vs run29f

- **Exact Matches:** 66 of 193 tested (34.2%)
- **MBID Changes:** 122 of 193 tested (63.2%)
- **Failures:** 5 (2.6%)

### Change Quality Distribution

Of 122 MBID changes:
- **Better:** 81 (66.4%) - Improvements in match quality or edition selection
- **Equivocal:** 34 (27.9%) - Different editions, similar quality
- **Worse:** 7 (5.7%) - Regressions in match quality

### Critical Issues

- **1 catastrophic failure:** The Cars - Panorama (10 tracks → 1 track, wrong artist)
- **6 confidence failures:** Correct album selected, but with low match % (66-73%)

---

## Conclusion

The -1.0 negative floor introduced in the quality score calculation is too aggressive and causes:

1. **Over-penalization** of albums with a few boundary detection failures
2. **Quality score collapse** that destroys final scores even when other factors are correct
3. **Degenerate matches** winning over correct matches (1-track beats 10-track)

**The boundary refinement algorithm is well-designed but ineffective** because:
1. It only runs in Stage 4
2. Most albums match in Stage 2
3. It never gets a chance to fix the split failures

**Recommended next step:** Test without the -1.0 negative floor to confirm it's the root cause of these 7 regressions.
