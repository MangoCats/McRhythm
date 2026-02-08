# Experiment Analysis - Compare experiment results to baseline
# Usage: .\analyze_experiment.ps1 -ExperimentFile "am\exp1a_results.json" -BaselineFile "am\run29f_baseline.json"

param(
    [Parameter(Mandatory=$true)]
    [string]$ExperimentFile,

    [Parameter(Mandatory=$true)]
    [string]$BaselineFile,

    [Parameter(Mandatory=$false)]
    [string]$OutputFile = $null
)

$ErrorActionPreference = "Stop"

function Write-Log {
    param([string]$Message)
    Write-Host $Message
}

# Load data
Write-Log "Loading experiment: $ExperimentFile"
$Experiment = Get-Content -Path $ExperimentFile -Raw | ConvertFrom-Json

Write-Log "Loading baseline: $BaselineFile"
$Baseline = Get-Content -Path $BaselineFile -Raw | ConvertFrom-Json

# Create lookup dictionary for baseline
$BaselineDict = @{}
foreach ($Entry in $Baseline) {
    $BaselineDict[$Entry.path] = $Entry
}

# Compare albums
$ExactMatches = 0
$MbidChanges = 0
$ClearlyBetter = 0
$Equivocal = 0
$ClearlyWorse = 0

$DetailedResults = @()

foreach ($ExpEntry in $Experiment) {
    $Path = $ExpEntry.path

    if (-not $BaselineDict.ContainsKey($Path)) {
        Write-Log "WARNING: Path not in baseline: $Path"
        continue
    }

    $BaseEntry = $BaselineDict[$Path]

    # Check if MBIDs match
    if ($ExpEntry.current_mbid -eq $BaseEntry.current_mbid) {
        $ExactMatches++
        continue  # No change
    }

    $MbidChanges++

    # Categorize change as Better/Equivocal/Worse
    $Category = Categorize-Change -Experiment $ExpEntry -Baseline $BaseEntry

    switch ($Category) {
        "Better" { $ClearlyBetter++ }
        "Equivocal" { $Equivocal++ }
        "Worse" { $ClearlyWorse++ }
    }

    # Store detailed result
    $DetailedResults += @{
        Path = $Path
        Artist = $ExpEntry.artist
        Album = $ExpEntry.album
        Category = $Category
        BaselineMbid = $BaseEntry.current_mbid
        BaselineTracks = $BaseEntry.current_tracks
        BaselineMatchPct = $BaseEntry.match_percentage
        ExperimentMbid = $ExpEntry.current_mbid
        ExperimentTracks = $ExpEntry.current_tracks
        ExperimentMatchPct = $ExpEntry.match_percentage
    }
}

# Summary
$Summary = @{
    TotalAlbums = $Experiment.Count
    ExactMatches = $ExactMatches
    ExactMatchPct = [math]::Round($ExactMatches / $Experiment.Count * 100, 2)
    MbidChanges = $MbidChanges
    MbidChangePct = [math]::Round($MbidChanges / $Experiment.Count * 100, 2)
    ClearlyBetter = $ClearlyBetter
    ClearlyBetterPct = [math]::Round($ClearlyBetter / $MbidChanges * 100, 2)
    Equivocal = $Equivocal
    EquivocalPct = [math]::Round($Equivocal / $MbidChanges * 100, 2)
    ClearlyWorse = $ClearlyWorse
    ClearlyWorsePct = [math]::Round($ClearlyWorse / $MbidChanges * 100, 2)
    DetailedResults = $DetailedResults
}

# Save output if requested
if ($OutputFile) {
    $Summary | ConvertTo-Json -Depth 10 | Set-Content -Path $OutputFile
    Write-Log "Analysis saved to: $OutputFile"
}

# Return summary
return $Summary

# Categorization function
function Categorize-Change {
    param(
        [Parameter(Mandatory=$true)]
        $Experiment,

        [Parameter(Mandatory=$true)]
        $Baseline
    )

    $ExpPct = $Experiment.match_percentage
    $BasePct = $Baseline.match_percentage
    $ExpTracks = $Experiment.current_tracks
    $BaseTracks = $Baseline.current_tracks

    # Rule 1: Clearly Better - higher match % AND same or better track count
    if ($ExpPct -gt $BasePct + 5.0 -and $ExpTracks -ge $BaseTracks) {
        return "Better"
    }

    # Rule 2: Clearly Better - same match % but track count closer to expected
    $BaselineExpected = $Baseline.baseline_tracks
    if ($null -ne $BaselineExpected) {
        $BaseDiff = [math]::Abs($BaseTracks - $BaselineExpected)
        $ExpDiff = [math]::Abs($ExpTracks - $BaselineExpected)

        if ($ExpDiff -lt $BaseDiff -and [math]::Abs($ExpPct - $BasePct) -lt 5.0) {
            return "Better"
        }
    }

    # Rule 3: Clearly Worse - lower match % by >10%
    if ($ExpPct -lt $BasePct - 10.0) {
        return "Worse"
    }

    # Rule 4: Clearly Worse - track count massively wrong (1 track when expecting 10+)
    if ($ExpTracks -eq 1 -and $BaselineExpected -gt 5 -and $BaseTracks -gt 5) {
        return "Worse"
    }

    # Rule 5: Clearly Worse - match % <70% when baseline was >80%
    if ($ExpPct -lt 70.0 -and $BasePct -gt 80.0) {
        return "Worse"
    }

    # Rule 6: Equivocal - similar match % (±5%), different MBIDs
    if ([math]::Abs($ExpPct - $BasePct) -lt 5.0) {
        return "Equivocal"
    }

    # Rule 7: Better - match % improved by 5-10%
    if ($ExpPct -gt $BasePct + 5.0) {
        return "Better"
    }

    # Default: Equivocal (unclear which is better)
    return "Equivocal"
}
