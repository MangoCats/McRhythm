# Implementation Breakdown: WKMP-AI Audio Import System Recode

**Plan:** PLAN024
**Created:** 2025-11-09
**Purpose:** Break down implementation into concrete tasks, modules, and dependencies

**Phase:** Phase 5 - Implementation Breakdown

---

## Module Dependency Graph

```
┌─────────────────────────────────────────────────────────────┐
│ Phase 0: Infrastructure (Weeks 1-2)                         │
├─────────────────────────────────────────────────────────────┤
│ 1. SPEC031 Verification                                     │
│ 2. FFI Wrappers (Chromaprint)                              │
│ 3. Database Schema Sync                                     │
│ 4. Base Traits & Types                                      │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ Phase 1: Tier 1 Extractors (Weeks 3-5)                     │
├─────────────────────────────────────────────────────────────┤
│ 5. ID3 Extractor                                            │
│ 6. Chromaprint Analyzer                                     │
│ 7. AcoustID Client                                          │
│ 8. MusicBrainz Client                                       │
│ 9. Essentia Analyzer                                        │
│ 10. AudioDerived Extractor                                  │
│ 11. ID3 Genre Mapper                                        │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ Phase 2: Tier 2 Fusion (Weeks 6-8)                         │
├─────────────────────────────────────────────────────────────┤
│ 12. Identity Resolver                                       │
│ 13. Metadata Fuser                                          │
│ 14. Flavor Synthesizer                                      │
│ 15. Boundary Fuser                                          │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ Phase 3: Tier 3 Validation (Weeks 9-10)                    │
├─────────────────────────────────────────────────────────────┤
│ 16. Consistency Validator                                   │
│ 17. Completeness Scorer                                     │
│ 18. Quality Scorer                                          │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ Phase 4: Orchestration (Weeks 11-12)                       │
├─────────────────────────────────────────────────────────────┤
│ 19. Workflow Orchestrator                                   │
│ 20. SSE Event System                                        │
│ 21. HTTP API Endpoints                                      │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│ Phase 5: Integration & Testing (Weeks 13-14)               │
├─────────────────────────────────────────────────────────────┤
│ 22. Integration Tests                                       │
│ 23. System Tests                                            │
│ 24. Performance Testing                                     │
│ 25. Documentation                                           │
└─────────────────────────────────────────────────────────────┘
```

---

## Task Breakdown (26 Tasks)

### Infrastructure Tasks (Weeks 1-2)

#### TASK-000: File-Level Import Tracking

- **Objective:** Implement file-level import tracking, user approval, and intelligent skip logic
- **Deliverable:** `wkmp-ai/src/services/file_tracker.rs` (350 LOC production + 300 LOC unit tests)
- **Effort:** 2 days (includes unit test development)
- **Dependencies:** None for coding (can parallelize with TASK-001); TASK-003 required for integration testing
- **Risk:** LOW-MEDIUM (new pre-import workflow, skip logic decision tree)
- **Acceptance:**
  - Hash-based duplicate detection works
  - Skip logic decision tree correct (user approval > modification time > confidence thresholds)
  - File-level confidence aggregation correct (MIN of passage composite scores)
  - Metadata merging preserves higher-confidence values
  - Re-import attempt limiting prevents infinite loops
- **Implementation:** Per Amendment 8 (REQ-AI-009-01 through REQ-AI-009-11)

**Components:**
- Pre-import skip logic (Phase -1)
- File hash computation (SHA-256)
- Confidence aggregation formulas
- Metadata merge algorithm
- Re-import attempt tracking

**Unit Tests:**
- Skip decision tree (7 skip conditions + 2 supporting algorithms)
- Confidence aggregation (various passage score combinations)
- Metadata merge (conflict resolution, confidence-based overwrite)
- Re-import loop prevention

#### TASK-001: SPEC031 Verification
- **Objective:** Verify SPEC031 SchemaSync exists in wkmp-common
- **Deliverable:** `wkmp-common/src/database/schema_sync.rs` verified or implemented
- **Effort:** 2 days (if exists: 0.5 days verify, if missing: 2 days implement)
- **Dependencies:** None
- **Risk:** HIGH if missing (blocks zero-config startup)
- **Acceptance:** SchemaSync trait available, unit tests passing

