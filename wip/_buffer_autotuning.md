# Audio Buffer Self-Tuning Algorithm

**Status:** DRAFT
**Created:** 2025-10-26
**Purpose:** Automatically determine optimal buffer parameter values to eliminate audio gaps

---

## 1. Problem Statement

Audio playback exhibits gaps/stutters when buffer parameters (mixer_check_interval_ms and audio_buffer_size) are misconfigured. Manual tuning is time-consuming and hardware-specific. Current "safe" values may be overly conservative or insufficient for different systems.

**Observed behavior:**
- Ring buffer underruns cause audible gaps during playback
- Parameter effectiveness depends on system capabilities (CPU speed, scheduler behavior, audio hardware)
- Relationship exists between mixer interval and buffer size (faster intervals can work with smaller buffers)
- Current parameters tuned manually for development hardware

**Need:**
- Automated tool to find optimal parameter values for a given system
- Initial use: Establish safe values for current development system
- Future use: Quick tuning for deployment targets (e.g., Raspberry Pi Zero 2W)

---

## 2. Objectives

### Primary Objectives

**[TUNE-OBJ-010]** Automatically determine safe operating values for:
- `mixer_check_interval_ms` [DBD-PARAM-111] (range: 1-100ms, default: 5ms)
- `audio_buffer_size` [DBD-PARAM-110] (range: 64-8192 frames, default: 512)

**[TUNE-OBJ-020]** Characterize the relationship between these parameters:
- Map the "curve" where problems begin for various mixer intervals
- Identify the optimal balance (minimum latency vs. stability)
- Document safe zones and failure zones

**[TUNE-OBJ-030]** Produce actionable results:
- Recommended parameter values for the tested system
- Confidence level in recommendations
- Performance characteristics at recommended values

### Secondary Objectives

**[TUNE-OBJ-040]** Support multiple hardware profiles:
- Save tuning results per system
- Compare results across different hardware
- Detect when current system doesn't match saved profile

**[TUNE-OBJ-050]** Minimize tuning time:
- Complete tuning run in <15 minutes
- Adaptive search strategy (focus on interesting regions)
- Early termination when clear optimal found

---

## 3. Requirements

### 3.1 Detection Requirements

**[TUNE-DET-010]** Detect buffer underruns during test playback
- Monitor ring buffer underrun events
- Count underruns per test duration
- Classify severity (occasional vs. persistent)

**[TUNE-DET-020]** Measure audio output health
- Callback regularity (jitter, missed callbacks)
- Buffer occupancy over time
- CPU usage per parameter combination

**[TUNE-DET-030]** Define failure threshold
- Failure: >1% underruns during sustained playback
- Warning: 0.1-1% underruns (marginal)
- Success: <0.1% underruns (stable)

### 3.2 Search Requirements

**[TUNE-SRC-010]** Explore parameter space systematically
- Test multiple mixer_check_interval_ms values (e.g., 1, 2, 5, 10, 20, 50ms)
- For each interval, find minimum stable audio_buffer_size
- Binary search within buffer size range for efficiency

**[TUNE-SRC-020]** Test each configuration adequately
- Run test for minimum 30 seconds per configuration
- Play actual audio content (not silence)
- Monitor throughout entire test duration

**[TUNE-SRC-030]** Adaptive search strategy
- Start with current defaults
- If stable: Try smaller buffer sizes (reduce latency)
- If unstable: Try larger buffer sizes (increase stability)
- Stop when clear boundary found

**[TUNE-SRC-040]** Safety constraints
- Never test below minimum parameter values (1ms interval, 64 frames buffer)
- Detect and abort if system becomes unresponsive
- Restore original values on completion or abort

### 3.3 Output Requirements

**[TUNE-OUT-010]** Generate tuning report containing:
- Tested parameter combinations and results
- Recommended values with justification
- Characterization curve (interval vs. minimum buffer size)
- System information (CPU, OS, audio hardware)

**[TUNE-OUT-020]** Update database settings (optional)
- Prompt user to accept recommended values
- Apply values to settings table if accepted
- Preserve old values as backup

**[TUNE-OUT-030]** Export results for comparison
- JSON format with structured data
- Timestamp and system identifier
- Allows comparing across hardware profiles

### 3.4 Integration Requirements

