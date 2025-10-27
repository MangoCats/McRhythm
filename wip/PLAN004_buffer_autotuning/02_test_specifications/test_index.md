# Test Index - Buffer Auto-Tuning

**Total Tests:** 47 (23 unit, 12 integration, 5 system, 7 manual)

## Quick Reference

| Test ID | Type | Requirement | Description | Priority |
|---------|------|-------------|-------------|----------|
| TC-U-DET-010-01 | Unit | TUNE-DET-010 | Mock underrun detection and counting | High |
| TC-U-DET-010-02 | Unit | TUNE-DET-010 | Underrun severity classification (fail/warning/success) | High |
| TC-U-DET-020-01 | Unit | TUNE-DET-020 | Callback jitter calculation from timing samples | High |
| TC-U-DET-020-02 | Unit | TUNE-DET-020 | Buffer occupancy statistics (min/max/mean/percentile) | High |
| TC-U-DET-030-01 | Unit | TUNE-DET-030 | Threshold classification (>1% fail, 0.1-1% warn, <0.1% success) | High |
| TC-U-SRC-010-01 | Unit | TUNE-SRC-010 | Binary search logic with mocked test results | High |
| TC-U-SRC-010-02 | Unit | TUNE-SRC-010 | Parameter space coverage (intervals 1-100ms) | High |
| TC-U-SRC-020-01 | Unit | TUNE-SRC-020 | Test duration validation (30-60s enforced) | High |
| TC-U-SRC-030-01 | Unit | TUNE-SRC-030 | Adaptive search: start with defaults, converge to boundary | High |
| TC-U-SRC-040-01 | Unit | TUNE-SRC-040 | Minimum value enforcement (1ms interval, 64 frames buffer) | High |
| TC-U-SRC-040-02 | Unit | TUNE-SRC-040 | Settings restore on abort (Ctrl+C simulation) | High |
| TC-U-OUT-010-01 | Unit | TUNE-OUT-010 | Report generation with all required fields | High |
| TC-U-OUT-020-01 | Unit | TUNE-OUT-020 | Database update with user confirmation | High |
| TC-U-OUT-020-02 | Unit | TUNE-OUT-020 | Settings backup preservation | High |
| TC-U-OUT-030-01 | Unit | TUNE-OUT-030 | JSON export structure validation | High |
| TC-U-OUT-040-01 | Unit | TUNE-OUT-040 | JSON schema compliance | Medium |
| TC-U-ALG-010-01 | Unit | TUNE-ALG-010 | Phase 1: Coarse sweep identifies viable intervals | High |
| TC-U-ALG-010-02 | Unit | TUNE-ALG-010 | Phase 2: Fine tuning finds minimum buffer | High |
| TC-U-ALG-020-01 | Unit | TUNE-ALG-020 | Curve plotting (interval vs buffer size) | High |
| TC-U-ALG-020-02 | Unit | TUNE-ALG-020 | Knee identification in curve | High |
| TC-U-SEARCH-010-01 | Unit | TUNE-SEARCH-010 | Binary search convergence within 128 frames | High |
| TC-U-SEARCH-020-01 | Unit | TUNE-SEARCH-020 | Early termination: interval ≥50ms, buffer 64 fails | High |
| TC-U-SEARCH-020-02 | Unit | TUNE-SEARCH-020 | Early termination: 3 consecutive failures | High |
| TC-U-CURVE-010-01 | Unit | TUNE-CURVE-010 | Interval-buffer relationship analysis | High |
| TC-U-CURVE-020-01 | Unit | TUNE-CURVE-020 | Primary recommendation (256-512 frame target) | High |
| TC-U-CURVE-020-02 | Unit | TUNE-CURVE-020 | Fallback recommendation (no interval <512 frames) | High |
| TC-U-CURVE-020-03 | Unit | TUNE-CURVE-020 | Edge case: All intervals unstable | Medium |
| TC-U-SAFE-010-01 | Unit | TUNE-SAFE-010 | Settings backup to memory and temp file | High |
| TC-U-SAFE-010-02 | Unit | TUNE-SAFE-010 | Settings restore on panic | High |
| TC-U-SAFE-020-01 | Unit | TUNE-SAFE-020 | Timeout detection (60s per test) | High |
| TC-U-SAFE-030-01 | Unit | TUNE-SAFE-030 | Parameter range validation | Medium |
| TC-U-SAFE-030-02 | Unit | TUNE-SAFE-030 | Anomalous results detection | Medium |
| TC-U-TEST-040-01 | Unit | TUNE-TEST-040 | Test tone generation (48kHz→44.1kHz, f32 stereo) | High |
| TC-I-INT-010-01 | Integration | TUNE-INT-010 | Standalone execution (no HTTP server) | High |
| TC-I-INT-020-01 | Integration | TUNE-INT-020 | Reuse ring buffer, audio output, DB settings | High |
| TC-I-INT-030-01 | Integration | TUNE-INT-030 | Progress indication during execution | Medium |
| TC-I-UI-010-01 | Integration | TUNE-UI-010 | CLI argument parsing (--quick, --thorough, --apply, --export) | High |
| TC-I-UI-020-01 | Integration | TUNE-UI-020 | Interactive mode user prompts | Medium |
| TC-I-ARCH-010-01 | Integration | TUNE-ARCH-010 | Component structure (6 components present) | High |
| TC-I-ARCH-020-01 | Integration | TUNE-ARCH-020 | Existing component reuse verified | High |
| TC-I-ARCH-030-01 | Integration | TUNE-ARCH-030 | No HTTP/SSE/queue dependencies | High |
| TC-I-TEST-050-01 | Integration | TUNE-TEST-050 | Tuning run on CI system completes | High |
| TC-I-TEST-050-02 | Integration | TUNE-TEST-050 | Recommended values are sane (within ranges) | High |
| TC-I-TEST-050-03 | Integration | TUNE-TEST-050 | JSON output is valid | High |
| TC-S-OBJ-010-01 | System | TUNE-OBJ-010 | End-to-end: Determine safe values for both parameters | High |
| TC-S-OBJ-020-01 | System | TUNE-OBJ-020 | End-to-end: Characterize interval-buffer curve | High |
| TC-S-OBJ-030-01 | System | TUNE-OBJ-030 | End-to-end: Generate actionable recommendations | High |
| TC-S-OBJ-040-01 | System | TUNE-OBJ-040 | Hardware profile export and comparison | Medium |
| TC-S-OBJ-050-01 | System | TUNE-OBJ-050 | Complete tuning in <15 minutes (thorough mode) | Medium |
| TC-M-TEST-060-01 | Manual | TUNE-TEST-060 | Run on development hardware, apply values | High |
| TC-M-TEST-060-02 | Manual | TUNE-TEST-060 | Verify 1+ hour stable playback with recommended values | High |
| TC-M-SUCCESS-010-01 | Manual | TUNE-SUCCESS-010 | Functional requirements validation (manual checklist) | High |
| TC-M-SUCCESS-020-01 | Manual | TUNE-SUCCESS-020 | Quality requirements validation (<0.1% underruns) | High |
| TC-M-SUCCESS-020-02 | Manual | TUNE-SUCCESS-020 | Reproducibility test (3 runs, ±10% variation) | High |
| TC-M-SUCCESS-030-01 | Manual | TUNE-SUCCESS-030 | Usability validation (manual checklist) | Medium |
| TC-M-OBJ-050-02 | Manual | TUNE-OBJ-050 | Quick mode completes in <5 minutes | Medium |

