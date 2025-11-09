# WKMP-AI Audio Import System: Ground-Up Recode Specification

**Document Type:** Technical Specification (for /plan workflow)
**Created:** 2025-01-08
**Status:** Draft for Implementation Planning
**Source:** Fusion of wip/hybrid_import_analysis.md and wip/per_song_import_analysis/

---

## Specification Directive

**This is a ground-up recode of wkmp-ai.**

- Keep existing wkmp-ai source for reference
- DO NOT copy legacy functions until they can be replaced with new design
- Recode must be clean of legacy code
- Focus on best implementation of new approaches

**This is NOT a phased approach** with minimal features to be expanded later. This is focused development to the final goal of a **complete automated import-identification-metadata capture process**.

Recent analysis documents (hybrid_import_analysis.md, per_song_import_analysis/) take precedence over existing documents in case of conflict.

---

## Executive Summary

### Problem Statement

Current wkmp-ai import process has critical limitations:

1. **File-level atomic processing**: No per-song granularity in multi-song files
2. **Linear override strategy**: Later data sources completely replace earlier ones, causing information loss
3. **AcousticBrainz obsolescence**: Service ended 2022, no multi-source flavor fusion
4. **No confidence framework**: All sources treated equally, no quality-based decisions
5. **Limited user feedback**: File-level progress only, no per-song status visibility

### Solution Architecture

**3-Tier Hybrid Fusion + Per-Song Sequential Processing + Real-Time SSE UI**

**Tier 1: Parallel Source Extractors**
- ID3 Metadata Extractor
- Chromaprint Fingerprint Analyzer
- AcoustID API Client
- MusicBrainz API Client
- Essentia Musical Flavor Analyzer
- Audio-Derived Feature Extractor
- ID3 Genre Mapping

**Tier 2: Confidence-Weighted Fusion Engine**
- Identity Resolution Module (Bayesian update with conflict detection)
- Metadata Fusion Module (field-wise weighted selection)
- Musical Flavor Synthesis Module (characteristic-wise weighted averaging)
- Passage Boundary Fusion Module (multi-strategy with validation)

**Tier 3: Quality Validation & Enrichment**
- Cross-source consistency checks
- Completeness scoring
- Conflict detection and flagging
- Quality score computation

