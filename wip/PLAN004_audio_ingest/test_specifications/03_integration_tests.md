# Integration Tests - wkmp-ai

**Requirements:** AIA-INT-010, AIA-INT-020, AIA-INT-030
**Priority:** P0 (Critical)
**Test Count:** 9

---

## TEST-048: SPEC008 File Discovery Integration

**Requirement:** AIA-INT-010
**Type:** Integration
**Priority:** P0

**Given:**
- Test library with mixed audio formats (MP3, FLAC, OGG, M4A, WAV)
- Nested directory structure: `artist/album/tracks/`
- Hidden files (.DS_Store) and non-audio files (cover.jpg)

**When:**
- Execute file discovery per SPEC008:32-88 workflow

**Then:**
- Only audio files discovered (no .DS_Store, no .jpg)
- All 5 formats detected correctly
- Nested structure traversed completely
- File paths preserved correctly

**Acceptance Criteria:**
- ✅ MP3, FLAC, OGG, M4A, WAV all discovered
- ✅ Hidden files ignored
- ✅ Non-audio files ignored
- ✅ Path accuracy verified

---

## TEST-049: SPEC008 Metadata Extraction Integration

**Requirement:** AIA-INT-010
**Type:** Integration
**Priority:** P0

**Given:**
- MP3 file with ID3v2 tags (title, artist, album, year)
- FLAC file with Vorbis comments
- M4A file with MP4 tags

**When:**
- Extract metadata per SPEC008:90-128 workflow

**Then:**
- ID3v2 tags extracted correctly (MP3)
- Vorbis comments extracted correctly (FLAC)
- MP4 tags extracted correctly (M4A)
- Duration calculated accurately (±100ms)

**Acceptance Criteria:**
- ✅ Title, artist, album extracted from all formats
- ✅ Duration matches file actual duration
- ✅ Tag encoding handled (UTF-8, Latin-1)
- ✅ Missing tags handled gracefully (NULL)

---

## TEST-050: SPEC008 MusicBrainz Lookup Integration

**Requirement:** AIA-INT-010
**Type:** Integration
**Priority:** P0

**Given:**
- Valid Recording MBID from AcoustID lookup
- MusicBrainz API responsive (or mock)

**When:**
- Query MusicBrainz per SPEC008:287-431 workflow

**Then:**
- Recording metadata retrieved
- Artist(s) linked correctly
- Work linked if present
- Album(s) linked if present
- Database entities created (songs, artists, works, albums tables)

**Acceptance Criteria:**
- ✅ Recording title matches
- ✅ Artist relationships created in song_artists table
- ✅ Work relationship set if present
- ✅ Album relationships created in passage_albums table

---

## TEST-051: IMPL005 Silence Detection Integration

**Requirement:** AIA-INT-020
**Type:** Integration
**Priority:** P0

**Given:**
- Audio file with 3 songs separated by silence (>500ms each)
- Silence threshold: -60dB
- Minimum silence duration: 500ms

**When:**
- Run silence detection per IMPL005 workflow

**Then:**
- 3 passages detected
- Passage boundaries at silence points
- Start/end times accurate (±100ms)

**Acceptance Criteria:**
- ✅ Correct number of passages found
- ✅ Silence gaps correctly identified
- ✅ Passage durations sensible (>30s, <45min)
- ✅ No overlapping passages

---

## TEST-052: IMPL005 Multi-Passage File Handling

**Requirement:** AIA-INT-020
**Type:** Integration
**Priority:** P0

**Given:**
- Single file with 10 classical tracks (vinyl rip)
- Clear silence between tracks

**When:**
- Import single file with multi-passage detection

**Then:**
- 10 passages created from 1 file
- Each passage links to same file_id
- MusicBrainz lookup attempted for each passage
- Distinct songs identified (different MBIDs)

**Acceptance Criteria:**
- ✅ 10 passages in database
- ✅ All have same file_id
- ✅ Different song_id for each (if MusicBrainz successful)
- ✅ Start/end times non-overlapping

