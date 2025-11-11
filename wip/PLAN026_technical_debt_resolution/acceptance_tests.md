# Acceptance Test Matrix - PLAN026: Technical Debt Resolution

**Test Coverage Goal:** 100% requirement verification via executable tests

---

## Sprint 1: Critical Requirements

### REQ-TD-001: Functional Boundary Detection

#### TC-TD-001-01: Multi-Track Album Detection
**Requirement:** REQ-TD-001 SHALL-1 (detect multiple passages within single audio file)
**Priority:** CRITICAL
**Type:** Integration Test

**Test Setup:**
```rust
#[tokio::test]
#[serial]
async fn test_boundary_detection_multi_track_album() {
    // Setup: Create test audio file with 10 tracks, 2-second silence gaps
    let test_file = create_album_wav(
        tracks: 10,
        track_duration_sec: 180.0,  // 3 minutes per track
        silence_duration_sec: 2.0,
    );

    // Initialize SessionOrchestrator with test database
    let pool = setup_test_db().await;
    let orchestrator = SessionOrchestrator::new(pool);

    // Execute: Run import session
    let session_id = Uuid::new_v4();
    orchestrator.process_session(session_id, vec![test_file]).await.unwrap();

    // Verify: Query passages table
    let passage_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM passages WHERE file_id = ?"
    )
    .bind(file_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(passage_count, 10, "Should detect 10 passages");
}
```

**Expected Result:** 10 passages detected (one per track)

---

#### TC-TD-001-02: Single Track Detection
**Requirement:** REQ-TD-001 Acceptance Criteria (single song → 1 passage)
**Priority:** CRITICAL
**Type:** Integration Test

**Test Setup:**
```rust
#[tokio::test]
#[serial]
async fn test_boundary_detection_single_track() {
    // Setup: Create continuous 3-minute audio (no silence gaps)
    let test_file = create_continuous_wav(duration_sec: 180.0);

    // Execute: Run import
    let orchestrator = SessionOrchestrator::new(test_pool().await);
    orchestrator.process_session(Uuid::new_v4(), vec![test_file]).await.unwrap();

    // Verify: Single passage detected
    let passage_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM passages").fetch_one(&pool).await.unwrap();

    assert_eq!(passage_count, 1, "Continuous audio should be single passage");
}
```

**Expected Result:** 1 passage spanning full file duration

---

#### TC-TD-001-03: Short Silence Ignored
**Requirement:** REQ-TD-001 Acceptance Criteria (silence <0.5s ignored)
**Priority:** CRITICAL
**Type:** Unit Test

**Test Setup:**
```rust
#[test]
fn test_silence_detector_ignores_short_gaps() {
    // Setup: Audio with 0.3-second silence gap (below 0.5s threshold)
    let samples = create_audio_with_silence(
        duration_before: 5.0,
        silence_duration: 0.3,  // Below min_duration threshold
        duration_after: 5.0,
    );

    let detector = SilenceDetector::new(-60.0, 0.5); // 0.5s minimum

    // Execute: Detect silence regions
    let regions = detector.detect(&samples, 44100).unwrap();

    // Verify: No regions detected (gap too short)
    assert!(regions.is_empty(), "Short silence gaps should be ignored");
}
```

**Expected Result:** No silence regions detected

---

#### TC-TD-001-04: Configurable Threshold
**Requirement:** REQ-TD-001 Acceptance Criteria (threshold configurable)
**Priority:** HIGH
**Type:** Unit Test

**Test Setup:**
```rust
#[test]
fn test_silence_detector_threshold_configuration() {
    // Setup: Audio with -50dB quiet section (not silence at -60dB threshold)
    let samples = create_audio_with_quiet_section(amplitude_db: -50.0);

    // Test 1: -60dB threshold (should NOT detect)
    let detector_60 = SilenceDetector::new(-60.0, 0.5);
    let regions_60 = detector_60.detect(&samples, 44100).unwrap();
    assert!(regions_60.is_empty(), "-50dB not silent at -60dB threshold");

    // Test 2: -40dB threshold (SHOULD detect)
    let detector_40 = SilenceDetector::new(-40.0, 0.5);
    let regions_40 = detector_40.detect(&samples, 44100).unwrap();
    assert!(!regions_40.is_empty(), "-50dB IS silent at -40dB threshold");
}
```

