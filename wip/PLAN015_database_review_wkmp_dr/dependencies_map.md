# Dependencies Map: Database Review Module (wkmp-dr)

**Plan:** PLAN015 - Database Review Module (wkmp-dr)
**Created:** 2025-11-01

---

## Dependency Categories

1. **Existing Code** - WKMP codebase we depend on
2. **External Libraries** - Cargo crates required
3. **Runtime Dependencies** - What must exist at runtime
4. **Development Dependencies** - Build/test tooling
5. **Integration Points** - Where we connect to other modules

---

## 1. Existing Code Dependencies

### wkmp-common Library (CRITICAL)

**Status:** ✅ Exists, stable
**Location:** `/home/sw/Dev/McRhythm/common/`
**Version:** Current main branch

**Used Components:**
| Component | Purpose | File Location |
|-----------|---------|---------------|
| `config::RootFolderResolver` | 4-tier root folder resolution | common/src/config.rs:186-345 |
| `config::RootFolderInitializer` | Directory creation, database path | common/src/config.rs:347-385 |
| `db::init::init_database()` | Database pool initialization | common/src/db/init.rs:15 |
| `db::models::*` | Database entity types | common/src/db/models.rs:10-48 |
| `events::EventBus` | (Optional) Real-time updates | common/src/events.rs:1131-1257 |
| `api::auth::*` | Authentication utilities | common/src/api/auth.rs |
| `error::Error` | Common error type | common/src/error.rs:9-28 |

**API Stability:** High (used by all 5 existing modules)

**Risk:** Low - well-tested utilities
**Mitigation:** Use stable APIs only, avoid experimental features

---

### Database Schema (CRITICAL)

**Status:** ✅ Exists, defined in wkmp-common
**Location:** `common/src/db/init.rs` (lines 62-403)

**Tables wkmp-dr Will Query:**
| Table | Purpose | Columns | Est. Rows |
|-------|---------|---------|-----------|
| `users` | Authentication | username, password_hash, salt | 1-10 |
| `settings` | Key-value config | key, value | 10-50 |
| `module_config` | Microservice config | module_name, host, port, enabled | 5 |
| `files` | Audio file metadata | guid, path, hash, duration | 100-100,000 |
| `passages` | Playable segments | guid, file_id, start_time, end_time, fade points | 500-500,000 |
| `queue` | Playback queue | guid, file_path, passage_guid, play_order, fade curves | 1-100 |
| `songs` | MusicBrainz recordings | guid, recording_mbid, work_mbid, title, duration | 500-50,000 |
| `artists` | MusicBrainz artists | guid, artist_mbid, name | 100-10,000 |
| `albums` | MusicBrainz releases | guid, release_mbid, title | 50-5,000 |
| `works` | MusicBrainz works | guid, work_mbid, title | 500-50,000 |
| `passage_songs` | Many-to-many link | passage_guid, song_guid | 500-500,000 |
| `song_artists` | Many-to-many link | song_guid, artist_guid | 1000-100,000 |
| `passage_albums` | Many-to-many link | passage_guid, album_guid | 500-500,000 |
| `acoustid_cache` | Fingerprint cache | fingerprint_hash, recording_mbid | 100-100,000 |
| `schema_version` | Migration tracking | version | 1 |

**Schema Stability:** Medium - may evolve with WKMP features
**Mitigation:** Use wkmp-common database models where possible, add schema version check

---

### wkmp-ui (for Integration)

**Status:** ✅ Exists, actively developed
**Location:** `/home/sw/Dev/McRhythm/wkmp-ui/`

**Integration Point:**
- Add "Database Review" launch button in Tools menu or Library view
- Modification scope: ~10-20 lines (HTML + routing)
- Risk: Low (minimal UI change)

**Dependencies ON wkmp-dr:**
- wkmp-dr must be running on port 5725
- Health check optional (wkmp-ui could ping /health before opening)