**[TUNE-INT-010]** Operate as standalone utility
- Separate binary or subcommand (e.g., `wkmp-ap tune-buffers`)
- Can run without full WKMP stack running
- No dependencies on modules beyond wkmp-ap

**[TUNE-INT-020]** Use existing infrastructure
- Read from same database as wkmp-ap
- Use same audio output code path
- Share ring buffer implementation

**[TUNE-INT-030]** Provide user feedback during tuning
- Progress indicator (N% complete, current test)
- Real-time results as discovered
- Estimated time remaining

---

## 4. Algorithm Design

### 4.1 Overall Strategy

**[TUNE-ALG-010]** Two-phase approach:

**Phase 1: Coarse sweep**
- Test 6-8 mixer interval values across range (1, 2, 5, 10, 20, 50, 100ms)
- For each interval, use current default buffer size (512)
- Quickly identify which intervals are viable at default buffer size

**Phase 2: Fine tuning**
- For each viable interval from Phase 1, find minimum stable buffer size
- Binary search between min (64) and a safe maximum (4096)
- Convergence criterion: Smallest buffer with <0.1% underruns

**[TUNE-ALG-020]** Result synthesis:
- Plot interval vs. minimum buffer size curve
- Identify "knee" of curve (sweet spot)
- Recommend values balancing latency and stability

### 4.2 Test Procedure

**[TUNE-TEST-010]** Per-configuration test:
```
1. Apply parameter values to database/config
2. Initialize audio output with new parameters
3. Start playback of test audio (30s duration)
4. Monitor underrun events continuously
5. Record results (underrun count, CPU usage, buffer stats)
6. Shut down audio output cleanly
7. Wait brief cooldown (2s) before next test
```

**[TUNE-TEST-020]** Test audio selection:
- Use actual music passage from database (if available)
- Fallback: Generate test tone (440 Hz sine wave)
- Must exercise full decode→resample→mix→output pipeline
- 30 seconds minimum, 60 seconds preferred

**[TUNE-TEST-030]** Metrics collection:
- Underrun count (from ring buffer monitoring)
- Callback timing statistics (mean, std dev, max jitter)
- Buffer occupancy (min, max, mean, 10th percentile)
- CPU usage (if available via system APIs)

### 4.3 Search Algorithm

**[TUNE-SEARCH-010]** Binary search for minimum buffer size:
```
Given: mixer_check_interval_ms = X
Find: Minimum audio_buffer_size where underruns < 0.1%

low = 64 (minimum allowed)
high = 4096 (safe maximum)
best_stable = high

while (high - low) > 128:  # Converge to within 128 frames
    mid = (low + high) / 2
    result = test_configuration(interval=X, buffer=mid)

    if result.underrun_rate < 0.1%:
        best_stable = mid
        high = mid  # Try smaller
    else:
        low = mid + 1  # Need larger

return best_stable
```

**[TUNE-SEARCH-020]** Early termination conditions:
- If interval ≥50ms and buffer 64 fails: Skip (interval too slow)
- If interval ≤2ms and buffer 4096 fails: Mark interval as unstable
- If 3 consecutive buffer sizes fail: Stop searching this interval

### 4.4 Curve Fitting

**[TUNE-CURVE-010]** After collecting (interval, min_buffer) pairs:
- Plot points on interval (x-axis) vs. buffer size (y-axis)
- Expected relationship: Smaller intervals need larger buffers
- Identify optimal region: Lowest interval where buffer ≤1024 frames

**[TUNE-CURVE-020]** Recommendation logic:
```
Primary recommendation:
- Target: 256-512 frame buffer (5.8-11.6ms latency @ 44.1kHz)
- Find smallest interval that achieves this buffer size
- Must have <0.05% underruns for safety margin

Fallback recommendation:
- If no interval achieves <512 frames: Use smallest buffer found
- Warn user that system may need hardware upgrade
```

---

## 5. User Interface

### 5.1 Command-Line Interface

**[TUNE-UI-010]** Basic invocation:
```bash
wkmp-ap tune-buffers [options]

Options:
  --quick          Fast tuning (fewer test points, 5 min)
  --thorough       Comprehensive tuning (more test points, 15 min)
  --apply          Automatically apply recommended values
  --export <file>  Export results to JSON file
  --compare <file> Compare with previous tuning results
```

