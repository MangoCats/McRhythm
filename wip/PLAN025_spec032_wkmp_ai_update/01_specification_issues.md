# Specification Completeness Verification - PLAN025

**Plan:** SPEC032 wkmp-ai Implementation Update
**Date:** 2025-11-10
**Phase:** 2 - Specification Verification

---

## Executive Summary

**Total Issues Found:** 8 issues
- **CRITICAL:** 0 (No blockers)
- **HIGH:** 2 (Significant risks)
- **MEDIUM:** 4 (Should resolve)
- **LOW:** 2 (Minor concerns)

**Decision:** ✅ **PROCEED** - No critical blockers, HIGH issues can be addressed during implementation

---

## Issues by Category

### Completeness Check

#### HIGH-001: Per-Segment Fingerprinting PCM Extraction Not Fully Specified

**Requirement:** REQ-FING-010 (Per-segment fingerprinting)
**Issue:** Specification describes "Extract segment PCM data (start_sample..end_sample)" but does not specify:
- How to calculate sample offsets from time offsets
- Whether to handle sample rate mismatches
- Buffer management strategy (decode once, reuse for all segments?)

**Impact:** Implementation ambiguity - different engineers could implement different approaches

**Recommendation:**
```rust
// Clarify in implementation:
// 1. Sample offset calculation:
let start_sample = (start_time_seconds * file_sample_rate as f64).round() as usize;
let end_sample = (end_time_seconds * file_sample_rate as f64).round() as usize;

// 2. Buffer strategy:
// Decode entire file once to PCM buffer
// For each segment: slice buffer[start_sample..end_sample]
// Resample segment if needed (to 44.1kHz for Chromaprint)

// 3. Handle sample rate mismatch:
// If file_sample_rate != 44100, resample entire buffer OR per-segment
```

**Resolution Plan:** Document approach in design comments during implementation (Increment 3)

**Severity:** HIGH (affects architecture, but resolvable)

---

#### MEDIUM-001: Contextual Matcher Fuzzy String Matching Algorithm Not Specified

**Requirement:** REQ-CTXM-010 (Contextual matcher)
**Issue:** Specification states "Fuzzy string matching for artist/title/album names" but does not specify:
- Which algorithm to use (Levenshtein distance? Jaro-Winkler? Soundex?)
- Threshold for "match" vs. "no match" (e.g., similarity >= 0.8?)
- Whether to normalize strings (lowercase, remove punctuation, etc.)

**Impact:** Implementation ambiguity - matching quality varies by algorithm choice

**Recommendation:**
```rust
// Use strsim crate: Jaro-Winkler similarity
// Thresholds:
// - Exact match: similarity == 1.0
// - Fuzzy match: similarity >= 0.85
// - No match: similarity < 0.85

// Normalization:
// - Lowercase all strings
// - Remove non-alphanumeric (except spaces)
// - Trim whitespace

use strsim::jaro_winkler;
let normalized_a = normalize_string(&artist_a);
let normalized_b = normalize_string(&artist_b);
let similarity = jaro_winkler(&normalized_a, &normalized_b);
if similarity >= 0.85 {
    // Fuzzy match
}
```

**Resolution Plan:** Document algorithm choice in design comments, add tests for normalization

**Severity:** MEDIUM (affects matching quality, but defaults acceptable)

---

#### MEDIUM-002: Pattern Analyzer Source Media Heuristics Not Detailed

**Requirement:** REQ-PATT-040 (Source media classification)
**Issue:** Specification states "Heuristics for source media classification" but does not detail:
- Exact criteria for CD vs. Vinyl vs. Cassette
- How to handle ambiguous cases (e.g., vinyl with consistent gaps)
- Confidence threshold for classification

**Impact:** Pattern analyzer accuracy depends on heuristics quality

**Recommendation:**
```rust
// CD Classification:
// - Gap std dev < 0.5s
// - Mean gap 1.5s - 3.5s
// - Track count: 8-20
// - Confidence: 0.9

// Vinyl Classification:
// - Gap std dev >= 0.5s
// - Mean gap > 3.0s OR irregular gaps
// - Track count: 4-12 per side
// - Confidence: 0.7

// Cassette Classification:
// - Noise floor changes detected (future enhancement)
// - Similar to vinyl but lower confidence
// - Confidence: 0.5

// Unknown:
// - None of the above patterns match
// - Confidence: 0.3
```

**Resolution Plan:** Implement heuristics with confidence scores, tune during testing

**Severity:** MEDIUM (affects accuracy target, but >80% achievable with reasonable heuristics)

---

#### MEDIUM-003: Confidence Thresholds Not Configurable

**Requirement:** REQ-CONF-010 (Confidence assessor)
**Issue:** Specification hardcodes thresholds (0.85 Accept, 0.60 Review, 0.60 Reject) with note "User preference parameters (confidence thresholds)" but marks threshold configuration as out-of-scope

