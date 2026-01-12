# Performance Instrumentation Plan
**Date:** 2026-01-11
**Purpose:** Add comprehensive performance monitoring to identify bottlenecks in album matching

## Overview

Add structured performance metrics collection throughout the album matching pipeline to identify bottlenecks without relying on debug log analysis. Metrics should be:
- **Minimal overhead** (<1% performance impact)
- **Structured** (JSON output for automated analysis)
- **Hierarchical** (per-album, per-stage, per-edition, per-operation)
- **Actionable** (identify specific slow operations)

## Instrumentation Points

### 1. Top-Level Album Processing

**Location:** `wkmp-ai/src/matching/album_matcher.rs::match_album()`

**Metrics:**
```rust
struct AlbumPerformanceMetrics {
    album_path: String,
    total_duration_ms: u64,

    // Phase timings
    decode_duration_ms: u64,
    musicbrainz_duration_ms: u64,
    edition_filtering_duration_ms: u64,
    orchestration_duration_ms: u64,

    // Counts
    mb_editions_fetched: usize,
    editions_after_filtering: usize,
    editions_tested: usize,

    // Results
    success: bool,
    winning_stage: Option<String>,
    match_percentage: f64,
}
```

**Implementation:**
- Use `std::time::Instant` for wall-clock timing
- Wrap each major phase in timing block
- Output JSON at completion

### 2. Stage-Level Performance

**Location:** `wkmp-ai/src/matching/orchestrator.rs::run_orchestration()`

**Metrics:**
```rust
struct StagePerformanceMetrics {
    stage_name: String,
    duration_ms: u64,
    editions_tested: usize,
    results_produced: usize,
    early_exit: bool,
    best_percentage: f64,
}
```

**Instrumentation:**
- Time each stage (Stage2, Stage3, Stage4, Stage5, Stage6)
- Record edition count and result count
- Track early exit conditions

### 3. Edition-Level Performance

**Location:** `wkmp-ai/src/matching/stages/stage2.rs::test_edition_stage2()`

**Metrics:**
```rust
struct EditionPerformanceMetrics {
    edition_mbid: String,
    stage: String,

    // Timings (microseconds for precision)
    parameter_search_us: u64,
    boundary_refinement_us: u64,
    total_duration_us: u64,

    // Parameter grid details (Stage 2 only)
    parameter_combinations_tested: usize,
    best_percentage: f64,
    refinement_applied: bool,
}
```

**Instrumentation:**
- Time parameter grid search separately from boundary refinement
- Record whether refinement was applied
- Track parameter combinations tested before early exit

### 4. Boundary Refinement Performance

**Location:** `wkmp-ai/src/matching/stages/boundary_refinement.rs::refine_missed_boundaries()`

**Metrics:**
```rust
struct BoundaryRefinementMetrics {
    detected_track_count: usize,
    expected_track_count: usize,

    // Detection
    split_failures_detected: usize,
    boundaries_searched: usize,

    // Timings (microseconds)
    detection_duration_us: u64,
    search_duration_us: u64,
    total_duration_us: u64,

    // Results
    boundaries_refined: usize,
    iterations_used: usize,
}
```

**Instrumentation:**
- Time split failure detection separately from boundary search
- Count actual searches performed
- Track refinement iterations

## Output Format

### Per-Album JSON Structure

```json
{
  "album_path": "C:\\Users\\...\\Cars\\Panorama.mp3",
  "timestamp": "2026-01-11T03:07:19Z",
  "performance": {
    "total_ms": 3266120,
    "phases": {
      "decode_ms": 45230,
      "musicbrainz_ms": 1250,
      "filtering_ms": 3420,
      "orchestration_ms": 3216220
    },
    "stages": [
      {
        "name": "Stage2",
        "duration_ms": 3000000,
        "editions_tested": 3,
        "early_exit": false,
        "editions": [
          {
            "mbid": "7e5f45e4-f449-4329-97c9-ca364cd89fbd",
            "parameter_search_us": 125000,
            "boundary_refinement_us": 999875000,
            "refinement_applied": true,
            "best_percentage": 82.1
          }
        ]
      },
      {
        "name": "Stage6",
        "duration_ms": 216220,
        "boundaries_refined": 2
      }
    ]
  },
  "result": {
    "success": true,
    "winning_stage": "Stage2",
    "match_percentage": 100.0
  }
}
```

