param (
    [string]$Url = "http://127.0.0.1:4040"
)

$ErrorActionPreference = "SilentlyContinue"

Write-Host "============================================================" -ForegroundColor Cyan
Write-Host "VORA WORKER LIVE DIAGNOSTICS & TELEMETRY" -ForegroundColor Cyan
Write-Host "============================================================" -ForegroundColor Cyan

try {
    $health = Invoke-RestMethod -Uri "$Url/api/health" -Method Get -TimeoutSec 3
    Write-Host "Worker Status: ONLINE" -ForegroundColor Green
    Write-Host "Node ID:       $($health.current_node)" -ForegroundColor Cyan
    Write-Host "Version:       $($health.version)" -ForegroundColor Cyan
    Write-Host "Mode:          $($health.mode)" -ForegroundColor Cyan
    Write-Host "Database:      $($health.database)" -ForegroundColor Cyan
    Write-Host "Active Tasks:  $($health.cluster.active_nodes)" -ForegroundColor Cyan
    Write-Host "Queue Metrics: Completed=$($health.queue.completed), Pending=$($health.queue.pending), Failed=$($health.queue.failed)" -ForegroundColor Yellow
} catch {
    Write-Host "Worker Status: OFFLINE ($Url unreachable)" -ForegroundColor Red
}

Write-Host "============================================================" -ForegroundColor Cyan
