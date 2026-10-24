$ErrorActionPreference = "SilentlyContinue"

Write-Host "============================================================" -ForegroundColor Cyan
Write-Host "STOPPING VORA AUTONOMOUS WORKER DAEMONS" -ForegroundColor Cyan
Write-Host "============================================================" -ForegroundColor Cyan

$procs = Get-Process -Name "vora_worker"
if ($procs) {
    foreach ($p in $procs) {
        Stop-Process -Id $p.Id -Force
        Write-Host "Killed vora_worker process (PID: $($p.Id))" -ForegroundColor Green
    }
} else {
    Write-Host "No active vora_worker process found." -ForegroundColor Yellow
}

Write-Host "============================================================" -ForegroundColor Cyan
