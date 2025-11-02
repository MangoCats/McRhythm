# Integration Tests: Web UI Endpoint (tc_i_ui_001 to tc_i_ui_003)

**Test Category:** Integration Tests
**Component:** wkmp-ai/src/api/handlers.rs - POST /api/settings/acoustid_api_key
**Requirements Covered:** APIK-UI-010, APIK-UI-020, APIK-UI-030, APIK-WB-030, APIK-SYNC-010

---

## tc_i_ui_001: POST /api/settings/acoustid_api_key Success

**Requirement:** [APIK-UI-010], [APIK-UI-020], [APIK-UI-030]

**Setup:**
- wkmp-ai HTTP server running
- Database initialized

**Execution:**
- POST http://localhost:5723/api/settings/acoustid_api_key
- Headers: Content-Type: application/json
- Body: `{"api_key": "new-ui-key-789"}`

**Expected Result:**
- Response: 200 OK
- Response body: `{"message": "API key saved successfully"}`
- Database contains "new-ui-key-789"

**Verification:**
- Assert response status == 200
- Assert response JSON matches expected format
- Query database: assert acoustid_api_key == "new-ui-key-789"

---

## tc_i_ui_002: POST with Empty Key Returns Error

**Requirement:** [APIK-UI-020], [APIK-VAL-010]

**Setup:**
- wkmp-ai HTTP server running

**Execution:**
- POST http://localhost:5723/api/settings/acoustid_api_key
- Body: `{"api_key": ""}`

**Expected Result:**
- Response: 400 Bad Request
- Response body: `{"error": "API key cannot be empty"}`
- Database unchanged

**Verification:**
- Assert response status == 400
- Assert error message contains "empty"

---

## tc_i_ui_003: POST Writes to Database and TOML

**Requirement:** [APIK-WB-030], [APIK-SYNC-010]

**Setup:**
- wkmp-ai HTTP server running
- TOML file exists

**Execution:**
- POST http://localhost:5723/api/settings/acoustid_api_key
- Body: `{"api_key": "ui-key-999"}`

**Expected Result:**
- Response: 200 OK
- Database contains "ui-key-999"
- TOML contains acoustid_api_key = "ui-key-999"
- Log: "INFO: API key backed up to TOML config"

**Verification:**
- Assert database updated
- Assert TOML updated
- Assert log contains "backed up to TOML"

---

**Test File:** tc_i_ui_001_to_003.md
**Total Tests:** 3
**Requirements Coverage:** APIK-UI-010, 020, 030, APIK-WB-030, APIK-SYNC-010, APIK-VAL-010