**Risk:** Low
**Mitigation:** Document integration in wkmp-ui changes, test with wkmp-dr running/stopped

---

## 2. External Library Dependencies (Cargo Crates)

### Required Crates

| Crate | Version | Purpose | License | Status |
|-------|---------|---------|---------|--------|
| `tokio` | 1.40+ | Async runtime | MIT | ✅ Standard |
| `axum` | 0.7+ | HTTP framework | MIT | ✅ Standard |
| `tower` | 0.5+ | Middleware support | MIT | ✅ Standard |
| `tower-http` | 0.5+ | Static file serving, CORS | MIT | ✅ Standard |
| `sqlx` | 0.8+ | Database access (async) | MIT/Apache-2.0 | ✅ Standard |
| `serde` | 1.0+ | Serialization | MIT/Apache-2.0 | ✅ Standard |
| `serde_json` | 1.0+ | JSON handling | MIT/Apache-2.0 | ✅ Standard |
| `uuid` | 1.10+ | UUID handling | MIT/Apache-2.0 | ✅ Standard |
| `anyhow` | 1.0+ | Error handling | MIT/Apache-2.0 | ✅ Standard |
| `tracing` | 0.1+ | Structured logging | MIT | ✅ Standard |
| `tracing-subscriber` | 0.3+ | Logging backend | MIT | ✅ Standard |
| `sha2` | 0.10+ | SHA-256 hashing (auth) | MIT/Apache-2.0 | ✅ Standard |
| `chrono` | 0.4+ | Timestamp handling | MIT/Apache-2.0 | ✅ Standard |

**All crates:** Widely used, stable, compatible with WKMP stack

### Development/Test Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `reqwest` | 0.12+ | HTTP client for integration tests |
| `tempfile` | 3.13+ | Temporary test databases |
| `tokio-test` | 0.4+ | Async test utilities |

---

## 3. Runtime Dependencies

### Required at Startup

1. **SQLite Database File:** `<root_folder>/wkmp.db`
   - **Status:** Created by wkmp-common::db::init if missing
   - **Risk:** Low - auto-initialized
   - **Fallback:** Module creates empty database if missing (read-only mode may fail)

2. **Shared Secret:** `settings` table key `shared_secret`
   - **Status:** Created by first WKMP module to start
   - **Risk:** Low - standard WKMP initialization
   - **Fallback:** wkmp-dr can generate if missing (though should already exist)

3. **Port 5725 Available:**
   - **Risk:** Medium - conflict with other software possible
   - **Mitigation:** Log clear error if port unavailable, document resolution
   - **Fallback:** Make port configurable via TOML (REQ-DR-NF-050)

4. **Root Folder Writable** (for log files, TOML config)
   - **Risk:** Low - user's home directory typically writable
   - **Mitigation:** Check on startup, log clear error if read-only

### Optional at Runtime

1. **wkmp-ui Running:** (for launch button to work)
   - **Not Required:** wkmp-dr runs independently
   - **User can:** Access http://localhost:5725 directly

2. **Internet Connection:** (if fetching external documentation)
   - **Not Required:** Fully functional offline

---

## 4. Development Dependencies

### Build Environment

**Required:**
- Rust toolchain 1.75+ (stable channel)
- Cargo build system
- Linux or macOS development environment
- Git version control

**Optional:**
- cargo-watch (live rebuild)
- cargo-tarpaulin (code coverage)
- rust-analyzer (IDE support)

### Test Environment

**Required:**
- Empty test database (created in `tempfile`)
- Test fixtures (sample data for queries)

**Optional:**
- Integration test with full wkmp.db (if available)

---

## 5. Integration Points

### wkmp-ui Integration (P1 Priority)

**Type:** UI Integration
**Scope:** Add launch button
**Files Modified:** `wkmp-ui/src/api/ui.rs` or similar
**Lines Changed:** ~10-20 lines

**Implementation:**
```html
<button onclick="window.open('http://localhost:5725', '_blank')">
  Database Review
</button>
```

