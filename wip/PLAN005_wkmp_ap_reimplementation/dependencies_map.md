# Dependencies Map - wkmp-ap Re-Implementation

**Plan:** PLAN005_wkmp_ap_reimplementation
**Source:** docs/GUIDE002-wkmp_ap_re_implementation_guide.md
**Date:** 2025-10-26

---

## Existing Code Dependencies

### wkmp-common Library (EXISTS)

**Status:** ✅ Exists - Available in workspace
**Path:** `common/`
**Provides:**
- Event types (PassageStarted, PassageCompleted, PlaybackProgress, etc.)
- Entity models (Passage, Song, Recording, QueueEntry, etc.)
- Database utilities
- Shared types and enums
- EventBus implementation

**Required By:**
- Foundation (Phase 1) - Event integration
- Database Layer (Phase 2) - Entity models
- Core Playback (Phase 4) - Event emission
- API Layer (Phase 6) - Request/response types

**Assumptions:**
- Event types are stable (no breaking changes expected)
- Entity models match IMPL001 schema
- EventBus provides tokio::broadcast integration

**Risk:** Low - Library is established, interfaces documented

---

### Database Schema (EXISTS)

**Status:** ✅ Exists - Defined in IMPL001
**Path:** `migrations/` + `docs/IMPL001-database_schema.md`
**Provides:**
- queue table (queue_entry_id, passage_id, play_order, fade overrides, etc.)
- passages table (passage_id, recording_id, start_time, end_time, fade timing, etc.)
- settings table (key-value configuration)
- module_config table (module-specific settings)
- recordings, songs, artists tables (metadata)

**Required By:**
- Database Layer (Phase 2) - All CRUD operations
- Core Playback (Phase 4) - Queue restoration, passage metadata
- Crossfade Mixer (Phase 5) - Fade parameter extraction

**Assumptions:**
- Schema is complete (no modifications allowed)
- SQLite JSON1 extension available
- Foreign key cascades configured correctly

**Risk:** Low - Schema is authoritative per IMPL001

---

## External Library Dependencies

### Rust Standard Libraries

**Status:** ✅ Stable channel available
**Provides:** std, core, alloc
**Risk:** None - Baseline requirement

---

### Tokio (Async Runtime)

**Status:** ✅ Available via crates.io
**Version:** Latest stable (1.x)
**Provides:**
- Async runtime (multi-threaded or current-thread)
- tokio::broadcast for EventBus
- tokio::sync primitives (RwLock, Mutex, channel, etc.)
- tokio::time for intervals

**Required By:**
- Foundation (Phase 1) - Event system, shared state
- Core Playback (Phase 4) - Async orchestration
- API Layer (Phase 6) - Axum requires tokio
- All async operations throughout

**Risk:** Low - Established, well-documented

---

### Axum (HTTP Framework)

**Status:** ✅ Available via crates.io
**Version:** Latest stable (0.7.x)
**Provides:**
- HTTP server
- Routing and middleware
- SSE support (axum::response::sse)
- JSON serialization integration
- Request/response handling

**Required By:**
- API Layer (Phase 6) - All HTTP/SSE endpoints

**Risk:** Low - Tokio-native, well-maintained

---

### Symphonia (Audio Decoding)

**Status:** ✅ Available via crates.io
**Version:** Latest stable (0.5.x)
**Provides:**
- Format detection and demuxing
- Codec support (MP3, FLAC, AAC, Vorbis, Opus, etc.)
- Seeking capability
- Sample extraction

**Required By:**
- Audio Subsystem (Phase 3) - Decoder integration
- Core Playback (Phase 4) - DecoderChain

**Assumptions:**
- Supports all required formats (MP3, FLAC, AAC, Vorbis, Opus)
- Seeking is sample-accurate
- Error handling is comprehensive

**Risk:** Low - Widely used for Rust audio decoding

---

### Rubato (Sample Rate Conversion)

**Status:** ✅ Available via crates.io
**Version:** Latest stable (0.15.x)
**Provides:**
- StatefulResampler (preserves state across chunks)
- High-quality resampling algorithms
- Flush capability for tail samples

**Required By:**
- Audio Subsystem (Phase 3) - Resampling implementation
- Core Playback (Phase 4) - DecoderChain resampling stage

**Assumptions:** ⚠️ AT-RISK
- StatefulResampler provides adequate state management
- Flush behavior handles tail samples correctly
- Pause/resume preserves state across chunk boundaries

**Fallback Plan:**
- If rubato insufficient, wrap in custom stateful adapter
- Manual tracking of input/output sample counts
- Custom flush logic if needed
- Estimated effort: 1-2 days

**Risk:** Low-Medium - Validated early in Phase 3

---

### cpal (Audio Output)

**Status:** ✅ Available via crates.io
**Version:** Latest stable (0.15.x)
**Provides:**
- Cross-platform audio output
- Device enumeration
- Stream configuration (sample rate, channels, format)
- Lock-free callback-based API

**Required By:**
- Audio Subsystem (Phase 3) - Output device setup
- Core Playback (Phase 4) - Audio callback integration

**Assumptions:**
- Works on Linux (ALSA/PulseAudio), macOS (CoreAudio), Windows (WASAPI)
- Pi Zero 2W audio device is enumerable and functional
- Callback latency is acceptable

**Risk:** Low-Medium - Platform-specific quirks may exist, tested early

---

### SQLx or Rusqlite (Database Access)

**Status:** ✅ Available via crates.io
**Options:**
- rusqlite (synchronous, simpler API)
- sqlx (async, compile-time query checking)

**Provides:**
- SQLite database connection
- Query execution
- Transaction support
- JSON1 extension support

