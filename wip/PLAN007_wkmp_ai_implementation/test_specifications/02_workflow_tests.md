# PLAN007: Workflow State Machine Tests

**Test File:** 02_workflow_tests.md
**Requirements Covered:** AIA-WF-010, AIA-WF-020
**Test Count:** 12
**Test Type:** Unit (state machine logic) + Integration (session management)

---

## Test: TC-WF-001 - SCANNING → EXTRACTING Transition

**Requirement:** AIA-WF-010
**Type:** Unit Test
**Priority:** P0

**Given:**
- Import session in SCANNING state
- File discovery complete (N files found)

**When:**
- Scanner emits completion event

**Then:**
- Session transitions to EXTRACTING state
- Progress counter resets to 0/N
- SSE state_changed event emitted

**Verify:**
- `session.state == State::Extracting`
- `session.progress.current == 0`
- `session.progress.total == N`

**Pass Criteria:**
- State transition occurs within 10ms
- SSE event contains correct old_state/new_state

**Implementation File:** `wkmp-ai/src/models/import_session.rs`

---

## Test: TC-WF-002 - EXTRACTING → FINGERPRINTING Transition

**Requirement:** AIA-WF-010
**Type:** Unit Test
**Priority:** P0

**Given:**
- Import session in EXTRACTING state
- All files metadata extracted

**When:**
- Extractor emits completion event

**Then:**
- Session transitions to FINGERPRINTING state
- Progress counter resets for fingerprinting phase

**Verify:**
- `session.state == State::Fingerprinting`
- No files skipped due to extraction errors

**Pass Criteria:**
- Transition only occurs when ALL files processed
- Partial completion does not trigger transition

**Implementation File:** `wkmp-ai/src/models/import_session.rs`

---

## Test: TC-WF-003 - FINGERPRINTING → SEGMENTING Transition

**Requirement:** AIA-WF-010
**Type:** Unit Test
**Priority:** P0

**Given:**
- Import session in FINGERPRINTING state
- All files fingerprinted (or skipped with warning)

**When:**
- Fingerprinter emits completion event

**Then:**
- Session transitions to SEGMENTING state
- Files with fingerprints proceed to segmentation
- Files without fingerprints marked for manual metadata entry

**Verify:**
- `session.state == State::Segmenting`
- Fingerprint success/failure counts accurate

**Pass Criteria:**
- Transition occurs even if some fingerprints failed (graceful degradation)

**Implementation File:** `wkmp-ai/src/models/import_session.rs`

---

## Test: TC-WF-004 - SEGMENTING → ANALYZING Transition

**Requirement:** AIA-WF-010
**Type:** Unit Test
**Priority:** P0

**Given:**
- Import session in SEGMENTING state
- All passages segmented (silence detection complete)

**When:**
- Segmenter emits completion event

**Then:**
- Session transitions to ANALYZING state
- Passage count finalized
- Amplitude analysis begins

**Verify:**
- `session.state == State::Analyzing`
- `session.passages_detected` matches expected count

**Pass Criteria:**
- Single-passage files handled correctly (no segmentation needed)
- Multi-passage files segmented accurately

**Implementation File:** `wkmp-ai/src/models/import_session.rs`

---

## Test: TC-WF-005 - ANALYZING → FLAVORING Transition

**Requirement:** AIA-WF-010
**Type:** Unit Test
**Priority:** P0

**Given:**
- Import session in ANALYZING state
- All passages amplitude-analyzed (lead-in/lead-out detected)

**When:**
- Amplitude analyzer emits completion event

**Then:**
- Session transitions to FLAVORING state
- Essentia analysis begins for Musical Flavor extraction

**Verify:**
- `session.state == State::Flavoring`
- Lead-in/lead-out times stored for each passage

**Pass Criteria:**
- Transition occurs after all passages analyzed
- Amplitude data persisted correctly

**Implementation File:** `wkmp-ai/src/models/import_session.rs`

---

## Test: TC-WF-006 - FLAVORING → COMPLETED Transition

**Requirement:** AIA-WF-010
**Type:** Unit Test
**Priority:** P0

**Given:**
- Import session in FLAVORING state
- All passages have Musical Flavor data (from Essentia)

**When:**
- Essentia runner emits completion event

**Then:**
- Session transitions to COMPLETED state
- Final database writes committed
- SSE completed event emitted with summary

**Verify:**
- `session.state == State::Completed`
- `session.end_time` set to current timestamp
- Completion summary includes: files_processed, files_failed, passages_created, duration

**Pass Criteria:**
- Musical Flavor NULL acceptable if Essentia fails (per Decision 1, import should abort if Essentia missing)
- Completion event contains accurate statistics

**Implementation File:** `wkmp-ai/src/models/import_session.rs`

---

## Test: TC-WF-007 - Any State → CANCELLED Transition

**Requirement:** AIA-WF-010 (cancellation mechanism)
**Type:** Integration Test
**Priority:** P0

**Given:**
- Import session in any active state (SCANNING, EXTRACTING, FINGERPRINTING, SEGMENTING, ANALYZING, FLAVORING)
- User triggers cancellation via `DELETE /import/session/{id}`

