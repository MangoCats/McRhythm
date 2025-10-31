# WKMP: Automatic TOML Config Directory Creation

## Problem Statement

Currently, when WKMP modules attempt to save TOML configuration files, they fail if the config directory doesn't exist:

```
WARN TOML write failed (database write succeeded):
  Configuration error: Temp file create failed: No such file or directory
```

**Observed in:** wkmp-ai when saving AcoustID API key to `~/.config/wkmp/wkmp-ai.toml`

## Required Behavior

**All 5 WKMP modules** (wkmp-ap, wkmp-ui, wkmp-pd, wkmp-ai, wkmp-le) MUST automatically create the necessary directory structure when saving TOML configuration files.

**Expected path:** `~/.config/wkmp/` (Linux/macOS) or `%APPDATA%\wkmp\` (Windows)

## Current State

**Modules affected:** All 5 (wkmp-ap, wkmp-ui, wkmp-pd, wkmp-ai, wkmp-le)

**Current TOML handling:**
- Path resolution: wkmp-common provides standard paths
- Directory creation: NOT currently implemented
- File write: Attempted without ensuring parent directory exists

## Questions for Analysis

1. **Where should directory creation logic live?**
   - Option A: wkmp-common (shared utility)
   - Option B: Each module implements independently
   - Option C: Part of TOML write operation

2. **When should directories be created?**
   - Option A: At module startup (proactive)
   - Option B: On first TOML write attempt (lazy)
   - Option C: Both (startup + fallback)

3. **What permissions should be set?**
   - Unix: 0700 (user-only) or 0755 (world-readable)?
   - Windows: Default or restricted?

4. **How should errors be handled?**
   - Fatal error if directory can't be created?
   - Log warning and continue (degraded mode)?
   - What if directory exists but isn't writable?

5. **Should this be specified in architecture or implementation docs?**
   - SPEC001-architecture.md (cross-cutting concern)?
   - IMPL003-project_structure.md (directory layout)?
   - REQ001-requirements.md (new requirement)?

## Acceptance Criteria

- [ ] All 5 modules create `~/.config/wkmp/` if missing
- [ ] Directory creation happens before TOML write attempts
- [ ] Appropriate permissions set (user-only on Unix)
- [ ] Graceful error handling (log but don't crash)
- [ ] Specification document updated
- [ ] Implementation consistent across modules

## Related Code

**Current implementations:**
- wkmp-ai: `src/api/settings.rs` (line 71-79) - TOML sync logic
- wkmp-common: `config` module - Path resolution utilities
- wkmp-common: `write_toml_config()` (line 419-449) - TOML atomic write

**Zero-config startup:** Already has directory creation for root folder (REQ-NF-030 through REQ-NF-037)

---

## After Analysis

**Analysis Date:** 2025-10-31
**Analysis Method:** `/think` Multi-Agent Workflow (8-Phase Analysis)
**Analysis Output:** See below (embedded - compact analysis)

### Executive Summary

**Problem:** `write_toml_config()` fails with "No such file or directory" when parent directory missing.

**Root Cause:** `fs::File::create(&temp_path)` at wkmp-common/src/config.rs:429 assumes parent directory exists.

**Solution:** Add `fs::create_dir_all()` to `write_toml_config()` before file creation.

**Impact:** All 5 modules (wkmp-ap, wkmp-ui, wkmp-pd, wkmp-ai, wkmp-le) benefit automatically (DRY principle).

### Questions Answered

**1. Where should directory creation logic live?**
- **Answer:** wkmp-common (Option A)
- **Rationale:** Follows DRY, consistent with `RootFolderInitializer` pattern, single point of change

**2. When should directories be created?**
- **Answer:** On TOML write attempt (Option B - lazy creation)
- **Rationale:** Simpler, only creates when needed, no proactive overhead

**3. What permissions should be set?**
- **Answer:** Unix 0700 (user-only) for directory, 0600 for file
- **Rationale:** Consistent with existing permission handling in `write_toml_config()`

**4. How should errors be handled?**
- **Answer:** Return error to caller (let caller decide)
- **Rationale:** Consistent with `write_toml_config()` behavior, caller knows context

**5. Should this be specified in architecture or implementation docs?**
- **Answer:** REQ001 (new requirement) + IMPL007 (graceful degradation)
- **Rationale:** Cross-cutting behavior (REQ001), implementation details (IMPL007)

### Specification Updates Required

**1. REQ001-requirements.md** (Tier 1 - Authoritative)
- Add new requirement: **[REQ-NF-038] TOML Config Directory Auto-Creation**
- Location: Non-functional requirements section (near REQ-NF-030 through REQ-NF-037)
- Content: "All modules MUST automatically create config directory when writing TOML files"

**2. IMPL007-graceful_degradation_implementation.md** (Tier 3 - Implementation)
- Update section on TOML write operations
- Document directory creation behavior in `write_toml_config()`
- Note permissions (Unix 0700 for directory, 0600 for file)

### Implementation

**File:** `wkmp-common/src/config.rs`
**Function:** `write_toml_config()` (line 419-449)
**Change:** Add parent directory creation before temp file creation

**Pseudocode:**
```rust
pub fn write_toml_config(config: &TomlConfig, target_path: &Path) -> Result<()> {
    // NEW: Ensure parent directory exists
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)?;
        set_unix_permissions_0700(parent)?; // Unix only
    }

    // EXISTING: Serialize, write temp file, set perms, rename
    // ... rest of function unchanged
}
```

**Benefit:** All 5 modules automatically gain this capability without modification.

### Risk Assessment

**Failure Risk:** Very Low

**Failure Modes:**
1. Directory creation fails (no permissions) - Probability: Low - Impact: Medium
   - Mitigation: Return error to caller, let them handle gracefully
   - Residual Risk: Very Low

2. Race condition (concurrent creates) - Probability: Very Low - Impact: None
   - Mitigation: `create_dir_all()` is idempotent, no issue
   - Residual Risk: None

**Quality:** High maintainability, follows established patterns, comprehensive solution

**Effort:** 30-45 minutes (add 5 lines of code + specifications + test)

### Next Steps

**To proceed with implementation:**
1. Update REQ001-requirements.md (add REQ-NF-038)
2. Update IMPL007-graceful_degradation_implementation.md
3. Modify `wkmp-common/src/config.rs::write_toml_config()`
4. Test with all 5 modules
5. Commit via `/commit` workflow

**Analysis Status:** ✅ **COMPLETE** - Ready for implementation
