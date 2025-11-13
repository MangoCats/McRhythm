# SPEC032 Implementation Update Specification

**Document Type:** Implementation Planning Input
**Status:** Draft - Ready for /plan
**Created:** 2025-11-10
**Target Module:** wkmp-ai (Audio Ingest microservice)

---

## Document Purpose

This specification defines the required updates to wkmp-ai to implement the revised SPEC032 architecture, which introduces an **intelligence-gathering, segmentation-first approach** to audio file import with evidence-based MBID identification.

**Intended Use:** Input to `/plan` workflow for detailed implementation planning with test-driven development.

---

## Executive Summary

### What Changed in SPEC032

**Major Architectural Shift:** The import pipeline sequence has been fundamentally redesigned from a **fingerprint-first** approach to a **segmentation-first, evidence-based** approach.

**Old Pipeline (Current wkmp-ai Implementation):**
```
SCANNING → EXTRACTING → FINGERPRINTING → SEGMENTING → ANALYZING → FLAVORING
```

**New Pipeline (SPEC032 Revision):**
```
SCANNING → Per-File Pipeline:
  1. Verify audio format
  2. Extract metadata (ID3 + filename/folder context)
  3. Calculate file hash
  4. SEGMENT FIRST (silence detection → structural clues)
  5. Contextual MusicBrainz search (metadata + pattern)
  6. Per-segment Chromaprint fingerprinting
  7. Per-segment AcoustID lookup
  8. Evidence-based MBID confidence assessment
  9. Amplitude analysis per passage
  10. AcousticBrainz flavor retrieval (only confirmed MBIDs)
  11. Convert timing to ticks and write to database
```

**Key Innovations:**
1. **Segmentation before fingerprinting** - Structural analysis (track count, gap patterns) provides contextual clues
2. **Contextual matching** - Metadata + segment patterns narrow MusicBrainz candidates before expensive fingerprinting
3. **Per-segment fingerprinting** - Individual segment fingerprints more accurate than whole-file for multi-track files
4. **Evidence-based identification** - Combines metadata (30%), pattern (10%), and fingerprints (60%) for high-confidence MBID matching
5. **Graceful degradation** - Falls back to zero-song passages when confidence insufficient

### Impact on Current Implementation

**Current State:**
- wkmp-ai implements **batch-phase architecture** (all files through each phase sequentially)
- Fingerprinting occurs **before** segmentation
- No pattern analyzer component
- No contextual matcher component
- No confidence assessor component
- Whole-file fingerprinting (not per-segment)

**Required Changes:**
- ✅ Already implements: Per-file pipeline with parallel workers (4 concurrent)
- ❌ **Critical gap:** Pipeline sequence is incorrect (fingerprint before segment)
- ❌ **Missing:** Pattern analyzer (structural analysis of segments)
- ❌ **Missing:** Contextual matcher (metadata + pattern → MusicBrainz search)
- ❌ **Missing:** Confidence assessor (evidence combination algorithm)
- ❌ **Missing:** Per-segment fingerprinting architecture
- ❌ **Missing:** Tick-based timing conversion (SPEC017 integration)
- ✅ Already implements: Zero-config database initialization (SPEC031)

---

## Current Implementation Analysis

### Existing Architecture (wkmp-ai/src/services/workflow_orchestrator/mod.rs)

**State Machine:**
```rust
// Current implementation (lines 5-6)
// SCANNING → EXTRACTING → FINGERPRINTING → SEGMENTING → ANALYZING → FLAVORING → COMPLETED
```

**Phase Modules:**
- `phase_scanning.rs` - File discovery (batch operation)
- `phase_extraction.rs` - ID3 metadata extraction (batch)
- `phase_fingerprinting.rs` - Chromaprint + AcoustID (batch, **happens before segmentation**)
- `phase_segmenting.rs` - Silence detection (batch)
- `phase_analyzing.rs` - Amplitude analysis (batch)
- `phase_flavoring.rs` - AcousticBrainz/Essentia (batch)

