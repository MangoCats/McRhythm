# Experiment Automation Scripts

Automated parallel execution system for album matching improvement experiments.

## Overview

Three-tier automation system:
1. **Worker:** `run_experiment.ps1` - Executes single experiment
2. **Coordinator:** `run_phase1_parallel.ps1` - Manages parallel experiments within a phase
3. **Orchestrator:** `run_all_phases_hybrid.ps1` - Manages all phases with hybrid strategy

## Quick Start

### Run Phase 1 (Quality Floor Experiments)

```powershell
cd c:\Users\Mango Cat\Dev\McRhythm
.\scripts\run_phase1_parallel.ps1
```

This will:
1. Launch 3 parallel experiments testing floor values {0.0, -0.3, -0.5}
2. Run 200-file comparison test for each
3. Analyze results and categorize as Better/Equivocal/Worse
4. Select winner based on fewest regressions, then most improvements
5. Save decision to `am\phase1_decision.json`

**Expected Duration:** ~30-45 minutes

### Run Single Experiment (Manual)

```powershell
.\scripts\run_experiment.ps1 `
    -ExperimentId "exp1a" `
    -BranchName "exp1a-floor-0.0" `
    -Changes @{
        "wkmp-ai\src\matching\editions\scoring.rs" = @(
            @{
                Old = "const MIN_QUALITY_FLOOR: f64 = -1.0;"
                New = "const MIN_QUALITY_FLOOR: f64 = 0.0;"
            }
        )
    }
```

### Analyze Existing Results

```powershell
.\scripts\analyze_experiment.ps1 `
    -ExperimentFile "am\exp1a_results.json" `
    -BaselineFile "am\run29f_comparison_results_with_stage4_only_refinement.json" `
    -OutputFile "am\exp1a_analysis.json"
```

## Script Reference

### run_experiment.ps1

**Purpose:** Execute a single experiment end-to-end

**Parameters:**
- `ExperimentId` (required): Unique identifier (e.g., "exp1a")
- `BranchName` (required): Git branch name for isolation
- `Changes` (required): Hashtable of file changes
- `ResultsDir` (optional): Output directory (default: "am")
- `SkipBuild` (optional): Skip cargo build (use existing binary)
- `CleanupBranch` (optional): Delete branch after completion (default: true)

**Output:**
- `am\{ExperimentId}_results.json` - Test results
- `logs\{ExperimentId}_*.log` - Execution logs
- Hashtable with metrics (returned to caller)

**Example:**
```powershell
$Result = .\scripts\run_experiment.ps1 `
    -ExperimentId "exp1a" `
    -BranchName "exp1a-floor-0.0" `
    -Changes @{
        "wkmp-ai\src\matching\editions\scoring.rs" = @(
            @{
                Old = "const MIN_QUALITY_FLOOR: f64 = -1.0;"
                New = "const MIN_QUALITY_FLOOR: f64 = 0.0;"
            }
        )
    }

# Access results
Write-Host "Status: $($Result.Status)"
Write-Host "Exact matches: $($Result.ExactMatches)"
Write-Host "MBID changes: $($Result.MbidChanges)"
```

---

### run_phase1_parallel.ps1

**Purpose:** Run Phase 1 quality floor experiments in parallel

**Parameters:**
- `SkipTests` (optional): Skip test execution (analyze existing results only)
- `AnalyzeOnly` (optional): Only run analysis, don't execute experiments

**Output:**
- `am\exp1a_results.json` - Experiment 1A results (floor = 0.0)
- `am\exp1b_results.json` - Experiment 1B results (floor = -0.3)
- `am\exp1c_results.json` - Experiment 1C results (floor = -0.5)
- `am\phase1_experiment_summary.json` - All experiment metrics
- `am\phase1_decision.json` - Winner selection
- `logs\phase1_coordinator_*.log` - Coordinator log

**Example:**
```powershell
# Run all experiments
.\scripts\run_phase1_parallel.ps1

