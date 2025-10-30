# Manual Tests: Migration Scenarios (tc_m_migration_001 to tc_m_migration_003)

**Test Category:** Manual Tests
**Component:** Migration workflows
**Requirements Covered:** APIK-WB-010, APIK-WB-020, APIK-WB-030, APIK-LOG-030

---

## tc_m_migration_001: ENV → Database + TOML Migration

**Requirement:** [APIK-WB-010], [APIK-LOG-030]

**Manual Steps:**
1. Delete existing database and TOML (clean slate)
2. Set environment variable:
   ```bash
   export WKMP_ACOUSTID_API_KEY="manual-env-test-key"
   ```
3. Start wkmp-ai
4. Observe logs during startup
5. Check database contents
6. Check TOML file contents
7. Stop wkmp-ai and unset ENV variable
8. Restart wkmp-ai
9. Verify still works (uses database key)

**Expected Observations:**
- Startup logs:
  ```
  INFO: AcoustID API key: Loaded from environment variable
  INFO: Migrating API key to database for persistence...
  INFO: API key saved to database
  INFO: API key backed up to TOML config
  ```
- Database query shows acoustid_api_key = "manual-env-test-key"
- TOML file contains:
  ```toml
  acoustid_api_key = "manual-env-test-key"
  ```
- Second startup (no ENV) logs: "INFO: AcoustID API key: Loaded from database"

**Pass Criteria:**
- All log messages appear as expected
- Database and TOML both contain key after first run
- Second run works without ENV variable

---

## tc_m_migration_002: TOML → Database Migration

**Requirement:** [APIK-WB-020]

**Manual Steps:**
1. Delete existing database (keep TOML)
2. Manually edit ~/.config/wkmp/wkmp-ai.toml:
   ```toml
   acoustid_api_key = "manual-toml-test-key"
   ```
3. Start wkmp-ai
4. Observe logs during startup
5. Check database contents
6. Check TOML file contents (should be unchanged)

**Expected Observations:**
- Startup logs:
  ```
  INFO: AcoustID API key: Loaded from TOML config
  INFO: Migrating API key to database for persistence...
  INFO: API key saved to database
  ```
- Database contains acoustid_api_key = "manual-toml-test-key"
- TOML file unchanged (same modification timestamp)
- No "backed up to TOML config" log (TOML already has key)

**Pass Criteria:**
- Database populated from TOML
- TOML file not modified (no write-back needed)
- Logs show TOML → database migration

---

## tc_m_migration_003: Web UI Save and Verification

**Requirement:** [APIK-WB-030], [APIK-UI-010 through 060]

**Manual Steps:**
1. Start wkmp-ai (with any existing key or none)
2. Open http://localhost:5723/settings in browser
3. Observe settings page UI:
   - Input field for API key
   - Save button
   - Link to https://acoustid.org/api-key
   - No current key displayed (security)
4. Enter test key: "manual-ui-test-key"
5. Click Save button
6. Observe success message: "API key saved successfully"
7. Check database contents
8. Check TOML file contents
9. Observe logs for write-back messages
10. Restart wkmp-ai
11. Verify startup uses saved key

**Expected Observations:**
- Settings page renders correctly (all UI elements present)
- Current key NOT displayed (shows "Key configured" or empty)
- Success message appears after save
- Database contains "manual-ui-test-key"
- TOML contains acoustid_api_key = "manual-ui-test-key"
- Logs: "INFO: API key backed up to TOML config"
- Restart logs: "INFO: AcoustID API key: Loaded from database"

**Pass Criteria:**
- Web UI functional and secure (no key display)
- Save updates database and TOML
- Restart uses saved key

---

**Test File:** tc_m_migration_001_to_003.md
**Total Tests:** 3
**Requirements Coverage:** APIK-WB-010, 020, 030, APIK-LOG-030, APIK-UI-010 through 060
