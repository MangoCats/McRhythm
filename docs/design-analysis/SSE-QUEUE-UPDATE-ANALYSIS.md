# Design Analysis: SSE-Based Queue Updates

**Date:** 2025-10-20
**Status:** Architectural Analysis
**Scope:** Developer UI Queue Contents Display

---

## Proposed Design Revision

### Current Implementation (Polling-Based)

**Queue Update Mechanism:**
- HTTP GET `/playback/queue` polled every 2 seconds
- SSE `/events` stream for event notifications only (not used for queue display)
- Simple request-response pattern

**Data Flow:**
```
UI (JavaScript) --[GET /playback/queue every 2s]--> API Handler
                <--[JSON response]--
```

### Proposed Implementation (SSE Push-Based)

**Queue Update Mechanism:**
- SSE `/events` stream carries full queue state updates
- On page load/refresh: API call triggers immediate SSE transmission
- No polling - purely event-driven updates

**Data Flow:**
```
Initial Load:
UI (JavaScript) --[POST /queue/subscribe]--> API Handler
                                              └─> Emit current queue via SSE

Queue Changes:
Queue Modified --[broadcast]--> SSE Stream --> All connected UIs
```

**Proposed Event Format:**
```json
{
  "event_type": "QueueContentsUpdate",
  "timestamp": "2025-10-20T17:30:00Z",
  "queue": [
    {
      "queue_entry_id": "12e7b586-fb05-44fc-8900-2deca3584445",
      "passage_id": "a1b2c3d4-...",
      "file_path": "/home/user/Music/song.mp3"
    },
    ...
  ]
}
```

---

## Benefits Analysis

### 1. ✅ Network Efficiency (Significant Improvement)

**Current (Polling):**
- Request every 2 seconds regardless of changes
- Empty queue: ~200 bytes request + ~50 bytes response = 250 bytes/2s
- 3-file queue: ~200 bytes request + ~600 bytes response = 800 bytes/2s
- **Per hour:** 1.44 MB (empty) to 1.44 MB (full queue)
- **Waste:** 100% of requests when queue unchanged (most of the time)

**Proposed (SSE):**
- Events only when queue actually changes
- Typical session: Enqueue 10 files, play through = ~10 events/hour
- **Per event:** ~800 bytes (full queue state)
- **Per hour:** ~8 KB for active session
- **Savings:** **99.4% reduction** in normal usage

**Network Comparison:**
| Scenario | Current (Polling) | Proposed (SSE) | Savings |
|----------|-------------------|----------------|---------|
| 1-hour idle session | 1.44 MB | 60 bytes (keepalive) | 99.996% |
| 1-hour active (10 changes) | 1.44 MB | 8 KB + keepalive | 99.4% |
| Multiple UIs (3 browsers) | 4.32 MB | 24 KB | 99.4% |

### 2. ✅ Real-Time Responsiveness (Major UX Improvement)

**Current (Polling):**
- Worst-case latency: 2 seconds (full polling interval)
- Average latency: 1 second
- User enqueues file → sees update in 0-2 seconds

**Proposed (SSE):**
- Latency: ~10-50ms (network + rendering time)
- User enqueues file → sees update **immediately**
- **50-200x faster** perceived responsiveness

**User Experience:**
```
Current:  [Enqueue] ----[0-2s delay]----> [Queue updates]
Proposed: [Enqueue] -[<50ms]-> [Queue updates]  ← Feels instant
```

### 3. ✅ Scalability with Multiple Clients

**Current (Polling):**
- 10 open browser tabs = 10× requests to server
- Server load: O(n) requests per interval where n = number of clients
- Each client independently polls

**Proposed (SSE):**
- 10 open browser tabs = 1 queue change event broadcast to all
- Server load: O(1) event emission, broadcast to all subscribers
- Efficient multicast pattern

