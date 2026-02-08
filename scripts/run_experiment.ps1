# Experiment Runner - Single Experiment Execution
# Usage: .\run_experiment.ps1 -ExperimentId "exp1a" -BranchName "exp1a-floor-0.0" -Changes @{...}

param(
    [Parameter(Mandatory=$true)]
    [string]$ExperimentId,

    [Parameter(Mandatory=$true)]
    [string]$BranchName,

    [Parameter(Mandatory=$true)]
    [hashtable]$Changes,

    [Parameter(Mandatory=$false)]
    [string]$ResultsDir = "am",

    [Parameter(Mandatory=$false)]
    [switch]$SkipBuild = $false,

    [Parameter(Mandatory=$false)]
    [switch]$CleanupBranch = $true
)

$ErrorActionPreference = "Stop"

# Setup logging
$LogFile = "logs\${ExperimentId}_$(Get-Date -Format 'yyyyMMdd_HHmmss').log"
New-Item -ItemType Directory -Force -Path "logs" | Out-Null

function Write-Log {
    param([string]$Message, [string]$Level = "INFO")
    $Timestamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
    $LogMessage = "[$Timestamp] [$Level] $Message"
    Write-Host $LogMessage
    Add-Content -Path $LogFile -Value $LogMessage
}

function Invoke-SafeCommand {
    param(
        [Parameter(Mandatory=$true)]
        [scriptblock]$Command,
        [string]$ErrorMessage = "Command failed"
    )

    try {
        & $Command
        if ($LASTEXITCODE -ne 0 -and $LASTEXITCODE -ne $null) {
            throw "$ErrorMessage (Exit code: $LASTEXITCODE)"
        }
    }
    catch {
        Write-Log "ERROR: $ErrorMessage - $_" "ERROR"
        throw
    }
}

