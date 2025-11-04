# SSE Connection Status Troubleshooting Guide

## Issue: wkmp-ai Root Page Shows "Connecting..." Stuck

### Root Cause
The wkmp-ai server is running an **old version** from before the SSE integration changes. The old binary doesn't have the `/events` endpoint, so the browser can't connect.

### Solution

**Step 1: Stop the running wkmp-ai server**
```bash
# On Windows:
taskkill /F /IM wkmp-ai.exe

# On Linux/Mac:
pkill wkmp-ai
```

**Step 2: Rebuild wkmp-ai**
```bash
cargo build -p wkmp-ai
```

**Step 3: Start the new version**
```bash
cargo run -p wkmp-ai
```

**Step 4: Test in browser**
- Open http://localhost:5723
- Status badge should show "Connecting..." briefly, then "Connected"
- Check browser console (F12) for log: "SSE connection opened to /events"

---

## How to Verify It's Working

### Backend (Server) Checks

**1. Check the route exists:**
```bash
curl -I http://localhost:5723/events
```
Expected: HTTP 200 with `content-type: text/event-stream`

**2. Check SSE stream is working:**
```bash
curl -N http://localhost:5723/events
```
Expected output:
```
event: ConnectionStatus
data: connected

: heartbeat
```

**3. Check JavaScript file is served:**
```bash
curl -I http://localhost:5723/static/wkmp-sse.js
```
Expected: HTTP 200 with `content-type: application/javascript`

### Frontend (Browser) Checks

**1. Open browser console (F12 → Console tab)**

**Expected logs:**
```
SSE connection opened to /events
```

**2. Check Network tab (F12 → Network tab)**
- Look for request to `/events`
- Type should be "eventsource"
- Status should be "200" or "pending" (stays open)

**3. Check for JavaScript errors**
- If you see: "WkmpSSEConnection is not defined" → Script didn't load
- If you see: "Failed to load resource: /static/wkmp-sse.js" → File not found
- If you see: "EventSource failed" → Backend endpoint not responding

---

## Common Issues

### Issue 1: Script Error (WkmpSSEConnection not defined)

**Symptoms:**
- Status shows "Script Error" or "Connecting..." stuck
- Console shows: "WkmpSSEConnection is not defined"

**Cause:** `/static/wkmp-sse.js` file not loading

**Fix:**
1. Check file exists: `ls wkmp-ai/static/wkmp-sse.js`
2. Restart server (files are loaded at startup)

### Issue 2: Connection Stuck at "Connecting..."

**Symptoms:**
- Status shows "Connecting..." indefinitely
- No console errors about WkmpSSEConnection

**Cause:** `/events` endpoint not responding or not exists

**Fix:**
1. Check server is running: `curl http://localhost:5723/health`
2. Check `/events` exists: `curl -N http://localhost:5723/events`
3. Rebuild and restart server

### Issue 3: "Script Error" Message

**Symptoms:**
- Status badge shows "Script Error"

**Cause:** `wkmp-sse.js` failed to load or has syntax error

**Fix:**
1. Check file syntax: `node -c wkmp-ai/static/wkmp-sse.js` (if node installed)
2. Check browser console for specific error
3. Verify file copied correctly from `wkmp-common/static/wkmp-sse.js`

### Issue 4: Old Version Still Running

**Symptoms:**
- Changes don't appear after rebuild
- Build fails with "Access is denied" error

**Cause:** Old binary is locked (still running)

**Fix:**
```bash
# Windows:
taskkill /F /IM wkmp-ai.exe
cargo build -p wkmp-ai
cargo run -p wkmp-ai

# Linux/Mac:
pkill wkmp-ai
cargo build -p wkmp-ai
cargo run -p wkmp-ai
```

### Issue 5: ServeDir Path Resolution Failure (RESOLVED)

