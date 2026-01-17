# Phase 1 Validation Test - All 9 Failed Albums
# Automated test with full output capture

$ErrorActionPreference = "Continue"
$musicRoot = "C:\Users\Mango Cat\Music"
$timestamp = Get-Date -Format "yyyyMMdd_HHmmss"
$outputDir = "phase1_test_results_$timestamp"

# Create output directory
New-Item -ItemType Directory -Force -Path $outputDir | Out-Null

# Test cases - all 9 failed albums
$testCases = @(
    @{
        File = "Mayall, John\AHardRoad.mp3"
        Artist = "John Mayall"
        Album = "A Hard Road"
        ExpectedMBArtist = "John Mayall & the Bluesbreakers"
        Expected = "SHOULD_MATCH"
        Reason = "Artist suffix normalization"
        Phase1Extension = "Extension 1 (normalization)"
    },
    @{
        File = "Go Gos, The\BeautyAndTheBeat.mp3"
        Artist = "The Go-Go's"
        Album = "Beauty And The Beat"
        ExpectedMBArtist = "The Go-Go's"
        Expected = "SHOULD_MATCH"
        Reason = "Prefix + punctuation normalization"
        Phase1Extension = "Extension 1 (normalization)"
    },
    @{
        File = "Brubeck, Dave\TheBestOfTheDaveBrubeckQuartet.mp3"
        Artist = "Dave Brubeck"
        Album = "The Best Of The Dave Brubeck Quartet"
        ExpectedMBArtist = "The Dave Brubeck Quartet"
        Expected = "SHOULD_MATCH"
        Reason = "Prefix + suffix normalization"
        Phase1Extension = "Extension 1 (normalization)"
    },
    @{
        File = "Santana\InvitationToIllumination.mp3"
        Artist = "Santana"
        Album = "Invitation to Illumination"
        ExpectedMBArtist = "Carlos Santana"
        Expected = "SHOULD_MATCH"
        Reason = "Substring bonus (+20%)"
        Phase1Extension = "Extension 2 (similarity bonus)"
    },
    @{
        File = "Police\RegattaDeBlanc.mp3"
        Artist = "Police"
        Album = "Reggatta De Blanc"
        ExpectedMBArtist = "The Police"
        Expected = "SHOULD_MATCH"
        Reason = "Prefix normalization"
        Phase1Extension = "Extension 1 (normalization)"
    },
    @{
        File = "Score, The\Atlas.mp3"
        Artist = "The Score"
        Album = "Atlas"
        ExpectedMBArtist = "The Score"
        Expected = "SHOULD_MATCH"
        Reason = "Prefix normalization"
        Phase1Extension = "Extension 1 (normalization)"
    },
    @{
        File = "Phildel\Ritual.mp3"
        Artist = "Phildel"
        Album = "Ritual"
        ExpectedMBArtist = "Delerium"
        Expected = "WILL_FAIL"
        Reason = "Wrong artist folder (file mislabeled)"
        Phase1Extension = "N/A - not fixable by artist normalization"
    },
    @{
        File = "Hooverphonic\LiveAtTheAncienneBelgique.mp3"
        Artist = "Hooverphonic"
        Album = "Live at the Ancienne Belgique"
        ExpectedMBArtist = "Hooverphonic"
        Expected = "LIKELY_FAILS"
        Reason = "Non-artist issue (year/title/format)"
        Phase1Extension = "N/A - artist already matches"
    },
    @{
        File = "Various\TheGreatestShowman.mp3"
        Artist = "Various Artists"
        Album = "The Greatest Showman"
        ExpectedMBArtist = "Various Artists"
        Expected = "LIKELY_FAILS"
        Reason = "VA compilation (needs special handling)"
        Phase1Extension = "N/A - requires VA compilation support"
    }
)

Write-Host "=" * 100 -ForegroundColor Cyan
Write-Host "Phase 1 Validation Test - All 9 Failed Albums" -ForegroundColor Cyan
Write-Host "=" * 100 -ForegroundColor Cyan
Write-Host ""
Write-Host "Output directory: $outputDir" -ForegroundColor Gray
Write-Host "Music root: $musicRoot" -ForegroundColor Gray
Write-Host ""

# Initialize results
$results = @()
$startTime = Get-Date

# Check music root
if (-not (Test-Path $musicRoot)) {
    Write-Host "ERROR: Music folder not found at: $musicRoot" -ForegroundColor Red
    exit 1
}

# Check files
Write-Host "Checking files..." -ForegroundColor Yellow
$missingCount = 0
foreach ($test in $testCases) {
    $fullPath = Join-Path $musicRoot $test.File
    if (-not (Test-Path $fullPath)) {
        Write-Host "  MISSING: $($test.File)" -ForegroundColor Red
        $missingCount++
    }
}

