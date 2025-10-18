# TESTING: Event-Driven Architecture Implementation

**📋 TIER R - REVIEW & CHANGE CONTROL**

Comprehensive testing guide for validating the event-driven position tracking implementation. This document provides step-by-step procedures for manual testing, SSE monitoring, and performance validation.

**Authority:** Operational guidance - Updated as testing progresses

**Status:** Active
**Date:** 2025-10-18
**Type:** Testing Plan
**Related Documents:**
- [MIGRATION-event_driven_architecture.md](./MIGRATION-event_driven_architecture.md) - Implementation plan (historical - archived)
- [REV002-event_driven_architecture_update.md](../REV002-event_driven_architecture_update.md) - Architecture change
- [SPEC001-architecture.md](../SPEC001-architecture.md) - Architecture specification

---

## Executive Summary

**Objective:** Validate that the event-driven position tracking implementation:
1. Emits `PositionUpdate` events from mixer at configured intervals
2. Detects song boundaries and emits `CurrentSongChanged` events
3. Emits `PlaybackProgress` events at configured intervals
4. Achieves <1% CPU usage
5. Achieves <50ms song boundary detection latency

**Estimated Time:** 2-3 hours

**Prerequisites:**
- Event-driven implementation complete and compiled
- Audio test files available (multi-song passages)
- Database with `passage_songs` table populated
- Browser with DevTools for SSE monitoring

---

## Table of Contents

