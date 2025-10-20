# Startup Performance Diagnostics

**Issue:** Manual testing shows first passage startup still slower than expected despite optimizations.

**Expected:** ~500-800ms for first track
**Observed:** Needs measurement

---

## Unit Test Limitations

### What Unit Tests VERIFY ✅
1. Event-driven notification works (sub-millisecond event delivery)
2. First-passage optimization uses 500ms threshold
3. Buffer events are emitted when threshold reached
4. Concurrent operations don't deadlock

### What Unit Tests DON'T VERIFY ❌
1. **Actual audio decode speed** - Tests use dummy data `vec![0.0; samples]`, no real Symphonia decoding
2. **End-to-end startup flow** - Tests don't exercise full pipeline (HTTP → queue → decode → buffer → event → mixer → audio output)
3. **Real file I/O performance** - No file opening, seeking, format probing
4. **Decoder pool integration** - BufferManager tests bypass DecoderPool entirely

---

## Critical Path Analysis

### End-to-End Startup Flow

```
HTTP /playback/enqueue
  ↓ (~1ms)
Queue insertion
  ↓ (~1ms)
Decoder pool submits decode request
  ↓ (POTENTIAL BOTTLENECK 1: Worker thread scheduling)
Worker picks up request
  ↓ (POTENTIAL BOTTLENECK 2: File open + format probe)
Symphonia opens file & probes format
  ↓ (POTENTIAL BOTTLENECK 3: Decode first 1 second)
Decode first chunk (88200 samples = 1s stereo @ 44.1kHz)
  ↓ (~1ms)
Append samples to buffer
  ↓ (~1ms)
notify_samples_appended() called
  ↓ (<1ms - verified by unit tests)
check_and_notify_ready() evaluates
  ↓ (If buffer >= 500ms: instant event)
ReadyForStart event sent
  ↓ (<1ms - verified by unit tests)
buffer_event_handler() receives event
  ↓ (POTENTIAL BOTTLENECK 4: Mixer start delay)
Mixer starts playback
  ↓ (POTENTIAL BOTTLENECK 5: Audio device init)
First audio samples to output
```

---

## Diagnostic Steps

### Step 1: Check If First-Passage Optimization is Active

**Look for this log line when first file is enqueued:**

```
⚡ Buffer ready for playback: <uuid> (500ms >= 500ms threshold)
```

or

```
⚡ Buffer ready for playback: <uuid> (1000ms >= 500ms threshold)
```

**If you see:**
```
⚡ Buffer ready for playback: <uuid> (3000ms >= 3000ms threshold)
```

❌ **First-passage optimization is NOT active** - threshold is still 3000ms

---

### Step 2: Measure Decode Time

**Add timing instrumentation to decoder_pool.rs:**

Check logs for:
```
Appended chunk 1/X (10.0%)   <-- After first 1-second chunk
```

Measure wall-clock time from "Worker X processing" to "Appended chunk 1/X".

**Expected:** <100ms for MP3, <200ms for FLAC
**If slower:** Decode is the bottleneck

---

### Step 3: Check Event Delivery

**Look for these logs in sequence:**

```
🚀 Buffer ready event received: <uuid> (XXXms available)
⚡ Starting playback instantly (buffer ready): passage=<uuid>
✅ Mixer started in X.XXms (event-driven instant start)
```

**If missing:** Event handler may not be running

---

### Step 4: Measure End-to-End

**Run these curl commands with timestamps:**

```bash
# Start fresh
killall -9 wkmp-ap
cargo run --package wkmp-ap > /tmp/startup.log 2>&1 &
sleep 2

# Measure enqueue to playback start
echo "=== ENQUEUE @ $(date +%s.%N) ==="
curl -X POST http://localhost:5721/playback/enqueue \
  -H "Content-Type: application/json" \
  -d '{"file_path": "/path/to/your/test.mp3"}'

echo "=== PLAY @ $(date +%s.%N) ==="
curl -X POST http://localhost:5721/playback/play

# Check logs
sleep 5
grep -E "Buffer ready|Mixer started|Starting playback" /tmp/startup.log
```

---

## Likely Bottlenecks (Ranked)

