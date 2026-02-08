# Boundary Refinement Algorithm - Bidirectional Improvement Plan

**Date:** 2026-01-03
**Status:** Planning
**Problem:** Current boundary refinement only handles forward split failures, not reverse patterns

---

## Problem Analysis

### Current Algorithm Behavior

The boundary refinement algorithm (in [wkmp-ai/src/matching/stages/boundary_refinement.rs](../wkmp-ai/src/matching/stages/boundary_refinement.rs:121-123)) detects only **forward split failures**:

```rust
let is_split_failure = detected_i < expected_i * 0.2        // Track i collapsed
    && detected_i1 > expected_i1 * 1.5      // Track i+1 absorbed it
    && (error_i + error_i1).abs() < tolerance_secs * 2.0;
```

**Pattern detected:** Missed boundary → Track N collapses, Track N+1 expands

### Missing Pattern: Reverse Split Failure

**Real-world case:** The Doobie Brothers - The Captain And Me (81.8% match)

```
Track 10 "Ukiah"              : detected=466.49s, expected=184.00s (+282.49s) ✗
Track 11 "The Captain and Me" : detected=  0.01s, expected=293.00s (-292.99s) ✗
```

**Pattern NOT detected:** Late boundary → Track N expands, Track N+1 collapses

**Why it happens:**
- Forward case: Boundary detection missed entirely → next track absorbs previous
- Reverse case: Boundary detected too late → current track absorbs next

**Both indicate the same root problem:** Boundary is misplaced and needs refinement.

### Impact Assessment

From the 200-file test, **17 albums (14.3%)** had match quality <90%. Review shows several exhibit this reverse pattern:
- The Doobie Brothers "The Captain And Me" (81.8%) - tracks 10-11
- Eagles "The Long Run" (70.0%) - likely similar pattern
- Rolling Stones "Let It Bleed" (66.7%) - likely similar pattern

**Estimated improvement:** Fixing reverse patterns could improve 5-10 of these 17 albums to ≥90% match quality.

---

## Solution Design

### Approach: Bidirectional Pattern Detection

Detect **both** split failure patterns:

#### Pattern 1: Forward Split Failure (current)
```
Track i:   detected < expected × 0.2   (collapsed)
Track i+1: detected > expected × 1.5   (absorbed)
Errors cancel: |error_i + error_i+1| < tolerance × 2.0
```

#### Pattern 2: Reverse Split Failure (NEW)
```
Track i:   detected > expected × 1.5   (absorbed)
Track i+1: detected < expected × 0.2   (collapsed)
Errors cancel: |error_i + error_i+1| < tolerance × 2.0
```

### Implementation Strategy

**Option A: Unified Detection Logic**
```rust
let error_i = detected_i - expected_i;
let error_i1 = detected_i1 - expected_i1;

// Check if errors roughly cancel (both patterns)
let errors_cancel = (error_i + error_i1).abs() < tolerance_secs * 2.0;

// Pattern 1: Forward split failure (i collapsed, i+1 absorbed)
let is_forward_split = detected_i < expected_i * 0.2
    && detected_i1 > expected_i1 * 1.5
    && errors_cancel;

// Pattern 2: Reverse split failure (i absorbed, i+1 collapsed)
let is_reverse_split = detected_i > expected_i * 1.5
    && detected_i1 < expected_i1 * 0.2
    && errors_cancel;

let is_split_failure = is_forward_split || is_reverse_split;
```

**Option B: Generalized Magnitude Check**
```rust
// More flexible: check if one track is significantly wrong in each direction
let i_too_short = detected_i < expected_i * 0.2;
let i_too_long = detected_i > expected_i * 1.5;
let i1_too_short = detected_i1 < expected_i1 * 0.2;
let i1_too_long = detected_i1 > expected_i1 * 1.5;

let errors_cancel = (error_i + error_i1).abs() < tolerance_secs * 2.0;

// Either pattern: one track wrong in one direction, adjacent wrong in opposite
let is_split_failure = errors_cancel &&
    ((i_too_short && i1_too_long) || (i_too_long && i1_too_short));
```

**Recommendation:** Option B (more maintainable, clearer logic)

### Boundary Search Strategy

The search approach differs based on pattern:

**Forward pattern (i collapsed, i+1 absorbed):**
- Missing boundary is BETWEEN tracks i and i+1
- Search from: `boundary[i]` (start of track i)
- Expected position: `boundary[i] + expected_i × sample_rate`
- Search window: ±15 seconds around expected position
- **Current implementation handles this correctly**

**Reverse pattern (i absorbed, i+1 collapsed):**
- Missing boundary is BETWEEN tracks i and i+1 (same as forward!)
- Search from: `boundary[i]` (start of track i)
- Expected position: `boundary[i] + expected_i × sample_rate`
- Search window: ±15 seconds around expected position
- **Same search strategy works for both patterns**

**Key insight:** Regardless of which track absorbed which, the missing boundary location is the same: between tracks i and i+1, at the expected duration offset from track i's start.

### Code Changes Required

**File:** `wkmp-ai/src/matching/stages/boundary_refinement.rs`

**Function:** `refine_missed_boundaries()` (lines 83-187)

**Changes:**
1. **Lines 121-123:** Replace single pattern check with bidirectional check
2. **Lines 126-133:** Update debug message to indicate pattern type (forward/reverse)
3. **Lines 136-162:** Search logic remains unchanged (works for both patterns)
4. **Add unit tests:** Test cases for both patterns

---

## Test Cases

### Test 1: Forward Split Failure (existing)
```
Input:
  Expected:  [200s, 180s, 220s]
  Detected:  [ 30s, 370s, 220s]  // Track 1 collapsed, Track 2 absorbed it

Expected output:
  Refined:   [200s, 180s, 220s]  // Boundary corrected
```

### Test 2: Reverse Split Failure (NEW)
```
Input:
  Expected:  [184s, 293s]  // Doobie Brothers case
  Detected:  [466s,   1s]  // Track 1 absorbed Track 2

Expected output:
  Refined:   [184s, 293s]  // Boundary corrected
```

### Test 3: Three-Track Reverse Pattern (NEW)
```
Input:
  Expected:  [150s, 200s, 180s]
  Detected:  [150s, 379s,   1s]  // Track 2 absorbed Track 3

Expected output:
  Refined:   [150s, 200s, 180s]  // Boundary corrected
```

### Test 4: Mixed Pattern (edge case)
```
Input:
  Expected:  [200s, 180s, 220s, 190s]
  Detected:  [ 30s, 370s, 410s,   1s]
  // Track 1 collapsed (forward), Track 3 absorbed Track 4 (reverse)

Expected output:
  Refined:   [200s, 180s, 220s, 190s]  // Both boundaries corrected
```

### Test 5: No Refinement Needed
```
Input:
  Expected:  [200s, 180s, 220s]
  Detected:  [198s, 182s, 218s]  // All within tolerance

Expected output:
  Refined:   [198s, 182s, 218s]  // No changes
```

### Test 6: Errors Don't Cancel (should NOT trigger)
```
Input:
  Expected:  [200s, 180s, 220s]
  Detected:  [ 50s, 500s, 220s]  // Track 2 too long, but errors don't cancel
  // Error 1: -150s, Error 2: +320s, Sum: +170s > tolerance

Expected output:
  Refined:   [ 50s, 500s, 220s]  // No refinement (not a split failure)
```

---

## Implementation Pseudocode