### Aggregated Performance Report

```json
{
  "test_run_id": "run30_baseline",
  "timestamp": "2026-01-11T19:42:01Z",
  "summary": {
    "total_albums": 200,
    "total_duration_ms": 240960000,
    "average_ms_per_album": 1204800,
    "median_ms_per_album": 720000
  },
  "bottlenecks": [
    {
      "operation": "boundary_refinement",
      "total_ms": 180000000,
      "percentage": 74.7,
      "affected_albums": 156
    },
    {
      "operation": "stage2_parameter_search",
      "total_ms": 42000000,
      "percentage": 17.4,
      "affected_albums": 200
    }
  ],
  "slowest_albums": [
    {
      "path": "Blackmore's Night/BeyondTheSunset.mp3",
      "duration_ms": 3533000,
      "bottleneck": "boundary_refinement"
    }
  ]
}
```

## Implementation Plan

### Phase 1: Core Infrastructure (2-3 hours)

1. Create `wkmp-ai/src/performance/mod.rs` module
2. Define metrics structs with `serde` derives
3. Create `PerformanceCollector` with thread-safe interior mutability
4. Add JSON output functions

### Phase 2: Instrumentation (4-5 hours)

1. **Album-level:** Wrap `match_album()` phases with timing
2. **Stage-level:** Add timing to each `run_stageN()` function
3. **Edition-level:** Time `test_edition_stage2()` components
4. **Refinement-level:** Time detection and search phases

### Phase 3: Reporting (2-3 hours)

1. Create aggregation functions for performance data
2. Build bottleneck detection algorithm
3. Generate HTML performance report (optional)
4. Add CI integration for regression detection

### Phase 4: Testing (1-2 hours)

1. Verify <1% performance overhead
2. Test JSON output validity
3. Verify aggregation accuracy
4. Document usage in README

## Usage

### Enable Performance Monitoring

```bash
# Environment variable to enable
WKMP_PERF_MONITORING=1 cargo test --test run29f_full_comparison_test -- --ignored

# Output files
# - Per-album: perf_data/album_<hash>.json
# - Aggregate: perf_data/run_summary_<timestamp>.json
```

### Analyze Performance

```bash
# Compare two runs
python scripts/compare_performance.py perf_data/run29f_summary.json perf_data/run30_summary.json

# Identify regressions
python scripts/detect_regressions.py --baseline perf_data/run29f_summary.json --current perf_data/run30_summary.json --threshold 10
```

## Success Criteria

- ✅ Performance overhead < 1% of total runtime
- ✅ Identifies bottlenecks automatically (matches manual analysis)
- ✅ JSON output validates against schema
- ✅ Regression detection catches >5% slowdowns
- ✅ Works with existing test infrastructure

## Integration with Existing Tools

### Tracing Integration

Performance metrics complement (not replace) debug tracing:
- **Tracing:** Detailed execution flow, debugging
- **Performance metrics:** Structured timing data, bottleneck identification

Both can be enabled simultaneously for deep analysis.

### Test Harness Integration

Add performance collection to `run29f_full_comparison_test.rs`:
```rust
let mut perf_collector = PerformanceCollector::new("run30_optimized");
for album in albums {
    let metrics = match_album_with_metrics(album, &mut perf_collector);
}
perf_collector.write_summary("perf_data/run30_summary.json")?;
```

## Future Enhancements

1. **Real-time dashboard:** Web UI showing progress and bottlenecks
2. **Memory profiling:** Track memory allocation patterns
3. **Cache effectiveness:** Measure cache hit rates and impact
4. **Parallel efficiency:** Measure actual speedup from parallelization
5. **Historical tracking:** Database of performance over time
