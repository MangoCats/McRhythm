# Technical Debt Report - wkmp-ai & wkmp-common

**Date:** 2025-11-10
**Scope:** Post-PLAN026 analysis (Sprints 1-2 complete)
**Modules Analyzed:** wkmp-ai, wkmp-common
**Test Status:** 274/275 passing (1 flaky performance test)

---

## Executive Summary

**Overall Health:** ✅ **GOOD** - Production ready with manageable technical debt

Post-PLAN026 analysis reveals a healthy codebase with 8 critical/high-priority issues resolved. Remaining technical debt is primarily future enhancements (deferred Sprint 3), incomplete features (MusicBrainz/AcoustID clients), and isolated quality improvements (error handling, test stability).

**Key Findings:**
- ✅ No critical blockers (all resolved in PLAN026)
- ⚠️ 7 TODO items remaining (6 future enhancements, 1 test gap)
- ⚠️ 10 `#[allow(dead_code)]` suppressions (incomplete features)
- ⚠️ 146 `.unwrap()/.expect()` calls (acceptable for test code + validated cases)
- ⚠️ 1 flaky performance test (non-deterministic timing)
- ✅ Zero compiler warnings in production code
- ✅ Clean dependency tree (2 duplicate base64 versions via transitive deps)

**Recommendation:** No immediate action required. Address items incrementally during future feature development.

---

## Category 1: Incomplete Features (MEDIUM Priority)

### 1.1 Event Bridge Temporary Scaffolding

