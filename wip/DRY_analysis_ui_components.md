# DRY Analysis: UI Components and Styles Across WKMP Modules

**Date:** 2025-11-03
**Scope:** wkmp-ap, wkmp-ai, wkmp-dr (current modules), future modules
**Focus:** UI headers, CSS styles, JavaScript patterns, SSE implementations

---

## Executive Summary

**Key Finding:** Significant code duplication exists across three microservices with web UIs (wkmp-ap, wkmp-ai, wkmp-dr), with future modules (wkmp-le, wkmp-ui) likely to repeat the same patterns.

**Impact:**
- **Maintenance burden:** CSS/HTML/JS changes require updates in 3+ files
- **Inconsistency risk:** Manual synchronization leads to visual/behavioral drift
- **Onboarding friction:** New modules must recreate standard components
- **Context window waste:** Repetitive code consumes AI context unnecessarily

**Recommendation:** Extract shared UI components, styles, and JavaScript utilities to `wkmp-common` for reuse across all modules.

---

## Category 1: CSS Styles (HIGH DUPLICATION)

### 1.1 Header Styles

**Duplicated across all 3 modules:**

```css
/* Standard header pattern - EXACT DUPLICATION */
header {
    background-color: #2a2a2a;
    border-bottom: 1px solid #3a3a3a;
    padding: 20px;
    margin-bottom: 30px;
}

.header-content {
    display: flex;
    justify-content: space-between;
    align-items: center;
}

.header-left { flex: 1; }

.header-right {
    text-align: right;
    font-size: 16px;
    color: #888;
    font-family: 'Courier New', monospace;
    line-height: 1.2;
}

h1 {
    font-size: 26px;
    margin-bottom: 5px;
    color: #4a9eff;
    display: flex;
    align-items: center;
    gap: 10px;
}

.subtitle {
    color: #888;
    font-size: 16px;
}

.build-info-line {
    margin-bottom: 1px;
}
```

**Files with this duplication:**
- `wkmp-ap/src/api/developer_ui.html` (lines 17-52)
- `wkmp-dr/src/ui/index.html` (lines 36-67)
- `wkmp-ai/src/api/ui.rs` (4 page functions, repeated in each)

**Estimated duplication:** ~150 lines CSS × 3 modules = **450 lines**

### 1.2 Connection Status Badge Styles

**Duplicated across all 3 modules:**

```css
/* Connection status badge - EXACT DUPLICATION */
.connection-status {
    display: inline-block;
    padding: 3px 8px;
    border-radius: 10px;
    font-size: 12px;
    font-weight: 600;
    margin-left: 10px;
}
.status-connected {
    background: #10b981;
    color: #fff;
}
.status-connecting {
    background: #f59e0b;
    color: #fff;
}
.status-disconnected {
    background: #ef4444;
    color: #fff;
}
```

**Files with this duplication:**
- `wkmp-ap/src/api/developer_ui.html` (lines 221-237)
- `wkmp-dr/src/ui/index.html` (lines 83-102)
- `wkmp-ai/src/api/ui.rs` (4 pages × ~20 lines each = 80 lines total)

**Estimated duplication:** ~25 lines CSS × 3 modules = **75 lines**

### 1.3 Base Styles & Dark Theme

**Duplicated across all 3 modules:**

```css
/* Base/reset styles - NEAR-IDENTICAL */
* {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
}

body {
    font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
    background: #1a1a1a;
    color: #e0e0e0;
    line-height: 1.6;
}
```

**Notable:** wkmp-dr uses CSS variables (`:root { --primary-color: #4a9eff; ... }`), but others hardcode colors.

**Estimated duplication:** ~100 lines base CSS × 3 modules = **300 lines**

---

## Category 2: JavaScript Patterns (HIGH DUPLICATION)

### 2.1 Connection Status Update Function

**Exact duplication across 6 locations:**

