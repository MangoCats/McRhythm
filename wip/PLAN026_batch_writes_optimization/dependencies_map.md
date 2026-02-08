# PLAN026: Batch Writes Optimization - Dependencies Map

**Plan:** PLAN026 - Batch Writes Optimization
**Created:** 2025-01-15

---

## Overview

This document catalogs all dependencies for the batch writes optimization project:
- Existing code to preserve or modify
- External libraries required
- Configuration settings used
- Documentation references

---

## Existing Code Dependencies

### Core Utilities (PRESERVE - Do Not Modify)

**1. Retry Logic** ✅
- **File:** `wkmp-ai/src/utils/db_retry.rs`
- **Function:** `retry_on_lock()`
- **Lines:** 31-121
- **Purpose:** Exponential backoff retry for "database is locked" errors
- **Status:** Exists, proven effective
- **Action:** Preserve and continue using

**2. Transaction Monitoring** ✅
- **File:** `wkmp-ai/src/utils/pool_monitor.rs`
- **Function:** `begin_monitored()`
- **Purpose:** Track transaction execution time and connection usage
- **Status:** Exists
- **Action:** Use for batch write transactions

### Reference Implementation (STUDY - Replicate Pattern)

**3. Passage Recorder** 📖
- **File:** `wkmp-ai/src/services/passage_recorder.rs`
- **Lines:** 84-150 (record_passages method)
- **Pattern:** Pre-fetch reads (lines 103-118) + batch writes in transaction (lines 130-145)
- **Key Insight:** Transaction hold time <100ms (line 80 comment)
- **Status:** Proven effective pattern
- **Action:** Replicate this pattern in other write-heavy paths

### Database Helpers (MAY MODIFY)

**4. File Operations** 🔧
- **File:** `wkmp-ai/src/db/files.rs`
- **Functions:**
  - `save_file()` (line 76)
  - `save_files_batch()` (exists)
  - `update_file_duration()` (exists)
- **Status:** May need batching wrappers
- **Action:** Review for batch optimization opportunities

**5. Passage Operations** 🔧
- **File:** `wkmp-ai/src/db/passages.rs`
- **Functions:**
  - `save_passage()`
  - `update_passage_timing()`
  - `update_passage_flavor()`
  - `update_passage_metadata()`
- **Status:** May consolidate into batch operations
- **Action:** Review usage patterns, add batch variants if needed

**6. Song/Artist/Album Operations** 🔧
- **Files:**
  - `wkmp-ai/src/db/songs.rs`
  - `wkmp-ai/src/db/artists.rs`
  - `wkmp-ai/src/db/albums.rs`
- **Status:** Current save_*() functions may need batch variants
- **Action:** Add batch insert functions

**7. Work Operations** 🔧
- **File:** `wkmp-ai/src/db/works.rs`
- **Functions:** `save_work()`, `link_song_to_work()`
- **Status:** Low volume, may not need optimization
- **Action:** Review, optimize if part of hot path

### Service Layer (MODIFY - Batch Write Candidates)

**8. Workflow Orchestrator** 🎯
- **File:** `wkmp-ai/src/services/workflow_orchestrator/mod.rs`
- **Lines:** 3,134 total
- **Write Calls:** 40+ helper function calls
- **Status:** PRIMARY optimization target
- **Action:** Consolidate writes into batched transactions per phase

**9. Phase Modules** 🎯
- **Files:**
  - `phase_scanning.rs` - Minimal writes
  - `phase_extraction.rs` - File metadata writes
  - `phase_fingerprinting.rs` - Song/artist/album writes
  - `phase_segmenting.rs` - Passage writes
  - `phase_analyzing.rs` - Passage metadata updates
  - `phase_flavoring.rs` - Flavor vector updates
- **Status:** Each phase has write operations
- **Action:** Batch writes within each phase

**10. Hash Deduplicator** 🔧
- **File:** `wkmp-ai/src/services/hash_deduplicator.rs`
- **Writes:** 6+ direct UPDATE statements
- **Status:** Scattered file status updates
- **Action:** Consolidate into batch update function

**11. Workflow Storage** 🔧
- **File:** `wkmp-ai/src/workflow/storage.rs`
- **Writes:** 8+ direct INSERT statements
- **Status:** Batch passage storage
- **Action:** Already batched, verify efficiency

---

## External Library Dependencies

### Database Access

**sqlx** 📦
- **Version:** (current in Cargo.toml)
- **Purpose:** Async SQLite database access
- **Features Used:**
  - Connection pooling (SqlitePool)
  - Transactions (pool.begin().await)
  - Query macros (sqlx::query!)
- **Status:** Core dependency, no changes needed
- **Documentation:** https://docs.rs/sqlx

### Async Runtime

**tokio** 📦
- **Version:** (current in Cargo.toml)
- **Purpose:** Async/await runtime
- **Features Used:**
  - spawn_blocking for CPU-bound work
  - task::unconstrained for preventing starvation
  - Async functions and futures
- **Status:** Core dependency, no changes needed
- **Documentation:** https://docs.rs/tokio

### Concurrency Primitives

**parking_lot** 📦
- **Version:** (current in Cargo.toml)
- **Purpose:** RwLock for worker activity tracking
- **Usage:** Statistics module (workflow_orchestrator/statistics.rs)
- **Status:** Existing dependency, no changes needed

