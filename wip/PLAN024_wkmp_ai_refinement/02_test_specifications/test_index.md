# Test Index: PLAN024 - wkmp-ai Refinement

**Plan:** PLAN024 - wkmp-ai Refinement Implementation
**Total Tests:** 78 (42 unit, 24 integration, 12 system)
**Coverage:** 100% (all 26 requirements have acceptance tests)
**Date:** 2025-11-12

---

## Quick Reference

**Test ID Format:** TC-{Type}-{ReqNum}-{SeqNum}
- **Type:** U (Unit), I (Integration), S (System), M (Manual)
- **ReqNum:** Requirement number (001-021, NF001-NF005)
- **SeqNum:** Sequential test number (01, 02, ...)

**Example:** TC-U-007-01 = Unit Test for REQ-SPEC032-007, test #1

---

## Test Summary by Requirement

| Req ID | Requirement | Unit | Integration | System | Total | Status |
|--------|-------------|------|-------------|--------|-------|--------|
| REQ-SPEC032-001 | Scope Definition (SPEC032 doc) | 0 | 0 | 1 | 1 | Defined |
| REQ-SPEC032-002 | Two-Stage Roadmap (SPEC032 doc) | 0 | 0 | 1 | 1 | Defined |
| REQ-SPEC032-003 | Five-Step Workflow (SPEC032 doc) | 0 | 0 | 1 | 1 | Defined |
| REQ-SPEC032-004 | AcoustID API Key Validation | 4 | 1 | 1 | 6 | Defined |
| REQ-SPEC032-005 | Folder Selection | 3 | 0 | 1 | 4 | Defined |
| REQ-SPEC032-006 | Ten-Phase Pipeline (SPEC032 doc) | 0 | 0 | 1 | 1 | Defined |
| REQ-SPEC032-007 | Filename Matching Logic | 3 | 1 | 0 | 4 | Defined |
| REQ-SPEC032-008 | Hash-Based Duplicate Detection | 5 | 2 | 1 | 8 | Defined |
| REQ-SPEC032-009 | Metadata Extraction Merging | 4 | 1 | 0 | 5 | Defined |
| REQ-SPEC032-010 | Silence-Based Segmentation | 4 | 1 | 1 | 6 | Defined |
| REQ-SPEC032-011 | Fingerprinting Per Passage | 3 | 2 | 0 | 5 | Defined |
| REQ-SPEC032-012 | Song Matching with Confidence | 5 | 2 | 1 | 8 | Defined |
| REQ-SPEC032-013 | Passage Recording | 3 | 2 | 0 | 5 | Defined |
| REQ-SPEC032-014 | Amplitude-Based Lead-In/Lead-Out | 4 | 1 | 1 | 6 | Defined |
| REQ-SPEC032-015 | Musical Flavor Retrieval | 4 | 3 | 1 | 8 | Defined |
| REQ-SPEC032-016 | File Completion | 2 | 1 | 0 | 3 | Defined |
| REQ-SPEC032-017 | Session Completion | 2 | 1 | 0 | 3 | Defined |
| REQ-SPEC032-018 | Database Settings Table | 4 | 1 | 0 | 5 | Defined |
| REQ-SPEC032-019 | Thread Count Auto-Initialization | 3 | 1 | 0 | 4 | Defined |
| REQ-SPEC032-020 | Thirteen UI Progress Sections | 0 | 1 | 1 | 2 | Defined |
| REQ-SPEC032-021 | Status Field Enumerations | 3 | 1 | 0 | 4 | Defined |
| REQ-SPEC032-NF-001 | Parallel Processing | 0 | 1 | 1 | 2 | Defined |
| REQ-SPEC032-NF-002 | Real-Time Progress Updates | 0 | 1 | 1 | 2 | Defined |
| REQ-SPEC032-NF-003 | Sample-Accurate Timing | 2 | 0 | 0 | 2 | Defined |
| REQ-SPEC032-NF-004 | Symlink/Junction Handling | 2 | 0 | 1 | 3 | Defined |
| REQ-SPEC032-NF-005 | Metadata Preservation | 2 | 1 | 0 | 3 | Defined |
| **TOTALS** | **26 requirements** | **42** | **24** | **12** | **78** | **100%** |

