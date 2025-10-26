# Implementation Guidance: How to Proceed with /plan Workflow

**Section:** Practical guidance for proceeding from analysis to implementation
**Parent Document:** [00_ANALYSIS_SUMMARY.md](00_ANALYSIS_SUMMARY.md)

---

## Purpose

This document provides step-by-step guidance for proceeding from specification consolidation analysis to implementation planning using the /plan workflow.

---

## Recommended Approach: Create GUIDE002 Implementation Guide

Per [02_approach_comparison.md](02_approach_comparison.md), **Approach 3 (Create GUIDE002 Implementation Guide)** is recommended due to lowest risk, correct tier placement, and best quality characteristics.

---

## Step-by-Step Workflow

### Phase 1: Pre-Planning Actions (Before /plan Invocation)

**Objective:** Resolve at-risk decisions and prepare specifications

**Timeline:** Week 0 (before implementation begins)

#### Action 1.1: Approve SPEC021 Error Handling Strategy

**Why:** SPEC021 currently has "Draft" status; implementation planning should use Approved specifications

**Who:** System Architecture Team

**Steps:**
1. Read SPEC021 in full (current version: Draft)
2. Review error taxonomy (FATAL/RECOVERABLE/DEGRADED/TRANSIENT)
3. Review error response strategy matrix (lines 76-84)
4. Review handling specifications for each error category
5. Verify event integration with SPEC011
6. If acceptable: Change status to "Approved" with version increment
7. If modifications needed: Revise, then approve

**Output:** SPEC021 status changed to "Approved" OR revision plan created

**Urgency:** HIGH (blocks implementation planning)

#### Action 1.2: Validate rubato Library Assumptions

**Why:** Resampler state management deferred to rubato; validate assumptions before implementation

**Who:** Implementation team (developer assigned to decoder pipeline)

**Steps:**
1. Review rubato documentation: https://docs.rs/rubato/latest/rubato/
2. Verify StatefulResampler provides required functionality:
   - State preservation across chunk boundaries
   - Flush behavior for tail samples
   - Pause/resume compatibility
3. Prototype simple resampler usage:
   ```rust
   use rubato::{Resampler, SincFixedIn, InterpolationType, InterpolationParameters, WindowFunction};

   // Initialize resampler
   let params = InterpolationParameters {
       sinc_len: 256,
       f_cutoff: 0.95,
       interpolation: InterpolationType::Linear,
       oversampling_factor: 256,
       window: WindowFunction::BlackmanHarris2,
   };
   let mut resampler = SincFixedIn::<f32>::new(
       48000.0 / 44100.0, // ratio
       params,
       1024,    // chunk_size
       2,       // channels
   )?;

   // Test flush behavior
   let input = vec![vec![1.0; 1024]; 2]; // 2 channels
   let output = resampler.process(&input)?;
   // Verify output sample count matches expected
   ```
4. Document findings
5. If rubato insufficient: Propose fallback (custom wrapper or alternative library)

**Output:** Validation report OR fallback plan

**Urgency:** MEDIUM (can be done during implementation kickoff week)

#### Action 1.3: Confirm Buffer Decode Strategy Interpretation

**Why:** SPEC016 [DBD-BUF-050] backpressure interpreted as "always incremental"; confirm interpretation

**Who:** System Architecture Team + Technical Lead

**Steps:**
1. Review SPEC016 [DBD-BUF-050] specification:
   ```
   [DBD-BUF-050] Decoder pauses when buffer nearly full
   ```
2. Confirm interpretation: "Always incremental decode (pause when full, resume when space)"
3. If confirmed: Proceed with implementation plan
4. If alternative intended: Document correct strategy
5. Optional: Add clarifying note to SPEC016 if ambiguity identified

**Output:** Interpretation confirmation OR strategy clarification

**Urgency:** LOW (interpretation is well-founded; confirmation can happen during planning)

---

### Phase 2: Create GUIDE002 Implementation Guide

**Objective:** Create EXEC-tier orchestration document for wkmp-ap implementation

**Timeline:** 4-6 hours (single session recommended)

**Who:** Technical Lead or assigned architect

#### Action 2.1: Create GUIDE002 Document Structure

**Location:** `docs/GUIDE002-wkmp_ap_re_implementation_guide.md`

