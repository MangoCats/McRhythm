# Increment 1: Partial Album Detection Functions

**Objective:** Implement core detection and calculation functions

---

## Deliverables

1. Create `wkmp-ai/src/matching/partial_matching.rs` with:
   - `is_partial_album_candidate(file_duration, edition_duration) -> bool`
   - `calculate_cumulative_durations(track_durations) -> Vec<f64>`
   - `find_partial_track_count(file_duration, track_durations, tolerance) -> Option<usize>`
   - `is_partial_match_acceptable(matched_tracks, total_tracks) -> bool`

2. Add module to `wkmp-ai/src/matching/mod.rs`

3. Unit tests for all functions

---

## Acceptance Tests

- TC-U-PAM-001: Partial candidate detection (50-85%)
- TC-U-PAM-002: Below threshold rejection
- TC-U-PAM-003: Above threshold skip
- TC-U-PAM-004: Cumulative duration calculation
- TC-U-PAM-005: Best N tracks selection
- TC-U-PAM-006: Minimum 60% coverage

---

## Implementation Notes

```rust
// Constants
const MIN_PARTIAL_RATIO: f64 = 0.50;  // 50%
const MAX_PARTIAL_RATIO: f64 = 0.85;  // 85% (below full match threshold)
const MIN_TRACK_COVERAGE: f64 = 0.60; // 60%
const DURATION_TOLERANCE: f64 = 0.02; // 2%

pub fn is_partial_album_candidate(file_duration: f64, edition_duration: f64) -> bool {
    let ratio = file_duration / edition_duration;
    ratio >= MIN_PARTIAL_RATIO && ratio < MAX_PARTIAL_RATIO
}
```

---

## Success Criteria

- All 6 unit tests pass
- Functions work independently (no integration yet)
- Module compiles with no warnings
