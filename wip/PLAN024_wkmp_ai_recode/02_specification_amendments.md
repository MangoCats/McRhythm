# Specification Amendments: WKMP-AI Audio Import System Recode

**Plan:** PLAN024
**Created:** 2025-11-09
**Purpose:** Document all amendments to be made to SPEC_wkmp_ai_recode.md and IMPL documents based on Phase 2 CRITICAL issue resolutions

**Status:** APPROVED - Awaiting execution after plan approval

---

## Executive Summary

This document serves as the **Single Source of Truth (SSOT)** for all specification amendments resulting from Phase 2 issue resolution. All plan documents reference this file instead of duplicating specification content (DRY principle).

**7 CRITICAL issues resolved, resulting in:**
- 6 new/amended requirements to add to SPEC_wkmp_ai_recode.md
- 2 new IMPL documents to create (IMPL012, IMPL013)
- 1 IMPL document to update (IMPL010)
- 1 specification clarification (AcousticBrainz status)
- 1 workflow sequence revision (entity-precise Phase 0-6)

**Execution Order:**
1. Update wip/SPEC_wkmp_ai_recode.md (all amendments below)
2. Create docs/IMPL012-acoustid_client.md
3. Create docs/IMPL013-chromaprint_integration.md
4. Update docs/IMPL010-parameter_management.md

---

## Amendments to wip/SPEC_wkmp_ai_recode.md

### Amendment 1: AcousticBrainz Status Clarification

**Location:** Line 33 (Executive Summary)

**Current Text:**
```
AcousticBrainz obsolescence: Service ended 2022
```

**Amended Text:**
```
AcousticBrainz status: Service stopped accepting new submissions in 2022, but API remains operational for querying 29M+ existing recordings (read-only access). Dataset frozen as of July 6, 2022.
```

**Rationale:** Research (2025-11-09) confirmed API is still functional, resolves specification contradiction.

---

### Amendment 2: Add [REQ-AI-012-01] Audio Segment Extraction Format

**Location:** After REQ-AI-012 (line 86)

**New Requirement:**
```markdown
#### [REQ-AI-012-01] Audio Segment Extraction Format

**Priority:** High
**Type:** Functional

**Requirement:**
SHALL extract audio segments in the following format:

- **Sample Format:** 32-bit floating point (f32)
- **Sample Rate:** Original sample rate (no automatic resampling)
- **Channels:** Preserve original channel count (mono or stereo)
- **Normalization:** [-1.0, 1.0] range (symphonia default output)
- **Conversion:** For analyzers requiring i16 PCM, convert on-demand via:
  ```rust
  i16_sample = (f32_sample * 32767.0).clamp(-32768.0, 32767.0) as i16
  ```

**Rationale:**
- Symphonia native output format (no initial conversion overhead)
- High precision for Essentia and AudioDerived analysis
- Trivial on-demand conversion for Chromaprint (requires i16)
- Stereo preserves spatial information when available

**References:**
- Symphonia decoder: Outputs f32 samples
- Chromaprint API: Requires i16 input (see IMPL013)
- Essentia: Accepts f32 input
```

**Traceability:** Resolves ISSUE-003 (audio segment format undefined)

---

### Amendment 3: Add [REQ-AI-041-02] Essentia Installation Detection

**Location:** After REQ-AI-041 (line 158)

**New Requirement:**
```markdown
#### [REQ-AI-041-02] Essentia Installation Detection

**Priority:** High
**Type:** Functional

**Requirement:**
SHALL detect Essentia availability via runtime command execution check:

1. **Detection Method:**
   - Execute: `essentia_streaming --version`
   - Success criteria: Exit code 0, version string parsed from stdout
   - Failure criteria: Exit code ≠ 0, command not found, or timeout (5 seconds)

2. **Graceful Degradation:**
   - If Essentia unavailable: Musical flavor dimensions remain NULL
   - Import SHALL complete successfully (non-blocking)
   - Completeness score SHALL be reduced by 30% (Essentia weight in scoring)
   - Later backfill possible if Essentia becomes available

3. **Binary Selection:**
   - Primary: `essentia_streaming` (flexible passage-level analysis)
   - Alternative: `essentia_extractor` (batch-oriented, may also work)

**Rationale:**
- Runtime detection allows deployment without Essentia (graceful degradation)
- Command execution is cross-platform compatible
- Non-blocking behavior ensures import success even without Essentia

**References:**
- Essentia project: https://essentia.upf.edu/
- Alternative data source: AudioDerived extractor (REQ-AI-041)
```

**Traceability:** Resolves ISSUE-002 (Essentia detection unspecified)

---

### Amendment 4: Add [REQ-AI-041-03] AcousticBrainz Availability Handling

**Location:** After REQ-AI-041 (line 158, after REQ-AI-041-02)

**New Requirement:**
```markdown
#### [REQ-AI-041-03] AcousticBrainz Availability Handling

**Priority:** High
**Type:** Functional

**Requirement:**
SHALL query AcousticBrainz API for musical flavor data with the following behavior:

1. **Service Status:**
   - AcousticBrainz stopped accepting new submissions in 2022
   - API remains operational for read-only queries (29M+ recordings)
   - Dataset frozen as of July 6, 2022

2. **API Endpoints:**
   - Base URL: `https://acousticbrainz.org/api/v1/`
   - Low-level features: `/api/v1/{mbid}/low-level`
   - High-level features: `/api/v1/{mbid}/high-level`
   - No rate limiting documented (read-only service)

