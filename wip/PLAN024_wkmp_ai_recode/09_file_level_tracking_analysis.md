# File-Level Import Tracking and User Approval: Analysis and Specification

**Plan:** PLAN024
**Created:** 2025-11-09
**Purpose:** Analyze user requirements for file-level import tracking, user approval, and intelligent skip logic

**Phase:** Post-Phase 8 Amendment (New Scope Addition)

---

## Executive Summary

**User Requirement:** Add file-level import tracking with confidence scores, user approval workflow, and intelligent skip logic for re-import scenarios.

**Analysis Status:** 🔴 **SIGNIFICANT CONFLICTS IDENTIFIED** - Requires resolution before specification

**Impact:**
- Database schema: +6 columns to `files` table, +2 parameters to `settings` table
- Workflow changes: Add pre-import skip logic, user approval interaction, metadata merging
- Scope expansion: ~15% additional implementation effort (2-3 days)

**Recommendation:** Resolve 5 CRITICAL conflicts, create Amendment 8, add TASK-000 (File-Level Tracking)

---

## User Requirements Analysis

### Requirement Breakdown

**1. File-Level Import Completion Tracking**
- `import_completed_at` (i64 unix epoch milliseconds) - When import succeeded
- Store in `files` table

**2. File-Level Identity Confidence**
- `identity_confidence` (f32 0.0-1.0) - Automatic identification success probability
- Store in `files` table
- **Question:** How does file-level confidence relate to per-passage identity confidence?

**3. User Approval Timestamp**
- `user_approved_at` (i64 unix epoch milliseconds) - When user approved metadata
- Store in `files` table
- **Behavior:** User approval protects metadata from future automatic changes

**4. Metadata Collection Tracking**
- `metadata_import_completed_at` (i64 unix epoch milliseconds) - When metadata collection finished
- `metadata_confidence` (f32 0.0-1.0) - How successful metadata collection was
- Store in `files` table
- **Question:** How does this differ from existing `metadata_completeness`?

**5. Skip Logic Based on User Approval**
- If `user_approved_at IS NOT NULL` → Skip entire import process
- Absolute protection of user-approved metadata

**6. Skip Logic Based on Identity Confidence**
- Database parameter: `identity_confidence_threshold` (default 0.75)
- If `identity_confidence >= threshold` → Skip file identification portion
- **Question:** What is "file identification portion"? Files don't have identity (passages do)

**7. Skip Logic Based on Metadata Confidence**
- Database parameter: `metadata_confidence_threshold` (default 0.66)
- If `metadata_confidence < threshold` → Re-run metadata collection, merge results
- If `metadata_confidence >= threshold` → Skip metadata collection

**8. Low-Confidence Flagging and User Interaction**
- Files with `identity_confidence < threshold` → Flag for user approval during import
- User approves or disapproves automatic identity determination
- **Implies:** Interactive import process (not fully automatic)

**9. Metadata Merging Logic**
- New metadata collection overwrites existing where same field collected
- Old metadata preserved where new collection fails
- **Example:**
  - Existing: `{title: "Song A", artist: "Artist B", album: NULL}`
  - New collection: `{title: "Song A", artist: NULL, genre: "Rock"}`
  - Merged: `{title: "Song A", artist: "Artist B", album: NULL, genre: "Rock"}`

---

## Conflict Analysis

### CONFLICT-001: File-Level vs Passage-Level Identity Confidence

**Severity:** 🔴 CRITICAL

**Existing Specification:**
- REQ-AI-021, REQ-AI-024: `identity_confidence` is per-passage (Bayesian MBID resolution)
- passages.identity_confidence REAL (0.0-1.0)
- **Rationale:** Single audio file can contain multiple passages, each with different Recording [ENT-MB-020] identities

**User Requirement:**
- `identity_confidence` at file level (files.identity_confidence)
- Used for skip logic: "Skip file identification if confidence high"

**Conflict:**
- A file with 3 passages might have confidences: [0.95, 0.45, 0.82]
- What is the file-level confidence? Average? Minimum? Maximum?
- What does "skip file identification" mean when identification is per-passage?

**Proposed Resolution Options:**

**Option A: Aggregate Passage Confidences**
```sql
-- File-level confidence = minimum of all passage confidences
files.identity_confidence = MIN(passages.identity_confidence WHERE file_id = files.guid)
```
- **Rationale:** Conservative (file only "confident" if ALL passages confident)
- **Skip Logic:** If file-level confidence >= 0.75, skip re-fingerprinting ALL passages
- **Risk:** One low-confidence passage forces re-import of entire file

