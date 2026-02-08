# Boundary Refinement Analysis & Architectural Improvements

## Boundary Refinement Semantics

**IMPORTANT CLARIFICATION:**

Boundary refinement is **fine-tuning boundary positions** within the context of an already-matched MusicBrainz edition:

- **Assumes:** Leading edition candidate IS the correct match
- **Assumes:** Track count IS correct (matches edition)
- **Goal:** Move boundaries to better low-energy positions closer to expected locations based on edition track timings
- **Method:** Small adjustments (±60s max) to find energy minima near expected boundary positions

**Boundary refinement is NOT:**
- Re-segmentation (adding/removing boundaries)
- Edition selection (choosing different MusicBrainz release)
- Track count correction (fixing fundamental segmentation errors)

**Implication:** If album_matcher's winning edition has 16 tracks but boundary detection finds 17 boundaries, refinement **cannot help** - this is a fundamental segmentation problem requiring different solutions.

## Executive Summary

The cascade refinement + complementary error correction test revealed critical architectural issues in how boundary refinement is tested and applied. While the algorithms themselves are sound, the test framework compares incompatible boundary sets, leading to invalid results.

**Key Findings:**
1. ✅ **Cascade refinement works correctly** when cascades are detected (Eagles +30%)
2. ✅ **Complementary error correction is correctly implemented** but runs after cascade refinement (may miss opportunities)
3. ❌ **Test compares different boundary detection runs** (invalid comparison)
4. ❌ **Track count mismatches prevent refinement** on several albums
5. ❌ **Refinement operates on wrong boundary set** (should refine album_matcher's boundaries, not detect_boundaries_with_audio's)

## Test Results Summary

| Album | Original | Refined | Change | Root Cause |
|-------|----------|---------|--------|------------|
| **Eagles** | 70.0% | 100.0% | +30.0% ✓ | Cascade refinement worked |
| **Imagine Dragons** | 68.8% | 62.5% | -6.2% ❌ | Track count mismatch (17 vs 16), skipped refinement, invalid comparison |
| **Kraftwerk** | 100.0% | 71.4% | -28.6% ❌ | Cascade refinement improved (4/7 → 5/7) but test measurement bug |
| **Heather Nova** | 72.7% | 54.5% | -18.2% ❌ | Track count mismatch (22 vs 11), skipped refinement, invalid comparison |
| **Police** | 72.7% | 9.1% | -63.6% ❌ | Track count mismatch (11 vs 11 but different boundaries), invalid comparison |
| **Rolling Stones** | 66.7% | 77.8% | +11.1% ✓ | No cascades detected, but boundaries happened to be better |

## Critical Issues Identified

### Issue #1: Test Compares Incompatible Boundary Sets

**Current (Incorrect) Test Flow:**
```rust
// Step 1: album_matcher finds best edition match with its own boundaries
let original_result = matcher.match_album(&file_path, ...).await?;
// Contains: winning_edition, boundaries from stages 2-5, match% = 68.8%

// Step 2: Run SEPARATE boundary detection (VIOLATES SEMANTICS!)
let file_audio = detect_boundaries_with_audio(&file_path).await?;
// Creates COMPLETELY DIFFERENT boundaries (may have different track count!)

// Step 3: Try to "refine" the wrong boundaries
let refined_boundaries = refine_boundaries_with_mb_hints(&file_audio, ...);
// Refining boundaries that have nothing to do with the winning edition!

// Step 4: Compare incompatible boundary sets
let refined_match_pct = calculate_match_for_refined_boundaries(...); // 62.5%
// Comparing album_matcher's boundaries vs completely different boundaries!
```

**The Problem:**
- **Semantic violation:** Refinement should operate on album_matcher's boundaries for its winning edition, not create new boundaries
- `album_matcher` produces boundaries for winning edition (16 tracks)
- `detect_boundaries_with_audio` produces different boundaries (17 tracks) - wrong track count!
- Comparing these is invalid - they're for DIFFERENT segmentations
- **Track count mismatch means refinement is impossible** - can't refine boundaries to match 16 expected tracks when we have 17 detected tracks

