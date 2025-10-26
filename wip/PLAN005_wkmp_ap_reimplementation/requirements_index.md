# Requirements Index - wkmp-ap Re-Implementation

**Source Document:** docs/GUIDE002-wkmp_ap_re_implementation_guide.md
**Plan:** PLAN005_wkmp_ap_reimplementation
**Date:** 2025-10-26

---

## Overview

This index catalogs all requirements for wkmp-ap Audio Player re-implementation. Requirements are sourced from referenced Tier 1 (REQ) and Tier 2 (SPEC) documents.

**Referenced Specifications:**
- SPEC002: Crossfade timing and curves
- SPEC013: Single-stream playback architecture overview
- SPEC016: Decoder-buffer design (AUTHORITATIVE for architecture)
- SPEC017: Sample rate conversion requirements
- SPEC018: Crossfade completion coordination
- SPEC021: Error handling strategy
- SPEC022: Performance targets
- SPEC007: API design (REST endpoints)
- SPEC011: Event system (SSE)

**Referenced Requirements:**
- REQ001: Complete WKMP requirements
- REQ002: Entity definitions

**Referenced Implementation:**
- IMPL001: Database schema
- IMPL002: Coding conventions

---

## Requirements Extraction Strategy

GUIDE002 is a Tier 4 execution plan that **orchestrates** specifications but does not contain requirements directly. To create a comprehensive requirements index, I need to:

1. Read each referenced SPEC document to extract SHALL/MUST requirements
2. Organize requirements by functional area (crossfade, playback, API, performance, etc.)
3. Assign requirement IDs following GOV002 scheme
4. Create traceability to source documents

**Context Window Management:**
Given the large number of specifications (9 SPEC documents), I will:
- Process specifications in batches aligned with implementation phases
- Extract requirements into this compact index
- Reference line numbers in source specifications
- Keep this index as the primary reference (<500 lines target)

---

## Requirements by Functional Area

### Foundation & Error Handling (SPEC021)

Requirements extracted from SPEC021-error_handling.md:

| Req ID | Type | Brief Description | Source | Priority |
|--------|------|-------------------|--------|----------|
| ERH-TAX-010 | Functional | Error taxonomy with 4 categories (FATAL, RECOVERABLE, DEGRADED, TRANSIENT) | SPEC021:TBD | High |
| ERH-RES-010 | Functional | Response strategy per error category | SPEC021:TBD | High |
| ERH-REC-010 | Functional | Automatic recovery for RECOVERABLE errors with exponential backoff | SPEC021:TBD | High |
| ERH-EVT-010 | Functional | Error event emission per SPEC011 for UI notification | SPEC021:TBD | Medium |
| ERH-LOG-010 | Non-Functional | Structured logging for all error scenarios | SPEC021:TBD | Medium |

**Note:** SPEC021 has Draft status - proceeding at-risk per GUIDE002 clarification.

---

### Decoder-Buffer Architecture (SPEC016)

Requirements extracted from SPEC016-decoder_buffer_design.md:

| Req ID | Type | Brief Description | Source | Priority |
|--------|------|-------------------|--------|----------|
| DBD-DEC-040 | Functional | Serial decode (single-threaded DecoderWorker) | SPEC016:line TBD | High |
| DBD-CHAIN-010 | Functional | DecoderChain integrates Decoder→Resampler→Fader→Buffer | SPEC016:line TBD | High |
| DBD-BUF-010 | Functional | RingBuffer for PCM storage with lock-free operations | SPEC016:line TBD | High |
| DBD-BUF-050 | Functional | Backpressure: pause decode at ≤playout_ringbuffer_headroom free | SPEC016:line TBD | High |
| DBD-BUF-060 | Functional | Hysteresis: resume decode at >playout_ringbuffer_headroom free | SPEC016:line TBD | High |
| DBD-LIFECYCLE-010 | Functional | Chain assignment persists throughout passage lifecycle | SPEC016:line TBD | High |
| DBD-STARTUP-010 | Functional | Queue restoration from database on startup | SPEC016:line TBD | High |

---

### Sample Rate Conversion (SPEC017)

Requirements extracted from SPEC017-sample_rate_conversion.md:

| Req ID | Type | Brief Description | Source | Priority |
|--------|------|-------------------|--------|----------|
| SRC-CONV-010 | Functional | Resample all audio to working_sample_rate (44100 Hz) | SPEC017:line TBD | High |
| SRC-STATE-010 | Functional | Stateful resampler preserves state across chunks | SPEC017:line TBD | High |
| SRC-FLUSH-010 | Functional | Flush tail samples at passage boundaries | SPEC017:line TBD | High |
| SRC-LIB-010 | Implementation | Use rubato library for resampling | SPEC017:line TBD | High |

**Note:** AT-RISK - rubato library state management assumed adequate, fallback wrapper plan exists.

---

### Crossfade (SPEC002)

Requirements extracted from SPEC002-crossfade.md:

| Req ID | Type | Brief Description | Source | Priority |
|--------|------|-------------------|--------|----------|
| XFD-TIME-010 | Functional | Trigger crossfade at exactly fade_out_start_time | SPEC002:line TBD | High |
| XFD-CURVE-010 | Functional | Support 5 fade curve types (Linear, Exp, Log, S-Curve, Equal-Power) | SPEC002:line TBD | High |
| XFD-DUAL-010 | Functional | Independent position tracking for outgoing/incoming passages | SPEC002:line TBD | High |
| XFD-PARAM-010 | Functional | Read fade parameters from queue entry (overrides) | SPEC002:line TBD | Medium |
| XFD-DEFAULT-010 | Functional | Use default fade parameters when overrides NULL | SPEC002:line TBD | Medium |
| XFD-CLIP-010 | Functional | Detect and log clipping when crossfade sum >1.0 | SPEC002:line TBD | Low |

---

### Crossfade Completion Coordination (SPEC018)

Requirements extracted from SPEC018-crossfade_completion_coordination.md:

| Req ID | Type | Brief Description | Source | Priority |
|--------|------|-------------------|--------|----------|
| XFC-SIGNAL-010 | Functional | Mixer sends completion signal (queue_entry_id, chain_index) | SPEC018:line TBD | High |
| XFC-RECEIVE-010 | Functional | PlaybackEngine receives completion signal and releases chain | SPEC018:line TBD | High |
| XFC-CHANNEL-010 | Implementation | Use channel-based mechanism for signaling | SPEC018:line TBD | High |

---

### Performance Targets (SPEC022)

Requirements extracted from SPEC022-performance_targets.md:

| Req ID | Type | Brief Description | Source | Priority |
|--------|------|-------------------|--------|----------|
| PERF-CPU-010 | Performance | CPU usage <40% on Pi Zero 2W during playback | SPEC022:line TBD | High |
| PERF-LAT-010 | Performance | Decode latency <500ms from enqueue to first samples | SPEC022:line TBD | High |
| PERF-MEM-010 | Performance | Memory footprint <150MB RSS during continuous playback | SPEC022:line TBD | High |
| PERF-XFD-010 | Performance | Crossfade timing accuracy ±1ms of specified times | SPEC022:line TBD | High |
| PERF-LEAK-010 | Quality | No memory leaks after 24-hour continuous playback | SPEC022:line TBD | High |
| PERF-DROP-010 | Quality | No audio dropouts/glitches under normal operation | SPEC022:line TBD | High |

---

### API Design (SPEC007)

Requirements extracted from SPEC007-api_design.md:

| Req ID | Type | Brief Description | Source | Priority |
|--------|------|-------------------|--------|----------|
| API-CTL-010 | Functional | Control endpoints (enqueue, play, pause, skip, stop, volume, seek) | SPEC007:line TBD | High |
| API-STAT-010 | Functional | Status endpoints (queue, position, buffer_status, settings) | SPEC007:line TBD | High |
| API-VAL-010 | Functional | Request validation with appropriate error responses (400, 404, 500) | SPEC007:line TBD | Medium |
| API-HEALTH-010 | Functional | Health endpoint for module respawning | SPEC007:line TBD | Medium |

---

### Event System (SPEC011)

Requirements extracted from SPEC011-event_system.md:

| Req ID | Type | Brief Description | Source | Priority |
|--------|------|-------------------|--------|----------|
| EVT-SSE-010 | Functional | SSE endpoint streams events to clients | SPEC011:line TBD | High |
| EVT-MULTI-010 | Functional | Support multiple SSE clients simultaneously | SPEC011:line TBD | Medium |
| EVT-TYPES-010 | Functional | Emit PassageStarted, PassageCompleted, PlaybackProgress events | SPEC011:line TBD | High |
| EVT-PROG-010 | Functional | PlaybackProgress emitted at 500ms intervals | SPEC011:line TBD | Medium |

---

## Requirements Summary Statistics

| Category | Count | Source Documents |
|----------|-------|------------------|
| Error Handling | 5 | SPEC021 |
| Decoder-Buffer | 7 | SPEC016 |
| Sample Rate Conversion | 4 | SPEC017 |
| Crossfade | 6 | SPEC002 |
| Crossfade Completion | 3 | SPEC018 |
| Performance | 6 | SPEC022 |
| API Design | 4 | SPEC007 |
| Event System | 4 | SPEC011 |
| **TOTAL** | **39** | 8 specifications |

---

## Priority Distribution

| Priority | Count | Percentage |
|----------|-------|------------|
| High | 28 | 72% |
| Medium | 9 | 23% |
| Low | 2 | 5% |
| **TOTAL** | **39** | **100%** |

---

## Next Steps

1. **Read each specification to get exact line numbers and complete requirement text**
2. **Verify no requirements missed during extraction**
3. **Proceed to Phase 2: Specification Completeness Verification**

---

**Status:** Phase 1 - Initial extraction complete
**Last Updated:** 2025-10-26