1. [Pre-Testing Setup](#pre-testing-setup)
2. [Unit Test Verification](#unit-test-verification)
3. [Test Data Preparation](#test-data-preparation)
4. [Manual Testing Procedures](#manual-testing-procedures)
5. [SSE Event Monitoring](#sse-event-monitoring)
6. [Performance Validation](#performance-validation)
7. [Edge Case Testing](#edge-case-testing)
8. [Troubleshooting Guide](#troubleshooting-guide)
9. [Sign-Off Checklist](#sign-off-checklist)

---

## Pre-Testing Setup

### 1. Build and Verify Compilation

```bash
# Navigate to project root
cd /home/sw/Dev/McRhythm

# Clean build
cargo clean

# Build wkmp-ap with all features
cargo build --package wkmp-ap --release

# Expected output:
#   Compiling wkmp-ap v0.1.0
#   Finished release [optimized] target(s) in X.XXs
```

**Expected Result:** ✅ Build succeeds with no errors

**Actual Result:** _____________

### 2. Run Unit Tests

```bash
# Run all wkmp-ap library tests
cargo test --package wkmp-ap --lib

# Expected: 106 tests pass
```

**Expected Result:** ✅ All 106 tests pass

**Actual Result:** _____________

### 3. Verify New Modules

```bash
# Check that new files exist
ls -la wkmp-ap/src/playback/events.rs
ls -la wkmp-ap/src/playback/song_timeline.rs
ls -la wkmp-ap/src/db/passage_songs.rs

# Expected: All three files present
```

**Expected Result:** ✅ All 3 new modules exist

**Actual Result:** _____________

---

## Unit Test Verification

### Verify Specific Test Suites

#### Song Timeline Tests (11 tests)

```bash
cargo test --package wkmp-ap --lib song_timeline::tests -- --nocapture
```

**Expected Tests:**
- ✅ `test_empty_timeline`
- ✅ `test_single_song`
- ✅ `test_multiple_songs_with_gaps`
- ✅ `test_forward_seek_across_songs`
- ✅ `test_backward_seek`
- ✅ `test_unsorted_entries_get_sorted`
- ✅ `test_get_current_song_no_state_change`
- ✅ `test_gap_only_passage`
- ✅ `test_entry_equality`

**Actual Results:** _____________

#### Passage Songs Database Tests (8 tests)

```bash
cargo test --package wkmp-ap --lib passage_songs::tests -- --nocapture
```

**Expected Tests:**
- ✅ `test_empty_passage_songs`
- ✅ `test_single_song`
- ✅ `test_multiple_songs`
- ✅ `test_gap_with_null_song_guid`
- ✅ `test_invalid_uuid_filtered_out`
- ✅ `test_invalid_time_range_filtered_out`
- ✅ `test_missing_table_returns_empty`

**Actual Results:** _____________

#### Events Module Tests (4 tests)

```bash
cargo test --package wkmp-ap --lib events::tests -- --nocapture
```

**Expected Tests:**
- ✅ `test_position_update_creation`
- ✅ `test_state_changed_creation`
- ✅ `test_event_clone`
- ✅ `test_event_debug`

**Actual Results:** _____________

---

## Test Data Preparation

### Database Setup

#### Option A: Create Test Database from Scratch

```bash
# Create test database
sqlite3 /tmp/wkmp_test.db

# Create required tables
```

```sql
-- Passages table
CREATE TABLE passages (
    guid TEXT PRIMARY KEY NOT NULL,
    file_path TEXT NOT NULL,
    start_time_ms INTEGER NOT NULL,
    end_time_ms INTEGER NOT NULL,
    fade_in_point_ms INTEGER NOT NULL,
    lead_in_point_ms INTEGER NOT NULL,
    lead_out_point_ms INTEGER NOT NULL,
    fade_out_point_ms INTEGER NOT NULL
);

-- Passage songs table (NEW - required for testing)
CREATE TABLE passage_songs (
    guid TEXT PRIMARY KEY NOT NULL,
    passage_guid TEXT NOT NULL,
    song_guid TEXT,
    start_time_ms INTEGER NOT NULL,
    end_time_ms INTEGER NOT NULL,
    FOREIGN KEY (passage_guid) REFERENCES passages(guid) ON DELETE CASCADE
);

-- Settings table
CREATE TABLE settings (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT
);

-- Queue table
CREATE TABLE queue (
    guid TEXT PRIMARY KEY NOT NULL,
    passage_id TEXT,
    file_path TEXT NOT NULL,
    position INTEGER NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Insert test settings
INSERT INTO settings (key, value) VALUES ('position_event_interval_ms', '1000');
INSERT INTO settings (key, value) VALUES ('playback_progress_interval_ms', '5000');
INSERT INTO settings (key, value) VALUES ('volume_level', '0.5');
```

#### Option B: Use Existing Database

```bash
# Backup existing database
cp ~/wkmp.db ~/wkmp.db.backup

# Add passage_songs table if missing
sqlite3 ~/wkmp.db "CREATE TABLE IF NOT EXISTS passage_songs (
    guid TEXT PRIMARY KEY NOT NULL,
    passage_guid TEXT NOT NULL,
    song_guid TEXT,
    start_time_ms INTEGER NOT NULL,
    end_time_ms INTEGER NOT NULL
);"

# Add test settings
sqlite3 ~/wkmp.db "INSERT OR REPLACE INTO settings (key, value) VALUES
    ('position_event_interval_ms', '1000'),
    ('playback_progress_interval_ms', '5000');"
```

### Test Audio Files

**Required:** At least one multi-song audio file (e.g., album track or DJ mix)

#### Option A: Use Provided Test File

```bash
# Use the test file mentioned in earlier sessions
ls -la "/home/sw/Music/Bigger,_Better,_Faster,_More/(4_Non_Blondes)Bigger,_Better,_Faster,_More-03-What's_Up_.mp3"

# If available, use this file for testing
```

#### Option B: Create Test Passage with Songs

```bash
# Find any audio file
FILE_PATH="/path/to/your/audio/file.mp3"

# Get duration using ffprobe
DURATION_MS=$(ffprobe -v error -show_entries format=duration -of default=noprint_wrappers=1:nokey=1 "$FILE_PATH" | awk '{print int($1 * 1000)}')

echo "File duration: ${DURATION_MS}ms"
```

**Insert test passage:**

```sql
-- Generate UUIDs (or use fixed UUIDs for testing)
-- Passage: aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa
-- Song 1:  11111111-1111-1111-1111-111111111111
-- Song 2:  22222222-2222-2222-2222-222222222222

-- Insert passage
INSERT INTO passages (guid, file_path, start_time_ms, end_time_ms, fade_in_point_ms, lead_in_point_ms, lead_out_point_ms, fade_out_point_ms)
VALUES (
    'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa',
    '/path/to/your/audio/file.mp3',
    0,
    30000,  -- 30 seconds
    2000,   -- Fade-in point
    2000,   -- Lead-in point
    28000,  -- Lead-out point
    28000   -- Fade-out point
);

-- Insert first song (0-15 seconds)
INSERT INTO passage_songs (guid, passage_guid, song_guid, start_time_ms, end_time_ms)
VALUES (
    'aaaaaaaa-bbbb-cccc-dddd-111111111111',
    'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa',
    '11111111-1111-1111-1111-111111111111',
    0,
    15000
);

-- Insert second song (15-30 seconds)
INSERT INTO passage_songs (guid, passage_guid, song_guid, start_time_ms, end_time_ms)
VALUES (
    'aaaaaaaa-bbbb-cccc-dddd-222222222222',
    'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa',
    '22222222-2222-2222-2222-222222222222',
    15000,
    30000
);

-- Enqueue the passage
INSERT INTO queue (guid, passage_id, file_path, position)
VALUES (
    'qqqqqqqq-qqqq-qqqq-qqqq-qqqqqqqqqqqq',
    'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa',
    '/path/to/your/audio/file.mp3',
    0
);
```

**Verify data:**

```bash
sqlite3 ~/wkmp.db "SELECT * FROM passage_songs WHERE passage_guid = 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa';"
```

**Expected Output:**
```
aaaaaaaa-bbbb-cccc-dddd-111111111111|aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa|11111111-1111-1111-1111-111111111111|0|15000
aaaaaaaa-bbbb-cccc-dddd-222222222222|aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa|22222222-2222-2222-2222-222222222222|15000|30000
```

---

## Manual Testing Procedures

### Test 1: Basic Playback with Event Emission

**Objective:** Verify that position events are emitted during playback

#### Steps:

1. **Start wkmp-ap with debug logging:**

   ```bash
   cd /home/sw/Dev/McRhythm
   RUST_LOG=info,wkmp_ap=debug cargo run --package wkmp-ap
   ```

2. **In another terminal, enqueue a passage:**

   ```bash
   curl -X POST http://localhost:5740/api/v1/playback/enqueue \
     -H "Content-Type: application/json" \
     -d '{"file_path": "/path/to/your/audio/file.mp3"}'
   ```

3. **Start playback:**

   ```bash
   curl -X POST http://localhost:5740/api/v1/playback/play
   ```

4. **Monitor logs for position events:**

   **Expected Log Output (every ~1 second):**
   ```
   DEBUG wkmp_ap::playback::pipeline::mixer] Emitted PositionUpdate: passage=<uuid>, position=1000ms
   DEBUG wkmp_ap::playback::pipeline::mixer] Emitted PositionUpdate: passage=<uuid>, position=2000ms
   DEBUG wkmp_ap::playback::pipeline::mixer] Emitted PositionUpdate: passage=<uuid>, position=3000ms
   ```

**Expected Result:**
- ✅ Position events emitted every ~1000ms
- ✅ Position increments monotonically
- ✅ No errors in logs

**Actual Result:** _____________

---

### Test 2: Song Boundary Detection

**Objective:** Verify `CurrentSongChanged` events are emitted when crossing song boundaries

#### Prerequisites:
- Passage with multiple songs in `passage_songs` table
- Song timeline loaded successfully

#### Steps:

1. **Start wkmp-ap with info logging:**

   ```bash
   RUST_LOG=info cargo run --package wkmp-ap
   ```

2. **Enqueue multi-song passage:**

   ```bash
   curl -X POST http://localhost:5740/api/v1/playback/enqueue \
     -H "Content-Type: application/json" \
     -d '{
       "passage_id": "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa",
       "file_path": "/path/to/your/audio/file.mp3"
     }'
   ```

3. **Start playback:**

   ```bash
   curl -X POST http://localhost:5740/api/v1/playback/play
   ```

4. **Monitor logs for song boundary crossings:**

**Expected Log Output:**

```
INFO wkmp_ap::playback::engine] Loaded song timeline: 2 entries
INFO wkmp_ap::playback::engine] Song boundary crossed: new_song=Some(11111111-1111-1111-1111-111111111111), position=0ms
[... ~15 seconds of playback ...]
INFO wkmp_ap::playback::engine] Song boundary crossed: new_song=Some(22222222-2222-2222-2222-222222222222), position=15000ms
```

**Expected Result:**
- ✅ Initial `CurrentSongChanged` event at position 0ms with first song
- ✅ Boundary crossing detected at 15000ms (when configured)
- ✅ Second `CurrentSongChanged` event with second song ID

**Actual Result:** _____________

**Timing Verification:**
- Boundary crossing should occur within 1000ms of actual boundary
- Latency: _________ ms (target: <50ms from actual boundary)

---

### Test 3: PlaybackProgress Events

**Objective:** Verify `PlaybackProgress` events are emitted every 5 seconds

#### Steps:

1. **Start playback** (same as Test 2)

2. **Monitor logs for PlaybackProgress events:**

**Expected Log Output (every ~5 seconds):**
```
DEBUG wkmp_ap::playback::engine] PlaybackProgress: position=5000ms, duration=30000ms
DEBUG wkmp_ap::playback::engine] PlaybackProgress: position=10000ms, duration=30000ms
DEBUG wkmp_ap::playback::engine] PlaybackProgress: position=15000ms, duration=30000ms
```

**Expected Result:**
- ✅ Events emitted every ~5000ms (±100ms tolerance)
- ✅ Position and duration accurate
- ✅ Events continue until playback ends

**Actual Result:** _____________

**Timing Measurements:**
- Event 1: _________ ms
- Event 2: _________ ms (delta: _______ ms)
- Event 3: _________ ms (delta: _______ ms)

**Target:** 5000ms ± 100ms between events

---

## SSE Event Monitoring

### Setup Browser-Based SSE Monitor

#### Option A: Using Browser DevTools

1. **Open browser to wkmp-ui (if available):**
   ```
   http://localhost:5720
   ```

2. **Open DevTools (F12) → Network tab**

3. **Filter for EventSource/SSE:**
   - Look for connection to `/api/v1/events` or SSE endpoint

4. **Monitor incoming events:**

   **Expected Events:**

   ```json
   // PassageStarted
   {
     "type": "PassageStarted",
     "passage_id": "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa",
     "timestamp": "2025-10-18T12:00:00Z"
   }

   // CurrentSongChanged (initial)
   {
     "type": "CurrentSongChanged",
     "passage_id": "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa",
     "song_id": "11111111-1111-1111-1111-111111111111",
     "position_ms": 0,
     "timestamp": "2025-10-18T12:00:00Z"
   }

   // PlaybackProgress (every 5s)
   {
     "type": "PlaybackProgress",
     "passage_id": "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa",
     "position_ms": 5000,
     "duration_ms": 30000,
     "timestamp": "2025-10-18T12:00:05Z"
   }

   // CurrentSongChanged (boundary at 15s)
   {
     "type": "CurrentSongChanged",
     "passage_id": "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa",
     "song_id": "22222222-2222-2222-2222-222222222222",
     "position_ms": 15000,
     "timestamp": "2025-10-18T12:00:15Z"
   }
   ```

#### Option B: Using curl with SSE

```bash
# Subscribe to SSE endpoint
curl -N http://localhost:5740/api/v1/events

# Expected: Stream of SSE events in text/event-stream format
```

**Expected Output:**
```
data: {"type":"PassageStarted","passage_id":"...","timestamp":"..."}

data: {"type":"CurrentSongChanged","passage_id":"...","song_id":"...","position_ms":0,"timestamp":"..."}

data: {"type":"PlaybackProgress","passage_id":"...","position_ms":5000,"duration_ms":30000,"timestamp":"..."}
```

### Event Verification Checklist

| Event Type | Expected Frequency | Observed? | Notes |
|------------|-------------------|-----------|-------|
| `PassageStarted` | Once per passage | ☐ | _________ |
| `CurrentSongChanged` (initial) | Once at start | ☐ | _________ |
| `CurrentSongChanged` (boundary) | At each boundary | ☐ | _________ |
| `PlaybackProgress` | Every 5 seconds | ☐ | _________ |
| `PassageCompleted` | Once at end | ☐ | _________ |

---

## Performance Validation

### CPU Usage Measurement

#### Method 1: Using `top`

```bash
# Start wkmp-ap in background
RUST_LOG=info cargo run --package wkmp-ap &
WKMP_PID=$!

# Start playback
curl -X POST http://localhost:5740/api/v1/playback/play

# Monitor CPU for 30 seconds
for i in {1..30}; do
    top -b -n 1 -p $WKMP_PID | grep wkmp-ap | awk '{print $9}'
    sleep 1
done
```

**Expected:** CPU usage <1% during steady playback

**Measurements:**
- Peak CPU: _________%
- Average CPU: _________%
- Baseline (idle): _________%

**Target:** <1% CPU during playback

#### Method 2: Using `pidstat`

```bash
# Install sysstat if needed
sudo apt-get install sysstat

# Monitor wkmp-ap process
pidstat -p $(pgrep wkmp-ap) 1 30

# Look at %CPU column
```

**Expected Output:**
```
Average:      PID    %usr %system  %CPU   Command
Average:    12345    0.50    0.20  0.70   wkmp-ap
```

**Actual Result:** _____________

### Memory Usage

```bash
# Check memory consumption
ps aux | grep wkmp-ap | awk '{print $6}'

# Expected: Memory increase <10KB from event system
```

**Measurements:**
- Memory before playback: _________ KB
- Memory during playback: _________ KB
- Delta: _________ KB

**Target:** <10KB overhead

### Latency Measurement

**Objective:** Measure song boundary detection latency

#### Manual Timing Method:

1. **Prepare passage with known boundary at exactly 15.000 seconds**

2. **Start playback and use stopwatch:**
   - Start timer when playback begins
   - Watch logs for `CurrentSongChanged` event at boundary
   - Record exact timestamp from log

3. **Calculate latency:**
   ```
   Latency = (Event timestamp - Playback start timestamp) - 15000ms
   ```

**Expected:** <50ms latency

**Actual Latency:** _________ ms

#### Automated Timing Method:

```bash
# Grep logs with timestamps
RUST_LOG=info cargo run --package wkmp-ap 2>&1 | \
  grep -E "PassageStarted|CurrentSongChanged" | \
  awk '{print $1, $2, $NF}'

# Manually calculate delta
```

---

## Edge Case Testing

### Test 4: Empty Timeline (No Songs)

**Objective:** Verify graceful handling of passages with no songs

#### Steps:

1. **Create passage with NO entries in `passage_songs`:**

   ```sql
   DELETE FROM passage_songs WHERE passage_guid = 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa';
   ```

2. **Start playback**

3. **Expected behavior:**
   - ✅ Playback continues normally
   - ✅ No `CurrentSongChanged` events emitted
   - ✅ `PlaybackProgress` events still emitted
   - ✅ No errors in logs

**Actual Result:** _____________

---

### Test 5: Missing passage_songs Table

**Objective:** Verify graceful fallback when table doesn't exist

#### Steps:

1. **Rename `passage_songs` table:**

   ```bash
   sqlite3 ~/wkmp.db "ALTER TABLE passage_songs RENAME TO passage_songs_backup;"
   ```

2. **Restart wkmp-ap and start playback**

3. **Expected behavior:**
   - ✅ Warning logged: "passage_songs table not found"
   - ✅ Playback continues
   - ✅ No `CurrentSongChanged` events
   - ✅ No crashes or panics

4. **Restore table:**

   ```bash
   sqlite3 ~/wkmp.db "ALTER TABLE passage_songs_backup RENAME TO passage_songs;"
   ```

**Actual Result:** _____________

---

### Test 6: Seek Across Song Boundaries

**Objective:** Verify boundary detection works after seeking

#### Steps:

1. **Start playback of multi-song passage**

2. **Wait for first song to play (0-15s)**

3. **Seek to 20 seconds (second song):**

   ```bash
   curl -X POST http://localhost:5740/api/v1/playback/seek \
     -H "Content-Type: application/json" \
     -d '{"position_ms": 20000}'
   ```

4. **Expected behavior:**
   - ✅ Immediate `CurrentSongChanged` event with second song ID
   - ✅ Subsequent position events show position=20000ms+
   - ✅ No boundary crossed until reaching 30000ms (passage end)

**Actual Result:** _____________

---

### Test 7: Backward Seek

**Objective:** Verify boundary detection works with backward seeks

#### Steps:

1. **Start playback, let it reach second song (>15s)**

2. **Seek backward to 5 seconds (first song):**

   ```bash
   curl -X POST http://localhost:5740/api/v1/playback/seek \
     -H "Content-Type: application/json" \
     -d '{"position_ms": 5000}'
   ```

3. **Expected behavior:**
   - ✅ `CurrentSongChanged` event with first song ID
   - ✅ Position resets to 5000ms
   - ✅ Forward playback resumes normally

**Actual Result:** _____________

---

### Test 8: Crossfade Between Passages

**Objective:** Verify timeline reloads when crossfading to next passage

#### Steps:

1. **Enqueue two different multi-song passages**

2. **Let first passage play until crossfade starts**

3. **Expected behavior:**
   - ✅ Timeline reloads for second passage
   - ✅ Log: "Loaded song timeline: N entries"
   - ✅ Initial `CurrentSongChanged` for second passage
   - ✅ No mixing of song IDs between passages

**Actual Result:** _____________

---

## Troubleshooting Guide

### Issue: No Position Events in Logs

**Symptoms:**
- No "Emitted PositionUpdate" messages in debug logs
- No `PlaybackProgress` SSE events

**Possible Causes:**

1. **Event channel not configured**
   - Check: `mixer.set_event_channel()` called in `engine.rs:new()`
   - Fix: Verify Phase 2 implementation

2. **Playback not actually running**
   - Check: Audio output working?
   - Check: `mixer.get_current_passage_id()` returns Some()?

3. **Log level too high**
   - Fix: Use `RUST_LOG=debug` for detailed logs

**Debug Commands:**

```bash
# Check if mixer has event channel
# Add temporary debug log to mixer.rs get_next_frame():
if self.event_tx.is_some() {
    eprintln!("Event channel configured");
} else {
    eprintln!("WARNING: Event channel NOT configured");
}
```

---

### Issue: No CurrentSongChanged Events

**Symptoms:**
- Position events emitted correctly
- But no song boundary crossings detected

**Possible Causes:**

1. **passage_songs table empty or missing**
   - Check: `SELECT * FROM passage_songs WHERE passage_guid = ?`
   - Fix: Add test data (see Test Data Preparation)

2. **Timeline loading failed**
   - Check logs for: "Failed to load song timeline"
   - Check logs for: "Loaded song timeline: 0 entries"

3. **Position never crosses boundary**
   - Check: Song boundaries at correct positions?
   - Check: Passage duration long enough to reach boundary?

**Debug Commands:**

```bash
# Verify passage_songs data
sqlite3 ~/wkmp.db "SELECT passage_guid, song_guid, start_time_ms, end_time_ms
                   FROM passage_songs
                   WHERE passage_guid = '<your-passage-id>';"

# Check if timeline loaded
grep "Loaded song timeline" wkmp-ap.log
```

---

### Issue: CPU Usage >1%

**Symptoms:**
- High CPU usage during playback

**Possible Causes:**

1. **Position event interval too short**
   - Check: `position_event_interval_ms` setting
   - Recommended: 500-1000ms
   - Fix: `UPDATE settings SET value = '1000' WHERE key = 'position_event_interval_ms';`

2. **Other processes consuming CPU**
   - Check: `top` to identify culprit

3. **Debug logging enabled**
   - RUST_LOG=debug adds overhead
   - Test with RUST_LOG=info

**Performance Commands:**

```bash
# Profile with perf
perf record -g -p $(pgrep wkmp-ap)
# Play for 30 seconds
perf report
```

---

### Issue: Song Boundary Latency >50ms

**Symptoms:**
- Boundary detected, but with significant delay

**Possible Causes:**

1. **Position event interval too long**
   - Latency is bounded by interval (0 to interval_ms)
   - Fix: Decrease interval (min 100ms)

2. **Ring buffer latency**
   - Check ring buffer size settings

**Measurement:**

```bash
# Add precise timestamps to logs
RUST_LOG=info cargo run 2>&1 | ts -s '%.T'

# Calculate actual latency from timestamps
```

---

## Sign-Off Checklist

### Unit Tests

- [ ] All 106 library tests pass
- [ ] Song timeline tests (11) pass
- [ ] Passage songs tests (8) pass
- [ ] Events tests (4) pass

### Functional Tests

- [ ] **Test 1:** Position events emitted every ~1 second
- [ ] **Test 2:** Song boundaries detected correctly
- [ ] **Test 3:** PlaybackProgress events every ~5 seconds
- [ ] **Test 4:** Empty timeline handled gracefully
- [ ] **Test 5:** Missing table handled gracefully
- [ ] **Test 6:** Forward seek across boundaries works
- [ ] **Test 7:** Backward seek across boundaries works
- [ ] **Test 8:** Crossfade reloads timeline

### SSE Events

- [ ] `PassageStarted` event emitted
- [ ] Initial `CurrentSongChanged` event emitted
- [ ] Boundary `CurrentSongChanged` events emitted
- [ ] `PlaybackProgress` events emitted at correct interval
- [ ] No duplicate or missing events

### Performance

- [ ] CPU usage <1% during playback: ________%
- [ ] Memory overhead <10KB: ________KB
- [ ] Song boundary latency <50ms: ________ms
- [ ] No audio glitches or dropouts

### Edge Cases

- [ ] No passage_songs entries: Playback continues
- [ ] Missing passage_songs table: Graceful fallback
- [ ] Invalid UUIDs in table: Filtered out
- [ ] Invalid time ranges: Filtered out
- [ ] Seek during playback: Timeline updates correctly

### Configuration

- [ ] `position_event_interval_ms` loads from database
- [ ] `playback_progress_interval_ms` loads from database
- [ ] Default values used if settings missing
- [ ] Runtime changes possible (or documented as restart-required)

---

## Test Results Summary

**Test Date:** __________________

**Tester:** __________________

**Build Version:** __________________

**Overall Status:** ☐ PASS  ☐ FAIL  ☐ NEEDS REVIEW

**Critical Issues Found:** __________________

**Non-Critical Issues:** __________________

**Performance Summary:**
- CPU Usage: ________%
- Memory Usage: ________KB
- Boundary Latency: ________ms

**Recommendation:**
☐ Approved for production deployment
☐ Requires additional fixes before deployment
☐ Requires performance tuning

**Notes:**
```
[Add any additional observations, concerns, or recommendations here]




```

---

## Next Steps After Testing

### If All Tests Pass:

1. **Document Production Deployment:**
   - Update EXEC001-implementation_order.md
   - Mark event-driven architecture as complete

2. **Monitor in Production:**
   - Enable info-level logging for first 48 hours
   - Monitor CPU/memory metrics
   - Collect user feedback

3. **Cleanup:**
   - Remove deprecated code comments
   - Archive REV002 and related documents
   - Update CHANGELOG

### If Issues Found:

1. **Document Issues:**
   - Create GitHub issues for each problem
   - Include: Steps to reproduce, expected vs actual, logs

2. **Prioritize Fixes:**
   - Critical: Crashes, data loss, broken core functionality
   - High: Performance degradation, missing events
   - Medium: Edge case failures
   - Low: Cosmetic issues, non-critical warnings

3. **Implement Fixes:**
   - Follow same testing process for fixes
   - Regression test all previously passing tests

---

**End of Testing Guide**

**Status:** Ready for execution
**Next Step:** Execute Pre-Testing Setup and begin Test 1