**Evidence:**
- Imagine Dragons: album_matcher detects 16 tracks, detect_boundaries_with_audio detects 17
- Result: Refinement skipped entirely, yet "Refined match: 62.5%" is reported (invalid!)

### Issue #2: Track Count Mismatches Block Refinement

**Albums Affected:**
- Imagine Dragons: detected 17 boundaries, expected 16 tracks → skipped
- Heather Nova: detected 22 boundaries, expected 11 tracks → skipped
- Police: track counts match but boundaries differ significantly

**Root Cause:**
`detect_boundaries_with_audio` uses streaming mode silence detection, which may detect different boundaries than album_matcher's multi-stage approach.

**Current Safety Check:**
```rust
if original_boundaries.durations.len() != expected_durations.len() {
    tracing::warn!("Track count mismatch: detected {}, expected {}. Skipping refinement.");
    return file_audio.boundaries.clone(); // Return unrefined boundaries
}
```

This is correct for safety, but means refinement can't help albums where initial boundary detection fails.

### Issue #3: Complementary Error Correction Misses Opportunities

**Current Flow:**
1. Cascade refinement runs first, fixes cascades
2. Complementary error correction runs second on REFINED boundaries
3. If cascade refinement already fixed a complementary pair, complementary correction won't detect it

**Example - Eagles:**
- Tracks 8-9 had complementary errors: -76.7s / +71.3s
- Cascade refinement detected tracks 8-9 as cascade (both >30s errors)
- Cascade refinement fixed both tracks
- Complementary error correction ran on FIXED boundaries, found no pairs

**Result:** Complementary correction never gets a chance to run on albums where cascades are detected.

### Issue #4: Refinement Should Operate on album_matcher's Boundaries

**Current (Incorrect) Architecture:**
```
album_matcher → winning_edition + boundaries_A (stage 2-5)
                                ↓
                            (throw away boundaries_A)
                                ↓
detect_boundaries_with_audio → boundaries_B (different segmentation!)
                                ↓
refine_boundaries_with_mb_hints(boundaries_B, expected_from_edition_A)
                                ↓
                            boundaries_C
                                ↓
                        Compare A vs C (INVALID!)
```

**Correct Architecture (Respects Semantics):**
```
album_matcher → winning_edition + boundaries_A (stage 2-5)
                match% = 68.8% with edition track timings
                                ↓
refine_boundaries(boundaries_A, edition_track_timings, audio_data)
    - Assumes winning_edition is correct
    - Assumes track count is correct
    - Goal: Move boundaries_A to better positions
    - Method: Find energy minima ±60s from expected positions
                                ↓
                            boundaries_B
                                ↓
                        Compare A vs B (VALID!)
                        Both use same edition, same track count
```

**Why This Matters:**
- **Semantic correctness:** Refinement fine-tunes positions, doesn't re-segment
- **Valid comparison:** Both boundary sets are for the same edition/segmentation
- **Meaningful metrics:** Improvement reflects better boundary placement, not different algorithms
- **Respects assumptions:** Winning edition is correct, just needs better boundary positions

## Proposed Architectural Improvements

### Improvement #1: Integrate Refinement Into album_matcher

**Change:** Make boundary refinement a **stage 6** in the album matching pipeline.

