# Plan Usage Tracking Script for Context Engineering Metrics (PLAN003)
# Usage: .\track_plan_usage.ps1

$OutputFile = "project_management\metrics\plan_usage_snapshot_$(Get-Date -Format 'yyyyMMdd').txt"
$CsvFile = "project_management\metrics\plan_usage_log.csv"
$Date = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
$SevenDaysAgo = (Get-Date).AddDays(-7)

# Create metrics directory if it doesn't exist
New-Item -ItemType Directory -Force -Path "project_management\metrics" | Out-Null

# Generate text report
@"
Plan Usage Tracking Snapshot
Generated: $Date
---

"@ | Set-Content -Path $OutputFile

# Count total PLAN folders
$PlanFolders = Get-ChildItem -Path wip -Directory -Filter "PLAN*" -ErrorAction SilentlyContinue
$TotalPlans = ($PlanFolders | Measure-Object).Count

"Total PLAN### folders in wip/: $TotalPlans" | Add-Content -Path $OutputFile
$PlanFolders | ForEach-Object { $_.FullName } | Add-Content -Path $OutputFile

"`n---" | Add-Content -Path $OutputFile
"Specification Issues Detected (All Plans):" | Add-Content -Path $OutputFile
"" | Add-Content -Path $OutputFile

# Count issues by severity
$CriticalCount = 0
$HighCount = 0
$MediumCount = 0
$LowCount = 0

$PlanFolders | ForEach-Object {
    $IssueFile = Join-Path $_.FullName "01_specification_issues.md"
    if (Test-Path $IssueFile) {
        $Content = Get-Content $IssueFile -Raw
        $CriticalCount += ([regex]::Matches($Content, "PRIORITY: CRITICAL")).Count
        $HighCount += ([regex]::Matches($Content, "PRIORITY: HIGH")).Count
        $MediumCount += ([regex]::Matches($Content, "PRIORITY: MEDIUM")).Count
        $LowCount += ([regex]::Matches($Content, "PRIORITY: LOW")).Count
    }
}

@"
CRITICAL: $CriticalCount
HIGH: $HighCount
MEDIUM: $MediumCount
LOW: $LowCount

---
Critical Issues Detail:
"@ | Add-Content -Path $OutputFile

$PlanFolders | ForEach-Object {
    $IssueFile = Join-Path $_.FullName "01_specification_issues.md"
    if (Test-Path $IssueFile) {
        Select-String -Path $IssueFile -Pattern "PRIORITY: CRITICAL" -Context 2,0 |
            ForEach-Object { "$($IssueFile): $($_.Line)" }
    }
} | Add-Content -Path $OutputFile

"`n---" | Add-Content -Path $OutputFile
"High Issues Detail:" | Add-Content -Path $OutputFile

$PlanFolders | ForEach-Object {
    $IssueFile = Join-Path $_.FullName "01_specification_issues.md"
    if (Test-Path $IssueFile) {
        Select-String -Path $IssueFile -Pattern "PRIORITY: HIGH" -Context 2,0 |
            ForEach-Object { "$($IssueFile): $($_.Line)" }
    }
} | Add-Content -Path $OutputFile

"`n---" | Add-Content -Path $OutputFile
"Plans Created/Modified in Last 7 Days:" | Add-Content -Path $OutputFile

$NewPlans = Get-ChildItem -Path wip -Directory -Filter "PLAN*" -ErrorAction SilentlyContinue |
    Where-Object { $_.LastWriteTime -gt $SevenDaysAgo }

$NewPlans | ForEach-Object { $_.FullName } | Add-Content -Path $OutputFile
$NewPlansCount = ($NewPlans | Measure-Object).Count

# Create CSV if it doesn't exist
if (-not (Test-Path $CsvFile)) {
    "Date,Total_Plans,Critical_Issues,High_Issues,Medium_Issues,Low_Issues,New_Plans_Last_7d" |
        Set-Content -Path $CsvFile
}

# Append to CSV
"$(Get-Date -Format 'yyyy-MM-dd'),$TotalPlans,$CriticalCount,$HighCount,$MediumCount,$LowCount,$NewPlansCount" |
    Add-Content -Path $CsvFile

# Print summary to console
Write-Host "=== Plan Usage Tracking Complete ===" -ForegroundColor Green
Write-Host "Total Plans: $TotalPlans"
Write-Host "Issues Found: C:$CriticalCount H:$HighCount M:$MediumCount L:$LowCount"
Write-Host "New Plans (7d): $NewPlansCount"
Write-Host "Report: $OutputFile"
Write-Host "CSV Log: $CsvFile"