**Load Comparison (10 simultaneous UIs):**
| Metric | Current | Proposed | Improvement |
|--------|---------|----------|-------------|
| Requests/minute | 300 (30/min × 10 clients) | ~1-2 (queue changes) | 150-300× reduction |
| Database reads | 300/min | ~1-2/min | Same reduction |
| CPU overhead | Medium (handle 300 req) | Low (1 event → 10 clients) | Significant |

### 4. ✅ Battery Efficiency (Mobile/Laptop)

**Current (Polling):**
- Wake up network stack every 2 seconds
- Prevents radio sleep on mobile devices
- High power consumption

**Proposed (SSE):**
- Network wakes only on actual changes
- Radio can sleep during idle periods
- **Estimated 80-90% power savings** for queue updates

### 5. ✅ Server Resource Efficiency

**Current:**
- 30 HTTP request cycles per minute (parse request, route, query queue, serialize response)
- Lock contention: 30 read locks/min on queue
- Thread pool overhead for each request

**Proposed:**
- 0 HTTP requests during idle time
- Lock contention: Only on actual queue changes (~1-2/min during playback)
- Event emission: Single broadcast operation

**Server CPU Estimate:**
| Operation | Current (per min) | Proposed (per min) | Savings |
|-----------|-------------------|-------------------|---------|
| HTTP parsing | 30 | 0-2 | ~93% |
| Queue read locks | 30 | 0-2 | ~93% |
| JSON serialization | 30 | 0-2 | ~93% |

---

## Drawbacks and Challenges

### 1. ❌ Increased Complexity (Significant)

**Current Implementation:**
- Simple `setInterval()` in JavaScript
- Standard HTTP request-response
- **Lines of code:** ~20 (frontend)

**Proposed Implementation Requires:**

**A. SSE Event Type Addition**
```rust
// In wkmp-common/src/events.rs
pub enum WkmpEvent {
    // ... existing events ...
    QueueContentsUpdate {
        timestamp: DateTime<Utc>,
        queue: Vec<QueueEntryInfo>,  // ← New: full queue array
    },
}
```

**B. Subscription Endpoint**
```rust
// New endpoint: POST /queue/subscribe
pub async fn subscribe_queue_updates(
    State(ctx): State<AppContext>,
) -> Result<StatusCode, (StatusCode, Json<StatusResponse>)> {
    // Get current queue
    let entries = ctx.engine.get_queue_entries().await;

    // Broadcast immediate update
    ctx.state.broadcast_event(WkmpEvent::QueueContentsUpdate {
        timestamp: Utc::now(),
        queue: convert_to_queue_info(entries),
    });

    Ok(StatusCode::NO_CONTENT)
}
```

**C. Modified Queue Change Logic**
Every location that modifies queue must emit full queue state:
- `enqueue_passage()` - emit full queue
- `remove_from_queue()` - emit full queue
- `clear_queue()` - emit empty queue
- `reorder_queue_entry()` - emit full queue
- `queue.advance()` in engine - emit full queue ← **New emission point**

**D. Frontend SSE Handler**
```javascript
// Much more complex than polling
eventSource.addEventListener('QueueContentsUpdate', (e) => {
    const data = JSON.parse(e.data);
    renderQueue(data.queue);
});

// Initial load trigger
async function initQueue() {
    await fetch(`${API_BASE}/queue/subscribe`, { method: 'POST' });
    // Wait for SSE event with queue data
}
```

**Complexity Metrics:**
| Aspect | Current | Proposed | Increase |
|--------|---------|----------|----------|
| Event types | 8 | 9 | +12% |
| API endpoints | 13 | 14 | +7% |
| Event emission points | 4 | 8+ | +100% |
| Frontend state management | Simple | Complex | +150% |
| Lines of code (estimate) | 50 | 200+ | +300% |

### 2. ❌ State Synchronization Issues

**Current (Polling):**
- Guaranteed eventual consistency within 2 seconds
- If SSE drops: Polling continues working
- Clear, predictable state refresh

**Proposed (SSE-Only):**

**Problem: What if SSE event is missed?**

Scenarios:
1. Network blip during queue change
2. Browser tab backgrounded (some browsers throttle SSE)
3. SSE connection temporarily drops and reconnects
4. Race condition: Page loads before subscription event sent

