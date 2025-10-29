# PLAN007: Component Service Tests

**Test File:** 03_component_tests.md
**Requirement Covered:** AIA-COMP-010 (Component Responsibility Matrix)
**Test Count:** 18 (9 components × 2 tests each)
**Test Type:** Unit + Integration

---

## Component 1: file_scanner

### TC-COMP-001: Directory Traversal

**Type:** Unit Test | **Priority:** P0

**Given:** Root folder `/test/music` with structure:
```
/test/music/
├── artist1/
│   ├── album1/
│   │   ├── track01.mp3
│   │   └── track02.flac
│   └── album2/
│       └── track03.ogg
└── artist2/
    └── single.wav
```

**When:** `file_scanner.scan("/test/music")`

**Then:** Returns list of 4 audio file paths

**Verify:**
- All `.mp3`, `.flac`, `.ogg`, `.wav` files found
- Hidden files (`.hidden.mp3`) skipped
- System directories (`.git/`) skipped
- Correct relative paths returned

**Pass Criteria:** 100% file discovery, no false positives

---

### TC-COMP-002: Symlink Cycle Detection

**Type:** Unit Test | **Priority:** P0

**Given:** Directory with symlink cycle:
```
/test/music/
├── real_folder/
│   └── track.mp3
└── symlink_loop -> /test/music/real_folder/symlink_loop
```

**When:** `file_scanner.scan("/test/music")`

**Then:**
- Detects symlink cycle
- Logs warning
- Returns files from `real_folder/` (track.mp3)
- Does not hang or panic

**Verify:** Cycle detection prevents infinite recursion

**Pass Criteria:** Completes in <1 second, no panic

---

## Component 2: metadata_extractor

### TC-COMP-003: ID3 Tag Parsing

**Type:** Unit Test | **Priority:** P0

**Given:** MP3 file with ID3v2.4 tags:
- Title: "Test Track"
- Artist: "Test Artist"
- Album: "Test Album"
- Duration: 180.5 seconds

**When:** `metadata_extractor.extract("/path/to/test.mp3")`

**Then:** Returns metadata struct with correct values

**Verify:**
- `metadata.title == "Test Track"`
- `metadata.artist == "Test Artist"`
- `metadata.album == "Test Album"`
- `metadata.duration_seconds == 180.5`

**Pass Criteria:** Accurate tag parsing for ID3v2.3 and ID3v2.4

---

### TC-COMP-004: Vorbis Tag Parsing

**Type:** Unit Test | **Priority:** P0

**Given:** FLAC file with Vorbis comments:
- TITLE: "Test Track"
- ARTIST: "Test Artist"
- ALBUMARTIST: "Various Artists"

**When:** `metadata_extractor.extract("/path/to/test.flac")`

**Then:** Returns metadata with Vorbis comment values

**Verify:** Vorbis comment parsing (FLAC, OGG, Opus)

**Pass Criteria:** Supports all Vorbis comment formats

---

## Component 3: fingerprinter (chromaprint-sys-next)

### TC-COMP-005: Chromaprint Generation

**Type:** Unit Test | **Priority:** P0

**Given:**
- Audio PCM data (44.1 kHz, 16-bit, mono)
- Duration: 120 seconds (first 2 minutes of track)

**When:** `fingerprinter.generate(pcm_data, sample_rate)`

**Then:** Returns Chromaprint fingerprint as base64 string

**Verify:**
- Fingerprint length appropriate for duration (~2400 bytes for 120s)
- Base64 encoding valid
- Identical audio produces identical fingerprint (deterministic)

**Pass Criteria:** Fingerprint matches AcoustID API expectations

---

### TC-COMP-006: Base64 Encoding

**Type:** Unit Test | **Priority:** P0

**Given:** Raw Chromaprint fingerprint bytes

**When:** `fingerprinter.encode_base64(raw_fingerprint)`

**Then:** Returns URL-safe base64 string

**Verify:**
- Base64 decoding reconstructs original bytes
- No padding characters (stripped per AcoustID spec)

