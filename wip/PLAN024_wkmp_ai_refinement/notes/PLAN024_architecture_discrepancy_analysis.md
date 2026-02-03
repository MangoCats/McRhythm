# PLAN024 Architecture Discrepancy Analysis

**Date:** 2025-11-13
**Analyst:** Claude Code
**Severity:** CRITICAL
**Category:** Architectural Non-Compliance

---

## Executive Summary

**Critical Finding:** The current wkmp-ai implementation uses **deprecated batch-phase architecture** instead of the **required per-file pipeline architecture** specified in SPEC032 [AIA-ASYNC-020].

**Impact:**
- ❌ User experience degraded (coarse-grained progress: "Phase 3 of 6" instead of "2,581 of 5,736 files")
- ❌ Resource utilization suboptimal (CPU/I/O/Network imbalanced across phases)
- ❌ Cancellation UX poor (can only cancel at phase boundaries, lose partial progress)
- ❌ Memory efficiency reduced (batch phases hold all intermediate data in memory)
- ❌ Error recovery complex (restart entire phase vs. resume from last file)

**Root Cause:** PLAN024 implemented **individual phase services** (10 phases) but failed to **replace the batch orchestration** in `execute_import()`. The new `process_file_plan024()` method exists but is **never called** by the UI workflow.

---

## Specification Requirements (SPEC032)

### Required Architecture: Per-File Pipeline

**[AIA-ASYNC-020]** Parallel per-file processing architecture (lines 559-633):

```
Step 4: PROCESSING (Parallel Worker Pool)
  ├─ Worker 1: File A → 10-Phase Pipeline → Complete
  ├─ Worker 2: File B → 10-Phase Pipeline → Complete
  ├─ Worker 3: File C → 10-Phase Pipeline → Complete
  └─ Worker N: File D → 10-Phase Pipeline → Complete

  Each Worker Processes One File Sequentially Through 10 Phases:
    Phase 1: FILENAME MATCHING  - Check if file exists (reuse fileId)
    Phase 2: HASHING            - Calculate hash, detect duplicates
    Phase 3: EXTRACTING         - Parse metadata tags (ID3/Vorbis/MP4)
    Phase 4: SEGMENTING         - Detect passage boundaries via silence
    Phase 5: FINGERPRINTING     - Generate Chromaprint per passage
    Phase 6: SONG MATCHING      - Combine metadata + fingerprint evidence
    Phase 7: RECORDING          - Write passages to database
    Phase 8: AMPLITUDE          - Detect lead-in/lead-out points
    Phase 9: FLAVORING          - Retrieve musical flavor (AcousticBrainz/Essentia)
    Phase 10: PASSAGES COMPLETE - Mark file as INGEST COMPLETE
```

**Key Requirements:**
- **Per-file sequential pipeline:** Each file goes through all 10 phases before next file starts in same worker
- **Parallelism:** N workers (from `ai_processing_thread_count` setting: CPU_core_count + 1)
- **Progress reporting:** File-level granularity ("2,581 of 5,736 files")
- **Cancellation:** File-level checkpoints (trivial resume from last completed file)
- **Resource balance:** CPU/I/O/Network operations overlap across workers

### Deprecated Architecture: Batch Phases

**[AIA-WF-020]** (DEPRECATED - lines 515):

> The following fine-grained phase states (EXTRACTING, FINGERPRINTING, SEGMENTING, ANALYZING, FLAVORING) are **deprecated** in favor of the unified PROCESSING state with 10-phase per-file pipeline. These legacy states may appear in database schema or logs but represent **obsolete batch-phase architecture**.

---

## Current Implementation Analysis

### What Currently Exists

**File:** `wkmp-ai/src/services/workflow_orchestrator/mod.rs`

**Method:** `execute_import()` (lines 143-237)

**Architecture:** Batch Phase Processing (DEPRECATED)