**Metadata:**
```markdown
# GUIDE002: wkmp-ap Audio Player Re-Implementation Guide

**🗂️ TIER 4 - EXECUTION PLAN**

Orchestrates implementation of wkmp-ap Audio Player across multiple Tier 2 specifications.

> **Related Documentation:** [SPEC002](SPEC002-crossfade.md) | [SPEC016](SPEC016-decoder_buffer_design.md) | [SPEC017](SPEC017-sample_rate_conversion.md) | [SPEC018](SPEC018-crossfade_completion_coordination.md) | [SPEC021](SPEC021-error_handling.md) | [SPEC022](SPEC022-performance_targets.md)

---

## Metadata

**Document Type:** Tier 4 - Execution Plan (Implementation Guide)
**Version:** 1.0
**Date:** 2025-10-25
**Status:** Active
**Author:** Technical Lead

**Parent Documents (Tier 2):**
- [SPEC002-crossfade.md](SPEC002-crossfade.md) - Crossfade timing and curves
- [SPEC016-decoder_buffer_design.md](SPEC016-decoder_buffer_design.md) - Decoder-buffer pipeline (AUTHORITATIVE)
- [SPEC017-sample_rate_conversion.md](SPEC017-sample_rate_conversion.md) - Sample rate conversion
- [SPEC018-crossfade_completion_coordination.md](SPEC018-crossfade_completion_coordination.md) - Crossfade completion
- [SPEC021-error_handling.md](SPEC021-error_handling.md) - Error handling strategy
- [SPEC022-performance_targets.md](SPEC022-performance_targets.md) - Performance benchmarks
```

#### Action 2.2: Write Executive Summary

**Content:**
- Purpose: Re-implement wkmp-ap Audio Player per updated specifications
- Scope: Core audio pipeline, crossfading, queue management, error handling
- Out of scope: UI (wkmp-ui), Program Director (wkmp-pd), API design (covered elsewhere)
- Key principles: Sample-accurate timing, graceful degradation, Pi Zero 2W performance

**Template:**
```markdown
## Executive Summary

This guide orchestrates re-implementation of wkmp-ap Audio Player microservice based on comprehensive specification updates completed in 2025-10-25.

**Implementation Scope:**
- Decoder-buffer chain pipeline (per SPEC016)
- Sample-accurate crossfading (per SPEC002, SPEC018)
- Queue management and persistence (per SPEC016, SPEC007)
- Error handling and recovery (per SPEC021)
- Performance optimization for Pi Zero 2W (per SPEC022)

**Key Changes from Original Implementation:**
- Serial decode execution (was: parallel 2-thread pool)
- Explicit crossfade completion signaling (was: polling-based)
- Comprehensive error taxonomy (was: ad-hoc handling)
- Quantified performance targets (was: undefined)

**Success Criteria:**
- All acceptance tests pass (derived from SPEC### requirements)
- Performance targets met (per SPEC022)
- Zero audio artifacts during crossfades
- Graceful degradation under error conditions
```

#### Action 2.3: Document Specification Inventory

**Content:**
- List all relevant SPEC### documents
- Brief purpose for each
- Priority (core vs. supporting)

**Template:**
```markdown
## Specification Inventory

### Core Specifications (Implementation Required)

**SPEC016 - Decoder Buffer Design (AUTHORITATIVE)**
- **Purpose:** Defines decoder-buffer chain architecture, serial decode, buffer management
- **Requirements:** [DBD-###] series (150+ requirements)
- **Priority:** CRITICAL (foundation for all audio processing)
- **Dependencies:** SPEC017 (sample rate), SPEC002 (fade curves)

**SPEC002 - Crossfade Design**
- **Purpose:** Defines crossfade timing points, fade curves, state machine
- **Requirements:** [XFD-###] series (50+ requirements)
- **Priority:** CRITICAL (core feature)
- **Dependencies:** SPEC018 (completion coordination)

**SPEC018 - Crossfade Completion Coordination**
- **Purpose:** Defines mixer-to-engine signaling for crossfade completion
- **Requirements:** [XFD-COMP-###] series (3 requirements)
- **Priority:** HIGH (fixes queue advancement bug)
- **Dependencies:** SPEC002 (crossfade state machine), SPEC016 (mixer)

**SPEC021 - Error Handling Strategy**
- **Purpose:** Defines error taxonomy and handling for all failure modes
- **Requirements:** [ERH-###] series (40+ requirements)
- **Priority:** HIGH (robustness)
- **Dependencies:** SPEC011 (event system), SPEC016 (pipeline errors)

**SPEC022 - Performance Targets**
- **Purpose:** Defines quantified performance benchmarks for Pi Zero 2W
- **Requirements:** Quantified metrics with acceptance criteria
- **Priority:** HIGH (validation criteria)
- **Dependencies:** None (informational)

### Supporting Specifications (Integration Required)

**SPEC017 - Sample Rate Conversion**
- **Purpose:** Defines tick-based timing and resampling behavior
- **Requirements:** [SRC-###] series
- **Priority:** MEDIUM (integrated into SPEC016 pipeline)
- **Dependencies:** SPEC016 (decoder chain)

**SPEC013 - Single Stream Playback**
- **Purpose:** High-level architecture overview
- **Requirements:** References SPEC016 for details
- **Priority:** LOW (informational, not implementation-driving)
```

