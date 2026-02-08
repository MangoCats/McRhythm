# Parallel Experiment Execution Strategy
**Created:** 2025-12-31
**Purpose:** Maximize throughput by running independent experiments concurrently

---

## Parallelization Potential Analysis

### What CAN Be Parallelized

#### 1. Independent Parameter Value Tests
**Experiments that test different values of the same parameter can run in parallel.**

Example: Phase 1 Quality Floor
- Experiment 1A (floor = 0.0)
- Experiment 1B (floor = -0.3)
- Experiment 1C (floor = -0.5)

**Method:** Git branch per experiment
```bash
# Agent 1
git checkout -b exp1a-floor-0.0
# Change MIN_QUALITY_FLOOR to 0.0
cargo test --release run29f_full_baseline_comparison
# Save results to am/exp1a_results.json

# Agent 2 (parallel)
git checkout -b exp1b-floor-neg0.3
# Change MIN_QUALITY_FLOOR to -0.3
cargo test --release run29f_full_baseline_comparison
# Save results to am/exp1b_results.json

# Agent 3 (parallel)
git checkout -b exp1c-floor-neg0.5
# Change MIN_QUALITY_FLOOR to -0.5
cargo test --release run29f_full_baseline_comparison
# Save results to am/exp1c_results.json
```

**Parallelization:** 3 agents simultaneously (1 per value)
**Time Savings:** 3× (from 90 min to 30 min for Phase 1)

---

#### 2. Independent Algorithm Variations
**Experiments testing different algorithms can run in parallel.**

Example: Phase 3 Scoring Algorithms
- Experiment 3A (pure match %)
- Experiment 3B (hybrid scoring)
- Experiment 3C (weighted scoring)

**Method:** Git branch per algorithm
**Parallelization:** 3 agents simultaneously
**Time Savings:** 3× (from 90 min to 30 min)

---

#### 3. Parameter Sweep Tests
**Testing multiple values of independent parameters.**

Example: Phase 4A Tolerance Values
- tolerance = 8s
- tolerance = 10s (baseline)
- tolerance = 12s
- tolerance = 15s

**Method:** Git branch per value
**Parallelization:** 4 agents simultaneously
**Time Savings:** 4× (from 120 min to 30 min)

---

#### 4. Novel Algorithm Exploration
**Phase 5 experiments are mostly independent.**

Example: Phase 5 Novel Algorithms
- 5A: Confidence-weighted selection
- 5B: Median quality score
- 5C: Adaptive tolerance
- 5D: Two-pass boundary detection

**Method:** Git branch per algorithm
**Parallelization:** 4 agents simultaneously
**Time Savings:** 4× (from 120 min to 30 min)

---

### What MUST Be Sequential

#### 1. Decision-Dependent Experiments
**Experiments where next step depends on previous results.**

Example: Phase 1 → Phase 2 dependency
```
Phase 1: Lock quality floor value
  ↓ (MUST WAIT for decision)
Phase 2: Test refinement timing with locked floor
```

**Why:** Phase 2 needs to know which floor value to use (from Phase 1 analysis).

**Solution:** Run Phase 1 in parallel (all floor values), then analyze results, make decision, THEN start Phase 2 with locked value.

---

#### 2. Cumulative Configuration Changes
**Experiments that build on previous changes.**

Example: Phase 2 Refinement Timing
- Experiment 2A: Refinement before selection
- Experiment 2B: Iterative refinement (builds on 2A)

**Why:** 2B only makes sense if 2A succeeds. Running 2B without knowing 2A's outcome is wasteful.

**Mitigation:** Can run 2B speculatively (parallel with 2A), but may discard results if 2A fails.

---

#### 3. Configuration Lock Points
**Phases that establish baseline for next phase.**

```
Phase 1 → Lock quality floor
Phase 2 → Lock refinement timing
Phase 3 → Lock scoring algorithm
Phase 4 → Optimize locked algorithm
```

Each phase must complete and decide before next phase starts.

**Why:** Phase N+1 experiments all use the locked configuration from Phase N.

---

## Parallel Execution Strategies

### Strategy A: Maximum Parallelization (Aggressive)

**Approach:** Run ALL independent experiments simultaneously, analyze later.

**Execution:**
```
Phase 1 (3 agents): 1A, 1B, 1C → Run all in parallel
Phase 2 (6 agents): 2A, 2B × 3 floor values → Run all combinations
Phase 3 (9 agents): 3A, 3B, 3C × 3 floor values → Run all combinations
Phase 4 (varies): Parameter sweeps
Phase 5 (12+ agents): Novel algorithms × locked configs
```

**Pros:**
- Minimum total wall-clock time
- Comprehensive data collection
- Can compare all combinations

