# Dependencies Map: PLAN024 - wkmp-ai Refinement

**Plan:** PLAN024 - wkmp-ai Refinement Implementation
**Date:** 2025-11-12

---

## Existing WKMP Documents

### Documentation (Read-Only References)

| Document | Type | Lines | Status | Purpose |
|----------|------|-------|--------|---------|
| [docs/SPEC032-audio_ingest_architecture.md](../../docs/SPEC032-audio_ingest_architecture.md) | **Target (WILL UPDATE)** | ~800 | Exists | Audio ingest architecture specification |
| [docs/REQ001-requirements.md](../../docs/REQ001-requirements.md) | Reference | ~2000+ | Exists | System requirements (may need updates) |
| [docs/SPEC002-crossfade.md](../../docs/SPEC002-crossfade.md) | Reference | ~600 | Exists | Passage timing definitions (lead-in/lead-out) |
| [docs/SPEC017-sample_rate_conversion.md](../../docs/SPEC017-sample_rate_conversion.md) | Reference | ~400 | Exists | Tick time units conversion |
| [docs/IMPL001-database_schema.md](../../docs/IMPL001-database_schema.md) | Reference | ~1200 | Exists | Database schema (files, passages, songs tables) |
| [docs/SPEC031-data_driven_schema_maintenance.md](../../docs/SPEC031-data_driven_schema_maintenance.md) | Reference | ~500 | Exists | Zero-config database initialization |
| [docs/GOV001-document_hierarchy.md](../../docs/GOV001-document_hierarchy.md) | Governance | ~400 | Exists | Documentation framework |
| [docs/GOV002-requirements_enumeration.md](../../docs/GOV002-requirements_enumeration.md) | Governance | ~300 | Exists | Requirement numbering conventions |
| [wip/wkmp-ai_refinement.md](../../wip/wkmp-ai_refinement.md) | Source | ~104 | Exists | Original refinement notes (source of truth) |

**Reading Strategy:**
- Read summaries first (GOV documents, SPEC summaries)
- Reference specific sections by line number as needed
- Do NOT load full specs into context repeatedly

---

## Existing wkmp-ai Code

### Files to Modify

**Workflow & Orchestration:**
- `wkmp-ai/src/main.rs` - Entry point (add API key validation, folder selection steps)
- `wkmp-ai/src/services/workflow_orchestrator/mod.rs` - Workflow state machine (update to 10-phase pipeline)
- `wkmp-ai/src/services/workflow_orchestrator/phase_scanning.rs` - Add symlink/junction detection
- `wkmp-ai/src/services/workflow_orchestrator/phase_extraction.rs` - Update for metadata merge logic
- `wkmp-ai/src/services/workflow_orchestrator/phase_segmenting.rs` - Update for NO AUDIO detection
- `wkmp-ai/src/services/workflow_orchestrator/phase_fingerprinting.rs` - Per-passage fingerprinting
- `wkmp-ai/src/services/workflow_orchestrator/phase_analyzing.rs` - Update for zero-song passages
- `wkmp-ai/src/services/workflow_orchestrator/phase_flavoring.rs` - Update status marking

**Services:**
- `wkmp-ai/src/services/file_scanner.rs` - File discovery (add symlink/junction skip logic)
- `wkmp-ai/src/services/metadata_extractor.rs` - Metadata extraction (implement merge: new overwrites, old preserved)
- `wkmp-ai/src/services/silence_detector.rs` - Silence detection (read thresholds from settings, detect NO AUDIO)
- `wkmp-ai/src/services/fingerprinter.rs` - Chromaprint (per-passage fingerprinting)
- `wkmp-ai/src/services/confidence_assessor.rs` - Song matching (support zero-song passages, adjacent merging)
- `wkmp-ai/src/services/amplitude_analyzer.rs` - Amplitude (read thresholds from settings, leave fade-in/fade-out NULL)
- `wkmp-ai/src/services/acoustid_client.rs` - AcoustID API client (already exists, may need API key validation integration)
- `wkmp-ai/src/services/acousticbrainz_client.rs` - AcousticBrainz API client (already exists)
- `wkmp-ai/src/services/essentia_client.rs` - Essentia subprocess runner (already exists)

**Database:**
- `wkmp-ai/src/db/files.rs` - Files table queries (add status field, matching_hashes field)
- `wkmp-ai/src/db/passages.rs` - Passages table queries (add status field, ensure fade-in/fade-out NULL)
- `wkmp-ai/src/db/songs.rs` - Songs table queries (add status field)
- `wkmp-ai/src/db/settings.rs` - Settings table (may exist, ensure 7 parameters supported)
- `wkmp-ai/src/db/schema.rs` - Schema definitions (update for status fields, matching_hashes)

