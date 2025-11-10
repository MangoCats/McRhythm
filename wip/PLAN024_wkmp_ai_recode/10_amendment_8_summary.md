# Amendment 8 Summary: File-Level Import Tracking

**Plan:** PLAN024
**Amendment:** Amendment 8
**Date:** 2025-11-09
**Status:** ✅ APPROVED AND INTEGRATED

---

## Executive Summary

Amendment 8 adds intelligent file-level import tracking, user approval workflow, and skip logic to the WKMP-AI audio import system. This enhancement enables the system to avoid redundant re-processing of previously imported files and provides a deferred approval mechanism for low-confidence imports.

**Key Changes:**
- 11 new requirements (REQ-AI-009-01 through REQ-AI-009-11)
- 1 new task (TASK-000: File-Level Import Tracking)
- 3 modified tasks (TASK-003, TASK-019, TASK-021)
- 7 new database columns on files table
- 3 new database parameters
- +3.5 days effort (absorbed by existing 20% buffer)
- Schedule unchanged: 14 weeks

---

## Requirements Added

### Database Schema (REQ-AI-009-01)

**7 new columns added to files table:**

| Column | Type | Purpose |
|--------|------|---------|
| `import_completed_at` | INTEGER (i64 unix epoch ms) | Timestamp of successful import completion |
| `import_success_confidence` | REAL (f32 0.0-1.0) | Aggregate quality score (MIN of passage composite scores) |
| `metadata_import_completed_at` | INTEGER (i64 unix epoch ms) | Timestamp of metadata collection completion |
| `metadata_confidence` | REAL (f32 0.0-1.0) | Metadata fusion quality score |
| `user_approved_at` | INTEGER (i64 unix epoch ms) | User approval timestamp (absolute protection) |
| `reimport_attempt_count` | INTEGER | Counter to prevent infinite re-import loops |
| `last_reimport_attempt_at` | INTEGER (i64 unix epoch ms) | Last automatic re-import attempt timestamp |

### Database Parameters (REQ-AI-009-02, REQ-AI-009-03)

**3 new parameters in settings table:**

| Parameter | Default | Purpose |
|-----------|---------|---------|
| `PARAM-AI-005: import_success_confidence_threshold` | 0.75 | Minimum confidence to skip full re-import |
| `PARAM-AI-006: metadata_confidence_threshold` | 0.66 | Minimum confidence to skip metadata re-collection |
| `PARAM-AI-007: max_reimport_attempts` | 3 | Maximum automatic re-import attempts before flagging |

### Skip Logic Decision Tree (REQ-AI-009-04 through REQ-AI-009-09)

**7 skip conditions evaluated in priority order:**

1. **User Approval (Absolute Priority)** - If user_approved_at IS NOT NULL, skip entire import
2. **Hash-Based Duplicate Detection** - If file hash unchanged since last import, skip import
3. **Modification Time Check** - If file modification time unchanged, skip import
4. **Import Success Confidence Threshold** - If import_success_confidence ≥ threshold (0.75), skip import
5. **Metadata Confidence Threshold** - If metadata_confidence ≥ threshold (0.66), skip metadata collection only
6. **Re-import Attempt Limiting** - If reimport_attempt_count ≥ max_attempts (3), flag for manual review
7. **Low-Confidence Flagging** - If import_success_confidence < threshold, flag for user review

**Supporting algorithms (not skip conditions):**

8. **Confidence Aggregation Formula** - File import_success_confidence = MIN(passage_composite_scores)
9. **Metadata Merge Algorithm** - Higher confidence metadata overwrites lower confidence metadata (applied during re-import)

### User Approval Workflow (REQ-AI-009-10)

**Deferred approval model (post-import review):**
- System imports files automatically without pausing
- Low-confidence files flagged for review during import
- User reviews flagged files after batch completion
- User can approve or reject flagged files
- Approved files protected from automatic modification

### API Endpoints (REQ-AI-009-11)

**3 new HTTP endpoints:**
- `POST /import/files/{id}/approve` - Approve file, set user_approved_at timestamp
- `POST /import/files/{id}/reject` - Reject file, mark for deletion or manual handling
- `GET /import/files/pending-review` - List all low-confidence files flagging for review

---

## Implementation Changes

### New Task: TASK-000 (File-Level Import Tracking)

**Effort:** 2 days
**Deliverable:** `wkmp-ai/src/services/file_tracker.rs` (350 LOC)
**Dependencies:** None (can parallelize with TASK-001)
**Risk:** LOW-MEDIUM

**Components:**
- Pre-import skip logic (Phase -1, before existing Phase 0)
- File hash computation (SHA-256)
- Confidence aggregation formulas
- Metadata merge algorithm
- Re-import attempt tracking

