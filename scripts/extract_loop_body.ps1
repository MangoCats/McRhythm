# Extract loop body from album_matcher_11.rs (lines 2525-2953)
# and transform it for the process_album_file function

$sourceFile = "c:\Users\Mango Cat\Dev\McRhythm\wkmp-ai\examples\album_matcher_11.rs"
$outputFile = "c:\Users\Mango Cat\Dev\McRhythm\loop_body_extracted.txt"

# Read source file
$lines = Get-Content $sourceFile

# Extract lines 2525-2953 (0-indexed so 2524-2952)
$loopBody = $lines[2524..2952]

# Transform the code:
# 1. Remove 2 levels of indentation (8 spaces)
# 2. Change "results.push(ValidationResult {" to "return ValidationResult {"
# 3. Remove lines that are just "println!();" followed by "continue;"

$transformed = @()
$skipNext = $false

for ($i = 0; $i -lt $loopBody.Length; $i++) {
    if ($skipNext) {
        $skipNext = $false
        continue
    }

    $line = $loopBody[$i]

    # Skip "println!();" followed by "continue;"
    if ($line -match '^\s+println!\(\);\s*$' -and $i+1 -lt $loopBody.Length -and $loopBody[$i+1] -match '^\s+continue;\s*$') {
        $skipNext = $true
        continue
    }

    # Skip standalone "continue;"
    if ($line -match '^\s+continue;\s*$') {
        continue
    }

    # Transform results.push to return
    if ($line -match '^\s+results\.push\(ValidationResult \{') {
        $line = $line -replace 'results\.push\(ValidationResult \{', 'return ValidationResult {'
    }

    # Remove closing }); for results.push -> return transformation
    if ($line -match '^\s+\}\);\s*$' -and $i -gt 0 -and $transformed[-1] -notmatch 'continue;') {
        $line = $line -replace '\}\);', '}'
    }

    # Remove 2 levels of indentation (8 spaces)
    if ($line.Length -ge 8 -and $line.Substring(0,8) -eq '        ') {
        $line = $line.Substring(8)
    }

    $transformed += $line
}

# Write to output file
$transformed | Set-Content $outputFile
Write-Host "Extracted and transformed $($transformed.Length) lines to $outputFile"
