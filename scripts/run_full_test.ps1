$timestamp = Get-Date -Format 'yyyyMMdd_HHmmss'
$consoleOutputFile = "stage6_overlap_full_test_console_$timestamp.txt"

Write-Host "================================================================================"
Write-Host "Starting Full 200-Album Test with Stage 6 Overlap Resolution"
Write-Host "================================================================================"
Write-Host ""
Write-Host "Console output will be saved to: $consoleOutputFile"
Write-Host "Debug logs will be in: wkmp-ai/test_run29f_full_YYYYMMDD_HHMMSS.log (created by test)"
Write-Host ""
Write-Host "Monitor test progress with:"
Write-Host "  Get-Content wkmp-ai\test_run29f_full_*.log -Wait -Tail 50"
Write-Host ""
Write-Host "================================================================================"
Write-Host ""

Set-Location wkmp-ai
$env:RUST_LOG = 'wkmp_ai::matching=debug'

# Run the test with --ignored flag to execute the long-running test
cargo test --release --test run29f_full_comparison_test -- --nocapture --test-threads=1 --ignored *> "..\$consoleOutputFile"

Write-Host ""
Write-Host "================================================================================"
Write-Host "Test completed!"
Write-Host "Console output: $consoleOutputFile"
Write-Host "Debug logs: wkmp-ai/test_run29f_full_*.log"
Write-Host "================================================================================"
