# Approach Selection: WKMP-AI Audio Import System Recode

**Plan:** PLAN024
**Created:** 2025-11-09
**Purpose:** Define architectural approach, technology selections, and design decisions for implementation

**Phase:** Phase 4 - Approach Selection

---

## Executive Summary

**Architectural Approach:** Modular 3-tier hybrid fusion with clear separation of concerns

**Key Decisions:**
- Tier 1: Independent extractor modules (7 modules, tokio parallel execution)
- Tier 2: Confidence-weighted fusion modules (4 modules, sequential per passage)
- Tier 3: Quality validation modules (3 modules, pipeline pattern)
- Technology stack: Rust stable, tokio async, FFI to C libraries where needed
- Database: SPEC031 zero-config schema sync (verify availability)

**Risk-First Evaluation:** All decisions evaluated for failure risk per CLAUDE.md Risk-First Framework

---

## Table of Contents

1. [Module Architecture](#module-architecture)
2. [Technology Selection](#technology-selection)
3. [Database Strategy](#database-strategy)
4. [Error Handling Strategy](#error-handling-strategy)
5. [Parallelization Strategy](#parallelization-strategy)
6. [API Client Design](#api-client-design)
7. [Testing Strategy](#testing-strategy)
8. [Decision Summary](#decision-summary)

---

## Module Architecture

### Tier 1: Source Extractors (Parallel)

**Architectural Pattern:** Independent modules with common trait

```rust
/// Tier 1 Extractor Trait - All extractors implement this
#[async_trait]
pub trait SourceExtractor {
    /// Extract data from passage audio/metadata
    async fn extract(&self, passage: &PassageContext) -> Result<ExtractionResult, ExtractionError>;

    /// Confidence score for this extractor (0.0-1.0)
    fn confidence(&self) -> f64;

    /// Extractor name for provenance tracking
    fn name(&self) -> &'static str;
}
```

**7 Extractor Modules:**

1. **`id3_extractor`**
   - Input: Audio file path
   - Output: ID3 metadata (title, artist, album, MBID if present)
   - Dependencies: `id3` crate or `mp3-metadata`
   - Confidence: 0.6 (user-editable metadata)

2. **`chromaprint_analyzer`**
   - Input: PCM f32 audio segment
   - Output: Base64 Chromaprint fingerprint
   - Dependencies: FFI to libchromaprint via `chromaprint-sys` or custom bindings
   - Confidence: N/A (fingerprint has no intrinsic confidence)

3. **`acoustid_client`**
   - Input: Chromaprint fingerprint + duration
   - Output: Candidate Recording [ENT-MB-020] MBIDs with scores
   - Dependencies: `reqwest` (HTTP client), `tokio::time` (rate limiting)
   - Confidence: AcoustID API provides score (0.0-1.0)

4. **`musicbrainz_client`**
   - Input: Recording [ENT-MB-020] MBID
   - Output: Recording metadata (title, artist, works, duration)
   - Dependencies: `reqwest`, `tokio::time` (rate limiting), XML parser
   - Confidence: 0.95 (authoritative database)

5. **`essentia_analyzer`** (optional)
   - Input: PCM f32 audio segment
   - Output: Musical flavor characteristics vector
   - Dependencies: Command execution (`essentia_streaming`) or FFI
   - Confidence: 0.9 (high-quality music analysis)
   - Detection: Runtime check per REQ-AI-041-02

6. **`audio_derived_extractor`**
   - Input: PCM f32 audio segment
   - Output: Basic musical flavor characteristics (tempo, loudness, spectral centroid)
   - Dependencies: Pure Rust audio analysis (custom implementation)
   - Confidence: 0.6 (basic features only)

7. **`id3_genre_mapper`**
   - Input: ID3 genre tag
   - Output: Musical flavor characteristics inferred from genre
   - Dependencies: Genre→flavor mapping table (in-memory HashMap)
   - Confidence: 0.5 (coarse mapping)

**Module Organization:**
```
wkmp-ai/src/extractors/
├── mod.rs              # SourceExtractor trait, parallel executor
├── id3_extractor.rs
├── chromaprint_analyzer.rs
├── acoustid_client.rs
├── musicbrainz_client.rs
├── essentia_analyzer.rs
├── audio_derived_extractor.rs
└── id3_genre_mapper.rs
```

**Parallel Execution Pattern:**
```rust
/// Execute all extractors in parallel for a passage
async fn extract_all(passage: &PassageContext) -> Vec<ExtractionResult> {
    let (id3, chromaprint, essentia, audio_derived, genre) = tokio::join!(
        id3_extractor.extract(passage),
        chromaprint_analyzer.extract(passage),
        essentia_analyzer.extract(passage),    // Gracefully skips if unavailable
        audio_derived_extractor.extract(passage),
        id3_genre_mapper.extract(passage),
    );

    // Sequential: AcoustID (needs chromaprint result)
    let acoustid = acoustid_client.extract_with_fingerprint(&chromaprint).await;

    // Sequential: MusicBrainz (needs MBID from AcoustID)
    let musicbrainz = musicbrainz_client.extract_with_mbid(&acoustid.mbid).await;

    vec![id3, chromaprint, acoustid, musicbrainz, essentia, audio_derived, genre]
}
```

---

### Tier 2: Fusion Modules (Sequential per Passage)

**Architectural Pattern:** Pipeline of fusion stages

**4 Fusion Modules:**

1. **`identity_resolver`** (REQ-AI-020)
   - Input: AcoustID MBIDs, MusicBrainz MBID, ID3 MBID (if present)
   - Algorithm: Bayesian update with conflict detection
   - Output: Resolved Recording [ENT-MB-020] MBID + confidence score
   - Dependencies: None (pure computation)

2. **`metadata_fuser`** (REQ-AI-030)
   - Input: ID3 metadata, MusicBrainz metadata
   - Algorithm: Field-wise weighted selection
   - Output: Fused metadata (title, artist, album, duration)
   - Dependencies: `strsim` (Levenshtein for consistency checks)

3. **`flavor_synthesizer`** (REQ-AI-040)
   - Input: AcousticBrainz flavor, Essentia flavor, AudioDerived flavor, ID3 genre flavor
   - Algorithm: Characteristic-wise weighted averaging
   - Output: Synthesized musical flavor vector + completeness score
   - Dependencies: None (pure computation)

4. **`boundary_fuser`** (REQ-AI-050)
   - Input: Initial boundaries (Phase 1), Recording duration (from metadata)
   - Algorithm: Boundary validation and refinement
   - Output: Refined Passage [ENT-MP-030] boundaries (i64 ticks)
   - Dependencies: SPEC017 tick conversion utilities

**Module Organization:**
```
wkmp-ai/src/fusion/
├── mod.rs              # Fusion pipeline orchestrator
├── identity_resolver.rs
├── metadata_fuser.rs
├── flavor_synthesizer.rs
└── boundary_fuser.rs
```

**Pipeline Pattern:**
```rust
/// Execute fusion pipeline for a passage
async fn fuse_all(
    extraction_results: Vec<ExtractionResult>,
    initial_boundaries: PassageBoundary,
) -> FusedPassage {
    // Tier 2 fusion (sequential, each depends on previous)
    let identity = identity_resolver.resolve(&extraction_results).await;
    let metadata = metadata_fuser.fuse(&extraction_results, &identity).await;
    let flavor = flavor_synthesizer.synthesize(&extraction_results).await;
    let boundaries = boundary_fuser.refine(initial_boundaries, &metadata.duration).await;

    FusedPassage { identity, metadata, flavor, boundaries }
}
```

---

### Tier 3: Quality Validators (Pipeline)

**Architectural Pattern:** Sequential validation checks

**3 Validator Modules:**

1. **`consistency_validator`** (REQ-AI-061, REQ-AI-062, REQ-AI-063)
   - Input: Fused metadata, fused flavor, ID3 metadata
   - Checks: Title consistency, duration consistency, genre-flavor alignment
   - Output: Validation flags + consistency score
   - Dependencies: `strsim` (Levenshtein ratio)

2. **`completeness_scorer`** (REQ-AI-045-01)
   - Input: Metadata fields, flavor characteristics
   - Algorithm: (present / expected) × 100%
   - Output: Metadata completeness, flavor completeness scores
   - Dependencies: Database parameter PARAM-AI-004

3. **`quality_scorer`** (REQ-AI-064)
   - Input: Identity confidence, completeness scores, consistency score
   - Algorithm: Weighted average of all quality signals
   - Output: Overall quality score (0.0-1.0)
   - Dependencies: None (pure computation)

**Module Organization:**
```
wkmp-ai/src/validation/
├── mod.rs              # Validation pipeline orchestrator
├── consistency_validator.rs
├── completeness_scorer.rs
└── quality_scorer.rs
```

**Validation Pipeline:**
```rust
/// Execute validation pipeline
async fn validate_all(fused: &FusedPassage) -> ValidationResult {
    // Sequential validation (fast, no I/O)
    let consistency = consistency_validator.validate(fused);
    let completeness = completeness_scorer.score(fused);
    let quality = quality_scorer.compute(fused, &consistency, &completeness);

    ValidationResult { consistency, completeness, quality }
}
```

---

## Technology Selection

### Rust Crates

**Audio Processing:**
- **`symphonia`** - Audio decoding (already in use per CLAUDE.md)
  - Output: PCM f32 samples (native format, no conversion needed)
  - Supports: MP3, FLAC, OGG, M4A, WAV
  - Risk: **LOW** - Stable, widely used, well-maintained

**HTTP and Async:**
- **`tokio`** - Async runtime (already in use per CLAUDE.md)
  - Features: `["full"]`
  - Risk: **LOW** - Standard async runtime for Rust

- **`axum`** - HTTP framework (already in use per CLAUDE.md)
  - SSE support for real-time events
  - Risk: **LOW** - Official tokio project

- **`reqwest`** - HTTP client for AcoustID/MusicBrainz APIs
  - Features: `["json"]`
  - Risk: **LOW** - De facto standard HTTP client

**Rate Limiting:**
- **`tower`** - Middleware framework
  - Use: `tower::limit::RateLimit` for API throttling
  - Risk: **LOW** - Official tokio ecosystem

**Alternative:** `governor` crate (dedicated rate limiting)
  - **Decision:** Use `tower` (already in dependencies, less crate bloat)

**Database:**
- **`rusqlite`** - SQLite driver (already in use per CLAUDE.md)
  - Features: `["bundled", "json"]` for JSON1 extension
  - Risk: **LOW** - Stable, widely used

**String Utilities:**
- **`strsim`** - String similarity (Levenshtein distance)
  - Use: Consistency validation (title matching)
  - Risk: **LOW** - Small, stable library

**Serialization:**
- **`serde`** / **`serde_json`** (already in use)
  - Use: JSON storage in database, API responses
  - Risk: **LOW** - Standard serialization

---

### External Libraries (FFI)

**Chromaprint (Audio Fingerprinting):**

**Options:**
1. **Pure Rust implementation** - Does not exist (as of 2025-11-09)
2. **`chromaprint-sys`** crate - FFI bindings (if exists)
3. **Custom FFI bindings** - Manual unsafe Rust to C

**Decision:** **Custom FFI bindings** (Option 3)

**Rationale (Risk-First):**
- **Failure Risk Analysis:**
  - Option 1: N/A (doesn't exist)
  - Option 2: UNKNOWN (crate may be unmaintained, incomplete)
  - Option 3: **LOW-MEDIUM** (manual FFI has safety risks, but we control it)

- **Risk Mitigation for Option 3:**
  - Wrap unsafe FFI in safe Rust API
  - Unit test all FFI boundary conditions
  - Reference: IMPL013-chromaprint_integration.md (contains workflow)

**System Dependency:**
```bash
# Ubuntu/Debian
apt-get install libchromaprint-dev

# macOS
brew install chromaprint
```

**FFI Wrapper Pattern:**
```rust
// wkmp-ai/src/ffi/chromaprint.rs
use std::ffi::CString;
use std::os::raw::{c_char, c_int};

#[link(name = "chromaprint")]
extern "C" {
    fn chromaprint_new(algorithm: c_int) -> *mut ChromaprintContext;
    fn chromaprint_start(ctx: *mut ChromaprintContext, sample_rate: c_int, num_channels: c_int) -> c_int;
    // ... other functions
}

/// Safe Rust wrapper
pub struct Chromaprint {
    ctx: *mut ChromaprintContext,
}

impl Chromaprint {
    pub fn new() -> Result<Self, ChromaprintError> {
        unsafe {
            let ctx = chromaprint_new(CHROMAPRINT_ALGORITHM_DEFAULT);
            if ctx.is_null() {
                return Err(ChromaprintError::InitFailed);
            }
            Ok(Chromaprint { ctx })
        }
    }

    pub fn generate_fingerprint(&mut self, audio: &[i16], sample_rate: u32) -> Result<String, ChromaprintError> {
        // Safe wrapper around unsafe FFI calls
        // See IMPL013 for full implementation
    }
}

impl Drop for Chromaprint {
    fn drop(&mut self) {
        unsafe {
            chromaprint_free(self.ctx);
        }
    }
}
```

---

**Essentia (Musical Analysis):**

**Options:**
1. **FFI to libessentia** - Complex C++ library, difficult FFI
2. **Command execution** - Call `essentia_streaming` binary, parse JSON output
3. **Skip Essentia entirely** - Use AudioDerived only

**Decision:** **Command execution** (Option 2)

**Rationale (Risk-First):**
- **Failure Risk Analysis:**
  - Option 1: **HIGH** - C++ FFI complex, ABI stability issues, hard to debug
  - Option 2: **LOW-MEDIUM** - Process execution overhead, JSON parsing, but isolated
  - Option 3: **MEDIUM** - Reduced quality (AudioDerived conf 0.6 vs Essentia 0.9)

- **Risk Mitigation for Option 2:**
  - Timeout on command execution (5 seconds per REQ-AI-041-02)
  - Graceful degradation if command fails
  - JSON parsing with `serde_json` (robust)
  - Subprocess isolation (failure doesn't crash wkmp-ai)

**Implementation Pattern:**
```rust
// wkmp-ai/src/extractors/essentia_analyzer.rs
use tokio::process::Command;
use tokio::time::timeout;
use std::time::Duration;

pub async fn extract_essentia_features(audio_path: &Path) -> Result<EssentiaFeatures, EssentiaError> {
    // Detection: Check if essentia_streaming exists
    let version_check = Command::new("essentia_streaming")
        .arg("--version")
        .output()
        .await;

    if version_check.is_err() {
        return Err(EssentiaError::NotInstalled);
    }

    // Execute with timeout
    let output = timeout(
        Duration::from_secs(5),
        Command::new("essentia_streaming")
            .arg(audio_path)
            .arg("--output")
            .arg("json")
            .output()
    ).await??;

    // Parse JSON output
    let features: EssentiaFeatures = serde_json::from_slice(&output.stdout)?;
    Ok(features)
}
```

---

## Database Strategy

### SPEC031 Zero-Config Schema Sync

**Requirement:** Verify SPEC031 is implemented in `wkmp-common`

**Verification Task (Phase 5):**
```rust
// Check if wkmp-common has:
pub trait SchemaSync {
    fn ensure_schema(&self, db: &rusqlite::Connection) -> Result<()>;
    fn add_column_if_missing(&self, table: &str, column: &str, sql_type: &str) -> Result<()>;
}
```

**If SPEC031 NOT implemented:**
- **Risk:** **HIGH** - Zero-config startup is a hard requirement (REQ-AI-078-01)
- **Mitigation:** Implement SchemaSync in wkmp-common first (add to project scope)
- **Fallback:** Manual schema migration (NOT ACCEPTABLE per CLAUDE.md zero-config mandate)

**If SPEC031 implemented:**
- Use existing SchemaSync trait
- Implement for `passages` table schema (17 new columns)
- Risk: **LOW** - Proven pattern

**Schema Sync Implementation:**
```rust
// wkmp-ai/src/database/schema.rs
use wkmp_common::database::SchemaSync;

pub struct PassagesTableSchema;

impl SchemaSync for PassagesTableSchema {
    fn ensure_schema(&self, db: &rusqlite::Connection) -> Result<()> {
        // Add 17 new columns if missing
        self.add_column_if_missing(db, "passages", "flavor_source_provenance", "TEXT")?;
        self.add_column_if_missing(db, "passages", "metadata_source_provenance", "TEXT")?;
        self.add_column_if_missing(db, "passages", "resolved_mbid", "TEXT")?;
        // ... 14 more columns per REQ-AI-080
        Ok(())
    }
}

// At startup (main.rs):
async fn main() -> Result<()> {
    let db_path = initializer.database_path();
    let db = rusqlite::Connection::open(&db_path)?;

    // Schema sync before any database access
    PassagesTableSchema.ensure_schema(&db)?;

    // Continue with normal startup
}
```

---

### Database Parameter Management

**PARAM-AI-001 through PARAM-AI-004:**

**Storage:** `settings` table (existing WKMP pattern per CLAUDE.md)

**Schema:**
```sql
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    description TEXT,
    unit TEXT,
    default_value TEXT,
    source_url TEXT
);
```

**Initialization:**
```rust
// wkmp-ai/src/database/parameters.rs
pub async fn ensure_parameters(db: &rusqlite::Connection) -> Result<()> {
    // Insert defaults if missing
    db.execute(
        "INSERT OR IGNORE INTO settings (key, value, description, unit, default_value, source_url) VALUES (?, ?, ?, ?, ?, ?)",
        params![
            "acoustid_rate_limit_ms",
            "400",
            "Rate limit for AcoustID API requests. AcoustID allows 3 requests/second (333ms). Default 400ms includes 20% safety margin.",
            "milliseconds",
            "400",
            "https://acoustid.org/webservice"
        ]
    )?;

    // Repeat for PARAM-AI-002, 003, 004
    Ok(())
}
```

---

## Error Handling Strategy

**Pattern:** Per-Song Error Isolation (REQ-AI-013, REQ-AI-NF-021)

**Error Types:**
```rust
#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("Audio decoding failed: {0}")]
    AudioDecode(#[from] symphonia::core::errors::Error),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Chromaprint error: {0}")]
    Chromaprint(String),

    #[error("Essentia error: {0}")]
    Essentia(String),

    #[error("Passage {passage_id} failed: {reason}")]
    PassageFailed { passage_id: usize, reason: String },
}
```

**Error Handling Pattern:**
```rust
// Per-passage error isolation
async fn process_passage(passage: Passage) -> Result<ImportedPassage, PassageError> {
    // Each phase wrapped in error handling
    let extraction = extract_all(&passage).await
        .map_err(|e| PassageError::ExtractionFailed(e))?;

    let fusion = fuse_all(extraction).await
        .map_err(|e| PassageError::FusionFailed(e))?;

    let validation = validate_all(&fusion).await
        .map_err(|e| PassageError::ValidationFailed(e))?;

    Ok(ImportedPassage { fusion, validation })
}

// File-level processing (continues on per-passage errors)
async fn process_file(file: AudioFile) -> FileResult {
    let passages = detect_passages(&file).await?;
    let mut results = Vec::new();

    for passage in passages {
        match process_passage(passage).await {
            Ok(imported) => {
                results.push(ImportResult::Success(imported));
            }
            Err(e) => {
                error!("Passage {passage.id} failed: {e}");
                results.push(ImportResult::Failed(e));
                // CONTINUE (don't abort entire file)
            }
        }
    }

    FileResult {
        total: passages.len(),
        successes: results.iter().filter(|r| r.is_success()).count(),
        failures: results.iter().filter(|r| r.is_failed()).count(),
        results,
    }
}
```

---

## Parallelization Strategy

**Per Specification:**
- **Sequential file processing** (one file at a time per REQ-AI-NF-011)
- **Sequential passage processing** (one passage at a time within file)
- **Parallel extraction** (Tier 1 extractors run in parallel per passage per REQ-AI-NF-012)

**tokio::join! Pattern:**
```rust
// Within a single passage, Tier 1 extractors run in parallel
async fn extract_parallel(passage: &PassageContext) -> Vec<ExtractionResult> {
    // These run concurrently (tokio::join!)
    let (id3, essentia, audio_derived, genre) = tokio::join!(
        id3_extractor.extract(passage),
        essentia_analyzer.extract(passage),
        audio_derived_extractor.extract(passage),
        id3_genre_mapper.extract(passage),
    );

    // These run sequentially (need prior results)
    let chromaprint = chromaprint_analyzer.extract(passage).await;
    let acoustid = acoustid_client.extract_with_fingerprint(&chromaprint).await;
    let musicbrainz = musicbrainz_client.extract_with_mbid(&acoustid.mbid).await;

    vec![id3, chromaprint, acoustid, musicbrainz, essentia, audio_derived, genre]
}
```

**Concurrency Limits:**
- **Files:** 1 concurrent (sequential)
- **Passages:** 1 concurrent (sequential)
- **Tier 1 extractors:** 4-7 concurrent (parallel within passage)
- **API calls:** Rate-limited (400ms for AcoustID, 1200ms for MusicBrainz)

---

## API Client Design

### AcoustID Client

**Module:** `wkmp-ai/src/extractors/acoustid_client.rs`

**Rate Limiting:**
```rust
use tokio::time::{sleep, Duration};

pub struct AcoustIDClient {
    client: reqwest::Client,
    api_key: String,
    rate_limit_ms: u64,  // From PARAM-AI-001
    last_request: tokio::sync::Mutex<std::time::Instant>,
}

impl AcoustIDClient {
    pub async fn lookup(&self, fingerprint: &str, duration: u32) -> Result<Vec<Recording>> {
        // Rate limiting
        let mut last = self.last_request.lock().await;
        let elapsed = last.elapsed();
        if elapsed < Duration::from_millis(self.rate_limit_ms) {
            sleep(Duration::from_millis(self.rate_limit_ms) - elapsed).await;
        }
        *last = std::time::Instant::now();
        drop(last);

        // HTTP request
        let response = self.client
            .post("https://api.acoustid.org/v2/lookup")
            .form(&[
                ("client", self.api_key.as_str()),
                ("fingerprint", fingerprint),
                ("duration", &duration.to_string()),
                ("meta", "recordings releasegroups compress"),
            ])
            .send()
            .await?;

        // Parse response
        let data: AcoustIDResponse = response.json().await?;
        Ok(data.results.into_iter().map(|r| r.into_recording()).collect())
    }
}
```

### MusicBrainz Client

**Module:** `wkmp-ai/src/extractors/musicbrainz_client.rs`

**Similar pattern** with:
- User-Agent header: `WKMP-AI/{version} (contact@example.com)`
- Rate limiting: 1200ms per PARAM-AI-002
- XML parsing via `quick-xml` or `serde-xml-rs`

---

## Testing Strategy

**Per Module:**
- Unit tests: >90% line coverage target (per REQ-AI-NF-032)
- Mock external dependencies (AcoustID, MusicBrainz APIs)

**Integration Tests:**
- Phase 0-6 integration (end-to-end passage processing)
- Mock HTTP servers for API tests (`wiremock` crate)

**Test Data:**
- Use fixtures from 03_acceptance_tests.md (8 audio files, 3 DB fixtures, 7 API mocks)

**CI/CD:**
```bash
# Test command
cargo test --all-features

# Coverage command
cargo tarpaulin --out Html --output-dir coverage

# Coverage target: >90% per REQ-AI-NF-032
```

---

## Decision Summary

| Decision Area | Options Considered | Selected Approach | Risk Level | Rationale |
|---------------|-------------------|-------------------|------------|-----------|
| **Architecture** | Monolithic vs Modular 3-tier | Modular 3-tier | LOW | Testability, maintainability, clear separation |
| **Chromaprint** | Pure Rust, FFI bindings crate, Custom FFI | Custom FFI | LOW-MEDIUM | We control the wrapper, safety via Rust patterns |
| **Essentia** | FFI, Command execution, Skip | Command execution | LOW-MEDIUM | Process isolation, simpler than C++ FFI |
| **Rate Limiting** | `tower`, `governor` | `tower` | LOW | Already in tokio ecosystem |
| **SPEC031** | Assume available, Implement if missing | Verify then decide | MEDIUM (if missing) | Critical dependency, must verify Phase 5 |
| **Error Handling** | Abort on error, Per-passage isolation | Per-passage isolation | LOW | Meets REQ-AI-013, REQ-AI-NF-021 |
| **Parallelization** | File-level, Passage-level, Extractor-level | Extractor-level only | LOW | Matches requirements, avoids API rate limit issues |
| **Database** | Manual migrations, Zero-config | Zero-config (SPEC031) | MEDIUM (if SPEC031 missing) | Hard requirement per CLAUDE.md |

**Overall Risk Assessment:** **LOW-MEDIUM**
- Highest risks: SPEC031 availability (verify Phase 5), Custom FFI safety (mitigate with safe wrappers)
- All risks have identified mitigations
- Risk-first decision framework applied throughout

---

## Next Phase

**Phase 5: Implementation Breakdown**
- Module-level task breakdown (17 modules total)
- Dependency ordering (build sequence)
- Interface contracts (trait definitions)
- Estimated lines of code per module

---

**Document Version:** 1.0
**Last Updated:** 2025-11-09
**Phase 4 Status:** ✅ COMPLETE