if ($missingCount -gt 0) {
    Write-Host ""
    Write-Host "ERROR: $missingCount file(s) not found" -ForegroundColor Red
    exit 1
}
Write-Host "  All 9 files found" -ForegroundColor Green
Write-Host ""

# Build album matcher
Write-Host "Building album matcher..." -ForegroundColor Yellow
Set-Location wkmp-ai
$buildLog = Join-Path "..\$outputDir" "build.log"
cargo build --example album_matcher_28 --release 2>&1 | Tee-Object -FilePath $buildLog | Out-Null
if ($LASTEXITCODE -ne 0) {
    Write-Host "Build failed! See: $buildLog" -ForegroundColor Red
    Set-Location ..
    exit 1
}
Set-Location ..
Write-Host "  Build successful" -ForegroundColor Green
Write-Host ""

# Test each album
Write-Host "=" * 100 -ForegroundColor Cyan
Write-Host "TESTING ALBUMS" -ForegroundColor Cyan
Write-Host "=" * 100 -ForegroundColor Cyan
Write-Host ""

$testNum = 0
foreach ($test in $testCases) {
    $testNum++

    Write-Host "[$testNum/9] Testing: $($test.Artist) - $($test.Album)" -ForegroundColor Cyan
    Write-Host "  File: $($test.File)" -ForegroundColor Gray
    Write-Host "  Expected: $($test.Expected)" -ForegroundColor $(if ($test.Expected -eq "SHOULD_MATCH") { "Green" } else { "Yellow" })
    Write-Host "  Reason: $($test.Reason)" -ForegroundColor Gray
    Write-Host "  Extension: $($test.Phase1Extension)" -ForegroundColor Gray

    $fullPath = Join-Path $musicRoot $test.File
    $safeName = $test.File -replace '[\\/:*?"<>|]', '_'
    $logFile = Join-Path $outputDir "test_${testNum}_${safeName}.log"

    # Run album matcher
    Write-Host "  Running matcher..." -ForegroundColor Gray
    Set-Location wkmp-ai
    $output = & cargo run --example album_matcher_28 --release -- --file $fullPath 2>&1 | Tee-Object -FilePath "..\$logFile" | Out-String
    Set-Location ..

    # Parse results
    $matched = $false
    $mbid = $null
    $confidence = $null
    $matchPct = $null
    $selectedArtist = $null
    $selectedAlbum = $null

    # Check for match indicators
    if ($output -match "(?i)matched to release|selected edition|assigned mbid") {
        $matched = $true
    }

    # Extract MBID
    if ($output -match "MBID:\s*([a-f0-9\-]+)") {
        $mbid = $matches[1]
    }

    # Extract match percentage
    if ($output -match "(\d+\.?\d*)\s*%") {
        $matchPct = [float]$matches[1]
    }

    # Extract selected artist/album
    if ($output -match "Artist:\s*(.+)") {
        $selectedArtist = $matches[1].Trim()
    }
    if ($output -match "Album:\s*(.+)") {
        $selectedAlbum = $matches[1].Trim()
    }

    # Determine result
    $status = if ($matched) { "MATCHED" } else { "FAILED" }
    $expectationMet = $null

    if ($test.Expected -eq "SHOULD_MATCH") {
        $expectationMet = ($status -eq "MATCHED")
    } elseif ($test.Expected -eq "WILL_FAIL") {
        $expectationMet = ($status -eq "FAILED")
    } else { # LIKELY_FAILS
        $expectationMet = $null # No strong expectation
    }

    # Display result
    if ($status -eq "MATCHED") {
        Write-Host "  Result: MATCHED" -ForegroundColor Green
        if ($matchPct) {
            Write-Host "    Match quality: $matchPct%" -ForegroundColor Green
        }
        if ($mbid) {
            Write-Host "    MBID: $mbid" -ForegroundColor Green
        }
        if ($selectedArtist) {
            Write-Host "    MB Artist: $selectedArtist" -ForegroundColor Green
        }
    } else {
        Write-Host "  Result: FAILED" -ForegroundColor Red
    }

    if ($expectationMet -eq $true) {
        Write-Host "  Expectation: MET (as predicted)" -ForegroundColor Green
    } elseif ($expectationMet -eq $false) {
        Write-Host "  Expectation: NOT MET (unexpected)" -ForegroundColor Yellow
    }

    # Store result
    $results += @{
        TestNumber = $testNum
        File = $test.File
        Artist = $test.Artist
        Album = $test.Album
        ExpectedMBArtist = $test.ExpectedMBArtist
        Expected = $test.Expected
        Reason = $test.Reason
        Phase1Extension = $test.Phase1Extension
        Status = $status
        MBID = $mbid
        MatchPercentage = $matchPct
        SelectedArtist = $selectedArtist
        SelectedAlbum = $selectedAlbum
        ExpectationMet = $expectationMet
        LogFile = $logFile
    }

    Write-Host ""
}