**Option B: Separate File vs Passage Identity**
```
File Identity: "This file contains audio content we've seen before" (hash-based)
Passage Identity: "This passage is Recording MBID xyz" (fingerprint-based)
```
- **File-level confidence:** Based on file hash match + previous successful import
- **Passage-level confidence:** Based on Bayesian MBID resolution (existing spec)
- **Skip Logic:**
  - File confidence high → Skip file scanning/segmentation
  - Passage confidence high → Skip fingerprinting/AcoustID lookup

**Option C: Rename to Avoid Confusion**
```
files.import_success_confidence (0.0-1.0) - Overall import quality
passages.identity_confidence (0.0-1.0) - MBID resolution quality (existing)
```
- **File confidence:** Aggregate of all passage quality scores
- **Passage confidence:** Unchanged from current spec
- **Skip Logic:** If `import_success_confidence >= 0.75`, skip re-import

**Recommendation:** **Option C** (rename file-level field to avoid conflict)

---

### CONFLICT-002: Metadata Confidence vs Metadata Completeness

**Severity:** 🔴 CRITICAL

**Existing Specification:**
- REQ-AI-045, REQ-AI-084: `metadata_completeness` (0-100%) measures field presence
- passages.metadata_completeness REAL (percentage)
- Formula: `(present_fields / expected_fields) * 100%`

**User Requirement:**
- `metadata_confidence` (f32 0.0-1.0) - How successful metadata collection/fusion was
- Used for skip logic and re-import threshold

**Conflict:**
- Are these the same concept (percentage vs 0.0-1.0)?
- Or different: completeness (quantity) vs confidence (quality)?

**Proposed Resolution:**

**Option A: Same Concept, Different Scale**
```
metadata_confidence = metadata_completeness / 100.0
// 85% completeness = 0.85 confidence
```

**Option B: Different Concepts**
```
metadata_completeness (0-100%) - How many fields filled
metadata_confidence (0.0-1.0) - How trustworthy the filled fields are

Example:
  completeness = 100% (all fields filled)
  confidence = 0.45 (but filled from low-quality sources)
```

**Option C: Composite Score**
```
metadata_confidence = (
    metadata_completeness * 0.5 +
    average(field_confidence_scores) * 0.5
)
```

**Recommendation:** **Option B** (different concepts) with **Option C** formula for file-level confidence

---

### CONFLICT-003: File-Level vs Passage-Level Metadata

**Severity:** 🟡 HIGH

**Existing Specification:**
- Metadata (title, artist, album) is per-passage (passages table, REQ-AI-030-034)
- **Rationale:** Multi-song files have different metadata per passage

**User Requirement:**
- File-level metadata confidence and tracking
- User approval protects "audio file's metadata" from changes

**Conflict:**
- What is "file's metadata" when file has 3 passages with different titles/artists?

**Proposed Resolution:**

**Option A: File Metadata = Aggregate Passage Metadata Confidence**
```sql
files.metadata_confidence = AVG(passages.metadata_completeness) / 100.0
```
- User approval of file → Protects ALL passages from metadata changes

**Option B: File Metadata Exists Separately**
```
files table: Container-level metadata (format, duration, hash)
passages table: Content-level metadata (title, artist, album per passage)
```
- File-level confidence = quality of container parsing
- Passage-level confidence = quality of content identification
- User approval only protects passage metadata (more granular)

**Recommendation:** **Option A** (aggregate) for simplicity, user approves entire file

---

### CONFLICT-004: Interactive User Approval vs Automatic Processing

**Severity:** 🟡 HIGH

**Existing Specification:**
- REQ-AI-010-013: Automatic per-song import workflow (no user interaction)
- Error handling is automatic (skip failed passages, continue)
- No UI interaction during import specified

**User Requirement:**
- "Files with low degree of automatic identification confidence shall be flagged and presented to the user during the import process for user approval or disapproval"
- **Implies:** Import process pauses, waits for user input

**Conflict:**
- Existing spec: Automatic, batch processing
- User requirement: Interactive, wait-for-approval

**Proposed Resolution:**

