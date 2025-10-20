# WKMP Audio Player Test Coverage Assessment

**Date:** 2025-10-19
**Module:** wkmp-ap
**Analyst:** Agent 1D (Test Coverage Assessment)

---

## Executive Summary

**Total Tests:** 226 (173 unit + 53 integration + 6 benchmarks)
**Estimated Coverage:** 65% (industry typical: 60-80%)
**Test Infrastructure:** Strong (criterion benchmarks, real audio fixtures, RMS analysis)
**Overall Quality:** Good (solid core, gaps in infrastructure)

### Key Findings

✅ **Strengths:**
- Comprehensive crossfade testing with timing and quality validation
- Multi-format audio decoding with real test files (MP3/FLAC/AAC/Opus/Vorbis/WAV)
- Strong buffer management and mixer test coverage (34 + 32 tests)
- Performance benchmarks using criterion framework

❌ **Critical Gaps:**
- **No startup time benchmarks** - Cannot measure <100ms Phase 5 goal
- Limited error recovery testing (corrupt files, I/O failures)
- No memory leak detection tests (long-running playback)
- Minimal buffer underrun/overflow scenario testing

---

## Test Inventory

### Unit Tests by Module (173 total)

| Module | LOC | Tests | Est Coverage | Quality |
|--------|-----|-------|--------------|---------|
| **playback/buffer_manager.rs** | 1458 | 34 | 85% | ⭐⭐⭐⭐⭐ |
| **playback/pipeline/mixer.rs** | 1664 | 32 | 80% | ⭐⭐⭐⭐⭐ |
| **audio/types.rs** | 596 | 23 | 90% | ⭐⭐⭐⭐⭐ |
| **db/settings.rs** | 708 | 14 | 80% | ⭐⭐⭐⭐ |
| **playback/song_timeline.rs** | 453 | 9 | 80% | ⭐⭐⭐⭐ |
| **playback/engine.rs** | 2120 | 9 | 45% | ⭐⭐ |
| **playback/queue_manager.rs** | 443 | 8 | 75% | ⭐⭐⭐⭐ |
| **playback/pipeline/timing.rs** | 408 | 8 | 75% | ⭐⭐⭐⭐ |
| **playback/ring_buffer.rs** | 470 | 7 | 70% | ⭐⭐⭐⭐ |
| **db/passage_songs.rs** | 397 | 7 | 70% | ⭐⭐⭐ |
| **audio/resampler.rs** | 253 | 6 | 70% | ⭐⭐⭐⭐ |
| **db/passages.rs** | 424 | 5 | 60% | ⭐⭐⭐ |
| **audio/output.rs** | 618 | 4 | 35% | ⭐⭐ |
| **playback/events.rs** | 119 | 4 | 80% | ⭐⭐⭐⭐ |
| **db/queue.rs** | 459 | 4 | 55% | ⭐⭐⭐ |
| **state.rs** | 169 | 3 | 70% | ⭐⭐⭐ |
| **db/init.rs** | 263 | 3 | 60% | ⭐⭐⭐ |
| **playback/decoder_pool.rs** | 515 | 2 | 30% | ⭐ |
| **config.rs** | 193 | 1 | 40% | ⭐⭐ |
| **audio/decoder.rs** | 600 | 1 | 20% | ⭐ |

**Modules with Zero Tests:**
- `api/handlers.rs` (852 LOC) - Covered by integration tests
- `api/server.rs` (103 LOC) - Covered by integration tests
- `api/sse.rs` (80 LOC) - **NO COVERAGE**
- `main.rs` (149 LOC) - Entry point (typical)
- `error.rs` (74 LOC) - Implicit coverage

### Integration Tests (53 total)

| Test File | Tests | Coverage Area | Quality |
|-----------|-------|---------------|---------|
| **api_integration.rs** | 15 | HTTP API endpoints | ⭐⭐⭐⭐⭐ |
| **audio_format_tests.rs** | 14 | Multi-format decoding | ⭐⭐⭐⭐⭐ |
| **playback_engine_integration.rs** | 8 | Engine orchestration | ⭐⭐⭐ |
| **crossfade_integration_tests.rs** | 7 | Crossfade quality | ⭐⭐⭐⭐⭐ |
| **audio_subsystem_test.rs** | 6 | Component integration | ⭐⭐⭐ |
| **crossfade_test.rs** | 2 | Basic crossfades | ⭐⭐⭐ |
| **audible_crossfade_test.rs** | 1 | Human-in-loop | ⭐⭐⭐⭐⭐ |

