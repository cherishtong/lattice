param(
    [string] $Version = "0.1.0",
    [switch] $Installer,
    [switch] $Offline
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$dist = Join-Path $root "dist"
$portable = Join-Path $dist "Lattice-$Version-windows-x64"
$zipPath = Join-Path $dist "Lattice-$Version-windows-x64.zip"
$installerPath = Join-Path $dist "Lattice-$Version-Setup.exe"
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"

function Write-Checksum {
    param([string] $Path)
    if (-not (Test-Path -LiteralPath $Path)) { return }
    $hash = (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
    $name = Split-Path -Leaf $Path
    Set-Content -LiteralPath "$Path.sha256" -Value "$hash  $name" -Encoding ascii
    Write-Host "SHA-256: $name"
}

function Sign-ReleaseFile {
    param([string] $Path)
    $sha1 = $env:LATTICE_SIGN_CERT_SHA1
    if ([string]::IsNullOrWhiteSpace($sha1) -or -not (Test-Path -LiteralPath $Path)) { return }
    $signtool = Get-Command "signtool.exe" -ErrorAction SilentlyContinue
    if (-not $signtool) {
        throw "LATTICE_SIGN_CERT_SHA1 is set but signtool.exe was not found."
    }
    & $signtool.Source sign /sha1 $sha1 /fd SHA256 /tr "http://timestamp.digicert.com" /td SHA256 $Path
    if ($LASTEXITCODE -ne 0) { throw "Code signing failed: $Path" }
}

Push-Location $root
try {
    Write-Host "Building release..."
    $buildArgs = @("build", "--release", "-p", "lattice", "-p", "lattice-translator")
    if ($Offline) { $buildArgs += "--offline" }
    & cargo @buildArgs
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    $metadataArgs = @("metadata", "--format-version", "1", "--no-deps")
    if ($Offline) { $metadataArgs += "--offline" }
    $targetDirectory = & cargo @metadataArgs | ConvertFrom-Json | Select-Object -ExpandProperty target_directory
    if (-not (Test-Path -LiteralPath $targetDirectory)) {
        throw "Cargo target directory not found: $targetDirectory"
    }

    Sign-ReleaseFile (Join-Path $targetDirectory "release\lattice.exe")
    Sign-ReleaseFile (Join-Path $targetDirectory "release\lattice-translator.exe")

    if (Test-Path -LiteralPath $portable) {
        Remove-Item -LiteralPath $portable -Recurse -Force
    }
    New-Item -ItemType Directory -Path $portable -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $targetDirectory "release\lattice.exe") -Destination $portable
    Copy-Item -LiteralPath (Join-Path $targetDirectory "release\lattice-translator.exe") -Destination $portable
    Copy-Item -LiteralPath (Join-Path $root "README.md") -Destination $portable
    Copy-Item -LiteralPath (Join-Path $root "LICENSE") -Destination $portable
    Copy-Item -LiteralPath (Join-Path $root "THIRD_PARTY_LICENSES.md") -Destination $portable
    New-Item -ItemType Directory -Path (Join-Path $portable "docs") -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $root "docs\size-budget.md") -Destination (Join-Path $portable "docs")
    Copy-Item -LiteralPath (Join-Path $root "docs\source-pack.md") -Destination (Join-Path $portable "docs")
    Copy-Item -LiteralPath (Join-Path $root "docs\translation.md") -Destination (Join-Path $portable "docs")
    Copy-Item -LiteralPath (Join-Path $root "docs\install.md") -Destination (Join-Path $portable "docs")
    New-Item -ItemType Directory -Path (Join-Path $portable "scripts") -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $root "scripts\fetch-opus-mt-model.ps1") -Destination (Join-Path $portable "scripts")

    if (Test-Path -LiteralPath $zipPath) {
        Remove-Item -LiteralPath $zipPath -Force
    }
    Compress-Archive -Path (Join-Path $portable "*") -DestinationPath $zipPath -CompressionLevel Optimal
    Write-Host "Portable package: $zipPath"
    Write-Checksum $zipPath

    if ($Installer) {
        $iscc = Get-Command "ISCC.exe" -ErrorAction SilentlyContinue
        if (-not $iscc) {
            $iscc = Get-ChildItem "C:\Program Files*\Inno Setup*\ISCC.exe" -ErrorAction SilentlyContinue |
                Select-Object -First 1
        }
        if (-not $iscc) {
            throw "Inno Setup not found. Install it or omit -Installer."
        }
        $isccPath = if ($iscc -is [System.Management.Automation.CommandInfo]) { $iscc.Source } else { $iscc.FullName }
        & $isccPath "/DAppVersion=$Version" "/DTargetDir=$(Join-Path $targetDirectory 'release')" (Join-Path $root "packaging\lattice.iss")
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
        Sign-ReleaseFile $installerPath
        Write-Checksum $installerPath
        Write-Host "Installer: $installerPath"
    }
} finally {
    Pop-Location
}