## Test Categories

### Unit Tests (23 tests)
- Detection: 5 tests
- Search: 6 tests
- Output: 5 tests
- Algorithm: 7 tests
- Safety: 5 tests
- Test Infrastructure: 1 test

### Integration Tests (12 tests)
- Integration: 3 tests
- UI: 2 tests
- Architecture: 3 tests
- Test Infrastructure: 3 tests
- Component Interaction: 1 test

### System Tests (5 tests)
- End-to-end objectives: 5 tests

### Manual Tests (7 tests)
- Hardware validation: 2 tests
- Success criteria: 3 tests
- Timing validation: 2 tests

## Coverage by Requirement

| Requirement | Unit | Integration | System | Manual | Total |
|-------------|------|-------------|--------|--------|-------|
| TUNE-DET-010 | 2 | 0 | 0 | 0 | 2 |
| TUNE-DET-020 | 2 | 0 | 0 | 0 | 2 |
| TUNE-DET-030 | 1 | 0 | 0 | 0 | 1 |
| TUNE-SRC-010 | 2 | 0 | 0 | 0 | 2 |
| TUNE-SRC-020 | 1 | 0 | 0 | 0 | 1 |
| TUNE-SRC-030 | 1 | 0 | 0 | 0 | 1 |
| TUNE-SRC-040 | 2 | 0 | 0 | 0 | 2 |
| TUNE-OUT-010 | 1 | 0 | 0 | 0 | 1 |
| TUNE-OUT-020 | 2 | 0 | 0 | 0 | 2 |
| TUNE-OUT-030 | 1 | 0 | 0 | 0 | 1 |
| TUNE-OUT-040 | 1 | 0 | 0 | 0 | 1 |
| TUNE-INT-010 | 0 | 1 | 0 | 0 | 1 |
| TUNE-INT-020 | 0 | 1 | 0 | 0 | 1 |
| TUNE-INT-030 | 0 | 1 | 0 | 0 | 1 |
| TUNE-ALG-010 | 2 | 0 | 0 | 0 | 2 |
| TUNE-ALG-020 | 2 | 0 | 0 | 0 | 2 |
| TUNE-SEARCH-010 | 1 | 0 | 0 | 0 | 1 |
| TUNE-SEARCH-020 | 2 | 0 | 0 | 0 | 2 |
| TUNE-CURVE-010 | 1 | 0 | 0 | 0 | 1 |
| TUNE-CURVE-020 | 3 | 0 | 0 | 0 | 3 |
| TUNE-UI-010 | 0 | 1 | 0 | 0 | 1 |
| TUNE-UI-020 | 0 | 1 | 0 | 0 | 1 |
| TUNE-ARCH-010 | 0 | 1 | 0 | 0 | 1 |
| TUNE-ARCH-020 | 0 | 1 | 0 | 0 | 1 |
| TUNE-ARCH-030 | 0 | 1 | 0 | 0 | 1 |
| TUNE-SAFE-010 | 2 | 0 | 0 | 0 | 2 |
| TUNE-SAFE-020 | 1 | 0 | 0 | 0 | 1 |
| TUNE-SAFE-030 | 2 | 0 | 0 | 0 | 2 |
| TUNE-TEST-040 | 1 | 0 | 0 | 0 | 1 |
| TUNE-TEST-050 | 0 | 3 | 0 | 0 | 3 |
| TUNE-TEST-060 | 0 | 0 | 0 | 2 | 2 |
| TUNE-OBJ-010 | 0 | 0 | 1 | 0 | 1 |
| TUNE-OBJ-020 | 0 | 0 | 1 | 0 | 1 |
| TUNE-OBJ-030 | 0 | 0 | 1 | 0 | 1 |
| TUNE-OBJ-040 | 0 | 0 | 1 | 0 | 1 |
| TUNE-OBJ-050 | 0 | 0 | 1 | 1 | 2 |
| TUNE-SUCCESS-010 | 0 | 0 | 0 | 1 | 1 |
| TUNE-SUCCESS-020 | 0 | 0 | 0 | 2 | 2 |
| TUNE-SUCCESS-030 | 0 | 0 | 0 | 1 | 1 |