**Existing Components:**
- ✅ `FileScanner` - Directory traversal, file discovery
- ✅ `MetadataExtractor` - ID3/Vorbis/MP4 tag parsing
- ✅ `Fingerprinter` - Chromaprint generation (via FFI)
- ✅ `AcoustIDClient` - AcoustID API client
- ✅ `MusicBrainzClient` - MusicBrainz API client
- ✅ `AcousticBrainzClient` - AcousticBrainz API client
- ✅ `AmplitudeAnalyzer` - RMS analysis, lead-in/lead-out detection
- ✅ `SilenceDetector` (assumed) - Silence boundary detection
- ❌ **Missing:** `PatternAnalyzer` - Segment pattern analysis
- ❌ **Missing:** `ContextualMatcher` - MusicBrainz search with metadata+pattern
- ❌ **Missing:** `ConfidenceAssessor` - Evidence combination

### Gap Analysis

| Requirement | SPEC032 Reference | Current State | Gap |
|-------------|-------------------|---------------|-----|
| **Segmentation-first pipeline** | Lines 344-375 | Fingerprinting happens first | **CRITICAL** - Reorder pipeline |
| **Pattern analyzer component** | Lines 149-180 | Not implemented | **HIGH** - New component |
| **Contextual matcher component** | Lines 182-237 | Not implemented | **HIGH** - New component |
| **Confidence assessor component** | Lines 239-321 | Not implemented | **HIGH** - New component |
| **Per-segment fingerprinting** | Lines 355-358, 694-741 | Whole-file only | **HIGH** - Architecture change |
| **Tick-based timing** | Lines 773-797, 568-571 | Not implemented | **MEDIUM** - Add conversion |
| **Zero-config DB init** | Lines 92-110 | Implemented | ✅ **COMPLETE** |
| **Per-file pipeline** | Lines 232-250, 543-578 | Implemented (batch phases) | **MEDIUM** - Refactor to per-file |

---

## Required Changes

### 1. Pipeline Sequence Reordering (CRITICAL)

**Current Sequence:**
```
SCANNING → EXTRACTING → FINGERPRINTING → SEGMENTING → ANALYZING → FLAVORING
```

**Required Sequence:**
```
SCANNING → PROCESSING (per-file pipeline):
  Verify → Extract → Hash → SEGMENT → Match → Fingerprint → Identify → Amplitude → Flavor → DB
```

**Implementation Impact:**
- **Refactor:** `workflow_orchestrator/mod.rs` state machine
- **Refactor:** Phase modules into per-file pipeline functions
- **Reorder:** Move segmentation BEFORE fingerprinting
- **Add:** Contextual matching step BEFORE fingerprinting
- **Add:** Evidence-based identification AFTER fingerprinting

**SPEC032 References:**
- Pipeline sequence: Lines 344-375
- State machine: Lines 143-178
- Rationale: Lines 370-377

---

### 2. New Component: Pattern Analyzer (HIGH PRIORITY)

**Purpose:** Analyze structural patterns in segmented audio to provide contextual clues for identification (SPEC032:149-180)

**Inputs:**
- List of detected segments (count, start times, end times, gap durations)
- Audio file metadata (total duration, format)

**Analysis Performed:**
- **Track count:** Number of segments detected
- **Gap patterns:** Consistent (CD) vs. variable (vinyl/cassette) gaps
- **Segment durations:** Statistical analysis (mean, variance)
- **Likely source media:** CD / Vinyl / Cassette / Unknown

**Outputs:**
```rust
pub struct PatternMetadata {
    pub track_count: usize,
    pub likely_source_media: SourceMedia,  // CD/Vinyl/Cassette/Unknown
    pub gap_pattern: GapPattern,            // Consistent/Variable/None
    pub segment_durations: Vec<f64>,
    pub confidence: f64,                    // 0.0-1.0
}
```

**Example:**
```
Input: 12 segments, 2.1s gaps (±0.3s), durations 180-360s
Output: {
  track_count: 12,
  likely_source_media: CD,
  gap_pattern: Consistent,
  confidence: 0.92
}
```

**Implementation Considerations:**
- Statistical analysis of gap durations (mean, standard deviation)
- Heuristics for source media classification
- Integration with existing silence detection results