**Expected Result:** Threshold affects detection as configured

---

### REQ-TD-002: Audio Segment Extraction

#### TC-TD-002-01: Exact Duration Extraction
**Requirement:** REQ-TD-002 SHALL-5 (extract 30s from 3min → exactly 30s)
**Priority:** CRITICAL
**Type:** Unit Test

**Test Setup:**
```rust
#[tokio::test]
async fn test_segment_extraction_exact_duration() {
    // Setup: 3-minute test file
    let test_file = create_test_wav(duration_sec: 180.0, sample_rate: 44100);
    let loader = AudioLoader::new();

    // Execute: Extract 30-second segment starting at 1:00
    let start_ticks = 60 * 28_224_000;  // 1 minute
    let end_ticks = 90 * 28_224_000;    // 1.5 minutes (30 seconds duration)

    let samples = loader.extract_segment(
        &test_file,
        start_ticks,
        end_ticks,
        44100,  // target sample rate
    ).await.unwrap();

    // Verify: Exactly 30 seconds of audio (30 * 44100 samples)
    let expected_samples = 30 * 44100;
    assert_eq!(
        samples.len(),
        expected_samples,
        "Should extract exactly 30 seconds"
    );
}
```

**Expected Result:** samples.len() == 1,323,000 (30s * 44.1kHz)

---

#### TC-TD-002-02: Precise Tick Positioning
**Requirement:** REQ-TD-002 SHALL-5 (extract at 1:45.000 → precise position)
**Priority:** CRITICAL
**Type:** Unit Test

**Test Setup:**
```rust
#[tokio::test]
async fn test_segment_extraction_precise_positioning() {
    // Setup: Test file with known tone at 1:45.000
    let test_file = create_wav_with_tone_at_position(
        position_sec: 105.0,  // 1:45
        tone_freq_hz: 440.0,   // A4 note
        tone_duration_sec: 1.0,
    );

    let loader = AudioLoader::new();

    // Execute: Extract 2-second segment centered on tone
    let start_ticks = 104 * 28_224_000;  // 1:44
    let end_ticks = 106 * 28_224_000;    // 1:46

    let samples = loader.extract_segment(&test_file, start_ticks, end_ticks, 44100).await.unwrap();

    // Verify: Tone appears at expected position (1 second into extracted segment)
    let tone_start_sample = 1 * 44100;  // 1 second offset
    let tone_samples = &samples[tone_start_sample..(tone_start_sample + 44100)];

    let rms = calculate_rms(tone_samples);
    assert!(rms > 0.1, "Tone should be present at expected position");
}
```

**Expected Result:** Tone detected at precise tick position

---

#### TC-TD-002-03: Stereo to Mono Conversion
**Requirement:** REQ-TD-002 SHALL-6 (stereo → mono via channel averaging)
**Priority:** CRITICAL
**Type:** Unit Test

**Test Setup:**
```rust
#[tokio::test]
async fn test_segment_extraction_stereo_to_mono() {
    // Setup: Stereo test file (L=1.0, R=0.0 for verification)
    let test_file = create_stereo_wav(left_amplitude: 1.0, right_amplitude: 0.0);

    let loader = AudioLoader::new();

    // Execute: Extract full file as mono
    let samples = loader.extract_segment(&test_file, 0, 180 * 28_224_000, 44100).await.unwrap();

    // Verify: Mono output (average of L and R channels)
    // Expected: (1.0 + 0.0) / 2.0 = 0.5
    let avg_amplitude = samples.iter().sum::<f32>() / samples.len() as f32;

    assert!((avg_amplitude - 0.5).abs() < 0.01, "Stereo should be averaged to mono");
}
```

**Expected Result:** Mono output with averaged channel values

---

#### TC-TD-002-04: Resampling
**Requirement:** REQ-TD-002 SHALL-7 (48kHz → 44.1kHz resampling)
**Priority:** CRITICAL
**Type:** Unit Test

