# PLAN012 - Scope Statement

**Specification:** SPEC025-api_key_configuration.md
**Plan Version:** 1.0
**Date:** 2025-10-30

---

## In Scope

### Code Implementation

**wkmp-common extensions:**
- Extend TomlConfig struct with acoustid_api_key field
- Implement generic TOML read/write utilities with field preservation
- Implement atomic file write function (temp + rename)
- Implement Unix file permission setting (0600)

**wkmp-ai implementation:**
- API key resolver function (multi-tier: Database → ENV → TOML)
- Database accessor functions (get/set acoustid_api_key)
- Generic settings sync function (HashMap-based, extensible to multiple keys)
- Auto-migration logic (ENV/TOML → Database + TOML)
- HTTP endpoint: POST /api/settings/acoustid_api_key
- Web UI settings page (/settings) with API key input form

**Integration:**
- Integrate resolver into wkmp-ai startup sequence
- Replace existing hardcoded/ENV-only AcoustID key loading
- Add logging for resolution source and migration

### Testing

**Unit tests (wkmp-common):**
- TOML read/write with field preservation
- Atomic file write operations
- Unix permission setting
- Windows graceful degradation

**Unit tests (wkmp-ai):**
- Multi-tier resolution (Database first, ENV fallback, TOML fallback)
- Validation (empty, whitespace, NULL)
- Auto-migration from ENV
- Auto-migration from TOML
- Write-back on UI update
- Error handling (no key found)
- Best-effort TOML write (graceful failure)

**Integration tests:**
- End-to-end wkmp-ai startup with key resolution
- Web UI endpoint functionality
- Database deletion → TOML restore
- Concurrent TOML reads (multiple modules)

**Manual tests:**
- ENV → Database + TOML migration
- TOML → Database migration
- Web UI save functionality
- Database deletion recovery
- Read-only filesystem graceful degradation
- Permission warnings (loose permissions)

### Documentation

**Updates to existing docs:**
- IMPL012-acoustid_client.md: Reference multi-tier configuration
- IMPL001-database_schema.md: Document acoustid_api_key setting usage
- wkmp-common/src/config.rs: Add inline documentation for new functions

**New documentation:**
- User guide section: How to configure AcoustID API key (3 methods)
- Security documentation: File permissions, ENV visibility warnings

### Generic Settings Sync (Option B)

**Implementation scope:**
- HashMap-based sync interface supporting multiple keys
- Key mapping in wkmp-ai config
- Extensibility pattern for future API keys
- Example: acoustid_api_key as first implementation

**Design scope:**
- Document extension pattern for future keys
- Demonstrate extensibility (but implement only AcoustID)

---

## Out of Scope

### Other API Keys

**Not implementing in this plan:**
- MusicBrainz token
- Spotify credentials
- Last.fm API key
- Discogs token

**Rationale:** Design is extensible, but PLAN012 implements only AcoustID as proof-of-concept. Future API keys follow same pattern (low risk, well-defined).

### Advanced Features

**Not implementing:**
- Encrypted storage (SQLCipher integration) - Future enhancement [APIK-FUT-040]
- Bulk settings sync (single TOML write for multiple changes) - Future optimization [APIK-FUT-030]
- API key rotation/versioning - No current requirement
- Key expiration tracking - No current requirement

### UI Enhancements

**Not implementing:**
- Key obfuscation in UI (show masked value) - Security by obscurity, low value
- Key validation via test API call - Deferred to actual usage
- Settings page for other modules - Only wkmp-ai in scope

### Migration Tooling

**Not implementing:**
- Automated migration script for hardcoded keys - Manual process acceptable
- Configuration import/export - No current requirement
- Multi-user configuration sync - Single-user application

---

## Assumptions

**Environment:**
- Unix systems support file permissions (chmod 0600)
- Windows NTFS provides default user-only access
- TOML crate supports round-trip serialization preserving field order (best effort)

**Existing code:**
- wkmp-common/src/config.rs provides TomlConfig struct
- wkmp-ai has database initialization using sqlx
- Settings table exists in database schema (key-value pattern)
- wkmp-ai can be extended with web UI endpoints

**Dependencies:**
- toml crate (existing dependency)
- serde (existing dependency)
- sqlx (existing dependency)
- No new external dependencies required

