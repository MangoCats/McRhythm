# Scope Statement: PLAN024 - wkmp-ai Refinement

**Plan:** PLAN024 - wkmp-ai Refinement Implementation
**Date:** 2025-11-12
**Source:** [wip/SPEC032_wkmp-ai_refinement_specification.md](../../wip/SPEC032_wkmp-ai_refinement_specification.md)

---

## ✅ In Scope

### Documentation Updates

**SPEC032-audio_ingest_architecture.md Updates:**
1. Add Scope Definition section (in-scope: automatic ingest; out-of-scope: quality control, manual editing)
2. Add Two-Stage Roadmap section (Stage One: root folder; Stage Two: external folders)
3. Add Five-Step Workflow section (API key validation → folder selection → scanning → processing → completion)
4. Add Ten-Phase Per-File Pipeline section (detailed enumeration of FILENAME MATCHING through PASSAGES COMPLETE)
5. Add Duplicate Detection Strategy section (filename matching + hash matching with bidirectional linking)
6. Add Settings Management section (database settings table with 7 parameters)
7. Add UI Progress Display Specification section (13 SSE-driven sections)
8. Add Status Field Enumeration section (files/passages/songs status values and transitions)
9. Update Overview section (remove quality control and manual editing from purpose)
10. Update Component Architecture section (10-phase pipeline, remove quality components)
11. Update Import Workflow State Machine section (add API key validation and folder selection steps)
12. Update Database Integration section (settings table operations, status fields, matching_hashes)
13. Remove quality control features (skip/gap/quality detection)
14. Remove manual passage editing features (user-directed fade points, manual MBID revision)

### Code Implementation

**Stage One Features (All In Scope):**

1. **AcoustID API Key Validation** (REQ-SPEC032-004)
   - Validate stored key at workflow start
   - Prompt user for valid key or acknowledge lack
   - Remember choice for session, re-prompt next session if still invalid
   - DEBUG logging for validation process

2. **Folder Selection** (REQ-SPEC032-005)
   - UI to select folder to scan
   - Default: root folder
   - Stage One constraint: Only root folder or subfolders allowed
   - Error message for external folders

3. **Ten-Phase Per-File Pipeline** (REQ-SPEC032-007-015, 021)

   **Phase 1: Filename Matching** (REQ-SPEC032-007)
   - Query files table by path/filename/metadata
   - Three outcomes: skip completed, reuse existing fileId, assign new fileId

   **Phase 2: Hashing** (REQ-SPEC032-008)
   - Compute file content hash
   - Query for matching hashes
   - Bidirectional matching_hashes linking
   - Mark 'DUPLICATE HASH' if completed match found

   **Phase 3: Extracting** (REQ-SPEC032-009)
   - ID3/metadata extraction
   - Merge with existing metadata (new overwrites, old preserved)

   **Phase 4: Segmenting** (REQ-SPEC032-010)
   - Silence-based boundary detection
   - Read thresholds from settings table
   - Detect 'NO AUDIO' files (<100ms non-silence)

   **Phase 5: Fingerprinting** (REQ-SPEC032-011)
   - Chromaprint analysis per potential passage
   - Submit to AcoustID API
   - Skip if API key invalid and acknowledged

   **Phase 6: Song Matching** (REQ-SPEC032-012)
   - Combine metadata + fingerprint evidence
   - Confidence scoring (High/Medium/Low/None)
   - Support zero-song passages (no-confidence)
   - Support adjacent passage merging (embedded silence)

   **Phase 7: Recording** (REQ-SPEC032-013)
   - Write finalized passages to database
   - Create/update song entries
   - Link passages to songs

   **Phase 8: Amplitude** (REQ-SPEC032-014)
   - Detect lead-in/lead-out points
   - Read thresholds from settings table
   - Leave fade-in/fade-out NULL (wkmp-pe responsibility)
   - Mark passage 'INGEST COMPLETE'

   **Phase 9: Flavoring** (REQ-SPEC032-015)
   - AcousticBrainz high-level profile retrieval
   - Essentia fallback if AcousticBrainz unavailable
   - Mark song status 'FLAVOR READY' or 'FLAVORING FAILED'

   **Phase 10: Passages Complete** (REQ-SPEC032-016)
   - Mark file 'INGEST COMPLETE' when all passages + songs complete