**Test Setup:**
```rust
#[tokio::test]
async fn test_segment_extraction_resampling() {
    // Setup: 48kHz test file, 10-second duration
    let test_file = create_test_wav(duration_sec: 10.0, sample_rate: 48000);

    let loader = AudioLoader::new();

    // Execute: Extract with 44.1kHz target
    let samples = loader.extract_segment(
        &test_file,
        0,
        10 * 28_224_000,
        44100,  // Target: 44.1kHz
    ).await.unwrap();

    // Verify: Output sample count matches 44.1kHz (not 48kHz)
    let expected_samples = 10 * 44100;  // 10 seconds at 44.1kHz
    assert_eq!(samples.len(), expected_samples, "Should resample to 44.1kHz");
}
```

**Expected Result:** 441,000 samples (10s * 44.1kHz)

---

#### TC-TD-002-05: Time Range Out of Bounds
**Requirement:** REQ-TD-002 Error Handling (time range exceeds file duration → error)
**Priority:** CRITICAL
**Type:** Error Test

**Test Setup:**
```rust
#[tokio::test]
async fn test_segment_extraction_out_of_bounds() {
    // Setup: 1-minute test file
    let test_file = create_test_wav(duration_sec: 60.0, sample_rate: 44100);

    let loader = AudioLoader::new();

    // Execute: Attempt to extract beyond file duration
    let start_ticks = 50 * 28_224_000;   // 50 seconds
    let end_ticks = 120 * 28_224_000;    // 2 minutes (exceeds 1-minute file)

    let result = loader.extract_segment(&test_file, start_ticks, end_ticks, 44100).await;

    // Verify: Error returned
    assert!(result.is_err(), "Should return error for out-of-bounds range");

    let error_msg = result.unwrap_err().to_string();
    assert!(
        error_msg.contains("TimeRangeOutOfBounds") || error_msg.contains("exceeds"),
        "Error should indicate time range issue"
    );
}
```

**Expected Result:** ImportError::TimeRangeOutOfBounds

---

### REQ-TD-003: Remove Amplitude Analysis Stub

#### TC-TD-003-01: Endpoint Removed
**Requirement:** REQ-TD-003 Option A SHALL-1 (remove endpoint from routes)
**Priority:** CRITICAL
**Type:** Integration Test

**Test Setup:**
```rust
#[tokio::test]
async fn test_amplitude_analysis_endpoint_removed() {
    // Setup: Start wkmp-ai server
    let server = start_test_server().await;

    // Execute: Attempt to call removed endpoint
    let client = reqwest::Client::new();
    let response = client
        .post(format!("{}/analyze/amplitude", server.base_url))
        .json(&serde_json::json!({
            "file_path": "/test/file.wav"
        }))
        .send()
        .await
        .unwrap();

    // Verify: 404 Not Found
    assert_eq!(response.status(), 404, "Endpoint should be removed (404)");
}
```

**Expected Result:** HTTP 404 Not Found

---

#### TC-TD-003-02: Module Removed from Codebase
**Requirement:** REQ-TD-003 Option A SHALL-2 (remove models)
**Priority:** CRITICAL
**Type:** Compilation Test

**Test Setup:**
```bash
# Verify module no longer exists
! grep -r "AmplitudeAnalysisRequest" wkmp-ai/src/
! grep -r "AmplitudeAnalysisResponse" wkmp-ai/src/
! grep -r "amplitude_analysis.rs" wkmp-ai/src/api/
```

**Expected Result:** No references found (grep returns empty)

---

## Sprint 2: High Priority Requirements

### REQ-TD-004: MBID Extraction from ID3 Tags

#### TC-TD-004-01: Extract MBID from MP3 UFID
**Requirement:** REQ-TD-004 SHALL-4 (parse MBID from UFID frame)
**Priority:** HIGH
**Type:** Unit Test

**Test Setup:**
```rust
#[tokio::test]
async fn test_mbid_extraction_from_ufid() {
    // Setup: MP3 file with MusicBrainz UFID frame
    let test_file = create_mp3_with_mbid(
        mbid: "550e8400-e29b-41d4-a716-446655440000"
    );

    let extractor = ID3Extractor::new();

    // Execute: Extract metadata
    let result = extractor.extract(&test_file).await.unwrap();

    // Verify: MBID extracted
    assert!(result.data.mbid.is_some(), "MBID should be extracted");
    assert_eq!(
        result.data.mbid.unwrap().to_string(),
        "550e8400-e29b-41d4-a716-446655440000"
    );
}
```

