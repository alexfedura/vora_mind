param (
    [int]$Port = 4040,
    [string]$Mode = "local",
    [string]$Token = ""
)

$ErrorActionPreference = "Stop"
$ProjectRoot = Split-Path -Parent $PSScriptRoot
$TauriDir = Join-Path $ProjectRoot "src-tauri"
$LogDir = Join-Path $TauriDir "logs"

if (-not (Test-Path $LogDir)) {
    New-Item -ItemType Directory -Path $LogDir | Out-Null
}
$LogOut = Join-Path $LogDir "worker_stdout.log"
$LogErr = Join-Path $LogDir "worker_stderr.log"

# تنظیم متغیرهای محیطی ران‌تایم
$env:WORKER_PORT = "$Port"
$env:WORKER_MODE = "$Mode"
if ($Token) {
    $env:WORKER_AUTH_TOKEN = "$Token"
} else {
    Remove-Item env:WORKER_AUTH_TOKEN -ErrorAction SilentlyContinue
}

Write-Host "============================================================" -ForegroundColor Cyan
Write-Host "STARTING VORA AUTONOMOUS WORKER BACKGROUND DAEMON" -ForegroundColor Cyan
Write-Host "============================================================" -ForegroundColor Cyan

$existing = Get-Process -Name "vora_worker" -ErrorAction SilentlyContinue
if ($existing) {
    Write-Host "Worker is already running (PID: $($existing.Id))" -ForegroundColor Yellow
    exit 0
}

# بیلد باینری ورکر قبل از اجرا جهت اطمینان از اعمال آخرین تغییرات
Write-Host "[*] Building vora_worker binary..." -ForegroundColor Yellow
cargo build --manifest-path "$TauriDir\Cargo.toml" --bin vora_worker

$BinaryPath = Join-Path $TauriDir "target\debug\vora_worker.exe"
$proc = Start-Process -FilePath $BinaryPath -ArgumentList "serve", "$Port" -WorkingDirectory $TauriDir -RedirectStandardOutput $LogOut -RedirectStandardError $LogErr -PassThru -WindowStyle Hidden

Write-Host "Worker Daemon started successfully!" -ForegroundColor Green
Write-Host "  PID:  $($proc.Id)" -ForegroundColor Green
Write-Host "  Port: http://127.0.0.1:$Port" -ForegroundColor Green
Write-Host "  Mode: $Mode" -ForegroundColor Green
Write-Host "  Logs: $LogOut" -ForegroundColor Green
Write-Host "============================================================" -ForegroundColor Cyan
