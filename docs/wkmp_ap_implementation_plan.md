# wkmp-ap Implementation Plan

**Module:** Audio Player (wkmp-ap)
**Purpose:** Core playback engine with sample-accurate crossfading
**Architecture:** Single-stream audio pipeline (symphonia + rubato + cpal)

---

## Executive Summary

This document provides a phased implementation plan for building wkmp-ap from scratch. The module is responsible for:
- Decoding audio files (MP3, FLAC, AAC, Vorbis, Opus)
- Sample-accurate crossfading (~0.02ms precision)
- Queue management and playback coordination
- HTTP/SSE-based control interface
- Real-time position tracking

**Approach:** Build foundational data structures first, then layer on audio processing, and finally integrate with HTTP API. Each phase builds upon previous phases with clear testing checkpoints.

**Estimated Timeline:** 8-10 weeks full-time development

---

## Module Inventory

| Component | File Path | Purpose | Complexity |
|-----------|-----------|---------|------------|
| Main Entry | `src/main.rs` | Server initialization, CLI args | Low |
| Playback Engine | `src/playback/engine.rs` | Queue coordination, lifecycle | High |
| Queue Manager | `src/playback/queue.rs` | Queue persistence, ordering | Medium |
| Shared State | `src/playback/state.rs` | Current passage, position tracking | Low |
| Buffer Manager | `src/playback/pipeline/single_stream/buffer.rs` | PCM buffer lifecycle | Medium |
| Decoder Pool | `src/playback/pipeline/single_stream/decoder.rs` | Parallel decode with symphonia | High |
| Crossfade Mixer | `src/playback/pipeline/single_stream/mixer.rs` | Sample-accurate mixing, fade curves | High |
| Audio Output | `src/playback/pipeline/single_stream/output.rs` | cpal integration, device management | Medium |
| HTTP Handlers | `src/api/handlers.rs` | REST endpoints for control | Medium |
| SSE Broadcaster | `src/api/sse.rs` | Real-time event streaming | Medium |
| Error Types | `src/error.rs` | Module-specific error handling | Low |
| Config Loader | `src/config.rs` | Database module_config reading | Low |
| Event Integration | `src/events.rs` | EventBus subscription, emission | Low |

**Total:** 13 major components (4 High, 5 Medium, 4 Low complexity)

---

## Implementation Phases

### Phase 1: Foundation (Week 1)
**Goal:** Establish project structure, error handling, configuration

**Components:**
1. **Project Setup** - Cargo.toml with dependencies, workspace integration
2. **Error Types** - Define `PlaybackError`, `DecodeError`, `ApiError` using thiserror
3. **Config Loader** - Read `module_config` table for host/port, root folder path
4. **Main Entry Point** - CLI arg parsing, logging initialization, root folder resolution
5. **Shared State** - Basic `Arc<RwLock<SharedPlaybackState>>` structure

**Why This Order:**
- Foundation before building on top
- Clear error propagation from day one
- Configuration needed before any networking

**Testing:** Unit tests for config parsing, error type conversions

---

### Phase 2: Database Layer (Week 1-2)
**Goal:** Queue persistence, passage metadata access

**Components:**
1. **Queue Manager** (`queue.rs`) - CRUD operations on `queue` table
2. **Passage Queries** - Read passage timing data from database
3. **Settings Access** - Volume, audio device, crossfade defaults
4. **Play Order Management** - Automatic renumbering, gap insertion

**Why This Order:**
- Queue is foundation for all playback operations
- Database operations are synchronous/blocking - isolate early
- Play order logic is self-contained

**Testing:** Integration tests with SQLite in-memory database

---

### Phase 3: Audio Subsystem Basics (Week 2-3)
**Goal:** Decode single audio file, output via cpal

**Components:**
1. **Audio Output** (`output.rs`) - cpal device enumeration, stream setup
2. **Simple Decoder** (`decoder.rs` - minimal) - Decode one file with symphonia
3. **Buffer Structure** (`buffer.rs`) - PassageBuffer struct, PCM storage
4. **Sample Rate Conversion** - rubato integration for 44.1kHz normalization