```rust
pub async fn execute_import(
    &self,
    mut session: ImportSession,
    cancel_token: tokio_util::sync::CancellationToken,
) -> Result<ImportSession> {
    // Phase 1: SCANNING - Discover audio files
    session = self.phase_scanning(session, start_time, &cancel_token).await?;

    // Phase 2: EXTRACTING - Extract metadata
    session = self.phase_extracting(session, start_time, &cancel_token).await?;

    // Phase 3: FINGERPRINTING - Audio fingerprinting (stub)
    session = self.phase_fingerprinting(session, start_time, &cancel_token).await?;

    // Phase 4: SEGMENTING - Passage detection (stub)
    session = self.phase_segmenting(session, start_time, &cancel_token).await?;

    // Phase 5: ANALYZING - Amplitude analysis (stub)
    session = self.phase_analyzing(session, start_time, &cancel_token).await?;

    // Phase 6: FLAVORING - Musical flavor extraction (stub)
    session = self.phase_flavoring(session, start_time, &cancel_token).await?;

    // Phase 7: COMPLETED
    session.transition_to(ImportState::Completed);
    ...
}
```

**Characteristics:**
- ✅ Implements batch phases (ALL files through Phase 1, then ALL files through Phase 2, etc.)
- ❌ Does NOT implement per-file pipeline
- ❌ Progress granularity: Phase-level ("Phase 3 of 6 complete")
- ❌ Cancellation: Phase boundaries only
- ❌ Resource utilization: Imbalanced (CPU-heavy during fingerprinting, network-heavy during flavoring)

### What Was Implemented in PLAN024

**Files:** 10 new service modules (Phases 1-10)

**Locations:**
- `wkmp-ai/src/services/filename_matcher.rs` (324 lines) - Phase 1
- `wkmp-ai/src/services/hash_deduplicator.rs` (619 lines) - Phase 2
- `wkmp-ai/src/services/metadata_merger.rs` (237 lines) - Phase 3
- `wkmp-ai/src/services/passage_segmenter.rs` (379 lines) - Phase 4
- `wkmp-ai/src/services/passage_fingerprinter.rs` (313 lines) - Phase 5
- `wkmp-ai/src/services/passage_song_matcher.rs` (415 lines) - Phase 6
- `wkmp-ai/src/services/passage_recorder.rs` (453 lines) - Phase 7
- `wkmp-ai/src/services/passage_amplitude_analyzer.rs` (341 lines) - Phase 8
- `wkmp-ai/src/services/passage_flavor_fetcher.rs` (367 lines) - Phase 9
- `wkmp-ai/src/services/passage_finalizer.rs` (334 lines) - Phase 10

**Orchestration:**
- `process_file_plan024()` (lines 2141-2385) - Wires 10 phases together for single file
- `process_file_plan024_with_decoding()` (lines 2403-2438) - Wrapper with audio decoding

**Status:**
- ✅ All 10 phase services implemented
- ✅ Per-file orchestration method implemented
- ✅ Audio decoding integration complete
- ✅ 48 unit tests passing
- ❌ **Per-file orchestration method NEVER CALLED by UI workflow**
- ❌ **Old batch orchestration (`execute_import()`) STILL ACTIVE**

---

## Gap Analysis

### Critical Gap: Orchestration Layer Not Replaced

**Issue:** PLAN024 created new services and orchestration method, but **did not replace the UI entry point**.

**Current Workflow:**
1. User clicks "Import Music" in wkmp-ui
2. wkmp-ui opens http://localhost:5723 (wkmp-ai)
3. wkmp-ai UI calls `/api/import/session/start` endpoint
4. API handler calls `WorkflowOrchestrator::execute_import()` ← **BATCH PHASES (DEPRECATED)**
5. Batch phases run: SCANNING → EXTRACTING → FINGERPRINTING → ...

**Expected Workflow (Per SPEC032):**
1. User clicks "Import Music" in wkmp-ui
2. wkmp-ui opens http://localhost:5723 (wkmp-ai)
3. wkmp-ai UI calls `/api/import/session/start` endpoint
4. API handler calls **NEW METHOD** ← **PER-FILE PIPELINE (REQUIRED)**
5. Per-file pipeline runs: N workers × (File → 10 phases) in parallel

**Why This Happened:**
1. PLAN024 focused on **individual phase services** (10 service implementations)
2. PLAN024 added orchestration method (`process_file_plan024`) as **new code**
3. PLAN024 **did not modify API handlers** to call new orchestration
4. PLAN024 **did not remove old orchestration** (`execute_import`)
5. Result: New code exists but is **never invoked**

