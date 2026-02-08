$timestamp = Get-Date -Format 'yyyyMMdd_HHmmss'
$outputFile = "stage6_overlap_resolution_full_test_$timestamp.txt"

Write-Host "Starting full 200-album test with debug logging"
Write-Host "Output file: $outputFile"
Write-Host ""

Set-Location wkmp-ai
$env:RUST_LOG = 'wkmp_ai::matching=debug'

cargo test --release --test run29f_full_comparison_test -- --nocapture --test-threads=1 *> "..\$outputFile"

Write-Host ""
Write-Host "Test completed. Output saved to: $outputFile"
