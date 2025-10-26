# WKMP Audio Player (wkmp-ap) - System Architecture Overview

**Document ID:** WKMP-AP-SPEC-001
**Version:** 1.0
**Date:** 2025-10-22
**Purpose:** High-level system architecture specification for re-creating the WKMP Audio Player

---

## 1. Executive Summary

The WKMP Audio Player (wkmp-ap) is a sample-accurate audio playback engine designed for automatic music passage playback with seamless crossfading. It operates as an independent HTTP microservice providing REST API and Server-Sent Events (SSE) interfaces for control and monitoring.

### Key Capabilities

- Sample-accurate audio playback with ~0.02ms precision
- Seamless crossfading between passages using 5 fade curve types
- Multi-format audio decoding (MP3, FLAC, AAC, Vorbis, Opus)
- Queue management with passage ordering
- Real-time position tracking and buffer monitoring
- HTTP/SSE control interface on port 5721

### Design Philosophy

- **Single-stream architecture**: One continuous audio stream with sample-accurate timing
- **Event-driven**: State machines and event emission avoid polling
- **Lock-free where possible**: Ring buffers and atomics for hot paths
- **Database-first configuration**: Runtime settings in SQLite
- **Graceful degradation**: Underrun recovery, device fallback

---

## 2. System Context

### 2.1 Operating Environment

- **Language:** Rust (stable channel)
- **Async Runtime:** Tokio multi-threaded executor
- **Target Platform:** Linux, macOS, Windows (cross-platform via cpal)
- **Database:** SQLite with JSON1 extension
- **Network:** HTTP server on configurable port (default 5721)

### 2.2 External Dependencies

**Core Audio Processing:**
- `symphonia` 0.5 - Multi-format audio decoding
- `symphonia-adapter-libopus` 0.2.3 - Opus codec via libopus C library FFI
- `rubato` 0.15 - Audio resampling (SincFixedIn algorithm)
- `cpal` 0.15 - Cross-platform audio device output
- `ringbuf` 0.4 - Lock-free ring buffers

**HTTP and Async:**
- `tokio` - Async runtime with channels and synchronization primitives
- `axum` - HTTP web framework
- `tower-http` - HTTP middleware (CORS, compression)

**Data and Persistence:**
- `sqlx` - SQLite database with compile-time query checking
- `serde` + `serde_json` - Serialization framework
- `uuid` - UUID generation and parsing
- `chrono` - Time/date handling

**Error Handling and Logging:**
- `thiserror` - Custom error type derivation
- `anyhow` - Error context wrapper
- `tracing` - Structured logging framework

---

## 3. High-Level Architecture

### 3.1 Architectural Pattern

The wkmp-ap uses a **single-stream, event-driven architecture** with three main layers:

```
┌─────────────────────────────────────────────────────────────┐
│                     HTTP API Layer                           │
│  (Axum server, REST endpoints, SSE event stream)            │
└──────────────────────┬──────────────────────────────────────┘
                       │
┌──────────────────────┴──────────────────────────────────────┐
│                 Playback Engine Layer                        │
│  (Queue management, buffer lifecycle, decode coordination)  │
└──────────┬───────────────────┬──────────────────────────────┘
           │                   │
    ┌──────┴─────┐      ┌──────┴────────┐
    │  Audio     │      │  Database     │
    │  Pipeline  │      │  Layer        │
    │  Layer     │      │  (SQLite)     │
    └────────────┘      └───────────────┘
```

### 3.2 Component Hierarchy

