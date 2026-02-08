# TC-S-E2E: End-to-End System Tests

**Requirements:** SSI-MB-010, SSI-FUS-010, SSI-FUS-020
**Type:** System Test

---

## TC-S-E2E-01: Single Song Classified When AcoustID Fails

**Environment:**
- wkmp-ai running with test database
- Mock external services (AcoustID returns failure, MB returns match)

**Scenario:** User imports single song file where AcoustID has no match

**Given:**
- Audio file: 3 minutes, ID3 tags (artist: "Queen", title: "Bohemian Rhapsody")
- AcoustID service returns no match
- MusicBrainz has recording for "Queen - Bohemian Rhapsody"

**When:**
- File is processed through ContentTypeClassifier

**Then:**
- File classified as SINGLE_SONG (not NOT_IN_MUSICBRAINZ)
- Recording MBID assigned from MusicBrainz search
- Confidence reflects MusicBrainz match quality

**Verify:**
```rust
#[tokio::test]
async fn test_e2e_classify_when_acoustid_fails() {
    // Setup: Create test file with ID3 tags
    let test_file = create_test_audio_file(
        "Queen",
        "Bohemian Rhapsody",
        180.0 // 3 minutes
    );

    // Setup: Mock AcoustID to fail
    let acoustid = MockAcoustIDClient::always_fails();

    // Setup: Mock MusicBrainz to return match
    let mb = MockMusicBrainzClient::with_recording(
        "Queen",
        "Bohemian Rhapsody",
        "mbid-queen-bohrap"
    );

    // Act
    let classifier = ContentTypeClassifier::new(acoustid, mb);
    let result = classifier.classify(&test_file, 180.0).await;

    // Assert
    assert_eq!(result.content_type, ContentType::SingleSong);
    assert_eq!(result.recording_mbid, Some("mbid-queen-bohrap".to_string()));
    assert!(result.confidence_value >= 0.80);
}
```

**Pass Criteria:**
- ContentType = SingleSong
- MBID assigned from MusicBrainz
- File not incorrectly marked as NOT_IN_MUSICBRAINZ

**Fail Criteria:**
- File marked NOT_IN_MUSICBRAINZ
- No MBID assigned
- AcoustID failure blocks import

---

## TC-S-E2E-02: Confidence Boosted When AcoustID and MB Agree

**Environment:**
- wkmp-ai running with test database
- Both AcoustID and MusicBrainz return same recording

**Scenario:** Both identification sources agree on song identity

**Given:**
- Audio file: 4 minutes, ID3 tags (artist: "Led Zeppelin", title: "Stairway to Heaven")
- AcoustID returns match with score 0.75 (below threshold alone)
- MusicBrainz returns same recording with similarity 0.88

**When:**
- File is processed with multi-source fusion

**Then:**
- Both sources combined using Bayesian fusion
- Final confidence higher than either individual source
- File classified as SINGLE_SONG with high confidence

**Verify:**
```rust
#[tokio::test]
async fn test_e2e_confidence_boosted_on_agreement() {
    // Setup: Create test file
    let test_file = create_test_audio_file(
        "Led Zeppelin",
        "Stairway to Heaven",
        480.0 // 8 minutes
    );

    // Setup: AcoustID returns moderate confidence (0.75)
    let acoustid = MockAcoustIDClient::with_result(
        "mbid-lz-stairway",
        0.75
    );

    // Setup: MusicBrainz returns same MBID with high similarity
    let mb = MockMusicBrainzClient::with_recording(
        "Led Zeppelin",
        "Stairway to Heaven",
        "mbid-lz-stairway"  // Same MBID!
    );

    // Act
    let classifier = ContentTypeClassifier::new(acoustid, mb);
    let result = classifier.classify_with_fusion(&test_file, 480.0).await;

    // Assert
    assert_eq!(result.content_type, ContentType::SingleSong);

    // Bayesian boost: 1 - (1-0.75)(1-0.88) = 1 - 0.25*0.12 = 0.97
    assert!(result.confidence_value > 0.90);
    assert!(result.confidence_value > 0.75); // Higher than AcoustID alone
    assert!(result.confidence_value > 0.88); // Higher than MB alone
}
```

**Pass Criteria:**
- Both sources used
- Bayesian fusion applied correctly
- Final confidence > max(individual confidences)

**Fail Criteria:**
- Only one source used
- No confidence boost
- Sources processed independently
