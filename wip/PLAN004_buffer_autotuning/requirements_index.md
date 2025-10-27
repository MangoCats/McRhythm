# Requirements Index - Buffer Auto-Tuning

**Source:** wip/_buffer_autotuning.md
**Total Requirements:** 31 (5 objectives + 26 requirements)

## Objectives

| ID | Type | Description | Line | Priority |
|----|------|-------------|------|----------|
| TUNE-OBJ-010 | Objective | Automatically determine safe operating values for mixer_check_interval_ms and audio_buffer_size | 30-32 | High |
| TUNE-OBJ-020 | Objective | Characterize parameter relationship curve, identify optimal balance | 34-37 | High |
| TUNE-OBJ-030 | Objective | Produce actionable results with recommendations and confidence levels | 39-42 | High |
| TUNE-OBJ-040 | Objective | Support multiple hardware profiles with comparison capability | 46-49 | Medium |
| TUNE-OBJ-050 | Objective | Minimize tuning time (<15 minutes, adaptive search, early termination) | 51-54 | Medium |

## Functional Requirements

### Detection (3 requirements)

| ID | Type | Description | Line | Priority |
|----|------|-------------|------|----------|
| TUNE-DET-010 | Functional | Detect buffer underruns, count per duration, classify severity | 62-65 | High |
| TUNE-DET-020 | Functional | Measure audio health (callback jitter, buffer occupancy, CPU usage) | 67-70 | High |
| TUNE-DET-030 | Functional | Define failure thresholds (>1% fail, 0.1-1% warning, <0.1% success) | 72-75 | High |

### Search (4 requirements)

| ID | Type | Description | Line | Priority |
|----|------|-------------|------|----------|
| TUNE-SRC-010 | Functional | Explore parameter space systematically with binary search | 79-82 | High |
| TUNE-SRC-020 | Functional | Test each configuration adequately (30s min, actual audio) | 84-87 | High |
| TUNE-SRC-030 | Functional | Adaptive search strategy (start with defaults, converge to boundary) | 89-93 | High |
| TUNE-SRC-040 | Functional | Safety constraints (minimum values, responsiveness detection, restore on abort) | 95-98 | High |

### Output (4 requirements)

| ID | Type | Description | Line | Priority |
|----|------|-------------|------|----------|
| TUNE-OUT-010 | Functional | Generate tuning report (results, recommendations, curve, system info) | 102-106 | High |
| TUNE-OUT-020 | Functional | Update database settings with user confirmation and backup | 108-111 | High |
| TUNE-OUT-030 | Functional | Export results in JSON format for comparison | 113-116 | High |
| TUNE-OUT-040 | Detailed | JSON export structure specification | 285-338 | Medium |

### Integration (3 requirements)

| ID | Type | Description | Line | Priority |
|----|------|-------------|------|----------|
| TUNE-INT-010 | Functional | Operate as standalone utility (separate binary/subcommand) | 120-123 | High |
| TUNE-INT-020 | Functional | Use existing infrastructure (DB, audio output, ring buffer) | 125-128 | High |
| TUNE-INT-030 | Functional | Provide user feedback (progress, real-time results, ETA) | 130-133 | Medium |

## Algorithm Requirements

### Overall Strategy (2 requirements)

| ID | Type | Description | Line | Priority |
|----|------|-------------|------|----------|
| TUNE-ALG-010 | Algorithm | Two-phase approach (coarse sweep, then fine tuning) | 141-151 | High |
| TUNE-ALG-020 | Algorithm | Result synthesis (plot curve, identify knee, recommend balance) | 153-156 | High |

### Test Procedure (3 requirements)

| ID | Type | Description | Line | Priority |
|----|------|-------------|------|----------|
| TUNE-TEST-010 | Procedure | Per-configuration test procedure (7 steps) | 160-169 | High |
| TUNE-TEST-020 | Procedure | Test audio selection (actual passage or test tone, full pipeline) | 171-175 | High |
| TUNE-TEST-030 | Procedure | Metrics collection (underruns, jitter, occupancy, CPU) | 177-181 | High |

### Search Algorithm (2 requirements)

