# PLAN007: Dependencies Map

**Project:** wkmp-ai (Audio Ingest) Microservice Implementation
**Date:** 2025-10-28

---

## Dependency Categories

| Category | Count | Status Overview |
|----------|-------|-----------------|
| **Internal WKMP Modules** | 2 | ✅ Exist, APIs stable |
| **Rust Crates** | 15 | ✅ Most exist, 2 TBD |
| **External APIs** | 3 | ✅ Free tier available, rate limits apply |
| **External Binaries** | 2 | ⚠️ Optional, graceful degradation needed |
| **WKMP Specifications** | 9 | ✅ All exist (4 created in PLAN004) |
| **Database Tables** | 9 | ✅ Schema defined in IMPL001 |

**Total Dependencies:** 40

---

## Internal WKMP Module Dependencies

### wkmp-common (Shared Library)

**Status:** ✅ Exists, stable API

**What wkmp-ai Needs:**
- `models::*` - Database entity models (File, Passage, Song, Artist, Work, Album, etc.)
- `db::*` - Database connection handling, migrations, pool management
- `events::*` - Event types for SSE (if shared event definitions exist)
- `config::*` - Configuration loading (TOML → database settings)
- `audio::tick_conversion` - Convert seconds ↔ ticks (28,224,000 ticks/second)

**Integration Points:**
- Shared database models for consistency across all modules
- Tick conversion functions (sample-accurate timing)
- Configuration system (graceful degradation per IMPL007)

**Risk:** Low - wkmp-common is stable, well-tested

---

### wkmp-ui (User Interface Microservice)

**Status:** ✅ Exists, integration required

**What wkmp-ai Provides to wkmp-ui:**
- `/health` endpoint - Health check for "Import Music" button enablement
- Import session events - SSE or webhook for "import complete" notification

**What wkmp-ai Needs from wkmp-ui:**
- **Integration:**
  - "Import Music" button in library view
  - Opens http://localhost:5723 in new tab/window when clicked
  - Checks wkmp-ai health before enabling button

**Requirements:**
- AIA-MS-010: Microservices integration
- AIA-UI-020: Health check integration

**Coordination Needed:** ✅ PLAN006 clarified UI architecture, wkmp-ui changes required

**Risk:** Low-Medium - Requires coordinated changes in wkmp-ui (separate module)

---

## Rust Crate Dependencies

### Core HTTP & Async (P0 - Critical)

| Crate | Version | Purpose | Status | Risk |
|-------|---------|---------|--------|------|
| **tokio** | 1.35+ | Async runtime, background tasks, broadcast channels | ✅ Existing | Low |
| **axum** | 0.7+ | HTTP server, routing, SSE support | ✅ Existing | Low |
| **tower** | 0.4+ | Middleware for Axum (logging, timeouts) | ✅ Existing | Low |
| **tower-http** | 0.5+ | HTTP-specific middleware (CORS, compression) | ✅ Existing | Low |

**Notes:** These are already used in wkmp-ap and wkmp-ui, proven stable.

---

### Audio Processing (P0 - Critical)

| Crate | Version | Purpose | Status | Risk |
|-------|---------|---------|--------|------|
| **symphonia** | 0.5+ | Audio decoding (MP3, FLAC, OGG, Opus, WAV) | ✅ Existing (wkmp-ap) | Low |
| **rubato** | 0.14+ | Resampling to 44.1 kHz for analysis | ✅ Existing (wkmp-ap) | Low |

**Notes:** Already battle-tested in wkmp-ap playback engine, reuse same versions.

---

### Metadata & Fingerprinting (P0 - Critical)

| Crate | Version | Purpose | Status | Risk |
|-------|---------|---------|--------|------|
| **lofty** | 0.18+ | ID3/Vorbis/MP4 tag parsing | ✅ Mature, widely used | Low |
| **sha2** | 0.10+ | SHA-256 file hashing for deduplication | ✅ Standard library | Low |
| **chromaprint-sys-next** | 1.6+ | Chromaprint FFI bindings with static linking | ✅ Production-ready, official algorithm | Low |

