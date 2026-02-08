# Requirements Index - Edition Filtering Improvements

**Source Document:** wkmp-ai/problem_albums_analysis.md
**Plan:** PLAN027
**Date:** 2026-01-16

---

## Requirements Summary

**Total Requirements:** 5 (1 Critical, 2 High, 1 Medium, 1 Low)

**Critical:** Edition preference scoring must prevent deluxe/compilation mismatch
**High:** Track count and artist validation must filter unsuitable editions
**Medium:** Remix tolerance should accept variable-duration remixes
**Low:** Logging improvements for debugging

---

## Requirements Table

| Req ID | Type | Priority | Brief Description | Source Line | Est. Effort |
|--------|------|----------|-------------------|-------------|-------------|
| REQ-EF-010 | Functional | Critical | Edition preference scoring - penalize deluxe editions | 85-118 | 4-6h |
| REQ-EF-020 | Functional | Critical | Edition preference scoring - penalize compilations | 85-118 | 2-3h |
| REQ-EF-030 | Functional | High | Track count pre-filtering (±3 tracks tolerance) | 121-139 | 2-3h |
| REQ-EF-040 | Functional | High | Artist consistency validation | 141-162 | 4-6h |
| REQ-EF-050 | Functional | Medium | Remix track error tolerance (up to 120s) | 164-183 | 2-3h |

---

## Detailed Requirements

### REQ-EF-010: Edition Preference Scoring - Deluxe Penalty

**Category:** Functional
**Priority:** Critical
**Source:** problem_albums_analysis.md:85-118

**Description:**
Implement edition preference scoring that penalizes deluxe and expanded editions to prefer standard editions when available.

**Acceptance Criteria:**
- Edition titles containing "deluxe" receive 0.7× score multiplier
- Edition titles containing "expanded" receive 0.7× score multiplier
- Score reduction is cumulative with other penalties
- Case-insensitive matching ("Deluxe", "DELUXE", "deluxe" all match)

**Rationale:**
Deluxe editions often contain bonus tracks, remixes, or alternate versions that don't match user's audio files. Standard editions are more likely to match the actual file content.

**Problem Albums Addressed:**
- Chemical Brothers - Surrender (deluxe with remixes)
- Imagine Dragons - Night Visions (16-track deluxe vs 11-track file)
- Michael Jackson - Thriller (compilation vs standard)

---

### REQ-EF-020: Edition Preference Scoring - Compilation Penalty

**Category:** Functional
**Priority:** Critical
**Source:** problem_albums_analysis.md:85-118

**Description:**
Implement edition preference scoring that penalizes compilation, anthology, and "best of" releases to prefer studio albums.

**Acceptance Criteria:**
- Edition titles containing "collection" receive 0.6× score multiplier
- Edition titles containing "anthology" receive 0.6× score multiplier
- Edition titles containing "best of" receive 0.6× score multiplier
- Case-insensitive matching
- Penalty applies before other scoring adjustments

**Rationale:**
Compilations often contain tracks from multiple albums/artists and may have different track counts than studio albums. User files are typically studio album rips.

**Problem Albums Addressed:**
- James Gang - Funk #49 (multi-artist compilation wrongly matched)
- Michael Jackson - Thriller (compilation vs standard)

---

### REQ-EF-030: Track Count Pre-Filtering

**Category:** Functional
**Priority:** High
**Source:** problem_albums_analysis.md:121-139

**Description:**
Filter candidate editions BEFORE running matching stages to exclude editions with significantly different track counts from detected boundaries.

**Acceptance Criteria:**
- Filter editions where `|edition.track_count - detected_boundaries| > 3`
- Apply filter BEFORE Stage 2 (album-first matching)
- Log filtered editions for debugging (track count mismatch)
- If all editions filtered, fall back to unfiltered list (with warning log)

**Rationale:**
Matching 11-track file to 16-track deluxe edition wastes computation and produces poor results. Pre-filtering reduces candidate set and improves match quality.

