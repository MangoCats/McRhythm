# SPEC020: Audio Player Developer UI Design

**Document Classification:** Tier 2 (Design Specification)
**Status:** Active
**Version:** 2.0
**Last Updated:** 2025-10-22

---

## Document Control

### Purpose
This document specifies the design of the Audio Player (wkmp-ap) Developer UI, a diagnostic interface for monitoring internal audio processing pipeline state in real-time.

### Scope
- Buffer chain monitor table layout and data presentation (8-column table)
- Module status panel design
- Playback controls interface
- Queue contents display and enqueue controls
- Settings management panel (database configuration editor)
- Event stream monitor (SSE event log with controls)
- File browser modal (interactive audio file selection)
- Buffer monitor rate control (dynamic update rate configuration)
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

**[SPEC016-MONITOR-030]** The buffer chain monitor table MUST have exactly 8 columns in the following order:

| Column | Header | Data Type | Description |
|--------|--------|-----------|-------------|
| 1 | Chain # | Integer (0-based) | Static chain index (0 to N-1) |
| 2 | Queue Pos | Integer or N/A | Queue position or idle state |
| 3 | Decoder | String | Decoder diagnostic information (MM:SS format) |
| 4 | Resample | String or N/A | Resampler diagnostic information |
| 5 | Fade | String | Fade stage and curve information |
| 6 | Buffer | Percentage | Buffer fill percentage with visual progress bar |
| 7 | Mixer | String | Mixer feed status and role |
| 8 | File Name | String | Audio file name or "-" for idle chains |

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

#### 2.4.8 Column 8: File Name

**[SPEC016-MONITOR-105]** File name MUST display:

**For active chains:**
- **File name only** (not full path) for brevity
- Extracted from passage's `file_path` attribute
- Word-wrapped if necessary to fit column width

**For idle chains:**
- Display: `-` (single dash)

**Example formats:**
```
01-Track_Name.mp3
symphony_no5.flac
-
```

**Rationale:** File name provides quick context for which audio file is loaded in each chain without requiring full path traversal. Essential for debugging multi-passage scenarios.

**Implementation note:** Maximum width 20% of viewport to prevent excessive horizontal scrolling.

---

## 2.5 Buffer Monitor Rate Control

**[SPEC016-MONITOR-120]** The buffer chain monitor MUST provide dynamic update rate configuration.

### 2.5.1 Purpose

Allows developers to control the frequency of BufferChainStatus SSE events based on diagnostic needs:
- **Fast mode (0.1s)**: High-frequency updates for visualizing rapid buffer filling during decode
- **Normal mode (1.0s)**: Standard monitoring with minimal overhead
- **Manual mode**: No automatic updates; developer triggers updates explicitly

### 2.5.2 UI Controls

**Update Rate Selector:**
- Dropdown with options: "0.1s", "1.0s", "Manual"
- Default: "1.0s" (normal monitoring)
- Located in Buffer Chain Monitor panel header

**Manual Update Button:**
- Triggers immediate buffer chain status update
- Enabled in all modes (useful for forcing update between automatic intervals)
- Label: "Update"

### 2.5.3 API Bindings

**[SPEC016-MONITOR-120]** Rate control MUST use these API endpoints:

| Control | HTTP Method | Endpoint | Payload |
|---------|-------------|----------|---------|
| Set Rate | POST | /playback/buffer_monitor/rate | `{"rate_ms": 100\|1000\|0}` |
| Force Update | POST | /playback/buffer_monitor/update | (none) |

**Behavior:**
- Setting rate to `0` disables automatic updates (manual mode)
- Setting rate to `100` or `1000` enables automatic SSE emission at that interval
- Force update works in all modes, including manual

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

**Implemented controls:**
1. **Play** - Start/resume playback (green button with ▶ icon)
2. **Pause** - Pause playback, maintains position (yellow button with ⏸ icon)
3. **Skip Current** - Advance to next passage (orange button with ⏭ icon)
4. **Clear Queue** - Remove all passages from queue (red button with 🗑 icon, requires confirmation)
5. **Volume Input** - Numeric input (0-100) with "Set Volume" button
6. **Audio Device Input** - Text input with "Set Device" button