**Option A: Deferred User Approval (Post-Import)**
```
During Import:
  - Flag low-confidence passages (validation_status = "ManualReviewRequired")
  - Continue automatic processing (no pause)

After Import:
  - UI displays flagged passages
  - User approves/rejects/edits metadata
  - Approval timestamp recorded
```

**Option B: Interactive Import (Pause on Low Confidence)**
```
During Import:
  - Detect low confidence (< threshold)
  - Emit SSE event: "UserApprovalRequired"
  - Pause import workflow
  - Wait for user response via API call
  - Resume on approval/rejection
```

**Option C: Hybrid (Batch with Review Phase)**
```
Phase 1: Automatic Import (no pauses)
  - Process all files
  - Flag low-confidence as "PendingReview"

Phase 2: User Review (interactive)
  - UI presents flagged files
  - User batch approves/rejects
  - Re-import rejected files with manual corrections
```

**Recommendation:** **Option A** (deferred approval) - aligns with existing automatic workflow, adds post-import review phase

---

### CONFLICT-005: Metadata Merging Logic Specification Gap

**Severity:** 🟡 HIGH

**Existing Specification:**
- REQ-AI-030-034: Field-wise weighted selection for metadata fusion
- No re-import or metadata merging logic specified
- No temporal merging (multiple import sessions)

**User Requirement:**
- Re-collect metadata if `metadata_confidence < threshold`
- Merge new with existing: new overwrites where collected, old preserved where not

**Specification Gap:**
- HOW to merge? Field-by-field overwrite?
- WHEN to merge? Every import? Only if confidence low?
- Provenance tracking? (Which import session provided which field?)

**Proposed Resolution:**

**Metadata Merge Algorithm:**
```rust
fn merge_metadata(existing: Metadata, new: Metadata) -> Metadata {
    Metadata {
        title: new.title.or(existing.title),  // New overwrites if present
        artist: new.artist.or(existing.artist),
        album: new.album.or(existing.album),
        genre: new.genre.or(existing.genre),
        // Confidence: max of old and new
        title_confidence: new.title_confidence.max(existing.title_confidence),
        // Provenance: track which import session updated field
        title_source: if new.title.is_some() {
            format!("Import-{}", new_session_id)
        } else {
            existing.title_source
        },
    }
}
```

**Merge Trigger:**
```
IF user_approved_at IS NOT NULL:
    SKIP (user approval protects metadata)
ELSE IF metadata_confidence >= metadata_confidence_threshold:
    SKIP (confidence sufficient)
ELSE:
    RE-COLLECT and MERGE
```

**Recommendation:** Implement merge algorithm, add provenance tracking per field

---

## Gap Analysis

### GAP-001: File Hash-Based Duplicate Detection

**Severity:** 🔴 CRITICAL

**Current Specification:**
- REQ-AI-075-06 mentions UI state "Skipping duplicate file..." but no functional spec
- files.hash exists but no duplicate detection logic specified

**User Requirement:**
- "When an audio file has been identified by hash code as already processed..."
- Skip logic depends on hash-based detection

**Required Specification:**
- Hash algorithm (SHA-256 already in files.hash)
- Duplicate detection query
- Skip behavior when duplicate found

**See:** Earlier analysis in conversation identified this as specification gap

---

### GAP-002: User Approval API Endpoints

**Severity:** 🟡 HIGH

**Current Specification:**
- No API endpoints for user approval defined
- REQ-AI-070-073 define SSE events, but no approval endpoints

**User Requirement:**
- User approves or disapproves during/after import
- Approval timestamp recorded

**Required Specification:**
```
POST /import/passages/{passage_id}/approve
POST /import/passages/{passage_id}/reject
POST /import/files/{file_id}/approve  // Approves all passages
```

**Response:**
- Update `user_approved_at` timestamp
- Protect metadata from future automatic changes
- Emit SSE event: "MetadataApproved"

---

### GAP-003: Re-Import Workflow

**Severity:** 🟡 HIGH

**Current Specification:**
- Import workflow is one-shot (Phase 0-6)
- No re-import logic

**User Requirement:**
- Re-run metadata collection if confidence < threshold
- Merge with existing metadata

**Required Specification:**
- Detect low confidence (query passages where metadata_confidence < threshold)
- Re-run Phases 2-5 (skip Phase 0 boundary detection, Phase 6 validation)
- Merge results with existing metadata (preserve user-approved fields)

---

