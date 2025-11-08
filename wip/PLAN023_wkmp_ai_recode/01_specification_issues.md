# Specification Issues: PLAN023 - WKMP-AI Ground-Up Recode

**Plan:** PLAN023 - wkmp-ai Ground-Up Recode
**Analysis Date:** 2025-01-08
**Specification Source:** wip/SPEC_wkmp_ai_recode.md (1222 lines)
**Requirements Analyzed:** 46 total (36 functional, 10 non-functional)

---

## Executive Summary

**Specification Completeness Assessment:**
- **CRITICAL Issues:** 4 (block implementation - must resolve)
- **HIGH Issues:** 8 (high risk without resolution)
- **MEDIUM Issues:** 6 (should resolve before implementation)
- **LOW Issues:** 3 (minor, can address during implementation)

**Overall Quality:** Good foundation with detailed algorithms, but missing several key technical details

**Decision:** **⚠️ CONDITIONAL PROCEED** - Resolve CRITICAL issues before Phase 3, resolve HIGH issues before implementation

---

## CRITICAL Issues (Must Resolve Before Implementation)

### CRITICAL-001: Genre → Characteristics Mapping Undefined

**Requirement:** REQ-AI-041 (Multi-Source Flavor Extraction) - Line 156
**Affected Requirement:** REQ-AI-041 (ID3 Genre Mapping extractor)

**Issue:**
- Specification states "Map ID3 genre string to characteristics (coarse mapping)"
- **No mapping table provided**
- Genre mapping is critical for ID3-derived musical flavor (confidence 0.3)

**Impact:**
- Cannot implement `ID3GenreMapper` extractor without this mapping
- Example given in spec (line 706): `"rock" → {"danceability.danceable": 0.4, ...}` but only 2 genres shown
- Need comprehensive mapping for common genres (Rock, Pop, Electronic, Classical, Jazz, Hip-Hop, Country, Metal, etc.)

**Resolution Required:**
1. Define complete genre → characteristics mapping table (at least 20-30 common genres)
2. Specify what to do with unknown/custom genres (default to empty characteristics? use heuristics?)
3. Document mapping in specification or reference external data file

**Suggested Resolution:**
- **Option A:** Create `genre_mapping.json` data file with comprehensive mappings
- **Option B:** Reference existing genre taxonomy (AllMusic, Discogs, Last.fm)
- **Option C:** Implement basic mapping for top 20 genres, return empty for others

---

### CRITICAL-002: Expected Characteristics Count Undefined

**Requirement:** REQ-AI-045 (Completeness Scoring) - Line 180
**Affected Requirements:** REQ-AI-045, REQ-AI-084 (flavor_completeness database field)

**Issue:**
- Formula: `completeness = (present_characteristics / expected_characteristics) * 100%`
- **What is expected_characteristics value?**
- SPEC003-musical_flavor.md reference mentioned but not provided in specification

**Impact:**
- Cannot compute completeness percentage without knowing denominator
- Different musical flavor characteristics may have different expected counts (binary vs complex)

**Resolution Required:**
1. Define total number of expected characteristics
2. Specify if this varies by characteristic category (binary: 2, complex: 3+)
3. Reference SPEC003-musical_flavor.md line numbers where this is defined

**Suggested Resolution:**
- Read SPEC003-musical_flavor.md to extract expected characteristics
- Document count in specification (e.g., "Expected: 23 characteristics total")
- If varies by category, provide table:
  ```
  Binary characteristics: 8 categories × 2 values = 16 expected
  Complex characteristics: 5 categories × avg 4 values = 20 expected
  Total: 36 expected characteristics
  ```

---

### CRITICAL-003: Levenshtein Ratio Implementation Ambiguous

**Requirement:** REQ-AI-061 (Title Consistency Check) - Line 209
**Affected Requirements:** REQ-AI-034, REQ-AI-061 (fuzzy matching)

