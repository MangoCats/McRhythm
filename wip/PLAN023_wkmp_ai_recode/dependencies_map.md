# Dependencies Map: PLAN023 - WKMP-AI Ground-Up Recode

**Plan:** PLAN023 - wkmp-ai Ground-Up Recode
**Created:** 2025-01-08

---

## Existing Code Dependencies

### wkmp-common (Shared Library)

**Status:** ✅ Exists - Read-Only Dependency

**What We Need:**
- Database connection pool utilities
- Event bus abstractions (`tokio::broadcast`)
- Configuration utilities (`RootFolderResolver`, `RootFolderInitializer`)
- Shared data models (if any relevant to import)

**Usage in This Plan:**
- Database operations: Use `wkmp_common::db` pool management
- Event broadcasting: Use `wkmp_common::events` utilities
- Configuration: Use `wkmp_common::config::RootFolderResolver` for database path

**Impact:** No changes to wkmp-common required

---

### Database Schema (wkmp.db)

**Status:** ✅ Exists - Will Be Extended

**Existing Tables We Depend On:**
- `passages` table (current schema per IMPL001-database_schema.md)
  - Primary key: `id TEXT`
  - File reference: `file_path TEXT`
  - Timing: `start_tick INTEGER`, `end_tick INTEGER`
  - Musical flavor: `musical_flavor_json TEXT`

**What We Will Add:**
- 13 new columns to `passages` table (see REQ-AI-080 series)
- New `import_provenance` table

**Migration Required:**
```sql
ALTER TABLE passages ADD COLUMN flavor_source_blend TEXT;
ALTER TABLE passages ADD COLUMN flavor_confidence_map TEXT;
-- (11 more columns)

CREATE TABLE import_provenance (...);
```

**Impact:** Database migration script required (non-destructive)

---

### Existing wkmp-ai Code

**Status:** ✅ Exists - Reference Only (DO NOT COPY)

**What We Reference (for understanding only):**
- HTTP API contracts (POST /import/start, GET /import/status, GET /import/events)
- SSE event format expectations
- Database passage record structure
- Audio file scanning logic (file discovery patterns)

**What We DO NOT Copy:**
- 7-phase linear workflow logic
- Existing extractor implementations
- Legacy fusion (or lack thereof)
- Current UI code

**Critical Rule:** Keep existing wkmp-ai source as reference, but all code in this plan is rewritten from scratch.

---

## External Library Dependencies

### Rust Crates (Cargo.toml)

**Runtime Dependencies:**

**Core Framework:**
- `tokio` (>= 1.35) - ✅ Already in WKMP
  - Features: `full` (async runtime, I/O, sync primitives)
- `axum` (>= 0.7) - ✅ Already in WKMP
  - HTTP server framework
  - SSE support via `axum::response::sse`

**Audio Processing:**
- `symphonia` (>= 0.5) - ✅ Already in WKMP
  - Audio decoding (MP3, FLAC, OGG, M4A, WAV)
- `chromaprint-rust` OR `acoustid` crate - ⚠️ **Need to verify**
  - Chromaprint fingerprint generation
  - **Action:** Check if already in wkmp-ai dependencies

**Database:**
- `sqlx` (>= 0.7) with `sqlite` feature - ✅ Already in WKMP
  - Async SQLite operations
  - JSON1 extension support

**HTTP Client:**
- `reqwest` (>= 0.11) - ✅ Likely already in WKMP
  - HTTP requests to AcoustID, MusicBrainz, AcousticBrainz APIs
  - Features: `json`, `rustls-tls`

**Serialization:**
- `serde` (>= 1.0) with `derive` feature - ✅ Already in WKMP
- `serde_json` (>= 1.0) - ✅ Already in WKMP
  - JSON serialization for SSE events, database provenance

**String Similarity:**
- `strsim` (>= 0.11) - ⚠️ **Need to add**
  - Levenshtein distance for fuzzy title matching (REQ-AI-061)
  - **Action:** Add to Cargo.toml

**UUID:**
- `uuid` (>= 1.6) with `v4, serde` features - ✅ Already in WKMP
  - Passage IDs, session IDs

**Error Handling:**
- `anyhow` (>= 1.0) - ✅ Already in WKMP
  - Error context and propagation

**Logging:**
- `tracing` (>= 0.1) - ✅ Already in WKMP
- `tracing-subscriber` (>= 0.3) - ✅ Already in WKMP

**Optional Dependencies:**

- `essentia-sys` OR `essentia-rust` - ⚠️ **Optional, check availability**
  - Musical flavor computation (if Essentia library installed)
  - **Status:** Optional dependency, graceful degradation if missing
  - **Action:** Research available Rust bindings for Essentia

---

