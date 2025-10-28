# Current State Assessment

**Navigation:** [← Back to Summary](00_SUMMARY.md) | [Next: Requirements Analysis →](02_requirements_analysis.md)

---

## wkmp-ai Implementation Status

**Current State:** Placeholder only

**File:** `wkmp-ai/src/main.rs`
```rust
fn main() {
    println!("wkmp-ai placeholder");
}
```

**Status:**
- No HTTP server
- No routes or API endpoints
- No business logic
- No database queries
- **Result:** Full greenfield specification required

**Module Position:**
- Microservice: Audio Ingest (wkmp-ai)
- Port: 5723
- Version: Full only (not in Lite/Minimal)
- Purpose: Guide users through music import workflow

---

## Existing Documentation Review

### SPEC008: Library Management (COMPLETE)

**Location:** `docs/SPEC008-library_management.md`
**Status:** ✅ Fully specified

**Coverage:**

1. **File Discovery Workflow** (lines 32-88)
   - Directory traversal with symlink support
   - File filtering by extension (.mp3, .flac, .ogg, .m4a, .wav)
   - SHA-256 hash calculation
   - Database recording (relative paths to root folder)
   - Incremental scan (detect deleted, modified, new files)

2. **Metadata Extraction** (lines 90-128)
   - Tag parsing: ID3v2, Vorbis Comments, MP4 tags, WAV RIFF
   - Recommended library: `lofty` crate (unified interface)
   - Fallback to filename parsing if tags missing

3. **Cover Art Extraction** (lines 130-209)
   - Extract embedded images (APIC, METADATA_BLOCK_PICTURE, covr)
   - Resize to max 1024x1024 (preserve aspect ratio)
   - Storage: Same folder as audio file
   - Additional image types: song-specific, passage-specific, work images

4. **Audio Fingerprinting** (lines 210-285)
   - Chromaprint integration (11025 Hz, mono, 16-bit)
   - AcoustID API query (rate limited: 3 req/s)
   - Caching in `acoustid_cache` table

5. **MusicBrainz Integration** (lines 287-431)
   - Recording lookup by MBID
   - Artist, work, album metadata fetching
   - Cover Art Archive integration
   - Rate limiting: 1 req/s (per MusicBrainz terms)

6. **AcousticBrainz Integration** (lines 435-510)
   - Fetch musical flavor data (high-level descriptors)
   - Fallback to local Essentia analysis if 404
   - Cache responses indefinitely (static data)

**Specification Quality:** Comprehensive, implementation-ready

---

### IMPL005: Audio File Segmentation (COMPLETE)

**Location:** `docs/IMPL005-audio_file_segmentation.md`
**Status:** ✅ Fully specified

**Coverage:**

**5-Step Workflow:**

1. **Source Media Identification** (lines 18-26)
   - User selects: CD / Vinyl / Cassette / Other
   - Sets default silence thresholds

2. **Automatic Silence Detection** (lines 28-43)
   - Default thresholds:
     - CD: -80 dB
     - Vinyl: -60 dB
     - Cassette (with Dolby): -70 dB
     - Cassette (no Dolby): -50 dB
     - Other: -60 dB
   - Minimum silence duration: 0.5 seconds
   - User-adjustable before scan

3. **MusicBrainz Release Matching** (lines 45-55)
   - AcoustID fingerprint entire file
   - Query MusicBrainz for matching Releases
   - User selects most likely match
   - Align segments with release track list

4. **User Review and Manual Adjustment** (lines 57-68)
   - Waveform visualization
   - Drag passage boundaries
   - Add/delete passages
   - Re-assign songs to passages

5. **Ingestion and Analysis** (lines 70-78)
   - Create passages in database
   - Associate songs with passages
   - AcousticBrainz lookup for musical flavor
   - Queue local Essentia jobs if needed

**Specification Quality:** Comprehensive, UI workflow specified

---

### IMPL001: Database Schema (RELEVANT SECTIONS)

**Location:** `docs/IMPL001-database_schema.md`

**Relevant Tables:**

