# PLAN026: wkmp-ai Refactoring - Requirements Index

**Source Documents:**
- [wkmp-ai/refactor1126.md](../../wkmp-ai/refactor1126.md) (878 lines)
- [wkmp-ai/SPEC_content_type_determination.md](../../wkmp-ai/SPEC_content_type_determination.md) (408 lines)
- [wkmp-ai/refactor1126_analysis.md](../../wkmp-ai/refactor1126_analysis.md) (resolution document)

**Extraction Date:** 2025-11-26

---

## Requirements Summary

| Category | Count | Priority Distribution |
|----------|-------|----------------------|
| Pipeline Steps | 7 | All P0 (core pipeline) |
| Database Schema | 9 | All P0 (foundational) |
| Algorithm/Processing | 14 | P0/P1 mix |
| Integration | 5 | P1 |
| Architecture | 4 | P0 |
| **Total** | **39** | |

---

## Pipeline Step Requirements (P0 - Core)

| Req ID | Type | Description | Source Line | Priority |
|--------|------|-------------|-------------|----------|
| REQ-STEP-001 | Functional | Step 1: Folder scan with magic byte analysis | refactor1126:85-107 | P0 |
| REQ-STEP-002 | Functional | Step 2: Known files check against database | refactor1126:109-119 | P0 |
| REQ-STEP-003 | Functional | Step 3: SHA-256 hash computation and duplicate detection | refactor1126:121-136 | P0 |
| REQ-STEP-004 | Functional | Step 4: Metadata extraction (ID3, Vorbis, MP4, APEv2, BWF, WMA) | refactor1126:137-144 | P0 |
| REQ-STEP-005 | Functional | Step 5: Audio decoding with NO AUDIO detection | refactor1126:145-154 | P0 |
| REQ-STEP-006 | Functional | Step 6: Content type determination (algorithm per SPEC_content_type) | refactor1126:155-189 | P0 |
| REQ-STEP-007 | Functional | Completed duplicate handling (AtomicBool signal pattern) | refactor1126:201-207 | P0 |

---

## Database Schema Requirements (P0 - Foundational)

| Req ID | Type | Description | Source Line | Priority |
|--------|------|-------------|-------------|----------|
| REQ-DB-001 | Data | Files table with guid, status, path, hash, metadata columns | refactor1126:482-624 | P0 |
| REQ-DB-002 | Data | Passages table with tick-based timing (6 timing points per SPEC002) | refactor1126:293-354 | P0 |
| REQ-DB-003 | Data | Songs table with recording_mbid (indexed, NOT unique) | refactor1126:357-382 | P0 |
| REQ-DB-004 | Data | Works table with work_mbid | refactor1126:386-404 | P0 |
| REQ-DB-005 | Data | Artists table with artist_mbid | refactor1126:407-422 | P0 |
| REQ-DB-006 | Data | Albums table with album_mbid | refactor1126:425-440 | P0 |
| REQ-DB-007 | Data | passage_songs relationship table with timing semantics (AMB-06) | refactor1126:445-460 | P0 |
| REQ-DB-008 | Data | song_artists relationship table with weights | refactor1126:461-469 | P0 |
| REQ-DB-009 | Data | passage_albums relationship table | refactor1126:471-478 | P0 |

---

## Algorithm/Processing Requirements

| Req ID | Type | Description | Source Line | Priority |
|--------|------|-------------|-------------|----------|
| REQ-ALG-001 | Functional | Duration-based triage (<12 min, 12-25 min, >25 min) | SPEC_content:57-71 | P0 |
| REQ-ALG-002 | Functional | Quick silence scan (-45 dB, 2s gaps, segment estimation) | SPEC_content:75-97 | P0 |
| REQ-ALG-003 | Functional | Single-song path: Chromaprint + AcoustID lookup | SPEC_content:100-155 | P0 |
| REQ-ALG-004 | Functional | Album path: metadata reconciliation + MusicBrainz search | SPEC_content:158-208 | P0 |
| REQ-ALG-005 | Functional | Edition testing (Stages 2-5 from am28) | SPEC_content:209-268 | P0 |
| REQ-ALG-006 | Functional | Fallback per-segment analysis | SPEC_content:272-296 | P1 |
| REQ-ALG-007 | Functional | Dual-path processing (12-25 min files) | SPEC_content:300-319 | P1 |
| REQ-ALG-008 | Functional | Lead-in/lead-out amplitude analysis | refactor1126:712-727 | P1 |
| REQ-ALG-009 | Functional | Artist verification (Jaro-Winkler ≥50%) | refactor1126:246-255 | P0 |
| REQ-ALG-010 | Functional | Artist mismatch handling (create + flag) | refactor1126:648-662 | P0 |
| REQ-ALG-011 | Functional | MULTIPLE_SONGS handling (one passage per segment) | refactor1126:180-189 | P1 |
| REQ-ALG-012 | Constraint | Match tolerance: 3.0 seconds | refactor1126:238-239 | P0 |
| REQ-ALG-013 | Constraint | High confidence threshold: ≥80% | refactor1126:219-220 | P0 |
| REQ-ALG-014 | Constraint | Full album: ≥95%, Partial album: 75-94% | refactor1126:242-243 | P0 |

