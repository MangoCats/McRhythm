# Archive Index

**Purpose:** Track archived documents and provide retrieval commands
**Maintained by:** /archive and /archive-plan workflows (automated)
**Last Updated:** 2025-11-12

---

## Overview

This index tracks documents that have been archived to the `archive` git branch to keep the working tree clean and optimize context windows. All archived documents preserve full git history and can be easily retrieved.

**Current Status:**
- **Active Documents:** All visible in working tree
- **Archived Documents:** 51 (PLAN025_spec032_wkmp_ai_update + PLAN024_wkmp_ai_recode + SPEC032_alignment_analysis + PLAN021 + wkmp-ai_static_serving + wkmp-ai_path_escaping + SSE_troubleshooting + SPEC_event_driven + SPEC_import_progress + DRY_analysis + DRY_implementation + PROJ001 + PLAN020 + SPEC024_tech_debt + SPEC024_implementation + wkmp-ap_technical_debt_report + TECH_DEBT_REVIEW + mixer_technical_debt_analysis + mixer_architecture_review + SPEC_spec017 + PLAN_sqlx_0.8_upgrade + MED-005 + increment2_zero_config_analysis + IMPL-GLOBAL-PARAMS + PLAN010 + PLAN006 + PLAN014 + PLAN008 + PLAN009 + PLAN016 + PLAN019 + PLAN017 + PLAN018 + 18 previous archives)
- **Context Reduction:** Significant (~88,964 lines total: 8,263 from PLAN025_spec032_wkmp_ai_update + 11,142 from PLAN024_wkmp_ai_recode + 4,286 from SPEC032_alignment_analysis + 7,543 from PLAN021 + 159 from wkmp-ai_static_serving + 178 from wkmp-ai_path_escaping + 258 from SSE_troubleshooting + 1,179 from SPEC_event_driven + 478 from SPEC_import_progress + 742 from DRY_analysis + 334 from DRY_implementation + 3,301 from PROJ001 + 5,120 from PLAN020 + 837 from SPEC024_tech_debt + 477 from SPEC024_implementation + 316 from wkmp-ap_technical_debt_report + 1,031 from TECH_DEBT_REVIEW + 354 from mixer_technical_debt_analysis + 453 from mixer_architecture_review + 998 from SPEC_spec017 + 349 from PLAN_sqlx_0.8_upgrade + 425 from MED-005 + 470 from increment2_zero_config_analysis + 607 from IMPL-GLOBAL-PARAMS + 2,000 from PLAN010 + 2,239 from PLAN006 + 11,624 from PLAN014 + 4,020 from PLAN008 + 540 from PLAN009 + 4,941 from PLAN016 + 3,450 from PLAN019 + 4,354 from PLAN017 + 6,500 from PLAN018)

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

### Archived Plan Documents

#### PLAN025: SPEC032 wkmp-ai Update (2025-11-12)

Archived SPEC032 wkmp-ai implementation update plan folder from wip/PLAN025_spec032_wkmp_ai_update/

**Contents:** 16+ files, 8,263 lines
- 00_PLAN_SUMMARY.md (450 lines) - Executive summary
- 01_specification_issues.md - 8 issues identified (0 CRITICAL, 2 HIGH, 4 MEDIUM, 2 LOW)
- 02_test_specifications/ (test_index.md, traceability_matrix.md)
- requirements_index.md (250 lines) - 12 requirements cataloged
- scope_statement.md - Scope definition
- PLAN025_integration_session[2-6].md - 6 integration sessions
- PLAN025_phase1_design.md, phase2_summary.md, phase4_summary.md
- PLAN025_session_summary.md - Overall session summary
- SPEC032_ALIGNMENT_REVIEW_FINDINGS.md - Alignment review
- SPEC032_IMPLEMENTATION_UPDATE.md - Implementation update
- SPEC032_wkmp-ai_refinement_specification.md - Refinement specification

**Reason:** Plan ready for implementation - Phases 1-3 complete (Week 1 Deliverable). Segmentation-first, evidence-based architecture plan created per /plan workflow.

**Key Deliverables:**
- 12 requirements analyzed (2 P0 Critical, 6 P1 High, 4 P2 Medium)
- 8 specification issues identified and resolved
- 32 tests defined with 100% requirement coverage
- Test-first, specification-driven planning complete
- Architecture: Segment → Match → Fingerprint → Identify

**Restoration:**
```bash
# Restore complete folder
git checkout archive -- wip/PLAN025_spec032_wkmp_ai_update/

# Or view archive branch
git checkout archive
cd wip/PLAN025_spec032_wkmp_ai_update/
# Browse files normally
git checkout ai-trial2  # Return to working branch
```

**Retrieval (individual files):**
```bash
# View plan summary
git show archive:wip/PLAN025_spec032_wkmp_ai_update/00_PLAN_SUMMARY.md

# View specification issues
git show archive:wip/PLAN025_spec032_wkmp_ai_update/01_specification_issues.md

# List all files in folder
git ls-tree -r archive -- wip/PLAN025_spec032_wkmp_ai_update/
```

