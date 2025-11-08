# Requirements Index: PLAN023 - WKMP-AI Ground-Up Recode

**Plan:** PLAN023 - wkmp-ai Ground-Up Recode
**Source:** wip/SPEC_wkmp_ai_recode.md
**Total Requirements:** 46 requirement IDs (36 functional, 10 non-functional)
**Individual Statements:** 98 SHALL/SHOULD/MUST statements (atomic requirements)
**Created:** 2025-01-08

**Note on Requirement Counting:**
- **46 Requirement IDs** (e.g., REQ-AI-010, REQ-AI-011): Logical groupings of related requirements
- **98 Individual Statements**: Atomic SHALL/SHOULD/MUST bullets under each requirement ID
- This index tracks requirement IDs for organization; each ID contains 1-8 individual statements
- Each individual statement will have dedicated test assertion(s) in Phase 3

---

## Functional Requirements

| Req ID | Category | Brief Description | Line # | Priority | Type |
|--------|----------|-------------------|--------|----------|------|
| **REQ-AI-010** | Workflow | Per-Song Import Workflow (parent) | 74 | P0 | Functional |
| REQ-AI-011 | Workflow | Phase 0: Passage Boundary Detection | 78 | P0 | Functional |
| REQ-AI-012 | Workflow | Phase 1-6: Per-Song Processing | 84 | P0 | Functional |
| REQ-AI-013 | Workflow | Per-Song Error Isolation | 94 | P0 | Functional |
| **REQ-AI-020** | Identity | Identity Resolution (Bayesian Fusion) | 100 | P0 | Functional |
| REQ-AI-021 | Identity | Multi-Source MBID Resolution | 104 | P0 | Functional |
| REQ-AI-022 | Identity | Conflict Detection | 111 | P0 | Functional |
| REQ-AI-023 | Identity | Bayesian Update Algorithm | 117 | P0 | Functional |
| REQ-AI-024 | Identity | Low-Confidence Flagging | 124 | P1 | Functional |
| **REQ-AI-030** | Metadata | Metadata Fusion (Weighted Selection) | 128 | P0 | Functional |
| REQ-AI-031 | Metadata | Multi-Source Metadata Extraction | 132 | P0 | Functional |
| REQ-AI-032 | Metadata | Quality Scoring | 136 | P1 | Functional |
| REQ-AI-033 | Metadata | Field-Wise Selection Strategy | 141 | P0 | Functional |
| REQ-AI-034 | Metadata | Consistency Validation | 147 | P1 | Functional |
| **REQ-AI-040** | Flavor | Musical Flavor Synthesis | 152 | P0 | Functional |
| REQ-AI-041 | Flavor | Multi-Source Flavor Extraction (Parallel) | 156 | P0 | Functional |
| REQ-AI-042 | Flavor | Source Priority and Confidence | 163 | P0 | Functional |
| REQ-AI-043 | Flavor | Characteristic-Wise Weighted Averaging | 169 | P0 | Functional |
| REQ-AI-044 | Flavor | Normalization (sum to 1.0 ± 0.0001) | 175 | P0 | Functional |
| REQ-AI-045 | Flavor | Completeness Scoring | 180 | P1 | Functional |
| **REQ-AI-050** | Boundary | Passage Boundary Detection | 184 | P0 | Functional |
| REQ-AI-051 | Boundary | Silence Detection (Baseline) | 188 | P0 | Functional |
| REQ-AI-052 | Boundary | Multi-Strategy Fusion (Future) | 193 | P2 | Functional |
| REQ-AI-053 | Boundary | Boundary Validation | 198 | P0 | Functional |
| **REQ-AI-060** | Validation | Quality Validation (Tier 3) | 205 | P0 | Functional |
| REQ-AI-061 | Validation | Title Consistency Check | 209 | P1 | Functional |
| REQ-AI-062 | Validation | Duration Consistency Check | 215 | P1 | Functional |
| REQ-AI-063 | Validation | Genre-Flavor Alignment Check | 220 | P1 | Functional |
| REQ-AI-064 | Validation | Overall Quality Score | 227 | P0 | Functional |
| **REQ-AI-070** | Events | Real-Time SSE Event Streaming | 232 | P0 | Functional |
| REQ-AI-071 | Events | Event Types (10 events defined) | 236 | P0 | Functional |
| REQ-AI-072 | Events | Event Format (SSE protocol) | 248 | P0 | Functional |
| REQ-AI-073 | Events | Event Throttling (max 30/sec) | 254 | P1 | Functional |
| **REQ-AI-080** | Database | Database Schema Extensions | 259 | P0 | Functional |
| REQ-AI-081 | Database | Flavor Source Provenance | 263 | P0 | Functional |
| REQ-AI-082 | Database | Metadata Source Provenance | 267 | P0 | Functional |
| REQ-AI-083 | Database | Identity Resolution Tracking | 273 | P0 | Functional |
| REQ-AI-084 | Database | Quality Scores | 278 | P0 | Functional |
| REQ-AI-085 | Database | Validation Flags | 283 | P0 | Functional |
| REQ-AI-086 | Database | Import Metadata | 287 | P1 | Functional |
| REQ-AI-087 | Database | Import Provenance Log Table | 292 | P0 | Functional |