**Expected Result:** MBID successfully extracted as Uuid

---

#### TC-TD-004-02: File Without MBID Returns None
**Requirement:** REQ-TD-004 Acceptance Criteria (file without MBID → None)
**Priority:** HIGH
**Type:** Unit Test

**Test Setup:**
```rust
#[test]
fn test_mbid_extraction_no_ufid() {
    // Setup: MP3 file without UFID frame
    let test_file = create_mp3_without_mbid();

    let extractor = ID3Extractor::new();

    // Execute: Extract metadata
    let result = extractor.extract(&test_file).await.unwrap();

    // Verify: MBID is None (fallback to AcoustID fingerprinting)
    assert!(result.data.mbid.is_none(), "Should return None when no UFID present");
}
```

**Expected Result:** None (graceful fallback)

---

#### TC-TD-004-03: Malformed MBID Handling (ERROR TEST)
**Requirement:** Specification gap - error handling for invalid MBID format
**Priority:** HIGH
**Type:** Error Test

**Test Setup:**
```rust
#[test]
fn test_mbid_extraction_invalid_format() {
    // Setup: MP3 file with malformed UFID frame (invalid UUID)
    let test_file = create_mp3_with_invalid_mbid(
        ufid_data: "not-a-valid-uuid"
    );

    let extractor = ID3Extractor::new();

    // Execute: Extract metadata
    let result = extractor.extract(&test_file).await;

    // Verify: Either returns None OR returns error (both acceptable)
    // Should NOT panic
    match result {
        Ok(metadata) => assert!(metadata.data.mbid.is_none(), "Invalid MBID should be treated as missing"),
        Err(_) => {}, // Error is acceptable for malformed data
    }
}
```

**Expected Result:** Graceful handling (no panic)

---

### REQ-TD-005: Consistency Checker Implementation

#### TC-TD-005-01: Conflicting Candidates Detected
**Requirement:** REQ-TD-005 SHALL-3 (strsim < 0.85 → Conflict)
**Priority:** HIGH
**Type:** Unit Test

**Test Setup:**
```rust
#[test]
fn test_consistency_checker_conflict_detection() {
    // Setup: Metadata bundle with conflicting title candidates
    let bundle = MetadataBundle {
        title: vec![
            MetadataField { value: "The Beatles".to_string(), confidence: 0.9, source: ID3Metadata },
            MetadataField { value: "Beatles, The".to_string(), confidence: 0.9, source: MusicBrainz },
        ],
        ..Default::default()
    };

    let checker = ConsistencyChecker::new(0.85);  // Conflict threshold

    // Execute: Validate title
    let result = checker.validate_title(&bundle);

    // Verify: Conflict detected
    assert_eq!(result, ValidationResult::Conflict("Title mismatch: 'The Beatles' vs 'Beatles, The'"));
}
```

**Expected Result:** ValidationResult::Conflict with description

---

#### TC-TD-005-02: Similar Candidates Produce Warning
**Requirement:** Specification gap - Warning threshold (0.85 ≤ strsim < 0.95)
**Priority:** HIGH
**Type:** Unit Test

**Test Setup:**
```rust
#[test]
fn test_consistency_checker_warning_for_minor_differences() {
    // Setup: Title with case difference only
    let bundle = MetadataBundle {
        title: vec![
            MetadataField { value: "Let It Be".to_string(), confidence: 0.9, source: ID3Metadata },
            MetadataField { value: "Let it Be".to_string(), confidence: 0.9, source: MusicBrainz },
        ],
        ..Default::default()
    };

    let checker = ConsistencyChecker::new(0.85);

    // Execute: Validate title
    let result = checker.validate_title(&bundle);

    // Verify: Warning (not conflict, not pass)
    assert!(matches!(result, ValidationResult::Warning(_)), "Minor differences should produce warning");
}
```

**Expected Result:** ValidationResult::Warning

---

#### TC-TD-005-03: Identical Candidates Pass
**Requirement:** REQ-TD-005 Acceptance Criteria (identical → Pass)
**Priority:** HIGH
**Type:** Unit Test

