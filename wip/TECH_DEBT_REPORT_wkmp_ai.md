# Technical Debt Report: wkmp-ai Microservice

**Date:** 2025-11-16
**Codebase Size:** 40,894 lines across 118 Rust files
**Severity:** HIGH - Multiple critical issues affecting reliability and performance
**Estimated Remediation:** 4-6 weeks of focused engineering effort

---

## Executive Summary

The wkmp-ai microservice contains significant technical debt that impacts reliability, performance, and maintainability. Most critical are 30+ `unwrap()` calls in production code that will panic at runtime, a 3,382-line monolithic orchestrator file (2x larger than documented), and unbounded memory growth from never-cleaned cancellation tokens.

**Immediate risks:**
- Runtime panics from unwrap() calls causing import failures
- Memory leaks from unbounded HashMap growth
- Performance degradation from synchronous I/O in async context
- Lost events due to hardcoded 100-event capacity limit

---

## Critical Issues (Fix Immediately)

### 1. Runtime Panic Risk - 30+ unwrap() Calls
**Severity:** CRITICAL
**Files:** Throughout services/ and models/
**Impact:** Will crash at runtime with minimal error context

**Examples:**
```rust
// wkmp-ai/src/services/fingerprinter.rs:202
let _lock = CHROMAPRINT_LOCK.lock().unwrap();  // Will panic if poisoned

// wkmp-ai/src/models/bootstrap_config.rs:281-375
// 29 unwrap/expect calls in production code
```

**Fix:** Replace with proper error handling using `?` operator

---

### 2. Memory Leak - Unbounded Cancellation Tokens
**Severity:** HIGH
**File:** lib.rs:41
**Impact:** Memory grows indefinitely during long import sessions

```rust
pub cancellation_tokens: Arc<RwLock<HashMap<Uuid, CancellationToken>>>,
// Tokens added but never removed
```

**Fix:** Add cleanup on import completion

---

### 3. AppState Constructor Mismatch
**Severity:** CRITICAL (Compilation failure)
**Files:** lib.rs:66-70 vs main.rs:208-212
**Impact:** Code won't compile - missing `memory_usage_threshold_bytes` parameter

**Fix:** Pass missing parameter in main.rs

---

## Architecture Issues

### 1. Monolithic Orchestrator - 3,382 Lines
**File:** workflow_orchestrator/mod.rs
**Impact:** Unmaintainable, hard to test, difficult to understand

The file acknowledges this at line 20-22:
```rust
// Future Refactoring
// This 1,459-line file could be split into separate modules per state
```
But it's actually **3,382 lines** - more than 2x documented size.

**Recommendation:** Split by phase into <500 line modules

---

### 2. Duplicate Implementations
Multiple versions of same functionality:
- **3 ID3 extractors** across different modules
- **3 MusicBrainz clients** with overlapping code
- **2 AcoustID clients** with similar logic
- **2 amplitude analyzers** with different approaches

**Impact:** Code rot, inconsistent behavior, maintenance burden

---

### 3. Synchronous I/O in Async Context
**Files:** Multiple service files
**Impact:** Blocks async executor threads, causes latency spikes

```rust
// main.rs:126 - Blocks async startup
std::fs::read_to_string(&toml_path)

// services/file_scanner.rs:283-288
File::open(path)  // Should use tokio::fs
```

---

## Performance Issues

### 1. N+1 Query Patterns
**Files:** db/passages.rs, db/files.rs
**Impact:** Slow imports with many passages

```rust
// db/passages.rs:83-105
for passage in passages {
    sqlx::query(...).execute(pool).await?;  // Individual query per passage
}
```

---

### 2. Excessive Cloning
**Count:** 279 clone operations across 36 files
**Impact:** Memory overhead, unnecessary allocations

Each service derives `Clone` and clones entire Arc structures including database pools.

---

### 3. Fixed Event Bus Capacity
**File:** main.rs:204
**Impact:** Events dropped silently when >100 pending

```rust
EventBus::new(100)  // Hardcoded capacity
```

---

## Configuration Debt

### Hardcoded Values Throughout
- Port: `127.0.0.1:5723` (main.rs:241)
- Event capacity: 100 (main.rs:204)
- Timeouts: 5000ms, 2000ms, 1000ms (db_retry.rs)
- Sample rate: 44100 (boundary_detector.rs:153,300)
- Pagination: 1000 max (file_classification.rs:161)

**Impact:** Cannot tune without code changes

---

## Test Coverage Issues

- **391 tests** covering **40,894 lines** = ~104 lines per test
- Tests use `.unwrap()` extensively (not production-quality)
- No integration tests across API boundaries
- Missing error path coverage

---

## Priority Action Plan

### Week 1 - Critical Fixes (8-12 hours)
1. [ ] Replace all unwrap() with proper error handling
2. [ ] Fix AppState constructor parameter mismatch
3. [ ] Add cancellation token cleanup
4. [ ] Increase event bus capacity to configurable value

### Week 2 - Performance (16-24 hours)
1. [ ] Convert synchronous file I/O to async
2. [ ] Fix N+1 queries with batch operations
3. [ ] Replace sync locks with async equivalents
4. [ ] Extract configuration to settings table

### Week 3-4 - Architecture (40-60 hours)
1. [ ] Split 3,382-line orchestrator into phase modules
2. [ ] Unify duplicate extractors with trait system
3. [ ] Add abstraction layer for external dependencies
4. [ ] Implement comprehensive integration tests

### Week 5-6 - Polish (20-30 hours)
1. [ ] Update documentation to match actual code
2. [ ] Standardize naming conventions
3. [ ] Add monitoring and metrics
4. [ ] Performance profiling and optimization

---

## Metrics for Success

**Before:**
- 30+ unwrap() calls in production
- 3,382-line monolithic file
- 0% integration test coverage
- Unbounded memory growth

**After:**
- 0 unwrap() in production code
- No file >500 lines
- 80% test coverage
- Bounded memory with cleanup

---

## Risk Assessment

| Risk | Probability | Impact | Mitigation |
|------|------------|--------|------------|
| Production panic from unwrap() | **HIGH** | System crash | Replace immediately |
| Memory exhaustion | **MEDIUM** | OOM kill | Add token cleanup |
| Import performance degradation | **HIGH** | User frustration | Fix N+1 queries |
| Code unmaintainable | **CERTAIN** | Development velocity drops | Refactor orchestrator |

---

## Recommendation

The wkmp-ai codebase requires immediate attention to prevent production failures. The combination of unwrap() calls, memory leaks, and performance issues creates a high risk of system instability during large imports.

**Prioritize:**
1. **Week 1 fixes** to stabilize system
2. **Performance improvements** to handle production loads
3. **Architecture refactoring** for long-term maintainability

The estimated 4-6 weeks of effort will transform this from a prototype-quality codebase to production-ready software.