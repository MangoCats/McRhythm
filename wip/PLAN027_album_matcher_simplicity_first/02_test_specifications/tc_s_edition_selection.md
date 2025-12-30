# System Tests: End-to-End Edition Selection

**Requirements:** REQ-AM-092, REQ-AM-093, REQ-AM-094, REQ-AM-095, REQ-AM-096
**Test Count:** 3 system tests
**Priority:** P0 (Critical)

---

## TC-S-ES-01: Aqualung - Prefer 11-Track Standard Over 147-Track Box Set

**Test ID:** TC-S-ES-01
**Test Type:** System Test (End-to-End)
**Requirements:** REQ-AM-092, REQ-AM-093, REQ-AM-094, REQ-AM-095
**Priority:** P0
**Environment:** Real Aqualung album file + Live MusicBrainz data

### Scenario
User imports Aqualung album file containing 11 tracks (standard US release). MusicBrainz search returns multiple editions including:
- 11-track standard edition (US/UK releases)
- 147-track box set "25th Anniversary Special Edition"
- 12-track Japanese edition (bonus track)
- 17-track deluxe edition

**Current Problem:** Run 27-29f selects 147-track box set (99.3% match on first 11 tracks)

### Setup
**Input File:**
- Path: `test_data/aqualung/Jethro Tull - Aqualung.flac`
- Detected total duration: ~2,400,000ms (40 minutes)
- Detected tracks: 11 (via silence detection)
- Track durations: Standard edition alignment

**MusicBrainz Editions (Live API):**
- Standard (11 tracks, 2,400,000ms): Excellent duration/quality/count match
- Box Set (147 tracks, 8,500,000ms): Poor duration match, severe count mismatch
- Japanese (12 tracks, 2,450,000ms): Good duration, ±1 track
- Deluxe (17 tracks, 3,200,000ms): Poor duration, ±6 tracks

### When
- Complete album matching pipeline executes:
  1. **Stage 0:** Validates file (>60s, decodable, MB candidates exist)
  2. **REQ-AM-096:** Multi-strategy search finds all editions
  3. **REQ-AM-092-095:** Edition selection scores all candidates
  4. **Stages 1-5:** Match album against top-ranked edition

### Then

**Expected Edition Scores (Calculated):**

**Standard Edition (11 tracks, 2,400,000ms):**
```
Duration: <5% diff → 0.95
Quality: Excellent alignment → 0.90
Name: "Aqualung" exact match → 0.95
Track Count: 11 = 11 → penalty 1.00

Score = [(0.95 × 0.30) + (0.90 × 0.45) + (0.95 × 0.25)] × 1.00
      = [0.285 + 0.405 + 0.2375] × 1.00
      = 0.9275
```

**Box Set (147 tracks, 8,500,000ms):**
```
Duration: 254% diff (>25%) → 0.05
Quality: Good on first 11 tracks → 0.85
Name: "Aqualung 25th Anniversary" → 0.70
Track Count: |147 - 11| = 136 → penalty 0.20

Score = [(0.05 × 0.30) + (0.85 × 0.45) + (0.70 × 0.25)] × 0.20
      = [0.015 + 0.3825 + 0.175] × 0.20
      = 0.1145
```

**Ratio:** Standard (0.9275) vs Box Set (0.1145) = **8.1x higher score**

**Expected Winner:** Standard Edition (11 tracks)

### Verify
```rust
// Execute full matching pipeline
let result = match_album("test_data/aqualung/Jethro Tull - Aqualung.flac").await?;

// Verify edition selection
assert!(result.is_accepted());
assert_eq!(result.selected_edition.track_count, 11);
assert!(result.selected_edition.title.contains("Aqualung"));
assert!(!result.selected_edition.title.contains("Anniversary"));
assert!(!result.selected_edition.title.contains("Box"));

// Verify passage MBIDs match standard edition
for passage in &result.passages {
    assert!(passage.mbid.is_some());
    // MBIDs should match standard 11-track edition recordings
}

// Verify NOT box set
assert_ne!(result.selected_edition.track_count, 147);
```

