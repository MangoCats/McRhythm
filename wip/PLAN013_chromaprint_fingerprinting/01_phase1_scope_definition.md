# PLAN013 Phase 1: Input Validation & Scope Definition

**Specification:** docs/IMPL012-acoustid_client.md
**Date:** 2025-10-30
**Status:** Phase 1 Complete

---

## Executive Summary

**Problem:** Audio fingerprinting completely unimplemented - `fingerprinter.rs` contains placeholder code returning dummy bytes, causing 100% AcoustID lookup failures.

**Specification:** IMPL012-acoustid_client.md provides complete implementation guide for Chromaprint fingerprinting and AcoustID API integration.

**Complexity Justification for /plan:**
- FFI integration (chromaprint-sys-next C bindings)
- Multi-stage audio pipeline (decode → resample → fingerprint)
- Multiple failure modes (I/O, decode, resample, network, API errors)
- Performance considerations (parallel processing, memory usage, caching)
- >10 distinct requirements identified

**Dependencies Already Present:** chromaprint-sys-next, symphonia, rubato, reqwest, sha2 (per Cargo.toml lines 30-42)

---

## Requirements Extraction

### Audio Processing Pipeline (Core Requirements)

**REQ-FP-010:** Audio Decoding
- SHALL decode audio files using Symphonia to PCM samples
- SHALL support all formats Symphonia supports (MP3, FLAC, OGG, AAC, etc.)
- SHALL extract default audio track
- SHALL decode maximum 120 seconds per file (Chromaprint recommendation)

**REQ-FP-020:** Audio Resampling
- SHALL resample decoded audio to 44.1kHz sample rate
- SHALL convert audio to mono (single channel)
- SHALL use Rubato SincFixedIn resampler with specified interpolation parameters:
  - sinc_len: 256
  - f_cutoff: 0.95
  - interpolation: Linear
  - oversampling_factor: 256
  - window: BlackmanHarris2

**REQ-FP-030:** Chromaprint Fingerprinting
- SHALL generate fingerprint using chromaprint-sys-next crate
- SHALL use Algorithm::Test2
- SHALL convert f32 samples to i16 for Chromaprint API
- SHALL return Base64-encoded fingerprint string
- SHALL start Chromaprint context with 44100 Hz, 1 channel

**REQ-FP-040:** Error Handling - Fingerprinting
- SHALL define FingerprintError enum with variants:
  - IoError (file access failures)
  - DecodeError (Symphonia failures)
  - ResampleError (Rubato failures)
  - ChromaprintError (Chromaprint library failures)
- SHALL propagate errors to caller with context

### AcoustID API Client (Network Requirements)

**REQ-AC-010:** API Communication
- SHALL POST to https://api.acoustid.org/v2/lookup
- SHALL use application/x-www-form-urlencoded content type
- SHALL pass required parameters: client (API key), duration (seconds), fingerprint
- SHALL request metadata: recordings+recordingids
- SHALL set 30-second HTTP timeout

**REQ-AC-020:** Response Parsing
- SHALL parse JSON response into typed structs (AcoustIDResponse, AcoustIDResult, AcoustIDRecording)
- SHALL check status field == "ok"
- SHALL extract results array

**REQ-AC-030:** MBID Selection
- SHALL find result with highest score
- SHALL require minimum score threshold of 0.5 (50% confidence)
- SHALL extract first recording MBID from best result
- SHALL return None if no results meet threshold

**REQ-AC-040:** Error Handling - API
- SHALL define AcoustIDError enum with variants:
  - NetworkError (HTTP/network failures)
  - ApiError (non-200 status or invalid status field)
  - ParseError (JSON parsing failures)
  - DatabaseError (cache access failures)
- SHALL propagate errors to caller with context

### Caching Layer (Performance Requirements)

**REQ-CA-010:** Cache Lookup
- SHALL check database cache before API call
- SHALL use SHA-256 hash of fingerprint as cache key
- SHALL query acoustid_cache table by fingerprint_hash
- SHALL return cached MBID if found

