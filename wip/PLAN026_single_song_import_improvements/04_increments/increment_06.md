# Increment 6: Integration Tests and E2E Verification

**Estimated Effort:** 2-3 hours
**Dependencies:** Increments 1-5
**Tests:** TC-S-E2E-01, TC-S-E2E-02, TC-I-INT-020-01

---

## Objective

Create comprehensive integration and end-to-end tests to verify complete single-song import workflow.

---

## Deliverables

### 1. New File: `tests/single_song_integration_tests.rs`

```rust
//! Integration tests for single-song import improvements
//!
//! Tests the complete flow from file to classification including:
//! - AcoustID fallback to MusicBrainz
//! - Multi-source fusion
//! - Rate limiting compliance
//! - Cache behavior

use wkmp_ai::services::{
    ContentTypeClassifier,
    content_type_classifier::ContentType,
};

mod common;
use common::{
    create_test_db,
    create_mock_acoustid_client,
    create_mock_musicbrainz_client,
    create_test_audio_file,
};

#[tokio::test]
async fn test_e2e_classify_when_acoustid_fails() {
    // Setup
    let db = create_test_db().await;
    let acoustid = create_mock_acoustid_client().always_fails();
    let mb = create_mock_musicbrainz_client()
        .with_recording("Queen", "Bohemian Rhapsody", "mbid-queen-bohrap");

    let classifier = ContentTypeClassifier::new(Arc::new(acoustid), db.clone());

    // Create test file
    let test_file = create_test_audio_file("Queen", "Bohemian Rhapsody", 180.0);

    // Act
    let result = classifier.classify(&test_file.path(), 180.0).await;

    // Assert
    assert!(result.is_ok());
    let classification = result.unwrap();
    assert_eq!(classification.content_type, ContentType::SingleSong);
    assert_eq!(classification.recording_mbid, Some("mbid-queen-bohrap".to_string()));
}

#[tokio::test]
async fn test_e2e_confidence_boosted_on_agreement() {
    // Setup: Both sources return same MBID
    let db = create_test_db().await;
    let acoustid = create_mock_acoustid_client()
        .with_result("mbid-lz-stairway", 0.75);
    let mb = create_mock_musicbrainz_client()
        .with_recording("Led Zeppelin", "Stairway to Heaven", "mbid-lz-stairway");

    let classifier = ContentTypeClassifier::new(Arc::new(acoustid), db.clone());

    let test_file = create_test_audio_file("Led Zeppelin", "Stairway to Heaven", 480.0);

    // Act
    let result = classifier.classify_with_fusion(&test_file.path(), 480.0).await;

    // Assert
    assert!(result.is_ok());
    let classification = result.unwrap();

    // Bayesian boost: 1 - (1-0.75)(1-0.88) = 0.97
    assert!(classification.confidence_value > 0.90);
    assert!(classification.confidence_value > 0.75); // Higher than AcoustID alone
}

#[tokio::test]
async fn test_rate_limiting_respected() {
    use std::time::Instant;

    let db = create_test_db().await;
    let acoustid = create_mock_acoustid_client().always_fails();
    let mb = create_mock_musicbrainz_client().with_delay(true);

    let classifier = ContentTypeClassifier::new(Arc::new(acoustid), db.clone());

    // Make 3 sequential requests
    let start = Instant::now();
    for i in 0..3 {
        let test_file = create_test_audio_file(
            &format!("Artist {}", i),
            &format!("Song {}", i),
            180.0
        );
        let _ = classifier.classify(&test_file.path(), 180.0).await;
    }
    let elapsed = start.elapsed();

    // Should take at least 3 seconds (3 requests at 1/sec)
    assert!(elapsed.as_secs() >= 2, "Rate limiting not enforced");
}

#[tokio::test]
async fn test_cache_prevents_redundant_queries() {
    let db = create_test_db().await;
    let acoustid = create_mock_acoustid_client().always_fails();
    let mb = create_mock_musicbrainz_client()
        .with_recording("Test Artist", "Test Song", "mbid-test")
        .with_call_counter();

    let classifier = ContentTypeClassifier::new(Arc::new(acoustid), db.clone());

    let test_file = create_test_audio_file("Test Artist", "Test Song", 180.0);

    // First call - should hit API
    let _ = classifier.classify(&test_file.path(), 180.0).await;
    assert_eq!(mb.call_count(), 1);

    // Second call - should hit cache
    let _ = classifier.classify(&test_file.path(), 180.0).await;
    assert_eq!(mb.call_count(), 1); // No additional call
}
```

### 2. Test Utilities: `tests/common/mod.rs`

```rust
//! Common test utilities for integration tests

use sqlx::SqlitePool;
use std::sync::atomic::{AtomicUsize, Ordering};
use tempfile::NamedTempFile;

pub async fn create_test_db() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    // Run migrations
    sqlx::migrate!("./migrations").run(&pool).await.unwrap();
    pool
}

pub struct MockAcoustIDClient {
    result: Option<(String, f64)>,
    fails: bool,
}

impl MockAcoustIDClient {
    pub fn always_fails() -> Self {
        Self { result: None, fails: true }
    }

    pub fn with_result(mbid: &str, score: f64) -> Self {
        Self { result: Some((mbid.to_string(), score)), fails: false }
    }
}

// ... additional mock implementations
```

---

## Files Modified

| File | Action | Lines |
|------|--------|-------|
| `tests/single_song_integration_tests.rs` | Create | ~200 |
| `tests/common/mod.rs` | Create | ~100 |

---

## Verification

- [ ] TC-S-E2E-01 passes (AcoustID fails, MB succeeds)
- [ ] TC-S-E2E-02 passes (confidence boosted on agreement)
- [ ] TC-I-INT-020-01 passes (rate limits respected)
- [ ] `cargo test --test single_song_integration_tests` passes
- [ ] Coverage ≥80% achieved

---

## Success Criteria

- All integration tests pass
- End-to-end workflow verified
- Rate limiting confirmed
- Cache behavior verified
- Coverage target met
