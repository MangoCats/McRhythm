# Specification: MusicBrainz API Caching for Album Matcher

## Document Information

**Document ID:** SPEC-CACHE-001
**Version:** 1.0
**Date:** 2025-01-24
**Status:** Draft
**Related Documents:**
- `album_matcher_25.rs` - Current implementation (source for copy)
- `album_matcher_25_designs.md` - Design documentation

## Purpose

Implement transparent caching layer for MusicBrainz API calls to enable rapid iteration on album identification algorithms without waiting for API rate limiting (1 request/second). Cache enables testing different filtering strategies, thresholds, and ranking algorithms in seconds instead of hours.

## Background

**Current State:**
- Album matcher queries MusicBrainz API directly
- Rate limit: 1 request/second
- Testing 200 albums takes ~30-60 minutes due to rate limiting
- Algorithm tuning requires many test runs
- Each test run queries same data repeatedly

**Problem:**
- Slow iteration cycles inhibit experimentation
- Cannot quickly test parameter variations
- Expensive to test algorithm changes (time cost)

**Solution:**
- Cache MusicBrainz API responses to disk
- Replay cached responses during tuning
- Switch back to live API for final validation

## Requirements

### REQ-CACHE-010: Cache Mode Configuration

**SHALL** support three cache modes selectable via command-line argument:

1. **Disabled** (flag: `--no-cache`): Normal operation, all queries go to live MusicBrainz API
2. **ReadWrite** (default): Cache hits return cached data, cache misses query API and store result
3. **ReadOnly** (flag: `--use-cache`): Only use cached data, error if cache miss

**Rationale:** Transparent switching between cached and live modes enables tuning (ReadOnly) then validation (Disabled).

### REQ-CACHE-020: Search Query Caching

**SHALL** cache MusicBrainz release search queries with the following:

- **Cache Key:** SHA-256 hash of query string (first 16 hex chars)
- **Cache Format:** JSON file containing:
  - Original query string (for human inspection)
  - Timestamp (ISO 8601 format)
  - Complete `MBSearchResponse` structure
- **Cache Location:** `./cache/musicbrainz/searches/{hash}.json`

**Example Cache File (`cache/musicbrainz/searches/a3f5d9e8c1b2.json`):**
```json
{
  "query": "artist:\"Sneaker Pimps\" AND release:\"Becoming X\"",
  "timestamp": "2025-01-24T15:32:45Z",
  "response": {
    "releases": [
      { "id": "0e60ad28-3654-464b-befc-96eb7ee0ff88", "title": "Becoming X", ... }
    ]
  }
}
```

### REQ-CACHE-030: Release Details Caching

**SHALL** cache MusicBrainz release detail queries with the following:

- **Cache Key:** Release MBID (exact UUID string)
- **Cache Format:** JSON file containing:
  - Release MBID (for verification)
  - Timestamp (ISO 8601 format)
  - Complete `MBReleaseDetails` structure
- **Cache Location:** `./cache/musicbrainz/releases/{mbid}.json`

**Example Cache File (`cache/musicbrainz/releases/0e60ad28-3654-464b-befc-96eb7ee0ff88.json`):**
```json
{
  "mbid": "0e60ad28-3654-464b-befc-96eb7ee0ff88",
  "timestamp": "2025-01-24T15:32:47Z",
  "details": {
    "id": "0e60ad28-3654-464b-befc-96eb7ee0ff88",
    "title": "Becoming X",
    "media": [ ... ]
  }
}
```

### REQ-CACHE-040: Transparent API Wrapper

**SHALL** implement `MBClient` wrapper that:

1. Accepts cache configuration at construction
2. Provides `search_releases(query)` method that transparently caches/retrieves
3. Provides `get_release_details(mbid)` method that transparently caches/retrieves
4. Maintains existing function signatures (drop-in replacement)
5. Respects rate limiting in Disabled and ReadWrite modes
6. Bypasses rate limiting in ReadOnly mode (no network access)

**Interface:**
```rust
struct MBClient {
    // Construction
    fn new(cache_config: CacheConfig) -> Self;

    // API Methods (transparent caching)
    async fn search_releases(&self, query: &str, stats: Option<&QueryStats>)
        -> Result<MBSearchResponse, String>;

    async fn get_release_details(&self, mbid: &str, stats: Option<&QueryStats>)
        -> Result<MBReleaseDetails, String>;
}
```

### REQ-CACHE-050: Cache Directory Structure

**SHALL** organize cache with the following structure:

```
cache/
  musicbrainz/
    searches/
      {hash1}.json
      {hash2}.json
      ...
    releases/
      {mbid1}.json
      {mbid2}.json
      ...
    metadata.json
```

**metadata.json** SHALL contain:
```json
{
  "version": "album_matcher_26",
  "created": "2025-01-24T15:30:00Z",
  "search_count": 42,
  "release_count": 156,
  "last_updated": "2025-01-24T16:45:23Z"
}
```