**REQ-CA-020:** Cache Storage
- SHALL cache successful fingerprint → MBID mappings
- SHALL use UPSERT (INSERT ... ON CONFLICT DO UPDATE)
- SHALL store fingerprint_hash, mbid, cached_at timestamp
- SHALL update cached_at on conflict

**REQ-CA-030:** Fingerprint Hashing
- SHALL use SHA-256 algorithm
- SHALL output lowercase hexadecimal string (64 characters)
- SHALL be deterministic (same fingerprint → same hash)

### Build Requirements (Tooling)

**REQ-BLD-010:** LLVM/Clang Dependency
- SHALL require LLVM/Clang at build time for chromaprint-sys-next bindgen
- SHALL support Windows, Linux, macOS installations
- SHALL document installation commands per platform
- NOTE: Only build-time dependency - runtime binary does not require LLVM

### Performance Requirements

**REQ-PERF-010:** Processing Time
- SHOULD process 3-minute MP3 in 2-5 seconds (decode + resample + fingerprint)
- SHALL support parallel processing via import_parallelism parameter

**REQ-PERF-020:** Memory Usage
- SHOULD use approximately 50MB per concurrent fingerprint operation

**REQ-PERF-030:** API Rate Limiting
- MAY rely on MusicBrainz 1 req/s bottleneck (no explicit AcoustID rate limiter)
- NOTE: AcoustID allows 3 req/s but MusicBrainz is slower

**REQ-PERF-040:** Cache Effectiveness
- SHOULD reduce API calls by ~60% on re-import (per specification benchmark)

### Testing Requirements

**REQ-TEST-010:** Unit Tests
- SHALL test fingerprint hash function (determinism, length)
- SHALL test MBID extraction logic (best score, threshold filtering)
- SHALL test error handling paths

**REQ-TEST-020:** Integration Tests
- SHALL test fingerprinting with real audio file
- SHALL verify fingerprint format (non-empty, starts with "AQAD")

---

## Scope Definition

### IN SCOPE

**Core Implementation:**
- Fingerprinter struct with three-stage pipeline:
  1. decode_audio() - Symphonia integration
  2. resample_to_44100() - Rubato integration
  3. generate_fingerprint() - Chromaprint integration
- AcoustIDClient struct with:
  - lookup() - HTTP API call with caching
  - extract_best_mbid() - Score-based selection
  - get_cached_mbid() / cache_mbid() - Database caching
  - hash_fingerprint() - SHA-256 hashing
- Error types (FingerprintError, AcoustIDError)
- Unit tests (hash, MBID extraction)
- Integration tests (real file fingerprinting)

**Database:**
- acoustid_cache table schema definition (if not exists)
- Caching queries (SELECT, INSERT ON CONFLICT)

**Documentation:**
- Update IMPL012 specification issues (see Specification Issues section)
- Add build requirements documentation if missing

### OUT OF SCOPE

**Not Part of This Plan:**
- MusicBrainz client integration (separate IMPL011)
- Import workflow orchestration changes (already exists in workflow_orchestrator.rs)
- UI components for progress/status
- Build tooling automation (LLVM/Clang installation scripts)
- Rate limiter implementation (AcoustID 3/s not needed per spec)
- Test fixtures management (project decision needed)

**Explicitly Deferred:**
- Performance optimization beyond specification requirements
- Alternative fingerprinting algorithms (only Test2 specified)
- Fingerprint caching strategies beyond SHA-256 hash

---

## Dependencies Analysis

### Rust Crates (Already in Cargo.toml)

**Present and Ready:**
- `chromaprint-sys-next = "1.6"` (line 36) - Chromaprint C bindings
- `symphonia = { version = "0.5", features = ["all"] }` (line 41) - Audio decoding
- `rubato = "0.15"` (line 42) - Audio resampling
- `reqwest = { version = "0.11", features = ["json"] }` (line 30) - HTTP client
- `sha2 = "0.10"` (line 39) - SHA-256 hashing
- `serde`/`serde_json` (lines 18-19) - JSON parsing
- `sqlx` (line 26) - Database queries
- `base64 = "0.22"` (line 37) - Base64 encoding

**Missing:**
- None - all required crates already present