---

#### PLAN024: WKMP-AI Audio Import System Recode (2025-11-12)

Archived complete WKMP-AI recode implementation plan folder from wip/PLAN024_wkmp_ai_recode/

**Contents:** 18 files, 11,142 lines
- 00_PLAN_SUMMARY.md (376 lines) - Executive summary
- 01_specification_issues.md (826 lines) - 37 issues identified (7 CRITICAL resolved)
- 02_specification_amendments.md (1,450 lines) - SSOT for all resolutions
- 03_acceptance_tests.md (1,773 lines) - Test specifications
- 04_approach_selection.md (756 lines) - Implementation approach
- 05_implementation_breakdown.md (409 lines) - Breakdown details
- 06_effort_and_schedule.md (317 lines) - Effort estimates (12-14 weeks)
- 07_risk_assessment.md (833 lines) - Risk analysis
- 08_final_plan_approval.md (678 lines) - Plan approval
- 09_file_level_tracking_analysis.md (981 lines) - File tracking analysis
- 10_amendment_8_summary.md (356 lines) - Amendment 8 summary
- 11_amendment_9_summary.md (581 lines) - Amendment 9 summary
- 11_plan_review_findings.md (556 lines) - Plan review
- APPROVAL_RECORD.md (228 lines) - Approval record
- IMPLEMENTATION_START.md (158 lines) - Implementation start marker
- dependencies_map.md (387 lines) - Dependencies
- requirements_index.md (193 lines) - 77 requirements cataloged
- scope_statement.md (284 lines) - Scope definition

**Reason:** Plan completed - Phases 1-3 complete (Week 1 Deliverable). Systematic implementation plan for 3-tier hybrid fusion architecture created per /plan workflow.

**Key Deliverables:**
- 77 requirements analyzed (72 original + 5 amendments)
- 7/7 CRITICAL specification issues resolved
- 100% requirement → test coverage achieved
- Test-first, specification-driven planning complete
- Ready for implementation (awaiting stakeholder approval)

**Restoration:**
```bash
# Restore complete folder
git checkout archive -- wip/PLAN024_wkmp_ai_recode/

# Or view archive branch
git checkout archive
cd wip/PLAN024_wkmp_ai_recode/
# Browse files normally
git checkout ai-trial2  # Return to working branch
```

**Retrieval (individual files):**
```bash
# View plan summary
git show archive:wip/PLAN024_wkmp_ai_recode/00_PLAN_SUMMARY.md

# View specification issues
git show archive:wip/PLAN024_wkmp_ai_recode/01_specification_issues.md

# List all files in folder
git ls-tree -r archive -- wip/PLAN024_wkmp_ai_recode/
```

---

#### PLAN021: Technical Debt Remediation (2025-11-05)

Archived complete technical debt remediation plan folder from wip/PLAN021_technical_debt_remediation/

**Contents:** 17 files, 7,543 lines
- SESSION1_SUMMARY.md through SESSION6_SUMMARY.md (6 session summaries)
- PROGRESS.md (master progress tracker)
- SPEC_technical_debt_remediation.md (implementation specification)
- core_refactoring_roadmap.md (refactoring design)
- test_baseline.md (initial test status)
- wkmp_common_test_coverage_report.md (test coverage analysis)
- PLAN_technical_debt_remediation_scope_statement.md
- PLAN_technical_debt_remediation_requirements_index.md
- PLAN_technical_debt_remediation_acceptance_tests.md
- PLAN_technical_debt_remediation_dependencies_map.md
- PLAN_technical_debt_remediation_specification_gaps.md
- PLAN_recommendations_applied.md

**Reason:** Technical debt remediation completed (100% - all 7 increments complete, 12 commits, 2,715 LOC removed, 25 tests added, all documentation updated)

**Key Deliverables:**
- core.rs refactored: 3,156 → 1,801 LOC (43% reduction)
- Extracted modules: chains.rs (279 LOC), playback.rs (1,133 LOC)
- Removed deprecated code: config.rs (206 LOC), auth_middleware duplicates (577 LOC)
- Code quality: 10 clippy warnings fixed, 25 tests added (uuid_utils, time, events)
- Documentation: IMPL003-project_structure.md updated with refactoring details

**Restoration:**
```bash
# Restore complete folder
git checkout archive -- wip/PLAN021_technical_debt_remediation/

# Or view archive branch
git checkout archive
cd wip/PLAN021_technical_debt_remediation/
# Browse files normally
git checkout dev  # Return to working branch
```

**Retrieval (individual files):**
```bash
# View specific session summary
git show archive:wip/PLAN021_technical_debt_remediation/SESSION6_SUMMARY.md

# View progress tracker
git show archive:wip/PLAN021_technical_debt_remediation/PROGRESS.md

# List all files in folder
git ls-tree -r archive -- wip/PLAN021_technical_debt_remediation/
```

---

#### PROJ001: Automated Queue Chain Tests (2025-11-04)