### 1. **Symphonia Decode Performance** (MOST LIKELY)
- First chunk decode of compressed audio (MP3/FLAC) can be slow
- Format probing adds overhead
- First read from disk is cold cache

**Fix:** None needed if <200ms - this is I/O bound
**If >500ms:** Consider using uncompressed WAV for testing, or pre-decode

---

### 2. **Decoder Pool Worker Scheduling**
- Request sits in queue waiting for worker thread to pick it up
- Workers may be blocked on previous decode

**Check:** Look for gap between "Submitting decode request" and "Worker X processing"
**Fix:** Already using 2 workers, sufficient for Pi Zero2W

---

### 3. **Event Channel Not Connected**
- buffer_event_handler may not be running
- Event channel not initialized

**Check:** Search logs for "Buffer event handler started"
**Fix:** Verify PlaybackEngine starts handler in `start()` method

---

### 4. **First-Passage Flag Not Cleared**
- `ever_played` flag may be persisted across restarts
- Every passage uses first-passage optimization (always 500ms)

**Check:** Enqueue two files, check if second also uses 500ms threshold
**Fix:** Flag is in-memory only, should reset on restart

---

### 5. **Database Settings Override**
- `minimum_buffer_threshold_ms` may be set to 3000ms in database
- Overrides first-passage optimization

**Check:**
```bash
sqlite3 /path/to/wkmp.db "SELECT key, value FROM settings WHERE key = 'minimum_buffer_threshold_ms'"
```

**Fix:** Delete setting or set to 500:
```sql
DELETE FROM settings WHERE key = 'minimum_buffer_threshold_ms';
-- OR
INSERT OR REPLACE INTO settings (key, value) VALUES ('minimum_buffer_threshold_ms', '500');
```

---

## Quick Diagnostic Script

```bash
#!/bin/bash
# Save as: test_startup_performance.sh

set -e

echo "=== Killing existing instances ==="
killall -9 wkmp-ap 2>/dev/null || true
sleep 1

echo "=== Starting wkmp-ap with logging ==="
cargo run --package wkmp-ap > /tmp/wkmp_diag.log 2>&1 &
WKMP_PID=$!
sleep 3

echo "=== Checking if server is up ==="
curl -s http://localhost:5721/health || { echo "Server not responding"; exit 1; }

echo "=== Measuring startup time ==="
START=$(date +%s.%N)

curl -s -X POST http://localhost:5721/playback/enqueue \
  -H "Content-Type: application/json" \
  -d '{"file_path": "/tmp/test_audio_10s_mp3.mp3"}' > /dev/null

curl -s -X POST http://localhost:5721/playback/play > /dev/null

# Wait for playback to start
sleep 1

END=$(date +%s.%N)
ELAPSED=$(echo "$END - $START" | bc)

echo "=== Results ==="
echo "Elapsed time: ${ELAPSED}s"
echo ""
echo "=== Key log lines ==="
grep -E "Buffer ready|Mixer started|Starting playback|Worker.*processing|Appended chunk 1/" /tmp/wkmp_diag.log | tail -20

echo ""
echo "=== First-passage threshold check ==="
grep "Buffer ready for playback" /tmp/wkmp_diag.log | head -1

kill $WKMP_PID 2>/dev/null || true
```

---

## Expected vs Actual Timeline

### Expected (Optimized)
- HTTP request: 0ms
- Queue + decode submit: +1ms
- Worker picks up: +5ms
- File open + probe: +50ms (MP3), +100ms (FLAC)
- Decode first 1s chunk: +100ms (MP3), +200ms (FLAC)
- Event notification: +1ms
- Mixer start: +10ms
- **Total: ~170ms (MP3), ~320ms (FLAC)**

### If Seeing >1000ms
Likely causes:
1. Decode taking >500ms (check codec/file format)
2. Still using 3000ms threshold (check database setting)
3. Event handler not running (check logs for handler startup)

---

## Recommendation

Run the diagnostic script with a real audio file and share:
1. The "Elapsed time" output
2. The "Key log lines" showing actual event sequence
3. The "First-passage threshold check" showing which threshold was used

This will pinpoint whether the bottleneck is:
- **Decode** (expected, can't optimize much further)
- **Configuration** (threshold not set correctly)
- **Event delivery** (handler not running)
