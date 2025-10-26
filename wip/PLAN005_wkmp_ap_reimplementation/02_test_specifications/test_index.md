# Test Index - wkmp-ap Re-Implementation

**Plan:** PLAN005_wkmp_ap_reimplementation
**Date:** 2025-10-26
**Total Tests:** 73 (estimated)

---

## Overview

This index provides quick reference to all acceptance tests for wkmp-ap re-implementation. Tests are organized by functional area matching requirements_index.md categories.

**Test Type Distribution:**
- Unit Tests: ~40
- Integration Tests: ~25
- System Tests: ~8

**Coverage Target:** 100% of 39 requirements per traceability matrix

---

## Test Index by Functional Area

### Foundation & Error Handling (SPEC021)

| Test ID | Requirement | Type | Description | File |
|---------|-------------|------|-------------|------|
| TC-U-ERH-010-01 | ERH-TAX-010 | Unit | Error taxonomy enum definitions | tc_u_erh_010_01.md |
| TC-U-ERH-RES-01 | ERH-RES-010 | Unit | Response strategy per category | tc_u_erh_res_01.md |
| TC-I-ERH-REC-01 | ERH-REC-010 | Integration | Automatic recovery with exponential backoff | tc_i_erh_rec_01.md |
| TC-I-ERH-EVT-01 | ERH-EVT-010 | Integration | Error event emission for UI notification | tc_i_erh_evt_01.md |
| TC-U-ERH-LOG-01 | ERH-LOG-010 | Unit | Structured logging for errors | tc_u_erh_log_01.md |

---

### Decoder-Buffer Architecture (SPEC016)

| Test ID | Requirement | Type | Description | File |
|---------|-------------|------|-------------|------|
| TC-I-DBD-DEC-01 | DBD-DEC-040 | Integration | Serial decode (single-threaded DecoderWorker) | tc_i_dbd_dec_01.md |
| TC-I-DBD-CHAIN-01 | DBD-CHAIN-010 | Integration | DecoderChain pipeline integration | tc_i_dbd_chain_01.md |
| TC-U-DBD-BUF-01 | DBD-BUF-010 | Unit | RingBuffer lock-free operations | tc_u_dbd_buf_01.md |
| TC-I-DBD-BUF-02 | DBD-BUF-050 | Integration | Backpressure pause at threshold | tc_i_dbd_buf_02.md |
| TC-I-DBD-BUF-03 | DBD-BUF-060 | Integration | Hysteresis resume after threshold | tc_i_dbd_buf_03.md |
| TC-I-DBD-LIFECYCLE-01 | DBD-LIFECYCLE-010 | Integration | Chain assignment persists throughout lifecycle | tc_i_dbd_lifecycle_01.md |
| TC-I-DBD-STARTUP-01 | DBD-STARTUP-010 | Integration | Queue restoration from database on startup | tc_i_dbd_startup_01.md |

---

### Sample Rate Conversion (SPEC017)

| Test ID | Requirement | Type | Description | File |
|---------|-------------|------|-------------|------|
| TC-I-SRC-CONV-01 | SRC-CONV-010 | Integration | Resample to working_sample_rate (44100 Hz) | tc_i_src_conv_01.md |
| TC-U-SRC-STATE-01 | SRC-STATE-010 | Unit | Stateful resampler preserves state across chunks | tc_u_src_state_01.md |
| TC-U-SRC-FLUSH-01 | SRC-FLUSH-010 | Unit | Flush tail samples at passage boundaries | tc_u_src_flush_01.md |
| TC-U-SRC-LIB-01 | SRC-LIB-010 | Unit | rubato library integration | tc_u_src_lib_01.md |

---

### Crossfade (SPEC002)

| Test ID | Requirement | Type | Description | File |
|---------|-------------|------|-------------|------|
| TC-S-XFD-TIME-01 | XFD-TIME-010 | System | Trigger crossfade at exactly fade_out_start_time | tc_s_xfd_time_01.md |
| TC-U-XFD-CURVE-01 | XFD-CURVE-010 | Unit | 5 fade curve types (Linear, Exp, Log, S-Curve, Equal-Power) | tc_u_xfd_curve_01.md |
| TC-I-XFD-DUAL-01 | XFD-DUAL-010 | Integration | Independent position tracking for outgoing/incoming | tc_i_xfd_dual_01.md |
| TC-I-XFD-PARAM-01 | XFD-PARAM-010 | Integration | Read fade parameters from queue entry | tc_i_xfd_param_01.md |
| TC-I-XFD-DEFAULT-01 | XFD-DEFAULT-010 | Integration | Use defaults when overrides NULL | tc_i_xfd_default_01.md |
| TC-U-XFD-CLIP-01 | XFD-CLIP-010 | Unit | Detect and log clipping when sum >1.0 | tc_u_xfd_clip_01.md |