---

## Unit Tests (42)

### REQ-SPEC032-004: AcoustID API Key Validation
- TC-U-004-01: Valid API key → Continue silently (DEBUG log only)
- TC-U-004-02: Invalid API key → Prompt user for valid key
- TC-U-004-03: Invalid API key acknowledged → Remember choice for session
- TC-U-004-04: Next session after invalid → Re-prompt user

### REQ-SPEC032-005: Folder Selection
- TC-U-005-01: Root folder selected → Accept
- TC-U-005-02: Subfolder under root selected → Accept
- TC-U-005-03: External folder selected (Stage One) → Reject with error

### REQ-SPEC032-007: Filename Matching Logic
- TC-U-007-01: Matching filename with 'INGEST COMPLETE' → Skip file
- TC-U-007-02: Matching filename without 'INGEST COMPLETE' → Reuse fileId
- TC-U-007-03: No matching filename → Assign new fileId

### REQ-SPEC032-008: Hash-Based Duplicate Detection
- TC-U-008-01: Hash matches 'INGEST COMPLETE' file → Mark 'DUPLICATE HASH'
- TC-U-008-02: Hash matches 'PENDING' file → Continue processing
- TC-U-008-03: No hash match → Continue processing
- TC-U-008-04: Bidirectional matching_hashes update (File A ↔ File B)
- TC-U-008-05: Matching fileId at front of matching_hashes list

### REQ-SPEC032-009: Metadata Extraction Merging
- TC-U-009-01: New metadata overwrites existing key
- TC-U-009-02: New NULL metadata preserves existing key (per ISSUE-HIGH-001 resolution)
- TC-U-009-03: Omitted key in new metadata preserves existing key
- TC-U-009-04: Completely new metadata file → All keys populated

### REQ-SPEC032-010: Silence-Based Segmentation
- TC-U-010-01: Audio below threshold for min duration → Detected as silence
- TC-U-010-02: Audio below threshold for < min duration → NOT silence
- TC-U-010-03: File with <100ms non-silence → Mark 'NO AUDIO'
- TC-U-010-04: Read thresholds from settings table (silence_threshold_dB, silence_min_duration_ticks)

### REQ-SPEC032-011: Fingerprinting Per Passage
- TC-U-011-01: Chromaprint fingerprint generated per potential passage
- TC-U-011-02: AcoustID API called with fingerprint → Returns MBID candidates
- TC-U-011-03: Invalid API key acknowledged → Skip AcoustID call

### REQ-SPEC032-012: Song Matching with Confidence
- TC-U-012-01: High confidence match (≥0.85) → Assign MBID
- TC-U-012-02: Medium confidence match (0.65-0.84) → Assign MBID
- TC-U-012-03: Low confidence match (0.45-0.64) → Assign MBID
- TC-U-012-04: No confidence match (<0.45) → Create zero-song passage
- TC-U-012-05: Adjacent passage merging (per ISSUE-HIGH-002 resolution - if implemented)

### REQ-SPEC032-013: Passage Recording
- TC-U-013-01: Create passage row with start/end times (SPEC017 ticks)
- TC-U-013-02: Create new song entry for novel MBID
- TC-U-013-03: Add passageId to existing song entry

### REQ-SPEC032-014: Amplitude-Based Lead-In/Lead-Out
- TC-U-014-01: Lead-in detected when amplitude exceeds threshold
- TC-U-014-02: Lead-out detected when amplitude exceeds threshold
- TC-U-014-03: Lead-in capped at 25% of passage if threshold never exceeded
- TC-U-014-04: Fade-in and fade-out fields remain NULL

### REQ-SPEC032-015: Musical Flavor Retrieval
- TC-U-015-01: AcousticBrainz retrieval successful → Store JSON, mark 'FLAVOR READY'
- TC-U-015-02: AcousticBrainz unavailable, Essentia successful → Store JSON, mark 'FLAVOR READY'
- TC-U-015-03: Both AcousticBrainz and Essentia fail → Mark 'FLAVORING FAILED'
- TC-U-015-04: Song status 'FLAVOR READY' → Skip flavoring