**User behavior:**
- Users understand file system paths (for TOML location)
- Users can set environment variables (for ENV method)
- Users can access web UI (for UI method)
- Development workflow frequently deletes database (motivates TOML backup)

---

## Constraints

### Technical Constraints

**Platform compatibility:**
- Must work on Windows, Linux, macOS
- File permissions only enforced on Unix (Windows uses NTFS ACLs)
- Atomic file operations may behave differently on Windows (rename not always atomic)

**Backward compatibility:**
- TOML schema must remain backward compatible (new fields optional)
- Database schema unchanged (uses existing settings table)
- Module startup sequence unchanged (same initialization flow)
- No breaking changes to public APIs

**Performance:**
- TOML write should not block critical path (best-effort approach)
- File I/O should be minimized (write only on change)
- Atomic write creates temp file (disk space for 2x TOML size)

### Security Constraints

**Plain text storage:**
- API keys stored in plain text (database and TOML)
- Acceptable for read-only API keys with low sensitivity
- File permissions (0600) provide basic protection
- Not suitable for high-value secrets (OAuth tokens, passwords)

**Environment variable visibility:**
- ENV vars visible in process list on some systems
- Users must understand security implications
- Documentation warns about shared systems

### Functional Constraints

**No encryption:**
- Encrypted storage deferred to future (APIK-FUT-040)
- Current implementation assumes keys have low sensitivity
- Users requiring encryption must use external tools (encrypted filesystem, vault)

**Single-user:**
- Configuration is per-machine, not per-user
- Multi-user systems share same TOML file (~/.config/wkmp/wkmp-ai.toml)
- No per-user key isolation

**Best-effort TOML:**
- TOML write failures do not fail operation (database write is authoritative)
- Read-only filesystems gracefully degrade
- Users must understand TOML backup is convenience, not requirement

---

## Dependencies

### External Dependencies

**Crates (existing):**
- toml = "0.8" (or current version in Cargo.toml)
- serde = "1.0"
- sqlx = { version = "0.8", features = ["sqlite", "runtime-tokio"] }
- tokio (async runtime)

**No new external dependencies required.**

### Internal Dependencies

**wkmp-common:**
- config.rs: TomlConfig struct (extend)
- config.rs: RootFolderResolver (use)
- Utilities: Atomic file write (new)
- Utilities: TOML read/write (new)

**wkmp-ai:**
- Database initialization pattern (existing)
- Settings accessor pattern (existing or new)
- HTTP server framework (existing - Axum)
- Web UI serving (existing)

**Shared database:**
- migrations/ (no new migrations required)
- Settings table schema (existing)

### Code Dependencies

**Files to modify:**
- wkmp-common/src/config.rs (extend TomlConfig, add utilities)
- wkmp-ai/src/config.rs (new or extend - resolver functions)
- wkmp-ai/src/db/settings.rs (new or extend - accessor functions)
- wkmp-ai/src/api/handlers.rs (new or extend - endpoint)
- wkmp-ai/static/ (new HTML/CSS/JS for settings page)

**Files to create:**
- wkmp-common/src/toml_utils.rs (or integrate into config.rs)
- wkmp-ai tests/integration_api_key.rs (integration tests)

### Upstream Requirements

**Documentation:**
- IMPL012-acoustid_client.md must exist (referenced for updates)
- IMPL001-database_schema.md must exist (referenced for updates)

**Code:**
- Settings table must exist in database schema
- wkmp-ai must have HTTP server initialized
- wkmp-common must have TomlConfig struct

**All upstream dependencies verified as existing (low risk).**

---

## Success Criteria

Implementation complete when:

**Functional:**
- Multi-tier resolution works (Database → ENV → TOML → Error)
- Auto-migration works (ENV → Database + TOML, TOML → Database)
- Web UI endpoint saves key to database + TOML
- TOML write-back preserves existing fields
- Best-effort TOML write gracefully degrades

**Quality:**
- 100% unit test coverage (all requirements traced)
- All integration tests pass
- Manual testing complete (6 scenarios)
- Code review approved

**Documentation:**
- IMPL012, IMPL001 updated with configuration references
- User guide section written (3 configuration methods)
- Security warnings documented

**Acceptance:**
- All 21 acceptance criteria checkboxes verified
- Traceability matrix shows 100% coverage (every requirement → test)
- No regressions in existing wkmp-ai functionality

---

**Scope Definition:** Complete
**Next Step:** Create dependencies_map.md
