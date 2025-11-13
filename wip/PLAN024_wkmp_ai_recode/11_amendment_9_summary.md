# Amendment 9 Summary: Pre-Import File Discovery

**Plan:** PLAN024
**Amendment:** Amendment 9
**Date:** 2025-11-09
**Status:** ✅ APPROVED AND INTEGRATED

---

## Executive Summary

Amendment 9 adds a pre-import file discovery phase to enable accurate percentage-based progress reporting. This enhancement scans all specified directories before processing begins, counts total files, and provides real-time discovery progress via SSE events.

**Key Changes:**
- 5 new requirements (REQ-AI-076-01 through REQ-AI-076-05)
- 4 modified tasks (TASK-019, TASK-020, TASK-021, TASK-022)
- 4 new SSE events (DiscoveryStarted, DiscoveryProgress, DiscoveryComplete, DiscoveryWarning)
- 1 API endpoint modification (POST /import/start request format)
- +1 day effort (absorbed by existing 20% buffer)
- Schedule unchanged: 14 weeks

---

## Problem Statement

**Specification Gap Identified:**
- REQ-AI-075-01 requires "real-time progress updates (percentage complete)"
- Acceptance test TEST-AI-075 shows "Fingerprinting passage 3/10" (implies total file count known)
- No requirement specified how to determine total file count before processing begins
- No specification for POST /import/start request format (single folder? multiple folders? file list?)

**User Impact:**
- Without total file count, cannot calculate accurate progress percentage
- Users have no visibility into import progress until processing begins
- No way to estimate time remaining for large imports

---

## Requirements Added

### REQ-AI-076-01: Import Request Format

**POST /import/start API specification:**

```json
{
  "root_paths": ["/home/user/Music/NewAlbums", "/home/user/Downloads"],
  "recursive": true,
  "file_extensions": ["mp3", "flac", "m4a", "ogg", "wav"],
  "session_id": "uuid-optional"
}
```

**Parameters:**
- `root_paths`: Array of absolute directory paths to scan (required)
- `recursive`: Boolean, scan subdirectories (default: true)
- `file_extensions`: Array of extensions to include (default: ["mp3", "flac", "m4a", "ogg", "wav"])
- `session_id`: Optional UUID for correlation (generated if not provided)

---

### REQ-AI-076-02: Pre-Import File Discovery Phase

**Discovery Phase Workflow:**

1. **Input:** root_paths, recursive flag, file_extensions filter
2. **Process:**
   - Recursively scan all root_paths
   - Filter by file_extensions (case-insensitive)
   - Collect absolute file paths
   - Count total files discovered
3. **Output:**
   - `files_discovered`: Total file count (integer)
   - `file_list`: Array of absolute file paths
4. **SSE Event:** `DiscoveryComplete` with total count

**Execution Order:**
- Discovery Phase (NEW) → Phase -1 → Phase 0-6 → Phase 7

**Performance:**
- Async I/O (tokio::fs) for efficient scanning
- Throttled progress updates (1 event/second max)
- No blocking main thread

---

### REQ-AI-076-03: Discovery Progress Reporting

**4 New SSE Events:**

**1. DiscoveryStarted** (immediate):
```json
{
  "event": "DiscoveryStarted",
  "session_id": "uuid-123",
  "root_paths": ["/home/user/Music"],
  "recursive": true,
  "file_extensions": ["mp3", "flac"],
  "timestamp": 1699564800000
}
```

**2. DiscoveryProgress** (throttled to 1/second):
```json
{
  "event": "DiscoveryProgress",
  "session_id": "uuid-123",
  "files_discovered": 42,
  "current_directory": "/home/user/Music/Subfolder",
  "timestamp": 1699564801000
}
```

**3. DiscoveryComplete** (at end):
```json
{
  "event": "DiscoveryComplete",
  "session_id": "uuid-123",
  "files_discovered": 237,
  "discovery_duration_ms": 2500,
  "timestamp": 1699564802500
}
```

**4. DiscoveryWarning** (on errors):
```json
{
  "event": "DiscoveryWarning",
  "session_id": "uuid-123",
  "warning_type": "PermissionDenied",
  "directory": "/home/user/restricted",
  "message": "Permission denied: /home/user/restricted",
  "timestamp": 1699564800500
}
```

**Throttling:**
- DiscoveryStarted: Immediate (< 100ms)
- DiscoveryProgress: Max 1 event/second
- DiscoveryComplete: Immediate when scan finishes
- DiscoveryWarning: Immediate (not throttled, critical for user feedback)

---

### REQ-AI-076-04: Percentage-Based Progress Calculation

**Formula:**
```
progress_percentage = (files_completed / files_total) × 100
```

Where:
- `files_total` = DiscoveryComplete.files_discovered
- `files_completed` = Count of files processed (successfully or with errors)