```javascript
function updateConnectionStatus(status) {
    const statusEl = document.getElementById('connection-status');
    if (statusEl) {
        statusEl.className = 'connection-status status-' + status;
        statusEl.textContent = status === 'connected' ? 'Connected' :
                              status === 'connecting' ? 'Connecting...' : 'Disconnected';
    }
}
```

**Locations:**
1. `wkmp-ap/src/api/developer_ui.html:1202`
2. `wkmp-dr/src/ui/app.js:774`
3. `wkmp-ai/src/api/ui.rs` (root page):195
4. `wkmp-ai/src/api/ui.rs` (segment-editor):793
5. `wkmp-ai/src/api/ui.rs` (import-complete):964
6. `wkmp-ai/static/import-progress.js:58`

**Estimated duplication:** ~10 lines × 6 locations = **60 lines**

### 2.2 SSE Connection Initialization Pattern

**Duplicated across 6 locations:**

```javascript
updateConnectionStatus('connecting');
const eventSource = new EventSource('/events');

eventSource.onopen = () => {
    console.log('SSE connection opened');
    updateConnectionStatus('connected');
};

eventSource.onerror = (err) => {
    console.error('SSE connection error:', err);
    updateConnectionStatus('disconnected');
};
```

**Same 6 locations as above.**

**Estimated duplication:** ~15 lines × 6 locations = **90 lines**

### 2.3 Build Info Population

**Similar pattern in all 3 modules:**

```javascript
// wkmp-ap pattern (3-line build info)
document.getElementById('build-info').innerHTML = `
    <div class="build-info-line">wkmp-ap v${buildData.version}</div>
    <div class="build-info-line">${buildData.git_hash} (${buildData.build_profile})</div>
    <div class="build-info-line">${buildData.build_timestamp}</div>
`;
```

**Locations:**
- `wkmp-ap/src/api/developer_ui.html` (JavaScript)
- `wkmp-dr/src/ui/index.html` (inline in HTML via template)
- `wkmp-ai/src/api/ui.rs` (Rust format strings, 4 pages)

**Variation:** wkmp-dr and wkmp-ai use server-side rendering, wkmp-ap uses client-side fetch + JS templating.

---

## Category 3: SSE Implementation (MEDIUM DUPLICATION)

### 3.1 Server-Side SSE Handlers

**Three nearly-identical SSE endpoint implementations:**

#### wkmp-ap (`src/api/sse.rs`): 137 lines
- Complex: broadcasts `WkmpEvent` from `EventBus`
- Initial state fetch (queue, position, volume)
- Event-driven stream

#### wkmp-dr (`src/api/sse.rs`): 48 lines
- Simple: heartbeat-only connection status
- No domain events

#### wkmp-ai (`src/api/sse.rs`): 83 lines (2 functions)
- `event_stream()`: Heartbeat-only (48 lines, identical to wkmp-dr)
- `import_event_stream()`: Import-specific events (35 lines)

**Common pattern (EXACT DUPLICATION in wkmp-dr and wkmp-ai):**

```rust
pub async fn event_stream(
    State(_state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    info!("New SSE client connected to general events");

    let stream = async_stream::stream! {
        info!("SSE: General event stream started");

        yield Ok(Event::default()
            .event("ConnectionStatus")
            .data("connected"));

        loop {
            tokio::time::sleep(Duration::from_secs(15)).await;
            debug!("SSE: Sending heartbeat");
            yield Ok(Event::default().comment("heartbeat"));
        }
    };

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("heartbeat")
    )
}
```

**Estimated duplication:** ~40 lines × 2 modules (wkmp-dr, wkmp-ai) = **80 lines**

---

## Category 4: HTML Structure (MEDIUM DUPLICATION)

### 4.1 Header HTML Template

**Standard header structure (used in all modules):**

```html
<header>
    <div class="header-content">
        <div class="header-left">
            <h1>
                WKMP [Module Name]
                <span class="connection-status" id="connection-status">Connecting...</span>
            </h1>
            <p class="subtitle">[Module subtitle]</p>
        </div>
        <div class="header-right">
            <div class="build-info-line">wkmp-XX v{version}</div>
            <div class="build-info-line">{git_hash} ({build_profile})</div>
            <div class="build-info-line">{build_timestamp}</div>
        </div>
    </div>
</header>
```