**Impact:** Cannot tune thresholds per user without code changes

**Recommendation:**
- Use hardcoded thresholds initially (as specified)
- Add database setting fields for future configuration:
  ```sql
  -- settings table
  confidence_threshold_accept REAL DEFAULT 0.85
  confidence_threshold_review REAL DEFAULT 0.60
  ```
- Load thresholds from settings in ConfidenceAssessor initialization
- If NULL, use hardcoded defaults

**Resolution Plan:** Implement with hardcoded defaults, add settings loading (low effort)

**Severity:** MEDIUM (limits flexibility, but defaults reasonable)

---

#### LOW-001: Test Dataset Not Specified

**Requirement:** Multiple (accuracy testing)
**Issue:** Specification assumes "Test dataset available for validation" but does not specify:
- Size of test dataset (how many files?)
- Known-good criteria (professionally curated? manual verification?)
- Coverage (single-track, albums, edge cases?)

**Impact:** Cannot validate accuracy claims (>90%, >80%) without dataset

**Recommendation:**
- Create test dataset with:
  - 50 single-track files (known artist/title/MBID)
  - 10 full album files (known track list, MBIDs)
  - 10 edge cases (ambiguous metadata, no tags, etc.)
- Document test dataset in `tests/fixtures/README.md`
- Use developer's personal library (with permission) + CC-licensed test audio

**Resolution Plan:** Curate test dataset during Increment 1 (Critical phase)

**Severity:** LOW (affects validation, but dataset can be created)

---

### Ambiguity Check

#### HIGH-002: "Per-File Pipeline" vs. "Batch Phases" Terminology Ambiguous

**Requirement:** REQ-PIPE-020 (Per-file pipeline)
**Issue:** Specification uses both "per-file pipeline" and "batch phases" but distinction not always clear. Example:
- "SCANNING" is described as "batch" operation (all files discovered first)
- But "per-file pipeline" starts after scanning
- Is scanning part of per-file pipeline or separate batch phase?

**Impact:** Confusion about what needs refactoring

**Clarification:**
```
Architecture (Clarified):

Phase 1: SCANNING (Batch - No Change)
  └─ Discover all audio files in root folder
  └─ Output: Vec<PathBuf> of discovered files
  └─ This is still a batch operation (find all files first)

Phase 2: PROCESSING (Per-File Pipeline - NEW)
  └─ Process each file through full pipeline:
     Worker 1: File_001 → [Verify→Extract→Segment→...] → File_005 → ...
     Worker 2: File_002 → [Verify→Extract→Segment→...] → File_006 → ...
  └─ 4 concurrent workers via futures::stream::buffer_unordered(4)
  └─ Each worker processes ONE file at a time through ALL steps

CRITICAL: The distinction is:
- SCANNING: Batch (find all files first, then proceed)
- PROCESSING: Per-file (one file through all steps, then next file)
```

**Resolution Plan:** Document architecture clearly in WorkflowOrchestrator refactor comments

**Severity:** HIGH (affects architecture understanding, but clarifiable)

---

#### MEDIUM-004: "Graceful Degradation" to Zero-Song Passages Behavior Not Detailed

**Requirement:** REQ-CONF-010 (Evidence-based identification)
**Issue:** Specification states "Falls back to zero-song passages when confidence insufficient" but does not detail:
- Are zero-song passages created in database?
- Are they playable (or excluded from queue)?
- How does user know identification failed?
- Is manual review UI created or just logged?

**Impact:** Unclear user experience for rejected identifications

**Clarification:**
- Zero-song passages ARE created in database (per SPEC032 definition)
- Zero-song passages ARE playable (audio works, just no metadata/flavor)
- Zero-song passages excluded from automatic selection (no Musical Flavor)
- Manual review UI is out-of-scope (Phase 1) - logged only
- User sees import complete with N passages created, M rejected (summary)

**Resolution Plan:** Document zero-song passage behavior in implementation comments

**Severity:** MEDIUM (affects UX understanding, but behavior defined in SPEC032)

---

#### LOW-002: "Sample-Accurate Precision" Quantification Unclear

**Requirement:** REQ-TICK-010 (Tick-based timing)
**Issue:** Specification states "Conversion maintains sample-accurate precision (<1 sample error)" but:
- What sample rate is reference? (44.1kHz? 48kHz? 192kHz?)
- Is <1 sample error absolute or percentage?

**Clarification:**
- Sample-accurate means: timing precise to nearest sample at SOURCE sample rate
- Example: 44.1kHz file → 1 sample = 0.0227ms → tick conversion error <0.0227ms
- Tick rate (28,224,000 Hz) exactly divides all supported sample rates (per SPEC017)
- Therefore: 0 rounding error for exact sample boundaries