**Why This Order:**
- Prove audio stack works before adding complexity
- Single-threaded decode first, parallelize later
- Validate cpal integration on target platform

**Testing:** Play single audio file end-to-end (no crossfade yet)

---

### Phase 4: Core Playback Engine (Week 3-5)
**Goal:** Queue processing, passage transitions, event emission

**Components:**
1. **Buffer Manager** (`buffer.rs` - complete) - Full/partial buffer strategy
2. **Decoder Pool** (`decoder.rs` - complete) - Thread pool, decode-and-skip
3. **Playback Loop** (`engine.rs`) - Queue pop, buffer wait, mixer coordination
4. **Position Tracking** - Sample-accurate position for SSE events
5. **Event Emission** - PassageStarted, PassageCompleted, PlaybackProgress

**Why This Order:**
- Buffer management needed before crossfading
- Playback loop ties everything together
- Position tracking enables UI integration

**Testing:** Queue 3 passages, verify sequential playback (no crossfade)

---

### Phase 5: Crossfade Mixer (Week 5-7)
**Goal:** Sample-accurate crossfading with configurable curves

**Components:**
1. **Crossfade State Machine** (`mixer.rs`) - None/SinglePassage/Crossfading states
2. **Fade Curve Implementation** - Linear, Exponential, Logarithmic, S-Curve, Equal-Power
3. **Sample-Accurate Triggering** - Buffer position-based crossfade start
4. **Dual Buffer Mixing** - Independent position tracking per passage
5. **Fade Parameter Extraction** - Read timing overrides from queue entries

**Why This Order:**
- State machine provides structure for complex logic
- Curves are pure functions - test independently
- Triggering logic depends on state machine
- Mixing is final integration of all components

**Testing:** Crossfade integration test (REF: `tests/crossfade_test.rs`)

---

### Phase 6: API Layer (Week 7-8)
**Goal:** HTTP endpoints for control, SSE event streaming

**Components:**
1. **HTTP Server Setup** (`main.rs`) - Axum router, middleware
2. **Control Endpoints** (`handlers.rs`) - enqueue, play, pause, skip, volume
3. **Status Endpoints** (`handlers.rs`) - queue, position, buffer_status
4. **SSE Broadcaster** (`sse.rs`) - Subscribe to EventBus, forward to clients
5. **Health Endpoint** - Module health check for respawning

**Why This Order:**
- Playback engine must work before exposing API
- Control endpoints are CRUD operations
- SSE requires working event system

**Testing:** API integration tests with curl, verify SSE event stream

---

### Phase 7: Integration & Polish (Week 8-10)
**Goal:** Robustness, error handling, performance tuning

**Tasks:**
1. **Error Recovery** - Decode failures, device unavailable, queue empty handling
2. **Database Lock Retry** - Exponential backoff for SQLite busy errors
3. **Buffer Underrun Handling** - Auto-pause, resume when buffer ready
4. **Clipping Detection** - Log warnings when crossfade sum > 1.0
5. **Performance Profiling** - Optimize hot paths, reduce allocations
6. **Memory Leak Testing** - Valgrind, play for 24 hours continuously

**Why This Order:**
- Core functionality must work before hardening
- Error scenarios emerge from integration testing
- Performance issues visible under load

**Testing:** Stress test with 100+ passages, rapid skipping, device changes

---

## Component Build Order

**Dependency-Ordered List (numbered for tracking):**

### Tier 1: Foundation (No dependencies)
1. Error types (`error.rs`)
2. Config loader (`config.rs`) - depends on `wkmp-common`
3. Shared state structs (`state.rs`)

### Tier 2: Database Access (Depends on Tier 1)
4. Queue manager (`queue.rs`) - CRUD operations
5. Passage queries (part of `queue.rs` or separate `db.rs`)
6. Settings access (part of `config.rs`)

