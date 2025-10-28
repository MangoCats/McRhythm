# Component Tests - wkmp-ai

**Requirements:** AIA-COMP-010
**Priority:** P0 (Critical)
**Test Count:** 9

---

## TEST-029: file_scanner Discovers Files

**Requirement:** AIA-COMP-010
**Type:** Unit
**Priority:** P0

**Given:**
- Root folder with: 5 MP3, 3 FLAC, 2 OGG, 1 M4A, 1 WAV
- Non-audio files: cover.jpg, readme.txt, .DS_Store

**When:**
- FileScanner::scan(root_folder)

**Then:**
- Input: Root folder path
- Output: List of 12 file paths (only audio files)
- No non-audio files in output
- All formats detected correctly

**Acceptance Criteria:**
- ✅ All 12 audio files discovered
- ✅ Non-audio files excluded
- ✅ Correct file paths returned
- ✅ Nested directories traversed

---

## TEST-030: metadata_extractor Parses Tags

**Requirement:** AIA-COMP-010
**Type:** Unit
**Priority:** P0

**Given:**
- MP3 with ID3v2 tags: title="Song", artist="Artist", album="Album", year=2020

**When:**
- MetadataExtractor::extract(file_path)

**Then:**
- Input: File path
- Output: Metadata struct with:
  - title: "Song"
  - artist: "Artist"
  - album: "Album"
  - duration_seconds: 180.5

**Acceptance Criteria:**
- ✅ All tag fields extracted
- ✅ Duration calculated accurately (±1s)
- ✅ UTF-8 encoding handled
- ✅ Missing tags return None

---

## TEST-031: fingerprinter Generates Chromaprint

**Requirement:** AIA-COMP-010
**Type:** Unit
**Priority:** P0

**Given:**
- Audio file: test.mp3 (valid PCM audio)

**When:**
- Fingerprinter::fingerprint_file(file_path)

**Then:**
- Input: Audio PCM data (any format)
- Output: Base64 fingerprint string starting with "AQAD..."
- Fingerprint length: ~400-800 characters

**Acceptance Criteria:**
- ✅ Fingerprint generated successfully
- ✅ Base64 encoded format
- ✅ Consistent output for same file
- ✅ Different output for different files

---

## TEST-032: musicbrainz_client Queries API

**Requirement:** AIA-COMP-010
**Type:** Integration
**Priority:** P0

**Given:**
- Recording MBID: "5e8d5f0b-3f8a-4c7e-9c4b-5e8d5f0b3f8a"
- MusicBrainz API responsive (or mock)

**When:**
- MusicBrainzClient::lookup_recording(mbid)

**Then:**
- Input: Recording MBID
- Output: Recording metadata (title, artist, work, album)
- Rate limited to 1 req/s

**Acceptance Criteria:**
- ✅ Recording metadata retrieved
- ✅ Artist(s) linked
- ✅ Work linked if present
- ✅ Response cached

---

## TEST-033: acousticbrainz_client Retrieves Flavor

**Requirement:** AIA-COMP-010
**Type:** Integration
**Priority:** P0

**Given:**
- Recording MBID with AcousticBrainz data available

**When:**
- AcousticBrainzClient::get_flavor(mbid)

**Then:**
- Input: Recording MBID
- Output: Musical flavor vector (JSON)
- Example: `{"danceability": 0.72, "energy": 0.85, ...}`

**Acceptance Criteria:**
- ✅ Flavor JSON retrieved
- ✅ Valid JSON format
- ✅ Response cached
- ✅ Missing data handled gracefully (returns None)

---

## TEST-034: amplitude_analyzer Detects Lead-in/out

**Requirement:** AIA-COMP-010
**Type:** Unit
**Priority:** P0

**Given:**
- Audio PCM data with 3s fade-in, constant middle, 2s fade-out
- Parameters: lead_in_threshold_db = -12.0, rms_window_ms = 100

**When:**
- AmplitudeAnalyzer::analyze_file(file_path, params)

**Then:**
- Input: Audio PCM data, parameters
- Output: AmplitudeAnalysisResult with:
  - lead_in_duration: ~3.0s
  - lead_out_duration: ~2.0s
  - peak_rms: ~0.9

**Acceptance Criteria:**
- ✅ Lead-in detected (±0.5s)
- ✅ Lead-out detected (±0.5s)
- ✅ Peak RMS accurate
- ✅ Quick ramp detection works

---

## TEST-035: silence_detector Finds Boundaries

