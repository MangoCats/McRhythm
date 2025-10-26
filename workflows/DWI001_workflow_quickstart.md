# WKMP Workflow Quick Start Guide

**Document ID:** DWI001 (Developer Work Instructions #1)
**Purpose:** Quick reference for WKMP development workflows
**Audience:** Developers using Claude Code with WKMP
**Last Updated:** 2025-10-25

---

## Quick Reference Card

| Command | When | Output |
|---------|------|--------|
| `/commit` | After any changes | Updates change_history.md, commits, syncs archive |
| `/doc-name <file>` | Ready to assign CAT### prefix | Renames file, updates registry |
| `/think <file>` | Have questions/problems | Creates RPT###_analysis_results.md |
| `/plan <spec>` | Ready to implement feature | Creates PLAN###_*/ folder |
| `/archive <file> "reason"` | Document completed/obsolete | Moves to archive branch |
| `/archive-plan PLAN###` | Plan implementation done | Batch archives all plan docs |

---

## 5-Minute Quick Start

### 1. Making Changes

**Scenario:** You've implemented a feature or fixed a bug

```bash
# Make your code changes, test them

# Commit using workflow (NOT git commit!)
/commit
```

**What happens:**
- Stages all changes
- Updates change_history.md automatically
- Creates commit with summary
- Syncs archive branch
- Prompts to push

---

### 2. Analyzing a Problem

**Scenario:** You're unsure which approach to take for a feature

```bash
# 1. Create a document with your questions
# File: wip/crossfade_algorithm_question.md
# Content: "Should we use linear or logarithmic crossfade curves?"

# 2. Run analysis workflow
/think wip/crossfade_algorithm_question.md

# 3. Review the analysis output
# File created: docs/RPT001_crossfade_algorithm_question_analysis_results.md
```

**What happens:**
- Multi-agent analysis of your question
- Research findings and option comparisons
- Executive summary added to your original document
- Complete analysis in new RPT### document

---

### 3. Planning an Implementation

**Scenario:** You have a spec ready and want to implement it

```bash
# 1. Ensure specification exists
# Example: docs/SPEC002-crossfade.md

# 2. Run planning workflow
/plan docs/SPEC002-crossfade.md

# 3. Review the plan
# Folder created: wip/PLAN001_crossfade/
#   - 00_PLAN_SUMMARY.md (read this first)
#   - 01_specification_issues.md (fix these!)
#   - requirements_index.md
#   - 02_test_specifications/
#   - traceability_matrix.md

# 4. Fix any critical specification issues identified

# 5. Implement following the plan increments

# 6. Commit after each increment
/commit
```

---

### 4. Organizing Documentation

**Scenario:** You created a new document in wip/ and it's ready for permanent naming

```bash
# Document exists: wip/musical_flavor_analysis.md

# Assign category and number
/doc-name wip/musical_flavor_analysis.md

# AI recommends: RPT (Reports & Analysis)
# Renames to: wip/RPT002_musical_flavor_analysis.md
# Updates: workflows/REG001_number_registry.md

# Commit the rename
/commit
```

---

### 5. Cleaning Up Completed Work

**Scenario:** You finished implementing a plan and all tests pass

```bash
# Plan complete: wip/PLAN001_crossfade/

# Archive all plan documents at once
/archive-plan PLAN001

# Commit the archival
/commit

# Documents moved to archive branch, working tree clean!
```

---

## Common Workflows

### Workflow A: Research → Design → Implement

```
1. Create question document: wip/api_design_question.md
2. /think wip/api_design_question.md
3. Review analysis: docs/RPT003_api_design_question_analysis_results.md
4. Make decision based on analysis
5. Update spec with chosen approach: docs/SPEC007-api_design.md
6. /commit (track spec update)
7. /plan docs/SPEC007-api_design.md
8. Review plan: wip/PLAN002_api_implementation/
9. Implement increment 1
10. /commit (track implementation)
11. Verify tests pass
12. Repeat 9-11 for remaining increments
13. /archive-plan PLAN002 (cleanup)
14. /commit (track archival)
```

---

### Workflow B: Quick Fix (No Planning Needed)

```
1. Identify and fix bug
2. Write/update tests
3. /commit
   - Updates change_history.md
   - Creates commit
   - Syncs archive
   - Pushes to remote
```

---

### Workflow C: Documentation Update

```
1. Update documentation file: docs/SPEC002-crossfade.md
2. /commit (track changes)
```

---

### Workflow D: New Feature (Full Cycle)

```
1. Create wip/audio_enhancement_idea.md with questions
2. /think wip/audio_enhancement_idea.md
3. Review analysis output
4. Write spec: wip/audio_enhancement_spec.md
5. /doc-name wip/audio_enhancement_spec.md → SPEC021_audio_enhancement.md
6. /commit (new spec)
7. /plan docs/SPEC021_audio_enhancement.md
8. Review wip/PLAN003_audio_enhancement/
9. Fix specification issues if any
10. /commit (spec fixes)
11. Implement following plan
12. /commit after each increment
13. All done: /archive-plan PLAN003
14. /commit (cleanup)
15. /archive docs/RPT004_audio_enhancement_idea_analysis_results.md "Implemented"
16. /commit (archive analysis)
```

---

## Category Selection Guide

**When using /doc-name, choose:**

| If Document Contains... | Use Category |
|------------------------|--------------|
| Governance, conventions, framework | GOV |
| What system must do | REQ |
| How to satisfy requirements | SPEC |
| Concrete implementation details | IMPL |
| When to build features | EXEC |
| Review of design/architecture | REV |
| How-to guide for developers | GUIDE |
| Analysis, investigation, /think output | RPT |
| Implementation plan, /plan output | PLAN |
| Workflow procedures | DWI |
| Reusable template | TMPL |
| Ongoing log or feedback | LOG |
| Registry or lookup table | REG |

**Location Hints:**

- `docs/` → Usually GOV, REQ, SPEC, IMPL, EXEC, REV, GUIDE, RPT
- `workflows/` → Usually DWI, TMPL, REG
- `project_management/` → Usually LOG
- `wip/` → Could be any (before finalization)

---

## Archive Decision Tree

**Should I archive this document?**

```
Is the document analysis/research (RPT###)?
├─ Yes: Is the decision implemented?
│  ├─ Yes → ARCHIVE
│  └─ No → KEEP
└─ No: Is it a plan (PLAN###)?
   ├─ Yes: Are all tests passing and implementation complete?
   │  ├─ Yes → ARCHIVE (use /archive-plan)
   │  └─ No → KEEP
   └─ No: Is it superseded by a newer version?
      ├─ Yes → ARCHIVE
      └─ No: Is it still referenced in active work?
         ├─ Yes → KEEP
         └─ No → Consider archiving
```

---

## File Location Reference

### By Workflow

**Created by /commit:**
- `project_management/change_history.md`

**Created by /doc-name:**
- Renames existing file with CAT### prefix
- Updates `workflows/REG001_number_registry.md`

**Created by /think:**
- `docs/RPT###_<name>_analysis_results.md`
- Updates input document with summary

**Created by /plan:**
- `wip/PLAN###_<name>/00_PLAN_SUMMARY.md`
- `wip/PLAN###_<name>/01_specification_issues.md`
- `wip/PLAN###_<name>/requirements_index.md`
- `wip/PLAN###_<name>/02_test_specifications/`
- `wip/PLAN###_<name>/traceability_matrix.md`

**Updated by /archive:**
- `workflows/REG002_archive_index.md`
- Removes document from working tree
- Adds to archive branch

---

## Git Branch Overview

WKMP uses three branches:

```
main (stable)
  ↑
  | (merge at milestones)
  |
dev (active development) ← YOU ARE HERE
  |
  | (completed documents)
  ↓
archive (historical docs)
```

**Archive Branch:**
- Contains completed/obsolete documents
- Reduces working tree clutter by 40-60%
- Full git history preserved
- Easy retrieval: `git show archive:docs/RPT001_old.md`

---

## Tips & Tricks

### Tip 1: Always Start with /think for Big Decisions

**Before:**
```
# Jump straight to implementation
# → Might choose wrong approach
# → Costly rework later
```

**After:**
```
/think wip/architecture_question.md
# → Comprehensive analysis
# → Evidence-based decision
# → Right the first time
```

---

### Tip 2: Use /plan to Catch Spec Gaps Early

**Before:**
```
# Start coding from incomplete spec
# → Discover missing requirements mid-implementation
# → Rework and delays
```

**After:**
```
/plan docs/SPEC###.md
# → Specification issues identified BEFORE coding
# → Fix gaps while still in design phase
# → Smooth implementation
```

---

### Tip 3: Commit Often with /commit

**Before:**
```
# Manual git commits with inconsistent messages
# → Difficult to track what changed when
# → No automatic change history
```

**After:**
```
/commit after each logical increment
# → Automatic change history
# → Consistent commit messages
# → Full traceability
```

---

### Tip 4: Archive Completed Work Regularly

**Before:**
```
# Keep all historical documents in working tree
# → Cluttered workspace
# → AI context window filled with irrelevant docs
# → Harder to find current work
```

**After:**
```
/archive docs/RPT001_old.md "Decision implemented"
/archive-plan PLAN002
# → Clean working tree
# → Only active docs visible
# → Better AI assistance
```

---

## Emergency Procedures

### Undo /commit (Before Push)

```bash
# Reset to before commit (keeps changes)
git reset HEAD~1

# Edit files as needed

# Re-run /commit
/commit
```

---

### Retrieve Archived Document

```bash
# View in terminal
git show archive:docs/RPT001_old_analysis.md

# Restore to working tree
git show archive:docs/RPT001_old_analysis.md > docs/RPT001_old_analysis.md

# Or browse archive branch
git checkout archive
# Look around
git checkout dev  # Return to working branch
```

---

### Fix Wrong Category Assignment

```bash
# If /doc-name assigned wrong category:

# 1. Manually rename the file
git mv docs/RPT001_wrong.md docs/SPEC021_correct.md

# 2. Manually update workflows/REG001_number_registry.md
#    - Decrement RPT next available
#    - Increment SPEC next available
#    - Update assignment history

# 3. Commit the correction
/commit
```

---

## Learning Resources

**Essential Reading:**
1. [.claude/commands/README.md](file:///c%3A/Users/Mango%20Cat/Dev/McRhythm/.claude/commands/README.md) - Complete command reference
2. [docs/GOV001-document_hierarchy.md](file:///c%3A/Users/Mango%20Cat/Dev/McRhythm/docs/GOV001-document_hierarchy.md) - 5-tier documentation framework
3. [workflows/REG003_category_definitions.md](file:///c%3A/Users/Mango%20Cat/Dev/McRhythm/workflows/REG003_category_definitions.md) - Full category system

**Quick References:**
- [workflows/REG001_number_registry.md](file:///c%3A/Users/Mango%20Cat/Dev/McRhythm/workflows/REG001_number_registry.md) - Current document numbers
- [workflows/REG002_archive_index.md](file:///c%3A/Users/Mango%20Cat/Dev/McRhythm/workflows/REG002_archive_index.md) - Archived document retrieval
- [project_management/change_history.md](file:///c%3A/Users/Mango%20Cat/Dev/McRhythm/project_management/change_history.md) - Complete audit trail

---

## FAQ

**Q: Should I use /commit or git commit?**
A: Always use `/commit`. It maintains change_history.md and syncs the archive branch automatically.

**Q: When should I use /think vs just implementing?**
A: Use `/think` when you have multiple options or uncertainty about the best approach. Skip for straightforward tasks.

**Q: Do I need /plan for small features?**
A: For trivial changes (<50 lines), you can skip `/plan`. For anything complex, `/plan` catches spec issues early.

**Q: How do I know when to archive a document?**
A: Archive when analysis is implemented, plan is complete, or document is superseded. See "Archive Decision Tree" above.

**Q: Can I manually edit change_history.md?**
A: You CAN (git doesn't prevent it), but DON'T. Use `/commit` exclusively to maintain consistency.

**Q: What if I forget to /commit before switching branches?**
A: Claude Code will likely remind you. If not, stash changes first: `git stash`, switch branch, return, `git stash pop`, then `/commit`.

**Q: How do I see all archived documents?**
A: Check `workflows/REG002_archive_index.md` for the complete list with retrieval commands.

---

## Cheat Sheet

```bash
# Daily Commands
/commit                              # After any changes
/doc-name wip/file.md               # Assign CAT### prefix

# Analysis & Planning
/think wip/question.md              # Analyze problem
/plan docs/SPEC###.md               # Plan implementation

# Cleanup
/archive docs/RPT###.md "reason"    # Archive single file
/archive-plan PLAN###               # Archive plan folder

# Git Operations
git status                          # See current changes
git log --oneline -10               # Recent commits
git show archive:path/file.md       # View archived doc
```

---

**Next Steps:**

1. Try `/commit` on a small change to get familiar
2. Create a test question document and run `/think`
3. Review the output structure
4. Experiment with `/doc-name` on a wip file
5. When ready for a real feature, use `/plan`

---

**Version:** 1.0
**Last Updated:** 2025-10-25
**Maintained by:** WKMP Development Team
