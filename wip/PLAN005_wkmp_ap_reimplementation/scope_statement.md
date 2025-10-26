# Scope Statement - wkmp-ap Re-Implementation

**Plan:** PLAN005_wkmp_ap_reimplementation
**Source:** docs/GUIDE002-wkmp_ap_re_implementation_guide.md
**Date:** 2025-10-26

---

## In Scope

**Complete wkmp-ap Audio Player microservice re-implementation including:**

1. **Foundation Components**
   - Error handling framework (SPEC021 taxonomy)
   - Configuration loading (database settings + TOML bootstrap)
   - Event system integration (wkmp-common EventBus)
   - Logging infrastructure
   - Shared playback state (Arc<RwLock>)

2. **Database Layer**
   - Queue persistence and restoration
   - Passage metadata access
   - Settings management
   - Play order management with automatic renumbering

3. **Audio Subsystem**
   - Symphonia decoder integration (MP3, FLAC, AAC, Vorbis, Opus)
   - Rubato sample rate conversion (StatefulResampler)
   - cpal audio output (cross-platform)
   - Ring buffer implementation (lock-free, PCM storage)

4. **Core Playback Engine**
   - DecoderChain (Decoder→Resampler→Fader→Buffer pipeline)
   - DecoderWorker (single-threaded serial decoder per SPEC016)
   - PlaybackEngine (queue orchestration, chain lifecycle)
   - Position tracking (sample-accurate)
   - Buffer backpressure with hysteresis-based pause/resume

5. **Crossfade Mixer**
   - 5 fade curve types (Linear, Exponential, Logarithmic, S-Curve, Equal-Power)
   - Sample-accurate triggering (fade_out_start_time)
   - Dual buffer mixing (independent position tracking)
   - Fade parameter extraction from queue entries
   - Completion signaling (SPEC018 channel-based mechanism)
   - Clipping detection and logging

6. **API Layer**
   - HTTP REST endpoints (enqueue, play, pause, skip, stop, volume, seek)
   - Status endpoints (queue, position, buffer_status, settings)
   - Request validation with error responses
   - Health endpoint for module respawning
   - Axum web framework integration

7. **Event System**
   - Server-Sent Events (SSE) broadcaster
   - Event types: PassageStarted, PassageCompleted, PlaybackProgress
   - Multi-client support
   - EventBus subscription and forwarding

8. **Error Handling & Recovery**
   - 4-category error taxonomy (FATAL, RECOVERABLE, DEGRADED, TRANSIENT)
   - Category-specific response strategies
   - Automatic recovery with exponential backoff
   - Error event emission for UI notification
   - Comprehensive error scenario testing

9. **Performance Optimization**
   - CPU usage <40% on Pi Zero 2W
   - Decode latency <500ms
   - Memory footprint <150MB RSS
   - Crossfade timing accuracy ±1ms
   - No memory leaks (24-hour validation)
   - No audio dropouts/glitches

---

## Out of Scope

**Explicitly excluded from this re-implementation:**

1. **Other Microservices**
   - wkmp-ui (User Interface microservice)
   - wkmp-pd (Program Director microservice)
   - wkmp-ai (Audio Ingest microservice)
   - wkmp-le (Lyric Editor microservice)

2. **Library Modifications**
   - wkmp-common library changes (use existing Event types, entities)
   - External library modifications (symphonia, rubato, cpal, axum, tokio)

3. **Schema Changes**
   - Database schema modifications (work with IMPL001 as-is)
   - New tables or columns not in existing schema

4. **New Features**
   - Feature development beyond current specifications
   - UI enhancements
   - New audio formats beyond symphonia support
   - New fade curve types beyond specified 5

5. **Infrastructure**
   - Database migration system (use existing migrations/)
   - Build system changes (use existing Cargo workspace)
   - Deployment automation (use existing scripts/)

---

## Assumptions

1. **Database:**
   - SQLite database exists with schema per IMPL001
   - All required tables present (queue, passages, settings, module_config, etc.)
   - JSON1 extension available for musical flavor storage

