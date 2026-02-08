# Validate Reproducibility - Ensure tests produce identical results
# Run before parallel experiments to confirm deterministic behavior

param(
    [Parameter(Mandatory=$false)]
    [int]$Iterations = 2,

    [Parameter(Mandatory=$false)]
    [switch]$FullTest = $false
)

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectRoot = Split-Path -Parent $ScriptDir
Set-Location $ProjectRoot

Write-Host "=== Reproducibility Validation ===" -ForegroundColor Cyan
Write-Host "Iterations: $Iterations" -ForegroundColor White
Write-Host "Full test: $FullTest" -ForegroundColor White
Write-Host ""

# Create temp directory for results
$TempDir = "temp_validation"
New-Item -ItemType Directory -Force -Path $TempDir | Out-Null

$ResultFiles = @()
$Checksums = @()

# Run test multiple times
for ($i = 1; $i -le $Iterations; $i++) {
    Write-Host "[$i/$Iterations] Running test..." -ForegroundColor Yellow

    if ($FullTest) {
        # Full 200-album test
        $StartTime = Get-Date
        cargo test --release run29f_full_baseline_comparison -- --nocapture 2>&1 | Out-Null
        $Duration = (Get-Date) - $StartTime

        Write-Host "  Completed in $($Duration.TotalMinutes.ToString('F2')) minutes" -ForegroundColor Green
    }
    else {
        # Quick smoke test (first 10 albums)
        Write-Host "  Running quick smoke test (use -FullTest for 200 albums)" -ForegroundColor Gray
        cargo test --release -- --nocapture 2>&1 | Select-String -Pattern "Album \d+" -CaseSensitive | Select-Object -First 10 | Out-Null
    }

    # Copy results
    $ResultFile = "$TempDir\results_run$i.json"
    if (Test-Path "wkmp-ai\run29f_comparison_results.json") {
        Copy-Item -Path "wkmp-ai\run29f_comparison_results.json" -Destination $ResultFile -Force
        $ResultFiles += $ResultFile

        # Calculate checksum
        $Hash = Get-FileHash -Path $ResultFile -Algorithm SHA256
        $Checksums += $Hash.Hash

        Write-Host "  Saved to: $ResultFile" -ForegroundColor Green
        Write-Host "  Checksum: $($Hash.Hash.Substring(0, 16))..." -ForegroundColor Gray
    }
    else {
        Write-Host "  ERROR: Results file not found!" -ForegroundColor Red
        exit 1
    }

    Write-Host ""
}

# Compare checksums
Write-Host "=== Reproducibility Analysis ===" -ForegroundColor Cyan

$AllIdentical = $true
$ReferenceChecksum = $Checksums[0]

for ($i = 1; $i -lt $Checksums.Count; $i++) {
    if ($Checksums[$i] -ne $ReferenceChecksum) {
        $AllIdentical = $false
        Write-Host "[X] Checksum mismatch: Run 1 vs Run $($i+1)" -ForegroundColor Red
        Write-Host "  Run 1: $ReferenceChecksum" -ForegroundColor Gray
        Write-Host "  Run $($i+1): $($Checksums[$i])" -ForegroundColor Gray
    }
}

if ($AllIdentical) {
    Write-Host "[OK] All $Iterations runs produced identical results!" -ForegroundColor Green
    Write-Host "  Checksum: $ReferenceChecksum" -ForegroundColor Gray
    Write-Host ""
    Write-Host "SUCCESS: Reproducibility VALIDATED - Safe to run parallel experiments" -ForegroundColor Green

    # Cleanup temp directory
    Remove-Item -Path $TempDir -Recurse -Force
    Write-Host "Temp files cleaned up" -ForegroundColor Gray
}
else {
    Write-Host ""
    Write-Host "[WARN] Non-deterministic behavior detected!" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "Possible causes:" -ForegroundColor White
    Write-Host "  1. Random number generation without fixed seed" -ForegroundColor Gray
    Write-Host "  2. Timestamp-based values in results" -ForegroundColor Gray
    Write-Host "  3. Non-deterministic HashMap iteration order" -ForegroundColor Gray
    Write-Host "  4. Parallel processing with race conditions" -ForegroundColor Gray
    Write-Host ""
    Write-Host "Investigate before running parallel experiments!" -ForegroundColor Yellow
    Write-Host "Result files saved in: $TempDir" -ForegroundColor White

    # Detailed diff
    if ($Iterations -eq 2) {
        Write-Host ""
        Write-Host "Comparing results in detail..." -ForegroundColor Yellow

        $Results1 = Get-Content -Path $ResultFiles[0] -Raw | ConvertFrom-Json
        $Results2 = Get-Content -Path $ResultFiles[1] -Raw | ConvertFrom-Json

        $Differences = 0
        for ($i = 0; $i -lt [Math]::Min($Results1.Count, $Results2.Count); $i++) {
            if ($Results1[$i].current_mbid -ne $Results2[$i].current_mbid) {
                $Differences++
                Write-Host "  Album $($i+1): $($Results1[$i].album)" -ForegroundColor White
                Write-Host "    Run 1: $($Results1[$i].current_mbid)" -ForegroundColor Gray
                Write-Host "    Run 2: $($Results2[$i].current_mbid)" -ForegroundColor Gray
            }
        }

        Write-Host ""
        Write-Host "Total differences: $Differences albums" -ForegroundColor Yellow
    }
}

Write-Host ""
Write-Host "=== Validation Complete ===" -ForegroundColor Cyan
