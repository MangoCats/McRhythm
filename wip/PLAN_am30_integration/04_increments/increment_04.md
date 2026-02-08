# Increment 4: Passage Conversion

**Increment:** 4 of 7
**Phase:** Implementation
**Estimated Effort:** 2-3 hours
**Confidence:** HIGH (±25%)
**Prerequisites:** Increment 3

---

## Objective

Implement `convert_album_to_passages()` to create `ProcessedPassage` objects from `AlbumMatchResult`.

---

## Deliverables

1. **Modified:** `wkmp-ai/src/workflow/pipeline.rs`
   - Implement `convert_album_to_passages()` method

---

## Requirements Covered

- **REQ-AM30-003:** Convert AlbumMatchResult to passages (complete)
- **REQ-AM30-009:** Track boundary to PassageBoundary conversion (complete)
- **REQ-AM30-013:** ConfidenceTier assignment for album tracks (complete)

---

## Implementation Notes

```rust
use crate::matching::{AlbumMatchResult, MatchedTrack, ConfidenceTier, assign_tier};
use crate::types::{FusedIdentity, FusedMetadata, FusedFlavor};

impl Pipeline {
    /// Convert AlbumMatchResult to Vec<ProcessedPassage>
    ///
    /// **[PLAN_am30_integration]** Creates one passage per matched track:
    /// 1. Calculate track boundaries (start/end times)
    /// 2. Create FusedIdentity with Recording MBID
    /// 3. Create FusedMetadata from track info
    /// 4. Create placeholder FusedFlavor (no AcousticBrainz data for album tracks)
    /// 5. Create ValidationResult based on match quality
    async fn convert_album_to_passages(
        &self,
        file_path: &Path,
        result: AlbumMatchResult,
    ) -> Result<Vec<ProcessedPassage>> {
        info!(
            "Converting album match to {} passages",
            result.tracks.len()
        );

        // Emit completion event
        self.emit_event(WorkflowEvent::AlbumMatchingCompleted {
            file_path: file_path.to_string_lossy().to_string(),
            release_mbid: result.release_mbid.clone().unwrap_or_default(),
            artist: result.matched_artist.clone().unwrap_or_default(),
            album: result.matched_album.clone().unwrap_or_default(),
            track_count: result.tracks.len(),
            match_percentage: result.match_percentage,
            stage: result.matching_stage.map(|s| s.to_string()).unwrap_or_default(),
        }).await;

        let mut passages = Vec::with_capacity(result.tracks.len());
        let mut accumulated_time = 0.0; // Track start time accumulator (seconds)

        // Assign confidence tier based on match quality
        let confidence_tier = self.assign_album_confidence_tier(&result);

        for (i, track) in result.tracks.iter().enumerate() {
            // Calculate boundary times
            let start_secs = accumulated_time;
            let end_secs = accumulated_time + track.detected_duration;
            accumulated_time = end_secs;

            // Convert seconds to ticks (SPEC017: 10,000 ticks per second)
            const TICKS_PER_SECOND: i64 = 10_000;
            let start_ticks = (start_secs * TICKS_PER_SECOND as f64) as i64;
            let end_ticks = (end_secs * TICKS_PER_SECOND as f64) as i64;

            // Create boundary
            let boundary = PassageBoundary {
                start_time: start_ticks,
                end_time: end_ticks,
                confidence: if track.within_tolerance { 0.95 } else { 0.7 },
            };

            // Emit boundary event
            self.emit_event(WorkflowEvent::BoundaryDetected {
                passage_index: i,
                start_time: start_ticks,
                end_time: end_ticks,
                confidence: boundary.confidence,
            }).await;

            // Create FusedIdentity with Recording MBID
            let identity = FusedIdentity {
                recording_mbid: Some(track.recording_mbid.clone()),
                confidence: result.match_percentage / 100.0,
                posterior_probability: result.match_percentage / 100.0,
                confidence_tier,
                conflicts: vec![],
            };

            // Create FusedMetadata from track info
            let metadata = FusedMetadata {
                title: Some(track.title.clone()),
                artist: result.matched_artist.clone(),
                album: result.matched_album.clone(),
                track_number: Some(track.track_number as u32),
                disc_number: Some(track.disc_number as u32),
                release_mbid: result.release_mbid.clone(),
                completeness: 0.9, // High completeness from MusicBrainz
                source: "AlbumMatcher".to_string(),
            };

            // Create placeholder FusedFlavor (no AcousticBrainz data)
            let flavor = FusedFlavor {
                characteristics: std::collections::HashMap::new(),
                completeness: 0.0,
                source: "none".to_string(),
            };

            // Create FusedPassage
            let fusion = FusedPassage {
                identity,
                metadata,
                flavor,
            };

            // Create ValidationResult based on track match quality
            let validation_score = if track.within_tolerance {
                0.9
            } else {
                0.6 + (1.0 - (track.timing_error / 10.0).min(0.4))
            };

            let validation = ValidationResult {
                score: validation_score as f32,
                status: if track.within_tolerance {
                    ValidationStatus::Valid
                } else {
                    ValidationStatus::Needs Review
                },
                issues: if track.within_tolerance {
                    vec![]
                } else {
                    vec![format!(
                        "Track timing error: {:.1}s (expected {:.1}s, detected {:.1}s)",
                        track.timing_error,
                        track.expected_duration,
                        track.detected_duration
                    )]
                },
            };

            // Emit passage events
            self.emit_event(WorkflowEvent::PassageStarted {
                passage_index: i,
                total_passages: result.tracks.len(),
            }).await;

            // Create ProcessedPassage
            let passage = ProcessedPassage {
                boundary,
                extractions: vec![], // No raw extractions for album tracks
                fusion,
                validation,
            };

            self.emit_event(WorkflowEvent::PassageCompleted {
                passage_index: i,
                quality_score: validation.score as f64,
                validation_status: format!("{:?}", validation.status),
            }).await;

            passages.push(passage);
        }

        info!(
            "Created {} passages from album match ({}% matched)",
            passages.len(),
            result.match_percentage
        );

        Ok(passages)
    }

    /// Assign confidence tier based on album match quality
    fn assign_album_confidence_tier(&self, result: &AlbumMatchResult) -> ConfidenceTier {
        // Album matching can achieve Tier2A-2B depending on match quality
        // Cannot achieve Tier1A/1B (those require embedded MBID)
        if result.match_percentage >= 95.0 && result.artist_verified {
            ConfidenceTier::Tier2A // High confidence match
        } else if result.match_percentage >= 80.0 {
            ConfidenceTier::Tier2B // Good match
        } else if result.match_percentage >= 60.0 {
            ConfidenceTier::Tier3 // Partial match
        } else {
            ConfidenceTier::Tier4 // Low confidence
        }
    }
}
```

