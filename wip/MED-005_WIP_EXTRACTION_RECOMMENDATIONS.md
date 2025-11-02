# MED-005: WIP Document Extraction Recommendations

**Date:** 2025-11-02
**Purpose:** Identify content from /wip to extract into concise /docs "Required Reading"
**Status:** ANALYSIS COMPLETE

---

## Executive Summary

**Current State:**
- 29 active WIP documents (~14,090 lines)
- 57 documents in /docs (~tens of thousands of lines)
- No consolidated architecture overview in /docs

**Recommendations:**
1. **Extract 3 critical documents** from WIP to /docs (creates "Required Reading" coverage)
2. **Archive 15+ completed analysis documents** (reduces WIP clutter by 50%)
3. **Create 1 new consolidated document** (fills critical gap in /docs)

**Impact:** Reduces new developer onboarding confusion, improves document discoverability

---

## Priority 1: Critical Missing Documents (Extract to /docs)

### 1. SPEC001-architecture.md (NEW - Consolidate from Multiple Sources)

**Status:** MISSING - Critical gap in documentation hierarchy
**Source Material:**
- `wip/mixer_architecture_review.md` (453 lines)
- `CLAUDE.md` (Microservices Architecture section)
- `wip/increment2_zero_config_analysis.md` (470 lines)

**Recommended Content (Target: <400 lines):**

```markdown
# SPEC001: WKMP Architecture Overview

## 1. System Architecture (50 lines)
- 6 microservices + 1 shared library
- HTTP REST + SSE communication
- SQLite shared database
- Extract from CLAUDE.md lines 62-115

## 2. Module Responsibilities (100 lines)
- wkmp-ap: Audio Player (core playback, crossfading, queue)
- wkmp-ui: User Interface (web UI, authentication, orchestration)
- wkmp-pd: Program Director (passage selection algorithm)
- wkmp-ai: Audio Ingest (file scanning, MusicBrainz - Full only)
- wkmp-le: Lyric Editor (on-demand lyric editing - Full only)
- wkmp-dr: Database Review (read-only inspection - Full only)
- wkmp-common: Shared library (models, events, utilities)

## 3. Zero-Configuration Startup (80 lines)
- Extract from increment2_zero_config_analysis.md
- 4-tier priority for root folder resolution
- Automatic directory creation
- Database initialization pattern

## 4. Audio Pipeline Architecture (100 lines)
- Extract from mixer_architecture_review.md
- Single-stream design with sample-accurate crossfading
- Decoder → Buffer → Mixer → Output
- Marker-based event system

## 5. Data Flow (50 lines)
- Request flow (UI → wkmp-ap → database)
- Event flow (SSE broadcasting)
- Audio buffer flow

## 6. Version Architecture (20 lines)
- Full vs Lite vs Minimal
- Packaging strategy (no conditional compilation)
```

**Why Critical:**
- Referenced by REQ001 and all SPEC### documents but doesn't exist
- DEV_QUICKSTART.md currently points to CLAUDE.md as workaround
- Prevents circular documentation dependencies

**Extraction Priority:** **IMMEDIATE** (blocks effective onboarding)

---

### 2. GUIDE004-technical_debt_management.md (NEW - Extract from WIP)

**Source Material:**
- `wip/TECH_DEBT_REVIEW_2025-11-02.md` (993 lines)
- `wip/TECHNICAL_DEBT_REPORT.md` (809 lines)

**Recommended Content (Target: <300 lines):**