---

### Crossfade Completion Coordination (SPEC018)

| Test ID | Requirement | Type | Description | File |
|---------|-------------|------|-------------|------|
| TC-I-XFC-SIGNAL-01 | XFC-SIGNAL-010 | Integration | Mixer sends completion signal (queue_entry_id, chain_index) | tc_i_xfc_signal_01.md |
| TC-I-XFC-RECEIVE-01 | XFC-RECEIVE-010 | Integration | PlaybackEngine receives signal and releases chain | tc_i_xfc_receive_01.md |
| TC-I-XFC-CHANNEL-01 | XFC-CHANNEL-010 | Integration | Channel-based signaling mechanism | tc_i_xfc_channel_01.md |

---

### Performance Targets (SPEC022)

| Test ID | Requirement | Type | Description | File |
|---------|-------------|------|-------------|------|
| TC-S-PERF-CPU-01 | PERF-CPU-010 | System | CPU usage <40% on Pi Zero 2W during playback | tc_s_perf_cpu_01.md |
| TC-S-PERF-LAT-01 | PERF-LAT-010 | System | Decode latency <500ms from enqueue to first samples | tc_s_perf_lat_01.md |
| TC-S-PERF-MEM-01 | PERF-MEM-010 | System | Memory footprint <150MB RSS during continuous playback | tc_s_perf_mem_01.md |
| TC-S-PERF-XFD-01 | PERF-XFD-010 | System | Crossfade timing accuracy ±1ms | tc_s_perf_xfd_01.md |
| TC-S-PERF-LEAK-01 | PERF-LEAK-010 | System | No memory leaks after 24-hour continuous playback | tc_s_perf_leak_01.md |
| TC-S-PERF-DROP-01 | PERF-DROP-010 | System | No audio dropouts/glitches under normal operation | tc_s_perf_drop_01.md |

---

### API Design (SPEC007)

| Test ID | Requirement | Type | Description | File |
|---------|-------------|------|-------------|------|
| TC-I-API-CTL-01 | API-CTL-010 | Integration | Control endpoints (enqueue, play, pause, skip, stop, volume, seek) | tc_i_api_ctl_01.md |
| TC-I-API-STAT-01 | API-STAT-010 | Integration | Status endpoints (queue, position, buffer_status, settings) | tc_i_api_stat_01.md |
| TC-I-API-VAL-01 | API-VAL-010 | Integration | Request validation with error responses (400, 404, 500) | tc_i_api_val_01.md |
| TC-I-API-HEALTH-01 | API-HEALTH-010 | Integration | Health endpoint for module respawning | tc_i_api_health_01.md |

---

### Event System (SPEC011)

| Test ID | Requirement | Type | Description | File |
|---------|-------------|------|-------------|------|
| TC-I-EVT-SSE-01 | EVT-SSE-010 | Integration | SSE endpoint streams events to clients | tc_i_evt_sse_01.md |
| TC-I-EVT-MULTI-01 | EVT-MULTI-010 | Integration | Support multiple SSE clients simultaneously | tc_i_evt_multi_01.md |
| TC-I-EVT-TYPES-01 | EVT-TYPES-010 | Integration | Emit PassageStarted, PassageCompleted, PlaybackProgress events | tc_i_evt_types_01.md |
| TC-I-EVT-PROG-01 | EVT-PROG-010 | Integration | PlaybackProgress emitted at 500ms intervals | tc_i_evt_prog_01.md |

---

## Test Summary Statistics

| Category | Unit | Integration | System | Total |
|----------|------|-------------|--------|-------|
| Error Handling | 3 | 2 | 0 | 5 |
| Decoder-Buffer | 1 | 6 | 0 | 7 |
| Sample Rate Conversion | 3 | 1 | 0 | 4 |
| Crossfade | 2 | 3 | 1 | 6 |
| Crossfade Completion | 0 | 3 | 0 | 3 |
| Performance | 0 | 0 | 6 | 6 |
| API Design | 0 | 4 | 0 | 4 |
| Event System | 0 | 4 | 0 | 4 |
| **TOTAL** | **9** | **23** | **7** | **39** |

---

## Coverage Verification

**Requirements Coverage:** 39/39 (100%)
**Traceability Matrix:** See traceability_matrix.md

---

**Status:** Phase 3 - Test index complete
**Next:** Individual test specification files
**Last Updated:** 2025-10-26