**API & UI:**
- `wkmp-ai/src/api/sse.rs` - SSE event emitter (add 13 event types)
- `wkmp-ai/src/api/ui/import_progress.rs` - UI progress page (add 13 sections, scrollable)
- `wkmp-ai/src/api/ui/root.rs` - Root UI (may need API key prompt integration)
- `wkmp-ai/src/api/import_workflow.rs` - Import workflow endpoints (add folder selection, API key validation)

**Models:**
- `wkmp-ai/src/models/import_session.rs` - Import session state (update for 10-phase pipeline)
- `wkmp-ai/src/models/parameters.rs` - Parameter definitions (add 7 settings if not already present)

### Files to Create

**New Services:**
- `wkmp-ai/src/services/api_key_validator.rs` - AcoustID API key validation logic
- `wkmp-ai/src/services/filename_matcher.rs` - Filename matching (3 outcomes: skip/reuse/new)
- `wkmp-ai/src/services/hash_deduplicator.rs` - Hash computation and duplicate detection
- `wkmp-ai/src/services/settings_manager.rs` - Settings table read/write with defaults

**New Models:**
- `wkmp-ai/src/models/progress_tracker.rs` - Progress statistics for 13 UI sections

**New Database:**
- `wkmp-ai/src/db/status_manager.rs` - Status field enumeration enforcement

**New API/UI:**
- `wkmp-ai/src/api/ui/folder_selector.rs` - Folder selection UI component
- `wkmp-ai/src/api/settings.rs` - Settings management API (if not exists)

### Files to Remove/Deprecate

**Quality Control (Out of Scope → Future wkmp-qa):**
- Any existing skip/gap/quality detection code (if present)
- Related UI components for quality assessment

**Manual Editing (Out of Scope → Future wkmp-pe):**
- User-directed fade point definition UI (if present)
- Manual MBID override UI (if present)

**Note:** Review codebase to identify these; may not exist yet.

---

## External Dependencies

### Rust Crates (Already in Cargo.toml)

**Confirmed Available:**
- `tokio` (1.x) - Async runtime
- `axum` (0.6+) - HTTP server + SSE
- `symphonia` (0.5+) - Audio decoding
- `lofty` (0.12+) - Metadata extraction (ID3, Vorbis, MP4)
- `serde` + `serde_json` - JSON serialization (for settings table, matching_hashes)
- `tracing` - Logging (DEBUG messages for API key validation)

**Likely Available (Verify):**
- `rusqlite` or `sqlx` - SQLite database access
- `chromaprint-rs` or FFI - Chromaprint fingerprinting
- `sha2` or `blake3` - File hashing
- `num_cpus` - CPU core count detection
- `uuid` - UUID generation (for fileId, passageId, songId)

**Add If Missing:**
- `num_cpus = "1.16"` - For thread count auto-initialization
- `sha2 = "0.10"` - For file content hashing (if not already present)

### External Tools

**Required at Runtime:**
- **AcoustID API** - Fingerprint matching service (requires valid API key)
- **AcousticBrainz API** - Musical flavor data (optional, Essentia fallback available)
- **Essentia** - Command-line tool for flavor analysis fallback (must be in PATH or specified location)

**Development Tools:**
- `cargo` - Rust build system
- `cargo-tarpaulin` or `cargo-llvm-cov` - Code coverage (for testing)
- `cargo clippy` - Linter

### Audio Codecs

**Supported via symphonia:**
- MP3, FLAC, AAC, Vorbis, Opus, WAV, AIFF, etc.
- No additional codec dependencies required

---

## Integration Points

### wkmp-ui Integration

**Launch Points (Existing, Not Modified):**
- wkmp-ui provides "Import Music" button
- Button opens http://localhost:5723 in new browser tab
- wkmp-ui checks wkmp-ai health via `/health` endpoint
- If wkmp-ai not running: Button shows "Install Full Version to enable import"

**No Changes Required to wkmp-ui:**
- wkmp-ai owns all import UX
- No embedded import UI in wkmp-ui

### Shared Database

**wkmp.db in Root Folder:**
- Location: `~/Music/wkmp.db` (or user-configured root folder)
- Access: SQLite file-based (no server)
- Concurrency: wkmp-ai writes during import, wkmp-ui reads for playback
- No concurrent imports expected (single-user workflow)

**Shared Tables (Read/Write by wkmp-ai):**
- `files` - Audio file metadata
- `passages` - Passage timing and metadata
- `songs`, `artists`, `works`, `albums` - MusicBrainz entities
- `passage_songs`, `passage_albums` - Relationships
- `acoustid_cache`, `musicbrainz_cache`, `acousticbrainz_cache` - API response caches
- `settings` - Shared configuration (all microservices read, wkmp-ai writes import params)

