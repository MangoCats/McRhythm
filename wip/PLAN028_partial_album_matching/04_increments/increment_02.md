# Increment 2: AlbumMatchResult Extensions

**Objective:** Extend result types to support partial match information

---

## Deliverables

1. Modify `AlbumMatchResult` struct to add:
   - `partial: bool` (default: false)
   - `matched_tracks: Option<usize>`
   - `total_tracks: Option<usize>`

2. Update result construction in existing code paths to set:
   - `partial: false` for full matches
   - `matched_tracks: None` / `total_tracks: None` for full matches

3. Ensure backward compatibility (existing tests unchanged)

---

## Files to Modify

- `wkmp-ai/src/matching/album_matcher.rs` - AlbumMatchResult struct
- `wkmp-ai/src/matching/orchestrator.rs` - Result construction
- Any files that create AlbumMatchResult instances

---

## Acceptance Tests

- Existing tests still pass (no regressions)
- New fields default appropriately for full matches

---

## Implementation Notes

```rust
pub struct AlbumMatchResult {
    pub matched: bool,
    pub percentage: f64,
    // ... existing fields ...

    // New fields for partial matching
    pub partial: bool,
    pub matched_tracks: Option<usize>,
    pub total_tracks: Option<usize>,
}

impl Default for AlbumMatchResult {
    fn default() -> Self {
        Self {
            // ... existing defaults ...
            partial: false,
            matched_tracks: None,
            total_tracks: None,
        }
    }
}
```

---

## Success Criteria

- All existing tests pass (backward compatible)
- New fields available but not used yet
- No breaking changes to API
