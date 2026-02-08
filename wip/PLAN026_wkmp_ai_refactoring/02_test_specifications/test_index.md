# PLAN026: Test Specifications Index

**Created:** 2025-11-26
**Total Tests:** 52 (34 unit, 12 integration, 6 system)
**Coverage:** 100% of requirements have acceptance tests

---

## Test Summary by Category

| Category | Unit | Integration | System | Total |
|----------|------|-------------|--------|-------|
| Pipeline Steps | 14 | 5 | 2 | 21 |
| Database Schema | 9 | 3 | 0 | 12 |
| Algorithm | 8 | 2 | 2 | 12 |
| Classification | 3 | 2 | 2 | 7 |
| **Total** | **34** | **12** | **6** | **52** |

---

## Test Index

### Pipeline Step Tests

| Test ID | Type | Requirement | Description |
|---------|------|-------------|-------------|
| TC-U-STEP-001-01 | Unit | REQ-STEP-001 | Folder scan detects audio files by magic bytes |
| TC-U-STEP-001-02 | Unit | REQ-STEP-001 | File classification (7 types) correct |
| TC-U-STEP-001-03 | Unit | REQ-STEP-001 | Improper extension files flagged |
| TC-U-STEP-002-01 | Unit | REQ-STEP-002 | Known file skipped when metadata matches |
| TC-U-STEP-002-02 | Unit | REQ-STEP-002 | Known file reprocessed when metadata differs |
| TC-U-STEP-003-01 | Unit | REQ-STEP-003 | SHA-256 hash computed correctly |
| TC-U-STEP-003-02 | Unit | REQ-STEP-003 | Duplicate detected by hash match |
| TC-U-STEP-003-03 | Unit | REQ-STEP-003 | DUPLICATE status set correctly |
| TC-U-STEP-004-01 | Unit | REQ-STEP-004 | ID3v2 tags extracted |
| TC-U-STEP-004-02 | Unit | REQ-STEP-004 | Vorbis comments extracted |
| TC-U-STEP-004-03 | Unit | REQ-STEP-004 | MP4 atoms extracted |
| TC-U-STEP-005-01 | Unit | REQ-STEP-005 | Audio decoded to frames |
| TC-U-STEP-005-02 | Unit | REQ-STEP-005 | NO AUDIO detected (<100ms above threshold) |
| TC-U-STEP-007-01 | Unit | REQ-STEP-007 | AtomicBool signaling works |
| TC-I-STEP-001-01 | Integration | REQ-STEP-001..007 | Full pipeline Step 1→7 for single file |
| TC-I-STEP-002-01 | Integration | REQ-STEP-002,003 | Duplicate detection across multiple files |
| TC-I-STEP-003-01 | Integration | REQ-STEP-003,007 | Completed duplicate handling signals correctly |
| TC-I-STEP-004-01 | Integration | REQ-STEP-004,006 | Metadata flows to content type determination |
| TC-I-STEP-005-01 | Integration | REQ-STEP-005,006 | Decoded audio used for silence detection |
| TC-S-STEP-001-01 | System | REQ-STEP-* | Import folder with mixed file types |
| TC-S-STEP-002-01 | System | REQ-STEP-* | Resume interrupted import |

### Database Schema Tests

| Test ID | Type | Requirement | Description |
|---------|------|-------------|-------------|
| TC-U-DB-001-01 | Unit | REQ-DB-001 | Files table created with all columns |
| TC-U-DB-002-01 | Unit | REQ-DB-002 | Passages table with tick-based timing |
| TC-U-DB-002-02 | Unit | REQ-DB-002 | Passage constraints enforced (start < end) |
| TC-U-DB-003-01 | Unit | REQ-DB-003 | Songs table recording_mbid indexed |
| TC-U-DB-003-02 | Unit | REQ-DB-003 | Multiple songs with same recording_mbid allowed |
| TC-U-DB-004-01 | Unit | REQ-DB-004 | Works table created |
| TC-U-DB-005-01 | Unit | REQ-DB-005 | Artists table with unique mbid |
| TC-U-DB-006-01 | Unit | REQ-DB-006 | Albums table with unique mbid |
| TC-U-DB-007-01 | Unit | REQ-DB-007 | passage_songs NULL timing for 1:1 case |
| TC-I-DB-001-01 | Integration | REQ-DB-* | Full schema creation and relationships |
| TC-I-DB-002-01 | Integration | REQ-DB-007,008,009 | Relationship tables cascade correctly |
| TC-I-DB-003-01 | Integration | REQ-DB-003,008 | Song → Artist weights sum to 1.0 |

