# PLAN012 - Implementation Checkpoints

**Purpose:** Define verification points every 5-10 increments to ensure progress and quality.

---

## Checkpoint 1: Foundation Complete (After Increment 2)

**Increments:** 1-2
**Date:** TBD

### Verification Criteria

- [ ] TomlConfig struct extended with acoustid_api_key field
- [ ] Serialize trait added to TomlConfig
- [ ] write_toml_config() function implemented
- [ ] Atomic write uses temp file + rename
- [ ] Unix permissions 0600 set correctly
- [ ] All wkmp-common unit tests pass (tc_u_toml_001-007)
- [ ] No regressions in existing wkmp-common tests

### Exit Criteria

**PASS:** All verification criteria met
**FAIL:** If any criterion fails, fix before proceeding to Increment 3

### Deliverables Ready for Next Phase

- wkmp-common TOML utilities usable by wkmp-ai
- Foundation stable for resolver implementation

---

## Checkpoint 2: Core Logic Complete (After Increment 5)

**Increments:** 3-5
**Date:** TBD

### Verification Criteria

- [ ] Database accessors (get/set_acoustid_api_key) implemented
- [ ] Multi-tier resolver (resolve_acoustid_api_key) implemented
- [ ] Validation (is_valid_key) working
- [ ] Settings sync (sync_settings_to_toml) implemented
- [ ] Write-back behavior (migrate_key_to_database) implemented
- [ ] All wkmp-ai unit tests pass (tc_u_res_*, tc_u_wb_*, tc_u_val_*, tc_u_db_*, tc_u_sec_*)
- [ ] Total unit tests: 28 (wkmp-common 7 + wkmp-ai 21)

### Exit Criteria

**PASS:** All verification criteria met
**FAIL:** If any criterion fails, fix before proceeding to Increment 6

### Deliverables Ready for Next Phase

- Core configuration logic complete and tested
- Ready for startup integration

---

## Checkpoint 3: Integration Complete (After Increment 9)

**Increments:** 6-9
**Date:** TBD

### Verification Criteria

- [ ] Startup integration (main.rs) complete
- [ ] Web UI endpoint (POST /api/settings/acoustid_api_key) working
- [ ] Web UI settings page (/settings) implemented
- [ ] All integration tests pass (tc_i_e2e_*, tc_i_ui_*, tc_i_recovery_*, tc_i_concurrent_*)
- [ ] All system tests pass (tc_s_workflow_001-003)
- [ ] Total tests passing: 41 (unit 28 + integration 10 + system 3)

### Exit Criteria

**PASS:** All verification criteria met
**FAIL:** If any criterion fails, fix before proceeding to Increment 10

### Deliverables Ready for Next Phase

- End-to-end functionality working
- Web UI functional
- Ready for manual testing and documentation

---

## Checkpoint 4: Implementation Complete (After Increment 10)

**Increments:** 10
**Date:** TBD

### Verification Criteria

- [ ] All 6 manual tests executed (tc_m_migration_*, tc_m_failure_*)
- [ ] IMPL012-acoustid_client.md updated
- [ ] IMPL001-database_schema.md updated
- [ ] User documentation complete and clear
- [ ] Total tests passing: 47 (unit 28 + integration 10 + system 3 + manual 6)
- [ ] 100% requirement coverage verified (62/62 requirements)
- [ ] All 21 acceptance criteria met

### Exit Criteria

**PASS:** All verification criteria met → Implementation COMPLETE
**FAIL:** If any criterion fails, create bugfix increment

### Deliverables Ready for Deployment

- Feature fully implemented
- All tests passing
- Documentation complete
- Ready for code review and /commit

---

## Checkpoint Usage Guidelines

**At Each Checkpoint:**
1. **Stop:** Pause implementation
2. **Verify:** Run all tests for completed increments
3. **Document:** Record checkpoint status (PASS/FAIL)
4. **Decide:** PASS → Continue to next increment, FAIL → Fix issues before proceeding

**Benefits:**
- Early detection of integration issues
- Prevents accumulation of technical debt
- Ensures each phase is stable before building on it
- Provides clear project status visibility

**Failure Recovery:**
- If checkpoint fails, create bugfix increment(s)
- Re-run checkpoint verification after fixes
- Do not proceed until checkpoint passes

---

## Checkpoint History

| Checkpoint | Date | Status | Notes |
|------------|------|--------|-------|
| 1 (Foundation) | TBD | Pending | Increments 1-2 |
| 2 (Core Logic) | TBD | Pending | Increments 3-5 |
| 3 (Integration) | TBD | Pending | Increments 6-9 |
| 4 (Complete) | TBD | Pending | Increment 10 |

---

**Checkpoints Status:** DEFINED (ready for implementation tracking)
