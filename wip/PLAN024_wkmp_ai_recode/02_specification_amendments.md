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

- [ ] **Step 2:** Create new IMPL documents
  - [ ] Create docs/IMPL012-acoustid_client.md
  - [ ] Create docs/IMPL013-chromaprint_integration.md

- [ ] **Step 3:** Update existing IMPL documents
  - [ ] Update docs/IMPL010-parameter_management.md (add 4 parameters)

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

**This document is SSOT for all resolutions. Plan documents reference this file.**

---

**Document Version:** 1.0
**Last Updated:** 2025-11-09
**Status:** APPROVED - Pending execution after plan approval
