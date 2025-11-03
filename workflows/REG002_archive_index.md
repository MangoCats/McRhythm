# Archive Index

**Purpose:** Track archived documents and provide retrieval commands
**Maintained by:** /archive and /archive-plan workflows (automated)
**Last Updated:** 2025-11-03

---

## Overview

This index tracks documents that have been archived to the `archive` git branch to keep the working tree clean and optimize context windows. All archived documents preserve full git history and can be easily retrieved.

**Current Status:**
- **Active Documents:** All visible in working tree
- **Archived Documents:** 21 (PLAN019 + PLAN017 + PLAN018 + 18 previous archives)
- **Context Reduction:** Significant (~14,300 lines total: 3,450 from PLAN019 + 4,354 from PLAN017 + 6,500 from PLAN018)

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

### Work-In-Progress Archives

| Filename | Reason | Date | Retrieval Command |
|----------|--------|------|-------------------|
| PLAN019_dry_metadata_validation/ | PLAN019 complete - DRY metadata validation (14 parameters, ~160 LOC eliminated, 100% test pass) | 2025-11-03 | `git checkout archive && cd wip/PLAN019_dry_metadata_validation` |
| PLAN017_spec017_compliance/ | PLAN017 complete - SPEC017 compliance remediation (7 requirements, 100% test pass) | 2025-11-03 | `git checkout archive && cd wip/PLAN017_spec017_compliance` |
| PLAN018_centralized_global_parameters/ | PLAN018 complete - All 15 parameters migrated to GlobalParams | 2025-11-02 | `git checkout archive && cd wip/PLAN018_centralized_global_parameters` |
| PLAN012_api_key_multi_tier_config/ | PLAN012 complete - API key config implemented | 2025-11-02 | `git checkout archive && cd wip/PLAN012_api_key_multi_tier_config` |
| PLAN011_import_progress_ui/ | PLAN011 complete | 2025-11-02 | `git checkout archive && cd wip/PLAN011_import_progress_ui` |
| _toml_directory_creation.md | Superseded by increment2_zero_config | 2025-11-02 | `git show archive:wip/_toml_directory_creation.md` |
| _context_engineering.md | Incorporated into workflows | 2025-11-02 | `git show archive:wip/_context_engineering.md` |
| _attitude_adjustment.md | Superseded by analysis_results | 2025-11-02 | `git show archive:wip/_attitude_adjustment.md` |
| TECHNICAL_DEBT_REPORT.md | Superseded by TECH_DEBT_REVIEW | 2025-11-02 | `git show archive:wip/TECHNICAL_DEBT_REPORT.md` |
| PLAN008_sprint3_completion_report.md | Sprint report complete | 2025-11-02 | `git show archive:wip/PLAN008_sprint3_completion_report.md` |
| PLAN011_execution_status.md | PLAN011 complete | 2025-11-02 | `git show archive:wip/PLAN011_execution_status.md` |
| PLAN011_COMPLETE.md | Implementation complete | 2025-11-02 | `git show archive:wip/PLAN011_COMPLETE.md` |
| test_fixes_summary.md | Completed - all fixes applied | 2025-11-02 | `git show archive:wip/test_fixes_summary.md` |
| plan_registry_backfill_analysis.md | Completed - registry backfilled | 2025-11-02 | `git show archive:wip/plan_registry_backfill_analysis.md` |
| plan_numbering_analysis_results.md | Completed - numbering in REG001 | 2025-11-02 | `git show archive:wip/plan_numbering_analysis_results.md` |
| wkmp_ap_test_investigation.md | Completed - tests fixed | 2025-11-02 | `git show archive:wip/wkmp_ap_test_investigation.md` |
| _attitude_adjustment_analysis_results.md | Completed - process applied | 2025-11-02 | `git show archive:wip/_attitude_adjustment_analysis_results.md` |
| _deprioritize_effort_analysis_results.md | Completed - decision applied | 2025-11-02 | `git show archive:wip/_deprioritize_effort_analysis_results.md` |
| _database_review_analysis.md | Completed - wkmp-dr implemented | 2025-11-02 | `git show archive:wip/_database_review_analysis.md` |
| spec017_compliance_review_analysis_results.md | Completed - duplicate of review | 2025-11-02 | `git show archive:wip/spec017_compliance_review_analysis_results.md` |
| spec017_compliance_review.md | Completed - remediation implemented | 2025-11-02 | `git show archive:wip/spec017_compliance_review.md` |

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