### GAP-004: Confidence Threshold Configuration

**Severity:** 🟠 MEDIUM

**Current Specification:**
- REQ-AI-024: Hardcoded thresholds (0.7 low confidence, 0.85 conflict)
- No database parameters for thresholds

**User Requirement:**
- `identity_confidence_threshold` (default 0.75)
- `metadata_confidence_threshold` (default 0.66)
- Stored in settings table

**Required Specification:**
- PARAM-AI-005: identity_confidence_threshold
- PARAM-AI-006: metadata_confidence_threshold
- Update REQ-AI-024 to reference configurable thresholds

---

### GAP-005: File-Level vs Passage-Level Separation

**Severity:** 🟠 MEDIUM

**Current Specification:**
- Focus is on passages (playable segments)
- Files are containers, minimal metadata

**User Requirement:**
- File-level import tracking and confidence
- Implies file is unit of import, not passage

**Clarification Needed:**
- Is import unit the file or the passage?
- Should skip logic be file-level or passage-level?

**Proposed:**
- **File-level tracking:** import_completed_at, user_approved_at (container metadata)
- **Passage-level confidence:** identity_confidence, metadata_confidence (content metadata)
- **Skip logic:** Check file-level approval first, then passage-level confidence

---

## Ambiguities Requiring User Clarification

### AMBIGUITY-001: Definition of "File Identification"

**User Statement:** "Skip file contents identification portion of the import process"

**Question:** What is "file identification" vs "passage identification"?

**Possible Interpretations:**
1. File identification = File hash calculation + database lookup
2. File identification = Aggregate of all passage identities (Recording MBIDs)
3. File identification = Container parsing (format, duration, channels)

**Recommended Clarification:** Use interpretation #2 (aggregate passage identities)

---

### AMBIGUITY-002: Metadata Collection "Completion"

**User Statement:** "Metadata import was completed along with an f32 0.0 to 1.0 estimation of how successful the automatic metadata collection / fusion for the audio file was"

**Question:** What constitutes "completed"?

**Possible Interpretations:**
1. Completed = All phases (0-6) finished without errors
2. Completed = Metadata fields populated (regardless of confidence)
3. Completed = Metadata confidence >= threshold

**Recommended Clarification:** Use interpretation #2 (phases finished, regardless of quality)

---

### AMBIGUITY-003: User Approval Granularity

**User Statement:** "User approval shall always protect an audio-file's metadata"

**Question:** Does user approval protect:
1. Entire file (all passages within file)?
2. Individual passages?
3. Specific metadata fields?

**Recommended Clarification:** Approve entire file (all passages) - simpler UX

---

### AMBIGUITY-004: Merge Behavior for Conflicts

**User Statement:** "New data to overwrite existing metadata where the same data is collected"

**Question:** What if new data CONFLICTS with existing data?

**Example:**
- Existing: `title = "Song A"` (confidence 0.8)
- New: `title = "Song B"` (confidence 0.6)
- Should lower-confidence new data overwrite higher-confidence existing data?

**Recommended Clarification:**
```
IF new_confidence > existing_confidence:
    OVERWRITE
ELSE:
    PRESERVE existing
```

---

## Proposed Database Schema Extensions

### Files Table Extensions

**Add to `files` table:**

```sql
-- Import Tracking
import_completed_at INTEGER,  -- Unix epoch milliseconds (i64)
import_success_confidence REAL,  -- 0.0-1.0 aggregate quality (NOT identity_confidence)

-- User Approval
user_approved_at INTEGER,  -- Unix epoch milliseconds (i64), NULL = not approved

-- Metadata Collection Tracking
metadata_import_completed_at INTEGER,  -- Unix epoch milliseconds (i64)
metadata_confidence REAL,  -- 0.0-1.0 fusion quality

-- Skip Logic Helpers
last_reimport_attempt_at INTEGER,  -- Unix epoch milliseconds (i64)
reimport_attempt_count INTEGER DEFAULT 0  -- Prevent infinite re-import loops
```

**Rationale:**
- Renamed `identity_confidence` → `import_success_confidence` (avoid conflict with passage-level)
- Added `reimport_attempt_count` to prevent infinite loops if confidence never improves

---

### Passages Table Extensions

**NO CHANGES** - Existing passages table already has:
- `identity_confidence` (Bayesian MBID resolution)
- `metadata_completeness` (field presence percentage)
- `overall_quality_score` (validation quality)
- `validation_status` (Pass/Warning/Fail)

