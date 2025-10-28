# Scope Statement - PLAN004

**Plan:** PLAN004 - wkmp-ai Audio Ingest Implementation
**Specification:** SPEC024-audio_ingest_architecture.md
**Date:** 2025-10-27

---

## In Scope

### Core Functionality
1. **HTTP Server (Axum)**
   - Port 5723 binding and routing
   - SSE endpoint for real-time events
   - REST API endpoints per IMPL008
   - Graceful shutdown handling

2. **Import Workflow State Machine**
   - 7-state workflow (SCANNING → EXTRACTING → FINGERPRINTING → SEGMENTING → ANALYZING → FLAVORING → COMPLETED)
   - Background async processing with Tokio
   - Cancellation support
   - Error recovery (Warning/Skip/Critical categorization)

3. **File Discovery and Metadata Extraction**
   - Recursive directory traversal
   - Audio file format detection (MP3, FLAC, OGG, AAC, WAV)
   - ID3/Vorbis/MP4 tag parsing (lofty crate)
   - File hash calculation (SHA-256)
   - Cover art extraction

4. **Audio Fingerprinting**
   - Chromaprint fingerprint generation
   - AcoustID API integration
   - MusicBrainz MBID lookup
   - Response caching (acoustid_cache, musicbrainz_cache)

5. **MusicBrainz Integration**
   - Recording metadata retrieval
   - Artist/work/album relationship mapping
   - Database entity creation and linking
   - Rate limiting (1 req/s compliance)

6. **Passage Boundary Detection**
   - Silence-based segmentation (IMPL005)
   - User-adjustable threshold parameters
   - Multi-passage file support (vinyl/CD rips)

7. **Amplitude Analysis (NEW)**
   - RMS envelope calculation with A-weighting
   - Lead-in/lead-out detection (1/4 intensity threshold)
   - Quick ramp detection (3/4 intensity < 1s)
   - Per-passage amplitude metadata storage
   - Parameter management (IMPL010)

8. **AcousticBrainz Integration**
   - Musical flavor vector retrieval
   - JSON storage in passages.musical_flavor
   - Response caching (acousticbrainz_cache)

9. **Progress Reporting**
   - Server-Sent Events (SSE) for real-time updates
   - HTTP polling fallback (/import/status)
   - Event types: state_changed, progress, error, completed
   - Aggregated error reporting

10. **Database Operations**
    - Tick-based timing conversion (seconds × 28,224,000)
    - Batch inserts (100 passages at a time)
    - Transaction handling
    - Foreign key cascade cleanup

11. **Testing**
    - Unit tests (state machine, validation, tick conversion)
    - Integration tests (mock APIs, sample files)
    - End-to-end tests (10-file library import)

---

## Out of Scope

### Excluded from This Implementation

1. **wkmp-ui Integration Logic**
   - UI workflow orchestration remains in wkmp-ui
   - Import wizard UI components (wkmp-ui responsibility)
   - User authentication/authorization (wkmp-ui responsibility)

2. **Database Schema Changes**
   - Schema already defined in IMPL001
   - Migrations already created
   - Only implementing data access layer

3. **Essentia Integration**
   - Local audio analysis using Essentia subprocess
   - Deferred to future enhancement (AIA-FUTURE-010)
   - Will use AcousticBrainz API exclusively for MVP

4. **Advanced Features (Future Enhancements)**
   - Resume after interruption (requires state persistence)
   - Incremental import (detect new/modified files only)
   - Conflict resolution (duplicate file handling)
   - Genre/BPM/key detection

5. **Manual Segmentation UI**
   - Manual passage boundary adjustment
   - Waveform visualization
   - Interactive editing (wkmp-le responsibility)

6. **Lyric Management**
   - Lyric extraction/editing (wkmp-le responsibility)
   - Lyric synchronization

---

## Assumptions

1. **Environment**
   - Rust stable toolchain available
   - SQLite database already initialized with schema
   - Root folder path configured in settings
   - Network connectivity for external APIs

2. **External APIs**
   - AcoustID API available and responsive
   - MusicBrainz API available (rate limit: 1 req/s)
   - AcousticBrainz API available
   - API keys provided via environment variables

3. **Audio Files**
   - Files are valid audio formats (symphonia-supported)
   - Files are readable (permissions)
   - Files are not DRM-protected
   - File paths are UTF-8 compatible

4. **Performance**
   - Import is single-user operation (no concurrent sessions)
   - User accepts multi-hour import for large libraries (>1000 files)
   - MusicBrainz rate limiting is primary bottleneck

5. **Data Quality**
   - MusicBrainz has recordings for most user library
   - AcousticBrainz has flavor data for ~60% of recordings
   - Missing data is handled gracefully (warnings, not failures)

---

## Constraints

### Technical Constraints

1. **Rate Limiting**
   - MusicBrainz: Maximum 1 request/second
   - AcoustID: Maximum 3 requests/second
   - Must implement request throttling

2. **Database**
   - Shared SQLite database (no concurrent import from multiple users)
   - No schema migrations (use existing IMPL001 schema)
   - Tick precision: 28,224,000 ticks/second (INTEGER)

3. **Microservices**
   - HTTP-only communication (no gRPC)
   - SSE for real-time events (no WebSockets)
   - Port 5723 fixed (WKMP convention)

4. **Audio Processing**
   - symphonia decoder (no FFmpeg dependency)
   - rubato resampler (to 44.1kHz for Chromaprint)
   - dasp for RMS calculations

5. **Parallelism**
   - Default 4 concurrent file operations
   - User-configurable via import_parallelism parameter
   - Maximum 16 workers (reasonable CPU/I/O balance)

### Process Constraints

1. **Documentation Hierarchy**
   - Follow GOV001 tier system
   - SPEC024 (Tier 2) is authoritative for architecture
   - Implementation must not contradict specification

2. **Testing Requirements**
   - 100% requirement traceability
   - All P0 requirements must have acceptance tests
   - No implementation without corresponding tests

3. **Code Quality**
   - Follow IMPL002 coding conventions
   - Clippy warnings treated as errors
   - rustfmt applied to all code

---

## Success Criteria

Implementation is complete when:

1. ✅ All 17 P0 requirements implemented and tested
2. ✅ All 5 P1 requirements implemented and tested
3. ✅ 100% acceptance test pass rate
4. ✅ End-to-end test imports 10-file library successfully
5. ✅ No Clippy warnings or rustfmt violations
6. ✅ API matches IMPL008 specification exactly
7. ✅ Performance within PERF-010 targets (±20%)
8. ✅ Integration with wkmp-ui validated

---

## Dependencies on External Work

**None identified** - All prerequisite specifications complete:
- ✅ SPEC024 (architecture) complete
- ✅ SPEC025 (amplitude analysis) complete
- ✅ IMPL008 (API design) complete
- ✅ IMPL009 (amplitude analyzer) complete
- ✅ IMPL010 (parameter management) complete
- ✅ IMPL001 (database schema) includes import_metadata columns
- ✅ SPEC008 (library management) includes amplitude analysis section

---

End of scope statement
