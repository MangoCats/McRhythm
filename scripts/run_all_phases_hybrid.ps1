# Hybrid Experiment Orchestrator - All Phases
# Runs experiments with hybrid parallelization strategy

param(
    [Parameter(Mandatory=$false)]
    [ValidateSet("Phase1", "Phase2", "Phase3", "Phase4", "Phase5", "All")]
    [string]$StartPhase = "All",

    [Parameter(Mandatory=$false)]
    [switch]$DryRun = $false
)

$ErrorActionPreference = "Stop"

# Setup
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Split-Path -Parent $ScriptDir
Set-Location $ProjectRoot

$LogFile = "logs\hybrid_orchestrator_$(Get-Date -Format 'yyyyMMdd_HHmmss').log"
New-Item -ItemType Directory -Force -Path "logs" | Out-Null

function Write-Log {
    param([string]$Message, [string]$Level = "INFO")
    $Timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    $LogMessage = "[$Timestamp] [$Level] $Message"
    $Color = switch ($Level) {
        "ERROR" { "Red" }
        "WARN" { "Yellow" }
        "SUCCESS" { "Green" }
        default { "White" }
    }
    Write-Host $LogMessage -ForegroundColor $Color
    Add-Content -Path $LogFile -Value $LogMessage
}

Write-Log "=== Hybrid Experiment Orchestrator ===" "INFO"
Write-Log "Starting phase: $StartPhase" "INFO"
Write-Log "Dry run: $DryRun" "INFO"

# Track decisions and configuration
$Configuration = @{
    QualityFloor = -1.0  # Current value
    RefinementTiming = "Stage2+4"  # Current value
    ScoringAlgorithm = "Multi-factor"  # Current value
    Tolerance = 10.0  # Current value
}

$PhaseResults = @()

# ============================================================================
# Phase 1: Quality Floor Experiments
# ============================================================================
if ($StartPhase -eq "All" -or $StartPhase -eq "Phase1") {
    Write-Log "" "INFO"
    Write-Log "========================================" "INFO"
    Write-Log "Phase 1: Quality Floor Experiments" "INFO"
    Write-Log "========================================" "INFO"

    if (-not $DryRun) {
        & "$ScriptDir\run_phase1_parallel.ps1"

        # Load decision
        $Phase1Decision = Get-Content -Path "am\phase1_decision.json" -Raw | ConvertFrom-Json

        Write-Log "" "INFO"
        Write-Log "Phase 1 Decision: $($Phase1Decision.WinnerName)" "SUCCESS"
        Write-Log "  Regressions: $($Phase1Decision.Regressions)" "INFO"
        Write-Log "  Improvements: $($Phase1Decision.Improvements)" "INFO"

        # Update configuration based on winner
        switch ($Phase1Decision.Winner) {
            "exp1a" { $Configuration.QualityFloor = 0.0 }
            "exp1b" { $Configuration.QualityFloor = -0.3 }
            "exp1c" { $Configuration.QualityFloor = -0.5 }
        }

        $PhaseResults += $Phase1Decision

        # Check success criteria
        if ($Phase1Decision.Regressions -gt 2) {
            Write-Log "⚠ Phase 1 did not meet success criteria (>2 regressions)" "WARN"
            Write-Log "Consider pivoting to Phase 3A (run29f-style scoring)" "WARN"

            $Response = Read-Host "Continue to Phase 2? (y/n)"
            if ($Response -ne "y") {
                Write-Log "Stopping execution" "INFO"
                exit 0
            }
        }
    }
    else {
        Write-Log "[DRY RUN] Would execute Phase 1" "INFO"
    }
}

# ============================================================================
# Phase 2: Boundary Refinement Timing
# ============================================================================
if ($StartPhase -eq "All" -or $StartPhase -eq "Phase2") {
    Write-Log "" "INFO"
    Write-Log "========================================" "INFO"
    Write-Log "Phase 2: Refinement Timing" "INFO"
    Write-Log "========================================" "INFO"
    Write-Log "Locked configuration: Quality floor = $($Configuration.QualityFloor)" "INFO"

    if (-not $DryRun) {
        # Speculative: Already run during Phase 1 analysis if using hybrid strategy
        # Check if results exist
        $Exp2aExists = Test-Path "am\exp2a_results.json"

        if (-not $Exp2aExists) {
            Write-Log "Running Phase 2 experiments..." "INFO"

            # Define Phase 2 experiments
            $Phase2Experiments = @(
                @{
                    Id = "exp2a"
                    Name = "Refinement Before Scoring Recalculation"
                    Branch = "exp2a-refine-before-score"
                    Description = "Move refinement to before match % recalculation in Stage 2"
                },
                @{
                    Id = "exp2b"
                    Name = "Iterative Refinement in Parameter Loop"
                    Branch = "exp2b-iterative-refine"
                    Description = "Apply refinement inside parameter grid search loop"
                }
            )

            # Note: Actual code changes would need to be defined here
            # For now, placeholder

            Write-Log "Phase 2 experiments defined but require manual implementation" "WARN"
            Write-Log "  - Experiment 2A: Move refinement timing in stage2.rs" "INFO"
            Write-Log "  - Experiment 2B: Iterative refinement in parameter loop" "INFO"
        }
        else {
            Write-Log "Phase 2 results found (speculative run from Phase 1)" "INFO"
        }

        # Analyze and select winner
        # (Simplified for now - would need full analysis)
        Write-Log "Phase 2 analysis would go here" "INFO"
    }
    else {
        Write-Log "[DRY RUN] Would execute Phase 2" "INFO"
    }
}

