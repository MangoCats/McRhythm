$json = Get-Content 'run29f_comparison_results.json' -Raw | ConvertFrom-Json
Write-Host "Total albums:" $json.Count
$first = $json[0]
Write-Host "First album:" $first.artist "-" $first.album
Write-Host "Tracks count:" $first.tracks.Count
Write-Host "Has release_metadata:" ($null -ne $first.release_metadata)
if ($first.tracks.Count -gt 0) {
    $track = $first.tracks[0]
    Write-Host "Sample track:" $track.title
    Write-Host "  MB duration:" $track.mb_duration_ms "ms"
    Write-Host "  Our duration:" $track.our_duration_ms "ms"
    Write-Host "  Error:" $track.timing_error_ms "ms"
}
if ($null -ne $first.release_metadata) {
    Write-Host "Release metadata:"
    Write-Host "  Title:" $first.release_metadata.title
    Write-Host "  Artist:" $first.release_metadata.artist
    Write-Host "  Date:" $first.release_metadata.release_date
    Write-Host "  Country:" $first.release_metadata.country
    Write-Host "  Label:" $first.release_metadata.label
}
