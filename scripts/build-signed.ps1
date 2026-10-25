param (
    [string]$KeyPath = (Join-Path (Join-Path $HOME ".tauri") "vora.key"),
    [string]$BundleDirectory = (Join-Path (Split-Path -Parent $PSScriptRoot) "target\release\bundle")
)

$ErrorActionPreference = "Stop"
$ProjectRoot = Split-Path -Parent $PSScriptRoot
$ResolvedKeyPath = Resolve-Path -LiteralPath $KeyPath -ErrorAction Stop

if (-not (Test-Path -LiteralPath $ResolvedKeyPath -PathType Leaf)) {
    throw "Updater private key file not found: $ResolvedKeyPath"
}

$password = Read-Host "Updater key password" -AsSecureString
if ($password.Length -eq 0) {
    throw "Enter the non-empty password for this updater key."
}

$previousSigningEnvironment = @{
    TAURI_SIGNING_PRIVATE_KEY = [Environment]::GetEnvironmentVariable("TAURI_SIGNING_PRIVATE_KEY", "Process")
    TAURI_SIGNING_PRIVATE_KEY_PATH = [Environment]::GetEnvironmentVariable("TAURI_SIGNING_PRIVATE_KEY_PATH", "Process")
    TAURI_SIGNING_PRIVATE_KEY_PASSWORD = [Environment]::GetEnvironmentVariable("TAURI_SIGNING_PRIVATE_KEY_PASSWORD", "Process")
}

$passwordPointer = [IntPtr]::Zero
$passwordPlainText = $null
$locationPushed = $false

try {
    $passwordPointer = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($password)
    $passwordPlainText = [Runtime.InteropServices.Marshal]::PtrToStringBSTR($passwordPointer)

    [Environment]::SetEnvironmentVariable("TAURI_SIGNING_PRIVATE_KEY", $ResolvedKeyPath.Path, "Process")
    [Environment]::SetEnvironmentVariable("TAURI_SIGNING_PRIVATE_KEY_PATH", $null, "Process")
    [Environment]::SetEnvironmentVariable("TAURI_SIGNING_PRIVATE_KEY_PASSWORD", $passwordPlainText, "Process")

    Push-Location $ProjectRoot
    $locationPushed = $true

    Write-Host "Building Vora with the updater key at: $($ResolvedKeyPath.Path)"
    & pnpm tauri:build
    if ($LASTEXITCODE -ne 0) {
        throw "pnpm tauri:build failed with exit code $LASTEXITCODE."
    }

    if (-not (Test-Path -LiteralPath $BundleDirectory -PathType Container)) {
        throw "Updater bundle directory not found: $BundleDirectory"
    }

    $signedInstallers = Get-ChildItem -LiteralPath $BundleDirectory -Recurse -File |
        Where-Object { $_.Name -like "*.exe.zip.sig" } |
        Where-Object {
            $installerPath = $_.FullName.Substring(0, $_.FullName.Length - 4)
            Test-Path -LiteralPath $installerPath -PathType Leaf
        }

    if (-not $signedInstallers) {
        throw "Build completed, but no signed NSIS updater archive (.exe.zip.sig) was found in $BundleDirectory."
    }

    foreach ($signature in $signedInstallers) {
        Write-Host "Signed updater installer signature: $($signature.FullName)"
    }
}
finally {
    $password.Dispose()

    if ($passwordPointer -ne [IntPtr]::Zero) {
        [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($passwordPointer)
    }

    $passwordPlainText = $null
    foreach ($name in $previousSigningEnvironment.Keys) {
        [Environment]::SetEnvironmentVariable($name, $previousSigningEnvironment[$name], "Process")
    }

    if ($locationPushed) {
        Pop-Location
    }
}
