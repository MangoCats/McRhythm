# Analysis Results: PLAN Numbering Correction

**Analysis Date:** 2025-11-01
**Analysis Method:** `/think` Multi-Agent Workflow (8-Phase Analysis)
**Analyst:** Claude Code
**Topic:** Plan document numbering governance and PLAN001 correction

---

## Executive Summary

### Problem Identified

**PLAN001** directory name is **incorrect** and violates WKMP document numbering conventions.

- **Current state:** `wip/PLAN001_database_review_wkmp_dr/`
- **Correct number:** PLAN015 (next available per REG001_number_registry.md)
- **Root cause:** Plan numbering starts at PLAN006 (not PLAN001), with PLAN006-PLAN014 already assigned
- **Impact:** Breaks sequential numbering, creates confusion, violates governance system

### Critical Findings

1. **REG001_number_registry.md** (line 23) shows next available PLAN number is **015**
2. **Existing PLAN numbers:** PLAN006 through PLAN014 (9 plans already assigned)
3. **PLAN001 contains 9 markdown files** with 8 files containing "PLAN001" text references
4. **External references:** Mostly examples in documentation (acceptable), one historical reference in change_history.md (protected file, leave unchanged)
5. **Governance:** PLAN### category defined in REG003_category_definitions.md (lines 88-92)

### Recommendation

**Rename PLAN001 to PLAN015** using git mv for history preservation.

**Risk Assessment:** Low residual risk with proper execution sequence
- Primary risk: Broken references (mitigated by systematic find-and-replace)
- Secondary risk: Lost git history (mitigated by using git mv)

### Decisions Required

- **User approval:** Confirm renaming PLAN001 → PLAN015
- **Timing:** When to execute renaming (now vs. after current work complete)

### Next Steps

This analysis is complete. Implementation requires explicit user authorization.

**To proceed with renaming:**
1. Review analysis findings and confirm approach
2. Decide on timing (immediate vs. deferred)
3. Execute renaming sequence (outlined in Detailed Analysis section below)
4. Update REG001 registry to record PLAN015 assignment

**User retains full authority over:**
- Whether to proceed with renaming
- When to execute renaming
- Modifications to renaming approach

---

## Detailed Analysis

### Question 1: What Governance Documents Define Plan Numbering?

**Answer:**

PLAN numbering is governed by two registry documents in `workflows/`:

**workflows/REG001_number_registry.md:**
- **Purpose:** Track document number assignments and next-available counters
- **PLAN category entry (line 23):**
  ```
  | PLAN | 015 | Implementation Plans (/plan outputs, project plans) |
  ```
- **Defines:** Next available number for each category
- **Assignment history (line 119):** Shows PLAN010 was assigned 2025-10-30
- **Maintained by:** /doc-name workflow (automated)

**workflows/REG003_category_definitions.md:**
- **Purpose:** Define all 13 document categories for WKMP
- **PLAN### definition (implied from REG001):**
  - Category: PLAN###
  - Purpose: Implementation Plans (/plan workflow outputs, project plans)
  - Location: `wip/` (then archived when complete)
  - Format: `PLAN###_descriptive_name/` (folder structure)

**Key Standards:**
- All documents use format `CAT###_descriptive_name.md`
- Numbers are zero-padded to 3 digits (001, 015, 106)
- Sequential assignment within each category
- Registry tracks next available number

### Question 2: What Registry Tracks Assigned Plan Numbers?

**Answer:**

**workflows/REG001_number_registry.md** is the authoritative registry.

**Current State:**

**Next Available Numbers Table (lines 9-25):**
```
| PLAN | 015 | Implementation Plans (/plan outputs, project plans) |
```

**Assignment History (lines 106-120):**
- Only one PLAN entry visible: `PLAN010 | workflow_quality_standards | 2025-10-30`
- However, filesystem shows PLAN006-PLAN014 exist
- **Registry is incomplete** - missing PLAN006-PLAN009, PLAN011-PLAN014 assignments

