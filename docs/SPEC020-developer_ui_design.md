# SPEC008: Audio Player Developer UI Design

**Document Classification:** Tier 2 (Design Specification)
**Status:** Active
**Version:** 1.0
**Last Updated:** 2025-10-20

---

## Document Control

### Purpose
This document specifies the design of the Audio Player (wkmp-ap) Developer UI, a diagnostic interface for monitoring internal audio processing pipeline state in real-time.

### Scope
- Buffer chain monitor table layout and data presentation
- Module status panel design
- Playback controls interface
- Queue contents display
- Real-time Server-Sent Events (SSE) integration

### Related Documents
- **[GOV001]** Document Hierarchy (governance framework)
- **[SPEC016]** Decoder-Buffer Design (buffer chain architecture)
- **[SPEC007]** API Design (SSE event definitions)
- **[REQ001]** Requirements (user interface requirements)

### Change Control
Changes to this document must be approved via formal change control process per GOV001.

---

## 1. Overview

### 1.1 Purpose
The Developer UI provides real-time visibility into the wkmp-ap audio processing pipeline for debugging, performance monitoring, and system diagnostics.

### 1.2 Design Principles
1. **Clarity over aesthetics** - Information density prioritized for diagnostic use
2. **Real-time updates** - All data refreshed via Server-Sent Events (SSE)
3. **Minimal user interaction** - Primarily read-only monitoring interface
4. **Uniform presentation** - Consistent table-based layout for structured data
5. **No production dependency** - Developer-only interface, not for end users

### 1.3 Technology Stack
- **Transport:** Server-Sent Events (SSE) over HTTP
- **Frontend:** Vanilla JavaScript (no framework dependencies)
- **Styling:** Inline CSS (self-contained single-file HTML)
- **Data Format:** JSON (SSE event payloads)

---

## 2. Buffer Chain Monitor

### 2.1 Purpose
**[SPEC016-MONITOR-010]** The buffer chain monitor displays the state of all N decoder-resampler-fade-buffer chains (where N = `maximum_decode_streams` parameter, default 12).

**Traceability:** Satisfies [DBD-OV-040] Full pipeline visibility requirement.

### 2.2 Layout: Table-Based Design

**[SPEC016-MONITOR-020]** The buffer chain monitor MUST use a **uniform table layout** with one row per chain.

**Rationale:** Table format provides:
- Consistent visual scanning pattern (top-to-bottom)
- Easy comparison across chains
- Compact representation of all N chains simultaneously
- Clear column-based data grouping

**Anti-Pattern:** Card-based layouts with variable heights create visual noise and make cross-chain comparison difficult.

### 2.3 Table Structure

**[SPEC016-MONITOR-030]** The buffer chain monitor table MUST have exactly 7 columns in the following order:

| Column | Header | Data Type | Description |
|--------|--------|-----------|-------------|
| 1 | Chain # | Integer (0-based) | Static chain index (0 to N-1) |
| 2 | Queue Pos | Integer or N/A | Queue position or idle state |
| 3 | Decoder | String | Decoder diagnostic information |
| 4 | Resample | String or N/A | Resampler diagnostic information |
| 5 | Fade | String | Fade stage and curve information |
| 6 | Buffer | Percentage | Buffer fill percentage |
| 7 | Mixer | String | Mixer feed status |

### 2.4 Column Specifications

#### 2.4.1 Column 1: Chain Number

**[SPEC016-MONITOR-040]** Chain number MUST be:
- **Static:** Chain indices 0 through (N-1) always displayed in ascending order
- **0-indexed:** First chain is 0, last chain is (N-1)
- **Always visible:** All N rows present even if idle

**Example:** For `maximum_decode_streams=12`, rows show chain numbers 0, 1, 2, ..., 11

**Rationale:** Static chain numbers provide stable reference points for debugging. Chain 0 is not special; queue position determines role.

#### 2.4.2 Column 2: Queue Position

**[SPEC016-MONITOR-050]** Queue position MUST display:

| Value | Meaning | Visual Treatment |
|-------|---------|------------------|
| N/A | Chain is idle (no passage assigned) | Gray text |
| 0 | Now Playing (current passage) | **Bold green** |
| 1 | Up Next (next passage) | **Bold yellow** |
| 2, 3, 4, ... | Queued passages (pre-buffering) | Normal blue text |