```markdown
# GUIDE004: Technical Debt Management Guide

## 1. Technical Debt Philosophy (30 lines)
- Risk-first prioritization (per PCH001)
- Quality-absolute goals require proactive debt management
- Classification system: CRITICAL, HIGH, MEDIUM, LOW

## 2. Debt Review Process (40 lines)
- Quarterly comprehensive reviews
- Automated detection (TODO/FIXME, unwrap/expect, test coverage)
- Prioritization framework

## 3. Common Debt Patterns (100 lines)
### Authentication Completeness
- Pattern: Partial auth implementation across modules
- Detection: grep for "TODO.*auth"
- Remediation: Complete Tower/Axum middleware pattern

### Test Placeholder Files
- Pattern: Empty test files or missing fixtures
- Detection: File size <100 bytes in tests/
- Remediation: Either implement or document skip reason

### Documentation Staleness
- Pattern: TODO comments referencing non-existent functions
- Detection: grep + cross-reference with current code
- Remediation: Update or convert to explanatory comments

## 4. Debt Tracking (30 lines)
- Location: wip/TECH_DEBT_REVIEW_<date>.md
- Archive when resolved: move to archive branch
- Reference in commit messages: [DEBT-###]

## 5. Prevention Strategies (50 lines)
- Requirement traceability comments [REQ-XXX-NNN]
- Test-first development (via /plan workflow)
- Code review focus areas
- Continuous refactoring vs tech debt sprints

## 6. Remediation Prioritization (50 lines)
- CRITICAL: Blocks CI/CD or introduces security risk
- HIGH: Affects production reliability or monitoring
- MEDIUM: Maintainability or code quality
- LOW: Nice-to-have improvements
```

**Why Important:**
- 993-line debt review is too large for quick reference
- Patterns are recurring (auth, tests, docs)
- New developers need concise debt management guide

**Extraction Priority:** **HIGH** (improves code quality practices)

---

### 3. ADR-003-zero_configuration_strategy.md (Extract from WIP)

**Source Material:**
- `wip/increment2_zero_config_analysis.md` (470 lines)

**Recommended Content (Target: <250 lines):**

```markdown
# ADR-003: Zero-Configuration Startup Strategy

## Context
All 6 microservices must start without configuration files ([REQ-NF-030] through [REQ-NF-037]).

## Decision
Implement 4-tier priority system for root folder resolution:
1. CLI argument (--root-folder)
2. Environment variable (WKMP_ROOT_FOLDER)
3. TOML config file (~/.config/wkmp/<module>.toml)
4. Compiled default (~/Music or %USERPROFILE%\Music)

## Implementation Pattern
- Shared utilities in wkmp_common::config
- Automatic directory creation via RootFolderInitializer
- Database initialization at root_folder/wkmp.db

## Consequences
### Positive
- Zero-config startup for 95% of users
- Power users can override via CLI or env vars
- Cross-platform compatibility (Windows, macOS, Linux)

### Negative
- Multiple resolution mechanisms add complexity
- TOML dependency even though rarely used
- Testing requires mocking all 4 tiers

## Alternatives Considered
### Hardcoded ~/Music only
- Rejected: No override mechanism for non-standard setups

### Environment variable only
- Rejected: Poor discoverability for casual users

### Config file required
- Rejected: Violates zero-config goal

## Status
Implemented (2025-10) - All 6 modules compliant
```

**Why Important:**
- Critical architectural decision affecting all modules
- Precedent for future configuration design
- Currently only exists as 470-line analysis in WIP

**Extraction Priority:** **HIGH** (documents key architectural pattern)

---

## Priority 2: Archive Candidates (Move to archive-branch)

**Recommendation:** Archive 15 completed documents to reduce WIP clutter by ~50%

### Completed Analysis Documents (Archive Immediately)

1. **`spec017_compliance_review.md`** (911 lines)
   - **Status:** COMPLETE - Remediation in SPEC_spec017_compliance_remediation.md
   - **Action:** Archive to `analysis/spec017_compliance_review.md`

2. **`spec017_compliance_review_analysis_results.md`** (911 lines)
   - **Status:** COMPLETE - Duplicate of above
   - **Action:** Archive to `analysis/spec017_compliance_review_analysis_results.md`

3. **`_database_review_analysis.md`** (903 lines)
   - **Status:** COMPLETE - Database review feature implemented
   - **Action:** Archive to `analysis/database_review_analysis.md`

4. **`_deprioritize_effort_analysis_results.md`** (828 lines)
   - **Status:** COMPLETE - Decision applied
   - **Action:** Archive to `analysis/deprioritize_effort_analysis_results.md`

5. **`_attitude_adjustment_analysis_results.md`** (714 lines)
   - **Status:** COMPLETE - Process improvement applied
   - **Action:** Archive to `analysis/attitude_adjustment_analysis_results.md`

