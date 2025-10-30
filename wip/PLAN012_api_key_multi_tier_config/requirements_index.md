# PLAN012 - Requirements Index

**Source Specification:** wip/SPEC025-api_key_configuration.md
**Plan Version:** 1.0
**Date:** 2025-10-30

---

## Requirements Summary

**Total Requirements:** 62

| Category | Count | ID Range |
|----------|-------|----------|
| Scope | 3 | APIK-SC-010 to APIK-SC-030 |
| Overview | 3 | APIK-OV-010 to APIK-OV-030 |
| Priority Resolution | 5 | APIK-RES-010 to APIK-RES-050 |
| Write-Back Behavior | 4 | APIK-WB-010 to APIK-WB-040 |
| TOML Persistence | 5 | APIK-TOML-010 to APIK-TOML-050 |
| Validation | 2 | APIK-VAL-010 to APIK-VAL-020 |
| Error Handling | 3 | APIK-ERR-010 to APIK-ERR-030 |
| Architecture | 3 | APIK-ARCH-010 to APIK-ARCH-030 |
| Generic Settings Sync | 3 | APIK-SYNC-010 to APIK-SYNC-030 |
| AcoustID Specific | 4 | APIK-ACID-010 to APIK-ACID-040 |
| TOML Schema | 2 | APIK-TOML-SCHEMA-010 to APIK-TOML-SCHEMA-020 |
| Database Storage | 3 | APIK-DB-010 to APIK-DB-030 |
| Atomic TOML Write | 2 | APIK-ATOMIC-010 to APIK-ATOMIC-020 |
| Security | 9 | APIK-SEC-010 to APIK-SEC-090 |
| Web UI Integration | 6 | APIK-UI-010 to APIK-UI-060 |
| Logging and Observability | 4 | APIK-LOG-010 to APIK-LOG-040 |
| Testing Requirements | 3 | APIK-TEST-010 to APIK-TEST-030 |
| Migration | 3 | APIK-MIG-010 to APIK-MIG-030 |
| Future Extensions | 5 | APIK-FUT-010 to APIK-FUT-050 |

---

## Requirements by Category

### Scope (APIK-SC)

**[APIK-SC-010]** This specification applies to all WKMP modules that require API keys or similar secret configuration values.

**[APIK-SC-020]** Initial implementation targets wkmp-ai (Audio Ingest) for AcoustID API key configuration.

**[APIK-SC-030]** Design is extensible to future API keys (MusicBrainz tokens, Spotify credentials, etc.).

### Overview (APIK-OV)

**[APIK-OV-010]** API keys and similar secrets require special configuration handling that balances security, usability, and deployment flexibility.

**[APIK-OV-020]** This specification defines a 3-tier resolution system with automatic write-back to TOML for durable configuration persistence across database deletions.

**[APIK-OV-030]** The system follows WKMP's established configuration patterns (database-first) while extending them to handle secrets appropriately.

### Priority Resolution (APIK-RES)

**[APIK-RES-010]** The system SHALL resolve API keys using 3-tier priority:
1. Database (authoritative) - settings table
2. Environment variable (fallback) - WKMP_{SERVICE}_{KEY}
3. TOML config file (fallback) - ~/.config/wkmp/{module}.toml

**[APIK-RES-020]** When database contains valid key, environment variable and TOML SHALL be ignored (database is authoritative).

**[APIK-RES-030]** When database does not contain valid key, environment variable SHALL be checked next.

**[APIK-RES-040]** When database and environment variable do not contain valid key, TOML config file SHALL be checked.

**[APIK-RES-050]** When no valid key found in any source, system SHALL fail with clear error message directing user to configuration methods.

### Write-Back Behavior (APIK-WB)

**[APIK-WB-010]** When environment variable provides key and database is empty, system SHALL:
1. Write key to database (authoritative storage)
2. Write key to TOML file (durable backup) using best-effort approach
3. Log migration completion

**[APIK-WB-020]** When TOML provides key and database is empty, system SHALL:
1. Write key to database (authoritative storage)
2. TOML already contains key (no write-back needed)
3. Log migration completion

