# PLAN030: Test Index

## Summary

| Category | Tests | Coverage |
|----------|-------|----------|
| Unit Tests | 45 | All 12 functional requirements |
| Integration Tests | 12 | Cross-component verification |
| System Tests | 3 | End-to-end album matching |
| **Total** | **60** | **100%** |

---

## Test Index by Increment

### Increment 1: Types Migration

| Test ID | Type | Description | Requirement |
|---------|------|-------------|-------------|
| TC-U-001-01 | Unit | Edition type serialization roundtrip | REQ-AM-004 |
| TC-U-001-02 | Unit | CacheConfig validation | REQ-AM-011 |
| TC-U-001-03 | Unit | MBSearchResponse parsing | REQ-AM-003 |
| TC-U-001-04 | Unit | CandidateTestResult construction | REQ-AM-005 |

### Increment 2: Constants & Configuration

| Test ID | Type | Description | Requirement |
|---------|------|-------------|-------------|
| TC-U-002-01 | Unit | Default threshold values in valid range | REQ-AM-005 |
| TC-U-002-02 | Unit | Parameter grid generates 180 combinations | REQ-AM-005 |
| TC-U-002-03 | Unit | AlbumMatcherConfig builder pattern | REQ-NF-002 |

### Increment 3: Metadata Extraction

| Test ID | Type | Description | Requirement |
|---------|------|-------------|-------------|
| TC-U-003-01 | Unit | ID3 tag extraction from MP3 | REQ-AM-002 |
| TC-U-003-02 | Unit | Filename parsing (artist - album) | REQ-AM-002 |
| TC-U-003-03 | Unit | Metadata reconciliation priority | REQ-AM-002 |
| TC-U-003-04 | Unit | Unicode handling in metadata | REQ-AM-002 |

### Increment 4: Silence Detection

| Test ID | Type | Description | Requirement |
|---------|------|-------------|-------------|
| TC-U-004-01 | Unit | Silence cache generation | REQ-AM-005 |
| TC-U-004-02 | Unit | Track boundary detection from silence | REQ-AM-005 |
| TC-U-004-03 | Unit | Parameter grid iteration | REQ-AM-005 |
| TC-U-004-04 | Unit | RMS calculation for audio samples | REQ-AM-007 |
| TC-I-004-01 | Integration | Silence detection with decoded audio | REQ-AM-001, REQ-AM-005 |

### Increment 5: Single-Track Discriminator

| Test ID | Type | Description | Requirement |
|---------|------|-------------|-------------|
| TC-U-005-01 | Unit | Pre-decode analysis (file size heuristic) | REQ-AM-012 |
| TC-U-005-02 | Unit | Post-decode analysis (silence count) | REQ-AM-012 |
| TC-U-005-03 | Unit | Single-track rejection decision | REQ-AM-012 |
| TC-U-005-04 | Unit | Album acceptance decision | REQ-AM-012 |

### Increment 6: Edition Grouping

| Test ID | Type | Description | Requirement |
|---------|------|-------------|-------------|
| TC-U-006-01 | Unit | Group releases by track count | REQ-AM-004 |
| TC-U-006-02 | Unit | Group releases by duration pattern | REQ-AM-004 |
| TC-U-006-03 | Unit | Edition filtering by relevance | REQ-AM-004 |
| TC-U-006-04 | Unit | Edition sorting by name distance | REQ-AM-004 |
| TC-U-006-05 | Unit | Handle multi-disc releases | REQ-AM-004 |

### Increment 7: Stage 2 - Parameter Optimization

| Test ID | Type | Description | Requirement |
|---------|------|-------------|-------------|
| TC-U-007-01 | Unit | Grid search iteration (180 combos) | REQ-AM-005 |
| TC-U-007-02 | Unit | Match percentage calculation | REQ-AM-005 |
| TC-U-007-03 | Unit | Best parameter selection | REQ-AM-005 |
| TC-U-007-04 | Unit | Over-segmented candidate collection | REQ-AM-005, REQ-AM-006 |
| TC-U-007-05 | Unit | Early-exit on 100% match | REQ-AM-010 |

### Increment 8: Stage 3 - Over-segmentation Assembly

| Test ID | Type | Description | Requirement |
|---------|------|-------------|-------------|
| TC-U-008-01 | Unit | DP optimal segment merging | REQ-AM-006 |
| TC-U-008-02 | Unit | Segment combination scoring | REQ-AM-006 |
| TC-U-008-03 | Unit | Assembly improves match percentage | REQ-AM-006 |
| TC-U-008-04 | Unit | Handle no-improvement case | REQ-AM-006 |

### Increment 9: Stage 4 - Quiet Spot Detection

| Test ID | Type | Description | Requirement |
|---------|------|-------------|-------------|
| TC-U-009-01 | Unit | RMS profile calculation | REQ-AM-007 |
| TC-U-009-02 | Unit | Quiet spot identification | REQ-AM-007 |
| TC-U-009-03 | Unit | 25% penalty application | REQ-AM-007 |
| TC-U-009-04 | Unit | Penalized result cannot be 100% | REQ-AM-007 |

### Increment 10: Stage 5 - Extra Track Merging

