# Buffer Fill Percentage Display Issue - Root Cause Analysis

**Issue:** Buffer fill percentages "freezing" at intermediate values (73.7%, 86.9%, 54.2%) instead of showing expected behavior per SPEC016
**Date:** 2025-10-20
**Reporter:** User observation during manual testing
**Related Requirements:** [DBD-BUF-060], [DBD-BUF-050], [DBD-PARAM-070]

---

## Expected Behavior per SPEC016

**[DBD-BUF-050]:** Buffers should fill to nearly full:
- playout_ringbuffer_size = 661,941 samples
- playout_ringbuffer_headroom = 441 samples
- Maximum fill = 661,941 - 441 = 661,500 samples
- Expected buffer fill percentage: **~99.9%** when decoder paused

**[DBD-BUF-060]:** When passage ends, buffer should drain to **0%** as samples are consumed by mixer

**[DBD-PARAM-070]:** 15.01 seconds of audio at 44.1kHz per buffer

**Expected Timeline:**
1. Passage enqueued → Buffer empty (0%)
2. Decoder fills buffer → Buffer fills to ~99.9% (decoder pauses per [DBD-BUF-050])
3. During playback → Buffer drains gradually as mixer consumes samples
4. Passage completion → Buffer reaches 0%

---

## Observed Behavior

Buffer fill percentages displayed in developer UI are "frozen" at values like:
- Chain 0: 73.7%
- Chain 1: 86.9%
- Chain 2: 54.2%

Values do not update in real-time as expected, even though underlying buffers are filling/draining.

---

## Root Cause Analysis

### Investigation Summary

Examined the BufferChainStatus SSE emission system in:
- `wkmp-ap/src/playback/engine.rs:2268-2309` (buffer_chain_status_emitter)
- `wkmp-common/src/events.rs:118-164` (BufferChainInfo struct)

### Root Cause 1: Unreliable Change Detection with f32 Fields

**File:** `wkmp-common/src/events.rs:118`

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BufferChainInfo {
    // ... other fields ...
    pub buffer_fill_percent: f32,  // LINE 153
    // ... other fields ...
}
```

**File:** `wkmp-ap/src/playback/engine.rs:2294-2297`

```rust
// Only emit if data has changed
let should_emit = match &last_chains {
    None => true, // First emission
    Some(prev) => prev != &current_chains, // Data changed - PROBLEM HERE
};
```

**Problem:** The emitter uses `prev != &current_chains` to detect changes. This performs a **full structural PartialEq comparison** including all f32 fields. Floating-point equality is unreliable:

1. **Floating-point precision issues**: Two visually identical percentages (e.g., both "73.7%") might differ in the last decimal places (73.69998 vs 73.70002), causing the equality check to succeed when it should fail
2. **Rounding artifacts**: Buffer fill percentage calculation may produce slightly different f32 values each time even when the underlying fill level hasn't meaningfully changed
3. **No significant change threshold**: A change from 73.699% to 73.701% is detected as "different" but rounds to the same display value

**Result:** Change detection becomes unreliable. Updates may be sent for insignificant changes, OR may fail to send when significant changes occur.

### Root Cause 2: Missing Playback Position Updates

**File:** `wkmp-ap/src/playback/engine.rs:1012`

```rust
buffer_fill_percent: buffer_info.as_ref().map(|b| b.fill_percent).unwrap_or(0.0),
```

Buffer fill percentage comes from `buffer_info.fill_percent` queried from BufferManager.

**Additional contributing factors:**
- `playback_position_frames` field (line 1017) is ONLY updated for queue_position 0 or 1 during crossfade
- Other dynamic fields (decoder_state, decode_progress_percent, fade_stage) are stubbed as `None` (lines 998-1008)
- Many fields are static or change infrequently

**Result:** If `buffer_fill_percent` remains constant due to paused decoder AND other fields don't change, the entire struct comparison returns `true` (equal), preventing any emission.

---

## Proposed Solutions

### Solution 1: Always Emit Buffer Chain Status (Simplest)

**Change:** Remove the change detection entirely, always emit every 1 second

**Implementation:**
```rust
// File: wkmp-ap/src/playback/engine.rs:2290-2307