**Updated ImportProgress SSE Event:**
```json
{
  "event": "ImportProgress",
  "session_id": "uuid-123",
  "files_completed": 42,
  "files_total": 237,
  "progress_percentage": 17.7,
  "current_file": "/home/user/Music/song.mp3",
  "current_operation": "Fingerprinting passage 1/3",
  "timestamp": 1699564810000
}
```

**Accuracy:**
- Progress percentage accurate to 1 decimal place
- Monotonically increasing (never decreases)
- Starts at 0%, ends at 100%

---

### REQ-AI-076-05: Discovery Error Handling

**3 Error Scenarios:**

**1. Permission Denied:**
- Emit DiscoveryWarning (warning_type="PermissionDenied")
- Skip directory, continue with accessible folders
- Import proceeds with reduced file count

**2. Symlink Loop Detection:**
- Track visited directories (canonical paths)
- Detect circular symlinks (A → B → C → A)
- Emit DiscoveryWarning (warning_type="SymlinkLoop")
- Skip loop, continue discovery

**3. Empty Discovery Result:**
- No files matching file_extensions found
- Emit DiscoveryComplete with files_discovered=0
- Terminate import gracefully (no files to process)
- Emit ImportComplete with status="NoFilesFound"

**Error Handling Principles:**
- Non-blocking: Errors do not abort discovery
- User-visible: All errors emit DiscoveryWarning events
- Graceful degradation: Process accessible files even if some folders fail

---

## Implementation Changes

### Modified Task: TASK-019 (Workflow Orchestrator)

**Change:** Added Discovery Phase (before Phase -1)

**Effort:** 6 days → 7 days (+1 day)

**Deliverable LOC:** 700 → 800 (+100 LOC)

**New Functionality:**
- Async directory scanning (tokio::fs::read_dir)
- File extension filtering (case-insensitive)
- Absolute path collection
- Total file counting
- Discovery SSE event emission
- Error handling (permissions, symlink loops)

**Phases (Updated):**
- **Discovery Phase:** Pre-scan folders, count files, emit SSE events (NEW)
- **Phase -1:** Pre-import skip logic (file tracker integration)
- **Phase 0:** Passage boundary detection
- **Phase 1-6:** Per-passage processing (existing spec)
- **Phase 7:** Post-import finalization (confidence aggregation, flagging)

**Acceptance Criteria:**
- Discovery Phase scans directories recursively
- File count matches actual files discovered
- SSE events emitted in correct order
- Error handling works (permission denied, symlink loops, empty results)
- Progress percentage calculation accurate

---

### Modified Task: TASK-020 (SSE Event System)

**Change:** Added 4 Discovery SSE events

**Effort:** 2.5 days (unchanged, +50 LOC within existing estimate)

**Deliverable LOC:** 250 → 300 (+50 LOC)

**New Event Types:**
- DiscoveryStarted (immediate)
- DiscoveryProgress (throttled to 1/second)
- DiscoveryComplete (immediate)
- DiscoveryWarning (immediate, not throttled)

**Event Count:** 10 original events → 14 total events

**Throttling Rules:**
- ImportProgress: 30 events/second max (unchanged)
- DiscoveryProgress: 1 event/second max (NEW)
- DiscoveryWarning: Not throttled (critical for user feedback)

**Acceptance Criteria:**
- 14 event types emitted
- DiscoveryProgress throttling works (1/sec max)
- All events include session_id for correlation
- Events arrive in correct chronological order

---

### Modified Task: TASK-021 (HTTP API Endpoints)

**Change:** Updated POST /import/start request format

**Effort:** 2.5 days (unchanged, +50 LOC within existing estimate)

**Deliverable LOC:** 250 → 300 (+50 LOC)

**API Changes:**

**Before Amendment 9:**
```json
POST /import/start
{
  "file_path": "/home/user/Music/album.mp3"
}
```

**After Amendment 9:**
```json
POST /import/start
{
  "root_paths": ["/home/user/Music/NewAlbums", "/home/user/Downloads"],
  "recursive": true,
  "file_extensions": ["mp3", "flac", "m4a", "ogg", "wav"],
  "session_id": "uuid-optional"
}
```

**Validation:**
- root_paths: Must be array of absolute paths
- recursive: Must be boolean (default: true)
- file_extensions: Must be array of strings (default: ["mp3", "flac", "m4a", "ogg", "wav"])
- session_id: Optional UUID (generated if not provided)

**Acceptance Criteria:**
- API accepts new request format
- Validation errors return 400 Bad Request
- Discovery Phase initiates before Phase -1
- SSE connection includes Discovery events

---

### Modified Task: TASK-022 (Integration Tests)

**Change:** Added file discovery integration tests

**Effort:** 3 days (unchanged, +100 LOC within existing estimate)