#### Action 2.4: Define Implementation Phases

**Content:**
- Logical increment order
- Rationale for ordering (dependencies, risk reduction)
- Estimated effort per phase
- Acceptance criteria per phase

**Template:**
```markdown
## Implementation Phases

### Phase 1: Core Pipeline Foundation (Week 1-2)

**Scope:**
- Decoder-buffer chain infrastructure (per SPEC016)
- Serial decode execution ([DBD-DEC-040])
- Sample rate conversion integration ([DBD-RSMP-###])
- Basic buffer management ([DBD-BUF-###])

**Rationale:**
- Foundation for all subsequent features
- Enables early integration testing
- Validates architecture feasibility

**Key Requirements:**
- [DBD-DEC-040]: Serial decode execution
- [DBD-BUF-010]: Ring buffer per passage
- [DBD-RSMP-010]: Rubato integration
- [DBD-LIFECYCLE-010]: Chain assignment on enqueue

**Acceptance Criteria:**
- Single passage decodes and buffers successfully
- Sample rate conversion works (44.1→48, 48→44.1, etc.)
- Buffer fills within SPEC022 latency target (≤2.0s for 15s buffer)
- No memory leaks during decode/buffer lifecycle

**Estimated Effort:** 40-50 hours

---

### Phase 2: Crossfade State Machine (Week 3)

**Scope:**
- Mixer state machine implementation (per SPEC002)
- Fade curve application (Fader component per SPEC016)
- Crossfade overlap calculation (per SPEC002 [XFD-IMPL-020])

**Rationale:**
- Core differentiating feature
- Dependencies on Phase 1 (buffer infrastructure)

**Key Requirements:**
- [XFD-SM-010] through [XFD-SM-030]: State machine (None/Single/Crossfading)
- [DBD-FADE-030/050]: Pre-buffer fade application
- [XFD-IMPL-020]: Crossfade duration calculation
- [DBD-MIX-040]: Sample mixing during overlap

**Acceptance Criteria:**
- Two passages crossfade smoothly
- No audible artifacts (clicks, pops, discontinuities)
- Fade curves applied correctly (verified via sample inspection)
- Crossfade duration matches min(lead-out, lead-in)

**Estimated Effort:** 30-40 hours

---

### Phase 3: Crossfade Completion Coordination (Week 4)

**Scope:**
- Crossfade completion signaling (per SPEC018)
- Queue advancement logic (engine-level)
- State consistency during transitions

**Rationale:**
- Fixes BUG-003 (queue advancement during crossfade)
- Requires Phase 2 (crossfade state machine)

**Key Requirements:**
- [XFD-COMP-010]: Crossfade completion detection
- [XFD-COMP-020]: Queue advancement without mixer restart
- [XFD-COMP-030]: State consistency

**Acceptance Criteria:**
- Crossfade completion triggers queue advancement
- Incoming passage continues seamlessly (no restart)
- PassageCompleted event emitted for outgoing passage only
- Mixer state matches queue state after advancement

**Estimated Effort:** 20-30 hours

---

### Phase 4: Error Handling (Week 5)

**Scope:**
- Error taxonomy implementation (per SPEC021)
- Error detection and recovery
- Event emission and logging

**Rationale:**
- Robustness requirement
- Can be integrated incrementally after core pipeline stable

**Key Requirements:**
- [ERH-TAX-010/020]: Error classification
- [ERH-DEC-###]: Decode error handling
- [ERH-BUF-###]: Buffer underrun handling
- [ERH-DEV-###]: Device error handling

**Acceptance Criteria:**
- Decode failures skip passage and continue with next
- Buffer underruns auto-pause and auto-resume
- Device failures attempt reconnection with timeout
- All errors emit events (per SPEC021) and log appropriately

**Estimated Effort:** 30-40 hours

---

### Phase 5: Performance Optimization (Week 6)

**Scope:**
- Performance profiling and optimization
- Validation against SPEC022 targets
- Pi Zero 2W deployment testing

**Rationale:**
- Ensures deployment feasibility
- Validates specification assumptions

**Key Requirements:**
- All SPEC022 performance targets met
- CPU usage ≤ 50% average (Pi Zero 2W)
- Decode latency ≤ 2.0s for 15s buffer
- Memory usage ≤ 150MB total

**Acceptance Criteria:**
- 90% of passages meet target latency
- CPU usage within limits during continuous playback
- No audio glitches on Pi Zero 2W under load
- Performance test suite passes

**Estimated Effort:** 20-30 hours
```