**SPEC032 Reference:** Lines 149-180

---

### 3. New Component: Contextual Matcher (HIGH PRIORITY)

**Purpose:** Search MusicBrainz database using combined metadata and structural pattern clues to narrow candidate list before fingerprinting (SPEC032:182-237)

**Inputs:**
- Extracted metadata (ID3 tags, filename, folder structure)
- Pattern analysis results (track count, source media type)
- Audio file characteristics (total duration, format)

**Matching Strategy:**

**For Single-Segment Files:**
```
1. Parse metadata: artist, title, album from ID3 tags
2. Search MusicBrainz:
   - Query: artist + title (exact and fuzzy match)
   - Filter by duration (±10% tolerance)
3. Return ranked candidate recordings with match scores
```

**For Multi-Segment Files:**
```
1. Parse metadata: album artist, album title, folder structure
2. Identify likely album structure:
   - If 12 tracks + CD pattern → likely full album CD rip
   - If 6-8 tracks + vinyl pattern → likely vinyl side rip
3. Search MusicBrainz releases:
   - Query: artist + album (exact and fuzzy match)
   - Filter by track count (exact match or ±1 tolerance)
   - Filter by total duration (±5% tolerance)
4. For each candidate release:
   - Fetch track list (track count, track durations)
   - Calculate alignment score:
     * Track count match: 40% weight
     * Duration alignment: 30% weight
     * Metadata quality: 30% weight
5. Return ranked candidate releases with match scores (0.0-1.0)
```

**Outputs:**
```rust
// For single-segment files
pub type SingleFileMatches = Vec<(RecordingMBID, MatchScore)>;

// For multi-segment files
pub type MultiFileMatches = Vec<(ReleaseMBID, TrackList, MatchScore)>;
```

**Implementation Considerations:**
- MusicBrainz API rate limiting (1 req/s)
- Fuzzy string matching for artist/title/album names
- Duration tolerance calculations
- Track list alignment algorithms
- Caching of search results

**SPEC032 Reference:** Lines 182-237

---

### 4. New Component: Confidence Assessor (HIGH PRIORITY)

**Purpose:** Combine evidence from multiple sources (metadata, pattern, fingerprints) to make high-confidence MBID identification decisions (SPEC032:239-321)

**Inputs:**
- Metadata match scores (from contextual matcher)
- Pattern match scores (from pattern analyzer)
- Fingerprint match scores (from AcoustID API, per segment)
- User preference parameters (confidence thresholds)

**Evidence Combination Algorithm:**

**For Single-Segment Files:**
```rust
// Collect evidence
let metadata_score: f64 = ...; // 0.0-1.0 from contextual matcher
let fingerprint_score: f64 = ...; // 0.0-1.0 from AcoustID API
let duration_match: f64 = ...; // 0.0-1.0 (actual vs. expected)

// Weighted combination
let confidence = (0.3 * metadata_score) + (0.6 * fingerprint_score) + (0.1 * duration_match);

// Decision
let decision = match confidence {
    c if c >= 0.85 => Decision::Accept,   // High confidence
    c if c >= 0.60 => Decision::Review,   // Manual verification recommended
    _ => Decision::Reject,                // Insufficient confidence, zero-song passage
};
```

**For Multi-Segment Files:**
```rust
// Collect evidence per segment
let contextual_score: f64 = ...; // Release-level match score
let pattern_score: f64 = ...; // Track count + duration alignment
let fingerprint_scores: Vec<f64> = ...; // Per-segment AcoustID scores

// Per-segment scoring
let segment_confidence: Vec<f64> = fingerprint_scores.iter()
    .map(|fp_score| {
        (0.2 * contextual_score) +
        (0.2 * pattern_score) +
        (0.6 * fp_score)
    })
    .collect();

// Overall confidence
let consistency_bonus = if all_segments_match_same_release() { 1.0 } else { 0.8 };
let overall_confidence = segment_confidence.iter().sum::<f64>() / segment_confidence.len() as f64 * consistency_bonus;

// Decision
let min_segment_confidence = segment_confidence.iter().min().unwrap();
let decision = match (overall_confidence, min_segment_confidence) {
    (oc, msc) if oc >= 0.85 && *msc >= 0.70 => Decision::Accept,
    (oc, _) if oc >= 0.65 => Decision::Review,
    _ => Decision::Reject,
};
```

