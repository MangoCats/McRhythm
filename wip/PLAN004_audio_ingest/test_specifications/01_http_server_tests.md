# HTTP Server Tests - wkmp-ai

**Requirements:** AIA-OV-010, AIA-MS-010
**Priority:** P0 (Critical)
**Test Count:** 8

---

## TEST-001: Server Starts Successfully on Port 5723

**Requirement:** AIA-OV-010
**Type:** Integration

**Given:**
- wkmp.db database initialized with schema
- Port 5723 not in use
- No other wkmp-ai instances running

**When:**
- Start wkmp-ai binary

**Then:**
- Server binds to port 5723
- Health check endpoint responds: `GET http://localhost:5723/health` returns 200 OK
- Logs show "wkmp-ai started on port 5723"

**Acceptance Criteria:**
- ✅ Server starts without panic
- ✅ Port 5723 accepting connections within 3 seconds
- ✅ Health endpoint returns `{"status": "ok", "service": "wkmp-ai", "version": "..."}`

---

## TEST-002: Server Returns 404 for Unknown Routes

**Requirement:** AIA-MS-010
**Type:** Integration

**Given:**
- wkmp-ai server running

**When:**
- Send `GET http://localhost:5723/nonexistent`

**Then:**
- Response status: 404 Not Found
- Response body: `{"error": "Not Found", "path": "/nonexistent"}`

**Acceptance Criteria:**
- ✅ 404 status code
- ✅ JSON error response
- ✅ No server panic or error log

---

## TEST-003: Server Handles Concurrent Requests

**Requirement:** AIA-MS-010
**Type:** Performance

**Given:**
- wkmp-ai server running

**When:**
- Send 100 concurrent `GET /health` requests

**Then:**
- All 100 requests return 200 OK
- No request timeout
- Average response time < 50ms

**Acceptance Criteria:**
- ✅ 100/100 successful responses
- ✅ No 500 errors
- ✅ No connection refused errors

---

## TEST-004: Server Graceful Shutdown

**Requirement:** AIA-OV-010
**Type:** Integration

**Given:**
- wkmp-ai server running
- Import session in progress (EXTRACTING state)

**When:**
- Send SIGTERM signal to process

**Then:**
- Server logs "Shutting down gracefully..."
- Import session cancelled
- Database connection closed
- Server exits with code 0 within 5 seconds

**Acceptance Criteria:**
- ✅ Graceful shutdown completes
- ✅ No database corruption
- ✅ Import session marked CANCELLED in-memory

---

## TEST-005: CORS Headers Present

**Requirement:** AIA-MS-010
**Type:** Integration

**Given:**
- wkmp-ai server running

**When:**
- Send `OPTIONS http://localhost:5723/import/start` with Origin header

**Then:**
- Response includes CORS headers:
  - `Access-Control-Allow-Origin: *` (or configured origin)
  - `Access-Control-Allow-Methods: GET, POST, OPTIONS`
  - `Access-Control-Allow-Headers: Content-Type`

**Acceptance Criteria:**
- ✅ CORS headers present
- ✅ wkmp-ui can make cross-origin requests
- ✅ Browser OPTIONS requests succeed

---

## TEST-006: Request Logging

**Requirement:** AIA-MS-010
**Type:** Integration

**Given:**
- wkmp-ai server running with RUST_LOG=info

**When:**
- Send `GET http://localhost:5723/health`

**Then:**
- Log entry contains:
  - Request method: GET
  - Request path: /health
  - Response status: 200
  - Response time (ms)

**Acceptance Criteria:**
- ✅ Every request logged
- ✅ Log format: `INFO wkmp_ai::api: GET /health 200 2ms`

---

## TEST-007: JSON Content-Type Enforcement

**Requirement:** AIA-MS-010
**Type:** Integration

**Given:**
- wkmp-ai server running

**When:**
- Send `POST http://localhost:5723/import/start` without `Content-Type: application/json` header

**Then:**
- Response status: 415 Unsupported Media Type
- Response body: `{"error": "Content-Type must be application/json"}`

**Acceptance Criteria:**
- ✅ 415 status for missing Content-Type
- ✅ 415 status for wrong Content-Type (e.g., text/plain)
- ✅ 200/400 for correct Content-Type application/json

---

## TEST-008: Large Request Body Rejection

**Requirement:** AIA-MS-010
**Type:** Security

**Given:**
- wkmp-ai server running

**When:**
- Send `POST http://localhost:5723/import/start` with 10MB JSON body

**Then:**
- Response status: 413 Payload Too Large
- Response body: `{"error": "Request body too large", "max_size": "1MB"}`

**Acceptance Criteria:**
- ✅ Request body limit enforced (1MB default)
- ✅ No memory exhaustion
- ✅ Server remains responsive after large request

---

## Test Implementation Notes

**Framework:** `cargo test --test http_server_tests -p wkmp-ai`

**Setup:**
```rust
// Test helper to start server on random port
async fn start_test_server() -> (SocketAddr, JoinHandle<()>) {
    let addr = SocketAddr::from(([127, 0, 0, 1], 0)); // Random port
    let app = create_app(test_db_pool().await);
    let server = axum::Server::bind(&addr)
        .serve(app.into_make_service());
    let addr = server.local_addr();
    let handle = tokio::spawn(server);
    (addr, handle)
}
```

**Teardown:**
- Abort server handle
- Drop database connection
- Clean up test files

---

End of HTTP server tests