**Files with this pattern:**
- `wkmp-ap/src/api/developer_ui.html`
- `wkmp-dr/src/ui/index.html`
- `wkmp-ai/src/api/ui.rs` (4 page functions)

**Variation points:**
- Module name (`WKMP Audio Player`, `WKMP Database Review`, `WKMP Audio Import`)
- Subtitle text
- Module ID in build info (`wkmp-ap`, `wkmp-dr`, `wkmp-ai`)

**Estimated duplication:** ~20 lines HTML × 3 modules × multiple pages = **~200 lines**

---

## Category 5: Architecture Patterns (LOW DUPLICATION, HIGH CONSISTENCY VALUE)

### 5.1 Zero-Configuration Startup

**All modules implement identical pattern:**

```rust
#[tokio::main]
async fn main() -> Result<()> {
    // Step 0: Initialize tracing
    tracing_subscriber::registry()...

    // Step 1: Log build info (ARCH-INIT-004)
    info!("Starting WKMP [Module] (module-id) v{} [{}] built {} ({})",
        env!("CARGO_PKG_VERSION"), env!("GIT_HASH"),
        env!("BUILD_TIMESTAMP"), env!("BUILD_PROFILE"));

    // Step 2: Resolve root folder (4-tier priority)
    let resolver = wkmp_common::config::RootFolderResolver::new("module-name");
    let root_folder = resolver.resolve();

    // Step 3: Create directory if missing
    let initializer = wkmp_common::config::RootFolderInitializer::new(root_folder);
    initializer.ensure_directory_exists()?;

    // Step 4: Get database path
    let db_path = initializer.database_path();
    ...
}
```

**Status:** Already DRY via `wkmp_common::config` utilities ✓

**Not duplicated, but worth documenting as successful DRY example.**

---

## Proposed Solutions

### Solution 1: Shared CSS via `wkmp-common/static/` (RECOMMENDED)

**Implementation:**

1. Create `wkmp-common/static/wkmp-ui.css` with shared styles:
   - Base reset & dark theme
   - Header styles (`.header-content`, `.header-left`, `.header-right`, etc.)
   - Connection status badge (`.connection-status`, `.status-*`)
   - Standard component styles (buttons, forms, panels)
   - CSS variables for theming (like wkmp-dr currently uses)

2. Update each module's HTML to include:
   ```html
   <link rel="stylesheet" href="/static/wkmp-common.css">
   <style>
   /* Module-specific styles only */
   </style>
   ```

3. Configure tower-http `ServeDir` in each module to serve `wkmp-common/static/`:
   ```rust
   .nest_service("/static/wkmp-common", ServeDir::new("wkmp-common/static"))
   ```

**Benefits:**
- Single source of truth for standard styles
- Consistency guaranteed across all modules
- Reduced CSS from ~450 lines/module to ~50 lines/module
- New modules get standard styling "for free"

**Effort:** Low (2-3 hours)

---

### Solution 2: Shared JavaScript via `wkmp-common/static/wkmp-sse.js` (RECOMMENDED)

**Implementation:**

1. Create `wkmp-common/static/wkmp-sse.js`:
   ```javascript
   // SSE connection management utility
   class WkmpSSEConnection {
       constructor(endpoint, statusElementId) {
           this.endpoint = endpoint;
           this.statusElementId = statusElementId;
           this.eventSource = null;
       }

       connect() {
           this.updateStatus('connecting');
           this.eventSource = new EventSource(this.endpoint);

           this.eventSource.onopen = () => {
               console.log(`SSE connection opened to ${this.endpoint}`);
               this.updateStatus('connected');
           };

           this.eventSource.onerror = (err) => {
               console.error(`SSE connection error for ${this.endpoint}:`, err);
               this.updateStatus('disconnected');
           };

           return this.eventSource;
       }

       updateStatus(status) {
           const statusEl = document.getElementById(this.statusElementId);
           if (statusEl) {
               statusEl.className = 'connection-status status-' + status;
               statusEl.textContent = status === 'connected' ? 'Connected' :
                                     status === 'connecting' ? 'Connecting...' : 'Disconnected';
           }
       }

       addEventListener(eventType, handler) {
           if (this.eventSource) {
               this.eventSource.addEventListener(eventType, handler);
           }
       }

       close() {
           if (this.eventSource) {
               this.eventSource.close();
           }
       }
   }
   ```

