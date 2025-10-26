# WKMP Audio Player (wkmp-ap) - Specification Index

**Created:** 2025-10-22
**Purpose:** Master index of all specification documents for re-creating wkmp-ap

---

## Document Overview

This index organizes all specification documents for the WKMP Audio Player (wkmp-ap). The specifications are organized hierarchically from high-level system architecture down to detailed component implementations.

**Total Specifications Created:** 3 core specifications + this index

---

## Specification Hierarchy

### Level 1: System Overview (1 document)

#### WKMP-AP-SPEC-001: System Architecture Overview
**File:** `/tmp/WKMP-AP-SPEC-001-System-Architecture-Overview.md`
**Status:** ✅ Complete
**Size:** ~17,000 words

**Contents:**
- Executive summary and key capabilities
- System context and external dependencies
- High-level architecture and component hierarchy
- Major subsystems overview (6 subsystems)
- Data flow diagrams
- Threading model and synchronization
- State machines (Buffer, Mixer, Playback)
- Performance characteristics
- Error handling strategy
- Configuration architecture
- API surface (25+ endpoints)
- Build and deployment
- Testing strategy
- Extensibility points
- Known limitations and future considerations

**Purpose:** Provides the complete big-picture view of the system. Start here to understand overall architecture before drilling into subsystems.

---

### Level 2: Major Subsystems (3 documents created, 4 pending)

#### WKMP-AP-SPEC-002: Playback Engine Subsystem
**File:** `/tmp/WKMP-AP-SPEC-002-Playback-Engine-Subsystem.md`
**Status:** ✅ Complete
**Size:** ~9,500 words

**Contents:**
- Component hierarchy and execution model
- PlaybackEngine orchestrator (data structure, initialization, main loop)
- Event handling (buffer events, position events)
- Queue progression logic
- Crossfade timing and initiation
- Passage completion handling
- Chain assignment management
- QueueManager component details
- DecoderWorker component details
- Integration points with other subsystems
- Timing precision specifications
- Error handling and recovery
- Performance optimization strategies

**Purpose:** Detailed specification for the central orchestrator that coordinates all playback operations.

**Key Sections:**
- Section 3: PlaybackEngine Component (data structure, initialization sequence, main loop)
- Section 3.3: Main Loop Implementation (detailed tick() logic)
- Section 3.5: Queue Progression Logic
- Section 3.6: Crossfade Timing Logic
- Section 4: QueueManager Component
- Section 5: DecoderWorker Component

---

#### WKMP-AP-SPEC-003: Audio Processing Subsystem
**File:** `/tmp/WKMP-AP-SPEC-003-Audio-Processing-Subsystem.md`
**Status:** 📝 Pending (template exists, needs completion)

**Planned Contents:**
- Audio decoder component (Symphonia integration)
- Audio resampler component (Rubato integration)
- Audio output component (CPAL integration)
- Audio types and data structures
- Sample rate conversion algorithms
- Audio format support matrix
- Device management and fallback
- Error recovery strategies

**Purpose:** Specification for audio file decoding, resampling, and device output.

---

#### WKMP-AP-SPEC-004: Buffer Management Subsystem
**File:** `/tmp/WKMP-AP-SPEC-004-Buffer-Management-Subsystem.md`
**Status:** 📝 Pending

**Planned Contents:**
- BufferManager component
- Buffer state machine (Empty→Filling→Ready→Playing→Finished)
- Event-driven buffer readiness notification
- Underrun detection and recovery
- Hysteresis logic for pause/resume
- Buffer allocation and deallocation
- Memory management strategies
- First-passage optimization

**Purpose:** Specification for buffer lifecycle management and state machine.

---

#### WKMP-AP-SPEC-005: Queue Management Subsystem
**File:** `/tmp/WKMP-AP-SPEC-005-Queue-Management-Subsystem.md`
**Status:** 📝 Pending (partially covered in SPEC-002)

**Planned Contents:**
- Queue database schema
- Queue entry data structure
- Queue operations (enqueue, remove, reorder, clear)
- Play order management
- Timing override handling
- Discovered endpoint propagation

**Purpose:** Specification for queue state tracking and database operations.

---

#### WKMP-AP-SPEC-006: HTTP API Subsystem
**File:** `/tmp/WKMP-AP-SPEC-006-HTTP-API-Subsystem.md`
**Status:** 📝 Pending

**Planned Contents:**
- Axum router setup
- REST endpoint implementations (25+ endpoints)
- Server-Sent Events (SSE) stream
- Request/response data structures
- Error responses and status codes
- CORS configuration
- Authentication/authorization (if applicable)