**[TUNE-UI-020]** Interactive mode:
```
Starting buffer auto-tuning...

System: AMD Ryzen 5 5600X, Linux 6.8.0, ALSA
Current: mixer_check_interval_ms=5, audio_buffer_size=512

Phase 1: Testing mixer intervals with default buffer...
[✓] 1ms: FAIL (23% underruns)
[✓] 2ms: FAIL (12% underruns)
[✓] 5ms: OK (0.02% underruns)
[✓] 10ms: OK (0.01% underruns)
[✓] 20ms: OK (0% underruns)
[✓] 50ms: OK (0% underruns)

Phase 2: Finding minimum buffer sizes...
[✓] 5ms interval: Testing buffers 64-4096... → 256 frames
[✓] 10ms interval: Testing buffers 64-4096... → 128 frames
[✓] 20ms interval: Testing buffers 64-4096... → 128 frames
[✓] 50ms interval: Testing buffers 64-4096... → 64 frames

Results:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Recommended: mixer_check_interval_ms = 10
             audio_buffer_size = 128

Rationale: Lowest interval with small buffer
Expected latency: ~2.9ms
Stability: Excellent (0.01% underruns)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Apply these values? [Y/n]
```

### 5.2 Output Format

**[TUNE-OUT-040]** JSON export structure:
```json
{
  "tuning_session": {
    "timestamp": "2025-10-26T22:45:00-04:00",
    "duration_seconds": 342,
    "version": "1.0"
  },
  "system_info": {
    "cpu": "AMD Ryzen 5 5600X",
    "os": "Linux 6.8.0-85-generic",
    "audio_backend": "ALSA",
    "audio_device": "default"
  },
  "test_results": [
    {
      "mixer_check_interval_ms": 5,
      "audio_buffer_size": 512,
      "test_duration_seconds": 30,
      "underrun_count": 6,
      "underrun_rate": 0.02,
      "callback_jitter_ms": {"mean": 11.6, "max": 23.1},
      "buffer_occupancy": {"min": 230, "mean": 380, "max": 512},
      "cpu_percent": 8.2,
      "verdict": "STABLE"
    }
    // ... more results
  ],
  "curve_data": [
    {"interval_ms": 1, "min_stable_buffer": null, "status": "UNSTABLE"},
    {"interval_ms": 2, "min_stable_buffer": null, "status": "UNSTABLE"},
    {"interval_ms": 5, "min_stable_buffer": 256, "status": "STABLE"},
    {"interval_ms": 10, "min_stable_buffer": 128, "status": "STABLE"},
    {"interval_ms": 20, "min_stable_buffer": 128, "status": "STABLE"},
    {"interval_ms": 50, "min_stable_buffer": 64, "status": "STABLE"}
  ],
  "recommendations": {
    "primary": {
      "mixer_check_interval_ms": 10,
      "audio_buffer_size": 128,
      "expected_latency_ms": 2.9,
      "confidence": "HIGH",
      "rationale": "Lowest interval with minimal buffer size"
    },
    "conservative": {
      "mixer_check_interval_ms": 20,
      "audio_buffer_size": 256,
      "expected_latency_ms": 5.8,
      "confidence": "VERY_HIGH",
      "rationale": "Extra safety margin for variable system load"
    }
  }
}
```

---

## 6. Implementation Considerations

### 6.1 Architecture

**[TUNE-ARCH-010]** Standalone utility structure:
```
wkmp-ap/src/bin/tune_buffers.rs (new)
  ├─ Main loop: Orchestrate testing
  ├─ Audio test harness: Simplified playback for testing
  ├─ Metrics collector: Gather underrun/timing data
  ├─ Search algorithm: Binary search implementation
  ├─ Report generator: Format results
  └─ Database updater: Apply recommended values
```

**[TUNE-ARCH-020]** Reuse existing components:
- Ring buffer from `wkmp-ap/src/playback/ring_buffer.rs`
- Audio output from `wkmp-ap/src/audio/output.rs`
- Database settings from `wkmp-ap/src/db/settings.rs`
- Decoder/resampler for test audio

**[TUNE-ARCH-030]** Minimize dependencies:
- No HTTP server needed
- No SSE event broadcasting
- No queue management complexity
- Just: DB → Audio output → Metrics → Results

### 6.2 Safety and Error Handling

**[TUNE-SAFE-010]** Preserve user settings:
- Read current values before starting
- Store in memory/temp file as backup
- Restore on abort (Ctrl+C, panic, error)

