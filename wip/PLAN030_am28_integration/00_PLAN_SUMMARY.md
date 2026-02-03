# PLAN030: Full am28 Album Matching Integration

**Status:** ✅ Planning Complete - Ready for Implementation
**Created:** 2025-11-26
**Specification Source:** `wkmp-ai/examples/am28/` (8,793 lines)
**Plan Location:** `wip/PLAN030_am28_integration/`

---

## READ THIS FIRST

This plan integrates the standalone am28 album matching algorithm into the wkmp-ai service. The am28 code currently exists as a CLI example; this plan migrates it into the library's `AlbumMatcher` service.

**For Implementation:**
- Read this summary
- Follow increments in order (dependency chain)
- Run tests after each increment
- Target: 2-4 hours per increment

**Context Window Budget:** ~650 lines per increment (summary + increment details)

---

## Executive Summary

### Problem Being Solved

The `AlbumMatcher` service in `wkmp-ai/src/matching/album_matcher.rs` is currently a **placeholder** - it returns "not yet implemented" for all album matching requests. The actual algorithm exists in `wkmp-ai/examples/am28/` but is:
- A standalone CLI tool (not a library)
- Tightly coupled to file I/O and command-line arguments
- Not integrated with wkmp-ai services

### Solution Approach

**Incremental migration** - Move am28 code module-by-module into `wkmp-ai/src/matching/`, adapting each component to:
1. Work as a library (no CLI dependencies)
2. Integrate with existing wkmp-ai services (MusicBrainz client, silence detector)
3. Return results via the `AlbumMatchResult` type already defined
4. Support async/await for non-blocking operation

### Implementation Status

**Foundation Complete (PLAN026):**
- ✅ `matching/` module structure created
- ✅ `AlbumMatchResult`, `MatchingStage`, `MatchedTrack`, `Edition` types defined
- ✅ `AlbumMatcher` service placeholder implemented
- ✅ `ResultIntegrator` for DB persistence
- ✅ `FallbackHandler` for edge cases

**This Plan (PLAN030):**
- ✅ Phase 1: Scope defined
- ✅ Phase 2: Specification verified (2 HIGH issues identified, resolvable)
- ✅ Phase 3: 60 acceptance tests defined (45 unit, 12 integration, 3 system)
- ✅ 15 increment specifications complete

---

## Requirements Summary

### Functional Requirements

| Req ID | Description | Priority |
|--------|-------------|----------|
| REQ-AM-001 | Decode audio file to PCM samples | P0 |
| REQ-AM-002 | Extract metadata (ID3 tags + filename) | P0 |
| REQ-AM-003 | Search MusicBrainz for candidate releases | P0 |
| REQ-AM-004 | Group releases into editions by track pattern | P0 |
| REQ-AM-005 | Stage 2: Parameter grid search (180 combinations) | P0 |
| REQ-AM-006 | Stage 3: Over-segmentation assembly (DP) | P1 |
| REQ-AM-007 | Stage 4: Quiet spot detection (RMS profiling) | P1 |
| REQ-AM-008 | Stage 5: Extra track merging | P2 |
| REQ-AM-009 | Artist verification (Jaro-Winkler similarity) | P0 |
| REQ-AM-010 | Early-exit optimization (100% match signals) | P1 |
| REQ-AM-011 | MusicBrainz response caching | P1 |
| REQ-AM-012 | Single-track discriminator (reject non-albums) | P0 |

### Non-Functional Requirements

