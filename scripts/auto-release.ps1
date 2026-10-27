param(
    [switch]$DryRun
)

$ErrorActionPreference = 'Stop'
$script:DryRun = $DryRun

function Write-Log {
    param([string]$Message)
    Write-Output "[auto-release] $Message"
}

function Invoke-Git {
    param([string[]]$Arguments)

    $output = @(& git @Arguments 2>&1)
    $exitCode = $LASTEXITCODE
    $text = ($output | ForEach-Object { "$_" }) -join [Environment]::NewLine
    if ($exitCode -ne 0) {
        throw "git $($Arguments -join ' ') failed (exit $exitCode): $text"
    }

    return $text.Trim()
}

function Write-Utf8File {
    param(
        [string]$Path,
        [string]$Content
    )

    [System.IO.File]::WriteAllText(
        $Path,
        $Content,
        [System.Text.UTF8Encoding]::new($false)
    )
}

function Invoke-AutoRelease {
    if ($env:SKIP_AUTO_RELEASE -ieq 'true' -or $env:CI -ieq 'true') {
        return
    }

    try {
        $repoRoot = Invoke-Git -Arguments @('rev-parse', '--show-toplevel')
    }
    catch {
        return
    }

    Set-Location -LiteralPath $repoRoot

    if (Test-Path -LiteralPath (Join-Path $repoRoot '.skip-auto-release')) {
        return
    }

    try {
        [void](Invoke-Git -Arguments @('symbolic-ref', '--quiet', '--short', 'HEAD'))
    }
    catch {
        return
    }

    foreach ($statePath in @('rebase-merge', 'rebase-apply', 'MERGE_HEAD')) {
        try {
            $gitPath = Invoke-Git -Arguments @('rev-parse', '--git-path', $statePath)
        }
        catch {
            return
        }

        if (-not [System.IO.Path]::IsPathRooted($gitPath)) {
            $gitPath = Join-Path $repoRoot $gitPath
        }
        if (Test-Path -LiteralPath $gitPath) {
            return
        }
    }

    try {
        $status = Invoke-Git -Arguments @('status', '--porcelain')
    }
    catch {
        Write-Log "Unable to inspect git status: $($_.Exception.Message)"
        return
    }

    if ([string]::IsNullOrWhiteSpace($status)) {
        Write-Log 'No changes, skipping.'
        return
    }

    try {
        $packagePath = Join-Path $repoRoot 'package.json'
        if (-not (Test-Path -LiteralPath $packagePath)) {
            throw 'package.json was not found.'
        }

        $packageText = [System.IO.File]::ReadAllText($packagePath)
        $packageJson = $packageText | ConvertFrom-Json -ErrorAction Stop
        $currentVersion = [string]$packageJson.version
        if ($currentVersion -notmatch '^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$') {
            throw "package.json has an invalid stable semantic version: $currentVersion"
        }

        $major = [long]::Parse($Matches[1], [Globalization.CultureInfo]::InvariantCulture)
        $minor = [long]::Parse($Matches[2], [Globalization.CultureInfo]::InvariantCulture)
        $patch = [long]::Parse($Matches[3], [Globalization.CultureInfo]::InvariantCulture)
        if ($patch -eq [long]::MaxValue) {
            throw "Patch version cannot be incremented: $currentVersion"
        }
        $newVersion = "$major.$minor.$($patch + 1)"

        $packageVersionPattern = '(?m)^(\s*"version"\s*:\s*")[^"]+(")'
        if (-not [regex]::IsMatch($packageText, $packageVersionPattern)) {
            throw 'Could not locate the version property in package.json.'
        }
        $newPackageText = [regex]::Replace(
            $packageText,
            $packageVersionPattern,
            ('$1' + $newVersion + '$2'),
            1
        )

        $changes = [System.Collections.Generic.List[object]]::new()
        $changes.Add([pscustomobject]@{ Path = $packagePath; Content = $newPackageText })

        $tauriPath = Join-Path $repoRoot 'src-tauri/tauri.conf.json'
        if (Test-Path -LiteralPath $tauriPath) {
            $tauriText = [System.IO.File]::ReadAllText($tauriPath)
            $tauriJson = $tauriText | ConvertFrom-Json -ErrorAction Stop
            if (-not $tauriJson.PSObject.Properties['version']) {
                throw 'src-tauri/tauri.conf.json has no version property.'
            }
            $tauriVersionPattern = '(?m)^(\s*"version"\s*:\s*")[^"]+(")'
            if (-not [regex]::IsMatch($tauriText, $tauriVersionPattern)) {
                throw 'Could not locate the version property in src-tauri/tauri.conf.json.'
            }
            $newTauriText = [regex]::Replace(
                $tauriText,
                $tauriVersionPattern,
                ('$1' + $newVersion + '$2'),
                1
            )
            $changes.Add([pscustomobject]@{ Path = $tauriPath; Content = $newTauriText })
        }

        $cargoPath = Join-Path $repoRoot 'src-tauri/Cargo.toml'
        if (Test-Path -LiteralPath $cargoPath) {
            $cargoText = [System.IO.File]::ReadAllText($cargoPath)
            $packageSection = [regex]::Match(
                $cargoText,
                '(?ms)^\[package\][ \t]*\r?\n(?<body>.*?)(?=^\[|\z)'
            )
            if (-not $packageSection.Success) {
                throw 'Could not locate the [package] section in src-tauri/Cargo.toml.'
            }
            $cargoVersion = [regex]::Match(
                $packageSection.Groups['body'].Value,
                '(?m)^(?<prefix>[ \t]*version[ \t]*=[ \t]*")[^"]+(?<suffix>")'
            )
            if (-not $cargoVersion.Success) {
                throw 'Could not locate version inside the [package] section of src-tauri/Cargo.toml.'
            }

            $body = $packageSection.Groups['body'].Value
            $updatedBody = $body.Substring(0, $cargoVersion.Index) +
                $cargoVersion.Groups['prefix'].Value + $newVersion +
                $cargoVersion.Groups['suffix'].Value +
                $body.Substring($cargoVersion.Index + $cargoVersion.Length)
            $bodyIndex = $packageSection.Groups['body'].Index
            $newCargoText = $cargoText.Substring(0, $bodyIndex) +
                $updatedBody +
                $cargoText.Substring($bodyIndex + $body.Length)
            $changes.Add([pscustomobject]@{ Path = $cargoPath; Content = $newCargoText })
        }

        if ($DryRun) {
            Write-Log "Would bump version: $currentVersion -> $newVersion"
            foreach ($change in $changes) {
                Write-Log "Would update: $($change.Path.Substring($repoRoot.Length).TrimStart('\', '/'))"
            }
            Write-Log 'Would run: git add -A'
            Write-Log "Would commit: chore(release): v$newVersion [auto]"
            Write-Log 'Would run: git push origin HEAD'
            return
        }

        foreach ($change in $changes) {
            Write-Utf8File -Path $change.Path -Content $change.Content
        }

        [void](Invoke-Git -Arguments @('add', '-A'))
        [void](Invoke-Git -Arguments @(
            'commit',
            '--no-verify',
            '-m',
            "chore(release): v$newVersion [auto]"
        ))
        Write-Log "Committed version $newVersion."

        [void](Invoke-Git -Arguments @('push', 'origin', 'HEAD'))
        Write-Log 'Pushed to origin.'
    }
    catch {
        Write-Log "Release failed: $($_.Exception.Message)"
    }
}

try {
    Invoke-AutoRelease
}
catch {
    Write-Log "Release failed: $($_.Exception.Message)"
}

exit 0