**Example Race Condition:**
```
T0: User loads page
T1: Page executes: POST /queue/subscribe
T2: Server emits QueueContentsUpdate event with [A, B, C]
T3: User enqueues file D
T4: Server emits QueueContentsUpdate event with [A, B, C, D]
T5: SSE event from T2 arrives (old data!) → UI shows [A, B, C]
T6: SSE event from T4 never arrives (network issue)
Result: UI stuck showing [A, B, C] when queue has [A, B, C, D]
```

**Mitigation Required:**
- Sequence numbers in events
- Client-side deduplication logic
- Periodic reconciliation (defeats the purpose!)
- Event replay capability on reconnect

**Added Complexity:**
```rust
pub struct QueueContentsUpdate {
    timestamp: DateTime<Utc>,
    sequence_number: u64,  // ← Track event order
    queue: Vec<QueueEntryInfo>,
}
```

```javascript
let lastSequence = 0;
eventSource.addEventListener('QueueContentsUpdate', (e) => {
    const data = JSON.parse(e.data);
    if (data.sequence_number <= lastSequence) {
        console.warn("Out-of-order event, ignoring");
        return;
    }
    lastSequence = data.sequence_number;
    renderQueue(data.queue);
});
```

### 3. ❌ SSE Connection Reliability

**Browser Tab Backgrounding:**
- Chrome/Safari may throttle or pause SSE when tab inactive
- Event loss possible if user switches tabs during playback

**Mobile Browsers:**
- Aggressive power management may close SSE connections
- iOS Safari particularly problematic with background SSE

**Network Issues:**
- WiFi handoff (laptop moving between access points)
- Mobile switching cell towers
- Corporate firewalls/proxies that drop long connections

**Current Polling:** Robust - if request fails, next one succeeds
**Proposed SSE:** Fragile - missed event = stale UI until next change

### 4. ❌ Debugging Difficulty

**Current (Polling):**
- Easy to inspect: Open DevTools Network tab, see requests every 2s
- Clear request/response in HAR exports
- Simple to reproduce issues: "Queue wrong after refresh"

**Proposed (SSE):**
- Event stream hard to inspect (continuous connection)
- Timing-dependent bugs difficult to reproduce
- "Did event arrive? Was it out of order? Did handler fire?"

**Debugging Session Example:**

Current:
```
DevTools Network Tab:
17:30:00 GET /playback/queue → 200 OK → {"queue": [A, B, C]}
17:30:02 GET /playback/queue → 200 OK → {"queue": [A, B, C]}
17:30:04 GET /playback/queue → 200 OK → {"queue": [A, B, C, D]}
                                          ^^^^^^^^^ Clear when change occurred
```

Proposed:
```
DevTools EventStream Tab:
[SSE connection open]
??? Event stream... hard to correlate with UI state
??? Did event for queue change arrive?
??? Was it processed?
??? Check console logs? Add custom instrumentation?
```

### 5. ❌ Memory Overhead (Server-Side)

**Current:**
- No persistent state per client
- Request → Process → Response → Forget

**Proposed:**
- Must track all connected SSE clients
- Store event subscribers in memory
- Cleanup on disconnect

**Memory Estimate (100 concurrent UIs):**
| Item | Size per Client | Total (100 clients) |
|------|-----------------|---------------------|
| SSE sender channel | ~1 KB | 100 KB |
| Client metadata | ~200 bytes | 20 KB |
| Event queue buffer | ~4 KB | 400 KB |
| **Total** | **~5 KB** | **~500 KB** |

**Comparison:**
- Current: ~0 bytes persistent (stateless)
- Proposed: ~500 KB for 100 users
- **Not huge, but non-zero** for a developer UI

### 6. ❌ Initial Load Complexity

**Current:**
```javascript
// Simple, immediate
async function loadQueue() {
    const response = await fetch('/playback/queue');
    const data = await response.json();
    renderQueue(data.queue);
}

// On page load:
loadQueue();  // ← Data available immediately
```

