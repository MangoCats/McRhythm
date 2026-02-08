# Test Index: PLAN026 - MusicBrainz API Caching

**Plan:** PLAN026 - MusicBrainz API Caching for Album Matcher
**Total Tests:** 24 (8 from specification test cases + 16 requirement coverage tests)
**Test Types:** 12 Integration, 8 Unit, 4 System

---

## Quick Reference Table

| Test ID | Type | Requirement | Brief Description | Priority |
|---------|------|-------------|-------------------|----------|
| TC-I-CACHE-001 | Integration | REQ-CACHE-020, REQ-CACHE-030 | First run builds cache correctly | High |
| TC-I-CACHE-002 | Integration | REQ-CACHE-040 | Second run uses cached data | High |
| TC-I-CACHE-003 | System | REQ-CACHE-010 | ReadOnly mode with complete cache succeeds | High |
| TC-I-CACHE-004 | System | REQ-CACHE-010, REQ-CACHE-090 | ReadOnly mode with incomplete cache fails gracefully | High |
| TC-I-CACHE-005 | System | REQ-CACHE-010 | Disabled mode never reads/writes cache | High |
| TC-I-CACHE-006 | Integration | REQ-CACHE-080 | Cache statistics report accurate | Medium |
| TC-I-CACHE-007 | Integration | REQ-CACHE-090, REQ-NF-CACHE-030 | Corrupted cache file detected and skipped | High |
| TC-I-CACHE-008 | System | REQ-CACHE-120, REQ-NF-CACHE-020 | Output identical between cached and live runs | High |
| TC-U-CACHE-010 | Unit | REQ-CACHE-010 | CacheMode enum correct | High |
| TC-U-CACHE-020 | Unit | REQ-CACHE-100 | Data structures compile and serialize | High |
| TC-I-CACHE-021 | Integration | REQ-CACHE-020 | Search query caching with SHA-256 hash | High |
| TC-I-CACHE-030 | Integration | REQ-CACHE-030 | Release details caching with MBID key | High |
| TC-U-CACHE-040 | Unit | REQ-CACHE-040 | MBClient constructor initializes correctly | High |
| TC-I-CACHE-041 | Integration | REQ-CACHE-040 | MBClient.search_releases() caches transparently | High |
| TC-I-CACHE-042 | Integration | REQ-CACHE-040 | MBClient.get_release_details() caches transparently | High |
| TC-I-CACHE-050 | Integration | REQ-CACHE-050 | Cache directory structure matches specification | Medium |
| TC-U-CACHE-060 | Unit | REQ-CACHE-060 | Command-line argument parsing | High |
| TC-I-CACHE-070 | Integration | REQ-CACHE-070 | Cache hit/miss logging correct | Medium |
| TC-I-CACHE-090 | Integration | REQ-CACHE-090 | All 5 error scenarios handled | High |
| TC-I-CACHE-130 | Integration | REQ-CACHE-130 | JSON format human-readable | Medium |
| TC-P-CACHE-010 | Performance | REQ-NF-CACHE-010 | Cache lookup <1ms | High |
| TC-P-CACHE-011 | Performance | REQ-NF-CACHE-010 | Cache store <5ms | High |
| TC-P-CACHE-012 | Performance | REQ-NF-CACHE-010 | ReadOnly mode 10-20× faster | High |
| TC-I-CACHE-120 | Integration | REQ-CACHE-120 | Integration with existing code successful | High |

---

## Test Specifications (Embedded)

### TC-I-CACHE-001: First Run Builds Cache

**Test Type:** Integration Test
**Requirements:** REQ-CACHE-020, REQ-CACHE-030, REQ-CACHE-050
**Priority:** High

**Given:**
- Empty cache directory (or cache/ does not exist)
- album_matcher_26 compiled and ready
- Test album set: 5 albums

**When:**
- Run: `cargo run --release --example album_matcher_26 -p wkmp-ai` (default ReadWrite mode)

**Then:**
- Cache directory `./cache/musicbrainz/` created
- Search cache files exist: `./cache/musicbrainz/searches/*.json`
- Release cache files exist: `./cache/musicbrainz/releases/*.json`
- metadata.json exists with correct counts

**Verify:**
- Count search cache files (should match number of unique queries)
- Count release cache files (should match number of unique releases)
- Parse metadata.json, verify search_count and release_count match file counts
- Open random cache file, verify JSON is valid and has required fields

**Pass Criteria:**
- All cache files created ✓
- All JSON files valid ✓
- metadata.json counts accurate ✓

**Fail Criteria:**
- Missing cache files
- Invalid JSON
- Incorrect metadata counts

---

### TC-I-CACHE-002: Second Run Uses Cached Data

**Test Type:** Integration Test
**Requirements:** REQ-CACHE-040, REQ-CACHE-070
**Priority:** High

**Given:**
- Cache populated from TC-I-CACHE-001
- Same test album set

**When:**
- Run: `cargo run --release --example album_matcher_26 -p wkmp-ai` (default ReadWrite mode)