# Main execution
try {
    Write-Log "=== Starting Experiment: $ExperimentId ===" "INFO"
    Write-Log "Branch: $BranchName" "INFO"

    # Step 0: Ensure we're in the project root
    $ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
    $ProjectRoot = Split-Path -Parent $ScriptDir
    Set-Location $ProjectRoot
    Write-Log "Working directory: $(Get-Location)" "INFO"

    # Step 1: Store current branch
    $OriginalBranch = git branch --show-current
    Write-Log "Current branch: $OriginalBranch" "INFO"

    # Step 2: Create and checkout experiment branch
    Write-Log "Creating branch: $BranchName" "INFO"
    $prevErrorPref = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    git checkout -b $BranchName 2>&1 | Out-Null
    $ErrorActionPreference = $prevErrorPref
    if ($LASTEXITCODE -ne 0) {
        Write-Log "ERROR: Failed to create branch $BranchName" "ERROR"
        throw "Failed to create branch $BranchName"
    }

    # Step 3: Apply code changes
    Write-Log "Applying code changes..." "INFO"
    foreach ($FilePath in $Changes.Keys) {
        $FileChanges = $Changes[$FilePath]
        Write-Log "  Modifying: $FilePath" "INFO"

        $Content = Get-Content -Path $FilePath -Raw

        foreach ($Change in $FileChanges) {
            $OldText = $Change.Old
            $NewText = $Change.New

            if ($Content -notmatch [regex]::Escape($OldText)) {
                Write-Log "WARNING: Old text not found in $FilePath" "WARN"
                Write-Log "  Looking for: $OldText" "WARN"
            }

            $Content = $Content -replace [regex]::Escape($OldText), $NewText
            Write-Log "    Changed: '$OldText' -> '$NewText'" "INFO"
        }

        Set-Content -Path $FilePath -Value $Content -NoNewline
    }

    # Step 4: Build release binary
    if (-not $SkipBuild) {
        Write-Log "Building release binary..." "INFO"
        $BuildStart = Get-Date

        # Cargo outputs to stderr during normal compilation, so don't use Invoke-SafeCommand
        $prevErrorPref = $ErrorActionPreference
        $ErrorActionPreference = "Continue"
        cargo build --release 2>&1 | Tee-Object -FilePath "logs\${ExperimentId}_build.log" | Out-Null
        $ErrorActionPreference = $prevErrorPref

        if ($LASTEXITCODE -ne 0) {
            Write-Log "ERROR: Build failed with exit code $LASTEXITCODE" "ERROR"
            throw "Build failed"
        }

        $BuildDuration = (Get-Date) - $BuildStart
        Write-Log "Build completed in $($BuildDuration.TotalMinutes.ToString('F2')) minutes" "INFO"
    }
    else {
        Write-Log "Skipping build (using existing binary)" "INFO"
    }

    # Step 5: Run 200-file comparison test
    Write-Log "Running 200-file comparison test..." "INFO"
    $TestStart = Get-Date

    # Cargo test also outputs to stderr, so don't use Invoke-SafeCommand
    $prevErrorPref = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    cargo test --release run29f_full_baseline_comparison -- --nocapture 2>&1 | Tee-Object -FilePath "logs\${ExperimentId}_test.log" | Out-Null
    $ErrorActionPreference = $prevErrorPref

    if ($LASTEXITCODE -ne 0) {
        Write-Log "ERROR: Test failed with exit code $LASTEXITCODE" "ERROR"
        throw "Test execution failed"
    }

    $TestDuration = (Get-Date) - $TestStart
    Write-Log "Test completed in $($TestDuration.TotalMinutes.ToString('F2')) minutes" "INFO"

    # Step 6: Copy results to designated location
    $SourceFile = "wkmp-ai\run29f_comparison_results.json"
    $DestFile = "$ResultsDir\${ExperimentId}_results.json"

    if (Test-Path $SourceFile) {
        Write-Log "Copying results to $DestFile" "INFO"
        Copy-Item -Path $SourceFile -Destination $DestFile -Force
    }
    else {
        throw "Results file not found: $SourceFile"
    }

    # Step 7: Extract quick metrics
    $Results = Get-Content -Path $DestFile -Raw | ConvertFrom-Json
    $TotalAlbums = $Results.Count
    $ExactMatches = ($Results | Where-Object { $_.matched -eq $true -and $_.mbid_match -eq $true }).Count
    $MbidChanges = ($Results | Where-Object { $_.mbid_match -eq $false }).Count

    Write-Log "Quick Metrics:" "INFO"
    Write-Log "  Total albums: $TotalAlbums" "INFO"
    Write-Log "  Exact matches: $ExactMatches ($([math]::Round($ExactMatches/$TotalAlbums*100, 2))%)" "INFO"
    Write-Log "  MBID changes: $MbidChanges ($([math]::Round($MbidChanges/$TotalAlbums*100, 2))%)" "INFO"

    # Step 8: Commit changes
    Write-Log "Committing experiment changes..." "INFO"
    git add . 2>&1 | Out-Null
    git commit -m "Experiment $ExperimentId - $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')" 2>&1 | Out-Null
    if ($LASTEXITCODE -ne 0) {
        Write-Log "WARNING: Git commit may have failed, but continuing..." "WARN"
    }

    # Step 9: Return to original branch
    Write-Log "Returning to original branch: $OriginalBranch" "INFO"
    $prevErrorPref = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    git checkout $OriginalBranch 2>&1 | Out-Null
    $ErrorActionPreference = $prevErrorPref
    if ($LASTEXITCODE -ne 0) {
        Write-Log "ERROR: Failed to return to original branch" "ERROR"
        throw "Failed to checkout original branch"
    }

    # Step 10: Optionally cleanup experiment branch
    if ($CleanupBranch) {
        Write-Log "Deleting experiment branch: $BranchName" "INFO"
        $prevErrorPref = $ErrorActionPreference
        $ErrorActionPreference = "Continue"
        git branch -D $BranchName 2>&1 | Out-Null
        $ErrorActionPreference = $prevErrorPref
        if ($LASTEXITCODE -ne 0) {
            Write-Log "WARNING: Failed to delete branch $BranchName" "WARN"
        }
    }

    # Output completion status
    $TotalDuration = (Get-Date) - $TestStart
    Write-Log "=== Experiment Complete: $ExperimentId ===" "INFO"
    Write-Log "Total duration: $($TotalDuration.TotalMinutes.ToString('F2')) minutes" "INFO"
    Write-Log "Results saved to: $DestFile" "INFO"

    # Return metrics for coordinator
    return @{
        ExperimentId = $ExperimentId
        Status = "SUCCESS"
        ResultsFile = $DestFile
        ExactMatches = $ExactMatches
        MbidChanges = $MbidChanges
        Duration = $TotalDuration.TotalMinutes
    }
}
catch {
    Write-Log "FATAL ERROR: $_" "ERROR"
    Write-Log "Stack trace: $($_.ScriptStackTrace)" "ERROR"

    # Attempt to return to original branch
    try {
        git checkout $OriginalBranch 2>&1 | Out-Null
    }
    catch {
        Write-Log "Failed to return to original branch" "ERROR"
    }

    return @{
        ExperimentId = $ExperimentId
        Status = "FAILED"
        Error = $_.Exception.Message
    }
}
