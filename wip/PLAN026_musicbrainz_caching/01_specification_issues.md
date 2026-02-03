# Specification Issues: PLAN026 - MusicBrainz API Caching

**Plan:** PLAN026 - MusicBrainz API Caching for Album Matcher
**Specification:** wip/SPEC_cache_architecture.md
**Analysis Date:** 2025-01-24
**Total Requirements Analyzed:** 17

---

## Executive Summary

**Specification Quality:** ✅ **GOOD** - Ready for implementation

- **CRITICAL Issues:** 0
- **HIGH Issues:** 2
- **MEDIUM Issues:** 3
- **LOW Issues:** 4

**Decision:** ✅ **PROCEED** - No blocking issues, HIGH issues are clarifications only

---

## Issue Categories

### ❌ CRITICAL Issues (0)

**None identified.** Specification is complete for implementation.

---

### ⚠️ HIGH Issues (2)

#### HIGH-001: Cache Metadata Update Timing Unspecified

**Requirement:** REQ-CACHE-110 (MBClient Implementation)
**Line:** 253-281
**Issue Type:** Ambiguity

**Problem:**
Specification requires `update_metadata()` helper method but does not specify:
- When metadata is updated (every cache write? periodically? on exit?)
- What happens if metadata update fails
- Whether metadata must be accurate or approximate

**Impact:**
Could implement different update strategies with different performance characteristics:
- Update on every cache write: Accurate but slow (~5ms overhead per cached item)
- Update on program exit: Fast but lost if program crashes
- Periodic updates: Balanced but complex

**Recommended Clarification:**
Specify metadata update strategy explicitly:
- **Recommendation:** Update metadata on program exit only
- **Rationale:** Metadata is informational only (not critical), minimizes I/O overhead
- **Fallback:** If program crashes, metadata will be stale but cache still functional

**Resolution:**
Document in implementation plan that metadata updates on exit only. Acceptable if metadata count is approximate.

---

#### HIGH-002: ReadWrite Mode Behavior on Cache Write Failure

**Requirement:** REQ-CACHE-090 (Error Handling)
**Line:** 195-205
**Issue Type:** Ambiguity

**Problem:**
Specification states "Cache file write failure: Log error, continue execution (don't fail run)"

Ambiguous behavior:
1. If cache write fails, should subsequent requests try to cache again?
2. Should we disable caching for that specific query after first failure?
3. Should we disable all caching after N consecutive failures?

**Impact:**
Could lead to repeated write failures flooding logs, or silent degradation to Disabled mode.

**Recommended Clarification:**
Specify retry policy for cache write failures:
- **Recommendation:** After cache write failure, continue operation but don't retry that specific cache entry
- **Rationale:** Avoids log spam, query still succeeds via live API, don't penalize future queries

**Resolution:**
Implement per-entry failure tracking. If write fails once for a query, don't retry caching that specific query. Allow other queries to cache normally.

---

### ⚡ MEDIUM Issues (3)

#### MEDIUM-001: Cache Directory Location Not Configurable

**Requirement:** REQ-CACHE-050 (Cache Directory Structure)
**Line:** 122-149
**Issue Type:** Incomplete (minor)

**Problem:**
Specification requires `./cache/` directory but constraints mention "Cache directory location configurable (future enhancement)" without defining current behavior.

**Current Specification:**
- Hard-coded `./cache/` relative to current working directory
- No configuration mechanism

**Questions:**
1. What if current directory is not writable (e.g., read-only file system)?
2. Should we fall back to temp directory?
3. Should we fail startup or degrade to Disabled mode?

**Recommended Resolution:**
**For initial implementation:**
- Use hard-coded `./cache/` relative to current directory
- If directory creation fails, log error and operate in Disabled mode (no caching)
- Document limitation: Cache requires writable current directory

**Future Enhancement:**
- Add `--cache-dir` command-line argument
- Add environment variable `WKMP_CACHE_DIR`

**Impact:** LOW - Most users run from writable directory, Disabled fallback acceptable

---

#### MEDIUM-002: Cache Key Collision Handling Unspecified

**Requirement:** REQ-CACHE-020 (Search Query Caching)
**Line:** 48-70
**Issue Type:** Edge Case