```
wkmp-ap/
├── main.rs                       # Entry point and initialization
├── api/                          # HTTP API Layer
│   ├── server.rs                 # Axum server setup and routing
│   ├── handlers.rs               # REST endpoint implementations
│   └── sse.rs                    # Server-Sent Events stream
│
├── playback/                     # Playback Engine Layer
│   ├── engine.rs                 # PlaybackEngine (orchestrator)
│   ├── queue_manager.rs          # Queue state tracking
│   ├── buffer_manager.rs         # Buffer lifecycle state machine
│   ├── decoder_worker.rs         # Single-threaded decode worker
│   ├── buffer_events.rs          # Buffer state events
│   ├── events.rs                 # Playback position events
│   ├── diagnostics.rs            # Pipeline validation metrics
│   ├── validation_service.rs    # Automatic integrity checks
│   ├── song_timeline.rs          # Passage boundary tracking
│   ├── playout_ring_buffer.rs   # Lock-free ring buffer
│   └── pipeline/                 # Audio Processing Pipeline
│       ├── mixer.rs              # CrossfadeMixer (sample-accurate)
│       ├── decoder_chain.rs      # Decode→Resample→Fade→Buffer chain
│       ├── fader.rs              # Fade curve application
│       └── timing.rs             # Sample-accurate timing calculations
│
├── audio/                        # Audio Processing Layer
│   ├── decoder.rs                # Symphonia-based audio decoder
│   ├── resampler.rs              # Rubato-based resampler
│   ├── output.rs                 # Cpal-based audio device output
│   └── types.rs                  # AudioFrame, PassageBuffer
│
├── db/                           # Database Layer
│   ├── init.rs                   # Schema initialization
│   ├── queue.rs                  # Queue table operations
│   ├── passages.rs               # Passage table queries
│   ├── passage_songs.rs          # Passage-song relationships
│   └── settings.rs               # Settings table (volume, etc.)
│
├── state.rs                      # Shared playback state
├── config.rs                     # Configuration loading
└── error.rs                      # Custom error types
```

---

## 4. Major Subsystems

### 4.1 Playback Engine Subsystem

**Purpose:** Orchestrates all playback operations, coordinates buffer management, decode scheduling, and mixer timing.

**Key Components:**
- `PlaybackEngine` - Main orchestrator running at ~100Hz
- `QueueManager` - Tracks current/next/queued passages
- `BufferManager` - Manages buffer lifecycle with state machine
- `DecoderWorker` - Single-threaded decoder with priority queue

**Responsibilities:**
- Queue progression (current → next → queued)
- Buffer allocation and lifecycle management
- Decode task scheduling with priorities
- Mixer synchronization and frame generation
- Event emission for state changes

### 4.2 Audio Processing Subsystem

**Purpose:** Decode audio files, resample to standard rate, apply fade curves, mix passages, output to audio device.

**Key Components:**
- `SimpleDecoder` - Symphonia-based multi-format decoder
- `Resampler` - Rubato-based resampling to 44.1kHz
- `CrossfadeMixer` - Sample-accurate crossfading with state machine
- `Fader` - Fade curve calculation and application
- `AudioOutput` - Cpal-based device output with callback

**Responsibilities:**
- Decode audio files from various formats
- Resample all audio to 44.1kHz standard rate
- Apply fade-in/fade-out curves during transitions
- Mix overlapping passages during crossfades
- Output audio stream to hardware device

### 4.3 Buffer Management Subsystem

**Purpose:** Manage buffer lifecycle, detect readiness, handle underruns, optimize memory usage.

**Key Components:**
- `BufferManager` - Central buffer lifecycle coordinator
- `PlayoutRingBuffer` - Lock-free ring buffer for audio samples
- `BufferState` - State machine (Empty→Filling→Ready→Playing→Finished)
- `BufferEvent` - Event types for state transitions

**Responsibilities:**
- Allocate buffers for queue entries
- Track buffer fill state and readiness
- Emit events when buffers reach thresholds
- Detect and signal buffer exhaustion
- Implement pause/resume hysteresis logic

### 4.4 Queue Management Subsystem

**Purpose:** Track which passages are in the playback pipeline, manage queue progression.

**Key Components:**
- `QueueManager` - Queue state tracking
- `QueueEntry` - Passage with timing overrides
- Database queue table operations

**Responsibilities:**
- Load queue from database on startup
- Track current/next/queued passages
- Advance queue on passage completion
- Support adding, removing, reordering entries
- Maintain O(1) queue length tracking

### 4.5 HTTP API Subsystem

**Purpose:** Provide control interface and real-time monitoring via HTTP REST and SSE.

**Key Components:**
- `AppContext` - Shared application state
- `server.rs` - Axum router setup
- `handlers.rs` - REST endpoint implementations
- `sse.rs` - Server-Sent Events stream

**Responsibilities:**
- Serve 25+ REST endpoints for control
- Stream real-time events via SSE
- Handle playback control (play, pause, skip, seek)
- Provide queue management (enqueue, remove, reorder)
- Expose buffer and diagnostic monitoring

### 4.6 Database Layer

**Purpose:** Persist queue state, passage metadata, and runtime configuration.