### Pass Criteria
- **Primary:** Standard 11-track edition selected (NOT 147-track box set)
- **Secondary:** All 11 passages have correct MBIDs from standard edition
- **Score:** Standard edition scores >5x higher than box set

### Fail Criteria
- Box set (147 tracks) selected as best edition
- Passages assigned MBIDs from box set recordings
- Standard edition not in top 3 ranked editions

**Success Metric:** This test validates the PRIMARY goal - outcome-focused accuracy (correct MBIDs), not intermediate metrics (editions found).

---

## TC-S-ES-02: Goodbye Yellow Brick Road - Prefer 17-Track Standard Over 71-Track Deluxe

**Test ID:** TC-S-ES-02
**Test Type:** System Test (End-to-End)
**Requirements:** REQ-AM-092, REQ-AM-093, REQ-AM-094, REQ-AM-095
**Priority:** P0
**Environment:** Real GYBR album file + Live MusicBrainz data

### Scenario
User imports "Goodbye Yellow Brick Road" double album (~17 tracks, 76 minutes). MusicBrainz returns multiple editions:
- ~17-track standard edition (original 1973 release, 2LP)
- 71-track deluxe edition (40th anniversary, 4CD box set)
- ~17-track remaster (various reissue years)

**Potential Problem:** Deluxe edition may rank high due to good initial track alignment

### Setup
**Input File:**
- Detected total duration: ~4,560,000ms (76 minutes)
- Detected tracks: ~17 (double album, 2 LPs)
- Track durations: Standard edition alignment

**MusicBrainz Editions:**
- Standard (~17 tracks, 4,560,000ms): Excellent match
- Deluxe (71 tracks, 15,840,000ms): Poor duration, severe count mismatch
- Remaster (~17 tracks, 4,590,000ms): Excellent match

### When
- Complete album matching pipeline executes

### Then

**Expected Edition Scores:**

**Standard Edition (~17 tracks, 4,560,000ms):**
```
Duration: <5% diff → 0.95
Quality: Excellent → 0.90
Name: "Goodbye Yellow Brick Road" → 0.95
Track Count: 17 = 17 → penalty 1.00

Score = 0.9275
```

**Deluxe Edition (71 tracks, 15,840,000ms):**
```
Duration: 247% diff (>25%) → 0.05
Quality: Unknown (many extra tracks) → 0.50
Name: "Goodbye Yellow Brick Road 40th Anniversary" → 0.75
Track Count: |71 - 17| = 54 → penalty 0.20

Score = [(0.05 × 0.30) + (0.50 × 0.45) + (0.75 × 0.25)] × 0.20
      = 0.1025
```

**Expected Winner:** Standard or Remaster (~17 tracks)

### Verify
```rust
let result = match_album("test_data/gybr/Elton John - Goodbye Yellow Brick Road.flac").await?;

// Verify NOT deluxe edition
assert!(result.selected_edition.track_count >= 15 && result.selected_edition.track_count <= 20);
assert_ne!(result.selected_edition.track_count, 71);
assert!(!result.selected_edition.title.contains("Anniversary"));
assert!(!result.selected_edition.title.contains("Deluxe"));
```

### Pass Criteria
- Standard or remaster edition selected (~17 tracks)
- Deluxe edition (71 tracks) NOT selected
- All passages have correct MBIDs

### Fail Criteria
- Deluxe box set selected
- Track count significantly wrong (~71 instead of ~17)

---

## TC-S-ES-03: Japanese Edition with ±1 Bonus Track

**Test ID:** TC-S-ES-03
**Test Type:** System Test (End-to-End)
**Requirements:** REQ-AM-092, REQ-AM-093, REQ-AM-095
**Priority:** P0
**Environment:** Synthetic test data (Japanese vs Standard)

### Scenario
User imports album file with 12 tracks (Japanese edition with bonus track). MusicBrainz returns:
- 11-track standard edition (excellent quality match)
- 12-track Japanese edition (exact track count)

**Test Goal:** Verify ±1 track tolerance allows Japanese edition to compete with standard