**Traceability:**
- [DBD-OV-060] Position 0 = "now playing"
- [DBD-OV-070] Position 1 = "up next"
- [DBD-OV-080] Passage-based association

**Important:** Queue position is **passage-centric**, not chain-centric. As the queue advances, a passage moves from position 2 → 1 → 0, potentially changing which chain it occupies.

#### 2.4.3 Column 3: Decoder Status

**[SPEC016-MONITOR-060]** Decoder status MUST display:

**Primary state:**
- `Idle` - No decode task assigned
- `Decoding` - Actively decoding samples
- `Paused` - Decode suspended (buffer full or lower priority)

**Diagnostic information (when available):**
- Decode progress percentage (0-100%)
- Current sample position / total samples
- Decode priority (Current, Next, Prefetch)
- Worker thread ID

**Example formats:**
```
Idle
Decoding 45% (1234567/2750000) [Priority: Current, Worker: 2]
Paused (buffer full)
```

**Traceability:** [DBD-DEC-030] Decoder pauses when buffer full

**Stubbing:** If decoder status tracking is not yet implemented, display `N/A` or `[Status unavailable]`

#### 2.4.4 Column 4: Resample Status

**[SPEC016-MONITOR-070]** Resample status MUST display:

**When source sample rate matches working_sample_rate (default 44100 Hz):**
- Display: `N/A` (no resampling needed)

**When resampling active:**
- Source sample rate (Hz)
- Target sample rate (always 44100 Hz per [DBD-PARAM-020])
- Resample ratio
- Resampler algorithm (if available)

**Example formats:**
```
N/A
48000 Hz → 44100 Hz (ratio: 0.91875) [Linear interpolation]
96000 Hz → 44100 Hz (ratio: 0.459375) [Sinc]
```

**Traceability:** [DBD-RSMP-010] Automatic sample rate conversion

**Stubbing:** If resample status unavailable, display `N/A`

#### 2.4.5 Column 5: Fade Status

**[SPEC016-MONITOR-080]** Fade status MUST display:

**Primary fade stage:**
- `PreStart` - Discarding samples before passage start time [DBD-FADE-020]
- `FadeIn` - Applying fade-in curve [DBD-FADE-030]
- `Body` - Passthrough (no fade applied) [DBD-FADE-040]
- `FadeOut` - Applying fade-out curve [DBD-FADE-050]
- `PostEnd` - After passage end (decode may continue briefly) [DBD-FADE-060]

**Diagnostic information:**
- Fade curve type (linear, exponential, exponential_logarithmic, etc.)
- Fade progress percentage (for FadeIn/FadeOut stages)
- End time signal status (has fader signaled decoder "end time" yet?)

**Example formats:**
```
PreStart
FadeIn 30% [exponential]
Body
FadeOut 75% [exponential_logarithmic] (end signaled)
PostEnd
```

**Traceability:** [DBD-FADE-010] through [DBD-FADE-060]

**Stubbing:** If fade status unavailable, display current stage only (e.g., `Body`)

#### 2.4.6 Column 6: Buffer Fill

**[SPEC016-MONITOR-090]** Buffer fill MUST display:

**Percentage format:**
- `0%` - Empty buffer
- `50%` - Half full
- `100%` - Full buffer

**Visual indicators:**
- < 25%: Red background (underrun risk)
- 25-75%: Normal
- > 75%: Green background (healthy)

**Additional diagnostics (optional):**
- Absolute sample counts (fill_samples / capacity_samples)
- Buffer state (Empty, Filling, Ready, Playing, Finished)
- Headroom in seconds

**Example formats:**
```
45% (123456/274000 samples) [Playing]
0% [Idle]
100% (Paused)
```

**Traceability:** [DBD-BUF-040] Buffer state machine

#### 2.4.7 Column 7: Mixer Status

**[SPEC016-MONITOR-100]** Mixer status MUST display:

**Feed status:**
- `Idle` - Not feeding mixer
- `Feeding` - Actively feeding mixer (current or crossfading)

**Mixer role:**
- `Current` - Primary audio source (position 0)
- `Crossfading` - Secondary source during crossfade
- `Idle` - Not in mixer

**Pause behavior:**
**[SPEC016-MONITOR-110]** When playback state is **Paused**, ALL chains MUST show mixer status as `Idle`.

**Example formats:**
```
Idle
Feeding [Current]
Feeding [Crossfading with chain 3]
```