**Test Setup:**
```rust
#[test]
fn test_consistency_checker_identical_pass() {
    // Setup: Identical title from multiple sources
    let bundle = MetadataBundle {
        title: vec![
            MetadataField { value: "Let It Be".to_string(), confidence: 0.9, source: ID3Metadata },
            MetadataField { value: "Let It Be".to_string(), confidence: 0.9, source: MusicBrainz },
        ],
        ..Default::default()
    };

    let checker = ConsistencyChecker::new(0.85);

    // Execute: Validate title
    let result = checker.validate_title(&bundle);

    // Verify: Pass
    assert_eq!(result, ValidationResult::Pass);
}
```

**Expected Result:** ValidationResult::Pass

---

### REQ-TD-006: Event Bridge session_id Fields

#### TC-TD-006-01: All Events Include session_id
**Requirement:** REQ-TD-006 SHALL-1 (add session_id to all variants)
**Priority:** HIGH
**Type:** Unit Test

**Test Setup:**
```rust
#[test]
fn test_import_event_session_id_present() {
    let session_id = Uuid::new_v4();

    // Test all ImportEvent variants
    let events = vec![
        ImportEvent::PassagesDiscovered { session_id, count: 10 },
        ImportEvent::SongStarted { session_id, passage_id: Uuid::new_v4() },
        ImportEvent::SongProgress { session_id, passage_id: Uuid::new_v4(), phase: "analysis".to_string() },
        ImportEvent::SongComplete { session_id, passage_id: Uuid::new_v4() },
        ImportEvent::SongFailed { session_id, passage_id: Uuid::new_v4(), error: "test error".to_string() },
        ImportEvent::SessionProgress { session_id, completed: 5, total: 10 },
        ImportEvent::SessionComplete { session_id },
        ImportEvent::SessionFailed { session_id, error: "test error".to_string() },
    ];

    // Verify: All events have session_id field (not Uuid::nil())
    for event in events {
        let extracted_session_id = event.session_id();  // Assume accessor method
        assert_ne!(extracted_session_id, Uuid::nil(), "session_id must not be nil");
        assert_eq!(extracted_session_id, session_id, "session_id must match");
    }
}
```

**Expected Result:** All events have valid session_id (not nil)

---

#### TC-TD-006-02: UI Event Correlation
**Requirement:** REQ-TD-006 Acceptance Criteria (UI correlates events correctly)
**Priority:** HIGH
**Type:** Integration Test

**Test Setup:**
```rust
#[tokio::test]
async fn test_event_correlation_by_session_id() {
    // Setup: Start import session
    let session_id = Uuid::new_v4();
    let orchestrator = SessionOrchestrator::new(test_pool().await);

    // Setup: Subscribe to event stream
    let mut event_stream = orchestrator.subscribe_events();

    // Execute: Run import
    tokio::spawn(async move {
        orchestrator.process_session(session_id, vec![test_file()]).await.unwrap();
    });

    // Verify: All received events have matching session_id
    let mut received_events = Vec::new();
    while let Some(event) = event_stream.recv().await {
        if matches!(event, ImportEvent::SessionComplete { .. }) {
            break;
        }
        received_events.push(event);
    }

    for event in received_events {
        assert_eq!(event.session_id(), session_id, "All events should have matching session_id");
    }
}
```

**Expected Result:** All events correlated to same session_id

---

### REQ-TD-007: Flavor Synthesis Implementation

#### TC-TD-007-01: Multiple Sources Combined
**Requirement:** REQ-TD-007 Acceptance Criteria (multiple sources → combined flavor)
**Priority:** HIGH
**Type:** Unit Test

**Test Setup:**
```rust
#[test]
fn test_flavor_synthesis_multiple_sources() {
    // Setup: Flavor data from multiple sources
    let flavor_sources = vec![
        FlavorExtraction {
            flavor: MusicalFlavor { tempo_bpm: 120.0, energy: 0.8, /* ... */ },
            confidence: 0.9,
            source: AudioDerived,
        },
        FlavorExtraction {
            flavor: MusicalFlavor { tempo_bpm: 122.0, energy: 0.75, /* ... */ },
            confidence: 0.85,
            source: AcousticBrainz,
        },
    ];

    let synthesizer = FlavorSynthesizer::new();

    // Execute: Synthesize flavor
    let result = synthesizer.synthesize(flavor_sources).unwrap();

    // Verify: Weighted average of sources
    // Expected tempo: (120 * 0.9 + 122 * 0.85) / (0.9 + 0.85) ≈ 120.97
    assert!((result.flavor.tempo_bpm - 120.97).abs() < 0.1, "Should combine sources via weighted average");
    assert!(result.flavor_confidence > 0.8, "Combined confidence should be high");
}
```