**Schema Updates Required:**
- `files` table: Add `matching_hashes` JSON field (if not exists)
- `files` table: Ensure `status` field exists (values: PENDING, PROCESSING, INGEST COMPLETE, DUPLICATE HASH, NO AUDIO)
- `passages` table: Ensure `status` field exists (values: PENDING, INGEST COMPLETE)
- `passages` table: Ensure `fade_in`, `fade_out` fields exist and nullable
- `songs` table: Ensure `status` field exists (values: PENDING, FLAVOR READY, FLAVORING FAILED)
- `settings` table: Ensure exists with (key TEXT PRIMARY KEY, value TEXT) structure

### Zero-Configuration Database (SPEC031)

**Automatic Schema Maintenance:**
- wkmp-ai MUST use `wkmp_common::db::schema_sync` for automatic table creation/updates
- Database created automatically if missing
- Schema drift detected and repaired automatically
- No manual migrations for column additions

**Implementation Requirement:**
- All schema changes via data-driven schema maintenance (SPEC031)
- No manual SQL migration scripts
- Code defines expected schema, runtime syncs actual schema

---

## Dependency Status Summary

| Dependency Type | Count | Status | Notes |
|----------------|-------|--------|-------|
| **WKMP Docs (Read-Only)** | 9 | ✅ All Exist | Reference as needed |
| **WKMP Docs (Update)** | 1 | ✅ Exists | SPEC032 target document |
| **Existing Code Files (Modify)** | ~25 | ✅ Likely Exist | Verify during implementation |
| **New Code Files (Create)** | ~7 | ⚠️ Must Create | New components |
| **Rust Crates (Confirmed)** | 6 | ✅ Available | Already in Cargo.toml |
| **Rust Crates (Verify)** | 5 | ⚠️ Verify | Check Cargo.toml |
| **Rust Crates (Add If Missing)** | 2 | ⚠️ Add If Needed | num_cpus, sha2 |
| **External APIs** | 2 | ⚠️ Runtime | Require network, API key |
| **External Tools** | 1 | ⚠️ Runtime | Essentia in PATH |
| **Database Tables (Shared)** | 10 | ✅ Exist | May need schema updates |
| **Integration Points** | 2 | ✅ Stable | wkmp-ui launch, shared database |

---

## Risk Assessment: Dependencies

### High Risk

❌ **None Identified**

All dependencies are existing or low-risk additions.

### Medium Risk

⚠️ **External APIs:**
- **AcoustID API:** Requires valid API key, subject to rate limits
  - Mitigation: API key validation at workflow start, skip fingerprinting if invalid
  - Fallback: Manual identification (out of scope, future wkmp-pe)

- **AcousticBrainz API:** May be unavailable or deprecated
  - Mitigation: Essentia fallback for flavor analysis
  - Fallback: Mark song as 'FLAVORING FAILED', continue processing

⚠️ **Essentia Tool:**
- Requires installation, must be in PATH or configured location
  - Mitigation: Document installation requirement
  - Fallback: Mark song as 'FLAVORING FAILED' if Essentia unavailable

### Low Risk

✅ **Rust Crates:**
- num_cpus: Widely used, stable
- sha2: Standard crypto library, stable
- All confirmed crates: Established in wkmp-ai

✅ **Database Schema Updates:**
- SPEC031 automatic schema maintenance handles additions
- Low risk of breaking existing data

✅ **Integration Points:**
- wkmp-ui integration: No changes required (low risk)
- Shared database: Already functional, no concurrency issues

---

## Dependency Resolution Plan

### Phase 1: Verification (Before Implementation)

1. **Verify Rust Crates:**
   - Check `wkmp-ai/Cargo.toml` for all required crates
   - Add missing crates (num_cpus, sha2 if needed)

2. **Verify External Tools:**
   - Check if Essentia installed and in PATH
   - Document installation if missing

3. **Verify Database Schema:**
   - Check IMPL001-database_schema.md for existing fields
   - Identify schema updates required (matching_hashes, status fields)

4. **Verify Code File Existence:**
   - List existing `wkmp-ai/src/**/*.rs` files
   - Confirm which files need modification vs. creation

### Phase 2: Preparation (Before Coding)

1. **Update Cargo.toml** (if needed)
2. **Document Essentia Installation** (if not documented)
3. **Plan Schema Updates** (via SPEC031 automatic maintenance)

### Phase 3: Implementation (During Coding)

1. **Modify existing files** incrementally
2. **Create new files** as needed
3. **Test dependencies** (crate functionality, API connectivity, Essentia execution)

---

## Dependency Traceability

**Requirement → External Dependency:**
- REQ-SPEC032-004, 011 → AcoustID API (fingerprinting)
- REQ-SPEC032-015 → AcousticBrainz API (flavor retrieval)
- REQ-SPEC032-015 → Essentia tool (flavor fallback)
- REQ-SPEC032-019 → num_cpus crate (CPU core count)
- REQ-SPEC032-008 → sha2 crate (file hashing)
- REQ-SPEC032-018 → serde_json (settings table JSON storage)

**All Other Requirements:**
- Depend on existing wkmp-ai infrastructure (tokio, axum, symphonia, SQLite)