### External APIs

**AcoustID API**
- **URL:** https://api.acoustid.org/v2/lookup
- **Authentication:** API key required (obtain from acoustid.org)
- **Rate Limiting:** 3 requests/second recommended
- **Response:** JSON with Recording MBID + confidence score
- **Status:** ✅ Publicly available, free tier
- **Action:** Obtain API key before testing

**MusicBrainz API**
- **URL:** https://musicbrainz.org/ws/2/recording/{mbid}
- **Authentication:** None required (user-agent string recommended)
- **Rate Limiting:** 1 request/second enforced, IP ban if exceeded
- **Response:** XML or JSON (specify format=json) with metadata
- **Status:** ✅ Publicly available, free
- **Action:** Implement 1-second throttling between requests

**AcousticBrainz API**
- **URL:** https://acousticbrainz.org/api/v1/{mbid}/high-level
- **Authentication:** None required
- **Rate Limiting:** Reasonable use expected
- **Response:** JSON with musical flavor characteristics
- **Status:** ⚠️ Service ended 2022 (read-only archive)
  - Pre-2022 recordings available
  - Post-2022 recordings return 404
- **Action:** Handle 404 gracefully, fallback to Essentia

---

### External Libraries (System-Level)

**Essentia (Optional)**
- **Type:** C++ library with Python/Rust bindings
- **Installation:** User must install separately (apt, brew, source)
- **Status:** ⚠️ Optional dependency
  - If installed: Use for musical flavor computation
  - If missing: Gracefully degrade (use Audio-derived + ID3-derived only)
- **Action:** Detect at runtime, do not require for compilation

**Chromaprint (System Library)**
- **Type:** C library for audio fingerprinting
- **Installation:** May be system-level dependency
- **Status:** ⚠️ Check if Rust crate includes bundled library
- **Action:** Verify `chromaprint-rust` or `acoustid` crate includes library or links to system lib

---

## Dependency Status Summary

| Dependency | Type | Status | Action Required |
|------------|------|--------|-----------------|
| tokio | Crate | ✅ Exists | None |
| axum | Crate | ✅ Exists | None |
| symphonia | Crate | ✅ Exists | None |
| sqlx | Crate | ✅ Exists | None |
| reqwest | Crate | ✅ Exists | Verify |
| serde / serde_json | Crate | ✅ Exists | None |
| uuid | Crate | ✅ Exists | None |
| anyhow | Crate | ✅ Exists | None |
| tracing | Crate | ✅ Exists | None |
| chromaprint-rust | Crate | ⚠️ Unknown | Research, add if needed |
| strsim | Crate | ❌ Missing | Add to Cargo.toml |
| essentia-rust | Crate | ⚠️ Optional | Research bindings |
| wkmp-common | Internal | ✅ Exists | Read-only use |
| passages table | Database | ✅ Exists | Will extend (migration) |
| AcoustID API | External | ✅ Available | Obtain API key |
| MusicBrainz API | External | ✅ Available | Implement rate limiting |
| AcousticBrainz API | External | ⚠️ Archive | Handle 404 gracefully |
| Essentia library | System | ⚠️ Optional | Detect at runtime |

---

## New Code Structure

**Ground-Up Recode Directory Structure:**

```
wkmp-ai/
  src/
    main.rs                           # NEW: HTTP server with SSE endpoints
    import/
      mod.rs                          # NEW
      orchestrator.rs                 # NEW: File-level coordination
      per_song_engine.rs              # NEW: Per-song workflow
    fusion/
      mod.rs                          # NEW
      extractors/                     # NEW: Tier 1
        mod.rs
        id3_extractor.rs              # NEW
        chromaprint_analyzer.rs       # NEW
        acoustid_client.rs            # NEW
        musicbrainz_client.rs         # NEW
        essentia_analyzer.rs          # NEW (optional feature)
        audio_derived_extractor.rs    # NEW
        id3_genre_mapper.rs           # NEW
      fusers/                         # NEW: Tier 2
        mod.rs
        identity_resolver.rs          # NEW: Bayesian update
        metadata_fuser.rs             # NEW: Field-wise selection
        flavor_synthesizer.rs         # NEW: Characteristic-wise averaging
        boundary_fuser.rs             # NEW: Multi-strategy (baseline: silence only)
      validators/                     # NEW: Tier 3
        mod.rs
        consistency_validator.rs      # NEW
        quality_scorer.rs             # NEW
        conflict_detector.rs          # NEW
    events/
      mod.rs                          # NEW
      import_events.rs                # NEW: Event type definitions
      sse_broadcaster.rs              # NEW: SSE logic
    db/
      mod.rs                          # NEW
      passage_repository.rs           # NEW: Database operations
      provenance_logger.rs            # NEW: Import provenance logging
  Cargo.toml                          # MODIFY: Add strsim, verify chromaprint
```

