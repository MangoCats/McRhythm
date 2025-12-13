# Specification Issues: Single Song Import Improvements

**Plan:** PLAN026
**Analysis Date:** 2025-12-13
**Phase 2 Status:** Complete

---

## Issue Summary

| Severity | Count | Status |
|----------|-------|--------|
| CRITICAL | 0 | - |
| HIGH | 2 | Resolvable |
| MEDIUM | 3 | Noted |
| LOW | 2 | Tracked |
| **Total** | **7** | **Proceed** |

**Decision:** No CRITICAL issues. HIGH issues are resolvable during implementation. Proceed to Phase 3.

---

## CRITICAL Issues

**None identified.**

---

## HIGH Issues

### HIGH-001: Recording Query Duration Format Unspecified

**Affected:** SSI-MB-020
**Issue:** MusicBrainz recording search `dur:` field uses milliseconds, not seconds.
**Resolution:** Specify query format as `dur:[{ms_min} TO {ms_max}]` where tolerance is ±10,000ms.
**Resolved:** Update SSI-MB-020 to specify milliseconds.

### HIGH-002: Cache Key Strategy Undefined

**Affected:** SSI-INT-030
**Issue:** How should recording cache be keyed? Options:
- By (artist, title) tuple (may miss variations)
- By normalized (artist, title) (better matching)
- By query string (exact dedup)

**Resolution:** Use normalized (artist, title) as cache key. Normalization uses same logic as comparison.
**Resolved:** Implementation detail, documented.

---

## MEDIUM Issues

### MED-001: Confidence Threshold Alignment

**Affected:** SSI-VAL-030, SSI-FUS-030
**Issue:** Multiple threshold values across system:
- AcoustID high confidence: 0.80
- Proposed title threshold: 0.85
- Proposed artist threshold: 0.80
- Existing IdentityResolver min_confidence: 0.30

**Status:** Intentional variation per source type. Document in implementation.

### MED-002: IdentityResolver Source Types

**Affected:** SSI-FUS-020
**Issue:** IdentityResolver expects `IdentityExtraction` struct with `recording_mbid`, `confidence`, `source`. Need to ensure MusicBrainz recording search produces compatible output.

**Status:** Review IdentityExtraction struct during implementation. May need adapter.

### MED-003: Multiple Artist Handling

**Affected:** SSI-MB-020
**Issue:** Songs with multiple artists (e.g., "Artist A feat. Artist B") may not match MB recording with different artist credit format.

**Status:** Accept as limitation for Phase 1. Use best-match artist from ID3 `TPE1` tag.

---

## LOW Issues

### LOW-001: Unicode Edge Cases

**Affected:** SSI-VAL-020
**Issue:** am29 normalize_artist_name handles basic ASCII folding but may miss some Unicode characters.

**Status:** Existing implementation is adequate for most cases. Defer enhanced Unicode handling.

### LOW-002: Empty Metadata Fallback

**Affected:** SSI-MB-020
**Issue:** If ID3 has no artist or title, fallback to filename parsing is not specified.

**Status:** Out of scope for this plan. Existing MetadataExtractor handles filename parsing.

---

## Completeness Verification

| Requirement | Inputs | Outputs | Behavior | Errors | Dependencies |
|-------------|--------|---------|----------|--------|--------------|
| SSI-MB-010 | ✓ | ✓ | ✓ | ✓ | ✓ |
| SSI-MB-020 | ✓* | ✓ | ✓ | ✓ | ✓ |
| SSI-MB-030 | ✓ | ✓ | ✓ | ✓ | ✓ |
| SSI-VAL-010 | ✓ | ✓ | ✓ | ✓ | ✓ |
| SSI-VAL-020 | ✓ | ✓ | ✓ | ✓ | ✓ |
| SSI-VAL-030 | ✓ | ✓ | ✓ | ✓ | ✓ |
| SSI-FUS-010 | ✓ | ✓ | ✓ | ✓ | ✓ |
| SSI-FUS-020 | ✓* | ✓ | ✓ | ✓ | ✓ |
| SSI-FUS-030 | ✓ | ✓ | ✓ | ✓ | ✓ |
| SSI-INT-010 | ✓ | ✓ | ✓ | ✓ | ✓ |
| SSI-INT-020 | ✓ | ✓ | ✓ | ✓ | ✓ |
| SSI-INT-030 | ✓* | ✓ | ✓ | ✓ | ✓ |
| SSI-QUA-010 | ✓ | ✓ | ✓ | ✓ | ✓ |
| SSI-QUA-020 | ✓ | ✓ | ✓ | ✓ | ✓ |
| SSI-QUA-030 | ✓ | ✓ | ✓ | ✓ | ✓ |

Legend: ✓ = Complete, ✓* = Complete with HIGH issue resolved

---

## Existing Infrastructure Analysis

### Available for Reuse

| Component | Location | Reuse Strategy |
|-----------|----------|----------------|
| `MusicBrainzClient.search_recordings()` | `services/musicbrainz_client.rs:295` | Direct use |
| `MusicBrainzClient` rate limiting | `services/musicbrainz_client.rs` | Automatic |
| `IdentityResolver` | `fusion/identity_resolver.rs` | Direct use |
| `jaro_winkler()` | `strsim` crate | Already a dependency |
| `normalize_artist_name()` | `examples/am29/matching/validation.rs:227` | Port to services |
| `best_jaro_winkler_ratio()` | `examples/am29/matching/validation.rs:147` | Port to services |

### New Development Required

| Component | Purpose | Estimated Lines |
|-----------|---------|-----------------|
| `RecordingMatcher` | Query building, candidate scoring | ~200 |
| `string_utils.rs` | Ported normalization, Jaro-Winkler wrappers | ~150 |
| Cache table migration | `recording_cache` table | ~20 |
| Unit tests | Recording matcher, string utils | ~300 |
| Integration tests | Multi-source fusion | ~150 |

---

## Testability Verification

| Requirement | Testable | Method |
|-------------|----------|--------|
| SSI-MB-010 | ✓ | Mock MusicBrainz response, verify fallback triggers |
| SSI-MB-020 | ✓ | Verify query string format matches spec |
| SSI-MB-030 | ✓ | Check sorting by similarity score |
| SSI-VAL-010 | ✓ | Unit test similarity scores |
| SSI-VAL-020 | ✓ | Unit test normalization transformations |
| SSI-VAL-030 | ✓ | Unit test threshold application |
| SSI-FUS-010 | ✓ | Integration test with multiple sources |
| SSI-FUS-020 | ✓ | Verify all three sources combined |
| SSI-FUS-030 | ✓ | Verify Bayesian formula implementation |
| SSI-INT-010 | ✓ | Integration test with ContentTypeClassifier |
| SSI-INT-020 | ✓ | Verify request timing |
| SSI-INT-030 | ✓ | Verify cache hit/miss behavior |
| SSI-QUA-010 | ✓ | Coverage report |
| SSI-QUA-020 | ✓ | Integration test suite |
| SSI-QUA-030 | ✓ | Coverage measurement |

---

## Conclusion

**Phase 2 Status:** COMPLETE

**Findings:**
- 0 CRITICAL issues (proceed to implementation)
- 2 HIGH issues (resolvable with minor spec clarification)
- 3 MEDIUM issues (documented, acceptable for implementation)
- 2 LOW issues (tracked, defer handling)

**Significant Infrastructure Exists:**
- MusicBrainzClient.search_recordings() already implemented
- IdentityResolver ready for integration
- Jaro-Winkler available via strsim crate
- Only need to port validation functions from am29

**Recommendation:** Proceed to Phase 3 (Acceptance Test Definition)
