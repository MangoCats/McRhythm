# Analyze cascade patterns in the 6 regression albums

$json = Get-Content 'run29f_comparison_results.json' -Raw | ConvertFrom-Json
$regressions = $json | Where-Object { -not $_.mbid_match -and $_.matched -and $_.match_percentage -lt 75.0 }

foreach ($album in $regressions) {
    Write-Host ""
    Write-Host "=== $($album.artist) - $($album.album) ($($album.match_percentage)%) ===" -ForegroundColor Cyan
    Write-Host ""

    $errorTracks = @()
    for ($i = 0; $i -lt $album.tracks.Count; $i++) {
        $track = $album.tracks[$i]
        $errorMs = [math]::Abs($track.timing_error_ms)
        if ($errorMs -gt 15000) {
            $errorTracks += $i
            Write-Host "Track $($track.track_number): $($track.title)" -ForegroundColor Yellow
            Write-Host "  MB: $([math]::Floor($track.mb_duration_ms / 1000))s  Our: $([math]::Floor($track.our_duration_ms / 1000))s  Error: $([math]::Floor($errorMs / 1000))s"
        }
    }

    # Detect cascade regions (2+ consecutive tracks with >15s error)
    if ($errorTracks.Count -ge 2) {
        $cascades = @()
        $start = $errorTracks[0]
        $end = $start

        for ($i = 1; $i -lt $errorTracks.Count; $i++) {
            if ($errorTracks[$i] -eq $end + 1) {
                # Consecutive - extend cascade
                $end = $errorTracks[$i]
            } else {
                # Gap - record cascade if 2+ tracks
                if ($end - $start -ge 1) {
                    $cascades += "Tracks $($start+1)-$($end+1)"
                }
                $start = $errorTracks[$i]
                $end = $start
            }
        }

        # Final cascade
        if ($end - $start -ge 1) {
            $cascades += "Tracks $($start+1)-$($end+1)"
        }

        if ($cascades.Count -gt 0) {
            Write-Host ""
            Write-Host "CASCADE REGIONS: $($cascades -join ', ')" -ForegroundColor Red
        }
    }

    Write-Host ""
    Write-Host "Total tracks with >15s error: $($errorTracks.Count)" -ForegroundColor Magenta
}
