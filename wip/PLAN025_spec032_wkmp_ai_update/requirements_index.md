# Requirements Index - PLAN025

**Plan:** SPEC032 wkmp-ai Implementation Update
**Source:** wip/SPEC032_IMPLEMENTATION_UPDATE.md
**Date:** 2025-11-10

---

## Requirements Summary

**Total Requirements:** 12 functional + architectural requirements
- **P0 (Critical):** 2 requirements
- **P1 (High):** 6 requirements
- **P2 (Medium):** 4 requirements

---

## Requirements Table

| Req ID | Priority | Type | Brief Description | Source Line | Component |
|--------|----------|------|-------------------|-------------|-----------|
| **REQ-PIPE-010** | P0 | Architectural | Segmentation-first pipeline sequence | 122-146 | WorkflowOrchestrator |
| **REQ-PIPE-020** | P0 | Architectural | Per-file pipeline (not batch phases) | 116, 419-465 | WorkflowOrchestrator |
| **REQ-PATT-010** | P1 | Functional | Pattern analyzer component | 149-191 | PatternAnalyzer (new) |
| **REQ-CTXM-010** | P1 | Functional | Contextual matcher component | 194-250 | ContextualMatcher (new) |
| **REQ-CONF-010** | P1 | Functional | Confidence assessor component | 253-328 | ConfidenceAssessor (new) |
| **REQ-FING-010** | P1 | Architectural | Per-segment fingerprinting | 331-393 | Fingerprinter |
| **REQ-TICK-010** | P2 | Functional | Tick-based timing conversion | 396-430 | All components |
| **REQ-PATT-020** | P2 | Functional | Track count detection | 157-161 | PatternAnalyzer |
| **REQ-PATT-030** | P2 | Functional | Gap pattern analysis | 159 | PatternAnalyzer |
| **REQ-PATT-040** | P2 | Functional | Source media classification | 161 | PatternAnalyzer |
| **REQ-CTXM-020** | P1 | Functional | Single-segment file matching | 205-212 | ContextualMatcher |
| **REQ-CTXM-030** | P1 | Functional | Multi-segment file matching | 214-231 | ContextualMatcher |

---

## Detailed Requirements

### REQ-PIPE-010: Segmentation-First Pipeline Sequence (P0 - CRITICAL)

**Description:** The import pipeline MUST execute segmentation BEFORE fingerprinting to leverage structural clues for identification.

**Current State:** Fingerprinting happens before segmentation (batch phase architecture)

**Required Sequence:**
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

**Acceptance Criteria:**
- Silence detection executes before any fingerprinting
- Segment boundaries available to fingerprinting step
- Pattern analysis results available to contextual matching
- All tests verify segment-first execution order

**Source:** Lines 122-146
**SPEC032 Reference:** Lines 344-375, 143-178

---

### REQ-PIPE-020: Per-File Pipeline Architecture (P0 - CRITICAL)

**Description:** The import workflow MUST process each file through the complete pipeline with parallel workers, NOT batch phases.

**Current State:** Batch-phase architecture (all files through each phase sequentially)

**Required Architecture:**
```
Worker 1: File_001 → [Complete Pipeline] → File_005 → ...
Worker 2: File_002 → [Complete Pipeline] → File_006 → ...
Worker 3: File_003 → [Complete Pipeline] → File_007 → ...
Worker 4: File_004 → [Complete Pipeline] → File_008 → ...
```

**Acceptance Criteria:**
- 4 concurrent workers processing different files
- Each worker executes full pipeline per file
- Progress reported as files completed (not phases)
- Workers use `futures::stream::buffer_unordered(4)`

**Source:** Lines 116, 419-465
**SPEC032 Reference:** Lines 232-250, 543-578

---

### REQ-PATT-010: Pattern Analyzer Component (P1 - HIGH)

**Description:** System MUST implement pattern analyzer to detect structural patterns in segmented audio.

**Inputs:**
- List of detected segments (count, start times, end times, gap durations)
- Audio file metadata (total duration, format)

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

**Analysis Required:**
- Track count (number of segments)
- Gap patterns (mean, std dev of gap durations)
- Segment durations (statistical analysis)
- Source media classification (CD/Vinyl/Cassette heuristics)

**Acceptance Criteria:**
- Pattern analyzer implemented as `PatternAnalyzer` service
- Accepts segment list, returns `PatternMetadata`
- Source media classification >80% accurate on test data
- Statistical analysis of gaps includes mean and standard deviation

**Source:** Lines 149-191
**SPEC032 Reference:** Lines 149-180

