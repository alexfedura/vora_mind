$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$resourceDirectory = Join-Path $root "src-tauri\resources\binaries\mpv"
$expectedResourceDirectory = [System.IO.Path]::GetFullPath(
    (Join-Path $root "src-tauri\resources\binaries\mpv")
)

if ($resourceDirectory -ne $expectedResourceDirectory) {
    throw "Resolved MPV resource path does not match the expected project resource directory."
}

$localRuntimeCandidates = @()
if (-not [string]::IsNullOrWhiteSpace($env:VORA_PLAYER_MPV_DIR)) {
    $localRuntimeCandidates += $env:VORA_PLAYER_MPV_DIR
}
$localRuntimeCandidates += Join-Path (Split-Path -Parent $root) "vora_player\libs\mpv"
$localRuntime = $localRuntimeCandidates |
    Where-Object { Test-Path -LiteralPath (Join-Path $_ "mpv.exe") -PathType Leaf } |
    Select-Object -First 1

$runtimeFiles = @()
$runtimeVersion = ""

if ($localRuntime) {
    $runtimeFiles = Get-ChildItem -LiteralPath $localRuntime -File -Recurse |
        Where-Object {
            $_.Name -eq "mpv.exe" -or
            $_.Name -eq "mpv.com" -or
            $_.Name -eq "d3dcompiler_43.dll" -or
            $_.Extension -eq ".dll" -or
            $_.Name -eq "fonts.conf"
        }
    $versionPath = Join-Path $localRuntime "VERSION.txt"
    if (Test-Path -LiteralPath $versionPath -PathType Leaf) {
        $runtimeVersion = Get-Content -LiteralPath $versionPath -Raw
    } else {
        $runtimeVersion = "Vora Player local runtime"
    }
} else {
    $assetName = "mpv-x86_64-20261107-git-9028187538.7z"
    $assetUrl = "https://github.com/shinchiro/mpv-winbuild-cmake/releases/download/20261107/$assetName"
    $expectedSha256 = "4b7afb4bbe7572634eea4352b9fa3101d4f2b328d83bff09f7b9bac125bb2b3a"
    $tempDirectory = Join-Path $env:TEMP "vora-mind-mpv-20261107"
    $archivePath = Join-Path $tempDirectory $assetName
    $extractDirectory = Join-Path $tempDirectory "extracted"

    $sevenZip = Get-Command 7z.exe -ErrorAction SilentlyContinue
    if (-not $sevenZip) {
        $sevenZipCandidates = @(
            (Join-Path $env:ProgramFiles "7-Zip\7z.exe"),
            (Join-Path ${env:ProgramFiles(x86)} "7-Zip\7z.exe")
        )
        $sevenZipPath = $sevenZipCandidates |
            Where-Object { Test-Path -LiteralPath $_ -PathType Leaf } |
            Select-Object -First 1
        if (-not $sevenZipPath) {
            throw "7-Zip is required to extract the pinned MPV runtime. Install 7-Zip or set VORA_PLAYER_MPV_DIR."
        }
    } else {
        $sevenZipPath = $sevenZip.Source
    }

    try {
        New-Item -ItemType Directory -Force -Path $tempDirectory | Out-Null
        Invoke-WebRequest -Uri $assetUrl -OutFile $archivePath -UseBasicParsing
        $actualSha256 = (Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($actualSha256 -ne $expectedSha256) {
            throw "The downloaded MPV archive failed its SHA-256 verification."
        }

        New-Item -ItemType Directory -Force -Path $extractDirectory | Out-Null
        & $sevenZipPath x $archivePath "-o$extractDirectory" -y | Out-Null
        if ($LASTEXITCODE -ne 0) {
            throw "7-Zip could not extract the MPV archive (exit code $LASTEXITCODE)."
        }

        $mpvExecutable = Get-ChildItem -LiteralPath $extractDirectory -Filter "mpv.exe" -File -Recurse |
            Select-Object -First 1
        if (-not $mpvExecutable) {
            throw "The pinned MPV archive did not contain mpv.exe."
        }
        $runtimeRoot = $mpvExecutable.DirectoryName
        $runtimeFiles = Get-ChildItem -LiteralPath $runtimeRoot -File -Recurse |
            Where-Object {
                $_.Name -eq "mpv.exe" -or
                $_.Name -eq "mpv.com" -or
                $_.Name -eq "d3dcompiler_43.dll" -or
                $_.Extension -eq ".dll" -or
                $_.Name -eq "fonts.conf"
            }
        $runtimeVersion = "MPV release 20261107 (9028187538)"
    } finally {
        if (Test-Path -LiteralPath $tempDirectory) {
            Remove-Item -LiteralPath $tempDirectory -Recurse -Force
        }
    }
}

if (-not ($runtimeFiles | Where-Object { $_.Name -eq "mpv.exe" })) {
    throw "The selected MPV runtime does not contain mpv.exe."
}

if (Test-Path -LiteralPath $resourceDirectory) {
    Remove-Item -LiteralPath $resourceDirectory -Recurse -Force
}
New-Item -ItemType Directory -Force -Path $resourceDirectory | Out-Null

foreach ($file in $runtimeFiles) {
    $relativePath = $file.FullName.Substring(
        [System.IO.Path]::GetFullPath(
            if ($localRuntime) { $localRuntime } else { $runtimeRoot }
        ).TrimEnd([System.IO.Path]::DirectorySeparatorChar).Length
    ).TrimStart([System.IO.Path]::DirectorySeparatorChar)
    $destination = Join-Path $resourceDirectory $relativePath
    $destinationDirectory = Split-Path -Parent $destination
    New-Item -ItemType Directory -Force -Path $destinationDirectory | Out-Null
    Copy-Item -LiteralPath $file.FullName -Destination $destination -Force
}

Set-Content -LiteralPath (Join-Path $resourceDirectory "MPV-VERSION.txt") -Value $runtimeVersion -Encoding UTF8
Copy-Item -LiteralPath (Join-Path $root "licenses\mpv-third-party-notice.txt") `
    -Destination (Join-Path $resourceDirectory "THIRD-PARTY-NOTICE.txt") -Force

if (-not (Test-Path -LiteralPath (Join-Path $resourceDirectory "mpv.exe") -PathType Leaf)) {
    throw "MPV preparation completed without creating the expected executable."
}

Write-Host "Prepared MPV runtime in $resourceDirectory"
