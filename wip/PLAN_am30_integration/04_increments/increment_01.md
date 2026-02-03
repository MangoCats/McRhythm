# Increment 1: Pipeline Config Extension

**Increment:** 1 of 7
**Phase:** Implementation
**Estimated Effort:** 1-2 hours
**Confidence:** HIGH (±20%)
**Prerequisites:** None

---

## Objective

Extend `PipelineConfig` to include album matching configuration options.

---

## Deliverables

1. **Modified:** `wkmp-ai/src/workflow/pipeline.rs`
   - Add album matching configuration fields to `PipelineConfig`

---

## Requirements Covered

- **REQ-AM30-007:** PipelineConfig extension for album matching (complete)

---

## Implementation Notes

```rust
// Add to PipelineConfig struct:

/// Pipeline configuration
#[derive(Clone)]
pub struct PipelineConfig {
    // ... existing fields ...

    // --- Album Matching Configuration (PLAN_am30_integration) ---
    /// Enable album detection and matching
    pub enable_album_matching: bool,
    /// Single-track score threshold (scores >= this are treated as single tracks)
    pub single_track_threshold: f64,
    /// Match percentage threshold to accept album match
    pub album_match_min_percentage: f64,
    /// Enable fallback to single-song processing if album match fails
    pub album_match_fallback: bool,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            // ... existing defaults ...

            // Album matching defaults
            enable_album_matching: true,
            single_track_threshold: 1.5,  // From SINGLE_TRACK_SCORE_THRESHOLD
            album_match_min_percentage: 80.0,
            album_match_fallback: true,
        }
    }
}
```

---

## Tests to Pass

- **TC-U-001-01:** Config defaults are sensible
- **TC-U-001-02:** Config can be constructed with custom values

---

## Acceptance Criteria

- [ ] `PipelineConfig` has album matching fields
- [ ] Defaults match expected values
- [ ] Existing tests continue to pass

---

## Verification Command

```bash
cargo test -p wkmp-ai pipeline::tests
cargo build -p wkmp-ai
```