Archived complete test infrastructure project folder from wip/PROJ001_automated_queue_chain_tests/

**Contents:** 11 files, 3301 lines
- FINAL_TEST_SUITE_STATUS.md (536 lines)
- README.md (249 lines)
- telemetry_implementation_plan.md (346 lines)
- test_harness_implementation_summary.md (316 lines)
- test_implementation_checklist.md (166 lines)
- test_results_summary.md (229 lines)
- test_session_4_summary.md (263 lines)
- test_session_5_summary.md (245 lines)
- test_session_6_summary.md (362 lines)
- test_session_7_summary.md (330 lines)
- test_session_8_summary.md (259 lines)

**Reason:** Test infrastructure project completed (7/7 tests passing, comprehensive test harness)

**Restoration:**
```bash
# Restore complete folder
git checkout archive -- wip/PROJ001_automated_queue_chain_tests/

# Or view archive branch
git checkout archive
cd wip/PROJ001_automated_queue_chain_tests/
# Browse files normally
git checkout dev  # Return to working branch
```

**Retrieval (individual files):**
```bash
# View specific file
git show archive:wip/PROJ001_automated_queue_chain_tests/README.md

# List all files in folder
git ls-tree -r archive -- wip/PROJ001_automated_queue_chain_tests/
```

---

#### PLAN020: Event-Driven Playback Orchestration (2025-11-04)

Archived complete implementation plan folder from wip/PLAN020_event_driven_playback/

**Contents:** 16 files, 5120 lines
- 00_PLAN_SUMMARY.md (508 lines)
- 01_specification_issues.md (399 lines)
- 02_test_specifications/ (4 files, 794 lines)
- DEFERRED_TESTS_ANALYSIS.md (254 lines)
- IMPLEMENTATION_PROGRESS.md (529 lines)
- PHASE_6_7_DOCUMENTATION_PLAN.md (372 lines)
- requirements_index.md (356 lines)
- SESSION_2025-11-04_PHASE5_SUMMARY.md (248 lines)
- SESSION_2025-11-04_SUMMARY.md (274 lines)
- SESSION_2025-11-04_WATCHDOG_VISIBILITY.md (541 lines)
- traceability_matrix.md (352 lines)
- WATCHDOG_SSE_ENHANCEMENT.md (282 lines)
- WATCHDOG_VISIBILITY_FEATURE.md (211 lines)

**Reason:** Plan implementation completed (all 7 phases, 12/12 tests passing, SPEC028 v2.0 documented)

**Restoration:**
```bash
# Restore complete folder
git checkout archive -- wip/PLAN020_event_driven_playback/

# Or view archive branch
git checkout archive
cd wip/PLAN020_event_driven_playback/
# Browse files normally
git checkout dev  # Return to working branch
```

**Retrieval (individual files):**
```bash
# View specific file
git show archive:wip/PLAN020_event_driven_playback/00_PLAN_SUMMARY.md

# List all files in folder
git ls-tree -r archive -- wip/PLAN020_event_driven_playback/
```

---

### Work-In-Progress Archives

| Filename | Reason | Date | Retrieval Command |
|----------|--------|------|-------------------|
| SPEC032_alignment_analysis/ | SPEC032 alignment analysis complete (4,286 lines) - Ready for decision; Recommends Approach 2 (Incremental Integration) | 2025-11-12 | `git checkout archive && cd wip/SPEC032_alignment_analysis` |
| wkmp-ai_static_serving_fix.md | wkmp-ai static file serving fix complete - All static files served correctly | 2025-11-04 | `git show archive:wip/wkmp-ai_static_serving_fix.md` |
| wkmp-ai_path_escaping_fix.md | wkmp-ai path escaping fix complete - Windows backslash paths working | 2025-11-04 | `git show archive:wip/wkmp-ai_path_escaping_fix.md` |
| SSE_troubleshooting.md | SSE troubleshooting complete - Connection issues resolved | 2025-11-04 | `git show archive:wip/SSE_troubleshooting.md` |
| SPEC_event_driven_playback_refactor.md | PLAN020 specification - Integrated into SPEC028 v2.0 | 2025-11-04 | `git show archive:wip/SPEC_event_driven_playback_refactor.md` |
| SPEC_import_progress_ui_enhancement.md | Import progress UI specification - Implementation complete | 2025-11-04 | `git show archive:wip/SPEC_import_progress_ui_enhancement.md` |
| DRY_analysis_ui_components.md | DRY analysis complete - UI components and styles consolidated | 2025-11-04 | `git show archive:wip/DRY_analysis_ui_components.md` |
| DRY_implementation_summary.md | DRY implementation complete - Shared styles integrated across modules | 2025-11-04 | `git show archive:wip/DRY_implementation_summary.md` |
| SPEC024-wkmp_ap_technical_debt_remediation.md | SPEC024 technical debt remediation draft - Superseded by PLAN008 implementation | 2025-11-03 | `git show archive:wip/SPEC024-wkmp_ap_technical_debt_remediation.md` |
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
