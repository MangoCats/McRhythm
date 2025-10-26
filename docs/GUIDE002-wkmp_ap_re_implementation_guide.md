# GUIDE002: wkmp-ap Audio Player Re-Implementation Guide

**🗂️ TIER 4 - EXECUTION PLAN**

Orchestrates re-implementation of wkmp-ap Audio Player across multiple Tier 2 specifications.

> **Related Documentation:** [SPEC002](SPEC002-crossfade.md) | [SPEC013](SPEC013-single_stream_playback.md) | [SPEC016](SPEC016-decoder_buffer_design.md) | [SPEC017](SPEC017-sample_rate_conversion.md) | [SPEC018](SPEC018-crossfade_completion_coordination.md) | [SPEC021](SPEC021-error_handling.md) | [SPEC022](SPEC022-performance_targets.md)

---

## Metadata

**Document Type:** Tier 4 - Execution Plan (Implementation Guide)
**Version:** 1.0
**Date:** 2025-10-26
**Status:** Active
**Author:** Technical Lead

**Parent Documents (Tier 2 Specifications):**
- [SPEC002-crossfade.md](SPEC002-crossfade.md) - Crossfade timing and curves
- [SPEC013-single_stream_playback.md](SPEC013-single_stream_playback.md) - Single-stream architecture overview
- [SPEC016-decoder_buffer_design.md](SPEC016-decoder_buffer_design.md) - Decoder-buffer pipeline (AUTHORITATIVE for architecture)
- [SPEC017-sample_rate_conversion.md](SPEC017-sample_rate_conversion.md) - Sample rate conversion requirements
- [SPEC018-crossfade_completion_coordination.md](SPEC018-crossfade_completion_coordination.md) - Crossfade completion signaling
- [SPEC021-error_handling.md](SPEC021-error_handling.md) - Error handling strategy
- [SPEC022-performance_targets.md](SPEC022-performance_targets.md) - Performance benchmarks and targets

**Parent Documents (Tier 1 Requirements):**
- [REQ001-requirements.md](REQ001-requirements.md) - Complete WKMP requirements
- [REQ002-entity_definitions.md](REQ002-entity_definitions.md) - Entity model (Passage, Song, Recording, etc.)

**Parent Documents (Tier 3 Implementation):**
- [IMPL001-database_schema.md](IMPL001-database_schema.md) - Database schema for queue, passages, settings
- [IMPL002-coding_conventions.md](IMPL002-coding_conventions.md) - Rust coding standards

---

## Purpose

This guide orchestrates the re-implementation of wkmp-ap (Audio Player microservice) based on authoritative specifications. It addresses specification gaps identified in analysis and provides clear implementation pathway.

**Key Objectives:**
1. Provide single entry point for all wkmp-ap implementation specifications
2. Clarify ambiguities and resolve contradictions across specifications
3. Define implementation sequence and dependencies
4. Establish acceptance criteria for implementation completion
5. Enable systematic /plan workflow execution

**Context:**
- This is a **re-implementation** of wkmp-ap due to problems with prior implementation
- All specifications have been reviewed and critical gaps addressed
- SPEC014 (Single Stream Design) is OUTDATED - use SPEC016 as authoritative source
- Implementation must satisfy all specifications and pass comprehensive testing

---

## Executive Summary

**What This Document Is:**
- Orchestration layer that references specifications without duplicating content
- Implementation roadmap showing dependencies and sequencing
- Clarification of ambiguities found in specifications
- Entry point for /plan workflow execution

