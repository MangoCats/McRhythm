# /archive-plan

### Task Description: Batch Archive Completed Plan Working Folders

Archive work-in-progress folders associated with a completed plan by removing them from working branch while preserving them in the archive branch through automatic git sync.

**Execution Style:** Interactive with user confirmation.

**Key Insight:** The archive branch automatically syncs via `/commit` Step 6.5. "Archiving" simply means removing folders from working branch - they're already preserved in archive branch history through automatic sync.

---

## Usage

```bash
/archive-plan <plan_number>
```

**Parameters:**
- `plan_number` (required): Plan number in any format (PLAN001, plan001, 001)

**Examples:**
```bash
/archive-plan PLAN001      # Archives wip/PLAN001_*/ folder
/archive-plan plan002      # Archives wip/PLAN002_*/ folder
/archive-plan 003          # Archives wip/PLAN003_*/ folder
```

---

## Configuration Constants

**Git Configuration:**
- Working branch: `dev`
- Archive branch: `archive`
- Remote: `origin`

**File Pattern:**
- Detection: `PLAN{###}_*/` folders (case insensitive, exactly 3 digits)
- Directory: `wip/`
- Example: `PLAN001_audio_player_core/`, `PLAN002_crossfade_engine/`

**Archive Index:**
- Path: `workflows/REG002_archive_index.md`
- Section: "### Archived Plan Documents"

---

## Output Size Standards

**Batch Archive Summary:**
- Target: <100 lines for entire batch operation
- List archived plans (ID + title)
- Summary statistics (count, total size reduction)
- Avoid detailed plan contents

**Per-Plan Entries in Archive Index:**
- Target: <30 lines per plan
- Format: PLAN###, title, date, status, retrieval method

---

## Simplified Workflow

### Step 1: Normalize and Detect Plan Folder

**Normalize input:**
```python
# PLAN001 → 001
# plan001 → 001
# 001 → 001
normalized = input.lower().replace("plan", "").zfill(3)
```

**Detect matching folder:**
```bash
# Pattern: PLAN{normalized}_*/ (case insensitive)
ls -d wip/PLAN001_*/  # Example for PLAN001
```

**Count folder contents:**
- Total files in folder (recursive)
- Total lines across all files
- Folder size summary

**If no folder found:**
- Report: "No plan folder found for PLAN{normalized} in wip/"
- Show available plan folders in wip/
- STOP workflow

**If folder found:**
- Continue to Step 2

---

### Step 2: Confirmation Prompt

**Display:**
```
Archive Plan Work-In-Progress Folder

Plan: PLAN{normalized}
Folder: wip/PLAN{normalized}_[feature_name]/

Contents:
  00_PLAN_SUMMARY.md                ({lines} lines)
  01_specification_issues.md         ({lines} lines)
  02_test_specifications/            ({files} files, {lines} lines)
  requirements_index.md              ({lines} lines)
  ...

Total: {total_files} files, {total_lines} lines

This will:
- Remove entire PLAN{normalized}_*/ folder from wip/ (working branch)
- Preserve complete folder in archive branch via automatic sync
- Update archive index with folder entry
- Full git history preserved

Confirm PLAN{normalized} is complete and ready to archive? [y/N]
```

**Accept:**
- "y", "yes", "Y" → Continue to Step 3
- "n", "no", "N", or any other input → STOP with "Archival cancelled"

---

### Step 3: Sync to Archive Branch (Safety First)

**Purpose:** Ensure archive branch has complete folder before deletion

```bash
# Switch to archive branch
git checkout archive

# Get latest from working branch
git checkout dev -- wip/

# Stage and commit
git add wip/
git commit -m "Sync wip/ before archiving PLAN{normalized} folder"

# Return to working branch
git checkout dev
```

**Error Handling:**
- If archive branch doesn't exist: Report "Archive branch not found" and STOP
- If checkout fails: Report error and STOP
- If no changes to commit: Continue silently (already synced)
- If commit fails: Report error and STOP

**Result:** Complete folder safely preserved in archive branch before deletion

---

### Step 4: Remove Folder from Working Branch

```bash
# Remove entire folder
git rm -r wip/PLAN{normalized}_*/
```

**Result:** Entire folder staged for deletion from working branch

**Error Handling:**
- If git rm fails: Report error, show which files failed, STOP
- If folder doesn't exist: Report error and STOP

---

### Step 5: Update Archive Index (Folder Entry)

**Read archive index:**
```bash
read_file: workflows/REG002_archive_index.md
```

**Find or create section:**
- Section header: "### Archived Plan Documents"
- If not found: Create section after "## Archived Documents by Category"

**Create folder entry:**
```markdown
#### PLAN{normalized}: {Plan_Title} ({date})

Archived complete implementation plan folder from wip/PLAN{normalized}_[feature_name]/

**Contents:** {file_count} files, {total_lines} lines
- 00_PLAN_SUMMARY.md
- 01_specification_issues.md
- 02_test_specifications/ (folder)
- requirements_index.md
- [additional files...]

**Reason:** Plan implementation completed

**Restoration:**
```bash
# Restore complete folder
git checkout archive -- wip/PLAN{normalized}_*/