**Cons:**
- Many experiments may be discarded
- High computational cost
- Complex result analysis
- Risk of pursuing dead ends

**Timeline:**
- Phase 1: 30 min (3 parallel)
- Phase 2: 30 min (6 parallel, may discard some)
- Phase 3: 30 min (9 parallel, may discard most)
- Analysis: 60 min (compare all results)
- **Total: ~2.5 hours** (vs. 7.5 hours sequential)

---

### Strategy B: Phase-Gated Parallelization (Conservative)

**Approach:** Parallelize within phases, but wait for phase decisions before proceeding.

**Execution:**
```
Phase 1 (3 agents): 1A, 1B, 1C → Run in parallel
  ↓ WAIT: Analyze, select best floor value
Phase 2 (2 agents): 2A, 2B → Run in parallel with locked floor
  ↓ WAIT: Analyze, select best refinement timing
Phase 3 (3 agents): 3A, 3B, 3C → Run in parallel with locked config
  ↓ WAIT: Analyze, select best scoring algorithm
Phase 4 (varies): Parameter sweeps with locked config
Phase 5 (varies): Novel algorithms with locked config
```

**Pros:**
- No wasted experiments
- Clear decision points
- Builds confidence incrementally
- Lower computational cost

**Cons:**
- Slower than Strategy A (more wait points)
- Sequential dependencies limit parallelism

**Timeline:**
- Phase 1: 30 min (3 parallel) + 15 min (analysis) = 45 min
- Phase 2: 30 min (2 parallel) + 15 min (analysis) = 45 min
- Phase 3: 30 min (3 parallel) + 15 min (analysis) = 45 min
- Phase 4: 30 min (parameter sweeps) + 15 min (analysis) = 45 min
- Phase 5: 30 min (novel algorithms) + 15 min (analysis) = 45 min
- **Total: ~3.75 hours** (vs. 7.5 hours sequential)

---

### Strategy C: Hybrid (Recommended)

**Approach:** Parallelize within phases, but speculatively run high-value next-phase experiments.

**Execution:**
```
Phase 1 (3 agents): 1A, 1B, 1C → Run all in parallel
  ↓ CONCURRENTLY (while analyzing Phase 1):
Phase 2 Speculative (3 agents):
  - 2A with floor=0.0 (most likely winner)
  - 2A with floor=-0.3 (backup)
  - 2A with floor=-0.5 (backup)
  ↓ Phase 1 analysis complete → Select best floor
  ↓ Phase 2 speculative runs complete → Use matching result
Phase 3 (3 agents): 3A, 3B, 3C → Run in parallel with locked config
...
```

**Pros:**
- Fast (near-Strategy A speed)
- Lower waste than Strategy A
- Can recover from wrong speculative bets

**Cons:**
- Some wasted experiments if speculation wrong
- More complex orchestration

**Timeline:**
- Phase 1: 30 min (3 parallel) + 0 min (analysis concurrent with Phase 2) = 30 min
- Phase 2: 30 min (3 speculative parallel, overlapped) = 0 additional
- Phase 3: 30 min (3 parallel) + 15 min (analysis) = 45 min
- Phase 4: 30 min + 15 min = 45 min
- Phase 5: 30 min + 15 min = 45 min
- **Total: ~2.5 hours** (Strategy A speed, Strategy B waste reduction)

---

## Agent Assignment Patterns

### Pattern 1: Branch-Per-Agent
**Each agent works on a dedicated git branch.**

```bash
# Agent 1
git checkout -b exp1a-floor-0.0
# Make changes, run test, save results
git add . && git commit -m "Experiment 1A: floor=0.0"

# Agent 2 (parallel, independent branch)
git checkout main
git checkout -b exp1b-floor-neg0.3
# Make changes, run test, save results
git add . && git commit -m "Experiment 1B: floor=-0.3"

# Agent 3 (parallel, independent branch)
git checkout main
git checkout -b exp1c-floor-neg0.5
# Make changes, run test, save results
git add . && git commit -m "Experiment 1C: floor=-0.5"

# Coordinator Agent (after all complete)
git checkout main
# Compare results from am/exp1a_results.json, exp1b_results.json, exp1c_results.json
# Make decision, document in roadmap
```

**Pros:**
- Clean separation
- No merge conflicts
- Easy rollback

**Cons:**
- Must coordinate branch creation

---

### Pattern 2: Workspace-Per-Agent
**Each agent works in a separate workspace copy.**

```bash
# Coordinator sets up workspaces
mkdir /tmp/wkmp-exp1a && cp -r . /tmp/wkmp-exp1a/
mkdir /tmp/wkmp-exp1b && cp -r . /tmp/wkmp-exp1b/
mkdir /tmp/wkmp-exp1c && cp -r . /tmp/wkmp-exp1c/

# Agent 1 → /tmp/wkmp-exp1a
# Agent 2 → /tmp/wkmp-exp1b
# Agent 3 → /tmp/wkmp-exp1c
```

