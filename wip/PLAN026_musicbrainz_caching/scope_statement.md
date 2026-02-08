# Scope Statement: PLAN026 - MusicBrainz API Caching

**Plan:** PLAN026 - MusicBrainz API Caching for Album Matcher
**Date:** 2025-01-24

---

## ✅ In Scope

**What WILL be implemented:**

1. **Cache Infrastructure:**
   - Cache directory structure (cache/musicbrainz/searches/, releases/)
   - Cache metadata tracking (metadata.json)
   - JSON-based storage with pretty-printing

2. **Three Cache Modes:**
   - Disabled mode (`--no-cache`): Live API only
   - ReadWrite mode (default): Cache + fallback to API on miss
   - ReadOnly mode (`--use-cache`): Cached data only, error on miss

3. **MBClient API Wrapper:**
   - `search_releases()` method with transparent caching
   - `get_release_details()` method with transparent caching
   - Drop-in replacement for existing HTTP client usage
   - Maintains existing function signatures and query stats

4. **Data Structures:**
   - `CacheMode` enum (3 variants)
   - `CacheConfig` struct (mode + directory)
   - `CachedSearch` struct (query + timestamp + response)
   - `CachedRelease` struct (mbid + timestamp + details)
   - `CacheMetadata` struct (version + counts + timestamps)

5. **Integration with album_matcher_25.rs:**
   - Copy album_matcher_25.rs → album_matcher_26.rs
   - Replace HTTP client instantiation with MBClient
   - Update `search_all_mb_strategies()` to use MBClient
   - Update `fetch_release_track_details()` to use MBClient
   - Add command-line argument parsing (--no-cache, --use-cache)

6. **Cache Operations:**
   - Search query hashing (SHA-256, first 16 chars)
   - Cache lookup (disk read)
   - Cache storage (disk write with error handling)
   - Cache hit/miss logging
   - Cache statistics reporting

