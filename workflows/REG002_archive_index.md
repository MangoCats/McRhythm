# Archive Index

**Purpose:** Track archived documents and provide retrieval commands
**Maintained by:** /archive and /archive-plan workflows (automated)
**Last Updated:** 2025-11-03

---

## Overview

This index tracks documents that have been archived to the `archive` git branch to keep the working tree clean and optimize context windows. All archived documents preserve full git history and can be easily retrieved.

**Current Status:**
- **Active Documents:** All visible in working tree
- **Archived Documents:** 37 (SPEC024_implementation + wkmp-ap_technical_debt_report + TECH_DEBT_REVIEW + mixer_technical_debt_analysis + mixer_architecture_review + SPEC_spec017 + PLAN_sqlx_0.8_upgrade + MED-005 + increment2_zero_config_analysis + IMPL-GLOBAL-PARAMS + PLAN010 + PLAN006 + PLAN014 + PLAN008 + PLAN009 + PLAN016 + PLAN019 + PLAN017 + PLAN018 + 18 previous archives)
- **Context Reduction:** Significant (~45,144 lines total: 477 from SPEC024_implementation + 316 from wkmp-ap_technical_debt_report + 1,031 from TECH_DEBT_REVIEW + 354 from mixer_technical_debt_analysis + 453 from mixer_architecture_review + 998 from SPEC_spec017 + 349 from PLAN_sqlx_0.8_upgrade + 425 from MED-005 + 470 from increment2_zero_config_analysis + 607 from IMPL-GLOBAL-PARAMS + 2,000 from PLAN010 + 2,239 from PLAN006 + 11,624 from PLAN014 + 4,020 from PLAN008 + 540 from PLAN009 + 4,941 from PLAN016 + 3,450 from PLAN019 + 4,354 from PLAN017 + 6,500 from PLAN018)

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
| SPEC024_human_readable_time_implementation.md | SPEC024 implementation complete - Human-readable time display deployed | 2025-11-03 | `git show archive:wip/SPEC024_human_readable_time_implementation.md` |
| wkmp-ap_technical_debt_report.md | wkmp-ap technical debt report (Oct 29) - Superseded by PLAN008 and broader TECH_DEBT_REVIEW | 2025-11-03 | `git show archive:wip/wkmp-ap_technical_debt_report.md` |
| TECH_DEBT_REVIEW_2025-11-02.md | Complete codebase technical debt review (6 microservices + shared library) - Historical snapshot | 2025-11-03 | `git show archive:wip/TECH_DEBT_REVIEW_2025-11-02.md` |
| mixer_technical_debt_analysis.md | Mixer technical debt analysis complete - All identified issues resolved or tracked | 2025-11-03 | `git show archive:wip/mixer_technical_debt_analysis.md` |
| mixer_architecture_review.md | Mixer architecture review complete - PLAN014/SPEC016 implemented | 2025-11-03 | `git show archive:wip/mixer_architecture_review.md` |
| SPEC_spec017_compliance_remediation.md | SPEC017 compliance remediation - Implementation complete (PLAN017) | 2025-11-03 | `git show archive:wip/SPEC_spec017_compliance_remediation.md` |
| PLAN_sqlx_0.8_upgrade.md | SQLx 0.8.1 upgrade plan - Historical planning document | 2025-11-03 | `git show archive:wip/PLAN_sqlx_0.8_upgrade.md` |
| MED-005_WIP_EXTRACTION_RECOMMENDATIONS.md | WIP document extraction recommendations - Historical guidance document | 2025-11-03 | `git show archive:wip/MED-005_WIP_EXTRACTION_RECOMMENDATIONS.md` |
| increment2_zero_config_analysis.md | PLAN015 Increment 2 analysis complete - Zero-config implementation finished | 2025-11-03 | `git show archive:wip/increment2_zero_config_analysis.md` |
| IMPL-GLOBAL-PARAMS-centralized_global_parameters.md | Implementation complete - Superseded by PLAN018 (centralized GlobalParams system) | 2025-11-03 | `git show archive:wip/IMPL-GLOBAL-PARAMS-centralized_global_parameters.md` |
| PLAN014_mixer_refactoring/ | PLAN014 complete - SPEC016 mixer integration (legacy mixer removed, event-driven markers, 219 tests pass) | 2025-11-03 | `git checkout archive && cd wip/PLAN014_mixer_refactoring` |
| PLAN008_wkmp_ap_technical_debt/ | PLAN008 complete - All 3 sprints (37 requirements, Oct 30); engine refactoring via PLAN016 (Nov 1) | 2025-11-03 | `git checkout archive && cd wip/PLAN008_wkmp_ap_technical_debt` |
| PLAN009_engine_module_extraction/ | Superseded by PLAN016 - Same goal (engine.rs refactoring) achieved via comprehensive /plan workflow | 2025-11-03 | `git checkout archive && cd wip/PLAN009_engine_module_extraction` |
| PLAN016_engine_refactoring/ | PLAN016 complete - Engine refactored into modular structure (4,251→4,324 lines across 4 modules, 219 tests pass) | 2025-11-03 | `git checkout archive && cd wip/PLAN016_engine_refactoring` |
| PLAN019_dry_metadata_validation/ | PLAN019 complete - DRY metadata validation (14 parameters, ~160 LOC eliminated, 100% test pass) | 2025-11-03 | `git checkout archive && cd wip/PLAN019_dry_metadata_validation` |
| PLAN017_spec017_compliance/ | PLAN017 complete - SPEC017 compliance remediation (7 requirements, 100% test pass) | 2025-11-03 | `git checkout archive && cd wip/PLAN017_spec017_compliance` |
| PLAN018_centralized_global_parameters/ | PLAN018 complete - All 15 parameters migrated to GlobalParams | 2025-11-02 | `git checkout archive && cd wip/PLAN018_centralized_global_parameters` |
| PLAN012_api_key_multi_tier_config/ | PLAN012 complete - API key config implemented | 2025-11-02 | `git checkout archive && cd wip/PLAN012_api_key_multi_tier_config` |
| PLAN006_wkmp_ai_ui_spec_updates/ | PLAN006 complete - wkmp-ai UI clarification (6 spec files updated, "on-demand" pattern defined) | 2025-10-28 | `git checkout archive && cd wip/PLAN006_wkmp_ai_ui_spec_updates` |
| PLAN010_workflow_quality_standards/ | PLAN010 complete - Workflow quality standards (Professional Objectivity, plan execution standards, Phase 9 tech debt) | 2025-10-30 | `git checkout archive && cd wip/PLAN010_workflow_quality_standards` |
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
