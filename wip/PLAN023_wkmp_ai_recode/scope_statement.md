# Scope Statement: PLAN023 - WKMP-AI Ground-Up Recode

**Plan:** PLAN023 - wkmp-ai Ground-Up Recode
**Created:** 2025-01-08
**Source:** wip/SPEC_wkmp_ai_recode.md

---

## ✅ In Scope

This plan WILL implement the following:

### Core Functionality

**1. Per-Song Import Workflow**
- Phase 0: Passage boundary detection (silence detection baseline)
- Phases 1-6: Sequential per-song processing through hybrid fusion pipeline
- Per-song error isolation (one failure doesn't abort entire import)
- Real-time SSE event broadcasting at each stage

**2. 3-Tier Hybrid Fusion Architecture**

**Tier 1: Source Extractors (Parallel)**
- ID3 Metadata Extractor
- Chromaprint Fingerprint Analyzer
- AcoustID API Client
- MusicBrainz API Client
- Essentia Musical Flavor Analyzer
- Audio-Derived Feature Extractor
- ID3 Genre Mapping

**Tier 2: Fusion Engine**
- Identity Resolution Module (Bayesian update with conflict detection)
- Metadata Fusion Module (field-wise weighted selection)
- Musical Flavor Synthesis Module (characteristic-wise weighted averaging)
- Passage Boundary Fusion Module (baseline: silence detection only)

**Tier 3: Quality Validation**
- Title Consistency Check (Levenshtein fuzzy matching)
- Duration Consistency Check
- Genre-Flavor Alignment Check
- Overall Quality Score computation
- Conflict detection and flagging

**3. Database Schema Extensions**
- Extend `passages` table with 13 new provenance tracking fields:
  - Flavor source provenance (blend, confidence map)
  - Metadata source provenance (title/artist source and confidence)
  - Identity resolution tracking (MBID, confidence, conflicts)
  - Quality scores (overall, metadata completeness, flavor completeness)
  - Validation flags (status, report JSON)
  - Import metadata (session ID, timestamp, strategy)
- Create new `import_provenance` table for detailed source logging

**4. Real-Time SSE Event System**
- 10 event types covering all workflow stages
- SSE protocol implementation
- Event throttling (max 30/sec with buffering)
- Per-song and file-level events

**5. Complete Ground-Up Recode**
- New module structure in wkmp-ai:
  - `import/` (orchestrator, per_song_engine)
  - `fusion/` (extractors/, fusers/, validators/)
  - `events/` (import_events, sse_broadcaster)
  - `db/` (passage_repository, provenance_logger)
- Clean implementation, NO legacy code copying
- Existing wkmp-ai kept for reference only

---

## ❌ Out of Scope

This plan will NOT implement the following:

### Explicitly Excluded

**1. Multi-Strategy Passage Boundary Fusion**
- Beat tracking for passage boundaries (REQ-AI-052: P2 priority)
- Structural analysis (intro/verse/chorus/outro detection)
- Metadata hints (cue sheets, ID3 track start times)
- **Rationale:** Future enhancement, requires additional dependencies not currently available

**2. Parallel Song Processing**
- Concurrent processing of multiple passages (REQ-AI-NF-042: P2 priority)
- Worker pool with semaphores
- Out-of-order completion handling
- **Rationale:** Sequential implementation is baseline; parallel is future optimization

**3. User Feedback Learning**
- Manual review workflow for conflicts (REQ-AI-NF-042: P2 priority)
- User correction storage and learning
- Feedback-driven fusion improvements
- **Rationale:** Advanced feature requiring UI development beyond current scope

**4. Existing wkmp-ai UI**
- Legacy import wizard UI
- File segmentation UI
- Existing SSE implementation
- **Rationale:** Ground-up recode creates new implementation; legacy UI not modified

**5. Performance Optimizations**
- Aggressive caching strategies
- Pre-computed fingerprint databases
- Batch API request optimization
- **Rationale:** Focus on correctness first; optimization is future work

**6. Advanced Error Recovery**
- Automatic retry with exponential backoff
- Partial passage recovery
- Database transaction rollback on import failure
- **Rationale:** Basic error isolation sufficient for initial implementation

**7. Import History and Replay**
- Import session history tracking
- Re-import with different settings
- Rollback to previous import state
- **Rationale:** Advanced feature not required for core functionality

**8. Migration from Legacy Import**
- Data migration from old passages to new provenance schema
- Backward compatibility with legacy import events
- **Rationale:** Ground-up recode is clean slate; no legacy migration required

---

## Assumptions

The following are assumed to be true:

**A1: Environment**
- Rust stable toolchain available (1.70+)
- Tokio async runtime available
- SQLite with JSON1 extension enabled

**A2: Dependencies**
- Essentia library MAY be installed (optional, graceful degradation if missing)
- AcoustID API accessible (internet connection required)
- MusicBrainz API accessible (internet connection required)
- AcousticBrainz API accessible (may return 404 for post-2022 recordings)

**A3: Existing WKMP Infrastructure**
- `wkmp-common` library provides database pool, event bus utilities
- `passages` table exists in database (will be extended, not recreated)
- Existing WKMP modules (wkmp-ap, wkmp-ui) continue operating independently

**A4: Musical Flavor Characteristics**
- SPEC003-musical_flavor.md defines expected characteristics
- Characteristics must sum to 1.0 (normalization required)
- Expected characteristics count is defined (for completeness scoring)

**A5: Audio File Formats**
- Supported formats: MP3, FLAC, OGG, M4A, WAV (per existing WKMP specs)
- Audio files are valid and decodable (symphonia can read)

**A6: Database Write Access**
- wkmp-ai has write access to wkmp.db database
- Database schema migrations can be applied (ALTER TABLE permissions)

**A7: Network Reliability**
- Network API calls (AcoustID, MusicBrainz, AcousticBrainz) may fail
- Graceful degradation handles API failures (partial data acceptable)

**A8: User Authorization**
- This is a ground-up recode approved by user
- Specification document (SPEC_wkmp_ai_recode.md) is authoritative

---

## Constraints

The following limitations apply:

### Technical Constraints

**T1: Programming Language**
- Rust (stable channel) - Non-negotiable per WKMP architecture

**T2: Async Runtime**
- Tokio - Established WKMP standard

**T3: Database**
- SQLite with JSON1 extension - Cannot use other databases

**T4: HTTP Framework**
- Axum - Established WKMP microservice standard

**T5: Audio Decoding**
- symphonia library - Existing WKMP audio stack

**T6: Fingerprinting**
- Chromaprint algorithm - Industry standard for AcoustID

**T7: Sequential Processing (Baseline)**
- One passage at a time initially (parallel is future optimization)

**T8: Existing Database Schema**
- Must extend `passages` table, not recreate (preserve existing data)
- Must maintain foreign key relationships

### Process Constraints

**P1: No Legacy Code Copying**
- Reference existing wkmp-ai for API contracts only
- All code must be rewritten from scratch per specification

**P2: Test-First Approach**
- All acceptance tests defined before implementation
- >90% code coverage required (REQ-AI-NF-032)

**P3: Specification Authority**
- wip/SPEC_wkmp_ai_recode.md is authoritative
- Recent analysis documents (hybrid_import_analysis.md, per_song_import_analysis/) take precedence over older docs in case of conflict

**P4: Modular Architecture**
- Clear separation of Tier 1 / Tier 2 / Tier 3
- Independent extractor modules (trait-based abstraction)

### Quality Constraints

**Q1: Normalization Precision**
- Musical flavor characteristics must sum to 1.0 ± 0.0001
- Validation failure if normalization violated

**Q2: Test Coverage**
- Minimum 90% code coverage
- All fusion algorithms unit tested
- End-to-end integration tests required

**Q3: Error Handling**
- No `.unwrap()` on user input or I/O operations
- All errors logged with context (passage_id, phase, source)

**Q4: Documentation**
- Module-level rustdoc comments required
- Public functions documented
- Fusion algorithms documented with formulas

### Schedule Constraints

**S1: Realistic Estimates**
- Per-song processing target: ≤ 2 minutes average (REQ-AI-NF-011)
- 10-song album baseline: ≤ 20 minutes (sequential)

**S2: No External Blockers**
- Cannot wait for beat tracking library development
- Cannot wait for structural analysis implementation
- Must use available tools only (Chromaprint, Essentia, APIs)

### Resource Constraints

**R1: API Rate Limits**
- AcoustID API: Rate limiting possible (throttle requests)
- MusicBrainz API: Rate limiting enforced (1 request/sec recommended)
- Must respect API limits to avoid bans

**R2: Memory Constraints**
- Audio decoding in-memory (passages loaded individually)
- Cannot hold entire multi-song file in memory simultaneously

**R3: Disk I/O**
- SQLite database may be on slower storage
- Database writes are bottleneck (batch inserts when possible)

---

## Boundaries and Interfaces

### System Boundaries

**Inputs:**
- Audio files (paths provided by user via POST /import/start)
- User configuration (root folder, import settings)

**Outputs:**
- Database passage records (extended schema)
- Import provenance log entries
- SSE events (real-time progress)
- Completion summary (success/warning/failure counts)

### External Interfaces

**API Integrations:**
- AcoustID API (fingerprint → Recording MBID)
- MusicBrainz API (MBID → metadata)
- AcousticBrainz API (MBID → musical flavor, pre-2022 only)

**Optional Dependencies:**
- Essentia library (musical flavor computation, if installed)

**Database Interface:**
- SQLite `passages` table (extended with 13 new fields)
- New `import_provenance` table

**Event Interface:**
- SSE endpoint (GET /import/events)
- 10 event types (file-level + per-song)

### Internal Module Interfaces

**Tier 1 Extractors:**
- All implement `FlavorExtractor` or `MetadataExtractor` trait
- Return `Result<ExtractedData, Error>` with confidence scores

**Tier 2 Fusers:**
- Input: Multiple `ExtractedData` sources
- Output: Fused result with provenance tracking

**Tier 3 Validators:**
- Input: Fused data (metadata, flavor, identity)
- Output: ValidationReport with quality scores and conflicts

---

## Success Criteria

This implementation is successful when:

**Functional Success:**
1. ✅ Per-song import workflow processes multi-song files correctly
2. ✅ Bayesian identity resolution produces higher confidence than single-source
3. ✅ Metadata fusion preserves high-quality data (no information loss)
4. ✅ Musical flavor synthesis handles AcousticBrainz obsolescence (multi-source blending)
5. ✅ Real-time SSE events show per-song progress (not just file-level)
6. ✅ Quality validation detects conflicts and flags low-quality passages
7. ✅ Database records include source provenance for all fields
8. ✅ Error isolation prevents single-song failure from aborting import

**Non-Functional Success:**
9. ✅ Test coverage >90% for all modules
10. ✅ All 46 requirements have passing acceptance tests
11. ✅ Modular architecture with clear Tier 1/2/3 separation
12. ✅ Code is clean of legacy dependencies (ground-up recode verified)
13. ✅ Performance baseline: ≤ 2 min/song average
14. ✅ Graceful degradation when Essentia not installed or APIs fail

---

## Scope Changes

**Change Control:**
- Scope changes require user approval
- Document all changes in this section
- Update requirements index and test specifications

**Current Status:** No scope changes yet (baseline scope from specification)

---

**Document Version:** 1.0
**Last Updated:** 2025-01-08