**Purpose:** Specification for HTTP control interface and monitoring.

---

#### WKMP-AP-SPEC-007: Database Layer Subsystem
**File:** `/tmp/WKMP-AP-SPEC-007-Database-Layer-Subsystem.md`
**Status:** 📝 Pending

**Planned Contents:**
- Database schema (queue, passages, passage_songs, settings tables)
- SQLite configuration
- Query patterns and optimizations
- Transaction management
- Database initialization sequence
- Settings management
- Migration strategy

**Purpose:** Specification for data persistence and configuration storage.

---

### Level 3: Component Details (1 document created, 11 pending)

#### WKMP-AP-SPEC-010: Crossfade Mixer Component
**File:** `/tmp/WKMP-AP-SPEC-010-Crossfade-Mixer-Component.md`
**Status:** ✅ Complete
**Size:** ~8,200 words

**Contents:**
- Mixer state machine (None→SinglePassage→Crossfading)
- Data structures (MixerState, UnderrunState, PauseState, ResumeState)
- Core operations (start_passage, start_crossfade, get_next_frame)
- Fade curve types and calculations (5 curve types)
- Underrun detection and auto-resume logic
- Pause/resume with fade-in
- Position tracking and event emission
- Performance characteristics
- Thread safety considerations
- Error handling

**Purpose:** Detailed specification for sample-accurate crossfading and audio mixing.

**Key Sections:**
- Section 2: Component Architecture (state machine, data structures)
- Section 3.4: Get Next Frame (core mixing logic - 150+ lines of pseudocode)
- Section 4: Fade Curve Application
- Section 5: Underrun Detection and Recovery
- Section 6: Position Tracking and Events

---

#### WKMP-AP-SPEC-011: Decoder Component
**File:** `/tmp/WKMP-AP-SPEC-011-Decoder-Component.md`
**Status:** 📝 Pending

**Planned Contents:**
- Symphonia integration
- Opus codec support (via libopus FFI)
- Decode-and-skip approach for passage timing
- Endpoint discovery for undefined endpoints
- Multi-format support (MP3, FLAC, AAC, Vorbis, Opus)
- Error handling (corrupt files, unsupported codecs)

**Purpose:** Specification for audio file decoding.

---

#### WKMP-AP-SPEC-012: Ring Buffer Component
**File:** `/tmp/WKMP-AP-SPEC-012-Ring-Buffer-Component.md`
**Status:** 📝 Pending

**Planned Contents:**
- PlayoutRingBuffer implementation
- Lock-free operations (push_frame, pop_frame)
- Capacity and headroom management
- Exhaustion detection
- Underrun handling
- Buffer statistics (fill percent, occupancy)
- Memory layout

**Purpose:** Specification for lock-free ring buffer used for audio sample storage.

---

#### Additional Pending Component Specs:
- **WKMP-AP-SPEC-013:** Resampler Component (Rubato integration)
- **WKMP-AP-SPEC-014:** Audio Output Component (CPAL integration)
- **WKMP-AP-SPEC-015:** DecoderChain Component (decode→resample→fade→buffer pipeline)
- **WKMP-AP-SPEC-016:** Fader Component (fade curve calculation)
- **WKMP-AP-SPEC-017:** ValidationService Component (pipeline integrity checks)
- **WKMP-AP-SPEC-018:** SharedState Component (thread-safe state management)

---

### Level 4: Cross-Cutting Concerns (pending)

#### WKMP-AP-SPEC-008: Initialization and Lifecycle
**File:** `/tmp/WKMP-AP-SPEC-008-Initialization-and-Lifecycle.md`
**Status:** 📝 Pending

**Planned Contents:**
- Application startup sequence (13 steps)
- Configuration resolution (CLI → env → TOML → database)
- Component initialization order
- Graceful shutdown sequence
- Error handling during startup
- Bootstrap configuration (TOML)
- Database configuration loading

**Purpose:** Specification for application lifecycle management.

---

#### WKMP-AP-SPEC-009: Data Structures and Types
**File:** `/tmp/WKMP-AP-SPEC-009-Data-Structures-and-Types.md`
**Status:** 📝 Pending

**Planned Contents:**
- AudioFrame structure
- PassageBuffer structure
- QueueEntry structure
- Event types (BufferEvent, PlaybackEvent, WkmpEvent)
- Configuration structures
- Error types
- Type conversions and utilities

**Purpose:** Specification for core data types used throughout the system.

---