**Pass Criteria:** AcoustID API accepts encoded fingerprint

---

## Component 4: musicbrainz_client

### TC-COMP-007: MBID Lookup

**Type:** Integration Test (mocked API) | **Priority:** P0

**Given:** Recording MBID: `abcd1234-5678-90ef-ghij-klmnopqrstuv`

**When:** `musicbrainz_client.lookup_recording(mbid)`

**Then:** Returns Recording metadata (title, artist, work)

**Verify:**
- HTTP GET to `https://musicbrainz.org/ws/2/recording/{mbid}`
- User-Agent header set
- Response parsed correctly

**Pass Criteria:** Handles 200 OK, 404 Not Found, 503 Rate Limited

---

### TC-COMP-008: Rate Limiting (1 req/s)

**Type:** Integration Test | **Priority:** P0

**Given:** 10 consecutive MusicBrainz API requests

**When:** Requests submitted in rapid succession

**Then:**
- First request executes immediately
- Subsequent requests delayed to enforce 1 req/s
- Total time ≥ 10 seconds (10 requests × 1 second)

**Verify:** Token bucket or sleep-based rate limiting

**Pass Criteria:** No 503 errors from MusicBrainz API

---

## Component 5: acoustid_client

### TC-COMP-009: Fingerprint → MBID

**Type:** Integration Test (mocked API) | **Priority:** P0

**Given:**
- Chromaprint fingerprint (base64)
- Duration: 180 seconds
- AcoustID API key

**When:** `acoustid_client.lookup(fingerprint, duration, api_key)`

**Then:** Returns list of MBID candidates with confidence scores

**Verify:**
- HTTP POST to `https://api.acoustid.org/v2/lookup`
- Request includes: `client`, `fingerprint`, `duration`, `meta=recordings`
- Response parsed for MBIDs

**Pass Criteria:** Top MBID match has confidence >0.8

---

### TC-COMP-010: Response Caching

**Type:** Integration Test | **Priority:** P0

**Given:**
- First fingerprint lookup → MBID result
- Same fingerprint lookup again

**When:** Second lookup for identical fingerprint

**Then:**
- Cache hit detected
- No API request made
- Cached MBID returned

**Verify:**
- `acoustid_cache` table entry created on first lookup
- Second lookup reads from cache (no network call)

**Pass Criteria:** Cache hit reduces latency from 500ms to <10ms

---

## Component 6: amplitude_analyzer

### TC-COMP-011: RMS Calculation

**Type:** Unit Test | **Priority:** P0

**Given:** PCM audio samples (sine wave, known RMS value)

**When:** `amplitude_analyzer.calculate_rms(pcm_samples, window_size_ms)`

**Then:** Returns RMS envelope (array of RMS values per window)

**Verify:**
- RMS formula: `sqrt(sum(samples^2) / count)`
- Window size: 100ms (per ISSUE-L04 resolution)
- Accuracy within 1% of known RMS

**Pass Criteria:** RMS values match expected for test waveforms

---

### TC-COMP-012: Lead-in/Lead-out Detection

**Type:** Unit Test | **Priority:** P0

**Given:**
- Audio passage with fade-in (0-3 seconds: volume ramps 0→1)
- Audio passage with fade-out (177-180 seconds: volume ramps 1→0)
- Threshold: 1/4 perceived intensity (per user story)

**When:** `amplitude_analyzer.detect_lead_in_out(rms_envelope)`

**Then:**
- Lead-in duration: 3.0 seconds (fade-in region)
- Lead-out duration: 3.0 seconds (fade-out region)

**Verify:**
- Lead-in/lead-out capped at 5 seconds max (per SPEC025)
- Abrupt starts/ends: lead-in/lead-out = 0 seconds

**Pass Criteria:** Detection accuracy within 100ms of ground truth

---

## Component 7: silence_detector

### TC-COMP-013: Threshold-Based Detection

**Type:** Unit Test | **Priority:** P0

**Given:**
- Audio with silence regions: 0-10s (music), 10-12s (silence), 12-180s (music)
- Silence threshold: -60dB (Vinyl preset per IMPL005)