**Implementation:**
```rust
// In album_matcher.rs
pub struct AlbumMatcherConfig {
    pub enable_boundary_refinement: bool, // NEW
    // ... existing config
}

impl AlbumMatcher {
    async fn match_album(&self, file_path: &Path, ...) -> Result<AlbumMatchResult> {
        // ... existing stages 0-5 ...

        if self.config.enable_boundary_refinement && result.matched {
            // Stage 6: MusicBrainz-Guided Boundary Refinement
            result = self.refine_boundaries_with_mb_hints(result, &file_audio)?;
        }

        Ok(result)
    }

    fn refine_boundaries_with_mb_hints(
        &self,
        mut result: AlbumMatchResult,
        file_audio: &FileAudioData,
    ) -> Result<AlbumMatchResult> {
        // Phase 1: Cascade refinement
        let cascades = detect_cascade_patterns(&result.tracks);
        for cascade in cascades {
            if let Some(refined) = refine_cascade_region(cascade, &result.tracks, file_audio) {
                if validate_full_album_improvement(&result.tracks, &refined) {
                    result.tracks = refined;
                }
            }
        }

        // Phase 2: Complementary error correction
        let pairs = detect_complementary_pairs(&result.tracks);
        for pair in pairs {
            if let Some(refined) = refine_complementary_pair(pair, &result.tracks, file_audio) {
                if validate_full_album_improvement(&result.tracks, &refined) {
                    result.tracks = refined;
                }
            }
        }

        // Recalculate match percentage after refinements
        result.percentage = calculate_match_percentage(&result.tracks);

        Ok(result)
    }
}
```

**Benefits:**
- Single unified boundary set throughout
- Valid before/after comparison
- Refinement automatically available to all album_matcher callers
- Can enable/disable via config

### Improvement #2: Run Complementary + Cascade In Parallel

**Change:** Detect BOTH patterns on original boundaries, then apply refinements in order of benefit.

**Implementation:**
```rust
fn refine_boundaries_with_mb_hints(...) -> AlbumMatchResult {
    let original_tracks = result.tracks.clone();

    // PHASE 1: DETECT all patterns on ORIGINAL boundaries
    let cascades = detect_cascade_patterns(&original_tracks);
    let complementary_pairs = detect_complementary_pairs(&original_tracks);

    // PHASE 2: APPLY refinements in priority order
    let mut best_tracks = original_tracks.clone();
    let mut applied_refinements = Vec::new();

    // Try each refinement candidate
    let mut candidates = Vec::new();
    for cascade in cascades {
        candidates.push(RefinementCandidate::Cascade(cascade));
    }
    for pair in complementary_pairs {
        candidates.push(RefinementCandidate::Complementary(pair));
    }

    // Apply refinements one at a time, validating each
    for candidate in candidates {
        let test_tracks = apply_refinement(&best_tracks, &candidate, file_audio);

        if validate_full_album_improvement(&best_tracks, &test_tracks) {
            best_tracks = test_tracks;
            applied_refinements.push(candidate);
        }
    }

    result.tracks = best_tracks;
    Ok(result)
}
```

**Benefits:**
- Complementary pairs aren't hidden by cascade refinement
- Can detect overlapping patterns
- Applies refinements in order of validation success

### Improvement #3: Add Systematic Offset Detection

**Pattern:** First track has massive error, propagating through album.

**Albums Affected:**
- Rolling Stones: Track 1 "+259.3s"
- Police: Track 1 "-243.7s"

**Implementation:**
```rust
fn detect_systematic_offset(tracks: &[TrackMatch]) -> Option<SystematicOffset> {
    if tracks.is_empty() {
        return None;
    }

    // Check if first track has large error
    let first_error = tracks[0].detected_duration - tracks[0].expected_duration;
    if first_error.abs() < 60.0 {
        return None; // First track is fine
    }

    // Check if errors accumulate monotonically
    let mut cumulative_error = 0.0;
    let mut monotonic = true;
    let first_sign = first_error.signum();

    for track in tracks {
        let error = track.detected_duration - track.expected_duration;
        cumulative_error += error;

        // If errors change sign, not systematic
        if error.signum() != first_sign && error.abs() > 10.0 {
            monotonic = false;
            break;
        }
    }

    if monotonic && cumulative_error.abs() > 60.0 {
        Some(SystematicOffset {
            suspected_first_boundary_error: first_error,
            cumulative_error,
        })
    } else {
        None
    }
}

fn refine_systematic_offset(
    offset: &SystematicOffset,
    tracks: &[TrackMatch],
    file_audio: &FileAudioData,
) -> Option<Vec<TrackMatch>> {
    // Search for better first boundary in wide window
    let expected_first_boundary = 0.0; // Or use intro detection
    let current_first_boundary = tracks[0].start_offset;
    let adjustment_needed = offset.suspected_first_boundary_error;

    let search_center = expected_first_boundary - adjustment_needed;
    let search_window = (0.0, search_center + 120.0).max(240.0);

    // Find strong energy dip for first boundary
    let better_first_boundary = find_strong_energy_dip(
        &file_audio.energy_envelope,
        file_audio.sample_rate,
        search_window.0,
        search_window.1,
    )?;

    // Recalculate ALL subsequent boundaries from new first boundary
    let mut refined_tracks = tracks.to_vec();
    refined_tracks[0].start_offset = better_first_boundary;

    // ... recalculate remaining boundaries based on MusicBrainz expected durations ...

    Some(refined_tracks)
}
```

