# PLAN025 Phase 1 Implementation Design

**Created:** 2025-11-10
**Status:** Implementation in progress

## Architecture Overview

### Current (Batch-Phase) Pipeline
```
SCANNING → EXTRACTING → FINGERPRINTING → SEGMENTING → ANALYZING → FLAVORING → COMPLETED
(all files) (all files)  (all files)      (all files)   (all files)  (all files)
```

### PLAN025 (Per-File) Pipeline
```
SCANNING → PROCESSING (4 parallel workers) → COMPLETED
            ↓
         Per file: Verify → Extract → Hash → SEGMENT → Match → Fingerprint →
                   Identify → Amplitude → Flavor → DB
```

## Implementation Strategy

### Step 1: Add execute_import_plan025() method
- Similar to existing execute_import_plan024()
- Calls phase_scanning (reuse)
- Calls phase_processing_plan025 (new)

### Step 2: Implement phase_processing_plan025()
- Load all files from database
- Process using `futures::stream::iter(files).buffer_unordered(4)`
- Each worker calls `process_file_plan025()`

### Step 3: Implement process_file_plan025()
**Per-file pipeline sequence:**
1. **Verify** - Check file exists
2. **Extract** - MetadataExtractor (existing)
3. **Hash** - Compute file hash
4. **SEGMENT** - SilenceDetector (existing) ← KEY: Before fingerprinting
5. **Match** - ContextualMatcher (stub for Phase 1, implement in Phase 2)
6. **Fingerprint** - Per-segment fingerprinting (stub for Phase 1, implement in Phase 3)
7. **Identify** - ConfidenceAssessor (stub for Phase 1, implement in Phase 2)
8. **Amplitude** - AmplitudeAnalyzer (existing)
9. **Flavor** - AcousticBrainzClient/Essentia (existing)
10. **DB** - Store passages

### Phase 1 Scope (Critical)
**Focus:** Pipeline reordering and per-file architecture

**IMPLEMENT:**
- execute_import_plan025() method
- phase_processing_plan025() method with 4 workers
- process_file_plan025() function
- Segmentation BEFORE fingerprinting (use existing SilenceDetector)
- Per-file processing with proper error isolation

**STUB (implement in later phases):**
- PatternAnalyzer - Return empty pattern metadata
- ContextualMatcher - Return empty candidate list
- ConfidenceAssessor - Return default confidence (0.0)
- Per-segment fingerprinting - Use whole-file fingerprinting temporarily

**Tests for Phase 1:**
- TC-U-PIPE-010-01: Verify segmentation before fingerprinting
- TC-U-PIPE-020-01: Verify 4 workers created
- TC-I-PIPE-020-01: Verify per-file processing

## Code Structure

### New Files (Phase 1)
None - all changes in workflow_orchestrator/mod.rs

### New Files (Phase 2+)
- services/pattern_analyzer.rs
- services/contextual_matcher.rs
- services/confidence_assessor.rs

### Modified Files (Phase 1)
- services/workflow_orchestrator/mod.rs - Add execute_import_plan025(), phase_processing_plan025()

## Implementation Sequence

1. ✅ Analyze current architecture
2. ✅ Design per-file pipeline
3. ✅ Implement execute_import_plan025() method
4. ✅ Implement phase_processing_plan025() with 4 workers
5. ✅ Implement process_file_plan025() function
6. ✅ Write unit tests (3 tests: TC-U-PIPE-010-01, TC-U-PIPE-020-01, TC-U-PIPE-020-02)
7. ⏸️ Integration test (TC-I-PIPE-020-01) - Deferred to Phase 1b
8. ✅ Verify no regressions (219 tests pass)

---

## Phase 1 Completion Status

**Implementation Status:** ✅ **COMPLETE**
**Date Completed:** 2025-11-10
**Total Lines Added:** ~320 lines

### What Was Implemented

1. **execute_import_plan025()** method (lines 282-354)
   - Entry point for PLAN025 pipeline
   - Calls phase_scanning (reuse) → phase_processing_plan025 (new) → COMPLETED
   - Proper event broadcasting and progress tracking

2. **phase_processing_plan025()** method (lines 855-1027)
   - **[REQ-PIPE-020]** 4 parallel workers via `futures::stream::buffer_unordered(4)`
   - Loads all files from database
   - Creates stream of file processing tasks
   - Per-file error isolation (failures don't stop other files)
   - Progress tracking with atomic counters
   - Cancellation support

3. **process_file_plan025()** method (lines 1029-1159)
   - **[REQ-PIPE-010]** Segmentation (Step 4) BEFORE fingerprinting (Step 6)
   - 10-step pipeline: Verify → Extract → Hash → **SEGMENT** → Match → Fingerprint → Identify → Amplitude → Flavor → DB
   - Phase 1 stub implementation (creates one passage per file)
   - Proper error handling and logging

4. **SegmentBoundary** struct (lines 43-50)
   - Represents audio segment boundaries
   - Used by segmentation step

5. **Unit Tests** (lines 1237-1319)
   - TC-U-PIPE-010-01: Segmentation before fingerprinting ✅
   - TC-U-PIPE-020-01: 4 workers configured ✅
   - TC-U-PIPE-020-02: Per-file processing architecture ✅

### Test Results

- **Unit Tests:** 4/4 passed (including 3 new PLAN025 tests)
- **All Tests:** 219/219 passed (no regressions)
- **Compilation:** Clean (no warnings)

### What's Stubbed (For Future Phases)

**Phase 2 (High Priority):**
- PatternAnalyzer - Returns empty pattern metadata (Step 5)
- ContextualMatcher - Returns empty candidate list (Step 5)
- ConfidenceAssessor - Returns default confidence 0.0 (Step 7)

**Phase 3 (High Priority):**
- Per-segment fingerprinting - Currently uses whole-file stub (Step 6)

**Phase 4 (Medium Priority):**
- Tick-based timing conversion - Currently uses existing conversion

### Architecture Verification

**✅ REQ-PIPE-010 (Segmentation-First):**
- Segmentation at Step 4 (before fingerprinting at Step 6)
- Pipeline sequence verified by TC-U-PIPE-010-01

**✅ REQ-PIPE-020 (Per-File Pipeline):**
- 4 concurrent workers via `buffer_unordered(4)` (line 983)
- Worker count verified by TC-U-PIPE-020-01
- Per-file architecture verified by TC-U-PIPE-020-02
- Each file processed through all steps before next file

### Integration Test Status

**TC-I-PIPE-020-01** (Integration test for per-file processing):
- **Status:** Deferred to Phase 1b
- **Reason:** Requires actual audio files and full pipeline implementation
- **Plan:** Will implement after Phase 2 components (PatternAnalyzer, ContextualMatcher, ConfidenceAssessor) are complete

---

**Status:** ✅ Phase 1 (Critical) COMPLETE - Ready for Phase 2 implementation
