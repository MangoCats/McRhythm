# /commit

### Task Description: Multi-step Commit with Multi-Developer Support

Perform the following tasks to commit changes in the WKMP repository. This workflow supports both single-developer and multi-developer environments by integrating remote changes and properly handling the commit hash tracking system.

**Execution Style:** Keep all output terse and factual. Only report when user input is needed or when errors occur. Silent continuation on success.

**User Input Convention:** Questions use `[Y/n]` or `[y/N]` format where capital letter indicates recommended choice. Type single letter (y/n/p/r/i/u) then press Enter.

**Change History Editing Policy:**
- **EXCLUSIVE EDITING AUTHORIZATION:** This /commit workflow is the ONLY automated process authorized to edit `project_management/change_history.md`
- DO NOT edit change_history.md during any other task, command, or workflow
- Automatic editing adds new entries and missing hashes ONLY
- Past entries are never automatically modified, even if inaccurate
- Users may manually edit change_history.md at any time

## Configuration Constants

**Timestamp Format:**
- ISO 8601 with space separator: `YYYY-MM-DD HH:MM:SS ±HHMM`
- Example: `2025-10-23 17:23:47 -0400`
- Precise to the second in local timezone with UTC offset
- PowerShell command: `$d = Get-Date; $d.ToString("yyyy-MM-dd HH:mm:ss ") + $d.ToString("zzz").Replace(":", "")`

**File Paths:**
- Change history file (relative): `project_management/change_history.md`
- Change history file (absolute): `c:\Users\Mango Cat\Dev\McRhythm\project_management\change_history.md`
- Workspace root: `c:\Users\Mango Cat\Dev\McRhythm`
- **IMPORTANT:** Use relative paths for all file operations (read_file, write, search_replace)

**Git Configuration:**
- Working branch: `dev`
- Archive branch: `archive`
- Remote: `origin`
- Main branch: `main` (reference only, not used in workflow)
- Commit message format: `{condensed description}` (no prefix)

**Change History File Structure:**
- Title: `# WKMP Change History`
- Separator: `---` (three dashes)

## Output Size Standards

**Commit Messages:**
- Target: <10 lines total (subject + body)
- Subject line: <72 characters (Git convention)
- Body: <8 lines, concise bullet points

**change_history.md Entries:**
- Target: <50 lines per entry
- Use bullet points for changes
- Link to detailed specs instead of repeating them

**Rationale:** Concise commits improve readability in git log and change_history.md while maintaining clarity.

## Workflow Steps

### -1. **Verify execution context:**

   a) **Check current directory:**
   * Run: `Get-Location` (PowerShell) or `pwd` (bash)
   * Expected: `c:\Users\Mango Cat\Dev\McRhythm`
   * If different:
     - State: "Wrong directory: [current]. Changing to workspace root."
     - Run: `cd "c:\Users\Mango Cat\Dev\McRhythm"`
     - Verify: `Get-Location` again

   b) **Verify git repository:**
   * Run: `git rev-parse --is-inside-work-tree`
   * If fails: State "Not in git repository" and STOP
   * If succeeds: Continue to step 0 silently

### 0. **Validate and update from remote:**

   a) **Validate staged hash:**
   * Run `git diff --cached --name-only` to list staged files
   * Check if output contains change history file (see Configuration Constants)
   * If the file is staged:
     - Run `git diff --cached` on change history file to see changes
     - Look for a line like `+## Date Time | Hash: [hash-value]`
     - Extract the hash value from that line
     - Get current HEAD hash: `git rev-parse HEAD`
     - Compare the extracted hash with HEAD hash:
       * If they match: Continue silently
       * If they don't match or extraction failed:
         - State: "Cleared stale staged hash"
         - Unstage: `git reset HEAD` on change history file
         - Verify unstaged: Run `git diff --cached --name-only` again
   * If the file is not staged: Continue silently

   b) **Check for remote updates:**
   * Run `git fetch origin` to check for remote changes (does not merge)
   * If fetch fails: Continue silently (may be offline)
   * Compare local with remote: `git rev-list HEAD..origin/dev --count`
   * If command fails or count is 0: Continue to step 1 silently
   * If count > 0:
     - State: "Remote has N new commits"
     - Ask: "Review remote first, or proceed? [P/r]"
       * Accept: "p", "proceed", "P" → Continue to step 1
       * Accept: "r", "review", "R" → STOP workflow

