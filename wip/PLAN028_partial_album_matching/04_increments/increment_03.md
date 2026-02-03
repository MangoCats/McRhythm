# Increment 3: Integrate Partial Matching into Pipeline

**Objective:** Wire partial matching into the album matching orchestrator

---

## Deliverables

1. Modify `orchestrator.rs` to:
   - Detect partial album candidates after full match duration filter fails
   - Call partial matching functions from Increment 1
   - Create partial AlbumMatchResult with proper fields set

2. Modify `filtering.rs` to:
   - Return specific rejection reason distinguishing "below 50%" from "50-85% (partial candidate)"
   - Allow orchestrator to decide whether to try partial matching

---

## Files to Modify

- `wkmp-ai/src/matching/orchestrator.rs` - Main integration point
- `wkmp-ai/src/matching/editions/filtering.rs` - Rejection reason details

---

## Logic Flow

```
file_duration / edition_duration:
  < 50%  → Reject (too short)
  50-85% → Try partial matching
  85-125% → Full match (existing path)
  > 125% → Reject (too long)
```

---

## Acceptance Tests

- TC-I-PAM-001: Partial match returns valid MBIDs
- TC-I-PAM-002: Quality threshold applies

---

## Implementation Notes

```rust
// In orchestrator, after duration filter fails
if is_partial_album_candidate(file_duration, edition_duration) {
    if let Some(track_count) = find_partial_track_count(
        file_duration,
        &track_durations,
        DURATION_TOLERANCE
    ) {
        if is_partial_match_acceptable(track_count, total_tracks) {
            // Attempt boundary detection for first N tracks
            let partial_result = match_partial_album(
                audio_features,
                edition,
                track_count
            );
            return partial_result;
        }
    }
}
```

---

## Success Criteria

- Partial matching triggers for 50-85% duration ratio
- Existing full matches unchanged
- Partial match attempts use correct track count