### REQ-CACHE-060: Command-Line Interface

**SHALL** accept the following command-line arguments:

- `--no-cache`: Disable caching (live API only)
- `--use-cache`: Read-only mode (cached data only, error on miss)
- (no flag): ReadWrite mode (default, build cache on misses)

**Examples:**
```bash
# Build cache on first run (default ReadWrite mode)
cargo run --release --example album_matcher_26 -p wkmp-ai

# Use cached data only (tuning mode)
cargo run --release --example album_matcher_26 -p wkmp-ai -- --use-cache

# Force live API (final validation)
cargo run --release --example album_matcher_26 -p wkmp-ai -- --no-cache
```

### REQ-CACHE-070: Cache Hit/Miss Logging

**SHALL** log cache operations:

- Cache hit: `info!("Cache hit: search query")`
- Cache hit: `info!("Cache hit: release {}", &mbid[..8])`
- Cache miss (ReadWrite): `info!("Cache miss: querying API")`
- Cache miss (ReadOnly): `error!("Cache miss in read-only mode: {}", query)`

### REQ-CACHE-080: Cache Statistics Report

**SHALL** print cache statistics at end of run:

```
=== Cache Statistics ===
Cache Mode: ReadOnly
Search queries: 42 (42 hits, 0 misses)
Release details: 156 (156 hits, 0 misses)
Cache hit rate: 100.0%
Cache location: ./cache
Total API calls avoided: 198
Estimated time saved: ~198 seconds
```

### REQ-CACHE-090: Error Handling

**SHALL** handle cache errors gracefully:

1. **Cache directory creation failure:** Log error, fall back to live API
2. **Cache file read failure:** Log warning, query live API, attempt to cache result
3. **Cache file write failure:** Log error, continue execution (don't fail run)
4. **Cache file parse failure:** Log error, invalidate cache entry, query live API
5. **ReadOnly mode cache miss:** Return error, do NOT query live API

**Rationale:** Cache failures should not break runs, except in ReadOnly mode where cache miss is an error condition.

### REQ-CACHE-100: Data Structures

**SHALL** define the following new data structures:

```rust
/// Cache mode configuration
#[derive(Debug, Clone, Copy)]
enum CacheMode {
    Disabled,       // Live API only
    ReadWrite,      // Cache + API fallback
    ReadOnly,       // Cache only, error on miss
}

/// Cache configuration
struct CacheConfig {
    mode: CacheMode,
    cache_dir: PathBuf,
}

/// Cached search response
#[derive(Serialize, Deserialize)]
struct CachedSearch {
    query: String,
    timestamp: String,        // ISO 8601 format
    response: MBSearchResponse,
}

/// Cached release details
#[derive(Serialize, Deserialize)]
struct CachedRelease {
    mbid: String,
    timestamp: String,        // ISO 8601 format
    details: MBReleaseDetails,
}

/// Cache metadata (cache/musicbrainz/metadata.json)
#[derive(Serialize, Deserialize)]
struct CacheMetadata {
    version: String,          // "album_matcher_26"
    created: String,          // ISO 8601 timestamp
    search_count: usize,
    release_count: usize,
    last_updated: String,     // ISO 8601 timestamp
}
```

### REQ-CACHE-110: MBClient Implementation Requirements

**SHALL** implement `MBClient` with:

1. **Constructor:** `new(cache_config: CacheConfig) -> Self`
   - Stores cache config
   - Initializes HTTP client
   - Creates cache directories if needed
   - Loads or creates metadata.json

2. **Search method:** `async fn search_releases(...) -> Result<...>`
   - Checks cache mode
   - If Disabled: Query API directly
   - If ReadWrite: Check cache, query API on miss, store result
   - If ReadOnly: Check cache, error on miss
   - Respects rate limiting (except ReadOnly mode)
   - Updates cache statistics

3. **Details method:** `async fn get_release_details(...) -> Result<...>`
   - Same logic as search method
   - Cache key is MBID (no hashing needed)

4. **Helper methods:**
   - `hash_query(query: &str) -> String` - SHA-256 hash, first 16 chars
   - `load_cached_search(&self, key: &str) -> Result<Option<CachedSearch>, String>`
   - `store_search_cache(&self, key: &str, query: &str, response: &MBSearchResponse) -> Result<(), String>`
   - `load_cached_release(&self, mbid: &str) -> Result<Option<CachedRelease>, String>`
   - `store_release_cache(&self, mbid: &str, details: &MBReleaseDetails) -> Result<(), String>`
   - `update_metadata(&self) -> Result<(), String>`

### REQ-CACHE-120: Integration with Existing Code

**SHALL** integrate with album_matcher_25.rs by:

1. Replace direct HTTP client usage with `MBClient` wrapper
2. Update `search_all_mb_strategies()` to use `mb_client.search_releases()`
3. Update `fetch_release_track_details()` to use `mb_client.get_release_details()`
4. Preserve all existing function signatures (minimize code changes)
5. Maintain compatibility with existing query stats tracking

**Migration Strategy:**
- Copy album_matcher_25.rs → album_matcher_26.rs
- Add new data structures (CacheMode, CacheConfig, etc.)
- Implement MBClient wrapper
- Replace HTTP client instantiation with MBClient
- Update call sites to use MBClient methods
- Add command-line argument parsing
- Add cache statistics reporting

### REQ-CACHE-130: Human-Readable Cache Format

**SHALL** use JSON format with pretty-printing for all cache files:

- `serde_json::to_string_pretty()` for serialization
- Indented, newline-separated
- Easy to inspect, diff, and manually edit

**Rationale:** Human-readable format enables:
- Manual cache inspection
- Debugging (verify cached data is correct)
- Version control (can commit cache for reproducibility)
- Manual editing if needed (e.g., fixing corrupted entries)

### REQ-CACHE-140: Cache Invalidation (Future)

**SHOULD** consider cache invalidation mechanisms:

- Cache entries older than N days
- Cache version mismatch (metadata.version != current version)
- Manual cache clearing command

**Note:** Not required for initial implementation. Cache can be manually deleted (`rm -rf cache/`) if needed.

## Non-Functional Requirements

### REQ-NF-CACHE-010: Performance

**SHALL** achieve:
- Cache lookup: <1ms per entry (disk read)
- Cache store: <5ms per entry (disk write)
- ReadOnly mode: 10-20× faster than live API (no rate limiting)

### REQ-NF-CACHE-020: Compatibility

**SHALL** maintain:
- Same output format as album_matcher_25.rs
- Same log format (with additional cache hit/miss messages)
- Same command-line behavior (when cache disabled)

### REQ-NF-CACHE-030: Reliability

**SHALL** ensure:
- Cache failures do not crash application (except ReadOnly mode on cache miss)
- Corrupted cache entries are detected and skipped
- Partial cache population is acceptable (some entries cached, some not)

## Success Criteria

**Implementation Successful When:**

1. ✅ Can run album_matcher_26 in default mode, builds cache
2. ✅ Can run album_matcher_26 with `--use-cache`, completes in <30 seconds (vs. 30+ minutes)
3. ✅ Can run album_matcher_26 with `--no-cache`, behaves identically to album_matcher_25
4. ✅ Cache hit rate 100% after first run (ReadOnly mode)
5. ✅ Cache files are human-readable JSON
6. ✅ Cache statistics report shows hits/misses correctly
7. ✅ Algorithm tuning workflow: build cache once, test 10+ parameter variations in <5 minutes

## Testing Strategy

**Test Cases:**

1. **TC-CACHE-001:** First run (empty cache) builds cache correctly
2. **TC-CACHE-002:** Second run (populated cache) uses cached data
3. **TC-CACHE-003:** ReadOnly mode with complete cache succeeds
4. **TC-CACHE-004:** ReadOnly mode with incomplete cache fails gracefully
5. **TC-CACHE-005:** Disabled mode never reads/writes cache
6. **TC-CACHE-006:** Cache statistics report accurate
7. **TC-CACHE-007:** Corrupted cache file detected and skipped
8. **TC-CACHE-008:** Output identical between cached and live runs

## Dependencies

**Existing Code:**
- `album_matcher_25.rs` - Source for copy

**Rust Dependencies:**
- `serde` - JSON serialization (already present)
- `serde_json` - JSON parsing (already present)
- `sha2` - SHA-256 hashing (already present in Cargo.toml)
- `reqwest` - HTTP client (already present)

**No New External Dependencies Required**

## Implementation Notes

**Scope:**
- Create `album_matcher_26.rs` by copying `album_matcher_25.rs`
- Add caching functionality on top of existing implementation
- Preserve all existing functionality
- Minimal changes to existing code paths

**Out of Scope:**
- Cache invalidation policies
- Cache compression
- Network cache (shared cache across machines)
- Cache versioning/migration

## Constraints

**Technical:**
- Must maintain compatibility with album_matcher_25.rs output format
- Must work on Windows (path handling, line endings)
- Cache directory location configurable (future enhancement)

**Process:**
- Test with existing 200-album test set
- Verify identical results between cached and live runs
- Document workflow in album_matcher_25_designs.md

## Glossary

**Cache Hit:** Requested data found in cache, no API call needed
**Cache Miss:** Requested data not in cache, API call required
**ReadWrite Mode:** Default mode, builds cache on misses
**ReadOnly Mode:** Tuning mode, only uses cached data
**Disabled Mode:** Live API mode, ignores cache entirely
**MBClient:** Wrapper class that transparently caches MusicBrainz API calls