**Issue:**
- Specification calls for "Levenshtein ratio" multiple times
- **Levenshtein distance and Levenshtein ratio are different metrics**
  - Levenshtein distance: Edit distance (integer)
  - Levenshtein ratio: Similarity score (0.0-1.0) = 1 - (distance / max(len_a, len_b))
- Threshold `similarity > 0.95` suggests ratio (0.0-1.0 scale), not distance

**Impact:**
- Ambiguity in which metric to use
- Different implementations may produce different results
- Thresholds (0.95, 0.85, 0.80) only make sense for ratio

**Resolution Required:**
1. Confirm using "normalized Levenshtein similarity" (ratio, not distance)
2. Specify exact formula: `similarity = 1 - (levenshtein_distance / max(len1, len2))`
3. Confirm using `strsim` crate's `normalized_levenshtein()` function

**Suggested Resolution:**
- Update specification to say "normalized Levenshtein similarity" (not "ratio")
- Document formula explicitly
- Specify crate: `strsim::normalized_levenshtein()`

---

### CRITICAL-004: SSE Event Buffering Strategy Undefined

**Requirement:** REQ-AI-073 (Event Throttling) - Line 254

**Issue:**
- "SHALL buffer events if emission rate exceeds limit"
- **What type of buffer?** In-memory? Disk-backed?
- **What buffer size limit?** Unlimited? Fixed size?
- **What happens if buffer full?** Drop oldest? Block? Fail?

**Impact:**
- If 10-song album emits 10 × 6 = 60 events in rapid succession (>30/sec limit)
- Buffering strategy affects memory usage and event delivery guarantees
- Unbounded buffer = potential memory exhaustion
- Bounded buffer with dropping = lost events (violates "do NOT drop events")

**Resolution Required:**
1. Define buffer implementation:
   - **Recommended:** Bounded in-memory queue (e.g., 1000 events max)
   - If buffer full: Block sender until space available (backpressure)
2. Specify buffer size limit (number of events)
3. Specify backpressure mechanism (block vs drop vs fail)

**Suggested Resolution:**
- Use `tokio::sync::mpsc` channel with bounded capacity (1000)
- If buffer full: Await space (backpressure to import workflow)
- Document assumption: Import workflow produces < 1000 buffered events

---

## HIGH Issues (High Risk Without Resolution)

### HIGH-001: Chromaprint Rust Binding Not Specified

**Requirement:** REQ-AI-021 (Multi-Source MBID Resolution) - Line 104
**Affected Requirements:** REQ-AI-012 (generate passage-specific fingerprint)

**Issue:**
- Specification requires Chromaprint fingerprinting
- **Which Rust crate to use?**
  - `chromaprint-rust` (unmaintained since 2017)
  - `acoustid` crate (unknown status)
  - FFI bindings to C library (manual)
- Dependencies map flags this as "Need to research, add if needed"

**Impact:**
- Cannot implement fingerprinting without choosing library
- Different libraries may have different APIs and quality
- May need to write FFI bindings if no maintained crate exists

**Resolution Required:**
1. Research available Chromaprint Rust crates
2. Select primary option (or decide to use FFI)
3. Update dependencies_map.md with decision
4. Add chosen crate to Cargo.toml

**Suggested Resolution:**
- **Increment 0 (Pre-Implementation):** Research Chromaprint Rust bindings
- Document decision in dependencies_map.md
- If no good crate: Plan FFI wrapper implementation

---

### HIGH-002: Essentia Rust Bindings Availability Unknown

**Requirement:** REQ-AI-041 (Multi-Source Flavor Extraction) - Line 156
**Affected Requirements:** REQ-AI-042 (Essentia confidence 0.9)

**Issue:**
- Specification assumes Essentia analyzer is available (conf 0.9, second-highest priority)
- **Essentia is C++/Python library** - Rust bindings may not exist
- Dependencies map flags as "Research bindings"
- Graceful degradation if missing, but this is high-value data source