4. **File & Session Completion** (REQ-SPEC032-016, 017)
   - Track file completion status
   - Determine session completion (all files dispositioned)

5. **Database Settings Table** (REQ-SPEC032-018)
   - Store/read 7 import parameters:
     - `silence_threshold_dB` (default: 35.0)
     - `silence_min_duration_ticks` (default: 300ms in ticks)
     - `minimum_passage_audio_duration_ticks` (default: 100ms in ticks)
     - `lead-in_threshold_dB` (default: 45.0)
     - `lead-out_threshold_dB` (default: 40.0)
     - `acoustid_api_key` (string)
     - `ai_processing_thread_count` (NULL, auto-initialized)

6. **Thread Count Auto-Initialization** (REQ-SPEC032-019)
   - Read ai_processing_thread_count from settings
   - If NULL: Compute CPU_core_count + 1, store to database
   - If defined: Use stored value (user tuning supported)

7. **Status Field Enumerations** (REQ-SPEC032-021)
   - files.status: PENDING, PROCESSING, INGEST COMPLETE, DUPLICATE HASH, NO AUDIO
   - passages.status: PENDING, INGEST COMPLETE
   - songs.status: PENDING, FLAVOR READY, FLAVORING FAILED

8. **Thirteen UI Progress Sections** (REQ-SPEC032-020, NF-002)
   - SSE-driven real-time updates
   - 13 sections: SCANNING, PROCESSING, FILENAME MATCHING, HASHING, EXTRACTING, SEGMENTING, FINGERPRINTING, SONG MATCHING, RECORDING, AMPLITUDE, FLAVORING, PASSAGES COMPLETE, FILES COMPLETE
   - Scrollable sections (RECORDING, AMPLITUDE)
   - Per-phase statistics

9. **Parallel Processing** (REQ-SPEC032-NF-001)
   - Multiple files processed concurrently
   - Thread count from ai_processing_thread_count setting

10. **Sample-Accurate Timing** (REQ-SPEC032-NF-003)
    - All timing points sample-accurate
    - Convert to SPEC017 ticks for storage

11. **Symlink/Junction Handling** (REQ-SPEC032-NF-004)
    - Do NOT follow symlinks, junctions, shortcuts during scanning

12. **Metadata Preservation** (REQ-SPEC032-NF-005)
    - Preserve all extracted metadata (even unused)

---

## ❌ Out of Scope

### Explicitly Excluded (Future Microservices)

**Quality Control Features → wkmp-qa (future):**
- Detecting skips, gaps, audio quality issues
- Audio quality assessment algorithms
- Quality scoring and reporting
- Related UI components

**Manual Passage Editing → wkmp-pe (future):**
- User-directed fade-in/fade-out point definition
- Manual MBID revision and override
- Passage boundary adjustment UI
- Metadata manual correction UI

**Stage Two Features (Future Enhancement):**
- External folder scanning (outside root folder)
- File movement/copying to root folder after identification
- Multi-location library management

**Segment Editor (Existing Feature):**
- waveform editor for passage boundaries (existing in SPEC032, not being modified)
- Manual segmentation UI (existing, out of scope for this refinement)

### Not Changed (Existing Features Preserved)

**Existing wkmp-ai Features NOT Modified:**
- Port assignment (5723)
- Zero-configuration database initialization (SPEC031)
- HTTP/SSE server infrastructure
- Waveform visualization (if existing)
- Integration with wkmp-ui launch points

**Existing Database Schema Preserved:**
- Core tables: files, passages, songs, artists, works, albums
- Relationship tables: passage_songs, passage_albums
- Cache tables: acoustid_cache, musicbrainz_cache, acousticbrainz_cache

---

## Assumptions

**Technical Assumptions:**
1. SQLite database supports JSON1 extension (for matching_hashes array storage)
2. CPU core count detectable at runtime (for thread count auto-initialization)
3. AcoustID API available and accessible (with valid API key)
4. AcousticBrainz API available (with Essentia fallback if unavailable)
5. Chromaprint library available and functional
6. Essentia command-line tool available for fallback flavor analysis
7. SPEC017 tick conversion functions available in wkmp_common
8. SSE infrastructure functional in wkmp-ai

**Workflow Assumptions:**
1. User has valid AcoustID API key OR is willing to acknowledge lack
2. Root folder exists and is writable
3. Audio files are in supported formats (determined by symphonia decoder)
4. File metadata (created/modified timestamps) is stable and accurate
5. Users understand Stage One constraint (root folder only)