## How to Use These Specifications

### For Complete System Re-creation:

1. **Start with SPEC-001** (System Architecture Overview)
   - Understand overall architecture
   - Identify major subsystems
   - Learn data flow and threading model

2. **Read SPEC-002** (Playback Engine Subsystem)
   - Understand the central orchestrator
   - Learn queue progression logic
   - Understand crossfade timing

3. **Read SPEC-010** (Crossfade Mixer Component)
   - Understand sample-accurate mixing
   - Learn fade curve application
   - Understand underrun handling

4. **Implement in Order:**
   - Audio types and data structures (SPEC-009)
   - Ring buffer component (SPEC-012)
   - Audio decoder (SPEC-011)
   - Audio resampler (SPEC-013)
   - Audio output (SPEC-014)
   - Crossfade mixer (SPEC-010)
   - Buffer manager (SPEC-004)
   - Queue manager (SPEC-005)
   - Decoder worker (partial in SPEC-002)
   - Playback engine (SPEC-002)
   - HTTP API (SPEC-006)
   - Initialization (SPEC-008)

### For Understanding Specific Features:

**Crossfading:**
- SPEC-010 (Crossfade Mixer) - Complete implementation
- SPEC-002 Section 3.6 (Crossfade timing in engine)

**Buffer Management:**
- SPEC-004 (Buffer Management) - State machine and lifecycle
- SPEC-012 (Ring Buffer) - Low-level buffer implementation

**Queue Management:**
- SPEC-002 Section 4 (QueueManager)
- SPEC-005 (Queue Management) - Database operations

**Audio Processing:**
- SPEC-003 (Audio Processing overview)
- SPEC-011 (Decoder)
- SPEC-013 (Resampler)
- SPEC-014 (Audio Output)

**API:**
- SPEC-006 (HTTP API) - All endpoints and SSE

---

## Specification Statistics

### Completed Specifications

| Spec ID | Document | Words | Status |
|---------|----------|-------|--------|
| SPEC-001 | System Architecture Overview | ~17,000 | ✅ Complete |
| SPEC-002 | Playback Engine Subsystem | ~9,500 | ✅ Complete |
| SPEC-010 | Crossfade Mixer Component | ~8,200 | ✅ Complete |
| **Total** | **3 specifications** | **~34,700** | **3 of 15+ planned** |

### Coverage Analysis

**Completed Coverage:**
- ✅ High-level architecture (100%)
- ✅ Playback engine orchestration (100%)
- ✅ Crossfade mixer implementation (100%)
- ✅ Queue management basics (80% - covered in SPEC-002)

**Pending Coverage:**
- 📝 Audio decoding (0%)
- 📝 Audio resampling (0%)
- 📝 Audio output (0%)
- 📝 Buffer management state machine (0%)
- 📝 Ring buffer implementation (0%)
- 📝 HTTP API (0%)
- 📝 Database layer (0%)
- 📝 Initialization sequence (0%)
- 📝 Data structures (0%)

---

## Implementation Roadmap

Based on the specifications created, here's a suggested implementation order:

### Phase 1: Foundation (Weeks 1-2)
1. Set up Rust project structure
2. Implement core data types (AudioFrame, PassageBuffer)
3. Implement ring buffer (PlayoutRingBuffer)
4. Unit test ring buffer operations

### Phase 2: Audio Processing (Weeks 3-4)
5. Implement audio decoder (SimpleDecoder with Symphonia)
6. Implement audio resampler (Rubato integration)
7. Implement audio output (CPAL integration)
8. Integration test: decode → resample → output

### Phase 3: Mixing (Weeks 5-6)
9. Implement fade curve calculations
10. Implement crossfade mixer (CrossfadeMixer)
11. Integration test: single passage playback
12. Integration test: crossfade between two passages

### Phase 4: Buffer Management (Weeks 7-8)
13. Implement buffer state machine (BufferManager)
14. Implement event emission
15. Integration test: buffer lifecycle

### Phase 5: Queue and Engine (Weeks 9-10)
16. Implement queue manager (QueueManager)
17. Implement decoder worker (DecoderWorker)
18. Implement playback engine (PlaybackEngine)
19. Integration test: full playback flow

### Phase 6: API and Database (Weeks 11-12)
20. Implement database layer (SQLite schema)
21. Implement HTTP API (Axum router + handlers)
22. Implement SSE event stream
23. Integration test: API control of playback

### Phase 7: Polish (Weeks 13-14)
24. Implement error handling and recovery
25. Implement validation service
26. Performance optimization
27. End-to-end testing

