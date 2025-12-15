# Increment 6: Error Handling & Fallback

**Increment:** 6 of 7
**Phase:** Implementation
**Estimated Effort:** 2-3 hours
**Confidence:** HIGH (±25%)
**Prerequisites:** Increment 4

---

## Objective

Implement robust error handling and fallback strategy for album matching failures.

---

## Deliverables

1. **Modified:** `wkmp-ai/src/workflow/pipeline.rs`
   - Refactor `process_as_single_song()` to use existing logic
   - Implement comprehensive error handling
   - Add retry logic for transient failures

---

## Requirements Covered

- **REQ-AM30-005:** Error handling and fallback strategy (complete)
- **REQ-AM30-011:** Handle single-track detection failure (complete)

---

## Implementation Notes

**Error Categories:**

| Error Type | Fallback Strategy |
|------------|-------------------|
| `AlbumMatchError::DecodeError` | Fallback to single-song (may also fail) |
| `AlbumMatchError::MetadataError` | Fallback to single-song |
| `AlbumMatchError::MusicBrainzError` | Retry once, then fallback |
| `AlbumMatchError::NoCandidates` | Fallback to single-song |
| `AlbumMatchError::InternalError` | Log and fail file |
| `AlbumMatchError::SingleTrackDetected` | Process as single-song |

**Implementation:**

```rust
impl Pipeline {
    /// Process file using existing single-song pipeline
    ///
    /// Extracts the original process_file() logic for reuse.
    async fn process_as_single_song(&self, file_path: &Path) -> Result<Vec<ProcessedPassage>> {
        info!("Processing file as single song: {:?}", file_path);

        // Phase 0: Detect passage boundaries with audio caching
        let file_audio = super::boundary_detector::detect_boundaries_with_audio(file_path)
            .await
            .context("Failed to detect passage boundaries")?;

        info!("Detected {} passages", file_audio.boundaries.len());

        // Emit boundary events
        for (i, boundary) in file_audio.boundaries.iter().enumerate() {
            self.emit_event(WorkflowEvent::BoundaryDetected {
                passage_index: i,
                start_time: boundary.start_time,
                end_time: boundary.end_time,
                confidence: boundary.confidence,
            })
            .await;
        }

        // Process each passage sequentially with cached audio
        let mut processed_passages = Vec::new();
        let total_passages = file_audio.boundaries.len();

        for (i, boundary) in file_audio.boundaries.iter().enumerate() {
            self.emit_event(WorkflowEvent::PassageStarted {
                passage_index: i,
                total_passages,
            })
            .await;

            match self
                .process_passage_with_audio(file_path, boundary, i, &file_audio)
                .await
            {
                Ok(passage) => {
                    self.emit_event(WorkflowEvent::PassageCompleted {
                        passage_index: i,
                        quality_score: passage.validation.score as f64,
                        validation_status: format!("{:?}", passage.validation.status),
                    })
                    .await;
                    processed_passages.push(passage);
                }
                Err(e) => {
                    error!("Failed to process passage {}: {}", i, e);
                    self.emit_event(WorkflowEvent::PassageFailed {
                        passage_index: i,
                        error: e.to_string(),
                    })
                    .await;
                    // Continue with remaining passages (per-passage error isolation)
                }
            }
        }

        // Emit completion
        self.emit_event(WorkflowEvent::FileCompleted {
            file_path: file_path.to_string_lossy().to_string(),
            passages_processed: processed_passages.len(),
            passages_failed: total_passages - processed_passages.len(),
        })
        .await;

        Ok(processed_passages)
    }

    /// Enhanced album match fallback with retry for transient errors
    async fn album_match_fallback(
        &self,
        file_path: &Path,
        error: &AlbumMatchError,
    ) -> Result<Vec<ProcessedPassage>> {
        // Check if error is retryable
        if self.is_retryable_error(error) {
            info!("Retrying album match after transient error...");

            // Wait before retry
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;

            // Create fresh matcher and retry
            if let Ok(matcher) = AlbumMatcher::new() {
                let retry_result = matcher
                    .match_album(file_path, None, None)
                    .await;

                if let Ok(result) = retry_result {
                    if result.matched {
                        return self.convert_album_to_passages(file_path, result).await;
                    }
                }
            }
        }

        warn!("Falling back to single-song processing: {}", error);
        self.emit_event(WorkflowEvent::AlbumMatchingFallback {
            file_path: file_path.to_string_lossy().to_string(),
            reason: error.to_string(),
        }).await;

        self.process_as_single_song(file_path).await
    }

    fn is_retryable_error(&self, error: &AlbumMatchError) -> bool {
        matches!(error,
            AlbumMatchError::MusicBrainzError(_) |
            AlbumMatchError::IoError(_)
        )
    }
}
```

---

## Tests to Pass

- **TC-U-006-01:** Decode error triggers fallback
- **TC-U-006-02:** MusicBrainz error triggers retry then fallback
- **TC-U-006-03:** SingleTrackDetected routes correctly
- **TC-U-006-04:** Fallback disabled prevents fallback

---

## Acceptance Criteria

- [ ] All error types handled appropriately
- [ ] Transient errors retry once before fallback
- [ ] Fallback produces valid single-song passages
- [ ] Error events emitted for tracking
- [ ] `album_match_fallback` config respected

---

## Verification Command

```bash
cargo build -p wkmp-ai
cargo test -p wkmp-ai workflow::tests::test_album_fallback
```