async fn buffer_chain_status_emitter(&self) {
    use tokio::time::interval;
    use std::time::Duration;

    info!("BufferChainStatus emitter started");
    let mut tick = interval(Duration::from_secs(1));

    loop {
        tick.tick().await;

        if !*self.running.read().await {
            info!("BufferChainStatus emitter stopping");
            break;
        }

        // Get current buffer chain status
        let current_chains = self.get_buffer_chains().await;

        // Always emit BufferChainStatus event every 1 second
        self.state.broadcast_event(wkmp_common::events::WkmpEvent::BufferChainStatus {
            timestamp: chrono::Utc::now(),
            chains: current_chains,
        });
    }
}
```

**Pros:**
- Simple fix (remove ~10 lines of code)
- Guaranteed real-time updates every 1 second
- No dependency on change detection logic
- Easier to debug (predictable emission schedule)

**Cons:**
- Slightly higher SSE bandwidth (sends same data even when unchanged)
- ~12 chains × 20 fields × 1 Hz = still modest data rate for local network

**Impact:** Minimal. SSE already sends PlaybackPosition every 1 second unconditionally (line 2311-2356). BufferChainStatus has similar data volume.

### Solution 2: Implement Smart Change Detection (More Complex)

**Change:** Detect "significant" changes using threshold-based comparison for f32 fields

**Implementation:**
```rust
impl BufferChainInfo {
    /// Check if this buffer chain info has changed significantly from another
    /// Uses threshold-based comparison for floating-point fields
    fn has_significant_change(&self, other: &Self) -> bool {
        const PERCENT_THRESHOLD: f32 = 0.5; // 0.5% change threshold

        // Check non-float fields for exact equality
        if self.slot_index != other.slot_index
            || self.queue_entry_id != other.queue_entry_id
            || self.passage_id != other.passage_id
            || self.queue_position != other.queue_position
            || self.buffer_state != other.buffer_state
            || self.mixer_role != other.mixer_role
            || self.is_active_in_mixer != other.is_active_in_mixer
        {
            return true;
        }

        // Check float field with threshold
        if (self.buffer_fill_percent - other.buffer_fill_percent).abs() > PERCENT_THRESHOLD {
            return true;
        }

        // Check sample counts (significant if changed by >1000 samples)
        if (self.buffer_fill_samples as i64 - other.buffer_fill_samples as i64).abs() > 1000 {
            return true;
        }

        // Check playback position (significant if changed by >44100 frames = 1 second)
        if (self.playback_position_frames as i64 - other.playback_position_frames as i64).abs() > 44100 {
            return true;
        }

        false
    }
}

// In buffer_chain_status_emitter:
let should_emit = match &last_chains {
    None => true,
    Some(prev) => {
        // Check if ANY chain has significant changes
        current_chains.iter().enumerate().any(|(i, chain)| {
            prev.get(i).map(|prev_chain| chain.has_significant_change(prev_chain)).unwrap_or(true)
        })
    }
};
```

**Pros:**
- Reduces SSE bandwidth (only sends when meaningful changes occur)
- More sophisticated change detection
- Prevents noise from floating-point precision issues

**Cons:**
- More complex implementation (~60 lines of code)
- Requires careful threshold tuning
- May still miss edge cases

### Solution 3: Hybrid Approach (Recommended)

**Change:** Always emit, but add optional rate limiting via database parameter

**Implementation:**
```rust
// Add to settings table:
// buffer_chain_status_interval_ms (default: 1000)