---

## TEST-053: IMPL005 User Review Integration

**Requirement:** AIA-INT-020
**Type:** Integration
**Priority:** P0

**Given:**
- Auto-detected passage boundaries from silence
- User adjusts boundaries manually (+2s start, -1s end)

**When:**
- Apply user adjustments to passage timing

**Then:**
- Passage start_time_ticks adjusted (+2s in ticks)
- Passage end_time_ticks adjusted (-1s in ticks)
- Re-run amplitude analysis with new boundaries
- Update import_metadata with manual adjustment flag

**Acceptance Criteria:**
- ✅ Tick conversion accurate (seconds → ticks)
- ✅ Amplitude analysis re-runs on adjusted range
- ✅ import_metadata JSON includes "manually_adjusted": true
- ✅ Database updated with new timing

---

## TEST-054: IMPL001 Tick Conversion Accuracy

**Requirement:** AIA-INT-030
**Type:** Unit
**Priority:** P0

**Given:**
- Time values: 0s, 1s, 2.5s, 60s, 180.5s

**When:**
- Convert to ticks using formula: `ticks = seconds × 28,224,000`

**Then:**
- 0s → 0 ticks
- 1s → 28,224,000 ticks
- 2.5s → 70,560,000 ticks
- 60s → 1,693,440,000 ticks
- 180.5s → 5,094,432,000 ticks

**Acceptance Criteria:**
- ✅ All conversions exact
- ✅ No floating-point precision loss
- ✅ Round-trip conversion accurate (ticks → seconds → ticks)

---

## TEST-055: IMPL001 Tick Rounding Behavior

**Requirement:** AIA-INT-030
**Type:** Unit
**Priority:** P0

**Given:**
- Fractional seconds: 1.23456789s

**When:**
- Convert to ticks and back to seconds

**Then:**
- Forward: 1.23456789s → 34,836,172 ticks (floor rounding)
- Reverse: 34,836,172 ticks → 1.234567897... seconds
- Precision loss: <0.001s

**Acceptance Criteria:**
- ✅ Floor rounding applied (not round/ceil)
- ✅ Precision loss acceptable (<1ms)
- ✅ Consistent rounding across all operations

---

## TEST-056: IMPL001 Tick Database Storage

**Requirement:** AIA-INT-030
**Type:** Integration
**Priority:** P0

**Given:**
- Passage with timing: start=10.5s, end=185.3s, lead_in=2.3s, lead_out=3.2s

**When:**
- Insert passage into database

**Then:**
- start_time_ticks = 296,352,000 (10.5 × 28,224,000)
- end_time_ticks = 5,228,707,200 (185.3 × 28,224,000)
- lead_in_start_ticks = 64,915,200 (2.3 × 28,224,000)
- lead_out_start_ticks = 90,316,800 (3.2 × 28,224,000)
- All stored as INTEGER (not REAL)

**Acceptance Criteria:**
- ✅ Database column types are INTEGER
- ✅ No floating-point values in database
- ✅ Query returns exact tick values
- ✅ Reverse conversion matches original (±1ms)

---

## Test Implementation Notes

**Framework:** `cargo test --test integration_tests -p wkmp-ai`

**Mock Setup:**
```rust
// Mock MusicBrainz responses
#[tokio::test]
async fn test_musicbrainz_integration() {
    let mock_server = mockito::Server::new();
    let mock = mock_server.mock("GET", "/recording/test-mbid")
        .with_status(200)
        .with_body(include_str!("../fixtures/mb_recording.json"))
        .create();

    // Test integration with mock
    // ...

    mock.assert();
}
```

**Tick Conversion Helper:**
```rust
const TICKS_PER_SECOND: i64 = 28_224_000;

fn seconds_to_ticks(seconds: f64) -> i64 {
    (seconds * TICKS_PER_SECOND as f64).floor() as i64
}

fn ticks_to_seconds(ticks: i64) -> f64 {
    ticks as f64 / TICKS_PER_SECOND as f64
}
```

---

End of integration tests
