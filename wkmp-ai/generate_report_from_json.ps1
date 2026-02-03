# Generate comprehensive markdown report from JSON results
param(
    [string]$InputFile = "run29f_comparison_results.json",
    [string]$OutputFile = "comprehensive_album_report.md"
)

function Format-Duration {
    param([int]$Milliseconds)

    if ($Milliseconds -eq 0 -or $null -eq $Milliseconds) {
        return "0:00"
    }

    $totalSeconds = [math]::Floor($Milliseconds / 1000)
    $minutes = [math]::Floor($totalSeconds / 60)
    $seconds = $totalSeconds % 60

    return "$($minutes):$($seconds.ToString('00'))"
}

Write-Host "Loading results from $InputFile..." -ForegroundColor Cyan
$albums = Get-Content $InputFile -Raw | ConvertFrom-Json

Write-Host "Generating report for $($albums.Count) albums..." -ForegroundColor Cyan

# Initialize report
$report = @()
$report += "# WKMP Album Matcher - Comprehensive Test Report"
$report += ""
$report += "**Generated:** $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')"
$report += "**Total Albums:** $($albums.Count)"
$report += ""

# Summary statistics
$tested = ($albums | Where-Object { -not $_.skipped }).Count
$matched = ($albums | Where-Object { $_.matched }).Count
$exact = ($albums | Where-Object { $_.tracks_match -and $_.mbid_match }).Count
$mbid_changes = ($albums | Where-Object { -not $_.mbid_match -and $_.matched }).Count

$report += "## Summary Statistics"
$report += ""
$report += "- **Total Albums:** $($albums.Count)"
$report += "- **Tested:** $tested"
$report += "- **Skipped:** $($albums.Count - $tested)"
$report += "- **Successfully Matched:** $matched"
$report += "- **Exact Matches:** $exact"
$report += "- **MBID Changes:** $mbid_changes"
$report += ""
$report += "---"
$report += ""

$albumNum = 1
foreach ($album in $albums) {
    Write-Host "Processing [$albumNum/$($albums.Count)]: $($album.artist) - $($album.album)" -ForegroundColor Green
    $albumNum++

    $report += "## $($albumNum - 1). $($album.artist) - $($album.album)"
    $report += ""
    $report += "**File Path:** ``$($album.path)``"
    $report += ""

    # Match status
    if ($album.matched) {
        $statusIcon = if ($album.mbid_match) { "✅" } else { "⚠️" }
        $report += "**Status:** $statusIcon Matched ($($album.match_percentage)%)"

        if (-not $album.mbid_match) {
            $report += ""
            $report += "> **Note:** MBID changed from baseline"
            $report += "> - Baseline MBID: ``$($album.baseline_mbid)``"
            $report += "> - Current MBID: ``$($album.current_mbid)``"
        }
    } elseif ($album.skipped) {
        $report += "**Status:** ⊘ Skipped"
        if ($album.error) {
            $report += "**Reason:** $($album.error)"
        }
    } else {
        $report += "**Status:** ❌ Failed to match"
        if ($album.error) {
            $report += "**Error:** $($album.error)"
        }
    }
    $report += ""

    # Release metadata
    if ($null -ne $album.release_metadata) {
        $meta = $album.release_metadata
        $report += "### Release Information"
        $report += ""
        $report += "- **Release MBID:** ``$($meta.release_mbid)``"
        $report += "- **Title:** $($meta.title)"
        $report += "- **Artist:** $($meta.artist)"

        if ($meta.release_date) {
            $report += "- **Release Date:** $($meta.release_date)"
        }

        if ($meta.country) {
            $report += "- **Country:** $($meta.country)"
        }

        if ($meta.label) {
            if ($meta.catalog_number) {
                $report += "- **Label:** $($meta.label) ($($meta.catalog_number))"
            } else {
                $report += "- **Label:** $($meta.label)"
            }
        }

        if ($meta.barcode) {
            $report += "- **Barcode:** $($meta.barcode)"
        }

        if ($meta.format) {
            $report += "- **Format:** $($meta.format)"
        }

        if ($meta.status) {
            $report += "- **Status:** $($meta.status)"
        }

        $report += ""
    }

    # Track listing
    if ($album.tracks.Count -gt 0) {
        $report += "### Track Listing"
        $report += ""
        $report += "| # | Title | MB Duration | Our Duration | Diff | Status |"
        $report += "|---|-------|-------------|--------------|------|--------|"

        foreach ($track in $album.tracks) {
            $mbDur = Format-Duration -Milliseconds $track.mb_duration_ms
            $ourDur = Format-Duration -Milliseconds $track.our_duration_ms

            # Format difference
            $diffMs = $track.timing_error_ms
            $diffSec = [math]::Abs($diffMs) / 1000.0
            $diffStr = if ($diffMs -ge 0) {
                "+$([math]::Round($diffSec, 2))s"
            } else {
                "-$([math]::Round($diffSec, 2))s"
            }

            # Status icon
            $statusIcon = if ($track.within_tolerance) { "OK" } else { "ERR" }

            # Escape pipe characters in title
            $title = $track.title -replace '\|', '\|'

            $report += "| $($track.track_number) | $title | $mbDur | $ourDur | $diffStr | $statusIcon |"
        }

        $report += ""

        # Track matching statistics
        $totalTracks = $album.tracks.Count
        $matchedTracks = ($album.tracks | Where-Object { $_.within_tolerance }).Count
        $matchPct = if ($totalTracks -gt 0) { [math]::Round(($matchedTracks / $totalTracks) * 100, 1) } else { 0 }

        $report += "**Track Matching:** $matchedTracks / $totalTracks tracks within tolerance ($matchPct%)"
        $report += ""
    }

    # AcousticBrainz coverage
    if ($album.acousticbrainz_total_tracks -gt 0) {
        $abPct = [math]::Round(($album.acousticbrainz_available / $album.acousticbrainz_total_tracks) * 100, 1)
        $report += "**AcousticBrainz Coverage:** $($album.acousticbrainz_available) / $($album.acousticbrainz_total_tracks) tracks ($abPct%)"
        $report += ""
    }

    # Links
    if ($album.current_mbid) {
        $report += "**MusicBrainz:** [View Release](https://musicbrainz.org/release/$($album.current_mbid))"
    }

    if (-not $album.mbid_match -and $album.baseline_mbid -and $album.matched) {
        $report += "**Baseline Release:** [View Baseline](https://musicbrainz.org/release/$($album.baseline_mbid))"
    }

    $report += ""
    $report += "---"
    $report += ""
}

# Write report
Write-Host ""
Write-Host "Writing report to $OutputFile..." -ForegroundColor Cyan
$report | Out-File -FilePath $OutputFile -Encoding UTF8

$sizeKB = [math]::Round((Get-Item $OutputFile).Length / 1KB, 2)
Write-Host "Report generated successfully!" -ForegroundColor Green
Write-Host "File: $OutputFile" -ForegroundColor Green
Write-Host "Size: $sizeKB KB" -ForegroundColor Green
Write-Host "Albums: $($albums.Count)" -ForegroundColor Green
Write-Host ""