### System Dependencies

**Build Time:**
- LLVM/Clang (for chromaprint-sys-next bindgen)
  - Windows: Install from https://releases.llvm.org/
  - Linux: `llvm-dev libclang-dev clang`
  - macOS: `brew install llvm`

**Runtime:**
- Chromaprint C library (statically linked by chromaprint-sys-next)

### Database Schema

**Required Table:**
```sql
CREATE TABLE IF NOT EXISTS acoustid_cache (
    fingerprint_hash TEXT PRIMARY KEY,
    mbid TEXT NOT NULL,
    cached_at TEXT NOT NULL
);
```

**Status:** Schema not defined in specification - needs verification if table exists or requires migration.

---

## Specification Issues & Questions

### ISSUE 1: API Key Source Discrepancy

**Specification (line 615-621):** "Environment Variable: ACOUSTID_API_KEY"

**Current Implementation:** Loads from database settings table (per wkmp-ai/src/api/import_workflow.rs:209-228)

**Analysis:** Database-first approach is CORRECT per WKMP architecture (REQ-NF-030 through REQ-NF-037: zero-configuration startup). TOML and environment variables are bootstrap only.

**Resolution:** Update IMPL012 specification to reflect database-first configuration:
- Primary: Database settings table
- Fallback: Environment variable for testing/CI
- TOML sync: Backup only

**Priority:** Medium (documentation accuracy)

### ISSUE 2: Crate Name Ambiguity

**Specification (line 208):** `use chromaprint::{Context, Algorithm};`

**Cargo.toml (line 36):** `chromaprint-sys-next = "1.6"`

**Question:** Is there a high-level `chromaprint` wrapper crate, or do we use `chromaprint_sys_next` directly?

**Investigation Needed:** Check chromaprint-sys-next crate documentation for API surface.

**Resolution:** Verify correct import path and update specification example code.

**Priority:** High (implementation blocker)

### ISSUE 3: Database Schema Not Defined

**Specification:** References `acoustid_cache` table throughout (lines 402-430) but never defines schema.

**Current Project State:** No SQL migrations directory found (per Glob search).

**Question:** How are database schemas managed in WKMP? Create table in code? Migration file?

**Investigation Needed:** Check wkmp-common database initialization or other modules for pattern.

**Resolution:** Define schema and follow project conventions for table creation.

**Priority:** High (implementation blocker)

### ISSUE 4: Test Fixtures Not Present

**Specification (line 588-594):** Integration test references `fixtures/sample.mp3`

**Current Project State:** No test fixtures found.

**Question:** Should we:
- Create fixtures/ directory with sample audio?
- Use user's actual music files for testing?
- Skip integration tests for now?
- Use generated/synthetic audio?

**Resolution:** Decide test fixture strategy and update specification.

**Priority:** Medium (testing completeness)

### ISSUE 5: Rate Limiting Clarification

**Specification (line 606-609):** "No explicit rate limiter needed (MusicBrainz 1/s is bottleneck)"

**Context:** AcoustID allows 3 req/s, MusicBrainz allows 1 req/s.

**Question:** Is parallel processing configured such that MusicBrainz bottleneck naturally limits AcoustID calls?

**Risk:** If Chromaprint finishes faster than MusicBrainz lookup, could we buffer multiple AcoustID calls and burst past 3/s limit?

**Resolution:** Verify workflow orchestrator parallelism settings and confirm no rate limiter needed.

**Priority:** Low (specification already states no limiter needed, but worth verification)

---

## Current Implementation Analysis

### Files Requiring Changes

**wkmp-ai/src/services/fingerprinter.rs (lines 91-103, 61-89)**
- **Current State:** Placeholder implementations
  - `generate_chromaprint()` returns `vec![0x01, 0x02, 0x03, 0x04, 0xAB, 0xCD, 0xEF]`
  - `decode_audio()` returns dummy PCM data
- **Required Changes:** Full implementation of all three pipeline stages
- **Estimated LOC:** ~300 lines (decode: ~100, resample: ~50, fingerprint: ~50, helpers: ~100)