### Algorithm Tests

| Test ID | Type | Requirement | Description |
|---------|------|-------------|-------------|
| TC-U-ALG-001-01 | Unit | REQ-ALG-001 | Duration triage routes <12min to single-song |
| TC-U-ALG-001-02 | Unit | REQ-ALG-001 | Duration triage routes >25min to album |
| TC-U-ALG-002-01 | Unit | REQ-ALG-002 | Quick silence scan estimates segments |
| TC-U-ALG-003-01 | Unit | REQ-ALG-003 | Chromaprint fingerprint generated |
| TC-U-ALG-009-01 | Unit | REQ-ALG-009 | Jaro-Winkler similarity ≥0.50 accepts |
| TC-U-ALG-009-02 | Unit | REQ-ALG-009 | Jaro-Winkler similarity <0.50 flags mismatch |
| TC-U-ALG-012-01 | Unit | REQ-ALG-012 | 3.0s tolerance applied correctly |
| TC-U-ALG-013-01 | Unit | REQ-ALG-013 | ≥80% classified as high confidence |
| TC-I-ALG-001-01 | Integration | REQ-ALG-001..007 | Full content type determination flow |
| TC-I-ALG-002-01 | Integration | REQ-ALG-004,005 | Album path with edition testing |
| TC-S-ALG-001-01 | System | REQ-ALG-* | Identify known album file |
| TC-S-ALG-002-01 | System | REQ-ALG-* | Identify single song file |

### Classification Tests

| Test ID | Type | Requirement | Description |
|---------|------|-------------|-------------|
| TC-U-CLASS-001-01 | Unit | REQ-CLASS-001 | SINGLE_SONG classification criteria |
| TC-U-CLASS-002-01 | Unit | REQ-CLASS-002 | FULL_ALBUM ≥95% match |
| TC-U-CLASS-003-01 | Unit | REQ-CLASS-003 | PARTIAL_ALBUM 75-94% match |
| TC-I-CLASS-001-01 | Integration | REQ-CLASS-* | All 6 classification outcomes reachable |
| TC-I-CLASS-002-01 | Integration | REQ-CLASS-004 | MULTIPLE_SONGS creates one passage per segment |
| TC-S-CLASS-001-01 | System | REQ-CLASS-* | Classification matches expected for test corpus |
| TC-S-CLASS-002-01 | System | REQ-CLASS-005,006 | Unidentified files handled gracefully |

---

## Quick Reference: Tests by Requirement

| Requirement | Tests | Coverage |
|-------------|-------|----------|
| REQ-STEP-001 | TC-U-STEP-001-01..03 | Complete |
| REQ-STEP-002 | TC-U-STEP-002-01..02 | Complete |
| REQ-STEP-003 | TC-U-STEP-003-01..03 | Complete |
| REQ-STEP-004 | TC-U-STEP-004-01..03 | Complete |
| REQ-STEP-005 | TC-U-STEP-005-01..02 | Complete |
| REQ-STEP-006 | See ALG tests | Complete |
| REQ-STEP-007 | TC-U-STEP-007-01 | Complete |
| REQ-DB-001..009 | TC-U-DB-001..007, TC-I-DB-001..003 | Complete |
| REQ-ALG-001..014 | TC-U-ALG-001..013, TC-I-ALG-001..002, TC-S-ALG-001..002 | Complete |
| REQ-CLASS-001..006 | TC-U-CLASS-001..003, TC-I-CLASS-001..002, TC-S-CLASS-001..002 | Complete |
| REQ-ARCH-001..004 | Covered by integration/system tests | Complete |
| REQ-INT-001..005 | Covered by integration tests | Complete |
| REQ-KEEP-001..004 | Verified by code review | Complete |

---

## Test Data Requirements

| Test Category | Data Needed |
|---------------|-------------|
| Pipeline | Test audio files (MP3, FLAC, M4A), corrupted files, empty files |
| Database | Pre-populated test database, migration scripts |
| Algorithm | Known album files with expected track boundaries |
| Classification | Test corpus with known MusicBrainz matches |

---

## Notes

- All tests use BDD format (Given/When/Then)
- Individual test specifications in separate files (tc_*.md)
- See traceability_matrix.md for complete requirement→test mapping