# Or view archive branch
git checkout archive
cd wip/PLAN{normalized}_*/
# Browse files normally
git checkout dev  # Return to working branch
```

**Retrieval (individual files):**
```bash
# View specific file
git show archive:wip/PLAN{normalized}_[feature]/00_PLAN_SUMMARY.md

# List all files in folder
git ls-tree -r archive -- wip/PLAN{normalized}_*/
```

---
```

**Insert entry:**
- Use `search_replace` to add entry after section header (reverse chronological order)
- Stage archive index

**Error Handling:**
- If section not found: Create section with header
- If update fails: Warn user but continue (non-blocking)

---

### Step 6: Commit via /commit

**Stage changes:**
- Folder deletion (already staged from Step 4)
- Archive index update (staged in Step 5)

**Use /commit workflow:**
```
# All changes staged, ready for /commit workflow
# /commit will be invoked automatically or prompt user
```

**Note:** `/commit` Step 6.5 will automatically sync the deletions to archive branch

**Result:**
- Working branch: Folder removed ✓
- Archive branch: Complete folder preserved ✓
- Archive index: Updated with folder entry and retrieval commands ✓
- Change history: Updated via `/commit` ✓

---

## Retrieval

**View archived plan folder contents:**
```bash
# List all files in archived plan
git ls-tree -r archive -- wip/PLAN001_*/

# View specific file
git show archive:wip/PLAN001_[feature]/00_PLAN_SUMMARY.md
```

**Restore complete plan folder:**
```bash
# Restore entire folder at once
git checkout archive -- wip/PLAN001_*/

# Or checkout archive branch to browse
git checkout archive
cd wip/PLAN001_*/
ls  # All files accessible normally
# When done: git checkout dev
```

**Restore single file:**
```bash
# Use git show with output redirection
git show archive:wip/PLAN001_[feature]/00_PLAN_SUMMARY.md > wip/PLAN001_[feature]/00_PLAN_SUMMARY.md
```

---

## Why This Approach Works

**Safety:**
- Archive sync happens BEFORE deletion (Step 3)
- Complete folder preserved before removal
- Atomic operation (all or nothing)
- No risk of data loss

**Simplicity:**
- No staging directory needed
- No complex branch manipulation
- Just: sync → delete folder → commit
- Single operation for entire plan

**Efficiency:**
- Archives entire plan folder in one operation
- Single entry in archive index
- One commit via `/commit` workflow
- Removes 500-3000 lines per plan from wip/

**Leverage Existing Infrastructure:**
- Uses `/commit` Step 6.5 for archive sync
- Follows DRY principle (single source of truth for commits)
- Integrates with existing archive index format

---

## Expected Impact

**Context Window Reduction:**
- PLAN001: ~2400 lines removed from wip/
- PLAN002: ~1800 lines removed from wip/
- PLAN003: ~2200 lines removed from wip/
- PLAN004: ~2600 lines removed from wip/
- Total reduction: ~9000 lines (85-90% of wip/ content)

**Before:**
```
wip/
├── PLAN001_audio_player_core/
│   ├── 00_PLAN_SUMMARY.md (650 lines)
│   ├── 01_specification_issues.md (700 lines)
│   ├── 02_test_specifications/ (3 files, 500 lines)
│   └── requirements_index.md (550 lines)
├── PLAN002_crossfade_engine/
│   ├── 00_PLAN_SUMMARY.md (450 lines)
│   ├── requirements_index.md (400 lines)
│   ├── test_specifications.md (600 lines)
│   └── README.md (350 lines)
├── PLAN003_program_director_algorithm/
│   ├── 00_PLAN_SUMMARY.md (600 lines)
│   ├── 01_specification_issues.md (550 lines)
│   ├── requirements_index.md (450 lines)
│   ├── test_specifications.md (400 lines)
│   └── README.md (200 lines)
├── PLAN004_ui_microservice/
│   ├── 00_PLAN_SUMMARY.md (700 lines)
│   ├── 01_specification_issues.md (600 lines)
│   ├── 02_test_specifications/ (7 files, 800 lines)
│   └── requirements_index.md (500 lines)
├── auto_archive_completed_plans_spec.md (400 lines)
├── microservice_communication_patterns.md (800 lines)
├── sse_implementation.md (170 lines)
└── README.md (50 lines)

Total: ~11,420 lines
```

**After archiving all completed plans:**
```
wip/
├── auto_archive_completed_plans_spec.md (400 lines)
├── microservice_communication_patterns.md (800 lines)
├── sse_implementation.md (170 lines)
└── README.md (50 lines)

Total: ~1,420 lines
Context reduction: 88%
```

---