### 1. **Check the current branch and identify untracked files:**

   * Run `git status --untracked-files=all`
   * Verify current branch is `dev`
     - If wrong branch: State "Wrong branch: [branch]" and STOP
   * Check for untracked files in repository:
     - If found: List files, ask "Add and proceed? [Y/n]"
       * Accept: "y", "yes", "Y" → Run `git add .` (stages all untracked files in current directory and subdirectories)
       * Accept: "n", "no", "N" → STOP
     - If none: Continue to step 2 silently

### 2. **Stage changes and generate summary:**

   a) **Stage the repository:**
   * Verify in workspace root
   * Run `git add .` (stages all changes in repository and subdirectories)
   * Alternative for deletions: `git add -A .`

   b) **Generate diff and summary:**
   * Run `git diff --staged`
   * Analyze changes and create summary:
     - Effects of all changes since last commit
     - Key functional/behavioral modifications
     - Objectives and purpose
   * Max 1000 words, concise, no redundancy, no longer than necessary
   * Focus on significant effects

   c) **Check for previous commit's hash:**
   * If change history file doesn't exist: Skip to 2d
   * If exists: Read file, find most recent entry (first `##` heading after title)
   * Check if entry includes `| Hash:` suffix
   * If hash is present: Skip to step 2d (no hash needed)
   * If hash is missing:
     - Get hash: `git log -1 --format=%H HEAD`
     - Will add in step 2d before new entry

   d) **Update change history:**

   **Generate timestamp first:**
   * Run PowerShell command (see Configuration Constants for format):
   ```powershell
   $d = Get-Date; $d.ToString("yyyy-MM-dd HH:mm:ss ") + $d.ToString("zzz").Replace(":", "")
   ```
   * Example output: `2025-10-25 15:30:45 -0400`
   * Store this timestamp for use in entry creation below

   **IMPORTANT - Scope of Automatic Editing:**
   See "Automatic Editing Scope" in File Format Specification section. Key points:
   * Add new entry with current timestamp and summary for THIS commit only
   * Add hash to previous entry if missing (one-commit-lag system)
   * Never modify past entries

   **If file doesn't exist yet:**
   * Use `write` tool to create change history file (see Configuration Constants for path)
   * Content format (match project style):
     ```
     # WKMP Change History

     ---

     ## {TIMESTAMP}

     [Your generated summary here]
     ```
   * Use current timestamp (see Configuration Constants for format)
   * Do NOT include hash yet (commit doesn't exist)

   **If file exists and previous entry needs hash:**
   * First, use `search_replace` to add the hash to previous entry:
     - Find the most recent `##` header line (without `| Hash:`)
     - Replace it with same line but add ` | Hash: [40-char-hash]`
     - Use the hash obtained in step 2c

   **Then append new entry (for existing file):**
   * Use `search_replace` to insert at the top of the file (after title)
   * old_string: File header through first separator (see Configuration Constants for structure)
   * new_string: Same header + new entry with current timestamp + separator
   * Entry format: separator, blank line, ## {TIMESTAMP}, blank line, summary, blank line, separator
   * See Tool Usage Examples for complete example

   **Validation:**
   * Verify new entry added correctly (silent on success)

   e) **Stage updated change history:**
   * Run `git add` on change history file (see Configuration Constants)
   * This captures the new entry in the staging area before commit

   f) **Present to user:**
   * Show the new change history entry
   * Ask: "Edit before commit? [y/N]"
     - Accept: "y", "yes", "Y" → Show user:
       * "Edit the file, then type 'continue'"
       * Show file paths (see Configuration Constants)
       * Wait for 'continue'
       * Re-stage change history file
     - Accept: "n", "no", "N" → Proceed to step 3

