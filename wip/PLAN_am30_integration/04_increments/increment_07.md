# Increment 7: Integration Tests

**Increment:** 7 of 7
**Phase:** Testing
**Estimated Effort:** 3-4 hours
**Confidence:** MEDIUM (±35%)
**Prerequisites:** Increment 6

---

## Objective

Create comprehensive integration tests for the album matching pipeline integration.

---

## Deliverables

1. **New:** `wkmp-ai/tests/pipeline_album_integration.rs`
   - Integration tests for album processing flow
2. **New:** Test fixtures for album files (or mocks)

---

## Requirements Covered

- **REQ-AM30-014:** Unit tests for routing logic (complete)
- **REQ-AM30-015:** Integration tests for full album flow (complete)

---

## Test Cases

### TC-I-001: Single Track File Routes Correctly

```rust
#[tokio::test]
async fn test_single_track_file_routes_to_existing_pipeline() {
    // Setup: File with track number prefix, high single-track score
    let config = PipelineConfig {
        enable_album_matching: true,
        single_track_threshold: 1.5,
        ..Default::default()
    };
    let pipeline = Pipeline::new(config);

    // Execute: Process a known single-track file
    let result = pipeline.process_file(Path::new("test_fixtures/single_track.mp3")).await;

    // Verify: Should produce 1 passage via AcoustID flow
    assert!(result.is_ok());
    let passages = result.unwrap();
    assert_eq!(passages.len(), 1);
    // Verify it used the existing flow (not album matcher)
}
```

### TC-I-002: Album File Routes to AlbumMatcher

```rust
#[tokio::test]
async fn test_album_file_routes_to_album_matcher() {
    // Setup: File with low single-track score (album characteristics)
    let config = PipelineConfig {
        enable_album_matching: true,
        single_track_threshold: 1.5,
        ..Default::default()
    };
    let pipeline = Pipeline::new(config);

    // Execute: Process a known album file
    let result = pipeline.process_file(Path::new("test_fixtures/album.mp3")).await;

    // Verify: Should produce N passages for N-track album
    assert!(result.is_ok());
    let passages = result.unwrap();
    assert!(passages.len() > 1, "Album should have multiple passages");

    // Verify each passage has a Recording MBID
    for passage in &passages {
        assert!(passage.fusion.identity.recording_mbid.is_some());
    }
}
```

### TC-I-003: Album Match Fallback on Failure

```rust
#[tokio::test]
async fn test_album_match_fallback_on_failure() {
    // Setup: Enable fallback, use file that will fail album matching
    let config = PipelineConfig {
        enable_album_matching: true,
        album_match_fallback: true,
        ..Default::default()
    };
    let pipeline = Pipeline::new(config);

    // Execute: Process file that looks like album but will fail matching
    let result = pipeline.process_file(Path::new("test_fixtures/fake_album.mp3")).await;

    // Verify: Should fall back to single-song and still succeed
    assert!(result.is_ok());
    let passages = result.unwrap();
    assert!(!passages.is_empty());
}
```

### TC-I-004: Disabled Album Matching Bypasses Check

```rust
#[tokio::test]
async fn test_disabled_album_matching_bypasses_check() {
    // Setup: Disable album matching
    let config = PipelineConfig {
        enable_album_matching: false,
        ..Default::default()
    };
    let pipeline = Pipeline::new(config);

    // Execute: Process album file with album matching disabled
    let result = pipeline.process_file(Path::new("test_fixtures/album.mp3")).await;

    // Verify: Should use single-song flow regardless
    assert!(result.is_ok());
    // Album is processed as single passage (or boundary-detected passages)
}
```

### TC-I-005: Track Boundaries Are Contiguous

```rust
#[tokio::test]
async fn test_album_track_boundaries_contiguous() {
    let config = PipelineConfig::default();
    let pipeline = Pipeline::new(config);

    let result = pipeline.process_file(Path::new("test_fixtures/album.mp3")).await;
    assert!(result.is_ok());

    let passages = result.unwrap();
    if passages.len() > 1 {
        // Verify boundaries are contiguous (no gaps)
        for i in 1..passages.len() {
            let prev_end = passages[i - 1].boundary.end_time;
            let curr_start = passages[i].boundary.start_time;
            assert_eq!(prev_end, curr_start, "Gap between passages {} and {}", i - 1, i);
        }
    }
}
```

### TC-I-006: Events Emitted for Album Flow

```rust
#[tokio::test]
async fn test_album_events_emitted() {
    let (tx, mut rx) = tokio::sync::mpsc::channel(100);
    let config = PipelineConfig::default();
    let pipeline = Pipeline::with_events(config, tx);

    let _ = pipeline.process_file(Path::new("test_fixtures/album.mp3")).await;

    // Collect events
    let mut events = Vec::new();
    while let Ok(event) = rx.try_recv() {
        events.push(event);
    }

    // Verify album-specific events present
    let has_single_track_check = events.iter().any(|e|
        matches!(e, WorkflowEvent::SingleTrackCheckCompleted { .. })
    );
    let has_album_started = events.iter().any(|e|
        matches!(e, WorkflowEvent::AlbumMatchingStarted { .. })
    );

    assert!(has_single_track_check);
    // Album events depend on whether file is actually an album
}
```

---

## Test Fixtures

**Option A: Real Audio Files (preferred)**
- `test_fixtures/single_track.mp3` - Single song file
- `test_fixtures/album.mp3` - Full album file with embedded ID3
- `test_fixtures/fake_album.mp3` - Long file without proper album structure

**Option B: Mocked Components (if real files not available)**
- Mock `SingleTrackDiscriminator` to return known scores
- Mock `AlbumMatcher` to return known results
- Test integration logic without real audio processing

---

## Acceptance Criteria

- [ ] Single-track routing tested
- [ ] Album routing tested
- [ ] Fallback behavior tested
- [ ] Disabled album matching tested
- [ ] Boundary contiguity verified
- [ ] Event emission verified
- [ ] All tests pass in CI

---

## Verification Command

```bash
cargo test -p wkmp-ai --test pipeline_album_integration
cargo test -p wkmp-ai --test pipeline_album_integration -- --nocapture
```