| Req ID | Description | Priority |
|--------|-------------|----------|
| REQ-NF-001 | Async/await compatible (tokio runtime) | P0 |
| REQ-NF-002 | Library API (no CLI dependencies) | P0 |
| REQ-NF-003 | Integrate with existing MusicBrainz client | P1 |
| REQ-NF-004 | Memory efficient (stream audio, don't load all) | P2 |

---

## Scope

### ✅ In Scope

- **Core matching algorithm**: Stages 2-5 from am28
- **MusicBrainz integration**: Search, release details, caching
- **Silence detection**: Parameter grid search
- **Edition handling**: Grouping, filtering, scoring
- **Artist verification**: Jaro-Winkler similarity
- **Single-track detection**: Reject non-album files
- **Types migration**: Move am28 types into library

### ❌ Out of Scope

- **CLI interface**: Not migrating command-line parsing
- **Parallel album processing**: Single-album API only
- **AcoustID verification**: Reserved for future (commented in am28)
- **Fingerprint-based matching**: Uses silence detection, not fingerprints
- **UI integration**: API only, UI handled by PLAN027

---

## Architecture Overview

### Current State (am28 example)

```
wkmp-ai/examples/am28/
├── main.rs           (1137 lines) - CLI entry, orchestration loop
├── types.rs          (783 lines)  - All data types
├── orchestration.rs  (361 lines)  - Edition testing stages 2-5
├── silence_detection.rs (387 lines) - Silence detection core
├── single_track_discriminator.rs (370 lines)
├── metadata.rs       (581 lines)  - ID3/path extraction
├── constants.rs      (408 lines)  - Configuration
├── helpers.rs        (111 lines)  - Utilities
├── stages/           - Individual stage implementations
├── matching/         - Candidate/edition handling
├── musicbrainz/      - API client and caching
└── utils/            - Audio, timing, fingerprint utilities
```

### Target State (library integration)

```
wkmp-ai/src/matching/
├── mod.rs            - Module exports
├── album_matcher.rs  - Main AlbumMatcher service (enhanced)
├── types.rs          - Merged types (existing + am28)
├── orchestration.rs  - NEW: Edition testing stages 2-5
├── silence_detection.rs - NEW: Silence detection
├── single_track.rs   - NEW: Single-track discriminator
├── metadata.rs       - NEW: Metadata extraction
├── constants.rs      - NEW: Configuration
├── stages/
│   ├── mod.rs
│   ├── stage2.rs     - Parameter grid search
│   ├── stage3.rs     - Over-segmentation assembly
│   ├── stage4.rs     - Quiet spot detection
│   └── stage5.rs     - Extra track merging
├── editions/
│   ├── mod.rs
│   ├── grouping.rs   - Edition grouping
│   ├── scoring.rs    - Match scoring
│   └── filtering.rs  - Edition filtering
└── musicbrainz/
    ├── mod.rs
    ├── search.rs     - Comprehensive search
    └── cache.rs      - Response caching
```

---

## Implementation Roadmap

### Phase 1: Foundation (Increments 1-3)

**Increment 1: Types Migration** (~3 hours)
- Merge am28 types.rs into matching/types.rs
- Resolve conflicts with existing types
- Tests: Type compilation, serialization roundtrip

**Increment 2: Constants & Configuration** (~2 hours)
- Create matching/constants.rs from am28
- Make constants configurable via AlbumMatcherConfig
- Tests: Config defaults, parameter ranges

**Increment 3: Metadata Extraction** (~3 hours)
- Create matching/metadata.rs
- Integrate with existing MetadataExtractor service
- Tests: ID3 extraction, path parsing, reconciliation

### Phase 2: Core Detection (Increments 4-6)

**Increment 4: Silence Detection** (~4 hours)
- Create matching/silence_detection.rs
- Parameter grid computation (180 combinations)
- Tests: Silence cache generation, track boundary detection

**Increment 5: Single-Track Discriminator** (~3 hours)
- Create matching/single_track.rs
- Pre-decode and post-decode analysis
- Tests: Single-track rejection, album acceptance

**Increment 6: Edition Grouping** (~4 hours)
- Create matching/editions/ module
- Grouping by track count/duration pattern
- Filtering and sorting by relevance
- Tests: Edition grouping, filtering criteria

### Phase 3: Matching Stages (Increments 7-10)

**Increment 7: Stage 2 - Parameter Optimization** (~4 hours)
- Create stages/stage2.rs
- Grid search with early-exit
- Tests: Parameter combinations, match percentage

**Increment 8: Stage 3 - Over-segmentation Assembly** (~4 hours)
- Create stages/stage3.rs
- Dynamic programming optimal merging
- Tests: Segment assembly, DP correctness

**Increment 9: Stage 4 - Quiet Spot Detection** (~3 hours)
- Create stages/stage4.rs
- RMS profiling, 25% penalty application
- Tests: RMS calculation, penalty application

**Increment 10: Stage 5 - Extra Track Merging** (~2 hours)
- Create stages/stage5.rs
- Adjacent track merging
- Tests: Track count adjustment, duration preservation

### Phase 4: Integration (Increments 11-13)

**Increment 11: Orchestration** (~4 hours)
- Create matching/orchestration.rs
- Stage coordination with early-exit
- Tests: Stage sequencing, early-exit triggers

**Increment 12: MusicBrainz Integration** (~4 hours)
- Integrate with existing MusicBrainzClient
- Add caching layer
- Tests: Search queries, cache hit/miss

**Increment 13: AlbumMatcher Implementation** (~4 hours)
- Replace placeholder with full implementation
- Wire all components together
- Tests: End-to-end album matching

### Phase 5: Testing & Polish (Increments 14-15)

**Increment 14: Integration Tests** (~4 hours)
- Full pipeline tests with sample audio
- Edge case coverage
- Performance benchmarks

**Increment 15: Documentation & Cleanup** (~2 hours)
- API documentation
- Usage examples
- Remove am28 example (graduated to library)

---

## Effort Summary

| Phase | Increments | Estimated Hours |
|-------|------------|-----------------|
| Foundation | 1-3 | 8 hours |
| Core Detection | 4-6 | 11 hours |
| Matching Stages | 7-10 | 13 hours |
| Integration | 11-13 | 12 hours |
| Testing & Polish | 14-15 | 6 hours |
| **Total** | **15** | **50 hours** |

---

## Risk Assessment

### High Risk

1. **Audio Decoding Compatibility**
   - am28 uses `symphonia` directly; wkmp-ai has existing decoders
   - Mitigation: Reuse existing `utils/audio.rs` in wkmp-ai

2. **MusicBrainz Rate Limiting**
   - Tests may hit rate limits if not properly mocked
   - Mitigation: Use existing MusicBrainzClient with built-in rate limiting

### Medium Risk

3. **Type Conflicts**
   - am28 types may conflict with existing matching/types.rs
   - Mitigation: Careful merge in Increment 1, prefer existing types

4. **Performance Regression**
   - Library version may be slower than optimized CLI
   - Mitigation: Benchmark after integration, optimize if needed

### Low Risk

5. **Test Coverage Gaps**
   - Some am28 code paths may lack tests
   - Mitigation: Add tests during migration

---

## Dependencies

### Existing Code (Reuse)

| Component | Location | Lines |
|-----------|----------|-------|
| AlbumMatcher placeholder | matching/album_matcher.rs | 196 |
| Existing types | matching/types.rs | 209 |
| MetadataExtractor | services/metadata_extractor.rs | ~300 |
| MusicBrainzClient | services/musicbrainz_client.rs | ~400 |
| SilenceDetector | services/silence_detector.rs | ~300 |
| Audio utilities | utils/audio.rs | ~200 |

### New Code (From am28)

| Component | Source | Target |
|-----------|--------|--------|
| types.rs | examples/am28/types.rs | matching/types.rs (merge) |
| orchestration.rs | examples/am28/orchestration.rs | matching/orchestration.rs |
| stages/* | examples/am28/stages/* | matching/stages/* |
| silence_detection.rs | examples/am28/silence_detection.rs | matching/silence_detection.rs |

---

## Success Metrics

### Quantitative

- ✅ All 15 increments complete
- ✅ 100% test coverage for new code
- ✅ Matching accuracy ≥ am28 CLI (no regression)
- ✅ Build time increase < 10%

### Qualitative

- ✅ Clean API for ContentTypeClassifier integration
- ✅ No CLI dependencies in library code
- ✅ Consistent with wkmp-ai coding conventions

---

## Next Steps

### Immediate (Ready Now)

1. Review this plan
2. Approve to proceed with Increment 1
3. Set up test fixtures (sample audio files)

### Implementation Sequence

1. Start with Increment 1: Types Migration
2. Verify compilation after each increment
3. Run tests before proceeding to next increment
4. Checkpoint after Phase 2 (Increment 6) for review

---

## Document Navigation

**Start Here:** This file (00_PLAN_SUMMARY.md)

**Planning Documents:**
- `requirements_index.md` - All requirements with line references
- `01_specification_issues.md` - Phase 2 analysis (2 HIGH, 4 MEDIUM, 3 LOW issues)
- `02_test_specifications/test_index.md` - 60 acceptance tests
- `02_test_specifications/traceability_matrix.md` - 100% requirement coverage

**Increment Specifications:**
- `04_increments/increment_01.md` - Types Migration
- `04_increments/increment_02.md` - Constants & Configuration
- `04_increments/increment_03.md` - Metadata Extraction
- `04_increments/increment_04.md` - Silence Detection
- `04_increments/increment_05.md` - Single-Track Discriminator
- `04_increments/increment_06.md` - Edition Grouping
- `04_increments/increment_07.md` - Stage 2: Parameter Grid Search
- `04_increments/increment_08.md` - Stage 3: Over-Segmentation Assembly
- `04_increments/increment_09.md` - Stage 4: Quiet Spot Detection
- `04_increments/increment_10.md` - Stage 5: Extra Track Merging
- `04_increments/increment_11.md` - Stage Orchestration
- `04_increments/increment_12.md` - MusicBrainz Integration
- `04_increments/increment_13.md` - AlbumMatcher Implementation
- `04_increments/increment_14.md` - Integration Tests
- `04_increments/increment_15.md` - Documentation & Cleanup