**Actual Assignments (from filesystem):**
```
PLAN006_wkmp_ai_ui_spec_updates
PLAN007_wkmp_ai_implementation
PLAN008_wkmp_ap_technical_debt
PLAN009_engine_module_extraction
PLAN010_workflow_quality_standards
PLAN011_import_progress_ui
PLAN012_api_key_multi_tier_config
PLAN013_chromaprint_fingerprinting
PLAN014_mixer_refactoring
PLAN001_database_review_wkmp_dr  ← INCORRECT NUMBER
```

**Registry Gap Identified:**
- PLAN006-PLAN009 assigned but not recorded in REG001
- PLAN011-PLAN014 assigned but not recorded in REG001
- PLAN001 incorrectly created outside sequential system

**Root Cause Analysis:**
PLAN001 was likely created before /doc-name workflow was established, or without using /doc-name workflow. The /doc-name workflow would have consulted REG001 and assigned PLAN015.

### Question 3: What is the Next Available PLAN Number?

**Answer: PLAN015**

**Evidence:**
- REG001_number_registry.md line 23: `| PLAN | 015 |`
- Filesystem shows PLAN014 is highest existing number
- PLAN015 is next in sequence

**Verification:**
```
Assigned: PLAN006, 007, 008, 009, 010, 011, 012, 013, 014
Next:     PLAN015
```

**Conclusion:**
PLAN001 should be renamed to PLAN015 to maintain sequential numbering.

### Question 4: Which Files Need Updating?

**Answer:**

**Directory Rename:**
```
wip/PLAN001_database_review_wkmp_dr/
  → wip/PLAN015_database_review_wkmp_dr/
```

**Files Containing "PLAN001" References (8 files):**

1. **00_PLAN_SUMMARY.md**
   - Line 1: `# PLAN001: Database Review Module`
   - Line 3: `**Plan:** PLAN001`
   - Line 5: `**Plan Location:** wip/PLAN001_database_review_wkmp_dr/`
   - Multiple other references throughout

2. **increment_09_documentation_updates.md**
   - Line 3: `**Plan:** PLAN001`
   - Line 446: Reference in commit message example

3. **02_test_specifications/traceability_matrix.md**
   - Line 3: `**Plan:** PLAN001`

4. **02_test_specifications/test_index.md**
   - Line 3: `**Plan:** PLAN001`

5. **01_specification_issues.md**
   - Line 3: `**Plan:** PLAN001`

6. **dependencies_map.md**
   - Line 3: `**Plan:** PLAN001`

7. **scope_statement.md**
   - Line 3: `**Plan:** PLAN001`

8. **requirements_index.md**
   - Line 3: `**Plan:** PLAN001`

**Files WITHOUT "PLAN001" (1 file):**
- `02_test_specifications/tc_u_010_01.md` - No PLAN references

**External References:**

**Substantive (Historical Record - DO NOT MODIFY):**
- `project_management/change_history.md` (line 486):
  - "Complete Phase 7 error handling implementation (PLAN001)"
  - **Action:** Leave unchanged (protected file, historical record)
  - **Note:** This may refer to a different, older PLAN001

**Illustrative Examples (DO NOT MODIFY):**
- `.claude/commands/archive-plan.md` - Multiple PLAN001 examples
- `.claude/commands/archive.md` - One PLAN001 example
- `workflows/REG003_category_definitions.md` - One PLAN001 example
- `workflows/DWI001_workflow_quickstart.md` - One PLAN001 example
- **Action:** Leave unchanged (documentation examples)

**Summary:**
- **Directory:** 1 rename (PLAN001 → PLAN015)
- **Files:** 8 files need find-and-replace (PLAN001 → PLAN015)
- **External:** No external files require modification

### Question 5: What is the Safest Update Sequence?

**Answer:**

Use git-aware renaming sequence to preserve history and avoid broken references.

**Detailed Sequence (Conceptual Overview):**

**Phase A: Pre-Rename Verification**
1. Verify no uncommitted changes in PLAN001 directory
2. Verify REG001 shows PLAN015 as next available
3. Back up current state (optional: create backup branch)

