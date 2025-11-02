# /archive

### Task Description: Archive Historical Document

Archive a document by removing it from the working branch while preserving it in the archive branch through automatic git sync.

**Execution Style:** Interactive with user confirmation.

**Key Insight:** The archive branch automatically syncs via `/commit` Step 6.5. "Archiving" simply means removing files from working branch - they're already preserved in archive branch history through automatic sync.

---

## Usage

```bash
/archive <document_path> [reason]
```

**Parameters:**
- `document_path` (required): Path to document relative to workspace root (e.g., `wip/old_analysis.md`)
- `reason` (optional): Brief reason for archival (e.g., "Completed", "Superseded by PLAN002", "Historical")

**Examples:**
```bash
/archive wip/plan001_audit.md "PLAN001 completed"
/archive docs/design/old_approach.md "Superseded by new design"
/archive wip/analysis.md
```

---

## Configuration Constants

**Git Configuration:**
- Working branch: `dev`
- Archive branch: `archive`
- Remote: `origin`

**Archive Index:**
- Path: `workflows/REG002_archive_index.md`
- Format: Markdown table with document info and retrieval commands

---

## Output Size Standards

**Archive Index Entries:**
- Target: <50 lines per archived document entry
- Include: document ID, title, date, category, reason, retrieval instructions
- Avoid: Repeating full document content

**Archive Commit Messages:**
- Target: <10 lines
- Format: "Archive [DOC-ID]: [title]" + brief rationale

**Rationale:** Concise index entries keep REG002_archive_index.md manageable as archive grows.

---

## Documentation Reading Protocol

When archiving documents:
- Read document summary/first 100 lines to extract metadata
- Do NOT load full document into context
- Reference document by ID and title only
- Archive index should be scannable, not comprehensive

**Example Entry:** "SPEC008 (Library Management, 850 lines): Archived 2025-10-25, superseded by modular SPEC008/ folder. Retrieve via git archive/docs branch."

---

## Simplified Workflow

### Step 1: Validate Document

**Check document exists:**
```bash
test -f <document_path>
```

**If not found:**
- Report error: "Document not found: <document_path>"
- STOP workflow

**If found:**
- Read document to get first line/title
- Count lines for archive index
- Continue to Step 2

---

### Step 2: Confirmation Prompt

**Display:**
```
Archive Document

Document: <document_path>
Title: <first_line>
Size: <line_count> lines
Reason: <reason or "Not specified">

This will:
- Remove document from working branch
- Preserve in archive branch via automatic sync
- Update archive index with retrieval command
- Full git history preserved

Confirm archival? [y/N]
```

**Accept:**
- "y", "yes", "Y" → Continue to Step 3
- "n", "no", "N", or any other input → STOP with "Archival cancelled"

---

### Step 3: Sync to Archive Branch (Safety First)

**Purpose:** Ensure archive branch has current document before deletion

```bash
# Switch to archive branch
git checkout archive

# Get latest content from working branch
git checkout dev -- docs/ workflows/ wip/ project_management/

# Stage and commit
git add .
git commit -m "Sync repository before archiving <filename>"

# Return to working branch
git checkout dev
```

**Error Handling:**
- If archive branch doesn't exist: Report "Archive branch not found" and STOP
- If checkout fails: Report error and STOP
- If no changes to commit: Continue silently (already synced)
- If commit fails: Report error and STOP

**Result:** Document safely preserved in archive branch before deletion

---

### Step 4: Remove from Working Branch

```bash
git rm <document_path>
```

**Result:** File staged for deletion from working branch

**Error Handling:**
- If git rm fails: Report error and STOP

---

### Step 5: Update Archive Index

**Read archive index:**
```bash
read_file: workflows/REG002_archive_index.md
```

**Find or create appropriate section:**
- For wip/ documents: "### Work-In-Progress Archives"
- For docs/ documents: "### Documentation Archives"
- For workflows/ documents: "### Workflow Archives"
- For project_management/ documents: "### Project Management Archives"
- For other documents: "### Other Archives"

**Create entry:**
```markdown
| <filename> | <reason> | <date> | `git show archive:<document_path>` |
```

**Insert entry:**
- Use `search_replace` to add row to table (reverse chronological order)
- Stage archive index