#### Action 2.5: Document At-Risk Decisions

**Content:**
- Reference [03_at_risk_decisions.md](03_at_risk_decisions.md)
- Summarize key risks
- Document mitigation in implementation plan

**Template:**
```markdown
## At-Risk Decisions

This implementation proceeds with the following at-risk decisions documented in analysis:

### 1. SPEC021 Draft Status
- **Risk:** Error handling specification may change during approval
- **Mitigation:** Early approval (Week 0), incremental error handling implementation (Phase 4)
- **Impact if changed:** 2-4 days refactoring (contained to error handling module)

### 2. Resampler State Management
- **Risk:** rubato library may not provide expected functionality
- **Mitigation:** Early validation (Week 1), fallback wrapper ready
- **Impact if changed:** 1-2 days for custom wrapper

### 3. Buffer Decode Strategy
- **Risk:** "Always incremental" interpretation may not match intent
- **Mitigation:** Confirm interpretation during planning, flexible implementation
- **Impact if changed:** 1-2 days for strategy selection logic

**Overall Risk:** LOW (all mitigations in place, impacts contained)

See [03_at_risk_decisions.md](wip/wkmp_ap_specification_consolidation_analysis/03_at_risk_decisions.md) for complete documentation.
```

#### Action 2.6: Define Cross-Cutting Concerns

**Content:**
- Error handling integration across all phases
- Event emission (SPEC011) usage
- Logging strategy (IMPL002)
- Testing approach

**Template:**
```markdown
## Cross-Cutting Concerns

### Error Handling Integration

**Strategy:** Implement error handling incrementally, integrating with each phase

**Phase 1 (Core Pipeline):**
- Decode errors: [ERH-DEC-###]
- Buffer errors: [ERH-BUF-###]
- Resampling errors: [ERH-RSMP-###]

**Phase 2 (Crossfade):**
- Crossfade transition errors (rare, log-only)

**Phase 3 (Completion):**
- Queue advancement errors (state consistency)

**Phase 4 (Error Handling):**
- Complete error taxonomy
- Device errors: [ERH-DEV-###]
- Queue errors: [ERH-QUEUE-###]

### Event Emission (SPEC011)

**Events to Implement:**
- PassageStarted
- PassageCompleted
- CrossfadeStarted
- CrossfadeCompleted
- BufferStateChanged
- ErrorOccurred (per SPEC021 error types)
- QueueChanged

**Integration Points:**
- Decoder: Emit PassageStarted when decode begins
- Mixer: Emit CrossfadeStarted/Completed on state transitions
- Queue Manager: Emit QueueChanged on advancement
- Error Handler: Emit ErrorOccurred per SPEC021 specifications

### Logging Strategy (IMPL002)

**Log Levels:**
- ERROR: Fatal errors, decode failures (permanent)
- WARN: Buffer underruns, transient errors, queue validation
- INFO: Passage start/complete, crossfade transitions
- DEBUG: Detailed pipeline state (decode progress, buffer fill)
- TRACE: Sample-level details (not used in production)

**Structured Logging:**
- Use `tracing` crate with structured fields
- Include passage_id, queue_entry_id, buffer_chain_index
- Include timing information (ticks, samples, milliseconds)

### Testing Approach

**Test Categories:**

**1. Unit Tests (Per-Component):**
- Decoder: Decode various formats (MP3, FLAC, AAC, Opus)
- Resampler: Various sample rate conversions
- Fader: Each fade curve type
- Mixer: State machine transitions
- Buffer: Ring buffer operations

**2. Integration Tests (Cross-Component):**
- Decoder→Resampler→Fader→Buffer pipeline
- Mixer reading from multiple buffers
- Queue advancement triggering buffer cleanup

**3. Acceptance Tests (Requirement-Driven):**
- One test per SHALL/MUST requirement
- Given/When/Then format
- Traceable to requirement IDs ([DBD-###], [XFD-###], etc.)

**4. Performance Tests:**
- Validate SPEC022 targets
- Measure on actual Pi Zero 2W hardware
- Continuous playback stress testing (10+ crossfades)
```