**Impact:**
- If no Rust bindings, must use FFI or skip Essentia entirely
- Skipping Essentia reduces flavor quality (next source is Audio-derived at conf 0.6)
- May significantly impact musical flavor accuracy for post-2022 recordings

**Resolution Required:**
1. Research Essentia Rust bindings (crates.io, GitHub)
2. If none exist: Decide FFI wrapper vs skip
3. If skip: Update spec to lower priority of Essentia (mark as future enhancement)

**Suggested Resolution:**
- **Increment 0 (Pre-Implementation):** Research Essentia availability
- If no bindings: **Recommend deferring Essentia to Phase 2 (future enhancement)**
- Initial implementation: AcousticBrainz (conf 1.0) + Audio-derived (conf 0.6) + ID3 (conf 0.3)
- This maintains graceful degradation without blocking implementation

---

### HIGH-003: Parallel Extraction Timeout Not Specified

**Requirement:** REQ-AI-NF-012 (Parallel Extraction) - Line 314
**Affected Requirements:** REQ-AI-041 (parallel extractor execution)

**Issue:**
- Tier 1 extractors run in parallel via `tokio::join!`
- **No timeout specified for individual extractors**
- Network APIs (AcoustID, MusicBrainz, AcousticBrainz) may hang
- Essentia computation may be slow for long passages

**Impact:**
- Single slow/hung API call blocks entire per-song processing
- Import workflow may hang indefinitely
- No way to recover from network timeouts

**Resolution Required:**
1. Define per-extractor timeout (e.g., 30 seconds for API calls, 60 seconds for Essentia)
2. Specify behavior when timeout occurs:
   - Treat as failed extraction (continue with other sources)
   - Log warning
   - Mark extractor as unavailable for this song

**Suggested Resolution:**
- API extractors (AcoustID, MusicBrainz, AcousticBrainz): 30-second timeout
- Essentia analyzer: 60-second timeout (computation may be slow)
- ID3/Chromaprint (local): No timeout needed (fast)
- On timeout: Log warning, treat as missing data source (graceful degradation)

---

### HIGH-004: API Rate Limiting Implementation Not Detailed

**Requirement:** REQ-AI-NF-012 (Parallel Extraction) - Line 314
**Affected Requirements:** Dependencies on MusicBrainz (1 req/sec), AcoustID (3 req/sec)

**Issue:**
- Specification mentions "Limit concurrent network requests to prevent API throttling"
- Dependencies map specifies MusicBrainz 1 req/sec, AcoustID 3 req/sec
- **How to implement rate limiting?**
  - Global rate limiter? Per-API rate limiter?
  - Async delay between requests?
  - Token bucket? Leaky bucket?

**Impact:**
- Without proper rate limiting, may get IP banned by MusicBrainz
- Too conservative rate limiting slows import unnecessarily
- Per-song parallel extraction may violate rate limits if not coordinated

**Resolution Required:**
1. Choose rate limiting strategy:
   - **Recommended:** Per-API rate limiter (separate for MusicBrainz, AcoustID)
   - Implementation: `tokio::sync::Semaphore` or `governor` crate
2. Specify MusicBrainz throttle: 1 request/second globally (across all songs)
3. Specify AcoustID throttle: 3 requests/second globally

**Suggested Resolution:**
- Use `governor` crate for rate limiting (mature, async-compatible)
- MusicBrainz rate limiter: 1 req/sec quota
- AcoustID rate limiter: 3 req/sec quota
- Rate limiters shared across all concurrent operations (global state)

---

### HIGH-005: Zero-Song Passage Handling Ambiguous

**Requirement:** REQ-AI-NF-022 (Graceful Degradation) - Line 326
**Affected Requirements:** REQ-AI-013 (error isolation), Open Question 6 (line 1192)

**Issue:**
- Specification says "Create zero-song passages when identification fails"
- Open Questions section (line 1192) asks: "Create zero-song passage or skip passage entirely?"
- **Contradictory guidance**

