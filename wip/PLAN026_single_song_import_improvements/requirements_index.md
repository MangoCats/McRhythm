# Requirements Index: Single Song Import Improvements

**Source:** Analysis of wkmp-ai single-song import gaps (2025-12-13)
**Plan:** PLAN026

---

## Requirements Summary

| Req ID | Type | Brief Description | Priority | Source |
|--------|------|-------------------|----------|--------|
| SSI-MB-010 | Functional | MusicBrainz recording search fallback when AcoustID fails | P0 | Gap Analysis |
| SSI-MB-020 | Functional | Query by artist + title + duration tolerance | P0 | Gap Analysis |
| SSI-MB-030 | Functional | Return sorted candidates with similarity scores | P1 | Gap Analysis |
| SSI-VAL-010 | Functional | Upgrade metadata validation to Jaro-Winkler | P0 | Gap Analysis |
| SSI-VAL-020 | Functional | Normalize strings before comparison (lowercase, strip "The", punctuation) | P0 | Gap Analysis |
| SSI-VAL-030 | Functional | Apply 0.85 threshold for title, 0.80 for artist | P1 | Gap Analysis |
| SSI-FUS-010 | Functional | Integrate IdentityResolver into single-song path | P1 | Gap Analysis |
| SSI-FUS-020 | Functional | Combine AcoustID + MusicBrainz + ID3 sources | P1 | Gap Analysis |
| SSI-FUS-030 | Functional | Use Bayesian posterior for confidence scoring | P1 | Gap Analysis |
| SSI-CTX-010 | Functional | Folder context awareness for album-ripped singles | P2 | Gap Analysis |
| SSI-CTX-020 | Functional | Query sibling files' album MBID for track matching | P2 | Gap Analysis |
| SSI-SEG-010 | Functional | Per-segment fingerprinting for dual-path files | P2 | Gap Analysis |
| SSI-SEG-020 | Functional | Enable EP and mixed-content detection | P2 | Gap Analysis |
| SSI-INT-010 | Integration | Integrate with ContentTypeClassifier | P0 | Architecture |
| SSI-INT-020 | Integration | Maintain rate limits (MB: 1/sec, AcoustID: 3/sec) | P0 | Architecture |
| SSI-INT-030 | Integration | Cache MusicBrainz recording lookups | P1 | Architecture |
| SSI-QUA-010 | Quality | Unit tests for recording matcher | P0 | Quality |
| SSI-QUA-020 | Quality | Integration tests for multi-source fusion | P1 | Quality |
| SSI-QUA-030 | Quality | Test coverage ≥80% for new code | P1 | Quality |

---

## Requirement Details

### SSI-MB-010: MusicBrainz Recording Search Fallback
**Type:** Functional | **Priority:** P0 (Critical)

**SHALL:** When AcoustID lookup fails or returns low confidence (<0.80), the system SHALL query MusicBrainz directly for matching recordings.

**Rationale:** Current implementation has no fallback when AcoustID fails - files are marked NOT_IN_MUSICBRAINZ even when metadata could identify them.

---

### SSI-MB-020: Query by Artist + Title + Duration
**Type:** Functional | **Priority:** P0 (Critical)

**SHALL:** The MusicBrainz recording search SHALL query using:
- Artist name (from ID3 or filename)
- Recording title (from ID3 or filename)
- Duration tolerance ±10 seconds

**Query format:** `artist:"X" AND recording:"Y" AND dur:[min TO max]`

---

### SSI-MB-030: Return Sorted Candidates
**Type:** Functional | **Priority:** P1 (High)

**SHALL:** The recording matcher SHALL return candidates sorted by combined similarity score (Jaro-Winkler artist + title).

---

### SSI-VAL-010: Jaro-Winkler Validation
**Type:** Functional | **Priority:** P0 (Critical)

**SHALL:** Replace Levenshtein edit distance with Jaro-Winkler similarity for artist/title matching.

**Rationale:** Levenshtein fails for long strings with minor variations. Jaro-Winkler is already used in album matcher (am29).

---

