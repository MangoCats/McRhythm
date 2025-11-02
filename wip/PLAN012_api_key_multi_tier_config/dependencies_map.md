# PLAN012 - Dependencies Map

**Specification:** SPEC025-api_key_configuration.md
**Plan Version:** 1.0
**Date:** 2025-10-30

---

## Dependency Graph

```
PLAN012 Implementation
│
├─ wkmp-common (Shared Library)
│  │
│  ├─ config.rs (MODIFY)
│  │  ├─ TomlConfig struct (ADD acoustid_api_key field)
│  │  ├─ read_toml_config() (EXISTING - may need updates)
│  │  └─ write_toml_config() (NEW - atomic write with field preservation)
│  │
│  └─ toml_utils.rs (NEW or integrate into config.rs)
│     ├─ atomic_write_toml() (NEW)
│     ├─ set_unix_permissions_0600() (NEW)
│     └─ preserve_toml_fields() (NEW)
│
├─ wkmp-ai (Audio Ingest Module)
│  │
│  ├─ config.rs (NEW or EXTEND)
│  │  ├─ resolve_acoustid_api_key() (NEW - multi-tier resolution)
│  │  └─ sync_settings_to_toml() (NEW - generic HashMap-based)
│  │
│  ├─ db/settings.rs (NEW or EXTEND)
│  │  ├─ get_acoustid_api_key() (NEW)
│  │  └─ set_acoustid_api_key() (NEW)
│  │
│  ├─ api/handlers.rs (EXTEND)
│  │  └─ post_acoustid_api_key() (NEW endpoint)
│  │
│  ├─ main.rs (MODIFY)
│  │  └─ Integrate resolve_acoustid_api_key() at startup
│  │
│  └─ static/ (NEW files)
│     ├─ settings.html (NEW - settings page UI)
│     ├─ settings.css (NEW - styling)
│     └─ settings.js (NEW - API interaction)
│
├─ Testing
│  │
│  ├─ wkmp-common/tests/ (NEW tests)
│  │  ├─ toml_utils_tests.rs (NEW)
│  │  └─ config_tests.rs (EXTEND)
│  │
│  └─ wkmp-ai/tests/ (NEW tests)
│     ├─ unit/
│     │  ├─ config_tests.rs (NEW)
│     │  └─ db_settings_tests.rs (NEW)
│     │
│     ├─ integration/
│     │  └─ api_key_resolution_tests.rs (NEW)
│     │
│     └─ manual/
│        └─ manual_test_checklist.md (NEW)
│
└─ Documentation
   ├─ IMPL012-acoustid_client.md (UPDATE - reference multi-tier config)
   ├─ IMPL001-database_schema.md (UPDATE - document acoustid_api_key usage)
   └─ User Guide (NEW section - API key configuration)
```

---

## External Dependencies

### Crates (Existing - No Changes Required)

| Crate | Version | Purpose | Status |
|-------|---------|---------|--------|
| toml | 0.8.x | TOML parsing/serialization | EXISTING |
| serde | 1.0.x | Serialization framework | EXISTING |
| sqlx | 0.8.x | Database access | EXISTING |
| tokio | 1.x | Async runtime | EXISTING |
| axum | 0.7.x | HTTP server | EXISTING |

**No new external dependencies required.**

### System Dependencies

| Dependency | Platform | Purpose | Availability |
|------------|----------|---------|--------------|
| File permissions API | Unix | chmod 0600 for TOML | Standard library |
| NTFS ACLs | Windows | Default user-only access | OS-provided |
| Environment variables | All | ENV fallback resolution | Standard library |
| Atomic file rename | All | TOML write safety | Standard library (std::fs::rename) |

---

## Internal Code Dependencies

### wkmp-common/src/config.rs (MODIFY)

**Current state:**
- Provides TomlConfig struct with root_folder, logging, static_assets fields
- Provides RootFolderResolver for 4-tier resolution
- Used by all modules for root folder configuration

**Required changes:**
- ADD: acoustid_api_key field to TomlConfig struct
- ADD: write_toml_config() function (atomic write)
- PRESERVE: All existing functionality (backward compatible)

**Risk:** LOW (additive changes only, existing fields unchanged)

### wkmp-ai/src/db/ (NEW or EXTEND)

**Current state:**
- Unknown if settings.rs exists (may need creation)
- Database initialization uses sqlx with migrations

**Required changes:**
- CREATE or EXTEND: settings.rs module
- ADD: get_acoustid_api_key() function
- ADD: set_acoustid_api_key() function
- PATTERN: Follow existing settings accessor pattern from wkmp-ap

**Risk:** LOW (pattern established in wkmp-ap/src/db/settings.rs)

**Dependency on wkmp-ap pattern:**
- Check wkmp-ap/src/db/settings.rs for reference implementation
- Replicate pattern in wkmp-ai

### wkmp-ai/src/api/ (EXTEND)

**Current state:**
- HTTP server exists (Axum-based)
- Endpoints likely exist for import wizard functionality

**Required changes:**
- ADD: POST /api/settings/acoustid_api_key endpoint
- ADD: Request/response types
- INTEGRATE: Call set_acoustid_api_key() and sync_settings_to_toml()

**Risk:** LOW (standard HTTP endpoint pattern)

### wkmp-ai/static/ (NEW)

**Current state:**
- Static assets directory exists for import wizard UI

