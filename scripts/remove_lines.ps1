# Remove lines 2768-3199 from album_matcher_12.rs
$file = "c:\Users\Mango Cat\Dev\McRhythm\wkmp-ai\examples\album_matcher_12.rs"
$lines = Get-Content $file
$newLines = $lines[0..2766] + $lines[3199..($lines.Length-1)]
Set-Content -Path $file -Value $newLines
Write-Host "Removed lines 2768-3199"
