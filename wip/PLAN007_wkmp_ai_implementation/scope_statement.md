# PLAN007: Scope Statement

**Project:** wkmp-ai (Audio Ingest) Microservice Implementation
**Specification:** [SPEC024-audio_ingest_architecture.md](../../docs/SPEC024-audio_ingest_architecture.md)
**Plan Date:** 2025-10-28
**Target Version:** Full version only (not Lite/Minimal)

---

## Executive Summary

**What We're Building:**
Complete implementation of wkmp-ai microservice to import user music collections into WKMP database with accurate MusicBrainz identification, automatic passage boundary detection, and amplitude-based crossfade timing analysis.

**Why It Matters:**
wkmp-ai enables new users to quickly onboard their music libraries with accurate metadata, unlocking the full power of WKMP's Musical Flavor-based Program Director. Without this module, users must manually create passages and metadata entries.

**Success Criteria:**
- Import 100 audio files in 2-5 minutes (Pi Zero2W) or 30-60 seconds (desktop)
- 95%+ MusicBrainz identification accuracy (for properly tagged files)
- Automatic passage boundary detection with user review/adjustment capability
- Real-time progress updates via SSE or polling
- Web UI for complete import workflow (http://localhost:5723)

---

## In Scope

### Phase 1: Core HTTP Server & Routing (P0)

**Deliverables:**
- Axum HTTP server on port 5723
- Health endpoint (`/health`) for wkmp-ui integration
- Basic routing structure for all endpoints
- Server startup/shutdown gracefully

**Requirements Satisfied:** AIA-OV-010, AIA-UI-010 (partial), AIA-UI-020 (partial)

---

### Phase 2: Import Workflow State Machine (P0)

**Deliverables:**
- 7-state workflow implementation (SCANNING → EXTRACTING → FINGERPRINTING → SEGMENTING → ANALYZING → FLAVORING → COMPLETED)
- In-memory session state management (UUID-based)
- State transition logic with error handling (→ FAILED, → CANCELLED)
- Session state persistence (transient, in-memory only)

**Requirements Satisfied:** AIA-WF-010, AIA-WF-020, AIA-ERR-010 (partial)

---

### Phase 3: File Discovery & Metadata Extraction (P0)

**Deliverables:**
- Directory traversal (recursive, symlink following with cycle detection)
- Audio file magic byte detection (not file extension)
- ID3/Vorbis/MP4 tag parsing (lofty crate)
- SHA-256 file hashing for deduplication
- File filtering per SPEC008 (skip hidden files, system directories)

**Requirements Satisfied:** AIA-COMP-010 (file_scanner, metadata_extractor), AIA-INT-010

**External Dependencies:** lofty crate, sha2 crate

---

### Phase 4: Audio Fingerprinting & MusicBrainz Identification (P0)

**Deliverables:**
- Chromaprint fingerprint generation (subprocess call to fpcalc)
- AcoustID API client (fingerprint → Recording MBID)
- MusicBrainz API client (MBID → Recording/Artist/Work/Album metadata)
- Rate limiting (1 req/s for MusicBrainz per API terms)
- Response caching (acoustid_cache, musicbrainz_cache tables)

**Requirements Satisfied:** AIA-COMP-010 (fingerprinter, musicbrainz_client), AIA-PERF-020 (caching, rate limits)

**External Dependencies:** chromaprint (fpcalc binary), AcoustID API, MusicBrainz API, reqwest crate

---

### Phase 5: Passage Boundary Detection (P0)

**Deliverables:**
- Silence-based passage boundary detection (RMS threshold, minimum duration)
- User-adjustable parameters (silence threshold, min duration) per source media type
- Passage boundary refinement via MusicBrainz track count alignment
- Manual boundary adjustment UI (waveform display, draggable markers)

**Requirements Satisfied:** AIA-INT-020 (IMPL005 Steps 1-4), AIA-COMP-010 (silence_detector)

**Related Specs:** IMPL005-audio_file_segmentation.md

---

### Phase 6: Amplitude Analysis (Lead-in/Lead-out Detection) (P0)

**Deliverables:**
- RMS amplitude envelope calculation with A-weighting
- Lead-in detection (0 to 5 seconds based on 1/4 perceived intensity threshold)
- Lead-out detection (0 to 5 seconds based on 1/4 perceived intensity threshold)
- Per-passage amplitude profile storage

**Requirements Satisfied:** AIA-COMP-010 (amplitude_analyzer)

**Related Specs:** SPEC025-amplitude_analysis.md, IMPL009-amplitude_analyzer_implementation.md

---

### Phase 7: Musical Flavor Retrieval (P0)

**Deliverables:**
- AcousticBrainz API client (Recording MBID → Musical Flavor vector JSON)
- Essentia subprocess integration (fallback when AcousticBrainz unavailable)
- Musical flavor vector storage in `passages.musical_flavor_vector` (JSON column)
- Response caching (acousticbrainz_cache table)

**Requirements Satisfied:** AIA-COMP-010 (acousticbrainz_client, essentia_runner), AIA-PERF-020 (caching)

**External Dependencies:** AcousticBrainz API, Essentia binary (optional), reqwest crate

---

### Phase 8: Database Integration (P0)

**Deliverables:**
- Database queries for all 9 tables written (files, passages, songs, artists, works, albums, passage_songs, passage_albums, caches)
- Tick-based timing conversion (28,224,000 ticks/second)
- Batch inserts for performance
- Transaction handling for atomic operations
- Settings table read for import parameters

**Requirements Satisfied:** AIA-DB-010, AIA-INT-030, AIA-PERF-020 (batch inserts)

**Related Specs:** IMPL001-database_schema.md, IMPL014-database_queries.md

---

### Phase 9: Real-Time Progress Updates (P0)

**Deliverables:**
- Server-Sent Events (SSE) endpoint (`GET /events?session_id={uuid}`)
- Event types: state_changed, progress, error, completed
- Tokio broadcast channel for event distribution
- Polling fallback endpoint (`GET /import/status/{session_id}`)
- Event history for reconnection support

**Requirements Satisfied:** AIA-SSE-010, AIA-POLL-010, AIA-ASYNC-010 (partial)

---

### Phase 10: Async Background Processing (P0/P1)

**Deliverables:**
- Tokio background task spawning for import workflow
- HTTP request immediate return with session ID
- Parallel file processing (4 concurrent workers) [P1]
- Graceful cancellation support
- Progress reporting during long operations

**Requirements Satisfied:** AIA-ASYNC-010, AIA-ASYNC-020

---

### Phase 11: Error Handling & Reporting (P0)

**Deliverables:**
- Error severity categorization (CRITICAL, WARNING, INFO)
- Error reporting via SSE, polling, and logs
- Per-file error tracking (file path, error code, message)
- Partial success support (continue on non-critical errors)
- Import summary with errors encountered

**Requirements Satisfied:** AIA-ERR-010, AIA-ERR-020

---

### Phase 12: Input Validation & Security (P0/P1)

**Deliverables:**
- Path traversal validation (prevent directory escape)
- Symlink validation (cycle detection)
- Parameter bounds checking
- API key secure storage (credentials table) [P1]
- File magic byte verification (not extension-based)

**Requirements Satisfied:** AIA-SEC-010, AIA-SEC-020

---

### Phase 13: Web UI Implementation (P0)

**Deliverables:**
- HTML/CSS/JavaScript for import wizard
- Routes: `/` (home), `/import-progress` (SSE-based live updates), `/segment-editor` (waveform + boundaries)
- waveform visualization (Canvas API or library)
- Draggable passage boundary markers
- Source media type selection (CD, Vinyl, Cassette, Other)
- MusicBrainz release matching UI
- Import completion screen with "Return to WKMP" link

**Requirements Satisfied:** AIA-UI-010, AIA-UI-020, AIA-UI-030, AIA-INT-020 (IMPL005 Steps 1-5 UI)

**Technology Stack:** HTML5, CSS3, vanilla JavaScript or lightweight framework (TBD)

---

### Phase 14: wkmp-ui Integration (P0)

**Deliverables:**
- wkmp-ui health check for wkmp-ai availability
- "Import Music" button in wkmp-ui library view
- Launch wkmp-ai in new browser tab/window (http://localhost:5723)
- "Full version required" message when wkmp-ai unavailable

**Requirements Satisfied:** AIA-MS-010, AIA-UI-020

**Dependencies:** wkmp-ui module (coordination required)

---

### Phase 15: Testing & Validation (P0/P1)

**Deliverables:**
- **Unit Tests (P0):** 8 categories per AIA-TEST-010
  - File scanner (directory traversal, file filtering)
  - Metadata extractor (tag parsing, hash calculation)
  - Fingerprinter (Chromaprint integration)
  - API clients (MusicBrainz, AcoustID, AcousticBrainz with mocks)
  - Amplitude analyzer (RMS, lead-in/lead-out detection)
  - Silence detector (threshold-based boundary detection)
  - Database queries (inserts, tick conversion)
  - State machine (transitions, error handling)

- **Integration Tests (P0):** Per AIA-TEST-020
  - HTTP endpoint testing (Axum routes)
  - SSE event streaming
  - Database integration (real SQLite)
  - wkmp-ui health check integration

- **E2E Tests (P1):** Per AIA-TEST-030
  - Complete import workflow (directory → passages in DB)
  - Mixed media types (MP3, FLAC, OGG, Opus, WAV)
  - Error scenarios (corrupt files, missing tags)

**Requirements Satisfied:** AIA-TEST-010, AIA-TEST-020, AIA-TEST-030

**Test Coverage Target:** >80% line coverage, 100% requirement coverage

---

## Out of Scope (Explicitly Excluded)

### Deferred to Future Enhancements

**AIA-FUTURE-010 Items:**
1. **Machine Learning-based Recording Identification**
   - Current: Relies on AcoustID fingerprinting
   - Future: Neural network-based audio matching

2. **Collaborative Filtering for Ambiguous Matches**
   - Current: User selects from MusicBrainz candidate list
   - Future: ML suggestions based on user's library patterns

3. **Automatic Genre Classification**
   - Current: Uses MusicBrainz genre tags
   - Future: ML-based genre inference from audio

4. **Duplicate Detection Beyond Hash Matching**
   - Current: SHA-256 file hash for exact duplicates
   - Future: Perceptual hashing for near-duplicates (different encodings of same recording)

5. **Multi-Language Metadata Support**
   - Current: English-centric UI and error messages
   - Future: i18n/l10n for international users

### Never In Scope for wkmp-ai

1. **Playback Functionality** - Owned by wkmp-ap
2. **Playlist Management** - Owned by wkmp-pd
3. **Lyric Editing** - Owned by wkmp-le (separate on-demand tool)
4. **User Authentication** - Owned by wkmp-ui
5. **CD Ripping** - External tool, import from ripped files only
6. **Music Download/Streaming** - Not a music acquisition tool
7. **Database Schema Changes** - Schema defined in IMPL001, wkmp-ai consumes only

---

## Assumptions

### Technical Assumptions

1. **SQLite Database Availability**
   - Assumption: Shared wkmp.db exists and is accessible
   - Impact: wkmp-ai cannot function without database
   - Mitigation: Database initialization handled by wkmp-common

2. **Chromaprint Binary Availability**
   - Assumption: fpcalc binary installed and in PATH
   - Impact: Fingerprinting fails without Chromaprint
   - Mitigation: Graceful degradation, manual metadata entry

3. **Internet Connectivity for External APIs**
   - Assumption: Internet available for MusicBrainz, AcoustID, AcousticBrainz
   - Impact: Identification and flavor retrieval fail offline
   - Mitigation: Cache past responses, allow manual metadata entry

4. **Essentia Availability (Optional)**
   - Assumption: Essentia may or may not be installed
   - Impact: Fallback for AcousticBrainz unavailable if Essentia missing
   - Mitigation: Musical flavor optional, import proceeds without it

5. **Audio Decoder Availability**
   - Assumption: symphonia crate can decode common formats (MP3, FLAC, OGG, Opus, WAV)
   - Impact: Files in unsupported formats skipped
   - Mitigation: Report unsupported formats to user

### Business Assumptions

6. **Single-User Import Sessions**
   - Assumption: Only one user imports at a time (no concurrent sessions)
   - Impact: Simplified state management, no session isolation
   - Validation: Current WKMP design is single-user system

7. **User Patience for Long Imports**
   - Assumption: Users willing to wait 2-5 minutes for 100 files (Pi Zero2W)
   - Impact: User experience acceptable with progress updates
   - Mitigation: SSE real-time progress, parallelization

8. **MusicBrainz Metadata Accuracy**
   - Assumption: MusicBrainz has accurate metadata for most commercial recordings
   - Impact: Identification quality depends on MusicBrainz coverage
   - Mitigation: Manual correction UI for mis-identified tracks

9. **User Music Files Tagged Reasonably**
   - Assumption: Most user files have basic tags (artist, title, album)
   - Impact: Fingerprinting more likely to succeed with metadata hints
   - Mitigation: Fingerprint-only matching when tags missing

### Environment Assumptions

10. **Target Hardware**
    - Primary: Raspberry Pi Zero 2W (1 GHz quad-core ARM, 512 MB RAM)
    - Secondary: Desktop/laptop (x86-64, multi-core, ≥2 GB RAM)
    - Impact: Performance targets calibrated for constrained hardware

11. **Operating Systems**
    - Primary: Linux (Raspberry Pi OS, Ubuntu, Debian)
    - Secondary: macOS, Windows
    - Impact: Path handling, symlink behavior varies by OS

12. **Rust Toolchain**
    - Assumption: Rust stable channel 1.70+
    - Impact: Language features and crate compatibility
    - Validation: Cargo.toml specifies minimum version

---

## Constraints

### Technical Constraints

1. **MusicBrainz API Rate Limit**
   - Constraint: 1 request per second per API terms
   - Impact: Import speed limited for large libraries
   - Mitigation: Response caching, batch queries where possible

2. **Memory Constraints (Pi Zero2W)**
   - Constraint: 512 MB RAM, shared with OS and other processes
   - Impact: Cannot load large files entirely into memory
   - Mitigation: Streaming audio analysis, chunk-based processing

3. **No Database Schema Modifications**
   - Constraint: wkmp-ai reads existing schema defined in IMPL001
   - Impact: Must work within existing tables and columns
   - Validation: Database queries match IMPL001 schema exactly

4. **Single SQLite Database**
   - Constraint: All WKMP modules share one wkmp.db file
   - Impact: Concurrency managed via SQLite locking, busy timeout
   - Mitigation: Keep transactions short, use WAL mode

5. **No Embedded wkmp-ui Integration**
   - Constraint: wkmp-ai provides own UI, not embedded in wkmp-ui
   - Impact: Requires separate browser tab/window
   - Validation: Per "on-demand" microservice pattern (PLAN006)

### Business Constraints

6. **Full Version Only**
   - Constraint: wkmp-ai not included in Lite/Minimal versions
   - Impact: Feature gating in wkmp-ui ("Full version required")
   - Validation: Packaging scripts exclude wkmp-ai from Lite/Minimal

7. **No Subscription/Commercial APIs**
   - Constraint: Must use free/open APIs only
   - Impact: Relies on MusicBrainz, AcoustID, AcousticBrainz free tiers
   - Mitigation: Respect rate limits, cache aggressively

8. **Backward Compatibility**
   - Constraint: Database format must remain compatible with existing wkmp-ap/pd
   - Impact: Cannot introduce breaking schema changes
   - Validation: Database migration tests

### Schedule Constraints

9. **Implementation Timeline**
   - Constraint: Target 3-4 weeks for MVP implementation (per original PLAN004 estimate)
   - Impact: Prioritize P0 requirements, defer P1/P3
   - Mitigation: Incremental delivery, checkpoint reviews

10. **Testing Requirements**
    - Constraint: Must achieve >80% test coverage before release
    - Impact: Testing time included in schedule
    - Validation: Coverage metrics tracked per increment

### Dependency Constraints

11. **External Binary Dependencies**
    - Constraint: Chromaprint (fpcalc), optionally Essentia
    - Impact: Must handle missing binaries gracefully
    - Mitigation: Runtime detection, graceful degradation

12. **Network Availability**
    - Constraint: External APIs require internet connectivity
    - Impact: Import functionality degraded offline
    - Mitigation: Cache responses, allow manual metadata entry

---

## Success Metrics

### Performance Metrics

| Metric | Target | Measurement Method |
|--------|--------|-------------------|
| **Import Speed (Pi Zero2W)** | 100 files in 2-5 minutes | E2E test with timer |
| **Import Speed (Desktop)** | 100 files in 30-60 seconds | E2E test with timer |
| **Identification Accuracy** | ≥95% for tagged files | Manual verification, 100-file sample |
| **Memory Usage (Pi Zero2W)** | <100 MB peak | Process monitoring during import |
| **Database Write Performance** | >100 inserts/second | Benchmark test |

### Quality Metrics

| Metric | Target | Measurement Method |
|--------|--------|-------------------|
| **Test Coverage** | >80% line coverage | cargo tarpaulin |
| **Requirement Coverage** | 100% P0/P1 requirements tested | Traceability matrix |
| **Error Handling** | No panics, graceful degradation | Fuzzing, error injection tests |
| **Code Quality** | clippy::pedantic, no warnings | CI checks |

### User Experience Metrics

| Metric | Target | Measurement Method |
|--------|--------|-------------------|
| **Progress Update Frequency** | Every 1-2 seconds | SSE event timing |
| **UI Responsiveness** | <100ms button click response | Manual testing |
| **Error Message Clarity** | User-actionable messages | User review |
| **Waveform Visualization** | Smooth rendering, <500ms load | Performance profiling |

---

## Risks and Mitigations

### High-Risk Items

1. **Risk:** MusicBrainz API unavailable or slow
   - **Impact:** Import stalls or fails
   - **Mitigation:** Response caching, timeout handling, retry logic

2. **Risk:** Chromaprint binary missing on user system
   - **Impact:** Fingerprinting fails, no automatic identification
   - **Mitigation:** Runtime detection, graceful degradation, manual metadata entry

3. **Risk:** Memory exhaustion on Pi Zero2W (512 MB limit)
   - **Impact:** Import crashes mid-session
   - **Mitigation:** Streaming processing, chunk-based analysis, memory profiling

### Medium-Risk Items

4. **Risk:** Web UI performance poor on low-end hardware
   - **Impact:** Sluggish user experience
   - **Mitigation:** Vanilla JS (no heavy frameworks), optimize rendering

5. **Risk:** Integration issues with wkmp-ui
   - **Impact:** Launch button fails, health check unreliable
   - **Mitigation:** Early integration testing, clear API contract

### Low-Risk Items

6. **Risk:** Test coverage target not met
   - **Impact:** Quality concerns, release delay
   - **Mitigation:** Test-first development, continuous coverage tracking

---

## Approval Checkpoint

**This scope statement defines what will and will not be implemented in PLAN007.**

**Approval Required Before Proceeding to Phase 2:**
- [ ] User confirms in-scope items are comprehensive
- [ ] User confirms out-of-scope items are acceptable
- [ ] User confirms assumptions are reasonable
- [ ] User confirms constraints are understood
- [ ] User approves success metrics

---

**End of Scope Statement**