$endTime = Get-Date
$duration = $endTime - $startTime

# Summary
Write-Host "=" * 100 -ForegroundColor Cyan
Write-Host "SUMMARY" -ForegroundColor Cyan
Write-Host "=" * 100 -ForegroundColor Cyan
Write-Host ""

$totalTested = $results.Count
$matchedCount = ($results | Where-Object { $_.Status -eq "MATCHED" }).Count
$failedCount = ($results | Where-Object { $_.Status -eq "FAILED" }).Count

$shouldMatchCount = ($results | Where-Object { $_.Expected -eq "SHOULD_MATCH" }).Count
$shouldMatchActual = ($results | Where-Object { $_.Expected -eq "SHOULD_MATCH" -and $_.Status -eq "MATCHED" }).Count

$willFailCount = ($results | Where-Object { $_.Expected -eq "WILL_FAIL" }).Count
$willFailActual = ($results | Where-Object { $_.Expected -eq "WILL_FAIL" -and $_.Status -eq "FAILED" }).Count

Write-Host "Total albums tested: $totalTested" -ForegroundColor White
Write-Host "Total matched: $matchedCount" -ForegroundColor $(if ($matchedCount -gt 0) { "Green" } else { "Red" })
Write-Host "Total failed: $failedCount" -ForegroundColor $(if ($failedCount -lt $totalTested) { "Green" } else { "Red" })
Write-Host ""

Write-Host "Phase 1 Predictions:" -ForegroundColor Cyan
Write-Host "  Should match (6 albums): $shouldMatchActual/$shouldMatchCount matched" -ForegroundColor $(if ($shouldMatchActual -eq $shouldMatchCount) { "Green" } else { "Yellow" })
Write-Host "  Should fail (1 album): $willFailActual/$willFailCount failed" -ForegroundColor $(if ($willFailActual -eq $willFailCount) { "Green" } else { "Yellow" })
Write-Host ""

Write-Host "Improvement Rate:" -ForegroundColor Cyan
$baselineFailures = 9
$improvement = $matchedCount
$improvementPct = [math]::Round(($improvement / $baselineFailures) * 100, 1)
Write-Host "  Baseline: 0/9 matched (100% failure)" -ForegroundColor Gray
Write-Host "  Phase 1:  $matchedCount/9 matched ($improvementPct% fixed)" -ForegroundColor White
Write-Host ""

Write-Host "Library Impact:" -ForegroundColor Cyan
$totalLibrary = 192
$baselineSuccess = 183
$newSuccess = $baselineSuccess + $matchedCount
$baselineRate = [math]::Round(($baselineSuccess / $totalLibrary) * 100, 1)
$newRate = [math]::Round(($newSuccess / $totalLibrary) * 100, 1)
$improvement = [math]::Round($newRate - $baselineRate, 1)
Write-Host "  Before: $baselineSuccess/$totalLibrary matched ($baselineRate%)" -ForegroundColor Gray
Write-Host "  After:  $newSuccess/$totalLibrary matched ($newRate%)" -ForegroundColor $(if ($newSuccess -gt $baselineSuccess) { "Green" } else { "White" })
Write-Host "  Improvement: +$improvement percentage points" -ForegroundColor $(if ($improvement -gt 0) { "Green" } else { "White" })
Write-Host ""

# Detailed results
Write-Host "Detailed Results:" -ForegroundColor Cyan
Write-Host ""

if ($shouldMatchActual -gt 0) {
    Write-Host "MATCHED (as expected):" -ForegroundColor Green
    foreach ($result in ($results | Where-Object { $_.Expected -eq "SHOULD_MATCH" -and $_.Status -eq "MATCHED" })) {
        Write-Host "  [OK] $($result.Artist) - $($result.Album)" -ForegroundColor Green
        Write-Host "       Fix: $($result.Phase1Extension)" -ForegroundColor Gray
        if ($result.MatchPercentage) {
            Write-Host "       Quality: $($result.MatchPercentage)%" -ForegroundColor Gray
        }
    }
    Write-Host ""
}

