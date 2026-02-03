# Experiment System - Quick Start Guide

Complete automation system for parallel album matching experiments.

## System Components

1. **Roadmap:** [algorithm_improvement_roadmap.md](algorithm_improvement_roadmap.md) - 5-phase experimental plan
2. **Strategy:** [parallel_execution_strategy.md](parallel_execution_strategy.md) - Parallelization approach
3. **Automation:** [../scripts/EXPERIMENTS_README.md](../scripts/EXPERIMENTS_README.md) - Script documentation
4. **Baseline:** [run29f_regression_analysis.md](run29f_regression_analysis.md) - Current issues analysis

---

## Quick Start (3 Steps)

### Step 1: Validate Reproducibility (5 minutes)

Ensure tests produce identical results before parallel execution:

```powershell
cd "c:\Users\Mango Cat\Dev\McRhythm"
.\scripts\validate_reproducibility.ps1
```

**Expected Output:**
```
✓ All 2 runs produced identical results!
🎉 Reproducibility VALIDATED - Safe to run parallel experiments
```

**If validation fails:**
- Do NOT run parallel experiments
- Investigate non-determinism (see script output)
- Fix issues before proceeding

---

### Step 2: Run Phase 1 Experiments (30-45 minutes)

Test 3 quality floor values in parallel:

```powershell
.\scripts\run_phase1_parallel.ps1
```

**What Happens:**
- Launches 3 parallel jobs (exp1a, exp1b, exp1c)
- Each job:
  1. Creates git branch
  2. Modifies `MIN_QUALITY_FLOOR` in scoring.rs
  3. Builds release binary
  4. Runs 200-album comparison test
  5. Saves results to `am\exp{id}_results.json`
- Analyzes all results and selects winner
- Saves decision to `am\phase1_decision.json`

**Monitor Progress:**
- Watch console for status updates
- Check `logs\phase1_coordinator_*.log` for details
- View `logs\exp1a_*.log`, `exp1b_*.log`, `exp1c_*.log` for individual experiments

---

### Step 3: Review Results and Decide (5 minutes)

Check Phase 1 decision:

```powershell
# View decision summary
Get-Content am\phase1_decision.json | ConvertFrom-Json | Format-List

# View detailed analysis
Get-Content am\exp1a_results.json | ConvertFrom-Json | Where-Object { $_.mbid_match -eq $false } | Select-Object artist, album, match_percentage
```

**Success Criteria:**
- ✓ **0 regressions:** Proceed to Phase 2
- ⚠ **1-2 regressions:** Consider proceeding with caution
- ✗ **>2 regressions:** Pivot to alternative approach (Phase 3A)

---

## What Each Experiment Tests

### Phase 1: Quality Floor

| Experiment | Floor Value | Hypothesis |
|------------|-------------|------------|
| **exp1a** | 0.0 | Match run29f: No negative penalties |
| **exp1b** | -0.3 | Softer penalty than -1.0 |
| **exp1c** | -0.5 | Middle ground |

**Goal:** Eliminate 7 regressions caused by quality score collapse

**Expected Winner:** exp1a (0.0 floor)
- Fixes The Cars - Panorama (quality score no longer collapses)
- Matches run29f behavior (errors contribute 0, not negative)

---

## Decision Tree

```
Phase 1 Complete
    ↓
Check Regressions
    ↓
    ├─ 0 regressions → ✓ Proceed to Phase 2 (Refinement Timing)
    ├─ 1-2 regressions → ⚠ Proceed with caution or investigate
    └─ >2 regressions → ✗ Pivot to Phase 3A (run29f-style scoring)
```

---

## Expected Timeline

| Phase | Duration | Parallelization | Agents |
|-------|----------|-----------------|--------|
| **Validation** | 5 min | N/A | 1 |
| **Phase 1** | 30-45 min | Full (3 parallel) | 3 |
| **Analysis** | 5 min | N/A | 1 |
| **Total** | **40-55 min** | - | - |

Compare to sequential: **7.5 hours** → 67% time savings

---

## Files Generated

### During Execution

```
am\
├── exp1a_results.json                     # Experiment 1A results (floor=0.0)
├── exp1b_results.json                     # Experiment 1B results (floor=-0.3)
├── exp1c_results.json                     # Experiment 1C results (floor=-0.5)
├── phase1_experiment_summary.json         # Summary of all experiments
└── phase1_decision.json                   # Winner selection + metrics

logs\
├── phase1_coordinator_*.log               # Coordinator execution log
├── exp1a_*.log                            # Experiment 1A logs (build, test)
├── exp1b_*.log                            # Experiment 1B logs
└── exp1c_*.log                            # Experiment 1C logs
```

### Git Branches (Temporary)

```
main                                       # Original branch
exp1a-floor-0.0                           # Experiment 1A branch (deleted after)
exp1b-floor-neg0.3                        # Experiment 1B branch (deleted after)
exp1c-floor-neg0.5                        # Experiment 1C branch (deleted after)
```

Branches are automatically deleted after results are saved.

---

## Troubleshooting

### "Build failed"

**Symptom:** Experiment fails during cargo build

**Solution:**
1. Check `logs\exp{id}_build.log` for errors
2. Verify code changes are syntactically correct
3. Try manual build: `cargo build --release`

### "Test never completes"

