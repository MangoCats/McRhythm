# TC-I-PAM-001: Partial Match Returns Valid MBIDs

**Test Type:** Integration Test
**Requirements:** REQ-PAM-002, REQ-PAM-003
**Priority:** P0

---

## Scope

Integration of partial track matching with MBID retrieval

---

## Setup

- Mock MusicBrainz client with canned release data
- Test release with 11 tracks and known MBIDs
- File duration matching first 8 tracks

---

## Test Case

**Given:**
- Edition with 11 tracks (durations: 373, 362, 383, 365, 402, 333, 349, 422, 348, 362, 340)
- File duration: 2989s (cumulative of tracks 1-8)
- Edition has valid Recording MBIDs for all tracks

**When:** Partial album matching is performed

**Then:**
- Result contains exactly 8 Recording MBIDs
- MBIDs match tracks 1-8 of the edition (in order)
- No MBIDs returned for tracks 9-11
- Result indicates `partial: true`
- Result includes `matched_tracks: 8, total_tracks: 11`

---

## Verification

```rust
assert!(result.matched);
assert!(result.partial);
assert_eq!(result.recording_mbids.len(), 8);
assert_eq!(result.matched_tracks, 8);
assert_eq!(result.total_tracks, 11);

// Verify MBIDs are for correct tracks
for (i, mbid) in result.recording_mbids.iter().enumerate() {
    assert_eq!(*mbid, expected_mbids[i], "Track {} MBID mismatch", i + 1);
}
```

---

## Pass Criteria

- 8 valid MBIDs returned
- MBIDs correspond to tracks 1-8 (not tracks 4-11 or any other subset)
- Partial match flag is set

---

## Estimated Effort

Implementation: 30 minutes
Test writing: 20 minutes
