# Unit Tests: Write-Back Behavior (tc_u_wb_001 to tc_u_wb_006)

**Test Category:** Unit Tests
**Component:** wkmp-ai/src/config.rs - sync_settings_to_toml(), write-back logic
**Requirements Covered:** APIK-WB-010, APIK-WB-020, APIK-WB-030, APIK-WB-040, APIK-LOG-030, APIK-LOG-040

---

## tc_u_wb_001: ENV to Database Write-Back

**Requirement:** [APIK-WB-010] (step 1)

**Setup:**
- Database empty
- ENV variable set: WKMP_ACOUSTID_API_KEY="env-key-456"

**Execution:**
- Call resolve_acoustid_api_key(db, resolver)

**Expected Result:**
- Key written to database
- Database contains "env-key-456"
- Log: "INFO: API key saved to database"

**Verification:**
- Query database for acoustid_api_key setting
- Assert value == "env-key-456"
- Assert log contains "saved to database"

---

## tc_u_wb_002: ENV to TOML Write-Back

**Requirement:** [APIK-WB-010] (step 2), [APIK-LOG-030]

**Setup:**
- Database empty
- ENV variable set: WKMP_ACOUSTID_API_KEY="env-key-456"
- TOML file exists but acoustid_api_key field missing

**Execution:**
- Call resolve_acoustid_api_key(db, resolver)

**Expected Result:**
- Key written to TOML file
- TOML contains acoustid_api_key = "env-key-456"
- Log: "INFO: API key backed up to TOML config"

**Verification:**
- Parse TOML file
- Assert acoustid_api_key == "env-key-456"
- Assert log contains "backed up to TOML config"

---

## tc_u_wb_003: TOML to Database Write-Back (No TOML Write)

**Requirement:** [APIK-WB-020]

**Setup:**
- Database empty
- ENV not set
- TOML contains acoustid_api_key = "toml-key-789"

**Execution:**
- Call resolve_acoustid_api_key(db, resolver)

**Expected Result:**
- Key written to database
- Database contains "toml-key-789"
- TOML unchanged (no write-back needed)
- Log: "INFO: API key saved to database"
- NO log: "backed up to TOML config"

**Verification:**
- Assert database contains "toml-key-789"
- Assert TOML unchanged (same modification time)
- Assert log does NOT contain "backed up to TOML"

---

## tc_u_wb_004: UI Update to Database Write-Back

**Requirement:** [APIK-WB-030] (step 1)

**Setup:**
- Database contains old key: "old-key-123"
- User submits new key via UI: "new-key-999"

**Execution:**
- Call set_acoustid_api_key(db, "new-key-999")

**Expected Result:**
- Database updated to "new-key-999"
- Old key overwritten

**Verification:**
- Query database
- Assert value == "new-key-999"

---

## tc_u_wb_005: UI Update to TOML Write-Back

**Requirement:** [APIK-WB-030] (step 2), [APIK-SYNC-010]

**Setup:**
- Database updated by UI to "new-key-999"
- TOML exists with old key

**Execution:**
- Call sync_settings_to_toml(resolver, changed_settings)
  - changed_settings = {"acoustid_api_key": "new-key-999"}

**Expected Result:**
- TOML updated to acoustid_api_key = "new-key-999"
- Other TOML fields preserved (root_folder, logging)
- Log: "INFO: API key backed up to TOML config"

**Verification:**
- Parse TOML file
- Assert acoustid_api_key == "new-key-999"
- Assert root_folder unchanged
- Assert logging unchanged

---

## tc_u_wb_006: TOML Write Failure Graceful Degradation

**Requirement:** [APIK-WB-040], [APIK-ERR-020], [APIK-LOG-040]

**Setup:**
- Database updated successfully
- TOML file is read-only (simulate with permissions or missing directory)

**Execution:**
- Call sync_settings_to_toml(resolver, changed_settings)

**Expected Result:**
- Function returns Ok (does not fail)
- Log: "WARN: Could not back up API key to TOML config: {error}."
- Log: "TOML may be read-only. Key saved to database successfully."
- Database write succeeded (verified in previous step)

**Verification:**
- Assert function returns Ok
- Assert log level is WARN (not ERROR)
- Assert log contains "Could not back up"
- Assert log contains "Key saved to database successfully"

---

**Test File:** tc_u_wb_001_to_006.md
**Total Tests:** 6
**Requirements Coverage:** APIK-WB-010, 020, 030, 040, APIK-ERR-020, APIK-LOG-030, 040, APIK-SYNC-010
