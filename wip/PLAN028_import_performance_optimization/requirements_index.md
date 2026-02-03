# Requirements Index for PLAN028

## Functional Requirements

| Req ID | Type | Description | Priority | Source |
|--------|------|-------------|----------|--------|
| REQ-PERF-001 | Functional | Implement in-memory progress tracking with periodic database sync (10-second intervals) | HIGH | PERF002 Phase 2.1 |
| REQ-PERF-002 | Functional | Decouple SSE broadcasting from database write operations | HIGH | PERF002 Phase 2.1 |
| REQ-PERF-003 | Functional | Add write-queue manager for serialized database updates with single writer thread | HIGH | PERF002 Phase 3.2 |
| REQ-PERF-007 | Functional | Batch database operations for passage recording (multiple passages per transaction) | MEDIUM | PERF002 Phase 3.1 |

## Non-Functional Requirements

| Req ID | Type | Description | Priority | Source |
|--------|------|-------------|----------|--------|
| REQ-PERF-004 | Performance | Reduce database write operations from O(files) to O(1) - target 50-100x reduction | HIGH | PERF002 Goal |
| REQ-PERF-005 | Reliability | Maintain maximum 10-second data loss window on application crash | MEDIUM | PERF002 Phase 2.1 |
| REQ-PERF-006 | Performance | Support real-time progress updates via SSE with <1 second latency | HIGH | PERF002 Phase 2.1 |
| REQ-PERF-008 | Architecture | Enforce single writer thread for all database operations (SQLite constraint) | HIGH | PERF002 Root Cause |

## Acceptance Criteria

### REQ-PERF-001: In-Memory Progress Tracking
- Progress updates stored in memory using RwLock
- Dirty flag set on any update
- Background task syncs every 10 seconds when dirty
- Sync can be forced on demand (e.g., phase transitions)

### REQ-PERF-002: SSE Decoupling
- SSE events broadcast immediately on progress update
- No database operations in SSE broadcast path
- EventBus used for broadcasting
- Less than 10ms latency from update to broadcast

### REQ-PERF-003: Write Queue Manager
- All database writes go through single queue
- Queue bounded at 1000 items (configurable)
- Single writer thread processes queue sequentially
- Backpressure when queue full

### REQ-PERF-004: O(1) Database Writes
- Import of 1000 files results in <10 database writes
- Import of 5736 files results in <60 database writes
- Measured via SQL query logging

### REQ-PERF-005: Data Loss Window
- Maximum 10 seconds of progress lost on crash
- Verified by killing process and checking database state
- Last sync timestamp tracked

### REQ-PERF-006: Real-Time Updates
- User sees progress updates every 1-2 seconds
- Progress bar moves smoothly
- Current file name updates in UI
- No perceived lag or freezing

### REQ-PERF-007: Batch Operations
- Multiple passages inserted in single transaction
- Transaction size configurable (default 100 passages)
- Rollback on any failure within batch

### REQ-PERF-008: Single Writer
- Only one database connection writing at a time
- Verified via SQLite lock monitoring
- No "database locked" errors

## Constraints

### Technical Constraints
- SQLite database (single writer limitation)
- Tokio async runtime
- Must maintain backward compatibility with existing schema
- Cannot modify other microservices (wkmp-ap, wkmp-ui)

### Performance Constraints
- Memory usage for progress tracking <10MB
- Write queue memory <100MB even at max capacity
- CPU overhead <5% for progress management

### Operational Constraints
- No external dependencies beyond existing crates
- Must work on Windows, Linux, macOS
- Configuration via existing settings table

## Assumptions

1. **EventBus exists and works** - SSE broadcasting infrastructure is functional
2. **Database schema unchanged** - sessions table structure remains same
3. **Import session lifecycle clear** - Created at start, destroyed at end
4. **Error handling exists** - Current error propagation patterns maintained
5. **Testing infrastructure available** - Can create test fixtures with 100-1000 files

## Dependencies

### Code Dependencies
- `wkmp-ai/src/services/workflow_orchestrator/mod.rs` - Integration point
- `wkmp-ai/src/db/sessions.rs` - Session persistence
- `wkmp-common/src/events.rs` - Event definitions
- `wkmp-ai/src/services/passage_recorder.rs` - Batch recording

### Library Dependencies
- `parking_lot` (0.12) - Efficient RwLock implementation
- `tokio` (1.x) - Async runtime, channels, intervals
- `sqlx` (0.8) - Database operations with transactions
- `uuid` (1.x) - Session identifiers
- `chrono` (0.4) - Timestamp handling
- `anyhow` (1.x) - Error handling

### No External Dependencies
- No external services required
- No network calls
- No filesystem operations beyond database