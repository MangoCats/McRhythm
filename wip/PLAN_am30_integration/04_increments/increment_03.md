# Increment 3: AlbumMatcher Integration

**Increment:** 3 of 7
**Phase:** Implementation
**Estimated Effort:** 3-4 hours
**Confidence:** HIGH (±25%)
**Prerequisites:** Increment 2

---

## Objective

Implement `process_album_file()` to call `AlbumMatcher.match_album()` for album files.

---

## Deliverables

1. **Modified:** `wkmp-ai/src/workflow/pipeline.rs`
   - Implement `process_album_file()` method
   - Add `AlbumMatcher` as optional pipeline dependency

---

## Requirements Covered

- **REQ-AM30-002:** Route album files to AlbumMatcher (complete)
- **REQ-AM30-006:** Share MusicBrainz client (partial)
- **REQ-AM30-008:** Integrate with Stage 0 (complete)

---

## Implementation Notes

```rust
use crate::matching::{AlbumMatcher, AlbumMatcherConfig, AlbumMatchResult, AlbumMatchError};
use crate::extractors::id3_extractor::ID3Extractor;

impl Pipeline {
    /// Process file as full album using AlbumMatcher
    ///
    /// **[PLAN_am30_integration]** Album processing flow:
    /// 1. Extract ID3 metadata for artist/album hints
    /// 2. Call AlbumMatcher.match_album()
    /// 3. Convert result to Vec<ProcessedPassage>
    async fn process_album_file(&self, file_path: &Path) -> Result<Vec<ProcessedPassage>> {
        info!("Processing file as album: {:?}", file_path);

        // Step 1: Extract ID3 metadata for hints
        let id3_result = ID3Extractor::new().extract_raw(file_path);
        let (artist_hint, album_hint) = match &id3_result {
            Ok(metadata) => (
                metadata.title.as_ref().map(|cv| cv.value.as_str()),
                metadata.album.as_ref().map(|cv| cv.value.as_str()),
            ),
            Err(e) => {
                warn!("Failed to extract ID3 for album hints: {}", e);
                (None, None)
            }
        };

        // Emit album matching started event
        self.emit_event(WorkflowEvent::AlbumMatchingStarted {
            file_path: file_path.to_string_lossy().to_string(),
            artist_hint: artist_hint.map(|s| s.to_string()),
            album_hint: album_hint.map(|s| s.to_string()),
        }).await;

        // Step 2: Create AlbumMatcher and run matching
        let album_matcher = match AlbumMatcher::new() {
            Ok(matcher) => matcher,
            Err(e) => {
                error!("Failed to create AlbumMatcher: {}", e);
                return self.album_match_fallback(file_path, &e.to_string()).await;
            }
        };

        let match_result = album_matcher
            .match_album(file_path, artist_hint, album_hint)
            .await;

        match match_result {
            Ok(result) if result.matched => {
                info!(
                    "Album matched: {} - {} ({:.1}% confidence)",
                    result.matched_artist.as_deref().unwrap_or("Unknown"),
                    result.matched_album.as_deref().unwrap_or("Unknown"),
                    result.match_percentage
                );

                // Check minimum match percentage
                if result.match_percentage < self.config.album_match_min_percentage {
                    warn!(
                        "Album match percentage {:.1}% below threshold {:.1}%",
                        result.match_percentage,
                        self.config.album_match_min_percentage
                    );
                    if self.config.album_match_fallback {
                        return self.album_match_fallback(
                            file_path,
                            &format!("Match percentage too low: {:.1}%", result.match_percentage)
                        ).await;
                    }
                }

                // Step 3: Convert to passages (Increment 4)
                self.convert_album_to_passages(file_path, result).await
            }
            Ok(result) => {
                // matched = false
                warn!("Album matching failed: {}", result.status);
                self.emit_event(WorkflowEvent::AlbumMatchingFailed {
                    file_path: file_path.to_string_lossy().to_string(),
                    reason: result.status.clone(),
                }).await;

                if self.config.album_match_fallback {
                    self.album_match_fallback(file_path, &result.status).await
                } else {
                    Err(anyhow::anyhow!("Album matching failed: {}", result.status))
                }
            }
            Err(AlbumMatchError::SingleTrackDetected { confidence, stage }) => {
                // Post-decode check detected single track
                info!(
                    "Album post-decode check: single track detected (confidence={:.2}, stage={})",
                    confidence, stage
                );
                // Process as single song
                self.process_as_single_song(file_path).await
            }
            Err(e) => {
                error!("Album matching error: {}", e);
                self.emit_event(WorkflowEvent::AlbumMatchingFailed {
                    file_path: file_path.to_string_lossy().to_string(),
                    reason: e.to_string(),
                }).await;

                if self.config.album_match_fallback {
                    self.album_match_fallback(file_path, &e.to_string()).await
                } else {
                    Err(anyhow::anyhow!("Album matching failed: {}", e))
                }
            }
        }
    }

    /// Fallback to single-song processing when album matching fails
    async fn album_match_fallback(
        &self,
        file_path: &Path,
        reason: &str,
    ) -> Result<Vec<ProcessedPassage>> {
        warn!("Falling back to single-song processing: {}", reason);
        self.emit_event(WorkflowEvent::AlbumMatchingFallback {
            file_path: file_path.to_string_lossy().to_string(),
            reason: reason.to_string(),
        }).await;

        self.process_as_single_song(file_path).await
    }

    /// Process file using existing single-song pipeline
    /// Extracted from original process_file() logic
    async fn process_as_single_song(&self, file_path: &Path) -> Result<Vec<ProcessedPassage>> {
        // This is the existing logic from process_file()
        // Phase 0: Detect passage boundaries with audio caching
        let file_audio = super::boundary_detector::detect_boundaries_with_audio(file_path)
            .await
            .context("Failed to detect passage boundaries")?;

        // ... rest of existing process_file() logic ...
        todo!("Refactor existing process_file logic into this method")
    }

    /// Convert AlbumMatchResult to passages (implemented in Increment 4)
    async fn convert_album_to_passages(
        &self,
        file_path: &Path,
        result: AlbumMatchResult,
    ) -> Result<Vec<ProcessedPassage>> {
        todo!("Implemented in Increment 4")
    }
}
```

---

## New Event Types

Add to `WorkflowEvent` enum:

```rust
/// Album matching process started
AlbumMatchingStarted {
    file_path: String,
    artist_hint: Option<String>,
    album_hint: Option<String>,
},

/// Album matching completed successfully
AlbumMatchingCompleted {
    file_path: String,
    release_mbid: String,
    artist: String,
    album: String,
    track_count: usize,
    match_percentage: f64,
    stage: String,
},

/// Album matching failed
AlbumMatchingFailed {
    file_path: String,
    reason: String,
},

/// Album matching falling back to single-song processing
AlbumMatchingFallback {
    file_path: String,
    reason: String,
},
```

---

## Tests to Pass

- **TC-U-003-01:** Album file produces AlbumMatchResult
- **TC-U-003-02:** Low match percentage triggers fallback
- **TC-U-003-03:** AlbumMatchError triggers fallback
- **TC-U-003-04:** SingleTrackDetected routes to single-song flow

---

## Acceptance Criteria

- [ ] `AlbumMatcher.match_album()` called for album files
- [ ] ID3 metadata extracted for artist/album hints
- [ ] Match percentage threshold enforced
- [ ] Errors handled with fallback option
- [ ] Events emitted for all state transitions

---

## Verification Command

```bash
cargo build -p wkmp-ai
cargo test -p wkmp-ai workflow::tests
```
