# Unit Tests: TOML Utilities (tc_u_toml_001 to tc_u_toml_007)

**Test Category:** Unit Tests
**Component:** wkmp-common/src/config.rs - TOML utilities
**Requirements Covered:** APIK-TOML-030, APIK-TOML-040, APIK-TOML-050, APIK-ATOMIC-010, APIK-ATOMIC-020, APIK-SEC-010, APIK-SEC-020

---

## tc_u_toml_001: Atomic Write Creates Temp File

**Requirement:** [APIK-ATOMIC-010]

**Setup:**
- Target path: /tmp/test-config.toml
- TomlConfig with acoustid_api_key = "test-key"

**Execution:**
- Call write_toml_config(config, "/tmp/test-config.toml")
- Pause during serialization (mock delay)

**Expected Result:**
- Temp file created: /tmp/test-config.toml.tmp
- Temp file contains serialized TOML data

**Verification:**
- Assert temp file exists during write
- Assert temp file contains correct TOML structure

---

## tc_u_toml_002: Atomic Write Renames to Target

**Requirement:** [APIK-ATOMIC-010]

**Setup:**
- Temp file created with TOML data
- Target path: /tmp/test-config.toml

**Execution:**
- Complete write_toml_config() call

**Expected Result:**
- Temp file renamed to target path
- Temp file no longer exists
- Target file contains TOML data

**Verification:**
- Assert temp file deleted
- Assert target file exists
- Assert target file contains correct TOML

---

## tc_u_toml_003: Atomic Write Preserves Existing Fields

**Requirement:** [APIK-TOML-030]

**Setup:**
- Existing TOML file:
  ```toml
  root_folder = "/home/user/Music"
  [logging]
  level = "debug"
  acoustid_api_key = "old-key"
  ```
- Update acoustid_api_key to "new-key"

**Execution:**
- Read existing TOML
- Modify acoustid_api_key field
- Call write_toml_config(modified_config, path)

**Expected Result:**
- TOML file contains:
  ```toml
  root_folder = "/home/user/Music"
  [logging]
  level = "debug"
  acoustid_api_key = "new-key"
  ```
- root_folder unchanged
- logging section unchanged
- acoustid_api_key updated

**Verification:**
- Parse updated TOML
- Assert root_folder == "/home/user/Music"
- Assert logging.level == "debug"
- Assert acoustid_api_key == "new-key"

---

## tc_u_toml_004: Atomic Write Sets Permissions 0600 (Unix)

**Requirement:** [APIK-TOML-050], [APIK-SEC-010], [APIK-SEC-020]

**Platform:** Unix only (Linux, macOS)

**Setup:**
- Target path: /tmp/test-config.toml
- TomlConfig with acoustid_api_key

**Execution:**
- Call write_toml_config(config, path)

**Expected Result:**
- File created with permissions 0600 (rw-------)
- Owner can read/write
- Group cannot read
- Others cannot read

**Verification:**
- Use std::fs::metadata() to get file permissions
- Assert mode & 0o777 == 0o600
- Or use `stat -c %a` command: assert output == "600"

---

## tc_u_toml_005: Atomic Write Graceful on Windows (No chmod)

**Requirement:** [APIK-SEC-030]

**Platform:** Windows

**Setup:**
- Target path: C:\Users\test\config.toml
- TomlConfig with acoustid_api_key

**Execution:**
- Call write_toml_config(config, path)

**Expected Result:**
- File created successfully
- No error on permission setting (chmod not available)
- Relies on NTFS ACLs (default user-only)

**Verification:**
- Assert file exists
- Assert no permission-related errors
- Note: Full NTFS ACL verification out of scope

---

## tc_u_toml_006: Round-Trip Serialization Preserves Data

**Requirement:** [APIK-TOML-030], [APIK-TOML-SCHEMA-010]

**Setup:**
- TomlConfig with all fields populated:
  - root_folder = Some("/home/user/Music")
  - logging.level = "info"
  - logging.log_file = Some("/var/log/wkmp.log")
  - static_assets = Some("/usr/share/wkmp")
  - acoustid_api_key = Some("test-key-123")

**Execution:**
- Serialize to TOML string
- Deserialize back to TomlConfig struct
- Compare original and deserialized

**Expected Result:**
- Deserialized config matches original
- All fields preserved
- No data loss

**Verification:**
- Assert root_folder unchanged
- Assert logging fields unchanged
- Assert static_assets unchanged
- Assert acoustid_api_key unchanged

---

## tc_u_toml_007: Corrupt Temp File Does Not Overwrite Target

**Requirement:** [APIK-ATOMIC-020]

**Setup:**
- Existing valid TOML file at target path
- Simulate crash during temp file write (partial data)

**Execution:**
- Start write_toml_config()
- Simulate crash before rename (delete temp file or corrupt it)

**Expected Result:**
- Target file unchanged (original data intact)
- No corruption of existing TOML
- Atomic rename did not occur

**Verification:**
- Assert target file contains original data
- Assert no data loss from failed write

---

**Test File:** tc_u_toml_001_to_007.md
**Total Tests:** 7
**Requirements Coverage:** APIK-TOML-030, 040, 050, APIK-ATOMIC-010, 020, APIK-SEC-010, 020, 030