### 3. **Perform the commit:**

   * Read the change history entry (as potentially edited by user)
   * Condense the summary to 25 words or less for commit message
   * Run `git commit -m "[condensed description]"` (no prefix, see Configuration Constants)
   * Verify: Run `git log -1 --oneline`
     - If success: Continue silently to next step
     - If failure: State error and STOP

### 3.5 **Integrate remote changes (if any exist):**

   * If step 0b indicated remote had new commits:
     - Run `git pull origin dev --no-rebase`
     - If fast-forward: State "Fast-forwarded" and continue
     - If auto-merge: State "Merged" and continue
     - If conflicts:
       * List conflicted files
       * State: "Resolve conflicts, then press Enter to continue"
       * Wait for Enter (or any input)
       * Continue to step 4

   * If no remote changes: Skip to step 4 silently

### 4. **Update and stage commit hash:**

   * Run `git rev-parse HEAD` to get full hash
   * Read change history file
   * Find most recent entry (first `##` heading after title)
   * Use `search_replace` to update header:
     - Old: `## {TIMESTAMP}` (keep the exact timestamp that was created in step 2d)
     - New: `## {TIMESTAMP} | Hash: [40-char-hash]` (append hash, don't modify timestamp)
   * Stage change history file
   * **Do NOT commit this change**
   * Proceed to step 5

### 5. **Report success:**

   * If successful:
     - Show the commit message used
     - Create a 100-word summary from the change_history.md entry by:
       * Taking first sentence or two as overview
       * Listing 3-5 key bullet points
       * Omitting detailed explanations
     - State: "Hash [8-char short hash] staged"

   * If failed:
     - State which step failed
     - One-sentence recovery guidance

### 5.5. **Synchronize archive branch with repository changes:**

   **Purpose:** Keep archive branch synchronized with all repository changes from working branch

   a) **Check if archive branch exists:**
   ```bash
   git branch --list archive
   ```
   - If not found: Skip this step silently (archive not yet created)
   - If found: Continue to b

   b) **Save current state:**
   ```bash
   # Stash any uncommitted changes (shouldn't be any, but safety measure)
   git stash --include-untracked --message "temp-before-archive-sync"
   ```
   - If stash fails (nothing to stash): Continue silently
   - If stash succeeds: Remember to pop later

   c) **Switch to archive branch:**
   ```bash
   git checkout archive
   ```
   - If checkout fails:
     * Report: "Warning: Archive branch sync skipped (checkout failed)"
     * Return to working branch: `git checkout dev`
     * Pop stash if created: `git stash pop`
     * Continue to file format specification section
   - If checkout succeeds: Continue to d

   d) **Merge repository changes from working branch:**
   ```bash
   # Get all changes from working branch (execute from repository root)
   git checkout dev -- .
   ```
   - This copies entire repository tree from working branch to archive branch
   - Note: Using `.` to select all files from workspace root

   e) **Commit to archive branch:**
   ```bash
   git add .
   git commit -m "Sync changes from working branch"
   ```
   - If no changes to commit: Continue silently
   - If commit fails:
     * Report: "Warning: Archive branch sync incomplete"
     * Continue to f anyway
   - If commit succeeds: Continue to f

   f) **Return to working branch:**
   ```bash
   git checkout dev
   ```
   - If checkout fails:
     * Report error
     * Manual recovery required
     * STOP workflow
   - If checkout succeeds: Continue to g

   g) **Restore stashed changes if any:**
   ```bash
   # Only if stash was created in step b
   git stash pop
   ```
   - If pop fails: Report warning but continue
   - If pop succeeds or no stash: Continue silently

   h) **Report sync status:**
   - If archive branch updated: State "Archive branch synchronized"
   - If archive branch didn't exist: (silently skipped, no message)
   - If sync failed: Already reported in previous steps

   **Error Handling:**
   - All archive sync failures are NON-BLOCKING
   - Working branch commit still succeeds even if archive sync fails
   - User can manually sync archive branch later if needed
   - Log warnings but continue workflow