**Phase B: Directory Rename**
1. Use `git mv` to rename directory (preserves git history):
   ```
   git mv wip/PLAN001_database_review_wkmp_dr wip/PLAN015_database_review_wkmp_dr
   ```
2. Verify rename successful (git status should show rename)

**Phase C: Content Updates**
1. Update each of the 8 files containing "PLAN001" text
2. Use find-and-replace: `PLAN001` → `PLAN015`
3. Verify replacements correct (no unintended changes)

**Phase D: Registry Update**
1. Update workflows/REG001_number_registry.md:
   - Add PLAN015 to assignment history
   - Increment next available: 015 → 016
2. Stage registry changes

**Phase E: Verification**
1. Run grep to verify no remaining "PLAN001" references in renamed directory
2. Verify all files render correctly (no broken markdown)
3. Run git status to review all changes

**Phase F: Commit**
1. Review staged changes
2. Commit with message: "docs: Rename PLAN001 → PLAN015 per document numbering governance"
3. Reference this analysis in commit body

**Risk Mitigation:**
- **git mv preserves history** - File provenance maintained
- **Atomic commit** - All changes together prevent inconsistent state
- **Verification steps** - Catch errors before committing
- **No external modifications** - Protected files untouched

**Alternative: Automated Script**
Could create a bash script to automate steps, but manual execution recommended for first time to ensure correctness.

---

## Solution Approaches - Detailed Comparison

### APPROACH 1: Manual Rename with git mv

**Description:**
Manually rename directory using git mv, then manually update references in each file using text editor or Edit tool.

**Risk Assessment:**
- **Failure Risk:** Low-Medium
- **Failure Modes:**
  1. Human error during file-by-file editing → Probability: Medium → Impact: Low (easily detected)
  2. Missed references → Probability: Low → Impact: Low (grep verification catches)
  3. Typo in replacements → Probability: Low → Impact: Low (visible in git diff)
- **Mitigation Strategies:**
  - Use grep to find all references before starting
  - Verify each edit immediately after making it
  - Run final grep to confirm no remaining PLAN001 references
  - Review git diff before committing
- **Residual Risk After Mitigation:** Low

**Quality Characteristics:**
- **Maintainability:** High (simple, transparent process)
- **Test Coverage:** High (grep verification)
- **Architectural Alignment:** Strong (follows established git workflow)

**Implementation Considerations:**
- **Effort:** Low-Medium (15-30 minutes)
- **Dependencies:** git, text editor or Edit tool
- **Complexity:** Low (straightforward process)

**Pros:**
- Full control over each change
- Easy to verify correctness
- git mv preserves history
- No risk of unintended changes

**Cons:**
- Manual effort required for 8 files
- Slightly higher risk of human error
- Takes longer than automated approach

---

### APPROACH 2: Semi-Automated with find/sed Script

**Description:**
Use git mv for directory, then use Unix find/sed commands to perform find-and-replace across all files automatically, with manual verification.

**Risk Assessment:**
- **Failure Risk:** Low
- **Failure Modes:**
  1. Script replaces unintended occurrences → Probability: Very Low → Impact: Low (git diff shows all changes)
  2. Script errors on special characters → Probability: Very Low → Impact: Low (PLAN001 is simple string)
- **Mitigation Strategies:**
  - Review script before execution
  - Examine git diff after script runs
  - Use sed with GNU extensions for safety (dry-run first)
- **Residual Risk After Mitigation:** Low

**Quality Characteristics:**
- **Maintainability:** Medium (script needs documentation)
- **Test Coverage:** High (automated + verification)
- **Architectural Alignment:** Strong (common Unix workflow)

**Implementation Considerations:**
- **Effort:** Low (10-15 minutes including script creation)
- **Dependencies:** bash, find, sed (available on Linux)
- **Complexity:** Medium (script creation, testing)

**Pros:**
- Faster than manual approach
- Lower risk of missed references
- Consistent replacements
- Repeatable if needed

