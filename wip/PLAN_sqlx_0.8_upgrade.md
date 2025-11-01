# SQLx 0.8.1 Upgrade Plan

**Priority:** HIGH (fixes RUSTSEC-2024-0363 vulnerability)
**Estimated Effort:** 2-4 hours (low risk, straightforward upgrade)
**Modules Affected:** All modules using database (wkmp-common, wkmp-ui, wkmp-dr, wkmp-ap, wkmp-ai)

---

## Executive Summary

Upgrade SQLx from 0.7.4 → 0.8.1+ to address security vulnerability RUSTSEC-2024-0363. Analysis shows minimal breaking changes for WKMP's SQLite-only usage pattern.

**Key Change:** Split combined `runtime-tokio-rustls` feature into separate `runtime-tokio` and `tls-rustls` features.

---

## Security Context

### RUSTSEC-2024-0363: Binary Protocol Misinterpretation
- **Severity:** Unspecified (has a fix)
- **Published:** 2024-08-15
- **Fixed in:** SQLx ≥0.8.1
- **Impact:** All WKMP modules using SQLx
- **Risk:** Unknown severity, but fix available

### RUSTSEC-2023-0071: RSA Marvin Attack
- **Severity:** 5.9/10 (medium)
- **Impact:** Transitive via sqlx-mysql (NOT USED by WKMP)
- **Risk:** LOW (WKMP only uses SQLite, never MySQL)
- **Action:** No action required (sqlx-mysql not in runtime dependency tree)

### RUSTSEC-2024-0436: paste crate unmaintained
- **Type:** Maintenance warning (not security vulnerability)
- **Impact:** Indirect via sqlx-core and lofty
- **Risk:** Low (crate still functions correctly)
- **Action:** Monitor for future replacement

---

## Breaking Changes Analysis

### 1. Feature Flag Changes (REQUIRED)

**Current (0.7.4):**
```toml
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "sqlite", "uuid", "chrono", "json"] }
```

**New (0.8.1+):**
```toml
sqlx = { version = "0.8.1", features = ["runtime-tokio", "tls-rustls", "sqlite", "uuid", "chrono", "json"] }
```

**Change:** `runtime-tokio-rustls` → `runtime-tokio` + `tls-rustls`

**Rationale:** SQLx 0.8 removed combined runtime+TLS features to allow more flexible configuration.

### 2. MSRV (Minimum Supported Rust Version)

**Requirement:** Rust ≥1.78.0
**Current:** Rust 1.89.0 ✅
**Action:** None required (well above MSRV)

### 3. Deprecated Type Ascription Syntax

**Status:** NOT USED in WKMP ✅
**Search Result:** No instances of deprecated `query!(...: _)` syntax found
**Action:** None required

### 4. Offline Mode Changes

**Status:** NOT USED in WKMP ✅
**Files Checked:** No `sqlx-data.json` or `.sqlx/` directories found
**Action:** None required

### 5. API Changes

**PgDatabaseError::position() renamed:** Not applicable (SQLite only)
**Statement Caching Changes:** Transparent for typical usage
**Connection Handling:** No breaking changes for SQLite

---

## Risk Assessment

### Residual Risk: LOW

**Failure Modes:**
1. **Build failure due to feature flags** - VERY LOW (clear migration path, documented)
2. **Runtime behavior changes** - LOW (SQLite driver stable, no reported regressions)
3. **Test failures** - LOW (API remains compatible for our usage patterns)
4. **Performance regression** - LOW (0.8 series focused on stability)

**Mitigation:**
- Comprehensive test coverage (18 tests in wkmp-dr, extensive tests in other modules)
- Staged rollout: Build → Unit tests → Integration tests → Manual verification
- Clean rollback path (git revert single commit)

**Risk Category:** LOW (equivalent to minor version bump for our usage)

---

## Implementation Plan

### Phase 1: Dependency Update (15 minutes)

**File:** `/home/sw/Dev/McRhythm/Cargo.toml`

**Change:**
```toml
[workspace.dependencies]
-sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "sqlite", "uuid", "chrono", "json"] }
+sqlx = { version = "0.8.1", features = ["runtime-tokio", "tls-rustls", "sqlite", "uuid", "chrono", "json"] }
```

**Verification:**
```bash
cargo update -p sqlx
cargo tree | grep sqlx
# Verify: sqlx v0.8.1 or higher
```

### Phase 2: Build Verification (30 minutes)

**Scope:** All modules in dependency order

```bash
# Clean build from scratch
cargo clean

# Build workspace
cargo build --workspace

# Check for warnings/errors
cargo clippy --workspace -- -D warnings
```

**Expected Outcome:**
- Zero compiler errors
- Zero compiler warnings
- Clean clippy pass

**If Build Fails:**
- Check error messages for deprecated API usage
- Consult SQLx 0.8 CHANGELOG for API changes
- Update code to new API (unlikely based on analysis)

### Phase 3: Test Verification (1-2 hours)

**Test Suite Coverage:**

```bash
# Unit tests (per module)
cargo test -p wkmp-common
cargo test -p wkmp-ap
cargo test -p wkmp-ui
cargo test -p wkmp-dr
cargo test -p wkmp-ai

# Integration tests
cargo test --workspace

# Count test results
cargo test --workspace 2>&1 | grep "test result"
```