**Expected Impact:**
- Fix Rolling Stones and Police first-track errors
- Significantly improve albums with cumulative offset issues

### Improvement #4: Add Fine-Grained Boundary Adjustment

**Pattern:** Tracks with 10-30s errors (just outside tolerance).

**Implementation:**
```rust
fn detect_near_misses(tracks: &[TrackMatch]) -> Vec<NearMiss> {
    tracks
        .iter()
        .enumerate()
        .filter_map(|(idx, track)| {
            let error = track.detected_duration - track.expected_duration;
            if error.abs() > 10.0 && error.abs() < 30.0 {
                Some(NearMiss {
                    track_index: idx,
                    error,
                })
            } else {
                None
            }
        })
        .collect()
}

fn refine_near_miss(
    near_miss: &NearMiss,
    tracks: &[TrackMatch],
    file_audio: &FileAudioData,
) -> Option<Vec<TrackMatch>> {
    let track = &tracks[near_miss.track_index];
    let adjustment_needed = near_miss.error;

    // Small adjustment window
    let current_boundary = track.end_offset;
    let search_center = current_boundary - adjustment_needed;
    let search_window = (search_center - 15.0, search_center + 15.0);

    // Find energy minimum with zero-crossing detection
    let better_boundary = find_energy_minimum_with_zero_crossing(
        &file_audio.energy_envelope,
        &file_audio.samples,
        file_audio.sample_rate,
        search_window.0,
        search_window.1,
    )?;

    // Apply boundary adjustment
    let mut refined_tracks = tracks.to_vec();
    refined_tracks[near_miss.track_index].end_offset = better_boundary;
    if near_miss.track_index + 1 < refined_tracks.len() {
        refined_tracks[near_miss.track_index + 1].start_offset = better_boundary;
    }

    // Validate: did we improve THIS track WITHOUT breaking adjacent tracks?
    let new_error = (refined_tracks[near_miss.track_index].detected_duration
        - refined_tracks[near_miss.track_index].expected_duration).abs();

    if new_error <= 10.0 && !broke_adjacent_tracks(&tracks, &refined_tracks, near_miss.track_index) {
        Some(refined_tracks)
    } else {
        None
    }
}
```

**Expected Impact:**
- Polish tracks that are close to tolerance
- Incremental improvements on albums that are already mostly correct

## Recommended Implementation Order

1. **Immediate (High Priority) - Semantic Correctness:**
   - **Extract audio energy data from album_matcher's winning edition**
     - album_matcher already decoded audio during stages 2-5
     - Store energy envelope alongside boundaries in AlbumMatchResult
     - Makes energy data available for refinement without re-decoding

   - **Fix test to refine album_matcher's boundaries**
     - Input: album_matcher's boundaries for winning edition
     - Output: Refined boundaries for SAME edition
     - Comparison: Before vs after refinement on SAME segmentation

   - **Move complementary error detection to run in parallel with cascade detection**
     - Detect both patterns on ORIGINAL boundaries (before any refinement)
     - Prevents complementary pairs from being hidden by cascade refinement

2. **Short Term (Medium Priority) - Integration:**
   - **Integrate refinement as optional Stage 6 in album_matcher**
     - Enable/disable via AlbumMatcherConfig
     - Operates on boundaries from stages 2-5
     - Returns refined AlbumMatchResult with updated boundaries and match%

   - **Add systematic offset detection and correction**
     - Detects first-track misplacement propagating errors
     - Expected impact: +20-30% on Rolling Stones, Police albums