---

## Integration Requirements

| Req ID | Type | Description | Source Line | Priority |
|--------|------|-------------|-------------|----------|
| REQ-INT-001 | Integration | am28 ValidationResult → Files + Passages mapping | refactor1126:631-662 | P1 |
| REQ-INT-002 | Integration | am28 TrackMatch → Passage timing computation | refactor1126:664-681 | P1 |
| REQ-INT-003 | Integration | am28 Edition → Album + Songs creation | refactor1126:683-696 | P1 |
| REQ-INT-004 | Integration | Recording MBID flow: Edition → Song → passage_songs | refactor1126:698-708 | P1 |
| REQ-INT-005 | Integration | Import metadata JSON structure (missing_tracks, rejected_editions) | refactor1126:729-761 | P1 |

---

## Architecture Requirements

| Req ID | Type | Description | Source Line | Priority |
|--------|------|-------------|-------------|----------|
| REQ-ARCH-001 | Architecture | Single-threaded db_access thread pattern | refactor1126:37 | P0 |
| REQ-ARCH-002 | Architecture | Files "played where they lie" (read-only, no modifications) | refactor1126:29 | P0 |
| REQ-ARCH-003 | Architecture | Network-free post-import (all data locally cached) | refactor1126:30-31 | P0 |
| REQ-ARCH-004 | Architecture | SSE progress reporting for UI live updates | refactor1126:13 | P1 |

---

## Content Type Classification Requirements

| Req ID | Type | Description | Source Line | Priority |
|--------|------|-------------|-------------|----------|
| REQ-CLASS-001 | Classification | SINGLE_SONG: One recording with MBID | SPEC_content:15 | P0 |
| REQ-CLASS-002 | Classification | FULL_ALBUM: ≥95% tracks matched to Release MBID | SPEC_content:16 | P0 |
| REQ-CLASS-003 | Classification | PARTIAL_ALBUM: 75-94% tracks matched | SPEC_content:17 | P0 |
| REQ-CLASS-004 | Classification | MULTIPLE_SONGS: Multiple recordings, no single release | SPEC_content:18 | P0 |
| REQ-CLASS-005 | Classification | NOT_IN_MUSICBRAINZ: Audio present, no MB identification | SPEC_content:19 | P0 |
| REQ-CLASS-006 | Classification | IDENTIFICATION_FAILED: Low confidence in any classification | SPEC_content:20 | P0 |

---

## Keep/Preserve Requirements

| Req ID | Type | Description | Source Line | Priority |
|--------|------|-------------|-------------|----------|
| REQ-KEEP-001 | Preservation | Reuse file scanner and magic byte reader | refactor1126:11 | P0 |
| REQ-KEEP-002 | Preservation | Preserve UI structure with SSE live updates | refactor1126:13 | P0 |
| REQ-KEEP-003 | Preservation | Keep single-threaded db_access pattern | refactor1126:17 | P0 |
| REQ-KEEP-004 | Preservation | Retain zero-conf database and settings table | refactor1126:19 | P0 |

---

## Constants/Thresholds (for reference)

| Constant | Value | Source |
|----------|-------|--------|
| HIGH_CONFIDENCE_THRESHOLD | 0.80 | refactor1126:220 |
| CONFIDENCE_EXCELLENT_MIN | 0.95 | refactor1126:229 |
| CONFIDENCE_GOOD_MIN | 0.75 | refactor1126:230 |
| CONFIDENCE_FAIR_MIN | 0.50 | refactor1126:231 |
| MATCH_TOLERANCE_SECS | 3.0 | refactor1126:239 |
| FULL_ALBUM_MIN_MATCH_PCT | 95.0 | refactor1126:242 |
| PARTIAL_ALBUM_MIN_MATCH_PCT | 75.0 | refactor1126:243 |
| ARTIST_SIMILARITY_ACCEPT | 0.50 | refactor1126:250 |
| LEAD_THRESHOLD_DB | -12.0 | refactor1126:261 |
| LEAD_RMS_WINDOW_MS | 100 | refactor1126:262 |
| DURATION_SINGLE_SONG_MAX_SECS | 720.0 | SPEC_content:68 |
| DURATION_AMBIGUOUS_MAX_SECS | 1500.0 | SPEC_content:69 |
| QUICK_SCAN_THRESHOLD_DB | -45.0 | SPEC_content:83 |
| QUICK_SCAN_MIN_GAP_SECS | 2.0 | SPEC_content:84 |
| ACOUSTID_HIGH_CONFIDENCE | 0.80 | SPEC_content:129 |
| ACOUSTID_MEDIUM_CONFIDENCE | 0.40 | SPEC_content:130 |
| MB_RATE_LIMIT_MS | 1550 | SPEC_content:190 |

---

## Notes

- Total unique requirements: 39
- All P0 requirements must be implemented before feature can be considered functional
- P1 requirements can be deferred to subsequent increments
- Database schema (REQ-DB-*) must be implemented first as foundation
- Pipeline steps (REQ-STEP-*) depend on database schema
- Algorithm requirements (REQ-ALG-*) implement Step 6 specifics