**Then:**
- No new cache files created (all cache hits)
- Log messages show "Cache hit: search query" and "Cache hit: release ..."
- Run completes much faster than first run (no rate limiting delays)

**Verify:**
- Count cache files before and after (should be identical)
- Grep logs for "Cache hit" messages (should match expected count)
- Grep logs for "Cache miss" messages (should be zero)
- Measure run time (should be <30 seconds vs. minutes on first run)

**Pass Criteria:**
- 100% cache hit rate ✓
- No new cache files ✓
- "Cache hit" log messages present ✓

**Fail Criteria:**
- Any cache misses
- New cache files created
- Missing log messages

---

### TC-I-CACHE-003: ReadOnly Mode with Complete Cache

**Test Type:** System Test
**Requirements:** REQ-CACHE-010
**Priority:** High

**Given:**
- Cache populated from TC-I-CACHE-001
- Same test album set

**When:**
- Run: `cargo run --release --example album_matcher_26 -p wkmp-ai -- --use-cache`

**Then:**
- Run completes successfully
- No API calls made (verify no rate limiting delays)
- All data from cache
- Output identical to TC-I-CACHE-002

**Verify:**
- Run completes in <30 seconds
- Grep logs for "Cache Mode: ReadOnly"
- No error messages about cache misses
- Output matches TC-I-CACHE-002 output (diff the result files)

**Pass Criteria:**
- Success ✓
- Fast completion ✓
- No API calls ✓

**Fail Criteria:**
- Errors
- API calls detected (timing suggests network access)
- Output differs from cached run

---

### TC-I-CACHE-004: ReadOnly Mode with Incomplete Cache

**Test Type:** System Test
**Requirements:** REQ-CACHE-010, REQ-CACHE-090
**Priority:** High

**Given:**
- Partial cache (delete 50% of release cache files)
- Test album set unchanged

**When:**
- Run: `cargo run --release --example album_matcher_26 -p wkmp-ai -- --use-cache`

**Then:**
- Run fails gracefully (does not crash)
- Error messages logged for cache misses
- Stops at first cache miss (does not continue with live API)

**Verify:**
- Exit code non-zero (failure)
- Grep logs for "Cache miss in read-only mode"
- No API calls made (verify no rate limiting delays)
- Partial results saved (if implemented) or clean exit

**Pass Criteria:**
- Graceful failure ✓
- Error logged ✓
- No API calls ✓

**Fail Criteria:**
- Crash (panic)
- API call attempted in ReadOnly mode
- Silent failure (no error message)

---

### TC-I-CACHE-005: Disabled Mode Never Uses Cache

**Test Type:** System Test
**Requirements:** REQ-CACHE-010
**Priority:** High

**Given:**
- Cache populated from TC-I-CACHE-001
- Test album set: 2 albums (for speed)

**When:**
- Run: `cargo run --release --example album_matcher_26 -p wkmp-ai -- --no-cache`

**Then:**
- All queries go to live API (rate limiting delays observed)
- No cache reads (even though cache exists)
- No cache writes (cache files unchanged)

**Verify:**
- Run takes 2-4 minutes (rate limiting delays)
- Grep logs for "Cache Mode: Disabled"
- Count cache files before and after (should be identical, no new files)
- No "Cache hit" or "Cache miss" log messages

**Pass Criteria:**
- API calls made ✓
- No cache access ✓
- Cache files unchanged ✓

**Fail Criteria:**
- Cache accessed (hit or miss logged)
- New cache files created
- Run completes too fast (indicates cache use)

---

### TC-I-CACHE-006: Cache Statistics Report

**Test Type:** Integration Test
**Requirements:** REQ-CACHE-080
**Priority:** Medium

**Given:**
- Cache populated from TC-I-CACHE-001

**When:**
- Run: `cargo run --release --example album_matcher_26 -p wkmp-ai -- --use-cache`

**Then:**
- At end of run, statistics block printed to stdout
- Format matches specification example

**Verify:**
- Grep output for "=== Cache Statistics ==="
- Verify fields present: Cache Mode, Search queries, Release details, Cache hit rate, Cache location
- Verify calculations correct (100% hit rate expected for complete cache)

**Pass Criteria:**
- Statistics block present ✓
- Format matches spec ✓
- Values accurate ✓

**Fail Criteria:**
- Statistics missing
- Wrong format
- Incorrect calculations

---

### TC-I-CACHE-007: Corrupted Cache File Handling

**Test Type:** Integration Test
**Requirements:** REQ-CACHE-090, REQ-NF-CACHE-030
**Priority:** High

**Given:**
- Cache populated from TC-I-CACHE-001
- Corrupt one cache file: `echo "invalid json" > cache/musicbrainz/releases/{some-mbid}.json`

**When:**
- Run: `cargo run --release --example album_matcher_26 -p wkmp-ai` (ReadWrite mode)

