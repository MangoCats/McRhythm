# Document Size Tracking Script for Context Engineering Metrics (PLAN003)
# Usage: .\track_doc_sizes.ps1 -WeekNumber <number>
# Example: .\track_doc_sizes.ps1 -WeekNumber 1

param(
    [Parameter(Mandatory=$true)]
    [int]$WeekNumber
)

$OutputFile = "project_management\metrics\doc_sizes_week_$WeekNumber.txt"
$CsvFile = "project_management\metrics\document_size_log.csv"
$Date = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
$SevenDaysAgo = (Get-Date).AddDays(-7)

# Create metrics directory if it doesn't exist
New-Item -ItemType Directory -Force -Path "project_management\metrics" | Out-Null

# Generate text report
@"
Document Size Tracking - Week $WeekNumber
Generated: $Date
Period: Last 7 days from $($SevenDaysAgo.ToString("yyyy-MM-dd")) to $(Get-Date -Format "yyyy-MM-dd")
---

"@ | Set-Content -Path $OutputFile

# Find all .md files modified in last 7 days
"All .md files created/modified in last 7 days:" | Add-Content -Path $OutputFile

Get-ChildItem -Path docs, wip, workflows -Recurse -Filter "*.md" |
    Where-Object { $_.LastWriteTime -gt $SevenDaysAgo } |
    ForEach-Object {
        $lines = (Get-Content $_.FullName | Measure-Object -Line).Lines
        "$lines $($_.FullName)"
    } | Sort-Object { [int]($_ -split ' ')[0] } -Descending |
    Add-Content -Path $OutputFile

"`n---" | Add-Content -Path $OutputFile
"Summary files only (modular documents):" | Add-Content -Path $OutputFile

Get-ChildItem -Path docs, wip -Recurse -Filter "00_SUMMARY.md" |
    Where-Object { $_.LastWriteTime -gt $SevenDaysAgo } |
    ForEach-Object {
        $lines = (Get-Content $_.FullName | Measure-Object -Line).Lines
        "$lines $($_.FullName)"
    } | Add-Content -Path $OutputFile

"`n---" | Add-Content -Path $OutputFile
"Full documents (modular - for archival comparison):" | Add-Content -Path $OutputFile

Get-ChildItem -Path docs, wip -Recurse -Filter "FULL_DOCUMENT.md" |
    Where-Object { $_.LastWriteTime -gt $SevenDaysAgo } |
    ForEach-Object {
        $lines = (Get-Content $_.FullName | Measure-Object -Line).Lines
        "$lines $($_.FullName)"
    } | Add-Content -Path $OutputFile

# Create CSV if it doesn't exist
if (-not (Test-Path $CsvFile)) {
    "Week,Date,Document,Lines,Type,Notes" | Set-Content -Path $CsvFile
}

# Append to CSV
Get-ChildItem -Path docs, wip, workflows -Recurse -Filter "*.md" |
    Where-Object { $_.LastWriteTime -gt $SevenDaysAgo } |
    ForEach-Object {
        $lines = (Get-Content $_.FullName | Measure-Object -Line).Lines
        $type = "single"

        if ($_.Name -eq "00_SUMMARY.md") {
            $type = "modular_summary"
        } elseif ($_.Name -eq "FULL_DOCUMENT.md") {
            $type = "modular_full"
        }

        "$WeekNumber,$(Get-Date -Format 'yyyy-MM-dd'),$($_.FullName),$lines,$type,"
    } | Add-Content -Path $CsvFile

"`nData appended to: $CsvFile" | Add-Content -Path $OutputFile
"Report generated: $OutputFile" | Add-Content -Path $OutputFile

# Print summary to console
$TotalDocs = (Get-ChildItem -Path docs, wip, workflows -Recurse -Filter "*.md" |
    Where-Object { $_.LastWriteTime -gt $SevenDaysAgo }).Count

Write-Host "=== Week $WeekNumber Document Size Tracking Complete ===" -ForegroundColor Green
Write-Host "Total documents found: $TotalDocs"
Write-Host "Report: $OutputFile"
Write-Host "CSV Log: $CsvFile"
