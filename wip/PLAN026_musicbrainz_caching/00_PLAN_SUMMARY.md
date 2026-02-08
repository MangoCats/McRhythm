# PLAN026: MusicBrainz API Caching - PLAN SUMMARY

**Status:** Ready for Implementation (Phases 1-3 Complete)
**Created:** 2025-01-24
**Specification Source:** wip/SPEC_cache_architecture.md
**Plan Location:** `wip/PLAN026_musicbrainz_caching/`

---

## READ THIS FIRST

This plan implements transparent caching for MusicBrainz API calls, enabling rapid algorithm tuning without rate limiting delays.

**For Implementation:**
1. Read this summary (400 lines)
2. Review test specifications: `02_test_specifications/test_index.md`
3. Follow traceability matrix: `02_test_specifications/traceability_matrix.md`
4. Implement test-first: Run tests as you implement each component

**Context Window Budget:** ~600-800 lines total per implementation session

---

## Executive Summary

### Problem Being Solved

**Current Pain Point:**
- Testing 200 albums takes 30-60 minutes due to MusicBrainz API rate limiting (1 request/second)
- Algorithm tuning requires many test runs (testing thresholds, filters, ranking changes)
- Each test run queries same data repeatedly
- Slow iteration cycles inhibit experimentation

**Quantified Impact:**
- Each full test run: 30-60 minutes
- Testing 10 parameter variations: 5-10 hours
- Impossible to rapidly iterate on algorithm improvements

### Solution Approach

**Transparent Caching Layer:**
- Cache MusicBrainz API responses to disk (JSON format)
- Three modes: Disabled (live API), ReadWrite (build cache), ReadOnly (use cache only)
- MBClient wrapper class replaces direct HTTP client usage
- Maintains exact compatibility with album_matcher_25.rs

**Expected Impact:**
- First run (build cache): 30-60 minutes (unchanged)
- Cached runs: <30 seconds (10-20× speedup)
- Test 10 parameter variations: <5 minutes (vs. 5-10 hours)

### Implementation Status

**Phases 1-3 Complete:**
- ✅ Phase 1: Scope Definition - 17 requirements extracted, scope clear
- ✅ Phase 2: Specification Verification - 0 Critical, 2 High (clarifications), 100% complete
- ✅ Phase 3: Test Definition - 24 tests defined, 100% coverage

**Phases 4-8 Status:** Not yet implemented (Week 2-3 enhancements)

---

## Requirements Summary

**Total Requirements:** 17 (14 functional, 3 non-functional)

### High Priority Requirements (12)

| Req ID | Description |
|--------|-------------|
| REQ-CACHE-010 | Three cache modes: Disabled, ReadWrite, ReadOnly |
| REQ-CACHE-020 | Cache search queries with SHA-256 hash keys |
| REQ-CACHE-030 | Cache release details with MBID keys |
| REQ-CACHE-040 | MBClient wrapper (transparent caching) |
| REQ-CACHE-060 | Command-line interface (--no-cache, --use-cache) |
| REQ-CACHE-090 | Graceful error handling (5 scenarios) |
| REQ-CACHE-100 | Data structures (5 structs/enums) |
| REQ-CACHE-110 | MBClient implementation (constructor, methods, helpers) |
| REQ-CACHE-120 | Integration with album_matcher_25.rs |
| REQ-NF-CACHE-010 | Performance (<1ms lookup, <5ms store, 10-20× speedup) |
| REQ-NF-CACHE-020 | Compatibility (same output as album_matcher_25) |
| REQ-NF-CACHE-030 | Reliability (no crashes on cache failures) |

**Full Requirements:** See `requirements_index.md`

---

## Scope

### ✅ In Scope

**Cache Infrastructure:**
- Directory structure: `cache/musicbrainz/searches/`, `releases/`, `metadata.json`
- JSON storage with pretty-printing
- SHA-256 hashing for search query keys
- MBID-based keys for release details

**Three Cache Modes:**
- Disabled (`--no-cache`): Live API only, ignore cache
- ReadWrite (default): Cache hits use cache, misses query API and store
- ReadOnly (`--use-cache`): Cache only, error on miss (for tuning)

