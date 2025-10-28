# Testing Framework Tests - wkmp-ai

**Requirements:** AIA-TEST-010, AIA-TEST-020, AIA-TEST-030
**Priority:** P0 (Critical)
**Test Count:** 15

---

## TEST-081: FileScanner Unit Test Coverage >80%

**Requirement:** AIA-TEST-010
**Type:** Unit
**Priority:** P0

**Given:**
- FileScanner component with all public methods
- Unit test suite covering main functionality

**When:**
- Run code coverage analysis

**Then:**
- Line coverage ≥ 80%
- Branch coverage ≥ 70%
- All public methods tested
- Edge cases covered (empty directory, symlinks, permission denied)

**Acceptance Criteria:**
- ✅ Line coverage ≥ 80%
- ✅ All public methods have tests
- ✅ Error paths tested
- ✅ Edge cases covered

---

## TEST-082: MetadataExtractor Unit Test Coverage >80%

**Requirement:** AIA-TEST-010
**Type:** Unit
**Priority:** P0

**Given:**
- MetadataExtractor component
- Test fixtures for all supported formats

**When:**
- Run code coverage analysis

**Then:**
- Line coverage ≥ 80%
- All format parsers tested (MP3, FLAC, OGG, M4A, WAV)
- Missing metadata cases tested
- Invalid file handling tested

**Acceptance Criteria:**
- ✅ Line coverage ≥ 80%
- ✅ All 9 audio formats tested
- ✅ UTF-8 encoding edge cases tested
- ✅ Corrupt metadata handling tested

---

## TEST-083: Fingerprinter Unit Test Coverage >80%

**Requirement:** AIA-TEST-010
**Type:** Unit
**Priority:** P0

**Given:**
- Fingerprinter component (chromaprint wrapper)
- Sample audio PCM data

**When:**
- Run code coverage analysis

**Then:**
- Line coverage ≥ 80%
- Audio conversion tested (f32 → i16)
- Sample rate handling tested
- Error cases tested (invalid audio, empty buffer)

**Acceptance Criteria:**
- ✅ Line coverage ≥ 80%
- ✅ Fingerprint generation tested
- ✅ Deterministic output verified
- ✅ Error handling tested

---

## TEST-084: AmplitudeAnalyzer Unit Test Coverage >80%

**Requirement:** AIA-TEST-010
**Type:** Unit
**Priority:** P0

**Given:**
- AmplitudeAnalyzer component
- Synthetic test signals (fade-in, constant, fade-out)

**When:**
- Run code coverage analysis

**Then:**
- Line coverage ≥ 80%
- RMS calculation tested
- Lead-in/out detection tested
- A-weighting filter tested
- Various threshold values tested

**Acceptance Criteria:**
- ✅ Line coverage ≥ 80%
- ✅ RMS window calculation verified
- ✅ Lead-in/out boundary accuracy tested
- ✅ Edge cases (no fade, full silence) tested

---

## TEST-085: Database Queries Unit Test Coverage >80%

**Requirement:** AIA-TEST-010
**Type:** Unit
**Priority:** P0

**Given:**
- Database query module (IMPL014)
- In-memory SQLite database

**When:**
- Run code coverage analysis

**Then:**
- Line coverage ≥ 80%
- All query functions tested
- Upsert logic tested
- Batch insert tested
- Transaction helpers tested

**Acceptance Criteria:**
- ✅ Line coverage ≥ 80%
- ✅ All CRUD operations tested
- ✅ Cache operations tested
- ✅ Tick conversion helpers tested

---

## TEST-086: Mock AcoustID API Responses

**Requirement:** AIA-TEST-020
**Type:** Integration
**Priority:** P0

**Given:**
- Mock HTTP server simulating AcoustID API
- Test fingerprints with known responses

**When:**
- AcoustIDClient::lookup(fingerprint, duration)

**Then:**
- Mock returns predefined MBID
- Client parses response correctly
- Cache populated with result
- No actual network calls to acoustid.org

**Acceptance Criteria:**
- ✅ Mock server responds with valid JSON
- ✅ Best match extraction works
- ✅ Confidence threshold applied (≥ 0.5)
- ✅ No external API calls

---

## TEST-087: Mock MusicBrainz API Responses

**Requirement:** AIA-TEST-020
**Type:** Integration
**Priority:** P0

**Given:**
- Mock HTTP server simulating MusicBrainz API
- Known recording MBIDs with test responses

**When:**
- MusicBrainzClient::lookup_recording(mbid)

**Then:**
- Mock returns recording JSON (title, artists, work)
- Client parses all entities correctly
- Rate limiter bypassed (or mocked)
- Response cached

**Acceptance Criteria:**
- ✅ Mock server returns valid MB JSON
- ✅ Artist relationships parsed
- ✅ Work relationships parsed
- ✅ Album relationships handled

---

## TEST-088: Mock AcousticBrainz API Responses

**Requirement:** AIA-TEST-020
**Type:** Integration
**Priority:** P0