**[APIK-WB-030]** When web UI or other system updates key in database, system SHALL:
1. Write key to database (authoritative storage)
2. Write key to TOML file (durable backup) using best-effort approach
3. Log update completion

**[APIK-WB-040]** TOML write operations SHALL use best-effort approach:
- If write succeeds: Log success
- If write fails (read-only filesystem, permissions): Warn but continue (database write succeeded)
- Never fail operation due to TOML write failure

### TOML Persistence (APIK-TOML)

**[APIK-TOML-010]** TOML file SHALL serve as durable configuration backup that survives database deletion.

**[APIK-TOML-020]** Primary use case: Development workflow where database is frequently deleted for testing.

**[APIK-TOML-030]** TOML write SHALL preserve all existing fields (root_folder, logging, static_assets, etc.).

**[APIK-TOML-040]** TOML write SHALL use atomic file operations (temp file + rename) to prevent corruption.

**[APIK-TOML-050]** TOML file permissions SHALL be set to 0600 (rw-------) on Unix systems for security.

### Validation (APIK-VAL)

**[APIK-VAL-010]** API key validation SHALL check:
- Key is not NULL
- Key is not empty string
- Key does not contain only whitespace

**[APIK-VAL-020]** Additional format validation (if applicable) SHALL be performed by consuming module (e.g., AcoustID client validates key format).

### Error Handling (APIK-ERR)

**[APIK-ERR-010]** When no valid key found in any source, system SHALL provide error message with:
- List of all 3 configuration methods (database, ENV, TOML)
- Exact environment variable name expected
- Exact TOML file path and key name
- Link to obtain API key (if applicable)

**[APIK-ERR-020]** TOML write failures SHALL be logged as warnings, not errors (database write is primary).

**[APIK-ERR-030]** Database write failures SHALL fail entire operation (database is authoritative).

### Architecture (APIK-ARCH)

**[APIK-ARCH-010]** Implementation SHALL consist of:
1. Resolver function - Multi-tier resolution with auto-migration
2. Database accessors - Get/set API key in settings table
3. TOML utilities - Read/write TOML with field preservation
4. Generic sync mechanism - Extensible to multiple API keys (Option B)

**[APIK-ARCH-020]** wkmp-common SHALL provide:
- TOML schema extension (TomlConfig struct)
- Generic TOML read/write utilities
- Atomic file write functions

**[APIK-ARCH-030]** Module-specific (e.g., wkmp-ai) SHALL provide:
- API key resolver function (multi-tier resolution)
- Database accessor functions (get/set API key)
- Settings sync function (database ↔ TOML synchronization)

### Generic Settings Sync (APIK-SYNC)

**[APIK-SYNC-010]** Settings sync function SHALL support multiple configuration keys via HashMap-based interface:

```rust
sync_settings_to_toml(
    resolver: &RootFolderResolver,
    changed_settings: &HashMap<String, String>
) -> Result<()>
```

**[APIK-SYNC-020]** Settings mapping SHALL be maintained in module configuration:

```rust
match key.as_str() {
    "acoustid_api_key" => config.acoustid_api_key = Some(value.clone()),
    "musicbrainz_token" => config.musicbrainz_token = Some(value.clone()),
    // Future keys here
    _ => {}  // Ignore unknown keys
}
```

**[APIK-SYNC-030]** New API keys SHALL be added by:
1. Extending TomlConfig struct in wkmp-common
2. Adding key mapping in module's sync function
3. Creating resolver function for that key

### AcoustID Specific (APIK-ACID)

**[APIK-ACID-010]** AcoustID API key SHALL be resolved in wkmp-ai module using function:

```rust
resolve_acoustid_api_key(
    db: &sqlx::SqlitePool,
    resolver: &wkmp_common::config::RootFolderResolver,
) -> Result<String>
```

**[APIK-ACID-020]** Environment variable name SHALL be: WKMP_ACOUSTID_API_KEY

**[APIK-ACID-030]** TOML key name SHALL be: acoustid_api_key

**[APIK-ACID-040]** Database settings key SHALL be: acoustid_api_key