2. Update each page to use:
   ```html
   <script src="/static/wkmp-common/wkmp-sse.js"></script>
   <script>
       const sse = new WkmpSSEConnection('/events', 'connection-status');
       sse.connect();
       // Module-specific event handlers
       sse.addEventListener('CustomEvent', handleCustomEvent);
   </script>
   ```

**Benefits:**
- Eliminates ~150 lines duplicated JavaScript
- Consistent SSE connection behavior
- Easier to add features (reconnection logic, exponential backoff, etc.)
- Testable as independent module

**Effort:** Low (2-3 hours)

---

### Solution 3: HTML Header Template Function in `wkmp-common` (MODERATE EFFORT)

**Implementation:**

Option A: Server-side template helper in `wkmp-common/src/ui_templates.rs`:

```rust
pub struct HeaderConfig {
    pub module_name: &'static str,
    pub subtitle: &'static str,
    pub module_id: &'static str,
}

pub fn render_standard_header(config: HeaderConfig) -> String {
    format!(r#"
<header>
    <div class="header-content">
        <div class="header-left">
            <h1>
                {}
                <span class="connection-status" id="connection-status">Connecting...</span>
            </h1>
            <p class="subtitle">{}</p>
        </div>
        <div class="header-right">
            <div class="build-info-line">{} v{}</div>
            <div class="build-info-line">{} ({})</div>
            <div class="build-info-line">{}</div>
        </div>
    </div>
</header>
    "#,
    config.module_name,
    config.subtitle,
    config.module_id,
    env!("CARGO_PKG_VERSION"),
    &env!("GIT_HASH")[..8],
    env!("BUILD_PROFILE"),
    env!("BUILD_TIMESTAMP")
    )
}
```

Usage in module:
```rust
let header_html = wkmp_common::ui_templates::render_standard_header(
    HeaderConfig {
        module_name: "WKMP Audio Player",
        subtitle: "Developer interface",
        module_id: "wkmp-ap",
    }
);
```

**Benefits:**
- Type-safe header generation
- Guaranteed consistency
- Centralized build info formatting