6. **`wkmp_ap_test_investigation.md`** (467 lines)
   - **Status:** COMPLETE - Tests fixed
   - **Action:** Archive to `analysis/wkmp_ap_test_investigation.md`

7. **`plan_numbering_analysis_results.md`** (461 lines)
   - **Status:** COMPLETE - Numbering system established in REG001
   - **Action:** Archive to `analysis/plan_numbering_analysis_results.md`

8. **`plan_registry_backfill_analysis.md`** (370 lines)
   - **Status:** COMPLETE - Registry backfilled
   - **Action:** Archive to `analysis/plan_registry_backfill_analysis.md`

9. **`test_fixes_summary.md`** (359 lines)
   - **Status:** COMPLETE - All test fixes applied
   - **Action:** Archive to `analysis/test_fixes_summary.md`

10. **`PLAN011_COMPLETE.md`** (585 lines)
    - **Status:** COMPLETE (literally in filename)
    - **Action:** Archive to `plans/PLAN011_COMPLETE.md`

11. **`PLAN011_execution_status.md`** (unknown lines)
    - **Status:** COMPLETE (companion to PLAN011)
    - **Action:** Archive to `plans/PLAN011_execution_status.md`

12. **`PLAN008_sprint3_completion_report.md`** (unknown lines)
    - **Status:** COMPLETE (sprint report)
    - **Action:** Archive to `plans/PLAN008_sprint3_completion_report.md`

13. **`_attitude_adjustment.md`** (unknown lines)
    - **Status:** COMPLETE - Analysis results supersede
    - **Action:** Archive to `analysis/attitude_adjustment_input.md`

14. **`_context_engineering.md`** (unknown lines)
    - **Status:** COMPLETE - Guidance incorporated into workflows
    - **Action:** Archive to `analysis/context_engineering.md`

15. **`_toml_directory_creation.md`** (unknown lines)
    - **Status:** COMPLETE - Pattern documented in increment2_zero_config_analysis
    - **Action:** Archive to `analysis/toml_directory_creation.md`

**Impact:** Removes ~6,000-7,000 lines from WIP, improving discoverability of active work

---

## Priority 3: Keep Active (No Action Needed)

### In-Progress Analysis Documents (Keep in WIP)

1. **`TECH_DEBT_REVIEW_2025-11-02.md`** (993 lines)
   - **Status:** ACTIVE - Being updated today (HIGH-005, MED-004 resolved)
   - **Action:** Keep until debt resolved, then archive

2. **`TECHNICAL_DEBT_REPORT.md`** (809 lines)
   - **Status:** SUPERSEDED by TECH_DEBT_REVIEW_2025-11-02.md
   - **Action:** Archive to `analysis/TECHNICAL_DEBT_REPORT_2025-10.md`

3. **`mixer_technical_debt_analysis.md`** (354 lines)
   - **Status:** ACTIVE - wkmp-ap refactoring ongoing
   - **Action:** Keep until resolved, then archive

4. **`wkmp-ap_technical_debt_report.md`** (unknown lines)
   - **Status:** Likely COMPLETE or SUPERSEDED
   - **Action:** Review, then archive to `analysis/wkmp_ap_technical_debt_report.md`

### Active Specifications (Keep in WIP Until Ready for /docs)

1. **`SPEC_spec017_compliance_remediation.md`** (998 lines)
   - **Status:** ACTIVE SPEC - Remediation plan for SPEC017 compliance
   - **Action:** Move to /docs as SPEC028 when complete

2. **`SPEC_import_progress_ui_enhancement.md`** (478 lines)
   - **Status:** ACTIVE SPEC - Import UI improvements
   - **Action:** Move to /docs as SPEC029 when complete

3. **`SPEC024_human_readable_time_implementation.md`** (477 lines)
   - **Status:** ACTIVE IMPL - Implementation details for SPEC024
   - **Action:** Move to /docs as IMPL015 when complete

4. **`SPEC024-wkmp_ap_technical_debt_remediation.md`** (837 lines)
   - **Status:** ACTIVE - wkmp-ap debt remediation plan
   - **Action:** Keep until resolved, then archive

5. **`PLAN_sqlx_0.8_upgrade.md`** (unknown lines)
   - **Status:** ACTIVE PLAN - SQLx upgrade planning
   - **Action:** Keep until upgrade complete, then archive