**Proposed:**
```javascript
// Complex, asynchronous timing
let queueReady = false;

eventSource.addEventListener('QueueContentsUpdate', (e) => {
    const data = JSON.parse(e.data);
    renderQueue(data.queue);
    queueReady = true;
});

async function initQueue() {
    await fetch('/queue/subscribe', { method: 'POST' });

    // Wait for SSE event... but how long?
    // What if event never arrives?
    setTimeout(() => {
        if (!queueReady) {
            console.error("Queue data not received!");
            // Fall back to polling? Retry? Show error?
        }
    }, 5000);
}

// On page load:
initQueue();  // ← Data arrives "eventually" via SSE
```

**Issues:**
- Race condition: SSE event may arrive before or after listener attached
- Timeout needed for error handling
- Loading state required ("Waiting for queue data...")
- Retry logic needed for failures

### 7. ❌ Payload Size on Every Change

**Current:**
- Only changed entry data sent (implicit - GET returns current state)
- Client reconciles state

**Proposed:**
- **FULL queue array sent on every change**
- Even if queue has 100 entries and only 1 changed

**Example:**
```
Queue has 50 entries (50 × 150 bytes avg = 7.5 KB)

Change: Remove 1 entry
Current:  DELETE /playback/queue/{id}  →  204 No Content (0 bytes response)
          Next poll: GET /playback/queue → 7.3 KB (49 entries)

Proposed: Remove 1 entry
          SSE event: QueueContentsUpdate with 49 entries → 7.3 KB
```

**For small queues (1-10 entries):** Negligible difference
**For large queues (50+ entries):** Same data sent, but pushed instead of pulled

**Optimization Opportunity:**
Could send **delta updates** instead of full queue:
```json
{
  "event_type": "QueueEntryRemoved",
  "queue_entry_id": "12e7b586...",
  "new_queue_length": 49
}
```

But this adds even MORE complexity (client must maintain state).

---

## Edge Cases and Error Scenarios

### 1. SSE Reconnection After Disconnect

**Scenario:** User's WiFi drops for 30 seconds during playback

**What Happens:**
- Playback continues (server-side)
- Queue advances (passage 1 → passage 2)
- SSE connection drops
- SSE reconnects after 30s
- **UI shows stale queue** (still shows passage 1)

**Current (Polling):** Next poll (within 2s of reconnect) fetches correct state
**Proposed:** No mechanism to detect staleness - UI stuck until next queue change

**Mitigation:**
- Send full queue state on SSE reconnect (requires connection state tracking)
- Add "last updated" timestamp to UI (warn user if old)
- Fallback polling every 30s as safety net (defeats design goal!)

### 2. Browser Throttling

**Scenario:** User switches to another tab for 10 minutes

**Chrome Behavior:**
- Background tabs may receive delayed SSE events
- Timers throttled to 1/minute minimum

**Impact:**
- Queue changes while tab backgrounded may not update UI
- User returns to tab: Sees stale queue

**Current:** Polling continues, state stays fresh
**Proposed:** Events may be queued/delayed by browser

### 3. Multiple Tabs Open

**Scenario:** User has 3 browser tabs with developer UI open

**Proposed Behavior:**
- Each tab has separate SSE connection
- Queue change → 3 SSE events sent (1 per tab)
- Each tab updates independently

**Challenge:**
- User enqueues file in Tab A
- Tab A makes POST /playback/enqueue
- Server broadcasts QueueContentsUpdate to all 3 tabs
- **Race:** Tab A receives own change via SSE before POST response
  - UI may flash/update twice
  - Sequencing issues

**Current:** Each tab polls independently, no race conditions

---

## Performance Benchmarks (Estimated)

### Server Load (1 user, 1-hour session)

