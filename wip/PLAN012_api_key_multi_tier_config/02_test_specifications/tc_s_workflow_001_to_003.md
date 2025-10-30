# System Tests: User Workflows (tc_s_workflow_001 to tc_s_workflow_003)

**Test Category:** System Tests
**Component:** End-to-end user workflows
**Requirements Covered:** APIK-UI-040 through 060, APIK-MIG-010, APIK-TOML-010

---

## tc_s_workflow_001: New User Configures Key via Web UI

**Requirement:** [APIK-UI-040], [APIK-UI-050], [APIK-UI-060]

**Scenario:** First-time user needs to configure AcoustID API key

**Steps:**
1. User installs wkmp-ai (fresh installation, no existing config)
2. User starts wkmp-ai
3. Startup fails with error: "AcoustID API key not found"
4. User visits https://acoustid.org/api-key and obtains free key
5. User opens http://localhost:5723/settings in browser
6. User sees settings page with:
   - Input field for API key
   - Save button
   - Link to acoustid.org/api-key
   - No current key displayed (security)
7. User enters API key in input field
8. User clicks Save button
9. User sees success message: "API key saved successfully"
10. User restarts wkmp-ai
11. wkmp-ai starts successfully

**Expected Result:**
- Settings page renders correctly
- API key saved to database and TOML
- Subsequent startup uses database key
- User never sees raw key displayed (security)

**Verification:**
- Manual: User completes workflow successfully
- Database contains saved key
- TOML contains saved key
- wkmp-ai starts without error after configuration

---

## tc_s_workflow_002: Developer Uses ENV Variable for CI/CD

**Requirement:** [APIK-MIG-010], [APIK-WB-010]

**Scenario:** Developer deploys wkmp-ai in CI/CD environment with secrets management

**Steps:**
1. Developer sets environment variable in CI/CD pipeline:
   ```bash
   export WKMP_ACOUSTID_API_KEY="ci-test-key-from-vault"
   ```
2. CI/CD starts wkmp-ai (no database or TOML pre-configured)
3. wkmp-ai starts successfully
4. Logs show ENV migration to database and TOML
5. Developer verifies AcoustID functionality works
6. Database and TOML now contain key (durable for future runs)
7. Developer can unset ENV variable after first run
8. Subsequent runs use database key

**Expected Result:**
- ENV variable provides key on first run
- Auto-migration makes configuration durable
- ENV can be unset after migration (security - reduces exposure)
- CI/CD pipeline works without pre-configuring TOML

**Verification:**
- wkmp-ai starts with ENV only (no database/TOML)
- Migration logs appear
- Database and TOML populated
- Second run works without ENV variable

---

## tc_s_workflow_003: Database Deletion Recovery Workflow

**Requirement:** [APIK-TOML-010], [APIK-TOML-020]

**Scenario:** Developer frequently deletes database during testing

**Steps:**
1. Developer has configured API key via web UI
2. Database and TOML both contain key
3. Developer runs `rm wkmp.db` to reset database for testing
4. Developer restarts wkmp-ai
5. wkmp-ai starts successfully (loads from TOML)
6. Key automatically migrated back to new database
7. Developer continues testing without reconfiguring key

**Expected Result:**
- TOML backup prevents need to reconfigure
- Database automatically recreated with key from TOML
- Developer workflow not interrupted
- Primary use case (APIK-TOML-020) validated

**Verification:**
- Database deletion does not require key reconfiguration
- TOML provides automatic recovery
- Developer productivity maintained

---

**Test File:** tc_s_workflow_001_to_003.md
**Total Tests:** 3
**Requirements Coverage:** APIK-UI-040 through 060, APIK-MIG-010, APIK-TOML-010, 020, APIK-WB-010
