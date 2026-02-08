# Increment 4: Logging and Result Formatting

**Objective:** Update logging to clearly indicate partial matches

---

## Deliverables

1. Update log format in orchestrator to include partial match details:
   ```
   Album match complete: matched=true, stage=Some(Stage2), percentage=100.0%, partial_match=8/11
   ```

2. Ensure `partial_match` field only appears for partial matches (backward compatible)

3. Update any summary/result formatting to show partial match status

---

## Files to Modify

- `wkmp-ai/src/matching/orchestrator.rs` - Log statements

---

## Acceptance Tests

- TC-I-PAM-003: Logging shows partial match details

---

## Implementation Notes

```rust
if result.partial {
    info!(
        "Album match complete: matched={}, stage={:?}, percentage={:.1}%, partial_match={}/{}",
        result.matched,
        result.stage,
        result.percentage,
        result.matched_tracks.unwrap_or(0),
        result.total_tracks.unwrap_or(0)
    );
} else {
    // Existing log format for full matches
    info!(
        "Album match complete: matched={}, stage={:?}, percentage={:.1}%",
        result.matched,
        result.stage,
        result.percentage
    );
}
```

---

## Success Criteria

- Partial matches logged with track count
- Full matches logged in existing format
- Log output parseable by existing tools
