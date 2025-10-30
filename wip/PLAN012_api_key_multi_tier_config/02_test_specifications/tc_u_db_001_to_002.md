# Unit Tests: Database Accessors (tc_u_db_001 to tc_u_db_002)

**Test Category:** Unit Tests
**Component:** wkmp-ai/src/db/settings.rs - Database accessor functions
**Requirements Covered:** APIK-DB-010, APIK-DB-020, APIK-ACID-040

---

## tc_u_db_001: get_acoustid_api_key Returns Value

**Requirement:** [APIK-DB-010], [APIK-DB-020]

**Setup:**
- In-memory SQLite database
- Settings table created
- Insert: key="acoustid_api_key", value="test-key-123"

**Execution:**
- Call get_acoustid_api_key(db)

**Expected Result:**
- Returns Ok(Some("test-key-123"))

**Verification:**
- Assert result is Ok
- Assert value == Some("test-key-123")

**Edge Case:**
- When key missing: returns Ok(None)

---

## tc_u_db_002: set_acoustid_api_key Writes Value

**Requirement:** [APIK-DB-010], [APIK-DB-020], [APIK-ERR-030]

**Setup:**
- In-memory SQLite database
- Settings table created

**Execution:**
- Call set_acoustid_api_key(db, "new-key-456")

**Expected Result:**
- Returns Ok(())
- Database contains key="acoustid_api_key", value="new-key-456"

**Verification:**
- Query settings table directly
- Assert key exists
- Assert value == "new-key-456"

**Edge Case:**
- Update existing key: old value overwritten

---

**Test File:** tc_u_db_001_to_002.md
**Total Tests:** 2
**Requirements Coverage:** APIK-DB-010, 020, APIK-ACID-040, APIK-ERR-030