---

### REQ-CTXM-010: Contextual Matcher Component (P1 - HIGH)

**Description:** System MUST implement contextual matcher to search MusicBrainz using metadata + pattern clues.

**Inputs:**
- Extracted metadata (ID3 tags, filename, folder structure)
- Pattern analysis results (track count, source media type)
- Audio file characteristics (total duration, format)

**Outputs:**
- For single-segment files: `Vec<(RecordingMBID, MatchScore)>`
- For multi-segment files: `Vec<(ReleaseMBID, TrackList, MatchScore)>`

**Matching Strategy:**
- Single-segment: Artist + title search with duration filter (±10%)
- Multi-segment: Artist + album search with track count and duration filters
- Fuzzy string matching for names
- Ranked candidates with match scores (0.0-1.0)

**Acceptance Criteria:**
- Contextual matcher implemented as `ContextualMatcher` service
- Single-segment and multi-segment matching strategies
- MusicBrainz API rate limiting (1 req/s) respected
- Narrows candidates to <10 results in >80% of cases

**Source:** Lines 194-250
**SPEC032 Reference:** Lines 182-237

---

### REQ-CONF-010: Confidence Assessor Component (P1 - HIGH)

**Description:** System MUST implement confidence assessor to combine evidence for high-confidence MBID identification.

**Inputs:**
- Metadata match scores (from contextual matcher)
- Pattern match scores (from pattern analyzer)
- Fingerprint match scores (from AcoustID API, per segment)
- User preference parameters (confidence thresholds)

**Evidence Weighting:**
- Single-segment: 30% metadata + 60% fingerprint + 10% duration
- Multi-segment: 20% contextual + 20% pattern + 60% fingerprint

**Decision Thresholds:**
- confidence >= 0.85: ACCEPT (high confidence)
- 0.60 <= confidence < 0.85: REVIEW (manual verification)
- confidence < 0.60: REJECT (zero-song passage)

**Acceptance Criteria:**
- Confidence assessor implemented as `ConfidenceAssessor` service
- Combines evidence using specified weights
- Returns MBID + confidence + decision + evidence summary
- Achieves >90% acceptance rate for known-good files
- <5% false positive rate (incorrect MBID accepted)

**Source:** Lines 253-328
**SPEC032 Reference:** Lines 239-321

---

### REQ-FING-010: Per-Segment Fingerprinting (P1 - HIGH)

**Description:** System MUST generate Chromaprint fingerprints for EACH segment individually, not whole-file.

**Current State:** Whole-file fingerprinting only

**Required Architecture:**
```rust
fn fingerprint_segments(
    file_path: &Path,
    segments: &[(f64, f64)]  // (start_time, end_time) pairs
) -> Result<Vec<String>> {
    // Generate fingerprint for EACH segment individually
}
```

**Implementation Strategy:**
1. Decode audio file once to PCM
2. For each segment:
   - Extract segment PCM data (start_sample..end_sample)
   - Resample to 44.1kHz if needed
   - Generate Chromaprint fingerprint via FFI
3. Query AcoustID API per segment (rate-limited)

**Acceptance Criteria:**
- Fingerprinter supports per-segment operation
- AcoustID client queries per segment with rate limiting
- Per-segment fingerprints more accurate than whole-file for albums
- CHROMAPRINT_LOCK mutex protects FFI calls

**Source:** Lines 331-393
**SPEC032 Reference:** Lines 355-358, 694-741

---

### REQ-TICK-010: Tick-Based Timing Conversion (P2 - MEDIUM)

**Description:** System MUST convert all timing points to INTEGER ticks per SPEC017 before database write.

**Conversion Formula:**
```rust
const TICK_RATE: i64 = 28_224_000; // Hz
fn seconds_to_ticks(seconds: f64) -> i64 {
    (seconds * TICK_RATE as f64).round() as i64
}
```

**Database Fields Affected:**
- `start_time_ticks`
- `lead_in_start_ticks`
- `fade_in_start_ticks`
- `fade_in_end_ticks`
- `fade_out_start_ticks`
- `lead_out_start_ticks`
- `end_time_ticks`

**Acceptance Criteria:**
- All passage timing stored as INTEGER ticks
- Conversion maintains sample-accurate precision (<1 sample error)
- Applied to segmentation results (silence detection)
- Applied to amplitude analysis results (lead-in/lead-out)

**Source:** Lines 396-430
**SPEC032 Reference:** Lines 773-797, 568-571

---

### REQ-PATT-020: Track Count Detection (P2 - MEDIUM)

