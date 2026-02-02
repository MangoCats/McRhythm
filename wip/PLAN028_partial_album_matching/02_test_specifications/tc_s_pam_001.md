# TC-S-PAM-001: Fluke/Puppy.mp3 System Test

**Test Type:** System Test
**Requirements:** REQ-PAM-001 through REQ-PAM-006
**Priority:** P0

---

## Environment

- Full wkmp-ai test environment
- MusicBrainz API access (rate-limited)
- Test file: `Fluke/Puppy.mp3` (71MB, 2995s)

---

## Scenario

### Pre-conditions

**Given:**
- Fluke/Puppy.mp3 exists in test corpus
- File duration: 2995 seconds (49:55)
- MusicBrainz release: a80b68e3-227b-4536-9b25-9c0aa51c1359
- Release duration: 4039 seconds (11 tracks)
- Tracks 1-8 cumulative duration: 2989 seconds

### Test Execution

**When:** Album matching is run on Fluke/Puppy.mp3

**Then:**
1. Partial album candidate detection triggers (74.2% ratio)
2. Cumulative track matching finds best fit at 8 tracks
3. Boundary detection runs for 8 tracks (not 11)
4. Match result returned with:
   - `matched: true`
   - `partial: true`
   - `matched_tracks: 8`
   - `total_tracks: 11`
   - 8 valid Recording MBIDs

---

## Verification

| Check | Expected | Verification Method |
|-------|----------|---------------------|
| Match result | `matched=true` | Assert in test |
| Partial flag | `partial=true` | Assert in test |
| Matched tracks | 8 | Assert `matched_tracks == 8` |
| Total tracks | 11 | Assert `total_tracks == 11` |
| MBID count | 8 | Assert `recording_mbids.len() == 8` |
| Match percentage | ≥80% | Assert `percentage >= 80.0` |
| Log output | Contains "partial=true, tracks=8/11" | Log inspection |

---

## Pass Criteria

- All 8 tracks matched with valid MBIDs
- Match percentage ≥80% (of the 8 matched tracks)
- Partial match indicator present in result
- No false positive (doesn't claim full album match)

---

## Fail Criteria

- File rejected by duration filter (current behavior)
- Match returns `matched: false`
- MBID count ≠ 8
- Reports as full album match (11 tracks)

---

## Test Data

**Expected Recording MBIDs (tracks 1-8 of Fluke - Puppy):**
- Track 1 (Absurd): MusicBrainz Recording ID from release
- Track 2 (Atom Bomb): MusicBrainz Recording ID from release
- Track 3 (Bullet): MusicBrainz Recording ID from release
- Track 4 (Squirt): MusicBrainz Recording ID from release
- Track 5 (Play Thing): MusicBrainz Recording ID from release
- Track 6 (Kitten Moon): MusicBrainz Recording ID from release
- Track 7 (Switch/Twitch): MusicBrainz Recording ID from release
- Track 8 (YKK): MusicBrainz Recording ID from release

Note: Actual MBIDs fetched from MusicBrainz API during test execution.

---

## Estimated Effort

Test implementation: 45 minutes
Execution: 5 minutes (including API calls)