**Priority Legend:**
- **P0 (Critical):** 30 requirements - Core functionality, must implement
- **P1 (High):** 10 requirements - Important features, should implement
- **P2 (Medium):** 1 requirement - Future enhancement (multi-strategy boundary fusion)

---

## Non-Functional Requirements

| Req ID | Category | Brief Description | Line # | Priority | Type |
|--------|----------|-------------------|--------|----------|------|
| **REQ-AI-NF-010** | Performance | Performance Requirements | 307 | P0 | Non-Functional |
| REQ-AI-NF-011 | Performance | Sequential Processing Performance (≤2 min/song) | 309 | P1 | Non-Functional |
| REQ-AI-NF-012 | Performance | Parallel Extraction (Tier 1 concurrent) | 314 | P0 | Non-Functional |
| **REQ-AI-NF-020** | Reliability | Reliability Requirements | 319 | P0 | Non-Functional |
| REQ-AI-NF-021 | Reliability | Error Isolation (song-level) | 321 | P0 | Non-Functional |
| REQ-AI-NF-022 | Reliability | Graceful Degradation | 326 | P0 | Non-Functional |
| **REQ-AI-NF-030** | Maintainability | Maintainability Requirements | 331 | P0 | Non-Functional |
| REQ-AI-NF-031 | Maintainability | Modular Architecture (Tier 1/2/3 separation) | 333 | P0 | Non-Functional |
| REQ-AI-NF-032 | Maintainability | Testability (>90% coverage) | 339 | P0 | Non-Functional |
| **REQ-AI-NF-040** | Extensibility | Extensibility Requirements | 345 | P1 | Non-Functional |
| REQ-AI-NF-041 | Extensibility | New Source Integration (trait-based) | 347 | P1 | Non-Functional |
| REQ-AI-NF-042 | Extensibility | Future Optimizations (parallel, learning) | 352 | P2 | Non-Functional |

**Priority Distribution (Non-Functional):**
- **P0 (Critical):** 7 requirements
- **P1 (High):** 2 requirements
- **P2 (Medium):** 1 requirement

---

## Requirements Summary by Category

| Category | Count | P0 | P1 | P2 |
|----------|-------|----|----|-----|
| Workflow | 4 | 3 | 0 | 0 |
| Identity Resolution | 5 | 4 | 1 | 0 |
| Metadata Fusion | 5 | 3 | 2 | 0 |
| Musical Flavor | 6 | 5 | 1 | 0 |
| Boundary Detection | 4 | 3 | 0 | 1 |
| Validation | 5 | 2 | 3 | 0 |
| Events (SSE) | 4 | 3 | 1 | 0 |
| Database | 8 | 7 | 1 | 0 |
| Performance | 3 | 1 | 1 | 0 |
| Reliability | 3 | 2 | 0 | 0 |
| Maintainability | 3 | 2 | 0 | 0 |
| Extensibility | 3 | 0 | 2 | 1 |
| **TOTAL** | **46** | **30** | **10** | **6** |

---

## Detailed Requirements Descriptions

### Workflow Requirements (REQ-AI-010 series)

