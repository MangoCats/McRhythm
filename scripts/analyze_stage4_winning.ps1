$content = Get-Content 'c:\Users\Mango Cat\Dev\McRhythm\album_matcher_output_run16.txt'
$currentAlbum = 0
$currentFile = ''
$winningEdition = @{}
$stage4Scores = @{}
$finalScores = @{}

foreach ($line in $content) {
    if ($line -match '=== Album (\d+)/200 ===') {
        $currentAlbum = [int]$matches[1]
        $stage4Scores[$currentAlbum] = @{}
    }
    if ($line -match 'File: (.+\.mp3)') {
        $currentFile = $matches[1]
    }
    # Track Stage 4 scores during processing for each edition
    if ($line -match '\[Edition (\d+)/\d+\] New best: ([\d.]+)% via guided quiet spots') {
        $ed = [int]$matches[1]
        $score = [double]$matches[2]
        if (-not $stage4Scores[$currentAlbum].ContainsKey($ed) -or $score -gt $stage4Scores[$currentAlbum][$ed]) {
            $stage4Scores[$currentAlbum][$ed] = $score
        }
    }
    # Capture final result
    if ($line -match 'Parallel processing complete\. Best: ([\d.]+)% from edition (\d+)') {
        $finalScores[$currentAlbum] = [double]$matches[1]
        $winningEdition[$currentAlbum] = [int]$matches[2]
    }
}

Write-Output "Albums where Stage 4 beat final score on WINNING edition:"
Write-Output "==========================================================="
foreach ($album in $finalScores.Keys | Sort-Object) {
    $final = $finalScores[$album]
    $winEd = $winningEdition[$album]
    if ($stage4Scores[$album].ContainsKey($winEd)) {
        $s4Score = $stage4Scores[$album][$winEd]
        if ($s4Score -gt $final) {
            $penalized = $s4Score * 0.75
            Write-Output ("Album {0}: Edition {1} - Stage4={2:N1}% (penalized={3:N1}%), Final={4:N1}% (diff=+{5:N1}%)" -f $album, $winEd, $s4Score, $penalized, $final, ($s4Score - $final))
        }
    }
}
