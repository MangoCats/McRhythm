# Increment 5: Fluke/Puppy.mp3 System Test

**Objective:** Verify Fluke/Puppy.mp3 now matches correctly

---

## Deliverables

1. Update test baseline for Fluke/Puppy.mp3:
   - Change from `baseline_tracks: 0` to `baseline_tracks: 8`
   - Add expected partial match flag

2. Run single-album test for Fluke/Puppy.mp3

3. Verify results:
   - `matched: true`
   - `partial: true`
   - `matched_tracks: 8`
   - `total_tracks: 11`
   - 8 valid Recording MBIDs

---

## Files to Modify

- `wkmp-ai/tests/run29f_full_comparison_test.rs` - Baseline update

---

## Acceptance Tests

- TC-S-PAM-001: Fluke/Puppy.mp3 matches 8/11 tracks

---

## Test Command

```bash
cargo test -p wkmp-ai -- --test-threads=1 fluke_puppy --nocapture
```

---

## Expected Output

```
Testing: Fluke/Puppy.mp3
Duration: 2995s
Partial album candidate: true (74.2%)
Finding best track count...
  Tracks 1-8 cumulative: 2989s (0.2% difference) ✓
  Attempting boundary detection for 8 tracks...
Match result:
  matched: true
  partial: true
  matched_tracks: 8
  total_tracks: 11
  percentage: 100.0%
  MBIDs: 8 valid recordings
```

---

## Success Criteria

- Fluke/Puppy.mp3 matches (was rejected before)
- 8 MBIDs returned for AcousticBrainz lookup
- Match percentage ≥80% for matched tracks