**[TUNE-SAFE-020]** Detect system problems:
- If any test hangs >60 seconds: Abort
- If audio device fails to open: Report error and exit
- If database unavailable: Exit gracefully

**[TUNE-SAFE-030]** Sanity checks:
- Validate all parameters are in allowed ranges
- Reject impossible combinations (e.g., 1ms + 8192 frames)
- Warn if results seem anomalous

### 6.3 Testing and Validation

**[TUNE-TEST-040]** Unit tests for:
- Binary search algorithm (mocked test results)
- Curve fitting logic
- Result synthesis (recommendation generation)
- JSON export/import

**[TUNE-TEST-050]** Integration tests:
- Run tuning on CI system
- Verify recommended values are sane
- Check JSON output is valid

**[TUNE-TEST-060]** Manual validation:
- Run on development hardware
- Apply recommended values
- Verify 1+ hour of stable playback

---

## 7. Future Enhancements

### 7.1 Advanced Features

**[TUNE-FUT-010]** Multi-track testing:
- Test with multiple simultaneous audio sources
- Simulate real-world crossfade scenarios
- Validate stability under load

**[TUNE-FUT-020]** Hardware fingerprinting:
- Automatically detect system capabilities
- Match to known hardware profiles
- Skip testing if profile exists

**[TUNE-FUT-030]** Continuous monitoring:
- Detect when parameters become suboptimal over time
- Suggest re-tuning if underruns increase
- Log parameter effectiveness in production

**[TUNE-FUT-040]** Remote tuning:
- Web UI for tuning control
- Live progress via SSE
- Share results across instances

### 7.2 Platform-Specific Tuning

**[TUNE-PLAT-010]** Raspberry Pi Zero 2W considerations:
- Slower CPU requires larger buffers or longer intervals
- Limited memory affects maximum buffer sizes
- May need different test durations (longer to detect issues)

**[TUNE-PLAT-020]** Windows vs. Linux vs. macOS:
- Different audio APIs (WASAPI, ALSA, CoreAudio)
- Different scheduler behaviors
- May need platform-specific tweaks

---

## 8. Success Criteria

**[TUNE-SUCCESS-010]** Functional requirements:
- ✓ Completes tuning run in <10 minutes (quick mode)
- ✓ Identifies stable parameter combinations
- ✓ Generates actionable recommendations
- ✓ Exports results in machine-readable format

**[TUNE-SUCCESS-020]** Quality requirements:
- ✓ Recommended values result in <0.1% underruns
- ✓ No false positives (marking stable as unstable)
- ✓ No false negatives (marking unstable as stable)
- ✓ Results reproducible (±10% variation)

**[TUNE-SUCCESS-030]** Usability requirements:
- ✓ Clear progress indication
- ✓ Understandable recommendations
- ✓ Easy to apply results
- ✓ Useful error messages

---

## 9. Open Questions

**[TUNE-Q-010]** How to handle audio device selection?
- Option A: Use default device (from database)  <<< Implement this option
- Option B: Allow specifying device for testing
- Option C: Test all available devices

**[TUNE-Q-020]** Should tuning be automatic on first run?  <<< Decision, no.  Not at this time.
- Pro: Ensures optimal values from start
- Con: Delays initial user experience
- Compromise: Prompt user, offer skip 

**[TUNE-Q-030]** How aggressive should recommendations be?
- Conservative: Larger buffers, more latency, higher stability <<< Implement this option, if possible determine variability of trouble points and tune for six sigma (very rare problems observed), if variability is not a practical measure recommend double the buffer size of the observed trouble point.
- Aggressive: Smaller buffers, less latency, tighter margins
- User preference?

**[TUNE-Q-040]** Integration with startup sequence?
- Run tuning during initial setup?
- Periodic re-tuning (monthly)?
- Only on-demand? <<< Implement this option

---

## 10. References

- [DBD-PARAM-111] mixer_check_interval_ms (SPEC016:282-295)
- [DBD-PARAM-110] audio_buffer_size (SPEC016:260-276)
- [SSD-RBUF-014] Ring buffer underrun monitoring
- [SSD-MIX-020] Mixer thread configuration

---

## Document Control

**Revision History:**
- v0.1 (2025-10-26): Initial draft

**Review Status:** Approved
**Approver:** Mango Cat
**Target Implementation:** Phase 8 or later
