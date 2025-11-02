# PLAN013: Chromaprint Fingerprinting Implementation

**Specification:** docs/IMPL012-acoustid_client.md (v1.1 - CORRECTED)
**Status:** ALL PHASES COMPLETE - **READY FOR IMPLEMENTATION**
**Date:** 2025-10-30

---

## Problem Statement

Audio fingerprinting is **completely unimplemented** in wkmp-ai. Current code contains placeholder implementations returning dummy data, causing 100% AcoustID lookup failures.

**Current State:**
- `generate_chromaprint()` returns `vec![0x01, 0x02, 0x03, 0x04, 0xAB, 0xCD, 0xEF]`
- `decode_audio()` returns dummy PCM data
- All files get same invalid fingerprint → AcoustID correctly rejects with error code 3

**Required Implementation:**
- Chromaprint fingerprinting pipeline (decode → resample → fingerprint)
- AcoustID API caching layer (reduce API calls by 60%)
- Database schema updates (acoustid_cache table)
- Specification corrections (current spec has errors)

---

## All Phases Summary

### ✅ Phase 1: Scope Definition (COMPLETE)
- Extracted 18 distinct requirements from IMPL012 specification
- Defined IN/OUT scope boundaries
- Cataloged dependencies (all present in Cargo.toml)
- Identified 5 specification issues requiring investigation

### ✅ Phase 2: Specification Verification (COMPLETE)
- Investigated all 5 issues and resolved
- Discovered specification vs implementation divergence
- Verified chromaprint-sys-next API (unsafe FFI bindings)
- Confirmed database schema pattern (wkmp-common/src/db/init.rs)

### ✅ Specification Update (COMPLETE)
- Updated IMPL012 to v1.1 with 5 critical corrections:
  - Corrected Chromaprint integration code (unsafe FFI)
  - Added database schema section (acoustid_cache table)
  - Corrected API key configuration (database-first)
  - Corrected rate limiting documentation
  - Updated testing strategy (manual integration tests)

### ✅ Phase 3: Acceptance Tests (COMPLETE)
- Defined 24 acceptance tests for 18 requirements (100% coverage)
- Created traceability matrix
- Organized tests by component (fingerprinter.rs, acoustid_client.rs)
- Defined test-first workflow

### ✅ Phase 4: Implementation Plan (COMPLETE)
- Broke implementation into 5 increments
- Estimated 4-6 hours total (920 LOC)
- Defined intermediate working states
- Created quality gates and rollback strategy

### 📊 Current Implementation Coverage

| Component | Status | Notes |
|---|---|---|
| **AcoustIDClient** | ⚠️ 70% Complete | HTTP/rate limiting ✅, caching ❌ |
| **Fingerprinter** | ❌ 5% Complete | Placeholders only |
| **Database Schema** | ❌ Missing | acoustid_cache table not created |
| **Specification** | ⚠️ Contains Errors | 3 critical corrections needed |

**Requirements Coverage:** 7/18 complete (39%)

---

## Implementation Overview

**5 Increments (Test-First Approach):**

1. **Database Schema & Caching** (30-45 min) - Setup caching layer
2. **Audio Decoding** (45-60 min) - Symphonia integration
3. **Audio Resampling** (30-45 min) - Rubato integration
4. **Chromaprint FFI** (60-90 min) - Unsafe FFI bindings (highest risk)
5. **Integration & Tests** (45-60 min) - Complete workflow validation

**Total Estimated Effort:** 4-6 hours, 920 LOC (330 implementation + 470 tests + 120 docs)

---

## Critical Findings (Resolved)

### ✅ ISSUE 1: Specification Incorrect Code (RESOLVED)

**Problem:** IMPL012 showed non-existent high-level Rust API

**Resolution:** Updated specification v1.1 with correct chromaprint-sys-next unsafe FFI code

**Impact:** Implementation can now proceed with accurate specification

---

### ✅ ISSUE 2: Caching Layer Not Implemented (PLANNED)

**Problem:** Specification calls for caching, current code has none

**Resolution:** Increment 1 will implement full caching layer:
- Database schema creation
- SHA-256 fingerprint hashing
- Cache lookup/storage functions
- Integration with AcoustID lookup

**Impact:** Will achieve 60% API call reduction on re-import (per spec)

---

### ✅ ISSUE 3: Database Table Missing (PLANNED)

**Problem:** acoustid_cache table not created

**Resolution:** Increment 1 adds `create_acoustid_cache_table()` to wkmp-common/src/db/init.rs

**Schema Defined:** Documented in IMPL012 v1.1 lines 291-338

---

## Implementation Requirements

### What Needs to Be Built

