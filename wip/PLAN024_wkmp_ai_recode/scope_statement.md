# Scope Statement: WKMP-AI Audio Import System Recode

**Plan:** PLAN024
**Source:** wip/SPEC_wkmp_ai_recode.md
**Created:** 2025-11-09

---

## ✅ In Scope

### Core Functionality

**1. Per-Song Import Workflow**
- Phase 0: Passage boundary detection (silence-based, entire file)
- Phases 1-6: Sequential per-song processing through hybrid fusion pipeline
- Per-song error isolation (one failure doesn't abort entire import)
- Real-time SSE events at each processing stage

**2. 3-Tier Hybrid Fusion Engine**

**Tier 1: Parallel Source Extractors**
- ID3 Metadata Extractor
- Chromaprint Fingerprint Analyzer
- AcoustID API Client (with rate limiting)
- MusicBrainz API Client (with rate limiting)
- Essentia Musical Flavor Analyzer
- Audio-Derived Feature Extractor
- ID3 Genre Mapper

**Tier 2: Confidence-Weighted Fusion Modules**
- Identity Resolver (Bayesian MBID fusion with conflict detection)
- Metadata Fuser (field-wise weighted selection)
- Musical Flavor Synthesizer (characteristic-wise weighted averaging)
- Passage Boundary Fuser (multi-strategy validation)

**Tier 3: Quality Validation & Enrichment**
- Cross-source consistency checks (title, duration, genre-flavor)
- Completeness scoring (metadata and flavor)
- Conflict detection and flagging
- Overall quality score computation

**3. Database Schema Extensions**
- 17 new columns in passages table (flavor/metadata provenance, quality scores, validation)
- New import_provenance table (source tracking per passage)
- SPEC017 tick-based timing compliance (28,224,000 Hz tick rate)
- SPEC031 zero-configuration schema maintenance

**4. Real-Time SSE Event Streaming**
- 10 per-song event types (PassagesDiscovered through SongCompleted)
- File-level AND song-level progress tracking
- Event throttling (max 30 events/second)
- Buffering strategy for high-rate scenarios

**5. UI Progress Reporting**
- Real-time progress updates (percentage complete)
- Estimated time remaining (ETA)
- File-by-file workflow clarity
- Multi-phase progress visualization
- Per-song granularity feedback
- Error and warning visibility

**6. Ground-Up Recode**
- NO legacy code copying until functions can be replaced
- Clean implementation following new architectural patterns
- Reference existing code only for API contracts, database schema
- Use existing wkmp-common utilities (database pool, event bus)

---

## ❌ Out of Scope

### Explicitly Excluded from Current Implementation

**1. Multi-Strategy Boundary Detection (Future)**
- Beat tracking-based boundary detection
- Structural analysis-based boundary detection
- Advanced fusion of multiple boundary detection strategies
- **Current implementation:** Silence detection only (baseline)
- **Rationale:** REQ-AI-052 marked as MEDIUM priority, SHOULD not SHALL

**2. Advanced Performance Optimizations (Future)**
- Caching of AcoustID/MusicBrainz API results
- Batch processing optimizations
- GPU-accelerated audio analysis
- **Current implementation:** Sequential per-song processing is acceptable
- **Rationale:** REQ-AI-NF-042 marked as LOW priority

**3. Existing wkmp-ai Features NOT Part of Recode**
- File scanning UI (on-demand microservice features)
- Lyric synchronization
- Manual passage editing
- Import wizard UI pages
- **Rationale:** This specification covers import engine only, not UI components

**4. Database Migration Tooling**
- Manual migration scripts for schema changes
- **Current implementation:** SPEC031 zero-conf handles schema evolution automatically
- **Rationale:** SPEC031 makes manual migrations obsolete for column additions

**5. Backward Compatibility with Pre-Recode Data**
- Migration of existing passages to new schema
- Import of legacy import logs
- **Rationale:** Ground-up recode, fresh schema starting point

---

## Assumptions

### Technical Assumptions

**1. External Services**
- **ASSUME:** AcoustID API is available and responsive
- **ASSUME:** MusicBrainz API is available and responsive
- **MITIGATION:** Graceful degradation per REQ-AI-NF-022 (proceed with partial data)

**2. Audio Format Support**
- **ASSUME:** symphonia library supports all common audio formats (MP3, FLAC, WAV, OGG, M4A)
- **ASSUME:** Files are not corrupted or malformed
- **MITIGATION:** Error isolation per REQ-AI-NF-021 (skip problematic passages)

**3. Essentia Availability**
- **ASSUME:** Essentia library is optionally available on deployment system
- **FALLBACK:** AudioDerived extractor provides degraded service if Essentia unavailable
- **IMPACT:** Lower musical flavor confidence (0.6 vs 0.9)

**4. Database Consistency**
- **ASSUME:** SQLite database is writable
- **ASSUME:** SPEC031 zero-conf schema maintenance is already implemented in wkmp-common
- **VERIFICATION:** Check wkmp-common for SchemaSync availability

**5. Tick-Based Timing**
- **ASSUME:** SPEC017 tick conversion utilities exist in wkmp-common
- **ASSUME:** Tick rate is 28,224,000 Hz per SPEC017
- **VERIFICATION:** Check wkmp-common for tick conversion functions

### Process Assumptions

**6. Specification Completeness**
- **ASSUME:** SPEC_wkmp_ai_recode.md is the authoritative specification
- **ASSUME:** Recent analysis documents (hybrid_import_analysis.md, per_song_import_analysis/) take precedence over older documents in case of conflict

**7. Requirements Traceability**
- **ASSUME:** All 72 requirements are testable
- **ASSUME:** Requirement IDs follow GOV002 numbering scheme
- **VERIFICATION:** Phase 2 will verify specification completeness

**8. Implementation Approach**
- **ASSUME:** Ground-up recode is approved (no incremental migration)
- **ASSUME:** Complete rewrite is phased over 12-14 weeks (per SPEC030 complete rewrite guidance)
- **VERIFICATION:** User approval obtained before implementation begins

---

## Constraints

### Technical Constraints

**1. Technology Stack (FIXED)**
- **Language:** Rust (stable channel)
- **Async Runtime:** tokio
- **HTTP Framework:** axum
- **Audio Decoding:** symphonia
- **Database:** SQLite with JSON1 extension
- **Musical Analysis:** Essentia (optional), custom AudioDerived extractor
- **Rationale:** Existing WKMP technology stack, cannot change

**2. API Rate Limits**
- **AcoustID:** 3 requests/second per API key
- **MusicBrainz:** 1 request/second (50 requests/second if private server)
- **Constraint:** Must implement rate limiting to avoid API bans
- **Impact:** Sequential processing speed limited by API throttling

**3. Database Schema Compatibility**
- **Constraint:** Must extend existing passages table schema
- **Constraint:** Must not break existing wkmp-ap, wkmp-pd, wkmp-ui microservices
- **Mitigation:** SPEC031 zero-conf adds columns without breaking readers

**4. Microservices Architecture**
- **Constraint:** wkmp-ai is independent HTTP server (port 5723)
- **Constraint:** Must communicate via HTTP REST APIs and SSE
- **Constraint:** Cannot directly call functions in other microservices
- **Rationale:** WKMP microservices architecture per SPEC001

### Process Constraints

**5. Test Coverage**
- **Constraint:** Test coverage must exceed 90% per REQ-AI-NF-032
- **Impact:** Comprehensive unit/integration/system tests required for all fusion modules

**6. Documentation Standards**
- **Constraint:** Must follow GOV001 documentation hierarchy
- **Constraint:** Must use GOV002 requirement enumeration (REQ-AI-XXX format)
- **Constraint:** Must reference SPEC017 for timing, SPEC030 for architecture patterns, SPEC031 for database

**7. Zero-Configuration Startup**
- **Constraint:** Must implement REQ-AI-078 zero-config startup per CLAUDE.md mandate
- **Constraint:** All microservices must use 4-tier root folder resolution
- **Constraint:** SPEC031 schema sync must run before any database access

### Timeline Constraints

**8. Implementation Duration**
- **Estimate:** 12-14 weeks for complete implementation (per SPEC030 complete rewrite)
- **Constraint:** Phased implementation (SPEC030 infrastructure weeks 1-2, concepts weeks 3-10)
- **Verification:** User approval required before implementation begins

**9. Specification Writing**
- **Current Phase:** /plan workflow execution (Phases 1-3 this week)
- **Constraint:** Must resolve CRITICAL specification issues before proceeding to implementation

---

## Success Metrics

### Functional Success

**Quantitative:**
- ✅ 100% of 72 requirements have acceptance tests defined
- ✅ 100% of acceptance tests pass
- ✅ Test coverage >90% (per REQ-AI-NF-032)
- ✅ All 10 SSE event types implemented and tested

**Qualitative:**
- ✅ Hybrid fusion produces higher-quality metadata than single-source
- ✅ Musical flavor synthesis handles AcousticBrainz obsolescence
- ✅ Per-song error isolation prevents cascade failures
- ✅ Real-time SSE UI shows per-song progress (not just file-level)

### Quality Success

**Code Quality:**
- ✅ Modular architecture (Tier 1/2/3 separation clear)
- ✅ Ground-up recode (no legacy code dependencies)
- ✅ SPEC030 software legibility patterns followed

**Database Quality:**
- ✅ Source provenance tracked for all metadata/flavor fields
- ✅ SPEC017 tick-based timing compliance (all timing fields INTEGER ticks)
- ✅ SPEC031 zero-conf schema maintenance works (17 columns auto-added)

### User Experience Success

**Progress Visibility:**
- ✅ Real-time per-song progress updates
- ✅ Accurate ETA (estimated time remaining)
- ✅ Clear current operation display
- ✅ Error and warning visibility

**Reliability:**
- ✅ Single-song failure does not abort entire import
- ✅ Graceful degradation when external services unavailable
- ✅ Zero-configuration startup (no manual schema setup)

---

## Risk Summary

**High Risk Areas (Require Attention):**
1. **API Rate Limiting:** AcoustID/MusicBrainz throttling may slow imports
2. **Bayesian Fusion Complexity:** Mathematical correctness critical for identity resolution
3. **SPEC031 Dependency:** Zero-conf schema maintenance must be implemented first
4. **Essentia Integration:** Optional dependency, fallback must work correctly

**Mitigation Strategies:**
- Comprehensive unit tests for fusion algorithms (verify mathematical correctness)
- Integration tests for API clients (mock API responses, test rate limiting)
- Verify SPEC031 availability in wkmp-common before beginning implementation
- Test Essentia fallback scenarios (ensure AudioDerived-only path works)

---

## Next Steps (After Phase 1 Complete)

1. **Phase 2:** Specification completeness verification
   - Analyze all 72 requirements for completeness, ambiguity, conflicts
   - Identify CRITICAL issues blocking implementation
   - Resolve open questions from specification (lines 1332-1345)

2. **Phase 3:** Acceptance test definition
   - Define Given/When/Then tests for all 72 requirements
   - Create traceability matrix (100% requirement → test coverage)
   - Identify test data needs

3. **User Checkpoint:** Review scope, confirm understanding, approve proceeding