**Data Assumptions:**
1. MusicBrainz database has recordings for most common music
2. AcousticBrainz has flavor data for popular recordings
3. File hashes uniquely identify file content (SHA-256 or similar)
4. Filename/path matching is sufficient for re-import detection

**Architectural Assumptions:**
1. wkmp-ai owns import workflow exclusively (no concurrent importers)
2. Database writes are atomic and consistent
3. SSE connections support real-time updates without excessive latency
4. UI can handle 13 concurrent SSE data streams

---

## Constraints

**Technical Constraints:**
1. **Platform:** Rust stable channel (existing wkmp-ai constraint)
2. **Async Runtime:** Tokio (existing wkmp-ai constraint)
3. **Web Framework:** Axum for HTTP + SSE (existing wkmp-ai constraint)
4. **Audio Stack:** symphonia (decode), Chromaprint (fingerprinting)
5. **Database:** SQLite with JSON1 extension (existing WKMP constraint)
6. **Sample Rate:** 48kHz internal processing (WKMP standard)
7. **Timing Units:** SPEC017 ticks for all time values

**Performance Constraints:**
1. Import speed: Limited by AcoustID API rate limits
2. Parallel processing: Limited by CPU core count
3. Memory usage: Must handle large libraries (100k+ files)
4. Disk I/O: Read-intensive during scanning/processing

**Process Constraints:**
1. **Documentation First:** SPEC032 updates MUST be completed before code changes
2. **Test-First:** Acceptance tests defined before implementation
3. **Traceability:** All code changes must trace to requirements
4. **No Shortcuts:** Complete implementation required (no partial features)

**Architectural Constraints:**
1. **Microservices Isolation:** wkmp-ai does NOT implement quality control or manual editing
2. **Database Schema Stability:** Changes must be backward-compatible
3. **API Compatibility:** HTTP/SSE interfaces must remain compatible with wkmp-ui
4. **Zero-Configuration:** Database initialization automatic (SPEC031)

**Timeline Constraints:**
1. **Estimated Effort:** 15-25 hours total
2. **Phased Delivery:** SPEC032 updates (4-6h) → Code refactoring (8-12h) → UI (3-5h) → Testing (2-4h)
3. **No Hard Deadline:** Quality over speed

---

## Dependencies

### Existing WKMP Documents (Read-Only)

**Architecture & Requirements:**
- [SPEC032-audio_ingest_architecture.md](../../docs/SPEC032-audio_ingest_architecture.md) - Target document to be UPDATED
- [REQ001-requirements.md](../../docs/REQ001-requirements.md) - May require requirement updates (identified during implementation)
- [SPEC002-crossfade.md](../../docs/SPEC002-crossfade.md) - Passage timing definitions (lead-in/lead-out)
- [SPEC017-sample_rate_conversion.md](../../docs/SPEC017-sample_rate_conversion.md) - Tick time units
- [IMPL001-database_schema.md](../../docs/IMPL001-database_schema.md) - Database schema (files, passages, songs tables)
- [SPEC031-data_driven_schema_maintenance.md](../../docs/SPEC031-data_driven_schema_maintenance.md) - Zero-config database initialization

**Governance:**
- [GOV001-document_hierarchy.md](../../docs/GOV001-document_hierarchy.md) - Documentation governance
- [GOV002-requirements_enumeration.md](../../docs/GOV002-requirements_enumeration.md) - Requirement numbering conventions

**Source Refinement:**
- [wip/wkmp-ai_refinement.md](../../wip/wkmp-ai_refinement.md) - Original refinement notes (source of truth)

### Existing wkmp-ai Code (To Be Modified)

