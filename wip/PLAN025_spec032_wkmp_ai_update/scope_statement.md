# Scope Statement - PLAN025

**Plan:** SPEC032 wkmp-ai Implementation Update
**Date:** 2025-11-10

---

## ✅ In Scope

### Pipeline Architecture Changes
1. **Reorder pipeline sequence** - Move segmentation BEFORE fingerprinting
2. **Convert to per-file pipeline** - Replace batch phases with per-file processing + parallel workers
3. **Integrate new components** - Pattern analyzer, contextual matcher, confidence assessor

### New Components

#### Pattern Analyzer
- Track count detection
- Gap pattern analysis (mean, std dev)
- Segment duration statistics
- Source media classification (CD/Vinyl/Cassette/Unknown)
- Confidence scoring for pattern detection

#### Contextual Matcher
- Single-segment file matching (artist + title → MusicBrainz)
- Multi-segment file matching (album structure → MusicBrainz releases)
- Fuzzy string matching for names
- Duration and track count filtering
- Ranked candidate lists with match scores

#### Confidence Assessor
- Evidence combination algorithm (metadata + pattern + fingerprints)
- Weighted scoring (30% metadata, 60% fingerprint, 10% duration for single-segment)
- Decision thresholds (Accept >= 0.85, Review 0.60-0.85, Reject < 0.60)
- Evidence summary reporting

### Architectural Changes

#### Per-Segment Fingerprinting
- Modify `Fingerprinter` to support per-segment operation
- Extract segment PCM data from decoded audio
- Generate individual Chromaprint fingerprints per segment
- Per-segment AcoustID API queries (rate-limited)

#### Tick-Based Timing
- Implement `seconds_to_ticks()` conversion function
- Apply to all passage timing points (7 fields)
- Apply to segmentation results (silence detection boundaries)
- Apply to amplitude analysis results (lead-in/lead-out points)

### Integration and Testing
- Update `WorkflowOrchestrator` state machine
- Refactor phase modules into per-file pipeline functions
- Comprehensive unit tests for new components
- Integration tests for evidence-based identification
- System tests for complete import workflows

---

## ❌ Out of Scope

### Not Changing in This Implementation

1. **File scanner** - No changes to directory traversal logic
2. **Metadata extractor** - ID3/Vorbis/MP4 parsing unchanged
3. **Amplitude analyzer** - RMS analysis, lead-in/lead-out detection unchanged
4. **Silence detector** - Boundary detection algorithm unchanged
5. **AcousticBrainz client** - Flavor retrieval unchanged
6. **Database schema** - Using existing schema (SPEC031 compliance assumed)
7. **UI/API** - No changes to HTTP/SSE interfaces
8. **Configuration** - No new user-facing settings (using defaults)

### Deferred to Future Releases

1. **Manual MBID review queue** - Review decisions (0.60-0.85 confidence) logged but not UI
2. **Advanced fuzzy matching** - Using basic string similarity, not advanced NLP
3. **Machine learning** - Pattern classification uses heuristics, not ML models
4. **Performance optimization** - Implement correct sequence first, optimize later
5. **Confidence threshold configuration** - Using hardcoded thresholds (0.85, 0.60)
6. **Essentia integration** - Local analysis fallback unchanged
7. **Album art matching** - Not using cover art for identification
8. **Genre/style analysis** - Not using genre as evidence

### Explicitly Not Implementing

1. **Whole-library re-import** - No migration of existing imports
2. **Backwards compatibility** - Old batch-phase architecture will be replaced
3. **A/B testing** - Not comparing old vs. new approaches
4. **Real-time monitoring dashboards** - Logging only, no live dashboards
5. **Distributed import** - Still single-machine, not distributed system

---

## Assumptions

### Technical Assumptions

1. **Existing silence detection produces valid segments**
   - Assumption: Current `SilenceDetector` correctly identifies track boundaries
   - Risk: If silence detection broken, pattern analysis inherits issues
   - Mitigation: Validate silence detection before pattern analysis implementation

