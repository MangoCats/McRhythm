$content = Get-Content 'c:\Users\Mango Cat\Dev\McRhythm\album_matcher_output_run16.txt'
$currentAlbum = 0
$finals = @{}
$stage4raws = @{}

foreach ($line in $content) {
    if ($line -match '=== Album (\d+)/200 ===') {
        $currentAlbum = [int]$matches[1]
        $stage4raws[$currentAlbum] = @()
    }
    if ($line -match 'Parallel processing complete\. Best: ([\d.]+)%') {
        $finals[$currentAlbum] = [double]$matches[1]
    }
    if ($line -match 'Stage 4 raw: ([\d.]+)%' -and $currentAlbum -gt 0) {
        $stage4raws[$currentAlbum] += [double]$matches[1]
    }
}

Write-Output "Albums where Max Stage4 Raw > Final Score:"
Write-Output "==========================================="
foreach ($album in $finals.Keys | Sort-Object) {
    $final = $finals[$album]
    $s4arr = $stage4raws[$album]
    if ($s4arr -and $s4arr.Count -gt 0) {
        $s4max = ($s4arr | Measure-Object -Maximum).Maximum
        if ($s4max -gt $final) {
            Write-Output ("Album {0}: Final={1:N1}%, Max Stage4 Raw={2:N1}% (diff=+{3:N1}%)" -f $album, $final, $s4max, ($s4max - $final))
        }
    }
}