**Clarification:** File-level confidence aggregates from passage-level scores

---

### Settings Table Extensions

**Add parameters:**

```sql
-- PARAM-AI-005
INSERT INTO settings (key, value, description, unit, default_value) VALUES (
    'import_success_confidence_threshold',
    '0.75',
    'Minimum file-level import success confidence to skip re-import. File import success confidence aggregates all passage quality scores. Lower = more re-imports.',
    'probability',
    '0.75'
);

-- PARAM-AI-006
INSERT INTO settings (key, value, description, unit, default_value) VALUES (
    'metadata_confidence_threshold',
    '0.66',
    'Minimum file-level metadata fusion confidence to skip metadata re-collection. Lower = more re-collection attempts.',
    'probability',
    '0.66'
);

-- PARAM-AI-007
INSERT INTO settings (key, value, description, unit, default_value) VALUES (
    'max_reimport_attempts',
    '3',
    'Maximum automatic re-import attempts before flagging for manual review. Prevents infinite re-import loops.',
    'count',
    '3'
);
```

---

## Proposed Workflow Modifications

### New Phase -1: Pre-Import Skip Logic

**Before Phase 0 (Passage Boundary Detection):**

```rust
async fn check_skip_import(file_path: &Path, db: &Connection) -> SkipDecision {
    // Step 1: Compute file hash
    let hash = compute_sha256(file_path)?;

    // Step 2: Check database for existing file
    let existing = db.query_row(
        "SELECT guid, user_approved_at, import_success_confidence,
                metadata_confidence, reimport_attempt_count, modification_time
         FROM files WHERE hash = ?",
        params![hash],
        |row| { /* ... */ }
    ).optional()?;

    let Some(existing) = existing else {
        return SkipDecision::Proceed; // New file, import normally
    };

    // Step 3: Check modification time
    let current_mtime = file_path.metadata()?.modified()?;
    if current_mtime == existing.modification_time {
        return SkipDecision::Skip("File unchanged since last import");
    }

    // Step 4: Check user approval (ABSOLUTE protection)
    if existing.user_approved_at.is_some() {
        return SkipDecision::Skip("User-approved metadata protected");
    }

    // Step 5: Check import success confidence
    let import_threshold = get_setting(db, "import_success_confidence_threshold")?;
    if existing.import_success_confidence >= import_threshold {
        return SkipDecision::SkipIdentification; // Skip Phases 2-4, run Phase 5-6
    }

    // Step 6: Check metadata confidence
    let metadata_threshold = get_setting(db, "metadata_confidence_threshold")?;
    if existing.metadata_confidence >= metadata_threshold {
        return SkipDecision::SkipMetadata; // Skip Phase 5, run Phase 2-4, 6
    }

    // Step 7: Check re-import attempt limit
    let max_attempts = get_setting_i64(db, "max_reimport_attempts")?;
    if existing.reimport_attempt_count >= max_attempts {
        return SkipDecision::FlagForManualReview; // Stop automatic re-imports
    }

    // Step 8: Proceed with re-import and merge
    SkipDecision::ReimportAndMerge {
        existing_file_id: existing.guid,
        merge_strategy: MergeStrategy::PreserveHighConfidence,
    }
}
```

---

### Modified Phase 7: Post-Import Completion Tracking

**After Phase 6 (Quality Validation), add Phase 7:**

