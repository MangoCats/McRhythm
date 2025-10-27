# Dependencies Map - Buffer Auto-Tuning

## Internal Dependencies (WKMP Codebase)

### Existing Code (Reuse)

| Component | Location | Status | Purpose | Notes |
|-----------|----------|--------|---------|-------|
| Ring Buffer | wkmp-ap/src/playback/ring_buffer.rs | ✓ Exists | Underrun detection and monitoring | Has underrun_count() method |
| Audio Output | wkmp-ap/src/audio/output.rs | ✓ Exists | cpal-based audio playback | Initialize with buffer_size parameter |
| Database Settings | wkmp-ap/src/db/settings.rs | ✓ Exists | load/save mixer and buffer parameters | Recently added load_clamped_setting() helper |
| Decoder | wkmp-ap/src/audio/decoder.rs | ✓ Exists | Decode audio files for test playback | symphonia-based |
| Resampler | wkmp-ap/src/audio/resampler.rs | ✓ Exists | Resample to target sample rate | rubato-based |
| Error Types | wkmp-ap/src/error.rs | ✓ Exists | Result<T> and Error enum | Extend for tuning-specific errors |
| Callback Monitor | wkmp-ap/src/playback/callback_monitor.rs | ✓ Exists | Track callback timing jitter | Has timing statistics methods |

### New Code (Implement)

| Component | Location | Purpose | Estimated LOC |
|-----------|----------|---------|---------------|
| Main Binary | wkmp-ap/src/bin/tune_buffers.rs | Orchestrate tuning workflow | ~400 |
| Test Harness | wkmp-ap/src/tuning/test_harness.rs | Simplified playback for testing | ~300 |
| Metrics Collector | wkmp-ap/src/tuning/metrics.rs | Gather underrun/timing data | ~200 |
| Search Algorithm | wkmp-ap/src/tuning/search.rs | Binary search implementation | ~250 |
| Curve Fitting | wkmp-ap/src/tuning/curve.rs | Analyze results, find optimal | ~150 |
| Report Generator | wkmp-ap/src/tuning/report.rs | Format CLI output and JSON | ~200 |
| System Info | wkmp-ap/src/tuning/system_info.rs | Detect CPU, OS, audio hardware | ~100 |

**Total New Code:** ~1,600 LOC

## External Dependencies (Cargo)

### Already Available

| Crate | Version | Purpose | Status |
|-------|---------|---------|--------|
| tokio | 1.x | Async runtime for test harness | ✓ In Cargo.toml |
| sqlx | 0.7.x | Database access for settings | ✓ In Cargo.toml |
| serde | 1.x | Serialization framework | ✓ In Cargo.toml |
| serde_json | 1.x | JSON export of results | ✓ In Cargo.toml |
| cpal | 0.15.x | Audio output via system APIs | ✓ In Cargo.toml |
| symphonia | 0.5.x | Audio file decoding | ✓ In Cargo.toml |
| rubato | 0.14.x | Sample rate conversion | ✓ In Cargo.toml |
| tracing | 0.1.x | Logging and diagnostics | ✓ In Cargo.toml |

**No new external dependencies required** ✓

### Standard Library

| Component | Purpose |
|-----------|---------|
| std::time | Timing test duration, measuring jitter |
| std::sync | Arc, Mutex for shared state |
| std::fs | Backup settings to temp file |
| std::env | Detect system information (OS, arch) |

## Referenced Specifications

### WKMP Documentation

| Document | Section | Purpose | Status |
|----------|---------|---------|--------|
| SPEC016 | Lines 282-295 | [DBD-PARAM-111] mixer_check_interval_ms definition | ✓ Exists |
| SPEC016 | Lines 260-276 | [DBD-PARAM-110] audio_buffer_size definition | ✓ Exists |
| SPEC016 | General | Ring buffer underrun monitoring | ✓ Exists |
| SPEC016 | Lines 244-266 | Mixer thread configuration | ✓ Exists |
| IMPL001 | General | Database schema (settings table) | ✓ Exists |
| GOV002 | General | Requirements enumeration format | ✓ Exists |

### Related Implementations

| Feature | Location | Relevance |
|---------|----------|-----------|
| Ring buffer grace period | settings.rs:load_ring_buffer_grace_period() | Similar monitoring concept |
| Mixer thread config | settings.rs:load_mixer_thread_config() | Parameters being tuned |
| Audio buffer size | settings.rs:load_audio_buffer_size() | Parameter being tuned |
| Callback monitoring | playback/callback_monitor.rs | Jitter measurement reuse |

