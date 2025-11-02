# Integration Tests: Database Recovery (tc_i_recovery_001 to tc_i_recovery_002)

**Test Category:** Integration Tests
**Component:** Database deletion recovery workflow
**Requirements Covered:** APIK-TOML-010, APIK-TOML-020, APIK-WB-020

---

## tc_i_recovery_001: Database Deletion Recovers from TOML

**Requirement:** [APIK-TOML-010], [APIK-TOML-020]

**Setup:**
- wkmp-ai running with key in database and TOML
- Database contains "recovery-test-key"
- TOML contains acoustid_api_key = "recovery-test-key"

**Execution:**
1. Stop wkmp-ai
2. Delete database file (rm wkmp.db)
3. Restart wkmp-ai

**Expected Result:**
- wkmp-ai starts successfully
- Key loaded from TOML: "recovery-test-key"
- Key migrated back to new database
- Logs:
  ```
  INFO: AcoustID API key: Loaded from TOML config
  INFO: Migrating API key to database for persistence...
  INFO: API key saved to database
  ```
- Module functional (can make AcoustID API calls)

**Verification:**
- Assert module starts (HTTP server responds)
- Assert database recreated with key
- Assert logs show TOML → database migration

---

## tc_i_recovery_002: Database Deletion with No TOML Fails Gracefully

**Requirement:** [APIK-RES-050], [APIK-ERR-010]

**Setup:**
- wkmp-ai running with key only in database (TOML empty)
- Database contains "db-only-key"
- TOML does not contain acoustid_api_key

**Execution:**
1. Stop wkmp-ai
2. Delete database file
3. Restart wkmp-ai

**Expected Result:**
- wkmp-ai startup fails
- Error message: "AcoustID API key not found"
- Error lists all 3 configuration methods
- User directed to configure key

**Verification:**
- Assert module startup fails
- Assert error message clear and helpful
- User understands how to resolve (reconfigure key)

---

**Test File:** tc_i_recovery_001_to_002.md
**Total Tests:** 2
**Requirements Coverage:** APIK-TOML-010, 020, APIK-WB-020, APIK-RES-050, APIK-ERR-010