```rust
async fn finalize_import(file: &File, passages: &[Passage], db: &Connection) -> Result<()> {
    // Compute file-level aggregate confidence
    let import_success_confidence = compute_file_confidence(passages);
    let metadata_confidence = compute_metadata_confidence(passages);

    // Update files table
    db.execute(
        "UPDATE files SET
            import_completed_at = ?,
            import_success_confidence = ?,
            metadata_import_completed_at = ?,
            metadata_confidence = ?,
            reimport_attempt_count = reimport_attempt_count + 1
         WHERE guid = ?",
        params![
            unix_epoch_millis(),
            import_success_confidence,
            unix_epoch_millis(),
            metadata_confidence,
            file.guid
        ]
    )?;

    // Check if flagging needed
    let import_threshold = get_setting(db, "import_success_confidence_threshold")?;
    if import_success_confidence < import_threshold {
        flag_for_review(file.guid, "LowImportConfidence", db)?;
        emit_sse_event("FileRequiresReview", &file)?;
    }

    let metadata_threshold = get_setting(db, "metadata_confidence_threshold")?;
    if metadata_confidence < metadata_threshold {
        flag_for_review(file.guid, "LowMetadataConfidence", db)?;
        emit_sse_event("FileRequiresReview", &file)?;
    }

    Ok(())
}

fn compute_file_confidence(passages: &[Passage]) -> f32 {
    if passages.is_empty() {
        return 0.0;
    }

    // Minimum confidence approach (conservative)
    passages.iter()
        .map(|p| {
            // Composite score from identity, metadata, quality
            (p.identity_confidence as f32 * 0.4) +
            (p.metadata_completeness / 100.0 * 0.3) +
            (p.overall_quality_score / 100.0 * 0.3)
        })
        .min_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or(0.0)
}

fn compute_metadata_confidence(passages: &[Passage]) -> f32 {
    if passages.is_empty() {
        return 0.0;
    }

    // Average metadata completeness + average field confidence
    let avg_completeness = passages.iter()
        .map(|p| p.metadata_completeness / 100.0)
        .sum::<f64>() / passages.len() as f64;

    let avg_field_confidence = passages.iter()
        .map(|p| (p.title_confidence + p.artist_confidence) / 2.0)
        .sum::<f64>() / passages.len() as f64;

    ((avg_completeness + avg_field_confidence) / 2.0) as f32
}
```

---

## Requirement Enumeration (per GOV002)

### New Requirements (REQ-AI-009-xxx series)

**REQ-AI-009: File-Level Import Tracking and Skip Logic**

The system SHALL track file-level import status and confidence scores to enable intelligent skip logic and user approval workflows:

**[REQ-AI-009-01]** File-Level Import Completion Tracking
- SHALL record `files.import_completed_at` (i64 unix epoch milliseconds) when all passages processed
- SHALL record `files.import_success_confidence` (f32 0.0-1.0) as minimum of all passage composite scores
- Composite score per passage: `(identity_confidence * 0.4) + (metadata_completeness * 0.3) + (overall_quality_score * 0.3)`

**[REQ-AI-009-02]** File-Level Metadata Collection Tracking
- SHALL record `files.metadata_import_completed_at` (i64 unix epoch milliseconds) when metadata fusion completes
- SHALL record `files.metadata_confidence` (f32 0.0-1.0) as average of passage metadata quality
- Metadata quality = `(metadata_completeness + avg(field_confidences)) / 2.0`

**[REQ-AI-009-03]** User Approval Timestamp
- SHALL provide `files.user_approved_at` (i64 unix epoch milliseconds, NULL = not approved)
- SHALL record timestamp when user explicitly approves file metadata via API
- User approval SHALL protect ALL passages within file from future automatic metadata changes

**[REQ-AI-009-04]** Skip Logic - User Approval (Absolute Priority)
- IF `user_approved_at IS NOT NULL`:
  - SHALL skip entire import process (Phases 0-6)
  - SHALL emit SSE event: "FileSkipped" (reason: "UserApproved")
  - SHALL NOT modify any passage metadata

**[REQ-AI-009-05]** Skip Logic - Modification Time Check
- SHALL check file modification time against `files.modification_time`
- IF modification time unchanged:
  - SHALL skip import (file content unchanged)
  - SHALL emit SSE event: "FileSkipped" (reason: "Unchanged")

**[REQ-AI-009-06]** Skip Logic - Import Success Confidence
- SHALL compare `import_success_confidence` against PARAM-AI-005 threshold (default 0.75)
- IF `import_success_confidence >= threshold` AND `user_approved_at IS NULL`:
  - SHALL skip Phases 2-4 (fingerprinting, identity resolution, metadata fusion)
  - SHALL run Phases 5-6 (flavor synthesis, validation)
  - SHALL emit SSE event: "PartialImport" (phases_skipped: "2-4")

**[REQ-AI-009-07]** Skip Logic - Metadata Confidence
- SHALL compare `metadata_confidence` against PARAM-AI-006 threshold (default 0.66)
- IF `metadata_confidence >= threshold` AND `user_approved_at IS NULL`:
  - SHALL skip Phase 5 (metadata fusion)
  - SHALL run Phases 2-4, 6 (identity resolution, validation)
  - SHALL emit SSE event: "PartialImport" (phases_skipped: "5")

