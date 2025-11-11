# Test Audio Fixtures

**Created:** 2025-11-10
**Purpose:** PLAN027 Sprint 3 - Integration Testing
**Generator:** [generate_test_fixtures.rs](../../generate_test_fixtures.rs)

---

## Fixtures

### 1. multi_track_album.wav
**Duration:** 19 seconds
**Format:** 44.1kHz, 16-bit stereo WAV
**Content:** 3 tracks with 2-second silence gaps

**Track Structure:**
- 0-5s: Track 1 (440Hz sine wave - A4)
- 5-7s: Silence
- 7-12s: Track 2 (523.25Hz sine wave - C5)
- 12-14s: Silence
- 14-19s: Track 3 (659.25Hz sine wave - E5)

**Expected Behavior:**
- Boundary detection: 3 passages detected
- Silence regions: 2 gaps (at 5-7s and 12-14s)
- Passage boundaries: [0-5s], [7-12s], [14-19s]

**Use Cases:**
- Test multi-track album import
- Validate silence-based boundary detection
- Verify per-passage fingerprinting

---

### 2. minimal_valid.wav
**Duration:** 3 seconds
**Format:** 44.1kHz, 16-bit stereo WAV
**Content:** 440Hz sine wave (A4)

**Expected Behavior:**
- Chromaprint: Accepts (3s is minimum)
- Boundary detection: Single passage
- Fingerprinting: Generates valid fingerprint

**Use Cases:**
- Test minimum duration handling
- Verify chromaprint 3-second minimum
- Edge case validation

---

### 3. short_invalid.wav
**Duration:** 1 second
**Format:** 44.1kHz, 16-bit stereo WAV
**Content:** 440Hz sine wave (A4)

**Expected Behavior:**
- Chromaprint: Rejects (too short, <3s minimum)
- Error: `ImportError::AudioProcessingFailed("Audio too short for fingerprinting")`
- Graceful degradation: Import continues with low confidence

**Use Cases:**
- Test error handling for short audio
- Verify chromaprint duration validation
- Test graceful failure modes

---

### 4. no_silence.wav
**Duration:** 5 seconds
**Format:** 44.1kHz, 16-bit stereo WAV
**Content:** 440Hz sine wave (A4), continuous

**Expected Behavior:**
- Boundary detection: Single passage (no silence)
- Fingerprinting: Valid fingerprint generated
- Duration: Full 5-second passage

**Use Cases:**
- Test continuous audio (no silence gaps)
- Verify single-passage handling
- Baseline for silence detection testing

---

## Regenerating Fixtures

```bash
cd wkmp-ai
cargo test --test generate_test_fixtures -- --ignored --nocapture
```

---

## MP3 Fixtures (Manual Creation)

The following MP3 fixtures require manual creation with ID3 tag editors:

### picard_tagged.mp3 (Planned)
**Purpose:** MBID extraction testing (REQ-TD-004)
**Requirements:**
- Contains UFID frame with owner `http://musicbrainz.org`
- MBID: Valid UUID (e.g., `12345678-1234-1234-1234-123456789abc`)
- Audio: Any valid MP3 (3+ seconds)

**Tool:** MusicBrainz Picard or id3 crate

### conflicting_metadata.mp3 (Planned)
**Purpose:** Consistency checker testing (REQ-TD-005)
**Requirements:**
- ID3v2.3 tag: artist = "Beatles"
- ID3v2.4 tag: artist = "The Beatles"
- Expected similarity: 0.82 (triggers WARNING threshold 0.85)

**Tool:** ID3 tag editor supporting multiple tag versions

---

## File Sizes

```
multi_track_album.wav  ~3.2 MB (19s × 44.1kHz × 16-bit × 2ch)
minimal_valid.wav      ~0.5 MB (3s)
short_invalid.wav      ~0.2 MB (1s)
no_silence.wav         ~0.9 MB (5s)
```

**Total:** ~4.8 MB (acceptable for test fixtures)

---

## Validation

All fixtures validated with:
- ✅ Correct sample rate (44.1kHz)
- ✅ Correct bit depth (16-bit)
- ✅ Stereo channels (2)
- ✅ Valid WAV headers
- ✅ Expected durations

**Generator Test:** `cargo test --test generate_test_fixtures`