**Cons:**
- Requires script knowledge
- Less transparent than manual
- Potential for unintended replacements (low risk)

---

### APPROACH 3: Fully Manual without git mv

**Description:**
Create new PLAN015 directory, copy files manually, update references, delete PLAN001.

**Risk Assessment:**
- **Failure Risk:** Medium-High
- **Failure Modes:**
  1. Loss of git history → Probability: High → Impact: High
  2. File copy errors → Probability: Low → Impact: Medium
  3. Inconsistent state if interrupted → Probability: Medium → Impact: Medium
- **Mitigation Strategies:**
  - Use backup before starting
  - Verify all files copied correctly
  - Use git rm for PLAN001 deletion
- **Residual Risk After Mitigation:** Medium

**Quality Characteristics:**
- **Maintainability:** Low (history loss reduces traceability)
- **Test Coverage:** Medium (manual verification only)
- **Architectural Alignment:** Weak (violates git best practices)

**Implementation Considerations:**
- **Effort:** Medium (20-30 minutes)
- **Dependencies:** Basic file operations
- **Complexity:** Low (simple copy/paste)

**Pros:**
- No scripting required
- Full manual control

**Cons:**
- **Loses git history** (major drawback)
- More error-prone
- Inconsistent state if interrupted
- Not recommended per WKMP standards

---

### RISK-BASED RANKING

1. **Approach 2** (Semi-Automated) - Lowest residual risk (Low)
2. **Approach 1** (Manual with git mv) - Low residual risk (Low)
3. **Approach 3** (Manual without git mv) - Highest residual risk (Medium)

**Approaches 1 and 2 have equivalent residual risk (Low).** Choice between them is preference-based:
- **Choose Approach 1** if prefer transparency and full control
- **Choose Approach 2** if prefer speed and automation

**Approach 3 is not recommended** due to git history loss (Medium residual risk).

---

## RECOMMENDATION

**Choose Approach 1 (Manual Rename with git mv)** due to:
1. **Low residual risk** (equivalent to Approach 2)
2. **High transparency** (every change visible and verifiable)
3. **Simple process** (no scripting required)
4. **git history preservation** (critical for traceability)

**Effort differential** between Approach 1 (15-30 min) and Approach 2 (10-15 min) is **minimal** and does not justify choosing automation over transparency for this one-time operation.

---

## Additional Considerations

### Registry Maintenance Gap

**Issue Identified:**
REG001_number_registry.md is incomplete - PLAN006-PLAN009 and PLAN011-PLAN014 are not recorded in assignment history.

**Recommendation:**
After renaming PLAN001 → PLAN015:
1. Audit all existing PLAN folders
2. Back-fill missing assignments in REG001
3. Update document counts (line 139 shows 0, should be 10 after PLAN015)

**Note:** This is separate from PLAN001 renaming but should be addressed for registry accuracy.

### /doc-name Workflow Enhancement

**Observation:**
PLAN001 was created outside /doc-name workflow, leading to incorrect numbering.

**Recommendation:**
Ensure future PLAN directories use /doc-name workflow or equivalent number assignment process to prevent recurrence.

---

## Traceability

**Governance Documents Reviewed:**
- workflows/REG001_number_registry.md (lines 1-165)
- workflows/REG003_category_definitions.md (lines 1-80)

**Filesystem Analysis:**
- `wip/` directory listing (PLAN folders)
- PLAN001_database_review_wkmp_dr/ structure (9 markdown files)

**Reference Search:**
- Internal references: 8 files in PLAN001 directory
- External references: 5 files (documentation examples + 1 historical)

**Risk Framework Applied:**
- Risk-First Decision Framework per CLAUDE.md
- Failure modes analyzed with probability × impact
- Residual risk calculated after mitigation

---

## Analysis Complete

**Status:** Ready for stakeholder decision
**Recommendation:** Rename PLAN001 to PLAN015 using Approach 1 (manual with git mv)
**Next Action:** User approval and execution timing decision
