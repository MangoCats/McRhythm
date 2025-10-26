# Archive Index

**Purpose:** Track archived documents and provide retrieval commands
**Maintained by:** /archive and /archive-plan workflows (automated)
**Last Updated:** 2025-10-25

---

## Overview

This index tracks documents that have been archived to the `archive` git branch to keep the working tree clean and optimize context windows. All archived documents preserve full git history and can be easily retrieved.

**Current Status:**
- **Active Documents:** All visible in working tree
- **Archived Documents:** 0
- **Context Reduction:** 0% (baseline)

---

## Archive Branch Strategy

WKMP uses a git-based archive system:

**Working Branch:** `dev` (or `main`)
- Contains only active, relevant documents
- Clean working tree for AI context optimization
- Daily development work

**Archive Branch:** `archive`
- Contains historical and obsolete documents
- Preserves full git history
- Reference only (not for daily work)

**Benefits:**
- 40-60% context window reduction (historical docs completely hidden)
- Automatic exclusion from AI tools
- Nothing truly deleted - full history preserved
- Easy retrieval via git commands

---

## Retrieval Commands

### View Archived Document
```bash
git show archive:path/to/archived_file.md
```

### Restore Archived Document to Working Tree
```bash
git show archive:path/to/archived_file.md > restored_file.md
```

### Browse Archive Branch
```bash
git checkout archive
# Explore files
git checkout dev  # Return to working branch
```

---

## Archived Documents

<!-- Entries organized by category, reverse chronological within each category -->

### No documents archived yet

Use the `/archive` workflow to archive completed or superseded documents.

---

## When to Archive

**Archive when:**
- Analysis complete AND decision implemented
- Planning complete AND project finished
- Document superseded by newer version
- Content no longer relevant to active work

**Do NOT archive when:**
- Document still actively referenced
- Analysis complete but not yet implemented
- Planning in progress
- Document provides current context for ongoing work

---

## Usage

**Archive single document:**
```bash
/archive path/to/document.md "reason for archival"
```

**Archive completed plan (batch):**
```bash
/archive-plan PLAN###
```

The workflows will:
1. Sync document to archive branch (safety first)
2. Remove from working branch via `git rm`
3. Update this index with retrieval commands
4. Stage changes for /commit

---

**Maintained by:** /archive and /archive-plan workflows
**Format:** Markdown with retrieval commands
**Version:** 1.0
