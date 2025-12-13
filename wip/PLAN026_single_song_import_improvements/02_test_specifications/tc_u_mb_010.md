# TC-U-MB-010: Recording Matcher Fallback Trigger

**Requirement:** SSI-MB-010 (MusicBrainz recording search fallback)
**Type:** Unit Test

---

## TC-U-MB-010-01: Triggers When AcoustID Fails

**Given:**
- Audio file with valid metadata (artist: "The Beatles", title: "Yesterday")
- AcoustID lookup returns `Err(AcoustIDError::NoMatches)`

**When:**
- `classify_single_song()` is called

**Then:**
- RecordingMatcher.search() is invoked
- Query includes artist and title from metadata

**Verify:**
```rust
#[tokio::test]
async fn test_fallback_on_acoustid_failure() {
    let mut mock_acoustid = MockAcoustIDClient::new();
    mock_acoustid.expect_lookup()
        .returning(|_, _| Err(AcoustIDError::NoMatches));

    let mut mock_mb = MockMusicBrainzClient::new();
    mock_mb.expect_search_recordings()
        .with(predicate::str::contains("Beatles"))
        .times(1)
        .returning(|_, _| Ok(mock_recording_response()));

    let classifier = ContentTypeClassifier::new(mock_acoustid, mock_mb);
    let result = classifier.classify_single_song(&path, 180.0).await;

    assert!(result.is_ok());
}
```

**Pass Criteria:** MusicBrainz recording search is called when AcoustID fails
**Fail Criteria:** File marked NOT_IN_MUSICBRAINZ without attempting MB search

---

## TC-U-MB-010-02: Triggers When AcoustID Confidence < 0.80

**Given:**
- Audio file with valid metadata
- AcoustID lookup returns result with score = 0.65 (below 0.80 threshold)

**When:**
- `classify_single_song()` is called

**Then:**
- RecordingMatcher.search() is invoked as fallback
- AcoustID result is NOT used as primary (low confidence)

**Verify:**
```rust
#[tokio::test]
async fn test_fallback_on_low_acoustid_confidence() {
    let mut mock_acoustid = MockAcoustIDClient::new();
    mock_acoustid.expect_lookup()
        .returning(|_, _| Ok(AcoustIDResponse {
            status: "ok".to_string(),
            results: vec![AcoustIDResult {
                id: "aid-123".to_string(),
                score: 0.65,  // Below 0.80 threshold
                recordings: Some(vec![mock_recording()]),
            }],
        }));

    let mut mock_mb = MockMusicBrainzClient::new();
    mock_mb.expect_search_recordings()
        .times(1)
        .returning(|_, _| Ok(mock_recording_response()));

    let classifier = ContentTypeClassifier::new(mock_acoustid, mock_mb);
    let result = classifier.classify_single_song(&path, 180.0).await;

    // Should attempt MB search as fallback
    assert!(result.is_ok());
}
```

**Pass Criteria:** MB search triggered when AcoustID score < 0.80
**Fail Criteria:** Low-confidence AcoustID result accepted as final
