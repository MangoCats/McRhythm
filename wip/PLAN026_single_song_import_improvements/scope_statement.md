# Scope Statement: Single Song Import Improvements

**Plan:** PLAN026
**Created:** 2025-12-13

---

## Problem Statement

The wkmp-ai single-song import path is under-developed compared to the album import path:
- Album path: 5-stage pipeline with 180-parameter optimization, edition scoring, MBID selection
- Single-song path: Basic AcoustID lookup with 0.80 threshold, no fallback

**Impact:** Many legitimate songs fail import when AcoustID is unavailable, times out, or returns low confidence.

---

## In Scope

### Phase 1: Core Improvements (P0 Requirements)

1. **MusicBrainz Recording Search Fallback**
   - New `RecordingMatcher` service
   - Query by artist + title + duration
   - Jaro-Winkler similarity scoring
   - Integration with ContentTypeClassifier

2. **Metadata Validation Upgrade**
   - Port Jaro-Winkler from am29/matching/validation.rs
   - String normalization (lowercase, strip articles, punctuation)
   - Replace Levenshtein thresholds

3. **Rate Limit Compliance**
   - Reuse existing MusicBrainzClient rate limiter
   - Maintain 1 req/sec for MB, 3 req/sec for AcoustID

4. **Unit Tests**
   - Recording matcher query building
   - String comparison functions
   - Candidate scoring

### Phase 2: Enhanced Integration (P1 Requirements)

5. **IdentityResolver Integration**
   - Multi-source Bayesian fusion
   - Combine AcoustID + MB + ID3 sources
   - Posterior probability calculation

6. **Recording Cache**
   - Cache MB recording lookups
   - Prevent redundant API calls

7. **Integration Tests**
   - Multi-source fusion workflow
   - Fallback scenarios
   - End-to-end classification

---

## Out of Scope

### Deferred to Future Plan

1. **Folder Context Awareness (P2)**
   - SSI-CTX-010, SSI-CTX-020
   - Requires album import integration
   - Deferred: Adds complexity, optional enhancement

2. **Per-Segment Fingerprinting (P2)**
   - SSI-SEG-010, SSI-SEG-020
   - Larger architectural change (PLAN025 Phase 3)
   - Deferred: Separate plan scope

3. **ML-based Classification**
   - Mentioned in am29 future enhancements
   - Requires training data and model development
   - Deferred: Research-level effort

---

## Assumptions

| ID | Assumption | Impact if False |
|----|------------|-----------------|
| A1 | MusicBrainz /ws/2/recording API is stable | Need alternative query approach |
| A2 | Jaro-Winkler similarity is adequate for music metadata | May need phonetic matching |
| A3 | Existing IdentityResolver is production-ready | May need additional testing |
| A4 | Rate limits (1 req/sec MB) are acceptable for batch import | May need parallel strategies |
| A5 | Most single songs have artist+title in metadata | May need filename parsing enhancement |

---

## Constraints

### Technical Constraints

- **Rate Limits:** MusicBrainz API limits to 1 request/second
- **AcoustID Timeout:** Current 1-second timeout (PLAN031) remains
- **Database Schema:** No schema changes required (use existing tables)
- **Architecture:** Must integrate with existing ContentTypeClassifier

### Process Constraints

- **Backward Compatibility:** Existing album import path unchanged
- **No Breaking Changes:** Existing API contracts preserved
- **Test Coverage:** ≥80% for new code

---

## Dependencies

### Existing Code (Reuse)

| Component | Location | Purpose |
|-----------|----------|---------|
| MusicBrainzClient | `services/musicbrainz_client.rs` | HTTP client, rate limiting |
| IdentityResolver | `fusion/identity_resolver.rs` | Bayesian MBID fusion |
| ContentTypeClassifier | `services/content_type_classifier.rs` | Integration point |
| Jaro-Winkler impl | `examples/am29/matching/validation.rs` | Port to services |
| AcoustIDClient | `services/acoustid_client.rs` | Existing fingerprint lookup |

### External Dependencies

| Dependency | Version | Purpose |
|------------|---------|---------|
| strsim | 0.11+ | Jaro-Winkler implementation |
| reqwest | existing | HTTP client |
| tokio | existing | Async runtime |
| sqlx | existing | Database caching |

---

## Success Criteria

### Quantitative

- [ ] 100% of P0 requirements implemented
- [ ] 100% of P1 requirements implemented
- [ ] ≥80% test coverage on new code
- [ ] All 20 acceptance tests pass

### Qualitative

- [ ] Single songs import successfully when AcoustID fails but MB has recording
- [ ] No regression in album import functionality
- [ ] Integration is transparent to existing import workflow
- [ ] Code follows existing wkmp-ai patterns

---

## Risk Summary

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| MusicBrainz rate limiting delays import | Medium | Medium | Batch queries, cache aggressively |
| Jaro-Winkler false positives | Low | Medium | Require both artist+title match |
| IdentityResolver edge cases | Low | Low | Comprehensive unit tests |
| AcoustID circuit breaker interactions | Medium | Low | Test fallback thoroughly |

---

## Estimated Effort

| Phase | Components | Estimate |
|-------|------------|----------|
| Phase 1 (P0) | RecordingMatcher, Jaro-Winkler, Unit tests | 8-12 hours |
| Phase 2 (P1) | IdentityResolver integration, Cache, Integration tests | 6-8 hours |
| **Total** | | **14-20 hours** |