**1. Fingerprinting Pipeline (fingerprinter.rs) - ~300 LOC**
- `decode_audio()` - Symphonia integration (decode to PCM)
- `resample_to_44100()` - Rubato integration (resample to 44.1kHz mono)
- `generate_chromaprint()` - chromaprint-sys-next FFI (unsafe code)
- `convert_to_mono_f32()` - Channel mixing helper
- Error handling for all stages

**2. Caching Layer (acoustid_client.rs) - ~100 LOC**
- Add `db: SqlitePool` field
- `hash_fingerprint()` - SHA-256 hashing
- `get_cached_mbid()` - Database lookup
- `cache_mbid()` - UPSERT to database
- Update `lookup()` to check cache first

**3. Database Schema (wkmp-common/src/db/init.rs) - ~30 LOC**
- `create_acoustid_cache_table()` function
- Add call to `init_database()`

**4. Specification Updates (docs/IMPL012-acoustid_client.md) - ~200 LOC**
- Replace Chromaprint integration section with correct unsafe FFI code
- Add database schema section
- Correct API key configuration section
- Correct rate limiting statement
- Update testing strategy section

**Total Estimated Effort:** 630 LOC + testing

---

## Technical Challenges

### 🔥 Challenge 1: Unsafe FFI Code

**chromaprint-sys-next API:**
- Requires unsafe blocks for all calls
- Must manually manage memory (chromaprint_free, chromaprint_dealloc)
- Return value checking critical (0 = failure, 1 = success)
- Null pointer checks required

**Mitigation:**
- Wrap FFI in safe Rust interface
- Use RAII pattern (impl Drop) for automatic cleanup
- Extensive error handling
- Follow chromaprint-sys-next examples precisely

**Risk:** Medium → Low-Medium (with careful implementation)

---

### Challenge 2: Audio Pipeline Complexity

**Three-Stage Pipeline:**
1. Symphonia decode (any format → PCM)
2. Rubato resample (any rate → 44.1kHz mono)
3. Chromaprint fingerprint (PCM → Base64 string)

**Failure Modes:**
- Unsupported codecs (Symphonia)
- Sample rate conversion errors (Rubato)
- FFI failures (Chromaprint)
- I/O errors (file access)

**Mitigation:** Graceful error handling at each stage, log failures, continue processing other files

**Risk:** Medium → Low (with proper error handling)

---

## Specification Corrections Needed

**Before proceeding to implementation, IMPL012 must be updated:**

| Section | Lines | Priority | Issue |
|---|---|---|---|
| Chromaprint Integration | 206-231 | 🔴 CRITICAL | Non-existent API shown, needs unsafe FFI code |
| Database Schema | N/A (missing) | 🟡 HIGH | Add new section with acoustid_cache table definition |
| API Key Configuration | 615-621 | 🟢 MEDIUM | Says env var, should say database-first |
| Rate Limiting | 606-609 | 🟢 MEDIUM | Says "no limiter needed", limiter exists and IS needed |
| Testing Strategy | 510-595 | 🟢 MEDIUM | Update with mock strategy (no fixtures) |

---

## Next Steps: Begin Implementation

**All planning complete. Ready to implement.**

**Start with Increment 1: Database Schema & Caching**

```bash
# Step 1: Create database table
# Edit: wkmp-common/src/db/init.rs
# Add: create_acoustid_cache_table() function

# Step 2: Write caching tests (RED phase)
# Edit: wkmp-ai/src/services/acoustid_client.rs
# Add: test_cache_hit(), test_cache_miss(), etc.

# Step 3: Implement caching layer (GREEN phase)
# Add: hash_fingerprint(), get_cached_mbid(), cache_mbid()

# Step 4: Verify tests pass
cargo test acoustid_client

# Step 5: Commit increment
/commit  # Or: git add . && git commit -m "..."
```

**Detailed Instructions:** See `04_phase4_implementation_plan.md` for complete workflow

---

## Files Created

**Planning Documents:**
- `wip/PLAN013_chromaprint_fingerprinting/00_PLAN_SUMMARY.md` (this file)
- `wip/PLAN013_chromaprint_fingerprinting/01_phase1_scope_definition.md` (326 lines)
- `wip/PLAN013_chromaprint_fingerprinting/02_phase2_specification_verification.md` (510 lines)
- `wip/PLAN013_chromaprint_fingerprinting/03_phase3_acceptance_tests.md` (386 lines)
- `wip/PLAN013_chromaprint_fingerprinting/04_phase4_implementation_plan.md` (512 lines)

**Specification Updates:**
- `docs/IMPL012-acoustid_client.md` (updated to v1.1 with 5 critical corrections)

**Total Documentation:** ~2,000 lines across 6 files

**All Phases:** ✅ COMPLETE - Ready for implementation
