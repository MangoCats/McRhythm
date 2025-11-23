# Album Matcher 22 - Design Documentation

## Overview

`album_matcher_22.rs` is an experimental album identification tool that matches concatenated audio files to MusicBrainz releases using silence detection and track duration comparison.

**Key innovation in Run 22:** Artist Fallback Algorithm that prevents wrong-artist matches from winning when a correct-artist match exists with acceptable time-fit.

---

## Algorithm Pipeline

### Phase 0: Metadata Reconciliation
- Extract artist/album from ID3 tags and filename
- Resolve conflicts between sources
- Generate search variants (e.g., "The Beatles" → "Beatles")

### Phase 1: MusicBrainz Search
- Query MB API with multiple search strategies
- Collect up to 150 unique releases
- Early NDR (Name Distance Rank) filtering removes poor matches

### Phase 2: Edition Grouping
- Group releases by track count + duration signature
- Create "editions" representing unique track layouts
- Rank editions by name similarity to source metadata

### Phase 3: Silence Detection & Track Segmentation
- Single-pass RMS analysis with 180 parameter combinations
- Cache silence boundaries for reuse across editions
- Detect track boundaries using quiet spots

### Phase 4: Edition Testing (Parallel)
- Test each edition against detected track boundaries
- Four matching stages with increasing sophistication:
  1. Initial detection (default parameters)
  2. Parameter optimization (grid search)
  3. Assembly (combine best parameters)
  4. Guided quiet spots (use MB durations as hints)
- Track match% and mean error for each edition

### Phase 5: Winner Selection with Artist Fallback (NEW in Run 22)
- Select best edition by time-fit (match% primary, mean error secondary)
- **If winner has artist mismatch, evaluate runner-ups**
- Promote runner-up if artist fit is significantly better and time-fit is acceptable

---

## Run 22: Artist Fallback Algorithm

### Problem Statement

Previous runs selected the edition with best time-fit (highest match%, lowest mean error) without considering artist similarity. This caused wrong-artist matches to win when:
- A different artist's album had similar track layout
- Album titles were generic (e.g., "Anthology", "Live at...")

**Known failures from Run 21c:**
| Album | Source Artist | Wrong Winner | Winner Sim | Correct Artist |
|-------|---------------|--------------|------------|----------------|
| A38 | Steve Howe | Dando Shaft | 41.3% | Steve Howe |
| A48 | Hooverphonic | Channel Zero | 47.8% | Hooverphonic |
| A30 | Carlos Santana | Santana | ~70% | Carlos Santana |
| A54 | John Mayall | John Lee Hooker | ~52% | John Mayall |

### Solution: Runner-Up Evaluation

When the winner has an artist mismatch (similarity < 50%), evaluate runner-ups to find a better match.

#### Algorithm Steps

1. **Sort editions by time-fit** (match% desc, mean error asc)
2. **Check winner's artist similarity** using Jaro-Winkler + substring matching
3. **If winner passes threshold (≥50%)**, use winner (no fallback needed)
4. **If winner fails threshold**, evaluate top 25% of editions (minimum 3)
5. **For each runner-up**, check two conditions:
   - **Artist fit is "significantly better"**
   - **Time fit is "not significantly worse"**
6. **First qualifying runner-up becomes new winner**

#### "Significantly Better" Artist Fit

A runner-up has significantly better artist fit if ALL of:
```
runner_sim >= 0.60                                    (minimum threshold)
AND
(runner_sim > winner_sim + 0.20  OR  runner_sim > winner_sim × 1.4)  (meaningful improvement)
```

This combined delta/ratio approach handles both:
- **Low winner similarity** (ratio catches it): 41% → 58% qualifies via ratio
- **High winner similarity** (delta catches it): 70% → 91% qualifies via delta

#### "Not Significantly Worse" Time Fit

Time-fit delta formula:
```
time_fit_delta = 10 × (runner_pct/winner_pct - 1) + 1 × (1 - runner_error/winner_error)
```

- **Match% weighted 10×** more than mean error
- **Positive** = runner-up is better
- **Negative** = runner-up is worse
- **Threshold: ≥ -0.5** means "not significantly worse"

**Example calculation:**
- Winner: 98.3% match, 3.4s mean error
- Runner: 96.6% match, 3.2s mean error
- Match% term: 10 × (0.966/0.983 - 1) = 10 × (-0.0173) = **-0.173**
- Error term: 1 × (1 - 3.2/3.4) = 1 × (0.059) = **+0.059**
- Total: -0.173 + 0.059 = **-0.114** → acceptable (≥ -0.5)

---

## Constants

```rust
// Artist similarity threshold for acceptable match
const ARTIST_MISMATCH_THRESHOLD: f64 = 0.5;

// Minimum similarity for runner-up to be considered
const ARTIST_FALLBACK_MIN_SIMILARITY: f64 = 0.60;

// Absolute improvement required (delta approach)
const ARTIST_FALLBACK_DELTA: f64 = 0.20;

// Relative improvement required (ratio approach)
const ARTIST_FALLBACK_RATIO: f64 = 1.4;

// Time-fit delta threshold
const TIME_FIT_DELTA_THRESHOLD: f64 = -0.5;

// Percentage of editions to evaluate as runner-ups
const ARTIST_FALLBACK_TOP_PCT: f64 = 0.25;

// Minimum number of runner-ups to evaluate
const ARTIST_FALLBACK_MIN_CANDIDATES: usize = 3;
```

---

## Expected Behavior for Known Failures

| Case | Winner (sim) | Correct Runner (sim) | Artist Better? | Time Acceptable? | Result |
|------|--------------|----------------------|----------------|------------------|--------|
| A38 | Dando Shaft (41%) | Steve Howe (~100%) | ✓ (100>61, 100>58) | ✓ (if within -0.5) | Promote |
| A48 | Channel Zero (48%) | Hooverphonic (~100%) | ✓ (100>68, 100>67) | ✓ (if within -0.5) | Promote |
| A30 | Santana (70%) | Carlos Santana (~100%) | ✓ (100>90, 100>98) | ✓ (if within -0.5) | Promote |
| A54 | John Lee Hooker (52%) | John Mayall (~100%) | ✓ (100>72, 100>73) | ✓ (if within -0.5) | Promote |

---

## Logging

The algorithm produces informative logs:

```
[A38]   🔍 Artist mismatch detected ('Steve Howe'→'Dando Shaft', sim=41.3%), evaluating runner-ups...
[A38]       Evaluating top 3 of 12 editions
[A38]   🎯 Artist fallback: Promoting runner-up #2 over winner
[A38]       Winner: 'Dando Shaft' (sim=41.3%, pct=82.9%, err=4.10s)
[A38]       Runner: 'Steve Howe' (sim=100.0%, pct=78.5%, err=4.50s)
[A38]       Time-fit delta: -0.182 (threshold: -0.5)
```

---

## Status Values

| Status | Meaning |
|--------|---------|
| `Success` | Artist matched, normal selection |
| `Success (Artist Fallback Used)` | Runner-up promoted due to better artist fit |
| `Artist Mismatch - Review Required` | No suitable runner-up, winner has artist mismatch but high match% |
| `Artist Mismatch - Likely Incorrect` | No suitable runner-up, winner has artist mismatch and low match% |

---

## Future Improvements

1. **Tune thresholds** based on larger validation set
2. **Consider album title similarity** in fallback decision
3. **Weight early tracks more** (they're more reliable for identification)
4. **Adaptive candidate count** based on edition diversity
