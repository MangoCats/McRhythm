# PLAN027 Validation Guide - Real MP3 Files

**Created:** 2025-12-28
**Purpose:** Validate edition selection improvements using real album files

---

## Overview

This guide provides practical steps to validate the PLAN027 edition selection improvements using actual MP3 files from the Music library.

**Key Validation Points:**
1. ✅ Box sets are NOT selected over standard editions
2. ✅ Track count penalties work correctly (±1 track = minimal penalty, ±6+ = severe)
3. ✅ Duration alignment prevents mismatches
4. ✅ Multi-factor scoring produces correct rankings

---

## Test Albums Available

**Full-album MP3 files in Music folder:**

| Album | Artist | File Path | Test Purpose |
|-------|--------|-----------|--------------|
| Happy Nation | Ace of Base | `Music/Ace of Base/HappyNation.mp3` | Standard edition selection |
| Pump | Aerosmith | `Music/Aerosmith/Pump.mp3` | Standard vs deluxe editions |
| Anthology | 38 Special | `Music/38 Special/Anthology.mp3` | Greatest hits vs box sets |
| At Fillmore East | Allman Brothers | `Music/Allman Brothers/AtFillmoreEast.mp3` | Live album editions |
| Heartbeat City | Cars | `Music/Cars/HeartbeatCity.mp3` | Remaster vs original |
| Debut | Bjork | `Music/Bjork/Debut.mp3` | International editions |

---

## Manual Validation Tests

### Test 1: Happy Nation - Standard Edition Selection

**Album:** Ace of Base - Happy Nation
**File:** `c:/Users/Mango Cat/Music/Ace of Base/HappyNation.mp3`

**MusicBrainz Editions Known to Exist:**
- Standard (11-13 tracks depending on region)
- Deluxe/Extended (~17-20 tracks with bonus tracks)
- Japanese Edition (usually +1 or +2 tracks)

**Expected Behavior:**
1. Album matcher detects ~11-13 track boundaries
2. MusicBrainz search returns multiple editions
3. Multi-factor scoring selects standard edition
4. Box set/deluxe editions score significantly lower

**Validation Steps:**
```bash
cd wkmp-ai
cargo run --bin album_matcher_test -- \
  --file "c:/Users/Mango Cat/Music/Ace of Base/HappyNation.mp3" \
  --artist "Ace of Base" \
  --album "Happy Nation"
```

**Success Criteria:**
- ✅ Standard edition MBID assigned
- ✅ Match percentage ≥ 98%
- ✅ All passages have correct MBIDs
- ✅ Box set/deluxe NOT selected

---

### Test 2: Aerosmith Pump - Deluxe vs Standard

**Album:** Aerosmith - Pump
**File:** `c:/Users/Mango Cat/Music/Aerosmith/Pump.mp3`

**MusicBrainz Editions:**
- Standard (10 tracks)
- Deluxe (15-20 tracks with bonus material)
- Japanese (11 tracks, +1 bonus)

**Expected Behavior:**
1. If file has 10 tracks → Standard edition selected
2. If file has 11 tracks → Japanese edition competitive with standard
3. Deluxe edition (15-20 tracks) receives ±5-10 track penalty (0.50 or 0.20)

**Validation Steps:**
```bash
cd wkmp-ai
cargo run --bin album_matcher_test -- \
  --file "c:/Users/Mango Cat/Music/Aerosmith/Pump.mp3"
```

**Success Criteria:**
- ✅ Correct edition selected based on track count
- ✅ Track count penalty applied correctly
- ✅ Duration alignment score ≥ 0.80

---

### Test 3: 38 Special Anthology - Greatest Hits vs Box Set

**Album:** 38 Special - Anthology
**File:** `c:/Users/Mango Cat/Music/38 Special/Anthology.mp3`

**MusicBrainz Editions:**
- Greatest Hits compilation (varies: 12-18 tracks)
- Box sets (50+ tracks)
- Original albums (8-12 tracks each)

**Expected Behavior:**
1. Compilation edition selected (matches detected track count)
2. Box sets score very low (±30+ tracks = 0.20 penalty)
3. Individual album editions unlikely (too few tracks)

**Validation Steps:**
```bash
cd wkmp-ai
cargo run --bin album_matcher_test -- \
  --file "c:/Users/Mango Cat/Music/38 Special/Anthology.mp3"
```

**Success Criteria:**
- ✅ Compilation edition selected
- ✅ Box sets NOT selected
- ✅ Track count matches detected count (±2 tracks)

---

## Automated Validation Test Suite

### PLAN027 Validation Tests (NEW)

**Test File:** `wkmp-ai/tests/plan027_validation_test.rs`

**Run individual album tests:**

