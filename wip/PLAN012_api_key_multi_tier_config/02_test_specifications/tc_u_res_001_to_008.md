# Unit Tests: Multi-Tier Resolution (tc_u_res_001 to tc_u_res_008)

**Test Category:** Unit Tests
**Component:** wkmp-ai/src/config.rs - resolve_acoustid_api_key()
**Requirements Covered:** APIK-RES-010, APIK-RES-020, APIK-RES-030, APIK-RES-040, APIK-RES-050, APIK-LOG-010, APIK-LOG-020

---

## tc_u_res_001: Database Priority (Database Overrides ENV and TOML)

**Requirement:** [APIK-RES-010], [APIK-RES-020]

**Setup:**
- Database contains key: "db-key-123"
- ENV variable set: WKMP_ACOUSTID_API_KEY="env-key-456"
- TOML file contains: acoustid_api_key = "toml-key-789"

**Execution:**
- Call resolve_acoustid_api_key(db, resolver)

**Expected Result:**
- Returns "db-key-123"
- Logs: "INFO: AcoustID API key: Loaded from database"

**Verification:**
- Assert returned key == "db-key-123"
- Assert log contains "Loaded from database"

---

## tc_u_res_002: ENV Fallback (Database Empty, ENV Provides Key)

**Requirement:** [APIK-RES-010], [APIK-RES-030], [APIK-WB-010]

**Setup:**
- Database empty (no acoustid_api_key setting)
- ENV variable set: WKMP_ACOUSTID_API_KEY="env-key-456"
- TOML file contains: acoustid_api_key = "toml-key-789"

**Execution:**
- Call resolve_acoustid_api_key(db, resolver)

**Expected Result:**
- Returns "env-key-456"
- Logs: "INFO: AcoustID API key: Loaded from environment variable"
- Logs: "INFO: Migrating API key to database for persistence..."
- Database now contains "env-key-456"
- TOML file now contains "env-key-456" (write-back)

**Verification:**
- Assert returned key == "env-key-456"
- Assert database contains "env-key-456"
- Assert TOML contains "env-key-456"
- Assert log contains "Loaded from environment variable"
- Assert log contains "Migrating API key"

---

## tc_u_res_003: TOML Fallback (Database and ENV Empty, TOML Provides Key)

**Requirement:** [APIK-RES-010], [APIK-RES-040], [APIK-WB-020]

**Setup:**
- Database empty
- ENV variable not set
- TOML file contains: acoustid_api_key = "toml-key-789"

**Execution:**
- Call resolve_acoustid_api_key(db, resolver)

**Expected Result:**
- Returns "toml-key-789"
- Logs: "INFO: AcoustID API key: Loaded from TOML config"
- Database now contains "toml-key-789"
- TOML file unchanged (already contains key)

**Verification:**
- Assert returned key == "toml-key-789"
- Assert database contains "toml-key-789"
- Assert TOML unchanged
- Assert log contains "Loaded from TOML config"

---

## tc_u_res_004: Error on No Key (All Sources Empty)

**Requirement:** [APIK-RES-050], [APIK-ERR-010]

**Setup:**
- Database empty
- ENV variable not set
- TOML file missing acoustid_api_key field

**Execution:**
- Call resolve_acoustid_api_key(db, resolver)

**Expected Result:**
- Returns Err with message containing:
  - "AcoustID API key not found"
  - "Database: acoustid_api_key (empty)"
  - "Environment: WKMP_ACOUSTID_API_KEY (not set)"
  - "TOML: ~/.config/wkmp/wkmp-ai.toml (acoustid_api_key not found)"
  - "Obtain free key: https://acoustid.org/api-key"

**Verification:**
- Assert error returned
- Assert error message contains all 3 methods
- Assert error message contains acoustid.org link

---

## tc_u_res_005: Database Ignores ENV When Present

**Requirement:** [APIK-RES-020]

**Setup:**
- Database contains: "db-key-123"
- ENV variable set: WKMP_ACOUSTID_API_KEY="env-key-456"
- TOML empty

**Execution:**
- Call resolve_acoustid_api_key(db, resolver)

**Expected Result:**
- Returns "db-key-123"
- ENV ignored
- No migration (database already has key)

**Verification:**
- Assert returned key == "db-key-123"
- Assert no "Migrating" log message

---

## tc_u_res_006: Database Ignores TOML When Present

**Requirement:** [APIK-RES-020]

**Setup:**
- Database contains: "db-key-123"
- ENV not set
- TOML contains: acoustid_api_key = "toml-key-789"

**Execution:**
- Call resolve_acoustid_api_key(db, resolver)

**Expected Result:**
- Returns "db-key-123"
- TOML ignored
- No migration

**Verification:**
- Assert returned key == "db-key-123"
- Assert no "Migrating" log message

---

## tc_u_res_007: ENV Ignores TOML When Present

**Requirement:** [APIK-RES-010] (priority order)

**Setup:**
- Database empty
- ENV variable set: WKMP_ACOUSTID_API_KEY="env-key-456"
- TOML contains: acoustid_api_key = "toml-key-789"

**Execution:**
- Call resolve_acoustid_api_key(db, resolver)

**Expected Result:**
- Returns "env-key-456"
- TOML ignored (ENV has higher priority)
- Migrates ENV key to database and TOML

**Verification:**
- Assert returned key == "env-key-456"
- Assert database contains "env-key-456"
- Assert TOML updated to "env-key-456" (overwrite)

---

## tc_u_res_008: Multiple Sources Warning Logged

**Requirement:** [APIK-LOG-020]

**Setup:**
- Database contains: "db-key-123"
- ENV variable set: WKMP_ACOUSTID_API_KEY="env-key-456"
- TOML contains: acoustid_api_key = "toml-key-789"

**Execution:**
- Call resolve_acoustid_api_key(db, resolver)

**Expected Result:**
- Returns "db-key-123"
- Logs warning about multiple sources:
  ```
  WARN: AcoustID API key found in multiple sources. Using database value.
        To use environment variable or TOML, delete key from database first.
  ```

**Verification:**
- Assert returned key == "db-key-123"
- Assert log level WARN
- Assert log contains "found in multiple sources"
- Assert log contains "Using database value"

---

**Test File:** tc_u_res_001_to_008.md
**Total Tests:** 8
**Requirements Coverage:** APIK-RES-010, 020, 030, 040, 050, APIK-LOG-010, 020