**Problem:**
Cache key is SHA-256 hash truncated to first 16 hex chars. Possible (but unlikely) hash collision.

**Math:**
- 16 hex chars = 64 bits = 2^64 possible values
- Birthday paradox: 50% collision chance at ~2^32 queries (~4 billion)
- For 200-album test set with ~50 queries each: ~10,000 total queries
- Collision probability: ~0.0000000012% (negligible)

**However:** Specification does not define behavior if collision occurs:
1. Overwrite existing cache entry (wrong data returned for one query)
2. Detect collision (compare full query string), use alternate key
3. Ignore (assume collision impossible)

**Recommended Resolution:**
**For initial implementation:**
- **Assume no collision** (probability negligible for expected usage)
- Include full query string in `CachedSearch.query` field for verification
- If implementing verification: Compare cached query string to requested query, re-query API if mismatch

**Impact:** VERY LOW - Collision extremely unlikely, verification adds safety

---

#### MEDIUM-003: Cache Corruption Detection Method Unspecified

**Requirement:** REQ-NF-CACHE-030 (Reliability)
**Line:** 342-347
**Issue Type:** Incomplete

**Problem:**
Specification requires "Corrupted cache entries are detected and skipped" but does not define:
1. What constitutes "corrupted" (parse failure only? semantic validation?)
2. How to detect corruption before parsing (checksum? magic number?)
3. Whether to delete corrupted entries or leave them

**Possible Corruption Types:**
- Invalid JSON syntax (parse failure)
- Missing required fields (schema violation)
- Invalid data types (string where number expected)
- Incomplete write (truncated file)

**Recommended Resolution:**
**For initial implementation:**
- **Detection:** JSON parse failure indicates corruption
- **Action:** Log warning, skip cached entry, query live API, overwrite cache file with new result
- **No Checksum:** JSON parsing is sufficient validation for initial implementation

**Future Enhancement:**
- Add CRC32 checksum to cache file format
- Validate checksums before parsing

**Impact:** LOW - JSON parsing detects most corruption, rare occurrence

---

### ℹ️ LOW Issues (4)

#### LOW-001: Cache Statistics Report Format Not Specified

**Requirement:** REQ-CACHE-080 (Cache Statistics Report)
**Line:** 180-193
**Issue Type:** Minor Ambiguity

**Problem:**
Example output provided but not specified as required format. Could be interpreted as recommendation only.

**Recommendation:**
Treat example as required format. Explicitly state "Output format MUST match example."

**Impact:** VERY LOW - Example is clear, minor clarification

---

#### LOW-002: Timestamp Precision Not Specified

**Requirement:** REQ-CACHE-020, REQ-CACHE-030
**Line:** 48-94
**Issue Type:** Minor Ambiguity

**Problem:**
Requires "ISO 8601 format" but ISO 8601 allows multiple precisions:
- Seconds: `2025-01-24T15:32:45Z`
- Milliseconds: `2025-01-24T15:32:45.123Z`
- Microseconds: `2025-01-24T15:32:45.123456Z`

**Recommendation:**
Use second precision (simplest, sufficient for cache tracking).
Format string: `%Y-%m-%dT%H:%M:%SZ` (UTC)

**Impact:** VERY LOW - All precisions acceptable, clarification prevents inconsistency

---

#### LOW-003: Cache File Permissions Not Specified

**Requirement:** REQ-CACHE-050 (Cache Directory Structure)
**Line:** 122-149
**Issue Type:** Implementation Detail

**Problem:**
No specification of file permissions (e.g., 0644, 0600).

**Recommendation:**
Use system defaults (Rust `std::fs::write` defaults). Cache files not security-sensitive.

**Impact:** VERY LOW - Default permissions acceptable

---

#### LOW-004: Handling of Concurrent Cache Access

**Requirement:** REQ-CACHE-040 (Transparent API Wrapper)
**Line:** 96-120
**Issue Type:** Edge Case

**Problem:**
Specification does not address concurrent runs writing to same cache (e.g., two terminal windows running album_matcher_26 simultaneously).