**Notes:**
- lofty replaces older id3 crate, supports all major formats
- sha2 is cryptographic hash, well-tested
- chromaprint-sys-next statically links official chromaprint C library (no fpcalc binary needed)
- Build requirements: cmake, libfftw3-dev (documented in README)

---

### HTTP Client (P0 - Critical for External APIs)

| Crate | Version | Purpose | Status | Risk |
|-------|---------|---------|--------|------|
| **reqwest** | 0.11+ | HTTP client for MusicBrainz, AcoustID, AcousticBrainz | ✅ Industry standard | Low |

**Notes:**
- Use `reqwest::blocking` for simpler API client code
- Or `reqwest::Client` async for better performance (Tokio integration)

---

### Serialization (P0 - Critical)

| Crate | Version | Purpose | Status | Risk |
|-------|---------|---------|--------|------|
| **serde** | 1.0+ | JSON serialization for API responses, SSE events | ✅ Existing | Low |
| **serde_json** | 1.0+ | JSON parsing for MusicBrainz, AcousticBrainz responses | ✅ Existing | Low |

**Notes:** Already used extensively in wkmp-ap/wkmp-ui.

---

### Database (P0 - Critical)

| Crate | Version | Purpose | Status | Risk |
|-------|---------|---------|--------|------|
| **rusqlite** | 0.30+ | SQLite access | ✅ Existing (wkmp-common) | Low |
| **uuid** | 1.6+ | Session ID generation | ✅ Existing | Low |

**Notes:** wkmp-common wraps rusqlite, use shared connection pool.

---

### Web UI Assets (P0 - Critical for UI)

| Crate | Version | Purpose | Status | Risk |
|-------|---------|---------|--------|------|
| **tower-http** | 0.5+ | Static file serving for HTML/CSS/JS | ✅ Existing | Low |

**Alternative:** Embed assets with `include_str!()` macro

---

### Future / TBD (P1 - High Priority)

| Crate | Version | Purpose | Status | Risk |
|-------|---------|---------|--------|------|
| **waveform-renderer** | TBD | Waveform visualization data generation | ⚠️ May need custom | Medium |

**Options for Waveform Visualization:**
1. **Server-side:** Generate waveform PNG/SVG on server, send to browser
2. **Client-side:** Send PCM samples to browser, render with Canvas API
3. **Hybrid:** Send downsampled peak/RMS data, browser renders

**Recommendation:** Client-side Canvas API (no additional crate needed)

---

## External API Dependencies

### MusicBrainz API

**Endpoint:** https://musicbrainz.org/ws/2/
**Documentation:** https://musicbrainz.org/doc/MusicBrainz_API
**Authentication:** Optional (User-Agent header required)

**Rate Limits:**
- **Free Tier:** 1 request per second (enforced by API)
- **Authenticated:** 1 request per second (same limit)
- **Penalty:** Temporary IP ban for violations

**What We Need:**
- Recording lookup by MBID
- Artist, Work, Album metadata retrieval
- Release (album) search for track matching

**Status:** ✅ Free, stable, reliable
**Risk:** Low - Well-documented, widely used

**Mitigation:**
- Implement 1 req/s rate limiter (tokio::time::sleep)
- Cache responses in `musicbrainz_cache` table
- Retry on 503 (rate limit exceeded)

**Requirements:** AIA-COMP-010 (musicbrainz_client), AIA-PERF-020 (rate limiting)

---

### AcoustID API

**Endpoint:** https://api.acoustid.org/v2/lookup
**Documentation:** https://acoustid.org/webservice
**Authentication:** API key required (free)

**Rate Limits:**
- **Free Tier:** 3 requests per second
- **Higher Tiers:** Paid plans available (not needed for MVP)

**What We Need:**
- Fingerprint → Recording MBID lookup
- Confidence scores for multiple matches

**Status:** ✅ Free API key available
**Risk:** Low - Stable service, generous free tier

**Mitigation:**
- Cache responses in `acoustid_cache` table
- Store API key in `credentials` table (AIA-SEC-020)
- Retry on transient failures

**Requirements:** AIA-COMP-010 (fingerprinter), AIA-PERF-020 (caching), AIA-SEC-020 (API key storage)

