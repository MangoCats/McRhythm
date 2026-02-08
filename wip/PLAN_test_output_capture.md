# Test Output Capture and Comparison Plan
**Date:** 2026-01-11
**Purpose:** Systematically capture test run outputs for regression detection and performance comparison

## Overview

Establish standardized output capture for all album matcher test runs to enable:
- **Accuracy regression detection:** Catch changes in match results
- **Performance regression detection:** Catch slowdowns
- **Baseline comparison:** Verify optimizations don't break correctness
- **Historical tracking:** Track improvements over time

## Output Artifacts

### 1. Match Results JSON

**File:** `test_results/<run_id>_results.json`

**Structure:**
```json
{
  "run_id": "run30_skip_refinement",
  "timestamp": "2026-01-11T19:42:01Z",
  "configuration": {
    "test_name": "run29f_full_comparison_test",
    "album_count": 200,
    "git_commit": "abc123def",
    "rust_version": "1.75.0",
    "optimizations": ["skip_refinement_perfect", "strict_early_exit"]
  },
  "summary": {
    "total_albums": 200,
    "successful_matches": 198,
    "failed_matches": 2,
    "match_rate_percentage": 99.0,
    "total_duration_seconds": 36000
  },
  "albums": [
    {
      "path": "38 Special/Anthology.mp3",
      "success": true,
      "winning_stage": "Stage2",
      "match_percentage": 100.0,
      "matched_mbid": "94c5881c-e47a-42ba-bea0-d9ff8cb79dd1",
      "matched_title": "Anthology",
      "matched_artist": "38 Special",
      "track_count": 39,
      "detected_track_count": 39,
      "duration_ms": 9454158,
      "processing_time_ms": 736291,
      "differences_from_baseline": []
    },
    {
      "path": "Phildel/Ritual.mp3",
      "success": false,
      "reason": "No match above 65% threshold",
      "best_percentage": 45.2,
      "processing_time_ms": 423122,
      "differences_from_baseline": ["previously_matched"]
    }
  ]
}
```

### 2. Performance Metrics JSON

**File:** `test_results/<run_id>_performance.json`

**Structure:** (See PLAN_performance_instrumentation.md for full structure)

```json
{
  "run_id": "run30_skip_refinement",
  "timestamp": "2026-01-11T19:42:01Z",
  "summary": {
    "total_duration_ms": 36000000,
    "average_ms_per_album": 180000,
    "median_ms_per_album": 120000,
    "p95_ms_per_album": 450000,
    "p99_ms_per_album": 720000
  },
  "per_album": [...],
  "bottlenecks": [...]
}
```

### 3. Comparison Report

**File:** `test_results/comparison_<run1>_vs_<run2>.md`

**Auto-generated markdown report:**

```markdown
# Test Comparison: run29f_baseline vs run30_skip_refinement

## Summary
- **Speedup:** 1.86x (67h → 36h)
- **Accuracy:** No regressions (198/200 matched in both)
- **Differences:** 0 albums changed results

## Performance Changes

| Metric | Baseline | Optimized | Change |
|--------|----------|-----------|--------|
| Total time | 67h 12m | 36h 0m | -46.4% ⬇️ |
| Avg/album | 20m 9s | 10m 48s | -46.4% ⬇️ |
| Median | 12m 30s | 7m 15s | -42.0% ⬇️ |
| P95 | 45m 0s | 28m 30s | -36.7% ⬇️ |

## Slowest Albums Comparison

| Album | Baseline | Optimized | Change |
|-------|----------|-----------|--------|
| Blackmore's Night | 58m 53s | 32m 10s | -45.4% ⬇️ |
| Cars - Panorama | 54m 26s | 28m 45s | -47.2% ⬇️ |
| Bjork - Body Talk | 51m 52s | 30m 22s | -41.4% ⬇️ |

## Accuracy Changes

### Albums with Different Results: 0

### Albums with Same Result, Different Stages: 2

- **Aerosmith/Pump.mp3**
  - Baseline: Stage2 (100.0%)
  - Optimized: Stage2 (100.0%)
  - Note: Early exit saved testing 5 editions

## Bottleneck Analysis

### Time Distribution

| Operation | Baseline | Optimized | Change |
|-----------|----------|-----------|--------|
| Boundary Refinement | 75% | 35% | -53.3% ⬇️ |
| Parameter Search | 17% | 45% | +164.7% ⬆️ |
| Edition Filtering | 5% | 15% | +200.0% ⬆️ |
| Audio Decode | 3% | 5% | +66.7% ⬆️ |

**Analysis:** Boundary refinement reduced from 75% to 35% of total time due to skipping on perfect matches. Parameter search is now dominant bottleneck.
```

