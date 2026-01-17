$content = Get-Content 'c:\Users\Mango Cat\Dev\McRhythm\album_matcher_output_run16.txt'
$currentAlbum = 0
$albums = @{}

foreach ($line in $content) {
    if ($line -match '=== Album (\d+)/200 ===') {
        $currentAlbum = [int]$matches[1]
        $albums[$currentAlbum] = @{
            SearchArtist = ''
            SearchAlbum = ''
            WinningEdition = 0
            MatchPct = 0
            Editions = @{}
        }
    }

    if ($line -match 'File: .+\\([^\\]+)\\([^\\]+)\.mp3' -and $currentAlbum -gt 0) {
        $albums[$currentAlbum].FileArtist = $matches[1]
        $albums[$currentAlbum].FileAlbum = $matches[2]
    }

    if ($line -match '^\s+Artist: (.+) \(source:' -and $currentAlbum -gt 0) {
        $albums[$currentAlbum].SearchArtist = $matches[1]
    }

    if ($line -match '^\s+Album: (.+) \(source:' -and $currentAlbum -gt 0) {
        $albums[$currentAlbum].SearchAlbum = $matches[1]
    }

    if ($line -match '\[Edition (\d+)/\d+\] (.+) - (.+) \((\d+) tracks' -and $currentAlbum -gt 0) {
        $edNum = [int]$matches[1]
        $artist = $matches[2]
        $album = $matches[3]
        if (-not $albums[$currentAlbum].Editions.ContainsKey($edNum)) {
            $albums[$currentAlbum].Editions[$edNum] = @{Artist = $artist; Album = $album}
        }
    }

    if ($line -match 'Parallel processing complete\. Best: ([\d.]+)% from edition (\d+)' -and $currentAlbum -gt 0) {
        $albums[$currentAlbum].MatchPct = [double]$matches[1]
        $albums[$currentAlbum].WinningEdition = [int]$matches[2]
    }
}

Write-Output "Album | Search Artist | Search Album | Pct | Winning Artist | Winning Album | OK"
Write-Output "------|---------------|--------------|-----|----------------|---------------|---"

foreach ($num in $albums.Keys | Sort-Object) {
    $a = $albums[$num]
    $winEd = $a.WinningEdition
    $winArtist = ''
    $winAlbum = ''

    if ($a.Editions.ContainsKey($winEd)) {
        $winArtist = $a.Editions[$winEd].Artist
        $winAlbum = $a.Editions[$winEd].Album
    }

    $searchArtist = if ($a.SearchArtist) { $a.SearchArtist } else { $a.FileArtist }
    $searchAlbum = if ($a.SearchAlbum) { $a.SearchAlbum } else { $a.FileAlbum }

    $firstWord = $searchArtist.Split(' ')[0].ToLower()
    $artistMatch = $winArtist.ToLower().Contains($firstWord) -or $firstWord.Length -lt 3
    $matchSymbol = if ($artistMatch) { "Y" } else { "?" }

    $line = "{0} | {1} | {2} | {3:N0} | {4} | {5} | {6}" -f $num, $searchArtist, $searchAlbum, $a.MatchPct, $winArtist, $winAlbum, $matchSymbol
    Write-Output $line
}