---

### AcousticBrainz API

**Endpoint:** https://acousticbrainz.org/api/v1/
**Documentation:** https://acousticbrainz.org/data
**Authentication:** None

**Status:** ⚠️ **DEPRECATED - Service shut down in 2022**

**Impact:** Musical Flavor vector retrieval not available via AcousticBrainz

**Mitigation:**
1. **Essentia Fallback (Recommended):** Run Essentia analysis locally
   - Requires Essentia binary installation
   - Subprocess call to Essentia extractor
   - Parse JSON output
2. **Future Enhancement:** Train custom ML model for flavor extraction
3. **Short-term:** Musical Flavor optional, import proceeds without it

**Requirements:** AIA-COMP-010 (acousticbrainz_client, essentia_runner)

**Risk:** Medium - Requires Essentia binary or deferred feature

**Decision Required:** Proceed with Essentia-only approach or defer Musical Flavor to future release?

---

## External Binary Dependencies

### ~~Chromaprint (fpcalc)~~ → REPLACED WITH CRATE

**Decision:** Use `chromaprint-sys-next` crate with static linking instead of external fpcalc binary

**Original Approach (Rejected):**
- Subprocess call to fpcalc binary
- Required user installation: `apt install chromaprint-tools`
- Risk: Runtime dependency, subprocess overhead

**New Approach (Approved):**
- `chromaprint-sys-next` Rust crate with static linking
- Produces single self-contained binary (no fpcalc needed at runtime)
- Uses official chromaprint C library (production quality)
- Build requirements: cmake, libfftw3-dev (build-time only, documented in README)

**Deployment:**
- ✅ Single wkmp-ai binary (chromaprint statically linked)
- ✅ No runtime dependencies
- ✅ Simplified cross-compilation for Pi Zero2W

**Status:** ✅ Production-ready, official algorithm
**Risk:** ✅ Low - Static linking eliminates runtime dependency

**Requirements:** AIA-COMP-010 (fingerprinter)

---

### Essentia (Optional)

**Binary:** `essentia_streaming_extractor_music`
**Purpose:** Musical Flavor vector extraction (fallback for AcousticBrainz)
**Installation:**
- Linux: Compile from source or use Docker image
- macOS/Windows: Limited support

**Detection Strategy:**
```bash
# Check if Essentia is available
which essentia_streaming_extractor_music

# If found, run analysis:
essentia_streaming_extractor_music audio_file.mp3 output.json
```

**Graceful Degradation:**
- If Essentia not found: Musical Flavor remains NULL, import proceeds
- Log info: "Essentia not available, musical flavor analysis skipped"

**Status:** ⚠️ Optional, harder to install than Chromaprint
**Risk:** Medium - Complex installation, may not be available

**Requirements:** AIA-COMP-010 (essentia_runner)

**Decision Required:** Make Musical Flavor optional for MVP or require Essentia installation?

---

## WKMP Specification Dependencies

### Tier 1 - Requirements (Authoritative)

| Document | Status | Purpose | Dependencies on wkmp-ai |
|----------|--------|---------|-------------------------|
| **REQ001-requirements.md** | ✅ Exists | Top-level requirements | Audio ingest requirements (added in PLAN004) |
| **REQ002-entity_definitions.md** | ✅ Exists | Entity models (Passage, Song, Artist, etc.) | Database model definitions |

---

### Tier 2 - Design Specifications

