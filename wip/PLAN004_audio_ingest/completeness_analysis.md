# Specification Completeness Analysis - PLAN004

**Plan:** PLAN004 - wkmp-ai Audio Ingest Implementation
**Date:** 2025-10-27
**Phase:** Phase 2 - Specification Completeness Verification

---

## Analysis Summary

**Overall Assessment:** ⚠️ **SUBSTANTIAL GAPS IDENTIFIED**

SPEC024 provides solid architectural foundation but lacks implementation details for 6 of 9 service modules. Critical gaps in API client implementations, error handling specifics, and database query patterns.

**Severity Breakdown:**
- 🔴 **CRITICAL:** 3 gaps (block implementation)
- 🟡 **MODERATE:** 5 gaps (increase risk/effort)
- 🟢 **MINOR:** 2 gaps (clarification needed)

---

## Critical Gaps (Implementation Blockers)

### GAP-001: MusicBrainz Client Implementation 🔴

**Issue:** SPEC024 references MusicBrainz integration (AIA-INT-010) but provides no implementation specification.

**Impact:**
- No API endpoint definitions
- No response parsing logic
- No entity relationship mapping (recording → artist/work/album)
- No rate limiting implementation details

**Referenced in:**
- SPEC024:319 "MusicBrainz integration (SPEC008:287-431)"
- SPEC008:287-431 provides workflow, NOT implementation

**What's Missing:**
1. HTTP client configuration (headers, timeout, retry policy)
2. Response schema definitions (JSON → Rust structs)
3. Error response handling (404, 503, rate limit)
4. Relationship traversal logic (recording.relations[])
5. Database entity creation sequence

**Recommendation:** Create IMPL011-musicbrainz_client.md with:
- API endpoint mapping
- Response deserialization structs
- Rate limiter implementation (1 req/s)
- Database entity creation patterns

**Workaround:** Reference SPEC008:287-431 during implementation, document decisions inline

---

### GAP-002: AcoustID Client Implementation 🔴

**Issue:** SPEC024 references Chromaprint/AcoustID (SPEC008:210-285) but no implementation details.

**Impact:**
- No Chromaprint library integration pattern
- No AcoustID API request format
- No response parsing (MBID extraction)
- No cache lookup/write logic

**What's Missing:**
1. Chromaprint library binding (FFI or pure Rust?)
2. Audio resampling pipeline (any format → 44.1kHz PCM)
3. AcoustID API request format (fingerprint, duration, metadata)
4. Response parsing (multiple MBID matches, score ranking)
5. Cache key format (fingerprint hash?)

**Recommendation:** Create IMPL012-acoustid_client.md

**Workaround:** Review existing Rust chromaprint bindings (chromaprint-rust crate), implement based on crate docs

---

### GAP-003: File Scanner Implementation 🔴

**Issue:** SPEC024:101 lists file_scanner component but no implementation details.

**Impact:**
- No file format detection logic
- No directory traversal pattern (recursive depth, symlink handling)
- No ignore patterns (.DS_Store, .git, etc.)
- No file validation (size limits, permissions)

**What's Missing:**
1. Audio format detection (magic bytes vs. extension)
2. Traversal strategy (BFS vs DFS, memory usage)
3. Symlink loop detection
4. Error handling (permission denied, broken symlinks)
5. Ignore patterns list

**Recommendation:** Create IMPL013-file_scanner.md

**Workaround:** Use walkdir crate defaults, implement basic extension-based detection (.mp3, .flac, .ogg, .m4a)

---

## Moderate Gaps (Risk/Effort Increase)

### GAP-004: Database Query Specifications 🟡

**Issue:** SPEC024:92 references db/queries.rs but no query specifications exist.

**Impact:**
- Unclear SQL query patterns
- No guidance on transaction boundaries
- No batch insert sizing (claimed "100 at a time" but no implementation)

**What's Missing:**
1. Passage insert query (with tick conversion)
2. Song/artist/work/album upsert logic (avoid duplicates)
3. Relationship table inserts (passage_songs, passage_albums)
4. Cache table queries (lookup by fingerprint/MBID)
5. Transaction scoping (per-file? per-batch?)

