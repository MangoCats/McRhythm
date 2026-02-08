# Dependencies Map: PLAN026 - MusicBrainz API Caching

**Plan:** PLAN026 - MusicBrainz API Caching for Album Matcher
**Date:** 2025-01-24

---

## Existing Code Dependencies

### Source File for Copy

**album_matcher_25.rs**
- Location: `wkmp-ai/examples/album_matcher_25.rs`
- Status: ✅ **EXISTS** (~4500 lines)
- Purpose: Source file to copy and modify
- Usage: Will be copied to album_matcher_26.rs
- Dependencies: READ only, no modifications to original

### Related Documentation

**album_matcher_25_designs.md**
- Location: Root directory
- Status: ✅ **EXISTS**
- Purpose: Design documentation for album matcher pipeline
- Usage: Will be updated with caching workflow section
- Dependencies: APPEND new section

---

## Rust Crate Dependencies

### Already Present in Cargo.toml

| Crate | Version | Purpose | Status |
|-------|---------|---------|--------|
| `serde` | workspace | JSON serialization (Serialize/Deserialize derives) | ✅ Available |
| `serde_json` | workspace | JSON parsing and generation | ✅ Available |
| `sha2` | 0.10 | SHA-256 hashing for cache keys | ✅ Available |
| `chrono` | workspace | ISO 8601 timestamp formatting | ✅ Available |
| `reqwest` | 0.11 | HTTP client (wrapped by MBClient) | ✅ Available |

**Verification:**
```toml
# From wkmp-ai/Cargo.toml (confirmed via earlier file reads)
serde = { workspace = true }
serde_json = { workspace = true }
sha2 = "0.10"
chrono = { workspace = true }
reqwest = { version = "0.11", features = ["json"] }
```

### No New Dependencies Required

✅ **All required dependencies already present in project**

---

## External Dependencies

### MusicBrainz API

**Status:** External service (no change)
**Endpoint:** https://musicbrainz.org/ws/2/
**Rate Limit:** 1 request/second
**Authentication:** None (public API)
**Dependency Type:** RUNTIME (when cache disabled or on cache miss)

**Impact of Caching:**
- Disabled mode: Full dependency (all queries go to API)
- ReadWrite mode: Partial dependency (cache misses only)
- ReadOnly mode: Zero dependency (no API calls)

### File System

**Requirements:**
- Write access to current directory
- ~1GB disk space for cache (200-album test set)
- Long filename support (36-char UUIDs)

**Platforms:**
- Windows (primary target - confirmed in constraints)
- Linux/macOS (should work, not explicitly tested)

---

## Data Structure Dependencies

### Existing Structs (Must Not Modify)

**MBSearchResponse**
- Defined in: album_matcher_25.rs
- Purpose: Represents MusicBrainz search API response
- Status: ✅ EXISTS, must remain unchanged
- Usage: Wrapped in `CachedSearch` struct
- Requirement: Must have Serialize/Deserialize derives

**MBReleaseDetails**
- Defined in: album_matcher_25.rs
- Purpose: Represents MusicBrainz release details API response
- Status: ✅ EXISTS, must remain unchanged
- Usage: Wrapped in `CachedRelease` struct
- Requirement: Must have Serialize/Deserialize derives

**RateLimiter**
- Defined in: album_matcher_25.rs
- Purpose: Rate limiting for API calls (1/second)
- Status: ✅ EXISTS, will be reused
- Usage: Used by MBClient in Disabled and ReadWrite modes
- Requirement: MBClient must integrate with existing rate limiter

**QueryStats**
- Defined in: album_matcher_25.rs
- Purpose: Track API query statistics
- Status: ✅ EXISTS, will be maintained
- Usage: MBClient must update stats for compatibility
- Requirement: Preserve existing stats tracking behavior

### New Structs (To Be Created)

**CacheMode** (enum)
- Purpose: Three cache modes (Disabled, ReadWrite, ReadOnly)
- Dependencies: None
- Size: ~20 lines

**CacheConfig** (struct)
- Purpose: Cache configuration (mode + directory)
- Dependencies: CacheMode, PathBuf (std)
- Size: ~10 lines

**CachedSearch** (struct)
- Purpose: Cached search query response
- Dependencies: MBSearchResponse (existing), String (std)
- Size: ~15 lines

**CachedRelease** (struct)
- Purpose: Cached release details
- Dependencies: MBReleaseDetails (existing), String (std)
- Size: ~15 lines

**CacheMetadata** (struct)
- Purpose: Cache metadata tracking
- Dependencies: String (std), usize (std)
- Size: ~20 lines

**MBClient** (struct)
- Purpose: MusicBrainz API wrapper with caching
- Dependencies: All new structs + reqwest::Client + RateLimiter
- Size: ~300-400 lines (implementation)

---

## Function Dependencies

### Existing Functions (To Be Modified)