**When:**
- Cancellation flag set on session

**Then:**
- Session transitions to CANCELLED state after current operation completes
- Partial results preserved (files/passages already imported remain in database)
- SSE cancelled event emitted

**Verify:**
- `session.state == State::Cancelled`
- `session.cancellation_reason` set to "User requested cancellation"
- Database contains partial import data
- No orphaned Tokio tasks

**Pass Criteria:**
- Cancellation completes within 5 seconds
- Graceful termination (no panics or crashes)

**Implementation File:** `wkmp-ai/src/api/import_workflow.rs` (cancellation endpoint)

---

## Test: TC-WF-008 - Error → FAILED Transition

**Requirement:** AIA-WF-010
**Type:** Integration Test
**Priority:** P0

**Given:**
- Import session in any state
- Critical error occurs (e.g., database write failure, out of disk space)

**When:**
- Component emits critical error event

**Then:**
- Session transitions to FAILED state
- Error details logged
- SSE error event emitted
- Partial results preserved (if possible)

**Verify:**
- `session.state == State::Failed`
- `session.error_details` populated with error code, message
- User-facing error message clear and actionable

**Pass Criteria:**
- Non-critical errors (e.g., single file decode failure) do NOT trigger FAILED state
- Only critical errors abort entire import

**Implementation File:** `wkmp-ai/src/models/import_session.rs`

---

## Test: TC-WF-009 - Session State Persistence (In-Memory)

**Requirement:** AIA-WF-020
**Type:** Unit Test
**Priority:** P0

**Given:**
- New import session created

**When:**
- Session data accessed from session manager

**Then:**
- Session state persisted in-memory
- Session data includes: UUID, state, progress, errors, timestamps

**Verify:**
- Session retrievable by UUID
- Session data structure matches specification (AIA-WF-020):
  - `session_id: Uuid`
  - `state: State`
  - `progress: Progress { current, total }`
  - `current_operation: String`
  - `errors: Vec<Error>`
  - `cancellation_flag: bool`
  - `started_at: Timestamp`
  - `ended_at: Option<Timestamp>`

**Pass Criteria:**
- Session data consistent across multiple reads
- No data loss during state transitions

**Implementation File:** `wkmp-ai/src/models/import_session.rs`

---

## Test: TC-WF-010 - Session UUID Generation

**Requirement:** AIA-WF-020
**Type:** Unit Test
**Priority:** P0

**Given:**
- Multiple import sessions created sequentially

**When:**
- Session IDs retrieved

**Then:**
- Each session has unique UUID
- UUID format: RFC 4122 version 4 (random)

**Verify:**
- `session_id_1 != session_id_2`
- UUID validity (via uuid crate validation)

**Pass Criteria:**
- 1000 sequential sessions generate 1000 unique UUIDs (no collisions)

**Implementation File:** `wkmp-ai/src/models/import_session.rs`

---

## Test: TC-WF-011 - Session TTL (1 Hour After Completion)

**Requirement:** AIA-WF-020 (per Phase 2 recommendation)
**Type:** Integration Test
**Priority:** P1

**Given:**
- Import session completed successfully
- Current time = T0

**When:**
- 1 hour + 1 minute elapses (time = T0 + 61 minutes)
- Session cleanup task runs

**Then:**
- Session removed from memory
- `/import/status/{id}` returns 404 Not Found

**Verify:**
- Session exists at T0 + 59 minutes
- Session purged at T0 + 61 minutes
- Memory freed (no leak)

**Pass Criteria:**
- TTL enforced within 1-minute precision
- No dangling references to purged sessions

**Implementation File:** `wkmp-ai/src/services/session_manager.rs`

**Note:** Requires mock time or test clock for time-based testing

---

## Test: TC-WF-012 - Session Inactivity Timeout (10 Minutes)

**Requirement:** AIA-WF-020 (per Phase 2 recommendation)
**Type:** Integration Test
**Priority:** P1

**Given:**
- Import session in COMPLETED/FAILED/CANCELLED state
- Last status check at time = T0

**When:**
- 10 minutes pass with no `/import/status/{id}` requests
- Current time = T0 + 10 minutes + 1 second

**Then:**
- Session marked as orphaned
- Session scheduled for cleanup (1-hour TTL from orphan time)

**Verify:**
- Active polling resets inactivity timer
- Orphaned sessions eventually purged

**Pass Criteria:**
- Inactivity timeout only applies to terminal states (not active imports)
- Active imports never timeout while progressing

**Implementation File:** `wkmp-ai/src/services/session_manager.rs`

---

## Test Execution Notes

**Unit Tests:**
- Mock time dependencies for TTL/timeout tests
- Use in-memory session storage (no persistence)
- Fast execution (<10ms per test)

**Integration Tests:**
- Use Tokio test runtime
- Mock HTTP endpoints for cancellation testing
- Moderate execution time (100ms-1s per test)

**Dependencies:**
- `tokio::test` for async testing
- `mockall` or `mockito` for HTTP mocking
- `chrono` for timestamp testing

---

**End of Workflow Tests**
