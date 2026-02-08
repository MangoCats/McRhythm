# Summarize ID3 tag information across all files

$outputFile = "id3_summary.txt"
$inputFile = "long_files_list.txt"

# Read the file list
$lines = Get-Content $inputFile | Where-Object { $_.Trim() -ne "" }

# Initialize counters
$tagStats = @{}
$filesWithMusicBrainz = 0
$totalFiles = 0

# Sample data for detailed examples
$sampleFiles = @()
$maxSamples = 10

foreach ($line in $lines) {
    if ($line -match '\] (.+)$') {
        $filePath = $matches[1]
        $totalFiles++

        if (-not (Test-Path $filePath)) {
            continue
        }

        # Use ffprobe to extract metadata
        $ffprobeOutput = & ffprobe -v quiet -show_format -of json "$filePath" 2>&1

        if ($LASTEXITCODE -eq 0) {
            $metadata = $ffprobeOutput | ConvertFrom-Json
            $tags = $metadata.format.tags

            if ($tags) {
                $hasMusicBrainz = $false
                $fileTagList = @()

                foreach ($prop in $tags.PSObject.Properties) {
                    $tagName = $prop.Name
                    $fileTagList += $tagName

                    # Count tag occurrences
                    if (-not $tagStats.ContainsKey($tagName)) {
                        $tagStats[$tagName] = 0
                    }
                    $tagStats[$tagName]++

                    # Check for MusicBrainz tags
                    if ($tagName -match "musicbrainz" -or $tagName -match "MUSICBRAINZ") {
                        $hasMusicBrainz = $true
                    }
                }

                if ($hasMusicBrainz) {
                    $filesWithMusicBrainz++
                }

                # Collect sample
                if ($sampleFiles.Count -lt $maxSamples) {
                    $sampleFiles += @{
                        Path = $filePath
                        Tags = $tags
                        TagNames = $fileTagList
                    }
                }
            }
        }
    }
}

# Write summary
"ID3 Tag Summary Report" | Out-File $outputFile
"Generated: $(Get-Date)" | Out-File $outputFile -Append
"=" * 80 | Out-File $outputFile -Append
"" | Out-File $outputFile -Append

"OVERVIEW" | Out-File $outputFile -Append
"-" * 80 | Out-File $outputFile -Append
"Total files analyzed: $totalFiles" | Out-File $outputFile -Append
"Files with MusicBrainz tags: $filesWithMusicBrainz" | Out-File $outputFile -Append
"Files without MusicBrainz tags: $($totalFiles - $filesWithMusicBrainz)" | Out-File $outputFile -Append
"" | Out-File $outputFile -Append

"TAG FREQUENCY" | Out-File $outputFile -Append
"-" * 80 | Out-File $outputFile -Append
"Tag occurrences across all files:" | Out-File $outputFile -Append
"" | Out-File $outputFile -Append

# Sort by frequency (most common first)
$sortedTags = $tagStats.GetEnumerator() | Sort-Object -Property Value -Descending
foreach ($tag in $sortedTags) {
    $percentage = [math]::Round(($tag.Value / $totalFiles) * 100, 1)
    "  $($tag.Key): $($tag.Value) files ($percentage%)" | Out-File $outputFile -Append
}

"" | Out-File $outputFile -Append
"" | Out-File $outputFile -Append

"SAMPLE FILES (First 10)" | Out-File $outputFile -Append
"-" * 80 | Out-File $outputFile -Append

foreach ($sample in $sampleFiles) {
    "" | Out-File $outputFile -Append
    "File: $($sample.Path)" | Out-File $outputFile -Append
    "Tags present:" | Out-File $outputFile -Append
    foreach ($tagName in $sample.TagNames | Sort-Object) {
        $value = $sample.Tags.$tagName
        # Truncate long values
        if ($value.Length -gt 100) {
            $value = $value.Substring(0, 97) + "..."
        }
        "  $tagName = $value" | Out-File $outputFile -Append
    }
}

Write-Host ""
Write-Host "Summary complete! Results written to $outputFile"
Write-Host "Total files analyzed: $totalFiles"
Write-Host "Files with MusicBrainz tags: $filesWithMusicBrainz"
Write-Host "Unique tag types found: $($tagStats.Count)"