# Analyze existing results only
.\scripts\run_phase1_parallel.ps1 -AnalyzeOnly
```

**Decision Criteria:**
1. **Primary:** Fewest regressions ("clearly worse" count)
2. **Secondary:** Most improvements ("clearly better" count)

---

### analyze_experiment.ps1

**Purpose:** Compare experiment results to baseline and categorize changes

**Parameters:**
- `ExperimentFile` (required): Path to experiment results JSON
- `BaselineFile` (required): Path to baseline results JSON
- `OutputFile` (optional): Save analysis to file

**Output:**
Hashtable with:
- `TotalAlbums` - Total albums tested
- `ExactMatches` - Albums with identical MBID (no change)
- `MbidChanges` - Albums with different MBID
- `ClearlyBetter` - Improved matches
- `Equivocal` - Similar quality, different edition
- `ClearlyWorse` - Degraded matches
- `DetailedResults` - Per-album categorization

**Categorization Rules:**
- **Clearly Better:**
  - Match % improved by >5% AND track count ≥ baseline
  - Track count closer to expected (within ±5% match %)
- **Clearly Worse:**
  - Match % decreased by >10%
  - Track count massively wrong (1 track when expecting 10+)
  - Match % <70% when baseline >80%
- **Equivocal:**
  - Similar match % (±5%), different MBID
  - Unclear which is better

---

### run_all_phases_hybrid.ps1

**Purpose:** Orchestrate all phases with hybrid parallelization

**Parameters:**
- `StartPhase` (optional): Which phase to start from (default: "All")
- `DryRun` (optional): Show what would run without executing

**Example:**
```powershell
# Run all phases
.\scripts\run_all_phases_hybrid.ps1

# Dry run to preview
.\scripts\run_all_phases_hybrid.ps1 -DryRun

# Start from Phase 2
.\scripts\run_all_phases_hybrid.ps1 -StartPhase Phase2
```

**Note:** Currently only Phase 1 is fully implemented. Phases 2-5 require manual code changes for experiments.

---

## File Structure

```
c:\Users\Mango Cat\Dev\McRhythm\
├── scripts\
│   ├── run_experiment.ps1                 # Worker: Single experiment
│   ├── run_phase1_parallel.ps1            # Coordinator: Phase 1
│   ├── analyze_experiment.ps1             # Analyzer: Categorize results
│   ├── run_all_phases_hybrid.ps1          # Orchestrator: All phases
│   └── EXPERIMENTS_README.md              # This file
├── am\
│   ├── run29f_comparison_results_with_stage4_only_refinement.json  # Baseline
│   ├── exp1a_results.json                 # Experiment results
│   ├── exp1b_results.json
│   ├── exp1c_results.json
│   ├── phase1_experiment_summary.json     # Phase summary
│   ├── phase1_decision.json               # Phase decision
│   ├── algorithm_improvement_roadmap.md   # Master roadmap
│   └── parallel_execution_strategy.md     # Strategy doc
└── logs\
    ├── exp1a_*.log                        # Experiment logs
    ├── phase1_coordinator_*.log           # Coordinator logs
    └── hybrid_orchestrator_*.log          # Orchestrator logs
```

---

## Workflow

### Phase 1 Execution Flow

```
User runs: run_phase1_parallel.ps1
    ↓
Coordinator launches 3 parallel PowerShell jobs
    ↓
Job 1: run_experiment.ps1 (exp1a, floor=0.0)
Job 2: run_experiment.ps1 (exp1b, floor=-0.3)
Job 3: run_experiment.ps1 (exp1c, floor=-0.5)
    ↓
Each job:
  1. Creates git branch
  2. Modifies scoring.rs
  3. Builds release binary
  4. Runs 200-file test
  5. Saves results to am\
  6. Commits and returns to main
    ↓
Coordinator waits for all jobs to complete
    ↓
For each result:
  1. Run analyze_experiment.ps1
  2. Categorize as Better/Equivocal/Worse
  3. Count regressions and improvements
    ↓
Select winner:
  - Fewest regressions (primary)
  - Most improvements (secondary)
    ↓
Save decision to phase1_decision.json
    ↓
