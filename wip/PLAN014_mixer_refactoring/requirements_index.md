# Requirements Index: PLAN014 Mixer Refactoring

**Source Document:** wip/mixer_architecture_review.md
**Date:** 2025-01-30
**Plan:** PLAN014_mixer_refactoring

---

## Requirements Extraction

Requirements extracted from "Recommendations" section (lines 390-424) of mixer architecture review.

---

## Requirements Table

| Req ID | Type | Brief Description | Source Line | Priority |
|--------|------|-------------------|-------------|----------|
| **Immediate Actions** |
| REQ-MIX-001 | Investigation | Determine which mixer is currently active in PlaybackEngine | 395 | P0-Critical |
| REQ-MIX-002 | Technical Debt | Remove inactive mixer implementation (archive or delete) | 397, 405 | P0-Critical |
| REQ-MIX-003 | Documentation | Add [XFD-CURV-050] to SPEC002 clarifying fade application timing | 400 | P1-High |
| REQ-MIX-004 | Documentation | Add [XFD-IMPL-025] to SPEC002 architectural note | 401 | P1-High |
| REQ-MIX-005 | Documentation | Clarify [DBD-MIX-040] resume-fade terminology | 402 | P1-High |
| REQ-MIX-006 | Documentation | Create ADR documenting mixer refactoring decision | 406 | P1-High |
| **Long-Term Improvements** |
| REQ-MIX-007 | Feature | Port buffer underrun detection to correct mixer (if not active) | 416 | P1-High |
| REQ-MIX-008 | Feature | Port position event emission to correct mixer (if not active) | 417 | P1-High |
| REQ-MIX-009 | Feature | Port resume fade-in support to correct mixer (if not active) | 418 | P1-High |
| REQ-MIX-010 | Testing | Unit tests verifying mixer reads pre-faded samples | 421 | P1-High |
| REQ-MIX-011 | Testing | Integration tests with Fader component | 422 | P1-High |
| REQ-MIX-012 | Testing | Crossfade overlap tests (simple addition verification) | 423 | P1-High |

---

## Requirement Details

### REQ-MIX-001: Determine Active Mixer

**Priority:** P0-Critical
**Type:** Investigation
**Source:** Lines 395-397

**Description:**
Investigate which mixer implementation is currently used by PlaybackEngine:
- Legacy mixer (`wkmp-ap/src/playback/pipeline/mixer.rs`) - 1,969 lines, applies fade curves at mix time
- Correct mixer (`wkmp-ap/src/playback/mixer.rs`) - 359 lines, reads pre-faded samples

**Acceptance Criteria:**
- Clear determination of which mixer is instantiated and called by PlaybackEngine
- Evidence provided (code references showing mixer instantiation and usage)
- Migration plan defined if legacy mixer is active

---

### REQ-MIX-002: Remove Inactive Mixer

**Priority:** P0-Critical
**Type:** Technical Debt
**Source:** Lines 397, 405

**Description:**
Remove or archive the inactive mixer implementation to eliminate confusion and maintain single source of truth.

**Two Scenarios:**

**Scenario A: Legacy mixer is active**
- Keep legacy mixer temporarily
- Migrate to correct mixer (see REQ-MIX-007/008/009)
- Remove legacy mixer after migration verified
- Document as technical debt until migration complete

**Scenario B: Correct mixer is active**
- Archive legacy mixer immediately (move to archive branch or add deprecation notice)
- Remove from active codebase
- Update imports/references if any exist

**Acceptance Criteria:**
- Only one mixer implementation exists in active codebase
- No ambiguity about which mixer to use
- Archived mixer (if preserved) clearly marked as deprecated

---

### REQ-MIX-003: Add [XFD-CURV-050] to SPEC002

**Priority:** P1-High
**Type:** Documentation
**Source:** Lines 262-269

**Description:**
Add explicit requirement to SPEC002-crossfade.md clarifying when/where fade curves are applied.

**Content to Add (after [XFD-CURV-040]):**
```
[XFD-CURV-050] Application Timing:
Fade curves are applied to audio samples by the Fader component BEFORE buffering.
The mixer reads pre-faded samples and performs simple addition during crossfade overlap.
See [SPEC016 DBD-MIX-042] for architectural separation details.
```