2. **Libraries:**
   - wkmp-common provides shared types (Event, entities, database models)
   - symphonia supports required formats (MP3, FLAC, AAC, Vorbis, Opus)
   - rubato provides adequate StatefulResampler functionality (AT-RISK - fallback wrapper plan exists)
   - cpal provides cross-platform audio output
   - axum provides HTTP/SSE functionality
   - tokio provides async runtime

3. **Environment:**
   - Deployment target is Pi Zero 2W (ARM Cortex-A53, 512MB RAM)
   - Development systems are Linux/macOS/Windows
   - Audio files are accessible via filesystem paths
   - Audio output device available and enumerable

4. **Configuration:**
   - TOML file provides bootstrap configuration (database path, logging)
   - Database settings table provides runtime configuration
   - Default values available for all settings

5. **Specifications:**
   - SPEC016 is authoritative for architecture (SPEC014 outdated)
   - SPEC021 (Draft status) will not change significantly (AT-RISK - proceeding)
   - All referenced specifications are complete and consistent

---

## Constraints

1. **Performance:**
   - MUST meet SPEC022 targets (CPU <40%, latency <500ms, memory <150MB)
   - MUST work on Pi Zero 2W (limited CPU/memory)
   - MUST achieve sample-accurate crossfading (±1ms tolerance)

2. **Compatibility:**
   - MUST maintain compatibility with wkmp-common Event types
   - MUST work with existing IMPL001 database schema
   - CANNOT break existing API contracts (SPEC007)

3. **Code Quality:**
   - MUST follow IMPL002 Rust coding conventions
   - MUST pass clippy without warnings
   - MUST achieve >80% unit test coverage for core components
   - MUST document all public APIs with rustdoc

4. **Architecture:**
   - MUST use SPEC016 serial decode design (single-threaded DecoderWorker)
   - MUST use SPEC018 channel-based completion signaling
   - CANNOT use SPEC014 parallel decoder pool (outdated)

5. **Platform:**
   - MUST work on both development systems and Pi Zero 2W
   - MUST use cross-platform libraries only
   - CANNOT rely on platform-specific APIs without fallbacks

---

## Boundaries and Interfaces

**Upstream Dependencies (What we consume):**
- wkmp-common library (Event types, entities, utilities)
- SQLite database (queue, passages, settings per IMPL001)
- Audio files (filesystem paths)
- Configuration (TOML bootstrap, database settings)

**Downstream Consumers (What consumes us):**
- wkmp-ui microservice (HTTP API client, SSE event consumer)
- wkmp-pd microservice (HTTP API client for queue management)
- User browsers (SSE event stream consumers)
- System administrators (HTTP health endpoint monitoring)

**External Libraries:**
- symphonia (audio decoding)
- rubato (sample rate conversion)
- cpal (audio output)
- axum (HTTP/SSE server)
- tokio (async runtime)
- sqlx or rusqlite (database access)
- thiserror (error handling)

**Not Interfacing With:**
- wkmp-ai, wkmp-le (Full version only, on-demand)
- MusicBrainz/AcousticBrainz APIs (wkmp-ai responsibility)
- User authentication (wkmp-ui responsibility)

---

## Success Criteria Summary

Implementation is complete when:

1. ✅ All 39 requirements satisfied (per requirements_index.md)
2. ✅ All 8 phases completed with acceptance criteria met
3. ✅ Unit test coverage >80% for core components
4. ✅ Integration tests pass for all major workflows
5. ✅ Performance targets met on Pi Zero 2W (SPEC022)
6. ✅ 24-hour continuous playback test passes (no crashes, no leaks)
7. ✅ Sample-accurate crossfade verification passes
8. ✅ No compiler warnings (clippy clean)
9. ✅ All public APIs documented with rustdoc
10. ✅ Manual testing complete on development system and Pi Zero 2W

---

**Status:** Phase 1 - Scope definition complete
**Last Updated:** 2025-10-26