### Tier 3: Audio Primitives (Depends on Tier 1)
7. Buffer structure (`buffer.rs` - data types only)
8. Audio output (`output.rs` - device setup, no playback yet)
9. Simple decoder (`decoder.rs` - single-threaded, no pool yet)

### Tier 4: Core Playback (Depends on Tiers 1-3)
10. Buffer manager (`buffer.rs` - lifecycle, strategy logic)
11. Decoder pool (`decoder.rs` - thread pool, priority queue)
12. Playback engine loop (`engine.rs` - queue processing)
13. Event integration (`events.rs` - EventBus subscription)

### Tier 5: Crossfade System (Depends on Tiers 1-4)
14. Fade curve functions (`mixer.rs` - pure functions)
15. Crossfade state machine (`mixer.rs` - state management)
16. Sample-accurate triggering (`mixer.rs` - trigger logic)
17. Dual buffer mixing (`mixer.rs` - mixing algorithm)

### Tier 6: API Layer (Depends on Tiers 1-5)
18. HTTP server setup (`main.rs` - Axum router)
19. Control endpoints (`handlers.rs` - REST API)
20. Status endpoints (`handlers.rs` - GET APIs)
21. SSE broadcaster (`sse.rs` - event streaming)

### Tier 7: Polish (Depends on all previous)
22. Error recovery strategies (all modules)
23. Performance optimizations (hot paths)
24. Integration testing (end-to-end scenarios)

---

## Critical Implementation Notes

### Sample-Accurate Timing
- All timing calculations in **samples**, not milliseconds
- Conversion: `samples = ms * 44100 / 1000`
- Crossfade trigger: `trigger_sample = duration_samples - overlap_samples`
- Position tracking: Increment by frames consumed, not wall-clock time

### Thread Synchronization
- **Audio callback thread**: No blocking operations, no allocations
- **Mixer thread**: Pre-calculates audio, fills ring buffer
- **Decoder threads**: Blocking I/O isolated in thread pool
- **HTTP thread pool**: Tokio async, database queries use `spawn_blocking`

### Buffer Handoff Mechanism
- **Partial buffer**: 15 seconds (configurable) for queued passages
- **Complete buffer**: Full passage decode when playing/next
- **Atomic swap**: Use `Arc<RwLock<PassageBuffer>>` for seamless transition
- **Position continuity**: Both buffers share sample position tracking

### Lock-Free Audio Thread
- **Ring buffer**: Pre-allocated, fixed size, lock-free reads
- **Volume control**: Atomic f32 for lock-free access
- **State queries**: Use atomic flags, avoid mutexes in audio callback

### Crossfade Algorithm
- **Independent position tracking**: Separate `current_passage_position` and `next_passage_position`
- **Fade gain calculation**: Pre-compute per sample, not per buffer
- **Clamping**: Always clamp output to `[-1.0, 1.0]` to prevent clipping
- **Curve application**: Multiplicative with resume-from-pause fade (if active)

### Event Emission Timing
- **PassageStarted**: Emit when mixer starts reading buffer
- **PassageCompleted**: Emit when mixer exhausts buffer OR user skips
- **PlaybackProgress**: Emit every 5000ms (configurable), also on play/pause
- **BufferStateChanged**: Emit on Decoding→Ready, Ready→Playing, Playing→Exhausted
- **CurrentSongChanged**: Check every 500ms, emit when boundary crossed

---

## Testing Checkpoints

### After Phase 1
- [x] Config loads from database correctly
- [x] Error types convert between layers
- [x] Logging outputs to file with timestamps

### After Phase 2
- [x] Queue persists across restarts
- [x] Play order maintains gaps correctly
- [x] Passage timing overrides read successfully

### After Phase 3
- [x] Audio device enumerates devices
- [x] Single audio file decodes and plays to completion
- [x] Sample rate conversion produces audible output