### TOML Schema (APIK-TOML-SCHEMA)

**[APIK-TOML-SCHEMA-010]** TomlConfig struct SHALL be extended:

```rust
#[derive(Debug, Deserialize, Serialize)]
pub struct TomlConfig {
    pub root_folder: Option<PathBuf>,
    #[serde(default)]
    pub logging: LoggingConfig,
    pub static_assets: Option<PathBuf>,

    /// AcoustID API key (optional)
    /// Used by: wkmp-ai (Audio Ingest) only
    pub acoustid_api_key: Option<String>,

    /// Future: MusicBrainz token, Spotify credentials, etc.
}
```

**[APIK-TOML-SCHEMA-020]** TOML schema SHALL be backward-compatible (all new fields optional).

### Database Storage (APIK-DB)

**[APIK-DB-010]** API keys SHALL be stored in settings table using existing key-value pattern.

**[APIK-DB-020]** Database accessors SHALL use generic get_setting<String>() and set_setting() functions.

**[APIK-DB-030]** No database schema changes required (uses existing settings table).

### Atomic TOML Write (APIK-ATOMIC)

**[APIK-ATOMIC-010]** TOML write SHALL use atomic file operations:
1. Serialize TomlConfig to string
2. Write to temporary file (.toml.tmp)
3. Set file permissions to 0600 (Unix)
4. Atomic rename to target path

**[APIK-ATOMIC-020]** Atomic write SHALL prevent:
- Partial writes (crash during write)
- Corruption of existing TOML
- Race conditions (multiple writers)

### Security (APIK-SEC)

**[APIK-SEC-010]** TOML files containing API keys SHALL have permissions set to 0600 (rw-------) on Unix systems.

**[APIK-SEC-020]** Permission setting SHALL occur automatically during atomic write operation.

**[APIK-SEC-030]** On Windows systems, file permissions SHALL rely on NTFS ACLs (default user-only access).

**[APIK-SEC-040]** On Unix systems, resolver function SHALL check TOML file permissions if file exists.

**[APIK-SEC-050]** If TOML file is readable by group or others (mode & 0o077 != 0), system SHALL log warning:

```
WARNING: TOML config file {path} has loose permissions (readable by others).
Recommend: chmod 600 {path}
```

**[APIK-SEC-060]** Warning SHALL be informational only (does not block operation).

**[APIK-SEC-070]** Environment variables are inherently less secure than file permissions (visible in process list, inherited by child processes).

**[APIK-SEC-080]** Documentation SHALL warn users that environment variables may be visible to other processes on shared systems.

**[APIK-SEC-090]** Auto-migration from ENV to TOML reduces exposure (ENV can be unset after first run).

### Web UI Integration (APIK-UI)

**[APIK-UI-010]** wkmp-ai SHALL provide HTTP endpoint:

```
POST /api/settings/acoustid_api_key
Content-Type: application/json

{"api_key": "string"}
```

**[APIK-UI-020]** Endpoint SHALL:
1. Validate API key not empty
2. Write to database using set_acoustid_api_key()
3. Write to TOML using sync_settings_to_toml() (best effort)
4. Return success/error response

**[APIK-UI-030]** Response format SHALL be:

```json
{
  "message": "API key saved successfully"
}
```

**[APIK-UI-040]** wkmp-ai web UI SHALL provide settings page at /settings.

