# wkmp-ai UI Architecture Clarification

**Date:** 2025-10-28
**Issue:** Documentation ambiguity about whether wkmp-ai should have its own UI or rely on wkmp-ui
**Context:** User clarified that wkmp-ai's complex one-time import operations were intended to have dedicated UI

---

## Current Documentation Analysis

### What the Specs Say

**SPEC032 (Audio Ingest Architecture) - Lines 37-43:**
```
| Module | Port | wkmp-ai Integration | Communication |
| wkmp-ui | 5720 | Import wizard UI, progress display | HTTP REST + SSE |

**Primary Integration:** wkmp-ui orchestrates import workflow via wkmp-ai API
```
**Interpretation:** Suggests wkmp-ui provides the UI

**SPEC032 Line 332:**
```
**[AIA-INT-020]** wkmp-ai implements segmentation workflow from IMPL005:
- User review and manual adjustment UI
```
**Interpretation:** Mentions UI but doesn't clarify WHERE it lives

**IMPL005 (Audio File Segmentation) - Lines 57-68:**
```
**[AFS-REV-010]** The user is presented with a review screen that shows:
- The audio waveform for the entire file
- The proposed Passage boundaries overlaid on the waveform
- The matched Recording/Song information

**[AFS-REV-020]** From this screen, the user has full manual control to:
- Adjust Boundaries: Drag the start and end points
- Add Passages: Create new passage boundaries
- Delete Passages: Remove incorrectly identified segments
- Re-assign Songs: Choose correct Recording
```
**Interpretation:** Describes complex interactive UI workflow, but doesn't specify which microservice provides it

**SPEC009 (UI Specification) - Lines 517-535:**
```
### Passage Editor

**[UI-EDIT-010]** Passage boundary editing interface:
- Waveform display of audio file
- Draggable markers for passage boundaries
- Play preview from marker position
- Save/Cancel buttons
```
**Interpretation:** Describes passage editing UI in wkmp-ui section

**CLAUDE.md - Line 246-247:**
```
| Audio Ingest (wkmp-ai) | 5723 | File import, MusicBrainz/AcousticBrainz integration | Full (on-demand) |
| Lyric Editor (wkmp-le) | 5724 | Split-window lyric editing interface | Full (on-demand) |
```
**Interpretation:** wkmp-le explicitly mentioned as "interface", wkmp-ai described as "integration" (less clear)

---

## Ambiguities Identified

### 1. UI Ownership is Unclear

**Problem:** Multiple interpretations possible:
- **Interpretation A:** wkmp-ui provides ALL UI (import wizard + passage editor), calls wkmp-ai backend
- **Interpretation B:** wkmp-ai provides its own dedicated UI for complex workflows, wkmp-ui orchestrates
- **Interpretation C:** Hybrid - simple import via wkmp-ui, complex segmentation via dedicated wkmp-ai UI

**User's Intent (Clarified):** wkmp-ai should have its own UI for complex one-time operations

### 2. "on-demand" Not Defined

**CLAUDE.md says:** wkmp-ai is "Full (on-demand)"

**Possible meanings:**
- Launched only when user clicks "Import" in wkmp-ui?
- Separate application user launches manually?
- Embedded in wkmp-ui as iframe/embedded browser?

**Not specified:** How user accesses wkmp-ai UI

### 3. Microservices Communication Pattern Unclear

**wkmp-le (Lyric Editor):**
- Described as "Split-window lyric editing **interface**"
- Implies standalone UI
- "on-demand" likely means user launches it separately

**wkmp-ai (Audio Ingest):**
- Described as "File import, MusicBrainz/AcousticBrainz **integration**"
- Sounds like backend service
- But has complex UI requirements per IMPL005

**Pattern not clear:** Should wkmp-ai follow same pattern as wkmp-le?

---

## Recommended Architecture (Based on User Clarification)

### wkmp-ai Should Have Dedicated UI

