# ═════════════════════════════════════════════════════════════════════
#  Creates a Desktop shortcut for Vora Mind launcher
#  Run ONCE. The shortcut is permanent.
# ═════════════════════════════════════════════════════════════════════

$ProjectRoot  = "D:\workspace\vora_mind"
$LauncherPath = "$ProjectRoot\vora-launch.ps1"
$DesktopPath  = [Environment]::GetFolderPath("Desktop")
$ShortcutPath = "$DesktopPath\Vora Mind.lnk"

if (-not (Test-Path $LauncherPath)) {
    Write-Host "ERROR: $LauncherPath not found." -ForegroundColor Red
    Write-Host "Create vora-launch.ps1 first." -ForegroundColor Red
    exit 1
}

$WshShell = New-Object -ComObject WScript.Shell
$Shortcut = $WshShell.CreateShortcut($ShortcutPath)
$Shortcut.TargetPath       = "pwsh.exe"
$Shortcut.Arguments        = "-NoExit -ExecutionPolicy Bypass -File `"$LauncherPath`""
$Shortcut.WorkingDirectory = $ProjectRoot
$Shortcut.IconLocation     = "pwsh.exe,0"
$Shortcut.Description      = "Launch Vora Mind dev mode (auto cleanup)"
$Shortcut.WindowStyle      = 1    # Normal window
$Shortcut.Save()

Write-Host ""
Write-Host "✓ Desktop shortcut created:" -ForegroundColor Green
Write-Host "  $ShortcutPath" -ForegroundColor Cyan
Write-Host ""
Write-Host "Double-click it from your desktop to launch Vora Mind." -ForegroundColor White
Write-Host ""