**wkmp-ai/src/services/acoustid_client.rs**
- **Current State:** Unknown - file may not exist yet
- **Required Changes:** Full AcoustIDClient implementation
- **Estimated LOC:** ~200 lines (client: ~150, caching: ~50)

**Database (schema)**
- **Current State:** acoustid_cache table may not exist
- **Required Changes:** CREATE TABLE IF NOT EXISTS
- **Location:** TBD based on project conventions

### Files NOT Requiring Changes

**wkmp-ai/src/services/workflow_orchestrator.rs**
- Already orchestrates FINGERPRINTING phase
- Already passes acoustid_api_key to fingerprinting logic
- No changes needed (integration point already exists)

**wkmp-ai/Cargo.toml**
- All dependencies already present
- No changes needed

---

## Risk Assessment

### Technical Risks

**RISK-001: FFI Complexity**
- **Description:** chromaprint-sys-next uses unsafe FFI bindings to C library
- **Probability:** Medium (FFI always has sharp edges)
- **Impact:** High (crashes, memory safety issues)
- **Mitigation:** Follow chromaprint-sys-next examples precisely, add extensive error handling
- **Residual Risk:** Low-Medium

**RISK-002: Audio Format Compatibility**
- **Description:** Symphonia may fail on certain audio formats/codecs
- **Probability:** Medium (wide format diversity in user libraries)
- **Impact:** Medium (some files unfingerprintable)
- **Mitigation:** Graceful error handling, log failures, continue processing
- **Residual Risk:** Low (acceptable - not all files need fingerprints)

**RISK-003: Resampling Quality**
- **Description:** Rubato resampling may alter audio characteristics affecting fingerprint accuracy
- **Probability:** Low (Rubato well-tested, Chromaprint expects resampling)
- **Impact:** Medium (reduced AcoustID match rates)
- **Mitigation:** Use specification-provided parameters (proven in Chromaprint ecosystem)
- **Residual Risk:** Very Low

**RISK-004: API Key Confusion**
- **Description:** User vs Application API key distinction (recent issue in conversation)
- **Probability:** Medium (user already encountered this)
- **Impact:** Medium (100% lookup failures)
- **Mitigation:** Clear UI labels, documentation, validate key format if possible
- **Residual Risk:** Low (addressed via documentation)

**RISK-005: Cache Collisions**
- **Description:** SHA-256 hash collisions causing incorrect MBID returns
- **Probability:** Very Low (SHA-256 collision probability negligible)
- **Impact:** Low (incorrect song identification)
- **Mitigation:** None needed (risk negligible)
- **Residual Risk:** Very Low

### Specification Risks

**RISK-006: Missing Schema Definition**
- **Description:** Specification doesn't define acoustid_cache table schema
- **Probability:** High (confirmed missing)
- **Impact:** Medium (implementation blocker)
- **Mitigation:** Phase 2 will address via specification verification
- **Residual Risk:** None (will be resolved before implementation)

**RISK-007: Test Strategy Unclear**
- **Description:** Integration tests reference non-existent fixtures
- **Probability:** High (confirmed missing)
- **Impact:** Low (affects test coverage, not functionality)
- **Mitigation:** Phase 3 will define test approach
- **Residual Risk:** None (will be resolved before implementation)

---

## Next Steps

**Phase 1 Status:** ✅ COMPLETE

**Phase 2 Actions:**
1. Investigate chromaprint-sys-next API surface (Context, Algorithm imports)
2. Verify database schema management approach (check wkmp-common patterns)
3. Review workflow orchestrator parallelism settings (rate limiting)
4. Check for existing acoustid_cache table definition
5. Generate specification completeness checklist
6. Flag CRITICAL issues requiring resolution before implementation

**Phase 3 Actions:**
1. Define acceptance tests for each requirement
2. Create traceability matrix (requirements → tests)
3. Decide test fixture strategy
4. Define integration test approach

**Phase 4 Actions:**
1. Generate implementation plan with increments
2. Define increment boundaries and test checkpoints
3. Create test-first workflow (test → implement → verify)

---

**Phase 1 Complete:** Ready for Phase 2 specification verification.
