# Unit Tests: Security (tc_u_sec_001 to tc_u_sec_002)

**Test Category:** Unit Tests
**Component:** wkmp-ai/src/config.rs - Permission checking
**Requirements Covered:** APIK-SEC-040, APIK-SEC-050, APIK-SEC-060

---

## tc_u_sec_001: Permission Check Detects Loose Permissions (Unix)

**Requirement:** [APIK-SEC-040], [APIK-SEC-050]

**Platform:** Unix only (Linux, macOS)

**Setup:**
- Create TOML file with permissions 0644 (rw-r--r--)
- File is readable by group and others (loose permissions)

**Execution:**
- Call check_toml_permissions(path)

**Expected Result:**
- Function detects mode & 0o077 != 0 (loose permissions)
- Returns warning flag or message

**Verification:**
- Assert warning detected
- Assert mode & 0o077 != 0

---

## tc_u_sec_002: Permission Warning Logged for Loose Permissions

**Requirement:** [APIK-SEC-050], [APIK-SEC-060]

**Setup:**
- TOML file with permissions 0644
- Call resolve_acoustid_api_key() which loads from TOML

**Execution:**
- Resolver checks permissions during TOML read

**Expected Result:**
- Log warning:
  ```
  WARNING: TOML config file /path/to/wkmp-ai.toml has loose permissions (readable by others).
  Recommend: chmod 600 /path/to/wkmp-ai.toml
  ```
- Operation continues (warning only, not error)

**Verification:**
- Assert log level is WARN
- Assert log contains "loose permissions"
- Assert log contains "chmod 600" recommendation
- Assert resolver returns key successfully (not blocked)

---

**Test File:** tc_u_sec_001_to_002.md
**Total Tests:** 2
**Requirements Coverage:** APIK-SEC-040, 050, 060