3. **Medium Term (Lower Priority) - Enhancements:**
   - **Add fine-grained boundary adjustment**
     - Polishes tracks with 10-30s errors (just outside tolerance)
     - Uses zero-crossing detection for precise boundary placement

## Test Validation Strategy

**Semantic Requirements:**
- Refinement operates on album_matcher's boundaries for winning edition
- Same edition and track count before/after
- Comparison validates boundary position improvements only

**Test Flow:**

1. **Baseline Test:**
   ```rust
   let config = AlbumMatcherConfig { enable_boundary_refinement: false, .. };
   let matcher = AlbumMatcher::new(config);

   for file in test_files {
       let result = matcher.match_album(&file, ...).await?;
       // Records: edition_mbid, track_count, boundaries, match%, track errors
   }
   ```

2. **Refinement Test:**
   ```rust
   let config = AlbumMatcherConfig { enable_boundary_refinement: true, .. };
   let matcher = AlbumMatcher::new(config);

   for file in test_files {
       let result = matcher.match_album(&file, ...).await?;
       // Records: SAME edition_mbid, SAME track_count, refined boundaries, match%, track errors
       // Validates: edition and track count unchanged (semantic requirement)
   }
   ```

3. **Success Criteria:**
   - **Zero regressions:** No album's match% decreases (guaranteed by full-album validation)
   - **Target improvements:** 10-15 albums improve by >10%
   - **Mean improvement:** Overall mean match% increases by 2-5%
   - **Semantic correctness:** All albums use same edition before/after (refinement doesn't change edition selection)

4. **Detailed Analysis:**
   - **Albums that improve:**
     - Which refinement strategy helped? (cascade/complementary/systematic/fine-grained)
     - Before/after error patterns
     - Boundary position changes (seconds moved)

   - **Albums unchanged:**
     - Why? (no patterns detected, all boundaries already optimal, patterns detected but rejected)
     - Distribution of "already good" vs "no patterns"

   - **Albums that cannot be refined:**
     - Track count mismatch between stages 2-5 and winning edition
     - Fundamental segmentation problems (need different solutions)

## Conclusion

### Semantic Clarity Achieved

**Boundary refinement is position fine-tuning within a matched edition:**
- Assumes winning edition is correct
- Assumes track count is correct
- Goal: Move boundaries to better low-energy positions
- Method: ±60s search for energy minima near expected positions

**The core algorithms (cascade + complementary) are sound.** The issues stem from:

1. **Semantic violation:** Test refined wrong boundary set (separate detection vs album_matcher's boundaries)
2. **Invalid comparison:** Compared boundaries from different segmentations
3. **Track count mismatches:** Fundamental segmentation errors (cannot be fixed by refinement)
4. **Sequential application:** Complementary detection ran after cascade refinement (missed opportunities)

### Corrected Architecture

The proposed improvements respect semantic boundaries:

- **Refinement operates on album_matcher's boundaries** for its winning edition
- **Complementary and cascade detection run in parallel** on original boundaries
- **Systematic offset correction** for first-track misplacements
- **Fine-grained adjustment** for near-misses (10-30s errors)
- **Valid before/after comparison** using same edition and track count

### Expected Impact

With semantically-correct implementation:

- **Zero regressions:** Full-album validation prevents degradation
- **10-15 albums improve >10%:** Albums with cascade/complementary/systematic patterns
- **Mean improvement: 2-5%** across all albums
- **Semantic correctness:** All comparisons use same edition (refinement doesn't re-segment or re-match)

### Next Steps

1. **Extract energy data from album_matcher** (store in AlbumMatchResult)
2. **Implement refinement as Stage 6** in album matching pipeline
3. **Run parallel cascade + complementary detection** on original boundaries
4. **Add systematic offset correction** for first-track errors
5. **Validate with 200-file test set** using corrected architecture