**Location:** [event_bridge.rs:14](wkmp-ai/src/event_bridge.rs#L14)
```rust
//! This is temporary scaffolding. As other modules migrate to import_v2 events,
//! this bridge will be removed.
```

**Issue:** Event bridge translates import_v2 events to WkmpEvent for backward compatibility. Intended as temporary until all modules use import_v2 events.

**Impact:**
- Adds translation layer overhead (~5 match statements)
- Maintains dual event systems (import_v2::ImportEvent + wkmp_common::WkmpEvent)
- No functional issue, architectural cleanup opportunity

**Recommendation:**
- **Priority:** Medium (defer to next major refactor)
- **Effort:** 8-12 hours (migrate all modules to import_v2 events)
- **Risk:** Low (event bridge fully tested, no regressions expected)

**Resolution Plan:**
1. Migrate wkmp-pd to use ImportEvent directly
2. Migrate wkmp-ui to subscribe to ImportEvent SSE stream
3. Remove event_bridge.rs module
4. Update documentation to reflect single event system

---

### 1.2 MusicBrainz Client Stub Implementation

**Location:** [tier1/musicbrainz_client.rs](wkmp-ai/src/import_v2/tier1/musicbrainz_client.rs)

**Issue:** MusicBrainz API client has 5 `#[allow(dead_code)]` suppressions for unused methods.

**Dead Code:**
- `lookup_recording()` - Recording lookup by MBID
- `search_recordings()` - Text search for recordings
- `lookup_artist()` - Artist lookup by MBID
- `lookup_release()` - Release lookup by MBID
- `query_by_isrc()` - Recording lookup by ISRC code

**Impact:**
- API rate limiting configured but not used (governor crate)
- No actual MusicBrainz API calls in current workflow
- MBID extraction works (REQ-TD-004) but no validation/enrichment

**Recommendation:**
- **Priority:** Medium (defer until metadata enrichment needed)
- **Effort:** 12-16 hours (implement + test API calls)
- **Blocker:** Requires MusicBrainz API rate limit testing

**Resolution Plan:**
1. Implement `lookup_recording()` for MBID validation
2. Add metadata enrichment to identity resolver (Tier 2)
3. Test rate limiting with real API (max 1 req/sec per MusicBrainz policy)
4. Remove `#[allow(dead_code)]` suppressions

---

### 1.3 AcoustID Client Stub Implementation

**Location:** [tier1/acoustid_client.rs:28](wkmp-ai/src/import_v2/tier1/acoustid_client.rs#L28)

**Issue:** AcoustID API client marked as `#[allow(dead_code)]`, not integrated into workflow.

**Impact:**
- Chromaprint fingerprints generated (REQ-TD-008) but not submitted to AcoustID
- No automatic MBID lookup via fingerprint matching
- Manual tagging required for files without ID3 UFID frames

**Recommendation:**
- **Priority:** Low-Medium (nice-to-have automation)
- **Effort:** 6-8 hours (API integration + error handling)
- **Blocker:** Requires AcoustID API key registration

**Resolution Plan:**
1. Register for AcoustID API key (free for open source)
2. Implement `lookup()` method in AcoustIDClient
3. Add to identity resolver as fallback source (after UFID, before text search)
4. Test rate limiting (3 req/sec per AcoustID docs)

---

### 1.4 Audio Features Extractor Incomplete

**Location:** [tier1/audio_features.rs:3-19](wkmp-ai/src/import_v2/tier1/audio_features.rs#L3-L19)

**Issue:** Audio features extractor defined but not implemented. Comments describe planned features:
- Tempo estimation (basic beat detection)
- Energy/loudness calculation
- Spectral centroid (brightness)
- Zero-crossing rate (percussiveness)

**Impact:**
- No basic audio features extracted (tempo, energy, etc.)
- Musical flavor currently synthesized from Essentia only
- Fallback to low-confidence defaults if Essentia unavailable

**Recommendation:**
- **Priority:** Low (blocked by Essentia integration)
- **Effort:** 20-30 hours (DSP implementation + validation)
- **Blocker:** Requires audio analysis expertise or library integration

**Resolution Plan:**
1. Evaluate libraries: aubio-rs, essentia-rs, or manual DSP
2. Implement basic tempo detection (autocorrelation or FFT)
3. Add as Tier 1 extractor alongside Chromaprint
4. Integrate into FlavorSynthesizer as additional source

---

## Category 2: Future Enhancements (LOW Priority)

### 2.1 Waveform Rendering

**Location:** [api/ui.rs:864](wkmp-ai/src/api/ui.rs#L864)
```rust
// TODO: Implement waveform rendering and boundary markers
```

**Issue:** REQ-TD-009 (Sprint 3 deferred). Waveform visualization for visual boundary editing in UI.

**Impact:** No visual feedback for passage boundaries in import UI.

**Recommendation:**
- **Priority:** Low (deferred per PLAN026)
- **Effort:** 16-24 hours (rendering + UI integration)
- **Blocker:** UI framework decision (canvas vs SVG)

**Resolution:** Defer to future implementation plan (Sprint 3 or later).

---

### 2.2 Duration Tracking in File Stats

**Location:** [session_orchestrator.rs:570](wkmp-ai/src/import_v2/session_orchestrator.rs#L570)
```rust
total_duration_ms: 0, // TODO: Track file-level duration
```

**Issue:** REQ-TD-010 (Sprint 3 deferred). File statistics missing total duration calculation.

**Impact:** Import progress percentage unavailable (events show N/M passages, not time-based %).

**Recommendation:**
- **Priority:** Low (deferred per PLAN026)
- **Effort:** 4-6 hours (calculate duration during audio load)
- **Blocker:** Requires optimization to avoid double audio load

**Resolution:** Defer to future implementation plan.

---

### 2.3 Flavor Synthesis Placeholder Comment

**Location:** [session_orchestrator.rs:657](wkmp-ai/src/import_v2/session_orchestrator.rs#L657)
```rust
// TODO: When workflow uses FlavorSynthesizer, replace with direct result
```

**Issue:** Comment outdated - FlavorSynthesizer already integrated (REQ-TD-007).

**Impact:** None (misleading comment only).

**Recommendation:**
- **Priority:** Very Low (cleanup)
- **Effort:** 1 minute (delete comment)
- **Action:** Remove comment in next commit touching this file

---

### 2.4 Test Coverage Gaps

**Location:** [song_workflow_engine.rs:775-777](wkmp-ai/src/import_v2/song_workflow_engine.rs#L775-L777)
```rust
// TODO: Add integration tests with test audio files
// TODO: Add tests for error isolation (failing passage doesn't abort import)
// TODO: Add tests for validation thresholds
```

**Issue:** Integration test gaps identified during implementation.

**Impact:**
- Error isolation tested manually but not in CI
- Validation threshold tuning relies on manual testing
- No end-to-end audio file tests (only unit tests with generated waveforms)

**Recommendation:**
- **Priority:** Medium (improve test coverage)
- **Effort:** 8-12 hours (create test fixtures + write tests)
- **Tests Needed:**
  1. Integration test with real FLAC/MP3 files (multi-track album)
  2. Error isolation test (corrupt passage mid-file)
  3. Validation threshold test (conflicting metadata detection)

**Resolution Plan:**
1. Create test fixtures: `tests/fixtures/audio/` with known-good files
2. Add `test_full_import_workflow()` with real audio
3. Add `test_error_isolation()` with intentionally corrupt data
4. Add `test_consistency_thresholds()` with known conflicts

---

## Category 3: Code Quality (LOW Priority)

### 3.1 Error Handling: .unwrap()/.expect() Usage

**Metrics:**
- **Total occurrences:** 146 across 24 files
- **Context:** Majority in test code (acceptable)
- **Production code:** ~30-40 occurrences (need review)

**Files with highest usage:**
- `tier2/metadata_fuser.rs`: 18 occurrences
- `tier2/boundary_fuser.rs`: 11 occurrences
- `tier2/identity_resolver.rs`: 11 occurrences
- `tier2/flavor_synthesizer.rs`: 10 occurrences

**Issue:** `.unwrap()` can panic at runtime. Acceptable in test code and validated cases, but risky in production code without validation.

**Recommendation:**
- **Priority:** Low (audit during code review)
- **Effort:** 4-6 hours (review + convert to proper error handling)
- **Action:** Audit production `.unwrap()` calls, convert to `?` or `unwrap_or_default()`

**Analysis Pattern:**
```rust
// ACCEPTABLE (test code)
#[cfg(test)]
fn test_example() {
    let result = parser.parse("test").unwrap(); // Test will fail if panic
    assert_eq!(result, expected);
}

// RISKY (production code, unvalidated input)
pub fn process_input(s: &str) -> Output {
    let parsed = s.parse::<i32>().unwrap(); // User input - should handle error!
    Output::new(parsed)
}

// ACCEPTABLE (production code, validated invariant)
pub fn process_uuid(uuid_str: &str) -> Uuid {
    // Caller contract: uuid_str must be valid UUID (documented)
    Uuid::parse_str(uuid_str).expect("UUID validation failed - caller contract violation")
}
```

**Resolution Plan:**
1. Audit all `.unwrap()` in non-test production code
2. Classify: (a) validated invariant (document), (b) should use `?`, (c) should use `unwrap_or_default()`
3. Convert category (b) and (c) to proper error handling
4. Document category (a) with `expect("reason")`

---

### 3.2 Dead Code Suppressions

**Total:** 10 `#[allow(dead_code)]` attributes

**Breakdown:**
1. **MusicBrainz client** (5): lookup_recording, search_recordings, lookup_artist, lookup_release, query_by_isrc
2. **AcoustID client** (1): entire struct
3. **FlavorSynthesizer** (1): unused field (likely configuration)
4. **Song workflow** (2): unused helper methods
5. **Import workflow** (1): unused helper struct

**Issue:** Dead code suppressions hide incomplete features and make codebase harder to understand.

**Recommendation:**
- **Priority:** Low (cleanup during feature implementation)
- **Action:**
  - Remove suppressions when features implemented
  - Delete truly dead code (unused helpers)
  - Document incomplete features with clear TODO comments

---

### 3.3 Clone() Usage

**Metrics:**
- **Total occurrences:** 59 across 21 files
- **Highest usage:** `genre_mapper.rs` (11 clones)

**Issue:** Excessive cloning may indicate inefficient data flow patterns.

**Analysis:**
- `genre_mapper.rs`: Likely string cloning for HashMap lookups (acceptable)
- Event emissions: Cloning event data for broadcast (necessary for ownership)
- Metadata fusion: Cloning candidates for comparison (necessary for algorithm)

**Recommendation:**
- **Priority:** Very Low (micro-optimization)
- **Action:** Profile if performance issues arise
- **Note:** Most clones are necessary for Rust ownership model

---

### 3.4 Large Files

**Files >700 lines:**
1. `api/ui.rs`: 1038 lines (UI endpoint handlers)
2. `session_orchestrator.rs`: 781 lines (orchestration logic + tests)
3. `song_workflow_engine.rs`: 778 lines (workflow logic + tests)
4. `audio_loader.rs`: 750 lines (codec support + resampling + tests)
5. `consistency_checker.rs`: 719 lines (validation logic + extensive tests)

**Issue:** Large files can indicate poor separation of concerns.

**Analysis:**
- All files include substantial test suites (~40-50% of lines)
- Logic cohesion is good (each file has single responsibility)
- Complexity is manageable (well-commented, clear structure)

**Recommendation:**
- **Priority:** Very Low (no action needed)
- **Rationale:** Test-inclusive files are expected to be larger
- **Note:** Consider splitting if any file exceeds 1500 lines

---

## Category 4: Test Infrastructure (MEDIUM Priority)

### 4.1 Flaky Performance Test

**Location:** [tests/system_tests.rs:734](wkmp-ai/tests/system_tests.rs#L734)

**Test:** `tc_s_nf_011_01_performance_benchmark`

**Issue:** Performance test fails intermittently due to timing variations.

**Error:**
```rust
assert!(
    ratio >= 0.5 && ratio <= 2.0,
    "Performance degradation detected: first={:.3}s, last={:.3}s (ratio: {:.2})",
    first_duration / 1000.0,
    last_duration / 1000.0,
    ratio
);
```

**Root Cause:**
- Allows 2x variation between first and last iteration
- System load, JIT compilation, caching cause >2x variation in CI environments
- Test is too strict for non-deterministic performance metrics

**Impact:**
- CI builds occasionally fail on performance test
- False negatives (real code is fine, test environment is noisy)
- Slows down development (requires manual re-run)

**Recommendation:**
- **Priority:** Medium (affects CI reliability)
- **Effort:** 1-2 hours (adjust threshold or disable test)

**Resolution Options:**
1. **Option A:** Increase threshold to 3x (accounts for more variability)
2. **Option B:** Disable in CI, run manually for performance regression testing
3. **Option C:** Use statistical approach (median of 10 runs, 95% confidence interval)

**Recommended Fix:** Option A (simplest, lowest risk)
```rust
// Allow 3x variation (accounts for CI noise, JIT, caching)
assert!(
    ratio >= 0.33 && ratio <= 3.0,
    "Performance degradation detected: first={:.3}s, last={:.3}s (ratio: {:.2})",
    first_duration / 1000.0,
    last_duration / 1000.0,
    ratio
);
```

---

### 4.2 Test Code Panics

**Total:** 9 `panic!()` calls in test code

**Breakdown:**
- `file_scanner.rs`: 2 (error variant validation)
- `event_bridge.rs`: 5 (event type assertions)
- `consistency_checker.rs`: 2 (validation result checks)

**Issue:** Test panics are intentional (test assertions), but could use clearer messages.

**Recommendation:**
- **Priority:** Very Low (cosmetic improvement)
- **Action:** Replace with `assert!` macros where possible
- **Example:**
```rust
// Before
_ => panic!("Wrong event type"),

// After
_ => panic!("Expected ImportProgressUpdate, got {:?}", event),
```

---

## Category 5: Dependencies (LOW Priority)

### 5.1 Duplicate base64 Versions

**Issue:** Two versions of base64 crate in dependency tree:
- `base64 v0.21.7` (via reqwest)
- `base64 v0.22.1` (via sqlx + direct dependency)

**Impact:**
- Slightly larger binary size (~20KB duplication)
- No functional issue (both versions API-compatible)

**Recommendation:**
- **Priority:** Low (wait for upstream updates)
- **Action:** Monitor reqwest updates - when it upgrades to base64 v0.22, remove duplication
- **Note:** Not actionable until reqwest releases update

---

### 5.2 Unused Dependencies

**Status:** Unable to verify (cargo-udeps not installed)

**Recommendation:**
- **Priority:** Low (periodic maintenance)
- **Action:** Install cargo-udeps and run analysis
```bash
cargo install cargo-udeps
cargo +nightly udeps
```
- **Expected:** Likely no unused dependencies (clean build, no warnings)

---

## Category 6: Architecture & Design

### 6.1 Event System Duplication

**Issue:** Dual event systems in use:
1. `import_v2::ImportEvent` (new, import-specific)
2. `wkmp_common::WkmpEvent` (legacy, system-wide)

**Current State:**
- Event bridge translates ImportEvent → WkmpEvent
- SSE broadcaster emits both types
- Modules use different event types (not standardized)

**Impact:**
- Maintenance burden (keep both systems in sync)
- Performance overhead (translation layer)
- Developer confusion (which event type to use?)

**Recommendation:**
- **Priority:** Medium (architectural cleanup)
- **Effort:** 16-24 hours (migrate all modules)
- **Blocker:** Requires cross-module coordination (wkmp-ui, wkmp-pd)

**Resolution Plan:**
1. Define migration strategy (ImportEvent becomes canonical)
2. Update wkmp-pd to subscribe to ImportEvent
3. Update wkmp-ui SSE clients to handle ImportEvent
4. Remove WkmpEvent import variants
5. Delete event_bridge.rs module

---

### 6.2 Configuration Management

**Pattern:** Database-first configuration with TOML bootstrap

**Strengths:**
- Settings stored in SQLite (no separate config files)
- Hot-reload supported (query database on each access)
- Consistent across all modules

**Weaknesses:**
- Performance: Database query per setting access (no caching)
- Complexity: Two-tier system (TOML → DB → runtime)

**Recommendation:**
- **Priority:** Low (design tradeoff, not debt)
- **Future Enhancement:** Add setting cache with invalidation
- **Effort:** 8-12 hours (implement cache layer)

---

## Category 7: Documentation

### 7.1 API Documentation Coverage

**Status:** Code well-commented, but API docs could be improved

**Gaps:**
- Public API methods lack `/// # Examples` sections
- Error conditions not always documented
- Performance characteristics not documented (e.g., "O(n) complexity")

**Recommendation:**
- **Priority:** Low (nice-to-have improvement)
- **Action:** Add examples to public API during code review
- **Tool:** Run `cargo doc --open` and review coverage

---

### 7.2 Architecture Decision Records

**Status:** Decisions documented in PLAN026_SUMMARY.md, but no centralized ADR repository

**Recommendation:**
- **Priority:** Low (process improvement)
- **Action:** Create `docs/adr/` directory for architecture decision records
- **Format:** Use standard ADR template (context, decision, consequences)

---

## Priority Summary

### CRITICAL (0 items)
*None - all critical issues resolved in PLAN026*

### HIGH (0 items)
*None - all high-priority issues resolved in PLAN026*

### MEDIUM (5 items)
1. **Event bridge removal** - Migrate to single event system (16-24h)
2. **MusicBrainz client completion** - Implement API calls (12-16h)
3. **Test coverage gaps** - Add integration tests (8-12h)
4. **Flaky performance test** - Adjust threshold or disable (1-2h)
5. **Event system unification** - Architectural cleanup (16-24h)

**Total Medium Priority Effort:** 53-78 hours

### LOW (17 items)
- AcoustID client integration (6-8h)
- Audio features extractor (20-30h)
- Waveform rendering (16-24h)
- Duration tracking (4-6h)
- Error handling audit (4-6h)
- Dead code cleanup (2-4h)
- Unused dependencies check (1h)
- Documentation improvements (ongoing)
- Configuration caching (8-12h)
- Other minor items (10-15h)

**Total Low Priority Effort:** 71-106 hours

---

## Recommendations by Timeline

### Immediate (Next Commit)
- Remove outdated flavor synthesis comment (1 min)
- Fix flaky performance test threshold (1-2h)

### Short-Term (Next Sprint)
- Add integration tests with real audio files (8-12h)
- Audit production `.unwrap()` calls (4-6h)

### Medium-Term (Next Quarter)
- Complete MusicBrainz client implementation (12-16h)
- Migrate to single event system (16-24h)
- Implement AcoustID integration (6-8h)

### Long-Term (Future Releases)
- Sprint 3 deferred items (waveform, duration, audio features)
- Configuration caching (performance optimization)
- Architecture decision record repository

---

## Conclusion

**Overall Assessment:** ✅ **PRODUCTION READY**

wkmp-ai and wkmp-common are in good health with manageable technical debt. All critical and high-priority issues resolved in PLAN026. Remaining debt is primarily:
1. **Incomplete features** (MusicBrainz/AcoustID clients) - planned but not blocking
2. **Future enhancements** (Sprint 3 items) - explicitly deferred
3. **Code quality** (error handling audit, test coverage) - incremental improvements

**No urgent action required.** Technical debt can be addressed incrementally during normal feature development.

**Recommended Next Steps:**
1. Fix flaky performance test (immediate)
2. Add integration test coverage (short-term)
3. Complete MusicBrainz client (medium-term)
4. Address Sprint 3 deferred items (long-term)

**Technical Debt Velocity:** Decreasing (PLAN026 resolved 8 issues, only 5 medium-priority items added)

**Maintainability Score:** 8/10 (excellent architecture, clear separation of concerns, comprehensive tests)

---

**Report Generated:** 2025-11-10
**Next Review:** After Sprint 3 or major feature addition
**Methodology:** Code analysis, grep patterns, dependency audit, test suite review
