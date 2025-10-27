# Buffer Auto-Tuning User Guide

**tune-buffers** - Automatically optimize audio buffer parameters for your system

---

## Table of Contents

- [Quick Start](#quick-start)
- [Understanding the Results](#understanding-the-results)
- [Command-Line Parameters](#command-line-parameters)
- [Interpreting Recommendations](#interpreting-recommendations)
- [Troubleshooting](#troubleshooting)
- [Advanced Usage](#advanced-usage)

---

## Quick Start

### Simplest Use Case

Run the tuning utility with default settings:

```bash
cargo run --bin tune-buffers
```

**What happens:**
1. System information is detected automatically
2. Audio tests run for ~5-10 minutes
3. Results and recommendations are displayed
4. No changes are made to your system automatically

**Expected duration:** 5-10 minutes with default settings

### Example Session

```bash
$ cargo run --bin tune-buffers

Starting buffer auto-tuning...

System: AMD Ryzen 5 5600X, Linux 6.8.0-85-generic, ALSA
Current: mixer_check_interval_ms=5, audio_buffer_size=512

Phase 1: Testing mixer intervals with default buffer...
[✓] 2ms interval, 512 buffer: FAIL (12.5% underruns)
[✓] 5ms interval, 512 buffer: OK (0.02% underruns)
[✓] 10ms interval, 512 buffer: OK (0.01% underruns)
[✓] 20ms interval, 512 buffer: OK (0% underruns)
[✓] 50ms interval, 512 buffer: OK (0% underruns)

Viable intervals: [5, 10, 20, 50]

Phase 2: Finding minimum buffer sizes...
[✓] 5ms interval: Min buffer = 256 frames
[✓] 10ms interval: Min buffer = 128 frames
[✓] 20ms interval: Min buffer = 128 frames
[✓] 50ms interval: Min buffer = 64 frames

╔════════════════════════════════════════╗
║     Buffer Auto-Tuning Complete      ║
╚════════════════════════════════════════╝

Duration: 342 seconds
Tests run: 23
Stable configurations: 4
System: AMD Ryzen 5 5600X

Results:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Recommended: mixer_check_interval_ms = 10
             audio_buffer_size = 256

Rationale: Lowest interval (10ms) with reasonable buffer size (256). 2x safety margin applied.
Expected latency: 5.8ms
Confidence: High
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Alternative (Conservative):
mixer_check_interval_ms = 20
audio_buffer_size = 384

Rationale: Extra safety margin for maximum stability. Buffer size: 384. Recommended for systems with variable load.
Expected latency: 8.7ms
Confidence: VeryHigh
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

---

## Understanding the Results

### Phase 1: Initial Testing (Adaptive)

Phase 1 now uses **adaptive buffer sizing** - it automatically tries larger buffers until it finds stable configurations:

```
Trying with buffer size: 512 frames (11.6ms @ 44.1kHz)
[✓] 5ms interval, 512 buffer: OK (0.02% underruns)
```

If 512 fails for all intervals, it automatically tries 1024, then 2048, then 4096.

**Components:**
- **[✓]** - Test passed (stable)
- **[⚠]** - Test warning (marginal stability)
- **[✗]** - Test failed (unstable)
- **5ms interval** - How often the mixer checks for new audio
- **512 buffer** - Size of audio output buffer in frames
- **0.02% underruns** - Percentage of audio callbacks that found empty buffer

**What this means:**
- Lower underrun rate = better (target: <0.1%)
- 0% underruns = perfect stability
- >1% underruns = unacceptable (audio gaps)

### Phase 2: Finding Optimal Settings

```
[✓] 10ms interval: Min buffer = 128 frames
```

**What this means:**
- The utility found that with a 10ms mixer interval, the system can maintain stable audio with as small as 128 frames buffer
- **Safety margins are applied automatically** (2x for primary, 3x for conservative)
- You don't need to do mental math - the recommendations already include safety

### Curve Summary Table

```
┌──────────┬────────────┬──────────┐
│ Interval │ Min Buffer │ Status   │
├──────────┼────────────┼──────────┤
│    2 ms  │    - frames│ Unstable │
│    5 ms  │  256 frames│ Stable   │
│   10 ms  │  128 frames│ Stable   │
│   20 ms  │  128 frames│ Stable   │
│   50 ms  │   64 frames│ Stable   │
└──────────┴────────────┴──────────┘
```

**Reading the table:**
- **Interval:** Mixer check frequency (lower = more responsive, higher CPU)
- **Min Buffer:** Minimum stable buffer size found (lower = less latency)
- **Status:**
  - Stable = Safe to use
  - Marginal = Use with caution
  - Unstable = Do not use

---

## Command-Line Parameters

### Basic Options

#### `--quick`
Fast tuning mode for quick results.

```bash
cargo run --bin tune-buffers -- --quick
```

- **Duration:** ~5 minutes
- **Tests fewer intervals:** [5, 10, 20, 50] milliseconds
- **Shorter test duration:** 20 seconds per configuration
- **Use when:** You want quick ballpark settings

#### `--thorough`
Comprehensive tuning for maximum accuracy.

```bash
cargo run --bin tune-buffers -- --thorough
```

- **Duration:** ~15 minutes
- **Tests more intervals:** [1, 2, 5, 10, 20, 50, 100] milliseconds
- **Standard test duration:** 30 seconds per configuration
- **Use when:** You need the most accurate recommendations

#### `--export <file>`
Save results to JSON file for later analysis.

```bash
cargo run --bin tune-buffers -- --export my_system.json
```

- **Output:** Complete test results in JSON format
- **Includes:** System info, all test data, recommendations
- **Use when:** You want to compare results across systems or save for documentation

#### `--apply`
Display recommendations that would be applied.

```bash
cargo run --bin tune-buffers -- --apply
```

- **Currently:** Shows what would be applied (database update not yet implemented)
- **Future:** Will automatically update database settings
- **Safety:** Currently safe - only displays, doesn't modify anything

#### `--test-duration <seconds>`
Customize how long each test runs.

```bash
cargo run --bin tune-buffers -- --test-duration 60
```

- **Default:** 30 seconds
- **Range:** 10-120 seconds recommended
- **Longer tests:** More accurate but slower
- **Shorter tests:** Faster but may miss intermittent issues

### Combining Options

```bash
# Quick tuning with JSON export
cargo run --bin tune-buffers -- --quick --export quick_results.json

# Thorough tuning with 60-second tests
cargo run --bin tune-buffers -- --thorough --test-duration 60

# Quick tuning and display what would be applied
cargo run --bin tune-buffers -- --quick --apply
```

---

## Interpreting Recommendations

### Primary Recommendation

```
Recommended: mixer_check_interval_ms = 10
             audio_buffer_size = 256

Rationale: Lowest interval (10ms) with reasonable buffer size (256). 2x safety margin applied.
Expected latency: 5.8ms
Confidence: High
```

**What this means:**
- **mixer_check_interval_ms = 10**
  - Mixer checks for new audio every 10 milliseconds
  - Lower values = more responsive to queue changes
  - Higher values = less CPU usage

- **audio_buffer_size = 256**
  - Audio output buffer holds 256 frames
  - At 44.1kHz: 256 frames = ~5.8ms of audio
  - Lower values = less latency
  - Higher values = more stability

- **Expected latency: 5.8ms**
  - How long between when audio is generated and when you hear it
  - This is very low latency (good for responsive audio)
  - For music player: latency <50ms is typically imperceptible

- **Confidence: High**
  - High/VeryHigh = Tested and verified stable
  - Medium = Based on extrapolation or limited testing
  - Low = Uncertain, may need retesting

### Conservative Recommendation

```
Alternative (Conservative):
mixer_check_interval_ms = 20
audio_buffer_size = 384

Rationale: Extra safety margin for maximum stability.
Expected latency: 8.7ms
Confidence: VeryHigh
```

**When to use conservative:**
- System has variable load (background processes)
- Running on slower hardware
- Prefer stability over low latency
- Initial deployment (can optimize later)

**When to use primary:**
- System is dedicated to music playback
- Hardware is modern and capable
- You've verified stability with test runs
- You want lowest possible latency

### Confidence Levels

| Level | Meaning | When to Trust |
|-------|---------|---------------|
| **VeryHigh** | Extensively tested, large safety margin | Always safe to use |
| **High** | Well-tested, good safety margin | Safe for production use |
| **Medium** | Limited testing or extrapolated | Use with monitoring |
| **Low** | Insufficient data | Re-run with --thorough |

---

## Troubleshooting

### Problem: No stable configurations found

The tool now provides detailed diagnostics if it can't find stable configurations:

```
✗ TUNING FAILED: Could not find stable configuration even with 4096 frame buffer

Diagnostic information:
  Tests completed: 25
  Best result: 50ms interval, 4096 buffer, 1.2% underruns

Possible causes:
  1. Audio device not available or inaccessible
  2. System audio configuration issue
  3. Audio backend (cpal/ALSA) initialization failure
  4. Permissions issue accessing audio device

Troubleshooting steps:
  1. Check audio device: aplay -l (Linux) or system audio settings
  2. Test audio: speaker-test -c 2 -t wav
  3. Check permissions: groups | grep audio
  4. Run with debug logging: RUST_LOG=debug cargo run --bin tune-buffers
```

**New in this version:** The tool automatically tries progressively larger buffers (512 → 1024 → 2048 → 4096) before giving up.

**If you see "Best result: X.X% underruns":**
- **<0.1%** = Should have passed (possible bug, run with RUST_LOG=debug)
- **0.1-1%** = Marginal (system might need more resources)
- **>1%** = Unstable (audio system issue)

**Solutions:**

1. **Verify audio device works**
   ```bash
   # Linux - list devices
   aplay -l

   # Test audio output
   speaker-test -c 2 -t wav -l 1

   # Check if you're in audio group
   groups | grep audio
   ```

2. **Check audio backend initialization**
   ```bash
   # Run with debug logging to see cpal initialization
   RUST_LOG=debug cargo run --bin tune-buffers -- --quick 2>&1 | grep -i "audio\|cpal\|alsa"
   ```

3. **Verify no other audio applications are running**
   ```bash
   # Linux - check what's using audio
   lsof /dev/snd/*

   # Kill PulseAudio if needed (it will auto-restart)
   pulseaudio -k
   ```

4. **Try manual audio test first**
   - Before running tune-buffers, verify your system can play audio
   - Any audio player should work (mpv, vlc, etc.)
   - If basic audio doesn't work, tune-buffers won't work either

5. **Check for permission issues**
   ```bash
   # Add yourself to audio group (then logout/login)
   sudo usermod -a -G audio $USER
   ```

### Problem: Results vary between runs

**This is normal!** Audio timing is affected by:
- Background CPU load
- Operating system scheduler behavior
- Thermal throttling
- Other system activities

**Solutions:**
- Run tuning multiple times and use most conservative recommendations
- Run during consistent system load
- Use `--thorough` mode for more stable results
- Add extra safety margin to recommendations (e.g., if it recommends 256, use 384)

### Problem: Recommended values cause audio gaps

**Possible causes:**
1. **System load increased since tuning**
   - Re-run tuning during typical usage

2. **Test duration too short**
   ```bash
   cargo run --bin tune-buffers -- --test-duration 60
   ```

3. **Use conservative recommendation instead**
   - Conservative settings have 3x safety margin vs 2x for primary

4. **Manually increase buffer size**
   - If recommends 256, try 512
   - If recommends 10ms interval, try 20ms

### Problem: Tuning takes too long

**Solutions:**
1. **Use quick mode**
   ```bash
   cargo run --bin tune-buffers -- --quick
   ```

2. **Reduce test duration** (less accurate)
   ```bash
   cargo run --bin tune-buffers -- --test-duration 20
   ```

3. **Run in background**
   ```bash
   nohup cargo run --bin tune-buffers -- --export results.json &
   ```

---

## Advanced Usage

### Comparing Systems

Test on multiple systems and compare:

```bash
# Development machine
cargo run --bin tune-buffers -- --export dev_machine.json

# Raspberry Pi
cargo run --bin tune-buffers -- --export raspberry_pi.json

# Compare
diff <(jq '.recommendations' dev_machine.json) \
     <(jq '.recommendations' raspberry_pi.json)
```

### Automated Testing Scripts

```bash
#!/bin/bash
# test_configurations.sh

# Test multiple configurations
for duration in 20 30 60; do
    echo "Testing with ${duration}s duration..."
    cargo run --bin tune-buffers -- \
        --test-duration $duration \
        --export "results_${duration}s.json"
done
```

### Understanding JSON Export

The JSON export contains:

```json
{
  "session": {
    "timestamp": "2025-10-27T14:30:00Z",
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
      "mixer_check_interval_ms": 10,
      "audio_buffer_size": 256,
      "test_duration_secs": 30,
      "underruns": {
        "underrun_count": 3,
        "callback_count": 2586,
        "underrun_rate": 0.116
      },
      "verdict": "Stable"
    }
  ],
  "curve_data": [...],
  "recommendations": {
    "primary": {...},
    "conservative": {...}
  }
}
```

**Useful for:**
- Documenting system configurations
- Comparing performance across hardware
- Tracking parameter stability over time
- Troubleshooting audio issues

### Best Practices

1. **Initial Setup**
   - Run `--thorough` mode once
   - Document results for your system
   - Start with conservative recommendation

2. **Regular Re-tuning**
   - Re-run after OS updates
   - Re-run after hardware changes
   - Re-run if audio gaps appear

3. **Production Deployment**
   - Use conservative recommendations
   - Monitor audio quality for 24 hours
   - Gradually optimize if needed

4. **Performance Monitoring**
   - Export results before and after changes
   - Compare underrun rates
   - Adjust based on real-world performance

---

## Parameter Reference

### mixer_check_interval_ms

**What it controls:** How often the mixer thread checks for new audio to process

**Range:** 1-100 milliseconds

**Effects:**
- **Lower values (1-5ms):**
  - ✓ More responsive to queue changes
  - ✓ Better for dynamic playlists
  - ✗ Higher CPU usage
  - ✗ Requires larger buffers

- **Higher values (20-100ms):**
  - ✓ Lower CPU usage
  - ✓ Can use smaller buffers
  - ✗ Less responsive to queue changes
  - ✗ Slower reaction to errors

**Typical values:**
- Fast system: 5-10ms
- Average system: 10-20ms
- Slow system: 20-50ms

### audio_buffer_size

**What it controls:** Size of audio output buffer in frames

**Range:** 64-8192 frames

**Effects:**
- **Smaller buffers (64-256 frames):**
  - ✓ Lower latency (~1.5-6ms)
  - ✓ More responsive
  - ✗ Higher risk of underruns
  - ✗ Requires faster mixer

- **Larger buffers (512-2048 frames):**
  - ✓ More stable
  - ✓ Better for variable loads
  - ✗ Higher latency (~12-46ms)
  - ✗ Slower response to pause/skip

**Typical values:**
- Low latency: 128-256 frames
- Balanced: 256-512 frames
- Stable: 512-1024 frames

**At 44.1kHz sample rate:**
- 128 frames = 2.9ms
- 256 frames = 5.8ms
- 512 frames = 11.6ms
- 1024 frames = 23.2ms

---

## Quick Reference

### Decision Tree

```
Need results fast?
├─ Yes → Use --quick
└─ No → Use --thorough (or default)

System has variable load?
├─ Yes → Use conservative recommendation
└─ No → Use primary recommendation

Need to compare systems?
├─ Yes → Use --export
└─ No → Just read console output

Confident in results?
├─ Yes → Apply to database (when --apply implemented)
└─ No → Run again with --thorough
```

### Common Commands

```bash
# Most common use case
cargo run --bin tune-buffers

# Quick check
cargo run --bin tune-buffers -- --quick

# Thorough analysis
cargo run --bin tune-buffers -- --thorough

# Save results
cargo run --bin tune-buffers -- --export my_results.json

# Quick with export
cargo run --bin tune-buffers -- --quick --export quick.json
```

---

## Support

If you experience issues:

1. Check [Troubleshooting](#troubleshooting) section
2. Review system logs for audio errors
3. Run with `RUST_LOG=debug` for detailed output:
   ```bash
   RUST_LOG=debug cargo run --bin tune-buffers
   ```
4. Export results and review for anomalies
5. Try conservative recommendations first

---

**Version:** 1.0
**Last Updated:** 2025-10-27
**Traceability:** PLAN004 Buffer Auto-Tuning Implementation
