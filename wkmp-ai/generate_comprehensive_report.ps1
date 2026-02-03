# Generate comprehensive album match report with MusicBrainz edition details
# This will query MusicBrainz API for each album - will take ~10-15 minutes for 200 albums

param(
    [int]$LimitAlbums = 0,  # Set to positive number to limit report (0 = all albums)
    [string]$OutputFile = "comprehensive_album_report.md"
)

# MusicBrainz API rate limiting (1 request per second)
$script:LastApiCall = [DateTime]::MinValue
$script:ApiDelay = 1000  # milliseconds

function Wait-ForRateLimit {
    $elapsed = ([DateTime]::Now - $script:LastApiCall).TotalMilliseconds
    if ($elapsed -lt $script:ApiDelay) {
        Start-Sleep -Milliseconds ($script:ApiDelay - $elapsed)
    }
    $script:LastApiCall = [DateTime]::Now
}

function Get-MusicBrainzRelease {
    param([string]$Mbid)

    Wait-ForRateLimit

    $url = "https://musicbrainz.org/ws/2/release/$Mbid?inc=artists+labels+recordings"

    try {
        $response = Invoke-RestMethod -Uri $url -Headers @{
            "User-Agent" = "WKMPTestReport/1.0 (contact@example.com)"
            "Accept" = "application/json"
        } -ErrorAction Stop
        return $response
    } catch {
        Write-Warning "Failed to fetch MBID $Mbid : $_"
        return $null
    }
}

function Format-Duration {
    param([int]$Milliseconds)

    if ($Milliseconds -eq 0 -or $null -eq $Milliseconds) {
        return "N/A"
    }

    $seconds = [math]::Floor($Milliseconds / 1000)
    $minutes = [math]::Floor($seconds / 60)
    $remainingSeconds = $seconds % 60

    return "$($minutes):$($remainingSeconds.ToString('00'))"
}

# Load results
Write-Host "Loading test results..." -ForegroundColor Cyan
$json = Get-Content 'run29f_comparison_results.json' -Raw | ConvertFrom-Json

$albums = $json
if ($LimitAlbums -gt 0) {
    $albums = $json | Select-Object -First $LimitAlbums
    Write-Host "Limited to first $LimitAlbums albums for testing" -ForegroundColor Yellow
}

Write-Host "Generating report for $($albums.Count) albums..." -ForegroundColor Cyan
Write-Host "This will take approximately $([math]::Ceiling($albums.Count * 1.1 / 60)) minutes due to MusicBrainz API rate limiting" -ForegroundColor Yellow
Write-Host ""

# Initialize report
$report = @()
$report += "# WKMP Album Matcher - Comprehensive Test Report"
$report += ""
$report += "**Generated:** $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')"
$report += "**Total Albums:** $($albums.Count)"
$report += ""
$report += "---"
$report += ""

$processed = 0
foreach ($album in $albums) {
    $processed++
    Write-Host "[$processed/$($albums.Count)] Processing: $($album.artist) - $($album.album)" -ForegroundColor Green

    $report += "## $($processed). $($album.artist) - $($album.album)"
    $report += ""
    $report += "**File Path:** ``$($album.path)``"
    $report += ""

    # Match status
    if ($album.matched) {
        $statusIcon = if ($album.mbid_match) { "✅" } else { "⚠️" }
        $report += "**Status:** $statusIcon Matched ($($album.match_percentage)%)"
    } else {
        $report += "**Status:** ❌ Failed to match"
    }

    if (-not $album.mbid_match -and $album.matched) {
        $report += "**Note:** MBID changed from baseline"
    }

    $report += ""

    # Fetch MusicBrainz details
    if ($album.matched -and $album.current_mbid) {
        $release = Get-MusicBrainzRelease -Mbid $album.current_mbid

        if ($release) {
            $report += "### MusicBrainz Release Details"
            $report += ""
            $report += "- **Release MBID:** ``$($album.current_mbid)``"
            $report += "- **Title:** $($release.title)"

            if ($release.'artist-credit') {
                $artistNames = $release.'artist-credit' | ForEach-Object { $_.name }
                $report += "- **Artist(s):** $($artistNames -join ', ')"
            }

            if ($release.date) {
                $report += "- **Release Date:** $($release.date)"
            }

            if ($release.country) {
                $report += "- **Country:** $($release.country)"
            }

            if ($release.'label-info' -and $release.'label-info'.Count -gt 0) {
                $labels = $release.'label-info' | ForEach-Object {
                    if ($_.label) { $_.label.name } else { "Unknown" }
                }
                $report += "- **Label(s):** $($labels -join ', ')"

                $catalogs = $release.'label-info' | ForEach-Object {
                    if ($_.'catalog-number') { $_.'catalog-number' } else { $null }
                } | Where-Object { $_ -ne $null }
                if ($catalogs.Count -gt 0) {
                    $report += "- **Catalog Number(s):** $($catalogs -join ', ')"
                }
            }

            if ($release.barcode) {
                $report += "- **Barcode:** $($release.barcode)"
            }

            $report += "- **Track Count:** $($release.'track-count')"
            $report += ""

            # Track listing
            $report += "### Track Listing"
            $report += ""
            $report += "| # | Title | MB Duration | Our Duration | Diff |"
            $report += "|---|-------|-------------|--------------|------|"

            if ($release.media -and $release.media.Count -gt 0) {
                $trackNum = 1
                foreach ($medium in $release.media) {
                    if ($medium.tracks) {
                        foreach ($track in $medium.tracks) {
                            $mbDuration = Format-Duration -Milliseconds $track.length

                            # We don't have our segmentation durations in the JSON, so mark as N/A
                            $ourDuration = "N/A"
                            $diff = "N/A"

                            $trackTitle = $track.title
                            if ($track.recording -and $track.recording.title) {
                                $trackTitle = $track.recording.title
                            }

                            $report += "| $trackNum | $trackTitle | $mbDuration | $ourDuration | $diff |"
                            $trackNum++
                        }
                    }
                }
            }

            $report += ""

        } else {
            $report += "**Error:** Could not retrieve MusicBrainz release details"
            $report += ""
        }
    } elseif ($album.skipped) {
        $report += "**Skipped:** Single song (baseline track count = 0)"
        $report += ""
    } elseif ($album.error) {
        $report += "**Error:** $($album.error)"
        $report += ""
    }

    # AcousticBrainz coverage
    if ($album.acousticbrainz_total_tracks -gt 0) {
        $abPct = [math]::Round(($album.acousticbrainz_available / $album.acousticbrainz_total_tracks) * 100, 1)
        $report += "**AcousticBrainz Coverage:** $($album.acousticbrainz_available)/$($album.acousticbrainz_total_tracks) tracks ($abPct%)"
        $report += ""
    }

    # Links
    $report += "**MusicBrainz:** [View Release](https://musicbrainz.org/release/$($album.current_mbid))"

    if (-not $album.mbid_match -and $album.baseline_mbid) {
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

Write-Host "Report generated successfully!" -ForegroundColor Green
Write-Host "File: $OutputFile" -ForegroundColor Green
$sizeKB = [math]::Round((Get-Item $OutputFile).Length / 1KB, 2)
Write-Host "Size: $sizeKB kilobytes" -ForegroundColor Green
Write-Host ""