**REQ-AI-011: Phase 0 Passage Boundary Detection**
- Detect passage boundaries before per-song processing
- Emit PassagesDiscovered event with count
- Support silence detection (baseline)
- Future: multi-strategy fusion (silence + beat + structural)

**REQ-AI-012: Phase 1-6 Per-Song Processing**
- Process each passage sequentially through hybrid fusion pipeline
- Extract audio segment per passage
- Generate passage-specific fingerprint
- Perform identity resolution per passage
- Fuse metadata per passage
- Synthesize musical flavor per passage
- Validate quality per passage
- Create database passage record per song

**REQ-AI-013: Per-Song Error Isolation**
- Continue processing if one song fails
- Do NOT abort entire import on single failure
- Emit SongFailed event for failures
- Report aggregate results (successes/warnings/failures)

---

### Identity Resolution Requirements (REQ-AI-020 series)

**REQ-AI-021: Multi-Source MBID Resolution**
- Extract MusicBrainz Recording ID from ID3 tags (if present)
- Generate Chromaprint fingerprint
- Query AcoustID API with fingerprint
- Apply Bayesian update to fuse ID3 MBID + AcoustID MBID
- Compute posterior confidence (0.0-1.0)

**REQ-AI-022: Conflict Detection**
- Detect when ID3 MBID ≠ AcoustID MBID
- Select higher-confidence source when conflict detected
- Emit conflict flag with details
- Downgrade confidence when conflicts occur

**REQ-AI-023: Bayesian Update Algorithm**
- If ID3 MBID == AcoustID MBID: `posterior = 1 - (1 - prior) * (1 - acoustid)`
- If ID3 MBID ≠ AcoustID MBID: Select higher confidence, apply conflict penalty
- If only one source: Use available source with native confidence
- Prior confidence for ID3 embedded MBID: 0.9
- Prior confidence for no ID3 MBID: 0.0

**REQ-AI-024: Low-Confidence Flagging**
- Flag passages with `posterior_confidence < 0.7` as low confidence
- Flag conflicting passages with `confidence < 0.85` for manual review

---

### Metadata Fusion Requirements (REQ-AI-030 series)

**REQ-AI-031: Multi-Source Metadata Extraction**
- Extract metadata from ID3 tags (title, artist, album, genre)
- Fetch metadata from MusicBrainz API (title, artists with weights, work, release)

**REQ-AI-032: Quality Scoring**
- Compute metadata quality score based on completeness
- Weight MusicBrainz MBID presence with bonus score (1.5)
- Compute quality: `score / field_count`

**REQ-AI-033: Field-Wise Selection Strategy**
- Title: Prefer MusicBrainz if `identity_confidence > 0.85`, else ID3
- Artist: Prefer MusicBrainz if `identity_confidence > 0.85`, else ID3
- Album: Prefer MusicBrainz if available and high confidence, else ID3
- Genre: Always prefer ID3 (MusicBrainz lacks genre)

**REQ-AI-034: Consistency Validation**
- Perform fuzzy match (Levenshtein ratio) on title: ID3 vs MusicBrainz
- Flag conflict if `similarity < 0.85`
- Store source provenance for each field

---

### Musical Flavor Requirements (REQ-AI-040 series)

**REQ-AI-041: Multi-Source Flavor Extraction (Parallel)**
- Query AcousticBrainz API (if Recording MBID exists and pre-2022)
- Compute Essentia features (if Essentia installed)
- Derive audio features from Chromaprint analysis (spectral/temporal)
- Map ID3 genre string to characteristics (coarse mapping)
- Execute all extractors in parallel (async)

**REQ-AI-042: Source Priority and Confidence**
- AcousticBrainz: Confidence 1.0 (highest quality, pre-computed)
- Essentia: Confidence 0.9 (high quality, computed locally)
- Audio-derived: Confidence 0.6 (medium quality, basic features)
- ID3-derived: Confidence 0.3 (low quality, genre mapping)

**REQ-AI-043: Characteristic-Wise Weighted Averaging**
- Compute union of all characteristics across sources
- For each characteristic: `fused_value = Σ(confidence_i * value_i) / Σ(confidence_i)`
- Compute per-characteristic confidence map
- Store source blend (e.g., "AcousticBrainz": 0.6, "Essentia": 0.3)

