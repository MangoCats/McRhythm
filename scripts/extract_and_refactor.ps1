# Extract loop body from album_matcher_11.rs and prepare for refactoring

$sourceFile = "c:\Users\Mango Cat\Dev\McRhythm\wkmp-ai\examples\album_matcher_11.rs"
$targetFile = "c:\Users\Mango Cat\Dev\McRhythm\wkmp-ai\examples\album_matcher_12.rs"

# Read the source file
$lines = Get-Content $sourceFile

# Extract the loop body (lines 2526-2955, which is indices 2525-2954)
# This is everything from "let reconciled = ..." to the final "println!();" before the closing brace
$loopBodyStart = 2525  # Line 2526 (0-indexed)
$loopBodyEnd = 2954     # Line 2955 (0-indexed)

$loopBody = $lines[$loopBodyStart..$loopBodyEnd]

Write-Host "Extracted $($loopBody.Length) lines from loop body"
Write-Host "First line: $($loopBody[0])"
Write-Host "Last line: $($loopBody[-1])"

# Write to a temp file for inspection
$loopBody | Set-Content "c:\Users\Mango Cat\Dev\McRhythm\loop_body_original.txt"
Write-Host "Saved original loop body to loop_body_original.txt"