### 6. **Prompt for push (optional):**

   **Purpose:** Offer to push both branches to remote in a single operation

   a) **Check current branch:**
   ```bash
   git branch --show-current
   ```
   - Expected: `dev` (working branch)
   - If different: Switch back to working branch before continuing

   b) **Report archive sync status:**
   - If archive branch was synced: State "Archive branch synchronized"
   - If archive branch didn't exist: State nothing (silently skipped)
   - If sync failed: Already reported in step 5.5

   c) **Prompt user:**
   ```
   Ready to push

   Push now? [Y/n]
   ```
   - Accept: "y", "Y", "yes", "Yes", "YES", "" (empty/Enter) → Continue to d
   - Reject: "n", "N", "no", "No", "NO" → Skip to step 7 (end workflow)

   d) **Push working branch:**
   ```bash
   git push origin dev
   ```
   - If successful: State "Pushed working branch"
   - If fails:
     * Report error message
     * Suggest: "Resolve conflicts or push manually"
     * Continue to e anyway (attempt archive push)

   e) **Push archive branch (if it exists and was synced):**
   ```bash
   # Only if archive branch exists
   git branch --list archive

   # If exists, push it
   git push origin archive
   ```
   - If archive branch doesn't exist: Skip silently
   - If successful: State "Pushed archive branch"
   - If fails:
     * Report error message
     * Suggest: "Push archive branch manually when ready"
     * Continue (non-blocking)

   f) **Verify on working branch:**
   ```bash
   git branch --show-current
   ```
   - Expected: `dev`
   - If different: Switch back to working branch
   ```bash
   git checkout dev
   ```

   g) **Report push status:**
   - If both pushed: State "Both branches pushed to remote"
   - If only working pushed: State "Working branch pushed. Archive branch skipped or failed."
   - If neither pushed: State "Push failed. See errors above."

### 7. **Final status and completion:**

   **Verify final state:**
   - Current branch: `dev` (must be on working branch)
   - Working tree: Clean except for staged change_history.md (with hash)
   - Ready for next commit

   **Report:**
   ```
   Workflow complete

   Next commit will include hash: [8-char short hash]
   ```

## change_history.md File Format Specification

**Automatic Editing Scope:**
- Automatic editing by /commit workflow is LIMITED to:
  * Adding new entry for current commit (date, time, summary)
  * Adding hash to previous entry if missing (one-commit-lag system)
- Automatic editing SHALL NOT modify or correct past entries
- Users may manually edit the file to correct errors or improve clarity
- Historical entries remain as originally written unless manually changed

**Required format for entries:**

```markdown
---

## {TIMESTAMP} | Hash: {HASH}

[Detailed summary of changes, 1000 words maximum...]

---

## {TIMESTAMP} | Hash: {HASH}

[Previous entry summary...]
```

See Configuration Constants for timestamp format. Hash is full 40-character SHA-1 (lowercase hexadecimal).

**Format rules:**
- Separator: `---` (three dashes) on its own line
- Header: `## {TIMESTAMP} | Hash: [40-char-hash]`
  - Before commit exists: `## {TIMESTAMP}` (without hash)
  - After commit: Add ` | Hash: [40-char-hash]`
- Timestamp format: See Configuration Constants
- Timestamp is created in step 2d and remains unchanged when hash is added in step 4
- Hash is full 40-character SHA-1 hash (lowercase hexadecimal)
- Entries in reverse chronological order (newest first)
- Summary follows header with blank line separator
- Optional: URLs or references can follow the summary

**If file doesn't exist:**
- Use write tool to create with first entry (no hash initially)
- Start with title, separator, then entry
- Follow the format specification above

