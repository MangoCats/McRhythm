# PLAN013 Phase 3: Acceptance Test Definition

**Specification:** docs/IMPL012-acoustid_client.md (v1.1 - corrected)
**Date:** 2025-10-30
**Status:** Phase 3 Complete

---

## Executive Summary

**Phase 3 Goal:** Define acceptance tests for all requirements and create traceability matrix.

**Test Coverage:** 18 requirements → 24 acceptance tests (100% coverage)

**Test Strategy:**
- Unit tests (automated): Component-level testing with mocks
- Integration tests (manual): End-to-end testing with real audio files
- Test-first approach: Write tests before implementation

---

## Acceptance Test Catalog

### Audio Processing Pipeline Tests

**AT-FP-010-01:** Audio Decoding - Valid File
- **Requirement:** REQ-FP-010 (Audio decoding with Symphonia)
- **Test:** Decode valid MP3 file to PCM samples
- **Given:** Valid MP3 file with known properties
- **When:** `decode_audio()` called
- **Then:** Returns AudioData with correct sample_rate and non-empty samples
- **Type:** Unit test (mock file I/O)

**AT-FP-010-02:** Audio Decoding - Unsupported Format
- **Requirement:** REQ-FP-010
- **Test:** Handle unsupported audio codec gracefully
- **Given:** File with unsupported codec
- **When:** `decode_audio()` called
- **Then:** Returns `FingerprintError::DecodeError` with meaningful message
- **Type:** Unit test (mock Symphonia error)

**AT-FP-010-03:** Audio Decoding - No Audio Track
- **Requirement:** REQ-FP-010
- **Test:** Handle file with no audio track
- **Given:** File with no default audio track
- **When:** `decode_audio()` called
- **Then:** Returns `FingerprintError::DecodeError("No audio track")`
- **Type:** Unit test (mock Symphonia)

**AT-FP-020-01:** Audio Resampling - 48kHz to 44.1kHz
- **Requirement:** REQ-FP-020 (Resample to 44.1kHz mono)
- **Test:** Resample 48kHz stereo audio to 44.1kHz mono
- **Given:** AudioData with 48000 Hz, 2-channel PCM
- **When:** `resample_to_44100()` called
- **Then:** Returns mono audio at 44100 Hz with correct sample count
- **Type:** Unit test (real Rubato)

**AT-FP-020-02:** Audio Resampling - Already 44.1kHz
- **Requirement:** REQ-FP-020
- **Test:** Skip resampling if already 44.1kHz
- **Given:** AudioData with 44100 Hz
- **When:** `resample_to_44100()` called
- **Then:** Returns input unchanged (optimization check)
- **Type:** Unit test

**AT-FP-020-03:** Audio Resampling - Stereo to Mono
- **Requirement:** REQ-FP-020
- **Test:** Convert stereo to mono by channel averaging
- **Given:** Stereo PCM data with distinct L/R channels
- **When:** `convert_to_mono_f32()` called
- **Then:** Returns mono data with averaged samples
- **Type:** Unit test

**AT-FP-030-01:** Chromaprint Fingerprinting - Valid Audio
- **Requirement:** REQ-FP-030 (Generate fingerprint)
- **Test:** Generate valid Chromaprint fingerprint from PCM
- **Given:** Valid 44.1kHz mono PCM samples
- **When:** `generate_fingerprint()` called
- **Then:** Returns Base64 string starting with "AQAD"
- **Type:** Integration test (manual, real Chromaprint FFI)

**AT-FP-030-02:** Chromaprint Fingerprinting - Empty Audio
- **Requirement:** REQ-FP-030
- **Test:** Handle empty PCM samples
- **Given:** Empty sample array
- **When:** `generate_fingerprint()` called
- **Then:** Returns `FingerprintError::ChromaprintError` or valid empty fingerprint
- **Type:** Unit test (mock FFI)