#### TASK-002: Chromaprint FFI Wrapper
- **Objective:** Create safe Rust wrapper for libchromaprint
- **Deliverable:** `wkmp-ai/src/ffi/chromaprint.rs` (300 LOC)
- **Effort:** 3 days
- **Dependencies:** None (can parallelize with TASK-001)
- **Risk:** MEDIUM (FFI safety, memory leaks)
- **Acceptance:** Unit tests passing, no memory leaks (valgrind), generates valid base64 fingerprints
- **Implementation:** Per IMPL013-chromaprint_integration.md

#### TASK-003: Database Schema Sync
- **Objective:** Implement SchemaSync for passages table (17 new columns) + files table (7 new columns)
- **Deliverable:** `wkmp-ai/src/database/schema.rs` (250 LOC)
- **Effort:** 2 days
- **Dependencies:** TASK-001 complete
- **Risk:** LOW
- **Acceptance:**
  - Fresh DB auto-creates all columns (passages + files)
  - Old DB auto-migrates without data loss
  - Files table columns: import_completed_at, import_success_confidence, user_approved_at, metadata_import_completed_at, metadata_confidence, reimport_attempt_count, last_reimport_attempt_at
  - Passages table columns: 17 per existing spec
- **Implementation:** Per REQ-AI-080-086 (passages), REQ-AI-009 (files)

#### TASK-004: Base Traits & Types
- **Objective:** Define core traits (SourceExtractor, Fusion, Validation)
- **Deliverable:** `wkmp-ai/src/types.rs`, `wkmp-ai/src/extractors/mod.rs` (400 LOC)
- **Effort:** 2 days
- **Dependencies:** None
- **Risk:** LOW
- **Acceptance:** Trait definitions compile, documented with examples

---

### Tier 1 Extractor Tasks (Weeks 3-5)

#### TASK-005: ID3 Extractor
- **Objective:** Extract ID3 metadata from audio files
- **Deliverable:** `wkmp-ai/src/extractors/id3_extractor.rs` (150 LOC)
- **Effort:** 1.5 days
- **Dependencies:** TASK-004 (SourceExtractor trait)
- **Risk:** LOW
- **Acceptance:** Extracts title, artist, album, MBID; handles missing tags gracefully

#### TASK-006: Chromaprint Analyzer
- **Objective:** Generate Chromaprint fingerprints from PCM audio
- **Deliverable:** `wkmp-ai/src/extractors/chromaprint_analyzer.rs` (200 LOC)
- **Effort:** 2 days
- **Dependencies:** TASK-002 (FFI wrapper), TASK-004 (trait)
- **Risk:** MEDIUM (FFI integration)
- **Acceptance:** Generates base64 fingerprints matching known test vectors, handles edge cases

#### TASK-007: AcoustID Client
- **Objective:** Query AcoustID API with fingerprints
- **Deliverable:** `wkmp-ai/src/extractors/acoustid_client.rs` (250 LOC)
- **Effort:** 2.5 days
- **Dependencies:** TASK-004 (trait), TASK-003 (PARAM-AI-001 for rate limit)
- **Risk:** MEDIUM (API integration, rate limiting)
- **Acceptance:** Returns MBIDs for known fingerprints, rate limiting works, handles API errors

#### TASK-008: MusicBrainz Client
- **Objective:** Query MusicBrainz API for Recording metadata
- **Deliverable:** `wkmp-ai/src/extractors/musicbrainz_client.rs` (300 LOC)
- **Effort:** 3 days
- **Dependencies:** TASK-004 (trait), TASK-003 (PARAM-AI-002 for rate limit)
- **Risk:** MEDIUM (XML parsing, rate limiting, HTTP 503 handling)
- **Acceptance:** Returns metadata for known MBIDs, rate limiting works, User-Agent header set

#### TASK-009: Essentia Analyzer
- **Objective:** Extract musical features via essentia_streaming command
- **Deliverable:** `wkmp-ai/src/extractors/essentia_analyzer.rs` (200 LOC)
- **Effort:** 2.5 days
- **Dependencies:** TASK-004 (trait)
- **Risk:** MEDIUM (process execution, JSON parsing, timeout handling)
- **Acceptance:** Detection works (installed/not installed), extracts features when available, graceful degradation