**Impact:**
- If identity resolution fails (no MBID), unclear what to do
- Creating zero-song passage: Passage record in database but not playable via auto-selection
- Skipping passage: No database record, user loses audio segment

**Resolution Required:**
1. Make definitive decision: Create or skip?
2. Update Open Questions section to remove ambiguity
3. If create: Specify database fields for zero-song passage (MBID=NULL, etc.)

**Suggested Resolution:**
- **Recommendation:** Create zero-song passage (per ENT-CNST-010 reference)
- Rationale: Audio data still exists and can be manually queued
- Zero-song passages excluded from automatic selection (per REQ002)
- Better to have passage with limited metadata than no passage at all

---

### HIGH-006: Database Migration Rollback Plan Missing

**Requirement:** REQ-AI-080 series (Database Schema Extensions) - Line 259

**Issue:**
- Specification defines 13 new columns to add to `passages` table
- New `import_provenance` table to create
- **No rollback plan if migration fails**
- Dependencies map mentions "Rollback migration, investigate issue" but no procedure

**Impact:**
- If migration fails partway through (e.g., 5 of 13 columns added), database in inconsistent state
- Rollback requires manually dropping columns (ALTER TABLE DROP COLUMN not supported in SQLite)
- May need to recreate table or restore from backup

**Resolution Required:**
1. Define migration rollback strategy:
   - **Option A:** Backup database before migration, restore on failure
   - **Option B:** Use sqlx migrations with down.sql rollback script
   - **Option C:** Accept that rollback requires manual intervention
2. Document rollback procedure in migration script

**Suggested Resolution:**
- Use sqlx migrations framework with up.sql and down.sql
- down.sql cannot DROP COLUMN in SQLite → Document manual rollback:
  1. Restore database from backup before migration
  2. Or: Accept new columns exist (nullable, no harm)
- **Recommended:** Database backup before migration (safest)

---

### HIGH-007: Musical Flavor Normalization Failure Handling

**Requirement:** REQ-AI-044 (Normalization) - Line 175

**Issue:**
- "SHALL validate normalization and fail import if violated"
- **What does "fail import" mean?**
  - Fail entire file import?
  - Fail only this song?
  - Abort entire import session?