**[APIK-UI-050]** Settings page SHALL include:
- Input field for API key
- Save button
- Link to obtain free API key (https://acoustid.org/api-key)
- Success/error message display

**[APIK-UI-060]** Current API key SHALL NOT be displayed (security - show only "Key configured" status).

### Logging and Observability (APIK-LOG)

**[APIK-LOG-010]** At module startup, system SHALL log API key source:

```
INFO: AcoustID API key: Loaded from database
INFO: AcoustID API key: Loaded from environment variable
INFO: AcoustID API key: Loaded from TOML config
```

**[APIK-LOG-020]** When database has key but ENV/TOML also present, system SHALL log warning:

```
WARN: AcoustID API key found in multiple sources. Using database value.
      To use environment variable or TOML, delete key from database first.
```

**[APIK-LOG-030]** When auto-migration occurs, system SHALL log:

```
INFO: AcoustID API key: Loaded from environment variable
INFO: Migrating API key to database for persistence...
INFO: API key saved to database
INFO: API key backed up to TOML config
```

**[APIK-LOG-040]** When TOML write fails, system SHALL log warning:

```
WARN: Could not back up API key to TOML config: {error}.
      TOML may be read-only. Key saved to database successfully.
```

### Testing Requirements (APIK-TEST)

**[APIK-TEST-010]** Unit tests SHALL verify:
- Database-first priority (database key used when present)
- ENV fallback (ENV key used when database empty)
- TOML fallback (TOML key used when database and ENV empty)
- Error when all sources empty
- Database write-back from ENV
- TOML write-back from ENV
- TOML write-back from UI update
- TOML field preservation (root_folder, logging preserved)
- Atomic TOML write (crash safety)
- Permission setting (Unix)

**[APIK-TEST-020]** Integration tests SHALL verify:
- End-to-end resolution in wkmp-ai startup
- Web UI endpoint functionality
- Database deletion recovery (TOML restores key)
- Multiple module startup (concurrent TOML reads safe)

**[APIK-TEST-030]** Manual tests SHALL verify:
- ENV → Database + TOML migration works
- TOML → Database migration works
- Web UI save works
- Database deletion → TOML restore works
- Read-only TOML graceful degradation works
- Permission warnings appear on loose permissions

### Migration (APIK-MIG)

**[APIK-MIG-010]** Existing deployments using environment variable SHALL:
- Continue working (ENV still checked as fallback)
- Auto-migrate to database + TOML on first run after upgrade
- Log migration completion

**[APIK-MIG-020]** Existing deployments using hardcoded key SHALL:
- Require manual migration to one of three sources (database/ENV/TOML)
- Receive clear error message if key not configured

**[APIK-MIG-030]** This specification introduces no breaking changes to:
- Database schema (uses existing settings table)
- TOML schema (backward compatible, new fields optional)
- Module startup sequence (same initialization flow)
- API interfaces (internal implementation only)

### Future Extensions (APIK-FUT)

**[APIK-FUT-010]** Future API keys SHALL follow same pattern:
1. Add field to TomlConfig struct
2. Add mapping in sync_settings_to_toml() function
3. Create resolver function for that key
4. Add web UI endpoint if needed

**[APIK-FUT-020]** Examples of future API keys:
- musicbrainz_token (rate limit increase)
- spotify_client_id + spotify_client_secret
- lastfm_api_key
- discogs_token

**[APIK-FUT-030]** Future enhancement: Sync all changed settings to TOML in single operation (reduces TOML writes).

**[APIK-FUT-040]** Future enhancement: Optional encryption for API keys in database using SQLCipher.

**[APIK-FUT-050]** Current design: Plain text acceptable for read-only API keys with low sensitivity.

---

## Acceptance Criteria (21 items)

1. Multi-tier resolution works (Database → ENV → TOML → Error)
2. Database is authoritative (ignores ENV/TOML when database has key)
3. Auto-migration works (ENV → Database + TOML)
4. Auto-migration works (TOML → Database)
5. TOML write-back works (ENV or UI update → Database + TOML)
6. TOML write is atomic (temp + rename)
7. TOML write preserves other fields
8. TOML permissions set to 0600 (Unix)
9. TOML write failures are graceful (warn, don't fail)
10. Generic settings sync supports multiple keys
11. Web UI endpoint works (POST /api/settings/acoustid_api_key)
12. Web UI settings page works
13. All unit tests pass
14. All integration tests pass
15. Manual testing complete (ENV, TOML, UI, database deletion)
16. Documentation updated (IMPL012, IMPL001 reference)
17. Logging provides clear observability
18. Error messages list all 3 configuration methods
19. Security warnings for loose permissions work
20. Backward compatibility maintained
21. Extensibility to future API keys demonstrated

---

**Requirements Extraction:** Complete
**Next Step:** Create scope_statement.md
