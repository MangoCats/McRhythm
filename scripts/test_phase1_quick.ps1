# Quick Phase 1 Test - Non-Interactive
# Tests 2 key albums to verify Phase 1 improvements

$musicRoot = "C:\Users\Mango Cat\Music"

Write-Host "=" * 80 -ForegroundColor Cyan
Write-Host "Phase 1 Quick Verification Test" -ForegroundColor Cyan
Write-Host "=" * 80 -ForegroundColor Cyan
Write-Host ""

# Test cases - pick 2 representative albums
$testCases = @(
    @{
        File = "Mayall, John\AHardRoad.mp3"
        Artist = "John Mayall"
        Album = "A Hard Road"
        Expected = "SHOULD MATCH"
        Reason = "Artist suffix normalization (John Mayall vs John Mayall & the Bluesbreakers)"
    },
    @{
        File = "Go Gos, The\BeautyAndTheBeat.mp3"
        Artist = "The Go-Go's"
        Album = "Beauty And The Beat"
        Expected = "SHOULD MATCH"
        Reason = "Prefix + punctuation normalization (The Go Gos vs Go-Go's)"
    }
)

# Check music root
if (-not (Test-Path $musicRoot)) {
    Write-Host "ERROR: Music folder not found at: $musicRoot" -ForegroundColor Red
    Write-Host "Update script with correct path" -ForegroundColor Red
    exit 1
}

Write-Host "Music root: $musicRoot" -ForegroundColor Gray
Write-Host ""

# Check files exist
$allExist = $true
foreach ($test in $testCases) {
    $fullPath = Join-Path $musicRoot $test.File
    if (-not (Test-Path $fullPath)) {
        Write-Host "Missing: $($test.File)" -ForegroundColor Red
        $allExist = $false
    }
}

if (-not $allExist) {
    Write-Host ""
    Write-Host "ERROR: Some test files not found" -ForegroundColor Red
    Write-Host "Cannot proceed with test" -ForegroundColor Red
    exit 1
}

# Build album matcher
Write-Host "Building album matcher..." -ForegroundColor Yellow
Set-Location wkmp-ai
$buildOutput = cargo build --example album_matcher_28 --release 2>&1 | Out-String
if ($LASTEXITCODE -ne 0) {
    Write-Host "Build failed!" -ForegroundColor Red
    Write-Host $buildOutput
    Set-Location ..
    exit 1
}
Set-Location ..
Write-Host "Build successful" -ForegroundColor Green
Write-Host ""

# Test each album
$results = @()

foreach ($test in $testCases) {
    Write-Host "Testing: $($test.Artist) - $($test.Album)" -ForegroundColor Cyan
    Write-Host "  File: $($test.File)" -ForegroundColor Gray
    Write-Host "  Reason: $($test.Reason)" -ForegroundColor Gray

    $fullPath = Join-Path $musicRoot $test.File

    # Run album matcher
    Set-Location wkmp-ai
    $output = & cargo run --example album_matcher_28 --release -- --file $fullPath 2>&1 | Out-String
    Set-Location ..

    # Check for match indicators
    $matched = $false
    $matchInfo = ""

    if ($output -match "(?i)matched to release|selected edition|assigned mbid") {
        $matched = $true
    }

    # Try to extract match percentage
    if ($output -match "(\d+\.?\d*)\s*%\s*match") {
        $matchInfo = "Match: $($matches[1])%"
    }

    # Result
    if ($matched) {
        Write-Host "  Result: MATCHED" -ForegroundColor Green
        if ($matchInfo) {
            Write-Host "  $matchInfo" -ForegroundColor Green
        }
        $results += @{ Test = $test; Status = "MATCHED" }
    } else {
        Write-Host "  Result: FAILED" -ForegroundColor Red
        $results += @{ Test = $test; Status = "FAILED" }
    }

    Write-Host ""
}

# Summary
Write-Host "=" * 80 -ForegroundColor Cyan
Write-Host "SUMMARY" -ForegroundColor Cyan
Write-Host "=" * 80 -ForegroundColor Cyan
Write-Host ""

$matchCount = ($results | Where-Object { $_.Status -eq "MATCHED" }).Count
$failCount = ($results | Where-Object { $_.Status -eq "FAILED" }).Count

Write-Host "Albums tested: $($results.Count)" -ForegroundColor White
Write-Host "Matched: $matchCount" -ForegroundColor $(if ($matchCount -gt 0) { "Green" } else { "Red" })
Write-Host "Failed: $failCount" -ForegroundColor $(if ($failCount -eq 0) { "Green" } else { "Red" })
Write-Host ""

if ($matchCount -eq $results.Count) {
    Write-Host "SUCCESS: All test albums matched!" -ForegroundColor Green
    Write-Host "Phase 1 extensions are working as expected." -ForegroundColor Green
    Write-Host ""
    Write-Host "Based on these results, Phase 1 should fix ~6/9 failed albums." -ForegroundColor White
} elseif ($matchCount -gt 0) {
    Write-Host "PARTIAL: Some albums matched" -ForegroundColor Yellow
    Write-Host "Phase 1 may need adjustment or investigation" -ForegroundColor Yellow
} else {
    Write-Host "FAILURE: No albums matched" -ForegroundColor Red
    Write-Host "Phase 1 extensions may not be working correctly" -ForegroundColor Red
}

Write-Host ""
Write-Host "Next steps:" -ForegroundColor Cyan
Write-Host "  - Review phase1_expected_improvements.md for full analysis" -ForegroundColor White
Write-Host "  - Test remaining 7 failed albums for complete validation" -ForegroundColor White
Write-Host "  - Consider Phase 2 for remaining failures" -ForegroundColor White

exit 0
