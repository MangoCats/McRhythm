# Stop running cargo test processes
Get-Process | Where-Object { $_.ProcessName -like "*cargo*" -or $_.ProcessName -like "*run29f*" } | Stop-Process -Force
Write-Host "Stopped cargo test processes"