**Required changes:**
- ADD: settings.html (new page)
- ADD: settings.css (styling)
- ADD: settings.js (API calls)

**Risk:** LOW (isolated web UI, no complex interactions)

---

## Database Dependencies

### Settings Table (EXISTING - No Schema Changes)

**Current schema (IMPL001-database_schema.md):**

```sql
CREATE TABLE settings (
    key TEXT PRIMARY KEY,
    value TEXT
);
```

**Usage for API keys:**
- key: "acoustid_api_key"
- value: API key string (plain text)

**No migrations required** - Uses existing table structure.

**Risk:** NONE (existing table, existing accessor pattern)

---

## Testing Dependencies

### Test Infrastructure (EXISTING)

**Current state:**
- cargo test framework configured
- Integration tests pattern exists in wkmp-ai/tests/
- Unit tests embedded in modules (tests/ submodules)

**Required:**
- No changes to test infrastructure
- Follow existing patterns for new tests

**Risk:** NONE (standard Rust testing)

### Test Data Requirements

**For unit tests:**
- Temporary databases (in-memory SQLite or temp files)
- Temporary TOML files (temp directory)
- Environment variable mocking (env::set_var in tests)

**For integration tests:**
- Test database (created/destroyed per test)
- Test TOML config (temp directory)
- Test HTTP server (Axum test utilities)

**For manual tests:**
- User-provided AcoustID API key (obtain from acoustid.org)
- Test database (can be real database, deleted for testing)
- Test TOML config (real file in ~/.config/wkmp/)

**Risk:** LOW (standard testing patterns)

---

## Documentation Dependencies

### IMPL012-acoustid_client.md (UPDATE)

**Current state:**
- Documents AcoustID client implementation
- May reference hardcoded or ENV-only key loading

**Required changes:**
- UPDATE: Configuration section to reference multi-tier resolution
- ADD: Link to SPEC025 for configuration details
- REMOVE or DEPRECATE: Hardcoded key references

**Location:** docs/IMPL012-acoustid_client.md (verify path)

**Risk:** LOW (documentation update, no code changes)

### IMPL001-database_schema.md (UPDATE)

**Current state:**
- Documents database schema including settings table

**Required changes:**
- UPDATE: Settings table usage section
- ADD: Document acoustid_api_key setting
- ADD: Reference to SPEC025 for multi-tier resolution

**Location:** docs/IMPL001-database_schema.md

**Risk:** LOW (documentation update)

### User Guide (NEW SECTION)

**Required:**
- NEW: Section on API key configuration
- CONTENT: How to configure AcoustID key (3 methods)
- CONTENT: Security warnings (ENV visibility, file permissions)
- CONTENT: Troubleshooting (key not found errors)

**Location:** TBD (docs/ or user-facing documentation)

**Risk:** LOW (new documentation)

---

## Dependency Risks

### High-Risk Dependencies

**None identified.** All dependencies are:
- Existing code (low risk)
- Standard library features (no external dep risk)
- Well-established patterns (settings accessor, HTTP endpoint)

### Medium-Risk Dependencies

**TOML field preservation:**
- RISK: TOML crate may not preserve field order or comments
- MITIGATION: Test round-trip serialization, accept best-effort behavior
- IMPACT: Minor (comments lost, but fields preserved via struct)

**Atomic file rename on Windows:**
- RISK: std::fs::rename not always atomic on Windows
- MITIGATION: Best-effort approach, document limitation
- IMPACT: Low (corruption unlikely, recoverable from database)

### Low-Risk Dependencies

**All other dependencies:** Existing code, standard patterns, no breaking changes.

---

## Dependency Verification Checklist

Before starting implementation, verify:

- [ ] wkmp-common/src/config.rs exists with TomlConfig struct
- [ ] wkmp-ai uses sqlx for database access
- [ ] wkmp-ai has HTTP server (Axum) configured
- [ ] Settings table exists in database schema
- [ ] wkmp-ap/src/db/settings.rs exists (reference pattern)
- [ ] toml crate version compatible with serde
- [ ] Test infrastructure supports temp files and in-memory SQLite

**All items verified as existing or standard (no blockers identified).**

---

## Implementation Order (Based on Dependencies)

**Phase 1: Foundation (wkmp-common)**
1. Extend TomlConfig struct (acoustid_api_key field)
2. Implement atomic_write_toml() utility
3. Implement set_unix_permissions_0600() utility
4. Unit test TOML utilities

**Phase 2: wkmp-ai Core**
5. Implement database accessors (get/set_acoustid_api_key)
6. Implement resolver function (multi-tier resolution)
7. Implement sync_settings_to_toml() function
8. Unit test resolver and accessors

**Phase 3: Integration**
9. Integrate resolver into wkmp-ai startup
10. Add logging for resolution and migration
11. Integration test startup resolution

**Phase 4: Web UI**
12. Implement POST /api/settings/acoustid_api_key endpoint
13. Create settings.html/css/js web UI
14. Integration test endpoint

**Phase 5: Testing and Documentation**
15. Manual testing (all 6 scenarios)
16. Update IMPL012, IMPL001 documentation
17. Create user guide section

**Dependencies respected:** Each phase depends only on prior phases.

---

**Dependencies Map:** Complete
**Next Step:** Phase 2 - Specification Completeness Verification
