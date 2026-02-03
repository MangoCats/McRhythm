# Increment 5: Progress Events

**Increment:** 5 of 7
**Phase:** Implementation
**Estimated Effort:** 1-2 hours
**Confidence:** HIGH (±20%)
**Prerequisites:** Increment 4

---

## Objective

Add detailed progress events for album matching stages to enable UI feedback.

---

## Deliverables

1. **Modified:** `wkmp-ai/src/workflow/mod.rs`
   - Add album-specific `WorkflowEvent` variants
2. **Modified:** `wkmp-ai/src/matching/album_matcher.rs` (optional)
   - Add progress callback support

---

## Requirements Covered

- **REQ-AM30-004:** Emit progress events for album stages (complete)
- **REQ-AM30-012:** Support album-specific workflow events (complete)

---

## Implementation Notes

**New Event Types in `WorkflowEvent`:**

```rust
/// Album matching stage started
AlbumMatchingStageStarted {
    file_path: String,
    stage: String,  // "stage2", "stage3", "stage4", "stage5"
    description: String,
},

/// Album matching stage completed
AlbumMatchingStageCompleted {
    file_path: String,
    stage: String,
    best_percentage: f64,
    editions_tested: usize,
},

/// Edition being tested
AlbumEditionTesting {
    file_path: String,
    edition_index: usize,
    total_editions: usize,
    artist: String,
    album: String,
},

/// Edition test result
AlbumEditionResult {
    file_path: String,
    edition_index: usize,
    match_percentage: f64,
    stage: String,
},
```

**AlbumMatcher Progress Callback (optional enhancement):**

```rust
// In AlbumMatcher, add optional progress callback:
pub type ProgressCallback = Box<dyn Fn(ProgressEvent) + Send + Sync>;

pub enum ProgressEvent {
    StageStarted { stage: MatchingStage, description: String },
    StageCompleted { stage: MatchingStage, percentage: f64 },
    EditionTesting { index: usize, total: usize, artist: String, album: String },
    EditionResult { index: usize, percentage: f64, stage: MatchingStage },
}

impl AlbumMatcher {
    pub fn with_progress_callback(
        config: AlbumMatcherConfig,
        callback: ProgressCallback,
    ) -> Result<Self, AlbumMatchError> {
        // ...
    }
}
```

**Pipeline Integration:**

```rust
// In process_album_file(), create progress callback:
let event_tx = self.event_tx.clone();
let file_path_str = file_path.to_string_lossy().to_string();

let progress_callback = Box::new(move |event: ProgressEvent| {
    if let Some(ref tx) = event_tx {
        let workflow_event = match event {
            ProgressEvent::StageStarted { stage, description } => {
                WorkflowEvent::AlbumMatchingStageStarted {
                    file_path: file_path_str.clone(),
                    stage: stage.to_string(),
                    description,
                }
            }
            // ... map other events
        };
        let _ = tx.try_send(workflow_event);
    }
});
```

---

## Tests to Pass

- **TC-U-005-01:** Stage events emitted during album matching
- **TC-U-005-02:** Edition events emitted for each tested edition
- **TC-U-005-03:** Events are serializable for SSE

---

## Acceptance Criteria

- [ ] All album matching stages emit start/complete events
- [ ] Per-edition progress visible via events
- [ ] Events are JSON-serializable
- [ ] No event loss under load

---

## Verification Command

```bash
cargo build -p wkmp-ai
cargo test -p wkmp-ai workflow
```
