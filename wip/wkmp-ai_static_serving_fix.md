# wkmp-ai Static File Serving Fix

**Date:** 2025-11-03
**Status:** ✅ RESOLVED
**Impact:** All wkmp-ai static files (JS, CSS) now served correctly

---

## Problem Summary

After implementing the DRY improvements (shared SSE components and CSS), wkmp-ai's static file serving broke:
- All requests to `/static/*` returned HTTP 404
- Browser showed "Script Error" in connection status badge
- `WkmpSSEConnection` class undefined (script didn't load)

## Root Cause

**tower_http::services::ServeDir path resolution issue:**

wkmp-ai was using `ServeDir::new("wkmp-ai/static")` with a relative path that depended on runtime working directory. This created a fragile configuration where:
- Path worked when running from workspace root
- Path could fail depending on execution context
- No clear error message (just 404s)

The `cache-control` header was present in 404 responses, confirming requests reached our router but ServeDir couldn't find the files.

## Solution

**Replaced ServeDir with include_str! embedding** (same pattern as wkmp-dr):

### Changes Made

**File:** [wkmp-ai/src/api/ui.rs](wkmp-ai/src/api/ui.rs)

**1. Added compile-time constants (lines 15-21):**
```rust
// Embed static files at compile time (same pattern as wkmp-dr)
const WKMP_SSE_JS: &str = include_str!("../../../wkmp-common/static/wkmp-sse.js");
const WKMP_UI_CSS: &str = include_str!("../../../wkmp-common/static/wkmp-ui.css");
const IMPORT_PROGRESS_JS: &str = include_str!("../../static/import-progress.js");
const SETTINGS_HTML: &str = include_str!("../../static/settings.html");
const SETTINGS_CSS: &str = include_str!("../../static/settings.css");
const SETTINGS_JS: &str = include_str!("../../static/settings.js");
```

**2. Replaced nest_service with direct routes (lines 23-35):**
```rust
pub fn ui_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(root_page))
        .route("/import-progress", get(import_progress_page))
        .route("/segment-editor", get(segment_editor_page))
        .route("/import-complete", get(import_complete_page))
        .route("/settings", get(settings_page))
        .route("/static/wkmp-sse.js", get(serve_wkmp_sse_js))
        .route("/static/wkmp-ui.css", get(serve_wkmp_ui_css))
        .route("/static/import-progress.js", get(serve_import_progress_js))
        .route("/static/settings.css", get(serve_settings_css))
        .route("/static/settings.js", get(serve_settings_js))
}
```

**3. Added handler functions for each static file:**
```rust
async fn serve_wkmp_sse_js() -> Response {
    (
        StatusCode::OK,
        [
            ("content-type", "application/javascript"),
            ("cache-control", "no-cache, no-store, must-revalidate"),
        ],
        WKMP_SSE_JS,
    )
        .into_response()
}
// ... similar handlers for CSS and other JS files
```

**4. Simplified settings_page (lines 1034-1036):**
```rust
async fn settings_page() -> impl IntoResponse {
    Html(SETTINGS_HTML)
}
```

### Removed Dependencies

- `tower_http::services::ServeDir` - No longer needed
- `tower_http::set_header::SetResponseHeaderLayer` - Moved to handler tuples
- `tower::ServiceBuilder` - No longer needed

---

## Verification

**All static files now serve correctly:**

```bash
$ curl -I http://localhost:5723/static/wkmp-sse.js
HTTP/1.1 200 OK
content-type: application/javascript
cache-control: no-cache, no-store, must-revalidate
content-length: 2557

$ curl -I http://localhost:5723/static/wkmp-ui.css
HTTP/1.1 200 OK
content-type: text/css
cache-control: no-cache, no-store, must-revalidate
content-length: 5801

$ curl -I http://localhost:5723/static/import-progress.js
HTTP/1.1 200 OK
content-type: application/javascript
content-length: 9894
```

**SSE endpoint working:**
```bash
$ curl -N http://localhost:5723/events
event: ConnectionStatus
data: connected
```

**Server logs confirm connection:**
```
INFO wkmp_common::sse: New SSE client connected to wkmp-ai general events
INFO wkmp_common::sse: SSE: wkmp-ai event stream started
```

---

## Benefits of include_str! Approach

1. **No runtime file system access** - Files embedded at compile time
2. **Working directory independent** - No path resolution issues
3. **Binary contains all assets** - Simpler deployment
4. **Consistent with wkmp-dr** - Same pattern across codebase
5. **More secure** - No file system traversal risk
6. **Faster** - No disk I/O at runtime

---

## Files Modified

- [wkmp-ai/src/api/ui.rs](wkmp-ai/src/api/ui.rs) - Replaced ServeDir with include_str! handlers

## Build Status

✅ **cargo build -p wkmp-ai** - Success
✅ **All static files serving** - HTTP 200 OK
✅ **SSE connection working** - Status shows "Connected"

---

## Related Documents

- [wip/SSE_troubleshooting.md](SSE_troubleshooting.md) - Complete troubleshooting guide
- [wip/CSS_integration_complete.md](CSS_integration_complete.md) - DRY implementation summary
- [wkmp-dr/src/api/ui.rs](../wkmp-dr/src/api/ui.rs) - Reference implementation pattern
