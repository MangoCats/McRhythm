# wkmp-ap Technical Debt Report

**Generated:** 2025-10-29
**Module:** wkmp-ap (Audio Player)
**Total Source Files:** 54
**Total Lines of Code:** 26,507

---

## Executive Summary

wkmp-ap has **moderate technical debt** concentrated in several key areas:

- **13 explicit TODOs** requiring implementation
- **376 .unwrap() calls** (potential panic points)
- **21 compiler warnings** (unused code, imports)
- **2 duplicate config files** (config.rs + config_new.rs)
- **2 backup files** (.backup, .backup2) committed to repo
- **1 CRITICAL SECURITY ISSUE**: POST/PUT authentication bypass
- **3573-line engine.rs** (largest file, needs decomposition)

**Priority Level:** MEDIUM-HIGH (security issue elevates from MEDIUM)

---

## 🔴 CRITICAL ISSUES (Must Fix)

### 1. **POST/PUT Authentication Bypass**
**File:** `wkmp-ap/src/api/auth_middleware.rs:832`
**Severity:** CRITICAL - Security vulnerability
**Impact:** All POST/PUT endpoints bypass authentication

```rust
Method::POST | Method::PUT => {
    // TODO: Implement proper POST/PUT authentication
    tracing::warn!("POST/PUT request bypassing authentication - not yet implemented");
    return Ok(Authenticated);  // ⚠️ ALLOWS ALL REQUESTS THROUGH
}
```

**Risk:** Any client can modify playback state, queue, settings without authentication.

**Recommendation:** Implement header-based auth or move to request body validation immediately.

---

## 🟡 HIGH-PRIORITY TODOs

### 2. **Missing File Path in Error Messages**
**Files:** `wkmp-ap/src/audio/decode.rs:161, 176`
**Impact:** Debugging decoder errors requires guessing which file failed

```rust
file_path: PathBuf::from("unknown"),  // TODO: Store file path in decoder
```

**Recommendation:** Add `file_path: PathBuf` field to decoder struct, populate on initialization.

---

### 3. **Buffer Configuration Not Reading from Database**
**File:** `wkmp-ap/src/playback/buffer_manager.rs:122`
**Impact:** Buffer tuning parameters ignored, hardcoded defaults used

```rust
// TODO: Read capacity and headroom from settings database
let buffer_arc = Arc::new(PlayoutRingBuffer::new(
    None, // Use default capacity (661,941)
    None, // Use default headroom (4410)
    Some(hysteresis),
    Some(queue_entry_id),
));
```

**Recommendation:** Query `settings` table for `buffer_capacity_samples` and `buffer_headroom_samples`.

---

### 4. **Incomplete Buffer Chain Diagnostics**
**File:** `wkmp-ap/src/playback/engine.rs:1203-1228`
**Impact:** Developer UI missing decoder state, resample progress, fade stage info

```rust
decoder_state: None, // TODO: Query decoder pool status
source_sample_rate: None, // TODO: Get from decoder
fade_stage: None, // TODO: Get from decoder
started_at: None, // TODO: Track start time
```

**Recommendation:** Add telemetry hooks in decoder_worker.rs to populate these fields.

---

### 5. **Missing Song Album UUIDs**
**Files:** `wkmp-ap/src/playback/engine.rs:1840, 2396, 2687`
**Impact:** Passage metadata incomplete in events (3 locations)

```rust
song_albums: Vec::new(), // TODO: Fetch album UUIDs from database
```

**Recommendation:** Add database query joining `passages` → `passage_albums` → `albums`.

---

### 6. **Duration Played Calculation Stubbed**
**Files:** `wkmp-ap/src/playback/engine.rs:2018, 2103`
**Impact:** PassageComplete events report duration_played as 0.0 seconds

```rust
duration_played: 0.0, // TODO: Calculate actual duration from passage timing
```

**Recommendation:** Track playback start time, calculate elapsed on completion.

---

## 🟢 MEDIUM-PRIORITY TODOs (Future Enhancements)

### 7. **Phase 5 Features Not Implemented**
**File:** `wkmp-ap/src/playback/pipeline/mixer.rs:893-960`
**Features:**
- Seeking support (set_position)
- Drain-based buffer management updates
- Ring buffer underrun detection improvements

```rust
// TODO Phase 5+: Implement reset_to_position() on PlayoutRingBuffer for seeking support
// TODO Phase 5: Update for drain-based buffer management
// TODO Phase 5: Update for ring buffer underrun detection
```

**Status:** Future roadmap items, not blocking current functionality.

---

### 8. **Developer UI Secret Embedding**
**File:** `wkmp-ap/src/api/handlers.rs:1302`
**Status:** TODO comment outdated - secret embedding already implemented in `server.rs:77`

```rust
// TODO: This currently serves static HTML. Need to implement dynamic shared_secret embedding.
```

**Action:** Remove outdated TODO comment (feature already complete).

---

### 9. **Clipping Warning Logging Missing**
**File:** `wkmp-ap/src/playback/pipeline/mixer.rs:534`
**Impact:** Silent audio clipping without diagnostic log

```rust
// TODO: Add clipping warning log
```

**Recommendation:** Add `warn!("Audio clipping detected at frame {}", frame_pos)` when samples exceed ±1.0.

---

## 🟠 CODE QUALITY ISSUES

### 10. **Excessive .unwrap() Usage**
**Count:** 376 instances
**Risk:** Potential panics in production