**Workflow: Per-Song Sequential Processing**
- Phase 0: Passage boundary detection (entire file)
- Phases 1-6: For each passage, sequential processing through hybrid fusion pipeline
- Real-time SSE events at each stage (granular user feedback)
- Per-song error isolation (one failure doesn't abort entire import)

---

## Requirements

### Functional Requirements

#### [REQ-AI-010] Per-Song Import Workflow

The system SHALL process multi-song audio files on a per-song basis:

**[REQ-AI-011]** Phase 0: Passage Boundary Detection
- SHALL detect passage boundaries before any per-song processing
- SHALL emit `PassagesDiscovered` event with count
- SHALL support silence detection (baseline)
- SHOULD support multi-strategy fusion (silence + beat + structural analysis) in future

**[REQ-AI-012]** Phase 1-6: Per-Song Processing
- SHALL process each detected passage sequentially through hybrid fusion pipeline
- SHALL extract audio segment for each passage
- SHALL generate passage-specific fingerprint
- SHALL perform identity resolution per passage
- SHALL fuse metadata per passage
- SHALL synthesize musical flavor per passage
- SHALL validate quality per passage
- SHALL create database passage record per song

**[REQ-AI-013]** Per-Song Error Isolation
- SHALL continue processing remaining songs if one song fails
- SHALL NOT abort entire import due to single song failure
- SHALL emit `SongFailed` event for failed songs
- SHALL report aggregate results (successes, warnings, failures) at file completion

#### [REQ-AI-020] Identity Resolution (Hybrid Fusion Tier 2)

The system SHALL use Bayesian update with conflict detection to resolve recording identity:

**[REQ-AI-021]** Multi-Source MBID Resolution
- SHALL extract MusicBrainz Recording ID from ID3 tags (if present)
- SHALL generate Chromaprint fingerprint for passage audio
- SHALL query AcoustID API with fingerprint
- SHALL apply Bayesian update algorithm to fuse ID3 MBID + AcoustID MBID
- SHALL compute posterior confidence (0.0-1.0)

**[REQ-AI-022]** Conflict Detection
- SHALL detect when ID3 MBID ≠ AcoustID MBID
- SHALL select higher-confidence source when conflict detected
- SHALL emit conflict flag with details ("ID3 MBID conflicts with AcoustID")
- SHALL downgrade confidence when conflicts occur

**[REQ-AI-023]** Bayesian Update Algorithm
- If ID3 MBID == AcoustID MBID: `posterior_confidence = 1 - (1 - prior_conf) * (1 - acoustid_conf)`
- If ID3 MBID ≠ AcoustID MBID: Select higher-confidence source, apply conflict penalty
- If only one source: Use available source with its native confidence
- Prior confidence for ID3 embedded MBID: 0.9
- Prior confidence for no ID3 MBID: 0.0

**[REQ-AI-024]** Low-Confidence Flagging
- SHALL flag passages with `posterior_confidence < 0.7` as low confidence
- SHALL flag conflicting passages with `confidence < 0.85` for manual review

#### [REQ-AI-030] Metadata Fusion (Hybrid Fusion Tier 2)

The system SHALL use field-wise weighted selection to fuse metadata:

**[REQ-AI-031]** Multi-Source Metadata Extraction
- SHALL extract metadata from ID3 tags (title, artist, album, genre)
- SHALL fetch metadata from MusicBrainz API (title, artists with weights, work, release)

**[REQ-AI-032]** Quality Scoring
- SHALL compute metadata quality score based on completeness
- SHALL weight MusicBrainz MBID presence with bonus score (1.5)
- SHALL compute quality: `score / field_count`

**[REQ-AI-033]** Field-Wise Selection Strategy
- Title: Prefer MusicBrainz if `identity_confidence > 0.85`, else ID3
- Artist: Prefer MusicBrainz if `identity_confidence > 0.85`, else ID3
- Album: Prefer MusicBrainz if available and high confidence, else ID3
- Genre: Always prefer ID3 (MusicBrainz lacks genre)

**[REQ-AI-034]** Consistency Validation
- SHALL perform fuzzy match (Levenshtein ratio) on title: ID3 vs MusicBrainz
- SHALL flag conflict if `similarity < 0.85`
- SHALL store source provenance for each field

#### [REQ-AI-040] Musical Flavor Synthesis (Hybrid Fusion Tier 2)

The system SHALL use characteristic-wise weighted averaging to synthesize musical flavor:

**[REQ-AI-041]** Multi-Source Flavor Extraction (Parallel)
- SHALL query AcousticBrainz API (if Recording MBID exists and pre-2022)
- SHALL compute Essentia features (if Essentia installed)
- SHALL derive audio features from Chromaprint analysis (spectral/temporal)
- SHALL map ID3 genre string to characteristics (coarse mapping)
- SHALL execute all extractors in parallel (async)

**[REQ-AI-042]** Source Priority and Confidence
- AcousticBrainz: Confidence 1.0 (highest quality, pre-computed, peer-reviewed)
- Essentia: Confidence 0.9 (high quality, computed locally)
- Audio-derived: Confidence 0.6 (medium quality, basic features)
- ID3-derived: Confidence 0.3 (low quality, genre mapping)

**[REQ-AI-043]** Characteristic-Wise Weighted Averaging
- SHALL compute union of all characteristics across sources
- For each characteristic: `fused_value = Σ(confidence_i * value_i) / Σ(confidence_i)`
- SHALL compute per-characteristic confidence map
- SHALL store source blend (e.g., "AcousticBrainz": 0.6, "Essentia": 0.3, "ID3": 0.1)

**[REQ-AI-044]** Normalization
- SHALL normalize binary characteristics to sum to 1.0 ± 0.0001 (per [MFL-DEF-030])
- SHALL normalize complex characteristics (per category) to sum to 1.0 ± 0.0001 (per [MFL-DEF-030])
- SHALL validate normalization and fail import if violated

**[REQ-AI-045]** Completeness Scoring
- SHALL compute `completeness = (present_characteristics / expected_characteristics) * 100%`
- SHALL store completeness score in database

#### [REQ-AI-050] Passage Boundary Detection (Hybrid Fusion Tier 1)

The system SHALL detect passage boundaries using multi-strategy fusion:

**[REQ-AI-051]** Silence Detection (Baseline)
- SHALL use RMS-based silence detection (default threshold: -60dB)
- SHALL find segments below threshold
- SHALL generate boundary candidates with confidence scores

**[REQ-AI-052]** Multi-Strategy Fusion (Future Extension)
- SHOULD support beat tracking (tempo change detection)
- SHOULD support structural analysis (intro/verse/chorus/outro patterns)
- SHOULD support metadata hints (ID3 track start times, cue sheets)

**[REQ-AI-053]** Boundary Validation
- SHALL enforce minimum passage duration: 30 seconds
- SHALL enforce maximum passage duration: 15 minutes
- SHALL cluster nearby candidates (within 500ms tolerance)
- SHALL compute consensus position via weighted averaging
- SHALL aggregate confidence from multiple sources

#### [REQ-AI-060] Quality Validation (Hybrid Fusion Tier 3)

The system SHALL validate passage data quality using cross-source consistency checks:

**[REQ-AI-061]** Title Consistency Check
- SHALL compute Levenshtein ratio for ID3 title vs MusicBrainz title
- Pass: `similarity > 0.95`
- Warning: `0.80 < similarity ≤ 0.95`
- Fail: `similarity ≤ 0.80`

**[REQ-AI-062]** Duration Consistency Check
- SHALL compare ID3 duration vs audio file duration
- Pass: `|id3_duration - audio_duration| ≤ 1000ms`
- Fail: `|id3_duration - audio_duration| > 1000ms`

**[REQ-AI-063]** Genre-Flavor Alignment Check
- SHALL map ID3 genre to expected characteristics
- SHALL compare expected vs actual musical flavor characteristics
- Pass: `avg_alignment > 0.7`
- Warning: `0.5 < avg_alignment ≤ 0.7`
- Fail: `avg_alignment ≤ 0.5`

**[REQ-AI-064]** Overall Quality Score
- SHALL compute `overall_quality = (passed_checks / total_checks) * 100%`
- SHALL store validation status ("Pass", "Warning", "Fail")
- SHALL store validation report (JSON with all check results)

#### [REQ-AI-070] Real-Time SSE Event Streaming

The system SHALL broadcast Server-Sent Events for real-time UI updates:

**[REQ-AI-071]** Event Types
- `FileImportStarted`: Emitted when file import begins
- `PassagesDiscovered`: Emitted after boundary detection (includes count)
- `SongExtracting`: Emitted when passage audio extraction begins
- `IdentityResolved`: Emitted after identity resolution (includes MBID, confidence, sources)
- `MetadataFused`: Emitted after metadata fusion (includes title, source, conflicts)
- `FlavorSynthesized`: Emitted after flavor synthesis (includes completeness, source blend)
- `ValidationComplete`: Emitted after validation (includes status, quality_score, warnings)
- `SongCompleted`: Emitted when passage creation succeeds (includes title, status)
- `SongFailed`: Emitted when song processing fails (includes error)
- `FileImportComplete`: Emitted when all passages processed (includes summary)

**[REQ-AI-072]** Event Format
- SHALL use SSE protocol (`event: <type>\ndata: <json>\n\n`)
- SHALL serialize event data as JSON
- SHALL include `passage_id` for all song-level events
- SHALL include `file_path` for all file-level events

**[REQ-AI-073]** Event Throttling
- SHALL limit SSE updates to maximum 30 events/second
- SHALL buffer events if emission rate exceeds limit
- SHALL NOT drop events (buffering, not dropping)

#### [REQ-AI-075] UI Progress Reporting and User Feedback

The system SHALL provide timely, accurate, and clear progress feedback to users during import operations:

**[REQ-AI-075-01]** Real-Time Progress Updates
- SHALL update progress indicators within 1 second of starting any file operation
- SHALL update progress counters immediately (not batched) as each file is processed
- SHALL display current file being processed in real-time
- SHALL NOT allow progress indicators to appear "stuck" or frozen during processing
- **Rationale**: Users need immediate feedback that the system is working, especially during long-running imports

**[REQ-AI-075-02]** Estimated Time Remaining (ETA)
- SHALL calculate and display estimated time remaining after processing at least 5 files
- SHALL update ETA dynamically as processing speed varies
- SHALL use rolling average of actual processing time (not static estimate)
- SHALL display ETA in human-readable format (minutes and seconds: "12m 34s")
- **Rationale**: Users need to know how long the import will take to plan their workflow

**[REQ-AI-075-03]** File-by-File Processing Workflow Clarity
- SHALL clearly indicate that processing happens file-by-file, not in batch phases
- SHALL display current processing stage for the current file (e.g., "Scanning file 123/5000")
- SHALL NOT give the impression that all files are scanned first, then all extracted, etc.
- SHALL show per-file workflow stages: Scan → Extract → Fingerprint → Segment → Analyze → Flavor
- **Rationale**: The current UI gives false impression of batch processing (like legacy wkmp-ai), causing user confusion

**[REQ-AI-075-04]** Multi-Phase Progress Visualization
- SHALL display overall import progress (files processed / total files)
- SHALL display per-file progress through workflow stages
- SHALL indicate which stage each file is currently in:
  - **Scanning**: File discovery and database deduplication check
  - **Extracting**: ID3 metadata extraction
  - **Fingerprinting**: Chromaprint generation + AcoustID/MusicBrainz lookup
  - **Segmenting**: Passage boundary detection (silence/beat analysis)
  - **Analyzing**: Audio-derived features (RMS, spectral analysis)
  - **Flavoring**: Musical flavor synthesis (Essentia/genre mapping)
  - **Fusing**: Information fusion across sources (Tier 2 hybrid fusion)
  - **Validating**: Quality validation and consistency checks (Tier 3)
- **Rationale**: Users need to understand the comprehensive nature of the import process

**[REQ-AI-075-05]** Accurate Progress Counter Behavior
- SHALL increment progress counter for EVERY file processed (not just files saved to database)
- SHALL count files that are skipped (unchanged, duplicates) toward progress total
- SHALL display: "Processing file X of Y" where X increments for each file encountered
- SHALL NOT display: "Saved X of Y" which causes counter to appear stuck
- **Example**: For 5000 files where 4000 are unchanged:
  - ✅ Correct: "Scanning file 4523 of 5000 (ETA: 2m 15s)" [counter always increments]
  - ❌ Incorrect: "Saved 523 of 5000" [counter stuck at 523 while 4000 files are skipped]
- **Rationale**: Counter must reflect actual work being done, not just database writes

**[REQ-AI-075-06]** Current Operation Clarity
- SHALL display the specific operation being performed on the current file
- SHALL differentiate between:
  - "Checking file..." (hash calculation + database lookup)
  - "Skipping unchanged file..." (modification time match, no rehash)
  - "Skipping duplicate file..." (different path, same hash)
  - "Importing new file..." (hash calculation + metadata extraction + database save)
  - "Updating modified file..." (file changed since last import)
- SHALL update "Currently Processing" field with actual file path being processed
- **Rationale**: Users need to understand why some files process quickly (skipped) vs slowly (imported)

**[REQ-AI-075-07]** Per-Song Granularity Feedback
- SHALL emit progress events for each passage/song within multi-song files
- SHALL display passage-level progress: "File 1/100: Song 2/5 - Extracting"
- SHALL show per-song workflow stages (not just file-level)
- SHALL indicate when information fusion is occurring per song
- **Rationale**: Multi-song files require per-song feedback for user understanding

**[REQ-AI-075-08]** Error and Warning Visibility
- SHALL display errors and warnings in real-time as they occur
- SHALL NOT hide errors until end of import
- SHALL maintain error count visible throughout import
- SHALL allow users to see which files failed without stopping import
- **Rationale**: Users need to know about problems immediately, not after hours of processing

#### [REQ-AI-078] Database Initialization and Self-Repair

The system SHALL adhere to WKMP zero-configuration database initialization requirements:

**[REQ-AI-078-01]** Zero-Configuration Startup
- SHALL start without requiring manual database setup or configuration files
- SHALL satisfy **[REQ-NF-036]** (automatic database creation with default schema)
- SHALL satisfy **[REQ-NF-037]** (modules create missing tables/columns automatically)
- **Reference:** [ADR-003-zero_configuration_strategy.md](../docs/ADR-003-zero_configuration_strategy.md)

**[REQ-AI-078-02]** Self-Repair for Schema Changes
- SHALL automatically add missing columns to existing tables on startup
- SHALL handle schema version upgrades transparently via migration framework
- SHALL NOT fail startup if database schema is older version
- **Reference:** [IMPL001-database_schema.md](../docs/IMPL001-database_schema.md) lines 1273-1277

**[REQ-AI-078-03]** Migration Framework Integration
- SHALL use `wkmp_common::db::migrations` for schema version management
- SHALL apply pending migrations on startup (idempotent, transaction-wrapped)
- SHALL handle concurrent initialization race conditions safely
- **Reference:** [IMPL001-database_schema.md](../docs/IMPL001-database_schema.md) lines 1255-1268

**[REQ-AI-078-04]** Breaking Changes Handling
- SHALL provide automated migration for `files.duration` → `files.duration_ticks` conversion
- Conversion formula: `ticks = CAST(seconds * 28224000 AS INTEGER)`
- 28224000 = 44100 Hz × 640 ticks/sample (WKMP tick rate per SPEC017)
- **Reference:** [IMPL001-database_schema.md](../docs/IMPL001-database_schema.md) lines 145-148

**Rationale:**
- Per project charter, 95% of users should experience zero-configuration startup
- Database schema evolution must be transparent to users (no manual SQL scripts)
- Existing databases must be upgraded automatically without data loss

#### [REQ-AI-080] Database Schema Extensions

The system SHALL extend the passages table with source provenance tracking:

**[REQ-AI-081]** Flavor Source Provenance
- `flavor_source_blend TEXT`: JSON array of contributing sources (e.g., `["AcousticBrainz", "Essentia"]`)
- `flavor_confidence_map TEXT`: JSON object mapping characteristics to confidence scores

**[REQ-AI-082]** Metadata Source Provenance
- `title_source TEXT`: Source of title ("ID3", "MusicBrainz", "Conflict")
- `title_confidence REAL`: Confidence score for title (0.0-1.0)
- `artist_source TEXT`: Source of artist
- `artist_confidence REAL`: Confidence score for artist

**[REQ-AI-083]** Identity Resolution Tracking
- `recording_mbid TEXT`: Final resolved Recording MBID
- `identity_confidence REAL`: Posterior confidence from Bayesian update
- `identity_conflicts TEXT`: JSON array of conflict reports

**[REQ-AI-084]** Quality Scores
- `overall_quality_score REAL`: 0-100% from validation
- `metadata_completeness REAL`: 0-100% (filled fields / total fields)
- `flavor_completeness REAL`: 0-100% (present characteristics / expected)

**[REQ-AI-085]** Validation Flags
- `validation_status TEXT`: "Pass", "Warning", "Fail", "Pending"
- `validation_report TEXT`: JSON object with full validation report

**[REQ-AI-086]** Import Metadata
- `import_session_id TEXT`: UUID for import session (group passages from same import)
- `import_timestamp INTEGER`: Unix timestamp of import
- `import_strategy TEXT`: "HybridFusion" (for future mode support)

**[REQ-AI-087]** Import Provenance Log Table
```sql
CREATE TABLE import_provenance (
    id TEXT PRIMARY KEY,
    passage_id TEXT NOT NULL,
    source_type TEXT NOT NULL,  -- "ID3", "AcoustID", "MusicBrainz", "Essentia", etc.
    data_extracted TEXT,  -- JSON: What data was extracted
    confidence REAL,
    timestamp INTEGER,
    FOREIGN KEY (passage_id) REFERENCES passages(id) ON DELETE CASCADE
);
```

#### [REQ-AI-088] SPEC017 Time Representation Compliance

**CRITICAL:** All passage timing fields SHALL comply with SPEC017 tick-based timing.

**[REQ-AI-088-01]** Tick Definition (per SPEC017 [SRC-TICK-030])
- One tick = 1/28,224,000 second (≈35.4 nanoseconds)
- Tick rate = LCM of all supported audio sample rates
- Ensures sample-accurate precision across all sample rates

**[REQ-AI-088-02]** Database Storage (per SPEC017 [SRC-DB-010] through [SRC-DB-016])
- `start_time INTEGER NOT NULL` - Passage start boundary (ticks from file start)
- `end_time INTEGER NOT NULL` - Passage end boundary (ticks from file start)
- `fade_in_point INTEGER` - Fade-in completion point (ticks from file start, NULL = use global default)
- `fade_out_point INTEGER` - Fade-out start point (ticks from file start, NULL = use global default)
- `lead_in_point INTEGER` - Lead-in end point (ticks from file start, NULL = use global default)
- `lead_out_point INTEGER` - Lead-out start point (ticks from file start, NULL = use global default)

**[REQ-AI-088-03]** Internal Representation (per SPEC017 [SRC-LAYER-011])
- SHALL use `i64` (64-bit signed integer) ticks for all internal timing
- SHALL use ticks in PassageBoundary structures
- SHALL use ticks in API requests/responses
- SHALL use ticks in database queries

**[REQ-AI-088-04]** Conversion Rules (per SPEC017 [SRC-LAYER-030])
- Samples to ticks: `ticks = samples × (28,224,000 ÷ sample_rate)`
- Ticks to seconds (display only): `seconds = ticks ÷ 28,224,000`
- SHALL convert to seconds ONLY for user-facing UI/SSE events
- SHALL NEVER store seconds in database

**[REQ-AI-088-05]** Boundary Detection
- SHALL detect passage boundaries in sample counts
- SHALL immediately convert sample counts to ticks
- SHALL emit boundary events with ticks (convert to seconds for SSE display)
- SHALL store ticks in PassageBoundary structures

**Rationale:**
- Database is "developer-facing layer" per SPEC017 [SRC-LAYER-011]
- Ticks ensure sample-accurate precision (no rounding errors)
- Enables exact crossfade point calculation in wkmp-ap
- Ensures interoperability with other WKMP modules

**Migration Note:**
- If existing database has REAL seconds columns, migration required
- Convert: `ticks = CAST(seconds * 28224000 AS INTEGER)`
- Add new INTEGER columns, populate from REAL, drop REAL

### Non-Functional Requirements

#### [REQ-AI-NF-010] Performance

**[REQ-AI-NF-011]** Sequential Processing Performance
- SHALL process songs sequentially (one at a time)
- SHOULD complete average song in ≤ 2 minutes
- SHOULD complete 10-song album in ≤ 20 minutes (sequential baseline)

**[REQ-AI-NF-012]** Parallel Extraction
- SHALL execute Tier 1 extractors in parallel (within each song)
- SHALL use Tokio async runtime for concurrent API calls
- SHALL limit concurrent network requests to prevent API throttling

#### [REQ-AI-NF-020] Reliability

**[REQ-AI-NF-021]** Error Isolation
- SHALL isolate errors to individual songs (not entire file)
- SHALL NOT propagate song-level errors to file-level processing
- SHALL log all errors with context (passage_id, phase, source)

**[REQ-AI-NF-022]** Graceful Degradation
- SHALL create zero-song passages when identification fails
- SHALL use partial flavor data when only subset of sources available
- SHALL continue import with warnings when validation fails (non-fatal)

#### [REQ-AI-NF-030] Maintainability

**[REQ-AI-NF-031]** Modular Architecture
- SHALL implement Tier 1 extractors as independent modules
- SHALL implement Tier 2 fusion modules as independent functions
- SHALL implement Tier 3 validation as independent module
- SHALL use clear separation of concerns (extraction vs fusion vs validation)

**[REQ-AI-NF-032]** Testability
- SHALL provide unit tests for all fusion algorithms
- SHALL provide integration tests for per-song workflow
- SHALL provide end-to-end tests for complete import flow
- SHALL achieve >90% code coverage

#### [REQ-AI-NF-040] Extensibility

**[REQ-AI-NF-041]** New Source Integration
- SHALL support adding new Tier 1 extractors without modifying fusion logic
- SHALL use trait-based abstraction for extractors
- SHALL support dynamic source registration

**[REQ-AI-NF-042]** Future Optimizations
- SHOULD support parallel song processing (future Phase 2)
- SHOULD support user feedback learning (future Phase 3)

---

## Architecture

### System Context

```
┌─────────────────────────────────────────────────────────────────┐
│ WKMP-AI Audio Import System (Ground-Up Recode)                 │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │ HTTP Server (Axum)                                        │ │
│  │  - POST /import/start                                     │ │
│  │  - GET /import/status                                     │ │
│  │  - GET /import/events (SSE endpoint)                      │ │
│  └───────────────────────────────────────────────────────────┘ │
│                              │                                  │
│                              ▼                                  │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │ Import Orchestrator                                       │ │
│  │  - File scanning                                          │ │
│  │  - Per-file import coordination                           │ │
│  │  - Event bus management                                   │ │
│  └───────────────────────────────────────────────────────────┘ │
│                              │                                  │
│                              ▼                                  │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │ Per-Song Import Engine (NEW)                              │ │
│  │  - Phase 0: Passage boundary detection                    │ │
│  │  - Phase 1-6: Sequential per-song processing              │ │
│  │  - Error isolation and recovery                           │ │
│  └───────────────────────────────────────────────────────────┘ │
│                              │                                  │
│                              ▼                                  │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │ 3-Tier Hybrid Fusion Engine (NEW)                         │ │
│  │                                                            │ │
│  │  TIER 1: Source Extractors (Parallel)                     │ │
│  │   - ID3MetadataExtractor                                  │ │
│  │   - ChromaprintAnalyzer                                   │ │
│  │   - AcoustIDClient                                        │ │
│  │   - MusicBrainzClient                                     │ │
│  │   - EssentiaAnalyzer                                      │ │
│  │   - AudioDerivedExtractor                                 │ │
│  │   - ID3GenreMapper                                        │ │
│  │                                                            │ │
│  │  TIER 2: Fusion Modules                                   │ │
│  │   - IdentityResolver (Bayesian)                           │ │
│  │   - MetadataFuser (Weighted selection)                    │ │
│  │   - FlavorSynthesizer (Weighted averaging)                │ │
│  │   - BoundaryFuser (Multi-strategy)                        │ │
│  │                                                            │ │
│  │  TIER 3: Validation & Enrichment                          │ │
│  │   - ConsistencyValidator                                  │ │
│  │   - QualityScorer                                         │ │
│  │   - ConflictDetector                                      │ │
│  └───────────────────────────────────────────────────────────┘ │
│                              │                                  │
│                              ▼                                  │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │ Database (SQLite) - wkmp-common                           │ │
│  │  - passages table (extended schema)                       │ │
│  │  - import_provenance table (NEW)                          │ │
│  └───────────────────────────────────────────────────────────┘ │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Component Interactions

#### Per-Song Import Flow

```
User → POST /import/start
       ↓
Import Orchestrator
       ↓
       For each audio file:
           ↓
           Per-Song Import Engine
               ↓
               Phase 0: Passage Boundary Detection
                   └→ detect_passage_boundaries() → Vec<Passage>
                   └→ emit PassagesDiscovered event
               ↓
               For each passage (sequential):
                   ↓
                   Phase 1: Extract Audio Segment
                       └→ extract_audio_segment() → AudioData
                       └→ emit SongExtracting event
                   ↓
                   Phase 2: Tier 1 Parallel Extraction
                       ├→ ID3MetadataExtractor::extract()
                       ├→ ChromaprintAnalyzer::analyze()
                       ├→ AcoustIDClient::lookup()
                       ├→ MusicBrainzClient::fetch()
                       ├→ EssentiaAnalyzer::compute()
                       ├→ AudioDerivedExtractor::derive()
                       └→ ID3GenreMapper::map()
                       (All execute in parallel via tokio::join!)
                   ↓
                   Phase 3: Tier 2 Identity Resolution
                       └→ IdentityResolver::resolve() → IdentityResolution
                       └→ emit IdentityResolved event
                   ↓
                   Phase 4: Tier 2 Metadata Fusion
                       └→ MetadataFuser::fuse() → FusedMetadata
                       └→ emit MetadataFused event
                   ↓
                   Phase 5: Tier 2 Flavor Synthesis
                       └→ FlavorSynthesizer::synthesize() → MusicalFlavorSynthesis
                       └→ emit FlavorSynthesized event
                   ↓
                   Phase 6: Tier 3 Validation
                       └→ ConsistencyValidator::validate() → ValidationReport
                       └→ QualityScorer::score() → QualityScores
                       └→ ConflictDetector::detect() → Vec<Conflict>
                       └→ emit ValidationComplete event
                   ↓
                   Phase 7: Passage Creation
                       └→ create_passage_record() → Passage
                       └→ db::insert_passage()
                       └→ db::insert_provenance_log()
                       └→ emit SongCompleted event
               ↓
           emit FileImportComplete event
       ↓
Return import session ID
```

### Data Flow: Single Song

```
Audio File Segment [0:00-3:45]
    ↓
┌─────────────────────────────────────────────┐
│ TIER 1: Parallel Extraction                 │
├─────────────────────────────────────────────┤
│ ID3Tags: {title, artist, mbid, genre, bpm}  │
│ Fingerprint: <chromaprint bytes>            │
│ AcoustID: {mbid, confidence: 0.92}          │
│ MusicBrainz: {title, artists, work}         │
│ Essentia: {characteristics map}             │
│ AudioDerived: {spectral features}           │
│ GenreMap: {genre → characteristics}         │
└─────────────────────────────────────────────┘
    ↓
┌─────────────────────────────────────────────┐
│ TIER 2: Fusion                              │
├─────────────────────────────────────────────┤
│ IdentityResolution:                         │
│   mbid: abc-123                             │
│   confidence: 0.95                          │
│   source: "AcoustID+ID3 (agreement)"        │
│                                             │
│ MetadataFusion:                             │
│   title: "Breathe (In The Air)"             │
│   title_source: "MusicBrainz"               │
│   artist: "Pink Floyd"                      │
│   artist_source: "MusicBrainz"              │
│                                             │
│ FlavorSynthesis:                            │
│   characteristics: {                        │
│     "danceability.danceable": 0.42,         │
│     "mood_aggressive.aggressive": 0.15,     │
│     ...                                     │
│   }                                         │
│   source_blend: {                           │
│     "AcousticBrainz": 0.6,                  │
│     "Essentia": 0.3,                        │
│     "ID3": 0.1                              │
│   }                                         │
│   completeness: 0.87                        │
└─────────────────────────────────────────────┘
    ↓
┌─────────────────────────────────────────────┐
│ TIER 3: Validation                          │
├─────────────────────────────────────────────┤
│ TitleConsistency: Pass (similarity: 0.98)   │
│ DurationConsistency: Pass (diff: 120ms)     │
│ GenreFlavorAlignment: Pass (alignment: 0.78)│
│ OverallQuality: 94%                         │
│ ValidationStatus: "Pass"                    │
└─────────────────────────────────────────────┘
    ↓
Database Passage Record
    + Import Provenance Log Entries
```

---

## Detailed Module Specifications

### Module: IdentityResolver

**Responsibility:** Fuse ID3 MBID + AcoustID MBID using Bayesian update

**Inputs:**
```rust
struct IdentityResolverInput {
    id3_mbid: Option<Uuid>,
    acoustid_result: AcoustIDResult,  // {mbid: Option<Uuid>, confidence: f64}
}
```

**Algorithm:**

```rust
fn resolve_identity(input: IdentityResolverInput) -> IdentityResolution {
    // Step 1: Establish prior belief
    let (prior_mbid, prior_confidence) = if let Some(mbid) = input.id3_mbid {
        (Some(mbid), 0.9)  // High trust in embedded MBID
    } else {
        (None, 0.0)  // No prior information
    };

    let acoustid_mbid = input.acoustid_result.mbid;
    let acoustid_confidence = input.acoustid_result.confidence;

    // Step 2: Bayesian update
    let (final_mbid, posterior_confidence, source, conflicts) =
        if prior_mbid == acoustid_mbid && prior_mbid.is_some() {
            // Agreement: Strengthen confidence
            let posterior = 1.0 - (1.0 - prior_confidence) * (1.0 - acoustid_confidence);
            (prior_mbid, posterior, "AcoustID+ID3 (agreement)", vec![])
        } else if prior_mbid.is_some() && acoustid_mbid.is_some() {
            // Conflict: Weighted selection
            if prior_confidence > acoustid_confidence {
                let posterior = prior_confidence * (1.0 - acoustid_confidence * 0.5);
                (prior_mbid, posterior, "ID3 (conflict)",
                 vec![ConflictReport {
                     conflict_type: "MBID mismatch",
                     details: format!("ID3: {:?}, AcoustID: {:?}", prior_mbid, acoustid_mbid),
                 }])
            } else {
                let posterior = acoustid_confidence * (1.0 - prior_confidence * 0.5);
                (acoustid_mbid, posterior, "AcoustID (conflict)",
                 vec![ConflictReport {
                     conflict_type: "MBID mismatch",
                     details: format!("ID3: {:?}, AcoustID: {:?}", prior_mbid, acoustid_mbid),
                 }])
            }
        } else {
            // One source only
            let final_mbid = prior_mbid.or(acoustid_mbid);
            let confidence = prior_confidence.max(acoustid_confidence);
            let source = if prior_mbid.is_some() { "ID3" } else { "AcoustID" };
            (final_mbid, confidence, source, vec![])
        };

    // Step 3: Flagging
    let mut flags = vec![];
    if posterior_confidence < 0.7 {
        flags.push("low_confidence".to_string());
    }
    if !conflicts.is_empty() && posterior_confidence < 0.85 {
        flags.push("manual_review_recommended".to_string());
    }

    IdentityResolution {
        recording_mbid: final_mbid,
        confidence: posterior_confidence,
        source: source.to_string(),
        conflicts,
        flags,
    }
}
```

**Output:**
```rust
struct IdentityResolution {
    recording_mbid: Option<Uuid>,
    confidence: f64,
    source: String,
    conflicts: Vec<ConflictReport>,
    flags: Vec<String>,
}
```

---

### Module: MetadataFuser

**Responsibility:** Field-wise weighted selection of metadata

**Inputs:**
```rust
struct MetadataFuserInput {
    id3_metadata: ID3Metadata,
    mb_metadata: MusicBrainzMetadata,
    identity_confidence: f64,
}
```

**Algorithm:**

```rust
fn fuse_metadata(input: MetadataFuserInput) -> FusedMetadata {
    let id3_quality = calculate_metadata_quality(&input.id3_metadata);
    let mb_quality = calculate_metadata_quality(&input.mb_metadata);

    let mut fused = FusedMetadata::default();
    let mut conflicts = vec![];

    // Title selection
    if input.mb_metadata.title.is_some() && input.identity_confidence > 0.85 {
        fused.title = input.mb_metadata.title.unwrap();
        fused.title_source = "MusicBrainz";
        fused.title_confidence = input.identity_confidence;

        // Consistency check
        if let Some(id3_title) = &input.id3_metadata.title {
            if !fuzzy_match(id3_title, &fused.title) {
                conflicts.push(ConflictReport {
                    conflict_type: "title_mismatch",
                    details: format!("ID3: {}, MusicBrainz: {}", id3_title, fused.title),
                });
            }
        }
    } else if let Some(id3_title) = input.id3_metadata.title {
        fused.title = id3_title;
        fused.title_source = "ID3";
        fused.title_confidence = id3_quality;
    } else {
        fused.title = "Unknown".to_string();
        fused.title_source = "Default";
        fused.title_confidence = 0.0;
    }

    // Artist selection (similar logic)
    // Album selection (similar logic)

    // Genre: Always prefer ID3
    fused.genre = input.id3_metadata.genre.clone();
    fused.genre_source = "ID3";

    // Compute overall quality
    fused.quality_score = compute_overall_quality(&fused);

    FusedMetadata {
        title: fused.title,
        title_source: fused.title_source,
        title_confidence: fused.title_confidence,
        artist: fused.artist,
        artist_source: fused.artist_source,
        artist_confidence: fused.artist_confidence,
        album: fused.album,
        genre: fused.genre,
        source_provenance: build_provenance_map(&fused),
        quality_score: fused.quality_score,
        conflicts,
    }
}

fn calculate_metadata_quality(metadata: &Metadata) -> f64 {
    let mut score = 0.0;
    let mut fields = 0;

    if metadata.title.is_some() { score += 1.0; fields += 1; }
    if metadata.artist.is_some() { score += 1.0; fields += 1; }
    if metadata.album.is_some() { score += 0.5; fields += 1; }
    if metadata.genre.is_some() { score += 0.3; fields += 1; }
    if metadata.musicbrainz_mbid.is_some() { score += 1.5; fields += 1; }

    if fields == 0 { 0.0 } else { score / fields as f64 }
}

fn fuzzy_match(a: &str, b: &str) -> bool {
    let similarity = levenshtein_ratio(a, b);
    similarity > 0.85
}
```

**Output:**
```rust
struct FusedMetadata {
    title: String,
    title_source: String,
    title_confidence: f64,
    artist: Vec<(String, f64)>,  // Artist + weight
    artist_source: String,
    artist_confidence: f64,
    album: Option<String>,
    genre: Option<String>,
    source_provenance: HashMap<String, String>,
    quality_score: f64,
    conflicts: Vec<ConflictReport>,
}
```

---

### Module: FlavorSynthesizer

**Responsibility:** Characteristic-wise weighted averaging of musical flavor

**Inputs:**
```rust
struct FlavorSynthesizerInput {
    recording_mbid: Option<Uuid>,
    audio_data: AudioData,
    id3_metadata: ID3Metadata,
}
```

**Algorithm:**

```rust
async fn synthesize_flavor(input: FlavorSynthesizerInput) -> MusicalFlavorSynthesis {
    // Step 1: Parallel extraction
    let (acousticbrainz, essentia, audio_derived, id3_derived) = tokio::join!(
        extract_acousticbrainz(input.recording_mbid),
        extract_essentia(&input.audio_data),
        extract_audio_derived(&input.audio_data),
        extract_id3_derived(&input.id3_metadata),
    );

    // Step 2: Collect successful sources
    let mut sources = vec![];
    if let Ok(ab) = acousticbrainz {
        sources.push(FlavorSource {
            characteristics: ab.characteristics,
            confidence: 1.0,
            source_type: "AcousticBrainz",
        });
    }
    if let Ok(es) = essentia {
        sources.push(FlavorSource {
            characteristics: es.characteristics,
            confidence: 0.9,
            source_type: "Essentia",
        });
    }
    if let Ok(ad) = audio_derived {
        sources.push(FlavorSource {
            characteristics: ad.characteristics,
            confidence: 0.6,
            source_type: "AudioDerived",
        });
    }
    if let Ok(id3) = id3_derived {
        sources.push(FlavorSource {
            characteristics: id3.characteristics,
            confidence: 0.3,
            source_type: "ID3Genre",
        });
    }

    // Step 3: Characteristic-wise fusion
    let mut fused_characteristics = HashMap::new();
    let all_characteristics: HashSet<String> = sources.iter()
        .flat_map(|s| s.characteristics.keys().cloned())
        .collect();

    for characteristic in all_characteristics {
        let mut weighted_sum = 0.0;
        let mut total_weight = 0.0;

        for source in &sources {
            if let Some(&value) = source.characteristics.get(&characteristic) {
                weighted_sum += value * source.confidence;
                total_weight += source.confidence;
            }
        }

        if total_weight > 0.0 {
            fused_characteristics.insert(
                characteristic.clone(),
                weighted_sum / total_weight
            );
        }
    }

    // Step 4: Normalization
    let normalized = normalize_characteristics(fused_characteristics);

    // Step 5: Compute metadata
    let confidence_map = compute_per_characteristic_confidence(&sources);
    let source_blend = sources.iter()
        .map(|s| (s.source_type.clone(), s.confidence))
        .collect();
    let completeness = (normalized.len() as f64 / EXPECTED_CHARACTERISTICS as f64) * 100.0;

    MusicalFlavorSynthesis {
        flavor_vector: normalized,
        confidence_map,
        source_blend,
        completeness,
    }
}

fn normalize_characteristics(chars: HashMap<String, f64>) -> HashMap<String, f64> {
    let mut normalized = HashMap::new();

    // Group by category (e.g., "danceability", "mood_aggressive")
    let categories = get_characteristic_categories(&chars);

    for category in categories {
        let values: Vec<_> = chars.iter()
            .filter(|(k, _)| k.starts_with(&category))
            .collect();

        let sum: f64 = values.iter().map(|(_, &v)| v).sum();

        if sum > 0.0 {
            for (key, value) in values {
                normalized.insert(key.clone(), value / sum);
            }
        }
    }

    // Validate normalization
    assert_normalized(&normalized);  // Panic if not normalized

    normalized
}
```

**Output:**
```rust
struct MusicalFlavorSynthesis {
    flavor_vector: HashMap<String, f64>,
    confidence_map: HashMap<String, f64>,
    source_blend: Vec<(String, f64)>,
    completeness: f64,
}
```

---

### Module: ConsistencyValidator

**Responsibility:** Cross-source consistency checks

**Inputs:**
```rust
struct ValidationInput {
    id3_metadata: ID3Metadata,
    fused_metadata: FusedMetadata,
    musical_flavor: MusicalFlavorSynthesis,
    audio_duration_ms: u64,
}
```

**Algorithm:**

```rust
fn validate(input: ValidationInput) -> ValidationReport {
    let mut checks = vec![];

    // Check 1: Title consistency
    if let Some(id3_title) = &input.id3_metadata.title {
        let similarity = levenshtein_ratio(id3_title, &input.fused_metadata.title);
        let result = if similarity > 0.95 {
            ValidationResult::Pass
        } else if similarity > 0.80 {
            ValidationResult::Warning(format!("Titles similar but not identical ({}%)", similarity * 100.0))
        } else {
            ValidationResult::Fail(format!("Title mismatch: ID3='{}', Fused='{}'", id3_title, input.fused_metadata.title))
        };
        checks.push(ValidationCheck {
            check_name: "TitleConsistency",
            result,
        });
    }

    // Check 2: Duration consistency
    if let Some(id3_duration) = input.id3_metadata.duration_ms {
        let diff_ms = (id3_duration as i64 - input.audio_duration_ms as i64).abs() as u64;
        let result = if diff_ms <= 1000 {
            ValidationResult::Pass
        } else {
            ValidationResult::Fail(format!("Duration mismatch: {}ms", diff_ms))
        };
        checks.push(ValidationCheck {
            check_name: "DurationConsistency",
            result,
        });
    }

    // Check 3: Genre-flavor alignment
    if let Some(genre) = &input.id3_metadata.genre {
        let expected_chars = genre_to_characteristics(genre);
        let mut alignment_score = 0.0;
        let mut checked = 0;

        for (char_name, expected_value) in expected_chars {
            if let Some(&actual_value) = input.musical_flavor.flavor_vector.get(&char_name) {
                alignment_score += 1.0 - (expected_value - actual_value).abs();
                checked += 1;
            }
        }

        if checked > 0 {
            let avg_alignment = alignment_score / checked as f64;
            let result = if avg_alignment > 0.7 {
                ValidationResult::Pass
            } else if avg_alignment > 0.5 {
                ValidationResult::Warning(format!("Moderate genre-flavor alignment ({}%)", avg_alignment * 100.0))
            } else {
                ValidationResult::Fail(format!("Poor genre-flavor alignment ({}%)", avg_alignment * 100.0))
            };
            checks.push(ValidationCheck {
                check_name: "GenreFlavorAlignment",
                result,
            });
        }
    }

    // Compute overall quality
    let passed = checks.iter().filter(|c| matches!(c.result, ValidationResult::Pass)).count();
    let overall_quality = (passed as f64 / checks.len() as f64) * 100.0;

    let validation_status = if checks.iter().any(|c| matches!(c.result, ValidationResult::Fail(_))) {
        "Fail"
    } else if checks.iter().any(|c| matches!(c.result, ValidationResult::Warning(_))) {
        "Warning"
    } else {
        "Pass"
    };

    let conflicts = checks.iter()
        .filter_map(|c| match &c.result {
            ValidationResult::Fail(msg) => Some(ConflictReport {
                conflict_type: c.check_name.clone(),
                details: msg.clone(),
            }),
            _ => None,
        })
        .collect();

    ValidationReport {
        checks,
        overall_quality,
        validation_status: validation_status.to_string(),
        conflicts,
    }
}
```

**Output:**
```rust
struct ValidationReport {
    checks: Vec<ValidationCheck>,
    overall_quality: f64,
    validation_status: String,
    conflicts: Vec<ConflictReport>,
}
```

---

## Testing Strategy

### Unit Tests

**Module: IdentityResolver**
```rust
#[test]
fn test_bayesian_update_agreement() {
    // Given: ID3 MBID = abc-123, AcoustID MBID = abc-123, confidence = 0.9
    // When: resolve_identity()
    // Then: posterior_confidence = 1 - (1 - 0.9) * (1 - 0.9) = 0.99
}

#[test]
fn test_bayesian_update_conflict() {
    // Given: ID3 MBID = abc-123 (conf 0.9), AcoustID MBID = xyz-789 (conf 0.6)
    // When: resolve_identity()
    // Then: final_mbid = abc-123, conflict flagged
}
```

**Module: FlavorSynthesizer**
```rust
#[test]
fn test_flavor_normalization() {
    // Given: Unnormalized characteristics
    // When: normalize_characteristics()
    // Then: Binary characteristics sum to 1.0 ± 0.0001
}

#[test]
fn test_multi_source_fusion() {
    // Given: AcousticBrainz + Essentia sources
    // When: synthesize_flavor()
    // Then: Fused values = weighted average
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_per_song_workflow() {
    // Given: Audio file with 3 passages
    // When: process_song() for each passage
    // Then: 3 passage records created, 3 SongCompleted events emitted
}

#[tokio::test]
async fn test_error_isolation() {
    // Given: Audio file with 3 passages, passage 2 has invalid MBID
    // When: process_file()
    // Then: Passages 1 and 3 succeed, passage 2 fails, SongFailed event emitted
}
```

### End-to-End Tests

```rust
#[tokio::test]
async fn test_complete_import_flow() {
    // Given: Test audio file "test_album.flac" with 5 songs
    // When: POST /import/start
    // Then:
    //   - PassagesDiscovered event (count=5)
    //   - 5x (IdentityResolved, MetadataFused, FlavorSynthesized, ValidationComplete, SongCompleted)
    //   - FileImportComplete event
    //   - 5 passage records in database
    //   - 5+ provenance log entries
}
```

---

## Migration Strategy

### Database Migration

**Migration: Add Source Provenance Fields**

```sql
-- Add new columns to passages table
ALTER TABLE passages ADD COLUMN flavor_source_blend TEXT;
ALTER TABLE passages ADD COLUMN flavor_confidence_map TEXT;
ALTER TABLE passages ADD COLUMN title_source TEXT;
ALTER TABLE passages ADD COLUMN title_confidence REAL;
ALTER TABLE passages ADD COLUMN artist_source TEXT;
ALTER TABLE passages ADD COLUMN artist_confidence REAL;
ALTER TABLE passages ADD COLUMN recording_mbid TEXT;
ALTER TABLE passages ADD COLUMN identity_confidence REAL;
ALTER TABLE passages ADD COLUMN identity_conflicts TEXT;
ALTER TABLE passages ADD COLUMN overall_quality_score REAL;
ALTER TABLE passages ADD COLUMN metadata_completeness REAL;
ALTER TABLE passages ADD COLUMN flavor_completeness REAL;
ALTER TABLE passages ADD COLUMN validation_status TEXT;
ALTER TABLE passages ADD COLUMN validation_report TEXT;
ALTER TABLE passages ADD COLUMN import_session_id TEXT;
ALTER TABLE passages ADD COLUMN import_timestamp INTEGER;
ALTER TABLE passages ADD COLUMN import_strategy TEXT;

-- Create import_provenance table
CREATE TABLE import_provenance (
    id TEXT PRIMARY KEY,
    passage_id TEXT NOT NULL,
    source_type TEXT NOT NULL,
    data_extracted TEXT,
    confidence REAL,
    timestamp INTEGER,
    FOREIGN KEY (passage_id) REFERENCES passages(id) ON DELETE CASCADE
);

CREATE INDEX idx_provenance_passage ON import_provenance(passage_id);
CREATE INDEX idx_passages_quality ON passages(overall_quality_score DESC);
CREATE INDEX idx_passages_validation ON passages(validation_status);
```

### Code Migration

**Step 1: Create New Module Structure**

```
wkmp-ai/
  src/
    main.rs
    import/
      mod.rs
      orchestrator.rs  (File-level coordination)
      per_song_engine.rs  (NEW: Per-song workflow)
    fusion/
      mod.rs
      extractors/  (NEW: Tier 1)
        mod.rs
        id3_extractor.rs
        chromaprint_analyzer.rs
        acoustid_client.rs
        musicbrainz_client.rs
        essentia_analyzer.rs
        audio_derived_extractor.rs
        id3_genre_mapper.rs
      fusers/  (NEW: Tier 2)
        mod.rs
        identity_resolver.rs
        metadata_fuser.rs
        flavor_synthesizer.rs
        boundary_fuser.rs
      validators/  (NEW: Tier 3)
        mod.rs
        consistency_validator.rs
        quality_scorer.rs
        conflict_detector.rs
    events/
      mod.rs
      import_events.rs  (Event types)
      sse_broadcaster.rs  (SSE logic)
    db/
      mod.rs
      passage_repository.rs  (Database operations)
      provenance_logger.rs  (Import provenance logging)
```

**Step 2: Implement Ground-Up Recode**

- DO NOT copy existing wkmp-ai functions
- Reference existing code for API contracts, database schema, event types
- Implement new architecture from scratch
- Use existing wkmp-common utilities (database pool, event bus)

---

## Open Questions for /plan

1. **Genre → Characteristics Mapping:** What is the complete mapping from ID3 genre strings to musical flavor characteristics? (Low priority - can use basic mapping initially)

2. **Levenshtein Ratio Implementation:** Use which library? (e.g., `strsim` crate)

3. **Expected Characteristics Count:** What is the total number of expected musical flavor characteristics for completeness scoring? (Reference SPEC003-musical_flavor.md)

4. **Event Buffering Strategy:** If SSE emission rate exceeds 30/sec, use in-memory buffer or disk-backed queue?

5. **Parallel Extraction Timeout:** What timeout for each Tier 1 extractor? (Prevent hanging on API calls)

6. **Zero-Song Passage Handling:** If identity resolution fails (no MBID), create zero-song passage or skip passage entirely? (Recommendation: Create zero-song passage per [ENT-CNST-010])

---

## Success Criteria

**This specification is successful if:**

1. ✅ `/plan` workflow can generate complete implementation plan without ambiguity
2. ✅ All functional requirements are testable (Given/When/Then)
3. ✅ Architecture supports ground-up recode (no legacy dependencies)
4. ✅ Hybrid fusion algorithms are fully specified (executable pseudocode)
5. ✅ Database schema supports all source provenance tracking
6. ✅ SSE event types cover all per-song workflow stages
7. ✅ Error isolation enables partial success (not all-or-nothing)

**Implementation is successful if:**

1. ✅ Per-song import workflow processes multi-song files correctly
2. ✅ Hybrid fusion produces higher-quality metadata than single-source
3. ✅ Musical flavor synthesis handles AcousticBrainz obsolescence (multi-source blending)
4. ✅ Real-time SSE UI shows per-song progress (not just file-level)
5. ✅ Validation detects conflicts and flags low-quality passages
6. ✅ Database records include source provenance for all fields
7. ✅ Error isolation prevents single-song failure from aborting entire import
8. ✅ Test coverage >90% for all fusion modules

---

**Document Status:** Draft - Ready for /plan Workflow
**Specification Completeness:** High (algorithms specified, architecture defined, requirements enumerated)
**Implementation Readiness:** Requires /plan for detailed increment breakdown and test specifications
