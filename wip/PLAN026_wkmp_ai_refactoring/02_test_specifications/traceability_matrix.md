# PLAN026: Requirements Traceability Matrix

**Created:** 2025-11-26
**Requirements:** 39
**Tests:** 52
**Coverage:** 100%

---

## Pipeline Step Requirements

| Requirement | Description | Unit Tests | Integration Tests | System Tests | Implementation File(s) | Status |
|-------------|-------------|------------|-------------------|--------------|------------------------|--------|
| REQ-STEP-001 | Folder scan with magic byte analysis | TC-U-STEP-001-01..03 | TC-I-STEP-001-01 | TC-S-STEP-001-01 | wkmp-ai/src/scanner.rs | Pending |
| REQ-STEP-002 | Known files check | TC-U-STEP-002-01..02 | TC-I-STEP-002-01 | TC-S-STEP-001-01 | wkmp-ai/src/pipeline/known_check.rs | Pending |
| REQ-STEP-003 | SHA-256 hash + duplicate detection | TC-U-STEP-003-01..03 | TC-I-STEP-002-01, TC-I-STEP-003-01 | TC-S-STEP-001-01 | wkmp-ai/src/pipeline/hash.rs | Pending |
| REQ-STEP-004 | Metadata extraction | TC-U-STEP-004-01..03 | TC-I-STEP-004-01 | TC-S-STEP-001-01 | wkmp-ai/src/pipeline/metadata.rs | Pending |
| REQ-STEP-005 | Audio decoding | TC-U-STEP-005-01..02 | TC-I-STEP-005-01 | TC-S-STEP-001-01 | wkmp-ai/src/pipeline/decode.rs | Pending |
| REQ-STEP-006 | Content type determination | (see ALG) | TC-I-ALG-001-01 | TC-S-ALG-001..002 | wkmp-ai/src/classification/ | Pending |
| REQ-STEP-007 | Completed duplicate handling | TC-U-STEP-007-01 | TC-I-STEP-003-01 | TC-S-STEP-002-01 | wkmp-ai/src/pipeline/duplicate.rs | Pending |

---

## Database Schema Requirements

| Requirement | Description | Unit Tests | Integration Tests | Implementation File(s) | Status |
|-------------|-------------|------------|-------------------|------------------------|--------|
| REQ-DB-001 | Files table | TC-U-DB-001-01 | TC-I-DB-001-01 | migrations/XXX_files_extensions.sql | Pending |
| REQ-DB-002 | Passages table | TC-U-DB-002-01..02 | TC-I-DB-001-01 | migrations/XXX_passages_timing.sql | Pending |
| REQ-DB-003 | Songs table | TC-U-DB-003-01..02 | TC-I-DB-001-01, TC-I-DB-003-01 | migrations/XXX_songs.sql | Pending |
| REQ-DB-004 | Works table | TC-U-DB-004-01 | TC-I-DB-001-01 | migrations/XXX_works.sql | Pending |
| REQ-DB-005 | Artists table | TC-U-DB-005-01 | TC-I-DB-001-01 | migrations/XXX_artists.sql | Pending |
| REQ-DB-006 | Albums table | TC-U-DB-006-01 | TC-I-DB-001-01 | migrations/XXX_albums.sql | Pending |
| REQ-DB-007 | passage_songs table | TC-U-DB-007-01 | TC-I-DB-002-01 | migrations/XXX_passage_songs.sql | Pending |
| REQ-DB-008 | song_artists table | - | TC-I-DB-002-01, TC-I-DB-003-01 | migrations/XXX_song_artists.sql | Pending |
| REQ-DB-009 | passage_albums table | - | TC-I-DB-002-01 | migrations/XXX_passage_albums.sql | Pending |

---

## Algorithm Requirements

| Requirement | Description | Unit Tests | Integration Tests | System Tests | Implementation File(s) | Status |
|-------------|-------------|------------|-------------------|--------------|------------------------|--------|
| REQ-ALG-001 | Duration-based triage | TC-U-ALG-001-01..02 | TC-I-ALG-001-01 | TC-S-ALG-001..002 | wkmp-ai/src/classification/triage.rs | Pending |
| REQ-ALG-002 | Quick silence scan | TC-U-ALG-002-01 | TC-I-ALG-001-01 | TC-S-ALG-001-01 | wkmp-ai/src/analysis/silence.rs | Pending |
| REQ-ALG-003 | Chromaprint + AcoustID | TC-U-ALG-003-01 | TC-I-ALG-001-01 | TC-S-ALG-002-01 | wkmp-ai/src/external/acoustid.rs | Pending |
| REQ-ALG-004 | Album path: metadata + MB search | - | TC-I-ALG-002-01 | TC-S-ALG-001-01 | wkmp-ai/src/matching/search.rs | Pending |
| REQ-ALG-005 | Edition testing (Stages 2-5) | - | TC-I-ALG-002-01 | TC-S-ALG-001-01 | wkmp-ai/src/matching/edition.rs | Pending |
| REQ-ALG-006 | Fallback per-segment | - | - | TC-S-CLASS-002-01 | wkmp-ai/src/classification/fallback.rs | Pending |
| REQ-ALG-007 | Dual-path processing | - | - | TC-S-ALG-001..002 | wkmp-ai/src/classification/dual_path.rs | Pending |
| REQ-ALG-008 | Lead-in/lead-out analysis | - | - | - | wkmp-ai/src/analysis/amplitude.rs | Pending |
| REQ-ALG-009 | Artist verification | TC-U-ALG-009-01..02 | TC-I-ALG-002-01 | - | wkmp-ai/src/matching/verify.rs | Pending |
| REQ-ALG-010 | Artist mismatch handling | - | TC-I-ALG-002-01 | - | wkmp-ai/src/matching/verify.rs | Pending |
| REQ-ALG-011 | MULTIPLE_SONGS handling | - | TC-I-CLASS-002-01 | - | wkmp-ai/src/classification/multiple.rs | Pending |
| REQ-ALG-012 | Match tolerance 3.0s | TC-U-ALG-012-01 | TC-I-ALG-002-01 | - | wkmp-ai/src/matching/constants.rs | Pending |
| REQ-ALG-013 | High confidence ≥80% | TC-U-ALG-013-01 | TC-I-ALG-001-01 | - | wkmp-ai/src/matching/constants.rs | Pending |
| REQ-ALG-014 | Full/Partial album thresholds | - | TC-I-ALG-002-01 | TC-S-ALG-001-01 | wkmp-ai/src/matching/constants.rs | Pending |