**When:** `silence_detector.detect(audio_pcm, threshold_db, min_duration_sec)`

**Then:** Returns list of silence regions: `[(10.0, 12.0)]`

**Verify:**
- Silence correctly identified by RMS threshold
- Non-silence regions excluded

**Pass Criteria:** 100% accuracy for synthetic test audio

---

### TC-COMP-014: Minimum Duration Filtering

**Type:** Unit Test | **Priority:** P0

**Given:**
- Audio with brief silences: 10.0-10.2s (0.2s silence), 50.0-51.0s (1.0s silence)
- Minimum silence duration: 0.5 seconds (per IMPL005)

**When:** `silence_detector.detect(audio_pcm, -60dB, 0.5s)`

**Then:** Returns only long silence: `[(50.0, 51.0)]`

**Verify:** Short silences (<0.5s) filtered out

**Pass Criteria:** Only silences ≥ minimum duration returned

---

## Component 8: essentia_runner

### TC-COMP-015: Subprocess Execution

**Type:** Integration Test | **Priority:** P0

**Given:**
- Essentia binary in PATH
- Audio file: `/test/track.mp3`
- Output JSON: `/tmp/essentia_output.json`

**When:** `essentia_runner.analyze("/test/track.mp3", "/tmp/essentia_output.json")`

**Then:**
- Subprocess spawned: `essentia_streaming_extractor_music /test/track.mp3 /tmp/essentia_output.json`
- Subprocess completes successfully (exit code 0)
- Output JSON file created

**Verify:**
- Subprocess stdout/stderr captured for debugging
- Timeout after 60 seconds (safety)

**Pass Criteria:** Essentia completes analysis in <30 seconds

---

### TC-COMP-016: JSON Parsing

**Type:** Integration Test | **Priority:** P0

**Given:** Essentia output JSON with Musical Flavor data:
```json
{
  "lowlevel": {
    "average_loudness": 0.75,
    "dynamic_complexity": 0.5
  },
  "rhythm": {
    "bpm": 120.0
  },
  "tonal": {
    "key_key": "C",
    "key_scale": "major"
  }
}
```

**When:** `essentia_runner.parse_json("/tmp/essentia_output.json")`

**Then:** Returns Musical Flavor vector (JSON object)

**Verify:**
- All relevant fields extracted
- Invalid JSON → error

**Pass Criteria:** Parsing handles all Essentia output formats

---

## Component 9: parameter_manager

### TC-COMP-017: Global Defaults

**Type:** Unit Test | **Priority:** P0

**Given:** Fresh database with no user settings

**When:** `parameter_manager.get("silence_threshold_db")`

**Then:** Returns default value: -60dB (Vinyl preset)

**Verify:**
- Default values from IMPL010 specification
- No database entry required for defaults

**Pass Criteria:** All parameters have sensible defaults

---

### TC-COMP-018: Per-File Overrides

**Type:** Unit Test | **Priority:** P0

**Given:**
- Global setting: `silence_threshold_db = -60dB`
- Per-file override for `/path/to/noisy_vinyl.flac`: `-50dB`

**When:** `parameter_manager.get("silence_threshold_db", file="/path/to/noisy_vinyl.flac")`

**Then:** Returns override value: -50dB

**Verify:**
- Per-file settings take precedence over global
- Other files use global default

**Pass Criteria:** Parameter resolution: file-specific > global > default

---

## Test Execution Summary

**Total Component Tests:** 18
**Unit Tests:** 12 (fast, no external dependencies)
**Integration Tests:** 6 (require mocks or real external services)

**Dependencies:**
- Mock HTTP client for MusicBrainz/AcoustID tests
- In-memory SQLite for cache tests
- Sample audio files (sine waves, silence, real music)
- Essentia binary (or mock subprocess)

**Estimated Execution Time:**
- Unit tests: <1 second total
- Integration tests: 5-10 seconds total
- **Total:** <15 seconds for all component tests

---

**End of Component Tests**