**Recommendation:** Create IMPL014-database_queries.md or add to IMPL001

**Workaround:** Use sqlx compile-time query verification, follow patterns from existing WKMP modules

---

### GAP-005: Metadata Extractor Details 🟡

**Issue:** SPEC024:102 lists metadata_extractor but minimal implementation guidance.

**Impact:**
- Unclear tag priority (ID3v2 vs ID3v1 vs Vorbis)
- No cover art size limits
- No handling of malformed tags

**What's Missing:**
1. Tag field mapping (TALB → album, TPE1 → artist)
2. Multi-value field handling (multiple artists)
3. Cover art extraction (which tag? APIC frame?)
4. Character encoding issues (non-UTF8 tags)
5. Duration calculation (from tags vs decode?)

**Recommendation:** Add to IMPL005 or create IMPL015-metadata_extraction.md

**Workaround:** Use lofty crate defaults, document assumptions in code comments

---

### GAP-006: Silence Detector Implementation 🟡

**Issue:** SPEC024:107 references IMPL005 but IMPL005 not fully reviewed for implementation details.

**Impact:**
- Unclear silence threshold interpretation (RMS? peak?)
- No guidance on minimum passage duration
- No boundary adjustment logic (fade detection)

**What's Missing:**
1. Silence detection algorithm (RMS-based? peak-based?)
2. Minimum silence duration (configurable?)
3. Minimum passage duration (avoid 2-second segments)
4. Boundary expansion logic (include fade-in/out)
5. User override mechanism

**Recommendation:** Review IMPL005 in detail, supplement if needed

**Workaround:** Implement basic RMS-based detection with -60dB threshold, 500ms minimum silence

---

### GAP-007: SSE Event Format Details 🟡

**Issue:** SPEC024:233-273 shows conceptual event format but missing implementation details.

**Impact:**
- No event ID generation (for reconnection)
- No event retry specification
- No keepalive interval

**What's Missing:**
1. SSE event ID format (sequence number? UUID?)
2. Reconnection handling (last-event-id header)
3. Keepalive comment interval (30 seconds?)
4. Event buffer size (if client disconnects)
5. Event expiration (drop old events after X minutes?)

**Recommendation:** Add to IMPL008 API specification

**Workaround:** Use Axum SSE defaults, simple event stream without reconnection tracking

---

### GAP-008: Error Code Enumeration 🟡

**Issue:** SPEC024:258 shows example "DECODE_ERROR" but no complete error code list.

**Impact:**
- Inconsistent error codes across components
- No UI error message mapping
- No error recovery guidance

**What's Missing:**
1. Complete error code list (DECODE_ERROR, MBID_LOOKUP_FAILED, etc.)
2. Error severity mapping (which codes are warnings vs fatal?)
3. User-facing error messages
4. Error recovery actions (retry? skip? abort?)

**Recommendation:** Add error code appendix to SPEC024 or IMPL008

**Workaround:** Define error codes inline in Rust enums, document in code

---

## Minor Gaps (Clarification Needed)

### GAP-009: Tick Conversion Edge Cases 🟢

**Issue:** SPEC024:341 shows formula but no edge case handling.

**Clarifications Needed:**
1. Rounding behavior (floor? ceil? nearest?)
2. Fractional tick handling (0.5 ticks → 0 or 1?)
3. Negative time handling (should never occur?)
4. Maximum time value (overflow at ~7600 years)

**Recommendation:** Add to IMPL001 database schema or IMPL010 parameter management

**Workaround:** Use floor rounding (`(seconds * 28_224_000.0) as i64`)

---

### GAP-010: Parallelism Tuning Guidance 🟢

**Issue:** SPEC024:218 states "4 concurrent file operations" but no tuning guidance.

**Clarifications Needed:**
1. Is 4 optimal for CPU-bound or I/O-bound?
2. Should parallelism scale with CPU cores?
3. Memory usage per worker (buffer sizes?)
4. Recommended range (1-16 is wide)

**Recommendation:** Add performance tuning section to SPEC024