### Secondary Gap: Progress Reporting Not Updated

**Current Progress Display (Batch Phases):**
```
Session State: EXTRACTING
Phase: 2 of 6 complete
Files Processed: 45% (2,581 of 5,736)
Current File: /music/albums/song.mp3
```

**Required Progress Display (Per-File Pipeline):**
```
Session State: PROCESSING
Files Completed: 2,581 of 5,736 (45%)
Current Workers:
  Worker 1: song_a.mp3 (Phase 5: Fingerprinting)
  Worker 2: song_b.mp3 (Phase 9: Flavoring)
  Worker 3: song_c.mp3 (Phase 3: Extracting)
  Worker 4: song_d.mp3 (Phase 7: Recording)
```

**Why This Matters:**
- User sees **coarse progress** ("Phase 2 of 6") instead of **fine progress** ("2,581 of 5,736 files")
- User cannot see **which files are being processed** in real-time
- User cannot see **parallel work** happening across workers

### Tertiary Gap: Database States Not Aligned

**Current Database States:**
- `ImportState` enum uses: SCANNING, EXTRACTING, FINGERPRINTING, SEGMENTING, ANALYZING, FLAVORING
- These map to batch phases (deprecated per SPEC032:515)

**Required Database States (Per SPEC032):**
- `ImportState` should use: SCANNING (batch), PROCESSING (per-file pipeline), COMPLETED
- EXTRACTING/FINGERPRINTING/etc. are **deprecated** and should not be used

---

## PLAN024 Root Cause Analysis

### Why Did PLAN024 Miss This?

**PLAN024 Scope:**
```
**Code Implementation (Stage One Features):**
3. Ten-Phase Per-File Pipeline:
   - Filename Matching (skip/reuse/new)
   - Hashing (BLAKE3, bidirectional matching_hashes)
   - Metadata Extraction Merging (new overwrites, old preserved)
   - ...
```

**What PLAN024 Delivered:**
- ✅ 10 phase service implementations (3,782 lines)
- ✅ Orchestration method (`process_file_plan024`)
- ✅ Audio decoding integration
- ✅ 48 unit tests

**What PLAN024 Missed:**
- ❌ **Replacing batch orchestration** in `execute_import()`
- ❌ **Updating API handlers** to call new orchestration
- ❌ **Modifying progress reporting** to show per-file granularity
- ❌ **Updating database states** to use PROCESSING instead of fine-grained phases
- ❌ **Integration testing** end-to-end workflow

### Why the Gap Occurred

**1. Incremental Implementation Strategy**

PLAN024 took an **additive approach** (add new code) rather than **replacement approach** (replace old code):

**Additive (What Happened):**
```
Old Code:  execute_import() [batch phases] ← STILL ACTIVE
New Code:  process_file_plan024() [per-file pipeline] ← NEVER CALLED
```

**Replacement (What Should Have Happened):**
```
Old Code:  execute_import() [batch phases] ← REMOVED/DEPRECATED
New Code:  execute_import_per_file() [per-file pipeline] ← ACTIVE
           └─ Calls process_file_plan024() in worker pool
```

**2. Lack of Integration Planning**

PLAN024 focused on **service implementation** but did not include **integration work**:

**What Was Planned:**
- ✅ Phase 1-10: Service implementations
- ⏳ Phase 20-21: Integration testing (marked "optional")

**What Was Missing:**
- ❌ API handler modification (not in plan)
- ❌ Progress reporting update (not in plan)
- ❌ Database state migration (not in plan)
- ❌ Old code removal/deprecation (not in plan)

**3. "Optional" Integration Testing**

PLAN024 marked integration testing as **"optional for initial functionality"**:

> **Remaining Work (Increments 20-21):**
> Integration Testing (~4-6 hours estimated)
> **Status:** Optional for initial functionality. All phases are individually tested and ready for orchestration.

**Problem:** Integration testing would have revealed that:
- New orchestration method never called
- Old batch orchestration still active
- Progress reporting not updated
- End-to-end workflow broken