**Testing:**
- Manual: Click button, verify wkmp-dr opens
- Automated: Check button exists (UI integration test)

**Risk:** Low
**Rollback:** Remove button if wkmp-dr not available

---

### Shared Database (Implicit Integration)

**Type:** Data Sharing
**Scope:** Read-only access to wkmp.db
**Concurrency:** SQLite read-only mode allows multiple readers

**Coordination:**
- NO explicit coordination required (read-only)
- wkmp-dr never writes to database
- Database locked by other modules does not affect wkmp-dr (reads allowed)

**Risk:** Very Low
**Mitigation:** Read-only connection enforced

---

### Build/Packaging Integration

**Type:** Build System
**Scope:** Add wkmp-dr to Full version packaging

**Files Modified:**
- `Cargo.toml` workspace members (add wkmp-dr)
- `scripts/package-full.sh` (include wkmp-dr binary)
- `README.md` (document wkmp-dr in Full version features)

**Testing:**
- Build Full version, verify wkmp-dr binary present
- Start Full version, verify all 6 modules start

**Risk:** Low
**Rollback:** Exclude wkmp-dr from package if blocking issues

---

## 6. Dependency Risk Assessment

### Critical Path Dependencies (Failure Blocks Module)

| Dependency | Failure Impact | Probability | Mitigation |
|------------|----------------|-------------|------------|
| wkmp-common config utilities | Module won't start | Very Low | Well-tested, used by all modules |
| wkmp.db file | No data to display | Low | Auto-created if missing |
| Port 5725 available | Module won't bind | Low | Log error, document resolution |
| Shared secret in settings | Auth fails | Very Low | Generate if missing |

**Overall Critical Path Risk:** Low

### Optional Dependencies (Failure Degrades Functionality)

| Dependency | Failure Impact | Probability | Mitigation |
|------------|----------------|-------------|------------|
| wkmp-ui running | Launch button doesn't work | Medium | User can access directly via URL |
| Test data available | Can't run full integration tests | Low | Generate synthetic test data |

**Overall Optional Dependency Risk:** Very Low

---

## 7. Dependency Graph

```
wkmp-dr
├── wkmp-common (library)
│   ├── tokio
│   ├── sqlx
│   ├── serde/serde_json
│   ├── uuid
│   └── sha2
├── axum (HTTP framework)
│   ├── tokio
│   ├── tower
│   └── hyper
├── tower-http (static files)
├── tracing (logging)
└── chrono (timestamps)

Runtime:
├── wkmp.db (SQLite database file)
├── settings table (shared secret)
└── Port 5725 (network)

Integration:
├── wkmp-ui (launch button) [optional]
└── Full version packaging scripts
```

---

## 8. Dependency Verification Checklist

**Before Implementation:**
- [ ] wkmp-common utilities exist (checked: ✅ yes)
- [ ] Database schema documented (checked: ✅ yes)
- [ ] Port 5725 available (check on dev machine)
- [ ] Cargo.toml dependencies resolve (check after setup)

**During Implementation:**
- [ ] All wkmp-common APIs work as expected
- [ ] Database queries return expected data
- [ ] Read-only mode prevents writes
- [ ] Authentication validates correctly

**Before Release:**
- [ ] wkmp-ui integration tested
- [ ] Full version packaging works
- [ ] All dependencies have acceptable licenses (checked: ✅ all MIT/Apache-2.0)
- [ ] No dependency version conflicts with existing WKMP modules

---

## 9. Dependency Updates Strategy

**Policy:** Use same dependency versions as existing WKMP modules for consistency.

**Update Frequency:**
- Security updates: Immediately
- Bug fixes: Next release cycle
- Feature updates: Evaluate case-by-case

**Compatibility:**
- MUST maintain compatibility with wkmp-common
- SHOULD match versions used by wkmp-ai, wkmp-le

---

**Dependencies Map Complete**
**Status:** Phase 1 complete, ready for Phase 2 (Specification Verification)