**Key Components:**
- SQLite database with JSON1 extension
- Queue, passages, passage_songs, settings tables
- Database initialization and migration

**Responsibilities:**
- Store queue entries with timing overrides
- Persist passage metadata (timing, crossfade points)
- Store runtime settings (volume, device, thresholds)
- Support atomic transactions for queue operations

---

## 5. Data Flow

### 5.1 Playback Initialization Flow

```
User calls POST /playback/enqueue
    ↓
Handler adds passage to database queue table
    ↓
PlaybackEngine tick detects new queue entry
    ↓
BufferManager allocates new buffer (Empty state)
    ↓
Engine dispatches decode task to DecoderWorker
    ↓
DecoderWorker creates DecoderChain:
  Decoder (symphonia) → Resampler (rubato) → Fade prep → Ring buffer
    ↓
Decoder reads audio file and decodes samples
    ↓
Resampler converts to 44.1kHz
    ↓
Samples pushed to ring buffer
    ↓
BufferManager: Empty→Filling→Ready (~500ms buffered)
    ↓
BufferManager emits ReadyForStart event
    ↓
Mixer receives event, transitions to Playing state
    ↓
Mixer reads frames from ring buffer, applies fade curves
    ↓
AudioOutput callback pulls frames from mixer
    ↓
Audio output to device (cpal) → user hears audio
```

### 5.2 Crossfade Flow

```
Current passage playing, next passage ready
    ↓
Engine calculates crossfade start point:
  (lead_out_point - fade_out_duration)
    ↓
Engine calls mixer.start_crossfade()
    ↓
Mixer transitions: SinglePassage → Crossfading
    ↓
For each frame during crossfade:
  - Read frame from current buffer, apply fade-out curve
  - Read frame from next buffer, apply fade-in curve
  - Mix (sum) the two frames
  - Clamp to [-1.0, 1.0] range
    ↓
When crossfade duration complete:
  - Mixer transitions: Crossfading → SinglePassage (next)
  - Engine removes old buffer
  - Engine advances queue (current → next → queued)
    ↓
Mixer continues playing new passage
```

### 5.3 Event Propagation Flow

```
Component generates event
    ↓
Event sent to tokio::broadcast channel
    ↓
SharedState broadcasts to all SSE subscribers
    ↓
SSE connection streams event to HTTP clients
    ↓
Web UI receives event and updates display
```

---

## 6. Threading Model

### 6.1 Thread Types

**Main Thread (Tokio Executor):**
- HTTP request handling
- Database operations
- Queue management
- Engine ticks (~100Hz)

**Audio Callback Thread (CPAL):**
- Real-time audio output
- Runs at audio device sample rate (typically 44.1kHz)
- Must never block or allocate

**Decoder Worker Thread (Tokio Task):**
- Background decoding of audio files
- Serial processing with priority queue

### 6.2 Synchronization Primitives

- `Arc<RwLock<T>>` - Shared state with concurrent reads
- `Arc<Mutex<T>>` - Exclusive access to mutable state
- `tokio::broadcast` - One-to-many event distribution
- `tokio::sync::mpsc` - Producer-consumer channels
- `AtomicU64`, `AtomicBool` - Lock-free counters and flags
- `PlayoutRingBuffer` - Lock-free ring buffer (internal mutex for ring ops only)

---

## 7. State Machines

### 7.1 Buffer State Machine

```
Empty
  ↓ (first samples appended)
Filling
  ↓ (threshold reached: 500ms-3s depending on config)
Ready
  ↓ (mixer starts reading)
Playing
  ↓ (decode complete and buffer drained)
Finished
```

### 7.2 Mixer State Machine

```
None (no audio)
  ↓ (start_passage called)
SinglePassage
  ↓ (start_crossfade called)
Crossfading
  ↓ (crossfade duration complete)
SinglePassage (next passage)
```

### 7.3 Playback State Machine

```
Playing
  ↔ (pause/resume)
Paused
```

---

## 8. Performance Characteristics

### 8.1 Timing Precision

- **Sample accuracy:** ~0.02ms at 44.1kHz (1 frame)
- **Engine tick rate:** ~100Hz (10ms intervals)
- **Position events:** Configurable (default: 1 second intervals)
- **Buffer headroom:** 0.1-5 seconds (configurable)

### 8.2 Latency Profile