**[REQ-AI-009-08]** Re-Import Attempt Limiting
- SHALL track `files.reimport_attempt_count` (increments each import)
- SHALL compare against PARAM-AI-007 (default max_reimport_attempts = 3)
- IF `reimport_attempt_count >= max_reimport_attempts`:
  - SHALL skip automatic re-import
  - SHALL flag file with `validation_status = "ManualReviewRequired"`
  - SHALL emit SSE event: "FileRequiresReview" (reason: "MaxReimportAttemptsExceeded")

**[REQ-AI-009-09]** Low-Confidence Flagging
- IF `import_success_confidence < PARAM-AI-005` after import:
  - SHALL flag file with `validation_status = "LowImportConfidence"`
  - SHALL emit SSE event: "FileRequiresReview"
- IF `metadata_confidence < PARAM-AI-006` after import:
  - SHALL flag file with `validation_status = "LowMetadataConfidence"`
  - SHALL emit SSE event: "FileRequiresReview"

**[REQ-AI-009-10]** Metadata Merging on Re-Import
- IF re-import triggered (confidence < threshold, not user-approved):
  - SHALL re-run metadata collection (Phases 2-5)
  - SHALL merge new metadata with existing per-field:
    - IF `new_field IS NOT NULL`: Use new value, update confidence
    - IF `new_field IS NULL` AND `existing_field IS NOT NULL`: Preserve existing
  - SHALL update field provenance: `"{field}_source" = "Reimport-{session_id}"`
  - SHALL preserve fields from user-edited passages (future enhancement)

**[REQ-AI-009-11]** Hash-Based Duplicate Detection
- SHALL compute SHA-256 hash of file contents before import
- SHALL query: `SELECT * FROM files WHERE hash = ? AND path != ?`
- IF match found (different path, same content):
  - SHALL emit SSE event: "FileSkipped" (reason: "DuplicateContent")
  - SHALL log duplicate paths for user review
  - SHALL NOT create new file record

---

### New Database Parameters (PARAM-AI-005 through PARAM-AI-007)

**PARAM-AI-005: import_success_confidence_threshold**
- Default: `0.75`
- Unit: probability (0.0-1.0)
- Description: Minimum file-level import success confidence to skip re-import
- Lower values → More re-imports (thoroughness)
- Higher values → Fewer re-imports (efficiency)

**PARAM-AI-006: metadata_confidence_threshold**
- Default: `0.66`
- Unit: probability (0.0-1.0)
- Description: Minimum file-level metadata confidence to skip metadata re-collection
- Lower values → More metadata re-collection
- Higher values → Trust existing metadata more

**PARAM-AI-007: max_reimport_attempts**
- Default: `3`
- Unit: count (integer)
- Description: Maximum automatic re-import attempts before manual review required
- Prevents infinite re-import loops when confidence cannot be improved

---

## Impact Assessment

### Implementation Effort

**New Tasks:**
- **TASK-000:** File-Level Import Tracking (Before TASK-001)
  - Effort: 2 days
  - Deliverable: Pre-import skip logic, file-level confidence aggregation
  - Dependencies: None (can parallelize with TASK-001)

**Modified Tasks:**
- **TASK-019:** Workflow Orchestrator (add Phase -1 and Phase 7)
  - Additional effort: +1 day (total 6 days instead of 5)
  - Add pre-import skip logic, post-import finalization

- **TASK-021:** HTTP API Endpoints (add approval endpoints)
  - Additional effort: +0.5 days (total 2.5 days instead of 2)
  - Add: `POST /import/files/{id}/approve`, `POST /import/files/{id}/reject`

**Total Additional Effort:** 3.5 days (6% increase over 55-day base estimate)

---

### Schedule Impact

**Original Schedule:** 14 weeks (55 days + 11-day buffer)
**Additional Effort:** 3.5 days
**Revised Schedule:** 14 weeks (still within buffer)

**Milestone Impact:**
- M5 (Orchestration Complete, Week 12): +1 day → Week 12 + 1 day
- M6 (Testing Complete, Week 14): No change (buffer absorbs 3.5 days)

**Risk:** If other risks materialize, buffer reduced from 11 days to 7.5 days

---

### Database Schema Impact