**Pros:**
- No git branch management
- True isolation

**Cons:**
- Disk space (3× codebase)
- Harder to track history

---

### Pattern 3: Result-File-Per-Agent
**All agents work on main branch sequentially, but analysis parallelized.**

```bash
# Run experiments sequentially (no parallel speedup here)
# But parallelize analysis:

# Agent 1: Analyze better/worse/equivocal for albums 1-50
# Agent 2: Analyze albums 51-100
# Agent 3: Analyze albums 101-150
# Agent 4: Analyze albums 151-200

# Coordinator: Merge analysis results
```

**Pros:**
- Simplest coordination
- No git branch complexity

**Cons:**
- Doesn't parallelize experiments (only analysis)
- Limited speedup

---

## Recommended Execution Plan

### Phase 1: Quality Floor (Parallel)

**Agents:** 3 (exp1a, exp1b, exp1c)
**Strategy:** Branch-per-agent
**Execution:**
```
Agent 1 (exp1a): floor = 0.0
Agent 2 (exp1b): floor = -0.3
Agent 3 (exp1c): floor = -0.5

All agents:
1. git checkout -b <exp-branch>
2. Edit scoring.rs, change MIN_QUALITY_FLOOR
3. cargo build --release
4. cargo test --release run29f_full_baseline_comparison -- --nocapture
5. cp wkmp-ai/run29f_comparison_results.json am/<exp>_results.json
6. git add . && git commit -m "Experiment <exp>"
```

**Coordinator:**
1. Wait for all 3 agents to complete
2. Load am/exp1a_results.json, exp1b_results.json, exp1c_results.json
3. Run categorization analysis (better/worse/equivocal) for each
4. Compare metrics
5. Select best configuration
6. Update roadmap with decision

**Time:** 30 min (parallel) + 15 min (analysis) = 45 min

---

### Phase 2: Refinement Timing (Speculative Parallel)

**Agents:** 2-3 (depending on Phase 1 decision)
**Strategy:** Speculative with most-likely Phase 1 winner

**If running speculatively during Phase 1 analysis:**
```
Agent 4 (exp2a-floor0.0): Refinement before selection, floor=0.0
Agent 5 (exp2b-floor0.0): Iterative refinement, floor=0.0

(Backup if needed):
Agent 6 (exp2a-floor-0.3): Refinement before selection, floor=-0.3
```

**If running after Phase 1 decision:**
```
Agent 1 (exp2a): Refinement before selection, locked floor
Agent 2 (exp2b): Iterative refinement, locked floor
```

**Time:** 30 min (parallel, overlapped or sequential)

---

### Phase 3: Scoring Algorithm (Parallel)

**Agents:** 3 (exp3a, exp3b, exp3c)
**Strategy:** Branch-per-agent with locked config from Phases 1-2

```
Agent 1 (exp3a): Pure match % scoring
Agent 2 (exp3b): Hybrid scoring
Agent 3 (exp3c): Weighted scoring with quality capped
```

**Time:** 30 min (parallel) + 15 min (analysis) = 45 min

---

### Phase 4: Parameter Tuning (Parallel)

**Agents:** 4-6 (tolerance values, track count penalties, etc.)
**Strategy:** Parameter sweep

```
Agent 1: tolerance = 8s
Agent 2: tolerance = 12s
Agent 3: tolerance = 15s
Agent 4: track_penalty = 4%
Agent 5: track_penalty = 6%
Agent 6: silence grid = 90 combinations
```

**Time:** 30 min (parallel) + 15 min (analysis) = 45 min

---

### Phase 5: Novel Algorithms (Parallel)

**Agents:** 4-6 (one per novel algorithm)
**Strategy:** Branch-per-agent

```
Agent 1 (exp5a): Confidence-weighted selection
Agent 2 (exp5b): Median quality score
Agent 3 (exp5c): Adaptive tolerance
Agent 4 (exp5d): Two-pass boundary detection
Agent 5 (exp5e): [Additional novel idea]
Agent 6 (exp5f): [Additional novel idea]
```

**Time:** 30 min (parallel) + 15 min (analysis) = 45 min

---

## Total Timeline Comparison

| Strategy | Wall-Clock Time | Compute Time | Wasted Runs |
|----------|----------------|--------------|-------------|
| **Sequential** | 7.5 hours | 7.5 hours | 0% |
| **Conservative (Phase-Gated)** | 3.75 hours | 7.5 hours | 0% |
| **Hybrid (Recommended)** | 2.5 hours | ~9 hours | ~20% |
| **Aggressive (All Parallel)** | 2 hours | ~15 hours | ~50% |