**Outputs:**
```rust
pub struct IdentificationResult {
    pub mbid: Option<RecordingMBID>,  // Some if accepted, None if rejected
    pub confidence: f64,               // 0.0-1.0
    pub decision: Decision,            // Accept/Review/Reject
    pub evidence_summary: EvidenceSummary,
}

pub struct EvidenceSummary {
    pub contextual: f64,
    pub pattern: f64,
    pub fingerprint_mean: f64,
    pub fingerprint_min: f64,
}
```

**Implementation Considerations:**
- Configurable confidence thresholds (per-user settings)
- Logging of evidence breakdown for debugging
- Zero-song passage creation for rejected identifications
- Review queue management for manual verification

**SPEC032 Reference:** Lines 239-321

---

### 5. Per-Segment Fingerprinting (HIGH PRIORITY)

**Current Implementation:** Whole-file fingerprinting
**Required:** Per-segment fingerprinting after segmentation

**Changes Required:**

**Architectural Shift:**
```rust
// CURRENT (whole-file)
fn fingerprint_file(file_path: &Path) -> Result<String> {
    // Generate single fingerprint for entire file
}

// REQUIRED (per-segment)
fn fingerprint_segments(
    file_path: &Path,
    segments: &[(f64, f64)]  // (start_time, end_time) pairs
) -> Result<Vec<String>> {
    // Generate fingerprint for EACH segment individually
    segments.iter()
        .map(|(start, end)| fingerprint_segment(file_path, *start, *end))
        .collect()
}
```

**Implementation Strategy:**
1. Decode audio file once to PCM
2. For each segment:
   - Extract segment PCM data (start_sample..end_sample)
   - Resample to 44.1kHz if needed
   - Generate Chromaprint fingerprint via FFI
   - Store fingerprint with segment association
3. Query AcoustID API once per segment (rate-limited)

**Performance Considerations:**
- Decode once, segment multiple times (avoid re-decoding)
- CHROMAPRINT_LOCK mutex for thread safety (FFI backend)
- Parallel per-segment processing across workers (not within single file)
- Cache per-segment fingerprints to avoid reprocessing

**SPEC032 References:**
- Pipeline step: Lines 355-358
- Detailed design: Lines 694-741

---

### 6. Tick-Based Timing Conversion (MEDIUM PRIORITY)

**Current Implementation:** Timing in floating-point seconds
**Required:** Timing in INTEGER ticks per SPEC017

**Conversion Formula:**
```rust
const TICK_RATE: i64 = 28_224_000; // Hz

fn seconds_to_ticks(seconds: f64) -> i64 {
    (seconds * TICK_RATE as f64).round() as i64
}
```

**Database Fields Affected:**
- `start_time_ticks` - Passage start point
- `lead_in_start_ticks` - Lead-in fade start
- `fade_in_start_ticks` - Fade-in start
- `fade_in_end_ticks` - Fade-in end (music at full volume)
- `fade_out_start_ticks` - Fade-out start (begin crossfade)
- `lead_out_start_ticks` - Lead-out start
- `end_time_ticks` - Passage end point

**Implementation Points:**
1. Convert all detected timing points (seconds → ticks) before database write
2. Apply to segmentation results (silence detection boundaries)
3. Apply to amplitude analysis results (lead-in/lead-out points)
4. Verify sample-accurate precision maintained

**SPEC032 References:**
- Integration section: Lines 773-797
- Pipeline step: Lines 568-571
- Component matrix: Lines 138, 146

---

### 7. Per-File Pipeline Refactoring (MEDIUM PRIORITY)

**Current:** Batch-phase architecture (all files through each phase)
**Required:** Per-file pipeline with parallel workers

