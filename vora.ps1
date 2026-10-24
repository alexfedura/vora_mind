# ═════════════════════════════════════════════════════════════════════
#  Vora Mind — Launcher + Watchdog v3.7
#  • Live "X/Y" cargo progress with color
#  • File-based child launcher (no quote-stripping bugs)
#  • Pause/Resume, PID tracking, disk-full recovery
#  • Target dir on C:, no nuke, no .pdb, targeted cleanup only
# ═════════════════════════════════════════════════════════════════════

[CmdletBinding()]
param(
    [switch]$Watch,
    [switch]$Force,
    [switch]$CleanupOnly,
    [switch]$NoCleanup,
    [switch]$Pause,
    [switch]$Resume,
    [int]$CheckEverySec    = 15,
    [int]$MinFreeGB        = 1,
    [int]$MaxRestarts      = 0,
    [int]$BuildFailFastSec = 180
)

[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$OutputEncoding           = [System.Text.Encoding]::UTF8
$ErrorActionPreference    = "Continue"

# ─── Constants ───
$ProjectRoot = "D:\workspace\vora_mind"
$TargetDir   = "C:\vora_build"
$LogFile     = "$ProjectRoot\.vora.log"
$BuildLog    = "$ProjectRoot\.vora-build.log"
$PidFile     = "$ProjectRoot\.vora-dev.pid"
$PauseFlag   = "$ProjectRoot\.vora-paused"
$LaunchFile  = "$ProjectRoot\.vora-dev-launch.ps1"

# ═════════════════════════════════════════════════════════════════════
#  Visual helpers
# ═════════════════════════════════════════════════════════════════════

function Show-Banner {
    param([string]$Title, [string[]]$Meta)
    Clear-Host
    Write-Host ""
    Write-Host ("  " + ("=" * 68)) -ForegroundColor DarkCyan
    Write-Host "    VORA MIND  .  " -NoNewline -ForegroundColor Cyan
    Write-Host "$Title" -ForegroundColor Magenta
    Write-Host ("  " + ("=" * 68)) -ForegroundColor DarkCyan
    foreach ($m in $Meta) {
        Write-Host "     " -NoNewline
        Write-Host $m -ForegroundColor DarkGray
    }
    Write-Host ("  " + ("=" * 68)) -ForegroundColor DarkCyan
    Write-Host ""
}

function Log {
    param(
        [string]$Msg,
        [ValidateSet("INFO","OK","WARN","ERR","STEP","DIM","HDR")]
        [string]$Kind = "INFO"
    )
    $ts  = Get-Date -Format "HH:mm:ss"
    $tag = switch ($Kind) {
        "OK"   { "[ OK ]" }
        "WARN" { "[WARN]" }
        "ERR"  { "[FAIL]" }
        "STEP" { "[ >> ]" }
        "INFO" { "[ -- ]" }
        "HDR"  { "[ ** ]" }
        default { "       " }
    }
    $color = switch ($Kind) {
        "OK"   { "Green" }
        "WARN" { "Yellow" }
        "ERR"  { "Red" }
        "STEP" { "Cyan" }
        "INFO" { "Gray" }
        "HDR"  { "Magenta" }
        "DIM"  { "DarkGray" }
        default { "Gray" }
    }
    Write-Host "[$ts] " -NoNewline -ForegroundColor DarkGray
    Write-Host "$tag " -NoNewline -ForegroundColor $color
    Write-Host $Msg -ForegroundColor $(if ($Kind -eq "OK") { "White" } else { $color })
    Add-Content -Path $LogFile -Value "[$ts] [$Kind] $Msg" -EA SilentlyContinue
}

function Log-Bar {
    param([string]$Msg)
    Write-Host "  " -NoNewline
    Write-Host ("-" * 68) -ForegroundColor DarkCyan
    Write-Host "   " -NoNewline
    Write-Host $Msg -ForegroundColor Cyan
    Write-Host "  " -NoNewline
    Write-Host ("-" * 68) -ForegroundColor DarkCyan
}

# ═════════════════════════════════════════════════════════════════════
#  Core helpers
# ═════════════════════════════════════════════════════════════════════

function Strip-Ansi {
    param([string]$Text)
    if ([string]::IsNullOrEmpty($Text)) { return "" }
    return ($Text -replace '\x1b\[[0-9;]*[A-Za-z]', '')
}

function Get-FreeGB { return [math]::Round((Get-PSDrive C).Free / 1GB, 2) }

function Test-DevAlive {
    $vm   = Get-Process -Name "vora_mind" -EA SilentlyContinue
    $port = Get-NetTCPConnection -LocalPort 1420 -State Listen -EA SilentlyContinue
    return [bool]($vm -and $port)
}

function Test-Compiling {
    return [bool](Get-Process -Name "cargo","rustc" -EA SilentlyContinue)
}

function Kill-Tree {
    param([int]$ProcessId)
    if ($ProcessId -le 0) { return }
    & taskkill /F /T /PID $ProcessId 2>&1 | Out-Null
}

function Kill-OldDev {
    if (Test-Path $PidFile) {
        $trackedPid = Get-Content $PidFile -EA SilentlyContinue | Select-Object -First 1
        if ($trackedPid -match '^\d+$') { Kill-Tree -ProcessId ([int]$trackedPid) }
        Remove-Item $PidFile -Force -EA SilentlyContinue
    }
    Get-Process -Name "vora_mind" -EA SilentlyContinue |
        Stop-Process -Force -EA SilentlyContinue
    Get-NetTCPConnection -LocalPort 1420 -EA SilentlyContinue | ForEach-Object {
        if ($_.OwningProcess -ne 0) { Kill-Tree -ProcessId $_.OwningProcess }
    }
}

# ═════════════════════════════════════════════════════════════════════
#  Space management
# ═════════════════════════════════════════════════════════════════════

function Ensure-Space {
    param([int]$NeedGB)
    $free = Get-FreeGB
    if ($free -ge $NeedGB) { return $true }

    Log "Low space ($free GB < $NeedGB GB) - cleaning" "WARN"

    Log "  [1/4] Removing .pdb files" "DIM"
    Get-ChildItem $TargetDir -Filter "*.pdb" -Recurse -EA SilentlyContinue |
        Remove-Item -Force -EA SilentlyContinue
    $free = Get-FreeGB
    Log "        -> Free now: $free GB" "DIM"
    if ($free -ge $NeedGB) { Log "Cleanup sufficient" "OK"; return $true }

    Log "  [2/4] Removing incremental/build/.fingerprint" "DIM"
    Remove-Item "$TargetDir\debug\incremental"  -Recurse -Force -EA SilentlyContinue
    Remove-Item "$TargetDir\debug\build"        -Recurse -Force -EA SilentlyContinue
    Remove-Item "$TargetDir\debug\.fingerprint" -Recurse -Force -EA SilentlyContinue
    $free = Get-FreeGB
    Log "        -> Free now: $free GB" "DIM"
    if ($free -ge $NeedGB) { Log "Cleanup sufficient" "OK"; return $true }

    Log "  [3/4] Removing deps .lib / .rlib" "DIM"
    Get-ChildItem "$TargetDir\debug\deps" -Filter "*.lib"  -EA SilentlyContinue |
        Remove-Item -Force -EA SilentlyContinue
    Get-ChildItem "$TargetDir\debug\deps" -Filter "*.rlib" -EA SilentlyContinue |
        Remove-Item -Force -EA SilentlyContinue
    $free = Get-FreeGB
    Log "        -> Free now: $free GB" "DIM"
    if ($free -ge $NeedGB) { Log "Cleanup sufficient" "OK"; return $true }

    Log "  [4/4] Targeted clean (NEVER nuke target)" "WARN"
    Remove-Item "$TargetDir\debug\examples"   -Recurse -Force -EA SilentlyContinue
    Remove-Item "$TargetDir\debug\build"      -Recurse -Force -EA SilentlyContinue
    Remove-Item "$TargetDir\debug\deps\*.pdb" -Force -EA SilentlyContinue
    Remove-Item "$TargetDir\flycheck0"        -Recurse -Force -EA SilentlyContinue
    Remove-Item "$TargetDir\tmp"              -Recurse -Force -EA SilentlyContinue
    $free = Get-FreeGB
    Log "        -> Free now: $free GB" "DIM"

    $ok = ($free -ge $NeedGB)
    if ($ok) { Log "Cleanup done (target preserved)" "OK" }
    else     { Log "Still low, but target preserved for cache reuse" "WARN" }
    return $ok
}

# ═════════════════════════════════════════════════════════════════════
#  Child launcher script (written to disk to avoid quoting bugs)
# ═════════════════════════════════════════════════════════════════════

function Write-LaunchScript {
    # Use @'...'@ (single-quoted) so PowerShell does NOT interpolate $vars
    $template = @'
# ═════════════════════════════════════════════════════════════════════
#  Vora Mind — dev launcher (auto-generated; do not edit)
# ═════════════════════════════════════════════════════════════════════
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$OutputEncoding           = [System.Text.Encoding]::UTF8

Set-Location "::PROJECT_ROOT::"

$env:CARGO_TARGET_DIR         = "::TARGET_DIR::"
$env:CARGO_TERM_COLOR         = "never"
$env:CARGO_TERM_PROGRESS_WHEN = "always"
$env:NO_COLOR                 = "1"
$env:FORCE_COLOR              = "0"
$env:TERM                     = "dumb"

$BuildLog = "::BUILD_LOG::"
"" | Out-File -FilePath $BuildLog -Encoding utf8 -Force

function Emit {
    param([string]$Text, [string]$Color = "Gray")
    Write-Host $Text -ForegroundColor $Color
    Add-Content -Path $BuildLog -Value $Text -Encoding utf8
}

Write-Host ""
Write-Host "  Launching dev (log: $BuildLog)" -ForegroundColor DarkCyan
Write-Host ""

pnpm tauri dev 2>&1 | ForEach-Object {
    $line = [string]$_

    # 1. strip ANSI
    $line = $line -replace '\x1b\[[0-9;]*[A-Za-z]', ''
    # 2. keep only printable ASCII + tab
    $line = $line -replace '[^\x09\x20-\x7E]', ''
    # 3. trim
    $line = $line.TrimEnd()
    if ([string]::IsNullOrWhiteSpace($line)) { return }

    # ── cargo progress line: "Building [====>] 162/594: darling..."
    if ($line -match 'Building\s+\[\s*[=>\-\s.]+\s*\]\s+(\d+)/(\d+):\s*(.+?)\s*\.*\s*$') {
        $cur   = $Matches[1]
        $tot   = $Matches[2]
        $crate = $Matches[3].Trim()
        Write-Host "     " -NoNewline
        Write-Host "[ $cur/$tot ]" -NoNewline -ForegroundColor Cyan
        Write-Host "  $crate" -ForegroundColor DarkGray
        Add-Content -Path $BuildLog -Value "PROGRESS $cur/$tot $crate" -Encoding utf8
        return
    }

    # ── Compiling line
    if ($line -match '^\s*Compiling\s+') {
        Emit $line "DarkCyan"
        return
    }

    # ── Error
    if ($line -match 'error\[E\d+\]|^\s*error:|error: could not compile') {
        Emit $line "Red"
        return
    }

    # ── Warning
    if ($line -match '^\s*warning:') {
        Emit $line "Yellow"
        return
    }

    # ── Vite / Tauri info
    if ($line -match 'VITE|Local:|Running\s+Before|Running\s+DevCommand|Watching|Finished|Info\s+Watching|Info\s+Compiling') {
        Emit $line "Gray"
        return
    }

    # ── Everything else
    Emit $line "DarkGray"
}

Write-Host ""
Write-Host "  [ tauri dev exited ]" -ForegroundColor DarkYellow
'@

    # Replace placeholders using .Replace (NOT -replace, which expands $)
    $script = $template.Replace('::PROJECT_ROOT::', $ProjectRoot)
    $script = $script.Replace('::TARGET_DIR::',   $TargetDir)
    $script = $script.Replace('::BUILD_LOG::',    $BuildLog)

    $script | Out-File -FilePath $LaunchFile -Encoding utf8 -Force
}

# ═════════════════════════════════════════════════════════════════════
#  Dev launcher
# ═════════════════════════════════════════════════════════════════════

function Start-Dev {
    # kill previous tracked dev
    if (Test-Path $PidFile) {
        $oldPid = Get-Content $PidFile -EA SilentlyContinue | Select-Object -First 1
        if ($oldPid -match '^\d+$') { Kill-Tree -ProcessId ([int]$oldPid) }
        Remove-Item $PidFile -Force -EA SilentlyContinue
    }

    # regenerate launcher script
    Write-LaunchScript

    # spawn detached
    $proc = Start-Process pwsh `
        -ArgumentList "-NoExit","-ExecutionPolicy","Bypass","-File",$LaunchFile `
        -PassThru
    if ($proc) {
        $proc.Id | Out-File -FilePath $PidFile -Encoding ascii -Force
    }
}

function Get-BuildError {
    if (-not (Test-Path $BuildLog)) { return $null }
    $tailLines = Get-Content $BuildLog -Tail 1000 -Encoding utf8 -EA SilentlyContinue
    if (-not $tailLines) { return $null }
    $text  = ($tailLines -join "`n")
    $clean = Strip-Ansi $text
    if ($clean -match 'no space on the disk|os error 112|LNK1140|LNK1104') {
        return "disk-full"
    }
    if ($clean -match 'error\[E\d+\]|error: could not compile') {
        return "compile-error"
    }
    return $null
}

function Show-BuildTail {
    param([int]$Lines = 20)
    if (-not (Test-Path $BuildLog)) { return }
    Log "Last $Lines lines of build log:" "INFO"
    Get-Content $BuildLog -Tail $Lines -Encoding utf8 -EA SilentlyContinue |
        ForEach-Object {
            $clean = Strip-Ansi $_
            if ($clean -match '^PROGRESS ') { return }   # skip progress spam
            Write-Host "        | " -NoNewline -ForegroundColor DarkGray
            Write-Host $clean -ForegroundColor DarkGray
        }
}

function Wait-ForBoot {
    param([int]$MaxSec = 900, [int]$FailFastSec = 180)
    $start = Get-Date
    $lastShown = -1
    for ($i = 0; $i -lt $MaxSec; $i++) {
        Start-Sleep -Seconds 1

        if (Test-Path $PauseFlag) { return @{ ok = $false; reason = "paused" } }
        if (Test-DevAlive)        { return @{ ok = $true;  reason = "booted" } }

        $elapsed = ((Get-Date) - $start).TotalSeconds
        if ($elapsed -gt $FailFastSec -and -not (Test-Compiling)) {
            $err = Get-BuildError
            if ($err) { return @{ ok = $false; reason = $err } }
            return @{ ok = $false; reason = "cargo-exited" }
        }

        # Every 30s: log current progress
        if ($i -gt 0 -and ($i % 30) -eq 0 -and $i -ne $lastShown) {
            $lastShown = $i
            $lastProg = ""
            if (Test-Path $BuildLog) {
                $tail = Get-Content $BuildLog -Tail 100 -Encoding utf8 -EA SilentlyContinue
                $prog = $tail | Where-Object { $_ -match '^PROGRESS ' } |
                        Select-Object -Last 1
                if ($prog) { $lastProg = " | " + ($prog -replace '^PROGRESS ','') }
            }
            Log "  still compiling ${i}s${lastProg}" "DIM"
        }
    }
    return @{ ok = $false; reason = "timeout" }
}

# ═════════════════════════════════════════════════════════════════════
#  MAIN
# ═════════════════════════════════════════════════════════════════════

$modeName = if ($Pause)             { "PAUSE" }
            elseif ($Resume)        { "RESUME" }
            elseif ($Watch)         { "WATCHDOG" }
            elseif ($CleanupOnly)   { "CLEANUP" }
            else                    { "LAUNCHER" }

Show-Banner -Title "$modeName mode" -Meta @(
    "interval     : $CheckEverySec s",
    "min-free     : $MinFreeGB GB",
    "max-restarts : $(if ($MaxRestarts -eq 0) { 'unlimited' } else { $MaxRestarts })",
    "fail-fast    : ${BuildFailFastSec} s"
)

# ─── PAUSE mode ───
if ($Pause) {
    Log "Pausing watchdog" "STEP"
    Kill-OldDev
    "" | Out-File -FilePath $PauseFlag -Encoding ascii -Force
    Log "Dev killed + pause flag set" "OK"
    Log "Run  .\vora.ps1 -Resume  to resume" "DIM"
    Start-Sleep -Seconds 2
    exit 0
}

# ─── RESUME mode ───
if ($Resume) {
    if (Test-Path $PauseFlag) {
        Remove-Item $PauseFlag -Force -EA SilentlyContinue
        Log "Pause flag removed" "OK"
        Log "Watchdog will restart dev on next check" "INFO"
    } else {
        Log "Not paused - nothing to do" "DIM"
    }
    if (-not $Watch) { Start-Sleep -Seconds 2; exit 0 }
}

# ─── CLEANUP-ONLY mode ───
if ($CleanupOnly) {
    Log "Cleanup-only mode" "STEP"
    Ensure-Space -NeedGB $MinFreeGB | Out-Null
    Log "Done" "OK"
    Start-Sleep -Seconds 2
    exit 0
}

# ─── Fast path: dev alive? ───
if ((Test-DevAlive) -and -not $Force) {
    if (Test-Compiling) { Log "Dev running + cargo compiling" "OK" }
    else                { Log "Dev already running (HMR active)" "OK" }
    if ($Watch) { Log "Entering watchdog loop..." "STEP" }
    else {
        Log "Nothing to do" "DIM"
        Start-Sleep -Seconds 2
        exit 0
    }
}

# ═══ Non-watch: retry loop ═══
if (-not $Watch) {
    $attempt = 0
    $maxAttempts = 4

    while ($attempt -lt $maxAttempts) {
        $attempt++
        Log "Attempt $attempt / $maxAttempts" "STEP"

        if (-not $NoCleanup) {
            $free = Get-FreeGB
            Log "C: Free = $free GB (threshold $MinFreeGB GB)" "INFO"
            if ($free -lt $MinFreeGB) { Ensure-Space -NeedGB $MinFreeGB | Out-Null }
            else                      { Log "Enough space" "OK" }
        }

        if ($Force -or $attempt -gt 1) {
            Log "Killing stale processes" "DIM"
            Kill-OldDev
            Start-Sleep -Seconds 1
        }

        Log "Launching dev (log -> $BuildLog)" "STEP"
        Start-Dev

        Log "Waiting (fail-fast ${BuildFailFastSec}s)..." "STEP"
        $result = Wait-ForBoot -MaxSec 900 -FailFastSec $BuildFailFastSec

        if ($result.ok) {
            Log "RUN: OK - vora_mind is running" "OK"
            Start-Sleep -Seconds 5
            exit 0
        }

        Log "Attempt $attempt failed: $($result.reason)" "ERR"
        Kill-OldDev
        Start-Sleep -Seconds 2

        if ($result.reason -eq "disk-full") {
            Log "Disk-full detected - aggressive cleanup + retry" "WARN"
            Ensure-Space -NeedGB ($MinFreeGB * 3) | Out-Null
            continue
        } elseif ($result.reason -eq "compile-error") {
            Log "Compile error - see $BuildLog" "ERR"
            Show-BuildTail -Lines 25
            exit 1
        } elseif ($result.reason -eq "cargo-exited") {
            Log "Cargo exited without vora_mind" "ERR"
            Show-BuildTail -Lines 25
            $free = Get-FreeGB
            if ($free -lt ($MinFreeGB * 2)) { Ensure-Space -NeedGB ($MinFreeGB * 2) | Out-Null }
            continue
        } else {
            Log "Timeout - not retrying" "WARN"
            exit 1
        }
    }

    Log "Max attempts reached" "ERR"
    exit 1
}

# ═══ Watch mode ═══
Log "Watchdog started" "OK"
if (Test-Path $PauseFlag) {
    Log "PAUSED state - dev will not launch until -Resume" "WARN"
}
Log-Bar "monitoring dev + cargo (Ctrl+C to stop)"
Write-Host ""

$restarts     = 0
$wasAlive     = Test-DevAlive
$wasCompiling = Test-Compiling

while ($true) {
    try {
        if (Test-Path $PauseFlag) {
            if ($wasAlive) {
                Log "PAUSED - monitoring halted" "WARN"
                $wasAlive = $false
                $wasCompiling = $false
            }
            Start-Sleep -Seconds $CheckEverySec
            continue
        }

        $alive     = Test-DevAlive
        $compiling = Test-Compiling

        if ($alive) {
            if (-not $wasAlive) {
                Log "Dev ALIVE (free $(Get-FreeGB) GB)" "OK"
            } elseif ($compiling -and -not $wasCompiling) {
                Log "cargo compiling" "DIM"
            } elseif (-not $compiling -and $wasCompiling) {
                Log "cargo finished (HMR active)" "DIM"
            }
            $wasAlive     = $true
            $wasCompiling = $compiling
        } else {
            if ($compiling) {
                if ($wasAlive) { Log "Dev stopped - build in progress" "WARN" }
                $wasAlive     = $false
                $wasCompiling = $true
            } else {
                if ($wasAlive) { Log "Dev CRASHED" "WARN" }
                else           { Log "Dev not running" "INFO" }
                $wasAlive     = $false
                $wasCompiling = $false

                if ($MaxRestarts -gt 0 -and $restarts -ge $MaxRestarts) {
                    Log "Max restarts ($MaxRestarts) reached - exiting" "ERR"
                    break
                }

                $err = Get-BuildError
                if ($err -eq "disk-full") {
                    Log "Disk-full in log - aggressive cleanup" "WARN"
                    Ensure-Space -NeedGB ($MinFreeGB * 3) | Out-Null
                } elseif (-not $NoCleanup) {
                    Ensure-Space -NeedGB $MinFreeGB | Out-Null
                }

                Start-Dev
                $restarts++

                Log "Restart #$restarts launched (fail-fast ${BuildFailFastSec}s)..." "STEP"
                $result = Wait-ForBoot -MaxSec 900 -FailFastSec $BuildFailFastSec

                if ($result.ok) {
                    Log "Boot success" "OK"
                    $wasAlive = $true
                } elseif ($result.reason -eq "paused") {
                    Log "Boot interrupted by pause" "WARN"
                } else {
                    Log "Boot failed: $($result.reason)" "WARN"
                }
            }
        }
    } catch {
        Log "Loop error: $_" "ERR"
    }

    Start-Sleep -Seconds $CheckEverySec
}

Log "Watchdog exited" "DIM"