---

## Data Mapping

| AlbumMatchResult Field | ProcessedPassage Location |
|------------------------|---------------------------|
| `tracks[i].recording_mbid` | `fusion.identity.recording_mbid` |
| `tracks[i].title` | `fusion.metadata.title` |
| `matched_artist` | `fusion.metadata.artist` |
| `matched_album` | `fusion.metadata.album` |
| `tracks[i].track_number` | `fusion.metadata.track_number` |
| `tracks[i].disc_number` | `fusion.metadata.disc_number` |
| `release_mbid` | `fusion.metadata.release_mbid` |
| `match_percentage` | `fusion.identity.confidence` |
| `tracks[i].detected_duration` | `boundary.end_time - boundary.start_time` |
| `tracks[i].within_tolerance` | `validation.status` |

---

## Tests to Pass

- **TC-U-004-01:** 10-track album produces 10 passages
- **TC-U-004-02:** Track boundaries are contiguous (no gaps)
- **TC-U-004-03:** Recording MBIDs correctly assigned
- **TC-U-004-04:** Track timing errors reflected in validation
- **TC-U-004-05:** Confidence tiers correctly assigned

---

## Acceptance Criteria

- [ ] One `ProcessedPassage` per `MatchedTrack`
- [ ] Boundaries calculated from detected durations
- [ ] Recording MBIDs from tracks assigned to identities
- [ ] Metadata populated from MusicBrainz data
- [ ] Validation scores reflect track match quality
- [ ] Events emitted for each passage

---

## Verification Command

```bash
cargo build -p wkmp-ai
cargo test -p wkmp-ai workflow
```