## Error Scenarios

**Scenario: "No plan folder found for PLAN###"**
- Cause: Plan number incorrect or folder already archived
- Resolution: Check plan number, check archive index
- Recovery: List available plan folders in wip/

**Example:**
```
No plan folder found for PLAN999 in wip/

Searched for folders matching: PLAN999_*/
Found: 0 folders

Possible reasons:
- Plan number incorrect (check workflows/REG001_number_registry.md)
- Folder already archived (check workflows/REG002_archive_index.md)
- Plan folder in different location

Available plan folders in wip/:
- PLAN001_audio_player_core/
- PLAN002_crossfade_engine/
- PLAN003_program_director_algorithm/
- PLAN004_ui_microservice/
```

**Scenario: "Archive branch not found"**
- Cause: Archive branch doesn't exist yet
- Resolution: Create archive branch first
- Recovery: Folder remains in working branch

**Scenario: "Archive sync failed"**
- Cause: Checkout failed or commit failed in Step 3
- Resolution: Report error, STOP before deletion
- Recovery: Manual sync or retry
- **Critical:** Folder NOT deleted if sync fails

**Scenario: "git rm failed"**
- Cause: Folder couldn't be removed
- Resolution: Report error, STOP
- Recovery: Fix issues, retry command

**Scenario: "User cancelled"**
- Cause: User responded "n" to confirmation
- Resolution: Clean exit, no changes
- Recovery: N/A (intentional cancellation)

---

## Comparison with File-Based Approach

**Old Approach (individual files):**
```
1. Detect plan###_*.md files
2. Archive multiple individual files
3. Update index with multiple entries
```

**Problems:**
- Only worked for flat files, not folder structures
- Plan folders (PLAN###_*/) were in wrong location
- Couldn't archive subfolder structures (02_test_specifications/)

**New Approach (folder-based):**
```
1. Detect PLAN###_*/ folder in wip/
2. Archive entire folder structure
3. Single index entry for complete plan
```

**Benefits:**
- ✅ Works with /plan output structure
- ✅ Archives complete plan including subfolders
- ✅ Correct location (wip/)
- ✅ Single operation for entire plan
- ✅ Simpler index entries
- ✅ Aligned with /plan workflow

---

## Integration with /plan and /commit

**This workflow integrates with:**

1. **/plan workflow:**
   - `/plan` creates folders in `wip/PLAN###_[feature]/`
   - `/archive-plan` archives from same location
   - Consistent folder naming convention

2. **/commit workflow:**
   - Stages folder deletion and index update
   - Uses `/commit` for proper change history
   - `/commit` Step 6.5 syncs to archive branch

**Result:** Complete lifecycle management of plan documents:
- Create (via `/plan`) → Work → Complete → Archive (via `/archive-plan`)

---

## Success Criteria

A successful folder archival results in:
1. ✓ Complete plan folder removed from working branch
2. ✓ Complete folder preserved in archive branch (verified in Step 3)
3. ✓ Archive index updated with folder entry and retrieval commands
4. ✓ `/commit` workflow used (not manual commits)
5. ✓ Change history updated
6. ✓ Full git history preserved
7. ✓ Context window reduction ≥2000 lines per plan
8. ✓ All files retrievable via archive index commands
9. ✓ No data loss (all files accessible from archive branch)
10. ✓ Folder structure preserved (including subfolders)

---

## Usage Examples

**Archive PLAN001 folder:**
```bash
/archive-plan PLAN001
# Shows: PLAN001_audio_player_core/, 5 files + subfolders, ~2400 lines
# Confirm: y
# Result: wip/ reduced by entire folder
```

**Archive PLAN002 folder:**
```bash
/archive-plan 002
# Shows: PLAN002_crossfade_engine/, 4 files, ~1800 lines
# Confirm: y
# Result: wip/ reduced by entire folder
```

**Check available plans before archiving:**
```bash
# List wip/ contents
ls -d wip/PLAN*/

# Or use grep to find plan folders
find wip/ -maxdepth 1 -type d -name "PLAN*"
```

---

## Documentation Reading Protocol

For batch archival:
- Read only plan summaries (00_PLAN_SUMMARY.md files)
- Extract: Plan ID, title, status, completion date
- Do NOT load full plans or detailed sections
- Archive index should list plans, not describe them in detail

---

## Related Documentation

- **Plan Workflow:** `.claude/commands/plan.md` (creates PLAN### folders in wip/)
- **Archive Workflow:** `.claude/commands/archive.md` (single document archival)
- **Commit Workflow:** `.claude/commands/commit.md` (Step 6.5 archive sync)
- **Archive Index:** `workflows/REG002_archive_index.md`
- **Specification:** `wip/auto_archive_completed_plans_spec.md`
- **WKMP Documentation:** `docs/` (complete technical specifications)
- **Microservices Architecture:** WKMP uses 5 independent HTTP servers (AP, UI, PD, AI, LE)
