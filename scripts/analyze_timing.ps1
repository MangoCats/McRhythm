$content = Get-Content 'c:\Users\Mango Cat\Dev\McRhythm\album_matcher_output_run16.txt'
$albums = @{}
$currentAlbum = 0
$currentState = ''

foreach ($line in $content) {
    # Match timestamp pattern: 2025-11-22T04:07:10.756526Z
    $ts = $null
    if ($line -match '(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d+)Z') {
        $ts = [datetime]::ParseExact($matches[1], "yyyy-MM-ddTHH:mm:ss.ffffff", $null)
    }

    if ($line -match '=== Album (\d+)/200 ===') {
        $currentAlbum = [int]$matches[1]
        $albums[$currentAlbum] = @{
            StartTime = $ts
            DecodeStart = $null
            DecodeEnd = $null
            MBStart = $null
            MBEnd = $null
            AnalysisEnd = $null
            FileDuration = 0
            MatchPct = 0
            MeanError = 0
        }
    }

    if ($line -match 'Decoding\.\.\.' -and $currentAlbum -gt 0 -and $ts) {
        $albums[$currentAlbum].DecodeStart = $ts
    }

    if ($line -match 'Decoded: \d+ samples at \d+ Hz \((\d+\.?\d*) mins\)' -and $currentAlbum -gt 0 -and $ts) {
        $albums[$currentAlbum].DecodeEnd = $ts
        $albums[$currentAlbum].FileDuration = [double]$matches[1]
    }

    if ($line -match 'Fetching MusicBrainz data' -and $currentAlbum -gt 0 -and $ts) {
        $albums[$currentAlbum].MBStart = $ts
    }

    if ($line -match 'Grouped into \d+ unique editions' -and $currentAlbum -gt 0 -and $ts) {
        $albums[$currentAlbum].MBEnd = $ts
    }

    if ($line -match 'Parallel processing complete\. Best: ([\d.]+)% from edition \d+ \(mean error: ([\d.]+)s\)' -and $currentAlbum -gt 0 -and $ts) {
        $albums[$currentAlbum].AnalysisEnd = $ts
        $albums[$currentAlbum].MatchPct = [double]$matches[1]
        $albums[$currentAlbum].MeanError = [double]$matches[2]
    }
}

Write-Output "Album | Analysis | File Dur | Ratio | Decode | Dec% | MB Lookup | MB% | Match% | Mean Err"
Write-Output "------|----------|----------|-------|--------|------|-----------|-----|--------|----------"

foreach ($num in $albums.Keys | Sort-Object) {
    $a = $albums[$num]
    if ($a.AnalysisEnd -and $a.StartTime) {
        $analysisTime = ($a.AnalysisEnd - $a.StartTime).TotalMinutes
        $fileDur = $a.FileDuration
        $ratio = if ($fileDur -gt 0) { $analysisTime / $fileDur } else { 0 }

        $decodeTime = 0
        $decodePct = 0
        if ($a.DecodeEnd -and $a.DecodeStart) {
            $decodeTime = ($a.DecodeEnd - $a.DecodeStart).TotalMinutes
            $decodePct = if ($analysisTime -gt 0) { ($decodeTime / $analysisTime) * 100 } else { 0 }
        }

        $mbTime = 0
        $mbPct = 0
        if ($a.MBEnd -and $a.MBStart) {
            $mbTime = ($a.MBEnd - $a.MBStart).TotalMinutes
            $mbPct = if ($analysisTime -gt 0) { ($mbTime / $analysisTime) * 100 } else { 0 }
        }

        $line = "{0,5} | {1,7:N1}m | {2,7:N1}m | {3,5:N2} | {4,6:N1}m | {5,4:N0}% | {6,9:N1}m | {7,3:N0}% | {8,5:N1}% | {9,7:N2}s" -f `
            $num, $analysisTime, $fileDur, $ratio, $decodeTime, $decodePct, $mbTime, $mbPct, $a.MatchPct, $a.MeanError
        Write-Output $line
    }
}