**Current Architecture:**
```
Phase 1 (EXTRACTING): Process all 5,736 files → Wait for all to complete
Phase 2 (FINGERPRINTING): Process all 5,736 files → Wait for all to complete
Phase 3 (SEGMENTING): Process all 5,736 files → Wait for all to complete
...
```

**Required Architecture:**
```
Worker 1: File_001 → [Full pipeline] → Complete → File_005 → ...
Worker 2: File_002 → [Full pipeline] → Complete → File_006 → ...
Worker 3: File_003 → [Full pipeline] → Complete → File_007 → ...
Worker 4: File_004 → [Full pipeline] → Complete → File_008 → ...
```

**Implementation Strategy:**
```rust
async fn process_file_complete(
    file: &AudioFile,
    db: &SqlitePool,
    // ... services
) -> Result<ProcessedFileResult> {
    // 1. Verify audio format
    // 2. Extract metadata
    // 3. Calculate hash
    // 4. Segment (silence detection)
    // 5. Analyze pattern
    // 6. Contextual match
    // 7. Per-segment fingerprint
    // 8. Per-segment AcoustID
    // 9. Confidence assessment
    // 10. Amplitude analysis
    // 11. AcousticBrainz flavor
    // 12. Convert to ticks and write DB
    Ok(result)
}

// Parallel execution
let results = stream::iter(discovered_files)
    .map(|file| process_file_complete(file, db, /* services */))
    .buffer_unordered(4)  // 4 concurrent workers
    .collect()
    .await;
```

**Benefits:**
- Better progress granularity (files completed, not phases)
- Better resource utilization (CPU/network overlap)
- Easier cancellation (file boundaries)
- Natural idempotency (process incomplete files only)

**SPEC032 References:**
- Per-file pipeline: Lines 543-578
- Architecture: Lines 232-250
- Benefits: Lines 263-328

---

## Implementation Priorities

### Phase 1: Pipeline Reordering (CRITICAL)
**Priority:** P0 (Blocking)
**Effort:** High (2-3 days)
**Dependencies:** None

1. Refactor `workflow_orchestrator/mod.rs` to per-file pipeline architecture
2. Move segmentation BEFORE fingerprinting in pipeline sequence
3. Update state machine to reflect new pipeline
4. Preserve existing component functionality

**Acceptance Criteria:**
- Pipeline processes files individually (not batch phases)
- Segmentation occurs before fingerprinting
- All existing tests pass with new sequence

### Phase 2: New Intelligence-Gathering Components (HIGH)
**Priority:** P1 (High Impact)
**Effort:** High (4-5 days)
**Dependencies:** Phase 1

1. Implement `PatternAnalyzer` component
2. Implement `ContextualMatcher` component
3. Implement `ConfidenceAssessor` component
4. Integrate into per-file pipeline

**Acceptance Criteria:**
- Pattern analyzer detects source media type with >80% accuracy
- Contextual matcher narrows MusicBrainz candidates to <10 results
- Confidence assessor achieves >90% acceptance rate for known-good files
- Integration tests demonstrate evidence-based identification

### Phase 3: Per-Segment Fingerprinting (HIGH)
**Priority:** P1 (High Impact)
**Effort:** Medium (2-3 days)
**Dependencies:** Phase 1, Phase 2

1. Refactor fingerprinting to support per-segment operation
2. Update AcoustID client for per-segment queries
3. Implement segment PCM extraction
4. Add per-segment result aggregation

**Acceptance Criteria:**
- Per-segment fingerprints generated for multi-track files
- AcoustID queries execute per segment with rate limiting
- Fingerprint accuracy improves for album-length files

### Phase 4: Tick-Based Timing (MEDIUM)
**Priority:** P2 (Medium Impact)
**Effort:** Low (1 day)
**Dependencies:** None (can parallelize)

1. Implement `seconds_to_ticks()` conversion function
2. Update database write operations to convert timing
3. Verify sample-accurate precision maintained
4. Update tests to validate tick-based timing

**Acceptance Criteria:**
- All passage timing stored as INTEGER ticks
- Conversion maintains sample-accurate precision (<1 sample error)
- Database writes succeed with tick-based timing