**Then:**
- Run completes successfully (does not crash)
- Corrupted file detected, warning logged
- Falls back to live API for that specific release
- Re-caches with valid data (overwrites corrupt file)

**Verify:**
- Grep logs for "Cache file parse failure" or similar warning
- Run completes without errors
- Corrupted file overwritten with valid JSON (verify file is valid after run)

**Pass Criteria:**
- No crash ✓
- Warning logged ✓
- Fallback to API ✓
- File repaired ✓

**Fail Criteria:**
- Crash/panic
- No warning logged
- Corrupt file not repaired

---

### TC-I-CACHE-008: Output Identical (Cached vs. Live)

**Test Type:** System Test
**Requirements:** REQ-CACHE-120, REQ-NF-CACHE-020
**Priority:** High

**Given:**
- Test album set: 5 albums

**When:**
- Run 1 (build cache): `cargo run --release --example album_matcher_26 -p wkmp-ai 2>&1 | tee output_live.txt`
- Run 2 (use cache): `cargo run --release --example album_matcher_26 -p wkmp-ai -- --use-cache 2>&1 | tee output_cached.txt`

**Then:**
- Both runs complete successfully
- Output identical (except cache-related log messages)

**Verify:**
- Extract final results from both outputs (album matches, percentages, MBIDs)
- Compare results (should be byte-for-byte identical)
- Strip cache-specific logs ("Cache hit", timing differences)
- Diff stripped outputs (should show zero differences)

**Pass Criteria:**
- Results identical ✓
- Same albums matched ✓
- Same percentages ✓
- Same MBIDs ✓

**Fail Criteria:**
- Any result differences
- Different match percentages
- Different MBIDs selected

---

### Additional Unit Tests

**TC-U-CACHE-010: CacheMode Enum**
- Verify 3 variants exist (Disabled, ReadWrite, ReadOnly)
- Verify derives (Debug, Clone, Copy)

**TC-U-CACHE-020: Data Structures**
- Verify all 5 structs compile
- Verify Serialize/Deserialize derives work
- Test JSON round-trip (serialize → deserialize → verify identical)

**TC-U-CACHE-040: MBClient Constructor**
- Verify initializes with CacheConfig
- Verify creates cache directories
- Verify loads/creates metadata.json

**TC-U-CACHE-060: CLI Argument Parsing**
- Test `--no-cache` sets Disabled mode
- Test `--use-cache` sets ReadOnly mode
- Test no args sets ReadWrite mode (default)
- Test invalid args show error message

---

### Performance Tests

**TC-P-CACHE-010: Cache Lookup Speed**
- Measure time to read and parse cache file
- Assert: <1ms average across 100 reads

**TC-P-CACHE-011: Cache Store Speed**
- Measure time to serialize and write cache file
- Assert: <5ms average across 100 writes

**TC-P-CACHE-012: ReadOnly Mode Speedup**
- Measure full run time in Disabled mode (200 albums): ~30-60 minutes
- Measure full run time in ReadOnly mode (200 albums): <30 seconds
- Assert: 10-20× speedup achieved

---

## Test Execution Strategy

**Phase 1: Unit Tests (Quick Validation)**
1. TC-U-CACHE-010, TC-U-CACHE-020, TC-U-CACHE-040, TC-U-CACHE-060
2. Estimated time: <5 minutes
3. Goal: Verify code compiles and basic functionality

**Phase 2: Integration Tests (Core Functionality)**
1. TC-I-CACHE-001, TC-I-CACHE-002 (first and second run)
2. TC-I-CACHE-003, TC-I-CACHE-004, TC-I-CACHE-005 (three cache modes)
3. TC-I-CACHE-007 (error handling)
4. Estimated time: 45-60 minutes (includes first cache build)
5. Goal: Verify caching works correctly in all modes

**Phase 3: System Tests (End-to-End Validation)**
1. TC-I-CACHE-008 (output comparison)
2. TC-I-CACHE-006 (statistics reporting)
3. Estimated time: 15-20 minutes
4. Goal: Verify integration and compatibility

**Phase 4: Performance Tests (Validation)**
1. TC-P-CACHE-010, TC-P-CACHE-011, TC-P-CACHE-012
2. Estimated time: 90 minutes (includes full 200-album run)
3. Goal: Verify performance requirements met

**Total Estimated Testing Time:** 2.5-3 hours

---

## Success Criteria

**All Tests Must Pass:**
- 24/24 tests passing
- No crashes or panics
- All performance goals met
- Output compatibility verified

**Test Coverage:**
- 17/17 requirements have tests
- 100% requirement coverage
- All error scenarios tested
- All three cache modes tested

---

## Test Data

**Small Test Set (5 albums):** For fast iteration
- Sneaker Pimps - Becoming X
- Elton John - Goodbye Yellow Brick Road
- Gerry Rafferty - City to City
- Oasis - (What's the Story) Morning Glory?
- Chumbawamba - Tubthumper

**Full Test Set (200 albums):** For final validation
- Use existing test set from album_matcher_25.rs runs