**4. Missing "Wire to UI" Task**

PLAN024 included **"Wire phases together"** but not **"Wire to UI entry point"**:

**What PLAN024 Had:**
```
Next Steps:
1. Orchestration: Wire the 10 phases together in workflow_orchestrator ← DONE
2. Manual Testing: Process real audio files ← PENDING
3. UI Integration: Connect to wkmp-ai import wizard ← PENDING
```

**What Was Ambiguous:**
- "UI Integration" could mean:
  - **Interpretation A:** Update UI to show better progress (cosmetic)
  - **Interpretation B:** Modify API handlers to call new orchestration (functional)
  - **Reality:** Both are required, but PLAN024 didn't clarify

**5. Assumption of Backward Compatibility**

PLAN024 may have assumed:
- New method coexists with old method
- UI gradually migrates to new method
- Old method deprecated but not removed

**Reality:**
- Per-file pipeline is **architectural replacement**, not feature addition
- Batch phases are **explicitly deprecated** in SPEC032
- Coexistence is not viable (conflicting progress models)

---

## Specification Gap Analysis

### SPEC032 Clarity Issues

**Issue 1: Implementation Sequence Not Specified**

SPEC032 specifies **WHAT** (per-file pipeline) but not **HOW TO MIGRATE**:

**What SPEC032 Says:**
- [AIA-ASYNC-020] Use per-file pipeline architecture
- [AIA-WF-020] Batch phases are deprecated

**What SPEC032 Doesn't Say:**
- How to migrate from batch to per-file
- Whether old code should be removed or coexist
- Which files/functions need modification
- API handler changes required

**Issue 2: Entry Point Not Documented**

SPEC032 shows **pipeline architecture** but not **API integration**:

**Documented:**
```rust
async fn process_file_complete(file: &AudioFile, ...) -> Result<()> {
    // 10 phases here
}
```

**Not Documented:**
- How does `POST /api/import/session/start` invoke this?
- What replaces `execute_import()`?
- How does worker pool get created?
- How does progress reporting integrate?

**Issue 3: Transition Strategy Missing**

SPEC032 declares batch phases deprecated but doesn't provide **migration path**:

**What's Missing:**
- Step-by-step refactoring guide
- Backward compatibility requirements
- Database migration strategy
- UI update requirements

---

## Corrective Action Plan

### Immediate (CRITICAL - 4-6 hours)

**1. Replace Batch Orchestration with Per-File Pipeline**

**Task:** Modify `WorkflowOrchestrator::execute_import()` to use per-file architecture

**Approach:**
```rust
pub async fn execute_import(
    &self,
    mut session: ImportSession,
    cancel_token: tokio_util::sync::CancellationToken,
) -> Result<ImportSession> {
    // Step 1: SCANNING (unchanged - batch file discovery)
    session = self.phase_scanning(session, start_time, &cancel_token).await?;

    // Step 2: PROCESSING (NEW - per-file pipeline with worker pool)
    session = self.phase_processing_per_file(session, start_time, &cancel_token).await?;

    // Step 3: COMPLETED
    session.transition_to(ImportState::Completed);
    ...
}
```