**Given:**
- Mock HTTP server simulating AcousticBrainz API
- Recording MBID with flavor data

**When:**
- AcousticBrainzClient::get_flavor(mbid)

**Then:**
- Mock returns musical flavor JSON
- Client deserializes flavor vector
- Missing data handled (404 → None)
- Response cached

**Acceptance Criteria:**
- ✅ Mock server returns flavor JSON
- ✅ Valid JSON deserialization
- ✅ Missing data returns None
- ✅ Cache populated correctly

---

## TEST-089: Mock Network Failures and Retries

**Requirement:** AIA-TEST-020
**Type:** Integration
**Priority:** P0

**Given:**
- Mock server configured to return network errors
- Retry policy: 3 attempts with exponential backoff

**When:**
- API client makes request
- Server returns 503, 503, then 200

**Then:**
- Client retries twice
- Third attempt succeeds
- Total delay matches backoff strategy (1s, 2s)
- Final result returned successfully

**Acceptance Criteria:**
- ✅ Retry logic triggered on network errors
- ✅ Exponential backoff timing verified
- ✅ Success after retries
- ✅ Max retries honored (3 attempts)

---

## TEST-090: Mock Rate Limit Enforcement

**Requirement:** AIA-TEST-020
**Type:** Integration
**Priority:** P0

**Given:**
- Mock MusicBrainz server with rate limit tracking
- Rate limit: 1 req/s

**When:**
- Make 5 rapid requests

**Then:**
- Mock records request timestamps
- Intervals between requests ≥ 1000ms
- No violations logged
- All requests succeed

**Acceptance Criteria:**
- ✅ Rate limiter enforces 1 req/s
- ✅ No requests faster than limit
- ✅ Timing accuracy ±50ms
- ✅ Concurrent safety verified

---

## TEST-091: E2E with Sample Library (10 Known Files)

**Requirement:** AIA-TEST-030
**Type:** End-to-End
**Priority:** P0

**Given:**
- Sample library: 10 audio files (2 × MP3, FLAC, OGG, M4A, WAV)
- Known metadata for all files
- Expected fingerprints pre-calculated

**When:**
- POST /import/start with root_folder
- Wait for completion

**Then:**
- All 10 files processed successfully
- Database contains:
  - 10 file records
  - 10+ passage records (if multi-passage files)
  - Correct metadata for each
  - Fingerprints match expected values
- No errors in import

**Acceptance Criteria:**
- ✅ 100% success rate (10/10 files)
- ✅ Metadata accuracy verified
- ✅ Fingerprints match expected
- ✅ Database state complete

---

## TEST-092: E2E with Multi-Passage File

**Requirement:** AIA-TEST-030
**Type:** End-to-End
**Priority:** P0

**Given:**
- Audio file with 3 songs separated by silence
- Expected passage boundaries known

**When:**
- Import file with silence_detection_enabled = true

**Then:**
- 3 passages created in database
- Passage boundaries accurate (±1s)
- Each passage:
  - Links to same file_id
  - Has correct start/end ticks
  - Has lead-in/out analysis
  - May link to different songs (if identified)

**Acceptance Criteria:**
- ✅ 3 passages detected
- ✅ Boundaries within tolerance
- ✅ All passages linked to file
- ✅ Independent song identification

---

## TEST-093: E2E with Various Audio Formats

**Requirement:** AIA-TEST-030
**Type:** End-to-End
**Priority:** P0

**Given:**
- Sample files in all 9 supported formats:
  - MP3 (CBR, VBR)
  - FLAC (16-bit, 24-bit)
  - OGG Vorbis
  - M4A (AAC)
  - WAV (PCM)
  - Opus
  - WMA
  - APE
  - ALAC

**When:**
- Import all files

**Then:**
- All formats processed successfully
- Fingerprints generated for all
- Metadata extracted correctly
- No format-specific errors

**Acceptance Criteria:**
- ✅ 100% format compatibility
- ✅ All fingerprints valid
- ✅ Metadata extraction per format
- ✅ No crashes or panics

---

## TEST-094: E2E with Error Recovery

**Requirement:** AIA-TEST-030
**Type:** End-to-End
**Priority:** P0

**Given:**
- Mixed library:
  - 5 valid audio files
  - 2 corrupt files (decode errors)
  - 1 file with missing metadata
  - 1 file with no MusicBrainz match

**When:**
- Import entire library

**Then:**
- State = COMPLETED (not FAILED)
- files_processed = 5
- files_failed = 2 (corrupt files)
- Warnings for missing metadata and no MBID
- Valid files fully imported
- Error details available in status endpoint

**Acceptance Criteria:**
- ✅ Import completes despite errors
- ✅ Valid files processed successfully
- ✅ Corrupt files skipped gracefully
- ✅ Error summary accurate

---

## TEST-095: E2E with Cache Population and Reuse

**Requirement:** AIA-TEST-030
**Type:** End-to-End
**Priority:** P0

**Given:**
- Sample library: 10 files
- Empty database (no cache)

**When:**
- First import: Populate cache
- Second import: Same 10 files