**Rationale:**
1. **Complex workflows** - IMPL005 describes multi-step interactive process (waveform editing, drag boundaries, MusicBrainz release selection)
2. **One-time use** - Import happens rarely, doesn't need tight integration with main playback UI
3. **Specialized tools** - Audio waveform display, silence threshold tuning, fingerprint visualization
4. **Similar to wkmp-le** - Both are "on-demand" tools for specialized tasks

**Implementation Pattern:**
- wkmp-ai runs on port 5723
- Serves HTML/CSS/JS UI (like wkmp-le presumably does)
- User accesses via http://localhost:5723 in browser
- wkmp-ui provides "Import Music" button that opens wkmp-ai in new tab/window
- After import complete, user returns to wkmp-ui

**Benefits:**
- ✅ Decoupled UI development (wkmp-ai team owns entire import UX)
- ✅ Specialized UI toolkit (can use charting libs for waveforms without bloating wkmp-ui)
- ✅ Clear separation of concerns
- ✅ Consistent with "on-demand" pattern

---

## Documentation Gaps and Recommended Fixes

### Gap 1: SPEC024 Contradictory Statements

**Current (Ambiguous):**
```
**Primary Integration:** wkmp-ui orchestrates import workflow via wkmp-ai API
```
```
- User review and manual adjustment UI
```

**Recommended Fix for SPEC024:**

Add new section after line 43:

```markdown
### UI Architecture

**[AIA-UI-010]** wkmp-ai provides its own web-based UI for import workflows:
- **Access:** User navigates to http://localhost:5723 in browser
- **Launch:** wkmp-ui provides "Import Music" button that opens wkmp-ai in new tab/window
- **Technology:** HTML/CSS/JavaScript served by wkmp-ai Axum server
- **Pattern:** Similar to wkmp-le (on-demand specialized tool with dedicated UI)

**[AIA-UI-020]** wkmp-ui integration:
- wkmp-ui checks if wkmp-ai is running (via /health endpoint)
- If running, "Import Music" button enabled (opens http://localhost:5723)
- If not running, button shows "Install Full Version to enable import"
- No embedded import UI in wkmp-ui (wkmp-ai owns all import UX)

**[AIA-UI-030]** After import completion:
- wkmp-ai displays "Import Complete" with link back to wkmp-ui
- User returns to wkmp-ui (http://localhost:5720) to use library
- wkmp-ui detects new files via database watch or SSE event
```

---

### Gap 2: IMPL005 Missing UI Location

**Current (Ambiguous):**
```
**[AFS-REV-010]** The user is presented with a review screen that shows:
```

**Recommended Fix for IMPL005:**

Add clarification at beginning of document:

```markdown
## UI Location

**[AFS-UI-010]** The segmentation workflow UI is provided by **wkmp-ai** (not wkmp-ui):
- User accesses via http://localhost:5723 in web browser
- wkmp-ai serves HTML/CSS/JavaScript for all UI screens
- wkmp-ui does NOT embed or proxy this UI
- Pattern: Standalone web application for specialized import tasks

**See:** [Audio Ingest Architecture - UI Architecture](SPEC032-audio_ingest_architecture.md#ui-architecture)
```

---

### Gap 3: SPEC009 Passage Editor Ownership

**Current (Ambiguous):**
SPEC009 describes "Passage Editor" as if it's part of wkmp-ui, but IMPL005 describes it as part of import workflow (wkmp-ai).

**Recommended Fix for SPEC009:**

Clarify lines 517-535:

```markdown
### Passage Editor

**[UI-EDIT-005]** Passage editing available in TWO contexts:

**Context A: Initial Import (wkmp-ai only):**
- During audio file segmentation workflow (IMPL005)
- Accessed via wkmp-ai UI (http://localhost:5723)
- Full waveform display, silence threshold tuning, MusicBrainz matching
- Available: Full version only

**Context B: Post-Import Editing (wkmp-ui):**
- After files imported, user can edit existing passages
- Accessed via wkmp-ui library view
- Simplified editor: adjust boundaries, edit metadata
- Available: Full, Lite versions (edit existing passages only)

**[UI-EDIT-010]** Passage boundary editing interface (both contexts):
- Waveform display of audio file
- Draggable markers for passage boundaries
- Play preview from marker position
- Save/Cancel buttons

**Implementation Note:** wkmp-ai and wkmp-ui may share waveform rendering code via wkmp-common, but UIs are separate.
```

