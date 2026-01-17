$content = Get-Content 'album_matcher_output_run10.txt' -Raw

# Extract all "Matched tracks: X/Y (Z%)" patterns
$matches = [regex]::Matches($content, 'Matched tracks: (\d+)/(\d+) \((\d+\.\d+)%\)')

Write-Output "=== Run 10 Album Success Rate Analysis ==="
Write-Output ""
Write-Output "Total albums processed: $($matches.Count)"
Write-Output ""

# Count perfect matches (100%)
$perfect = ($matches | Where-Object { $_.Groups[1].Value -eq $_.Groups[2].Value }).Count
Write-Output "Perfect matches (100%): $perfect ($([math]::Round($perfect/$matches.Count*100, 1))%)"

# Count high matches (>=90%)
$high = ($matches | Where-Object { [double]$_.Groups[3].Value -ge 90.0 }).Count
Write-Output "High matches (>=90%): $high ($([math]::Round($high/$matches.Count*100, 1))%)"

# Count medium matches (75-89%)
$medium = ($matches | Where-Object { [double]$_.Groups[3].Value -ge 75.0 -and [double]$_.Groups[3].Value -lt 90.0 }).Count
Write-Output "Medium matches (75-89%): $medium ($([math]::Round($medium/$matches.Count*100, 1))%)"

# Count low matches (<75%)
$low = ($matches | Where-Object { [double]$_.Groups[3].Value -lt 75.0 }).Count
Write-Output "Low matches (<75%): $low ($([math]::Round($low/$matches.Count*100, 1))%)"

Write-Output ""
Write-Output "=== Individual Album Results ==="
Write-Output ""

# Show all match rates
for ($i = 0; $i -lt $matches.Count; $i++) {
    $matched = $matches[$i].Groups[1].Value
    $total = $matches[$i].Groups[2].Value
    $percent = $matches[$i].Groups[3].Value
    $albumNum = $i + 1
    Write-Output "Album $albumNum : $matched/$total tracks ($percent%)"
}