**Acceptance Criteria:**
- [XFD-CURV-050] added to SPEC002 in correct location (after [XFD-CURV-040])
- Cross-reference to SPEC016 [DBD-MIX-042] included
- Change logged in SPEC002 revision history

---

### REQ-MIX-004: Add [XFD-IMPL-025] to SPEC002

**Priority:** P1-High
**Type:** Documentation
**Source:** Lines 282-289

**Description:**
Add architectural clarification to SPEC002 Implementation Algorithm section.

**Content to Add (after [XFD-IMPL-020]):**
```
[XFD-IMPL-025] Architectural Note:
This algorithm calculates crossfade TIMING (when passages overlap). Fade curve
APPLICATION is handled separately by the Fader component per [DBD-MIX-042].
The mixer implements simple addition of pre-faded samples per [DBD-MIX-041].
```

**Acceptance Criteria:**
- [XFD-IMPL-025] added to SPEC002 in correct location (after [XFD-IMPL-020])
- Clear distinction between timing (mixer responsibility) vs fade application (fader responsibility)
- Cross-references to SPEC016 [DBD-MIX-041] and [DBD-MIX-042] included
- Change logged in SPEC002 revision history

---

### REQ-MIX-005: Clarify [DBD-MIX-040] Resume-Fade Terminology

**Priority:** P1-High
**Type:** Documentation
**Source:** Lines 313-320

**Description:**
Update SPEC016 [DBD-MIX-040] to clarify that resume-from-pause fade is a MIXER-LEVEL operation (orthogonal to passage-level fades).

**Current Text (Line 608):**
```
- When "fading in after pause" also multiplies the sample values by the
  current fade in curve value
```

**Improved Text:**
```
- When "fading in after pause" also multiplies the mixed output by the
  resume fade-in curve (mixer-level fade, orthogonal to passage-level fades
  applied by Fader component)
```

**Acceptance Criteria:**
- [DBD-MIX-040] text updated in SPEC016
- Distinction between mixer-level and passage-level fades clarified
- No ambiguity remains about which component applies which fades
- Change logged in SPEC016 revision history

---

### REQ-MIX-006: Create ADR for Mixer Refactoring

**Priority:** P1-High
**Type:** Documentation
**Source:** Line 406

**Description:**
Create Architectural Decision Record documenting the mixer refactoring decision and rationale.

**ADR Content (Nygard Template):**
- **Title:** ADR-XXX: Adopt Correct Mixer (mixer.rs) as Authoritative Implementation
- **Status:** Accepted
- **Date:** 2025-01-30
- **Context:**
  - Two mixer implementations exist (legacy 1969 lines, correct 359 lines)
  - Legacy mixer violates SPEC016 [DBD-MIX-042] architectural separation
  - Correct mixer follows architectural separation principle
  - Root cause: Architectural separation added to spec after legacy implementation
- **Decision:** [Depends on REQ-MIX-001 findings]
  - If legacy active: Migrate to correct mixer, port missing features
  - If correct active: Archive legacy mixer immediately
- **Consequences:**
  - Single mixer implementation (reduced confusion)
  - Architectural compliance with SPEC016
  - May require porting features (underrun detection, position events, resume fade)
  - Testing required to verify equivalent functionality

**Acceptance Criteria:**
- ADR document created following Nygard template
- Decision rationale clearly documented
- Consequences (both positive and negative) listed
- Traceability to mixer architecture review analysis
- Stored in appropriate location (docs/ or wip/)

---

### REQ-MIX-007: Port Buffer Underrun Detection

**Priority:** P1-High
**Type:** Feature
**Source:** Line 416

**Description:**
If correct mixer is determined to be active (REQ-MIX-001), port buffer underrun detection from legacy mixer while maintaining architectural separation.

**Source Reference:** Legacy mixer lines 422-611

**Implementation Constraints:**
- MUST maintain architectural separation (no fade curve knowledge in mixer)
- MUST NOT violate [DBD-MIX-042]
- Underrun detection based on buffer status, not fade curve state

