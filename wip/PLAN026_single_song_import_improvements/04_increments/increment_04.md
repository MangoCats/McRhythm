# Increment 4: ContentTypeClassifier Integration

**Estimated Effort:** 3-4 hours
**Dependencies:** Increment 1, 2, 3
**Tests:** TC-U-MB-010-01, TC-U-MB-010-02, TC-I-INT-010-01, TC-I-FUS-010-01

---

## Objective

Integrate RecordingMatcher into ContentTypeClassifier as fallback when AcoustID fails or returns low confidence.

---

## Deliverables

### 1. Modify: `src/services/content_type_classifier.rs`

Add RecordingMatcher field and update classify_single_song():

```rust
/// Content type classifier service
pub struct ContentTypeClassifier {
    acoustid_client: Arc<AcoustIDClient>,
    fingerprinter: Fingerprinter,
    album_matcher: AlbumMatcher,
    recording_matcher: RecordingMatcher,  // NEW
    db: SqlitePool,  // For caching
}

impl ContentTypeClassifier {
    pub fn new(
        acoustid_client: Arc<AcoustIDClient>,
        db: SqlitePool,
    ) -> Self {
        let mb_client = Arc::new(MusicBrainzClient::new().expect("MusicBrainz client"));
        Self {
            acoustid_client,
            fingerprinter: Fingerprinter::new(),
            album_matcher: AlbumMatcher::new().expect("AlbumMatcher"),
            recording_matcher: RecordingMatcher::new(mb_client.clone()),
            db,
        }
    }

    /// Enhanced single-song classification with MusicBrainz fallback
    pub async fn classify_single_song(
        &self,
        audio_path: &Path,
        duration_seconds: f64,
        metadata: Option<&AudioMetadata>,
    ) -> Result<ClassificationResult, ClassificationError> {
        // Step 1: Try AcoustID
        let acoustid_result = self.try_acoustid(audio_path, duration_seconds).await;

        // Step 2: If AcoustID succeeds with high confidence, use it
        if let Ok(ref result) = acoustid_result {
            if result.confidence_value >= HIGH_CONFIDENCE_THRESHOLD {
                return Ok(result.clone());
            }
        }

        // Step 3: Try MusicBrainz recording search as fallback
        if let Some(meta) = metadata {
            if let (Some(artist), Some(title)) = (&meta.artist, &meta.title) {
                let mb_result = self.try_recording_search(
                    artist, title, Some(duration_seconds)
                ).await;

                if let Ok(candidates) = mb_result {
                    if let Some(best) = candidates.first() {
                        // Have both AcoustID (low) and MB result
                        if let Ok(acoustid) = &acoustid_result {
                            return self.fuse_results(acoustid, best);
                        }
                        // Only MB result
                        return Ok(ClassificationResult::single_song(
                            best.mbid.clone(),
                            best.similarity,
                        ));
                    }
                }
            }
        }

        // Step 4: Return AcoustID result if available (even low confidence)
        if let Ok(result) = acoustid_result {
            return Ok(result);
        }

        // Step 5: No match found
        Ok(ClassificationResult::not_in_musicbrainz())
    }

    async fn try_acoustid(
        &self,
        audio_path: &Path,
        duration_seconds: f64,
    ) -> Result<ClassificationResult, ClassificationError> {
        // Existing AcoustID logic
        let fingerprint = self.fingerprinter.fingerprint_file(audio_path)
            .map_err(|e| ClassificationError::FingerprintError(e.to_string()))?;

        let response = self.acoustid_client.lookup(&fingerprint, duration_seconds as u64).await
            .map_err(|e| ClassificationError::AcoustIdError(e.to_string()))?;

        // ... existing processing
    }

    async fn try_recording_search(
        &self,
        artist: &str,
        title: &str,
        duration: Option<f64>,
    ) -> Result<Vec<RecordingCandidate>, ClassificationError> {
        self.recording_matcher
            .search_with_cache(artist, title, duration, &self.db)
            .await
            .map_err(|e| ClassificationError::MusicBrainzError(e.to_string()))
    }

    fn fuse_results(
        &self,
        acoustid: &ClassificationResult,
        mb: &RecordingCandidate,
    ) -> Result<ClassificationResult, ClassificationError> {
        // If they agree on MBID, boost confidence
        if acoustid.recording_mbid.as_ref() == Some(&mb.mbid) {
            let combined = 1.0 - (1.0 - acoustid.confidence_value) * (1.0 - mb.similarity);
            return Ok(ClassificationResult::single_song(mb.mbid.clone(), combined));
        }

        // If they disagree, prefer higher confidence
        if mb.similarity > acoustid.confidence_value {
            Ok(ClassificationResult::single_song(mb.mbid.clone(), mb.similarity))
        } else {
            Ok(acoustid.clone())
        }
    }
}
```

---

## Files Modified

| File | Action | Lines |
|------|--------|-------|
| `src/services/content_type_classifier.rs` | Modify | ~100 |

---

## Verification

- [ ] TC-U-MB-010-01 passes (fallback triggers on AcoustID failure)
- [ ] TC-U-MB-010-02 passes (fallback triggers on low confidence)
- [ ] TC-I-INT-010-01 passes (integrates with ContentTypeClassifier)
- [ ] TC-I-FUS-010-01 passes (IdentityResolver integration)
- [ ] Existing album classification tests still pass
- [ ] `cargo test content_type_classifier` passes

---

## Success Criteria

- AcoustID failure triggers MusicBrainz search
- Low AcoustID confidence triggers MusicBrainz search
- Both results fused when available
- No regression in album classification