**Expected Result:** Weighted combination of multiple sources

---

#### TC-TD-007-02: Agreeing Sources High Confidence
**Requirement:** REQ-TD-007 Acceptance Criteria (agreeing sources → high confidence)
**Priority:** HIGH
**Type:** Unit Test

**Test Setup:**
```rust
#[test]
fn test_flavor_synthesis_agreeing_sources() {
    // Setup: Two sources with nearly identical flavor data
    let flavor_sources = vec![
        FlavorExtraction {
            flavor: MusicalFlavor { tempo_bpm: 120.0, energy: 0.8 },
            confidence: 0.9,
            source: AudioDerived,
        },
        FlavorExtraction {
            flavor: MusicalFlavor { tempo_bpm: 120.5, energy: 0.8 },
            confidence: 0.85,
            source: AcousticBrainz,
        },
    ];

    let synthesizer = FlavorSynthesizer::new();
    let result = synthesizer.synthesize(flavor_sources).unwrap();

    // Verify: High synthesis confidence (sources agree)
    assert!(result.flavor_confidence >= 0.9, "Agreeing sources should produce high confidence");
}
```

**Expected Result:** flavor_confidence ≥ 0.9

---

#### TC-TD-007-03: Conflicting Sources Lower Confidence
**Requirement:** REQ-TD-007 Acceptance Criteria (conflicting sources → lower confidence)
**Priority:** HIGH
**Type:** Unit Test

**Test Setup:**
```rust
#[test]
fn test_flavor_synthesis_conflicting_sources() {
    // Setup: Two sources with significantly different flavor data
    let flavor_sources = vec![
        FlavorExtraction {
            flavor: MusicalFlavor { tempo_bpm: 120.0, energy: 0.8 },
            confidence: 0.9,
            source: AudioDerived,
        },
        FlavorExtraction {
            flavor: MusicalFlavor { tempo_bpm: 180.0, energy: 0.3 },  // Very different
            confidence: 0.85,
            source: AcousticBrainz,
        },
    ];

    let synthesizer = FlavorSynthesizer::new();
    let result = synthesizer.synthesize(flavor_sources).unwrap();

    // Verify: Lower synthesis confidence (sources conflict)
    assert!(result.flavor_confidence < 0.7, "Conflicting sources should reduce confidence");
}
```

**Expected Result:** flavor_confidence < 0.7

---

### REQ-TD-008: Chromaprint Compressed Fingerprint

#### TC-TD-008-01: Fingerprint Compressed to Base64
**Requirement:** REQ-TD-008 SHALL-3 (encode as base64)
**Priority:** HIGH
**Type:** Unit Test

**Test Setup:**
```rust
#[tokio::test]
async fn test_chromaprint_compression_base64() {
    // Setup: Test audio file
    let test_file = create_test_wav(duration_sec: 30.0, sample_rate: 44100);

    let analyzer = ChromaprintAnalyzer::new();

    // Execute: Generate fingerprint
    let result = analyzer.analyze(&test_file, 44100).await.unwrap();

    // Verify: Compressed fingerprint is base64 string
    assert!(result.data.fingerprint_compressed.is_some(), "Compressed fingerprint should be present");

    let compressed = result.data.fingerprint_compressed.unwrap();

    // Base64 strings should only contain [A-Za-z0-9+/=]
    assert!(compressed.chars().all(|c| c.is_alphanumeric() || c == '+' || c == '/' || c == '='));
}
```

**Expected Result:** Valid base64 string

---

#### TC-TD-008-02: AcoustID Accepts Compressed Format
**Requirement:** REQ-TD-008 Acceptance Criteria (AcoustID accepts format)
**Priority:** HIGH
**Type:** Integration Test