async fn buffer_chain_status_emitter(&self) {
    // ... read interval from database settings ...
    let interval_ms = self.db.get_setting("buffer_chain_status_interval_ms").await
        .unwrap_or(1000);

    let mut tick = interval(Duration::from_millis(interval_ms));

    loop {
        tick.tick().await;
        // ... always emit, no change detection ...
    }
}
```

**Pros:**
- Simple implementation (Solution 1)
- Configurable update rate for advanced users
- Future-proof (can add change detection later if needed)

**Cons:**
- Adds database parameter (minor complexity)

---

## Recommendation

**Implement Solution 1: Always Emit**

Rationale:
1. **Simplest fix** - removes problematic change detection code
2. **Guaranteed updates** - user will see real-time buffer fill changes every 1 second as requested
3. **Minimal performance impact** - BufferChainStatus data volume is modest (~1-2KB JSON per update)
4. **Consistent with PlaybackPosition** - which already emits unconditionally every 1 second
5. **Developer UI use case** - This SSE endpoint is primarily for development/debugging, not production user-facing UI

Later, if bandwidth becomes an issue, we can:
- Add Solution 3's configurable interval
- Implement Solution 2's smart change detection
- Add SSE client-side filtering

---

## Implementation Plan

### Phase 1: Remove Change Detection (Immediate Fix)

**File:** `wkmp-ap/src/playback/engine.rs:2268-2309`

1. Remove `last_chains` variable (line 2279)
2. Remove change detection logic (lines 2293-2297)
3. Always emit BufferChainStatus event
4. Update comment to reflect "emits every 1 second" (not "when changed")

**Testing:**
- Start wkmp-ap with multiple passages enqueued
- Open developer UI
- Observe buffer fill percentages update every 1 second
- Verify buffers fill to ~99% then drain to 0% during playback

### Phase 2: Verify Buffer Manager Updates (If Still Frozen)

If buffer fill percentages STILL don't update after Phase 1, investigate BufferManager:

1. Check if `buffer_info.fill_percent` is being calculated correctly
2. Verify BufferManager is polling buffer state frequently enough
3. Add debug logging to track buffer fill changes

**File to examine:** `wkmp-ap/src/buffer_manager.rs` (or equivalent)

### Phase 3: Add Configurable Interval (Optional Enhancement)

**File:** `docs/IMPL001-database_schema.md`

Add to settings table:
```sql
-- Buffer chain status update interval (milliseconds)
-- Default: 1000ms (1 second)
-- **[DBD-PARAM-110]** buffer_chain_status_interval_ms
INSERT INTO settings (key, value) VALUES ('buffer_chain_status_interval_ms', '1000')
```

---

## Testing Checklist

After implementing Phase 1:

- [ ] Enqueue 3 passages with different durations
- [ ] Open developer UI buffer chain monitor
- [ ] Verify buffer fill percentages update every ~1 second
- [ ] Verify Chain 0 (now playing) shows:
  - Initial fill to ~99%
  - Gradual drain during playback
  - Reaches 0% near passage completion
- [ ] Verify Chains 1-2 (queued) show:
  - Fill to ~99%
  - Remain at ~99% until they become "now playing"
- [ ] Verify idle chains (3-11) show 0%
- [ ] Check SSE event log shows BufferChainStatus events every 1 second
- [ ] Verify browser network tab shows acceptable SSE bandwidth

---

## Expected Outcome

After implementing Solution 1, users should observe:

1. **Real-time updates**: Buffer fill percentages update every 1 second
2. **Correct lifecycle**:
   - Empty (0%) → Filling → Full (~99%) → Draining → Empty (0%)
3. **No freezing**: Values continuously update to reflect current buffer state
4. **Predictable behavior**: Updates occur on 1-second intervals regardless of change magnitude

---

## Related Issues

This investigation revealed several stubbed fields in BufferChainInfo that could provide additional monitoring value:

- `decoder_state: Option<DecoderState>` - Currently `None` (line 998)
- `decode_progress_percent: Option<u8>` - Currently `None` (line 999)
- `source_sample_rate: Option<u32>` - Currently `None` (line 1003)
- `fade_stage: Option<FadeStage>` - Currently `None` (line 1008)
- `started_at: Option<String>` - Currently `None` (line 1022)

Future enhancement: Populate these fields to provide complete pipeline visibility per [DBD-OV-040].

---

**Document Status:** Root Cause Analysis Complete
**Next Action:** Implement Phase 1 - Remove Change Detection
**Estimated Effort:** 15 minutes (code change + testing)