**What This Document Is NOT:**
- A replacement for detailed specifications (those remain authoritative)
- A tutorial or step-by-step implementation guide
- A design specification (use SPEC### documents for design details)

**How to Use This Document:**
1. Read this summary to understand scope and approach
2. Review clarifications section for resolved ambiguities
3. Follow implementation sequence to determine build order
4. Use /plan workflow on specific specifications for detailed implementation plans
5. Verify completion against acceptance criteria

---

## Scope

**In Scope:**
- Complete wkmp-ap Audio Player microservice re-implementation
- All components: decoder pipeline, buffer management, mixer, queue, API, SSE
- Sample-accurate crossfading with 5 fade curve types
- Error handling and recovery mechanisms
- Performance optimization for Pi Zero 2W deployment target
- HTTP REST API and Server-Sent Events (SSE) integration
- Database integration (queue persistence, passage metadata, settings)

**Out of Scope:**
- wkmp-ui (User Interface microservice)
- wkmp-pd (Program Director microservice)
- wkmp-ai (Audio Ingest microservice)
- wkmp-le (Lyric Editor microservice)
- wkmp-common library modifications (unless absolutely necessary)
- Database schema changes (work with existing IMPL001 schema)
- New feature development beyond current specifications

**Assumptions:**
1. SQLite database exists with schema per IMPL001
2. wkmp-common library provides shared types (Event, entities, etc.)
3. Deployment target is Pi Zero 2W (ARM Cortex-A53, 512MB RAM)
4. Audio files are in supported formats (MP3, FLAC, AAC, Vorbis, Opus)
5. rubato library provides adequate resampling functionality
6. symphonia library supports required audio formats
7. cpal library provides cross-platform audio output

**Constraints:**
- Must meet SPEC022 performance targets (CPU <40%, latency <500ms, memory <150MB)
- Must maintain compatibility with existing wkmp-common Event types
- Must follow IMPL002 Rust coding conventions
- Cannot modify database schema (use IMPL001 as-is)
- Must work on both development systems and Pi Zero 2W

---

## Specification Clarifications

This section resolves ambiguities found in specifications during analysis.

### SPEC014 vs SPEC016 Contradiction (RESOLVED)

**Issue:** SPEC014-single_stream_design.md describes parallel 2-thread decoder pool. SPEC016-decoder_buffer_design.md [DBD-DEC-040] specifies serial decode (single-threaded DecoderWorker).

**Resolution:**
- **SPEC016 is AUTHORITATIVE** - Use serial decode design (single-threaded DecoderWorker)
- SPEC014 contains WARNING redirecting to SPEC016
- Rationale: Serial decode improves cache coherency and reduces complexity

**Implementation Guidance:**
- Implement DecoderWorker as single thread processing all DecoderChains serially
- Follow SPEC016 architecture precisely
- Ignore parallel decoder pool references in SPEC014

### Buffer Decode Strategy (CLARIFIED)

**Issue:** SPEC016 [DBD-BUF-050] states "decoder is told to pause decoding until more than playout_ringbuffer_headroom samples are available" but does not explicitly define full decode strategy.

**Clarification (Authoritative):**
- **Strategy:** Incremental decode with hysteresis-based pause/resume
- **Behavior:**
  - Decoder fills buffer incrementally (not all-at-once)
  - When buffer reaches capacity (≤ playout_ringbuffer_headroom samples free), decoder PAUSES
  - Decoder remains paused while buffer has ≤ playout_ringbuffer_headroom samples free
  - When buffer has > playout_ringbuffer_headroom samples free (hysteresis threshold), decoder RESUMES
  - Process repeats: decode → pause at threshold → resume when space available → repeat

**Implementation Guidance:**
- Implement backpressure mechanism in DecoderChain
- DecoderWorker checks buffer fill status before each decode operation
- If buffer nearly full (≤ playout_ringbuffer_headroom free), skip decode for that chain
- Resume decode when buffer consumption creates > playout_ringbuffer_headroom free space
- This prevents buffer overflow and provides automatic flow control

**Rationale:**
- Prevents buffer overflow without complex coordination
- Automatic flow control based on consumption rate
- Reduces memory pressure (decode only as needed)
- Aligns with SPEC022 memory targets (<150MB total)

### SPEC021 Draft Status (AT-RISK DECISION)

**Issue:** SPEC021 (Error Handling Strategy) has "Draft" status, not yet approved.

**Decision:** Proceed at-risk using SPEC021 as authoritative for implementation.

**Mitigation:**
- Review SPEC021 during Phase 1 (Foundation) implementation
- Implement error handling framework early to minimize rework if changes required
- Use typed error enums (thiserror) for easy modification if taxonomy changes
- Test error scenarios comprehensively to validate strategy effectiveness

**Impact if SPEC021 changes:**
- Error taxonomy modifications → update error type enums
- Response strategy changes → update error handling logic in components
- Low rework risk due to modular error handling design

### Resampler State Management (AT-RISK DECISION)

**Issue:** SPEC017 defers resampler state management details to rubato library documentation.

**Decision:** Assume rubato StatefulResampler provides required functionality.

**Mitigation:**
- Validate rubato library early in Phase 3 (Audio Subsystem Basics)
- Test flush behavior for tail samples at passage boundaries
- Test pause/resume state preservation across chunk boundaries
- If rubato insufficient, implement thin wrapper to manage state externally

**Fallback Plan:**
- Wrap rubato resampler in custom stateful adapter
- Manually track input/output sample counts
- Implement custom flush logic if needed

**Impact if assumptions wrong:**
- 1-2 days additional effort for wrapper implementation
- No architectural changes required (wrapper is internal to DecoderChain)

---

## Implementation Architecture Overview

### Component Hierarchy

```
wkmp-ap/
├── main.rs                          # Server entry point, Axum setup
├── config.rs                        # Configuration loading (database + TOML)
├── error.rs                         # Error types (SPEC021 taxonomy)
├── events.rs                        # EventBus integration
├── api/
│   ├── handlers.rs                  # HTTP REST endpoints
│   └── sse.rs                       # Server-Sent Events broadcaster
├── playback/
│   ├── engine.rs                    # PlaybackEngine (orchestrator)
│   ├── queue.rs                     # Queue management, persistence
│   ├── state.rs                     # Shared playback state
│   └── pipeline/
│       ├── buffer.rs                # RingBuffer implementation
│       ├── decoder_chain.rs         # DecoderChain (Decoder→Resampler→Fader→Buffer)
│       ├── decoder_worker.rs        # Single-threaded serial decoder (SPEC016)
│       ├── mixer.rs                 # Crossfade mixer (SPEC002, SPEC018)
│       └── output.rs                # cpal audio output
└── tests/
    ├── integration/
    │   ├── crossfade_test.rs        # Sample-accurate crossfade verification
    │   ├── queue_test.rs            # Queue persistence and ordering
    │   └── error_handling_test.rs   # Error recovery scenarios
    └── unit/
        ├── buffer_test.rs           # Ring buffer operations
        ├── fade_curves_test.rs      # Fade curve mathematics
        └── decoder_chain_test.rs    # Decode→resample→fade pipeline
```

### Key Components and Specifications

| Component | Primary Spec | Requirements | Purpose |
|-----------|--------------|--------------|---------|
| PlaybackEngine | SPEC016 [DBD-DEC-040], SPEC018 | Queue orchestration | Top-level coordinator for playback lifecycle |
| DecoderChain | SPEC016 [DBD-CHAIN-###] | Decode→resample→fade→buffer | Integrated pipeline per passage |
| DecoderWorker | SPEC016 [DBD-DEC-040] | Serial decode of all chains | Single-threaded decoder processor |
| RingBuffer | SPEC016 [DBD-BUF-###] | PCM storage, backpressure | Lock-free ring buffer with hysteresis |
| Mixer | SPEC002 [XFD-###], SPEC018 | Crossfade execution | Sample-accurate crossfading with curves |
| Queue | SPEC016 [DBD-STARTUP-###] | Persistence, ordering | Queue management and database sync |
| API Handlers | SPEC007 | REST endpoints | HTTP control interface |
| SSE Broadcaster | SPEC011 | Real-time events | Server-Sent Events for UI updates |
| Error Handling | SPEC021 [ERH-###] | Recovery strategies | Comprehensive error taxonomy and responses |

---

## Implementation Sequence

Implementation follows dependency order: foundation → database → audio primitives → core playback → integration.

### Phase 1: Foundation (Week 1)

**Objective:** Establish project structure, error handling, configuration, and event integration.

**Specifications:**
- SPEC021 (Error Handling) - Error taxonomy and response strategies
- IMPL002 (Coding Conventions) - Rust standards
- IMPL001 (Database Schema) - Settings table structure

**Components:**
1. Project setup (Cargo.toml, workspace integration)
2. Error types (error.rs) - Implement SPEC021 [ERH-###] taxonomy
3. Config loader (config.rs) - Database settings + TOML bootstrap
4. Event integration (events.rs) - EventBus subscription/emission
5. Main entry point (main.rs) - CLI args, logging, initialization
6. Shared state (state.rs) - Arc<RwLock<SharedPlaybackState>>

**Acceptance Criteria:**
- All error types defined per SPEC021 taxonomy (FATAL, RECOVERABLE, DEGRADED, TRANSIENT)
- Configuration loaded from database (module_config table)
- Logging initialized (configured via settings or TOML)
- Event system integrated with wkmp-common EventBus
- Unit tests pass for config parsing and error type conversions

**/plan Invocation:**
```bash
/plan docs/SPEC021-error_handling.md
```

---

### Phase 2: Database Layer (Week 1-2)

**Objective:** Queue persistence, passage metadata access, settings management.

**Specifications:**
- IMPL001 (Database Schema) - queue, passages, settings tables
- SPEC016 [DBD-STARTUP-###] - Queue restoration on startup

**Components:**
1. Queue manager (queue.rs) - CRUD operations on queue table
2. Passage queries - Read passage timing data (start_time, end_time, fade_*, etc.)
3. Settings access - Volume, audio device, crossfade defaults, working_sample_rate
4. Play order management - Automatic renumbering, gap insertion

**Acceptance Criteria:**
- Queue entries can be created, read, updated, deleted
- Queue restored from database on startup per SPEC016 [DBD-STARTUP-010]
- Passage metadata accessible by passage_id
- Settings read from database with fallback to defaults
- Integration tests with SQLite in-memory database pass

**Dependencies:** Phase 1 (error types, config)

**/plan Invocation:**
```bash
/plan docs/SPEC016-decoder_buffer_design.md
# Focus on [DBD-STARTUP-###] requirements only for Phase 2
```

---

### Phase 3: Audio Subsystem Basics (Week 2-3)

**Objective:** Decode single audio file, resample, output via cpal.

**Specifications:**
- SPEC017 (Sample Rate Conversion) - Resampling requirements
- SPEC022 (Performance Targets) - Decode latency targets

**Components:**
1. Audio output (output.rs) - cpal device enumeration, stream setup
2. Simple decoder (decoder_chain.rs - minimal) - Decode one file with symphonia
3. Ring buffer (buffer.rs) - RingBuffer struct, PCM storage, basic operations
4. Sample rate conversion - rubato integration, StatefulResampler usage

**Acceptance Criteria:**
- Audio device enumerated and opened via cpal
- Single audio file decoded with symphonia (MP3, FLAC tested)
- Audio resampled to working_sample_rate (44100 Hz) using rubato
- Audio output to device (simple playback, no queue, no crossfade)
- Decode latency measured and compared to SPEC022 targets (<500ms)
- rubato StatefulResampler flush behavior validated for tail samples

**Dependencies:** Phase 1 (error types, config, events)

**AT-RISK Validation:**
- Confirm rubato library provides required state management
- Test pause/resume behavior across chunk boundaries
- If insufficient, implement wrapper (fallback plan)

**/plan Invocation:**
```bash
/plan docs/SPEC017-sample_rate_conversion.md
```

---

### Phase 4: Core Playback Engine (Week 3-5)

**Objective:** Queue processing, decoder-buffer chains, passage transitions, position tracking.

**Specifications:**
- SPEC016 (Decoder Buffer Design) - Complete DecoderChain, DecoderWorker architecture
- SPEC011 (Event System) - PassageStarted, PassageCompleted, PlaybackProgress events

**Components:**
1. DecoderChain complete (decoder_chain.rs) - Integrated Decoder→Resampler→Fader→Buffer pipeline
2. DecoderWorker (decoder_worker.rs) - Single-threaded serial decoder processing all chains
3. Buffer manager logic (buffer.rs complete) - Backpressure, hysteresis, pause/resume per clarification
4. PlaybackEngine (engine.rs) - Queue orchestration, chain assignment, lifecycle management
5. Position tracking - Sample-accurate position for events and API

**Acceptance Criteria:**
- DecoderChain integrates symphonia decoder, rubato resampler, fader, and RingBuffer
- DecoderWorker processes multiple chains serially per SPEC016 [DBD-DEC-040]
- Buffer backpressure implemented: pause at ≤ playout_ringbuffer_headroom free space
- Buffer resume implemented: resume at > playout_ringbuffer_headroom free space (hysteresis)
- PlaybackEngine assigns chains per SPEC016 [DBD-LIFECYCLE-###] rules
- Chain assignment persists throughout passage lifecycle (HashMap<QueueEntryId, ChainIndex>)
- Events emitted: PassageStarted, PassageCompleted, PlaybackProgress (500ms intervals)
- Integration test: Queue 3 passages, verify sequential playback (no crossfade yet)
- All chains released properly on passage completion or removal

**Dependencies:** Phase 2 (queue), Phase 3 (audio primitives)

**/plan Invocation:**
```bash
/plan docs/SPEC016-decoder_buffer_design.md
# Focus on complete architecture: DecoderChain, DecoderWorker, PlaybackEngine
```

---

### Phase 5: Crossfade Mixer (Week 5-7)

**Objective:** Sample-accurate crossfading with configurable curves and completion signaling.

**Specifications:**
- SPEC002 (Crossfade) - Timing, fade curves, sample-accurate triggering
- SPEC018 (Crossfade Completion Coordination) - Mixer-to-engine completion signaling

**Components:**
1. Mixer state machine (mixer.rs) - None/SinglePassage/Crossfading states
2. Fade curve implementation - Linear, Exponential, Logarithmic, S-Curve, Equal-Power (5 curves)
3. Sample-accurate triggering - Position-based crossfade start per SPEC002 [XFD-TIME-###]
4. Dual buffer mixing - Independent position tracking for outgoing/incoming passages
5. Fade parameter extraction - Read timing overrides from queue entries
6. Completion signaling - Implement SPEC018 channel-based mechanism

**Acceptance Criteria:**
- All 5 fade curves implemented and mathematically validated (unit tests)
- Crossfade triggered at exactly fade_out_start_time per SPEC002 [XFD-TIME-010]
- Dual buffer mixing maintains independent positions for outgoing/incoming passages
- Fade parameters read from queue entry (fade_in_duration_ms, fade_out_duration_ms, fade_type)
- Default fade parameters used when overrides are NULL
- Completion signaling per SPEC018: mixer sends (queue_entry_id, chain_index) via channel
- PlaybackEngine receives completion signal and releases chain
- Integration test: Crossfade between 2 passages with sample-accurate verification
- Clipping detection: log warning when crossfade sum > 1.0

**Dependencies:** Phase 4 (core playback engine)

**/plan Invocation:**
```bash
/plan docs/SPEC002-crossfade.md
/plan docs/SPEC018-crossfade_completion_coordination.md
```

---

### Phase 6: API Layer (Week 7-8)

**Objective:** HTTP REST endpoints for control and Server-Sent Events for real-time updates.

**Specifications:**
- SPEC007 (API Design) - REST endpoints, request/response formats
- SPEC011 (Event System) - SSE event streaming

**Components:**
1. HTTP server setup (main.rs) - Axum router, middleware, CORS
2. Control endpoints (handlers.rs) - enqueue, play, pause, skip, stop, volume, seek
3. Status endpoints (handlers.rs) - queue, position, buffer_status, settings
4. SSE broadcaster (sse.rs) - Subscribe to EventBus, forward events to connected clients
5. Health endpoint - Module health check for respawning

**Acceptance Criteria:**
- All REST endpoints per SPEC007 implemented and tested
- Request validation with appropriate error responses (400, 404, 500)
- SSE endpoint streams events to clients (PassageStarted, PlaybackProgress, etc.)
- Multiple SSE clients supported simultaneously
- API integration tests using curl or HTTP client library
- Health endpoint returns status and uptime

**Dependencies:** Phase 5 (crossfade mixer - complete playback functionality)

**/plan Invocation:**
```bash
/plan docs/SPEC007-api_design.md
# Focus on wkmp-ap endpoints only (not wkmp-ui or other services)
```

---

### Phase 7: Error Handling & Recovery (Week 8-9)

**Objective:** Implement comprehensive error handling per SPEC021 with graceful degradation.

**Specifications:**
- SPEC021 (Error Handling Strategy) - Error taxonomy, response strategies, recovery

**Components:**
1. Decode failure handling - Skip passage, emit error event, continue playback
2. Device unavailable handling - Retry with backoff, emit degraded mode event
3. Buffer underrun handling - Auto-pause, resume when buffer ready
4. Queue inconsistency handling - Reconcile chain assignments on error
5. Database lock retry - Exponential backoff for SQLite busy errors
6. Event emission for all error scenarios

**Acceptance Criteria:**
- All SPEC021 [ERH-###] error categories handled appropriately
- FATAL errors: Clean shutdown with state persistence
- RECOVERABLE errors: Retry with exponential backoff (3 attempts)
- DEGRADED errors: Continue with reduced functionality, emit events
- TRANSIENT errors: Automatic recovery without user intervention
- Error events emitted per SPEC011 for UI notification
- Integration tests for each error scenario
- No panics or crashes under error conditions
- Memory leaks tested under error scenarios

**Dependencies:** Phase 6 (API layer - complete system)

**/plan Invocation:**
```bash
/plan docs/SPEC021-error_handling.md
```

---

### Phase 8: Performance Optimization & Validation (Week 9-10)

**Objective:** Meet SPEC022 performance targets and validate on Pi Zero 2W.

**Specifications:**
- SPEC022 (Performance Targets) - CPU, latency, memory, throughput benchmarks

**Tasks:**
1. Performance profiling - Identify hot paths using flamegraph or perf
2. Optimize allocations - Reduce heap allocations in audio thread
3. Lock contention analysis - Minimize RwLock/Mutex hold times
4. Memory leak testing - Valgrind, 24+ hour continuous playback
5. CPU usage measurement - Verify <40% on Pi Zero 2W under normal load
6. Decode latency measurement - Verify <500ms from enqueue to decode start
7. Memory footprint measurement - Verify <150MB total (RSS) under load
8. Throughput validation - Verify crossfade timing accuracy (<1ms error)

**Acceptance Criteria:**
- CPU usage <40% on Pi Zero 2W during playback with 2 active chains
- Decode latency <500ms from passage enqueue to first buffer samples
- Memory footprint <150MB RSS during continuous playback
- Crossfade timing accuracy within ±1ms of specified times
- No memory leaks detected after 24-hour continuous playback
- No audio dropouts or glitches under normal operating conditions
- Performance regression tests added to test suite

**Dependencies:** Phase 7 (error handling - complete implementation)

**Validation Platform:** Raspberry Pi Zero 2W (ARM Cortex-A53, 512MB RAM)

**/plan Invocation:**
```bash
/plan docs/SPEC022-performance_targets.md
```

---

## Acceptance Criteria for Completion

Implementation is complete when ALL criteria below are satisfied:

### Functional Completeness

- [ ] All specifications implemented (SPEC002, SPEC013, SPEC016, SPEC017, SPEC018, SPEC021, SPEC022)
- [ ] All phases (1-8) completed with acceptance criteria met
- [ ] All REST API endpoints functional per SPEC007
- [ ] All SSE events emitted per SPEC011
- [ ] All error handling scenarios covered per SPEC021
- [ ] Queue persistence working per SPEC016 [DBD-STARTUP-###]

### Quality Standards

- [ ] Unit test coverage >80% for core components (buffer, mixer, decoder_chain)
- [ ] Integration tests pass for all major workflows (playback, crossfade, queue, errors)
- [ ] No compiler warnings (clippy clean)
- [ ] Code follows IMPL002 coding conventions
- [ ] All public APIs documented with rustdoc comments
- [ ] Performance targets met per SPEC022 (CPU <40%, latency <500ms, memory <150MB)

### Validation Testing

- [ ] Manual testing on development system (Linux/macOS/Windows)
- [ ] Manual testing on Pi Zero 2W deployment target
- [ ] 24-hour continuous playback test (no crashes, no leaks)
- [ ] Crossfade accuracy test (sample-accurate timing verification)
- [ ] Multi-client SSE test (multiple browsers connected)
- [ ] Error recovery test (decode failures, device removal, database errors)

### Documentation

- [ ] Implementation deviations documented (if any)
- [ ] Known limitations documented
- [ ] Performance benchmarks recorded
- [ ] API documentation generated (rustdoc)

---

## Risk Assessment

### High-Risk Areas

**1. Crossfade Timing Accuracy**
- **Risk:** Sample-accurate crossfading requires precise position tracking
- **Mitigation:** Comprehensive testing with known audio files, automated verification
- **Contingency:** Manual verification with oscilloscope/audio analysis tools

**2. Performance on Pi Zero 2W**
- **Risk:** Limited CPU/memory may not meet SPEC022 targets
- **Mitigation:** Early performance testing, profiling, optimization
- **Contingency:** Reduce maximum_decode_streams from 3 to 2, adjust buffer sizes

**3. Error Recovery Complexity**
- **Risk:** SPEC021 defines complex error taxonomy with many scenarios
- **Mitigation:** Incremental implementation, comprehensive testing per scenario
- **Contingency:** Simplify to fewer error categories if implementation proves too complex

### Medium-Risk Areas

**4. rubato Library Compatibility**
- **Risk:** Library may not provide required state management (AT-RISK decision)
- **Mitigation:** Early validation in Phase 3, fallback wrapper design ready
- **Impact:** 1-2 days additional effort if wrapper needed

**5. Database Lock Contention**
- **Risk:** SQLite busy errors under concurrent access
- **Mitigation:** Retry with exponential backoff per SPEC021
- **Impact:** Performance degradation under heavy load, mitigated by retry logic

### Low-Risk Areas

**6. cpal Audio Output**
- **Risk:** Cross-platform audio output may have platform-specific issues
- **Mitigation:** Test on all target platforms early
- **Impact:** Platform-specific workarounds may be needed

---

## Implementation Tools and Workflow

### Development Workflow

1. **For each phase:**
   - Run `/plan [specification]` to generate detailed implementation plan
   - Implement components following plan increments
   - Write tests BEFORE implementation (TDD approach)
   - Run tests after each increment
   - Commit after passing tests using `/commit` workflow

2. **Testing:**
   - Unit tests for all core functions (fade curves, buffer operations, etc.)
   - Integration tests for workflows (playback, crossfade, queue, errors)
   - Manual testing on development system and Pi Zero 2W
   - Performance profiling at end of each phase

3. **Documentation:**
   - rustdoc comments for all public APIs
   - Update this guide if implementation deviations occur
   - Record performance benchmarks in SPEC022 verification notes

### /plan Workflow Usage

This guide serves as the orchestration layer for /plan execution. Use /plan on individual specifications:

```bash
# Phase 1: Foundation
/plan docs/SPEC021-error_handling.md

# Phase 2: Database Layer
/plan docs/SPEC016-decoder_buffer_design.md  # Focus on [DBD-STARTUP-###]

# Phase 3: Audio Subsystem
/plan docs/SPEC017-sample_rate_conversion.md

# Phase 4: Core Playback
/plan docs/SPEC016-decoder_buffer_design.md  # Complete architecture

# Phase 5: Crossfade
/plan docs/SPEC002-crossfade.md
/plan docs/SPEC018-crossfade_completion_coordination.md

# Phase 6: API Layer
/plan docs/SPEC007-api_design.md

# Phase 7: Error Handling
/plan docs/SPEC021-error_handling.md

# Phase 8: Performance
/plan docs/SPEC022-performance_targets.md
```

Each /plan invocation produces:
- Requirements index for that specification
- Acceptance test definitions
- Implementation increments
- Traceability matrix

---

## Version History

| Version | Date | Changes | Author |
|---------|------|---------|--------|
| 1.0 | 2025-10-26 | Initial creation - orchestrates wkmp-ap re-implementation | Technical Lead |

---

## References

**Tier 1 (Requirements):**
- [REQ001-requirements.md](REQ001-requirements.md) - Complete WKMP requirements
- [REQ002-entity_definitions.md](REQ002-entity_definitions.md) - Entity model

**Tier 2 (Specifications):**
- [SPEC002-crossfade.md](SPEC002-crossfade.md) - Crossfade timing and curves
- [SPEC007-api_design.md](SPEC007-api_design.md) - REST API design
- [SPEC011-event_system.md](SPEC011-event_system.md) - Event system and SSE
- [SPEC013-single_stream_playback.md](SPEC013-single_stream_playback.md) - Architecture overview
- [SPEC014-single_stream_design.md](SPEC014-single_stream_design.md) - **OUTDATED** - See SPEC016 instead
- [SPEC016-decoder_buffer_design.md](SPEC016-decoder_buffer_design.md) - **AUTHORITATIVE** decoder-buffer architecture
- [SPEC017-sample_rate_conversion.md](SPEC017-sample_rate_conversion.md) - Sample rate conversion
- [SPEC018-crossfade_completion_coordination.md](SPEC018-crossfade_completion_coordination.md) - Crossfade completion signaling
- [SPEC021-error_handling.md](SPEC021-error_handling.md) - Error handling strategy
- [SPEC022-performance_targets.md](SPEC022-performance_targets.md) - Performance benchmarks

**Tier 3 (Implementation):**
- [IMPL001-database_schema.md](IMPL001-database_schema.md) - Database schema
- [IMPL002-coding_conventions.md](IMPL002-coding_conventions.md) - Rust coding standards

**Tier 0 (Governance):**
- [GOV001-document_hierarchy.md](GOV001-document_hierarchy.md) - Documentation framework
- [GOV002-requirements_enumeration.md](GOV002-requirements_enumeration.md) - Requirement ID scheme

**Related Guides:**
- [GUIDE001-wkmp_ap_implementation_plan.md](GUIDE001-wkmp_ap_implementation_plan.md) - Original implementation plan (reference)

---

**Document Status:** Active - Ready for /plan workflow execution
**Next Action:** Run `/plan docs/SPEC021-error_handling.md` to begin Phase 1 implementation planning