**New Method:**
```rust
async fn phase_processing_per_file(
    &self,
    mut session: ImportSession,
    start_time: std::time::Instant,
    cancel_token: &tokio_util::sync::CancellationToken,
) -> Result<ImportSession> {
    // Get parallelism level from settings
    let parallelism: usize = sqlx::query_scalar(
        "SELECT value FROM settings WHERE key = 'ai_processing_thread_count'"
    )
    .fetch_one(&self.db)
    .await?
    .parse()?;

    // Get list of files from database
    let files: Vec<(String, String)> = sqlx::query_as(
        "SELECT guid, path FROM files WHERE session_id = ? AND status = 'PENDING'"
    )
    .bind(session.session_id.to_string())
    .fetch_all(&self.db)
    .await?;

    // Create worker pool using FuturesUnordered
    let mut tasks = FuturesUnordered::new();
    let mut file_iter = files.iter();

    // Seed initial workers
    for _ in 0..parallelism {
        if let Some((file_id, file_path)) = file_iter.next() {
            let task = self.process_single_file(
                file_id.clone(),
                file_path.clone(),
                session.root_folder.clone(),
                cancel_token.clone(),
            );
            tasks.push(task);
        }
    }

    // Process completions and spawn next file
    let mut completed = 0;
    while let Some(result) = tasks.next().await {
        match result {
            Ok(_) => completed += 1,
            Err(e) => tracing::error!("File processing failed: {}", e),
        }

        // Update progress
        session.update_progress(completed, files.len(), format!("{} of {} files", completed, files.len()));
        self.broadcast_progress(&session, start_time);

        // Maintain parallelism level
        if let Some((file_id, file_path)) = file_iter.next() {
            let task = self.process_single_file(
                file_id.clone(),
                file_path.clone(),
                session.root_folder.clone(),
                cancel_token.clone(),
            );
            tasks.push(task);
        }

        // Check cancellation
        if cancel_token.is_cancelled() {
            break;
        }
    }

    Ok(session)
}

async fn process_single_file(
    &self,
    file_id: String,
    file_path: String,
    root_folder: PathBuf,
    cancel_token: tokio_util::sync::CancellationToken,
) -> Result<()> {
    if cancel_token.is_cancelled() {
        return Ok(());
    }

    // Call PLAN024 orchestration method
    self.process_file_plan024_with_decoding(
        Path::new(&file_path),
        &root_folder,
    ).await
}
```

**Files to Modify:**
- `wkmp-ai/src/services/workflow_orchestrator/mod.rs`

**Estimated Effort:** 3-4 hours

**2. Update Progress Reporting**

**Task:** Change progress model from phase-based to file-based

**Current:**
```rust
ImportState::Extracting
progress: "Phase 2 of 6"
```

**Required:**
```rust
ImportState::Processing
progress: "2,581 of 5,736 files"
```

**Files to Modify:**
- `wkmp-ai/src/models/import_session.rs` (ImportState enum)
- `wkmp-ai/src/services/workflow_orchestrator/mod.rs` (progress updates)
- UI templates (progress display)

**Estimated Effort:** 1-2 hours

**3. Remove Deprecated Batch Phase Methods**

**Task:** Delete or deprecate old phase methods

**Methods to Remove/Deprecate:**
- `phase_extracting()`
- `phase_fingerprinting()`
- `phase_segmenting()`
- `phase_analyzing()`
- `phase_flavoring()`

**Approach:** Add `#[deprecated]` attribute with migration note

**Estimated Effort:** 1 hour

### Short-Term (HIGH PRIORITY - 4-6 hours)

**4. Integration Testing**

**Task:** Test end-to-end workflow with real audio files

**Test Cases:**
1. Single file import (verify all 10 phases execute)
2. Multi-file import (verify parallel processing)
3. Cancel during processing (verify file-level checkpoints)
4. Error handling (verify worker continues after file failure)
5. Progress reporting (verify file counts accurate)

**Estimated Effort:** 2-3 hours

**5. UI Progress Display Update**

**Task:** Show per-worker status in UI

**Required Changes:**
- Add SSE events for per-worker status
- Update UI to show N worker panels
- Display current file + phase per worker

**Estimated Effort:** 2-3 hours

### Long-Term (RECOMMENDED - 2-4 hours)

**6. SPEC032 Enhancement**

**Task:** Add "Migration from Batch to Per-File" section

**Content:**
- Step-by-step refactoring guide
- Entry point integration
- Progress reporting model
- Database state migration

**Estimated Effort:** 2 hours

**7. PLAN025 (Next Implementation)**

**Task:** Learn from PLAN024 gaps

**Recommendations:**
- Include "Wire to Entry Point" as explicit task
- Mark integration testing as MANDATORY
- Add "Old Code Removal" phase
- Define "Done" as "UI calls new code, old code removed"

**Estimated Effort:** Planning overhead (no code changes)

---

## Lessons Learned

### For Future Implementation Plans

**1. "Done" Definition Must Include Integration**

**Bad Definition:**
- ✅ Service implemented
- ✅ Unit tests pass