---

### Gap 4: CLAUDE.md Module Descriptions

**Current (Ambiguous):**
```
| Audio Ingest (wkmp-ai) | 5723 | File import, MusicBrainz/AcousticBrainz integration | Full (on-demand) |
```

**Recommended Fix for CLAUDE.md:**

Update line 246:

```markdown
| Audio Ingest (wkmp-ai) | 5723 | Import wizard UI, file scanning, MusicBrainz identification | Full (on-demand) |
```

Add clarification after table:

```markdown
**On-Demand Modules:**
- **wkmp-ai** and **wkmp-le** are "on-demand" specialized tools
- Each provides its own web UI served on dedicated port
- User accesses via browser (http://localhost:5723 for wkmp-ai, http://localhost:5724 for wkmp-le)
- wkmp-ui provides links/buttons to launch these tools in new browser tabs
- Pattern: Decoupled specialized UIs for complex one-time operations
```

---

### Gap 5: REQ001 WebUI Reference

**Current (Ambiguous):**
Line 308: "WebUI provides interface to input/edit lyrics"

**Implies:** wkmp-ui owns all UI

**Recommended Fix for REQ001:**

Add clarification section:

```markdown
### Microservice UI Ownership

**[REQ-UI-010]** UI is distributed across microservices:

| Microservice | UI Responsibility | Access Method |
|--------------|------------------|---------------|
| **wkmp-ui** | Main playback interface, library browser, preferences | http://localhost:5720 (primary access point) |
| **wkmp-ai** | Import wizard, file segmentation, passage boundary editor (initial import) | http://localhost:5723 (launched from wkmp-ui) |
| **wkmp-le** | Lyric editor (split-window interface) | http://localhost:5724 (launched from wkmp-ui) |
| **wkmp-ap** | No UI (backend audio engine) | - |
| **wkmp-pd** | No UI (backend selection algorithm) | - |

**[REQ-UI-020]** "WebUI" in requirements refers to browser-based UI, NOT exclusively wkmp-ui module:
- wkmp-ui is PRIMARY UI (user starts here)
- wkmp-ai provides SPECIALIZED UI for import workflows
- wkmp-le provides SPECIALIZED UI for lyric editing
- All are web-based (HTML/CSS/JS in browser)
```

---

## Implementation Implications

### Current MVP Status (as of 2025-10-28)

**wkmp-ai currently:**
- ✅ Backend API implemented (HTTP REST + SSE)
- ❌ No UI implemented (returns 404 for /)
- ✅ SSE progress events working
- ⚠️ Designed to be called by wkmp-ui, NOT standalone

**This contradicts user's clarified intent.**

### Required Changes for Alignment

#### 1. Add wkmp-ai Web UI (High Priority)

**Files to Create:**
```
wkmp-ai/
├── static/                    # NEW
│   ├── index.html            # Import wizard home page
│   ├── import-progress.html  # Real-time progress display
│   ├── segment-editor.html   # Waveform editor (IMPL005 Step 4)
│   ├── css/
│   │   └── import-wizard.css
│   └── js/
│       ├── sse-client.js     # SSE event handling
│       ├── waveform-editor.js # Canvas-based waveform display
│       └── import-wizard.js
```

**Axum Routes to Add:**
```rust
// Serve static files
.route("/", get(serve_index))
.route("/import-progress", get(serve_progress_page))
.route("/segment-editor", get(serve_segment_editor))
.nest_service("/static", ServeDir::new("wkmp-ai/static"))

// Existing API routes
.route("/import/start", post(start_import))
.route("/import/events", get(import_event_stream))
// ...
```