### After Phase 4
- [x] Queue processes 3+ passages sequentially
- [x] Position tracking reports accurate values
- [x] PassageStarted/Completed events emit correctly

### After Phase 5
- [x] Crossfade triggers at correct sample position
- [x] Two passages mix with smooth fade curves
- [x] No audible clicks or pops during transition

### After Phase 6
- [x] REST API endpoints respond correctly
- [x] SSE stream delivers events to web clients
- [x] Health endpoint reports module status

### After Phase 7
- [x] 24-hour continuous playback without crashes
- [x] Rapid skip operations don't cause glitches
- [x] Device unplugged/replugged recovers gracefully

---

## Key Integration Tests

### Test 1: Sequential Playback
**Setup:** Queue 5 passages with no crossfade (zero lead durations)
**Verify:** Each plays start to finish, queue decrements, events emit

### Test 2: Crossfade Transitions
**Setup:** Queue 3 passages with 8-second crossfade
**Verify:** Sample-accurate triggering, smooth transitions, no gaps

### Test 3: Rapid Skipping
**Setup:** Queue 20 passages, skip every 2 seconds
**Verify:** No buffer underruns, events emit correctly, queue updates

### Test 4: Volume Changes
**Setup:** Play passage, change volume 10 times during playback
**Verify:** Volume changes audible, no clicks, events emit

### Test 5: Device Switching
**Setup:** Play passage, switch audio device mid-playback
**Verify:** Playback continues on new device, no audio dropout

### Test 6: Empty Queue Behavior
**Setup:** Play until queue empty, then enqueue new passage
**Verify:** Player idle when empty, resumes immediately when enqueued

### Test 7: Database Lock Contention
**Setup:** Simulate heavy database load with concurrent operations
**Verify:** Exponential backoff succeeds, no playback interruption

### Test 8: Buffer Underrun Recovery
**Setup:** Force slow decode (large file, slow disk)
**Verify:** Auto-pause when buffer exhausted, resume when ready

---

## Performance Criteria

### Memory (Raspberry Pi Zero2W Target)
- Idle: < 50 MB
- 5 passages queued (2 full, 3 partial): < 150 MB
- Peak decode: < 200 MB

### CPU (Raspberry Pi Zero2W Target)
- Idle: < 2%
- Decoding: < 40% (two decoder threads active)
- Mixing during crossfade: < 10%
- Total during active playback: < 50%

### Latency
- API response time: < 50ms for control commands
- Crossfade trigger precision: < 20ms (~880 samples @ 44.1kHz)
- Skip operation latency: < 100ms (if buffer ready)

### Throughput
- Decode speed: 2-5x real-time (depends on file format)
- Queue refill request handling: < 100ms response (acknowledgment only)

---

## Implementation Checklist

### Phase 1: Foundation
- [ ] Create `Cargo.toml` with dependencies (tokio, symphonia, rubato, cpal, axum)
- [ ] Define error types in `src/error.rs`
- [ ] Implement config loader reading `module_config` table
- [ ] Set up logging with tracing crate (file + line numbers)
- [ ] Create `SharedPlaybackState` struct
- [ ] Write unit tests for config parsing

### Phase 2: Database Layer
- [ ] Implement `QueueManager` with queue table CRUD
- [ ] Add play order gap management and renumbering
- [ ] Create passage timing query functions
- [ ] Implement settings read/write (volume, audio device)
- [ ] Write integration tests with SQLite in-memory database
- [ ] Test play order overflow protection (> 2.1B)

### Phase 3: Audio Subsystem Basics
- [ ] Implement cpal device enumeration and selection
- [ ] Create simple single-file decoder with symphonia
- [ ] Add rubato resampling to 44.1kHz
- [ ] Define `PassageBuffer` struct (PCM data + metadata)
- [ ] Build basic audio output with cpal stream
- [ ] Test end-to-end playback of single audio file

