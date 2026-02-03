# Test Phase 1 Extensions on Failed Albums
# Quick verification script for Phase 1 improvements

$musicRoot = "C:\Users\Mango Cat\Music"
$failedAlbums = @(
    @{
        File = "Mayall, John\AHardRoad.mp3"
        Artist = "John Mayall"
        Album = "A Hard Road"
        Expected = "SHOULD MATCH"
        Reason = "Artist suffix normalization"
    },
    @{
        File = "Go Gos, The\BeautyAndTheBeat.mp3"
        Artist = "The Go-Go's"
        Album = "Beauty And The Beat"
        Expected = "SHOULD MATCH"
        Reason = "Prefix + punctuation normalization"
    },
    @{
        File = "Brubeck, Dave\TheBestOfTheDaveBrubeckQuartet.mp3"
        Artist = "The Dave Brubeck Quartet"
        Album = "The Best Of..."
        Expected = "SHOULD MATCH"
        Reason = "Prefix + suffix normalization"
    },
    @{
        File = "Santana\InvitationToIllumination.mp3"
        Artist = "Carlos Santana"
        Album = "Invitation to Illumination"
        Expected = "SHOULD MATCH"
        Reason = "Substring bonus (+20%)"
    },
    @{
        File = "Police\RegattaDeBlanc.mp3"
        Artist = "The Police"
        Album = "Reggatta De Blanc"
        Expected = "SHOULD MATCH"
        Reason = "Prefix normalization"
    },
    @{
        File = "Score, The\Atlas.mp3"
        Artist = "The Score"
        Album = "Atlas"
        Expected = "SHOULD MATCH"
        Reason = "Prefix normalization"
    },
    @{
        File = "Phildel\Ritual.mp3"
        Artist = "Delerium"
        Album = "Ritual"
        Expected = "LIKELY FAILS"
        Reason = "Wrong artist folder (not fixable by Phase 1)"
    },
    @{
        File = "Hooverphonic\LiveAtTheAncienneBelgique.mp3"
        Artist = "Hooverphonic"
        Album = "Live at the Ancienne Belgique"
        Expected = "LIKELY FAILS"
        Reason = "Non-artist issue (year/title)"
    },
    @{
        File = "Various\TheGreatestShowman.mp3"
        Artist = "Various Artists"
        Album = "The Greatest Showman"
        Expected = "LIKELY FAILS"
        Reason = "VA compilation (needs special handling)"
    }
)

Write-Host "=" * 80 -ForegroundColor Cyan
Write-Host "Phase 1 Extensions - Failed Albums Test" -ForegroundColor Cyan
Write-Host "=" * 80 -ForegroundColor Cyan
Write-Host ""

Write-Host "This script helps test the 9 previously failed albums" -ForegroundColor Yellow
Write-Host "Phase 1 should fix 6/9 albums (67% improvement)" -ForegroundColor Yellow
Write-Host ""

# Check if music root exists
if (-not (Test-Path $musicRoot)) {
    Write-Host "ERROR: Music folder not found at: $musicRoot" -ForegroundColor Red
    Write-Host "Please update `$musicRoot variable in this script" -ForegroundColor Red
    exit 1
}

# Check if files exist
Write-Host "Checking files..." -ForegroundColor Gray
$missingFiles = 0
foreach ($album in $failedAlbums) {
    $fullPath = Join-Path $musicRoot $album.File
    if (-not (Test-Path $fullPath)) {
        Write-Host "  Missing: $($album.File)" -ForegroundColor Red
        $missingFiles++
    }
}

if ($missingFiles -gt 0) {
    Write-Host ""
    Write-Host "WARNING: $missingFiles file(s) not found" -ForegroundColor Yellow
    Write-Host "Tests will be skipped for missing files" -ForegroundColor Yellow
}
Write-Host ""

# Ask user if they want to run tests
Write-Host "Ready to test albums. Choose an option:" -ForegroundColor Cyan
Write-Host "  1. Test all 9 albums automatically (recommended)" -ForegroundColor White
Write-Host "  2. Test only 'should match' albums (6 albums)" -ForegroundColor White
Write-Host "  3. Test albums one by one (manual)" -ForegroundColor White
Write-Host "  4. Just show album list and exit" -ForegroundColor White
Write-Host ""
$choice = Read-Host "Enter choice (1-4)"

if ($choice -eq "4") {
    # Just list albums
    Write-Host ""
    Write-Host "Failed Albums List:" -ForegroundColor Cyan
    for ($i = 0; $i -lt $failedAlbums.Count; $i++) {
        $album = $failedAlbums[$i]
        Write-Host ""
        Write-Host "[$($i+1)] $($album.Artist) - $($album.Album)" -ForegroundColor White
        Write-Host "    File: $($album.File)" -ForegroundColor Gray
        Write-Host "    Expected: $($album.Expected)" -ForegroundColor $(if ($album.Expected -eq "SHOULD MATCH") { "Green" } else { "Yellow" })
        Write-Host "    Reason: $($album.Reason)" -ForegroundColor Gray
    }
    exit 0
}

# Build album matcher first
Write-Host "Building album matcher (this may take a minute)..." -ForegroundColor Yellow
Set-Location wkmp-ai
$buildResult = cargo build --example album_matcher_28 --release 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Host "Build failed!" -ForegroundColor Red
    Write-Host $buildResult
    Set-Location ..
    exit 1
}
Set-Location ..
Write-Host "Build successful" -ForegroundColor Green
Write-Host ""