**Symptoms:**
- All static files return HTTP 404
- Status shows "Script Error"
- Server responds (proves it's running) but files not found

**Root Cause:**
`tower_http::services::ServeDir` was configured with relative path `"wkmp-ai/static"`, which depends on the working directory at runtime. Path resolution can fail depending on how the binary is executed.

**Fix:**
Replace `ServeDir` with compile-time embedding using `include_str!` (same pattern as wkmp-dr):

**Before (broken):**
```rust
.nest_service(
    "/static",
    ServiceBuilder::new()
        .layer(SetResponseHeaderLayer::overriding(...))
        .service(ServeDir::new("wkmp-ai/static"))  // ❌ Runtime path resolution
)
```

**After (working):**
```rust
// Embed at compile time
const WKMP_SSE_JS: &str = include_str!("../../../wkmp-common/static/wkmp-sse.js");
const WKMP_UI_CSS: &str = include_str!("../../../wkmp-common/static/wkmp-ui.css");
const IMPORT_PROGRESS_JS: &str = include_str!("../../static/import-progress.js");

// Route directly to handlers
.route("/static/wkmp-sse.js", get(serve_wkmp_sse_js))
.route("/static/wkmp-ui.css", get(serve_wkmp_ui_css))
.route("/static/import-progress.js", get(serve_import_progress_js))
```

**Benefits of include_str! approach:**
- No runtime file system access needed
- Works regardless of working directory
- Files bundled into binary (no deployment issues)
- Consistent with wkmp-dr implementation
- More secure (no file system traversal risk)

---

## Testing the SSE Connection

### Manual Test
1. Start wkmp-ai: `cargo run -p wkmp-ai`
2. Open http://localhost:5723 in browser
3. Watch status badge:
   - Should show "Connecting..." briefly (< 1 second)
   - Then "Connected" with green background
4. Stop server (Ctrl+C)
5. Status badge should change to "Disconnected" with red background

### Automated Test (using curl)
```bash
# Terminal 1: Start server
cargo run -p wkmp-ai

# Terminal 2: Test SSE endpoint
curl -N http://localhost:5723/events

# Expected output:
# event: ConnectionStatus
# data: connected
#
# : heartbeat
# (repeats every 15 seconds)
```

---

## All Three Modules Status

| Module | Port | SSE Endpoint | Status |
|--------|------|--------------|--------|
| **wkmp-dr** | 5725 | `/api/events` | ✅ Working (serves via include_str) |
| **wkmp-ai** | 5723 | `/events` | ✅ Fixed (now serves via include_str, issue was ServeDir path resolution) |
| **wkmp-ap** | 5721 | `/events` | ℹ️ Not modified (has own complex SSE) |

---

## Quick Reference: File Locations

### wkmp-ai
- **Backend SSE:** `wkmp-ai/src/api/sse.rs` (line 21-25: `event_stream()`)
- **Router:** `wkmp-ai/src/lib.rs` (line 63: `.route("/events", get(api::event_stream))`)
- **Static JS:** `wkmp-ai/static/wkmp-sse.js` (served via ServeDir)
- **HTML pages:** `wkmp-ai/src/api/ui.rs` (4 pages, all include `/static/wkmp-sse.js`)

### wkmp-dr
- **Backend SSE:** `wkmp-dr/src/api/sse.rs` (line 19-22: `event_stream()`)
- **Router:** `wkmp-dr/src/lib.rs` (line 54: `.route("/api/events", get(api::event_stream))`)
- **Static JS:** `wkmp-dr/src/ui/` (embedded via include_str in `ui.rs`)
- **HTML:** `wkmp-dr/src/ui/index.html` (includes `/static/wkmp-common/wkmp-sse.js`)

### Shared Code
- **Rust SSE helper:** `wkmp-common/src/sse.rs`
- **JavaScript client:** `wkmp-common/static/wkmp-sse.js`
- **Shared CSS:** `wkmp-common/static/wkmp-ui.css`

---

## Next Steps

**To fix the immediate issue:**
1. Stop wkmp-ai server (Ctrl+C or `taskkill /F /IM wkmp-ai.exe`)
2. Rebuild: `cargo build -p wkmp-ai`
3. Start: `cargo run -p wkmp-ai`
4. Refresh browser at http://localhost:5723

**Expected result:** Status badge shows "Connecting..." then "Connected" ✅