**MBClient Wrapper:**
- `search_releases()` method (transparent caching)
- `get_release_details()` method (transparent caching)
- Drop-in replacement for reqwest::Client
- Maintains rate limiting and query stats

**Integration:**
- Copy album_matcher_25.rs → album_matcher_26.rs
- Replace HTTP client with MBClient (~30 lines modified)
- Add new structures and helpers (~550 lines added)
- Command-line argument parsing
- Cache statistics reporting

**Error Handling:**
- Cache directory creation failure → fall back to live API
- Cache file read failure → query API, attempt to cache
- Cache file write failure → log error, continue
- Cache parse failure → invalidate, query API
- ReadOnly cache miss → return error (don't query)

### ❌ Out of Scope

**Not Included:**
- Cache invalidation policies (manual deletion only)
- Cache compression
- Network cache (shared across machines)
- Configurable cache directory (hard-coded `./cache/`)
- In-memory LRU cache layer
- Cache pre-warming or parallel population

**Full Scope:** See `scope_statement.md`

---

## Specification Issues

**Phase 2 Analysis Results:**
- **CRITICAL Issues:** 0 ✅
- **HIGH Issues:** 2 (clarifications)
  - HIGH-001: Metadata update timing (resolved: update on exit only)
  - HIGH-002: Cache write retry policy (resolved: no retry per entry)
- **MEDIUM Issues:** 3 (recommendations provided)
- **LOW Issues:** 4 (minor, documented)

**Decision:** ✅ **PROCEED** - No blockers

**Full Analysis:** See `01_specification_issues.md`

---

## Test Coverage Summary

**Total Tests:** 24 (8 from spec + 16 requirement coverage)
- **Unit Tests:** 4 (data structures, basic functionality)
- **Integration Tests:** 12 (caching operations, error handling)
- **System Tests:** 5 (end-to-end workflows, all modes)
- **Performance Tests:** 3 (non-functional requirements)

**Coverage:** 100% (16/16 active requirements have tests)

### Key Tests

**TC-I-CACHE-001:** First run builds cache correctly
**TC-I-CACHE-002:** Second run uses cached data
**TC-I-CACHE-003:** ReadOnly mode with complete cache succeeds
**TC-I-CACHE-004:** ReadOnly mode with incomplete cache fails gracefully
**TC-I-CACHE-005:** Disabled mode never uses cache
**TC-I-CACHE-008:** Output identical between cached and live runs

**Traceability:** Complete matrix in `02_test_specifications/traceability_matrix.md`
**Full Tests:** See `02_test_specifications/test_index.md`

---

## Implementation Approach

### Files to Create/Modify

**NEW: album_matcher_26.rs** (copy from album_matcher_25.rs)
- Add 5 new data structures (~100 lines)
- Add MBClient implementation (~300-400 lines)
- Add cache helper functions (~200 lines)
- Add command-line parsing (~50 lines)
- Add statistics reporting (~50 lines)
- Modify 2 integration points (~30 lines)
- **Total:** ~5150 lines (from ~4500)

### Key Components

**Data Structures (REQ-CACHE-100):**
```rust
enum CacheMode { Disabled, ReadWrite, ReadOnly }
struct CacheConfig { mode: CacheMode, cache_dir: PathBuf }
struct CachedSearch { query: String, timestamp: String, response: MBSearchResponse }
struct CachedRelease { mbid: String, timestamp: String, details: MBReleaseDetails }
struct CacheMetadata { version: String, created: String, search_count: usize, ... }
```

**MBClient (REQ-CACHE-040, REQ-CACHE-110):**
```rust
impl MBClient {
    fn new(cache_config: CacheConfig) -> Self;
    async fn search_releases(&self, query: &str, stats: Option<&QueryStats>) -> Result<...>;
    async fn get_release_details(&self, mbid: &str, stats: Option<&QueryStats>) -> Result<...>;
    // + 6 helper methods for cache operations
}
```

**Integration Points (REQ-CACHE-120):**
- `search_all_mb_strategies()`: Replace `reqwest::Client` with `MBClient`
- `fetch_release_track_details()`: Use `MBClient` methods
- `main()`: Parse args, create `CacheConfig`, print statistics

### Implementation Sequence

**Step 1: Data Structures** (30 minutes)
- Define 5 structs/enums
- Add Serialize/Deserialize derives
- **Test:** TC-U-CACHE-020

**Step 2: Cache Helper Functions** (90 minutes)
- Implement hash_query()
- Implement load/store for searches and releases
- Implement metadata operations
- **Test:** (unit tests for helpers)

**Step 3: MBClient Implementation** (2 hours)
- Constructor (directory creation, metadata init)
- search_releases() method
- get_release_details() method
- Error handling for all scenarios
- **Test:** TC-U-CACHE-040, TC-I-CACHE-041, TC-I-CACHE-042

**Step 4: Integration** (90 minutes)
- Command-line argument parsing
- Replace HTTP client with MBClient in 2 locations
- Add statistics reporting
- **Test:** TC-I-CACHE-008, TC-I-CACHE-120

**Step 5: End-to-End Testing** (2-3 hours)
- First run (build cache): TC-I-CACHE-001
- Second run (use cache): TC-I-CACHE-002
- All three modes: TC-I-CACHE-003, TC-I-CACHE-004, TC-I-CACHE-005
- Error scenarios: TC-I-CACHE-007, TC-I-CACHE-090
- Performance: TC-P-CACHE-010, TC-P-CACHE-011, TC-P-CACHE-012

**Total Estimated Effort:** 6-10 hours

---

## Success Metrics

**Quantitative:**
- ✅ ReadOnly mode completes in <30 seconds (200 albums)
- ✅ Cache lookup <1ms, store <5ms
- ✅ 10-20× speedup achieved vs. live API
- ✅ 100% cache hit rate after first run
- ✅ Output identical to album_matcher_25.rs

**Qualitative:**
- ✅ Algorithm tuning workflow: Build cache once, test 10+ parameter variations in <5 minutes
- ✅ Command-line interface intuitive
- ✅ Cache files human-readable (can inspect/debug)
- ✅ Minimal changes to existing code (<12% LOC change)

---

## Dependencies

**Existing Documents (Read-Only):**
- album_matcher_25.rs (~4500 lines) - Source for copy
- album_matcher_25_designs.md - Will be updated with caching workflow

**Rust Dependencies (All Present):**
- serde, serde_json (JSON serialization) ✅
- sha2 (SHA-256 hashing) ✅
- chrono (ISO 8601 timestamps) ✅
- reqwest (HTTP client, wrapped by MBClient) ✅

**No External Dependencies** ✅

**Full Dependency Map:** See `dependencies_map.md`

---

## Constraints

**Technical:**
- Must maintain compatibility with album_matcher_25.rs output format
- Must work on Windows (path handling confirmed)
- Cache directory hard-coded to `./cache/` (configurable in future)
- Cannot modify existing MBSearchResponse or MBReleaseDetails structs

**Process:**
- Test with existing 200-album test set
- Verify identical results between cached and live runs (TC-I-CACHE-008)
- Measure and document performance improvement

**Timeline:**
- Estimated 6-10 hours total effort
- Can be completed in single session
- No external dependencies or approvals needed

---

## Risk Assessment

**Residual Risk:** LOW ✓

**Risk Mitigation:**
1. **Copy-based approach:** Start from working album_matcher_25.rs (no breaking changes)
2. **Additive pattern:** Cache layer is addition, not modification of core logic
3. **Transparent wrapper:** Existing code paths structurally unchanged
4. **Graceful degradation:** Cache failures fall back to working live API
5. **Test validation:** Compare output to album_matcher_25 for correctness

**Top Risks:**
1. Cache corruption → **Mitigation:** Detect and invalidate, re-query API
2. File I/O errors → **Mitigation:** Fall back to live API, log warnings
3. Windows path issues → **Mitigation:** Use PathBuf, test on Windows

---

## Usage Workflow

### Phase 1: Build Cache (First Run)

```bash
# Default ReadWrite mode - builds cache on misses
cargo run --release --example album_matcher_26 -p wkmp-ai 2>&1 | tee output_run1.txt
# Time: 30-60 minutes (rate limiting delays)
# Result: Cache populated in ./cache/musicbrainz/
```

### Phase 2: Algorithm Tuning (Fast Iteration)

```bash
# ReadOnly mode - use cached data only
cargo run --release --example album_matcher_26 -p wkmp-ai -- --use-cache 2>&1 | tee output_run2.txt
# Time: <30 seconds (no rate limiting, no network)

# Modify algorithm in album_matcher_26.rs (e.g., change MIN_COMBINED_RATIO)

cargo run --release --example album_matcher_26 -p wkmp-ai -- --use-cache 2>&1 | tee output_run3.txt
# Time: <30 seconds

# Test 10 different parameter values in <5 minutes total
```

### Phase 3: Final Validation (Live API)

```bash
# Disabled mode - force live API (verify tuned algorithm)
cargo run --release --example album_matcher_26 -p wkmp-ai -- --no-cache 2>&1 | tee output_final.txt
# Time: 30-60 minutes (back to rate limiting)
# Purpose: Verify tuned algorithm works with real API
```

---

## Technical Debt and Known Issues

**Status:** Not applicable - plan not yet implemented

**Note:** After implementation completes, Phase 9 (Post-Implementation Review) will systematically discover and document:
- Known bugs and limitations
- Test coverage gaps
- Performance concerns
- Security issues
- Deferred requirements

See Phase 9 section of /plan workflow for 7-step technical debt discovery process.

---

## Next Steps

### Immediate (Ready Now)

1. **Approve plan:** Review this summary, confirm approach acceptable
2. **Begin implementation:** Follow test-first approach (Step 1: Data Structures)
3. **Run tests incrementally:** Don't wait until end to test

### Implementation Sequence

1. **Step 1:** Data structures (~30 min) → Test TC-U-CACHE-020
2. **Step 2:** Cache helpers (~90 min) → Unit test helpers
3. **Step 3:** MBClient (~2 hrs) → Test TC-U-CACHE-040, TC-I-CACHE-041, TC-I-CACHE-042
4. **Step 4:** Integration (~90 min) → Test TC-I-CACHE-008, TC-I-CACHE-120
5. **Step 5:** End-to-end (~2-3 hrs) → Test all 24 tests, verify 100% pass

### After Implementation

1. ✅ Execute Phase 9: Post-Implementation Review (MANDATORY)
2. ✅ Generate technical debt report
3. ✅ Run all 24 tests, verify 100% pass
4. ✅ Verify traceability matrix 100% complete
5. ✅ Update album_matcher_25_designs.md with caching workflow
6. ✅ Archive plan using `/archive-plan PLAN026`

---

## Document Navigation

**Start Here:** This file (00_PLAN_SUMMARY.md)

**Detailed Planning:**
- `requirements_index.md` - All 17 requirements with priorities
- `scope_statement.md` - In/out scope, assumptions, constraints
- `01_specification_issues.md` - Phase 2 analysis (2 HIGH issues resolved)
- `dependencies_map.md` - All dependencies verified ✅

**Test Specifications:**
- `02_test_specifications/test_index.md` - All 24 tests (detailed)
- `02_test_specifications/traceability_matrix.md` - Requirements ↔ Tests mapping (100% coverage)

**For Implementation:**
- Read this summary (~400 lines)
- Read test_index.md (~300 lines)
- Read traceability_matrix.md (~100 lines)
- **Total context:** ~800 lines (optimal for implementation)

---

## Plan Status

**Phase 1-3 Status:** ✅ Complete
**Phases 4-8 Status:** Not yet implemented (Week 2-3 enhancements)
**Current Status:** Ready for Implementation
**Estimated Timeline:** 6-10 hours over 1-2 days

---

## Approval and Sign-Off

**Plan Created:** 2025-01-24
**Plan Status:** Ready for Implementation Review

**Next Action:** Review and approve plan, then begin Step 1 (Data Structures)

**Questions Before Starting:**
1. Confirm caching approach acceptable (transparent layer, three modes)
2. Confirm test coverage adequate (24 tests, 100% requirement coverage)
3. Confirm estimated effort reasonable (6-10 hours)
4. Any concerns about HIGH issues (metadata timing, cache write retry)?

**Once Approved:** Begin implementation following test-first approach in Step 1.