**Scenarios:**
1. Both read same cache file: OK (read-only, no conflict)
2. Both write different cache files: OK (independent files)
3. Both write same cache file: CONFLICT (race condition, possible corruption)

**Recommended Resolution:**
**For initial implementation:**
- **Document limitation:** Do not run multiple instances simultaneously in ReadWrite mode
- **Detection:** No locking mechanism (rely on user behavior)
- **Workaround:** Run in ReadOnly mode if multiple instances needed

**Future Enhancement:**
- File locking (e.g., `fs2` crate for cross-platform file locking)
- Process detection (check for lock file)

**Impact:** VERY LOW - Single-user tool, concurrent execution rare

---

## Completeness Analysis by Requirement

| Req ID | Inputs Specified | Outputs Specified | Behavior Specified | Constraints Specified | Errors Specified | Complete? |
|--------|------------------|-------------------|--------------------|-----------------------|------------------|-----------|
| REQ-CACHE-010 | ✅ (CLI args) | ✅ (mode selection) | ✅ (3 modes) | ✅ (flags) | ✅ (invalid args) | ✅ |
| REQ-CACHE-020 | ✅ (query string) | ✅ (JSON file) | ✅ (SHA-256 hash) | ✅ (file location) | ⚠️ (collision) | ✅ |
| REQ-CACHE-030 | ✅ (MBID) | ✅ (JSON file) | ✅ (direct key) | ✅ (file location) | ✅ | ✅ |
| REQ-CACHE-040 | ✅ (cache config) | ✅ (API responses) | ✅ (transparent) | ✅ (rate limit) | ✅ | ✅ |
| REQ-CACHE-050 | ✅ (cache dir) | ✅ (file structure) | ✅ (hierarchy) | ⚠️ (dir creation) | ⚠️ (not writable) | ✅ |
| REQ-CACHE-060 | ✅ (CLI args) | ✅ (mode set) | ✅ (parsing) | ✅ (flags) | ✅ | ✅ |
| REQ-CACHE-070 | ✅ (cache ops) | ✅ (log messages) | ✅ (hit/miss) | ✅ (log levels) | ✅ | ✅ |
| REQ-CACHE-080 | ✅ (stats) | ✅ (report format) | ✅ (calculation) | ⚠️ (format exact?) | ✅ | ✅ |
| REQ-CACHE-090 | ✅ (5 scenarios) | ✅ (5 responses) | ⚠️ (retry policy) | ✅ (degradation) | ✅ (5 cases) | ✅ |
| REQ-CACHE-100 | ✅ (data needs) | ✅ (5 structs) | ✅ (definitions) | ✅ (derives) | ✅ | ✅ |
| REQ-CACHE-110 | ✅ (config) | ✅ (methods) | ⚠️ (metadata timing) | ✅ (async) | ✅ | ✅ |
| REQ-CACHE-120 | ✅ (existing code) | ✅ (integration) | ✅ (replacement) | ✅ (compatibility) | ✅ | ✅ |
| REQ-CACHE-130 | ✅ (JSON) | ✅ (pretty-print) | ✅ (format) | ✅ (human-readable) | ✅ | ✅ |
| REQ-CACHE-140 | ⚠️ (future) | N/A (future) | N/A (future) | N/A (future) | N/A (future) | N/A |
| REQ-NF-CACHE-010 | ✅ (operations) | ✅ (timing goals) | ✅ (measurement) | ✅ (performance) | ✅ | ✅ |
| REQ-NF-CACHE-020 | ✅ (compatibility) | ✅ (unchanged) | ✅ (transparent) | ✅ (format match) | ✅ | ✅ |
| REQ-NF-CACHE-030 | ✅ (failures) | ✅ (no crash) | ⚠️ (corruption detect) | ✅ (graceful) | ✅ | ✅ |

**Overall Completeness:** 17/17 requirements sufficiently specified for implementation (100%)

---

## Ambiguity Analysis

### Unquantified Requirements: 0

All performance targets quantified (cache lookup <1ms, store <5ms, 10-20× speedup).

### Vague Language: 0

No "appropriate," "reasonable," "good," "fast" without quantification.

### Undefined Terms: 0

All technical terms defined in Glossary (cache hit, miss, modes, etc.).

---