---

## Key Design Decisions Captured

### Architecture
- **Single-stream:** One continuous audio stream (not multiple parallel streams)
- **Event-driven:** State changes trigger events (not polling)
- **Lock-free hot path:** Ring buffers and atomics for audio callback

### Threading
- **100Hz engine loop:** Tokio interval timer for orchestration
- **Audio callback thread:** Real-time constraints, no blocking
- **Single-threaded decoder:** Maximize cache efficiency

### Buffer Management
- **State machine:** Empty→Filling→Ready→Playing→Finished
- **Event emission:** Non-blocking notification of state changes
- **Hysteresis:** Prevent oscillation during pause/resume

### Crossfading
- **Sample-accurate:** Frame-level precision (~0.02ms)
- **5 fade curves:** Linear, Exponential, Cosine, S-Curve, Logarithmic
- **Automatic completion detection:** Mixer signals when crossfade done

### Configuration
- **Database-first:** Runtime settings in SQLite
- **4-tier priority:** CLI → env → TOML → database

---

## File Locations

All specification documents are located in `/tmp/` directory:

```
/tmp/
├── WKMP-AP-SPECIFICATIONS-INDEX.md (this file)
├── WKMP-AP-SPEC-001-System-Architecture-Overview.md
├── WKMP-AP-SPEC-002-Playback-Engine-Subsystem.md
└── WKMP-AP-SPEC-010-Crossfade-Mixer-Component.md
```

**Note:** These are temporary files. Copy them to a permanent location before the system clears `/tmp/`.

---

## Contributing Additional Specifications

If you create additional specifications to fill the pending gaps, follow these guidelines:

### Naming Convention
- Format: `WKMP-AP-SPEC-XXX-Title-With-Hyphens.md`
- XXX = 3-digit number (001-999)
- Use hyphens to separate words in title

### Document Structure
1. Header (Document ID, Version, Date, Parent)
2. Purpose (1-2 paragraphs)
3. Architecture/Overview (diagrams helpful)
4. Data Structures (Rust-style pseudocode)
5. Core Operations (detailed algorithms)
6. Integration Points (with other components)
7. Performance Characteristics
8. Error Handling
9. Testing Approach
10. Related Specifications
11. Document History

### Level Assignment
- **Level 1:** System-wide overview (1 document total)
- **Level 2:** Major subsystems (6-8 documents)
- **Level 3:** Individual components (10-20 documents)
- **Level 4:** Cross-cutting concerns (2-5 documents)

---

## Next Steps

### To Complete Full Specification Set:

1. **Audio Processing Subsystem (SPEC-003)**
   - Read source files: `audio/decoder.rs`, `audio/resampler.rs`, `audio/output.rs`
   - Document Symphonia integration
   - Document Rubato configuration
   - Document CPAL device management

2. **Buffer Management Subsystem (SPEC-004)**
   - Read source files: `playback/buffer_manager.rs`, `playback/buffer_events.rs`
   - Document state machine in detail
   - Document event emission logic
   - Document hysteresis calculations

3. **HTTP API Subsystem (SPEC-006)**
   - Read source files: `api/server.rs`, `api/handlers.rs`, `api/sse.rs`
   - Document all 25+ endpoints
   - Document request/response schemas
   - Document SSE event stream format

4. **Ring Buffer Component (SPEC-012)**
   - Read source file: `playback/playout_ring_buffer.rs`
   - Document lock-free push/pop operations
   - Document memory layout and capacity management
   - Document exhaustion detection algorithm

5. **Initialization and Lifecycle (SPEC-008)**
   - Read source file: `main.rs`
   - Document 13-step initialization sequence
   - Document configuration resolution
   - Document graceful shutdown

---

## Document History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2025-10-22 | System Analyst | Initial index with 3 complete specifications |

---

## Summary

**Status:** 3 of 15+ specifications complete (~20% coverage)

**Completed specifications provide:**
- Complete system architecture understanding
- Detailed playback engine implementation guide
- Complete crossfade mixer implementation
- Foundation for implementing core playback functionality

**Remaining specifications needed:**
- Audio processing components (decoder, resampler, output)
- Buffer management details
- Ring buffer implementation
- HTTP API details
- Database schema and operations
- Initialization and lifecycle
- Supporting components (fader, validation service, etc.)

**Estimated effort to complete:**
- Completed: ~35,000 words, ~110 pages
- Remaining: ~65,000 words, ~200 pages
- Total: ~100,000 words, ~310 pages (comprehensive specification set)