**High-Risk Locations:**
- `wkmp-ap/src/audio/buffer.rs`: Mutex locks (`lock().unwrap()` - poisoned mutex panic)
- Event receiving (`rx.recv().await.unwrap()` - channel panic)

**Recommendation:** Replace critical unwraps with proper error handling:
```rust
// Before
let guard = mutex.lock().unwrap();

// After
let guard = mutex.lock().map_err(|e| Error::MutexPoisoned(e.to_string()))?;
```

**Priority:** Focus on hot paths (audio thread, decoder workers) first.

---

### 11. **Large Monolithic Files**
**Top 5 by line count:**

| File | Lines | Recommendation |
|------|-------|----------------|
| playback/engine.rs | 3,573 | Split into engine_core, engine_queue, engine_diagnostics |
| playback/pipeline/mixer.rs | 1,933 | Extract crossfade logic, buffer management |
| api/handlers.rs | 1,305 | Group by feature (queue, settings, diagnostics) |
| audio/decoder.rs | 1,083 | Separate symphonia integration, error handling |
| playback/buffer_manager.rs | 1,068 | Extract lifecycle management, diagnostics |

**Recommendation:** Refactor engine.rs as top priority (14% of entire codebase).

---

### 12. **Compiler Warnings**
**Count:** 21 warnings
**Categories:**

**Unused Imports (11):**
- `api/mod.rs`: `Uri`, `self`
- `main.rs`: `AssertUnwindSafe`, `catch_unwind`, `AtomicU32`
- `playback/mod.rs`: Multiple pipeline imports

**Dead Code (8):**
- `developer_ui()`, `auth_middleware_fn()` - Used indirectly via routing
- `panic_payload_to_string()` - Error handling utility
- `PassageBuffer`, `DecodeResult` - Legacy types

**Unused Variables (2):**
- `buffer`, `crossfade_start_ms`, `next_buffer`, `min_buffer`

**Recommendation:** Run `cargo fix --lib -p wkmp-ap` to auto-fix 4 issues, manually review remaining.

---

### 13. **Duplicate Configuration Files**
**Files:**
- `wkmp-ap/src/config.rs` (6,994 bytes, Oct 26 13:44)
- `wkmp-ap/src/config_new.rs` (14,804 bytes, Oct 26 20:11)

**Issue:** Unclear which is canonical, potential for divergence.

**Recommendation:**
1. Determine which config system is active (check `main.rs`)
2. Remove obsolete file
3. Rename `config_new.rs` → `config.rs` if new is active

---

### 14. **Backup Files Committed to Repo**
**Files:**
- `wkmp-ap/src/events.rs.backup`
- `wkmp-ap/src/events.rs.backup2`

**Issue:** Version control serves as backup; these clutter repo.

**Recommendation:** Delete backup files, rely on git history.

---

## 📊 Metrics Summary

| Metric | Count | Assessment |
|--------|-------|------------|
| Total TODOs | 13 | Moderate |
| Critical TODOs | 1 | **HIGH RISK** |
| .unwrap() calls | 376 | High (needs audit) |
| Compiler warnings | 21 | Moderate |
| Files >1000 lines | 5 | Needs refactoring |
| Backup files | 2 | Cleanup required |
| Unsafe blocks | 0 | ✅ Excellent |
| Explicit panics (tests only) | 7 | ✅ Acceptable |

---

## 🎯 Recommended Action Plan

### Phase 1: Security & Critical (Sprint 1)
1. **FIX CRITICAL**: Implement POST/PUT authentication (`auth_middleware.rs:832`)
2. Add file_path to decoder error messages (`decode.rs:161, 176`)
3. Implement buffer config database reads (`buffer_manager.rs:122`)

### Phase 2: Quality & Diagnostics (Sprint 2)
4. Add buffer chain telemetry for developer UI (`engine.rs:1203-1228`)
5. Implement song_albums database queries (3 locations in `engine.rs`)
6. Calculate duration_played for PassageComplete events
7. Remove backup files (`events.rs.backup*`)
8. Resolve config.rs vs config_new.rs

### Phase 3: Code Health (Sprint 3)
9. Audit and fix high-risk .unwrap() calls (audio thread, decoders)
10. Fix all compiler warnings via `cargo fix` + manual cleanup
11. Refactor engine.rs (split into 3-4 modules)
12. Add clipping warning log (`mixer.rs:534`)

### Phase 4: Future Enhancements (Backlog)
13. Phase 5 features (seeking, improved underrun detection)

---

## 🔍 Audit Details

**Methodology:**
- Searched for TODO, FIXME, XXX, HACK markers
- Analyzed .unwrap() usage patterns
- Compiled warnings from `cargo build`
- Identified large files (>1000 lines)
- Checked for backup files and duplicates
- Reviewed unsafe blocks and explicit panics

**Files Not Requiring Action:**
- Test panics are acceptable (7 test-only panics verified)
- No unsafe blocks found (excellent memory safety)
- Temp directory usage in tests is appropriate

---

## Summary

wkmp-ap is **production-ready with caveats**:

✅ **Strengths:**
- No unsafe code
- Good test coverage (panics isolated to tests)
- Clean architecture (modular design)

⚠️ **Weaknesses:**
- Authentication bypass is a **showstopper** for multi-user deployments
- High .unwrap() count needs audit (especially audio thread)
- Large files reduce maintainability

**Overall Assessment:** Address the authentication bypass immediately, then systematically work through the 3-phase plan above.
