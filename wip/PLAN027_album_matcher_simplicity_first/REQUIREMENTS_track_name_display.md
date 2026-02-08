# Track Name Display Requirements

**Date**: 2025-12-30
**Status**: ✅ **COMPLETE** - All requirements implemented and tested
**Related**: Top-5 Candidate Ranking Feature

---

## Requirement Summary

**REQ-TND-001**: The candidate ranking tables MUST display track names/titles for each track in addition to durations and timing errors.

**REQ-TND-002**: Track names MUST be sourced from MusicBrainz metadata for each candidate edition.

**REQ-TND-003**: The enhanced tables MUST be output for ALL albums in validation tests.

---

## Rationale

Track names provide critical context for understanding:
1. **Structural differences** between editions (bonus tracks, extended versions)
2. **Track ordering** differences (remastered vs original)
3. **Content identification** (which tracks are extra/missing)
4. **Human readability** (easier to spot anomalies by track name vs just numbers)

**Example**: Without track names, you see "Track 3: 17.00s expected". With names, you see "Track 3: Young Lust (Intro) - 17.00s" - immediately clear this is a bonus intro track.

---

## Technical Requirements

### REQ-TND-004: Edition Type Enhancement

The `Edition` struct MUST include:
```rust
pub struct Edition {
    // ... existing fields ...

    /// Track titles from MusicBrainz
    pub track_titles: Vec<String>,
}
```

### REQ-TND-005: PassageComparison Type Enhancement

The `PassageComparison` struct MUST include:
```rust
pub struct PassageComparison {
    /// Track/passage number (1-based)
    pub track_number: usize,

    /// Track title from MusicBrainz edition
    pub track_title: String,

    /// Detected passage duration from audio analysis (seconds)
    pub detected_duration: f64,

    /// Expected track duration from MusicBrainz (seconds)
    pub expected_duration: f64,

    /// Absolute timing error (seconds)
    pub error: f64,

    /// Whether error is within tolerance
    pub within_tolerance: bool,
}
```

### REQ-TND-006: Edition Grouping Enhancement

The `group_into_editions()` function MUST:
1. Collect track titles from `MBTrack.title` for each track
2. Store titles in `Edition.track_titles` field
3. Maintain track order (same as durations and recording MBIDs)

### REQ-TND-007: Ranking Algorithm Enhancement

The `rank_top_candidates()` function MUST:
1. Include track titles when building `PassageComparison` structs
2. Match track titles by index from `Edition.track_titles`
3. Handle missing titles gracefully (empty string if not available)

### REQ-TND-008: Display Output Format

The candidate ranking output MUST display track information in this format:
```
  Trk   Track Title                    Expected   Detected      Error     Status
  --------------------------------------------------------------------------------------
    1   Young Lust                      259.44s    257.85s      1.59s          ✓
    2   F.I.N.E.                        249.67s    250.05s      0.38s          ✓
    3   Love in an Elevator             338.96s    339.10s      0.14s          ✓
    ...
```

**Format Specifications**:
- Track number: Right-aligned, 3 characters
- Track title: Left-aligned, 30 characters (truncate if longer, add ellipsis)
- Durations: Right-aligned, 9 characters with 2 decimal places
- Error: Right-aligned, 9 characters with 2 decimal places
- Status: Right-aligned, 10 characters (✓ or ✗)

### REQ-TND-009: Title Truncation

Long track titles MUST be truncated intelligently:
- Maximum display length: 30 characters
- If title >30 chars: Truncate to 27 chars + "..."
- Examples:
  - "Love in an Elevator" → "Love in an Elevator"
  - "The Ballad of John and Yoko (Remastered 2009)" → "The Ballad of John and Yok..."

### REQ-TND-010: Missing Title Handling

If track title is not available from MusicBrainz:
- Use placeholder: "(Track {number})"
- Example: "(Track 3)" for track 3 with missing title

---

## Test Requirements

### REQ-TND-011: Validation Test Output

ALL validation tests MUST output enhanced tables with track names:
- `test_plan027_aerosmith_pump_standard_edition`
- `test_plan027_happy_nation_standard_edition`
- `test_plan027_anthology_not_box_set`
- `test_plan027_comprehensive_validation` (all 5 albums)

### REQ-TND-012: Example Output Validation

The Aerosmith Pump test output MUST include track names for all 5 ranked candidates, clearly showing:
- Standard 10-track editions with album track names
- 14-track deluxe edition with bonus track names (e.g., "Young Lust (Intro)")
- 11-track Japanese edition with bonus track name

---

## Implementation Checklist

- [x] **Step 1**: Add `track_titles: Vec<String>` to `Edition` struct ([types.rs](../../wkmp-ai/src/matching/types.rs)) ✅
- [x] **Step 2**: Add `track_title: String` to `PassageComparison` struct ([types.rs](../../wkmp-ai/src/matching/types.rs)) ✅
- [x] **Step 3**: Update `group_into_editions()` to collect track titles ([editions/grouping.rs](../../wkmp-ai/src/matching/editions/grouping.rs)) ✅
- [x] **Step 4**: Update `rank_top_candidates()` to include track titles ([orchestrator.rs](../../wkmp-ai/src/matching/orchestrator.rs)) ✅
- [x] **Step 5**: Update display output format ([album_matcher.rs](../../wkmp-ai/src/matching/album_matcher.rs)) ✅
- [x] **Step 6**: Test with Aerosmith Pump album ✅
- [x] **Step 7**: Verify comprehensive validation test outputs ✅

**Implementation Complete**: 2025-12-30
**Example Output**: See [Pump_Enhanced_Output_With_Track_Names.md](../../../Pump_Enhanced_Output_With_Track_Names.md)

---

## Success Criteria

✅ **Feature is complete when**:
1. All validation tests output track names for all candidates
2. Track names align correctly with durations
3. Bonus tracks are clearly identifiable by name
4. Output is readable and properly formatted
5. No compilation errors or test failures

---

## Related Documents

- [Top-5 Candidate Ranking Implementation](COMPREHENSIVE_VALIDATION_RESULTS.md)
- [PLAN027 Summary](00_PLAN_SUMMARY.md)
- [Session Completion Summary](SESSION_COMPLETION_SUMMARY.md)