### Reference Documents (Keep in WIP)

1. **`_user_story.md`** (unknown lines)
   - **Status:** TEMPLATE/REFERENCE
   - **Action:** Keep (reference material)

2. **`mixer_architecture_review.md`** (453 lines)
   - **Status:** REFERENCE - Source material for SPEC001 extraction
   - **Action:** Keep until SPEC001 created, then archive

3. **`increment2_zero_config_analysis.md`** (470 lines)
   - **Status:** REFERENCE - Source material for ADR-003 extraction
   - **Action:** Keep until ADR-003 created, then archive

4. **`_wkmp_ai_ui_architecture_clarification.md`** (437 lines)
   - **Status:** REFERENCE - On-demand microservice architecture notes
   - **Action:** Keep (may extract to SPEC### later)

---

## Extraction Action Plan

### Phase 1: Critical Gaps (Week 1)

1. **Create SPEC001-architecture.md** (2-3 hours)
   - Consolidate from mixer_architecture_review.md + CLAUDE.md + increment2_zero_config_analysis.md
   - Target: <400 lines
   - Review for technical accuracy
   - Add to DEV_QUICKSTART.md Required Reading

2. **Create ADR-003-zero_configuration_strategy.md** (1-2 hours)
   - Extract from increment2_zero_config_analysis.md
   - Target: <250 lines
   - Document decision rationale and consequences

3. **Update DEV_QUICKSTART.md** (15 min)
   - Replace CLAUDE.md reference with SPEC001 link
   - Verify Required Reading list is complete

### Phase 2: Debt Management (Week 2)

4. **Create GUIDE004-technical_debt_management.md** (2-3 hours)
   - Extract patterns from TECH_DEBT_REVIEW_2025-11-02.md
   - Target: <300 lines
   - Focus on recurring patterns and remediation strategies

5. **Archive completed analysis documents** (30 min)
   - Use `/archive` workflow for 15 documents
   - Update REG002_archive_index.md

### Phase 3: Cleanup (Week 3)

6. **Review and archive remaining completed work** (1 hour)
   - TECHNICAL_DEBT_REPORT.md → archive
   - wkmp-ap_technical_debt_report.md → archive (if complete)
   - Update REG002_archive_index.md

7. **Verify WIP reduction** (15 min)
   - Target: <15 active WIP documents
   - All active documents have clear completion criteria

---

## Success Metrics

### Before (Current State)
- **WIP documents:** 29 files, ~14,090 lines
- **Required Reading gap:** No SPEC001, no debt guide, no ADR-003
- **Archive discoverability:** Poor (no workflows/README.md)

### After (Target State)
- **WIP documents:** <15 files, ~7,000 lines (50% reduction)
- **Required Reading complete:** SPEC001, ADR-003, GUIDE004 in /docs
- **Archive discoverability:** Excellent (workflows/README.md + improved REG002 references)

### Onboarding Impact
- **Before:** New developers confused by 29 WIP documents, missing architecture doc
- **After:** Clear Required Reading path, complete architecture coverage, archived clutter

---

## Recommendations Summary

### Extract to /docs (Required Reading)
1. ✅ **SPEC001-architecture.md** - Consolidate from 3 WIP sources (<400 lines)
2. ✅ **ADR-003-zero_configuration_strategy.md** - Extract from increment2_zero_config (<250 lines)
3. ✅ **GUIDE004-technical_debt_management.md** - Extract patterns from debt reviews (<300 lines)

### Archive to archive-branch (50% WIP Reduction)
- 15 completed analysis documents (~6,000-7,000 lines)
- Use `/archive` workflow for batch operation

### Keep Active in WIP
- 6 active specifications/plans (SPEC_*, PLAN_*)
- 2 active debt reports (will archive when resolved)
- 4 reference documents (source material for extractions)

**Total Estimated Effort:** 8-10 hours over 3 weeks

**Impact:** Dramatically improves documentation discoverability and new developer onboarding

---

**Status:** RECOMMENDATIONS READY FOR REVIEW
**Next Steps:** Approve extraction plan, begin Phase 1 (SPEC001 creation)
