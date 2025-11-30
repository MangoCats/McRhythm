# Increment 14: Integration Tests

**Estimated Effort:** 4 hours
**Dependencies:** Increment 13
**Deliverables:** tests/album_matcher_integration.rs

---

## Objective

Create comprehensive integration tests validating the complete album matching pipeline against real audio files.

---

## Tasks

### 14.1 Create Test Infrastructure

Create `wkmp-ai/tests/album_matcher_integration.rs`:

```rust
//! Album Matcher Integration Tests
//!
//! Tests the complete matching pipeline with real audio samples.

use wkmp_ai::services::AlbumMatcher;
use wkmp_ai::matching::types::*;
use std::path::PathBuf;

mod test_fixtures {
    use super::*;

    /// Get path to test audio fixture
    pub fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/audio")
            .join(name)
    }

    /// Check if test fixtures are available
    pub fn fixtures_available() -> bool {
        fixture_path("test_album.mp3").exists()
    }
}

/// Test fixture info for documentation
struct TestFixture {
    filename: &'static str,
    expected_artist: &'static str,
    expected_album: &'static str,
    expected_tracks: usize,
    expected_type: ContentType,
}

const TEST_FIXTURES: &[TestFixture] = &[
    TestFixture {
        filename: "known_album_10tracks.mp3",
        expected_artist: "Test Artist",
        expected_album: "Test Album",
        expected_tracks: 10,
        expected_type: ContentType::IdentifiedAlbum,
    },
    TestFixture {
        filename: "single_track.mp3",
        expected_artist: "Single Artist",
        expected_album: "Single",
        expected_tracks: 1,
        expected_type: ContentType::SingleTrack,
    },
];
```

### 14.2 Full Pipeline Tests

```rust
#[tokio::test]
#[ignore = "Requires test audio fixtures"]
async fn test_full_album_match_pipeline() {
    use test_fixtures::*;

    if !fixtures_available() {
        eprintln!("Skipping: test fixtures not available");
        return;
    }

    let matcher = AlbumMatcher::new(
        AlbumMatcherConfig::default(),
        create_test_mb_client().await,
    );

    let path = fixture_path("known_album_10tracks.mp3");
    let result = matcher.match_album(&path).await;

    assert!(result.is_ok(), "Match should succeed");
    let result = result.unwrap();

    assert_eq!(result.content_type, ContentType::IdentifiedAlbum);
    assert!(result.match_percentage.unwrap_or(0.0) >= 80.0);
    assert!(result.artist_verified);
}

#[tokio::test]
#[ignore = "Requires test audio fixtures"]
async fn test_single_track_rejection() {
    use test_fixtures::*;

    if !fixtures_available() {
        return;
    }

    let matcher = AlbumMatcher::new(
        AlbumMatcherConfig::default(),
        create_test_mb_client().await,
    );

    let path = fixture_path("single_track.mp3");
    let result = matcher.match_album(&path).await.unwrap();

    assert_eq!(result.content_type, ContentType::SingleTrack);
}

#[tokio::test]
#[ignore = "Requires test audio fixtures"]
async fn test_stage_progression() {
    use test_fixtures::*;

    if !fixtures_available() {
        return;
    }

    let matcher = AlbumMatcher::new(
        AlbumMatcherConfig {
            early_exit: false, // Test all stages
            ..Default::default()
        },
        create_test_mb_client().await,
    );

    let path = fixture_path("difficult_album.mp3");
    let result = matcher.match_album(&path).await.unwrap();

    // Verify stage was recorded
    assert!(result.matching_stage.is_some());
}
```

### 14.3 Error Handling Tests

```rust
#[tokio::test]
async fn test_missing_file_error() {
    let matcher = AlbumMatcher::new(
        AlbumMatcherConfig::default(),
        create_test_mb_client().await,
    );

    let result = matcher.match_album(Path::new("/nonexistent/file.mp3")).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_invalid_audio_error() {
    let matcher = AlbumMatcher::new(
        AlbumMatcherConfig::default(),
        create_test_mb_client().await,
    );

    // Create a non-audio file
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("not_audio.mp3");
    std::fs::write(&path, b"not audio content").unwrap();

    let result = matcher.match_album(&path).await;
    assert!(result.is_err());
}

#[tokio::test]
#[ignore = "Requires network"]
async fn test_musicbrainz_no_results() {
    let matcher = AlbumMatcher::new(
        AlbumMatcherConfig::default(),
        create_test_mb_client().await,
    );

    // Use fixture with nonsense metadata
    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("unknown.mp3");
    // Would need to create test audio with garbage metadata

    // Expected: IdentificationFailed
}
```

### 14.4 Performance Tests

```rust
#[tokio::test]
#[ignore = "Performance test - requires fixtures"]
async fn test_matching_performance() {
    use test_fixtures::*;
    use std::time::Instant;

    if !fixtures_available() {
        return;
    }

    let matcher = AlbumMatcher::new(
        AlbumMatcherConfig::default(),
        create_test_mb_client().await,
    );

    let path = fixture_path("known_album_10tracks.mp3");

    let start = Instant::now();
    let _ = matcher.match_album(&path).await;
    let duration = start.elapsed();

    // Should complete within reasonable time (adjust based on audio length)
    // ~45 minutes of audio should process in <60 seconds
    assert!(duration.as_secs() < 60, "Matching took too long: {:?}", duration);
}
```

---

## Test Data Requirements

### Required Test Fixtures

| Fixture | Description | Purpose |
|---------|-------------|---------|
| known_album_10tracks.mp3 | 10-track album with clean silences | Full pipeline test |
| single_track.mp3 | Single 5-minute track | Single-track rejection |
| difficult_album.mp3 | Album with quiet transitions | Stage progression test |
| noisy_album.mp3 | Album with no clear silences | Stage 4 test |
| bonus_tracks.mp3 | Album with hidden track | Stage 5 test |

### Test Fixture Generation

```rust
/// Script to generate test fixtures (run separately)
/// Uses existing audio library files with known MusicBrainz data
fn generate_test_fixtures() {
    // This would be a separate utility
}
```

---

## Tests

| Test ID | Description | Pass Criteria |
|---------|-------------|---------------|
| TC-I-014-01 | Full pipeline match | Identifies known album |
| TC-I-014-02 | Single-track rejection | Rejects single tracks |
| TC-I-014-03 | Stage progression | All stages accessible |
| TC-I-014-04 | Missing file error | Graceful error |
| TC-I-014-05 | Invalid audio error | Graceful error |
| TC-I-014-06 | No MusicBrainz results | Returns IdentificationFailed |
| TC-S-014-01 | Performance benchmark | <60s for 45min audio |

---

## Acceptance Criteria

- [ ] Integration test file created
- [ ] Test fixtures documented
- [ ] Full pipeline test passes
- [ ] Error handling tests pass
- [ ] Performance baseline established
- [ ] All 7 tests pass (or skipped with reason)