**If file exists:**
- Match existing separator style (---)
- Maintain consistency within the file
- Append new entries at top (after title), keeping newest first

**Example of entry without hash (immediately after commit):**
```markdown
---

## {TIMESTAMP}

[Summary of what was just committed...]
```

**After hash is added and staged:**
```markdown
---

## {TIMESTAMP} | Hash: {HASH}

[Summary of what was just committed...]
```

## Notes on Multi-Developer Usage

**Handling change_history.md conflicts:**
- Keep entries in reverse chronological order (newest first)
- Each entry should eventually have format: `## {TIMESTAMP} | Hash: {HASH}`
- Your newest entry won't have its hash yet (will be added after commit)
- Other developers' entries should already have their hashes
- When merging: Simply order all entries by timestamp, newest first
- Preserve all entries from both branches

**The one-commit-lag system:**
- Each commit includes its entry in change_history.md but without its own hash
- Immediately after commit, the hash is added and staged
- The next commit will include the previous commit's hash
- This creates complete traceability without requiring git hooks or amend operations

**Before major milestones:**
- Consider making one final commit to include the last hash:
  `git commit {change_history_file} -m "Update change history with final commit hash"`
- This ensures all hashes are in version control before pushing

## Error Handling Guidelines

**If a git command fails:**
- Report the exact error message to the user
- Explain what the command was trying to do
- Suggest corrective action or manual intervention
- STOP the workflow unless error is explicitly handled

**If file operations fail:**
- Verify file paths are correct relative to workspace root (see Configuration Constants)
- Check if directory exists (create if needed)
- Report specific error (file not found, permission denied, etc.)
- Offer to retry or proceed manually

**If search_replace fails to find unique match:**
- Show user what you were searching for
- Show actual file content in that area
- Ask user for guidance or manual intervention
- Consider using more context in old_string to make it unique

**If user interaction times out:**
- After asking user a question, wait for explicit response
- Don't proceed with assumptions
- If ambiguous, ask for clarification

**Validation failures:**
- After each critical operation, verify it succeeded
- Check git status, file contents, or command output
- If verification fails, report immediately and stop

## Pre-Execution Checklist

Before running this command, ensure:
- [ ] Working directory is the workspace root (see Configuration Constants)
- [ ] Current branch is `dev`
- [ ] You have changes ready to commit
- [ ] You have network access (for git fetch/pull operations)
- [ ] Change history directory exists (relative to workspace root)
- [ ] File paths in this workflow are relative to workspace root

See Configuration Constants for all specific paths, branch names, and formats.

## Platform-Specific Behavior

**Windows Line Ending Normalization:**
- Git on Windows may show files as modified due to line ending differences (LF vs CRLF)
- Warning message: "in the working copy of 'file', LF will be replaced by CRLF the next time Git touches it"
- This is expected behavior with Git's `core.autocrlf` setting on Windows
- Files will be staged but may show no actual content changes in the diff
- These files will not appear in the final commit if only line endings differ
- This normalization ensures consistent line endings across platforms

## Success Criteria

A successful execution will result in:
1. ✓ Local changes committed with descriptive message
2. ✓ change_history.md updated with entry and previous hash (if applicable)
3. ✓ Current commit's hash added to change_history.md and staged
4. ✓ Remote changes integrated if they existed
5. ✓ Archive branch synchronized with repository changes (if archive branch exists)
6. ✓ Working tree clean except for staged change_history.md
7. ✓ On working branch (dev)
8. ✓ Optional: Both branches pushed to remote (if user confirmed)

## Workflow Execution Verification Checklist

**Before marking /commit workflow complete, verify ALL steps executed:**