---

## Testing Strategy

### Unit Tests
- Pattern analyzer: Statistical analysis algorithms
- Contextual matcher: MusicBrainz search logic
- Confidence assessor: Evidence combination algorithm
- Per-segment fingerprinting: Segment extraction and fingerprint generation
- Tick conversion: Seconds-to-ticks precision

### Integration Tests
- Per-file pipeline: Complete pipeline execution for single file
- Evidence-based identification: End-to-end MBID identification
- Database writes: Tick-based timing in database

### System Tests
- Small library import (10 files, known MBIDs)
- Album-length file import (multi-segment)
- Mixed library import (single-track + album files)
- Confidence assessment accuracy (known-good vs. ambiguous files)

### Performance Tests
- Per-file pipeline throughput (files/second)
- Contextual matching speedup (vs. blind fingerprinting)
- Per-segment fingerprinting overhead (vs. whole-file)

---

## Risk Assessment

### High-Risk Areas

**Risk 1: Per-Segment Fingerprinting Performance**
- **Failure Mode:** Per-segment fingerprinting significantly slower than whole-file
- **Probability:** Medium
- **Impact:** Medium (longer import times)
- **Mitigation:** Optimize PCM extraction, cache decoded audio, parallel per-segment processing
- **Residual Risk:** Low

**Risk 2: Contextual Matching Accuracy**
- **Failure Mode:** Contextual matcher fails to narrow candidates effectively
- **Probability:** Medium
- **Impact:** High (no benefit over existing approach)
- **Mitigation:** Implement fuzzy string matching, tune tolerance thresholds, extensive testing
- **Residual Risk:** Low-Medium

**Risk 3: Evidence-Based Identification False Positives**
- **Failure Mode:** Confidence assessor accepts incorrect MBID matches
- **Probability:** Low
- **Impact:** High (incorrect metadata in database)
- **Mitigation:** Conservative confidence thresholds, extensive validation, manual review queue
- **Residual Risk:** Low

### Medium-Risk Areas

**Risk 4: Pipeline Reordering Regression**
- **Failure Mode:** New pipeline breaks existing functionality
- **Probability:** Low
- **Impact:** High (import completely broken)
- **Mitigation:** Comprehensive testing, gradual rollout, feature flag
- **Residual Risk:** Low

---

## Success Criteria

### Functional Requirements
- ✅ Pipeline processes files in correct sequence (segment → match → fingerprint → identify)
- ✅ Pattern analyzer detects source media type
- ✅ Contextual matcher narrows MusicBrainz candidates
- ✅ Confidence assessor combines evidence for MBID identification
- ✅ Per-segment fingerprinting implemented
- ✅ Tick-based timing conversion applied
- ✅ Zero-config database initialization preserved

### Performance Requirements
- ✅ Import throughput: ≥20 files/second (4 workers, average 3-minute songs)
- ✅ Contextual matching: <2 seconds per file (MusicBrainz search)
- ✅ Per-segment fingerprinting overhead: <20% vs. whole-file

### Quality Requirements
- ✅ MBID identification accuracy: >90% for known-good files
- ✅ False positive rate: <5% (incorrect MBID accepted)
- ✅ Zero-song passage fallback: 100% for rejected identifications
- ✅ Test coverage: >80% for new components

---

## Related Documentation

- **SPEC032:** Audio Ingest Architecture (revised)
- **SPEC017:** Sample Rate Conversion (tick-based timing)
- **SPEC031:** Data-Driven Schema Maintenance (zero-config DB)
- **IMPL005:** Audio File Segmentation
- **SPEC025:** Amplitude Analysis
- **SPEC008:** Library Management

---

## Next Steps

This specification is ready for `/plan` workflow to generate:
1. Detailed implementation plan with phased increments
2. Acceptance test specifications (Given/When/Then format)
3. Test-first development approach
4. Traceability matrix (requirements → tests → implementation)

**To proceed:**
```
/plan wip/SPEC032_IMPLEMENTATION_UPDATE.md
```

---

**END OF SPECIFICATION**
