# Phase 1 Parallel Coordinator - Quality Floor Experiments
# Runs Experiments 1A, 1B, 1C in parallel and compares results

param(
    [Parameter(Mandatory=$false)]
    [switch]$SkipTests = $false,

    [Parameter(Mandatory=$false)]
    [switch]$AnalyzeOnly = $false
)

$ErrorActionPreference = "Stop"

# Setup
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Split-Path -Parent $ScriptDir
Set-Location $ProjectRoot

$LogFile = "logs\phase1_coordinator_$(Get-Date -Format 'yyyyMMdd_HHmmss').log"
New-Item -ItemType Directory -Force -Path "logs" | Out-Null

function Write-Log {
    param([string]$Message, [string]$Level = "INFO")
    $Timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    $LogMessage = "[$Timestamp] [$Level] $Message"
    Write-Host $LogMessage -ForegroundColor $(if ($Level -eq "ERROR") { "Red" } elseif ($Level -eq "WARN") { "Yellow" } else { "White" })
    Add-Content -Path $LogFile -Value $LogMessage
}

Write-Log "=== Phase 1: Quality Floor Parallel Experiments ===" "INFO"
Write-Log "Project root: $ProjectRoot" "INFO"

# Define experiments
$Experiments = @(
    @{
        Id = "exp1a"
        Name = "No Negative Floor (0.0)"
        Branch = "exp1a-floor-0.0"
        Changes = @{
            "wkmp-ai\src\matching\editions\scoring.rs" = @(
                @{
                    Old = "const MIN_QUALITY_FLOOR: f64 = -1.0;    // Negative penalty for catastrophic failures"
                    New = "const MIN_QUALITY_FLOOR: f64 = 0.0;     // No negative floor - errors beyond tolerance contribute 0.0"
                }
            )
        }
    },
    @{
        Id = "exp1b"
        Name = "Reduced Negative Floor (-0.3)"
        Branch = "exp1b-floor-neg0.3"
        Changes = @{
            "wkmp-ai\src\matching\editions\scoring.rs" = @(
                @{
                    Old = "const MIN_QUALITY_FLOOR: f64 = -1.0;    // Negative penalty for catastrophic failures"
                    New = "const MIN_QUALITY_FLOOR: f64 = -0.3;    // Reduced negative floor - softer penalty"
                }
            )
        }
    },
    @{
        Id = "exp1c"
        Name = "Middle Ground Floor (-0.5)"
        Branch = "exp1c-floor-neg0.5"
        Changes = @{
            "wkmp-ai\src\matching\editions\scoring.rs" = @(
                @{
                    Old = "const MIN_QUALITY_FLOOR: f64 = -1.0;    // Negative penalty for catastrophic failures"
                    New = "const MIN_QUALITY_FLOOR: f64 = -0.5;    // Middle ground negative floor"
                }
            )
        }
    }
)

# Run experiments in parallel
if (-not $AnalyzeOnly) {
    if (-not $SkipTests) {
        Write-Log "Starting parallel experiment execution..." "INFO"
        Write-Log "Experiments to run: $($Experiments.Count)" "INFO"

        $Jobs = @()
        $StaggerDelay = 90  # Seconds between launching experiments to avoid cargo lock

        foreach ($Exp in $Experiments) {
            Write-Log "Launching experiment: $($Exp.Id) - $($Exp.Name)" "INFO"

            $Job = Start-Job -ScriptBlock {
                param($ProjectRoot, $ScriptDir, $Exp)

                Set-Location $ProjectRoot
                $Result = & "$ScriptDir\run_experiment.ps1" `
                    -ExperimentId $Exp.Id `
                    -BranchName $Exp.Branch `
                    -Changes $Exp.Changes `
                    -CleanupBranch:$false

                return $Result
            } -ArgumentList $ProjectRoot, $ScriptDir, $Exp

            $Jobs += @{
                Job = $Job
                ExperimentId = $Exp.Id
                Name = $Exp.Name
            }

            # Stagger launches to avoid cargo lock contention
            if ($Jobs.Count -lt $Experiments.Count) {
                Write-Log "Waiting $StaggerDelay seconds before launching next experiment..." "INFO"
                Start-Sleep -Seconds $StaggerDelay
            }
        }

        Write-Log "All experiments launched. Waiting for completion..." "INFO"

        # Monitor jobs
        $CompletedCount = 0
        while ($CompletedCount -lt $Jobs.Count) {
            Start-Sleep -Seconds 10

            $CompletedCount = ($Jobs | Where-Object { $_.Job.State -eq "Completed" -or $_.Job.State -eq "Failed" }).Count
            $RunningCount = ($Jobs | Where-Object { $_.Job.State -eq "Running" }).Count

            Write-Log "Progress: $CompletedCount/$($Jobs.Count) completed, $RunningCount running" "INFO"
        }

        # Collect results
        Write-Log "All experiments completed. Collecting results..." "INFO"
        $ExperimentResults = @()

        foreach ($JobInfo in $Jobs) {
            $Job = $JobInfo.Job
            $ExpId = $JobInfo.ExperimentId
            $ExpName = $JobInfo.Name

            if ($Job.State -eq "Completed") {
                $Result = Receive-Job -Job $Job
                $ExperimentResults += @{
                    ExperimentId = $ExpId
                    Name = $ExpName
                    Status = $Result.Status
                    ExactMatches = $Result.ExactMatches
                    MbidChanges = $Result.MbidChanges
                    Duration = $Result.Duration
                    ResultsFile = $Result.ResultsFile
                }
                Write-Log "[OK] $ExpId ($ExpName): $($Result.Status)" "INFO"
            }
            else {
                $Error = Receive-Job -Job $Job
                $ExperimentResults += @{
                    ExperimentId = $ExpId
                    Name = $ExpName
                    Status = "FAILED"
                    Error = $Error
                }
                Write-Log "[FAIL] $ExpId ($ExpName): FAILED - $Error" "ERROR"
            }

            Remove-Job -Job $Job
        }

        # Save summary
        $ExperimentResults | ConvertTo-Json -Depth 10 | Set-Content -Path "am\phase1_experiment_summary.json"
        Write-Log "Experiment summary saved to am\phase1_experiment_summary.json" "INFO"
    }
    else {
        Write-Log "Skipping test execution (results expected to exist)" "INFO"
    }
}