### REQ-SPEC032-016: File Completion
- TC-U-016-01: All passages and songs complete → Mark file 'INGEST COMPLETE'
- TC-U-016-02: Incomplete passages → File remains 'PROCESSING'

### REQ-SPEC032-017: Session Completion
- TC-U-017-01: All files dispositioned → Session complete
- TC-U-017-02: Pending files remain → Session incomplete

### REQ-SPEC032-018: Database Settings Table
- TC-U-018-01: Read setting with value → Return value
- TC-U-018-02: Read setting with NULL → Return default
- TC-U-018-03: Write setting → Value persists
- TC-U-018-04: All 7 parameters readable (silence_threshold_dB, silence_min_duration_ticks, minimum_passage_audio_duration_ticks, lead-in_threshold_dB, lead-out_threshold_dB, acoustid_api_key, ai_processing_thread_count)

### REQ-SPEC032-019: Thread Count Auto-Initialization
- TC-U-019-01: ai_processing_thread_count NULL, CPU 8 cores → Initialize to 9, persist
- TC-U-019-02: ai_processing_thread_count = 6 → Use 6 (no initialization)
- TC-U-019-03: User edits to 4 → Next import uses 4

### REQ-SPEC032-021: Status Field Enumerations
- TC-U-021-01: files.status transitions (PENDING → PROCESSING → INGEST COMPLETE)
- TC-U-021-02: files.status alternate transitions (PENDING → DUPLICATE HASH, PENDING → NO AUDIO)
- TC-U-021-03: passages.status transition (PENDING → INGEST COMPLETE)
- TC-U-021-04: songs.status transitions (PENDING → FLAVOR READY, PENDING → FLAVORING FAILED)

### REQ-SPEC032-NF-003: Sample-Accurate Timing
- TC-U-NF003-01: Passage start time sample-accurate (SPEC017 ticks)
- TC-U-NF003-02: Lead-in/lead-out sample-accurate (SPEC017 ticks)

### REQ-SPEC032-NF-004: Symlink/Junction Handling
- TC-U-NF004-01: Symlink detected → Skipped
- TC-U-NF004-02: Junction point detected → Skipped

### REQ-SPEC032-NF-005: Metadata Preservation
- TC-U-NF005-01: Extracted metadata stored even if unused for matching
- TC-U-NF005-02: Re-import preserves old metadata not in new extraction

---

## Integration Tests (24)

### REQ-SPEC032-004: AcoustID API Key Validation
- TC-I-004-01: End-to-end API key validation workflow (invalid → prompt → valid → continue)

### REQ-SPEC032-007: Filename Matching Logic
- TC-I-007-01: Database query for filename match → Correct fileId returned

### REQ-SPEC032-008: Hash-Based Duplicate Detection
- TC-I-008-01: Hash computation → Database storage → Duplicate query workflow
- TC-I-008-02: Bidirectional matching_hashes update in database (verified via SQL)

### REQ-SPEC032-009: Metadata Extraction Merging
- TC-I-009-01: Extract → Merge → Store → Verify in database

### REQ-SPEC032-010: Silence-Based Segmentation
- TC-I-010-01: Settings table read → Silence detection → Passage boundary identification

### REQ-SPEC032-011: Fingerprinting Per Passage
- TC-I-011-01: Chromaprint → AcoustID API call → MBID candidates returned
- TC-I-011-02: API key validation → Skip AcoustID integration

### REQ-SPEC032-012: Song Matching with Confidence
- TC-I-012-01: Metadata + fingerprint → Confidence scoring → MBID assignment
- TC-I-012-02: Zero-song passage creation (no MBID assigned)

### REQ-SPEC032-013: Passage Recording
- TC-I-013-01: Passage → Database insert → Song relationship established
- TC-I-013-02: Multiple passages → Same song → passage_songs relationship