### SSI-VAL-020: String Normalization
**Type:** Functional | **Priority:** P0 (Critical)

**SHALL:** Before comparison, normalize strings by:
- Converting to lowercase
- Removing leading "The ", "A ", "An "
- Removing punctuation
- Folding Unicode to ASCII equivalents

---

### SSI-VAL-030: Similarity Thresholds
**Type:** Functional | **Priority:** P1 (High)

**SHALL:** Accept matches when:
- Title similarity ≥ 0.85
- Artist similarity ≥ 0.80

---

### SSI-FUS-010: IdentityResolver Integration
**Type:** Functional | **Priority:** P1 (High)

**SHALL:** The single-song classification path SHALL use IdentityResolver for Bayesian fusion of multiple MBID sources.

---

### SSI-FUS-020: Multi-Source Fusion
**Type:** Functional | **Priority:** P1 (High)

**SHALL:** The fusion process SHALL combine:
1. AcoustID result (if available)
2. MusicBrainz recording search result (if available)
3. ID3 metadata match (if available)

---

### SSI-FUS-030: Bayesian Posterior Scoring
**Type:** Functional | **Priority:** P1 (High)

**SHALL:** Final confidence SHALL be computed using Bayesian update:
```
posterior = 1 - (1 - c1) * (1 - c2) * ... * (1 - cN)
```
where c1...cN are confidence scores from agreeing sources.

---

### SSI-CTX-010: Folder Context Awareness
**Type:** Functional | **Priority:** P2 (Medium)

**SHALL:** When processing a single-track file in a folder with multiple audio files:
- Check if sibling files have been imported with an album MBID
- Use that album context to assist identification

---

### SSI-CTX-020: Album Track Matching
**Type:** Functional | **Priority:** P2 (Medium)

**SHALL:** Query the album's track listing from MusicBrainz and match current file's title/duration against album tracks.

---

### SSI-SEG-010: Per-Segment Fingerprinting
**Type:** Functional | **Priority:** P2 (Medium)

**SHALL:** For files in dual-path range (12-25 min), generate separate fingerprints for each detected segment.

---

### SSI-SEG-020: EP/Mixed Content Detection
**Type:** Functional | **Priority:** P2 (Medium)

**SHALL:** Use per-segment fingerprinting to enable:
- EP detection (3-5 songs)
- Mixed content files (some identified, others not)
- MULTIPLE_SONGS classification

---

### SSI-INT-010: ContentTypeClassifier Integration
**Type:** Integration | **Priority:** P0 (Critical)

**SHALL:** New recording matcher SHALL integrate with existing ContentTypeClassifier in content_type_classifier.rs.

---

### SSI-INT-020: Rate Limit Compliance
**Type:** Integration | **Priority:** P0 (Critical)

**SHALL:** Maintain existing rate limits:
- MusicBrainz: 1 request per second (1550ms delay)
- AcoustID: 3 requests per second (334ms delay)

---

### SSI-INT-030: Recording Cache
**Type:** Integration | **Priority:** P1 (High)

**SHALL:** Cache MusicBrainz recording lookup results to avoid redundant API calls.

---

### SSI-QUA-010: Unit Test Coverage
**Type:** Quality | **Priority:** P0 (Critical)

**SHALL:** Provide unit tests for:
- Recording matcher query building
- Jaro-Winkler string comparison
- Candidate scoring and sorting

---

### SSI-QUA-020: Integration Tests
**Type:** Quality | **Priority:** P1 (High)

**SHALL:** Provide integration tests for:
- Multi-source fusion workflow
- Fallback from AcoustID to MusicBrainz
- End-to-end single-song classification

---

### SSI-QUA-030: Coverage Target
**Type:** Quality | **Priority:** P1 (High)

**SHALL:** New code SHALL achieve ≥80% test coverage.

---

## Priority Summary

| Priority | Count | Description |
|----------|-------|-------------|
| P0 | 8 | Critical - Core functionality required |
| P1 | 8 | High - Important for quality/reliability |
| P2 | 4 | Medium - Enhanced functionality |
| **Total** | **20** | |
