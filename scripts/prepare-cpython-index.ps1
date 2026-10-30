param(
    [string] $Pack = ""
)

$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
if ([string]::IsNullOrWhiteSpace($Pack)) {
    if (-not [string]::IsNullOrWhiteSpace($env:LATTICE_SOURCE_PACK)) {
        $Pack = $env:LATTICE_SOURCE_PACK
    } else {
        $Pack = Join-Path $root ".content\cpython-3.13.2"
    }
}

if (-not (Test-Path -LiteralPath (Join-Path $Pack ".lattice\pack.json"))) {
    throw "Source pack not found: $Pack. Fetch it first or pass -Pack <directory>."
}

$env:LATTICE_SOURCE_PACK = (Resolve-Path -LiteralPath $Pack).Path
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"

Push-Location $root
try {
    cargo run --release -p lattice-query --example index_cpython
    cargo run --release -p lattice-query --example prebuild_documents
    Write-Host ""
    Write-Host "Prebuilt index and document cache:"
    Get-Item (Join-Path $env:LATTICE_SOURCE_PACK ".lattice\symbol-index-v3.postcard") |
        Select-Object FullName, Length, LastWriteTime
} finally {
    Pop-Location
}