**Success Criteria:**
- All existing tests pass (no regressions)
- Test counts match baseline (no tests accidentally disabled)
- No new warnings in test output

**Specific Test Focus Areas:**
- **wkmp-dr:** 18 tests (14 API + 3 security + 1 doc)
- **wkmp-ap:** Database queue operations, passage queries
- **wkmp-ai:** File/passage/song/work CRUD operations
- **wkmp-common:** Database initialization, migrations

### Phase 4: Runtime Verification (30 minutes)

**Manual Verification Steps:**

```bash
# 1. Start wkmp-dr (database review module)
cargo run -p wkmp-dr
# Access: http://localhost:5725
# Test: Browse tables, filters, search

# 2. Start wkmp-ai (audio ingest)
cargo run -p wkmp-ai
# Access: http://localhost:5723
# Test: View import history, check database queries

# 3. Start wkmp-ap (audio player)
cargo run -p wkmp-ap
# Test: Queue operations, passage retrieval
```

**Verification Checklist:**
- [ ] Database connections establish successfully
- [ ] Queries execute without errors
- [ ] Read operations return expected data
- [ ] Write operations persist correctly
- [ ] No connection pool exhaustion warnings
- [ ] No "prepared statement" errors
- [ ] Response times similar to baseline

### Phase 5: Security Audit (15 minutes)

**Re-run cargo audit:**
```bash
cargo audit
```

**Expected Outcome:**
- RUSTSEC-2024-0363 RESOLVED ✅
- RUSTSEC-2023-0071 remains (MySQL - not used)
- RUSTSEC-2024-0436 remains (paste unmaintained warning)

**Success Criteria:**
- No HIGH or CRITICAL vulnerabilities
- SQLx-related vulnerability count reduced from 2 to 0 (for SQLite usage)

### Phase 6: Documentation Update (15 minutes)

**Files to Update:**

1. **wkmp-dr/README.md** (if version mentioned)
2. **project_management/change_history.md** (via `/commit`)
3. This upgrade plan → Archive after completion

**Commit Message:**
```
Upgrade SQLx from 0.7.4 to 0.8.1+

Addresses RUSTSEC-2024-0363 security vulnerability.

Changes:
- Split runtime-tokio-rustls into runtime-tokio + tls-rustls features
- Update workspace dependency to sqlx 0.8.1
- Verify all 18+ tests pass across all modules

Security Impact:
- Fixes RUSTSEC-2024-0363 (Binary Protocol Misinterpretation)
- No impact from RUSTSEC-2023-0071 (MySQL RSA attack - not used)

Testing:
- All unit tests pass
- All integration tests pass
- Manual verification: wkmp-dr, wkmp-ai, wkmp-ap functional

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>
```

---

## Rollback Plan

**If upgrade causes issues:**

```bash
# Revert the commit
git revert HEAD

# Clean and rebuild
cargo clean
cargo build --workspace

# Verify rollback successful
cargo test --workspace
```

**Rollback Risk:** VERY LOW (single commit, clean revert)

---

## Alternative: Incremental Approach

**If conservative approach preferred:**

1. **Week 1:** Upgrade wkmp-dr only (isolated module, comprehensive test suite)
2. **Week 2:** If wkmp-dr stable, upgrade wkmp-common + wkmp-ap
3. **Week 3:** Complete rollout with wkmp-ai + wkmp-ui

**Rationale:** Allows monitoring for subtle regressions in production-like environment.

**Recommendation:** NOT necessary based on risk assessment. Standard upgrade approach sufficient.

---

## Dependencies Affected

**Direct SQLx Users:**
- wkmp-common (database models, initialization)
- wkmp-ap (queue, passages, settings)
- wkmp-ui (authentication)
- wkmp-dr (table viewing, filters, search)
- wkmp-ai (file/passage/song/work management)

**Indirect Users:**
- wkmp-pd (uses wkmp-common models, but no direct SQLx usage)
- wkmp-le (likely uses wkmp-common, database not yet implemented)

---

## Post-Upgrade Monitoring

**Watch for:**
- Increased database connection errors
- Query timeout warnings
- Memory usage changes (statement caching differences)
- Test flakiness (connection pool behavior)

**Monitoring Period:** 1-2 weeks after merge

**Escalation:** If >2 production issues attributed to upgrade, consider rollback and deeper investigation.

---

## References

- **SQLx CHANGELOG:** https://github.com/launchbadge/sqlx/blob/main/CHANGELOG.md
- **RUSTSEC-2024-0363:** https://rustsec.org/advisories/RUSTSEC-2024-0363
- **SQLx 0.8.0 Release:** 2024-07-22 (70 PRs merged)
- **Current cargo audit output:** See section above

---

## Approval

**Recommendation:** PROCEED with standard upgrade approach (all modules simultaneously)

**Rationale:**
1. Security vulnerability fix (high priority)
2. Low residual risk (well-tested migration path)
3. Clean rollback available (single commit)
4. Comprehensive test coverage (high confidence)
5. Rust version compatible (1.89.0 >> 1.78.0 MSRV)

**Estimated Total Time:** 2-4 hours (including testing and verification)

**Blocker Status:** None identified

---

**Plan Status:** READY FOR IMPLEMENTATION
**Created:** 2025-11-01
**Author:** Claude Code (analysis and plan generation)