```bash
cd wkmp-ai

# Test 1: Happy Nation (standard vs deluxe)
cargo test --test plan027_validation_test test_plan027_happy_nation_standard_edition -- --ignored --nocapture

# Test 2: Aerosmith Pump (standard vs deluxe)
cargo test --test plan027_validation_test test_plan027_aerosmith_pump_standard_edition -- --ignored --nocapture

# Test 3: 38 Special Anthology (compilation vs box set)
cargo test --test plan027_validation_test test_plan027_anthology_compilation_not_box_set -- --ignored --nocapture
```

**Run comprehensive validation (all available test albums):**

```bash
cd wkmp-ai
cargo test --test plan027_validation_test test_plan027_comprehensive_validation -- --ignored --nocapture
```

**What It Tests:**
- Standard editions selected over box sets
- Track count penalties applied correctly
- Duration alignment prevents mismatches
- Multi-factor scoring produces correct rankings

**Success Criteria:**
- ✅ Each test album matches to appropriate edition
- ✅ Box sets NOT selected
- ✅ Track counts within expected ranges
- ✅ Match scores ≥ 0.80

---

### Full Library Test

**Run album matcher on all available MP3 files:**

```bash
cd wkmp-ai
cargo test --test full_library_import_test -- --nocapture
```

**Metrics to Track:**
- Album-level success rate (target: ≥98%)
- Passage-level MBID accuracy (target: ≥99.5%)
- Box set selection rate (target: ~0%)
- Average matching time (target: ≤115s per album)

**Comparison Baseline:**
- Previous (Run 27): 85.3% album success
- Expected (PLAN027): ≥98% album success

---

## Edition Selection Scoring Analysis

### Inspect Scoring Details

To see edition selection scoring in detail, enable debug logging:

```bash
export RUST_LOG=wkmp_ai::matching::editions=debug
cd wkmp-ai
cargo run --bin album_matcher_test -- \
  --file "c:/Users/Mango Cat/Music/Ace of Base/HappyNation.mp3"
```

**Expected Debug Output:**
```
[DEBUG] Edition scoring for "Happy Nation":
  - Standard (11 tracks): score=0.8975 [duration=0.95, quality=0.90, name=0.85, penalty=1.00]
  - Deluxe (17 tracks): score=0.4125 [duration=0.80, quality=0.85, name=0.80, penalty=0.50]
  - Box Set (147 tracks): score=0.0258 [duration=0.05, quality=0.15, name=0.85, penalty=0.20]
  - Selected: Standard (MBID: xxx-yyy-zzz)
```

---

## Performance Comparison

### Before PLAN027 (Run 27)

**Known Issues:**
- Aqualung: 147-track box set selected over 11-track standard
- GYBR: 71-track deluxe selected over 17-track standard
- ~15% of albums matched to wrong edition

**Album Success Rate:** 85.3%

### After PLAN027 (Expected)

**Improvements:**
- Box sets receive 0.20× penalty (±6+ tracks)
- Duration alignment prevents mismatches (>25% = 0.05 score)
- Track quality weighted highest (45%)

**Expected Album Success Rate:** ≥98%

---

## Troubleshooting

### Issue: Box Set Still Selected

**Diagnosis:**
1. Check track count penalty applied
2. Verify duration alignment score
3. Inspect name similarity (should NOT dominate)

**Fix:**
- Ensure `calculate_track_count_penalty()` is called
- Verify penalty range (0.20-1.00)
- Check multi-factor weighting (30/45/25)

### Issue: Japanese Edition Not Competitive

**Diagnosis:**
1. Check ±1 track penalty (should be 0.95)
2. Verify track quality score
3. Check duration alignment

**Expected:**
- Japanese edition within ~15% of standard score
- Both should be viable candidates

### Issue: No Editions Found

**Diagnosis:**
1. Check MusicBrainz search strategies
2. Verify artist/album metadata extraction
3. Ensure comprehensive_search() executed

**Fix:**
- Verify ID3 tags present
- Check filename pattern matching
- Ensure network connectivity to MusicBrainz

---

## Success Criteria Summary

**Unit Tests:** ✅ 31/31 passing
**Integration Tests:** ✅ 1/1 implemented (TC-I-092-01)
**System Tests:** ⏳ Manual validation required

**Deployment Checklist:**
- [x] Core algorithm implemented
- [x] Unit tests comprehensive
- [x] Integration test validates multi-edition scenarios
- [x] Orchestrator integration complete
- [ ] System tests with real MP3 files (manual validation)
- [ ] Full library test run (179 albums)
- [ ] Performance comparison vs Run 27

**Ready for Testing:** ✅ Yes
**Ready for Production:** ⏳ After full library validation

---

## Next Steps

1. **Manual Validation:** Test 3-5 representative albums from Music folder
2. **Full Library Test:** Run all 179 albums through matcher
3. **Performance Analysis:** Generate comparison report vs Run 27
4. **Edge Case Validation:** Test Japanese editions, box sets, remasters
5. **Deploy to Test Environment:** Validate in production-like setting

---

**Document Version:** 1.0
**Last Updated:** 2025-12-28
**Status:** Ready for validation testing