**Acceptance Criteria:**
- Underrun detection functional in correct mixer
- Architectural separation maintained (mixer queries buffer status, doesn't know about fades)
- Flatline output behavior implemented per [SSD-UND-017]
- Auto-resume logic implemented per [SSD-UND-018]
- Tests verify underrun detection and recovery

**Dependency:** REQ-MIX-001 (only needed if correct mixer is active)

---

### REQ-MIX-008: Port Position Event Emission

**Priority:** P1-High
**Type:** Feature
**Source:** Line 417

**Description:**
If correct mixer is determined to be active (REQ-MIX-001), port position event emission from legacy mixer.

**Source Reference:** Legacy mixer lines 614-642

**Implementation:**
- Periodic PositionUpdate event emission via tokio::sync::mpsc
- Configurable interval (default 1 second, 44100 frames @ 44.1kHz)
- Frame counter incremented on each get_next_frame() call
- Event contains queue_entry_id and position_ms

**Acceptance Criteria:**
- Position events emitted periodically
- Interval configurable from database setting
- Events contain accurate position information
- No blocking on event channel (use try_send or similar)
- Tests verify event emission frequency and accuracy

**Dependency:** REQ-MIX-001 (only needed if correct mixer is active)

---

### REQ-MIX-009: Port Resume Fade-In Support

**Priority:** P1-High
**Type:** Feature
**Source:** Line 418

**Description:**
If correct mixer is determined to be active (REQ-MIX-001), port resume-from-pause fade-in from legacy mixer.

**Source Reference:** Legacy mixer lines 644-671

**Implementation:**
- ResumeState tracking (fade duration, curve, frames since resume)
- Fade applied MULTIPLICATIVELY after all other processing (mixer-level)
- Fade-in from 0.0 to 1.0 over configured duration
- Supports linear and exponential curves

**Acceptance Criteria:**
- Resume fade-in functional (pause → resume with fade-in)
- Fade applied multiplicatively to mixed output (orthogonal to passage fades)
- Configurable duration and curve type
- Tests verify fade-in behavior (starts at 0.0, reaches 1.0, correct curve shape)

**Dependency:** REQ-MIX-001 (only needed if correct mixer is active)

---

### REQ-MIX-010: Unit Tests for Pre-Faded Sample Reading

**Priority:** P1-High
**Type:** Testing
**Source:** Line 421

**Description:**
Create unit tests verifying mixer correctly reads pre-faded samples from buffers and does NOT apply fade curves at mix time.

**Test Cases:**
1. **Test: Mixer API Does Not Accept Fade Curves**
   - Verify mixer APIs accept only passage IDs, not fade curve parameters
   - Compile-time verification (type system enforces this)

2. **Test: Mixer Reads Pre-Faded Samples**
   - Populate buffer with pre-faded samples (e.g., linear ramp 0.0 → 1.0)
   - Mixer reads samples and applies master volume only
   - Output matches: `buffer_sample * master_volume`
   - No fade curve calculations performed

3. **Test: Crossfade Is Simple Addition**
   - Populate two buffers with different constant values
   - Mixer reads from both during crossfade
   - Output = `(buffer1_sample + buffer2_sample) * master_volume`
   - No fade curve calculations performed

**Acceptance Criteria:**
- All unit tests pass
- Tests verify mixer does NOT perform fade curve calculations
- Tests verify simple addition during crossfade
- Code coverage includes mixer's mixing logic

---

### REQ-MIX-011: Integration Tests with Fader Component

**Priority:** P1-High
**Type:** Testing
**Source:** Line 422

**Description:**
Create integration tests verifying end-to-end pipeline: Fader applies curves → Buffer stores pre-faded samples → Mixer reads and sums.

**Test Cases:**
1. **Test: Fader-Buffer-Mixer Pipeline**
   - Fader applies fade-in curve to raw samples
   - Faded samples written to buffer
   - Mixer reads from buffer
   - Output matches expected faded values (no additional fading by mixer)

2. **Test: Crossfade Overlap with Pre-Fading**
   - Two passages: A fading out, B fading in
   - Fader applies fade-out curve to A samples
   - Fader applies fade-in curve to B samples
   - Buffer stores both pre-faded sample sets
   - Mixer performs simple addition
   - Output = `(A_faded + B_faded) * master_volume`

**Acceptance Criteria:**
- Integration tests pass
- Tests verify Fader applies curves BEFORE buffering
- Tests verify Mixer reads pre-faded samples and performs simple addition
- No double-fading detected (samples not faded twice)

---

### REQ-MIX-012: Crossfade Overlap Tests (Simple Addition Verification)

**Priority:** P1-High
**Type:** Testing
**Source:** Line 423

**Description:**
Create tests specifically verifying crossfade overlap implements simple addition (not complex mixing with runtime fade calculations).

**Test Cases:**
1. **Test: Simple Addition Math**
   - Buffer A: All samples = 0.5
   - Buffer B: All samples = 0.3
   - Master volume = 1.0
   - Expected output during overlap: 0.8 (0.5 + 0.3)
   - Verify no fade curve calculations

2. **Test: Volume Normalization Not Applied**
   - Buffer A: All samples = 0.7
   - Buffer B: All samples = 0.7
   - Master volume = 1.0
   - Expected output during overlap: 1.4 (sum exceeds 1.0 before master volume)
   - Mixer applies master volume, then clamps: min(1.4 * 1.0, 1.0) = 1.0
   - Verify clipping behavior is as specified in [SSD-CLIP-010]

3. **Test: Asymmetric Fade Durations**
   - Passage A: Fade-out duration 1000 samples
   - Passage B: Fade-in duration 500 samples
   - Both buffers contain pre-faded samples (faded by Fader)
   - Mixer performs simple addition for overlap duration
   - No runtime fade calculations based on asymmetric durations

**Acceptance Criteria:**
- All crossfade overlap tests pass
- Tests verify simple addition (A + B)
- Tests verify master volume applied after summing
- Tests verify no runtime fade curve calculations during crossfade

---

## Requirements Summary

**Total Requirements:** 12
- **P0-Critical:** 2 (REQ-MIX-001, REQ-MIX-002)
- **P1-High:** 10 (REQ-MIX-003 through REQ-MIX-012)

**By Type:**
- Investigation: 1 (REQ-MIX-001)
- Technical Debt: 1 (REQ-MIX-002)
- Documentation: 4 (REQ-MIX-003, REQ-MIX-004, REQ-MIX-005, REQ-MIX-006)
- Feature: 3 (REQ-MIX-007, REQ-MIX-008, REQ-MIX-009)
- Testing: 3 (REQ-MIX-010, REQ-MIX-011, REQ-MIX-012)

**Dependencies:**
- REQ-MIX-002 depends on REQ-MIX-001 (must know which mixer is active before removing other)
- REQ-MIX-007, REQ-MIX-008, REQ-MIX-009 depend on REQ-MIX-001 (only port features if correct mixer is active and missing features)
- REQ-MIX-011 depends on Fader component being functional
- REQ-MIX-012 depends on buffer management being functional

---

## Out of Scope

**Explicitly NOT included in this plan:**
- Implementing new mixer features beyond porting from legacy
- Modifying Fader component
- Modifying buffer management system
- Performance optimization
- Changing crossfade timing algorithm
- Modifying fade curve calculation logic

**Rationale:** This plan focuses on architectural cleanup (removing duplicate mixer, ensuring compliance with SPEC016), documentation improvements, and testing. Feature development is out of scope.

---

## Assumptions

1. **Assumption:** SPEC016 [DBD-MIX-042] architectural separation principle is correct and authoritative
   - Fader applies curves before buffering
   - Mixer reads pre-faded samples
   - Mixer performs simple addition during crossfade

2. **Assumption:** One of the two mixers is currently active in PlaybackEngine (investigation will confirm)

3. **Assumption:** Fader component ([wkmp-ap/src/playback/pipeline/fader.rs](../wkmp-ap/src/playback/pipeline/fader.rs)) is functional and applies curves correctly

4. **Assumption:** Buffer management system provides pre-faded samples to mixer

5. **Assumption:** Test infrastructure (tokio::test, buffer mocks, etc.) is available

---

## Constraints

**Technical:**
- Must maintain architectural separation per [DBD-MIX-042]
- Must not break existing playback functionality
- Must support all existing fade curves (exponential, cosine, linear)
- Must maintain sample-accurate timing

**Process:**
- Documentation changes require review
- Code removal requires verification of no active references
- Feature porting requires equivalent test coverage
- ADR requires architectural rationale

**Timeline:**
- P0-Critical requirements should be completed first (investigation, cleanup)
- Documentation improvements can proceed in parallel with code changes
- Testing should be completed before archiving plan

---

**Requirements Index Complete**
**Date:** 2025-01-30