| Metric | Current (Polling) | Proposed (SSE) | Winner |
|--------|-------------------|----------------|--------|
| HTTP requests | 1,800 | 1-10 | SSE |
| Database reads | 1,800 | 1-10 | SSE |
| JSON serializations | 1,800 | 1-10 | SSE |
| Network bytes | 1.44 MB | 8-80 KB | SSE |
| Memory (persistent) | 0 bytes | ~5 KB | Polling |
| CPU (estimated) | 0.5% | 0.01% | SSE |

### Server Load (100 users, 1-hour session)

| Metric | Current (Polling) | Proposed (SSE) | Winner |
|--------|-------------------|----------------|--------|
| HTTP requests | 180,000 | 100-1,000 | SSE |
| Database reads | 180,000 | 100-1,000 | SSE |
| Broadcast operations | 0 | 100-1,000 | Polling |
| Network bytes | 144 MB | 0.8-8 MB | SSE |
| Memory (persistent) | 0 KB | 500 KB | Polling |
| CPU (estimated) | 50% | 1-5% | SSE |

**Conclusion:** SSE wins dramatically at scale, but adds memory overhead.

---

## Implementation Effort Estimate

### Current System Modifications Required

| Task | Complexity | Est. Time |
|------|------------|-----------|
| Add QueueContentsUpdate event type | Low | 30 min |
| Add /queue/subscribe endpoint | Low | 30 min |
| Modify all queue change points to emit full state | Medium | 2 hours |
| Update SSE handler to include queue data | Low | 30 min |
| Rewrite frontend queue display logic | Medium | 2 hours |
| Add sequence number tracking | Medium | 1 hour |
| Add reconnection handling | High | 2 hours |
| Add fallback/error handling | Medium | 1 hour |
| Testing (edge cases, timing, errors) | High | 4 hours |
| Documentation | Low | 1 hour |
| **Total** | | **14.5 hours** |

### Ongoing Maintenance Burden

**Current:**
- Simple polling logic, rarely needs changes
- Bugs easy to reproduce and fix

**Proposed:**
- Complex event-driven system
- Timing-dependent bugs
- More edge cases to handle
- Requires expertise in SSE, event-driven architecture

**Estimated Maintenance Increase:** +50% for queue-related issues

---

## Alternative Hybrid Approaches

### Option A: SSE + Polling Fallback

**Design:**
- Primary: SSE events for queue updates
- Fallback: Poll every 30 seconds as safety net
- On SSE disconnect: Increase poll frequency to 5s until reconnect

**Benefits:**
- Best of both worlds: Speed + Reliability
- Graceful degradation

**Drawbacks:**
- Complexity of both systems
- Still polling (reduced benefit)

### Option B: SSE + Smart Polling

**Design:**
- Use SSE for notifications ("queue changed!")
- On notification: Poll `/playback/queue` to get current state
- Decouples event delivery from data delivery

**Benefits:**
- Simpler event format (just notification, no full queue)
- Polling ensures consistency
- Fast updates when queue changes

**Drawbacks:**
- Still requires polling endpoint
- Extra request on each change

### Option C: SSE with Sequence Numbers + Periodic Reconciliation

**Design:**
- SSE for updates (with sequence numbers)
- Poll every 60 seconds to verify sequence continuity
- If gap detected, poll to reconcile

**Benefits:**
- Detect missed events
- Self-healing

**Drawbacks:**
- Complex sequence tracking
- Still need polling infrastructure

---

## Recommendations

### For Developer UI (Current Use Case)

**KEEP CURRENT POLLING DESIGN**

**Rationale:**
1. **Developer UI is low-traffic** - 1-5 concurrent users max
   - Network savings: 1.44 MB/hour → negligible for modern networks
   - Server load: 30 req/min → trivial for any server

2. **Simplicity is valuable** - Developer tools should be reliable
   - Polling is dead simple to debug
   - No timing issues, race conditions, or state sync problems

