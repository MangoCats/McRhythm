# Increment 10: Manual Testing and Documentation

**Estimated Effort:** 3-4 hours
**Dependencies:** Increments 1-9 (all implementation)
**Risk:** LOW

---

## Objectives

Complete manual testing scenarios and update documentation.

---

## Requirements Addressed

- [APIK-TEST-030] - Manual tests
- [APIK-MIG-010], [APIK-MIG-020] - Migration documentation
- [APIK-SEC-070], [APIK-SEC-080] - Security warnings documentation

---

## Deliverables

### Manual Tests

**File: Manual test execution** (tc_m_migration_001-003, tc_m_failure_001-003)

```
tc_m_migration_001: ENV → Database + TOML migration
- Fresh install (no database, no TOML)
- Set WKMP_ACOUSTID_API_KEY=test-key
- Start wkmp-ai
- Verify: Database contains key
- Verify: TOML contains key
- Verify: Log shows migration from environment

tc_m_migration_002: TOML → Database migration
- Fresh install (no database)
- Create ~/.config/wkmp/wkmp-ai.toml with acoustid_api_key
- Start wkmp-ai
- Verify: Database contains key
- Verify: TOML unchanged
- Verify: Log shows migration from TOML

tc_m_migration_003: Web UI save and verification
- Open http://localhost:5723/settings
- Enter API key
- Click Save
- Verify: Success message displayed
- Verify: Database contains key (check with sqlite3)
- Verify: TOML contains key (check file)

tc_m_failure_001: Read-only TOML filesystem
- Set filesystem read-only (mount -o ro or Windows read-only attribute)
- Configure key via ENV or UI
- Start wkmp-ai
- Verify: Database write succeeds
- Verify: TOML write warning logged
- Verify: Startup continues (not blocked)

tc_m_failure_002: Permission warnings on loose permissions
- Create TOML with 0644 permissions (world-readable)
- Add acoustid_api_key to TOML
- Start wkmp-ai
- Verify: Warning logged about loose permissions
- Verify: Startup continues (not blocked)

tc_m_failure_003: Invalid key error messages
- Fresh install (no key configured)
- Start wkmp-ai
- Verify: Error message lists all 3 configuration methods
- Verify: Error includes link to acoustid.org
- Verify: Error is clear and actionable
```

---

### Documentation Updates

**File: docs/IMPL012-acoustid_client.md** (update)

Add section:

```markdown
## Configuration

AcoustID API key is configured using multi-tier resolution:

1. **Database** (highest priority) - Configured via web UI or auto-migrated
2. **Environment Variable** - `WKMP_ACOUSTID_API_KEY=your-key-here`
3. **TOML Config** - `~/.config/wkmp/wkmp-ai.toml` field `acoustid_api_key`

### First-Time Setup

**Option 1: Web UI (Recommended)**
1. Start wkmp-ai: `cargo run -p wkmp-ai`
2. Open http://localhost:5723/settings
3. Enter API key (obtain from https://acoustid.org/api-key)
4. Click Save

**Option 2: Environment Variable**
```bash
export WKMP_ACOUSTID_API_KEY=your-key-here
cargo run -p wkmp-ai
```

Key is automatically migrated to database and TOML for persistence.

**Option 3: TOML Config**
Edit `~/.config/wkmp/wkmp-ai.toml`:
```toml
acoustid_api_key = "your-key-here"
```

Key is automatically migrated to database on first startup.

### Security Notes

- TOML file permissions are automatically set to 0600 (Unix) for security
- Environment variables are visible to all processes (less secure)
- Database and TOML store keys in plain text (acceptable for read-only API keys)
- Web UI does not display existing key (security)

### Migration Behavior

- ENV → Database + TOML (auto-migration on startup)
- TOML → Database (auto-migration on startup, TOML unchanged)
- UI update → Database + TOML (write-back)
- Database is authoritative (ENV/TOML ignored when database has key)

### Database Recovery

If database is deleted:
1. Key recovered from TOML (if exists)
2. Key automatically migrated back to database
3. Normal operation resumes

If both database and TOML deleted:
- wkmp-ai fails to start with clear error message
- Reconfigure using one of the 3 methods above
```

**File: docs/IMPL001-database_schema.md** (update)

Add to settings table documentation:

```markdown
### Settings Table

Key-value store for system-wide settings.

**API Key Storage:**
- `acoustid_api_key` - AcoustID API key (plain text, acceptable for read-only keys)
- Future: `musicbrainz_token`, etc.

**Multi-Tier Configuration:**
API keys use 3-tier resolution (Database → ENV → TOML) with auto-migration.
See SPEC025-api_key_configuration.md for details.
```

---

## Acceptance Criteria

- [ ] All 6 manual tests executed and documented
- [ ] IMPL012-acoustid_client.md updated (configuration section)
- [ ] IMPL001-database_schema.md updated (settings table reference)
- [ ] User-facing documentation clear and complete
- [ ] Security warnings documented
- [ ] Migration behavior explained

---

## Test Traceability

- tc_m_migration_001-003: Migration scenarios
- tc_m_failure_001-003: Failure scenarios

---

## Rollback Plan

Revert documentation changes. No impact on implementation.