**Error Handling:**
- If section not found: Create section with table header
- If update fails: Warn user but continue (non-blocking)

---

### Step 6: Commit via /commit

**Stage changes:**
- Document deletion (already staged from Step 4)
- Archive index update (staged in Step 5)

**Invoke /commit workflow:**
```
All changes staged, ready for /commit
```

**Note:** `/commit` Step 6.5 will automatically sync the deletion to archive branch

**Result:**
- Working branch: Document removed ✓
- Archive branch: Document preserved ✓
- Archive index: Updated with retrieval command ✓
- Change history: Updated via `/commit` ✓

---

## Retrieval

**View archived document:**
```bash
git show archive:<document_path>
```

**Restore archived document:**
```bash
# From archive index, copy the retrieval command and redirect to file
git show archive:wip/file.md > wip/file.md

# Or checkout entire archive branch to browse
git checkout archive
# Files are accessible normally
# When done: git checkout dev
```

---

## Why This Approach Works

**Safety:**
- Archive sync happens BEFORE deletion
- Document preserved before removal
- No risk of data loss

**Simplicity:**
- No staging directory needed
- No complex branch manipulation
- Just: sync → delete → commit

**Leverage Existing Infrastructure:**
- `/commit` Step 6.5 syncs repository automatically
- No manual archive branch commits after deletion
- Single source of truth for commits (DRY principle)

**Same End Result:**
- Document removed from working branch ✓
- Document preserved in archive branch ✓
- Archive index updated ✓
- Full git history ✓

---

## Error Scenarios

**Scenario: "Archive branch not found"**
- Cause: Archive branch doesn't exist yet
- Resolution: Create archive branch first or skip archival
- Recovery: Document remains in working branch

**Scenario: "Document not found"**
- Cause: Path incorrect or already archived
- Resolution: Check path, check archive index
- Recovery: N/A (no changes made)

**Scenario: "Archive sync failed"**
- Cause: Checkout failed or commit failed in Step 3
- Resolution: Report error, STOP before deletion
- Recovery: Manual sync or retry

**Scenario: "User cancelled"**
- Cause: User responded "n" to confirmation
- Resolution: Clean exit, no changes
- Recovery: N/A (intentional cancellation)

---

## Integration with /commit

**This workflow stages changes for /commit:**
1. Document deletion (git rm)
2. Archive index update

**Then calls /commit which:**
1. Creates change history entry
2. Commits with proper message format
3. Stages commit hash
4. **Syncs deletion to archive branch via Step 6.5**

**Result:** Archive branch mirrors working branch state (minus deleted files, but with full history)

---

## Success Criteria

A successful archival results in:
1. ✓ Document removed from working branch
2. ✓ Document preserved in archive branch (verified in Step 3)
3. ✓ Archive index updated with retrieval command
4. ✓ `/commit` workflow used (not manual commits)
5. ✓ Change history updated
6. ✓ Full git history preserved
7. ✓ Document retrievable via archive index commands

---

## Related Documentation

- **Commit Workflow:** `.claude/commands/commit.md` (Step 6.5 archive sync)
- **Batch Archive:** `.claude/commands/archive-plan.md` (uses this workflow)
- **Archive Index:** `workflows/REG002_archive_index.md`
- **Document Hierarchy:** `docs/GOV001-document_hierarchy.md`

---

## WKMP-Specific Context

**Project:** WKMP Music Player (Rust-based Auto DJ system)

**Archivable Locations:**
- `docs/` - Technical documentation (requirements, specs, implementations)
- `workflows/` - Development workflows and procedures
- `wip/` - Work-in-progress documents and analyses
- `project_management/` - Project planning and tracking documents

**Common Archive Reasons:**
- "Completed" - Work item finished
- "Superseded by <document>" - Replaced by newer version
- "Historical" - No longer relevant but preserved for reference
- "Merged into <document>" - Content integrated elsewhere
- "Outdated" - No longer accurate or applicable

**Archive Considerations:**
- REQ/SPEC/IMPL documents rarely archived (living documentation)
- WIP documents archived upon completion
- Design alternatives archived when decision made
- Implementation notes archived post-deployment
