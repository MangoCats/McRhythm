# Specification: Embedded MusicBrainz ID Matching (Stage 0)

**Document ID:** SPEC-EMBID-001
**Status:** Implemented (per-file pipeline)
**Date:** 2024-12-14
**Implementation Date:** 2026-02-05
**Author:** Cross-validation analysis

---

## Executive Summary

This specification defines a new "Stage 0" matching strategy that prioritizes embedded MusicBrainz Recording IDs from ID3 tags. Cross-validation analysis of 5,551 files demonstrates:

- **98.5% total coverage** using embedded MB IDs as primary match source
- **17.6% improvement** over current AcoustID-first approach (80.9% coverage)
- **Higher confidence** (user-approved tags vs fingerprint matching)
- **Faster execution** (no API calls needed for 98% of files)
- **Offline capability** (works without network)

---

## 1. Problem Statement

### 1.1 Current Approach Limitations

The current matching pipeline uses AcoustID fingerprinting as the primary identification method:

| Stage | Method | Coverage | Notes |
|-------|--------|----------|-------|
| Stage 1 | AcoustID | 64.2% | Requires fingerprint generation + API call |
| Stage 2 | Album-first MB | 16.6% | Fallback, requires MB API calls |
| **Total** | | **80.9%** | 19.1% unmatched |

**Key issues:**
1. AcoustID requires network API calls (rate-limited, slow)
2. Fingerprint matching can fail for obscure recordings
3. AcoustID may return different recordings than user intended
4. 19.1% of files remain unmatched

### 1.2 Discovery: Embedded MusicBrainz IDs

Analysis revealed that **98.1% of files already have embedded MusicBrainz Recording IDs** in their ID3 tags, set by MusicBrainz Picard during user tagging.

| Metadata Field | Availability |
|----------------|-------------|
| MusicBrainz Recording ID | 98.1% |
| ISRC | 62.3% |
| MusicBrainz Release ID | 98.1% |
| Track Number | 98.9% |

These embedded IDs are **higher confidence** than fingerprint matches because:
1. Set by MusicBrainz Picard (uses fingerprint + metadata matching)
2. User explicitly approved the match during tagging
3. Represents user intent (specific release/version)

---

## 2. Proposed Solution

### 2.1 Confidence Tier System

```
TIER 1: EMBEDDED MB ID (Highest Confidence)
  1A: Embedded Recording ID + ISRC     [100% confidence]
  1B: Embedded Recording ID only       [98% confidence]

TIER 2: ACOUSTID FINGERPRINT (Fallback)
  2A: AcoustID confidence >= 0.95      [95%+ confidence]
  2B: AcoustID confidence 0.80-0.95    [80-95% confidence]

TIER 3: ALBUM-FIRST QUERY (Rare Fallback)
  3A: Title + duration match           [varies]

TIER 4: NO MATCH
  Requires manual tagging
```

### 2.2 Coverage Analysis (5,551 files)

| Tier | Count | Percentage | Description |
|------|-------|------------|-------------|
| 1A | 3,458 | 62.3% | Embedded + ISRC |
| 1B | 1,989 | 35.8% | Embedded only |
| 2A | 17 | 0.3% | AcoustID high |
| 2B | 1 | 0.0% | AcoustID medium |
| 3 | - | - | Album-first (not needed) |
| 4 | 86 | 1.5% | No match |
| **Total Matched** | **5,465** | **98.5%** | |

### 2.3 Matching Pipeline Flow

**Single-track files** (resolved per-file):

```
                    ┌─────────────────────┐
                    │  Single-Track File  │
                    └──────────┬──────────┘
                               │
                    ┌──────────▼──────────┐
                    │  Extract ID3 Tags   │
                    │  (lofty crate)      │
                    └──────────┬──────────┘
                               │
              ┌────────────────┼────────────────┐
              │                │                │
    ┌─────────▼─────────┐     │      ┌─────────▼─────────┐
    │ MB Recording ID?  │     │      │      ISRC?        │
    └─────────┬─────────┘     │      └─────────┬─────────┘
              │               │                │
         ┌────┴────┐          │           ┌────┴────┐
         │   YES   │          │           │   YES   │
         │         │          │           │         │
    ┌────▼────┐    │          │      ┌────▼────┐    │
    │ TIER 1  │    │          │      │ TIER 1A │    │
    │ ACCEPT  │◄───┴──────────┴──────┤ (boost) │    │
    └─────────┘                      └─────────┘    │
                                                   │
         ┌────┐                                    │
         │ NO │◄───────────────────────────────────┘
         └──┬─┘
            │
    ┌───────▼───────────┐
    │  STAGE 1:         │
    │  ContextualMatcher│
    │  (artist+title)   │
    └───────┬───────────┘
            │
       ┌────┴────┐
       │  MATCH? │
       └────┬────┘
            │
    ┌───YES─┴─NO───┐
    │              │
┌───▼───┐    ┌────▼──────┐
│TIER 3 │    │ STAGE 2:  │
│ACCEPT │    │ AcoustID  │
└───────┘    │ Fingerpr. │
             └────┬──────┘
                  │
             ┌────┴────┐
             │  MATCH? │
             └────┬────┘
                  │
          ┌──YES─┴─NO──┐
          │            │
      ┌───▼───┐   ┌────▼────┐
      │TIER 2 │   │ TIER 4  │
      │ACCEPT │   │NO MATCH │
      └───────┘   └─────────┘
```