**Deliverable LOC:** 800 → 900 (+100 LOC)

**New Tests:**
- TEST-AI-076-01: Import Request Format (verify POST /import/start)
- TEST-AI-076-02: Discovery Phase Execution (verify folder scanning)
- TEST-AI-076-03: Discovery Progress SSE Events (verify events emitted)
- TEST-AI-076-04: Percentage Progress Calculation (verify formula)
- TEST-AI-076-05: Discovery Error Handling (verify permission errors, symlink loops, empty results)

**Test Coverage:**
- 93/93 requirements (100%, was 88/88)
- Traceability matrix updated
- Test data includes: large directory tree, permission-restricted folders, symlink loops

**Acceptance Criteria:**
- All 5 new acceptance tests passing
- Discovery Phase integration works end-to-end
- Error scenarios handled gracefully
- Progress percentage accurate

---

## Effort and Schedule Impact

### Effort Breakdown

| Category | Before Amendment 9 | After Amendment 9 | Change |
|----------|-------------------|-------------------|--------|
| Infrastructure (TASK-000 to TASK-004) | 10.5 days | 10.5 days | No change |
| Tier 1 Extractors (TASK-005 to TASK-011) | 17 days | 17 days | No change |
| Tier 2 Fusion (TASK-012 to TASK-015) | 11.5 days | 11.5 days | No change |
| Tier 3 Validation (TASK-016 to TASK-018) | 5.5 days | 5.5 days | No change |
| Orchestration (TASK-019 to TASK-021) | 11 days | 12 days | +1 day |
| Integration & Testing (TASK-022 to TASK-025) | 9 days | 9 days | No change |
| **Total Base Effort** | **58.5 days** | **59.5 days** | **+1 day (+2%)** |
| Buffer (20%) | 11.7 days | 11.9 days | +0.2 days |
| **Total with Buffer** | **70.2 days** | **71.4 days** | **+1.2 days** |

### Schedule Impact

**Schedule:** 14 weeks (UNCHANGED)
- Amendment 9 adds 1 day of base effort
- Buffer absorbs additional effort (11.9 days available)
- No schedule extension required

### Critical Path Impact

**Critical Path:** 56.5 days → 57.5 days (+1 day)
- TASK-019 extended by 1 day (6 → 7 days) - ON critical path
- Discovery Phase adds no parallelization opportunities (must run before all processing)
- Total critical path extension: 1 day

---

## LOC Impact

### Production Code

| Module | Before Amendment 9 | After Amendment 9 | Change |
|--------|-------------------|-------------------|--------|
| Workflow Orchestrator (TASK-019) | 700 | 800 | +100 (Discovery Phase) |
| SSE Event System (TASK-020) | 250 | 300 | +50 (Discovery events) |
| HTTP API Endpoints (TASK-021) | 250 | 300 | +50 (Request format) |
| **Total Production** | **5,800** | **6,000** | **+200 (+3%)** |

### Test Code

| Module | Before Amendment 9 | After Amendment 9 | Change |
|--------|-------------------|-------------------|--------|
| Integration Tests (TASK-022) | 800 | 900 | +100 (Discovery tests) |
| **Total Test** | **1,800** | **1,900** | **+100 (+6%)** |

### Grand Total

**7,600 LOC → 7,900 LOC (+300 LOC, +4%)**

---

## Statistics Summary

### Before Amendment 9

- **Requirements:** 88 (72 original + 16 from Amendments 1-8)
- **Tasks:** 26
- **Production LOC:** 5,800
- **Test LOC:** 1,800
- **Total LOC:** 7,600
- **Base Effort:** 58.5 days
- **Critical Path:** 56.5 days
- **Schedule:** 14 weeks
- **SSE Events:** 10 types

### After Amendment 9

- **Requirements:** 93 (72 original + 21 from Amendments 1-9)
- **Tasks:** 26 (unchanged)
- **Production LOC:** 6,000
- **Test LOC:** 1,900
- **Total LOC:** 7,900
- **Base Effort:** 59.5 days
- **Critical Path:** 57.5 days
- **Schedule:** 14 weeks (UNCHANGED)
- **SSE Events:** 14 types

### Changes

- **Requirements:** +5 (+6%)
- **Tasks:** No change
- **Production LOC:** +200 (+3%)
- **Test LOC:** +100 (+6%)
- **Total LOC:** +300 (+4%)
- **Base Effort:** +1 day (+2%)
- **Critical Path:** +1 day (+2%)
- **Schedule:** No change (buffer absorbs increase)
- **SSE Events:** +4 types (+40%)

---

## Workflow Sequence

**Complete Import Workflow (After Amendment 9):**

1. **User Request:** POST /import/start with root_paths array
2. **Discovery Phase** (NEW):
   - Emit DiscoveryStarted
   - Scan directories recursively
   - Filter by file_extensions
   - Emit DiscoveryProgress (throttled to 1/sec)
   - Count total files
   - Emit DiscoveryComplete with files_discovered count
