# Increment 2: Single-Track Check Integration

**Increment:** 2 of 7
**Phase:** Implementation
**Estimated Effort:** 2-3 hours
**Confidence:** HIGH (±25%)
**Prerequisites:** Increment 1

---

## Objective

Add single-track discrimination check at the start of `process_file()` to determine routing.

---

## Deliverables

1. **Modified:** `wkmp-ai/src/workflow/pipeline.rs`
   - Add `use crate::matching::SingleTrackDiscriminator;`
   - Add routing logic in `process_file()`

---

## Requirements Covered

- **REQ-AM30-001:** Single-track discrimination in pipeline (complete)
- **REQ-AM30-011:** Handle single-track detection failure (partial)

---

## Implementation Notes

```rust
// In process_file(), after initial event emission:

pub async fn process_file(&self, file_path: &Path) -> Result<Vec<ProcessedPassage>> {
    info!("Pipeline processing file: {:?}", file_path);

    // Emit file started event
    self.emit_event(WorkflowEvent::FileStarted {
        file_path: file_path.to_string_lossy().to_string(),
        timestamp: chrono::Utc::now().timestamp(),
    })
    .await;

    // **[PLAN_am30_integration]** Step 1: Detect file type (single track vs album)
    if self.config.enable_album_matching {
        let single_track_analysis = SingleTrackDiscriminator::analyze_pre_decode(file_path, None);

        self.emit_event(WorkflowEvent::SingleTrackCheckCompleted {
            file_path: file_path.to_string_lossy().to_string(),
            score: single_track_analysis.score,
            is_single_track: single_track_analysis.score >= self.config.single_track_threshold,
        }).await;

        if single_track_analysis.score < self.config.single_track_threshold {
            // Likely an album - route to album processing
            info!(
                "File appears to be an album (score={:.2} < threshold={})",
                single_track_analysis.score,
                self.config.single_track_threshold
            );
            return self.process_album_file(file_path).await;
        } else {
            info!(
                "File appears to be single track (score={:.2} >= threshold={})",
                single_track_analysis.score,
                self.config.single_track_threshold
            );
        }
    }

    // Continue with existing single-song processing...
    // Phase 0: Detect passage boundaries with audio caching
    // ...
}
```

**New Method Stub:**

```rust
/// Process file as full album using AlbumMatcher
async fn process_album_file(&self, file_path: &Path) -> Result<Vec<ProcessedPassage>> {
    // Implemented in Increment 3
    todo!("Album processing not yet implemented")
}
```

---

## Tests to Pass

- **TC-U-002-01:** Single track file routes to existing flow
- **TC-U-002-02:** Album file routes to album processing
- **TC-U-002-03:** Disabled album matching bypasses check

---

## Acceptance Criteria

- [ ] `SingleTrackDiscriminator::analyze_pre_decode()` called for all files
- [ ] Files with score >= 1.5 continue through existing pipeline
- [ ] Files with score < 1.5 call `process_album_file()`
- [ ] `enable_album_matching=false` skips discrimination check
- [ ] New event type added for single-track check result

---

## New Event Type

Add to `WorkflowEvent` enum in `mod.rs`:

```rust
/// Single-track vs album check completed
SingleTrackCheckCompleted {
    /// Path to audio file
    file_path: String,
    /// Discrimination score
    score: f64,
    /// Whether file is classified as single track
    is_single_track: bool,
},
```

---

## Verification Command

```bash
cargo build -p wkmp-ai
cargo test -p wkmp-ai workflow
```