**Symptom:** Test runs for >1 hour without finishing

**Solution:**
1. Check system resources (disk, RAM)
2. Review `logs\exp{id}_test.log` for last output
3. Kill hung process: `Stop-Process -Name cargo`
4. Investigate infinite loops in modified code

### "All experiments failed"

**Symptom:** All 3 jobs fail with git errors

**Solution:**
1. Ensure clean working directory: `git status`
2. Commit or stash any changes before running
3. Check branches don't already exist: `git branch -a`
4. Run experiments sequentially (slower but more stable)

### "Reproducibility validation failed"

**Symptom:** Different checksums across runs

**Solution:**
1. **DO NOT** run parallel experiments
2. Review validation output for specific differences
3. Check for:
   - Random number generation without fixed seed
   - Timestamp inclusion in results
   - HashMap iteration order issues
4. Fix non-determinism before proceeding

---

## Next Steps After Phase 1

### If Phase 1 Succeeds (0-2 regressions)

1. **Document Winner:** Update roadmap with Phase 1 results
2. **Lock Configuration:** Use winning floor value for Phase 2
3. **Proceed to Phase 2:** Test refinement timing with locked floor

**Phase 2 Execution:**
```powershell
# Manual for now - requires code changes
# See roadmap Experiment 2A and 2B specifications
```

### If Phase 1 Fails (>2 regressions)

1. **Analyze Failures:** Review which albums still regress
2. **Consider Alternatives:**
   - Increase tolerance (8s → 12s or 15s)
   - Revert to run29f-style scoring (Phase 3A)
   - Investigate boundary detection improvements
3. **Update Roadmap:** Document findings and pivot strategy

---

## Advanced Usage

### Rerun Single Experiment

If one experiment fails, rerun just that one:

```powershell
.\scripts\run_experiment.ps1 `
    -ExperimentId "exp1a" `
    -BranchName "exp1a-floor-0.0-retry" `
    -Changes @{
        "wkmp-ai\src\matching\editions\scoring.rs" = @(
            @{
                Old = "const MIN_QUALITY_FLOOR: f64 = -1.0;"
                New = "const MIN_QUALITY_FLOOR: f64 = 0.0;"
            }
        )
    }
```

### Compare Specific Albums

Compare how experiments handled specific regression cases:

```powershell
# Load results
$Exp1a = Get-Content am\exp1a_results.json | ConvertFrom-Json
$Exp1b = Get-Content am\exp1b_results.json | ConvertFrom-Json
$Baseline = Get-Content am\run29f_comparison_results_with_stage4_only_refinement.json | ConvertFrom-Json

# Find The Cars - Panorama
$Cars1a = $Exp1a | Where-Object { $_.album -eq "Panorama" -and $_.artist -eq "The Cars" }
$Cars1b = $Exp1b | Where-Object { $_.album -eq "Panorama" -and $_.artist -eq "The Cars" }
$CarsBase = $Baseline | Where-Object { $_.album -eq "Panorama" -and $_.artist -eq "The Cars" }

Write-Host "Baseline: $($CarsBase.current_mbid) ($($CarsBase.current_tracks) tracks, $($CarsBase.match_percentage)%)"
Write-Host "Exp1a:    $($Cars1a.current_mbid) ($($Cars1a.current_tracks) tracks, $($Cars1a.match_percentage)%)"
Write-Host "Exp1b:    $($Cars1b.current_mbid) ($($Cars1b.current_tracks) tracks, $($Cars1b.match_percentage)%)"
```

### Extract Regression List

Get list of all regressions from winning experiment:

```powershell
$Decision = Get-Content am\phase1_decision.json | ConvertFrom-Json
$Winner = $Decision.Winner

# Load analysis (would need to regenerate)
.\scripts\analyze_experiment.ps1 `
    -ExperimentFile "am\${Winner}_results.json" `
    -BaselineFile "am\run29f_comparison_results_with_stage4_only_refinement.json" `
    -OutputFile "am\${Winner}_analysis.json"

$Analysis = Get-Content "am\${Winner}_analysis.json" | ConvertFrom-Json

# Extract regressions
$Regressions = $Analysis.DetailedResults | Where-Object { $_.Category -eq "Worse" }

Write-Host "Regressions in $Winner:"
$Regressions | Format-Table Artist, Album, Category, BaselineMatchPct, ExperimentMatchPct
```

---

## References

- **Full Roadmap:** [algorithm_improvement_roadmap.md](algorithm_improvement_roadmap.md)
- **Parallel Strategy:** [parallel_execution_strategy.md](parallel_execution_strategy.md)
- **Script Docs:** [../scripts/EXPERIMENTS_README.md](../scripts/EXPERIMENTS_README.md)
- **Baseline Analysis:** [run29f_regression_analysis.md](run29f_regression_analysis.md)
- **run29f Source:** [../wkmp-ai/examples/am29/](../wkmp-ai/examples/am29/)

---

## Summary

**3-Step Quick Start:**
1. Validate reproducibility (5 min)
2. Run Phase 1 parallel (30-45 min)
3. Review and decide (5 min)

**Total: 40-55 minutes** to complete Phase 1 vs. 2.5 hours sequential

**Success Metric:** 0 regressions (current: 7)

**Next:** Phase 2 (refinement timing) if successful, or pivot to alternative approach if not.