**Workaround:** Use fixed 4 workers, document as "empirically determined baseline"

---

## Ambiguities (Unclear Requirements)

### AMB-001: AcousticBrainz Fallback to Essentia

**Issue:** SPEC024:149 shows "FLAVORING" state but unclear when to use Essentia vs AcousticBrainz.

**Ambiguity:**
- "AcousticBrainz or Essentia" - decision logic not specified
- Is Essentia a fallback or user choice?
- Essentia marked as AIA-FUTURE-010 (out of scope) - conflict?

**Clarification Needed:**
- MVP uses AcousticBrainz exclusively
- If AcousticBrainz has no data, warn user (no fallback)
- Essentia integration is future enhancement

**Recommendation:** Update SPEC024:149 to clarify "AcousticBrainz only (Essentia deferred)"

---

### AMB-002: Session State Persistence

**Issue:** SPEC024:171 states "Session state NOT persisted to database (transient workflow only)" but AIA-FUTURE-010 mentions "Resume After Interruption"

**Ambiguity:**
- Is resume functionality desired or explicitly deferred?
- Should we design session state for future persistence?

**Clarification Needed:**
- MVP has no resume capability (transient only)
- Future enhancement may require session state schema
- Do NOT over-engineer for future resume in MVP

**Recommendation:** Document that resume is out of scope, state is in-memory only

---

### AMB-003: Concurrent Import Sessions

**Issue:** SPEC024:59 states "Single-user import workflow (no concurrent import sessions from different users)" but doesn't address single user running multiple sessions.

**Ambiguity:**
- Can single user import multiple root folders simultaneously?
- Should API prevent concurrent sessions or allow?

**Clarification Needed:**
- Single global import session at a time (per wkmp instance)
- API should return error if import already in progress
- Session IDs exist for future multi-session support

**Recommendation:** Add explicit constraint to SPEC024 or IMPL008 API

---

## Conflicts (Contradictory Requirements)

### CONFLICT-001: Essentia Scope

**Issue:** SPEC024:84 lists essentia_runner.rs module but AIA-FUTURE-010:448 defers Essentia integration.

**Contradiction:**
- Component architecture shows essentia_runner module
- Future enhancements defer Essentia
- Scope statement excludes Essentia

**Resolution:**
- Remove essentia_runner from component architecture (SPEC024:84)
- OR mark as "placeholder for future enhancement"
- Clarify that module exists but is empty stub

**Severity:** Low (implementation clear: no Essentia in MVP)

---

## Specification Coverage Matrix

| Area | SPEC024 | IMPL008 | IMPL009 | IMPL010 | IMPL005 | Other | Status |
|------|---------|---------|---------|---------|---------|-------|--------|
| **HTTP Server** | ✅ Arch | ✅ API | - | - | - | - | ✅ Complete |
| **Import Workflow** | ✅ State machine | ✅ Endpoints | - | - | - | - | ✅ Complete |
| **File Discovery** | ⚠️ High-level | - | - | - | - | - | ⚠️ **GAP-003** |
| **Metadata Extraction** | ⚠️ High-level | - | - | - | - | - | ⚠️ **GAP-005** |
| **Fingerprinting** | ⚠️ SPEC008 ref | - | - | - | - | SPEC008 | ⚠️ **GAP-002** |
| **MusicBrainz** | ⚠️ SPEC008 ref | - | - | - | - | SPEC008 | ⚠️ **GAP-001** |
| **AcousticBrainz** | ⚠️ SPEC008 ref | - | - | - | - | SPEC008 | ⚠️ Moderate |
| **Amplitude Analysis** | ✅ SPEC025 ref | - | ✅ Complete | ✅ Params | - | SPEC025 | ✅ Complete |
| **Silence Detection** | ⚠️ IMPL005 ref | - | - | - | ⚠️ TBD | - | ⚠️ **GAP-006** |
| **Database Queries** | ⚠️ High-level | - | - | - | - | IMPL001 | ⚠️ **GAP-004** |
| **SSE Events** | ✅ Format | ⚠️ Partial | - | - | - | - | ⚠️ **GAP-007** |
| **Error Handling** | ✅ Strategy | ⚠️ Partial | - | - | - | - | ⚠️ **GAP-008** |