| ID | Type | Description | Line | Priority |
|----|------|-------------|------|----------|
| TUNE-SEARCH-010 | Algorithm | Binary search for minimum buffer size (convergence <128 frames) | 185-205 | High |
| TUNE-SEARCH-020 | Algorithm | Early termination conditions (3 scenarios) | 207-210 | High |

### Curve Fitting (2 requirements)

| ID | Type | Description | Line | Priority |
|----|------|-------------|------|----------|
| TUNE-CURVE-010 | Algorithm | Plot and analyze interval vs. buffer size relationship | 214-217 | High |
| TUNE-CURVE-020 | Algorithm | Recommendation logic (primary 256-512 frames, fallback) | 219-229 | High |

## User Interface Requirements

| ID | Type | Description | Line | Priority |
|----|------|-------------|------|----------|
| TUNE-UI-010 | Interface | Command-line interface with options (quick, thorough, apply, export, compare) | 237-247 | High |
| TUNE-UI-020 | Interface | Interactive mode with progress display and user prompts | 249-281 | Medium |

## Architecture Requirements

| ID | Type | Description | Line | Priority |
|----|------|-------------|------|----------|
| TUNE-ARCH-010 | Architecture | Standalone utility structure (6 components) | 346-355 | High |
| TUNE-ARCH-020 | Architecture | Reuse existing components (ring buffer, audio output, DB settings) | 357-361 | High |
| TUNE-ARCH-030 | Architecture | Minimize dependencies (no HTTP server, SSE, queue management) | 363-367 | High |

## Safety and Error Handling (3 requirements)

| ID | Type | Description | Line | Priority |
|----|------|-------------|------|----------|
| TUNE-SAFE-010 | Safety | Preserve user settings (backup, restore on abort) | 371-374 | High |
| TUNE-SAFE-020 | Safety | Detect system problems (hangs, device failures, DB unavailable) | 376-379 | High |
| TUNE-SAFE-030 | Safety | Sanity checks (validate ranges, reject impossible combinations) | 381-384 | Medium |

## Testing Requirements

| ID | Type | Description | Line | Priority |
|----|------|-------------|------|----------|
| TUNE-TEST-040 | Testing | Unit tests (search algorithm, curve fitting, recommendation, JSON) | 388-392 | High |
| TUNE-TEST-050 | Testing | Integration tests (run on CI, verify sanity, validate JSON) | 394-397 | High |
| TUNE-TEST-060 | Testing | Manual validation (run on hardware, apply values, verify 1+ hour stability) | 399-402 | High |

## Success Criteria (3 requirements)

| ID | Type | Description | Line | Priority |
|----|------|-------------|------|----------|
| TUNE-SUCCESS-010 | Quality | Functional requirements (<10 min, stable combinations, actionable, exportable) | 446-450 | High |
| TUNE-SUCCESS-020 | Quality | Quality requirements (<0.1% underruns, no false positives/negatives, reproducible) | 452-456 | High |
| TUNE-SUCCESS-030 | Quality | Usability requirements (progress, understandable, easy to apply, useful errors) | 458-462 | Medium |

## Open Questions (Decisions Made)

| ID | Question | Decision | Line |
|----|----------|----------|------|
| TUNE-Q-010 | Audio device selection strategy | Option A: Use default device from database | 468-471 |
| TUNE-Q-020 | Automatic tuning on first run? | No, on-demand only | 473-476 |
| TUNE-Q-030 | Recommendation aggressiveness | Conservative (6-sigma or 2x trouble point) | 478-481 |
| TUNE-Q-040 | Integration with startup | On-demand only | 483-486 |

## Requirements Summary

**Total:** 31 requirements
- **High Priority:** 29 (94%)
- **Medium Priority:** 2 (6%)

**By Category:**
- Objectives: 5
- Detection: 3
- Search: 4
- Output: 4
- Integration: 3
- Algorithm: 9
- UI: 2
- Architecture: 3
- Safety: 3
- Testing: 3
- Success Criteria: 3

**Referenced Standards:**
- [DBD-PARAM-111] mixer_check_interval_ms (SPEC016:282-295)
- [DBD-PARAM-110] audio_buffer_size (SPEC016:260-276)
- [SSD-RBUF-014] Ring buffer underrun monitoring
- [SSD-MIX-020] Mixer thread configuration