| Document | Status | Purpose | Dependencies on wkmp-ai |
|----------|--------|---------|-------------------------|
| **SPEC024-audio_ingest_architecture.md** | ✅ Exists (Updated PLAN006) | wkmp-ai architecture (this plan's source) | Primary specification |
| **SPEC025-amplitude_analysis.md** | ✅ Created in PLAN004 | Amplitude-based lead-in/lead-out detection | AIA-COMP-010 (amplitude_analyzer) |
| **SPEC008-library_management.md** | ✅ Exists (Updated PLAN006) | File discovery workflows | AIA-INT-010 |

---

### Tier 3 - Implementation Specifications

| Document | Status | Purpose | Dependencies on wkmp-ai |
|----------|--------|---------|-------------------------|
| **IMPL001-database_schema.md** | ✅ Exists | Database tables, migrations | AIA-DB-010 (all table definitions) |
| **IMPL005-audio_file_segmentation.md** | ✅ Exists (Updated PLAN006) | Segmentation workflow Steps 1-5 | AIA-INT-020 (UI workflow) |
| **IMPL008-audio_ingest_api.md** | ✅ Created in PLAN004 | HTTP API endpoints, SSE events | AIA-UI-010, AIA-SSE-010, AIA-POLL-010 |
| **IMPL009-amplitude_analyzer_implementation.md** | ✅ Created in PLAN004 | RMS calculation, A-weighting, lead-in/lead-out | AIA-COMP-010 (amplitude_analyzer) |
| **IMPL010-parameter_management.md** | ✅ Created in PLAN004 | Parameter storage, global defaults, per-file overrides | AIA-COMP-010 (parameter_manager) |
| **IMPL011-musicbrainz_client.md** | ✅ Created in PLAN004 | MusicBrainz API client (rate limiting, caching) | AIA-COMP-010 (musicbrainz_client) |
| **IMPL012-acoustid_client.md** | ✅ Created in PLAN004 | AcoustID API client (fingerprint → MBID) | AIA-COMP-010 (fingerprinter integration) |
| **IMPL013-file_scanner.md** | ✅ Created in PLAN004 | Directory traversal, file filtering, security | AIA-COMP-010 (file_scanner), AIA-SEC-010 |
| **IMPL014-database_queries.md** | ✅ Created in PLAN004 | SQL queries, tick conversion, batch inserts | AIA-DB-010, AIA-INT-030 |

**Notes:** PLAN004 created IMPL008-014 to fill specification gaps identified in Phase 2 analysis.

---

## Database Table Dependencies

**Source:** IMPL001-database_schema.md

### Tables Written by wkmp-ai (AIA-DB-010)

| Table | Purpose | Key Columns | Foreign Keys |
|-------|---------|-------------|--------------|
| **files** | Audio file metadata | id (UUID), path, hash (SHA-256), duration_ticks | - |
| **passages** | Passage definitions | id (UUID), file_id, start_ticks, end_ticks, lead_in_ticks, lead_out_ticks, musical_flavor_vector (JSON) | file_id → files.id |
| **songs** | MusicBrainz recordings | id (UUID), mbid, title | - |
| **artists** | MusicBrainz artists | id (UUID), mbid, name | - |
| **works** | MusicBrainz works | id (UUID), mbid, title | - |
| **albums** | MusicBrainz releases | id (UUID), mbid, title | - |
| **passage_songs** | Passage-song associations | passage_id, song_id | passage_id → passages.id, song_id → songs.id |
| **passage_albums** | Passage-album associations | passage_id, album_id | passage_id → passages.id, album_id → albums.id |
| **acoustid_cache** | AcoustID response cache | fingerprint (PK), response_json, cached_at | - |
| **musicbrainz_cache** | MusicBrainz response cache | mbid (PK), response_json, cached_at | - |
| **acousticbrainz_cache** | AcousticBrainz response cache (unused if service down) | mbid (PK), response_json, cached_at | - |

**Total Tables Written:** 11 (including 3 cache tables)

---

### Tables Read by wkmp-ai

| Table | Purpose | Columns Read |
|-------|---------|--------------|
| **settings** | Global import parameters | key, value (JSON) |
| **credentials** | API keys (AcoustID) | service, key_encrypted |

**Total Tables Read:** 2

---

## Dependency Risk Assessment

### Critical Path Dependencies (Blocks MVP)

| Dependency | Type | Risk Level | Mitigation |
|------------|------|------------|------------|
| **tokio, axum** | Rust crate | ✅ Low | Already used in wkmp-ap, stable |
| **symphonia, rubato** | Rust crate | ✅ Low | Already used in wkmp-ap, proven |
| **lofty** | Rust crate | ✅ Low | Mature, widely used |
| **reqwest** | Rust crate | ✅ Low | Industry standard HTTP client |
| **rusqlite** | Rust crate | ✅ Low | Already used in wkmp-common |
| **MusicBrainz API** | External API | ✅ Low | Stable, caching mitigates outages |
| **AcoustID API** | External API | ✅ Low | Free tier sufficient, caching |
| **IMPL001-014** | Specifications | ✅ Low | All exist, created in PLAN004 |

**Overall Critical Path Risk:** ✅ **Low** - All critical dependencies exist and are stable.

---

### High-Priority Dependencies (Affects Quality)

| Dependency | Type | Risk Level | Mitigation |
|------------|------|------------|------------|
| **~~Chromaprint (fpcalc)~~** | ~~External binary~~ | ✅ **RESOLVED** | Static linking via chromaprint-sys-next |
| **wkmp-ui integration** | Internal module | ⚠️ Medium | Coordination needed, clear API contract |
| **Waveform visualization** | Feature | ✅ **RESOLVED** | Client-side Canvas API (no extra dependency) |

**Overall High-Priority Risk:** ✅ **Low** - Major dependencies resolved via static linking and client-side rendering.

---

### Required Dependencies (Musical Flavor via Essentia)

| Dependency | Type | Risk Level | Impact if Missing |
|------------|------|------------|-------------------|
| **Essentia** | External binary | ⚠️ Medium | Musical Flavor unavailable, **import fails per Decision 1** |
| **AcousticBrainz API** | External API | ❌ High | Service shut down, Essentia **required** for flavor |

**Decision:** Musical Flavor via Essentia is **required** for MVP (Decision 1: Option A)
**Overall Risk:** ⚠️ **Medium** - Essentia installation required, documented in README

---

## Action Items from Dependencies Analysis

### Decisions Approved (User Confirmed)

1. ✅ **Musical Flavor:** **Option A - Essentia required**
   - Document installation in README
   - Essentia runtime detection mandatory
   - Import fails if Essentia unavailable (clear error message)

2. ✅ **Waveform Visualization:** **Option B - Client-side Canvas API**
   - No server-side image generation
   - Vanilla JavaScript (no framework)
   - No additional Rust crate needed

3. ✅ **Chromaprint:** **Static linking via chromaprint-sys-next**
   - No fpcalc binary needed at runtime
   - Single self-contained wkmp-ai binary
   - Build requirements: cmake, libfftw3-dev

### Before Phase 2 (Immediate)

4. **Verify:** wkmp-ui team aware of integration requirements
   - "Import Music" button implementation
   - Health check endpoint usage
   - Launch in new tab/window behavior

### During Implementation

5. **Implement:** Essentia runtime detection with clear error handling
   - Check for essentia_streaming_extractor_music in PATH
   - If missing: Display error, link to installation instructions
   - No graceful degradation (Musical Flavor required per Decision 1)

6. **Document:** Installation instructions in README
   - Build requirements: cmake, libfftw3-dev (for chromaprint static linking)
   - Runtime requirement: Essentia binary
   - Cross-compilation notes for Pi Zero2W

7. **Test:** Missing Essentia binary scenario
   - Error message clarity
   - Installation link provided

---

## Summary

**Dependencies Status:**
- ✅ **40 dependencies identified**
- ✅ **39 dependencies exist and are stable** (97.5%)
- ✅ **Chromaprint resolved** - Static linking via chromaprint-sys-next (no runtime dependency)
- ✅ **Waveform visualization resolved** - Client-side Canvas API (no extra crate)
- ⚠️ **1 external binary required** - Essentia (Musical Flavor extraction per Decision 1)
- ❌ **AcousticBrainz API unavailable** - Service shut down 2022, Essentia required

**Overall Risk:** ✅ **Low** - All critical path dependencies resolved, Essentia documented requirement.

**Blockers:** ❌ **None** - All P0 dependencies available.

**Approved Decisions:**
1. ✅ Musical Flavor via Essentia required (Option A)
2. ✅ Client-side Canvas API for waveform visualization (Option B)
3. ✅ chromaprint-sys-next with static linking (Option B)
4. ⚠️ Coordinate with wkmp-ui team on integration requirements

---

**End of Dependencies Map**