**`files` table** (lines 120-140)
```sql
CREATE TABLE files (
  guid TEXT PRIMARY KEY,
  path TEXT NOT NULL UNIQUE,  -- Relative to root folder
  hash TEXT NOT NULL,          -- SHA-256
  duration REAL,               -- Seconds, NULL = not scanned
  modification_time TIMESTAMP NOT NULL,
  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

**`passages` table** (lines 142-197)
```sql
CREATE TABLE passages (
  guid TEXT PRIMARY KEY,
  file_id TEXT NOT NULL REFERENCES files(guid) ON DELETE CASCADE,

  -- Tick-based timing (28,224,000 ticks/second)
  start_time_ticks INTEGER NOT NULL,
  fade_in_start_ticks INTEGER,    -- NULL = use global default
  lead_in_start_ticks INTEGER,    -- NULL = use global default
  lead_out_start_ticks INTEGER,   -- NULL = use global default
  fade_out_start_ticks INTEGER,   -- NULL = use global default
  end_time_ticks INTEGER NOT NULL,

  -- Fade curves
  fade_in_curve TEXT,   -- exponential, cosine, linear
  fade_out_curve TEXT,  -- logarithmic, cosine, linear

  -- Metadata (denormalized cache)
  title TEXT,
  user_title TEXT,
  artist TEXT,
  album TEXT,

  -- Musical flavor (JSON blob)
  musical_flavor_vector TEXT,  -- AcousticBrainz high-level descriptors

  created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

**Tick-Based Timing:**
- Lossless precision for sample-accurate crossfading
- Conversion: `ticks = seconds * 28_224_000`
- One tick ≈ 35.4 nanoseconds
- See SPEC017-sample_rate_conversion.md for details

**JSON Metadata Pattern:**
- `musical_flavor_vector` column stores JSON
- Precedent for flexible, schema-less metadata
- Used for AcousticBrainz high-level descriptors

---

### REQ001: Requirements (RELEVANT SECTIONS)

**Location:** `docs/REQ001-requirements.md`

**Passage Import Requirements** (lines 271-290)

**[REQ-PI-040]** Each audio file may contain one or more passages

**[REQ-PI-050]** Users must be able to:
- **[REQ-PI-051]** Define multiple passages within a single audio file
- **[REQ-PI-052]** Manually edit passage boundaries and timing points
- **[REQ-PI-053]** Add or delete passage definitions
- **[REQ-PI-054]** Associate each passage with MusicBrainz entities

**[REQ-PI-060]** On initial import, system must assist users by offering automatic passage boundary detection. Detailed workflow in IMPL005-audio_file_segmentation.md.

**[REQ-PI-070]** Store MusicBrainz IDs and fetch basic metadata (artist names, release titles, genre tags)

**[REQ-PI-080]** WebUI provides interface to input/edit lyrics (Full version only)

**Note:** No requirements for amplitude-based lead-in/lead-out detection (gap identified)

---

### REQ002: Entity Definitions

**Location:** `docs/REQ002-entity_definitions.md`

**Key Entities:**

**[ENT-MP-020] Audio File:** File on disk (.mp3, .flac, .ogg, .m4a, .wav)

**[ENT-MP-030] Passage:** Defined span of audio with timing points:
- start_time, fade_in_start, lead_in_start, lead_out_start, fade_out_start, end_time
- Timing points defined in SPEC002-crossfade.md

**[ENT-MP-010] Song:** Recording + Artists + Works
- Used for selection and cooldown tracking

**[ENT-MP-035] Audio file as Passage:** Passage with undefined timing points defaults to:
- Start: beginning of file
- End: end of file
- Zero duration for all lead/fade times

**Zero-Song Passage:**
- Passage not associated with any MusicBrainz Recording
- Can be played but excluded from automatic selection
- No musical flavor (no AcousticBrainz data)

---

## Specification Coverage Summary

| Feature | Existing Specification | Status | Location |
|---------|----------------------|--------|----------|
| File scanning | Complete | ✅ | SPEC008:32-88 |
| Metadata extraction | Complete | ✅ | SPEC008:90-128 |
| Cover art extraction | Complete | ✅ | SPEC008:130-209 |
| Chromaprint fingerprinting | Complete | ✅ | SPEC008:210-285 |
| MusicBrainz integration | Complete | ✅ | SPEC008:287-431 |
| AcousticBrainz integration | Complete | ✅ | SPEC008:435-510 |
| Silence-based segmentation | Complete | ✅ | IMPL005:28-78 |
| User review workflow | Complete | ✅ | IMPL005:57-68 |
| Database schema (files, passages) | Complete | ✅ | IMPL001:120-197 |
| **Amplitude-based lead-in/lead-out** | **None** | **⚠️ Missing** | **NEW** |
| **Parameter management** | **Partial (UI shown)** | **⚠️ Incomplete** | **NEW** |
| **Extensible metadata** | **None** | **⚠️ Missing** | **NEW** |
| **HTTP API design** | **None** | **⚠️ Missing** | **NEW** |

---

## WKMP Architecture Context

**Microservices (5 total):**
1. **wkmp-ap** (Audio Player) - Port 5721 - Playback, crossfading, queue
2. **wkmp-ui** (User Interface) - Port 5720 - Web UI, authentication, orchestration
3. **wkmp-pd** (Program Director) - Port 5722 - Automatic passage selection
4. **wkmp-ai** (Audio Ingest) - Port 5723 - File import (Full version only)
5. **wkmp-le** (Lyric Editor) - Port 5724 - Lyric editing (Full version only)

**Communication:**
- HTTP REST APIs between modules
- Server-Sent Events (SSE) for real-time updates
- Shared SQLite database (`wkmp.db` in root folder)

**Technology Stack:**
- Language: Rust (stable)
- Async Runtime: Tokio
- Web Framework: Axum (HTTP + SSE)
- Audio: symphonia (decode), rubato (resample), cpal (output)
- Database: SQLite with JSON1 extension

**Development Principles:**
- Sample-accurate timing (tick-based precision)
- Automatic operation with manual control when desired
- Real-time UI updates (SSE events)
- Offline operation (local caches of online data)

---

**Navigation:** [← Back to Summary](00_SUMMARY.md) | [Next: Requirements Analysis →](02_requirements_analysis.md)