**Drawbacks:**
- Rust-only (doesn't help with pure HTML files like wkmp-dr/src/ui/index.html)
- Requires refactoring static HTML to use server-side rendering

**Effort:** Moderate (4-6 hours including refactoring)

---

### Solution 4: Shared SSE Heartbeat Function in `wkmp-common` (LOW EFFORT, HIGH VALUE)

**Implementation:**

Add to `wkmp-common/src/sse.rs`:

```rust
use axum::response::sse::{Event, Sse};
use futures::stream::Stream;
use std::convert::Infallible;
use std::time::Duration;
use tracing::{debug, info};

/// Create a simple heartbeat-only SSE stream for connection status monitoring
///
/// Used by on-demand microservices (wkmp-dr, wkmp-ai, wkmp-le) that don't
/// have domain events to broadcast but still need connection status UI.
pub fn create_heartbeat_sse_stream(
    service_name: &'static str,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    info!("New SSE client connected to {} general events", service_name);

    let stream = async_stream::stream! {
        info!("SSE: {} event stream started", service_name);

        // Send initial connected status
        yield Ok(Event::default()
            .event("ConnectionStatus")
            .data("connected"));

        loop {
            // Heartbeat every 15 seconds
            tokio::time::sleep(Duration::from_secs(15)).await;
            debug!("SSE: Sending heartbeat");
            yield Ok(Event::default().comment("heartbeat"));
        }
    };

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("heartbeat")
    )
}
```

Usage in `wkmp-dr/src/api/sse.rs` and `wkmp-ai/src/api/sse.rs`:

```rust
pub async fn event_stream(
    State(_state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    wkmp_common::sse::create_heartbeat_sse_stream("wkmp-dr")
}
```

**Benefits:**
- Eliminates 40-line duplication in wkmp-dr and wkmp-ai
- Future modules (wkmp-le) get heartbeat SSE for free
- Consistent heartbeat timing across all modules

**Effort:** Very Low (1 hour)

**Note:** wkmp-ap has complex SSE (EventBus broadcasting) and wouldn't use this helper.

---

## Implementation Priority

### Phase 1: Quick Wins (Low Effort, High Impact)
1. **Solution 4:** Shared SSE heartbeat function in `wkmp-common` (1 hour)
   - Immediate impact: Eliminates 80 lines duplication
   - Future-proof: wkmp-le will use this

2. **Solution 2:** Shared JavaScript SSE client in `wkmp-common/static/` (2-3 hours)
   - Immediate impact: Eliminates ~150 lines JavaScript duplication
   - Future benefit: Reconnection logic, error handling improvements benefit all modules

### Phase 2: CSS Consolidation (Moderate Effort, High Long-Term Value)
3. **Solution 1:** Shared CSS in `wkmp-common/static/wkmp-ui.css` (2-3 hours)
   - Immediate impact: Eliminates ~825 lines CSS duplication (450 header + 75 badges + 300 base)
   - Future benefit: Visual consistency guaranteed, new modules get standard styling

### Phase 3: Optional Enhancements (Higher Effort, Lower Priority)
4. **Solution 3:** HTML template functions (4-6 hours)
   - Deferred until templating strategy decided
   - May conflict with static HTML preference (wkmp-dr approach)

---

## Future Modules Impact Analysis

### wkmp-le (Lyric Editor)
**Current status:** Not yet implemented
**Expected UI needs:**
- Standard header (module name: "WKMP Lyric Editor", subtitle: "Synchronized lyric editing")
- Connection status badge
- SSE heartbeat for connection monitoring
- Dark theme base styles

**With proposed DRY improvements:**
- Include `wkmp-common/static/wkmp-ui.css` → Standard header/badge/theme for free
- Include `wkmp-common/static/wkmp-sse.js` → Connection status for free
- Use `wkmp_common::sse::create_heartbeat_sse_stream("wkmp-le")` → Backend SSE for free

**Estimated savings:** ~300 lines CSS + ~50 lines JS + ~40 lines Rust = **~390 lines** not written

### wkmp-ui (User Interface - Main Playback UI)
**Current status:** Not yet implemented
**Expected UI needs:**
- Standard header
- Connection status badge
- Complex SSE (playback events, similar to wkmp-ap)
- Dark theme base styles
- Custom playback controls (not shared)

**With proposed DRY improvements:**
- Standard CSS/JS → ~350 lines saved
- SSE client library → ~50 lines saved (connection status handling)
- Custom SSE backend (like wkmp-ap, not using heartbeat helper)

**Estimated savings:** **~400 lines**

---

## Measurement Metrics

### Current State (Before DRY)
- **Total duplicated CSS:** ~825 lines (header + badges + base × 3 modules)
- **Total duplicated JavaScript:** ~150 lines (updateConnectionStatus + SSE init × 6 locations)
- **Total duplicated Rust (SSE):** ~80 lines (heartbeat SSE × 2 modules)
- **Total duplication:** **~1,055 lines** across 3 modules

### After Phase 1 (Quick Wins)
- **Eliminated:** ~230 lines (JS + Rust SSE)
- **Remaining:** ~825 lines CSS

### After Phase 2 (CSS Consolidation)
- **Eliminated:** ~1,055 lines total
- **New shared code:** ~300 lines (wkmp-common/static/wkmp-ui.css + wkmp-sse.js + wkmp-common/src/sse.rs)
- **Net reduction:** **~755 lines** (72% reduction)

### Future Modules (wkmp-le + wkmp-ui)
- **Additional savings:** ~790 lines not written (2 modules × ~395 lines/module)

### Total Long-Term Impact
- **Total lines eliminated/prevented:** ~1,845 lines (current duplication + future module savings)
- **Maintenance burden reduction:** Changes to standard components require 1 update (not 5)
- **Consistency improvement:** Visual/behavioral drift eliminated

---

## Risks and Considerations

### Risk 1: Breaking Changes in Shared Code
**Mitigation:**
- Version shared CSS/JS files if breaking changes needed (`wkmp-ui-v1.css`, `wkmp-ui-v2.css`)
- Document breaking changes in CHANGELOG
- Test all modules before merging shared component updates

### Risk 2: Module-Specific Customization Needs
**Mitigation:**
- Shared CSS uses CSS variables for theming (allow overrides)
- Shared JS classes accept configuration objects
- Module-specific styles still allowed (append after shared CSS)

### Risk 3: Build Complexity (Static File Serving)
**Current:** Each module serves static files from its own directory
**With shared files:** Each module needs to serve `wkmp-common/static/` at `/static/wkmp-common/`

**Mitigation:**
- Use tower-http `ServeDir` (already in use, proven pattern)
- Document in IMPL002 (coding conventions) or create new IMPL007 (UI standards)

---

## Recommendation

**Implement Phases 1 and 2 (total ~6-8 hours effort):**

1. Create `wkmp-common/src/sse.rs` with `create_heartbeat_sse_stream()` helper
2. Create `wkmp-common/static/wkmp-sse.js` with `WkmpSSEConnection` class
3. Create `wkmp-common/static/wkmp-ui.css` with shared header/badge/base styles
4. Update wkmp-ap, wkmp-ai, wkmp-dr to use shared components
5. Document in IMPL002 or new IMPL007 (UI Component Standards)

**Benefits:**
- Immediate: Eliminate ~1,055 lines duplication across 3 modules (72% reduction)
- Future: Save ~790 lines when implementing wkmp-le and wkmp-ui
- Maintenance: Single source of truth for standard UI components
- Consistency: Visual/behavioral consistency guaranteed

**Defer Phase 3 (HTML templates) until:**
- Templating strategy decided (server-side rendering vs. static HTML)
- Pattern emerges across more modules showing clear need

---

## Appendix: File Locations

### CSS Duplication Locations
- `wkmp-ap/src/api/developer_ui.html` (lines 8-250, ~242 lines CSS)
- `wkmp-dr/src/ui/index.html` (lines 8-280, ~272 lines CSS)
- `wkmp-ai/src/api/ui.rs`:
  - `root_page()` (lines 54-150, ~96 lines CSS)
  - `import_progress_page()` (lines 228-532, ~304 lines CSS)
  - `segment_editor_page()` (lines 640-730, ~90 lines CSS)
  - `import_complete_page()` (lines 805-901, ~96 lines CSS)

### JavaScript Duplication Locations
- `wkmp-ap/src/api/developer_ui.html` (lines 1202-1215, ~13 lines)
- `wkmp-dr/src/ui/app.js` (lines 774-810, ~36 lines)
- `wkmp-ai/src/api/ui.rs` (3 inline scripts, ~20 lines each = 60 lines)
- `wkmp-ai/static/import-progress.js` (lines 58-82 + 267-280, ~40 lines)

### Rust SSE Duplication Locations
- `wkmp-dr/src/api/sse.rs` (lines 21-48, heartbeat SSE)
- `wkmp-ai/src/api/sse.rs` (lines 21-48, heartbeat SSE - identical)
- `wkmp-ap/src/api/sse.rs` (lines 29-134, complex EventBus SSE - NOT duplicated)

---

**End of Analysis**