**REQ-AI-044: Normalization**
- Normalize binary characteristics to sum to 1.0 ± 0.0001 (per MFL-DEF-030)
- Normalize complex characteristics (per category) to sum to 1.0 ± 0.0001
- Validate normalization and fail import if violated

**REQ-AI-045: Completeness Scoring**
- Compute `completeness = (present_characteristics / expected_characteristics) * 100%`
- Store completeness score in database

---

### Passage Boundary Detection Requirements (REQ-AI-050 series)

**REQ-AI-051: Silence Detection (Baseline)**
- Use RMS-based silence detection (default threshold: -60dB)
- Find segments below threshold
- Generate boundary candidates with confidence scores

**REQ-AI-052: Multi-Strategy Fusion (Future Extension)**
- SHOULD support beat tracking (tempo change detection) - P2
- SHOULD support structural analysis (intro/verse/chorus/outro) - P2
- SHOULD support metadata hints (ID3 track start times, cue sheets) - P2

**REQ-AI-053: Boundary Validation**
- Enforce minimum passage duration: 30 seconds
- Enforce maximum passage duration: 15 minutes
- Cluster nearby candidates (within 500ms tolerance)
- Compute consensus position via weighted averaging
- Aggregate confidence from multiple sources

---

### Quality Validation Requirements (REQ-AI-060 series)

**REQ-AI-061: Title Consistency Check**
- Compute Levenshtein ratio for ID3 title vs MusicBrainz title
- Pass: `similarity > 0.95`
- Warning: `0.80 < similarity ≤ 0.95`
- Fail: `similarity ≤ 0.80`

**REQ-AI-062: Duration Consistency Check**
- Compare ID3 duration vs audio file duration
- Pass: `|id3_duration - audio_duration| ≤ 1000ms`
- Fail: `|id3_duration - audio_duration| > 1000ms`

**REQ-AI-063: Genre-Flavor Alignment Check**
- Map ID3 genre to expected characteristics
- Compare expected vs actual musical flavor characteristics
- Pass: `avg_alignment > 0.7`
- Warning: `0.5 < avg_alignment ≤ 0.7`
- Fail: `avg_alignment ≤ 0.5`

**REQ-AI-064: Overall Quality Score**
- Compute `overall_quality = (passed_checks / total_checks) * 100%`
- Store validation status ("Pass", "Warning", "Fail")
- Store validation report (JSON with all check results)

---

### Real-Time SSE Event Requirements (REQ-AI-070 series)

**REQ-AI-071: Event Types**
- FileImportStarted: When file import begins
- PassagesDiscovered: After boundary detection (includes count)
- SongExtracting: When passage audio extraction begins
- IdentityResolved: After identity resolution (MBID, confidence, sources)
- MetadataFused: After metadata fusion (title, source, conflicts)
- FlavorSynthesized: After flavor synthesis (completeness, source blend)
- ValidationComplete: After validation (status, quality_score, warnings)
- SongCompleted: When passage creation succeeds (title, status)
- SongFailed: When song processing fails (error)
- FileImportComplete: When all passages processed (summary)

**REQ-AI-072: Event Format**
- Use SSE protocol (`event: <type>\ndata: <json>\n\n`)
- Serialize event data as JSON
- Include `passage_id` for all song-level events
- Include `file_path` for all file-level events

**REQ-AI-073: Event Throttling**
- Limit SSE updates to maximum 30 events/second
- Buffer events if emission rate exceeds limit
- Do NOT drop events (buffering, not dropping)

---

### Database Schema Requirements (REQ-AI-080 series)

**REQ-AI-081: Flavor Source Provenance**
- `flavor_source_blend TEXT`: JSON array of contributing sources
- `flavor_confidence_map TEXT`: JSON object mapping characteristics to confidence

**REQ-AI-082: Metadata Source Provenance**
- `title_source TEXT`: Source of title ("ID3", "MusicBrainz", "Conflict")
- `title_confidence REAL`: Confidence score for title (0.0-1.0)
- `artist_source TEXT`: Source of artist
- `artist_confidence REAL`: Confidence score for artist

