# Investigate the 6 likely regressions (<75% match quality)

$json = Get-Content 'run29f_comparison_results.json' -Raw | ConvertFrom-Json

# Filter to low quality MBID changes
$regressions = $json | Where-Object {
    -not $_.mbid_match -and
    $_.matched -and
    $_.match_percentage -lt 75.0
}

Write-Host ""
Write-Host "=== Detailed Regression Analysis ===" -ForegroundColor Red
Write-Host ""
Write-Host "Found $($regressions.Count) albums with <75% match quality after MBID change"
Write-Host ""

foreach ($album in $regressions) {
    Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor DarkGray
    Write-Host "ALBUM: $($album.artist) - $($album.album)" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "File: $($album.path)"
    Write-Host ""
    Write-Host "BASELINE MBID: $($album.baseline_mbid)"
    Write-Host "  Track count: $($album.baseline_tracks)"
    Write-Host ""
    Write-Host "CURRENT MBID:  $($album.current_mbid)"
    Write-Host "  Track count: $($album.current_tracks)"
    Write-Host "  Match quality: $($album.match_percentage)%" -ForegroundColor Red
    Write-Host ""

    $track_diff = $album.current_tracks - $album.baseline_tracks
    if ($track_diff -ne 0) {
        $sign = if ($track_diff -gt 0) { "+" } else { "" }
        Write-Host "TRACK COUNT CHANGE: $sign$track_diff tracks" -ForegroundColor Magenta
    } else {
        Write-Host "Track count: SAME" -ForegroundColor Green
    }

    Write-Host ""
    Write-Host "AcousticBrainz Coverage:"
    Write-Host "  Total tracks: $($album.acousticbrainz_total_tracks)"
    Write-Host "  Available: $($album.acousticbrainz_available)"
    Write-Host "  Missing: $($album.acousticbrainz_missing)"

    if ($album.acousticbrainz_total_tracks -gt 0) {
        $ab_pct = ($album.acousticbrainz_available / $album.acousticbrainz_total_tracks) * 100
        Write-Host "  Coverage: $([math]::Round($ab_pct, 1))%"
    }

    Write-Host ""
    Write-Host "MusicBrainz URLs:"
    Write-Host "  Baseline: https://musicbrainz.org/release/$($album.baseline_mbid)"
    Write-Host "  Current:  https://musicbrainz.org/release/$($album.current_mbid)"
    Write-Host ""
}

Write-Host "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━" -ForegroundColor DarkGray
Write-Host ""
Write-Host "ANALYSIS SUMMARY:" -ForegroundColor Cyan
Write-Host ""

$track_count_changes = $regressions | Where-Object { $_.baseline_tracks -ne $_.current_tracks }
$same_track_count = $regressions | Where-Object { $_.baseline_tracks -eq $_.current_tracks }

Write-Host "Track count changed: $($track_count_changes.Count) albums"
Write-Host "Track count same: $($same_track_count.Count) albums"
Write-Host ""

$avg_quality = ($regressions | Measure-Object -Property match_percentage -Average).Average
Write-Host "Average match quality: $([math]::Round($avg_quality, 1))%"
Write-Host ""

Write-Host "NEXT STEPS:" -ForegroundColor Yellow
Write-Host "1. Visit MusicBrainz URLs above to compare editions"
Write-Host "2. Check if current MBID is actually a better match (different remaster, etc.)"
Write-Host "3. Determine if artist filter is too aggressive for these albums"
Write-Host "4. Consider adjusting artist filter threshold if needed"
Write-Host ""
