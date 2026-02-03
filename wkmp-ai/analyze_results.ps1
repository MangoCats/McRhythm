$json = Get-Content 'run29f_comparison_results.json' -Raw | ConvertFrom-Json
$total = $json.Count
$exact = ($json | Where-Object { $_.tracks_match -and $_.mbid_match }).Count
$mbid_changes = ($json | Where-Object { -not $_.mbid_match -and $_.matched }).Count
$track_changes = ($json | Where-Object { -not $_.tracks_match -and $_.matched }).Count
$failures = ($json | Where-Object { -not $_.matched -and -not $_.skipped }).Count
$skipped = ($json | Where-Object { $_.skipped }).Count
$tested = $total - $skipped

# AcousticBrainz stats
$ab_albums = ($json | Where-Object { $_.acousticbrainz_total_tracks -gt 0 }).Count
$ab_total = ($json | Measure-Object -Property acousticbrainz_total_tracks -Sum).Sum
$ab_available = ($json | Measure-Object -Property acousticbrainz_available -Sum).Sum
$ab_missing = ($json | Measure-Object -Property acousticbrainz_missing -Sum).Sum

Write-Host ""
Write-Host "=== Comparison Summary ===" -ForegroundColor Cyan
Write-Host "Total albums: $total"
Write-Host "Tested: $tested"
Write-Host "Skipped: $skipped"
Write-Host "Exact matches: $exact ($('{0:F1}' -f (($exact / $tested) * 100))%)"
Write-Host "Track count changes: $track_changes ($('{0:F1}' -f (($track_changes / $tested) * 100))%)"
Write-Host "MBID changes: $mbid_changes ($('{0:F1}' -f (($mbid_changes / $tested) * 100))%)"
Write-Host "Failures: $failures ($('{0:F1}' -f (($failures / $tested) * 100))%)"
Write-Host ""
Write-Host "=== AcousticBrainz Coverage ===" -ForegroundColor Cyan
Write-Host "Albums queried: $ab_albums"
Write-Host "Total recordings: $ab_total"
Write-Host "Available: $ab_available"
Write-Host "Missing: $ab_missing"
if ($ab_total -gt 0) {
    Write-Host "Availability: $('{0:F1}' -f (($ab_available / $ab_total) * 100))%"
}
Write-Host ""
