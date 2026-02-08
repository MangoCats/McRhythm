# Transform the extracted loop body for the process_album_file function

$inputFile = "c:\Users\Mango Cat\Dev\McRhythm\loop_body_original.txt"
$outputFile = "c:\Users\Mango Cat\Dev\McRhythm\loop_body_transformed.txt"

$lines = Get-Content $inputFile

$transformed = @()
$inResultsPush = $false
$skipNextPrintln = $false
$skipNextContinue = $false

for ($i = 0; $i -lt $lines.Length; $i++) {
    $line = $lines[$i]

    # Skip if flagged
    if ($skipNextPrintln -and $line -match '^\s+println!\(\);\s*$') {
        Write-Host "Skipping flagged println!() at line $($i+1)"
        $skipNextPrintln = $false
        $skipNextContinue = $true
        continue
    }

    if ($skipNextContinue -and $line -match '^\s+continue;\s*$') {
        Write-Host "Skipping flagged continue; at line $($i+1)"
        $skipNextContinue = $false
        continue
    }

    # Skip the final "println!();" at the very end of the loop
    if ($i -eq ($lines.Length - 1) -and $line -match '^\s+println!\(\);\s*$') {
        Write-Host "Skipping final println!() at end of loop"
        continue
    }

    # Detect start of results.push
    if ($line -match 'results\.push\(ValidationResult \{') {
        $inResultsPush = $true
        $line = $line -replace 'results\.push\(ValidationResult \{', 'return ValidationResult {'
    }

    # Detect end of results.push (closing });)
    if ($inResultsPush -and $line -match '^\s+\}\);\s*$') {
        $line = $line -replace '\}\);', '}'
        $inResultsPush = $false
        $skipNextPrintln = $true
        Write-Host "Found end of return statement at line $($i+1), will skip next println!() and continue;"
    }

    # Remove 8 spaces of indentation (2 levels)
    if ($line.Length -ge 8) {
        $prefix = $line.Substring(0, 8)
        if ($prefix -eq '        ') {
            $line = $line.Substring(8)
        }
    }

    $transformed += $line
}

# Write transformed output
$transformed | Set-Content $outputFile

Write-Host "`nTransformed $($lines.Length) original lines into $($transformed.Length) lines"
Write-Host "Saved to $outputFile"
Write-Host "First line: $($transformed[0])"
Write-Host "Last 3 lines:"
Write-Host "  $($transformed[-3])"
Write-Host "  $($transformed[-2])"
Write-Host "  $($transformed[-1])"