**Files Table:**
- +6 columns (import_completed_at, import_success_confidence, user_approved_at, metadata_import_completed_at, metadata_confidence, reimport_attempt_count, last_reimport_attempt_at)
- SPEC031 SchemaSync handles automatic addition

**Settings Table:**
- +3 parameters (PARAM-AI-005, PARAM-AI-006, PARAM-AI-007)

**Migration Complexity:** LOW (column additions only, no data transformation)

---

### Test Coverage Impact

**New Tests Required:**
- File-level confidence aggregation (unit tests)
- Skip logic decision tree (unit + integration tests)
- Metadata merging (unit tests with various scenarios)
- User approval API endpoints (integration tests)
- Re-import attempt limiting (integration tests)

**Estimated Test LOC:** +400 LOC (total test code: 1,400 → 1,800 LOC)

---

## Recommendations

### CRITICAL: Resolve Conflicts Before Amendment

1. **User Clarification Required:**
   - AMBIGUITY-001: Confirm "file identification" = aggregate passage identities
   - AMBIGUITY-003: Confirm user approval protects entire file (all passages)
   - AMBIGUITY-004: Confirm merge uses confidence-based overwrite

2. **Adopt Resolutions:**
   - CONFLICT-001: Use Option C (rename to `import_success_confidence`)
   - CONFLICT-002: Use Option B (different concepts) with Option C formula
   - CONFLICT-003: Use Option A (aggregate passage metadata confidence)
   - CONFLICT-004: Use Option A (deferred user approval, post-import review)
   - CONFLICT-005: Implement proposed merge algorithm

3. **Address Gaps:**
   - GAP-001: Specify hash-based duplicate detection (REQ-AI-009-11)
   - GAP-002: Add user approval API endpoints to TASK-021
   - GAP-003: Specify re-import workflow (REQ-AI-009-10)
   - GAP-004: Add confidence threshold parameters (PARAM-AI-005, 006, 007)

### Create Amendment 8

**File:** `wip/PLAN024_wkmp_ai_recode/02_specification_amendments.md`

**Content:**
- 7 amendments to SPEC_wkmp_ai_recode.md (REQ-AI-009-01 through REQ-AI-009-11)
- 6 columns to `files` table schema
- 3 database parameters (PARAM-AI-005, 006, 007)
- 2 API endpoints for user approval
- Metadata merge algorithm specification

### Update Implementation Breakdown

**File:** `wip/PLAN024_wkmp_ai_recode/05_implementation_breakdown.md`

**Changes:**
- Add TASK-000: File-Level Import Tracking (2 days, Week 1)
- Update TASK-019: Add Phase -1 and Phase 7 logic (+1 day)
- Update TASK-021: Add approval endpoints (+0.5 days)
- Update TASK-003: Add 6 columns to files table schema (no time change, SPEC031 handles)

### Update Schedule

**File:** `wip/PLAN024_wkmp_ai_recode/06_effort_and_schedule.md`

**Changes:**
- Infrastructure phase: 8.5 days → 10.5 days (add TASK-000)
- Orchestration phase: 9.5 days → 10.5 days (TASK-019 +1 day, TASK-021 +0.5 days)
- Total base effort: 55 days → 58.5 days
- Total with buffer: 66 days → 70 days (14 weeks unchanged)

---

## Open Questions for User

**Before proceeding with Amendment 8, please confirm:**

1. **File vs Passage Identity:** Is it acceptable to aggregate passage identity confidences into a single file-level score? Or should file and passage identity remain separate concepts?

2. **User Approval Granularity:** Should user approval protect:
   - Entire file (all passages)? ← Recommended
   - Individual passages?
   - Specific metadata fields?

3. **Interactive vs Deferred Approval:** Should low-confidence files:
   - Pause import and wait for user input? (Interactive)
   - Flag for post-import review? ← Recommended (deferred)

4. **Metadata Merge Conflicts:** When new metadata conflicts with existing (different values), should we:
   - Always use new? (User's description)
   - Use higher-confidence value? ← Recommended
   - Prompt user to choose?

5. **Re-Import Trigger:** Should re-import happen:
   - Automatically on next import if confidence < threshold? ← Assumed
   - Only when user explicitly requests?
   - On a scheduled basis (nightly, weekly)?

---

**Document Version:** 1.0 (DRAFT - Awaiting User Approval)
**Last Updated:** 2025-11-09
**Status:** 🔴 **CONFLICTS IDENTIFIED** - User decisions required before amendment