**Then:**
- **First import:**
  - External API calls: ~30 (AcoustID + MusicBrainz + AcousticBrainz per file)
  - Duration: ~3-5 minutes
  - Cache tables populated
- **Second import:**
  - External API calls: 0 (100% cache hit)
  - Duration: ~1-2 minutes (50% faster)
  - Database state identical

**Acceptance Criteria:**
- ✅ First import populates all 3 cache tables
- ✅ Second import uses cache exclusively
- ✅ Zero external API calls on second run
- ✅ Performance improvement >40%

---

## Test Implementation Notes

**Framework:** `cargo test --test testing_framework_tests -p wkmp-ai`

**Code Coverage Tool:**
```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Run coverage for wkmp-ai
cargo tarpaulin -p wkmp-ai --out Html --output-dir coverage/

# View report
firefox coverage/index.html
```

**Coverage Thresholds:**
```toml
# Cargo.toml
[package.metadata.tarpaulin]
min-coverage = 80
fail-under = 80
ignore-tests = true
```

**Mock Server Setup:**
```rust
use wiremock::{MockServer, Mock, ResponseTemplate};
use wiremock::matchers::{method, path};

#[tokio::test]
async fn test_mock_acoustid_api() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v2/lookup"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "results": [{
                "id": "recording-uuid",
                "score": 0.95
            }]
        })))
        .mount(&mock_server)
        .await;

    let client = AcoustIDClient::new_with_base_url(
        "test_api_key",
        mock_server.uri(),
        db.clone()
    );

    let result = client.lookup("test_fingerprint", 180).await.unwrap();
    assert_eq!(result.unwrap(), "recording-uuid");
}
```

**E2E Test Setup:**
```rust
#[tokio::test]
async fn test_e2e_sample_library() {
    // Create sample library
    let temp_dir = create_sample_library();

    // Initialize database
    let db = setup_test_db().await;

    // Start import server
    let app = create_app(db.clone()).await;
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Send import request
    let client = reqwest::Client::new();
    let response = client.post(format!("http://{}/import/start", addr))
        .json(&json!({
            "root_folder": temp_dir.path().to_str().unwrap()
        }))
        .send()
        .await
        .unwrap();

    let session: ImportSession = response.json().await.unwrap();

    // Poll for completion
    loop {
        let status: ImportStatus = client
            .get(format!("http://{}/import/status/{}", addr, session.session_id))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();

        if status.state == "COMPLETED" {
            assert_eq!(status.files_processed, 10);
            assert_eq!(status.files_failed, 0);
            break;
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    // Verify database state
    let file_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM files")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(file_count.0, 10);
}

fn create_sample_library() -> TempDir {
    let dir = TempDir::new().unwrap();

    // Copy test fixtures
    for file in ["sample1.mp3", "sample2.flac", "sample3.ogg", /*...*/] {
        std::fs::copy(
            format!("fixtures/{}", file),
            dir.path().join(file)
        ).unwrap();
    }

    dir
}
```

**Cache Testing:**
```rust
#[tokio::test]
async fn test_cache_population_and_reuse() {
    let db = setup_test_db().await;
    let temp_dir = create_sample_library();

    // First import
    let start = Instant::now();
    let session_id = start_import(&temp_dir).await;
    wait_for_completion(session_id).await;
    let first_duration = start.elapsed();

    // Verify cache populated
    let cache_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM acoustid_cache")
        .fetch_one(&db)
        .await
        .unwrap();
    assert!(cache_count.0 > 0, "Cache should be populated");

    // Second import (same files)
    let start = Instant::now();
    let session_id = start_import(&temp_dir).await;
    wait_for_completion(session_id).await;
    let second_duration = start.elapsed();

    // Verify performance improvement
    let speedup = first_duration.as_secs_f64() / second_duration.as_secs_f64();
    assert!(
        speedup >= 1.4,
        "Cache should provide >40% speedup, got {}x", speedup
    );
}
```

**Network Failure Simulation:**
```rust
#[tokio::test]
async fn test_network_retry_logic() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};

    let attempt_counter = Arc::new(AtomicU32::new(0));
    let counter_clone = attempt_counter.clone();

    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/ws/2/recording"))
        .respond_with(move |_req: &wiremock::Request| {
            let attempts = counter_clone.fetch_add(1, Ordering::SeqCst);

            if attempts < 2 {
                // First 2 attempts fail
                ResponseTemplate::new(503)
            } else {
                // Third attempt succeeds
                ResponseTemplate::new(200).set_body_json(json!({
                    "id": "recording-uuid",
                    "title": "Test Song"
                }))
            }
        })
        .mount(&mock_server)
        .await;

    let client = MusicBrainzClient::new_with_base_url(
        mock_server.uri(),
        db.clone()
    );

    let result = client.lookup_recording("test-mbid").await;

    assert!(result.is_ok(), "Should succeed after retries");
    assert_eq!(attempt_counter.load(Ordering::SeqCst), 3, "Should retry exactly twice");
}
```

---

End of testing framework tests