## Testability Analysis

| Req ID | Can Test? | Test Type | Pass Criteria | Fail Criteria |
|--------|-----------|-----------|---------------|---------------|
| REQ-CACHE-010 | ✅ | Unit | Mode set correctly based on args | Wrong mode or arg parse failure |
| REQ-CACHE-020 | ✅ | Integration | Query cached, file exists, correct content | Missing file or wrong content |
| REQ-CACHE-030 | ✅ | Integration | Release cached, file exists, correct content | Missing file or wrong content |
| REQ-CACHE-040 | ✅ | Integration | API calls transparently cached | Cache not used or API not called |
| REQ-CACHE-050 | ✅ | Integration | Directory structure matches spec | Wrong structure or missing files |
| REQ-CACHE-060 | ✅ | Unit | Args parsed correctly | Parse failure or wrong mode |
| REQ-CACHE-070 | ✅ | Integration | Log messages present and correct | Missing logs or wrong level |
| REQ-CACHE-080 | ✅ | Integration | Statistics output matches format | Wrong format or missing stats |
| REQ-CACHE-090 | ✅ | Integration | 5 error scenarios handled correctly | Crash or wrong behavior |
| REQ-CACHE-100 | ✅ | Unit | Structs compile and serialize | Compile error or serialize failure |
| REQ-CACHE-110 | ✅ | Integration | Methods work as specified | Wrong behavior or missing methods |
| REQ-CACHE-120 | ✅ | Integration | Integration successful, existing code unchanged | Breaking changes or incompatibility |
| REQ-CACHE-130 | ✅ | Integration | JSON is pretty-printed and human-readable | Minified or binary format |
| REQ-CACHE-140 | N/A | N/A | Future enhancement | N/A |
| REQ-NF-CACHE-010 | ✅ | Performance | Timing goals met (<1ms, <5ms, 10-20×) | Timing goals not met |
| REQ-NF-CACHE-020 | ✅ | Integration | Output matches album_matcher_25 | Output differs |
| REQ-NF-CACHE-030 | ✅ | Integration | No crashes on cache errors | Crash on cache failure |

**Testability:** 16/16 active requirements are testable (100%)

---

## Consistency Analysis

**No Conflicts Identified**

All requirements are mutually compatible. No contradictions found.

**Resource Budget:**
- Cache storage: ~1GB for 200 albums (acceptable)
- Performance overhead: <1ms read, <5ms write (acceptable)
- Code changes: ~12% (acceptable, within maintainability guidelines)

---

## Dependency Validation

**All Dependencies Verified ✅**

- Source file exists: album_matcher_25.rs
- All Rust crates present: serde, serde_json, sha2, chrono, reqwest
- Existing structs compatible: MBSearchResponse, MBReleaseDetails have Serialize/Deserialize
- No blocking dependencies

---

## Recommendations

### Immediate Actions (Before Implementation)

1. **Clarify HIGH-001:** Document metadata update strategy (recommend: update on exit only)
2. **Clarify HIGH-002:** Document cache write retry policy (recommend: no retry per entry)

### Implementation Guidance

1. **MEDIUM Issues:** Use recommended resolutions (documented above)
2. **LOW Issues:** Use defaults, document limitations
3. **Validation:** Compare output to album_matcher_25.rs to verify compatibility

### Documentation Updates

Update specification with clarifications:
- Metadata update timing (HIGH-001)
- Cache write retry policy (HIGH-002)
- Cache directory fallback behavior (MEDIUM-001)

---

## Decision

✅ **PROCEED TO PHASE 3**

**Rationale:**
- No CRITICAL issues blocking implementation
- HIGH issues are clarifications only (not blockers)
- MEDIUM and LOW issues have recommended resolutions
- Specification completeness: 100%
- Testability: 100%
- All dependencies verified

**Condition:**
Document HIGH issue resolutions in implementation plan (Phase 4+) or proceed with recommended resolutions.

---

## Sign-Off

**Phase 2 Analysis Complete:** 2025-01-24
**Analyst:** PLAN026 Workflow
**Status:** ✅ Ready for Phase 3 (Test Definition)
**Next Action:** Define acceptance tests for all 17 requirements
