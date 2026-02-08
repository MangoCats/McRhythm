# Extract ID3 information from all files in long_files_list.txt

$outputFile = "id3_information_report.txt"
$inputFile = "long_files_list.txt"

# Read the file list
$lines = Get-Content $inputFile | Where-Object { $_.Trim() -ne "" }

# Initialize output
"ID3 Tag Information Report" | Out-File $outputFile
"Generated: $(Get-Date)" | Out-File $outputFile -Append
"=" * 80 | Out-File $outputFile -Append
"" | Out-File $outputFile -Append

$fileCount = 0
$filesWithID3 = 0
$filesWithoutID3 = 0
$filesNotFound = 0

foreach ($line in $lines) {
    # Extract file path (remove the timestamp prefix)
    if ($line -match '\] (.+)$') {
        $filePath = $matches[1]
        $fileCount++

        Write-Host "Processing ($fileCount/$($lines.Count)): $filePath"

        # Check if file exists
        if (-not (Test-Path $filePath)) {
            "[$fileCount] FILE NOT FOUND: $filePath" | Out-File $outputFile -Append
            "" | Out-File $outputFile -Append
            $filesNotFound++
            continue
        }

        # Use ffprobe to extract metadata
        $ffprobeOutput = & ffprobe -v quiet -show_format -show_streams -of json "$filePath" 2>&1

        if ($LASTEXITCODE -eq 0) {
            $metadata = $ffprobeOutput | ConvertFrom-Json

            # Extract tags from format section
            $tags = $metadata.format.tags

            "[$fileCount] $filePath" | Out-File $outputFile -Append
            "-" * 80 | Out-File $outputFile -Append

            if ($tags) {
                $filesWithID3++

                # Common ID3 tags
                $tagFields = @(
                    "title",
                    "artist",
                    "album",
                    "album_artist",
                    "date",
                    "year",
                    "track",
                    "disc",
                    "genre",
                    "comment",
                    "composer",
                    "performer",
                    "publisher",
                    "copyright",
                    "encoder",
                    "encoded_by",
                    "ISRC",
                    "BARCODE",
                    "CATALOGNUMBER",
                    "MusicBrainz Album Id",
                    "MusicBrainz Artist Id",
                    "MusicBrainz Album Artist Id",
                    "MusicBrainz Release Track Id",
                    "MUSICBRAINZ_TRACKID",
                    "MUSICBRAINZ_ALBUMID",
                    "MUSICBRAINZ_ARTISTID",
                    "MUSICBRAINZ_ALBUMARTISTID",
                    "MUSICBRAINZ_RELEASEGROUPID"
                )

                $foundAny = $false
                foreach ($field in $tagFields) {
                    # Check both uppercase and lowercase versions
                    $value = $null
                    if ($tags.PSObject.Properties.Name -contains $field) {
                        $value = $tags.$field
                    } elseif ($tags.PSObject.Properties.Name -contains $field.ToUpper()) {
                        $value = $tags.($field.ToUpper())
                    } elseif ($tags.PSObject.Properties.Name -contains $field.ToLower()) {
                        $value = $tags.($field.ToLower())
                    }

                    if ($value) {
                        "  ${field}: $value" | Out-File $outputFile -Append
                        $foundAny = $true
                    }
                }

                # List any other tags not in the common list
                foreach ($prop in $tags.PSObject.Properties) {
                    if ($tagFields -notcontains $prop.Name -and
                        $tagFields.ToUpper() -notcontains $prop.Name.ToUpper() -and
                        $tagFields.ToLower() -notcontains $prop.Name.ToLower()) {
                        "  $($prop.Name): $($prop.Value)" | Out-File $outputFile -Append
                        $foundAny = $true
                    }
                }

                if (-not $foundAny) {
                    "  (No ID3 tags found)" | Out-File $outputFile -Append
                    $filesWithID3--
                    $filesWithoutID3++
                }
            } else {
                "  (No ID3 tags found)" | Out-File $outputFile -Append
                $filesWithoutID3++
            }

        } else {
            "  ERROR: Could not read file metadata" | Out-File $outputFile -Append
            $filesWithoutID3++
        }

        "" | Out-File $outputFile -Append
    }
}

# Summary
"=" * 80 | Out-File $outputFile -Append
"SUMMARY" | Out-File $outputFile -Append
"=" * 80 | Out-File $outputFile -Append
"Total files processed: $fileCount" | Out-File $outputFile -Append
"Files with ID3 tags: $filesWithID3" | Out-File $outputFile -Append
"Files without ID3 tags: $filesWithoutID3" | Out-File $outputFile -Append
"Files not found: $filesNotFound" | Out-File $outputFile -Append

Write-Host ""
Write-Host "Complete! Results written to $outputFile"
Write-Host "Total files: $fileCount"
Write-Host "Files with ID3 tags: $filesWithID3"
Write-Host "Files without ID3 tags: $filesWithoutID3"
Write-Host "Files not found: $filesNotFound"