## Hardware Dependencies

### Required Hardware

| Component | Requirement | Validation Method |
|-----------|-------------|-------------------|
| Audio Output Device | At least one working device | Detect via cpal enumerate |
| CPU | Real-time audio capable | Test during tuning run |
| RAM | Sufficient for 8192-frame buffers | ~150KB per buffer |

### Platform Support

| Platform | Audio API | Status | Notes |
|----------|-----------|--------|-------|
| Linux | ALSA | ✓ Primary target | Development platform |
| Linux | PulseAudio | ✓ Via cpal | Common desktop |
| Linux | JACK | ⚠ Possible | May need testing |
| Windows | WASAPI | ⚠ Future | Not primary target |
| macOS | CoreAudio | ⚠ Future | Not primary target |

## Data Dependencies

### Input Data

| Data | Source | Required? | Fallback |
|------|--------|-----------|----------|
| Test audio passage | Database (passages table) | No | Generate 440 Hz sine wave |
| Current parameter values | Database (settings table) | Yes | Use defaults if missing |
| Audio device name | Database (audio_sink setting) | No | Use system default |

### Output Data

| Data | Destination | Format |
|------|-------------|--------|
| Recommended values | Database (settings table) | Native types (u64, u32) |
| Tuning results | JSON file (user-specified) | Structured JSON |
| Progress feedback | Console (stdout) | Plain text |
| Errors/warnings | Console (stderr) | Plain text |

## Dependency Graph

```
tune_buffers binary
  ├─ system_info (detect CPU, OS, audio device)
  │   └─ std::env
  │
  ├─ test_harness (simplified playback)
  │   ├─ decoder (symphonia)
  │   ├─ resampler (rubato)
  │   ├─ ring_buffer (existing)
  │   └─ audio_output (cpal)
  │
  ├─ metrics_collector (gather data)
  │   ├─ ring_buffer.underrun_count()
  │   ├─ callback_monitor (existing)
  │   └─ std::time
  │
  ├─ search_algorithm (find optimal)
  │   ├─ test_harness
  │   ├─ metrics_collector
  │   └─ binary_search logic
  │
  ├─ curve_fitting (analyze results)
  │   └─ search_algorithm results
  │
  ├─ report_generator (format output)
  │   ├─ curve_fitting results
  │   └─ serde_json (export)
  │
  └─ database settings (save results)
      └─ sqlx
```

## Critical Dependencies

### Must Work Correctly

1. **Ring buffer underrun detection**
   - Status: ✓ Implemented in ring_buffer.rs
   - Risk: If underrun detection is inaccurate, tuning will fail
   - Mitigation: Validate underrun detection in unit tests

2. **Database settings persistence**
   - Status: ✓ load_clamped_setting() recently added
   - Risk: Settings not persisting would break tuning
   - Mitigation: Test backup/restore in integration tests

3. **Audio output with configurable buffer**
   - Status: ✓ cpal supports buffer_size parameter
   - Risk: Some devices may not honor requested size
   - Mitigation: Verify actual buffer size matches requested

4. **Callback timing measurement**
   - Status: ✓ callback_monitor.rs tracks jitter
   - Risk: Inaccurate timing data
   - Mitigation: Validate timing measurements

### Nice to Have (Non-Blocking)

1. **CPU usage measurement**
   - Status: ⚠ Platform-dependent APIs
   - Risk: May not be available on all platforms
   - Mitigation: Make optional, skip if unavailable

2. **Test audio from database**
   - Status: ⚠ Depends on passages being imported
   - Risk: Database may be empty
   - Mitigation: Fallback to generated test tone

## Dependency Status Summary

**Internal Dependencies:**
- Existing code: 7 components ✓ All available
- New code: 7 components ⚠ To be implemented

**External Dependencies:**
- Cargo crates: 8 required ✓ All available
- Standard library: 4 modules ✓ All available

**Hardware Dependencies:**
- Audio device: 1 required ⚠ User responsibility
- CPU/RAM: Adequate ⚠ Validated during tuning

**Documentation Dependencies:**
- Specifications: 2 documents ✓ Both exist
- Related code: 4 references ✓ All documented

**Overall Readiness:** ✓ All critical dependencies available