#### TASK-010: AudioDerived Extractor
- **Objective:** Compute basic musical features (tempo, loudness, spectral)
- **Deliverable:** `wkmp-ai/src/extractors/audio_derived_extractor.rs` (400 LOC)
- **Effort:** 4 days
- **Dependencies:** TASK-004 (trait)
- **Risk:** MEDIUM (algorithm complexity)
- **Acceptance:** Tempo detection ±5 BPM on test files, loudness within 1 dB, spectral features reasonable

#### TASK-011: ID3 Genre Mapper
- **Objective:** Map ID3 genre tags to musical flavor characteristics
- **Deliverable:** `wkmp-ai/src/extractors/id3_genre_mapper.rs` (150 LOC)
- **Effort:** 1.5 days
- **Dependencies:** TASK-004 (trait), TASK-005 (ID3 extractor)
- **Risk:** LOW
- **Acceptance:** Maps 50+ common genres, unknown genres return neutral flavor

---

### Tier 2 Fusion Tasks (Weeks 6-8)

#### TASK-012: Identity Resolver
- **Objective:** Bayesian fusion of MBIDs from multiple sources
- **Deliverable:** `wkmp-ai/src/fusion/identity_resolver.rs` (350 LOC)
- **Effort:** 4 days
- **Dependencies:** TASK-007 (AcoustID), TASK-008 (MusicBrainz), TASK-005 (ID3)
- **Risk:** HIGH (Bayesian math correctness)
- **Acceptance:** Unit tests with hand-verified calculations, handles conflicts correctly, confidence scores accurate

#### TASK-013: Metadata Fuser
- **Objective:** Field-wise weighted fusion of metadata
- **Deliverable:** `wkmp-ai/src/fusion/metadata_fuser.rs` (250 LOC)
- **Effort:** 2.5 days
- **Dependencies:** TASK-005 (ID3), TASK-008 (MusicBrainz)
- **Risk:** LOW
- **Acceptance:** Selects best metadata per field, consistency checks work, handles missing fields

#### TASK-014: Flavor Synthesizer
- **Objective:** Characteristic-wise weighted averaging of flavor vectors
- **Deliverable:** `wkmp-ai/src/fusion/flavor_synthesizer.rs` (300 LOC)
- **Effort:** 3 days
- **Dependencies:** TASK-009 (Essentia), TASK-010 (AudioDerived), TASK-011 (Genre)
- **Risk:** MEDIUM (weighted averaging correctness, normalization)
- **Acceptance:** Weighted average mathematically correct, completeness scoring works, handles missing sources

#### TASK-015: Boundary Fuser
- **Objective:** Refine passage boundaries based on metadata
- **Deliverable:** `wkmp-ai/src/fusion/boundary_fuser.rs` (200 LOC)
- **Effort:** 2 days
- **Dependencies:** TASK-013 (metadata for duration), SPEC017 tick utilities
- **Risk:** LOW
- **Acceptance:** Refines boundaries to match Recording duration, handles multi-song passages

---

### Tier 3 Validation Tasks (Weeks 9-10)

#### TASK-016: Consistency Validator
- **Objective:** Title, duration, genre-flavor consistency checks
- **Deliverable:** `wkmp-ai/src/validation/consistency_validator.rs` (250 LOC)
- **Effort:** 2.5 days
- **Dependencies:** TASK-013 (metadata), TASK-014 (flavor)
- **Risk:** LOW
- **Acceptance:** Levenshtein matching works, duration tolerance correct, genre-flavor alignment reasonable

#### TASK-017: Completeness Scorer
- **Objective:** Compute metadata and flavor completeness scores
- **Deliverable:** `wkmp-ai/src/validation/completeness_scorer.rs` (150 LOC)
- **Effort:** 1.5 days
- **Dependencies:** TASK-013 (metadata), TASK-014 (flavor), TASK-003 (PARAM-AI-004)
- **Risk:** LOW
- **Acceptance:** Scores match formula, handles missing data correctly

#### TASK-018: Quality Scorer
- **Objective:** Compute overall quality score from all signals
- **Deliverable:** `wkmp-ai/src/validation/quality_scorer.rs` (150 LOC)
- **Effort:** 1.5 days
- **Dependencies:** TASK-012 (identity), TASK-016 (consistency), TASK-017 (completeness)
- **Risk:** LOW
- **Acceptance:** Weighted average correct, score in [0.0, 1.0]

