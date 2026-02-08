# Single-Track Discriminator - Design Documentation

## Overview

The Single-Track Discriminator identifies audio files that are individual tracks (not concatenated albums) before or during album matching. This prevents wasted processing and incorrect matches when single tracks are accidentally included in the training set.

**Introduced in Run 23** - Detection and logging only (no skipping).

---

## Problem Statement

Album matcher expects **concatenated album files** (all tracks joined into one file). When single-track files are processed:

1. **Runtime mismatch**: Single track (5-15 min) vs album (40-80 min) causes runtime filter to reject correct editions
2. **Wrong matches**: Algorithm matches against whatever MB releases have similar duration
3. **Wasted resources**: Full silence detection and MB queries for invalid input

**Run 22 examples:**
| ID | Runtime | File | Issue |
|----|---------|------|-------|
| A1 | 7.74 min | `04 - Fillmore East.mp3` | Track from Santana IV |
| A5 | 8.89 min | `08 - The Call of Ktulu.mp3` | Track from Ride the Lightning |
| A17 | 15.92 min | `12 - False Echoes.mp3` | Track from Banana Wind |

---

## Layered Detection Approach

### Layer 1: Filename Pattern (Pre-Decode, Zero-Cost)

Detect track number prefix in filename:
- Pattern: `^\d{1,2}\s*[-_\.]\s*` (e.g., "04 - ", "12_", "8.")
- High confidence indicator when present
- Cannot catch all cases (e.g., `SoulSacrifice.mp3`)

```rust
const TRACK_NUMBER_PATTERN: &str = r"^(\d{1,2})\s*[-_\.]\s*";
```

**Scoring:**
- Match found: +1.0 (strong indicator)
- No match: 0.0 (inconclusive)

---

### Layer 2: Directory File Count (Pre-Decode, Cheap)

Count audio files in parent directory:
- Many files (>N) suggests individual tracks
- Few files (1-2) suggests concatenated albums

```rust
const DIR_FILE_COUNT_THRESHOLD: usize = 4;  // >= 4 files = likely individual tracks
```

**Scoring:**
- `file_count >= threshold`: +0.8 (strong indicator)
- `file_count == 2-3`: +0.3 (weak indicator)
- `file_count == 1`: -0.5 (counter-indicator, suggests concatenated)

---

### Layer 3: ID3 Track Number Tag (Pre-Decode, Cheap)

Check TRCK tag for track/total format:
- "4/12" = track 4 of 12 tracks → single track
- "1/1" or missing = inconclusive

```rust
// Threshold: if total_tracks > this, file is definitely a single track
const ID3_TRACK_TOTAL_THRESHOLD: u32 = 1;
```

**Scoring:**
- `total_tracks > 1`: +1.0 (definitive)
- `track_number present, no total`: +0.4 (suggestive)
- Tag missing: 0.0 (inconclusive)

---

### Layer 4: Duration Heuristic (Pre-Decode or Post-Decode)

Very short files are unlikely to be albums:
- Can use ID3 duration tag (pre-decode) or actual decoded duration (post-decode)

```rust
const MIN_ALBUM_DURATION_MINS: f64 = 20.0;  // Albums typically > 20 minutes
const TYPICAL_TRACK_DURATION_MINS: f64 = 8.0;  // Single tracks typically < 8 minutes
```

**Scoring:**
- `duration < TYPICAL_TRACK_DURATION_MINS`: +0.7 (likely single track)
- `duration < MIN_ALBUM_DURATION_MINS`: +0.4 (possibly single track)
- `duration >= MIN_ALBUM_DURATION_MINS`: -0.3 (counter-indicator)

---

### Layer 5: Silence Gap Count (Post-Decode, Definitive)

Concatenated albums have **N-1 silence gaps** for N tracks. Single tracks have few gaps.

```rust
const MIN_EXPECTED_GAPS: usize = 3;  // Albums typically have >= 4 tracks
```

**Scoring:**
- `gap_count < MIN_EXPECTED_GAPS`: +1.0 (definitive)
- `gap_count >= MIN_EXPECTED_GAPS`: -0.5 (counter-indicator)

---

## Score Aggregation

```rust
#[derive(Debug)]
pub struct SingleTrackAnalysis {
    pub filename_score: f64,
    pub dir_count_score: f64,
    pub id3_track_score: f64,
    pub duration_score: f64,
    pub silence_gap_score: Option<f64>,  // None until post-decode

    pub total_score: f64,
    pub is_likely_single_track: bool,
    pub confidence: Confidence,
}

pub enum Confidence {
    High,      // score >= 2.0 or definitive indicator present
    Medium,    // score >= 1.0
    Low,       // score >= 0.5
    Unlikely,  // score < 0.5
}
```

**Decision threshold (configurable):**
```rust
const SINGLE_TRACK_SCORE_THRESHOLD: f64 = 1.5;  // Score >= 1.5 = likely single track
```

---

## Logging Format

### Pre-Decode Summary
```
[A17] 🔍 Single-track analysis (pre-decode):
[A17]     Filename pattern: +1.00 (matched "12 - ")
[A17]     Directory files:  +0.80 (12 audio files in dir)
[A17]     ID3 track tag:    +1.00 (track 12/12)
[A17]     Duration hint:    +0.40 (ID3 duration: 15.9 min < 20 min threshold)
[A17]     PRE-DECODE TOTAL: 3.20 (High confidence single track)
```

### Post-Decode Update
```
[A17] 🔍 Single-track analysis (post-decode):
[A17]     Silence gaps:     +1.00 (only 1 gap detected, expected >= 3)
[A17]     FINAL TOTAL:      4.20 (High confidence single track)
[A17]     ⚠️ SINGLE TRACK DETECTED - Results may be invalid
```

### Summary at End of Run
```
=== Single-Track Detection Summary ===
High confidence:   6 files (A1, A5, A9, A13, A17, A21)
Medium confidence: 2 files (A34, A67)
Low confidence:    1 file (A89)
Total flagged:     9 / 200 (4.5%)
```

---

## Constants Reference

| Constant | Default | Description |
|----------|---------|-------------|
| `TRACK_NUMBER_PATTERN` | `r"^(\d{1,2})\s*[-_\.]\s*"` | Regex for track number prefix |
| `DIR_FILE_COUNT_THRESHOLD` | 4 | Files in dir >= this = likely tracks |
| `ID3_TRACK_TOTAL_THRESHOLD` | 1 | ID3 total tracks > this = single track |
| `MIN_ALBUM_DURATION_MINS` | 20.0 | Shorter than this is suspicious |
| `TYPICAL_TRACK_DURATION_MINS` | 8.0 | Shorter than this is very suspicious |
| `MIN_EXPECTED_GAPS` | 3 | Fewer gaps than this = likely single |
| `SINGLE_TRACK_SCORE_THRESHOLD` | 1.5 | Score >= this = flag as single track |

---

## Future Enhancements

1. **Skip mode**: Once tuned, add option to skip single tracks entirely
2. **Training set cleanup**: Generate list of files to remove from training set
3. **Adaptive thresholds**: Adjust based on music genre (prog rock has fewer, longer tracks)
4. **ML classifier**: Train on labeled examples for more nuanced detection

---

## Version History

| Run | Changes |
|-----|---------|
| 23 | Initial implementation - detection and logging only |