**Requirement:** AIA-COMP-010
**Type:** Unit
**Priority:** P0

**Given:**
- Audio file with 3 songs separated by 1s silence each
- Silence threshold: -60dB

**When:**
- SilenceDetector::detect_passages(audio_data, threshold)

**Then:**
- Input: Audio PCM data, threshold
- Output: List of 3 (start, end) time pairs
- Boundaries at silence gaps

**Acceptance Criteria:**
- ✅ 3 passages detected
- ✅ Start/end times accurate (±0.2s)
- ✅ Silence gaps correctly identified
- ✅ Minimum silence duration honored (500ms)

---

## TEST-036: parameter_manager Loads/Saves Params

**Requirement:** AIA-COMP-010
**Type:** Integration
**Priority:** P0

**Given:**
- Database with settings table

**When:**
- ParameterManager::load_global()
- ParameterManager::save_global(params)

**Then:**
- Input: Parameter name
- Output: Parameter value (or default)
- Save updates database

**Acceptance Criteria:**
- ✅ Parameters loaded from database
- ✅ Defaults used if missing
- ✅ Save updates settings table
- ✅ JSON serialization/deserialization works

---

## TEST-037: Component Integration (Full Pipeline)

**Requirement:** AIA-COMP-010
**Type:** End-to-End
**Priority:** P0

**Given:**
- Single audio file: test.mp3
- All components initialized

**When:**
- Run complete import pipeline:
  1. file_scanner finds file
  2. metadata_extractor parses tags
  3. fingerprinter generates fingerprint
  4. acoustid_client looks up MBID
  5. musicbrainz_client gets metadata
  6. silence_detector finds passage (1 passage = whole file)
  7. amplitude_analyzer detects lead-in/out
  8. acousticbrainz_client gets flavor
  9. parameter_manager loads params
  10. Database queries insert all data

**Then:**
- Database has:
  - 1 file record
  - 1 passage record
  - 1 song record
  - 1+ artist records
  - Relationships linked
  - Cache entries populated

**Acceptance Criteria:**
- ✅ All components execute successfully
- ✅ Data flows correctly between components
- ✅ Database state complete and consistent
- ✅ No errors or warnings (unless expected)

---

## Test Implementation Notes

**Framework:** `cargo test --test component_tests -p wkmp-ai`

**Component Isolation:**
```rust
#[test]
fn test_file_scanner_unit() {
    let scanner = FileScanner::new();
    let files = scanner.scan(Path::new("fixtures/test_library")).unwrap();

    assert_eq!(files.len(), 12);
    assert!(files.iter().all(|f| is_audio_file(f)));
}

#[tokio::test]
async fn test_metadata_extractor_unit() {
    let extractor = MetadataExtractor::new();
    let metadata = extractor.extract(Path::new("fixtures/sample.mp3")).await.unwrap();

    assert_eq!(metadata.title, Some("Test Song".to_string()));
    assert_eq!(metadata.artist, Some("Test Artist".to_string()));
    assert!(metadata.duration_seconds > 0.0);
}
```

**Integration Test:**
```rust
#[tokio::test]
async fn test_full_component_integration() {
    let db = setup_test_db().await;

    // Initialize all components
    let scanner = FileScanner::new();
    let metadata_extractor = MetadataExtractor::new();
    let fingerprinter = Fingerprinter::new();
    let acoustid_client = AcoustIDClient::new(api_key, db.clone());
    let mb_client = MusicBrainzClient::new(db.clone());
    let amplitude_analyzer = AmplitudeAnalyzer::new(params);

    // Run pipeline
    let files = scanner.scan(test_folder).unwrap();
    assert_eq!(files.len(), 1);

    let file = &files[0];
    let metadata = metadata_extractor.extract(file).await.unwrap();
    let fingerprint = fingerprinter.fingerprint_file(file).unwrap();
    let mbid = acoustid_client.lookup(&fingerprint, metadata.duration_seconds as u32).await.unwrap();
    let recording = mb_client.lookup_recording(&mbid.unwrap()).await.unwrap();
    let amplitude_result = amplitude_analyzer.analyze_file(file, 0.0, metadata.duration_seconds).await.unwrap();

    // Verify database state
    let file_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM files")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(file_count.0, 1);

    let passage_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM passages")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(passage_count.0, 1);

    let song_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM songs")
        .fetch_one(&db)
        .await
        .unwrap();
    assert!(song_count.0 > 0);
}
```

---

End of component tests
