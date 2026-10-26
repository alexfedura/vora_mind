param (
    [string]$Version = ""
)

$ErrorActionPreference = "Stop"
$ProjectRoot = Split-Path -Parent $PSScriptRoot

# خواندن نسخه از فایل مرجع VERSION در صورت عدم ارسال دستی
$VersionFile = Join-Path $ProjectRoot "VERSION"
if (-not $Version -and (Test-Path $VersionFile)) {
    $Version = (Get-Content $VersionFile -Raw).Trim()
}
if (-not $Version) {
    $Version = "0.55.0"
}

Write-Host "============================================================" -ForegroundColor Cyan
Write-Host "STARTING VORA RELEASE FACTORY PIPELINE (v$Version)" -ForegroundColor Cyan
Write-Host "============================================================" -ForegroundColor Cyan

# کنترل مصرف پردازنده و رم
$env:CARGO_BUILD_JOBS = "4"

# 1. اجرای تست‌های رگرسیون بک‌اند و ماتریس تاب‌آوری
Write-Host "`n[1/4] Running Backend & Worker Resilience Tests..." -ForegroundColor Yellow
cargo test --manifest-path "$ProjectRoot/src-tauri/Cargo.toml" --lib -- --nocapture
cargo test --manifest-path "$ProjectRoot/src-tauri/Cargo.toml" --test worker_resilience_matrix_test -- --nocapture

# 2. بیلد کامل فرانت‌اند
Write-Host "`n[2/4] Building Frontend..." -ForegroundColor Yellow
Set-Location $ProjectRoot
pnpm build

# 3. بیلد باینری‌های ریموت ورکر و پکیج نهایی Tauri
Write-Host "`n[3/4] Building Release Artifacts (vora_worker & Tauri App)..." -ForegroundColor Yellow
cargo build --manifest-path "$ProjectRoot/src-tauri/Cargo.toml" --release --bin vora_worker

$KeyPath = Join-Path (Join-Path $HOME ".tauri") "vora.key"
if (Test-Path $KeyPath) {
    $env:TAURI_SIGNING_PRIVATE_KEY = Get-Content $KeyPath -Raw
}
pnpm tauri build

# 4. تولید چک‌سام‌های SHA256 و مانیفست انتشار
Write-Host "`n[4/4] Generating Artifact SHA256 Checksums..." -ForegroundColor Yellow
$bundleDir = "C:\vora-compile\cargo-target\release\bundle\nsis"
if (Test-Path $bundleDir) {
    $setupExe = Get-ChildItem -Path $bundleDir -Filter "*setup.exe" | Select-Object -First 1
    if ($setupExe) {
        $hash = (Get-FileHash -Path $setupExe.FullName -Algorithm SHA256).Hash
        $manifest = @{
            version  = $Version
            date     = (Get-Date).ToString("yyyy-MM-dd HH:mm:ss")
            artifact = $setupExe.Name
            sha256   = $hash
        } | ConvertTo-Json -Depth 3

        Set-Content -Path "$bundleDir/release_manifest.json" -Value $manifest -Encoding utf8
        Write-Host "  Artifact: $($setupExe.Name)" -ForegroundColor Green
        Write-Host "  SHA256:   $hash" -ForegroundColor Green
        Write-Host "  Manifest: $bundleDir/release_manifest.json" -ForegroundColor Green
    }
}

Write-Host "`n============================================================" -ForegroundColor Green
Write-Host "RELEASE FACTORY PIPELINE COMPLETED SUCCESSFULLY (v$Version)" -ForegroundColor Green
Write-Host "============================================================" -ForegroundColor Green