**Existing Components (Modify):**
- `wkmp-ai/src/main.rs` - Entry point, may need workflow updates
- `wkmp-ai/src/services/workflow_orchestrator/` - Workflow state machine (update to 10-phase pipeline)
- `wkmp-ai/src/services/file_scanner.rs` - File discovery (add symlink handling)
- `wkmp-ai/src/services/metadata_extractor.rs` - Metadata extraction (add merge logic)
- `wkmp-ai/src/services/silence_detector.rs` - Silence detection (add NO AUDIO handling)
- `wkmp-ai/src/services/fingerprinter.rs` - Chromaprint fingerprinting (per-passage)
- `wkmp-ai/src/services/confidence_assessor.rs` - Song matching (add zero-song support)
- `wkmp-ai/src/services/amplitude_analyzer.rs` - Amplitude analysis (lead-in/lead-out, leave fade NULL)
- `wkmp-ai/src/api/sse.rs` - SSE event emitter (13 event types)
- `wkmp-ai/src/api/ui/import_progress.rs` - UI progress page (13 sections)
- `wkmp-ai/src/db/` - Database queries (settings table, status fields, matching_hashes)

**New Components (Create):**
- `wkmp-ai/src/services/api_key_validator.rs` - AcoustID API key validation
- `wkmp-ai/src/services/filename_matcher.rs` - Filename matching logic
- `wkmp-ai/src/services/hash_deduplicator.rs` - Hash-based deduplication
- `wkmp-ai/src/services/settings_manager.rs` - Database settings table management
- `wkmp-ai/src/models/progress_tracker.rs` - Progress statistics for 13 UI sections
- `wkmp-ai/src/db/status_manager.rs` - Status field enumeration enforcement
- `wkmp-ai/src/api/ui/folder_selector.rs` - Folder selection UI

### External Libraries

**Required (Already Available):**
- `tokio` - Async runtime
- `axum` - HTTP + SSE server
- `symphonia` - Audio decoding
- `lofty` - ID3 metadata extraction
- `sqlx` or `rusqlite` - SQLite access
- `serde_json` - JSON handling (for settings table)
- `chromaprint-rs` or FFI bindings - Audio fingerprinting

**May Be Required:**
- `num_cpus` - CPU core count detection (for thread count auto-initialization)
- `sha2` or `blake3` - File content hashing
- Process spawning for Essentia - Already available via `std::process::Command`

### Integration Points (wkmp-ui)

**wkmp-ui Integration (Existing, Not Modified):**
- Launch point: "Import Music" button opens http://localhost:5723
- Health check: wkmp-ui checks wkmp-ai `/health` endpoint
- No embedded import UI in wkmp-ui (wkmp-ai owns all import UX)

**Database (Shared):**
- wkmp.db in root folder
- Shared tables: files, passages, songs, artists, works, albums
- Settings table: Shared across microservices
- No concurrent writes expected (wkmp-ai owns import, wkmp-ui reads)

---

## Success Criteria

**Documentation Success:**
1. ✅ SPEC032 includes all 8 new sections
2. ✅ SPEC032 updates all 4 existing sections
3. ✅ All out-of-scope features moved to future microservice references
4. ✅ Document passes markdown linting
5. ✅ All internal references valid

**Implementation Success:**
1. ✅ All 21 functional requirements implemented
2. ✅ All 5 non-functional requirements met
3. ✅ All acceptance tests pass (100% coverage)
4. ✅ Traceability matrix complete (requirement → test → code)
5. ✅ No regression in existing wkmp-ai functionality
6. ✅ Zero-configuration startup still functional

**Quality Success:**
1. ✅ Code coverage ≥80% for new code
2. ✅ All compiler warnings resolved
3. ✅ cargo clippy passes with no warnings
4. ✅ Integration tests pass (wkmp-ai ↔ database)
5. ✅ System tests pass (end-to-end import workflow)

**User Experience Success:**
1. ✅ API key validation UX clear and helpful
2. ✅ Folder selection prevents external folders (Stage One)
3. ✅ UI progress sections update smoothly in real-time
4. ✅ Import completes successfully for typical music library (100-1000 files)
5. ✅ Duplicate detection works (same file, renamed file, reorganized file)

---

## Scope Control

**Change Control Process:**
1. Any scope addition requires explicit user approval
2. Deferred requirements documented with user approval
3. Scope reduction documented with impact analysis
4. All changes tracked in plan completion report

**Scope Creep Prevention:**
1. No feature additions beyond 26 enumerated requirements
2. No "while we're at it" improvements
3. Quality control features → Defer to wkmp-qa
4. Manual editing features → Defer to wkmp-pe
5. Stage Two features → Defer to future release

**Scope Validation:**
- Checkpoint after SPEC032 update: Review scope alignment
- Checkpoint after each increment: Verify no scope expansion
- Final review: Confirm all in-scope requirements met, all out-of-scope items deferred