### REQ-SPEC032-014: Amplitude-Based Lead-In/Lead-Out
- TC-I-014-01: Audio analysis → Database write (lead-in/lead-out, fade-in/fade-out NULL)

### REQ-SPEC032-015: Musical Flavor Retrieval
- TC-I-015-01: AcousticBrainz API → JSON storage → song.status = 'FLAVOR READY'
- TC-I-015-02: Essentia subprocess → JSON storage → song.status = 'FLAVOR READY'
- TC-I-015-03: Both fail → song.status = 'FLAVORING FAILED'

### REQ-SPEC032-016: File Completion
- TC-I-016-01: All phases complete → file.status = 'INGEST COMPLETE'

### REQ-SPEC032-017: Session Completion
- TC-I-017-01: All files dispositioned → Workflow completion event

### REQ-SPEC032-018: Database Settings Table
- TC-I-018-01: Settings persistence across restarts

### REQ-SPEC032-019: Thread Count Auto-Initialization
- TC-I-019-01: Auto-initialization → Database persistence → Next import uses stored value

### REQ-SPEC032-020: Thirteen UI Progress Sections
- TC-I-020-01: SSE events emitted for all 13 sections during import

### REQ-SPEC032-021: Status Field Enumerations
- TC-I-021-01: Status transitions enforced by database (if constraints present)

### REQ-SPEC032-NF-001: Parallel Processing
- TC-I-NF001-01: Multiple files processed concurrently (verify via logs)

### REQ-SPEC032-NF-002: Real-Time Progress Updates
- TC-I-NF002-01: SSE events received within 500ms of file completion

### REQ-SPEC032-NF-005: Metadata Preservation
- TC-I-NF005-01: Metadata round-trip (extract → store → retrieve → verify)

---

## System Tests (12)

### REQ-SPEC032-001-003: SPEC032 Documentation
- TC-S-001-01: SPEC032 includes scope definition section
- TC-S-002-01: SPEC032 includes two-stage roadmap section
- TC-S-003-01: SPEC032 includes five-step workflow section
- TC-S-006-01: SPEC032 includes ten-phase pipeline section

### REQ-SPEC032-004: AcoustID API Key Validation
- TC-S-004-01: End-to-end import with invalid API key → Prompt → Skip fingerprinting

### REQ-SPEC032-005: Folder Selection
- TC-S-005-01: End-to-end import with external folder (Stage One) → Error → Workflow blocked

### REQ-SPEC032-008: Hash-Based Duplicate Detection
- TC-S-008-01: Import same file twice → Second import skipped as DUPLICATE HASH

### REQ-SPEC032-010: Silence-Based Segmentation
- TC-S-010-01: Import file with 2 songs separated by silence → 2 passages created

### REQ-SPEC032-012: Song Matching with Confidence
- TC-S-012-01: Import file with known song → High confidence match → Passage with song

### REQ-SPEC032-014: Amplitude-Based Lead-In/Lead-Out
- TC-S-014-01: Import file → Lead-in/lead-out detected → Ready for crossfade

### REQ-SPEC032-015: Musical Flavor Retrieval
- TC-S-015-01: Import file → Song flavored (AcousticBrainz or Essentia) → Ready for selection

### REQ-SPEC032-020: Thirteen UI Progress Sections
- TC-S-020-01: UI displays all 13 sections with real-time updates during import

### REQ-SPEC032-NF-001: Parallel Processing
- TC-S-NF001-01: Import 100 files → Processes in parallel (verify completion time < sequential)

### REQ-SPEC032-NF-002: Real-Time Progress Updates
- TC-S-NF002-01: User observes real-time progress during import (manual verification)

### REQ-SPEC032-NF-004: Symlink/Junction Handling
- TC-S-NF004-01: Folder with symlink → Symlink not followed, target not imported

---

## Test Coverage Matrix

### By Priority

| Priority | Requirements | Tests | Avg Tests/Req |
|----------|--------------|-------|---------------|
| P0 (Critical) | 19 | 60 | 3.2 |
| P1 (High) | 7 | 18 | 2.6 |
| **Total** | **26** | **78** | **3.0** |