**Good Definition:**
- ✅ Service implemented
- ✅ Unit tests pass
- ✅ **API handler calls service**
- ✅ **UI displays results**
- ✅ **End-to-end test passes**
- ✅ **Old code removed/deprecated**

**2. Architectural Replacements ≠ Feature Additions**

When replacing architecture:
- **Identify entry point** (e.g., `execute_import()`)
- **Plan replacement strategy** (not coexistence)
- **Remove old code** (not just add new code)
- **Update all callers** (API handlers, UI, tests)

**3. Integration Testing is MANDATORY for Architectural Changes**

Mark integration testing as:
- ❌ NOT "optional for initial functionality"
- ✅ MANDATORY for verifying architectural replacement
- ✅ BLOCKING for considering work "complete"

**4. Specifications Should Include Migration Paths**

When specifications declare something deprecated:
- Document **what replaces it**
- Document **how to migrate**
- Document **which files change**
- Document **entry point modifications**

**5. Progress Models Are Architecture-Dependent**

Different architectures have different progress models:
- Batch phases → Phase-based progress
- Per-file pipeline → File-based progress

Changing architecture **requires** changing progress model.

---

## Conclusion

**Severity:** CRITICAL - User-facing workflow completely uses deprecated architecture

**Scope:** wkmp-ai entire import workflow (affects all users)

**Fix Complexity:** MEDIUM (4-6 hours for core fix, 4-6 hours for polish)

**Risk:** MEDIUM (well-understood fix, clear requirements, existing tests)

**Priority:** IMMEDIATE (user experience severely degraded compared to SPEC032 design)

**Next Steps:**
1. Implement per-file orchestration in `execute_import()` (3-4 hours)
2. Update progress reporting model (1-2 hours)
3. Integration testing (2-3 hours)
4. UI progress display enhancement (2-3 hours)
5. SPEC032 migration guide addition (2 hours)

**Estimated Total Effort:** 10-14 hours to full compliance

---

## Appendix: Code Comparison

### Current (Batch Phases - DEPRECATED)

```rust
// wkmp-ai/src/services/workflow_orchestrator/mod.rs:143-237
pub async fn execute_import(...) -> Result<ImportSession> {
    // SCANNING
    session = self.phase_scanning(session, ...).await?;

    // EXTRACTING (ALL files)
    session = self.phase_extracting(session, ...).await?;

    // FINGERPRINTING (ALL files)
    session = self.phase_fingerprinting(session, ...).await?;

    // SEGMENTING (ALL files)
    session = self.phase_segmenting(session, ...).await?;

    // ANALYZING (ALL files)
    session = self.phase_analyzing(session, ...).await?;

    // FLAVORING (ALL files)
    session = self.phase_flavoring(session, ...).await?;

    // COMPLETED
    session.transition_to(ImportState::Completed);
    ...
}
```

### Required (Per-File Pipeline - SPEC032)

```rust
// PROPOSED IMPLEMENTATION
pub async fn execute_import(...) -> Result<ImportSession> {
    // Step 1: SCANNING (batch file discovery)
    session = self.phase_scanning(session, ...).await?;

    // Step 2: PROCESSING (per-file pipeline with N workers)
    let parallelism = get_thread_count(&self.db).await?;
    let files = get_pending_files(&self.db, session.session_id).await?;

    let mut tasks = FuturesUnordered::new();
    let mut file_iter = files.iter();

    // Seed workers
    for _ in 0..parallelism {
        if let Some(file) = file_iter.next() {
            tasks.push(self.process_file_plan024_with_decoding(file.path, &session.root_folder));
        }
    }

    // Process completions
    while let Some(result) = tasks.next().await {
        // Handle result, update progress, spawn next file
        if let Some(file) = file_iter.next() {
            tasks.push(self.process_file_plan024_with_decoding(file.path, &session.root_folder));
        }
    }

    // Step 3: COMPLETED
    session.transition_to(ImportState::Completed);
    ...
}
```

**Key Difference:**
- Batch: ALL files → Phase N → ALL files → Phase N+1
- Per-File: File A → ALL 10 phases → File B → ALL 10 phases (N workers in parallel)

---

**End of Analysis**