**Description:** Pattern analyzer MUST detect number of segments (track count).

**Acceptance Criteria:**
- Track count = number of segments detected
- Included in `PatternMetadata` output
- Used by contextual matcher for album matching

**Source:** Lines 157-161

---

### REQ-PATT-030: Gap Pattern Analysis (P2 - MEDIUM)

**Description:** Pattern analyzer MUST analyze gap durations between segments.

**Analysis:**
- Calculate mean gap duration
- Calculate standard deviation
- Classify: Consistent (CD), Variable (vinyl/cassette), or None

**Acceptance Criteria:**
- Gap pattern classification implemented
- Uses statistical analysis (mean, std dev)
- Heuristics: std dev < 0.5s → Consistent

**Source:** Line 159

---

### REQ-PATT-040: Source Media Classification (P2 - MEDIUM)

**Description:** Pattern analyzer MUST classify likely source media type.

**Classifications:**
- CD: Consistent gaps (2-3s), precise timing
- Vinyl: Variable gaps, longer gaps possible
- Cassette: Noise floor changes, variable gaps
- Unknown: Cannot determine

**Acceptance Criteria:**
- Source media classification implemented
- >80% accuracy on test dataset
- Uses gap patterns + other heuristics

**Source:** Line 161

---

### REQ-CTXM-020: Single-Segment File Matching (P1 - HIGH)

**Description:** Contextual matcher MUST search MusicBrainz for single-segment files using artist + title.

**Search Strategy:**
1. Parse metadata: artist, title, album from ID3 tags
2. Search MusicBrainz: artist + title (exact + fuzzy)
3. Filter by duration (±10% tolerance)
4. Return ranked candidates

**Acceptance Criteria:**
- Single-segment matching implemented
- Uses fuzzy string matching
- Duration filter applied
- Returns ranked candidates with scores

**Source:** Lines 205-212

---

### REQ-CTXM-030: Multi-Segment File Matching (P1 - HIGH)

**Description:** Contextual matcher MUST search MusicBrainz for multi-segment files using album structure.

**Search Strategy:**
1. Parse metadata: album artist, album title, folder
2. Identify likely album (12 tracks + CD pattern → CD album)
3. Search MusicBrainz releases: artist + album
4. Filter by track count (exact or ±1)
5. Filter by total duration (±5%)
6. Calculate alignment score (track count 40%, duration 30%, metadata 30%)

**Acceptance Criteria:**
- Multi-segment matching implemented
- Album structure detection logic
- Track count and duration filters
- Alignment score calculation

**Source:** Lines 214-231

---

## Dependencies

**Existing Components (No Changes):**
- `FileScanner` - Directory traversal
- `MetadataExtractor` - ID3/Vorbis/MP4 parsing
- `Fingerprinter` - Chromaprint (modify for per-segment)
- `AcoustIDClient` - AcoustID API (modify for per-segment)
- `MusicBrainzClient` - MusicBrainz API (used by new components)
- `AcousticBrainzClient` - AcousticBrainz API
- `AmplitudeAnalyzer` - RMS analysis
- `SilenceDetector` - Silence boundary detection

**New Components Required:**
- `PatternAnalyzer` - Segment pattern analysis
- `ContextualMatcher` - MusicBrainz search with metadata+pattern
- `ConfidenceAssessor` - Evidence combination

**External Dependencies:**
- SPEC017 (tick-based timing)
- SPEC031 (zero-config DB - already implemented)
- SPEC032 (audio ingest architecture - source specification)

---

## Assumptions

1. **Existing silence detection works correctly** - Pattern analyzer builds on existing segment boundaries
2. **MusicBrainz API access available** - Contextual matcher requires MusicBrainz queries
3. **AcoustID API key configured** - Per-segment fingerprinting requires AcoustID access
4. **Test dataset available** - Pattern analyzer accuracy testing requires known-good files
5. **Current database schema supports tick-based timing** - SPEC031 compliance assumed

---

## Constraints

**Technical:**
- Rust stable channel required
- Tokio async runtime (existing)
- SQLite database (existing)
- MusicBrainz API rate limit: 1 req/s
- AcoustID API rate limit: 3 req/s

**Performance:**
- Import throughput: ≥20 files/second (4 workers)
- Per-segment fingerprinting overhead: <20% vs. whole-file
- Contextual matching: <2 seconds per file

**Quality:**
- MBID identification accuracy: >90%
- False positive rate: <5%
- Test coverage: >80%

---

**END OF REQUIREMENTS INDEX**
