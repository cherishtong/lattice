param(
    [string] $Pack = "",
    [switch] $Offline
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"

Push-Location $root
try {
    $arguments = @("run", "--release", "-p", "lattice-content", "--example", "stamp_checksums")
    if ($Offline) { $arguments += "--offline" }
    if (-not [string]::IsNullOrWhiteSpace($Pack)) {
        $arguments += "--"
        $arguments += (Resolve-Path -LiteralPath $Pack).Path
    }
    & cargo @arguments
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
} finally {
    Pop-Location
}