# Analyze results
Write-Log "=== Analyzing Results ===" "INFO"

# Load baseline for comparison
$BaselinePath = "am\run29f_comparison_results_with_stage4_only_refinement.json"
if (-not (Test-Path $BaselinePath)) {
    Write-Log "ERROR: Baseline file not found: $BaselinePath" "ERROR"
    exit 1
}

Write-Log "Loading baseline from: $BaselinePath" "INFO"
$Baseline = Get-Content -Path $BaselinePath -Raw | ConvertFrom-Json

# Analyze each experiment
Write-Log "Comparing experiments to baseline..." "INFO"

$AnalysisScript = "$ScriptDir\analyze_experiment.ps1"
$ComparisonResults = @()

foreach ($Exp in $Experiments) {
    $ResultsFile = "am\$($Exp.Id)_results.json"

    if (-not (Test-Path $ResultsFile)) {
        Write-Log "WARNING: Results file not found for $($Exp.Id): $ResultsFile" "WARN"
        continue
    }

    Write-Log "Analyzing $($Exp.Id)..." "INFO"

    # Run analysis script
    $Analysis = & $AnalysisScript -ExperimentFile $ResultsFile -BaselineFile $BaselinePath

    $ComparisonResults += @{
        ExperimentId = $Exp.Id
        Name = $Exp.Name
        Analysis = $Analysis
    }

    Write-Log "  Exact matches: $($Analysis.ExactMatches)" "INFO"
    Write-Log "  MBID changes: $($Analysis.MbidChanges)" "INFO"
    Write-Log "  Clearly better: $($Analysis.ClearlyBetter)" "INFO"
    Write-Log "  Equivocal: $($Analysis.Equivocal)" "INFO"
    Write-Log "  Clearly worse: $($Analysis.ClearlyWorse)" "INFO"
}

# Compare experiments and select winner
Write-Log "=== Phase 1 Decision ===" "INFO"

$BestExperiment = $null
$MinRegressions = 999
$MaxImprovements = 0

foreach ($Result in $ComparisonResults) {
    $Regressions = $Result.Analysis.ClearlyWorse
    $Improvements = $Result.Analysis.ClearlyBetter

    Write-Log "$($Result.ExperimentId) ($($Result.Name)):" "INFO"
    Write-Log "  Regressions: $Regressions, Improvements: $Improvements" "INFO"

    # Select based on: fewest regressions, then most improvements
    $IsBetter = $false
    if ($Regressions -lt $MinRegressions) {
        $IsBetter = $true
    }
    elseif ($Regressions -eq $MinRegressions -and $Improvements -gt $MaxImprovements) {
        $IsBetter = $true
    }

    if ($IsBetter) {
        $MinRegressions = $Regressions
        $MaxImprovements = $Improvements
        $BestExperiment = $Result
    }
}

if ($null -ne $BestExperiment) {
    Write-Log "" "INFO"
    Write-Log "WINNER: $($BestExperiment.ExperimentId) - $($BestExperiment.Name)" "INFO"
    Write-Log "  Regressions: $MinRegressions (target: 0)" "INFO"
    Write-Log "  Improvements: $MaxImprovements (baseline: 81)" "INFO"

    # Save decision
    $Decision = @{
        Phase = "Phase 1: Quality Floor"
        Winner = $BestExperiment.ExperimentId
        WinnerName = $BestExperiment.Name
        Regressions = $MinRegressions
        Improvements = $MaxImprovements
        Timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    }

    $Decision | ConvertTo-Json -Depth 10 | Set-Content -Path "am\phase1_decision.json"
    Write-Log "Decision saved to am\phase1_decision.json" "INFO"

    # Update roadmap
    Write-Log "" "INFO"
    Write-Log "Next steps:" "INFO"
    if ($MinRegressions -eq 0) {
        Write-Log "SUCCESS: Zero regressions achieved!" "INFO"
        Write-Log "NEXT: Proceed to Phase 2 (Refinement Timing) with locked floor: $($BestExperiment.Name)" "INFO"
    }
    elseif ($MinRegressions -le 2) {
        Write-Log "PARTIAL SUCCESS: $MinRegressions regressions remaining" "WARN"
        Write-Log "NEXT: Consider proceeding to Phase 2 or investigating remaining regressions" "INFO"
    }
    else {
        Write-Log "INSUFFICIENT IMPROVEMENT: $MinRegressions regressions" "ERROR"
        Write-Log "NEXT: Consider Phase 3A (revert to run29f scoring) instead" "INFO"
    }
}
else {
    Write-Log "ERROR: No valid experiment results to compare" "ERROR"
    exit 1
}

Write-Log "=== Phase 1 Complete ===" "INFO"
Write-Log "Log file: $LogFile" "INFO"
