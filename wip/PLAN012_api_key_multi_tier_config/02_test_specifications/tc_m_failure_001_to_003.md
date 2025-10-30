# Manual Tests: Failure Scenarios (tc_m_failure_001 to tc_m_failure_003)

**Test Category:** Manual Tests
**Component:** Error handling and graceful degradation
**Requirements Covered:** APIK-WB-040, APIK-ERR-010, APIK-ERR-020, APIK-LOG-040, APIK-SEC-050, APIK-SEC-060

---

## tc_m_failure_001: Read-Only TOML Filesystem

**Requirement:** [APIK-WB-040], [APIK-ERR-020], [APIK-LOG-040]

**Manual Steps:**
1. Configure key via ENV variable (to trigger TOML write-back)
2. Make TOML file directory read-only:
   ```bash
   # Linux/macOS
   chmod 555 ~/.config/wkmp
   ```
3. Start wkmp-ai
4. Observe logs and startup success/failure
5. Check database contents
6. Restore write permissions:
   ```bash
   chmod 755 ~/.config/wkmp
   ```

**Expected Observations:**
- wkmp-ai starts SUCCESSFULLY (database write succeeded)
- Logs show database migration:
  ```
  INFO: AcoustID API key: Loaded from environment variable
  INFO: Migrating API key to database for persistence...
  INFO: API key saved to database
  ```
- Logs show TOML write warning (not error):
  ```
  WARN: Could not back up API key to TOML config: Permission denied (os error 13).
        TOML may be read-only. Key saved to database successfully.
  ```
- Database contains key
- TOML write failed gracefully (best-effort approach)

**Pass Criteria:**
- Startup succeeds despite TOML write failure
- Warning logged (not error)
- Database write succeeded (authoritative storage)
- User can continue using wkmp-ai

---

## tc_m_failure_002: Permission Warnings on Loose Permissions

**Requirement:** [APIK-SEC-050], [APIK-SEC-060]

**Platform:** Unix only (Linux, macOS)

**Manual Steps:**
1. Manually create TOML file with loose permissions:
   ```bash
   echo 'acoustid_api_key = "test-key"' > ~/.config/wkmp/wkmp-ai.toml
   chmod 644 ~/.config/wkmp/wkmp-ai.toml
   ```
2. Verify permissions are loose (readable by group/others):
   ```bash
   ls -l ~/.config/wkmp/wkmp-ai.toml
   # Should show: -rw-r--r--
   ```
3. Start wkmp-ai
4. Observe logs for permission warning
5. Verify wkmp-ai continues to run (warning only)

**Expected Observations:**
- wkmp-ai starts successfully (warning does not block)
- Logs show permission warning:
  ```
  WARNING: TOML config file /home/user/.config/wkmp/wkmp-ai.toml has loose permissions (readable by others).
  Recommend: chmod 600 /home/user/.config/wkmp/wkmp-ai.toml
  ```
- Warning includes exact file path
- Warning includes chmod 600 recommendation
- Module continues to run (security warning, not error)

**Pass Criteria:**
- Warning logged with correct file path
- Recommendation actionable (chmod 600 command)
- Startup not blocked (informational warning)

---

## tc_m_failure_003: Invalid Key Error Messages

**Requirement:** [APIK-ERR-010]

**Manual Steps:**
1. Delete database, unset ENV, remove TOML key
2. Attempt to start wkmp-ai
3. Observe error message content
4. Verify error lists all 3 configuration methods
5. Verify error includes link to obtain key

**Expected Observations:**
- wkmp-ai startup fails
- Error message comprehensive:
  ```
  ERROR: AcoustID API key not found.

  Configure using one of these methods:

  1. Database: Use web UI at http://localhost:5723/settings
     (currently: empty)

  2. Environment variable: WKMP_ACOUSTID_API_KEY
     (currently: not set)

  3. TOML config: ~/.config/wkmp/wkmp-ai.toml
     Add line: acoustid_api_key = "YOUR_KEY_HERE"
     (currently: acoustid_api_key not found)

  Obtain free API key: https://acoustid.org/api-key
  ```
- Error is clear and actionable
- User understands how to resolve

**Pass Criteria:**
- All 3 configuration methods listed
- Current state shown for each method
- Link to obtain key included
- Error message helpful for troubleshooting

---

**Test File:** tc_m_failure_001_to_003.md
**Total Tests:** 3
**Requirements Coverage:** APIK-WB-040, APIK-ERR-010, 020, APIK-LOG-040, APIK-SEC-050, 060