**Test Setup:**
```rust
#[tokio::test]
async fn test_chromaprint_acoustid_compatibility() {
    // Setup: Generate compressed fingerprint
    let test_file = create_test_wav(duration_sec: 30.0, sample_rate: 44100);
    let analyzer = ChromaprintAnalyzer::new();
    let fingerprint_result = analyzer.analyze(&test_file, 44100).await.unwrap();
    let compressed = fingerprint_result.data.fingerprint_compressed.unwrap();

    // Execute: Query AcoustID API with compressed fingerprint
    let client = AcoustIDClient::new("test-api-key".to_string());
    let result = client.lookup(&compressed, 30).await;

    // Verify: API accepts fingerprint (no format errors)
    // Note: May return "no matches" but should not return "invalid fingerprint"
    match result {
        Ok(_) => {},  // Success
        Err(e) => {
            let error_msg = e.to_string();
            assert!(
                !error_msg.contains("invalid fingerprint") && !error_msg.contains("format"),
                "AcoustID should accept compressed format"
            );
        }
    }
}
```

**Expected Result:** No fingerprint format errors from API

---

#### TC-TD-008-03: Compression Error Handling (ERROR TEST)
**Requirement:** Specification gap - handle compression failures
**Priority:** HIGH
**Type:** Error Test

**Test Setup:**
```rust
#[tokio::test]
async fn test_chromaprint_compression_error_handling() {
    // Setup: Extremely short audio (may fail fingerprinting)
    let test_file = create_test_wav(duration_sec: 0.5, sample_rate: 44100);

    let analyzer = ChromaprintAnalyzer::new();

    // Execute: Attempt fingerprint generation
    let result = analyzer.analyze(&test_file, 44100).await;

    // Verify: Either succeeds OR returns graceful error (no panic)
    match result {
        Ok(fingerprint) => {
            // If succeeds, compressed fingerprint should be present
            assert!(fingerprint.data.fingerprint_compressed.is_some());
        },
        Err(e) => {
            // Error is acceptable for edge cases, but should be clear
            let error_msg = e.to_string();
            assert!(!error_msg.is_empty(), "Error message should be descriptive");
        }
    }
}
```

**Expected Result:** Graceful handling (no panic, clear error if fails)

---

## Test Coverage Summary

| Requirement | Unit Tests | Integration Tests | Error Tests | Total Tests |
|-------------|-----------|------------------|-------------|-------------|
| REQ-TD-001  | 2         | 2                | 0           | 4           |
| REQ-TD-002  | 4         | 0                | 1           | 5           |
| REQ-TD-003  | 0         | 1                | 1           | 2           |
| REQ-TD-004  | 2         | 0                | 1           | 3           |
| REQ-TD-005  | 3         | 0                | 0           | 3           |
| REQ-TD-006  | 1         | 1                | 0           | 2           |
| REQ-TD-007  | 3         | 0                | 0           | 3           |
| REQ-TD-008  | 1         | 1                | 1           | 3           |
| **TOTAL**   | **16**    | **5**            | **4**       | **25**      |

**Coverage:** 100% of requirements have executable tests defined

---

## Test Execution Strategy

### Sprint 1 Tests (REQ-TD-001, 002, 003)
**Total:** 11 tests
**Execution Time Estimate:** <5 minutes

**Order:**
1. REQ-TD-001 unit tests (silence detection)
2. REQ-TD-002 unit tests (segment extraction)
3. REQ-TD-001 integration tests (full import workflow)
4. REQ-TD-003 tests (endpoint removal verification)

### Sprint 2 Tests (REQ-TD-004 through 008)
**Total:** 14 tests
**Execution Time Estimate:** <10 minutes (includes AcoustID API call)

**Order:**
1. Unit tests first (fast feedback)
2. Integration tests second (require services)
3. Error tests last (edge cases)

### Continuous Integration
**All tests MUST pass before merging:**
```bash
cargo test --package wkmp-ai --lib import_v2
cargo test --package wkmp-ai integration_workflow
```

**Performance Benchmarks (Sprint 1):**
```bash
cargo bench boundary_detection  # <200ms target
cargo bench segment_extraction  # <100ms target
```