**AT-FP-040-01:** Error Handling - I/O Error
- **Requirement:** REQ-FP-040 (Fingerprinting error handling)
- **Test:** Propagate file I/O errors with context
- **Given:** Non-existent file path
- **When:** `fingerprint_file()` called
- **Then:** Returns `FingerprintError::IoError` with path context
- **Type:** Unit test

### AcoustID API Client Tests

**AT-AC-010-01:** API Communication - Successful Lookup
- **Requirement:** REQ-AC-010 (API communication)
- **Test:** POST fingerprint to AcoustID API and receive response
- **Given:** Valid fingerprint and API key
- **When:** `lookup()` called
- **Then:** Returns AcoustIDResponse with results
- **Type:** Unit test (mock HTTP client)

**AT-AC-010-02:** API Communication - Network Timeout
- **Requirement:** REQ-AC-010
- **Test:** Handle network timeout (30s)
- **Given:** Slow/unresponsive API endpoint
- **When:** `lookup()` called
- **Then:** Returns `AcoustIDError::NetworkError` after 30s timeout
- **Type:** Unit test (mock HTTP timeout)

**AT-AC-020-01:** Response Parsing - Valid JSON
- **Requirement:** REQ-AC-020 (Parse response)
- **Test:** Parse valid AcoustID JSON response
- **Given:** Valid JSON with status="ok" and results array
- **When:** Response deserialized
- **Then:** Returns populated AcoustIDResponse struct
- **Type:** Unit test

**AT-AC-020-02:** Response Parsing - Invalid JSON
- **Requirement:** REQ-AC-020
- **Test:** Handle malformed JSON response
- **Given:** Invalid JSON from API
- **When:** Response parsing attempted
- **Then:** Returns `AcoustIDError::ParseError`
- **Type:** Unit test

**AT-AC-030-01:** MBID Selection - High Score Match
- **Requirement:** REQ-AC-030 (MBID selection with score threshold)
- **Test:** Extract MBID from result with score ≥ 0.5
- **Given:** Response with result score=0.95
- **When:** `get_best_mbid()` called
- **Then:** Returns first recording MBID
- **Type:** Unit test (already exists in acoustid_client.rs)

**AT-AC-030-02:** MBID Selection - Low Score Rejection
- **Requirement:** REQ-AC-030
- **Test:** Reject results below 0.5 score threshold
- **Given:** Response with best result score=0.3
- **When:** `get_best_mbid()` called
- **Then:** Returns None
- **Type:** Unit test (already exists in acoustid_client.rs)

**AT-AC-040-01:** API Error Handling - Invalid API Key
- **Requirement:** REQ-AC-040 (API error handling)
- **Test:** Handle 401 Unauthorized (invalid API key)
- **Given:** Invalid API key
- **When:** `lookup()` called
- **Then:** Returns `AcoustIDError::InvalidApiKey`
- **Type:** Unit test (mock 401 response)

### Caching Layer Tests

**AT-CA-010-01:** Cache Lookup - Cache Hit
- **Requirement:** REQ-CA-010 (Cache lookup)
- **Test:** Return cached MBID without API call
- **Given:** Fingerprint previously cached in database
- **When:** `lookup()` called
- **Then:** Returns cached MBID, no HTTP request made
- **Type:** Unit test (mock database)

**AT-CA-010-02:** Cache Lookup - Cache Miss
- **Requirement:** REQ-CA-010
- **Test:** Query API on cache miss
- **Given:** Fingerprint not in cache
- **When:** `lookup()` called
- **Then:** Makes API call and returns result
- **Type:** Unit test (mock database + HTTP)

**AT-CA-020-01:** Cache Storage - Successful Insert
- **Requirement:** REQ-CA-020 (Cache storage)
- **Test:** Store fingerprint → MBID mapping
- **Given:** Successful API lookup result
- **When:** `cache_mbid()` called
- **Then:** Database contains mapping with cached_at timestamp
- **Type:** Unit test (real SQLite in-memory)

