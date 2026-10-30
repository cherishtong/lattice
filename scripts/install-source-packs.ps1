param(
    [string[]] $Pack = @(),
    [switch] $Force
)

$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$archiveDir = Join-Path $root "source-packs"
$content = Join-Path $root ".content"
$tar = Join-Path $env:SystemRoot "System32\tar.exe"
. (Join-Path $PSScriptRoot "source-pack-common.ps1")

if (-not (Test-Path -LiteralPath $archiveDir)) {
    throw "Archive directory not found: $archiveDir"
}
New-Item -ItemType Directory -Path $content -Force | Out-Null

if ($Pack.Count -eq 0) {
    $Pack = Get-ChildItem -LiteralPath $archiveDir -Filter *.zip |
        ForEach-Object { $_.BaseName }
}
if ($Pack.Count -eq 0) {
    throw "No source pack archives found in $archiveDir"
}

$installed = 0
$skipped = 0
foreach ($name in $Pack) {
    $archive = Join-Path $archiveDir "$name.zip"
    if (-not (Test-Path -LiteralPath $archive)) {
        throw "Source pack archive not found: $archive"
    }

    $target = Join-Path $content $name
    $manifest = Join-Path $target ".lattice\pack.json"
    if (Test-Path -LiteralPath $manifest) {
        if (-not $Force) {
            Write-Host "Skip $name (already installed; use -Force to replace)"
            $skipped++
            continue
        }
        Remove-Tree -Path $target
    }

    Write-Host "Installing $name..."
    & $tar -xf $archive -C $content
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to extract $name."
    }
    if (-not (Test-Path -LiteralPath $manifest)) {
        throw "Archive did not contain .lattice/pack.json: $archive"
    }
    $installed++
}

Write-Host ""
Write-Host "Installed $installed source pack(s), skipped $skipped."
Write-Host "Open Lattice and select a pack; symbol and document caches rebuild on first use."