#### Action 2.7: Define Success Criteria

**Content:**
- Definition of "done" for wkmp-ap re-implementation
- Comprehensive acceptance criteria
- Validation methodology

**Template:**
```markdown
## Success Criteria

### Functional Requirements

**All SHALL/MUST requirements implemented:**
- ✅ All [DBD-###] requirements (decoder-buffer pipeline)
- ✅ All [XFD-###] requirements (crossfade)
- ✅ All [XFD-COMP-###] requirements (crossfade completion)
- ✅ All [ERH-###] requirements (error handling)

**Verification Method:** Acceptance test suite (one test per requirement)

### Performance Requirements

**SPEC022 targets met:**
- ✅ Decode latency ≤ 2.0s for 15s buffer (90% of passages)
- ✅ CPU usage ≤ 50% average on Pi Zero 2W
- ✅ Memory usage ≤ 150MB total application
- ✅ Initial playback start ≤ 1.0s

**Verification Method:** Performance test suite on actual Pi Zero 2W hardware

### Quality Requirements

**Audio Quality:**
- ✅ No audible artifacts during crossfades
- ✅ No clicks, pops, or discontinuities
- ✅ Crossfade duration matches specification
- ✅ Fade curves applied correctly

**Verification Method:** Manual listening tests + sample-level inspection

**Robustness:**
- ✅ Graceful degradation under all error conditions (per SPEC021)
- ✅ No crashes during error scenarios
- ✅ Appropriate logging for all errors
- ✅ Events emitted for all state changes

**Verification Method:** Error injection testing + chaos engineering

### Test Coverage

**Unit Test Coverage: ≥90%**
- Line coverage
- Branch coverage

**Acceptance Test Coverage: 100%**
- All SHALL/MUST requirements have acceptance tests
- All tests pass

**Verification Method:** `cargo tarpaulin` for coverage, custom script for requirement traceability

### Definition of Done

Implementation is complete when:
1. ✅ All acceptance tests pass (100% requirement coverage)
2. ✅ All performance tests pass (SPEC022 targets met)
3. ✅ Manual audio quality validation complete (no artifacts)
4. ✅ Unit test coverage ≥ 90%
5. ✅ All error handling scenarios tested
6. ✅ Pi Zero 2W deployment validated
7. ✅ Code review complete
8. ✅ Documentation updated (IMPL### if needed)
```

---

### Phase 3: /plan Workflow Invocation

**Objective:** Generate detailed implementation plans with test specifications

**Timeline:** Week 0 (after GUIDE002 created)

#### Option A: Single /plan Invocation for GUIDE002

**Approach:** Invoke /plan with GUIDE002 as input

**Command:**
```bash
/plan docs/GUIDE002-wkmp_ap_re_implementation_guide.md
```

**Expected Output:**
- Detailed implementation plan with test specifications
- Traceability matrix (requirements → tests)
- Increment breakdown (tasks per phase)
- Acceptance test definitions (Given/When/Then)

**Advantages:**
- Single invocation (simple workflow)
- Tests cross-reference GUIDE002 phases
- Unified traceability

**Disadvantages:**
- May miss SPEC###-specific details
- Requires GUIDE002 to be comprehensive

#### Option B: Multiple /plan Invocations (Per SPEC)

**Approach:** Invoke /plan for each core SPEC, integrate per GUIDE002

**Commands:**
```bash
/plan docs/SPEC016-decoder_buffer_design.md
/plan docs/SPEC002-crossfade.md
/plan docs/SPEC018-crossfade_completion_coordination.md
/plan docs/SPEC021-error_handling.md
```

**Integration:** Use GUIDE002 Phase structure to organize results

**Advantages:**
- Detailed test coverage per specification
- SPEC###-specific traceability

**Disadvantages:**
- Manual integration required
- Potential duplicate tests

#### Recommended: Hybrid Approach