**Removed controls:**
- **Stop** - Not implemented in current design (Pause serves this purpose)

### 4.3 API Bindings

**[SPEC016-CONTROLS-020]** Controls MUST map to wkmp-ap REST API endpoints:

| Control | HTTP Method | Endpoint |
|---------|-------------|----------|
| Play | POST | /playback/play |
| Pause | POST | /playback/pause |
| Skip Current | POST | /playback/next |
| Clear Queue | POST | /playback/queue/clear |
| Set Volume | POST | /audio/volume |
| Set Audio Device | POST | /audio/device |

**Error Handling:** Display success/error messages inline below each control for 3 seconds.

**Confirmation Dialogs:**
- **Clear Queue:** Requires confirmation dialog ("Clear entire queue? This will remove all passages.")

---

## 5. Queue Contents & Enqueue Controls

### 5.1 Purpose
**[SPEC016-QUEUE-010]** Display current playback queue for verification and debugging, and provide controls for adding passages.

### 5.2 Layout
**[SPEC016-QUEUE-020]** Queue MUST display entries as card-based items (not table layout).

**Rationale:** Card layout provides better readability for variable-length file paths without table column width constraints.

### 5.3 Queue Entry Display

**Each queue entry card MUST show:**
1. **File Path** - Full file path (top line)
2. **Queue Entry ID** - UUID in smaller gray text (bottom line, prefixed with "ID: ")

**Visual treatment:**
- Each entry: Dark background card with rounded corners
- Padding: 6px vertical, 6px horizontal
- Margin: 4px between entries
- Font: 12px for file path, 10px for ID

**Empty queue:**
- Display: "Queue is empty" in gray italic text

**Example:**
```
┌────────────────────────────────────────┐
│ /home/music/track1.mp3                 │
│ ID: 550e8400-e29b-41d4-a716-446655440000│
└────────────────────────────────────────┘
┌────────────────────────────────────────┐
│ /home/music/track2.flac                │
│ ID: 6ba7b810-9dad-11d1-80b4-00c04fd430c8│
└────────────────────────────────────────┘
```

**Implementation note:** Queue position is implicit (top-to-bottom order). Position 0 = first card, position 1 = second card, etc.

### 5.4 Enqueue Controls

**[SPEC016-QUEUE-050]** Queue panel MUST include controls for adding new passages:

**File Path Input:**
- Text input field for audio file path
- Placeholder: "/path/to/audio/file.mp3"
- Full width minus Browse button

**Browse Button:**
- Label: "Browse"
- Action: Opens File Browser Modal (see Section 9)
- Gray background to distinguish from primary action
- Positioned inline to right of file path input

**Enqueue Button:**
- Label: "Enqueue"
- Action: POST /playback/enqueue with file path from input field
- Primary blue background
- Full width
- Displays success/error message inline for 3 seconds

**Controls separator:**
- Visual separator (border-top) between queue display and enqueue controls
- Padding: 8px top margin

### 5.5 Interactive Features

**[SPEC016-QUEUE-040]** Queue clearing is handled by "Clear Queue" button in Playback Controls (Section 4).

**Individual entry removal:** Not implemented in current version (future enhancement)

---

## 6. Settings Management Panel

### 6.1 Purpose

**[SPEC016-SETTINGS-010]** Provide real-time database configuration editing for all wkmp-ap parameters defined in SPEC016 and other settings.

**Design Rationale:** Centralizes all runtime configuration in one interface, enabling rapid parameter tuning for decoder/buffer architecture without manual database editing or code recompilation.

### 6.2 Layout

**[SPEC016-SETTINGS-020]** Settings panel MUST use a scrollable table layout with comprehensive parameter metadata.