**Recommended:** Hybrid strategy
- Fast (2.5 hours vs. 7.5 hours)
- Low waste (~20%)
- Good confidence building

---

## Agent Coordination Protocol

### 1. Coordinator Agent Responsibilities
- Create experiment branches
- Assign agents to experiments
- Collect results from all agents
- Run comparative analysis
- Make configuration decisions
- Update roadmap
- Archive results

### 2. Worker Agent Responsibilities
- Check out assigned branch
- Make code changes per experiment spec
- Build release binary
- Run 200-file test
- Save results to designated file
- Commit changes with experiment metadata
- Report completion to coordinator

### 3. Communication Protocol
```json
// Coordinator → Worker
{
  "agent_id": "worker-1",
  "experiment": "exp1a",
  "branch": "exp1a-floor-0.0",
  "changes": [
    {
      "file": "wkmp-ai/src/matching/editions/scoring.rs",
      "line": 339,
      "old": "const MIN_QUALITY_FLOOR: f64 = -1.0;",
      "new": "const MIN_QUALITY_FLOOR: f64 = 0.0;"
    }
  ],
  "result_file": "am/exp1a_results.json"
}

// Worker → Coordinator
{
  "agent_id": "worker-1",
  "experiment": "exp1a",
  "status": "complete",
  "result_file": "am/exp1a_results.json",
  "metrics": {
    "exact_matches": 68,
    "mbid_changes": 125,
    "runtime_seconds": 1874
  }
}
```

---

## Risk Mitigation

### 1. Result Consistency
**Risk:** Different agents get different results for same experiment.
**Mitigation:**
- Use same release profile (`--release`)
- Same test command
- Same baseline file (run29f_albums.txt)
- Verify reproducibility before parallel runs

### 2. Disk Space
**Risk:** Running multiple builds in parallel fills disk.
**Mitigation:**
- Use git branches (share target/ directory)
- Clean builds between experiments if needed
- Monitor disk space

### 3. Resource Contention
**Risk:** Too many parallel builds slow system.
**Mitigation:**
- Limit to 3-4 parallel agents
- Stagger test runs if needed
- Monitor system load

### 4. Branch Management
**Risk:** Git branch proliferation, merge conflicts.
**Mitigation:**
- Use ephemeral branches (delete after results collected)
- Never merge experiment branches to main
- Keep experiments isolated

---

## Implementation Steps

### Step 1: Validate Reproducibility
```bash
# Run same experiment twice, verify identical results
cargo test --release run29f_full_baseline_comparison -- --nocapture > run1.txt
cargo test --release run29f_full_baseline_comparison -- --nocapture > run2.txt
diff run1.txt run2.txt  # Should be identical
```

### Step 2: Create Experiment Template
```bash
# Template script for worker agents
#!/bin/bash
EXPERIMENT=$1
BRANCH=$2
CHANGES=$3  # JSON file with changes

git checkout -b $BRANCH
# Apply changes from JSON
cargo build --release
cargo test --release run29f_full_baseline_comparison -- --nocapture
cp wkmp-ai/run29f_comparison_results.json am/${EXPERIMENT}_results.json
git add .
git commit -m "Experiment $EXPERIMENT"
echo "COMPLETE: $EXPERIMENT"
```

### Step 3: Launch Phase 1 (Parallel Test)
```bash
# Coordinator launches 3 agents
./run_experiment.sh exp1a exp1a-floor-0.0 exp1a_changes.json &
./run_experiment.sh exp1b exp1b-floor-neg0.3 exp1b_changes.json &
./run_experiment.sh exp1c exp1c-floor-neg0.5 exp1c_changes.json &
wait  # Wait for all to complete
# Analyze results
```

---

## Success Metrics

**Parallelization is successful if:**
1. Wall-clock time reduced by ≥50% (7.5h → <4h)
2. All experiments produce valid results
3. Results are reproducible (same experiment = same results)
4. Coordinator can make clear decisions from parallel results
5. No merge conflicts or git issues

---

## Conclusion

**Parallelization Potential: HIGH**
- Phase 1-5 can all be parallelized within phase
- Hybrid strategy recommended: 2.5 hours vs. 7.5 hours (67% time savings)
- 3-6 agents working concurrently (optimal)
- Git branches provide clean isolation
- Coordinator agent makes decisions between phases

**Next Steps:**
1. Validate reproducibility (same experiment → same results)
2. Implement experiment runner script
3. Launch Phase 1 with 3 parallel agents
4. Collect and analyze results
5. Proceed to Phase 2 with locked configuration