3. **Fallback Behavior:**
   - Single timeout: 5 seconds
   - On 404 or timeout: Fallback to Essentia immediately
   - No retry needed (dataset is static, won't change)
   - Only ~29M recordings covered (not comprehensive)

4. **Transition Logic:**
   - Query AcousticBrainz first (if MBID known)
   - If MBID not in dataset (HTTP 404): Use Essentia
   - If Essentia unavailable: Use AudioDerived only

**Rationale:**
- AcousticBrainz provides high-quality pre-computed features when available
- Frozen dataset means failures are deterministic (no retry needed)
- Essentia provides equivalent features for recordings not in dataset

**References:**
- AcousticBrainz status: https://acousticbrainz.org (accessed 2025-11-09)
- See IMPL012 for client implementation details
```

**Traceability:** Resolves ISSUE-001 (AcousticBrainz availability undefined)

---

### Amendment 5: Add [REQ-AI-045-01] Expected Characteristics Count

**Location:** After REQ-AI-045 (line 181)

**New Requirement:**
```markdown
#### [REQ-AI-045-01] Expected Characteristics Count

**Priority:** High
**Type:** Functional

**Requirement:**
SHALL use `expected_characteristics = 50` as the default denominator for completeness scoring calculations.

**Formula:**
```
completeness = (present_characteristics / expected_characteristics) * 100%
```

**Deferred Refinement:**
- Default value: 50 (initial estimate)
- Actual value to be determined during early implementation testing
- Value stored in database parameters table (see PARAM-AI-004 in IMPL010)
- Update mechanism: Manual database UPDATE or configuration change

**Rationale:**
- Initial estimate based on typical AcousticBrainz + Essentia feature counts
- Deferred resolution acceptable (completeness score may be inaccurate initially)
- Can be refined during testing without breaking existing functionality
- Testability acknowledged as limited until value settles

**References:**
- SPEC003-musical_flavor.md (musical flavor taxonomy)
- PARAM-AI-004 (database parameter, see IMPL010)
```

**Traceability:** Resolves ISSUE-004 (expected characteristics count undefined)

---

### Amendment 6: Add [REQ-AI-021-01] Chromaprint Specification

**Location:** After REQ-AI-021 (line 106)

**New Requirement:**
```markdown
#### [REQ-AI-021-01] Chromaprint Specification

**Priority:** High
**Type:** Functional

**Requirement:**
SHALL generate Chromaprint fingerprints with the following specifications:

1. **Version:**
   - Chromaprint version: 1.6.0 (latest stable as of 2025-11-09)
   - AcoustID API version: v2
   - Compatibility verified: Chromaprint 1.6.0 with AcoustID API v2

2. **Algorithm:**
   - Default: `CHROMAPRINT_ALGORITHM_TEST2` (via `CHROMAPRINT_ALGORITHM_DEFAULT`)
   - Alternative: `CHROMAPRINT_ALGORITHM_TEST4` (removes leading silence, consider for passages)
   - Recommended: Use TEST2 for best compatibility

3. **Input Format:**
   - Sample format: 16-bit signed integers (i16), native byte-order
   - Channels: Mono or stereo (Chromaprint handles both)
   - Sample rate: Any (Chromaprint resamples internally to 11025 Hz)
   - See REQ-AI-012-01 for f32→i16 conversion

4. **Output Format:**
   - Format: Base64-encoded compressed fingerprint (URL-safe scheme)
   - Generation: `chromaprint_encode_fingerprint(..., base64=1, ...)`
   - Result: Alphanumeric string suitable for URL parameters or JSON

5. **Duration:**
   - Use full passage duration OR first 120 seconds (whichever is shorter)
   - Minimum: ~10 seconds (lower accuracy)
   - Optimal: 30-120 seconds (best identification accuracy)
   - Configurable via PARAM-AI-003 (see IMPL010)

6. **Workflow:**
   1. Decode audio to PCM i16 samples (convert from f32 per REQ-AI-012-01)
   2. Call `chromaprint_start(sample_rate, channels)`
   3. Call `chromaprint_feed(pcm_i16_samples, sample_count)` repeatedly
   4. Call `chromaprint_finish()`
   5. Call `chromaprint_get_fingerprint()` → returns base64 string
   6. Send to AcoustID API with duration in seconds

**References:**
- Chromaprint project: https://acoustid.org/chromaprint
- Implementation details: See IMPL013-chromaprint_integration.md
- AcoustID client: See IMPL012-acoustid_client.md
```

**Traceability:** Resolves ISSUE-007 (Chromaprint format undefined)

---

### Amendment 7: Revise Workflow Sequence (Entity-Precise)

**Location:** REQ-AI-010 through REQ-AI-012 (lines 74-94)

**Action:** Replace existing Phase 0-6 description with entity-precise workflow

**New Text:**
```markdown
### [REQ-AI-010] Per-Song Import Workflow

**Priority:** High
**Type:** Functional

**Requirement:**
SHALL process audio files using the following multi-phase workflow with entity-precise terminology per [REQ002-entity_definitions.md](../docs/REQ002-entity_definitions.md):

#### Phase 0: Audio File [ENT-MP-020] Scanning & Metadata Extraction

**Input:** Audio File [ENT-MP-020] path on disk

**Actions:**
- Extract filesystem metadata (filename, path, file size, modification time)
- Extract embedded metadata (ID3 tags, container metadata, album art)
- Estimate Song [ENT-MP-010] count from context clues (track count field, title patterns)
- Extract audio format specifications (codec, sample rate, channels, duration)

**Output:** Raw metadata for context-aware boundary detection

#### Phase 1: Passage [ENT-MP-030] Boundary Detection

**Input:** Audio File [ENT-MP-020] + metadata from Phase 0

**Actions:**
- Silence-based boundary detection (baseline algorithm per REQ-AI-051)
- Context-aware segmentation using metadata hints:
  - Track count field suggests number of Passages [ENT-MP-030]
  - Total duration ÷ track count suggests average Passage duration
  - Title patterns may indicate multi-Song [ENT-MP-010] Passages
- Generate Passage [ENT-MP-030] boundary candidates (start_time_ticks, end_time_ticks per SPEC017)

**Output:** Initial Passage [ENT-MP-030] boundaries (may need refinement in Phase 6)

**Note:** At this stage, Passages [ENT-MP-030] contain zero Songs [ENT-MP-010] (not yet identified)

#### Phases 2-6: Per-Passage [ENT-MP-030] Processing

For each Passage [ENT-MP-030] detected in Phase 1:

**Phase 2: Chromaprint Fingerprinting (REQ-AI-021)**
- Generate acoustic fingerprint for Passage audio segment
- Send fingerprint to AcoustID API
- Receive candidate Recording [ENT-MB-020] MBID(s)

**Output:** Zero or more candidate Recording [ENT-MB-020] MBIDs with confidence scores

**Phase 3: Identity Resolution (REQ-AI-020) - Tier 2 Fusion**
- Query MusicBrainz for Recording [ENT-MB-020] metadata via MBID
- Fuse AcoustID, MusicBrainz, and ID3 metadata sources (Bayesian algorithm per REQ-AI-023)
- Resolve Recording [ENT-MB-020] identity with confidence score
- Extract Work [ENT-MB-030] associations (0-many per ENT-CARD-045)
- Extract Artist [ENT-MB-040] credits with weights

**Output:** Recording [ENT-MB-020] + Work(s) [ENT-MB-030] + Artist(s) [ENT-MB-040]

**Phase 4: Song [ENT-MP-010] Creation**
- Combine Recording [ENT-MB-020] + Works [ENT-MB-030] + Artists [ENT-MB-040] → Song [ENT-MP-010]
- Normalize Artist weights (sum = 1.0 per ENT-MP-010)
- Associate Song [ENT-MP-010] with Passage [ENT-MP-030] (ENT-REL-060: many-to-many)

**Output:** Song [ENT-MP-010] entity linked to Passage [ENT-MP-030]

**Note:** Multi-Song Passages [ENT-MP-030] repeat Phases 2-4 for each detected Song

**Phase 5: Musical Flavor Synthesis (REQ-AI-040)**
- Query AcousticBrainz for Recording [ENT-MB-020] MBID (REQ-AI-041-03)
- Extract Essentia features if available (REQ-AI-041-02)
- Compute AudioDerived features from Passage audio
- Fuse flavor sources with confidence weighting (Tier 2 per REQ-AI-042)
- If multi-Song Passage: compute weighted centroid (ENT-CNST-020)

**Output:** Musical Flavor vector for Passage [ENT-MP-030]

**Phase 6: Quality Validation & Boundary Refinement (REQ-AI-060)**
- Cross-source consistency checks (title, duration, genre-flavor)
- Completeness scoring (metadata + flavor per REQ-AI-045-01)
- Conflict detection and flagging
- Boundary refinement if Recording duration ≠ Passage duration

**Output:** Quality scores, validation flags, refined Passage boundaries

#### Final Phase: Database Persistence (REQ-AI-080)

- Store Passage [ENT-MP-030] with timing metadata (SPEC017 ticks)
- Store Song [ENT-MP-010] → Passage [ENT-MP-030] associations
- Store provenance, quality scores, validation flags
- Store Musical Flavor vector (for Program Director selection)

#### Entity Distinctions (per REQ002-entity_definitions.md)

- **Audio File [ENT-MP-020]:** Physical file on disk (MP3, FLAC, etc.)
- **Passage [ENT-MP-030]:** Defined span within Audio File with timing metadata
  - Per ENT-REL-070: Passage is part of Audio File
  - Per ENT-CARD-070: Multiple Passages can exist within one Audio File
- **Recording [ENT-MB-020]:** MusicBrainz unique audio entity (has MBID)
- **Work [ENT-MB-030]:** Musical composition (0-many per Song per ENT-CARD-045)
- **Artist [ENT-MB-040]:** Performer/creator (0-many per Song with weights)
- **Song [ENT-MP-010]:** WKMP entity = Recording + Works + Artists
  - Per ENT-CARD-040: Each Song contains exactly one Recording
  - Per ENT-REL-060: Passage contains zero or more Songs (many-to-many)

#### Special Cases

**Zero-Song Passage [ENT-MP-030] Handling:**
- Per ENT-CNST-010: Passages with zero Songs are allowed
- Phases 2-4 may fail to identify Recording [ENT-MB-020] → Passage remains zero-Song
- Zero-Song Passages still have Musical Flavor (from Essentia/AudioDerived, no AcousticBrainz)
- Excluded from automatic selection, but playable via manual enqueue

**Multi-Song Passage [ENT-MP-030] Handling:**
- Single Audio File [ENT-MP-020] may yield multiple Passages [ENT-MP-030]
- Single Passage [ENT-MP-030] may contain multiple Songs [ENT-MP-010] (per ENT-CARD-060)
- Musical Flavor for multi-Song Passage: weighted centroid per ENT-CNST-020
- Boundary refinement may split initial Passage into multiple Passages
```

**Traceability:** Resolves ISSUE-003 (workflow sequence conflict), incorporates entity-precise terminology per user requirement

---

## New IMPL Documents to Create

### IMPL012-acoustid_client.md

**Location:** `docs/IMPL012-acoustid_client.md`

**Purpose:** AcoustID API client implementation specifications

**Contents:**
```markdown
# AcoustID Client Implementation

**Tier:** 3 (Implementation Specification)

## [IMPL-AI-AC-010] API Endpoint

- **Base URL:** `https://api.acoustid.org/v2/lookup`
- **Method:** POST (preferred for compressed fingerprints)
- **Alternative:** GET (for short fingerprints)
- **Response Format:** JSON (default) or XML

## [IMPL-AI-AC-020] Rate Limiting

- **Official Limit:** 3 requests per second
- **Implementation:** 400ms between requests (333ms + 20% safety margin)
- **Database Parameter:** PARAM-AI-001 (see IMPL010-parameter_management.md)
- **Source:** https://acoustid.org/webservice (accessed 2025-11-09)

**Implementation:**
```rust
use tokio::time::{sleep, Duration};

async fn rate_limited_request(rate_limit_ms: u64) {
    sleep(Duration::from_millis(rate_limit_ms)).await;
    // Make request
}
```

## [IMPL-AI-AC-030] Authentication

- **API Key:** Required (free registration at acoustid.org)
- **Parameter Name:** `client` in query string or POST body
- **Storage:** Environment variable `ACOUSTID_API_KEY` or database parameter

## [IMPL-AI-AC-040] Request Format

**POST body:**
```json
{
  "client": "API_KEY",
  "duration": 180,
  "fingerprint": "BASE64_ENCODED_FINGERPRINT",
  "meta": "recordings releasegroups compress"
}
```

## [IMPL-AI-AC-050] Error Handling

- **HTTP 503:** Rate limit exceeded, exponential backoff
- **HTTP 404:** No match found, graceful degradation
- **Timeout:** 10 seconds, fallback to metadata-only identification

## References

- AcoustID API documentation: https://acoustid.org/webservice
- Chromaprint integration: See IMPL013-chromaprint_integration.md
- Rate limit parameter: See IMPL010 PARAM-AI-001
```

---

### IMPL013-chromaprint_integration.md

**Location:** `docs/IMPL013-chromaprint_integration.md`

**Purpose:** Chromaprint library integration specifications

**Contents:**
```markdown
# Chromaprint Integration Implementation

**Tier:** 3 (Implementation Specification)

## [IMPL-AI-CP-010] Library Version

- **Chromaprint Version:** 1.6.0 (released 2025-08-28)
- **AcoustID API Compatibility:** v2
- **Source:** https://acoustid.org/chromaprint

## [IMPL-AI-CP-020] Rust Integration

**Crate Options:**
- **Recommended:** FFI to C library via `chromaprint-sys` or custom bindings
- **Pure Rust:** None available (as of 2025-11-09)

**System Dependency:**
```bash
# Ubuntu/Debian
apt-get install libchromaprint-dev

# macOS
brew install chromaprint

# Rust build.rs
pkg-config --libs --cflags libchromaprint
```

## [IMPL-AI-CP-030] Algorithm Selection

- **Default:** `CHROMAPRINT_ALGORITHM_TEST2` (via `CHROMAPRINT_ALGORITHM_DEFAULT`)
- **Alternative:** `CHROMAPRINT_ALGORITHM_TEST4` (removes leading silence)
- **Recommended:** TEST2 for maximum compatibility with AcoustID

## [IMPL-AI-CP-040] Input Requirements

- **Sample Format:** 16-bit signed integers (i16), native byte-order
- **Channels:** Mono or stereo (Chromaprint handles both)
- **Sample Rate:** Any (Chromaprint resamples internally to 11025 Hz)

**Conversion from f32 (per REQ-AI-012-01):**
```rust
fn f32_to_i16(sample: f32) -> i16 {
    (sample * 32767.0).clamp(-32768.0, 32767.0) as i16
}
```

## [IMPL-AI-CP-050] Output Format

- **Format:** Base64-encoded compressed fingerprint (URL-safe scheme)
- **Encoding:** `chromaprint_encode_fingerprint(..., base64=1, ...)`
- **Result:** Alphanumeric string suitable for JSON/URL

## [IMPL-AI-CP-060] Duration Configuration

- **Default:** 120 seconds
- **Database Parameter:** PARAM-AI-003 (see IMPL010-parameter_management.md)
- **Minimum:** 10 seconds (lower accuracy)
- **Optimal:** 30-120 seconds

## [IMPL-AI-CP-070] Workflow Implementation

```rust
use chromaprint_sys::*; // FFI bindings

fn generate_fingerprint(
    audio_f32: &[f32],
    sample_rate: u32,
    channels: u8,
    duration_seconds: u32,
) -> Result<String, Error> {
    // Convert f32 to i16
    let audio_i16: Vec<i16> = audio_f32
        .iter()
        .map(|&s| f32_to_i16(s))
        .collect();

    unsafe {
        // 1. Create context
        let ctx = chromaprint_new(CHROMAPRINT_ALGORITHM_DEFAULT);

        // 2. Start fingerprinting
        chromaprint_start(ctx, sample_rate as i32, channels as i32);

        // 3. Feed audio data
        let sample_count = (sample_rate * channels as u32 * duration_seconds) as usize;
        let truncated = &audio_i16[..sample_count.min(audio_i16.len())];
        chromaprint_feed(ctx, truncated.as_ptr(), truncated.len() as i32);

        // 4. Finalize
        chromaprint_finish(ctx);

        // 5. Get fingerprint (base64)
        let mut fingerprint_ptr: *mut i8 = std::ptr::null_mut();
        chromaprint_get_fingerprint(ctx, &mut fingerprint_ptr);

        // 6. Convert to Rust String
        let fingerprint = CStr::from_ptr(fingerprint_ptr).to_string_lossy().to_string();

        // 7. Cleanup
        chromaprint_free(ctx);
        chromaprint_dealloc(fingerprint_ptr);

        Ok(fingerprint)
    }
}
```

## References

- Chromaprint source: https://github.com/acoustid/chromaprint
- AcoustID client: See IMPL012-acoustid_client.md
- Audio format: See REQ-AI-012-01 in SPEC_wkmp_ai_recode.md
```

---

## Updates to Existing IMPL Documents

### IMPL010-parameter_management.md

**Location:** `docs/IMPL010-parameter_management.md`

**Action:** Add new database parameters

**Parameters to Add:**

```markdown
## Audio Import Parameters (wkmp-ai)

### [PARAM-AI-001] acoustid_rate_limit_ms

- **Type:** INTEGER
- **Default:** 400
- **Unit:** milliseconds between requests
- **Valid Range:** 100-10000
- **Description:** Rate limit for AcoustID API requests. AcoustID allows 3 requests/second (333ms). Default 400ms includes 20% safety margin to prevent API throttling.
- **Source:** https://acoustid.org/webservice (accessed 2025-11-09)
- **Implementation:** See IMPL012-acoustid_client.md [IMPL-AI-AC-020]
- **Requirement:** REQ-AI-NF-012 (prevent API throttling)

### [PARAM-AI-002] musicbrainz_rate_limit_ms

- **Type:** INTEGER
- **Default:** 1200
- **Unit:** milliseconds between requests
- **Valid Range:** 500-10000
- **Description:** Rate limit for MusicBrainz API requests. MusicBrainz allows 1 request/second per IP (1000ms). Default 1200ms includes 20% safety margin. Server returns HTTP 503 if exceeded.
- **Source:** https://musicbrainz.org/doc/MusicBrainz_API/Rate_Limiting (accessed 2025-11-09)
- **Implementation:** See IMPL011-musicbrainz_client.md [IMPL-AI-MB-020]
- **Requirement:** REQ-AI-NF-012 (prevent API throttling)

### [PARAM-AI-003] chromaprint_fingerprint_duration_seconds

- **Type:** INTEGER
- **Default:** 120
- **Unit:** seconds
- **Valid Range:** 10-300
- **Description:** Duration of audio to fingerprint with Chromaprint for AcoustID lookup. AcoustID recommends 30-120 seconds for optimal accuracy. Use full passage if shorter than this value. Minimum 10 seconds (reduced accuracy).
- **Source:** https://acoustid.org/chromaprint, https://github.com/acoustid/chromaprint (accessed 2025-11-09)
- **Implementation:** See IMPL013-chromaprint_integration.md [IMPL-AI-CP-060]
- **Requirement:** REQ-AI-021-01 (Chromaprint specification)

### [PARAM-AI-004] expected_musical_flavor_characteristics

- **Type:** INTEGER
- **Default:** 50
- **Unit:** count (dimensionless)
- **Valid Range:** 1-200
- **Description:** Expected count of musical flavor characteristics per SPEC003-musical_flavor.md. Determines completeness scoring denominator. Value will be refined during initial testing based on actual AcousticBrainz + Essentia + AudioDerived feature counts.
- **Formula:** `completeness = (present_characteristics / expected_characteristics) * 100%`
- **Update Mechanism:** Manual database UPDATE during early implementation testing
- **Implementation:** Completeness scoring in FlavorSynthesizer (Tier 2)
- **Requirement:** REQ-AI-045-01 (expected characteristics count)
```

---

## Amendment 8: File-Level Import Tracking and User Approval

**Issue Addressed:** User requirement for file-level import tracking, confidence scoring, user approval workflow, and intelligent skip logic

**Source:** User request 2025-11-09 (post-Phase 8)

**Location:** New requirement section (REQ-AI-009 series), files table schema, settings table

**Type:** New functionality (scope expansion)

**Specification:**

Add comprehensive file-level import tracking to enable intelligent skip logic, user approval workflows, and metadata protection:

### REQ-AI-009: File-Level Import Tracking and User Approval

The system SHALL track file-level import status, confidence scores, and user approval to enable intelligent skip logic and protect user-approved metadata from automatic changes.

#### [REQ-AI-009-01] File-Level Import Completion Tracking

The system SHALL track when file import completes and aggregate passage-level confidence into file-level metrics:

- **files.import_completed_at** (INTEGER, i64 unix epoch milliseconds)
  - Timestamp when all passages within file successfully processed (Phase 0-6 complete)
  - NULL = not yet imported or import failed

- **files.import_success_confidence** (REAL, f32 0.0-1.0)
  - Aggregate quality score across all passages within file
  - Formula: `MIN(passage_composite_scores)` (conservative, minimum passage score)
  - Passage composite score: `(identity_confidence * 0.4) + (metadata_completeness / 100.0 * 0.3) + (overall_quality_score / 100.0 * 0.3)`
  - NULL = not yet computed

**Rationale:** Renamed from user's "identity_confidence" to avoid conflict with existing per-passage identity_confidence (Bayesian MBID resolution). File-level score aggregates all quality signals.

#### [REQ-AI-009-02] File-Level Metadata Collection Tracking

The system SHALL track metadata collection completion and quality:

- **files.metadata_import_completed_at** (INTEGER, i64 unix epoch milliseconds)
  - Timestamp when metadata fusion (Phase 5) completed for all passages
  - NULL = metadata not yet collected

- **files.metadata_confidence** (REAL, f32 0.0-1.0)
  - Quality of metadata fusion across all passages
  - Formula: `(avg_metadata_completeness + avg_field_confidence) / 2.0`
  - `avg_metadata_completeness = AVG(passages.metadata_completeness) / 100.0`
  - `avg_field_confidence = AVG((title_confidence + artist_confidence) / 2.0)`
  - NULL = not yet computed

**Rationale:** Distinct from metadata_completeness (quantity of fields) - this measures quality of fusion process.

#### [REQ-AI-009-03] User Approval Timestamp

The system SHALL provide user approval tracking:

- **files.user_approved_at** (INTEGER, i64 unix epoch milliseconds)
  - Timestamp when user explicitly approved file metadata via API
  - NULL = not yet approved by user

**Behavior:**
- User approval SHALL protect ALL passages within file from future automatic metadata changes
- Re-import SHALL skip files with user_approved_at IS NOT NULL (absolute protection)
- User approval implies acceptance of all passage metadata as correct

#### [REQ-AI-009-04] Skip Logic - User Approval (Absolute Priority)

The system SHALL enforce user approval as absolute protection:

**IF** `files.user_approved_at IS NOT NULL`:
- SHALL skip entire import process (Phases 0-6)
- SHALL NOT modify any passage metadata (title, artist, album, identity, flavor, etc.)
- SHALL emit SSE event: `FileSkipped` with reason: "UserApproved"
- SHALL increment progress counter (file counted as processed)

**Rationale:** User approval is highest-priority signal - overrides all confidence thresholds and re-import triggers.

#### [REQ-AI-009-05] Skip Logic - Modification Time Check

The system SHALL detect unchanged files before processing:

**Before Phase 0:**
- SHALL query: `SELECT modification_time FROM files WHERE hash = ?`
- SHALL compare database modification_time with current file modification_time

**IF** modification times match (file unchanged since last import):
- SHALL skip import (Phases 0-6)
- SHALL emit SSE event: `FileSkipped` with reason: "FileUnchanged"
- SHALL NOT re-compute hash or re-scan file

**Rationale:** Fast early-exit for unchanged files (no I/O beyond modification time check).

#### [REQ-AI-009-06] Skip Logic - Import Success Confidence

The system SHALL skip re-import for high-confidence files:

**Before Phase 0:**
- SHALL query: `SELECT import_success_confidence FROM files WHERE hash = ?`
- SHALL retrieve PARAM-AI-005 threshold from settings (default 0.75)

**IF** `import_success_confidence >= threshold` AND `user_approved_at IS NULL`:
- SHALL skip Phases 2-4 (fingerprinting, identity resolution, metadata fusion)
- SHALL run Phase 0 (boundary detection, check for new passages)
- SHALL run Phases 5-6 (flavor synthesis, validation, may have new data)
- SHALL emit SSE event: `PartialImport` with phases_skipped: "2-4"

**Rationale:** High-confidence identification means Recording MBID already correct - skip expensive API calls, but re-compute flavor (may have new sources).

#### [REQ-AI-009-07] Skip Logic - Metadata Confidence

The system SHALL skip metadata re-collection for sufficient-quality metadata:

**Before Phase 5:**
- SHALL query: `SELECT metadata_confidence FROM files WHERE hash = ?`
- SHALL retrieve PARAM-AI-006 threshold from settings (default 0.66)

**IF** `metadata_confidence >= threshold` AND `user_approved_at IS NULL`:
- SHALL skip Phase 5 (metadata fusion re-collection)
- SHALL run Phases 2-4 (identity resolution, may update MBID)
- SHALL run Phase 6 (validation, may flag new issues)
- SHALL emit SSE event: `PartialImport` with phases_skipped: "5"

**Rationale:** Metadata quality sufficient - avoid re-querying MusicBrainz, but allow identity updates if new fingerprint data available.

#### [REQ-AI-009-08] Re-Import Attempt Limiting

The system SHALL prevent infinite re-import loops:

- **files.reimport_attempt_count** (INTEGER, default 0)
  - Increments each time file is imported (initial import + re-imports)

- **files.last_reimport_attempt_at** (INTEGER, i64 unix epoch milliseconds)
  - Timestamp of most recent import attempt

**Before Phase 0:**
- SHALL query: `SELECT reimport_attempt_count FROM files WHERE hash = ?`
- SHALL retrieve PARAM-AI-007 from settings (default max_reimport_attempts = 3)

**IF** `reimport_attempt_count >= max_reimport_attempts` AND `user_approved_at IS NULL`:
- SHALL skip automatic re-import
- SHALL set `passages.validation_status = "ManualReviewRequired"` for all passages
- SHALL emit SSE event: `FileRequiresReview` with reason: "MaxReimportAttemptsExceeded"
- SHALL log file path for user review

**Rationale:** If confidence doesn't improve after 3 attempts, automatic processing cannot resolve issue - flag for manual intervention.

#### [REQ-AI-009-09] Low-Confidence Flagging

The system SHALL flag files requiring manual review:

**After Phase 6 (Quality Validation):**
- SHALL compare `import_success_confidence` against PARAM-AI-005 threshold
- SHALL compare `metadata_confidence` against PARAM-AI-006 threshold

**IF** `import_success_confidence < PARAM-AI-005` (default 0.75):
- SHALL set `passages.validation_status = "LowImportConfidence"` for all passages
- SHALL emit SSE event: `FileRequiresReview` with reason: "LowImportConfidence"
- SHALL continue processing (non-fatal, flag for post-import review)

**IF** `metadata_confidence < PARAM-AI-006` (default 0.66):
- SHALL set `passages.validation_status = "LowMetadataConfidence"` for all passages
- SHALL emit SSE event: `FileRequiresReview` with reason: "LowMetadataConfidence"
- SHALL trigger re-import on next import run (unless max attempts exceeded)

**Rationale:** Deferred user approval model - flag during import, user reviews after batch completes (preserves automatic workflow).

#### [REQ-AI-009-10] Metadata Merging on Re-Import

The system SHALL merge new metadata with existing when re-import triggered:

**Trigger Conditions:**
- File modification time changed (content updated)
- metadata_confidence < PARAM-AI-006 (quality insufficient)
- user_approved_at IS NULL (not user-protected)
- reimport_attempt_count < PARAM-AI-007 (not max attempts)

**Merge Algorithm (per passage, per field):**

```
FOR each metadata field (title, artist, album, genre):
    IF new_field IS NOT NULL AND new_field_confidence > existing_field_confidence:
        Use new_field value
        Update field_confidence = new_field_confidence
        Update field_source = "Reimport-{session_id}"
    ELSE IF new_field IS NULL AND existing_field IS NOT NULL:
        Preserve existing_field value (no overwrite with NULL)
        Keep existing_field_confidence
        Keep existing_field_source
    ELSE IF new_field IS NOT NULL AND new_field_confidence <= existing_field_confidence:
        Preserve existing_field value (higher confidence wins)
        Keep existing_field_confidence
        Keep existing_field_source
```

**Provenance Tracking:**
- SHALL update `{field}_source` to indicate which import session provided value
- Format: `"Reimport-{import_session_id}"` vs `"Initial-{import_session_id}"`
- SHALL preserve provenance history in `import_provenance` table

**Rationale:** Confidence-based merge preserves highest-quality metadata, prevents regression from low-quality re-import.

#### [REQ-AI-009-11] Hash-Based Duplicate Detection

The system SHALL detect duplicate files before import:

**Before Phase 0:**
- SHALL compute SHA-256 hash of file contents
- SHALL query: `SELECT guid, path FROM files WHERE hash = ? AND path != ?`

**IF** duplicate found (same hash, different path):
- SHALL emit SSE event: `FileSkipped` with reason: "DuplicateContent"
- SHALL log both paths for user review (original: {existing_path}, duplicate: {new_path})
- SHALL NOT create new file record
- SHALL NOT process passages
- SHALL increment progress counter (file counted as processed)

**Rationale:** Avoid duplicate processing of same audio content at different paths (e.g., original + backup copy).

---

### Database Schema Extensions (files table)

**Add to `files` table:**

```sql
-- Import Tracking
import_completed_at INTEGER,           -- i64 unix epoch milliseconds, NULL = not imported
import_success_confidence REAL,        -- f32 0.0-1.0, aggregate passage quality
metadata_import_completed_at INTEGER,  -- i64 unix epoch milliseconds, NULL = not collected
metadata_confidence REAL,              -- f32 0.0-1.0, metadata fusion quality

-- User Approval
user_approved_at INTEGER,              -- i64 unix epoch milliseconds, NULL = not approved

-- Re-Import Control
reimport_attempt_count INTEGER DEFAULT 0,       -- Increments each import
last_reimport_attempt_at INTEGER                -- i64 unix epoch milliseconds
```

**Indexes:**
```sql
CREATE INDEX idx_files_user_approved ON files(user_approved_at) WHERE user_approved_at IS NOT NULL;
CREATE INDEX idx_files_import_confidence ON files(import_success_confidence);
CREATE INDEX idx_files_hash ON files(hash);  -- For duplicate detection
```

---

### New Database Parameters

#### [PARAM-AI-005] import_success_confidence_threshold

```sql
INSERT INTO settings (key, value, description, unit, default_value, source_url) VALUES (
    'import_success_confidence_threshold',
    '0.75',
    'Minimum file-level import success confidence to skip re-import. File import success confidence aggregates passage identity, metadata completeness, and overall quality scores. Lower values trigger more re-imports (higher thoroughness). Higher values skip more re-imports (higher efficiency).',
    'probability',
    '0.75',
    'PLAN024 Amendment 8, REQ-AI-009-06'
);
```

**Valid Range:** 0.0-1.0 (probability)
**Default:** 0.75 (75% confidence required to skip re-import)
**Tuning Guidance:**
- 0.90: Very conservative (only skip highest-confidence imports)
- 0.75: Balanced (default, skip good imports, re-process uncertain)
- 0.50: Aggressive (skip most imports, only re-process failures)

#### [PARAM-AI-006] metadata_confidence_threshold

```sql
INSERT INTO settings (key, value, description, unit, default_value, source_url) VALUES (
    'metadata_confidence_threshold',
    '0.66',
    'Minimum file-level metadata fusion confidence to skip metadata re-collection. Metadata confidence combines completeness (field presence) and field-level confidence scores. Lower values trigger more metadata re-collection (higher thoroughness). Higher values skip more re-collection (trust existing metadata).',
    'probability',
    '0.66',
    'PLAN024 Amendment 8, REQ-AI-009-07'
);
```

**Valid Range:** 0.0-1.0 (probability)
**Default:** 0.66 (66% confidence required to skip metadata re-collection)
**Tuning Guidance:**
- 0.80: Conservative (re-collect unless high confidence)
- 0.66: Balanced (default, re-collect if quality concerns)
- 0.40: Permissive (accept most metadata, rare re-collection)

#### [PARAM-AI-007] max_reimport_attempts

```sql
INSERT INTO settings (key, value, description, unit, default_value, source_url) VALUES (
    'max_reimport_attempts',
    '3',
    'Maximum automatic re-import attempts before flagging file for manual review. Prevents infinite re-import loops when confidence cannot be improved automatically. After max attempts, file flagged with validation_status = "ManualReviewRequired".',
    'count',
    '3',
    'PLAN024 Amendment 8, REQ-AI-009-08'
);
```

**Valid Range:** 1-10 (integer count)
**Default:** 3 (three automatic attempts before manual review)
**Tuning Guidance:**
- 1: Immediate manual review if first import low confidence
- 3: Balanced (default, allow 2 re-attempts before escalation)
- 5: Permissive (allow more automatic attempts, delay manual review)

---

### New API Endpoints

#### POST /import/files/{file_id}/approve

**Purpose:** User approves file metadata, protecting from future automatic changes

**Request:**
```json
{
  "approval_comment": "Metadata verified correct" // Optional
}
```

**Response:**
```json
{
  "file_id": "uuid-123",
  "user_approved_at": 1699564800000,
  "passages_protected": 3
}
```

**Behavior:**
- Sets `files.user_approved_at = current_timestamp_millis()`
- Emits SSE event: `FileMetadataApproved`
- All passages within file protected from automatic metadata changes
- Future imports skip this file (REQ-AI-009-04)

#### POST /import/files/{file_id}/reject

**Purpose:** User rejects automatic metadata, triggers re-import

**Request:**
```json
{
  "rejection_reason": "Artist name incorrect", // Optional
  "force_reimport": true
}
```

**Response:**
```json
{
  "file_id": "uuid-123",
  "reimport_scheduled": true,
  "passages_affected": 3
}
```

**Behavior:**
- Clears `files.user_approved_at = NULL`
- Sets `files.metadata_confidence = 0.0` (force re-collection)
- Resets `files.reimport_attempt_count = 0` if `force_reimport = true`
- Emits SSE event: `FileRejected`, triggers re-import on next import run

#### GET /import/files/pending-review

**Purpose:** List files flagged for manual review

**Response:**
```json
{
  "files": [
    {
      "file_id": "uuid-123",
      "path": "music/album/track.mp3",
      "import_success_confidence": 0.45,
      "metadata_confidence": 0.52,
      "validation_status": "LowImportConfidence",
      "reimport_attempt_count": 3,
      "passages": [
        {
          "passage_id": "uuid-456",
          "title": "Song Title",
          "identity_confidence": 0.45,
          "flags": ["low_confidence", "manual_review_recommended"]
        }
      ]
    }
  ],
  "total_count": 15
}
```

**Behavior:**
- Returns files where `validation_status IN ('LowImportConfidence', 'LowMetadataConfidence', 'ManualReviewRequired')`
- Ordered by `import_success_confidence ASC` (lowest confidence first)
- Paginated (default 50 files per page)

---

### Workflow Integration

**New Phase -1: Pre-Import Skip Logic (before Phase 0)**

Insert before Phase 0 (Passage Boundary Detection):

1. Compute file SHA-256 hash
2. Query database for existing file (hash match)
3. Check user approval (if approved, SKIP all phases)
4. Check modification time (if unchanged, SKIP)
5. Check duplicate content (if duplicate path, SKIP)
6. Check import confidence (if high, SKIP Phases 2-4)
7. Check metadata confidence (if high, SKIP Phase 5)
8. Check re-import attempt count (if exceeded, FLAG and SKIP)
9. Proceed to Phase 0 (if no skip conditions met)

**New Phase 7: Post-Import Finalization (after Phase 6)**

Insert after Phase 6 (Quality Validation):

1. Compute file-level `import_success_confidence` (aggregate passage scores)
2. Compute file-level `metadata_confidence` (aggregate metadata quality)
3. Update `files` table with completion timestamps and confidence scores
4. Increment `reimport_attempt_count`
5. Check confidence thresholds, flag if below (REQ-AI-009-09)
6. Emit SSE events for manual review if needed

---

### Traceability

**Resolves:** User requirement 2025-11-09 (file-level tracking, user approval, skip logic)

**Conflicts Resolved:**
- File-level vs passage-level identity: Renamed to `import_success_confidence` (aggregate)
- Metadata confidence vs completeness: Separate concepts (quality vs quantity)
- Interactive vs automatic: Deferred approval model (post-import review)

**Ambiguities Resolved:**
- "File identification" = aggregate passage identities
- User approval protects entire file (all passages)
- Merge uses higher-confidence value

**Dependencies:**
- SPEC031 SchemaSync (handles automatic column addition)
- PARAM-AI-001 through PARAM-AI-004 (existing parameters)
- REQ-AI-080-086 (existing passages table schema)

**Impact:**
- +7 columns to `files` table
- +3 database parameters (PARAM-AI-005, 006, 007)
- +3 API endpoints
- +2 workflow phases (Phase -1, Phase 7)
- +3.5 days implementation effort (TASK-000, TASK-019 modification)

---

## Execution Checklist

**After plan approval, execute amendments in this order:**

- [ ] **Step 1:** Update wip/SPEC_wkmp_ai_recode.md
  - [ ] Amendment 1: Line 33 AcousticBrainz status clarification
  - [ ] Amendment 2: Add REQ-AI-012-01 (audio format)
  - [ ] Amendment 3: Add REQ-AI-041-02 (Essentia detection)
  - [ ] Amendment 4: Add REQ-AI-041-03 (AcousticBrainz handling)
  - [ ] Amendment 5: Add REQ-AI-045-01 (expected characteristics)
  - [ ] Amendment 6: Add REQ-AI-021-01 (Chromaprint spec)
  - [ ] Amendment 7: Revise REQ-AI-010 workflow (entity-precise)
  - [ ] Amendment 8: Add REQ-AI-009-01 through REQ-AI-009-11 (file-level tracking, user approval)

- [ ] **Step 2:** Create new IMPL documents
  - [ ] Create docs/IMPL012-acoustid_client.md
  - [ ] Create docs/IMPL013-chromaprint_integration.md

- [ ] **Step 3:** Update existing IMPL documents
  - [ ] Update docs/IMPL010-parameter_management.md (add PARAM-AI-001 through PARAM-AI-007)
  - [ ] Update docs/IMPL001-database_schema.md (add 7 columns to files table, 3 indexes)

- [ ] **Step 4:** Update wkmp-dr to recognize new parameters
  - [ ] PARAM-AI-001 through PARAM-AI-004 descriptions
  - [ ] Reference IMPL012, IMPL013 for implementation context

- [ ] **Step 5:** Verification
  - [ ] All requirement IDs unique and traceable
  - [ ] All ENT-### references valid per REQ002-entity_definitions.md
  - [ ] All IMPL documents cross-referenced correctly
  - [ ] DRY principle maintained (no duplicate specifications)

---

## DRY References for Plan Documents

**For all PLAN024 documents, reference this amendment file instead of duplicating:**

- **AcousticBrainz status:** See Amendment 1, REQ-AI-041-03
- **Audio segment format:** See Amendment 2, REQ-AI-012-01
- **Essentia detection:** See Amendment 3, REQ-AI-041-02
- **Expected characteristics:** See Amendment 5, REQ-AI-045-01
- **Chromaprint specs:** See Amendment 6, REQ-AI-021-01, IMPL013
- **API rate limits:** See PARAM-AI-001, PARAM-AI-002 in IMPL010 updates
- **Workflow sequence:** See Amendment 7 (entity-precise Phase 0-6)
- **File-level tracking:** See Amendment 8, REQ-AI-009-01 through REQ-AI-009-11
- **User approval workflow:** See Amendment 8, API endpoints, skip logic
- **Import confidence thresholds:** See PARAM-AI-005, PARAM-AI-006, PARAM-AI-007

**This document is SSOT for all resolutions. Plan documents reference this file.**

---

**Document Version:** 2.0
**Last Updated:** 2025-11-09
**Status:** APPROVED - Amendment 8 added (file-level tracking, user approval)
