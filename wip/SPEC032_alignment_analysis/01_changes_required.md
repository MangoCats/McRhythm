# Detailed Changes Required for SPEC032

**Document:** SPEC032-audio_ingest_architecture.md alignment analysis
**Section:** Complete specification of all required changes
**Related:** [00_ANALYSIS_SUMMARY.md] for overview | [02_implementation_approaches.md] for approach comparison

---

## Navigation

This document details all 10 change categories required to align SPEC032 with SPEC_wkmp_ai_recode.md:

1. [Hybrid Architecture Integration](#change-1-hybrid-architecture-integration) - NEW (5 sections)
2. [Processing Model Enhancement](#change-2-processing-model-enhancement) - MODIFY (2 sections)
3. [Multi-Source Data Fusion](#change-3-multi-source-data-fusion) - NEW (3 sections)
4. [Confidence & Quality Framework](#change-4-confidence--quality-framework) - NEW (3 sections)
5. [Essentia Integration](#change-5-essentia-integration) - NEW (1 section)
6. [Database Schema Extensions](#change-6-database-schema-extensions) - MODIFY (3 sections)
7. [Granular SSE Events](#change-7-granular-sse-events) - NEW (3 sections)
8. [GOV002 Compliance](#change-8-gov002-compliance) - DOCUMENT (1 amendment)
9. [SPEC017 Visibility](#change-9-spec017-tick-based-timing-visibility) - ENHANCE (2 sections)
10. [/plan Workflow Structure](#change-10-plan-workflow-structure) - REORGANIZE

---

## CHANGE 1: Hybrid Architecture Integration

### Current State (SPEC032)

- Simple linear workflow: scan → extract → fingerprint → segment → analyze → flavor
- Single-source data for each type (AcousticBrainz for flavor, MusicBrainz for metadata)
- No fusion architecture
- No parallel source extraction

### Required State

- **3-Tier Hybrid Fusion Engine** as central architecture
- **Multiple parallel sources** with confidence-weighted fusion
- Clear tier separation (extraction → fusion → validation)

### Specification Additions Needed

#### [AIA-ARCH-010] System Architecture Overview

**Content:**
```markdown
**[AIA-ARCH-010]** WKMP Audio Ingest implements a 3-tier hybrid fusion architecture:

Tier 1: Source Extractors (Parallel)
  - Multiple independent data sources
  - Parallel execution via tokio::join!
  - Each source produces data with confidence score

Tier 2: Fusion Modules (Sequential per passage)
  - Confidence-weighted data fusion
  - Conflict detection and resolution
  - Source provenance tracking

Tier 3: Validation & Quality (Sequential per passage)
  - Cross-source consistency checks
  - Quality scoring
  - Conflict flagging for manual review

Workflow:
  Per-File: 4 parallel workers
  Per-Song: Sequential through fusion pipeline (Tier 1 → Tier 2 → Tier 3)
```

#### [AIA-ARCH-020] Tier 1: Source Extractors (Parallel)

**Content:**
```markdown
**[AIA-ARCH-020]** Tier 1 consists of 7 independent source extractors:

1. **ID3MetadataExtractor**
   - Extracts: Title, artist, album, genre, MBID, year, track number
   - Input: Audio file path
   - Output: ID3Metadata struct
   - Confidence: Varies by tag completeness (0.3-0.9)

2. **ChromaprintAnalyzer**
   - Generates: Chromaprint acoustic fingerprint
   - Input: Audio segment PCM data
   - Output: Fingerprint bytes
   - Used by: AcoustIDClient

3. **AcoustIDClient**
   - Queries: AcoustID API with Chromaprint fingerprint
   - Input: Fingerprint bytes
   - Output: Recording MBID + confidence from API
   - Rate Limited: 3 req/s

4. **MusicBrainzClient**
   - Queries: MusicBrainz API for recording metadata
   - Input: Recording MBID
   - Output: Title, artists (with weights), work, release, relationships
   - Rate Limited: 1 req/s

5. **EssentiaAnalyzer** (NEW)
   - Computes: Musical characteristics from audio analysis
   - Input: Audio segment PCM data
   - Output: Characteristics map (danceability, energy, mood, etc.)
   - Confidence: 0.9 (high quality local computation)

6. **AudioDerivedExtractor** (NEW)
   - Derives: Basic audio features (spectral, temporal)
   - Input: Audio segment PCM data
   - Output: Basic characteristics map
   - Confidence: 0.6 (medium quality)

7. **ID3GenreMapper** (NEW)
   - Maps: ID3 genre string → musical characteristics
   - Input: Genre string from ID3
   - Output: Coarse characteristics map
   - Confidence: 0.3 (low quality, rough mapping)

Execution:
  - All extractors run in parallel via tokio::join!
  - Failures in one extractor don't abort others
  - Outputs collected for Tier 2 fusion
```

#### [AIA-ARCH-030] Tier 2: Fusion Modules

**Content:**
```markdown
**[AIA-ARCH-030]** Tier 2 fuses data from multiple sources using confidence weighting:

1. **IdentityResolver**
   - Purpose: Resolve recording identity (MBID) from multiple sources
   - Algorithm: Bayesian update with conflict detection
   - Inputs: ID3 MBID (optional), AcoustID MBID + confidence
   - Outputs: Resolved MBID, posterior confidence, conflict flags
   - See: [AIA-FUSION-010]

2. **MetadataFuser**
   - Purpose: Select best metadata per field from multiple sources
   - Algorithm: Field-wise weighted selection
   - Inputs: ID3 metadata, MusicBrainz metadata, identity confidence
   - Outputs: Fused metadata, source per field, conflicts
   - See: [AIA-FUSION-020]

3. **FlavorSynthesizer**
   - Purpose: Synthesize musical flavor from multiple sources
   - Algorithm: Characteristic-wise weighted averaging
   - Inputs: AcousticBrainz, Essentia, AudioDerived, ID3Genre with confidences
   - Outputs: Fused characteristics, source blend, completeness
   - See: [AIA-FUSION-030]

4. **BoundaryFuser** (Future)
   - Purpose: Fuse passage boundaries from multiple detection strategies
   - Algorithm: Multi-strategy with weighted consensus
   - Current: Baseline silence detection only
   - Future: Beat tracking, structural analysis, metadata hints
   - See: [AIA-FUSION-040]

Execution:
  - Sequential per passage (identity → metadata → flavor)
  - Each module produces provenance data (source tracking)
  - Conflicts flagged for Tier 3 validation
```

#### [AIA-ARCH-040] Tier 3: Validation & Quality

**Content:**
```markdown
**[AIA-ARCH-040]** Tier 3 validates fused data quality:

1. **ConsistencyValidator**
   - Checks: Title consistency (ID3 vs MusicBrainz)
   - Checks: Duration consistency (ID3 vs audio)
   - Checks: Genre-flavor alignment (ID3 genre vs synthesized flavor)
   - Output: Per-check status (Pass/Warning/Fail)
   - See: [AIA-QUAL-020]

2. **QualityScorer**
   - Computes: Overall quality score (0-100%)
   - Computes: Metadata completeness (0-100%)
   - Computes: Flavor completeness (0-100%)
   - Formula: (passed_checks / total_checks) × 100%
   - See: [AIA-QUAL-020]

3. **ConflictDetector**
   - Detects: MBID conflicts (ID3 ≠ AcoustID)
   - Detects: Metadata conflicts (title/artist mismatches)
   - Detects: Flavor conflicts (genre suggests X, analysis shows Y)
   - Flags: Low-confidence passages for manual review
   - See: [AIA-QUAL-030]

Execution:
  - Sequential after all Tier 2 fusion complete
  - Produces validation report (JSON)
  - Determines validation status (Pass/Warning/Fail)
  - Flags passages requiring user review
```

#### [AIA-ARCH-050] Component Interactions

**Content:**
```markdown
**[AIA-ARCH-050]** Data flow through 3-tier architecture:

Per-File Workflow:
  1. File selected by orchestrator (one of 4 parallel workers)
  2. Phase 0: Passage boundary detection (entire file)
     Output: Vec<PassageBoundary>
  3. For each passage (sequential):
     a. Extract audio segment (PCM data)
     b. TIER 1: Parallel source extraction (tokio::join!)
        - ID3, Chromaprint, AcoustID, MusicBrainz, Essentia, AudioDerived, GenreMap
        - Output: 7 independent datasets with confidences
     c. TIER 2: Sequential fusion
        - IdentityResolver → MBID + confidence
        - MetadataFuser → Title, artist, album with sources
        - FlavorSynthesizer → Characteristics with source blend
     d. TIER 3: Validation
        - ConsistencyValidator → Check results
        - QualityScorer → Quality percentage
        - ConflictDetector → Conflict flags
     e. Database persistence
        - Insert passage with extended schema
        - Insert provenance log entries
        - Emit SongCompleted event

Error Handling:
  - Tier 1 extractor failures: Continue with remaining sources (reduced quality)
  - Tier 2 fusion failures: Skip passage, emit SongFailed event
  - Tier 3 validation warnings: Store passage, flag for review
  - Per-song error isolation: One passage failure doesn't abort file import
```

---

## CHANGE 2: Processing Model Enhancement

### Current State (SPEC032)

- **Per-file pipeline** with 4 parallel workers
- Each file processes through entire pipeline sequentially
- States: SCANNING → PROCESSING → COMPLETED
- No per-song granularity

### Required State

- **Hybrid: Per-file parallelism + per-song sequential**
- Phase 0: Detect passages in file (once)
- Phase 1-7: Process each passage sequentially through fusion
- Track file-level AND song-level progress

### Specification Modifications

#### Update [AIA-ASYNC-020] Parallel Per-File Processing Architecture

**Replace existing section with:**
```markdown
**[AIA-ASYNC-020]** Hybrid Per-File + Per-Song Processing Architecture

File-Level Parallelism:
  - 4 concurrent files processing simultaneously
  - Implementation: futures::stream::buffer_unordered(4)
  - Load balancing: Automatic work distribution

Per-File Sequential Workflow:
  Phase 0: Passage Boundary Detection (once per file)
    - Detect ALL passages in file using silence detection
    - Output: Vec<PassageBoundary> with tick-based timing
    - Emit: PassagesDiscovered event
    - See: [AIA-FUSION-040] for detection algorithm

  Phase 1-7: Per-Song Sequential Processing (for each passage)
    FOR EACH passage in file:
      Phase 1: Extract Audio Segment
        - Segment audio based on passage boundaries
        - Convert to PCM for analysis
        - Emit: SongExtracting event

      Phase 2: Tier 1 Parallel Extraction
        - Execute all 7 source extractors in parallel
        - Collect results with confidence scores
        - See: [AIA-ARCH-020]

      Phase 3: Tier 2 Identity Resolution
        - Fuse ID3 MBID + AcoustID MBID
        - Compute posterior confidence
        - Detect conflicts
        - Emit: IdentityResolved event
        - See: [AIA-FUSION-010]

      Phase 4: Tier 2 Metadata Fusion
        - Select best metadata per field
        - Track source provenance
        - Emit: MetadataFused event
        - See: [AIA-FUSION-020]

      Phase 5: Tier 2 Flavor Synthesis
        - Weighted averaging across sources
        - Normalize characteristics
        - Emit: FlavorSynthesized event
        - See: [AIA-FUSION-030]

      Phase 6: Tier 3 Validation
        - Consistency checks
        - Quality scoring
        - Conflict detection
        - Emit: ValidationComplete event
        - See: [AIA-ARCH-040]

      Phase 7: Database Persistence
        - Insert passage record (extended schema)
        - Insert provenance log entries
        - Emit: SongCompleted event

Progress Tracking:
  - File level: "Processing file 2,581 / 5,736 (45.0%)"
  - Song level: "File 2,581: Song 3 / 5 - Phase 4: Fusing metadata"
  - Both levels reported simultaneously in SSE events
```

#### Update [AIA-WF-010] Import Workflow State Machine

**Replace state diagram with:**
```markdown
**[AIA-WF-010]** Import session progresses through states:

State Diagram:
```
POST /import/start
       │
       ▼
┌────────────┐
│  SCANNING  │  (Batch: Directory traversal, magic byte verification)
└─────┬──────┘
      │
      ▼
┌─────────────────┐
│ PROCESSING_FILE │  (Per-file workflow with per-song pipeline)
│                 │
│  Internal:      │
│  ├─ DETECTING_PASSAGES (Phase 0)
│  └─ PROCESSING_SONGS (Phase 1-7 per passage)
└─────┬───────────┘
      │
      ▼
┌────────────┐
│ COMPLETED  │
└────────────┘

Cancel available at any state → CANCELLED
Error in any state → FAILED (with error details)
```

State Semantics:
  - SCANNING: Batch file discovery (parallel magic byte verification)
  - PROCESSING_FILE: Per-file orchestration
    - DETECTING_PASSAGES: Phase 0 boundary detection (internal)
    - PROCESSING_SONGS: Phase 1-7 per passage (internal)
  - COMPLETED: All files successfully processed

Progress Reporting:
  - SCANNING: "Scanning: X / Y files discovered"
  - PROCESSING_FILE: "Processing file X / Y: Song M / N - Phase P: <operation>"
  - COMPLETED: "Import complete: X files, Y songs, Z warnings, W failures"
```

---

## CHANGE 3: Multi-Source Data Fusion

### Current State (SPEC032)

- Single-source lookups (AcoustID → MusicBrainz → AcousticBrainz)
- Linear override: Last-write-wins
- No conflict detection
- No source provenance

### Required State

- Multiple parallel sources
- Confidence-weighted fusion
- Conflict detection and resolution
- Source provenance tracking

### New Sections Needed

#### [AIA-FUSION-010] Identity Resolution (Bayesian)

**Content:**
```markdown
**[AIA-FUSION-010]** Identity Resolution via Bayesian MBID Fusion

Purpose: Resolve recording identity (MusicBrainz Recording ID) from multiple sources with conflict detection.

Algorithm: Bayesian Update

Inputs:
  - id3_mbid: Option<Uuid> (from ID3 UFID tag)
  - acoustid_mbid: Uuid (from AcoustID API)
  - acoustid_confidence: f64 (from AcoustID API, 0.0-1.0)

Prior Confidences:
  - ID3 MBID (if present): 0.9 (high confidence, user-curated tags)
  - ID3 MBID (if absent): 0.0 (no prior information)

Bayesian Update Process:

  CASE 1: ID3 MBID matches AcoustID MBID
    posterior_conf = 1 - (1 - id3_conf) × (1 - acoustid_conf)
    posterior_conf = 1 - (1 - 0.9) × (1 - acoustid_conf)
    posterior_conf = 1 - 0.1 × (1 - acoustid_conf)

    Example: If acoustid_conf = 0.92:
      posterior_conf = 1 - 0.1 × 0.08 = 0.992 (very high confidence)

  CASE 2: ID3 MBID conflicts with AcoustID MBID
    Select higher-confidence source
    Apply conflict penalty: posterior_conf ×= 0.85
    Flag conflict: "ID3 MBID conflicts with AcoustID MBID"

    Example: If ID3 conf = 0.9, AcoustID conf = 0.75:
      Select ID3 (higher confidence)
      posterior_conf = 0.9 × 0.85 = 0.765

  CASE 3: ID3 MBID absent, AcoustID only
    posterior_conf = acoustid_conf (no fusion, single source)

Outputs:
  - resolved_mbid: Uuid (selected MBID)
  - identity_confidence: f64 (posterior confidence 0.0-1.0)
  - identity_conflicts: Vec<String> (conflict descriptions)
  - source: String ("ID3+AcoustID agreement" | "AcoustID only" | "ID3 only" | "Conflict: ID3 selected")

Thresholds:
  - High confidence: ≥ 0.85
  - Medium confidence: 0.70-0.84
  - Low confidence: < 0.70 (flag for manual review)
  - Conflict with low confidence: < 0.85 after conflict (flag for manual review)

Error Handling:
  - If both sources missing: Fail passage (cannot proceed without MBID)
  - If AcoustID API failure: Use ID3 MBID if available, else fail
```

#### [AIA-FUSION-020] Metadata Fusion (Weighted Selection)

**Content:**
```markdown
**[AIA-FUSION-020]** Metadata Fusion via Field-Wise Weighted Selection

Purpose: Select best metadata per field from multiple sources with provenance tracking.

Sources:
  - ID3 tags (from ID3MetadataExtractor)
  - MusicBrainz API (from MusicBrainzClient)

Per-Field Selection Strategy:

  **Title:**
    IF identity_confidence > 0.85:
      Use MusicBrainz title (high confidence in MBID match)
    ELSE:
      Use ID3 title (low confidence in MBID, prefer user tags)

    Consistency Check:
      Levenshtein ratio (ID3 title, MusicBrainz title)
      IF ratio < 0.85:
        Flag conflict: "Title mismatch: ID3 vs MusicBrainz"
        Store both versions in metadata_conflicts

  **Artist:**
    IF identity_confidence > 0.85:
      Use MusicBrainz artists (with artist weights for multi-artist tracks)
    ELSE:
      Use ID3 artist string

    Consistency Check:
      Fuzzy match artist names (handle "The Beatles" vs "Beatles")
      IF no match:
        Flag conflict: "Artist mismatch"

  **Album:**
    IF MusicBrainz has release data AND identity_confidence > 0.85:
      Use MusicBrainz release title
    ELSE:
      Use ID3 album tag

  **Genre:**
    ALWAYS use ID3 genre
    Rationale: MusicBrainz lacks genre data, ID3 genre is user-curated

Quality Scoring:
  metadata_quality = (filled_fields / total_fields) × 100%
  MBID bonus: +1.5 to score if MusicBrainz MBID present

Outputs:
  - fused_metadata: Metadata struct
  - title_source: String ("ID3" | "MusicBrainz")
  - title_confidence: f64 (from identity_confidence)
  - artist_source: String
  - artist_confidence: f64
  - album_source: String
  - metadata_conflicts: Vec<String> (conflict descriptions)
  - metadata_quality_score: f64 (0.0-100.0%)
```

#### [AIA-FUSION-030] Musical Flavor Synthesis (Weighted Averaging)

**Content:**
```markdown
**[AIA-FUSION-030]** Musical Flavor Synthesis via Characteristic-Wise Weighted Averaging

Purpose: Synthesize musical flavor characteristics from multiple sources with confidence weighting.

Sources (Tier 1 Parallel Extraction):
  1. AcousticBrainz API (if Recording MBID exists and pre-2022 data available)
     Confidence: 1.0 (highest quality, pre-computed, peer-reviewed)

  2. Essentia local computation (if Essentia library available)
     Confidence: 0.9 (high quality, computed locally)

  3. Audio-derived features (basic spectral/temporal analysis)
     Confidence: 0.6 (medium quality, coarse features)

  4. ID3 genre mapping (genre string → characteristics)
     Confidence: 0.3 (low quality, rough mapping)

Characteristic-Wise Weighted Averaging:

  FOR EACH characteristic in union_of_all_sources:
    fused_value = Σ(confidence_i × value_i) / Σ(confidence_i)

  Example: "danceability.danceable" characteristic
    Sources:
      - AcousticBrainz: 0.65 (conf: 1.0)
      - Essentia: 0.70 (conf: 0.9)
      - AudioDerived: 0.55 (conf: 0.6)
      - ID3Genre: 0.80 (conf: 0.3)

    Calculation:
      fused = (1.0×0.65 + 0.9×0.70 + 0.6×0.55 + 0.3×0.80) / (1.0 + 0.9 + 0.6 + 0.3)
      fused = (0.65 + 0.63 + 0.33 + 0.24) / 2.8
      fused = 1.85 / 2.8 = 0.661

    Characteristic confidence = max(1.0, 0.9, 0.6, 0.3) = 1.0

Normalization (per MFL-DEF-030):
  - Binary characteristics: sum to 1.0 ± 0.0001
  - Complex characteristics: per-category sum to 1.0 ± 0.0001
  - Validate normalization, fail import if violated

Source Blend Tracking:
  flavor_source_blend = {
    "AcousticBrainz": 1.0 / 2.8 = 0.357,
    "Essentia": 0.9 / 2.8 = 0.321,
    "AudioDerived": 0.6 / 2.8 = 0.214,
    "ID3Genre": 0.3 / 2.8 = 0.107
  }

  Interpretation: 35.7% AcousticBrainz, 32.1% Essentia, 21.4% AudioDerived, 10.7% ID3

Completeness Scoring:
  flavor_completeness = (present_characteristics / expected_characteristics) × 100%

  Expected characteristics (per MFL-DEF-030): ~50 characteristics
  If 43 characteristics present: completeness = 43/50 = 86%

Outputs:
  - fused_characteristics: Map<String, f64> (characteristic → value)
  - flavor_source_blend: Map<String, f64> (source → weight)
  - flavor_confidence_map: Map<String, f64> (characteristic → confidence)
  - flavor_completeness: f64 (0.0-100.0%)
```

#### [AIA-FUSION-040] Passage Boundary Detection (Baseline + Future)

**Content:**
```markdown
**[AIA-FUSION-040]** Passage Boundary Detection

Purpose: Detect passage boundaries within audio files (single file may contain multiple songs).

**Baseline Strategy: Silence Detection**

Algorithm:
  1. Decode audio to PCM
  2. Compute RMS (Root Mean Square) per frame (e.g., 2048 samples)
  3. Convert RMS to dB: dB = 20 × log10(RMS)
  4. Detect silence segments: frames below threshold (default: -60 dB)
  5. Find continuous silence regions (min duration: 1 second)
  6. Passage boundaries: midpoint of silence regions

Parameters:
  - silence_threshold_db: f64 (default: -60.0)
  - min_silence_duration: Duration (default: 1.0 seconds)
  - min_passage_duration: Duration (default: 30 seconds)
  - max_passage_duration: Duration (default: 15 minutes)

Validation:
  - Reject boundaries creating passages < 30 seconds
  - Reject boundaries creating passages > 15 minutes
  - If file has no silence: treat entire file as single passage

Output:
  - Vec<PassageBoundary> with tick-based timing (per SPEC017)

PassageBoundary Structure:
  - start_time: i64 (ticks from file start)
  - end_time: i64 (ticks from file start)
  - confidence: f64 (0.0-1.0, for silence detection: 1.0 if clear silence, lower if ambiguous)
  - detection_method: String ("silence_detection")

**Future Extensions (Multi-Strategy Fusion):**

Strategy 2: Beat Tracking
  - Detect tempo changes (indicate track boundaries)
  - Confidence based on beat strength

Strategy 3: Structural Analysis
  - Detect intro/verse/chorus/outro patterns
  - Track transitions often align with boundaries

Strategy 4: Metadata Hints
  - ID3 track start times (rare)
  - Cue sheet files (.cue)
  - High confidence when present

Fusion Algorithm (Future):
  FOR EACH boundary candidate from all strategies:
    Cluster nearby candidates (within 500ms tolerance)
    Compute consensus position via weighted averaging
    Aggregate confidence from multiple sources

Current Implementation:
  - Baseline silence detection only
  - Multi-strategy fusion deferred to future enhancement
```

---

## CHANGE 4: Confidence & Quality Framework

### Current State (SPEC032)

- No confidence tracking
- No quality validation
- No conflict detection
- No manual review flagging

### Required State

- Confidence scores for all data sources (0.0-1.0)
- Quality validation with pass/warning/fail status
- Conflict detection and user flagging
- Comprehensive validation framework

### New Sections Needed

#### [AIA-QUAL-010] Confidence Framework

**Content:**
```markdown
**[AIA-QUAL-010]** Confidence Framework

Purpose: Track confidence scores for all data sources and computed results.

Source Confidences (predefined):
  - ID3 MBID: 0.9 (if present, user-curated)
  - AcoustID MBID: from API response (typically 0.5-0.99)
  - MusicBrainz metadata: identity_confidence (depends on MBID match)
  - AcousticBrainz flavor: 1.0 (highest quality, if available)
  - Essentia flavor: 0.9 (high quality local computation)
  - Audio-derived flavor: 0.6 (medium quality)
  - ID3 genre flavor: 0.3 (low quality, coarse mapping)

Computed Confidences (via fusion algorithms):
  - identity_confidence: Bayesian posterior (0.0-1.0)
    See: [AIA-FUSION-010]

  - title_confidence: identity_confidence
    Rationale: Title quality depends on MBID match quality

  - artist_confidence: identity_confidence
    Rationale: Artist quality depends on MBID match quality

  - flavor_completeness: Percentage of expected characteristics present (0.0-100.0%)
    See: [AIA-FUSION-030]

Confidence Thresholds:
  - High confidence: ≥ 0.85 (accept without manual review)
  - Medium confidence: 0.70-0.84 (accept, note in validation report)
  - Low confidence: < 0.70 (flag for manual review)

  - Conflict with reduced confidence: < 0.85 after conflict penalty (flag for manual review)

Confidence Storage:
  - identity_confidence: REAL column in passages table
  - title_confidence: REAL column in passages table
  - artist_confidence: REAL column in passages table
  - flavor_confidence_map: TEXT (JSON) mapping characteristic → confidence

Confidence Display (SSE events):
  - Include confidence scores in IdentityResolved, MetadataFused, FlavorSynthesized events
  - UI displays confidence as percentage (e.g., "MBID match: 95% confidence")
  - Low-confidence passages highlighted for user review
```

#### [AIA-QUAL-020] Quality Validation

**Content:**
```markdown
**[AIA-QUAL-020]** Quality Validation

Purpose: Validate passage data quality using cross-source consistency checks.

Validation Checks:

**Check 1: Title Consistency**

  Purpose: Verify ID3 title matches MusicBrainz title

  Algorithm:
    Levenshtein ratio (ID3_title, MusicBrainz_title)

  Thresholds:
    Pass: similarity > 0.95
    Warning: 0.80 ≤ similarity ≤ 0.95
    Fail: similarity < 0.80

  Example:
    ID3: "Breathe (In The Air)"
    MusicBrainz: "Breathe (In the Air)"
    Similarity: 0.97 → Pass

**Check 2: Duration Consistency**

  Purpose: Verify ID3 duration matches audio file duration

  Algorithm:
    diff = |ID3_duration_ms - audio_duration_ms|

  Thresholds:
    Pass: diff ≤ 1000ms
    Fail: diff > 1000ms

  Rationale:
    ID3 tags may have inaccurate duration
    Audio file duration is ground truth

**Check 3: Genre-Flavor Alignment**

  Purpose: Verify ID3 genre aligns with synthesized musical flavor

  Algorithm:
    1. Map ID3 genre → expected characteristics
       Example: "Rock" → high energy, high aggressive, guitar-heavy
    2. Compare expected vs actual synthesized characteristics
    3. Compute alignment = avg(characteristic_similarity)

  Thresholds:
    Pass: alignment > 0.7
    Warning: 0.5 ≤ alignment ≤ 0.7
    Fail: alignment < 0.5

  Rationale:
    Large genre-flavor mismatches indicate data quality issues
    May suggest incorrect MBID match or inaccurate genre tag

**Overall Quality Score:**

  Formula:
    overall_quality = (passed_checks / total_checks) × 100%

  Example:
    3 checks: Title (Pass), Duration (Pass), Genre-Flavor (Warning)
    overall_quality = (2 / 3) × 100% = 66.7%

**Validation Status:**

  Determination:
    IF all checks passed: "Pass"
    ELSE IF at least one warning AND no failures: "Warning"
    ELSE IF at least one failure: "Fail"
    ELSE: "Pending" (validation not yet run)

**Validation Report (JSON):**

  Structure:
    {
      "title_consistency": { "status": "Pass", "similarity": 0.97 },
      "duration_consistency": { "status": "Pass", "diff_ms": 120 },
      "genre_flavor_alignment": { "status": "Warning", "alignment": 0.65 }
    }

**Storage:**
  - overall_quality_score: REAL (0.0-100.0)
  - validation_status: TEXT ("Pass" | "Warning" | "Fail" | "Pending")
  - validation_report: TEXT (JSON with check details)
```

#### [AIA-QUAL-030] Conflict Detection

**Content:**
```markdown
**[AIA-QUAL-030]** Conflict Detection

Purpose: Detect and flag conflicts between data sources for manual review.

Conflict Types:

**1. Identity Conflicts (MBID)**

  Detection:
    ID3 MBID ≠ AcoustID MBID

  Action:
    - Select higher-confidence source
    - Apply conflict penalty (×0.85)
    - Store both MBIDs in identity_conflicts JSON
    - Flag: "MBID conflict: ID3 vs AcoustID"

  Example:
    ID3 MBID: abc-123-def (confidence: 0.9)
    AcoustID MBID: xyz-789-ghi (confidence: 0.75)
    Selected: abc-123-def
    Final confidence: 0.9 × 0.85 = 0.765
    Conflict: "MBID conflict: ID3 (abc-123-def) vs AcoustID (xyz-789-ghi)"

**2. Metadata Conflicts (Title/Artist)**

  Detection:
    Levenshtein ratio (ID3_title, MusicBrainz_title) < 0.85

  Action:
    - Store both versions in metadata_conflicts JSON
    - Flag: "Title mismatch: ID3 vs MusicBrainz"
    - Proceed with selected source (per [AIA-FUSION-020])

  Example:
    ID3 title: "Dark Side of the Moon"
    MusicBrainz title: "The Dark Side of the Moon"
    Similarity: 0.83 < 0.85
    Conflict: "Title mismatch: ID3 (Dark Side of the Moon) vs MusicBrainz (The Dark Side of the Moon)"

**3. Flavor Conflicts (Genre-Analysis Mismatch)**

  Detection:
    Genre-flavor alignment < 0.5 (per [AIA-QUAL-020])

  Action:
    - Flag: "Genre-flavor mismatch: Genre suggests X, analysis shows Y"
    - Proceed with synthesized flavor (higher confidence than genre mapping)
    - User review recommended

  Example:
    ID3 genre: "Classical"
    Synthesized flavor: High energy, high aggressive, electronic
    Alignment: 0.32
    Conflict: "Genre-flavor mismatch: Classical genre suggests low energy, analysis shows high energy"

**Conflict Storage:**

  Database Fields:
    - identity_conflicts: TEXT (JSON array of identity conflict descriptions)
    - metadata_conflicts: TEXT (JSON array of metadata conflict descriptions)

  Example JSON:
    identity_conflicts: ["MBID conflict: ID3 (abc) vs AcoustID (xyz)"]
    metadata_conflicts: ["Title mismatch: ID3 vs MusicBrainz", "Artist variation detected"]

**Conflict Flagging for Manual Review:**

  Criteria:
    - Identity conflict with final confidence < 0.85
    - Metadata conflict (any)
    - Flavor conflict with alignment < 0.5
    - Overall quality score < 70%

  Action:
    - Set validation_status = "Fail" (if conflict severe)
    - Include passage_id in manual review queue
    - SSE event: Include conflict details in ValidationComplete event
    - UI highlights passage with conflict indicator
```

---

## CHANGE 5: Essentia Integration

### Current State (SPEC032)

- AcousticBrainz API only (service ended 2022 - **DEAD API**)
- No fallback for musical flavor computation
- No local flavor analysis capability

### Required State

- **Essentia local computation** as primary flavor source
- AcousticBrainz as fallback (if data exists for pre-2022 recordings)
- Multi-source flavor synthesis (Essentia + AcousticBrainz + AudioDerived + GenreMap)

### New Section Needed

#### [AIA-ESSENT-010] Essentia Integration

**Content:**
```markdown
**[AIA-ESSENT-010]** Essentia Integration

Purpose: Local musical flavor computation to replace obsolete AcousticBrainz API.

Background:
  - AcousticBrainz service ended in 2022 (API no longer available)
  - Essentia library provides similar audio analysis capabilities
  - Local computation eliminates external API dependency

Essentia Library:
  - C++ audio analysis library with Python and Rust bindings
  - Computes musical characteristics from raw audio
  - Open source (AGPL license)
  - Installation: Package manager (apt, brew) or compile from source

Extracted Features (Musical Characteristics):

  **Danceability:**
    - danceability.danceable: 0.0-1.0
    - danceability.not_danceable: 0.0-1.0 (complement)

  **Energy/Aggression:**
    - mood_aggressive.aggressive: 0.0-1.0
    - mood_aggressive.not_aggressive: 0.0-1.0

  **Mood:**
    - mood_happy.happy: 0.0-1.0
    - mood_sad.sad: 0.0-1.0
    - mood_relaxed.relaxed: 0.0-1.0
    - mood_party.party: 0.0-1.0

  **Tonal/Atonal:**
    - tonal.tonal: 0.0-1.0
    - tonal.atonal: 0.0-1.0

  **Acoustic/Electronic:**
    - timbre.acoustic: 0.0-1.0
    - timbre.electronic: 0.0-1.0

  **Tempo/Rhythm:**
    - rhythm.steady: 0.0-1.0
    - rhythm.variable: 0.0-1.0

  [Full characteristic set per MFL-DEF-030]

Input:
  - Audio segment PCM data (mono or stereo, any sample rate)
  - Passage boundaries (for segment extraction)

Algorithm:
  1. Extract audio segment based on passage boundaries
  2. Resample to 44.1kHz (Essentia standard input rate)
  3. Convert to mono if stereo (mix channels)
  4. Compute Essentia features:
     - MusicExtractor (high-level features)
     - Or individual extractors (danceability, mood, timbre, etc.)
  5. Map Essentia output → WKMP musical flavor characteristics
  6. Normalize characteristics per MFL-DEF-030

Output:
  - Characteristics map: Map<String, f64>
  - Confidence: 0.9 (high quality local computation)
  - Source: "Essentia"

Error Handling:
  - If Essentia not installed: Log warning, continue without (use other sources)
  - If Essentia analysis fails: Log error for specific passage, mark as failed extractor
  - Fusion continues with remaining sources (AcousticBrainz, AudioDerived, GenreMap)

Integration Points:
  - Tier 1 Source Extractor (parallel with AcousticBrainz, AudioDerived, GenreMap)
  - Input: Audio segment PCM from Phase 1
  - Output: Passed to FlavorSynthesizer (Tier 2) for weighted averaging

Performance:
  - Computation time: ~2-5 seconds per 3-minute passage (CPU-dependent)
  - Parallelization: Multiple passages can run Essentia in parallel across workers
  - Blocking: Use tokio::task::spawn_blocking() for CPU-intensive Essentia call

Source Prioritization (for multi-source synthesis):
  1. AcousticBrainz (if available): confidence 1.0
  2. Essentia: confidence 0.9
  3. Audio-derived: confidence 0.6
  4. ID3 genre: confidence 0.3

Rationale:
  - Essentia provides higher quality than basic audio-derived features
  - AcousticBrainz (when available) still preferred due to peer-reviewed data
  - Essentia fills gap for post-2022 recordings without AcousticBrainz data
```

---

## CHANGE 6: Database Schema Extensions

### Current State (SPEC032)

- Standard passages table per IMPL001
- No source provenance tracking
- No confidence/quality fields
- No validation status
- No import metadata

### Required State

- **Extended passages table** with 15+ new fields
- **New import_provenance table** for complete audit trail
- **SPEC017 tick-based timing compliance** (explicit visibility)

### New Sections Needed

#### [AIA-DB-020] Extended Passages Table

**Content:**
```markdown
**[AIA-DB-020]** Extended Passages Table Schema

Purpose: Extend passages table with source provenance, confidence, quality, and validation fields.

Schema Extensions (add to existing passages table):

```sql
-- Source Provenance Fields
flavor_source_blend TEXT,              -- JSON: {"AcousticBrainz": 0.6, "Essentia": 0.3, ...}
flavor_confidence_map TEXT,            -- JSON: {"danceability.danceable": 0.95, ...}

title_source TEXT,                     -- "ID3" | "MusicBrainz" | "Conflict"
title_confidence REAL,                 -- 0.0-1.0
artist_source TEXT,                    -- "ID3" | "MusicBrainz" | "Conflict"
artist_confidence REAL,                -- 0.0-1.0

-- Identity Resolution Fields
recording_mbid TEXT,                   -- Final resolved MusicBrainz Recording ID
identity_confidence REAL,              -- Posterior confidence from Bayesian update (0.0-1.0)
identity_conflicts TEXT,               -- JSON array: ["MBID conflict: ID3 vs AcoustID"]

-- Quality Metrics Fields
overall_quality_score REAL,            -- 0.0-100.0% (validation score)
metadata_completeness REAL,            -- 0.0-100.0% (filled fields / total fields)
flavor_completeness REAL,              -- 0.0-100.0% (present characteristics / expected)

-- Validation Fields
validation_status TEXT,                -- "Pass" | "Warning" | "Fail" | "Pending"
validation_report TEXT,                -- JSON: {"title_consistency": {"status": "Pass", ...}, ...}

-- Import Metadata Fields
import_session_id TEXT,                -- UUID grouping passages from same import session
import_timestamp INTEGER,              -- Unix timestamp (milliseconds) when passage imported
import_strategy TEXT,                  -- "HybridFusion" (for future mode support)
```

Field Descriptions:

  **flavor_source_blend:**
    - JSON object mapping source name → weight contribution
    - Example: {"AcousticBrainz": 0.357, "Essentia": 0.321, "AudioDerived": 0.214, "ID3Genre": 0.107}
    - Interpretation: 35.7% from AcousticBrainz, 32.1% from Essentia, etc.
    - Computed per [AIA-FUSION-030]

  **flavor_confidence_map:**
    - JSON object mapping characteristic → confidence score
    - Example: {"danceability.danceable": 0.95, "mood_aggressive.aggressive": 0.88, ...}
    - Per-characteristic confidence (max confidence of sources providing that characteristic)

  **title_source / artist_source:**
    - Source selected for title/artist field
    - Values: "ID3" | "MusicBrainz" | "Conflict"
    - "Conflict" indicates both sources present but differ, selection made per [AIA-FUSION-020]

  **recording_mbid:**
    - Final resolved MusicBrainz Recording ID
    - Result of Bayesian fusion per [AIA-FUSION-010]
    - Used for MusicBrainz metadata lookup, AcousticBrainz flavor lookup

  **identity_confidence:**
    - Posterior confidence from Bayesian MBID fusion (0.0-1.0)
    - Computed per [AIA-FUSION-010]
    - Used for metadata source selection ([AIA-FUSION-020])

  **identity_conflicts:**
    - JSON array of conflict descriptions
    - Example: ["MBID conflict: ID3 (abc-123) vs AcoustID (xyz-789)"]
    - Empty array if no conflicts

  **overall_quality_score:**
    - Percentage of validation checks passed (0.0-100.0%)
    - Computed per [AIA-QUAL-020]
    - Used for UI filtering (e.g., show only passages with quality > 70%)

  **validation_status:**
    - Overall validation result: "Pass" | "Warning" | "Fail" | "Pending"
    - "Pass": All checks passed
    - "Warning": At least one warning, no failures
    - "Fail": At least one failure
    - "Pending": Validation not yet run (should not occur in production)

  **validation_report:**
    - Complete JSON validation details
    - Includes status and metrics for each check
    - Example: {"title_consistency": {"status": "Pass", "similarity": 0.97}, ...}

  **import_session_id:**
    - UUID identifying import session
    - All passages imported in same session share this ID
    - Used for querying/deleting all passages from specific import

  **import_timestamp:**
    - Unix timestamp in milliseconds when passage was imported
    - Used for sorting, filtering by import date

  **import_strategy:**
    - Always "HybridFusion" for current implementation
    - Future: May support "ManualEntry", "LegacyImport", etc.
    - Enables schema to accommodate multiple import methods

Backward Compatibility:
  - All new fields are additions (no deletions or renames)
  - NULL allowed for all new fields (default values)
  - Existing passages table rows remain valid (new fields default to NULL)
  - Migration: ALTER TABLE ADD COLUMN for each new field (idempotent)
```

#### [AIA-DB-030] Import Provenance Table

**Content:**
```markdown
**[AIA-DB-030]** Import Provenance Table Schema

Purpose: Provide complete audit trail of all data sources contributing to each passage.

Schema:

```sql
CREATE TABLE IF NOT EXISTS import_provenance (
    id TEXT PRIMARY KEY,                -- UUID for provenance record
    passage_id TEXT NOT NULL,           -- Foreign key to passages(id)
    source_type TEXT NOT NULL,          -- "ID3" | "AcoustID" | "MusicBrainz" | "Essentia" | "AudioDerived" | "GenreMap"
    data_extracted TEXT,                -- JSON: Complete data extracted from this source
    confidence REAL,                    -- Source confidence (0.0-1.0)
    timestamp INTEGER,                  -- Unix timestamp (ms) when extraction occurred
    FOREIGN KEY (passage_id) REFERENCES passages(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_provenance_passage_id ON import_provenance(passage_id);
CREATE INDEX IF NOT EXISTS idx_provenance_source_type ON import_provenance(source_type);
```

Field Descriptions:

  **id:**
    - UUID for this provenance record
    - Primary key

  **passage_id:**
    - Links to passages table
    - CASCADE DELETE: Deleting passage removes all provenance records

  **source_type:**
    - Which Tier 1 source extractor produced this data
    - Values:
      - "ID3" - ID3MetadataExtractor
      - "AcoustID" - AcoustIDClient
      - "MusicBrainz" - MusicBrainzClient
      - "Essentia" - EssentiaAnalyzer
      - "AudioDerived" - AudioDerivedExtractor
      - "GenreMap" - ID3GenreMapper
      - "AcousticBrainz" - AcousticBrainzClient (if used)

  **data_extracted:**
    - JSON blob of complete extracted data from this source
    - Allows reconstruction of fusion process
    - Example (ID3):
      ```json
      {
        "title": "Breathe (In The Air)",
        "artist": "Pink Floyd",
        "album": "The Dark Side of the Moon",
        "genre": "Progressive Rock",
        "mbid": "abc-123-def",
        "year": 1973
      }
      ```
    - Example (Essentia):
      ```json
      {
        "danceability.danceable": 0.42,
        "mood_aggressive.aggressive": 0.15,
        "mood_happy.happy": 0.55,
        ...
      }
      ```

  **confidence:**
    - Confidence score for this source (0.0-1.0)
    - Fixed per source type ([AIA-QUAL-010])
    - Used for weighted fusion

  **timestamp:**
    - When extraction occurred (Unix milliseconds)
    - Allows temporal analysis (e.g., API response times)

Usage Examples:

  **Query 1: View all sources for a passage**
  ```sql
  SELECT source_type, confidence, data_extracted
  FROM import_provenance
  WHERE passage_id = 'passage-uuid-123'
  ORDER BY confidence DESC;
  ```

  **Query 2: Find passages where Essentia was used**
  ```sql
  SELECT p.id, p.title, ip.confidence
  FROM passages p
  JOIN import_provenance ip ON p.id = ip.passage_id
  WHERE ip.source_type = 'Essentia';
  ```

  **Query 3: Reconstruct fusion process for passage**
  ```sql
  SELECT source_type, data_extracted, confidence
  FROM import_provenance
  WHERE passage_id = 'passage-uuid-123'
  ORDER BY timestamp ASC;
  ```

Rationale:
  - Complete transparency: User can see exactly which sources contributed
  - Debugging: If fusion produces unexpected result, can trace back to source data
  - Future analysis: May reveal patterns (e.g., "Essentia consistently outperforms ID3 genre")
  - Regulatory: Provides audit trail for data provenance (useful for licensing/attribution)
```

#### [AIA-DB-040] SPEC017 Tick-Based Timing Compliance

**Content:**
```markdown
**[AIA-DB-040]** SPEC017 Tick-Based Timing Compliance

Purpose: Ensure all passage timing fields use tick-based representation per SPEC017.

Tick System (per SPEC017):
  - Tick rate: 28,224,000 Hz (LCM of all supported sample rates)
  - One tick = 1/28,224,000 second ≈ 35.4 nanoseconds
  - Ensures sample-accurate precision across all sample rates
  - Reference: SPEC017 [SRC-TICK-020], [SRC-TICK-030]

Passages Table Timing Fields (all INTEGER, i64 ticks):

  **start_time** (ticks from file start)
    - Passage start boundary
    - Example: 0 ticks (file start)

  **end_time** (ticks from file start)
    - Passage end boundary
    - Example: 6,618,528,000 ticks (234.5 seconds)

  **fade_in_point** (ticks from file start, NULL = use global default)
    - Crossfade fade-in completion point
    - Example: 56,448,000 ticks (2.0 seconds from start)

  **fade_out_point** (ticks from file start, NULL = use global default)
    - Crossfade fade-out start point
    - Example: 6,562,080,000 ticks (232.5 seconds from start)

  **lead_in_point** (ticks from file start, NULL = use global default)
    - Amplitude analysis lead-in end point (when actual audio content starts)
    - Example: 42,336,000 ticks (1.5 seconds from start)
    - See: SPEC025 Amplitude Analysis

  **lead_out_point** (ticks from file start, NULL = use global default)
    - Amplitude analysis lead-out start point (when actual audio content ends)
    - Example: 6,576,192,000 ticks (233.0 seconds from start)
    - See: SPEC025 Amplitude Analysis

  **Reference:** SPEC017 [SRC-DB-010] through [SRC-DB-016]

Conversion Formulas (per SPEC017):

  **Samples → Ticks:**
    ticks = samples × (28,224,000 ÷ sample_rate)

    Example: 220,500 samples at 44.1kHz
      ticks = 220,500 × (28,224,000 ÷ 44,100)
      ticks = 220,500 × 640 = 141,120,000 ticks

  **Ticks → Seconds** (for UI display only):
    seconds = ticks ÷ 28,224,000

    Example: 141,120,000 ticks
      seconds = 141,120,000 ÷ 28,224,000 = 5.0 seconds

Layer Distinctions (per SPEC017 [SRC-LAYER-010]):

  **Developer-Facing Layers (USE TICKS):**
    - Database (passages table - THIS SPECIFICATION)
    - REST API (passage timing endpoints)
    - SSE Events (internal timing fields)
    - Developer UI (display ticks AND seconds)

  **User-Facing Layers (USE SECONDS):**
    - End-user UI (web interface display)
    - SSE Events (display fields for user consumption)
    - Human-readable logs

Boundary Detection (Tick-Based):

  Process:
    1. Detect passage boundaries in samples (native sample rate)
    2. IMMEDIATELY convert to ticks using formula above
    3. Store in PassageBoundary structure (ticks)
    4. Write to database (INTEGER ticks)
    5. SSE events: Include BOTH ticks (internal) AND seconds (display)

  Example SSE Event:
    ```json
    {
      "event": "PassagesDiscovered",
      "data": {
        "file_path": "file.flac",
        "passage_count": 3,
        "boundaries": [
          {
            "start_time_ticks": 0,
            "end_time_ticks": 141120000,
            "start_time_seconds": 0.0,
            "end_time_seconds": 5.0
          },
          ...
        ]
      }
    }
    ```

  Rationale:
    - Developer layers need ticks for sample-accurate precision
    - User layers display seconds for human readability
    - Conversion happens ONCE at user-facing boundary (not repeatedly)

NO Seconds in Database:
  - Database is developer-facing layer (per SPEC017 [SRC-LAYER-011])
  - NEVER store seconds in database (floating-point rounding errors)
  - ALWAYS store ticks (INTEGER, exact, no precision loss)
  - Convert to seconds ONLY for end-user UI/SSE display

Validation:
  - All timing fields MUST be INTEGER (i64) type
  - All timing values MUST be non-negative
  - start_time < end_time (basic sanity check)
  - fade_in_point ≤ fade_out_point (if both non-NULL)
  - lead_in_point ≤ lead_out_point (if both non-NULL)
```

#### [AIA-DB-050] SPEC031 Zero-Conf Schema Maintenance Integration

**Content:**
```markdown
**[AIA-DB-050]** SPEC031 Zero-Conf Schema Maintenance Integration

Purpose: Leverage SPEC031 data-driven schema maintenance to make extensive database changes transparent to users.

**Critical Context:**

CHANGE 6 adds **15+ new fields** to the passages table and introduces a **new import_provenance table**. Without automatic schema maintenance, this would require:
- Manual ALTER TABLE statements
- User intervention during upgrades
- Risk of schema drift between installations
- Complex migration documentation

**SPEC031 Solution: Automatic Schema Synchronization**

SPEC031 (Data-Driven Schema Maintenance) provides zero-configuration database evolution:
- Schema definitions are single source of truth
- System automatically detects missing columns on startup
- ALTER TABLE statements generated and applied automatically
- No manual migrations required for column additions
- Users never see database maintenance prompts

**Integration Requirements for SPEC032:**

1. **TableSchema Implementation for Passages Table**
   ```rust
   impl TableSchema for PassageSchema {
       fn table_name() -> &'static str { "passages" }

       fn expected_columns() -> Vec<ColumnDefinition> {
           vec![
               // Existing fields (omitted for brevity)

               // New fields from CHANGE 6 (auto-added via SPEC031):
               ColumnDefinition::new("flavor_source_blend", "TEXT"),
               ColumnDefinition::new("flavor_confidence_map", "TEXT"),
               ColumnDefinition::new("title_source", "TEXT"),
               ColumnDefinition::new("title_confidence", "REAL"),
               ColumnDefinition::new("artist_source", "TEXT"),
               ColumnDefinition::new("artist_confidence", "REAL"),
               ColumnDefinition::new("recording_mbid", "TEXT"),
               ColumnDefinition::new("identity_confidence", "REAL"),
               ColumnDefinition::new("identity_conflicts", "TEXT"),
               ColumnDefinition::new("overall_quality_score", "REAL"),
               ColumnDefinition::new("metadata_completeness", "REAL"),
               ColumnDefinition::new("flavor_completeness", "REAL"),
               ColumnDefinition::new("validation_status", "TEXT"),
               ColumnDefinition::new("validation_report", "TEXT"),
               ColumnDefinition::new("import_session_id", "TEXT"),
               ColumnDefinition::new("import_timestamp", "INTEGER"),
               ColumnDefinition::new("import_strategy", "TEXT"),
           ]
       }
   }
   ```

2. **TableSchema Implementation for import_provenance Table**
   ```rust
   impl TableSchema for ImportProvenanceSchema {
       fn table_name() -> &'static str { "import_provenance" }

       fn expected_columns() -> Vec<ColumnDefinition> {
           vec![
               ColumnDefinition::new("id", "TEXT").primary_key(),
               ColumnDefinition::new("passage_id", "TEXT").not_null(),
               ColumnDefinition::new("source_type", "TEXT").not_null(),
               ColumnDefinition::new("data_extracted", "TEXT"),
               ColumnDefinition::new("confidence", "REAL"),
               ColumnDefinition::new("timestamp", "INTEGER"),
           ]
       }
   }
   ```

3. **Automatic Sync on Startup**

   Per SPEC031 [ARCH-DB-SYNC-010] through [ARCH-DB-SYNC-030]:
   ```rust
   // In wkmp-ai init_database() or main():

   // Phase 1: CREATE TABLE IF NOT EXISTS (creates table skeletons)
   create_passages_table(&pool).await?;
   create_import_provenance_table(&pool).await?;

   // Phase 2: Schema Auto-Sync (NEW - adds missing columns)
   SchemaSync::sync_table::<PassageSchema>(&pool).await?;
   SchemaSync::sync_table::<ImportProvenanceSchema>(&pool).await?;

   // Phase 3: Manual Migrations (for complex transformations)
   run_migrations(&pool).await?;
   ```

**User Experience:**

**Before SPEC031 (Manual):**
- User upgrades wkmp-ai to version with SPEC032 changes
- Application detects schema mismatch
- Error: "Database schema outdated. Run migration tool."
- User must run: `wkmp-ai --migrate-schema`
- Risk: User forgets, application won't start

**After SPEC031 (Automatic):**
- User upgrades wkmp-ai to version with SPEC032 changes
- Application starts normally
- SPEC031 detects 17 missing columns
- Logs: "Adding column: passages.flavor_source_blend (TEXT)" × 17
- Application ready in <1 second
- User sees no schema maintenance prompts

**Benefits:**

1. **Zero User Intervention:** Database evolves automatically during normal startup
2. **Development Velocity:** Add fields to schema definition → restart → done
3. **Error Prevention:** No "column not found" runtime errors (schema always current)
4. **Deployment Simplicity:** No migration steps in deployment documentation
5. **Version Independence:** Fresh install and upgraded install converge to identical schema

**Backward Compatibility:**

Per SPEC031 [ARCH-DB-MIG-030]:
- All new fields are additions (no renames, no deletions)
- NULL allowed for all new fields
- Existing passages rows remain valid (new fields default to NULL)
- Old wkmp-ai versions (pre-SPEC032) ignore unknown columns (SQLite compatibility)

**Testing Requirements:**

1. Fresh database initialization (all columns created correctly)
2. Upgrade from pre-SPEC032 schema (17 columns added automatically)
3. Idempotent sync (running twice produces no changes)
4. Concurrent initialization (no duplicate column errors)

**Reference:**

- SPEC031-data_driven_schema_maintenance.md (complete specification)
- SPEC031 [ARCH-DB-SYNC-010]: Declarative schema definition via TableSchema trait
- SPEC031 [ARCH-DB-SYNC-020]: Automatic column addition via ALTER TABLE
- SPEC031 [ARCH-DB-SYNC-030]: Schema introspection and drift detection via PRAGMA table_info

**SPEC032 Documentation Requirements:**

1. **Architecture Section:** Reference SPEC031 for database evolution strategy
2. **Database Schema Section:** State "All schema changes applied automatically per SPEC031"
3. **Deployment Section:** State "No manual migrations required"
4. **Developer Guide:** "To add field: Update TableSchema → restart"
```

---

## CHANGE 7: Granular SSE Events

### Current State (SPEC032)

- Generic file-based progress events
- Coarse granularity (file completed)
- No per-song visibility
- Limited operation detail

### Required State

- **10 per-song event types** for fine-grained tracking
- File-level AND song-level progress
- Detailed operation status per phase
- Real-time conflict/quality reporting

### New Sections Needed

#### [AIA-SSE-020] Per-Song Event Types

**Content:**
```markdown
**[AIA-SSE-020]** Per-Song SSE Event Types

Purpose: Provide granular real-time updates for each passage/song during import.

Event Catalog (10 event types):

**1. PassagesDiscovered**

  When: After Phase 0 (passage boundary detection complete)

  Payload:
    - file_path: String
    - passage_count: u32
    - boundaries: Vec<PassageBoundary> (with tick-based timing)

  Example:
    ```json
    {
      "event": "PassagesDiscovered",
      "data": {
        "session_id": "uuid",
        "file_path": "album.flac",
        "passage_count": 12,
        "boundaries": [
          {
            "start_time_ticks": 0,
            "end_time_ticks": 141120000,
            "start_time_seconds": 5.0,
            "end_time_seconds": 0.0,
            "confidence": 1.0,
            "detection_method": "silence_detection"
          },
          ...
        ],
        "timestamp": "2025-11-09T20:30:00Z"
      }
    }
    ```

**2. SongExtracting**

  When: Phase 1 start (audio segment extraction begins)

  Payload:
    - file_path: String
    - passage_index: u32 (0-based)
    - total_passages: u32
    - boundary: PassageBoundary

  Example:
    ```json
    {
      "event": "SongExtracting",
      "data": {
        "session_id": "uuid",
        "file_path": "album.flac",
        "passage_index": 2,
        "total_passages": 12,
        "boundary": {
          "start_time_ticks": 282240000,
          "end_time_ticks": 423360000,
          "start_time_seconds": 10.0,
          "end_time_seconds": 15.0
        },
        "timestamp": "2025-11-09T20:30:05Z"
      }
    }
    ```

**3. IdentityResolved**

  When: Phase 3 complete (Tier 2 identity resolution finished)

  Payload:
    - passage_index: u32
    - mbid: Uuid
    - confidence: f64
    - source: String
    - conflicts: Vec<String>

  Example:
    ```json
    {
      "event": "IdentityResolved",
      "data": {
        "session_id": "uuid",
        "file_path": "album.flac",
        "passage_index": 2,
        "total_passages": 12,
        "mbid": "abc-123-def-456",
        "confidence": 0.95,
        "source": "ID3+AcoustID agreement",
        "conflicts": [],
        "timestamp": "2025-11-09T20:30:07Z"
      }
    }
    ```

**4. MetadataFused**

  When: Phase 4 complete (Tier 2 metadata fusion finished)

  Payload:
    - passage_index: u32
    - title: String
    - artist: String
    - title_source: String
    - artist_source: String
    - conflicts: Vec<String>

  Example:
    ```json
    {
      "event": "MetadataFused",
      "data": {
        "session_id": "uuid",
        "file_path": "album.flac",
        "passage_index": 2,
        "title": "Breathe (In The Air)",
        "artist": "Pink Floyd",
        "title_source": "MusicBrainz",
        "artist_source": "MusicBrainz",
        "conflicts": [],
        "timestamp": "2025-11-09T20:30:10Z"
      }
    }
    ```

**5. FlavorSynthesized**

  When: Phase 5 complete (Tier 2 flavor synthesis finished)

  Payload:
    - passage_index: u32
    - completeness: f64 (percentage)
    - source_blend: Map<String, f64>
    - characteristic_count: u32

  Example:
    ```json
    {
      "event": "FlavorSynthesized",
      "data": {
        "session_id": "uuid",
        "file_path": "album.flac",
        "passage_index": 2,
        "completeness": 87.5,
        "source_blend": {
          "AcousticBrainz": 0.357,
          "Essentia": 0.321,
          "AudioDerived": 0.214,
          "ID3Genre": 0.107
        },
        "characteristic_count": 43,
        "timestamp": "2025-11-09T20:30:15Z"
      }
    }
    ```

**6. ValidationComplete**

  When: Phase 6 complete (Tier 3 validation finished)

  Payload:
    - passage_index: u32
    - status: String ("Pass" | "Warning" | "Fail")
    - quality_score: f64 (percentage)
    - warnings: Vec<String>
    - failures: Vec<String>

  Example:
    ```json
    {
      "event": "ValidationComplete",
      "data": {
        "session_id": "uuid",
        "file_path": "album.flac",
        "passage_index": 2,
        "status": "Pass",
        "quality_score": 94.0,
        "warnings": [],
        "failures": [],
        "timestamp": "2025-11-09T20:30:17Z"
      }
    }
    ```

**7. SongCompleted**

  When: Phase 7 complete (database persistence successful)

  Payload:
    - passage_index: u32
    - passage_id: Uuid
    - title: String
    - status: String ("Pass" | "Warning")

  Example:
    ```json
    {
      "event": "SongCompleted",
      "data": {
        "session_id": "uuid",
        "file_path": "album.flac",
        "passage_index": 2,
        "total_passages": 12,
        "passage_id": "passage-uuid-789",
        "title": "Breathe (In The Air)",
        "status": "Pass",
        "timestamp": "2025-11-09T20:30:18Z"
      }
    }
    ```

**8. SongFailed**

  When: Any phase fails for a song

  Payload:
    - passage_index: u32
    - phase: String
    - error: String
    - file_path: String

  Example:
    ```json
    {
      "event": "SongFailed",
      "data": {
        "session_id": "uuid",
        "file_path": "album.flac",
        "passage_index": 5,
        "total_passages": 12,
        "phase": "IdentityResolution",
        "error": "No MBID sources available (ID3 missing, AcoustID API failed)",
        "timestamp": "2025-11-09T20:30:25Z"
      }
    }
    ```

**9. FileImportStarted**

  When: File processing begins (before Phase 0)

  Payload:
    - file_path: String
    - index: u32 (file index in import session)
    - total_files: u32

  Example:
    ```json
    {
      "event": "FileImportStarted",
      "data": {
        "session_id": "uuid",
        "file_path": "album.flac",
        "index": 2581,
        "total_files": 5736,
        "timestamp": "2025-11-09T20:30:00Z"
      }
    }
    ```

**10. FileImportComplete**

  When: All passages in file processed

  Payload:
    - file_path: String
    - passages_total: u32
    - success_count: u32
    - warning_count: u32
    - fail_count: u32

  Example:
    ```json
    {
      "event": "FileImportComplete",
      "data": {
        "session_id": "uuid",
        "file_path": "album.flac",
        "index": 2581,
        "total_files": 5736,
        "passages_total": 12,
        "success_count": 10,
        "warning_count": 1,
        "fail_count": 1,
        "timestamp": "2025-11-09T20:30:45Z"
      }
    }
    ```
```

#### [AIA-SSE-030] Event Format Standard

**Content:**
```markdown
**[AIA-SSE-030]** SSE Event Format Standard

Purpose: Define standard format for all import SSE events.

SSE Protocol:

  Format:
    ```
    event: <event_type>
    data: <json_payload>

    ```

  Example:
    ```
    event: IdentityResolved
    data: {"session_id": "uuid", "passage_index": 2, ...}

    ```

Common Fields (all events):

  - session_id: Uuid (import session identifier)
  - timestamp: ISO 8601 datetime string (when event occurred)

File-Scoped Fields (file events + song events):

  - file_path: String (relative to root folder)
  - index: u32 (file index in import session, only for file events)
  - total_files: u32 (total files in import session)

Song-Scoped Fields (song events only):

  - passage_index: u32 (0-based index of passage within file)
  - total_passages: u32 (total passages in current file)

Field Types:

  - session_id, passage_id, mbid: UUID strings
  - file_path: String (POSIX path, forward slashes)
  - Counters (index, total_files, passage_index, etc.): u32
  - Confidences, quality scores: f64 (0.0-1.0 or 0.0-100.0 depending on field)
  - Tick-based timing: i64 (ticks from file start)
  - Second-based timing: f64 (for display only)
  - Source names, error messages: String
  - Arrays: JSON arrays
  - Objects: JSON objects

Event Ordering Guarantee:

  - Events for SAME passage are ordered (IdentityResolved before MetadataFused, etc.)
  - Events for DIFFERENT passages may arrive out of order (parallel workers)
  - Events for SAME file are ordered at file level (FileImportStarted before FileImportComplete)

Reconnection Handling:

  - Client may disconnect and reconnect
  - Missed events NOT replayed (fire-and-forget semantics)
  - Alternative: Use GET /import/status for current state snapshot
```

#### [AIA-SSE-040] Progress Calculation

**Content:**
```markdown
**[AIA-SSE-040]** Progress Calculation

Purpose: Define how progress is computed and displayed for file-level and song-level tracking.

File-Level Progress:

  Formula:
    progress_percentage = (completed_files / total_files) × 100

  Display:
    "Processing file 2,581 / 5,736 (45.0%)"

  Update Frequency:
    - On FileImportStarted event
    - On FileImportComplete event
    - Every 2 seconds (polling-based update)

Song-Level Progress (within current file):

  Formula:
    song_progress_percentage = (completed_passages / total_passages) × 100

  Display:
    "File 2,581: Song 3 / 5 (60.0%) - Phase 4: Fusing metadata"

  Update Frequency:
    - On each per-song event (SongExtracting, IdentityResolved, etc.)
    - Indicates which phase current song is in

Combined Display (simultaneous file + song tracking):

  Format:
    "Processing file X / Y: Song M / N - Phase P: <operation>"

  Example:
    "Processing file 2,581 / 5,736: Song 3 / 5 - Phase 4: Fusing metadata"

  Rationale:
    - User sees both macro progress (files) and micro progress (songs within file)
    - No "stuck" indicators (always shows current operation)

Overall Progress (aggregate across all files and songs):

  Total Songs Estimate (if not known exactly):
    avg_songs_per_file = completed_songs / completed_files
    estimated_total_songs = avg_songs_per_file × total_files

  Completed Songs Count:
    - Track via atomic counter (increment on SongCompleted event)

  Display:
    "Overall: 12,345 songs completed (estimated 27,000 total, 45.7%)"

  Rationale:
    - Total song count unknown until all files scanned
    - Estimate provides ETA calculation

Rate Calculation (files/second or songs/second):

  Formula:
    rate = completed_count / elapsed_seconds

  Display:
    "Processing rate: 23.5 files/sec" or "Processing rate: 87.2 songs/sec"

  ETA Calculation:
    remaining = total - completed
    eta_seconds = remaining / rate (if rate > 0)

  Display:
    "ETA: 4m 32s remaining"

  Update Frequency:
    - Recalculate every 2 seconds
    - Use rolling average (last 30 seconds) to smooth fluctuations
```

---

## CHANGE 8: GOV002 Compliance

### Current State

- SPEC032 uses AIA prefix (Audio Ingest Architecture)
- AIA not formally registered in GOV002
- Category codes defined ad-hoc in SPEC032

### Required State

- Formalize AIA document code in GOV002
- Define all category codes
- Add 4 new categories for hybrid fusion architecture

### Amendment Needed

#### GOV002 Document Code Registration

**Add to GOV002 document codes table (around line 80):**
```markdown
| AIA | audio_ingest_architecture.md | Audio Ingest system architecture and fusion engine |
```

#### GOV002 Category Codes Definition

**Add new section to GOV002 (after existing category definitions):**
```markdown
### AIA (audio_ingest_architecture.md)

| Code     | Section                    | Scope                                      |
|----------|----------------------------|--------------------------------------------|
| OV       | Overview                   | High-level system description              |
| MS       | Microservices              | Integration with WKMP microservices        |
| UI       | User Interface             | Web UI and user interaction                |
| DB       | Database                   | Database schema and storage                |
| COMP     | Components                 | Component responsibilities                  |
| WF       | Workflow                   | Import workflow state machine              |
| ASYNC    | Asynchronous               | Async processing and parallelism           |
| ARCH     | Architecture               | 3-tier hybrid fusion architecture (NEW)    |
| FUSION   | Fusion                     | Data fusion algorithms (NEW)               |
| QUAL     | Quality                    | Quality and confidence framework (NEW)     |
| ESSENT   | Essentia                   | Essentia integration (NEW)                 |
| SSE      | SSE Events                 | Server-Sent Events                         |
| POLL     | Polling                    | Polling fallback                           |
| INT      | Integration                | Integration with existing workflows        |
| ERR      | Error Handling             | Error categorization and reporting         |
| PERF     | Performance                | Performance targets and optimizations      |
| SEC      | Security                   | Security considerations                    |
| TEST     | Testing                    | Testing strategy                           |
```

---

## CHANGE 9: SPEC017 Tick-Based Timing Visibility

### Current State (SPEC032)

- Brief mention of tick-based timing
- No detailed conversion guidance
- Insufficient visibility for implementers

### Required State

- Prominent SPEC017 references throughout
- Explicit conversion formulas in relevant sections
- Clear layer distinctions (ticks vs seconds)

### Enhancement Needed

Already covered in [AIA-DB-040] above, additional sections:

#### [AIA-FUSION-040] Boundary Detection with Ticks

(Already included in CHANGE 3, section [AIA-FUSION-040])

#### [AIA-PERF-060] Amplitude Analysis with Ticks

**New section to add:**
```markdown
**[AIA-PERF-060]** Amplitude Analysis with Tick-Based Timing

Purpose: Ensure amplitude analysis produces tick-based timing points per SPEC017.

Amplitude Analysis (per SPEC025):

  Purpose:
    - Detect lead-in end point (when actual audio content starts)
    - Detect lead-out start point (when actual audio content ends)
    - Used for crossfade timing optimization

  Process:
    1. Analyze passage audio in samples (native sample rate)
    2. Compute RMS amplitude per frame
    3. Detect lead-in threshold crossing (sample position)
    4. Detect lead-out threshold crossing (sample position)
    5. IMMEDIATELY convert sample positions to ticks:
       lead_in_ticks = lead_in_samples × (28,224,000 ÷ sample_rate)
       lead_out_ticks = lead_out_samples × (28,224,000 ÷ sample_rate)

  Output:
    - lead_in_point: i64 (ticks from file start)
    - lead_out_point: i64 (ticks from file start)

  Database Storage:
    - lead_in_point INTEGER (ticks, per SPEC017 [SRC-DB-015])
    - lead_out_point INTEGER (ticks, per SPEC017 [SRC-DB-016])

  SSE Event (user-facing display):
    ```json
    {
      "event": "AmplitudeAnalysisComplete",
      "data": {
        "passage_index": 2,
        "lead_in_ticks": 42336000,
        "lead_in_seconds": 1.5,
        "lead_out_ticks": 6576192000,
        "lead_out_seconds": 233.0
      }
    }
    ```

  Conversion for Display:
    seconds = ticks ÷ 28,224,000

  Reference: SPEC017 [SRC-LAYER-010] through [SRC-LAYER-030]
```

---

## CHANGE 10: /plan Workflow Structure

### Current State

- SPEC032 is a specification document
- Organized by architectural sections

### Required State

- Reorganize for /plan workflow compatibility
- Per-requirement format with acceptance criteria
- Clear testable requirements
- Priority and dependency tracking

### Reorganization Needed

**Document Structure (no new content, just reorganization):**

```markdown
SPEC032: WKMP Audio Ingest Architecture

## Executive Summary
  - Problem statement
  - Solution overview (3-tier hybrid fusion)
  - Key innovations
  - Expected benefits

## Requirements (Enumerated, Testable)

  ### Architecture Requirements
    [AIA-ARCH-010] 3-Tier Fusion Engine
      Requirement: System SHALL implement 3-tier hybrid fusion architecture
      Acceptance Criteria:
        - AC-1: Tier 1 has 7 independent source extractors
        - AC-2: Tier 2 has 4 fusion modules
        - AC-3: Tier 3 has 3 validation components
      Test Scenarios:
        - TS-1: All extractors execute in parallel
        - TS-2: Fusion modules process sequentially per passage
      Priority: P0 (Critical path)
      Dependencies: None

    [AIA-ARCH-020] Tier 1 Source Extractors
      [Detail per-requirement format as above]

  ### Fusion Algorithm Requirements
    [AIA-FUSION-010] Identity Resolution
      [Detail per-requirement format]

  [Continue for all requirements...]

## Component Specifications
  - Detailed module designs
  - Interface contracts
  - Data structures
  - Algorithms

## Implementation Guidance
  - Recommended implementation order
  - Testing strategy
  - Acceptance criteria per requirement
  - Integration points

## Traceability
  - Requirement → Design → Implementation mapping
  - Cross-references to other specs (SPEC017, SPEC025, IMPL001)
  - GOV002 compliance verification
```

**Per-Requirement Template:**

```markdown
[AIA-FUSION-010] Identity Resolution (Bayesian)

**Requirement:**
The system SHALL fuse ID3 MBID and AcoustID MBID using Bayesian update algorithm with conflict detection.

**Inputs:**
- id3_mbid: Option<Uuid>
- acoustid_mbid: Uuid
- acoustid_confidence: f64 (0.0-1.0)

**Algorithm:**
[Detailed Bayesian formula - as specified earlier]

**Outputs:**
- resolved_mbid: Uuid
- identity_confidence: f64
- identity_conflicts: Vec<String>
- source: String

**Acceptance Criteria:**
- AC-1: When ID3 MBID matches AcoustID MBID, posterior confidence > both individual confidences
- AC-2: When ID3 MBID conflicts with AcoustID MBID, conflict flag is set
- AC-3: Conflict penalty applied (posterior_conf ×= 0.85)
- AC-4: Low confidence passages (< 0.7) are flagged
- AC-5: Algorithm produces deterministic results for same inputs

**Test Scenarios:**
- TS-1: ID3 and AcoustID agree with high confidence (expect posterior > 0.95)
- TS-2: ID3 and AcoustID conflict (expect conflict flag + penalty applied)
- TS-3: ID3 missing, AcoustID only (expect posterior = acoustid_conf)
- TS-4: AcoustID low confidence < 0.5 (expect low confidence flag)
- TS-5: Both sources missing (expect failure)

**Priority:** P0 (Critical path - cannot proceed without MBID)
**Dependencies:**
- [AIA-ARCH-020] Tier 1 extractors (ID3MetadataExtractor, AcoustIDClient)
**References:**
- SPEC_wkmp_ai_recode.md [REQ-AI-020] through [REQ-AI-024]
- GOV002 requirement enumeration standards

**Error Handling:**
- If both sources missing: Fail passage import (emit SongFailed event)
- If AcoustID API fails: Use ID3 MBID if present, else fail
- If Bayesian formula produces invalid result (< 0 or > 1): Log error, use max(id3_conf, acoustid_conf)
```

---

## Summary

**Total New Content Estimate:** 2000-2500 lines

**Breakdown by Change Category:**
1. Hybrid Architecture: ~400 lines (5 sections)
2. Processing Model: ~200 lines (2 sections updated)
3. Multi-Source Fusion: ~500 lines (3 sections + algorithms)
4. Confidence & Quality: ~400 lines (3 sections + framework)
5. Essentia Integration: ~150 lines (1 section)
6. Database Schema: ~400 lines (3 sections + SQL)
7. SSE Events: ~350 lines (3 sections + 10 event types)
8. GOV002 Compliance: ~50 lines (amendment)
9. SPEC017 Visibility: ~150 lines (2 sections)
10. /plan Structure: ~0 lines (reorganization, no new content)

**Implementation Recommendation:**
Use Approach 2 (Incremental Integration) to stage these changes across 5 coherent specification increments.

**Next Step:**
Run `/plan docs/SPEC032-audio_ingest_architecture.md` to generate detailed implementation plan for specification writing.