| Test ID | Type | Description | Requirement |
|---------|------|-------------|-------------|
| TC-U-010-01 | Unit | Identify extra tracks | REQ-AM-008 |
| TC-U-010-02 | Unit | Adjacent track merging | REQ-AM-008 |
| TC-U-010-03 | Unit | Maintain 100% match after merge | REQ-AM-008 |

### Increment 11: Orchestration

| Test ID | Type | Description | Requirement |
|---------|------|-------------|-------------|
| TC-U-011-01 | Unit | Stage sequencing (2→3→4→5) | All stages |
| TC-U-011-02 | Unit | Early-exit signal propagation | REQ-AM-010 |
| TC-U-011-03 | Unit | Grace period timing (20s) | REQ-AM-010 |
| TC-U-011-04 | Unit | Best result selection across stages | All stages |
| TC-I-011-01 | Integration | Full edition test through all stages | All stages |

### Increment 12: MusicBrainz Integration

| Test ID | Type | Description | Requirement |
|---------|------|-------------|-------------|
| TC-U-012-01 | Unit | Comprehensive search query building | REQ-AM-003 |
| TC-U-012-02 | Unit | Cache hit returns cached response | REQ-AM-011 |
| TC-U-012-03 | Unit | Cache miss queries API and stores | REQ-AM-011 |
| TC-U-012-04 | Unit | Rate limiting compliance | REQ-AM-003 |
| TC-I-012-01 | Integration | Search + release details fetch | REQ-AM-003 |

### Increment 13: AlbumMatcher Implementation

| Test ID | Type | Description | Requirement |
|---------|------|-------------|-------------|
| TC-U-013-01 | Unit | match_album returns AlbumMatchResult | REQ-NF-002 |
| TC-U-013-02 | Unit | Artist verification integration | REQ-AM-009 |
| TC-U-013-03 | Unit | Jaro-Winkler similarity calculation | REQ-AM-009 |
| TC-I-013-01 | Integration | Full pipeline with mocked MB | All requirements |
| TC-I-013-02 | Integration | Error handling for failed decode | REQ-AM-001 |

### Increment 14: Integration Tests

| Test ID | Type | Description | Requirement |
|---------|------|-------------|-------------|
| TC-I-014-01 | Integration | Album with 10 tracks matches | All requirements |
| TC-I-014-02 | Integration | Album with hidden track matches | REQ-AM-008 |
| TC-I-014-03 | Integration | Single-track file rejected | REQ-AM-012 |
| TC-I-014-04 | Integration | No MusicBrainz match returns error | REQ-AM-003 |
| TC-S-014-01 | System | End-to-end with real audio file | All requirements |

### Increment 15: Documentation & Cleanup

| Test ID | Type | Description | Requirement |
|---------|------|-------------|-------------|
| TC-S-015-01 | System | API documentation complete | REQ-NF-002 |
| TC-S-015-02 | System | No CLI dependencies in library | REQ-NF-002 |

---

## Traceability Matrix

| Requirement | Unit Tests | Integration Tests | System Tests | Coverage |
|-------------|------------|-------------------|--------------|----------|
| REQ-AM-001 | - | TC-I-004-01, TC-I-013-02 | TC-S-014-01 | Complete |
| REQ-AM-002 | TC-U-003-01..04 | - | TC-S-014-01 | Complete |
| REQ-AM-003 | TC-U-012-01..04 | TC-I-012-01, TC-I-014-04 | TC-S-014-01 | Complete |
| REQ-AM-004 | TC-U-006-01..05 | - | TC-S-014-01 | Complete |
| REQ-AM-005 | TC-U-004-01..03, TC-U-007-01..05 | TC-I-004-01 | TC-S-014-01 | Complete |
| REQ-AM-006 | TC-U-008-01..04 | TC-I-011-01 | TC-S-014-01 | Complete |
| REQ-AM-007 | TC-U-004-04, TC-U-009-01..04 | TC-I-011-01 | TC-S-014-01 | Complete |
| REQ-AM-008 | TC-U-010-01..03 | TC-I-014-02 | TC-S-014-01 | Complete |
| REQ-AM-009 | TC-U-013-02..03 | TC-I-013-01 | TC-S-014-01 | Complete |
| REQ-AM-010 | TC-U-007-05, TC-U-011-02..03 | TC-I-011-01 | TC-S-014-01 | Complete |
| REQ-AM-011 | TC-U-001-02, TC-U-012-02..03 | TC-I-012-01 | TC-S-014-01 | Complete |
| REQ-AM-012 | TC-U-005-01..04 | TC-I-014-03 | TC-S-014-01 | Complete |
| REQ-NF-001 | - | TC-I-013-01 | TC-S-014-01 | Complete |
| REQ-NF-002 | TC-U-002-03, TC-U-013-01 | - | TC-S-015-01..02 | Complete |
| REQ-NF-003 | - | TC-I-012-01, TC-I-013-01 | TC-S-014-01 | Complete |
| REQ-NF-004 | - | TC-I-004-01 | TC-S-014-01 | Complete |

**Coverage:** 100% - All 16 requirements have acceptance tests