### 4. Test Log File

**File:** `test_results/<run_id>_test.log`

**Content:** Full debug log output from test run (already captured by test)

### 5. Git Metadata

**File:** `test_results/<run_id>_metadata.json`

```json
{
  "run_id": "run30_skip_refinement",
  "git_commit": "abc123def456",
  "git_branch": "perf/skip-refinement",
  "git_dirty": false,
  "timestamp": "2026-01-11T19:42:01Z",
  "host": "DESKTOP-XYZ",
  "rust_version": "rustc 1.75.0",
  "cargo_features": [],
  "environment": {
    "RUST_LOG": "wkmp_ai=debug"
  }
}
```

## Directory Structure

```
test_results/
├── registry.json                    # Index of all test runs
├── run29f_baseline/
│   ├── results.json
│   ├── performance.json
│   ├── metadata.json
│   └── test.log
├── run30_skip_refinement/
│   ├── results.json
│   ├── performance.json
│   ├── metadata.json
│   └── test.log
├── run31_strict_early_exit/
│   ├── results.json
│   ├── performance.json
│   ├── metadata.json
│   └── test.log
└── comparisons/
    ├── run29f_vs_run30.md
    ├── run30_vs_run31.md
    └── run29f_vs_run31.md
```

## Implementation

### Phase 1: Result Capture (2 hours)

**Update:** `wkmp-ai/tests/run29f_full_comparison_test.rs`

```rust
#[derive(Serialize)]
struct TestRunResults {
    run_id: String,
    timestamp: String,
    configuration: TestConfiguration,
    summary: TestSummary,
    albums: Vec<AlbumResult>,
}

fn capture_test_results(
    run_id: &str,
    albums: &[AlbumResult],
) -> Result<()> {
    let results = TestRunResults {
        run_id: run_id.to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        configuration: get_test_configuration(),
        summary: calculate_summary(albums),
        albums: albums.to_vec(),
    };

    let output_dir = PathBuf::from("test_results").join(run_id);
    fs::create_dir_all(&output_dir)?;

    let results_file = output_dir.join("results.json");
    let json = serde_json::to_string_pretty(&results)?;
    fs::write(results_file, json)?;

    Ok(())
}
```

### Phase 2: Comparison Tool (4 hours)

**New file:** `scripts/compare_test_runs.py`

```python
#!/usr/bin/env python3
"""
Compare two album matcher test runs for regressions.

Usage:
    python scripts/compare_test_runs.py run29f_baseline run30_skip_refinement
"""

import json
import sys
from pathlib import Path
from typing import Dict, List

def load_results(run_id: str) -> dict:
    results_path = Path("test_results") / run_id / "results.json"
    with open(results_path) as f:
        return json.load(f)

def compare_accuracy(baseline: dict, current: dict) -> dict:
    """Detect accuracy regressions."""
    regressions = []
    improvements = []

    baseline_albums = {a["path"]: a for a in baseline["albums"]}
    current_albums = {a["path"]: a for a in current["albums"]}

    for path in baseline_albums:
        if path not in current_albums:
            continue

        b = baseline_albums[path]
        c = current_albums[path]

        # Regression: was successful, now failed
        if b["success"] and not c["success"]:
            regressions.append({
                "path": path,
                "issue": "match_lost",
                "baseline_percentage": b["match_percentage"],
                "current_percentage": c.get("best_percentage", 0)
            })

        # Regression: MBID changed
        elif b["success"] and c["success"] and b["matched_mbid"] != c["matched_mbid"]:
            regressions.append({
                "path": path,
                "issue": "mbid_changed",
                "baseline_mbid": b["matched_mbid"],
                "current_mbid": c["matched_mbid"]
            })

        # Improvement: was failed, now successful
        elif not b["success"] and c["success"]:
            improvements.append({
                "path": path,
                "current_percentage": c["match_percentage"]
            })

    return {
        "regressions": regressions,
        "improvements": improvements,
        "total_regressions": len(regressions),
        "total_improvements": len(improvements)
    }

def compare_performance(baseline: dict, current: dict) -> dict:
    """Calculate performance changes."""
    baseline_perf = Path("test_results") / baseline["run_id"] / "performance.json"
    current_perf = Path("test_results") / current["run_id"] / "performance.json"

    if not baseline_perf.exists() or not current_perf.exists():
        return {"available": False}

    with open(baseline_perf) as f:
        b = json.load(f)
    with open(current_perf) as f:
        c = json.load(f)

    return {
        "available": True,
        "speedup": b["summary"]["total_duration_ms"] / c["summary"]["total_duration_ms"],
        "baseline_total_ms": b["summary"]["total_duration_ms"],
        "current_total_ms": c["summary"]["total_duration_ms"],
        "time_saved_ms": b["summary"]["total_duration_ms"] - c["summary"]["total_duration_ms"],
    }

def generate_markdown_report(baseline: dict, current: dict, output_path: Path):
    """Generate comparison markdown report."""
    accuracy = compare_accuracy(baseline, current)
    performance = compare_performance(baseline, current)

    with open(output_path, 'w') as f:
        f.write(f"# Test Comparison: {baseline['run_id']} vs {current['run_id']}\n\n")

        # Summary
        f.write("## Summary\n\n")
        if performance["available"]:
            f.write(f"- **Speedup:** {performance['speedup']:.2f}x\n")
            f.write(f"- **Time saved:** {performance['time_saved_ms'] / 3600000:.1f}h\n")
        f.write(f"- **Accuracy regressions:** {accuracy['total_regressions']}\n")
        f.write(f"- **Accuracy improvements:** {accuracy['total_improvements']}\n\n")

        # Regressions
        if accuracy["regressions"]:
            f.write("## ⚠️ REGRESSIONS DETECTED\n\n")
            for reg in accuracy["regressions"]:
                f.write(f"### {reg['path']}\n")
                f.write(f"- **Issue:** {reg['issue']}\n")
                f.write(f"- **Details:** {reg}\n\n")

        # More sections...

if __name__ == "__main__":
    if len(sys.argv) != 3:
        print("Usage: compare_test_runs.py <baseline_run_id> <current_run_id>")
        sys.exit(1)

    baseline = load_results(sys.argv[1])
    current = load_results(sys.argv[2])

    output_path = Path("test_results") / "comparisons" / f"{sys.argv[1]}_vs_{sys.argv[2]}.md"
    output_path.parent.mkdir(parents=True, exist_ok=True)

    generate_markdown_report(baseline, current, output_path)
    print(f"Report generated: {output_path}")
```

