# Integration Test: Concurrency (tc_i_concurrent_001)

**Test Category:** Integration Test
**Component:** Concurrent TOML reads
**Requirements Covered:** APIK-ATOMIC-020 (read safety)

---

## tc_i_concurrent_001: Multiple Module Startup TOML Reads Safe

**Requirement:** [APIK-ATOMIC-020] (race condition prevention)

**Setup:**
- TOML file exists with acoustid_api_key = "concurrent-test-key"
- Multiple modules configured to read same TOML (simulate with multiple wkmp-ai instances or different modules)

**Execution:**
- Start 3 instances of wkmp-ai simultaneously
- All instances read from same TOML file during startup

**Expected Result:**
- All 3 instances start successfully
- All 3 instances load "concurrent-test-key"
- No TOML corruption
- No read errors (file locked, partial reads, etc.)
- TOML file unchanged after startup

**Verification:**
- Assert all 3 instances running (HTTP servers respond)
- Assert all 3 instances loaded same key
- Assert TOML file intact (no corruption)
- Parse TOML file: assert acoustid_api_key unchanged

**Note:** Concurrent writes tested separately (atomic write tests verify write safety via temp file + rename).

---

**Test File:** tc_i_concurrent_001.md
**Total Tests:** 1
**Requirements Coverage:** APIK-ATOMIC-020
