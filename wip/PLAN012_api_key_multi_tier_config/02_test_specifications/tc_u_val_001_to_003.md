# Unit Tests: Validation (tc_u_val_001 to tc_u_val_003)

**Test Category:** Unit Tests
**Component:** wkmp-ai/src/config.rs - API key validation
**Requirements Covered:** APIK-VAL-010

---

## tc_u_val_001: Empty Key Rejected

**Requirement:** [APIK-VAL-010]

**Setup:**
- Validation function receives empty string: ""

**Execution:**
- Call validate_api_key("")

**Expected Result:**
- Returns Err
- Error message: "API key cannot be empty"

**Verification:**
- Assert error returned
- Assert error message mentions "empty"

---

## tc_u_val_002: Whitespace-Only Key Rejected

**Requirement:** [APIK-VAL-010]

**Setup:**
- Validation function receives whitespace: "   \t\n  "

**Execution:**
- Call validate_api_key("   \t\n  ")

**Expected Result:**
- Returns Err
- Error message: "API key cannot be whitespace only"

**Verification:**
- Assert error returned
- Assert error message mentions "whitespace"

---

## tc_u_val_003: NULL Key Rejected

**Requirement:** [APIK-VAL-010]

**Setup:**
- Database query returns NULL for acoustid_api_key
- ENV variable not set (None)
- TOML field missing (None)

**Execution:**
- Call resolve_acoustid_api_key(db, resolver) with NULL sources

**Expected Result:**
- Returns Err (no valid key found)
- Error message lists all 3 methods

**Verification:**
- Assert error returned
- Assert error is "not found" error (not validation error)
- Validates that NULL is treated as "missing" not "invalid"

---

**Test File:** tc_u_val_001_to_003.md
**Total Tests:** 3
**Requirements Coverage:** APIK-VAL-010