**AT-CA-020-02:** Cache Storage - UPSERT Conflict
- **Requirement:** REQ-CA-020
- **Test:** Update existing cache entry on conflict
- **Given:** Fingerprint hash already exists in cache
- **When:** `cache_mbid()` called with new MBID
- **Then:** Existing row updated, cached_at refreshed
- **Type:** Unit test (real SQLite in-memory)

**AT-CA-030-01:** Fingerprint Hashing - Determinism
- **Requirement:** REQ-CA-030 (Fingerprint hashing)
- **Test:** Same fingerprint produces same hash
- **Given:** Identical fingerprint string
- **When:** `hash_fingerprint()` called twice
- **Then:** Returns identical 64-character hex string
- **Type:** Unit test (already exists in acoustid_client.rs tests)

**AT-CA-030-02:** Fingerprint Hashing - Uniqueness
- **Requirement:** REQ-CA-030
- **Test:** Different fingerprints produce different hashes
- **Given:** Two different fingerprint strings
- **When:** `hash_fingerprint()` called for each
- **Then:** Returns different hashes
- **Type:** Unit test (already exists in acoustid_client.rs tests)

### Performance Tests

**AT-PERF-030-01:** Rate Limiting - 3 Requests/Second
- **Requirement:** REQ-PERF-030 (Rate limiting)
- **Test:** Enforce 334ms minimum interval between requests
- **Given:** Multiple consecutive lookup calls
- **When:** Rate limiter enforced
- **Then:** Requests spaced at least 334ms apart
- **Type:** Unit test (already exists in acoustid_client.rs tests)

### Build Tests

**AT-BLD-010-01:** LLVM Dependency - Build Success
- **Requirement:** REQ-BLD-010 (LLVM/Clang dependency)
- **Test:** Verify chromaprint-sys-next builds with LLVM
- **Given:** LLVM/Clang installed on build system
- **When:** `cargo build` executed
- **Then:** chromaprint-sys-next compiles without errors
- **Type:** Build verification (CI)

---

## Traceability Matrix

| Requirement ID | Test IDs | Coverage |
|---|---|---|
| REQ-FP-010 | AT-FP-010-01, AT-FP-010-02, AT-FP-010-03 | ✅ 100% |
| REQ-FP-020 | AT-FP-020-01, AT-FP-020-02, AT-FP-020-03 | ✅ 100% |
| REQ-FP-030 | AT-FP-030-01, AT-FP-030-02 | ✅ 100% |
| REQ-FP-040 | AT-FP-040-01 | ✅ 100% |
| REQ-AC-010 | AT-AC-010-01, AT-AC-010-02 | ✅ 100% |
| REQ-AC-020 | AT-AC-020-01, AT-AC-020-02 | ✅ 100% |
| REQ-AC-030 | AT-AC-030-01, AT-AC-030-02 | ✅ 100% |
| REQ-AC-040 | AT-AC-040-01 | ✅ 100% |
| REQ-CA-010 | AT-CA-010-01, AT-CA-010-02 | ✅ 100% |
| REQ-CA-020 | AT-CA-020-01, AT-CA-020-02 | ✅ 100% |
| REQ-CA-030 | AT-CA-030-01, AT-CA-030-02 | ✅ 100% |
| REQ-BLD-010 | AT-BLD-010-01 | ✅ 100% |
| REQ-PERF-010 | *(Performance benchmark, not automated test)* | ⚠️ Manual |
| REQ-PERF-020 | *(Memory profiling, not automated test)* | ⚠️ Manual |
| REQ-PERF-030 | AT-PERF-030-01 | ✅ 100% |
| REQ-PERF-040 | *(Cache effectiveness measured via metrics, not test)* | ⚠️ Manual |
| REQ-TEST-010 | *(Meta-requirement: unit tests exist)* | ✅ Satisfied |
| REQ-TEST-020 | *(Meta-requirement: integration tests exist)* | ✅ Satisfied |

