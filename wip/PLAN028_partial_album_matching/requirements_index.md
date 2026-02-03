# PLAN028: Partial Album Matching - Requirements Index

## Problem Statement

Files containing a contiguous subset of tracks from the beginning of an album are currently rejected by the duration filter (85% minimum threshold). Example:

| File | Duration | Album Duration | Ratio | Current Result |
|------|----------|----------------|-------|----------------|
| Fluke/Puppy.mp3 | 2995s (49:55) | 4039s (67:19) | 74.2% | REJECTED |

However, this file contains **tracks 1-8** of the 11-track album (cumulative duration: 2989s), which could yield 8 valid Recording MBIDs for AcousticBrainz lookup.

## Objective

Enable matching of files that contain a contiguous subset of tracks from the beginning of an album, returning valid MBIDs for the matched tracks.

---

## Requirements

| Req ID | Type | Priority | Description |
|--------|------|----------|-------------|
| REQ-PAM-001 | Functional | P0 | Detect when file duration is 50-85% of edition duration (partial album candidate) |
| REQ-PAM-002 | Functional | P0 | Attempt track matching from beginning of album when partial candidate detected |
| REQ-PAM-003 | Functional | P0 | Return valid Recording MBIDs for all matched tracks in partial match |
| REQ-PAM-004 | Functional | P1 | Require minimum 60% track coverage for partial match to be accepted |
| REQ-PAM-005 | Functional | P1 | Apply match quality threshold (≥80% of matched tracks within tolerance) |
| REQ-PAM-006 | Functional | P2 | Log partial match with clear indication it's not a full album |
| REQ-PAM-007 | Non-Functional | P1 | No regressions on existing 200-album test suite |
| REQ-PAM-008 | Non-Functional | P2 | Partial matching adds <5% to overall matching time |

---

## Detailed Requirements

### REQ-PAM-001: Partial Album Candidate Detection

**SHALL** detect files where:
- `file_duration / edition_duration` is between 0.50 and 0.85 (50-85%)
- Edition has valid track durations
- File duration is sufficient for at least 4 tracks (minimum viable match)

**Rationale:** Below 50% there's insufficient content for reliable matching. Above 85% the existing full-album match applies.

### REQ-PAM-002: Partial Track Matching

**SHALL** attempt to match tracks from the beginning of the album by:
1. Calculating cumulative duration for tracks 1..N
2. Finding N where cumulative duration best matches file duration (within 2%)
3. Running boundary detection for N tracks instead of full album
4. Matching detected boundaries against first N tracks

**Example (Puppy.mp3):**
- File duration: 2995s
- Tracks 1-8 cumulative: 2989s (0.2% difference)
- Match against 8 tracks, not 11

### REQ-PAM-003: Return Valid MBIDs

**SHALL** return `AlbumMatchResult` containing:
- `matched: true` with partial match flag
- Recording MBIDs for all matched tracks
- Track boundaries for matched tracks only
- Match percentage based on matched tracks (not full album)

### REQ-PAM-004: Minimum Track Coverage

**SHALL** require at least 60% of album tracks to be present:
- 11-track album: minimum 7 tracks
- 10-track album: minimum 6 tracks
- 8-track album: minimum 5 tracks

**Rationale:** Below 60% the match is unreliable and provides limited value.

### REQ-PAM-005: Match Quality Threshold

**SHALL** apply same quality standards as full album match:
- At least 80% of matched tracks within 10s tolerance
- Use existing scoring algorithms for quality assessment

### REQ-PAM-006: Logging

**SHALL** log partial matches with clear distinction:
```
Album match complete: matched=true, stage=Some(Stage2), percentage=100.0%, partial=true, tracks=8/11
```

### REQ-PAM-007: No Regressions

**SHALL** pass all existing tests in run29f_full_comparison_test without regression:
- All 187 matched albums maintain same or better score
- No new failures introduced

### REQ-PAM-008: Performance

**SHOULD** add minimal overhead:
- Partial matching only attempted after full match fails duration filter
- No additional API calls to MusicBrainz
- Reuse existing boundary detection algorithms

---

## Out of Scope

- Matching tracks from middle or end of album (non-contiguous)
- Matching shuffled track order
- Handling multiple partial segments (e.g., tracks 1-4 + 8-11)
- Automatic file repair/completion suggestions

---

## Assumptions

1. Partial files typically contain tracks from the beginning (most common ripping/encoding pattern)
2. MusicBrainz track durations are accurate within 2-3 seconds
3. Files are not corrupted (just incomplete)
4. Existing boundary detection works for partial track sets

---

## Success Criteria

1. Fluke/Puppy.mp3 matches with 8/11 tracks (100% of matched tracks within tolerance)
2. 8 valid Recording MBIDs returned for AcousticBrainz lookup
3. No regressions on 200-album test suite
4. Clear logging distinguishes partial from full matches