7. **Error Handling (5 scenarios):**
   - Cache directory creation failure → fall back to live API
   - Cache file read failure → query live API, attempt to cache
   - Cache file write failure → log error, continue execution
   - Cache file parse failure → invalidate entry, query live API
   - ReadOnly mode cache miss → return error (don't query API)

8. **Testing:**
   - 8 test cases covering first run, cached run, all 3 modes, error handling
   - Verify output identical between cached and live runs
   - Measure performance improvement (10-20× speedup expected)

---

## ❌ Out of Scope

**What will NOT be implemented:**

1. **Cache Invalidation:**
   - Automatic expiration of old entries
   - Cache version migration
   - Selective cache clearing (除 manual `rm -rf cache/`)

2. **Advanced Features:**
   - Cache compression (zlib, gzip)
   - Network cache (shared across machines)
   - Cache synchronization (multi-user scenarios)

3. **Configuration Beyond Command-Line:**
   - Config file for cache settings
   - Environment variables for cache location
   - Per-album or per-run cache controls

4. **Optimization:**
   - In-memory cache layer (LRU)
   - Cache pre-warming
   - Parallel cache population

5. **Monitoring/Observability:**
   - Cache hit rate metrics over time
   - Cache size monitoring/alerts
   - Cache performance profiling

6. **Changes to algorithm_matcher Logic:**
   - No changes to NDR ranking
   - No changes to filtering strategies
   - No changes to acoustic fingerprinting
   - Cache is transparent layer, algorithm unchanged

---

## Assumptions

**Explicit statements of what is assumed true:**

1. **File System:**
   - Write access to current directory (for ./cache/)
   - Sufficient disk space for cache (assume <1GB for 200-album test set)
   - File system supports long filenames (MBID = 36 chars UUID)

2. **Existing Code:**
   - album_matcher_25.rs is stable and tested
   - Existing `MBSearchResponse` and `MBReleaseDetails` structs are Serialize/Deserialize
   - Existing query stats tracking can be maintained

3. **Dependencies:**
   - sha2 crate already in Cargo.toml (confirmed)
   - serde/serde_json already in Cargo.toml (confirmed)
   - chrono already in Cargo.toml (for ISO 8601 timestamps)

4. **Usage Pattern:**
   - Cache will be manually deleted when algorithm changes significantly
   - Cache corruption is rare (disk errors, power failure)
   - ReadOnly mode only used after successful ReadWrite run (cache populated)

5. **Performance:**
   - Disk I/O is fast enough (<1ms read, <5ms write)
   - JSON parsing overhead is acceptable
   - Windows path handling works correctly with PathBuf

---

## Constraints

### Technical Constraints

1. **Compatibility:**
   - Output format must match album_matcher_25.rs exactly (when cache disabled)
   - Log format must be compatible (only add cache hit/miss messages, no removals)
   - Command-line behavior unchanged when `--no-cache` used

2. **Platform:**
   - Must work on Windows (path separators, line endings)
   - Path handling must use PathBuf (cross-platform)
   - File I/O must handle Windows permissions

3. **Data Structures:**
   - Cannot modify existing `MBSearchResponse` or `MBReleaseDetails` structs
   - Must preserve all fields (even if unused) for API completeness

4. **Error Handling:**
   - Cache failures cannot crash application (except ReadOnly cache miss)
   - Must degrade gracefully to live API on cache errors
   - Partial cache population acceptable (don't require atomic operations)

### Process Constraints

1. **Testing:**
   - Must test with existing 200-album test set
   - Must verify identical output between cached/live runs
   - Must measure and document performance improvement

2. **Documentation:**
   - Update album_matcher_25_designs.md with caching workflow
   - Document cache directory structure
   - Provide examples of all 3 cache modes

3. **Code Quality:**
   - Minimal changes to existing album_matcher_25 code paths
   - Preserve existing function signatures where possible
   - Follow Rust conventions (Error/Result types, async/await)

### Timeline Constraints

1. **Implementation:**
   - Estimated 6-10 hours total effort
   - Can be completed in single session
   - No external dependencies or approvals needed

2. **Validation:**
   - First run (build cache): ~30-60 minutes
   - Cached run validation: <30 seconds
   - Algorithm tuning workflow: <5 minutes for 10 parameter variations

---

## Success Metrics

### Quantitative Metrics

1. **Performance:**
   - ✅ ReadOnly mode completes in <30 seconds (vs. 30+ minutes live)
   - ✅ Cache lookup: <1ms per entry
   - ✅ Cache store: <5ms per entry
   - ✅ 10-20× speedup achieved in ReadOnly mode

2. **Correctness:**
   - ✅ 100% output match between cached and live runs
   - ✅ 100% cache hit rate after first run (ReadOnly mode)
   - ✅ All 8 test cases pass

3. **Coverage:**
   - ✅ All 17 requirements implemented
   - ✅ All error scenarios handled gracefully

### Qualitative Metrics

1. **Usability:**
   - ✅ Workflow: build cache once, test 10+ parameter variations in <5 minutes
   - ✅ Command-line interface intuitive (--use-cache, --no-cache)
   - ✅ Cache files human-readable (can inspect/debug)

2. **Maintainability:**
   - ✅ Minimal changes to existing code (< 10% LOC change)
   - ✅ Clear separation of concerns (MBClient wrapper)
   - ✅ Code follows Rust conventions

3. **Reliability:**
   - ✅ Cache failures don't break runs
   - ✅ Corrupted cache entries detected
   - ✅ Manual cache deletion works (rm -rf cache/)

---

## Dependencies Summary

**Existing Code (READ):**
- album_matcher_25.rs (source for copy, ~4500 lines)

**Existing Dependencies (ALREADY PRESENT):**
- serde, serde_json (JSON serialization)
- sha2 (SHA-256 hashing)
- chrono (ISO 8601 timestamps)
- reqwest (HTTP client)

**New Code (CREATE):**
- album_matcher_26.rs (copy + additions)
- MBClient implementation (~300-400 lines estimated)
- Cache helper functions (~200 lines estimated)
- Command-line parsing (~50 lines estimated)
- Statistics reporting (~50 lines estimated)

**No External Dependencies Required** ✓

---

## Risk Assessment

**Low Risk Implementation:**

1. **Copy-based approach:** Start from working album_matcher_25.rs
2. **Additive changes:** Cache layer is addition, not modification
3. **Transparent wrapper:** Existing code paths unchanged structurally
4. **Graceful degradation:** Cache failures fall back to working live API
5. **Validation:** Can compare output to album_matcher_25 for correctness

**Mitigation:**
- Test with small album subset first (5-10 albums)
- Keep album_matcher_25.rs unchanged (fallback option)
- Validate output matches before full 200-album run