**Table structure:**
- Fixed header with sticky positioning
- Scrollable body (max-height: 300px)
- 7 columns: Parameter, Type, Default, Current Value, Valid Range, New Value, Description

### 6.3 Table Columns

**[SPEC016-SETTINGS-030]** Settings table columns specification:

| Column | Header | Width | Content |
|--------|--------|-------|---------|
| 1 | Parameter | 20% | Setting key name (cyan monospace font, 11px) |
| 2 | Type | 8% | Data type (yellow text, 11px: f32, u32, String, etc.) |
| 3 | Default | 12% | Default value from code (gray monospace font, **13px, right-aligned**) |
| 4 | Current Value | 12% | Database value or "(not set)" (green monospace font, **13px**, **highlighted orange if differs from default**) |
| 5 | Valid Range | 15% | Valid range or enum values (blue monospace font, **13px**) |
| 6 | New Value | 12% | Text input for user edits (empty by default) |
| 7 | Description | 21% | Setting description only (gray text, 11px) |

**[SPEC016-SETTINGS-031]** Non-default value highlighting:
- When `current_value ≠ default_value`, the Current Value cell MUST have orange background (#854d0e) with yellow text (#fef08a)
- This provides immediate visual feedback about which settings have been customized from defaults
- Helps users identify configuration changes when troubleshooting

**[SPEC016-SETTINGS-032]** Typography and alignment:
- Default, Current Value, and Valid Range columns use **13px font** (larger than 11px baseline) for improved readability of critical configuration values
- Default column is **right-aligned** to facilitate numerical comparison with Current Value column
- Monospace font for all value columns ensures proper alignment of numerical data

### 6.4 Supported Settings

**[SPEC016-SETTINGS-040]** Settings panel MUST display all wkmp-ap database settings (26+ parameters):

**SPEC016 Decoder/Buffer Parameters:**
- `working_sample_rate` [DBD-PARAM-020]
- `output_ringbuffer_size` [DBD-PARAM-030]
- `output_refill_period` [DBD-PARAM-040]
- `maximum_decode_streams` [DBD-PARAM-050]
- `decode_work_period` [DBD-PARAM-060]
- `decode_chunk_size` [DBD-PARAM-065]
- `playout_ringbuffer_size` [DBD-PARAM-070]
- `playout_ringbuffer_headroom` [DBD-PARAM-080]
- `decoder_resume_hysteresis_samples` [DBD-PARAM-085]
- `mixer_min_start_level` [DBD-PARAM-088]
- `pause_decay_factor` [DBD-PARAM-090]
- `pause_decay_floor` [DBD-PARAM-100]
- `audio_buffer_size` [DBD-PARAM-110]
- `mixer_check_interval_ms` [DBD-PARAM-111]

**Audio Output:**
- `volume_level` [DBD-PARAM-010]
- `audio_sink`

**Crossfade:**
- `global_crossfade_time`
- `global_fade_curve`

**Event Intervals:**
- `position_event_interval_ms`
- `playback_progress_interval_ms`

**Mixer/Buffer (legacy):**
- `minimum_buffer_threshold_ms`
- `audio_ring_buffer_grace_period_ms`
- `mixer_check_interval_us`
- `mixer_batch_size_low`
- `mixer_batch_size_optimal`

**Resume from Pause:**
- `resume_from_pause_fade_in_duration`
- `resume_from_pause_fade_in_curve`

### 6.5 Editing Workflow

**[SPEC016-SETTINGS-050]** Settings editing workflow:

1. **Load Settings:** Click "🔄 Refresh" button to fetch current values from database
2. **Edit Values:** Type new values in "New Value" column (only modified rows are updated)
3. **Bulk Update:** Click "💾 Save All & Restart" button
4. **Confirmation:** Dialog warns: "You are about to update N settings. The application will shut down in 2 seconds. You will need to manually restart it. Continue?"
5. **Update & Shutdown:** Settings written to database, application exits after 2 seconds
6. **Manual Restart:** User must restart wkmp-ap for changes to take effect

**Rationale for shutdown:** Most settings are loaded at startup as constants (e.g., buffer sizes, sample rates). Changing these at runtime would cause undefined behavior. Explicit restart ensures clean initialization with new values.

### 6.6 Validation

**[SPEC016-SETTINGS-060]** Client-side validation displays valid ranges/enums in Description column.

**Server-side validation:**
- Type conversion errors returned as HTTP 500
- Range validation performed by settings loader functions (clamped to valid range)

**No pre-submit validation:** User can enter any value; validation occurs server-side on save.

### 6.7 API Bindings

**[SPEC016-SETTINGS-070]** Settings management API endpoints:

| Action | HTTP Method | Endpoint | Response |
|--------|-------------|----------|----------|
| Load All Settings | GET | /settings/all | `{"settings": [{"key": "...", "value": "...", "data_type": "...", "description": "...", "validation": "..."}]}` |
| Bulk Update | POST | /settings/bulk_update | `{"updated_count": N, "message": "Updated N settings. Application will shut down in 2 seconds..."}` |

**Bulk update payload:**
```json
{
  "settings": {
    "mixer_min_start_level": "88200",
    "maximum_decode_streams": "16"
  }
}
```

### 6.8 Visual Design

**Panel header:**
- Title: "Database Settings"
- Actions: "💾 Save All & Restart" (orange warning button) + "🔄 Refresh" (blue button)
- Status bar: Success/error messages (hidden by default, auto-hide after completion)

**Table styling:**
- Sticky header with blue bottom border
- Alternating row backgrounds for readability
- Monospace fonts for parameter names and values
- Small fonts (11-12px) for information density

---

## 7. Event Stream Monitor

### 7.1 Purpose

**[SPEC016-EVENTS-010]** Provide real-time visibility into SSE event stream for debugging SSE integration and monitoring event flow.

**Use cases:**
- Verify correct event types are emitted
- Monitor event timing and frequency
- Debug event payload structure
- Export event history for offline analysis

### 7.2 Layout

**[SPEC016-EVENTS-020]** Event monitor MUST use a reverse-chronological scrollable log with controls.

**Components:**
1. **Control bar** - Buttons for Clear Log, Export JSON, Pause/Resume
2. **Event log** - Scrollable container (flex: 1) with monospace font
3. **Event items** - One line per event with timestamp, type, and data

### 7.3 Event Display Format

**[SPEC016-EVENTS-030]** Each event MUST display:

```
[HH:MM:SS] EventType: Event data summary
```

**Components:**
- **Timestamp:** [HH:MM:SS] format in gray
- **Event Type:** Color-coded by type (see 7.4), bold weight
- **Event Data:** Abbreviated payload (full data in eventHistory for export)

**Examples:**
```
[14:32:10] PlaybackStateChanged: State: playing
[14:32:09] QueueStateUpdate: Queue updated: 3 entries
[14:32:05] VolumeChanged: Volume: 75%
[14:32:01] InitialState: Connected to SSE event stream
[14:32:00] system: Connecting to event stream...
```

### 7.4 Event Type Color Coding

**[SPEC016-EVENTS-040]** Event types MUST use consistent color scheme:

| Event Type | Color | Purpose |
|------------|-------|---------|
| InitialState | Purple (#8b5cf6) | SSE connection established |
| QueueStateUpdate | Blue (#3b82f6) | Queue modifications |
| PlaybackPosition | Green (#10b981) | Position updates (not logged to UI, too frequent) |
| VolumeChanged | Orange (#f59e0b) | Volume adjustments |
| PlaybackStateChanged | Red (#ef4444) | State transitions |
| CrossfadeStarted | Pink (#ec4899) | Crossfade start |
| PassageStarted | Cyan (#06b6d4) | Passage playback start |
| PassageCompleted | Lime (#84cc16) | Passage completion |
| BufferChainStatus | (not logged) | Buffer updates (handled by Buffer Chain Monitor) |
| system | Gray (#6b7280) | Internal UI events (connection, errors) |

**Rationale:** Color coding enables rapid visual scanning for specific event types during debugging.

### 7.5 Event Controls

**[SPEC016-EVENTS-050]** Event monitor controls:

**Clear Log Button:**
- Clears visible event log and resets eventHistory array
- Adds system event: "Event log cleared"

**Export JSON Button:**
- Downloads eventHistory as JSON file
- Filename: `wkmp-events-{ISO_TIMESTAMP}.json`
- Includes full event payloads (not abbreviated)
- Adds system event: "Event log exported"

**Pause/Resume Button:**
- Toggles eventsPaused flag
- Label changes: "Pause" ↔ "Resume"
- When paused, events are still received from SSE but not displayed or stored
- Adds system event: "Event logging paused" / "Event logging resumed"

### 7.6 Event History

**[SPEC016-EVENTS-060]** Event storage:

- **Max visible events:** 100 (DOM pruning after limit)
- **Max stored events:** 200 (for export)
- **Order:** Reverse chronological (newest first)
- **Storage:** In-memory JavaScript array (lost on page refresh)

**Rationale:** Limited history prevents memory leaks during long debugging sessions while retaining sufficient context for analysis.

### 7.7 Special Event Handling

**PlaybackPosition events:**
- **Not logged to UI** (would spam log with 1Hz updates)
- Directly update Module Status Panel position display
- Not added to eventHistory

**BufferChainStatus events:**
- **Not logged to UI** (handled by dedicated Buffer Chain Monitor)
- Directly update Buffer Chain Monitor table
- Not added to eventHistory

---

## 8. File Browser Modal

### 8.1 Purpose

**[SPEC016-FILES-010]** Provide interactive filesystem navigation for selecting audio files to enqueue.

**Design Rationale:** Eliminates manual path typing errors and improves discoverability of audio files within configured root folder.

### 8.2 Modal Layout

**[SPEC016-FILES-020]** File browser MUST use a modal overlay with centered content box.

**Components:**
1. **Modal Overlay:** Full-screen semi-transparent black background
2. **Modal Content:** Centered box (90% width, max 700px, max-height 80vh)
3. **Modal Header:** Title + close button
4. **Modal Body:** Current path display + file list

### 8.3 File Browser Components

**Modal Header:**
- Title: "Browse Audio Files" (blue, 16px)
- Close button: "×" (24px, top-right, gray, hover white)

**Current Path Display:**
- Full canonical path in monospace font
- Dark background, 12px text
- Word-wrap for long paths
- Updated on directory navigation

**File List:**
- Scrollable container (max-height: 400px)
- Each entry: icon + name
- Directories listed first, then audio files, alphabetically

### 8.4 File Entry Display

**[SPEC016-FILES-030]** File entries MUST show:

**Directory Entry:**
- Icon: 📁
- Name: Directory name
- Hover: Background change to #333
- Click: Navigate into directory

**Parent Directory Entry:**
- Icon: 📁
- Name: ".." (double dot)
- Only shown if not at root folder
- Click: Navigate to parent directory

**Audio File Entry:**
- Icon: 🎵
- Name: File name (green color for audio files)
- Hover: Background change to #333
- Click: Select file, populate enqueue input, close modal

**Non-audio files:** Hidden (not displayed in list)

### 8.5 Supported Audio Formats

**[SPEC016-FILES-040]** File browser MUST recognize these extensions as audio files:

- `.mp3` - MPEG Audio Layer 3
- `.flac` - Free Lossless Audio Codec
- `.ogg` - Ogg Vorbis
- `.wav` - Waveform Audio File
- `.m4a` - MPEG-4 Audio
- `.aac` - Advanced Audio Coding
- `.opus` - Opus Interactive Audio Codec
- `.wma` - Windows Media Audio

**Hidden files:** Files/directories starting with `.` are excluded from display.

### 8.6 Navigation Behavior

**[SPEC016-FILES-050]** File browser navigation:

**Initial open:**
- Fetches configured `root_folder` from database settings
- If root folder doesn't exist, uses OS-specific default (see wkmp-common/config)
- Creates default folder if it doesn't exist

**Directory click:**
- Sends GET /files/browse?path={selected_directory}
- Updates current path display
- Refreshes file list

**Parent navigation:**
- Navigates to parent directory (path.parent())
- Disabled if already at root folder

**File selection:**
- Populates "File Path" input in Queue panel with selected path
- Closes modal
- Sets focus to Enqueue button

### 8.7 Security

**[SPEC016-FILES-060]** File browser MUST enforce path security:

**Server-side security (handlers.rs:browse_files):**
- Canonicalize both root folder and requested path
- Verify requested path starts with root folder (path traversal protection)
- Return HTTP 403 Forbidden if path outside root folder
- Example: Requesting `../../etc/passwd` is blocked

**Client-side security:**
- Paths are URL-encoded in query parameters
- No direct file access (all reads through server API)

**Rationale:** Prevents directory traversal attacks while allowing legitimate navigation within music library.

### 8.8 API Bindings

**[SPEC016-FILES-070]** File browser API:

| Action | HTTP Method | Endpoint | Query Params |
|--------|-------------|----------|--------------|
| Browse Directory | GET | /files/browse | `?path=/optional/path` (omit for root folder) |

**Response format:**
```json
{
  "current_path": "/home/music",
  "parent_path": "/home",
  "entries": [
    {
      "name": "subdir",
      "path": "/home/music/subdir",
      "is_directory": true,
      "is_audio_file": false
    },
    {
      "name": "song.mp3",
      "path": "/home/music/song.mp3",
      "is_directory": false,
      "is_audio_file": true
    }
  ]
}
```

**Error responses:**
- 403 Forbidden: Path outside root folder
- 400 Bad Request: Cannot read directory

### 8.9 Modal Lifecycle

**Open:**
- Triggered by "Browse" button in Queue panel (Section 5.4)
- Adds `open` class to modal (display: flex)
- Fetches root folder contents

**Close:**
- Triggered by close button (×) or file selection
- Removes `open` class from modal (display: none)
- Clears selectedFilePath variable

---

## 9. SSE Integration

### 9.1 Event Types

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

### 9.2 Connection Management

**[SPEC016-SSE-020]** SSE connection handling:

1. **Auto-connect on page load**
2. **Display connection status** in Module Status Panel
3. **Auto-reconnect** on disconnect (exponential backoff: 1s, 2s, 4s, 8s, max 30s)
4. **Error display** if connection fails after 5 attempts

### 9.3 Event Processing

**[SPEC016-SSE-030]** Event processing rules:

1. **Parse JSON** from SSE data field
2. **Validate event type** matches expected schema
3. **Update UI atomically** (batch DOM updates if needed)
4. **Log errors** to browser console (do not crash UI)

---

## 10. Responsive Design

### 10.1 Minimum Viewport

**[SPEC016-LAYOUT-010]** Developer UI MUST support minimum viewport width of **1280px**.

**Rationale:** Developer interface prioritizes information density over mobile responsiveness.

### 10.2 Scrolling Behavior

**[SPEC016-LAYOUT-020]** Scrolling:
- **Buffer Chain Monitor:** Vertical scroll if N > 20 chains
- **Queue Contents:** Vertical scroll if queue > 10 entries
- **Settings Management Panel:** Vertical scroll for 26+ parameters
- **Event Stream Monitor:** Vertical scroll for event log
- **File Browser Modal:** Vertical scroll for file list

**Note:** Buffer Chain table has 8 columns (not 7), requiring slightly more horizontal space.

---

## 11. Performance Requirements

### 11.1 Update Latency

**[SPEC016-PERF-010]** UI update latency targets:

| Metric | Target | Measurement |
|--------|--------|-------------|
| SSE event to DOM update | < 50ms | Client-side JS timing |
| Buffer chain table refresh | < 100ms | For N=12 chains |
| Queue table refresh | < 100ms | For 50 entries |

### 11.2 Memory Footprint

**[SPEC016-PERF-020]** Client-side memory:
- Maximum: 50 MB for page + SSE buffers + event history
- EventSource buffer: Auto-managed by browser
- Event history: ~200 events × ~1KB = 200KB

### 11.3 CPU Usage

**[SPEC016-PERF-030]** Client-side CPU:
- < 5% average during idle (SSE connected, no updates)
- < 15% average during active playback (1Hz updates)

---

## 12. Implementation Notes

### 12.1 Technology Constraints

**[SPEC016-IMPL-010]** Implementation constraints:

1. **Single HTML file** - All CSS/JS inline (no external dependencies)
2. **Vanilla JavaScript** - No frameworks (React, Vue, etc.)
3. **No build step** - HTML file served directly by wkmp-ap
4. **No persistent state** - Refresh clears all client-side data

**Rationale:** Minimize deployment complexity; developer UI is diagnostic tool, not production interface.

### 12.2 Browser Compatibility

**[SPEC016-IMPL-020]** Minimum browser support:

- Chrome/Edge 90+ (Chromium-based)
- Firefox 88+
- Safari 14+

**Required features:**
- EventSource API (SSE)
- Fetch API
- ES6 JavaScript (arrow functions, template literals, async/await)

### 12.3 Accessibility

**[SPEC016-IMPL-030]** Accessibility is **not required** for developer UI.

**Rationale:** Developer-only diagnostic interface; accessibility adds complexity without user benefit.

---

## 13. Future Enhancements

### 13.1 Deferred Features

**[SPEC016-FUTURE-010]** Features deferred to future versions:

1. **Decoder status tracking** - Full visibility into decoder worker state
2. **Resample diagnostics** - Real-time resample ratio and algorithm info
3. **Fade curve visualization** - Graphical representation of fade curves
4. **Buffer waveform preview** - Mini waveform display per chain
5. **Performance graphs** - Historical CPU/memory/latency charts
6. **Individual queue entry removal** - Delete button per entry
7. **Seek controls** - Position slider for current passage

### 13.2 Export Capabilities

**[SPEC016-FUTURE-020]** Potential export features:

1. **CSV export** - Buffer chain state snapshots
2. **JSON dump** - Full pipeline state for offline analysis
3. **Screenshot capture** - Save current UI state as image

**Note:** Event log JSON export is already implemented (Section 7.5).

---

## 14. Validation Criteria

### 14.1 Visual Validation

**[SPEC016-VALIDATE-010]** Manual validation checklist:

- [ ] All N chains visible in table (0 to N-1)
- [ ] Queue position colors correct (green=0, yellow=1, blue=2+)
- [ ] Idle chains show "N/A" in appropriate columns
- [ ] Buffer fill percentages update in real-time
- [ ] Mixer status shows "Idle" when paused
- [ ] Module status panel displays all required fields
- [ ] Queue table matches actual queue state
- [ ] Settings management panel loads and displays all parameters
- [ ] Event stream monitor logs events with correct colors
- [ ] File browser modal opens and displays directories/files

### 14.2 Functional Validation

**[SPEC016-VALIDATE-020]** Functional test cases:

1. **Empty queue** - All chains show as idle
2. **Single passage** - Chain 0 at position 0, others idle
3. **Full queue (N passages)** - All chains active, positions 0 to N-1
4. **Queue advance** - Position numbers shift correctly after skip
5. **Pause state** - All mixer statuses change to "Idle"
6. **Resume state** - Mixer statuses restore to active
7. **SSE reconnect** - UI recovers after connection loss
8. **Settings bulk update** - Changes saved and application restarts
9. **File browser** - Select file populates enqueue input
10. **Event export** - JSON download contains full event history

### 14.3 Performance Validation

**[SPEC016-VALIDATE-030]** Performance test scenarios:

1. **High update rate** - 10 Hz buffer chain updates for 60 seconds
2. **Large queue** - 100 entries in queue table
3. **Maximum chains** - N=32 chains (maximum_decode_streams limit)
4. **Event log stress** - 1000+ events without memory leak

**Acceptance criteria:** No memory leaks, < 15% CPU usage, < 100ms update latency

---

## 15. Traceability Matrix

### 15.1 Upstream Requirements

| Requirement | Satisfied By | Notes |
|-------------|--------------|-------|
| [DBD-OV-040] | Buffer Chain Monitor table | Full pipeline visibility |
| [DBD-OV-060] | Queue position column | Position 0 = "now playing" |
| [DBD-OV-070] | Queue position column | Position 1 = "up next" |
| [DBD-OV-080] | Chain # column | Passage-based association |
| [DBD-BUF-040] | Buffer fill column | Buffer state monitoring |
| [SSE-UI-020] | Queue Contents panel | Queue state updates |
| [SSE-UI-030] | Module Status panel | Playback position updates |
| [DBD-PARAM-088] | Settings Management panel | Mixer minimum start level parameter |
| [ARCH-FB-010] | File Browser Modal | Filesystem browsing for audio files |

### 15.2 Downstream Artifacts

| Artifact | Type | Location |
|----------|------|----------|
| developer_ui.html | Implementation | wkmp-ap/src/api/ |
| BufferChainInfo | Data model | wkmp-common/src/events.rs |
| /playback/buffer_chains | API endpoint | wkmp-ap/src/api/handlers.rs |
| /settings/all | API endpoint | wkmp-ap/src/api/handlers.rs |
| /settings/bulk_update | API endpoint | wkmp-ap/src/api/handlers.rs |
| /files/browse | API endpoint | wkmp-ap/src/api/handlers.rs |
| /playback/buffer_monitor/rate | API endpoint | wkmp-ap/src/api/handlers.rs |
| /playback/buffer_monitor/update | API endpoint | wkmp-ap/src/api/handlers.rs |
| BufferChainStatus SSE event | Event type | wkmp-common/src/events.rs |

---

## Appendix A: Example Table Rendering

### A.1 Buffer Chain Monitor (N=12, 3 active passages)

**Note:** Example shows 8-column table (added File Name column in current implementation)

```
+-------+-----------+------------+-------------+------------------+---------+-------------------+------------------+
| Chain | Queue Pos | Decoder    | Resample    | Fade             | Buffer  | Mixer             | File Name        |
+-------+-----------+------------+-------------+------------------+---------+-------------------+------------------+
| 0     | 0         | 2:15       | N/A 44100Hz | Body             | 65%     | Current ✓         | track01.mp3      |
| 1     | 1         | 0:18       | From 48000Hz| PreStart         | 15%     | Idle              | track02.flac     |
| 2     | 2         | 0:05       | N/A 44100Hz | PreStart         | 3%      | Idle              | track03.mp3      |
| 3     | N/A       | -          | N/A         | -                | 0%      | Idle              | -                |
| 4     | N/A       | -          | N/A         | -                | 0%      | Idle              | -                |
| 5     | N/A       | -          | N/A         | -                | 0%      | Idle              | -                |
| 6     | N/A       | -          | N/A         | -                | 0%      | Idle              | -                |
| 7     | N/A       | -          | N/A         | -                | 0%      | Idle              | -                |
| 8     | N/A       | -          | N/A         | -                | 0%      | Idle              | -                |
| 9     | N/A       | -          | N/A         | -                | 0%      | Idle              | -                |
| 10    | N/A       | -          | N/A         | -                | 0%      | Idle              | -                |
| 11    | N/A       | -          | N/A         | -                | 0%      | Idle              | -                |
+-------+-----------+------------+-------------+------------------+---------+-------------------+------------------+
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
