param(
    [string] $Version = "0.1.0",
    [switch] $Installer
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$dist = Join-Path $root "dist"
$portable = Join-Path $dist "Lattice-$Version-windows-x64"
$zipPath = Join-Path $dist "Lattice-$Version-windows-x64.zip"
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"

Push-Location $root
try {
    Write-Host "Building release..."
    cargo build --release -p lattice -p lattice-translator --offline
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    $targetDirectory = cargo metadata --format-version 1 --no-deps --offline |
        ConvertFrom-Json |
        Select-Object -ExpandProperty target_directory
    if (-not (Test-Path -LiteralPath $targetDirectory)) {
        throw "Cargo target directory not found: $targetDirectory"
    }

    if (Test-Path -LiteralPath $portable) {
        Remove-Item -LiteralPath $portable -Recurse -Force
    }
    New-Item -ItemType Directory -Path $portable -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $targetDirectory "release\lattice.exe") -Destination $portable
    Copy-Item -LiteralPath (Join-Path $targetDirectory "release\lattice-translator.exe") -Destination $portable
    Copy-Item -LiteralPath (Join-Path $root "README.md") -Destination $portable
    New-Item -ItemType Directory -Path (Join-Path $portable "docs") -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $root "docs\size-budget.md") -Destination (Join-Path $portable "docs")
    Copy-Item -LiteralPath (Join-Path $root "docs\source-pack.md") -Destination (Join-Path $portable "docs")
    Copy-Item -LiteralPath (Join-Path $root "docs\translation.md") -Destination (Join-Path $portable "docs")
    New-Item -ItemType Directory -Path (Join-Path $portable "scripts") -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $root "scripts\fetch-opus-mt-model.ps1") -Destination (Join-Path $portable "scripts")

    if (Test-Path -LiteralPath $zipPath) {
        Remove-Item -LiteralPath $zipPath -Force
    }
    Compress-Archive -Path (Join-Path $portable "*") -DestinationPath $zipPath -CompressionLevel Optimal
    Write-Host "Portable package: $zipPath"

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
        & $isccPath "/DAppVersion=$Version" (Join-Path $root "packaging\lattice.iss")
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
        Write-Host "Installer: $(Join-Path $dist 'Lattice-Setup.exe')"
    }
} finally {
    Pop-Location
}