- This contradicts REQ-AI-013 (error isolation - one failure doesn't abort entire import)

**Impact:**
- Normalization failure could occur due to:
  - Fusion algorithm bug
  - Corrupted source data
  - Rounding errors accumulating
- If "fail import" means abort entire session, violates error isolation principle
- If "fail import" means fail this song, may lose valid audio passage

**Resolution Required:**
1. Clarify "fail import" scope:
   - **Recommended:** Fail this song only (emit SongFailed event)
   - Log error with details (which characteristics, what sum was)
2. Add validation tolerance: Allow 1.0 ± 0.0001 (per spec) but add 0.001 tolerance for safety

**Suggested Resolution:**
- Normalization failure → Treat as song-level error (per REQ-AI-013 error isolation)
- Emit `SongFailed` event with error details
- Continue processing remaining songs
- Log normalization violation details for debugging

---

### HIGH-008: Concurrent Database Writes Not Addressed

**Requirement:** REQ-AI-012 (create database passage record per song) - Line 84
**Affected Requirements:** Sequential processing assumption

**Issue:**
- Specification assumes sequential song processing (one at a time)
- Future enhancement: Parallel song processing (REQ-AI-NF-042)
- **If parallel processing added later, concurrent database writes may conflict**
- SQLite has limited concurrency (write locking)

**Impact:**
- If multiple songs processed in parallel, database writes may serialize anyway
- May encounter "database is locked" errors
- Performance degradation if write lock contention high

**Resolution Required:**
1. Document assumption: Sequential processing → No concurrent writes initially
2. For future parallel enhancement: Plan batching or write queue
3. Consider using database connection pool with write queue

**Suggested Resolution:**
- **Initial implementation:** Sequential processing, no concurrency issues
- **Future parallel enhancement (out of scope):** Implement write queue or batch inserts
- Document assumption in specification: "Assumes sequential processing (REQ-AI-NF-011)"

---

## MEDIUM Issues (Should Resolve Before Implementation)

### MEDIUM-001: AcoustID API Key Configuration Not Specified

**Requirement:** Dependencies on AcoustID API - See dependencies_map.md

**Issue:**
- AcoustID API requires API key
- Dependencies map says "Obtain API key before testing"
- **How is API key configured?**
  - Environment variable? Config file? Hardcoded (bad)?
- No guidance in specification

**Impact:**
- Cannot test AcoustID integration without API key
- Configuration method affects deployment (Docker, systemd, manual)

**Resolution Required:**
1. Define API key configuration method:
   - **Recommended:** Environment variable `WKMP_ACOUSTID_API_KEY`
   - Fallback: TOML config file (`~/.config/wkmp/wkmp-ai.toml`)
2. Document in specification or deployment guide
3. Handle missing API key gracefully (skip AcoustID, degrade to ID3 MBID only)

**Suggested Resolution:**
- Environment variable: `ACOUSTID_API_KEY` (standard name)
- If missing: Log warning, skip AcoustID extractor
- Graceful degradation: Use ID3 embedded MBID only (if available)

---

### MEDIUM-002: Fuzzy Match Threshold Justification Missing

**Requirement:** REQ-AI-034, REQ-AI-061 (fuzzy matching thresholds) - Lines 147, 209

**Issue:**
- Thresholds specified: 0.95 (Pass), 0.85 (conflict flag), 0.80 (Warning)
- **No justification for these specific values**
- Are these empirically derived? Arbitrary? Industry standard?

**Impact:**
- Thresholds may be too strict (false positives) or too lenient (false negatives)
- Without justification, hard to tune if problems occur
- Different string pairs may need different thresholds (short titles vs long)

**Resolution Required:**
1. Document rationale for threshold values (empirical testing? reference?)
2. Consider length-based threshold adjustment (shorter strings need higher threshold)
3. Plan for threshold tuning based on testing

**Suggested Resolution:**
- **Accept thresholds as initial values (good defaults)**
- Document as "empirical defaults, tune based on testing"
- Plan to log threshold violations during testing to assess appropriateness

---

### MEDIUM-003: Import Session ID Usage Unclear

**Requirement:** REQ-AI-086 (Import Metadata: import_session_id) - Line 287

**Issue:**
- `import_session_id TEXT`: UUID for import session
- Purpose stated: "group passages from same import"
- **How is session defined?**
  - One session = one file import?
  - One session = one user import request (may include multiple files)?
  - One session = one wkmp-ai process lifetime?

**Impact:**
- Session scope affects queries (e.g., "show me all passages from last import")
- Different interpretations lead to different implementations

**Resolution Required:**
1. Define session scope clearly:
   - **Recommended:** One session = one file import (new UUID per file)
   - Alternative: One session = batch import (all files in single POST request)
2. Document in specification

**Suggested Resolution:**
- Session scope: One file import (new UUID per file processed)
- Rationale: Matches error isolation boundary (file-level FileImportComplete event)
- If batch import spans multiple files: Each file gets separate session ID

---

### MEDIUM-004: Event Throttling Implementation Details Missing

**Requirement:** REQ-AI-073 (Event Throttling) - Line 254

**Issue:**
- "Maximum 30 events/second"
- **How to implement throttling?**
  - Sleep delay between events?
  - Token bucket rate limiter?
  - Emit events in batches?

**Impact:**
- Different throttling strategies have different latency characteristics
- Sleep delay may introduce unnecessary latency if event rate is bursty
- Poor throttling implementation may lag real-time feedback

**Resolution Required:**
1. Choose throttling strategy:
   - **Recommended:** Token bucket with 30 tokens/second refill rate
   - If no tokens: Buffer event (per CRITICAL-004 buffer strategy)
2. Specify implementation (e.g., `governor` crate or manual)

**Suggested Resolution:**
- Use `governor` crate with 30 events/second quota
- If rate exceeded: Buffer events (bounded queue)
- Emit buffered events as soon as tokens available (smooth rate)

---

### MEDIUM-005: Confidence Score Precision Not Specified

**Requirement:** All confidence scores (0.0-1.0 throughout spec)

**Issue:**
- Confidence scores stored as `REAL` in database
- **What precision? f32 or f64?**
- Bayesian calculations may accumulate rounding errors with f32

**Impact:**
- f32: 6-7 decimal digits precision (may not be enough for 0.0001 normalization tolerance)
- f64: 15-16 decimal digits precision (overkill but safe)
- Rust defaults to f64 for floating-point literals

**Resolution Required:**
1. Specify f32 or f64 for confidence scores
2. Document in specification or coding conventions

**Suggested Resolution:**
- **Use f64 for all confidence scores** (Rust default, avoids precision issues)
- SQLite REAL is 8-byte float (f64 compatible)
- Accept slightly higher memory usage for precision safety

---

### MEDIUM-006: Error Logging Context Standards Missing

**Requirement:** REQ-AI-NF-021 (Error Isolation: log with context) - Line 321

**Issue:**
- "Log all errors with context (passage_id, phase, source)"
- **No logging format specified**
- Which logging level? (error!, warn!, info!)
- Structured logging? (JSON) or text?

**Impact:**
- Inconsistent logging makes debugging harder
- No standard format for parsing logs

**Resolution Required:**
1. Define logging standards:
   - Use `tracing` crate (already in WKMP)
   - Structured logging with fields: `passage_id`, `phase`, `source`, `error`
   - Log level: `error!` for failures, `warn!` for degradation
2. Document in coding conventions or specification

**Suggested Resolution:**
- Use `tracing::error!` with structured fields:
  ```rust
  tracing::error!(
      passage_id = %passage.id,
      phase = "identity_resolution",
      source = "AcoustID",
      error = %err,
      "Failed to resolve identity"
  );
  ```
- Standard format enables log parsing and analysis

---

## LOW Issues (Minor, Can Address During Implementation)

### LOW-001: Example Code Line Numbers in Spec

**Requirement:** Multiple code examples throughout specification (lines 459, 525, 636, etc.)

**Issue:**
- Specification includes executable pseudocode examples
- Examples are helpful but take up significant space
- Could be moved to appendix or separate document

**Impact:**
- None (examples are useful for implementation guidance)
- Minor: Makes specification longer

**Resolution Required:**
- None (accept as-is)
- Optional: Move detailed code examples to appendix

**Suggested Resolution:**
- Keep examples in specification (helpful for implementation)
- Consider adding "skip to requirements" navigation links

---

### LOW-002: Open Questions Section Still Present

**Requirement:** Open Questions for /plan (lines 1185-1200)

**Issue:**
- Specification includes "Open Questions for /plan" section
- Several questions already addressed in requirements
- Some remain open (genre mapping, expected characteristics)

**Impact:**
- Minor confusion (some questions answered elsewhere, some not)
- May cause implementer to think decisions are still pending

**Resolution Required:**
- Review Open Questions section
- Mark resolved questions with answers
- Keep unresolved questions (already flagged as CRITICAL-001, CRITICAL-002)

**Suggested Resolution:**
- Update Open Questions section:
  1. ~~Levenshtein Ratio Implementation~~ → Use `strsim` crate (see CRITICAL-003)
  2. Genre → Characteristics Mapping → **Open (CRITICAL-001)**
  3. Expected Characteristics Count → **Open (CRITICAL-002)**
  4. Event Buffering Strategy → **Open (CRITICAL-004)**
  5. Parallel Extraction Timeout → **Open (HIGH-003)**
  6. Zero-Song Passage Handling → Create zero-song passage (see HIGH-005)

---

### LOW-003: Success Criteria Testability

**Requirement:** Success Criteria section (lines 1167-1176)

**Issue:**
- Success criteria listed but not all are objectively measurable
- Example: "Musical flavor synthesis handles AcousticBrainz obsolescence"
  - How to measure "handles"? What is success threshold?

**Impact:**
- Ambiguous success criteria make acceptance testing harder
- May lead to disputes about whether implementation is "successful"

**Resolution Required:**
- Refine success criteria to be measurable:
  - ✅ "Post-2022 recordings have musical flavor (>0% completeness)" (measurable)
  - ✅ "Test coverage >90%" (measurable)
  - ⚠️ "Produces higher confidence than single-source" (how to measure?)

**Suggested Resolution:**
- Refine ambiguous criteria during test specification (Phase 3)
- Each success criterion should map to specific test(s)
- Accept current criteria as guidance, refine in acceptance tests

---

## Issues Summary by Category

| Category | Critical | High | Medium | Low | Total |
|----------|----------|------|--------|-----|-------|
| Completeness (Missing Info) | 3 | 3 | 3 | 0 | 9 |
| Ambiguity (Multiple Interpretations) | 1 | 2 | 2 | 2 | 7 |
| Consistency (Conflicts) | 0 | 1 | 0 | 0 | 1 |
| Testability (Hard to Verify) | 0 | 0 | 0 | 1 | 1 |
| Dependencies (External Unknowns) | 0 | 2 | 1 | 0 | 3 |
| **TOTAL** | **4** | **8** | **6** | **3** | **21** |

---

## Recommended Next Actions

### Immediate (Before Phase 3)

**Resolve CRITICAL Issues:**
1. **CRITICAL-001 (Genre Mapping):** Create basic genre → characteristics mapping (20-30 genres)
2. **CRITICAL-002 (Expected Characteristics):** Read SPEC003-musical_flavor.md, extract count
3. **CRITICAL-003 (Levenshtein):** Confirm using `strsim::normalized_levenshtein()`
4. **CRITICAL-004 (Event Buffering):** Define buffer strategy (bounded queue, backpressure)

### Before Implementation

**Resolve HIGH Issues:**
1. **HIGH-001 (Chromaprint):** Research Rust crates, select or plan FFI
2. **HIGH-002 (Essentia):** Research bindings, defer if unavailable
3. **HIGH-003 (Timeouts):** Define per-extractor timeouts (30s API, 60s Essentia)
4. **HIGH-004 (Rate Limiting):** Choose `governor` crate, define quotas
5. **HIGH-005 (Zero-Song):** Confirm creating zero-song passages (per ENT-CNST-010)
6. **HIGH-006 (Migration Rollback):** Plan database backup before migration
7. **HIGH-007 (Normalization Failure):** Treat as song-level error (error isolation)
8. **HIGH-008 (Concurrent Writes):** Document sequential processing assumption

**Consider MEDIUM Issues:**
- Most can be resolved during implementation with reasonable defaults
- Document decisions as they're made

**Ignore LOW Issues:**
- Accept as-is or address opportunistically

---

## Decision: Proceed to Phase 3?

**Recommendation:** **⚠️ CONDITIONAL PROCEED**

**Rationale:**
- Specification is well-structured with detailed algorithms
- CRITICAL issues are resolvable (mostly missing data, not fundamental flaws)
- HIGH issues are mostly dependency research (can be done in parallel)
- MEDIUM/LOW issues are minor and can be addressed during implementation

**Conditions for Proceeding:**
1. ✅ User acknowledges CRITICAL issues and approves resolutions
2. ✅ User approves conditional approach (resolve CRITICAL now, HIGH before Increment 1)
3. ✅ Dependencies research scheduled as Increment 0 (pre-implementation)

**If User Approves:**
- Proceed to Phase 3 (Acceptance Test Definition)
- Create Increment 0: Resolve CRITICAL issues + Dependencies research
- Tests will reference resolution decisions

---

**Analysis Complete:** 2025-01-08
**Analyst:** Claude Code /plan Workflow
**Status:** Ready for User Review
