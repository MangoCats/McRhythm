# Specification Issues Analysis

**Plan ID:** PLAN002
**Date:** 2025-10-27
**Phase:** 2 - Specification Completeness Verification

---

## Executive Summary

**Total Issues Found:** 8
- **CRITICAL:** 0
- **HIGH:** 4
- **MEDIUM:** 3
- **LOW:** 1

**Decision:** Proceed to Phase 3 (Test Definition)
- No CRITICAL issues blocking implementation
- HIGH issues are clarifications that can be resolved during investigation phase
- MEDIUM/LOW issues are implementation details

---

## Issues by Severity

### CRITICAL Issues (Blocks Implementation)

**None found** - Specification provides sufficient information to proceed.

---

### HIGH Issues (High Risk Without Resolution)

#### ISSUE-H1: Decoder Worker Command Mechanism Unspecified

**Requirement:** REQ-FIX-020 (Release decoder chain resources)

**Problem:**
- Bug report doesn't specify how decoder worker receives commands
- Unknown if command channel exists or needs creation
- Unknown message format for "stop chain" command

**Impact:** Can't implement decoder chain clearing without this mechanism

**Resolution Required:**
- **Before Implementation:** Investigate `decoder_worker.rs` to find:
  - Existing command channel (if any)
  - Command message types (enum?)
  - Pattern for adding new commands
- **If no mechanism exists:** Design minimal command pattern
- **Estimated investigation time:** 30-60 minutes

**Severity Justification:** HIGH because implementation approach depends on this, but can be resolved through code investigation (not specification change)

---

#### ISSUE-H2: Mixer Stop/Clear Mechanism Unspecified

**Requirement:** REQ-FIX-030 (Clear mixer state)

**Problem:**
- Bug report doesn't specify how to signal mixer to stop
- Unknown if mixer has stop/clear methods
- Unknown mixer state structure

**Impact:** Can't implement mixer clearing without this mechanism

**Resolution Required:**
- **Before Implementation:** Investigate `mixer.rs` to find:
  - Mixer state structure (fields that need clearing)
  - Existing stop/reset methods (if any)
  - Access pattern (shared Arc? channel?)
- **If no mechanism exists:** Design minimal clearing approach
- **Estimated investigation time:** 30-60 minutes

**Severity Justification:** HIGH because implementation approach depends on this, but can be resolved through code investigation

---

#### ISSUE-H3: Chain Assignment Tracking Location Unclear

**Requirement:** REQ-FIX-020 (Clear decoder chain)

**Problem:**
- Bug report assumes chain assignment map exists
- Unknown location and structure of this mapping
- Unknown how to look up chain for given `queue_entry_id`

**Impact:** Can't identify which chain to clear

**Resolution Required:**
- **Before Implementation:** Find in `engine.rs`:
  - Data structure mapping `queue_entry_id` → `chain_id`
  - Method to query this mapping
  - Method to clear mapping entry
- **If doesn't exist:** Need to add tracking (small additional work)
- **Estimated investigation time:** 20-30 minutes

**Severity Justification:** HIGH because critical for identifying what to clear

---

#### ISSUE-H4: "Start Next Passage" Logic Location Unknown

**Requirement:** REQ-FIX-050 (Start next passage if queue non-empty)

**Problem:**
- Bug report says to "use existing playback start logic"
- Unknown where this logic lives
- Unknown how to trigger "start playing current passage"

**Impact:** Can't implement automatic start of next passage

**Resolution Required:**
- **Before Implementation:** Find in `engine.rs`:
  - Logic for starting passage playback
  - How natural passage end triggers next passage
  - Method to call for "start current passage"
- **If mechanism unclear:** May need to refactor
- **Estimated investigation time:** 30 minutes

**Severity Justification:** HIGH because affects core fix behavior, but likely exists (natural end works)

---

### MEDIUM Issues (Should Resolve Before Implementation)

#### ISSUE-M1: Crossfade Abort Behavior Undefined

**Requirement:** REQ-FIX-010 (Stop playback immediately)

**Problem:**
- If removal happens during crossfade, behavior undefined
- Should we:
  - Abort crossfade and stop both passages?
  - Complete crossfade then stop?
  - Stop current, start next immediately (no crossfade)?

**Impact:** Edge case behavior unclear

**Resolution:**
- **Recommended:** Abort crossfade, stop current, start next (no crossfade)
- **Rationale:** User explicitly removed passage, immediate stop expected
- **Implementation:** Clear crossfade state, treat as normal passage end

**Severity Justification:** MEDIUM because edge case, but should define behavior explicitly

---

####ISSUE-M2: Ring Buffer Handling Unspecified

**Requirement:** REQ-FIX-010 (Stop playback immediately)

**Problem:**
- When mixer stops, what happens to ring buffer?
- Options:
  - Drain remaining samples (gradual stop)
  - Flush buffer immediately (hard stop, may cause click)
  - Leave buffer (may cause brief continuation)

**Impact:** User experience varies based on choice

**Resolution:**
- **Recommended:** Flush ring buffer immediately
- **Rationale:** "Immediate stop" requirement suggests hard stop acceptable
- **Implementation:** Clear ring buffer when stopping
- **Note:** May cause audio click, but matches user expectation