**search_all_mb_strategies()**
- Location: album_matcher_25.rs line ~2450
- Current: Creates reqwest::Client, calls MusicBrainz API directly
- Change: Create MBClient instead, call mb_client.search_releases()
- Impact: ~10-20 lines modified

**fetch_release_track_details()**
- Location: album_matcher_25.rs line ~2641
- Current: Takes reqwest::Client, calls API directly
- Change: Take MBClient reference, call mb_client.get_release_details()
- Impact: ~5-10 lines modified

**main()**
- Location: album_matcher_25.rs (top-level)
- Current: No command-line argument parsing for cache
- Change: Add argument parsing, create CacheConfig, pass to functions
- Impact: ~50 lines added

### New Functions (To Be Created)

**MBClient::new()**
- Purpose: Constructor, initialize cache
- Dependencies: PathBuf, CacheConfig
- Size: ~50 lines

**MBClient::search_releases()**
- Purpose: Cached search query method
- Dependencies: Cache helper functions
- Size: ~80-100 lines

**MBClient::get_release_details()**
- Purpose: Cached release details method
- Dependencies: Cache helper functions
- Size: ~80-100 lines

**Cache Helper Functions** (6 functions)
- `hash_query()`: SHA-256 hashing
- `load_cached_search()`: Disk read + JSON parse
- `store_search_cache()`: JSON serialize + disk write
- `load_cached_release()`: Disk read + JSON parse
- `store_release_cache()`: JSON serialize + disk write
- `update_metadata()`: Update metadata.json
- Total size: ~200 lines

**Statistics Reporting**
- `print_cache_statistics()`: End-of-run statistics
- Size: ~50 lines

---

## Integration Points

### Where Changes Will Be Made

1. **Top of album_matcher_26.rs:**
   - Add new data structure definitions (5 structs/enums)
   - Add MBClient implementation
   - Add cache helper functions
   - Estimated: +550 lines

2. **main() function:**
   - Add command-line argument parsing
   - Create CacheConfig based on args
   - Pass cache config to album processing
   - Estimated: +50 lines

3. **search_all_mb_strategies() function:**
   - Replace `reqwest::Client::new()` with `MBClient::new(cache_config)`
   - Replace direct HTTP calls with `mb_client.search_releases()`
   - Estimated: ~20 lines modified

4. **fetch_release_track_details() function:**
   - Change parameter from `client: &reqwest::Client` to `mb_client: &MBClient`
   - Replace direct HTTP call with `mb_client.get_release_details()`
   - Estimated: ~10 lines modified

5. **End of main() (before exit):**
   - Add call to `print_cache_statistics()`
   - Estimated: +50 lines

**Total Estimated Changes:**
- Lines added: ~650
- Lines modified: ~30
- Total file size: ~5150 lines (from ~4500)
- Change percentage: ~12% (within acceptable range)

---

## Dependency Graph

```
album_matcher_26.rs
├── NEW: Cache Infrastructure
│   ├── CacheMode enum
│   ├── CacheConfig struct
│   ├── CachedSearch struct (wraps MBSearchResponse)
│   ├── CachedRelease struct (wraps MBReleaseDetails)
│   ├── CacheMetadata struct
│   └── MBClient struct
│       ├── Uses: reqwest::Client (existing)
│       ├── Uses: RateLimiter (existing)
│       ├── Uses: QueryStats (existing)
│       └── Implements: search_releases(), get_release_details()
│
├── MODIFIED: Integration Points
│   ├── main() - parse args, create MBClient
│   ├── search_all_mb_strategies() - use MBClient
│   └── fetch_release_track_details() - use MBClient
│
└── UNCHANGED: Algorithm Logic
    ├── NDR ranking
    ├── Runtime filtering
    ├── Acoustic fingerprinting
    └── Result selection

External Dependencies:
├── serde, serde_json (JSON) - ✅ Available
├── sha2 (hashing) - ✅ Available
├── chrono (timestamps) - ✅ Available
└── reqwest (HTTP) - ✅ Available
```

---

## Dependency Risk Assessment

**LOW RISK:**

1. ✅ All Rust dependencies already present (no new crates)
2. ✅ Source file exists and is stable (album_matcher_25.rs)
3. ✅ No breaking changes to existing code (additive pattern)
4. ✅ External API unchanged (MusicBrainz)
5. ✅ Graceful degradation on cache failures

**No Blockers Identified**

---

## Dependency Verification Checklist

- [x] Source file album_matcher_25.rs exists and is readable
- [x] All required Rust crates present in Cargo.toml
- [x] Existing data structures (MBSearchResponse, MBReleaseDetails) have Serialize/Deserialize
- [x] RateLimiter exists and can be integrated
- [x] QueryStats exists and can be maintained
- [x] File system supports required operations (read/write, long filenames)
- [x] No conflicting dependencies identified
- [x] No version conflicts identified

**Status:** ✅ All dependencies verified, no blockers