$unexpectedMatches = $results | Where-Object { $_.Expected -ne "SHOULD_MATCH" -and $_.Status -eq "MATCHED" }
if ($unexpectedMatches.Count -gt 0) {
    Write-Host "MATCHED (unexpected bonus):" -ForegroundColor Yellow
    foreach ($result in $unexpectedMatches) {
        Write-Host "  [+] $($result.Artist) - $($result.Album)" -ForegroundColor Yellow
        Write-Host "      Expected: $($result.Expected)" -ForegroundColor Gray
    }
    Write-Host ""
}

$expectedFailures = $results | Where-Object { ($_.Expected -eq "WILL_FAIL" -or $_.Expected -eq "LIKELY_FAILS") -and $_.Status -eq "FAILED" }
if ($expectedFailures.Count -gt 0) {
    Write-Host "FAILED (as expected - not fixable by Phase 1):" -ForegroundColor Yellow
    foreach ($result in $expectedFailures) {
        Write-Host "  [-] $($result.Artist) - $($result.Album)" -ForegroundColor Yellow
        Write-Host "      Reason: $($result.Reason)" -ForegroundColor Gray
    }
    Write-Host ""
}

$unexpectedFailures = $results | Where-Object { $_.Expected -eq "SHOULD_MATCH" -and $_.Status -eq "FAILED" }
if ($unexpectedFailures.Count -gt 0) {
    Write-Host "FAILED (unexpected - should have matched):" -ForegroundColor Red
    foreach ($result in $unexpectedFailures) {
        Write-Host "  [!] $($result.Artist) - $($result.Album)" -ForegroundColor Red
        Write-Host "      Expected fix: $($result.Phase1Extension)" -ForegroundColor Gray
        Write-Host "      Log: $($result.LogFile)" -ForegroundColor Gray
    }
    Write-Host ""
}

Write-Host "Test duration: $($duration.ToString('mm\:ss'))" -ForegroundColor Gray
Write-Host ""

# Save JSON results
$jsonFile = Join-Path $outputDir "results.json"
$results | ConvertTo-Json -Depth 10 | Set-Content $jsonFile
Write-Host "Results saved to: $jsonFile" -ForegroundColor Cyan

# Save text summary
$summaryFile = Join-Path $outputDir "summary.txt"
@"
Phase 1 Validation Test Results
================================
Timestamp: $(Get-Date -Format "yyyy-MM-dd HH:mm:ss")
Duration: $($duration.ToString('mm\:ss'))

Summary:
--------
Total albums tested: $totalTested
Matched: $matchedCount
Failed: $failedCount

Phase 1 Predictions:
-------------------
Should match (6 albums): $shouldMatchActual/$shouldMatchCount
Should fail (1 album): $willFailActual/$willFailCount

Improvement:
-----------
Baseline: 0/9 matched (100% failure)
Phase 1:  $matchedCount/9 matched ($improvementPct% fixed)

Library Impact:
--------------
Before: $baselineSuccess/$totalLibrary ($baselineRate%)
After:  $newSuccess/$totalLibrary ($newRate%)
Improvement: +$improvement percentage points

Individual Results:
------------------
$($results | ForEach-Object {
    "$($_.Status.PadRight(10)) $($_.Artist) - $($_.Album)"
    "           Expected: $($_.Expected), Met: $($_.ExpectationMet)"
    "           Extension: $($_.Phase1Extension)"
    if ($_.MatchPercentage) { "           Quality: $($_.MatchPercentage)%" }
    if ($_.MBID) { "           MBID: $($_.MBID)" }
    ""
} | Out-String)

Detailed logs saved in: $outputDir
"@ | Set-Content $summaryFile

Write-Host "Summary saved to: $summaryFile" -ForegroundColor Cyan
Write-Host ""

# Final assessment
if ($shouldMatchActual -eq $shouldMatchCount -and $willFailActual -eq $willFailCount) {
    Write-Host "PHASE 1 STATUS: SUCCESS" -ForegroundColor Green
    Write-Host "All predictions met - Phase 1 extensions working as designed" -ForegroundColor Green
} elseif ($matchedCount -gt 0) {
    Write-Host "PHASE 1 STATUS: PARTIAL SUCCESS" -ForegroundColor Yellow
    Write-Host "Some improvements observed, may need investigation" -ForegroundColor Yellow
} else {
    Write-Host "PHASE 1 STATUS: NEEDS INVESTIGATION" -ForegroundColor Red
    Write-Host "Results differ from predictions - review logs" -ForegroundColor Red
}

Write-Host ""
Write-Host "Output directory contains:" -ForegroundColor Cyan
Write-Host "  - results.json (machine-readable results)" -ForegroundColor White
Write-Host "  - summary.txt (human-readable summary)" -ForegroundColor White
Write-Host "  - build.log (build output)" -ForegroundColor White
Write-Host "  - test_N_*.log (individual test logs)" -ForegroundColor White

exit 0