**User Flow:**
1. User opens http://localhost:5720 (wkmp-ui)
2. Clicks "Import Music" → Opens http://localhost:5723 (wkmp-ai) in new tab
3. wkmp-ai shows import wizard UI
4. User selects folders, tunes parameters
5. Starts import → Progress page with SSE updates
6. For multi-track files → Segment editor (waveform display, drag boundaries)
7. Import complete → "Return to WKMP" button → Back to wkmp-ui

**Effort:** 2-3 weeks (new front-end work)

#### 2. Update wkmp-ui Integration (Medium Priority)

**wkmp-ui Changes:**
```rust
// Check if wkmp-ai is running
async fn check_wkmp_ai_health() -> bool {
    reqwest::get("http://localhost:5723/health")
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

// In library view template:
// <button id="import-music-btn" onclick="window.open('http://localhost:5723', '_blank')">
//   Import Music
// </button>
```

**Disable button if wkmp-ai not running** (Lite/Minimal versions)

**Effort:** 1-2 days

#### 3. Update Documentation (High Priority)

**Files to Update:**
- `docs/SPEC032-audio_ingest_architecture.md` - Add UI Architecture section
- `docs/IMPL005-audio_file_segmentation.md` - Clarify UI location
- `docs/SPEC009-ui_specification.md` - Clarify Passage Editor ownership
- `docs/REQ001-requirements.md` - Add microservice UI ownership table
- `CLAUDE.md` - Update module descriptions, add on-demand clarification

**Effort:** 1 day

---

## Summary and Recommendations

### Documentation Issues Identified

1. **SPEC032** says "wkmp-ui orchestrates" but also mentions wkmp-ai has "User review and manual adjustment UI" (contradictory)
2. **IMPL005** describes complex interactive UI workflow but doesn't say WHERE UI lives
3. **SPEC009** describes Passage Editor as if it's in wkmp-ui, but IMPL005 workflow is clearly wkmp-ai territory
4. **CLAUDE.md** describes wkmp-ai as "integration" (sounds like backend) not "interface" (like wkmp-le)
5. **"on-demand" pattern never defined** - unclear how user accesses wkmp-ai vs. wkmp-le

### Root Cause

**Specification was written assuming TWO possible architectures:**
- **Option A:** wkmp-ui provides all UI, wkmp-ai is pure backend
- **Option B:** wkmp-ai provides own UI for specialized workflows

**But never explicitly chose one** → Ambiguity throughout specs

### User's Clarified Intent

**wkmp-ai SHOULD have dedicated UI** (Option B)
- Complex one-time operations warrant specialized interface
- Consistent with wkmp-le pattern ("on-demand" tools with own UI)
- Decoupled development and maintenance

### Immediate Actions

**1. Update Documentation (1 day):**
- Apply all recommended fixes above
- Make Option B explicit throughout specs
- Define "on-demand" pattern clearly

**2. Implement wkmp-ai Web UI (2-3 weeks):**
- Static file serving (index.html, CSS, JS)
- Import wizard interface
- Waveform editor (IMPL005 Step 4)
- SSE client for real-time progress

**3. Update wkmp-ui Integration (1-2 days):**
- Add "Import Music" button that opens wkmp-ai in new tab
- Health check to enable/disable button
- Link back from wkmp-ai to wkmp-ui on completion

**Total Effort:** ~3 weeks for full alignment

---

## Questions for Review

1. **Confirm architecture:** Should wkmp-ai provide its own UI (Option B) as user clarified?
2. **Launch mechanism:** New browser tab vs. embedded iframe vs. popup window?
3. **Shared UI code:** Should waveform rendering be in `wkmp-common` for reuse in wkmp-ui's post-import editor?
4. **wkmp-le consistency:** Should wkmp-le follow same pattern? (Seems implied but not specified)

---

**END OF ANALYSIS**