3. **Phase -1** (Skip Logic):
   - For each file: evaluate 7 skip conditions
   - Emit FileSkipped if conditions met
4. **Phase 0-6** (Per-File Processing):
   - For each non-skipped file: extract, fuse, validate
   - Emit ImportProgress with progress_percentage
5. **Phase 7** (Finalization):
   - Aggregate file-level confidence
   - Flag low-confidence files
   - Update file table timestamps

**Progress Visibility:**
- **Before:** "Processing file X" (no total known)
- **After:** "Processing file 42 of 237 (17.7% complete)"

---

## Benefits

### User Benefits

**1. Accurate Progress Reporting:**
- Users see exact percentage complete (e.g., "17.7% complete")
- Estimated time remaining calculable (based on files completed / time elapsed)
- Better user experience for large imports (visibility into progress)

**2. Multi-Folder Import:**
- Can import from multiple directories in single request
- Example: `["~/Music/NewAlbums", "~/Downloads"]`
- Reduces number of import sessions needed

**3. File Extension Filtering:**
- User specifies which formats to import
- Example: `["mp3", "flac"]` (skip `.wav` files)
- Reduces unnecessary processing

**4. Discovery Progress Feedback:**
- Users see scan progress ("Scanning: 42 files found")
- Visibility into discovery phase for large directories
- Warnings for permission errors visible immediately

### Technical Benefits

**1. Progress Calculation Accuracy:**
- Denominator known before processing begins
- No estimation or approximation needed
- Formula simple: `(completed / total) × 100`

**2. Error Handling:**
- Permission errors detected during discovery (not during processing)
- Users notified of inaccessible directories upfront
- Graceful degradation (process accessible files)

**3. API Flexibility:**
- root_paths array supports multiple directories
- recursive flag allows flat or deep scans
- file_extensions filter reduces unnecessary I/O

---

## Risk Assessment

### New Risks Introduced

**RISK-000: Discovery Phase Performance (LOW)**
- Large directory trees may take 5-10 seconds to scan
- Risk: User perceives import as "slow to start"
- Mitigation: DiscoveryProgress events provide feedback, async I/O prevents blocking

**RISK-001: Symlink Loop Handling (LOW)**
- Circular symlinks could cause infinite loops
- Risk: Discovery phase never completes
- Mitigation: Track visited directories (canonical paths), emit DiscoveryWarning and skip loop

**RISK-002: Permission Error Handling (LOW)**
- Some directories may be inaccessible
- Risk: Discovery phase fails entirely
- Mitigation: Emit DiscoveryWarning, continue with accessible folders

### Risks Mitigated

**BENEFIT-000: Progress Visibility**
- Users no longer "blind" to import progress
- Reduces support requests ("Is import working?")
- Improves perceived reliability

---

## Documents Modified

1. **02_specification_amendments.md** - Added Amendment 9 (REQ-AI-076-01 through REQ-AI-076-05)
2. **03_acceptance_tests.md** - Added 5 new tests (TEST-AI-076-01 through TEST-AI-076-05), updated traceability matrix
3. **05_implementation_breakdown.md** - Updated TASK-019/020/021/022, updated LOC estimates
4. **06_effort_and_schedule.md** - Updated effort breakdown, schedule timeline, milestones
5. **08_final_plan_approval.md** - Updated executive summary, statistics, milestones, added Amendment 9 section

---

## Approval

**User Approval:** ✅ APPROVED (2025-11-09)
**User Statement:** "Please create amendment for Option A, make all appropriate modifications to the specification and plan including test coverage."

**Approved Resolutions:**
- Specification gap filled (total file count determination)
- API request format specified (root_paths array)
- Progress reporting mechanism defined (percentage formula)
- Error handling strategies defined (permissions, symlinks, empty results)

---

## Implementation Readiness

**Status:** ✅ READY FOR IMPLEMENTATION

**Prerequisites Satisfied:**
- All 5 requirements enumerated with GOV002-compliant identifiers
- Acceptance tests added (5 new tests, 93/93 coverage maintained)
- Implementation tasks updated with concrete deliverables
- Effort estimates updated with buffer validation
- Schedule confirmed feasible (14 weeks, buffer absorbs increase)
- Risk assessment updated

**Next Steps:**
1. Proceed with implementation per existing task order
2. TASK-019 now includes Discovery Phase (+1 day effort)
3. TASK-020 includes 4 new Discovery SSE events
4. TASK-021 implements new POST /import/start request format
5. TASK-022 includes 5 new acceptance tests

---

**Document Version:** 1.0
**Created:** 2025-11-09
**Purpose:** Consolidated summary of Amendment 9 changes for stakeholder review and implementation reference
