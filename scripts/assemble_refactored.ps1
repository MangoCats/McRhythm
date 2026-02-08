# Assemble the refactored album_matcher_12.rs

$targetFile = "c:\Users\Mango Cat\Dev\McRhythm\wkmp-ai\examples\album_matcher_12.rs"
$loopBodyFile = "c:\Users\Mango Cat\Dev\McRhythm\loop_body_transformed.txt"

# Read files
$targetLines = Get-Content $targetFile
$loopBody = Get-Content $loopBodyFile

Write-Host "Target file has $($targetLines.Length) lines"
Write-Host "Loop body has $($loopBody.Length) lines"

# Step 1: Find and replace the placeholder function body
# The function is at line 2432-2463 (roughly)
# Find "async fn process_album_file(" and replace everything until the closing brace

$functionStart = -1
$functionEnd = -1
$placeholderStart = -1
$placeholderEnd = -1

for ($i = 0; $i -lt $targetLines.Length; $i++) {
    if ($targetLines[$i] -match '^async fn process_album_file\(') {
        $functionStart = $i
        Write-Host "Found function start at line $($i+1)"
    }

    # Find the placeholder return statement (starts with "    // This function will contain")
    if ($functionStart -ge 0 -and $targetLines[$i] -match '// This function will contain') {
        $placeholderStart = $i
        Write-Host "Found placeholder comment at line $($i+1)"
    }

    # Find the closing brace of the function (after placeholder)
    if ($placeholderStart -ge 0 -and $functionEnd -eq -1 -and $targetLines[$i] -eq '}') {
        $functionEnd = $i
        Write-Host "Found function end at line $($i+1)"
        break
    }
}

if ($placeholderStart -eq -1 -or $functionEnd -eq -1) {
    Write-Host "ERROR: Could not find placeholder or function end"
    exit 1
}

# Build new file content
$newLines = @()

# Add everything before the placeholder
$newLines += $targetLines[0..($placeholderStart-1)]

# Add the transformed loop body with proper indentation (4 spaces)
foreach ($line in $loopBody) {
    if ($line.Trim() -eq '') {
        $newLines += ''
    } else {
        $newLines += "    $line"
    }
}

# Skip the placeholder and old function body, add everything after function end
$newLines += $targetLines[$functionEnd..($targetLines.Length-1)]

Write-Host "New file has $($newLines.Length) lines"

# Write to file
$newLines | Set-Content $targetFile

Write-Host "Updated $targetFile"