### Other Dependencies

**chrono**, **uuid**, **serde**, **tracing** 📦
- **Purpose:** Time handling, IDs, serialization, logging
- **Status:** Used throughout, no changes needed

---

## Configuration Dependencies

### Database Settings (settings table)

**1. ai_database_max_lock_wait_ms** ⚙️
- **Type:** Integer (milliseconds)
- **Default:** 5000 (5 seconds)
- **Purpose:** Maximum retry time for database lock errors
- **Used By:** retry_on_lock() in db_retry.rs
- **Action:** Preserve, continue using

**2. ai_database_lock_retry_ms** ⚙️
- **Type:** Integer (milliseconds)
- **Default:** 250
- **Purpose:** Initial backoff delay for lock retries
- **Status:** May not be actively used (retry_on_lock uses hardcoded 10ms)
- **Action:** Review for consistency

**3. ingest_max_concurrent_jobs** ⚙️
- **Type:** Integer (worker count)
- **Default:** 12
- **Purpose:** Maximum concurrent file processing workers
- **Impact:** More workers = more lock contention
- **Action:** Unchanged, but batching reduces impact

**4. ai_longwork_yield_interval_ms** ⚙️
- **Type:** Integer (milliseconds)
- **Default:** 990
- **Purpose:** CPU task yield interval to prevent starvation
- **Impact:** Affects task scheduling, not database access
- **Action:** Unchanged

### Global Parameters (GlobalParams)

**5. working_sample_rate** ⚙️
- **Type:** Integer (Hz)
- **Default:** 44100
- **Impact:** Audio processing, not database
- **Action:** No changes needed

**6. Database path resolution** ⚙️
- **Source:** Root folder resolver (4-tier priority)
- **Impact:** Database location
- **Action:** Unchanged

---

## Documentation References

### WKMP Architecture

**1. IMPL001: Database Schema** 📄
- **Location:** `docs/IMPL001-database_schema.md`
- **Relevance:** SQLite WAL mode, transaction semantics
- **Key Sections:**
  - WAL mode configuration
  - Transaction isolation
  - Foreign key constraints
- **Action:** Reference for transaction correctness

**2. SPEC001: Architecture** 📄
- **Location:** `docs/SPEC001-architecture.md`
- **Relevance:** Microservices communication, async patterns
- **Action:** Ensure batch writes don't break architecture

**3. IMPL002: Coding Conventions** 📄
- **Location:** `docs/IMPL002-coding_conventions.md`
- **Relevance:** Rust style, error handling patterns
- **Action:** Follow conventions in batch write code

### Conversation Analysis (This Session)

**4. Feasibility Analysis** 💬
- **Source:** This conversation
- **Key Findings:**
  - Read/write ratio: ~50/50 overall, ~70/30 in hot path
  - 102 total write operations across 27 files
  - Dedicated writer task rejected (MEDIUM-HIGH risk)
  - Batch writes recommended (LOW risk, proven pattern)
- **Action:** Use conversation analysis to guide implementation

**5. Complexity Analysis** 💬
- **Source:** This conversation
- **Key Findings:**
  - Dedicated writer: 3-5 days, MEDIUM-HIGH risk
  - Batch writes: 1-2 days, LOW risk
  - passage_recorder.rs demonstrates pattern
- **Action:** Follow simpler, lower-risk approach

---

## Dependency Status Matrix

| Component | Type | Status | Action | Risk |
|-----------|------|--------|--------|------|
| retry_on_lock | Utility | Exists | Preserve | Low |
| begin_monitored | Utility | Exists | Use | Low |
| passage_recorder | Reference | Exists | Study pattern | Low |
| db/*.rs helpers | Database | Exists | Add batch variants | Low-Medium |
| Orchestrator | Service | Exists | Refactor writes | Medium |
| Phase modules | Service | Exists | Batch writes | Medium |
| sqlx | External | Stable | No change | Low |
| tokio | External | Stable | No change | Low |
| Settings | Config | Exists | Preserve | Low |

---

## Missing Dependencies

**None identified.**

All required infrastructure exists:
- ✅ Retry logic in place
- ✅ Transaction monitoring available
- ✅ Reference pattern demonstrated
- ✅ Database helpers exist
- ✅ Configuration settings defined

**No new dependencies required.**

---

## Dependency Risks

### Low Risk

1. **retry_on_lock compatibility:** Batch writes should work with existing retry logic
   - Mitigation: Test retry behavior with batched transactions

2. **Transaction monitoring overhead:** begin_monitored adds minimal overhead
   - Mitigation: Already in use, proven acceptable

### Medium Risk

1. **Database helper API changes:** Adding batch variants may require caller updates
   - Mitigation: Add new functions, keep old ones for compatibility
   - Mitigation: Update callers incrementally

2. **Orchestrator refactoring scope:** 40+ write calls across 3,134 lines
   - Mitigation: Work incrementally, one phase at a time
   - Mitigation: Comprehensive test coverage

### High Risk

**None identified.**

---

## Sign-Off

**Dependencies Cataloged:** 2025-01-15
**Status:** Complete
**Missing Dependencies:** None
**High Risks:** None identified

**Ready for Phase 2:** Specification Completeness Verification