---

### Orchestration Tasks (Weeks 11-12)

#### TASK-019: Workflow Orchestrator
- **Objective:** Complete import pipeline (Discovery Phase through Phase 7), per-song sequential processing
- **Deliverable:** `wkmp-ai/src/services/workflow_orchestrator.rs` (800 LOC)
- **Effort:** 7 days (was 5 days originally, +1 day for Phase -1/7 in Amendment 8, +1 day for Discovery Phase in Amendment 9)
- **Dependencies:** All Tier 1, 2, 3 tasks complete, TASK-000 (file tracker)
- **Risk:** MEDIUM (complex orchestration, error handling, skip logic integration, file discovery)
- **Acceptance:**
  - Discovery Phase: Recursively scans folders, filters by extensions, counts files, emits SSE events
  - Phase -1 (pre-import skip logic) works correctly (7 skip conditions evaluated)
  - Phase 0-6 sequence correct (existing spec)
  - Phase 7 (post-import finalization) updates files table
  - Error isolation works (per-passage failures don't abort file)
  - Parallel extraction works (Tier 1)
  - Skip logic respects user approval (absolute protection)
  - Confidence thresholds loaded from database parameters
  - Discovery error handling works (permission denied, symlink loops, empty results)
- **Implementation:** Per Amendment 7 (Phase 0-6), Amendment 8 (Phase -1, Phase 7), Amendment 9 (Discovery Phase)

**Phases:**
- Discovery Phase: Pre-scan folders, count files, emit DiscoveryStarted/Progress/Complete SSE events
- Phase -1: Pre-import skip logic (file tracker integration)
- Phase 0: Passage boundary detection
- Phase 1-6: Per-passage processing (existing spec)
- Phase 7: Post-import finalization (confidence aggregation, flagging)

#### TASK-020: SSE Event System
- **Objective:** Real-time event streaming for UI progress (including Discovery Phase events)
- **Deliverable:** `wkmp-ai/src/events/sse_broadcaster.rs` (300 LOC)
- **Effort:** 2.5 days (no change, +50 LOC for Discovery events within existing estimate)
- **Dependencies:** TASK-019 (workflow emits events)
- **Risk:** LOW
- **Acceptance:** 14 event types emitted (10 original + 4 Discovery events), throttling works (30/sec max for ImportProgress, 1/sec for DiscoveryProgress), no event loss

#### TASK-021: HTTP API Endpoints
- **Objective:** Import control + user approval endpoints + file discovery request format
- **Deliverable:** `wkmp-ai/src/api/import_routes.rs` (300 LOC)
- **Effort:** 2.5 days (was 2 days, +0.5 day for approval endpoints in Amendment 8, Discovery Phase request format within existing estimate)
- **Dependencies:** TASK-019 (orchestrator), TASK-020 (SSE), TASK-000 (file tracker)
- **Risk:** LOW
- **Acceptance:**
  - POST /import/start accepts new request format (root_paths array, recursive boolean, file_extensions array)
  - GET /import/events (SSE connection stable, includes Discovery events)
  - GET /import/status (status queries accurate)
  - POST /import/files/{id}/approve (user approval recorded)
  - POST /import/files/{id}/reject (triggers re-import)
  - GET /import/files/pending-review (lists flagged files)
  - All endpoints emit appropriate SSE events
- **Implementation:** Per Amendment 8 (user approval API endpoints), Amendment 9 (POST /import/start request format)

---

### Integration & Testing Tasks (Weeks 13-14)

#### TASK-022: Integration Tests
- **Objective:** End-to-end passage processing tests + file discovery tests
- **Deliverable:** `wkmp-ai/tests/integration/` (900 LOC: 700 original + 100 Amendment 8 + 100 Amendment 9)
- **Effort:** 3 days (no change, Amendment 9 tests within existing estimate)
- **Dependencies:** All implementation tasks complete
- **Risk:** LOW
- **Acceptance:** All acceptance tests from 03_acceptance_tests.md passing (93/93 requirements including Amendments 8 and 9)

#### TASK-023: System Tests
- **Objective:** Full import scenarios with real audio files
- **Deliverable:** `wkmp-ai/tests/system/` (400 LOC)
- **Effort:** 2 days
- **Dependencies:** TASK-022 complete
- **Risk:** LOW
- **Acceptance:** Happy path works, error scenarios handled, performance acceptable

#### TASK-024: Performance Testing
- **Objective:** Verify performance targets (import time, throughput)
- **Deliverable:** Performance test suite (200 LOC)
- **Effort:** 2 days
- **Dependencies:** TASK-023 complete
- **Risk:** LOW
- **Acceptance:** 10-passage import < 5 minutes, parallel extraction speedup >1.5x

#### TASK-025: Documentation
- **Objective:** Update IMPL documents, API docs, code comments
- **Deliverable:** Updated IMPL012, IMPL013, inline docs
- **Effort:** 2 days
- **Dependencies:** All tasks complete
- **Risk:** LOW
- **Acceptance:** All modules documented, API docs complete, IMPL docs reflect implementation

---

## Implementation Sequence

**Critical Path:** TASK-001 → TASK-003 → TASK-004 → Tier 1 → Tier 2 → Tier 3 → TASK-019 → TASK-020 → TASK-021 → Testing

**Parallelization Opportunities:**
- Week 1: TASK-000, TASK-001, and TASK-002 in parallel (3 concurrent tasks)
- Weeks 3-5: Tier 1 extractors (TASK-005 through TASK-011) can be parallelized (7 concurrent tasks)
- Weeks 9-10: Tier 3 validators (TASK-016, TASK-017, TASK-018) can be parallelized

**Note:** TASK-000 (File-Level Tracking) can run in parallel with TASK-001 and TASK-002 during Week 1

---

## Lines of Code Estimate

| Category | LOC Estimate |
|----------|--------------|
| Infrastructure (TASK-000 to TASK-004) | 1,300 |
| Tier 1 Extractors (TASK-005 to TASK-011) | 1,650 |
| Tier 2 Fusion (TASK-012 to TASK-015) | 1,100 |
| Tier 3 Validation (TASK-016 to TASK-018) | 550 |
| Orchestration (TASK-019 to TASK-021) | 1,400 |
| Tests (TASK-022 to TASK-024) | 1,900 |
| **Total Production Code** | **6,000 LOC** |
| **Total Test Code** | **1,900 LOC** |
| **Grand Total** | **7,900 LOC** |

**Changes from original estimate (+1,250 LOC total):**
- Infrastructure: +400 LOC (TASK-000: 350 LOC production code, TASK-003: +50 LOC for files table schema)
- Orchestration: +350 LOC total:
  - Amendment 8: +150 LOC (TASK-019: +100 LOC Phase -1/7, TASK-021: +50 LOC approval endpoints)
  - Amendment 9: +200 LOC (TASK-019: +100 LOC Discovery Phase, TASK-020: +50 LOC Discovery events, TASK-021: +50 LOC request format)
- Tests: +500 LOC distributed across test tasks:
  - TASK-000 unit tests: 300 LOC (skip logic, confidence aggregation, metadata merge)
  - TASK-022 integration tests: +200 LOC (Amendment 8: +100 LOC, Amendment 9: +100 LOC)
  - Note: All 300 LOC TASK-000 unit tests included in TASK-000's 2-day effort estimate

---

**Document Version:** 3.0 (Updated for Amendment 9)
**Last Updated:** 2025-11-09
**Phase 5 Status:** ✅ COMPLETE (Updated with Amendments 8 and 9)

**Amendment 9 Updates:**
- TASK-019: Added Discovery Phase (+1 day effort, +100 LOC, now 7 days total)
- TASK-020: Added Discovery SSE events (+50 LOC, effort unchanged)
- TASK-021: Updated POST /import/start request format (+50 LOC, effort unchanged)
- TASK-022: Added file discovery integration tests (+100 LOC, effort unchanged)
- LOC totals: Production 6,000 (+200), Tests 1,900 (+100), Grand Total 7,900 (+300)

**MED Clarifications Applied (Amendment 8):**
- MED-002: Clarified test LOC attribution (TASK-000: 300 LOC unit tests, TASK-022: +100 LOC integration tests)
- MED-001: Clarified TASK-000 dependency (coding parallelizes, integration testing requires TASK-003)
