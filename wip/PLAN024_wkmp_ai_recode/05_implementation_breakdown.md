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

## Task Breakdown (25 Tasks)

### Infrastructure Tasks (Weeks 1-2)

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
- **Objective:** Implement SchemaSync for passages table (17 new columns)
- **Deliverable:** `wkmp-ai/src/database/schema.rs` (200 LOC)
- **Effort:** 2 days
- **Dependencies:** TASK-001 complete
- **Risk:** LOW
- **Acceptance:** Fresh DB auto-creates columns, old DB auto-migrates, no data loss

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
- **Objective:** Phase 0-6 pipeline, per-song sequential processing
- **Deliverable:** `wkmp-ai/src/services/workflow_orchestrator.rs` (600 LOC)
- **Effort:** 5 days
- **Dependencies:** All Tier 1, 2, 3 tasks complete
- **Risk:** MEDIUM (complex orchestration, error handling)
- **Acceptance:** Phase sequence correct, error isolation works, parallel extraction works

#### TASK-020: SSE Event System
- **Objective:** Real-time event streaming for UI progress
- **Deliverable:** `wkmp-ai/src/events/sse_broadcaster.rs` (250 LOC)
- **Effort:** 2.5 days
- **Dependencies:** TASK-019 (workflow emits events)
- **Risk:** LOW
- **Acceptance:** 10 event types emitted, throttling works (30/sec max), no event loss

#### TASK-021: HTTP API Endpoints
- **Objective:** POST /import/start, GET /import/events, GET /import/status
- **Deliverable:** `wkmp-ai/src/api/import_routes.rs` (200 LOC)
- **Effort:** 2 days
- **Dependencies:** TASK-019 (orchestrator), TASK-020 (SSE)
- **Risk:** LOW
- **Acceptance:** Endpoints work, SSE connection stable, status queries accurate

---

### Integration & Testing Tasks (Weeks 13-14)

#### TASK-022: Integration Tests
- **Objective:** End-to-end passage processing tests
- **Deliverable:** `wkmp-ai/tests/integration/` (800 LOC)
- **Effort:** 3 days
- **Dependencies:** All implementation tasks complete
- **Risk:** LOW
- **Acceptance:** All acceptance tests from 03_acceptance_tests.md passing

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
- Week 1: TASK-001 and TASK-002 in parallel
- Weeks 3-5: Tier 1 extractors (TASK-005 through TASK-011) can be parallelized (7 concurrent tasks)
- Weeks 9-10: Tier 3 validators (TASK-016, TASK-017, TASK-018) can be parallelized

---

## Lines of Code Estimate

| Category | LOC Estimate |
|----------|--------------|
| Infrastructure (TASK-001 to TASK-004) | 900 |
| Tier 1 Extractors (TASK-005 to TASK-011) | 1,650 |
| Tier 2 Fusion (TASK-012 to TASK-015) | 1,100 |
| Tier 3 Validation (TASK-016 to TASK-018) | 550 |
| Orchestration (TASK-019 to TASK-021) | 1,050 |
| Tests (TASK-022 to TASK-024) | 1,400 |
| **Total Production Code** | **5,250 LOC** |
| **Total Test Code** | **1,400 LOC** |
| **Grand Total** | **6,650 LOC** |

---

**Document Version:** 1.0
**Last Updated:** 2025-11-09
**Phase 5 Status:** ✅ COMPLETE