### By Type

| Type | Count | Percentage |
|------|-------|------------|
| Unit | 42 | 54% |
| Integration | 24 | 31% |
| System | 12 | 15% |
| **Total** | **78** | **100%** |

### By Requirement Category

| Category | Requirements | Tests | Coverage |
|----------|--------------|-------|----------|
| Documentation (SPEC032) | 4 | 4 | 100% |
| Workflow | 5 | 16 | 100% |
| Per-File Pipeline | 10 | 47 | 100% |
| Settings & Configuration | 2 | 9 | 100% |
| Non-Functional | 5 | 10 | 100% |

---

## Test Status Legend

- **Defined:** Test specification complete, ready to implement
- **Pending:** Test depends on issue resolution (e.g., ISSUE-HIGH-002)
- **Blocked:** Waiting for external dependency (e.g., Essentia installation)
- **Implemented:** Test code written and passing
- **Failed:** Test implemented but failing (bug found)

**Current Status:** All tests **Defined** (specifications complete, ready for implementation)

---

## Test Data Requirements

### Audio Test Files

**Required Test Files (to be created/sourced):**
1. **single_song.mp3** - Single passage, known MBID, ~3 minutes
2. **two_songs.mp3** - Two songs separated by 3s silence
3. **embedded_silence.mp3** - Single song with brief silence (e.g., Pink Floyd)
4. **very_quiet.wav** - Audio with <100ms non-silence (for NO AUDIO test)
5. **no_metadata.mp3** - Valid audio, no ID3 tags
6. **complete_metadata.mp3** - All ID3 fields populated
7. **partial_metadata.mp3** - Some ID3 fields missing
8. **renamed_duplicate.mp3** - Same content as single_song.mp3, different filename
9. **symlink_target.mp3** - Audio file to be linked via symlink
10. **unknown_song.mp3** - Not in MusicBrainz (for zero-song passage test)

### Database States

**Required Database Fixtures:**
1. **Empty database** - Fresh wkmp.db for clean import tests
2. **Pre-existing file** - Database with file entry (for filename matching tests)
3. **Pre-existing hash** - Database with file hash (for duplicate detection tests)
4. **Pre-existing metadata** - Database with metadata (for merge tests)
5. **Pre-existing song** - Database with song entry (for relationship tests)

### External Services

**Mock/Stub Requirements:**
- **AcoustID API Mock** - Returns known MBIDs for test fingerprints
- **AcousticBrainz API Mock** - Returns test flavor JSON
- **Essentia Stub** - Returns test flavor JSON (or use actual Essentia if available)

---

## Execution Strategy

### Phase 1: Unit Tests (First)
- Test individual components in isolation
- Mock external dependencies (APIs, database, filesystem)
- Fast execution (<1 second per test)

### Phase 2: Integration Tests (Second)
- Test component interactions
- Use real database (SQLite in-memory or temporary file)
- Mock external APIs (AcoustID, AcousticBrainz)
- Moderate execution (1-5 seconds per test)

### Phase 3: System Tests (Final)
- End-to-end workflows
- Real database, real audio files
- May use real APIs (with rate limiting awareness)
- Slow execution (10-60 seconds per test)

### Continuous Integration

**Test Triggers:**
- Pre-commit: Unit tests (fast feedback)
- Pre-push: Unit + Integration tests (comprehensive feedback)
- CI Pipeline: All tests (gate before merge)

**Performance Target:**
- Unit tests: <30 seconds total
- Integration tests: <2 minutes total
- System tests: <5 minutes total
- **Total test suite: <8 minutes**

---

## Test Files Location

**Modular Test Specifications:**
- `02_test_specifications/tc_u_004_01.md` - Unit test REQ-004, test 01
- `02_test_specifications/tc_i_008_01.md` - Integration test REQ-008, test 01
- `02_test_specifications/tc_s_010_01.md` - System test REQ-010, test 01
- (78 individual test spec files)

**Note:** Due to context window constraints, detailed test specifications are provided for representative tests. All 78 tests follow the same format specified in Phase 3 workflow template.