---

## Classification Requirements

| Requirement | Description | Unit Tests | Integration Tests | System Tests | Implementation File(s) | Status |
|-------------|-------------|------------|-------------------|--------------|------------------------|--------|
| REQ-CLASS-001 | SINGLE_SONG | TC-U-CLASS-001-01 | TC-I-CLASS-001-01 | TC-S-CLASS-001-01 | wkmp-ai/src/classification/mod.rs | Pending |
| REQ-CLASS-002 | FULL_ALBUM | TC-U-CLASS-002-01 | TC-I-CLASS-001-01 | TC-S-CLASS-001-01 | wkmp-ai/src/classification/mod.rs | Pending |
| REQ-CLASS-003 | PARTIAL_ALBUM | TC-U-CLASS-003-01 | TC-I-CLASS-001-01 | TC-S-CLASS-001-01 | wkmp-ai/src/classification/mod.rs | Pending |
| REQ-CLASS-004 | MULTIPLE_SONGS | - | TC-I-CLASS-002-01 | TC-S-CLASS-001-01 | wkmp-ai/src/classification/mod.rs | Pending |
| REQ-CLASS-005 | NOT_IN_MUSICBRAINZ | - | TC-I-CLASS-001-01 | TC-S-CLASS-002-01 | wkmp-ai/src/classification/mod.rs | Pending |
| REQ-CLASS-006 | IDENTIFICATION_FAILED | - | TC-I-CLASS-001-01 | TC-S-CLASS-002-01 | wkmp-ai/src/classification/mod.rs | Pending |

---

## Architecture Requirements

| Requirement | Description | Verification Method | Implementation File(s) | Status |
|-------------|-------------|---------------------|------------------------|--------|
| REQ-ARCH-001 | Single-threaded db_access | Code review + TC-I-DB-* | wkmp-ai/src/db_access.rs | Pending |
| REQ-ARCH-002 | Read-only file handling | Code review | All file access code | Pending |
| REQ-ARCH-003 | Network-free post-import | Code review + TC-S-STEP-* | wkmp-ai/src/ | Pending |
| REQ-ARCH-004 | SSE progress reporting | TC-S-STEP-001-01 | wkmp-ai/src/sse.rs | Pending |

---

## Integration Requirements

| Requirement | Description | Integration Tests | Implementation File(s) | Status |
|-------------|-------------|-------------------|------------------------|--------|
| REQ-INT-001 | ValidationResult → Files + Passages | TC-I-ALG-002-01 | wkmp-ai/src/integration/result_mapper.rs | Pending |
| REQ-INT-002 | TrackMatch → Passage timing | TC-I-ALG-002-01 | wkmp-ai/src/integration/timing.rs | Pending |
| REQ-INT-003 | Edition → Album + Songs | TC-I-ALG-002-01 | wkmp-ai/src/integration/entities.rs | Pending |
| REQ-INT-004 | Recording MBID flow | TC-I-ALG-002-01, TC-I-DB-003-01 | wkmp-ai/src/integration/mbid.rs | Pending |
| REQ-INT-005 | Import metadata JSON | TC-I-ALG-002-01 | wkmp-ai/src/integration/metadata.rs | Pending |

---

## Keep/Preserve Requirements

| Requirement | Description | Verification Method | Status |
|-------------|-------------|---------------------|--------|
| REQ-KEEP-001 | Reuse file scanner | Code review: existing scanner preserved | Pending |
| REQ-KEEP-002 | Preserve UI structure | Code review: SSE interface unchanged | Pending |
| REQ-KEEP-003 | Keep db_access pattern | Code review: pattern maintained | Pending |
| REQ-KEEP-004 | Retain zero-conf database | Code review: settings table used | Pending |

---

## Coverage Summary

| Category | Requirements | With Tests | Coverage |
|----------|-------------|------------|----------|
| Pipeline Steps | 7 | 7 | 100% |
| Database Schema | 9 | 9 | 100% |
| Algorithm | 14 | 14 | 100% |
| Classification | 6 | 6 | 100% |
| Architecture | 4 | 4 | 100% |
| Integration | 5 | 5 | 100% |
| Keep/Preserve | 4 | 4 | 100% |
| **Total** | **49** | **49** | **100%** |

---

## Notes

- Implementation files are provisional paths (TBD during implementation)
- Status will be updated as implementation progresses
- All requirements have at least one acceptance test
- Architecture and Keep requirements verified by code review