### Setup
**Input File:**
- Detected total duration: 2,450,000ms
- Detected tracks: 12
- Track durations: Japanese edition alignment (first 11 match standard, track 12 is bonus)

**Editions:**
- Standard (11 tracks, 2,400,000ms): Excellent quality on first 11, missing track 12
- Japanese (12 tracks, 2,450,000ms): Excellent quality on all 12

### When
- Complete edition selection pipeline executes

### Then

**Expected Edition Scores:**

**Standard Edition (11 tracks):**
```
Duration: 2.04% diff → 0.95
Quality: 0.90 (first 11 tracks only)
Name: 0.90
Track Count: |11 - 12| = 1 → penalty 0.95

Score = [(0.95 × 0.30) + (0.90 × 0.45) + (0.90 × 0.25)] × 0.95
      = 0.914 × 0.95
      = 0.868
```

**Japanese Edition (12 tracks):**
```
Duration: <5% diff → 0.95
Quality: 0.95 (all 12 tracks)
Name: 0.85 (slightly lower, "Japan Edition" suffix)
Track Count: |12 - 12| = 0 → penalty 1.00

Score = [(0.95 × 0.30) + (0.95 × 0.45) + (0.85 × 0.25)] × 1.00
      = 0.9250
```

**Expected Winner:** Japanese Edition (0.925 vs 0.868 = 6.6% higher)

### Verify
```rust
let result = match_album("test_data/japanese/Album with Bonus Track.flac").await?;

// Verify Japanese edition selected
assert_eq!(result.selected_edition.track_count, 12);
assert!(result.selected_edition.title.contains("Japan") || result.selected_edition.title.contains("Japanese"));

// Verify all 12 passages matched
assert_eq!(result.passages.len(), 12);

// Verify standard edition ranked second (competitive but loses)
let rankings = result.edition_rankings;
assert_eq!(rankings[0].track_count, 12);  // Japanese wins
assert_eq!(rankings[1].track_count, 11);  // Standard second

// Verify scores are close (within 10%)
let score_diff_pct = (rankings[0].score - rankings[1].score) / rankings[0].score * 100.0;
assert!(score_diff_pct < 10.0);
```

### Pass Criteria
- Japanese edition (12 tracks) selected as best match
- Standard edition (11 tracks) ranks second (competitive)
- Score difference <10% (demonstrates ±1 tolerance effectiveness)

### Fail Criteria
- Standard edition selected despite missing bonus track
- Large score gap (>20%) suggests ±1 penalty too severe

**Note:** This test validates graduated track count tolerance - ±1 track is competitive with exact match when other factors are good.

---

## Test Data Requirements

**Real Album Files:**
- `test_data/aqualung/` - Jethro Tull album (11 tracks)
- `test_data/gybr/` - Elton John double album (~17 tracks)

**Synthetic Test Files:**
- `test_data/japanese/` - Mock album with 12 tracks (Japanese edition scenario)

**MusicBrainz Integration:**
- Live API calls to MusicBrainz (require network)
- OR Mock MusicBrainz responses with realistic edition data
- OR Cached MusicBrainz responses (snapshot testing)

**Ground Truth Validation:**
- Pre-verified correct MBIDs for each passage
- Expected edition ID for each test case
- Used to validate outcome accuracy (not just intermediate metrics)

---

## Success Criteria Summary

**All 3 System Tests Must Pass:**
- ✅ Aqualung: Standard (11) selected, NOT box set (147)
- ✅ GYBR: Standard (~17) selected, NOT deluxe (71)
- ✅ Japanese: Japanese (12) selected, standard (11) competitive

**Primary Metric (Outcome-Focused):**
- All passages have correct MBIDs (passage-level MBID accuracy)
- Correct edition selected (album-level success)

**NOT Success Metrics:**
- Number of editions found in search
- Match percentage with wrong edition

**Baseline:**
- Current: 85.3% album-level success (145/170 albums)
- Target: ≥98% album-level success
- These 3 tests represent critical failure cases from current implementation

---

**Test File Version:** 1.0
**Last Updated:** 2025-12-28
**Estimated Implementation Time:** 6-8 hours (all 3 tests + ground truth validation + real/mock MB data)