2. **MusicBrainz API access available and stable**
   - Assumption: MusicBrainz API accessible, rate limit (1 req/s) respected
   - Risk: API unavailable, rate limit changed, service degraded
   - Mitigation: Implement caching, exponential backoff, graceful degradation

3. **AcoustID API key configured and valid**
   - Assumption: User has configured valid AcoustID API key
   - Risk: No key, invalid key, quota exceeded
   - Mitigation: Check key at startup, provide clear error messages

4. **Test dataset available for validation**
   - Assumption: Can obtain known-good audio files for testing (licensed music, test files)
   - Risk: No test data, cannot validate accuracy claims
   - Mitigation: Use small subset of developer's personal library (with permission)

5. **Current database schema supports tick-based timing**
   - Assumption: SPEC031 compliance means tick fields exist
   - Risk: Schema missing required fields
   - Mitigation: Verify schema during Phase 2, update if needed

6. **Chromaprint FFI is thread-safe with mutex**
   - Assumption: CHROMAPRINT_LOCK mutex sufficient for per-segment parallelism
   - Risk: FFI backend not thread-safe even with mutex
   - Mitigation: Test with concurrent fingerprinting, monitor for crashes

### Process Assumptions

1. **User approval for architectural changes**
   - Assumption: Stakeholders approve replacing batch-phase architecture
   - Risk: Users prefer incremental approach (keep old + add new)
   - Mitigation: Present plan for review before implementation

2. **Acceptable performance regression during transition**
   - Assumption: Initial per-segment implementation may be slower
   - Risk: Users reject due to performance
   - Mitigation: Optimize after correctness proven, set expectations

3. **Test-first development approach**
   - Assumption: Write tests before implementation (TDD)
   - Risk: Tests may not cover all edge cases
   - Mitigation: Comprehensive test review in Phase 3

---

## Constraints

### Technical Constraints

1. **Rust Stable Channel**
   - Must compile on Rust stable (no nightly features)
   - Async: Tokio runtime (existing)
   - FFI: C bindings to Chromaprint library

2. **SQLite Database**
   - Single file database (wkmp.db)
   - No distributed database support
   - Single-writer, multiple-reader concurrency

3. **API Rate Limits**
   - MusicBrainz: 1 request/second (strict)
   - AcoustID: 3 requests/second
   - Must implement rate limiting (governor crate)

4. **Memory Constraints**
   - Decode audio file once per worker (4 concurrent)
   - Hold decoded PCM for segmentation + fingerprinting
   - Release after processing complete

5. **Performance Requirements**
   - Import throughput: ≥20 files/second (4 workers, 3-min songs)
   - Per-segment fingerprinting overhead: <20% vs. whole-file
   - Contextual matching: <2 seconds per file (MusicBrainz search)

### Process Constraints

1. **No Breaking Changes to API**
   - HTTP/SSE interfaces unchanged
   - Clients (wkmp-ui) continue working without modification

2. **Zero-Configuration Preserved**
   - Database auto-initialization (SPEC031)
   - No new required configuration
   - Defaults work out-of-box

3. **Test Coverage Target**
   - Unit tests: >80% coverage for new components
   - Integration tests: All new component interactions
   - System tests: End-to-end import workflows

### Timeline Constraints

1. **Phased Implementation**
   - Phase 1 (Critical): Pipeline reordering - 2-3 days
   - Phase 2 (High): New components - 4-5 days
   - Phase 3 (High): Per-segment fingerprinting - 2-3 days
   - Phase 4 (Medium): Tick-based timing - 1 day
   - **Total Estimated:** 9-12 days (2-3 weeks)

2. **No Parallel Work on Other Features**
   - Single developer focus (or AI agent)
   - No concurrent wkmp-ai changes during implementation

---

## Dependencies

### Internal Dependencies (WKMP Project)