**Required By:**
- Database Layer (Phase 2) - All database operations
- Configuration (Phase 1) - Settings loading

**Decision:** Recommend rusqlite for simplicity (synchronous is adequate for wkmp-ap use case)

**Risk:** Low - Both libraries are mature

---

### Thiserror (Error Handling)

**Status:** ✅ Available via crates.io
**Version:** Latest stable (1.x)
**Provides:**
- Derive macro for Error trait
- Easy error type definitions
- Error chaining

**Required By:**
- Foundation (Phase 1) - Error type definitions per SPEC021

**Risk:** None - Simple derive macro library

---

### Serde (Serialization)

**Status:** ✅ Available via crates.io
**Version:** Latest stable (1.x)
**Provides:**
- JSON serialization/deserialization
- Derive macros for Serialize/Deserialize

**Required By:**
- API Layer (Phase 6) - Request/response JSON
- Event System (Phase 6) - SSE event JSON
- Configuration (Phase 1) - TOML parsing (serde_json, toml crate)

**Risk:** None - Standard Rust serialization

---

## Hardware Dependencies

### Raspberry Pi Zero 2W (Deployment Target)

**Status:** ✅ Available for testing
**Specifications:**
- CPU: ARM Cortex-A53 (4 cores, 1GHz)
- RAM: 512MB
- Audio: PWM audio via GPIO or USB audio adapter

**Required By:**
- Performance Optimization (Phase 8) - Validation testing

**Constraints:**
- CPU <40% during playback (SPEC022)
- Memory <150MB RSS (SPEC022)

**Risk:** Medium - Performance targets may be challenging on limited hardware

---

### Audio Output Device

**Status:** ✅ Assumed available on all platforms
**Types:**
- Development: Built-in audio or USB
- Pi Zero 2W: PWM GPIO audio or USB audio adapter

**Required By:**
- Audio Subsystem (Phase 3) - Initial testing
- All subsequent phases - Continuous validation

**Risk:** Low - Fallback to default device if specific device unavailable

---

## Dependency Graph by Phase

```
Phase 1 (Foundation)
├── Tokio ✅
├── Thiserror ✅
├── Rusqlite/SQLx ✅
├── Serde (TOML) ✅
└── wkmp-common ✅

Phase 2 (Database Layer)
├── Phase 1 (Foundation) ✅
├── Rusqlite/SQLx ✅
├── wkmp-common (entities) ✅
└── IMPL001 schema ✅

Phase 3 (Audio Subsystem)
├── Phase 1 (Foundation) ✅
├── Symphonia ✅
├── Rubato ⚠️ (AT-RISK - validate early)
├── cpal ✅
└── Audio device (hardware) ✅

Phase 4 (Core Playback)
├── Phase 1 (Foundation) ✅
├── Phase 2 (Database Layer) ✅
├── Phase 3 (Audio Subsystem) ✅
├── Tokio ✅
└── wkmp-common (events) ✅

Phase 5 (Crossfade Mixer)
├── Phase 4 (Core Playback) ✅
└── (No new dependencies)

Phase 6 (API Layer)
├── Phase 5 (Crossfade Mixer) ✅
├── Axum ✅
├── Tokio ✅
├── Serde (JSON) ✅
└── wkmp-common (events for SSE) ✅

Phase 7 (Error Handling)
├── Phase 6 (API Layer) ✅
└── (No new dependencies - refactoring)

Phase 8 (Performance)
├── Phase 7 (Error Handling) ✅
├── Pi Zero 2W hardware ✅
└── Profiling tools (flamegraph, perf) ✅
```

---

## Missing or Unknown Dependencies

**None identified at this stage.**

All required dependencies are:
- Available in crates.io
- Established and stable
- Well-documented
- Proven in production Rust applications

**AT-RISK Items:**
1. **Rubato state management** - Validated in Phase 3, fallback wrapper plan ready

---

## Dependency Validation Plan

### Phase 1 (Foundation)
- ✅ Verify tokio version compatibility
- ✅ Verify thiserror derive macros work as expected
- ✅ Verify rusqlite/sqlx connects to database

### Phase 3 (Audio Subsystem) - CRITICAL VALIDATION
- ⚠️ **Rubato validation (AT-RISK):**
  - Test StatefulResampler flush behavior
  - Test pause/resume state preservation
  - Test tail sample handling at passage boundaries
  - If inadequate: Implement wrapper (1-2 days)
- ✅ Verify symphonia decodes all required formats
- ✅ Verify cpal enumerates audio device on all platforms
- ✅ Measure decode latency vs. SPEC022 target (<500ms)

### Phase 8 (Performance)
- ✅ Verify CPU <40% on Pi Zero 2W
- ✅ Verify memory <150MB RSS
- ✅ Verify no memory leaks (24-hour test)

---

## Dependency Risk Summary

| Dependency | Status | Risk Level | Mitigation |
|------------|--------|------------|------------|
| wkmp-common | Exists | Low | None needed |
| Database (IMPL001) | Exists | Low | None needed |
| Tokio | Available | Low | None needed |
| Axum | Available | Low | None needed |
| Symphonia | Available | Low | Format testing |
| **Rubato** | **Available** | **Medium** | **Early validation, wrapper fallback** |
| cpal | Available | Low-Medium | Platform testing |
| Rusqlite/SQLx | Available | Low | None needed |
| Thiserror | Available | None | None needed |
| Serde | Available | None | None needed |
| Pi Zero 2W | Available | Medium | Early performance testing |

**Overall Risk:** Low - One AT-RISK dependency (rubato) with clear fallback plan

---

**Status:** Phase 1 - Dependencies mapped
**Last Updated:** 2025-10-26