### Benchmarks (6 benchmark groups)

**File:** `benches/crossfade_bench.rs` (criterion framework)

1. **Fade curves** - All 5 curve types (Linear, Exponential, Logarithmic, S-Curve, Equal-Power)
2. **Crossfade mixing** - Throughput at 1ms/10ms/100ms/1s buffers
3. **Timing precision** - Verifies 0.02ms requirement
4. **Buffer operations** - Allocation, copy, clear performance
5. **Parallel processing** - Decoder pool simulation
6. **Real-time constraints** - Audio callback latency

---

## Test Infrastructure

### Test Fixtures ✅

**Location:** `/home/sw/Dev/McRhythm/wkmp-ap/tests/fixtures/audio/`

- `test_audio_10s_wav.wav` (1.7MB) - Uncompressed reference
- `test_audio_10s_flac.flac` (130KB) - Lossless compression
- `test_audio_10s_mp3.mp3` (242KB) - Lossy compression
- `test_audio_10s_vorbis.ogg` (47KB) - Ogg Vorbis
- `test_audio_10s_opus.opus` (164KB) - Modern low-latency codec
- `test_audio_10s_aac.m4a` (136KB) - AAC in MP4 container

### Audio Analysis Tools ✅

- **RMS tracking** - `AudioLevelTracker` for level monitoring
- **Clipping detection** - Peak detection in crossfade tests
- **Timing verification** - Sample-accurate timing validation
- **Sine wave generation** - Synthetic test signals
- ❌ **Spectral analysis** - Not present (gap)
- ❌ **THD+N measurement** - Not present (gap)

### Performance Framework ✅

- **Criterion v0.5** - Industry-standard Rust benchmarking
- HTML reports enabled
- Throughput measurements
- Statistical analysis

### CI/CD Integration ❌

- No GitHub Actions workflow detected
- No automated coverage tracking
- No test execution on PR/commit

---

## Critical Testing Gaps

### 1. No Startup Time Benchmarks ⚠️ CRITICAL

**Impact:** Cannot measure progress toward Phase 5 goal (<100ms startup)

**Current State:** STARTUP_PERFORMANCE_ANALYSIS.md shows >3 second delay

**Missing Tests:**
- Cold start time (main() to first audio)
- Warm start time (subsequent playback)
- Component initialization timing breakdown
- Memory footprint at startup

**Recommendation:** Create `benches/startup_bench.rs`
```rust
// Measure time from main() to audio output ready
criterion_main!(startup_benches);
```

### 2. Minimal Decoder Error Recovery Tests ⚠️ HIGH

**Impact:** Production crashes on corrupt/invalid files

**Affected Modules:**
- `audio/decoder.rs` (1 test for 600 LOC)
- `playback/decoder_pool.rs` (2 tests for 515 LOC)

**Missing Tests:**
- Corrupt file handling
- Unsupported format detection
- Mid-decode I/O errors
- Seek precision on damaged files
- Out-of-memory during decode

**Recommendation:** Create `tests/decoder_error_handling.rs`

### 3. No Memory Leak Detection ⚠️ HIGH

**Impact:** Long-running playback may leak memory

**Missing Tests:**
- 1000+ passage playback cycle
- Buffer pool cleanup verification
- Arc reference counting validation
- Decoder worker thread cleanup

**Recommendation:** Create `tests/stress_test.rs`
```rust
#[tokio::test]
async fn test_long_running_playback_no_leak() {
    let baseline = current_memory_usage();
    for _ in 0..1000 {
        play_passage_full_cycle().await;
    }
    assert!(memory_growth < 5%);
}
```

### 4. No Buffer Underrun/Overflow Tests ⚠️ HIGH

**Impact:** Audio glitches under load

**Affected Modules:**
- `audio/output.rs` (4 tests, limited scenarios)
- `playback/ring_buffer.rs` (7 tests, no stress)

**Missing Tests:**
- Slow decode simulation
- Fast consumption simulation
- Buffer starvation recovery
- Real-time deadline violation

**Recommendation:** Add to `tests/audio_underrun_test.rs`

### 5. Limited Thread Safety Verification ⚠️ MEDIUM

**Impact:** Race conditions in concurrent scenarios

**Missing Tests:**
- Concurrent buffer access
- Decoder pool task queue overflow
- Lock-free data structure verification (use `loom`)