Display next steps based on results
```

---

## Troubleshooting

### Build Fails

**Symptom:** `cargo build --release` fails

**Solutions:**
1. Check for syntax errors in code changes
2. Verify Changes hashtable has correct file paths
3. Review `logs\{ExperimentId}_build.log`

### Test Hangs

**Symptom:** Test runs indefinitely without output

**Solutions:**
1. Check for infinite loops in modified code
2. Monitor system resources (disk, memory)
3. Kill test: `Stop-Process -Name cargo`

### Results File Missing

**Symptom:** `wkmp-ai\run29f_comparison_results.json` not found

**Solutions:**
1. Check test actually ran (see `logs\{ExperimentId}_test.log`)
2. Verify test output directory
3. Check for test failures in log

### Parallel Jobs Fail

**Symptom:** All 3 jobs fail with git errors

**Solutions:**
1. Ensure no uncommitted changes before starting
2. Check git branches aren't already in use
3. Run jobs sequentially instead of parallel (slower but more stable)

### Analysis Shows 0 Changes

**Symptom:** All albums categorized as "Exact Matches"

**Solutions:**
1. Verify code changes were actually applied
2. Check experiment and baseline are different files
3. Rebuild binary after changes

---

## Performance

**Single Experiment:**
- Build: ~5-10 min (first time), ~2-3 min (incremental)
- Test: ~25-30 min (200 albums)
- Total: ~30-40 min

**Phase 1 (3 parallel):**
- Wall-clock: ~30-45 min (3× speedup)
- CPU: 3× build time (all parallel)

**Resource Requirements:**
- Disk: ~10GB free (target/ directory)
- RAM: ~8GB (3 parallel builds)
- CPU: 4+ cores recommended

---

## Advanced Usage

### Custom Experiment Definition

Create custom experiment without using coordinator:

```powershell
$CustomChanges = @{
    "wkmp-ai\src\matching\editions\scoring.rs" = @(
        @{
            Old = "const DURATION_WEIGHT: f64 = 0.30;"
            New = "const DURATION_WEIGHT: f64 = 0.40;"
        },
        @{
            Old = "const QUALITY_WEIGHT: f64 = 0.45;"
            New = "const QUALITY_WEIGHT: f64 = 0.35;"
        }
    )
}

.\scripts\run_experiment.ps1 `
    -ExperimentId "custom1" `
    -BranchName "custom1-reweight" `
    -Changes $CustomChanges
```

### Rerun Failed Experiment

If experiment failed partway through:

```powershell
# Resume without recreating branch
git checkout exp1a-floor-0.0

# Manual rebuild and test
cargo build --release
cargo test --release run29f_full_baseline_comparison -- --nocapture

# Copy results
cp wkmp-ai\run29f_comparison_results.json am\exp1a_results.json

# Return to main
git checkout main
```

### Compare Two Experiments

```powershell
# Compare exp1a vs exp1b (both vs baseline)
$Analysis1a = .\scripts\analyze_experiment.ps1 `
    -ExperimentFile "am\exp1a_results.json" `
    -BaselineFile "am\run29f_baseline.json"

$Analysis1b = .\scripts\analyze_experiment.ps1 `
    -ExperimentFile "am\exp1b_results.json" `
    -BaselineFile "am\run29f_baseline.json"

# Compare metrics
Write-Host "exp1a: $($Analysis1a.ClearlyWorse) regressions, $($Analysis1a.ClearlyBetter) improvements"
Write-Host "exp1b: $($Analysis1b.ClearlyWorse) regressions, $($Analysis1b.ClearlyBetter) improvements"
```

---

## Next Steps

After Phase 1 completes:

1. **Review Decision:** Check `am\phase1_decision.json`
2. **Analyze Regressions:** If any regressions remain, review detailed results
3. **Update Roadmap:** Document findings in `am\algorithm_improvement_roadmap.md`
4. **Proceed to Phase 2:** If successful (≤2 regressions)
5. **Pivot to Phase 3A:** If unsuccessful (>2 regressions), consider run29f-style scoring

---

## References

- **Roadmap:** [am/algorithm_improvement_roadmap.md](../am/algorithm_improvement_roadmap.md)
- **Strategy:** [am/parallel_execution_strategy.md](../am/parallel_execution_strategy.md)
- **Regression Analysis:** [am/run29f_regression_analysis.md](../am/run29f_regression_analysis.md)