**Traceability:** Mixer pipeline stage visibility

---

## 3. Module Status Panel

### 3.1 Purpose
**[SPEC016-MODULE-010]** Display wkmp-ap module operational status and metadata.

### 3.2 Layout
**[SPEC016-MODULE-020]** Module status MUST use a compact panel layout with key-value pairs.

### 3.3 Data Elements

**Required fields:**
1. **Module Name:** "WKMP Audio Player"
2. **Version:** Build version from Cargo.toml
3. **Build Info:** Commit hash, build date (from compile-time env vars)
4. **Playback State:** Playing, Paused, Stopped
5. **Current Position:** Playback position (mm:ss / mm:ss format)
6. **Volume:** Current volume level (0-100%)
7. **Audio Device:** Output device name
8. **SSE Connection:** Connected / Disconnected status

**Example:**
```
WKMP Audio Player v0.1.0
Build: abc1234 (2025-10-20 14:35:22 UTC)
State: Playing
Position: 02:34 / 04:12
Volume: 75%
Device: Default Output
SSE: Connected
```

### 3.4 Update Frequency
- **Playback State:** Immediate on change (SSE: PlaybackStateChanged)
- **Position:** Every 1 second (SSE: PlaybackPosition)
- **Volume:** Immediate on change (SSE: VolumeChanged)
- **SSE Connection:** Immediate on connect/disconnect

---

## 4. Playback Controls

### 4.1 Purpose
**[SPEC016-CONTROLS-010]** Provide basic playback control for testing and debugging.

### 4.2 Control Set

**Minimum controls:**
1. **Play** - Start/resume playback
2. **Pause** - Pause playback (maintains position)
3. **Stop** - Stop playback (resets position)
4. **Skip** - Advance to next passage

**Optional controls:**
5. Volume slider (0-100%)
6. Seek bar (if position seeking supported)

### 4.3 API Bindings

**[SPEC016-CONTROLS-020]** Controls MUST map to wkmp-ap REST API endpoints:

| Control | HTTP Method | Endpoint |
|---------|-------------|----------|
| Play | POST | /api/playback/play |
| Pause | POST | /api/playback/pause |
| Stop | POST | /api/playback/stop |
| Skip | POST | /api/playback/skip |
| Set Volume | POST | /api/playback/volume |

**Error Handling:** Display error messages from API responses inline.

---

## 5. Queue Contents

### 5.1 Purpose
**[SPEC016-QUEUE-010]** Display current playback queue for verification and debugging.

### 5.2 Layout
**[SPEC016-QUEUE-020]** Queue MUST use a table layout with one row per queue entry.

### 5.3 Columns

| Column | Header | Description |
|--------|--------|-------------|
| 1 | Position | Queue position (0 = now playing, 1 = up next, 2+ = queued) |
| 2 | Passage ID | UUID of queue entry |
| 3 | File Path | Audio file path (basename only for readability) |

**Optional columns:**
4. Duration (if available)
5. Start/End times (if passage has time bounds)

### 5.4 Visual Indicators

**[SPEC016-QUEUE-030]** Visual treatment:
- Position 0: **Bold green** (now playing)
- Position 1: **Bold yellow** (up next)
- Position 2+: Normal text

### 5.5 Interactive Features

**[SPEC016-QUEUE-040]** Queue entries SHOULD support:
1. **Remove** - Delete entry from queue (DELETE /api/queue/{id})
2. **Clear All** - Clear entire queue (DELETE /api/queue)

---

## 6. SSE Integration

### 6.1 Event Types

**[SPEC016-SSE-010]** Developer UI MUST subscribe to the following SSE events:

| Event Type | Update Trigger | Target UI Component |
|------------|----------------|---------------------|
| `BufferChainStatus` | Every 1s (when data changes) | Buffer Chain Monitor |
| `PlaybackStateChanged` | State transitions | Module Status Panel |
| `PlaybackPosition` | Every 1s during playback | Module Status Panel |
| `QueueStateUpdate` | Queue modifications | Queue Contents |
| `VolumeChanged` | Volume adjustments | Module Status Panel |
| `InitialState` | SSE connection established | All components |

**Traceability:** [SSE-UI-010] through [SSE-UI-050] (SPEC007 API Design)

### 6.2 Connection Management

**[SPEC016-SSE-020]** SSE connection handling:

1. **Auto-connect on page load**
2. **Display connection status** in Module Status Panel
3. **Auto-reconnect** on disconnect (exponential backoff: 1s, 2s, 4s, 8s, max 30s)
4. **Error display** if connection fails after 5 attempts

### 6.3 Event Processing

**[SPEC016-SSE-030]** Event processing rules:

1. **Parse JSON** from SSE data field
2. **Validate event type** matches expected schema
3. **Update UI atomically** (batch DOM updates if needed)
4. **Log errors** to browser console (do not crash UI)

---

## 7. Responsive Design

### 7.1 Minimum Viewport

**[SPEC016-LAYOUT-010]** Developer UI MUST support minimum viewport width of **1280px**.

**Rationale:** Developer interface prioritizes information density over mobile responsiveness.

### 7.2 Scrolling Behavior

**[SPEC016-LAYOUT-020]** Scrolling:
- **Buffer Chain Monitor:** Vertical scroll if N > 20 chains
- **Queue Contents:** Vertical scroll if queue > 10 entries
- **No horizontal scroll** required for standard 7-column table

---

## 8. Performance Requirements

### 8.1 Update Latency

**[SPEC016-PERF-010]** UI update latency targets:

| Metric | Target | Measurement |
|--------|--------|-------------|
| SSE event to DOM update | < 50ms | Client-side JS timing |
| Buffer chain table refresh | < 100ms | For N=12 chains |
| Queue table refresh | < 100ms | For 50 entries |

### 8.2 Memory Footprint

**[SPEC016-PERF-020]** Client-side memory:
- Maximum: 50 MB for page + SSE buffers
- EventSource buffer: Auto-managed by browser

### 8.3 CPU Usage

**[SPEC016-PERF-030]** Client-side CPU:
- < 5% average during idle (SSE connected, no updates)
- < 15% average during active playback (1Hz updates)

---

## 9. Implementation Notes

### 9.1 Technology Constraints

**[SPEC016-IMPL-010]** Implementation constraints:

1. **Single HTML file** - All CSS/JS inline (no external dependencies)
2. **Vanilla JavaScript** - No frameworks (React, Vue, etc.)
3. **No build step** - HTML file served directly by wkmp-ap
4. **No persistent state** - Refresh clears all client-side data

**Rationale:** Minimize deployment complexity; developer UI is diagnostic tool, not production interface.

### 9.2 Browser Compatibility

**[SPEC016-IMPL-020]** Minimum browser support:

- Chrome/Edge 90+ (Chromium-based)
- Firefox 88+
- Safari 14+

**Required features:**
- EventSource API (SSE)
- Fetch API
- ES6 JavaScript (arrow functions, template literals, async/await)

### 9.3 Accessibility

**[SPEC016-IMPL-030]** Accessibility is **not required** for developer UI.

**Rationale:** Developer-only diagnostic interface; accessibility adds complexity without user benefit.

---

## 10. Future Enhancements

### 10.1 Deferred Features

**[SPEC016-FUTURE-010]** Features deferred to future versions:

1. **Decoder status tracking** - Full visibility into decoder worker state
2. **Resample diagnostics** - Real-time resample ratio and algorithm info
3. **Fade curve visualization** - Graphical representation of fade curves
4. **Buffer waveform preview** - Mini waveform display per chain
5. **Performance graphs** - Historical CPU/memory/latency charts

### 10.2 Export Capabilities

**[SPEC016-FUTURE-020]** Potential export features:

1. **CSV export** - Buffer chain state snapshots
2. **JSON dump** - Full pipeline state for offline analysis
3. **Screenshot capture** - Save current UI state as image

---

## 11. Validation Criteria

### 11.1 Visual Validation

**[SPEC016-VALIDATE-010]** Manual validation checklist:

- [ ] All N chains visible in table (0 to N-1)
- [ ] Queue position colors correct (green=0, yellow=1, blue=2+)
- [ ] Idle chains show "N/A" in appropriate columns
- [ ] Buffer fill percentages update in real-time
- [ ] Mixer status shows "Idle" when paused
- [ ] Module status panel displays all required fields
- [ ] Queue table matches actual queue state

### 11.2 Functional Validation

**[SPEC016-VALIDATE-020]** Functional test cases:

1. **Empty queue** - All chains show as idle
2. **Single passage** - Chain 0 at position 0, others idle
3. **Full queue (N passages)** - All chains active, positions 0 to N-1
4. **Queue advance** - Position numbers shift correctly after skip
5. **Pause state** - All mixer statuses change to "Idle"
6. **Resume state** - Mixer statuses restore to active
7. **SSE reconnect** - UI recovers after connection loss

### 11.3 Performance Validation

**[SPEC016-VALIDATE-030]** Performance test scenarios:

1. **High update rate** - 10 Hz buffer chain updates for 60 seconds
2. **Large queue** - 100 entries in queue table
3. **Maximum chains** - N=32 chains (maximum_decode_streams limit)

**Acceptance criteria:** No memory leaks, < 15% CPU usage, < 100ms update latency

---

## 12. Traceability Matrix

### 12.1 Upstream Requirements

| Requirement | Satisfied By | Notes |
|-------------|--------------|-------|
| [DBD-OV-040] | Buffer Chain Monitor table | Full pipeline visibility |
| [DBD-OV-060] | Queue position column | Position 0 = "now playing" |
| [DBD-OV-070] | Queue position column | Position 1 = "up next" |
| [DBD-OV-080] | Chain # column | Passage-based association |
| [DBD-BUF-040] | Buffer fill column | Buffer state monitoring |
| [SSE-UI-020] | Queue Contents panel | Queue state updates |
| [SSE-UI-030] | Module Status panel | Playback position updates |

### 12.2 Downstream Artifacts

| Artifact | Type | Location |
|----------|------|----------|
| developer_ui.html | Implementation | wkmp-ap/src/api/ |
| BufferChainInfo | Data model | wkmp-common/src/events.rs |
| /api/buffer_chains | API endpoint | wkmp-ap/src/api/handlers.rs |
| BufferChainStatus SSE event | Event type | wkmp-common/src/events.rs |

---

## Appendix A: Example Table Rendering

### A.1 Buffer Chain Monitor (N=12, 3 active passages)

```
+-------+-----------+------------------+-------------+------------------+---------+-------------------+
| Chain | Queue Pos | Decoder          | Resample    | Fade             | Buffer  | Mixer             |
+-------+-----------+------------------+-------------+------------------+---------+-------------------+
| 0     | 0         | Decoding 78%     | N/A         | Body             | 65%     | Feeding [Current] |
| 1     | 1         | Decoding 12%     | 48→44.1 kHz | PreStart         | 15%     | Idle              |
| 2     | 2         | Decoding 5%      | N/A         | PreStart         | 3%      | Idle              |
| 3     | N/A       | Idle             | N/A         | -                | 0%      | Idle              |
| 4     | N/A       | Idle             | N/A         | -                | 0%      | Idle              |
| 5     | N/A       | Idle             | N/A         | -                | 0%      | Idle              |
| 6     | N/A       | Idle             | N/A         | -                | 0%      | Idle              |
| 7     | N/A       | Idle             | N/A         | -                | 0%      | Idle              |
| 8     | N/A       | Idle             | N/A         | -                | 0%      | Idle              |
| 9     | N/A       | Idle             | N/A         | -                | 0%      | Idle              |
| 10    | N/A       | Idle             | N/A         | -                | 0%      | Idle              |
| 11    | N/A       | Idle             | N/A         | -                | 0%      | Idle              |
+-------+-----------+------------------+-------------+------------------+---------+-------------------+
```

**Visual indicators:**
- Chain 0, Queue Pos "0": **Bold green text**
- Chain 1, Queue Pos "1": **Bold yellow text**
- Chain 2, Queue Pos "2": **Normal blue text**
- Chains 3-11, Queue Pos "N/A": **Gray text**

---

## Appendix B: Glossary

| Term | Definition |
|------|------------|
| **Chain** | Decoder-resampler-fade-buffer processing pipeline instance |
| **Queue Position** | Ordinal position in playback queue (0 = now playing) |
| **Passage** | Continuous playable audio region (may be subset of file) |
| **SSE** | Server-Sent Events (one-way HTTP push protocol) |
| **Working Sample Rate** | Target sample rate for mixer (default 44100 Hz) |
| **Fade Stage** | Current position in fade processing lifecycle |
| **Buffer Fill** | Percentage of buffer capacity occupied by decoded samples |
| **Mixer Role** | Function of chain in audio mixer (Current, Crossfading, Idle) |

---

**Document Version:** 1.0
**Author:** System Architect
**Status:** Active
**Approval:** Pending technical review