**Coverage Summary:**
- Automated tests: 12/18 requirements (67%)
- Manual verification: 4/18 requirements (22%)
- Meta-requirements: 2/18 requirements (11%)
- **Total:** 18/18 requirements covered (100%)

---

## Test Implementation Status

### Already Implemented (acoustid_client.rs tests)

✅ **AT-AC-030-01:** test_get_best_mbid() (lines 238-255)
✅ **AT-AC-030-02:** test_get_best_mbid_low_score() (lines 257-266)
✅ **AT-CA-030-01:** test_fingerprint_hash() - determinism (lines 620-630)
✅ **AT-PERF-030-01:** test_rate_limiter_allows_3_per_second() (lines 218-235)

**Status:** 4/24 tests already exist (17%)

### To Be Implemented

**Fingerprinting Pipeline (11 tests):**
- AT-FP-010-01, AT-FP-010-02, AT-FP-010-03 (decode_audio tests)
- AT-FP-020-01, AT-FP-020-02, AT-FP-020-03 (resample tests)
- AT-FP-030-01, AT-FP-030-02 (Chromaprint tests)
- AT-FP-040-01 (error handling)

**API Client (5 tests):**
- AT-AC-010-01, AT-AC-010-02 (HTTP communication)
- AT-AC-020-01, AT-AC-020-02 (JSON parsing)
- AT-AC-040-01 (API errors)

**Caching Layer (4 tests):**
- AT-CA-010-01, AT-CA-010-02 (cache lookup)
- AT-CA-020-01, AT-CA-020-02 (cache storage)
- AT-CA-030-02 (hash uniqueness)

**Build (1 test):**
- AT-BLD-010-01 (LLVM dependency verification)

**Status:** 20/24 tests to implement (83%)

---

## Test-First Implementation Workflow

**For Each Increment:**

1. **Write Acceptance Tests First**
   - Implement all AT-* tests for requirements in increment
   - Tests should fail initially (no implementation)

2. **Implement Feature**
   - Write minimum code to make tests pass
   - Follow TDD red-green-refactor cycle

3. **Verify Coverage**
   - Confirm all tests pass
   - Check traceability matrix (requirement → test → pass)

4. **Refactor**
   - Improve code quality while maintaining test coverage
   - Run tests after each refactor to ensure no regression

---

## Test Organization

### File Structure

```
wkmp-ai/src/services/
├── fingerprinter.rs
│   └── #[cfg(test)] mod tests {
│       ├── test_decode_audio_valid_file()        // AT-FP-010-01
│       ├── test_decode_audio_unsupported()       // AT-FP-010-02
│       ├── test_decode_audio_no_track()          // AT-FP-010-03
│       ├── test_resample_48k_to_44k()            // AT-FP-020-01
│       ├── test_resample_already_44k()           // AT-FP-020-02
│       ├── test_convert_stereo_to_mono()         // AT-FP-020-03
│       ├── test_generate_fingerprint_valid()     // AT-FP-030-01 (manual)
│       ├── test_generate_fingerprint_empty()     // AT-FP-030-02
│       └── test_io_error_propagation()           // AT-FP-040-01
│   }
│
├── acoustid_client.rs
│   └── #[cfg(test)] mod tests {
│       ├── test_lookup_successful()              // AT-AC-010-01
│       ├── test_lookup_network_timeout()         // AT-AC-010-02
│       ├── test_parse_valid_json()               // AT-AC-020-01
│       ├── test_parse_invalid_json()             // AT-AC-020-02
│       ├── test_get_best_mbid()                  // AT-AC-030-01 ✅ EXISTS
│       ├── test_get_best_mbid_low_score()        // AT-AC-030-02 ✅ EXISTS
│       ├── test_invalid_api_key()                // AT-AC-040-01
│       ├── test_cache_hit()                      // AT-CA-010-01
│       ├── test_cache_miss()                     // AT-CA-010-02
│       ├── test_cache_insert()                   // AT-CA-020-01
│       ├── test_cache_upsert()                   // AT-CA-020-02
│       ├── test_fingerprint_hash()               // AT-CA-030-01 ✅ EXISTS
│       ├── test_hash_uniqueness()                // AT-CA-030-02
│       └── test_rate_limiter_allows_3_per_second() // AT-PERF-030-01 ✅ EXISTS
│   }
```