- **Decode latency:** 0.5-3 seconds (pre-buffering)
- **Crossfade latency:** 0ms (pre-planned based on timing points)
- **Seek latency:** N/A (seeking not supported with drain-based ring buffers)
- **API response:** <10ms for most endpoints

### 8.3 Memory Usage

- **Per-buffer overhead:** ~15 seconds @ 44.1kHz = ~2.6MB stereo
- **Ring buffer capacity:** Configurable (default: 661,941 samples)
- **Typical usage:** 3-5 buffers active = ~10-15MB audio data
- **Database:** <100KB for queue and settings

### 8.4 CPU Usage

- **Idle:** <1% CPU (engine ticking, no decode)
- **Decode:** 10-30% CPU (symphonia, rubato, single-threaded)
- **Playback:** <5% CPU (mixer, cpal callback)
- **Peak:** ~40% CPU (decode + playback + HTTP requests)

---

## 9. Error Handling Strategy

### 9.1 Error Categories

**Recoverable Errors:**
- Audio device disconnected → fallback to default device
- Buffer underrun → pause and resume when sufficient data
- Decode error on packet → skip packet and continue
- Network timeout → retry with backoff

**Non-Recoverable Errors:**
- Database corruption → terminate with error
- No audio devices available → terminate with error
- Invalid passage timing → skip passage, continue queue

### 9.2 Recovery Mechanisms

**Underrun Recovery:**
1. Detect: Buffer empty but decode incomplete
2. Pause: Output last valid frame (flatline)
3. Resume: When buffer >10% full
4. Log: Diagnostic information for troubleshooting

**Device Fallback:**
1. Detect: Audio stream error callback
2. Stop: Current audio stream
3. Fallback: Attempt default device
4. Resume: Restart playback from last position

---

## 10. Configuration Architecture

### 10.1 Configuration Layers

1. **Bootstrap Configuration (TOML file):**
   - Database path
   - Log level
   - HTTP port (can be overridden by database)

2. **Database Configuration (settings table):**
   - Volume
   - Audio device name
   - Buffer thresholds
   - Position event intervals
   - All runtime-tunable parameters

3. **Command-Line Arguments:**
   - Override database path
   - Override HTTP port
   - Override root folder

### 10.2 Configuration Priority

```
CLI Args > Environment Variables > TOML > Database > Compiled Defaults
```

---

## 11. API Surface

### 11.1 REST Endpoints (25+)

**Playback Control:**
- `POST /playback/enqueue` - Add passage to queue
- `POST /playback/play` - Resume playback
- `POST /playback/pause` - Pause playback
- `POST /playback/next` - Skip to next passage
- `POST /playback/seek` - Seek within passage (not supported)
- `GET /playback/state` - Get playback state
- `GET /playback/position` - Get current position
- `GET /playback/queue` - Get queue contents

**Queue Management:**
- `DELETE /playback/queue/{id}` - Remove from queue
- `POST /playback/queue/clear` - Clear entire queue
- `POST /playback/queue/reorder` - Reorder queue entry

**Audio Control:**
- `GET /audio/devices` - List available devices
- `GET /audio/device` - Get current device
- `POST /audio/device` - Set audio device
- `GET /audio/volume` - Get volume
- `POST /audio/volume` - Set volume

**Monitoring:**
- `GET /playback/buffer_status` - Buffer fill percentages
- `GET /playback/buffer_chains` - Detailed buffer chain info
- `GET /playback/diagnostics` - Pipeline integrity metrics

**System:**
- `GET /health` - Health check
- `GET /build_info` - Build version and timestamp
- `GET /settings/all` - All runtime settings
- `POST /settings/bulk_update` - Update multiple settings

### 11.2 Server-Sent Events (SSE)

**Event Stream:** `GET /events`

**Event Types:**
- `QueueChanged` - Queue modified (enqueue, remove, clear)
- `PlaybackStateChanged` - Playing/Paused state change
- `PassageStarted` - New passage began playing
- `PassageCompleted` - Passage finished
- `PositionUpdate` - Periodic position updates (1s interval)
- `BufferStatusChanged` - Buffer state transitions

---

## 12. Build and Deployment

### 12.1 Build Commands

```bash
# Development build
cargo build -p wkmp-ap

# Release build (optimized)
cargo build -p wkmp-ap --release

# Run tests
cargo test -p wkmp-ap

# Run benchmarks
cargo bench -p wkmp-ap
```

### 12.2 Deployment Artifacts