**Acceptance Criteria:**
- Hash-based duplicate detection works
- Skip logic decision tree evaluates conditions in correct priority order
- File-level confidence aggregation uses MIN of passage composite scores
- Metadata merging preserves higher-confidence values
- Re-import attempt limiting prevents infinite loops

### Modified Task: TASK-003 (Database Schema Sync)

**Change:** Now syncs 24 columns (17 passages + 7 files)
**Effort:** Unchanged (2 days)

**Additional columns:**
- 7 new file-level tracking columns (see schema section above)

### Modified Task: TASK-019 (Workflow Orchestrator)

**Change:** Added Phase -1 (pre-import skip logic) and Phase 7 (post-import finalization)
**Effort:** 5 days → 6 days (+1 day)

**New phases:**
- **Phase -1:** Pre-import skip logic (evaluate 9 skip conditions, emit FileSkipped events)
- **Phase 7:** Post-import finalization (aggregate confidence, flag low-confidence files, update timestamps)

**Existing phases (unchanged):**
- Phase 0: Passage boundary detection
- Phase 1-6: Per-passage processing (extraction, fusion, validation)

### Modified Task: TASK-021 (HTTP API Endpoints)

**Change:** Added 3 user approval endpoints
**Effort:** 2 days → 2.5 days (+0.5 day)

**New endpoints:**
- `POST /import/files/{id}/approve`
- `POST /import/files/{id}/reject`
- `GET /import/files/pending-review`

**Existing endpoints (unchanged):**
- `POST /import/start`
- `GET /import/events` (SSE connection)
- `GET /import/status`

---

## Effort and Schedule Impact

### Effort Breakdown

| Category | Original | Amendment 8 | Change |
|----------|----------|-------------|--------|
| Infrastructure (TASK-000 to TASK-004) | 8.5 days | 10.5 days | +2 days (TASK-000 added) |
| Tier 1 Extractors (TASK-005 to TASK-011) | 17 days | 17 days | No change |
| Tier 2 Fusion (TASK-012 to TASK-015) | 11.5 days | 11.5 days | No change |
| Tier 3 Validation (TASK-016 to TASK-018) | 5.5 days | 5.5 days | No change |
| Orchestration (TASK-019 to TASK-021) | 9.5 days | 11 days | +1.5 days (TASK-019 +1d, TASK-021 +0.5d) |
| Integration & Testing (TASK-022 to TASK-025) | 9 days | 9 days | No change |
| **Total Base Effort** | **55 days** | **58.5 days** | **+3.5 days (+6%)** |
| Buffer (20%) | 11 days | 11.7 days | +0.7 days |
| **Total with Buffer** | **66 days** | **70.2 days** | **+4.2 days** |

### Schedule Impact

**Schedule:** 14 weeks (UNCHANGED)
- Original plan included 20% buffer (11 days)
- Amendment 8 adds 3.5 days of base effort
- Buffer absorbs additional effort (11.7 days available)
- No schedule extension required

### Critical Path Impact

**Critical Path:** 55 days → 56.5 days (+1.5 days)
- TASK-000 does NOT extend critical path (parallelizes with TASK-001)
- TASK-019 extended by 1 day (5 → 6 days) - ON critical path
- TASK-021 extended by 0.5 days (2 → 2.5 days) - ON critical path
- Total critical path extension: 1.5 days

---

## LOC Impact

### Production Code

| Module | Original LOC | Amendment 8 LOC | Change |
|--------|--------------|-----------------|--------|
| File Tracker (TASK-000) | 0 | 350 | +350 (NEW) |
| Workflow Orchestrator (TASK-019) | 600 | 700 | +100 |
| HTTP API Endpoints (TASK-021) | 300 | 400 | +100 |
| **Total Production** | **5,250** | **5,800** | **+550 (+10%)** |

### Test Code

| Module | Original LOC | Amendment 8 LOC | Change |
|--------|--------------|-----------------|--------|
| File Tracker Tests | 0 | 300 | +300 (NEW) |
| Workflow Orchestrator Tests | 120 | 150 | +30 |
| API Endpoint Tests | 80 | 150 | +70 |
| **Total Test** | **1,400** | **1,800** | **+400 (+29%)** |

### Grand Total

**6,650 LOC → 7,600 LOC (+950 LOC, +14%)**

---

## Statistics Summary

### Before Amendment 8

- **Requirements:** 77 (72 original + 5 from Amendments 1-7)
- **Tasks:** 25
- **Production LOC:** 5,250
- **Test LOC:** 1,400
- **Total LOC:** 6,650
- **Base Effort:** 55 days
- **Critical Path:** 55 days
- **Schedule:** 14 weeks
- **Database Parameters:** 4

### After Amendment 8

