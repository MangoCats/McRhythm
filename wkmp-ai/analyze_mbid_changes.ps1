# Analyze MBID changes to determine improvements vs regressions

$json = Get-Content 'run29f_comparison_results.json' -Raw | ConvertFrom-Json

# Filter to albums with MBID changes
$mbid_changes = $json | Where-Object { -not $_.mbid_match -and $_.matched }

Write-Host ""
Write-Host "=== MBID Change Analysis ($($mbid_changes.Count) albums) ===" -ForegroundColor Cyan
Write-Host ""

# Categorize by match percentage
$perfect = $mbid_changes | Where-Object { $_.match_percentage -eq 100.0 }
$high = $mbid_changes | Where-Object { $_.match_percentage -ge 90.0 -and $_.match_percentage -lt 100.0 }
$medium = $mbid_changes | Where-Object { $_.match_percentage -ge 75.0 -and $_.match_percentage -lt 90.0 }
$low = $mbid_changes | Where-Object { $_.match_percentage -lt 75.0 }

Write-Host "Match Quality Distribution:" -ForegroundColor Yellow
Write-Host "  100% match: $($perfect.Count) albums"
Write-Host "  90-99% match: $($high.Count) albums"
Write-Host "  75-89% match: $($medium.Count) albums"
Write-Host "  <75% match: $($low.Count) albums"
Write-Host ""

# Show sample of each category
Write-Host "=== Perfect Match MBID Changes (100%) ===" -ForegroundColor Green
$perfect | Select-Object -First 10 | ForEach-Object {
    Write-Host "  $($_.artist) - $($_.album)"
    Write-Host "    Tracks: $($_.baseline_tracks) (baseline) -> $($_.current_tracks) (current)"
    Write-Host "    MBID: $($_.baseline_mbid) -> $($_.current_mbid)"
    Write-Host ""
}
if ($perfect.Count -gt 10) {
    Write-Host "  ... and $($perfect.Count - 10) more"
    Write-Host ""
}

Write-Host "=== High Quality MBID Changes (90-99%) ===" -ForegroundColor Green
$high | Select-Object -First 10 | ForEach-Object {
    Write-Host "  $($_.artist) - $($_.album) ($($_.match_percentage)%)"
    Write-Host "    Tracks: $($_.baseline_tracks) (baseline) -> $($_.current_tracks) (current)"
    Write-Host "    MBID: $($_.baseline_mbid) -> $($_.current_mbid)"
    Write-Host ""
}
if ($high.Count -gt 10) {
    Write-Host "  ... and $($high.Count - 10) more"
    Write-Host ""
}

Write-Host "=== Medium Quality MBID Changes (75-89%) ===" -ForegroundColor Yellow
$medium | ForEach-Object {
    Write-Host "  $($_.artist) - $($_.album) ($($_.match_percentage)%)"
    Write-Host "    Tracks: $($_.baseline_tracks) (baseline) -> $($_.current_tracks) (current)"
    Write-Host "    MBID: $($_.baseline_mbid) -> $($_.current_mbid)"
    Write-Host ""
}

Write-Host "=== Low Quality MBID Changes (<75%) ===" -ForegroundColor Red
$low | ForEach-Object {
    Write-Host "  $($_.artist) - $($_.album) ($($_.match_percentage)%)"
    Write-Host "    Tracks: $($_.baseline_tracks) (baseline) -> $($_.current_tracks) (current)"
    Write-Host "    MBID: $($_.baseline_mbid) -> $($_.current_mbid)"
    Write-Host ""
}

# Track count match analysis
$track_count_same = $mbid_changes | Where-Object { $_.baseline_tracks -eq $_.current_tracks }
$track_count_diff = $mbid_changes | Where-Object { $_.baseline_tracks -ne $_.current_tracks }

Write-Host "=== Track Count Analysis ===" -ForegroundColor Cyan
Write-Host "  Same track count: $($track_count_same.Count) albums"
Write-Host "  Different track count: $($track_count_diff.Count) albums"
Write-Host ""

if ($track_count_diff.Count -gt 0) {
    Write-Host "Albums with different track counts:" -ForegroundColor Yellow
    $track_count_diff | ForEach-Object {
        $diff = $_.current_tracks - $_.baseline_tracks
        $sign = if ($diff -gt 0) { "+" } else { "" }
        Write-Host "  $($_.artist) - $($_.album): $($_.baseline_tracks) -> $($_.current_tracks) ($sign$diff tracks)"
    }
}

Write-Host ""
Write-Host "=== Summary ===" -ForegroundColor Cyan
Write-Host "Perfect matches (100%): $($perfect.Count) - LIKELY IMPROVEMENTS (same quality, different edition)"
Write-Host "High quality (90-99%): $($high.Count) - LIKELY IMPROVEMENTS (minor differences)"
Write-Host "Medium quality (75-89%): $($medium.Count) - NEED REVIEW (potential regressions)"
Write-Host "Low quality (<75%): $($low.Count) - LIKELY REGRESSIONS (significant differences)"
Write-Host ""

$likely_improvements = $perfect.Count + $high.Count
$need_review = $medium.Count
$likely_regressions = $low.Count

Write-Host "VERDICT:" -ForegroundColor Cyan
Write-Host "  Likely improvements: $likely_improvements ($([math]::Round(($likely_improvements / $mbid_changes.Count) * 100, 1))%)"
Write-Host "  Need manual review: $need_review ($([math]::Round(($need_review / $mbid_changes.Count) * 100, 1))%)"
Write-Host "  Likely regressions: $likely_regressions ($([math]::Round(($likely_regressions / $mbid_changes.Count) * 100, 1))%)"
Write-Host ""