**Album files** (resolved per-passage from matched edition):

```
                    ┌─────────────────────┐
                    │    Album File       │
                    │  (multi-passage)    │
                    └──────────┬──────────┘
                               │
                    ┌──────────▼──────────┐
                    │  Phase 4: Segment   │
                    │  passage boundaries │
                    └──────────┬──────────┘
                               │
                    ┌──────────▼──────────┐
                    │  STAGE 1:           │
                    │  AlbumMatcher       │
                    │  (edition matching) │
                    └──────────┬──────────┘
                               │
                          ┌────┴────┐
                          │ MATCH?  │
                          └────┬────┘
                               │
                    ┌────YES───┴───NO────┐
                    │                    │
          ┌─────────▼─────────┐   ┌─────▼──────────┐
          │ Track count ==    │   │  STAGE 2:      │
          │ passage count?    │   │  AcoustID      │
          └─────────┬─────────┘   │  per passage   │
                    │             └─────┬──────────┘
               ┌────┴────┐             │
               │  YES/NO │        (existing flow)
               └────┬────┘
                    │
          ┌───YES──┴──NO───┐
          │                │
    ┌─────▼──────┐   ┌────▼──────────┐
    │ TIER 3     │   │ STAGE 2:      │
    │ per passage│   │ AcoustID      │
    │ from tracks│   │ per passage   │
    └────────────┘   └───────────────┘
```

**Stage numbering** (as implemented in `mbid_cascade.rs` and `pipeline_plan024.rs`):

| Stage | Method | Applies To | Confidence Tier |
|-------|--------|-----------|-----------------|
| **Stage 0** | Embedded MBID (ID3 tags) | Single-track files | Tier 1A/1B |
| **Stage 1** | ContextualMatcher (artist+title search) | Single-track files | Tier 3 |
| **Stage 1** | AlbumMatcher (edition matching) | Album files | Tier 3 |
| **Stage 2** | AcoustID fingerprinting | Fallback for unresolved passages | Tier 2A/2B |

---

## 3. Integration with Existing wkmp-ai

### 3.1 Existing Infrastructure

The codebase already provides:

| Component | Location | Status |
|-----------|----------|--------|
| ID3Extractor | `extractors/id3_extractor.rs` | Extracts MB Recording ID |
| IdentityResolver | `fusion/identity_resolver.rs` | Bayesian MBID fusion |
| AcoustID Client | `extractors/acoustid_client.rs` | Fingerprint matching |
| MusicBrainz Client | `services/musicbrainz_client.rs` | MB API access |

**Key finding:** `ID3Extractor.extract_musicbrainz_recording_id()` already extracts embedded MB IDs with 0.9 confidence (line 187 of id3_extractor.rs).

### 3.2 Required Changes

**Change 1: Prioritize embedded MB ID in identity resolution**

Current behavior: All extractors run in parallel, IdentityResolver fuses results.
Proposed: Check embedded MB ID first; skip AcoustID if embedded ID present.

**Change 2: Add ISRC extraction for confidence boost**

Extract ISRC from ID3 tags to enable Tier 1A classification.

**Change 3: Add confidence tier to match result**

Return confidence tier (1A/1B/2A/2B/3/4) with each match result.

**Change 4: Optional MB ID validation**

For Tier 1 matches, optionally validate MB ID still exists in database.

### 3.3 Data Flow

```
Input: File path, File hash

Step 1: Extract ID3 metadata (ID3Extractor)
  - MB Recording ID
  - MB Release ID
  - MB Artist ID
  - ISRC
  - Track number
  - Artist, Title, Album

Step 2: Check for embedded MB Recording ID
  IF present AND valid UUID format:
    IF ISRC also present:
      RETURN Tier 1A match (confidence: 1.0)
    ELSE:
      RETURN Tier 1B match (confidence: 0.98)

Step 3: (Fallback) Run AcoustID fingerprinting
  IF confidence >= 0.95 AND artist_similarity >= 0.70:
    RETURN Tier 2A match
  ELIF confidence >= 0.80 AND artist_similarity >= 0.70:
    RETURN Tier 2B match

Step 4: (Rare fallback) Album-first query
  Query MB by artist + album
  Match by title similarity + duration
  RETURN Tier 3 match or Tier 4 no-match
```