**Existing Components (Used, Not Modified):**
- `wkmp-common` - Database models, events, utilities
- `FileScanner` - Directory traversal, file discovery
- `MetadataExtractor` - ID3/Vorbis/MP4 tag parsing
- `AmplitudeAnalyzer` - RMS analysis, lead-in/lead-out
- `SilenceDetector` - Silence boundary detection
- `AcousticBrainzClient` - AcousticBrainz API client
- `EssentiaClient` - Local analysis fallback

**Existing Components (Modified):**
- `WorkflowOrchestrator` - Refactor state machine + pipeline
- `Fingerprinter` - Add per-segment support
- `AcoustIDClient` - Add per-segment query support
- `MusicBrainzClient` - Used by new ContextualMatcher

**New Components (Create):**
- `PatternAnalyzer` - Segment pattern analysis (new file)
- `ContextualMatcher` - MusicBrainz search with context (new file)
- `ConfidenceAssessor` - Evidence combination (new file)

### External Dependencies

**Libraries (Existing):**
- `tokio` - Async runtime
- `axum` - HTTP server
- `sqlx` - SQLite database
- `symphonia` - Audio decoding
- `rubato` - Sample rate conversion
- `reqwest` - HTTP client (MusicBrainz, AcoustID, AcousticBrainz)
- `serde` / `serde_json` - Serialization

**Libraries (May Need to Add):**
- `strsim` or similar - Fuzzy string matching (for contextual matcher)
- `governor` - Rate limiting (may already be present)

**External Services:**
- **MusicBrainz API** - MBID lookup (free, rate limited 1 req/s)
- **AcoustID API** - Fingerprint → MBID (requires API key)
- **AcousticBrainz API** - Musical flavor data (free)

### Documentation Dependencies

**Specifications Referenced:**
- **SPEC032** - Audio Ingest Architecture (source specification)
- **SPEC017** - Sample Rate Conversion (tick-based timing)
- **SPEC031** - Data-Driven Schema Maintenance (zero-config DB)
- **IMPL001** - Database Schema (tick fields verification)
- **SPEC008** - Library Management (integration context)

---

## Risks

### High-Risk Areas (Identified, Mitigated in Phase 7)

1. **Per-Segment Fingerprinting Performance**
   - Risk: Significantly slower than whole-file
   - Mitigation: Optimize PCM extraction, cache decoded audio

2. **Contextual Matching Accuracy**
   - Risk: Fails to narrow candidates effectively
   - Mitigation: Fuzzy matching, tune tolerances, extensive testing

3. **Evidence-Based Identification False Positives**
   - Risk: Accepts incorrect MBID matches
   - Mitigation: Conservative thresholds, validation, review queue

4. **Pipeline Reordering Regression**
   - Risk: Breaks existing functionality
   - Mitigation: Comprehensive testing, gradual rollout

### Medium-Risk Areas

1. **API Rate Limiting Complexity**
   - Risk: Hit rate limits, degrade performance
   - Mitigation: Implement robust rate limiter, respect limits

2. **Test Data Availability**
   - Risk: Cannot validate accuracy claims
   - Mitigation: Use developer library subset, document limitations

---

## Success Criteria

### Functional Success

1. ✅ Pipeline executes in correct sequence (segment → match → fingerprint → identify)
2. ✅ Pattern analyzer detects source media type with >80% accuracy
3. ✅ Contextual matcher narrows candidates to <10 results in >80% of cases
4. ✅ Confidence assessor achieves >90% acceptance rate for known-good files
5. ✅ Per-segment fingerprinting implemented and working
6. ✅ Tick-based timing conversion applied to all passage timing
7. ✅ Zero-config database initialization preserved

### Performance Success

1. ✅ Import throughput ≥20 files/second (4 workers, average 3-min songs)
2. ✅ Contextual matching <2 seconds per file
3. ✅ Per-segment fingerprinting overhead <20% vs. whole-file

### Quality Success

1. ✅ MBID identification accuracy >90% for known-good files
2. ✅ False positive rate <5% (incorrect MBID accepted)
3. ✅ Test coverage >80% for new components
4. ✅ All acceptance tests pass
5. ✅ No regressions in existing functionality

---

**END OF SCOPE STATEMENT**