**Recommendation:** Add `loom` tests to `playback/ring_buffer.rs`

### 6. No SSE Lifecycle Tests ⚠️ MEDIUM

**Impact:** Real-time UI update failures

**Affected Modules:**
- `api/sse.rs` (0 tests)

**Missing Tests:**
- Client connect/disconnect
- Event ordering guarantees
- Backpressure handling
- Concurrent client scaling

**Recommendation:** Create `tests/sse_integration.rs`

### 7. Missing Audio Artifact Detection ⚠️ MEDIUM

**Impact:** Crossfade quality regressions undetected

**Current State:** RMS tracking exists, but no:
- Click/pop detection
- Phase cancellation detection
- THD+N (Total Harmonic Distortion + Noise)
- Spectral analysis

**Recommendation:** Add to `tests/audio_quality_analysis.rs`

### 8. No Configuration Validation Tests ⚠️ LOW

**Impact:** Startup errors from invalid TOML

**Affected Modules:**
- `config.rs` (1 test)

**Missing Tests:**
- Malformed TOML parsing
- Missing required fields
- Invalid value ranges

**Recommendation:** Expand `config.rs` test module

### 9. No Property-Based Tests ⚠️ LOW

**Impact:** Edge cases in mathematical operations

**Candidates:**
- Timing calculations (sample rate conversions)
- Resampling ratios
- Volume scaling (no clipping)

**Recommendation:** Add `proptest` to `Cargo.toml`

### 10. No API Security Tests ⚠️ MEDIUM

**Impact:** Multi-user security vulnerabilities

**Missing Tests:**
- Unauthorized access attempts
- CSRF protection
- Rate limiting
- Input validation

**Recommendation:** Add to `tests/api_security.rs`

---

## Test Quality Analysis

### What's Tested Well ✅

1. **Crossfade Logic** (32 unit + 7 integration + 1 audible test)
   - All 5 fade curve types
   - Sample-accurate timing
   - RMS level tracking
   - Timing verification within tolerance

2. **Buffer Management** (34 unit tests)
   - Lifecycle (decoding → ready → playing)
   - State transitions
   - Concurrent access
   - Progress tracking

3. **Audio Format Support** (14 integration tests)
   - MP3, FLAC, AAC, Opus, Vorbis, WAV
   - Real fixture files
   - Decode verification

4. **Data Types** (23 unit tests)
   - AudioFrame operations
   - PassageBuffer manipulation
   - Sample conversions

5. **Timing Calculations** (8 unit tests)
   - Sample-accurate precision
   - Crossfade point calculation
   - Edge cases (zero-duration, overflow)

### What's Tested Poorly ❌

1. **Engine Orchestration** (9 tests for 2120 LOC = 45% coverage)
   - Critical playback loop logic minimally tested
   - State machine transitions incomplete
   - Error recovery paths untested

2. **Decoder** (1 test for 600 LOC = 20% coverage)
   - Only mono-to-stereo conversion tested
   - No error handling tests
   - No format detection tests

3. **Decoder Pool** (2 tests for 515 LOC = 30% coverage)
   - Worker lifecycle untested
   - Concurrency scenarios minimal
   - Task queue overflow untested

4. **Audio Output** (4 tests for 618 LOC = 35% coverage)
   - Hardware dependency limits testing
   - Real-time performance untested
   - Error recovery minimal

5. **API Layer** (0 unit tests)
   - Handlers covered only by integration tests
   - SSE completely untested
   - Security not validated

### Test-to-Code Ratio

**226 tests / 13,719 LOC = 16.4 tests per 1000 lines**

**Comparison:**
- Rust typical: 20-30 tests/1000 LOC
- Assessment: Slightly below typical, acceptable for early development

---

## Recommendations

### Immediate Priority (Week 1)

1. **Create startup benchmark** (`benches/startup_bench.rs`)
   - Measure main() → audio ready
   - Target: <100ms per Phase 5 requirements
   - Break down by component (DB init, audio init, HTTP server)

2. **Add corrupt file tests** (`tests/decoder_error_handling.rs`)
   - Create corrupt MP3/FLAC fixtures
   - Verify graceful error handling
   - Test mid-decode failures

3. **Add long-running stress test** (`tests/stress_test.rs`)
   - 1000 passage playback cycle
   - Monitor memory growth
   - Verify cleanup

### High Priority (Week 2-3)

