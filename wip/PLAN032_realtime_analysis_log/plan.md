# PLAN032: Real-Time Analysis Log UI

## Overview

Add a scrolling log panel to the wkmp-ai import progress page showing timestamped events during folder analysis, including album match details and AcousticBrainz lookup results.

## Current State

### Existing Infrastructure
- SSE endpoint `/import/events` streams `ImportProgressUpdate` events
- Progress page displays: file count (X/Y), current file, phase checklist, worker activity
- Phase statistics show aggregated counts only

### Gaps
1. No timestamped event log visible to user
2. Album match track-by-track discrepancies not displayed
3. AcousticBrainz lookup results per-passage not shown

## Requested Features

1. **Timestamped message log** - scrolling panel with HH:MM:SS timestamps
2. **File progress (X/Y)** - already exists, verify visibility
3. **Album match details** - track-by-track timing errors for final selection
4. **AcousticBrainz results** - success/failure per **passage/song** lookup (not per file)

## Implementation Plan

### Phase 1: Backend Event Additions

#### 1.1 New SSE Event Types

**File:** `wkmp-common/src/events.rs`

Add new event variants to `ImportEvent` enum:

```rust
/// Detailed log message for UI display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisLogEntry {
    pub timestamp: DateTime<Utc>,
    pub file_index: u32,
    pub total_files: u32,
    pub file_path: String,
    pub message_type: AnalysisLogType,
    pub message: String,
    pub details: Option<AnalysisLogDetails>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnalysisLogType {
    Info,
    Success,
    Warning,
    Error,
    AlbumMatch,
    FlavorLookup,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnalysisLogDetails {
    AlbumMatch {
        album_title: String,
        artist: String,
        match_percentage: f64,
        track_count: u32,
        track_errors: Vec<TrackTimingError>,
    },
    FlavorLookup {
        passage_index: u32,        // Which passage/song within the file
        passage_total: u32,        // Total passages in file
        song_title: Option<String>, // Song title if known
        source: String,            // "AcousticBrainz", "Essentia", "PreExisting"
        success: bool,
        recording_mbid: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackTimingError {
    pub track_number: u32,
    pub track_title: String,
    pub expected_duration_secs: f64,
    pub detected_duration_secs: f64,
    pub error_secs: f64,
}
```

Add to `ImportEvent`:
```rust
pub enum ImportEvent {
    // ... existing variants ...
    AnalysisLog(AnalysisLogEntry),
}
```

#### 1.2 Emit Events from Album Matcher

**File:** `wkmp-ai/src/matching/album_matcher.rs`

After `AlbumMatchResult` is constructed (around line 900), emit log event:

```rust
// Emit analysis log event for UI
if let Some(edition) = &result.matched_edition {
    let log_entry = AnalysisLogEntry {
        timestamp: Utc::now(),
        file_index: current_file_index,
        total_files: total_files,
        file_path: file_path.to_string(),
        message_type: AnalysisLogType::AlbumMatch,
        message: format!(
            "Album matched: {} - {} ({:.1}%)",
            edition.artist, edition.title, result.match_percentage
        ),
        details: Some(AnalysisLogDetails::AlbumMatch {
            album_title: edition.title.clone(),
            artist: edition.artist.clone(),
            match_percentage: result.match_percentage,
            track_count: edition.tracks.len() as u32,
            track_errors: result.track_errors.iter().enumerate().map(|(i, &err)| {
                TrackTimingError {
                    track_number: (i + 1) as u32,
                    track_title: edition.tracks.get(i)
                        .map(|t| t.title.clone())
                        .unwrap_or_else(|| format!("Track {}", i + 1)),
                    expected_duration_secs: edition.tracks.get(i)
                        .map(|t| t.length_ms as f64 / 1000.0)
                        .unwrap_or(0.0),
                    detected_duration_secs: detected_durations.get(i).copied().unwrap_or(0.0),
                    error_secs: err,
                }
            }).collect(),
        }),
    };
    // Emit via event channel
    event_tx.send(ImportEvent::AnalysisLog(log_entry)).ok();
}
```

**Dependency:** Need to pass `event_tx: broadcast::Sender<ImportEvent>` to album matcher or emit from workflow layer.

#### 1.3 Emit Events from Flavor Lookup (Per Passage)

**File:** `wkmp-ai/src/workflow/phase_flavoring.rs` (or equivalent)

After each AcousticBrainz/Essentia lookup **per passage/song**:

```rust
// For each passage in the file
for (passage_idx, passage) in passages.iter().enumerate() {
    let log_entry = AnalysisLogEntry {
        timestamp: Utc::now(),
        file_index,
        total_files,
        file_path: file_path.to_string(),
        message_type: AnalysisLogType::FlavorLookup,
        message: if success {
            format!(
                "Flavor found for \"{}\" via {}",
                song_title.as_deref().unwrap_or("Unknown"),
                source
            )
        } else {
            format!(
                "Flavor not found for \"{}\" ({})",
                song_title.as_deref().unwrap_or("Unknown"),
                source
            )
        },
        details: Some(AnalysisLogDetails::FlavorLookup {
            passage_index: (passage_idx + 1) as u32,
            passage_total: passages.len() as u32,
            song_title: song_title.clone(),
            source: source.to_string(),
            success,
            recording_mbid,
        }),
    };
    event_tx.send(ImportEvent::AnalysisLog(log_entry)).ok();
}
```

**Note:** Each passage/song generates its own log entry, so a 10-track album file produces 10 flavor lookup messages.

### Phase 2: Frontend Log Panel

#### 2.1 HTML Structure Addition

**File:** `wkmp-ai/src/api/ui/import_progress.rs`

Add log panel section after phase statistics:

```html
<div id="analysis-log" class="panel" style="display: none;">
    <h3>Analysis Log</h3>
    <div id="log-controls">
        <label><input type="checkbox" id="log-autoscroll" checked> Auto-scroll</label>
        <label><input type="checkbox" id="log-show-info" checked> Info</label>
        <label><input type="checkbox" id="log-show-success" checked> Success</label>
        <label><input type="checkbox" id="log-show-warning" checked> Warnings</label>
        <label><input type="checkbox" id="log-show-error" checked> Errors</label>
        <button id="log-clear">Clear</button>
    </div>
    <div id="log-entries" class="log-scroll"></div>
</div>
```

CSS additions:
```css
#analysis-log {
    margin-top: 20px;
    background: #252525;
    border-radius: 8px;
    padding: 15px;
}
.log-scroll {
    max-height: 400px;
    overflow-y: auto;
    font-family: monospace;
    font-size: 12px;
    background: #1a1a1a;
    padding: 10px;
    border-radius: 4px;
}
.log-entry {
    margin: 4px 0;
    line-height: 1.4;
}
.log-entry .timestamp {
    color: #888;
    margin-right: 8px;
}
.log-entry .file-index {
    color: #4a9eff;
    margin-right: 8px;
}
.log-entry.info { color: #ccc; }
.log-entry.success { color: #4caf50; }
.log-entry.warning { color: #ff9800; }
.log-entry.error { color: #f44336; }
.log-entry.album-match { color: #8bc34a; }
.log-entry.flavor-lookup { color: #00bcd4; }

.track-details {
    margin-left: 40px;
    color: #999;
    font-size: 11px;
}
.track-error {
    color: #ff9800;
}
.track-ok {
    color: #4caf50;
}
```

#### 2.2 JavaScript Log Handling

**File:** `wkmp-ai/static/import-progress.js`

Add log management functions:

```javascript
const MAX_LOG_ENTRIES = 500;
let logEntries = [];

function handleAnalysisLog(event) {
    const entry = event;
    logEntries.push(entry);

    // Trim old entries
    if (logEntries.length > MAX_LOG_ENTRIES) {
        logEntries = logEntries.slice(-MAX_LOG_ENTRIES);
    }

    appendLogEntry(entry);
}

function appendLogEntry(entry) {
    const container = document.getElementById('log-entries');
    if (!container) return;

    // Check filter
    const typeClass = entry.message_type.toLowerCase().replace('_', '-');
    const checkbox = document.getElementById(`log-show-${typeClass}`);
    if (checkbox && !checkbox.checked) return;

    const div = document.createElement('div');
    div.className = `log-entry ${typeClass}`;

    // Format timestamp HH:MM:SS
    const ts = new Date(entry.timestamp);
    const timestamp = ts.toLocaleTimeString('en-US', { hour12: false });

    // File index X/Y
    const fileIndex = `[${entry.file_index}/${entry.total_files}]`;

    div.innerHTML = `
        <span class="timestamp">${timestamp}</span>
        <span class="file-index">${fileIndex}</span>
        <span class="message">${escapeHtml(entry.message)}</span>
    `;

    // Add track details for album matches
    if (entry.details && entry.details.AlbumMatch) {
        const match = entry.details.AlbumMatch;
        const trackDetails = document.createElement('div');
        trackDetails.className = 'track-details';

        let trackHtml = match.track_errors.map(t => {
            const errClass = Math.abs(t.error_secs) > 5 ? 'track-error' : 'track-ok';
            const sign = t.error_secs >= 0 ? '+' : '';
            return `<div class="${errClass}">
                Track ${t.track_number}: ${escapeHtml(t.track_title)}
                (expected ${formatDuration(t.expected_duration_secs)},
                detected ${formatDuration(t.detected_duration_secs)},
                error: ${sign}${t.error_secs.toFixed(1)}s)
            </div>`;
        }).join('');

        trackDetails.innerHTML = trackHtml;
        div.appendChild(trackDetails);
    }

    container.appendChild(div);

    // Auto-scroll
    if (document.getElementById('log-autoscroll')?.checked) {
        container.scrollTop = container.scrollHeight;
    }
}

function formatDuration(secs) {
    const mins = Math.floor(secs / 60);
    const s = Math.floor(secs % 60);
    return `${mins}:${s.toString().padStart(2, '0')}`;
}

function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

function clearLog() {
    logEntries = [];
    const container = document.getElementById('log-entries');
    if (container) container.innerHTML = '';
}
```