**REQ-AI-083: Identity Resolution Tracking**
- `recording_mbid TEXT`: Final resolved Recording MBID
- `identity_confidence REAL`: Posterior confidence from Bayesian update
- `identity_conflicts TEXT`: JSON array of conflict reports

**REQ-AI-084: Quality Scores**
- `overall_quality_score REAL`: 0-100% from validation
- `metadata_completeness REAL`: 0-100% (filled fields / total fields)
- `flavor_completeness REAL`: 0-100% (present characteristics / expected)

**REQ-AI-085: Validation Flags**
- `validation_status TEXT`: "Pass", "Warning", "Fail", "Pending"
- `validation_report TEXT`: JSON object with full validation report

**REQ-AI-086: Import Metadata**
- `import_session_id TEXT`: UUID for import session
- `import_timestamp INTEGER`: Unix timestamp of import
- `import_strategy TEXT`: "HybridFusion" (for future mode support)

**REQ-AI-087: Import Provenance Log Table**
- Create `import_provenance` table with:
  - `id TEXT PRIMARY KEY`
  - `passage_id TEXT NOT NULL` (FK to passages)
  - `source_type TEXT NOT NULL` (ID3, AcoustID, MusicBrainz, etc.)
  - `data_extracted TEXT` (JSON)
  - `confidence REAL`
  - `timestamp INTEGER`

---

### Performance Requirements (REQ-AI-NF-010 series)

**REQ-AI-NF-011: Sequential Processing Performance**
- Process songs sequentially (one at a time)
- Complete average song in ≤ 2 minutes
- Complete 10-song album in ≤ 20 minutes (sequential baseline)

**REQ-AI-NF-012: Parallel Extraction**
- Execute Tier 1 extractors in parallel (within each song)
- Use Tokio async runtime for concurrent API calls
- Limit concurrent network requests to prevent API throttling

---

### Reliability Requirements (REQ-AI-NF-020 series)

**REQ-AI-NF-021: Error Isolation**
- Isolate errors to individual songs (not entire file)
- Do NOT propagate song-level errors to file-level processing
- Log all errors with context (passage_id, phase, source)

**REQ-AI-NF-022: Graceful Degradation**
- Create zero-song passages when identification fails
- Use partial flavor data when only subset of sources available
- Continue import with warnings when validation fails (non-fatal)

---

### Maintainability Requirements (REQ-AI-NF-030 series)

**REQ-AI-NF-031: Modular Architecture**
- Implement Tier 1 extractors as independent modules
- Implement Tier 2 fusion modules as independent functions
- Implement Tier 3 validation as independent module
- Use clear separation of concerns (extraction vs fusion vs validation)

**REQ-AI-NF-032: Testability**
- Provide unit tests for all fusion algorithms
- Provide integration tests for per-song workflow
- Provide end-to-end tests for complete import flow
- Achieve >90% code coverage

---

### Extensibility Requirements (REQ-AI-NF-040 series)

**REQ-AI-NF-041: New Source Integration**
- Support adding new Tier 1 extractors without modifying fusion logic
- Use trait-based abstraction for extractors
- Support dynamic source registration

**REQ-AI-NF-042: Future Optimizations**
- SHOULD support parallel song processing (future Phase 2) - P2
- SHOULD support user feedback learning (future Phase 3) - P2

---

## Requirements Traceability

**All requirements are traceable to:**
- Source specification: wip/SPEC_wkmp_ai_recode.md
- Analysis documents: wip/hybrid_import_analysis.md, wip/per_song_import_analysis/
- Existing WKMP specs: SPEC003-musical_flavor.md, REQ002-entity_definitions.md

**Requirements reference existing IDs:**
- Musical Flavor: MFL-DEF-030, MFL-DEF-040 (normalization)
- Entity Definitions: ENT-MP-030 (passage), ENT-CNST-010 (zero-song passages)

---

## Navigation

**This Document:** Requirements quick reference (read this for overview)
**Full Specification:** wip/SPEC_wkmp_ai_recode.md (1222 lines - reference by line number)
**Next:** scope_statement.md (in/out of scope, assumptions, constraints)