4. **Add buffer underrun tests** (`tests/audio_underrun_test.rs`)
   - Simulate slow decode
   - Test starvation recovery
   - Verify no audio glitches

5. **Add SSE lifecycle tests** (`tests/sse_integration.rs`)
   - Client connect/disconnect
   - Event ordering
   - Concurrent clients

6. **Expand decoder_pool tests** (in `src/playback/decoder_pool.rs`)
   - Worker lifecycle
   - Task queue overflow
   - Thread safety

### Medium Priority (Month 2)

7. **Add property-based tests** (`proptest`)
   - Timing calculations
   - Resampling ratios
   - Volume scaling

8. **Add audio quality analysis** (`tests/audio_quality_analysis.rs`)
   - Click/pop detection
   - Phase cancellation
   - THD+N measurement

9. **Set up CI/CD** (`.github/workflows/ci.yml`)
   - Run tests on PR
   - Coverage tracking (cargo-tarpaulin)
   - Performance regression detection

### Low Priority (Backlog)

10. **Add configuration tests** (expand `config.rs`)
11. **Add API security tests** (`tests/api_security.rs`)
12. **Add loom tests** (lock-free verification)

---

## Coverage Measurement

### Current Method

**Estimation based on:**
- Test count vs module size
- Presence of error handling tests
- Complexity analysis

**Estimated Overall Coverage:** 65%

### Recommended Tool

**cargo-tarpaulin** - Rust code coverage tool

```bash
cargo install cargo-tarpaulin
cargo tarpaulin --package wkmp-ap --out Html
```

**Benefits:**
- Line-by-line coverage
- Branch coverage
- HTML reports
- CI integration

**Action:** Add tarpaulin to CI pipeline

---

## Test Execution

### Current Test Count

```bash
cargo test --lib --tests -- --list
# Output: 368 tests (includes generated variants)
```

### Run All Tests

```bash
# Unit + integration
cargo test --package wkmp-ap

# Benchmarks
cargo bench --package wkmp-ap

# Ignored tests (audible test)
cargo test --package wkmp-ap -- --ignored
```

### Test Performance

**Current:** No CI timing data

**Recommendation:** Track test execution time in CI

---

## Comparison to WKMP Requirements

### Requirements Coverage

| Requirement | Coverage | Notes |
|-------------|----------|-------|
| **REQ-CF-010** Crossfade timing | ✅ Excellent | 7 integration tests, RMS validation |
| **REQ-CF-020** Fade curves | ✅ Excellent | All 5 curves tested, benchmarked |
| **REQ-PBUF-028** Instant play | ❌ Untested | No startup benchmark |
| **REQ-TECH-022A** Opus support | ✅ Good | Integration test with real file |
| **SSD-XFD-*** Crossfade specs | ✅ Excellent | Timing + quality validated |
| **SSD-PBUF-*** Buffer specs | ✅ Good | 34 unit tests |

---

## Conclusion

### Summary Assessment

**Overall:** Good foundation, critical infrastructure gaps

**Test Count:** 226 tests (solid)
**Coverage:** ~65% (acceptable for early stage)
**Quality:** High for core playback, low for infrastructure

### Must-Fix Before Production

1. ⚠️ **Startup benchmark** - Required for Phase 5 verification
2. ⚠️ **Error recovery tests** - Required for production reliability
3. ⚠️ **Memory leak detection** - Required for long-running stability

### Strengths to Maintain

- Comprehensive crossfade testing (industry-leading)
- Multi-format audio validation
- Performance benchmarking infrastructure
- Real test fixtures

### Path Forward

**Week 1:** Address 3 critical gaps (startup, errors, leaks)
**Week 2-3:** Add 3 high-priority tests (underrun, SSE, decoder_pool)
**Month 2:** Set up CI/CD + quality analysis
**Ongoing:** Maintain >70% coverage as codebase grows

---

## Appendix: Test Inventory Details

See companion file: `IMPL-ANALYSIS-004-test-coverage.json`

**Contains:**
- Complete module-by-module analysis
- All 226 test locations
- Detailed gap analysis
- Recommendations with effort estimates

---

**Report Generated:** 2025-10-19
**Agent:** 1D (Test Coverage Assessment)
**Status:** Complete
**JSON Data:** `/home/sw/Dev/McRhythm/docs/validation/wkmp-ap/IMPL-ANALYSIS-004-test-coverage.json`