#### 2.3 SSE Event Handler Update

In `connectSSE()`, add handler for new event type:

```javascript
sse.addEventListener('AnalysisLog', (e) => {
    const event = JSON.parse(e.data);
    handleAnalysisLog(event);

    // Show log panel on first entry
    const panel = document.getElementById('analysis-log');
    if (panel && panel.style.display === 'none') {
        panel.style.display = 'block';
    }
});
```

### Phase 3: Event Channel Plumbing

#### 3.1 Pass Event Sender to Processing Pipeline

**File:** `wkmp-ai/src/workflow/orchestrator.rs`

Add `event_tx` parameter to phase execution:

```rust
pub async fn run_import_workflow(
    config: ImportConfig,
    event_tx: broadcast::Sender<ImportEvent>,
    // ... other params
) -> Result<ImportResult> {
    // Pass event_tx to phases that need to emit log entries
    run_album_matching_phase(&config, &event_tx, &files).await?;
    run_flavoring_phase(&config, &event_tx, &passages).await?;
}
```

#### 3.2 Broadcast from SSE Endpoint

**File:** `wkmp-ai/src/api/sse.rs`

Handle `AnalysisLog` events in the SSE stream:

```rust
ImportEvent::AnalysisLog(entry) => {
    let data = serde_json::to_string(&entry)?;
    yield Event::default()
        .event("AnalysisLog")
        .data(data);
}
```

## File Modifications Summary

| File | Changes |
|------|---------|
| `wkmp-common/src/events.rs` | Add `AnalysisLogEntry`, `AnalysisLogType`, `AnalysisLogDetails`, `TrackTimingError` structs |
| `wkmp-ai/src/matching/album_matcher.rs` | Emit `AnalysisLog` events after album match |
| `wkmp-ai/src/workflow/phase_flavoring.rs` | Emit `AnalysisLog` events for flavor lookups |
| `wkmp-ai/src/workflow/orchestrator.rs` | Thread `event_tx` to phases |
| `wkmp-ai/src/api/sse.rs` | Handle `AnalysisLog` in SSE stream |
| `wkmp-ai/src/api/ui/import_progress.rs` | Add log panel HTML/CSS |
| `wkmp-ai/static/import-progress.js` | Add log handling functions |

## Test Plan

1. **Unit tests:** Verify `AnalysisLogEntry` serialization
2. **Integration test:** Mock album match, verify SSE event emitted
3. **Manual test:** Run import on test folder, verify:
   - Timestamps appear correctly
   - File index X/Y shows
   - Album match details with track errors display
   - AcousticBrainz success/failure messages appear
   - Auto-scroll works
   - Filter checkboxes work
   - Clear button works

## Risk Assessment

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| SSE bandwidth with track details | Low | Low | Cap at 500 entries, throttle |
| DOM performance with many entries | Medium | Low | Use document fragment, limit visible entries |
| Event channel backpressure | Low | Medium | Use lossy broadcast, log-only events non-critical |

## Implementation Order

1. Add event types to `wkmp-common/src/events.rs`
2. Add SSE handler for new event type
3. Add HTML/CSS for log panel
4. Add JavaScript log handling
5. Wire event emission from album matcher
6. Wire event emission from flavor lookup
7. Test end-to-end