### Phase 3: CI Integration (2 hours)

**New file:** `.github/workflows/performance-regression.yml`

```yaml
name: Performance Regression Check

on:
  pull_request:
    paths:
      - 'wkmp-ai/src/matching/**'
      - 'wkmp-ai/tests/**'

jobs:
  check-performance:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Run baseline comparison
        run: |
          cargo test --test run29f_full_comparison_test -- --ignored
          python scripts/compare_test_runs.py run29f_baseline current_run

      - name: Check for regressions
        run: |
          python scripts/detect_regressions.py \
            --baseline test_results/run29f_baseline/results.json \
            --current test_results/current_run/results.json \
            --accuracy-threshold 0 \
            --performance-threshold 10

      - name: Upload results
        uses: actions/upload-artifact@v3
        with:
          name: test-comparison
          path: test_results/comparisons/
```

## Usage Workflow

### Running a Baseline Test

```bash
# Run test with specific run ID
WKMP_TEST_RUN_ID=run29f_baseline cargo test --test run29f_full_comparison_test -- --ignored

# Results automatically captured to test_results/run29f_baseline/
```

### Running an Optimized Test

```bash
# After implementing optimization
WKMP_TEST_RUN_ID=run30_skip_refinement cargo test --test run29f_full_comparison_test -- --ignored

# Compare with baseline
python scripts/compare_test_runs.py run29f_baseline run30_skip_refinement

# Check report
cat test_results/comparisons/run29f_baseline_vs_run30_skip_refinement.md
```

### Detecting Regressions

```bash
# Strict check (fail on any regression)
python scripts/detect_regressions.py \
  --baseline test_results/run29f_baseline/results.json \
  --current test_results/run30_skip_refinement/results.json \
  --accuracy-threshold 0 \
  --performance-threshold 0

# Returns exit code 1 if regressions detected
```

## Success Criteria

- ✅ All test runs automatically capture results JSON
- ✅ Comparison tool detects 100% of accuracy regressions
- ✅ Performance comparisons accurate within 1%
- ✅ Reports generated automatically
- ✅ CI integration catches regressions before merge
- ✅ Historical data preserved for all runs

## Future Enhancements

1. **Web dashboard:** Interactive visualization of test results over time
2. **Automatic bisection:** Binary search to find regression commit
3. **Detailed diff:** Track per-album changes in detected durations
4. **Alerting:** Slack/email notifications on regressions
5. **Trend analysis:** Detect gradual performance degradation
