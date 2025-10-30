# PLAN012 - Manual Test Procedures

**Purpose:** Document manual test procedures for system-level workflows.

**Note:** These tests require manual execution and cannot be fully automated as they involve:
- Multi-module startup scenarios
- Environment variable configuration
- File system operations across sessions
- End-user workflows

---

## tc_s_workflow_001: New User Configures Key via Web UI

**Purpose:** Verify complete new user workflow from clean state to configured system

**Prerequisites:**
- wkmp-ai not yet configured (no database, no TOML)
- Browser available for accessing web UI

**Procedure:**
1. Start wkmp-ai from clean state:
   ```bash
   cargo run -p wkmp-ai
   ```

2. Verify startup logs show:
   ```
   Failed to resolve AcoustID API key: AcoustID API key not configured
   AcoustID fingerprinting will not be available
   Import workflow will be limited to file metadata only
   ```

3. Open http://localhost:5723/settings in browser

4. Enter valid AcoustID API key in password field

5. Click "Save API Key" button

6. Verify success message: "AcoustID API key configured successfully"

7. Verify input field cleared (security requirement)

8. Stop wkmp-ai (Ctrl+C)

9. Restart wkmp-ai:
   ```bash
   cargo run -p wkmp-ai
   ```

10. Verify startup logs show:
    ```
    AcoustID API key loaded from database
    ```

11. Verify TOML file exists at `~/.config/wkmp/wkmp-ai.toml`

12. Verify TOML contains API key:
    ```bash
    cat ~/.config/wkmp/wkmp-ai.toml
    # Should show: acoustid_api_key = "your-key-here"
    ```

**Expected Outcome:**
- ✅ Key saved to database
- ✅ Key written to TOML backup
- ✅ Key persists across restarts
- ✅ Startup loads from database (not TOML or ENV)

**Cleanup:**
```bash
rm ~/.config/wkmp/wkmp-ai.toml
rm ~/Music/wkmp.db  # Or whatever root folder is configured
```

---

## tc_s_workflow_002: Developer Uses ENV Variable for CI/CD

**Purpose:** Verify environment variable workflow for automated deployments

**Prerequisites:**
- Clean wkmp-ai installation (no existing configuration)
- Shell environment for setting variables

**Procedure:**
1. Set environment variable:
   ```bash
   export WKMP_ACOUSTID_API_KEY=test-key-env-migration
   ```

2. Start wkmp-ai:
   ```bash
   cargo run -p wkmp-ai
   ```

3. Verify startup logs show:
   ```
   AcoustID API key loaded from environment variable
   AcoustID API key migrated from environment to database
   ```

4. Verify TOML file created at `~/.config/wkmp/wkmp-ai.toml`

5. Verify TOML contains key:
   ```bash
   cat ~/.config/wkmp/wkmp-ai.toml | grep acoustid_api_key
   # Should show: acoustid_api_key = "test-key-env-migration"
   ```

6. Verify database contains key:
   ```bash
   sqlite3 ~/Music/wkmp.db "SELECT value FROM settings WHERE key = 'acoustid_api_key';"
   # Should output: test-key-env-migration
   ```

7. Unset environment variable:
   ```bash
   unset WKMP_ACOUSTID_API_KEY
   ```

8. Stop and restart wkmp-ai:
   ```bash
   cargo run -p wkmp-ai
   ```

9. Verify startup logs show:
   ```
   AcoustID API key loaded from database
   ```
   (NOT from environment, since it's unset)

10. Verify key persists from database (ENV no longer needed)

**Expected Outcome:**
- ✅ ENV variable auto-migrated to database
- ✅ ENV variable auto-migrated to TOML (backup)
- ✅ After unsetting ENV, key persists in database
- ✅ Database tier takes priority over ENV

**Cleanup:**
```bash
unset WKMP_ACOUSTID_API_KEY
rm ~/.config/wkmp/wkmp-ai.toml
rm ~/Music/wkmp.db
```

---

## tc_s_workflow_003: Database Deletion Recovery Workflow

**Purpose:** Verify TOML backup enables recovery from database deletion

**Prerequisites:**
- wkmp-ai configured with API key (via UI or ENV)
- Database and TOML both exist with API key

**Procedure:**
1. Configure key if not already configured (use tc_s_workflow_001 or tc_s_workflow_002)

2. Verify both database and TOML have key:
   ```bash
   sqlite3 ~/Music/wkmp.db "SELECT value FROM settings WHERE key = 'acoustid_api_key';"
   cat ~/.config/wkmp/wkmp-ai.toml | grep acoustid_api_key
   ```

3. Stop wkmp-ai

4. Delete database file (simulates corruption or development reset):
   ```bash
   rm ~/Music/wkmp.db
   ```

5. Verify TOML still exists:
   ```bash
   ls ~/.config/wkmp/wkmp-ai.toml
   # Should exist
   ```

6. Start wkmp-ai:
   ```bash
   cargo run -p wkmp-ai
   ```

7. Verify startup logs show:
   ```
   AcoustID API key loaded from TOML config
   ```
   (Falls back to TOML since database is missing)

8. Verify database recreated with key migrated from TOML

9. Verify key accessible:
   ```bash
   sqlite3 ~/Music/wkmp.db "SELECT value FROM settings WHERE key = 'acoustid_api_key';"
   ```

10. Restart wkmp-ai and verify loads from database:
    ```
    AcoustID API key loaded from database
    ```

**Expected Outcome:**
- ✅ Database deletion doesn't lose configuration
- ✅ TOML backup enables automatic recovery
- ✅ Key auto-migrated back to database
- ✅ Subsequent restarts load from database (normal operation restored)

**Cleanup:**
```bash
rm ~/.config/wkmp/wkmp-ai.toml
rm ~/Music/wkmp.db
```

---

## Test Execution Record

| Test ID | Date | Tester | Status | Notes |
|---------|------|--------|--------|-------|
| tc_s_workflow_001 | - | - | Pending | New user via web UI |
| tc_s_workflow_002 | - | - | Pending | Developer ENV workflow |
| tc_s_workflow_003 | - | - | Pending | Database recovery |

---

## Common Issues and Troubleshooting

**Issue: TOML file not created**
- Check wkmp-ai logs for write errors
- Verify `~/.config/wkmp/` directory exists and is writable
- Run: `chmod 755 ~/.config/wkmp/` (Linux/macOS)

**Issue: Database not found**
- Check root folder resolution (logs show path on startup)
- Verify ~/Music directory exists (or configured root folder)
- Check database path in logs: `Database: /path/to/wkmp.db`

**Issue: ENV variable not recognized**
- Verify spelling: `WKMP_ACOUSTID_API_KEY` (case-sensitive)
- Check variable is exported (not just set): `export WKMP_ACOUSTID_API_KEY=...`
- Verify variable visible to process: `env | grep WKMP`

**Issue: Key not persisting**
- Check TOML permissions (should be readable)
- Verify database write succeeded (check logs for errors)
- Ensure database file not deleted between restarts

---

**Manual Test Status:** DOCUMENTED (ready for execution)