### Phase 4: Core Playback Engine
- [ ] Implement `PassageBufferManager` with full/partial strategy
- [ ] Create `DecoderPool` with thread pool (2 workers)
- [ ] Add decode-and-skip approach (never use compressed seek)
- [ ] Build playback loop in `PlaybackEngine`
- [ ] Implement position tracking (sample-accurate)
- [ ] Emit PassageStarted, PassageCompleted, PlaybackProgress events
- [ ] Test sequential playback of 3+ passages (no crossfade)

### Phase 5: Crossfade Mixer
- [ ] Implement CrossfadeState enum (None/SinglePassage/Crossfading)
- [ ] Define fade curve functions (Linear, Exp, Log, SCurve, EqualPower)
- [ ] Add crossfade trigger calculation (sample-based)
- [ ] Build sample-accurate triggering logic (auto-start)
- [ ] Implement dual buffer mixing with independent positions
- [ ] Extract fade parameters from timing overrides
- [ ] Write crossfade integration test (`tests/crossfade_test.rs`)
- [ ] Test with 3 passages, 8-second crossfade

### Phase 6: API Layer
- [ ] Set up Axum HTTP server with router
- [ ] Implement control endpoints (enqueue, play, pause, skip, volume)
- [ ] Implement status endpoints (queue, position, buffer_status)
- [ ] Build SSE broadcaster subscribing to EventBus
- [ ] Add health endpoint for module monitoring
- [ ] Write API integration tests with curl

### Phase 7: Integration & Polish
- [ ] Implement error recovery (decode failure, device unavailable)
- [ ] Add database lock retry with exponential backoff
- [ ] Implement buffer underrun handling (auto-pause/resume)
- [ ] Add clipping detection with logging
- [ ] Profile hot paths and optimize allocations
- [ ] Run 24-hour continuous playback test
- [ ] Test rapid skipping (100+ skips in 60 seconds)
- [ ] Test device switching during playback

**Total:** 30 major checklist tasks

---

## Top 3 Complexity Areas

### 1. Crossfade Mixer State Machine
**Challenge:** Coordinating dual buffer mixing with sample-accurate triggering
**Risk:** Off-by-one errors causing clicks, buffer overruns, position drift
**Mitigation:** Extensive unit tests for each state transition, integration test with known audio

### 2. Decoder Pool Thread Safety
**Challenge:** Parallel decode with priority queue, buffer handoff, cancellation
**Risk:** Race conditions, deadlocks, memory leaks, buffer use-after-free
**Mitigation:** Use Arc/RwLock correctly, test with ThreadSanitizer, validate buffer lifecycle

### 3. Lock-Free Audio Thread
**Challenge:** Real-time audio callback with zero blocking operations
**Risk:** Audio underruns, glitches, dropouts, jitter
**Mitigation:** Pre-allocate ring buffer, atomic operations only, profile callback latency

---

## Implementation Order Summary

**First:** Error types, config, shared state (foundation)
**Second:** Queue manager, database access (persistence)
**Third:** Simple decoder, audio output (prove audio stack)
**Fourth:** Playback engine, buffer manager, decoder pool (core playback)
**Fifth:** Crossfade mixer, fade curves, triggering (sample-accurate mixing)
**Sixth:** HTTP API, SSE broadcaster (external interface)
**Last:** Error recovery, performance tuning, stress testing (robustness)

---

**Document Version:** 1.0
**Created:** 2025-10-17
**Author:** AI Assistant (Claude)
**Purpose:** Practical implementation guide for code-implementer agent

**References:**
- `/home/sw/Dev/McRhythm/docs/single-stream-design.md` - Complete architecture specification
- `/home/sw/Dev/McRhythm/docs/api_design.md` - API endpoint contracts
- `/home/sw/Dev/McRhythm/docs/crossfade.md` - Crossfade algorithm details
- `/home/sw/Dev/McRhythm/docs/event_system.md` - SSE event specifications
- `/home/sw/Dev/McRhythm/docs/architecture.md` - System architecture overview
- `/home/sw/Dev/McRhythm/docs/coding_conventions.md` - Rust coding standards