# Function to test a single album
function Test-Album {
    param($album)

    $fullPath = Join-Path $musicRoot $album.File

    if (-not (Test-Path $fullPath)) {
        return @{
            Status = "FILE_NOT_FOUND"
            Message = "File not found"
        }
    }

    Write-Host "Testing: $($album.Artist) - $($album.Album)" -ForegroundColor Cyan
    Write-Host "  Expected: $($album.Expected) ($($album.Reason))" -ForegroundColor Gray

    # Run album matcher
    Set-Location wkmp-ai
    $output = & cargo run --example album_matcher_28 --release -- --file $fullPath 2>&1 | Out-String
    Set-Location ..

    # Check if matched
    $matched = $output -match "Matched to release" -or $output -match "Selected edition" -or $output -match "MBID:"

    # Extract match percentage if available
    $matchPercentage = $null
    if ($output -match "(\d+\.?\d*)%") {
        $matchPercentage = $matches[1]
    }

    if ($matched) {
        Write-Host "  Result: MATCHED" -ForegroundColor Green
        if ($matchPercentage) {
            Write-Host "  Quality: $matchPercentage%" -ForegroundColor Green
        }
        return @{
            Status = "MATCHED"
            Percentage = $matchPercentage
        }
    } else {
        Write-Host "  Result: FAILED" -ForegroundColor Red
        return @{
            Status = "FAILED"
        }
    }
}

# Run tests based on choice
$results = @()

if ($choice -eq "1") {
    # Test all albums
    foreach ($album in $failedAlbums) {
        $result = Test-Album $album
        $results += @{
            Album = $album
            Result = $result
        }
        Write-Host ""
    }
}
elseif ($choice -eq "2") {
    # Test only "should match" albums
    $shouldMatchAlbums = $failedAlbums | Where-Object { $_.Expected -eq "SHOULD MATCH" }
    foreach ($album in $shouldMatchAlbums) {
        $result = Test-Album $album
        $results += @{
            Album = $album
            Result = $result
        }
        Write-Host ""
    }
}
elseif ($choice -eq "3") {
    # Manual one-by-one
    for ($i = 0; $i -lt $failedAlbums.Count; $i++) {
        $album = $failedAlbums[$i]

        Write-Host ""
        Write-Host "[$($i+1)/$($failedAlbums.Count)] Next album:" -ForegroundColor Cyan
        Write-Host "  $($album.Artist) - $($album.Album)" -ForegroundColor White
        Write-Host "  Expected: $($album.Expected)" -ForegroundColor $(if ($album.Expected -eq "SHOULD MATCH") { "Green" } else { "Yellow" })

        $continue = Read-Host "Test this album? (y/n/q)"
        if ($continue -eq "q") { break }
        if ($continue -ne "y") { continue }

        $result = Test-Album $album
        $results += @{
            Album = $album
            Result = $result
        }
        Write-Host ""
    }
}

# Summary
if ($results.Count -gt 0) {
    Write-Host ""
    Write-Host "=" * 80 -ForegroundColor Cyan
    Write-Host "SUMMARY" -ForegroundColor Cyan
    Write-Host "=" * 80 -ForegroundColor Cyan
    Write-Host ""

    $totalTested = $results.Count
    $matched = ($results | Where-Object { $_.Result.Status -eq "MATCHED" }).Count
    $failed = ($results | Where-Object { $_.Result.Status -eq "FAILED" }).Count

    Write-Host "Albums tested: $totalTested" -ForegroundColor White
    Write-Host "Matched: $matched" -ForegroundColor Green
    Write-Host "Failed: $failed" -ForegroundColor Red
    Write-Host ""

    $expectedMatches = ($results | Where-Object { $_.Album.Expected -eq "SHOULD MATCH" }).Count
    if ($expectedMatches -gt 0) {
        $actualMatches = ($results | Where-Object {
            $_.Album.Expected -eq "SHOULD MATCH" -and $_.Result.Status -eq "MATCHED"
        }).Count

        Write-Host "Expected to match: $expectedMatches" -ForegroundColor Yellow
        Write-Host "Actually matched: $actualMatches" -ForegroundColor $(if ($actualMatches -eq $expectedMatches) { "Green" } else { "Yellow" })

        if ($actualMatches -eq $expectedMatches) {
            Write-Host ""
            Write-Host "SUCCESS: Phase 1 working as expected!" -ForegroundColor Green
        } elseif ($actualMatches -gt 0) {
            Write-Host ""
            Write-Host "PARTIAL: Some expected matches succeeded" -ForegroundColor Yellow
        } else {
            Write-Host ""
            Write-Host "ISSUE: Expected matches did not succeed" -ForegroundColor Red
        }
    }

    Write-Host ""
    Write-Host "Baseline: 0/9 matched (100% failure rate)" -ForegroundColor Gray
    Write-Host "Phase 1:  $matched/$totalTested matched ($(100 - ($failed * 100 / $totalTested))% success rate)" -ForegroundColor White
}

Write-Host ""
Write-Host "See phase1_expected_improvements.md for detailed analysis" -ForegroundColor Cyan