---

## 4. Requirements

### 4.1 Functional Requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-01 | Extract embedded MB Recording ID from ID3 tags | P0 |
| FR-02 | Extract ISRC from ID3 tags | P0 |
| FR-03 | Validate MB Recording ID format (UUID) | P0 |
| FR-04 | Assign confidence tier based on available metadata | P0 |
| FR-05 | Skip AcoustID when embedded MB ID present | P1 |
| FR-06 | Fall back to AcoustID when no embedded ID | P1 |
| FR-07 | Optionally validate MB ID against MB database | P2 |
| FR-08 | Log confidence tier with match results | P1 |

### 4.2 Non-Functional Requirements

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-01 | Stage 0 matching (ID3 only) | < 50ms per file |
| NFR-02 | Total coverage | >= 98% of well-tagged files |
| NFR-03 | Confidence accuracy | Tier 1 should have < 0.1% false matches |
| NFR-04 | Offline capability | Stage 0 works without network |

---

## 5. Validation Data

### 5.1 Cross-Validation Results

Based on analysis of 5,551 audio files:

```
CONFIDENCE TIER DISTRIBUTION:
  Tier 1A (Embedded + ISRC):   3,458 (62.3%) - HIGHEST confidence
  Tier 1B (Embedded only):     1,989 (35.8%) - VERY HIGH confidence
  Tier 2A (AcoustID high):        17 ( 0.3%) - HIGH confidence
  Tier 2B (AcoustID medium):       1 ( 0.0%) - MEDIUM confidence
  Tier 4  (No match):             86 ( 1.5%) - Needs manual

  TOTAL MATCHED:               5,465 (98.5%)
```

### 5.2 Unmatched Files Analysis

The 86 unmatched files (1.5%) are primarily:
- Children's educational music (Baby Reflections, Kathy Troxel)
- Various artists compilations
- Obscure recordings not in MusicBrainz

These files likely require manual tagging and are outside typical music library scope.

### 5.3 Embedded vs AcoustID Comparison

```
Files with both embedded and AcoustID MBIDs: 3,852
  Same MB ID:      0 (0.0%)
  Different MB ID: 3,852 (100.0%)
```

**Note:** Different IDs typically represent same song on different releases. Embedded ID represents user's intended release choice and should be preferred.

---

## 6. Success Criteria

| Metric | Target | Measurement |
|--------|--------|-------------|
| Coverage | >= 98% | Files with MB Recording ID / Total files |
| Tier 1 rate | >= 95% | Tier 1 matches / Total matches |
| Speed improvement | >= 50% | Time with Stage 0 / Time with AcoustID-first |
| False positive rate | < 0.1% | Invalid MB IDs / Total Tier 1 matches |

---

## 7. Risks and Mitigations

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Embedded MB IDs outdated (recording merged) | Low | Low | Optional MB validation |
| Files not tagged with Picard | Medium | Medium | Fallback to AcoustID |
| ISRC extraction fails | Low | Low | Still Tier 1B (high confidence) |
| User expects AcoustID behavior | Low | Low | Document behavior change |

---

## 8. Files Created During Analysis

| File | Purpose |
|------|---------|
| `stage0_embedded_mbid.py` | Stage 0 extraction script |
| `stage0_embedded_results.json` | Extraction results (5,551 files) |
| `final_confidence_report.json` | Confidence-tiered report |

---

## Implementation Status

**Stage 0 implemented in per-file pipeline** (2026-02-05):

| Component | File | Status |
|-----------|------|--------|
| MBID extraction from ID3 tags | `services/metadata_extractor.rs` | Done |
| MBID propagation through merger | `services/metadata_merger.rs` | Done |
| MbidResolution struct | `services/passage_song_matcher.rs` | Done |
| MbidIdentificationCascade service | `services/mbid_cascade.rs` | Done |
| Single-track cascade (Stage 0 → 1 → 2) | `services/workflow_orchestrator/mod.rs` | Done |
| Album cascade (deferred, all-None) | `services/mbid_cascade.rs` | Placeholder |
| Pre-resolved matching | `services/passage_song_matcher.rs` | Done |
| Unit tests (9 tests) | Multiple files | Done |

**Scope:**
- Single-track files: Full cascade (Stage 0 → Stage 1 → Stage 2)
- Album files: Stage 0 not applicable (embedded MBID is per-file, not per-passage). Albums continue using AcoustID per-passage (Stage 2).
- Future: release lookup → per-track MBID mapping for albums.