# ============================================================================
# Phase 3: Scoring Algorithm
# ============================================================================
if ($StartPhase -eq "All" -or $StartPhase -eq "Phase3") {
    Write-Log "" "INFO"
    Write-Log "========================================" "INFO"
    Write-Log "Phase 3: Scoring Algorithm" "INFO"
    Write-Log "========================================" "INFO"
    Write-Log "Locked configuration:" "INFO"
    Write-Log "  Quality floor: $($Configuration.QualityFloor)" "INFO"
    Write-Log "  Refinement: $($Configuration.RefinementTiming)" "INFO"

    if (-not $DryRun) {
        Write-Log "Phase 3 experiments require significant code changes" "WARN"
        Write-Log "  - Experiment 3A: Pure match % (run29f style)" "INFO"
        Write-Log "  - Experiment 3B: Hybrid scoring" "INFO"
        Write-Log "  - Experiment 3C: Weighted scoring with quality capped" "INFO"
        Write-Log "" "INFO"
        Write-Log "Recommend running these manually due to complexity" "INFO"
    }
    else {
        Write-Log "[DRY RUN] Would execute Phase 3" "INFO"
    }
}

# ============================================================================
# Phase 4: Parameter Tuning
# ============================================================================
if ($StartPhase -eq "All" -or $StartPhase -eq "Phase4") {
    Write-Log "" "INFO"
    Write-Log "========================================" "INFO"
    Write-Log "Phase 4: Parameter Tuning" "INFO"
    Write-Log "========================================" "INFO"
    Write-Log "Locked configuration:" "INFO"
    Write-Log "  Quality floor: $($Configuration.QualityFloor)" "INFO"
    Write-Log "  Refinement: $($Configuration.RefinementTiming)" "INFO"
    Write-Log "  Scoring: $($Configuration.ScoringAlgorithm)" "INFO"

    if (-not $DryRun) {
        Write-Log "Phase 4: Parameter sweep experiments" "INFO"
        Write-Log "  - Tolerance values: 8s, 12s, 15s" "INFO"
        Write-Log "  - Track count penalties: 4%, 6%" "INFO"
        Write-Log "  - Silence grid optimization" "INFO"
    }
    else {
        Write-Log "[DRY RUN] Would execute Phase 4" "INFO"
    }
}

# ============================================================================
# Phase 5: Novel Algorithms
# ============================================================================
if ($StartPhase -eq "All" -or $StartPhase -eq "Phase5") {
    Write-Log "" "INFO"
    Write-Log "========================================" "INFO"
    Write-Log "Phase 5: Novel Algorithms" "INFO"
    Write-Log "========================================" "INFO"
    Write-Log "Locked configuration:" "INFO"
    Write-Log "  Quality floor: $($Configuration.QualityFloor)" "INFO"
    Write-Log "  Refinement: $($Configuration.RefinementTiming)" "INFO"
    Write-Log "  Scoring: $($Configuration.ScoringAlgorithm)" "INFO"
    Write-Log "  Tolerance: $($Configuration.Tolerance)" "INFO"

    if (-not $DryRun) {
        Write-Log "Phase 5: Exploratory novel algorithms" "INFO"
        Write-Log "  - Confidence-weighted selection" "INFO"
        Write-Log "  - Median quality score" "INFO"
        Write-Log "  - Adaptive tolerance per track" "INFO"
        Write-Log "  - Two-pass boundary detection" "INFO"
    }
    else {
        Write-Log "[DRY RUN] Would execute Phase 5" "INFO"
    }
}

# ============================================================================
# Final Summary
# ============================================================================
Write-Log "" "INFO"
Write-Log "========================================" "INFO"
Write-Log "Orchestrator Complete" "INFO"
Write-Log "========================================" "INFO"

Write-Log "Final Configuration:" "INFO"
Write-Log "  Quality floor: $($Configuration.QualityFloor)" "INFO"
Write-Log "  Refinement timing: $($Configuration.RefinementTiming)" "INFO"
Write-Log "  Scoring algorithm: $($Configuration.ScoringAlgorithm)" "INFO"
Write-Log "  Tolerance: $($Configuration.Tolerance)" "INFO"

Write-Log "" "INFO"
Write-Log "Phase Results:" "INFO"
foreach ($Result in $PhaseResults) {
    Write-Log "  $($Result.Phase): $($Result.WinnerName)" "INFO"
    Write-Log "    Regressions: $($Result.Regressions), Improvements: $($Result.Improvements)" "INFO"
}

Write-Log "" "INFO"
Write-Log "Log file: $LogFile" "INFO"