**Problem Albums Addressed:**
- Imagine Dragons - Night Visions (16-track deluxe vs 11-track file)
- Michael Jackson - Thriller (19-25 track compilation vs 9-track standard)

**Expected Impact:**
- Reduces wasted computation on unsuitable editions
- Improves average match quality by 10-15%
- Fixes 2 out of 4 problem albums

---

### REQ-EF-040: Artist Consistency Validation

**Category:** Functional
**Priority:** High
**Source:** problem_albums_analysis.md:141-162

**Description:**
Validate that all tracks in an edition belong to the same artist (or related artists) to prevent multi-artist compilation matching.

**Acceptance Criteria:**
- Extract unique artist names from all tracks in edition
- If >3 unique artists detected, reject edition as multi-artist compilation
- Allow artist name variations (e.g., "The Beatles" vs "Beatles")
- Fuzzy matching: source artist name contained in track artist or vice versa
- Log rejection reason for debugging

**Rationale:**
Multi-artist compilations contain tracks from unrelated artists and should never match single-artist audio files. James Gang file matched to compilation containing Michael Jackson and Ian Anderson tracks.

**Problem Albums Addressed:**
- James Gang - Funk #49 (matched to multi-artist compilation with Michael Jackson, Ian Anderson tracks)

**Expected Impact:**
- Fixes 1 out of 4 problem albums
- Prevents future multi-artist compilation mismatches

---

### REQ-EF-050: Remix Track Error Tolerance

**Category:** Functional
**Priority:** Medium
**Source:** problem_albums_analysis.md:164-183

**Description:**
Accept larger duration errors (up to 120 seconds) for tracks identified as remixes, extended versions, or alternate mixes.

**Acceptance Criteria:**
- Detect remix tracks by keywords: "remix", "extended", "mix)", "version" in track title
- If remix detected AND abs(error) < 120.0 seconds, accept match
- Standard tolerance still applies to non-remix tracks
- Case-insensitive keyword matching
- Log when remix tolerance applied

**Rationale:**
Remix tracks have variable durations by nature. The "Kink extended remix" in Chemical Brothers - Surrender has -83.16s error, which is acceptable for a remix variant.

**Problem Albums Addressed:**
- Chemical Brothers - Surrender (1 track with -83s error on remix)

**Expected Impact:**
- Allows legitimate remix editions to pass validation
- No change for standard (non-remix) tracks

---

## Requirements Traceability

| Requirement | Problem Album(s) | Expected Fix | Verification Method |
|-------------|------------------|--------------|---------------------|
| REQ-EF-010 | Chemical Brothers, Imagine Dragons, Michael Jackson | Prefer standard editions | Test with deluxe vs standard MBIDs |
| REQ-EF-020 | James Gang, Michael Jackson | Reject compilations | Test with compilation MBIDs |
| REQ-EF-030 | Imagine Dragons, Michael Jackson | Filter track count mismatch | Test 11-track file against 16-track edition |
| REQ-EF-040 | James Gang | Reject multi-artist compilations | Test single-artist file against multi-artist edition |
| REQ-EF-050 | Chemical Brothers | Accept remix errors | Test remix track with 83s error |

---

## Implementation Notes

**Code Location:**
All changes will be in `wkmp-ai/src/matching/` module:
- `stages/mod.rs` or new `edition_filter.rs` module for filtering logic
- `stages/stage2_album_first.rs` to apply pre-filtering
- Shared scoring function used by all stages

**No Database Changes Required:**
All logic is in-memory filtering during matching process.

**No API Changes Required:**
Internal matching algorithm improvements only.

---

## Success Metrics

**Quantitative:**
- Fix 3 out of 4 problem albums (75% improvement)
- Reduce average track duration error by 10-15%
- No regressions on currently-passing albums (baseline: 186/200)

**Qualitative:**
- Deluxe editions deprioritized vs standard editions
- Multi-artist compilations rejected
- Remix tracks validated correctly