**Workflow:**
1. Invoke `/plan GUIDE002` for overall orchestration
2. For complex specs (SPEC016, SPEC021): Invoke `/plan SPEC###` for detailed tests
3. Integrate SPEC###-specific tests into GUIDE002 phase structure
4. Deduplicate tests across specs

**Rationale:**
- Best of both worlds (orchestration + detail)
- Manageable integration effort
- Comprehensive test coverage

---

### Phase 4: Implementation Execution

**Objective:** Implement per /plan output, following GUIDE002 phases

**Timeline:** Weeks 1-6 (per phase estimates in GUIDE002)

#### Execution Workflow

**For Each Phase:**
1. **Review acceptance criteria** (from GUIDE002 + /plan output)
2. **Write acceptance tests first** (TDD approach)
   - Tests should FAIL initially (red)
3. **Implement increment** per /plan task breakdown
   - Follow IMPL002 coding conventions
   - Emit events per SPEC011
   - Handle errors per SPEC021
4. **Run tests continuously** during implementation
   - Tests should PASS when increment complete (green)
5. **Refactor as needed** (maintain test passage)
6. **Code review** before proceeding to next phase
7. **Update GUIDE002** if implementation reveals issues

#### Validation Per Phase

**Phase 1 (Core Pipeline):**
- Run unit tests for decoder, resampler, fader, buffer
- Run integration tests for complete decode-to-buffer flow
- Measure decode latency (compare to SPEC022 target)

**Phase 2 (Crossfade):**
- Run crossfade acceptance tests
- Manual listening test (no artifacts)
- Sample-level inspection (fade curves correct)

**Phase 3 (Completion Coordination):**
- Run queue advancement tests
- Verify state consistency (mixer + queue)
- Test multiple crossfades in sequence

**Phase 4 (Error Handling):**
- Error injection tests (all ERH-### scenarios)
- Verify graceful degradation
- Verify event emission and logging

**Phase 5 (Performance Optimization):**
- Run performance test suite on Pi Zero 2W
- Profiling (identify bottlenecks)
- Optimization (if targets not met)

---

## Alternative Approach: Use Specifications Directly (Approach 1)

If GUIDE002 creation is deferred (timeline constraints), follow simplified workflow:

### Simplified Workflow

1. **Pre-Planning:** Resolve at-risk decisions (same as Phase 1 above)
2. **Specification Ordering:** Prioritize specs for implementation:
   - SPEC016 (foundation)
   - SPEC002 (crossfade)
   - SPEC018 (completion)
   - SPEC021 (error handling)
3. **Per-Specification Planning:**
   ```bash
   /plan docs/SPEC016-decoder_buffer_design.md
   /plan docs/SPEC002-crossfade.md
   /plan docs/SPEC018-crossfade_completion_coordination.md
   /plan docs/SPEC021-error_handling.md
   ```
4. **Manual Integration:**
   - Combine /plan outputs
   - Resolve dependencies
   - Deduplicate tests
   - Create unified implementation order
5. **Implementation:** Per integrated plan

**Trade-offs:**
- Lower upfront effort (no GUIDE002 creation)
- Higher integration effort (manual)
- No single orchestration document
- Higher risk of integration gaps

**Use when:** Timeline constraints exist AND integration overhead acceptable

---

## Summary

**Recommended Workflow:**
1. ✅ Resolve at-risk decisions (approve SPEC021, validate rubato, confirm buffer strategy)
2. ✅ Create GUIDE002 implementation guide (4-6 hours)
3. ✅ Invoke /plan for GUIDE002 + detailed specs (hybrid approach)
4. ✅ Implement per phase (Weeks 1-6)
5. ✅ Validate against success criteria

**Alternative (if timeline constrained):**
1. ✅ Resolve at-risk decisions
2. ⚠️ Skip GUIDE002 creation
3. ✅ Invoke /plan for each SPEC directly
4. ⚠️ Manually integrate /plan outputs
5. ✅ Implement per integrated plan

**Selection Criteria:**
- Use Recommended if: 4-6 hours available for GUIDE002 creation
- Use Alternative if: Immediate implementation required, integration effort acceptable

---

**Section Complete**

**Return to Summary:** [00_ANALYSIS_SUMMARY.md](00_ANALYSIS_SUMMARY.md)

**See Also:**
- [01_gap_resolution_status.md](01_gap_resolution_status.md) - Gap verification
- [02_approach_comparison.md](02_approach_comparison.md) - Approach comparison
- [03_at_risk_decisions.md](03_at_risk_decisions.md) - At-risk decisions
