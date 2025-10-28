# Dependencies Map - PLAN004

**Plan:** PLAN004 - wkmp-ai Audio Ingest Implementation
**Date:** 2025-10-27

---

## Rust Crate Dependencies

### HTTP Server & Async Runtime

| Crate | Version | Purpose | Usage |
|-------|---------|---------|-------|
| **tokio** | 1.x | Async runtime | Background jobs, task spawning, channels |
| **axum** | 0.7.x | Web framework | HTTP routes, SSE endpoint, JSON handling |
| **tower** | 0.4.x | Middleware | Request logging, CORS, timeouts |
| **tower-http** | 0.5.x | HTTP middleware | CORS, compression, tracing |

### Database

| Crate | Version | Purpose | Usage |
|-------|---------|---------|-------|
| **sqlx** | 0.7.x | Database client | SQLite access, migrations, queries |
| **uuid** | 1.x | Unique IDs | Session IDs, entity primary keys |

### Audio Processing

| Crate | Version | Purpose | Usage |
|-------|---------|---------|-------|
| **symphonia** | 0.5.x | Audio decoder | Decode MP3/FLAC/OGG/AAC to PCM |
| **rubato** | 0.14.x | Audio resampler | Resample to 44.1kHz for Chromaprint |
| **dasp** | 0.11.x | DSP utilities | RMS calculation, signal processing |
| **chromaprint** | 0.x | Audio fingerprinting | Generate Chromaprint fingerprints |

### Metadata Extraction

| Crate | Version | Purpose | Usage |
|-------|---------|---------|-------|
| **lofty** | 0.x | Tag parsing | Extract ID3/Vorbis/MP4 tags |
| **image** | 0.x | Image decoding | Cover art extraction and validation |

### External API Clients

| Crate | Version | Purpose | Usage |
|-------|---------|---------|-------|
| **reqwest** | 0.11.x | HTTP client | MusicBrainz/AcoustID/AcousticBrainz APIs |
| **serde** | 1.x | Serialization | JSON parsing, API responses |
| **serde_json** | 1.x | JSON handling | Musical flavor vectors, parameters |

### Utilities

| Crate | Version | Purpose | Usage |
|-------|---------|---------|-------|
| **thiserror** | 1.x | Error handling | Custom error types with context |
| **tracing** | 0.1.x | Structured logging | Debug logging, error tracking |
| **walkdir** | 2.x | File traversal | Recursive directory scanning |
| **sha2** | 0.10.x | Hashing | File hash calculation (SHA-256) |

---

## External API Dependencies

### AcoustID
- **Endpoint:** `https://api.acoustid.org/v2/lookup`
- **Purpose:** Audio fingerprint → MusicBrainz Recording MBID
- **Rate Limit:** 3 requests/second
- **Authentication:** API key required (environment variable)
- **Response Caching:** `acoustid_cache` table
- **Error Handling:** Network timeout, API errors, no matches

### MusicBrainz
- **Endpoint:** `https://musicbrainz.org/ws/2/`
- **Purpose:** Recording/artist/work/album metadata retrieval
- **Rate Limit:** 1 request/second (STRICT - primary bottleneck)
- **Authentication:** None (public API)
- **Response Caching:** `musicbrainz_cache` table
- **Error Handling:** Rate limit violations, no metadata found, network errors

### AcousticBrainz
- **Endpoint:** `https://acousticbrainz.org/api/v1/`
- **Purpose:** Musical flavor vector retrieval
- **Rate Limit:** None specified (use conservative 1 req/s)
- **Authentication:** None (public API)
- **Response Caching:** `acousticbrainz_cache` table
- **Error Handling:** No data available (~40% miss rate expected)

---

## Internal WKMP Module Dependencies

### wkmp-common (Shared Library)
- **Purpose:** Shared database models, events, utilities
- **Used Types:**
  - Database models (File, Passage, Song, Artist, Work, Album)
  - Event types (broadcast channel events)
  - Utility functions (tick conversion)
- **Relationship:** Compile-time dependency

### wkmp-ui (User Interface)
- **Port:** 5720
- **Integration:** wkmp-ui orchestrates import workflow
- **Communication:** HTTP client → wkmp-ai server
- **Endpoints Used:**
  - `POST /import/start` - Begin import
  - `GET /events?session_id={id}` - Subscribe to SSE
  - `GET /import/status/{id}` - Poll progress
  - `POST /import/cancel/{id}` - Cancel import