```
wkmp-ap                  # Binary executable
wkmp-ap.toml             # Bootstrap configuration (optional)
wkmp.db                  # SQLite database (created on first run)
```

### 12.3 Runtime Requirements

- **OS:** Linux, macOS, or Windows
- **Architecture:** x86_64 or aarch64
- **Libraries:** System audio libraries (ALSA/PulseAudio/CoreAudio/WASAPI)
- **Opus codec:** libopus installed (for Opus file support)

---

## 13. Testing Strategy

### 13.1 Test Types

**Unit Tests:**
- Located in `#[cfg(test)]` modules within each source file
- Test individual functions and data structures
- ~150+ unit tests across all modules

**Integration Tests:**
- Located in `tests/` directory
- Test end-to-end workflows (enqueue → decode → play)
- Test API endpoints and SSE streams
- ~20 integration test files

**Performance Benchmarks:**
- Located in `benches/` directory
- Measure decode speed, mixer throughput, crossfade overhead
- Track performance regressions
- 8 benchmark suites

### 13.2 Test Coverage

- **Target:** >80% line coverage
- **Critical paths:** 100% coverage (mixer, buffer manager, queue manager)
- **Error paths:** Tested with intentional failures

---

## 14. Extensibility Points

### 14.1 Adding New Audio Formats

1. Register codec with symphonia codec registry
2. Update `SimpleDecoder` to handle new format
3. Add format-specific tests

### 14.2 Adding New Fade Curves

1. Add variant to `FadeCurve` enum (wkmp-common)
2. Implement `calculate_fade_in()` and `calculate_fade_out()` methods
3. Update API to accept new curve names

### 14.3 Adding New API Endpoints

1. Add route to `api/server.rs` router
2. Implement handler in `api/handlers.rs`
3. Add endpoint to API documentation

### 14.4 Adding New Events

1. Add variant to `WkmpEvent` enum (wkmp-common)
2. Emit event from appropriate component
3. Update SSE stream handling if needed

---

## 15. Known Limitations

### 15.1 Current Limitations

- **No seeking:** Drain-based ring buffers don't support seeking
- **No pitch shifting:** All audio played at original pitch
- **No real-time effects:** No EQ, reverb, or other DSP effects
- **Single output device:** Cannot split audio to multiple devices simultaneously
- **No gapless playback:** Crossfades only, no true gapless

### 15.2 Performance Limits

- **Max queue length:** Unlimited (limited by database size)
- **Max buffer size:** ~15 seconds (configurable up to 5s)
- **Max concurrent decodes:** 1 (single-threaded decoder worker)
- **Max SSE connections:** Limited by system resources (~1000+)

---

## 16. Future Considerations

### 16.1 Potential Enhancements

- **Parallel decoding:** Multi-threaded decoder for faster startup
- **Seeking support:** Implement position reset in ring buffers
- **Pitch shifting:** Time-stretch/pitch-shift during playback
- **Real-time effects:** DSP effects chain (EQ, compression, etc.)
- **Multiple outputs:** Route to multiple devices simultaneously
- **Gapless playback:** True gapless for consecutive tracks

### 16.2 Architecture Evolution

- **Plugin system:** Dynamic loading of audio processors
- **Distributed decoding:** Off-load decoding to separate service
- **Hardware acceleration:** GPU-accelerated resampling/effects
- **WASM support:** Run in browser via WebAssembly

---

## 17. Related Specifications

This document is the top-level specification. Detailed specifications for subsystems:

- `WKMP-AP-SPEC-002` - Playback Engine Subsystem
- `WKMP-AP-SPEC-003` - Audio Processing Subsystem
- `WKMP-AP-SPEC-004` - Buffer Management Subsystem
- `WKMP-AP-SPEC-005` - Queue Management Subsystem
- `WKMP-AP-SPEC-006` - HTTP API Subsystem
- `WKMP-AP-SPEC-007` - Database Layer Subsystem
- `WKMP-AP-SPEC-008` - Initialization and Lifecycle
- `WKMP-AP-SPEC-009` - Data Structures and Types
- `WKMP-AP-SPEC-010` - Crossfade Mixer Component
- `WKMP-AP-SPEC-011` - Decoder Component
- `WKMP-AP-SPEC-012` - Ring Buffer Component

---

## Document History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2025-10-22 | System Analyst | Initial comprehensive specification |