```rust
// In refine_missed_boundaries() function

for i in 0..detected_durations.len().saturating_sub(1) {
    let detected_i = detected_durations[i];
    let expected_i = expected_durations.get(i).copied().unwrap_or(0.0);
    let detected_i1 = detected_durations[i + 1];
    let expected_i1 = expected_durations.get(i + 1).copied().unwrap_or(0.0);

    if expected_i <= 0.0 || expected_i1 <= 0.0 {
        continue;
    }

    let error_i = detected_i - expected_i;
    let error_i1 = detected_i1 - expected_i1;

    // Check magnitude thresholds
    let i_too_short = detected_i < expected_i * 0.2;
    let i_too_long = detected_i > expected_i * 1.5;
    let i1_too_short = detected_i1 < expected_i1 * 0.2;
    let i1_too_long = detected_i1 > expected_i1 * 1.5;

    // Check if errors cancel
    let errors_cancel = (error_i + error_i1).abs() < tolerance_secs * 2.0;

    // Detect either pattern
    let is_forward_split = i_too_short && i1_too_long && errors_cancel;
    let is_reverse_split = i_too_long && i1_too_short && errors_cancel;

    if is_forward_split || is_reverse_split {
        let pattern_type = if is_forward_split { "forward" } else { "reverse" };
        debug!(
            "Split failure detected ({}) at track {}: detected={:.2}s (expected {:.2}s), next={:.2}s (expected {:.2}s)",
            pattern_type, i + 1, detected_i, expected_i, detected_i1, expected_i1
        );

        // Search for missing boundary (same logic for both patterns)
        let search_start = refined[i];
        let search_end = refined[i + 1];
        let expected_boundary_samples = search_start + (expected_i * sample_rate) as usize;
        let window_samples = (15.0 * sample_rate) as usize;
        let window_start = expected_boundary_samples.saturating_sub(window_samples);
        let window_end = (expected_boundary_samples + window_samples).min(search_end);

        if let Some(new_boundary) = find_local_quiet_spot(
            audio_samples,
            window_start,
            window_end,
            sample_rate
        ) {
            info!(
                "Refining boundary {} ({}): moving from sample {} to {} (expected: {})",
                i + 1, pattern_type, refined[i + 1], new_boundary, expected_boundary_samples
            );
            refined[i + 1] = new_boundary;
            refined_this_iteration = true;
            any_refined = true;
        }
    }
}
```

---

## Risk Assessment

### Low Risk
- **Change scope:** Localized to one function in `boundary_refinement.rs`
- **Backward compatibility:** Existing forward pattern detection unchanged
- **Test coverage:** Comprehensive unit tests for both patterns

### Potential Issues

**Issue 1: False Positives**
- **Risk:** New reverse pattern detection might trigger on non-split-failure cases
- **Mitigation:** `errors_cancel` check (errors must sum to near-zero) prevents most false positives
- **Validation:** Test on full 200-file corpus before/after to verify no regressions

**Issue 2: Search Window Misalignment**
- **Risk:** Reverse pattern might need different search strategy
- **Analysis:** No - expected boundary location is identical for both patterns
- **Mitigation:** None needed, existing search logic works

**Issue 3: Consecutive Failures**
- **Risk:** Three+ consecutive collapsed/absorbed tracks (e.g., tracks 10-11-12)
- **Current handling:** Iterative refinement (max 3 iterations) already handles this
- **Validation:** Create test case with 3+ consecutive failures

---

## Validation Plan

### Phase 1: Unit Tests
1. Add 6 test cases (listed above) to `boundary_refinement.rs`
2. Run `cargo test -p wkmp-ai --lib boundary_refinement`
3. Verify all tests pass

### Phase 2: Real-World Validation
1. Re-run Doobie Brothers "The Captain And Me" album
2. Verify tracks 10-11 boundary is corrected
3. Expected result: 81.8% → 100% match quality

### Phase 3: Regression Testing
1. Re-run full 200-file test corpus
2. Compare match percentages before/after
3. Verify:
   - No albums decrease in match quality
   - 5-10 albums improve from <90% to ≥90%
   - Overall match success rate increases

### Phase 4: Edge Case Testing
1. Test albums with known forward split failures (verify no regression)
2. Test albums with mixed patterns (forward + reverse in same file)
3. Test albums with 3+ consecutive failures

---

## Success Criteria

**Primary:**
- Doobie Brothers "The Captain And Me" improves from 81.8% to ≥90% match
- No regressions in 200-file test corpus

**Secondary:**
- 5+ albums improve from <90% to ≥90% match quality
- Overall test corpus average match percentage increases by 1-2%

**Code Quality:**
- All unit tests pass
- Code coverage ≥80% for modified function
- Debug/info logging clearly indicates pattern type

---

## Implementation Estimate

- **Complexity:** Low-Medium (focused algorithm improvement)
- **Files modified:** 1 file (`boundary_refinement.rs`)
- **Lines changed:** ~30 lines (logic) + ~100 lines (tests)
- **Testing time:** 30 minutes (unit tests) + 2-3 hours (200-file regression test with AcousticBrainz cache)

---

## Next Steps

1. **Decision:** Approve plan for implementation?
2. **Implementation:** Modify `boundary_refinement.rs` with bidirectional detection
3. **Testing:** Add unit tests and validate on Doobie Brothers album
4. **Validation:** Re-run 200-file test corpus and analyze improvements
5. **Documentation:** Update algorithm comments to reflect bidirectional handling