- **Relationship:** Runtime dependency (HTTP communication)

### Shared Database (wkmp.db)
- **Location:** Root folder (configured in settings)
- **Tables Written:**
  - `files` - Audio file metadata
  - `passages` - Passage definitions with timing
  - `songs`, `artists`, `works`, `albums` - MusicBrainz entities
  - `passage_songs`, `passage_albums` - Relationships
  - `acoustid_cache`, `musicbrainz_cache`, `acousticbrainz_cache` - API caches
- **Tables Read:**
  - `settings` - Import parameters (global defaults)
- **Concurrency:** Single-user import (no concurrent sessions)

---

## Specification Dependencies

### Authoritative Specifications (Tier 2)

| Spec | Lines | Purpose | Usage |
|------|-------|---------|-------|
| **SPEC024** | 476 | wkmp-ai architecture | Primary design specification |
| **SPEC025** | 654 | Amplitude analysis | RMS algorithm, lead-in/lead-out detection |
| **SPEC008** | 512+ | Library management | File discovery, metadata, fingerprinting |

### Implementation Specifications (Tier 3)

| Spec | Lines | Purpose | Usage |
|------|-------|---------|-------|
| **IMPL008** | 210 | Audio ingest API | HTTP endpoint definitions |
| **IMPL009** | 407 | Amplitude analyzer | Rust implementation code |
| **IMPL010** | 275 | Parameter management | Parameter storage/loading |
| **IMPL005** | N/A | Audio segmentation | Silence detection, boundary detection |
| **IMPL001** | N/A | Database schema | Table structures, queries |
| **IMPL002** | N/A | Coding conventions | Code style, patterns |

### Requirements (Tier 1)

| Spec | Lines | Purpose | Usage |
|------|-------|---------|-------|
| **REQ001** | N/A | Requirements | Authoritative requirements (REQ-PI-061 to 064) |
| **REQ002** | N/A | Entity definitions | Passage, Song, File definitions |

---

## Development Environment Dependencies

### Build Tools
- **Rust Toolchain:** Stable 1.70+ (async/await, const generics)
- **Cargo:** Package manager and build system
- **rustfmt:** Code formatting (enforced)
- **clippy:** Linting (warnings = errors)

### Database Tools
- **SQLite 3.x:** Database engine with JSON1 extension
- **sqlx-cli:** Migration runner and compile-time query verification

### Testing
- **cargo test:** Unit/integration test runner
- **cargo-nextest:** Fast test runner (optional)
- **Sample audio files:** Various formats for testing

---

## Risk Assessment - Dependency Failures

### High Risk (Mitigation Required)

1. **MusicBrainz Rate Limit Violations**
   - Risk: IP ban, import failure
   - Mitigation: Implement request throttling (1 req/s enforced), exponential backoff

2. **symphonia Decode Failures**
   - Risk: Skip files, incomplete import
   - Mitigation: Comprehensive error handling, warn user, continue processing

3. **Database Contention**
   - Risk: SQLite BUSY errors during concurrent writes
   - Mitigation: Batch inserts, transaction management, retry logic

### Medium Risk (Monitor)

4. **AcoustID API Unavailability**
   - Risk: No MusicBrainz MBID, passage created without song link
   - Mitigation: Graceful degradation, warn user, allow manual MBID entry later

5. **Network Timeouts**
   - Risk: Import hangs on slow connections
   - Mitigation: 30-second timeouts, retry with exponential backoff

### Low Risk (Accept)

6. **Missing AcousticBrainz Data**
   - Risk: No musical flavor (~40% of recordings)
   - Mitigation: Warn user, passage created without flavor (PD will skip)

7. **Chromaprint Build Issues**
   - Risk: Platform-specific build failures
   - Mitigation: Pre-built binaries for common platforms

---

## Dependency Update Policy

**Conservative approach:**
- Pin major versions (e.g., `axum = "0.7"` not `"*"`)
- Test minor updates in CI before merging
- Document breaking changes in migration guide

**Critical security updates:**
- Apply immediately (e.g., tokio, reqwest vulnerabilities)
- Test compatibility, update dependencies.md

---

End of dependencies map