**Legend:**
- ✅ Complete specification exists
- ⚠️ Partial or referenced specification
- ❌ Missing specification

---

## Implementation Readiness Assessment

### Can We Implement Now?

**YES, with workarounds** - All critical gaps have documented workarounds using existing SPEC008 workflows and standard Rust crate patterns.

### Should We Fill Gaps First?

**RECOMMENDED** - Creating 4 additional IMPL specifications would reduce risk:
1. **IMPL011-musicbrainz_client.md** (CRITICAL)
2. **IMPL012-acoustid_client.md** (CRITICAL)
3. **IMPL013-file_scanner.md** (CRITICAL)
4. **IMPL014-database_queries.md** (MODERATE)

**Effort Estimate:**
- 4 specs × ~300 lines each = ~1200 lines
- Writing time: 4-6 hours (with research)
- Risk reduction: HIGH (eliminates 3 CRITICAL gaps)

**Alternative:** Proceed with implementation, document decisions in code, extract specifications post-implementation (reverse engineering).

---

## Recommendations

### Option A: Fill Critical Gaps Before Implementation (COMPLETED ✅)

**Status:** COMPLETED - All 4 specifications created

**Specifications Created:**
- ✅ IMPL011-musicbrainz_client.md (472 lines) - MusicBrainz API client, rate limiting, entity creation
- ✅ IMPL012-acoustid_client.md (497 lines) - Chromaprint fingerprinting, AcoustID lookup
- ✅ IMPL013-file_scanner.md (465 lines) - File discovery, magic byte detection, symlink handling
- ✅ IMPL014-database_queries.md (481 lines) - SQL queries, transactions, tick conversion

**Total:** 1,915 lines of implementation specifications

**Gaps Resolved:**
- 🔴 GAP-001: MusicBrainz client → RESOLVED by IMPL011
- 🔴 GAP-002: AcoustID client → RESOLVED by IMPL012
- 🔴 GAP-003: File scanner → RESOLVED by IMPL013
- 🟡 GAP-004: Database queries → RESOLVED by IMPL014

**Timeline:**
- ~~Day 1: Write IMPL011, IMPL012, IMPL013, IMPL014~~ ✅ COMPLETED
- Day 2+: Begin implementation with complete specifications

---

### Option B: Implement with Workarounds (FASTER)

**Pros:**
- Start coding immediately
- Discover real-world constraints during implementation
- Specifications reflect actual working code

**Cons:**
- Higher risk of inconsistent patterns
- May need refactoring if design decisions conflict
- Less upfront design review

**Timeline:**
- Day 1+: Begin implementation, document decisions inline
- Post-implementation: Extract specifications from working code

---

### Option C: Hybrid Approach (BALANCED)

**Pros:**
- Write IMPL011 (MusicBrainz) and IMPL012 (AcoustID) only (highest risk)
- Use workarounds for file_scanner and database_queries (lower risk)
- Balance risk reduction with faster start

**Cons:**
- Still has moderate gaps (queries, scanner)
- Partial specification coverage

**Timeline:**
- Day 1 (morning): Write IMPL011 and IMPL012
- Day 1 (afternoon)+: Begin implementation

---

## Next Steps

**If Option A chosen:**
1. Create IMPL011-musicbrainz_client.md
2. Create IMPL012-acoustid_client.md
3. Create IMPL013-file_scanner.md
4. Create IMPL014-database_queries.md
5. Proceed to Phase 3 (Acceptance Tests)

**If Option B chosen:**
1. Skip to Phase 3 (Acceptance Tests)
2. Begin implementation with inline decision documentation
3. Extract specifications post-implementation

**If Option C chosen:**
1. Create IMPL011-musicbrainz_client.md
2. Create IMPL012-acoustid_client.md
3. Proceed to Phase 3 (Acceptance Tests)

**User Decision Required:** Which option to proceed with?

---

End of completeness analysis
