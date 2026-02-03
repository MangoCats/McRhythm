# Increment 5: Multi-Source Fusion Integration

**Estimated Effort:** 2-3 hours
**Dependencies:** Increment 4
**Tests:** TC-I-FUS-020-01, TC-I-FUS-020-02, TC-U-FUS-030-01

---

## Objective

Integrate IdentityResolver for full Bayesian fusion of multiple identity sources.

---

## Deliverables

### 1. Enhance ContentTypeClassifier fusion logic

Replace simple fuse_results() with full IdentityResolver integration:

```rust
use crate::fusion::identity_resolver::IdentityResolver;
use crate::types::IdentityExtraction;

impl ContentTypeClassifier {
    /// Full multi-source fusion using IdentityResolver
    async fn fuse_all_sources(
        &self,
        acoustid_result: Option<&AcoustIDResponse>,
        mb_candidates: &[RecordingCandidate],
        metadata: Option<&AudioMetadata>,
    ) -> ClassificationResult {
        let mut sources: Vec<IdentityExtraction> = Vec::new();

        // Source 1: AcoustID
        if let Some(acoustid) = acoustid_result {
            if let Some(best) = acoustid.results.first() {
                if let Some(recordings) = &best.recordings {
                    if let Some(rec) = recordings.first() {
                        sources.push(IdentityExtraction {
                            recording_mbid: rec.id.clone(),
                            confidence: best.score as f32,
                            source: "AcoustID".to_string(),
                        });
                    }
                }
            }
        }

        // Source 2: MusicBrainz recording search
        if let Some(best_mb) = mb_candidates.first() {
            sources.push(IdentityExtraction {
                recording_mbid: best_mb.mbid.clone(),
                confidence: best_mb.similarity as f32,
                source: "MusicBrainz".to_string(),
            });
        }

        // Source 3: ID3 metadata (if we can correlate to MBID)
        // This requires existing MB result to verify
        if let Some(meta) = metadata {
            if let (Some(acoustid_mbid), Some(mb_mbid)) = (
                sources.first().map(|s| &s.recording_mbid),
                sources.get(1).map(|s| &s.recording_mbid)
            ) {
                // If metadata matches and both external sources agree
                if acoustid_mbid == mb_mbid {
                    sources.push(IdentityExtraction {
                        recording_mbid: acoustid_mbid.clone(),
                        confidence: 0.70, // ID3 metadata implicit confidence
                        source: "ID3".to_string(),
                    });
                }
            }
        }

        // Fuse all sources
        let resolver = IdentityResolver::new();
        match resolver.fuse(sources).await {
            Ok(fusion_result) => {
                if let Some(mbid) = fusion_result.output.recording_mbid {
                    ClassificationResult {
                        content_type: ContentType::SingleSong,
                        confidence: MatchConfidence::from_value(fusion_result.confidence as f64),
                        confidence_value: fusion_result.confidence as f64,
                        release_mbid: None,
                        recording_mbid: Some(mbid),
                        match_percentage: Some(100.0),
                        artist_verified: true,
                        matching_stage: Some("multi_source_fusion".to_string()),
                    }
                } else {
                    ClassificationResult::not_in_musicbrainz()
                }
            }
            Err(_) => ClassificationResult::identification_failed(),
        }
    }

    /// Updated classify_single_song with full fusion
    pub async fn classify_single_song_with_fusion(
        &self,
        audio_path: &Path,
        duration_seconds: f64,
        metadata: Option<&AudioMetadata>,
    ) -> Result<ClassificationResult, ClassificationError> {
        // Collect all sources in parallel
        let acoustid_result = self.try_acoustid(audio_path, duration_seconds).await.ok();

        let mb_candidates = if let Some(meta) = metadata {
            if let (Some(artist), Some(title)) = (&meta.artist, &meta.title) {
                self.try_recording_search(artist, title, Some(duration_seconds))
                    .await
                    .unwrap_or_default()
            } else {
                vec![]
            }
        } else {
            vec![]
        };

        // Fuse all available sources
        Ok(self.fuse_all_sources(
            acoustid_result.as_ref().and_then(|r| r.acoustid_response.as_ref()),
            &mb_candidates,
            metadata,
        ).await)
    }
}
```

---

## Files Modified

| File | Action | Lines |
|------|--------|-------|
| `src/services/content_type_classifier.rs` | Modify | ~80 |

---

## Verification

- [ ] TC-I-FUS-020-01 passes (three sources combined)
- [ ] TC-I-FUS-020-02 passes (agreeing sources boost confidence)
- [ ] TC-U-FUS-030-01 passes (Bayesian formula correct)
- [ ] `cargo test identity_resolver` passes
- [ ] `cargo test content_type_classifier` passes

---

## Success Criteria

- All available sources combined via IdentityResolver
- Bayesian posterior correctly calculated
- Agreeing sources boost confidence
- Conflicting sources documented