- [ ] Step -1: Verified execution context (directory, git repository)
- [ ] Step 0: Validated and updated from remote (staged hash check, fetch, pull if needed)
- [ ] Step 1: Checked current branch and untracked files
- [ ] Step 2: Staged changes and generated summary (including change_history.md entry)
- [ ] Step 3: Performed the commit
- [ ] Step 3.5: Integrated remote changes (if any existed)
- [ ] Step 4: Updated and staged commit hash in change_history.md
- [ ] Step 5: Reported success (commit message + 100-word summary)
- [ ] Step 5.5: Synchronized archive branch with repository changes
- [ ] Step 6: **Prompted user to push (REQUIRED - don't skip this!)**
- [ ] Step 7: Verified final status and completion

**Common Skipped Steps:**
- ❌ Step 6 (push prompt) - Easy to forget after archive sync
- ❌ Step 0b (remote update check) - Can skip if not checking fetch results
- ❌ Step 5.5 (archive sync) - Non-blocking but should execute if branch exists

**If any step skipped:**
1. Note in workflow execution log
2. Complete missing steps before marking workflow done
3. Document deviation if needed

## Common Failure Scenarios and Resolutions

**Scenario: "Not a git repository"**
- Cause: Running from wrong directory
- Resolution: Navigate to repository root before running /commit

**Scenario: "Merge conflict in change_history.md"**
- Cause: Another developer committed while you were working
- Resolution: Follow step 3.5 conflict resolution guidance
- Keep entries in chronological order, preserve all entries

**Scenario: "No changes to commit"**
- Cause: All changes already committed or nothing staged
- Resolution: Make changes first, or check git status

**Scenario: "Cannot find entry to update hash"**
- Cause: File format doesn't match expected pattern
- Resolution: Manually verify format matches specification
- Ensure headers use ## and separator uses ---

**Scenario: "Archive branch sync failed"**
- Cause: Archive branch doesn't exist, checkout failed, or merge conflict
- Resolution: Non-blocking - working branch commit still succeeded
- Manual sync: Can sync archive branch later using /archive workflow
- Prevention: Ensure archive branch exists and is up to date

**Scenario: "Detached HEAD state"**
- Cause: Not on a branch
- Resolution: Checkout proper branch first (dev)

**Scenario: "Push rejected or failed"**
- Cause: Remote has commits not in local, network issues, or permission problems
- Resolution: Non-blocking - commit succeeded locally
- Recovery options:
  * If remote ahead: Pull changes first, then push manually
  * If network issue: Push manually when connection restored
  * If permission issue: Check repository access rights
- Note: Workflow always completes successfully even if push fails

## Documentation Reading Protocol

When `/commit` workflow needs to reference documentation:
- Read document summaries first (executive summaries)
- Load only relevant sections by line number
- Never load full SPEC###/REQ### documents
- Cite line numbers in commit messages when referencing specs

**Example:** "Implements REQ-CF-010 (lines 45-67): sample-accurate crossfade timing"

## Tool Usage Examples

**Note:** All file paths, timestamps, and branch names below should use values from Configuration Constants.

**Reading change history file:**
```
read_file: {change_history_file}
```

**Checking if file is staged:**
```
run_terminal_cmd: git diff --cached --name-only
```

**Creating new change_history file:**
```
write tool with file_path: {change_history_file}
contents: [See File Format Specification section]
```

**Adding hash to existing entry:**
```
old_string: ## {TIMESTAMP}
new_string: ## {TIMESTAMP} | Hash: {HASH}
```

**Prepending new entry to existing file:**
```
old_string: [File header through first separator - see Configuration Constants]

new_string: [Same header] + new entry + separator

Entry format:
---

## {TIMESTAMP}

[Summary text here...]

---

```
**Notes:**
- Use file structure from Configuration Constants
- Timestamp format from Configuration Constants
- Use 3 dashes (---) separator to match existing style

**Getting commit hash:**
```
run_terminal_cmd: git rev-parse HEAD
Output: {40-character SHA-1 hash}
```

**Staging change history file:**
```
run_terminal_cmd: git add {change_history_file}
```

**Committing:**
```
run_terminal_cmd: git commit -m "Brief description here"
```
See Configuration Constants for commit message format (no prefix).