**All files above are NEW (ground-up recode).** No copying from existing wkmp-ai code.

---

## Integration Points

### Where This Plan Interacts with Existing WKMP

**1. Database (wkmp.db)**
- **Integration:** Extend `passages` table with migration
- **Coordination:** No conflicts (new columns)
- **Testing:** Verify migration does not break wkmp-ap playback

**2. HTTP Server (Port 5723)**
- **Integration:** Keep existing HTTP endpoints for compatibility
- **Coordination:** SSE endpoint format must match UI expectations
- **Testing:** Verify UI can consume new SSE events

**3. wkmp-ui (User Interface)**
- **Integration:** UI may need updates to display new provenance data
- **Coordination:** SSE event format backward compatible (if possible)
- **Out of Scope:** UI changes not in this plan

**4. wkmp-common (Shared Library)**
- **Integration:** Use database pool, event bus utilities
- **Coordination:** Read-only dependency, no changes
- **Testing:** Verify no regressions in wkmp-common usage

---

## Dependency Resolution Tasks

**Before Implementation Begins:**

1. **Verify Existing Dependencies**
   - [ ] Confirm `reqwest` is in wkmp-ai Cargo.toml
   - [ ] Confirm `symphonia` audio decoding works for test files
   - [ ] Confirm `sqlx` JSON1 extension enabled

2. **Add New Dependencies**
   - [ ] Add `strsim` to Cargo.toml (for fuzzy string matching)
   - [ ] Research and add Chromaprint crate (`chromaprint-rust`, `acoustid`, or similar)
   - [ ] Research Essentia Rust bindings (optional feature)

3. **Obtain API Keys**
   - [ ] Register for AcoustID API key (https://acoustid.org/new-application)
   - [ ] Document API key configuration (environment variable or config file)

4. **Database Migration**
   - [ ] Create migration script (ALTER TABLE + CREATE TABLE)
   - [ ] Test migration on development database
   - [ ] Verify rollback procedure (if needed)

5. **Test External APIs**
   - [ ] Verify AcoustID API accessible (test fingerprint lookup)
   - [ ] Verify MusicBrainz API accessible (test MBID query)
   - [ ] Confirm AcousticBrainz API returns 404 for post-2022 (expected behavior)

6. **Optional Dependency Detection**
   - [ ] Implement Essentia runtime detection (check for library presence)
   - [ ] Graceful degradation test (run without Essentia installed)

---

## Risks Related to Dependencies

**R1: Chromaprint Rust Bindings Quality**
- **Risk:** Available Rust crates may be unmaintained or incomplete
- **Mitigation:** Research multiple options (chromaprint-rust, acoustid, FFI bindings)
- **Fallback:** If no good Rust crate, use FFI to C library directly

**R2: Essentia Rust Bindings Availability**
- **Risk:** Essentia is primarily C++/Python, Rust bindings may not exist
- **Mitigation:** Make Essentia optional, use Audio-derived + ID3-derived as fallback
- **Fallback:** Defer Essentia integration if bindings unavailable

**R3: API Rate Limiting**
- **Risk:** AcoustID or MusicBrainz may ban IP if rate limits exceeded
- **Mitigation:** Implement throttling (1 req/sec for MusicBrainz, 3 req/sec for AcoustID)
- **Fallback:** Exponential backoff on 429 responses

**R4: AcousticBrainz Obsolescence**
- **Risk:** AcousticBrainz may go fully offline (currently read-only archive)
- **Mitigation:** Multi-source flavor synthesis handles absence gracefully
- **Fallback:** Essentia + Audio-derived + ID3-derived still functional

**R5: Database Migration Failure**
- **Risk:** ALTER TABLE may fail on large passages table
- **Mitigation:** Test migration on copy of production database
- **Fallback:** Rollback migration, investigate issue

---

## Documentation Dependencies

**Existing WKMP Documentation We Reference:**

- **SPEC003-musical_flavor.md** - Musical flavor definitions ([MFL-DEF-030], [MFL-DEF-040])
- **REQ002-entity_definitions.md** - Entity definitions ([ENT-MP-030], [ENT-CNST-010])
- **IMPL001-database_schema.md** - Current passages table schema
- **GOV002-requirements_enumeration.md** - Requirement numbering scheme

**Analysis Documents (Source for This Spec):**

- **wip/hybrid_import_analysis.md** - 3-tier fusion architecture design
- **wip/per_song_import_analysis/** - Per-song workflow design

**Status:** All documentation dependencies exist and are read-only.

---

**Document Version:** 1.0
**Last Updated:** 2025-01-08