**Severity Justification:** MEDIUM because affects UX but straightforward to implement

---

#### ISSUE-M3: Remove During Pause State Undefined

**Requirement:** REQ-FIX-010 (Stop playback)

**Problem:**
- Bug report doesn't cover paused state
- If playback paused and user removes current passage:
  - Should we resume then stop?
  - Or just clear without resume?

**Impact:** Edge case behavior

**Resolution:**
- **Recommended:** Clear without resume (don't unpause)
- **Rationale:** User wants passage gone, don't play it
- **Implementation:** Check state, clear regardless of pause/play

**Severity Justification:** MEDIUM because edge case but simple to handle

---

### LOW Issues (Minor, Can Address During Implementation)

#### ISSUE-L1: SSE Event Timing Unspecified

**Requirement:** Related to REQ-FIX-040 (queue updates)

**Problem:**
- When should SSE events be emitted after removal?
- Before decoder clear or after?
- Multiple events or single consolidated event?

**Impact:** UI update timing

**Resolution:**
- **Recommended:** Emit after all cleanup complete
- **Rationale:** UI sees consistent state
- **Implementation:** Emit QueueChanged event at end of `remove_queue_entry()`

**Severity Justification:** LOW because existing code likely handles this correctly already

---

## Completeness Check Results

For each requirement, checked:
- ✓ Inputs specified (queue_entry_id parameter)
- ✓ Outputs specified (playback stopped, resources cleared)
- ⚠️ Behavior specified (HIGH issues - need investigation)
- ✓ Constraints specified (immediate stop, resource cleanup)
- ⚠️ Error cases specified (MEDIUM issues - edge cases)
- ⚠️ Dependencies specified (HIGH issues - mechanisms unclear)

---

## Ambiguity Check Results

**Vague Language Found:**
- "Immediate stop" (MEDIUM - resolved: flush ring buffer)
- "Clear mixer state" (HIGH - needs investigation to define what to clear)
- "Release resources" (HIGH - needs investigation to find release mechanism)

**Quantified Requirements:**
- None needed - this is boolean behavior (works or doesn't)

**Undefined Terms:**
- "Decoder chain" - RESOLVED (refers to decoder-buffer chain in architecture)
- "Mixer state" - HIGH ISSUE (need to investigate structure)

---

## Consistency Check Results

**No contradictions found** - Requirements are internally consistent.

**No resource conflicts** - Fix uses existing infrastructure.

**No timing conflicts** - Sequential operation, no parallelism issues.

**Interface consistency** - Existing `remove_queue_entry()` signature unchanged.

---

## Testability Check Results

All requirements are testable:
- ✓ REQ-FIX-010: Verify audio output stops (measure ring buffer)
- ✓ REQ-FIX-020: Verify decoder resources released (check file handles)
- ✓ REQ-FIX-030: Verify mixer not reading stale data (check state)
- ✓ REQ-FIX-040: Verify queue structure correct (already working)
- ✓ REQ-FIX-050: Verify next passage starts (observe playback)
- ✓ REQ-FIX-060: Verify non-current removal doesn't disrupt (observe continued playback)
- ✓ REQ-FIX-070: Verify removed passage doesn't resume (the bug scenario)
- ✓ REQ-FIX-080: Verify new passage starts (the bug scenario)

---

## Dependency Validation

All dependencies identified in Phase 1:
- ✓ Queue management - exists, works
- ✓ Database operations - exists, works
- ⚠️ Decoder command channel - HIGH ISSUE (investigate existence)
- ⚠️ Mixer state access - HIGH ISSUE (investigate mechanism)
- ⚠️ Chain assignment tracking - HIGH ISSUE (investigate existence)

---

## Resolution Strategy

### Approach for HIGH Issues:
1. **Investigation Phase (before detailed planning):**
   - Read `decoder_worker.rs` to find command mechanism
   - Read `mixer.rs` to find state structure
   - Read `engine.rs` to find chain tracking and start logic
   - Document findings

2. **If mechanisms exist:** Use them
3. **If mechanisms missing:** Design minimal additions
4. **Estimated total investigation:** 2-3 hours

### Approach for MEDIUM Issues:
- Define explicit behavior during implementation
- Document choices in code comments
- Add to test scenarios

### Approach for LOW Issues:
- Handle during implementation
- Follow existing patterns

---

## Decision: Proceed to Phase 3

**Rationale:**
- No CRITICAL blockers
- HIGH issues are investigation tasks, not specification gaps
- Specification provides sufficient direction
- Test scenarios well-defined

**Next Steps:**
1. Complete Phase 3 (Test Definitions)
2. Investigation phase before implementation
3. Implementation with clear test targets

---

## Auto-/think Trigger Assessment

**Criteria for automatic /think:**
- 5+ Critical issues: NO (0 critical)
- 10+ High issues: NO (4 high)
- Unclear architecture: NO (architecture clear)
- Novel/risky elements: NO (using existing infrastructure)

**Decision:** /think NOT triggered - proceed with planning
