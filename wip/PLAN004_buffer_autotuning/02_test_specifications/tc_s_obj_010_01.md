# TC-S-OBJ-010-01: End-to-End Parameter Determination

**Requirement:** TUNE-OBJ-010 (lines 30-32)
**Test Type:** System Test (End-to-End)
**Priority:** High
**Estimated Effort:** 60 minutes

---

## Test Objective

Verify complete tuning workflow successfully determines safe operating values for both mixer_check_interval_ms and audio_buffer_size on real hardware.

---

## Test Specification

### Given: Clean database with default settings and working audio device

```bash
# Initial conditions
Database: Fresh wkmp.db with default settings
  mixer_check_interval_ms = 5
  audio_buffer_size = 512
Audio Device: System default (verified available)
Test Audio: Music passage in database OR generated test tone
```

### When: Full tuning run executed in thorough mode

```bash
wkmp-ap tune-buffers --thorough --export results.json
```

### Then: Tuning completes successfully with actionable recommendations

**Expected Output:**
```
Starting buffer auto-tuning...

System: [Detected CPU], [Detected OS], [Audio API]
Current: mixer_check_interval_ms=5, audio_buffer_size=512

Phase 1: Testing mixer intervals with default buffer... [6-8 tests, ~3 min]
[✓] 1ms: FAIL (23% underruns)
[✓] 2ms: FAIL (12% underruns)
[✓] 5ms: OK (0.02% underruns)
[✓] 10ms: OK (0.01% underruns)
[✓] 20ms: OK (0% underruns)
[✓] 50ms: OK (0% underruns)

Phase 2: Finding minimum buffer sizes... [10-15 min]
[✓] 5ms interval: Testing buffers 64-4096... → 256 frames
[✓] 10ms interval: Testing buffers 64-4096... → 128 frames
[✓] 20ms interval: Testing buffers 64-4096... → 128 frames
[✓] 50ms interval: Testing buffers 64-4096... → 64 frames

Results:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Recommended: mixer_check_interval_ms = [5-20]
             audio_buffer_size = [128-256]

Rationale: [Explanation]
Expected latency: [~3-6ms]
Stability: [Excellent/Good]
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Export saved to: results.json

Apply these values? [Y/n]
```

---

## Verify

### Success Assertions

```bash
# 1. Tuning completes within time limit
Duration < 15 minutes (thorough mode)

# 2. Recommendations are within valid ranges
mixer_check_interval_ms: 1-100
audio_buffer_size: 64-8192

# 3. JSON export is valid
jq . results.json  # Should parse without errors
jq '.recommendations.primary' results.json  # Should exist

# 4. Recommended values are reasonable
# (based on development hardware characteristics)
mixer_check_interval_ms >= 5
audio_buffer_size <= 1024

# 5. System info captured correctly
jq '.system_info.cpu' results.json  # Should contain CPU model
jq '.system_info.os' results.json   # Should contain OS version
```

### Manual Verification

```bash
# Apply recommended values
wkmp-ap tune-buffers --apply

# Verify database updated
sqlite3 wkmp.db "SELECT value FROM settings WHERE key='mixer_check_interval_ms';"
# Should match recommendation

sqlite3 wkmp.db "SELECT value FROM settings WHERE key='audio_buffer_size';"
# Should match recommendation
```

### Pass Criteria

- ✓ Tuning completes in <15 minutes
- ✓ At least one valid recommendation generated
- ✓ Recommended values within parameter ranges
- ✓ JSON export valid and complete
- ✓ System information captured
- ✓ Curve data populated (≥3 data points)
- ✓ No crashes or hangs during execution
- ✓ Database not corrupted

### Fail Criteria

- ✗ Tuning hangs or crashes
- ✗ No recommendations generated
- ✗ Recommendations outside valid ranges
- ✗ JSON export invalid or incomplete
- ✗ Duration >15 minutes
- ✗ Database corrupted

---

## Test Scenarios

### Scenario 1: Development System (Fast CPU)

**Expected Results:**
- Multiple viable intervals (5ms, 10ms, 20ms, 50ms)
- Small buffers work (128-256 frames)
- Primary recommendation: 10ms interval, 128 frames
- Conservative recommendation: 20ms interval, 256 frames
- Duration: ~10 minutes

### Scenario 2: Slow System (Raspberry Pi Zero 2W simulation)

**Expected Results:**
- Fewer viable intervals (20ms, 50ms only)
- Larger buffers needed (512-1024 frames)
- Primary recommendation: 20ms interval, 512 frames
- Conservative recommendation: 50ms interval, 1024 frames
- Duration: ~15 minutes
- Warning: "High latency unavoidable on this system"

### Scenario 3: No Audio Device

**Expected Result:**
- Error message: "No audio output device found"
- Graceful exit with error code
- No database changes
- Duration: <1 second

---

## Environment Setup

### Prerequisites

```bash
# 1. Build project
cargo build --release -p wkmp-ap

# 2. Initialize database
sqlite3 wkmp.db < migrations/001_initial_schema.sql

# 3. Set default settings
sqlite3 wkmp.db "INSERT INTO settings (key, value) VALUES
  ('mixer_check_interval_ms', '5'),
  ('audio_buffer_size', '512');"

# 4. Verify audio device available
pactl list short sinks  # Linux/PulseAudio
# OR
aplay -l  # Linux/ALSA
```

### Cleanup

```bash
# Remove test database
rm wkmp.db

# Remove exported results
rm results.json

# Remove temp backup file
rm /tmp/wkmp_tuning_backup.json
```

---

## Test Data

**Input:**
- Database: wkmp.db (fresh or existing)
- Audio device: System default
- Test audio: Generated 440 Hz tone (30s duration, 48kHz→44.1kHz resample)

**Output:**
- JSON file: results.json (~5-10KB)
- Updated database settings (2 keys)
- Console output (~100 lines)

---

## Integration Points

**System Components Exercised:**
- Database access (sqlx)
- Audio output (cpal)
- Decoder (symphonia) - if using passage
- Resampler (rubato)
- Ring buffer (underrun monitoring)
- Callback monitor (jitter tracking)
- All tuning modules (search, curve, report)

**External Dependencies:**
- SQLite database
- Audio device driver (ALSA/PulseAudio/etc.)
- System CPU scheduler
- Filesystem (for JSON export)

---

## Performance Benchmarks

| Hardware | Expected Duration | Expected Recommendations |
|----------|-------------------|--------------------------|
| Development (Ryzen 5 5600X) | ~10 min | 10ms, 128 frames |
| Mid-range (i5-8250U) | ~12 min | 10-20ms, 256 frames |
| Low-end (Raspberry Pi Zero 2W) | ~15 min | 20-50ms, 512-1024 frames |

---

## Known Issues / Limitations

1. **Audio device must support requested buffer sizes**
   - Some devices may not honor exact buffer size request
   - Mitigation: Verify actual buffer size after initialization

2. **System load affects results**
   - Heavy background tasks may cause false failures
   - Mitigation: Recommend running with minimal system load

3. **Results may vary ±10% between runs**
   - Scheduler behavior is non-deterministic
   - Mitigation: Accept variation, provide conservative recommendations

---

## Traceability

**Requirement:** TUNE-OBJ-010
**Related Requirements:** TUNE-DET-010, TUNE-SRC-010, TUNE-ALG-010, TUNE-OUT-010
**Related Tests:**
- TC-S-OBJ-020-01 (curve characterization)
- TC-S-OBJ-030-01 (recommendation generation)
- TC-M-TEST-060-01 (hardware validation)

**Validates:** Complete workflow from database → test → analysis → recommendation → export