3. **Not worth the complexity** - 14.5 hours implementation + ongoing maintenance
   - For ~1 MB/hour savings per user
   - For 50ms latency improvement (user won't notice 1s vs 50ms for dev UI)

4. **Current design works well** - No user complaints about queue display

### For Production Multi-User UI (Future)

**CONSIDER SSE-BASED DESIGN IF:**

1. **Scale requirements change:**
   - 1,000+ concurrent users
   - Network bandwidth costs significant
   - Server load becomes bottleneck

2. **Real-time critical:**
   - Multiple users collaborating on same queue
   - Sub-100ms updates essential for UX

3. **Mobile app:**
   - Battery life critical
   - Push notifications already in infrastructure

**Implementation Strategy:**
- Start with Option B (SSE notifications + poll for data)
- Migrate to full SSE if proven necessary
- Keep polling as documented fallback

---

## Architectural Best Practices Analysis

### REST vs Event-Driven Architectures

**Polling (Current) = REST Pattern:**
- Client-driven: "I want data when I want it"
- Stateless server
- Simple, proven, standardized

**SSE (Proposed) = Event-Driven Pattern:**
- Server-driven: "I'll tell you when things change"
- Stateful connections
- Complex, requires careful design

**Industry Practice:**
- REST polling: Gmail, most web apps (simple, reliable)
- SSE/WebSockets: Slack, Discord, trading platforms (real-time critical)
- **Hybrid:** Many modern apps (Twitter, GitHub) - SSE for notifications, polling for data

### When to Use Each

**Use Polling When:**
- Data changes infrequently (<10 changes/hour)
- Latency tolerance: 1-5 seconds acceptable
- Simplicity valued over efficiency
- Low concurrent user count (<100)
- **← Developer UI matches this profile**

**Use SSE When:**
- Data changes frequently (>10 changes/minute)
- Latency critical (<100ms required)
- High concurrent users (>1,000)
- Network efficiency critical (mobile, high-bandwidth data)

---

## Conclusion

### For Current Developer UI: ❌ **DO NOT IMPLEMENT**

**Recommendation:** **Keep current polling-based design**

**Why:**
1. ✅ **Current design works well** - No user complaints, meets requirements
2. ✅ **Simplicity valuable for dev tools** - Easy to debug and maintain
3. ✅ **Network overhead negligible** - 1.44 MB/hour for 1 user
4. ✅ **Server load trivial** - 30 req/min easily handled
5. ❌ **14.5 hours implementation cost** - Not justified for marginal benefit
6. ❌ **Increased complexity** - More bugs, harder maintenance
7. ❌ **No compelling user benefit** - Users won't notice 1s vs 50ms latency

### For Future Production UI: ⚠️ **MAYBE**

**Recommendation:** **Revisit if scale/requirements change**

**Triggers to Reconsider:**
- 100+ concurrent users regularly
- Network costs become significant (>$100/month for queue polling)
- Real-time collaboration features needed
- Mobile app with battery concerns
- Users complain about queue update latency

**If Implemented Later:**
- Use **Option B** (SSE notifications + poll for data) as starting point
- Add sequence numbers and reconnection handling from day 1
- Keep polling as fallback for reliability
- Extensive testing of edge cases before production

---

## Cost-Benefit Summary

| Aspect | Benefit | Cost | Worth It? |
|--------|---------|------|-----------|
| Network savings | 99.4% reduction | 14.5 hours + complexity | ❌ No (for dev UI) |
| Latency improvement | 50ms vs 1s | Reliability/debugging issues | ❌ No (not noticeable) |
| Server CPU | 93% reduction | Memory overhead + state management | ❌ No (not bottleneck) |
| Scalability | Better at 1,000+ users | Works worse at 1-10 users | ❌ No (current scale) |
| User experience | Slightly faster updates | Potential for stale state | ❌ No (worse UX risk) |
| Code quality | More "modern" architecture | Higher complexity, more bugs | ❌ No (worse quality) |

**Overall Verdict:** ❌ **Not recommended for developer UI**

The proposed SSE-based design is technically interesting and offers theoretical benefits at scale, but introduces significant complexity and reliability concerns that outweigh the marginal efficiency gains for the developer UI use case. The current polling-based design is simpler, more reliable, and perfectly adequate for the expected usage patterns.

**Keep it simple. Polling works.**
