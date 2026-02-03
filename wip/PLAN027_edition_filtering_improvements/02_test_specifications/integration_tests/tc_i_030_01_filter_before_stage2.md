# TC-I-030-01: Track Count Filter Before Stage 2

**Test ID:** TC-I-030-01
**Test Type:** Integration Test
**Requirement:** REQ-EF-030 (Track Count Pre-Filtering)
**Priority:** Critical
**Estimated Effort:** 45 minutes (write + implement + verify)

---

## Test Objective

Verify that track count pre-filtering is applied BEFORE Stage 2 (album-first matching) runs, reducing candidate editions and improving performance.

---

## Test Specification

**Scope:** Integration of `filter_by_track_count()` with Stage 2 matching pipeline

**Setup:**
- Use real album from test library (or subset)
- Album with multiple MusicBrainz editions (varying track counts)
- Example: Imagine Dragons - Night Visions (11-track standard, 16-track deluxe)

**Given:**
- Audio file with 11 detected tracks
- MusicBrainz returns 3 editions:
  1. Standard edition: 11 tracks
  2. Deluxe edition: 16 tracks
  3. Japanese edition: 14 tracks
- Stage 2 matching algorithm expects filtered edition list

**When:**
- Run full matching pipeline with track count pre-filtering enabled
- Monitor which editions are passed to Stage 2

**Then:**
- Track count filter runs before Stage 2
- Editions passed to Stage 2: Standard (11), Japanese (14)
- Edition NOT passed to Stage 2: Deluxe (16)
- Stage 2 receives only 2 editions (not all 3)

**Verify:**
- Log output shows filter applied before Stage 2
- Stage 2 receives filtered list (2 editions, not 3)
- Final match selects Standard edition (11 tracks)
- Performance: Stage 2 processes 2 editions instead of 3 (33% reduction)

---

## Pass Criteria

✅ Test passes if:
- Track count filter executed before Stage 2 (confirmed in logs)
- Stage 2 receives correct filtered edition count (2, not 3)
- 16-track deluxe edition filtered out
- 11-track standard edition selected as final match

❌ Test fails if:
- Filter not applied before Stage 2
- Stage 2 receives unfiltered list (3 editions)
- Deluxe edition not filtered
- Final match is incorrect

---

## Implementation Example

```rust
#[test]
fn test_track_count_filter_integration() {
    // Arrange
    let audio_file = "test_data/imagine_dragons_night_visions.flac";
    let detected_tracks = 11;

    // Mock MusicBrainz response with 3 editions
    let editions = vec![
        Edition { mbid: "standard-mbid", title: "Night Visions", track_count: 11, ... },
        Edition { mbid: "deluxe-mbid", title: "Night Visions (Deluxe)", track_count: 16, ... },
        Edition { mbid: "japanese-mbid", title: "Night Visions (Japan)", track_count: 14, ... },
    ];

    // Act
    let (result, logs) = run_matching_pipeline_with_logging(audio_file, editions, detected_tracks);

    // Assert
    // 1. Filter was applied
    assert!(logs.contains("Applying track count pre-filter"), "Filter should run before Stage 2");

    // 2. Correct editions filtered
    assert!(logs.contains("Filtered 1 edition(s)"), "Deluxe edition should be filtered");

    // 3. Stage 2 received filtered list
    assert!(logs.contains("Stage 2 processing 2 edition(s)"), "Stage 2 should receive 2 editions");

    // 4. Final match is correct
    assert_eq!(result.mbid, "standard-mbid", "Should match 11-track standard edition");
    assert_eq!(result.match_percentage, expected_percentage);
}
```

---

## Test Data

**Input:**
- Audio file: Imagine Dragons - Night Visions (11 tracks)
- MusicBrainz editions:
  - Standard: 11 tracks (within ±3)
  - Deluxe: 16 tracks (outside ±3, filtered)
  - Japanese: 14 tracks (within ±3)

**Expected Processing:**
```
1. MusicBrainz fetch: 3 editions
2. Track count filter: 3 → 2 (deluxe filtered)
3. Stage 2 input: 2 editions
4. Stage 2 output: Standard edition selected
```

**Expected Output:**
- Final match: Standard edition (11 tracks, MBID: standard-mbid)

---

## Logging Expectations

**Required Log Messages:**
```
DEBUG Detected 11 tracks in audio file
DEBUG Retrieved 3 editions from MusicBrainz
DEBUG Applying track count pre-filter (tolerance: ±3)
DEBUG   Standard edition: 11 tracks (diff=0) → PASS
DEBUG   Deluxe edition: 16 tracks (diff=5) → FILTERED
DEBUG   Japanese edition: 14 tracks (diff=3) → PASS
DEBUG Filtered 1 edition(s), 2 remaining
DEBUG Stage 2 processing 2 edition(s)
INFO  Matched: Night Visions (Standard) - 89.5% confidence
```

**Verification:**
Parse logs to confirm filter execution order and results.

---

## Performance Expectations

**Before Filtering:**
- Stage 2 processes 3 editions
- Estimated time: ~45ms (15ms per edition)

**After Filtering:**
- Stage 2 processes 2 editions
- Estimated time: ~30ms (15ms per edition)
- Performance improvement: 33% reduction

**Measurement:**
Log timestamps before/after Stage 2 to verify performance improvement.

---

## Edge Cases

**Covered in This Test:**
- Multiple editions available
- One edition filtered, others pass
- Final match from filtered set

**Not Covered (See Other Tests):**
- All editions filtered (TC-I-030-03)
- Empty edition list (TC-U-030-04)
- Integration with scoring (TC-I-030-02)

---

## Dependencies

**Code Dependencies:**
- `filter_by_track_count()` implemented (REQ-EF-030)
- Stage 2 modified to apply filter before matching
- Logging infrastructure available

**Test Dependencies:**
- Test audio file available
- MusicBrainz client can be mocked or real
- Log capture mechanism (set RUST_LOG=debug)

---

## Notes

**Rationale:** Track count pre-filtering must happen BEFORE Stage 2 to:
1. Reduce computation (fewer editions to process)
2. Improve match quality (unsuitable editions removed)
3. Fix problem albums (Imagine Dragons, Michael Jackson)

**Problem Album Context:**
- Imagine Dragons currently matches 16-track deluxe (wrong)
- After filter: Deluxe filtered, Standard selected (correct)

**Related Tests:**
- TC-U-030-03: Unit test for filtering logic
- TC-S-030-01: Imagine Dragons system test (end-to-end)
- TC-I-030-02: Integration with edition scoring