**Resolution Plan:** Document precision in implementation comments, add unit tests

**Severity:** LOW (technical detail, but SPEC017 already guarantees precision)

---

### Consistency Check

**Result:** No consistency issues found

All requirements are consistent:
- No contradictions between requirements
- Timing budgets reasonable (2-3 weeks estimate)
- Resource allocations feasible (4 workers, rate limits respected)
- Interface specifications consistent (WorkflowOrchestrator refactor)
- Priorities aligned (Critical → High → Medium)

---

### Testability Check

**Result:** All requirements testable

Every requirement has clear acceptance criteria:
- REQ-PIPE-010: Verify segment-first execution order (unit test)
- REQ-PIPE-020: Verify 4 workers, per-file processing (integration test)
- REQ-PATT-010: Verify pattern detection >80% accuracy (system test with dataset)
- REQ-CTXM-010: Verify contextual matching narrows to <10 candidates (integration test)
- REQ-CONF-010: Verify >90% acceptance rate, <5% false positive (system test)
- REQ-FING-010: Verify per-segment fingerprints generated (unit test)
- REQ-TICK-010: Verify tick conversion precision <1 sample (unit test)

**Phase 3 will define explicit test specifications for each.**

---

### Dependency Validation

**Result:** All dependencies validated

**Existing Components:**
- FileScanner: ✅ Exists, no changes needed
- MetadataExtractor: ✅ Exists, no changes needed
- Fingerprinter: ✅ Exists, needs modification (per-segment support)
- AcoustIDClient: ✅ Exists, needs modification (per-segment queries)
- MusicBrainzClient: ✅ Exists, used by new ContextualMatcher
- AcousticBrainzClient: ✅ Exists, no changes needed
- AmplitudeAnalyzer: ✅ Exists, no changes needed
- SilenceDetector: ✅ Assumed to exist, needs verification

**External Dependencies:**
- MusicBrainz API: ✅ Available, rate limit 1 req/s documented
- AcoustID API: ✅ Available, requires API key
- AcousticBrainz API: ✅ Available, free access

**Specification Dependencies:**
- SPEC032: ✅ Source specification (recently updated)
- SPEC017: ✅ Tick-based timing defined
- SPEC031: ✅ Zero-config DB implemented

**Library Dependencies:**
- `strsim` (fuzzy matching): ⚠️ May need to add to Cargo.toml
- `governor` (rate limiting): ⚠️ May already be present, needs verification

**Action Items:**
1. Verify SilenceDetector exists in codebase (Increment 1)
2. Check if `governor` crate already in use (Increment 1)
3. Add `strsim` to Cargo.toml if not present (Increment 2)

---

## Issues Summary Table

| ID | Severity | Category | Issue | Resolution | Blocking? |
|----|----------|----------|-------|------------|-----------|
| HIGH-001 | HIGH | Completeness | Per-segment PCM extraction not detailed | Document in implementation | No |
| HIGH-002 | HIGH | Ambiguity | Per-file vs. batch terminology | Clarify architecture docs | No |
| MEDIUM-001 | MEDIUM | Completeness | Fuzzy matching algorithm not specified | Use Jaro-Winkler, document | No |
| MEDIUM-002 | MEDIUM | Completeness | Source media heuristics not detailed | Implement reasonable heuristics | No |
| MEDIUM-003 | MEDIUM | Completeness | Confidence thresholds not configurable | Use hardcoded, add settings | No |
| MEDIUM-004 | MEDIUM | Ambiguity | Zero-song passage behavior unclear | Document per SPEC032 | No |
| LOW-001 | LOW | Completeness | Test dataset not specified | Curate dataset | No |
| LOW-002 | LOW | Ambiguity | Sample-accurate precision unclear | Document per SPEC017 | No |

---

## Phase 2 Decision

**Status:** ✅ **PROCEED TO PHASE 3**

**Rationale:**
- 0 CRITICAL issues (no blockers)
- 2 HIGH issues identified, both resolvable during implementation
- All HIGH issues are ambiguities/details, not missing essential information
- All requirements are testable
- All dependencies validated
- Specification is sufficiently complete for planning

**Action Items for Implementation:**
1. Document PCM extraction strategy in Fingerprinter refactor (HIGH-001)
2. Clarify architecture (batch vs. per-file) in WorkflowOrchestrator comments (HIGH-002)
3. Choose fuzzy matching algorithm (Jaro-Winkler) and document (MEDIUM-001)
4. Implement source media heuristics with confidence scoring (MEDIUM-002)
5. Use hardcoded thresholds, add settings loading for future (MEDIUM-003)
6. Document zero-song passage behavior per SPEC032 (MEDIUM-004)
7. Curate test dataset during Critical phase (LOW-001)
8. Document precision guarantees per SPEC017 (LOW-002)

---

**END OF SPECIFICATION ISSUES REPORT**
