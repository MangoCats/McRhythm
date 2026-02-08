# Replace the main loop with refactored version calling process_album_file

$targetFile = "c:\Users\Mango Cat\Dev\McRhythm\wkmp-ai\examples\album_matcher_12.rs"

$lines = Get-Content $targetFile
Write-Host "File has $($lines.Length) lines"

# Find the loop start (line with "for (idx, file_path) in training_files")
$loopStart = -1
$loopEnd = -1

for ($i = 0; $i -lt $lines.Length; $i++) {
    if ($lines[$i] -match 'for \(idx, file_path\) in training_files') {
        $loopStart = $i
        Write-Host "Found loop start at line $($i+1): $($lines[$i].Substring(0,50))..."
    }

    # Find "// Write results" which comes after the loop
    if ($loopStart -ge 0 -and $lines[$i] -match '^\s+// Write results') {
        $loopEnd = $i - 2  # Go back 2 lines (past blank line and closing brace)
        Write-Host "Found '// Write results' at line $($i+1), loop ends at line $($loopEnd+1)"
        break
    }
}

if ($loopStart -eq -1 -or $loopEnd -eq -1) {
    Write-Host "ERROR: Could not find loop boundaries"
    exit 1
}

# Build new file content
$newLines = @()

# Add everything before the loop
$newLines += $lines[0..($loopStart-1)]

# Add refactored loop
$refactoredLoop = @"
    for (idx, file_path) in training_files.iter().enumerate() {
        if !file_path.exists() {
            println!("=== Album {}/{} ===", idx + 1, training_files.len());
            println!("File: {}", file_path.display());
            println!("  ERROR: File not found\n");
            continue;
        }

        println!("=== Album {}/{} ===", idx + 1, training_files.len());
        println!("File: {}", file_path.display());

        let result = process_album_file(
            file_path,
            &rate_limiter,
            threshold_db,
            min_duration_secs,
            match_tolerance_secs,
            &threshold_values,
            &min_duration_values,
        ).await;

        results.push(result);
        println!();
    }
"@

$newLines += $refactoredLoop -split "`n"

# Add everything after the loop (from loopEnd+1 which is the blank line before "// Write results")
$newLines += $lines[($loopEnd+1)..($lines.Length-1)]

Write-Host "New file has $($newLines.Length) lines (was $($lines.Length), diff: $($newLines.Length - $lines.Length))"
Write-Host "Removed loop body from lines $($loopStart+1) to $($loopEnd+1) ($($loopEnd - $loopStart + 1) lines)"

# Write to file
$newLines | Set-Content $targetFile

Write-Host "Updated $targetFile"