- **Requirements:** 88 (72 original + 5 from Amendments 1-7 + 11 from Amendment 8)
- **Tasks:** 26
- **Production LOC:** 5,800
- **Test LOC:** 1,800
- **Total LOC:** 7,600
- **Base Effort:** 58.5 days
- **Critical Path:** 56.5 days
- **Schedule:** 14 weeks (UNCHANGED)
- **Database Parameters:** 7

### Changes

- **Requirements:** +11 (+14%)
- **Tasks:** +1 (+4%)
- **Production LOC:** +550 (+10%)
- **Test LOC:** +400 (+29%)
- **Total LOC:** +950 (+14%)
- **Base Effort:** +3.5 days (+6%)
- **Critical Path:** +1.5 days (+3%)
- **Schedule:** No change (buffer absorbs increase)
- **Database Parameters:** +3 (+75%)

---

## Key Formulas

### Confidence Aggregation

**File-level import success confidence:**
```
import_success_confidence = MIN(passage_composite_scores)
```

**Passage composite score:**
```
composite_score = (identity_confidence * 0.4)
                + (metadata_completeness / 100.0 * 0.3)
                + (overall_quality_score / 100.0 * 0.3)
```

**File-level metadata confidence:**
```
metadata_confidence = (avg_metadata_completeness + avg_field_confidence) / 2.0
```

### Metadata Merge Algorithm

```
FOR each metadata field (title, artist, album, genre):
    IF new_field IS NOT NULL AND new_field_confidence > existing_field_confidence:
        Use new_field value
        Update field_confidence = new_field_confidence
    ELSE IF new_field IS NULL AND existing_field IS NOT NULL:
        Preserve existing_field value (no overwrite with NULL)
    ELSE IF new_field IS NOT NULL AND new_field_confidence <= existing_field_confidence:
        Preserve existing_field value (higher confidence wins)
```

---

## Risk Assessment

### New Risks Introduced

**RISK-000: Skip Logic Complexity (LOW)**
- 9 skip conditions evaluated in priority order
- Risk: Logic errors could skip files incorrectly or re-import unnecessarily
- Mitigation: Comprehensive unit tests for all 9 conditions, integration tests for decision tree

**RISK-001: Confidence Threshold Tuning (LOW-MEDIUM)**
- Default thresholds (0.75, 0.66) may not be optimal for all use cases
- Risk: Too high = unnecessary re-imports, too low = accept poor quality
- Mitigation: Thresholds configurable via settings table, documentation includes tuning guidance

### Risks Mitigated

**BENEFIT-000: Redundant Processing Eliminated**
- Skip logic prevents re-processing of unchanged files
- Reduces import time for large libraries with few changes
- User approval protects manually curated metadata

---

## Documents Modified

1. **02_specification_amendments.md** - Added Amendment 8 (REQ-AI-009-01 through REQ-AI-009-11)
2. **05_implementation_breakdown.md** - Added TASK-000, updated TASK-003/019/021, updated LOC estimates
3. **06_effort_and_schedule.md** - Updated effort breakdown, schedule timeline, milestones
4. **08_final_plan_approval.md** - Updated executive summary, statistics, milestones, added Amendment 8 section

---

## Approval

**User Approval:** ✅ APPROVED (2025-11-09)
**User Statement:** "Approve and confirm all items. Proceed."

**Approved Resolutions:**
- 5 CRITICAL conflicts resolved (from 09_file_level_tracking_analysis.md)
- 4 ambiguities clarified
- 5 specification gaps filled
- All recommendations approved without modification

---

## Implementation Readiness

**Status:** ✅ READY FOR IMPLEMENTATION

**Prerequisites Satisfied:**
- All requirements enumerated with GOV002-compliant identifiers
- Acceptance tests updated (100% coverage maintained: 88/88 requirements)
- Implementation tasks updated with concrete deliverables
- Effort estimates updated with buffer validation
- Schedule confirmed feasible (14 weeks, buffer absorbs increase)
- Risk assessment updated

**Next Steps:**
1. Proceed with implementation per TASK-000 (File-Level Import Tracking)
2. Execute tasks in order: Infrastructure → Tier 1 → Tier 2 → Tier 3 → Orchestration → Testing
3. Maintain traceability to requirements (REQ-AI-009-01 through REQ-AI-009-11)
4. Achieve 100% test coverage per acceptance test definitions

---

**Document Version:** 1.1 (HIGH-001 fix applied)
**Created:** 2025-11-09
**Updated:** 2025-11-09
**Purpose:** Consolidated summary of Amendment 8 changes for stakeholder review and implementation reference

**HIGH-001 Fix Applied:**
- Clarified skip logic section: 7 skip conditions (not 9)
- Separated confidence aggregation and metadata merge as "Supporting algorithms" (not skip conditions)
- Improved accuracy of skip logic documentation