---

## Mock Strategy

### Mocked Components

**For Unit Tests:**
- Symphonia `FormatReader`: Mock with test PCM data
- Chromaprint FFI: Mock with dummy fingerprints
- HTTP Client (reqwest): Mock with test responses
- Database (SQLite): Use in-memory database (`:memory:`)

**For Integration Tests:**
- No mocks - use real components with test data

### Test Data Fixtures

**Audio Data:**
```rust
fn mock_audio_data_44k_mono() -> AudioData {
    AudioData {
        samples: vec![0.0; 44100], // 1 second of silence
        sample_rate: 44100,
    }
}

fn mock_audio_data_48k_stereo() -> Vec<[f32; 2]> {
    vec![[0.5, -0.5]; 48000] // 1 second stereo
}
```

**AcoustID Responses:**
```rust
fn mock_acoustid_response_high_score() -> AcoustIDResponse {
    AcoustIDResponse {
        status: "ok".to_string(),
        results: vec![AcoustIDResult {
            id: "acoustid-uuid".to_string(),
            score: 0.95,
            recordings: Some(vec![AcoustIDRecording {
                id: "mbid-uuid".to_string(),
                title: Some("Test Song".to_string()),
                artists: None,
                duration: Some(180),
            }]),
        }],
    }
}
```

---

## Integration Test Execution

**Manual Test Checklist:**

1. **Prerequisites:**
   - [ ] LLVM/Clang installed on build system
   - [ ] Test audio file selected (MP3, FLAC, or OGG)
   - [ ] Environment variable `WKMP_TEST_AUDIO_FILE` set

2. **Run Integration Tests:**
   ```bash
   export WKMP_TEST_AUDIO_FILE="/path/to/audio.mp3"
   cargo test --ignored -- test_fingerprint_real_file
   ```

3. **Verify Output:**
   - [ ] Fingerprint generated successfully
   - [ ] Fingerprint starts with "AQAD"
   - [ ] Fingerprint length reasonable (typically 500-5000 chars)

4. **Performance Check:**
   - [ ] Processing time < 5 seconds for 3-minute audio file
   - [ ] Memory usage < 100MB during fingerprinting

---

## Acceptance Criteria Summary

**Increment Complete When:**
- [ ] All acceptance tests for increment pass
- [ ] Traceability matrix shows 100% coverage for increment requirements
- [ ] No test marked `#[ignore]` unless documented as manual test
- [ ] Code coverage ≥ 80% for implemented code (measured via `cargo tarpaulin` or similar)
- [ ] Manual integration test passes with real audio file
- [ ] No compiler warnings in test code

---

## Phase 3 Completeness Checklist

- [✅] Defined 24 acceptance tests for 18 requirements
- [✅] Created traceability matrix (100% coverage)
- [✅] Identified 4 existing tests (17% already implemented)
- [✅] Organized tests by component (fingerprinter.rs, acoustid_client.rs)
- [✅] Defined mock strategy for unit tests
- [✅] Documented test-first workflow
- [✅] Created integration test execution checklist
- [✅] Defined increment completion criteria

---

## Next Steps

**Phase 3 Status:** ✅ COMPLETE

**Phase 4 Actions:**
1. Break implementation into increments
2. Assign requirements to increments
3. Define increment boundaries (working intermediate states)
4. Create implementation order with test checkpoints
5. Estimate effort per increment

---

**Phase 3 Complete:** All requirements have defined acceptance tests, ready for implementation planning.