**Coverage Summary:**
- 31 requirements covered
- 47 total tests
- Average 1.5 tests per requirement
- 100% requirement coverage ✓

## Test Execution Order

### Phase 1: Unit Tests (Fast, ~10 minutes)
Run first to catch logic errors early:
1. Detection tests (TC-U-DET-*)
2. Search algorithm tests (TC-U-SRC-*, TC-U-SEARCH-*)
3. Curve fitting tests (TC-U-CURVE-*)
4. Output generation tests (TC-U-OUT-*)
5. Safety tests (TC-U-SAFE-*)
6. Algorithm tests (TC-U-ALG-*)
7. Test infrastructure (TC-U-TEST-040-01)

### Phase 2: Integration Tests (Medium, ~15 minutes)
Run after unit tests pass:
1. Component integration (TC-I-INT-*, TC-I-ARCH-*)
2. CLI interface (TC-I-UI-*)
3. CI system execution (TC-I-TEST-050-*)

### Phase 3: System Tests (Slow, ~60 minutes)
Run after integration tests pass:
1. End-to-end objectives (TC-S-OBJ-*)

### Phase 4: Manual Tests (Very Slow, ~2-3 hours)
Run before release:
1. Hardware validation (TC-M-TEST-060-*)
2. Success criteria (TC-M-SUCCESS-*)
3. Timing validation (TC-M-OBJ-050-02)

## Test Data Requirements

### Mocked Data for Unit Tests
- Underrun count samples (0-1000 range)
- Timing jitter samples (0-50ms range)
- Buffer occupancy samples (0-8192 range)
- Test configuration results (stable/unstable)

### Real Data for Integration Tests
- Test audio passage (from database) or generated tone
- Working audio device
- SQLite database with settings table

### Hardware for System/Manual Tests
- Development system (Linux, adequate CPU)
- Optional: Raspberry Pi Zero 2W for cross-platform validation

## Expected Test Duration

| Phase | Tests | Duration | Can Parallelize? |
|-------|-------|----------|------------------|
| Unit | 23 | ~10 min | Yes (cargo test) |
| Integration | 12 | ~15 min | Partial |
| System | 5 | ~60 min | No (sequential) |
| Manual | 7 | ~2-3 hours | No |
| **Total** | **47** | **~4 hours** | - |

**CI Duration:** ~25 minutes (unit + integration only)
**Full Validation:** ~4 hours (all tests including manual)
